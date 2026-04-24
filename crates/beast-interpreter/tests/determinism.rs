//! Integration test: the 1000-run determinism gate (S4.6 — issue #60).
//!
//! Runs [`beast_interpreter::interpret_phenotype`] 1000 times against a
//! fixture world (5 channels, 4 primitives, 6 hooks, 20 synthetic phenotypes)
//! and asserts byte-identical `Vec<PrimitiveEffect>` on every run.
//!
//! This is the per-story version of the 1000-tick replay gate described in
//! `CLAUDE.md` / `INVARIANTS.md` §1. The full tick loop gate lands with
//! `beast-sim` in S6.

mod common;

use beast_core::{EntityId, Q3232};
use beast_interpreter::interpret_phenotype;

use crate::common::{standard_phenotypes, standard_world};

/// Run the pipeline once across every fixture phenotype and return the
/// concatenated per-phenotype effect vectors. Used as the deterministic
/// "expected value" that every subsequent run must equal byte-for-byte.
fn run_once() -> Vec<Vec<beast_primitives::PrimitiveEffect>> {
    let world = standard_world();
    let phenotypes = standard_phenotypes();
    phenotypes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            interpret_phenotype(
                p,
                &world.hooks,
                &world.channel_registry,
                &world.primitive_registry,
                EntityId::new(i as u32),
            )
            .expect("fixture world emits valid primitives")
        })
        .collect()
}

#[test]
fn one_thousand_runs_produce_byte_identical_output() {
    let baseline = run_once();

    // Sanity: the fixture must actually emit primitives, otherwise the test
    // would be vacuously green.
    let total_effects: usize = baseline.iter().map(|v| v.len()).sum();
    assert!(
        total_effects > 0,
        "fixture must produce at least one primitive across the 20 phenotypes"
    );

    for run in 0..1000 {
        let actual = run_once();
        assert_eq!(
            actual, baseline,
            "run {run} diverged from baseline — determinism invariant broken"
        );
    }
}

#[test]
fn output_is_sorted_by_primitive_id() {
    let effects = run_once();
    for (phenotype_idx, per_phenotype) in effects.iter().enumerate() {
        let ids: Vec<&str> = per_phenotype
            .iter()
            .map(|e| e.primitive_id.as_str())
            .collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(
            ids, sorted,
            "phenotype {phenotype_idx}: primitive ids must be sorted"
        );
    }
}

#[test]
fn input_phenotype_is_not_mutated() {
    // Regression: the pipeline clones internally before applying the
    // scale-band filter. Verify the caller's copy is unchanged after a call.
    let world = standard_world();
    let phenotypes = standard_phenotypes();
    for (i, p) in phenotypes.iter().enumerate() {
        let before = p.global_channels.clone();
        let before_mass = p.body_mass_kg;
        let _ = interpret_phenotype(
            p,
            &world.hooks,
            &world.channel_registry,
            &world.primitive_registry,
            EntityId::new(i as u32),
        )
        .unwrap();
        assert_eq!(
            p.global_channels, before,
            "phenotype {i}: global_channels mutated"
        );
        assert_eq!(p.body_mass_kg, before_mass, "phenotype {i}: mass mutated");
    }
}

#[test]
fn every_effect_has_registered_primitive_id() {
    let world = standard_world();
    let effects = run_once();
    for per_phenotype in &effects {
        for effect in per_phenotype {
            assert!(
                world.primitive_registry.contains(&effect.primitive_id),
                "primitive `{}` not in registry",
                effect.primitive_id,
            );
        }
    }
}

#[test]
fn activation_cost_is_present_on_every_effect() {
    // Activation cost is now a first-class field on `PrimitiveEffect` (#67);
    // the emitter must populate it non-zero for every emission against the
    // fixture world.
    //
    // The `> Q3232::ZERO` assertion is fixture-specific: every primitive in
    // `common::standard_world()` declares a positive `base_metabolic_cost`,
    // so any emission must carry a strictly positive cost. A zero-cost
    // primitive (none exist in the fixture today) would trip this check
    // even on a correct emission — add it to the fixture first, or relax
    // the assertion to `>= Q3232::ZERO`, before introducing one.
    let effects = run_once();
    for per_phenotype in &effects {
        for effect in per_phenotype {
            assert!(
                effect.activation_cost > Q3232::ZERO,
                "effect `{}` has zero activation_cost",
                effect.primitive_id,
            );
        }
    }
}
