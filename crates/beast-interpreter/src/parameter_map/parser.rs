//! Recursive-descent parser for the S4.4 parameter-mapping language.
//!
//! Byte-at-a-time over ASCII source. Channel symbols are resolved against
//! the [`ChannelRegistry`] during `ch[<ident>]` parsing — unknown symbols
//! surface as [`InterpreterError::UnknownChannelSymbol`], every other
//! failure as [`InterpreterError::ParseError`]. See the
//! [`super`][super] module doc for the full grammar.

use beast_channels::ChannelRegistry;

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
/// Whitespace (spaces, tabs, newlines) is skipped between tokens. Channel
/// symbols are resolved to ids at parse time (sprint plan Q4): the symbol
/// must be registered in `registry` or the parser returns
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
    let mut parser = Parser::new(src, registry);
    let node = parser.parse_expr()?;
    parser.skip_whitespace();
    if !parser.at_end() {
        return Err(InterpreterError::ParseError {
            message: format!(
                "unexpected trailing input at byte {}: `{}`",
                parser.pos,
                parser.remaining()
            ),
        });
    }
    Ok(CompiledExpr::from_node(node))
}

// ---------------------------------------------------------------------------
// Parser (recursive descent, byte-at-a-time over ASCII source).
// ---------------------------------------------------------------------------

struct Parser<'src, 'reg> {
    src: &'src [u8],
    pos: usize,
    registry: &'reg ChannelRegistry,
}

impl<'src, 'reg> Parser<'src, 'reg> {
    fn new(src: &'src str, registry: &'reg ChannelRegistry) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
            registry,
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn remaining(&self) -> String {
        String::from_utf8_lossy(&self.src[self.pos..]).into_owned()
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Consume `expected` if it matches the next bytes after whitespace;
    /// return `true` on success, `false` otherwise (without advancing on
    /// failure).
    fn eat(&mut self, expected: u8) -> bool {
        self.skip_whitespace();
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Parse an `expr` (addition-level).
    fn parse_expr(&mut self) -> Result<ExprNode> {
        let mut lhs = self.parse_term()?;
        loop {
            self.skip_whitespace();
            if self.peek() == Some(b'+') {
                self.pos += 1;
                let rhs = self.parse_term()?;
                lhs = ExprNode::Add(Box::new(lhs), Box::new(rhs));
            } else {
                return Ok(lhs);
            }
        }
    }

    /// Parse a `term` (multiplication-level).
    fn parse_term(&mut self) -> Result<ExprNode> {
        let mut lhs = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            if self.peek() == Some(b'*') {
                self.pos += 1;
                let rhs = self.parse_factor()?;
                lhs = ExprNode::Mul(Box::new(lhs), Box::new(rhs));
            } else {
                return Ok(lhs);
            }
        }
    }

    /// Parse a `factor` — literal, channel reference, or parenthesised `expr`.
    fn parse_factor(&mut self) -> Result<ExprNode> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(InterpreterError::ParseError {
                message: "unexpected end of input while parsing factor".into(),
            }),
            Some(b'(') => {
                self.pos += 1;
                let inner = self.parse_expr()?;
                if !self.eat(b')') {
                    return Err(InterpreterError::ParseError {
                        message: format!(
                            "expected `)` at byte {}, found `{}`",
                            self.pos,
                            self.remaining()
                        ),
                    });
                }
                Ok(inner)
            }
            Some(b) if b.is_ascii_digit() => self.parse_literal(),
            Some(b'c') if self.src.get(self.pos..self.pos + 3) == Some(b"ch[") => {
                self.parse_channel_ref()
            }
            Some(b) => Err(InterpreterError::ParseError {
                message: format!(
                    "unexpected character `{}` (0x{:02x}) at byte {}",
                    b as char, b, self.pos
                ),
            }),
        }
    }

    /// Parse a numeric literal with optional underscore digit separators and
    /// an optional fractional part.
    fn parse_literal(&mut self) -> Result<ExprNode> {
        let start = self.pos;
        // Integer part (required).
        let mut saw_digit = false;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                saw_digit = true;
                self.pos += 1;
            } else if b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if !saw_digit {
            return Err(InterpreterError::ParseError {
                message: format!("expected digit at byte {}", start),
            });
        }
        // Optional fractional part.
        if self.peek() == Some(b'.') {
            self.pos += 1;
            let mut frac_digit = false;
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() {
                    frac_digit = true;
                    self.pos += 1;
                } else if b == b'_' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if !frac_digit {
                return Err(InterpreterError::ParseError {
                    message: format!("expected fractional digits after `.` at byte {}", self.pos),
                });
            }
        }
        let raw = &self.src[start..self.pos];
        // Integer and fractional parsers treat `_` as a separator only —
        // `fixed`'s FromStr rejects underscores, so [`strip_underscores`]
        // normalises first.
        let cleaned = strip_underscores(raw);
        let value = parse_q3232_literal(&cleaned).ok_or_else(|| InterpreterError::ParseError {
            message: format!("invalid numeric literal `{}`", cleaned),
        })?;
        Ok(ExprNode::Literal(value))
    }

    /// Parse a channel reference `ch[<ident>]` and resolve the symbol via
    /// the registry.
    fn parse_channel_ref(&mut self) -> Result<ExprNode> {
        // We already know the next three bytes are `ch[`.
        self.pos += 3;
        let ident_start = self.pos;
        while let Some(b) = self.peek() {
            let valid_first = ident_start == self.pos && (b.is_ascii_lowercase() || b == b'_');
            let valid_rest = ident_start != self.pos
                && (b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
            if valid_first || valid_rest {
                self.pos += 1;
            } else {
                break;
            }
        }
        if ident_start == self.pos {
            return Err(InterpreterError::ParseError {
                message: format!("expected channel identifier at byte {}", self.pos),
            });
        }
        let symbol = std::str::from_utf8(&self.src[ident_start..self.pos]).map_err(|_| {
            InterpreterError::ParseError {
                message: "channel identifier is not valid UTF-8".into(),
            }
        })?;
        if self.bump() != Some(b']') {
            return Err(InterpreterError::ParseError {
                message: format!("expected `]` closing `ch[{}`", symbol),
            });
        }
        if !self.registry.contains(symbol) {
            return Err(InterpreterError::UnknownChannelSymbol {
                symbol: symbol.to_owned(),
            });
        }
        Ok(ExprNode::ChannelRef(symbol.to_owned()))
    }
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
}
