//! [`Archipelago`] grid + [`generate_archipelago`] generator.
//!
//! Output is row-major: cell `(x, y)` is at index `y * width + x`.
//! Iteration order is left-to-right, top-to-bottom, matching every
//! consumer that scans the grid as a sorted sequence.

#![allow(clippy::float_arithmetic)] // boundary code, see crate docs

use beast_core::Q3232;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::biome_tag::BiomeTag;
use crate::config::WorldConfig;
use crate::noise::fbm_2d;

/// World-generation error variants.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum GenerationError {
    /// `width` or `height` was zero.
    #[error("world dimensions must be at least 1x1; got {width}x{height}")]
    EmptyDimensions {
        /// The supplied width.
        width: u32,
        /// The supplied height.
        height: u32,
    },
    /// `octaves` was zero.
    #[error("octaves must be at least 1")]
    ZeroOctaves,
    /// One of the threshold pairs was inverted (e.g.,
    /// `mountain_threshold <= sea_level`).
    #[error("threshold ordering invalid: {detail}")]
    InvalidThresholds {
        /// Which pair was inverted, in human-readable form.
        detail: String,
    },
}

/// A generated archipelago — a row-major grid of [`BiomeTag`] cells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Archipelago {
    /// Grid width in cells.
    pub width: u32,
    /// Grid height in cells.
    pub height: u32,
    /// World seed used to generate this grid. Stored so the
    /// archipelago can be reproduced from the seed alone.
    pub seed: u64,
    /// Row-major cell grid; length is `width * height`.
    pub cells: Vec<BiomeTag>,
}

impl Archipelago {
    /// Look up the biome tag at `(x, y)`. Returns `None` if either
    /// coordinate is out of bounds.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<BiomeTag> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.cells.get(idx).copied()
    }

    /// Number of cells of a given biome.
    #[must_use]
    pub fn count_of(&self, tag: BiomeTag) -> usize {
        self.cells.iter().filter(|&&c| c == tag).count()
    }
}

/// Generate an archipelago from a [`WorldConfig`] and a `seed`.
///
/// Per [`crate`] docs, the generator is a pure function of
/// `(config, seed)`: identical inputs produce identical output bytes
/// across processes and runs.
///
/// # Errors
///
/// * [`GenerationError::EmptyDimensions`] when `width` or `height`
///   is zero.
/// * [`GenerationError::ZeroOctaves`] when `octaves` is zero.
/// * [`GenerationError::InvalidThresholds`] when a threshold pair is
///   in the wrong order (e.g., `mountain_threshold ≤ sea_level`).
pub fn generate_archipelago(
    config: &WorldConfig,
    seed: u64,
) -> Result<Archipelago, GenerationError> {
    validate_config(config)?;

    // Two independent noise channels: elevation and moisture.
    // Different per-channel seeds keep them decorrelated.
    let elevation_seed = seed;
    let moisture_seed = seed ^ 0x5A5A_5A5A_5A5A_5A5A;

    let width = config.width;
    let height = config.height;

    let frequency = q_to_f64(config.frequency);
    let gain = q_to_f64(config.gain);
    let lacunarity = q_to_f64(config.lacunarity);

    let sea_level = q_to_f64(config.sea_level);
    let mountain_threshold = q_to_f64(config.mountain_threshold);
    let tundra_threshold = q_to_f64(config.tundra_latitude_threshold);
    let desert_threshold = q_to_f64(config.desert_threshold);
    let forest_threshold = q_to_f64(config.forest_threshold);

    // Width and height in `f64` for normalisation. Promoted to f64
    // once and reused inside the loop.
    let width_f = width as f64;
    let height_f = height as f64;

    let mut cells = Vec::with_capacity((width as usize) * (height as usize));

    // Iteration order: row-major, y in 0..height (outer), x in 0..width
    // (inner). Deterministic and matches the storage layout, so a
    // consumer that scans `cells.iter().enumerate()` gets cells in
    // the same order they were produced.
    for y in 0..height {
        // Normalised |latitude|: 0 at the equator, 1 at the poles.
        let lat = ((y as f64 + 0.5) / height_f) * 2.0 - 1.0;
        let abs_lat = lat.abs();

        for x in 0..width {
            // Sample noise at normalised lattice coordinates.
            let nx = (x as f64 + 0.5) / width_f * frequency;
            let ny = (y as f64 + 0.5) / height_f * frequency;

            let elevation = fbm_2d(elevation_seed, nx, ny, config.octaves, lacunarity, gain);
            let elevation_unit = (elevation + 1.0) * 0.5;

            let moisture = fbm_2d(moisture_seed, nx, ny, config.octaves, lacunarity, gain);
            let moisture_unit = (moisture + 1.0) * 0.5;

            let tag = classify(
                elevation_unit,
                moisture_unit,
                abs_lat,
                sea_level,
                mountain_threshold,
                tundra_threshold,
                desert_threshold,
                forest_threshold,
            );
            cells.push(tag);
        }
    }

    Ok(Archipelago {
        width,
        height,
        seed,
        cells,
    })
}

fn validate_config(config: &WorldConfig) -> Result<(), GenerationError> {
    if config.width == 0 || config.height == 0 {
        return Err(GenerationError::EmptyDimensions {
            width: config.width,
            height: config.height,
        });
    }
    if config.octaves == 0 {
        return Err(GenerationError::ZeroOctaves);
    }
    if config.mountain_threshold <= config.sea_level {
        return Err(GenerationError::InvalidThresholds {
            detail: format!(
                "mountain_threshold ({:?}) must be > sea_level ({:?})",
                config.mountain_threshold, config.sea_level,
            ),
        });
    }
    if config.forest_threshold < config.desert_threshold {
        return Err(GenerationError::InvalidThresholds {
            detail: format!(
                "forest_threshold ({:?}) must be >= desert_threshold ({:?})",
                config.forest_threshold, config.desert_threshold,
            ),
        });
    }
    Ok(())
}

/// Pure classification: takes already-normalised noise samples plus
/// thresholds, returns a [`BiomeTag`]. Pulled out so the logic is
/// unit-testable without spinning up the noise pipeline.
#[allow(clippy::too_many_arguments)]
fn classify(
    elevation_unit: f64,
    moisture_unit: f64,
    abs_lat: f64,
    sea_level: f64,
    mountain_threshold: f64,
    tundra_threshold: f64,
    desert_threshold: f64,
    forest_threshold: f64,
) -> BiomeTag {
    if elevation_unit < sea_level {
        return BiomeTag::Ocean;
    }
    if elevation_unit > mountain_threshold {
        return BiomeTag::Mountain;
    }
    if abs_lat > tundra_threshold {
        return BiomeTag::Tundra;
    }
    if moisture_unit < desert_threshold {
        return BiomeTag::Desert;
    }
    if moisture_unit > forest_threshold {
        return BiomeTag::Forest;
    }
    BiomeTag::Plains
}

/// Convert Q3232 to f64 for the float-only noise pipeline. Lossy
/// for sub-2^-32 fractions (none of which we use), exact for every
/// `from_num(integer)` and `from_num(float)` value `WorldConfig`
/// stores. Centralised here so the conversion has one home and the
/// `clippy::float_arithmetic` allow stays narrow.
fn q_to_f64(q: Q3232) -> f64 {
    q.to_num::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_picks_ocean_for_low_elevation() {
        assert_eq!(
            classify(0.1, 0.5, 0.0, 0.45, 0.78, 0.85, 0.35, 0.65),
            BiomeTag::Ocean
        );
    }

    #[test]
    fn classify_picks_mountain_for_high_elevation() {
        assert_eq!(
            classify(0.9, 0.5, 0.0, 0.45, 0.78, 0.85, 0.35, 0.65),
            BiomeTag::Mountain
        );
    }

    #[test]
    fn classify_picks_tundra_for_high_latitude_above_sea_level() {
        // High |lat| beats moisture-driven biomes when above sea
        // level and below mountain.
        assert_eq!(
            classify(0.6, 0.5, 0.9, 0.45, 0.78, 0.85, 0.35, 0.65),
            BiomeTag::Tundra
        );
    }

    #[test]
    fn classify_picks_desert_for_dry_mid_latitude() {
        assert_eq!(
            classify(0.6, 0.2, 0.0, 0.45, 0.78, 0.85, 0.35, 0.65),
            BiomeTag::Desert
        );
    }

    #[test]
    fn classify_picks_forest_for_wet_mid_latitude() {
        assert_eq!(
            classify(0.6, 0.8, 0.0, 0.45, 0.78, 0.85, 0.35, 0.65),
            BiomeTag::Forest
        );
    }

    #[test]
    fn classify_picks_plains_for_balanced_mid_latitude() {
        assert_eq!(
            classify(0.6, 0.5, 0.0, 0.45, 0.78, 0.85, 0.35, 0.65),
            BiomeTag::Plains
        );
    }

    #[test]
    fn classify_priority_ocean_beats_tundra_at_high_latitude() {
        // Below sea level + high latitude → still Ocean (the polar
        // sea is open water, not Tundra).
        assert_eq!(
            classify(0.1, 0.5, 0.95, 0.45, 0.78, 0.85, 0.35, 0.65),
            BiomeTag::Ocean
        );
    }

    #[test]
    fn classify_priority_mountain_beats_tundra_at_high_latitude() {
        // High elevation + high latitude → Mountain (the peak),
        // not Tundra. Could revisit if a "polar peak" combined
        // biome makes sense post-MVP.
        assert_eq!(
            classify(0.9, 0.5, 0.95, 0.45, 0.78, 0.85, 0.35, 0.65),
            BiomeTag::Mountain
        );
    }

    #[test]
    fn rejects_zero_dimensions() {
        let mut cfg = WorldConfig::default_archipelago();
        cfg.width = 0;
        assert!(matches!(
            generate_archipelago(&cfg, 0).unwrap_err(),
            GenerationError::EmptyDimensions {
                width: 0,
                height: 64
            }
        ));
    }

    #[test]
    fn rejects_zero_octaves() {
        let mut cfg = WorldConfig::default_archipelago();
        cfg.octaves = 0;
        assert!(matches!(
            generate_archipelago(&cfg, 0).unwrap_err(),
            GenerationError::ZeroOctaves
        ));
    }

    #[test]
    fn rejects_inverted_elevation_thresholds() {
        let mut cfg = WorldConfig::default_archipelago();
        cfg.sea_level = Q3232::from_num(0.9_f64);
        cfg.mountain_threshold = Q3232::from_num(0.5_f64);
        let err = generate_archipelago(&cfg, 0).unwrap_err();
        assert!(matches!(err, GenerationError::InvalidThresholds { .. }));
    }

    #[test]
    fn rejects_inverted_moisture_thresholds() {
        let mut cfg = WorldConfig::default_archipelago();
        cfg.desert_threshold = Q3232::from_num(0.7_f64);
        cfg.forest_threshold = Q3232::from_num(0.5_f64);
        let err = generate_archipelago(&cfg, 0).unwrap_err();
        assert!(matches!(err, GenerationError::InvalidThresholds { .. }));
    }

    #[test]
    fn produces_grid_of_correct_size() {
        let cfg = WorldConfig::default_archipelago();
        let world = generate_archipelago(&cfg, 0).unwrap();
        assert_eq!(world.width, 64);
        assert_eq!(world.height, 64);
        assert_eq!(world.cells.len(), 64 * 64);
    }

    #[test]
    fn generation_is_deterministic_across_calls() {
        let cfg = WorldConfig::default_archipelago();
        let a = generate_archipelago(&cfg, 0xCAFE_BABE).unwrap();
        let b = generate_archipelago(&cfg, 0xCAFE_BABE).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_produce_different_worlds() {
        let cfg = WorldConfig::default_archipelago();
        let a = generate_archipelago(&cfg, 0x1).unwrap();
        let b = generate_archipelago(&cfg, 0x2).unwrap();
        assert_ne!(a.cells, b.cells);
    }

    #[test]
    fn seed_is_stored_in_archipelago() {
        let cfg = WorldConfig::default_archipelago();
        let world = generate_archipelago(&cfg, 0xCAFE_BABE).unwrap();
        assert_eq!(world.seed, 0xCAFE_BABE);
    }

    #[test]
    fn get_returns_none_out_of_bounds() {
        let cfg = WorldConfig::default_archipelago();
        let world = generate_archipelago(&cfg, 0).unwrap();
        assert!(world.get(64, 0).is_none());
        assert!(world.get(0, 64).is_none());
        assert!(world.get(100, 100).is_none());
    }

    #[test]
    fn get_returns_some_in_bounds() {
        let cfg = WorldConfig::default_archipelago();
        let world = generate_archipelago(&cfg, 0).unwrap();
        for y in 0..64 {
            for x in 0..64 {
                assert!(world.get(x, y).is_some(), "missing cell at ({x},{y})");
            }
        }
    }

    #[test]
    fn default_archipelago_contains_every_biome_for_canonical_seed() {
        // Fixture seed picked so the default config produces at
        // least one cell of every biome. If a future tuning change
        // breaks this, that's the signal to retune (open-water
        // worlds aren't fun to play).
        let cfg = WorldConfig::default_archipelago();
        let world = generate_archipelago(&cfg, 0xCAFE_BABE).unwrap();
        for tag in [
            BiomeTag::Ocean,
            BiomeTag::Forest,
            BiomeTag::Plains,
            BiomeTag::Desert,
            BiomeTag::Mountain,
            BiomeTag::Tundra,
        ] {
            assert!(
                world.count_of(tag) > 0,
                "default world has no {tag:?} cells (seed 0xCAFE_BABE)",
            );
        }
    }

    #[test]
    fn default_archipelago_proportions_are_sensible() {
        // A playable world has a meaningful land/sea split. Lock
        // the proportions in a wide band so a noise-tweak that
        // produces a 100%-ocean grid (or 100%-mountain) is caught.
        let cfg = WorldConfig::default_archipelago();
        let world = generate_archipelago(&cfg, 0xCAFE_BABE).unwrap();
        let total = world.cells.len();
        let ocean_pct = world.count_of(BiomeTag::Ocean) * 100 / total;
        assert!(
            (10..=80).contains(&ocean_pct),
            "ocean coverage {}% is implausible (expected 10..=80)",
            ocean_pct
        );
    }

    #[test]
    fn small_world_generates_without_panic() {
        // 1x1 is the smallest legal world. Stress the bounds of
        // every loop and lookup.
        let mut cfg = WorldConfig::default_archipelago();
        cfg.width = 1;
        cfg.height = 1;
        let world = generate_archipelago(&cfg, 0).unwrap();
        assert_eq!(world.cells.len(), 1);
        assert!(world.get(0, 0).is_some());
        assert!(world.get(1, 0).is_none());
    }
}
