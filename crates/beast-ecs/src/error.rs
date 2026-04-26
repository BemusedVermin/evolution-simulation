//! Crate-local error type.
//!
//! `beast-ecs` exposes a narrow error surface so callers in `beast-sim`
//! can `?`-propagate without depending on `specs`'s error types. Variants
//! are added as stories land; stubs stay `#[non_exhaustive]` so new ones
//! don't break downstream matches.

use thiserror::Error;

/// Crate error type. Kept `#[non_exhaustive]` so downstream code must
/// handle the catch-all match arm — we'll add variants as S5.2–S5.7 land
/// without forcing breaking changes.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EcsError {
    /// A [`crate::system::System::run`] call reported an error. The
    /// inner string is the system-supplied message; structured
    /// classification will be added once we have concrete systems to
    /// categorise.
    #[error("system `{system}` failed: {message}")]
    SystemRunFailed {
        /// Name of the offending system (from
        /// [`crate::system::System::name`]).
        system: &'static str,
        /// Opaque message returned by the system.
        message: String,
    },
    /// A [`crate::components::PhenotypeComponent::try_new`] caller
    /// passed an effect list that was not sorted by
    /// `(primitive_id, body_site)`. The interpreter emits sorted
    /// output by contract; this error exists for callers that build
    /// phenotypes outside the interpreter (tests, save-loaders) so
    /// the determinism invariant is enforced in release builds.
    #[error(
        "phenotype effects out of order at index {index}: \
         downstream systems hash phenotypes in visit order, so an \
         unsorted list breaks INVARIANTS §1 (deterministic iteration)"
    )]
    PhenotypeNotSorted {
        /// Zero-based position of the first out-of-order pair (the
        /// effect at `index + 1` was less than the effect at `index`).
        index: usize,
    },
}

/// Crate-level `Result` alias. Every public API that can fail uses this.
pub type Result<T> = core::result::Result<T, EcsError>;
