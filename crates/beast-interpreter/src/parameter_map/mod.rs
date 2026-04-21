//! Parameter-mapping expression parser and evaluator (S4.4 — issue #58).
//!
//! Expressions look like `ch[vocal_modulation] * 8 + ch[auditory_sensitivity]`
//! in the manifest source. They are parsed **once at manifest load time**
//! (sprint plan Q2) into a [`CompiledExpr`] where channel symbols have
//! already been resolved to channel ids (Q4). Evaluation is a pure fold
//! over Q32.32 fixed-point arithmetic.
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
//!
//! # Module layout (issue #76 split)
//!
//! The original monolithic `parameter_map.rs` was split into focused
//! sub-modules in S4.6 without changing behaviour:
//!
//! | Sub-module   | Responsibility                                  |
//! |--------------|-------------------------------------------------|
//! | `ast`        | [`CompiledExpr`] newtype + crate-private AST    |
//! | `parser`     | Recursive-descent parser                        |
//! | `literal`    | Numeric-literal parsing (Q32.32)                |
//! | `eval`       | Pure Q32.32 evaluator                           |
//! | `analysis`   | Static passes (e.g. [`collect_channel_refs`])   |
//!
//! Only the parser can construct a [`CompiledExpr`] — the AST variants are
//! crate-private — so "channel symbols resolved at parse time" is
//! enforced in the type system, not by convention.

mod analysis;
mod ast;
mod eval;
mod literal;
mod parser;

#[cfg(test)]
pub(crate) mod test_support;

pub use analysis::collect_channel_refs;
pub use ast::{CompiledExpr, Expr};
pub use eval::eval_expression;
pub use parser::parse_expression;
