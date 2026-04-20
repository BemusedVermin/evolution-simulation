//! Deterministic point-mutation operators for individual trait genes.
//!
//! Each operator draws from the caller-supplied [`beast_core::Prng`], which
//! must be derived from [`beast_core::Stream::Genetics`]. Draw order within
//! [`mutate_point`] is fixed so that identical `(gene, params, seed)` triples
//! produce bit-identical results across platforms.

use beast_core::{gaussian_q3232, reflect_clamp01, Prng, Q3232};

use crate::gene::TraitGene;
use crate::genome::GenomeParams;
use crate::modifier::{Modifier, ModifierEffect};

/// Apply all point-level mutations to a single gene.
///
/// Mutations fire independently (one Bernoulli trial per type) in a fixed
/// order that must not change across versions:
///
/// 1. **Magnitude** — N(0, σ) drift, reflect-clamped to `[0, 1]`
/// 2. **Channel shift** — N(0, σ) per channel entry (unbounded)
/// 3. **Body-site drift** — N(0, σ) on `surface_vs_internal` and
///    `body_region`, reflect-clamped to `[0, 1]`
/// 4. **Silencing toggle** — flip `enabled`
///
/// Channel contributions are intentionally unbounded (inhibitory / synergistic
/// values are legal); clamping happens downstream in the interpreter.
pub fn mutate_point(gene: &mut TraitGene, params: &GenomeParams, rng: &mut Prng) {
    mutate_magnitude(gene, params, rng);
    mutate_channels(gene, params, rng);
    mutate_body_site(gene, params, rng);
    mutate_silencing(gene, params, rng);
}

fn mutate_magnitude(gene: &mut TraitGene, params: &GenomeParams, rng: &mut Prng) {
    if rng.next_q3232_unit() < params.point_mutation_rate {
        let delta = gaussian_q3232(rng, Q3232::ZERO, params.point_mutation_sigma);
        gene.effect.magnitude = reflect_clamp01(gene.effect.magnitude + delta);
    }
}

fn mutate_channels(gene: &mut TraitGene, params: &GenomeParams, rng: &mut Prng) {
    if rng.next_q3232_unit() < params.channel_shift_rate {
        for ch in &mut gene.effect.channel {
            let delta = gaussian_q3232(rng, Q3232::ZERO, params.channel_shift_sigma);
            *ch += delta;
        }
    }
}

fn mutate_body_site(gene: &mut TraitGene, params: &GenomeParams, rng: &mut Prng) {
    if rng.next_q3232_unit() < params.body_site_drift_rate {
        let delta_s = gaussian_q3232(rng, Q3232::ZERO, params.body_site_drift_sigma);
        gene.body_site.surface_vs_internal =
            reflect_clamp01(gene.body_site.surface_vs_internal + delta_s);

        let delta_r = gaussian_q3232(rng, Q3232::ZERO, params.body_site_drift_sigma);
        gene.body_site.body_region = reflect_clamp01(gene.body_site.body_region + delta_r);
    }
}

fn mutate_silencing(gene: &mut TraitGene, params: &GenomeParams, rng: &mut Prng) {
    if rng.next_q3232_unit() < params.silencing_toggle_rate {
        gene.enabled = !gene.enabled;
    }
}

/// Apply all regulatory-rewiring operators to a single gene's
/// [`crate::Modifier`] list.
///
/// Operators fire in a fixed order so that identical
/// `(gene, source_gene_index, genome_len, params, seed)` inputs produce
/// bit-identical results:
///
/// 1. **Add** — push a new modifier targeting a random gene
///    `!= source_gene_index`
/// 2. **Remove** — drop a random existing modifier
/// 3. **Mutate existing** — pick a random modifier and drift its
///    `strength` (Gaussian, reflect-clamped to `[-1, 1]`); with
///    `regulatory_effect_type_flip_prob` also swap its `effect_type`
///    for one of the other two variants
///
/// The `Modifier` list is iterated by index, never by hashed order, so
/// the result is stable across platforms.
///
/// `source_gene_index` must match the gene's position in its owning
/// [`crate::Genome`]; `genome_len` must equal `genome.len()`. These are
/// passed in rather than looked up to keep this function callable on
/// unowned `TraitGene` values in tests.
pub fn mutate_regulatory(
    gene: &mut TraitGene,
    source_gene_index: u32,
    genome_len: usize,
    params: &GenomeParams,
    rng: &mut Prng,
) {
    try_add_modifier(gene, source_gene_index, genome_len, params, rng);
    try_remove_modifier(gene, params, rng);
    try_mutate_modifier(gene, params, rng);
}

fn try_add_modifier(
    gene: &mut TraitGene,
    source: u32,
    genome_len: usize,
    params: &GenomeParams,
    rng: &mut Prng,
) {
    if rng.next_q3232_unit() >= params.regulatory_add_rate {
        return;
    }
    // A valid target must be distinct from `source`, so we need at least
    // two genes in the owning genome.
    if genome_len < 2 {
        return;
    }
    // Uniform over [0, genome_len - 1); shift past `source` to skip it.
    // Safe to cast: `genome_len <= u32::MAX` is guaranteed at Genome
    // construction (see `Genome::validate`).
    let span = (genome_len - 1) as u64;
    let raw = rng.gen_range_u64(0, span) as u32;
    let target = if raw < source { raw } else { raw + 1 };
    let effect_type = draw_effect_type(rng);
    let strength = draw_strength(rng);
    gene.regulatory.push(Modifier {
        target_gene_index: target,
        effect_type,
        strength,
    });
}

fn try_remove_modifier(gene: &mut TraitGene, params: &GenomeParams, rng: &mut Prng) {
    if rng.next_q3232_unit() >= params.regulatory_remove_rate {
        return;
    }
    let n = gene.regulatory.len();
    if n == 0 {
        return;
    }
    let idx = rng.gen_range_u64(0, n as u64) as usize;
    gene.regulatory.remove(idx);
}

fn try_mutate_modifier(gene: &mut TraitGene, params: &GenomeParams, rng: &mut Prng) {
    if rng.next_q3232_unit() >= params.regulatory_mutate_rate {
        return;
    }
    let n = gene.regulatory.len();
    if n == 0 {
        return;
    }
    let idx = rng.gen_range_u64(0, n as u64) as usize;
    let delta = gaussian_q3232(rng, Q3232::ZERO, params.regulatory_mutate_sigma);
    let new_strength = reflect_clamp_signed_unit(gene.regulatory[idx].strength + delta);
    gene.regulatory[idx].strength = new_strength;
    if rng.next_q3232_unit() < params.regulatory_effect_type_flip_prob {
        let current = gene.regulatory[idx].effect_type;
        let pick = rng.gen_range_u64(0, 2) as u8;
        gene.regulatory[idx].effect_type = flipped_effect_type(current, pick);
    }
}

fn draw_effect_type(rng: &mut Prng) -> ModifierEffect {
    match rng.gen_range_u64(0, 3) {
        0 => ModifierEffect::Activate,
        1 => ModifierEffect::Suppress,
        _ => ModifierEffect::Modulate,
    }
}

/// Draw a strength in `[-1, 1]` by remapping a unit-range draw.
fn draw_strength(rng: &mut Prng) -> Q3232 {
    let u = rng.next_q3232_unit();
    u * Q3232::from_num(2_i32) - Q3232::ONE
}

/// Reflect a [`Q3232`] value back into `[-1, 1]` off the nearest
/// boundary, then hard-clamp any residual overshoot. Analogous to
/// [`beast_core::reflect_clamp01`] but over the signed-unit interval.
fn reflect_clamp_signed_unit(v: Q3232) -> Q3232 {
    let neg_one = -Q3232::ONE;
    if v >= neg_one && v <= Q3232::ONE {
        return v;
    }
    let two = Q3232::from_num(2_i32);
    let reflected = if v < neg_one { -two - v } else { two - v };
    reflected.clamp(neg_one, Q3232::ONE)
}

/// Return one of the other two [`ModifierEffect`] variants, chosen by
/// `pick ∈ {0, 1}`.
fn flipped_effect_type(current: ModifierEffect, pick: u8) -> ModifierEffect {
    match (current, pick) {
        (ModifierEffect::Activate, 0) => ModifierEffect::Suppress,
        (ModifierEffect::Activate, _) => ModifierEffect::Modulate,
        (ModifierEffect::Suppress, 0) => ModifierEffect::Activate,
        (ModifierEffect::Suppress, _) => ModifierEffect::Modulate,
        (ModifierEffect::Modulate, 0) => ModifierEffect::Activate,
        (ModifierEffect::Modulate, _) => ModifierEffect::Suppress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_site::BodyVector;
    use crate::gene::{EffectVector, Target, Timing};
    use crate::lineage::LineageTag;
    use beast_channels::Provenance;

    fn make_gene(channels: usize) -> TraitGene {
        TraitGene::new(
            "kinetic_force",
            EffectVector::new(
                vec![Q3232::from_num(0.5_f64); channels],
                Q3232::from_num(0.5_f64),
                Q3232::from_num(0.25_f64),
                Timing::Passive,
                Target::SelfEntity,
            )
            .unwrap(),
            BodyVector::new(
                Q3232::from_num(0.5_f64),
                Q3232::from_num(0.5_f64),
                false,
                Q3232::from_num(0.5_f64),
            )
            .unwrap(),
            vec![],
            true,
            LineageTag::from_raw(1),
            Provenance::Core,
        )
        .unwrap()
    }

    fn high_rate_params() -> GenomeParams {
        GenomeParams {
            point_mutation_rate: Q3232::ONE,
            point_mutation_sigma: Q3232::from_num(0.1_f64),
            channel_shift_rate: Q3232::ONE,
            channel_shift_sigma: Q3232::from_num(0.15_f64),
            body_site_drift_rate: Q3232::ONE,
            body_site_drift_sigma: Q3232::from_num(0.1_f64),
            silencing_toggle_rate: Q3232::ONE,
            regulatory_add_rate: Q3232::ZERO,
            regulatory_remove_rate: Q3232::ZERO,
            regulatory_mutate_rate: Q3232::ZERO,
            regulatory_mutate_sigma: Q3232::from_num(0.15_f64),
            regulatory_effect_type_flip_prob: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
            duplication_noise_sigma: Q3232::from_num(0.05_f64),
            duplication_rate_drift_rate: Q3232::ZERO,
            duplication_rate_drift_sigma: Q3232::from_num(1.0e-4_f64),
        }
    }

    #[test]
    fn determinism_same_seed() {
        let params = high_rate_params();
        for seed in [1u64, 42, 999, 0xDEAD] {
            let mut gene_a = make_gene(4);
            let mut gene_b = make_gene(4);
            let mut rng_a = Prng::from_seed(seed);
            let mut rng_b = Prng::from_seed(seed);
            for _ in 0..1000 {
                mutate_point(&mut gene_a, &params, &mut rng_a);
                mutate_point(&mut gene_b, &params, &mut rng_b);
            }
            assert_eq!(gene_a, gene_b, "diverged at seed {seed}");
        }
    }

    #[test]
    fn zero_rate_no_mutation() {
        let params = GenomeParams {
            point_mutation_rate: Q3232::ZERO,
            point_mutation_sigma: Q3232::from_num(0.1_f64),
            channel_shift_rate: Q3232::ZERO,
            channel_shift_sigma: Q3232::from_num(0.15_f64),
            body_site_drift_rate: Q3232::ZERO,
            body_site_drift_sigma: Q3232::from_num(0.1_f64),
            silencing_toggle_rate: Q3232::ZERO,
            regulatory_add_rate: Q3232::ZERO,
            regulatory_remove_rate: Q3232::ZERO,
            regulatory_mutate_rate: Q3232::ZERO,
            regulatory_mutate_sigma: Q3232::from_num(0.15_f64),
            regulatory_effect_type_flip_prob: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
            duplication_noise_sigma: Q3232::from_num(0.05_f64),
            duplication_rate_drift_rate: Q3232::ZERO,
            duplication_rate_drift_sigma: Q3232::from_num(1.0e-4_f64),
        };
        let original = make_gene(4);
        let mut gene = original.clone();
        let mut rng = Prng::from_seed(42);
        for _ in 0..1000 {
            mutate_point(&mut gene, &params, &mut rng);
        }
        assert_eq!(gene, original);
    }

    #[test]
    fn unit_range_preserved_after_many_mutations() {
        let params = high_rate_params();
        let mut gene = make_gene(4);
        let mut rng = Prng::from_seed(7);
        for _ in 0..10_000 {
            mutate_point(&mut gene, &params, &mut rng);
            assert!(
                gene.effect.magnitude >= Q3232::ZERO && gene.effect.magnitude <= Q3232::ONE,
                "magnitude out of range: {:?}",
                gene.effect.magnitude
            );
            assert!(
                gene.body_site.surface_vs_internal >= Q3232::ZERO
                    && gene.body_site.surface_vs_internal <= Q3232::ONE,
                "surface_vs_internal out of range: {:?}",
                gene.body_site.surface_vs_internal
            );
            assert!(
                gene.body_site.body_region >= Q3232::ZERO
                    && gene.body_site.body_region <= Q3232::ONE,
                "body_region out of range: {:?}",
                gene.body_site.body_region
            );
        }
    }

    #[test]
    fn channels_are_unbounded() {
        let params = GenomeParams {
            point_mutation_rate: Q3232::ZERO,
            point_mutation_sigma: Q3232::from_num(0.1_f64),
            channel_shift_rate: Q3232::ONE,
            channel_shift_sigma: Q3232::from_num(1_i32),
            body_site_drift_rate: Q3232::ZERO,
            body_site_drift_sigma: Q3232::from_num(0.1_f64),
            silencing_toggle_rate: Q3232::ZERO,
            regulatory_add_rate: Q3232::ZERO,
            regulatory_remove_rate: Q3232::ZERO,
            regulatory_mutate_rate: Q3232::ZERO,
            regulatory_mutate_sigma: Q3232::from_num(0.15_f64),
            regulatory_effect_type_flip_prob: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
            duplication_noise_sigma: Q3232::from_num(0.05_f64),
            duplication_rate_drift_rate: Q3232::ZERO,
            duplication_rate_drift_sigma: Q3232::from_num(1.0e-4_f64),
        };
        let mut gene = make_gene(4);
        let mut rng = Prng::from_seed(99);
        for _ in 0..500 {
            mutate_point(&mut gene, &params, &mut rng);
        }
        let any_outside_unit = gene
            .effect
            .channel
            .iter()
            .any(|&c| c < Q3232::ZERO || c > Q3232::ONE);
        assert!(
            any_outside_unit,
            "expected at least one channel outside [0,1] after 500 large-sigma shifts"
        );
    }

    #[test]
    fn silencing_always_toggles_at_rate_one() {
        let params = GenomeParams {
            point_mutation_rate: Q3232::ZERO,
            point_mutation_sigma: Q3232::from_num(0.1_f64),
            channel_shift_rate: Q3232::ZERO,
            channel_shift_sigma: Q3232::from_num(0.15_f64),
            body_site_drift_rate: Q3232::ZERO,
            body_site_drift_sigma: Q3232::from_num(0.1_f64),
            silencing_toggle_rate: Q3232::ONE,
            regulatory_add_rate: Q3232::ZERO,
            regulatory_remove_rate: Q3232::ZERO,
            regulatory_mutate_rate: Q3232::ZERO,
            regulatory_mutate_sigma: Q3232::from_num(0.15_f64),
            regulatory_effect_type_flip_prob: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
            duplication_noise_sigma: Q3232::from_num(0.05_f64),
            duplication_rate_drift_rate: Q3232::ZERO,
            duplication_rate_drift_sigma: Q3232::from_num(1.0e-4_f64),
        };
        let mut gene = make_gene(2);
        assert!(gene.enabled);
        let mut rng = Prng::from_seed(1);
        mutate_point(&mut gene, &params, &mut rng);
        assert!(!gene.enabled);
        mutate_point(&mut gene, &params, &mut rng);
        assert!(gene.enabled);
    }

    #[test]
    fn magnitude_drift_empirical_mean() {
        let params = GenomeParams {
            point_mutation_rate: Q3232::ONE,
            point_mutation_sigma: Q3232::from_num(0.1_f64),
            channel_shift_rate: Q3232::ZERO,
            channel_shift_sigma: Q3232::from_num(0.15_f64),
            body_site_drift_rate: Q3232::ZERO,
            body_site_drift_sigma: Q3232::from_num(0.1_f64),
            silencing_toggle_rate: Q3232::ZERO,
            regulatory_add_rate: Q3232::ZERO,
            regulatory_remove_rate: Q3232::ZERO,
            regulatory_mutate_rate: Q3232::ZERO,
            regulatory_mutate_sigma: Q3232::from_num(0.15_f64),
            regulatory_effect_type_flip_prob: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
            duplication_noise_sigma: Q3232::from_num(0.05_f64),
            duplication_rate_drift_rate: Q3232::ZERO,
            duplication_rate_drift_sigma: Q3232::from_num(1.0e-4_f64),
        };
        let n = 10_000_i32;
        let start = Q3232::from_num(0.5_f64);
        let mut sum = Q3232::ZERO;
        let mut rng = Prng::from_seed(42);
        for _ in 0..n {
            let mut gene = make_gene(2);
            mutate_point(&mut gene, &params, &mut rng);
            sum += gene.effect.magnitude - start;
        }
        let mean_drift: f64 = (sum / Q3232::from_num(n)).to_num();
        assert!(
            mean_drift.abs() < 0.01,
            "mean drift {mean_drift} too far from 0"
        );
    }

    #[test]
    fn coverage_and_radius_unchanged() {
        let params = high_rate_params();
        let original = make_gene(4);
        let mut gene = original.clone();
        let mut rng = Prng::from_seed(55);
        for _ in 0..1000 {
            mutate_point(&mut gene, &params, &mut rng);
        }
        assert_eq!(gene.effect.radius, original.effect.radius);
        assert_eq!(gene.body_site.coverage, original.body_site.coverage);
    }

    #[test]
    fn provenance_and_lineage_unchanged() {
        let params = high_rate_params();
        let original = make_gene(4);
        let mut gene = original.clone();
        let mut rng = Prng::from_seed(77);
        for _ in 0..1000 {
            mutate_point(&mut gene, &params, &mut rng);
        }
        assert_eq!(gene.lineage_tag, original.lineage_tag);
        assert_eq!(gene.provenance, original.provenance);
        assert_eq!(gene.channel_id, original.channel_id);
    }

    // --- S3.4 regulatory rewiring ------------------------------------------

    fn rewiring_params() -> GenomeParams {
        GenomeParams {
            point_mutation_rate: Q3232::ZERO,
            point_mutation_sigma: Q3232::from_num(0.1_f64),
            channel_shift_rate: Q3232::ZERO,
            channel_shift_sigma: Q3232::from_num(0.15_f64),
            body_site_drift_rate: Q3232::ZERO,
            body_site_drift_sigma: Q3232::from_num(0.1_f64),
            silencing_toggle_rate: Q3232::ZERO,
            regulatory_add_rate: Q3232::ONE,
            regulatory_remove_rate: Q3232::ONE,
            regulatory_mutate_rate: Q3232::ONE,
            regulatory_mutate_sigma: Q3232::from_num(0.25_f64),
            regulatory_effect_type_flip_prob: Q3232::from_num(0.1_f64),
            duplication_rate: Q3232::ZERO,
            duplication_noise_sigma: Q3232::from_num(0.05_f64),
            duplication_rate_drift_rate: Q3232::ZERO,
            duplication_rate_drift_sigma: Q3232::from_num(1.0e-4_f64),
        }
    }

    #[test]
    fn rewiring_zero_rates_no_change() {
        let mut params = rewiring_params();
        params.regulatory_add_rate = Q3232::ZERO;
        params.regulatory_remove_rate = Q3232::ZERO;
        params.regulatory_mutate_rate = Q3232::ZERO;
        let original = make_gene(2);
        let mut gene = original.clone();
        let mut rng = Prng::from_seed(42);
        for _ in 0..1000 {
            mutate_regulatory(&mut gene, 0, 4, &params, &mut rng);
        }
        assert_eq!(gene, original);
    }

    #[test]
    fn rewiring_determinism_same_seed() {
        let params = rewiring_params();
        for seed in [1u64, 42, 999, 0xDEAD] {
            let mut gene_a = make_gene(2);
            let mut gene_b = make_gene(2);
            let mut rng_a = Prng::from_seed(seed);
            let mut rng_b = Prng::from_seed(seed);
            for _ in 0..1000 {
                mutate_regulatory(&mut gene_a, 0, 4, &params, &mut rng_a);
                mutate_regulatory(&mut gene_b, 0, 4, &params, &mut rng_b);
            }
            assert_eq!(
                gene_a.regulatory, gene_b.regulatory,
                "regulatory diverged at seed {seed}"
            );
        }
    }

    #[test]
    fn rewiring_never_produces_self_loop() {
        let params = rewiring_params();
        let source: u32 = 2;
        let genome_len = 5;
        let mut gene = make_gene(2);
        let mut rng = Prng::from_seed(7);
        for _ in 0..10_000 {
            mutate_regulatory(&mut gene, source, genome_len, &params, &mut rng);
            for m in &gene.regulatory {
                assert_ne!(
                    m.target_gene_index, source,
                    "modifier targeted its own source gene index"
                );
            }
        }
    }

    #[test]
    fn rewiring_target_gene_index_in_range() {
        let params = rewiring_params();
        let genome_len = 6;
        let mut gene = make_gene(2);
        let mut rng = Prng::from_seed(11);
        for _ in 0..10_000 {
            mutate_regulatory(&mut gene, 3, genome_len, &params, &mut rng);
            for m in &gene.regulatory {
                assert!(
                    (m.target_gene_index as usize) < genome_len,
                    "target_gene_index {} out of range (genome_len={})",
                    m.target_gene_index,
                    genome_len
                );
            }
        }
    }

    #[test]
    fn rewiring_strength_stays_in_signed_unit() {
        let params = rewiring_params();
        let mut gene = make_gene(2);
        let mut rng = Prng::from_seed(17);
        let neg_one = -Q3232::ONE;
        for _ in 0..10_000 {
            mutate_regulatory(&mut gene, 0, 4, &params, &mut rng);
            for m in &gene.regulatory {
                assert!(
                    m.strength >= neg_one && m.strength <= Q3232::ONE,
                    "modifier strength out of range: {:?}",
                    m.strength
                );
            }
        }
    }

    #[test]
    fn rewiring_add_needs_genome_len_ge_two() {
        let mut params = rewiring_params();
        params.regulatory_remove_rate = Q3232::ZERO;
        params.regulatory_mutate_rate = Q3232::ZERO;
        let mut gene = make_gene(2);
        let mut rng = Prng::from_seed(99);
        for _ in 0..500 {
            mutate_regulatory(&mut gene, 0, 1, &params, &mut rng);
        }
        assert!(
            gene.regulatory.is_empty(),
            "singleton genome must never grow a modifier (need a distinct target)"
        );
    }

    #[test]
    fn rewiring_remove_on_empty_is_noop() {
        let mut params = rewiring_params();
        params.regulatory_add_rate = Q3232::ZERO;
        params.regulatory_mutate_rate = Q3232::ZERO;
        let original = make_gene(2);
        let mut gene = original.clone();
        let mut rng = Prng::from_seed(5);
        for _ in 0..500 {
            mutate_regulatory(&mut gene, 0, 4, &params, &mut rng);
        }
        assert_eq!(gene, original);
    }

    #[test]
    fn rewiring_mutate_on_empty_is_noop() {
        let mut params = rewiring_params();
        params.regulatory_add_rate = Q3232::ZERO;
        params.regulatory_remove_rate = Q3232::ZERO;
        let original = make_gene(2);
        let mut gene = original.clone();
        let mut rng = Prng::from_seed(3);
        for _ in 0..500 {
            mutate_regulatory(&mut gene, 0, 4, &params, &mut rng);
        }
        assert_eq!(gene, original);
    }

    #[test]
    fn rewiring_add_populates_modifiers_when_valid() {
        let mut params = rewiring_params();
        params.regulatory_remove_rate = Q3232::ZERO;
        params.regulatory_mutate_rate = Q3232::ZERO;
        let mut gene = make_gene(2);
        let mut rng = Prng::from_seed(33);
        for _ in 0..10 {
            mutate_regulatory(&mut gene, 1, 4, &params, &mut rng);
        }
        assert_eq!(
            gene.regulatory.len(),
            10,
            "add-only pipeline should push exactly one modifier per call"
        );
    }

    #[test]
    fn reflect_clamp_signed_unit_in_range_passthrough() {
        assert_eq!(reflect_clamp_signed_unit(Q3232::ZERO), Q3232::ZERO);
        assert_eq!(reflect_clamp_signed_unit(Q3232::ONE), Q3232::ONE);
        assert_eq!(reflect_clamp_signed_unit(-Q3232::ONE), -Q3232::ONE);
        assert_eq!(
            reflect_clamp_signed_unit(Q3232::from_num(0.5_f64)),
            Q3232::from_num(0.5_f64)
        );
        assert_eq!(
            reflect_clamp_signed_unit(Q3232::from_num(-0.5_f64)),
            Q3232::from_num(-0.5_f64)
        );
    }

    #[test]
    fn reflect_clamp_signed_unit_reflects_above_one() {
        assert_eq!(
            reflect_clamp_signed_unit(Q3232::from_num(1.3_f64)),
            Q3232::from_num(0.7_f64)
        );
        assert_eq!(
            reflect_clamp_signed_unit(Q3232::from_num(1.9_f64)),
            Q3232::from_num(0.1_f64)
        );
    }

    #[test]
    fn reflect_clamp_signed_unit_reflects_below_neg_one() {
        assert_eq!(
            reflect_clamp_signed_unit(Q3232::from_num(-1.3_f64)),
            Q3232::from_num(-0.7_f64)
        );
        assert_eq!(
            reflect_clamp_signed_unit(Q3232::from_num(-1.9_f64)),
            Q3232::from_num(-0.1_f64)
        );
    }

    #[test]
    fn reflect_clamp_signed_unit_clamps_pathological() {
        // Beyond one reflection off each boundary — final clamp saves us.
        assert_eq!(
            reflect_clamp_signed_unit(Q3232::from_num(3.5_f64)),
            -Q3232::ONE
        );
        assert_eq!(
            reflect_clamp_signed_unit(Q3232::from_num(-3.5_f64)),
            Q3232::ONE
        );
    }

    #[test]
    fn flipped_effect_type_always_different() {
        for current in [
            ModifierEffect::Activate,
            ModifierEffect::Suppress,
            ModifierEffect::Modulate,
        ] {
            for pick in [0_u8, 1_u8] {
                assert_ne!(flipped_effect_type(current, pick), current);
            }
        }
    }
}
