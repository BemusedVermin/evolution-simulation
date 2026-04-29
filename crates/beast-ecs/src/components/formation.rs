//! Formation component — the 5-slot continuous spatial substrate every
//! later combat / AI / UI story reads.
//!
//! Backs `documentation/systems/06_combat_system.md` §3 ("Formation
//! Model: Continuous Slot Properties"). Slots are *not* discrete grid
//! cells — each slot has continuous `engagement` and `exposure` Q32.32
//! values modulated by adjacent slot occupancy and terrain.
//!
//! This module owns the *data layout only*. The pure math that derives
//! `engagement` / `exposure` from occupancy + terrain lives in
//! `beast_sim::formation` (per `CRATE_LAYOUT.md`: domain logic belongs
//! in L4, not L3). beast-ecs is the data-foundation crate; system code
//! in L4 reads/writes these components.
//!
//! # Design choice — component, not resource
//!
//! The story (#252) lets us pick "resource" or "component on a
//! Keeper-led `Encounter` entity"; we picked the latter so:
//!
//! * Multiple simultaneous encounters (planned for later sprints) get a
//!   `Formation` per encounter without contorting `Resources` into a
//!   collection.
//! * Save/load drops out for free — the `Formation` rides whichever
//!   entity hosts the encounter, and `specs`'s component-storage save
//!   path picks it up alongside everything else.
//!
//! No new marker is added today; the encounter loop (S13) will choose
//! which entity hosts the formation.
//!
//! # Determinism
//!
//! * All math is Q32.32 (`beast_core::Q3232`); no `f32`/`f64` ever
//!   appears in sim state per INVARIANTS §1.
//! * Slots are a fixed-size `[FormationSlot; SLOT_COUNT]`; iteration is
//!   array index order, never insertion order.
//! * Adjacency tables (in `beast_sim::formation`) iterate sorted by
//!   slot index via `BTreeMap`/`BTreeSet`.

use beast_core::Q3232;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};

/// Number of slots in the canonical formation. Locked at five.
pub const SLOT_COUNT: usize = 5;

/// Canonical slot identifiers in index order.
///
/// These are *structural* names taken verbatim from combat doc §3 — used
/// only for diagnostic messages, panic strings, and assertion failures.
/// They are **not** a UI labelling vocabulary; the slot index (`u8` in
/// `0..SLOT_COUNT`) is the canonical id on the sim path. UI / chronicler
/// layers may localise these strings to player-facing text if and when
/// they need to.
///
/// * `0` — Vanguard (front line, melee contact)
/// * `1` — Flank-Left  (forward-side, partial melee exposure)
/// * `2` — Flank-Right (forward-side, partial melee exposure)
/// * `3` — Center     (interior, support / AoE focus)
/// * `4` — Rear       (back line, ranged / support)
pub const SLOT_NAMES: [&str; SLOT_COUNT] =
    ["vanguard", "flank-left", "flank-right", "center", "rear"];

/// One slot in the formation.
///
/// `occupant` is an opaque encounter-local id (the encounter assembler —
/// S13 — populates it from `specs::Entity::id()` when the encounter is
/// built). Storing a bare `u32` keeps `FormationSlot` `Serialize` /
/// `Deserialize` without bringing `specs::Entity` into the save path.
///
/// `engagement` / `exposure` are *derived* values: `Default` returns
/// `Q3232::ZERO` for both, awaiting a call to
/// `beast_sim::formation::recompute` before the slot is read.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormationSlot {
    /// Encounter-local id of whoever holds this slot, or `None` if the
    /// slot is empty.
    pub occupant: Option<u32>,
    /// Proximity to melee threats. `[0, 1]`. Derived field — set by
    /// `beast_sim::formation::recompute`.
    pub engagement: Q3232,
    /// Targetability by enemies. `[0, 1]`. Derived field — set by
    /// `beast_sim::formation::recompute`.
    pub exposure: Q3232,
    /// Terrain bias added to exposure. Encounter assembler sets this
    /// when the formation drops into a particular tile (e.g. a forest
    /// tile lowers exposure across all slots; an open-plain tile
    /// raises it).
    ///
    /// Expected range: `[-1, 1]`. Out-of-range inputs are admitted (the
    /// final `.clamp(0, 1)` on `exposure` keeps the *output* in the
    /// unit interval) but they push the intermediate sum towards
    /// `Q3232::MAX` / `Q3232::MIN` saturation, which is wasteful work.
    /// The S13 encounter assembler should validate this range on input.
    pub terrain_modifier: Q3232,
}

impl FormationSlot {
    /// Convenience constructor: empty slot with the given terrain
    /// modifier and zero derived values. Engagement / exposure are
    /// expected to be filled in by `beast_sim::formation::recompute`
    /// before the slot is read.
    #[must_use]
    pub fn empty(terrain_modifier: Q3232) -> Self {
        Self {
            occupant: None,
            engagement: Q3232::ZERO,
            exposure: Q3232::ZERO,
            terrain_modifier,
        }
    }
}

/// Five-slot continuous formation.
///
/// Slots are indexed 0..[`SLOT_COUNT`]; the index *is* the position —
/// no separate ordering metadata. See [`SLOT_NAMES`] for the canonical
/// labels.
///
/// `Default` is intentionally **not** derived — a default-constructed
/// `Formation` would have every slot's `engagement` / `exposure` at
/// `Q3232::ZERO`, which is *uncomputed* state, not a meaningful zero.
/// Construct via `beast_sim::formation::build` (which calls `recompute`)
/// or by storing the array literal and calling `recompute` explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Formation {
    /// Slots in canonical index order. Always exactly five.
    pub slots: [FormationSlot; SLOT_COUNT],
}

impl Formation {
    /// Wrap a slot array into a `Formation`. Engagement / exposure are
    /// **not** computed by this constructor — call
    /// `beast_sim::formation::recompute` afterwards.
    ///
    /// This is the data-only constructor; use `beast_sim::formation::build`
    /// when you want a fully-derived formation in one step.
    #[must_use]
    pub fn from_slots(slots: [FormationSlot; SLOT_COUNT]) -> Self {
        Self { slots }
    }
}

impl Component for Formation {
    type Storage = DenseVecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_count_is_five() {
        assert_eq!(SLOT_COUNT, 5);
        assert_eq!(SLOT_NAMES.len(), SLOT_COUNT);
    }

    #[test]
    fn slot_names_are_distinct() {
        let mut sorted: Vec<&str> = SLOT_NAMES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), SLOT_NAMES.len());
    }

    #[test]
    fn formation_storage_is_densevec() {
        fn is_dense<C>()
        where
            C: specs::Component<Storage = specs::DenseVecStorage<C>>,
        {
        }
        is_dense::<Formation>();
    }

    #[test]
    fn formation_slot_default_is_zeroed() {
        // Default produces an empty, terrain-neutral slot — explicitly
        // *not* a "computed" state. recompute() must run before reads.
        let s = FormationSlot::default();
        assert_eq!(s.occupant, None);
        assert_eq!(s.engagement, Q3232::ZERO);
        assert_eq!(s.exposure, Q3232::ZERO);
        assert_eq!(s.terrain_modifier, Q3232::ZERO);
    }

    #[test]
    fn from_slots_does_not_compute_derived_state() {
        // Pre-condition for beast_sim::formation::recompute — the
        // data-only constructor leaves engagement / exposure at zero.
        let f = Formation::from_slots([FormationSlot::empty(Q3232::ZERO); SLOT_COUNT]);
        for slot in &f.slots {
            assert_eq!(slot.engagement, Q3232::ZERO);
            assert_eq!(slot.exposure, Q3232::ZERO);
        }
    }
}
