//! Composition-threshold acceptance fixtures (S4.6 — issue #60).
//!
//! Exercises the §6.2 threshold rule end-to-end through
//! [`beast_interpreter::interpret_phenotype`]:
//!
//! * A 3-channel threshold hook fires **only when all three operands meet
//!   their thresholds**.
//! * A dormant operand (`Q3232::ZERO`) auto-fails the gate even when the
//!   threshold itself is also zero — the per-channel "zero operand ⇒
//!   threshold fails" rule cascades through the full pipeline.

mod common;

use beast_core::{EntityId, Q3232};
use beast_interpreter::{
    composition::{CompositionKind, EmitSpec, HookId, InterpreterHook},
    interpret_phenotype,
};

use crate::common::{build_channel_registry, build_primitive_registry, expr, phenotype};
use beast_interpreter::phenotype::{Environment, LifeStage};

fn q(v: f64) -> Q3232 {
    Q3232::from_num(v)
}

/// Helper: build a 3-channel threshold hook that emits `fire` at intensity =
/// sum of the three channel values.
fn three_channel_threshold_hook(
    registry: &beast_channels::ChannelRegistry,
    thresholds: [f64; 3],
) -> InterpreterHook {
    let sum_expr = expr("ch[a] + ch[b] + ch[c]", registry);
    InterpreterHook {
        id: HookId(1),
        kind: CompositionKind::Threshold,
        channel_ids: vec!["a".into(), "b".into(), "c".into()],
        thresholds: thresholds.iter().map(|v| q(*v)).collect(),
        coefficient: Q3232::ONE,
        expression_conditions: Vec::new(),
        emits: vec![EmitSpec {
            primitive_id: "fire".into(),
            parameter_mapping: vec![("intensity".into(), sum_expr)],
        }],
    }
}

fn world() -> (
    beast_channels::ChannelRegistry,
    beast_primitives::PrimitiveRegistry,
) {
    let creg = build_channel_registry(&[
        ("a", 0.0, 1e9, false),
        ("b", 0.0, 1e9, false),
        ("c", 0.0, 1e9, false),
    ]);
    let preg = build_primitive_registry(&[("fire", "intensity", false)]);
    (creg, preg)
}

#[test]
fn threshold_hook_fires_when_all_three_operands_exceed_thresholds() {
    let (creg, preg) = world();
    let hook = three_channel_threshold_hook(&creg, [0.2, 0.3, 0.4]);

    let p = phenotype(
        10.0,
        LifeStage::Adult,
        &[("a", 0.5), ("b", 0.5), ("c", 0.5)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        std::slice::from_ref(&hook),
        &creg,
        &preg,
        EntityId::new(1),
    )
    .unwrap();
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].primitive_id, "fire");
    // intensity = 0.5 + 0.5 + 0.5 = 1.5
    assert_eq!(effects[0].parameters["intensity"], q(1.5));
}

#[test]
fn threshold_hook_does_not_fire_when_one_operand_below_threshold() {
    let (creg, preg) = world();
    let hook = three_channel_threshold_hook(&creg, [0.2, 0.3, 0.4]);

    // Channel `b` is below its 0.3 threshold.
    let p = phenotype(
        10.0,
        LifeStage::Adult,
        &[("a", 0.5), ("b", 0.1), ("c", 0.5)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        std::slice::from_ref(&hook),
        &creg,
        &preg,
        EntityId::new(1),
    )
    .unwrap();
    assert!(
        effects.is_empty(),
        "hook fired despite operand `b` being below threshold"
    );
}

#[test]
fn threshold_hook_does_not_fire_when_two_operands_below_thresholds() {
    let (creg, preg) = world();
    let hook = three_channel_threshold_hook(&creg, [0.2, 0.3, 0.4]);

    let p = phenotype(
        10.0,
        LifeStage::Adult,
        &[("a", 0.05), ("b", 0.1), ("c", 0.5)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        std::slice::from_ref(&hook),
        &creg,
        &preg,
        EntityId::new(1),
    )
    .unwrap();
    assert!(effects.is_empty());
}

#[test]
fn threshold_hook_fires_exactly_at_thresholds() {
    // `>=` semantics: an operand equal to its threshold must pass.
    let (creg, preg) = world();
    let hook = three_channel_threshold_hook(&creg, [0.2, 0.3, 0.4]);

    let p = phenotype(
        10.0,
        LifeStage::Adult,
        &[("a", 0.2), ("b", 0.3), ("c", 0.4)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        std::slice::from_ref(&hook),
        &creg,
        &preg,
        EntityId::new(1),
    )
    .unwrap();
    assert_eq!(effects.len(), 1, "equal-to-threshold must pass (`>=` rule)");
}

/// §6.2 dormant-channel rule: any operand at `Q3232::ZERO` auto-fails, even
/// when the per-channel threshold is also zero. This is a deliberate
/// deviation from naive `value >= threshold` — dormant channels must never
/// propagate a spurious "all zero" pass.
#[test]
fn dormant_operand_auto_fails_even_when_threshold_is_zero() {
    let (creg, preg) = world();
    let hook = three_channel_threshold_hook(&creg, [0.0, 0.0, 0.0]);

    // `a` is dormant (zero value). The other two are above zero.
    let p = phenotype(
        10.0,
        LifeStage::Adult,
        &[("a", 0.0), ("b", 0.1), ("c", 0.1)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        std::slice::from_ref(&hook),
        &creg,
        &preg,
        EntityId::new(1),
    )
    .unwrap();
    assert!(
        effects.is_empty(),
        "dormant operand must auto-fail even with zero threshold (§6.2)"
    );
}

/// Complementary: no operand dormant, all thresholds zero → hook must fire
/// (every operand is `> 0` ∧ `>= 0`).
#[test]
fn all_operands_positive_with_zero_thresholds_fires() {
    let (creg, preg) = world();
    let hook = three_channel_threshold_hook(&creg, [0.0, 0.0, 0.0]);

    let p = phenotype(
        10.0,
        LifeStage::Adult,
        &[("a", 0.1), ("b", 0.1), ("c", 0.1)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        std::slice::from_ref(&hook),
        &creg,
        &preg,
        EntityId::new(1),
    )
    .unwrap();
    assert_eq!(effects.len(), 1);
}
