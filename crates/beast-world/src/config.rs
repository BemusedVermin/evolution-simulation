//! [`WorldConfig`] — input parameters for [`crate::generate_archipelago`].
//!
//! Thresholds use [`Q3232`] so they match the runtime numeric type
//! used everywhere else in the project. `width` and `height` are
//! `u32` (cell counts).
//!
//! All thresholds are in the unit interval `[0, 1]` and operate on
//! the **normalised** noise output:
//!
//! * Elevation thresholds (`sea_level`, `mountain_threshold`) compare
//!   against `(elevation + 1.0) / 2.0` — i.e., elevation remapped
//!   from `[-1, 1]` to `[0, 1]`.
//! * Moisture thresholds (`desert_threshold`, `forest_threshold`)
//!   compare against `(moisture + 1.0) / 2.0`.
//! * `tundra_latitude_threshold` compares against
//!   `|2 * (y / height) - 1|` — 0 at the equator, 1 at the poles.

use beast_core::Q3232;

/// World-generation parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldConfig {
    /// Number of cells along the X axis. Must be ≥ 1.
    pub width: u32,
    /// Number of cells along the Y axis. Must be ≥ 1.
    pub height: u32,
    /// Scale at which the noise lattice is sampled. Smaller values
    /// produce larger, smoother features. `4.0` is a reasonable
    /// default for a 64×64 grid.
    pub frequency: Q3232,
    /// Number of fBm octaves. More octaves = more fine detail at
    /// the cost of runtime. `4` is a sensible default.
    pub octaves: u32,
    /// Per-octave amplitude multiplier (typical `0.5`).
    pub gain: Q3232,
    /// Per-octave frequency multiplier (typical `2.0`).
    pub lacunarity: Q3232,
    /// Cells with normalised elevation below this are [`crate::BiomeTag::Ocean`].
    /// Must be in `[0, 1]`.
    pub sea_level: Q3232,
    /// Cells with normalised elevation above this become
    /// [`crate::BiomeTag::Mountain`]. Must be > `sea_level` and ≤ `1`.
    pub mountain_threshold: Q3232,
    /// Cells with normalised |latitude| above this become
    /// [`crate::BiomeTag::Tundra`] (overriding moisture). Must be in `[0, 1]`.
    pub tundra_latitude_threshold: Q3232,
    /// Cells with normalised moisture below this become
    /// [`crate::BiomeTag::Desert`] (when above sea level and below
    /// mountain). Must be in `[0, 1]`.
    pub desert_threshold: Q3232,
    /// Cells with normalised moisture above this become
    /// [`crate::BiomeTag::Forest`] (when above sea level and below
    /// mountain). Must be in `[0, 1]` and ≥ `desert_threshold`.
    pub forest_threshold: Q3232,
}

impl WorldConfig {
    /// A reasonable default config for a 64×64 archipelago.
    ///
    /// Tuned so a default seed produces ~40% ocean, ~10% mountain,
    /// ~10% tundra, and the remaining ~40% split between forest /
    /// plains / desert. Tests in `archipelago::tests` lock in these
    /// proportions for the canonical seed `0xCAFE_BABE`.
    #[must_use]
    pub fn default_archipelago() -> Self {
        Self {
            width: 64,
            height: 64,
            frequency: Q3232::from_num(4_i32),
            octaves: 4,
            gain: Q3232::from_num(0.5_f64),
            lacunarity: Q3232::from_num(2_i32),
            sea_level: Q3232::from_num(0.45_f64),
            mountain_threshold: Q3232::from_num(0.78_f64),
            tundra_latitude_threshold: Q3232::from_num(0.85_f64),
            desert_threshold: Q3232::from_num(0.35_f64),
            forest_threshold: Q3232::from_num(0.65_f64),
        }
    }
}
