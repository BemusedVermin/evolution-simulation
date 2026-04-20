//! Environmental affordance filter (S4.2 — issue #56).
//!
//! Thin interpreter-side wrapper over
//! [`beast_channels::evaluate_expression_conditions`]. Given an
//! [`crate::Environment`] and a slice of [`crate::InterpreterHook`]s, returns
//! the sorted subset of hook ids whose `expression_conditions` pass.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §5.0c and §6.2.

// Implementation — see story S4.2 (#56).
