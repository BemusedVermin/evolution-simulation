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

// Resolver implementation — see story S4.3 (#57).
