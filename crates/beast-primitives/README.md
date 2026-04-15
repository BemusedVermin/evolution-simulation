# beast-primitives

Layer 1 primitive effect registry and cost evaluator for Beast Evolution Game.

Primitives are the atomic output vocabulary of the phenotype interpreter.
They are never named abilities in themselves ("echolocation", "venom"); the
Chronicler assigns labels post-hoc over recurring primitive clusters — see
`documentation/INVARIANTS.md` §2 (Mechanics-Label Separation).

## Responsibilities

- **Manifest loading** — `PrimitiveManifest::from_json_str` mirrors the
  two-stage pattern used by `beast-channels`: JSON Schema validation, then
  semantic checks (range ordering, scaling parameters referring to declared
  inputs, default type matching, provenance parsing).
- **Registry** — `PrimitiveRegistry` is `BTreeMap`-backed with a secondary
  index by [`PrimitiveCategory`]. Iteration is deterministic.
- **Cost evaluation** — `evaluate_cost` computes

    cost = base_metabolic_cost + Σ coefficient_i · value_i ^ exponent_i

  deterministically over `Q3232`. A crate-local fixed-point `exp`/`ln` pair
  implements the power function; see `src/math.rs` for the algorithms. All
  manifest exponents in `documentation/schemas/primitive_vocabulary/` are
  supported (integers, half-integers, 0.6/0.7/0.8, 1.2, -1.0, -1.2, …).

## Determinism

- All sim-math goes through `Q3232` from `beast-core`.
- The embedded `exp`/`ln` approximation uses Taylor series with fixed
  iteration counts — no conditional early exits that would make results
  depend on floating-point rounding.
- `clippy::float_arithmetic = "warn"` prevents future regressions.

## Layering

Depends on `beast-core` and `beast-channels`. The channel dependency is used
to type-check `composition_compatibility.channel_family` entries against the
shared `ChannelFamily` enum and to validate `channel_id` references against
a live `ChannelRegistry` at startup.
