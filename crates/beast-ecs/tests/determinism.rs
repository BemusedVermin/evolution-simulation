//! Integration determinism tests for the ECS foundation (S5.7 — issue #112).
//!
//! These tests exercise the full S5 surface:
//!
//! * `EcsWorld::new` + `components::register_all` (S5.1 + S5.2)
//! * `System` trait + `SystemStage` (S5.3)
//! * `Resources` with PRNG streams + tick counter (S5.4)
//! * `SortedEntityIndex` (S5.5)
//! * `for_each_entity_of` (S5.6)
//!
//! The goal is to *prove* the INVARIANTS §1 contract holds end-to-end at
//! the L3 layer, before S6's scheduler and per-stage systems arrive.

use beast_channels::ChannelRegistry;
use beast_core::Q3232;
use beast_ecs::components::{Age, HealthState, Position};
use beast_ecs::storage::for_each_entity_of;
use beast_ecs::{
    components, Builder, EcsWorld, MarkerKind, Resources, System, SystemStage, WorldExt,
};
use beast_primitives::PrimitiveRegistry;
use specs::Entity;

// Use WorldExt via the re-export for create_entity under-the-hood.
use beast_ecs as _;

/// Toy system: advance every creature's `Age.ticks` by one. Uses the
/// safe-sequential iteration pattern (Pattern A from `storage`).
struct AgingSystem;

impl System for AgingSystem {
    fn name(&self) -> &'static str {
        "aging"
    }

    fn stage(&self) -> SystemStage {
        SystemStage::InputAndAging
    }

    fn run(&mut self, world: &mut EcsWorld, resources: &mut Resources) -> beast_ecs::Result<()> {
        // Snapshot the index so we can iterate without holding a borrow
        // through the mutable storage write.
        let creatures: Vec<Entity> = resources
            .entity_index
            .entities_of(MarkerKind::Creature)
            .collect();
        let mut ages = world.world().write_storage::<Age>();
        for entity in creatures {
            if let Some(age) = ages.get_mut(entity) {
                age.ticks += 1;
            }
        }
        Ok(())
    }
}

/// Build a fixture world with `n` creatures. Each creature has `Age`,
/// `HealthState`, and a deterministic `Position` derived from the seed.
fn build_world(seed: u64, n: usize) -> (EcsWorld, Resources) {
    let mut world = EcsWorld::new();
    components::register_all(&mut world);
    let mut resources = Resources::new(seed, ChannelRegistry::new(), PrimitiveRegistry::new());

    for i in 0..n {
        // Deterministic position derived from index — no RNG draws so
        // the PRNG streams are preserved for later assertions.
        let x = Q3232::from_num(i as i32);
        let y = Q3232::from_num(-(i as i32));
        let entity = world
            .create_entity()
            .with(components::Creature)
            .with(Age::new(0))
            .with(HealthState::full())
            .with(Position::new(x, y))
            .build();
        resources.entity_index.insert(entity, MarkerKind::Creature);
    }
    (world, resources)
}

/// Compute a simple rolling hash over the ages + positions of every
/// creature, visited via the sorted index. The order-of-visitation is
/// what makes this a determinism test — if the index ever yielded a
/// different order, the hash would change.
fn state_hash(world: &EcsWorld, resources: &Resources) -> u64 {
    let ages = world.world().read_storage::<Age>();
    let positions = world.world().read_storage::<Position>();
    let mut hash: u64 = 0xCBF2_9CE4_8422_2325; // FNV-1a offset basis
    for entity in resources.entity_index.entities_of(MarkerKind::Creature) {
        let age = ages.get(entity).copied().unwrap_or_default();
        let pos = positions.get(entity).copied().unwrap_or_default();
        for byte in age.ticks.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        for byte in pos.x.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        for byte in pos.y.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
    }
    hash
}

#[test]
fn aging_system_is_byte_identical_across_runs() {
    // Run 1000 ticks twice from identical seeds and assert the final
    // hash matches. The test is intentionally O(N_creatures * N_ticks)
    // so its completion time bounds our per-tick cost.
    const TICKS: usize = 1_000;
    const CREATURES: usize = 100;

    let run = || {
        let (mut world, mut resources) = build_world(0xDEAD_BEEF, CREATURES);
        let mut aging = AgingSystem;
        for _ in 0..TICKS {
            aging.run(&mut world, &mut resources).expect("run");
            resources.advance_tick();
        }
        (state_hash(&world, &resources), resources.tick_counter.raw())
    };
    let (hash_a, ticks_a) = run();
    let (hash_b, ticks_b) = run();
    assert_eq!(hash_a, hash_b, "state hash must be identical across runs");
    assert_eq!(ticks_a, TICKS as u64);
    assert_eq!(ticks_b, TICKS as u64);
}

#[test]
fn final_ages_are_equal_to_tick_count() {
    // Each creature's age starts at 0; AgingSystem increments once per
    // tick for every creature. After N ticks, every creature must have
    // age = N.
    const TICKS: u64 = 250;
    const CREATURES: usize = 10;
    let (mut world, mut resources) = build_world(42, CREATURES);
    let mut aging = AgingSystem;
    for _ in 0..TICKS {
        aging.run(&mut world, &mut resources).expect("run");
    }
    let ages = world.world().read_storage::<Age>();
    for entity in resources.entity_index.entities_of(MarkerKind::Creature) {
        assert_eq!(ages.get(entity).copied().unwrap_or_default().ticks, TICKS);
    }
}

#[test]
fn for_each_entity_of_visits_in_ascending_order_on_every_call() {
    // Regression guard: build a world, then call for_each_entity_of
    // several times, asserting every call visits entities in the exact
    // same ascending order.
    let (_world, resources) = build_world(7, 32);
    let mut runs: Vec<Vec<Entity>> = Vec::new();
    for _ in 0..5 {
        let mut visited = Vec::new();
        for_each_entity_of(&resources, MarkerKind::Creature, |e| visited.push(e));
        runs.push(visited);
    }
    for i in 1..runs.len() {
        assert_eq!(runs[0], runs[i], "visitation order diverged on run {i}");
    }
    // And ascending by specs Entity (which orders by (index, generation)).
    for pair in runs[0].windows(2) {
        assert!(pair[0] < pair[1], "visitation not ascending: {pair:?}");
    }
}

#[test]
fn different_seeds_do_not_affect_aging_determinism() {
    // AgingSystem doesn't touch PRNG; changing the world_seed should
    // produce identical ages + identical iteration order (the only
    // PRNG draws happen at `Resources::new`, which we run twice).
    let (mut world_a, mut res_a) = build_world(1, 20);
    let (mut world_b, mut res_b) = build_world(2, 20);
    let mut aging = AgingSystem;
    for _ in 0..100 {
        aging.run(&mut world_a, &mut res_a).unwrap();
        aging.run(&mut world_b, &mut res_b).unwrap();
    }
    // Positions + ages are driven by (index, TICKS), not PRNG, so hashes match.
    assert_eq!(state_hash(&world_a, &res_a), state_hash(&world_b, &res_b));
}
