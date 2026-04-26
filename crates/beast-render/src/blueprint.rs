//! Output of the visual pipeline: a [`CreatureBlueprint`].
//!
//! Renderer-agnostic description of a creature's form (skeleton, volumes,
//! surface details, materials, effects). Animation rigging is added in
//! S9.6.
//!
//! All numeric fields that participate in the determinism contract use
//! [`Q3232`]. Floating point values are forbidden in this module — see
//! `documentation/INVARIANTS.md` §1.

use beast_core::{BodySite, Q3232};

use crate::directive::{
    AppendageKind, ColorSpec, Distribution, HardenPattern, OrificeOrientation, ProtrusionShape,
    SurfaceRegion, TexturePattern,
};

// ---------------------------------------------------------------------------
// Top-level blueprint
// ---------------------------------------------------------------------------

/// Renderer-agnostic creature description. Hash of this struct uniquely
/// identifies the visual output for a given (genotype, biome) pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CreatureBlueprint {
    pub skeleton: BoneTree,
    pub volumes: Vec<Volume>,
    pub surfaces: Vec<SurfaceDetail>,
    pub materials: Vec<MaterialRegion>,
    pub effects: Vec<AttachedEffect>,
    /// Animation rig — locomotion / idle / damage / death clips. See
    /// [`crate::animation`].
    pub animations: crate::animation::AnimationSet,
    pub metadata: BlueprintMetadata,
}

/// Stable metadata describing the blueprint as a whole.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlueprintMetadata {
    /// Axis-aligned bounding box in local space (Q3232 units, one unit
    /// = one body length).
    pub bounding_box: Aabb,
    /// Display name supplied by the chronicler / UI layer; the visual
    /// pipeline never inspects the contents.
    pub display_name: String,
}

/// Axis-aligned bounding box in Q3232 local space.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

/// 3-component vector. Z is held at 0 by the current pipeline (the design
/// doc keeps blueprints 2D-compatible for sprite renderers); kept three
/// dimensions so 3D renderers can consume the same blueprint without a
/// shape change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Vec3 {
    pub x: Q3232,
    pub y: Q3232,
    pub z: Q3232,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: Q3232::ZERO,
        y: Q3232::ZERO,
        z: Q3232::ZERO,
    };

    pub const fn new(x: Q3232, y: Q3232, z: Q3232) -> Self {
        Self { x, y, z }
    }
}

// ---------------------------------------------------------------------------
// Skeleton
// ---------------------------------------------------------------------------

/// A bone hierarchy. Stores bones in declaration order; child / parent
/// relations are encoded by [`Bone::parent_id`] (root has `None`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoneTree {
    pub bones: Vec<Bone>,
}

impl BoneTree {
    /// Total bone count.
    pub fn len(&self) -> usize {
        self.bones.len()
    }

    /// True iff the tree has no bones.
    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bone {
    pub id: u32,
    pub name: String,
    /// `None` for the root bone.
    pub parent_id: Option<u32>,
    /// Position relative to parent.
    pub local_position: Vec3,
    /// Resting rotation in milli-degrees (use Q3232 directly).
    pub local_rotation: Q3232,
    pub length: Q3232,
    pub thickness: Q3232,
    pub tags: Vec<BoneTag>,
    pub constraints: JointConstraint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BoneTag {
    Core,
    Head,
    Tail,
    Limb,
    LimbTip,
    Appendage,
    Jaw,
    /// Bilateral mirror flag — this bone has a mirror counterpart on
    /// the opposite side.
    Symmetric,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JointConstraint {
    /// Min angle in degrees.
    pub min_angle: Q3232,
    /// Max angle in degrees.
    pub max_angle: Q3232,
    /// Stiffness in [0,1] — 0 = floppy, 1 = rigid.
    pub stiffness: Q3232,
    /// Resting angle in degrees.
    pub preferred: Q3232,
}

// ---------------------------------------------------------------------------
// Volumes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Volume {
    pub id: u32,
    pub attached_bones: Vec<u32>,
    pub shape: VolumeShape,
    pub symmetry: SymmetryMode,
    /// Layer index for overlapping volumes; higher = more outer.
    pub layer: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VolumeShape {
    /// Full ellipsoid in three radii.
    Ellipsoid { radii: Vec3 },
    /// Capsule along the bone axis.
    Capsule { radius: Q3232, length: Q3232 },
    /// Tapered cone-ish.
    Tapered {
        radius_start: Q3232,
        radius_end: Q3232,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymmetryMode {
    None,
    BilateralX,
    BilateralY,
    Radial { n: u8 },
}

// ---------------------------------------------------------------------------
// Surface details
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SurfaceDetail {
    pub id: u32,
    pub target_volume: u32,
    pub detail: SurfaceType,
    pub placement: Placement,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SurfaceType {
    /// Outward feature (spike, horn, plate).
    Protrusion {
        shape: ProtrusionShape,
        height: Q3232,
        base_width: Q3232,
        taper: Q3232,
    },
    /// Surface texture overlay.
    Texture {
        pattern: TexturePattern,
        scale: Q3232,
        depth: Q3232,
    },
    /// Hard armor pattern.
    Hardening {
        pattern: HardenPattern,
        roughness: Q3232,
        segmentation: u8,
    },
    /// Aperture / mouth.
    Orifice {
        radius: Q3232,
        depth: Q3232,
        rim_width: Q3232,
        orientation: OrificeOrientation,
    },
    /// Soft / membranous patch.
    Membrane {
        smoothness: Q3232,
        transparency: Q3232,
    },
    /// Appendage attachment point (the geometry itself is built as a
    /// child bone in [`BoneTree`]; the surface entry records the
    /// attachment metadata).
    AppendageAttachment { kind: AppendageKind, count: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Placement {
    /// Position along bone, [0,1].
    pub along_bone: Q3232,
    /// Surface region targeted (for protrusions).
    pub surface_region: SurfaceRegion,
    /// Number of features.
    pub count: u8,
    pub distribution: Distribution,
    pub mirror: bool,
}

// ---------------------------------------------------------------------------
// Materials
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialRegion {
    pub id: u32,
    pub target: MaterialTarget,
    pub props: MaterialProps,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MaterialTarget {
    /// Applies to the whole creature unless overridden.
    Global,
    /// Applies to a specific volume.
    Volume { volume_id: u32 },
    /// Applies to a specific surface detail.
    Detail { detail_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialProps {
    pub base_color: ColorSpec,
    /// [0,1]. 0 = mirror, 1 = chalk.
    pub roughness: Q3232,
    /// [0,1]. 0 = organic, 1 = shiny.
    pub metallic: Q3232,
    /// [0,1] — translucency through the surface.
    pub subsurface: Q3232,
    /// Optional emission color (None = no glow).
    pub emission: Option<ColorSpec>,
    pub emission_power: Q3232,
    pub pattern: Option<PatternOverlay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternOverlay {
    pub pattern: TexturePattern,
    pub color_a: ColorSpec,
    pub color_b: ColorSpec,
    pub scale: Q3232,
    pub contrast: Q3232,
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttachedEffect {
    pub id: u32,
    pub attach: AttachPoint,
    pub spec: EffectSpec,
    pub trigger: EffectTrigger,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttachPoint {
    /// Attached to a body region as a whole (used for full-body glows).
    Region { region: BodySite },
    /// Attached to a volume.
    Volume { volume_id: u32 },
    /// Attached to a surface detail.
    Detail { detail_id: u32 },
    /// Spherical aura around the bounding-box centre.
    Aura { radius: Q3232 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectSpec {
    pub kind: EffectKind,
    pub color: ColorSpec,
    /// Emission rate, units/tick.
    pub rate: Q3232,
    /// Particle size.
    pub size: Q3232,
    /// Particle lifetime in ticks.
    pub lifetime: Q3232,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectKind {
    Glow,
    Drip,
    Particle,
    Smoke,
    Spore,
    Spark,
    Trail,
    Bubble,
    Ring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectTrigger {
    Always,
    WhenMoving,
    WhenAttacking,
    WhenDamaged,
    WhenInCombat,
    WhenIdle,
}
