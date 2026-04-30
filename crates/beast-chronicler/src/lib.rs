//! Pattern recognition and label assignment for the Beast Evolution
//! Game.
//!
//! S10.5 brought the ingestion + signature + observation surface; S10.6
//! adds manifest-driven labels via [`crate::label::LabelEngine`] and a
//! shared [`crate::confidence`] scoring helper. The read-only query API
//! for the UI layer (S10.7) layers on top of these.
//!
//! # Determinism
//!
//! Per `documentation/INVARIANTS.md` §1, every public surface in this
//! crate is deterministic across runs and platforms:
//!
//! * Signatures are BLAKE3 over a length-prefixed, lexicographically
//!   sorted byte stream of the primitive ids in the snapshot.
//! * Storage uses [`BTreeMap`](std::collections::BTreeMap) /
//!   [`BTreeSet`](std::collections::BTreeSet) so iteration order is a
//!   total function of contents, never of insertion order.
//! * Confidence math runs through [`Q3232`](beast_core::Q3232) saturating
//!   arithmetic — no floats, no wall-clock reads, no OS RNG on the sim
//!   path.
//!
//! Per `documentation/INVARIANTS.md` §2 (Mechanics-Label Separation),
//! human-readable label strings appear *only* in [`crate::label`]; they
//! are never observed by sim systems and are loaded from JSON manifests
//! at startup with zero hardcoded heuristics.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod chronicler;
pub mod confidence;
pub mod event;
pub mod label;
pub mod pattern;
pub mod query;
pub mod snapshot;
pub mod tick_range;

pub use chronicler::Chronicler;
pub use confidence::compute_confidence;
pub use event::{DeathCause, EventKey, LifecycleEvent, LifecycleEventLog};
pub use label::{Label, LabelEngine, LabelEngineError, LabelLoadError, LabelManifest};
pub use pattern::{PatternObservation, PatternSignature};
pub use query::{
    BestiaryEntry, BestiaryFilter, BestiarySortBy, ChroniclerQuery, InMemoryChronicler, LabelId,
    SpeciesId,
};
pub use snapshot::PrimitiveSnapshot;
pub use tick_range::TickRange;
