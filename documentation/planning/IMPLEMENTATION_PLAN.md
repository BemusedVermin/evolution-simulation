# Beast Evolution Game: Implementation Plan

## Executive Summary

**Total MVP Scope**: 480 points (~12 sprints, ~3 months solo + Claude)

**Deep System Scope** (post-MVP): 150 points (~3.75 sprints, ~1 month)

**Total Horizon**: ~630 points (~15–16 weeks)

**Sprint Cadence**: 40 points/sprint = 1 week part-time solo dev + Claude pair programming. Sprints are organized in phases:
- **Phase 1 (S1–S4)**: Foundations & Core Simulation (beast-core, PRNG, fixed-point, manifests)
- **Phase 2 (S5–S9)**: Evolution & ECS Wiring (genome, interpreter, ECS framework, tick loop)
- **Phase 3 (S10–S14)**: Simulation Polish & MVP Integration (determinism, save/load, UI, world generation)
- **Phase 4 (S15–S18)**: One Deep System (branching at S14 end; four options planned)

**MVP Definition** (at end of S14):
- Playable tick loop with 50+ creatures in a biome
- Deterministic replay validation (save 100 ticks → replay → bit-identical state hash)
- Working bestiary UI with discovered creatures and emergent labels (via Chronicler pattern detection)
- One full encounter (creature observation + basic combat demo)
- Functional save/load with schema validation
- World generation and player avatar movement on map
- Music/audio optional for MVP

**Deep System Branching** (post-MVP, S15+):
- **Evolution Depth**: Expanded mutation operators, deeper channel networks, genetic algorithms for optimal forms
- **Disease**: Pathogen-specific transmission networks, epidemiological dynamics (SEIR), host-pathogen coevolution
- **Economy**: Settlement trading, resource chains, economic specialization feedback on evolution
- **Culture**: Language emergence from communication channels, cultural drift, narrative historiography via Chronicler

---

## Epics (13 Total)

### E1: Core Foundations (L0)
**Title**: Fixed-Point PRNG & Core Math

**Description**: Establish beast-core crate with all foundational types used by other crates: Q3232 fixed-point wrapper, Xoshiro256PlusPlus PRNG, EntityID, TickCounter, error handling, and math utilities (Gaussian sampling, saturating operations).

**Business Value**: Determinism guarantee. No crate can depend on uncertain or platform-specific math.

**Dependencies**: None

**Estimate**: 24 points
- Q3232 fixed-point type & operations (6 pts)
- Xoshiro256PlusPlus seeding & stream splitting (6 pts)
- EntityID, TickCounter, error types (4 pts)
- Math utils (Gaussian Box-Muller, saturation, clamping) (4 pts)
- Unit tests: fixed-point overflow/underflow, PRNG determinism (4 pts)

**Definition of Done**:
- All 4 number types pass property-based tests (100k samples of arithmetic ops match reference implementations)
- PRNG seeding test: same seed → identical sequence over 1M samples
- Fixed-point multiply/divide tests verify bit-identical output
- No panics on overflow; all ops use saturating arithmetic

---

### E2: Channel & Primitive Registries (L1)
**Title**: Manifest Loading & Schema Validation

**Description**: beast-channels and beast-primitives crates. Load JSON manifests for channels and primitives; validate against schemas; expose queryable registries. Channel families (sensory, motor, metabolic, etc.), composition hooks, expression conditions, scale-band constraints.

**Business Value**: Enables modding and genesis (runtime channel creation). Manifest validation prevents corrupted registries.

**Dependencies**: E1 (beast-core)

**Estimate**: 32 points
- Channel manifest schema & loading (8 pts)
- Primitive manifest schema & loading (8 pts)
- ChannelRegistry & PrimitiveRegistry (queryable by id, family) (6 pts)
- Composition hook parsing & expression condition evaluation (6 pts)
- Manifest validation tests (4 pts)

**Definition of Done**:
- Load core manifests (18 core channels, 25 primitives) without errors
- Reject malformed manifests with clear error messages
- QueryAPI: `registry.by_family(Family::Sensory)` returns 4 channels
- Composition hook evaluation on sample hooks produces expected numeric outputs

---

### E3: Genome & Mutation Operators (L1)
**Title**: Genotype Structure & Evolutionary Operations

**Description**: beast-genome crate. Define TraitGene (effect vector, body site, regulatory modifiers), Genome (variable-length gene list), and Mutator (point mutation, regulatory rewiring, duplication, divergence, loss, silencing, body-site shift). Implement mutation operators for solo dev + Claude to extend later (deep system).

**Business Value**: Core of evolution. Mutations drive open-ended adaptation.

**Dependencies**: E1, E2

**Estimate**: 36 points
- Genome & TraitGene struct definitions (4 pts)
- Body-site model (anterior/posterior/lateral, coverage) (4 pts)
- Point mutation operator (Gaussian drift on effect values) (6 pts)
- Regulatory modifier rewiring (add/remove/strengthen regulatory links) (6 pts)
- Gene duplication & reclassification (genesis) (6 pts)
- Mutation tests: verify genome structure, mutation distributions (4 pts)

**Definition of Done**:
- Mutate 1000 genomes 10 times each; verify effects remain in [0,1]
- Genesis operator produces valid paralog (unique ID, inherited manifest)
- Mutation kernel sample distributions match expected Gaussians (KS test, p > 0.05)

---

### E4: Phenotype Interpreter (L2)
**Title**: Genotype → Primitive-Effect Pipeline

**Description**: beast-interpreter crate. Implement deterministic PhenotypeInterpreter: reads genome + environment (biome, season, creature mass), applies scale-band filtering (dormant channels → Q3232::ZERO), evaluates expression conditions, applies composition hooks, emits primitive effects. No floating-point math; all fixed-point.

**Business Value**: Bridge between evolution and mechanics. Emergent behaviors arise here.

**Dependencies**: E1, E2, E3

**Estimate**: 48 points
- Scale-band filter system (gate channels by creature mass) (6 pts)
- Expression condition evaluator (biome, season, dev stage, population density gates) (6 pts)
- Composition hook resolver (threshold, additive, multiplicative, gating) (8 pts)
- Fixed-point parameter mapping evaluator (emits PrimitiveEffect with parameters) (8 pts)
- Body region tiling (per-body-region channel aggregation) (6 pts)
- Determinism test: same genome+env → identical primitive set (6 pts)
- Test fixture: macro creature with micro-only channel → zero primitives (2 pts)

**Definition of Done**:
- Interpret 100 random genomes; each produces a PrimitiveEffect set (never fails)
- Scale-band test: 100kg creature with [1e-15, 1e-3kg] channel outputs zero
- Composition hook test: threshold hook fires iff both operands > T
- Fixed-point determinism: 1000 ticks at same seed → bit-identical primitive outputs

---

### E5: ECS Foundation (L3)
**Title**: Entity-Component-System Framework

**Description**: beast-ecs crate. Wrap specs World; define all component types (Creature, Pathogen, Agent, Faction, Settlement, Biome, Genome, Phenotype, HealthState, Position, etc.). Define System trait for all simulation systems. Create Resources struct (global state: registries, PRNG streams, tick counter). Deterministic entity ID generation and sorted iteration helpers.

**Business Value**: Unified state representation. Enables deterministic iteration and parallel system scheduling.

**Dependencies**: E1, E2

**Estimate**: 40 points
- EcsWorld wrapper & specs integration (4 pts)
- Component type definitions (marker + data components) (8 pts)
- System trait & SystemStage enum (4 pts)
- Resources struct (registries, PRNG streams, tick counter) (4 pts)
- Sorted entity index (BTreeMap of sorted entity IDs per entity type) (6 pts)
- Component storage adapters (sparse set, dense vector) (6 pts)
- Deterministic iteration test: same world state → same entity order (2 pts)

**Definition of Done**:
- Create world with 1000 creatures; iteration order is always sorted entity ID
- Define all 15 component types without compilation errors
- Resources struct initializes all PRNG streams with seed; streams are independent
- System trait implemented; can be called on mock world

---

### E6: Simulation Tick Loop & Determinism Guards (L4)
**Title**: 8-Stage ECS Schedule & Performance Budgeting

**Description**: beast-sim crate. Implement Simulation state machine, SystemSchedule (8 ordered stages), per-stage parallel dispatch via rayon. Implement TickResult (summary of one tick: state hash, event summaries, budget usage). Determinism validation: compute world-state hash via sorted entity iteration + XOR of component hashes.

**Business Value**: Determinism is non-negotiable. Schedule defines the simulation contract.

**Dependencies**: E1, E3, E4, E5

**Estimate**: 44 points
- Simulation struct & initialization (4 pts)
- SystemSchedule & system registration (6 pts)
- Per-stage parallel dispatch (via rayon) (6 pts)
- TickResult & performance budget tracking (4 pts)
- State hash computation (sorted iteration + XOR of entity hashes) (6 pts)
- 100-tick determinism test (same seed → identical hashes every tick) (6 pts)
- Tick budget profiling (per-system timing) (4 pts)

**Definition of Done**:
- Run 100 ticks, save state hash after each tick
- Replay with same seed + inputs; all 100 hashes match original
- Schedule correctly orders stages (no inter-stage mutations out of order)
- Budget reporting shows per-stage elapsed time

---

### E7: Save/Load & Replay Validation (L4)
**Title**: Deterministic Serialization & Save File Validation

**Description**: beast-serde crate. Implement SaveFile (wrapper: metadata + serialized world state), SaveManager (load/save orchestration), ReplayJournal (input sequence), SaveValidator (schema validation, forbidden-key rejection for UI-ephemeral state like bestiary_discovered). Ensure bit-identical replay when loading a save and replaying input journal.

**Business Value**: Enables checkpoints, replays, and modding. Validator prevents UI state pollution of sim state.

**Dependencies**: E1, E5, E6

**Estimate**: 44 points
- SaveFile struct & serialization (6 pts)
- SaveManager (save/load, schema versioning) (6 pts)
- ReplayJournal & replay validation (6 pts)
- SaveValidator (JSON schema, forbidden-key checks) (6 pts)
- Migration system (upgrade old saves to new schema) (6 pts)
- Determinism via replay test (save 100 ticks → load → replay 100 ticks → hashes match) (8 pts)

**Definition of Done**:
- Save world, load it back; world state unchanged
- Reject save with forbidden key `bestiary_discovered` (clear error message)
- Replay journal test: save state N → replay N ticks with same inputs → final state matches saved state
- Load pre-versioned save file, apply migrations, run 10 ticks without error

---

### E8: World Generation & Biome (L4)
**Title**: Procedural Map & Biome System

**Description**: Implement world generation: create archipelago map (island biomes: tundra, grassland, forest, volcanic, aquatic). Populate with initial species (3 starter species with sensory/motor/metabolic channels tuned for biomes). Create seed creatures. BiomeCell component (resource density, hazard intensity, season, climate state). Climate model (simplified Milankovitch + temperature/precipitation gradients).

**Business Value**: Playable world. Starter species enable immediate gameplay.

**Dependencies**: E1, E3, E4, E5, E6

**Estimate**: 40 points
- Procedural archipelago generation (Perlin noise, island biomes) (8 pts)
- Biome system & BiomeCell component (8 pts)
- Initial species definition (3 starter species + genomes) (6 pts)
- Seed creature spawning (populate biomes with initial creatures) (6 pts)
- Climate model (temperature, precipitation, season) (6 pts)
- Biome/seed species tests (verify species can survive starter biomes) (4 pts)

**Definition of Done**:
- Generate 5 archipelago maps; each has 3+ distinct biome types
- Starter species survive 100 ticks in their home biome without extinction
- Resource density varies spatially (measurable gradient)
- Season cycles every 1000 ticks (4 seasons)

---

### E9: Rendering (World Map + Encounter) (L5)
**Title**: SDL3 Graphics Pipeline & UI Renderer

**Description**: beast-render crate. Implement Renderer (SDL3 wrapper, canvas, sprite atlas, draw calls). RenderMode (WorldMap, EncounterView). WorldMapRenderer: top-down isometric terrain tiles, creature symbols, faction outposts. EncounterRenderer: 2.5D perspective, creature meshes, environment, particles. Visual pipeline (System 10): convert VisualDirectives → mesh/sprite/particles.

**Business Value**: Player sees the world. Procedural visual pipeline ensures form-function coherence.

**Dependencies**: E3, E4, E5, E8

**Estimate**: 52 points
- SDL3 initialization & window management (4 pts)
- Sprite atlas management (sprite loading, sheet packing) (6 pts)
- World map renderer (terrain tiles, creature glyphs, UI overlays) (8 pts)
- Encounter view renderer (creature meshes, environment, camera) (8 pts)
- Visual directive → mesh/sprite pipeline (Protrude, Harden, Colorize, etc.) (10 pts)
- Animation rigging (skeleton + periodic motion functions) (8 pts)
- Render tests (generate 100 random creatures, render without crashing) (2 pts)

**Definition of Done**:
- Render world map at 60 FPS with 200 creatures visible
- Render encounter at 60 FPS with 5 creatures + terrain + particles
- 100 random creatures render without visual glitches (mesh degenerate test)
- Camera transitions smoothly between map and encounter

---

### E10: UI Layer & Chronicler Query API (L5)
**Title**: Widget Framework & Pattern Labeling

**Description**: beast-ui crate. Implement WidgetTree (hierarchy of Widgets), Widget trait (layout, event, render), primitives (Button, List, Card, Dialog, Chart). Define UIState (screen state, open tabs, selections; NOT persisted to save file). Implement data binding to sim state. Screen definitions (WorldMapScreen, BestiaryScreen, SettingsScreen). Chronicler (pattern detection, signature clustering, label assignment). QueryAPI for UI to query discovered labels.

**Business Value**: Player understands world. Emergent naming (via Chronicler) provides sense of discovery.

**Dependencies**: E5, E9

**Estimate**: 56 points
- Widget trait & basic primitives (Button, List, Dialog) (8 pts)
- WidgetTree & layout engine (flex-like) (8 pts)
- Event handling & data binding (6 pts)
- Screen definitions (WorldMap, Bestiary, Settings, Encounter UI) (8 pts)
- Chronicler pattern detection (primitive signature clustering) (8 pts)
- Label generation & confidence scoring (6 pts)
- QueryAPI (query labels for signature, creatures with label) (6 pts)
- Bestiary screen test (display 50 discovered creatures with labels) (2 pts)

**Definition of Done**:
- Render complex UI screens at 60 FPS
- Bestiary displays all discovered creatures with emergent labels
- Pattern detection on 1000-tick chronicle: identifies recurring primitive clusters
- Label confidence > 0.8 on known behaviors (e.g., "biting" from high kinetic_force + jaw channel)

---

### E11: Combat & Primitive-Driven Mechanics (L4–L5)
**Title**: Formation & Combat Resolution System

**Description**: Implement combat encounter system. Keeper (player avatar) has personality traits (charisma, neural_speed, empathy) that derive leadership capacity. Formation (5 creature slots with position/exposure values). Combat resolution: creature interactions (predator-prey, parasitism) resolved via primitive effects (force, state induction, transmission). Combat readout computed fresh each turn (no lookup tables). Formation disruption and damage as parallel threat tracks.

**Business Value**: Combat is the primary mechanic. Simulation-first design ensures emergent challenge.

**Dependencies**: E3, E4, E5, E6, E10

**Estimate**: 48 points
- Keeper personality & leadership budget system (6 pts)
- Formation structure & position/exposure calculations (8 pts)
- Combat resolution (offense/defense from primitives, damage formula) (8 pts)
- Predation & parasitism mechanics (predator sensing, transmission success) (8 pts)
- Formation disruption & movement constraints (6 pts)
- Combat readout UI (health bars, ability highlights, formation diagram) (8 pts)
- Combat encounter test (resolve 10 rounds, verify no crashes/invalid states) (2 pts)

**Definition of Done**:
- Encounter with 5 creatures, 3 enemy creatures: resolve 10 rounds deterministically
- Damage formula depends on creature primitives, not hardcoded values
- Formation slot disruption prevents certain creature actions (e.g., no range attacks from disrupted back slot)
- Leadership capacity varies with Keeper stress level (measurable reduction under duress)

---

### E12: Chronicler & Emergent Labeling (L4)
**Title**: Pattern Recognition & Lore Generation

**Description**: Extend Chronicler system (E10) to record significant simulation events (creature birth, death, extinction, new phenotype discovery). Implement pattern clustering on primitive signatures. Label generation heuristics (high kinetic_force + jaw channels → "biter"). Confidence scoring. QueryAPI integration with UI for Bestiary, event log, and history.

**Business Value**: Player observes emergence. Labeling creates sense of discovery and biological understanding.

**Dependencies**: E4, E10

**Estimate**: 32 points
- Event recording (creature lifecycle, primitive emissions) (6 pts)
- Primitive signature clustering (K-means or hierarchical) (8 pts)
- Label generation heuristics (map primitive clusters to evocative names) (6 pts)
- Confidence scoring (frequency, stability over time) (4 pts)
- QueryAPI integration (bestiary search, creature filtering by label) (4 pts)
- Chronicler persistence (save/load event log) (2 pts)
- Chronicler test (1000-tick run, detect 5+ distinct emergent behaviors with confidence > 0.7) (2 pts)

**Definition of Done**:
- Chronicle 1000 ticks with 200 creatures
- Detect 5+ unique primitive clusters (e.g., echolocation, pack hunting, camouflage)
- Assign labels to clusters with confidence > 0.7
- UI query API returns creatures by label (e.g., "echolocation" returns 12 creatures)

---

### E13: MVP Integration & Polish (L6)
**Title**: Menu, Settings, Determinism Validation, Documentation

**Description**: Integrate all subsystems into a playable whole. Implement main menu (new game, load game, settings). Game state machine (menu → worldmap → encounter → save). Input handling (keyboard, mouse). Settings UI (difficulty, graphics quality, audio volume). Final determinism validation: replay 100 ticks from save file, compare all tick hashes. Package build artifacts.

**Business Value**: Ship MVP. Players can play start-to-finish.

**Dependencies**: E1–E12

**Estimate**: 52 points
- Main menu & state machine (6 pts)
- New game initialization (biome + starter species + Keeper) (6 pts)
- Input handling (keyboard, mouse, SDL3 events) (6 pts)
- Settings UI & persistence (4 pts)
- Game state transitions (save/load/pause/resume) (6 pts)
- Full determinism validation (100-tick replay, all hashes) (8 pts)
- Performance profiling & optimization (find/fix budget overruns) (8 pts)
- Build & CI integration (cargo test, clippy, format checks) (4 pts)
- Documentation (README, architecture guide, developer onboarding) (4 pts)
- MVP demo spec (script 5-minute playthrough showing all features) (2 pts)

**Definition of Done**:
- Start game, play 100 ticks, save, close, load, play 100 ticks, all state matches
- All systems run within per-stage budget (no overruns > 5%)
- CI passes: cargo build (release), all tests, clippy (no warnings)
- README documents how to build, run, save/load, and report bugs

---

### E14: Deep System — Branching Post-MVP (L2+)
**Title**: One of: Evolution Depth, Disease, Economy, Culture

**Description**: Placeholder epic. At end of S14, team chooses one deep system to develop (S15–S18):

1. **Evolution Depth**: Expand mutation operators, regulatory network complexity, genetic algorithms for optimal solutions
2. **Disease**: Pathogen-specific SEIR model, transmission networks, host-pathogen coevolution
3. **Economy**: Settlements, trading chains, crafting, economic specialization driving evolution
4. **Culture**: Language emergence, cultural drift, narrative historiography, NPC dialect variation

**Business Value**: Replayability and depth. Each system unlocks new gameplay dimensions.

**Dependencies**: All MVP epics (E1–E13)

**Estimate**: 150 points (3.75 sprints, branching)
- Branching plan detailed in Section 5 below

**Definition of Done**: 
- Chosen deep system fully integrated and playable
- New gameplay mechanics tested at 1000-tick scale
- New content (e.g., pathogens, settlements) visible in game world

---

## Sprint Plan (MVP: S1–S14)

### Sprint S1: Fixed-Point & PRNG (W1)
**Goal**: Establish deterministic numerical foundation.

**Stories** (40 pts):
- Story 1.1: Q3232 fixed-point type with add/sub/mul/div/saturate (8 pts)
  - AC: `Q3232(0.5) + Q3232(0.3) == Q3232(0.8)`; overflow saturates to max
  - Tasks: type definition, operator overloads, tests
  - Points: 8

- Story 1.2: Xoshiro256PlusPlus with seed & stream splitting (8 pts)
  - AC: seeding with same value → identical sequence over 1M samples
  - AC: stream_split() produces independent streams
  - Tasks: PRNG implementation, splitting logic, validation test
  - Points: 8

- Story 1.3: EntityID, TickCounter, custom error type (6 pts)
  - AC: EntityID(1) < EntityID(2); no key conflicts
  - AC: TickCounter increments deterministically
  - Tasks: newtype definitions, Ord/Eq derives, Error trait impl
  - Points: 6

- Story 1.4: Box-Muller Gaussian sampling & saturating math utils (6 pts)
  - AC: sample_gaussian(rng, mean=0, stddev=0.1) produces ~ 68% samples in [-0.1, 0.1]
  - Tasks: Box-Muller, saturation ops, clamping
  - Points: 6

- Story 1.5: All unit tests + property-based fuzz (6 pts)
  - AC: 100% pass rate; 100k property tests of arithmetic
  - Tasks: proptest integration, overflow edge cases, PRNG period test
  - Points: 6

- Story 1.6: Bench baseline + documentation (6 pts)
  - AC: Fixed-point multiply < 2 CPU cycles (measured)
  - Tasks: criterion.rs benchmarks, inline comments
  - Points: 6

**Demo Criteria**: Run determinism_test.rs; same seed produces identical PRNG output over 1M iterations. No panics on overflow/underflow.

**Exit DoD**: All 6 stories completed. beast-core lib published; no external deps on other beast crates.

**Known Risks**:
- PRNG period may be shorter than expected (risk: medium, mitigation: verify against Xoshiro reference impl)

---

### Sprint S2: Manifests & Registries (W2)
**Goal**: Load channels and primitives; enable modding.

**Stories** (38 pts):
- Story 2.1: Channel manifest schema & JSON loader (8 pts)
  - AC: Load core/channels.json (18 channels) without errors
  - Tasks: serde_json schema, file I/O, error handling
  - Points: 8

- Story 2.2: Primitive manifest schema & JSON loader (8 pts)
  - AC: Load core/primitives.json (25 primitives) without errors
  - Tasks: serde_json schema, primitive struct defs
  - Points: 8

- Story 2.3: ChannelRegistry with queryable indexing (6 pts)
  - AC: registry.by_family(Sensory) returns 4 channels
  - AC: registry.by_id("auditory_sensitivity") returns channel
  - Tasks: HashMap + index builders, query methods
  - Points: 6

- Story 2.4: PrimitiveRegistry with cost function evaluation (6 pts)
  - AC: Cost(primitive, parameter_map) returns valid f32
  - Tasks: registry struct, cost_function eval
  - Points: 6

- Story 2.5: Composition hook parser & evaluator (8 pts)
  - AC: Parse threshold hook; evaluate with sample operands; fires iff both > T
  - Tasks: hook struct def, expression parser, eval logic
  - Points: 8

- Story 2.6: Schema validation & rejection of malformed manifests (2 pts)
  - AC: Reject manifest with missing "id" field; clear error message
  - Tasks: jsonschema integration, validation test
  - Points: 2

**Demo Criteria**: Load manifests, query registries, evaluate 10 composition hooks. No crashes.

**Exit DoD**: All registries loaded and queryable. Schema validation rejects 5 malformed test manifests.

**Known Risks**:
- JSON schema complexity may require iterative refinement (risk: medium, mitigation: start with simple schema, extend as needed)

---

### Sprint S3: Genome & Mutation (W3)
**Goal**: Implement evolvable genotypes and mutation operators.

**Stories** (40 pts):
- Story 3.1: Genome & TraitGene data structures (6 pts)
  - AC: Genome holds 10–100 genes; each gene has channel_id, effect_vector, body_site, regulatory_mods
  - Tasks: struct defs, serde support
  - Points: 6

- Story 3.2: Body-site model (anterior/posterior/lateral, coverage, symmetry) (6 pts)
  - AC: BodySite specifies location on creature; coverage ranges [0,1]
  - Tasks: struct def, distribution enum
  - Points: 6

- Story 3.3: Point mutation operator (Gaussian drift on channel values) (8 pts)
  - AC: Mutate genome 1000 times; effect_vector values stay in [0,1]
  - AC: Mutation kernel sigma=0.08 produces expected sample distribution
  - Tasks: mutator struct, mutation logic, bounds enforcement
  - Points: 8

- Story 3.4: Regulatory modifier rewiring (add/remove regulatory links) (6 pts)
  - AC: Add regulatory link from gene A to gene B; remove link; verify structure
  - Tasks: regulatory update logic, link validation
  - Points: 6

- Story 3.5: Gene duplication & genesis (channel paralog creation) (8 pts)
  - AC: Duplication creates new gene with unique ID, inherited manifest, marked genesis event
  - Tasks: duplication logic, paralog ID generation, manifest lookup
  - Points: 8

- Story 3.6: Genome mutation tests & property tests (6 pts)
  - AC: 1000 mutations of 10 genomes; no panics, all values valid
  - Tasks: proptest cases, mutation distribution KS test
  - Points: 6

**Demo Criteria**: Mutate 100 random genomes. Verify mutation distributions and genesis events are valid.

**Exit DoD**: All mutation operators functional. Genome structure supports variable-length gene lists. No panics on mutation.

**Known Risks**:
- Genesis event complexity (creating new channels) may require careful registry sync (risk: medium, mitigation: coordinate with E2 registries)

---

### Sprint S4: Phenotype Interpreter (W4)
**Goal**: Convert genotypes to primitive effects deterministically.

**Stories** (48 pts):
- Story 4.1: Scale-band filtering (gate channels by creature mass) (6 pts)
  - AC: 100kg creature with [1e-15, 1e-3kg] channel outputs Q3232::ZERO
  - AC: Same creature with [10kg, 1000kg] channel outputs evolved value
  - Tasks: filter logic, mass computation
  - Points: 6

- Story 4.2: Expression condition evaluator (biome, season, dev stage gates) (6 pts)
  - AC: Tropical channel silent in tundra biome
  - AC: Winter coat channel active in winter, silent in summer
  - Tasks: condition struct, eval logic, test fixtures
  - Points: 6

- Story 4.3: Composition hook resolver (threshold, additive, multiplicative) (8 pts)
  - AC: Threshold hook fires iff both operands > T; output = operand1 * operand2
  - AC: Zero operand → zero output (no errors)
  - Tasks: hook evaluation loop, operator implementations
  - Points: 8

- Story 4.4: Fixed-point parameter mapping (emit PrimitiveEffect from hooks) (8 pts)
  - AC: Hook emits primitive with parameters: force = ch1 * ch2 * 1.5 (fixed-point)
  - Tasks: expression eval, primitive struct creation
  - Points: 8

- Story 4.5: Body region tiling & phenotype aggregation (6 pts)
  - AC: Interpret genome → list of BodyRegion with per-region channel values
  - Tasks: body map struct, aggregation logic
  - Points: 6

- Story 4.6: Determinism & fixture tests (8 pts)
  - AC: Same genome + env (1000 samples) → identical primitive outputs
  - AC: Macro creature with micro-only channel → zero primitives (no crash)
  - Tasks: test fixtures, determinism harness
  - Points: 8

- Story 4.7: Documentation & integration test (4 pts)
  - AC: Interpret 100 random genomes; each produces valid PrimitiveEffect set
  - Tasks: docstrings, integration test
  - Points: 4

**Demo Criteria**: Interpret 100 random genomes. All produce primitive effects. Scale-band filtering works. Determinism test passes.

**Exit DoD**: Interpreter fully functional. All 1000 determinism test ticks produce bit-identical outputs. No panics on any input.

**Known Risks**:
- Fixed-point composition hook evaluation may overflow (risk: high, mitigation: use saturating arithmetic, test edge cases)
- Scale-band filtering edge cases (risk: medium, mitigation: property-based testing on mass ranges)

---

### Sprint S5: ECS Foundation (W5)
**Goal**: Build Entity-Component-System framework.

**Stories** (40 pts):
- Story 5.1: EcsWorld wrapper & specs integration (4 pts)
  - AC: Create world, insert 100 entities, iterate in O(N)
  - Tasks: specs World wrapper, entity insertion
  - Points: 4

- Story 5.2: Component type definitions (Creature, Pathogen, Agent, Health, Position, etc.) (8 pts)
  - AC: All 15 component types defined with serde
  - Tasks: struct defs, derive macros
  - Points: 8

- Story 5.3: System trait & SystemStage enum (4 pts)
  - AC: Define System trait; implement for mock system; call on world
  - Tasks: trait definition, stage enum
  - Points: 4

- Story 5.4: Resources struct (registries, PRNG streams, tick counter) (4 pts)
  - AC: Resources.rng_evolution, .rng_physics independent; tick_counter increments
  - Tasks: resource struct, stream initialization
  - Points: 4

- Story 5.5: Sorted entity index (BTreeMap per entity type) (6 pts)
  - AC: Iterate 1000 creatures in ascending EntityID order every time
  - Tasks: BTreeMap index, iterator API
  - Points: 6

- Story 5.6: Component storage & parallelization safety (6 pts)
  - AC: Parallel system on 1000 creatures: no race conditions
  - Tasks: dense storage layout, test with rayon
  - Points: 6

- Story 5.7: Tests & determinism (8 pts)
  - AC: Same world state → same iteration order (10 runs)
  - Tasks: iteration order test, race condition tests
  - Points: 8

**Demo Criteria**: Create world with 1000 creatures. Iterate in sorted order. Define 5 mock systems. No panics or data corruption.

**Exit DoD**: ECS framework complete. Deterministic iteration order verified. Parallel safety validated (no race condition tests fail).

**Known Risks**:
- specs library learning curve (risk: low, mitigation: use well-documented examples)
- Sorted index overhead (risk: medium, mitigation: benchmark vs. unsorted; may be negligible)

---

### Sprint S6: Tick Loop & Determinism (W6)
**Goal**: Implement the 8-stage simulation schedule.

**Stories** (44 pts):
- Story 6.1: Simulation struct & initialization (4 pts)
  - AC: Create simulation with config; world & resources initialized
  - Tasks: struct def, init logic
  - Points: 4

- Story 6.2: SystemSchedule & system registration (6 pts)
  - AC: Register 8 stages; systems assigned to stages
  - Tasks: schedule struct, registration API
  - Points: 6

- Story 6.3: Per-stage parallel dispatch (rayon) (6 pts)
  - AC: Dispatch stage 1 (parallel per-creature); wait for completion; move to stage 2
  - Tasks: dispatch loop, rayon integration
  - Points: 6

- Story 6.4: TickResult & performance budget tracking (4 pts)
  - AC: TickResult includes state_hash, per_stage_elapsed_ms
  - Tasks: struct def, profiling hooks
  - Points: 4

- Story 6.5: State hash computation (sorted iteration + XOR) (6 pts)
  - AC: Hash all creatures + their components; XOR results
  - AC: Same world state → same hash
  - Tasks: hash function, iteration order enforce
  - Points: 6

- Story 6.6: 100-tick determinism test (save state, replay, hash compare) (12 pts)
  - AC: Save after tick 50; replay ticks 1–100; all hashes match original
  - Tasks: save/load (stub), replay harness, hash comparison
  - Points: 12

- Story 6.7: Tick budget profiling & early-exit on overrun (6 pts)
  - AC: Stage exceeds budget → skip next "cold" system
  - Tasks: budget check logic, deferred system logic
  - Points: 6

**Demo Criteria**: Run 100 ticks. Save at tick 50. Replay ticks 1–100. All tick hashes match. Per-stage timing printed.

**Exit DoD**: 8-stage schedule fully implemented. Determinism test passes for 100 ticks. Budget tracking functional. No system ordering violations.

**Known Risks**:
- State hash collisions (risk: low, mitigation: use XOR of all entity hashes; check for collisions in test)
- Rayon overhead (risk: low, mitigation: use work-stealing per-stage; profile)

---

### Sprint S7: Save/Load & Replay (W7)
**Goal**: Implement persistence and replay validation.

**Stories** (44 pts):
- Story 7.1: SaveFile struct & serialization (6 pts)
  - AC: Serialize world state to binary; header includes schema_version
  - Tasks: struct def, serde_json/bincode integration
  - Points: 6

- Story 7.2: SaveManager (save/load orchestration) (6 pts)
  - AC: Save world to "save.dat"; load it back; worlds are equal
  - Tasks: manager struct, I/O logic, error handling
  - Points: 6

- Story 7.3: ReplayJournal (log input sequence) (6 pts)
  - AC: Record player input each tick; serialize to JSON
  - Tasks: journal struct, input struct, I/O
  - Points: 6

- Story 7.4: SaveValidator (schema validation + forbidden-key rejection) (6 pts)
  - AC: Reject save with bestiary_discovered key; clear error
  - AC: Validate schema version matches or is migratable
  - Tasks: JSON schema, forbidden key check, validator struct
  - Points: 6

- Story 7.5: Migration system (upgrade old saves) (6 pts)
  - AC: Load schema v1.0 save; apply migration to v1.1; runs without error
  - Tasks: migration registry, apply_migrations() logic
  - Points: 6

- Story 7.6: Determinism via replay test (100 ticks, save, load, replay, hash compare) (10 pts)
  - AC: Save after 100 ticks; load; replay 100 ticks with same inputs; all hashes match original
  - Tasks: test harness, integration with E6 determinism test
  - Points: 10

- Story 7.7: Documentation & edge case tests (4 pts)
  - AC: Save/load with all component types; no panics
  - Tasks: docstrings, edge case tests (empty world, max creatures)
  - Points: 4

**Demo Criteria**: Save world after 100 ticks. Load it. Replay 100 ticks. All tick hashes match original. No forbidden keys in save.

**Exit DoD**: SaveFile & SaveManager complete. SaveValidator rejects 5 malformed saves with clear errors. Replay determinism test passes.

**Known Risks**:
- Serialization format brittleness (risk: medium, mitigation: version schema; plan for migrations)
- Large save file sizes (risk: low, mitigation: use bincode; optimize later if needed)

---

### Sprint S8: World Generation (W8)
**Goal**: Generate playable world with biomes and starter species.

**Stories** (40 pts):
- Story 8.1: Procedural archipelago generation (Perlin noise) (8 pts)
  - AC: Generate 5 maps; each has 3+ distinct biome types
  - AC: Map is 128×128 cells; islands separated by ocean
  - Tasks: Perlin noise impl, island biome assignment
  - Points: 8

- Story 8.2: Biome system & BiomeCell component (8 pts)
  - AC: Each biome cell has resource_density, hazard_intensity, climate_state, season
  - AC: Biome component defines temperature, precipitation, seasonal modifiers
  - Tasks: biome struct, BiomeCell component, resource model
  - Points: 8

- Story 8.3: Initial species definition (3 starter genomes for MVP biomes) (6 pts)
  - AC: Grassland species (herbivore-tuned), Forest species (omnivore), Tundra species (cold-adapted)
  - AC: Each genome has 10–15 genes tuned for home biome
  - Tasks: 3 starter genomes, channel selection for each
  - Points: 6

- Story 8.4: Seed creature spawning (populate biomes with 50 initial creatures) (6 pts)
  - AC: Spawn 50 creatures across biomes (10 per biome type)
  - AC: Creatures are instantiated with health=1.0, age=0
  - Tasks: spawning logic, position assignment
  - Points: 6

- Story 8.5: Climate model (temperature, precipitation, seasonal cycles) (8 pts)
  - AC: Temperature gradient: equator hot, poles cold
  - AC: Season cycles every 1000 ticks (spring, summer, fall, winter)
  - AC: Season affects biome resource density (summer: +20%, winter: -30%)
  - Tasks: climate struct, seasonal modifier logic
  - Points: 8

- Story 8.6: Tests (verify starter species survive 100 ticks; biome resources replenish) (4 pts)
  - AC: 100 ticks: no extinctions; resource density varies spatially
  - AC: Seasonal cycle completes: season goes spring → summer → fall → winter → spring
  - Tasks: world gen test, survival test, season test
  - Points: 4

**Demo Criteria**: Generate world. Spawn 50 creatures. Run 100 ticks. Creatures survive. Season cycles. Resources vary.

**Exit DoD**: World generation complete. Starter species definitions finalized. Climate model integrated. No extinctions in 100-tick test.

**Known Risks**:
- Biome spawning too dense (risk: medium, mitigation: tune carrying capacity per biome; test)
- Seasonal changes too extreme (risk: low, mitigation: tune modifier magnitudes; validate with survival test)

---

### Sprint S9: Rendering & Visual Pipeline (W9)
**Goal**: Implement 2D/2.5D rendering and procedural visuals.

**Stories** (52 pts):
- Story 9.1: SDL3 initialization & window management (4 pts)
  - AC: Create SDL3 window, canvas; 60 FPS game loop runs without tearing
  - Tasks: SDL3 init, Renderer struct, event loop
  - Points: 4

- Story 9.2: Sprite atlas management (6 pts)
  - AC: Load sprite sheet (512×512); pack 50 sprites; lookup by ID
  - Tasks: atlas struct, sprite loading, packing logic
  - Points: 6

- Story 9.3: World map renderer (terrain tiles, creature glyphs) (8 pts)
  - AC: Render 128×128 biome map; 200 creature glyphs at 60 FPS
  - AC: Zoom in/out; pan around map
  - Tasks: tile rendering, creature sprite lookup, camera
  - Points: 8

- Story 9.4: Encounter view renderer (creature meshes, environment) (8 pts)
  - AC: Render 5 creatures + terrain in 2.5D isometric view
  - AC: Camera transitions smoothly from world map to encounter
  - Tasks: encounter camera, mesh rendering, lighting
  - Points: 8

- Story 9.5: Visual directive → mesh/sprite pipeline (Protrude, Harden, Colorize) (10 pts)
  - AC: Interpret genome → VisualDirective set → 3D mesh
  - AC: 100 random creatures render without glitches
  - AC: Asymmetric directives (left vs. right body sites) produce asymmetric meshes
  - Tasks: visual directive struct, shape generators, mesh assembly
  - Points: 10

- Story 9.6: Animation rigging (skeleton + periodic motion) (8 pts)
  - AC: Creatures have walk/run/idle animations driven by movement speed
  - AC: Animations are deterministic (seeded by creature ID)
  - Tasks: skeleton struct, animation driver, motion functions
  - Points: 8

- Story 9.7: Render tests & edge cases (2 pts)
  - AC: Render 100 random creatures; no crashes; no degenerate meshes
  - Tasks: test harness, mesh validity check
  - Points: 2

- Story 9.8: Performance profiling (documentation only for MVP) (6 pts)
  - AC: Record render times per stage; world map: 1–2ms, encounter: 3–5ms
  - Tasks: profiling hooks, benchmark
  - Points: 6

**Demo Criteria**: Generate world. Render at 60 FPS. Spawn 50 creatures visible on map. Enter encounter; render 5 creatures with meshes and animations.

**Exit DoD**: Rendering pipeline complete. 60 FPS maintained on world map and encounter. All visual directives rendered correctly. No degenerate meshes.

**Known Risks**:
- Mesh generation complexity (risk: high, mitigation: use simple primitives first; optimize later)
- Animation frame rate (risk: medium, mitigation: use simple periodic functions; profile)

---

### Sprint S10: UI & Chronicler Basics (W10)
**Goal**: Implement basic UI framework and pattern labeling.

**Stories** (56 pts):
- Story 10.1: Widget trait & basic primitives (Button, List, Dialog) (8 pts)
  - AC: Define Widget trait; implement Button, List; click handling works
  - Tasks: trait def, widget structs, event routing
  - Points: 8

- Story 10.2: WidgetTree & layout engine (flex-like) (8 pts)
  - AC: Nest 10 widgets; layout computes positions correctly
  - AC: Layout is deterministic (same tree → same positions)
  - Tasks: tree struct, layout algorithm, position computation
  - Points: 8

- Story 10.3: Event handling & data binding (6 pts)
  - AC: Click button → handler called; bind text widget to creature name
  - Tasks: event dispatch, binding mechanism
  - Points: 6

- Story 10.4: Screen definitions (WorldMap, Bestiary, Settings, Encounter UI) (8 pts)
  - AC: Define 4 main screens; navigation between screens works
  - Tasks: screen structs, transition logic
  - Points: 8

- Story 10.5: Chronicler pattern detection (primitive signature clustering) (8 pts)
  - AC: Cluster 1000 primitive signatures; identify 5+ unique clusters
  - AC: Cluster stability test: 10 runs with same data → same clusters
  - Tasks: signature hashing, clustering algorithm (K-means or hierarchical)
  - Points: 8

- Story 10.6: Label generation & confidence scoring (6 pts)
  - AC: Cluster → label (e.g., high kinetic_force + jaw → "biter")
  - AC: Confidence score reflects cluster frequency and stability
  - Tasks: label generation heuristics, confidence formula
  - Points: 6

- Story 10.7: QueryAPI (query labels, creatures with label) (6 pts)
  - AC: chronicler.query_label_for_signature("sig_123") → "echolocation"
  - AC: chronicler.query_creatures_with_label("echolocation") → [creature_ids]
  - Tasks: query methods, index structures
  - Points: 6

- Story 10.8: Integration test (render bestiary with 50 creatures, all labeled) (4 pts)
  - AC: Bestiary screen displays 50 discovered creatures with emergent labels
  - Tasks: test harness, bestiary screen test
  - Points: 4

**Demo Criteria**: Render worldmap and bestiary screens. Pattern detection finds 5+ behaviors. Labels assigned with confidence > 0.7. Bestiary displays creatures grouped by label.

**Exit DoD**: UI framework functional. Chronicler detects patterns and assigns labels. Bestiary screen displays discovered creatures with labels. QueryAPI works.

**Known Risks**:
- Label quality (risk: medium, mitigation: tune heuristics via manual testing; expect iteration)
- Clustering stability (risk: medium, mitigation: use seeded random; property-test stability)

---

### Sprint S11: Combat & Formation System (W11)
**Goal**: Implement combat encounters and formation mechanics.

**Stories** (48 pts):
- Story 11.1: Keeper personality & leadership budget (6 pts)
  - AC: Keeper has charisma, neural_speed, empathy; these derive leadership capacity
  - AC: capacity = ceil(charisma * 8 + neural_speed * 4) * (1 - stress * empathy * 0.3)
  - Tasks: Keeper struct, leadership budget formula
  - Points: 6

- Story 11.2: Formation structure (5 creature slots, position/exposure) (8 pts)
  - AC: Define formation with 5 slots; each slot has position, exposure_to_front, exposure_to_flanks
  - AC: Disruption reduces exposure (creature cannot act if exposure too low)
  - Tasks: formation struct, slot position/exposure calcs
  - Points: 8

- Story 11.3: Combat resolution (offense/defense from primitives) (8 pts)
  - AC: Creature A with high kinetic_force + low defense takes 0.5× normal damage from Creature B
  - AC: Damage formula = (A.offense_force * (1 - B.defense_rigidity)).saturate()
  - AC: No lookup tables; all values computed fresh each turn
  - Tasks: combat resolution loop, damage formula, primitive effect reading
  - Points: 8

- Story 11.4: Predation & parasitism mechanics (8 pts)
  - AC: Predator with high vocal_modulation + auditory_sensitivity can detect prey
  - AC: Parasite with transmission > 0.5 infects host with X% chance
  - Tasks: predation trigger logic, parasitism transmission logic
  - Points: 8

- Story 11.5: Formation disruption & movement constraints (6 pts)
  - AC: Disrupted creature (exposure < 0.3) cannot move to attack; can only defend
  - Tasks: disruption state component, action constraint logic
  - Points: 6

- Story 11.6: Combat readout UI (health bars, ability highlights, formation diagram) (8 pts)
  - AC: Encounter screen shows 5 friendly creatures + 3 enemies; health bars, disruption status
  - AC: Abilities highlighted on creatures with relevant primitives
  - Tasks: UI panel design, creature state binding
  - Points: 8

- Story 11.7: Combat encounter test (10 rounds, no crashes) (2 pts)
  - AC: Run 10 combat rounds; all creatures remain valid; no divergences
  - Tasks: combat test harness
  - Points: 2

- Story 11.8: Documentation (combat mechanics, Keeper rules) (2 pts)
  - AC: Docstrings explain combat formula and formation rules
  - Tasks: docs
  - Points: 2

**Demo Criteria**: Run 10-round combat encounter. Damage formula computes fresh. Formation disruption works. UI displays creature state, abilities, and damage taken.

**Exit DoD**: Combat system complete. Formation mechanics validated. Readout UI functional. No crashes in 50-round combat test.

**Known Risks**:
- Combat balance (risk: high, mitigation: tune damage formula via playtesting; adjust multipliers)
- Keeper stress mechanics feel boring (risk: medium, mitigation: add visual feedback; iterate on formula)

---

### Sprint S12: Chronicler & Event Recording (W12)
**Goal**: Record simulation events and integrate labels with UI.

**Stories** (32 pts):
- Story 12.1: Event recording (creature birth, death, extinction, novel phenotype) (6 pts)
  - AC: Record every significant event to chronicle (birth, death, extinction, new pattern)
  - Tasks: event struct, recording system
  - Points: 6

- Story 12.2: Primitive signature clustering (K-means or hierarchical) (8 pts)
  - AC: Cluster 1000-tick chronicle into 5+ clusters; stability > 0.8 over 10 runs
  - Tasks: clustering algorithm, stability metric
  - Points: 8

- Story 12.3: Label generation heuristics (map clusters to names) (6 pts)
  - AC: High kinetic_force + jaw channels → "Biter"
  - AC: High auditory_sensitivity + vocal_modulation → "Vocalizer"
  - Tasks: heuristic rules, name generation
  - Points: 6

- Story 12.4: Confidence scoring (frequency, stability) (4 pts)
  - AC: Label with confidence score ([0, 1]); display only if > 0.7
  - Tasks: confidence formula
  - Points: 4

- Story 12.5: QueryAPI integration with Bestiary (query creatures by label) (4 pts)
  - AC: Bestiary search: "Show all creatures labeled 'Biter'" → returns matching creatures
  - Tasks: search UI, query integration
  - Points: 4

- Story 12.6: Chronicler persistence (save/load event log) (2 pts)
  - AC: Save game; event log persists; load; chronicle is intact
  - Tasks: chronicle serialization
  - Points: 2

- Story 12.7: 1000-tick chronicle test (5+ distinct behaviors detected, confidence > 0.7) (2 pts)
  - AC: Run 1000 ticks with 200 creatures; chronicle records 5+ distinct behaviors with high confidence
  - Tasks: integration test
  - Points: 2

**Demo Criteria**: Record 1000-tick simulation. Detect 5+ emergent behaviors with labels and confidence scores. Bestiary displays creatures grouped by label.

**Exit DoD**: Event recording complete. Chronicler detects patterns and assigns labels with confidence. Bestiary search by label functional.

**Known Risks**:
- Label specificity (risk: medium, mitigation: tune heuristics; may need domain expert input)
- Chronicler performance (risk: low, mitigation: run every 100 ticks; amortize cost)

---

### Sprint S13: MVP Integration & Polish (W13)
**Goal**: Integrate all subsystems; final testing and optimization.

**Stories** (52 pts):
- Story 13.1: Main menu & state machine (6 pts)
  - AC: Launch app → main menu; "New Game" → world gen → game loop starts
  - AC: "Load Game" → file dialog → world loads
  - Tasks: menu UI, app state machine
  - Points: 6

- Story 13.2: New game initialization (biome + starter species + Keeper) (6 pts)
  - AC: New game: ask for difficulty → gen world → spawn Keeper + 50 creatures → ready to play
  - Tasks: new game flow, difficulty tuning
  - Points: 6

- Story 13.3: Input handling (keyboard, mouse, SDL3 events) (6 pts)
  - AC: WASD moves avatar; mouse clicks select actions; ESC pauses
  - Tasks: input event handling, action routing
  - Points: 6

- Story 13.4: Settings UI & persistence (4 pts)
  - AC: Settings screen: graphics quality, audio volume, difficulty
  - AC: Settings saved to UI state file (NOT sim state)
  - Tasks: settings screen, UI state persistence
  - Points: 4

- Story 13.5: Game state transitions (save/load/pause/resume) (6 pts)
  - AC: Game → Save → Pause → Load → Resume all work correctly
  - AC: Save mid-encounter → load → encounter state intact
  - Tasks: state transition logic
  - Points: 6

- Story 13.6: Full determinism validation (100-tick replay, all hashes) (8 pts)
  - AC: Save world; replay 100 ticks from save; all 100 tick hashes match original
  - AC: CI test runs on every commit; failure blocks merge
  - Tasks: test integration, CI setup
  - Points: 8

- Story 13.7: Performance profiling & budget validation (8 pts)
  - AC: Profile 1000 ticks; per-stage budget adherence: target 16ms/tick, achieve <15ms
  - AC: If stage overruns, skip cold system; verify no budget exceedance
  - Tasks: profiling, budget checks, optimization
  - Points: 8

- Story 13.8: Build & CI integration (4 pts)
  - AC: `cargo build --release` succeeds; all tests pass; clippy clean
  - Tasks: CI config, build script
  - Points: 4

- Story 13.9: Documentation (README, architecture guide, onboarding) (4 pts)
  - AC: README: build, run, save/load, report bugs
  - AC: Architecture guide links to crate docs
  - Tasks: docstrings, README, guides
  - Points: 4

- Story 13.10: MVP demo script & final QA (2 pts)
  - AC: 5-minute playthrough script: start → explore → encounter → save → load
  - Tasks: QA checklist, demo plan
  - Points: 2

**Demo Criteria**: Play full game loop: new game → explore world → encounter → combat → save → load → resume. All 13 MVP stories functional. Determinism test passes.

**Exit DoD**: MVP ready to ship. All systems integrated. Determinism validated. Performance budget met. CI passes. Documentation complete.

**Known Risks**:
- Integration regressions (risk: high, mitigation: run full integration tests frequently)
- Performance not meeting budget (risk: medium, mitigation: profile aggressively; may need optimization or feature cuts)

---

### Sprint S14: Final Refinement & MVP Release (W14)
**Goal**: Finish MVP and prepare for deep system branching.

**Stories** (40 pts):
- Story 14.1: Bug fixes & edge cases (identify from playtesting) (12 pts)
  - AC: Playtesting session finds N bugs; prioritize and fix top 3
  - Tasks: bug fixes
  - Points: 12

- Story 14.2: UI/UX polish (button feedback, error messages, tooltips) (8 pts)
  - AC: All UI elements have clear hover states, error messages are helpful
  - Tasks: UI refinement
  - Points: 8

- Story 14.3: Creature variety (add 3 more starter species or variant genomes) (6 pts)
  - AC: 6 total starter species in MVP; 2 per major biome type
  - Tasks: genome design, tuning
  - Points: 6

- Story 14.4: Final determinism validation (1000-tick full replay) (8 pts)
  - AC: Save after 500 ticks; replay all 500; all hashes match; no divergence
  - Tasks: extended determinism test
  - Points: 8

- Story 14.5: Performance tuning (if needed from profiling) (6 pts)
  - AC: Any stage exceeding budget: optimize or defer systems
  - Tasks: profiling, optimization
  - Points: 6

**Exit DoD**: MVP feature-complete and stable. Determinism validated to 1000+ ticks. All major bugs fixed. Ready to branch for deep systems.

---

## Deep System Branching (Post-MVP, S15–S18)

At the end of S14, the team chooses **one deep system** to develop in detail. Each is 3–4 sprints (~150 points). The branching structure allows the plan to be adaptable based on MVP reception.

### Option A: Evolution Depth (S15–S18A)

**Goal**: Deeper evolutionary dynamics, genetic algorithms, complex regulatory networks.

**Scope**:
- Expand mutation operators: recombination (sexual reproduction), inversion, translocation
- Regulatory network complexity: add feedback loops, time-delay regulators
- Genetic algorithms: hill-climbing search space (creatures solving complex morphological problems)
- Adaptive landscape visualization: UI shows fitness landscape for current biome
- Advanced speciation: reproductive isolation, sympatric speciation via disruptive selection

**Estimate**: 150 points

**S15A (40 pts)**: Sexual reproduction & recombination operator
- Implement crossover (mate two genomes → offspring)
- Test recombination produces novel genotypes
- Mating system in simulation (select compatible mates)

**S16A (40 pts)**: Regulatory network feedback & time-delay elements
- Add feedback regulators (A inhibits B, B inhibits A)
- Time-delay components (effect appears 10 ticks after trigger)
- Test network dynamics don't diverge chaotically

**S17A (40 pts)**: Fitness landscape visualization & genetic algorithms
- UI overlay: show fitness value across channel space
- Implement hill-climbing algorithm (optional, for creature AI)
- Test creatures converge toward local optima

**S18A (30 pts)**: Advanced speciation & reproductive isolation
- Track reproductive compatibility (genetically distant mates have lower fertility)
- Sympatric speciation: disruptive selection in single biome → population splits
- Test: two species emerge in same biome after 1000 ticks of divergent selection

---

### Option B: Disease (S15–S18B)

**Goal**: Pathogen coevolution, SEIR epidemiological dynamics, host-pathogen arms races.

**Scope**:
- Pathogen entity type: micro-scale organisms, scale_band filtering
- Host coupling profile: compute host-pathogen fitness from channel values
- Transmission network: spatial contact graph, transmission success rates
- SEIR compartmentalization: Susceptible, Exposed, Infected, Recovered states
- Virulence evolution: pathogens evolve toward optimal virulence (too high → kills host)
- Host immune evolution: hosts evolve defense channels in response to pathogen prevalence

**Estimate**: 150 points

**S15B (40 pts)**: Pathogen entity & scale-band filtering
- Create Pathogen component (micro-scale organisms)
- Test scale-band filtering: pathogen-only channels dormant in macro, active in micro
- Implement HostCouplingProfile computation (energetic_drain, transmission, virulence)

**S16B (40 pts)**: Transmission network & SEIR dynamics
- Build transmission contact graph (spatial proximity → potential transmission)
- Implement SEIR state transitions (exposure → incubation → acute → recovery)
- Test epidemic outbreak: 10% of creatures infected → 80% infected after 50 ticks

**S17B (40 pts)**: Virulence evolution & host-pathogen coevolution
- Pathogens evolve virulence (trade-off: high virulence → quick death, low transmission)
- Hosts evolve immune channels in response to prevalence
- Test: arms race dynamics (host defense ↑ → pathogen virulence ↑ → cycle)

**S18B (30 pts)**: Epidemic aftermath & extinction/persistence
- Track pathogen extinction (rare in isolated populations)
- Endemic equilibrium (pathogen persists at low level)
- Test: 1000-tick disease simulation produces stable or extinct endpoint

---

### Option C: Economy (S15–S18C)

**Goal**: Settlement economics, resource trading, specialization feedback on evolution.

**Scope**:
- Settlement entity type: faction headquarters, resource production/consumption
- Trade network: settlements trade with adjacent settlements
- Resource chains: raw materials → crafted goods → luxury items
- Economic specialization: settlements focus on specific products based on local biome advantages
- Evolutionary feedback: creatures evolve in response to economic niches (e.g., domestication)
- Player economy: Keeper can trade/craft, resources affect recruitment of creatures

**Estimate**: 150 points

**S15C (40 pts)**: Settlement entity & resource production
- Create Settlement component (faction, location, production capacity)
- Define resource types (food, metal, hide, gems, etc.)
- Implement per-biome production rates (forest: more wood, less metal)
- Test: 5 settlements on 5 biomes produce resources according to biome type

**S16C (40 pts)**: Trade network & resource chains
- Build trade connections (adjacent settlements exchange goods)
- Implement supply/demand dynamics (shortage → price ↑, surplus → price ↓)
- Define crafting recipes (hide + skill → armor)
- Test: 10-settlement network reaches trade equilibrium after 100 ticks

**S17C (40 pts)**: Economic specialization & feedback on evolution
- Settlements develop specialization (e.g., "Hide Workers" biome → focus on hide production)
- Creatures evolve in response to economic niches (hide-producing creatures more viable)
- Player can recruit creatures from settlements (limited quantity, cost = resources)
- Test: settlement specialization emerges; creature evolution correlates with local economy

**S18C (30 pts)**: Keeper economy & long-term gameplay loop
- Keeper can trade with settlements, earn resources, spend on recruitment
- Resource constraints force strategic choices (do you recruit now or save for better creature?)
- Test: 1000-tick economic simulation produces stable settlement network and creature recruitment patterns

---

### Option D: Culture (S15–S18D)

**Goal**: Language emergence, cultural drift, narrative historiography.

**Scope**:
- Language system: creatures develop communication signals (high vocal_modulation + auditory_sensitivity)
- Cultural traits: behaviors/aesthetics that spread via social learning
- Historiography: Chronicler records events; NPCs misremember/retell with bias
- Lore generation: contradictory historical accounts create mystery and depth
- Player as archaeologist: discovers lore fragments, interprets history
- Narrative emergence: gameplay generates unique stories

**Estimate**: 150 points

**S15D (40 pts)**: Communication signals & language scaffolding
- Implement Vocalizer entity (broadcasts signals in area)
- Listeners receive signals; interpret as social cues (warning, mating, territorial)
- Test: creatures with matching vocal/auditory channels share more signals

**S16D (40 pts)**: Cultural drift & social learning
- Define cultural traits (aesthetic choices, behavioral preferences)
- Spread mechanism: creatures near similar-culture creatures adopt traits (via proximity/sociality)
- Test: isolated populations develop distinct aesthetics; populations mix → cultures blend

**S17D (40 pts)**: Historiography & unreliable narrator
- Extend Chronicler to record major events (faction founding, extinction, discovery)
- NPCs debate history; recount events with faction bias
- Player encounters 2-3 contradictory accounts of same event
- Test: 1000-tick chronicle generates 5+ major events with >3 contradictory retellings each

**S18D (30 pts)**: Lore UI & player archaeology
- UI screen: "Historical Records" → browse events, NPC accounts, player annotations
- Player can mark theories (connect events, identify patterns)
- Test: 1000-tick gameplay surfaces emergent lore; player can read coherent (if ambiguous) history

---

## Dependency Graph

```
E1 (Foundations)
  ↓
E2 (Registries) ← E1
E3 (Genome)    ← E1, E2
E4 (Interpreter) ← E1, E2, E3
E5 (ECS)       ← E1, E2
E6 (Tick Loop) ← E1, E3, E4, E5
E7 (Save/Load) ← E1, E5, E6
E8 (World Gen) ← E1, E3, E4, E5, E6
E9 (Render)    ← E3, E4, E5, E8
E10 (UI/Chronicler) ← E5, E9
E11 (Combat)   ← E3, E4, E5, E6, E10
E12 (Chronicler Events) ← E4, E10
E13 (MVP Integration) ← E1–E12
  ↓
E14 (Deep System) ← E1–E13 (branching)
  │
  ├─ A (Evolution)
  ├─ B (Disease)
  ├─ C (Economy)
  └─ D (Culture)
```

---

## Risk Register (Top 10)

| # | Risk | Probability | Impact | Mitigation |
|---|------|-------------|--------|-----------|
| 1 | Determinism divergence at scale (1000+ ticks) | Medium | Critical | Early determinism testing (E6); binary diff tool for debugging; property tests on all arithmetic |
| 2 | Performance budget exceeded (> 20ms/tick) | Medium | High | Profile early (S6); prioritize hot path; defer cold systems aggressively |
| 3 | Mutation operators produce invalid genomes (values escape [0,1]) | Medium | High | Rigorous bounds testing (E3); saturating arithmetic; property-based tests |
| 4 | Complex composition hooks cause stack overflows or slow evaluation | Medium | High | Limit hook depth; cache evaluated hooks; optimize fixed-point math |
| 5 | Procedural visuals produce degenerate meshes (zero-area triangles) | Medium | Medium | Validation pass on all meshes (E9); clamp minimum face sizes |
| 6 | Chronicler label quality is poor (labels don't match emergent behaviors) | Low | Medium | Manual playtesting; domain expert review; tune heuristics iteratively |
| 7 | Combat balance is broken (creatures always die too fast or too slow) | High | Medium | Playtesting early (S11); tune damage formula; gather feedback from MVP users |
| 8 | ECS framework overhead (entity lookup, component access) degrades performance | Low | Medium | Profile early; benchmark vs. naive arrays; use dense storage |
| 9 | Save file format incompatibility across versions (migrations fail) | Low | High | Version schema carefully; test migrations on real saves; plan forward compatibility |
| 10 | UI event handling becomes bottleneck (1000 widgets, lag on input) | Low | Medium | Lazy UI rendering; batch events; profile widget tree depth |

---

## Definition of Done (Project-Level)

### MVP Release (End of S14)
- [ ] All 13 epics E1–E13 completed and integrated
- [ ] Determinism test passes: 1000-tick replay, all tick hashes bit-identical
- [ ] Bestiary functional: 20+ discovered creatures, all labeled
- [ ] One full encounter playable: 5 friendly creatures, 3 enemies, 10 combat rounds
- [ ] Save/load: game persists and resumes correctly; forbidden keys rejected
- [ ] World generation: playable archipelago with 3+ biome types and 50+ creatures
- [ ] UI: main menu, worldmap, bestiary, encounter, settings screens all functional
- [ ] Performance: sustained 60 FPS on worldmap and encounter views
- [ ] CI passing: all tests green; clippy clean; build release succeeds
- [ ] Documentation: README, architecture guide, developer onboarding

### Deep System Release (End of S18)
- [ ] One deep system (A/B/C/D) fully implemented and playable
- [ ] New mechanics demonstrated in 1000-tick gameplay
- [ ] Determinism preserved with new systems
- [ ] New content visible in-game (e.g., pathogens, settlements, cultural traits)
- [ ] Updated documentation for deep system

### Post-Release
- [ ] Mod loading functional (load custom channel/primitive manifests)
- [ ] Replay validation tool available (CLI: `beast-cli validate <save> <input_journal>`)
- [ ] Community feedback loop established

---

## Story Templates (5 Exemplars)

### Story 1.1: Q3232 Fixed-Point Type
**Title**: Implement Q3232 fixed-point wrapper with saturating arithmetic

**Description**: Create a Q3232 (32 bits integer, 32 bits fractional) fixed-point type that wraps Rust's `i64` and provides deterministic arithmetic. All operations must saturate on overflow (not wrap).

**Acceptance Criteria**:
- `Q3232(0.5) + Q3232(0.3) == Q3232(0.8)` (fixed-point add)
- `Q3232(0.9) + Q3232(0.5) == Q3232(1.0)` (saturate to max)
- `Q3232(0.5) * Q3232(0.5) == Q3232(0.25)` (fixed-point multiply)
- Bitwise output identical when seeded with same value and operations

**Tasks**:
- Define `pub struct Q3232(i64)` with private repr
- Implement `Add`, `Sub`, `Mul`, `Div` traits with saturating ops
- Implement `Ord`, `Eq`, `PartialOrd`, `PartialEq` for comparisons
- Add `from_float(f32)` and `to_float()` for testing (conversion only; not used in sim)
- Write unit tests for all operations
- Benchmark multiply: target < 2 CPU cycles

**Points**: 8

**Tests Required**:
- 100+ unit tests of arithmetic
- Property-based test: `forall x, y: (x + y - y) ≈ x` (within rounding error)
- Overflow edge case: `MAX + 1 = MAX` (saturate)
- Underflow edge case: `MIN - 1 = MIN` (saturate)

---

### Story 3.5: Gene Duplication & Genesis
**Title**: Implement genesis operator (gene duplication + manifest copy)

**Description**: When a creature mutates, rare duplication events (rate ~5e-5) cause a gene to be duplicated. The paralog inherits the parent's manifest but is marked with a genesis event (`genesis:parent_id:generation`). This enables the channel set itself to evolve.

**Acceptance Criteria**:
- Duplication creates new gene with unique ID, inherited manifest, genesis provenance tag
- Duplication rate is ~5e-5 per mutation (configurable)
- Paralog channel ID is distinct from parent (never collides)
- Manifest lookup succeeds for paralog in registry
- Subsequent mutations on paralog drift it toward different family (e.g., motor → sensory)

**Tasks**:
- Implement `mutate_duplication(rng, genome) -> Result<()>`
- Generate unique paralog IDs deterministically (based on parent ID + tick)
- Inherit manifest from parent channel
- Add paralog to genome.genes with marked genesis event
- Add paralog to registry (new entry, not override)
- Test duplication doesn't create ID collisions

**Points**: 8

**Tests Required**:
- 1000 duplications of 10 genomes: no ID collisions
- Paralog manifest lookup succeeds every time
- Genesis provenance tag correctly records parent + generation
- Paralog can be mutated independently; subsequent mutations diverge from parent

---

### Story 6.6: 100-Tick Determinism Test
**Title**: Implement determinism validation: save state, replay, compare hashes

**Description**: After implementing the full tick loop (E6), validate that the same world state + seed + inputs produce bit-identical tick hashes at every tick. This is the gold standard for determinism.

**Acceptance Criteria**:
- Save world state after tick 50
- Load save file; verify world state matches original
- Replay ticks 1–100 with same RNG seed and input sequence
- All 100 tick hashes (from `compute_state_hash()`) match original run
- Test harness accepts pre-generated fixture (determinism_test.json with save state + inputs)

**Tasks**:
- Implement SaveFile serialization (stub from E7)
- Implement load_game() to restore Simulation state
- Implement ReplayJournal to log and replay inputs
- Write test harness that:
  - Runs 100 ticks, saves tick hashes
  - Loads save at tick 50
  - Replays ticks 1–100
  - Compares all hashes; fail if any diverge
- Add binary diff tool (debug output showing first divergence)

**Points**: 12

**Tests Required**:
- 100-tick determinism test on multiple world seeds (5 seeds)
- Replay from mid-game save (tick 50): all hashes match
- Fixture test: load pre-generated save file; replay fixture input journal
- Hash collision test: intentional small change in world state → different hash
- Determinism test part of CI; failure blocks merge

---

### Story 9.5: Visual Directive → Mesh Pipeline
**Title**: Implement procedural mesh generation from visual directives

**Description**: The interpreter produces VisualDirective objects (Protrude, Harden, Colorize, etc.). This story converts directives into 3D meshes (or 2D sprites for MVP). Each directive type triggers specific geometry:
- `Protrude(shape=Spike, scale=0.5, distribution=100)` → 100 small spikes covering a body region
- `Harden(rigidity=0.8)` → thicker/denser geometry
- `Colorize(rgb=[255, 100, 50])` → apply color to mesh

**Acceptance Criteria**:
- 100 random creatures render without visual glitches (no degenerate meshes)
- Asymmetric directives (left vs. right body sites) produce asymmetric visuals
- Procedural generation is deterministic (same creature ID → same mesh every time)
- Mesh generation completes in < 1ms per creature (1000 creatures → < 1 sec)

**Tasks**:
- Define `VisualDirective` struct with union of directive types
- Implement shape generators: `generate_spikes()`, `generate_plates()`, etc.
- Implement mesh assembly: skeleton + directed shapes → combined mesh
- Implement deterministic random seed for mesh generation (based on creature ID)
- Add mesh validation: no degenerate triangles (area > 0); no NaN vertices
- Benchmark: 1000 creatures per second

**Points**: 10

**Tests Required**:
- 100 random creatures render without crashes
- Validation test: all meshes pass degeneracy check
- Determinism test: same creature ID → identical mesh (10 renders)
- Asymmetry test: creatures with asymmetric directives have visibly different left/right sides
- Performance test: 1000 creatures in < 1 sec

---

### Story 11.1: Keeper Personality & Leadership Budget
**Title**: Define Keeper personality traits and derive leadership capacity

**Description**: The player character (Keeper) has personality traits (charisma, neural_speed, empathy) that determine how many creature actions they can direct per combat round. Stress accumulates during combat; high stress reduces leadership capacity.

**Acceptance Criteria**:
- Keeper has charisma, neural_speed, empathy channels [0, 1]
- Base leadership capacity = ceil(charisma * 8 + neural_speed * 4)
- Stress reduction factor = 1.0 - (current_stress * empathy * 0.3)
- Active capacity = floor(base * stress_reduction)
- Stress increases when crew is injured or killed
- Stress decreases when Keeper rests (off-combat)

**Tasks**:
- Define `KeeperState` struct with personality channels and stress
- Implement `compute_leadership_capacity(keeper) -> int`
- Implement `apply_stress(keeper, damage_witnessed, casualties_witnessed)`
- Implement `reduce_stress_on_rest(keeper, rest_ticks)`
- Test capacity ranges: min=1 (low charisma, high stress), max=12 (high charisma, low stress)

**Points**: 6

**Tests Required**:
- High charisma (0.8) + low stress (0.1) → capacity ~8
- Low charisma (0.2) + high stress (0.9) → capacity 1
- Stress accumulation test: crew takes 3 hits → stress rises; capacity drops
- Stress recovery test: 10 ticks at rest → stress falls; capacity recovers
- Edge case: capacity never < 1 (can always take 1 action)

---

## Milestones & Demos

### M1: Tick Loop with One Creature (End of S6)
**Timeline**: Week 6

**Demo**: Single creature spawned in biome. Tick loop runs. Creature gains energy, mutates once per tick, ages. Phenotype interpreter runs each tick. State hash computed and logged.

**Criteria**:
- Creature persists for 100 ticks without crashing
- Mutation visible (phenotype changes each tick)
- State hash logged; reproducible with same seed
- No panics or assertion failures

---

### M2: First Deterministic Replay (End of S7)
**Timeline**: Week 7

**Demo**: Save world at tick 50. Load save file. Replay ticks 51–150. All tick hashes match original run.

**Criteria**:
- Save/load round-trip preserves world state
- Replay is deterministic (all hashes match)
- No divergence even at tick 150
- Determinism test part of CI

---

### M3: Emergent Label Discovery (End of S12)
**Timeline**: Week 12

**Demo**: Run 1000-tick simulation with 200 creatures. Chronicle records events. Chronicler detects 5+ unique primitive clusters. Assigns labels with confidence > 0.7. Bestiary displays creatures grouped by label.

**Criteria**:
- 5+ labels emerge (e.g., "Biter", "Vocalizer", "Camouflaged")
- Labels are meaningful (not random)
- Confidence scoring works (high confidence = high cluster frequency)
- Bestiary search by label returns correct creatures

---

### M4: MVP Playable Slice (End of S14)
**Timeline**: Week 14

**Demo**: Full game loop. Start → New Game → Explore worldmap (50 creatures visible) → Enter encounter → 10-round combat → Save → Exit → Load → Resume.

**Criteria**:
- All 13 MVP epics functional
- Combat resolves correctly (damage computed from primitives)
- Bestiary displays 20+ discovered creatures with labels
- Save/load works; game state preserved
- Performance: 60 FPS sustained
- No crashes or divergences

---

### M5: Deep System Demo (End of S18)
**Timeline**: Week 18

**Demo**: Chosen deep system (A/B/C/D) fully integrated. 1000-tick gameplay demonstrates new mechanics.

**Example (if Option B: Disease)**:
- Pathogen spawns at tick 100
- Transmission spreads through creature population
- Host immune evolution counters pathogen virulence
- UI shows pathogen prevalence over time
- Simulation reaches endemic equilibrium (or pathogen extinction)

**Criteria**:
- New mechanics visible in gameplay
- No determinism divergence with new systems
- New content (pathogens, settlements, language, or evolved channels) present in world
- Updated documentation

---

## Velocity & Adjustment

**Baseline Assumption**: 40 points per sprint (1 week part-time solo + Claude).

**If Velocity Differs**:

| Scenario | Adjustment |
|----------|-----------|
| Actual velocity: 35 pts/sprint (-12%) | Add 2 sprints; MVP on S16 (W16). Reduce some stretch goals (e.g., audio, advanced UI). |
| Actual velocity: 45 pts/sprint (+12%) | Bring forward deep system; start at S14 (overlap last MVP sprint). Polish time adds. |
| Actual velocity: 30 pts/sprint (-25%) | Extend MVP to S18 (W18). Cut one deep system; ship only one deep system in v2.0. |

**Tracking**:
- Sprint velocity calculated at sprint end (completed story points / planned points)
- Burndown chart maintained in GitHub Issues
- Spike stories (risk stories) estimated conservatively; if completed early, allocate slack to integration/polish

**Replanning**:
- Sprint S8 (mid-project): reassess velocity; adjust S9–S14 if needed
- Sprint S14 (MVP gate): decision on deep system (A/B/C/D) made based on team interest and feedback

---

## Summary

This implementation plan maps the Beast Evolution Game from zero to MVP in 14 sprints (~630 points, ~3.5 months for solo dev + Claude), followed by one deep system in 3.75 additional sprints. The plan prioritizes determinism, emergent mechanics, and playability. Epic dependencies are clear; sprint stories are estimated at 1–12 points (Claude + solo dev can complete in 1–4 hours of focused work per story). Risks are identified and mitigated early. The MVP is a playable tick loop with 50+ creatures, emergent labels, save/load, and one full combat encounter. Deep systems branch post-MVP, allowing the team to pursue the most interesting direction (Evolution, Disease, Economy, or Culture) based on MVP reception.
