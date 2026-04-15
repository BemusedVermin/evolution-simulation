# Beast Evolution Game: Architecture Documentation

This directory contains comprehensive architectural documentation for the Beast Evolution Game implementation.

## Documents

### 1. **IMPLEMENTATION_ARCHITECTURE.md** (61 KB)
**Primary architecture document.** Start here.

Covers:
- Executive summary (stack, principles, horizon)
- 17-crate workspace layout with dependency DAG
- Tradeoff matrices for all major library choices (specs vs. bevy_ecs, fixed vs. rug, etc.)
- ECS architecture (entity kinds, component mapping, system schedule)
- Data flow through one simulation tick
- Determinism guards (sorted iteration, fixed-point wrappers, replay validation)
- Rendering architecture (two-mode renderer: world map + encounter view)
- Mod system (data-only in MVP)
- Scale-band specialization for pathogens/parasites
- Chronicler integration (labeling pipeline, query API)
- Testing strategy (unit, property, snapshot, determinism)
- Build & CI (cargo workspace, determinism replay job)
- MVP scope vs. one-full-deep-system horizon
- Open questions & risks

**Read if**: You need to understand the overall system, technology choices, and how everything fits together.

---

### 2. **CRATE_LAYOUT.md** (19 KB)
**Detailed crate organization and module structure.**

Covers:
- Workspace file tree
- All 17 crates with purpose, key types, dependencies, and modules
- Strict layering rules (L0 → L1 → L2 → ... → L6)
- Inter-crate dependency diagram
- Testing structure per crate
- Compilation notes
- Future expansion hooks

**Read if**: You need to understand which crate does what, how to navigate the codebase, and where to add new functionality.

---

### 3. **ECS_SCHEDULE.md** (23 KB)
**Detailed system schedule, parallelism model, and determinism invariants.**

Covers:
- 8-stage tick loop with ASCII diagram
- Per-stage system details (components read/write, parallelism, RNG stream)
- Parallelism & determinism rules (4 hard rules)
- Performance budget allocation per system (~16ms total)
- System implementation template
- Debugging & profiling techniques

**Read if**: You're implementing systems or need to understand the tick loop order and performance constraints.

---

## Quick Reference

### Stack
- **Language**: Rust
- **ECS**: specs
- **Graphics**: SDL3
- **Math**: fixed (Q32.32)
- **PRNG**: rand_xoshiro (Xoshiro256PlusPlus)
- **Serialization**: serde + bincode (deterministic)
- **UI**: Custom retained-mode on SDL3

### Key Invariants
1. **Determinism**: Q32.32 fixed-point, xoshiro256** PRNG, sorted iteration, no wall-clock
2. **Mechanics-Label Separation**: Simulation emits primitives; UI assigns names (Chronicler)
3. **Simulation-First**: Combat, AI, ecology all derive from primitive effects
4. **Scale-Band Unification**: Macro hosts and micro pathogens use same genome/interpreter
5. **Channel Registry Monolithicism**: Single runtime registry; no hardcoded channel assumptions

### MVP Scope
- Single biome, 1-2 species, 100-1000 creatures
- Basic combat (1v1, small groups)
- Bestiary with observation tracking
- Binary save/load + full replay validation

### One Full Deep System (User Chooses)
- **Evolution**: Channel genesis, epistasis modeling, speciation
- **Disease**: Micro-scale pathogens, host coevolution, transmission networks
- **Economy**: Settlements, trading, crafting, resource management
- **Culture**: Faction naming, language drift, narrative generation

---

## Navigating the Code (Once Written)

```
beast-evolution-game/
├── crates/
│   ├── beast-core/              # Foundations (Q32.32, PRNG, errors)
│   ├── beast-channels/          # Channel registry
│   ├── beast-primitives/        # Primitive registry
│   ├── beast-genome/            # Genome, mutations
│   ├── beast-interpreter/       # Phenotype interpretation
│   ├── beast-evolution/         # Selection, fitness, populations
│   ├── beast-disease/           # Pathogen specialization
│   ├── beast-ecs/               # Components, systems
│   ├── beast-sim/               # Tick loop orchestration
│   ├── beast-chronicler/        # Pattern recognition, labeling
│   ├── beast-serde/             # Save/load, replay
│   ├── beast-render/            # SDL3 rendering
│   ├── beast-ui/                # Widget framework
│   ├── beast-audio/             # SDL3 audio (optional)
│   ├── beast-mod/               # Mod loading
│   ├── beast-cli/               # Headless testing
│   └── beast-app/               # Main binary
├── tests/
│   ├── determinism_test.rs      # Replay validation
│   └── fixtures/
└── architecture/                # THIS DIRECTORY
    ├── README.md
    ├── IMPLEMENTATION_ARCHITECTURE.md
    ├── CRATE_LAYOUT.md
    └── ECS_SCHEDULE.md
```

---

## Key Files to Read (from Design Phase)

Before implementing any crate, read these in order:

1. **INVARIANTS.md** (in parent dir)
   - Determinism contract
   - Mechanics-Label Separation
   - UI state vs. sim state boundary

2. **systems/01_evolutionary_model.md** through **systems/23_ui_overview.md**
   - Core design for each system
   - Research basis, entity definitions, mechanics

3. **schemas/README.md** + channel manifest examples
   - Channel families, composition hooks, expression conditions
   - Primitive vocabulary

4. **IMPLEMENTATION_ARCHITECTURE.md** (in this dir)
   - How the design is realized in code

---

## Determinism Testing (CI)

The CI job `test_deterministic_replay_1000_ticks` is critical:

```bash
# Locally
cargo test --test determinism_test -- --nocapture

# In CI
cargo test --test determinism_test || exit 1
```

Failure = binary diff of first diverging entity/component at first diverging tick. Investigate numerical precision, PRNG seeding, or iteration order.

---

## Development Workflow

### Phase 1: Crate Scaffolding (Week 1-2)
1. Create 17 crates with empty lib.rs
2. Set up Cargo.toml dependencies (follow DAG)
3. Define component types in beast-ecs
4. Implement beast-core types (Q32.32, Xoshiro256PlusPlus)

### Phase 2: Core Systems (Week 2-4)
1. Implement beast-channels (manifest loading, validation)
2. Implement beast-genome (mutation operators)
3. Implement beast-interpreter (phenotype resolution)

### Phase 3: ECS & Sim Loop (Week 4-6)
1. Implement beast-sim (tick loop orchestration)
2. Implement 8 system stages in order
3. Add determinism test; verify bit-identical replay

### Phase 4: Graphics & UI (Week 6-8)
1. Implement beast-render (SDL3 integration)
2. Implement beast-ui (widget framework)
3. Connect to beast-sim snapshot

### Phase 5: Polish & Testing (Week 8+)
1. Add benchmarks (criterion)
2. Add property tests (proptest)
3. Comprehensive determinism testing
4. Performance profiling & optimization

---

## Contact & Future Work

- **Determinism Invariant**: Non-negotiable; all code must be audit-able
- **Research-First**: Every mechanic backed by published biology/physics
- **Modular**: Design enables four different deep systems post-MVP

For questions on architectural decisions, see "Open Questions & Risks" section of IMPLEMENTATION_ARCHITECTURE.md.
