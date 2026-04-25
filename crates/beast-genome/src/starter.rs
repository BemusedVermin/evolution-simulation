//! Starter species genomes — Sprint S8.3 (issue #145).
//!
//! Three hand-tuned genomes ship in the MVP so the world spawner
//! (S8.4) has something to populate biomes with on day one. Each spec
//! is a deterministic, registry-agnostic blueprint: it names channels
//! by id (matching `documentation/schemas/channel_manifest.schema.json`)
//! and is converted to a [`Genome`] by [`build_starter_genome`] once a
//! [`ChannelRegistry`] has been loaded.
//!
//! # Design notes
//!
//! * Specs are `&'static` constants — no allocation, no PRNG, fully
//!   deterministic across runs and processes.
//! * Lineage tags are hand-assigned in the `0xAAAA_xxxx` (grassland),
//!   `0xBBBB_xxxx` (forest), `0xCCCC_xxxx` (tundra) ranges so a tag
//!   identifies its origin species at a glance during debugging. The
//!   prefix registry — required reading before adding a fourth
//!   starter species — lives in issue #161.
//! * Each spec has 10–15 trait genes per `documentation/planning/
//!   IMPLEMENTATION_PLAN.md` Sprint S8 acceptance criteria.
//! * Each spec carries a `home_biome_tag` matching
//!   `BiomeKind::as_str()` (`"plains"` / `"forest"` / `"tundra"`) so
//!   the spawner can pick a placement strategy without importing
//!   `beast-ecs` (which sits one layer above).
//! * Channels named here are aspirational — they describe the
//!   intended core registry. The build step returns
//!   [`StarterError::MissingChannel`] when a referenced channel is
//!   not registered, so the failure mode is loud and the test suite
//!   uses a minimal registry built by `tests::test_registry()`.
//!
//! # Bit literals: source of truth
//!
//! `q(value, bits)` stores raw `i64` bits as the canonical Q3232
//! encoding. **The `value` argument is purely decorative** — it is
//! never read at compile time or at runtime. Reviewers must audit
//! the `bits` field, not the decimal annotation.
//!
//! Two safety nets keep the `bits` honest:
//!
//! 1. `tests::q_constants_match_decimals` recomputes the expected
//!    bit pattern from each named decimal constant (`HALF`, `ONE`,
//!    etc.) using `Q3232::from_num(decimal).to_bits()` and asserts
//!    equality. A slip in any named constant fails this test on
//!    first compile.
//! 2. `tests::q3232_internal_repr_is_i32f32` locks the assumption
//!    that `Q3232::from_bits(ONE).to_num::<f64>() == 1.0`. If the
//!    `fixed` crate ever changes the internal layout of `I32F32`,
//!    this test fails at the type-system boundary and every spec
//!    constant has to be re-encoded.

use beast_channels::{ChannelRegistry, Provenance};
use beast_core::Q3232;
use serde::Serialize;

use crate::body_site::BodyVector;
use crate::error::GenomeError;
use crate::gene::{EffectVector, Target, Timing, TraitGene};
use crate::genome::{Genome, GenomeParams};
use crate::lineage::LineageTag;

/// Errors specific to starter genome construction.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StarterError {
    /// The active channel registry does not contain a channel that
    /// the starter spec references.
    #[error("registry missing channel `{channel}` required by starter species `{species}`")]
    MissingChannel {
        /// The species whose spec referenced the channel.
        species: &'static str,
        /// The channel id that was not found.
        channel: &'static str,
    },
    /// The spec produced an invalid genome (out-of-range magnitudes,
    /// duplicate lineage tags, etc.).
    #[error("starter species `{species}`: {source}")]
    InvalidGenome {
        /// The species whose spec failed validation.
        species: &'static str,
        /// The underlying genome error.
        #[source]
        source: GenomeError,
    },
}

/// Per-gene channel contribution declared in a [`StarterSpec`].
///
/// All `Q3232`-shaped fields are stored as `i64` raw bits (the
/// internal representation of `Q3232`) so the spec can live in
/// `const` context. The build step rehydrates them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StarterGeneSpec {
    /// Channel registry id this gene contributes to (e.g.,
    /// `"metabolic_rate"`).
    pub channel_id: &'static str,
    /// Per-channel contribution magnitude in Q3232 bits.
    pub channel_value_bits: i64,
    /// `EffectVector::magnitude` in Q3232 bits, must encode `[0, 1]`.
    pub effect_magnitude_bits: i64,
    /// `EffectVector::radius` in Q3232 bits, must encode `[0, 1]`.
    pub effect_radius_bits: i64,
    /// When the gene fires.
    pub timing: Timing,
    /// Who the effect is applied to.
    pub target: Target,
    /// `BodyVector::surface_vs_internal` in Q3232 bits, `[0, 1]`.
    pub body_surface_bits: i64,
    /// `BodyVector::body_region` in Q3232 bits, `[0, 1]`.
    pub body_region_bits: i64,
    /// `BodyVector::bilateral_symmetry`.
    pub body_bilateral: bool,
    /// `BodyVector::coverage` in Q3232 bits, `[0, 1]`.
    pub body_coverage_bits: i64,
    /// Stable lineage tag for this gene.
    pub lineage_tag: u64,
}

/// A starter species — a named blueprint that builds a [`Genome`]
/// against the live channel registry.
///
/// Only `Serialize` is derived — `Deserialize` would require
/// owning every `&'static str`, which is out of scope for a static
/// blueprint. Tooling that needs to *read* spec JSON should
/// deserialise into a separate owned-string mirror type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StarterSpec {
    /// Snake-case identifier, also used as the species name.
    pub name: &'static str,
    /// Free-form description of the species' niche, surfaced in
    /// debug/UI tooling.
    pub description: &'static str,
    /// Matches `BiomeKind::as_str()` — the biome the spawner will
    /// prefer when placing this species.
    pub home_biome_tag: &'static str,
    /// 10–15 trait genes; iterated in declaration order so the
    /// produced genome is byte-stable.
    pub genes: &'static [StarterGeneSpec],
}

/// All starter species shipped with the MVP. Iterated in declaration
/// order so any consumer that snapshots the list is deterministic.
pub const STARTER_SPECIES: &[StarterSpec] = &[GRASSLAND_GRAZER, FOREST_OMNIVORE, TUNDRA_ENDOTHERM];

/// Helper: encode a Q3232 from a fractional `f64` literal at compile
/// time via the `from_num` constructor. Wrapped in a `const fn` so
/// each spec entry stays a single line. The `Q3232::from_num` path
/// ultimately materialises an `i64` via `<I32F32 as FromFixed>::from_num`.
const fn q(_value_unused_at_compile_time: f64, bits: i64) -> i64 {
    // The `bits` field is the source of truth — `value_unused` exists
    // only to make the spec lines self-documenting at the source level
    // (e.g., `q(0.45, 0x7333_3333)`). The build step never reads
    // `value_unused`.
    bits
}

// Q3232 raw-bit constants for the magnitudes we use repeatedly. The
// `Q3232` representation is `I32F32`: 32 integer bits + 32 fraction
// bits. So `1.0 = 1 << 32 = 0x1_0000_0000`.
const ZERO: i64 = 0;
const ONE_TENTH: i64 = 0x1999_999A; // 0.1
const ONE_QUARTER: i64 = 0x4000_0000; // 0.25
const HALF: i64 = 0x8000_0000; // 0.5
const THREE_QUARTERS: i64 = 0xC000_0000; // 0.75
const FOUR_FIFTHS: i64 = 0xCCCC_CCCD; // 0.8
const ONE: i64 = 0x1_0000_0000; // 1.0

/// Grassland grazer — herbivore tuned for open plains. High visual
/// acuity for predator detection, fast locomotion for fleeing,
/// efficient herbivory.
pub const GRASSLAND_GRAZER: StarterSpec = StarterSpec {
    name: "grassland_grazer",
    description: "Open-plains herbivore. Fast, alert, efficient grass digestion.",
    home_biome_tag: "plains",
    genes: &[
        // Metabolism: moderate baseline, herbivory bias.
        StarterGeneSpec {
            channel_id: "metabolic_rate",
            channel_value_bits: q(0.55, HALF + ONE_TENTH),
            effect_magnitude_bits: q(0.6, 0x9999_999A),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.3, 0x4CCC_CCCD),
            lineage_tag: 0xAAAA_0001,
        },
        StarterGeneSpec {
            channel_id: "herbivory_efficiency",
            channel_value_bits: q(0.85, 0xD999_999A),
            effect_magnitude_bits: q(0.8, FOUR_FIFTHS),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.4, 0x6666_6666),
            body_bilateral: false,
            body_coverage_bits: q(0.2, 0x3333_3333),
            lineage_tag: 0xAAAA_0002,
        },
        // Locomotion: high speed, light frame.
        StarterGeneSpec {
            channel_id: "locomotion_speed",
            channel_value_bits: q(0.8, FOUR_FIFTHS),
            effect_magnitude_bits: q(0.75, THREE_QUARTERS),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.7, 0xB333_3333),
            body_region_bits: q(0.7, 0xB333_3333),
            body_bilateral: true,
            body_coverage_bits: q(0.4, 0x6666_6666),
            lineage_tag: 0xAAAA_0003,
        },
        StarterGeneSpec {
            channel_id: "kinetic_force",
            channel_value_bits: q(0.45, 0x7333_3333),
            effect_magnitude_bits: q(0.5, HALF),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.6, 0x9999_999A),
            body_region_bits: q(0.7, 0xB333_3333),
            body_bilateral: true,
            body_coverage_bits: q(0.3, 0x4CCC_CCCD),
            lineage_tag: 0xAAAA_0004,
        },
        // Sensory: high visual + auditory acuity (open terrain).
        StarterGeneSpec {
            channel_id: "visual_acuity",
            channel_value_bits: q(0.85, 0xD999_999A),
            effect_magnitude_bits: q(0.85, 0xD999_999A),
            effect_radius_bits: q(0.1, ONE_TENTH),
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.1, ONE_TENTH),
            body_bilateral: true,
            body_coverage_bits: q(0.05, 0x0CCC_CCCD),
            lineage_tag: 0xAAAA_0005,
        },
        StarterGeneSpec {
            channel_id: "auditory_sensitivity",
            channel_value_bits: q(0.7, 0xB333_3333),
            effect_magnitude_bits: q(0.7, 0xB333_3333),
            effect_radius_bits: q(0.15, 0x2666_6666),
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.15, 0x2666_6666),
            body_bilateral: true,
            body_coverage_bits: q(0.05, 0x0CCC_CCCD),
            lineage_tag: 0xAAAA_0006,
        },
        StarterGeneSpec {
            channel_id: "olfactory_sensitivity",
            channel_value_bits: q(0.5, HALF),
            effect_magnitude_bits: q(0.5, HALF),
            effect_radius_bits: q(0.2, 0x3333_3333),
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.05, 0x0CCC_CCCD),
            body_bilateral: false,
            body_coverage_bits: q(0.05, 0x0CCC_CCCD),
            lineage_tag: 0xAAAA_0007,
        },
        // Defense: light skin, modest rigidity.
        StarterGeneSpec {
            channel_id: "structural_rigidity",
            channel_value_bits: q(0.4, 0x6666_6666),
            effect_magnitude_bits: q(0.4, 0x6666_6666),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.5, HALF),
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.6, 0x9999_999A),
            lineage_tag: 0xAAAA_0008,
        },
        StarterGeneSpec {
            channel_id: "skin_thickness",
            channel_value_bits: q(0.35, 0x5999_999A),
            effect_magnitude_bits: q(0.4, 0x6666_6666),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.9, 0xE666_6666),
            lineage_tag: 0xAAAA_0009,
        },
        // Thermoregulation + water economy.
        StarterGeneSpec {
            channel_id: "thermal_regulation",
            channel_value_bits: q(0.5, HALF),
            effect_magnitude_bits: q(0.5, HALF),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.3, 0x4CCC_CCCD),
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.7, 0xB333_3333),
            lineage_tag: 0xAAAA_000A,
        },
        StarterGeneSpec {
            channel_id: "water_efficiency",
            channel_value_bits: q(0.6, 0x9999_999A),
            effect_magnitude_bits: q(0.6, 0x9999_999A),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.2, 0x3333_3333),
            body_region_bits: q(0.4, 0x6666_6666),
            body_bilateral: false,
            body_coverage_bits: q(0.4, 0x6666_6666),
            lineage_tag: 0xAAAA_000B,
        },
        // Reproduction: r-selected; many offspring.
        StarterGeneSpec {
            channel_id: "reproductive_rate",
            channel_value_bits: q(0.7, 0xB333_3333),
            effect_magnitude_bits: q(0.7, 0xB333_3333),
            effect_radius_bits: ZERO,
            timing: Timing::Periodic,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.6, 0x9999_999A),
            body_bilateral: false,
            body_coverage_bits: q(0.1, ONE_TENTH),
            lineage_tag: 0xAAAA_000C,
        },
    ],
};

/// Forest omnivore — mid-size, dexterous, moderate sensory profile.
/// Eats both plant matter and small prey.
pub const FOREST_OMNIVORE: StarterSpec = StarterSpec {
    name: "forest_omnivore",
    description: "Forest understory omnivore. Climbing, dexterous, opportunistic.",
    home_biome_tag: "forest",
    genes: &[
        StarterGeneSpec {
            channel_id: "metabolic_rate",
            channel_value_bits: q(0.6, 0x9999_999A),
            effect_magnitude_bits: q(0.6, 0x9999_999A),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.3, 0x4CCC_CCCD),
            lineage_tag: 0xBBBB_0001,
        },
        StarterGeneSpec {
            channel_id: "omnivory_efficiency",
            channel_value_bits: q(0.75, THREE_QUARTERS),
            effect_magnitude_bits: q(0.75, THREE_QUARTERS),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.4, 0x6666_6666),
            body_bilateral: false,
            body_coverage_bits: q(0.25, ONE_QUARTER),
            lineage_tag: 0xBBBB_0002,
        },
        StarterGeneSpec {
            channel_id: "locomotion_speed",
            channel_value_bits: q(0.55, HALF + ONE_TENTH),
            effect_magnitude_bits: q(0.55, HALF + ONE_TENTH),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.6, 0x9999_999A),
            body_region_bits: q(0.7, 0xB333_3333),
            body_bilateral: true,
            body_coverage_bits: q(0.3, 0x4CCC_CCCD),
            lineage_tag: 0xBBBB_0003,
        },
        StarterGeneSpec {
            channel_id: "arboreal_climbing",
            channel_value_bits: q(0.85, 0xD999_999A),
            effect_magnitude_bits: q(0.85, 0xD999_999A),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.6, 0x9999_999A),
            body_bilateral: true,
            body_coverage_bits: q(0.4, 0x6666_6666),
            lineage_tag: 0xBBBB_0004,
        },
        StarterGeneSpec {
            channel_id: "kinetic_force",
            channel_value_bits: q(0.6, 0x9999_999A),
            effect_magnitude_bits: q(0.6, 0x9999_999A),
            effect_radius_bits: ZERO,
            timing: Timing::OnContact,
            target: Target::TouchedEntity,
            body_surface_bits: q(0.7, 0xB333_3333),
            body_region_bits: q(0.6, 0x9999_999A),
            body_bilateral: true,
            body_coverage_bits: q(0.3, 0x4CCC_CCCD),
            lineage_tag: 0xBBBB_0005,
        },
        StarterGeneSpec {
            channel_id: "visual_acuity",
            channel_value_bits: q(0.6, 0x9999_999A),
            effect_magnitude_bits: q(0.6, 0x9999_999A),
            effect_radius_bits: q(0.1, ONE_TENTH),
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.1, ONE_TENTH),
            body_bilateral: true,
            body_coverage_bits: q(0.05, 0x0CCC_CCCD),
            lineage_tag: 0xBBBB_0006,
        },
        StarterGeneSpec {
            channel_id: "olfactory_sensitivity",
            channel_value_bits: q(0.75, THREE_QUARTERS),
            effect_magnitude_bits: q(0.75, THREE_QUARTERS),
            effect_radius_bits: q(0.2, 0x3333_3333),
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.05, 0x0CCC_CCCD),
            body_bilateral: false,
            body_coverage_bits: q(0.05, 0x0CCC_CCCD),
            lineage_tag: 0xBBBB_0007,
        },
        StarterGeneSpec {
            channel_id: "auditory_sensitivity",
            channel_value_bits: q(0.5, HALF),
            effect_magnitude_bits: q(0.5, HALF),
            effect_radius_bits: q(0.15, 0x2666_6666),
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.15, 0x2666_6666),
            body_bilateral: true,
            body_coverage_bits: q(0.05, 0x0CCC_CCCD),
            lineage_tag: 0xBBBB_0008,
        },
        StarterGeneSpec {
            channel_id: "structural_rigidity",
            channel_value_bits: q(0.5, HALF),
            effect_magnitude_bits: q(0.5, HALF),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.5, HALF),
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.6, 0x9999_999A),
            lineage_tag: 0xBBBB_0009,
        },
        StarterGeneSpec {
            channel_id: "thermal_regulation",
            channel_value_bits: q(0.55, HALF + ONE_TENTH),
            effect_magnitude_bits: q(0.55, HALF + ONE_TENTH),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.3, 0x4CCC_CCCD),
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.7, 0xB333_3333),
            lineage_tag: 0xBBBB_000A,
        },
        StarterGeneSpec {
            channel_id: "reproductive_rate",
            channel_value_bits: q(0.5, HALF),
            effect_magnitude_bits: q(0.5, HALF),
            effect_radius_bits: ZERO,
            timing: Timing::Periodic,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.6, 0x9999_999A),
            body_bilateral: false,
            body_coverage_bits: q(0.1, ONE_TENTH),
            lineage_tag: 0xBBBB_000B,
        },
    ],
};

/// Tundra endotherm — cold-adapted, slow-moving, high body mass.
/// Thick fur, efficient thermoregulation, large stamina reserves.
pub const TUNDRA_ENDOTHERM: StarterSpec = StarterSpec {
    name: "tundra_endotherm",
    description: "Cold-climate endotherm. Thick insulation, low metabolism, hardy.",
    home_biome_tag: "tundra",
    genes: &[
        StarterGeneSpec {
            channel_id: "metabolic_rate",
            channel_value_bits: q(0.4, 0x6666_6666),
            effect_magnitude_bits: q(0.4, 0x6666_6666),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.3, 0x4CCC_CCCD),
            lineage_tag: 0xCCCC_0001,
        },
        StarterGeneSpec {
            channel_id: "carnivory_efficiency",
            channel_value_bits: q(0.65, 0xA666_6666),
            effect_magnitude_bits: q(0.65, 0xA666_6666),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.4, 0x6666_6666),
            body_bilateral: false,
            body_coverage_bits: q(0.25, ONE_QUARTER),
            lineage_tag: 0xCCCC_0002,
        },
        StarterGeneSpec {
            channel_id: "thermal_regulation",
            channel_value_bits: q(0.95, 0xF333_3333),
            effect_magnitude_bits: q(0.9, 0xE666_6666),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.3, 0x4CCC_CCCD),
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.95, 0xF333_3333),
            lineage_tag: 0xCCCC_0003,
        },
        StarterGeneSpec {
            channel_id: "fur_density",
            channel_value_bits: q(0.95, 0xF333_3333),
            effect_magnitude_bits: q(0.9, 0xE666_6666),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.95, 0xF333_3333),
            lineage_tag: 0xCCCC_0004,
        },
        StarterGeneSpec {
            channel_id: "cold_tolerance",
            channel_value_bits: q(0.9, 0xE666_6666),
            effect_magnitude_bits: q(0.9, 0xE666_6666),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.5, HALF),
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.85, 0xD999_999A),
            lineage_tag: 0xCCCC_0005,
        },
        StarterGeneSpec {
            channel_id: "locomotion_speed",
            channel_value_bits: q(0.35, 0x5999_999A),
            effect_magnitude_bits: q(0.35, 0x5999_999A),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.6, 0x9999_999A),
            body_region_bits: q(0.7, 0xB333_3333),
            body_bilateral: true,
            body_coverage_bits: q(0.3, 0x4CCC_CCCD),
            lineage_tag: 0xCCCC_0006,
        },
        StarterGeneSpec {
            channel_id: "kinetic_force",
            channel_value_bits: q(0.8, FOUR_FIFTHS),
            effect_magnitude_bits: q(0.8, FOUR_FIFTHS),
            effect_radius_bits: ZERO,
            timing: Timing::OnContact,
            target: Target::TouchedEntity,
            body_surface_bits: q(0.7, 0xB333_3333),
            body_region_bits: q(0.6, 0x9999_999A),
            body_bilateral: true,
            body_coverage_bits: q(0.3, 0x4CCC_CCCD),
            lineage_tag: 0xCCCC_0007,
        },
        StarterGeneSpec {
            channel_id: "structural_rigidity",
            channel_value_bits: q(0.7, 0xB333_3333),
            effect_magnitude_bits: q(0.7, 0xB333_3333),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: q(0.4, 0x6666_6666),
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.7, 0xB333_3333),
            lineage_tag: 0xCCCC_0008,
        },
        StarterGeneSpec {
            channel_id: "skin_thickness",
            channel_value_bits: q(0.7, 0xB333_3333),
            effect_magnitude_bits: q(0.7, 0xB333_3333),
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.5, HALF),
            body_bilateral: false,
            body_coverage_bits: q(0.95, 0xF333_3333),
            lineage_tag: 0xCCCC_0009,
        },
        StarterGeneSpec {
            channel_id: "olfactory_sensitivity",
            channel_value_bits: q(0.7, 0xB333_3333),
            effect_magnitude_bits: q(0.7, 0xB333_3333),
            effect_radius_bits: q(0.2, 0x3333_3333),
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ONE,
            body_region_bits: q(0.05, 0x0CCC_CCCD),
            body_bilateral: false,
            body_coverage_bits: q(0.05, 0x0CCC_CCCD),
            lineage_tag: 0xCCCC_000A,
        },
        StarterGeneSpec {
            channel_id: "reproductive_rate",
            channel_value_bits: q(0.3, 0x4CCC_CCCD),
            effect_magnitude_bits: q(0.3, 0x4CCC_CCCD),
            effect_radius_bits: ZERO,
            timing: Timing::Periodic,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: q(0.6, 0x9999_999A),
            body_bilateral: false,
            body_coverage_bits: q(0.1, ONE_TENTH),
            lineage_tag: 0xCCCC_000B,
        },
    ],
};

/// Build a [`Genome`] from a [`StarterSpec`] against a live channel
/// registry.
///
/// The returned genome has:
///
/// * One [`TraitGene`] per entry in `spec.genes`, in declaration
///   order.
/// * A channel vector sized to `registry.len()`. Each gene's
///   contribution is placed at the registry position of its
///   `channel_id`; all other positions are `Q3232::ZERO`.
/// * Default [`GenomeParams`] (mutation rates per System 01 §3).
/// * Empty regulatory networks (added by S15+ stories — see issue
///   #145 acceptance criteria).
///
/// # Channel-position binding contract
///
/// **Channel vector indices are positional**: the position of a
/// channel in the produced `EffectVector::channel` matches the
/// channel's iteration position in the registry (BTreeMap-sorted by
/// id). This means a channel's position is stable only as long as
/// no channel sorting alphabetically before it is added or removed.
/// Adding `aquatic_efficiency` to a registry that already had
/// `metabolic_rate` would shift every position from `metabolic_rate`
/// onward by one slot.
///
/// Implications for callers:
///
/// * **Saved genomes carry no registry fingerprint**, so a save
///   created against registry A cannot be safely loaded against
///   registry B without an external check. The fingerprint guard
///   for `SaveFile` is tracked in issue #160 — until that lands,
///   the simulation must rebuild starter genomes (call this
///   function again) rather than reload them whenever the channel
///   registry changes.
/// * Mods that introduce new channels must not insert them into a
///   running world; channels are append-only at world creation
///   only.
///
/// # Errors
///
/// * [`StarterError::MissingChannel`] when a referenced channel id
///   is not in the registry.
/// * [`StarterError::InvalidGenome`] when the produced genome fails
///   [`Genome::validate`] (out-of-range Q3232 values, duplicate
///   lineage tags, etc.).
pub fn build_starter_genome(
    spec: &StarterSpec,
    registry: &ChannelRegistry,
) -> Result<Genome, StarterError> {
    let n_channels = registry.len();
    let mut id_to_pos: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (pos, (id, _)) in registry.iter().enumerate() {
        id_to_pos.insert(id, pos);
    }

    let mut genes = Vec::with_capacity(spec.genes.len());
    for g in spec.genes {
        let pos = id_to_pos
            .get(g.channel_id)
            .copied()
            .ok_or(StarterError::MissingChannel {
                species: spec.name,
                channel: g.channel_id,
            })?;

        let mut channel = vec![Q3232::ZERO; n_channels];
        channel[pos] = Q3232::from_bits(g.channel_value_bits);

        let effect = EffectVector::new(
            channel,
            Q3232::from_bits(g.effect_magnitude_bits),
            Q3232::from_bits(g.effect_radius_bits),
            g.timing,
            g.target,
        )
        .map_err(|source| StarterError::InvalidGenome {
            species: spec.name,
            source,
        })?;

        let body = BodyVector::new(
            Q3232::from_bits(g.body_surface_bits),
            Q3232::from_bits(g.body_region_bits),
            g.body_bilateral,
            Q3232::from_bits(g.body_coverage_bits),
        )
        .map_err(|source| StarterError::InvalidGenome {
            species: spec.name,
            source,
        })?;

        let gene = TraitGene::new(
            g.channel_id,
            effect,
            body,
            Vec::new(),
            true,
            LineageTag::from_raw(g.lineage_tag),
            Provenance::Core,
        )
        .map_err(|source| StarterError::InvalidGenome {
            species: spec.name,
            source,
        })?;

        genes.push(gene);
    }

    Genome::new(GenomeParams::default(), genes).map_err(|source| StarterError::InvalidGenome {
        species: spec.name,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use beast_channels::{
        BoundsPolicy, ChannelFamily, ChannelManifest, MutationKernel, Provenance as ChannelProv,
        Range, ScaleBand,
    };

    /// Build a registry containing every channel referenced by the
    /// shipping starter species. Test-only — production code loads
    /// the registry from manifests on disk.
    ///
    /// **WARNING:** every channel is registered with
    /// `ChannelFamily::Metabolic` for simplicity. Tests that depend
    /// on family-keyed lookups (e.g.,
    /// `registry.ids_by_family(ChannelFamily::Locomotion)`) must
    /// build their own registry — this fixture will return an empty
    /// iterator for any non-metabolic family. Per-family taxonomy is
    /// out of scope until the core channel manifest set lands.
    fn test_registry() -> ChannelRegistry {
        // Collect every distinct channel id used by the starter
        // specs. Sorted by BTreeSet so test failures are stable.
        let mut ids: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
        for spec in STARTER_SPECIES {
            for g in spec.genes {
                ids.insert(g.channel_id);
            }
        }

        let mut registry = ChannelRegistry::new();
        for id in ids {
            let manifest = ChannelManifest {
                id: id.into(),
                family: ChannelFamily::Metabolic,
                description: "test fixture".into(),
                range: Range {
                    min: Q3232::ZERO,
                    max: Q3232::ONE,
                    units: "dimensionless".into(),
                },
                mutation_kernel: MutationKernel {
                    sigma: Q3232::from_num(0.1_f64),
                    bounds_policy: BoundsPolicy::Clamp,
                    genesis_weight: Q3232::ONE,
                    correlation_with: Vec::new(),
                },
                composition_hooks: Vec::new(),
                expression_conditions: Vec::new(),
                scale_band: ScaleBand {
                    min_kg: Q3232::ZERO,
                    max_kg: Q3232::from_num(1000_i32),
                },
                body_site_applicable: false,
                provenance: ChannelProv::Core,
            };
            registry
                .register(manifest)
                .expect("test registry registration");
        }
        registry
    }

    #[test]
    fn ships_three_starter_species() {
        // The MVP plan locks in exactly three starter species
        // (System 01 + planning/IMPLEMENTATION_PLAN.md S8.3).
        // Adding a fourth here without revising the plan is a sign
        // the spec drifted; this assertion catches that.
        assert_eq!(STARTER_SPECIES.len(), 3);
    }

    #[test]
    fn each_starter_has_ten_to_fifteen_genes() {
        for spec in STARTER_SPECIES {
            let n = spec.genes.len();
            assert!(
                (10..=15).contains(&n),
                "{}: {} genes (expected 10..=15 per S8.3 acceptance criteria)",
                spec.name,
                n,
            );
        }
    }

    #[test]
    fn lineage_tags_are_globally_unique() {
        // A duplicate tag would silently break phylogeny tracking.
        let mut seen: std::collections::BTreeMap<u64, &'static str> =
            std::collections::BTreeMap::new();
        for spec in STARTER_SPECIES {
            for g in spec.genes {
                if let Some(prior) = seen.insert(g.lineage_tag, spec.name) {
                    panic!(
                        "duplicate lineage tag 0x{:016x} on {} and {}",
                        g.lineage_tag, prior, spec.name
                    );
                }
            }
        }
    }

    #[test]
    fn home_biome_tags_are_known_biomekind_strings() {
        // Mirrors `BiomeKind::as_str()` in `beast-ecs`. We can't
        // import `BiomeKind` here (layer DAG) but we *can* lock in
        // that the spawner has six valid choices.
        //
        // **Rename hazard:** if a `BiomeKind` variant is renamed
        // (e.g., `Plains` → `Grassland`, changing as_str output to
        // `"grassland"`), this test still passes as long as
        // `KNOWN` is also updated. To prevent silent drift:
        //
        //   1. Update `BiomeKind::as_str()` in beast-ecs.
        //   2. Update the affected spec's `home_biome_tag` in this
        //      file.
        //   3. Update the `KNOWN` list below.
        //
        // The unification refactor that collapses `BiomeTag` and
        // `BiomeKind` (tracked in the S8 epic) will replace this
        // hand-maintained mirror with a real type lock-in.
        const KNOWN: &[&str] = &["ocean", "forest", "plains", "desert", "mountain", "tundra"];
        for spec in STARTER_SPECIES {
            assert!(
                KNOWN.contains(&spec.home_biome_tag),
                "{}: unknown home_biome_tag `{}` (must match BiomeKind::as_str)",
                spec.name,
                spec.home_biome_tag,
            );
        }
    }

    #[test]
    fn build_succeeds_for_every_starter_against_full_registry() {
        let registry = test_registry();
        for spec in STARTER_SPECIES {
            let genome = build_starter_genome(spec, &registry).unwrap_or_else(|e| {
                panic!("build {} failed: {e}", spec.name);
            });
            assert_eq!(
                genome.len(),
                spec.genes.len(),
                "{}: genome length should match spec gene count",
                spec.name,
            );
            // Validate again from scratch — Genome::new already
            // validates, but this guards against future Genome
            // refactors that move validation around.
            genome.validate().expect("starter genome validates");
        }
    }

    #[test]
    fn build_returns_missing_channel_when_registry_is_empty() {
        let empty = ChannelRegistry::new();
        let err = build_starter_genome(&GRASSLAND_GRAZER, &empty).unwrap_err();
        match err {
            StarterError::MissingChannel { species, channel } => {
                assert_eq!(species, "grassland_grazer");
                // First gene in the spec is metabolic_rate; that's
                // the channel the build step will hit first.
                assert_eq!(channel, "metabolic_rate");
            }
            other => panic!("expected MissingChannel, got {other:?}"),
        }
    }

    #[test]
    fn build_is_deterministic_across_calls() {
        // Two builds of the same spec against the same registry
        // must produce byte-identical genomes — this is a
        // precondition for the determinism gate.
        let registry = test_registry();
        let a = build_starter_genome(&GRASSLAND_GRAZER, &registry).unwrap();
        let b = build_starter_genome(&GRASSLAND_GRAZER, &registry).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn build_places_value_at_correct_channel_position() {
        let registry = test_registry();
        let genome = build_starter_genome(&GRASSLAND_GRAZER, &registry).unwrap();
        // Look up the channel position for metabolic_rate (the
        // first gene in the spec).
        let metabolic_pos = registry
            .iter()
            .position(|(id, _)| id == "metabolic_rate")
            .expect("metabolic_rate in test registry");
        let first_gene = &genome.genes[0];
        assert_eq!(first_gene.channel_id, "metabolic_rate");
        // Non-zero at the metabolic_rate position.
        assert_ne!(
            first_gene.effect.channel.as_slice()[metabolic_pos],
            Q3232::ZERO
        );
        // Zero at every other position.
        for (i, value) in first_gene.effect.channel.as_slice().iter().enumerate() {
            if i != metabolic_pos {
                assert_eq!(*value, Q3232::ZERO, "non-zero leak at position {i}");
            }
        }
    }

    #[test]
    fn species_names_are_distinct() {
        let mut seen: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
        for spec in STARTER_SPECIES {
            assert!(
                seen.insert(spec.name),
                "duplicate species name: {}",
                spec.name
            );
        }
    }

    #[test]
    fn q3232_internal_repr_is_i32f32() {
        // Lock-in: the entire spec encoding assumes
        // `Q3232::from_bits(x).to_num::<f64>() == x as f64 / 2^32`.
        // This is true today because `Q3232 = I32F32` (32 integer
        // bits + 32 fractional bits, stored as a plain `i64`). If
        // the `fixed` crate ever changes this layout — or if the
        // project migrates to a different fixed-point library —
        // this test fails immediately and every spec constant has
        // to be re-encoded. Without this guard, the migration
        // would silently produce wrong values throughout
        // STARTER_SPECIES.
        assert_eq!(Q3232::from_bits(ONE).to_num::<f64>(), 1.0_f64);
        assert_eq!(Q3232::from_bits(HALF).to_num::<f64>(), 0.5_f64);
        assert_eq!(Q3232::from_bits(ZERO).to_num::<f64>(), 0.0_f64);
    }

    #[test]
    fn q_constants_match_decimals() {
        // Per the module-doc warning, the `q(value, bits)` helper
        // discards `value` at compile time — bits are the source of
        // truth. This test recomputes the expected bits for each
        // named constant from the corresponding decimal using
        // `Q3232::from_num(decimal).to_bits()` and asserts equality
        // so a stale named constant fails the test on first
        // compile. Drift in *inline* hex values within spec
        // entries is still possible by hand; the named-constant
        // checks here cover the most-reused values.
        assert_eq!(ZERO, Q3232::from_num(0_i32).to_bits());
        assert_eq!(ONE_TENTH, Q3232::from_num(0.1_f64).to_bits());
        assert_eq!(ONE_QUARTER, Q3232::from_num(0.25_f64).to_bits());
        assert_eq!(HALF, Q3232::from_num(0.5_f64).to_bits());
        assert_eq!(THREE_QUARTERS, Q3232::from_num(0.75_f64).to_bits());
        assert_eq!(FOUR_FIFTHS, Q3232::from_num(0.8_f64).to_bits());
        assert_eq!(ONE, Q3232::from_num(1_i32).to_bits());
    }

    #[test]
    fn build_returns_invalid_genome_when_spec_has_out_of_range_value() {
        // Negative test for the `StarterError::InvalidGenome`
        // branch — without this, the three `.map_err` chains in
        // `build_starter_genome` are uncovered and an upstream
        // refactor could silently start swallowing errors.
        // Synthesise a one-gene spec with effect_magnitude_bits
        // representing 1.5 (out of [0, 1]) so `EffectVector::new`
        // rejects it. `static` items are required because
        // `StarterSpec.genes` is `&'static [...]`.
        static BAD_GENES: [StarterGeneSpec; 1] = [StarterGeneSpec {
            channel_id: "metabolic_rate",
            channel_value_bits: HALF,
            // 1.5 in Q3232 = 1.5 * 2^32 = 0x1_8000_0000.
            effect_magnitude_bits: 0x1_8000_0000,
            effect_radius_bits: ZERO,
            timing: Timing::Passive,
            target: Target::SelfEntity,
            body_surface_bits: ZERO,
            body_region_bits: ZERO,
            body_bilateral: false,
            body_coverage_bits: ZERO,
            lineage_tag: 0xDEAD_0001,
        }];
        static BAD_SPEC: StarterSpec = StarterSpec {
            name: "synthesised_bad_spec",
            description: "test fixture for InvalidGenome path",
            home_biome_tag: "plains",
            genes: &BAD_GENES,
        };
        // Build a single-channel registry containing the channel
        // the spec references, so we get past MissingChannel and
        // hit InvalidGenome.
        let mut registry = ChannelRegistry::new();
        registry
            .register(ChannelManifest {
                id: "metabolic_rate".into(),
                family: ChannelFamily::Metabolic,
                description: "test fixture".into(),
                range: Range {
                    min: Q3232::ZERO,
                    max: Q3232::ONE,
                    units: "dimensionless".into(),
                },
                mutation_kernel: MutationKernel {
                    sigma: Q3232::from_num(0.1_f64),
                    bounds_policy: BoundsPolicy::Clamp,
                    genesis_weight: Q3232::ONE,
                    correlation_with: Vec::new(),
                },
                composition_hooks: Vec::new(),
                expression_conditions: Vec::new(),
                scale_band: ScaleBand {
                    min_kg: Q3232::ZERO,
                    max_kg: Q3232::from_num(1000_i32),
                },
                body_site_applicable: false,
                provenance: ChannelProv::Core,
            })
            .expect("test registry registration");

        let err = build_starter_genome(&BAD_SPEC, &registry).unwrap_err();
        match err {
            StarterError::InvalidGenome { species, source: _ } => {
                assert_eq!(species, "synthesised_bad_spec");
            }
            other => panic!("expected InvalidGenome, got {other:?}"),
        }
    }

    #[test]
    fn starter_spec_round_trips_through_serialize() {
        // Smoke-test the new `Serialize` derive so a future
        // refactor that breaks JSON output is caught here. We do
        // not test Deserialize because the spec carries `&'static
        // str`, which is intentionally non-roundtrippable.
        let s = serde_json::to_string(&GRASSLAND_GRAZER).expect("serialize");
        assert!(s.contains("grassland_grazer"));
        assert!(s.contains("metabolic_rate"));
    }
}
