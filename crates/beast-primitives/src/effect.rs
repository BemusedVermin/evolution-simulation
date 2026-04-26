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

use beast_core::{BodySite, EntityId, Q3232};
use serde::{Deserialize, Serialize};

use crate::manifest::Provenance;

/// A single primitive emission.
///
/// `primitive_id` must correspond to a manifest registered in the
/// [`crate::PrimitiveRegistry`]. `parameters` uses a [`BTreeMap`] so
/// iteration and hashing are order-stable — a prerequisite for the
/// determinism invariant when effect streams are hashed into per-tick state
/// digests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveEffect {
    /// Primitive manifest id.
    pub primitive_id: String,
    /// Body site this emission applies to, when the firing hook uses
    /// `body_site_applicable` channels. `None` means the emission is
    /// global — it applies to the creature as a whole, not to any single
    /// anatomical region.
    ///
    /// Per the phenotype-interpreter spec (§6.0B / §6.2B) the dedup key
    /// is `(primitive_id, body_site)`, so the same primitive can be
    /// emitted concurrently for different sites without collapsing.
    pub body_site: Option<BodySite>,
    /// Channel ids that composed to produce this emission.
    ///
    /// **Ordering contract**: callers must populate this in
    /// lexicographic ascending order. Determinism gates and any
    /// downstream Chronicler / save-layer hashing treat this as a
    /// canonical sequence, so the same emission produced by two
    /// independent code paths must yield the same byte sequence.
    /// The phenotype interpreter satisfies this contract by
    /// iterating its hook map (a `BTreeMap`) and pushing channel
    /// ids in iteration order, which is sorted by definition.
    /// New producers should either (a) build via a `BTreeSet<String>`
    /// then `into_iter().collect()`, or (b) call
    /// `source_channels.sort()` before constructing the
    /// `PrimitiveEffect`.
    pub source_channels: Vec<String>,
    /// Parameter values, keyed by parameter name.
    pub parameters: BTreeMap<String, Q3232>,
    /// Activation cost for this emission, evaluated from the manifest's
    /// [`crate::CostFunction`] against `parameters` at emission time.
    ///
    /// First-class field rather than a `_activation_cost` sentinel key in
    /// `parameters` — manifests therefore cannot override the merge
    /// strategy for activation cost, which always sums across hooks that
    /// emit the same primitive in one tick.
    pub activation_cost: Q3232,
    /// Entity emitting the effect.
    pub emitter: EntityId,
    /// Origin of the primitive (propagated from the manifest so downstream
    /// auditing can trace effects to their mod / genesis origin).
    pub provenance: Provenance,
}
