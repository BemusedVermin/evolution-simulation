//! Malformed primitive manifests — each one must be rejected. Covers both
//! JSON-Schema-layer failures and semantic failures the schema cannot catch.

use beast_primitives::{PrimitiveLoadError, PrimitiveManifest};

const BASE_VALID: &str = r#"{
    "id": "example",
    "category": "signal_emission",
    "description": "Minimal baseline used by malformed-manifest fixtures.",
    "parameter_schema": {
        "frequency_hz": { "type": "number", "range": { "min": 20, "max": 1000 }, "default": 100 }
    },
    "composition_compatibility": [ { "channel_family": "motor" } ],
    "cost_function": {
        "base_metabolic_cost": 1.0,
        "parameter_scaling": [ { "parameter": "frequency_hz", "exponent": 1.0, "coefficient": 0.01 } ]
    },
    "observable_signature": {
        "modality": "acoustic",
        "detection_range_m": 10,
        "pattern_key": "example_v1"
    },
    "provenance": "core"
}"#;

#[test]
fn baseline_still_loads() {
    assert!(PrimitiveManifest::from_json_str(BASE_VALID).is_ok());
}

#[test]
fn fixture_1_unknown_category_rejected() {
    let src = BASE_VALID.replace(
        r#""category": "signal_emission""#,
        r#""category": "telepathy""#,
    );
    assert!(matches!(
        PrimitiveManifest::from_json_str(&src),
        Err(PrimitiveLoadError::SchemaViolation(_))
    ));
}

#[test]
fn fixture_2_unknown_modality_rejected() {
    let src = BASE_VALID.replace(r#""modality": "acoustic""#, r#""modality": "spirit""#);
    assert!(matches!(
        PrimitiveManifest::from_json_str(&src),
        Err(PrimitiveLoadError::SchemaViolation(_))
    ));
}

#[test]
fn fixture_3_missing_cost_function_rejected() {
    // Drop the entire `cost_function` field.
    let src = r#"{
        "id": "example",
        "category": "signal_emission",
        "description": "Missing cost_function.",
        "parameter_schema": { "p": { "type": "number" } },
        "composition_compatibility": [ { "channel_family": "motor" } ],
        "observable_signature": { "modality": "acoustic", "detection_range_m": 10, "pattern_key": "k" },
        "provenance": "core"
    }"#;
    assert!(matches!(
        PrimitiveManifest::from_json_str(src),
        Err(PrimitiveLoadError::SchemaViolation(_))
    ));
}

#[test]
fn fixture_4_scaling_references_unknown_parameter_rejected() {
    // Schema allows it (the referenced name is any string); semantic layer
    // rejects references to names that don't appear in `parameter_schema`.
    let src = BASE_VALID.replace(r#""parameter": "frequency_hz""#, r#""parameter": "ghost""#);
    assert!(matches!(
        PrimitiveManifest::from_json_str(&src),
        Err(PrimitiveLoadError::UnknownScalingParameter { .. })
    ));
}

#[test]
fn fixture_5_default_type_mismatch_rejected() {
    let src = BASE_VALID.replace(r#""default": 100"#, r#""default": "loud""#);
    // Semantic layer flags the mismatch between declared type=number and a
    // string default; schema otherwise allows any JSON type for `default`.
    assert!(matches!(
        PrimitiveManifest::from_json_str(&src),
        Err(PrimitiveLoadError::InvalidDefault { .. })
    ));
}

#[test]
fn fixture_6_invalid_provenance_rejected() {
    let src = BASE_VALID.replace(r#""provenance": "core""#, r#""provenance": "bogus:foo""#);
    assert!(matches!(
        PrimitiveManifest::from_json_str(&src),
        Err(PrimitiveLoadError::SchemaViolation(_))
    ));
}

#[test]
fn fixture_7_negative_detection_range_rejected() {
    let src = BASE_VALID.replace(r#""detection_range_m": 10"#, r#""detection_range_m": -5"#);
    assert!(matches!(
        PrimitiveManifest::from_json_str(&src),
        Err(PrimitiveLoadError::SchemaViolation(_))
    ));
}
