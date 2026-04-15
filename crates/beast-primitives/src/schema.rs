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

use std::sync::OnceLock;

use jsonschema::JSONSchema;
use thiserror::Error;

use crate::manifest::PrimitiveManifest;

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
    #[error("manifest failed schema validation:\n{}", format_schema_errors(.0))]
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

/// A single JSON Schema validation error, flattened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolation {
    /// JSON Pointer path to the failing node.
    pub pointer: String,
    /// Human-readable message from the validator.
    pub message: String,
}

fn format_schema_errors(errors: &[SchemaViolation]) -> String {
    let mut out = String::new();
    for e in errors {
        out.push_str("  at ");
        out.push_str(if e.pointer.is_empty() {
            "<root>"
        } else {
            e.pointer.as_str()
        });
        out.push_str(": ");
        out.push_str(&e.message);
        out.push('\n');
    }
    out
}

fn compiled_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let raw: serde_json::Value = serde_json::from_str(PRIMITIVE_MANIFEST_SCHEMA)
            .expect("embedded primitive manifest schema is valid JSON");
        JSONSchema::options()
            .compile(&raw)
            .expect("embedded primitive manifest schema compiles")
    })
}

/// Load and fully validate a primitive manifest from its JSON source.
pub fn load_primitive_manifest(source: &str) -> Result<PrimitiveManifest, PrimitiveLoadError> {
    let value: serde_json::Value =
        serde_json::from_str(source).map_err(|e| PrimitiveLoadError::InvalidJson(e.to_string()))?;

    let schema = compiled_schema();
    if let Err(errors) = schema.validate(&value) {
        let violations = errors
            .map(|e| SchemaViolation {
                pointer: e.instance_path.to_string(),
                message: e.to_string(),
            })
            .collect::<Vec<_>>();
        return Err(PrimitiveLoadError::SchemaViolation(violations));
    }

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
