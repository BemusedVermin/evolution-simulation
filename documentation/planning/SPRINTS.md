# Beast Evolution Game: Sprint Plan & Detailed Breakdown

---

## Sprint Calendar (14 Sprints, MVP + 1 Deep System)

```
Phase 1: Foundations & Core Sim (S1–S4, Weeks 1–4)
├─ S1 (W1): Fixed-Point & PRNG (beast-core)
├─ S2 (W2): Manifests & Registries (beast-channels, beast-primitives)
├─ S3 (W3): Genome & Mutation (beast-genome)
└─ S4 (W4): Phenotype Interpreter (beast-interpreter)

Phase 2: Evolution & ECS (S5–S9, Weeks 5–9)
├─ S5 (W5): ECS Foundation (beast-ecs)
├─ S6 (W6): Tick Loop & Determinism (beast-sim)
├─ S7 (W7): Save/Load & Replay (beast-serde)
├─ S8 (W8): World Generation (biome + procgen)
└─ S9 (W9): Rendering & Visuals (beast-render + beast-interpreter visuals)

Phase 3: UI & Combat (S10–S14, Weeks 10–14)
├─ S10 (W10): UI & Chronicler Basics (beast-ui, beast-chronicler)
├─ S11 (W11): Combat & Formation (beast-sim + beast-ui combat readout)
├─ S12 (W12): Chronicler Events & Labeling (extend E10)
├─ S13 (W13): MVP Integration & Polish (beast-app, beast-cli)
└─ S14 (W14): Final Refinement & MVP Release (bug fixes, balance, final testing)

Phase 4: Deep System (S15–S18, Weeks 15–18) [BRANCHING]
├─ S15 (W15): Deep System Part 1 (branching on chosen system A/B/C/D)
├─ S16 (W16): Deep System Part 2
├─ S17 (W17): Deep System Part 3
└─ S18 (W18): Deep System Part 4 + release

Timeline: MVP ships end of S14 (Week 14, ~3.5 months).
Deep system ships end of S18 (Week 18, ~4.5 months total).
```

---

## Phase 1: Foundations & Core Sim (Weeks 1–4)

### Sprint S1: Fixed-Point & PRNG (W1)

**Sprint Goal**: Establish deterministic numerical foundation. No simulation yet, but all downstream code depends on this.

**Points**: 40 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 1.1 | Q3232 fixed-point type with saturating arithmetic | 8 | Solo | Not Started |
| 1.2 | Xoshiro256PlusPlus PRNG with seeding & streams | 8 | Solo | Not Started |
| 1.3 | EntityID, TickCounter, custom Error type | 6 | Solo | Not Started |
| 1.4 | Box-Muller Gaussian sampling & saturating math utils | 6 | Solo | Not Started |
| 1.5 | Unit tests + property-based fuzzing (100k samples) | 6 | Claude | Not Started |
| 1.6 | Benchmarking & documentation | 6 | Solo | Not Started |

**Dependencies**: None

**Demo Criteria**:
- Run determinism_test.rs: same seed produces identical PRNG output over 1M iterations
- No panics on overflow/underflow
- Fixed-point multiply < 2 CPU cycles (measured)

**Exit DoD**:
- [ ] All 6 stories completed
- [ ] beast-core crate published (no external deps on other beast crates)
- [ ] CI passes (cargo test, clippy clean)
- [ ] README for beast-core with usage examples

---

### Sprint S2: Manifests & Registries (W2)

**Sprint Goal**: Load channels and primitives. Enable modding and registry validation.

**Points**: 38 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 2.1 | Channel manifest schema & JSON loader | 8 | Solo | Not Started |
| 2.2 | Primitive manifest schema & JSON loader | 8 | Solo | Not Started |
| 2.3 | ChannelRegistry with queryable indexing | 6 | Claude | Not Started |
| 2.4 | PrimitiveRegistry with cost function evaluation | 6 | Claude | Not Started |
| 2.5 | Composition hook parser & evaluator | 8 | Solo | Not Started |
| 2.6 | Schema validation & malformed manifest rejection | 2 | Claude | Not Started |

**Dependencies**: E1 (beast-core)

**Demo Criteria**:
- Load core/channels.json (18 channels) without errors
- Load core/primitives.json (25 primitives) without errors
- Query registries; retrieve expected channels
- Evaluate 10 composition hooks; all produce expected results

**Exit DoD**:
- [ ] Both registries loaded and queryable
- [ ] Schema validation rejects 5 malformed test manifests
- [ ] Composition hook evaluation deterministic
- [ ] CI passes

---

### Sprint S3: Genome & Mutation (W3)

**Sprint Goal**: Implement evolvable genotypes and core mutation operators.

**Points**: 40 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 3.1 | Genome & TraitGene data structures | 6 | Solo | Not Started |
| 3.2 | Body-site model (location, coverage, symmetry) | 6 | Claude | Not Started |
| 3.3 | Point mutation operator (Gaussian drift) | 8 | Solo | Not Started |
| 3.4 | Regulatory modifier rewiring | 6 | Claude | Not Started |
| 3.5 | Gene duplication & genesis (channel paralog creation) | 8 | Solo | Not Started |
| 3.6 | Mutation tests & property tests | 6 | Claude | Not Started |

**Dependencies**: E1, E2

**Demo Criteria**:
- Mutate 100 random genomes; all remain valid
- Genesis operator creates valid parolog with unique ID
- Mutation distribution matches expected Gaussian (KS test, p > 0.05)

**Exit DoD**:
- [ ] All mutation operators functional
- [ ] Genome structure supports variable-length gene lists
- [ ] 1000 mutations of 10 genomes: no panics, all values valid
- [ ] CI passes

---

### Sprint S4: Phenotype Interpreter (W4)

**Sprint Goal**: Convert genotypes to primitive effects deterministically.

**Points**: 48 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 4.1 | Scale-band filtering (gate channels by creature mass) | 6 | Solo | Not Started |
| 4.2 | Expression condition evaluator (biome, season gates) | 6 | Claude | Not Started |
| 4.3 | Composition hook resolver (threshold, additive, multiplicative) | 8 | Solo | Not Started |
| 4.4 | Fixed-point parameter mapping & primitive effect emission | 8 | Claude | Not Started |
| 4.5 | Body region tiling & phenotype aggregation | 6 | Solo | Not Started |
| 4.6 | Determinism tests & scale-band fixture | 8 | Claude | Not Started |

**Dependencies**: E1, E2, E3

**Demo Criteria**:
- Interpret 100 random genomes; all produce valid PrimitiveEffect sets
- Scale-band filtering: 100kg creature with [1e-15, 1e-3kg] channel → zero primitives
- Composition hook threshold: fires iff both operands > T
- 1000-tick determinism test: same seed → identical outputs

**Exit DoD**:
- [ ] Interpreter fully functional
- [ ] All composition hook types working
- [ ] Fixed-point arithmetic throughout (no floats in hot path)
- [ ] 1000 determinism test ticks pass
- [ ] CI passes

---

## Phase 2: Evolution & ECS (Weeks 5–9)

### Sprint S5: ECS Foundation (W5)

**Sprint Goal**: Build Entity-Component-System framework.

**Points**: 40 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 5.1 | EcsWorld wrapper & specs integration | 4 | Solo | Not Started |
| 5.2 | Component type definitions (15 components) | 8 | Claude | Not Started |
| 5.3 | System trait & SystemStage enum | 4 | Solo | Not Started |
| 5.4 | Resources struct (registries, PRNG streams) | 4 | Claude | Not Started |
| 5.5 | Sorted entity index (BTreeMap per type) | 6 | Solo | Not Started |
| 5.6 | Component storage & parallelization safety | 6 | Claude | Not Started |
| 5.7 | Tests & determinism validation | 8 | Claude | Not Started |

**Dependencies**: E1, E2

**Demo Criteria**:
- Create world with 1000 creatures; iteration in sorted order
- Define 5 mock systems; call on world
- No panics or data corruption

**Exit DoD**:
- [ ] ECS framework complete
- [ ] Deterministic iteration order verified (10 runs)
- [ ] Parallel safety validated
- [ ] CI passes

---

### Sprint S6: Tick Loop & Determinism (W6)

**Sprint Goal**: Implement the 8-stage simulation schedule and determinism guards.

**Points**: 44 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 6.1 | Simulation struct & initialization | 4 | Solo | Not Started |
| 6.2 | SystemSchedule & system registration (8 stages) | 6 | Claude | Not Started |
| 6.3 | Per-stage parallel dispatch (rayon) | 6 | Solo | Not Started |
| 6.4 | TickResult & performance budget tracking | 4 | Claude | Not Started |
| 6.5 | State hash computation (sorted iteration + XOR) | 6 | Solo | Not Started |
| 6.6 | 100-tick determinism test (save, replay, hash compare) | 12 | Claude | Not Started |
| 6.7 | Tick budget profiling & deferred systems | 6 | Solo | Not Started |

**Dependencies**: E1, E3, E4, E5

**Demo Criteria**:
- Run 100 ticks; save hash after each
- Replay with same seed; all hashes match
- Per-stage timing printed

**Exit DoD**:
- [ ] 8-stage schedule implemented
- [ ] Determinism test passes (100 ticks, multiple seeds)
- [ ] Budget tracking functional
- [ ] CI passes

**Risk Focus**: Determinism divergence (R1), Performance budget (R2)

---

### Sprint S7: Save/Load & Replay (W7)

**Sprint Goal**: Implement persistence and replay validation.

**Points**: 44 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 7.1 | SaveFile struct & serialization | 6 | Solo | Not Started |
| 7.2 | SaveManager (save/load orchestration) | 6 | Claude | Not Started |
| 7.3 | ReplayJournal (input sequence logging) | 6 | Claude | Not Started |
| 7.4 | SaveValidator (schema validation + forbidden keys) | 6 | Solo | Not Started |
| 7.5 | Migration system (upgrade old saves) | 6 | Claude | Not Started |
| 7.6 | Determinism via replay test (100 ticks → save → load → replay) | 10 | Claude | Not Started |

**Dependencies**: E1, E5, E6

**Demo Criteria**:
- Save world; load back; state unchanged
- Reject save with bestiary_discovered key
- Replay 100 ticks from save; state matches original

**Exit DoD**:
- [ ] SaveFile & SaveManager complete
- [ ] SaveValidator working
- [ ] Replay determinism test passes
- [ ] CI passes

---

### Sprint S8: World Generation (W8)

**Sprint Goal**: Generate playable archipelago with biomes and starter species.

**Points**: 40 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 8.1 | Procedural archipelago generation (Perlin noise) | 8 | Solo | Not Started |
| 8.2 | Biome system & BiomeCell component | 8 | Claude | Not Started |
| 8.3 | Initial species definition (3 starter genomes) | 6 | Solo | Not Started |
| 8.4 | Seed creature spawning (50 initial creatures) | 6 | Claude | Not Started |
| 8.5 | Climate model (temperature, precipitation, seasons) | 8 | Claude | Not Started |
| 8.6 | Tests (starter species survival, biome resources) | 4 | Solo | Not Started |

**Dependencies**: E1, E3, E4, E5, E6

**Demo Criteria**:
- Generate world; biomes visible
- 50 creatures spawned; survive 100 ticks
- Season cycles every 1000 ticks

**Exit DoD**:
- [ ] World generation complete
- [ ] Starter species survive
- [ ] Climate model integrated
- [ ] CI passes

---

### Sprint S9: Rendering & Visuals (W9)

**Sprint Goal**: Implement SDL3 rendering and procedural visual pipeline.

**Points**: 52 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 9.1 | SDL3 initialization & window management | 4 | Solo | Not Started |
| 9.2 | Sprite atlas management | 6 | Claude | Not Started |
| 9.3 | World map renderer (tiles, creature glyphs) | 8 | Solo | Not Started |
| 9.4 | Encounter view renderer (2.5D perspective) | 8 | Claude | Not Started |
| 9.5 | Visual directive → mesh/sprite pipeline | 10 | Solo+Claude | Not Started |
| 9.6 | Animation rigging (skeleton + periodic motion) | 8 | Claude | Not Started |
| 9.7 | Render tests (100 creatures, no glitches) | 2 | Solo | Not Started |
| 9.8 | Performance profiling | 6 | Solo | Not Started |

**Dependencies**: E3, E4, E5, E8

**Demo Criteria**:
- Render world map at 60 FPS (200 creatures visible)
- Render encounter at 60 FPS (5 creatures + terrain)
- 100 random creatures render without glitches

**Exit DoD**:
- [ ] Rendering pipeline complete
- [ ] 60 FPS maintained
- [ ] All visual directives rendered
- [ ] CI passes

**Risk Focus**: Procedural mesh quality (R5), Performance budget (R2)

---

## Phase 3: UI & Combat (Weeks 10–14)

### Sprint S10: UI & Chronicler Basics (W10)

**Sprint Goal**: Implement widget framework and basic pattern labeling.

**Points**: 56 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 10.1 | Widget trait & primitives (Button, List, Dialog) | 8 | Solo | Not Started |
| 10.2 | WidgetTree & layout engine (flex-like) | 8 | Claude | Not Started |
| 10.3 | Event handling & data binding | 6 | Solo | Not Started |
| 10.4 | Screen definitions (WorldMap, Bestiary, Settings, Encounter) | 8 | Claude | Not Started |
| 10.5 | Chronicler pattern detection (clustering) | 8 | Solo+Claude | Not Started |
| 10.6 | Label generation & confidence scoring | 6 | Claude | Not Started |
| 10.7 | QueryAPI (query labels, creatures with label) | 6 | Solo | Not Started |
| 10.8 | Integration test (bestiary with 50 creatures) | 4 | Claude | Not Started |

**Dependencies**: E5, E9

**Demo Criteria**:
- Render UI screens at 60 FPS
- Pattern detection on 1000-tick chronicle
- Bestiary displays creatures with labels

**Exit DoD**:
- [ ] UI framework functional
- [ ] Chronicler detects patterns
- [ ] Bestiary search works
- [ ] CI passes

---

### Sprint S11: Combat & Formation System (W11)

**Sprint Goal**: Implement combat encounters and formation mechanics.

**Points**: 48 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 11.1 | Keeper personality & leadership budget | 6 | Solo | Not Started |
| 11.2 | Formation structure (5 slots, position/exposure) | 8 | Claude | Not Started |
| 11.3 | Combat resolution (offense/defense from primitives) | 8 | Solo | Not Started |
| 11.4 | Predation & parasitism mechanics | 8 | Claude | Not Started |
| 11.5 | Formation disruption & movement constraints | 6 | Solo | Not Started |
| 11.6 | Combat readout UI (health bars, abilities) | 8 | Claude | Not Started |
| 11.7 | Combat encounter test (10 rounds, no crashes) | 2 | Claude | Not Started |

**Dependencies**: E3, E4, E5, E6, E10

**Demo Criteria**:
- Run 10-round combat; creatures remain valid
- Damage formula computes fresh (no lookup tables)
- Formation disruption works

**Exit DoD**:
- [ ] Combat system complete
- [ ] Formation mechanics validated
- [ ] Combat UI functional
- [ ] CI passes

**Risk Focus**: Combat balance (R7), Chronicler label quality (R6)

---

### Sprint S12: Chronicler Events & Labeling (W12)

**Sprint Goal**: Record simulation events and integrate labels with UI.

**Points**: 32 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 12.1 | Event recording (birth, death, extinction, phenotype) | 6 | Solo | Not Started |
| 12.2 | Primitive signature clustering (stability test) | 8 | Claude | Not Started |
| 12.3 | Label generation heuristics (map clusters to names) | 6 | Solo | Not Started |
| 12.4 | Confidence scoring (frequency, stability) | 4 | Claude | Not Started |
| 12.5 | QueryAPI integration with Bestiary | 4 | Solo | Not Started |
| 12.6 | Chronicler persistence (save/load) | 2 | Claude | Not Started |
| 12.7 | 1000-tick chronicle test (5+ behaviors, confidence > 0.7) | 2 | Claude | Not Started |

**Dependencies**: E4, E10

**Demo Criteria**:
- Chronicle 1000 ticks; detect 5+ behaviors
- Assign labels with confidence > 0.7
- Bestiary search by label works

**Exit DoD**:
- [ ] Event recording complete
- [ ] Pattern detection with confidence
- [ ] Bestiary integration works
- [ ] CI passes

---

### Sprint S13: MVP Integration & Polish (W13)

**Sprint Goal**: Integrate all subsystems into playable MVP.

**Points**: 52 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 13.1 | Main menu & state machine | 6 | Solo | Not Started |
| 13.2 | New game initialization | 6 | Claude | Not Started |
| 13.3 | Input handling (keyboard, mouse) | 6 | Solo | Not Started |
| 13.4 | Settings UI & persistence | 4 | Claude | Not Started |
| 13.5 | Game state transitions (save/load/pause/resume) | 6 | Solo | Not Started |
| 13.6 | Full determinism validation (100-tick replay) | 8 | Claude | Not Started |
| 13.7 | Performance profiling & optimization | 8 | Solo | Not Started |
| 13.8 | Build & CI integration | 4 | Claude | Not Started |
| 13.9 | Documentation (README, guides) | 4 | Solo | Not Started |
| 13.10 | MVP demo script & final QA | 2 | Claude | Not Started |

**Dependencies**: E1–E12

**Demo Criteria**:
- Play full game loop: new game → explore → encounter → combat → save → load
- All MVP features functional
- Determinism test passes
- Performance within budget

**Exit DoD**:
- [ ] MVP feature-complete
- [ ] All systems integrated
- [ ] Determinism validated (1000 ticks)
- [ ] CI passes
- [ ] Ready for release

---

### Sprint S14: Final Refinement & MVP Release (W14)

**Sprint Goal**: Finish MVP and prepare for deep system branching.

**Points**: 40 (estimated)

**Stories**:

| ID | Title | Points | Owner | Status |
|----|-------|--------|-------|--------|
| 14.1 | Bug fixes & edge cases (from playtesting) | 12 | Solo+Claude | Not Started |
| 14.2 | UI/UX polish (hover states, error messages) | 8 | Claude | Not Started |
| 14.3 | Creature variety (3 more starter species/variants) | 6 | Solo | Not Started |
| 14.4 | Final determinism validation (1000-tick replay) | 8 | Claude | Not Started |
| 14.5 | Performance tuning (if needed from profiling) | 6 | Solo | Not Started |

**Demo Criteria**:
- MVP feature-complete and stable
- Playtesting feedback incorporated
- Determinism validated to 1000+ ticks
- Ready to ship

**Exit DoD**:
- [ ] All bugs fixed
- [ ] Determinism validated to 1000 ticks
- [ ] MVP shippable
- [ ] Deep system branching ready
- [ ] CI passes

---

## Phase 4: Deep System Branching (Weeks 15–18)

At end of S14, team chooses **one deep system** (A/B/C/D) for S15–S18.

### Option A: Evolution Depth (S15–S18A, 150 points)

**S15A (W15)**: Sexual reproduction & recombination (40 pts)
- Implement crossover operator (mate two genomes)
- Mating system in simulation
- Test recombination produces novel genotypes

**S16A (W16)**: Regulatory network complexity (40 pts)
- Add feedback regulators (A inhibits B, B inhibits A)
- Time-delay components
- Test network stability

**S17A (W17)**: Fitness landscape visualization (40 pts)
- UI overlay showing fitness across channel space
- Optional genetic algorithms (hill-climbing)
- Test creature convergence toward optima

**S18A (W18)**: Advanced speciation (30 pts)
- Reproductive isolation (distant mates → lower fertility)
- Sympatric speciation (disruptive selection)
- Test: 1000 ticks → 2 species emerge in single biome

---

### Option B: Disease (S15–S18B, 150 points)

**S15B (W15)**: Pathogen entity & scale-band filtering (40 pts)
- Create Pathogen component (micro-scale)
- Scale-band filtering (pathogen channels dormant in macro)
- HostCouplingProfile computation

**S16B (W16)**: Transmission network & SEIR (40 pts)
- Build transmission contact graph
- SEIR state transitions
- Test: 10% infected → 80% after 50 ticks

**S17B (W17)**: Virulence evolution & arms race (40 pts)
- Pathogen virulence evolution
- Host immune evolution
- Test: arms race dynamics cycle

**S18B (W18)**: Epidemic aftermath (30 pts)
- Pathogen extinction vs. endemic equilibrium
- Test: 1000-tick simulation produces stable endpoint

---

### Option C: Economy (S15–S18C, 150 points)

**S15C (W15)**: Settlement entity & resources (40 pts)
- Settlement component (faction, production)
- Resource types & per-biome production
- Test: 5 settlements produce by biome type

**S16C (W16)**: Trade network & supply/demand (40 pts)
- Trade connections between adjacent settlements
- Supply/demand dynamics
- Crafting recipes
- Test: 10-settlement network reaches equilibrium

**S17C (W17)**: Economic specialization (40 pts)
- Settlement specialization emergence
- Creature evolution responds to economic niche
- Player recruitment from settlements
- Test: specialization emerges; creature evolution correlates

**S18C (W18)**: Keeper economy & long-term loop (30 pts)
- Keeper trade with settlements, earn resources
- Resource constraints force strategic choices
- Test: 1000-tick economy produces stable settlements

---

### Option D: Culture (S15–S18D, 150 points)

**S15D (W15)**: Communication signals & language (40 pts)
- Vocalizer entity (broadcasts signals)
- Listeners receive & interpret signals
- Test: matching vocal/auditory → shared signals

**S16D (W16)**: Cultural drift & social learning (40 pts)
- Cultural traits (aesthetics, behaviors)
- Spread mechanism (proximity → adoption)
- Test: isolated populations → distinct culture; mix → blend

**S17D (W17)**: Historiography & unreliable narrator (40 pts)
- Chronicler records major events
- NPCs debate history with faction bias
- Player encounters 2-3 contradictory accounts
- Test: 1000 ticks → 5+ events with 3+ retellings each

**S18D (W18)**: Lore UI & player archaeology (30 pts)
- Historical Records UI screen
- Player annotations & theory building
- Test: 1000-tick gameplay surfaces coherent (if ambiguous) history

---

## Dependencies & Critical Path

```
S1 (E1: Foundations)
  ↓
S2 (E2: Registries) ← S1
  ↓
S3 (E3: Genome) ← S1, S2
  ↓
S4 (E4: Interpreter) ← S1, S2, S3
  ↓
S5 (E5: ECS) ← S1, S2 [parallel]
S6 (E6: Tick Loop) ← S1, S3, S4, S5
  ↓
S7 (E7: Save/Load) ← S1, S5, S6
S8 (E8: World Gen) ← S1, S3, S4, S5, S6 [parallel]
S9 (E9: Render) ← S3, S4, S5, S8 [parallel]
  ↓
S10 (E10: UI) ← S5, S9
S11 (E11: Combat) ← S3, S4, S5, S6, S10 [parallel]
S12 (E12: Chronicler) ← S4, S10 [parallel]
  ↓
S13 (E13: MVP Integration) ← E1–E12 (serial, tight integration)
  ↓
S14 (Final Polish & Release)
  ↓
S15–S18 (E14: Deep System) ← S14 (branching on A/B/C/D)
```

**Critical Path** (longest dependency chain):
S1 → S2 → S3 → S4 → S6 → S13 → S14

**Parallel Opportunities**:
- S5 (ECS) can start during S2 (Registries)
- S8 (World Gen) can start during S3 (Genome)
- S9 (Render) can start during S4 (Interpreter)
- S10 (UI) and S11 (Combat) can overlap (both depend on S5, S9, S6)
- S11 (Combat) and S12 (Chronicler) can overlap

---

## Velocity Tracking Template

**Weekly Burn-Down** (track actual points completed):

| Sprint | Planned | Completed | Velocity | Notes |
|--------|---------|-----------|----------|-------|
| S1 | 40 | TBD | TBD | |
| S2 | 38 | TBD | TBD | |
| S3 | 40 | TBD | TBD | |
| ... | | | | |
| **Total MVP (S1–S14)** | **~480** | TBD | TBD | |

**Adjustment Rule**:
- If velocity < 36 pts/sprint (10% miss): add 1 sprint to timeline
- If velocity > 44 pts/sprint (10% beat): shorten by 1 sprint; consider scope increases

---

## Milestone & Gate Criteria

| Milestone | Sprint | Gate Criteria |
|-----------|--------|---------------|
| **M1: Core Loop** | S6 | Single creature runs 100 ticks; state hash computed |
| **M2: Determinism** | S7 | Save at T50 → Replay → All hashes match (100 ticks) |
| **M3: Labels** | S12 | 1000-tick run; 5+ emergent behaviors labeled (confidence > 0.7) |
| **M4: MVP Playable** | S14 | Full game loop: new game → explore → encounter → save → load |
| **M5: Deep System** | S18 | Chosen system fully integrated; new mechanics visible |

---

## Definition of Ready (Sprint Entry)

Before sprint begins:
- [ ] All stories in sprint have clear acceptance criteria
- [ ] Story points estimated (team consensus)
- [ ] Dependencies identified and resolved
- [ ] Blockers from prior sprint addressed
- [ ] Team capacity clear (hours available per dev)

## Definition of Done (Sprint Exit)

Before sprint ends:
- [ ] All stories completed (100% of planned points, or burn-down explainable)
- [ ] CI passes (cargo build, cargo test, clippy)
- [ ] Demo recorded or walkthrough documented
- [ ] Bugs logged (any found during testing)
- [ ] Velocity recorded; adjusted estimates for next sprint if needed
