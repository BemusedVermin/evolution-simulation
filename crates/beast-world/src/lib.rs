//! Beast Evolution Game — Layer 3 procedural world generation.
//!
//! Sprint S8.1 (issue #143): generates a deterministic archipelago — a
//! 2D grid of [`BiomeTag`] cells — from a seed and a [`WorldConfig`].
//!
//! # Determinism
//!
//! Per `documentation/INVARIANTS.md` §1, world generation must be
//! seed-reproducible. This crate honours that with two strategies:
//!
//! * **Hash-based value noise**: every `(ix, iy)` integer corner draws
//!   a value from a deterministic SplitMix64 mix of `(seed, octave,
//!   ix, iy)`. No mutable state, no PRNG sequencing — calling
//!   [`value_noise_2d`] twice with the same arguments returns the same
//!   `f64` byte-for-byte. This sidesteps the "did the noise function
//!   advance state in a different order this run?" class of bug.
//! * **Sorted iteration**: the public surface is a row-major
//!   `Vec<BiomeTag>` so callers can scan it left-to-right, top-to-
//!   bottom without depending on hash randomization.
//!
//! Floats are used internally during generation (the noise lattice is
//! naturally `f64`) but every value that crosses into sim state is
//! quantised before it leaves this crate. Mirrors how `beast-channels`
//! loads `f64` from JSON manifests and exits to Q3232 at the boundary.
//!
//! # Layer DAG
//!
//! Sits at L3 — depends on `beast-core` only. The output uses
//! `BiomeTag` (a local enum mirroring `beast_ecs::components::BiomeKind`)
//! rather than `BiomeKind` itself so this crate doesn't depend up on
//! `beast-ecs`. The spawner (S8.4) bridges the two by mapping
//! `BiomeTag::as_str() -> BiomeKind` once both have landed on master.
//!
//! # Algorithm sketch
//!
//! Per cell `(x, y)`:
//!
//! 1. **Elevation** = sum of 4 octaves of value noise, normalised to
//!    `[-1, 1]`. Higher = more land.
//! 2. **Moisture** = sum of `config.octaves` octaves of value noise
//!    (same fBm shape as elevation, with a different per-channel
//!    seed derived deterministically from the world `seed` via
//!    `splitmix64(seed ^ MAGIC)` — only the world `seed` needs to
//!    be persisted to reproduce both channels).
//! 3. **Latitude** = `|y / height - 0.5| * 2`, in `[0, 1]` — 0 at the
//!    equator, 1 at the poles. Used to gate tundra.
//! 4. **Classification**:
//!    - `elevation < sea_level` → [`BiomeTag::Ocean`]
//!    - `elevation > mountain_threshold` → [`BiomeTag::Mountain`]
//!    - `latitude > tundra_threshold` → [`BiomeTag::Tundra`]
//!    - `moisture < desert_threshold` → [`BiomeTag::Desert`]
//!    - `moisture > forest_threshold` → [`BiomeTag::Forest`]
//!    - else → [`BiomeTag::Plains`]

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

/// Archipelago generation: seed + [`WorldConfig`] → row-major [`BiomeTag`] grid.
pub mod archipelago;
/// `BiomeTag` enum — local mirror of `beast_ecs::components::BiomeKind`.
pub mod biome_tag;
/// `WorldConfig`: tunable thresholds for elevation, moisture, and latitude bands.
pub mod config;
/// Deterministic value-noise primitives ([`value_noise_2d`], [`fbm_2d`]).
pub mod noise;

pub use archipelago::{generate_archipelago, Archipelago, GenerationError};
pub use biome_tag::BiomeTag;
pub use config::WorldConfig;
pub use noise::{fbm_2d, value_noise_2d};
