//! Shared fixture builders for the interpreter's integration tests.
//!
//! Rust treats each file under `tests/` as a separate integration-test binary,
//! so this module is included via `mod common;` in every test file that needs
//! the fixtures. All helpers are deterministic: no RNG, no wall-clock, no
//! filesystem reads.
//!
//! The helpers build lightweight channel and primitive registries with
//! schema-valid JSON manifests (we reuse
//! [`beast_primitives::PrimitiveManifest::from_json_str`]), plus a handful of
//! composed `InterpreterHook`s covering Additive, Threshold, Multiplicative,
//! Antagonistic, and Gating kinds.

#![allow(dead_code)] // not every test file uses every helper

use std::collections::BTreeMap;

use beast_channels::{
    BoundsPolicy, ChannelFamily, ChannelManifest, ChannelRegistry, ExpressionCondition,
    MutationKernel, Provenance as ChannelProvenance, Range, ScaleBand,
};
use beast_core::{TickCounter, Q3232};
use beast_interpreter::{
    composition::{CompositionKind, EmitSpec, HookId, InterpreterHook},
    parameter_map::{parse_expression, Expr},
    phenotype::{BodyRegion, BodySite, Environment, LifeStage, ResolvedPhenotype},
};
use beast_primitives::{PrimitiveManifest, PrimitiveRegistry};

pub fn q(v: f64) -> Q3232 {
    Q3232::from_num(v)
}

/// Build a bare-bones channel manifest with caller-controlled scale band and
/// body-site flag. All other fields take benign defaults — the interpreter
/// only reads `id`, `scale_band`, and `body_site_applicable` from channel
/// manifests (composition rules live on the `InterpreterHook`s).
pub fn channel_manifest(
    id: &str,
    min_kg: f64,
    max_kg: f64,
    body_site_applicable: bool,
) -> ChannelManifest {
    ChannelManifest {
        id: id.into(),
        family: ChannelFamily::Sensory,
        description: "integration-test fixture".into(),
        range: Range {
            min: Q3232::ZERO,
            max: Q3232::ONE,
            units: "dimensionless".into(),
        },
        mutation_kernel: MutationKernel {
            sigma: q(0.1),
            bounds_policy: BoundsPolicy::Clamp,
            genesis_weight: Q3232::ONE,
            correlation_with: Vec::new(),
        },
        composition_hooks: Vec::new(),
        expression_conditions: Vec::new(),
        scale_band: ScaleBand {
            min_kg: q(min_kg),
            max_kg: q(max_kg),
        },
        body_site_applicable,
        provenance: ChannelProvenance::Core,
    }
}

/// Build a channel registry from `(id, min_kg, max_kg, body_site_applicable)`
/// tuples.
pub fn build_channel_registry(specs: &[(&str, f64, f64, bool)]) -> ChannelRegistry {
    let mut reg = ChannelRegistry::new();
    for (id, min_kg, max_kg, body_site) in specs {
        reg.register(channel_manifest(id, *min_kg, *max_kg, *body_site))
            .expect("fixture ids are unique");
    }
    reg
}

/// Build a primitive manifest with one default parameter and a simple
/// `{base 1.0 + coefficient * param^1}` cost function. The `force` variant
/// scales with the `force` parameter; the plain variant has no scaling term.
pub fn primitive_manifest_json(id: &str, param_name: &str, with_force_cost: bool) -> String {
    let cost_scaling = if with_force_cost {
        format!(r#"[{{ "parameter": "{param_name}", "exponent": 1.0, "coefficient": 0.5 }}]"#)
    } else {
        "[]".into()
    };
    format!(
        r#"{{
            "id": "{id}",
            "category": "force_application",
            "description": "integration-test fixture",
            "parameter_schema": {{ "{param_name}": {{ "type": "number", "default": 0 }} }},
            "composition_compatibility": [{{ "channel_family": "sensory" }}],
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
    )
}

/// Parse a manifest string into a `PrimitiveManifest`. Panics on parse error
/// (fixture authors control the source, so failures here are bugs in the
/// fixture).
pub fn parse_primitive_manifest(json: &str) -> PrimitiveManifest {
    PrimitiveManifest::from_json_str(json).expect("fixture primitive manifest must parse")
}

/// Build a primitive registry from `(id, param_name, with_force_cost)`
/// tuples.
pub fn build_primitive_registry(specs: &[(&str, &str, bool)]) -> PrimitiveRegistry {
    let mut reg = PrimitiveRegistry::new();
    for (id, param, with_cost) in specs {
        let json = primitive_manifest_json(id, param, *with_cost);
        reg.register(parse_primitive_manifest(&json))
            .expect("fixture primitive ids are unique");
    }
    reg
}

/// Parse a parameter expression; panics on error (fixture failures are bugs).
pub fn expr(src: &str, registry: &ChannelRegistry) -> Expr {
    parse_expression(src, registry).expect("fixture expression must parse")
}

/// Build a single interpreter hook covering the common case (additive,
/// no thresholds, no conditions). Callers that need threshold or gating
/// behaviour build hooks by hand using the [`InterpreterHook`] struct
/// directly.
pub fn additive_hook(
    id: u32,
    channel_ids: &[&str],
    emit_primitive_id: &str,
    params: Vec<(&str, Expr)>,
) -> InterpreterHook {
    InterpreterHook {
        id: HookId(id),
        kind: CompositionKind::Additive,
        channel_ids: channel_ids.iter().map(|s| (*s).to_string()).collect(),
        thresholds: Vec::new(),
        coefficient: Q3232::ONE,
        expression_conditions: Vec::new(),
        emits: vec![EmitSpec {
            primitive_id: emit_primitive_id.into(),
            parameter_mapping: params
                .into_iter()
                .map(|(n, e)| (n.to_string(), e))
                .collect(),
        }],
    }
}

/// Build a threshold hook: fires only when every channel value meets its
/// parallel threshold entry.
pub fn threshold_hook(
    id: u32,
    channel_ids: &[&str],
    thresholds: &[Q3232],
    emit_primitive_id: &str,
    params: Vec<(&str, Expr)>,
) -> InterpreterHook {
    InterpreterHook {
        id: HookId(id),
        kind: CompositionKind::Threshold,
        channel_ids: channel_ids.iter().map(|s| (*s).to_string()).collect(),
        thresholds: thresholds.to_vec(),
        coefficient: Q3232::ONE,
        expression_conditions: Vec::new(),
        emits: vec![EmitSpec {
            primitive_id: emit_primitive_id.into(),
            parameter_mapping: params
                .into_iter()
                .map(|(n, e)| (n.to_string(), e))
                .collect(),
        }],
    }
}

/// Build a resolved phenotype for the fixture world.
pub fn phenotype(
    body_mass_kg: f64,
    life_stage: LifeStage,
    globals: &[(&str, f64)],
    env: Environment,
) -> ResolvedPhenotype {
    let mut p = ResolvedPhenotype {
        global_channels: BTreeMap::new(),
        body_map: Vec::new(),
        body_mass_kg: q(body_mass_kg),
        life_stage,
        expression_tick: TickCounter::default(),
        environment: env,
    };
    for (id, v) in globals {
        p.global_channels.insert((*id).to_string(), q(*v));
    }
    p
}

/// Q3232-valued variant of [`phenotype`]. Preferred by callers that need to
/// stay clear of `float_arithmetic` — channel values are assembled from
/// integer base + increment via `Q3232::saturating_add`.
pub fn phenotype_q(
    body_mass: Q3232,
    life_stage: LifeStage,
    globals: &[(&str, Q3232)],
    env: Environment,
) -> ResolvedPhenotype {
    let mut p = ResolvedPhenotype {
        global_channels: BTreeMap::new(),
        body_map: Vec::new(),
        body_mass_kg: body_mass,
        life_stage,
        expression_tick: TickCounter::default(),
        environment: env,
    };
    for (id, v) in globals {
        p.global_channels.insert((*id).to_string(), *v);
    }
    p
}

/// Attach a region to a phenotype with the given per-channel amplitudes.
/// Returns a modified copy — keeps usage immutable-by-default.
pub fn with_region(
    mut phenotype: ResolvedPhenotype,
    region_id: u32,
    site: BodySite,
    amps: &[(&str, f64)],
) -> ResolvedPhenotype {
    let mut channel_amplitudes = BTreeMap::new();
    for (ch, v) in amps {
        channel_amplitudes.insert((*ch).to_string(), q(*v));
    }
    phenotype.body_map.push(BodyRegion {
        id: region_id,
        body_site: site,
        surface_vs_internal: q(0.5),
        channel_amplitudes,
    });
    phenotype
}

/// Builds the "standard" fixture world used by determinism and proptest:
///
/// * 5 channels named `alpha`..`epsilon`, all universal-scale (0..1e9 kg)
/// * 4 primitives `p0`..`p3` (first three plain, `p3` cost-scaling on its param)
/// * 6 hooks mixing Additive, Threshold, Multiplicative, Antagonistic, and
///   Gating kinds.
///
/// All ids are ASCII lowercase and sort lexicographically.
pub struct FixtureWorld {
    pub channel_registry: ChannelRegistry,
    pub primitive_registry: PrimitiveRegistry,
    pub hooks: Vec<InterpreterHook>,
}

pub fn standard_world() -> FixtureWorld {
    let channel_registry = build_channel_registry(&[
        ("alpha", 0.0, 1e9, false),
        ("beta", 0.0, 1e9, false),
        ("gamma", 0.0, 1e9, false),
        ("delta", 0.0, 1e9, false),
        ("epsilon", 0.0, 1e9, false),
    ]);
    let primitive_registry = build_primitive_registry(&[
        ("p0_pulse", "intensity", false),
        ("p1_strike", "force", true),
        ("p2_signal", "magnitude", false),
        ("p3_calm", "level", false),
    ]);

    let e_alpha = expr("ch[alpha]", &channel_registry);
    let e_alpha_times_two = expr("ch[alpha] * 2", &channel_registry);
    let e_alpha_plus_beta = expr("ch[alpha] + ch[beta]", &channel_registry);
    let e_gamma = expr("ch[gamma]", &channel_registry);
    let e_delta = expr("ch[delta]", &channel_registry);
    let e_epsilon_plus_one = expr("ch[epsilon] + 1", &channel_registry);

    let hooks = vec![
        additive_hook(
            1,
            &["alpha"],
            "p0_pulse",
            vec![("intensity", e_alpha.clone())],
        ),
        additive_hook(
            2,
            &["alpha", "beta"],
            "p2_signal",
            vec![("magnitude", e_alpha_plus_beta)],
        ),
        threshold_hook(
            3,
            &["gamma"],
            &[q(0.25)],
            "p1_strike",
            vec![("force", e_gamma)],
        ),
        InterpreterHook {
            id: HookId(4),
            kind: CompositionKind::Multiplicative,
            channel_ids: vec!["alpha".into()],
            thresholds: Vec::new(),
            coefficient: q(0.5),
            expression_conditions: Vec::new(),
            emits: vec![EmitSpec {
                primitive_id: "p0_pulse".into(),
                parameter_mapping: vec![("intensity".into(), e_alpha_times_two)],
            }],
        },
        InterpreterHook {
            id: HookId(5),
            kind: CompositionKind::Antagonistic,
            channel_ids: vec!["delta".into(), "epsilon".into()],
            thresholds: Vec::new(),
            coefficient: q(0.1),
            expression_conditions: Vec::new(),
            emits: vec![EmitSpec {
                primitive_id: "p3_calm".into(),
                parameter_mapping: vec![("level".into(), e_delta)],
            }],
        },
        InterpreterHook {
            id: HookId(6),
            kind: CompositionKind::Gating,
            channel_ids: vec!["epsilon".into()],
            thresholds: vec![q(0.1)],
            coefficient: Q3232::ONE,
            expression_conditions: vec![ExpressionCondition::DevelopmentalStage {
                stage: "adult".into(),
            }],
            emits: vec![EmitSpec {
                primitive_id: "p2_signal".into(),
                parameter_mapping: vec![("magnitude".into(), e_epsilon_plus_one)],
            }],
        },
    ];

    FixtureWorld {
        channel_registry,
        primitive_registry,
        hooks,
    }
}

/// Build 20 phenotypes used by the 1000-run determinism test. Values are
/// assembled entirely via `Q3232` saturating arithmetic (no `f64` math) so
/// the fixture stays inside the determinism invariant (INVARIANTS §1) and
/// does not trip `clippy::float_arithmetic`.
///
/// Values are chosen to exercise all firing paths:
/// * some phenotypes have every channel above the threshold hook's 0.25 gate
/// * some have gamma below 0.25 so the threshold hook drops
/// * some have delta == epsilon so the antagonistic hook drops
/// * some have juvenile life-stage so the gating hook drops
pub fn standard_phenotypes() -> Vec<ResolvedPhenotype> {
    let base_envs = [
        Environment::default(),
        Environment {
            biome_flags: vec!["forest".into()],
            season: Some("spring".into()),
            ..Environment::default()
        },
        Environment {
            biome_flags: vec!["aquatic".into()],
            season: Some("summer".into()),
            population_density_per_km2: Some(q(50.0)),
            ..Environment::default()
        },
    ];
    let stages = [LifeStage::Juvenile, LifeStage::Adult, LifeStage::Elderly];

    // Step constants — each evaluated once via `Q3232::from_num` on a literal,
    // then combined via saturating arithmetic inside the loop.
    let alpha_base = q(0.1);
    let alpha_step = q(0.05);
    let beta_base = q(0.2);
    let beta_step = q(0.1);
    let gamma_hi = q(0.4);
    let gamma_lo = q(0.1);
    let delta_base = q(0.2);
    let delta_step = q(0.08);
    let epsilon_base = q(0.3);
    let epsilon_step = q(0.05);
    let mass_base = Q3232::ONE;
    let mass_step = q(10.0);

    let mut out = Vec::with_capacity(20);
    for i in 0..20u32 {
        let alpha = alpha_base.saturating_add(alpha_step.saturating_mul(Q3232::from_num(i)));
        let beta = beta_base.saturating_add(beta_step.saturating_mul(Q3232::from_num(i % 5)));
        let gamma = if i % 3 == 0 { gamma_hi } else { gamma_lo };
        let delta = delta_base.saturating_add(delta_step.saturating_mul(Q3232::from_num(i % 7)));
        let epsilon =
            epsilon_base.saturating_add(epsilon_step.saturating_mul(Q3232::from_num(i % 11)));
        let mass = mass_base.saturating_add(mass_step.saturating_mul(Q3232::from_num(i)));
        let env = base_envs[(i as usize) % base_envs.len()].clone();
        let stage = stages[(i as usize) % stages.len()];
        let p = phenotype_q(
            mass,
            stage,
            &[
                ("alpha", alpha),
                ("beta", beta),
                ("gamma", gamma),
                ("delta", delta),
                ("epsilon", epsilon),
            ],
            env,
        );
        out.push(p);
    }
    out
}
