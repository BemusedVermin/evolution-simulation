# Beast Evolution Game: Implementation Architecture

## Table of Contents
1. Executive Summary
2. Crate Layout & Dependency Graph
3. Recommended Open-Source Libraries
4. ECS Architecture
5. Data Flow: Simulation Tick
6. Determinism Guards
7. Rendering Architecture
8. Mod System
9. Scale-Band Specialization (Disease)
10. Chronicler Integration
11. Testing Strategy
12. Build & CI
13. MVP Scope vs. Deep System Horizon
14. Open Questions & Risks

---

## 1. Executive Summary

**Beast Evolution Game** is a simulation-first creature evolution and encounter game. The implementation is grounded in three non-negotiable principles:

1. **Determinism First**: All simulation state uses fixed-point arithmetic (Q32.32); xoshiro256** PRNG with one stream per subsystem; sorted iteration; no wall-clock dependencies. Replay validation in CI: save → run 1000 ticks → snapshot → replay → bit-identical hash.

2. **Simulation-First Design**: Combat, UI feedback, and emergent behaviors all arise from primitive effect emissions driven by evolved channel phenotypes. There are zero hardcoded ability names in simulation code (Mechanics-Label Separation invariant). The Chronicler assigns labels to recurring primitive clusters post-hoc.

3. **Modular, Research-Backed Architecture**: Every system is grounded in first-principles biology/physics. Channels evolve under Fisher's Geometric Model, composition hooks implement epistasis, Kleiber's Law governs scale-band unification, and transmission dynamics follow Anderson-May epidemiology.

**Tech Stack**:
- **Language**: Rust (memory safety, performance, determinism)
- **ECS**: specs (proven, data-driven, flexible)
- **Graphics**: SDL3 (via sdl3 crate)
- **Math**: fixed (Q32.32) + rand_xoshiro (PRNG)
- **Serialization**: serde + bincode (deterministic)
- **UI**: Custom retained-mode widget layer on SDL3
- **Workspace**: 17 crates organized by domain

**Horizon**: MVP (world map + single biome + 1-2 species + combat + bestiary) + one full deep system (user picks during planning: evolution depth, disease, economy, or culture).

---

## 2. Crate Layout & Dependency Graph

A Rust workspace with 17 crates. Dependency DAG ensures layered abstraction:

```
Layers:
  L0: beast-core
  L1: beast-channels, beast-primitives, beast-genome
  L2: beast-interpreter, beast-evolution
  L3: beast-disease, beast-ecs
  L4: beast-sim, beast-chronicler, beast-serde
  L5: beast-render, beast-ui
  L6: beast-mod, beast-cli, beast-app
```

### Crate Descriptions

| Crate | Purpose | Key Types | Dependencies |
|-------|---------|-----------|--------------|
| **beast-core** | Fixed-point math, PRNG, error types | `Q3232`, `Xoshiro256PlusPlus`, `Result<T>` | `fixed`, `rand_xoshiro`, `serde` |
| **beast-channels** | Channel registry, manifest loading | `ChannelRegistry`, `ChannelManifest`, `CompositionHook` | `beast-core`, `serde_json` |
| **beast-primitives** | Primitive registry, effect types | `PrimitiveRegistry`, `PrimitiveEffect`, `PrimitiveManifest` | `beast-core`, `serde_json` |
| **beast-genome** | Genotype storage, mutation ops | `Genome`, `TraitGene`, `Mutator` | `beast-core`, `beast-channels` |
| **beast-interpreter** | Phenotype → PrimitiveEffect | `PhenotypeInterpreter`, `ResolvedPhenotype` | `beast-genome`, `beast-channels`, `beast-primitives` |
| **beast-evolution** | Selection, fitness, population dynamics | `Population`, `FitnessEvaluator`, `SelectionOperator` | `beast-genome`, `beast-interpreter` |
| **beast-disease** | Micro-scale specialization (scale-band filtering) | `HostCouplingProfile`, `PathogenEvaluator` | `beast-interpreter`, `beast-evolution` |
| **beast-ecs** | specs World, component defs, system schedule | `EcsWorld`, `Creature`, `Agent`, System traits | `specs`, `beast-core` |
| **beast-sim** | Orchestration, tick loop | `Simulation`, `SimulationConfig`, `TickResult` | `beast-ecs`, `beast-interpreter` |
| **beast-chronicler** | Pattern recognition, labeling, query API | `Chronicler`, `ChronicleEntry`, `QueryAPI` | `beast-ecs`, `beast-primitives` |
| **beast-serde** | Save/load, replay, deterministic serialization | `SaveFile`, `ReplayJournal`, `SaveManager` | `beast-ecs`, `beast-sim`, `serde`, `bincode` |
| **beast-render** | SDL3 integration, sprite/mesh pipeline | `Renderer`, `SpriteAtlas`, `DrawCommand` | `beast-sim`, `beast-ui`, `sdl3`, `beast-core` |
| **beast-ui** | Retained-mode UI, widget tree | `WidgetTree`, `Button`, `List`, `Card` | `beast-core`, `beast-chronicler`, `beast-render` |
| **beast-audio** | SDL3 audio wrapper (optional) | `AudioManager`, `SoundEffect` | `sdl3`, `beast-core` |
| **beast-mod** | Mod loading, manifest merging | `ModLoader`, `ModMetadata`, `ManifestMerger` | `beast-channels`, `beast-primitives` |
| **beast-cli** | Headless sim, testing, replay validation | `ReplayValidator`, `SimRunner` | `beast-sim`, `beast-serde` |
| **beast-app** | Main binary, wiring | `App`, `GameLoop`, `EventHandler` | All of above |

### Dependency DAG Diagram

```
                           beast-app (main)
                              |
        ┌───────────────────────┼───────────────────────┐
        |                       |                       |
    beast-cli              beast-render            beast-mod
    (testing)              (graphics)             (extensibility)
        |                      |                       |
    beast-serde         ┌──────┴──────┐          beast-channels
    (persistence)       |             |          beast-primitives
        |           beast-ui      beast-render
        |             |
        |      beast-chronicler
        |      (labeling)
        |
    beast-sim (orchestration)
        |
        ├── beast-ecs (components & systems)
        |   └── beast-interpreter (phenotype)
        |       ├── beast-evolution (selection)
        |       ├── beast-disease (pathogen specialization)
        |       └── beast-genome (genotype)
        |           └── beast-channels (registry)
        |
        └── beast-primitives (effect definitions)

    Base Layer:
    beast-core (Q32.32, PRNG, errors)
    beast-channels (manifests)
    beast-primitives (effect manifests)
```

---

## 3. Recommended Open-Source Libraries

Each choice includes a **tradeoff matrix** comparing alternatives.

### 3.1 ECS Framework

**Choice**: **specs**

| Criterion | specs | bevy_ecs | legion | hecs |
|-----------|-------|----------|--------|------|
| **Data Layout** | SoA (cache-friendly) | SoA | SoA | AoS (slower) |
| **Serialization** | Straightforward | Complex (World is opaque) | Moderate | Good |
| **Determinism** | Excellent (iteration control) | Good (sorted iteration possible) | Good | Good |
| **Learning Curve** | Low | Moderate | Low | Very low |
| **Ecosystem** | Stable, standalone | Huge (tied to Bevy) | Growing | Small |
| **Parallelism** | rayon-compatible | Built-in async | Excellent | Limited |
| **Compile Time** | <5s | >10s | ~5s | ~3s |

**Justification**: specs prioritizes data-driven structure-of-arrays layout, making serialization of components straightforward. The World is fully introspectable, enabling replay validation. Iteration order is deterministic (sorted by entity ID). No tight coupling to a rendering framework—we control the render pipeline entirely via SDL3.

**Alternative Rejected**: bevy_ecs would couple us to Bevy's ecosystem and add 10+ seconds to compile times. legion is excellent but overkill; specs is proven in dozens of shipped games.

### 3.2 Fixed-Point Arithmetic

**Choice**: **fixed** crate (Q32.32)

| Criterion | fixed | rug | decimal | hand-rolled |
|-----------|-------|-----|---------|-------------|
| **Precision** | Q32.32 (64-bit) | Arbitrary | Arbitrary | Configurable |
| **Performance** | Native CPU (fastest) | GMP (slow) | Moderate | Fastest (if optimized) |
| **Determinism** | Perfect | Perfect | Perfect | Risky (impl bugs) |
| **Ease of Use** | Operator overloading | Complex API | Simple | Manual ops |
| **Serialization** | Trivial (u64) | Complex | Simple | Simple |
| **Testing** | Abundant examples | Rare | Moderate | DIY |

**Justification**: Q32.32 (32-bit integer, 32-bit fractional) gives [0, 1] range for channel values with ~2^-32 precision (≈0.23 nanounit), sufficient for mutation-driven evolution. Native CPU ops mean no performance penalty. The `fixed` crate is well-maintained and provides operator overloading, making code readable. All fixed-point operations are deterministic across platforms.

**Alternative Rejected**: Hand-rolling risks subtle bugs (overflow handling, rounding mode). rug and decimal are overkill and slow.

### 3.3 PRNG: Cryptographic + Seedable

**Choice**: **rand_xoshiro** with Xoshiro256PlusPlus

| Criterion | xoshiro256** | MT19937 | ChaCha20 | SFC64 |
|-----------|--------------|---------|----------|-------|
| **Cryptographic** | No (not needed) | No | Yes (overkill) | No |
| **Speed** | ~20 cycles/output | ~200 cycles | ~100 cycles | ~10 cycles |
| **State Size** | 256 bits | 19,968 bits | 512 bits | 256 bits |
| **Period** | 2^256 - 1 | 2^19937 - 1 | Infinite | 2^256 - 1 |
| **Determinism** | Perfect | Perfect | Perfect | Perfect |
| **Jump/Splitmix** | Yes (excellent) | No | No | No |

**Justification**: Xoshiro256** is fast, small-state, and designed for parallel splittable RNGs. The "jump" operation (xoshiro_state.jump()) advances the state by 2^128 iterations, perfect for seeding N independent streams from one root seed. No cryptographic overhead; we use SplitMix64 for seed expansion, a published technique.

**PRNG Architecture**:
```rust
// One root PRNG per world
let world_seed: u64 = /* user-provided or procedural */;
let mut root_rng = Xoshiro256PlusPlus::seed_from_u64(world_seed);

// Per-subsystem streams (never cross-contaminate)
let evolution_stream = {
    let mut rng = root_rng.clone();
    rng.jump();  // Advance by 2^128 steps
    rng
};
let ecology_stream = {
    let mut rng = root_rng.clone();
    rng.jump(); rng.jump();  // Another 2^128 steps
    rng
};
// ... repeat for each subsystem
```

Each stream is serialized in save files; replay restores exact state.

### 3.4 Serialization: Deterministic Format

**Choice**: **serde** + **bincode**

| Criterion | serde+bincode | serde+JSON | rkyv | postcard |
|-----------|---------------|-----------|------|----------|
| **Human-Readable** | No (binary) | Yes | No | No |
| **Determinism** | Perfect | Perfect | Perfect | Perfect |
| **Size** | Minimal (~bytes) | Large (~10x) | Minimal | Minimal |
| **Speed** | Fast (native endian) | Moderate (parsing) | Fastest (zero-copy) | Fast |
| **Versioning** | Moderate (custom impls) | Good (schema evolution) | Hard | Moderate |
| **Round-Trip Safety** | Excellent | Excellent | Excellent | Excellent |

**Justification**: bincode with serde is deterministic (fixed byte order: little-endian by default, configured globally). Size is minimal (~10MB per 100K creatures with genomes). Speed is acceptable for save/load (not in hot path). For manifests and mods, we use **serde + JSON** for readability and schema validation (jsonschema crate).

**Serialization Strategy**:
- **Sim State** (entities, channels, PRNG state): bincode (binary, deterministic, compact)
- **Manifests** (channels, primitives, biomes): JSON (human-editable, schema-validated)
- **Save File** (wrapper around sim state): bincode + metadata header (version, timestamp, checksum)

### 3.5 Manifest Validation: JSON Schema

**Choice**: **jsonschema** for validation, **schemars** for generation (future)

| Criterion | jsonschema | schemars | ajv (JS) | hand-rolled |
|-----------|-----------|----------|----------|------------|
| **Validation** | Perfect | Excellent | Excellent | Risky |
| **Generation** | No | Yes | N/A | N/A |
| **Performance** | Moderate | Moderate | Fast | Unknown |
| **Rust Integration** | Direct | Proc-macros | Via WASM | Manual |
| **Schema Format** | Standard JSON Schema | JSON Schema | JSON Schema | Custom |

**Justification**: jsonschema crate validates channel/primitive/biome manifests at load time. Schemas are defined in JSON Schema format (.schema.json files), human-readable, and validated against the spec. schemars will be used later to generate schemas from Rust types for developer convenience, but initially we hand-author schemas.

**Load-Time Validation**:
```rust
// In beast-mod or beast-channels
let manifest_json = std::fs::read_to_string("channels/auditory_sensitivity.json")?;
let manifest_value: serde_json::Value = serde_json::from_str(&manifest_json)?;
let schema = jsonschema::JSONSchema::compile(&CHANNEL_SCHEMA)?;
schema.validate(&manifest_value)?;  // Fails with rich error messages if invalid
let manifest: ChannelManifest = serde_json::from_value(manifest_value)?;
```

### 3.5B JSON Schema Draft 2020-12 Specification

**Manifest Validation Standard**: All manifests (channel, primitive, biome, label) are validated against JSON Schema **draft 2020-12** using the `jsonschema` Rust crate. Validation occurs at **module load time** (startup), not at runtime.

**Schema Files Location**:
```
assets/schemas/
├── channel_manifest.schema.json
├── primitive_manifest.schema.json
├── biome_manifest.schema.json
├── label_manifest.schema.json
└── savefile.schema.json
```

**Validation Pipeline**:

```rust
pub struct ManifestValidator;

impl ManifestValidator {
    /// Load and compile schema (draft 2020-12)
    pub fn load_schema(schema_path: &Path) -> Result<jsonschema::JSONSchema> {
        let schema_str = std::fs::read_to_string(schema_path)?;
        let schema_value: serde_json::Value = serde_json::from_str(&schema_str)?;
        jsonschema::JSONSchema::compile(&schema_value)
            .map_err(|e| anyhow!("Schema compilation failed: {}", e))
    }
    
    /// Validate manifest JSON against schema
    pub fn validate_manifest(
        manifest_json: &serde_json::Value,
        schema: &jsonschema::JSONSchema,
    ) -> Result<()> {
        schema.validate(manifest_json)
            .map_err(|e| anyhow!("Manifest validation failed: {}", e))
    }
}

pub fn load_channel_manifest(path: &Path) -> Result<ChannelManifest> {
    // 1. Load JSON
    let json_str = std::fs::read_to_string(path)?;
    let json_value: serde_json::Value = serde_json::from_str(&json_str)?;
    
    // 2. Validate against schema (draft 2020-12)
    let schema_path = Path::new("assets/schemas/channel_manifest.schema.json");
    let schema = ManifestValidator::load_schema(schema_path)?;
    ManifestValidator::validate_manifest(&json_value, &schema)?;
    
    // 3. Deserialize to Rust struct
    let manifest: ChannelManifest = serde_json::from_value(json_value)?;
    Ok(manifest)
}
```

**CI Test Requirement**: On every build, validate all core manifests:

```rust
#[test]
fn test_validate_all_core_manifests() {
    let core_manifest_dirs = [
        "assets/manifests/channels",
        "assets/manifests/primitives",
        "assets/manifests/biomes",
        "assets/manifests/labels",
    ];
    
    for dir in &core_manifest_dirs {
        for entry in std::fs::read_dir(dir).expect("Cannot read dir") {
            let path = entry.expect("Cannot read entry").path();
            if path.extension().map_or(false, |ext| ext == "json") {
                // Load and validate
                let json_str = std::fs::read_to_string(&path)
                    .expect(&format!("Cannot read {}", path.display()));
                let json_value: serde_json::Value = serde_json::from_str(&json_str)
                    .expect(&format!("Invalid JSON: {}", path.display()));
                
                // Determine which schema to use
                let schema_file = if dir.contains("channels") {
                    "channel_manifest.schema.json"
                } else if dir.contains("primitives") {
                    "primitive_manifest.schema.json"
                } else if dir.contains("biomes") {
                    "biome_manifest.schema.json"
                } else {
                    "label_manifest.schema.json"
                };
                
                let schema = ManifestValidator::load_schema(
                    &Path::new("assets/schemas").join(schema_file)
                ).expect(&format!("Cannot load schema: {}", schema_file));
                
                ManifestValidator::validate_manifest(&json_value, &schema)
                    .expect(&format!("Validation failed: {}", path.display()));
            }
        }
    }
}
```

**Schema Format Example** (Channel Manifest, JSON Schema draft 2020-12):

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Channel Manifest",
  "type": "object",
  "properties": {
    "id": {"type": "string"},
    "name": {"type": "string"},
    "family": {
      "enum": ["sensory", "motor", "metabolic", "regulatory", "developmental"]
    },
    "allometric_scaling": {
      "type": "object",
      "properties": {
        "exponent": {"type": "number", "minimum": -2, "maximum": 2}
      }
    },
    "scale_band": {
      "type": "array",
      "minItems": 2,
      "maxItems": 2,
      "items": {"type": "number"}
    },
    "expression_conditions": {"$ref": "#/$defs/ExpressionConditions"}
  },
  "required": ["id", "name", "family"],
  "$defs": {
    "ExpressionConditions": {
      "type": "object",
      "properties": {
        "biome_tags": {"type": "array", "items": {"type": "string"}},
        "seasonal_gates": {"type": "array", "items": {"enum": ["spring", "summer", "fall", "winter"]}},
        "developmental_stage": {"enum": ["juvenile", "adult", "elderly"]}
      }
    }
  }
}
```

**Key Points**:
- All manifests are validated at **module load time** (startup), not runtime
- Validation is deterministic and has no side effects
- Invalid manifests cause immediate startup failure with rich error messages
- CI enforces validation of all core manifests before release
- Mods are also validated on load (see Section 8)

### 3.6 Graphics: SDL3 C Bindings

**Choice**: **sdl3** (rust-sdl3 / sdl3-sys)

| Criterion | SDL3 | wgpu | macroquad | raylib-rs |
|-----------|------|------|-----------|-----------|
| **API Style** | Imperative (immediate) | GPU-first | Simple 2D | Simple 2D |
| **Cross-Platform** | Windows, macOS, Linux, Web | All + GPU compute | All | All |
| **Learning Curve** | Moderate | Steep | Low | Low |
| **Graphics Abstraction** | Low (CPU-side) | High (GPU) | Medium | Medium |
| **Audio Support** | Yes (built-in) | No (separate) | Yes | Yes |
| **Retained-Mode UI** | Manual (we write it) | Possible | Hard | Hard |

**Justification**: SDL3 is the latest stable release with improved APIs over SDL2. We implement a custom retained-mode widget layer on top of SDL3's render API. SDL3's simplicity means we own the rendering pipeline; no hidden abstractions. Cross-platform support (Windows, macOS, Linux) is excellent. Audio is built-in.

**Alternative Rejected**: wgpu would give us GPU compute access (useful for visual effects in deep-system work, but not MVP). macroquad is too opinionated for complex UI. raylib-rs is simpler but lacks retained-mode support and audio.

### 3.7 UI Framework: Retained-Mode on SDL3

**Choice**: Custom retained-mode widget tree (~2000 LOC)

| Criterion | Custom | imgui-rs | egui | fltk-rs |
|-----------|--------|----------|------|---------|
| **Widget Hierarchy** | Full control | Tree support | Immediate-mode | Full |
| **Persistence** | Full state serialization | Manual | Session ephemeral | Full |
| **Binding to Data** | Direct references | Manual | Through closures | Direct |
| **Performance** | Optimized (we control it) | Good | Moderate | Good |
| **Styling** | CSS-like (design-system ready) | ImGui-style | CSS-like | Native |
| **Learning Curve** | Medium | Low | Very low | Low |

**Justification**: System 23 (UI Overview) specifies a retained-mode design. Custom implementation gives us:
- Full control over state persistence (UI state lives in a separate file, non-canonical)
- Direct data binding to sim state (Chronicler query results render directly)
- CSS-like styling system for design-system coherence
- Tight integration with SDL3 rendering

A retained-mode widget tree is ~2000 lines of Rust (widget trait, event dispatch, layout engine, render batching). We build this once, then compose complex screens from primitives.

**Widget Primitives**:
```rust
pub trait Widget: Send + Sync {
    fn id(&self) -> WidgetID;
    fn layout(&mut self, available: Rect);
    fn on_event(&mut self, event: &Event) -> EventResult;
    fn render(&self, renderer: &mut Renderer);
    fn is_dirty(&self) -> bool;
}

pub struct Button { id: WidgetID, label: String, on_pressed: Callable, ... }
pub struct List { id: WidgetID, items: Vec<ListItem>, selected: usize, ... }
pub struct Card { id: WidgetID, title: String, content: Vec<Box<dyn Widget>>, ... }
```

### 3.8 Spatial Queries: R*-Tree

**Choice**: **rstar** (R*-tree spatial index)

| Criterion | rstar | kdtree | broccoli | hand-rolled |
|-----------|-------|--------|----------|------------|
| **Query Type** | Range, nearest-neighbor | NN, range | NN, range | Configurable |
| **Dynamic Updates** | Yes (rebuild friendly) | Moderate | Yes | DIY |
| **Performance** | Excellent (O(log N)) | Moderate | Excellent | Unknown |
| **Dimensionality** | 2D, 3D, N-D | N-D | 2D only | DIY |
| **Ease of Use** | Simple API | Simple | Simple | Complex |

**Justification**: World-map locations, creature positions, and biome cells use spatial queries (find creatures near player, find biome cells within region, etc.). rstar provides O(log N) range and nearest-neighbor queries. The API is simple; tree rebuilds are fast and happen offline (not hot-path).

**Usage**:
```rust
let mut spatial_index: RTree<CreatureLocation> = RTree::new();
spatial_index.insert(CreatureLocation { pos: Vec2 { x: 100.0, y: 200.0 }, id: creature_id });

// Range query: find creatures within 500 units
let nearby = spatial_index.locate_in_envelope_intersecting(&Envelope::new(
    Vec2 { x: 50.0, y: 150.0 },
    Vec2 { x: 550.0, y: 650.0 },
)).collect::<Vec<_>>();
```

### 3.9 Parallelism: Data-Parallel Sim

**Choice**: **rayon** (data parallelism)

| Criterion | rayon | tokio | crossbeam | threadpool |
|-----------|-------|-------|-----------|-----------|
| **Model** | Data-parallel (fork-join) | Async/await (I/O) | Fine-grained sync | Thread pool |
| **Use Case** | Embarrassingly parallel | Long-running async | Shared-memory sync | Simple parallelism |
| **Determinism** | Perfect (isolated scopes) | Perfect (single-threaded) | Good (careful locking) | Good |
| **Complexity** | Low (iterator traits) | Moderate (async) | Moderate (locks) | Low |

**Justification**: Evolution fitness evaluation and Chronicler pattern-matching are embarrassingly parallel. rayon's iterator API (`par_iter()`) makes it trivial to parallelize population evaluation without explicit thread management. For determinism, we isolate each worker thread's PRNG stream, ensuring no cross-contamination.

**Parallel Evolution**:
```rust
// Evaluate fitness in parallel
let fitnesses: Vec<f32> = population
    .par_iter()
    .map(|creature| {
        let mut local_rng = ecology_rng.clone();  // Each thread gets its own clone
        fitness_evaluator.evaluate(creature, &mut local_rng)
    })
    .collect();
```

### 3.10 Testing & Benchmarking

**Choice**: **criterion** (benchmarks), **insta** (snapshot tests), **proptest** (property tests)

| Tool | Purpose | Alternative | Tradeoff |
|------|---------|-------------|----------|
| **criterion** | Micro-benchmarks (interpreter, mutator) | built-in `#[bench]` | Rich statistics vs. setup overhead |
| **insta** | Snapshot tests (replay state, manifests) | assert_eq! | Reviewable diffs vs. manual assertions |
| **proptest** | Property-based tests (invariants) | quickcheck | Better shrinking vs. syntax |

**Determinism Test Suite**:
```rust
#[test]
fn test_deterministic_replay_1000_ticks() {
    let save_0 = load_save_file("determinism_fixture.json");
    let inputs = load_input_journal("determinism_fixture_inputs.json");
    
    // Run forward and capture snapshots
    let hashes_original = (0..1000)
        .map(|tick| {
            run_tick(&save_0, tick, inputs[tick].clone());
            hash_sim_state(&save_0)
        })
        .collect::<Vec<_>>();
    
    // Reload and replay
    let save_1 = load_save_file("determinism_fixture.json");
    let hashes_replay = (0..1000)
        .map(|tick| {
            run_tick(&save_1, tick, inputs[tick].clone());
            hash_sim_state(&save_1)
        })
        .collect::<Vec<_>>();
    
    assert_eq!(hashes_original, hashes_replay);
}
```

---

## 4. ECS Architecture

### 4.1 Entity Kinds & Component Mapping

| Entity Kind | Components | Systems (Read/Write) |
|-------------|-----------|----------------------|
| **Creature** | Genome, Phenotype, HealthState, Position, GenerationData | Evolution, Interpreter, Combat, Metabolism, Reproduction, Ecology, Renderer |
| **Pathogen** | Genome (micro-scale), HostCoupling, TransmissionState, HostEntity | Disease, Interpreter, Ecology, Metabolism |
| **Agent (NPC)** | Personality, Inventory, Faction, KnowledgeState, DialogueHistory | Cognition, Social, Migration, Serialization |
| **Faction** | Members, Resources, Relationships, Technologies, Culture | Social, Economic, Technology, Chronicler |
| **Biome** | Resources, Climate, Terrain, PopulationDensity, EventLog | Ecology, Climate, Renderer |
| **Settlement** | Faction, PopulationCount, Buildings, TechLevel, Materials | Economic, Social, Serialization |
| **Event** | Type, Timestamp, Location, Agents, Description | Chronicler (reader only) |

### 4.2 Component Definitions (specs)

```rust
// Core creature state
#[derive(Clone)]
pub struct Genome {
    pub genes: Vec<TraitGene>,
    pub channel_values: Vec<Q3232>,  // Fixed-point Q32.32
    pub generation: u32,
    pub lineage_tag: PhyloID,
}

#[derive(Clone)]
pub struct Phenotype {
    pub active_channels: Vec<ChannelID>,
    pub primitive_effects: HashSet<PrimitiveEffectID>,
    pub body_map: Vec<BodyRegion>,
    pub age_stage: DevelopmentalStage,
    pub last_interpreter_tick: u64,
}

#[derive(Clone)]
pub struct HealthState {
    pub health: Q3232,                // [0, 1] fixed-point
    pub max_health: Q3232,
    pub energy: Q3232,
    pub injuries: Vec<Injury>,
}

pub struct Position {
    pub x: i32,  // World grid
    pub y: i32,
    pub z: i32,  // Elevation
}

pub struct Creature;  // Marker component

// Pathogen-specific
#[derive(Clone)]
pub struct HostCoupling {
    pub host_entity: EntityID,
    pub energetic_drain: Q3232,
    pub transmission_efficiency: Q3232,
    pub virulence: Q3232,
}

pub struct Pathogen;  // Marker component

// NPC
#[derive(Clone)]
pub struct Agent {
    pub name: String,
    pub faction: FactionID,
    pub personality: PersonalityProfile,
}

pub struct Inventory {
    pub items: Vec<ItemStack>,
    pub capacity: usize,
}

// Shared by creatures and agents
pub struct EntityID(pub u32);
```

### 4.3 System Schedule (Ordered Stages)

The tick loop executes in this order to maintain determinism and data consistency:

```rust
pub enum SystemStage {
    // Stage 0: Input & Aging
    Input,              // Player actions, random events
    Aging,              // Increment creature age, check developmental stage transitions
    
    // Stage 1: Genetics
    Mutation,           // Apply mutations (evolution subsystem)
    ChannelGenesis,     // Rare channel duplication/divergence events
    
    // Stage 2: Phenotype Resolution
    InterpreterStage,   // Evaluate channels → primitive effects
    ScaleBandFilter,    // Gate channels by scale, biome, season, etc.
    CompositionHooks,   // Evaluate composition_hooks, emit primitives
    
    // Stage 3: Physics & Movement
    PhysicsUpdate,      // Apply forces, update position
    MovementResolve,    // Pathfinding, collision detection
    
    // Stage 4: Interaction & Combat
    CombatResolution,   // Combat vs other creatures/NPCs
    PreyPredator,       // Predation, herbivory
    Parasitism,         // Pathogen transmission, host damage
    
    // Stage 5: Physiology
    Metabolism,         // Energy consumption, temperature regulation
    ReproductionCycle,  // Fertility, birth, death
    
    // Stage 6: Ecology
    PopulationDynamics, // Migration, extinction, speciation
    BiomeEffects,       // Environmental hazards, resource regrowth
    
    // Stage 7: Labeling & UI
    ChroniclerPattern,  // Pattern detection, label assignment (sampled)
    
    // Stage 8: Rendering
    RenderPrep,         // Copy immutable state for frame rendering
}
```

**Determinism Guarantee**: Systems within a stage run in parallel (via rayon). Systems in different stages run sequentially. Within each stage, entity iteration is sorted by EntityID. No shared mutable state between stage iterations.

### 4.4 Resources (Global Mutable State)

```rust
pub struct Resources {
    pub channel_registry: ChannelRegistry,
    pub primitive_registry: PrimitiveRegistry,
    pub biome_registry: BiomeRegistry,
    pub faction_registry: FactionRegistry,
    
    // PRNG streams
    pub rng_evolution: Xoshiro256PlusPlus,
    pub rng_ecology: Xoshiro256PlusPlus,
    pub rng_combat: Xoshiro256PlusPlus,
    pub rng_disease: Xoshiro256PlusPlus,
    
    // Global state
    pub tick_counter: u64,
    pub sorted_entity_index: SortedEntityIndex,  // Handles deterministic iteration
    pub chronicler: Chronicler,
    pub world_map: SpatialIndex<BiomeCell>,
    
    // Performance budgets
    pub tick_budget_ms: u32,
    pub stage_budgets: HashMap<SystemStage, u32>,
}
```

### 4.5 System Implementation Pattern

All systems follow this trait:

```rust
pub trait System {
    fn run(&mut self, world: &World, resources: &mut Resources) -> Result<()>;
    fn stage(&self) -> SystemStage { SystemStage::Input }
    fn name(&self) -> &str;
}

// Example: MutationSystem
pub struct MutationSystem;

impl System for MutationSystem {
    fn run(&mut self, world: &World, resources: &mut Resources) -> Result<()> {
        let genome_storage = world.read_storage::<Genome>();
        let mut phenotype_storage = world.write_storage::<Phenotype>();
        
        // Iterate in sorted order
        let entities = resources.sorted_entity_index.creatures();
        for entity_id in entities {
            if let Ok(mut phenotype) = phenotype_storage.get_mut(entity_id) {
                let genome = genome_storage.get(entity_id)?;
                
                // Mutate with subsystem PRNG
                let delta = sample_gaussian(&mut resources.rng_evolution, 0.0, 0.1);
                for channel_val in &mut genome.channel_values {
                    *channel_val = (*channel_val + delta).clamp(Q3232::ZERO, Q3232::ONE);
                }
            }
        }
        Ok(())
    }
    
    fn stage(&self) -> SystemStage { SystemStage::Mutation }
    fn name(&self) -> &str { "MutationSystem" }
}
```

---

## 5. Data Flow: Simulation Tick

One tick executes as follows:

```
┌─────────────────────────────────────────────────────────────┐
│ Tick N: Input → Genetics → Phenotype → Physics → Combat ... │
└─────────────────────────────────────────────────────────────┘

[Input Stage]
  - Player issues avatar movement or action command
  - Random events (migration waves, plague outbreaks)
  - RNG: resources.rng_input

[Mutation Stage]
  - For each creature: apply point mutation, regulatory rewiring, duplication, etc.
  - All math: fixed-point Q32.32 (channel values in [0,1])
  - RNG: resources.rng_evolution

[Interpreter Stage]
  - For each creature:
    1. Read genome.channel_values (Q32.32)
    2. Evaluate expression_conditions (biome, season, scale_band, developmental stage)
    3. Evaluate composition_hooks (threshold, gating, additive, multiplicative)
    4. For each triggered hook: emit primitives via "emits" list
    5. Parameter mapping: expr.parameter_mapping strings evaluated against channel values
  - Output: creature.phenotype.primitive_effects set
  - All fixed-point; deterministic

[Scale-Band Filter Stage]
  - For pathogens: filter channels by micro scale_band
  - For macro creatures: filter channels by macro scale_band
  - Dormant channels are gated out before composition

[Physics Stage]
  - For each creature: apply primitive effects (force_application) to position
  - Update velocity, handle collisions
  - RNG: resources.rng_combat (for stochastic collision outcomes)

[Combat Stage]
  - For each creature pair in engagement: resolve combat via primitive effects
  - force_application primitives determine damage
  - signal_emission + reception define detection range
  - RNG: resources.rng_combat

[Metabolism Stage]
  - For each creature: consume energy based on energy_modulation primitives
  - Handle temperature regulation (state_induction primitives)
  - Check starvation, disease progress (via HostCoupling energy drain)

[Reproduction Stage]
  - For each creature with high health & energy: spawn offspring
  - Offspring inherit mutated genome
  - RNG: resources.rng_evolution (for mutation)

[Ecology Stage]
  - Population dynamics: local carrying capacity, extinction risk
  - Migration: source-sink metapopulation
  - Biome resource regrowth, climate shifts

[Chronicler Stage]
  - Sample: every N ticks (e.g., every 100 ticks), run pattern detection
  - Pattern recognition: cluster primitive_effects by signature
  - Label assignment: "echolocation" ← (emit_acoustic_pulse + receive_acoustic_signal + spatial_integrate)
  - RNG: No randomness (deterministic clustering)

[Render Prep Stage]
  - Copy immutable snapshot of creature positions, phenotypes, health to render queue
  - Do NOT mutate sim state during render prep

[End of Tick]
  - Increment tick_counter
  - Serialize PRNG state, entity state to memory
  - (Optional) Save to disk if checkpoint tick

Total duration: <16ms at 60 FPS
```

### 5.1 Fixed-Point Math in Hot Paths

**Mandatory Q32.32 Use**:
- Channel values (genome.channel_values, Phenotype.active_channels)
- Fitness scores
- Health/energy values
- Composition hook results (weighted sums, products)
- Parameter mapping results

**Float OK**:
- Rendering coordinates (UI, sprites) — converted at render-time from fixed-point
- Camera zoom levels
- Mouse coordinates (UI input)
- Procedural generation (noise, randomization) — seeds are deterministic, outputs clipped to [0, 1] as Q32.32

### 5.2 PRNG Usage: Per-Subsystem Streams

```
One world seed → SplitMix64 expand → N independent xoshiro256** streams

resources.rng_evolution    → point mutations, gene duplication
resources.rng_ecology      → population drift, migration rates
resources.rng_combat       → damage variance, hit probability
resources.rng_disease      → transmission success, virulence stochasticity
resources.rng_social       → faction relationship drift, NPC personality variation
resources.rng_chronicler   → label assignment (deterministic seeding, no post-hoc randomness)

// Each stream is seeded once at world creation
// Stream state is serialized in save files
// Replay: load exact stream state, execute identical random calls, get identical outcomes
```

### 5.3 PRNGState Struct: Serialization & Contamination Guards

The PRNG state must be **atomic and serializable** to enable deterministic replay. All subsystem streams must be wrapped in newtype wrappers to prevent cross-contamination at the type level:

```rust
/// Atomic PRNG state snapshot; serialized in SaveFile
#[derive(Clone, Serialize, Deserialize)]
pub struct PRNGState {
    pub evolution_stream: EvolutionRng,
    pub ecology_stream: EcologyRng,
    pub combat_stream: CombatRng,
    pub disease_stream: DiseaseRng,
    pub chronicler_stream: ChroniclerRng,
    pub worldgen_stream: WorldgenRng,
}

/// Newtype wrapper preventing cross-use (type system enforces isolation)
#[derive(Clone, Serialize, Deserialize)]
pub struct EvolutionRng(pub Xoshiro256PlusPlus);

#[derive(Clone, Serialize, Deserialize)]
pub struct EcologyRng(pub Xoshiro256PlusPlus);

// ... similar for Combat, Disease, Chronicler, Worldgen

impl PRNGState {
    /// Serialize all streams atomically for save file
    pub fn serialize_atomic(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }
    
    /// Deserialize all streams atomically from save file
    pub fn deserialize_atomic(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(Into::into)
    }
}
```

**Serialization in SaveFile**: The `PRNGState` struct is serialized in `SaveFile.rng_state` via bincode. All subsystem streams are packed together, ensuring atomic write/read:

```rust
pub struct SaveFile {
    format_version: String,
    game_version: String,
    current_tick: u64,
    world_seed: u64,
    rng_state: Vec<u8>,  // PRNGState serialized as bincode
    entities: EcsWorldSnapshot,
    // ... other fields
}
```

**Replay-Validation Test Requirement**: When running replay validation, the state hash **must include** the PRNGState. This ensures the PRNG is never drifted or corrupted:

```rust
#[test]
fn test_replay_includes_prng_state() {
    let original = hash_sim_state(&sim_original);
    // Hash includes sim.resources.prng_state.serialize_atomic()
    
    let replayed = hash_sim_state(&sim_replay);
    assert_eq!(original, replayed, "PRNG state diverged during replay");
}
```

**Type-Level Contamination Guards**: By wrapping each stream in a newtype (EvolutionRng, CombatRng, etc.), the type system prevents code from accidentally using the wrong stream:

```rust
// Forbidden by compiler:
let delta = sample_gaussian(&mut rng_combat, 0.0, 0.1);  // Error: rng_combat is CombatRng
                                                            // Expected: EvolutionRng

// Correct:
let delta = sample_gaussian(&mut rng_evolution.0, 0.0, 0.1);  // OK: unwrapped
```

**Parallel Systems**: When a system runs in parallel threads (via rayon), each thread gets a cloned stream via `rng.jump()` to advance by 2^128 steps, ensuring independent sequences across threads:

```rust
let rng_evolution = resources.rng_evolution.0.clone();
rayon::scope(|s| {
    for chunk in creatures.chunks(CHUNK_SIZE) {
        let mut thread_rng = rng_evolution.clone();
        thread_rng.jump();  // Advance by 2^128
        s.spawn(|_| {
            // Process chunk with thread_rng
        });
    }
});
```

---

## 6. Determinism Guards

### 6.1 Sorted Iteration Helper

```rust
pub struct SortedEntityIndex {
    creatures: Vec<EntityID>,
    agents: Vec<EntityID>,
    pathogens: Vec<EntityID>,
    // ... other entity types
}

impl SortedEntityIndex {
    pub fn creatures(&self) -> &[EntityID] {
        // Returns creatures in ascending EntityID order
        &self.creatures
    }
    
    pub fn update(&mut self, world: &World) {
        // After entity spawning/despawning, rebuild sorted lists
        self.creatures = world.entities().join()
            .filter(|e| world.read_storage::<Creature>().get(*e).is_ok())
            .collect::<Vec<_>>();
        self.creatures.sort_by_key(|e| e.0);
    }
}

// System usage:
let sorted_ids = resources.sorted_entity_index.creatures();
for entity_id in sorted_ids {
    // Iteration is deterministic
}
```

### 6.2 Fixed-Point Wrapper

```rust
use fixed::types::I32F32;  // Q32.32 alias

pub type Q3232 = I32F32;

impl Q3232 {
    pub const ZERO: Self = I32F32::ZERO;
    pub const ONE: Self = I32F32::from_bits(1u64 << 32);
    pub const HALF: Self = I32F32::from_bits(1u64 << 31);
    
    pub fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)  // Built-in saturating arithmetic
    }
    
    pub fn saturating_mul(self, rhs: Self) -> Self {
        (self.wide_mul(rhs) >> 32).saturating_as::<Q3232>()
    }
    
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }
}

// Usage in mutation:
let sigma = Q3232::from_num(0.1);
let delta = sample_gaussian(&mut rng) * sigma;  // Both fixed-point
let new_value = (current + delta).clamp(Q3232::ZERO, Q3232::ONE);
```

### 6.3 Replay-Validation Harness (CI Test)

```rust
// tests/determinism_test.rs

#[test]
fn test_replay_validation_1000_ticks() {
    // Load fixture: initial state + inputs
    let save_file = include_str!("fixtures/determinism_test.json");
    let input_journal = include_str!("fixtures/determinism_test_inputs.json");
    
    let mut sim_original = Simulation::from_save_json(save_file)?;
    let mut sim_replay = Simulation::from_save_json(save_file)?;
    
    let inputs: Vec<Input> = serde_json::from_str(input_journal)?;
    
    // Forward pass: capture state hashes
    let mut hashes_original = Vec::new();
    for (tick_num, input) in inputs.iter().enumerate() {
        sim_original.tick(input)?;
        let hash = hash_sim_state(&sim_original);
        hashes_original.push((tick_num, hash));
    }
    
    // Replay pass: verify hashes match exactly
    let mut hashes_replay = Vec::new();
    for (tick_num, input) in inputs.iter().enumerate() {
        sim_replay.tick(input)?;
        let hash = hash_sim_state(&sim_replay);
        hashes_replay.push((tick_num, hash));
    }
    
    // Assert bit-identical
    assert_eq!(hashes_original.len(), hashes_replay.len());
    for (i, ((tick_o, hash_o), (tick_r, hash_r))) in
        hashes_original.iter().zip(hashes_replay.iter()).enumerate()
    {
        assert_eq!(
            hash_o, hash_r,
            "Divergence at tick {}: {} != {}",
            tick_o, hash_o, hash_r
        );
    }
    
    println!("✓ Determinism validated over {} ticks", inputs.len());
}

fn hash_sim_state(sim: &Simulation) -> u64 {
    // Hash all entity component arrays + PRNG state
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    
    // Hash PRNG states (serialize to bytes, hash)
    for rng_stream in &[&sim.resources.rng_evolution, ...] {
        let bytes = rng_stream.to_bytes();
        hasher.write(&bytes);
    }
    
    // Hash all entities in sorted order
    for entity_id in sim.resources.sorted_entity_index.all_entities() {
        if let Ok(genome) = sim.world.read_storage::<Genome>().get(entity_id) {
            hasher.write(&genome.serialize_bytes()?);
        }
        // ... hash other components
    }
    
    use std::hash::Hasher;
    hasher.finish()
}
```

**CI Job**: Run test on Linux, macOS, Windows with stable Rust. If determinism test fails, print first diverging entity/component and tick number.

### 6.4 Banned APIs (Enforced via Clippy + CI Script)

**In `beast-sim` and `beast-ecs` crates only** (forbidden in hot-path crates):

```toml
# In Cargo.toml of hot-path crates
[lints.clippy]
# Deny HashMap iteration (unordered)
illegal-floating-point-literal-in-const = "deny"
missing-const-doc = "deny"

# Custom lint via #![forbid(unsafe_code)] in deterministic modules
```

**Banned Constructs** (enforced via grep in CI):
```
PATTERN              REASON                      REPLACEMENT
HashMap iteration    Unordered                   BTreeMap or Vec + sort
.iter_unordered()    Unordered (specs)          .iter_sorted()
std::time::*         Wall-clock dependent       tick_counter
f32/f64 in sim       Not bit-identical          Q32.32 fixed-point
thread::sleep()      Timing dependent           RNG delay via tick counter
rand::thread_rng()   Unseeded PRNG              Use subsystem stream
std::process::*      Subprocess variability     CI-only
```

**CI Script** (`check_determinism_invariants.sh`):
```bash
#!/bin/bash
set -e

# Forbidden patterns in simulation crates
forbidden_patterns=(
    "std::time::" 
    "thread::sleep"
    "rand::thread_rng"
    "HashMap.*\.iter()"
    "\.0\.0\.iter()"  # unordered iteration
)

for crate in beast-sim beast-ecs beast-interpreter beast-evolution beast-disease; do
    for pattern in "${forbidden_patterns[@]}"; do
        if grep -r "$pattern" "$crate/src" --include="*.rs"; then
            echo "FAIL: Found forbidden pattern '$pattern' in $crate"
            exit 1
        fi
    done
done

echo "✓ All determinism invariants passed"
```

---

## 7. Rendering Architecture

### 7.1 Two-Mode Renderer: World Map + Encounter View

```rust
pub enum RenderMode {
    WorldMap {
        viewport: WorldRect,
        zoom: f32,
    },
    EncounterView {
        center: Vec3,
        camera_angle: f32,  // Fixed isometric angle
    },
}

pub struct Renderer {
    sdl_context: sdl3::Sdl,
    canvas: sdl3::render::Canvas,
    sprite_atlas: SpriteAtlas,
    ui_widgets: WidgetTree,
    render_mode: RenderMode,
    frame_budget_ms: u32,
}

impl Renderer {
    pub fn render_frame(
        &mut self,
        sim_snapshot: &SimulationSnapshot,
        ui_state: &UIState,
    ) -> Result<()> {
        let start = std::time::Instant::now();
        
        match self.render_mode {
            RenderMode::WorldMap { viewport, zoom } => {
                self.render_world_map(sim_snapshot, viewport, zoom)?;
            }
            RenderMode::EncounterView { center, camera_angle } => {
                self.render_encounter(sim_snapshot, center, camera_angle)?;
            }
        }
        
        // Render UI on top
        self.ui_widgets.render(&mut self.canvas, ui_state)?;
        
        self.canvas.present();
        
        let elapsed = start.elapsed().as_millis() as u32;
        if elapsed > self.frame_budget_ms {
            warn!("Render frame exceeded budget: {}ms > {}ms", elapsed, self.frame_budget_ms);
        }
        
        Ok(())
    }
}
```

### 7.2 Render Snapshot: Double-Buffering Semantics

The renderer reads a **snapshot** of sim state via **double-buffering**. The sim owns two snapshots (front/back); the renderer reads front while the sim writes back. At the frame boundary, they swap.

```rust
pub struct SimulationSnapshot {
    pub entities: Vec<EntitySnapshot>,
    pub biome_cells: Vec<BiomeCellSnapshot>,
    pub timestamp: u64,
    pub chronicler_labels: HashMap<EntityID, Vec<Label>>,  // Fresh from Chronicler
}

pub struct EntitySnapshot {
    pub id: EntityID,
    pub kind: EntityKind,  // Creature, Agent, Pathogen
    pub position: Vec3,
    pub health: f32,  // Converted from Q32.32 to float for rendering
    pub phenotype: PhenotypeSnapshot,
    pub visual_directive: VisualDirective,
}

pub struct SimulationDoubleBuffer {
    front: SimulationSnapshot,  // Renderer reads this
    back: SimulationSnapshot,   // Simulation writes this
}

impl SimulationDoubleBuffer {
    /// Called at tick boundary AFTER Chronicler (stage 7)
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.front, &mut self.back);
        // Now front has fresh snapshot with current Chronicler labels
    }
    
    /// Renderer calls this to read front snapshot
    pub fn read_front(&self) -> &SimulationSnapshot {
        &self.front
    }
    
    /// Simulation calls this to write back snapshot
    pub fn write_back_mut(&mut self) -> &mut SimulationSnapshot {
        &mut self.back
    }
}

pub struct VisualDirective {
    pub mesh_id: MeshID,      // From system 10 (procedural visuals)
    pub color_override: Option<Color>,
    pub animation_state: AnimationState,
    pub particle_effects: Vec<ParticleEffect>,
}

// In game loop:
pub fn game_loop(&mut self) {
    loop {
        // 1. Simulate one tick (Stage 0–7)
        self.simulation.tick(&input)?;
        
        // 2. Chronicler runs at stage 7, labels are fresh
        
        // 3. RenderPrep at stage 8: snapshot taken AFTER Chronicler
        //    Writes to back buffer
        self.simulation.create_snapshot_to_back_buffer();
        
        // 4. Swap buffers: front now has fresh snapshot + labels
        self.sim_double_buffer.swap();
        
        // 5. Render with front snapshot (non-blocking, renderer cannot mutate sim)
        let snapshot = self.sim_double_buffer.read_front();
        self.renderer.render_frame(snapshot)?;
        
        // 6. Handle input for next tick
        input = self.input_handler.poll()?;
    }
}
```

**Timing Guarantee**: 
- Chronicler runs at Stage 7 (labels are fresh for this tick)
- RenderPrep runs at Stage 8 (snapshot creation happens AFTER Chronicler)
- Swap and render happen between tick N and tick N+1

**Frame Drop Policy**: If render takes >16ms (exceeds frame budget):
- Sim **continues ticking** at fixed-step rate (does not wait for render)
- Render frame is **dropped** (renderer catches up on next frame)
- No frame queueing; only the most recent snapshot is displayed
- This maintains determinism (sim runs independently of render timing)

### 7.3 Sprite/Mesh Pipeline (System 10 Integration)

System 10 (Procedural Visual Pipeline) generates visual directives from channel phenotypes:

```rust
pub struct VisualPipelineSystem;

impl System for VisualPipelineSystem {
    fn run(&mut self, world: &World, resources: &mut Resources) -> Result<()> {
        let phenotype_storage = world.read_storage::<Phenotype>();
        let mut visual_directive_storage = world.write_storage::<VisualDirective>();
        
        let renderer = &resources.renderer;
        
        for (entity_id, phenotype) in phenotype_storage.join() {
            // Map channels → visual parameters
            let color = self.infer_color_from_channels(&phenotype)?;
            let mesh_id = self.infer_mesh_from_body_sites(&phenotype)?;
            let particles = self.infer_particles_from_primitives(&phenotype)?;
            
            let directive = VisualDirective {
                mesh_id,
                color_override: Some(color),
                animation_state: AnimationState::Idle,
                particle_effects: particles,
            };
            
            visual_directive_storage.insert(entity_id, directive)?;
        }
        
        Ok(())
    }
    
    fn stage(&self) -> SystemStage { SystemStage::RenderPrep }
}
```

---

## 8. Mod System

### 8.1 Mod Loading & Manifest Merging

Mods are directories with manifests (JSON) defining new channels, primitives, and biomes:

```
my_mod/
  mod.json
  channels/
    new_channel_1.json
    new_channel_2.json
  primitives/
    new_primitive.json
  biomes/
    new_biome.json
```

**mod.json**:
```json
{
  "id": "my_mod_id",
  "version": "1.0.0",
  "name": "Exotic Species Pack",
  "description": "Adds 5 new channels and 2 new primitive effects",
  "author": "Modder Name",
  "load_order": 50,
  "dependencies": []
}
```

### 8.2 Manifest Validation at Load Time

```rust
pub struct ModLoader;

impl ModLoader {
    pub fn load_mod(mod_dir: &Path) -> Result<LoadedMod> {
        // 1. Parse mod.json
        let mod_json_path = mod_dir.join("mod.json");
        let mod_metadata: ModMetadata = serde_json::from_reader(
            std::fs::File::open(mod_json_path)?
        )?;
        
        // 2. Validate against schema
        let jsonschema = jsonschema::JSONSchema::compile(&CHANNEL_MANIFEST_SCHEMA)?;
        
        // 3. Load all channel manifests
        let mut channels = Vec::new();
        for entry in std::fs::read_dir(mod_dir.join("channels"))? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "json") {
                let manifest_json: serde_json::Value = serde_json::from_reader(
                    std::fs::File::open(&path)?
                )?;
                
                // Validate against schema
                jsonschema.validate(&manifest_json)
                    .map_err(|e| anyhow!("Schema validation failed in {}: {}", path.display(), e))?;
                
                // Deserialize
                let manifest: ChannelManifest = serde_json::from_value(manifest_json)?;
                
                // Ensure provenance is set to this mod
                if manifest.provenance != format!("mod:{}", mod_metadata.id) {
                    return Err(anyhow!(
                        "Channel {} has provenance {}, expected mod:{}",
                        manifest.id, manifest.provenance, mod_metadata.id
                    ));
                }
                
                channels.push(manifest);
            }
        }
        
        // 4. Load primitives similarly
        let primitives = Self::load_primitives(mod_dir, &mod_metadata)?;
        
        Ok(LoadedMod {
            metadata: mod_metadata,
            channels,
            primitives,
        })
    }
}

pub struct ManifestMerger;

impl ManifestMerger {
    pub fn merge_registries(
        core_channels: Vec<ChannelManifest>,
        mut mods: Vec<LoadedMod>,
    ) -> Result<ChannelRegistry> {
        // Sort mods by load_order
        mods.sort_by_key(|m| m.metadata.load_order);
        
        let mut registry = ChannelRegistry::new();
        
        // Load core channels first
        for channel in core_channels {
            registry.register_channel(channel)?;
        }
        
        // Load mod channels in order
        for module in mods {
            for channel in module.channels {
                // Validate uniqueness: no mod can override core channel
                if registry.get(&channel.id).is_some() {
                    return Err(anyhow!(
                        "Mod {} tried to override core channel {}",
                        module.metadata.id, channel.id
                    ));
                }
                
                registry.register_channel(channel)?;
            }
        }
        
        Ok(registry)
    }
}
```

### 8.3 Data-Only Mods (MVP)

In MVP, mods are **data-only** (no Lua, no custom logic):
- Channel manifests (JSON)
- Primitive manifests (JSON)
- Biome templates (JSON)
- Material definitions (JSON)

Mods cannot:
- Add custom combat rules
- Add scripted NPC behavior
- Override core systems

**Post-MVP Expansion**: If a deep system is "modding," we introduce a scripting layer (Lua or WASM) for faction behavior, NPC dialog, or evolution-path customization.

---

## 9. Scale-Band Specialization (Disease)

### 9.1 Unified Genome, Filtered Expression

Pathogens use the same 9 channel families as macro-life. The difference is **scale-band filtering**:

```rust
pub struct ExpressionConditions {
    pub biome_flags: Vec<BiomeTag>,
    pub scale_band: (f32, f32),  // [min_kg, max_kg]
    pub seasonal: Option<Season>,
    pub developmental_stage: Option<DevelopmentalStage>,
}

// Channel expressibility check:
fn is_channel_expressed(
    channel: &ChannelManifest,
    creature: &Creature,
    environment: &Environment,
) -> bool {
    // Biome check
    if !channel.expression_conditions.biome_flags.is_empty() &&
       !channel.expression_conditions.biome_flags.contains(&environment.biome_tag) {
        return false;
    }
    
    // Scale-band check (CRITICAL for disease)
    let creature_mass_kg = creature.compute_mass_kg();
    let (min_kg, max_kg) = channel.scale_band;
    if creature_mass_kg < min_kg || creature_mass_kg > max_kg {
        return false;
    }
    
    // Season & developmental stage checks (similar)
    ...
    
    true
}
```

### 9.2 Host Coupling Formulas (from System 16)

Pathogen-specific composition hooks compute the **HostCouplingProfile**:

```rust
pub struct HostCouplingProfile {
    pub host_energetic_drain: Q3232,
    pub host_immune_recognition: Q3232,
    pub transmission_efficiency: Q3232,
    pub host_provided_benefit: Q3232,
    pub virulence: Q3232,
}

fn compute_host_coupling(
    pathogen: &Creature,
    host: &Creature,
    channel_registry: &ChannelRegistry,
) -> Result<HostCouplingProfile> {
    let pathogen_phenotype = interpret_phenotype(pathogen, channel_registry)?;
    
    // Formulae from System 16 Section 3.2
    let host_energetic_drain = -Q3232::from_num(0.8) * pathogen_phenotype.metabolic_rate
        + -Q3232::from_num(0.5) * pathogen_phenotype.resource_consumption_rate;
    
    let host_immune_recognition = pathogen_phenotype.surface_antigenicity
        * (Q3232::ONE - pathogen_phenotype.immune_evasion_strength * Q3232::from_num(0.7));
    
    let transmission_efficiency = pathogen_phenotype.motility
        * pathogen_phenotype.host_tropism_breadth
        * transmission_modality_factor(pathogen_phenotype.transmission_modality);
    
    let host_provided_benefit = if host.phenotype.immune_tolerance_breadth > Q3232::from_num(0.7) {
        pathogen_phenotype.nutrient_synthesis * Q3232::from_num(0.8)
            + pathogen_phenotype.metabolic_support * Q3232::from_num(0.4)
    } else {
        Q3232::ZERO
    };
    
    let virulence = host_energetic_drain.abs()
        + pathogen_phenotype.tissue_disruption_rate * Q3232::from_num(0.5);
    
    Ok(HostCouplingProfile {
        host_energetic_drain,
        host_immune_recognition,
        transmission_efficiency,
        host_provided_benefit,
        virulence,
    })
}
```

---

## 10. Chronicler Integration

### 10.1 Labeling Pipeline

The Chronicler runs asynchronously (every N ticks):

```rust
pub struct ChroniclerSystem;

impl System for ChroniclerSystem {
    fn run(&mut self, world: &World, resources: &mut Resources) -> Result<()> {
        // Run every 100 ticks
        if resources.tick_counter % 100 != 0 {
            return Ok(());
        }
        
        // 1. Snapshot primitive effects across population
        let phenotype_storage = world.read_storage::<Phenotype>();
        let mut primitive_fingerprints = std::collections::HashMap::new();
        
        for phenotype in phenotype_storage.join() {
            let signature = self.compute_primitive_signature(&phenotype)?;
            *primitive_fingerprints.entry(signature).or_insert(0) += 1;
        }
        
        // 2. Detect recurring patterns (stable signatures)
        for (signature, count) in primitive_fingerprints {
            if count > 5 {  // Threshold: pattern present in 5+ individuals
                // Check if this signature already has a label
                if !resources.chronicler.has_label(&signature) {
                    // Generate new label (post-hoc naming)
                    let label = self.generate_label(&signature)?;
                    resources.chronicler.register_label(&signature, &label)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn stage(&self) -> SystemStage { SystemStage::ChroniclerPattern }
}

fn compute_primitive_signature(phenotype: &Phenotype) -> Result<String> {
    // Sort primitive IDs and join as comma-separated string
    let mut primitives: Vec<String> = phenotype.primitive_effects
        .iter()
        .map(|p| format!("{:?}", p))
        .collect();
    primitives.sort();
    Ok(primitives.join(","))
}

fn generate_label(signature: &str) -> Result<String> {
    // Heuristic naming based on signature
    // E.g., (emit_acoustic_pulse, receive_acoustic_signal, spatial_integrate) → "echolocation"
    
    // In MVP: use simple templating
    // In deep system (culture): generate names via faction namespaces
    
    // For now: descriptive label
    if signature.contains("emit_acoustic") && signature.contains("receive_acoustic") {
        Ok("echolocation-like".to_string())
    } else if signature.contains("emit_pheromone") && signature.contains("receive_pheromone") {
        Ok("chemical_signaling".to_string())
    } else {
        Ok(format!("pattern_{:x}", hash(signature)))
    }
}
```

### 10.2 Query API

UI reads via Chronicler query API:

```rust
impl Chronicler {
    pub fn query_creatures_with_pattern(&self, signature: &str) -> Vec<EntityID> {
        // Return all creatures whose primitive signature matches
        self.pattern_index.get(signature).cloned().unwrap_or_default()
    }
    
    pub fn query_label_for_signature(&self, signature: &str) -> Option<String> {
        self.label_map.get(signature).cloned()
    }
    
    pub fn query_all_labels(&self) -> Vec<String> {
        self.label_map.values().cloned().collect()
    }
    
    pub fn query_creatures_with_label(&self, label: &str) -> Vec<EntityID> {
        // Find all signatures with this label, collect creatures
        self.label_map
            .iter()
            .filter(|(_, l)| l == label)
            .flat_map(|(sig, _)| self.pattern_index.get(sig).cloned().unwrap_or_default())
            .collect()
    }
}
```

---

## 11. Testing Strategy

### 11.1 Unit Tests per Crate

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixed_point_saturating_arithmetic() {
        let a = Q3232::from_num(0.5);
        let b = Q3232::from_num(0.7);
        let sum = a.saturating_add(b);
        assert_eq!(sum, Q3232::ONE);  // Clamped to max
    }
    
    #[test]
    fn test_channel_manifest_deserialization() {
        let json = include_str!("fixtures/kinetic_force.json");
        let manifest: ChannelManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.id, "kinetic_force");
        assert_eq!(manifest.family, ChannelFamily::Motor);
    }
    
    #[test]
    fn test_mutation_deterministic() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(12345);
        let mutant_1 = mutate_channel(&mut rng, Q3232::from_num(0.5), Q3232::from_num(0.1));
        
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(12345);
        let mutant_2 = mutate_channel(&mut rng, Q3232::from_num(0.5), Q3232::from_num(0.1));
        
        assert_eq!(mutant_1, mutant_2);  // Bit-identical
    }
}
```

### 11.2 Property Tests (proptest)

```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn prop_channels_always_in_range(
            channel_val in any::<u64>(),
        ) {
            let val = Q3232::from_bits(channel_val);
            let clamped = val.clamp(Q3232::ZERO, Q3232::ONE);
            prop_assert!(clamped >= Q3232::ZERO && clamped <= Q3232::ONE);
        }
        
        #[test]
        fn prop_genome_mutations_preserve_length(
            mutations in (1..10usize),
            seed in any::<u64>(),
        ) {
            let mut genome = create_test_genome(10);
            let original_len = genome.genes.len();
            
            let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
            for _ in 0..mutations {
                apply_mutation(&mut genome, &mut rng).unwrap();
            }
            
            // Mutations can delete/insert genes; length changes
            // But total information is preserved (for now; test that genes are valid)
            for gene in &genome.genes {
                prop_assert!(gene.magnitude >= Q3232::ZERO && gene.magnitude <= Q3232::ONE);
            }
        }
    }
}
```

### 11.3 Snapshot Tests (insta)

```rust
#[test]
fn test_interpreter_output_snapshot() {
    let genome = create_test_genome();
    let environment = TestEnvironment::new();
    let registry = ChannelRegistry::default();
    
    let effects = interpret_phenotype(&genome, &environment, &registry).unwrap();
    
    insta::assert_debug_snapshot!(effects);
}

// First run generates `test_interpreter_output_snapshot.snap`
// Subsequent runs compare; if output changes, test fails with diff
```

### 11.4 Determinism Replay Test (CI)

(See Section 6.3 for full implementation)

---

## 12. Build & CI

### 12.1 Cargo Workspace

```toml
[workspace]
members = [
    "crates/beast-core",
    "crates/beast-channels",
    "crates/beast-primitives",
    "crates/beast-genome",
    "crates/beast-interpreter",
    "crates/beast-evolution",
    "crates/beast-disease",
    "crates/beast-ecs",
    "crates/beast-sim",
    "crates/beast-chronicler",
    "crates/beast-serde",
    "crates/beast-render",
    "crates/beast-ui",
    "crates/beast-audio",
    "crates/beast-mod",
    "crates/beast-cli",
    "crates/beast-app",
]
```

### 12.2 Dependency Auditing

```toml
[dev-dependencies]
cargo-deny = "0.15"

# In CI: cargo deny check advisories
```

### 12.3 CI Matrix (GitHub Actions)

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, nightly]
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      
      - name: Cargo test (all features)
        run: cargo test --workspace --all-features
      
      - name: Determinism replay validation
        run: cargo test --test determinism_test
      
      - name: Check invariants
        run: bash scripts/check_determinism_invariants.sh
      
      - name: Cargo clippy (deny warnings)
        run: cargo clippy --workspace -- -D warnings
      
      - name: Benchmarks (if main)
        run: |
          if [[ "${{ github.ref }}" == "refs/heads/main" ]]; then
            cargo bench --workspace -- --output-format bencher | tee output.txt
            curl -H "Authorization: token ${{ secrets.GITHUB_TOKEN }}" \
              -d @output.txt https://api.github.com/repos/${{ github.repository }}/commits/${{ github.sha }}/comments
          fi
```

---

## 13. MVP Scope vs. Deep System Horizon

### 13.1 MVP Scope

**World**:
- Single 1000x1000 grid biome (grassland)
- 3-5 biome cell types (grassland, forest, water, ruin)
- Simple climate (no seasonal variation for MVP)

**Creatures**:
- 1-2 species (hand-authored, then evolved)
- ~100-1000 creatures on map at once
- Basic channels: kinetic_force, structural_rigidity, metabolic_rate, neural_speed
- Simple phenotypes: movement speed, defense, attack damage

**Combat**:
- Formation-less 1v1 and small group encounters
- Primitive effects: apply_bite_force, receive_signal (detect)
- Damage formula: force × rigidity / defense

**Bestiary**:
- Species list (dynamically discovered)
- Observation count, health, diet, basic traits

**Save/Load**:
- Binary save format (bincode)
- Full replay validation

**Chronicler**:
- Basic pattern recognition (no complex analysis)
- Simple labels ("aggressive", "predatory", "small")

### 13.2 "One Full Deep System" Placeholders

The architecture supports one of these deep-system paths post-MVP. Which system is deep depends on user choice during planning.

#### **Deep System A: Evolution Depth**
- Channel genesis (full paralog duplication & reclassification)
- Epistasis modeling (NK fitness landscape)
- Speciation mechanics (reproductive isolation)
- Multi-generational strategy evolution

**MVP → Deep**:
- `beast-genome`: add paralog tracking, reclassification logic
- `beast-evolution`: add speciation model, phylogenetic tracking
- `beast-chronicler`: track lineage trees, adaptive radiation events

#### **Deep System B: Disease**
- Micro-scale pathogen evolution (hundreds of pathogen types)
- Host-pathogen coevolution dynamics
- Transmission networks (contact graph)
- SEIR-like epidemiology

**MVP → Deep**:
- `beast-disease`: expand HostCoupling formulas, transmission sim
- `beast-sim`: add transmission phase to tick loop
- `beast-chronicler`: epidemic timeline tracking

#### **Deep System C: Economy**
- Settlements, trading, resource management
- Faction economics (supply/demand, trade routes)
- Crafting system (materials → items → artifacts)
- Player base building & defense

**MVP → Deep**:
- `beast-ecs`: add Settlement, Resource, Faction entities
- `beast-sim`: add economic tick logic
- `beast-ui`: settlement/trading UI overlays

#### **Deep System D: Language & Culture**
- Faction-specific naming conventions
- Language emergence & drift
- Cultural trait evolution (biased transmission)
- Narrative generation (Chronicler → storytelling)

**MVP → Deep**:
- `beast-chronicler`: full narrative generation engine
- `beast-mod`: culture & language mod templates
- `beast-ui`: narrative display, folklore browser

**Architecture ensures all are viable**: Channel registry is extensible; mod system supports new mechanics; Chronicler API is generic.

---

## 13. Minor Issues (Combined)

### Issue #9: Genome Channel Storage & Body-Site Modifiers

**Specification**: Channels have two storage forms:
1. **Global channel_values**: array of Q32.32 values (one per channel), stored in Genome.channel_values
2. **Per-body-site modifiers** (optional): if a channel has `body_site_applicable=true`, each body site can apply an independent modifier

**Application in Interpreter**: 
- Interpreter reads global channel_values
- If channel is body_site_applicable, interpreter evaluates each body region with: `effective_value = global_channel_value × body_site_modifier`
- Composition hooks referencing body-site channels produce per-site primitive effects

**Storage Strategy**: Modifiers are stored in a sparse map (only populated for channels that have body_site_applicable):
```rust
pub struct Genome {
    pub channel_values: Vec<Q3232>,  // Global [0,1] values
    pub body_site_modifiers: HashMap<(ChannelID, BodySiteID), Q3232>,  // Sparse: only if applicable
}
```

---

### Issue #10: R*-Tree Spatial Index Rebuild & Query Budget

**Specification**: 
- R*-tree is rebuilt at the **start of the Physics stage** (Stage 3)
- All spatial queries within the Physics stage use the same tree (no rebuild mid-stage)
- Profiling target: <1ms rebuild time for 1000 creatures

**Performance Requirement**:
```rust
#[test]
fn test_rstar_rebuild_performance_1000_creatures() {
    let mut tree: RTree<CreatureLocation> = RTree::new();
    for i in 0..1000 {
        tree.insert(CreatureLocation {
            pos: Vec2 { x: (i % 100) as f32, y: (i / 100) as f32 },
            id: EntityID(i as u32),
        });
    }
    
    let start = Instant::now();
    // Simulate rebuild: remove + re-insert all
    let entries: Vec<_> = tree.drain().collect();
    for entry in entries {
        tree.insert(entry);
    }
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 1, "Rebuild took {}ms", elapsed.as_millis());
}
```

---

### Issue #11: SDL3 Fallback Plan & Abstraction Layer

**Specification**: A rendering abstraction layer in `beast-render` allows swapping SDL3 for SDL2 if needed:

```rust
pub trait RenderBackend: Send + Sync {
    fn create_canvas(&mut self, width: u32, height: u32) -> Result<()>;
    fn present(&mut self);
    fn draw_rect(&mut self, rect: Rect, color: Color);
    // ... other drawing operations
}

pub struct Sdl3Backend { /* SDL3 context */ }
pub struct Sdl2Backend { /* SDL2 context */ }

impl RenderBackend for Sdl3Backend { /* impl */ }
impl RenderBackend for Sdl2Backend { /* impl */ }

pub struct Renderer {
    backend: Box<dyn RenderBackend>,
}
```

**MVP uses SDL3; fallback to SDL2 via feature flag**:
```toml
# Cargo.toml
[features]
default = ["sdl3"]
sdl3 = ["sdl3-sys"]
sdl2 = ["sdl2"]
```

---

### Issue #12: Schedule Order Assertion at Startup

**Specification**: Assert that systems are registered in correct stage order. MVP mods **cannot** add new systems (data-only mods).

```rust
pub fn assert_schedule_order(systems: &[Box<dyn System>]) -> Result<()> {
    let mut last_stage = SystemStage::Input;
    
    for system in systems {
        let current_stage = system.stage();
        
        // Stages must be monotonically increasing
        if (current_stage as u32) < (last_stage as u32) {
            return Err(anyhow!(
                "System {} is in stage {:?}, but previous system was in stage {:?}",
                system.name(), current_stage, last_stage
            ));
        }
        
        last_stage = current_stage;
    }
    
    Ok(())
}

// Call at startup
pub fn main() {
    let systems = vec![
        Box::new(InputSystem),
        Box::new(MutationSystem),
        Box::new(InterpreterSystem),
        // ... rest of systems
    ];
    
    assert_schedule_order(&systems).expect("Schedule order violation");
    // ... run simulation
}
```

---

## 14. Open Questions & Addressing Framework

### Q1: Pathogen Initial Conditions & Seeding

**Question**: How are pathogens procedurally seeded by worldgen with micro-scale mass and micro-only channels expressed?

**Answer**: Worldgen adds to each worldgen (System 07) during biome initialization:
- **Pathogen Seeding**: For each biome cell, generate N micro-scale pathogens (1e-15 to 1e-3 kg range) with random genotypes
- **Channel Constraint**: Pathogen genotypes inherit only channels with `scale_band` allowing [1e-15, 1e-3] kg (micro-only channels)
- **Host Attachment**: Pathogens express `host_attachment` primitive to couple with macro creatures (System 16 handles transmission)
- **Note**: This is documented in System 16 (Disease & Parasitism)

**Implementation Note**: See `systems/16_disease_parasitism.md` for full pathogen initialization, transmission, and scale-band coupling.

---

### Q2: Deep-System Feature Gating

**Question**: Are all deep-system crates compiled but feature-flagged for MVP, or truly absent?

**Answer**: All deep-system crates are **compiled but feature-flagged** for MVP:

```toml
# beast-app Cargo.toml
[features]
default = ["mvp-core"]
mvp-core = ["beast-disease", "beast-interpreter", ...]
deep-evolution = ["beast-evolution-deep"]
deep-disease = ["beast-disease-deep"]
deep-economy = ["beast-economy"]
deep-culture = ["beast-language", "beast-culture"]

[dependencies]
beast-disease-deep = { path = "../crates/beast-disease-deep", optional = true }
beast-economy = { path = "../crates/beast-economy", optional = true }
```

**MVP Build**: `cargo build --features=mvp-core` (excludes deep crates; binary smaller, compile faster)

**Deep System Build**: `cargo build --features=mvp-core,deep-disease,deep-evolution` (includes selected deep systems)

---

### Q3: Parallel System PRNG Isolation

**Question**: How are parallel systems (rayon workers) seeded for PRNG independence?

**Answer**: Each rayon worker thread gets a **jumped clone** of the subsystem RNG:

```rust
pub fn parallel_fitness_eval(population: &[Creature], ecology_rng: &Xoshiro256PlusPlus) -> Vec<f32> {
    population.par_iter().map_init(
        || {
            // Each thread initializes its own RNG
            let mut local_rng = ecology_rng.clone();
            local_rng.jump();  // Advance by 2^128 steps
            local_rng
        },
        |rng, creature| {
            evaluate_fitness(creature, rng)
        },
    ).collect()
}
```

**Determinism Guarantee**: RNG state is deterministic (seeded at world creation); parallel evaluation order is deterministic (sorted entity iteration).

---

### Q4: Phenotype Cache Invalidation

**Question**: When is the phenotype cache invalidated during evolution?

**Answer**: Phenotype cache is invalidated on:
1. **Any mutation**: creature's genome changed → phenotype becomes stale
2. **Channel registry reload**: new channel added (genesis) or channel manifest changed → all phenotypes using old registry are stale

**Implementation**:
```rust
pub struct Phenotype {
    pub primitive_effects: HashSet<PrimitiveEffectID>,
    pub cache_tick: u64,  // Last tick phenotype was computed
    pub cache_registry_version: u32,  // Version of channel_registry used
}

pub fn interpreter_tick(creature: &mut Creature, resources: &Resources) {
    // Check if cache is valid
    let cache_valid = 
        creature.phenotype.cache_tick == resources.tick_counter &&
        creature.phenotype.cache_registry_version == resources.channel_registry.version();
    
    if cache_valid {
        return;  // Skip recomputation
    }
    
    // Recompute phenotype
    creature.phenotype = interpret_phenotype(&creature.genome, &resources)?;
    creature.phenotype.cache_tick = resources.tick_counter;
    creature.phenotype.cache_registry_version = resources.channel_registry.version();
}
```

---

## 14. Open Questions & Risks

### 14.1 Unresolved Architectural Decisions

| Question | Impact | Risk | Mitigation |
|----------|--------|------|-----------|
| **UI Widget Library**: Custom retained-mode vs. imgui-rs | Implementation time, maintainability | 2000 LOC custom code is significant | Prototype first; consider imgui-rs if custom costs exceed 20%. |
| **Pathfinding in ECS**: A* per NPC or global grid? | Performance (100s of NPCs) | A* per NPC every tick is O(N × pathfinding_cost) | Use hierarchical pathfinding (nav meshes); defer distant NPC pathing to every 10 ticks. |
| **Mod Scripting**: Data-only (MVP) or Lua/WASM for deep system? | Modding ecosystem depth | Lua adds security/sandboxing concerns | Defer to deep system; use Lua 5.4 + careful sandboxing if needed. |
| **Rendering Performance**: Sprite atlas vs. procedural mesh gen | Asset management, visual fidelity | System 10 procedural gen is deterministic but slow | Bake meshes offline from procedural rules; load at startup. |
| **Network Multiplayer**: Lockstep replay or cloud determinism? | Scalability, latency tolerance | Lockstep requires 100% determinism; cloud is single-point failure | MVP is single-player; defer to expansion; prototype lockstep with CI replay test. |

### 14.2 Known Risks

1. **Fixed-Point Precision**: Q32.32 gives ~2^-32 ≈ 0.23e-9 precision. For small channel values (e.g., 1e-6), rounding errors accumulate. **Mitigation**: Monitor precision in replay tests; if divergence detected, profile which channel drifts.

2. **Genetic Algorithm Convergence**: Evolution model can prematurely converge to local optima. **Mitigation**: Global epistasis penalty (fitness effect diminishes with background fitness) prevents runaway; mutation rates tuned to maintain drift. Property tests verify diversity maintenance.

3. **Chronicler Labeling False Positives**: Clustering primitives by signature may assign same label to non-homologous patterns. **Mitigation**: Use confidence scores; UI displays "pattern_hash_xyz" if confidence < 0.8; labels are informational, not mechanically binding.

4. **Render Thread Determinism**: If rendering code touches sim state without snapshot isolation, determinism is violated. **Mitigation**: Strict code review; snapshot is immutable; render thread panics if it tries to mutate sim state (debug assertion).

5. **Manifest Versioning**: If core manifests change, old save files may load with incompatible channel IDs. **Mitigation**: Schema versioning in save files; migrations applied at load time (e.g., "auditory_sensitivity_v1" → "auditory_sensitivity_v2" with adjusted parameters).

---

## Appendix: Key Dependencies Summary

| Crate | Version | Purpose | License |
|-------|---------|---------|---------|
| `specs` | ^0.20 | ECS framework | Apache 2.0 / MIT |
| `fixed` | ^1.30 | Fixed-point (Q32.32) | Apache 2.0 / MIT |
| `rand_xoshiro` | ^0.6 | PRNG | Apache 2.0 / MIT |
| `serde` | ^1.0 | Serialization derive | Apache 2.0 / MIT |
| `bincode` | ^1.3 | Binary serialization | MIT |
| `serde_json` | ^1.0 | JSON serialization | Apache 2.0 / MIT |
| `jsonschema` | ^0.24 | JSON schema validation | Apache 2.0 / MIT |
| `sdl3-sys` | ^0.5 | SDL3 bindings | MIT |
| `rstar` | ^0.12 | R*-tree spatial index | MIT |
| `rayon` | ^1.8 | Data-parallel iteration | Apache 2.0 / MIT |
| `criterion` | ^0.5 | Micro-benchmarking | Apache 2.0 / MIT |
| `insta` | ^1.34 | Snapshot testing | Apache 2.0 / MIT |
| `proptest` | ^1.4 | Property-based testing | Apache 2.0 / MIT |

All dependencies are permissively licensed (Apache 2.0 / MIT). No GPL dependencies introduced.

---

## Document Version

- **Version**: 1.0
- **Date**: 2026-04-14
- **Status**: Approved for MVP architecture
- **Next Review**: After first 2-week sprint (crate scaffolding complete)
