//! Stage 1A: scale-band channel filtering (S4.1 — issue #55).
//!
//! Filters the channel values on a [`crate::ResolvedPhenotype`] so that any
//! channel whose manifest `scale_band` excludes the creature's body mass is
//! reduced to [`beast_core::Q3232::ZERO`]. This makes the dormant-channel
//! propagation rule in §6.2 (zero operand ⇒ threshold fails, zero parameter
//! ⇒ zero intensity) fall out of the arithmetic automatically.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §6.0 and
//! INVARIANTS §5 (scale-band unification).
//!
//! # Semantics
//!
//! For every registered channel, let `[min_kg, max_kg]` be its
//! `scale_band`. The comparison uses **inclusive bounds on both ends** —
//! i.e. a creature whose mass equals `min_kg` or `max_kg` exactly still
//! expresses the channel. This matches the semantics already used by
//! [`beast_channels::evaluate_expression_conditions`].
//!
//! When `body_mass_kg` falls outside the band:
//!
//! * the channel's entry in [`crate::ResolvedPhenotype::global_channels`] is
//!   reset to [`beast_core::Q3232::ZERO`] (if present);
//! * if the manifest has `body_site_applicable = true`, the channel's entry
//!   in every [`crate::BodyRegion::channel_amplitudes`] in
//!   [`crate::ResolvedPhenotype::body_map`] is also reset to
//!   [`beast_core::Q3232::ZERO`] (if present).
//!
//! When the channel is in-band, or when the phenotype simply does not
//! contain a value for that channel, the filter is a no-op for that entry.
//!
//! # Determinism
//!
//! The implementation iterates [`beast_channels::ChannelRegistry::iter`]
//! which is backed by a `BTreeMap` and therefore yields channels in sorted
//! id order. No floats are used anywhere in this module; all comparisons go
//! through [`beast_core::Q3232`]'s `PartialOrd`. The filter is idempotent:
//! applying it twice to the same `(phenotype, registry)` pair produces the
//! same result as applying it once.

use beast_channels::ChannelRegistry;
use beast_core::Q3232;

use crate::phenotype::ResolvedPhenotype;

/// Apply Stage 1A scale-band filtering to `phenotype` in place.
///
/// For every channel in `registry` whose `scale_band` excludes
/// `phenotype.body_mass_kg`, zero the channel's global value and (if the
/// manifest is `body_site_applicable`) the channel's per-region
/// amplitudes. Channels not present in the phenotype are ignored.
///
/// See the module-level documentation for the full contract.
pub fn apply_scale_band_filter(phenotype: &mut ResolvedPhenotype, registry: &ChannelRegistry) {
    let mass = phenotype.body_mass_kg;

    for (id, manifest) in registry.iter() {
        let band = &manifest.scale_band;
        let in_band = mass >= band.min_kg && mass <= band.max_kg;
        if in_band {
            continue;
        }

        if let Some(value) = phenotype.global_channels.get_mut(id) {
            *value = Q3232::ZERO;
        }

        if manifest.body_site_applicable {
            for region in &mut phenotype.body_map {
                if let Some(amplitude) = region.channel_amplitudes.get_mut(id) {
                    *amplitude = Q3232::ZERO;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phenotype::{BodyRegion, BodySite, LifeStage};
    use beast_channels::{
        BoundsPolicy, ChannelFamily, ChannelManifest, MutationKernel, Provenance, Range, ScaleBand,
    };
    use std::collections::BTreeMap;

    /// Build a bare-bones manifest with a custom scale band and
    /// `body_site_applicable` flag. All other fields take benign defaults;
    /// the scale-band filter only reads `id`, `scale_band`, and
    /// `body_site_applicable`.
    fn manifest_with_band(
        id: &str,
        min_kg: f64,
        max_kg: f64,
        body_site_applicable: bool,
    ) -> ChannelManifest {
        ChannelManifest {
            id: id.into(),
            family: ChannelFamily::Sensory,
            description: "scale-band filter fixture".into(),
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
                min_kg: Q3232::from_num(min_kg),
                max_kg: Q3232::from_num(max_kg),
            },
            body_site_applicable,
            provenance: Provenance::Core,
        }
    }

    fn registry_with(manifests: Vec<ChannelManifest>) -> ChannelRegistry {
        let mut reg = ChannelRegistry::new();
        for m in manifests {
            reg.register(m).expect("fixture ids are unique");
        }
        reg
    }

    fn phenotype_with(
        body_mass_kg: f64,
        globals: &[(&str, f64)],
        regions: Vec<BodyRegion>,
    ) -> ResolvedPhenotype {
        let mut p = ResolvedPhenotype::new(Q3232::from_num(body_mass_kg), LifeStage::Adult);
        for (id, v) in globals {
            p.global_channels
                .insert((*id).to_string(), Q3232::from_num(*v));
        }
        p.body_map = regions;
        p
    }

    fn region(id: u32, site: BodySite, amps: &[(&str, f64)]) -> BodyRegion {
        let mut channel_amplitudes = BTreeMap::new();
        for (ch, v) in amps {
            channel_amplitudes.insert((*ch).to_string(), Q3232::from_num(*v));
        }
        BodyRegion {
            id,
            body_site: site,
            surface_vs_internal: Q3232::from_num(0.5_f64),
            channel_amplitudes,
        }
    }

    // --- Acceptance-criteria unit tests ------------------------------------

    #[test]
    fn zeros_micro_only_channel_on_macro_creature() {
        // 100 kg creature, channel band [1e-15, 1e-3] kg → out of band.
        let reg = registry_with(vec![manifest_with_band(
            "host_attachment",
            1e-15,
            1e-3,
            false,
        )]);
        let mut p = phenotype_with(100.0, &[("host_attachment", 0.75)], Vec::new());

        apply_scale_band_filter(&mut p, &reg);

        assert_eq!(p.global_channels["host_attachment"], Q3232::ZERO);
    }

    #[test]
    fn zeros_macro_only_channel_on_micro_creature() {
        // 1 µg = 1e-9 kg creature, channel band [1.0, 1e9] kg → out of band.
        let reg = registry_with(vec![manifest_with_band(
            "large_neural_integration",
            1.0,
            1e9,
            false,
        )]);
        let mut p = phenotype_with(1e-9, &[("large_neural_integration", 0.5)], Vec::new());

        apply_scale_band_filter(&mut p, &reg);

        assert_eq!(p.global_channels["large_neural_integration"], Q3232::ZERO);
    }

    #[test]
    fn leaves_regulatory_channel_untouched_at_both_scales() {
        // Full-band regulatory channel [0, 1e9] → active at every mass.
        let reg = registry_with(vec![manifest_with_band(
            "immune_response_baseline",
            0.0,
            1e9,
            false,
        )]);

        let mut micro = phenotype_with(1e-9, &[("immune_response_baseline", 0.3)], Vec::new());
        apply_scale_band_filter(&mut micro, &reg);
        assert_eq!(
            micro.global_channels["immune_response_baseline"],
            Q3232::from_num(0.3_f64)
        );

        let mut macro_ = phenotype_with(100.0, &[("immune_response_baseline", 0.8)], Vec::new());
        apply_scale_band_filter(&mut macro_, &reg);
        assert_eq!(
            macro_.global_channels["immune_response_baseline"],
            Q3232::from_num(0.8_f64)
        );
    }

    #[test]
    fn zeros_body_site_amplitudes_when_out_of_band() {
        // 1 µg creature, macro-only bite_force [1.0, 1e9] → all regions zero.
        let reg = registry_with(vec![manifest_with_band("bite_force", 1.0, 1e9, true)]);
        let mut p = phenotype_with(
            1e-9,
            &[("bite_force", 0.6)],
            vec![
                region(0, BodySite::Head, &[("bite_force", 0.7)]),
                region(1, BodySite::Jaw, &[("bite_force", 0.9)]),
                region(2, BodySite::LimbLeft, &[("bite_force", 0.4)]),
            ],
        );

        apply_scale_band_filter(&mut p, &reg);

        assert_eq!(p.global_channels["bite_force"], Q3232::ZERO);
        for r in &p.body_map {
            assert_eq!(
                r.channel_amplitudes["bite_force"],
                Q3232::ZERO,
                "region {} should be zeroed",
                r.id
            );
        }
    }

    #[test]
    fn leaves_body_site_amplitudes_when_in_band() {
        // Macro-only channel on a macro creature → nothing is touched.
        let reg = registry_with(vec![manifest_with_band("bite_force", 1.0, 1e9, true)]);
        let mut p = phenotype_with(
            50.0,
            &[("bite_force", 0.6)],
            vec![
                region(0, BodySite::Head, &[("bite_force", 0.7)]),
                region(1, BodySite::Jaw, &[("bite_force", 0.9)]),
            ],
        );
        let before = p.clone();

        apply_scale_band_filter(&mut p, &reg);

        assert_eq!(p.global_channels, before.global_channels);
        for (a, b) in p.body_map.iter().zip(before.body_map.iter()) {
            assert_eq!(a.channel_amplitudes, b.channel_amplitudes);
        }
    }

    #[test]
    fn does_not_touch_body_map_for_non_body_site_channel() {
        // Out-of-band global channel with body_site_applicable=false:
        // regions that happen to carry this channel id are left alone.
        let reg = registry_with(vec![manifest_with_band("metabolic_rate", 1.0, 1e9, false)]);
        let mut p = phenotype_with(
            1e-9,
            &[("metabolic_rate", 0.5)],
            vec![region(0, BodySite::Core, &[("metabolic_rate", 0.5_f64)])],
        );

        apply_scale_band_filter(&mut p, &reg);

        assert_eq!(p.global_channels["metabolic_rate"], Q3232::ZERO);
        // body_site_applicable=false ⇒ per-region amplitude is *not* zeroed.
        assert_eq!(
            p.body_map[0].channel_amplitudes["metabolic_rate"],
            Q3232::from_num(0.5_f64)
        );
    }

    #[test]
    fn channel_absent_from_phenotype_is_noop() {
        // Registry has channels the phenotype never expressed — must not
        // panic and must not insert new keys.
        let reg = registry_with(vec![
            manifest_with_band("host_attachment", 1e-15, 1e-3, false),
            manifest_with_band("bite_force", 1.0, 1e9, true),
        ]);
        let mut p = phenotype_with(100.0, &[], vec![region(0, BodySite::Head, &[])]);
        let before = p.clone();

        apply_scale_band_filter(&mut p, &reg);

        assert!(p.global_channels.is_empty());
        assert_eq!(
            p.global_channels.len(),
            before.global_channels.len(),
            "no keys were inserted"
        );
        assert!(p.body_map[0].channel_amplitudes.is_empty());
    }

    #[test]
    fn inclusive_bounds_at_band_edges() {
        // A creature sitting exactly on min_kg or max_kg expresses the
        // channel — the spec treats both bounds as inclusive.
        let reg = registry_with(vec![manifest_with_band("edge_ch", 1.0, 10.0, false)]);

        let mut low = phenotype_with(1.0, &[("edge_ch", 0.5)], Vec::new());
        apply_scale_band_filter(&mut low, &reg);
        assert_eq!(low.global_channels["edge_ch"], Q3232::from_num(0.5_f64));

        let mut high = phenotype_with(10.0, &[("edge_ch", 0.9)], Vec::new());
        apply_scale_band_filter(&mut high, &reg);
        assert_eq!(high.global_channels["edge_ch"], Q3232::from_num(0.9_f64));
    }

    #[test]
    fn empty_registry_is_noop() {
        let reg = ChannelRegistry::new();
        let mut p = phenotype_with(
            42.0,
            &[("a", 0.1), ("b", 0.2)],
            vec![region(0, BodySite::Head, &[("a", 0.3)])],
        );
        let before = p.clone();

        apply_scale_band_filter(&mut p, &reg);

        assert_eq!(p.global_channels, before.global_channels);
        assert_eq!(
            p.body_map[0].channel_amplitudes,
            before.body_map[0].channel_amplitudes
        );
    }

    #[test]
    fn filter_is_idempotent_on_fixed_inputs() {
        // Running the filter twice must produce the same state as running
        // it once. Covered here with a fixed fixture; proptest below fuzzes
        // this more broadly.
        let reg = registry_with(vec![
            manifest_with_band("micro_only", 1e-15, 1e-3, false),
            manifest_with_band("macro_only", 1.0, 1e9, true),
            manifest_with_band("always_on", 0.0, 1e9, false),
        ]);
        let mut once = phenotype_with(
            100.0,
            &[("micro_only", 0.5), ("macro_only", 0.7), ("always_on", 0.3)],
            vec![region(
                0,
                BodySite::Jaw,
                &[("macro_only", 0.8_f64), ("always_on", 0.4_f64)],
            )],
        );
        let mut twice = once.clone();

        apply_scale_band_filter(&mut once, &reg);
        apply_scale_band_filter(&mut twice, &reg);
        apply_scale_band_filter(&mut twice, &reg);

        assert_eq!(once.global_channels, twice.global_channels);
        for (a, b) in once.body_map.iter().zip(twice.body_map.iter()) {
            assert_eq!(a.channel_amplitudes, b.channel_amplitudes);
        }
    }

    // --- Property test: idempotence ----------------------------------------

    use proptest::prelude::*;

    /// A compact spec for fuzzed channels: id, band bounds in kg, and whether
    /// the channel is body-site applicable. We generate masses and
    /// channel values as integers to stay well inside Q32.32's exactly
    /// representable range while still exercising a wide dynamic range.
    fn arb_channel_spec() -> impl Strategy<Value = (String, i64, i64, bool)> {
        // Deterministic id pool — duplicates are filtered at registration.
        let id = prop_oneof![
            Just("alpha".to_string()),
            Just("beta".to_string()),
            Just("gamma".to_string()),
            Just("delta".to_string()),
            Just("epsilon".to_string()),
        ];
        // min_kg ∈ [0, 1e6], span ∈ [0, 1e6] → max_kg = min + span.
        (id, 0_i64..1_000_000, 0_i64..1_000_000, any::<bool>())
            .prop_map(|(id, min, span, body_site)| (id, min, min.saturating_add(span), body_site))
    }

    proptest! {
        /// Applying the filter twice must equal applying it once, for any
        /// combination of (body_mass, channels, per-region amplitudes).
        #[test]
        fn apply_scale_band_filter_is_idempotent(
            body_mass in 0_i64..10_000_000,
            specs in proptest::collection::vec(arb_channel_spec(), 0..6),
            global_values in proptest::collection::vec(0_i64..1000, 0..6),
            region_values in proptest::collection::vec(0_i64..1000, 0..12),
        ) {
            let mut reg = ChannelRegistry::new();
            for (id, min_kg, max_kg, body_site) in &specs {
                // ignore duplicate-id failures — the fuzzer is allowed to
                // propose colliding ids.
                let _ = reg.register(manifest_with_band(
                    id,
                    *min_kg as f64,
                    *max_kg as f64,
                    *body_site,
                ));
            }

            let ids: Vec<String> = reg.ids().map(str::to_string).collect();
            let mut p = ResolvedPhenotype::new(
                Q3232::from_num(body_mass),
                LifeStage::Adult,
            );
            for (i, id) in ids.iter().enumerate() {
                if let Some(v) = global_values.get(i) {
                    p.global_channels.insert(id.clone(), Q3232::from_num(*v));
                }
            }
            // Two regions with whatever channel amplitudes the fuzzer draws.
            for r_idx in 0..2 {
                let mut amps = BTreeMap::new();
                for (i, id) in ids.iter().enumerate() {
                    let slot = r_idx * ids.len() + i;
                    if let Some(v) = region_values.get(slot) {
                        amps.insert(id.clone(), Q3232::from_num(*v));
                    }
                }
                p.body_map.push(BodyRegion {
                    id: r_idx as u32,
                    body_site: BodySite::Core,
                    surface_vs_internal: Q3232::ZERO,
                    channel_amplitudes: amps,
                });
            }

            let mut once = p.clone();
            let mut twice = p;
            apply_scale_band_filter(&mut once, &reg);
            apply_scale_band_filter(&mut twice, &reg);
            apply_scale_band_filter(&mut twice, &reg);

            prop_assert_eq!(&once.global_channels, &twice.global_channels);
            prop_assert_eq!(once.body_map.len(), twice.body_map.len());
            for (a, b) in once.body_map.iter().zip(twice.body_map.iter()) {
                prop_assert_eq!(&a.channel_amplitudes, &b.channel_amplitudes);
            }
        }
    }
}
