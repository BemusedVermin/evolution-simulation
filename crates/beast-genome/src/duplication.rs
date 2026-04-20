//! Deterministic gene-duplication operator and `duplication_rate` drift.
//!
//! Implements System 01 §2B paralog creation: a random source gene is
//! cloned, marked with a fresh [`crate::LineageTag`], tagged with
//! `Provenance::Genesis { parent: source.channel_id, generation: tick }`,
//! and perturbed by a small Gaussian noise on its channel vector so the
//! paralog is immediately distinguishable from its parent. Divergence
//! beyond this initial noise, reclassification, and loss/pruning are
//! **out of scope** — they belong to future sprints (see cut-scope
//! issues).
//!
//! The per-genome [`crate::GenomeParams::duplication_rate`] is itself
//! mutable (§6B): [`mutate_duplication_rate`] applies a Gaussian step
//! at [`crate::GenomeParams::duplication_rate_drift_rate`].
//!
//! Both operators draw from the caller-supplied [`Prng`], which must be
//! derived from [`beast_core::Stream::Genetics`]. Draw order is fixed
//! so `(genome, tick, seed)` inputs produce bit-identical results.

use beast_channels::Provenance;
use beast_core::{gaussian_q3232, reflect_clamp01, Prng, TickCounter, Q3232};

use crate::genome::Genome;
use crate::lineage::LineageTag;

/// Attempt to duplicate a random gene on `genome` into a fresh paralog.
///
/// Fires with probability [`crate::GenomeParams::duplication_rate`].
/// When it fires, a random existing gene is cloned; the clone receives:
///
/// - a fresh [`LineageTag`] drawn from `rng`,
/// - `provenance = Provenance::Genesis { parent: source.channel_id.clone(),
///   generation: current_tick.raw() }`,
/// - per-channel-entry Gaussian noise with σ =
///   [`crate::GenomeParams::duplication_noise_sigma`].
///
/// All other fields (`effect.magnitude`, `effect.radius`, `timing`,
/// `target`, `body_site`, `regulatory`, `enabled`) are copied verbatim.
///
/// The paralog is appended at the end of `genome.genes`, preserving the
/// existing index order. Caller is responsible for calling
/// [`Genome::validate`] if they need structural invariants re-checked.
///
/// No-op when `genome.is_empty()`.
pub fn mutate_duplicate(genome: &mut Genome, current_tick: TickCounter, rng: &mut Prng) {
    if rng.next_q3232_unit() >= genome.params.duplication_rate {
        return;
    }
    let n = genome.genes.len();
    if n == 0 {
        return;
    }
    let source_idx = rng.gen_range_u64(0, n as u64) as usize;
    let mut paralog = genome.genes[source_idx].clone();
    let parent_channel_id = paralog.channel_id.clone();
    paralog.lineage_tag = LineageTag::fresh(rng);
    paralog.provenance = Provenance::Genesis {
        parent: parent_channel_id,
        generation: current_tick.raw(),
    };
    let sigma = genome.params.duplication_noise_sigma;
    for ch in &mut paralog.effect.channel {
        *ch += gaussian_q3232(rng, Q3232::ZERO, sigma);
    }
    genome.genes.push(paralog);
}

/// Drift `genome.params.duplication_rate` by a Gaussian step.
///
/// Fires with probability
/// [`crate::GenomeParams::duplication_rate_drift_rate`] (System 01 §3
/// lists the aggregate meta-rate as `1e-4`). The drift is reflect-clamped
/// to `[0, 1]` so the rate stays a valid probability.
pub fn mutate_duplication_rate(genome: &mut Genome, rng: &mut Prng) {
    if rng.next_q3232_unit() >= genome.params.duplication_rate_drift_rate {
        return;
    }
    let delta = gaussian_q3232(rng, Q3232::ZERO, genome.params.duplication_rate_drift_sigma);
    genome.params.duplication_rate = reflect_clamp01(genome.params.duplication_rate + delta);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_site::BodyVector;
    use crate::gene::{EffectVector, Target, Timing, TraitGene};
    use crate::genome::GenomeParams;
    use regex::Regex;

    const PROVENANCE_REGEX: &str = r"^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$";

    fn gene(tag: u64, channels: usize) -> TraitGene {
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
            BodyVector::default_internal(),
            vec![],
            true,
            LineageTag::from_raw(tag),
            Provenance::Core,
        )
        .unwrap()
    }

    fn params_with_duplication_rate(rate: Q3232) -> GenomeParams {
        GenomeParams {
            duplication_rate: rate,
            // Disable every other operator; the duplication operator is
            // the only one that consumes from the PRNG in these tests.
            point_mutation_rate: Q3232::ZERO,
            channel_shift_rate: Q3232::ZERO,
            body_site_drift_rate: Q3232::ZERO,
            silencing_toggle_rate: Q3232::ZERO,
            regulatory_add_rate: Q3232::ZERO,
            regulatory_remove_rate: Q3232::ZERO,
            regulatory_mutate_rate: Q3232::ZERO,
            regulatory_effect_type_flip_prob: Q3232::ZERO,
            duplication_rate_drift_rate: Q3232::ZERO,
            ..GenomeParams::default()
        }
    }

    fn genome_with(params: GenomeParams, gene_count: usize) -> Genome {
        let genes: Vec<TraitGene> = (0..gene_count).map(|i| gene((i as u64) + 1, 3)).collect();
        Genome::new(params, genes).unwrap()
    }

    #[test]
    fn default_duplication_params_produce_no_duplicates() {
        let params = GenomeParams::default();
        assert_eq!(params.duplication_rate, Q3232::ZERO);
        let mut genome = genome_with(params, 3);
        let mut rng = Prng::from_seed(42);
        for tick in 0..10_000u64 {
            mutate_duplicate(&mut genome, TickCounter::new(tick), &mut rng);
        }
        assert_eq!(
            genome.genes.len(),
            3,
            "default rate (0.0) must never trigger a duplication"
        );
    }

    #[test]
    fn rate_tenth_produces_expected_paralog_count() {
        let params = params_with_duplication_rate(Q3232::from_num(0.1_f64));
        let mut genome = genome_with(params, 3);
        let mut rng = Prng::from_seed(7);
        for tick in 0..10_000u64 {
            mutate_duplicate(&mut genome, TickCounter::new(tick), &mut rng);
        }
        let added = genome.genes.len() - 3;
        assert!(
            (800..=1200).contains(&added),
            "expected ~1000 paralogs from 10000 Bernoulli(0.1) trials, got {added}"
        );
    }

    #[test]
    fn paralog_provenance_matches_schema_regex() {
        let params = params_with_duplication_rate(Q3232::from_num(0.5_f64));
        let mut genome = genome_with(params, 3);
        let mut rng = Prng::from_seed(11);
        let re = Regex::new(PROVENANCE_REGEX).unwrap();
        for tick in 0..200u64 {
            mutate_duplicate(&mut genome, TickCounter::new(tick), &mut rng);
        }
        let paralogs: Vec<_> = genome
            .genes
            .iter()
            .filter(|g| matches!(g.provenance, Provenance::Genesis { .. }))
            .collect();
        assert!(!paralogs.is_empty(), "expected at least one paralog");
        for p in paralogs {
            let s = p.provenance.to_schema_string();
            assert!(re.is_match(&s), "provenance {s:?} fails schema regex");
        }
    }

    #[test]
    fn paralog_lineage_tag_unique_in_genome() {
        let params = params_with_duplication_rate(Q3232::from_num(0.3_f64));
        let mut genome = genome_with(params, 3);
        let mut rng = Prng::from_seed(13);
        for tick in 0..500u64 {
            mutate_duplicate(&mut genome, TickCounter::new(tick), &mut rng);
        }
        // `Genome::validate` refuses duplicate lineage tags.
        genome
            .validate()
            .expect("duplication operator must preserve lineage-tag uniqueness");
    }

    #[test]
    fn paralog_channel_vector_differs_from_parent() {
        // With a non-zero noise sigma, every paralog should differ from
        // its parent in at least one channel entry.
        let mut params = params_with_duplication_rate(Q3232::ONE);
        params.duplication_noise_sigma = Q3232::from_num(0.1_f64);
        let mut genome = genome_with(params, 1);
        let mut rng = Prng::from_seed(3);
        let parent_snapshot = genome.genes[0].effect.channel.clone();
        mutate_duplicate(&mut genome, TickCounter::new(0), &mut rng);
        assert_eq!(genome.genes.len(), 2);
        let paralog = &genome.genes[1];
        assert_ne!(
            paralog.effect.channel, parent_snapshot,
            "paralog channel vector must be noised away from parent"
        );
    }

    #[test]
    fn paralog_preserves_non_channel_fields() {
        let mut params = params_with_duplication_rate(Q3232::ONE);
        // Zero noise so the parent is copied bit-identical except for the
        // fields that must change.
        params.duplication_noise_sigma = Q3232::ZERO;
        let mut genome = genome_with(params, 1);
        let parent = genome.genes[0].clone();
        let mut rng = Prng::from_seed(21);
        mutate_duplicate(&mut genome, TickCounter::new(42), &mut rng);
        let paralog = &genome.genes[1];
        assert_eq!(paralog.channel_id, parent.channel_id);
        assert_eq!(paralog.effect.channel, parent.effect.channel);
        assert_eq!(paralog.effect.magnitude, parent.effect.magnitude);
        assert_eq!(paralog.effect.radius, parent.effect.radius);
        assert_eq!(paralog.effect.timing, parent.effect.timing);
        assert_eq!(paralog.effect.target, parent.effect.target);
        assert_eq!(paralog.body_site, parent.body_site);
        assert_eq!(paralog.regulatory, parent.regulatory);
        assert_eq!(paralog.enabled, parent.enabled);
        assert_ne!(paralog.lineage_tag, parent.lineage_tag);
        assert!(matches!(
            &paralog.provenance,
            Provenance::Genesis { parent, generation }
                if parent == "kinetic_force" && *generation == 42
        ));
    }

    #[test]
    fn parent_gene_is_not_mutated() {
        let params = params_with_duplication_rate(Q3232::ONE);
        let mut genome = genome_with(params, 3);
        let parents = genome.genes.clone();
        let mut rng = Prng::from_seed(99);
        for tick in 0..50u64 {
            mutate_duplicate(&mut genome, TickCounter::new(tick), &mut rng);
        }
        // The first three slots are still the original parents.
        for (before, after) in parents.iter().zip(genome.genes.iter().take(3)) {
            assert_eq!(before, after);
        }
    }

    #[test]
    fn duplication_is_noop_on_empty_genome() {
        let params = params_with_duplication_rate(Q3232::ONE);
        let mut genome = Genome::with_params(params);
        let mut rng = Prng::from_seed(17);
        for tick in 0..100u64 {
            mutate_duplicate(&mut genome, TickCounter::new(tick), &mut rng);
        }
        assert!(genome.is_empty());
    }

    #[test]
    fn duplication_determinism_same_seed() {
        let params = params_with_duplication_rate(Q3232::from_num(0.2_f64));
        for seed in [1u64, 42, 999, 0xDEAD] {
            let mut ga = genome_with(params.clone(), 3);
            let mut gb = genome_with(params.clone(), 3);
            let mut ra = Prng::from_seed(seed);
            let mut rb = Prng::from_seed(seed);
            for tick in 0..1000u64 {
                mutate_duplicate(&mut ga, TickCounter::new(tick), &mut ra);
                mutate_duplicate(&mut gb, TickCounter::new(tick), &mut rb);
            }
            assert_eq!(ga, gb, "duplication diverged at seed {seed}");
        }
    }

    #[test]
    fn duplication_rate_drift_zero_rate_no_change() {
        let params = GenomeParams {
            duplication_rate: Q3232::from_num(0.05_f64),
            duplication_rate_drift_rate: Q3232::ZERO,
            ..GenomeParams::default()
        };
        let mut genome = genome_with(params, 1);
        let original = genome.params.duplication_rate;
        let mut rng = Prng::from_seed(5);
        for _ in 0..10_000 {
            mutate_duplication_rate(&mut genome, &mut rng);
        }
        assert_eq!(genome.params.duplication_rate, original);
    }

    #[test]
    fn duplication_rate_drift_stays_in_unit_range() {
        let params = GenomeParams {
            duplication_rate: Q3232::from_num(0.5_f64),
            duplication_rate_drift_rate: Q3232::ONE,
            duplication_rate_drift_sigma: Q3232::from_num(0.2_f64),
            ..GenomeParams::default()
        };
        let mut genome = genome_with(params, 1);
        let mut rng = Prng::from_seed(19);
        for _ in 0..10_000 {
            mutate_duplication_rate(&mut genome, &mut rng);
            assert!(
                genome.params.duplication_rate >= Q3232::ZERO
                    && genome.params.duplication_rate <= Q3232::ONE,
                "duplication_rate drifted out of [0, 1]: {:?}",
                genome.params.duplication_rate
            );
        }
    }

    #[test]
    fn duplication_rate_drift_determinism_same_seed() {
        let params = GenomeParams {
            duplication_rate: Q3232::from_num(0.5_f64),
            duplication_rate_drift_rate: Q3232::ONE,
            ..GenomeParams::default()
        };
        for seed in [1u64, 42, 0xBEEF] {
            let mut ga = genome_with(params.clone(), 1);
            let mut gb = genome_with(params.clone(), 1);
            let mut ra = Prng::from_seed(seed);
            let mut rb = Prng::from_seed(seed);
            for _ in 0..1000 {
                mutate_duplication_rate(&mut ga, &mut ra);
                mutate_duplication_rate(&mut gb, &mut rb);
            }
            assert_eq!(ga.params.duplication_rate, gb.params.duplication_rate);
        }
    }
}
