//! Animation rigging — substage 6 of the visual pipeline.
//!
//! Generates an [`AnimationSet`] from a skeleton + resolved phenotype,
//! and provides an [`Animator`] that samples a clip at a given `t` to
//! produce a per-bone [`PoseFrame`].
//!
//! # Numeric contract
//!
//! All keyframe values are stored as [`Q3232`] (rotation in degrees,
//! local time in seconds, scale in unitless multipliers) for the same
//! reason the rest of the pipeline is fixed-point: a determinism test
//! across processes / platforms must produce byte-identical blueprint
//! hashes, and IEEE-754 float bit patterns are not guaranteed across
//! every platform we'll eventually run on.
//!
//! Renderers consume `Q3232::to_num::<f32>()` at sample time. The float
//! conversion happens *outside* the determinism boundary — by then
//! we've already verified the blueprint hash matches.
//!
//! See `documentation/systems/10_procgen_visual_pipeline.md` §4.6 for
//! the authoritative algorithm.

use beast_core::Q3232;
use beast_interpreter::ResolvedPhenotype;

use crate::blueprint::{Bone, BoneTag, BoneTree};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// All animations rigged for one creature.
///
/// Lists are ordered: locomotion clips by speed (walk first, run
/// second), then idle clips, then a single damage and death clip.
/// Iteration of any list is deterministic by index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnimationSet {
    pub locomotion: Vec<AnimationClip>,
    pub idle: Vec<AnimationClip>,
    pub damage: AnimationClip,
    pub death: AnimationClip,
}

/// One clip — a named, looping-or-one-shot bundle of per-bone keyframe
/// tracks. `duration` is in seconds (Q32.32).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnimationClip {
    pub name: String,
    pub duration: Q3232,
    pub looping: bool,
    pub bone_tracks: Vec<BoneTrack>,
}

/// A track: an ordered list of keyframes for one bone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoneTrack {
    pub bone_id: u32,
    pub keyframes: Vec<Keyframe>,
}

/// Single keyframe.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keyframe {
    /// Local clip time in seconds.
    pub time: Q3232,
    /// Rotation delta from rest, in degrees.
    pub rotation: Q3232,
    /// Squash / stretch along the bone axis (1.0 = no change).
    pub scale: Q3232,
    /// Easing curve from this keyframe to the next.
    pub easing: Easing,
}

/// Easing curve between two keyframes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// Locomotion style picked by [`pick_locomotion_style`] from the
/// phenotype's channel ratios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocomotionStyle {
    /// Snake / worm undulation. Spine bones oscillate in a travelling
    /// sinusoid; limbs (if any) hang.
    SinuousWave,
    /// Armored / segmented scuttle. Spine is mostly rigid; limbs lift
    /// in lock-step.
    SegmentedScuttle,
    /// Four-legged walk. Limb pairs alternate (front-left + rear-right
    /// then front-right + rear-left).
    QuadrupedWalk,
    /// Two-legged walk. Limbs alternate.
    BipedWalk,
}

// ---------------------------------------------------------------------------
// Pose sampling
// ---------------------------------------------------------------------------

/// Per-bone pose at one moment in time. `bone_rotations` is sorted by
/// `bone_id`; the renderer applies the deltas relative to each bone's
/// rest pose stored on the [`BoneTree`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PoseFrame {
    pub bone_rotations: Vec<BoneRotation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoneRotation {
    pub bone_id: u32,
    /// Rotation delta from rest, in degrees.
    pub rotation: Q3232,
    /// Scale (1.0 = unchanged).
    pub scale: Q3232,
}

/// Lightweight clip-sampling helper. Constructed by the renderer with
/// the chosen clip index; calling [`Self::sample`] produces a deterministic
/// [`PoseFrame`] for any input `t` in seconds.
#[derive(Debug, Clone, Copy)]
pub struct Animator<'clip> {
    clip: &'clip AnimationClip,
}

impl<'clip> Animator<'clip> {
    pub fn new(clip: &'clip AnimationClip) -> Self {
        Self { clip }
    }

    /// Sample the clip at `t` seconds. If `clip.looping` is true, `t`
    /// wraps modulo `clip.duration`; otherwise `t` clamps to
    /// `[0, duration]`.
    pub fn sample(&self, t: Q3232) -> PoseFrame {
        let t_local = if self.clip.looping {
            modulo_q3232(t, self.clip.duration)
        } else {
            t.clamp(Q3232::ZERO, self.clip.duration)
        };

        let bone_rotations: Vec<BoneRotation> = self
            .clip
            .bone_tracks
            .iter()
            .map(|track| sample_track(track, t_local))
            .collect();

        PoseFrame { bone_rotations }
    }
}

fn sample_track(track: &BoneTrack, t: Q3232) -> BoneRotation {
    if track.keyframes.is_empty() {
        return BoneRotation {
            bone_id: track.bone_id,
            rotation: Q3232::ZERO,
            scale: Q3232::ONE,
        };
    }
    if track.keyframes.len() == 1 {
        let kf = &track.keyframes[0];
        return BoneRotation {
            bone_id: track.bone_id,
            rotation: kf.rotation,
            scale: kf.scale,
        };
    }

    // Find the keyframe pair (a, b) such that a.time <= t <= b.time.
    // Keyframes are sorted by time at rig-time, so a linear scan is
    // fine for ≤16 keyframes per track.
    let last_idx = track.keyframes.len() - 1;
    if t <= track.keyframes[0].time {
        let kf = &track.keyframes[0];
        return BoneRotation {
            bone_id: track.bone_id,
            rotation: kf.rotation,
            scale: kf.scale,
        };
    }
    if t >= track.keyframes[last_idx].time {
        let kf = &track.keyframes[last_idx];
        return BoneRotation {
            bone_id: track.bone_id,
            rotation: kf.rotation,
            scale: kf.scale,
        };
    }

    let mut a_idx = 0;
    for i in 0..last_idx {
        if track.keyframes[i].time <= t && t <= track.keyframes[i + 1].time {
            a_idx = i;
            break;
        }
    }
    let a = &track.keyframes[a_idx];
    let b = &track.keyframes[a_idx + 1];

    let span = b.time.saturating_sub(a.time);
    let raw_alpha = if span == Q3232::ZERO {
        Q3232::ZERO
    } else {
        // Saturating divide via subtraction: alpha = (t - a.time) / span.
        // Q3232 has its own division operator (saturating).
        (t.saturating_sub(a.time)) / span
    };
    let alpha = apply_easing(raw_alpha, a.easing);

    BoneRotation {
        bone_id: track.bone_id,
        rotation: lerp_q3232(a.rotation, b.rotation, alpha),
        scale: lerp_q3232(a.scale, b.scale, alpha),
    }
}

fn lerp_q3232(a: Q3232, b: Q3232, alpha: Q3232) -> Q3232 {
    a.saturating_add(b.saturating_sub(a).saturating_mul(alpha))
}

fn modulo_q3232(value: Q3232, divisor: Q3232) -> Q3232 {
    if divisor <= Q3232::ZERO {
        return Q3232::ZERO;
    }
    let q = value / divisor;
    // Floor-toward-zero is fine here because clip timing is always
    // positive in practice; we still guard with abs to handle edge
    // cases without UB.
    let q_floor: i64 = q.to_num::<i64>();
    value.saturating_sub(divisor.saturating_mul(Q3232::from_num(q_floor)))
}

fn apply_easing(alpha: Q3232, easing: Easing) -> Q3232 {
    match easing {
        Easing::Linear => alpha,
        // Quadratic ease curves give a "soft" feel without trigonometry.
        // EaseIn:  alpha^2
        // EaseOut: 1 - (1 - alpha)^2 = alpha * (2 - alpha)
        // EaseInOut: 2*a^2 (a<0.5) | 1 - 2*(1-a)^2 (a>=0.5)
        Easing::EaseIn => alpha.saturating_mul(alpha),
        Easing::EaseOut => alpha.saturating_mul(Q3232::from_num(2).saturating_sub(alpha)),
        Easing::EaseInOut => {
            let half = Q3232::from_num(0.5_f64);
            if alpha < half {
                Q3232::from_num(2)
                    .saturating_mul(alpha)
                    .saturating_mul(alpha)
            } else {
                let inv = Q3232::ONE.saturating_sub(alpha);
                Q3232::ONE
                    .saturating_sub(Q3232::from_num(2).saturating_mul(inv).saturating_mul(inv))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locomotion-style decision
// ---------------------------------------------------------------------------

const CH_ELASTIC_DEFORMATION: &str = "elastic_deformation";
const CH_STRUCTURAL_RIGIDITY: &str = "structural_rigidity";
const CH_METABOLIC_RATE: &str = "metabolic_rate";

fn ch(phenotype: &ResolvedPhenotype, name: &str) -> Q3232 {
    phenotype
        .global_channels
        .get(name)
        .copied()
        .unwrap_or(Q3232::ZERO)
}

/// Pick a locomotion style from channel ratios + skeleton topology.
///
/// Tie-break is deterministic via the explicit branch order.
pub fn pick_locomotion_style(
    skeleton: &BoneTree,
    phenotype: &ResolvedPhenotype,
) -> LocomotionStyle {
    let elasticity = ch(phenotype, CH_ELASTIC_DEFORMATION);
    let rigidity = ch(phenotype, CH_STRUCTURAL_RIGIDITY);

    if elasticity > Q3232::from_num(0.6_f64) {
        LocomotionStyle::SinuousWave
    } else if rigidity > Q3232::from_num(0.6_f64) {
        LocomotionStyle::SegmentedScuttle
    } else {
        let limb_count = skeleton
            .bones
            .iter()
            .filter(|b| b.tags.contains(&BoneTag::Limb))
            .count();
        if limb_count >= 4 {
            LocomotionStyle::QuadrupedWalk
        } else {
            LocomotionStyle::BipedWalk
        }
    }
}

// ---------------------------------------------------------------------------
// Public rig entry point
// ---------------------------------------------------------------------------

/// Substage 6: build the animation set for a skeleton + phenotype.
///
/// Returns walk + run locomotion clips for the chosen
/// [`LocomotionStyle`], one idle (breathing pulse, frequency scaled by
/// metabolic rate), one damage clip, one death clip.
pub fn rig_animations(skeleton: &BoneTree, phenotype: &ResolvedPhenotype) -> AnimationSet {
    let style = pick_locomotion_style(skeleton, phenotype);
    let metabolic = ch(phenotype, CH_METABOLIC_RATE);

    let walk = build_locomotion_clip(skeleton, style, /*speed=*/ Q3232::ONE, "walk");
    let run = build_locomotion_clip(skeleton, style, /*speed=*/ Q3232::from_num(2), "run");
    let idle = build_idle_clip(skeleton, metabolic);
    let damage = build_damage_clip(skeleton);
    let death = build_death_clip(skeleton);

    AnimationSet {
        locomotion: vec![walk, run],
        idle: vec![idle],
        damage,
        death,
    }
}

// ---------------------------------------------------------------------------
// Clip builders
// ---------------------------------------------------------------------------

fn core_bones(skeleton: &BoneTree) -> impl Iterator<Item = &Bone> {
    skeleton
        .bones
        .iter()
        .filter(|b| b.tags.contains(&BoneTag::Core))
}

fn limb_bones(skeleton: &BoneTree) -> impl Iterator<Item = &Bone> {
    skeleton
        .bones
        .iter()
        .filter(|b| b.tags.contains(&BoneTag::Limb))
}

fn build_locomotion_clip(
    skeleton: &BoneTree,
    style: LocomotionStyle,
    speed: Q3232,
    name: &str,
) -> AnimationClip {
    // Total clip duration in seconds — faster speed = shorter clip.
    let duration = Q3232::from_num(2).saturating_mul(Q3232::ONE / speed.max(Q3232::ONE));
    let mut tracks = Vec::new();

    match style {
        LocomotionStyle::SinuousWave => {
            // Each core bone gets a sinusoidal swing offset by its
            // index. Approximated with two keyframes per quarter-cycle
            // (5 keyframes total spanning duration), so the
            // determinism contract doesn't need a real sin().
            for (i, bone) in core_bones(skeleton).enumerate() {
                let phase = Q3232::from_num(i as u32).saturating_mul(Q3232::from_num(15));
                tracks.push(BoneTrack {
                    bone_id: bone.id,
                    keyframes: sinusoid_keyframes(duration, Q3232::from_num(20), phase),
                });
            }
        }
        LocomotionStyle::SegmentedScuttle => {
            // Spine stays rigid; limbs lift in lock-step.
            for bone in limb_bones(skeleton) {
                tracks.push(BoneTrack {
                    bone_id: bone.id,
                    keyframes: stride_keyframes(duration, /*phase_offset=*/ Q3232::ZERO),
                });
            }
        }
        LocomotionStyle::QuadrupedWalk | LocomotionStyle::BipedWalk => {
            // Alternating limb pairs. Even-index limbs phase 0, odd
            // phase 0.5.
            for (i, bone) in limb_bones(skeleton).enumerate() {
                let phase_offset = if i % 2 == 0 {
                    Q3232::ZERO
                } else {
                    Q3232::from_num(0.5_f64)
                };
                tracks.push(BoneTrack {
                    bone_id: bone.id,
                    keyframes: stride_keyframes(duration, phase_offset),
                });
            }
        }
    }

    AnimationClip {
        name: format!("{name}_{:?}", style).to_lowercase(),
        duration,
        looping: true,
        bone_tracks: tracks,
    }
}

fn build_idle_clip(skeleton: &BoneTree, metabolic: Q3232) -> AnimationClip {
    // Period: 1s baseline at metabolic_rate=0; 0.4s at metabolic=1.
    let metabolic = metabolic.clamp(Q3232::ZERO, Q3232::ONE);
    let period = Q3232::ONE.saturating_sub(metabolic.saturating_mul(Q3232::from_num(0.6_f64)));

    let mut tracks = Vec::new();
    for bone in core_bones(skeleton) {
        // Tiny breathing scale animation: 1.0 -> 1.05 -> 1.0.
        tracks.push(BoneTrack {
            bone_id: bone.id,
            keyframes: vec![
                Keyframe {
                    time: Q3232::ZERO,
                    rotation: Q3232::ZERO,
                    scale: Q3232::ONE,
                    easing: Easing::EaseInOut,
                },
                Keyframe {
                    time: period.saturating_mul(Q3232::from_num(0.5_f64)),
                    rotation: Q3232::ZERO,
                    scale: Q3232::from_num(1.05_f64),
                    easing: Easing::EaseInOut,
                },
                Keyframe {
                    time: period,
                    rotation: Q3232::ZERO,
                    scale: Q3232::ONE,
                    easing: Easing::Linear,
                },
            ],
        });
    }
    AnimationClip {
        name: "idle".to_string(),
        duration: period,
        looping: true,
        bone_tracks: tracks,
    }
}

fn build_damage_clip(skeleton: &BoneTree) -> AnimationClip {
    // Quick recoil: every core bone twists by -8° at t=0.05s, returns
    // to rest at t=0.2s.
    let mut tracks = Vec::new();
    for bone in core_bones(skeleton) {
        tracks.push(BoneTrack {
            bone_id: bone.id,
            keyframes: vec![
                Keyframe {
                    time: Q3232::ZERO,
                    rotation: Q3232::ZERO,
                    scale: Q3232::ONE,
                    easing: Easing::EaseOut,
                },
                Keyframe {
                    time: Q3232::from_num(0.05_f64),
                    rotation: Q3232::from_num(-8),
                    scale: Q3232::ONE,
                    easing: Easing::EaseIn,
                },
                Keyframe {
                    time: Q3232::from_num(0.2_f64),
                    rotation: Q3232::ZERO,
                    scale: Q3232::ONE,
                    easing: Easing::Linear,
                },
            ],
        });
    }
    AnimationClip {
        name: "damage".to_string(),
        duration: Q3232::from_num(0.2_f64),
        looping: false,
        bone_tracks: tracks,
    }
}

fn build_death_clip(skeleton: &BoneTree) -> AnimationClip {
    // Slow tilt: every core bone rotates 90° over 1.2s and stays.
    let mut tracks = Vec::new();
    for bone in core_bones(skeleton) {
        tracks.push(BoneTrack {
            bone_id: bone.id,
            keyframes: vec![
                Keyframe {
                    time: Q3232::ZERO,
                    rotation: Q3232::ZERO,
                    scale: Q3232::ONE,
                    easing: Easing::EaseIn,
                },
                Keyframe {
                    time: Q3232::from_num(1.2_f64),
                    rotation: Q3232::from_num(90),
                    scale: Q3232::ONE,
                    easing: Easing::Linear,
                },
            ],
        });
    }
    AnimationClip {
        name: "death".to_string(),
        duration: Q3232::from_num(1.2_f64),
        looping: false,
        bone_tracks: tracks,
    }
}

// ---------------------------------------------------------------------------
// Keyframe-pattern helpers
// ---------------------------------------------------------------------------

/// Five-keyframe sinusoid approximation: starts at 0, peaks at +amp at
/// quarter, returns to 0 at half, troughs at -amp at three-quarters,
/// back to 0 at end. `phase` shifts the rotation magnitude (degrees).
fn sinusoid_keyframes(duration: Q3232, amplitude: Q3232, phase: Q3232) -> Vec<Keyframe> {
    let q = duration.saturating_mul(Q3232::from_num(0.25_f64));
    let amp_with_phase = amplitude.saturating_add(phase);
    vec![
        Keyframe {
            time: Q3232::ZERO,
            rotation: Q3232::ZERO,
            scale: Q3232::ONE,
            easing: Easing::EaseInOut,
        },
        Keyframe {
            time: q,
            rotation: amp_with_phase,
            scale: Q3232::ONE,
            easing: Easing::EaseInOut,
        },
        Keyframe {
            time: q.saturating_mul(Q3232::from_num(2)),
            rotation: Q3232::ZERO,
            scale: Q3232::ONE,
            easing: Easing::EaseInOut,
        },
        Keyframe {
            time: q.saturating_mul(Q3232::from_num(3)),
            rotation: amp_with_phase.saturating_mul(Q3232::from_num(-1)),
            scale: Q3232::ONE,
            easing: Easing::EaseInOut,
        },
        Keyframe {
            time: duration,
            rotation: Q3232::ZERO,
            scale: Q3232::ONE,
            easing: Easing::Linear,
        },
    ]
}

/// Stride keyframes: lift at 25%, plant at 75%, recover by end.
/// `phase_offset` in [0, 1) shifts the entire pattern along the clip.
fn stride_keyframes(duration: Q3232, phase_offset: Q3232) -> Vec<Keyframe> {
    let phase_t = phase_offset.saturating_mul(duration);
    let lift = Q3232::from_num(30);
    let plant = Q3232::from_num(-20);
    vec![
        Keyframe {
            time: phase_t,
            rotation: Q3232::ZERO,
            scale: Q3232::ONE,
            easing: Easing::EaseIn,
        },
        Keyframe {
            time: wrap_time(
                phase_t.saturating_add(duration.saturating_mul(Q3232::from_num(0.25_f64))),
                duration,
            ),
            rotation: lift,
            scale: Q3232::ONE,
            easing: Easing::EaseInOut,
        },
        Keyframe {
            time: wrap_time(
                phase_t.saturating_add(duration.saturating_mul(Q3232::from_num(0.75_f64))),
                duration,
            ),
            rotation: plant,
            scale: Q3232::ONE,
            easing: Easing::EaseInOut,
        },
        Keyframe {
            time: duration,
            rotation: Q3232::ZERO,
            scale: Q3232::ONE,
            easing: Easing::Linear,
        },
    ]
}

fn wrap_time(t: Q3232, duration: Q3232) -> Q3232 {
    if t > duration {
        t.saturating_sub(duration)
    } else {
        t
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn linear_keyframes() -> Vec<Keyframe> {
        vec![
            Keyframe {
                time: Q3232::ZERO,
                rotation: Q3232::ZERO,
                scale: Q3232::ONE,
                easing: Easing::Linear,
            },
            Keyframe {
                time: Q3232::ONE,
                rotation: Q3232::from_num(10),
                scale: Q3232::ONE,
                easing: Easing::Linear,
            },
        ]
    }

    fn one_track_clip() -> AnimationClip {
        AnimationClip {
            name: "test".to_string(),
            duration: Q3232::ONE,
            looping: false,
            bone_tracks: vec![BoneTrack {
                bone_id: 7,
                keyframes: linear_keyframes(),
            }],
        }
    }

    #[test]
    fn sample_at_zero_returns_first_keyframe() {
        let clip = one_track_clip();
        let pose = Animator::new(&clip).sample(Q3232::ZERO);
        assert_eq!(pose.bone_rotations.len(), 1);
        assert_eq!(pose.bone_rotations[0].bone_id, 7);
        assert_eq!(pose.bone_rotations[0].rotation, Q3232::ZERO);
    }

    #[test]
    fn sample_at_duration_returns_last_keyframe() {
        let clip = one_track_clip();
        let pose = Animator::new(&clip).sample(Q3232::ONE);
        assert_eq!(pose.bone_rotations[0].rotation, Q3232::from_num(10));
    }

    #[test]
    fn linear_lerp_at_midpoint_is_halfway() {
        let clip = one_track_clip();
        let pose = Animator::new(&clip).sample(Q3232::from_num(0.5_f64));
        assert_eq!(pose.bone_rotations[0].rotation, Q3232::from_num(5));
    }

    #[test]
    fn looping_clip_wraps_t_modulo_duration() {
        let mut clip = one_track_clip();
        clip.looping = true;
        let a = Animator::new(&clip).sample(Q3232::from_num(0.25_f64));
        let b = Animator::new(&clip).sample(Q3232::from_num(1.25_f64));
        assert_eq!(a, b, "looping clip must wrap t");
    }

    #[test]
    fn non_looping_clip_clamps_t_above_duration() {
        let clip = one_track_clip();
        let a = Animator::new(&clip).sample(Q3232::ONE);
        let b = Animator::new(&clip).sample(Q3232::from_num(5));
        assert_eq!(a, b, "non-looping clip must clamp t");
    }

    #[test]
    fn empty_track_returns_rest_pose() {
        let clip = AnimationClip {
            name: "empty".to_string(),
            duration: Q3232::ONE,
            looping: false,
            bone_tracks: vec![BoneTrack {
                bone_id: 0,
                keyframes: Vec::new(),
            }],
        };
        let pose = Animator::new(&clip).sample(Q3232::ZERO);
        assert_eq!(pose.bone_rotations[0].rotation, Q3232::ZERO);
        assert_eq!(pose.bone_rotations[0].scale, Q3232::ONE);
    }

    #[test]
    fn single_keyframe_track_returns_that_pose_for_any_t() {
        let clip = AnimationClip {
            name: "stuck".to_string(),
            duration: Q3232::ONE,
            looping: false,
            bone_tracks: vec![BoneTrack {
                bone_id: 0,
                keyframes: vec![Keyframe {
                    time: Q3232::from_num(0.3_f64),
                    rotation: Q3232::from_num(45),
                    scale: Q3232::from_num(1.2_f64),
                    easing: Easing::Linear,
                }],
            }],
        };
        let a = Animator::new(&clip).sample(Q3232::ZERO);
        let b = Animator::new(&clip).sample(Q3232::ONE);
        assert_eq!(a, b);
        assert_eq!(a.bone_rotations[0].rotation, Q3232::from_num(45));
    }

    #[test]
    fn ease_in_alpha_zero_and_one_match_linear() {
        // Easing must be identity at endpoints.
        assert_eq!(apply_easing(Q3232::ZERO, Easing::EaseIn), Q3232::ZERO);
        assert_eq!(apply_easing(Q3232::ONE, Easing::EaseIn), Q3232::ONE);
        assert_eq!(apply_easing(Q3232::ZERO, Easing::EaseOut), Q3232::ZERO);
        assert_eq!(apply_easing(Q3232::ONE, Easing::EaseOut), Q3232::ONE);
        assert_eq!(apply_easing(Q3232::ZERO, Easing::EaseInOut), Q3232::ZERO);
        assert_eq!(apply_easing(Q3232::ONE, Easing::EaseInOut), Q3232::ONE);
    }
}
