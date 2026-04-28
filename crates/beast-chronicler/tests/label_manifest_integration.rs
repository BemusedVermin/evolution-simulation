//! End-to-end test for the on-disk label manifest at
//! `assets/manifests/labels.json`. Ensures the schema, manifest, and
//! engine all line up — a regression here likely means a manifest edit
//! broke schema conformance or vice versa.

use std::collections::BTreeSet;

use beast_chronicler::{Chronicler, LabelEngine, PrimitiveSnapshot};
use beast_core::{EntityId, TickCounter};

const ASSET_LABELS_JSON: &str = include_str!("../../../assets/manifests/labels.json");

/// All ids declared in the shipped manifest. Hard-coded *here* (a test
/// file) per the S10.6 DoD which carves out tests + `assets/` from the
/// no-hardcoded-label-name rule.
const SHIPPED_LABEL_IDS: &[&str] = &["echolocation", "bioluminescence", "pack_hunting"];

fn snap(tick: u64, entity: u32, primitives: &[&str]) -> PrimitiveSnapshot {
    PrimitiveSnapshot::new(
        TickCounter::new(tick),
        EntityId::new(entity),
        primitives.iter().copied(),
    )
}

#[test]
fn shipped_manifest_loads_and_matches_expected_label_set() {
    let engine = LabelEngine::from_json_str(ASSET_LABELS_JSON)
        .expect("shipped label manifest must load cleanly");
    assert_eq!(
        engine.len(),
        SHIPPED_LABEL_IDS.len(),
        "shipped manifest entry count drifted; sync SHIPPED_LABEL_IDS in this test"
    );
}

#[test]
fn shipped_manifest_assigns_echolocation_under_full_frequency() {
    let engine = LabelEngine::from_json_str(ASSET_LABELS_JSON).unwrap();
    let mut chronicler = Chronicler::new();
    // Drive 100 ticks of a single creature emitting the echolocation
    // primitive set — full frequency, full stability.
    for tick in 0..100 {
        chronicler.ingest(&snap(
            tick,
            1,
            &[
                "emit_acoustic_pulse",
                "receive_acoustic_signal",
                "spatial_integrate",
            ],
        ));
    }
    chronicler.assign_labels(&engine, chronicler.last_observed_tick());
    let assigned: BTreeSet<&str> = chronicler
        .labels()
        .values()
        .map(|l| l.id.as_str())
        .collect();
    assert!(
        assigned.contains("echolocation"),
        "echolocation must label the recurring acoustic pattern; got {assigned:?}"
    );
}

#[test]
fn shipped_manifest_suppresses_pack_hunting_below_threshold() {
    // pack_hunting has min_confidence = 0.7 in the shipped manifest.
    // A single observation in a 1000-observation chronicler scores
    // ~0.6 * (1/1000) = 0.0006 — well below the 0.7 floor.
    let engine = LabelEngine::from_json_str(ASSET_LABELS_JSON).unwrap();
    let mut chronicler = Chronicler::new();
    for tick in 0..999 {
        chronicler.ingest(&snap(tick, 1, &["emit_chemical_marker"]));
    }
    chronicler.ingest(&snap(
        999,
        1,
        &[
            "form_pair_bond",
            "apply_bite_force",
            "apply_locomotive_thrust",
        ],
    ));
    chronicler.assign_labels(&engine, chronicler.last_observed_tick());
    let label_ids: BTreeSet<&str> = chronicler
        .labels()
        .values()
        .map(|l| l.id.as_str())
        .collect();
    assert!(
        !label_ids.contains("pack_hunting"),
        "pack_hunting should not stabilize from a single observation"
    );
}

#[test]
fn label_assignment_is_deterministic_across_ingestion_order() {
    // Two chroniclers built with reversed ingestion order must produce
    // bit-identical label indexes after assignment — INVARIANTS §1.
    let engine = LabelEngine::from_json_str(ASSET_LABELS_JSON).unwrap();
    let snapshots: Vec<PrimitiveSnapshot> = (0..200)
        .map(|i| {
            let primitives: &[&str] = if i % 3 == 0 {
                &[
                    "emit_acoustic_pulse",
                    "receive_acoustic_signal",
                    "spatial_integrate",
                ]
            } else if i % 3 == 1 {
                &["emit_chemical_marker", "elevate_metabolic_rate"]
            } else {
                &["emit_chemical_marker"]
            };
            snap(i, (i % 5) as u32, primitives)
        })
        .collect();

    let mut forward = Chronicler::new();
    let mut reverse = Chronicler::new();
    for s in &snapshots {
        forward.ingest(s);
    }
    for s in snapshots.iter().rev() {
        reverse.ingest(s);
    }
    let tick = forward.last_observed_tick();
    forward.assign_labels(&engine, tick);
    reverse.assign_labels(&engine, tick);

    let collect = |c: &Chronicler| -> Vec<(String, i64)> {
        c.labels()
            .values()
            .map(|l| (l.id.clone(), l.confidence.to_bits()))
            .collect()
    };
    assert_eq!(collect(&forward), collect(&reverse));
}
