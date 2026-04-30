//! Lifecycle event log (S12.1).
//!
//! Where [`PatternObservation`](crate::PatternObservation) accumulates
//! "this primitive set fires often", a [`LifecycleEvent`] records a
//! discrete moment in an entity's life: it was born, it died, its
//! species went extinct, its emitted phenotype shifted.
//!
//! # Determinism
//!
//! Per `documentation/INVARIANTS.md` §1, iteration over the log is a
//! total function of contents, never of insertion order. The primary
//! store is a [`BTreeMap`] keyed by [`EventKey`] (`tick → kind → id`),
//! and the per-entity / per-species reverse indices use
//! [`BTreeSet<EventKey>`] for the same reason.
//!
//! # Mechanics-label separation
//!
//! Per `documentation/INVARIANTS.md` §2, the log carries no
//! human-readable ability strings. [`DeathCause`] is a small structural
//! enum the sim layer fills in (it knows whether the kill came through
//! the predation path or the metabolic path); the chronicler's label
//! pipeline never branches on it.

use std::collections::{BTreeMap, BTreeSet};

use beast_core::{EntityId, TickCounter};
use serde::{Deserialize, Serialize};

use crate::pattern::PatternSignature;
use crate::query::SpeciesId;
use crate::tick_range::TickRange;

/// Coarse cause-of-death enum populated by the sim layer.
///
/// Structural — *not* a label. The sim system that drove the death
/// (predation, parasitism, metabolic, etc.) tags the event so the
/// bestiary can group "this species mostly dies of starvation" without
/// the chronicler having to inspect any primitive id strings.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeathCause {
    /// Killed by another entity in a combat / predation round.
    Predation,
    /// Metabolic failure: depleted body channels with no source.
    Starvation,
    /// Lifespan exceeded; integrity decayed below viability.
    OldAge,
    /// Anything not yet categorised. Sim systems should prefer one of
    /// the three specific variants above when the path is known.
    Other,
}

impl DeathCause {
    /// Stable ordinal for `EventKey` packing. Sorted by ordinal so two
    /// deaths at the same tick + same entity produce a deterministic
    /// iteration order even though the entity / tick already match.
    #[inline]
    const fn ord(self) -> u8 {
        match self {
            DeathCause::Predation => 0,
            DeathCause::Starvation => 1,
            DeathCause::OldAge => 2,
            DeathCause::Other => 3,
        }
    }
}

/// One discrete moment in an entity's (or species') life history.
///
/// Variants are tagged by [`Self::kind_ord`] so the [`EventKey`] sort
/// produces a deterministic ordering when multiple events land on the
/// same `(tick, primary_id)` pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleEvent {
    /// A new entity entered the simulation.
    Birth {
        /// Newly-allocated entity id.
        entity: EntityId,
        /// Species the entity belongs to.
        species: SpeciesId,
        /// Tick on which the entity was spawned.
        tick: TickCounter,
    },
    /// An entity left the simulation.
    Death {
        /// Entity that died.
        entity: EntityId,
        /// Species the entity belonged to.
        species: SpeciesId,
        /// Tick on which the entity died.
        tick: TickCounter,
        /// Coarse cause-of-death from the sim layer.
        cause: DeathCause,
    },
    /// The last living member of a species died.
    Extinction {
        /// Species that went extinct.
        species: SpeciesId,
        /// Final entity id removed; useful for the bestiary's "last
        /// observed" annotation.
        last_member: EntityId,
        /// Tick on which the last member died.
        tick: TickCounter,
    },
    /// An entity's emitted-primitive set shifted from one signature to
    /// another. Drives the lineage / mutation timeline view.
    PhenotypeChange {
        /// Entity whose phenotype shifted.
        entity: EntityId,
        /// Signature before the shift.
        prior: PatternSignature,
        /// Signature after the shift.
        current: PatternSignature,
        /// Tick on which the shift was first observed.
        tick: TickCounter,
    },
}

impl LifecycleEvent {
    /// Tick on which this event occurred. Used for tick-range filtering
    /// and as the primary `EventKey` axis.
    #[inline]
    pub fn tick(&self) -> TickCounter {
        match self {
            LifecycleEvent::Birth { tick, .. }
            | LifecycleEvent::Death { tick, .. }
            | LifecycleEvent::Extinction { tick, .. }
            | LifecycleEvent::PhenotypeChange { tick, .. } => *tick,
        }
    }

    /// Stable per-variant ordinal used for `EventKey` packing. Sorted
    /// in declaration order so two events at the same tick produce a
    /// reader-intuitive ordering (births, then deaths, then
    /// extinctions, then phenotype shifts).
    #[inline]
    fn kind_ord(&self) -> u8 {
        match self {
            LifecycleEvent::Birth { .. } => 0,
            LifecycleEvent::Death { .. } => 1,
            LifecycleEvent::Extinction { .. } => 2,
            LifecycleEvent::PhenotypeChange { .. } => 3,
        }
    }

    /// Fine-grained ordinal that breaks ties when `tick` + `kind_ord` +
    /// `primary_id` already match. Currently only `Death` carries a
    /// secondary tag (the cause); other variants return `0`.
    #[inline]
    fn sub_ord(&self) -> u8 {
        match self {
            LifecycleEvent::Death { cause, .. } => cause.ord(),
            _ => 0,
        }
    }

    /// `u32` axis used as the third sort key. Entity-bearing variants
    /// emit the entity id; [`LifecycleEvent::Extinction`] emits the
    /// species id (the entity is `last_member`, but the canonical
    /// identity of an extinction is the species). Both id types are
    /// `#[repr(transparent)] u32` so the laundering is byte-identical.
    #[inline]
    fn primary_id(&self) -> u32 {
        match self {
            LifecycleEvent::Birth { entity, .. }
            | LifecycleEvent::Death { entity, .. }
            | LifecycleEvent::PhenotypeChange { entity, .. } => entity.raw(),
            LifecycleEvent::Extinction { species, .. } => species.raw(),
        }
    }

    /// Build the storage key for this event.
    #[inline]
    pub fn key(&self) -> EventKey {
        EventKey {
            tick: self.tick(),
            kind_ord: self.kind_ord(),
            primary_id: self.primary_id(),
            sub_ord: self.sub_ord(),
        }
    }

    /// Entity attribution if any. `Extinction` returns the
    /// `last_member` so per-entity queries surface the final death of
    /// a lineage.
    #[inline]
    pub fn entity(&self) -> Option<EntityId> {
        match self {
            LifecycleEvent::Birth { entity, .. }
            | LifecycleEvent::Death { entity, .. }
            | LifecycleEvent::PhenotypeChange { entity, .. } => Some(*entity),
            LifecycleEvent::Extinction { last_member, .. } => Some(*last_member),
        }
    }

    /// Species attribution if any. `PhenotypeChange` does not carry a
    /// species (it is observed at the entity level, before the
    /// chronicler may have settled on a species mapping for the
    /// post-shift signature).
    #[inline]
    pub fn species(&self) -> Option<SpeciesId> {
        match self {
            LifecycleEvent::Birth { species, .. }
            | LifecycleEvent::Death { species, .. }
            | LifecycleEvent::Extinction { species, .. } => Some(*species),
            LifecycleEvent::PhenotypeChange { .. } => None,
        }
    }
}

/// Storage key for [`LifecycleEventLog`].
///
/// Lexicographic ordering on `(tick, kind_ord, primary_id, sub_ord)` is
/// load-bearing — it is the sort the bestiary's "history" pane reads.
/// The fields are public so callers writing tests can construct synthetic
/// keys without going through [`LifecycleEvent::key`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventKey {
    /// Primary axis: tick on which the event occurred.
    pub tick: TickCounter,
    /// Secondary axis: per-variant ordinal (Birth=0, Death=1, ...).
    pub kind_ord: u8,
    /// Tertiary axis: `EntityId` or `SpeciesId` cast to `u32`.
    pub primary_id: u32,
    /// Quaternary axis: `DeathCause` ordinal for `Death`; `0` otherwise.
    pub sub_ord: u8,
}

/// Append-only log of [`LifecycleEvent`]s with deterministic iteration.
///
/// Storage layout:
///
/// * `events: BTreeMap<EventKey, LifecycleEvent>` — primary store. Sort
///   order is `tick → kind → id → sub`.
/// * `by_entity: BTreeMap<EntityId, BTreeSet<EventKey>>` — reverse
///   index for `events_for_entity`.
/// * `by_species: BTreeMap<SpeciesId, BTreeSet<EventKey>>` — reverse
///   index for `events_for_species`.
///
/// Both reverse indices use `BTreeSet<EventKey>` so iteration over a
/// per-entity / per-species view is also tick-ordered.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleEventLog {
    events: BTreeMap<EventKey, LifecycleEvent>,
    by_entity: BTreeMap<EntityId, BTreeSet<EventKey>>,
    by_species: BTreeMap<SpeciesId, BTreeSet<EventKey>>,
}

impl LifecycleEventLog {
    /// Construct an empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of events recorded so far.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// `true` when no events have been recorded.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Record one event. The first event at a given [`EventKey`] wins;
    /// duplicate keys are rejected via [`debug_assert!`] in dev builds
    /// and silently ignored in release builds.
    ///
    /// Two events that share `(tick, kind, primary_id, sub)` are
    /// genuinely indistinguishable to the bestiary — nothing reads
    /// payload bytes between two `Death` events at the same tick for
    /// the same entity with the same cause. The release-mode silent
    /// drop preserves determinism (no panic-vs-not divergence between
    /// debug and release builds running the same input log).
    pub fn record(&mut self, event: LifecycleEvent) {
        let key = event.key();
        debug_assert!(
            !self.events.contains_key(&key),
            "duplicate LifecycleEvent at key {key:?}: {existing:?} vs {incoming:?}",
            existing = self.events.get(&key),
            incoming = event,
        );
        if self.events.contains_key(&key) {
            return;
        }
        if let Some(entity) = event.entity() {
            self.by_entity.entry(entity).or_default().insert(key);
        }
        if let Some(species) = event.species() {
            self.by_species.entry(species).or_default().insert(key);
        }
        self.events.insert(key, event);
    }

    /// Iterate over every event in tick-then-kind order.
    pub fn iter(&self) -> impl Iterator<Item = &LifecycleEvent> + '_ {
        self.events.values()
    }

    /// Events whose `tick` falls inside `range`.
    ///
    /// Uses `BTreeMap::range` for the lower bound and a half-open upper
    /// bound — O(log N + matches) rather than a full scan.
    pub fn events_in_range(&self, range: TickRange) -> impl Iterator<Item = &LifecycleEvent> + '_ {
        let lower = EventKey {
            tick: range.start,
            kind_ord: 0,
            primary_id: 0,
            sub_ord: 0,
        };
        let upper = EventKey {
            tick: range.end,
            kind_ord: 0,
            primary_id: 0,
            sub_ord: 0,
        };
        self.events.range(lower..upper).map(|(_, e)| e)
    }

    /// Events attributed to `entity` (Birth / Death / PhenotypeChange,
    /// plus the `Extinction` whose `last_member` is `entity`), in
    /// chronological order.
    pub fn events_for_entity(
        &self,
        entity: EntityId,
    ) -> impl Iterator<Item = &LifecycleEvent> + '_ {
        // Materialise the per-entity `Vec` so the returned iterator owns
        // its key list; without this we would need to borrow the index
        // for the iterator's lifetime, which complicates the type sig
        // for callers without buying anything in iteration speed.
        let keys: Vec<EventKey> = self
            .by_entity
            .get(&entity)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        keys.into_iter().filter_map(|k| self.events.get(&k))
    }

    /// Events attributed to `species` (Birth / Death / Extinction), in
    /// chronological order.
    pub fn events_for_species(
        &self,
        species: SpeciesId,
    ) -> impl Iterator<Item = &LifecycleEvent> + '_ {
        let keys: Vec<EventKey> = self
            .by_species
            .get(&species)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        keys.into_iter().filter_map(|k| self.events.get(&k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(n: u64) -> TickCounter {
        TickCounter::new(n)
    }

    fn e(n: u32) -> EntityId {
        EntityId::new(n)
    }

    fn sp(n: u32) -> SpeciesId {
        SpeciesId::new(n)
    }

    fn sig(byte: u8) -> PatternSignature {
        PatternSignature([byte; 32])
    }

    /// `LifecycleEvent::Birth { entity: e(eid), species: sp(spid), tick: t(tk) }`.
    /// Test-only builder. Consolidated so each test site reads as the
    /// scenario it cares about, not as a wall of struct-literal noise.
    fn birth(eid: u32, spid: u32, tk: u64) -> LifecycleEvent {
        LifecycleEvent::Birth {
            entity: e(eid),
            species: sp(spid),
            tick: t(tk),
        }
    }

    /// `LifecycleEvent::Death { entity, species, tick, cause }`.
    fn death(eid: u32, spid: u32, tk: u64, cause: DeathCause) -> LifecycleEvent {
        LifecycleEvent::Death {
            entity: e(eid),
            species: sp(spid),
            tick: t(tk),
            cause,
        }
    }

    /// `LifecycleEvent::Extinction { species, last_member, tick }`.
    fn extinction(spid: u32, last: u32, tk: u64) -> LifecycleEvent {
        LifecycleEvent::Extinction {
            species: sp(spid),
            last_member: e(last),
            tick: t(tk),
        }
    }

    /// `LifecycleEvent::PhenotypeChange { entity, prior, current, tick }`.
    fn phenotype(eid: u32, prior: u8, current: u8, tk: u64) -> LifecycleEvent {
        LifecycleEvent::PhenotypeChange {
            entity: e(eid),
            prior: sig(prior),
            current: sig(current),
            tick: t(tk),
        }
    }

    #[test]
    fn empty_log_reports_zero_length() {
        let log = LifecycleEventLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
        assert_eq!(log.iter().count(), 0);
    }

    #[test]
    fn record_then_iter_returns_event() {
        let mut log = LifecycleEventLog::new();
        log.record(birth(1, 7, 10));
        assert_eq!(log.len(), 1);
        assert_eq!(log.iter().count(), 1);
    }

    #[test]
    fn out_of_order_insertion_yields_sorted_iteration() {
        let mut log = LifecycleEventLog::new();
        // Insert tick=20 first, then tick=10 — iteration must be by
        // tick, not by insertion order.
        log.record(birth(1, 0, 20));
        log.record(birth(2, 0, 10));
        let ticks: Vec<u64> = log.iter().map(|e| e.tick().raw()).collect();
        assert_eq!(ticks, vec![10, 20]);
    }

    #[test]
    fn same_tick_orders_by_kind_then_id() {
        let mut log = LifecycleEventLog::new();
        // Death (kind=1) before Birth (kind=0) in insertion order, but
        // iteration must put Birth first. Two births at the same tick
        // also exercise the `primary_id` tie-break.
        log.record(death(2, 0, 5, DeathCause::Predation));
        log.record(birth(1, 0, 5));
        log.record(birth(0, 0, 5));
        let kinds: Vec<u8> = log.iter().map(|e| e.kind_ord()).collect();
        assert_eq!(kinds, vec![0, 0, 1], "births before deaths at same tick");
        let ids: Vec<u32> = log.iter().map(|e| e.primary_id()).collect();
        assert_eq!(
            ids,
            vec![0, 1, 2],
            "tied (tick, kind) is broken by primary_id ascending"
        );
    }

    #[test]
    fn deaths_at_same_tick_order_by_cause() {
        let mut log = LifecycleEventLog::new();
        // Same (tick, kind, entity) triple — sub_ord (cause) must
        // disambiguate. Two deaths of the same entity at the same tick
        // with different causes is degenerate, but the sort still has
        // to be a total order.
        log.record(death(0, 0, 1, DeathCause::OldAge)); // ord = 2
        log.record(death(0, 0, 1, DeathCause::Predation)); // ord = 0
        let causes: Vec<DeathCause> = log
            .iter()
            .filter_map(|e| match e {
                LifecycleEvent::Death { cause, .. } => Some(*cause),
                _ => None,
            })
            .collect();
        assert_eq!(causes, vec![DeathCause::Predation, DeathCause::OldAge]);
    }

    #[test]
    fn events_in_range_is_half_open() {
        let mut log = LifecycleEventLog::new();
        for tk in [4, 5, 7, 10] {
            log.record(birth(tk as u32, 0, tk));
        }
        let range = TickRange::new(t(5), t(10)).unwrap();
        let ticks: Vec<u64> = log.events_in_range(range).map(|e| e.tick().raw()).collect();
        // 5 included, 10 excluded.
        assert_eq!(ticks, vec![5, 7]);
    }

    #[test]
    fn events_for_entity_filters_to_the_entity() {
        let mut log = LifecycleEventLog::new();
        log.record(birth(1, 0, 1));
        log.record(birth(2, 0, 1));
        log.record(death(1, 0, 5, DeathCause::Other));
        let mut ticks: Vec<u64> = log
            .events_for_entity(e(1))
            .map(|e| e.tick().raw())
            .collect();
        ticks.sort_unstable();
        assert_eq!(ticks, vec![1, 5]);
        // Entity 2 only has the birth event.
        assert_eq!(log.events_for_entity(e(2)).count(), 1);
        // Unknown entity returns empty.
        assert_eq!(log.events_for_entity(e(999)).count(), 0);
    }

    #[test]
    fn events_for_species_includes_extinction() {
        let mut log = LifecycleEventLog::new();
        log.record(birth(1, 7, 1));
        log.record(death(1, 7, 50, DeathCause::OldAge));
        log.record(extinction(7, 1, 50));
        let count = log.events_for_species(sp(7)).count();
        assert_eq!(count, 3);
    }

    #[test]
    fn phenotype_change_has_no_species_attribution() {
        let mut log = LifecycleEventLog::new();
        log.record(phenotype(1, 0xAA, 0xBB, 20));
        // Surfaces under per-entity but not per-species — the post-shift
        // signature may not even map to a species yet.
        assert_eq!(log.events_for_entity(e(1)).count(), 1);
        assert_eq!(log.events_for_species(sp(0)).count(), 0);
    }

    #[test]
    fn iteration_is_byte_identical_across_two_runs_with_same_inputs() {
        // Determinism property: two builders running the same insertion
        // sequence produce byte-identical iteration. Locked in here so
        // a future refactor (e.g. switching the storage map) cannot
        // silently break it.
        fn build() -> LifecycleEventLog {
            let mut log = LifecycleEventLog::new();
            log.record(birth(3, 1, 7));
            log.record(birth(1, 1, 2));
            log.record(death(3, 1, 99, DeathCause::Starvation));
            log.record(phenotype(1, 1, 2, 50));
            log
        }
        let a = build();
        let b = build();
        // PartialEq: structural deep equality.
        assert_eq!(a, b);
        // Iteration order check: collect both and compare.
        let av: Vec<&LifecycleEvent> = a.iter().collect();
        let bv: Vec<&LifecycleEvent> = b.iter().collect();
        assert_eq!(av, bv);
    }

    #[test]
    fn events_in_full_range_returns_all() {
        let mut log = LifecycleEventLog::new();
        for tk in [1, 2, 3, 4, 5] {
            log.record(birth(tk as u32, 0, tk));
        }
        let count = log.events_in_range(TickRange::ALL).count();
        assert_eq!(count, 5);
    }

    #[test]
    fn key_orders_lexicographically_on_all_four_axes() {
        // Pin the EventKey sort: (tick, kind_ord, primary_id, sub_ord).
        let a = EventKey {
            tick: t(1),
            kind_ord: 0,
            primary_id: 0,
            sub_ord: 0,
        };
        let b = EventKey {
            tick: t(1),
            kind_ord: 0,
            primary_id: 0,
            sub_ord: 1,
        };
        let c = EventKey {
            tick: t(1),
            kind_ord: 0,
            primary_id: 1,
            sub_ord: 0,
        };
        let d = EventKey {
            tick: t(1),
            kind_ord: 1,
            primary_id: 0,
            sub_ord: 0,
        };
        let e_ = EventKey {
            tick: t(2),
            kind_ord: 0,
            primary_id: 0,
            sub_ord: 0,
        };
        let mut keys = vec![e_, d, c, b, a];
        keys.sort();
        assert_eq!(keys, vec![a, b, c, d, e_]);
    }
}
