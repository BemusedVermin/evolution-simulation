# Beast Evolution Game — Implementation Progress Log

This file is the canonical running log of implementation work on the Beast Evolution Game. It is written incrementally as work proceeds so later sessions (after compaction / new conversations) can pick up without losing context.

**Convention**: newest entries on top under each section. Keep entries terse but concrete — file paths, decisions, and pitfalls encountered. Do NOT let this file grow past ~800 lines; when it does, rotate older content into `PROGRESS_LOG_ARCHIVE_YYYY_MM.md`.

**Scope**: this log is a narrative diary — decisions, pitfalls, commit references. **Live sprint/story status is on the [GitHub Project board](https://github.com/users/BemusedVermin/projects/1) and in the [sprint epic issues](https://github.com/BemusedVermin/evolution-simulation/issues?q=is%3Aissue+label%3Aepic), not here.** The snapshot below is refreshed opportunistically but is not the source of truth; filter the board by `Sprint = Sn` or `Status = In Progress` for current state.

---

## Current Status Snapshot

- **Active Sprint**: S3 — Genome & Mutation (beast-genome) [Week 3] — not yet started
- **Completed Sprints**: S1 — Fixed-Point & PRNG (beast-core) [Week 1]; S2 — Manifests & Registries (beast-channels, beast-primitives) [Week 2]
- **Next Sprint**: S4 — Phenotype Interpreter (beast-interpreter) [Week 4]
- **Phase**: 1 — Foundations & Core Sim
- **Workspace scaffolded**: yes (beast-core, beast-channels, beast-primitives; other 14 crates deferred to their sprints)
- **CI**: `.github/workflows/ci.yml` runs on every PR — fmt, clippy, test, doctests, release build, and cross-platform tests on windows/macOS. First run on PR #2 passed all 4 jobs.
- **Push protection**: client-side (`.githooks/pre-push`) blocks direct pushes to master once activated via `git config core.hooksPath .githooks`. Server-side branch protection **not** configured — GitHub REST API rejected the call on this private/free repo (requires Pro or public visibility). Workflow job names are documented in `CONTRIBUTING.md` for the day we turn that on.
- **Merged PRs on master**:
  - **#1** `sprint-s1-beast-core` — all six S1 stories (scaffold + Q3232 + PRNG + EntityId/TickCounter/Error + Gaussian + proptests + benches) plus the post-review fixes (`EntityIdAllocator::alloc → Option<EntityId>`, `gen_range_i64` i128-widened final add).
  - **#2** `ci-workflow` — GitHub Actions CI + `rust-toolchain.toml` + rustfmt normalisation pass.
  - **#3** `push-protection` — pre-push hook + `CONTRIBUTING.md`.
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

**Test count**: 82 tests (47 unit + 33 proptest + 2 doctests). All green. (Grew by 4 during PR #1 review: 3 unit + 1 proptest added to cover `EntityIdAllocator` exhaustion and `gen_range_i64` wide-span regressions.)

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

### 2026-04-15 — GitHub Project board + per-sprint issues (Claude)

Migrated sprint/story tracking from the markdown planning docs to GitHub
so live status has one source of truth and parallel sprints are easier
to see at a glance.

- **49 labels** created: `sprint:s1..s18`, `phase:1..4`, `epic`, `story`,
  `determinism`, `invariant`, `security`, plus `crate:*` for all 13
  workspace crates.
- **18 sprint epic issues** (#13–#30) with sprint goal, story checklist,
  demo criteria, DoD, invariant notes, and design-doc pointers sourced
  from `SPRINTS.md`. S1/S2 closed as done with PR links; S3–S18 are Todo.
- **Project v2 board** "Beast Evolution Game — Roadmap" at
  https://github.com/users/BemusedVermin/projects/1 with custom fields:
  **Sprint** (S1–S18 single-select), **Phase** (1–4 single-select),
  **Points** (number). All 18 items added and populated.
- **Documentation updates** (this commit): top-level `README.md` and
  `CONTRIBUTING.md` now point at the board + issues as the source of
  truth for live status; `documentation/planning/README.md` drops the
  "update SPRINTS.md status columns weekly" workflow in favour of moving
  issues on the board; tracking banners prepended to `SPRINTS.md` and
  `EPICS.md` clarifying that their Status columns are historical scope.
  This log gets a pointer at the top too — PROGRESS_LOG stays a narrative
  diary, not a status tracker.

Rationale: the markdown Status columns had already drifted out of date
during the S2 session, and the one-to-many relationship (one sprint →
many stories, each potentially a PR) is more naturally expressed as
labelled issues than as rows in a table. The planning docs remain the
reference for design intent — scope, risks, branch options — which does
not change every sprint.

### 2026-04-15 — Post-S2 infra: cargo-deny, Dependabot, schema drift, coverage (Claude)

Five small commits on `infra/cargo-deny-and-coverage`, landing the
non-tracking items from the post-merge recommendations (item #2, the
GitHub Project board + per-sprint issues, deferred to a separate
session).

- **`cargo-deny`** (`8e3c170`) — `deny.toml` covers advisories (block
  on any RUSTSEC), licenses (allow-list of ~12 permissive SPDX
  expressions + `[licenses.private] ignore = true` so our own
  `publish = false` "Proprietary" crates are skipped), bans
  (`multiple-versions = "warn"` for the benign `getrandom` 0.2/0.3
  duplicate, `wildcards = "deny"` with `allow-wildcard-paths = true`
  so `{ workspace = true }` is permitted), sources (crates.io only).
  CI job installs via `taiki-e/install-action` to avoid the 2-minute
  compile-from-source cost.
- **Dependabot** (`ca0b5c9`) — weekly Monday 09:00 UTC for cargo +
  github-actions. Patch+minor bumps grouped into one PR per ecosystem;
  majors come through individually because the determinism-critical
  crates (jsonschema, fixed, rand_xoshiro) warrant per-bump review.
- **Schema-drift guards** (`ac9bece` for channels, `0025505` for
  primitives) — integration tests that walk
  `documentation/schemas/examples/` and `primitive_vocabulary/` at
  test time via `std::fs` from `CARGO_MANIFEST_DIR`. Every `.json`
  file must parse via the runtime loader. Second test in each crate
  pins the embedded `include_str!` schema byte-for-byte against the
  canonical file on disk. The primitive-side test also asserts the
  starter-vocabulary count is exactly 16 so adding/removing a
  primitive is a reviewer-visible change.
- **Coverage reporting** (`37ffa31`) — new `coverage` job runs
  `cargo-llvm-cov` with `--summary-only` (per-crate numbers in log)
  and `--lcov --output-path lcov.info` (uploaded as `coverage-lcov`
  artifact with 14-day retention). Report-only to start; enabling
  `--fail-under-lines 80` is a one-line follow-up once we have a
  baseline.

- **Issue + PR templates** (`2c42b04`, issue templates commit,
  PR templates commit) — five issue YAML forms (feature_request,
  feature_task, bug_report, security, determinism_regression) with
  `config.yml` disabling blank issues and linking to CONTRIBUTING.md
  and the private-advisory form. Default `pull_request_template.md`
  for generic PRs plus four specialised templates (feature.md,
  bug_fix.md, security.md, determinism.md) activated via
  `?template=<name>.md`. `SECURITY.md` at the repo root establishes
  private-disclosure process with explicit SLAs.

Deferred to its own session: GitHub Project v2 board with per-sprint
issues.

### 2026-04-15 — Sprint S2 implementation (Claude)

Two new crates, both Layer 1, both depend only on `beast-core`
(`beast-primitives` also depends on `beast-channels` to share the
`ChannelFamily` enum and validate `channel_id` references).

**`beast-channels`** — 5 source files (~900 LOC) + 3 test files + README:
- `manifest.rs` — `ChannelManifest`, `ChannelFamily`, `MutationKernel`,
  `Range`, `ScaleBand`, `Provenance`, `CorrelationEntry`, `BoundsPolicy`.
  Two-stage loader: `RawChannelManifest` (serde f64/String mirror of the
  JSON schema) → semantic `into_manifest()` that converts every sim-math
  field to `Q3232`, checks range ordering, de-duplicates composition
  hook targets, and enforces the "threshold required for
  `kind ∈ {threshold, gating}`" rule.
- `composition.rs` — `CompositionHook`, `CompositionKind`, `HookOutcome`,
  `evaluate_hook()`. Formulas mirror the schema table (additive,
  multiplicative, threshold, gating, antagonistic). All Q3232.
- `expression.rs` — `ExpressionCondition` discriminated union,
  `ExpressionContext`, `evaluate_expression_conditions()` — empty slice
  always passes, missing context fields evaluate to `false`.
- `registry.rs` — `ChannelRegistry` over `BTreeMap<String, ChannelManifest>`
  + `BTreeMap<ChannelFamily, BTreeSet<String>>` for the family index.
  `validate_cross_references()` catches unknown hook targets and
  correlation targets after all manifests are loaded (literal `"self"` is
  always accepted).
- `schema.rs` — embeds the authoritative schema via `include_str!`
  (`../../../documentation/schemas/channel_manifest.schema.json`), caches
  the compiled `jsonschema::JSONSchema` in a `OnceLock`, flattens schema
  errors to `(pointer, message)` pairs.

**`beast-primitives`** — 7 source files (~1000 LOC) + 3 test files + README:
- `manifest.rs` / `schema.rs` / `registry.rs` mirror the channel crate's
  patterns. `validate_channel_references(&ChannelRegistry)` cross-checks
  `composition_compatibility.channel_id` entries against a live channel
  registry.
- `category.rs` — `PrimitiveCategory` (8 variants) + `Modality` (8
  variants). Both derive `Ord` so they can key BTreeMap indices.
- `math.rs` — Q3232 `q_ln` (artanh series via the
  `x = 2^k·m, m ∈ [1,2)` decomposition + `int_log2`), `q_exp` (Taylor on
  the `(-1, 1)` fractional part + integer scaling by `e^k`), and
  `q_pow(base, exp) = q_exp(exp · q_ln(base))`. Handles non-positive
  bases defensively. All 16 starter primitive manifests evaluate without
  `CostEvalError`.
- `cost.rs` — `evaluate_cost(&PrimitiveManifest, &BTreeMap)`. Resolves
  parameter values from the caller map, then the manifest's declared
  default (numeric only), then errors. Uses `q_pow` for the power term.
- `effect.rs` — `PrimitiveEffect` shape for the interpreter to emit in S4.
  Defined now so downstream crates compile against it.

**Determinism invariants maintained**:
- All numeric manifest values converted to `Q3232` at load time.
- Registries backed by `BTreeMap`/`BTreeSet` — sorted iteration verified
  in unit tests (`iteration_is_sorted`, `iteration_sorted_by_id`).
- `clippy::float_arithmetic = "warn"` at both new crate levels.
- Cost evaluator uses fixed-point exp/ln; Taylor loops have fixed
  iteration counts (no rounding-dependent early termination).

**Schema handling pragma**: the schema files declare
`$schema: ".../draft/2020-12/..."` but `jsonschema` 0.17 (the latest that
compiles on stable Rust 1.75) defaults to Draft 2019-09. The subset of
features we use (types, enums, patterns, required, if/then/else, oneOf)
is identical between the two drafts, so we let the validator use its
default. Upgrading `jsonschema` is tracked as a follow-up for when MSRV
moves past 1.77.

**Test count**: 74 beast-channels (31 unit + 3 example integration + 7
malformed + 3 doctest); 38 beast-primitives (25 unit + 3 example
integration + 8 malformed + 2 doctest). `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --all-targets --locked --doc`, and
`cargo build --workspace --release --locked` all pass locally on
Windows.

**Follow-ups for S3+**:
- The Q3232 `q_pow` / `q_exp` / `q_ln` helpers should be promoted to
  `beast-core::math` once the interpreter (S4) or physiology systems
  also want them; for now they stay `pub(crate)` to keep S2's API
  surface minimal.
- `validate_cross_references` on `ChannelRegistry` and
  `validate_channel_references` on `PrimitiveRegistry` will be wired
  into world init in S5/S6 when we actually load a full manifest bundle.

### 2026-04-15 — Infra & README (Claude)

After PR #1 merged, added:
- **PR #2 (merged)** — `.github/workflows/ci.yml` with four jobs (lint-and-test ubuntu, test windows, test macos, release-build ubuntu). `rust-toolchain.toml` pinning stable + rustfmt + clippy. `cargo fmt --all` normalisation pass over the existing code (import grouping + comment indentation; zero behaviour change).
- **PR #3 (merged)** — `.githooks/pre-push` (opt-in client-side push gate for master), `CONTRIBUTING.md`. Verified the hook works by attempting a direct push — exit 1 with the PR-workflow message.
- **PR #4 (this one, open)** — top-level `README.md` so future sessions (human or Claude) have a single entry point. PROGRESS_LOG remains the session-to-session diary; README is the project-level orientation.

**Server-side branch protection blocker**: `gh api repos/.../branches/master/protection -X PUT` responds 403 `Upgrade to GitHub Pro or make this repository public to enable this feature`. Options: (a) make the repo public, (b) GitHub Pro $4/mo. Workflow job names in `CONTRIBUTING.md` are the required-check names to plug in when either path is taken.

**Next action for S2**: scaffold `beast-channels` and `beast-primitives`. Entry ordering unchanged from previous note (manifests → composition hooks → registries → schema rejection tests). Validate channel and primitive manifests against the authoritative schemas in `documentation/schemas/`.

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
- `EntityId::NONE = u32::MAX`. `EntityIdAllocator::alloc` returns
  `Option<EntityId>` — `Some` for the first `u32::MAX - 1` calls, `None`
  thereafter. (Original saturating impl was a uniqueness bug; caught in PR #1
  review and fixed in commit `4765d39`.)
- `Prng::gen_range_i64` narrows via `i128` on the final add to handle spans
  wider than `i64::MAX` (e.g. `i64::MIN..i64::MAX`). Narrow proptest ranges
  had hidden this; fixed in the same PR #1 review commit.

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
- `crates/beast-core/tests/proptest_{fixed_point,prng,math}.rs` — property + 100k-sample tests (Story 1.5).
- `README.md` — top-level repo orientation (PR #4).
- `CONTRIBUTING.md` — workflow, hook activation, push policy (PR #3).
- `rust-toolchain.toml` — pins stable + rustfmt + clippy (PR #2).
- `.github/workflows/ci.yml` — GitHub Actions CI (PR #2).
- `.githooks/pre-push` — opt-in master-push gate (PR #3).
- `crates/beast-channels/` — Sprint S2 channel registry crate: `Cargo.toml`, `README.md`, `src/{lib,manifest,composition,expression,registry,schema}.rs`, `tests/{example_manifests,malformed_manifests}.rs`.
- `crates/beast-primitives/` — Sprint S2 primitive registry crate: `Cargo.toml`, `README.md`, `src/{lib,manifest,category,cost,effect,math,registry,schema}.rs`, `tests/{example_manifests,malformed_manifests}.rs`.

### GitHub-side tracking (live, not in-repo)
- **[Project board](https://github.com/users/BemusedVermin/projects/1)** — Sprint/Phase/Points/Status per item for all 18 sprints.
- **[Sprint epic issues](https://github.com/BemusedVermin/evolution-simulation/issues?q=is%3Aissue+label%3Aepic)** (#13–#30) — one per sprint, with story checklists.
