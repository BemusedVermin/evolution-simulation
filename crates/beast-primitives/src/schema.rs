//! JSON Schema validation for primitive manifests.
//!
//! Two-stage loader, matching `beast_channels::schema`:
//!
//! 1. Parse source as `serde_json::Value`, validate against the embedded
//!    schema (failures flattened to path + message pairs).
//! 2. Deserialize the validated value into [`PrimitiveManifest`] and run
//!    semantic checks (range ordering, defaults matching declared types,
//!    scaling parameters referring to known input parameters, provenance
//!    parsing).
//!
//! The schema compilation + pointer-flattened validator lives in
//! [`beast_manifest::CompiledSchema`]; primitive-specific semantic errors
//! are encoded in [`PrimitiveLoadError`] below.

use std::sync::OnceLock;

use beast_manifest::{CompiledSchema, ProvenanceParseError, SchemaLoadError};
use thiserror::Error;

use crate::manifest::PrimitiveManifest;

/// Re-export of the shared violation record — kept here so downstream
/// callers can continue to `use beast_primitives::SchemaViolation`.
pub use beast_manifest::SchemaViolation;

/// Raw JSON text of the primitive manifest schema. Embedded at compile time.
pub const PRIMITIVE_MANIFEST_SCHEMA: &str =
    include_str!("../../../documentation/schemas/primitive_manifest.schema.json");

/// Errors produced while loading a primitive manifest.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PrimitiveLoadError {
    /// Input was not parsable as JSON.
    #[error("manifest is not valid JSON: {0}")]
    InvalidJson(String),

    /// JSON Schema validation produced one or more errors.
    #[error("manifest failed schema validation:\n{}", beast_manifest::format_schema_errors(.0))]
    SchemaViolation(Vec<SchemaViolation>),

    /// Deserialization into the typed manifest struct failed after schema
    /// validation passed — indicates a schema/type drift bug.
    #[error("manifest deserialized unexpectedly: {0}")]
    BadShape(String),

    /// `provenance` did not match one of the recognized shapes.
    #[error("invalid provenance string: {0}")]
    InvalidProvenance(String),

    /// A numeric range was inverted.
    #[error("primitive {primitive_id} has invalid range: {reason}")]
    InvalidRange {
        /// Primitive id from the manifest.
        primitive_id: String,
        /// Human description of which range inverted.
        reason: String,
    },

    /// A cost-function scaling term referenced a parameter that is not
    /// declared in `parameter_schema`.
    #[error("primitive {primitive_id} cost scaling references unknown parameter `{parameter}`")]
    UnknownScalingParameter {
        /// Primitive id from the manifest.
        primitive_id: String,
        /// Offending parameter name.
        parameter: String,
    },

    /// A `merge_strategy` entry referenced a parameter that is not declared
    /// in `parameter_schema`. Merge strategies are per-parameter, so an
    /// unknown key is a manifest authoring bug.
    #[error("primitive {primitive_id} merge_strategy references unknown parameter `{parameter}`")]
    UnknownMergeStrategyParameter {
        /// Primitive id from the manifest.
        primitive_id: String,
        /// Offending parameter name.
        parameter: String,
    },

    /// A parameter default value did not match the declared type.
    #[error("primitive {primitive_id} parameter `{parameter}` has invalid default: {reason}")]
    InvalidDefault {
        /// Primitive id from the manifest.
        primitive_id: String,
        /// Offending parameter name.
        parameter: String,
        /// Human explanation.
        reason: String,
    },
}

impl From<SchemaLoadError> for PrimitiveLoadError {
    fn from(e: SchemaLoadError) -> Self {
        match e {
            SchemaLoadError::InvalidJson(s) => Self::InvalidJson(s),
            SchemaLoadError::SchemaViolation(v) => Self::SchemaViolation(v),
        }
    }
}

impl From<ProvenanceParseError> for PrimitiveLoadError {
    fn from(e: ProvenanceParseError) -> Self {
        Self::InvalidProvenance(e.0)
    }
}

fn compiled_schema() -> &'static CompiledSchema {
    static SCHEMA: OnceLock<CompiledSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| CompiledSchema::compile(PRIMITIVE_MANIFEST_SCHEMA))
}

/// Load and fully validate a primitive manifest from its JSON source.
pub fn load_primitive_manifest(source: &str) -> Result<PrimitiveManifest, PrimitiveLoadError> {
    let value = compiled_schema().load(source)?;
    PrimitiveManifest::from_validated_value(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_schema_compiles() {
        let _ = compiled_schema();
    }

    #[test]
    fn minimal_primitive_loads() {
        let src = r#"{
            "id": "minimal",
            "category": "force_application",
            "description": "Minimal valid primitive for tests.",
            "parameter_schema": {
                "p": { "type": "number", "range": { "min": 0, "max": 1 } }
            },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": { "base_metabolic_cost": 0.5 },
            "observable_signature": {
                "modality": "mechanical",
                "detection_range_m": 1,
                "pattern_key": "minimal_v1"
            },
            "provenance": "core"
        }"#;
        assert!(load_primitive_manifest(src).is_ok());
    }

    #[test]
    fn schema_rejects_unknown_category() {
        let src = r#"{
            "id": "bad",
            "category": "telepathy",
            "description": "Not a real category.",
            "parameter_schema": { "p": { "type": "number" } },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": { "base_metabolic_cost": 1 },
            "observable_signature": { "modality": "behavioral", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        assert!(matches!(
            load_primitive_manifest(src),
            Err(PrimitiveLoadError::SchemaViolation(_))
        ));
    }

    #[test]
    fn semantic_unknown_scaling_parameter_rejected() {
        let src = r#"{
            "id": "bad",
            "category": "force_application",
            "description": "Cost refers to a parameter that does not exist.",
            "parameter_schema": { "p": { "type": "number" } },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": {
                "base_metabolic_cost": 1,
                "parameter_scaling": [ { "parameter": "ghost", "exponent": 1, "coefficient": 1 } ]
            },
            "observable_signature": { "modality": "behavioral", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        assert!(matches!(
            load_primitive_manifest(src),
            Err(PrimitiveLoadError::UnknownScalingParameter { .. })
        ));
    }

    #[test]
    fn merge_strategy_accepts_all_four_variants() {
        use crate::manifest::MergeStrategy;

        let src = r#"{
            "id": "strategies",
            "category": "force_application",
            "description": "Exercises every merge strategy variant.",
            "parameter_schema": {
                "a": { "type": "number" },
                "b": { "type": "number" },
                "c": { "type": "number" },
                "d": { "type": "number" }
            },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": { "base_metabolic_cost": 1 },
            "observable_signature": { "modality": "behavioral", "detection_range_m": 1, "pattern_key": "k" },
            "merge_strategy": {
                "a": "sum",
                "b": "max",
                "c": "mean",
                "d": "union"
            },
            "provenance": "core"
        }"#;
        let manifest = load_primitive_manifest(src).unwrap();
        assert_eq!(manifest.merge_strategy.len(), 4);
        assert_eq!(manifest.merge_strategy["a"], MergeStrategy::Sum);
        assert_eq!(manifest.merge_strategy["b"], MergeStrategy::Max);
        assert_eq!(manifest.merge_strategy["c"], MergeStrategy::Mean);
        assert_eq!(manifest.merge_strategy["d"], MergeStrategy::Union);
    }

    #[test]
    fn merge_strategy_absent_is_empty_map() {
        let src = r#"{
            "id": "no_strategy",
            "category": "force_application",
            "description": "Omits merge_strategy entirely — loader leaves the map empty.",
            "parameter_schema": { "p": { "type": "number" } },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": { "base_metabolic_cost": 1 },
            "observable_signature": { "modality": "behavioral", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        let manifest = load_primitive_manifest(src).unwrap();
        assert!(manifest.merge_strategy.is_empty());
    }

    #[test]
    fn schema_rejects_unknown_merge_strategy_value() {
        let src = r#"{
            "id": "bad",
            "category": "force_application",
            "description": "Declares a merge strategy that isn't in the enum.",
            "parameter_schema": { "p": { "type": "number" } },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": { "base_metabolic_cost": 1 },
            "observable_signature": { "modality": "behavioral", "detection_range_m": 1, "pattern_key": "k" },
            "merge_strategy": { "p": "median" },
            "provenance": "core"
        }"#;
        assert!(matches!(
            load_primitive_manifest(src),
            Err(PrimitiveLoadError::SchemaViolation(_))
        ));
    }

    #[test]
    fn semantic_unknown_merge_strategy_parameter_rejected() {
        let src = r#"{
            "id": "bad",
            "category": "force_application",
            "description": "merge_strategy references a parameter that does not exist.",
            "parameter_schema": { "p": { "type": "number" } },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": { "base_metabolic_cost": 1 },
            "observable_signature": { "modality": "behavioral", "detection_range_m": 1, "pattern_key": "k" },
            "merge_strategy": { "ghost": "sum" },
            "provenance": "core"
        }"#;
        assert!(matches!(
            load_primitive_manifest(src),
            Err(PrimitiveLoadError::UnknownMergeStrategyParameter { .. })
        ));
    }

    #[test]
    fn semantic_default_type_mismatch_rejected() {
        let src = r#"{
            "id": "bad",
            "category": "force_application",
            "description": "Boolean default on a numeric parameter.",
            "parameter_schema": {
                "p": { "type": "number", "default": true }
            },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": { "base_metabolic_cost": 1 },
            "observable_signature": { "modality": "behavioral", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        assert!(matches!(
            load_primitive_manifest(src),
            Err(PrimitiveLoadError::InvalidDefault { .. })
        ));
    }
}
