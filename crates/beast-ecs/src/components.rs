//! Component type definitions (S5.2 — issue #106).
//!
//! The simulation's core data types, split across submodules by theme.
//! Marker components (empty structs with `NullStorage`) distinguish
//! "what kind of thing am I" without costing per-entity memory; data
//! components (`DenseVecStorage`) carry per-entity state.
//!
//! Every component in this module implements [`specs::Component`]. Data
//! components additionally derive `Serialize`/`Deserialize` so the save
//! path (S7) can round-trip them without further plumbing.

pub mod biome;
pub mod formation;
pub mod keeper;
pub mod markers;
pub mod physiology;
pub mod spatial;
pub mod traits;

pub use biome::{BiomeCell, BiomeKind};
pub use formation::{Formation, FormationSlot, SLOT_COUNT, SLOT_NAMES};
pub use keeper::{leadership_presence, KeeperState};
pub use markers::{Agent, Biome, Creature, Faction, Pathogen, Settlement};
pub use physiology::{Age, DevelopmentalStage, HealthState, Mass, Species};
pub use spatial::{Position, Velocity};
pub use traits::{GenomeComponent, PhenotypeComponent};

/// Register every component defined in this crate on the given
/// [`crate::EcsWorld`]. Convenience helper so tests and higher-layer
/// code don't have to repeat the fifteen `register_component` calls.
pub fn register_all(world: &mut crate::EcsWorld) {
    world.register_component::<Creature>();
    world.register_component::<Pathogen>();
    world.register_component::<Agent>();
    world.register_component::<Faction>();
    world.register_component::<Settlement>();
    world.register_component::<Biome>();

    world.register_component::<Position>();
    world.register_component::<Velocity>();

    world.register_component::<Age>();
    world.register_component::<Mass>();
    world.register_component::<HealthState>();
    world.register_component::<DevelopmentalStage>();
    world.register_component::<Species>();

    world.register_component::<GenomeComponent>();
    world.register_component::<PhenotypeComponent>();

    world.register_component::<BiomeCell>();

    world.register_component::<Formation>();
    world.register_component::<KeeperState>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EcsWorld;

    #[test]
    fn register_all_does_not_panic_on_fresh_world() {
        let mut world = EcsWorld::new();
        register_all(&mut world);
    }

    #[test]
    fn marker_storage_is_null() {
        // NullStorage is zero-sized per entity — so adding a marker to a
        // million entities costs the same as adding it to one. Locked in
        // so future refactors that accidentally switch to VecStorage fail
        // this test.
        fn is_null<C>()
        where
            C: specs::Component<Storage = specs::NullStorage<C>>,
        {
        }
        is_null::<Creature>();
        is_null::<Pathogen>();
        is_null::<Agent>();
        is_null::<Faction>();
        is_null::<Settlement>();
        is_null::<Biome>();
    }

    #[test]
    fn data_storage_is_densevec() {
        fn is_dense<C>()
        where
            C: specs::Component<Storage = specs::DenseVecStorage<C>>,
        {
        }
        is_dense::<Position>();
        is_dense::<Velocity>();
        is_dense::<Age>();
        is_dense::<Mass>();
        is_dense::<HealthState>();
        is_dense::<DevelopmentalStage>();
        is_dense::<Species>();
        is_dense::<GenomeComponent>();
        is_dense::<PhenotypeComponent>();
        is_dense::<BiomeCell>();
        is_dense::<Formation>();
        is_dense::<KeeperState>();
    }
}
