//! Composition hook resolver (S4.3 — issue #57).
//!
//! Defines the interpreter-level [`InterpreterHook`] type (wraps the
//! channel-manifest hook and adds the `emits` list) and the [`FiredHook`]
//! output consumed by [`crate::emission`].
//!
//! The resolver itself — evaluating hooks against channel values and producing
//! `Vec<FiredHook>` — is implemented by story 4.3. Only the shared data types
//! are defined here during the Wave 0 scaffold so stories 4.3 and 4.4 share a
//! stable surface.
//!
//! See `documentation/systems/11_phenotype_interpreter.md` §5.0a and §6.2.

use beast_core::Q3232;

pub use beast_channels::composition::CompositionKind;

use crate::parameter_map::Expr;
use crate::phenotype::ResolvedPhenotype;

/// Stable identifier for an interpreter-level hook. Assigned at load time.
///
/// The value is opaque; callers must not rely on its internal layout beyond
/// ordering for determinism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookId(pub u32);

/// A single primitive emission attached to a hook: which primitive to fire and
/// how its parameters are derived from channel values.
///
/// `parameter_mapping` is stored in sorted-key order so iteration is
/// deterministic. The [`Expr`] values were parsed at manifest load time.
#[derive(Debug, Clone)]
pub struct EmitSpec {
    /// Primitive id in the [`beast_primitives::PrimitiveRegistry`].
    pub primitive_id: String,
    /// Parameter name → parsed expression. Sorted by parameter name.
    pub parameter_mapping: Vec<(String, Expr)>,
}

/// Interpreter-level composition hook.
///
/// Wraps the channel-manifest hook data (kind, thresholds, coefficient, gating
/// conditions) and extends it with the [`emits`](Self::emits) list, which is
/// the only path by which the interpreter produces
/// [`beast_primitives::PrimitiveEffect`]s. Per §6.2 and invariant §2
/// (mechanics-label separation), the `emits` list references primitives by id
/// only — never by name.
#[derive(Debug, Clone)]
pub struct InterpreterHook {
    /// Stable hook id.
    pub id: HookId,
    /// Composition kind.
    pub kind: CompositionKind,
    /// Participating channel ids (sorted by caller convention).
    pub channel_ids: Vec<String>,
    /// Per-channel thresholds for [`CompositionKind::Threshold`] /
    /// [`CompositionKind::Gating`]. Indexed parallel to `channel_ids`.
    pub thresholds: Vec<Q3232>,
    /// Scaling coefficient.
    pub coefficient: Q3232,
    /// Environmental gating. Empty vec = always-on.
    pub expression_conditions: Vec<beast_channels::ExpressionCondition>,
    /// Primitives to fire when this hook triggers.
    pub emits: Vec<EmitSpec>,
}

/// Output of the hook resolver — one per hook that fired.
///
/// Downstream [`crate::emission`] consumes the [`emits`](Self::emits) list
/// (borrowed back from the source hook) to build
/// [`beast_primitives::PrimitiveEffect`] values.
#[derive(Debug, Clone)]
pub struct FiredHook {
    /// The hook that fired.
    pub hook_id: HookId,
    /// Kind (preserved for emission-time decisions such as
    /// Additive/Multiplicative intensity computation).
    pub kind: CompositionKind,
    /// Channel values at fire time, parallel to the hook's `channel_ids`.
    pub channel_values: Vec<Q3232>,
    /// Coefficient (copied for emission convenience).
    pub coefficient: Q3232,
    /// Emission specs to fire (cloned from the source hook).
    pub emits: Vec<EmitSpec>,
}

/// Resolve which [`InterpreterHook`]s fire against a [`ResolvedPhenotype`] and
/// return one [`FiredHook`] per firing hook.
///
/// This implements Stage 2 of the phenotype interpreter per
/// `documentation/systems/11_phenotype_interpreter.md` §5.0a and §6.2. The
/// caller is expected to have already filtered `hooks` by expression
/// conditions (Stage 2A affordance filter) — the resolver does **not**
/// re-evaluate `expression_conditions`. The iteration order of `hooks` is
/// preserved exactly in the returned `Vec<FiredHook>`.
///
/// # Firing semantics by [`CompositionKind`]
///
/// * [`CompositionKind::Threshold`] — fires iff every channel value is
///   strictly greater than [`Q3232::ZERO`] **and** `>=` its matching entry in
///   [`InterpreterHook::thresholds`]. The explicit dormant-channel rule (a
///   channel value of `Q3232::ZERO` auto-fails the gate, even when its
///   threshold is also zero) comes from §6.2.
/// * [`CompositionKind::Gating`] — same gating rule as `Threshold`. The
///   single-channel case is the common one; the multi-channel case is an AND.
/// * [`CompositionKind::Additive`] and [`CompositionKind::Multiplicative`] —
///   always fire once the caller has admitted them; the [`coefficient`] is
///   consumed downstream at emission time.
/// * [`CompositionKind::Antagonistic`] — fires iff
///   `|channel_values[0] - channel_values[1]| >= coefficient`. Malformed hooks
///   with fewer than two channels are silently skipped (validation is the
///   loader's responsibility).
///
/// # Unknown channels (lazy genesis)
///
/// If any entry in [`InterpreterHook::channel_ids`] is absent from
/// [`ResolvedPhenotype::global_channels`] the hook is silently skipped. This
/// mirrors the "genesis-dependent hooks are deferred until their channels
/// exist" rule from §5.0a; it is **not** an error.
///
/// # Determinism
///
/// The function is pure: no interior mutability, no RNG, no global state.
/// Iteration order is:
///
/// 1. over `hooks` in input order,
/// 2. over `hook.channel_ids` in the caller-supplied order (which the
///    manifest loader sorts lexicographically).
///
/// No `HashMap`/`HashSet` iteration occurs, satisfying the determinism
/// invariant (INVARIANTS §1).
#[must_use]
pub fn resolve_hooks(phenotype: &ResolvedPhenotype, hooks: &[InterpreterHook]) -> Vec<FiredHook> {
    let mut fired = Vec::with_capacity(hooks.len());
    for hook in hooks {
        if let Some(channel_values) = collect_channel_values(phenotype, hook) {
            if should_fire(hook, &channel_values) {
                fired.push(FiredHook {
                    hook_id: hook.id,
                    kind: hook.kind,
                    channel_values,
                    coefficient: hook.coefficient,
                    emits: hook.emits.clone(),
                });
            }
        }
    }
    fired
}

/// Look up each channel id in the phenotype. Returns `None` if any channel is
/// missing (genesis-dependent hook deferred; see [`resolve_hooks`] docs).
fn collect_channel_values(
    phenotype: &ResolvedPhenotype,
    hook: &InterpreterHook,
) -> Option<Vec<Q3232>> {
    let mut values = Vec::with_capacity(hook.channel_ids.len());
    for channel_id in &hook.channel_ids {
        let value = phenotype.global_channels.get(channel_id)?;
        values.push(*value);
    }
    Some(values)
}

/// Apply the kind-specific firing rule.
fn should_fire(hook: &InterpreterHook, channel_values: &[Q3232]) -> bool {
    match hook.kind {
        CompositionKind::Additive | CompositionKind::Multiplicative => true,
        CompositionKind::Threshold | CompositionKind::Gating => {
            threshold_met(channel_values, &hook.thresholds)
        }
        CompositionKind::Antagonistic => antagonistic_met(channel_values, hook.coefficient),
    }
}

/// Threshold / Gating rule: every channel must be strictly positive AND meet
/// its threshold. Length mismatch between `channel_values` and `thresholds`
/// indicates a malformed manifest — fail closed (do not fire).
fn threshold_met(channel_values: &[Q3232], thresholds: &[Q3232]) -> bool {
    if channel_values.len() != thresholds.len() {
        return false;
    }
    for (value, threshold) in channel_values.iter().zip(thresholds.iter()) {
        // Dormant channels auto-fail (§6.2: "If any operand is zero, this
        // threshold automatically fails").
        if *value <= Q3232::ZERO {
            return false;
        }
        if *value < *threshold {
            return false;
        }
    }
    true
}

/// Antagonistic rule: two opposing channels disagree enough. Malformed
/// (< 2 channels) hooks fail closed.
fn antagonistic_met(channel_values: &[Q3232], coefficient: Q3232) -> bool {
    if channel_values.len() < 2 {
        return false;
    }
    let diff = channel_values[0].saturating_sub(channel_values[1]);
    diff.saturating_abs() >= coefficient
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phenotype::{LifeStage, ResolvedPhenotype};
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    /// Build a `ResolvedPhenotype` with the given global channels. Body map
    /// and environment are left empty — the resolver does not consult them.
    fn phenotype_with(channels: &[(&str, Q3232)]) -> ResolvedPhenotype {
        let mut p = ResolvedPhenotype::new(Q3232::from_num(1_i32), LifeStage::Adult);
        for (id, value) in channels {
            p.global_channels.insert((*id).to_string(), *value);
        }
        p
    }

    fn hook(
        id: u32,
        kind: CompositionKind,
        channels: &[&str],
        thresholds: &[Q3232],
        coefficient: Q3232,
    ) -> InterpreterHook {
        InterpreterHook {
            id: HookId(id),
            kind,
            channel_ids: channels.iter().map(|s| (*s).to_string()).collect(),
            thresholds: thresholds.to_vec(),
            coefficient,
            expression_conditions: Vec::new(),
            emits: vec![EmitSpec {
                primitive_id: format!("primitive_{id}"),
                parameter_mapping: Vec::new(),
            }],
        }
    }

    fn q(v: f64) -> Q3232 {
        Q3232::from_num(v)
    }

    // ---------------------------------------------------------------------
    // Threshold
    // ---------------------------------------------------------------------

    #[test]
    fn threshold_fires_when_all_values_meet_thresholds() {
        let phenotype = phenotype_with(&[("auditory", q(0.8)), ("vocal", q(0.6))]);
        let hooks = vec![hook(
            1,
            CompositionKind::Threshold,
            &["auditory", "vocal"],
            &[q(0.5), q(0.5)],
            Q3232::ONE,
        )];

        let fired = resolve_hooks(&phenotype, &hooks);

        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].hook_id, HookId(1));
        assert_eq!(fired[0].kind, CompositionKind::Threshold);
        assert_eq!(fired[0].channel_values, vec![q(0.8), q(0.6)]);
    }

    #[test]
    fn threshold_does_not_fire_when_one_value_below_threshold() {
        let phenotype = phenotype_with(&[("auditory", q(0.3)), ("vocal", q(0.8))]);
        let hooks = vec![hook(
            1,
            CompositionKind::Threshold,
            &["auditory", "vocal"],
            &[q(0.5), q(0.5)],
            Q3232::ONE,
        )];

        assert!(resolve_hooks(&phenotype, &hooks).is_empty());
    }

    #[test]
    fn threshold_dormant_channel_auto_fails_even_when_threshold_is_zero() {
        // §6.2: "If any operand is zero, this threshold automatically fails"
        // — even when the threshold itself is also zero.
        let phenotype = phenotype_with(&[("auditory", Q3232::ZERO), ("vocal", q(0.9))]);
        let hooks = vec![hook(
            1,
            CompositionKind::Threshold,
            &["auditory", "vocal"],
            &[Q3232::ZERO, q(0.5)],
            Q3232::ONE,
        )];

        assert!(resolve_hooks(&phenotype, &hooks).is_empty());
    }

    #[test]
    fn threshold_fires_when_value_equals_threshold() {
        // Spec uses `>=` for the cross condition.
        let phenotype = phenotype_with(&[("auditory", q(0.5))]);
        let hooks = vec![hook(
            1,
            CompositionKind::Threshold,
            &["auditory"],
            &[q(0.5)],
            Q3232::ONE,
        )];
        assert_eq!(resolve_hooks(&phenotype, &hooks).len(), 1);
    }

    // ---------------------------------------------------------------------
    // Gating
    // ---------------------------------------------------------------------

    #[test]
    fn gating_open_fires() {
        let phenotype = phenotype_with(&[("light", q(0.9))]);
        let hooks = vec![hook(
            2,
            CompositionKind::Gating,
            &["light"],
            &[q(0.5)],
            Q3232::ONE,
        )];
        assert_eq!(resolve_hooks(&phenotype, &hooks).len(), 1);
    }

    #[test]
    fn gating_closed_does_not_fire() {
        let phenotype = phenotype_with(&[("light", q(0.1))]);
        let hooks = vec![hook(
            2,
            CompositionKind::Gating,
            &["light"],
            &[q(0.5)],
            Q3232::ONE,
        )];
        assert!(resolve_hooks(&phenotype, &hooks).is_empty());
    }

    #[test]
    fn gating_dormant_channel_auto_fails() {
        let phenotype = phenotype_with(&[("light", Q3232::ZERO)]);
        let hooks = vec![hook(
            2,
            CompositionKind::Gating,
            &["light"],
            &[Q3232::ZERO],
            Q3232::ONE,
        )];
        assert!(resolve_hooks(&phenotype, &hooks).is_empty());
    }

    // ---------------------------------------------------------------------
    // Additive / Multiplicative (always fire once admitted)
    // ---------------------------------------------------------------------

    #[test]
    fn additive_always_fires_and_records_channel_values() {
        let phenotype = phenotype_with(&[("metabolism", q(0.2))]);
        let hooks = vec![hook(
            3,
            CompositionKind::Additive,
            &["metabolism"],
            &[],
            q(0.25),
        )];

        let fired = resolve_hooks(&phenotype, &hooks);

        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].channel_values, vec![q(0.2)]);
        assert_eq!(fired[0].coefficient, q(0.25));
    }

    #[test]
    fn multiplicative_always_fires() {
        let phenotype = phenotype_with(&[("regulator", q(0.75))]);
        let hooks = vec![hook(
            4,
            CompositionKind::Multiplicative,
            &["regulator"],
            &[],
            q(0.5),
        )];

        assert_eq!(resolve_hooks(&phenotype, &hooks).len(), 1);
    }

    // ---------------------------------------------------------------------
    // Antagonistic
    // ---------------------------------------------------------------------

    #[test]
    fn antagonistic_fires_when_difference_meets_coefficient() {
        let phenotype = phenotype_with(&[("aggression", q(0.9)), ("calm", q(0.2))]);
        let hooks = vec![hook(
            5,
            CompositionKind::Antagonistic,
            &["aggression", "calm"],
            &[],
            q(0.5),
        )];
        assert_eq!(resolve_hooks(&phenotype, &hooks).len(), 1);
    }

    #[test]
    fn antagonistic_does_not_fire_when_difference_below_coefficient() {
        let phenotype = phenotype_with(&[("aggression", q(0.5)), ("calm", q(0.4))]);
        let hooks = vec![hook(
            5,
            CompositionKind::Antagonistic,
            &["aggression", "calm"],
            &[],
            q(0.5),
        )];
        assert!(resolve_hooks(&phenotype, &hooks).is_empty());
    }

    #[test]
    fn antagonistic_uses_absolute_value() {
        // Negative difference still fires — we compare |diff| to coefficient.
        let phenotype = phenotype_with(&[("aggression", q(0.1)), ("calm", q(0.9))]);
        let hooks = vec![hook(
            5,
            CompositionKind::Antagonistic,
            &["aggression", "calm"],
            &[],
            q(0.5),
        )];
        assert_eq!(resolve_hooks(&phenotype, &hooks).len(), 1);
    }

    #[test]
    fn antagonistic_with_fewer_than_two_channels_fails_closed() {
        let phenotype = phenotype_with(&[("aggression", q(0.9))]);
        let hooks = vec![hook(
            5,
            CompositionKind::Antagonistic,
            &["aggression"],
            &[],
            q(0.5),
        )];
        assert!(resolve_hooks(&phenotype, &hooks).is_empty());
    }

    // ---------------------------------------------------------------------
    // Unknown-channel handling (lazy genesis)
    // ---------------------------------------------------------------------

    #[test]
    fn unknown_channel_silently_skips_hook() {
        let phenotype = phenotype_with(&[("auditory", q(0.9))]);
        let hooks = vec![hook(
            1,
            CompositionKind::Threshold,
            &["auditory", "genesis_channel_not_yet_present"],
            &[q(0.5), q(0.5)],
            Q3232::ONE,
        )];

        // Must not panic, must not error — just an empty result.
        assert!(resolve_hooks(&phenotype, &hooks).is_empty());
    }

    #[test]
    fn emits_list_is_preserved_when_hook_fires() {
        let phenotype = phenotype_with(&[("auditory", q(0.9))]);
        let mut h = hook(
            1,
            CompositionKind::Threshold,
            &["auditory"],
            &[q(0.5)],
            Q3232::ONE,
        );
        h.emits = vec![
            EmitSpec {
                primitive_id: "emit_acoustic_pulse".to_string(),
                parameter_mapping: Vec::new(),
            },
            EmitSpec {
                primitive_id: "receive_acoustic_signal".to_string(),
                parameter_mapping: Vec::new(),
            },
        ];

        let fired = resolve_hooks(&phenotype, &[h]);

        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].emits.len(), 2);
        assert_eq!(fired[0].emits[0].primitive_id, "emit_acoustic_pulse");
        assert_eq!(fired[0].emits[1].primitive_id, "receive_acoustic_signal");
    }

    // ---------------------------------------------------------------------
    // Ordering & multi-hook
    // ---------------------------------------------------------------------

    #[test]
    fn input_order_is_preserved() {
        let phenotype = phenotype_with(&[
            ("auditory", q(0.9)),
            ("metabolism", q(0.2)),
            ("regulator", q(0.75)),
        ]);
        let hooks = vec![
            hook(10, CompositionKind::Additive, &["metabolism"], &[], q(0.1)),
            hook(
                20,
                CompositionKind::Threshold,
                &["auditory"],
                &[q(0.5)],
                Q3232::ONE,
            ),
            hook(
                30,
                CompositionKind::Multiplicative,
                &["regulator"],
                &[],
                q(0.5),
            ),
        ];

        let fired = resolve_hooks(&phenotype, &hooks);
        let ids: Vec<HookId> = fired.iter().map(|f| f.hook_id).collect();
        assert_eq!(ids, vec![HookId(10), HookId(20), HookId(30)]);
    }

    // ---------------------------------------------------------------------
    // Determinism property test
    // ---------------------------------------------------------------------

    /// Fixed channel vocabulary for proptest generators. `"missing"` is
    /// intentionally not inserted into the phenotype so the "unknown channel"
    /// path exercises.
    const PROPTEST_CHANNELS: &[&str] = &["a", "b", "c", "d", "missing"];

    /// Kind index → `CompositionKind`.
    fn kind_from_index(i: u8) -> CompositionKind {
        match i % 5 {
            0 => CompositionKind::Additive,
            1 => CompositionKind::Multiplicative,
            2 => CompositionKind::Threshold,
            3 => CompositionKind::Gating,
            _ => CompositionKind::Antagonistic,
        }
    }

    /// Build a phenotype with the channels `a..d` populated (but not `missing`).
    fn phenotype_from_bits(values: [i64; 4]) -> ResolvedPhenotype {
        let mut p = ResolvedPhenotype::new(Q3232::from_num(1_i32), LifeStage::Adult);
        let mut map: BTreeMap<String, Q3232> = BTreeMap::new();
        for (id, bits) in ["a", "b", "c", "d"].iter().zip(values.iter()) {
            map.insert((*id).to_string(), Q3232::from_bits(*bits));
        }
        p.global_channels = map;
        p
    }

    /// Build an interpreter hook from raw proptest scalars. `channel_indices`
    /// picks from `PROPTEST_CHANNELS`. `threshold_bits` is padded / truncated
    /// to match the number of channels.
    fn hook_from_raw(
        index: u32,
        kind_index: u8,
        channel_indices: Vec<usize>,
        threshold_bits: Vec<i64>,
        coefficient_bits: i64,
    ) -> InterpreterHook {
        let channel_ids: Vec<String> = channel_indices
            .iter()
            .map(|i| PROPTEST_CHANNELS[*i % PROPTEST_CHANNELS.len()].to_string())
            .collect();
        let mut thresholds: Vec<Q3232> = threshold_bits
            .iter()
            .map(|b| Q3232::from_bits(*b))
            .collect();
        thresholds.resize(channel_ids.len(), Q3232::ZERO);
        InterpreterHook {
            id: HookId(index),
            kind: kind_from_index(kind_index),
            channel_ids,
            thresholds,
            coefficient: Q3232::from_bits(coefficient_bits),
            expression_conditions: Vec::new(),
            emits: vec![EmitSpec {
                primitive_id: format!("p_{index}"),
                parameter_mapping: Vec::new(),
            }],
        }
    }

    /// Sample `Q3232` raw bits in `±2.0`. Using `from_bits` keeps the
    /// distribution deterministic (no `f64` rounding in the generator).
    const PROPTEST_Q_BOUND: i64 = 1_i64 << 33;

    proptest! {
        /// Pure-function determinism: calling `resolve_hooks` twice on the
        /// same inputs yields identical results. Required by INVARIANTS §1.
        #[test]
        fn resolve_hooks_is_pure(
            phenotype_bits in prop::array::uniform4(-PROPTEST_Q_BOUND..=PROPTEST_Q_BOUND),
            hook_specs in prop::collection::vec(
                (
                    0u8..5,
                    prop::collection::vec(0usize..PROPTEST_CHANNELS.len(), 1..=3),
                    prop::collection::vec(-PROPTEST_Q_BOUND..=PROPTEST_Q_BOUND, 0..=3),
                    -PROPTEST_Q_BOUND..=PROPTEST_Q_BOUND,
                ),
                0..=8,
            ),
        ) {
            let phenotype = phenotype_from_bits(phenotype_bits);
            let hooks: Vec<InterpreterHook> = hook_specs
                .into_iter()
                .enumerate()
                .map(|(idx, (kind_i, chans, thresh, coef))| {
                    hook_from_raw(idx as u32, kind_i, chans, thresh, coef)
                })
                .collect();

            let first = resolve_hooks(&phenotype, &hooks);
            let second = resolve_hooks(&phenotype, &hooks);

            prop_assert_eq!(first.len(), second.len());
            prop_assert!(first.len() <= hooks.len());
            for (a, b) in first.iter().zip(second.iter()) {
                prop_assert_eq!(a.hook_id, b.hook_id);
                prop_assert_eq!(a.kind, b.kind);
                prop_assert_eq!(&a.channel_values, &b.channel_values);
                prop_assert_eq!(a.coefficient, b.coefficient);
                prop_assert_eq!(a.emits.len(), b.emits.len());
            }
        }
    }
}
