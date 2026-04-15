//! Runtime primitive emission.
//!
//! [`PrimitiveEffect`] is the per-tick record emitted by the phenotype
//! interpreter (Sprint 4) and consumed by the Chronicler when it clusters
//! signatures into labelled abilities.
//!
//! Only the shape is defined here. Sprint 2 needs the type so downstream
//! crates can compile against it, but the interpreter that produces effects
//! is out of scope for this sprint.

use std::collections::BTreeMap;

use beast_core::{EntityId, Q3232};

use crate::manifest::Provenance;

/// A single primitive emission.
///
/// `primitive_id` must correspond to a manifest registered in the
/// [`crate::PrimitiveRegistry`]. `parameters` uses a [`BTreeMap`] so
/// iteration and hashing are order-stable — a prerequisite for the
/// determinism invariant when effect streams are hashed into per-tick state
/// digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveEffect {
    /// Primitive manifest id.
    pub primitive_id: String,
    /// Channel ids that composed to produce this emission.
    pub source_channels: Vec<String>,
    /// Parameter values, keyed by parameter name.
    pub parameters: BTreeMap<String, Q3232>,
    /// Entity emitting the effect.
    pub emitter: EntityId,
    /// Origin of the primitive (propagated from the manifest so downstream
    /// auditing can trace effects to their mod / genesis origin).
    pub provenance: Provenance,
}
