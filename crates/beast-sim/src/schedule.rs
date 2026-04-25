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

use crate::budget::{BudgetOverrun, Stopwatch};
use crate::error::Result;

/// Budget value meaning "no declared budget" — a system registered via
/// [`SystemSchedule::register`] gets this; nothing ever overruns.
/// Explicit constant (not `u64::MAX`) so `run_tick` can skip the
/// overrun-recording branch cheaply.
const NO_BUDGET: u64 = u64::MAX;

/// Internal entry — a registered system plus its declared budget.
struct SystemEntry {
    system: Box<dyn System + Send>,
    /// Declared budget in microseconds. `NO_BUDGET` means "don't
    /// record overruns for this system".
    budget_us: u64,
}

/// Ordered collection of systems keyed by [`SystemStage`].
///
/// Build with [`SystemSchedule::new`], push systems with
/// [`SystemSchedule::register`] or
/// [`SystemSchedule::register_with_budget`], and drive it each tick
/// with [`SystemSchedule::run_tick`].
#[derive(Default)]
pub struct SystemSchedule {
    // BTreeMap → deterministic stage order; Vec inside → deterministic
    // within-stage order. Boxing is necessary because `System` is dyn;
    // the allocation cost is one-per-system-registration, not per-tick.
    // `Send` on the inner trait object keeps the schedule `Send` so
    // S6.3 can parallelise inside a stage via rayon without fighting
    // the trait bound.
    systems: BTreeMap<SystemStage, Vec<SystemEntry>>,
}

impl SystemSchedule {
    /// Empty schedule — no systems registered.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a system without a budget. Appends to the stage
    /// bucket; within-stage order is registration order. Budgetless
    /// systems never appear in [`crate::TickResult::overruns`].
    pub fn register<S>(&mut self, system: S)
    where
        S: System + Send + 'static,
    {
        self.register_with_budget(system, NO_BUDGET);
    }

    /// Register a system with an explicit budget in microseconds
    /// (S6.7). Any tick where the system's wall-clock duration exceeds
    /// `budget_us` adds an entry to [`crate::TickResult::overruns`].
    /// The scheduler does not act on overruns — it only reports them.
    pub fn register_with_budget<S>(&mut self, system: S, budget_us: u64)
    where
        S: System + Send + 'static,
    {
        self.systems
            .entry(system.stage())
            .or_default()
            .push(SystemEntry {
                system: Box::new(system),
                budget_us,
            });
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
    /// Returns `(stage_durations, overruns)`. Only stages that
    /// actually ran appear in the map; `overruns` lists systems whose
    /// wall-clock duration exceeded their declared `budget_us` in
    /// `(stage, registration_index)` order. Timing is observation
    /// only — it must never influence sim-state control flow
    /// (INVARIANTS §1).
    pub fn run_tick(
        &mut self,
        world: &mut EcsWorld,
        resources: &mut Resources,
    ) -> Result<(BTreeMap<SystemStage, u64>, Vec<BudgetOverrun>)> {
        let mut stage_durations: BTreeMap<SystemStage, u64> = BTreeMap::new();
        let mut overruns: Vec<BudgetOverrun> = Vec::new();
        for (stage, systems) in self.systems.iter_mut() {
            let stage_watch = Stopwatch::start();
            for entry in systems.iter_mut() {
                let system_watch = Stopwatch::start();
                entry.system.run(world, resources)?;
                let actual_us = system_watch.elapsed_us();
                if entry.budget_us != NO_BUDGET && actual_us > entry.budget_us {
                    overruns.push(BudgetOverrun {
                        system: entry.system.name(),
                        stage: *stage,
                        budget_us: entry.budget_us,
                        actual_us,
                    });
                }
            }
            stage_durations.insert(*stage, stage_watch.elapsed_us());
        }
        Ok((stage_durations, overruns))
    }

    /// Number of systems registered across every stage. Useful for
    /// tests and diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.systems.values().map(Vec::len).sum()
    }

    /// `true` iff no systems are registered.
    ///
    /// Since `register` always pushes immediately after `or_default()`,
    /// an empty `Vec` bucket can never persist in the map — so checking
    /// `self.systems.is_empty()` is equivalent and more obvious than
    /// walking every bucket.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
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
            self.log.lock().expect("log mutex poisoned").push(self.name);
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

        assert_eq!(
            *log.lock().expect("log mutex poisoned"),
            vec!["input", "genetics", "ecology"],
        );
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
        assert_eq!(
            *log.lock().expect("log mutex poisoned"),
            vec!["first", "second", "third"]
        );
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
        assert_eq!(*log.lock().expect("log mutex poisoned"), vec!["before"]);
    }

    #[test]
    fn empty_schedule_is_a_noop() {
        let mut schedule = SystemSchedule::new();
        assert!(schedule.is_empty());
        assert_eq!(schedule.len(), 0);
        let (mut w, mut r) = scratch_world_and_resources();
        schedule.run_tick(&mut w, &mut r).expect("empty tick ok");
    }

    /// Test system that sleeps `sleep_us` microseconds before
    /// returning. Used to deterministically exceed a budget.
    struct SlowSystem {
        name: &'static str,
        stage: SystemStage,
        sleep_us: u64,
    }

    impl System for SlowSystem {
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
            std::thread::sleep(std::time::Duration::from_micros(self.sleep_us));
            Ok(())
        }
    }

    #[test]
    fn run_tick_reports_overrun_when_system_exceeds_budget() {
        // SlowSystem sleeps 5_000 µs; budget 500 µs → overrun. The
        // margins are generous (10x) so this isn't flaky on loaded CI.
        let mut schedule = SystemSchedule::new();
        schedule.register_with_budget(
            SlowSystem {
                name: "slow",
                stage: SystemStage::Ecology,
                sleep_us: 5_000,
            },
            500,
        );
        let (mut w, mut r) = scratch_world_and_resources();
        let (_stage_durations, overruns) = schedule.run_tick(&mut w, &mut r).expect("tick");
        assert_eq!(overruns.len(), 1);
        assert_eq!(overruns[0].system, "slow");
        assert_eq!(overruns[0].stage, SystemStage::Ecology);
        assert_eq!(overruns[0].budget_us, 500);
        assert!(
            overruns[0].actual_us > 500,
            "actual_us ({}) should exceed the budget ({})",
            overruns[0].actual_us,
            overruns[0].budget_us
        );
    }

    #[test]
    fn register_without_budget_never_reports_overrun() {
        // A system registered via `register` (no budget) can take as
        // long as it wants without landing in overruns. Guards against
        // accidental regression where `NO_BUDGET` stops behaving as
        // the "don't record" sentinel.
        let mut schedule = SystemSchedule::new();
        schedule.register(SlowSystem {
            name: "no-budget",
            stage: SystemStage::Ecology,
            sleep_us: 2_000,
        });
        let (mut w, mut r) = scratch_world_and_resources();
        let (_stage_durations, overruns) = schedule.run_tick(&mut w, &mut r).expect("tick");
        assert!(overruns.is_empty());
    }

    #[test]
    fn overruns_are_ordered_by_stage_then_registration() {
        // Register two over-budget systems across two stages in
        // reverse declaration order; overruns should come back in
        // (stage, registration_index) order.
        let mut schedule = SystemSchedule::new();
        schedule.register_with_budget(
            SlowSystem {
                name: "ecology-slow",
                stage: SystemStage::Ecology,
                sleep_us: 3_000,
            },
            100,
        );
        schedule.register_with_budget(
            SlowSystem {
                name: "genetics-slow-a",
                stage: SystemStage::Genetics,
                sleep_us: 3_000,
            },
            100,
        );
        schedule.register_with_budget(
            SlowSystem {
                name: "genetics-slow-b",
                stage: SystemStage::Genetics,
                sleep_us: 3_000,
            },
            100,
        );
        let (mut w, mut r) = scratch_world_and_resources();
        let (_stage_durations, overruns) = schedule.run_tick(&mut w, &mut r).expect("tick");
        assert_eq!(overruns.len(), 3);
        let names: Vec<_> = overruns.iter().map(|o| o.system).collect();
        // Genetics comes before Ecology; within Genetics registration order.
        assert_eq!(
            names,
            vec!["genetics-slow-a", "genetics-slow-b", "ecology-slow"]
        );
    }

    #[test]
    fn under_budget_systems_are_not_reported() {
        // System that does nothing; budget is 1 second. Never overruns.
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut schedule = SystemSchedule::new();
        schedule.register_with_budget(
            Recorder {
                name: "fast",
                stage: SystemStage::Genetics,
                log: log.clone(),
            },
            1_000_000,
        );
        let (mut w, mut r) = scratch_world_and_resources();
        let (_stage_durations, overruns) = schedule.run_tick(&mut w, &mut r).expect("tick");
        assert!(overruns.is_empty());
        assert_eq!(*log.lock().expect("log mutex poisoned"), vec!["fast"]);
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
