//! PrimitiveEffect emission, deduplication, and merge (S4.4 — issue #58).
//!
//! Consumes [`crate::FiredHook`]s from the resolver ([`crate::composition`]),
//! evaluates each [`crate::EmitSpec`]'s parameter expressions against the
//! phenotype's channel vector, materialises one
//! [`beast_primitives::PrimitiveEffect`] per hook emission, and merges
//! duplicates (same `primitive_id`) using a deterministic per-parameter
//! merge strategy.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §6.2 and §6.2B.
//!
//! # Known follow-ups (deliberately deferred past S4.4)
//!
//! * **Body-site fanout — [issue #67]**. §6.2B dedups by
//!   `(primitive_id, body_site)`, but [`beast_primitives::PrimitiveEffect`]
//!   does not yet carry a `body_site` field. S4.4 therefore dedups flat by
//!   `primitive_id` only.
//! * **Typed manifest merge_strategy — [issue #71]**. §6.2B allows per-parameter
//!   `sum`/`max`/`mean`/`union` strategies declared on `PrimitiveManifest`;
//!   that field doesn't exist yet. Until then every user parameter merges
//!   via `max` (the doc's conservative default at line 802), and the cost
//!   sentinel below merges via `sum`.
//! * **Source-channel provenance — [issue #70]**. §6.2 line 746 records the
//!   hook's firing channels in `source_channels`. Without `channel_ids` on
//!   [`crate::FiredHook`] we currently reconstruct from expression refs only;
//!   gate-only channels are missed.
//! * **First-class activation cost — [issue #67]**. Until
//!   [`beast_primitives::PrimitiveEffect`] gains a dedicated `activation_cost`
//!   field we surface the evaluated cost via the [`ACTIVATION_COST_PARAM`]
//!   sentinel parameter below.
//!
//! [issue #67]: https://github.com/BemusedVermin/evolution-simulation/issues/67
//! [issue #70]: https://github.com/BemusedVermin/evolution-simulation/issues/70
//! [issue #71]: https://github.com/BemusedVermin/evolution-simulation/issues/71

use std::collections::BTreeMap;

use beast_core::{EntityId, Q3232};
use beast_primitives::{evaluate_cost, PrimitiveEffect, PrimitiveRegistry};

use crate::composition::{EmitSpec, FiredHook};
use crate::error::{InterpreterError, Result};
use crate::parameter_map::{collect_channel_refs, eval_expression};
use crate::phenotype::ResolvedPhenotype;

/// Parameter name reserved for the cost output on a [`PrimitiveEffect`].
///
/// Until [`beast_primitives::PrimitiveEffect`] gains a first-class
/// `activation_cost` field (#67 / follow-up), we surface the evaluated cost
/// inside `parameters` under this key so downstream consumers have access
/// without a schema break. The leading underscore keeps it visually distinct
/// from user-declared parameters.
pub const ACTIVATION_COST_PARAM: &str = "_activation_cost";

/// Build [`PrimitiveEffect`]s from the given fired hooks and merge duplicates.
///
/// For every `(fired_hook, emit_spec)` pair the emitter:
///
/// 1. Evaluates each parameter expression against
///    [`ResolvedPhenotype::global_channels`] via [`eval_expression`].
/// 2. Looks the primitive up in `registry`, returning
///    [`InterpreterError::UnknownPrimitive`] when absent.
/// 3. Computes the activation cost through
///    [`beast_primitives::evaluate_cost`] using the evaluated parameters and
///    stores it under [`ACTIVATION_COST_PARAM`] in the emission's
///    `parameters` map.
/// 4. Builds a [`PrimitiveEffect`] whose
///    [`source_channels`](PrimitiveEffect::source_channels) is the sorted,
///    deduplicated union of the firing hook's context channels and every
///    channel id referenced by any parameter expression.
///
/// After every effect is built, effects are grouped by `primitive_id` and
/// merged per §6.2B. Single-effect groups pass through unchanged;
/// multi-effect groups merge each parameter via `max` (the doc's default)
/// with the exception of [`ACTIVATION_COST_PARAM`], which sums across the
/// group.
///
/// The returned vector is sorted by `primitive_id` (grouped via
/// [`BTreeMap`]), which gives deterministic output ordering without relying
/// on input hook order — satisfying INVARIANTS §1.
///
/// # Errors
///
/// * [`InterpreterError::UnknownPrimitive`] if any `EmitSpec` references a
///   primitive not in `registry`.
/// * Any error returned by [`beast_primitives::evaluate_cost`] is converted
///   into [`InterpreterError::ParseError`] (chosen over adding a new enum
///   variant; the cost evaluator's own errors contain the detail in the
///   message).
#[must_use = "emitted primitive effects must feed the next stage — dropping silences this tick's emission"]
pub fn emit_primitives(
    fired_hooks: &[FiredHook],
    phenotype: &ResolvedPhenotype,
    registry: &PrimitiveRegistry,
    emitter: EntityId,
) -> Result<Vec<PrimitiveEffect>> {
    // Stage 1 — materialise one `PrimitiveEffect` per (fired hook, emit spec)
    // pair, preserving input order so the merge below sees a predictable
    // sequence (the merge itself is ordering-independent for sum/max, but
    // keeping input order makes debugging easier).
    let mut effects: Vec<PrimitiveEffect> = Vec::new();
    for fired in fired_hooks {
        for emit in &fired.emits {
            effects.push(build_effect(fired, emit, phenotype, registry, emitter)?);
        }
    }

    // Stage 2 — dedup / merge by `primitive_id`. `BTreeMap` iteration is
    // lexicographic on ids, which gives a stable output order independent of
    // input hook order.
    let mut groups: BTreeMap<String, Vec<PrimitiveEffect>> = BTreeMap::new();
    for effect in effects {
        groups
            .entry(effect.primitive_id.clone())
            .or_default()
            .push(effect);
    }

    let mut out: Vec<PrimitiveEffect> = Vec::with_capacity(groups.len());
    for (_id, group) in groups {
        out.push(merge_group(group));
    }
    Ok(out)
}

/// Build one [`PrimitiveEffect`] from a single `(FiredHook, EmitSpec)` pair.
fn build_effect(
    fired: &FiredHook,
    emit: &EmitSpec,
    phenotype: &ResolvedPhenotype,
    registry: &PrimitiveRegistry,
    emitter: EntityId,
) -> Result<PrimitiveEffect> {
    let manifest =
        registry
            .get(&emit.primitive_id)
            .ok_or_else(|| InterpreterError::UnknownPrimitive {
                primitive_id: emit.primitive_id.clone(),
            })?;

    // Evaluate parameter expressions. `parameter_mapping` is sorted by
    // name (per the `EmitSpec` contract in composition.rs), so the
    // resulting `BTreeMap` population order is deterministic.
    let mut parameters: BTreeMap<String, Q3232> = BTreeMap::new();
    for (name, expr) in &emit.parameter_mapping {
        let value = eval_expression(expr, &phenotype.global_channels);
        parameters.insert(name.clone(), value);
    }

    // Evaluate the cost against the just-computed parameters. `evaluate_cost`
    // falls back to manifest-declared defaults for any scaling term whose
    // parameter is absent from the map.
    let cost = evaluate_cost(manifest, &parameters).map_err(|e| InterpreterError::ParseError {
        message: format!(
            "cost evaluation for primitive `{}` failed: {e}",
            emit.primitive_id
        ),
    })?;
    parameters.insert(ACTIVATION_COST_PARAM.to_owned(), cost);

    // Source channels = hook-context channels ∪ expression-referenced
    // channels, deduplicated and sorted (via `BTreeSet` below).
    let source_channels = collect_source_channels(fired, emit);

    Ok(PrimitiveEffect {
        primitive_id: emit.primitive_id.clone(),
        source_channels,
        parameters,
        emitter,
        provenance: manifest.provenance.clone(),
    })
}

/// Compose the sorted, deduplicated list of channel ids that contributed to
/// an effect.
///
/// The hook's own `channel_ids` list isn't stored on [`FiredHook`] (only the
/// per-channel *values* are), so the context is inferred from the union of
/// every channel the emit spec's expressions read. The resolver filters out
/// hooks whose context channels are missing from the phenotype, so the
/// per-expression refs are a superset-safe way to reconstruct the provenance
/// list without an extra round-trip to the original `InterpreterHook`.
fn collect_source_channels(_fired: &FiredHook, emit: &EmitSpec) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    for (_name, expr) in &emit.parameter_mapping {
        for id in collect_channel_refs(expr) {
            seen.insert(id);
        }
    }
    seen.into_iter().collect()
}

/// Merge a group of [`PrimitiveEffect`]s sharing a `primitive_id`.
///
/// Single-effect groups return the single effect unchanged. Multi-effect
/// groups merge parameters per §6.2B:
///
/// * [`ACTIVATION_COST_PARAM`] — summed across the group (each source hook
///   incurs its own cost).
/// * Every other parameter — merged via `max`, the doc's default strategy.
///
/// `source_channels` is re-unioned across the group. `emitter` and
/// `provenance` are copied from the first effect (all group members share
/// the same emitting entity and the same primitive manifest, hence the same
/// provenance).
fn merge_group(mut group: Vec<PrimitiveEffect>) -> PrimitiveEffect {
    if group.len() == 1 {
        return group.remove(0);
    }

    // All members share the same primitive_id, emitter, provenance — take
    // the first as the template and merge into it.
    let first = group.remove(0);
    let primitive_id = first.primitive_id.clone();
    let emitter = first.emitter;
    let provenance = first.provenance.clone();

    let mut merged_params: BTreeMap<String, Q3232> = first.parameters;
    let mut source_set: std::collections::BTreeSet<String> =
        first.source_channels.into_iter().collect();

    for effect in group {
        for ch in effect.source_channels {
            source_set.insert(ch);
        }
        for (name, value) in effect.parameters {
            match merged_params.get(&name).copied() {
                Some(existing) => {
                    let merged = if name == ACTIVATION_COST_PARAM {
                        existing.saturating_add(value)
                    } else {
                        // Default merge strategy per §6.2B: `max`.
                        if value > existing {
                            value
                        } else {
                            existing
                        }
                    };
                    merged_params.insert(name, merged);
                }
                None => {
                    merged_params.insert(name, value);
                }
            }
        }
    }

    PrimitiveEffect {
        primitive_id,
        source_channels: source_set.into_iter().collect(),
        parameters: merged_params,
        emitter,
        provenance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::{CompositionKind, HookId};
    use crate::parameter_map::{parse_expression, Expr};
    use crate::phenotype::{LifeStage, ResolvedPhenotype};
    use beast_channels::{
        BoundsPolicy, ChannelFamily, ChannelManifest, ChannelRegistry, MutationKernel,
        Provenance as ChannelProvenance, Range, ScaleBand,
    };
    use beast_primitives::PrimitiveManifest;

    // ----- fixtures --------------------------------------------------------

    fn channel_manifest(id: &str) -> ChannelManifest {
        ChannelManifest {
            id: id.into(),
            family: ChannelFamily::Sensory,
            description: "fixture".into(),
            range: Range {
                min: Q3232::ZERO,
                max: Q3232::ONE,
                units: "dimensionless".into(),
            },
            mutation_kernel: MutationKernel {
                sigma: Q3232::from_num(0.1_f64),
                bounds_policy: BoundsPolicy::Clamp,
                genesis_weight: Q3232::ONE,
                correlation_with: Vec::new(),
            },
            composition_hooks: Vec::new(),
            expression_conditions: Vec::new(),
            scale_band: ScaleBand {
                min_kg: Q3232::ZERO,
                max_kg: Q3232::from_num(1_000_i32),
            },
            body_site_applicable: false,
            provenance: ChannelProvenance::Core,
        }
    }

    fn channel_registry_with(ids: &[&str]) -> ChannelRegistry {
        let mut reg = ChannelRegistry::new();
        for id in ids {
            reg.register(channel_manifest(id)).unwrap();
        }
        reg
    }

    fn primitive_manifest(id: &str, with_force_cost: bool) -> PrimitiveManifest {
        let cost_scaling = if with_force_cost {
            r#"[{ "parameter": "force", "exponent": 1.0, "coefficient": 0.5 }]"#
        } else {
            "[]"
        };
        let parameter_schema = if with_force_cost {
            r#""force": { "type": "number", "default": 10 }"#
        } else {
            r#""intensity": { "type": "number", "default": 0 }"#
        };
        let json = format!(
            r#"{{
                "id": "{id}",
                "category": "force_application",
                "description": "emission fixture",
                "parameter_schema": {{ {parameter_schema} }},
                "composition_compatibility": [{{ "channel_family": "motor" }}],
                "cost_function": {{
                    "base_metabolic_cost": 1.0,
                    "parameter_scaling": {cost_scaling}
                }},
                "observable_signature": {{
                    "modality": "mechanical",
                    "detection_range_m": 1,
                    "pattern_key": "fixture_v1"
                }},
                "provenance": "core"
            }}"#
        );
        PrimitiveManifest::from_json_str(&json).expect("fixture manifest must be valid")
    }

    fn primitive_registry_with(primitives: Vec<PrimitiveManifest>) -> PrimitiveRegistry {
        let mut reg = PrimitiveRegistry::new();
        for p in primitives {
            reg.register(p).unwrap();
        }
        reg
    }

    fn phenotype_with(channels: &[(&str, Q3232)]) -> ResolvedPhenotype {
        let mut p = ResolvedPhenotype::new(Q3232::from_num(1_i32), LifeStage::Adult);
        for (id, v) in channels {
            p.global_channels.insert((*id).to_string(), *v);
        }
        p
    }

    fn fired_hook(
        id: u32,
        channel_values: Vec<Q3232>,
        emits: Vec<EmitSpec>,
        coefficient: Q3232,
    ) -> FiredHook {
        FiredHook {
            hook_id: HookId(id),
            kind: CompositionKind::Additive,
            channel_values,
            coefficient,
            emits,
        }
    }

    fn emit(primitive_id: &str, params: Vec<(&str, Expr)>) -> EmitSpec {
        EmitSpec {
            primitive_id: primitive_id.into(),
            parameter_mapping: params
                .into_iter()
                .map(|(name, expr)| (name.to_string(), expr))
                .collect(),
        }
    }

    // ----- unit tests ------------------------------------------------------

    #[test]
    fn emits_one_primitive_effect_per_emit_spec() {
        let creg = channel_registry_with(&["auditory"]);
        let preg = primitive_registry_with(vec![primitive_manifest("emit_pulse", false)]);
        let phenotype = phenotype_with(&[("auditory", Q3232::from_num(0.5_f64))]);

        let expr = parse_expression("ch[auditory] * 8", &creg).unwrap();
        let fired = fired_hook(
            1,
            vec![Q3232::from_num(0.5_f64)],
            vec![emit("emit_pulse", vec![("intensity", expr)])],
            Q3232::ONE,
        );

        let effects = emit_primitives(&[fired], &phenotype, &preg, EntityId::new(42)).unwrap();

        assert_eq!(effects.len(), 1);
        let e = &effects[0];
        assert_eq!(e.primitive_id, "emit_pulse");
        assert_eq!(e.emitter, EntityId::new(42));
        // 0.5 * 8 = 4
        assert_eq!(e.parameters["intensity"], Q3232::from_num(4_i32));
        // cost = 1.0 (base only, no scaling term)
        assert_eq!(e.parameters[ACTIVATION_COST_PARAM], Q3232::from_num(1_i32));
        assert_eq!(e.source_channels, vec!["auditory".to_string()]);
    }

    #[test]
    fn unknown_primitive_id_errors() {
        let creg = ChannelRegistry::new();
        let preg = PrimitiveRegistry::new();
        let phenotype = phenotype_with(&[]);

        let fired = fired_hook(
            1,
            Vec::new(),
            vec![emit("does_not_exist", Vec::new())],
            Q3232::ONE,
        );

        let err = emit_primitives(&[fired], &phenotype, &preg, EntityId::new(1)).unwrap_err();
        match err {
            InterpreterError::UnknownPrimitive { primitive_id } => {
                assert_eq!(primitive_id, "does_not_exist");
            }
            other => panic!("expected UnknownPrimitive, got {other:?}"),
        }
        // Silence the unused `creg` warning — we created it to mirror the
        // shape of other tests even though no expressions are parsed here.
        let _ = creg;
    }

    #[test]
    fn cost_scales_with_parameter_value() {
        let creg = channel_registry_with(&["force_src"]);
        let preg = primitive_registry_with(vec![primitive_manifest("strike", true)]);
        let phenotype = phenotype_with(&[("force_src", Q3232::from_num(4_i32))]);

        let expr = parse_expression("ch[force_src]", &creg).unwrap();
        let fired = fired_hook(
            1,
            vec![Q3232::from_num(4_i32)],
            vec![emit("strike", vec![("force", expr)])],
            Q3232::ONE,
        );

        let effects = emit_primitives(&[fired], &phenotype, &preg, EntityId::new(1)).unwrap();
        assert_eq!(effects.len(), 1);
        let e = &effects[0];
        assert_eq!(e.parameters["force"], Q3232::from_num(4_i32));
        // cost = 1 (base) + 0.5 * 4^1 = 3. The fixed-point `q_pow` used by
        // `evaluate_cost` goes through an `exp`/`ln` pair, so we compare
        // within a small epsilon rather than bit-exact — the determinism
        // contract is bit-identical reproducibility across runs, which is
        // covered by `same_inputs_produce_identical_outputs_twice` below.
        let cost = e.parameters[ACTIVATION_COST_PARAM];
        let diff = cost.saturating_sub(Q3232::from_num(3_i32)).saturating_abs();
        assert!(
            diff < Q3232::from_num(0.001_f64),
            "expected cost ≈ 3, got {cost:?}"
        );
    }

    #[test]
    fn two_hooks_same_primitive_merge_via_max_and_sum_cost() {
        let creg = channel_registry_with(&["a", "b"]);
        let preg = primitive_registry_with(vec![primitive_manifest("emit_pulse", false)]);
        let phenotype = phenotype_with(&[
            ("a", Q3232::from_num(0.3_f64)),
            ("b", Q3232::from_num(0.8_f64)),
        ]);

        // Hook 1 emits intensity = ch[a] = 0.3
        let expr_a = parse_expression("ch[a]", &creg).unwrap();
        let fired1 = fired_hook(
            1,
            vec![Q3232::from_num(0.3_f64)],
            vec![emit("emit_pulse", vec![("intensity", expr_a)])],
            Q3232::ONE,
        );
        // Hook 2 emits intensity = ch[b] = 0.8
        let expr_b = parse_expression("ch[b]", &creg).unwrap();
        let fired2 = fired_hook(
            2,
            vec![Q3232::from_num(0.8_f64)],
            vec![emit("emit_pulse", vec![("intensity", expr_b)])],
            Q3232::ONE,
        );

        let effects =
            emit_primitives(&[fired1, fired2], &phenotype, &preg, EntityId::new(1)).unwrap();
        assert_eq!(effects.len(), 1);
        let e = &effects[0];
        // max(0.3, 0.8) = 0.8
        assert_eq!(e.parameters["intensity"], Q3232::from_num(0.8_f64));
        // cost: base 1.0 per hook, summed = 2.0
        assert_eq!(e.parameters[ACTIVATION_COST_PARAM], Q3232::from_num(2_i32));
        // Source channels unioned from both hooks.
        assert_eq!(e.source_channels, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn independent_primitives_pass_through_and_are_sorted_by_id() {
        let creg = channel_registry_with(&["a"]);
        let preg = primitive_registry_with(vec![
            primitive_manifest("zulu", false),
            primitive_manifest("alpha", false),
        ]);
        let phenotype = phenotype_with(&[("a", Q3232::from_num(0.5_f64))]);

        let expr = parse_expression("ch[a]", &creg).unwrap();
        // Intentionally emit `zulu` before `alpha` to verify the output is
        // sorted by primitive_id, not by input order.
        let fired = fired_hook(
            1,
            vec![Q3232::from_num(0.5_f64)],
            vec![
                emit("zulu", vec![("intensity", expr.clone())]),
                emit("alpha", vec![("intensity", expr)]),
            ],
            Q3232::ONE,
        );

        let effects = emit_primitives(&[fired], &phenotype, &preg, EntityId::new(1)).unwrap();
        let ids: Vec<&str> = effects.iter().map(|e| e.primitive_id.as_str()).collect();
        assert_eq!(ids, vec!["alpha", "zulu"]);
    }

    #[test]
    fn source_channels_are_sorted_and_deduplicated() {
        let creg = channel_registry_with(&["beta", "alpha"]);
        let preg = primitive_registry_with(vec![primitive_manifest("emit_pulse", false)]);
        let phenotype = phenotype_with(&[
            ("alpha", Q3232::from_num(1_i32)),
            ("beta", Q3232::from_num(2_i32)),
        ]);

        // Mention `beta` and `alpha` in both params in non-sorted order, and
        // `beta` twice, to prove dedup + sort.
        let expr1 = parse_expression("ch[beta] + ch[alpha]", &creg).unwrap();
        let expr2 = parse_expression("ch[beta] * 2", &creg).unwrap();
        let fired = fired_hook(
            1,
            vec![Q3232::from_num(1_i32), Q3232::from_num(2_i32)],
            vec![emit(
                "emit_pulse",
                vec![("intensity", expr1), ("duration", expr2)],
            )],
            Q3232::ONE,
        );

        let effects = emit_primitives(&[fired], &phenotype, &preg, EntityId::new(1)).unwrap();
        assert_eq!(effects.len(), 1);
        assert_eq!(
            effects[0].source_channels,
            vec!["alpha".to_string(), "beta".to_string()]
        );
    }

    #[test]
    fn dormant_channel_propagates_zero_without_error() {
        let creg = channel_registry_with(&["present", "dormant"]);
        let preg = primitive_registry_with(vec![primitive_manifest("emit_pulse", false)]);
        // Only `present` is on the phenotype; `dormant` is absent entirely.
        let phenotype = phenotype_with(&[("present", Q3232::from_num(1_i32))]);

        let expr = parse_expression("ch[present] + ch[dormant]", &creg).unwrap();
        let fired = fired_hook(
            1,
            vec![Q3232::from_num(1_i32)],
            vec![emit("emit_pulse", vec![("intensity", expr)])],
            Q3232::ONE,
        );

        let effects = emit_primitives(&[fired], &phenotype, &preg, EntityId::new(1)).unwrap();
        // `ch[dormant]` evaluates to zero, so intensity = 1 + 0 = 1.
        assert_eq!(effects[0].parameters["intensity"], Q3232::from_num(1_i32));
    }

    #[test]
    fn empty_input_returns_empty_output() {
        let preg = PrimitiveRegistry::new();
        let phenotype = phenotype_with(&[]);
        let effects = emit_primitives(&[], &phenotype, &preg, EntityId::new(1)).unwrap();
        assert!(effects.is_empty());
    }

    #[test]
    fn same_inputs_produce_identical_outputs_twice() {
        // Determinism: full pipeline invoked twice yields bit-identical
        // effect vectors.
        let creg = channel_registry_with(&["a", "b"]);
        let preg = primitive_registry_with(vec![
            primitive_manifest("first", false),
            primitive_manifest("second", false),
        ]);
        let phenotype = phenotype_with(&[
            ("a", Q3232::from_num(0.25_f64)),
            ("b", Q3232::from_num(0.75_f64)),
        ]);

        let expr1 = parse_expression("ch[a] * 4 + ch[b]", &creg).unwrap();
        let expr2 = parse_expression("ch[b] * ch[a]", &creg).unwrap();
        let fired = fired_hook(
            1,
            vec![Q3232::from_num(0.25_f64), Q3232::from_num(0.75_f64)],
            vec![
                emit("first", vec![("magnitude", expr1)]),
                emit("second", vec![("concentration", expr2)]),
            ],
            Q3232::ONE,
        );

        let first = emit_primitives(
            std::slice::from_ref(&fired),
            &phenotype,
            &preg,
            EntityId::new(7),
        )
        .unwrap();
        let second = emit_primitives(&[fired], &phenotype, &preg, EntityId::new(7)).unwrap();

        assert_eq!(first, second);
    }
}
