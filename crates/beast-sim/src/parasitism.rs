//! Parasitism — sustained host coupling between attacker and defender.
//!
//! Backs `documentation/systems/06_combat_system.md` §3.4 ("host
//! coupling profile") and `documentation/systems/16_disease_parasitism.md`.
//!
//! Where [`crate::predation::resolve_predation`] is a one-shot
//! consumption, parasitism is a *channel projection* between the
//! parasite and the host that persists across rounds. The
//! [`HostCoupling`] resource carries the projection magnitudes per
//! channel; per-tick decay shrinks them and per-tick draw subtracts
//! from the host's metabolic state.
//!
//! # Scale-band unification (INVARIANTS §5)
//!
//! The parasitism path is the same code path as predation — both
//! aggregate the same primitive set produced by `interpret_phenotype`.
//! Parasites are micro-scale creatures whose phenotype expresses
//! micro-band channels (the scale-band filter at the interpreter
//! handles the cohort split — already implemented in S4); their
//! [`PrimitiveEffect`] sets are structurally identical to macro
//! creatures'. No branching here on attacker / defender mass.
//!
//! # Stacking (DoD)
//!
//! Two parasites on one host stack via additive Q32.32 summation in
//! `projection`. Two `HostCoupling` records — one per (host, parasite)
//! pair — coexist; per-tick draw aggregates across all couplings for
//! a given host via [`aggregate_projection_for_host`].

use std::collections::BTreeMap;

use beast_core::{EntityId, Q3232};
use beast_primitives::PrimitiveEffect;
use serde::{Deserialize, Serialize};

/// Persistent channel projection from a parasite onto a host.
///
/// One record per (host, parasite) pair. Identity ordering is by the
/// `(host, parasite)` tuple — both are `EntityId` (transparent `u32`),
/// which derives `Ord` consistently across runs, so a
/// `BTreeMap<(EntityId, EntityId), HostCoupling>` keyed by `(host,
/// parasite)` iterates in a stable, replay-safe order (INVARIANTS §1).
///
/// `installed_tick` records the tick at which the coupling was
/// installed — useful for chronicler observation events that report
/// "this parasitism has been active for N ticks". Decay does not
/// touch this field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCoupling {
    /// Entity carrying the coupling.
    pub host: EntityId,
    /// Entity inducing the coupling.
    pub parasite: EntityId,
    /// Channel-id → projected magnitude. Keyed by channel id (string)
    /// so the projection lines up with the host's channel state when
    /// the per-tick draw subtracts from `HealthState::energy` etc.
    /// `BTreeMap` for sorted-iteration determinism.
    pub projection: BTreeMap<String, Q3232>,
    /// Tick at which the coupling was installed (in `TickCounter::raw()`
    /// form — a `u64`).
    pub installed_tick: u64,
}

impl HostCoupling {
    /// Convenience constructor: empty projection, ready to be populated
    /// by [`install_host_coupling`] or set up by hand in tests.
    #[must_use]
    pub fn new(host: EntityId, parasite: EntityId, installed_tick: u64) -> Self {
        Self {
            host,
            parasite,
            projection: BTreeMap::new(),
            installed_tick,
        }
    }

    /// Comparison key used for deterministic ordering when storing in
    /// a collection. `(host, parasite)` tuple — both transparent `u32`
    /// newtypes, so the derived `Ord` is stable across runs.
    #[must_use]
    pub fn key(&self) -> (EntityId, EntityId) {
        (self.host, self.parasite)
    }
}

/// Build a [`HostCoupling`] from the parasite's [`PrimitiveEffect`]
/// set.
///
/// Pure function: same `(host, parasite, parasite_effects, installed_tick)`
/// → byte-identical [`HostCoupling`].
///
/// # Algorithm
///
/// For every effect, every channel listed in `source_channels` gets
/// `+= activation_cost` in the projection map (saturating Q3232
/// addition). The `source_channels` ordering contract on
/// [`PrimitiveEffect`] (lexicographic ascending) keeps the iteration
/// order stable, but `BTreeMap::entry` is order-independent for
/// summation anyway.
///
/// # Mechanics-label separation (INVARIANTS §2)
///
/// `install_host_coupling` does not branch on `primitive_id` strings.
/// It reads only `source_channels` (already structural metadata) and
/// `activation_cost` (a uniform first-class field). The projection is
/// keyed by *channel* id, not primitive id — channels are the
/// substrate; primitives are the events that trigger them. Renaming
/// every primitive id while preserving its source_channels yields a
/// byte-identical projection.
#[must_use]
pub fn install_host_coupling(
    host: EntityId,
    parasite: EntityId,
    parasite_effects: &[PrimitiveEffect],
    installed_tick: u64,
) -> HostCoupling {
    let mut coupling = HostCoupling::new(host, parasite, installed_tick);
    for effect in parasite_effects {
        for channel_id in &effect.source_channels {
            *coupling
                .projection
                .entry(channel_id.clone())
                .or_insert(Q3232::ZERO) += effect.activation_cost;
        }
    }
    coupling
}

/// Decay every projection magnitude by multiplying by `decay_factor`.
///
/// `decay_factor` is expected to be in `[0, 1]` — `0.99` for a slow
/// burn, `0.5` for a half-life-per-tick cull. Saturating Q3232
/// multiplication; values cannot wrap. Entries that decay to or
/// below `Q3232::ZERO` (i.e. negative `decay_factor`) are clamped at
/// zero.
///
/// Stage 5 (Physiology) of the tick loop is the natural caller — the
/// per-tick decay is a metabolic process from the host's
/// perspective.
pub fn decay_host_coupling(coupling: &mut HostCoupling, decay_factor: Q3232) {
    for v in coupling.projection.values_mut() {
        *v = (*v * decay_factor).max(Q3232::ZERO);
    }
}

/// Drop projection entries whose magnitude has fallen at or below
/// `epsilon`. Keeps the map from accumulating dust over thousands of
/// ticks of decay.
pub fn prune_host_coupling(coupling: &mut HostCoupling, epsilon: Q3232) {
    coupling.projection.retain(|_, v| *v > epsilon);
}

/// Aggregate every coupling targeting `target_host` into a single
/// channel-keyed projection.
///
/// Two parasites on one host stack via additive Q32.32 summation —
/// the DoD's "two parasites on one host stack" requirement. Returns
/// an empty `BTreeMap` if no coupling matches.
///
/// Iterates `couplings` in the order the caller provides; if the
/// caller hands in a `BTreeMap<(EntityId, EntityId), HostCoupling>`
/// via `.values()`, iteration is sorted by `(host, parasite)` and
/// the aggregate is therefore order-stable.
#[must_use]
pub fn aggregate_projection_for_host<'a>(
    couplings: impl IntoIterator<Item = &'a HostCoupling>,
    target_host: EntityId,
) -> BTreeMap<String, Q3232> {
    let mut out: BTreeMap<String, Q3232> = BTreeMap::new();
    for coupling in couplings {
        if coupling.host != target_host {
            continue;
        }
        for (channel_id, magnitude) in &coupling.projection {
            *out.entry(channel_id.clone()).or_insert(Q3232::ZERO) += *magnitude;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use beast_primitives::Provenance;

    fn effect(activation_cost: Q3232, source_channels: &[&str]) -> PrimitiveEffect {
        PrimitiveEffect {
            primitive_id: "parasite_emit".into(),
            body_site: None,
            source_channels: source_channels.iter().map(|s| s.to_string()).collect(),
            parameters: BTreeMap::new(),
            activation_cost,
            emitter: EntityId::new(0),
            provenance: Provenance::Core,
        }
    }

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    fn host() -> EntityId {
        EntityId::new(100)
    }
    fn parasite_a() -> EntityId {
        EntityId::new(200)
    }
    fn parasite_b() -> EntityId {
        EntityId::new(201)
    }

    // --- install_host_coupling -------------------------------------------

    #[test]
    fn install_aggregates_channels_by_activation_cost() {
        let effects = vec![
            effect(q(0.2), &["chemical_output", "thermal_output"]),
            effect(q(0.3), &["chemical_output"]),
        ];
        let coupling = install_host_coupling(host(), parasite_a(), &effects, /* tick */ 42);

        assert_eq!(coupling.host, host());
        assert_eq!(coupling.parasite, parasite_a());
        assert_eq!(coupling.installed_tick, 42);
        // chemical_output appeared in both effects: 0.2 + 0.3 = 0.5
        let chem = coupling.projection.get("chemical_output").copied().unwrap();
        let chem_diff = (chem - q(0.5)).saturating_abs();
        assert!(chem_diff <= Q3232::from_bits(4));
        // thermal_output appeared only in the first effect: 0.2 (exact)
        assert_eq!(
            coupling.projection.get("thermal_output").copied().unwrap(),
            q(0.2),
        );
    }

    #[test]
    fn install_with_empty_effects_yields_empty_projection() {
        let coupling = install_host_coupling(host(), parasite_a(), &[], 0);
        assert!(coupling.projection.is_empty());
    }

    #[test]
    fn install_is_deterministic() {
        let effects = vec![effect(q(0.4), &["chemical_output"])];
        let a = install_host_coupling(host(), parasite_a(), &effects, 7);
        let b = install_host_coupling(host(), parasite_a(), &effects, 7);
        assert_eq!(a, b);
    }

    // --- decay -----------------------------------------------------------

    #[test]
    fn decay_reduces_projection_magnitudes_uniformly() {
        let mut coupling = install_host_coupling(
            host(),
            parasite_a(),
            &[effect(q(0.8), &["chemical_output"])],
            0,
        );
        let initial = coupling.projection["chemical_output"];
        decay_host_coupling(&mut coupling, q(0.5));
        let after = coupling.projection["chemical_output"];
        // Half-life decay: post ≈ initial * 0.5
        let expected = initial * q(0.5);
        let diff = (after - expected).saturating_abs();
        assert!(diff <= Q3232::from_bits(4));
    }

    #[test]
    fn decay_with_factor_zero_zeros_all_entries() {
        let mut coupling = install_host_coupling(
            host(),
            parasite_a(),
            &[
                effect(q(0.5), &["chemical_output"]),
                effect(q(0.5), &["thermal_output"]),
            ],
            0,
        );
        decay_host_coupling(&mut coupling, Q3232::ZERO);
        for v in coupling.projection.values() {
            assert_eq!(*v, Q3232::ZERO);
        }
    }

    #[test]
    fn decay_without_input_is_stable() {
        // Decay reduces values but does not stabilise without further
        // input. Repeated decay with a 0.99 factor over many ticks
        // converges towards zero. Pin that monotonicity property.
        let mut coupling = install_host_coupling(
            host(),
            parasite_a(),
            &[effect(q(0.8), &["chemical_output"])],
            0,
        );
        let mut prev = coupling.projection["chemical_output"];
        for _ in 0..100 {
            decay_host_coupling(&mut coupling, q(0.99));
            let curr = coupling.projection["chemical_output"];
            assert!(curr <= prev, "non-monotone decay: {curr:?} > {prev:?}");
            prev = curr;
        }
    }

    // --- prune -----------------------------------------------------------

    #[test]
    fn prune_removes_below_epsilon() {
        // Build a coupling with one tiny entry (below epsilon) and one
        // large entry (above). Prune drops the tiny one.
        let mut coupling = HostCoupling::new(host(), parasite_a(), 0);
        coupling.projection.insert("big".into(), q(0.9));
        // 2 LSBs at Q3232 ≈ 4.66e-10 — way below the 1e-6 cutoff.
        coupling
            .projection
            .insert("dust".into(), Q3232::from_bits(2));
        prune_host_coupling(&mut coupling, q(0.000_001));
        assert!(coupling.projection.contains_key("big"));
        assert!(!coupling.projection.contains_key("dust"));
    }

    #[test]
    fn prune_with_high_epsilon_clears_all() {
        let mut coupling = install_host_coupling(
            host(),
            parasite_a(),
            &[effect(q(0.5), &["chemical_output"])],
            0,
        );
        prune_host_coupling(&mut coupling, q(1.0));
        assert!(coupling.projection.is_empty());
    }

    // --- aggregate (two parasites on one host) ---------------------------

    #[test]
    fn two_parasites_on_one_host_stack_additively() {
        // The DoD case. Two distinct parasite couplings target the
        // same host with overlapping channels; aggregation sums.
        let coupling_a = install_host_coupling(
            host(),
            parasite_a(),
            &[effect(q(0.3), &["chemical_output"])],
            0,
        );
        let coupling_b = install_host_coupling(
            host(),
            parasite_b(),
            &[effect(q(0.4), &["chemical_output"])],
            0,
        );
        let agg = aggregate_projection_for_host([&coupling_a, &coupling_b], host());
        // 0.3 + 0.4 = 0.7
        let chem = agg["chemical_output"];
        let diff = (chem - q(0.7)).saturating_abs();
        assert!(diff <= Q3232::from_bits(4));
    }

    #[test]
    fn aggregate_filters_by_target_host() {
        let other_host = EntityId::new(999);
        let coupling_a = install_host_coupling(
            host(),
            parasite_a(),
            &[effect(q(0.5), &["chemical_output"])],
            0,
        );
        let coupling_b = install_host_coupling(
            other_host,
            parasite_b(),
            &[effect(q(0.5), &["chemical_output"])],
            0,
        );
        // Only coupling_a should contribute.
        let agg = aggregate_projection_for_host([&coupling_a, &coupling_b], host());
        assert_eq!(agg["chemical_output"], q(0.5));
    }

    #[test]
    fn aggregate_empty_when_no_match() {
        let coupling = install_host_coupling(
            EntityId::new(999),
            parasite_a(),
            &[effect(q(0.5), &["chemical_output"])],
            0,
        );
        let agg = aggregate_projection_for_host([&coupling], host());
        assert!(agg.is_empty());
    }

    #[test]
    fn aggregate_iteration_order_is_sorted_by_channel_id() {
        // The output BTreeMap iterates in sorted key order. This pins
        // the determinism contract for downstream consumers (e.g. the
        // per-tick draw computation).
        let coupling = install_host_coupling(
            host(),
            parasite_a(),
            &[effect(q(0.1), &["zeta", "alpha", "mu"])],
            0,
        );
        let agg = aggregate_projection_for_host([&coupling], host());
        let keys: Vec<&str> = agg.keys().map(|s| s.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
    }

    // --- ordering key for storage in a BTreeMap --------------------------

    #[test]
    fn coupling_key_is_host_then_parasite() {
        let c = HostCoupling::new(EntityId::new(7), EntityId::new(13), 0);
        assert_eq!(c.key(), (EntityId::new(7), EntityId::new(13)));
    }
}
