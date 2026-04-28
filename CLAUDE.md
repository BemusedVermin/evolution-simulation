# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Status

Sprints S1 (fixed-point + PRNG) and S2 (manifests + registries) are shipped. The Cargo workspace is live at the repo root; `crates/beast-core`, `crates/beast-channels`, and `crates/beast-primitives` are implemented. The other planned crates are scaffolded per-sprint — don't pre-stub them.

Design docs under `documentation/` remain authoritative for architecture and invariants; before writing code, read the relevant design doc and do not invent architecture that contradicts it.

## Live work tracking

Sprint and story status live on GitHub, not in the markdown planning docs.

- **Project board**: https://github.com/users/BemusedVermin/projects/1 (Sprint / Phase / Points / Status per item).
- **Sprint epics**: `label:epic` — one tracker issue per sprint (S1–S18) with the story checklist, demo criteria, DoD.
- **Story issues**: opened per-sprint using the Feature task template; labelled `story` + `sprint:sN` + `crate:*`; reference the sprint epic.

When the user asks "what's the current sprint?" or "what's next?", check the board or the open epic issues — **not** `SPRINTS.md`. The Status columns in `SPRINTS.md` and `EPICS.md` are historical scope and are not kept in sync. `documentation/PROGRESS_LOG.md` is a narrative diary (decisions, pitfalls, commits), not a status tracker.

## Project Summary

Beast Evolution Game is a deterministic evolution-simulation game planned in **Rust**. Target stack:

- **ECS**: `specs`
- **Graphics/Audio**: SDL3
- **Math**: `fixed` crate using Q32.32 fixed-point (`I32F32`) — **never `f32`/`f64` in sim state**
- **PRNG**: `rand_xoshiro` (Xoshiro256PlusPlus), one stream per subsystem
- **Serialization**: `serde` + `bincode` (deterministic config)
- **Planned workspace**: 17 base crates + 3 emergence-pillar crates (`beast-graph`, `beast-cog`, `beast-genesis` at L4 — see docs 56/57/58) under `crates/`, strictly layered L0 → L6.

The workspace `Cargo.toml` is at the repo root. When adding new crates, follow the dependency DAG in `CRATE_LAYOUT.md` — don't introduce cycles or skip layers.

## Documentation Map (read order)

1. **`documentation/INVARIANTS.md`** — the load-bearing contract (10 invariants). Read first, every session.
2. **`documentation/architecture/IMPLEMENTATION_ARCHITECTURE.md`** — primary architecture doc (stack, tradeoffs, data flow).
3. **`documentation/architecture/CRATE_LAYOUT.md`** — base crates + the three planned L4 emergence crates, layering rules, inter-crate deps.
4. **`documentation/architecture/ECS_SCHEDULE.md`** — 8-stage tick loop, per-stage parallelism and RNG-stream rules, per-system performance budget. Stage 1 and Stage 7 carry the new active-inference, community-detection, and channel-genesis sub-stages.
5. **`documentation/emergence/`** — the v2 emergence-first track. Reading order: 00 master synthesis → 30 biomes → 50 social → 56 relationship-graph → 57 agent AI → 58 channel genesis → 70 naming. Where v2 emergence pillars contradict the older `systems/01-23` specs, the emergence docs are authoritative.
6. **`documentation/systems/01_*.md` … `23_*.md`** — design specs for each game system. Several are subsumed by emergence docs (see headers on 03/04/06/08/17/22). Consult both before implementing.
7. **`documentation/schemas/`** — JSON schemas + examples for channel manifests and primitive vocabulary. The schema files (`channel_manifest.schema.json`, `primitive_manifest.schema.json`) are authoritative; mods and core data must validate against them.
8. **`documentation/planning/`** — `IMPLEMENTATION_PLAN.md`, `EPICS.md`, `SPRINTS.md`, `RISK_REGISTER.md`. Use for scope/sequencing; not needed for most implementation tasks.

`documentation/Beast_Evolution_Game_Master_Design.docx` is the source design doc — prefer the markdown files when possible.

## Non-Negotiable Invariants

These come from `INVARIANTS.md` (10 invariants total; full statements there). Violating any of them is a bug regardless of what else looks right.

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
7. **Groups Are Derived** (doc 56): factions / settlements / polities / guilds / households / packs are derived views over the relationship multigraph; never authoritative state on disk. Save = edges + carriers; cluster memberships recomputed on load.
8. **No Authored Relationship-Type or Group-Type Vocabulary** (doc 56): no `"kingdom"`, `"fealty"`, `"guild"`-class strings in sim path. Etic vocabularies are JSON galleries (P7 layer-5 inputs only); per-population emic names live in `population.lexicon`.
9. **All Decision-Making Is Active Inference** (doc 57): every agent's actions come from a policy posterior produced by EFE planning over a discrete factored POMDP. No per-system FSM / behaviour tree / GOAP / utility-AI / LLM-driven decision logic in the sim path.
10. **Genesis Lineage Closure** (doc 58): every channel id matching `genesis:*` MUST resolve to a `genesis_event_log` entry with parent lineage; kernel/mod registries are read-only at runtime.

## Tick Loop (8 stages, sequential between stages, parallel within)

From `ECS_SCHEDULE.md`. Systems within a stage may run in parallel (rayon); stages run in order:

0. Input & Aging · 1. Genetics + Cognitive Perception (mutation, VMP perceptual update, ToM cache, MCTS-EFE planning) · 2. Phenotype Resolution · 3. Physics & Movement · 4. Interaction & Combat (action sampling from policy posterior, primitive emission, RelationshipEdge update, latent-pressure update) · 5. Physiology · 6. Ecology · 7. Labeling, Emergence & Persistence (edge decay, link-community + hierarchical Leiden, cluster characterisation, channel-genesis pipeline, Chronicler etic-gallery match, P7 phonotactic-gen + iterated-learning, save) · 8. Render Prep.

Per-tick budget: ~16ms (60 FPS). When adding a system, place it in the correct stage and stay within its allotted slice; overruns defer to the next tick rather than blowing the budget. Stage-7 substages run on registered cadences (e.g., `CADENCE_LEIDEN_L2 = 1024` ticks); cost is amortised across ticks.

## Working with Channels and Primitives

- Channels are defined by JSON manifests validated against `documentation/schemas/channel_manifest.schema.json`. Family is mandatory and cannot be inferred.
- `provenance` must match `^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$`. Doc 58 extends this to accept the four-component form `genesis:<src_pop>:<tick>:<kind>:<sig_hash>` for runtime-born channels with composition or latent-extraction lineage; the existing two-component form remains valid for mutation-only events.
- Primitives are the 8-category output vocabulary of the phenotype interpreter. See `documentation/schemas/README.md` for the taxonomy and `primitive_vocabulary/` for the 16 starter primitives. Doc 56 adds a `target_kind` tag (`self` / `broadcast` / `pair` / `group`) — pair-targeted primitives drive the relationship-graph engine.
- When adding a channel or primitive, validate the manifest against the schema before wiring it in.
- New per-channel-kind registries planned by docs 57 and 58: `factor_manifest`, `preference_channel_manifest`, `action_skill_manifest`, `composition_operator`, `relationship_label_gallery`, `cluster_label_gallery`, `genesis_event` — all follow the same JSON-manifest pattern.

## Scoping

This repository will go through many iterations. We have many planned features that we have decided to push to a later date. If you encounter ANY scoped features, you must create a GitHub issue to track it in the future.

## Commands

The checks CI runs on every PR (runnable locally):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
cargo build --workspace --release --locked
cargo deny check
```

The planned determinism gate (enforced once `beast-sim` lands in S6):

```bash
cargo test --test determinism_test -- --nocapture
```

A failure prints a binary diff of the first diverging entity/component at the first diverging tick; investigate numerical precision, PRNG seeding, or iteration order (in that order).

## Conventions for Edits

- Documentation is the source of truth right now. If a task requires changing a design decision, update the relevant doc in the same change — don't let code and specs drift.
- Docs cross-reference by relative path (e.g., `/architecture/ECS_SCHEDULE.md`). Preserve that when moving files.
- The docs use `architecture/`, `planning/`, `systems/`, `schemas/` as stable top-level sections under `documentation/`. Don't reorganize without updating every README index.
