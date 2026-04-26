//! Shared fixtures for beast-sim integration tests.
//!
//! This module is included via `mod common;` from each integration
//! test (Cargo treats files under `tests/` as separate binaries; the
//! `common/` subdirectory with `mod.rs` is the standard cross-test
//! sharing pattern that does **not** itself produce a `[[test]]`
//! target).
//!
//! Issue #165 tracks promoting these fixtures to a real
//! `beast-test-utils` crate so cross-crate tests (notably
//! `beast-serde::tests::replay_determinism_test`) can drop their
//! own copies. Until then this module covers the two beast-sim
//! integration tests.
//!
//! # Allow dead-code
//!
//! Each integration test only uses a subset of the fixtures, but a
//! tidy `mod common;` from one of them would otherwise warn about
//! "unused" items in the other. The crate-level `#[allow(dead_code)]`
//! on this module silences those warnings — additions here cannot
//! mask real dead code in production sources because this file is
//! `tests/`-only.

#![allow(dead_code)]

use beast_core::Q3232;
use beast_ecs::components::Age;
use beast_ecs::{EcsWorld, MarkerKind, Resources, System, SystemStage, WorldExt};

/// Q3232 representation of `0.5` — the cell-centre offset applied
/// when translating a cell coordinate to a world position. Built
/// from raw bits so no f64 literal appears on the sim path
/// (`I32F32`'s binary point is after bit 31, so `1 << 31 = 0.5`).
/// Mirrors `beast_sim::spawner::HALF_CELL`.
pub const HALF_CELL: Q3232 = Q3232::from_bits(1_i64 << 31);

/// Sequential-pattern aging system (Pattern A). Increments every
/// creature's `Age.ticks` once per run. Per INVARIANTS §1 the
/// iteration is via `entity_index`, not `specs::Join`.
///
/// The `name` is parameterised because integration tests are
/// independent binaries and the human-readable identifier serves the
/// per-test panic message channel — sharing the implementation while
/// keeping the labels distinct keeps any failure attributable to a
/// specific test file.
pub struct AgingSystem {
    pub name: &'static str,
}

impl AgingSystem {
    /// Construct with a stable system label. Use the test-file name
    /// (e.g. `"determinism-test-aging"`) so a panic message points at
    /// the right test binary.
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl System for AgingSystem {
    fn name(&self) -> &'static str {
        self.name
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
