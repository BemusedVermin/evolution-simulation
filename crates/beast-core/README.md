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
use beast_core::{Prng, Q3232, TickCounter};

let mut rng = Prng::from_seed(0xDEAD_BEEF_CAFE_BABE);
let mut tick = TickCounter::ZERO;

let a = Q3232::from_num(0.25_f64);
let b = Q3232::from_num(0.5_f64);
let c = a + b; // saturating

tick.advance();
let roll = rng.next_u64();
let _ = (c, tick, roll);
```

See `documentation/INVARIANTS.md` and `documentation/architecture/CRATE_LAYOUT.md` for the contract this crate exists to uphold.
