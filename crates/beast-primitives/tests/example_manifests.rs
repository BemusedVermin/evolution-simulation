//! Integration test: every manifest in
//! `documentation/schemas/primitive_vocabulary/` must load, validate, and
//! register. Demonstrates the Sprint 2 demo criterion "load the 16 starter
//! primitives without errors."

use std::collections::BTreeMap;

use beast_core::Q3232;
use beast_primitives::{evaluate_cost, PrimitiveCategory, PrimitiveManifest, PrimitiveRegistry};

/// `(id, source)` for every starter primitive. The tuples are sorted to keep
/// this list reviewable; the registry guarantees deterministic iteration
/// independently of this ordering.
const PRIMITIVES: &[(&str, &str)] = &[
    (
        "absorb_substance",
        include_str!("../../../documentation/schemas/primitive_vocabulary/absorb_substance.json"),
    ),
    (
        "apply_bite_force",
        include_str!("../../../documentation/schemas/primitive_vocabulary/apply_bite_force.json"),
    ),
    (
        "apply_locomotive_thrust",
        include_str!(
            "../../../documentation/schemas/primitive_vocabulary/apply_locomotive_thrust.json"
        ),
    ),
    (
        "elevate_metabolic_rate",
        include_str!(
            "../../../documentation/schemas/primitive_vocabulary/elevate_metabolic_rate.json"
        ),
    ),
    (
        "emit_acoustic_pulse",
        include_str!(
            "../../../documentation/schemas/primitive_vocabulary/emit_acoustic_pulse.json"
        ),
    ),
    (
        "emit_chemical_marker",
        include_str!(
            "../../../documentation/schemas/primitive_vocabulary/emit_chemical_marker.json"
        ),
    ),
    (
        "enter_torpor",
        include_str!("../../../documentation/schemas/primitive_vocabulary/enter_torpor.json"),
    ),
    (
        "form_host_attachment",
        include_str!(
            "../../../documentation/schemas/primitive_vocabulary/form_host_attachment.json"
        ),
    ),
    (
        "form_pair_bond",
        include_str!("../../../documentation/schemas/primitive_vocabulary/form_pair_bond.json"),
    ),
    (
        "induce_paralysis",
        include_str!("../../../documentation/schemas/primitive_vocabulary/induce_paralysis.json"),
    ),
    (
        "inject_substance",
        include_str!("../../../documentation/schemas/primitive_vocabulary/inject_substance.json"),
    ),
    (
        "receive_acoustic_signal",
        include_str!(
            "../../../documentation/schemas/primitive_vocabulary/receive_acoustic_signal.json"
        ),
    ),
    (
        "receive_photic_signal",
        include_str!(
            "../../../documentation/schemas/primitive_vocabulary/receive_photic_signal.json"
        ),
    ),
    (
        "spatial_integrate",
        include_str!("../../../documentation/schemas/primitive_vocabulary/spatial_integrate.json"),
    ),
    (
        "temporal_integrate",
        include_str!("../../../documentation/schemas/primitive_vocabulary/temporal_integrate.json"),
    ),
    (
        "thermoregulate_self",
        include_str!(
            "../../../documentation/schemas/primitive_vocabulary/thermoregulate_self.json"
        ),
    ),
];

#[test]
fn every_starter_primitive_loads() {
    assert_eq!(
        PRIMITIVES.len(),
        16,
        "expected exactly 16 starter primitives"
    );
    for (id, source) in PRIMITIVES {
        let manifest =
            PrimitiveManifest::from_json_str(source).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(manifest.id, *id);
    }
}

#[test]
fn starter_primitives_cover_all_categories() {
    let mut registry = PrimitiveRegistry::new();
    for (_, source) in PRIMITIVES {
        registry
            .register(PrimitiveManifest::from_json_str(source).unwrap())
            .unwrap();
    }

    // Schema design mandates exactly eight functional categories, and the
    // starter vocabulary dedicates two primitives to each — see
    // `documentation/schemas/README.md`.
    for category in [
        PrimitiveCategory::SignalEmission,
        PrimitiveCategory::SignalReception,
        PrimitiveCategory::ForceApplication,
        PrimitiveCategory::StateInduction,
        PrimitiveCategory::SpatialIntegration,
        PrimitiveCategory::MassTransfer,
        PrimitiveCategory::EnergyModulation,
        PrimitiveCategory::BondFormation,
    ] {
        let count = registry.ids_by_category(category).count();
        assert_eq!(count, 2, "category {category:?} should have 2 primitives");
    }
}

#[test]
fn cost_function_evaluates_with_defaults_for_every_primitive() {
    for (id, source) in PRIMITIVES {
        let manifest = PrimitiveManifest::from_json_str(source).unwrap();
        let cost = evaluate_cost(&manifest, &BTreeMap::new())
            .unwrap_or_else(|e| panic!("{id} cost eval failed: {e}"));
        assert!(cost >= Q3232::ZERO, "{id} cost should be non-negative");
    }
}
