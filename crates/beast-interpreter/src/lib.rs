//! Beast Evolution Game — Layer 2 phenotype interpreter.
//!
//! The interpreter is the formal composition law: it converts an evolved
//! [`beast_genome::Genome`] plus an [`Environment`] into a deterministic
//! `Set<PrimitiveEffect>` by consulting the channel and primitive registries.
//! See `documentation/systems/11_phenotype_interpreter.md` for the canonical
//! specification and `documentation/INVARIANTS.md` for the contracts this
//! crate enforces.
//!
//! **Non-negotiable invariants enforced by every module in this crate:**
//!
//! * Mechanics-label separation (INVARIANTS §2): only primitive ids and
//!   parameters leave the interpreter. Named abilities never appear here.
//! * Determinism (INVARIANTS §1): all arithmetic in [`beast_core::Q3232`];
//!   iteration over `BTreeMap` / sorted keys only; no wall-clock reads, no OS
//!   RNG, no floats in the hot path.
//! * Scale-band unification (INVARIANTS §5): one pipeline across macro hosts
//!   and micro pathogens; scale-band filtering is the first stage
//!   ([`scale_band`]).
//! * Registry-driven mutability (INVARIANTS §3): every channel and primitive
//!   reference resolves through the shared registries at load / tick time; no
//!   hardcoded ids in system code.
//!
//! # Sprint scope (S4 — epic [#16])
//!
//! | Story | Module                       | Issue |
//! |-------|------------------------------|-------|
//! | 4.1   | [`scale_band`]               | #55   |
//! | 4.2   | [`expression`]               | #56   |
//! | 4.3   | [`composition`]              | #57   |
//! | 4.4   | [`parameter_map`], [`emission`] | #58 |
//! | 4.5   | [`body_map`]                 | #59   |
//! | 4.6   | `tests/determinism.rs` (integration) | #60 |
//!
//! The top-level entry point [`interpreter::interpret_phenotype`] is wired up
//! incrementally as each story lands.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

/// Per-body-site channel aggregation (Story 4.5 — issue #59).
pub mod body_map;
/// Hook composition + ordering (Story 4.3 — issue #57).
pub mod composition;
/// Primitive emission (Story 4.4 — issue #58).
pub mod emission;
/// Typed [`InterpreterError`] / [`Result`] for fallible interpreter ops.
pub mod error;
/// Affordance-driven hook expression filter (Story 4.2 — issue #56).
pub mod expression;
/// Top-level [`interpret_phenotype`] orchestrator wiring all stages.
pub mod interpreter;
/// Parameter expression parser + evaluator (Story 4.4 — issue #58).
pub mod parameter_map;
/// Phenotype data types: body sites, regions, life stages, environment.
pub mod phenotype;
/// Scale-band gating (Story 4.1 — issue #55).
pub mod scale_band;

pub use body_map::{
    aggregate_channel_globally, aggregate_to_global, per_site_channel_values, AggregationStrategy,
};
pub use composition::{
    resolve_hooks, CompositionKind, EmitSpec, FiredHook, HookId, InterpreterHook,
};
pub use emission::emit_primitives;
pub use error::{InterpreterError, Result};
pub use expression::filter_hooks_by_affordances;
pub use interpreter::interpret_phenotype;
pub use parameter_map::{
    collect_channel_refs, eval_expression, parse_expression, CompiledExpr, Expr,
};
pub use phenotype::{BodyRegion, BodySite, Environment, LifeStage, ResolvedPhenotype};
pub use scale_band::apply_scale_band_filter;
