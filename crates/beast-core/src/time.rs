//! Tick counter and time primitives. Populated in Story 1.3.

use serde::{Deserialize, Serialize};

/// Monotonic tick counter. Implementation lands in Story 1.3.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct TickCounter(pub u64);

impl TickCounter {
    /// Tick zero — world creation instant.
    pub const ZERO: Self = Self(0);
}
