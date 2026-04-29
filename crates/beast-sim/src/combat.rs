//! Combat round resolution from primitives — Layer 4 domain logic.
//!
//! Backs `documentation/systems/06_combat_system.md` §4 ("Damage
//! Formula: From Primitives"). Every number on the combat readout is
//! computed fresh per round from the attacker's
//! [`PrimitiveEffect`] set, the defender's [`PrimitiveEffect`] set,
//! and the formation slot `engagement` / `exposure` scalars. No lookup
//! tables, no balance constants, no hand-tuned matchup numbers —
//! emergence-closure (INVARIANTS §4) gates this story.
//!
//! # Mechanics-label separation (INVARIANTS §2)
//!
//! This module reads [`PrimitiveCategory`] only — never `primitive_id`
//! strings, never named-ability vocabulary. The category mapping comes
//! from [`PrimitiveManifest::category`], which is structural metadata
//! resolved through the registry. The combat path stays
//! emergence-clean: renaming every primitive id while preserving
//! categories must produce byte-identical output (locked in by
//! `tests::renaming_primitive_ids_preserves_outcome`).
//!
//! # Determinism (INVARIANTS §1)
//!
//! [`resolve_round`] is a pure function — no `&mut`, no PRNG, no
//! wall-clock. Saturating Q32.32 throughout. Per-category aggregation
//! uses a `BTreeMap<PrimitiveCategory, _>` so iteration order is
//! sorted by category enum value, fixed across runs. The `+` operator
//! on [`Q3232`] is saturating (see `beast_core::Q3232` docs); damage
//! cannot wrap into negative values.
//!
//! [`PrimitiveManifest::category`]: beast_primitives::PrimitiveManifest::category

use std::collections::BTreeMap;

use beast_core::Q3232;
use beast_ecs::components::FormationSlot;
use beast_primitives::{PrimitiveCategory, PrimitiveEffect, PrimitiveRegistry};

/// All eight primitive categories in declaration order. Iteration over
/// this array gives a deterministic walk over the category space —
/// matches the canonical sort order under
/// `derive(PartialOrd, Ord)` on [`PrimitiveCategory`].
const ALL_CATEGORIES: [PrimitiveCategory; 8] = [
    PrimitiveCategory::SignalEmission,
    PrimitiveCategory::SignalReception,
    PrimitiveCategory::ForceApplication,
    PrimitiveCategory::StateInduction,
    PrimitiveCategory::SpatialIntegration,
    PrimitiveCategory::MassTransfer,
    PrimitiveCategory::EnergyModulation,
    PrimitiveCategory::BondFormation,
];

/// Outcome of a single combat round.
///
/// Every field is derived purely from the inputs to [`resolve_round`]
/// — no PRNG, no wall-clock. Saturating Q32.32 throughout per
/// INVARIANTS §1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundOutcome {
    /// Damage dealt to the defender after slot-scalar attenuation.
    /// Always `>= Q3232::ZERO` — saturating subtraction inside
    /// [`resolve_round`] floors net force at zero before the slot
    /// scalars are applied.
    pub damage: Q3232,
    /// `true` when the attacker's net mass-transfer (positional)
    /// magnitude exceeds the defender's same-category resistance —
    /// i.e. the attacker successfully repositioned past the
    /// defender's zone of control. The Q32.32 form of the
    /// "Mobility & Zone-of-Control Checks" pass/fail in combat doc §4
    /// (deterministic counterpart — randomised version lands in S11.5
    /// alongside the displacement primitives).
    pub mobility_check: bool,
    /// Sum of `activation_cost` over every attacker effect whose
    /// `primitive_id` resolves in the registry. Saturating Q3232 sum,
    /// so an attacker with effects whose costs add past `Q3232::MAX`
    /// simply caps at `MAX` rather than wrapping.
    ///
    /// Unregistered effects do **not** contribute. This is intentional:
    /// the billing gate is the same as the recognition gate, so an
    /// effect with no manifest provenance can neither produce damage
    /// nor consume stamina (which would be a "ghost mechanic" — a
    /// resource cost with no emergence-traceable output, banned by
    /// INVARIANTS §4).
    pub stamina_cost_attacker: Q3232,
    /// Per-category net (offense − defense, saturating, floored at
    /// zero). Keyed by [`PrimitiveCategory`] so downstream readers
    /// (UI, chronicler observation events) can drill into *which*
    /// category drove the damage. `BTreeMap` so iteration is sorted
    /// by category enum value, never insertion order.
    pub net_per_category: BTreeMap<PrimitiveCategory, Q3232>,
}

/// Resolve a single combat round from primitive sets and formation
/// slots.
///
/// Pure function: no `&mut`, no PRNG, no wall-clock reads. Replaying
/// with the same inputs yields byte-identical [`RoundOutcome`] (locked
/// in by `tests::output_is_bit_identical_for_same_input`).
///
/// # Algorithm
///
/// ```text
/// // Aggregate by category (uses each effect's activation_cost as
/// // its scalar magnitude — a uniform first-class field on
/// // PrimitiveEffect, free of mechanics-label coupling).
/// for each PrimitiveCategory C:
///   offense[C] = Σ activation_cost over attacker effects in C
///   defense[C] = Σ activation_cost over defender effects in C
///   net[C]     = saturating_sub(offense[C], defense[C])
///
/// outbound       = net[ForceApplication] * attacker_slot.engagement
/// damage         = outbound                * defender_slot.exposure
/// mobility_check = net[MassTransfer] > 0
/// stamina_cost   = Σ activation_cost over attacker effects
/// ```
///
/// # Effects lacking a registered manifest
///
/// Skipped — they contribute zero to every aggregate. An effect with
/// no manifest provenance is not a recognised mechanic and cannot
/// drive damage; this matches the emergence-closure rule.
///
/// # Mechanics-label separation
///
/// [`resolve_round`] never reads [`PrimitiveEffect::primitive_id`]
/// directly to make decisions; the id is used solely as a registry
/// key to fetch [`PrimitiveCategory`]. The damage formula above
/// references categories only — renaming every primitive while
/// preserving categories is bit-equivalent.
#[must_use]
pub fn resolve_round(
    registry: &PrimitiveRegistry,
    attacker_effects: &[PrimitiveEffect],
    defender_effects: &[PrimitiveEffect],
    attacker_slot: &FormationSlot,
    defender_slot: &FormationSlot,
) -> RoundOutcome {
    let attacker_offense = aggregate_by_category(registry, attacker_effects);
    let defender_defense = aggregate_by_category(registry, defender_effects);

    let mut net_per_category: BTreeMap<PrimitiveCategory, Q3232> = BTreeMap::new();
    for category in ALL_CATEGORIES {
        let offense = attacker_offense
            .get(&category)
            .copied()
            .unwrap_or(Q3232::ZERO);
        let defense = defender_defense
            .get(&category)
            .copied()
            .unwrap_or(Q3232::ZERO);
        // Q3232::saturating_sub saturates at I32F32::MIN/MAX (signed),
        // not at zero — so a defense > offense leaves a negative net.
        // Floor explicitly at zero per the algorithm doc above; a
        // negative `force_net` would otherwise drive `damage` negative
        // when multiplied by the slot scalars.
        let net = offense.saturating_sub(defense).max(Q3232::ZERO);
        net_per_category.insert(category, net);
    }

    let force_net = net_per_category
        .get(&PrimitiveCategory::ForceApplication)
        .copied()
        .unwrap_or(Q3232::ZERO);
    let outbound = force_net * attacker_slot.engagement;
    let damage = outbound * defender_slot.exposure;

    let mobility_net = net_per_category
        .get(&PrimitiveCategory::MassTransfer)
        .copied()
        .unwrap_or(Q3232::ZERO);
    let mobility_check = mobility_net > Q3232::ZERO;

    // Only registered effects contribute to stamina — keeps the
    // billing gate symmetric with the recognition gate above. An
    // effect without manifest provenance is not a recognised mechanic
    // and must not bill the attacker (otherwise: a resource cost with
    // no emergence-traceable output, INVARIANTS §4).
    let stamina_cost_attacker = attacker_effects
        .iter()
        .filter(|e| registry.contains(&e.primitive_id))
        .fold(Q3232::ZERO, |acc, e| acc + e.activation_cost);

    RoundOutcome {
        damage,
        mobility_check,
        stamina_cost_attacker,
        net_per_category,
    }
}

/// Sum activation costs grouped by primitive category.
///
/// Effects whose `primitive_id` is not registered are skipped —
/// emergence-closure forbids treating an unknown primitive as if it
/// belonged to any category.
fn aggregate_by_category(
    registry: &PrimitiveRegistry,
    effects: &[PrimitiveEffect],
) -> BTreeMap<PrimitiveCategory, Q3232> {
    let mut out: BTreeMap<PrimitiveCategory, Q3232> = BTreeMap::new();
    for effect in effects {
        if let Some(manifest) = registry.get(&effect.primitive_id) {
            *out.entry(manifest.category).or_insert(Q3232::ZERO) += effect.activation_cost;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use beast_channels::ChannelFamily;
    use beast_core::EntityId;
    use beast_ecs::components::FormationSlot;
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

    /// Build a registry mapping fresh ids to a category each. Returns
    /// the registry plus a `Vec<&'static str>` of registered ids in
    /// the same order as `categories`.
    fn registry_with(categories: &[(&'static str, PrimitiveCategory)]) -> PrimitiveRegistry {
        let mut reg = PrimitiveRegistry::new();
        for (id, cat) in categories {
            reg.register(manifest(id, *cat))
                .expect("test fixture should not duplicate ids");
        }
        reg
    }

    fn live_slot(engagement: Q3232, exposure: Q3232) -> FormationSlot {
        // Slot with non-zero engagement / exposure so monotonicity
        // tests don't get masked by a zero-multiplier attenuation.
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

    // --- Pure-function / determinism contract -----------------------------

    #[test]
    fn output_is_bit_identical_for_same_input() {
        let reg = registry_with(&[
            ("force_a", PrimitiveCategory::ForceApplication),
            ("mass_a", PrimitiveCategory::MassTransfer),
        ]);
        let attacker = vec![effect("force_a", q(0.4)), effect("mass_a", q(0.2))];
        let defender = vec![effect("force_a", q(0.1))];
        let a_slot = live_slot(q(0.8), q(0.7));
        let d_slot = live_slot(q(0.5), q(0.6));

        let first = resolve_round(&reg, &attacker, &defender, &a_slot, &d_slot);
        for _ in 0..100 {
            let again = resolve_round(&reg, &attacker, &defender, &a_slot, &d_slot);
            // PartialEq drives the compare; bit-equality is the actual
            // contract — assert via `to_bits` on the damage scalar
            // explicitly so a future op-overload swap can't mask drift.
            assert_eq!(again.damage.to_bits(), first.damage.to_bits());
            assert_eq!(
                again.stamina_cost_attacker.to_bits(),
                first.stamina_cost_attacker.to_bits(),
            );
            assert_eq!(again.mobility_check, first.mobility_check);
            assert_eq!(again.net_per_category, first.net_per_category);
        }
    }

    // --- Damage = 0 when defense covers offense --------------------------

    #[test]
    fn damage_is_zero_when_defense_covers_force_offense() {
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        // Defender's force_application defense >= attacker's offense.
        let attacker = vec![effect("force_a", q(0.5))];
        let defender = vec![effect("force_a", q(0.5))];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &defender, &slot, &slot);
        assert_eq!(outcome.damage, Q3232::ZERO);
    }

    #[test]
    fn damage_is_zero_when_defense_exceeds_offense() {
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        // Defender has more force_application defense than attacker has offense.
        let attacker = vec![effect("force_a", q(0.3))];
        let defender = vec![effect("force_a", q(0.9))];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &defender, &slot, &slot);
        assert_eq!(outcome.damage, Q3232::ZERO);
    }

    #[test]
    fn full_category_coverage_zeros_damage() {
        // Every offensive category has a matching defensive entry in the
        // same category — covers the full surface, damage must be zero.
        let reg = registry_with(&[
            ("att_force", PrimitiveCategory::ForceApplication),
            ("att_state", PrimitiveCategory::StateInduction),
            ("att_mass", PrimitiveCategory::MassTransfer),
            ("def_force", PrimitiveCategory::ForceApplication),
            ("def_state", PrimitiveCategory::StateInduction),
            ("def_mass", PrimitiveCategory::MassTransfer),
        ]);
        let attacker = vec![
            effect("att_force", q(0.4)),
            effect("att_state", q(0.4)),
            effect("att_mass", q(0.4)),
        ];
        let defender = vec![
            effect("def_force", q(0.5)),
            effect("def_state", q(0.5)),
            effect("def_mass", q(0.5)),
        ];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &defender, &slot, &slot);
        assert_eq!(outcome.damage, Q3232::ZERO);
    }

    // --- Damage monotonic in attacker force_application -------------------
    // Property tests (proptest) per #253 DoD — randomised inputs across
    // unit_cost and k. Bounded so the saturating Q3232 product can't hit
    // I32F32::MAX between consecutive k values, which would mask
    // monotonicity violations.

    proptest::proptest! {
        #[test]
        fn zero_defender_damage_strictly_increases_with_attacker_magnitude(
            // Q3232::from_bits — raw bits in [1, ~0.23 in Q3232 form]
            // give plenty of monotonicity headroom: 20 * 1e9 ≈ 2e10
            // bits, well below I32F32::MAX (~9.2e18).
            unit_cost in 1i64..1_000_000_000,
            k in 2usize..=20,
        ) {
            let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
            // Engagement and exposure both = ONE so the slot scalars
            // are identity multipliers — the property here is about
            // the offense aggregation alone.
            let slot = live_slot(Q3232::ONE, Q3232::ONE);
            let damages: Vec<Q3232> = (1..=k)
                .map(|n| {
                    let att: Vec<PrimitiveEffect> = (0..n)
                        .map(|_| effect("force_a", Q3232::from_bits(unit_cost)))
                        .collect();
                    resolve_round(&reg, &att, &[], &slot, &slot).damage
                })
                .collect();
            for w in damages.windows(2) {
                proptest::prop_assert!(
                    w[1] > w[0],
                    "non-strict step: {:?} not > {:?} (unit_cost={unit_cost}, k={k})",
                    w[1],
                    w[0],
                );
            }
        }

        #[test]
        fn fixed_defender_damage_monotone_in_attacker_count(
            unit_cost in 1i64..1_000_000_000,
            // 0 included so the property covers the zero-defender case.
            defender_cost in 0i64..1_000_000_000,
            k in 2usize..=20,
        ) {
            let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
            let defender: Vec<PrimitiveEffect> = if defender_cost == 0 {
                Vec::new()
            } else {
                vec![effect("force_a", Q3232::from_bits(defender_cost))]
            };
            let slot = live_slot(Q3232::ONE, Q3232::ONE);
            let damages: Vec<Q3232> = (1..=k)
                .map(|n| {
                    let att: Vec<PrimitiveEffect> = (0..n)
                        .map(|_| effect("force_a", Q3232::from_bits(unit_cost)))
                        .collect();
                    resolve_round(&reg, &att, &defender, &slot, &slot).damage
                })
                .collect();
            for w in damages.windows(2) {
                proptest::prop_assert!(
                    w[1] >= w[0],
                    "non-monotone step: {:?} not >= {:?} (unit_cost={unit_cost}, defender_cost={defender_cost}, k={k})",
                    w[1],
                    w[0],
                );
            }
        }
    }

    // --- Slot scalars attenuate damage -----------------------------------

    #[test]
    fn zero_engagement_zeros_damage() {
        // Attacker has plenty of force, defender has none — but the
        // attacker's slot has zero engagement, so no damage flows.
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        let attacker = vec![effect("force_a", q(0.9))];
        let a_slot = live_slot(Q3232::ZERO, q(0.8));
        let d_slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &[], &a_slot, &d_slot);
        assert_eq!(outcome.damage, Q3232::ZERO);
    }

    #[test]
    fn zero_exposure_zeros_damage() {
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        let attacker = vec![effect("force_a", q(0.9))];
        let a_slot = live_slot(q(0.8), q(0.8));
        let d_slot = live_slot(q(0.8), Q3232::ZERO);
        let outcome = resolve_round(&reg, &attacker, &[], &a_slot, &d_slot);
        assert_eq!(outcome.damage, Q3232::ZERO);
    }

    // --- Mechanics-label separation (INVARIANTS §2) ----------------------

    #[test]
    fn renaming_primitive_ids_preserves_outcome() {
        // The same fixture under two completely different id schemes —
        // categories preserved, costs preserved, slots preserved. The
        // function must produce byte-identical outcomes; if it did not,
        // some code path would be reading the id string instead of the
        // structural category.
        let reg_a = registry_with(&[
            ("alpha", PrimitiveCategory::ForceApplication),
            ("bravo", PrimitiveCategory::MassTransfer),
        ]);
        let reg_b = registry_with(&[
            ("zulu", PrimitiveCategory::ForceApplication),
            ("yankee", PrimitiveCategory::MassTransfer),
        ]);
        let att_a = vec![effect("alpha", q(0.3)), effect("bravo", q(0.2))];
        let att_b = vec![effect("zulu", q(0.3)), effect("yankee", q(0.2))];
        let slot = live_slot(q(0.8), q(0.7));

        let out_a = resolve_round(&reg_a, &att_a, &[], &slot, &slot);
        let out_b = resolve_round(&reg_b, &att_b, &[], &slot, &slot);

        assert_eq!(out_a.damage.to_bits(), out_b.damage.to_bits());
        assert_eq!(
            out_a.stamina_cost_attacker.to_bits(),
            out_b.stamina_cost_attacker.to_bits(),
        );
        assert_eq!(out_a.mobility_check, out_b.mobility_check);
        assert_eq!(out_a.net_per_category, out_b.net_per_category);
    }

    #[test]
    fn unregistered_primitive_contributes_zero() {
        // Effect with a primitive_id absent from the registry: must
        // contribute zero to *every* aggregate — damage, per-category
        // net, and stamina cost. Symmetry between the recognition
        // gate and the billing gate is the emergence-closure rule
        // (INVARIANTS §4): a resource cost with no emergence-
        // traceable output would be a ghost mechanic.
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        let mystery = effect("not_in_registry", q(0.9));
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, std::slice::from_ref(&mystery), &[], &slot, &slot);
        assert_eq!(outcome.damage, Q3232::ZERO);
        assert_eq!(
            outcome.net_per_category[&PrimitiveCategory::ForceApplication],
            Q3232::ZERO,
        );
        assert_eq!(outcome.stamina_cost_attacker, Q3232::ZERO);
    }

    #[test]
    fn unregistered_primitive_does_not_billing_gate_around_registered() {
        // Mixed batch: one registered effect, one mystery. Registered
        // contributes; mystery does not — including its stamina cost.
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        let attacker = vec![effect("force_a", q(0.2)), effect("not_in_registry", q(0.7))];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &[], &slot, &slot);
        // Stamina = 0.2 (only the registered cost); 0.7 is dropped.
        let diff = (outcome.stamina_cost_attacker - q(0.2)).saturating_abs();
        assert!(diff <= Q3232::from_bits(4));
    }

    // --- Mobility check ---------------------------------------------------

    #[test]
    fn mobility_check_true_when_attacker_dominates_mass_transfer() {
        let reg = registry_with(&[("mass_a", PrimitiveCategory::MassTransfer)]);
        let attacker = vec![effect("mass_a", q(0.4))];
        let defender = vec![effect("mass_a", q(0.1))];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &defender, &slot, &slot);
        assert!(outcome.mobility_check);
    }

    #[test]
    fn mobility_check_false_when_defender_resists() {
        let reg = registry_with(&[("mass_a", PrimitiveCategory::MassTransfer)]);
        let attacker = vec![effect("mass_a", q(0.2))];
        let defender = vec![effect("mass_a", q(0.4))];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &defender, &slot, &slot);
        assert!(!outcome.mobility_check);
    }

    #[test]
    fn mobility_check_false_when_no_mass_transfer_effects() {
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        let attacker = vec![effect("force_a", q(0.5))];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &[], &slot, &slot);
        assert!(!outcome.mobility_check);
    }

    // --- Stamina ---------------------------------------------------------

    #[test]
    fn stamina_cost_sums_attacker_activation_costs() {
        let reg = registry_with(&[
            ("force_a", PrimitiveCategory::ForceApplication),
            ("force_b", PrimitiveCategory::ForceApplication),
        ]);
        let attacker = vec![
            effect("force_a", q(0.1)),
            effect("force_b", q(0.2)),
            effect("force_a", q(0.05)),
        ];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &[], &slot, &slot);
        // 0.1 + 0.2 + 0.05 = 0.35 (within 4-ULP tolerance — saturating
        // Q3232 sums of distinct from_num literals can drift by 1 LSB).
        let diff = (outcome.stamina_cost_attacker - q(0.35)).saturating_abs();
        assert!(diff <= Q3232::from_bits(4));
    }

    // --- Per-category breakdown is correct -------------------------------

    // --- ALL_CATEGORIES sort invariant -----------------------------------

    #[test]
    fn all_categories_is_sorted() {
        // The const claims to match `PrimitiveCategory`'s derived `Ord`
        // sort order. Pin that via comparison with a sorted copy —
        // adding a new variant before `SignalEmission` (or any
        // re-ordering of the enum) silently invalidates the
        // declaration order without a compile error; this test would
        // catch it at the next `cargo test`.
        let mut sorted = ALL_CATEGORIES;
        sorted.sort();
        assert_eq!(ALL_CATEGORIES, sorted);
    }

    #[test]
    fn all_categories_has_no_duplicates() {
        // Eight categories, eight distinct entries.
        let mut sorted = ALL_CATEGORIES;
        sorted.sort();
        let mut deduped = sorted.to_vec();
        deduped.dedup();
        assert_eq!(deduped.len(), ALL_CATEGORIES.len());
    }

    #[test]
    fn net_per_category_includes_all_eight_categories() {
        let reg = registry_with(&[("force_a", PrimitiveCategory::ForceApplication)]);
        let attacker = vec![effect("force_a", q(0.3))];
        let slot = live_slot(q(0.8), q(0.8));
        let outcome = resolve_round(&reg, &attacker, &[], &slot, &slot);
        assert_eq!(outcome.net_per_category.len(), ALL_CATEGORIES.len());
        for cat in &ALL_CATEGORIES {
            assert!(outcome.net_per_category.contains_key(cat));
        }
    }
}
