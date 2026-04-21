//! Shared test fixtures for the `parameter_map` sub-modules.
//!
//! Kept here (rather than duplicated across the three child modules) so
//! the parser, evaluator, and analysis test suites all exercise identical
//! channel-manifest shapes.

use std::collections::BTreeMap;

use beast_channels::{
    BoundsPolicy, ChannelFamily, ChannelManifest, ChannelRegistry, MutationKernel, Provenance,
    Range, ScaleBand,
};
use beast_core::Q3232;

pub(crate) fn manifest(id: &str) -> ChannelManifest {
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
        provenance: Provenance::Core,
    }
}

pub(crate) fn registry_with(ids: &[&str]) -> ChannelRegistry {
    let mut reg = ChannelRegistry::new();
    for id in ids {
        reg.register(manifest(id)).expect("unique fixture ids");
    }
    reg
}

pub(crate) fn channels(pairs: &[(&str, Q3232)]) -> BTreeMap<String, Q3232> {
    pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
}
