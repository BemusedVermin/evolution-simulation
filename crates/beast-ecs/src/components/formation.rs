//! Formation component — the 5-slot continuous spatial substrate every
//! later combat / AI / UI story reads.
//!
//! Backs `documentation/systems/06_combat_system.md` §3 ("Formation
//! Model: Continuous Slot Properties"). Slots are *not* discrete grid
//! cells — each slot has continuous `engagement` and `exposure` Q32.32
//! values modulated by adjacent slot occupancy and terrain.
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
//! * Adjacency lookups go through [`slot_adjacency`], whose backing
//!   store is a `BTreeMap<u8, BTreeSet<u8>>` — iteration is sorted by
//!   slot index, so the order of contributions to a given slot's
//!   exposure / engagement is fixed.
//! * Slots are a fixed-size `[FormationSlot; 5]`; iteration is array
//!   index order, never insertion order.

use std::collections::{BTreeMap, BTreeSet};

use beast_core::Q3232;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};

/// Number of slots in the canonical formation. Locked at five — the
/// slot semantics in [`SLOT_NAMES`] are wired into the combat doc and
/// every UI screen that reads formation state.
pub const SLOT_COUNT: usize = 5;

/// Canonical slot names in index order.
///
/// * `0` — Vanguard (front line, melee contact)
/// * `1` — Flank-Left  (forward-side, partial melee exposure)
/// * `2` — Flank-Right (forward-side, partial melee exposure)
/// * `3` — Center     (interior, support / AoE focus)
/// * `4` — Rear       (back line, ranged / support)
///
/// String form is stable — UI screens and chronicler labels match on it.
pub const SLOT_NAMES: [&str; SLOT_COUNT] =
    ["vanguard", "flank-left", "flank-right", "center", "rear"];

/// Per-slot baseline `engagement` (proximity-to-melee, `[0, 1]`).
///
/// Used by [`compute_engagement`]: the template baseline gets bumped up
/// when adjacent slots are empty (no allies to absorb threats) and
/// stays put when they're occupied. Values are calibration knobs and
/// may shift across stories, but the relative ordering — vanguard >
/// flanks > center > rear — is locked by the combat doc §3.
///
/// `Q3232::from_num` is not `const`, so we keep the float literals
/// alongside `baseline_engagement()` and pay the (negligible) cost of
/// rebuilding the array per `recompute` call. Using literals here also
/// dodges the bit-pattern fragility of round-to-nearest in
/// `saturating_from_num`.
const BASE_ENGAGEMENT: [f64; SLOT_COUNT] = [
    0.9, // vanguard
    0.6, // flank-left
    0.6, // flank-right
    0.4, // center
    0.1, // rear
];

/// Per-slot baseline `exposure` (targetability, `[0, 1]`).
///
/// Used by [`compute_exposure`]: terrain adds, occupied adjacent slots
/// subtract (shielding), and the result is clamped back into `[0, 1]`.
const BASE_EXPOSURE: [f64; SLOT_COUNT] = [
    0.8, // vanguard
    0.7, // flank-left
    0.7, // flank-right
    0.5, // center
    0.3, // rear
];

/// Engagement bump per *empty* adjacent slot.
///
/// `0.1` per missing neighbour: a vanguard with both flanks empty is
/// drawing more melee attention than one with flanks held.
const ENGAGEMENT_GAP: f64 = 0.1;

/// Exposure reduction per *occupied* adjacent slot. Locked at `0.1` —
/// a flanking ally provides ~10% shielding for an in-formation slot.
const EXPOSURE_SHIELD: f64 = 0.1;

/// The five baseline engagement values as Q32.32. Computed once per
/// call; bit-identical across calls for a given build of the `fixed`
/// crate.
fn baseline_engagement() -> [Q3232; SLOT_COUNT] {
    [
        Q3232::from_num(BASE_ENGAGEMENT[0]),
        Q3232::from_num(BASE_ENGAGEMENT[1]),
        Q3232::from_num(BASE_ENGAGEMENT[2]),
        Q3232::from_num(BASE_ENGAGEMENT[3]),
        Q3232::from_num(BASE_ENGAGEMENT[4]),
    ]
}

/// The five baseline exposure values as Q32.32.
fn baseline_exposure() -> [Q3232; SLOT_COUNT] {
    [
        Q3232::from_num(BASE_EXPOSURE[0]),
        Q3232::from_num(BASE_EXPOSURE[1]),
        Q3232::from_num(BASE_EXPOSURE[2]),
        Q3232::from_num(BASE_EXPOSURE[3]),
        Q3232::from_num(BASE_EXPOSURE[4]),
    ]
}

/// One slot in the formation.
///
/// `occupant` is an opaque encounter-local id (the encounter assembler —
/// S13 — populates it from `specs::Entity::id()` when the encounter is
/// built). Storing a bare `u32` keeps `FormationSlot` `Serialize` /
/// `Deserialize` without bringing `specs::Entity` into the save path.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormationSlot {
    /// Encounter-local id of whoever holds this slot, or `None` if the
    /// slot is empty.
    pub occupant: Option<u32>,
    /// Proximity to melee threats. `[0, 1]`. Computed by
    /// [`compute_engagement`].
    pub engagement: Q3232,
    /// Targetability by enemies. `[0, 1]`. Computed by
    /// [`compute_exposure`].
    pub exposure: Q3232,
    /// Terrain bias added to exposure. Encounter assembler sets this
    /// when the formation drops into a particular tile (e.g. a forest
    /// tile lowers exposure across all slots; an open-plain tile
    /// raises it).
    pub terrain_modifier: Q3232,
}

impl FormationSlot {
    /// Convenience constructor: empty slot with the given terrain
    /// modifier and zero derived values. Engagement / exposure are
    /// expected to be filled in by [`Formation::recompute`] before
    /// the slot is read.
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
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Formation {
    /// Slots in canonical index order. Always exactly five.
    pub slots: [FormationSlot; SLOT_COUNT],
}

impl Formation {
    /// Build a formation from the five slot definitions, then recompute
    /// engagement / exposure so the returned value is internally
    /// consistent.
    #[must_use]
    pub fn new(slots: [FormationSlot; SLOT_COUNT]) -> Self {
        let mut formation = Self { slots };
        formation.recompute();
        formation
    }

    /// Recompute every slot's `engagement` and `exposure` from the
    /// current occupancy + terrain, using the canonical adjacency
    /// table.
    ///
    /// Pure with respect to inputs: same `slots` array (occupancy +
    /// terrain) → same Q32.32 bytes out, byte-for-byte. This is the
    /// determinism property the unit tests pin.
    pub fn recompute(&mut self) {
        let adjacency = slot_adjacency();
        let engagements = compute_engagement(&self.slots, &adjacency);
        let exposures = compute_exposure(&self.slots, &adjacency);
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            slot.engagement = engagements[idx];
            slot.exposure = exposures[idx];
        }
    }
}

impl Component for Formation {
    type Storage = DenseVecStorage<Self>;
}

/// Canonical adjacency table for the 5-slot formation.
///
/// Returns a fresh `BTreeMap` rather than a static — the sorted-iteration
/// guarantee is what the determinism contract rests on, and `BTreeMap`'s
/// `Default` cannot be made `const`. The map is small (five entries),
/// so allocating one per `recompute` call is negligible.
///
/// Topology (matches combat doc §3 — vanguard up front, flanks beside
/// it, center between flanks and rear, rear at the back):
///
/// ```text
///         [0 vanguard]
///        /            \
///   [1 flank-L]   [2 flank-R]
///        \            /
///          [3 center]
///              |
///           [4 rear]
/// ```
#[must_use]
pub fn slot_adjacency() -> BTreeMap<u8, BTreeSet<u8>> {
    let mut m: BTreeMap<u8, BTreeSet<u8>> = BTreeMap::new();
    m.insert(0, BTreeSet::from([1, 2]));
    m.insert(1, BTreeSet::from([0, 3]));
    m.insert(2, BTreeSet::from([0, 3]));
    m.insert(3, BTreeSet::from([1, 2, 4]));
    m.insert(4, BTreeSet::from([3]));
    m
}

/// Compute the per-slot `engagement` value.
///
/// Pure function. Given the current occupancy of every slot and the
/// adjacency table, returns `[Q3232; 5]` where index `i` is slot `i`'s
/// engagement.
///
/// Formula: `engagement[i] = base_engagement[i] + 0.1 * empty_neighbours[i]`,
/// then clamped to `[0, 1]`. An empty adjacent slot raises a slot's
/// engagement (no ally to share melee pressure).
///
/// Iteration over `adjacency[&i]` is `BTreeSet` order — strictly
/// ascending by slot index — so the order in which neighbour
/// contributions are summed is deterministic. Q32.32 addition is
/// associative, so this matters less for `+` than it does for, say,
/// floating-point sums, but the contract is still cheaper to keep
/// stable than to debug.
#[must_use]
pub fn compute_engagement(
    slots: &[FormationSlot; SLOT_COUNT],
    adjacency: &BTreeMap<u8, BTreeSet<u8>>,
) -> [Q3232; SLOT_COUNT] {
    let gap = Q3232::from_num(ENGAGEMENT_GAP);
    let baseline = baseline_engagement();
    let mut out = [Q3232::ZERO; SLOT_COUNT];
    for idx in 0..SLOT_COUNT {
        let neighbours = adjacency.get(&(idx as u8));
        let empty_count = neighbours
            .map(|set| {
                set.iter()
                    .filter(|n| slots[**n as usize].occupant.is_none())
                    .count()
            })
            .unwrap_or(0);
        // Sum gap * empty_count via repeated addition rather than
        // multiplying by an integer — keeps the saturating-add semantics
        // explicit and avoids any int → Q3232 conversion ambiguity.
        let mut bonus = Q3232::ZERO;
        for _ in 0..empty_count {
            bonus += gap;
        }
        out[idx] = (baseline[idx] + bonus).clamp(Q3232::ZERO, Q3232::ONE);
    }
    out
}

/// Compute the per-slot `exposure` value.
///
/// Pure function. Given the current occupancy + per-slot terrain
/// modifier and the adjacency table, returns `[Q3232; 5]` where index
/// `i` is slot `i`'s exposure.
///
/// Formula: `exposure[i] = base_exposure[i] + terrain_modifier[i] -
/// 0.1 * occupied_neighbours[i]`, clamped to `[0, 1]`. An occupied
/// adjacent slot reduces a slot's exposure (the ally provides cover);
/// terrain raises or lowers it linearly.
///
/// Iteration over `adjacency[&i]` is sorted by slot index, matching
/// [`compute_engagement`].
#[must_use]
pub fn compute_exposure(
    slots: &[FormationSlot; SLOT_COUNT],
    adjacency: &BTreeMap<u8, BTreeSet<u8>>,
) -> [Q3232; SLOT_COUNT] {
    let shield = Q3232::from_num(EXPOSURE_SHIELD);
    let baseline = baseline_exposure();
    let mut out = [Q3232::ZERO; SLOT_COUNT];
    for idx in 0..SLOT_COUNT {
        let neighbours = adjacency.get(&(idx as u8));
        let occupied_count = neighbours
            .map(|set| {
                set.iter()
                    .filter(|n| slots[**n as usize].occupant.is_some())
                    .count()
            })
            .unwrap_or(0);
        let mut shielding = Q3232::ZERO;
        for _ in 0..occupied_count {
            shielding += shield;
        }
        let raw = baseline[idx] + slots[idx].terrain_modifier - shielding;
        out[idx] = raw.clamp(Q3232::ZERO, Q3232::ONE);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    /// Assert two `Q3232` values agree to within `2^-30 ≈ 1e-9`. The
    /// exact bits drift by 1 LSB across `from_num(0.6)` vs
    /// `from_num(0.5) + from_num(0.1)` because `0.6_f64` and
    /// `0.5_f64 + 0.1_f64` aren't bit-identical IEEE-754 inputs to
    /// `Q3232::from_num`. We don't need bit equality for the *value*
    /// shape tests — `recompute_is_byte_identical_for_same_input` /
    /// `recompute_is_idempotent` cover the bit-equality contract.
    fn assert_q_close(actual: Q3232, expected: Q3232) {
        let diff = (actual - expected).saturating_abs();
        let tolerance = Q3232::from_bits(4); // 4 ULP at Q32.32
        assert!(
            diff <= tolerance,
            "expected {expected:?}, got {actual:?} (diff {diff:?} > tol {tolerance:?})",
        );
    }

    fn empty_formation() -> Formation {
        Formation::new([FormationSlot::empty(Q3232::ZERO); SLOT_COUNT])
    }

    fn full_formation() -> Formation {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        for (idx, slot) in slots.iter_mut().enumerate() {
            slot.occupant = Some(idx as u32);
        }
        Formation::new(slots)
    }

    // --- Adjacency --------------------------------------------------------

    #[test]
    fn adjacency_is_symmetric_and_complete() {
        // Every (a, b) edge must appear in both directions.
        let adj = slot_adjacency();
        for (&a, neighbours) in &adj {
            for &b in neighbours {
                assert!(
                    adj.get(&b).is_some_and(|set| set.contains(&a)),
                    "asymmetric edge: {a} -> {b} but not {b} -> {a}",
                );
            }
        }
        // Every slot must have at least one neighbour.
        for idx in 0..SLOT_COUNT as u8 {
            assert!(
                !adj.get(&idx)
                    .expect("slot index missing from adjacency")
                    .is_empty(),
                "slot {idx} has empty adjacency",
            );
        }
    }

    #[test]
    fn adjacency_iteration_is_sorted() {
        // BTreeMap + BTreeSet guarantee sorted iteration; this test pins
        // the contract so a future replacement (HashMap, indexmap, etc.)
        // can't silently break determinism.
        let adj = slot_adjacency();
        let keys: Vec<u8> = adj.keys().copied().collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "slot_adjacency keys must iterate sorted");

        for set in adj.values() {
            let xs: Vec<u8> = set.iter().copied().collect();
            let mut s = xs.clone();
            s.sort_unstable();
            assert_eq!(xs, s, "adjacency values must iterate sorted");
        }
    }

    // --- Engagement / exposure shapes ------------------------------------

    #[test]
    fn empty_formation_has_max_engagement_at_front() {
        let f = empty_formation();
        // Vanguard: base 0.9 + 2 empty neighbours * 0.1 = 1.1 → clamped to 1.0.
        // Bit-exact (clamp pins to ONE).
        assert_eq!(f.slots[0].engagement, Q3232::ONE);
        // Rear: base 0.1 + 1 empty neighbour * 0.1 = 0.2.
        assert_q_close(f.slots[4].engagement, q(0.2));
    }

    #[test]
    fn full_formation_uses_baseline_engagement() {
        let f = full_formation();
        // No empty neighbours → bonus is zero, engagement is exactly the
        // base. Same `from_num` literal on both sides → bit-exact.
        assert_eq!(f.slots[0].engagement, q(0.9));
        assert_eq!(f.slots[1].engagement, q(0.6));
        assert_eq!(f.slots[2].engagement, q(0.6));
        assert_eq!(f.slots[3].engagement, q(0.4));
        assert_eq!(f.slots[4].engagement, q(0.1));
    }

    #[test]
    fn full_formation_reduces_exposure_via_neighbours() {
        let f = full_formation();
        // Vanguard: base 0.8 - 2 occupied neighbours * 0.1 = 0.6.
        assert_q_close(f.slots[0].exposure, q(0.6));
        // Center: base 0.5 - 3 occupied neighbours * 0.1 = 0.2.
        assert_q_close(f.slots[3].exposure, q(0.2));
        // Rear: base 0.3 - 1 occupied neighbour * 0.1 = 0.2.
        assert_q_close(f.slots[4].exposure, q(0.2));
    }

    #[test]
    fn empty_formation_uses_baseline_exposure() {
        let f = empty_formation();
        // No occupied neighbours → no shielding, exposure is baseline.
        // Bit-exact (no arithmetic between distinct from_num literals).
        assert_eq!(f.slots[0].exposure, q(0.8));
        assert_eq!(f.slots[1].exposure, q(0.7));
        assert_eq!(f.slots[2].exposure, q(0.7));
        assert_eq!(f.slots[3].exposure, q(0.5));
        assert_eq!(f.slots[4].exposure, q(0.3));
    }

    #[test]
    fn partial_formation_only_vanguard_held() {
        // Only vanguard occupied. Flanks (0's neighbours) are empty, so
        // vanguard takes the full +0.2 engagement bump (clamped because
        // 0.9 + 0.2 = 1.1 → 1.0).
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].occupant = Some(42);
        let f = Formation::new(slots);

        assert_eq!(f.slots[0].engagement, Q3232::ONE);
        // Flank-L: base 0.6 + 1 empty neighbour (center=3 empty;
        // vanguard=0 occupied). One empty neighbour → +0.1.
        assert_q_close(f.slots[1].engagement, q(0.7));
        assert_q_close(f.slots[2].engagement, q(0.7));
        // Center: 3 empty neighbours (1, 2, 4) → 0.4 + 0.3 = 0.7
        assert_q_close(f.slots[3].engagement, q(0.7));
        // Rear: 1 empty neighbour (3) → 0.1 + 0.1 = 0.2
        assert_q_close(f.slots[4].engagement, q(0.2));

        // Exposure: vanguard has 0 occupied neighbours (flanks empty),
        // base 0.8 → 0.8 bit-exact.
        assert_eq!(f.slots[0].exposure, q(0.8));
        // Flank-L: base 0.7 - 1 occupied (vanguard) = 0.6
        assert_q_close(f.slots[1].exposure, q(0.6));
        assert_q_close(f.slots[2].exposure, q(0.6));
    }

    #[test]
    fn edge_occupant_at_rear_only() {
        // Single occupant in the rear slot.
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[4].occupant = Some(99);
        let f = Formation::new(slots);

        // Rear: base 0.1 + 1 empty neighbour (center=3) = 0.2
        assert_q_close(f.slots[4].engagement, q(0.2));
        // Center: 1, 2 empty + 4 occupied → 2 empty → 0.4 + 0.2 = 0.6
        assert_q_close(f.slots[3].engagement, q(0.6));
        // Center exposure: 1 occupied neighbour (rear) - 0.1 = 0.4
        assert_q_close(f.slots[3].exposure, q(0.4));
    }

    // --- Terrain ---------------------------------------------------------

    #[test]
    fn terrain_modifier_raises_exposure_linearly() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[3].terrain_modifier = q(0.2);
        let f = Formation::new(slots);
        // Center: base 0.5 + terrain 0.2 - 0 shielding = 0.7.
        assert_q_close(f.slots[3].exposure, q(0.7));
    }

    #[test]
    fn terrain_modifier_can_lower_exposure() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[3].terrain_modifier = q(-0.4);
        let f = Formation::new(slots);
        // Center: base 0.5 + (-0.4) = 0.1, no shielding.
        assert_q_close(f.slots[3].exposure, q(0.1));
    }

    #[test]
    fn exposure_clamps_at_zero_for_extreme_terrain() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].terrain_modifier = q(-2.0);
        let f = Formation::new(slots);
        assert_eq!(f.slots[0].exposure, Q3232::ZERO);
    }

    #[test]
    fn exposure_clamps_at_one_for_extreme_terrain() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].terrain_modifier = q(2.0);
        let f = Formation::new(slots);
        assert_eq!(f.slots[0].exposure, Q3232::ONE);
    }

    // --- Determinism (the DoD property) ----------------------------------

    #[test]
    fn recompute_is_byte_identical_for_same_input() {
        // Build two formations with the same inputs (occupancy + terrain)
        // and assert every Q3232 byte matches. This is the DoD property.
        let mut slots_a = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots_a[0].occupant = Some(1);
        slots_a[3].occupant = Some(7);
        slots_a[3].terrain_modifier = q(0.15);
        let slots_b = slots_a;

        let f_a = Formation::new(slots_a);
        let f_b = Formation::new(slots_b);

        for idx in 0..SLOT_COUNT {
            assert_eq!(
                f_a.slots[idx].engagement.to_bits(),
                f_b.slots[idx].engagement.to_bits(),
                "engagement bits diverged at slot {idx}",
            );
            assert_eq!(
                f_a.slots[idx].exposure.to_bits(),
                f_b.slots[idx].exposure.to_bits(),
                "exposure bits diverged at slot {idx}",
            );
        }
    }

    #[test]
    fn recompute_is_idempotent() {
        // Calling recompute twice with no input change must not drift —
        // the function reads its own outputs only via the slot array, but
        // engagement / exposure don't feed back, so the second call must
        // produce byte-identical state.
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[1].occupant = Some(2);
        slots[3].occupant = Some(4);
        slots[2].terrain_modifier = q(0.1);
        let mut f = Formation::new(slots);

        let snapshot: Vec<(i64, i64)> = f
            .slots
            .iter()
            .map(|s| (s.engagement.to_bits(), s.exposure.to_bits()))
            .collect();

        for _ in 0..5 {
            f.recompute();
            let after: Vec<(i64, i64)> = f
                .slots
                .iter()
                .map(|s| (s.engagement.to_bits(), s.exposure.to_bits()))
                .collect();
            assert_eq!(snapshot, after, "recompute is not idempotent");
        }
    }

    // --- Storage shape ---------------------------------------------------

    #[test]
    fn formation_storage_is_densevec() {
        fn is_dense<C>()
        where
            C: specs::Component<Storage = specs::DenseVecStorage<C>>,
        {
        }
        is_dense::<Formation>();
    }
}
