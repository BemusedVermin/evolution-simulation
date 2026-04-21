# Beast Evolution Game

A deterministic evolution-simulation game in Rust. Genomes mutate, phenotypes express against biomes, creatures interact, and a separate "Chronicler" layer observes emergent behaviour and names it — sim code emits only *primitive effects* and never hand-authored ability names. Every tick is replayable bit-for-bit from a seed plus input journal.

**Status**: Sprints S1 (fixed-point + PRNG) and S2 (manifests + registries) complete. Sprint S3 (genome + mutation) is next.

## Live project tracking — on GitHub

Sprint/story status, work-in-progress, and the up-to-date roadmap live on GitHub, **not** in the markdown planning docs. Anything here about "current sprint" or "story status" in `documentation/` is a snapshot for context; the board and issues are the source of truth.

- **Project board**: https://github.com/users/BemusedVermin/projects/1 — 18 sprint epics with Sprint, Phase, Points, and Status fields. Filter by Phase or Sprint for focused views.
- **Sprint epics**: [`label:epic`](https://github.com/BemusedVermin/evolution-simulation/issues?q=is%3Aissue+label%3Aepic) — one tracker issue per sprint (S1–S18) with goal, story checklist, demo criteria, and DoD.
- **Current sprint issues**: filter by `label:sprint:sN` (e.g. [`sprint:s3`](https://github.com/BemusedVermin/evolution-simulation/issues?q=is%3Aopen+label%3Asprint%3As3)).
- **Open a story issue**: use the "Feature task" GitHub issue template, label it `story` + `sprint:sN` + the relevant `crate:*`, and reference the sprint epic.

The planning markdown under `documentation/planning/` (EPICS.md, SPRINTS.md, RISK_REGISTER.md) captures *design intent*: scope, risks, branch options, dependency graph. It is not edited to reflect day-to-day status.

## Quick orientation

If you're picking this up fresh (human or Claude session), read in this order:

1. **[Project board](https://github.com/users/BemusedVermin/projects/1)** and the **[sprint epic for the active sprint](https://github.com/BemusedVermin/evolution-simulation/issues?q=is%3Aissue+label%3Aepic)** — what's actually in flight right now.
2. **`documentation/PROGRESS_LOG.md`** — session-to-session handoff diary: decisions taken, pitfalls, commit references. Historical; the GitHub board tracks live status.
3. **`CLAUDE.md`** — repo conventions, tooling, invariants called out for Claude Code.
4. **`documentation/INVARIANTS.md`** — the load-bearing contract (determinism, mechanics-label separation, registry monolithicism, scale-band unification, UI-vs-sim state). Violating any of these is a bug regardless of what else looks right.
5. **`documentation/architecture/IMPLEMENTATION_ARCHITECTURE.md`** — primary architecture doc (stack, tradeoffs, data flow).
6. **`documentation/architecture/CRATE_LAYOUT.md`** — all 17 planned crates, strict L0→L6 layering, inter-crate dependency DAG.
7. **`documentation/architecture/ECS_SCHEDULE.md`** — 8-stage tick loop, per-stage parallelism, RNG-stream rules, per-system performance budget.
8. **`documentation/systems/01_*.md` … `23_*.md`** — design spec per game system. Consult the specific file before implementing its crate.
9. **`documentation/schemas/`** — authoritative JSON schemas for channel and primitive manifests. Mods and core data must validate against these.
10. **`documentation/planning/`** — `IMPLEMENTATION_PLAN.md`, `EPICS.md`, `SPRINTS.md`, `RISK_REGISTER.md`. Sprint-level *scope and sequencing* (design intent); live status lives on the GitHub board.

`documentation/Beast_Evolution_Game_Master_Design.docx` is the original design doc — prefer the markdown when possible.

## Tech stack

| Layer       | Choice                                                    |
|-------------|-----------------------------------------------------------|
| Language    | Rust (stable, pinned via `rust-toolchain.toml`)           |
| Sim math    | `fixed::I32F32` (Q32.32) wrapped in `beast_core::Q3232`   |
| PRNG        | `rand_xoshiro::Xoshiro256PlusPlus`, one stream per subsystem |
| ECS         | `specs` (planned, lands in S5)                            |
| Graphics    | SDL3 (planned, lands in S9)                               |
| Serde       | `serde` + `bincode` for saves, `serde_json` for manifests |
| Property tests | `proptest`                                             |
| Benches     | `criterion`                                               |

**Float arithmetic is forbidden in sim state** (lint `clippy::float_arithmetic = "warn"` at crate level, `#[allow]`'d only for the one sanctioned use: `gaussian_q3232`'s Box–Muller transform). Render/UI code may use floats freely.

## Workspace layout

```
.
├── Cargo.toml                 # workspace root
├── rust-toolchain.toml        # stable + rustfmt + clippy
├── crates/
│   └── beast-core/            # L0 foundations: Q3232, Prng, EntityId, TickCounter, math
│                              # (other 16 crates are scaffolded per sprint; see CRATE_LAYOUT.md)
├── .github/workflows/ci.yml   # fmt, clippy, test, doc, release build, cross-platform
├── .githooks/pre-push         # opt-in hook, blocks direct pushes to master
├── CONTRIBUTING.md            # workflow, hook activation, push policy
├── CLAUDE.md                  # instructions specific to Claude Code sessions
└── documentation/             # design docs (authoritative), progress log, planning
```

Planned crates (added one per sprint, never pre-stubbed):

`beast-channels`, `beast-primitives`, `beast-genome`, `beast-interpreter`, `beast-evolution`, `beast-disease`, `beast-ecs`, `beast-sim`, `beast-chronicler`, `beast-serde`, `beast-render`, `beast-ui`, `beast-audio`, `beast-mod`, `beast-cli`, `beast-app`.

## Getting started

### First-time setup

```bash
# Clone and activate repo-tracked hooks (one-time per clone)
git clone https://github.com/BemusedVermin/evolution-simulation.git
cd evolution-simulation
git config core.hooksPath .githooks
```

`rustup` reads `rust-toolchain.toml` automatically; no manual toolchain selection needed.

### Build & test

```bash
cargo build --workspace
cargo test --workspace --all-targets
cargo test --workspace --doc
```

### The checks CI runs on every PR

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
cargo build --workspace --release --locked
```

CI also runs `cargo test` on `windows-latest` and `macos-latest` as a cross-platform determinism sanity check.
It also runs `quality-metrics (ubuntu)`, a metric-based maintainability pass on production Rust code that tracks per-function complexity, duplicate-rate budget, crate/module fan-in-fan-out coupling, and Rustdoc coverage.

### Benchmarks

```bash
cargo bench -p beast-core --bench core_bench
```

See `crates/beast-core/README.md` for measured baseline numbers.

## Workflow

- `master` is the integration branch. **All** changes land via PR.
- Open a topic branch (`sprint-sN-scope`, `fix-...`, `docs-...`, `ci-...`), push it, and `gh pr create --base master`.
- Direct pushes to `master` are blocked client-side by `.githooks/pre-push` (once activated). Server-side branch protection is not yet on — see `CONTRIBUTING.md` for the reason and the fix.
- CI must be green before merging. The job names are stable and listed in `CONTRIBUTING.md`.
- Commit prefixes: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci`.

## Non-negotiables

These come from `documentation/INVARIANTS.md`. In short:

1. **Determinism**: bit-identical replay across 1000+ ticks is a CI gate once `beast-sim` exists (Sprint S6). Q32.32 everywhere on the sim path; one PRNG stream per subsystem, split from a single master seed; sorted iteration in hot loops; tick-count time only, no wall-clock reads; no OS RNG.
2. **Mechanics–label separation**: systems emit primitives; the Chronicler names patterns. No hardcoded ability names in systems 01–20.
3. **Channel registry monolithicism**: a single runtime registry; no hardcoded channel IDs in system code.
4. **Emergence closure**: every observable behaviour traces back to a primitive emission.
5. **Scale-band unification**: one genome/interpreter pipeline covers macro hosts and micro pathogens.
6. **UI state vs. sim state**: derived UI flags never appear in save files.

## License

Proprietary — see the workspace `Cargo.toml`. Not yet OSS.
