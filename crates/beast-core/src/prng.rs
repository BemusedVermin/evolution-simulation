//! PRNG wrapper. Populated in Story 1.2.

use serde::{Deserialize, Serialize};

/// Deterministic PRNG. Implementation lands in Story 1.2.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prng {
    // placeholder; replaced in 1.2
    _seed: u64,
}

impl Prng {
    /// Construct from a 64-bit master seed. Placeholder for Story 1.2.
    pub fn from_seed(seed: u64) -> Self {
        Self { _seed: seed }
    }
}

/// Enumeration of per-subsystem PRNG streams. Populated in Story 1.2.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Stream {
    /// Placeholder variant so the enum is non-empty during S1 scaffolding.
    Placeholder,
}
