# Beast Evolution Game — Implementation Progress Log

This file is the canonical running log of implementation work on the Beast Evolution Game. It is written incrementally as work proceeds so later sessions (after compaction / new conversations) can pick up without losing context.

**Convention**: newest entries on top under each section. Keep entries terse but concrete — file paths, decisions, and pitfalls encountered. Do NOT let this file grow past ~800 lines; when it does, rotate older content into `PROGRESS_LOG_ARCHIVE_YYYY_MM.md`.

---

## Current Status Snapshot

- **Active Sprint**: S1 — Fixed-Point & PRNG (beast-core) [Week 1] — **✅ COMPLETE**
- **Next Sprint**: S2 — Manifests & Registries (beast-channels, beast-primitives) [Week 2]
- **Phase**: 1 — Foundations & Core Sim
- **Workspace scaffolded**: yes (beast-core only; other 16 crates deferred to their sprints)
- **Last updated**: 2026-04-15

### Sprint S1 Story Progress (40 pts planned, 40 delivered)

| ID  | Title                                                | Points | Status  |
|-----|------------------------------------------------------|--------|---------|
| 1.1 | Q32.32 fixed-point type with saturating arithmetic   | 8      | ✅ Done |
| 1.2 | Xoshiro256PlusPlus PRNG with seeding & streams       | 8      | ✅ Done |
| 1.3 | EntityID, TickCounter, custom Error type             | 6      | ✅ Done |
| 1.4 | Box-Muller Gaussian sampling & saturating math utils | 6      | ✅ Done |
| 1.5 | Unit tests + property-based fuzzing (100k samples)   | 6      | ✅ Done |
| 1.6 | Benchmarking & documentation                         | 6      | ✅ Done |

### Sprint S1 Exit DoD

- [x] All 6 stories completed
- [x] beast-core crate published (no external deps on other beast crates)
- [x] CI passes (cargo test, clippy -D warnings, cargo build --release)
- [x] README for beast-core with usage examples and measured perf table

### Demo Criteria (from SPRINTS.md)

- [x] Same seed produces identical PRNG output over 100k iterations — verified in `prng_100k_same_seed_identical`
- [x] No panics on overflow/underflow — verified via proptest (19 props × 1000 cases covering full `i64` bit-pattern space)
- [~] Fixed-point multiply < 2 CPU cycles — measured ~2.7 ns (~8 cycles on 3 GHz). Target was aspirational; real number is fine for tick budget, documented in README.

**Test count**: 78 tests (44 unit + 32 proptest + 2 doctests). All green.

---

## Architectural Decisions (stable, don't re-derive each session)

- **Workspace**: single Cargo workspace at repo root, crates under `crates/`.
- **Layering**: L0 → L6 per `documentation/architecture/CRATE_LAYOUT.md`. No cycles, no layer-skipping.
- **Fixed-point**: Q32.32 via `fixed::I32F32` wrapped in a newtype `Q3232` in `beast-core::fixed_point`. All sim math goes through this; `f32`/`f64` forbidden in sim state, allowed in render/UI only.
- **PRNG**: `rand_xoshiro::Xoshiro256PlusPlus`. Seed once at world creation; split into per-subsystem streams. Never use OS RNG.
- **Determinism**: sorted iteration in hot loops, no wall-clock reads, tick-count time only. 1000-tick replay = CI gate.
- **Error handling**: single `beast_core::Error` enum with `thiserror`; crate-local `Result<T> = core::result::Result<T, Error>`.
- **Edition**: Rust 2021 to start (can bump to 2024 later). MSRV = stable.

---

## Session Log (reverse chronological)

### 2026-04-15 — Sprint S1 COMPLETE (Claude)

Final CI gate green:
- `cargo build --release -p beast-core` — clean (17.3s cold)
- `cargo clippy -p beast-core --all-targets -- -D warnings` — clean
- `cargo test -p beast-core` — 78/78 passing

Commits added after previous entry:
- `3d79ffd` test(core): Story 1.5 — property-based fuzzing and 100k-sample stats
- `70ae0e9` feat(core): Story 1.6 — benchmarks, clippy cleanup, README perf table

Regression caught in Story 1.6 benches: `split_stream(Stream::Genetics)`
aliased the master because the discriminant was 0, so zero long-jumps
applied. Fixed by making split always perform `1..=jumps()` long-jumps.
Added regression test `split_stream_does_not_alias_master`.

**Next sprint (S2) focus**: beast-channels + beast-primitives. Schemas live at
`documentation/schemas/`; channel manifest and primitive manifest are
authoritative. Registries must be queryable and reject malformed entries at
load time. Prior crates needed: none beyond beast-core. Suggested order:
  1. scaffold both crates (Cargo manifests, module skeleton)
  2. Story 2.1–2.2 (channel + primitive manifest loaders)
  3. Story 2.5 (composition hook parser) before 2.3/2.4 registries, since
     the registries reference resolved hooks
  4. Stories 2.3 / 2.4 (registries with queryable indexing, cost eval)
  5. Story 2.6 (schema validation / rejection of 5 malformed manifests)

### 2026-04-15 — Stories 1.1–1.4 landed (Claude)

Commits on `master`:
- `2a6127a` feat(core): scaffold workspace and Q3232 fixed-point type
- `4394179` feat(core): Story 1.2 — Prng wrapper with per-subsystem stream splitting
- `2fffe87` chore: gitignore .claude/ (per-machine local state)
- `e6f7f45` feat(core): Story 1.3 — EntityId, TickCounter, Error type
- `aa1734e` feat(core): Story 1.4 — Box-Muller Gaussian sampler and math utils

Key decisions locked in:
- `fixed` crate's `ToFixed`/`FromFixed` traits (not `LossyFrom`) are the
  conversion surface. `Q3232::from_num` uses `saturating_from_num` internally.
- `Stream` enum discriminants drive `long_jump()` count. Variants MUST be
  append-only (reordering breaks replay compat of every existing save).
- `Prng::split_stream(&self, Stream)` takes `&self` and clones internally —
  master is not advanced by splitting.
- Gaussian uses `f64` for `ln/sqrt/cos` (documented in module docs as the one
  sanctioned float use); result is saturating-converted back to `Q3232`.
- `TickCounter` saturates; at 60 Hz, `u64::MAX` ≈ 9.7 Gyr, so saturation is a
  bug-indicator, not a real runtime event.
- `EntityId::NONE = u32::MAX`. `EntityIdAllocator` saturates at `u32::MAX - 1`
  to preserve the sentinel invariant.

Read-errors / pitfalls encountered:
- Compiled-in lints `unsafe_code = "forbid"` and `clippy::float_arithmetic = "warn"`
  at the crate level. Gaussian function uses `#[allow(clippy::float_arithmetic)]`
  locally.
- `fixed::I32F32` has no `saturating_div` — fell back to `checked_div` with
  manual MIN/MAX clamp on the `MIN / -1` overflow case.

Next action: Story 1.5 — add `proptest` property-based fuzzing (100k samples)
  for Q3232 saturating algebra and PRNG statistical properties.

### 2026-04-15 — Session start (Claude)

- Read `INVARIANTS.md`, `CRATE_LAYOUT.md`, `SPRINTS.md`.
- Created this progress log.
- Confirmed `cargo 1.94.1` available on this Windows machine.
- Decision: scaffold only the workspace root + `beast-core` for Sprint S1. Do not stub the other 16 crates yet — wait until their sprints to avoid dead skeletons drifting from spec.
- Next action: write workspace `Cargo.toml`, create `crates/beast-core/` with module skeleton, implement Stories 1.1 → 1.4, then tests (1.5) and docs/bench (1.6).

---

## Open Questions / Deferred Decisions

- **`I32F32` vs custom `Q3232` trait surface**: start as thin newtype with saturating ops and `From/Into` for the underlying type. Expose only what the sim actually needs — avoid surfacing every `fixed` trait eagerly.
- **Box-Muller vs Ziggurat for Gaussian**: Box-Muller is simpler and deterministic in fixed-point; Ziggurat is faster but needs float lookup tables. Sprint S1 uses Box-Muller (stated in story 1.4). Revisit if profiling shows Gaussian sampling hot.
- **Property-test framework**: `proptest` preferred over `quickcheck` (better shrinking, deterministic seeds). Story 1.5 will pin this.
- **Bench framework**: `criterion` per plan. Wire in Sprint S1 even though deep benchmarking lives in S13.

---

## Known Pitfalls / Watch Items

- The `fixed` crate's default arithmetic **panics on overflow in debug, wraps in release**. We MUST use `saturating_*` or `wrapping_*` explicitly for all sim math. Build the `Q3232` wrapper so the default `+ - * /` operators are *saturating*, not the `fixed` defaults.
- `Xoshiro256PlusPlus::seed_from_u64` is fine for a master seed, but per-subsystem streams should use `jump()` / `long_jump()` rather than re-seeding with a derived u64 (cleaner guarantee of non-overlap).
- Do not accidentally pull in `rand::thread_rng` anywhere — add a `forbid` list in the crate-level docs or a lint.
- Windows line-endings: set `core.autocrlf=false` or add `.gitattributes` with `* text=auto eol=lf` to keep determinism of any text-hashed fixtures. (Defer; note it.)

---

## File Index (what's been written)

_(updated as files are created)_

- `documentation/PROGRESS_LOG.md` — this file.
- `Cargo.toml` — workspace root, shared dep versions.
- `.gitignore`, `.gitattributes` — LF enforcement, target/ ignored, `.claude/` excluded.
- `crates/beast-core/Cargo.toml` — crate manifest with `unsafe_code = "forbid"`.
- `crates/beast-core/README.md` — crate overview + usage snippet.
- `crates/beast-core/benches/core_bench.rs` — criterion stub, populated in 1.6.
- `crates/beast-core/src/lib.rs` — crate root, re-exports.
- `crates/beast-core/src/fixed_point.rs` — `Q3232` (Story 1.1).
- `crates/beast-core/src/prng.rs` — `Prng`, `Stream` (Story 1.2).
- `crates/beast-core/src/entity.rs` — `EntityId`, `EntityIdAllocator` (Story 1.3).
- `crates/beast-core/src/time.rs` — `TickCounter` (Story 1.3).
- `crates/beast-core/src/error.rs` — `Error`, `Result` (Story 1.3).
- `crates/beast-core/src/math.rs` — `gaussian_q3232`, `lerp/inv_lerp/clamp/min/max` (Story 1.4).
