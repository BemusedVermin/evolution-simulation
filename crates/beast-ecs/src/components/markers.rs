//! Marker (tag) components with [`specs::NullStorage`].
//!
//! Zero-size per entity: presence vs. absence is the data. Use for
//! "what kind of entity am I" questions (Is this a creature? A biome
//! cell?). Storage cost stays constant regardless of how many entities
//! carry the tag.

use serde::{Deserialize, Serialize};
use specs::{Component, NullStorage};

/// The entity is a creature (macro-scale beast).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Creature;

/// The entity is a pathogen (micro-scale organism — disease, parasite,
/// symbiont). Treated identically to a creature under the scale-band
/// unification invariant (INVARIANTS §5); the marker lets systems pick
/// the appropriate spatial strategy without dropping into a generic
/// `Mass < 1g` check.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pathogen;

/// The entity is an agent — settlement NPC, caravan leader, faction
/// diplomat, etc. Distinct from `Creature` so the interaction/combat
/// pipelines can skip creature-specific primitive effects.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent;

/// The entity is a faction — a social grouping of agents, settlements,
/// or territories. Zero-state marker; the actual roster lives on other
/// components (to be added as the faction system matures).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faction;

/// The entity is a settlement — a persistent location inhabited by
/// agents.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settlement;

/// The entity is a biome cell — a discrete patch of the world map with
/// its own climate, carrying capacity, and hazard profile.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Biome;

impl Component for Creature {
    type Storage = NullStorage<Self>;
}
impl Component for Pathogen {
    type Storage = NullStorage<Self>;
}
impl Component for Agent {
    type Storage = NullStorage<Self>;
}
impl Component for Faction {
    type Storage = NullStorage<Self>;
}
impl Component for Settlement {
    type Storage = NullStorage<Self>;
}
impl Component for Biome {
    type Storage = NullStorage<Self>;
}
