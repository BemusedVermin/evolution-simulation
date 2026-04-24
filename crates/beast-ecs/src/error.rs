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
}

/// Crate-level `Result` alias. Every public API that can fail uses this.
pub type Result<T> = core::result::Result<T, EcsError>;
