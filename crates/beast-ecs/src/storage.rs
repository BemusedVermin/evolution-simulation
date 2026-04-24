//! Component storage & parallelization helpers (S5.6 — issue #110).
//!
//! The ECS itself does not enforce a particular concurrency strategy —
//! that lives in the scheduler (S6). This module documents the two
//! allowed patterns and provides a tiny helper for the safe-sequential
//! one so callers don't reach for `specs::Join` directly.
//!
//! # Pattern A: sequential iteration via the sorted entity index
//!
//! The default for any system whose per-entity work may write to
//! downstream state. Iterates the `Resources::entity_index` bucket for
//! the relevant marker, in ascending `Entity` order (see
//! [`crate::entity_id::SortedEntityIndex`]). Use
//! [`for_each_entity_of`].
//!
//! # Pattern B: parallel read-only iteration via `specs::ParJoin`
//!
//! Only safe when the per-entity work reads components but writes
//! nothing that another thread will later read within the same tick
//! (rule of thumb: the work is a pure function of the entity's own
//! components and produces a result that is aggregated serially after
//! the parallel region). Use `specs::ParJoin` on the relevant storages.
//! The scheduler will never put two systems with overlapping writes in
//! the same stage.
//!
//! The iteration order of `ParJoin` is non-deterministic; aggregation
//! must therefore use an associative+commutative reduction (e.g., sum,
//! max) or feed back into the entity index and run a serial pass
//! second.

use specs::Entity;

use crate::entity_id::MarkerKind;
use crate::resources::Resources;

// Re-export both join traits so downstream crates can `use
// beast_ecs::Join` / `beast_ecs::ParJoin` without bringing in specs.
pub use specs::{Join, ParJoin};

/// Visit every entity tagged with `marker` in ascending-entity order.
///
/// This is the safe-sequential path (Pattern A above). Use this when
/// the per-entity work writes to downstream state — the ordering
/// guarantee is what keeps the tick deterministic.
///
/// Borrowing is explicit: pass `&Resources` to look up the bucket, and
/// the closure decides what to do with each `Entity`. The helper
/// intentionally does not open component storages — that is the
/// caller's responsibility, and doing so inside this helper would
/// force a single storage type on every system.
///
/// # Example
///
/// ```
/// use beast_channels::ChannelRegistry;
/// use beast_ecs::{storage::for_each_entity_of, MarkerKind, Resources};
/// use beast_primitives::PrimitiveRegistry;
///
/// let resources =
///     Resources::new(0, ChannelRegistry::new(), PrimitiveRegistry::new());
/// // Empty world → closure is never called. Compiles, no panic.
/// for_each_entity_of(&resources, MarkerKind::Creature, |_entity| {
///     unreachable!("no creatures registered in this example");
/// });
/// ```
pub fn for_each_entity_of<F>(resources: &Resources, marker: MarkerKind, mut f: F)
where
    F: FnMut(Entity),
{
    for entity in resources.entity_index.entities_of(marker) {
        f(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use beast_channels::ChannelRegistry;
    use beast_primitives::PrimitiveRegistry;
    use specs::{Builder, WorldExt};

    use crate::EcsWorld;

    fn test_resources() -> Resources {
        Resources::new(3, ChannelRegistry::new(), PrimitiveRegistry::new())
    }

    #[test]
    fn for_each_entity_of_visits_in_ascending_order() {
        let mut world = EcsWorld::new();
        let entities: Vec<Entity> = (0..8).map(|_| world.create_entity().build()).collect();

        let mut resources = test_resources();
        // Insert out of order; the index sorts on insertion.
        for idx in [5, 0, 7, 3, 1, 6, 2, 4] {
            resources
                .entity_index
                .insert(entities[idx], MarkerKind::Creature);
        }

        let mut visited: Vec<Entity> = Vec::new();
        for_each_entity_of(&resources, MarkerKind::Creature, |e| visited.push(e));
        assert_eq!(visited, entities);
    }

    #[test]
    fn for_each_entity_of_is_noop_for_empty_bucket() {
        let resources = test_resources();
        let mut called = 0;
        for_each_entity_of(&resources, MarkerKind::Settlement, |_| called += 1);
        assert_eq!(called, 0);
    }

    #[test]
    fn parjoin_reexport_is_available() {
        // Compile-time smoke: the ParJoin re-export exists and names
        // the same trait specs defines under its `parallel` feature.
        // Registering Position first ensures the specs World has the
        // storage resource; without registration, `read_storage` panics
        // from shred's MetaTable.
        fn takes_par_join<T: ParJoin>(_: T) {}
        let mut world = EcsWorld::new();
        world.register_component::<crate::components::Position>();
        let storage = world.world().read_storage::<crate::components::Position>();
        takes_par_join(&storage);
    }

    #[test]
    fn for_each_respects_other_bucket_entries() {
        let mut world = EcsWorld::new();
        let entities: Vec<Entity> = (0..4).map(|_| world.create_entity().build()).collect();
        let mut resources = test_resources();
        resources
            .entity_index
            .insert(entities[0], MarkerKind::Creature);
        resources
            .entity_index
            .insert(entities[1], MarkerKind::Pathogen);
        resources
            .entity_index
            .insert(entities[2], MarkerKind::Creature);
        resources
            .entity_index
            .insert(entities[3], MarkerKind::Biome);

        let mut creatures = Vec::new();
        for_each_entity_of(&resources, MarkerKind::Creature, |e| creatures.push(e));
        assert_eq!(creatures, vec![entities[0], entities[2]]);
    }
}
