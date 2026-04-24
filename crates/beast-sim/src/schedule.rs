//! `SystemSchedule` — stage-by-stage dispatcher (S6.2 — issue #116).
//!
//! Groups every [`beast_ecs::System`] by its declared
//! [`beast_ecs::SystemStage`] and runs them in order once per tick.
//! Today's dispatch is serial; S6.3 adds parallel execution inside a
//! stage where systems have no write conflicts.
//!
//! # Determinism
//!
//! Stage iteration uses a [`std::collections::BTreeMap`] keyed by
//! `SystemStage`, so ordering is `InputAndAging → Genetics → … →
//! RenderPrep` without any sort step. Within a stage, systems run in
//! registration order (a stable `Vec`); two stage-mates that race to
//! write the same component resolve with the later-registered one
//! winning, and that winner never changes across runs.

use std::collections::BTreeMap;

use beast_ecs::{EcsWorld, Resources, System, SystemStage};

use crate::budget::Stopwatch;
use crate::error::Result;

/// Ordered collection of systems keyed by [`SystemStage`].
///
/// Build with [`SystemSchedule::new`], push systems with
/// [`SystemSchedule::register`], and drive it each tick with
/// [`SystemSchedule::run_tick`].
#[derive(Default)]
pub struct SystemSchedule {
    // BTreeMap → deterministic stage order; Vec inside → deterministic
    // within-stage order. Boxing is necessary because `System` is dyn;
    // the allocation cost is one-per-system-registration, not per-tick.
    // `Send` on the inner trait object keeps the schedule `Send` so
    // S6.3 can parallelise inside a stage via rayon without fighting
    // the trait bound.
    systems: BTreeMap<SystemStage, Vec<Box<dyn System + Send>>>,
}

impl SystemSchedule {
    /// Empty schedule — no systems registered.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a system. Appends to the stage bucket; within-stage
    /// order is registration order.
    pub fn register<S>(&mut self, system: S)
    where
        S: System + Send + 'static,
    {
        self.systems
            .entry(system.stage())
            .or_default()
            .push(Box::new(system));
    }

    /// Run one tick's worth of systems, stage by stage in declared
    /// order. Aborts on the first error; systems in later stages (or
    /// later within the same stage) do not run.
    ///
    /// Does **not** advance the tick counter — the caller
    /// ([`crate::Simulation::tick`]) owns that step so a partial tick
    /// (aborted mid-way by an error) is still observable via
    /// `resources.tick_counter` being unchanged from the pre-tick
    /// value.
    ///
    /// Returns the per-stage wall-clock-microsecond breakdown (S6.4).
    /// Only stages that actually ran (≥ 1 registered system) appear in
    /// the map. Timing is observation only; it must never influence
    /// sim-state control flow (INVARIANTS §1).
    pub fn run_tick(
        &mut self,
        world: &mut EcsWorld,
        resources: &mut Resources,
    ) -> Result<BTreeMap<SystemStage, u64>> {
        let mut stage_durations: BTreeMap<SystemStage, u64> = BTreeMap::new();
        for (stage, systems) in self.systems.iter_mut() {
            let watch = Stopwatch::start();
            for system in systems.iter_mut() {
                system.run(world, resources)?;
            }
            stage_durations.insert(*stage, watch.elapsed_us());
        }
        Ok(stage_durations)
    }

    /// Number of systems registered across every stage. Useful for
    /// tests and diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.systems.values().map(Vec::len).sum()
    }

    /// `true` iff no systems are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.systems.values().all(Vec::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Mutex};

    use beast_ecs::{EcsWorld, Resources, SystemStage};

    /// Test system that pushes its name onto a shared log whenever it
    /// runs. `Arc<Mutex<_>>` keeps the system `Send` without requiring
    /// unsafe impls.
    struct Recorder {
        name: &'static str,
        stage: SystemStage,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl System for Recorder {
        fn name(&self) -> &'static str {
            self.name
        }
        fn stage(&self) -> SystemStage {
            self.stage
        }
        fn run(
            &mut self,
            _world: &mut EcsWorld,
            _resources: &mut Resources,
        ) -> beast_ecs::Result<()> {
            self.log.lock().unwrap().push(self.name);
            Ok(())
        }
    }

    /// System that always fails; used to verify error propagation.
    struct Failing;
    impl System for Failing {
        fn name(&self) -> &'static str {
            "failing"
        }
        fn stage(&self) -> SystemStage {
            SystemStage::Physiology
        }
        fn run(
            &mut self,
            _world: &mut EcsWorld,
            _resources: &mut Resources,
        ) -> beast_ecs::Result<()> {
            Err(beast_ecs::EcsError::SystemRunFailed {
                system: "failing",
                message: "boom".into(),
            })
        }
    }

    fn scratch_world_and_resources() -> (EcsWorld, Resources) {
        (
            EcsWorld::new(),
            Resources::new(
                0,
                beast_channels::ChannelRegistry::new(),
                beast_primitives::PrimitiveRegistry::new(),
            ),
        )
    }

    #[test]
    fn run_tick_visits_stages_in_declared_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut schedule = SystemSchedule::new();
        // Register in deliberately wrong stage order; schedule sorts by
        // stage (BTreeMap) so Input → Genetics → Ecology emerges.
        schedule.register(Recorder {
            name: "ecology",
            stage: SystemStage::Ecology,
            log: log.clone(),
        });
        schedule.register(Recorder {
            name: "input",
            stage: SystemStage::InputAndAging,
            log: log.clone(),
        });
        schedule.register(Recorder {
            name: "genetics",
            stage: SystemStage::Genetics,
            log: log.clone(),
        });

        let (mut w, mut r) = scratch_world_and_resources();
        schedule.run_tick(&mut w, &mut r).expect("tick");

        assert_eq!(*log.lock().unwrap(), vec!["input", "genetics", "ecology"],);
    }

    #[test]
    fn same_stage_systems_run_in_registration_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut schedule = SystemSchedule::new();
        for name in ["first", "second", "third"] {
            schedule.register(Recorder {
                name,
                stage: SystemStage::Physiology,
                log: log.clone(),
            });
        }
        let (mut w, mut r) = scratch_world_and_resources();
        schedule.run_tick(&mut w, &mut r).expect("tick");
        assert_eq!(*log.lock().unwrap(), vec!["first", "second", "third"]);
    }

    #[test]
    fn run_tick_aborts_on_first_error() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut schedule = SystemSchedule::new();
        schedule.register(Recorder {
            name: "before",
            stage: SystemStage::Genetics,
            log: log.clone(),
        });
        schedule.register(Failing); // Physiology — after Genetics
        schedule.register(Recorder {
            name: "after",
            stage: SystemStage::RenderPrep,
            log: log.clone(),
        });
        let (mut w, mut r) = scratch_world_and_resources();
        let res = schedule.run_tick(&mut w, &mut r);
        assert!(res.is_err(), "failing system should propagate its error");
        // 'before' ran (Genetics < Physiology); 'after' did not
        // (RenderPrep > Physiology and dispatch aborted).
        assert_eq!(*log.lock().unwrap(), vec!["before"]);
    }

    #[test]
    fn empty_schedule_is_a_noop() {
        let mut schedule = SystemSchedule::new();
        assert!(schedule.is_empty());
        assert_eq!(schedule.len(), 0);
        let (mut w, mut r) = scratch_world_and_resources();
        schedule.run_tick(&mut w, &mut r).expect("empty tick ok");
    }

    #[test]
    fn len_counts_across_all_stages() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut schedule = SystemSchedule::new();
        schedule.register(Recorder {
            name: "a",
            stage: SystemStage::Genetics,
            log: log.clone(),
        });
        schedule.register(Recorder {
            name: "b",
            stage: SystemStage::Genetics,
            log: log.clone(),
        });
        schedule.register(Recorder {
            name: "c",
            stage: SystemStage::Ecology,
            log: log.clone(),
        });
        assert_eq!(schedule.len(), 3);
        assert!(!schedule.is_empty());
    }
}
