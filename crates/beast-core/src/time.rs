//! Simulation time primitives.
//!
//! The simulation uses **tick count** as its only notion of time. Wall-clock
//! reads are forbidden on the sim path (see `documentation/INVARIANTS.md`
//! §1). One tick is the unit of the schedule defined in
//! `documentation/architecture/ECS_SCHEDULE.md`.
//!
//! `TickCounter` is a `u64` newtype that **saturates** on increment rather
//! than wrapping: at 60 ticks/s, `u64::MAX` ticks is ~9.7 billion years, so
//! overflow in practice means a logic bug, not a legitimate runtime event.
//! Saturating keeps the sim alive and deterministic while surfacing the bug
//! via the tick no longer advancing.

use core::fmt;
use core::ops::{Add, AddAssign, Sub};

use serde::{Deserialize, Serialize};

/// Monotonic simulation tick counter.
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct TickCounter(u64);

impl TickCounter {
    /// Tick zero — world creation instant.
    pub const ZERO: Self = Self(0);

    /// The saturation ceiling. In practice unreachable, but exposed for tests.
    pub const MAX: Self = Self(u64::MAX);

    /// Construct from a raw `u64`.
    #[inline]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The underlying `u64`.
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Advance by one tick, saturating on overflow.
    #[inline]
    pub fn advance(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

    /// Advance by `n` ticks, saturating on overflow.
    #[inline]
    pub fn advance_by(&mut self, n: u64) {
        self.0 = self.0.saturating_add(n);
    }

    /// Tick count, saturating: `self - other`, clamped at zero.
    #[inline]
    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// `true` if this counter is at the saturation ceiling. If this fires in
    /// production it almost certainly indicates a logic bug.
    #[inline]
    pub const fn is_saturated(self) -> bool {
        self.0 == u64::MAX
    }
}

impl fmt::Debug for TickCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tick({})", self.0)
    }
}

impl fmt::Display for TickCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<u64> for TickCounter {
    type Output = Self;
    #[inline]
    fn add(self, rhs: u64) -> Self {
        Self(self.0.saturating_add(rhs))
    }
}

impl AddAssign<u64> for TickCounter {
    #[inline]
    fn add_assign(&mut self, rhs: u64) {
        self.advance_by(rhs);
    }
}

impl Sub for TickCounter {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_and_advance() {
        let mut t = TickCounter::ZERO;
        assert_eq!(t.raw(), 0);
        t.advance();
        assert_eq!(t.raw(), 1);
        t.advance_by(9);
        assert_eq!(t.raw(), 10);
    }

    #[test]
    fn advance_saturates() {
        let mut t = TickCounter::new(u64::MAX - 1);
        t.advance();
        assert!(t.is_saturated());
        t.advance();
        assert!(t.is_saturated());
        assert_eq!(t.raw(), u64::MAX);
    }

    #[test]
    fn add_saturates() {
        let t = TickCounter::new(u64::MAX - 5);
        let r = t + 100;
        assert_eq!(r, TickCounter::MAX);
    }

    #[test]
    fn sub_clamps_at_zero() {
        let a = TickCounter::new(3);
        let b = TickCounter::new(10);
        assert_eq!(a - b, TickCounter::ZERO);
        assert_eq!(b - a, TickCounter::new(7));
    }

    #[test]
    fn ordering_is_numeric() {
        assert!(TickCounter::new(1) < TickCounter::new(2));
    }

    #[test]
    fn serde_transparent() {
        let t = TickCounter::new(42);
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "42");
        let back: TickCounter = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }
}
