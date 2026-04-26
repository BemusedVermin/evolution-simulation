//! Visual directives — the IR consumed by the procedural visual pipeline.
//!
//! Visual directives are produced by the phenotype interpreter (Stage 4)
//! and consumed by [`crate::pipeline`] to build a [`crate::blueprint::CreatureBlueprint`].
//! See `documentation/systems/10_procgen_visual_pipeline.md` §3.1 for the
//! authoritative shape.
//!
//! For S9.5 the type lives in `beast-render` because the interpreter
//! doesn't yet emit Stage 4 directives — tests construct directives by
//! hand. When `beast-interpreter` grows directive emission, the natural
//! refactor is to lift this module to a shared L2 / L3 crate so both
//! producer and consumer can depend on it. Until then, callers ferry
//! `Vec<VisualDirective>` across the boundary themselves.
//!
//! # Determinism
//!
//! Every parameter type uses [`Q3232`] for numeric values that flow into
//! the pipeline's pure compile step (skeleton + volumes + surfaces + base
//! material). Floats only appear in fields that the renderer consumes
//! directly without feeding back into the blueprint hash (e.g. animation
//! timings — those live in [`crate::blueprint`]).

use beast_core::{BodySite, Q3232};

/// One directive emitted for a single body region.
///
/// `priority` lets later substages resolve conflicts deterministically
/// (highest priority wins for non-stackable directive types). Equal
/// priorities resolve by sort order — see [`crate::pipeline::canonicalise`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VisualDirective {
    /// Stable identifier across the directive set produced for one
    /// phenotype. Sorts deterministically.
    pub id: u32,
    /// Anatomical region this directive applies to.
    pub body_region: BodySite,
    /// Type-specific parameters.
    pub params: DirectiveParams,
    /// Tie-breaker for conflicting directives. Higher = more prominent.
    pub priority: u32,
}

/// Parameter payload, switched on directive kind.
///
/// Variants map 1:1 to the eight directive types in
/// `documentation/systems/10_procgen_visual_pipeline.md` §3.1.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DirectiveParams {
    /// Push surface features outward (spikes, horns, plates).
    Protrude(Protrude),
    /// Add hard texture (scales, plates) to the surface.
    Harden(Harden),
    /// Smooth / make translucent (membranes).
    Soften(Soften),
    /// Open an aperture (mouth, nostril, gill).
    Orifice(Orifice),
    /// Append a non-locomotion limb (fin, wing, antenna).
    Append(Append),
    /// Bloat a volume (puffer-fish style).
    Inflate(Inflate),
    /// Surface-pattern overlay (mottled, striped).
    Texture(Texture),
    /// Color the volume.
    Colorize(Colorize),
}

// ---------------------------------------------------------------------------
// Parameter records
// ---------------------------------------------------------------------------

/// `Protrude` parameters: spike / horn / plate / etc.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Protrude {
    pub shape: ProtrusionShape,
    pub scale: Q3232,
    pub density: u8,
    pub distribution: Distribution,
    pub surface_region: SurfaceRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtrusionShape {
    Spike,
    Horn,
    Plate,
    Knob,
    Hook,
    Tendril,
    Bulb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Distribution {
    Regular,
    Random,
    Cluster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceRegion {
    Dorsal,
    Ventral,
    Lateral,
    Anterior,
    AllSurface,
}

/// `Harden` parameters: scales / plates / ridges.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Harden {
    pub roughness: Q3232,
    pub segmentation: u8,
    pub pattern: HardenPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardenPattern {
    Scales,
    Plates,
    Ridges,
    Cracked,
}

/// `Soften` parameters: smoothness + translucency.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Soften {
    pub smoothness: Q3232,
    pub transparency: Q3232,
}

/// `Orifice` parameters: aperture size + position.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Orifice {
    pub size: Q3232,
    /// Position along body length, [0,1].
    pub position: Q3232,
    pub count: u8,
    pub orientation: OrificeOrientation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrificeOrientation {
    Forward,
    Lateral,
    Ventral,
}

/// `Append` parameters: non-locomotion appendage attached to a region.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Append {
    pub appendage_type: AppendageKind,
    pub count: u8,
    /// Position along body, [0,1].
    pub position: Q3232,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppendageKind {
    Fin,
    Wing,
    Tentacle,
    Horn,
    Crest,
}

/// `Inflate` parameters: scale a volume up or down.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Inflate {
    /// Scale multiplier; the design doc expects values in `[0.8, 2.0]`.
    pub scale: Q3232,
}

/// `Texture` parameters: surface pattern overlay.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Texture {
    pub pattern: TexturePattern,
    pub scale: Q3232,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TexturePattern {
    Scales,
    Bumps,
    Ridges,
    Mottled,
    Striped,
    Spotted,
    Pitted,
    Cracked,
    Smooth,
    Rocky,
    Wrinkled,
    Crystalline,
}

/// `Colorize` parameters: base color + optional emission + optional pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Colorize {
    pub base_color: ColorSpec,
    pub emission: Option<ColorSpec>,
    pub emission_intensity: Q3232,
    pub pattern: Option<TexturePattern>,
    pub pattern_color_secondary: Option<ColorSpec>,
    pub contrast: Q3232,
}

/// HSV(A) color spec. Hue is 0..360, others are [0,1] in Q32.32.
///
/// `hue == None` is the **biome-color sentinel** described in §4.4: the
/// material-assignment substage substitutes the biome's dominant color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorSpec {
    /// Hue in degrees [0, 360]. `None` = use biome color (sentinel).
    pub hue: Option<Q3232>,
    pub saturation: Q3232,
    pub value: Q3232,
    pub alpha: Q3232,
}

impl ColorSpec {
    /// Build a fully-specified HSV color (no biome sentinel).
    pub fn rgb(hue: Q3232, saturation: Q3232, value: Q3232) -> Self {
        Self {
            hue: Some(hue),
            saturation,
            value,
            alpha: Q3232::ONE,
        }
    }

    /// Biome-color sentinel — the renderer fills in the biome's dominant
    /// color at material-assignment time.
    pub fn biome_color(saturation: Q3232, value: Q3232) -> Self {
        Self {
            hue: None,
            saturation,
            value,
            alpha: Q3232::ONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directive_eq_is_value_based() {
        let a = VisualDirective {
            id: 1,
            body_region: BodySite::Core,
            params: DirectiveParams::Inflate(Inflate { scale: Q3232::ONE }),
            priority: 0,
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, a.clone());
    }

    #[test]
    fn color_spec_biome_sentinel_is_distinguishable() {
        let biome = ColorSpec::biome_color(Q3232::ONE, Q3232::ONE);
        let rgb = ColorSpec::rgb(Q3232::ONE, Q3232::ONE, Q3232::ONE);
        assert!(biome.hue.is_none());
        assert!(rgb.hue.is_some());
        assert_ne!(biome, rgb);
    }
}
