//! Scale-band acceptance fixtures (S4.6 — issue #60).
//!
//! Three end-to-end scenarios per
//! `documentation/systems/11_phenotype_interpreter.md` §6.0 and the S4 epic
//! demo criteria:
//!
//! * **Fixture A**: macro creature (100 kg) + micro-only channel. Channel
//!   value is zeroed; no dependent primitive fires.
//! * **Fixture B**: micro creature (1 µg) + macro-only channel. Same in reverse.
//! * **Fixture C**: 500 g creature + universal regulatory channel. Channel
//!   value is preserved; dependent primitives fire normally.

mod common;

use beast_core::{EntityId, Q3232};
use beast_interpreter::interpret_phenotype;

use crate::common::{
    additive_hook, build_channel_registry, build_primitive_registry, expr, phenotype,
    threshold_hook,
};
use beast_interpreter::composition::InterpreterHook;
use beast_interpreter::phenotype::{Environment, LifeStage};

/// Fixture A — 100 kg macro creature + micro-only (1e-15..1e-3 kg) channel.
/// The scale-band filter must zero that channel's global value and the
/// dependent threshold hook must therefore not fire (dormant-channel
/// auto-fail, §6.2).
#[test]
fn fixture_a_macro_creature_zeros_micro_only_channel() {
    let channel_registry = build_channel_registry(&[
        ("host_attachment", 1e-15, 1e-3, false),
        ("universal", 0.0, 1e9, false),
    ]);
    let primitive_registry =
        build_primitive_registry(&[("p_attach", "strength", false), ("p_signal", "mag", false)]);

    let e_host = expr("ch[host_attachment]", &channel_registry);
    let e_universal = expr("ch[universal]", &channel_registry);

    let hooks: Vec<InterpreterHook> = vec![
        // This threshold hook depends on the micro-only channel; should
        // auto-fail once the scale-band filter zeros it.
        threshold_hook(
            1,
            &["host_attachment"],
            &[Q3232::from_num(0.1_f64)],
            "p_attach",
            vec![("strength", e_host)],
        ),
        // Control: a universal-scale channel hook that MUST fire so the test
        // proves the pipeline is alive.
        additive_hook(2, &["universal"], "p_signal", vec![("mag", e_universal)]),
    ];

    let p = phenotype(
        100.0,
        LifeStage::Adult,
        &[("host_attachment", 0.8), ("universal", 0.5)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        &hooks,
        &channel_registry,
        &primitive_registry,
        EntityId::new(1),
    )
    .unwrap();

    // `p_attach` never fires — micro-only channel was zeroed by scale-band.
    assert!(
        effects.iter().all(|e| e.primitive_id != "p_attach"),
        "p_attach fired despite micro-only channel being out-of-band"
    );
    // `p_signal` fires from the universal control.
    assert!(
        effects.iter().any(|e| e.primitive_id == "p_signal"),
        "p_signal (universal control) did not fire — pipeline wiring broken"
    );
}

/// Fixture B — 1 µg (1e-9 kg) micro creature + macro-only (1.0..1e9 kg)
/// channel. Mirror of fixture A.
#[test]
fn fixture_b_micro_creature_zeros_macro_only_channel() {
    let channel_registry = build_channel_registry(&[
        ("large_neural_integration", 1.0, 1e9, false),
        ("universal", 0.0, 1e9, false),
    ]);
    let primitive_registry =
        build_primitive_registry(&[("p_neural", "intensity", false), ("p_signal", "mag", false)]);

    let e_neural = expr("ch[large_neural_integration]", &channel_registry);
    let e_universal = expr("ch[universal]", &channel_registry);

    let hooks: Vec<InterpreterHook> = vec![
        threshold_hook(
            1,
            &["large_neural_integration"],
            &[Q3232::from_num(0.1_f64)],
            "p_neural",
            vec![("intensity", e_neural)],
        ),
        additive_hook(2, &["universal"], "p_signal", vec![("mag", e_universal)]),
    ];

    let p = phenotype(
        1e-9,
        LifeStage::Adult,
        &[("large_neural_integration", 0.9), ("universal", 0.5)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        &hooks,
        &channel_registry,
        &primitive_registry,
        EntityId::new(1),
    )
    .unwrap();

    assert!(
        effects.iter().all(|e| e.primitive_id != "p_neural"),
        "p_neural fired despite macro-only channel being out-of-band"
    );
    assert!(
        effects.iter().any(|e| e.primitive_id == "p_signal"),
        "p_signal (universal control) did not fire"
    );
}

/// Fixture C — 500 g (0.5 kg) creature + universal (0..1e9 kg) regulatory
/// channel. Channel value must be preserved; dependent primitives fire.
#[test]
fn fixture_c_universal_channel_preserved_at_every_scale() {
    let channel_registry = build_channel_registry(&[("immune_response_baseline", 0.0, 1e9, false)]);
    let primitive_registry = build_primitive_registry(&[("p_immune", "baseline", false)]);

    let e_immune = expr("ch[immune_response_baseline]", &channel_registry);
    let hooks = vec![threshold_hook(
        1,
        &["immune_response_baseline"],
        &[Q3232::from_num(0.05_f64)],
        "p_immune",
        vec![("baseline", e_immune)],
    )];

    let p = phenotype(
        0.5,
        LifeStage::Adult,
        &[("immune_response_baseline", 0.3)],
        Environment::default(),
    );

    let effects = interpret_phenotype(
        &p,
        &hooks,
        &channel_registry,
        &primitive_registry,
        EntityId::new(1),
    )
    .unwrap();

    assert_eq!(effects.len(), 1, "exactly one effect should fire");
    assert_eq!(effects[0].primitive_id, "p_immune");
    // `immune_response_baseline` was 0.3 — must be preserved through the
    // scale-band stage and returned in the effect's parameter.
    assert_eq!(
        effects[0].parameters["baseline"],
        Q3232::from_num(0.3_f64),
        "universal channel value must be preserved at 500 g",
    );
}

/// Extra: proves that the same fixture world applied to both a macro and a
/// micro creature yields different effect sets — that is, the scale-band
/// filter is actually gating behaviour on body mass, not just a no-op.
#[test]
fn macro_and_micro_creatures_get_different_effect_sets() {
    let channel_registry = build_channel_registry(&[
        ("macro_only", 1.0, 1e9, false),
        ("micro_only", 1e-15, 1e-3, false),
    ]);
    let primitive_registry =
        build_primitive_registry(&[("p_macro", "val", false), ("p_micro", "val", false)]);

    let e_macro = expr("ch[macro_only]", &channel_registry);
    let e_micro = expr("ch[micro_only]", &channel_registry);
    let hooks = vec![
        threshold_hook(
            1,
            &["macro_only"],
            &[Q3232::from_num(0.1_f64)],
            "p_macro",
            vec![("val", e_macro)],
        ),
        threshold_hook(
            2,
            &["micro_only"],
            &[Q3232::from_num(0.1_f64)],
            "p_micro",
            vec![("val", e_micro)],
        ),
    ];

    let macro_p = phenotype(
        100.0,
        LifeStage::Adult,
        &[("macro_only", 0.8), ("micro_only", 0.8)],
        Environment::default(),
    );
    let micro_p = phenotype(
        1e-9,
        LifeStage::Adult,
        &[("macro_only", 0.8), ("micro_only", 0.8)],
        Environment::default(),
    );

    let macro_effects = interpret_phenotype(
        &macro_p,
        &hooks,
        &channel_registry,
        &primitive_registry,
        EntityId::new(1),
    )
    .unwrap();
    let micro_effects = interpret_phenotype(
        &micro_p,
        &hooks,
        &channel_registry,
        &primitive_registry,
        EntityId::new(1),
    )
    .unwrap();

    let macro_ids: Vec<&str> = macro_effects
        .iter()
        .map(|e| e.primitive_id.as_str())
        .collect();
    let micro_ids: Vec<&str> = micro_effects
        .iter()
        .map(|e| e.primitive_id.as_str())
        .collect();
    assert_eq!(macro_ids, vec!["p_macro"]);
    assert_eq!(micro_ids, vec!["p_micro"]);
}
