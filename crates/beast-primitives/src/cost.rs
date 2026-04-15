//! Deterministic cost-function evaluator.
//!
//! The cost of a single primitive emission is:
//!
//! ```text
//! cost = base_metabolic_cost + Σ coefficient_i · value_i ^ exponent_i
//! ```
//!
//! where the sum ranges over `parameter_scaling` entries and `value_i` is the
//! caller-supplied value for the corresponding parameter (falling back to the
//! parameter's declared `default` when absent). The power is computed in
//! fixed-point via [`crate::math::q_pow`], so results are bit-identical
//! across platforms.

use std::collections::BTreeMap;

use beast_core::Q3232;
use thiserror::Error;

use crate::manifest::{ParameterDefault, ParameterSpec, PrimitiveManifest};
use crate::math::q_pow;

/// Errors produced during cost evaluation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CostEvalError {
    /// A scaling term's parameter is missing from the caller-supplied map and
    /// the manifest declares no numeric default for it.
    #[error(
        "primitive `{primitive_id}` is missing a value for parameter `{parameter}` (no default)"
    )]
    MissingParameter {
        /// Primitive id from the manifest.
        primitive_id: String,
        /// Parameter name that has no resolvable value.
        parameter: String,
    },

    /// The parameter resolved to a non-numeric default (e.g. boolean) — the
    /// cost formula only makes sense for continuous / integer values.
    #[error("primitive `{primitive_id}` parameter `{parameter}` default is not numeric")]
    NonNumericDefault {
        /// Primitive id from the manifest.
        primitive_id: String,
        /// Parameter name.
        parameter: String,
    },

    /// `pow(base, exp)` was undefined for the supplied values (e.g. negative
    /// base with a fractional exponent).
    #[error(
        "primitive `{primitive_id}` parameter `{parameter}` produced an undefined power (base={base}, exp={exp})"
    )]
    UndefinedPower {
        /// Primitive id from the manifest.
        primitive_id: String,
        /// Parameter name.
        parameter: String,
        /// Base value (the parameter's runtime value).
        base: Q3232,
        /// Exponent from the manifest.
        exp: Q3232,
    },
}

/// Evaluate a primitive's cost function for a given parameter environment.
///
/// `params` maps parameter names to runtime values. Missing entries fall back
/// to the parameter's declared `default` (numeric only). Entries in `params`
/// that aren't referenced by the cost function are ignored, which keeps
/// callers free to pass a single shared environment across many primitives.
///
/// ```
/// use std::collections::BTreeMap;
/// use beast_core::Q3232;
/// use beast_primitives::{evaluate_cost, PrimitiveManifest};
///
/// let json = r#"{
///   "id": "bite",
///   "category": "force_application",
///   "description": "Simplified bite cost function for docs.",
///   "parameter_schema": {
///     "force": { "type": "number", "range": { "min": 0, "max": 1000 }, "default": 100 }
///   },
///   "composition_compatibility": [ { "channel_family": "motor" } ],
///   "cost_function": {
///     "base_metabolic_cost": 1.0,
///     "parameter_scaling": [
///       { "parameter": "force", "exponent": 2.0, "coefficient": 0.001 }
///     ]
///   },
///   "observable_signature": { "modality": "mechanical", "detection_range_m": 5, "pattern_key": "bite_v1" },
///   "provenance": "core"
/// }"#;
/// let manifest = PrimitiveManifest::from_json_str(json).unwrap();
///
/// // Using defaults: cost ≈ 1 + 0.001 * 100^2 = 11.
/// let cost = evaluate_cost(&manifest, &BTreeMap::new()).unwrap();
/// let as_f64: f64 = cost.to_num::<f64>();
/// assert!((as_f64 - 11.0).abs() < 1e-2);
/// ```
pub fn evaluate_cost(
    manifest: &PrimitiveManifest,
    params: &BTreeMap<String, Q3232>,
) -> Result<Q3232, CostEvalError> {
    let mut total = manifest.cost_function.base_metabolic_cost;
    for scaling in &manifest.cost_function.parameter_scaling {
        let value = resolve_value(manifest, &scaling.parameter, params)?;
        let term = match q_pow(value, scaling.exponent) {
            Some(v) => scaling.coefficient * v,
            None => {
                return Err(CostEvalError::UndefinedPower {
                    primitive_id: manifest.id.clone(),
                    parameter: scaling.parameter.clone(),
                    base: value,
                    exp: scaling.exponent,
                });
            }
        };
        total += term;
    }
    Ok(total)
}

fn resolve_value(
    manifest: &PrimitiveManifest,
    parameter: &str,
    params: &BTreeMap<String, Q3232>,
) -> Result<Q3232, CostEvalError> {
    if let Some(v) = params.get(parameter) {
        return Ok(*v);
    }
    // Load-time validation in `RawPrimitiveManifest::into_manifest` rejects
    // scaling entries that name unknown parameters, so in the normal flow
    // this lookup always succeeds. `PrimitiveManifest`'s fields are public,
    // though, so tests and future code can hand-construct manifests that
    // bypass the loader — surface that as `MissingParameter` rather than
    // panicking out of a cost evaluation call.
    let spec: &ParameterSpec = manifest.parameter_schema.get(parameter).ok_or_else(|| {
        CostEvalError::MissingParameter {
            primitive_id: manifest.id.clone(),
            parameter: parameter.to_owned(),
        }
    })?;
    match &spec.default {
        Some(ParameterDefault::Number(v)) => Ok(*v),
        Some(ParameterDefault::Integer(i)) => Ok(Q3232::from_num(*i)),
        Some(ParameterDefault::String(_)) | Some(ParameterDefault::Boolean(_)) => {
            Err(CostEvalError::NonNumericDefault {
                primitive_id: manifest.id.clone(),
                parameter: parameter.to_owned(),
            })
        }
        None => Err(CostEvalError::MissingParameter {
            primitive_id: manifest.id.clone(),
            parameter: parameter.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrimitiveManifest;

    #[test]
    fn base_cost_only() {
        let json = r#"{
            "id": "p",
            "category": "signal_emission",
            "description": "Only a base cost.",
            "parameter_schema": { "x": { "type": "number", "default": 1 } },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": { "base_metabolic_cost": 3.5 },
            "observable_signature": { "modality": "acoustic", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        let m = PrimitiveManifest::from_json_str(json).unwrap();
        let cost = evaluate_cost(&m, &BTreeMap::new()).unwrap();
        assert_eq!(cost, Q3232::from_num(3.5_f64));
    }

    #[test]
    fn uses_default_when_missing() {
        let json = r#"{
            "id": "p",
            "category": "signal_emission",
            "description": "Default used because params is empty.",
            "parameter_schema": {
                "force": { "type": "number", "default": 10 }
            },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": {
                "base_metabolic_cost": 0,
                "parameter_scaling": [
                    { "parameter": "force", "exponent": 2.0, "coefficient": 1.0 }
                ]
            },
            "observable_signature": { "modality": "acoustic", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        let m = PrimitiveManifest::from_json_str(json).unwrap();
        let cost = evaluate_cost(&m, &BTreeMap::new()).unwrap();
        let f: f64 = cost.to_num::<f64>();
        assert!((f - 100.0).abs() < 1e-3);
    }

    #[test]
    fn uses_caller_value_overriding_default() {
        let json = r#"{
            "id": "p",
            "category": "signal_emission",
            "description": "Caller overrides default.",
            "parameter_schema": {
                "force": { "type": "number", "default": 10 }
            },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": {
                "base_metabolic_cost": 0,
                "parameter_scaling": [
                    { "parameter": "force", "exponent": 1.5, "coefficient": 1.0 }
                ]
            },
            "observable_signature": { "modality": "acoustic", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        let m = PrimitiveManifest::from_json_str(json).unwrap();
        let mut params = BTreeMap::new();
        params.insert("force".into(), Q3232::from_num(4_i32));
        let cost = evaluate_cost(&m, &params).unwrap();
        let f: f64 = cost.to_num::<f64>();
        // 4^1.5 = 8
        assert!((f - 8.0).abs() < 1e-2);
    }

    #[test]
    fn missing_parameter_without_default_errors() {
        let json = r#"{
            "id": "p",
            "category": "signal_emission",
            "description": "No default for the parameter.",
            "parameter_schema": { "force": { "type": "number" } },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": {
                "base_metabolic_cost": 0,
                "parameter_scaling": [
                    { "parameter": "force", "exponent": 1.0, "coefficient": 1.0 }
                ]
            },
            "observable_signature": { "modality": "acoustic", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        let m = PrimitiveManifest::from_json_str(json).unwrap();
        assert!(matches!(
            evaluate_cost(&m, &BTreeMap::new()),
            Err(CostEvalError::MissingParameter { .. })
        ));
    }

    #[test]
    fn hand_constructed_manifest_missing_param_is_reported_not_panicked() {
        // PrimitiveManifest's fields are `pub`, so callers can bypass the
        // loader's validation. This test mutates a loaded manifest so its
        // cost function references a parameter absent from the schema —
        // the path that used to panic via .expect() — and asserts we
        // surface MissingParameter instead.
        use crate::manifest::ParameterScaling;
        let json = r#"{
            "id": "p",
            "category": "signal_emission",
            "description": "Normal manifest; tampered with post-load.",
            "parameter_schema": { "x": { "type": "number", "default": 1 } },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": {
                "base_metabolic_cost": 0,
                "parameter_scaling": [
                    { "parameter": "x", "exponent": 1.0, "coefficient": 1.0 }
                ]
            },
            "observable_signature": { "modality": "acoustic", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        let mut m = PrimitiveManifest::from_json_str(json).unwrap();
        // Tamper: point the scaling term at a name that was never declared.
        m.cost_function.parameter_scaling.push(ParameterScaling {
            parameter: "ghost".into(),
            exponent: Q3232::ONE,
            coefficient: Q3232::ONE,
        });
        match evaluate_cost(&m, &BTreeMap::new()) {
            Err(CostEvalError::MissingParameter { parameter, .. }) => {
                assert_eq!(parameter, "ghost");
            }
            other => panic!("expected MissingParameter, got {other:?}"),
        }
    }

    #[test]
    fn deterministic_across_calls() {
        let json = r#"{
            "id": "p",
            "category": "signal_emission",
            "description": "Deterministic cost across calls.",
            "parameter_schema": {
                "force": { "type": "number", "default": 100 },
                "duration": { "type": "number", "default": 50 }
            },
            "composition_compatibility": [ { "channel_family": "motor" } ],
            "cost_function": {
                "base_metabolic_cost": 1,
                "parameter_scaling": [
                    { "parameter": "force", "exponent": 1.5, "coefficient": 0.01 },
                    { "parameter": "duration", "exponent": 1.0, "coefficient": 0.05 }
                ]
            },
            "observable_signature": { "modality": "acoustic", "detection_range_m": 1, "pattern_key": "k" },
            "provenance": "core"
        }"#;
        let m = PrimitiveManifest::from_json_str(json).unwrap();
        let first = evaluate_cost(&m, &BTreeMap::new()).unwrap();
        for _ in 0..100 {
            assert_eq!(evaluate_cost(&m, &BTreeMap::new()).unwrap(), first);
        }
    }
}
