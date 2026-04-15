# Beast Evolution Game — Implementation Progress Log

This file is the canonical running log of implementation work on the Beast Evolution Game. It is written incrementally as work proceeds so later sessions (after compaction / new conversations) can pick up without losing context.

**Convention**: newest entries on top under each section. Keep entries terse but concrete — file paths, decisions, and pitfalls encountered. Do NOT let this file grow past ~800 lines; when it does, rotate older content into `PROGRESS_LOG_ARCHIVE_YYYY_MM.md`.

---

## Current Status Snapshot

- **Active Sprint**: S1 — Fixed-Point & PRNG (beast-core) [Week 1]
- **Phase**: 1 — Foundations & Core Sim
- **Workspace scaffolded**: no (in progress)
- **Last updated**: 2026-04-15

### Sprint S1 Story Progress

| ID  | Title                                                | Points | Status      |
|-----|------------------------------------------------------|--------|-------------|
| 1.1 | Q32.32 fixed-point type with saturating arithmetic   | 8      | In Progress |
| 1.2 | Xoshiro256PlusPlus PRNG with seeding & streams       | 8      | Not Started |
| 1.3 | EntityID, TickCounter, custom Error type             | 6      | Not Started |
| 1.4 | Box-Muller Gaussian sampling & saturating math utils | 6      | Not Started |
| 1.5 | Unit tests + property-based fuzzing (100k samples)   | 6      | Not Started |
| 1.6 | Benchmarking & documentation                         | 6      | Not Started |

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
