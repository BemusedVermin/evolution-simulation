//! Sorted entity index — `BTreeMap<MarkerKind, BTreeSet<Entity>>`
//! (S5.5 — issue #108).
//!
//! Makes deterministic iteration a first-class feature rather than an
//! emergent behaviour of `specs::Join`. Systems that need to iterate
//! "all creatures" (or all agents, settlements, etc.) ask this index
//! instead of joining storage directly — the index guarantees `(index,
//! generation)` ascending order by construction.
//!
//! See INVARIANTS §1 (sorted iteration in hot loops).

use std::collections::{BTreeMap, BTreeSet};

use specs::Entity;

/// Kinds of entity tracked in the sorted index — one variant per marker
/// component from [`crate::components::markers`]. `Ord` is derived so
/// this type can key a `BTreeMap` and `iter_all` can yield `(MarkerKind,
/// Entity)` in totally ordered fashion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarkerKind {
    /// Macro-scale beast.
    Creature,
    /// Micro-scale organism (disease, parasite, symbiont).
    Pathogen,
    /// NPC / diplomat / caravan leader.
    Agent,
    /// Social grouping of agents/settlements.
    Faction,
    /// Persistent inhabited location.
    Settlement,
    /// Biome cell on the world map.
    Biome,
}

impl MarkerKind {
    /// Every variant in declaration order. Useful for tests and
    /// iteration-order assertions.
    pub const ALL: [MarkerKind; 6] = [
        MarkerKind::Creature,
        MarkerKind::Pathogen,
        MarkerKind::Agent,
        MarkerKind::Faction,
        MarkerKind::Settlement,
        MarkerKind::Biome,
    ];
}

/// Deterministic entity index: one `BTreeSet<Entity>` per
/// [`MarkerKind`]. Iteration always returns entities in ascending
/// `(index, generation)` order because that is how `specs::Entity`
/// implements `Ord`.
///
/// The index is kept outside the `specs::World` so that systems can
/// look up "all creatures" without joining; this matters because
/// `Join` iteration order is a `specs` implementation detail, while
/// this index makes the ordering contract ours.
///
/// # Maintenance contract
///
/// Callers update the index by hand at entity creation / destruction.
/// S5.6 will add storage adapters that thread this automatically; until
/// then, the spawner is responsible for calling [`Self::insert`].
#[derive(Debug, Default)]
pub struct SortedEntityIndex {
    buckets: BTreeMap<MarkerKind, BTreeSet<Entity>>,
}

impl SortedEntityIndex {
    /// Create an empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `entity` carries the given marker. Idempotent — a
    /// duplicate insert is a no-op.
    pub fn insert(&mut self, entity: Entity, marker: MarkerKind) {
        self.buckets.entry(marker).or_default().insert(entity);
    }

    /// Remove `entity` from the given marker bucket. Returns `true`
    /// iff the entry existed.
    pub fn remove(&mut self, entity: Entity, marker: MarkerKind) -> bool {
        match self.buckets.get_mut(&marker) {
            Some(set) => set.remove(&entity),
            None => false,
        }
    }

    /// Drop `entity` from every bucket it appears in. Convenience for
    /// entity deletion paths that do not track the marker kinds the
    /// entity carried.
    pub fn remove_everywhere(&mut self, entity: Entity) {
        for set in self.buckets.values_mut() {
            set.remove(&entity);
        }
    }

    /// Iterator of entities tagged with `marker`, in ascending entity
    /// order. Returns an empty iterator when the marker has never been
    /// inserted.
    pub fn entities_of(&self, marker: MarkerKind) -> impl Iterator<Item = Entity> + '_ {
        self.buckets
            .get(&marker)
            .into_iter()
            .flat_map(BTreeSet::iter)
            .copied()
    }

    /// Count of entities in the given marker bucket.
    #[must_use]
    pub fn len_of(&self, marker: MarkerKind) -> usize {
        self.buckets.get(&marker).map_or(0, BTreeSet::len)
    }

    /// Total number of `(entity, marker)` memberships across all
    /// buckets. Note: an entity with two markers counts twice.
    #[must_use]
    pub fn total_memberships(&self) -> usize {
        self.buckets.values().map(BTreeSet::len).sum()
    }

    /// Iterate all `(MarkerKind, Entity)` pairs in total order — first
    /// by `MarkerKind` (declaration order), then by `Entity` (ascending
    /// index, generation). The returned iterator visits every bucket,
    /// even empty ones are skipped silently.
    pub fn iter_all(&self) -> impl Iterator<Item = (MarkerKind, Entity)> + '_ {
        self.buckets
            .iter()
            .flat_map(|(kind, set)| set.iter().map(move |e| (*kind, *e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EcsWorld;
    use specs::Builder;

    fn make_entities(n: usize) -> Vec<Entity> {
        // Build a throwaway world; specs hands out entity indices 0..n.
        let mut world = EcsWorld::new();
        (0..n).map(|_| world.create_entity().build()).collect()
    }

    #[test]
    fn insert_then_iterate_yields_ascending_order() {
        let entities = make_entities(5);
        let mut index = SortedEntityIndex::new();
        // Insert out of order.
        for e in [4, 1, 3, 0, 2] {
            index.insert(entities[e], MarkerKind::Creature);
        }
        let out: Vec<Entity> = index.entities_of(MarkerKind::Creature).collect();
        // entities[0] has the smallest specs index, entities[4] the largest.
        assert_eq!(out, entities);
    }

    #[test]
    fn remove_only_affects_target_bucket() {
        let entities = make_entities(3);
        let mut index = SortedEntityIndex::new();
        for e in &entities {
            index.insert(*e, MarkerKind::Creature);
            index.insert(*e, MarkerKind::Pathogen);
        }
        assert!(index.remove(entities[1], MarkerKind::Creature));
        let creatures: Vec<Entity> = index.entities_of(MarkerKind::Creature).collect();
        let pathogens: Vec<Entity> = index.entities_of(MarkerKind::Pathogen).collect();
        assert_eq!(creatures, vec![entities[0], entities[2]]);
        assert_eq!(pathogens.len(), 3, "pathogen bucket untouched");
    }

    #[test]
    fn duplicate_insert_is_idempotent() {
        let entities = make_entities(1);
        let mut index = SortedEntityIndex::new();
        index.insert(entities[0], MarkerKind::Creature);
        index.insert(entities[0], MarkerKind::Creature);
        assert_eq!(index.len_of(MarkerKind::Creature), 1);
    }

    #[test]
    fn remove_missing_returns_false() {
        let entities = make_entities(1);
        let mut index = SortedEntityIndex::new();
        assert!(!index.remove(entities[0], MarkerKind::Biome));
    }

    #[test]
    fn remove_everywhere_drops_from_all_buckets() {
        let entities = make_entities(1);
        let e = entities[0];
        let mut index = SortedEntityIndex::new();
        for marker in MarkerKind::ALL {
            index.insert(e, marker);
        }
        index.remove_everywhere(e);
        for marker in MarkerKind::ALL {
            assert_eq!(index.len_of(marker), 0, "{marker:?} still has e");
        }
    }

    #[test]
    fn iter_all_is_marker_then_entity_sorted() {
        // Build an index with entities spread across three markers in
        // deliberately-wrong insertion order; confirm iter_all gives
        // MarkerKind ascending, Entity ascending within each.
        let entities = make_entities(4);
        let mut index = SortedEntityIndex::new();
        // Pathogen < Biome ordering check (Pathogen is declared second).
        index.insert(entities[3], MarkerKind::Pathogen);
        index.insert(entities[1], MarkerKind::Creature);
        index.insert(entities[0], MarkerKind::Biome);
        index.insert(entities[2], MarkerKind::Creature);
        index.insert(entities[1], MarkerKind::Pathogen);

        let out: Vec<(MarkerKind, Entity)> = index.iter_all().collect();
        let expected = vec![
            (MarkerKind::Creature, entities[1]),
            (MarkerKind::Creature, entities[2]),
            (MarkerKind::Pathogen, entities[1]),
            (MarkerKind::Pathogen, entities[3]),
            (MarkerKind::Biome, entities[0]),
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn iteration_is_deterministic_across_runs() {
        // Same sequence of operations → same iteration order every call.
        let entities = make_entities(6);
        let mut a = SortedEntityIndex::new();
        let mut b = SortedEntityIndex::new();
        // Insert in different orders but at the same marker.
        for e in [4, 0, 2, 5, 1, 3] {
            a.insert(entities[e], MarkerKind::Agent);
        }
        for e in [5, 1, 3, 0, 4, 2] {
            b.insert(entities[e], MarkerKind::Agent);
        }
        let out_a: Vec<Entity> = a.entities_of(MarkerKind::Agent).collect();
        let out_b: Vec<Entity> = b.entities_of(MarkerKind::Agent).collect();
        assert_eq!(out_a, out_b);
    }

    #[test]
    fn total_memberships_counts_each_pair_once() {
        let entities = make_entities(2);
        let mut index = SortedEntityIndex::new();
        index.insert(entities[0], MarkerKind::Creature);
        index.insert(entities[0], MarkerKind::Pathogen);
        index.insert(entities[1], MarkerKind::Creature);
        assert_eq!(index.total_memberships(), 3);
    }
}
