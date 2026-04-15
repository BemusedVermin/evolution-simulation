//! Five malformed channel manifests — each one must be rejected with a
//! specific, descriptive error so mod authors can diagnose their own JSON.
//!
//! Sprint 2 demo criterion: "Schema validation rejects 5 malformed test
//! manifests."

use beast_channels::{ChannelLoadError, ChannelManifest};

/// Helper: a baseline minimal-valid manifest that each fixture tweaks in
/// exactly one way to isolate the failure mode under test.
const BASE_VALID: &str = r#"{
    "id": "example",
    "family": "sensory",
    "description": "Minimal baseline used by malformed-manifest fixtures.",
    "range": { "min": 0, "max": 1, "units": "dimensionless" },
    "mutation_kernel": {
        "sigma": 0.1,
        "bounds_policy": "clamp",
        "genesis_weight": 1.0
    },
    "composition_hooks": [],
    "expression_conditions": [],
    "scale_band": { "min_kg": 0.01, "max_kg": 1000 },
    "body_site_applicable": true,
    "provenance": "core"
}"#;

#[test]
fn baseline_still_loads() {
    // Safety net: if the baseline fixture ever stops loading, every other
    // test in this file is meaningless.
    assert!(ChannelManifest::from_json_str(BASE_VALID).is_ok());
}

#[test]
fn fixture_1_unknown_family_rejected() {
    let src = BASE_VALID.replace(r#""family": "sensory""#, r#""family": "telepathy""#);
    let err = ChannelManifest::from_json_str(&src).unwrap_err();
    assert!(
        matches!(err, ChannelLoadError::SchemaViolation(_)),
        "expected schema violation, got {err:?}"
    );
}

#[test]
fn fixture_2_missing_required_field_rejected() {
    // Drop `scale_band` entirely.
    let src = r#"{
        "id": "example",
        "family": "sensory",
        "description": "Missing scale_band.",
        "range": { "min": 0, "max": 1, "units": "dimensionless" },
        "mutation_kernel": { "sigma": 0.1, "bounds_policy": "clamp", "genesis_weight": 1.0 },
        "composition_hooks": [],
        "expression_conditions": [],
        "body_site_applicable": true,
        "provenance": "core"
    }"#;
    let err = ChannelManifest::from_json_str(src).unwrap_err();
    assert!(matches!(err, ChannelLoadError::SchemaViolation(_)));
}

#[test]
fn fixture_3_threshold_kind_without_threshold_value_rejected() {
    let src = BASE_VALID.replace(
        r#""composition_hooks": []"#,
        r#""composition_hooks": [ { "with": "other", "kind": "threshold", "coefficient": 1.0 } ]"#,
    );
    let err = ChannelManifest::from_json_str(&src).unwrap_err();
    assert!(
        matches!(
            err,
            ChannelLoadError::SchemaViolation(_) | ChannelLoadError::MissingThreshold { .. }
        ),
        "expected schema-violation or missing-threshold, got {err:?}"
    );
}

#[test]
fn fixture_4_inverted_scale_band_rejected() {
    // Schema allows any numeric; semantic layer rejects min > max.
    let src = BASE_VALID.replace(
        r#""scale_band": { "min_kg": 0.01, "max_kg": 1000 }"#,
        r#""scale_band": { "min_kg": 1000, "max_kg": 0.01 }"#,
    );
    let err = ChannelManifest::from_json_str(&src).unwrap_err();
    assert!(matches!(err, ChannelLoadError::InvalidRange { .. }));
}

#[test]
fn fixture_5_invalid_provenance_pattern_rejected() {
    let src = BASE_VALID.replace(r#""provenance": "core""#, r#""provenance": "random:value""#);
    let err = ChannelManifest::from_json_str(&src).unwrap_err();
    // The JSON Schema pattern catches this before the semantic layer runs.
    assert!(matches!(err, ChannelLoadError::SchemaViolation(_)));
}

#[test]
fn fixture_6_sigma_zero_rejected() {
    // Schema requires exclusiveMinimum: 0 for sigma.
    let src = BASE_VALID.replace(r#""sigma": 0.1"#, r#""sigma": 0"#);
    let err = ChannelManifest::from_json_str(&src).unwrap_err();
    assert!(matches!(err, ChannelLoadError::SchemaViolation(_)));
}

#[test]
fn fixture_7_correlation_out_of_bounds_rejected() {
    let src = BASE_VALID.replace(
        r#""genesis_weight": 1.0"#,
        r#""genesis_weight": 1.0, "correlation_with": [{ "channel": "other", "coefficient": 1.5 }]"#,
    );
    let err = ChannelManifest::from_json_str(&src).unwrap_err();
    assert!(matches!(err, ChannelLoadError::SchemaViolation(_)));
}
