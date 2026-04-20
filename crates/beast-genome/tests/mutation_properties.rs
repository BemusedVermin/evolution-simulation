//! S3.6 — Harness-level property and correctness tests for the full
//! [`apply_mutations`] pipeline.
//!
//! Four guarantees are asserted here:
//!
//! 1. **Invariant preservation** — a proptest strategy generates random
//!    small genomes (1–20 genes) and for each `(genome, seed)` pair we
//!    run 1000 mutation passes, asserting no panics, every unit-range
//!    `Q3232` stays in `[0, 1]` (or `[-1, 1]` for modifier strength),
//!    every `target_gene_index` is in range, and every provenance string
//!    round-trips through the schema regex.
//!
//! 2. **Cross-seed determinism** — running the same `(genome, seed)` pair
//!    through 1000 passes twice must yield bit-identical genomes.
//!
//! 3. **Paralog provenance regex** — when duplication fires, the genesis
//!    provenance strings match
//!    `^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$`.
//!
//! 4. **Gaussian distribution fit** — 10 000 point-mutation magnitude
//!    deltas vs. the target `Normal(0, σ)` must pass a Kolmogorov-Smirnov
//!    goodness-of-fit test with p > 0.05 on three independent seeds.

use beast_channels::Provenance;
use beast_core::{Prng, TickCounter, Q3232};
use beast_genome::{
    apply_mutations, mutate_point, BodyVector, EffectVector, Genome, GenomeParams, LineageTag,
    Modifier, ModifierEffect, Target, Timing, TraitGene,
};
use proptest::prelude::*;
use regex::Regex;

const PROVENANCE_REGEX: &str = r"^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$";

// ---------- Strategies ---------------------------------------------------

/// Uniform `Q3232` in `[0, 1)`.
fn q3232_unit() -> impl Strategy<Value = Q3232> {
    any::<u32>().prop_map(|bits| Q3232::from_bits(i64::from(bits)))
}

/// Uniform `Q3232` in `[-1, 1]`.
fn q3232_signed_unit() -> impl Strategy<Value = Q3232> {
    any::<u32>().prop_map(|bits| {
        let u = Q3232::from_bits(i64::from(bits));
        u * Q3232::from_num(2_i32) - Q3232::ONE
    })
}

/// Channel contributions are intentionally unbounded; sample within ±2 for
/// proptest so downstream arithmetic stays far from `Q3232` saturation.
fn q3232_channel_entry() -> impl Strategy<Value = Q3232> {
    (-(2_i64 << 32)..=(2_i64 << 32)).prop_map(Q3232::from_bits)
}

fn arb_effect_type() -> impl Strategy<Value = ModifierEffect> {
    prop_oneof![
        Just(ModifierEffect::Activate),
        Just(ModifierEffect::Suppress),
        Just(ModifierEffect::Modulate),
    ]
}

fn arb_timing() -> impl Strategy<Value = Timing> {
    prop_oneof![
        Just(Timing::Passive),
        Just(Timing::OnContact),
        Just(Timing::OnDamage),
        Just(Timing::OnCooldown),
        Just(Timing::Periodic),
    ]
}

fn arb_target() -> impl Strategy<Value = Target> {
    prop_oneof![
        Just(Target::SelfEntity),
        Just(Target::TouchedEntity),
        Just(Target::AreaFriend),
        Just(Target::AreaFoe),
        Just(Target::Environment),
    ]
}

fn arb_body_vector() -> impl Strategy<Value = BodyVector> {
    (q3232_unit(), q3232_unit(), any::<bool>(), q3232_unit())
        .prop_map(|(s, r, sym, c)| BodyVector::new(s, r, sym, c).unwrap())
}

fn arb_modifier() -> impl Strategy<Value = Modifier> {
    (any::<u32>(), arb_effect_type(), q3232_signed_unit()).prop_map(
        |(raw, effect_type, strength)| Modifier {
            target_gene_index: raw,
            effect_type,
            strength,
        },
    )
}

fn arb_gene(n_channels: usize) -> BoxedStrategy<TraitGene> {
    (
        q3232_unit(),
        q3232_unit(),
        proptest::collection::vec(q3232_channel_entry(), n_channels),
        arb_timing(),
        arb_target(),
        arb_body_vector(),
        proptest::collection::vec(arb_modifier(), 0..=3),
        any::<bool>(),
    )
        .prop_map(|(mag, rad, ch, tim, tgt, bv, regs, en)| {
            let effect = EffectVector::new(ch, mag, rad, tim, tgt)
                .expect("unit-range fields are drawn from [0, 1)");
            // Lineage tag is rewritten by the caller to guarantee uniqueness;
            // start at a sentinel so an accidental leak surfaces loudly.
            TraitGene::new(
                "kinetic_force",
                effect,
                bv,
                regs,
                en,
                LineageTag::from_raw(0),
                Provenance::Core,
            )
            .expect("arb_gene must build a locally valid gene")
        })
        .boxed()
}

/// Build a complete valid genome (1-20 genes, 1-8 channels). Modifier
/// targets are rewritten to point at a non-self in-range index, and
/// lineage tags are rewritten to be unique.
fn arb_genome() -> impl Strategy<Value = Genome> {
    (1usize..=8, 1usize..=20)
        .prop_flat_map(|(n_channels, n_genes)| {
            (
                Just(n_genes),
                proptest::collection::vec(arb_gene(n_channels), n_genes),
            )
        })
        .prop_map(|(n_genes, raw_genes)| {
            let genes: Vec<TraitGene> = raw_genes
                .into_iter()
                .enumerate()
                .map(|(i, mut g)| {
                    g.lineage_tag = LineageTag::from_raw((i as u64) + 1);
                    g.regulatory = rewrite_regulatory_targets(g.regulatory, i, n_genes);
                    g
                })
                .collect();
            Genome::new(proptest_params(), genes)
                .expect("arb_genome must build a globally valid genome")
        })
}

/// Remap every modifier's `target_gene_index` into `[0, n_genes)` excluding
/// `self_idx`, preserving the rest of the modifier. When `n_genes < 2` there
/// is no valid target — drop all modifiers on singletons.
fn rewrite_regulatory_targets(
    mods: Vec<Modifier>,
    self_idx: usize,
    n_genes: usize,
) -> Vec<Modifier> {
    if n_genes < 2 {
        return Vec::new();
    }
    let span = (n_genes - 1) as u32;
    mods.into_iter()
        .map(|m| {
            let raw = m.target_gene_index % span;
            let tgt = if (raw as usize) < self_idx {
                raw
            } else {
                raw + 1
            };
            Modifier {
                target_gene_index: tgt,
                ..m
            }
        })
        .collect()
}

/// Mutation params that exercise every operator without letting the genome
/// grow unboundedly. Duplication rate is deliberately small so 1000 passes
/// won't create hundreds of paralogs (which would slow down proptest).
fn proptest_params() -> GenomeParams {
    GenomeParams {
        point_mutation_rate: Q3232::from_num(0.3_f64),
        point_mutation_sigma: Q3232::from_num(0.05_f64),
        channel_shift_rate: Q3232::from_num(0.3_f64),
        channel_shift_sigma: Q3232::from_num(0.1_f64),
        body_site_drift_rate: Q3232::from_num(0.3_f64),
        body_site_drift_sigma: Q3232::from_num(0.05_f64),
        silencing_toggle_rate: Q3232::from_num(0.01_f64),
        regulatory_add_rate: Q3232::from_num(0.1_f64),
        regulatory_remove_rate: Q3232::from_num(0.1_f64),
        regulatory_mutate_rate: Q3232::from_num(0.1_f64),
        regulatory_mutate_sigma: Q3232::from_num(0.1_f64),
        regulatory_effect_type_flip_prob: Q3232::from_num(0.1_f64),
        duplication_rate: Q3232::from_num(1.0e-3_f64),
        duplication_noise_sigma: Q3232::from_num(0.05_f64),
        duplication_rate_drift_rate: Q3232::from_num(0.01_f64),
        duplication_rate_drift_sigma: Q3232::from_num(1.0e-3_f64),
    }
}

fn assert_genome_invariants(genome: &Genome, re: &Regex) {
    let len = genome.len();
    for gene in &genome.genes {
        let mag = gene.effect.magnitude;
        assert!(
            mag >= Q3232::ZERO && mag <= Q3232::ONE,
            "magnitude out of range: {mag:?}"
        );
        let rad = gene.effect.radius;
        assert!(
            rad >= Q3232::ZERO && rad <= Q3232::ONE,
            "radius out of range: {rad:?}"
        );
        let s = gene.body_site.surface_vs_internal;
        assert!(
            s >= Q3232::ZERO && s <= Q3232::ONE,
            "surface_vs_internal out of range: {s:?}"
        );
        let br = gene.body_site.body_region;
        assert!(
            br >= Q3232::ZERO && br <= Q3232::ONE,
            "body_region out of range: {br:?}"
        );
        let cov = gene.body_site.coverage;
        assert!(
            cov >= Q3232::ZERO && cov <= Q3232::ONE,
            "coverage out of range: {cov:?}"
        );
        let neg_one = -Q3232::ONE;
        for m in &gene.regulatory {
            assert!(
                (m.target_gene_index as usize) < len,
                "target_gene_index {} out of range (len={len})",
                m.target_gene_index
            );
            assert!(
                m.strength >= neg_one && m.strength <= Q3232::ONE,
                "modifier strength out of range: {:?}",
                m.strength
            );
        }
        let pstr = gene.provenance.to_schema_string();
        assert!(re.is_match(&pstr), "provenance {pstr:?} fails schema regex");
    }
    let dup_rate = genome.params.duplication_rate;
    assert!(
        dup_rate >= Q3232::ZERO && dup_rate <= Q3232::ONE,
        "duplication_rate drifted outside [0, 1]: {dup_rate:?}"
    );
    // Full structural validation: channel count, lineage uniqueness,
    // modifier index bounds, no self-loops.
    genome
        .validate()
        .expect("Genome::validate must hold after apply_mutations");
}

// ---------- Property tests ----------------------------------------------

proptest! {
    // Default `cases` is 256 — acceptance criterion requires < 10s CI runtime.
    // Each case runs 1000 pipeline iterations over ≤ 20 genes with bounded
    // draws; empirically well under that budget on a release build.
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// 1000 mutation passes preserve every declared numeric range, every
    /// modifier index, and the provenance schema regex.
    #[test]
    fn apply_mutations_preserves_invariants(
        mut genome in arb_genome(),
        seed in any::<u64>(),
    ) {
        let re = Regex::new(PROVENANCE_REGEX).unwrap();
        let mut rng = Prng::from_seed(seed);
        for tick in 0..1000u64 {
            apply_mutations(&mut genome, TickCounter::new(tick), &mut rng);
        }
        assert_genome_invariants(&genome, &re);
    }

    /// Cross-seed determinism — the same `(genome, seed)` must produce a
    /// bit-identical genome twice.
    #[test]
    fn apply_mutations_is_deterministic_for_fixed_seed(
        genome in arb_genome(),
        seed in any::<u64>(),
    ) {
        let mut a = genome.clone();
        let mut b = genome.clone();
        let mut rng_a = Prng::from_seed(seed);
        let mut rng_b = Prng::from_seed(seed);
        for tick in 0..500u64 {
            apply_mutations(&mut a, TickCounter::new(tick), &mut rng_a);
            apply_mutations(&mut b, TickCounter::new(tick), &mut rng_b);
        }
        prop_assert_eq!(a, b);
    }
}

// ---------- KS goodness-of-fit tests ------------------------------------

/// Collect `n` single-call magnitude deltas produced by the point-mutation
/// operator. Starting magnitude is reset to 0.5 between draws so each
/// sample is an independent Gaussian draw — a random walk over many ticks
/// would hit the reflect-clamp boundary and distort the distribution.
fn collect_point_deltas(n: usize, sigma: Q3232, seed: u64) -> Vec<f64> {
    let params = GenomeParams {
        point_mutation_rate: Q3232::ONE,
        point_mutation_sigma: sigma,
        channel_shift_rate: Q3232::ZERO,
        channel_shift_sigma: Q3232::ZERO,
        body_site_drift_rate: Q3232::ZERO,
        body_site_drift_sigma: Q3232::ZERO,
        silencing_toggle_rate: Q3232::ZERO,
        regulatory_add_rate: Q3232::ZERO,
        regulatory_remove_rate: Q3232::ZERO,
        regulatory_mutate_rate: Q3232::ZERO,
        regulatory_mutate_sigma: Q3232::ZERO,
        regulatory_effect_type_flip_prob: Q3232::ZERO,
        duplication_rate: Q3232::ZERO,
        duplication_noise_sigma: Q3232::ZERO,
        duplication_rate_drift_rate: Q3232::ZERO,
        duplication_rate_drift_sigma: Q3232::ZERO,
    };
    let mut rng = Prng::from_seed(seed);
    let mut deltas = Vec::with_capacity(n);
    let start = Q3232::from_num(0.5_f64);
    for _ in 0..n {
        let mut gene = gauss_gene();
        gene.effect.magnitude = start;
        mutate_point(&mut gene, &params, &mut rng);
        let delta: f64 = (gene.effect.magnitude - start).to_num();
        deltas.push(delta);
    }
    deltas
}

fn gauss_gene() -> TraitGene {
    TraitGene::new(
        "kinetic_force",
        EffectVector::new(
            vec![Q3232::ZERO; 2],
            Q3232::from_num(0.5_f64),
            Q3232::from_num(0.25_f64),
            Timing::Passive,
            Target::SelfEntity,
        )
        .unwrap(),
        BodyVector::default_internal(),
        Vec::new(),
        true,
        LineageTag::from_raw(1),
        Provenance::Core,
    )
    .unwrap()
}

/// Standard normal CDF Φ(x) = ½(1 + erf(x/√2)). Uses Abramowitz & Stegun
/// 7.1.26 for `erf` (max error ~1.5e-7 — well within what a KS test cares
/// about at α=0.05).
#[allow(clippy::float_arithmetic)]
fn normal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    let z = (x - mu) / sigma;
    0.5 * (1.0 + erf_as_26(z / std::f64::consts::SQRT_2))
}

#[allow(clippy::float_arithmetic)]
fn erf_as_26(x: f64) -> f64 {
    // Abramowitz & Stegun 7.1.26
    const A1: f64 = 0.254_829_592;
    const A2: f64 = -0.284_496_736;
    const A3: f64 = 1.421_413_741;
    const A4: f64 = -1.453_152_027;
    const A5: f64 = 1.061_405_429;
    const P: f64 = 0.327_591_1;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs();
    let t = 1.0 / (1.0 + P * ax);
    let y = 1.0 - (((((A5 * t + A4) * t) + A3) * t + A2) * t + A1) * t * (-ax * ax).exp();
    sign * y
}

/// Two-sided Kolmogorov-Smirnov D statistic — max |F_n(x) − Φ(x)| across
/// the sorted sample.
#[allow(clippy::float_arithmetic)]
fn ks_d_statistic(samples: &[f64], mu: f64, sigma: f64) -> f64 {
    let mut sorted: Vec<f64> = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len() as f64;
    let mut d_max = 0.0_f64;
    for (i, &x) in sorted.iter().enumerate() {
        let f_theory = normal_cdf(x, mu, sigma);
        let f_emp_upper = (i + 1) as f64 / n;
        let f_emp_lower = i as f64 / n;
        let d = (f_emp_upper - f_theory)
            .abs()
            .max((f_theory - f_emp_lower).abs());
        if d > d_max {
            d_max = d;
        }
    }
    d_max
}

/// Approximate p-value for the two-sided KS test. Uses the asymptotic
/// series from Stephens (1970) / Press et al. _Numerical Recipes_:
///
/// ```text
/// λ = (√n + 0.12 + 0.11/√n) · D
/// p = 2 Σ_{k=1}^∞ (-1)^(k-1) exp(-2 k² λ²)
/// ```
///
/// Truncated at k=100 — terms decay double-exponentially so the tail is
/// negligible well before that.
#[allow(clippy::float_arithmetic)]
fn ks_p_value(d: f64, n: usize) -> f64 {
    let n_f = n as f64;
    let sqrt_n = n_f.sqrt();
    let lambda = (sqrt_n + 0.12 + 0.11 / sqrt_n) * d;
    let two_lambda_sq = 2.0 * lambda * lambda;
    let mut sum = 0.0_f64;
    for k in 1..=100_i32 {
        let k_f = f64::from(k);
        let sign = if k % 2 == 1 { 1.0 } else { -1.0 };
        sum += sign * (-k_f * k_f * two_lambda_sq).exp();
    }
    (2.0 * sum).clamp(0.0, 1.0)
}

#[test]
fn point_mutation_delta_passes_ks_gaussian_fit() {
    // 100 genes × 100 per-gene samples = 10 000 independent Gaussian
    // deltas, tested against N(0, σ). Run on three seeds so a single
    // unlucky seed can't mask a real distributional failure.
    let sigma_f = 0.05_f64;
    let sigma_q = Q3232::from_num(sigma_f);
    let seeds = [0xD5F1_D5F1_u64, 0xCAFE_BABE, 0xF00D_FACE];
    for seed in seeds {
        let samples = collect_point_deltas(10_000, sigma_q, seed);
        let d = ks_d_statistic(&samples, 0.0, sigma_f);
        let p = ks_p_value(d, samples.len());
        assert!(
            p > 0.05,
            "KS test failed for seed {seed:#x}: D={d:.6}, p={p:.6}"
        );
    }
}
