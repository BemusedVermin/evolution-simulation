//! Crate-local error type. `#[non_exhaustive]` so later stories can add
//! variants without breaking downstream matches.

use thiserror::Error;

/// Top-level simulation error. Every public API in this crate that can
/// fail returns `Result<T>` (aliased below).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SimError {
    /// A system in the schedule reported an error during its `run` call.
    #[error("ecs error: {0}")]
    Ecs(#[from] beast_ecs::EcsError),
}

/// Crate-level `Result` alias.
pub type Result<T> = core::result::Result<T, SimError>;
