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
    ///
    /// Panics if `width * height` overflows `usize` — that's a
    /// programming bug, not a valid input. The check guards against
    /// silent truncation on 32-bit targets where two values above
    /// `u16::MAX` would otherwise wrap.
    pub fn solid(width: u32, height: u32, color: [u8; 4]) -> Self {
        let len = (width as usize)
            .checked_mul(height as usize)
            .expect("BiomeView tile count overflows usize");
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
///
/// Render-only DTO: must NOT be serialized into save files. Per
/// `documentation/INVARIANTS.md` §1, all sim-state math runs in
/// Q32.32 fixed-point — `hp_pct: f32` lives here strictly because
/// the encounter renderer needs a render-side hp fraction. If you
/// find yourself wanting to persist this, convert it to `Q3232`
/// (or to whatever sim-side type drives it) before serialising.
/// `Serialize` / `Deserialize` are intentionally NOT derived to
/// keep the type out of bincode save paths.
#[derive(Clone, Debug, PartialEq)]
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

/// One slot in a [`FormationView`] — render-only mirror of
/// `beast_ecs::components::FormationSlot`.
///
/// Render-only DTO: must NOT be serialized into save files. The sim
/// formation slot lives on `Formation`; this view is what the
/// encounter screen paints. `f32` here is the *display* HP / stamina
/// fraction — the underlying sim values are Q32.32 (INVARIANTS §1).
/// `Serialize` / `Deserialize` are intentionally NOT derived.
#[derive(Clone, Debug, PartialEq)]
pub struct FormationSlotView {
    /// Encounter-local id of the occupant, or `None` if the slot is
    /// empty. Mirrors `FormationSlot::occupant` on the sim side.
    pub occupant: Option<u32>,
    /// Display name of the occupant — already resolved to a string
    /// (e.g. `"alpha"`). Empty when `occupant` is `None`.
    pub occupant_name: String,
    /// Slot label (e.g. `"vanguard"`, `"flank-left"`). Stable
    /// structural identifier from `beast_ecs::components::SLOT_NAMES`;
    /// the UI is free to localise it, but the encounter screen uses
    /// the canonical form.
    pub slot_label: String,
    /// Hit-point fraction in `[0, 1]`. Out-of-range values are
    /// clamped at the rendering layer.
    pub hp_pct: f32,
    /// Stamina fraction in `[0, 1]`.
    pub stamina_pct: f32,
    /// Engagement (`Q3232` upstream, normalised to `[0, 1]` here).
    pub engagement_pct: f32,
    /// Exposure, normalised likewise.
    pub exposure_pct: f32,
}

impl FormationSlotView {
    /// Empty slot at `slot_label` — no occupant, all bars at zero.
    pub fn empty(slot_label: impl Into<String>) -> Self {
        Self {
            occupant: None,
            occupant_name: String::new(),
            slot_label: slot_label.into(),
            hp_pct: 0.0,
            stamina_pct: 0.0,
            engagement_pct: 0.0,
            exposure_pct: 0.0,
        }
    }
}

/// Read-only formation view — five slots, one per
/// `beast_ecs::components::SLOT_COUNT` position. The encounter screen
/// reads this; the encounter loop (S13) constructs it from the sim
/// `Formation` component before painting a frame.
///
/// Render-only DTO. See [`FormationSlotView`] for the per-slot fields.
#[derive(Clone, Debug, PartialEq)]
pub struct FormationView {
    /// Slots in canonical index order: 0 = vanguard, 1 = flank-left,
    /// 2 = flank-right, 3 = center, 4 = rear.
    pub slots: Vec<FormationSlotView>,
}

impl FormationView {
    /// Construct an empty formation — `SLOT_COUNT` empty slots
    /// labelled with the canonical names from
    /// [`beast_ecs::components::SLOT_NAMES`]. Sourcing the names
    /// from the sim layer keeps the UI's view in lockstep with the
    /// canonical slot vocabulary; if `SLOT_COUNT` ever grows, both
    /// the sim-side array and this constructor advance together.
    pub fn empty() -> Self {
        use beast_ecs::components::SLOT_NAMES;
        Self {
            slots: SLOT_NAMES
                .iter()
                .map(|name| FormationSlotView::empty(*name))
                .collect(),
        }
    }
}

/// Read-only Keeper view — what the encounter screen reads when
/// painting the leadership-budget bar.
///
/// `f32` percentages here mirror `beast_ecs::components::KeeperState`
/// fields (`Q3232` on the sim side) normalised for display. The
/// encounter loop (S13) computes `leadership_pct` from
/// `leadership_presence(&KeeperState)` divided by the per-Keeper
/// maximum; this DTO carries the already-normalised value.
#[derive(Clone, Debug, PartialEq)]
pub struct KeeperView {
    /// Display name of the Keeper. Empty for a not-yet-named Keeper.
    pub name: String,
    /// Leadership presence as a fraction of the per-Keeper max,
    /// `[0, 1]`. The encounter screen renders this as a horizontal
    /// bar in the screen frame.
    pub leadership_pct: f32,
    /// Stress fraction `[0, 1]` — surfaced as a small inline indicator
    /// alongside the leadership bar.
    pub stress_pct: f32,
}

impl KeeperView {
    /// Empty placeholder: no name, both bars at zero.
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            leadership_pct: 0.0,
            stress_pct: 0.0,
        }
    }
}

/// Read-only snapshot of an active encounter — what the encounter
/// renderer + creature list / action bar need to paint a frame.
///
/// Render-only DTO: must NOT be serialized into save files (see
/// [`EncounterCreatureSnapshot`] doc comment + INVARIANTS §1).
#[derive(Clone, Debug, PartialEq)]
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
    /// UI-derived view of the sim `Formation` — one entry per slot.
    /// The encounter screen renders this as a row of slot cards
    /// (engagement / exposure / hp / stamina + ability labels from
    /// the chronicler).
    pub formation: FormationView,
    /// UI-derived view of the Keeper's state. The screen renders
    /// `leadership_pct` as a horizontal bar in the frame.
    pub keeper: KeeperView,
}

impl EncounterSnapshot {
    /// Construct an empty snapshot — no creatures, no selection,
    /// empty formation, empty Keeper. Stays panic-safe per the S11.6
    /// DoD: the screen builder must not panic on an unbuilt
    /// encounter.
    pub fn empty(biome_label: impl Into<String>) -> Self {
        Self {
            biome_label: biome_label.into(),
            creatures: Vec::new(),
            selected: None,
            formation: FormationView::empty(),
            keeper: KeeperView::empty(),
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
            formation: FormationView::empty(),
            keeper: KeeperView::empty(),
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
            formation: FormationView::empty(),
            keeper: KeeperView::empty(),
        };
        assert_eq!(snap.selected_creature().map(|c| c.id), Some(2));
    }

    #[test]
    fn empty_snapshot_has_five_canonical_slots() {
        // S11.6 DoD: empty constructor stays panic-safe and seeds the
        // five canonical slots from `beast_ecs::components::SLOT_NAMES`.
        // Pin every slot label so a typo in any middle entry surfaces
        // here rather than slipping through.
        let snap = EncounterSnapshot::empty("forest");
        assert_eq!(snap.formation.slots.len(), 5);
        assert_eq!(snap.formation.slots[0].slot_label, "vanguard");
        assert_eq!(snap.formation.slots[1].slot_label, "flank-left");
        assert_eq!(snap.formation.slots[2].slot_label, "flank-right");
        assert_eq!(snap.formation.slots[3].slot_label, "center");
        assert_eq!(snap.formation.slots[4].slot_label, "rear");
        assert_eq!(snap.keeper, KeeperView::empty());
    }
}
