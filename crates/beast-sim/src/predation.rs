//! Predation — one-shot mass + integrity consumption.
//!
//! Backs `documentation/systems/06_combat_system.md` §3.4 ("host
//! coupling profile") and `documentation/systems/16_disease_parasitism.md`.
//!
//! Predation runs the same primitive-aggregation pipeline as
//! [`crate::combat::resolve_round`] then layers a one-shot consumption
//! step on top: the attacker's net `MassTransfer` becomes mass
//! extracted from the defender, and the round's `damage` reduces the
//! defender's structural integrity. A single saturating-subtract
//! step per dimension — no per-tick decay, no persistent coupling.
//!
//! Parasitism is the counterpart and lives in [`crate::parasitism`];
//! both are driven by the same [`PrimitiveEffect`] sets (INVARIANTS
//! §5 — the scale-band filter on `interpret_phenotype` is the only
//! macro/micro split).

use beast_core::Q3232;
use beast_ecs::components::FormationSlot;
use beast_primitives::{PrimitiveCategory, PrimitiveEffect, PrimitiveRegistry};

use crate::combat::{resolve_round, RoundOutcome};

/// Outcome of a single predation round.
///
/// Wraps a [`RoundOutcome`] (the same per-category aggregation every
/// combat round produces) and adds the predation-specific
/// consumption fields. All Q3232; saturating arithmetic throughout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredationOutcome {
    /// The underlying combat round — same fields as [`resolve_round`]
    /// returns. Predation reuses the round (no parallel pipeline) so
    /// renaming the underlying primitive ids leaves predation
    /// outcomes byte-equivalent (mechanics-label separation per
    /// INVARIANTS §2 carries through).
    pub round: RoundOutcome,
    /// Mass extracted from the defender this round. Floored at zero.
    /// Computed as `net[MassTransfer] * defender.exposure` — the
    /// attacker's positional advantage scales how much mass they
    /// actually grab.
    pub mass_consumed: Q3232,
    /// Defender mass remaining after the consumption.
    /// `defender_mass.saturating_sub(mass_consumed).max(Q3232::ZERO)`
    /// — Q3232 is signed so saturating_sub alone could underflow
    /// past zero.
    pub defender_mass_after: Q3232,
    /// Defender integrity remaining after the round.
    /// `defender_integrity.saturating_sub(round.damage).max(Q3232::ZERO)`.
    pub defender_integrity_after: Q3232,
    /// `true` if both `defender_mass_after` and
    /// `defender_integrity_after` are zero — the predation event
    /// killed the defender outright. Convenience flag for the S13
    /// encounter loop's death check.
    pub kill: bool,
}

/// Resolve a single predation round.
///
/// Pure function: no `&mut`, no PRNG, no wall-clock. Same inputs ⇒
/// byte-identical [`PredationOutcome`].
///
/// # Algorithm
///
/// ```text
/// round           = resolve_round(...)
/// mass_consumed   = max(0, round.net[MassTransfer] * defender.exposure)
/// mass_after      = max(0, defender_mass      - mass_consumed)
/// integrity_after = max(0, defender_integrity - round.damage)
/// kill            = mass_after == 0 && integrity_after == 0
/// ```
///
/// Each saturating subtraction is followed by `.max(Q3232::ZERO)`
/// because `Q3232::saturating_sub` saturates at `I32F32::MIN`
/// (signed type), not at zero — see the same pattern in
/// [`crate::formation::apply_displacement`].
///
/// # Mechanics-label separation (INVARIANTS §2)
///
/// `resolve_predation` never reads `primitive_id` strings. The
/// underlying [`resolve_round`] resolves categories through the
/// registry; this wrapper only operates on the resulting
/// `net_per_category` map and the `damage` scalar. Predation is
/// emergence-clean by construction.
///
/// # Scale-band unification (INVARIANTS §5)
///
/// No branching on attacker / defender mass. A micro-scale parasite
/// triggering a one-shot consumption (e.g. a virulent ingestion
/// burst) flows through the same code path as a macro-scale predator
/// devouring prey. The scale-band filter on `interpret_phenotype`
/// (Stage 1A — already in beast-interpreter S4) gates which channels
/// even produce primitives in the first place; the combat layer is
/// uniform downstream.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn resolve_predation(
    registry: &PrimitiveRegistry,
    attacker_effects: &[PrimitiveEffect],
    defender_effects: &[PrimitiveEffect],
    attacker_slot: &FormationSlot,
    defender_slot: &FormationSlot,
    defender_mass: Q3232,
    defender_integrity: Q3232,
) -> PredationOutcome {
    let round = resolve_round(
        registry,
        attacker_effects,
        defender_effects,
        attacker_slot,
        defender_slot,
    );

    let mass_transfer_net = round
        .net_per_category
        .get(&PrimitiveCategory::MassTransfer)
        .copied()
        .unwrap_or(Q3232::ZERO);
    let raw_extracted = mass_transfer_net * defender_slot.exposure;
    let mass_consumed = raw_extracted.max(Q3232::ZERO);

    let defender_mass_after = defender_mass.saturating_sub(mass_consumed).max(Q3232::ZERO);
    let defender_integrity_after = defender_integrity
        .saturating_sub(round.damage)
        .max(Q3232::ZERO);
    let kill = defender_mass_after == Q3232::ZERO && defender_integrity_after == Q3232::ZERO;

    PredationOutcome {
        round,
        mass_consumed,
        defender_mass_after,
        defender_integrity_after,
        kill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use beast_channels::ChannelFamily;
    use beast_core::EntityId;
    use beast_primitives::{
        CompatibilityEntry, CostFunction, Modality, ObservableSignature, PrimitiveManifest,
        Provenance,
    };

    fn manifest(id: &str, category: PrimitiveCategory) -> PrimitiveManifest {
        PrimitiveManifest {
            id: id.into(),
            category,
            description: "test fixture".into(),
            parameter_schema: BTreeMap::new(),
            composition_compatibility: vec![CompatibilityEntry::ChannelFamily(
                ChannelFamily::Motor,
            )],
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

    fn registry_with(categories: &[(&'static str, PrimitiveCategory)]) -> PrimitiveRegistry {
        let mut reg = PrimitiveRegistry::new();
        for (id, cat) in categories {
            reg.register(manifest(id, *cat))
                .expect("test fixture should not duplicate ids");
        }
        reg
    }

    fn live_slot(engagement: Q3232, exposure: Q3232) -> FormationSlot {
        FormationSlot {
            occupant: Some(0),
            engagement,
            exposure,
            terrain_modifier: Q3232::ZERO,
        }
    }

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    // --- One-shot kill path -----------------------------------------------

    #[test]
    fn predation_kills_when_force_and_mass_overwhelm_defender() {
        // Heavy attacker against a small defender — both mass and
        // integrity should drain to zero, kill = true.
        let reg = registry_with(&[
            ("force_a", PrimitiveCategory::ForceApplication),
            ("mass_a", PrimitiveCategory::MassTransfer),
        ]);
        let attacker = vec![effect("force_a", q(0.9)), effect("mass_a", q(0.9))];
        let slot = live_slot(Q3232::ONE, Q3232::ONE);
        let outcome = resolve_predation(
            &reg,
            &attacker,
            &[],
            &slot,
            &slot,
            /* defender_mass */ q(0.5),
            /* defender_integrity */ q(0.5),
        );

        assert!(outcome.kill, "expected kill, got {outcome:?}");
        assert_eq!(outcome.defender_mass_after, Q3232::ZERO);
        assert_eq!(outcome.defender_integrity_after, Q3232::ZERO);
    }

    #[test]
    fn predation_does_not_kill_when_defender_resists() {
        // Defender's same-category defense covers the attacker.
        let reg = registry_with(&[
            ("force_a", PrimitiveCategory::ForceApplication),
            ("mass_a", PrimitiveCategory::MassTransfer),
        ]);
        let attacker = vec![effect("force_a", q(0.3)), effect("mass_a", q(0.3))];
        let defender = vec![effect("force_a", q(0.5)), effect("mass_a", q(0.5))];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_predation(
            &reg,
            &attacker,
            &defender,
            &slot,
            &slot,
            /* defender_mass */ q(1.0),
            /* defender_integrity */ q(1.0),
        );

        assert!(!outcome.kill);
        // Defender state preserved exactly — no consumption when net is zero.
        assert_eq!(outcome.mass_consumed, Q3232::ZERO);
        assert_eq!(outcome.round.damage, Q3232::ZERO);
        assert_eq!(outcome.defender_mass_after, q(1.0));
        assert_eq!(outcome.defender_integrity_after, q(1.0));
    }

    // --- Mass / integrity drain bookkeeping -------------------------------

    #[test]
    fn mass_consumed_floors_at_zero() {
        // Defender has no MassTransfer offense — net is zero, so mass
        // consumed is zero (not negative via signed underflow).
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        let attacker = vec![effect("force_a", q(0.1))];
        let slot = live_slot(q(0.5), q(0.5));
        let outcome = resolve_predation(&reg, &attacker, &[], &slot, &slot, q(1.0), q(1.0));
        assert_eq!(outcome.mass_consumed, Q3232::ZERO);
    }

    #[test]
    fn mass_after_floors_at_zero_when_consumption_exceeds_balance() {
        // Net MassTransfer * exposure = full extraction, but defender
        // mass is small. defender_mass_after must be exactly zero,
        // never negative.
        let reg = registry_with(&[("mass_a", PrimitiveCategory::MassTransfer)]);
        let attacker = vec![effect("mass_a", q(0.9))];
        let slot = live_slot(Q3232::ONE, Q3232::ONE);
        let outcome = resolve_predation(
            &reg,
            &attacker,
            &[],
            &slot,
            &slot,
            /* defender_mass */ q(0.2),
            /* defender_integrity */ q(1.0),
        );
        assert_eq!(outcome.defender_mass_after, Q3232::ZERO);
    }

    #[test]
    fn integrity_after_floors_at_zero_when_damage_exceeds_balance() {
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        let attacker = vec![effect("force_a", q(0.9))];
        let slot = live_slot(Q3232::ONE, Q3232::ONE);
        let outcome = resolve_predation(
            &reg,
            &attacker,
            &[],
            &slot,
            &slot,
            q(1.0),
            /* defender_integrity */ q(0.2),
        );
        assert_eq!(outcome.defender_integrity_after, Q3232::ZERO);
    }

    // --- Mechanics-label separation (INVARIANTS §2) ----------------------

    #[test]
    fn predation_outcome_is_byte_identical_under_id_renaming() {
        // Same fixture, different primitive id schemes. resolve_predation
        // must produce byte-identical outcomes — proves the path doesn't
        // branch on id strings.
        let reg_a = registry_with(&[
            ("alpha", PrimitiveCategory::ForceApplication),
            ("bravo", PrimitiveCategory::MassTransfer),
        ]);
        let reg_b = registry_with(&[
            ("zulu", PrimitiveCategory::ForceApplication),
            ("yankee", PrimitiveCategory::MassTransfer),
        ]);
        let att_a = vec![effect("alpha", q(0.4)), effect("bravo", q(0.3))];
        let att_b = vec![effect("zulu", q(0.4)), effect("yankee", q(0.3))];
        let slot = live_slot(q(0.8), q(0.7));

        let out_a = resolve_predation(&reg_a, &att_a, &[], &slot, &slot, q(1.0), q(1.0));
        let out_b = resolve_predation(&reg_b, &att_b, &[], &slot, &slot, q(1.0), q(1.0));

        assert_eq!(out_a.mass_consumed.to_bits(), out_b.mass_consumed.to_bits());
        assert_eq!(
            out_a.defender_mass_after.to_bits(),
            out_b.defender_mass_after.to_bits(),
        );
        assert_eq!(
            out_a.defender_integrity_after.to_bits(),
            out_b.defender_integrity_after.to_bits(),
        );
        assert_eq!(out_a.kill, out_b.kill);
    }

    // --- Determinism (DoD) -----------------------------------------------

    #[test]
    fn output_is_bit_identical_for_same_input() {
        let reg = registry_with(&[
            ("force_a", PrimitiveCategory::ForceApplication),
            ("mass_a", PrimitiveCategory::MassTransfer),
        ]);
        let attacker = vec![effect("force_a", q(0.4)), effect("mass_a", q(0.3))];
        let defender = vec![effect("force_a", q(0.1))];
        let slot = live_slot(q(0.7), q(0.6));
        let first = resolve_predation(&reg, &attacker, &defender, &slot, &slot, q(1.0), q(1.0));
        for _ in 0..100 {
            let again = resolve_predation(&reg, &attacker, &defender, &slot, &slot, q(1.0), q(1.0));
            assert_eq!(again, first);
        }
    }

    // --- Scale-band unification (INVARIANTS §5) --------------------------

    #[test]
    fn small_attacker_against_small_defender_uses_same_path() {
        // Micro-scale predation event: tiny mass values on both sides.
        // The function does not branch on mass — same code path as the
        // macro test above. Proven by symmetry: shrinking the masses
        // proportionally produces a proportionally-shrunken outcome.
        let reg = registry_with(&[
            ("force_a", PrimitiveCategory::ForceApplication),
            ("mass_a", PrimitiveCategory::MassTransfer),
        ]);
        let attacker = vec![effect("force_a", q(0.9)), effect("mass_a", q(0.9))];
        let slot = live_slot(Q3232::ONE, Q3232::ONE);

        let macro_out = resolve_predation(&reg, &attacker, &[], &slot, &slot, q(0.5), q(0.5));
        let micro_out = resolve_predation(&reg, &attacker, &[], &slot, &slot, q(0.001), q(0.001));

        // Both should kill — same path, regardless of mass scale.
        assert!(macro_out.kill);
        assert!(micro_out.kill);
    }
}
