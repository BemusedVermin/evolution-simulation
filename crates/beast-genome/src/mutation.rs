//! Deterministic point-mutation operators for individual trait genes.
//!
//! Each operator draws from the caller-supplied [`beast_core::Prng`], which
//! must be derived from [`beast_core::Stream::Genetics`]. Draw order within
//! [`mutate_point`] is fixed so that identical `(gene, params, seed)` triples
//! produce bit-identical results across platforms.

use beast_core::{gaussian_q3232, reflect_clamp01, Prng, Q3232};

use crate::gene::TraitGene;
use crate::genome::GenomeParams;

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
            regulatory_rewire_rate: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
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
            regulatory_rewire_rate: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
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
            regulatory_rewire_rate: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
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
            regulatory_rewire_rate: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
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
            regulatory_rewire_rate: Q3232::ZERO,
            duplication_rate: Q3232::ZERO,
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
}
