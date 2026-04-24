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
        Self { world, resources }
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

    /// Convenience: current tick.
    #[must_use]
    pub fn tick(&self) -> TickCounter {
        self.resources.tick_counter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_tick_zero_with_given_seed() {
        let sim = Simulation::new(SimulationConfig::empty(0xABCD));
        assert_eq!(sim.tick().raw(), 0);
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
