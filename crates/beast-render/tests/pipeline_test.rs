//! S9.5 acceptance tests for the visual pipeline.
//!
//! These tests don't go through `beast-interpreter` — that crate's
//! channel-registry machinery is overkill for what we need to lock down
//! here. Instead they construct [`ResolvedPhenotype`] fixtures by hand
//! that exercise the channel patterns the pipeline reads.
//!
//! What's covered:
//!
//! * **Determinism** — two compiles of the same input produce
//!   byte-identical hashes (INVARIANTS §1).
//! * **Structural validity** — every phenotype profile produces a
//!   non-empty blueprint with at least one Core bone, at least one
//!   volume, every surface-detail target volume id resolves, every
//!   material target volume id resolves, every effect attach point
//!   resolves.
//! * **Mechanics-label separation (INVARIANTS §2)** — no
//!   species-specific name appears in the pipeline's output unless the
//!   caller supplied it via `display_name`. The pipeline never invents
//!   a name.
//! * **All directive variants exercised** — one fixture invokes every
//!   `DirectiveParams::*` variant.

use std::collections::{BTreeMap, BTreeSet};

use beast_core::{BodySite, Q3232};
use beast_interpreter::{LifeStage, ResolvedPhenotype};
use beast_render::blueprint::{AttachPoint, BoneTag, MaterialTarget};
use beast_render::directive::{
    Append, AppendageKind, ColorSpec, Colorize, DirectiveParams, Distribution, Harden,
    HardenPattern, Inflate, Orifice, OrificeOrientation, Protrude, ProtrusionShape, Soften,
    SurfaceRegion, Texture, TexturePattern, VisualDirective,
};
use beast_render::{compile_blueprint, CreatureBlueprint};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn channel(name: &str, value: f64) -> (String, Q3232) {
    (name.to_string(), Q3232::from_num(value))
}

fn phenotype_with(channels: &[(&str, f64)]) -> ResolvedPhenotype {
    let mut p = ResolvedPhenotype::new(Q3232::from_num(50_i32), LifeStage::Adult);
    let map: BTreeMap<String, Q3232> = channels
        .iter()
        .map(|(name, value)| channel(name, *value))
        .collect();
    p.global_channels = map;
    p
}

fn elastic_worm() -> ResolvedPhenotype {
    phenotype_with(&[
        ("elastic_deformation", 0.8),
        ("structural_rigidity", 0.1),
        ("mass_density", 0.4),
        ("metabolic_rate", 0.3),
        ("surface_friction", 0.2),
        ("kinetic_force", 0.2),
    ])
}

fn rigid_armored() -> ResolvedPhenotype {
    phenotype_with(&[
        ("elastic_deformation", 0.1),
        ("structural_rigidity", 0.85),
        ("mass_density", 0.7),
        ("metabolic_rate", 0.4),
        ("surface_friction", 0.6),
        ("kinetic_force", 0.5),
    ])
}

fn luminous_glow() -> ResolvedPhenotype {
    phenotype_with(&[
        ("elastic_deformation", 0.3),
        ("structural_rigidity", 0.3),
        ("mass_density", 0.5),
        ("metabolic_rate", 0.4),
        ("surface_friction", 0.4),
        ("light_emission", 0.7),
        ("thermal_output", 0.6),
        ("chemical_output", 0.5),
    ])
}

fn neutral_biome_color() -> ColorSpec {
    ColorSpec::rgb(
        Q3232::from_num(120_i32),
        Q3232::from_num(0.4_f64),
        Q3232::from_num(0.5_f64),
    )
}

fn protrude_dorsal_spikes() -> VisualDirective {
    VisualDirective {
        id: 1,
        body_region: BodySite::Core,
        params: DirectiveParams::Protrude(Protrude {
            shape: ProtrusionShape::Spike,
            scale: Q3232::from_num(0.4_f64),
            density: 6,
            distribution: Distribution::Regular,
            surface_region: SurfaceRegion::Dorsal,
        }),
        priority: 10,
    }
}

fn colorize_biome_camo() -> VisualDirective {
    VisualDirective {
        id: 2,
        body_region: BodySite::Core,
        params: DirectiveParams::Colorize(Colorize {
            base_color: ColorSpec::biome_color(Q3232::from_num(0.6_f64), Q3232::from_num(0.5_f64)),
            emission: None,
            emission_intensity: Q3232::ZERO,
            pattern: Some(TexturePattern::Mottled),
            pattern_color_secondary: None,
            contrast: Q3232::from_num(0.4_f64),
        }),
        priority: 5,
    }
}

fn one_of_every_directive() -> Vec<VisualDirective> {
    vec![
        protrude_dorsal_spikes(),
        VisualDirective {
            id: 2,
            body_region: BodySite::Core,
            params: DirectiveParams::Harden(Harden {
                roughness: Q3232::from_num(0.6_f64),
                segmentation: 8,
                pattern: HardenPattern::Plates,
            }),
            priority: 8,
        },
        VisualDirective {
            id: 3,
            body_region: BodySite::Head,
            params: DirectiveParams::Soften(Soften {
                smoothness: Q3232::from_num(0.6_f64),
                transparency: Q3232::from_num(0.2_f64),
            }),
            priority: 5,
        },
        VisualDirective {
            id: 4,
            body_region: BodySite::Head,
            params: DirectiveParams::Orifice(Orifice {
                size: Q3232::from_num(0.2_f64),
                position: Q3232::from_num(0.1_f64),
                count: 1,
                orientation: OrificeOrientation::Forward,
            }),
            priority: 6,
        },
        VisualDirective {
            id: 5,
            body_region: BodySite::Core,
            params: DirectiveParams::Append(Append {
                appendage_type: AppendageKind::Crest,
                count: 2,
                position: Q3232::from_num(0.3_f64),
            }),
            priority: 3,
        },
        VisualDirective {
            id: 6,
            body_region: BodySite::Core,
            params: DirectiveParams::Inflate(Inflate {
                scale: Q3232::from_num(1.2_f64),
            }),
            priority: 7,
        },
        VisualDirective {
            id: 7,
            body_region: BodySite::Core,
            params: DirectiveParams::Texture(Texture {
                pattern: TexturePattern::Striped,
                scale: Q3232::from_num(0.5_f64),
            }),
            priority: 4,
        },
        colorize_biome_camo(),
    ]
}

// ---------------------------------------------------------------------------
// Structural-validity helpers
// ---------------------------------------------------------------------------

fn assert_structurally_valid(blueprint: &CreatureBlueprint) {
    // At least one Core bone.
    let has_core = blueprint
        .skeleton
        .bones
        .iter()
        .any(|b| b.tags.contains(&BoneTag::Core));
    assert!(
        has_core,
        "blueprint has no Core bone: {:#?}",
        blueprint.skeleton
    );

    // Volumes non-empty and every volume's attached_bones reference real bone ids.
    assert!(!blueprint.volumes.is_empty(), "blueprint has no volumes");
    let bone_ids: BTreeSet<u32> = blueprint.skeleton.bones.iter().map(|b| b.id).collect();
    for vol in &blueprint.volumes {
        for bone_id in &vol.attached_bones {
            assert!(
                bone_ids.contains(bone_id),
                "volume {} references bone {} not present in skeleton",
                vol.id,
                bone_id
            );
        }
    }

    // Every surface detail's target_volume must be a real volume id.
    let volume_ids: BTreeSet<u32> = blueprint.volumes.iter().map(|v| v.id).collect();
    for s in &blueprint.surfaces {
        assert!(
            volume_ids.contains(&s.target_volume),
            "surface detail {} targets volume {} but no such volume exists",
            s.id,
            s.target_volume
        );
    }

    // Every material target resolves to a real id (or is Global).
    let detail_ids: BTreeSet<u32> = blueprint.surfaces.iter().map(|s| s.id).collect();
    for m in &blueprint.materials {
        match &m.target {
            MaterialTarget::Global => {}
            MaterialTarget::Volume { volume_id } => {
                assert!(
                    volume_ids.contains(volume_id),
                    "material {} targets volume {} which doesn't exist",
                    m.id,
                    volume_id
                );
            }
            MaterialTarget::Detail { detail_id } => {
                assert!(
                    detail_ids.contains(detail_id),
                    "material {} targets detail {} which doesn't exist",
                    m.id,
                    detail_id
                );
            }
        }
    }

    // Effects: attach points resolve.
    for e in &blueprint.effects {
        match &e.attach {
            AttachPoint::Region { .. } | AttachPoint::Aura { .. } => {}
            AttachPoint::Volume { volume_id } => {
                assert!(
                    volume_ids.contains(volume_id),
                    "effect {} attaches to volume {} which doesn't exist",
                    e.id,
                    volume_id
                );
            }
            AttachPoint::Detail { detail_id } => {
                assert!(
                    detail_ids.contains(detail_id),
                    "effect {} attaches to detail {} which doesn't exist",
                    e.id,
                    detail_id
                );
            }
        }
    }

    // Bounding box non-degenerate.
    assert!(
        blueprint.metadata.bounding_box.max.x > blueprint.metadata.bounding_box.min.x,
        "bounding box has zero or negative x extent: {:?}",
        blueprint.metadata.bounding_box
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn elastic_worm_compiles_to_valid_blueprint() {
    let bp = compile_blueprint(
        &elastic_worm(),
        &[colorize_biome_camo()],
        neutral_biome_color(),
        "elastic_worm",
    );
    assert_structurally_valid(&bp);
    // Elastic phenotype should have the maximum segment count (~ 8).
    let core_count = bp
        .skeleton
        .bones
        .iter()
        .filter(|b| b.tags.contains(&BoneTag::Core))
        .count();
    assert!(
        core_count >= 6,
        "elastic worm should have many core segments, got {core_count}"
    );
}

#[test]
fn rigid_armored_compiles_to_valid_blueprint() {
    let bp = compile_blueprint(
        &rigid_armored(),
        &[protrude_dorsal_spikes()],
        neutral_biome_color(),
        "rigid_armored",
    );
    assert_structurally_valid(&bp);
    // Rigid phenotype should have a small segment count.
    let core_count = bp
        .skeleton
        .bones
        .iter()
        .filter(|b| b.tags.contains(&BoneTag::Core))
        .count();
    assert!(
        core_count <= 4,
        "rigid armored should have few core segments, got {core_count}"
    );
    // Spike directive should produce one Protrusion surface detail.
    let protrusion_count = bp
        .surfaces
        .iter()
        .filter(|s| {
            matches!(
                s.detail,
                beast_render::blueprint::SurfaceType::Protrusion { .. }
            )
        })
        .count();
    assert_eq!(
        protrusion_count, 1,
        "expected exactly one Protrusion detail"
    );
}

#[test]
fn luminous_glow_attaches_glow_and_thermal_effects() {
    let bp = compile_blueprint(
        &luminous_glow(),
        &[],
        neutral_biome_color(),
        "luminous_glow",
    );
    assert_structurally_valid(&bp);
    let kinds: Vec<_> = bp.effects.iter().map(|e| e.spec.kind).collect();
    assert!(
        kinds.contains(&beast_render::blueprint::EffectKind::Glow),
        "expected Glow effect, got {kinds:?}"
    );
    assert!(
        kinds.contains(&beast_render::blueprint::EffectKind::Particle),
        "expected Particle (thermal) effect, got {kinds:?}"
    );
    assert!(
        kinds.contains(&beast_render::blueprint::EffectKind::Drip),
        "expected Drip (chemical) effect, got {kinds:?}"
    );
}

#[test]
fn pipeline_is_deterministic() {
    let phenotype = elastic_worm();
    let directives = one_of_every_directive();
    let biome = neutral_biome_color();
    let a = compile_blueprint(&phenotype, &directives, biome.clone(), "fixture");
    let b = compile_blueprint(&phenotype, &directives, biome.clone(), "fixture");
    assert_eq!(
        a, b,
        "two compiles of identical input must produce equal blueprints"
    );

    // Hash equality via Debug-format fingerprint — the type derives
    // PartialEq + Hash but `format!` is human-readable for diffs.
    let af = format!("{a:#?}");
    let bf = format!("{b:#?}");
    assert_eq!(af, bf, "Debug formats diverged across compiles");
}

#[test]
fn every_directive_variant_is_handled() {
    let phenotype = elastic_worm();
    let directives = one_of_every_directive();
    let bp = compile_blueprint(
        &phenotype,
        &directives,
        neutral_biome_color(),
        "every_directive",
    );
    assert_structurally_valid(&bp);

    // Inflate → first volume is scaled. Hard to assert without knowing
    // the unscaled radius; use the volume count as a smoke check.
    assert!(!bp.volumes.is_empty());

    // Colorize biome-camo → a non-Global material region exists with
    // hue resolved from biome color.
    let biome_resolved = bp.materials.iter().any(|m| {
        matches!(m.target, MaterialTarget::Volume { .. })
            && m.props.base_color.hue == neutral_biome_color().hue
    });
    assert!(
        biome_resolved,
        "biome-color sentinel should have been resolved to neutral hue"
    );
}

#[test]
fn pipeline_output_does_not_invent_names() {
    // Mechanics-label separation (INVARIANTS §2): the pipeline must not
    // bake species-specific or ability-specific names into the
    // blueprint. The only string fields in CreatureBlueprint that can
    // carry external text are `display_name` (caller-supplied) and
    // bone names (auto-generated as core_/limb_).
    let bp = compile_blueprint(
        &rigid_armored(),
        &[protrude_dorsal_spikes()],
        neutral_biome_color(),
        "fixture-display",
    );
    for bone in &bp.skeleton.bones {
        assert!(
            bone.name.starts_with("core_") || bone.name.starts_with("limb_"),
            "bone name `{}` is not auto-generated",
            bone.name
        );
        // Sanity: no species / ability label.
        for forbidden in ["echolocation", "predator", "prey", "armored", "elastic"] {
            assert!(
                !bone.name.contains(forbidden),
                "bone name `{}` contains label-like substring `{}`",
                bone.name,
                forbidden
            );
        }
    }
}
