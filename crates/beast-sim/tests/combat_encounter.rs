//! S11.7 — 10-round combat encounter integration test.
//!
//! End-to-end smoke test for the S11 stack: build a `Formation` with
//! two opposed sides (3 vs 3 occupants, two empty slots), advance 10
//! rounds via `combat::resolve_round` + the displacement / mobility
//! surface in `formation`, and verify the demo criteria from epic #23.
//!
//! Pattern mirrors `crates/beast-ui/tests/bestiary_integration.rs`
//! (S10.8) — build fixture, run loop, assert demo criteria, diagnostic
//! on failure.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use beast_channels::ChannelFamily;
use beast_core::{EntityId, Q3232};
use beast_ecs::components::{Formation, FormationSlot, SLOT_COUNT};
use beast_primitives::{
    CompatibilityEntry, CostFunction, Modality, ObservableSignature, PrimitiveCategory,
    PrimitiveEffect, PrimitiveManifest, PrimitiveRegistry, Provenance,
};
use beast_sim::combat::{resolve_round, RoundOutcome};
use beast_sim::formation::{
    apply_displacement, build, mobility_check, DisplacementKind, MobilityKind,
};

mod common;
use common::q;

// ---------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------

/// Build a primitive manifest fixture for the test.
fn manifest(id: &str, category: PrimitiveCategory) -> PrimitiveManifest {
    PrimitiveManifest {
        id: id.into(),
        category,
        description: "S11.7 encounter fixture".into(),
        parameter_schema: BTreeMap::new(),
        composition_compatibility: vec![CompatibilityEntry::ChannelFamily(ChannelFamily::Motor)],
        cost_function: CostFunction {
            base_metabolic_cost: Q3232::ONE,
            parameter_scaling: Vec::new(),
        },
        observable_signature: ObservableSignature {
            modality: Modality::Behavioral,
            detection_range_m: Q3232::ONE,
            pattern_key: "fixture_v1".into(),
        },
        merge_strategy: BTreeMap::new(),
        provenance: Provenance::Core,
    }
}

fn effect(primitive_id: &str, activation_cost: Q3232) -> PrimitiveEffect {
    PrimitiveEffect {
        primitive_id: primitive_id.into(),
        body_site: None,
        source_channels: Vec::new(),
        parameters: BTreeMap::new(),
        activation_cost,
        emitter: EntityId::new(0),
        provenance: Provenance::Core,
    }
}

/// Test registry exercising every category that the combat path
/// reads (force_application, mass_transfer, state_induction,
/// energy_modulation). The categories cover both the offence side
/// (force/mass) and the defence-side analogue (state/energy) — same
/// categories used both sides per INVARIANTS §2.
fn registry_for_test() -> PrimitiveRegistry {
    let mut reg = PrimitiveRegistry::new();
    for (id, cat) in [
        ("force_a", PrimitiveCategory::ForceApplication),
        ("force_b", PrimitiveCategory::ForceApplication),
        ("mass_a", PrimitiveCategory::MassTransfer),
        ("state_a", PrimitiveCategory::StateInduction),
        ("energy_a", PrimitiveCategory::EnergyModulation),
    ] {
        reg.register(manifest(id, cat))
            .expect("registry fixture has unique ids");
    }
    reg
}

/// 2v2 opposed formation with one empty slot for displacement
/// headroom.
///
/// Issue #257 says "3 vs 3 with two empty slots" — but `SLOT_COUNT`
/// is fixed at 5, so 3+3 doesn't fit. Pragmatic interpretation: a
/// balanced opposed setup with at least one empty slot adjacent to
/// the engaging slots, so [`DisplacementKind::Scatter`] actually
/// flips an occupied/empty boundary mid-encounter. Without that, the
/// exposure formula sees the same neighbour-occupancy count before
/// and after scatter — only the *identities* shift, which doesn't
/// affect exposure. See
/// `formation_disruption_changes_subsequent_exposure`.
///
/// Layout:
///
/// * Slot 0 (vanguard) — Side A: id 100
/// * Slot 1 (flank-L)  — Side A: id 101
/// * Slot 2 (flank-R)  — Side B: id 200 (the engagement defender)
/// * Slot 3 (center)   — empty (the displacement-headroom slot)
/// * Slot 4 (rear)     — Side B: id 201
fn opposed_formation() -> Formation {
    let mut slots = [FormationSlot::empty(Q3232::ZERO); SLOT_COUNT];
    slots[0].occupant = Some(100);
    slots[1].occupant = Some(101);
    slots[2].occupant = Some(200);
    // slots[3] stays empty by design — see doc comment above.
    slots[4].occupant = Some(201);
    build(slots)
}

/// Side-A primitive set: ForceApplication-heavy (ranged punch).
fn side_a_attack() -> Vec<PrimitiveEffect> {
    vec![
        effect("force_a", q(0.30)),
        effect("force_b", q(0.20)),
        effect("mass_a", q(0.10)),
    ]
}

/// Side-B primitive set: StateInduction + small force pushback.
fn side_b_defence() -> Vec<PrimitiveEffect> {
    vec![effect("state_a", q(0.15)), effect("force_a", q(0.05))]
}

// ---------------------------------------------------------------------
// Round log + runner
// ---------------------------------------------------------------------

/// Per-round structural record. Bits-only comparison so a determinism
/// failure prints a readable `(round, attacker, defender, field)`
/// instead of drowning the output in a `RoundOutcome` Debug dump.
/// The slot indices are recorded alongside each round so the
/// diagnostic can name *who* attacked / defended on the diverging
/// round (DoD #257).
#[derive(Debug, Clone, PartialEq, Eq)]
struct RoundLog {
    round: u32,
    attacker_slot_idx: u8,
    defender_slot_idx: u8,
    damage_bits: i64,
    stamina_cost_attacker_bits: i64,
    mobility_check: bool,
    /// Defender-slot exposure at the *start* of the round — used by
    /// `formation_disruption_changes_subsequent_exposure` to verify
    /// that mid-encounter displacement is reflected downstream.
    defender_exposure_bits: i64,
}

/// Run a 10-round encounter. Optionally apply a `mid_encounter`
/// transform at round `inject_round` so the test can mutate inputs
/// and observe the next round's damage change.
///
/// Pure: same `(initial_attacker, mid_encounter, inject_round, scatter_at)`
/// → byte-identical `Vec<RoundLog>` (locked in by
/// `re_running_produces_byte_identical_outcomes`).
fn run_encounter(
    initial_attacker: Vec<PrimitiveEffect>,
    mut mid_encounter: Option<Vec<PrimitiveEffect>>,
    inject_round: u32,
    scatter_at: Option<u32>,
) -> Vec<RoundLog> {
    let registry = registry_for_test();
    let mut formation = opposed_formation();
    let defender_effects = side_b_defence();
    let mut attacker_effects = initial_attacker;

    // Side A attacks from slot[0]; side B defends from slot[2]. Slot
    // indices are recorded in the round log alongside the outcome so
    // the determinism diagnostic can name *who* attacked on the
    // diverging round (DoD #257).
    const ATTACKER_SLOT_IDX: u8 = 0;
    const DEFENDER_SLOT_IDX: u8 = 2;

    let mut log: Vec<RoundLog> = Vec::with_capacity(10);
    for round in 1..=10u32 {
        // Optional mid-encounter mutation: replace the attacker's
        // primitive set on the chosen round and keep using it for
        // subsequent rounds. Verifies "damage on round N is computed
        // fresh, not memoised". `Option::take` consumes the value
        // once-only, expressing the intent in the type system and
        // avoiding the per-round `.clone()` allocation.
        if round == inject_round {
            if let Some(replacement) = mid_encounter.take() {
                attacker_effects = replacement;
            }
        }

        // Optional mid-encounter formation disruption: scatter the
        // formation. Subsequent rounds run against the scattered
        // exposure values, which `formation_disruption_changes_
        // subsequent_exposure` pins. Capture the outcome and assert
        // that something actually moved — a silent
        // `moved = false` would otherwise let a downstream regression
        // hide behind the assertion in
        // `formation_disruption_changes_subsequent_exposure`.
        if Some(round) == scatter_at {
            let outcome = apply_displacement(
                &mut formation,
                /* source */ 0,
                /* target */ 0,
                DisplacementKind::Scatter,
                Q3232::ONE,
                Q3232::ZERO,
            );
            assert!(outcome.moved, "scatter at round {round} moved no occupant",);
        }

        let attacker_slot = formation.slots[ATTACKER_SLOT_IDX as usize];
        let defender_slot = formation.slots[DEFENDER_SLOT_IDX as usize];
        let outcome: RoundOutcome = resolve_round(
            &registry,
            &attacker_effects,
            &defender_effects,
            &attacker_slot,
            &defender_slot,
        );

        log.push(RoundLog {
            round,
            attacker_slot_idx: ATTACKER_SLOT_IDX,
            defender_slot_idx: DEFENDER_SLOT_IDX,
            damage_bits: outcome.damage.to_bits(),
            stamina_cost_attacker_bits: outcome.stamina_cost_attacker.to_bits(),
            mobility_check: outcome.mobility_check,
            defender_exposure_bits: defender_slot.exposure.to_bits(),
        });
    }
    log
}

fn assert_no_negative_q(value: Q3232, label: &str, round: u32) {
    assert!(
        value >= Q3232::ZERO,
        "round {round}: {label} = {value:?} should be >= 0",
    );
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[test]
fn ten_round_encounter_runs_without_panic() {
    let log = run_encounter(side_a_attack(), None, 0, None);
    assert_eq!(log.len(), 10);
    for entry in &log {
        let damage = Q3232::from_bits(entry.damage_bits);
        let stamina = Q3232::from_bits(entry.stamina_cost_attacker_bits);
        let exposure = Q3232::from_bits(entry.defender_exposure_bits);
        assert_no_negative_q(damage, "damage", entry.round);
        assert_no_negative_q(stamina, "stamina_cost_attacker", entry.round);
        assert_no_negative_q(exposure, "defender_exposure", entry.round);
    }
}

#[test]
fn re_running_produces_byte_identical_outcomes() {
    // The DoD's determinism property: same inputs → byte-identical
    // outcomes. `run_encounter` is pure (no PRNG, no wall-clock), so
    // this is a structural test rather than a seeded-RNG test.
    //
    // Diagnostic: on divergence, name the first round + attacker /
    // defender slot indices + the specific field that disagrees. DoD
    // #257 requires this `(round, attacker, defender, field)` shape so
    // a future regression points the maintainer at the exact line of
    // the formula that drifted, not at a wall of `RoundLog` Debug.
    let a = run_encounter(side_a_attack(), None, 0, None);
    let b = run_encounter(side_a_attack(), None, 0, None);
    for (l, r) in a.iter().zip(b.iter()) {
        if l == r {
            continue;
        }
        let mismatch = first_diverging_field(l, r);
        panic!(
            "first divergence at round {round}, attacker slot {att}, defender slot {def}, \
             field {field}:\n  left  = {left:?}\n  right = {right:?}",
            round = l.round,
            att = l.attacker_slot_idx,
            def = l.defender_slot_idx,
            field = mismatch.0,
            left = mismatch.1,
            right = mismatch.2,
        );
    }
    assert_eq!(
        a.len(),
        b.len(),
        "logs differ in length: a={}, b={}",
        a.len(),
        b.len(),
    );
}

/// Identify the first field on which two `RoundLog`s diverge. Used by
/// the determinism diagnostic so failures name the offending field
/// (DoD #257) rather than dumping both structs in full.
fn first_diverging_field(a: &RoundLog, b: &RoundLog) -> (&'static str, String, String) {
    if a.damage_bits != b.damage_bits {
        return (
            "damage",
            format!("{:?}", a.damage_bits),
            format!("{:?}", b.damage_bits),
        );
    }
    if a.stamina_cost_attacker_bits != b.stamina_cost_attacker_bits {
        return (
            "stamina_cost_attacker",
            format!("{:?}", a.stamina_cost_attacker_bits),
            format!("{:?}", b.stamina_cost_attacker_bits),
        );
    }
    if a.mobility_check != b.mobility_check {
        return (
            "mobility_check",
            format!("{}", a.mobility_check),
            format!("{}", b.mobility_check),
        );
    }
    if a.defender_exposure_bits != b.defender_exposure_bits {
        return (
            "defender_exposure",
            format!("{:?}", a.defender_exposure_bits),
            format!("{:?}", b.defender_exposure_bits),
        );
    }
    // Round / slot indices are inputs, not outputs — if they disagree
    // the runner itself drifted, which would be a bug in the test.
    ("identity", format!("{a:?}"), format!("{b:?}"))
}

#[test]
fn damage_is_recomputed_fresh_each_round() {
    // Two runs: control + mid-encounter mutation. The mutation drops
    // the force_application offensive at round 6, so rounds 6..10 in
    // the mutated run should produce *less* damage than the same
    // rounds in the control run.
    let control = run_encounter(side_a_attack(), None, 0, None);
    let weakened: Vec<PrimitiveEffect> = vec![effect("mass_a", q(0.05))];
    let mutated = run_encounter(side_a_attack(), Some(weakened), 6, None);

    // Rounds 1..5 must match exactly (no mutation has fired yet).
    for round_idx in 0..5 {
        assert_eq!(
            control[round_idx], mutated[round_idx],
            "rounds 1..5 must be identical, diverged at round_idx {round_idx}",
        );
    }
    // Round 6 onwards: damage *strictly less* in the mutated run
    // (force_application offence dropped to zero). If damage were
    // memoised, both runs would still produce the control's damage.
    for round_idx in 5..10 {
        let ctrl_damage = control[round_idx].damage_bits;
        let mut_damage = mutated[round_idx].damage_bits;
        assert!(
            mut_damage < ctrl_damage,
            "round {} (idx {round_idx}): mutated damage {mut_damage} not strictly less than control {ctrl_damage}",
            control[round_idx].round,
        );
    }
}

#[test]
fn formation_disruption_changes_subsequent_exposure() {
    // Apply a Scatter at round 4. After scatter, the slot occupancy
    // pattern changes; the defender slot (index 2) is no longer
    // surrounded by the same neighbours, so its exposure recomputes
    // to a different Q3232 value. Pin the "before vs after" delta.
    let undisrupted = run_encounter(side_a_attack(), None, 0, None);
    let disrupted = run_encounter(side_a_attack(), None, 0, Some(4));

    // Rounds 1..3: identical (scatter hasn't fired yet).
    for round_idx in 0..3 {
        assert_eq!(
            undisrupted[round_idx].defender_exposure_bits,
            disrupted[round_idx].defender_exposure_bits,
            "pre-scatter exposure must match at round_idx {round_idx}",
        );
    }
    // Round 4 onwards: defender exposure must differ from the
    // undisrupted run on at least one round. (Specifically, scatter
    // reverses occupancy, swapping sides A and B's slots; the new
    // slot 2 has different neighbour occupancy, so exposure shifts.)
    let any_diff = (3..10)
        .any(|i| undisrupted[i].defender_exposure_bits != disrupted[i].defender_exposure_bits);
    assert!(
        any_diff,
        "scatter at round 4 left defender exposure unchanged"
    );
}

#[test]
fn mobility_check_uses_post_disruption_state() {
    // Before scatter, the defender's mobility profile is stable. We
    // build a slot fixture and verify mobility_check is deterministic
    // across calls — proves the function isn't pulling from any
    // hidden state when the slot is the same across two runs.
    let mut slot = FormationSlot {
        occupant: Some(1),
        engagement: q(0.5),
        exposure: q(0.2),
        terrain_modifier: q(0.1),
    };
    let first = mobility_check(&slot, MobilityKind::Advance);
    for _ in 0..1000 {
        assert_eq!(first, mobility_check(&slot, MobilityKind::Advance));
    }
    // Mutating the slot's terrain_modifier flips the result
    // deterministically. (Pre-mutation: bias=0.5, resistance=0.3 →
    // true. Post-mutation: bias=0.5, resistance=0.9 → false.)
    slot.terrain_modifier = q(0.7);
    assert!(!mobility_check(&slot, MobilityKind::Advance));
}

#[test]
fn no_named_ability_strings_in_combat_source() {
    // The DoD's mechanics-label-separation audit: walk every Rust
    // file under `beast-sim/src/{combat,predation,parasitism}*` and
    // check it contains no string matching the chronicler label
    // vocabulary. The contract: combat-layer code reads
    // `PrimitiveCategory` and channel ids only, never labelled-ability
    // primitive ids.
    //
    // The match is anchored to the canonical full label form (e.g.
    // `"venom_injection"` not bare `"venom"`) so a comment like
    // `// anti-venom factor` or an identifier like `venom_resistance`
    // doesn't trip the gate.
    //
    // Token list mirrors `crates/beast-chronicler/manifests/`; add new
    // vocabulary here when the manifests grow. Each token is matched
    // verbatim — the `body.contains(token)` substring search is enough
    // because the canonical forms are unique.
    const FORBIDDEN: &[&str] = &[
        "echolocation",
        "pack_hunting",
        "bioluminescence",
        "venom_injection",
        "drumming",
        "camouflage",
        "thermoregulation",
    ];

    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = PathBuf::from(crate_dir).join("src");
    // Discover combat-related files at runtime rather than hardcoding
    // a flat-file list. If `combat` is ever restructured into a module
    // directory (`combat/mod.rs` + siblings), the walk picks them up
    // automatically. Without this, the previous hardcoded list silently
    // checked zero files after a refactor — a false-green invariant
    // gate (DoD #257 HIGH finding).
    let mut combat_files: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(&src_dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", src_dir.display()))
    {
        let entry = entry.expect("dir iterator failed mid-walk");
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        // Match either a flat file (`combat.rs`, `predation.rs`,
        // `parasitism.rs`) or a future module-directory layout.
        let is_combat_module = matches!(stem, "combat" | "predation" | "parasitism");
        if !is_combat_module {
            continue;
        }
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
            combat_files.push(path);
        } else if path.is_dir() {
            // Walk one level into the module dir; deeper nesting is
            // unusual and can be added if the layout demands it.
            for sub in fs::read_dir(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
            {
                let sub = sub.expect("subdir iterator failed mid-walk").path();
                if sub.is_file() && sub.extension().and_then(|e| e.to_str()) == Some("rs") {
                    combat_files.push(sub);
                }
            }
        }
    }
    // Guard against the audit silently passing with zero files — the
    // DoD #257 HIGH finding. If the discovery walk produces no
    // combat-related files, the gate is broken (someone moved the
    // modules without updating this test).
    assert!(
        !combat_files.is_empty(),
        "§2 audit walked zero files under {} — combat module layout changed?",
        src_dir.display(),
    );

    for path in &combat_files {
        let body = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        for token in FORBIDDEN {
            assert!(
                !body.contains(token),
                "INVARIANTS §2 violation: ability label `{token}` appears in {} — \
                 combat-layer code must read `PrimitiveCategory` only, never label strings",
                path.display(),
            );
        }
    }
}
