//! Interpreter input: [`ResolvedPhenotype`] and its supporting types.
//!
//! The `ResolvedPhenotype` is the materialised view of a creature's genotype
//! that the interpreter consumes. It carries the per-channel global values,
//! per-body-region amplitudes, and the environmental context that gates
//! expression.
//!
//! Shape maps 1:1 to §3.1 of the design doc
//! (`documentation/systems/11_phenotype_interpreter.md`). Fields are kept
//! narrow on purpose — upstream systems are responsible for populating only
//! what the interpreter actually reads.

use std::collections::BTreeMap;

use beast_core::{TickCounter, Q3232};

/// Canonical body-site enum. Ordinal order is the deterministic iteration
/// order used by per-site emission in [`crate::body_map`]. Do **not** reorder
/// variants without updating fixture tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodySite {
    /// The creature as a whole (global emissions).
    Global,
    /// Head.
    Head,
    /// Jaw / mouth.
    Jaw,
    /// Body core / torso.
    Core,
    /// Left limb.
    LimbLeft,
    /// Right limb.
    LimbRight,
    /// Tail.
    Tail,
    /// Generic appendage (antenna, tentacle, etc.).
    Appendage,
}

/// Life stage used as an expression-condition gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LifeStage {
    /// Juvenile.
    Juvenile,
    /// Adult.
    Adult,
    /// Elderly.
    Elderly,
}

impl LifeStage {
    /// Stable lowercase label used when matching schema-loaded strings.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Juvenile => "juvenile",
            Self::Adult => "adult",
            Self::Elderly => "elderly",
        }
    }
}

/// One body region with its per-region channel amplitudes.
///
/// `channel_amplitudes` is keyed by channel id. `BTreeMap` — not `HashMap` —
/// so iteration order is stable without relying on hash randomisation.
#[derive(Debug, Clone)]
pub struct BodyRegion {
    /// Stable region id (unique within the phenotype). Assigned by the
    /// upstream body-plan builder.
    pub id: u32,
    /// Anatomical site this region represents.
    pub body_site: BodySite,
    /// Surface-vs-internal coordinate in `[0, 1]` (0 = deep, 1 = surface).
    pub surface_vs_internal: Q3232,
    /// Per-channel amplitudes (post-resolution). Keyed by channel id.
    pub channel_amplitudes: BTreeMap<String, Q3232>,
}

/// Environmental context that gates channel / hook expression.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    /// Biome flags active at the creature's location.
    pub biome_flags: Vec<String>,
    /// Current season label (e.g. `"spring"`).
    pub season: Option<String>,
    /// Local light level `[0, 1]` — 0 = dark, 1 = full sunlight.
    pub light_level: Option<Q3232>,
    /// Local temperature in °C.
    pub temperature_c: Option<Q3232>,
    /// Local population density (individuals per km²).
    pub population_density_per_km2: Option<Q3232>,
}

/// Materialised phenotype input passed to [`crate::interpreter::interpret_phenotype`].
#[derive(Debug, Clone)]
pub struct ResolvedPhenotype {
    /// Per-channel global values, keyed by channel id. `BTreeMap` guarantees
    /// deterministic iteration order.
    pub global_channels: BTreeMap<String, Q3232>,
    /// Body-region breakdown. Populated when any channel has
    /// `body_site_applicable = true`.
    pub body_map: Vec<BodyRegion>,
    /// Creature body mass in kg (used by [`crate::scale_band`]).
    pub body_mass_kg: Q3232,
    /// Life stage.
    pub life_stage: LifeStage,
    /// Tick at which this phenotype was first expressed (provenance metadata).
    pub expression_tick: TickCounter,
    /// Environmental context.
    pub environment: Environment,
}

impl ResolvedPhenotype {
    /// Build an empty phenotype at the given mass and life stage. Used
    /// primarily by tests; production callers materialise from a
    /// [`beast_genome::Genome`] + environment snapshot.
    pub fn new(body_mass_kg: Q3232, life_stage: LifeStage) -> Self {
        Self {
            global_channels: BTreeMap::new(),
            body_map: Vec::new(),
            body_mass_kg,
            life_stage,
            expression_tick: TickCounter::default(),
            environment: Environment::default(),
        }
    }
}
