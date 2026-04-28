//! [`Chronicler`] — accumulates pattern observations from primitive
//! snapshots and (S10.6) holds the latest manifest-driven label
//! assignments. The read API for the UI layer (S10.7) layers on top.

use std::collections::BTreeMap;

use beast_core::TickCounter;

use crate::label::{Label, LabelEngine};
use crate::pattern::{PatternObservation, PatternSignature};
use crate::snapshot::PrimitiveSnapshot;
use crate::tick_range::TickRange;

/// Pattern observation accumulator with cached label assignments.
///
/// Stores one [`PatternObservation`] per unique [`PatternSignature`] that
/// has ever been ingested. Iteration order over the storage is the
/// natural total order of `PatternSignature` (lexicographic over its 32
/// bytes), which keeps tests and downstream UI views deterministic.
///
/// Labels are *derived* from observations + a [`LabelEngine`] via
/// [`Self::assign_labels`]. They are not produced by `ingest()` — per
/// `INVARIANTS.md` §2, labels remain off the sim path until an
/// explicit assignment pass runs them against the manifest catalog.
#[derive(Clone, Debug, Default)]
pub struct Chronicler {
    observations: BTreeMap<PatternSignature, PatternObservation>,
    labels: BTreeMap<PatternSignature, Label>,
}

impl Chronicler {
    /// Construct an empty chronicler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest one snapshot.
    ///
    /// * Hashes the snapshot's primitive set into a [`PatternSignature`].
    /// * On first sight, creates a [`PatternObservation`] with `count = 1`,
    ///   `first_tick = last_tick = snapshot.tick`, and stores the
    ///   primitive set so reverse-lookups stay O(1).
    /// * On subsequent sight, increments `count` (saturating at `u64::MAX`)
    ///   and *expands* the `[first_tick, last_tick]` span:
    ///   - `first_tick = min(first_tick, snapshot.tick)`
    ///   - `last_tick  = max(last_tick,  snapshot.tick)`
    ///
    ///   So out-of-order ingestion (older ticks arriving after newer
    ///   ones) keeps the span well-formed; the invariant
    ///   `first_tick <= last_tick` always holds.
    ///
    /// Empty snapshots (no primitives emitted) are tracked under the
    /// "all-empty" signature — that's a valid pattern that means "this
    /// entity was inert this tick".
    pub fn ingest(&mut self, snapshot: &PrimitiveSnapshot) {
        let signature = PatternSignature::from_sorted_set(&snapshot.primitives);
        let entry = self
            .observations
            .entry(signature)
            .or_insert_with(|| PatternObservation {
                signature,
                count: 0,
                first_tick: snapshot.tick,
                last_tick: snapshot.tick,
                primitives: snapshot.primitives.clone(),
            });
        entry.count = entry.count.saturating_add(1);
        if snapshot.tick < entry.first_tick {
            entry.first_tick = snapshot.tick;
        }
        if snapshot.tick > entry.last_tick {
            entry.last_tick = snapshot.tick;
        }
    }

    /// All [`PatternObservation`]s whose `[first_tick, last_tick]` span
    /// overlaps `window`. Iteration order is by `PatternSignature` —
    /// the natural total order of the underlying `BTreeMap`.
    ///
    /// Returns an owned `Vec` of references rather than `impl Iterator`
    /// so callers (notably the S10.7 query API) can name the type, hold
    /// it across function boundaries, and re-iterate without rebuilding
    /// the filter. Internal storage is `BTreeMap`-backed, so the
    /// allocation here is one Vec of references — not a clone of the
    /// observations themselves.
    pub fn cluster(&self, window: TickRange) -> Vec<&PatternObservation> {
        self.observations
            .values()
            .filter(|obs| window.overlaps_inclusive(obs.first_tick, obs.last_tick))
            .collect()
    }

    /// Read-only access to the full observation index.
    pub fn observations(&self) -> &BTreeMap<PatternSignature, PatternObservation> {
        &self.observations
    }

    /// Number of unique signatures observed so far.
    pub fn unique_pattern_count(&self) -> usize {
        self.observations.len()
    }

    /// Total ingestions across every signature.
    pub fn total_ingested(&self) -> u64 {
        self.observations.values().map(|o| o.count).sum()
    }

    /// Look up a single observation by signature.
    pub fn observation(&self, signature: &PatternSignature) -> Option<&PatternObservation> {
        self.observations.get(signature)
    }

    /// Highest `last_tick` across all observations, or `TickCounter::ZERO`
    /// if the chronicler has seen no snapshots yet.
    pub fn last_observed_tick(&self) -> TickCounter {
        self.observations
            .values()
            .map(|o| o.last_tick)
            .max()
            .unwrap_or(TickCounter::ZERO)
    }

    /// Run the manifest-driven [`LabelEngine`] against every stored
    /// observation, replacing the cached label index.
    ///
    /// Each pass starts from an empty index so labels whose backing
    /// pattern slipped below `min_confidence` (because newer
    /// observations diluted their frequency, or their stability span
    /// did not keep pace with the sim clock) drop out. Iteration is
    /// over the `BTreeMap`'s sorted key set, so the resulting label
    /// index is identical between two equally-ingested chroniclers
    /// regardless of insertion order.
    ///
    /// `current_tick` is normally [`Self::last_observed_tick`] but
    /// callers running the assignment ahead of the world clock can
    /// supply a different value.
    pub fn assign_labels(&mut self, engine: &LabelEngine, current_tick: TickCounter) {
        let total = self.total_ingested();
        let mut next: BTreeMap<PatternSignature, Label> = BTreeMap::new();
        for (signature, observation) in &self.observations {
            if let Some(label) = engine.assign(observation, total, current_tick) {
                next.insert(*signature, label);
            }
        }
        self.labels = next;
    }

    /// Read-only view of the cached label index.
    ///
    /// Empty until [`Self::assign_labels`] has been called at least
    /// once. Iteration order is the natural total order of
    /// [`PatternSignature`].
    pub fn labels(&self) -> &BTreeMap<PatternSignature, Label> {
        &self.labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beast_core::EntityId;
    use std::collections::BTreeSet;

    fn snap(tick: u64, entity: u32, primitives: &[&str]) -> PrimitiveSnapshot {
        PrimitiveSnapshot::new(
            TickCounter::new(tick),
            EntityId::new(entity),
            primitives.iter().copied(),
        )
    }

    #[test]
    fn ingest_creates_observation_on_first_sight() {
        let mut c = Chronicler::new();
        c.ingest(&snap(7, 1, &["echo", "spatial"]));
        assert_eq!(c.unique_pattern_count(), 1);
        assert_eq!(c.total_ingested(), 1);
        let obs = c.observations().values().next().unwrap();
        assert_eq!(obs.count, 1);
        assert_eq!(obs.first_tick, TickCounter::new(7));
        assert_eq!(obs.last_tick, TickCounter::new(7));
        assert_eq!(obs.primitives.len(), 2);
    }

    #[test]
    fn ingest_increments_existing_observation() {
        let mut c = Chronicler::new();
        c.ingest(&snap(10, 1, &["a", "b"]));
        c.ingest(&snap(20, 2, &["b", "a"])); // same set, different entity / tick
        assert_eq!(c.unique_pattern_count(), 1);
        let obs = c.observations().values().next().unwrap();
        assert_eq!(obs.count, 2);
        assert_eq!(obs.first_tick, TickCounter::new(10));
        assert_eq!(obs.last_tick, TickCounter::new(20));
    }

    #[test]
    fn out_of_order_ingest_keeps_first_le_last() {
        // Reverse-order ingestion of the same signature must keep
        // first_tick = min, last_tick = max — never inverted.
        let mut c = Chronicler::new();
        c.ingest(&snap(20, 1, &["a"])); // first sight: first = last = 20
        c.ingest(&snap(5, 2, &["a"])); // older tick lands second
        c.ingest(&snap(50, 3, &["a"])); // newer tick lands third
        let obs = c.observations().values().next().unwrap();
        assert_eq!(obs.count, 3);
        assert_eq!(obs.first_tick, TickCounter::new(5));
        assert_eq!(obs.last_tick, TickCounter::new(50));
    }

    #[test]
    fn distinct_primitive_sets_keep_separate_signatures() {
        let mut c = Chronicler::new();
        c.ingest(&snap(1, 1, &["a", "b"]));
        c.ingest(&snap(2, 1, &["a", "c"]));
        assert_eq!(c.unique_pattern_count(), 2);
        assert_eq!(c.total_ingested(), 2);
    }

    #[test]
    fn empty_snapshot_is_recorded_under_empty_signature() {
        let mut c = Chronicler::new();
        let empty = PrimitiveSnapshot {
            tick: TickCounter::new(5),
            entity: EntityId::new(1),
            primitives: BTreeSet::new(),
        };
        c.ingest(&empty);
        assert_eq!(c.unique_pattern_count(), 1);
        let obs = c.observations().values().next().unwrap();
        assert!(obs.primitives.is_empty());
    }

    #[test]
    fn cluster_filters_by_window() {
        let mut c = Chronicler::new();
        c.ingest(&snap(10, 1, &["a"])); // first/last = 10
        c.ingest(&snap(50, 1, &["b"])); // first/last = 50
        c.ingest(&snap(100, 1, &["c"])); // first/last = 100
        let window = TickRange::new(TickCounter::new(40), TickCounter::new(60)).unwrap();
        let names: Vec<_> = c
            .cluster(window)
            .into_iter()
            .flat_map(|o| o.primitives.iter().cloned())
            .collect();
        assert_eq!(names, vec!["b".to_string()]);
    }

    #[test]
    fn cluster_iteration_order_is_deterministic() {
        // Two chroniclers built in different ingestion orders should
        // produce identical cluster outputs over the same window.
        let mut a = Chronicler::new();
        let mut b = Chronicler::new();
        let inputs: Vec<PrimitiveSnapshot> = (0..50)
            .map(|i| snap(i, (i % 7) as u32, &[&format!("p{}", i % 5)]))
            .collect();

        for s in &inputs {
            a.ingest(s);
        }
        for s in inputs.iter().rev() {
            b.ingest(s);
        }
        let bytes_a: Vec<u8> = a
            .cluster(TickRange::ALL)
            .into_iter()
            .flat_map(|o| o.signature.0.into_iter())
            .collect();
        let bytes_b: Vec<u8> = b
            .cluster(TickRange::ALL)
            .into_iter()
            .flat_map(|o| o.signature.0.into_iter())
            .collect();
        assert_eq!(bytes_a, bytes_b);
    }

    #[test]
    fn last_observed_tick_tracks_max_last_tick() {
        let mut c = Chronicler::new();
        assert_eq!(c.last_observed_tick(), TickCounter::ZERO);
        c.ingest(&snap(7, 1, &["a"]));
        c.ingest(&snap(3, 2, &["b"]));
        assert_eq!(c.last_observed_tick(), TickCounter::new(7));
    }

    #[test]
    fn count_saturates_at_u64_max() {
        let mut c = Chronicler::new();
        let key = snap(1, 1, &["a"]);
        c.ingest(&key);
        // Tweak the count so we hit saturation in one extra step.
        let sig = PatternSignature::from_sorted_set(&key.primitives);
        c.observations.get_mut(&sig).unwrap().count = u64::MAX;
        c.ingest(&key);
        assert_eq!(c.observation(&sig).unwrap().count, u64::MAX);
    }

    #[test]
    fn labels_starts_empty_until_assign_runs() {
        let c = Chronicler::new();
        assert!(c.labels().is_empty());
    }

    #[test]
    fn assign_labels_is_idempotent_for_stable_state() {
        let manifest = r#"{
            "labels": [
                { "id": "marker", "primitives": ["a"], "min_confidence": 0.0 }
            ]
        }"#;
        let engine = LabelEngine::from_json_str(manifest).unwrap();
        let mut c = Chronicler::new();
        for tick in 0..50 {
            c.ingest(&snap(tick, 1, &["a"]));
        }
        c.assign_labels(&engine, c.last_observed_tick());
        let first_pass: Vec<_> = c
            .labels()
            .values()
            .map(|l| (l.id.clone(), l.confidence.to_bits()))
            .collect();
        c.assign_labels(&engine, c.last_observed_tick());
        let second_pass: Vec<_> = c
            .labels()
            .values()
            .map(|l| (l.id.clone(), l.confidence.to_bits()))
            .collect();
        assert_eq!(first_pass, second_pass);
        assert_eq!(first_pass.len(), 1);
    }

    #[test]
    fn assign_labels_drops_entries_below_min_confidence() {
        // Two patterns; only the one whose frequency clears the bar gets
        // labelled. The other observation sits in storage unlabelled.
        let manifest = r#"{
            "labels": [
                { "id": "common", "primitives": ["a"], "min_confidence": 0.5 },
                { "id": "rare",   "primitives": ["b"], "min_confidence": 0.5 }
            ]
        }"#;
        let engine = LabelEngine::from_json_str(manifest).unwrap();
        let mut c = Chronicler::new();
        for tick in 0..99 {
            c.ingest(&snap(tick, 1, &["a"]));
        }
        c.ingest(&snap(99, 1, &["b"]));
        c.assign_labels(&engine, c.last_observed_tick());
        let label_ids: Vec<&str> = c.labels().values().map(|l| l.id.as_str()).collect();
        assert_eq!(label_ids, vec!["common"]);
    }
}
