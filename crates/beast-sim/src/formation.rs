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

// =====================================================================
// S11.5 — Displacement and zone-of-control mobility checks
// =====================================================================

/// Kind of formation-disrupting displacement to apply.
///
/// Backs combat doc §4 ("Mobility & Zone-of-Control Checks") and the
/// Into-the-Breach displacement-as-tactic inspiration cited in §2 — a
/// scattered formation is as dangerous as a damaged one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplacementKind {
    /// Shove the target's occupant *away* from the source — into the
    /// neighbour of `target` that is **furthest** from `source` in the
    /// adjacency graph (ties broken by smallest slot index). Occupants
    /// swap if the destination is held; nothing happens if `target`
    /// has no neighbours, no occupant, or is the source itself.
    Push,
    /// Drag the target's occupant *toward* the source — into the
    /// neighbour of `target` that is **closest** to `source` (ties
    /// broken by smallest slot index). Mirrors `Push`.
    Pull,
    /// Reverse the slot order: occupant of slot `0` ↔ `4`, `1` ↔ `3`,
    /// `2` is its own image. Deterministic and self-inverse — applying
    /// `Scatter` twice returns the original occupancy. `source` and
    /// `target` are ignored.
    Scatter,
}

/// Kind of mobility action a slot's occupant is trying to perform.
///
/// Backs combat doc §4: each kind reads the slot's terrain, exposure,
/// and engagement scalars to compute a deterministic pass/fail.
/// (The randomised variant — sigmoid-of-mobility-advantage — lands in
/// a future story alongside the active-inference policy interface.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobilityKind {
    /// Move forward into a more-engaged slot. Easier when the
    /// occupant is already engaged (i.e. close to melee contact) and
    /// terrain doesn't bog them down.
    Advance,
    /// Withdraw to a less-engaged slot. Easier when the occupant is
    /// not currently in melee and terrain isn't trapping them.
    Retreat,
    /// Move laterally. Easier when the slot is not heavily exposed
    /// (an unexposed slot has cover allowing the swing) and terrain
    /// is permissive.
    Flank,
}

/// Outcome of an [`apply_displacement`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplacementOutcome {
    /// `true` if any slot's occupant changed as a result of the call.
    /// `false` for no-op cases (target has no neighbours, source ==
    /// target, source/target out of range, etc.).
    pub moved: bool,
    /// Stamina remaining after the action. Saturating Q32.32
    /// subtraction floors at `Q3232::ZERO`.
    pub stamina_remaining: Q3232,
}

/// Apply a [`DisplacementKind`] to `formation`, deducting `stamina_cost`
/// from `stamina`.
///
/// Pure with respect to its inputs: same `(formation, source, target,
/// kind, stamina, stamina_cost)` → byte-identical
/// [`DisplacementOutcome`] and byte-identical mutated `formation`.
/// No PRNG, no wall-clock — INVARIANTS §1.
///
/// # Determinism
///
/// * `slot_neighbours_sorted_by` walks `slot_adjacency()` in `BTreeSet`
///   order (sorted by slot index), so the tie-breaking rule "smallest
///   index wins" holds without a separate sort.
/// * BFS distances are computed with a `BTreeSet` frontier, also for
///   sorted-iteration determinism.
/// * `recompute` runs at the end so engagement / exposure stay
///   internally consistent with the new occupancy.
///
/// # Stamina
///
/// `stamina_cost` is subtracted via `Q3232::saturating_sub`, which
/// saturates at `I32F32::MIN` (the type is signed). Wrap that in
/// `.max(Q3232::ZERO)` so a cost greater than current stamina simply
/// drains to zero rather than underflowing into a negative reserve.
/// The cost is paid **regardless** of whether the displacement
/// actually moved anyone — the action was attempted.
///
/// # Out-of-range slot indices
///
/// Returns a no-op outcome (`moved = false`) without panicking. The
/// caller (S13 encounter loop) should normalise indices before
/// invoking; surfacing a hard panic on a stray index would let one
/// bad command kill the whole encounter.
pub fn apply_displacement(
    formation: &mut Formation,
    source: u8,
    target: u8,
    kind: DisplacementKind,
    stamina: Q3232,
    stamina_cost: Q3232,
) -> DisplacementOutcome {
    let stamina_remaining = stamina.saturating_sub(stamina_cost).max(Q3232::ZERO);

    let moved = match kind {
        DisplacementKind::Scatter => apply_scatter(formation),
        DisplacementKind::Push | DisplacementKind::Pull => {
            apply_directed_displacement(formation, source, target, kind)
        }
    };

    if moved {
        recompute(formation);
    }

    DisplacementOutcome {
        moved,
        stamina_remaining,
    }
}

/// Reverse occupant order in place. Returns `true` if anything
/// actually moved (false only when the formation is symmetric
/// already, e.g. all empty or palindromic occupancy).
fn apply_scatter(formation: &mut Formation) -> bool {
    let mut moved = false;
    for i in 0..(SLOT_COUNT / 2) {
        let j = SLOT_COUNT - 1 - i;
        if formation.slots[i].occupant != formation.slots[j].occupant {
            moved = true;
        }
        let tmp = formation.slots[i].occupant;
        formation.slots[i].occupant = formation.slots[j].occupant;
        formation.slots[j].occupant = tmp;
    }
    moved
}

/// Push or Pull: swap target's occupant with one of its neighbours
/// chosen by distance from source.
fn apply_directed_displacement(
    formation: &mut Formation,
    source: u8,
    target: u8,
    kind: DisplacementKind,
) -> bool {
    if source == target || (source as usize) >= SLOT_COUNT || (target as usize) >= SLOT_COUNT {
        return false;
    }

    let distances = slot_distances(source);
    let adjacency = slot_adjacency();
    let neighbours = match adjacency.get(&target) {
        Some(set) => set,
        None => return false,
    };

    // Pick the neighbour by Push/Pull semantics. Iteration over
    // `BTreeSet` is sorted by slot index, so the "ties broken by
    // smallest index" rule is implicit when we use `<`/`>` on the
    // distance comparison.
    let mut pick: Option<u8> = None;
    let mut pick_dist: i16 = match kind {
        DisplacementKind::Push => -1, // start below any real distance, so first hit wins
        DisplacementKind::Pull => i16::MAX,
        DisplacementKind::Scatter => unreachable!(),
    };
    for &n in neighbours {
        let d = i16::from(distances[n as usize]);
        let take = match kind {
            DisplacementKind::Push => d > pick_dist,
            DisplacementKind::Pull => d < pick_dist,
            DisplacementKind::Scatter => unreachable!(),
        };
        if take {
            pick = Some(n);
            pick_dist = d;
        }
    }
    let Some(neighbour) = pick else {
        return false; // no neighbours — impossible for the canonical 5-slot graph
    };

    // Swap occupants; engagement/exposure recomputed by caller.
    let tmp = formation.slots[target as usize].occupant;
    formation.slots[target as usize].occupant = formation.slots[neighbour as usize].occupant;
    formation.slots[neighbour as usize].occupant = tmp;
    formation.slots[target as usize].occupant != formation.slots[neighbour as usize].occupant
        || tmp.is_some()
}

/// BFS distances from `start` to every slot in the canonical adjacency.
///
/// Returns `[u8; SLOT_COUNT]` indexed by slot. Unreachable slots get
/// `u8::MAX` (cannot occur on the canonical 5-slot graph since it is
/// connected, but keeps the function total).
fn slot_distances(start: u8) -> [u8; SLOT_COUNT] {
    let mut dist = [u8::MAX; SLOT_COUNT];
    if (start as usize) >= SLOT_COUNT {
        return dist;
    }
    dist[start as usize] = 0;
    // Frontier as BTreeSet so iteration is sorted — keeps BFS layer
    // exploration deterministic.
    let mut frontier: BTreeSet<u8> = BTreeSet::new();
    frontier.insert(start);
    let adjacency = slot_adjacency();
    while !frontier.is_empty() {
        let mut next: BTreeSet<u8> = BTreeSet::new();
        for &node in &frontier {
            let Some(neighbours) = adjacency.get(&node) else {
                continue;
            };
            let node_dist = dist[node as usize];
            for &n in neighbours {
                if dist[n as usize] == u8::MAX {
                    dist[n as usize] = node_dist.saturating_add(1);
                    next.insert(n);
                }
            }
        }
        frontier = next;
    }
    dist
}

/// Deterministic mobility check for the slot's occupant.
///
/// Returns `true` if the action succeeds. The check is a saturating
/// Q32.32 comparison between an action-specific *bias* (based on the
/// slot's `engagement` / `exposure`) and the slot's *resistance*
/// (`terrain_modifier + exposure`):
///
/// ```text
/// resistance = terrain_modifier + exposure         (zone-of-control + ground)
/// bias = match kind {
///     Advance  => engagement,                      // already engaged → easier to push forward
///     Retreat  => 1 - engagement,                  // not in melee   → easier to step back
///     Flank    => 1 - exposure,                    // covered slot  → easier to slip sideways
/// }
/// success = bias > resistance
/// ```
///
/// All scalars are saturating Q3232; out-of-range `engagement` /
/// `exposure` (the `Formation` keeps both clamped, but defensive
/// coding here is cheap) cannot wrap into negative bias.
#[must_use]
pub fn mobility_check(slot: &FormationSlot, kind: MobilityKind) -> bool {
    let resistance = slot.terrain_modifier + slot.exposure;
    let bias = match kind {
        MobilityKind::Advance => slot.engagement,
        MobilityKind::Retreat => Q3232::ONE.saturating_sub(slot.engagement),
        MobilityKind::Flank => Q3232::ONE.saturating_sub(slot.exposure),
    };
    bias > resistance
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

    // --- BFS distance helper --------------------------------------------

    #[test]
    fn slot_distances_from_vanguard_match_canonical_topology() {
        // From slot 0 (vanguard): 1=1, 2=1, 3=2, 4=3.
        let d = slot_distances(0);
        assert_eq!(d, [0, 1, 1, 2, 3]);
    }

    #[test]
    fn slot_distances_from_rear_match_canonical_topology() {
        // From slot 4 (rear): 3=1, 1=2, 2=2, 0=3.
        let d = slot_distances(4);
        assert_eq!(d, [3, 2, 2, 1, 0]);
    }

    #[test]
    fn slot_distances_out_of_range_is_max() {
        let d = slot_distances(99);
        assert_eq!(d, [u8::MAX; SLOT_COUNT]);
    }

    // --- Displacement: Push / Pull --------------------------------------

    #[test]
    fn push_swaps_target_with_neighbour_furthest_from_source() {
        // source=0 (vanguard), target=3 (center). 3's neighbours are
        // {1, 2, 4}; distances from 0 are 1, 1, 3. Furthest is 4.
        // Push should swap 3 ↔ 4.
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[3].occupant = Some(11);
        slots[4].occupant = Some(22);
        let mut f = build(slots);

        let outcome = apply_displacement(
            &mut f,
            /* source */ 0,
            /* target */ 3,
            DisplacementKind::Push,
            Q3232::ONE,
            q(0.1),
        );
        assert!(outcome.moved);
        assert_eq!(f.slots[3].occupant, Some(22));
        assert_eq!(f.slots[4].occupant, Some(11));
    }

    #[test]
    fn pull_swaps_target_with_neighbour_closest_to_source() {
        // source=0 (vanguard), target=3 (center). Closest neighbour of
        // 3 to 0 is 1 (or 2) — tie at distance 1, smallest index wins
        // → 1. Pull should swap 3 ↔ 1.
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[3].occupant = Some(11);
        slots[1].occupant = Some(22);
        let mut f = build(slots);

        let outcome = apply_displacement(
            &mut f,
            /* source */ 0,
            /* target */ 3,
            DisplacementKind::Pull,
            Q3232::ONE,
            q(0.1),
        );
        assert!(outcome.moved);
        assert_eq!(f.slots[3].occupant, Some(22));
        assert_eq!(f.slots[1].occupant, Some(11));
    }

    #[test]
    fn push_then_pull_is_identity_for_unique_neighbour() {
        // Symmetric setup: target=4 (rear) has only one neighbour (3).
        // Both Push and Pull select that single neighbour, so the two
        // ops are mutual inverses.
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[3].occupant = Some(11);
        slots[4].occupant = Some(22);
        let mut f = build(slots);

        let snapshot: Vec<Option<u32>> = f.slots.iter().map(|s| s.occupant).collect();

        apply_displacement(
            &mut f,
            0,
            4,
            DisplacementKind::Push,
            Q3232::ONE,
            Q3232::ZERO,
        );
        apply_displacement(
            &mut f,
            0,
            4,
            DisplacementKind::Pull,
            Q3232::ONE,
            Q3232::ZERO,
        );

        let after: Vec<Option<u32>> = f.slots.iter().map(|s| s.occupant).collect();
        assert_eq!(snapshot, after, "push-then-pull is not identity");
    }

    #[test]
    fn push_with_source_equal_target_is_noop() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[3].occupant = Some(11);
        let mut f = build(slots);
        let snapshot = f;

        let outcome = apply_displacement(
            &mut f,
            3,
            3,
            DisplacementKind::Push,
            Q3232::ONE,
            Q3232::ZERO,
        );
        assert!(!outcome.moved);
        assert_eq!(f.slots, snapshot.slots);
    }

    #[test]
    fn push_with_out_of_range_indices_is_noop() {
        let mut f = build([FormationSlot::empty(Q3232::ZERO); SLOT_COUNT]);
        let outcome = apply_displacement(
            &mut f,
            99,
            3,
            DisplacementKind::Push,
            Q3232::ONE,
            Q3232::ZERO,
        );
        assert!(!outcome.moved);
        let outcome = apply_displacement(
            &mut f,
            0,
            99,
            DisplacementKind::Push,
            Q3232::ONE,
            Q3232::ZERO,
        );
        assert!(!outcome.moved);
    }

    // --- Displacement: Scatter -------------------------------------------

    #[test]
    fn scatter_reverses_occupant_order() {
        // Occupant in slot 0 ↔ 4, 1 ↔ 3, 2 stays.
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].occupant = Some(10);
        slots[1].occupant = Some(20);
        slots[2].occupant = Some(30);
        slots[3].occupant = Some(40);
        slots[4].occupant = Some(50);
        let mut f = build(slots);

        let outcome = apply_displacement(
            &mut f,
            0,
            0,
            DisplacementKind::Scatter,
            Q3232::ONE,
            Q3232::ZERO,
        );
        assert!(outcome.moved);
        assert_eq!(f.slots[0].occupant, Some(50));
        assert_eq!(f.slots[1].occupant, Some(40));
        assert_eq!(f.slots[2].occupant, Some(30));
        assert_eq!(f.slots[3].occupant, Some(20));
        assert_eq!(f.slots[4].occupant, Some(10));
    }

    #[test]
    fn scatter_twice_is_identity() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].occupant = Some(10);
        slots[1].occupant = Some(20);
        slots[3].occupant = Some(40);
        let mut f = build(slots);
        let snapshot: Vec<Option<u32>> = f.slots.iter().map(|s| s.occupant).collect();

        for _ in 0..2 {
            apply_displacement(
                &mut f,
                0,
                0,
                DisplacementKind::Scatter,
                Q3232::ONE,
                Q3232::ZERO,
            );
        }
        let after: Vec<Option<u32>> = f.slots.iter().map(|s| s.occupant).collect();
        assert_eq!(snapshot, after);
    }

    #[test]
    fn scatter_preserves_occupant_count() {
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].occupant = Some(10);
        slots[2].occupant = Some(30);
        slots[4].occupant = Some(50);
        let mut f = build(slots);

        let before: usize = f.slots.iter().filter(|s| s.occupant.is_some()).count();
        apply_displacement(
            &mut f,
            0,
            0,
            DisplacementKind::Scatter,
            Q3232::ONE,
            Q3232::ZERO,
        );
        let after: usize = f.slots.iter().filter(|s| s.occupant.is_some()).count();
        assert_eq!(before, after);
    }

    #[test]
    fn scatter_on_palindromic_formation_reports_no_movement() {
        // Slots 0 and 4 hold the same occupant id; slot 2 stays. The
        // reverse-order swap is a no-op on this configuration.
        let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
        slots[0].occupant = Some(7);
        slots[2].occupant = Some(9);
        slots[4].occupant = Some(7);
        let mut f = build(slots);
        let outcome = apply_displacement(
            &mut f,
            0,
            0,
            DisplacementKind::Scatter,
            Q3232::ONE,
            Q3232::ZERO,
        );
        assert!(!outcome.moved);
    }

    // --- Stamina ---------------------------------------------------------

    #[test]
    fn stamina_drains_by_cost() {
        let mut f = build([FormationSlot::empty(Q3232::ZERO); SLOT_COUNT]);
        let outcome =
            apply_displacement(&mut f, 0, 0, DisplacementKind::Scatter, Q3232::ONE, q(0.3));
        let diff = (outcome.stamina_remaining - q(0.7)).saturating_abs();
        assert!(diff <= Q3232::from_bits(4));
    }

    #[test]
    fn stamina_floors_at_zero_when_cost_exceeds_balance() {
        // Cost > stamina: result must be ZERO, never negative.
        let mut f = build([FormationSlot::empty(Q3232::ZERO); SLOT_COUNT]);
        let outcome = apply_displacement(&mut f, 0, 0, DisplacementKind::Scatter, q(0.2), q(0.5));
        assert_eq!(outcome.stamina_remaining, Q3232::ZERO);
    }

    #[test]
    fn stamina_charged_even_on_noop_displacement() {
        // Action attempted = stamina spent, regardless of whether the
        // formation actually changed.
        let mut f = build([FormationSlot::empty(Q3232::ZERO); SLOT_COUNT]);
        let outcome = apply_displacement(
            &mut f,
            3,
            3, // source == target → no-op
            DisplacementKind::Push,
            Q3232::ONE,
            q(0.4),
        );
        assert!(!outcome.moved);
        let diff = (outcome.stamina_remaining - q(0.6)).saturating_abs();
        assert!(diff <= Q3232::from_bits(4));
    }

    // --- Determinism ----------------------------------------------------

    #[test]
    fn apply_displacement_is_deterministic() {
        let snapshot_run = || {
            let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
            slots[0].occupant = Some(1);
            slots[3].occupant = Some(2);
            slots[3].terrain_modifier = q(0.1);
            let mut f = build(slots);
            apply_displacement(&mut f, 0, 3, DisplacementKind::Push, Q3232::ONE, q(0.05));
            f
        };
        let a = snapshot_run();
        let b = snapshot_run();
        for idx in 0..SLOT_COUNT {
            assert_eq!(a.slots[idx].occupant, b.slots[idx].occupant);
            assert_eq!(
                a.slots[idx].engagement.to_bits(),
                b.slots[idx].engagement.to_bits(),
            );
            assert_eq!(
                a.slots[idx].exposure.to_bits(),
                b.slots[idx].exposure.to_bits(),
            );
        }
    }

    // --- Mobility check --------------------------------------------------

    #[test]
    fn mobility_advance_succeeds_when_engaged_and_terrain_permissive() {
        // engagement > resistance(=terrain+exposure) ⇒ true
        let slot = FormationSlot {
            occupant: Some(1),
            engagement: q(0.8),
            exposure: q(0.2),
            terrain_modifier: q(0.1),
        };
        // resistance = 0.3, bias = 0.8 → true
        assert!(mobility_check(&slot, MobilityKind::Advance));
    }

    #[test]
    fn mobility_advance_fails_when_terrain_dominates() {
        let slot = FormationSlot {
            occupant: Some(1),
            engagement: q(0.2),
            exposure: q(0.5),
            terrain_modifier: q(0.4),
        };
        // resistance = 0.9, bias = 0.2 → false
        assert!(!mobility_check(&slot, MobilityKind::Advance));
    }

    #[test]
    fn mobility_retreat_succeeds_when_disengaged() {
        let slot = FormationSlot {
            occupant: Some(1),
            engagement: q(0.1),
            exposure: q(0.1),
            terrain_modifier: q(0.1),
        };
        // resistance = 0.2, bias = (1 - 0.1) = 0.9 → true
        assert!(mobility_check(&slot, MobilityKind::Retreat));
    }

    #[test]
    fn mobility_retreat_fails_when_in_melee() {
        let slot = FormationSlot {
            occupant: Some(1),
            engagement: q(0.95),
            exposure: q(0.1),
            terrain_modifier: q(0.1),
        };
        // resistance = 0.2, bias = (1 - 0.95) = 0.05 → false
        assert!(!mobility_check(&slot, MobilityKind::Retreat));
    }

    #[test]
    fn mobility_flank_succeeds_when_covered() {
        let slot = FormationSlot {
            occupant: Some(1),
            engagement: q(0.5),
            exposure: q(0.1),
            terrain_modifier: q(0.1),
        };
        // resistance = 0.2, bias = (1 - 0.1) = 0.9 → true
        assert!(mobility_check(&slot, MobilityKind::Flank));
    }

    #[test]
    fn mobility_flank_fails_when_exposed() {
        let slot = FormationSlot {
            occupant: Some(1),
            engagement: q(0.5),
            exposure: q(0.95),
            terrain_modifier: q(0.1),
        };
        // resistance = 1.05, bias = 0.05 → false
        assert!(!mobility_check(&slot, MobilityKind::Flank));
    }

    #[test]
    fn mobility_check_is_deterministic() {
        let slot = FormationSlot {
            occupant: Some(1),
            engagement: q(0.7),
            exposure: q(0.3),
            terrain_modifier: q(0.2),
        };
        let first = mobility_check(&slot, MobilityKind::Advance);
        for _ in 0..1000 {
            assert_eq!(first, mobility_check(&slot, MobilityKind::Advance));
        }
    }
}
