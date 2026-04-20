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

## Usage

Build a `Genome`, then advance it one tick at a time with
`apply_mutations`. All randomness flows through a single `Prng` handle —
derive it from `Stream::Genetics` at the world root and pass it down.

```rust
use beast_channels::Provenance;
use beast_core::{Prng, Stream, TickCounter, Q3232};
use beast_genome::{
    apply_mutations, BodyVector, EffectVector, Genome, GenomeParams,
    LineageTag, Target, Timing, TraitGene,
};

// One trait gene: a passive "kinetic_force" contribution at half strength.
let effect = EffectVector::new(
    vec![Q3232::from_num(0.25_f64); 3], // per-channel contributions
    Q3232::from_num(0.5_f64),           // magnitude
    Q3232::from_num(0.25_f64),          // radius
    Timing::Passive,
    Target::SelfEntity,
)
.unwrap();

let gene = TraitGene::new(
    "kinetic_force",
    effect,
    BodyVector::default_internal(),
    Vec::new(),                  // no regulatory modifiers
    true,                        // enabled
    LineageTag::from_raw(1),
    Provenance::Core,
)
.unwrap();

let mut genome = Genome::new(GenomeParams::default(), vec![gene]).unwrap();

// Derive the genetics stream from the master seed once per world.
let master = Prng::from_seed(0xDEAD_BEEF);
let mut genetics = master.split_stream(Stream::Genetics);

// Advance the genome for 100 ticks.
for tick in 0..100u64 {
    apply_mutations(&mut genome, TickCounter::new(tick), &mut genetics);
}

// Structural invariants always hold after mutation.
genome.validate().unwrap();
```

Individual operators (`mutate_point`, `mutate_regulatory`,
`mutate_duplicate`, `mutate_duplication_rate`) are also public for tests
and bespoke pipelines; prefer `apply_mutations` for the canonical
per-tick order.

## Status

Sprint S3 complete. See the tracker epic (GitHub `label:epic
sprint:s3`) for the story checklist.
