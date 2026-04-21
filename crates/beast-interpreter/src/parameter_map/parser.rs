//! Winnow-based parser for the S4.4 parameter-mapping language.
//!
//! The grammar is expressed declaratively as composed
//! [`winnow::Parser`]s; adding a new binary operator is a local edit to
//! the corresponding precedence-level function plus a new variant on
//! [`ExprNode`] (see issue #89 / #61).
//!
//! Channel-symbol resolution happens as a single post-parse pass in
//! [`validate_channels`] — the parser itself is pure syntactic. This
//! keeps the grammar free of registry state and lets resolution errors
//! surface with their own [`InterpreterError::UnknownChannelSymbol`]
//! variant rather than being tangled with parse failures.
//!
//! The `self.pos + 3` hazard that was tracked as #87 is retired here:
//! byte arithmetic is done exclusively by winnow's combinators, which
//! operate over `&str` slices and cannot wrap.

use beast_channels::ChannelRegistry;

use winnow::ascii::multispace0;
use winnow::combinator::{alt, cut_err, delimited, opt, preceded, repeat};
use winnow::error::{ContextError, ErrMode, ParseError, StrContext, StrContextValue};
use winnow::token::{one_of, take_while};
use winnow::{ModalResult, Parser};

use crate::error::{InterpreterError, Result};

use super::ast::{CompiledExpr, ExprNode};
use super::literal::{parse_q3232_literal, strip_underscores};

/// Parse a parameter-mapping expression source string into a [`CompiledExpr`].
///
/// # Grammar (minimal operator set — sprint plan Q3)
///
/// ```text
/// expr    ::= term ( "+" term )*       // left-associative
/// term    ::= factor ( "*" factor )*   // left-associative, binds tighter than `+`
/// factor  ::= literal | ch_ref | "(" expr ")"
/// literal ::= [0-9][0-9_]* ( "." [0-9_]+ )?
/// ch_ref  ::= "ch[" ident "]"
/// ident   ::= [a-z_][a-z0-9_]*
/// ```
///
/// Whitespace (spaces, tabs, newlines) is skipped between tokens.
/// Channel symbols are resolved to ids at parse time (sprint plan Q4):
/// the symbol must be registered in `registry` or the parser returns
/// [`InterpreterError::UnknownChannelSymbol`]. Any other parse failure
/// returns [`InterpreterError::ParseError`].
///
/// # Examples
///
/// ```
/// use beast_channels::{
///     BoundsPolicy, ChannelFamily, ChannelManifest, ChannelRegistry,
///     MutationKernel, Provenance, Range, ScaleBand,
/// };
/// use beast_core::Q3232;
/// use beast_interpreter::parameter_map::parse_expression;
///
/// let manifest = ChannelManifest {
///     id: "vocal_modulation".into(),
///     family: ChannelFamily::Motor,
///     description: "fixture".into(),
///     range: Range { min: Q3232::ZERO, max: Q3232::ONE, units: "dimensionless".into() },
///     mutation_kernel: MutationKernel {
///         sigma: Q3232::from_num(0.1_f64),
///         bounds_policy: BoundsPolicy::Clamp,
///         genesis_weight: Q3232::ONE,
///         correlation_with: Vec::new(),
///     },
///     composition_hooks: Vec::new(),
///     expression_conditions: Vec::new(),
///     scale_band: ScaleBand { min_kg: Q3232::ZERO, max_kg: Q3232::from_num(1_000_i32) },
///     body_site_applicable: false,
///     provenance: Provenance::Core,
/// };
/// let mut registry = ChannelRegistry::new();
/// registry.register(manifest).unwrap();
///
/// let _expr = parse_expression("ch[vocal_modulation] * 8", &registry).unwrap();
/// ```
pub fn parse_expression(src: &str, registry: &ChannelRegistry) -> Result<CompiledExpr> {
    let node = delimited(multispace0, expr, multispace0)
        .parse(src)
        .map_err(|e| translate_parse_error(src, &e))?;
    validate_channels(&node, registry)?;
    Ok(CompiledExpr::from_node(node))
}

// ---------------------------------------------------------------------------
// Grammar — one function per precedence level. Adding `-` / `/` (issue
// #61) means extending the `one_of([...])` set and the fold's match in
// the corresponding level; no new function required.
// ---------------------------------------------------------------------------

fn expr(input: &mut &str) -> ModalResult<ExprNode> {
    let init = term.parse_next(input)?;
    repeat(0.., (preceded(multispace0, one_of(['+'])), term))
        .fold(
            move || init.clone(),
            |acc, (op, rhs): (char, ExprNode)| match op {
                '+' => ExprNode::Add(Box::new(acc), Box::new(rhs)),
                _ => unreachable!("one_of restricts the set"),
            },
        )
        .parse_next(input)
}

fn term(input: &mut &str) -> ModalResult<ExprNode> {
    let init = factor.parse_next(input)?;
    repeat(0.., (preceded(multispace0, one_of(['*'])), factor))
        .fold(
            move || init.clone(),
            |acc, (op, rhs): (char, ExprNode)| match op {
                '*' => ExprNode::Mul(Box::new(acc), Box::new(rhs)),
                _ => unreachable!("one_of restricts the set"),
            },
        )
        .parse_next(input)
}

fn factor(input: &mut &str) -> ModalResult<ExprNode> {
    delimited(
        multispace0,
        alt((literal, channel_ref, parens)),
        multispace0,
    )
    .context(StrContext::Label("factor"))
    .parse_next(input)
}

fn parens(input: &mut &str) -> ModalResult<ExprNode> {
    delimited(
        '(',
        cut_err(expr),
        cut_err(')'.context(StrContext::Expected(StrContextValue::CharLiteral(')')))),
    )
    .parse_next(input)
}

fn literal(input: &mut &str) -> ModalResult<ExprNode> {
    let raw = (
        one_of(|c: char| c.is_ascii_digit()),
        take_while(0.., |c: char| c.is_ascii_digit() || c == '_'),
        opt((
            '.',
            // Committed: a `.` after the integer part demands fractional
            // digits. `cut_err` prevents the surrounding `opt` from
            // silently swallowing the failure.
            cut_err(
                take_while(1.., |c: char| c.is_ascii_digit() || c == '_').context(
                    StrContext::Expected(StrContextValue::Description(
                        "fractional digits after `.`",
                    )),
                ),
            ),
        )),
    )
        .take()
        .parse_next(input)?;
    let cleaned = strip_underscores(raw.as_bytes());
    // winnow already validated the byte shape (digits / `_` / `.`); if
    // `parse_q3232_literal` still refuses, the Q32.32 range is the
    // problem, not syntax. `Cut` short-circuits alternatives so callers
    // see a literal-specific failure.
    let value = parse_q3232_literal(&cleaned).ok_or_else(|| ErrMode::Cut(ContextError::new()))?;
    Ok(ExprNode::Literal(value))
}

fn channel_ref(input: &mut &str) -> ModalResult<ExprNode> {
    let _ = "ch[".parse_next(input)?;
    // Committed: after `ch[` the grammar demands an identifier and a
    // closing `]`. `cut_err` prevents `alt` from backtracking past this
    // point, so failures surface with channel-specific context instead
    // of the generic "expected factor".
    let symbol = cut_err(
        (
            one_of(|c: char| c.is_ascii_lowercase() || c == '_').context(StrContext::Expected(
                StrContextValue::Description("channel identifier"),
            )),
            take_while(0.., |c: char| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
            }),
        )
            .take(),
    )
    .parse_next(input)?;
    let _ = cut_err(']'.context(StrContext::Expected(StrContextValue::Description(
        "`]` closing channel reference",
    ))))
    .parse_next(input)?;
    Ok(ExprNode::ChannelRef(symbol.to_owned()))
}

// ---------------------------------------------------------------------------
// Semantic pass: channel-symbol resolution.
// ---------------------------------------------------------------------------

fn validate_channels(node: &ExprNode, registry: &ChannelRegistry) -> Result<()> {
    match node {
        ExprNode::Literal(_) => Ok(()),
        ExprNode::ChannelRef(symbol) => {
            if registry.contains(symbol) {
                Ok(())
            } else {
                Err(InterpreterError::UnknownChannelSymbol {
                    symbol: symbol.clone(),
                })
            }
        }
        ExprNode::Add(lhs, rhs) | ExprNode::Mul(lhs, rhs) => {
            validate_channels(lhs, registry)?;
            validate_channels(rhs, registry)
        }
    }
}

// ---------------------------------------------------------------------------
// Error translation
// ---------------------------------------------------------------------------

fn translate_parse_error(src: &str, err: &ParseError<&str, ContextError>) -> InterpreterError {
    let offset = err.offset();
    let bytes = src.as_bytes();

    // Describe what was expected, if the parser left context breadcrumbs.
    let expected = err
        .inner()
        .context()
        .filter_map(|ctx| match ctx {
            StrContext::Expected(v) => Some(format!("{v}")),
            _ => None,
        })
        .next();

    let message = if offset >= bytes.len() {
        match expected {
            Some(desc) => format!("unexpected end of input at byte {offset}, expected {desc}"),
            None => format!("unexpected end of input while parsing factor at byte {offset}"),
        }
    } else {
        let bad = bytes[offset];
        let ch = bad as char;
        match expected {
            Some(desc) => format!(
                "unexpected character `{ch}` (0x{bad:02x}) at byte {offset}, expected {desc}"
            ),
            None => format!("unexpected character `{ch}` (0x{bad:02x}) at byte {offset}"),
        }
    };

    InterpreterError::ParseError { message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parameter_map::test_support::registry_with;
    use beast_channels::ChannelRegistry;
    use beast_core::Q3232;

    // ------- parser happy paths --------------------------------------------

    #[test]
    fn parses_integer_literal() {
        let reg = ChannelRegistry::new();
        let expr = parse_expression("8", &reg).unwrap();
        match expr.node() {
            ExprNode::Literal(v) => assert_eq!(*v, Q3232::from_num(8_i32)),
            _ => panic!("expected Literal"),
        }
    }

    #[test]
    fn parses_fractional_literal() {
        let reg = ChannelRegistry::new();
        let expr = parse_expression("0.5", &reg).unwrap();
        match expr.node() {
            ExprNode::Literal(v) => assert_eq!(*v, Q3232::from_num(0.5_f64)),
            _ => panic!("expected Literal"),
        }
    }

    #[test]
    fn parses_underscore_separated_literal() {
        let reg = ChannelRegistry::new();
        let expr = parse_expression("1_000", &reg).unwrap();
        match expr.node() {
            ExprNode::Literal(v) => assert_eq!(*v, Q3232::from_num(1_000_i32)),
            _ => panic!("expected Literal"),
        }
    }

    #[test]
    fn parses_channel_reference_when_registered() {
        let reg = registry_with(&["vocal_modulation"]);
        let expr = parse_expression("ch[vocal_modulation]", &reg).unwrap();
        match expr.node() {
            ExprNode::ChannelRef(id) => assert_eq!(id, "vocal_modulation"),
            _ => panic!("expected ChannelRef"),
        }
    }

    #[test]
    fn parses_addition_left_associative() {
        let reg = registry_with(&["a", "b", "c"]);
        let expr = parse_expression("ch[a] + ch[b] + ch[c]", &reg).unwrap();
        // Shape must be ((a + b) + c), not (a + (b + c)).
        match expr.node() {
            ExprNode::Add(lhs, _rhs) => match lhs.as_ref() {
                ExprNode::Add(_, _) => {}
                other => panic!("expected nested Add on lhs, got {other:?}"),
            },
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let reg = registry_with(&["a", "b"]);
        // a + b * 2 must parse as Add(a, Mul(b, 2)).
        let expr = parse_expression("ch[a] + ch[b] * 2", &reg).unwrap();
        match expr.node() {
            ExprNode::Add(_, rhs) => match rhs.as_ref() {
                ExprNode::Mul(_, _) => {}
                other => panic!("expected Mul on rhs of Add, got {other:?}"),
            },
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn parentheses_override_precedence() {
        let reg = registry_with(&["a", "b"]);
        let expr = parse_expression("(ch[a] + ch[b]) * 2", &reg).unwrap();
        match expr.node() {
            ExprNode::Mul(lhs, _) => match lhs.as_ref() {
                ExprNode::Add(_, _) => {}
                other => panic!("expected Add inside parens, got {other:?}"),
            },
            other => panic!("expected Mul, got {other:?}"),
        }
    }

    #[test]
    fn skips_whitespace_everywhere() {
        let reg = registry_with(&["a"]);
        let expr = parse_expression("  ch[a]   *   8  +  0  ", &reg).unwrap();
        // Structure: Add(Mul(ChannelRef, 8), 0)
        match expr.node() {
            ExprNode::Add(lhs, _) => match lhs.as_ref() {
                ExprNode::Mul(_, _) => {}
                other => panic!("unexpected shape: {other:?}"),
            },
            other => panic!("unexpected shape: {other:?}"),
        }
    }

    // ------- parser error paths --------------------------------------------

    #[test]
    fn unknown_channel_symbol_errors() {
        let reg = ChannelRegistry::new();
        let err = parse_expression("ch[mystery]", &reg).unwrap_err();
        match err {
            InterpreterError::UnknownChannelSymbol { symbol } => assert_eq!(symbol, "mystery"),
            other => panic!("expected UnknownChannelSymbol, got {other:?}"),
        }
    }

    #[test]
    fn trailing_garbage_is_a_parse_error() {
        let reg = ChannelRegistry::new();
        assert!(matches!(
            parse_expression("8 xyz", &reg),
            Err(InterpreterError::ParseError { .. })
        ));
    }

    #[test]
    fn unbalanced_parenthesis_is_a_parse_error() {
        let reg = ChannelRegistry::new();
        assert!(matches!(
            parse_expression("(8 + 1", &reg),
            Err(InterpreterError::ParseError { .. })
        ));
    }

    #[test]
    fn empty_input_is_a_parse_error() {
        let reg = ChannelRegistry::new();
        assert!(matches!(
            parse_expression("   ", &reg),
            Err(InterpreterError::ParseError { .. })
        ));
    }

    #[test]
    fn ch_without_brackets_is_an_error() {
        let reg = ChannelRegistry::new();
        assert!(matches!(
            parse_expression("ch foo", &reg),
            Err(InterpreterError::ParseError { .. })
        ));
    }

    #[test]
    fn missing_closing_bracket_is_an_error() {
        let reg = registry_with(&["a"]);
        assert!(matches!(
            parse_expression("ch[a", &reg),
            Err(InterpreterError::ParseError { .. })
        ));
    }

    // ------- error-message regression (issue #89) -------------------------

    /// Pin the shape of key parse-error messages so future grammar tweaks
    /// are a deliberate choice, not a silent drift. Each row is
    /// `(source, expected_substring)` — substring match keeps the test
    /// resilient to incidental wording changes while still asserting the
    /// essential bits (byte position, anchor keyword).
    #[test]
    fn error_messages_carry_useful_context() {
        let reg = registry_with(&["a"]);
        let table: &[(&str, &str)] = &[
            ("8 xyz", "at byte 2"),
            ("   ", "at byte 3"),
            ("(8 + 1", "`)`"),
            ("ch[a", "`]` closing channel reference"),
            ("ch[", "channel identifier"),
            ("1.", "fractional digits after `.`"),
        ];
        for (src, needle) in table {
            let err = parse_expression(src, &reg).unwrap_err();
            let InterpreterError::ParseError { message } = &err else {
                panic!("expected ParseError for `{src}`, got {err:?}");
            };
            assert!(
                message.contains(needle),
                "parse_expression({src:?}) → `{message}`, expected to contain `{needle}`"
            );
        }
    }

    // ------- #87 regression: ch[ probe must not wrap usize ---------------

    /// Before #89, `parse_factor` used a bare `self.pos + 3` to probe for
    /// `ch[`. That add could wrap on pathological inputs. The winnow
    /// rewrite uses slice combinators throughout, which are wrap-free by
    /// construction. This test exercises the boundary path where the
    /// input is shorter than three bytes starting with `c`.
    #[test]
    fn short_c_prefixed_input_does_not_overflow() {
        let reg = ChannelRegistry::new();
        for src in ["c", "ch"] {
            let err = parse_expression(src, &reg).unwrap_err();
            assert!(
                matches!(err, InterpreterError::ParseError { .. }),
                "expected ParseError for {src:?}, got {err:?}"
            );
        }
    }
}
