# beast-genome

Layer 2 genotype and mutation crate for Beast Evolution Game.

This crate owns the *structure* of an organism's genome — trait genes, body
sites, regulatory modifiers, and lineage metadata — along with the
deterministic mutation operators that act on them. Downstream crates
(`beast-interpreter`, `beast-sim`) consume `Genome` values; this crate is
pure-data + pure-function, with no ECS or tick-loop awareness.

## Responsibilities

- **Genome data structures** — `Genome`, `TraitGene`, `EffectVector`,
  `BodyVector`, `Modifier`, `LineageTag`. All numeric fields are
  `beast_core::Q3232`.
- **Mutation operators** — point mutation (Gaussian drift), channel shift,
  body-site drift, silencing toggle, regulatory rewiring, and gene
  duplication / genesis (default-off per System 01 §6B).
- **Provenance tracking** — new paralogs carry `genesis:{parent}:{tick}`
  strings that satisfy the INVARIANTS §3 regex shared with channel
  manifests.

## Determinism

- No `f32`/`f64` in sim-state types. `clippy::float_arithmetic = "warn"`
  guards the arithmetic surface.
- All randomness flows through a single `beast_core::Prng` handle derived
  from `Stream::Genetics` — this crate never constructs its own streams.
- Genomes iterate by index (`Vec<TraitGene>`); no `HashMap`/`HashSet`
  in structure that affects sim state.

## Layering

Depends on `beast-core` (foundations) and `beast-channels` (for the
provenance type and registry lookups used by duplication). Per
`CRATE_LAYOUT.md`, this crate sits at Layer 2 and does **not** depend on
`beast-primitives`, `beast-interpreter`, `beast-ecs`, or anything above.

## Status

Sprint S3 work in progress. See the tracker epic (GitHub `label:epic
sprint:s3`) for the story checklist.
