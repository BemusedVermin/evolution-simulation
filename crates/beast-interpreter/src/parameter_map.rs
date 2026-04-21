//! Parameter-mapping expression parser and evaluator (S4.4 — issue #58).
//!
//! Expressions look like `ch[vocal_modulation] * 8 + ch[auditory_sensitivity]`
//! in the manifest source. They are parsed **once at manifest load time**
//! (sprint plan Q2) into an [`Expr`] AST where channel symbols have already
//! been resolved to channel ids (Q4). Evaluation is a pure fold over Q32.32
//! fixed-point arithmetic.
//!
//! The **minimal operator set** shipped in S4 is listed below; everything
//! else is tracked in issue #61.
//!
//! | Construct          | S4.4 | Deferred (#61) |
//! |--------------------|------|----------------|
//! | `ch[<symbol>]`     | ✓    |                |
//! | scalar literal     | ✓    |                |
//! | `+`, `*`           | ✓    |                |
//! | `sqrt(...)`        |      | ✓              |
//! | `[lo, hi]` range   |      | ✓              |
//! | implicit `clamp`   |      | ✓              |
//! | `-`, `/`           |      | ✓              |
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §6.2.

use std::collections::BTreeMap;

use beast_channels::ChannelRegistry;
use beast_core::Q3232;

use crate::error::{InterpreterError, Result};

/// Parsed parameter expression.
///
/// `ChannelRef` carries a resolved channel id (sprint plan Q4): the parser
/// looks the symbol up in the [`beast_channels::ChannelRegistry`] at load
/// time and rejects unknown symbols early. Evaluator therefore does not need
/// the registry — it walks the AST over a pre-indexed channel-value vector.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A literal Q32.32 value.
    Literal(Q3232),
    /// Reference to a channel value by resolved id.
    ChannelRef(String),
    /// Binary addition.
    Add(Box<Expr>, Box<Expr>),
    /// Binary multiplication.
    Mul(Box<Expr>, Box<Expr>),
}

/// Parse a parameter-mapping expression source string into an [`Expr`].
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
pub fn parse_expression(src: &str, registry: &ChannelRegistry) -> Result<Expr> {
    let mut parser = Parser::new(src, registry);
    let expr = parser.parse_expr()?;
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
    Ok(expr)
}

/// Evaluate a parsed expression against per-channel global values.
///
/// Returns [`Q3232::ZERO`] for any channel id not present in
/// `channel_values` — this matches the "dormant channels propagate zero"
/// rule from §6.2. Addition and multiplication use [`Q3232`] saturating
/// arithmetic, so overflow clamps to [`Q3232::MAX`] / [`Q3232::MIN`] rather
/// than panicking or wrapping.
///
/// This function is pure: the output is entirely determined by
/// `(expr, channel_values)`, satisfying the determinism invariant
/// (INVARIANTS §1).
#[must_use]
pub fn eval_expression(expr: &Expr, channel_values: &BTreeMap<String, Q3232>) -> Q3232 {
    match expr {
        Expr::Literal(v) => *v,
        Expr::ChannelRef(id) => channel_values.get(id).copied().unwrap_or(Q3232::ZERO),
        Expr::Add(lhs, rhs) => eval_expression(lhs, channel_values)
            .saturating_add(eval_expression(rhs, channel_values)),
        Expr::Mul(lhs, rhs) => eval_expression(lhs, channel_values)
            .saturating_mul(eval_expression(rhs, channel_values)),
    }
}

/// Return every channel id referenced by `expr`, sorted and deduplicated.
///
/// Used by the emission path to compose a hook's
/// [`beast_primitives::PrimitiveEffect::source_channels`] without having to
/// re-walk the raw manifest source string.
#[must_use]
pub fn collect_channel_refs(expr: &Expr) -> Vec<String> {
    let mut out = std::collections::BTreeSet::new();
    fn walk(expr: &Expr, out: &mut std::collections::BTreeSet<String>) {
        match expr {
            Expr::Literal(_) => {}
            Expr::ChannelRef(id) => {
                out.insert(id.clone());
            }
            Expr::Add(l, r) | Expr::Mul(l, r) => {
                walk(l, out);
                walk(r, out);
            }
        }
    }
    walk(expr, &mut out);
    out.into_iter().collect()
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
    fn parse_expr(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_term()?;
        loop {
            self.skip_whitespace();
            if self.peek() == Some(b'+') {
                self.pos += 1;
                let rhs = self.parse_term()?;
                lhs = Expr::Add(Box::new(lhs), Box::new(rhs));
            } else {
                return Ok(lhs);
            }
        }
    }

    /// Parse a `term` (multiplication-level).
    fn parse_term(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            if self.peek() == Some(b'*') {
                self.pos += 1;
                let rhs = self.parse_factor()?;
                lhs = Expr::Mul(Box::new(lhs), Box::new(rhs));
            } else {
                return Ok(lhs);
            }
        }
    }

    /// Parse a `factor` — literal, channel reference, or parenthesised `expr`.
    fn parse_factor(&mut self) -> Result<Expr> {
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
    fn parse_literal(&mut self) -> Result<Expr> {
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
        // Strip underscore separators before parsing. Both the integer and
        // fractional parsers treat `_` as a separator only — `fixed`'s FromStr
        // rejects underscores, so we strip here rather than depending on it.
        let cleaned: String = raw
            .iter()
            .filter(|b| **b != b'_')
            .map(|b| *b as char)
            .collect();
        let value = parse_q3232_literal(&cleaned).ok_or_else(|| InterpreterError::ParseError {
            message: format!("invalid numeric literal `{}`", cleaned),
        })?;
        Ok(Expr::Literal(value))
    }

    /// Parse a channel reference `ch[<ident>]` and resolve the symbol via
    /// the registry.
    fn parse_channel_ref(&mut self) -> Result<Expr> {
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
        Ok(Expr::ChannelRef(symbol.to_owned()))
    }
}

/// Parse a dot-decimal ASCII literal (already stripped of `_`) into a
/// [`Q3232`]. Returns `None` when neither the integer nor the combined
/// rational representation fit, or when the string is malformed for either
/// path.
fn parse_q3232_literal(cleaned: &str) -> Option<Q3232> {
    if let Some(dot) = cleaned.find('.') {
        let int_part = &cleaned[..dot];
        let frac_part = &cleaned[dot + 1..];
        if frac_part.is_empty() {
            return None;
        }
        // Parse as (numerator / 10^frac_len) in Q32.32 arithmetic so we
        // avoid floats. Both pieces must fit in `i64`.
        let int_val: i64 = int_part.parse().ok()?;
        let frac_val: i64 = frac_part.parse().ok()?;
        let scale: i64 = 10_i64.checked_pow(u32::try_from(frac_part.len()).ok()?)?;
        // int_val + frac_val / scale, in Q32.32.
        let int_q = Q3232::from_num(int_val);
        let frac_q = Q3232::from_num(frac_val).saturating_div(Q3232::from_num(scale));
        Some(int_q.saturating_add(frac_q))
    } else {
        let int_val: i64 = cleaned.parse().ok()?;
        Some(Q3232::from_num(int_val))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beast_channels::{
        BoundsPolicy, ChannelFamily, ChannelManifest, ChannelRegistry, MutationKernel, Provenance,
        Range, ScaleBand,
    };
    use proptest::prelude::*;

    fn manifest(id: &str) -> ChannelManifest {
        ChannelManifest {
            id: id.into(),
            family: ChannelFamily::Sensory,
            description: "fixture".into(),
            range: Range {
                min: Q3232::ZERO,
                max: Q3232::ONE,
                units: "dimensionless".into(),
            },
            mutation_kernel: MutationKernel {
                sigma: Q3232::from_num(0.1_f64),
                bounds_policy: BoundsPolicy::Clamp,
                genesis_weight: Q3232::ONE,
                correlation_with: Vec::new(),
            },
            composition_hooks: Vec::new(),
            expression_conditions: Vec::new(),
            scale_band: ScaleBand {
                min_kg: Q3232::ZERO,
                max_kg: Q3232::from_num(1_000_i32),
            },
            body_site_applicable: false,
            provenance: Provenance::Core,
        }
    }

    fn registry_with(ids: &[&str]) -> ChannelRegistry {
        let mut reg = ChannelRegistry::new();
        for id in ids {
            reg.register(manifest(id)).expect("unique fixture ids");
        }
        reg
    }

    fn channels(pairs: &[(&str, Q3232)]) -> BTreeMap<String, Q3232> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    // ------- parser happy paths --------------------------------------------

    #[test]
    fn parses_integer_literal() {
        let reg = ChannelRegistry::new();
        let expr = parse_expression("8", &reg).unwrap();
        match expr {
            Expr::Literal(v) => assert_eq!(v, Q3232::from_num(8_i32)),
            _ => panic!("expected Literal"),
        }
    }

    #[test]
    fn parses_fractional_literal() {
        let reg = ChannelRegistry::new();
        let expr = parse_expression("0.5", &reg).unwrap();
        match expr {
            Expr::Literal(v) => assert_eq!(v, Q3232::from_num(0.5_f64)),
            _ => panic!("expected Literal"),
        }
    }

    #[test]
    fn parses_underscore_separated_literal() {
        let reg = ChannelRegistry::new();
        let expr = parse_expression("1_000", &reg).unwrap();
        match expr {
            Expr::Literal(v) => assert_eq!(v, Q3232::from_num(1_000_i32)),
            _ => panic!("expected Literal"),
        }
    }

    #[test]
    fn parses_channel_reference_when_registered() {
        let reg = registry_with(&["vocal_modulation"]);
        let expr = parse_expression("ch[vocal_modulation]", &reg).unwrap();
        match expr {
            Expr::ChannelRef(id) => assert_eq!(id, "vocal_modulation"),
            _ => panic!("expected ChannelRef"),
        }
    }

    #[test]
    fn parses_addition_left_associative() {
        let reg = registry_with(&["a", "b", "c"]);
        let expr = parse_expression("ch[a] + ch[b] + ch[c]", &reg).unwrap();
        // Shape must be ((a + b) + c), not (a + (b + c)).
        match expr {
            Expr::Add(lhs, _rhs) => match *lhs {
                Expr::Add(_, _) => {}
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
        match expr {
            Expr::Add(_, rhs) => match *rhs {
                Expr::Mul(_, _) => {}
                other => panic!("expected Mul on rhs of Add, got {other:?}"),
            },
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn parentheses_override_precedence() {
        let reg = registry_with(&["a", "b"]);
        let expr = parse_expression("(ch[a] + ch[b]) * 2", &reg).unwrap();
        match expr {
            Expr::Mul(lhs, _) => match *lhs {
                Expr::Add(_, _) => {}
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
        match expr {
            Expr::Add(lhs, _) => match *lhs {
                Expr::Mul(_, _) => {}
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

    // ------- evaluator -----------------------------------------------------

    #[test]
    fn evaluates_literal() {
        let vals = BTreeMap::new();
        let out = eval_expression(&Expr::Literal(Q3232::from_num(3_i32)), &vals);
        assert_eq!(out, Q3232::from_num(3_i32));
    }

    #[test]
    fn evaluates_channel_ref_present() {
        let vals = channels(&[("a", Q3232::from_num(5_i32))]);
        let out = eval_expression(&Expr::ChannelRef("a".into()), &vals);
        assert_eq!(out, Q3232::from_num(5_i32));
    }

    #[test]
    fn evaluates_channel_ref_missing_as_zero() {
        let vals = BTreeMap::new();
        let out = eval_expression(&Expr::ChannelRef("missing".into()), &vals);
        assert_eq!(out, Q3232::ZERO);
    }

    #[test]
    fn roundtrip_parse_then_evaluate() {
        let reg = registry_with(&["a", "b"]);
        let expr = parse_expression("ch[a] * 8 + ch[b]", &reg).unwrap();
        let vals = channels(&[
            ("a", Q3232::from_num(0.5_f64)),
            ("b", Q3232::from_num(2_i32)),
        ]);
        // 0.5 * 8 + 2 = 6
        let out = eval_expression(&expr, &vals);
        assert_eq!(out, Q3232::from_num(6_i32));
    }

    #[test]
    fn roundtrip_same_input_yields_same_output_twice() {
        let reg = registry_with(&["a", "b"]);
        let expr = parse_expression("ch[a] * 8 + ch[b]", &reg).unwrap();
        let vals = channels(&[
            ("a", Q3232::from_num(0.25_f64)),
            ("b", Q3232::from_num(1_i32)),
        ]);
        let first = eval_expression(&expr, &vals);
        let second = eval_expression(&expr, &vals);
        assert_eq!(first, second);
    }

    #[test]
    fn saturating_addition_clamps() {
        let vals = BTreeMap::new();
        let expr = Expr::Add(
            Box::new(Expr::Literal(Q3232::MAX)),
            Box::new(Expr::Literal(Q3232::ONE)),
        );
        assert_eq!(eval_expression(&expr, &vals), Q3232::MAX);
    }

    #[test]
    fn collect_channel_refs_returns_sorted_unique_ids() {
        let reg = registry_with(&["alpha", "beta"]);
        let expr = parse_expression("ch[beta] + ch[alpha] * 2 + ch[beta]", &reg).unwrap();
        let refs = collect_channel_refs(&expr);
        assert_eq!(refs, vec!["alpha".to_string(), "beta".to_string()]);
    }

    // ------- proptest: evaluator purity -----------------------------------

    /// Sample a small, balanced `Expr` AST. We intentionally keep the channel
    /// alphabet small so the "missing channel → zero" path is exercised.
    fn arb_expr() -> impl Strategy<Value = Expr> {
        let leaf = prop_oneof![
            (-100_000_i64..=100_000_i64).prop_map(|n| Expr::Literal(Q3232::from_num(n))),
            prop::sample::select(vec!["a", "b", "c", "d", "missing"])
                .prop_map(|s| Expr::ChannelRef(s.to_string())),
        ];
        leaf.prop_recursive(4, 16, 2, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(l, r)| Expr::Add(Box::new(l), Box::new(r))),
                (inner.clone(), inner).prop_map(|(l, r)| Expr::Mul(Box::new(l), Box::new(r))),
            ]
        })
    }

    proptest! {
        /// `eval_expression` is a pure function of `(expr, channel_values)`:
        /// calling it twice on the same inputs always yields the same output.
        /// Required by INVARIANTS §1 (determinism).
        #[test]
        fn eval_is_pure_and_deterministic(
            expr in arb_expr(),
            bits_a in any::<i64>(),
            bits_b in any::<i64>(),
            bits_c in any::<i64>(),
            bits_d in any::<i64>(),
        ) {
            let vals = channels(&[
                ("a", Q3232::from_bits(bits_a)),
                ("b", Q3232::from_bits(bits_b)),
                ("c", Q3232::from_bits(bits_c)),
                ("d", Q3232::from_bits(bits_d)),
            ]);
            let first = eval_expression(&expr, &vals);
            let second = eval_expression(&expr, &vals);
            prop_assert_eq!(first, second);
        }
    }
}
