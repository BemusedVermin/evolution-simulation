//! Per-tick timing and budget tracking (S6.4 — issue #119).
//!
//! Timing here is **observation only** — the returned
//! [`TickResult`] feeds logging and future budget-defer logic (S6.7),
//! but never flows back into sim state. INVARIANTS §1 forbids wall-clock
//! reads on the sim path; this module stays clean by never exposing its
//! timing values to the scheduler's per-system decisions — the scheduler
//! calls `stopwatch()` and later writes the result into `TickResult`
//! without reading it for control flow.

use std::collections::BTreeMap;
use std::time::Instant;

use beast_core::TickCounter;
use beast_ecs::SystemStage;

/// Summary of a single tick: what tick number it was, total wall-clock
/// duration, and per-stage breakdown.
///
/// `stage_durations` only contains entries for stages that had at least
/// one system registered; empty stages do not appear.
#[derive(Debug, Clone)]
pub struct TickResult {
    /// The tick counter value **after** the tick completed. For the
    /// first successful tick this is `1`.
    pub tick: TickCounter,
    /// Total wall-clock microseconds for the whole tick (all stages +
    /// the `advance_tick` bump).
    pub duration_us: u64,
    /// Wall-clock microseconds per stage that actually ran. Uses
    /// `BTreeMap` so iteration is deterministic if two tests compare
    /// serialised results.
    pub stage_durations: BTreeMap<SystemStage, u64>,
}

/// Minimal wrapper over `std::time::Instant` so test code can audit
/// every start/stop site. Consumers should not read `raw_start` —
/// the accessor exists only for observability, not control flow.
#[derive(Debug)]
pub(crate) struct Stopwatch {
    started_at: Instant,
}

impl Stopwatch {
    /// Start a new stopwatch at "now".
    pub(crate) fn start() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    /// Elapsed microseconds since `start`. Saturates at `u64::MAX` on
    /// overflow (via `u64::try_from(u128)`) — any tick longer than
    /// ~584,000 years saturates rather than wrapping, which is fine.
    pub(crate) fn elapsed_us(&self) -> u64 {
        let elapsed = self.started_at.elapsed();
        let micros: u128 = elapsed.as_micros();
        u64::try_from(micros).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopwatch_elapsed_is_monotonic_non_negative() {
        let w = Stopwatch::start();
        let a = w.elapsed_us();
        // Sleep rather than busy-spin. Busy-spinning made the test
        // unreliable on loaded CI runners where scheduler yield times
        // could exceed the 10 µs comparison window, and on platforms
        // with coarser `Instant` resolution it would run longer than
        // intended. 50 µs sleep is enough to advance every clock we
        // target, without load-sensitivity.
        std::thread::sleep(std::time::Duration::from_micros(50));
        let b = w.elapsed_us();
        assert!(b >= a, "elapsed went backwards: {a} -> {b}");
    }

    #[test]
    fn tick_result_defaults_cleanly() {
        let t = TickResult {
            tick: TickCounter::new(0),
            duration_us: 0,
            stage_durations: BTreeMap::new(),
        };
        assert_eq!(t.tick.raw(), 0);
        assert_eq!(t.duration_us, 0);
        assert!(t.stage_durations.is_empty());
    }

    #[test]
    fn stage_durations_is_btreemap_for_ordered_iteration() {
        let mut sd: BTreeMap<SystemStage, u64> = BTreeMap::new();
        sd.insert(SystemStage::Ecology, 5);
        sd.insert(SystemStage::InputAndAging, 2);
        sd.insert(SystemStage::Genetics, 3);
        let keys: Vec<SystemStage> = sd.keys().copied().collect();
        assert_eq!(
            keys,
            vec![
                SystemStage::InputAndAging,
                SystemStage::Genetics,
                SystemStage::Ecology
            ]
        );
    }
}
