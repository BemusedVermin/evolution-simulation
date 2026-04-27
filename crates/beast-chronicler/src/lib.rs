//! Pattern recognition and (later) label assignment for the Beast
//! Evolution Game.
//!
//! S10.5 — this story — only covers ingesting [`PrimitiveSnapshot`]s,
//! hashing them into [`PatternSignature`]s, and counting observations
//! across a [`TickRange`]. Label generation lives in S10.6
//! ([`crate::label`] is reserved); the read-only query surface lives in
//! S10.7.
//!
//! # Determinism
//!
//! Per `documentation/INVARIANTS.md` §1, every public surface in this
//! crate is deterministic across runs and platforms:
//!
//! * The signature is BLAKE3 over a length-prefixed, lexicographically
//!   sorted byte stream of the primitive ids in the snapshot.
//! * Storage uses [`BTreeMap`](std::collections::BTreeMap) /
//!   [`BTreeSet`](std::collections::BTreeSet) so iteration order is a
//!   total function of contents, never of insertion order.
//! * No floats, no wall-clock reads, no OS RNG.
//!
//! Per `documentation/INVARIANTS.md` §2 (Mechanics-Label Separation),
//! this crate stores raw signatures + counts only. Human-readable labels
//! are *not* introduced here — they arrive in S10.6 from a
//! manifest-driven catalog.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod chronicler;
pub mod pattern;
pub mod snapshot;
pub mod tick_range;

pub use chronicler::Chronicler;
pub use pattern::{PatternObservation, PatternSignature};
pub use snapshot::PrimitiveSnapshot;
pub use tick_range::TickRange;
