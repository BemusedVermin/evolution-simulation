//! `ReplayJournal` — input-sequence log for deterministic re-execution
//! (S7.3 — issue #131).
//!
//! Together with a [`crate::SaveFile`], a `ReplayJournal` is enough to
//! reproduce any future tick byte-identically: load the save, then on
//! each tick drain the events recorded for that tick before invoking
//! `Simulation::tick`. The pair is the foundation the M2 Determinism
//! gate (S7.6) builds on.
//!
//! # Sprint scope
//!
//! The MVP has no player input — the avatar/UI stories land in S13+ —
//! so [`InputEvent`] starts with a single `Noop` variant. The struct,
//! the recording API, and the round-trip serialization are all real;
//! only the variant set is a placeholder. Real variants land where the
//! input source does, and the `#[non_exhaustive]` annotation means
//! existing match sites won't break.
//!
//! # Determinism contract
//!
//! * Events are stored in `BTreeMap<u64, Vec<InputEvent>>` keyed by
//!   tick. Tick keys iterate in ascending order; per-tick events
//!   iterate in insertion order. JSON serialization preserves both.
//! * `world_seed` is stamped into the journal so a save/journal pair
//!   can be sanity-checked: replay against a save with a different
//!   `world_seed` is always a bug.
//! * `format_version` mirrors `SaveFile::format_version` so the same
//!   migration registry can upgrade journals when the wire format
//!   shifts.

use std::collections::BTreeMap;

use beast_core::TickCounter;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema version stamped into every [`ReplayJournal`]. Bumped
/// independently of [`crate::save::SAVE_FORMAT_VERSION`] because the
/// journal layout can drift on its own — adding an `InputEvent`
/// variant does not necessarily change the save envelope.
pub const REPLAY_FORMAT_VERSION: &str = "0.1.0";

/// Recorded events that drove the simulation forward, keyed by the
/// tick at which they were observed. Together with a [`crate::SaveFile`]
/// captured at tick 0 (or any earlier tick), this is sufficient to
/// reproduce every later tick byte-identically.
///
/// Sized cheaply enough to keep in memory for any reasonable
/// MVP run; deferred-streaming-to-disk is tracked as a future
/// optimisation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayJournal {
    /// Schema version. Must equal [`REPLAY_FORMAT_VERSION`].
    pub format_version: String,
    /// Master world seed of the run the journal records. Verified
    /// against the loaded save's `world_seed` before replay starts —
    /// a mismatch is always a bug, never a recoverable case.
    pub world_seed: u64,
    /// Recorded events per tick. Ascending tick order, insertion order
    /// per tick.
    pub events: BTreeMap<u64, Vec<InputEvent>>,
}

impl ReplayJournal {
    /// Build a fresh journal for a run with the given `world_seed`. No
    /// events recorded.
    #[must_use]
    pub fn new(world_seed: u64) -> Self {
        Self {
            format_version: REPLAY_FORMAT_VERSION.to_string(),
            world_seed,
            events: BTreeMap::new(),
        }
    }

    /// Append `event` under the given tick. Multiple calls with the
    /// same tick stack in insertion order — a future replay will play
    /// them back in that same order.
    pub fn record(&mut self, tick: TickCounter, event: InputEvent) {
        self.events.entry(tick.raw()).or_default().push(event);
    }

    /// All events that should fire at `tick`, in insertion order.
    /// Returns an empty slice when no events were recorded for the
    /// tick.
    #[must_use]
    pub fn events_at(&self, tick: TickCounter) -> &[InputEvent] {
        self.events
            .get(&tick.raw())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Total number of recorded events across every tick. Useful for
    /// summary logging.
    #[must_use]
    pub fn total_events(&self) -> usize {
        self.events.values().map(Vec::len).sum()
    }

    /// Render the journal to canonical JSON. `BTreeMap` keys serialize
    /// in ascending order; field order follows declaration order.
    /// Two equal journals serialize to byte-identical output.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError::Json`] on encoder failure.
    pub fn to_json(&self) -> Result<String, ReplayError> {
        serde_json::to_string(self).map_err(ReplayError::Json)
    }

    /// Restore a journal from JSON produced by [`Self::to_json`].
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError::Json`] on parse failure or unknown fields
    /// (the envelope uses `deny_unknown_fields`). Returns
    /// [`ReplayError::UnsupportedVersion`] if the parsed
    /// `format_version` does not match [`REPLAY_FORMAT_VERSION`].
    pub fn from_json(s: &str) -> Result<Self, ReplayError> {
        let journal: ReplayJournal = serde_json::from_str(s).map_err(ReplayError::Json)?;
        if journal.format_version != REPLAY_FORMAT_VERSION {
            return Err(ReplayError::UnsupportedVersion {
                expected: REPLAY_FORMAT_VERSION,
                found: journal.format_version,
            });
        }
        Ok(journal)
    }
}

/// One recorded input event. Closed today (`Noop` placeholder), but
/// `#[non_exhaustive]` so future variants — avatar movement, breeding
/// pair selection, dialog choice, time-warp toggle — slot in without
/// forcing wildcard arms on every match site.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum InputEvent {
    /// Placeholder. The MVP has no player input yet — real variants
    /// land per the avatar/UI sprints (S13+). Kept so the journal
    /// data structure is exercisable today.
    Noop,
}

/// Errors produced by [`ReplayJournal`] (de)serialization.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReplayError {
    /// `serde_json` failed to encode or decode the journal.
    #[error("replay journal (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    /// Journal `format_version` does not match the binary's expected
    /// version. Migrations land in S7.5 and will narrow this case.
    #[error("unsupported replay format version: expected {expected}, found {found}")]
    UnsupportedVersion {
        /// Version this build understands ([`REPLAY_FORMAT_VERSION`]).
        expected: &'static str,
        /// Version the journal declared.
        found: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_empty_for_given_seed() {
        let j = ReplayJournal::new(0xDEAD_BEEF);
        assert_eq!(j.world_seed, 0xDEAD_BEEF);
        assert_eq!(j.format_version, REPLAY_FORMAT_VERSION);
        assert!(j.events.is_empty());
        assert_eq!(j.total_events(), 0);
    }

    #[test]
    fn record_appends_in_insertion_order_per_tick() {
        let mut j = ReplayJournal::new(0);
        j.record(TickCounter::new(5), InputEvent::Noop);
        j.record(TickCounter::new(5), InputEvent::Noop);
        j.record(TickCounter::new(5), InputEvent::Noop);
        assert_eq!(j.events_at(TickCounter::new(5)).len(), 3);
        assert_eq!(j.total_events(), 3);
    }

    #[test]
    fn events_at_returns_empty_slice_for_unrecorded_tick() {
        let j = ReplayJournal::new(0);
        assert!(j.events_at(TickCounter::new(99)).is_empty());
    }

    #[test]
    fn json_round_trip_is_lossless() {
        let mut j = ReplayJournal::new(7);
        j.record(TickCounter::new(1), InputEvent::Noop);
        j.record(TickCounter::new(3), InputEvent::Noop);
        j.record(TickCounter::new(3), InputEvent::Noop);
        let s = j.to_json().unwrap();
        let parsed = ReplayJournal::from_json(&s).unwrap();
        assert_eq!(j, parsed);
    }

    #[test]
    fn equal_journals_serialize_to_equal_json() {
        let mut a = ReplayJournal::new(7);
        let mut b = ReplayJournal::new(7);
        for tick in [10u64, 4, 99, 4, 1] {
            a.record(TickCounter::new(tick), InputEvent::Noop);
            b.record(TickCounter::new(tick), InputEvent::Noop);
        }
        // Insertion order across ticks differs intentionally; BTreeMap
        // sorts by tick, so the JSON output should still match.
        assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
    }

    #[test]
    fn json_keys_iterate_in_ascending_tick_order() {
        let mut j = ReplayJournal::new(0);
        // Insert in shuffled order; the BTreeMap normalises.
        for tick in [50u64, 1, 999, 7, 3, 100] {
            j.record(TickCounter::new(tick), InputEvent::Noop);
        }
        let s = j.to_json().unwrap();
        // Crude scan: find each tick key in the order it must appear.
        let pos: Vec<usize> = ["\"1\"", "\"3\"", "\"7\"", "\"50\"", "\"100\"", "\"999\""]
            .iter()
            .map(|key| s.find(key).expect("key missing"))
            .collect();
        let mut sorted = pos.clone();
        sorted.sort_unstable();
        assert_eq!(pos, sorted, "tick keys not in ascending order");
    }

    #[test]
    fn from_json_rejects_unknown_envelope_field() {
        let mut j = ReplayJournal::new(0);
        j.record(TickCounter::new(1), InputEvent::Noop);
        let mut tampered: serde_json::Value = serde_json::from_str(&j.to_json().unwrap()).unwrap();
        tampered["future_field"] = serde_json::json!(1);
        let bad = serde_json::to_string(&tampered).unwrap();
        let err = ReplayJournal::from_json(&bad).expect_err("unknown field should fail");
        assert!(err.to_string().contains("unknown field"), "got: {err}");
    }

    #[test]
    fn from_json_rejects_format_version_mismatch() {
        let j = ReplayJournal::new(0);
        let mut tampered: serde_json::Value = serde_json::from_str(&j.to_json().unwrap()).unwrap();
        tampered["format_version"] = serde_json::json!("9.9.9");
        let bad = serde_json::to_string(&tampered).unwrap();
        let err = ReplayJournal::from_json(&bad).expect_err("version should fail");
        match err {
            ReplayError::UnsupportedVersion { expected, found } => {
                assert_eq!(expected, REPLAY_FORMAT_VERSION);
                assert_eq!(found, "9.9.9");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn input_event_uses_tagged_kind_field_in_json() {
        // Locks in the wire format: a future variant sees `"kind":
        // "<snake_case>"` consistently. Hand-edited replay-fixtures
        // depend on this.
        let s = serde_json::to_string(&InputEvent::Noop).unwrap();
        assert_eq!(s, r#"{"kind":"noop"}"#);
    }
}
