//! Half-open `[start, end)` tick range used for windowed cluster queries.

use beast_core::TickCounter;
use serde::{Deserialize, Serialize};

/// A half-open range of ticks: `start` inclusive, `end` exclusive.
///
/// `TickCounter` already has `Ord`, so we lean on that for containment
/// instead of inventing range arithmetic.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickRange {
    /// First tick included in the range.
    pub start: TickCounter,
    /// First tick excluded from the range. Must satisfy `end >= start`.
    pub end: TickCounter,
}

impl TickRange {
    /// Construct a range. Returns `None` if `end < start`.
    pub fn new(start: TickCounter, end: TickCounter) -> Option<Self> {
        if end < start {
            return None;
        }
        Some(Self { start, end })
    }

    /// Range containing every possible tick (`[ZERO, MAX)`).
    pub const ALL: Self = Self {
        start: TickCounter::ZERO,
        end: TickCounter::MAX,
    };

    /// Test whether `tick` falls inside this range.
    pub fn contains(self, tick: TickCounter) -> bool {
        tick >= self.start && tick < self.end
    }

    /// Test whether the range `[a, b]` (inclusive) overlaps any tick
    /// inside this range. Used to decide whether a [`PatternObservation`](
    /// crate::PatternObservation) — which spans `[first_tick, last_tick]` —
    /// has any presence in the window.
    pub fn overlaps_inclusive(self, a: TickCounter, b: TickCounter) -> bool {
        // `a..=b` overlaps `[start, end)` iff a < end && b >= start.
        a < self.end && b >= self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_inverted_range() {
        let t = TickCounter::new;
        assert!(TickRange::new(t(10), t(5)).is_none());
        assert!(TickRange::new(t(5), t(10)).is_some());
        assert!(TickRange::new(t(5), t(5)).is_some(), "empty range allowed");
    }

    #[test]
    fn contains_is_half_open() {
        let t = TickCounter::new;
        let r = TickRange::new(t(5), t(10)).unwrap();
        assert!(r.contains(t(5)), "lower bound inclusive");
        assert!(r.contains(t(9)), "interior");
        assert!(!r.contains(t(10)), "upper bound exclusive");
        assert!(!r.contains(t(4)));
    }

    #[test]
    fn overlaps_inclusive_picks_up_observation_spans() {
        let t = TickCounter::new;
        let r = TickRange::new(t(5), t(10)).unwrap();
        // [3, 4] entirely before window: no overlap.
        assert!(!r.overlaps_inclusive(t(3), t(4)));
        // [4, 5] crosses lower bound.
        assert!(r.overlaps_inclusive(t(4), t(5)));
        // [9, 9] sits at last included tick.
        assert!(r.overlaps_inclusive(t(9), t(9)));
        // [10, 12] starts at exclusive upper: no overlap.
        assert!(!r.overlaps_inclusive(t(10), t(12)));
    }

    #[test]
    fn all_contains_every_finite_tick() {
        assert!(TickRange::ALL.contains(TickCounter::ZERO));
        assert!(TickRange::ALL.contains(TickCounter::new(123_456)));
    }
}
