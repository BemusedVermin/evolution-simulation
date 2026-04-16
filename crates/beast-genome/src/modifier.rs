//! Regulatory modifiers — directed edges in the gene regulatory network.
//!
//! A [`Modifier`] lives on a source [`crate::TraitGene`] and perturbs the
//! expression level of another gene in the same genome. This module only
//! owns the data structure — the network relaxation algorithm that
//! consumes modifiers lives in `beast-interpreter` (see System 01 §Layer 2
//! and cut-scope issue for Tarjan SCC damping).

use beast_core::Q3232;
use serde::{Deserialize, Serialize};

use crate::error::{GenomeError, Result};

/// How a modifier perturbs its target's expression.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifierEffect {
    /// Add signed influence to target expression (see §Layer 2).
    Activate,
    /// Subtract signed influence from target expression.
    Suppress,
    /// Multiplicatively modulate target expression.
    Modulate,
}

/// A directed regulatory edge from one gene to another within the same
/// [`crate::Genome`].
///
/// `target_gene_index` is a plain `u32` offset into the owning genome's
/// `Vec<TraitGene>`. This keeps iteration deterministic (index order) and
/// cheap; it also means modifiers are fragile under gene deletion, which
/// is why the duplication and loss operators have to re-number modifiers
/// on mutation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifier {
    /// Index into the owning genome's gene vector.
    pub target_gene_index: u32,
    /// How the modifier perturbs its target.
    pub effect_type: ModifierEffect,
    /// Signed influence strength in `[-1, 1]`.
    pub strength: Q3232,
}

impl Modifier {
    /// Construct a modifier, validating `strength ∈ [-1, 1]` and that
    /// `target_gene_index != source_gene_index`.
    pub fn new(
        source_gene_index: u32,
        target_gene_index: u32,
        effect_type: ModifierEffect,
        strength: Q3232,
    ) -> Result<Self> {
        if source_gene_index == target_gene_index {
            return Err(GenomeError::ModifierSelfLoop {
                index: target_gene_index,
            });
        }
        let neg_one = -Q3232::ONE;
        if strength < neg_one || strength > Q3232::ONE {
            return Err(GenomeError::ModifierStrengthOutOfRange {
                value: format!("{strength:?}"),
            });
        }
        Ok(Self {
            target_gene_index,
            effect_type,
            strength,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_self_loop() {
        let err = Modifier::new(3, 3, ModifierEffect::Activate, Q3232::ZERO).unwrap_err();
        assert!(matches!(err, GenomeError::ModifierSelfLoop { index: 3 }));
    }

    #[test]
    fn rejects_out_of_range_strength() {
        let s = Q3232::from_num(1.5_f64);
        let err = Modifier::new(0, 1, ModifierEffect::Suppress, s).unwrap_err();
        assert!(matches!(
            err,
            GenomeError::ModifierStrengthOutOfRange { .. }
        ));
    }

    #[test]
    fn accepts_boundary_strengths() {
        Modifier::new(0, 1, ModifierEffect::Activate, Q3232::ONE).unwrap();
        Modifier::new(0, 1, ModifierEffect::Suppress, -Q3232::ONE).unwrap();
    }

    #[test]
    fn serde_roundtrip() {
        let m = Modifier::new(1, 2, ModifierEffect::Modulate, Q3232::from_num(0.5_f64)).unwrap();
        let json = serde_json::to_string(&m).unwrap();
        let back: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
