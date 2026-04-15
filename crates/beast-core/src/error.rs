//! Shared error type for `beast-core` and downstream crates that want a common
//! `Result` alias at the foundation layer.
//!
//! Higher layers will likely grow their own richer error types; those should
//! convert from [`Error`] via `#[from]` so foundation errors propagate
//! cleanly without being flattened into strings.

use thiserror::Error;

/// Error variants produced by `beast-core` primitives.
///
/// The variants intentionally describe *classes* of failure rather than leaking
/// specifics from upstream libraries (`fixed`, `rand_xoshiro`). Where we do
/// wrap a foreign error, we carry just enough context to reproduce the issue.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fixed-point conversion failed (value out of Q32.32 range).
    #[error("fixed-point conversion out of range: {0}")]
    FixedPointRange(String),

    /// PRNG seeding or stream-splitting failed.
    #[error("prng error: {0}")]
    Prng(String),

    /// Invariant violation in `beast-core` — indicates a programmer bug, not a
    /// recoverable runtime condition.
    #[error("invariant violation: {0}")]
    Invariant(&'static str),

    /// A value that must be finite / in-range was not.
    #[error("value out of range: {0}")]
    OutOfRange(&'static str),
}

/// Convenience `Result` alias used throughout `beast-core`.
pub type Result<T> = core::result::Result<T, Error>;
