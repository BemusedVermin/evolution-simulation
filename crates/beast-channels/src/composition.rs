//! Composition hook evaluation.
//!
//! A channel's manifest declares an array of composition hooks describing how
//! its own contribution combines with other channels. The evaluator defined
//! here runs deterministically over [`beast_core::Q3232`] channel values so
//! downstream interpreters can rely on bit-identical results under replay.
//!
//! The shapes mirror the schema 1:1:
//!
//! | Kind            | Formula (per hook)                                     |
//! |-----------------|--------------------------------------------------------|
//! | `additive`      | `out += coefficient * other`                           |
//! | `multiplicative`| `out *= 1 + coefficient * other`                       |
//! | `threshold`     | `out *= coefficient` when `other >= threshold`, else 0 |
//! | `gating`        | binary: active if `other >= threshold`, else inactive  |
//! | `antagonistic`  | `out -= coefficient * other`                           |
//!
//! The `threshold` kind is multiplicative-with-gate: below threshold the hook
//! contributes nothing; above, it contributes `coefficient * other`. The
//! `gating` kind is a pure boolean — callers typically AND gate outcomes
//! across hooks to decide whether the owning channel expresses at all.

use beast_core::Q3232;
use serde::{Deserialize, Serialize};

/// Composition kinds supported by the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionKind {
    /// Sum contributions: `result += coefficient * other_channel`.
    Additive,
    /// Product contributions: `result *= (1 + coefficient * other_channel)`.
    Multiplicative,
    /// Activation gate: contribute `coefficient * other` only when
    /// `other >= threshold`.
    Threshold,
    /// Binary switch: the hook is *active* when `other >= threshold`.
    Gating,
    /// Subtract contributions: `result -= coefficient * other_channel`.
    Antagonistic,
}

/// A single composition hook parsed from a channel manifest.
///
/// The `threshold` field is `Some` iff `kind ∈ {Threshold, Gating}` (enforced
/// at load time by [`crate::manifest::ChannelManifest::from_json_str`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionHook {
    /// Other channel id, or the literal `"self"` for an auto-interaction.
    pub with: String,
    /// Composition kind.
    pub kind: CompositionKind,
    /// Scaling coefficient.
    pub coefficient: Q3232,
    /// Activation threshold — required iff `kind ∈ {Threshold, Gating}`.
    pub threshold: Option<Q3232>,
}

/// Outcome of evaluating a single hook against a specific `other_channel` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HookOutcome {
    /// Additive contribution to the running total.
    pub delta: Q3232,
    /// Multiplicative factor to apply to the running total
    /// (`Q3232::ONE` when the hook doesn't contribute multiplicatively).
    pub factor: Q3232,
    /// Whether the hook's gating condition is satisfied.
    ///
    /// For `Additive`, `Multiplicative`, and `Antagonistic` hooks this is
    /// always `true`. For `Threshold` and `Gating` hooks it mirrors the
    /// threshold test.
    pub gate_open: bool,
}

impl HookOutcome {
    /// A neutral outcome: zero delta, unit factor, gate open.
    pub const NEUTRAL: Self = Self {
        delta: Q3232::ZERO,
        factor: Q3232::ONE,
        gate_open: true,
    };
}

/// Evaluate a single composition hook against the current value of the
/// referenced channel (`other`).
///
/// The returned [`HookOutcome`] is self-describing — callers compose hooks by
/// accumulating `delta`, multiplying `factor`, and ANDing `gate_open` across
/// a channel's hooks.
///
/// ```
/// use beast_channels::{evaluate_hook, CompositionHook, CompositionKind, HookOutcome};
/// use beast_core::Q3232;
///
/// let hook = CompositionHook {
///     with: "spatial_cognition".into(),
///     kind: CompositionKind::Threshold,
///     coefficient: Q3232::ONE,
///     threshold: Some(Q3232::from_num(0.5_f64)),
/// };
///
/// // Below threshold: gate closed, zero contribution.
/// let below = evaluate_hook(&hook, Q3232::from_num(0.2_f64));
/// assert_eq!(below.gate_open, false);
/// assert_eq!(below.delta, Q3232::ZERO);
///
/// // Above threshold: gate open, additive contribution applied.
/// let above = evaluate_hook(&hook, Q3232::from_num(0.8_f64));
/// assert_eq!(above.gate_open, true);
/// assert_eq!(above.delta, Q3232::from_num(0.8_f64));
/// ```
pub fn evaluate_hook(hook: &CompositionHook, other: Q3232) -> HookOutcome {
    match hook.kind {
        CompositionKind::Additive => HookOutcome {
            delta: hook.coefficient * other,
            factor: Q3232::ONE,
            gate_open: true,
        },
        CompositionKind::Multiplicative => HookOutcome {
            delta: Q3232::ZERO,
            factor: Q3232::ONE + hook.coefficient * other,
            gate_open: true,
        },
        CompositionKind::Antagonistic => HookOutcome {
            delta: -(hook.coefficient * other),
            factor: Q3232::ONE,
            gate_open: true,
        },
        CompositionKind::Threshold => evaluate_threshold(hook.coefficient, other, hook.threshold),
        CompositionKind::Gating => evaluate_gating(other, hook.threshold),
    }
}

/// Closed-gate fallback used by Threshold/Gating when `threshold: None`
/// slips past load-time validation.
///
/// The schema-validated load path always sets `threshold` for
/// Threshold/Gating kinds, but `CompositionHook` has public fields —
/// external callers (mods, integration tests, fuzzers) can construct
/// `threshold: None`. Fail-safe behaviour is **closed gate, zero
/// contribution**: callers AND-gate `gate_open` across hooks to decide
/// channel expression, so a missing-threshold hook must not silently
/// enable expression (`HookOutcome::NEUTRAL`'s `gate_open: true` would
/// do exactly that — opposite of fail-safe).
const CLOSED_GATE: HookOutcome = HookOutcome {
    delta: Q3232::ZERO,
    factor: Q3232::ONE,
    gate_open: false,
};

fn evaluate_threshold(coefficient: Q3232, other: Q3232, threshold: Option<Q3232>) -> HookOutcome {
    let Some(t) = threshold else {
        debug_assert!(
            false,
            "CompositionHook {{ kind: Threshold, threshold: None }} — \
             load-time validation should have rejected this. \
             Falling back to closed gate in release.",
        );
        return CLOSED_GATE;
    };
    if other >= t {
        HookOutcome {
            delta: coefficient * other,
            factor: Q3232::ONE,
            gate_open: true,
        }
    } else {
        CLOSED_GATE
    }
}

fn evaluate_gating(other: Q3232, threshold: Option<Q3232>) -> HookOutcome {
    let Some(t) = threshold else {
        debug_assert!(
            false,
            "CompositionHook {{ kind: Gating, threshold: None }} — \
             load-time validation should have rejected this. \
             Falling back to closed gate in release.",
        );
        return CLOSED_GATE;
    };
    HookOutcome {
        delta: Q3232::ZERO,
        factor: Q3232::ONE,
        gate_open: other >= t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    fn hook(kind: CompositionKind, coefficient: f64, threshold: Option<f64>) -> CompositionHook {
        CompositionHook {
            with: "other".into(),
            kind,
            coefficient: q(coefficient),
            threshold: threshold.map(q),
        }
    }

    #[test]
    fn additive_produces_expected_delta() {
        let out = evaluate_hook(&hook(CompositionKind::Additive, 0.5, None), q(2.0));
        assert_eq!(out.delta, q(1.0));
        assert_eq!(out.factor, Q3232::ONE);
        assert!(out.gate_open);
    }

    #[test]
    fn multiplicative_produces_expected_factor() {
        let out = evaluate_hook(&hook(CompositionKind::Multiplicative, 0.5, None), q(2.0));
        // 1 + 0.5 * 2 = 2
        assert_eq!(out.factor, q(2.0));
        assert_eq!(out.delta, Q3232::ZERO);
    }

    #[test]
    fn antagonistic_subtracts() {
        let out = evaluate_hook(&hook(CompositionKind::Antagonistic, 0.25, None), q(4.0));
        assert_eq!(out.delta, -q(1.0));
    }

    #[test]
    fn threshold_below_keeps_gate_closed() {
        let out = evaluate_hook(&hook(CompositionKind::Threshold, 1.0, Some(0.5)), q(0.25));
        assert!(!out.gate_open);
        assert_eq!(out.delta, Q3232::ZERO);
    }

    #[test]
    fn threshold_above_activates_with_delta() {
        let out = evaluate_hook(&hook(CompositionKind::Threshold, 1.0, Some(0.5)), q(0.75));
        assert!(out.gate_open);
        assert_eq!(out.delta, q(0.75));
    }

    #[test]
    fn threshold_at_boundary_activates() {
        let out = evaluate_hook(&hook(CompositionKind::Threshold, 1.0, Some(0.5)), q(0.5));
        assert!(out.gate_open);
    }

    #[test]
    fn gating_binary_switch() {
        let closed = evaluate_hook(&hook(CompositionKind::Gating, 1.0, Some(0.5)), q(0.25));
        let open = evaluate_hook(&hook(CompositionKind::Gating, 1.0, Some(0.5)), q(0.75));
        assert!(!closed.gate_open);
        assert!(open.gate_open);
        assert_eq!(open.delta, Q3232::ZERO);
        assert_eq!(open.factor, Q3232::ONE);
    }

    #[test]
    fn evaluation_is_deterministic_across_calls() {
        let h = hook(CompositionKind::Multiplicative, 0.125, None);
        for _ in 0..100 {
            assert_eq!(evaluate_hook(&h, q(0.5)).factor, q(1.0625));
        }
    }

    // Pre-audit behaviour: a hand-constructed Threshold/Gating hook
    // with `threshold: None` panicked via `unreachable!()`. The audit
    // (PR #168, see also #36) flagged this as a panic-on-public-input
    // hazard. New contract: debug builds panic via `debug_assert!`,
    // release builds degrade to **closed-gate, zero-contribution**.
    //
    // The two contracts are tested separately by cfg-gating: the
    // `should_panic` debug tests run under `cargo test` (default
    // debug profile); the closed-gate release tests run under
    // `cargo test --release`. CI must run both — neither half
    // proves the other.

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "load-time validation should have rejected this")]
    fn threshold_without_threshold_value_panics_in_debug() {
        let invalid = hook(CompositionKind::Threshold, 1.0, None);
        let _ = evaluate_hook(&invalid, q(0.5));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "load-time validation should have rejected this")]
    fn gating_without_threshold_value_panics_in_debug() {
        let invalid = hook(CompositionKind::Gating, 1.0, None);
        let _ = evaluate_hook(&invalid, q(0.5));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn threshold_without_threshold_value_closes_gate_in_release() {
        let invalid = hook(CompositionKind::Threshold, 1.0, None);
        let outcome = evaluate_hook(&invalid, q(0.5));
        assert_eq!(outcome.delta, Q3232::ZERO);
        assert_eq!(outcome.factor, Q3232::ONE);
        assert!(
            !outcome.gate_open,
            "missing-threshold fallback must close the gate (fail-safe)"
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn gating_without_threshold_value_closes_gate_in_release() {
        let invalid = hook(CompositionKind::Gating, 1.0, None);
        let outcome = evaluate_hook(&invalid, q(0.5));
        assert_eq!(outcome.delta, Q3232::ZERO);
        assert_eq!(outcome.factor, Q3232::ONE);
        assert!(
            !outcome.gate_open,
            "missing-threshold fallback must close the gate (fail-safe)"
        );
    }
}
