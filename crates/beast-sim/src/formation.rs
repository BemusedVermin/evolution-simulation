//! Formation engagement / exposure math (Layer 4).
//!
//! Backs `documentation/systems/06_combat_system.md` §3 ("Formation
//! Model: Continuous Slot Properties"). The data layout
//! ([`beast_ecs::components::Formation`] / [`FormationSlot`]) lives in
//! `beast-ecs`; the pure math that turns occupancy + terrain into
//! per-slot `engagement` and `exposure` lives here so domain logic
//! stays in L4.
//!
//! # Determinism
//!
//! * Q32.32 throughout (`beast_core::Q3232`); no `f32`/`f64` on the sim
//!   path — `[lints.clippy] float_arithmetic = "deny"` is set in
//!   `beast-sim/Cargo.toml` to enforce it.
//! * Adjacency is a `BTreeMap<u8, BTreeSet<u8>>` initialised once via
//!   `OnceLock`; iteration is sorted by slot index so the order in
//!   which neighbour contributions are summed is fixed.
//! * Per-slot baseline arrays are also `OnceLock<[Q3232; 5]>` — built
//!   from the same `f64` literals every run, giving bit-identical
//!   Q32.32 values for a given build of `fixed`.
//!
//! Same input (occupancy + terrain) → byte-identical Q32.32 output.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use beast_core::Q3232;
use beast_ecs::components::{Formation, FormationSlot, SLOT_COUNT};

/// Per-slot baseline `engagement` (proximity-to-melee, `[0, 1]`).
///
/// Used by [`compute_engagement`]: the template baseline gets bumped up
/// when adjacent slots are empty (no allies to absorb threats) and
/// stays put when they're occupied. Values are calibration knobs and
/// may shift across stories, but the relative ordering — vanguard >
/// flanks > center > rear — is locked by combat doc §3.
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

/// Canonical adjacency table for the 5-slot formation.
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
///
/// Note the deliberate non-edges: flank-left (1) and flank-right (2)
/// are **not** adjacent to each other, and vanguard (0) is **not**
/// adjacent to center (3). Per doc §3 the formation is rank-ordered —
/// lateral coordination between flanks flows through the center as
/// intermediary, and rear-line support reaches the front through
/// center as well. A future maintainer reading the diagram may be
/// tempted to "fix" the missing diagonals; resist — it would collapse
/// the rank-ordered topology that gives center its tactical role.
///
/// Built once via `OnceLock`; [`slot_adjacency`] returns a `&'static`
/// reference. The map is purely a function of the compile-time
/// `SLOT_COUNT`, so single-init is correct.
///
/// `OnceLock` rather than `LazyLock` because the workspace MSRV is
/// 1.75 (`Cargo.toml`) and `LazyLock` only stabilised in 1.80. The
/// codebase already uses `OnceLock` for similar single-init caches —
/// see `beast_chronicler::label::compiled_schema`.
static ADJACENCY: OnceLock<BTreeMap<u8, BTreeSet<u8>>> = OnceLock::new();

/// Per-slot baseline `engagement` as Q32.32. Built from
/// [`BASE_ENGAGEMENT`] once via `OnceLock`.
static BASELINE_ENGAGEMENT: OnceLock<[Q3232; SLOT_COUNT]> = OnceLock::new();

/// Per-slot baseline `exposure` as Q32.32. Built from
/// [`BASE_EXPOSURE`] once via `OnceLock`.
static BASELINE_EXPOSURE: OnceLock<[Q3232; SLOT_COUNT]> = OnceLock::new();

/// Engagement bump per empty adjacent slot, as Q32.32.
static GAP: OnceLock<Q3232> = OnceLock::new();

/// Exposure reduction per occupied adjacent slot, as Q32.32.
static SHIELD: OnceLock<Q3232> = OnceLock::new();

/// Borrow the canonical adjacency table.
///
/// `BTreeMap` / `BTreeSet` iteration is sorted by key, so neighbour
/// contributions are summed in a fixed, documented order — the
/// determinism contract for [`compute_engagement`] / [`compute_exposure`]
/// rests on that ordering.
#[must_use]
pub fn slot_adjacency() -> &'static BTreeMap<u8, BTreeSet<u8>> {
    ADJACENCY.get_or_init(|| {
        let mut m: BTreeMap<u8, BTreeSet<u8>> = BTreeMap::new();
        m.insert(0, BTreeSet::from([1, 2]));
        m.insert(1, BTreeSet::from([0, 3]));
        m.insert(2, BTreeSet::from([0, 3]));
        m.insert(3, BTreeSet::from([1, 2, 4]));
        m.insert(4, BTreeSet::from([3]));
        m
    })
}

fn baseline_engagement() -> &'static [Q3232; SLOT_COUNT] {
    BASELINE_ENGAGEMENT.get_or_init(|| {
        [
            Q3232::from_num(BASE_ENGAGEMENT[0]),
            Q3232::from_num(BASE_ENGAGEMENT[1]),
            Q3232::from_num(BASE_ENGAGEMENT[2]),
            Q3232::from_num(BASE_ENGAGEMENT[3]),
            Q3232::from_num(BASE_ENGAGEMENT[4]),
        ]
    })
}

fn baseline_exposure() -> &'static [Q3232; SLOT_COUNT] {
    BASELINE_EXPOSURE.get_or_init(|| {
        [
            Q3232::from_num(BASE_EXPOSURE[0]),
            Q3232::from_num(BASE_EXPOSURE[1]),
            Q3232::from_num(BASE_EXPOSURE[2]),
            Q3232::from_num(BASE_EXPOSURE[3]),
            Q3232::from_num(BASE_EXPOSURE[4]),
        ]
    })
}

fn gap() -> Q3232 {
    *GAP.get_or_init(|| Q3232::from_num(ENGAGEMENT_GAP))
}

fn shield() -> Q3232 {
    *SHIELD.get_or_init(|| Q3232::from_num(EXPOSURE_SHIELD))
}

/// Compute the per-slot `engagement` value.
///
/// Pure function. Given the current occupancy of every slot and the
/// adjacency table, returns `[Q3232; SLOT_COUNT]` where index `i` is
/// slot `i`'s engagement.
///
/// Formula: `engagement[i] = base_engagement[i] + 0.1 * empty_neighbours[i]`,
/// then clamped to `[0, 1]`. An empty adjacent slot raises a slot's
/// engagement (no ally to share melee pressure).
///
/// Panics if the adjacency table is missing a slot index in
/// `0..SLOT_COUNT` — indicates the table fell out of sync with
/// [`SLOT_COUNT`] and surfacing the gap immediately is preferable to
/// silently treating the slot as having zero neighbours.
#[must_use]
pub fn compute_engagement(
    slots: &[FormationSlot; SLOT_COUNT],
    adjacency: &BTreeMap<u8, BTreeSet<u8>>,
) -> [Q3232; SLOT_COUNT] {
    let gap = gap();
    let baseline = *baseline_engagement();
    let mut out = [Q3232::ZERO; SLOT_COUNT];
    for idx in 0..SLOT_COUNT {
        let neighbours = adjacency.get(&(idx as u8)).unwrap_or_else(|| {
            panic!(
                "slot {idx} missing from adjacency table — \
                 update slot_adjacency() when SLOT_COUNT changes",
            )
        });
        let empty_count = neighbours
            .iter()
            .filter(|n| slots[**n as usize].occupant.is_none())
            .count();
        let bonus = gap * Q3232::from_num(empty_count as i32);
        out[idx] = (baseline[idx] + bonus).clamp(Q3232::ZERO, Q3232::ONE);
    }
    out
}

/// Compute the per-slot `exposure` value.
///
/// Pure function. Given the current occupancy + per-slot terrain
/// modifier and the adjacency table, returns `[Q3232; SLOT_COUNT]`
/// where index `i` is slot `i`'s exposure.
///
/// Formula: `exposure[i] = base_exposure[i] + terrain_modifier[i] -
/// 0.1 * occupied_neighbours[i]`, clamped to `[0, 1]`. An occupied
/// adjacent slot reduces a slot's exposure (the ally provides cover);
/// terrain raises or lowers it linearly.
///
/// Panics if the adjacency table is missing a slot — see
/// [`compute_engagement`] for the rationale.
#[must_use]
pub fn compute_exposure(
    slots: &[FormationSlot; SLOT_COUNT],
    adjacency: &BTreeMap<u8, BTreeSet<u8>>,
) -> [Q3232; SLOT_COUNT] {
    let shield = shield();
    let baseline = *baseline_exposure();
    let mut out = [Q3232::ZERO; SLOT_COUNT];
    for idx in 0..SLOT_COUNT {
        let neighbours = adjacency.get(&(idx as u8)).unwrap_or_else(|| {
            panic!(
                "slot {idx} missing from adjacency table — \
                 update slot_adjacency() when SLOT_COUNT changes",
            )
        });
        let occupied_count = neighbours
            .iter()
            .filter(|n| slots[**n as usize].occupant.is_some())
            .count();
        let shielding = shield * Q3232::from_num(occupied_count as i32);
        let raw = baseline[idx] + slots[idx].terrain_modifier - shielding;
        out[idx] = raw.clamp(Q3232::ZERO, Q3232::ONE);
    }
    out
}

/// Recompute every slot's `engagement` and `exposure` from current
/// occupancy + terrain, using the canonical adjacency table.
///
/// Pure with respect to inputs (occupancy + terrain → derived state):
/// same input → byte-identical Q32.32 output. This is the determinism
/// property pinned by [`tests::recompute_is_byte_identical_for_same_input`].
pub fn recompute(formation: &mut Formation) {
    let adjacency = slot_adjacency();
    let engagements = compute_engagement(&formation.slots, adjacency);
    let exposures = compute_exposure(&formation.slots, adjacency);
    for (idx, slot) in formation.slots.iter_mut().enumerate() {
        slot.engagement = engagements[idx];
        slot.exposure = exposures[idx];
    }
}

/// Build a `Formation` from the five slot definitions, recomputing
/// engagement / exposure so the returned value is internally
/// consistent.
///
/// One-shot convenience wrapper around
/// `Formation::from_slots(slots)` followed by [`recompute`].
#[must_use]
pub fn build(slots: [FormationSlot; SLOT_COUNT]) -> Formation {
    let mut f = Formation::from_slots(slots);
    recompute(&mut f);
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    /// Assert two `Q3232` values agree to within a 4-ULP tolerance at
    /// Q32.32 (`≈ 9.3e-10`). The exact bits drift by 1 LSB across
    /// `from_num(0.6)` vs `from_num(0.5) + from_num(0.1)` because
    /// `0.6_f64` and `0.5_f64 + 0.1_f64` aren't bit-identical IEEE-754
    /// inputs to `Q3232::from_num`. We don't need bit equality for the
    /// *value* shape tests — `recompute_is_byte_identical_for_same_input`
    /// / `recompute_is_idempotent` cover the bit-equality contract.
    fn assert_q_close(actual: Q3232, expected: Q3232) {
        let diff = (actual - expected).saturating_abs();
        let tolerance = Q3232::from_bits(4);
        assert!(
            diff <= tolerance,
            "expected {expected:?}, got {actual:?} (diff {diff:?} > tol {tolerance:?})",
        );
    }

    fn empty_formation() -> Formation {
        build([FormationSlot::empty(Q3232::ZERO); SLOT_COUNT])
    }

    fn full_formation() -> Formation {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        for (idx, slot) in slots.iter_mut().enumerate() {
            slot.occupant = Some(idx as u32);
        }
        build(slots)
    }

    // --- Adjacency --------------------------------------------------------

    #[test]
    fn adjacency_is_symmetric_and_complete() {
        let adj = slot_adjacency();
        for (&a, neighbours) in adj {
            for &b in neighbours {
                assert!(
                    adj.get(&b).is_some_and(|set| set.contains(&a)),
                    "asymmetric edge: {a} -> {b} but not {b} -> {a}",
                );
            }
        }
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

    #[test]
    fn adjacency_returns_same_static_reference() {
        // OnceLock produces one allocation; subsequent calls return
        // the same static reference.
        let a = slot_adjacency() as *const _;
        let b = slot_adjacency() as *const _;
        assert_eq!(a, b, "slot_adjacency must return a stable static");
    }

    // --- Engagement / exposure shapes ------------------------------------

    #[test]
    fn empty_formation_has_max_engagement_at_front() {
        let f = empty_formation();
        // Vanguard: base 0.9 + 2 empty neighbours * 0.1 = 1.1 → clamped to 1.0.
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
        assert_eq!(f.slots[0].exposure, q(0.8));
        assert_eq!(f.slots[1].exposure, q(0.7));
        assert_eq!(f.slots[2].exposure, q(0.7));
        assert_eq!(f.slots[3].exposure, q(0.5));
        assert_eq!(f.slots[4].exposure, q(0.3));
    }

    #[test]
    fn partial_formation_only_vanguard_held() {
        // Only vanguard occupied. Flanks (0's neighbours) are empty.
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].occupant = Some(42);
        let f = build(slots);

        assert_eq!(f.slots[0].engagement, Q3232::ONE);
        assert_q_close(f.slots[1].engagement, q(0.7));
        assert_q_close(f.slots[2].engagement, q(0.7));
        // Center: 3 empty neighbours (1, 2, 4) → 0.4 + 0.3 = 0.7
        assert_q_close(f.slots[3].engagement, q(0.7));
        // Rear: 1 empty neighbour (3) → 0.1 + 0.1 = 0.2
        assert_q_close(f.slots[4].engagement, q(0.2));

        // Exposure: vanguard has 0 occupied neighbours, base 0.8.
        assert_eq!(f.slots[0].exposure, q(0.8));
        // Flank-L: base 0.7 - 1 occupied (vanguard) = 0.6
        assert_q_close(f.slots[1].exposure, q(0.6));
        assert_q_close(f.slots[2].exposure, q(0.6));
    }

    #[test]
    fn edge_occupant_at_rear_only() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[4].occupant = Some(99);
        let f = build(slots);

        assert_q_close(f.slots[4].engagement, q(0.2));
        assert_q_close(f.slots[3].engagement, q(0.6));
        assert_q_close(f.slots[3].exposure, q(0.4));
    }

    // --- Terrain ---------------------------------------------------------

    #[test]
    fn terrain_modifier_raises_exposure_linearly() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[3].terrain_modifier = q(0.2);
        let f = build(slots);
        assert_q_close(f.slots[3].exposure, q(0.7));
    }

    #[test]
    fn terrain_modifier_can_lower_exposure() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[3].terrain_modifier = q(-0.4);
        let f = build(slots);
        assert_q_close(f.slots[3].exposure, q(0.1));
    }

    #[test]
    fn exposure_clamps_at_zero_for_extreme_terrain() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].terrain_modifier = q(-2.0);
        let f = build(slots);
        assert_eq!(f.slots[0].exposure, Q3232::ZERO);
    }

    #[test]
    fn exposure_clamps_at_one_for_extreme_terrain() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].terrain_modifier = q(2.0);
        let f = build(slots);
        assert_eq!(f.slots[0].exposure, Q3232::ONE);
    }

    // --- Determinism (the DoD property) ----------------------------------

    #[test]
    fn recompute_is_byte_identical_for_same_input() {
        let mut slots_a = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots_a[0].occupant = Some(1);
        slots_a[3].occupant = Some(7);
        slots_a[3].terrain_modifier = q(0.15);
        let slots_b = slots_a;

        let f_a = build(slots_a);
        let f_b = build(slots_b);

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
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[1].occupant = Some(2);
        slots[3].occupant = Some(4);
        slots[2].terrain_modifier = q(0.1);
        let mut f = build(slots);

        let snapshot: Vec<(i64, i64)> = f
            .slots
            .iter()
            .map(|s| (s.engagement.to_bits(), s.exposure.to_bits()))
            .collect();

        for _ in 0..5 {
            recompute(&mut f);
            let after: Vec<(i64, i64)> = f
                .slots
                .iter()
                .map(|s| (s.engagement.to_bits(), s.exposure.to_bits()))
                .collect();
            assert_eq!(snapshot, after, "recompute is not idempotent");
        }
    }
}
