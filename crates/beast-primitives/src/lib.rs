//! Beast Evolution Game — Layer 1 primitive effect registry.
//!
//! Primitives are the atomic vocabulary that the phenotype interpreter will
//! emit to the world. They are never named abilities in themselves
//! ("echolocation", "venom"); those labels are assigned post-hoc by the
//! Chronicler over recurring primitive clusters — see
//! `documentation/INVARIANTS.md` §2 (Mechanics-Label Separation).
//!
//! This crate mirrors `beast-channels` in shape:
//!
//! * [`manifest`] holds the strongly typed representation of a primitive
//!   manifest, loaded via the two-stage (JSON Schema + semantic) validator in
//!   [`schema`].
//! * [`registry`] is a deterministic [`BTreeMap`]-backed index keyed by id
//!   with a secondary index by [`PrimitiveCategory`].
//! * [`cost`] evaluates a manifest's cost function deterministically over
//!   Q32.32 parameter values. Because cost formulas use fractional power-law
//!   scaling (e.g. `force^1.5`), this module also provides a small
//!   fixed-point `exp`/`ln` pair used to implement `pow`.
//!
//! [`BTreeMap`]: std::collections::BTreeMap

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

/// `PrimitiveCategory` taxonomy + `Modality` flags.
pub mod category;
/// Deterministic cost-function evaluator over Q32.32 parameter values.
pub mod cost;
/// `PrimitiveEffect`: an emitted (id, parameters) record from the interpreter.
pub mod effect;
/// Strongly typed primitive-manifest representation.
pub mod manifest;
/// Fixed-point `exp` / `ln` / `pow` helpers used by the cost evaluator.
pub(crate) mod math;
/// `PrimitiveRegistry`: deterministic [`std::collections::BTreeMap`]-backed index.
pub mod registry;
/// Two-stage (JSON Schema + semantic) manifest validator.
pub mod schema;

pub use category::{Modality, PrimitiveCategory};
pub use cost::{evaluate_cost, CostEvalError};
pub use effect::PrimitiveEffect;
pub use manifest::{
    CompatibilityEntry, CostFunction, MergeStrategy, ObservableSignature, ParameterScaling,
    ParameterSpec, ParameterType, PrimitiveManifest, Provenance,
};
pub use registry::{PrimitiveRegistry, RegistryError};
pub use schema::{PrimitiveLoadError, PRIMITIVE_MANIFEST_SCHEMA};
