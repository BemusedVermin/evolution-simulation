//! The [`Genome`] — a variable-length, index-ordered collection of trait genes
//! plus per-genome mutation parameters.
//!
//! Iteration order is always index-order. This crate never stores genes in a
//! `HashMap` (or any hash-backed container) because hash randomization would
//! break the determinism invariant documented in `INVARIANTS.md` §1.

use beast_core::Q3232;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::error::{GenomeError, Result};
use crate::gene::TraitGene;

/// Per-genome mutation rates and Gaussian standard deviations.
///
/// Rate fields are probability-per-tick in `[0, 1]`. Sigma fields are the
/// standard deviation passed to `gaussian_q3232` when the corresponding
/// mutation fires.
///
/// The canonical default values mirror System 01 §3 and §6B. In particular
/// `duplication_rate` defaults to `0.0` — per §6B the genesis operators are
/// **disabled for v0** until telemetry confirms baseline evolution is stable.
/// Tests that exercise duplication supply an explicit non-zero rate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeParams {
    /// Per-gene probability of a point-magnitude mutation each tick.
    pub point_mutation_rate: Q3232,
    /// Gaussian σ for point-magnitude mutations.
    pub point_mutation_sigma: Q3232,
    /// Per-gene probability of a channel-shift mutation.
    pub channel_shift_rate: Q3232,
    /// Gaussian σ for channel-shift mutations.
    pub channel_shift_sigma: Q3232,
    /// Per-gene probability of a body-site drift mutation.
    pub body_site_drift_rate: Q3232,
    /// Gaussian σ for body-site drift mutations.
    pub body_site_drift_sigma: Q3232,
    /// Per-gene probability of flipping `enabled`.
    pub silencing_toggle_rate: Q3232,
    /// Per-gene probability of a regulatory rewire.
    pub regulatory_rewire_rate: Q3232,
    /// Per-genome probability of a gene duplication. **Default 0** (v0).
    pub duplication_rate: Q3232,
}

impl Default for GenomeParams {
    fn default() -> Self {
        Self {
            point_mutation_rate: Q3232::from_num(1.0e-3_f64),
            point_mutation_sigma: Q3232::from_num(0.1_f64),
            channel_shift_rate: Q3232::from_num(5.0e-4_f64),
            channel_shift_sigma: Q3232::from_num(0.15_f64),
            body_site_drift_rate: Q3232::from_num(1.0e-3_f64),
            body_site_drift_sigma: Q3232::from_num(0.1_f64),
            silencing_toggle_rate: Q3232::from_num(1.0e-3_f64),
            regulatory_rewire_rate: Q3232::from_num(5.0e-4_f64),
            // Disabled for v0 (System 01 §6B).
            duplication_rate: Q3232::ZERO,
        }
    }
}

/// A full monster genome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Genome {
    /// Mutation-rate parameters for this genome. Mutable so they can evolve
    /// (see §6B — `duplication_rate` is itself mutable).
    pub params: GenomeParams,
    /// Ordered list of genes. Indices are referenced directly by
    /// [`crate::Modifier::target_gene_index`].
    pub genes: Vec<TraitGene>,
}

impl Genome {
    /// Construct an empty genome with the given parameters.
    #[inline]
    #[must_use]
    pub fn with_params(params: GenomeParams) -> Self {
        Self {
            params,
            genes: Vec::new(),
        }
    }

    /// Construct from a gene list. Runs [`Genome::validate`] so the returned
    /// value is guaranteed in-spec; prefer this entry point over populating
    /// `genes` directly in tests and fixtures.
    pub fn new(params: GenomeParams, genes: Vec<TraitGene>) -> Result<Self> {
        let g = Self { params, genes };
        g.validate()?;
        Ok(g)
    }

    /// Number of genes in the genome.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.genes.len()
    }

    /// Whether the genome has zero genes.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.genes.is_empty()
    }

    /// Full structural validation:
    ///
    /// * Every gene passes [`TraitGene::validate_local`].
    /// * Every [`crate::Modifier::target_gene_index`] is in-bounds.
    /// * Every modifier is non-self-loop.
    /// * Lineage tags are unique across the genome.
    /// * All genes share the same channel vector length.
    pub fn validate(&self) -> Result<()> {
        let len = self.genes.len();
        let mut seen_tags: BTreeSet<u64> = BTreeSet::new();
        let expected_channels = self.genes.first().map(|g| g.effect.channel_count());

        for (src_idx, gene) in self.genes.iter().enumerate() {
            gene.validate_local()?;

            if let Some(expected) = expected_channels {
                let got = gene.effect.channel_count();
                if got != expected {
                    return Err(GenomeError::ChannelCountMismatch { expected, got });
                }
            }

            if !seen_tags.insert(gene.lineage_tag.as_u64()) {
                return Err(GenomeError::DuplicateLineageTag {
                    tag: gene.lineage_tag.as_u64(),
                });
            }

            let src_u32 = u32::try_from(src_idx).expect(
                "genome size exceeds u32::MAX — Modifier::target_gene_index can't address this",
            );
            for m in &gene.regulatory {
                if (m.target_gene_index as usize) >= len {
                    return Err(GenomeError::ModifierIndexOutOfBounds {
                        index: m.target_gene_index,
                        len,
                    });
                }
                if m.target_gene_index == src_u32 {
                    return Err(GenomeError::ModifierSelfLoop {
                        index: m.target_gene_index,
                    });
                }
            }
        }
        Ok(())
    }

    /// Iterate genes in index order.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, TraitGene> {
        self.genes.iter()
    }

    /// Iterate genes mutably in index order.
    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, TraitGene> {
        self.genes.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_site::BodyVector;
    use crate::gene::{EffectVector, Target, Timing};
    use crate::lineage::LineageTag;
    use crate::modifier::{Modifier, ModifierEffect};
    use beast_channels::Provenance;

    fn effect(channels: usize) -> EffectVector {
        EffectVector::new(
            vec![Q3232::from_num(0.25_f64); channels],
            Q3232::from_num(0.5_f64),
            Q3232::from_num(0.25_f64),
            Timing::Passive,
            Target::SelfEntity,
        )
        .unwrap()
    }

    fn gene(tag: u64, channels: usize, regs: Vec<Modifier>) -> TraitGene {
        TraitGene::new(
            "kinetic_force",
            effect(channels),
            BodyVector::default_internal(),
            regs,
            true,
            LineageTag::from_raw(tag),
            Provenance::Core,
        )
        .unwrap()
    }

    #[test]
    fn default_params_disable_duplication_for_v0() {
        let p = GenomeParams::default();
        assert_eq!(p.duplication_rate, Q3232::ZERO);
    }

    #[test]
    fn validate_accepts_well_formed_genome() {
        let g = Genome::new(
            GenomeParams::default(),
            vec![gene(1, 3, vec![]), gene(2, 3, vec![])],
        )
        .unwrap();
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn validate_rejects_duplicate_lineage_tags() {
        let err = Genome::new(
            GenomeParams::default(),
            vec![gene(7, 3, vec![]), gene(7, 3, vec![])],
        )
        .unwrap_err();
        assert!(matches!(err, GenomeError::DuplicateLineageTag { tag: 7 }));
    }

    #[test]
    fn validate_rejects_out_of_bounds_modifier() {
        let bad = Modifier {
            target_gene_index: 99,
            effect_type: ModifierEffect::Activate,
            strength: Q3232::from_num(0.5_f64),
        };
        let err = Genome::new(
            GenomeParams::default(),
            vec![gene(1, 3, vec![bad]), gene(2, 3, vec![])],
        )
        .unwrap_err();
        assert!(matches!(
            err,
            GenomeError::ModifierIndexOutOfBounds { index: 99, len: 2 }
        ));
    }

    #[test]
    fn validate_rejects_self_loop_modifier() {
        let bad = Modifier {
            target_gene_index: 0,
            effect_type: ModifierEffect::Activate,
            strength: Q3232::from_num(0.5_f64),
        };
        let err = Genome::new(
            GenomeParams::default(),
            vec![gene(1, 3, vec![bad]), gene(2, 3, vec![])],
        )
        .unwrap_err();
        assert!(matches!(err, GenomeError::ModifierSelfLoop { index: 0 }));
    }

    #[test]
    fn validate_rejects_channel_count_mismatch() {
        let err = Genome::new(
            GenomeParams::default(),
            vec![gene(1, 3, vec![]), gene(2, 4, vec![])],
        )
        .unwrap_err();
        assert!(matches!(
            err,
            GenomeError::ChannelCountMismatch {
                expected: 3,
                got: 4
            }
        ));
    }

    #[test]
    fn serde_roundtrip() {
        let g = Genome::new(
            GenomeParams::default(),
            vec![gene(1, 2, vec![]), gene(2, 2, vec![])],
        )
        .unwrap();
        let json = serde_json::to_string(&g).unwrap();
        let back: Genome = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
        back.validate().unwrap();
    }

    #[test]
    fn iteration_is_index_ordered() {
        let g = Genome::new(
            GenomeParams::default(),
            vec![
                gene(10, 2, vec![]),
                gene(20, 2, vec![]),
                gene(30, 2, vec![]),
            ],
        )
        .unwrap();
        let tags: Vec<u64> = g.iter().map(|gg| gg.lineage_tag.as_u64()).collect();
        assert_eq!(tags, vec![10, 20, 30]);
    }
}
