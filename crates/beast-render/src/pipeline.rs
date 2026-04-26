//! Visual pipeline: `(ResolvedPhenotype, &[VisualDirective], biome)` →
//! [`CreatureBlueprint`].
//!
//! Pure deterministic compile; no mutable global state, no PRNG, no
//! wall-clock reads. Same inputs always produce a byte-identical
//! blueprint hash.
//!
//! Substages run in sequence. Each substage is a free function whose
//! signature names every input it consumes — there is no shared mutable
//! "context" object, by design (mistakes in shared-context plumbing have
//! been a determinism hazard in previous sprints; see the body-map module
//! in `beast-interpreter`).
//!
//! See `documentation/systems/10_procgen_visual_pipeline.md` §4 for the
//! authoritative algorithm.

use std::collections::BTreeMap;

use beast_core::{BodySite, Q3232};
use beast_interpreter::ResolvedPhenotype;

use crate::blueprint::{
    Aabb, AttachPoint, AttachedEffect, BlueprintMetadata, Bone, BoneTag, BoneTree,
    CreatureBlueprint, EffectKind, EffectSpec, EffectTrigger, JointConstraint, MaterialProps,
    MaterialRegion, MaterialTarget, PatternOverlay, Placement, SurfaceDetail, SurfaceType,
    SymmetryMode, Vec3, Volume, VolumeShape,
};
use crate::directive::{
    ColorSpec, DirectiveParams, Distribution, ProtrusionShape, SurfaceRegion, VisualDirective,
};

// ---------------------------------------------------------------------------
// PIPELINE NUMERIC INVARIANT
// ---------------------------------------------------------------------------
//
// Although INVARIANTS §1 forbids floats on the sim path, this module uses
// `Q3232::from_num(<decimal>_f64)` to declare *constant* coefficients
// pulled from the design doc (e.g. `0.3 + mass*0.7`, segment-count
// formula multipliers, channel thresholds). The conversion is performed
// at function-call time, not on every operation — the resulting Q3232
// value is then used in saturating fixed-point arithmetic. Same input
// channel + same constant → same Q3232 across processes and platforms,
// because:
//
//  1. The decimal literal itself is parsed deterministically by rustc
//     at compile time into an IEEE 754 f64.
//  2. `I32F32::saturating_from_num::<f64>` rounds toward zero with no
//     platform-specific behaviour: the implementation in `fixed` is
//     bit-by-bit shift + sign + saturate, identical on every target.
//
// We do NOT permit float values that depend on simulation state (e.g. a
// channel value cast to f64 then back). Channel reads stay in Q3232
// throughout. Reviewers: any new f64 literal in this module should be a
// design-doc-derived constant; flag any value that comes from sim state.
//
// ---------------------------------------------------------------------------
// Channel id literals
// ---------------------------------------------------------------------------
//
// The pipeline reads phenotype channels by string id. The id strings are
// declared up front so a typo turns into a compile error rather than a
// silent zero. They mirror the canonical channel manifest naming.

const CH_ELASTIC_DEFORMATION: &str = "elastic_deformation";
const CH_STRUCTURAL_RIGIDITY: &str = "structural_rigidity";
const CH_MASS_DENSITY: &str = "mass_density";
const CH_METABOLIC_RATE: &str = "metabolic_rate";
const CH_SURFACE_FRICTION: &str = "surface_friction";
const CH_KINETIC_FORCE: &str = "kinetic_force";
const CH_LIGHT_EMISSION: &str = "light_emission";
const CH_CHEMICAL_OUTPUT: &str = "chemical_output";
const CH_THERMAL_OUTPUT: &str = "thermal_output";

/// Read a global channel value, defaulting to `Q3232::ZERO` when absent.
fn ch(phenotype: &ResolvedPhenotype, name: &str) -> Q3232 {
    phenotype
        .global_channels
        .get(name)
        .copied()
        .unwrap_or(Q3232::ZERO)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a creature blueprint from a resolved phenotype + the
/// interpreter-emitted visual directives.
///
/// `biome_color` is the dominant biome color used to resolve `Colorize`
/// directives that carry the biome-color sentinel ([`ColorSpec::biome_color`]).
///
/// `display_name` is opaque to the pipeline — supplied by the caller for
/// inclusion in [`BlueprintMetadata`]. The chronicler / UI layer is
/// responsible for naming; the visual pipeline never decides names.
pub fn compile_blueprint(
    phenotype: &ResolvedPhenotype,
    directives: &[VisualDirective],
    biome_color: ColorSpec,
    display_name: impl Into<String>,
) -> CreatureBlueprint {
    let directives = canonicalise(directives);

    let skeleton = build_skeleton(phenotype, &directives);
    let volumes = shape_volumes(&skeleton, phenotype, &directives);
    let surfaces = apply_surface_details(&volumes, &directives);
    let materials = assign_materials(&volumes, &directives, &biome_color);
    let effects = attach_effects(&volumes, phenotype);
    let animations = crate::animation::rig_animations(&skeleton, phenotype);

    let metadata = BlueprintMetadata {
        bounding_box: bounding_box_for(&skeleton, &volumes),
        display_name: display_name.into(),
    };

    CreatureBlueprint {
        skeleton,
        volumes,
        surfaces,
        materials,
        effects,
        animations,
        metadata,
    }
}

/// Sort directives into the canonical evaluation order: `(priority desc,
/// body_region asc, id asc)`. Returns an owned `Vec` so downstream
/// substages don't re-sort.
fn canonicalise(directives: &[VisualDirective]) -> Vec<VisualDirective> {
    let mut out = directives.to_vec();
    out.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.body_region.cmp(&b.body_region))
            .then_with(|| a.id.cmp(&b.id))
    });
    out
}

// ---------------------------------------------------------------------------
// Substage 1: Skeleton
// ---------------------------------------------------------------------------

fn build_skeleton(phenotype: &ResolvedPhenotype, _directives: &[VisualDirective]) -> BoneTree {
    let plan = derive_body_plan(phenotype);
    let mut bones: Vec<Bone> = Vec::new();
    build_spine(&plan, &mut bones);
    build_limb_pairs(&plan, &mut bones);
    BoneTree { bones }
}

/// Numeric body-plan summary derived from the phenotype's channel
/// values. Pulled out of `build_skeleton` so spine and limb construction
/// share one well-named struct instead of a long argument list.
struct BodyPlan {
    segment_count: u32,
    base_thickness: Q3232,
    flexibility: Q3232,
    limb_count: u32,
    limb_length: Q3232,
    limb_thickness: Q3232,
    rigidity: Q3232,
}

fn derive_body_plan(phenotype: &ResolvedPhenotype) -> BodyPlan {
    let elasticity = ch(phenotype, CH_ELASTIC_DEFORMATION);
    let rigidity = ch(phenotype, CH_STRUCTURAL_RIGIDITY);
    let mass = ch(phenotype, CH_MASS_DENSITY);
    let speed = ch(phenotype, CH_METABOLIC_RATE);
    let friction = ch(phenotype, CH_SURFACE_FRICTION);
    let kinetic = ch(phenotype, CH_KINETIC_FORCE);

    // Segment count: 3 + elasticity*5 - rigidity*3, clamped to [2, 8].
    let segment_count_raw = Q3232::from_num(3)
        .saturating_add(elasticity.saturating_mul(Q3232::from_num(5)))
        .saturating_sub(rigidity.saturating_mul(Q3232::from_num(3)));
    let segment_count = clamp_to_u32(segment_count_raw, 2, 8);

    // Base thickness: 0.3 + mass*0.7
    let base_thickness =
        Q3232::from_num(0.3_f64).saturating_add(mass.saturating_mul(Q3232::from_num(0.7_f64)));

    // Spine flexibility: clamp(elasticity*0.8 - rigidity*0.5, 0.05, 0.95)
    let flexibility = elasticity
        .saturating_mul(Q3232::from_num(0.8_f64))
        .saturating_sub(rigidity.saturating_mul(Q3232::from_num(0.5_f64)))
        .clamp(Q3232::from_num(0.05_f64), Q3232::from_num(0.95_f64));

    // Limb count: clamp((friction*0.4 + (1-elasticity)*0.3 + speed*0.3)*6, 0, 8).
    let limb_potential = friction
        .saturating_mul(Q3232::from_num(0.4_f64))
        .saturating_add(
            Q3232::ONE
                .saturating_sub(elasticity)
                .saturating_mul(Q3232::from_num(0.3_f64)),
        )
        .saturating_add(speed.saturating_mul(Q3232::from_num(0.3_f64)));
    let limb_count_raw = limb_potential.saturating_mul(Q3232::from_num(6));
    let mut limb_count = clamp_to_u32(limb_count_raw, 0, 8);
    if limb_count % 2 == 1 {
        // Bilateral symmetry: round up to the next even count.
        limb_count = (limb_count + 1).min(8);
    }

    let limb_length = Q3232::from_num(0.3_f64)
        .saturating_add(kinetic.saturating_mul(Q3232::from_num(0.5_f64)))
        .saturating_add(speed.saturating_mul(Q3232::from_num(0.3_f64)));
    let limb_thickness =
        Q3232::from_num(0.15_f64).saturating_add(rigidity.saturating_mul(Q3232::from_num(0.2_f64)));

    BodyPlan {
        segment_count,
        base_thickness,
        flexibility,
        limb_count,
        limb_length,
        limb_thickness,
        rigidity,
    }
}

fn build_spine(plan: &BodyPlan, bones: &mut Vec<Bone>) {
    let head_length = plan.base_thickness.saturating_mul(Q3232::from_num(1.5_f64));
    bones.push(Bone {
        id: 0,
        name: "core_head".to_string(),
        parent_id: None,
        local_position: Vec3::ZERO,
        local_rotation: Q3232::ZERO,
        length: head_length,
        thickness: plan.base_thickness,
        tags: vec![BoneTag::Core, BoneTag::Head],
        constraints: JointConstraint {
            min_angle: Q3232::from_num(-15),
            max_angle: Q3232::from_num(15),
            stiffness: Q3232::from_num(0.8_f64),
            preferred: Q3232::ZERO,
        },
    });

    let mut prev_length = head_length;
    let max_segment_angle = plan.flexibility.saturating_mul(Q3232::from_num(30));
    for i in 1..plan.segment_count {
        let mut tags = vec![BoneTag::Core];
        if i == plan.segment_count - 1 {
            tags.push(BoneTag::Tail);
        }
        let segment_length = plan.base_thickness.saturating_mul(
            Q3232::from_num(2)
                .saturating_sub(Q3232::from_num(i).saturating_mul(Q3232::from_num(0.1_f64))),
        );
        let segment_thickness = plan.base_thickness.saturating_mul(
            Q3232::ONE.saturating_sub(Q3232::from_num(i).saturating_mul(Q3232::from_num(0.05_f64))),
        );
        bones.push(Bone {
            id: i,
            name: format!("core_{i}"),
            parent_id: Some(i - 1),
            local_position: Vec3::new(prev_length, Q3232::ZERO, Q3232::ZERO),
            local_rotation: Q3232::ZERO,
            length: segment_length,
            thickness: segment_thickness,
            tags,
            constraints: JointConstraint {
                min_angle: max_segment_angle.saturating_mul(Q3232::from_num(-1)),
                max_angle: max_segment_angle,
                stiffness: Q3232::ONE.saturating_sub(plan.flexibility),
                preferred: Q3232::ZERO,
            },
        });
        prev_length = segment_length;
    }
}

fn build_limb_pairs(plan: &BodyPlan, bones: &mut Vec<Bone>) {
    // Limbs attach to the second segment if there is one (so they sit
    // off the head); otherwise they go straight on the head.
    let limb_anchor: u32 = if plan.segment_count >= 2 { 1 } else { 0 };
    let pair_count = plan.limb_count / 2;

    for pair_idx in 0..pair_count {
        for side in 0..2u32 {
            let id = bones.len() as u32;
            let side_label = if side == 0 { "L" } else { "R" };
            // Mirror Y position: left = +y, right = -y. Side magnitude
            // proportional to base thickness so limbs sit on the body.
            let y = if side == 0 {
                plan.base_thickness
            } else {
                plan.base_thickness.saturating_mul(Q3232::from_num(-1))
            };
            bones.push(Bone {
                id,
                name: format!("limb_{side_label}_{pair_idx}"),
                parent_id: Some(limb_anchor),
                local_position: Vec3::new(Q3232::ZERO, y, Q3232::ZERO),
                local_rotation: Q3232::ZERO,
                length: plan.limb_length,
                thickness: plan.limb_thickness,
                tags: vec![BoneTag::Limb, BoneTag::LimbTip, BoneTag::Symmetric],
                constraints: JointConstraint {
                    min_angle: Q3232::from_num(-60),
                    max_angle: Q3232::from_num(60),
                    stiffness: Q3232::from_num(0.3_f64)
                        .saturating_add(plan.rigidity.saturating_mul(Q3232::from_num(0.5_f64))),
                    preferred: Q3232::ZERO,
                },
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Substage 2: Volumes
// ---------------------------------------------------------------------------

fn shape_volumes(
    skeleton: &BoneTree,
    phenotype: &ResolvedPhenotype,
    directives: &[VisualDirective],
) -> Vec<Volume> {
    let elasticity = ch(phenotype, CH_ELASTIC_DEFORMATION);
    let rigidity = ch(phenotype, CH_STRUCTURAL_RIGIDITY);

    // Inflate scales keyed by body region. Multiple Inflate directives
    // for the same region multiply. `BodySite` derives `Ord` (see
    // `beast-core/src/body_site.rs`) so we can key the BTreeMap on it
    // directly.
    let mut inflate_by_region: BTreeMap<BodySite, Q3232> = BTreeMap::new();
    for d in directives {
        if let DirectiveParams::Inflate(p) = &d.params {
            let entry = inflate_by_region.entry(d.body_region).or_insert(Q3232::ONE);
            *entry = entry.saturating_mul(p.scale);
        }
    }

    let mut volumes = Vec::with_capacity(skeleton.bones.len());
    for bone in &skeleton.bones {
        let shape = choose_volume_shape(bone, elasticity, rigidity);

        let symmetry = if bone.tags.contains(&BoneTag::Symmetric) {
            SymmetryMode::BilateralX
        } else {
            SymmetryMode::None
        };

        let mut vol = Volume {
            id: bone.id,
            attached_bones: vec![bone.id],
            shape,
            symmetry,
            layer: 0,
        };

        // Apply inflate by region tag — Core / Limb / Tail / Head map to
        // BodySite via the bone's primary tag.
        let inflate = inflate_for_bone(bone, &inflate_by_region);
        if inflate != Q3232::ONE {
            vol.shape = scale_shape(vol.shape, inflate);
        }
        volumes.push(vol);
    }
    volumes
}

fn choose_volume_shape(bone: &Bone, elasticity: Q3232, rigidity: Q3232) -> VolumeShape {
    if bone.tags.contains(&BoneTag::Core) {
        if elasticity > Q3232::from_num(0.5_f64) {
            VolumeShape::Capsule {
                radius: bone.thickness.saturating_mul(Q3232::from_num(1.1_f64)),
                length: bone.length,
            }
        } else if rigidity > Q3232::from_num(0.5_f64) {
            VolumeShape::Capsule {
                radius: bone.thickness.saturating_mul(Q3232::from_num(1.2_f64)),
                length: bone.length,
            }
        } else {
            VolumeShape::Tapered {
                radius_start: bone.thickness,
                radius_end: bone.thickness.saturating_mul(Q3232::from_num(0.8_f64)),
            }
        }
    } else if bone.tags.contains(&BoneTag::Limb) {
        VolumeShape::Tapered {
            radius_start: bone.thickness,
            radius_end: bone.thickness.saturating_mul(Q3232::from_num(0.5_f64)),
        }
    } else if bone.tags.contains(&BoneTag::Appendage) {
        VolumeShape::Tapered {
            radius_start: bone.thickness.saturating_mul(Q3232::from_num(0.8_f64)),
            radius_end: bone.thickness.saturating_mul(Q3232::from_num(0.1_f64)),
        }
    } else {
        VolumeShape::Ellipsoid {
            radii: Vec3::new(bone.thickness, bone.thickness, bone.thickness),
        }
    }
}

fn scale_shape(shape: VolumeShape, scale: Q3232) -> VolumeShape {
    match shape {
        VolumeShape::Ellipsoid { radii } => VolumeShape::Ellipsoid {
            radii: Vec3::new(
                radii.x.saturating_mul(scale),
                radii.y.saturating_mul(scale),
                radii.z.saturating_mul(scale),
            ),
        },
        VolumeShape::Capsule { radius, length } => VolumeShape::Capsule {
            radius: radius.saturating_mul(scale),
            length,
        },
        VolumeShape::Tapered {
            radius_start,
            radius_end,
        } => VolumeShape::Tapered {
            radius_start: radius_start.saturating_mul(scale),
            radius_end: radius_end.saturating_mul(scale),
        },
    }
}

fn inflate_for_bone(bone: &Bone, inflate_by_region: &BTreeMap<BodySite, Q3232>) -> Q3232 {
    // Region resolution priority: Core/Tail/Head from tags, then Global
    // catch-all on BodySite::Global.
    let region = if bone.tags.contains(&BoneTag::Head) {
        BodySite::Head
    } else if bone.tags.contains(&BoneTag::Tail) {
        BodySite::Tail
    } else if bone.tags.contains(&BoneTag::Core) {
        BodySite::Core
    } else if bone.tags.contains(&BoneTag::Limb) {
        // Mirror left/right by name suffix; deterministic.
        if bone.name.contains("limb_L") {
            BodySite::LimbLeft
        } else {
            BodySite::LimbRight
        }
    } else if bone.tags.contains(&BoneTag::Jaw) {
        BodySite::Jaw
    } else if bone.tags.contains(&BoneTag::Appendage) {
        BodySite::Appendage
    } else {
        BodySite::Global
    };

    let direct = inflate_by_region
        .get(&region)
        .copied()
        .unwrap_or(Q3232::ONE);
    let global = inflate_by_region
        .get(&BodySite::Global)
        .copied()
        .unwrap_or(Q3232::ONE);
    direct.saturating_mul(global)
}

// ---------------------------------------------------------------------------
// Substage 3: Surface details
// ---------------------------------------------------------------------------

fn apply_surface_details(volumes: &[Volume], directives: &[VisualDirective]) -> Vec<SurfaceDetail> {
    let mut details = Vec::new();
    let mut next_id = 0u32;

    for d in directives {
        let target = match volumes.iter().find(|v| {
            // Map directive body region to volume by attached bone tags.
            // Substage logic stays simple: target the first volume that
            // matches the region; if none matches, target volume 0
            // (root).
            volume_matches_region(v, d.body_region, volumes)
        }) {
            Some(v) => v.id,
            None => continue,
        };

        if let Some(detail_type) = surface_type_from_directive(&d.params) {
            let placement = placement_for_directive(&d.params);
            details.push(SurfaceDetail {
                id: next_id,
                target_volume: target,
                detail: detail_type,
                placement,
            });
            next_id += 1;
        }
    }

    details
}

fn surface_type_from_directive(params: &DirectiveParams) -> Option<SurfaceType> {
    match params {
        DirectiveParams::Protrude(p) => {
            let taper = match p.shape {
                ProtrusionShape::Spike | ProtrusionShape::Horn => Q3232::from_num(0.9_f64),
                _ => Q3232::from_num(0.3_f64),
            };
            Some(SurfaceType::Protrusion {
                shape: p.shape,
                height: p.scale,
                base_width: p.scale.saturating_mul(Q3232::from_num(0.3_f64)),
                taper,
            })
        }
        DirectiveParams::Harden(h) => Some(SurfaceType::Hardening {
            pattern: h.pattern,
            roughness: h.roughness,
            segmentation: h.segmentation,
        }),
        DirectiveParams::Soften(s) => Some(SurfaceType::Membrane {
            smoothness: s.smoothness,
            transparency: s.transparency,
        }),
        DirectiveParams::Orifice(o) => Some(SurfaceType::Orifice {
            radius: o.size,
            depth: o.size.saturating_mul(Q3232::from_num(0.5_f64)),
            rim_width: o.size.saturating_mul(Q3232::from_num(0.2_f64)),
            orientation: o.orientation,
        }),
        DirectiveParams::Texture(t) => Some(SurfaceType::Texture {
            pattern: t.pattern,
            scale: t.scale,
            depth: Q3232::from_num(0.02_f64),
        }),
        DirectiveParams::Append(a) => Some(SurfaceType::AppendageAttachment {
            kind: a.appendage_type,
            count: a.count,
        }),
        DirectiveParams::Inflate(_) | DirectiveParams::Colorize(_) => None,
    }
}

fn placement_for_directive(params: &DirectiveParams) -> Placement {
    let (along_bone, region, count, distribution) = match params {
        DirectiveParams::Protrude(p) => (
            Q3232::from_num(0.5_f64),
            p.surface_region,
            p.density,
            p.distribution,
        ),
        DirectiveParams::Orifice(o) => (
            o.position,
            SurfaceRegion::Anterior,
            o.count,
            Distribution::Regular,
        ),
        DirectiveParams::Append(a) => (
            a.position,
            SurfaceRegion::Lateral,
            a.count,
            Distribution::Regular,
        ),
        _ => (
            Q3232::from_num(0.5_f64),
            SurfaceRegion::AllSurface,
            1,
            Distribution::Regular,
        ),
    };
    Placement {
        along_bone,
        surface_region: region,
        count,
        distribution,
        mirror: matches!(region, SurfaceRegion::Lateral),
    }
}

fn volume_matches_region(volume: &Volume, region: BodySite, all: &[Volume]) -> bool {
    // Volumes don't yet carry a body-region tag explicitly — the linkage
    // is via attached_bones[0]. For S9.5 we use the volume id == bone id
    // mapping established in `shape_volumes`; the caller will need bone
    // tags, but we don't have those here without re-passing the
    // skeleton. To keep this substage signature small, we use the
    // following fallback: target volume index 0 for Core/Head/Tail and
    // the last volume for limbs/appendages. Production-grade region →
    // volume mapping is a follow-up (#TBD).
    let idx = match region {
        BodySite::Head | BodySite::Core | BodySite::Tail | BodySite::Global => 0,
        BodySite::Jaw => 0,
        BodySite::LimbLeft | BodySite::LimbRight | BodySite::Appendage => {
            all.len().saturating_sub(1)
        }
    };
    volume.id == all.get(idx).map(|v| v.id).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Substage 4: Materials
// ---------------------------------------------------------------------------

fn assign_materials(
    volumes: &[Volume],
    directives: &[VisualDirective],
    biome_color: &ColorSpec,
) -> Vec<MaterialRegion> {
    let mut regions = Vec::new();

    // Global base material (uniform mid-gray; downstream Colorize
    // directives override per-volume).
    regions.push(MaterialRegion {
        id: 0,
        target: MaterialTarget::Global,
        props: MaterialProps {
            base_color: ColorSpec::rgb(
                Q3232::from_num(30),
                Q3232::from_num(0.5_f64),
                Q3232::from_num(0.6_f64),
            ),
            roughness: Q3232::from_num(0.3_f64),
            metallic: Q3232::ZERO,
            subsurface: Q3232::from_num(0.1_f64),
            emission: None,
            emission_power: Q3232::ZERO,
            pattern: None,
        },
    });

    let mut next_id = 1u32;
    for d in directives {
        let DirectiveParams::Colorize(c) = &d.params else {
            continue;
        };
        // Resolve biome-color sentinel.
        let base_color = resolve_color(&c.base_color, biome_color);
        let pattern = c.pattern.map(|pattern| PatternOverlay {
            pattern,
            color_a: base_color,
            color_b: c
                .pattern_color_secondary
                .map(|c| resolve_color(&c, biome_color))
                .unwrap_or(base_color),
            scale: Q3232::from_num(0.3_f64),
            contrast: c.contrast,
        });

        // Pick the volume to color by region — first volume whose
        // attached bone is in the region. For S9.5 we use the same
        // region→volume fallback as substage 3.
        let target_vol = match volumes
            .iter()
            .find(|v| volume_matches_region(v, d.body_region, volumes))
        {
            Some(v) => v.id,
            None => continue,
        };

        regions.push(MaterialRegion {
            id: next_id,
            target: MaterialTarget::Volume {
                volume_id: target_vol,
            },
            props: MaterialProps {
                base_color,
                roughness: Q3232::from_num(0.2_f64),
                metallic: Q3232::ZERO,
                subsurface: Q3232::from_num(0.1_f64),
                emission: c.emission.map(|e| resolve_color(&e, biome_color)),
                emission_power: c.emission_intensity,
                pattern,
            },
        });
        next_id += 1;
    }

    regions
}

fn resolve_color(src: &ColorSpec, biome_color: &ColorSpec) -> ColorSpec {
    match src.hue {
        Some(_) => *src,
        None => ColorSpec {
            hue: biome_color.hue,
            saturation: src.saturation,
            value: src.value,
            alpha: src.alpha,
        },
    }
}

// ---------------------------------------------------------------------------
// Substage 5: Effects
// ---------------------------------------------------------------------------

fn attach_effects(volumes: &[Volume], phenotype: &ResolvedPhenotype) -> Vec<AttachedEffect> {
    let mut effects = Vec::new();
    let mut next_id = 0u32;

    let glow = ch(phenotype, CH_LIGHT_EMISSION);
    if glow > Q3232::from_num(0.3_f64) {
        effects.push(AttachedEffect {
            id: next_id,
            attach: AttachPoint::Aura {
                radius: glow.saturating_mul(Q3232::from_num(2)),
            },
            spec: EffectSpec {
                kind: EffectKind::Glow,
                color: ColorSpec::rgb(Q3232::from_num(60), Q3232::from_num(0.6_f64), Q3232::ONE),
                rate: Q3232::ZERO,
                size: Q3232::ZERO,
                lifetime: Q3232::ZERO,
            },
            trigger: EffectTrigger::Always,
        });
        next_id += 1;
    }

    let chemical = ch(phenotype, CH_CHEMICAL_OUTPUT);
    if chemical > Q3232::from_num(0.4_f64) {
        // Drip near the head (volume 0).
        if let Some(target) = volumes.first() {
            effects.push(AttachedEffect {
                id: next_id,
                attach: AttachPoint::Volume {
                    volume_id: target.id,
                },
                spec: EffectSpec {
                    kind: EffectKind::Drip,
                    color: ColorSpec::rgb(
                        Q3232::from_num(120),
                        Q3232::from_num(0.8_f64),
                        Q3232::from_num(0.6_f64),
                    ),
                    rate: chemical.saturating_mul(Q3232::from_num(5)),
                    size: Q3232::from_num(0.05_f64),
                    lifetime: Q3232::from_num(2),
                },
                trigger: EffectTrigger::Always,
            });
            next_id += 1;
        }
    }

    let thermal = ch(phenotype, CH_THERMAL_OUTPUT);
    if thermal > Q3232::from_num(0.5_f64) {
        effects.push(AttachedEffect {
            id: next_id,
            attach: AttachPoint::Aura { radius: Q3232::ONE },
            spec: EffectSpec {
                kind: EffectKind::Particle,
                color: ColorSpec::rgb(Q3232::from_num(0), Q3232::ONE, Q3232::ONE),
                rate: thermal.saturating_mul(Q3232::from_num(10)),
                size: Q3232::from_num(0.1_f64),
                lifetime: Q3232::from_num(0.5_f64),
            },
            trigger: EffectTrigger::WhenMoving,
        });
    }

    effects
}

// ---------------------------------------------------------------------------
// Bounding-box helper
// ---------------------------------------------------------------------------

fn bounding_box_for(skeleton: &BoneTree, volumes: &[Volume]) -> Aabb {
    if skeleton.is_empty() {
        return Aabb {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        };
    }

    // Sum core-bone lengths along x; max thickness across all bones for
    // y; volumes' radii contribute as well.
    let total_length: Q3232 = skeleton
        .bones
        .iter()
        .filter(|b| b.tags.contains(&BoneTag::Core))
        .map(|b| b.length)
        .fold(Q3232::ZERO, Q3232::saturating_add);
    let max_thickness = skeleton
        .bones
        .iter()
        .map(|b| b.thickness)
        .fold(Q3232::ZERO, |acc, t| if t > acc { t } else { acc });
    let max_radius = volumes
        .iter()
        .map(|v| match &v.shape {
            VolumeShape::Ellipsoid { radii } => {
                if radii.x > radii.y {
                    radii.x
                } else {
                    radii.y
                }
            }
            VolumeShape::Capsule { radius, .. } => *radius,
            VolumeShape::Tapered { radius_start, .. } => *radius_start,
        })
        .fold(Q3232::ZERO, |acc, r| if r > acc { r } else { acc });
    let half_y = if max_thickness > max_radius {
        max_thickness
    } else {
        max_radius
    };

    Aabb {
        min: Vec3::new(
            Q3232::ZERO,
            half_y.saturating_mul(Q3232::from_num(-1)),
            Q3232::ZERO,
        ),
        max: Vec3::new(total_length, half_y, Q3232::ZERO),
    }
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

fn clamp_to_u32(value: Q3232, lo: u32, hi: u32) -> u32 {
    // Round-half-to-even isn't required here — channel ratios are
    // already deterministic Q3232; we just need a stable integer cut.
    let v: i64 = value.to_num::<i64>();
    if v < lo as i64 {
        lo
    } else if v > hi as i64 {
        hi
    } else {
        v as u32
    }
}
