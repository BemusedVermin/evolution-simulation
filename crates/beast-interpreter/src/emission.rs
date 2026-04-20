//! PrimitiveEffect emission, deduplication, and merge (S4.4 — issue #58).
//!
//! Consumes [`crate::FiredHook`]s from the resolver ([`crate::composition`]),
//! evaluates each [`crate::EmitSpec`]'s parameter expressions against the
//! phenotype's channel vector, materialises one
//! [`beast_primitives::PrimitiveEffect`] per (primitive_id, body_site) pair,
//! and merges duplicates using the per-parameter `merge_strategy` declared in
//! the primitive manifest.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §6.2 and §6.2B.

// Implementation — see story S4.4 (#58).
