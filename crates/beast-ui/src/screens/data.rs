//! Read-only inputs for the [`crate::screens`] builders.
//!
//! Per `documentation/INVARIANTS.md` §6 the UI layer is read-only against
//! sim state. Screen builders therefore consume:
//!
//! * [`WorldStatus`] — a minimal trait surface the world-map status bar
//!   and creature count rely on. Implemented at the application layer
//!   (S13) by adapting whatever sim handle is on hand; tests implement
//!   it on a tiny struct so the bestiary / world-map layout snapshots
//!   don't need a full simulation.
//! * [`BiomeView`] — a row-major snapshot of biome tile colours the
//!   world-map [`crate::RenderViewport`] paints over. Holding the
//!   colours as `[u8; 4]` keeps `beast-ui` from depending on the
//!   `BiomeTag` enum that lives in `beast-world` — the caller maps
//!   biome ids to colours via `beast_render::biome_tint` (or any
//!   custom palette) before constructing the snapshot.
//! * [`EncounterSnapshot`] — list of creatures in the current
//!   encounter, the active biome label, and the index of the active
//!   creature. Mirrors the data the encounter-view renderer (S9.4)
//!   needs without re-importing it.
//!
//! All four types are POD: no Drop side effects, no inner mutation,
//! and they are constructable in tests without touching the renderer
//! or chronicler.

use serde::{Deserialize, Serialize};

/// Read-only world status surface used by the world-map status bar.
///
/// Trait-shaped (rather than a concrete struct) so the application
/// layer can implement it directly on its sim handle — wrapping a
/// `World` reference and forwarding `current_tick()` /
/// `creature_count()` — without an intermediate snapshot allocation.
/// `&self` receivers pin the read-only contract.
pub trait WorldStatus {
    /// Current simulation tick. Status-bar bindings format this as
    /// `tick: NNNN`; the screen never reads anything else from the
    /// status surface, so a minimal `u64` keeps the trait stable.
    fn current_tick(&self) -> u64;

    /// Number of creatures currently alive in the simulation. The
    /// world-map status bar surfaces this as `creatures: NN`.
    fn creature_count(&self) -> usize;
}

/// Snapshot of biome tile colours rendered behind the world-map view.
///
/// `tile_colors` is row-major: index `y * width + x` holds the
/// `[r, g, b, a]` for tile `(x, y)`. The tile palette is the caller's
/// responsibility — for typical use, populate from
/// `beast_render::biome_tint` after fetching the biome enum from the
/// `Archipelago`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiomeView {
    /// Width of the tile grid, in tiles.
    pub width: u32,
    /// Height of the tile grid, in tiles.
    pub height: u32,
    /// Row-major tile colours. `tile_colors.len() == width * height`.
    pub tile_colors: Vec<[u8; 4]>,
}

impl BiomeView {
    /// Construct an empty (`0×0`) biome view. Useful as a placeholder
    /// before the world has finished generating.
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            tile_colors: Vec::new(),
        }
    }

    /// Construct a uniform-colour biome view of the given dimensions.
    /// Snapshot tests use this so the recorded paint commands don't
    /// drift when `beast_render::biome_tint` is tweaked.
    pub fn solid(width: u32, height: u32, color: [u8; 4]) -> Self {
        let len = width as usize * height as usize;
        Self {
            width,
            height,
            tile_colors: vec![color; len],
        }
    }

    /// True if the snapshot has no tiles. The world-map status bar
    /// surfaces "loading…" rather than "0×0" when this is true.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// One creature in an [`EncounterSnapshot`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterCreatureSnapshot {
    /// Stable entity id.
    pub id: u32,
    /// Display name shown in the encounter creature list.
    pub name: String,
    /// Hit-point fraction in `[0.0, 1.0]`. Out-of-range values are
    /// clamped at the rendering layer; callers should still produce
    /// a normalised value.
    pub hp_pct: f32,
}

/// Read-only snapshot of an active encounter — what the encounter
/// renderer + creature list / action bar need to paint a frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterSnapshot {
    /// Display label for the host biome (e.g. `"forest"`). Surfaced
    /// in the encounter status bar.
    pub biome_label: String,
    /// Creatures in the encounter, in display order.
    pub creatures: Vec<EncounterCreatureSnapshot>,
    /// Index into [`Self::creatures`] of the currently selected /
    /// active creature, if any. Must be `< creatures.len()` when
    /// `Some`; out-of-range values are treated as `None` by the
    /// screen builder.
    pub selected: Option<usize>,
}

impl EncounterSnapshot {
    /// Construct an empty snapshot — no creatures, no selection.
    pub fn empty(biome_label: impl Into<String>) -> Self {
        Self {
            biome_label: biome_label.into(),
            creatures: Vec::new(),
            selected: None,
        }
    }

    /// Currently selected creature, if any. Returns `None` when the
    /// stored index is out of range so screen builders don't have to
    /// repeat the bounds check.
    pub fn selected_creature(&self) -> Option<&EncounterCreatureSnapshot> {
        self.selected.and_then(|i| self.creatures.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedWorld {
        tick: u64,
        creatures: usize,
    }

    impl WorldStatus for FixedWorld {
        fn current_tick(&self) -> u64 {
            self.tick
        }
        fn creature_count(&self) -> usize {
            self.creatures
        }
    }

    #[test]
    fn world_status_trait_is_object_safe() {
        let world = FixedWorld {
            tick: 42,
            creatures: 7,
        };
        let dyn_world: &dyn WorldStatus = &world;
        assert_eq!(dyn_world.current_tick(), 42);
        assert_eq!(dyn_world.creature_count(), 7);
    }

    #[test]
    fn biome_view_solid_fills_tile_grid() {
        let view = BiomeView::solid(4, 3, [10, 20, 30, 40]);
        assert_eq!(view.tile_colors.len(), 12);
        assert!(view.tile_colors.iter().all(|c| *c == [10, 20, 30, 40]));
        assert!(!view.is_empty());
    }

    #[test]
    fn biome_view_empty_is_empty() {
        let view = BiomeView::empty();
        assert!(view.is_empty());
        assert!(view.tile_colors.is_empty());
    }

    #[test]
    fn encounter_selected_clamps_to_range() {
        let snap = EncounterSnapshot {
            biome_label: "forest".into(),
            creatures: vec![EncounterCreatureSnapshot {
                id: 1,
                name: "alpha".into(),
                hp_pct: 1.0,
            }],
            selected: Some(99),
        };
        // Out-of-range index returns None rather than panicking.
        assert!(snap.selected_creature().is_none());
    }

    #[test]
    fn encounter_selected_returns_pointer_to_creature() {
        let snap = EncounterSnapshot {
            biome_label: "forest".into(),
            creatures: vec![
                EncounterCreatureSnapshot {
                    id: 1,
                    name: "alpha".into(),
                    hp_pct: 1.0,
                },
                EncounterCreatureSnapshot {
                    id: 2,
                    name: "beta".into(),
                    hp_pct: 0.5,
                },
            ],
            selected: Some(1),
        };
        assert_eq!(snap.selected_creature().map(|c| c.id), Some(2));
    }
}
