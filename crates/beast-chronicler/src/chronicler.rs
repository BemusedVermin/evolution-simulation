//! [`Chronicler`] — accumulates pattern observations from primitive
//! snapshots and (S10.6) holds the latest manifest-driven label
//! assignments. The read API for the UI layer (S10.7) layers on top.

use std::collections::{BTreeMap, BTreeSet};

use beast_core::{EntityId, TickCounter, Q3232};

use crate::label::{Label, LabelEngine};
use crate::pattern::{PatternObservation, PatternSignature};
use crate::query::{
    label_ids_match_search, sort_bestiary, BestiaryEntry, BestiaryFilter, ChroniclerQuery, LabelId,
    SpeciesId,
};
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
    /// Reverse index from `EntityId` → per-signature ingestion count
    /// for that entity. Built during [`Self::ingest`]; consumed by the
    /// S10.7 query API.
    ///
    /// Per-entity counts are tracked separately from the global
    /// `observations` count so the bestiary aggregator can attribute
    /// snapshots to a single species without double-counting when two
    /// entities of different species share a signature. The map per
    /// entity is small (number of distinct primitive sets the entity
    /// has ever produced), so the storage cost stays bounded.
    entity_emissions: BTreeMap<EntityId, BTreeMap<PatternSignature, u64>>,
    /// Map from `EntityId` → `SpeciesId`, populated by
    /// [`Self::set_entity_species`]. The chronicler does not derive
    /// species itself — higher-layer code (sim spawner, save loader) is
    /// responsible for declaring it. Entities without a registered
    /// species are excluded from bestiary queries; they still appear
    /// under [`Self::labels_for_entity`].
    entity_species: BTreeMap<EntityId, SpeciesId>,
    /// Reverse index of [`Self::entity_species`] — the set of entities
    /// registered for each species. Maintained in lockstep with
    /// `entity_species` by [`Self::set_entity_species`].
    ///
    /// Without this, [`ChroniclerQuery::bestiary_entries`] would scan
    /// every entry of `entity_species` once per distinct species —
    /// O(S * E) work on every render. With the reverse index a single
    /// bestiary build is O(members) and the full sweep is O(E * P)
    /// (each entity contributes its own signatures exactly once),
    /// matching the optimal cost.
    species_entities: BTreeMap<SpeciesId, BTreeSet<EntityId>>,
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
        // Maintain the entity → signatures reverse index for the S10.7
        // query API, with per-entity per-signature counts so the
        // bestiary aggregator can attribute snapshots to a single
        // species without double-counting shared signatures.
        let per_entity = self.entity_emissions.entry(snapshot.entity).or_default();
        let count = per_entity.entry(signature).or_insert(0);
        *count = count.saturating_add(1);
    }

    /// Register the species an entity belongs to.
    ///
    /// The chronicler does not derive species itself; higher-layer code
    /// (sim spawner, save loader) is the source of truth. Repeated calls
    /// for the same entity overwrite the prior species — useful for
    /// speciation events where an entity transitions between species
    /// ids over its lifetime. The `species_entities` reverse index is
    /// kept in sync: the entity is removed from the previous species'
    /// set (and that set dropped if it became empty) before being
    /// inserted into the new one.
    pub fn set_entity_species(&mut self, entity: EntityId, species: SpeciesId) {
        if let Some(prev) = self.entity_species.insert(entity, species) {
            if prev == species {
                // Same species, set already contains the entity — fast
                // path skips the reverse-index churn.
                return;
            }
            if let Some(prev_members) = self.species_entities.get_mut(&prev) {
                prev_members.remove(&entity);
                if prev_members.is_empty() {
                    self.species_entities.remove(&prev);
                }
            }
        }
        self.species_entities
            .entry(species)
            .or_default()
            .insert(entity);
    }

    /// Read-only view of the entity → per-signature emission counts.
    pub fn entity_emissions(&self) -> &BTreeMap<EntityId, BTreeMap<PatternSignature, u64>> {
        &self.entity_emissions
    }

    /// Read-only view of the entity → species registration map.
    pub fn entity_species(&self) -> &BTreeMap<EntityId, SpeciesId> {
        &self.entity_species
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

    /// Build a [`BestiaryEntry`] for one species by aggregating across
    /// every registered entity that belongs to it.
    ///
    /// Returns `None` when no entity has been registered for `species`.
    /// Pulled out of the [`ChroniclerQuery`] impl so
    /// [`Self::bestiary_entry`] and [`Self::bestiary_entries`] share
    /// one aggregation pass.
    ///
    /// Uses [`Self::species_entities`] for an O(members) member walk
    /// instead of an O(E) scan over every registered entity — required
    /// for the bestiary screen to stay within its render budget at
    /// 1000+ species × 10000+ entities.
    fn build_bestiary_entry(&self, species: SpeciesId) -> Option<BestiaryEntry> {
        let members = self.species_entities.get(&species)?;
        if members.is_empty() {
            // Defensive: the index invariant is "no empty sets" but
            // returning None here keeps the contract identical to the
            // pre-index version regardless.
            return None;
        }

        let mut observation_count: u64 = 0;
        let mut first_tick: Option<TickCounter> = None;
        let mut confidence: Q3232 = Q3232::ZERO;
        let mut label_ids: BTreeSet<LabelId> = BTreeSet::new();

        for entity in members {
            let Some(per_signature) = self.entity_emissions.get(entity) else {
                continue;
            };
            for (sig, count) in per_signature {
                observation_count = observation_count.saturating_add(*count);
                if let Some(observation) = self.observations.get(sig) {
                    first_tick = Some(match first_tick {
                        Some(prev) if prev <= observation.first_tick => prev,
                        _ => observation.first_tick,
                    });
                }
                if let Some(label) = self.labels.get(sig) {
                    label_ids.insert(label.id.clone());
                    if label.confidence > confidence {
                        confidence = label.confidence;
                    }
                }
            }
        }

        Some(BestiaryEntry {
            species,
            label_ids: label_ids.into_iter().collect(),
            observation_count,
            confidence,
            first_tick: first_tick.unwrap_or(TickCounter::ZERO),
        })
    }
}

impl ChroniclerQuery for Chronicler {
    fn label_for_signature(&self, sig: &PatternSignature) -> Option<&Label> {
        self.labels.get(sig)
    }

    fn labels_for_entity(&self, entity: EntityId) -> Vec<&Label> {
        let Some(per_signature) = self.entity_emissions.get(&entity) else {
            return Vec::new();
        };
        // Iterating the BTreeMap keys yields signatures in ascending
        // byte order, so the resulting Vec is deterministic across calls.
        per_signature
            .keys()
            .filter_map(|sig| self.labels.get(sig))
            .collect()
    }

    fn entities_with_label(&self, label_id: &str) -> Vec<EntityId> {
        // Collect signatures matching the requested label id first; the
        // labels map is small relative to entity_emissions, so this
        // narrows the work for the entity scan.
        let signatures: BTreeSet<PatternSignature> = self
            .labels
            .iter()
            .filter(|(_, label)| label.id == label_id)
            .map(|(sig, _)| *sig)
            .collect();
        if signatures.is_empty() {
            return Vec::new();
        }
        // Iteration over BTreeMap<EntityId, _> is ascending by EntityId,
        // satisfying INVARIANTS §1.
        self.entity_emissions
            .iter()
            .filter(|(_, per_signature)| per_signature.keys().any(|sig| signatures.contains(sig)))
            .map(|(entity, _)| *entity)
            .collect()
    }

    fn bestiary_entries(&self, filter: &BestiaryFilter) -> Vec<BestiaryEntry> {
        // BTreeMap iteration over species_entities yields species in
        // ascending SpeciesId order — gives a stable pre-sort baseline
        // before applying the configured order, and avoids the O(E)
        // scan a `entity_species.values().copied().collect()` would
        // otherwise cost.
        let mut entries: Vec<BestiaryEntry> = self
            .species_entities
            .keys()
            .copied()
            .filter_map(|s| self.build_bestiary_entry(s))
            .filter(|entry| !filter.discovered_only || entry.observation_count >= 1)
            .filter(|entry| match &filter.search {
                None => true,
                Some(needle) => label_ids_match_search(&entry.label_ids, needle),
            })
            .collect();
        sort_bestiary(&mut entries, filter.sort_by);
        entries
    }

    fn bestiary_entry(&self, species: SpeciesId) -> Option<BestiaryEntry> {
        self.build_bestiary_entry(species)
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
    fn set_entity_species_maintains_reverse_index() {
        let mut c = Chronicler::new();
        c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
        c.set_entity_species(EntityId::new(2), SpeciesId::new(0));
        c.set_entity_species(EntityId::new(3), SpeciesId::new(1));
        assert_eq!(
            c.species_entities.get(&SpeciesId::new(0)).map(|s| s.len()),
            Some(2)
        );
        assert_eq!(
            c.species_entities.get(&SpeciesId::new(1)).map(|s| s.len()),
            Some(1)
        );
    }

    #[test]
    fn set_entity_species_speciation_moves_entity_between_sets() {
        // Speciation event: entity 1 starts as species 0, then transitions
        // to species 1. The reverse index must follow.
        let mut c = Chronicler::new();
        c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
        c.set_entity_species(EntityId::new(1), SpeciesId::new(1));
        assert!(
            !c.species_entities.contains_key(&SpeciesId::new(0)),
            "empty species set should be removed"
        );
        assert_eq!(
            c.species_entities.get(&SpeciesId::new(1)).map(|s| s.len()),
            Some(1)
        );
    }

    #[test]
    fn set_entity_species_repeat_same_species_is_idempotent() {
        let mut c = Chronicler::new();
        c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
        c.set_entity_species(EntityId::new(1), SpeciesId::new(0));
        let members = c.species_entities.get(&SpeciesId::new(0)).unwrap();
        assert_eq!(members.len(), 1);
        assert!(members.contains(&EntityId::new(1)));
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
