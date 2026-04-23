//! JSON Schema validation for channel manifests.
//!
//! The canonical schema lives in
//! `documentation/schemas/channel_manifest.schema.json`. It is the single
//! source of truth; we embed it into the binary via [`include_str!`] so every
//! validator in the repository agrees, and so there is no runtime file-system
//! dependency.
//!
//! Validation is **two-stage**:
//!
//! 1. Parse the input as `serde_json::Value` and run the JSON Schema
//!    validator via [`beast_manifest::CompiledSchema`]. This catches shape
//!    errors (missing fields, wrong types, pattern failures, conditional
//!    `if/then` requirements like `threshold` being required for
//!    `kind ∈ {threshold, gating}`, etc.) with well-formed pointer-style
//!    paths.
//! 2. Deserialize into [`crate::manifest::ChannelManifest`] and run
//!    cross-field semantic checks (`range.min <= range.max`, unique
//!    composition targets, provenance string decomposition).
//!
//! External callers should use
//! [`crate::manifest::ChannelManifest::from_json_str`]; this module is the
//! implementation.

use std::sync::OnceLock;

use beast_manifest::{CompiledSchema, ProvenanceParseError, SchemaLoadError};
use thiserror::Error;

use crate::composition::CompositionKind;
use crate::manifest::ChannelManifest;

/// Re-export of the shared violation record — kept here so downstream
/// callers can continue to `use beast_channels::SchemaViolation`.
pub use beast_manifest::SchemaViolation;

/// Raw JSON text of the channel manifest schema. Embedded at compile time so
/// downstream crates need no runtime filesystem access to validate.
pub const CHANNEL_MANIFEST_SCHEMA: &str =
    include_str!("../../../documentation/schemas/channel_manifest.schema.json");

/// Errors produced while loading a channel manifest.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ChannelLoadError {
    /// Input was not parsable as JSON.
    #[error("manifest is not valid JSON: {0}")]
    InvalidJson(String),

    /// JSON Schema validation produced one or more errors. Each entry carries
    /// the failing JSON Pointer path plus a human-readable message, which is
    /// enough to point a mod author at the exact field.
    #[error("manifest failed schema validation:\n{}", beast_manifest::format_schema_errors(.0))]
    SchemaViolation(Vec<SchemaViolation>),

    /// Deserialization into the typed manifest struct failed after schema
    /// validation passed — indicates a gap between the schema and the Rust
    /// types (treat as a bug).
    #[error("manifest deserialized unexpectedly: {0}")]
    BadShape(String),

    /// `provenance` did not match one of `core`, `mod:<id>`, or
    /// `genesis:<parent>:<generation>`.
    #[error("invalid provenance string: {0}")]
    InvalidProvenance(String),

    /// A numeric range was inverted (e.g. `range.min > range.max`).
    #[error("channel {channel_id} has invalid range: {reason}")]
    InvalidRange {
        /// Channel id from the manifest.
        channel_id: String,
        /// Human description of which range inverted.
        reason: String,
    },

    /// Two composition hooks referenced the same other channel.
    #[error("channel {channel_id} has duplicate composition hook for `{with}`")]
    DuplicateHook {
        /// Channel id from the manifest.
        channel_id: String,
        /// Repeated target id.
        with: String,
    },

    /// A composition hook of `threshold`/`gating` kind omitted `threshold`.
    #[error(
        "channel {channel_id} composition hook `with={with}` kind={kind:?} is missing the required `threshold`"
    )]
    MissingThreshold {
        /// Channel id from the manifest.
        channel_id: String,
        /// Target id for the offending hook.
        with: String,
        /// Offending composition kind.
        kind: CompositionKind,
    },
}

impl From<SchemaLoadError> for ChannelLoadError {
    fn from(e: SchemaLoadError) -> Self {
        match e {
            SchemaLoadError::InvalidJson(s) => Self::InvalidJson(s),
            SchemaLoadError::SchemaViolation(v) => Self::SchemaViolation(v),
        }
    }
}

impl From<ProvenanceParseError> for ChannelLoadError {
    fn from(e: ProvenanceParseError) -> Self {
        Self::InvalidProvenance(e.0)
    }
}

fn compiled_schema() -> &'static CompiledSchema {
    static SCHEMA: OnceLock<CompiledSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| CompiledSchema::compile(CHANNEL_MANIFEST_SCHEMA))
}

/// Load and fully validate a channel manifest from its JSON source.
///
/// See [`crate::manifest::ChannelManifest::from_json_str`] for the public
/// entry point — it forwards here.
pub fn load_channel_manifest(source: &str) -> Result<ChannelManifest, ChannelLoadError> {
    let value = compiled_schema().load(source)?;
    ChannelManifest::from_validated_value(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_schema_compiles() {
        // Accessing `compiled_schema()` panics if the embedded string is ill-
        // formed or the schema itself is malformed. Pulling it once exercises
        // the OnceLock.
        let _ = compiled_schema();
    }

    #[test]
    fn minimal_core_manifest_loads() {
        let src = r#"{
            "id": "example",
            "family": "metabolic",
            "description": "A minimal valid manifest for testing.",
            "range": { "min": 0, "max": 1, "units": "dimensionless" },
            "mutation_kernel": {
                "sigma": 0.1,
                "bounds_policy": "clamp",
                "genesis_weight": 1.0
            },
            "composition_hooks": [],
            "expression_conditions": [],
            "scale_band": { "min_kg": 0.01, "max_kg": 1000 },
            "body_site_applicable": false,
            "provenance": "core"
        }"#;
        let m = load_channel_manifest(src).unwrap();
        assert_eq!(m.id, "example");
    }

    #[test]
    fn schema_rejects_missing_required_field() {
        // `family` omitted.
        let src = r#"{
            "id": "example",
            "description": "missing family",
            "range": { "min": 0, "max": 1, "units": "dimensionless" },
            "mutation_kernel": { "sigma": 0.1, "bounds_policy": "clamp", "genesis_weight": 1.0 },
            "composition_hooks": [],
            "expression_conditions": [],
            "scale_band": { "min_kg": 0.01, "max_kg": 1000 },
            "body_site_applicable": false,
            "provenance": "core"
        }"#;
        let err = load_channel_manifest(src).unwrap_err();
        assert!(matches!(err, ChannelLoadError::SchemaViolation(_)));
    }

    #[test]
    fn semantic_invalid_range_rejected() {
        // Schema allows min=5, max=1 (it checks types, not ordering); semantic
        // layer catches the inversion.
        let src = r#"{
            "id": "example",
            "family": "metabolic",
            "description": "An inverted numeric range.",
            "range": { "min": 5, "max": 1, "units": "dimensionless" },
            "mutation_kernel": { "sigma": 0.1, "bounds_policy": "clamp", "genesis_weight": 1.0 },
            "composition_hooks": [],
            "expression_conditions": [],
            "scale_band": { "min_kg": 0.01, "max_kg": 1000 },
            "body_site_applicable": false,
            "provenance": "core"
        }"#;
        assert!(matches!(
            load_channel_manifest(src),
            Err(ChannelLoadError::InvalidRange { .. })
        ));
    }

    #[test]
    fn threshold_missing_rejected_by_schema() {
        let src = r#"{
            "id": "example",
            "family": "sensory",
            "description": "Threshold kind without threshold field.",
            "range": { "min": 0, "max": 1, "units": "dimensionless" },
            "mutation_kernel": { "sigma": 0.1, "bounds_policy": "clamp", "genesis_weight": 1.0 },
            "composition_hooks": [
                { "with": "other", "kind": "threshold", "coefficient": 1.0 }
            ],
            "expression_conditions": [],
            "scale_band": { "min_kg": 0.01, "max_kg": 1000 },
            "body_site_applicable": false,
            "provenance": "core"
        }"#;
        let err = load_channel_manifest(src).unwrap_err();
        // Either the JSON Schema conditional catches it, or the semantic layer
        // does — both are acceptable.
        assert!(matches!(
            err,
            ChannelLoadError::SchemaViolation(_) | ChannelLoadError::MissingThreshold { .. }
        ));
    }

    #[test]
    fn duplicate_composition_hook_rejected() {
        let src = r#"{
            "id": "example",
            "family": "sensory",
            "description": "Two hooks referencing the same other channel.",
            "range": { "min": 0, "max": 1, "units": "dimensionless" },
            "mutation_kernel": { "sigma": 0.1, "bounds_policy": "clamp", "genesis_weight": 1.0 },
            "composition_hooks": [
                { "with": "other", "kind": "additive", "coefficient": 1.0 },
                { "with": "other", "kind": "multiplicative", "coefficient": 0.5 }
            ],
            "expression_conditions": [],
            "scale_band": { "min_kg": 0.01, "max_kg": 1000 },
            "body_site_applicable": false,
            "provenance": "core"
        }"#;
        assert!(matches!(
            load_channel_manifest(src),
            Err(ChannelLoadError::DuplicateHook { .. })
        ));
    }

    #[test]
    fn invalid_provenance_rejected_by_pattern() {
        let src = r#"{
            "id": "example",
            "family": "sensory",
            "description": "Unknown provenance prefix.",
            "range": { "min": 0, "max": 1, "units": "dimensionless" },
            "mutation_kernel": { "sigma": 0.1, "bounds_policy": "clamp", "genesis_weight": 1.0 },
            "composition_hooks": [],
            "expression_conditions": [],
            "scale_band": { "min_kg": 0.01, "max_kg": 1000 },
            "body_site_applicable": false,
            "provenance": "unknown:foo"
        }"#;
        let err = load_channel_manifest(src).unwrap_err();
        assert!(matches!(err, ChannelLoadError::SchemaViolation(_)));
    }
}
