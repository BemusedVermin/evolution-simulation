//! Strongly typed channel-manifest representation.
//!
//! The types here are the in-memory counterpart of
//! `documentation/schemas/channel_manifest.schema.json`. JSON Schema
//! validation happens in [`crate::schema`]; this module owns the *semantic*
//! parsing step — converting already-valid JSON into Q32.32 fixed-point
//! configuration data and rejecting anything that is structurally valid but
//! invariant-violating (e.g. `range.min > range.max`).

use std::collections::BTreeSet;

use beast_core::Q3232;
use serde::{Deserialize, Serialize};

use crate::composition::{CompositionHook, CompositionKind};
use crate::expression::ExpressionCondition;
use crate::schema::ChannelLoadError;

/// Biological family of a channel.
///
/// Determines typical mutation breadth, composition patterns, and expression
/// logic (see `documentation/schemas/README.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelFamily {
    /// Perception thresholds, acuity ranges.
    Sensory,
    /// Strength, speed, precision.
    Motor,
    /// Energy, digestion rate, body mass.
    Metabolic,
    /// Bone density, hide thickness.
    Structural,
    /// Hormones, internal clocks, threshold gates.
    Regulatory,
    /// Group-size preference, bonding, density gating.
    Social,
    /// Learning, memory, integration.
    Cognitive,
    /// Fertility, mate choice, parental investment.
    Reproductive,
    /// Growth rate, body-plan variation, stage-gated traits.
    Developmental,
}

/// Bounds policy applied when a mutation pushes a channel value out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundsPolicy {
    /// Truncate to the boundary.
    Clamp,
    /// Reflect off the boundary.
    Reflect,
    /// Wrap around the range (periodic).
    Wrap,
}

/// Origin of a channel. Mirrors the schema's `provenance` discriminated regex.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// A canonical channel shipped by the core game.
    Core,
    /// Registered by a mod with the given snake_case id.
    Mod(String),
    /// Duplicated from a parent channel at generation `n`.
    Genesis {
        /// Parent channel id.
        parent: String,
        /// Generation at which the duplication occurred.
        generation: u64,
    },
}

impl Provenance {
    /// Parse the JSON `provenance` string.
    ///
    /// The schema's regex (`^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$`)
    /// is enforced by JSON Schema validation, so this parser assumes the
    /// structural shape is well-formed and only does the split.
    pub(crate) fn parse(raw: &str) -> Result<Self, ChannelLoadError> {
        if raw == "core" {
            return Ok(Self::Core);
        }
        if let Some(rest) = raw.strip_prefix("mod:") {
            return Ok(Self::Mod(rest.to_owned()));
        }
        if let Some(rest) = raw.strip_prefix("genesis:") {
            // rest is `parent_id:generation`.
            let mut parts = rest.rsplitn(2, ':');
            let gen_str = parts
                .next()
                .ok_or_else(|| ChannelLoadError::InvalidProvenance(raw.to_owned()))?;
            let parent = parts
                .next()
                .ok_or_else(|| ChannelLoadError::InvalidProvenance(raw.to_owned()))?;
            let generation: u64 = gen_str
                .parse()
                .map_err(|_| ChannelLoadError::InvalidProvenance(raw.to_owned()))?;
            return Ok(Self::Genesis {
                parent: parent.to_owned(),
                generation,
            });
        }
        Err(ChannelLoadError::InvalidProvenance(raw.to_owned()))
    }
}

/// Numeric range with physical units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    /// Minimum valid value (inclusive).
    pub min: Q3232,
    /// Maximum valid value (inclusive).
    pub max: Q3232,
    /// Physical units (e.g. `"dB"`, `"kg"`, `"Hz"`, `"dimensionless"`).
    pub units: String,
}

/// Applicable body-mass range (macro/meso/micro scale).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleBand {
    /// Lower bound (kg, inclusive).
    pub min_kg: Q3232,
    /// Upper bound (kg, inclusive).
    pub max_kg: Q3232,
}

/// Correlation declaration between two channels' mutations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelationEntry {
    /// Target channel id (validated as snake_case by JSON Schema).
    pub channel: String,
    /// Pearson coefficient, clipped to `[-1, 1]` by the schema.
    pub coefficient: Q3232,
}

/// Gaussian mutation kernel parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationKernel {
    /// Standard deviation proportional to the range width (> 0 by schema).
    pub sigma: Q3232,
    /// How mutations hitting a range boundary are handled.
    pub bounds_policy: BoundsPolicy,
    /// Relative selection weight during channel genesis (≥ 0 by schema).
    pub genesis_weight: Q3232,
    /// Optional correlated-mutation declarations.
    pub correlation_with: Vec<CorrelationEntry>,
}

/// In-memory representation of a single channel manifest.
///
/// Load a manifest with [`ChannelManifest::from_json_str`] or
/// [`crate::schema::load_channel_manifest`]. Both flows run JSON Schema
/// validation first so downstream code can trust every invariant encoded in
/// the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelManifest {
    /// Unique snake_case identifier.
    pub id: String,
    /// Biological family.
    pub family: ChannelFamily,
    /// Human-readable description.
    pub description: String,
    /// Numeric range with physical units.
    pub range: Range,
    /// Gaussian mutation kernel parameters.
    pub mutation_kernel: MutationKernel,
    /// Composition hooks (interactions with other channels).
    pub composition_hooks: Vec<CompositionHook>,
    /// Expression condition gates (all must hold).
    pub expression_conditions: Vec<ExpressionCondition>,
    /// Applicable body-mass range.
    pub scale_band: ScaleBand,
    /// Whether the channel can vary across body sites.
    pub body_site_applicable: bool,
    /// Origin of this channel.
    pub provenance: Provenance,
}

impl ChannelManifest {
    /// Load and validate a channel manifest from a JSON string.
    ///
    /// ```
    /// use beast_channels::ChannelManifest;
    /// let json = r#"{
    ///   "id": "example_channel",
    ///   "family": "sensory",
    ///   "description": "An example channel used in documentation tests.",
    ///   "range": { "min": 0, "max": 1, "units": "dimensionless" },
    ///   "mutation_kernel": {
    ///     "sigma": 0.1,
    ///     "bounds_policy": "clamp",
    ///     "genesis_weight": 1.0
    ///   },
    ///   "composition_hooks": [],
    ///   "expression_conditions": [],
    ///   "scale_band": { "min_kg": 0.01, "max_kg": 1000 },
    ///   "body_site_applicable": true,
    ///   "provenance": "core"
    /// }"#;
    /// let manifest = ChannelManifest::from_json_str(json).unwrap();
    /// assert_eq!(manifest.id, "example_channel");
    /// ```
    pub fn from_json_str(source: &str) -> Result<Self, ChannelLoadError> {
        crate::schema::load_channel_manifest(source)
    }

    /// Construct from an already-validated `serde_json::Value`.
    ///
    /// Called by [`crate::schema::load_channel_manifest`] after the schema
    /// validator has accepted the document. External callers should prefer
    /// [`Self::from_json_str`] so JSON Schema validation always runs.
    pub(crate) fn from_validated_value(
        value: &serde_json::Value,
    ) -> Result<Self, ChannelLoadError> {
        let raw: RawChannelManifest = serde_json::from_value(value.clone())
            .map_err(|e| ChannelLoadError::BadShape(e.to_string()))?;
        raw.into_manifest()
    }
}

// ---------------------------------------------------------------------------
// Raw serde-facing mirror of the schema.
// ---------------------------------------------------------------------------

/// Exact serde mirror of the JSON Schema. Kept private — downstream types
/// consume [`ChannelManifest`] with Q32.32 fields.
#[derive(Debug, Deserialize)]
struct RawChannelManifest {
    id: String,
    family: ChannelFamily,
    description: String,
    range: RawRange,
    mutation_kernel: RawMutationKernel,
    composition_hooks: Vec<RawCompositionHook>,
    expression_conditions: Vec<RawExpressionCondition>,
    scale_band: RawScaleBand,
    body_site_applicable: bool,
    provenance: String,
}

#[derive(Debug, Deserialize)]
struct RawRange {
    min: f64,
    max: f64,
    units: String,
}

#[derive(Debug, Deserialize)]
struct RawMutationKernel {
    sigma: f64,
    bounds_policy: BoundsPolicy,
    genesis_weight: f64,
    #[serde(default)]
    correlation_with: Vec<RawCorrelationEntry>,
}

#[derive(Debug, Deserialize)]
struct RawCorrelationEntry {
    channel: String,
    coefficient: f64,
}

#[derive(Debug, Deserialize)]
struct RawCompositionHook {
    with: String,
    kind: CompositionKind,
    coefficient: f64,
    #[serde(default)]
    threshold: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RawExpressionCondition {
    BiomeFlag { flag: String },
    ScaleBand { min_kg: f64, max_kg: f64 },
    Season { season: String },
    DevelopmentalStage { stage: String },
    SocialDensity { min_per_km2: f64, max_per_km2: f64 },
}

#[derive(Debug, Deserialize)]
struct RawScaleBand {
    min_kg: f64,
    max_kg: f64,
}

impl RawChannelManifest {
    fn into_manifest(self) -> Result<ChannelManifest, ChannelLoadError> {
        // --- range ---
        if self.range.min > self.range.max {
            return Err(ChannelLoadError::InvalidRange {
                channel_id: self.id.clone(),
                reason: format!(
                    "range.min ({}) must be <= range.max ({})",
                    self.range.min, self.range.max
                ),
            });
        }
        let range = Range {
            min: Q3232::from_num(self.range.min),
            max: Q3232::from_num(self.range.max),
            units: self.range.units,
        };

        // --- scale band ---
        if self.scale_band.min_kg > self.scale_band.max_kg {
            return Err(ChannelLoadError::InvalidRange {
                channel_id: self.id.clone(),
                reason: format!(
                    "scale_band.min_kg ({}) must be <= scale_band.max_kg ({})",
                    self.scale_band.min_kg, self.scale_band.max_kg
                ),
            });
        }
        let scale_band = ScaleBand {
            min_kg: Q3232::from_num(self.scale_band.min_kg),
            max_kg: Q3232::from_num(self.scale_band.max_kg),
        };

        // --- mutation kernel ---
        let kernel = MutationKernel {
            sigma: Q3232::from_num(self.mutation_kernel.sigma),
            bounds_policy: self.mutation_kernel.bounds_policy,
            genesis_weight: Q3232::from_num(self.mutation_kernel.genesis_weight),
            correlation_with: self
                .mutation_kernel
                .correlation_with
                .into_iter()
                .map(|c| CorrelationEntry {
                    channel: c.channel,
                    coefficient: Q3232::from_num(c.coefficient),
                })
                .collect(),
        };

        // --- composition hooks: reject duplicates on `with`, enforce threshold rule ---
        let mut seen = BTreeSet::new();
        let mut hooks = Vec::with_capacity(self.composition_hooks.len());
        for raw in self.composition_hooks {
            let needs_threshold = matches!(
                raw.kind,
                CompositionKind::Threshold | CompositionKind::Gating
            );
            let threshold = match (needs_threshold, raw.threshold) {
                (true, Some(t)) => Some(Q3232::from_num(t)),
                (true, None) => {
                    return Err(ChannelLoadError::MissingThreshold {
                        channel_id: self.id.clone(),
                        with: raw.with.clone(),
                        kind: raw.kind,
                    });
                }
                (false, _) => None,
            };
            if !seen.insert(raw.with.clone()) {
                return Err(ChannelLoadError::DuplicateHook {
                    channel_id: self.id.clone(),
                    with: raw.with,
                });
            }
            hooks.push(CompositionHook {
                with: raw.with,
                kind: raw.kind,
                coefficient: Q3232::from_num(raw.coefficient),
                threshold,
            });
        }

        // --- expression conditions ---
        let mut conditions = Vec::with_capacity(self.expression_conditions.len());
        for raw in self.expression_conditions {
            conditions.push(match raw {
                RawExpressionCondition::BiomeFlag { flag } => {
                    ExpressionCondition::BiomeFlag { flag }
                }
                RawExpressionCondition::ScaleBand { min_kg, max_kg } => {
                    if min_kg > max_kg {
                        return Err(ChannelLoadError::InvalidRange {
                            channel_id: self.id.clone(),
                            reason: format!(
                                "expression_conditions.scale_band min_kg ({min_kg}) must be <= max_kg ({max_kg})"
                            ),
                        });
                    }
                    ExpressionCondition::ScaleBand {
                        min_kg: Q3232::from_num(min_kg),
                        max_kg: Q3232::from_num(max_kg),
                    }
                }
                RawExpressionCondition::Season { season } => {
                    ExpressionCondition::Season { season }
                }
                RawExpressionCondition::DevelopmentalStage { stage } => {
                    ExpressionCondition::DevelopmentalStage { stage }
                }
                RawExpressionCondition::SocialDensity {
                    min_per_km2,
                    max_per_km2,
                } => {
                    if min_per_km2 > max_per_km2 {
                        return Err(ChannelLoadError::InvalidRange {
                            channel_id: self.id.clone(),
                            reason: format!(
                                "expression_conditions.social_density min_per_km2 ({min_per_km2}) must be <= max_per_km2 ({max_per_km2})"
                            ),
                        });
                    }
                    ExpressionCondition::SocialDensity {
                        min_per_km2: Q3232::from_num(min_per_km2),
                        max_per_km2: Q3232::from_num(max_per_km2),
                    }
                }
            });
        }

        let provenance = Provenance::parse(&self.provenance)?;

        Ok(ChannelManifest {
            id: self.id,
            family: self.family,
            description: self.description,
            range,
            mutation_kernel: kernel,
            composition_hooks: hooks,
            expression_conditions: conditions,
            scale_band,
            body_site_applicable: self.body_site_applicable,
            provenance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_core_parses() {
        assert_eq!(Provenance::parse("core").unwrap(), Provenance::Core);
    }

    #[test]
    fn provenance_mod_parses() {
        assert_eq!(
            Provenance::parse("mod:my_mod").unwrap(),
            Provenance::Mod("my_mod".to_owned())
        );
    }

    #[test]
    fn provenance_genesis_parses() {
        assert_eq!(
            Provenance::parse("genesis:auditory_sensitivity:50").unwrap(),
            Provenance::Genesis {
                parent: "auditory_sensitivity".to_owned(),
                generation: 50,
            }
        );
    }

    #[test]
    fn provenance_genesis_missing_generation_rejected() {
        assert!(Provenance::parse("genesis:parent").is_err());
    }

    #[test]
    fn provenance_unknown_prefix_rejected() {
        assert!(Provenance::parse("unknown:foo").is_err());
    }
}
