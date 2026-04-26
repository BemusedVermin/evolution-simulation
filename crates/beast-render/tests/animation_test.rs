//! S9.6 acceptance tests for animation rigging.
//!
//! Unit tests for the lerp / easing / sample primitives live alongside
//! the implementation in `src/animation.rs`. These integration tests
//! cover the higher-level contract:
//!
//! * `compile_blueprint` produces a non-empty [`AnimationSet`] for the
//!   pipeline's three reference phenotype profiles (elastic worm,
//!   rigid armored, generic biped).
//! * Locomotion-style selection picks the right variant for each
//!   phenotype.
//! * `Animator::sample` is byte-deterministic across calls.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use beast_core::{BodySite, Q3232};
use beast_interpreter::{LifeStage, ResolvedPhenotype};
use beast_render::animation::{rig_animations, Animator, LocomotionStyle};
use beast_render::directive::{ColorSpec, DirectiveParams, Inflate, VisualDirective};
use beast_render::{compile_blueprint, CreatureBlueprint};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn phenotype_with(channels: &[(&str, f64)]) -> ResolvedPhenotype {
    let mut p = ResolvedPhenotype::new(Q3232::from_num(50_i32), LifeStage::Adult);
    let map: BTreeMap<String, Q3232> = channels
        .iter()
        .map(|(name, value)| (name.to_string(), Q3232::from_num(*value)))
        .collect();
    p.global_channels = map;
    p
}

fn elastic_worm() -> ResolvedPhenotype {
    phenotype_with(&[
        ("elastic_deformation", 0.85),
        ("structural_rigidity", 0.05),
        ("mass_density", 0.3),
        ("metabolic_rate", 0.6),
        ("surface_friction", 0.2),
    ])
}

fn rigid_armored() -> ResolvedPhenotype {
    phenotype_with(&[
        ("elastic_deformation", 0.05),
        ("structural_rigidity", 0.85),
        ("mass_density", 0.7),
        ("metabolic_rate", 0.3),
        ("surface_friction", 0.6),
    ])
}

fn quadruped_walker() -> ResolvedPhenotype {
    // Mid elasticity / rigidity + high friction → fall-through to
    // limb-count branch with ≥4 limbs.
    phenotype_with(&[
        ("elastic_deformation", 0.2),
        ("structural_rigidity", 0.2),
        ("mass_density", 0.5),
        ("metabolic_rate", 0.6),
        ("surface_friction", 0.9),
        ("kinetic_force", 0.7),
    ])
}

fn biped_walker() -> ResolvedPhenotype {
    // Mid elasticity / rigidity + low friction + low speed → limb
    // potential ≈ (0.1*0.4 + 0.7*0.3 + 0.2*0.3)*6 = 1.74 → limb_count=2.
    phenotype_with(&[
        ("elastic_deformation", 0.3),
        ("structural_rigidity", 0.3),
        ("mass_density", 0.4),
        ("metabolic_rate", 0.2),
        ("surface_friction", 0.1),
        ("kinetic_force", 0.5),
    ])
}

fn neutral_biome() -> ColorSpec {
    ColorSpec::rgb(
        Q3232::from_num(120_i32),
        Q3232::from_num(0.4_f64),
        Q3232::from_num(0.5_f64),
    )
}

fn no_directives() -> Vec<VisualDirective> {
    Vec::new()
}

fn one_inflate_directive() -> Vec<VisualDirective> {
    vec![VisualDirective {
        id: 0,
        body_region: BodySite::Core,
        params: DirectiveParams::Inflate(Inflate {
            scale: Q3232::from_num(1.1_f64),
        }),
        priority: 0,
    }]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn elastic_worm_picks_sinuous_wave_locomotion() {
    let phenotype = elastic_worm();
    let bp = compile_blueprint(
        &phenotype,
        &no_directives(),
        neutral_biome(),
        "elastic_worm",
    );
    let style = beast_render::animation::pick_locomotion_style(&bp.skeleton, &phenotype);
    assert_eq!(style, LocomotionStyle::SinuousWave);
}

#[test]
fn rigid_armored_picks_segmented_scuttle() {
    let phenotype = rigid_armored();
    let bp = compile_blueprint(
        &phenotype,
        &no_directives(),
        neutral_biome(),
        "rigid_armored",
    );
    let style = beast_render::animation::pick_locomotion_style(&bp.skeleton, &phenotype);
    assert_eq!(style, LocomotionStyle::SegmentedScuttle);
}

#[test]
fn quadruped_walker_picks_quadruped_walk() {
    let phenotype = quadruped_walker();
    let bp = compile_blueprint(&phenotype, &no_directives(), neutral_biome(), "quadruped");
    let style = beast_render::animation::pick_locomotion_style(&bp.skeleton, &phenotype);
    assert_eq!(style, LocomotionStyle::QuadrupedWalk);
}

#[test]
fn biped_walker_picks_biped_walk() {
    let phenotype = biped_walker();
    let bp = compile_blueprint(&phenotype, &no_directives(), neutral_biome(), "biped");
    let style = beast_render::animation::pick_locomotion_style(&bp.skeleton, &phenotype);
    assert_eq!(style, LocomotionStyle::BipedWalk);
}

#[test]
fn rigged_animation_set_is_never_empty() {
    let cases = [elastic_worm(), rigid_armored(), quadruped_walker()];
    for phenotype in cases {
        let bp = compile_blueprint(&phenotype, &no_directives(), neutral_biome(), "fixture");
        let anim = &bp.animations;
        assert!(
            anim.locomotion.len() >= 2,
            "expected ≥2 locomotion clips (walk+run), got {}",
            anim.locomotion.len()
        );
        assert!(!anim.idle.is_empty(), "expected ≥1 idle clip");
        assert!(
            !anim.damage.bone_tracks.is_empty() || !anim.death.bone_tracks.is_empty(),
            "damage / death clips can't both be empty"
        );
    }
}

#[test]
fn animator_sample_at_zero_and_duration_is_deterministic() {
    let bp = compile_blueprint(
        &elastic_worm(),
        &no_directives(),
        neutral_biome(),
        "fixture",
    );
    let walk = &bp.animations.locomotion[0];
    let animator = Animator::new(walk);

    let pose_a0 = animator.sample(Q3232::ZERO);
    let pose_a1 = animator.sample(Q3232::ZERO);
    assert_eq!(pose_a0, pose_a1, "two samples at t=0 must match");

    let pose_b0 = animator.sample(walk.duration);
    let pose_b1 = animator.sample(walk.duration);
    assert_eq!(pose_b0, pose_b1, "two samples at t=duration must match");
}

#[test]
fn rig_is_byte_deterministic_across_compiles() {
    fn h<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
    let phenotype = elastic_worm();
    let a: CreatureBlueprint =
        compile_blueprint(&phenotype, &no_directives(), neutral_biome(), "fx");
    let b: CreatureBlueprint =
        compile_blueprint(&phenotype, &no_directives(), neutral_biome(), "fx");
    assert_eq!(a.animations, b.animations, "AnimationSet must be equal");
    assert_eq!(
        h(&a.animations),
        h(&b.animations),
        "AnimationSet hash must match"
    );
}

#[test]
fn directives_do_not_affect_animation_rig() {
    // Animation depends on skeleton + phenotype only — visual
    // directives that don't change those should produce an identical
    // AnimationSet. This guards the substage's input contract.
    let phenotype = elastic_worm();
    let with = compile_blueprint(
        &phenotype,
        &one_inflate_directive(),
        neutral_biome(),
        "fixture",
    );
    let without = compile_blueprint(&phenotype, &no_directives(), neutral_biome(), "fixture");
    assert_eq!(
        with.animations, without.animations,
        "Inflate directive must not perturb the animation rig"
    );
}

#[test]
fn standalone_rig_matches_pipeline_rig() {
    // `rig_animations` is exposed publicly so the eventual app crate
    // can re-rig a creature without re-running the full pipeline. The
    // result must match what the pipeline produces.
    let phenotype = elastic_worm();
    let bp = compile_blueprint(&phenotype, &no_directives(), neutral_biome(), "fixture");
    let standalone = rig_animations(&bp.skeleton, &phenotype);
    assert_eq!(bp.animations, standalone);
}
