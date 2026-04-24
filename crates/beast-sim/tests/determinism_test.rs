//! M1 Core Loop determinism gate (S6.6 — issue #123).
//!
//! The contract this file enforces: given identical
//! [`SimulationConfig`], running the tick loop twice must produce
//! bit-identical [`beast_sim::compute_state_hash`] at **every** tick.
//! If any of these tests ever regresses, the simulation is no longer
//! deterministic and replay is broken — so this file is the CI gate
//! for the whole determinism invariant (INVARIANTS §1) until S7's
//! save/load layer extends it across process boundaries.
//!
//! # Scope (explicitly)
//!
//! * Only the tick loop + the toy systems registered below. The full
//!   8-stage game-system pipeline lands per-system in Phase 3.
//! * Only in-process replay. Cross-process, save-to-disk replay lands
//!   in S7.

use beast_core::Q3232;
use beast_ecs::components::{Age, Creature, Mass};
use beast_ecs::{Builder, EcsWorld, MarkerKind, Resources, System, SystemStage, WorldExt};
use beast_sim::{compute_state_hash, Simulation, SimulationConfig};

/// Sequential-pattern aging system (Pattern A). Increments every
/// creature's `Age.ticks` once per run. Per INVARIANTS §1 the
/// iteration is via `entity_index`, not `specs::Join`.
struct AgingSystem;

impl System for AgingSystem {
    fn name(&self) -> &'static str {
        "determinism-test-aging"
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

fn build_fixture(seed: u64, n_creatures: usize) -> Simulation {
    let mut sim = Simulation::new(SimulationConfig::empty(seed));
    sim.register_system(AgingSystem);
    // Deterministic entity creation: positions derived from index so
    // the world content is a pure function of `(seed, n_creatures)`.
    for i in 0..n_creatures {
        let entity = sim
            .world_mut()
            .create_entity()
            .with(Creature)
            .with(Age::new(0))
            .with(Mass::new(Q3232::from_num((i + 1) as i32)))
            .build();
        sim.resources_mut()
            .entity_index
            .insert(entity, MarkerKind::Creature);
    }
    sim
}

/// Capture `(tick, hash)` every tick for `ticks` ticks. Returns the
/// trace that two replays must byte-match.
fn capture_trace(mut sim: Simulation, ticks: usize) -> Vec<(u64, [u8; 32])> {
    let mut trace = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        let result = sim.tick().expect("tick ok");
        trace.push((result.tick.raw(), compute_state_hash(&sim)));
    }
    trace
}

#[test]
fn replay_is_bit_identical_across_100_ticks() {
    const TICKS: usize = 100;
    const CREATURES: usize = 50;
    const SEED: u64 = 0xDEAD_BEEF_CAFE_F00D;

    let trace_a = capture_trace(build_fixture(SEED, CREATURES), TICKS);
    let trace_b = capture_trace(build_fixture(SEED, CREATURES), TICKS);

    assert_eq!(trace_a.len(), TICKS);
    assert_eq!(trace_b.len(), TICKS);
    for i in 0..TICKS {
        assert_eq!(
            trace_a[i], trace_b[i],
            "divergence at tick {}: {:?} vs {:?}",
            trace_a[i].0, trace_a[i], trace_b[i]
        );
    }
}

#[test]
fn different_seeds_diverge_within_a_handful_of_ticks() {
    const TICKS: usize = 8;
    const CREATURES: usize = 10;

    let trace_a = capture_trace(build_fixture(1, CREATURES), TICKS);
    let trace_b = capture_trace(build_fixture(2, CREATURES), TICKS);

    // Preamble carries the seed, so hashes differ from tick 1 onward
    // even before any RNG draws happen.
    assert_ne!(
        trace_a[0].1, trace_b[0].1,
        "different seeds should produce different hashes from tick 1"
    );
    // Full trace divergence: all 8 entries differ in the hash half.
    for i in 0..TICKS {
        assert_ne!(
            trace_a[i].1,
            trace_b[i].1,
            "traces unexpectedly matched at tick {} under different seeds",
            i + 1
        );
    }
}

#[test]
fn hash_changes_every_tick_when_state_mutates() {
    // With AgingSystem incrementing every creature every tick, the
    // state hash must strictly change tick-over-tick. An unchanged hash
    // means the aging didn't take effect — silent determinism break.
    const TICKS: usize = 50;
    const CREATURES: usize = 5;

    let trace = capture_trace(build_fixture(42, CREATURES), TICKS);

    for i in 1..TICKS {
        assert_ne!(
            trace[i - 1].1,
            trace[i].1,
            "hash did not advance between tick {} and tick {}",
            trace[i - 1].0,
            trace[i].0,
        );
    }
}

#[test]
fn empty_world_replay_still_advances_preamble_hash() {
    // No entities, no systems. Just the tick counter advancing. The
    // hash must still change every tick (preamble carries the tick
    // counter) and both replays must produce the identical trace.
    const TICKS: usize = 25;
    let run = || {
        let sim = Simulation::new(SimulationConfig::empty(99));
        capture_trace(sim, TICKS)
    };
    let a = run();
    let b = run();
    assert_eq!(a, b);
    for i in 1..TICKS {
        assert_ne!(
            a[i - 1].1,
            a[i].1,
            "empty-world hash stalled between tick {} and tick {}",
            a[i - 1].0,
            a[i].0,
        );
    }
}

#[test]
fn tick_counter_in_trace_is_monotonic_one_per_tick() {
    // Sanity: the tick numbers in the trace are 1..=TICKS, no gaps,
    // no repeats. A skipped or repeated tick would silently invalidate
    // every assertion above.
    const TICKS: usize = 100;
    let trace = capture_trace(build_fixture(7, 0), TICKS);
    for (i, (tick, _hash)) in trace.iter().enumerate() {
        assert_eq!(*tick, (i + 1) as u64, "tick counter skipped at index {i}");
    }
}
