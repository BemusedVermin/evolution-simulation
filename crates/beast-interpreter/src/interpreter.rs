//! Top-level entry point [`interpret_phenotype`].
//!
//! Sequences the four pipeline stages from
//! `documentation/systems/11_phenotype_interpreter.md` §6 into a single pure,
//! deterministic function:
//!
//! 1. Scale-band filter     ([`crate::scale_band::apply_scale_band_filter`])
//! 2. Affordance filter     ([`crate::expression::filter_hooks_by_affordances`])
//! 3. Composition resolve   ([`crate::composition::resolve_hooks`])
//! 4. Primitive emission    ([`crate::emission::emit_primitives`])
//!
//! Behaviour compilation (stage 3 in the doc) and body-site primitive fanout
//! (§6.0B, tracked in issue #67) are deliberately out of scope for S4.
//!
//! # Determinism
//!
//! This function is pure: `(phenotype, hooks, channel_registry,
//! primitive_registry, emitter)` uniquely determines the returned
//! `Vec<PrimitiveEffect>`. No RNG, no wall-clock reads, no `HashMap` /
//! `HashSet` iteration, and every arithmetic path goes through
//! [`beast_core::Q3232`]. See INVARIANTS §1.
//!
//! The input `phenotype` is **not** mutated — the scale-band stage operates on
//! an internal clone so callers can reuse the same phenotype across repeated
//! invocations without observing side effects.

use beast_channels::ChannelRegistry;
use beast_core::EntityId;
use beast_primitives::{PrimitiveEffect, PrimitiveRegistry};

use crate::composition::{resolve_hooks, InterpreterHook};
use crate::emission::emit_primitives;
use crate::expression::filter_hooks_by_affordances;
use crate::phenotype::ResolvedPhenotype;
use crate::scale_band::apply_scale_band_filter;
use crate::Result;

/// Run the full interpreter pipeline and return the primitive-effect set.
///
/// # Stages
///
/// 1. **Scale-band filter** — zero out channel values that fall outside their
///    manifest's `scale_band` for the creature's body mass
///    ([`crate::scale_band::apply_scale_band_filter`]). Applied to a clone of
///    `phenotype` so the caller's copy is left untouched.
/// 2. **Affordance filter** — drop hooks whose environmental
///    `expression_conditions` fail
///    ([`crate::expression::filter_hooks_by_affordances`]). The filter returns
///    the surviving hook ids sorted ascending; those ids are then materialised
///    back into a filtered `Vec<InterpreterHook>` preserving that order.
/// 3. **Composition resolve** — decide which surviving hooks fire against the
///    scale-band-filtered phenotype and produce `FiredHook`s
///    ([`crate::composition::resolve_hooks`]).
/// 4. **Primitive emission** — evaluate each emit spec's parameter
///    expressions, compute costs, merge duplicates by `primitive_id`
///    ([`crate::emission::emit_primitives`]). The returned vector is sorted by
///    `primitive_id`.
///
/// # Arguments
///
/// * `phenotype` — materialised creature state (per-channel globals,
///   per-region amplitudes, life stage, mass, environment).
/// * `hooks` — per-tick `InterpreterHook` slice; typically loaded once at
///   world init and shared across creatures. This function does not mutate
///   or retain a reference to the slice.
/// * `channel_registry` — authoritative channel registry (core + mods +
///   genesis). Used by stage 1 to look up scale bands.
/// * `primitive_registry` — authoritative primitive registry. Used by stage
///   4 to look up primitive manifests for cost evaluation.
/// * `emitter` — the [`EntityId`] attributed to every emitted
///   [`PrimitiveEffect`].
///
/// # Errors
///
/// Returns the first error from [`emit_primitives`] — in practice
/// [`crate::InterpreterError::UnknownPrimitive`] if any `EmitSpec`
/// references a primitive not in `primitive_registry`, or
/// [`crate::InterpreterError::ParseError`] propagated from cost evaluation.
/// Stages 1-3 never fail; malformed hooks and missing channels are handled
/// by dropping the offending hook (dormant-channel / lazy-genesis semantics,
/// §6.2).
///
/// # Determinism
///
/// Pure function: identical inputs produce bit-identical outputs on every
/// call, across runs, and across platforms — a precondition for the
/// 1000-tick replay gate (INVARIANTS §1). Covered by integration tests in
/// `tests/determinism.rs`.
pub fn interpret_phenotype(
    phenotype: &ResolvedPhenotype,
    hooks: &[InterpreterHook],
    channel_registry: &ChannelRegistry,
    primitive_registry: &PrimitiveRegistry,
    emitter: EntityId,
) -> Result<Vec<PrimitiveEffect>> {
    // Stage 1 — scale-band filter. Operate on a clone so the caller's
    // phenotype is untouched; the filter mutates `global_channels` and, for
    // `body_site_applicable` channels, `body_map` amplitudes.
    let mut filtered_phenotype = phenotype.clone();
    apply_scale_band_filter(&mut filtered_phenotype, channel_registry);

    // Stage 2 — affordance filter. Returns the sorted subset of HookIds
    // whose environmental conditions all hold.
    let active_ids = filter_hooks_by_affordances(
        &filtered_phenotype.environment,
        filtered_phenotype.life_stage,
        filtered_phenotype.body_mass_kg,
        hooks,
    );

    // Materialise the active hooks in id-sorted order (the order the
    // affordance filter returned). `active_ids` is typically small, so a
    // linear scan per id is fine; a BTreeMap index would add allocation for
    // no speed-up at interpreter-scale hook counts.
    let active_hooks: Vec<InterpreterHook> = active_ids
        .iter()
        .filter_map(|id| hooks.iter().find(|h| h.id == *id).cloned())
        .collect();

    // Stage 3 — composition resolve. Produces one `FiredHook` per hook that
    // fires; input order (== id-sorted from stage 2) is preserved.
    let fired = resolve_hooks(&filtered_phenotype, &active_hooks);

    // Stage 4 — primitive emission. Evaluates parameter expressions, merges
    // duplicates by primitive_id, returns a vector sorted by primitive_id.
    emit_primitives(&fired, &filtered_phenotype, primitive_registry, emitter)
}
