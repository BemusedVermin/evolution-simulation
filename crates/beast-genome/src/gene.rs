//! Trait genes — the unit of mutation and channel contribution.
//!
//! A [`TraitGene`] is a compositional record: it declares *what* it produces
//! (the [`EffectVector`]), *how* it activates ([`Timing`]/[`Target`]), and
//! (in a follow-up story) *where* on the body it manifests. Ancestry is
//! tracked via a [`crate::LineageTag`] and a
//! [`beast_channels::Provenance`] string so that speciation metrics can
//! walk back through duplication events without scanning the entire
//! phylogeny.
//!
//! `BodyVector` is introduced in story S3.2; for now every gene carries a
//! placeholder `body_site` field typed as `Option<BodyVector>` so the shape
//! is forward-compatible without forcing call sites to supply a default
//! today.

use beast_channels::Provenance;
use beast_core::Q3232;
use serde::{Deserialize, Serialize};

use crate::error::{GenomeError, Result};
use crate::lineage::LineageTag;
use crate::modifier::Modifier;

/// When a gene's effect fires during the tick loop.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    /// Always on (background expression).
    Passive,
    /// Fires when this entity touches another.
    OnContact,
    /// Fires when this entity takes damage.
    OnDamage,
    /// Fires on a cooldown timer (period controlled by interpreter).
    OnCooldown,
    /// Fires every `N` ticks while conditions hold.
    Periodic,
}

/// Who a gene's effect targets when it fires.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// The gene's own entity.
    SelfEntity,
    /// The entity this gene's owner just touched.
    TouchedEntity,
    /// An area around the owner containing friendly entities.
    AreaFriend,
    /// An area around the owner containing foes.
    AreaFoe,
    /// The surrounding environment (biome cell).
    Environment,
}

/// What a gene produces when expressed.
///
/// The `channel` vector is indexed by position in the loaded
/// [`beast_channels::ChannelRegistry`]. All other numeric fields are in the
/// canonical `[0, 1]` range (see [`Q3232`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectVector {
    /// Per-channel contribution, one entry per channel in the registry.
    pub channel: Vec<Q3232>,
    /// Overall expression strength in `[0, 1]`.
    pub magnitude: Q3232,
    /// Spatial reach in `[0, 1]` (0 = self-only, 1 = wide AoE).
    pub radius: Q3232,
    /// When the effect fires.
    pub timing: Timing,
    /// Who the effect is applied to.
    pub target: Target,
}

impl EffectVector {
    /// Construct a new effect vector, validating unit-range fields.
    pub fn new(
        channel: Vec<Q3232>,
        magnitude: Q3232,
        radius: Q3232,
        timing: Timing,
        target: Target,
    ) -> Result<Self> {
        check_unit("magnitude", magnitude)?;
        check_unit("radius", radius)?;
        Ok(Self {
            channel,
            magnitude,
            radius,
            timing,
            target,
        })
    }

    /// Number of channels declared by this effect.
    #[inline]
    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channel.len()
    }
}

/// A single trait gene.
///
/// Field ordering matches the design doc (System 01 §3, Layer 1). Fields
/// are `pub` so mutation operators can rewrite them in place; constructors
/// validate ranges so newly minted genes can't be out-of-spec.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitGene {
    /// The `id` of the channel family this gene primarily serves (used for
    /// reclassification and duplication provenance). Free-form snake_case;
    /// validated against the live [`beast_channels::ChannelRegistry`] at a
    /// higher layer.
    pub channel_id: String,
    /// What the gene produces.
    pub effect: EffectVector,
    /// Where on the body the effect manifests (introduced in S3.2).
    pub body_site: Option<BodySitePlaceholder>,
    /// Outgoing regulatory edges.
    pub regulatory: Vec<Modifier>,
    /// Whether the gene is currently expressed. Silencing toggles flip
    /// this without deleting the gene.
    pub enabled: bool,
    /// Phylogenetic identifier for this lineage.
    pub lineage_tag: LineageTag,
    /// Where the gene came from (`core`, a mod, or a genesis paralog).
    pub provenance: Provenance,
}

/// Placeholder for the full body-site vector introduced in story S3.2.
///
/// Exists so `TraitGene` already reserves the slot and save-file layout is
/// forward-compatible. Do not add fields here — extend
/// `crate::body_site::BodyVector` instead when S3.2 lands and swap the
/// placeholder out.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BodySitePlaceholder;

impl TraitGene {
    /// Construct a trait gene. Performs only the validations that are local
    /// to a single gene; inter-gene checks (modifier index bounds, lineage
    /// uniqueness) happen in [`crate::Genome::validate`].
    pub fn new(
        channel_id: impl Into<String>,
        effect: EffectVector,
        regulatory: Vec<Modifier>,
        enabled: bool,
        lineage_tag: LineageTag,
        provenance: Provenance,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            effect,
            body_site: None,
            regulatory,
            enabled,
            lineage_tag,
            provenance,
        }
    }

    /// Validate local ranges (effect magnitudes, modifier strengths). Does
    /// not validate modifier target indices — the owning genome does that.
    pub fn validate_local(&self) -> Result<()> {
        check_unit("magnitude", self.effect.magnitude)?;
        check_unit("radius", self.effect.radius)?;
        let neg_one = -Q3232::ONE;
        for m in &self.regulatory {
            if m.strength < neg_one || m.strength > Q3232::ONE {
                return Err(GenomeError::ModifierStrengthOutOfRange {
                    value: format!("{:?}", m.strength),
                });
            }
        }
        Ok(())
    }
}

fn check_unit(field: &'static str, v: Q3232) -> Result<()> {
    if v < Q3232::ZERO || v > Q3232::ONE {
        return Err(GenomeError::OutOfUnitRange {
            field,
            value: format!("{v:?}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modifier::ModifierEffect;

    fn effect(channels: usize) -> EffectVector {
        EffectVector::new(
            vec![Q3232::from_num(0.1_f64); channels],
            Q3232::from_num(0.5_f64),
            Q3232::from_num(0.25_f64),
            Timing::Passive,
            Target::SelfEntity,
        )
        .unwrap()
    }

    fn gene() -> TraitGene {
        TraitGene::new(
            "kinetic_force",
            effect(4),
            vec![],
            true,
            LineageTag::from_raw(0xAAAA),
            Provenance::Core,
        )
    }

    #[test]
    fn rejects_out_of_range_magnitude() {
        let err = EffectVector::new(
            vec![Q3232::ZERO; 2],
            Q3232::from_num(1.5_f64),
            Q3232::ZERO,
            Timing::Passive,
            Target::SelfEntity,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            GenomeError::OutOfUnitRange {
                field: "magnitude",
                ..
            }
        ));
    }

    #[test]
    fn rejects_out_of_range_radius() {
        let err = EffectVector::new(
            vec![Q3232::ZERO; 2],
            Q3232::ZERO,
            -Q3232::from_num(0.1_f64),
            Timing::Passive,
            Target::SelfEntity,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            GenomeError::OutOfUnitRange {
                field: "radius",
                ..
            }
        ));
    }

    #[test]
    fn validate_local_catches_modifier_strength() {
        let mut g = gene();
        // Bypass Modifier::new's validation by pushing a raw struct.
        g.regulatory.push(Modifier {
            target_gene_index: 0,
            effect_type: ModifierEffect::Activate,
            strength: Q3232::from_num(2_i32),
        });
        assert!(matches!(
            g.validate_local().unwrap_err(),
            GenomeError::ModifierStrengthOutOfRange { .. }
        ));
    }

    #[test]
    fn serde_roundtrip_full_gene() {
        let g = gene();
        let json = serde_json::to_string(&g).unwrap();
        let back: TraitGene = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn serde_preserves_provenance_genesis_form() {
        let g = TraitGene::new(
            "kinetic_force",
            effect(2),
            vec![],
            true,
            LineageTag::from_raw(1),
            Provenance::Genesis {
                parent: "kinetic_force".to_owned(),
                generation: 42,
            },
        );
        let json = serde_json::to_string(&g).unwrap();
        assert!(json.contains("\"genesis:kinetic_force:42\""));
        let back: TraitGene = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}
