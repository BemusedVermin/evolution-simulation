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
//! * **First-class activation cost — [issue #67]**. Until
//!   [`beast_primitives::PrimitiveEffect`] gains a dedicated `activation_cost`
//!   field we surface the evaluated cost via the [`ACTIVATION_COST_PARAM`]
//!   sentinel parameter below.
//! * **Set-valued Union merge — [issue #95]**. `MergeStrategy::Union`
//!   currently collapses to `Max` because parameter values are scalar
//!   `Q3232`. When set-valued parameters land, `Union` can implement true
//!   set-union semantics.
//!
//! [issue #67]: https://github.com/BemusedVermin/evolution-simulation/issues/67
//! [issue #95]: https://github.com/BemusedVermin/evolution-simulation/issues/95

use std::collections::{BTreeMap, BTreeSet};

use beast_core::{EntityId, Q3232};
use beast_primitives::{
    evaluate_cost, MergeStrategy, PrimitiveEffect, PrimitiveManifest, PrimitiveRegistry,
};

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
    for (id, group) in groups {
        // Every id here was already looked up by `build_effect` above —
        // `UnknownPrimitive` would have short-circuited the whole call. The
        // lookup is therefore infallible on this path; `unreachable!` is a
        // tighter contract than re-emitting the error, since no test or
        // caller can observe the branch.
        let manifest = registry.get(&id).unwrap_or_else(|| {
            unreachable!("build_effect admitted primitive id `{id}` but registry lookup failed")
        });
        out.push(merge_group(group, manifest));
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
/// Per §6.2 (pseudocode line 746) the provenance is the union of:
///
/// * the firing hook's own [`FiredHook::channel_ids`] — which includes
///   "gate-only" channels that decide whether the hook fires but never
///   appear in any `parameter_mapping` expression, and
/// * every channel id referenced by an expression in `emit.parameter_mapping`.
///
/// Taking the union ensures a hook whose trigger set differs from its
/// expression refs (e.g. `[auditory, vocal, spatial]` thresholded but only
/// `vocal` read downstream) still reports all three as sources.
fn collect_source_channels(fired: &FiredHook, emit: &EmitSpec) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for channel_id in &fired.channel_ids {
        seen.insert(channel_id.clone());
    }
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
/// groups collect every parameter's values across the group, then apply
/// the per-parameter strategy declared on `manifest.merge_strategy` — with
/// two fixed rules that do not come from the manifest:
///
/// * [`ACTIVATION_COST_PARAM`] always sums (each source hook incurs its
///   own cost; the sentinel parameter is not declared in
///   `parameter_schema`, so manifests can't override it).
/// * Parameters absent from `manifest.merge_strategy` default to
///   [`MergeStrategy::Max`], the spec's conservative fallback per §6.2B
///   (line 802).
///
/// `source_channels` is re-unioned across the group. `emitter` and
/// `provenance` are copied from the first effect (all group members share
/// the same emitting entity and the same primitive manifest, hence the same
/// provenance).
///
/// # Determinism
///
/// The collect-then-apply-once shape avoids the pairwise rolling-mean
/// artefact (`((a+b)/2 + c)/2 != (a+b+c)/3`). Per-parameter values are
/// inserted into a [`BTreeMap`] keyed by name, preserving the
/// lexicographic iteration order required by INVARIANTS §1. Within a
/// parameter's value list, order is `group` order — stable with respect
/// to the caller's input since `emit_primitives` pushes into each group
/// in the order `fired_hooks` appears.
fn merge_group(mut group: Vec<PrimitiveEffect>, manifest: &PrimitiveManifest) -> PrimitiveEffect {
    debug_assert!(
        !group.is_empty(),
        "merge_group invoked on an empty group — grouping contract violated"
    );

    // Single-effect groups are the common case; short-circuit before the
    // collect/apply pipeline to avoid an allocation per parameter.
    if group.len() == 1 {
        return group.remove(0);
    }

    // Carry identity fields from the first effect; group members share
    // them by construction.
    let primitive_id = group[0].primitive_id.clone();
    let emitter = group[0].emitter;
    let provenance = group[0].provenance.clone();

    // Collect every parameter value across the group before applying any
    // strategy. Using a `BTreeMap<String, Vec<Q3232>>` keeps both the
    // outer key iteration and the inner per-key order deterministic
    // (lexicographic over keys, input-order within each key).
    let mut per_param: BTreeMap<String, Vec<Q3232>> = BTreeMap::new();
    let mut source_set: BTreeSet<String> = BTreeSet::new();
    for effect in group {
        for ch in effect.source_channels {
            source_set.insert(ch);
        }
        for (name, value) in effect.parameters {
            per_param.entry(name).or_default().push(value);
        }
    }

    let merged_params: BTreeMap<String, Q3232> = per_param
        .into_iter()
        .map(|(name, values)| {
            let strategy = strategy_for(&name, manifest);
            (name, apply_strategy(strategy, &values))
        })
        .collect();

    PrimitiveEffect {
        primitive_id,
        source_channels: source_set.into_iter().collect(),
        parameters: merged_params,
        emitter,
        provenance,
    }
}

/// Resolve the merge strategy for a single parameter.
///
/// * The sentinel [`ACTIVATION_COST_PARAM`] always sums — it is not
///   declarable via `manifest.merge_strategy` because it is not part of
///   `parameter_schema`. Callers (mod authors) cannot override this until
///   activation cost is promoted to a first-class field per issue #67.
/// * Other parameters consult `manifest.merge_strategy`; an absent key
///   resolves to [`MergeStrategy::Max`] per §6.2B line 802.
fn strategy_for(param_name: &str, manifest: &PrimitiveManifest) -> MergeStrategy {
    if param_name == ACTIVATION_COST_PARAM {
        return MergeStrategy::Sum;
    }
    manifest
        .merge_strategy
        .get(param_name)
        .copied()
        .unwrap_or(MergeStrategy::Max)
}

/// Collapse a list of values for one parameter using the declared strategy.
///
/// `values` is guaranteed non-empty by the caller — `merge_group` only
/// populates a parameter's vec when at least one effect in the group
/// emitted it.
fn apply_strategy(strategy: MergeStrategy, values: &[Q3232]) -> Q3232 {
    debug_assert!(
        !values.is_empty(),
        "apply_strategy received an empty value list — merge_group invariant violated"
    );
    match strategy {
        MergeStrategy::Sum => values
            .iter()
            .copied()
            .fold(Q3232::ZERO, Q3232::saturating_add),
        // Deterministic tie-break: `Q3232::max` uses the `Ord` impl, which
        // compares via the underlying `I32F32` — lexicographic on the
        // fixed-point bit pattern. Equal values pass through unchanged.
        MergeStrategy::Max => values.iter().copied().max().unwrap_or(Q3232::ZERO),
        MergeStrategy::Mean => {
            // NOTE: saturation is intentional — Q3232 sim math never raises,
            // and the ECS tick budget bounds group size so realistic sums
            // stay well under `I32F32::MAX`. A saturating fold here
            // silently clips an out-of-range sum to the max, which would
            // under-report the mean. Balance tuning work that drives
            // parameter magnitudes toward that ceiling should consider
            // per-primitive range clamps at manifest load rather than
            // raising the sentinel here.
            let sum = values
                .iter()
                .copied()
                .fold(Q3232::ZERO, Q3232::saturating_add);
            sum.saturating_div(Q3232::from_num(values.len() as i64))
        }
        // See `MergeStrategy::Union` docs and issue #95 — falls through to
        // Max until `PrimitiveEffect.parameters` can represent sets.
        MergeStrategy::Union => values.iter().copied().max().unwrap_or(Q3232::ZERO),
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
        channel_ids: &[&str],
        channel_values: Vec<Q3232>,
        emits: Vec<EmitSpec>,
        coefficient: Q3232,
    ) -> FiredHook {
        FiredHook {
            hook_id: HookId(id),
            kind: CompositionKind::Additive,
            channel_ids: channel_ids.iter().map(|s| (*s).to_string()).collect(),
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

    /// Run the emitter against a list of fired hooks and assert exactly one
    /// [`PrimitiveEffect`] comes back, returning it. Breaks up the
    /// `emit_primitives → unwrap → assert_eq!(len, 1)` shape that lizard's
    /// duplicate detector flags as repeated boilerplate when it recurs in
    /// every test body.
    fn run_expecting_single_effect(
        fired: Vec<FiredHook>,
        phenotype: &ResolvedPhenotype,
        preg: &PrimitiveRegistry,
    ) -> PrimitiveEffect {
        let mut effects = emit_primitives(&fired, phenotype, preg, EntityId::new(1)).unwrap();
        assert_eq!(effects.len(), 1);
        effects.remove(0)
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
            &["auditory"],
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
            &[],
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
            &["force_src"],
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
            &["a"],
            vec![Q3232::from_num(0.3_f64)],
            vec![emit("emit_pulse", vec![("intensity", expr_a)])],
            Q3232::ONE,
        );
        // Hook 2 emits intensity = ch[b] = 0.8
        let expr_b = parse_expression("ch[b]", &creg).unwrap();
        let fired2 = fired_hook(
            2,
            &["b"],
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
            &["a"],
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
            &["alpha", "beta"],
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
            &["present"],
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
            &["a", "b"],
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

    // ----- regression tests for #70: gate-only source-channel provenance -----

    /// Build the standard single-primitive registry + phenotype setup used
    /// by the #70 regression tests. Each channel id gets phenotype value 0.5
    /// (well above the auto-fail zero threshold) so threshold/gating hooks
    /// admit without each test repeating the arithmetic.
    fn single_primitive_setup(
        channels: &[&str],
    ) -> (ChannelRegistry, PrimitiveRegistry, ResolvedPhenotype) {
        let creg = channel_registry_with(channels);
        let preg = primitive_registry_with(vec![primitive_manifest("emit_pulse", false)]);
        let values: Vec<(&str, Q3232)> = channels
            .iter()
            .map(|id| (*id, Q3232::from_num(0.5_f64)))
            .collect();
        let phenotype = phenotype_with(&values);
        (creg, preg, phenotype)
    }

    #[test]
    fn gate_only_channels_appear_in_source_channels() {
        // §6.2 (line 746) records the hook's firing channels in
        // `source_channels`. A threshold hook gated on `[auditory, vocal,
        // spatial]` whose only parameter expression reads `vocal` must still
        // report all three — `auditory` and `spatial` are gate-only and
        // otherwise silently dropped from provenance.
        let (creg, preg, phenotype) = single_primitive_setup(&["auditory", "spatial", "vocal"]);

        let expr = parse_expression("ch[vocal]", &creg).unwrap();
        let fired = fired_hook(
            1,
            &["auditory", "vocal", "spatial"],
            vec![
                Q3232::from_num(0.5_f64),
                Q3232::from_num(0.5_f64),
                Q3232::from_num(0.5_f64),
            ],
            vec![emit("emit_pulse", vec![("intensity", expr)])],
            Q3232::ONE,
        );

        let effect = run_expecting_single_effect(vec![fired], &phenotype, &preg);
        assert_eq!(
            effect.source_channels,
            vec![
                "auditory".to_string(),
                "spatial".to_string(),
                "vocal".to_string(),
            ],
        );
    }

    #[test]
    fn literal_parameter_additive_hook_still_records_firing_channel() {
        // Additive hook on `metabolism` with a purely literal parameter
        // (`"delta": "8"`). No ChannelRef in the expression — before the fix
        // `source_channels` came back empty, dropping provenance entirely.
        let (creg, preg, phenotype) = single_primitive_setup(&["metabolism"]);

        let expr = parse_expression("8", &creg).unwrap();
        let fired = fired_hook(
            1,
            &["metabolism"],
            vec![Q3232::from_num(0.5_f64)],
            vec![emit("emit_pulse", vec![("delta", expr)])],
            Q3232::ONE,
        );

        let effect = run_expecting_single_effect(vec![fired], &phenotype, &preg);
        assert_eq!(effect.source_channels, vec!["metabolism".to_string()]);
    }

    #[test]
    fn gate_only_channels_survive_merge_across_hooks() {
        // Two hooks emit the same primitive but have disjoint firing-channel
        // sets. Hook A fires on [x, y] with expression `ch[y]`; hook B fires
        // on [y, z] with expression `ch[y]`. The merged effect's
        // `source_channels` must be the full union [x, y, z] — proving that
        // `x` and `z` (each a gate-only channel on one hook and entirely
        // absent from the other's expression) survive the group merge step.
        let (creg, preg, phenotype) = single_primitive_setup(&["x", "y", "z"]);

        let expr_a = parse_expression("ch[y]", &creg).unwrap();
        let fired_a = fired_hook(
            1,
            &["x", "y"],
            vec![Q3232::from_num(0.5_f64), Q3232::from_num(0.5_f64)],
            vec![emit("emit_pulse", vec![("intensity", expr_a)])],
            Q3232::ONE,
        );
        let expr_b = parse_expression("ch[y]", &creg).unwrap();
        let fired_b = fired_hook(
            2,
            &["y", "z"],
            vec![Q3232::from_num(0.5_f64), Q3232::from_num(0.5_f64)],
            vec![emit("emit_pulse", vec![("intensity", expr_b)])],
            Q3232::ONE,
        );

        let effect = run_expecting_single_effect(vec![fired_a, fired_b], &phenotype, &preg);
        assert_eq!(
            effect.source_channels,
            vec!["x".to_string(), "y".to_string(), "z".to_string()],
        );
    }

    // ----- regression tests for #71: typed merge_strategy per parameter -----

    /// Build a primitive manifest whose `parameter_schema` declares one
    /// `number`-typed parameter per name in `params`, and whose
    /// `merge_strategy` entries come from `strategies` (must be a subset of
    /// `params`). No cost scaling so every test's `_activation_cost` stays
    /// at base = 1.0 and therefore sums to `hook_count` across the group.
    fn manifest_with_strategies(
        id: &str,
        params: &[&str],
        strategies: &[(&str, &str)],
    ) -> PrimitiveManifest {
        let schema = params
            .iter()
            .map(|p| format!(r#""{p}": {{ "type": "number" }}"#))
            .collect::<Vec<_>>()
            .join(", ");
        let strategy = if strategies.is_empty() {
            String::new()
        } else {
            let entries = strategies
                .iter()
                .map(|(k, v)| format!(r#""{k}": "{v}""#))
                .collect::<Vec<_>>()
                .join(", ");
            format!(r#","merge_strategy": {{ {entries} }}"#)
        };
        let json = format!(
            r#"{{
                "id": "{id}",
                "category": "force_application",
                "description": "merge-strategy fixture",
                "parameter_schema": {{ {schema} }},
                "composition_compatibility": [{{ "channel_family": "motor" }}],
                "cost_function": {{ "base_metabolic_cost": 1.0 }},
                "observable_signature": {{
                    "modality": "mechanical",
                    "detection_range_m": 1,
                    "pattern_key": "strategy_v1"
                }}{strategy},
                "provenance": "core"
            }}"#
        );
        PrimitiveManifest::from_json_str(&json).expect("fixture manifest must be valid")
    }

    /// Fire two hooks emitting the same primitive with one parameter each,
    /// evaluating `literal_a` for hook 1 and `literal_b` for hook 2.
    fn run_two_hook_merge(
        manifest: PrimitiveManifest,
        param: &str,
        literal_a: &str,
        literal_b: &str,
    ) -> PrimitiveEffect {
        let primitive_id = manifest.id.clone();
        let creg = channel_registry_with(&["src"]);
        let preg = primitive_registry_with(vec![manifest]);
        let phenotype = phenotype_with(&[("src", Q3232::ONE)]);

        let expr_a = parse_expression(literal_a, &creg).unwrap();
        let fired_a = fired_hook(
            1,
            &["src"],
            vec![Q3232::ONE],
            vec![emit(&primitive_id, vec![(param, expr_a)])],
            Q3232::ONE,
        );
        let expr_b = parse_expression(literal_b, &creg).unwrap();
        let fired_b = fired_hook(
            2,
            &["src"],
            vec![Q3232::ONE],
            vec![emit(&primitive_id, vec![(param, expr_b)])],
            Q3232::ONE,
        );

        run_expecting_single_effect(vec![fired_a, fired_b], &phenotype, &preg)
    }

    #[test]
    fn sum_strategy_merges_two_hooks_by_saturating_addition() {
        // 3 + 5 = 8 — Sum is the distinguishing behaviour vs. Max (which
        // would return 5) and Mean (which would return 4).
        let manifest = manifest_with_strategies("with_sum", &["delta"], &[("delta", "sum")]);
        let effect = run_two_hook_merge(manifest, "delta", "3", "5");
        assert_eq!(effect.parameters["delta"], Q3232::from_num(8_i32));
    }

    #[test]
    fn max_strategy_is_explicit_opt_in_behaviour() {
        // Explicitly declaring `max` matches the pre-#71 default so this
        // also guards against accidentally inverting the implementation.
        let manifest = manifest_with_strategies("with_max", &["peak"], &[("peak", "max")]);
        let effect = run_two_hook_merge(manifest, "peak", "3", "5");
        assert_eq!(effect.parameters["peak"], Q3232::from_num(5_i32));
    }

    #[test]
    fn mean_strategy_is_true_average_not_pairwise_rolling() {
        // 3 + 5 = 8, 8 / 2 = 4. With three hooks we also need to confirm
        // that mean is computed over the full group, not a left-folded
        // running average: (2 + 4 + 9) / 3 = 5, but ((2+4)/2 + 9)/2 = 6.
        let manifest = manifest_with_strategies("with_mean", &["avg"], &[("avg", "mean")]);
        let effect = run_two_hook_merge(manifest.clone(), "avg", "3", "5");
        assert_eq!(effect.parameters["avg"], Q3232::from_num(4_i32));

        // Three-hook variant.
        let creg = channel_registry_with(&["src"]);
        let preg = primitive_registry_with(vec![manifest]);
        let phenotype = phenotype_with(&[("src", Q3232::ONE)]);
        let fired: Vec<_> = ["2", "4", "9"]
            .iter()
            .enumerate()
            .map(|(i, literal)| {
                let expr = parse_expression(literal, &creg).unwrap();
                fired_hook(
                    i as u32,
                    &["src"],
                    vec![Q3232::ONE],
                    vec![emit("with_mean", vec![("avg", expr)])],
                    Q3232::ONE,
                )
            })
            .collect();
        let effect = run_expecting_single_effect(fired, &phenotype, &preg);
        assert_eq!(effect.parameters["avg"], Q3232::from_num(5_i32));
    }

    #[test]
    fn union_strategy_collapses_to_max_until_set_valued_params_land() {
        // Placeholder behaviour tracked by issue #95: Q3232 parameters are
        // scalar, so Union falls through to Max. Test guards against a
        // future regression where Union accidentally diverges (e.g. starts
        // summing) before the data-model upgrade lands.
        let manifest = manifest_with_strategies("with_union", &["tags"], &[("tags", "union")]);
        let effect = run_two_hook_merge(manifest, "tags", "3", "5");
        assert_eq!(effect.parameters["tags"], Q3232::from_num(5_i32));
    }

    #[test]
    fn absent_strategy_falls_back_to_max() {
        // Manifest declares no merge_strategy at all — every parameter
        // takes the `max` default per §6.2B line 802.
        let manifest = manifest_with_strategies("no_strategy", &["intensity"], &[]);
        let effect = run_two_hook_merge(manifest, "intensity", "3", "5");
        assert_eq!(effect.parameters["intensity"], Q3232::from_num(5_i32));
    }

    #[test]
    fn mixed_strategies_apply_per_parameter() {
        // Same primitive, three parameters, three strategies. The merge
        // must route each parameter to its declared strategy rather than
        // applying one blanket rule.
        let manifest = manifest_with_strategies(
            "mixed",
            &["s", "m", "a"],
            &[("s", "sum"), ("m", "max"), ("a", "mean")],
        );
        let creg = channel_registry_with(&["src"]);
        let preg = primitive_registry_with(vec![manifest]);
        let phenotype = phenotype_with(&[("src", Q3232::ONE)]);

        let mk_fired = |id: u32, s: &str, m: &str, a: &str| {
            fired_hook(
                id,
                &["src"],
                vec![Q3232::ONE],
                vec![emit(
                    "mixed",
                    vec![
                        ("a", parse_expression(a, &creg).unwrap()),
                        ("m", parse_expression(m, &creg).unwrap()),
                        ("s", parse_expression(s, &creg).unwrap()),
                    ],
                )],
                Q3232::ONE,
            )
        };

        let effect = run_expecting_single_effect(
            vec![mk_fired(1, "2", "3", "4"), mk_fired(2, "5", "7", "8")],
            &phenotype,
            &preg,
        );
        assert_eq!(effect.parameters["s"], Q3232::from_num(7_i32)); // 2+5
        assert_eq!(effect.parameters["m"], Q3232::from_num(7_i32)); // max(3,7)
        assert_eq!(effect.parameters["a"], Q3232::from_num(6_i32)); // (4+8)/2
    }

    #[test]
    fn sparse_parameter_present_in_only_one_effect_passes_through() {
        // Hook A emits `{intensity: 3}`; hook B emits
        // `{intensity: 5, falloff: 2}`. `falloff` only exists on hook B's
        // effect, so the merged group's `per_param["falloff"]` is a
        // single-element vec. The strategy still runs; for any strategy,
        // the one-value result must equal that value.
        let manifest = manifest_with_strategies(
            "sparse",
            &["intensity", "falloff"],
            &[("intensity", "sum"), ("falloff", "sum")],
        );
        let creg = channel_registry_with(&["src"]);
        let preg = primitive_registry_with(vec![manifest]);
        let phenotype = phenotype_with(&[("src", Q3232::ONE)]);

        let expr_i_a = parse_expression("3", &creg).unwrap();
        let fired_a = fired_hook(
            1,
            &["src"],
            vec![Q3232::ONE],
            vec![emit("sparse", vec![("intensity", expr_i_a)])],
            Q3232::ONE,
        );
        let expr_i_b = parse_expression("5", &creg).unwrap();
        let expr_f_b = parse_expression("2", &creg).unwrap();
        let fired_b = fired_hook(
            2,
            &["src"],
            vec![Q3232::ONE],
            vec![emit(
                "sparse",
                vec![("falloff", expr_f_b), ("intensity", expr_i_b)],
            )],
            Q3232::ONE,
        );

        let effect = run_expecting_single_effect(vec![fired_a, fired_b], &phenotype, &preg);
        // `intensity` is in both; sum = 8. `falloff` is only in B; passes
        // through as 2 (single-element sum).
        assert_eq!(effect.parameters["intensity"], Q3232::from_num(8_i32));
        assert_eq!(effect.parameters["falloff"], Q3232::from_num(2_i32));
    }

    #[test]
    fn activation_cost_still_sums_regardless_of_declared_strategy() {
        // `_activation_cost` is a sentinel parameter inserted by
        // `build_effect` — not something manifests declare in
        // `parameter_schema`, and not something they can override via
        // `merge_strategy`. It must continue to sum across the group so
        // each source hook's cost is accounted for.
        let manifest = manifest_with_strategies("cost_fixture", &["delta"], &[("delta", "sum")]);
        let effect = run_two_hook_merge(manifest, "delta", "3", "5");
        // Base cost 1.0 per hook × 2 hooks = 2.0.
        assert_eq!(
            effect.parameters[ACTIVATION_COST_PARAM],
            Q3232::from_num(2_i32)
        );
    }
}
