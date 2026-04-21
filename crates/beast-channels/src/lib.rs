//! Beast Evolution Game — Layer 1 channel registry and manifest loader.
//!
//! This crate is the runtime home of the **Channel Manifest** contract
//! described in `documentation/schemas/channel_manifest.schema.json` and
//! the architectural invariants in `documentation/INVARIANTS.md`:
//!
//! * **Channel Registry Monolithicism** — a single authoritative registry
//!   indexes every channel (core, mod, genesis). Nothing downstream hardcodes
//!   channel IDs or composition rules; they consult the registry.
//! * **Determinism** — every numeric value that participates in sim math is
//!   stored as [`beast_core::Q3232`]. JSON-side `f64` values are converted
//!   exactly once at load time (configuration, not sim state), and the lint
//!   `clippy::float_arithmetic` keeps arithmetic from sneaking back into the
//!   hot path.
//! * **Sorted iteration** — registries expose iteration via [`BTreeMap`] so
//!   ordering never depends on hash randomization.
//!
//! The crate has three primary surfaces:
//!
//! 1. [`manifest`] — strongly typed representations of a channel manifest
//!    with a two-stage loader (JSON Schema validation, then semantic parse).
//! 2. [`registry`] — [`ChannelRegistry`] with by-id and by-family indices.
//! 3. [`composition`] — deterministic evaluator for composition hooks over
//!    Q32.32 channel values.
//!
//! [`BTreeMap`]: std::collections::BTreeMap

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod composition;
pub mod expression;
pub mod fingerprint;
pub mod manifest;
pub mod registry;
pub mod schema;

pub use composition::{evaluate_hook, CompositionHook, CompositionKind, HookOutcome};
pub use expression::{evaluate_expression_conditions, ExpressionCondition, ExpressionContext};
pub use fingerprint::RegistryFingerprint;
pub use manifest::{
    BoundsPolicy, ChannelFamily, ChannelManifest, CorrelationEntry, MutationKernel, Provenance,
    Range, ScaleBand,
};
pub use registry::{ChannelRegistry, RegistryError};
pub use schema::{ChannelLoadError, CHANNEL_MANIFEST_SCHEMA};
