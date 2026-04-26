//! Strongly typed primitive-manifest representation.
//!
//! Counterpart to `documentation/schemas/primitive_manifest.schema.json`.
//! Follows the same two-stage load pattern as [`beast_channels::manifest`]:
//! JSON Schema validation first, typed deserialization + semantic checks
//! second.

use std::collections::BTreeMap;

use beast_core::Q3232;
use serde::{Deserialize, Serialize};

use crate::category::{Modality, PrimitiveCategory};
use crate::schema::PrimitiveLoadError;

/// Canonical provenance enum, re-exported from `beast-manifest` to keep
/// mod registration symmetric with [`beast_channels::Provenance`].
pub use beast_manifest::Provenance;

/// Parameter value type in a primitive's input schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    /// Continuous numeric value.
    Number,
    /// Discrete integer value.
    Integer,
    /// Categorical string.
    String,
    /// Binary switch.
    Boolean,
}

/// How multiple emissions of the same primitive (in the same tick) collapse
/// into one per parameter. See
/// `documentation/systems/11_phenotype_interpreter.md` §6.2B.
///
/// Declared per-parameter on [`PrimitiveManifest::merge_strategy`]. Absence
/// of a declaration resolves to [`MergeStrategy::Max`] downstream, the spec's
/// conservative default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Sum values via saturating addition. Intended for additive quantities
    /// (force, magnitude, intensity accumulated across multiple sources).
    Sum,
    /// Take the maximum value. Intended for intensity-like quantities where
    /// the strongest contributor dominates (concentration, frequency).
    Max,
    /// Arithmetic mean across the group (`sum / count`).
    Mean,
    /// Set-union semantics.
    ///
    /// The spec defines this for set-valued parameters (tags, molecular
    /// types). [`PrimitiveEffect::parameters`] is a
    /// `BTreeMap<String, Q3232>` today, so there is no representable set —
    /// the interpreter collapses `Union` to the same deterministic rule as
    /// [`MergeStrategy::Max`]. Tracked by
    /// <https://github.com/BemusedVermin/evolution-simulation/issues/95> for
    /// when set-valued parameters land.
    ///
    /// [`PrimitiveEffect::parameters`]: crate::effect::PrimitiveEffect::parameters
    Union,
}

/// Default value stored alongside a parameter spec. Kept as-is from JSON so
/// callers can apply the default into their own domain types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterDefault {
    /// Default for a `number`-typed parameter, already converted to Q32.32.
    Number(Q3232),
    /// Default for an `integer`-typed parameter.
    Integer(i64),
    /// Default for a `string`-typed parameter.
    String(String),
    /// Default for a `boolean`-typed parameter.
    Boolean(bool),
}

/// Specification for a single input parameter on a primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSpec {
    /// Value type (number / integer / string / boolean).
    pub ty: ParameterType,
    /// Optional numeric range `[min, max]`. Only meaningful when
    /// `ty ∈ {Number, Integer}`.
    pub range: Option<(Q3232, Q3232)>,
    /// Optional physical units (free-form string).
    pub units: Option<String>,
    /// Optional default value.
    pub default: Option<ParameterDefault>,
}

/// A cost function's per-parameter power-law scaling term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterScaling {
    /// Name of the parameter driving this term (must exist in `parameter_schema`).
    pub parameter: String,
    /// Power-law exponent (may be negative, fractional, etc.).
    pub exponent: Q3232,
    /// Multiplicative coefficient (non-negative by schema).
    pub coefficient: Q3232,
}

/// Metabolic cost function for a primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostFunction {
    /// Base cost incurred each emission, independent of parameters.
    pub base_metabolic_cost: Q3232,
    /// Zero or more parameter-driven scaling terms, summed to the base.
    pub parameter_scaling: Vec<ParameterScaling>,
}

/// Declaration of which channel (by family or specific id) can emit this
/// primitive. The discriminated union from the schema is preserved here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityEntry {
    /// Compatible with all channels in a family.
    ChannelFamily(beast_channels::ChannelFamily),
    /// Compatible with one specific channel id.
    ChannelId(String),
}

/// How a primitive emission appears to other organisms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservableSignature {
    /// Sense modality.
    pub modality: Modality,
    /// Maximum detection distance (metres). Zero = non-propagating / local.
    pub detection_range_m: Q3232,
    /// Signature string used by the Chronicler for label inference.
    pub pattern_key: String,
}

/// In-memory representation of a primitive manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveManifest {
    /// Unique snake_case identifier.
    pub id: String,
    /// Functional category.
    pub category: PrimitiveCategory,
    /// Human-readable description.
    pub description: String,
    /// Parameter schema in declaration order.
    ///
    /// We use [`BTreeMap`] rather than a `HashMap` so iteration is
    /// deterministic — callers building evaluation pipelines get a stable
    /// order across runs.
    pub parameter_schema: BTreeMap<String, ParameterSpec>,
    /// Channels (by family or by id) that can emit this primitive.
    pub composition_compatibility: Vec<CompatibilityEntry>,
    /// Cost function.
    pub cost_function: CostFunction,
    /// Observable signature.
    pub observable_signature: ObservableSignature,
    /// Per-parameter merge strategy used when multiple hooks emit this
    /// primitive in the same tick. Parameters absent from this map fall
    /// back to [`MergeStrategy::Max`] at merge time (the spec's default).
    ///
    /// [`BTreeMap`] keeps iteration deterministic. Every key in this map
    /// must exist in [`parameter_schema`](Self::parameter_schema); the
    /// loader rejects manifests that reference unknown parameters.
    pub merge_strategy: BTreeMap<String, MergeStrategy>,
    /// Origin of this primitive.
    pub provenance: Provenance,
}

impl PrimitiveManifest {
    /// Load and fully validate a primitive manifest from its JSON source.
    ///
    /// ```
    /// use beast_primitives::PrimitiveManifest;
    /// let json = r#"{
    ///   "id": "example_primitive",
    ///   "category": "signal_emission",
    ///   "description": "A minimal primitive used in documentation tests.",
    ///   "parameter_schema": {
    ///     "frequency_hz": { "type": "number", "range": { "min": 20, "max": 1000 }, "units": "Hz", "default": 100 }
    ///   },
    ///   "composition_compatibility": [ { "channel_family": "motor" } ],
    ///   "cost_function": { "base_metabolic_cost": 1.0 },
    ///   "observable_signature": {
    ///     "modality": "acoustic",
    ///     "detection_range_m": 10,
    ///     "pattern_key": "example_v1"
    ///   },
    ///   "provenance": "core"
    /// }"#;
    /// let manifest = PrimitiveManifest::from_json_str(json).unwrap();
    /// assert_eq!(manifest.id, "example_primitive");
    /// ```
    pub fn from_json_str(source: &str) -> Result<Self, PrimitiveLoadError> {
        crate::schema::load_primitive_manifest(source)
    }

    pub(crate) fn from_validated_value(
        value: &serde_json::Value,
    ) -> Result<Self, PrimitiveLoadError> {
        let raw: RawPrimitiveManifest = serde_json::from_value(value.clone())
            .map_err(|e| PrimitiveLoadError::BadShape(e.to_string()))?;
        raw.into_manifest()
    }
}

// ---------------------------------------------------------------------------
// Raw serde-facing mirror of the schema.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawPrimitiveManifest {
    id: String,
    category: PrimitiveCategory,
    description: String,
    parameter_schema: BTreeMap<String, RawParameterSpec>,
    composition_compatibility: Vec<RawCompatibilityEntry>,
    cost_function: RawCostFunction,
    observable_signature: RawObservableSignature,
    #[serde(default)]
    merge_strategy: BTreeMap<String, MergeStrategy>,
    provenance: String,
}

#[derive(Debug, Deserialize)]
struct RawParameterSpec {
    #[serde(rename = "type")]
    ty: ParameterType,
    #[serde(default)]
    range: Option<RawRange>,
    #[serde(default)]
    units: Option<String>,
    #[serde(default)]
    default: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RawRange {
    min: f64,
    max: f64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCompatibilityEntry {
    Family {
        channel_family: beast_channels::ChannelFamily,
    },
    Id {
        channel_id: String,
    },
}

#[derive(Debug, Deserialize)]
struct RawCostFunction {
    base_metabolic_cost: f64,
    #[serde(default)]
    parameter_scaling: Vec<RawParameterScaling>,
}

#[derive(Debug, Deserialize)]
struct RawParameterScaling {
    parameter: String,
    exponent: f64,
    coefficient: f64,
}

#[derive(Debug, Deserialize)]
struct RawObservableSignature {
    modality: Modality,
    detection_range_m: f64,
    pattern_key: String,
}

impl RawPrimitiveManifest {
    fn into_manifest(self) -> Result<PrimitiveManifest, PrimitiveLoadError> {
        // --- parameter schema ---
        let mut parameter_schema: BTreeMap<String, ParameterSpec> = BTreeMap::new();
        for (name, raw) in self.parameter_schema {
            let range = if let Some(r) = raw.range {
                if r.min > r.max {
                    return Err(PrimitiveLoadError::InvalidRange {
                        primitive_id: self.id.clone(),
                        reason: format!(
                            "parameter `{name}` range.min ({}) must be <= range.max ({})",
                            r.min, r.max
                        ),
                    });
                }
                Some((Q3232::from_num(r.min), Q3232::from_num(r.max)))
            } else {
                None
            };

            let default = if let Some(v) = raw.default {
                Some(parse_default(&name, &self.id, raw.ty, &v)?)
            } else {
                None
            };

            parameter_schema.insert(
                name,
                ParameterSpec {
                    ty: raw.ty,
                    range,
                    units: raw.units,
                    default,
                },
            );
        }

        // --- composition compatibility ---
        let composition_compatibility: Vec<CompatibilityEntry> = self
            .composition_compatibility
            .into_iter()
            .map(|e| match e {
                RawCompatibilityEntry::Family { channel_family } => {
                    CompatibilityEntry::ChannelFamily(channel_family)
                }
                RawCompatibilityEntry::Id { channel_id } => {
                    CompatibilityEntry::ChannelId(channel_id)
                }
            })
            .collect();

        // --- cost function ---
        let cost_function = CostFunction {
            base_metabolic_cost: Q3232::from_num(self.cost_function.base_metabolic_cost),
            parameter_scaling: self
                .cost_function
                .parameter_scaling
                .into_iter()
                .map(|s| {
                    if !parameter_schema.contains_key(&s.parameter) {
                        return Err(PrimitiveLoadError::UnknownScalingParameter {
                            primitive_id: self.id.clone(),
                            parameter: s.parameter.clone(),
                        });
                    }
                    Ok(ParameterScaling {
                        parameter: s.parameter,
                        exponent: Q3232::from_num(s.exponent),
                        coefficient: Q3232::from_num(s.coefficient),
                    })
                })
                .collect::<Result<_, _>>()?,
        };

        // --- observable signature ---
        let observable_signature = ObservableSignature {
            modality: self.observable_signature.modality,
            detection_range_m: Q3232::from_num(self.observable_signature.detection_range_m),
            pattern_key: self.observable_signature.pattern_key,
        };

        // --- merge strategy ---
        for param in self.merge_strategy.keys() {
            if !parameter_schema.contains_key(param) {
                return Err(PrimitiveLoadError::UnknownMergeStrategyParameter {
                    primitive_id: self.id.clone(),
                    parameter: param.clone(),
                });
            }
        }

        // --- provenance ---
        let provenance = Provenance::parse(&self.provenance)?;

        Ok(PrimitiveManifest {
            id: self.id,
            category: self.category,
            description: self.description,
            parameter_schema,
            composition_compatibility,
            cost_function,
            observable_signature,
            merge_strategy: self.merge_strategy,
            provenance,
        })
    }
}

fn parse_default(
    param_name: &str,
    primitive_id: &str,
    ty: ParameterType,
    value: &serde_json::Value,
) -> Result<ParameterDefault, PrimitiveLoadError> {
    let mismatch = || PrimitiveLoadError::InvalidDefault {
        primitive_id: primitive_id.to_owned(),
        parameter: param_name.to_owned(),
        reason: format!("default value {value} does not match declared type {ty:?}"),
    };
    match ty {
        ParameterType::Number => {
            let f = value.as_f64().ok_or_else(mismatch)?;
            Ok(ParameterDefault::Number(Q3232::from_num(f)))
        }
        ParameterType::Integer => {
            let i = value.as_i64().ok_or_else(mismatch)?;
            Ok(ParameterDefault::Integer(i))
        }
        ParameterType::String => {
            let s = value.as_str().ok_or_else(mismatch)?;
            Ok(ParameterDefault::String(s.to_owned()))
        }
        ParameterType::Boolean => {
            let b = value.as_bool().ok_or_else(mismatch)?;
            Ok(ParameterDefault::Boolean(b))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_parses_all_shapes() {
        assert_eq!(Provenance::parse("core").unwrap(), Provenance::Core);
        assert_eq!(
            Provenance::parse("mod:my_mod").unwrap(),
            Provenance::Mod("my_mod".to_owned())
        );
        assert_eq!(
            Provenance::parse("genesis:parent:12").unwrap(),
            Provenance::Genesis {
                parent: "parent".to_owned(),
                generation: 12
            }
        );
        assert!(Provenance::parse("unknown").is_err());
    }
}
