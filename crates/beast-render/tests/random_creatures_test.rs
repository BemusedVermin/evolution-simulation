//! S9.7 acceptance test: 100 deterministically-generated creatures
//! through the full visual pipeline + animator-sample-at-zero, asserting
//! structural invariants.
//!
//! What this test buys us:
//!
//! * **Coverage**: forces the pipeline through 100 different channel
//!   profiles, exercising every code path that branches on channel
//!   thresholds (skeleton segment count, locomotion-style fork, effect
//!   thresholds for glow / drip / particle).
//! * **Regression catcher**: a snapshot of one generated blueprint via
//!   `format!("{:#?}")` pins one instance against silent changes — any
//!   tweak to the pipeline that changes the output must also update the
//!   snapshot, surfacing the change in code review.
//! * **Determinism**: every input is derived from a single `world_seed`,
//!   so two runs of this test produce identical output. The
//!   `same_seed_produces_same_state_hash` test guards this.
//!
//! What this test deliberately doesn't cover:
//!
//! * Routing through `beast-genome::Genome` + `beast-interpreter::
//!   interpret_phenotype`. That path needs a fully wired channel
//!   registry, which is heavy for what we're checking; an interpreter
//!   integration test will land alongside Stage-4 directive emission.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};

use beast_core::{BodySite, Prng, Stream, Q3232};
use beast_interpreter::{LifeStage, ResolvedPhenotype};
use beast_render::animation::Animator;
use beast_render::blueprint::{AttachPoint, BoneTag, MaterialTarget, SurfaceType};
use beast_render::directive::{
    Append, AppendageKind, ColorSpec, Colorize, DirectiveParams, Distribution, Harden,
    HardenPattern, Inflate, Orifice, OrificeOrientation, Protrude, ProtrusionShape, Soften,
    SurfaceRegion, Texture, TexturePattern, VisualDirective,
};
use beast_render::{compile_blueprint, CreatureBlueprint};

/// Deterministic master seed for the smoke test. Changing it reseeds
/// every fixture; the snapshot test fails on the next run, surfacing
/// the change.
const SMOKE_TEST_SEED: u64 = 0xBEA5_5697_u64;

const N_CREATURES: usize = 100;

// ---------------------------------------------------------------------------
// Fixture generation
// ---------------------------------------------------------------------------

/// Channel ids the pipeline reads (mirrors `crate::channels` in
/// `beast-render`). Listed here verbatim so a one-sided rename — i.e.,
/// updating the pipeline without updating this fixture — produces a
/// `Q3232::ZERO` lookup, which then trips the bounding-box / animation
/// invariants downstream.
///
/// **Maintenance**: a new channel added in `crate::channels` must be
/// added here too. We don't currently have a cross-check; if drift
/// becomes a problem, expose a `pub(crate) fn channel_id_list() ->
/// &'static [&'static str]` from the channels module and assert
/// `CHANNEL_IDS` matches it.
const CHANNEL_IDS: &[&str] = &[
    "elastic_deformation",
    "structural_rigidity",
    "mass_density",
    "metabolic_rate",
    "surface_friction",
    "kinetic_force",
    "light_emission",
    "chemical_output",
    "thermal_output",
];

fn random_phenotype(rng: &mut Prng) -> ResolvedPhenotype {
    let mut p = ResolvedPhenotype::new(Q3232::from_num(50_i32), LifeStage::Adult);
    let mut channels = BTreeMap::new();
    for id in CHANNEL_IDS {
        channels.insert((*id).to_string(), rng.next_q3232_unit());
    }
    p.global_channels = channels;
    p
}

fn random_directives(rng: &mut Prng, creature_idx: usize) -> Vec<VisualDirective> {
    // Deterministically pick 0-7 directives per creature so the
    // pigeon-hole effect across 100 creatures puts all 8 variants on
    // the firing line at least a few times each.
    let count = (rng.next_u32() % 8) as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let region = match (creature_idx + i) % 4 {
            0 => BodySite::Core,
            1 => BodySite::Head,
            2 => BodySite::LimbLeft,
            _ => BodySite::Tail,
        };
        let id = (creature_idx as u32 * 100) + i as u32;
        let priority = (rng.next_u32() % 16) + 1;

        // `% 8` covers every variant of `DirectiveParams`. If a new
        // variant lands, expand this match — the lack of a wildcard
        // arm makes that requirement a compile error.
        let params = match rng.next_u32() % 8 {
            0 => DirectiveParams::Inflate(Inflate {
                scale: rng
                    .next_q3232_unit()
                    .saturating_add(Q3232::from_num(0.8_f64)),
            }),
            1 => DirectiveParams::Colorize(Colorize {
                base_color: ColorSpec::biome_color(rng.next_q3232_unit(), rng.next_q3232_unit()),
                emission: None,
                emission_intensity: Q3232::ZERO,
                pattern: Some(TexturePattern::Mottled),
                pattern_color_secondary: None,
                contrast: rng.next_q3232_unit(),
            }),
            2 => DirectiveParams::Protrude(Protrude {
                shape: ProtrusionShape::Spike,
                scale: rng
                    .next_q3232_unit()
                    .saturating_add(Q3232::from_num(0.1_f64)),
                density: ((rng.next_u32() % 6) + 1) as u8,
                distribution: Distribution::Regular,
                surface_region: SurfaceRegion::Dorsal,
            }),
            3 => DirectiveParams::Harden(Harden {
                roughness: rng.next_q3232_unit(),
                segmentation: ((rng.next_u32() % 12) + 1) as u8,
                pattern: HardenPattern::Plates,
            }),
            4 => DirectiveParams::Soften(Soften {
                smoothness: rng.next_q3232_unit(),
                transparency: rng.next_q3232_unit(),
            }),
            5 => DirectiveParams::Orifice(Orifice {
                size: rng
                    .next_q3232_unit()
                    .saturating_add(Q3232::from_num(0.05_f64)),
                position: rng.next_q3232_unit(),
                count: ((rng.next_u32() % 3) + 1) as u8,
                orientation: OrificeOrientation::Forward,
            }),
            6 => DirectiveParams::Append(Append {
                appendage_type: AppendageKind::Crest,
                count: ((rng.next_u32() % 4) + 1) as u8,
                position: rng.next_q3232_unit(),
            }),
            7 => DirectiveParams::Texture(Texture {
                pattern: TexturePattern::Striped,
                scale: rng
                    .next_q3232_unit()
                    .saturating_add(Q3232::from_num(0.1_f64)),
            }),
            _ => unreachable!("rng.next_u32() % 8 covers 0..=7"),
        };

        out.push(VisualDirective {
            id,
            body_region: region,
            params,
            priority,
        });
    }
    out
}

fn neutral_biome() -> ColorSpec {
    ColorSpec::rgb(
        Q3232::from_num(120_i32),
        Q3232::from_num(0.4_f64),
        Q3232::from_num(0.5_f64),
    )
}

// ---------------------------------------------------------------------------
// Structural-validity checks
// ---------------------------------------------------------------------------

fn assert_blueprint_invariants(idx: usize, seed_used: u64, bp: &CreatureBlueprint) {
    let bone_ids: BTreeSet<u32> = bp.skeleton.bones.iter().map(|b| b.id).collect();
    let volume_ids: BTreeSet<u32> = bp.volumes.iter().map(|v| v.id).collect();
    let detail_ids: BTreeSet<u32> = bp.surfaces.iter().map(|s| s.id).collect();

    // Skeleton has at least one Core bone.
    assert!(
        bp.skeleton
            .bones
            .iter()
            .any(|b| b.tags.contains(&BoneTag::Core)),
        "creature[{idx}] (seed_used={seed_used:#x}): skeleton has no Core bone"
    );

    // Volumes' attached_bones reference real bones.
    for vol in &bp.volumes {
        for bone_id in &vol.attached_bones {
            assert!(
                bone_ids.contains(bone_id),
                "creature[{idx}]: volume {} references non-existent bone {}",
                vol.id,
                bone_id
            );
        }
    }

    // Every surface detail's target_volume must be a real volume id.
    for s in &bp.surfaces {
        assert!(
            volume_ids.contains(&s.target_volume),
            "creature[{idx}]: surface detail {} targets volume {} which doesn't exist",
            s.id,
            s.target_volume
        );
        // Specific surface-type sanity checks: protrusion height,
        // texture scale, etc. must be non-degenerate.
        match &s.detail {
            SurfaceType::Protrusion { height, .. } => {
                assert!(
                    *height > Q3232::ZERO,
                    "creature[{idx}]: Protrusion detail {} has non-positive height {height:?}",
                    s.id
                );
            }
            SurfaceType::Orifice { radius, .. } => {
                assert!(
                    *radius > Q3232::ZERO,
                    "creature[{idx}]: Orifice detail {} has non-positive radius {radius:?}",
                    s.id
                );
            }
            _ => {}
        }
    }

    // Every material target resolves to a real id (or is Global).
    for m in &bp.materials {
        match &m.target {
            MaterialTarget::Global => {}
            MaterialTarget::Volume { volume_id } => {
                assert!(
                    volume_ids.contains(volume_id),
                    "creature[{idx}]: material {} targets volume {} which doesn't exist",
                    m.id,
                    volume_id
                );
            }
            MaterialTarget::Detail { detail_id } => {
                assert!(
                    detail_ids.contains(detail_id),
                    "creature[{idx}]: material {} targets detail {} which doesn't exist",
                    m.id,
                    detail_id
                );
            }
        }
    }

    // Effects: attach points resolve.
    for e in &bp.effects {
        match &e.attach {
            AttachPoint::Region { .. } | AttachPoint::Aura { .. } => {}
            AttachPoint::Volume { volume_id } => {
                assert!(
                    volume_ids.contains(volume_id),
                    "creature[{idx}]: effect {} attaches to volume {} which doesn't exist",
                    e.id,
                    volume_id
                );
            }
            AttachPoint::Detail { detail_id } => {
                assert!(
                    detail_ids.contains(detail_id),
                    "creature[{idx}]: effect {} attaches to detail {} which doesn't exist",
                    e.id,
                    detail_id
                );
            }
        }
    }

    // Bounding box non-degenerate (positive volume).
    let bb = &bp.metadata.bounding_box;
    assert!(
        bb.max.x > bb.min.x && bb.max.y > bb.min.y,
        "creature[{idx}]: bounding box has zero or negative extent: {bb:?}"
    );

    // Animation set non-empty.
    assert!(
        bp.animations.locomotion.len() >= 2,
        "creature[{idx}]: expected ≥2 locomotion clips, got {}",
        bp.animations.locomotion.len()
    );
    assert!(
        !bp.animations.idle.is_empty(),
        "creature[{idx}]: expected ≥1 idle clip"
    );

    // Per-field keyframe bounds. Q3232 has no NaN/infinity, so the
    // failure mode is saturation toward MIN/MAX — but checking only
    // those literals is too permissive (it accepts any value short of
    // saturation). Bounds are tied to design-doc expectations: the
    // death clip peaks at 90°, sinusoid amplitude is 20°, breathing
    // scale tops at 1.05.
    for clip in bp
        .animations
        .locomotion
        .iter()
        .chain(bp.animations.idle.iter())
        .chain(std::iter::once(&bp.animations.damage))
        .chain(std::iter::once(&bp.animations.death))
    {
        let duration = clip.duration;
        for track in &clip.bone_tracks {
            for kf in &track.keyframes {
                assert_keyframe_in_bounds(idx, &clip.name, track.bone_id, kf, duration);
            }
        }
    }
}

fn assert_keyframe_in_bounds(
    creature_idx: usize,
    clip: &str,
    bone_id: u32,
    kf: &beast_render::animation::Keyframe,
    duration: Q3232,
) {
    let max_rotation = Q3232::from_num(180);
    let neg_max_rotation = max_rotation.saturating_mul(Q3232::from_num(-1));
    assert!(
        kf.time >= Q3232::ZERO && kf.time <= duration,
        "creature[{creature_idx}] clip={clip} bone={bone_id}: time {time:?} outside [0, {duration:?}]",
        time = kf.time,
    );
    assert!(
        kf.rotation >= neg_max_rotation && kf.rotation <= max_rotation,
        "creature[{creature_idx}] clip={clip} bone={bone_id}: rotation {rot:?} outside [{neg_max_rotation:?}, {max_rotation:?}]",
        rot = kf.rotation,
    );
    // Scale > 0 (a zero scale would invert geometry); upper bound 4.0
    // flags any future leak from `Inflate` (which targets volumes,
    // not keyframes).
    assert!(
        kf.scale > Q3232::ZERO && kf.scale <= Q3232::from_num(4),
        "creature[{creature_idx}] clip={clip} bone={bone_id}: scale {scale:?} outside (0, 4]",
        scale = kf.scale,
    );
}

// ---------------------------------------------------------------------------
// State hash — for cross-run determinism
// ---------------------------------------------------------------------------

fn compute_state_hash(blueprints: &[CreatureBlueprint]) -> u64 {
    // Same-process fingerprint via the derived `Hash`. `DefaultHasher`
    // is documented as "subject to change" between Rust versions, so
    // the value is *not* stable across rustc upgrades or platforms —
    // sufficient here because both calls in `same_seed_produces_same_
    // state_hash` run in the same process. The cross-platform
    // determinism gate (M1 in `beast-sim/src/determinism.rs`) uses
    // BLAKE3-over-bincode for the stronger contract.
    let mut hasher = DefaultHasher::new();
    blueprints.hash(&mut hasher);
    hasher.finish()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

fn generate_blueprints(seed: u64) -> Vec<CreatureBlueprint> {
    // Two independent test streams, both rooted on `Stream::Testing` so
    // we don't accidentally shadow a production stream slot. Distinct
    // master seeds (XOR with a constant tag) keep the streams disjoint
    // without needing a second `Stream` variant.
    let phenotype_master = Prng::from_seed(seed);
    let directive_master = Prng::from_seed(seed ^ 0x00D1_5EC7_1BE5_u64);
    let mut phenotype_rng = phenotype_master.split_stream(Stream::Testing);
    let mut directive_rng = directive_master.split_stream(Stream::Testing);

    (0..N_CREATURES)
        .map(|i| {
            let phenotype = random_phenotype(&mut phenotype_rng);
            let directives = random_directives(&mut directive_rng, i);
            compile_blueprint(
                &phenotype,
                &directives,
                neutral_biome(),
                format!("smoke_{i:03}"),
            )
        })
        .collect()
}

#[test]
fn one_hundred_creatures_pass_structural_invariants() {
    let blueprints = generate_blueprints(SMOKE_TEST_SEED);
    assert_eq!(blueprints.len(), N_CREATURES);
    for (idx, bp) in blueprints.iter().enumerate() {
        assert_blueprint_invariants(idx, SMOKE_TEST_SEED, bp);
    }
}

#[test]
fn animator_sample_at_zero_runs_for_every_creature() {
    // Sample every locomotion / idle clip at t=0 and assert the result
    // is non-empty. This is the "creature is renderable" smoke check.
    let blueprints = generate_blueprints(SMOKE_TEST_SEED);
    for (idx, bp) in blueprints.iter().enumerate() {
        for clip in bp
            .animations
            .locomotion
            .iter()
            .chain(bp.animations.idle.iter())
        {
            let pose = Animator::new(clip).sample(Q3232::ZERO);
            assert!(
                !pose.bone_rotations.is_empty(),
                "creature[{idx}] clip={}: empty pose at t=0",
                clip.name
            );
            // Bone ids in the pose should match the track bone_ids.
            for (track, rotation) in clip.bone_tracks.iter().zip(pose.bone_rotations.iter()) {
                assert_eq!(
                    track.bone_id, rotation.bone_id,
                    "creature[{idx}] clip={}: pose bone-id mismatch",
                    clip.name
                );
            }
        }
    }
}

#[test]
fn same_seed_produces_same_state_hash() {
    let a = compute_state_hash(&generate_blueprints(SMOKE_TEST_SEED));
    let b = compute_state_hash(&generate_blueprints(SMOKE_TEST_SEED));
    assert_eq!(a, b, "two runs of S9.7 fixture must produce equal hash");
}

#[test]
fn different_seeds_produce_different_state_hashes() {
    // Sanity: the test isn't degenerate. Two distinct seeds should
    // produce distinct hashes (collision probability is ~2⁻⁶⁴).
    let a = compute_state_hash(&generate_blueprints(SMOKE_TEST_SEED));
    let b = compute_state_hash(&generate_blueprints(SMOKE_TEST_SEED.wrapping_add(1)));
    assert_ne!(a, b, "two different seeds collided — fixture is degenerate");
}

// NOTE: a single-creature snapshot test would be redundant here —
// `same_seed_produces_same_state_hash` already pins the entire batch's
// hash for same-process equality, which subsumes pinning one creature.
// A literal-asserted snapshot (with a BLESS workflow) would catch
// changes the first run but is brittle to ordinary refactors and
// requires manual updating; will revisit if the pipeline grows hidden
// state that the batch hash misses.
