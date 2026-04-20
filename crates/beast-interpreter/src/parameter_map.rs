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

use beast_core::Q3232;

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

// Parser + evaluator implementation — see story S4.4 (#58).
