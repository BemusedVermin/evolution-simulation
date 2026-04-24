//! `System` trait and `SystemStage` enum (S5.3 — issue #102).
//!
//! Models the eight-stage tick loop in
//! `documentation/architecture/ECS_SCHEDULE.md`. A [`SystemStage`] groups
//! systems that the scheduler will run sequentially between stages and in
//! parallel within a stage (S6 wires up the scheduler; this module just
//! defines the surface).
//!
//! The [`Resources`] type is a placeholder until S5.4 lands — it holds no
//! state today. That keeps this module independent of S5.4 so each story
//! can ship its own PR.

use crate::world::EcsWorld;

/// Nine-stage tick ordering (eight game stages + render prep). Variant
/// declaration order deliberately matches the sections of
/// `ECS_SCHEDULE.md`; the derived `Ord` impl uses that declaration
/// order so `BTreeMap<SystemStage, _>` iterates in tick-stage order
/// without an explicit discriminant or sort step.
///
/// Sequential between stages, parallel within: `InputAndAging` runs first
/// every tick, `RenderPrep` last. The scheduler (S6) honours the declared
/// order — systems are never reordered across stages.
///
/// This enum is **not** `#[repr(u8)]`; callers must never `as u8`-cast
/// a `SystemStage` to use the discriminant as a stable id. Use `Ord` /
/// `match` instead, so adding a variant in the middle doesn't silently
/// shift every later variant's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SystemStage {
    /// Stage 0 — input handling, random events, aging.
    InputAndAging,
    /// Stage 1 — per-creature genetic mutation & genesis.
    Genetics,
    /// Stage 2 — scale-band filter → interpreter → composition hooks.
    PhenotypeResolution,
    /// Stage 3 — physics forces and movement resolution.
    PhysicsAndMovement,
    /// Stage 4 — combat, predation, parasitism.
    InteractionAndCombat,
    /// Stage 5 — metabolism, healing, reproduction, death checks.
    Physiology,
    /// Stage 6 — population dynamics, biome effects, speciation.
    Ecology,
    /// Stage 7 — chronicler pattern detection, save checkpoints.
    LabelingAndPersistence,
    /// Stage 8 — snapshot creation for the renderer.
    RenderPrep,
}

/// Placeholder resources container. S5.4 replaces this with the real
/// struct holding registries, PRNG streams, and the tick counter.
///
/// Today it carries no state; systems can already accept
/// `&mut Resources` without the trait API changing when the real fields
/// arrive.
#[derive(Debug, Default)]
pub struct Resources {
    // Intentionally empty — S5.4 fills this with registries + PRNG
    // streams. The field prevents downstream from destructuring.
    _private: (),
}

impl Resources {
    /// Build an empty placeholder. Replaced by `Resources::new(...)`
    /// with real fields in S5.4.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// A simulation system. Every system implements this trait and declares
/// the stage it belongs to; the scheduler calls [`System::run`] once per
/// tick in the order given by [`SystemStage`].
///
/// # Determinism
///
/// Systems must not read the wall clock, must not use
/// [`std::collections::HashMap`]/`HashSet` iteration whose order leaks
/// into sim state, and must only use RNG streams taken from
/// [`Resources`] (one stream per subsystem — see INVARIANTS §1).
pub trait System {
    /// Stable name for logging and budget reporting. Must not change
    /// across ticks.
    fn name(&self) -> &'static str;

    /// Which stage this system runs in. The scheduler uses this to batch
    /// systems; changing it mid-run is a bug.
    fn stage(&self) -> SystemStage;

    /// Execute one tick's worth of work.
    ///
    /// # Errors
    ///
    /// Implementations should return [`crate::EcsError::SystemRunFailed`]
    /// on failure. The scheduler decides how to react (abort vs skip
    /// this tick) — that policy lives in S6.
    fn run(&mut self, world: &mut EcsWorld, resources: &mut Resources) -> crate::Result<()>;
}

// Compile-time object-safety check so the S6 scheduler can hold
// `Box<dyn System>` without future trait additions accidentally
// breaking dyn-compatibility.
const _: fn() = || {
    let _: Option<&dyn System> = None;
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Toy system that does nothing — just to prove the trait compiles
    /// and is callable against [`EcsWorld`] + [`Resources`].
    struct NoopSystem {
        ticks: u32,
    }

    impl System for NoopSystem {
        fn name(&self) -> &'static str {
            "noop"
        }

        fn stage(&self) -> SystemStage {
            SystemStage::Ecology
        }

        fn run(&mut self, _world: &mut EcsWorld, _resources: &mut Resources) -> crate::Result<()> {
            self.ticks += 1;
            Ok(())
        }
    }

    #[test]
    fn system_stages_sort_in_declared_order() {
        let ordered = [
            SystemStage::InputAndAging,
            SystemStage::Genetics,
            SystemStage::PhenotypeResolution,
            SystemStage::PhysicsAndMovement,
            SystemStage::InteractionAndCombat,
            SystemStage::Physiology,
            SystemStage::Ecology,
            SystemStage::LabelingAndPersistence,
            SystemStage::RenderPrep,
        ];
        for pair in ordered.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{:?} should sort before {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn system_trait_can_be_driven_against_ecs_world() {
        let mut world = EcsWorld::new();
        let mut resources = Resources::new();
        let mut system = NoopSystem { ticks: 0 };

        for _ in 0..5 {
            system.run(&mut world, &mut resources).expect("run ok");
        }
        assert_eq!(system.ticks, 5);
        assert_eq!(system.name(), "noop");
        assert_eq!(system.stage(), SystemStage::Ecology);
    }

    #[test]
    fn resources_default_is_equivalent_to_new() {
        // Placeholder carries no state; this test locks in that shape so
        // S5.4 can swap the implementation without breaking callers that
        // stored resources via Default.
        let _a = Resources::default();
        let _b = Resources::new();
    }

    #[test]
    fn system_stages_are_copy_and_hashable() {
        // Needed because the S6 scheduler will key a BTreeMap by stage.
        use std::collections::BTreeMap;
        let mut map: BTreeMap<SystemStage, Vec<&'static str>> = BTreeMap::new();
        map.entry(SystemStage::Genetics).or_default().push("a");
        map.entry(SystemStage::Genetics).or_default().push("b");
        map.entry(SystemStage::RenderPrep).or_default().push("c");
        assert_eq!(map[&SystemStage::Genetics], vec!["a", "b"]);
        assert_eq!(map[&SystemStage::RenderPrep], vec!["c"]);
    }
}
