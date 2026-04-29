//! Keeper components: [`KeeperState`] and the pure
//! [`leadership_presence`] budget function.
//!
//! Backs `06_combat_system.md` §3 ("The Keeper's Leadership Budget"). The
//! Keeper does not own a generic command-point pool; the per-round
//! Leadership Presence is a direct projection of personality channels
//! (`charisma`, `neural_speed`, `empathy`) attenuated by the Keeper's
//! current `stress` and `fatigue`. This module owns the resource layer
//! only — the *decision* layer (which orders to issue) is unified under
//! the active-inference loop in `documentation/emergence/57_agent_ai.md`
//! per INVARIANTS §9, and lands in a later sprint.

use beast_core::{clamp01, Q3232};
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};

/// Per-Keeper personality and psychological state used to derive the
/// per-round Leadership Presence budget.
///
/// All five fields are [`Q3232`] and are expected to lie in the unit
/// interval `[0, 1]`. The two input classes are treated differently by
/// [`leadership_presence`]:
///
/// * `stress` and `fatigue` are clamped to `[0, 1]` on entry so the
///   multipliers stay in the unit interval — out-of-range values
///   produce the same result as the corresponding endpoint.
/// * Personality fields (`charisma`, `neural_speed`, `empathy`) are
///   consumed as-is. Saturating Q32.32 arithmetic prevents panics, but
///   the *output* is not bounded: a Keeper with `charisma = 2.0` will
///   contribute `2.0` to the personality sum, not `1.0`.
///
/// Fields:
///
/// * `charisma`     — persuasiveness / force of will
/// * `neural_speed` — decision-making quickness, reaction time
/// * `empathy`      — attunement to crew morale and needs
/// * `stress`       — accumulated combat stress this session, `[0, 1]`
/// * `fatigue`      — physical exhaustion, `[0, 1]`
///
/// Personality channels (`charisma`, `neural_speed`, `empathy`) are
/// authoritative state set at character creation per System 01;
/// `stress` and `fatigue` are updated by the encounter loop (S13).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeeperState {
    /// Persuasiveness, force of will. Unit interval.
    pub charisma: Q3232,
    /// Decision-making quickness, reaction time. Unit interval.
    pub neural_speed: Q3232,
    /// Attunement to crew morale and needs. Unit interval.
    pub empathy: Q3232,
    /// Accumulated combat stress this session. Unit interval.
    pub stress: Q3232,
    /// Physical exhaustion. Unit interval.
    pub fatigue: Q3232,
}

impl KeeperState {
    /// Convenience constructor.
    #[must_use]
    pub fn new(
        charisma: Q3232,
        neural_speed: Q3232,
        empathy: Q3232,
        stress: Q3232,
        fatigue: Q3232,
    ) -> Self {
        Self {
            charisma,
            neural_speed,
            empathy,
            stress,
            fatigue,
        }
    }
}

impl Component for KeeperState {
    type Storage = DenseVecStorage<Self>;
}

/// Compute the Keeper's per-round Leadership Presence budget.
///
/// Pure function: no `&mut`, no PRNG, no wall-clock reads — replaying a
/// tick with the same `KeeperState` always returns the same result, as
/// required by INVARIANTS §1.
///
/// Formula (combat doc §3, Q32.32 form):
///
/// ```text
///   personality = charisma + neural_speed + empathy
///   stress_mult  = clamp01(1 - stress)
///   fatigue_mult = clamp01(1 - fatigue)
///   presence = personality * stress_mult * fatigue_mult
/// ```
///
/// `stress` and `fatigue` are clamped to `[0, 1]` on entry via
/// [`beast_core::clamp01`] so the multipliers stay in the unit interval
/// — feeding `stress > 1` then flipping the sign of the personality
/// term via wrap-around would silently break monotonicity.
///
/// # Returns
///
/// A value in `[0, charisma + neural_speed + empathy]`. When all three
/// personality channels are in `[0, 1]` the result lies in `[0, 3]`;
/// callers that need a unit-interval budget must scale or clamp the
/// returned value. The output is *not* bounded by `Q3232::ONE` —
/// saturating arithmetic prevents panics on extreme inputs but does
/// not normalise the personality sum.
///
/// Monotonicity (locked in by unit tests):
///
/// * Increasing `stress` never increases the result.
/// * Increasing `fatigue` never increases the result.
/// * `stress = 0, fatigue = 0` produces the per-Keeper maximum
///   (`charisma + neural_speed + empathy`).
/// * `stress = 1` *or* `fatigue = 1` produces [`Q3232::ZERO`].
#[must_use]
pub fn leadership_presence(state: &KeeperState) -> Q3232 {
    let stress = clamp01(state.stress);
    let fatigue = clamp01(state.fatigue);
    let stress_mult = Q3232::ONE - stress;
    let fatigue_mult = Q3232::ONE - fatigue;
    let personality = state.charisma + state.neural_speed + state.empathy;
    personality * stress_mult * fatigue_mult
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    fn personality(c: f64, n: f64, e: f64) -> KeeperState {
        KeeperState::new(q(c), q(n), q(e), Q3232::ZERO, Q3232::ZERO)
    }

    #[test]
    fn zero_stress_zero_fatigue_returns_full_personality() {
        let state = personality(0.5, 0.25, 0.125);
        let expected = q(0.5) + q(0.25) + q(0.125);
        assert_eq!(leadership_presence(&state), expected);
    }

    #[test]
    fn full_stress_floors_to_zero() {
        let mut state = personality(1.0, 1.0, 1.0);
        state.stress = Q3232::ONE;
        assert_eq!(leadership_presence(&state), Q3232::ZERO);
    }

    #[test]
    fn full_fatigue_floors_to_zero() {
        let mut state = personality(1.0, 1.0, 1.0);
        state.fatigue = Q3232::ONE;
        assert_eq!(leadership_presence(&state), Q3232::ZERO);
    }

    #[test]
    fn either_full_floors_to_zero() {
        // Stress at max with non-zero fatigue still floors.
        let state = KeeperState::new(q(0.5), q(0.5), q(0.5), Q3232::ONE, q(0.4));
        assert_eq!(leadership_presence(&state), Q3232::ZERO);
    }

    // The two monotonicity helpers compute step values via
    // `f64::from(step) * 0.1` to drive the sweep. Float arithmetic is
    // forbidden on the sim path (`[lints.clippy] float_arithmetic =
    // "warn"` in `Cargo.toml`, promoted to deny in CI), so we scope
    // the exemption tightly: these two functions only, not the whole
    // tests module — keeping the lint live for any future test
    // helper added here.
    #[test]
    #[allow(clippy::float_arithmetic)]
    fn monotonic_in_stress() {
        let base = personality(0.4, 0.3, 0.2);
        let mut prev = leadership_presence(&base);
        for step in 1..=10 {
            let stress = f64::from(step) * 0.1;
            let mut state = base;
            state.stress = q(stress);
            let curr = leadership_presence(&state);
            assert!(
                curr <= prev,
                "stress={stress:.1} produced {curr:?} > prev {prev:?}",
            );
            prev = curr;
        }
    }

    #[test]
    #[allow(clippy::float_arithmetic)]
    fn monotonic_in_fatigue() {
        let base = personality(0.4, 0.3, 0.2);
        let mut prev = leadership_presence(&base);
        for step in 1..=10 {
            let fatigue = f64::from(step) * 0.1;
            let mut state = base;
            state.fatigue = q(fatigue);
            let curr = leadership_presence(&state);
            assert!(
                curr <= prev,
                "fatigue={fatigue:.1} produced {curr:?} > prev {prev:?}",
            );
            prev = curr;
        }
    }

    #[test]
    fn over_unit_stress_clamps_to_full_stress() {
        // Out-of-band inputs (>1) must not flip the sign of the multiplier
        // via underflow saturation. We expect the same result as stress=1.
        let mut over = personality(0.5, 0.5, 0.5);
        over.stress = q(2.5);
        let mut maxed = personality(0.5, 0.5, 0.5);
        maxed.stress = Q3232::ONE;
        assert_eq!(leadership_presence(&over), leadership_presence(&maxed));
        assert_eq!(leadership_presence(&over), Q3232::ZERO);
    }

    #[test]
    fn negative_stress_clamps_to_zero_stress() {
        let mut neg = personality(0.5, 0.25, 0.125);
        neg.stress = q(-0.5);
        let zero = personality(0.5, 0.25, 0.125);
        assert_eq!(leadership_presence(&neg), leadership_presence(&zero));
    }

    #[test]
    fn negative_fatigue_clamps_to_zero_fatigue() {
        let mut neg = personality(0.5, 0.25, 0.125);
        neg.fatigue = q(-0.5);
        let zero = personality(0.5, 0.25, 0.125);
        assert_eq!(leadership_presence(&neg), leadership_presence(&zero));
    }

    #[test]
    fn property_zero_stress_dominates_max_stress() {
        // The DoD's "for any personality vector" property test: zero-stress
        // is always ≥ max-stress for the same personality.
        for c in [0.0, 0.25, 0.5, 0.75, 1.0] {
            for n in [0.0, 0.25, 0.5, 0.75, 1.0] {
                for e in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let zero_stressed = personality(c, n, e);
                    let mut max_stressed = zero_stressed;
                    max_stressed.stress = Q3232::ONE;
                    assert!(
                        leadership_presence(&zero_stressed) >= leadership_presence(&max_stressed),
                    );
                }
            }
        }
    }

    #[test]
    fn pure_function_is_deterministic() {
        // Same input → same output across many calls. Guards against any
        // future refactor that accidentally reads from a thread-local PRNG
        // or wall-clock source.
        let state = personality(0.42, 0.31, 0.27);
        let first = leadership_presence(&state);
        for _ in 0..1000 {
            assert_eq!(leadership_presence(&state), first);
        }
    }

    #[test]
    fn keeper_state_storage_is_densevec() {
        fn is_dense<C>()
        where
            C: specs::Component<Storage = specs::DenseVecStorage<C>>,
        {
        }
        is_dense::<KeeperState>();
    }
}
