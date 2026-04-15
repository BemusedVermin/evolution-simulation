# Beast Evolution Game: Crate Layout & Module Organization

## Workspace Structure

```
beast-evolution-game/
├── Cargo.toml (workspace root)
├── crates/
│   ├── beast-core/              (L0: Foundations)
│   ├── beast-channels/          (L1: Channel registry)
│   ├── beast-primitives/        (L1: Primitive registry)
│   ├── beast-genome/            (L1: Genotype)
│   ├── beast-interpreter/       (L2: Phenotype → effects)
│   ├── beast-evolution/         (L2: Selection & fitness)
│   ├── beast-disease/           (L2: Pathogen specialization)
│   ├── beast-ecs/               (L3: ECS framework)
│   ├── beast-sim/               (L4: Orchestration)
│   ├── beast-chronicler/        (L4: Labeling & queries)
│   ├── beast-serde/             (L4: Persistence)
│   ├── beast-render/            (L5: SDL3 graphics)
│   ├── beast-ui/                (L5: Widget framework)
│   ├── beast-audio/             (L5: SDL3 audio)
│   ├── beast-mod/               (L6: Mod loading)
│   ├── beast-cli/               (L6: Testing & replay)
│   └── beast-app/               (L6: Main binary)
├── tests/                       (Integration tests)
│   └── determinism_test.rs
├── assets/
│   ├── manifests/               (Core channels, primitives, biomes)
│   ├── fixtures/                (Test data)
│   └── sprites/                 (Sprite atlases)
└── docs/
    ├── IMPLEMENTATION_ARCHITECTURE.md  (this file)
    ├── CRATE_LAYOUT.md
    ├── ECS_SCHEDULE.md
    └── DATA_FLOW.md
```

---

## Layer 0: Foundations

### beast-core

**Purpose**: Primitives used by all other crates. No dependencies on other beast crates.

**Key Types**:
- `Q3232` (fixed-point Q32.32) — wrapper around `fixed::I32F32`
- `Xoshiro256PlusPlus` — PRNG (from `rand_xoshiro`)
- `EntityID` (u32 wrapper)
- `Result<T>` (custom error type)
- `TickCounter` (u64 simulation time)
- Math utilities (saturating ops, Box-Muller, gaussian sampling)

**Dependencies**: `fixed`, `rand_xoshiro`, `serde`

**Modules**:
```rust
pub mod fixed_point;       // Q3232 wrapper, deterministic math
pub mod prng;              // Xoshiro256PlusPlus, seeding, splitting
pub mod error;             // Result type, Error enum
pub mod entity;            // EntityID, entity types
pub mod time;              // TickCounter, schedule utilities
pub mod math;              // Gaussian sampling, saturating ops
```

---

## Layer 1: Data Definitions & Genetics

### beast-channels

**Purpose**: Channel registry, manifest loading, schema validation.

**Key Types**:
- `ChannelManifest` (JSON schema for channels)
- `ChannelRegistry` (in-memory store, queryable by ID or family)
- `ChannelFamily` (enum: sensory, motor, metabolic, etc.)
- `CompositionHook` (rule for multi-channel interaction)
- `ExpressionConditions` (biome, scale_band, season gates)

**Dependencies**: `beast-core`, `serde_json`, `jsonschema`

**Modules**:
```rust
pub mod manifest;          // ChannelManifest definition
pub mod registry;          // ChannelRegistry, loading
pub mod composition;       // CompositionHook logic
pub mod expression;        // ExpressionConditions evaluation
pub mod schema;            // JSON schema validation
```

**Key Functions**:
```rust
pub fn load_channel_manifest(path: &Path) -> Result<ChannelManifest>;
pub fn validate_manifest(manifest: &ChannelManifest, schema: &JsonSchema) -> Result<()>;
```

---

### beast-primitives

**Purpose**: Primitive effect registry, manifests, cost functions.

**Key Types**:
- `PrimitiveManifest` (definition of emit_acoustic_pulse, etc.)
- `PrimitiveRegistry` (searchable by ID, category)
- `PrimitiveEffect` (runtime emission: primitive_id, parameters, source_channels)
- `PrimitiveCategory` (enum: signal_emission, force_application, etc.)

**Dependencies**: `beast-core`, `serde_json`

**Modules**:
```rust
pub mod manifest;          // PrimitiveManifest
pub mod registry;          // PrimitiveRegistry
pub mod effect;            // PrimitiveEffect, emission
pub mod category;          // Category taxonomy
pub mod cost;              // Cost function evaluation
```

---

### beast-genome

**Purpose**: Genotype storage, mutation operators, channel genesis.

**Key Types**:
- `Genome` (collection of TraitGenes)
- `TraitGene` (effect_vector, body_site, regulatory_modifiers, enabled)
- `Mutator` (point mutation, regulatory rewiring, duplication, divergence, loss, silencing, body-site shift)
- `BodySite` (location on creature where gene expresses)

**Dependencies**: `beast-core`, `beast-channels`, `serde`

**Modules**:
```rust
pub mod genome;            // Genome struct, genotype operations
pub mod trait_gene;        // TraitGene definition, modifiers
pub mod mutator;           // Mutation operations
pub mod body_site;         // Body region modeling
pub mod genesis;           // Gene duplication, reclassification
```

**Key Functions**:
```rust
pub fn mutate_point(rng: &mut Xoshiro256PlusPlus, genome: &mut Genome) -> Result<()>;
pub fn mutate_duplication(rng: &mut Xoshiro256PlusPlus, genome: &mut Genome) -> Result<()>;
pub fn mutate_reclassify(genome: &mut Genome, new_family: ChannelFamily) -> Result<()>;
```

---

## Layer 2: Interpretation & Evolution

### beast-interpreter

**Purpose**: Convert genotype → phenotype (primitive effects).

**Key Types**:
- `PhenotypeInterpreter` (deterministic evaluation engine)
- `ResolvedPhenotype` (active channels, body map, life stage)
- `BodyRegion` (head, limbs, tail, core, etc. with per-region channel values)

**Dependencies**: `beast-core`, `beast-channels`, `beast-primitives`, `beast-genome`

**Modules**:
```rust
pub mod interpreter;       // Interpreter struct, run() method
pub mod phenotype;         // ResolvedPhenotype, channel resolution
pub mod body_map;          // BodyRegion aggregation per site
pub mod composition;       // Composition hook evaluation (additive, multiplicative, threshold)
pub mod expression;        // Expression condition filtering (biome, scale_band, season, stage)
pub mod emission;          // Primitive effect emission from hooks
```

**Key Functions**:
```rust
pub fn interpret_phenotype(
    genome: &Genome,
    environment: &Environment,
    channel_registry: &ChannelRegistry,
    primitive_registry: &PrimitiveRegistry,
) -> Result<Set<PrimitiveEffect>>;
```

---

### beast-evolution

**Purpose**: Fitness evaluation, selection, population dynamics.

**Key Types**:
- `Population` (collection of creatures with fitness scores)
- `FitnessEvaluator` (environment-dependent, multi-objective fitness)
- `SelectionOperator` (tournament, roulette wheel, truncation)
- `PopulationDynamics` (carrying capacity, migration, extinction risk)

**Dependencies**: `beast-core`, `beast-interpreter`, `beast-genome`

**Modules**:
```rust
pub mod population;        // Population struct
pub mod fitness;           // Fitness evaluation (multi-objective)
pub mod selection;         // Selection operators
pub mod dynamics;          // Population dynamics (birth, death, migration)
pub mod epistasis;         // Global epistasis penalty (Diaz-Colunga)
```

**Key Functions**:
```rust
pub fn evaluate_fitness(
    creature: &Creature,
    environment: &Environment,
    rng: &mut Xoshiro256PlusPlus,
) -> Result<f32>;

pub fn apply_selection(
    population: &mut Population,
    operator: SelectionOperator,
) -> Result<()>;
```

---

### beast-disease

**Purpose**: Pathogen-specific specialization (micro-scale filtering, host coupling).

**Key Types**:
- `HostCouplingProfile` (energetic_drain, transmission_efficiency, virulence, benefit)
- `TransmissionNetwork` (host-pathogen contact graph)
- `PathogenEvaluator` (fitness in context of host)

**Dependencies**: `beast-core`, `beast-interpreter`, `beast-evolution`, `beast-channels`

**Modules**:
```rust
pub mod host_coupling;     // HostCouplingProfile computation
pub mod transmission;      // Transmission success, SEIR-like
pub mod evaluator;         // Pathogen-specific fitness
pub mod scale_band;        // Micro-scale filtering
```

---

## Layer 3: ECS Foundation

### beast-ecs

**Purpose**: specs World, components, and system traits.

**Key Types**:
- `EcsWorld` (wrapper around specs World)
- `Creature`, `Pathogen`, `Agent`, `Faction`, `Settlement`, `Biome` (marker components)
- `Genome`, `Phenotype`, `HealthState`, `Position`, etc. (data components)
- `System` trait (for all systems)
- `Resources` (global mutable state: registries, PRNG streams)

**Dependencies**: `specs`, `beast-core`, `beast-channels`, `beast-primitives`

**Modules**:
```rust
pub mod world;             // EcsWorld wrapper
pub mod components;        // All component types
pub mod system;            // System trait definition
pub mod storage;           // Component storage adapters
pub mod resources;         // Global Resources struct
pub mod entity_id;         // Deterministic entity ID generation
```

---

## Layer 4: Simulation & Persistence

### beast-sim

**Purpose**: Tick loop orchestration, system scheduling, determinism guards.

**Key Types**:
- `Simulation` (main state machine)
- `SimulationConfig` (parameters: world_seed, tick_budget, etc.)
- `TickResult` (summary of one tick: events, state hash)
- `SystemSchedule` (ordered stages)

**Dependencies**: `beast-core`, `beast-ecs`, `beast-interpreter`, `beast-evolution`, `beast-disease`

**Modules**:
```rust
pub mod simulation;        // Simulation struct, game loop
pub mod schedule;          // SystemSchedule, system ordering
pub mod tick;              // Per-tick orchestration
pub mod budget;            // Performance budget tracking
pub mod determinism;       // State hash, sorted iteration helper
```

**Key Functions**:
```rust
pub fn tick(&mut self, input: &InputEvent) -> Result<TickResult>;
pub fn compute_state_hash(&self) -> u64;  // For determinism testing
```

---

### beast-chronicler

**Purpose**: Pattern recognition, labeling, query API.

**Key Types**:
- `Chronicler` (event log, pattern index, label map)
- `ChronicleEntry` (recorded event)
- `PatternSignature` (hashed primitive cluster)
- `Label` (assigned name: "echolocation", "aggressive", etc.)
- `QueryAPI` (methods for UI to query labels)

**Dependencies**: `beast-core`, `beast-ecs`, `beast-primitives`

**Modules**:
```rust
pub mod chronicler;        // Chronicler struct
pub mod pattern;           // Pattern detection, clustering
pub mod label;             // Label generation, naming heuristics
pub mod query;             // Query API for UI
pub mod confidence;        // Confidence scoring for labels
```

**Key Functions**:
```rust
pub fn detect_patterns(creatures: &[Creature]) -> HashMap<PatternSignature, usize>;
pub fn assign_labels(&mut self, patterns: HashMap<PatternSignature, usize>) -> Result<()>;
pub fn query_label_for_signature(&self, sig: &str) -> Option<String>;
pub fn query_creatures_with_label(&self, label: &str) -> Vec<EntityID>;
```

---

### beast-serde

**Purpose**: Save/load, replay journaling, deterministic serialization, validation.

**Key Types**:
- `SaveFile` (wrapper: metadata + serialized world state)
- `ReplayJournal` (sequence of inputs for determinism testing)
- `SaveManager` (load/save orchestration)
- `SaveValidator` (schema validation, forbidden-key rejection)

**Dependencies**: `beast-core`, `beast-ecs`, `beast-sim`, `serde`, `bincode`, `serde_json`, `jsonschema`

**Modules**:
```rust
pub mod save;              // SaveFile struct, save/load
pub mod replay;            // ReplayJournal, replay validation
pub mod manager;           // SaveManager
pub mod migration;         // Schema versioning, migrations
pub mod deterministic;     // Deterministic serialization helpers
pub mod validator;         // SaveValidator, schema validation, forbidden-key checks
```

**Key Functions**:
```rust
pub fn save_game(sim: &Simulation, path: &Path) -> Result<()>;
pub fn load_game(path: &Path) -> Result<Simulation>;
pub fn save_replay_journal(inputs: &[InputEvent], path: &Path) -> Result<()>;
pub fn load_replay_journal(path: &Path) -> Result<Vec<InputEvent>>;
pub fn validate_save_file(path: &Path) -> Result<()>;
pub fn validate_save_json(json: &serde_json::Value) -> Result<()>;
```

**SaveValidator** (new module):
- Validates save files against JSON Schema at load time
- Rejects forbidden keys: `bestiary_discovered`, `ui_*` (UI-ephemeral state)
- Ensures deterministic replay: no corrupted PRNG state, entity IDs intact
- Test requirement: hand-crafted invalid save file must be rejected with clear error message

---

## Layer 5: Rendering & UI

### beast-render

**Purpose**: SDL3 integration, sprite/mesh pipeline, draw calls.

**Key Types**:
- `Renderer` (SDL3 wrapper, canvas, sprite atlas, render mode)
- `RenderMode` (WorldMap, EncounterView)
- `DrawCommand` (batched render instructions)
- `SpriteAtlas` (texture atlas, sprite lookup)
- `VisualDirective` (per-entity: mesh_id, color, particles, animation)

**Dependencies**: `beast-core`, `beast-sim`, `sdl3-sys`

**Modules**:
```rust
pub mod renderer;          // Renderer struct
pub mod modes;             // RenderMode (world map, encounter)
pub mod sprite;            // SpriteAtlas, sprite management
pub mod mesh;              // Mesh definitions, drawing
pub mod camera;            // Camera (world map zoom, encounter perspective)
pub mod batching;          // Draw call batching
```

**Key Functions**:
```rust
pub fn render_frame(&mut self, snapshot: &SimulationSnapshot) -> Result<()>;
pub fn set_render_mode(&mut self, mode: RenderMode);
```

---

### beast-ui

**Purpose**: Retained-mode widget framework, screen hierarchy, data binding.

**Key Types**:
- `WidgetTree` (hierarchy of Widgets)
- `Widget` trait (layout, event handling, rendering)
- `Button`, `List`, `Card`, `Dialog`, `Chart` (widget primitives)
- `UIState` (screen state, open tabs, filters, selections)
- `DataBinding` (reference to UI-relevant sim state)

**Dependencies**: `beast-core`, `beast-render`, `beast-chronicler`

**Modules**:
```rust
pub mod widget;            // Widget trait, primitives
pub mod tree;              // WidgetTree, hierarchy
pub mod layout;            // Layout engine (flex-like)
pub mod event;             // Event dispatch
pub mod binding;           // Data binding to sim state
pub mod screen;            // Screen definitions (WorldMapScreen, etc.)
pub mod styling;           // CSS-like styling
```

**Key Functions**:
```rust
pub fn layout_widgets(&mut self, available: Rect);
pub fn on_event(&mut self, event: &InputEvent) -> EventResult;
pub fn render(&self, renderer: &mut Renderer);
```

---

### beast-audio

**Purpose**: SDL3 audio playback, music, sound effects (optional for MVP).

**Key Types**:
- `AudioManager` (channel mixer, volume control)
- `SoundEffect` (loaded WAV/MP3)
- `Music` (background track)

**Dependencies**: `beast-core`, `sdl3-sys`

**Modules**:
```rust
pub mod manager;           // AudioManager
pub mod sound;             // SoundEffect, Music
pub mod mixer;             // Channel mixing, panning
```

---

## Layer 6: Extensibility & Binaries

### beast-mod

**Purpose**: Mod loading, manifest merging, validation.

**Key Types**:
- `ModLoader` (file I/O, JSON parsing)
- `ModMetadata` (id, version, author, load_order)
- `LoadedMod` (channels, primitives, loaded from disk)
- `ManifestMerger` (core + mod manifests → unified registries)

**Dependencies**: `beast-core`, `beast-channels`, `beast-primitives`, `serde_json`, `jsonschema`

**Modules**:
```rust
pub mod loader;            // ModLoader
pub mod metadata;          // ModMetadata
pub mod merger;            // ManifestMerger
pub mod validation;        // Manifest schema validation
pub mod io;                // File I/O, mod directory structure
```

**Key Functions**:
```rust
pub fn load_mod(mod_dir: &Path) -> Result<LoadedMod>;
pub fn merge_registries(
    core_channels: Vec<ChannelManifest>,
    mods: Vec<LoadedMod>,
) -> Result<(ChannelRegistry, PrimitiveRegistry)>;
```

---

### beast-cli

**Purpose**: Headless simulation, testing, replay validation.

**Key Types**:
- `ReplayValidator` (orchestrates determinism tests)
- `SimRunner` (runs N ticks with optional profiling)

**Dependencies**: `beast-core`, `beast-sim`, `beast-serde`, `criterion`

**Modules**:
```rust
pub mod validator;         // ReplayValidator
pub mod runner;            // SimRunner
pub mod profiler;          // Tick profiling (per-system timing)
pub mod bench;             // Criterion benchmarks
```

**Key Functions**:
```rust
pub fn validate_replay(save_file: &Path, input_journal: &Path) -> Result<()>;
pub fn run_simulation_headless(config: SimConfig, ticks: u32) -> Result<Vec<TickResult>>;
pub fn profile_tick_breakdown(sim: &Simulation) -> HashMap<SystemStage, Duration>;
```

---

### beast-app

**Purpose**: Main binary, initialization, game loop wiring.

**Key Types**:
- `App` (top-level game state machine)
- `GameLoop` (60 FPS tick + render loop)
- `InputHandler` (SDL3 event polling)

**Dependencies**: All crates, especially `beast-sim`, `beast-render`, `beast-ui`, `beast-serde`

**Modules**:
```rust
pub mod app;               // App initialization, state machine
pub mod loop;              // GameLoop, frame timing
pub mod input;             // InputHandler (keyboard, mouse)
pub mod menu;              // Main menu state
pub mod game;              // Active game state
pub mod settings;          // Settings/config
```

**Key Function**:
```rust
pub fn main() {
    let mut app = App::new()?;
    app.run()?;  // Starts 60 FPS game loop
}
```

---

## Inter-Crate Dependencies: Strict Layering

```
beast-app
  ↓ depends on
beast-cli, beast-mod, beast-render, beast-ui
  ↓ depend on
beast-sim, beast-chronicler, beast-serde
  ↓ depend on
beast-ecs, beast-disease, beast-evolution, beast-interpreter
  ↓ depend on
beast-genome, beast-channels, beast-primitives
  ↓ depend on
beast-core

Rules:
- L0 (beast-core) ← no dependencies on other beast crates
- L1 → L0 only
- L2 → L0, L1 only
- L3 → L0, L1 only
- L4 → L0, L1, L2, L3 only
- L5 → all of above
- L6 → all of above (but not interdependent)

NO circular dependencies allowed.
```

---

## Testing Structure

```
tests/
  ├── determinism_test.rs         (replay validation)
  ├── fixtures/
  │   ├── determinism_test.json   (save state)
  │   └── determinism_test_inputs.json
  └── integration_tests.rs        (multi-crate behavior)

Each crate/src/lib.rs:
  #[cfg(test)]
  mod tests { ... }
  
  #[cfg(test)]
  mod property_tests { ... }
```

---

## Compilation Notes

- **Total workspace**: ~17 crates, ~60K lines of Rust (estimated MVP)
- **Compile time**: ~10–15s clean (beast-core cached), ~2–3s incremental
- **Binary size**: ~50–100 MB (release, with SDL3 + all deps)
- **Dependencies**: 50–70 direct crates (carefully audited; see Cargo.deny)

---

## Future Expansion (Post-MVP)

- **Deep System: Evolution** → expand beast-genome, beast-evolution
- **Deep System: Disease** → expand beast-disease, beast-ecs (new Infection component)
- **Deep System: Economy** → new beast-economy crate, Settlement entities
- **Deep System: Culture** → expand beast-chronicler, new beast-language crate
- **Scripting** (if modding deepens) → new beast-script crate (Lua/WASM VM)
- **Networking** (if multiplayer) → new beast-net crate (lockstep or cloud)
