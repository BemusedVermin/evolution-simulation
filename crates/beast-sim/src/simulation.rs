//! `Simulation` — the top-level tick-loop owner (S6.1 — issue #114).
//!
//! Holds the [`beast_ecs::EcsWorld`] and [`beast_ecs::Resources`] that
//! every system reads and writes. Construction is a single deterministic
//! step: [`Simulation::new`] seeds the master PRNG, splits per-subsystem
//! streams, registers the fifteen core components, and leaves the tick
//! counter at zero.
//!
//! The actual tick loop is a later story; today `Simulation` is just a
//! correctly-wired pair of types. Every S6 follow-up extends this struct
//! rather than replacing it.

use beast_channels::ChannelRegistry;
use beast_core::TickCounter;
use beast_ecs::{components, EcsWorld, Resources};
use beast_primitives::PrimitiveRegistry;

use crate::budget::{Stopwatch, TickResult};
use crate::error::Result;
use crate::schedule::SystemSchedule;

/// Inputs required to construct a [`Simulation`]. Kept as its own struct
/// so later stories can add optional fields (e.g., a replay journal, a
/// mod list) without breaking every call site.
#[derive(Debug)]
pub struct SimulationConfig {
    /// 64-bit seed for the master PRNG. Save files persist this
    /// verbatim so replay can reconstruct the same derived streams.
    pub world_seed: u64,
    /// Channel registry loaded from manifests (core + mods + genesis).
    pub channels: ChannelRegistry,
    /// Primitive registry loaded from vocabulary manifests.
    pub primitives: PrimitiveRegistry,
}

impl SimulationConfig {
    /// Convenience constructor for tests — empty registries.
    #[must_use]
    pub fn empty(world_seed: u64) -> Self {
        Self {
            world_seed,
            channels: ChannelRegistry::new(),
            primitives: PrimitiveRegistry::new(),
        }
    }
}

/// Top-level simulation state. Owns an [`EcsWorld`] and a
/// [`Resources`]; every later S6 story attaches scheduling /
/// budget-tracking / determinism machinery to this type.
///
/// Construction is a pure function of the [`SimulationConfig`] — two
/// `Simulation` instances built with the same config are
/// byte-indistinguishable, a precondition for the 1000-tick replay gate
/// (INVARIANTS §1).
pub struct Simulation {
    world: EcsWorld,
    resources: Resources,
    schedule: SystemSchedule,
}

impl Simulation {
    /// Build a new simulation from a [`SimulationConfig`].
    ///
    /// Work done here:
    /// 1. `Resources::new` derives one PRNG stream per subsystem from
    ///    `config.world_seed` (one-time split) and initialises the
    ///    tick counter at zero.
    /// 2. `components::register_all` registers the fifteen core
    ///    components on the inner `specs::World` so storages are ready
    ///    before any entity is spawned.
    ///
    /// No entities are created; that's the world-gen sprint (S8).
    ///
    /// # Example
    ///
    /// ```
    /// use beast_sim::{Simulation, SimulationConfig};
    ///
    /// let sim = Simulation::new(SimulationConfig::empty(42));
    /// assert_eq!(sim.resources().tick_counter.raw(), 0);
    /// assert_eq!(sim.resources().world_seed, 42);
    /// ```
    #[must_use]
    pub fn new(config: SimulationConfig) -> Self {
        let mut world = EcsWorld::new();
        components::register_all(&mut world);
        let resources = Resources::new(config.world_seed, config.channels, config.primitives);
        Self {
            world,
            resources,
            schedule: SystemSchedule::new(),
        }
    }

    /// Register a system on the schedule. Thin pass-through to
    /// [`SystemSchedule::register`] so callers who hold a
    /// `&mut Simulation` don't need to reach into `.schedule_mut()`
    /// explicitly.
    ///
    /// Either path — `sim.register_system(s)` or
    /// `sim.schedule_mut().register(s)` — drops `s` into the same
    /// internal `SystemSchedule` and is picked up by the next
    /// [`Self::tick`] call. The pass-through exists for ergonomics;
    /// the `schedule_mut` path is useful when several systems are
    /// being registered in a loop or via a configuration helper.
    pub fn register_system<S>(&mut self, system: S)
    where
        S: beast_ecs::System + Send + 'static,
    {
        self.schedule.register(system);
    }

    /// Immutable view of the registered schedule. Useful for tests and
    /// diagnostics that want to count systems without mutating state.
    #[must_use]
    pub fn schedule(&self) -> &SystemSchedule {
        &self.schedule
    }

    /// Mutable view of the registered schedule — register systems
    /// directly when the pass-through is insufficient.
    ///
    /// Equivalent to [`Self::register_system`] for single-system
    /// registration: both call sites mutate the same
    /// [`SystemSchedule`] instance, which [`Self::tick`] drives every
    /// frame. Choose `register_system` for one-off ergonomics and
    /// `schedule_mut` when you want to iterate over a config-driven
    /// system list. There is no "register me but skip me on tick"
    /// variant; the schedule is always the source of truth for what
    /// `tick()` runs.
    pub fn schedule_mut(&mut self) -> &mut SystemSchedule {
        &mut self.schedule
    }

    /// Advance the simulation by one tick.
    ///
    /// Runs every registered system in declared [`beast_ecs::SystemStage`]
    /// order, then increments the tick counter. Returns a
    /// [`TickResult`] with the total + per-stage wall-clock durations
    /// in microseconds — observation only, never fed back into sim
    /// state.
    ///
    /// # Errors
    ///
    /// Returns the first error any system reports. The counter is
    /// **not** advanced on error, but systems that ran before the
    /// failure retain their mutations — the world is in a partial
    /// state. Callers that need rollback semantics must snapshot
    /// before calling `tick()`.
    pub fn tick(&mut self) -> Result<TickResult> {
        let watch = Stopwatch::start();
        let (stage_durations, overruns) = self
            .schedule
            .run_tick(&mut self.world, &mut self.resources)?;
        self.resources.advance_tick();
        Ok(TickResult {
            tick: self.resources.tick_counter,
            duration_us: watch.elapsed_us(),
            stage_durations,
            overruns,
        })
    }

    /// Register a system with an explicit microsecond budget (S6.7).
    /// Any tick where the system's wall-clock duration exceeds
    /// `budget_us` will be reported in [`TickResult::overruns`]. The
    /// scheduler does not act on overruns — that's the caller's
    /// decision.
    pub fn register_system_with_budget<S>(&mut self, system: S, budget_us: u64)
    where
        S: beast_ecs::System + Send + 'static,
    {
        self.schedule.register_with_budget(system, budget_us);
    }

    /// Immutable view of the ECS world.
    #[must_use]
    pub fn world(&self) -> &EcsWorld {
        &self.world
    }

    /// Mutable view of the ECS world. Needed by spawner code (S8) and
    /// systems that run outside the scheduler loop during init.
    pub fn world_mut(&mut self) -> &mut EcsWorld {
        &mut self.world
    }

    /// Immutable view of the simulation resources.
    #[must_use]
    pub fn resources(&self) -> &Resources {
        &self.resources
    }

    /// Mutable view of the simulation resources.
    pub fn resources_mut(&mut self) -> &mut Resources {
        &mut self.resources
    }

    /// Convenience: current tick counter value. Renamed from `tick`
    /// when S6.2 gave [`Self::tick`] its new meaning (advance one
    /// step).
    #[must_use]
    pub fn current_tick(&self) -> TickCounter {
        self.resources.tick_counter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_tick_zero_with_given_seed() {
        let sim = Simulation::new(SimulationConfig::empty(0xABCD));
        assert_eq!(sim.current_tick().raw(), 0);
        assert_eq!(sim.resources().world_seed, 0xABCD);
    }

    #[test]
    fn register_all_runs_during_new() {
        // If components::register_all were skipped, reading the storage
        // below would panic with shred's MetaTable error. This test
        // proves Simulation::new wires registration.
        use beast_ecs::components::Position;
        use beast_ecs::WorldExt;

        let sim = Simulation::new(SimulationConfig::empty(1));
        let _storage = sim.world().world().read_storage::<Position>();
    }

    #[test]
    fn tick_advances_the_counter_exactly_once() {
        let mut sim = Simulation::new(SimulationConfig::empty(5));
        for expected in 1..=20 {
            let result = sim.tick().expect("tick");
            assert_eq!(sim.current_tick().raw(), expected);
            assert_eq!(result.tick.raw(), expected);
        }
    }

    #[test]
    fn tick_with_no_systems_succeeds_and_advances() {
        let mut sim = Simulation::new(SimulationConfig::empty(5));
        assert!(sim.schedule().is_empty());
        let result = sim.tick().expect("empty tick ok");
        assert_eq!(sim.current_tick().raw(), 1);
        assert_eq!(result.tick.raw(), 1);
        // No systems ran → no stage entries.
        assert!(result.stage_durations.is_empty());
    }

    #[test]
    fn tick_result_reports_stage_and_tick_metadata() {
        use beast_ecs::{EcsWorld, Resources, System, SystemStage};

        struct Noop(SystemStage);
        impl System for Noop {
            fn name(&self) -> &'static str {
                "noop"
            }
            fn stage(&self) -> SystemStage {
                self.0
            }
            fn run(
                &mut self,
                _world: &mut EcsWorld,
                _resources: &mut Resources,
            ) -> beast_ecs::Result<()> {
                Ok(())
            }
        }

        let mut sim = Simulation::new(SimulationConfig::empty(11));
        sim.register_system(Noop(SystemStage::Genetics));
        sim.register_system(Noop(SystemStage::Ecology));

        let r = sim.tick().expect("tick");
        assert_eq!(r.tick.raw(), 1);
        // Both stages ran exactly once — each appears in the map.
        assert!(r.stage_durations.contains_key(&SystemStage::Genetics));
        assert!(r.stage_durations.contains_key(&SystemStage::Ecology));
        assert_eq!(r.stage_durations.len(), 2);
        // Per-stage + total are monotonically consistent (total ≥ sum).
        let sum: u64 = r.stage_durations.values().sum();
        assert!(
            r.duration_us >= sum,
            "total ({}) must be ≥ sum of stages ({})",
            r.duration_us,
            sum
        );
    }

    #[test]
    fn two_sims_with_same_seed_have_identical_first_draws() {
        let mut a = Simulation::new(SimulationConfig::empty(7));
        let mut b = Simulation::new(SimulationConfig::empty(7));
        for _ in 0..32 {
            assert_eq!(
                a.resources_mut().rng_genetics.next_u64(),
                b.resources_mut().rng_genetics.next_u64()
            );
        }
    }

    #[test]
    fn world_mut_is_exposed_for_spawners() {
        // Smoke: spawner-style code should be able to grab world_mut and
        // use the usual specs builder chain against it.
        use beast_ecs::components::{Creature, Mass};
        use beast_ecs::{Builder, WorldExt};

        let mut sim = Simulation::new(SimulationConfig::empty(3));
        let world = sim.world_mut();
        let _entity = world
            .create_entity()
            .with(Creature)
            .with(Mass::new(beast_core::Q3232::from_num(10)))
            .build();

        let mass_storage = sim
            .world()
            .world()
            .read_storage::<beast_ecs::components::Mass>();
        assert_eq!(mass_storage.count(), 1);
    }
}
