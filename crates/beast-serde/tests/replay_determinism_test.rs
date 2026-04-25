//! In-process M2 determinism gate (S7.6 — issue #134).
//!
//! Closing test for Sprint 7. Proves that a `SaveFile` re-hydrated
//! into a fresh [`beast_sim::Simulation`] produces a bit-identical
//! hash trace when run forward — at tick 0, at mid-run, and against
//! the on-disk path I/O.
//!
//! # Scope
//!
//! * In-memory save → load round-trip plus continued execution.
//! * On-disk save → load round-trip via `save_to_path` /
//!   `load_from_path` (atomic write through `tempfile::persist`).
//! * Mid-run snapshot: snapshot at tick 50, continue to tick 100,
//!   trace tail must equal the original trace tail.
//! * `ReplayJournal` consistency: a journal recorded against the same
//!   seed must round-trip JSON byte-identically.
//!
//! # What this gate is *not*
//!
//! Both sims run inside the **same OS process**. INVARIANTS §1
//! describes a cross-process gate (serialize → spawn new process →
//! load → replay → compare). That is tracked in issue #154 and is the
//! prerequisite for ticking the audit checklist item
//! `Determinism CI test: replay divergence causes test failure +
//! binary diff report`. Today's gate is the in-process slice of M2;
//! the cross-process slice is deferred.
//!
//! Other deferred slices:
//! * No real input replay — the MVP has no input source, so
//!   `ReplayJournal::events` is empty across the test. Real input
//!   variants land with the avatar/UI sprints (S13+) and will extend
//!   this suite.
//! * No migration round-trip — the registry is empty until the first
//!   `format_version` bump; tracked in issue #155.
//! * No cross-platform reproducibility — CI runs one platform per
//!   shard; multi-platform replay parity is out of scope for M2.

use beast_channels::ChannelRegistry;
use beast_core::Q3232;
use beast_ecs::components::{Age, Creature, Mass};
use beast_ecs::{Builder, MarkerKind, WorldExt};
use beast_primitives::PrimitiveRegistry;
use beast_serde::{
    load_from_path, load_game, save_game, save_to_path, ReplayJournal, REPLAY_FORMAT_VERSION,
};
use beast_sim::{compute_state_hash, Simulation, SimulationConfig};

/// Aging system mirroring the one in
/// `beast-sim/tests/determinism_test.rs`. Defined locally to avoid
/// publishing a test-only system from beast-sim.
///
/// SYNC: keep in step with the `AgingSystem` in
/// `beast-sim/tests/determinism_test.rs`. M1 (sim-only) and M2
/// (save-layer) gates assume identical tick behavior; if the beast-sim
/// copy changes its arithmetic, update this one too or the two
/// milestones silently drift.
struct AgingSystem;

impl beast_ecs::System for AgingSystem {
    fn name(&self) -> &'static str {
        "replay-determinism-aging"
    }
    fn stage(&self) -> beast_ecs::SystemStage {
        beast_ecs::SystemStage::InputAndAging
    }
    fn run(
        &mut self,
        world: &mut beast_ecs::EcsWorld,
        resources: &mut beast_ecs::Resources,
    ) -> beast_ecs::Result<()> {
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

/// Run `sim` forward `ticks` ticks, capturing `(tick, hash)` after
/// each tick.
fn capture_trace(sim: &mut Simulation, ticks: usize) -> Vec<(u64, [u8; 32])> {
    let mut trace = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        let r = sim.tick().expect("tick");
        trace.push((r.tick.raw(), compute_state_hash(sim)));
    }
    trace
}

#[test]
fn save_at_tick_zero_then_replay_matches_original_trace() {
    // 20 creatures × 100 ticks: long enough to surface PRNG drift but
    // short enough to keep the test under a second. The other tests
    // use smaller fixtures because they exercise additional layers
    // (disk I/O, mid-run snapshot logic).
    const TICKS: usize = 100;
    const CREATURES: usize = 20;
    const SEED: u64 = 0xCAFE_F00D_DEAD_BEEF;

    // Original run: capture full trace.
    let mut sim_a = build_fixture(SEED, CREATURES);
    let trace_a = capture_trace(&mut sim_a, TICKS);

    // Replay: save at tick 0 (fresh fixture), load into sim_b, run TICKS.
    let sim_b_pre = build_fixture(SEED, CREATURES);
    let save = save_game(&sim_b_pre).expect("save");
    let mut sim_b =
        load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()).expect("load");

    // After load, sim_b's state hash equals sim_a's pre-tick hash.
    assert_eq!(
        compute_state_hash(&sim_b_pre),
        compute_state_hash(&sim_b),
        "load did not preserve initial state hash"
    );

    // sim_b must reattach the AgingSystem; the schedule is not part of
    // the save (systems are code, not data). This is documented
    // behaviour: the save stores world + resources, the application
    // re-installs systems on load.
    sim_b.register_system(AgingSystem);

    let trace_b = capture_trace(&mut sim_b, TICKS);

    assert_eq!(trace_a, trace_b, "replay trace diverged from original");
}

#[test]
fn mid_run_snapshot_continues_bit_identically() {
    // 16 creatures: smaller than the tick-0 test so the
    // build → run → snapshot → load → continue pipeline stays well
    // under a second even with the extra mid-run save/load step.
    const TOTAL: usize = 100;
    const SNAP_AT: usize = 50;
    const CREATURES: usize = 16;
    const SEED: u64 = 0x1234_5678_9ABC_DEF0;

    // Original run: capture full TOTAL-tick trace.
    let mut sim_a = build_fixture(SEED, CREATURES);
    let trace_a = capture_trace(&mut sim_a, TOTAL);

    // Snapshot run: build a fresh sim, advance to SNAP_AT ticks, snapshot.
    let mut sim_pre = build_fixture(SEED, CREATURES);
    // Advance sim_pre to SNAP_AT; trace intentionally discarded — we
    // only need the post-tick state to snapshot, not the per-tick
    // hashes. (`let _` here is not a silenced `Result`; `capture_trace`
    // returns a `Vec<(u64, [u8; 32])>`.)
    let _ = capture_trace(&mut sim_pre, SNAP_AT);
    let save = save_game(&sim_pre).expect("save");

    // Hydrate and re-attach systems.
    let mut sim_post =
        load_game(save, ChannelRegistry::new(), PrimitiveRegistry::new()).expect("load");
    sim_post.register_system(AgingSystem);

    // Continue the loaded sim for the remaining ticks.
    let trace_tail = capture_trace(&mut sim_post, TOTAL - SNAP_AT);

    // The tail must equal the corresponding tail of trace_a.
    assert_eq!(
        trace_tail.len(),
        TOTAL - SNAP_AT,
        "tail length mismatch: expected {}, got {}",
        TOTAL - SNAP_AT,
        trace_tail.len()
    );
    for (i, (expected, actual)) in trace_a[SNAP_AT..].iter().zip(trace_tail.iter()).enumerate() {
        assert_eq!(
            expected,
            actual,
            "mid-run replay diverged at tick offset {}: original {:?} vs continued {:?}",
            i + SNAP_AT + 1,
            expected,
            actual,
        );
    }
}

#[test]
fn on_disk_save_load_round_trip_preserves_replay() {
    // 5 creatures × 25 ticks: kept small because this test pays for
    // tempdir creation and disk I/O on top of the simulation work;
    // the save→load logic is the variable under test, not throughput.
    // SEED differs from the in-memory tests on purpose so a regression
    // in tempfile-backed atomic rename surfaces against an independent
    // PRNG trajectory rather than re-using a hash already covered.
    const TICKS: usize = 25;
    const CREATURES: usize = 5;
    const SEED: u64 = 0x99;

    let mut sim_a = build_fixture(SEED, CREATURES);
    let trace_a = capture_trace(&mut sim_a, TICKS);

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("snapshot.bsv");

    let sim_b_pre = build_fixture(SEED, CREATURES);
    save_to_path(&sim_b_pre, &path).expect("save_to_path");
    let mut sim_b =
        load_from_path(&path, ChannelRegistry::new(), PrimitiveRegistry::new()).expect("load");
    sim_b.register_system(AgingSystem);

    let trace_b = capture_trace(&mut sim_b, TICKS);
    assert_eq!(trace_a, trace_b);
}

#[test]
fn replay_journal_round_trips_for_seeded_run() {
    // The MVP has no real input events, so this test only exercises
    // the journal's metadata + JSON round-trip against an empty events
    // map. Per-tick ordering guarantees are unobservable until real
    // input variants land in S13+; that extension is owned by the
    // avatar/UI sprints, not this gate.
    let journal = ReplayJournal::new(0xDEAD);
    assert_eq!(journal.format_version, REPLAY_FORMAT_VERSION);
    assert_eq!(journal.world_seed, 0xDEAD);

    let s = journal.to_json().expect("json");
    let parsed = ReplayJournal::from_json(&s).expect("parse");
    assert_eq!(journal, parsed);
}
