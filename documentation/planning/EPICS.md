# Beast Evolution Game: Epic Definitions & Tracking

This document defines the 14 epics with acceptance criteria, dependencies, and tracking metrics.

> **Live epic status is on GitHub — not in this file.**
>
> The `**Status**:` lines below reflect initial planning and are kept as historical scope. They are **not** updated as work progresses.
>
> For current status see the **[Project board](https://github.com/users/BemusedVermin/projects/1)** and the **[Sprint epic issues](https://github.com/BemusedVermin/evolution-simulation/issues?q=is%3Aissue+label%3Aepic)**. Sprint-level rollup on the board maps 1:1 to the epics defined here.

---

## E1: Core Foundations (L0)
**Status**: Not Started | **Confidence**: High

**Definition**: Establish beast-core crate with deterministic numerical primitives.

**Components**:
- Q3232 fixed-point arithmetic
- Xoshiro256PlusPlus PRNG
- EntityID, TickCounter, Error types
- Math utilities (Gaussian Box-Muller, saturating operations)

**Business Value**: Determinism is non-negotiable. All downstream code depends on bit-identical arithmetic.

**Dependencies**: None

**Estimate**: 24 points

**Success Criteria**:
- [ ] Q3232 fixed-point multiply: 1000 samples produce bit-identical output when seeded
- [ ] PRNG: same seed produces identical sequence over 1M samples
- [ ] EntityID comparison is deterministic and consistent
- [ ] No panics on overflow; all ops saturate
- [ ] All unit tests pass; property-based tests (100k samples) pass with p > 0.95

**Definition of Done**:
- [ ] beast-core crate compiles without warnings
- [ ] All 4 numeric types fully tested
- [ ] Benchmarks baseline: Q3232 multiply < 2 CPU cycles
- [ ] Documentation complete with examples

**Risks**:
- PRNG period shorter than expected (mitigation: compare against reference impl)
- Fixed-point rounding errors accumulate over 1000+ ticks (mitigation: property tests catch this)

---

## E2: Channel & Primitive Registries (L1)
**Status**: Not Started | **Confidence**: High

**Definition**: Load and validate channel/primitive manifests; expose queryable registries.

**Components**:
- Channel manifest schema (JSON)
- Primitive manifest schema (JSON)
- ChannelRegistry (in-memory, indexed)
- PrimitiveRegistry (in-memory, indexed)
- Composition hook parser and evaluator
- Schema validation against JSON Schema

**Business Value**: Enables modding (load custom manifests), genesis (runtime channel creation), and schema enforcement (prevent corrupted registries).

**Dependencies**: E1 (beast-core)

**Estimate**: 32 points

**Success Criteria**:
- [ ] Load core/channels.json (18 channels) without errors
- [ ] Load core/primitives.json (25 primitives) without errors
- [ ] Query by ID returns expected channel
- [ ] Query by family (e.g., Sensory) returns expected subset
- [ ] Composition hook evaluation produces expected numeric results
- [ ] Malformed manifest rejected with clear error message

**Definition of Done**:
- [ ] beast-channels and beast-primitives crates compile
- [ ] All registries queryable and indexed
- [ ] Schema validation enabled; 5 malformed manifests rejected correctly
- [ ] Composition hook evaluation deterministic (same input → same output)

**Risks**:
- JSON schema complexity (mitigation: start simple; extend iteratively)
- Registry lookup performance (mitigation: use HashMap; benchmark if > 100 channels)

---

## E3: Genome & Mutation Operators (L1)
**Status**: Not Started | **Confidence**: Medium

**Definition**: Implement evolvable genotypes and mutation operators.

**Components**:
- Genome struct (variable-length gene list)
- TraitGene struct (effect vector, body site, regulatory modifiers)
- BodySite model (location on creature)
- Point mutation operator (Gaussian drift)
- Regulatory modifier rewiring (add/remove/strengthen links)
- Gene duplication & genesis (channel paralog creation)
- Gene loss and silencing operators

**Business Value**: Core evolutionary mechanism. Mutations drive open-ended adaptation.

**Dependencies**: E1, E2

**Estimate**: 36 points

**Success Criteria**:
- [ ] Mutate 1000 genomes 10 times each; all remain valid (effects in [0,1])
- [ ] Genesis operator produces valid paralog (unique ID, inherited manifest)
- [ ] Mutation kernel sample distribution matches expected Gaussian (KS test, p > 0.05)
- [ ] Regulatory rewiring produces valid regulatory networks (no circular infinite loops)
- [ ] Body-site shift produces valid location updates

**Definition of Done**:
- [ ] All mutation operators implemented
- [ ] Genome structure supports variable-length gene lists
- [ ] Point mutation, duplication, reclassification, loss all work
- [ ] Property-based tests validate mutation distribution

**Risks**:
- Genesis event complexity (mitigation: coordinate with E2 registries; test paralog lookup)
- Regulatory network instability (mitigation: limit network depth; test for divergence)

---

## E4: Phenotype Interpreter (L2)
**Status**: Not Started | **Confidence**: Medium

**Definition**: Convert genotype + environment → primitive effects deterministically.

**Components**:
- Scale-band filtering (gate channels by creature mass)
- Expression condition evaluator (biome, season, dev stage gates)
- Composition hook resolver (threshold, additive, multiplicative)
- Fixed-point parameter mapping (emit PrimitiveEffect from hooks)
- Body region tiling and aggregation
- Determinism validation framework

**Business Value**: Bridge between evolution and mechanics. Emergent behaviors arise here.

**Dependencies**: E1, E2, E3

**Estimate**: 48 points

**Success Criteria**:
- [ ] Interpret 100 random genomes; each produces valid PrimitiveEffect set
- [ ] 1000-tick determinism test: same seed → identical primitive outputs
- [ ] Scale-band test: 100kg creature with [1e-15, 1e-3kg] channel outputs zero
- [ ] Composition hook threshold test: hook fires iff both operands > T
- [ ] Zero operand → zero output (no errors on dormant channels)

**Definition of Done**:
- [ ] Interpreter fully functional
- [ ] All composition hook types work (threshold, additive, multiplicative, gating)
- [ ] Fixed-point arithmetic used throughout (no floats in hot path)
- [ ] All 1000 determinism test ticks produce bit-identical outputs
- [ ] No panics on any input

**Risks**:
- Fixed-point composition hook evaluation may overflow (mitigation: saturating arithmetic; edge case tests)
- Scale-band filtering edge cases (mitigation: property-based testing on mass ranges)

---

## E5: ECS Foundation (L3)
**Status**: Not Started | **Confidence**: High

**Definition**: Establish Entity-Component-System framework.

**Components**:
- EcsWorld wrapper (specs library integration)
- 15+ component types (Creature, Pathogen, Agent, Health, Position, etc.)
- System trait and SystemStage enum
- Resources struct (registries, PRNG streams, tick counter)
- Sorted entity index (deterministic iteration order)
- Component storage (dense vector + sparse index)

**Business Value**: Unified state representation. Enables deterministic iteration and parallel system scheduling.

**Dependencies**: E1, E2

**Estimate**: 40 points

**Success Criteria**:
- [ ] Create world with 1000 creatures; iteration order is always sorted entity ID
- [ ] All 15 component types defined without compilation errors
- [ ] Resources.rng_evolution and .rng_physics are independent streams
- [ ] System trait can be implemented and called on mock world

**Definition of Done**:
- [ ] ECS framework complete and integrated
- [ ] Deterministic iteration order verified (10 runs → same order)
- [ ] Parallel safety validated (no race condition test failures)
- [ ] All specs integration tests pass

**Risks**:
- specs library learning curve (mitigation: use well-documented examples)
- Sorted index overhead (mitigation: benchmark vs. unsorted; likely negligible)

---

## E6: Simulation Tick Loop & Determinism Guards (L4)
**Status**: Not Started | **Confidence**: Medium

**Definition**: Implement 8-stage ECS schedule with determinism validation.

**Components**:
- Simulation state machine
- SystemSchedule (8 ordered stages)
- Per-stage parallel dispatch (rayon)
- TickResult (state hash, per-stage timing)
- State hash computation (sorted iteration + XOR)
- Performance budget tracking

**Business Value**: Determinism is non-negotiable. Schedule defines the simulation contract.

**Dependencies**: E1, E3, E4, E5

**Estimate**: 44 points

**Success Criteria**:
- [ ] Run 100 ticks, save state hash after each tick
- [ ] Replay with same seed + inputs; all 100 hashes match original
- [ ] Schedule correctly orders stages (no inter-stage mutations out of order)
- [ ] Per-stage timing reported (budget tracking functional)

**Definition of Done**:
- [ ] 8-stage schedule fully implemented
- [ ] Determinism test passes for 100 ticks (on multiple seeds)
- [ ] Budget tracking functional; per-stage times logged
- [ ] Rayon parallelization within stages working

**Risks**:
- State hash collisions (mitigation: use XOR of all entity hashes; collision test)
- Rayon overhead (mitigation: profile; use work-stealing per-stage)

---

## E7: Save/Load & Replay Validation (L4)
**Status**: Not Started | **Confidence**: Medium

**Definition**: Implement persistence, replay validation, and save file validation.

**Components**:
- SaveFile struct (metadata + serialized state)
- SaveManager (save/load orchestration)
- ReplayJournal (input sequence)
- SaveValidator (schema validation, forbidden-key rejection)
- Migration system (upgrade old saves)

**Business Value**: Enables checkpoints, replays, and modding. Validator prevents UI state pollution.

**Dependencies**: E1, E5, E6

**Estimate**: 44 points

**Success Criteria**:
- [ ] Save world, load it back; world state unchanged
- [ ] Reject save with forbidden key `bestiary_discovered` (clear error message)
- [ ] Replay 100 ticks from saved state with same inputs; final state matches
- [ ] Load pre-versioned save file, apply migrations, run without error

**Definition of Done**:
- [ ] SaveFile & SaveManager complete
- [ ] SaveValidator rejects 5 malformed saves with clear errors
- [ ] Replay determinism test passes
- [ ] Migration system working

**Risks**:
- Serialization format brittleness (mitigation: version schema; plan migrations)
- Large save file sizes (mitigation: use bincode; optimize later if needed)

---

## E8: World Generation & Biome (L4)
**Status**: Not Started | **Confidence**: High

**Definition**: Generate playable archipelago with biomes and starter species.

**Components**:
- Procedural archipelago generation (Perlin noise, island biomes)
- Biome system (tundra, grassland, forest, volcanic, aquatic)
- BiomeCell component (resource density, hazard, season, climate)
- 3 starter species (genomes tuned for home biomes)
- Seed creature spawning (50 creatures across biomes)
- Climate model (temperature, precipitation, seasonal cycles)

**Business Value**: Playable world. Starter species enable immediate gameplay.

**Dependencies**: E1, E3, E4, E5, E6

**Estimate**: 40 points

**Success Criteria**:
- [ ] Generate 5 maps; each has 3+ distinct biome types
- [ ] Starter species survive 100 ticks in home biome (no extinction)
- [ ] Resource density varies spatially (measurable gradient)
- [ ] Season cycles every 1000 ticks (4 seasons)

**Definition of Done**:
- [ ] World generation complete and reproducible
- [ ] Starter species definitions finalized
- [ ] Climate model integrated
- [ ] No extinctions in 100-tick test

**Risks**:
- Biome spawning too dense (mitigation: tune carrying capacity; test)
- Seasonal changes too extreme (mitigation: tune modifiers; validate with survival test)

---

## E9: Rendering (World Map + Encounter) (L5)
**Status**: Not Started | **Confidence**: Medium

**Definition**: Implement SDL3 graphics pipeline for world map and encounter views.

**Components**:
- SDL3 initialization and window management
- Sprite atlas management
- World map renderer (top-down tiles, creature glyphs)
- Encounter view renderer (2.5D perspective, meshes)
- Visual directive → mesh/sprite pipeline
- Animation rigging (skeleton + periodic motion)

**Business Value**: Player sees the world. Procedural visuals ensure form-function coherence.

**Dependencies**: E3, E4, E5, E8

**Estimate**: 52 points

**Success Criteria**:
- [ ] Render world map at 60 FPS with 200 creatures visible
- [ ] Render encounter at 60 FPS with 5 creatures + terrain + particles
- [ ] 100 random creatures render without visual glitches
- [ ] Asymmetric directives produce asymmetric meshes

**Definition of Done**:
- [ ] Rendering pipeline complete
- [ ] 60 FPS maintained on all views
- [ ] All visual directives rendered correctly
- [ ] No degenerate meshes

**Risks**:
- Mesh generation complexity (mitigation: use simple primitives first; optimize later)
- Animation frame rate (mitigation: simple periodic functions; profile)

---

## E10: UI Layer & Chronicler Query API (L5)
**Status**: Not Started | **Confidence**: Medium

**Definition**: Implement widget framework, screens, and pattern labeling system.

**Components**:
- Widget trait and primitives (Button, List, Card, Dialog, Chart)
- WidgetTree and layout engine (flex-like)
- Event handling and data binding
- Screen definitions (WorldMap, Bestiary, Settings, Encounter)
- Chronicler pattern detection (primitive signature clustering)
- Label generation and confidence scoring
- QueryAPI (query labels, creatures with label)

**Business Value**: Player understands world. Emergent naming provides sense of discovery.

**Dependencies**: E5, E9

**Estimate**: 56 points

**Success Criteria**:
- [ ] Render complex UI screens at 60 FPS
- [ ] Bestiary displays all discovered creatures with labels
- [ ] Pattern detection on 1000-tick chronicle: identifies 5+ clusters
- [ ] Label confidence > 0.8 on known behaviors

**Definition of Done**:
- [ ] UI framework functional
- [ ] Chronicler detects patterns and assigns labels
- [ ] Bestiary search by label works
- [ ] QueryAPI complete

**Risks**:
- Label quality (mitigation: tune heuristics; manual testing)
- Clustering stability (mitigation: seeded random; property-test stability)

---

## E11: Combat & Primitive-Driven Mechanics (L4–L5)
**Status**: Not Started | **Confidence**: Medium

**Definition**: Implement formation, combat resolution, and combat UI.

**Components**:
- Keeper personality (charisma, neural_speed, empathy)
- Leadership budget system
- Formation structure (5 creature slots, position/exposure)
- Combat resolution (offense/defense from primitives)
- Predation and parasitism mechanics
- Formation disruption and movement constraints
- Combat readout UI

**Business Value**: Combat is the primary mechanic. Simulation-first design ensures emergent challenge.

**Dependencies**: E3, E4, E5, E6, E10

**Estimate**: 48 points

**Success Criteria**:
- [ ] Encounter with 5 friendly, 3 enemy creatures: resolve 10 rounds deterministically
- [ ] Damage computed from creature primitives (no hardcoded values)
- [ ] Disrupted creature cannot perform certain actions
- [ ] Leadership capacity varies with Keeper stress

**Definition of Done**:
- [ ] Combat system complete
- [ ] Formation mechanics validated
- [ ] Readout UI functional
- [ ] No crashes in 50-round combat test

**Risks**:
- Combat balance (mitigation: playtest; tune damage formula)
- Keeper stress mechanics feel boring (mitigation: visual feedback; iterate)

---

## E12: Chronicler & Emergent Labeling (L4)
**Status**: Not Started | **Confidence**: Medium

**Definition**: Extend Chronicler with event recording and label assignment.

**Components**:
- Event recording (creature lifecycle, primitive emissions)
- Primitive signature clustering
- Label generation heuristics
- Confidence scoring
- QueryAPI integration
- Chronicler persistence

**Business Value**: Player observes emergence. Labeling creates sense of discovery.

**Dependencies**: E4, E10

**Estimate**: 32 points

**Success Criteria**:
- [ ] Chronicle 1000 ticks; detect 5+ unique primitive clusters
- [ ] Assign labels with confidence > 0.7
- [ ] QueryAPI returns creatures by label

**Definition of Done**:
- [ ] Event recording complete
- [ ] Chronicler detects patterns
- [ ] Labels assigned with confidence
- [ ] UI integration works

**Risks**:
- Label quality (mitigation: domain expert review; iterate heuristics)
- Chronicler performance (mitigation: run every 100 ticks; amortize cost)

---

## E13: MVP Integration & Polish (L6)
**Status**: Not Started | **Confidence**: High

**Definition**: Integrate all subsystems into a playable whole; final testing and optimization.

**Components**:
- Main menu and state machine
- New game initialization
- Input handling (keyboard, mouse)
- Settings UI and persistence
- Game state transitions (save/load/pause/resume)
- Full determinism validation (1000-tick replay)
- Performance profiling and optimization
- Build & CI integration
- Documentation
- MVP demo spec

**Business Value**: Ship MVP. Players can play start-to-finish.

**Dependencies**: E1–E12

**Estimate**: 52 points

**Success Criteria**:
- [ ] Start game, play 100 ticks, save, close, load, play 100 ticks, all state matches
- [ ] All systems run within per-stage budget (no overruns > 5%)
- [ ] CI passes: cargo build (release), all tests, clippy (no warnings)
- [ ] README documents how to build, run, save/load, and report bugs

**Definition of Done**:
- [ ] MVP feature-complete and stable
- [ ] Determinism validated to 1000+ ticks
- [ ] All major bugs fixed
- [ ] Ready to branch for deep systems

**Risks**:
- Integration regressions (mitigation: run full integration tests frequently)
- Performance not meeting budget (mitigation: profile aggressively; may need cuts)

---

## E14: Deep System — Branching Post-MVP
**Status**: Not Started | **Confidence**: Medium

**Definition**: One of four deep systems (Evolution, Disease, Economy, Culture).

**Components** (vary by system):
- A: Sexual reproduction, regulatory feedback, genetic algorithms
- B: Pathogens, SEIR dynamics, host-pathogen coevolution
- C: Settlements, trading, economic specialization
- D: Language, cultural drift, historiography

**Business Value**: Replayability and depth. Each system unlocks new gameplay dimensions.

**Dependencies**: All MVP epics (E1–E13)

**Estimate**: 150 points (branching)

**Success Criteria**:
- [ ] Chosen deep system fully integrated and playable
- [ ] New gameplay mechanics tested at 1000-tick scale
- [ ] New content visible in game world

**Definition of Done**:
- [ ] Deep system implemented and tested
- [ ] Determinism preserved with new systems
- [ ] Updated documentation

**Risks**:
- Scope creep (mitigation: define scope tightly; track burn-down)
- Deep system interactions with existing systems (mitigation: early integration testing)

---

## Epic Tracking Matrix

| Epic | Sprints | Points | Lead | Status | Blockers |
|------|---------|--------|------|--------|----------|
| E1 | S1 | 24 | Solo+Claude | Not Started | None |
| E2 | S2 | 32 | Solo+Claude | Not Started | E1 |
| E3 | S3 | 36 | Solo+Claude | Not Started | E1, E2 |
| E4 | S4 | 48 | Solo+Claude | Not Started | E1, E2, E3 |
| E5 | S5 | 40 | Solo+Claude | Not Started | E1, E2 |
| E6 | S6 | 44 | Solo+Claude | Not Started | E1, E3, E4, E5 |
| E7 | S7 | 44 | Solo+Claude | Not Started | E1, E5, E6 |
| E8 | S8 | 40 | Solo+Claude | Not Started | E1, E3, E4, E5, E6 |
| E9 | S9 | 52 | Solo+Claude | Not Started | E3, E4, E5, E8 |
| E10 | S10 | 56 | Solo+Claude | Not Started | E5, E9 |
| E11 | S11 | 48 | Solo+Claude | Not Started | E3, E4, E5, E6, E10 |
| E12 | S12 | 32 | Solo+Claude | Not Started | E4, E10 |
| E13 | S13–S14 | 52 | Solo+Claude | Not Started | E1–E12 |
| E14 | S15–S18 | 150 | Solo+Claude | Not Started | E1–E13 |

---

## Health Tracking

**Velocity Target**: 40 points/sprint

**Actual Velocity** (to be updated weekly):
- S1: TBD
- S2: TBD
- ...

**Burn-Down**: See GitHub Issues for detailed sprint boards.

**Risk Escalation**: Any epic with > 10% velocity miss flagged for adjustment in next sprint planning.
