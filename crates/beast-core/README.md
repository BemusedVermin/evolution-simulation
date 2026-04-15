# beast-core

Foundational primitives shared by every other crate in the Beast Evolution Game workspace. Layer 0 — depends on no other `beast-*` crate.

## Scope

- `fixed_point`: `Q3232`, a deterministic Q32.32 fixed-point wrapper around `fixed::I32F32` with **saturating-by-default** arithmetic. All simulation state uses this; `f32`/`f64` are forbidden in sim state.
- `prng`: `Prng`, a thin wrapper around `rand_xoshiro::Xoshiro256PlusPlus` with explicit `Stream` splitting (one stream per subsystem, no cross-contamination).
- `entity`: `EntityId`, a `u32`-backed newtype used across the ECS.
- `time`: `TickCounter`, a `u64` tick counter with saturating increment.
- `error`: `Error` / `Result<T>` — the single error type used throughout `beast-core`.
- `math`: saturating helpers, clamp, lerp, and Box–Muller Gaussian sampling in `Q3232`.

## Non-negotiables enforced here

- **Determinism**: no wall-clock reads, no OS RNG, no float in any public API on the sim path.
- **Saturation**: `Q3232` arithmetic saturates rather than wraps or panics. Overflow is a silent clamp to `MIN`/`MAX`, which is preferable to non-deterministic panic behaviour between `debug` and `release`.
- **`#![forbid(unsafe_code)]`**: crate-level lint.

## Usage

```rust
use beast_core::{Prng, Q3232, Stream, TickCounter};

// Every world is seeded once.
let master = Prng::from_seed(0xDEAD_BEEF_CAFE_BABE);

// Each subsystem gets its own stream, derived via long-jump.
let mut genetics = master.split_stream(Stream::Genetics);
let mut physics = master.split_stream(Stream::Physics);

// Sim math flows through Q3232.
let mut tick = TickCounter::ZERO;
let drift = Q3232::from_num(0.25_f64);
let stride = Q3232::from_num(0.5_f64);
let step = drift + stride; // saturating

tick.advance();
let _ = (step, tick, genetics.next_u64(), physics.next_u64());
```

## Measured performance (Windows 11, release build, representative run)

| Operation                    | Time      |
|------------------------------|-----------|
| `Q3232::saturating_add`      | ~0.83 ns  |
| `Q3232::saturating_mul`      | ~2.70 ns  |
| `Q3232::saturating_div`      | ~8.30 ns  |
| `Prng::next_u64`             | ~0.71 ns  |
| `Prng::next_q3232_unit`      | ~0.73 ns  |
| `Prng::split_stream`         | ~1.52 ns  |
| `gaussian_q3232` (Box–Muller)| ~33.0 ns  |

Numbers are indicative — rerun `cargo bench -p beast-core` to measure on your machine. Saturating multiply lands at ~8 cycles on a 3 GHz CPU; the sprint plan's aspirational "< 2 cycles" target is not achievable for a 64×64→128 saturating mul, which is fine — the tick budget at 60 Hz still accommodates millions of these per tick.

## Invariants enforced here

See `documentation/INVARIANTS.md` and `documentation/architecture/CRATE_LAYOUT.md` for the full contract this crate exists to uphold.
