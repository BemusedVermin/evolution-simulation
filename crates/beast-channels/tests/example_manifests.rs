//! Integration test: the five worked examples in
//! `documentation/schemas/examples/` must load, validate, and register.
//!
//! This doubles as the "demo criterion" from Sprint 2: load the core channel
//! set without errors and confirm the registry queries return expected
//! entries.

use beast_channels::{ChannelFamily, ChannelManifest, ChannelRegistry};

const EXAMPLES: &[(&str, &str)] = &[
    (
        "auditory_sensitivity",
        include_str!("../../../documentation/schemas/examples/auditory_sensitivity.json"),
    ),
    (
        "host_coupling",
        include_str!("../../../documentation/schemas/examples/host_coupling.json"),
    ),
    (
        "kinetic_force",
        include_str!("../../../documentation/schemas/examples/kinetic_force.json"),
    ),
    (
        "structural_rigidity",
        include_str!("../../../documentation/schemas/examples/structural_rigidity.json"),
    ),
    (
        "vocal_modulation",
        include_str!("../../../documentation/schemas/examples/vocal_modulation.json"),
    ),
];

#[test]
fn every_example_manifest_loads() {
    for (id, source) in EXAMPLES {
        let manifest =
            ChannelManifest::from_json_str(source).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(manifest.id, *id, "filename and id must match");
    }
}

#[test]
fn example_manifests_register_and_iterate_sorted() {
    let mut registry = ChannelRegistry::new();
    for (_, source) in EXAMPLES {
        let manifest = ChannelManifest::from_json_str(source).unwrap();
        registry.register(manifest).unwrap();
    }
    assert_eq!(registry.len(), EXAMPLES.len());

    let ids: Vec<&str> = registry.ids().collect();
    let mut expected: Vec<&str> = EXAMPLES.iter().map(|(id, _)| *id).collect();
    expected.sort_unstable();
    assert_eq!(ids, expected);
}

#[test]
fn family_queries_return_expected_members() {
    let mut registry = ChannelRegistry::new();
    for (_, source) in EXAMPLES {
        let manifest = ChannelManifest::from_json_str(source).unwrap();
        registry.register(manifest).unwrap();
    }

    // Known from the worked examples: auditory_sensitivity is Sensory;
    // vocal_modulation and kinetic_force are Motor; structural_rigidity is
    // Structural; host_coupling is Social.
    let sensory: Vec<&str> = registry.ids_by_family(ChannelFamily::Sensory).collect();
    assert_eq!(sensory, vec!["auditory_sensitivity"]);

    let motor: Vec<&str> = registry.ids_by_family(ChannelFamily::Motor).collect();
    assert_eq!(motor, vec!["kinetic_force", "vocal_modulation"]);

    let structural: Vec<&str> = registry.ids_by_family(ChannelFamily::Structural).collect();
    assert_eq!(structural, vec!["structural_rigidity"]);

    let social: Vec<&str> = registry.ids_by_family(ChannelFamily::Social).collect();
    assert_eq!(social, vec!["host_coupling"]);
}
