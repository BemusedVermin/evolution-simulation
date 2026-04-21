//! Property-based tests on random phenotypes (S4.6 — issue #60).
//!
//! Generates 100 random phenotypes against the standard fixture world and
//! asserts every invariant the integration tests require of
//! [`beast_interpreter::interpret_phenotype`]:
//!
//! * never panics
//! * always returns `Ok`
//! * every `PrimitiveEffect.primitive_id` is present in the registry — no
//!   phantom ids leak
//! * every parameter value is a finite `Q3232` (saturating arithmetic only —
//!   no raw `MAX`/`MIN` unless produced via saturating ops, and we don't
//!   expect to hit those in the fixture's bounded value range)
//! * output is sorted by `primitive_id`
//! * calling the pipeline twice with the same input produces identical output

mod common;

use beast_core::{EntityId, Q3232};
use beast_interpreter::{interpret_phenotype, phenotype::LifeStage};
use proptest::prelude::*;

use crate::common::{phenotype, standard_world};
use beast_interpreter::phenotype::Environment;

/// Sample each channel value as `Q3232::from_bits(n)` for `n` in a bounded
/// integer range. Using `from_bits` keeps the distribution deterministic
/// (no `f64` rounding in the generator) and `|n| <= 2^20 << 32` stays well
/// inside the range where saturating arithmetic on sums of ~8 terms cannot
/// overflow.
const BITS_BOUND: i64 = 1 << 20;

fn arb_life_stage() -> impl Strategy<Value = LifeStage> {
    prop_oneof![
        Just(LifeStage::Juvenile),
        Just(LifeStage::Adult),
        Just(LifeStage::Elderly),
    ]
}

fn arb_environment() -> impl Strategy<Value = Environment> {
    // Environment variation is bounded to the small set the fixture hooks
    // actually look at — biome flags + season + developmental stage.
    prop_oneof![
        Just(Environment::default()),
        Just(Environment {
            biome_flags: vec!["forest".into()],
            season: Some("spring".into()),
            ..Environment::default()
        }),
        Just(Environment {
            biome_flags: vec!["aquatic".into()],
            season: Some("summer".into()),
            ..Environment::default()
        }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        // Keep the harness headroom low — this is a determinism test, not a
        // failure-mode search.
        max_shrink_iters: 32,
        ..ProptestConfig::default()
    })]

    /// The main property: random phenotypes against the standard fixture world
    /// must produce valid, sorted, reproducible effect sets.
    #[test]
    fn interpret_phenotype_on_random_input(
        bits in prop::collection::vec(-BITS_BOUND..=BITS_BOUND, 5),
        mass_bits in 1_i64..=(100_i64 * (1 << 32)),
        stage in arb_life_stage(),
        env in arb_environment(),
    ) {
        let world = standard_world();

        // Mass: use integers (then cast through `from_num`) to stay inside
        // every channel's universal scale band. The standard_world() fixture
        // uses 0..1e9 kg bands so any positive mass is in-band.
        let mass_kg = Q3232::from_bits(mass_bits).to_num::<f64>().abs();
        // Pad to at least 1e-6 kg so we stay well away from zero mass edge
        // cases on hooks that consult ScaleBand expression conditions.
        let mass_kg = if mass_kg < 1e-6 { 1e-6 } else { mass_kg };

        let globals: Vec<(&str, f64)> = ["alpha", "beta", "gamma", "delta", "epsilon"]
            .iter()
            .zip(bits.iter())
            .map(|(id, b)| (*id, Q3232::from_bits(*b).to_num::<f64>()))
            .collect();

        let p = phenotype(mass_kg, stage, &globals, env);

        // --- Invariant 1: no panic, always `Ok`. ---
        let first = interpret_phenotype(
            &p,
            &world.hooks,
            &world.channel_registry,
            &world.primitive_registry,
            EntityId::new(1),
        );
        prop_assert!(first.is_ok(), "interpret_phenotype returned error on random input");
        let effects = first.unwrap();

        // --- Invariant 2: every primitive id is registered. ---
        for effect in &effects {
            prop_assert!(
                world.primitive_registry.contains(&effect.primitive_id),
                "primitive `{}` is not in the registry — phantom id leaked",
                effect.primitive_id,
            );
        }

        // --- Invariant 3: no parameter value saturated to Q3232::MIN / MAX.
        //     The fixture's value range keeps arithmetic well inside the
        //     representable window; a saturated value here would signal a
        //     real overflow bug.
        for effect in &effects {
            for (name, value) in &effect.parameters {
                prop_assert!(
                    *value != Q3232::MAX && *value != Q3232::MIN,
                    "parameter `{name}` on effect `{}` saturated to an extremum",
                    effect.primitive_id,
                );
            }
        }

        // --- Invariant 4: output sorted by primitive_id. ---
        let ids: Vec<&str> = effects.iter().map(|e| e.primitive_id.as_str()).collect();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        prop_assert_eq!(&ids, &sorted_ids, "output must be sorted by primitive_id");

        // --- Invariant 5: calling twice with the same input is idempotent. ---
        let second = interpret_phenotype(
            &p,
            &world.hooks,
            &world.channel_registry,
            &world.primitive_registry,
            EntityId::new(1),
        ).unwrap();
        prop_assert_eq!(&effects, &second, "repeat call diverged — determinism broken");
    }
}
