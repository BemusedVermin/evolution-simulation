//! JSON Schema validation primitives shared by every manifest loader.
//!
//! The [`CompiledSchema`] wrapper owns a `jsonschema::Validator` compiled once
//! from a static schema string. Call [`CompiledSchema::load`] to run the
//! two-stage pipeline (parse → validate) and get back a `serde_json::Value`
//! the caller can then deserialize into a domain type.
//!
//! Validation errors are flattened into [`SchemaViolation`] records — a
//! JSON-Pointer path plus the `jsonschema` message — so downstream error
//! enums don't have to leak `jsonschema` types into their public API.

use jsonschema::Validator;
use thiserror::Error;

/// A single JSON Schema validation error, flattened to a simple
/// `(pointer, message)` pair so callers don't need to depend on `jsonschema`
/// types directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolation {
    /// JSON Pointer path to the failing node (e.g.
    /// `/composition_hooks/0/threshold`). Empty string means the root.
    pub pointer: String,
    /// Human-readable message from the validator.
    pub message: String,
}

/// Format a slice of [`SchemaViolation`] for embedding in error messages.
///
/// The output has one line per violation: `  at <pointer>: <message>\n`.
/// An empty `pointer` is rendered as `<root>`.
#[must_use]
pub fn format_schema_errors(errors: &[SchemaViolation]) -> String {
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

/// Errors returned by the generic two-stage load pipeline.
///
/// Domain crates wrap this in their own error enum with additional
/// semantic-validation variants (invalid range ordering, duplicate hook,
/// etc.). [`From`] is implemented manually by each consumer so the variants
/// flow through without loss.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SchemaLoadError {
    /// Input was not parsable as JSON.
    #[error("manifest is not valid JSON: {0}")]
    InvalidJson(String),

    /// JSON Schema validation produced one or more errors.
    #[error("manifest failed schema validation:\n{}", format_schema_errors(.0))]
    SchemaViolation(Vec<SchemaViolation>),
}

/// A pre-compiled JSON Schema validator.
///
/// Compile once (typically inside a `std::sync::OnceLock`) and reuse for
/// every manifest load. The wrapper auto-selects the draft from the
/// schema's `$schema` URI — the schemas in this workspace declare Draft
/// 2020-12.
pub struct CompiledSchema {
    validator: Validator,
}

impl CompiledSchema {
    /// Compile the given schema source text.
    ///
    /// # Panics
    ///
    /// Panics if `schema_source` is not valid JSON or does not compile to a
    /// valid JSON Schema. Intended for embedded (`include_str!`) schema
    /// text: a compile failure is a bug in the shipped schema, not a
    /// runtime condition to recover from.
    #[must_use]
    pub fn compile(schema_source: &str) -> Self {
        let raw: serde_json::Value =
            serde_json::from_str(schema_source).expect("embedded schema is valid JSON");
        let validator = jsonschema::validator_for(&raw).expect("embedded schema compiles");
        Self { validator }
    }

    /// Collect every validation error for `value`, flattened to
    /// [`SchemaViolation`].
    ///
    /// An empty return value indicates the document passed validation.
    #[must_use]
    pub fn validate(&self, value: &serde_json::Value) -> Vec<SchemaViolation> {
        self.validator
            .iter_errors(value)
            .map(|e| SchemaViolation {
                pointer: e.instance_path().to_string(),
                message: e.to_string(),
            })
            .collect()
    }

    /// Run the full two-stage load: parse `source` as JSON, then validate
    /// against the schema.
    ///
    /// Returns the parsed `serde_json::Value` on success so the caller can
    /// run its own semantic layer (typed deserialization, cross-field
    /// checks, etc.) without reparsing.
    pub fn load(&self, source: &str) -> Result<serde_json::Value, SchemaLoadError> {
        let value: serde_json::Value = serde_json::from_str(source)
            .map_err(|e| SchemaLoadError::InvalidJson(e.to_string()))?;
        let violations = self.validate(&value);
        if !violations.is_empty() {
            return Err(SchemaLoadError::SchemaViolation(violations));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI_SCHEMA: &str = r#"{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["id", "count"],
        "properties": {
            "id": { "type": "string", "pattern": "^[a-z_]+$" },
            "count": { "type": "integer", "minimum": 0 }
        },
        "additionalProperties": false
    }"#;

    #[test]
    fn compile_accepts_valid_schema() {
        let _schema = CompiledSchema::compile(MINI_SCHEMA);
    }

    #[test]
    fn load_accepts_valid_document() {
        let schema = CompiledSchema::compile(MINI_SCHEMA);
        let value = schema.load(r#"{"id": "foo", "count": 3}"#).unwrap();
        assert_eq!(value["id"], "foo");
    }

    #[test]
    fn load_rejects_invalid_json() {
        let schema = CompiledSchema::compile(MINI_SCHEMA);
        assert!(matches!(
            schema.load("not json"),
            Err(SchemaLoadError::InvalidJson(_))
        ));
    }

    #[test]
    fn load_reports_schema_violations() {
        let schema = CompiledSchema::compile(MINI_SCHEMA);
        let err = schema.load(r#"{"id": "UPPER", "count": -1}"#).unwrap_err();
        match err {
            SchemaLoadError::SchemaViolation(violations) => {
                assert!(!violations.is_empty());
                // Every violation must carry a non-empty message.
                assert!(violations.iter().all(|v| !v.message.is_empty()));
            }
            other => panic!("expected SchemaViolation, got {other:?}"),
        }
    }

    #[test]
    fn format_schema_errors_renders_root_and_path() {
        let lines = format_schema_errors(&[
            SchemaViolation {
                pointer: String::new(),
                message: "missing required field".to_owned(),
            },
            SchemaViolation {
                pointer: "/count".to_owned(),
                message: "must be >= 0".to_owned(),
            },
        ]);
        assert!(lines.contains("  at <root>: missing required field\n"));
        assert!(lines.contains("  at /count: must be >= 0\n"));
    }

    #[test]
    fn validate_reports_empty_when_document_matches() {
        let schema = CompiledSchema::compile(MINI_SCHEMA);
        let value: serde_json::Value =
            serde_json::from_str(r#"{"id": "foo", "count": 0}"#).unwrap();
        assert!(schema.validate(&value).is_empty());
    }
}
