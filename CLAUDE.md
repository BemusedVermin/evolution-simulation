# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Status

This repo is **design-phase only** — no source code, Cargo workspace, or build tooling exists yet. It contains planning and specification documents that define an unimplemented project. Before writing code, read the design docs; do not invent architecture that contradicts them.

The only top-level directory is `documentation/`. All substantive content lives there.

## Project Summary

Beast Evolution Game is a deterministic evolution-simulation game planned in **Rust**. Target stack:

- **ECS**: `specs`
- **Graphics/Audio**: SDL3
- **Math**: `fixed` crate using Q32.32 fixed-point (`I32F32`) — **never `f32`/`f64` in sim state**
- **PRNG**: `rand_xoshiro` (Xoshiro256PlusPlus), one stream per subsystem
- **Serialization**: `serde` + `bincode` (deterministic config)
- **Planned workspace**: 17 crates under `crates/` (see `documentation/architecture/CRATE_LAYOUT.md`), strictly layered L0 → L6.

There is no `Cargo.toml` yet. When scaffolding, follow the crate dependency DAG in `CRATE_LAYOUT.md` — don't introduce cycles or skip layers.

## Documentation Map (read order)

1. **`documentation/INVARIANTS.md`** — the load-bearing contract. Read first, every session.
2. **`documentation/architecture/IMPLEMENTATION_ARCHITECTURE.md`** — primary architecture doc (stack, tradeoffs, data flow).
3. **`documentation/architecture/CRATE_LAYOUT.md`** — all 17 crates, layering rules, inter-crate deps.
4. **`documentation/architecture/ECS_SCHEDULE.md`** — 8-stage tick loop, per-stage parallelism and RNG-stream rules, per-system performance budget.
5. **`documentation/systems/01_*.md` … `23_*.md`** — design specs for each game system (evolution, traits, combat, phenotype interpreter, serialization, UI, etc.). Consult the specific system doc before implementing its crate.
6. **`documentation/schemas/`** — JSON schemas + examples for channel manifests and primitive vocabulary. The schema files (`channel_manifest.schema.json`, `primitive_manifest.schema.json`) are authoritative; mods and core data must validate against them.
7. **`documentation/planning/`** — `IMPLEMENTATION_PLAN.md`, `EPICS.md`, `SPRINTS.md`, `RISK_REGISTER.md`. Use for scope/sequencing; not needed for most implementation tasks.

`documentation/Beast_Evolution_Game_Master_Design.docx` is the source design doc — prefer the markdown files when possible.

## Non-Negotiable Invariants

These come from `INVARIANTS.md`. Violating any of them is a bug regardless of what else looks right.

1. **Determinism**: bit-identical replay across 1000+ ticks is a CI gate. Implies:
   - All sim-state math in Q32.32; floats allowed *only* in render/UI code.
   - Xoshiro256PlusPlus seeded once at world creation; one stream per subsystem (no cross-contamination).
   - Iterate sorted entity keys in hot loops — no `HashMap`/`HashSet` iteration where order leaks into state.
   - No wall-clock reads; tick-count-based logic only.
   - No `std`/OS RNG anywhere.
2. **Mechanics-Label Separation**: sim code emits primitive effects only; named abilities ("echolocation", "pack hunting") never appear in systems 01–20 control flow. The Chronicler assigns labels post-hoc.
3. **Channel Registry Monolithicism**: a single runtime registry (core + mods + genesis). Never hardcode channel IDs or composition rules in system code — read from the registry.
4. **Emergence Closure**: every observable behavior must trace back to primitive emissions. No ghost mechanics.
5. **Scale-Band Unification**: one genome/interpreter pipeline across macro hosts and micro pathogens. No scale-specific branches.
6. **UI State vs. Sim State**: `bestiary_observations` is sim state (an integer); `bestiary_discovered` is *derived* at the UI layer. Save files must not contain derived UI flags.

## Tick Loop (8 stages, sequential between stages, parallel within)

From `ECS_SCHEDULE.md`. Systems within a stage may run in parallel (rayon); stages run in order:

0. Input & Aging · 1. Genetics · 2. Phenotype Resolution (sub-stages: scale-band filter → composition → interpreter) · 3. Physics & Movement · 4. Interaction & Combat · 5. Physiology · 6. Ecology · 7. Labeling & Persistence · 8. Render Prep.

Per-tick budget: ~16ms (60 FPS). When adding a system, place it in the correct stage and stay within its allotted slice; overruns defer to the next tick rather than blowing the budget.

## Working with Channels and Primitives

- Channels are defined by JSON manifests validated against `documentation/schemas/channel_manifest.schema.json`. Family is mandatory and cannot be inferred.
- `provenance` must match `^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$`.
- Primitives are the 8-category output vocabulary of the phenotype interpreter. See `documentation/schemas/README.md` for the taxonomy and `primitive_vocabulary/` for the 16 starter primitives.
- When adding a channel or primitive, validate the manifest against the schema before wiring it in.

## Commands

No build/test/lint commands exist yet — there is no code. Once the Cargo workspace is scaffolded, the planned determinism gate is:

```bash
cargo test --test determinism_test -- --nocapture
```

A failure prints a binary diff of the first diverging entity/component at the first diverging tick; investigate numerical precision, PRNG seeding, or iteration order (in that order).

## Conventions for Edits

- Documentation is the source of truth right now. If a task requires changing a design decision, update the relevant doc in the same change — don't let code and specs drift.
- Docs cross-reference by relative path (e.g., `/architecture/ECS_SCHEDULE.md`). Preserve that when moving files.
- The docs use `architecture/`, `planning/`, `systems/`, `schemas/` as stable top-level sections under `documentation/`. Don't reorganize without updating every README index.
