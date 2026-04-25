//! S8.6 starter species survival + biome resources tests
//! (issue #149).
//!
//! Validates the demo criteria from epic #20:
//!
//! * **50 creatures spawned, survive 100 ticks.** — A population of
//!   50 creatures running for 100 ticks must end with the same
//!   count (no extinctions) and no panic.
//! * **No regressions in tick determinism** under a non-trivial
//!   creature population — re-runs the M1 hash stability check
//!   with the seeded population.
//!
//! # Why this lives in beast-sim
//!
//! These are end-to-end stability tests for the simulator under a
//! realistic-shape population. They use only what's on master at
//! the time this PR opens — manual entity creation rather than the
//! upcoming S8.4 spawner — so the file is master-rebaseable.
//! When S8.4 (#163), S8.1 (#159), S8.5 (#162) and S8.3 (#158) all
//! land, a follow-up rewrites the seed routine to call
//! `plan_spawns` + `apply_spawn_plans` against a real
//! `Archipelago` and `STARTER_SPECIES`, and adds a season-cycle
//! assertion. The shape of those follow-up edits is described in
//! comments inline.

use beast_core::Q3232;
use beast_ecs::components::{Age, Creature, Mass, Position};
use beast_ecs::{Builder, EcsWorld, MarkerKind, Resources, System, SystemStage, WorldExt};
use beast_sim::{compute_state_hash, Simulation, SimulationConfig};

/// Trivial aging system mirroring `tests/determinism_test.rs`. Kept
/// local (not exposed from beast-sim) because the production
/// physiology module hasn't shipped yet.
///
/// SYNC: keep in step with `tests/determinism_test.rs::AgingSystem`
/// and `crates/beast-serde/tests/replay_determinism_test.rs::AgingSystem`.
/// All three test fixtures must implement the same arithmetic so the
/// M1, M2, and S8 stability gates verify the same thing.
struct AgingSystem;

impl System for AgingSystem {
    fn name(&self) -> &'static str {
        "starter-survival-aging"
    }
    fn stage(&self) -> SystemStage {
        SystemStage::InputAndAging
    }
    fn run(&mut self, world: &mut EcsWorld, resources: &mut Resources) -> beast_ecs::Result<()> {
        let creatures: Vec<_> = resources
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

/// Build a `Simulation` with `n_creatures` placeholder seed
/// creatures spread uniformly across a `width × height` cell grid.
///
/// Follow-up shape (post S8.1/S8.3/S8.4 merge):
/// ```ignore
/// let mut prng = Prng::from_seed(seed).split_stream(Stream::Worldgen);
/// let archipelago = generate_archipelago(&WorldConfig::default_archipelago(), seed)?;
/// let plans = plan_spawns(&mut prng, archipelago.width, archipelago.height,
///                          n_creatures, |x, y| archipelago.get(x, y).map(BiomeTag::as_str),
///                          species_for_biome)?;
/// apply_spawn_plans(&mut sim, &plans, &genomes)?;
/// ```
fn build_seeded_population(seed: u64, width: u32, height: u32, n_creatures: usize) -> Simulation {
    let mut sim = Simulation::new(SimulationConfig::empty(seed));
    sim.register_system(AgingSystem);

    // Spread n_creatures across the grid in a deterministic stride.
    // A real spawner uses PRNG-rejection placement (S8.4); the
    // stride keeps this test self-contained without losing the
    // "creatures occupy distinct cells" property a real spawner
    // would respect.
    let total_cells = (width as u64) * (height as u64);
    assert!(
        n_creatures as u64 <= total_cells,
        "{n_creatures} creatures don't fit in {width}×{height} = {total_cells} cells",
    );
    let stride = total_cells / (n_creatures as u64);

    for i in 0..n_creatures {
        let cell_index = (i as u64) * stride;
        let cell_x = (cell_index % u64::from(width)) as u32;
        let cell_y = (cell_index / u64::from(width)) as u32;
        // Centre on cell. Mirrors `beast_sim::spawner::cell_to_position`
        // which lands once S8.4 (#163) merges; same +0.5 offset.
        let position = Position::new(
            Q3232::from_num(cell_x).saturating_add(Q3232::from_bits(1_i64 << 31)),
            Q3232::from_num(cell_y).saturating_add(Q3232::from_bits(1_i64 << 31)),
        );
        let entity = sim
            .world_mut()
            .create_entity()
            .with(Creature)
            .with(position)
            .with(Mass::new(Q3232::from_num(1_i32)))
            .with(Age::new(0))
            .build();
        sim.resources_mut()
            .entity_index
            .insert(entity, MarkerKind::Creature);
    }
    sim
}

#[test]
fn fifty_creatures_survive_one_hundred_ticks() {
    // Demo criterion (epic #20): 50 creatures survive 100 ticks.
    // No metabolism / death system in S8 yet, so "survive" means
    // (a) the tick loop runs without panicking and (b) the
    // creature count is unchanged.
    const N_CREATURES: usize = 50;
    const TICKS: usize = 100;
    const SEED: u64 = 0xCAFE_BABE;

    let mut sim = build_seeded_population(SEED, 32, 32, N_CREATURES);

    for _ in 0..TICKS {
        sim.tick().expect("tick must succeed");
    }

    let count = sim
        .resources()
        .entity_index
        .entities_of(MarkerKind::Creature)
        .count();
    assert_eq!(
        count, N_CREATURES,
        "creature count drifted from {N_CREATURES} to {count} over {TICKS} ticks",
    );
}

#[test]
fn ages_advance_uniformly_for_population() {
    // The aging system runs every tick; after N ticks every
    // creature should have age = N. This protects against a
    // future regression where the aging system silently skips
    // entities (e.g., if entity_index iteration order ever
    // diverges from storage order).
    const N_CREATURES: usize = 50;
    const TICKS: u64 = 100;
    const SEED: u64 = 0x1234_5678_9ABC_DEF0;

    let mut sim = build_seeded_population(SEED, 32, 32, N_CREATURES);

    for _ in 0..TICKS {
        sim.tick().expect("tick must succeed");
    }

    let world = sim.world();
    let ages = world.world().read_storage::<Age>();
    let creatures: Vec<_> = sim
        .resources()
        .entity_index
        .entities_of(MarkerKind::Creature)
        .collect();
    assert_eq!(creatures.len(), N_CREATURES);
    for entity in creatures {
        let age = ages.get(entity).expect("age component present");
        assert_eq!(
            age.ticks, TICKS,
            "creature {entity:?} aged to {} ticks, expected {TICKS}",
            age.ticks,
        );
    }
}

#[test]
fn determinism_holds_for_starter_population() {
    // Two sims with the same seed and population shape must
    // produce identical state hashes at every tick. This is the
    // M1 contract restated for the S8 demo population.
    const N_CREATURES: usize = 50;
    const TICKS: usize = 100;
    const SEED: u64 = 0xDEAD_BEEF;

    let mut sim_a = build_seeded_population(SEED, 32, 32, N_CREATURES);
    let mut sim_b = build_seeded_population(SEED, 32, 32, N_CREATURES);

    for tick in 0..TICKS {
        sim_a.tick().expect("a.tick");
        sim_b.tick().expect("b.tick");
        let ha = compute_state_hash(&sim_a);
        let hb = compute_state_hash(&sim_b);
        assert_eq!(
            ha, hb,
            "state hash divergence at tick {tick}: {ha:?} vs {hb:?}",
        );
    }
}

#[test]
fn population_count_is_stable_over_long_run() {
    // 1000 ticks (one full season cycle worth) — extends the
    // 100-tick test to the duration mentioned in the seasonal
    // demo criterion. Once S8.5's climate model lands a follow-up
    // can assert seasonal-cycle invariants here.
    const N_CREATURES: usize = 50;
    const TICKS: usize = 1000;
    const SEED: u64 = 0xCAFE_F00D;

    let mut sim = build_seeded_population(SEED, 32, 32, N_CREATURES);

    for _ in 0..TICKS {
        sim.tick().expect("tick");
    }

    let count = sim
        .resources()
        .entity_index
        .entities_of(MarkerKind::Creature)
        .count();
    assert_eq!(count, N_CREATURES);
}

#[test]
fn single_creature_world_does_not_panic() {
    // Smallest legal population. Stress-tests the entity_index
    // / storage edges that the 50-creature tests skim past.
    let mut sim = build_seeded_population(0, 4, 4, 1);
    for _ in 0..100 {
        sim.tick().expect("tick");
    }
    let count = sim
        .resources()
        .entity_index
        .entities_of(MarkerKind::Creature)
        .count();
    assert_eq!(count, 1);
}
