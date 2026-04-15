# beast-channels

Layer 1 channel registry and manifest loader for Beast Evolution Game.

This crate owns the runtime side of the **Channel Manifest** contract
defined in `documentation/schemas/channel_manifest.schema.json`. It is a
pure-data, side-effect-free crate: it parses JSON, validates it against the
canonical JSON Schema, and produces strongly typed, deterministic
[`ChannelManifest`] and [`ChannelRegistry`] values for downstream consumers.

## Responsibilities

- **Manifest loading** — `ChannelManifest::from_json_str` runs a two-stage
  validator: JSON Schema first (catches shape, enum, and pattern violations
  with JSON-pointer paths), then a semantic pass (`range.min <= range.max`,
  unique composition-hook targets, provenance string parsing).
- **Registry** — `ChannelRegistry` is a `BTreeMap`-backed lookup with
  secondary indices by [`ChannelFamily`]. Deterministic iteration is a
  requirement of `documentation/INVARIANTS.md` §1.
- **Composition hooks** — `evaluate_hook` deterministically reduces a single
  hook into `(delta, factor, gate_open)` using `beast_core::Q3232`
  arithmetic, so interpreters can fold hooks without introducing float
  non-determinism.
- **Expression conditions** — `evaluate_expression_conditions` applies the
  schema's discriminated-union gates (biome flag, scale band, season,
  developmental stage, social density) against an [`ExpressionContext`].

## Determinism

- All numeric manifest fields that participate in sim math are converted to
  `Q3232` at load time via `Q3232::from_num`.
- The crate's `Cargo.toml` sets `clippy::float_arithmetic = "warn"`; any
  future regression that introduces float arithmetic will surface in CI.
- Registry indices are `BTreeMap`/`BTreeSet`-based, giving stable iteration
  regardless of hash randomization.

## Layering

Depends only on `beast-core`. Downstream crates (`beast-primitives`,
`beast-interpreter`, `beast-genome`, …) read manifests through this crate
but never parse JSON themselves.
