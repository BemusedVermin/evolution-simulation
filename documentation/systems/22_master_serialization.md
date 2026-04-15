# System 22: Master Serialization & Save State

## 1. Overview

The simulation is the single source of truth. World state is serialized at tick boundaries into a canonical, version-tagged schema. This enables:
- **Save/Load**: Player checkpoints world state; can resume later.
- **Replay**: Given a seed and input sequence, the simulation is deterministic; replays are bit-identical.
- **Modding & Scenarios**: Starter-state format allows custom initial conditions (geography, factions, species, technologies).
- **Delta Journaling**: Incremental changes since last checkpoint for crash recovery and bandwidth-efficient network multiplayer (future expansion).
- **Performance Budgets**: Hot tick budget ~16ms (60 FPS); per-system tick budgets allocated; non-essential subsystems deferred.

**Key principle**: Serialization schema is stable and versioned. Future updates to the code are backward-compatible; old save files can be loaded and "upgraded" to new schema.

This doc establishes engineering invariants, not gameplay mechanics. It is the contract between the simulation and persistence layer.

---

## 2. Research Basis

### State Representation & Determinism (Ford & Coulter, 2014; Cook, 2014)
Deterministic simulations require no randomness post-hoc; all randomness must be seeded and logged. RTS games (StarCraft, Age of Empires) use deterministic lockstep simulation to enable replays. The simulation must be deterministic given: initial state + seed + input sequence. This is achieved by seeding the PRNG with a world seed at startup and logging all input timestamps.

- Ford, B. & Coulter, R. (2014). "Deterministic Replay of C++ Multithreaded Applications." In *Proceedings of OSDI*.
- Cook, D. (2014). "Post Mortem: Dwarf Fortress." *Gamasutra*.

**Application**: World has a root seed (integer) that seeds all RNGs. Ticks are deterministic. Player input (avatar movement, breeding pairs, etc.) is timestamped with tick number. Replay = seed + input log → identical world state.

### Entity-Component-System (ECS) Architecture (West, 2019; Bilas, 2002)
ECS decouples data (components) from logic (systems). Data is stored in structure-of-arrays (SOA) for cache efficiency and serialization simplicity. Entities are integer IDs. This makes serialization straightforward: serialize the component arrays and entity ID mappings.

- West, M. (2019). "ECS Back and Forth." In *GDC Vault*.
- Bilas, S. (2002). "A Data-Driven Game Object System." In *GDC Proceedings*.

**Application**: All entities (Agent, Settlement, Creature, Biome Cell, etc.) are stored as ECS components. Serialization is: for each component type, serialize the array. Deserialization is: for each component type, deserialize the array.

### Schema Versioning & Backwards Compatibility (Fowler, 2006; Ambler, 2003)
To support old save files with new code, schema must be versioned. Each save file records the schema version. When loading an old save, the code applies a series of "migrations" to upgrade the schema. This allows the code to evolve without breaking old saves.

- Fowler, M. (2006). *Refactoring: Improving the Design of Existing Code*. Addison-Wesley.
- Ambler, S.W. & Sadalage, P.J. (2003). *Refactoring Databases*. Addison-Wesley.

**Application**: Save file header includes schema_version (e.g., "2.1.3"). Code contains migrations: if save is schema 2.0 and code is 2.1, apply migration_2_0_to_2_1(). This is applied iteratively until save schema matches code schema.

### Performance Budgets & Priority Queues (Gregory, 2018; Akenine-Möller et al., 2018)
To maintain 60 FPS, each tick must complete in ~16ms. This is achieved by: (1) hot path optimization for core simulation, (2) deferring non-essential systems (e.g., pathfinding for NPCs not in focus), (3) adaptive quality (reduce update frequency for distant agents). A priority queue orders systems and subsystems by cost/importance ratio.

- Gregory, J. (2018). *Game Engine Architecture* (3rd ed.). CRC Press.
- Akenine-Möller, T., et al. (2018). *Real-Time Rendering* (4th ed.). CRC Press.

**Application**: Each tick allocates ~16ms budget. Systems are scheduled via priority queue: hot systems (agent needs, ecosystem updates) run every tick; cold systems (long-term history compression, distant NPC pathfinding) run every 10–100 ticks. Budget is monitored; if over-budget, cold systems are skipped.

---

## 2.5 Determinism Contract

All simulation-state math must be **deterministic and bit-identical** across replays. Floats are forbidden in sim state; only fixed-point arithmetic and seeded PRNG are allowed.

**Representation**:
- Continuous quantities [0, 1]: Q32.32 fixed-point (64-bit signed integer, treating as 32 bits integer + 32 bits fractional).
- Counts and temporal values: i32 or i64 (never float).
- Serialized form: fixed-point values encoded as integers in JSON; never as decimal floats.

**PRNG Specification**:
- Algorithm: xoshiro256** (Blackman & Vigna, 2018) seeded at world creation.
- Seeding: world_seed (user-provided or procedural) initializes four 64-bit state variables via SplitMix64.
- One stream per subsystem: Evolution, Ecology, Agents, Settlements, Combat, etc. each maintain independent xoshiro256** state to prevent cross-contamination.
- Determinism: PRNG state is serialized in every save; replay restores exact state and re-executes identical sequence.

**Iteration Order**:
- All entity loops: iterate over sorted entity IDs (ascending order).
- No unordered maps in hot tick paths; only sorted dicts or array indices.
- Composition hook evaluation: ordered by channel ID and body region ID.

**Wall-Clock Independence**:
- Tick count is the sole source of timing truth (no gettimeofday(), clock(), or OS timers in sim code).
- All timing decisions: if (current_tick % cooldown_ticks == 0) or delta = tick_N - tick_M; never elapsed_ms.

**Replay Validation** (CI Test):
```
function test_deterministic_replay():
  save_0 = load_save_file("determinism_test_state.json")
  inputs = load_input_journal("determinism_test_inputs.json")
  
  # Run forward N ticks and snapshot
  state_snapshots_original = []
  for tick in 0..N:
    tick_simulation(save_0, tick, inputs[tick])
    state_snapshots_original.append(compute_state_hash(save_0))
  
  # Reload and replay
  save_1 = load_save_file("determinism_test_state.json")
  state_snapshots_replay = []
  for tick in 0..N:
    tick_simulation(save_1, tick, inputs[tick])
    state_snapshots_replay.append(compute_state_hash(save_1))
  
  # Verify bit-identity
  assert state_snapshots_original == state_snapshots_replay, "Replay diverged"
  print("Determinism test PASSED")
```

**Failure Analysis**:
- If divergence detected: compute binary diff of tick snapshots; identify first differing value (entity ID, channel, count).
- Review: check for uninitialized floating-point, platform-specific rounding, or unordered iteration.
- Remediation: fix root cause, re-run test, verify with multiple compiler/platform combinations.

---

## 2.6 UI State vs. Sim State (Formal Boundary)

See System 23 (UI Overview, Section 6.5) for the complete formal specification.

**Sim State** (serialized, versioned, deterministic):
- **Entities**: Agent, Creature, Settlement, BiomeCell, Faction, Technology, Language, CulturalTrait, etc.
- **Channel Values**: fixed-point Q32.32; phenotypic traits.
- **Observation Counts**: per-species integer incremented each time a Creature is sighted; used to determine UI "discovered" status.
- **Labels** (derived by Chronicler, see System 09 Sections 16–17): Canonical names, confidence scores, provenance (thesaurus / compositional / faction-coined), first-seen ticks. Labels are computed from population-level primitive patterns and stored in sim state.
- **Primitive Fingerprints**: Structural signatures of stable primitive patterns; used to derive labels.

**UI State** (ephemeral, non-canonical, never serialized to save file):
- Camera position and zoom level.
- Open tabs and filter selections (e.g., bestiary filters).
- Player-authored private notes and custom creature/location names.
- Bestiary "discovered" flag: **computed at load time** as (observation_count >= 1); never written to save file.
- Sort order and display options.
- Scroll positions and collapsed/expanded card states.

**Invariant**: Save file validation schema rejects any file containing a `bestiary_discovered` key or any other UI-ephemeral state. The "discovered" status is deterministically re-derived at load: `discovered := observation_count >= 1`.

**Rationale**: By serializing observation_count but not the derived discovered flag, the simulation preserves the ability to learn about extinct species (count > 0, species extinct) while ensuring UI state can never retroactively affect future deterministic playback.

---

## 3. Canonical World State Schema

All entities are stored in ECS format. Here is the canonical schema:

### Top-Level Save File

```
SaveFile {
  format_version: "2.2.0",  // Semantic versioning
  game_version: string,      // Code version that created this save
  creation_timestamp: int,   // Unix timestamp
  current_tick: int,         // World clock
  
  // Random state
  world_seed: int,
  rng_state: {
    mt19937_state: [int, ...],  // Mersenne Twister state
    rng_position: int,
  },
  
  // Configuration
  permadeath_mode: enum { Sandbox, Hardcore, Ironman },
  difficulty_settings: {
    creature_mortality_modifier: float,  // 1.0 = normal; 2.0 = fast death
    npc_mortality_modifier: float,
    resource_scarcity_modifier: float,
    disease_frequency_modifier: float,
  },
  
  // Core world state (entities)
  entities: {
    agents: [Agent, ...],
    creatures: [Creature, ...],
    settlements: [Settlement, ...],
    biome_cells: [BiomeCell, ...],
    factions: [Faction, ...],
    knowledge_facts: [KnowledgeFact, ...],
    chronicle_entries: [ChronicleEntry, ...],
    technologies: [Technology, ...],
    languages: [Language, ...],
    cultural_traits: [CulturalTrait, ...],
    materials_and_stacks: [MaterialStack, ...],
    items_and_equipment: [EquipmentPiece, ...],
  },
  
  // Entity ID mappings (lookup tables)
  entity_id_to_type: { [id: int]: enum { Agent, Creature, Settlement, ... } },
  entity_id_to_array_index: { [id: int]: int },  // Index in component array
  
  // World parameters
  world_parameters: {
    MAP_WIDTH: int,
    MAP_HEIGHT: int,
    CURRENT_CALENDAR_YEAR: int,
    CURRENT_CALENDAR_SEASON: enum { Spring, Summer, Fall, Winter },
    CURRENT_CALENDAR_DAY: int,  // 1-30
  },
  
  // Checksum for integrity
  save_checksum: int,  // CRC32 of all entity data for corruption detection
}
```

### Agent Entity

```
Agent {
  entity_id: int,           // Global unique ID
  type: "Agent",
  
  // Identification
  name: string,
  faction_id: int,
  settlement_id: int or null,
  
  // State
  age_ticks: int,
  health: float [0, 1],
  energy: float [0, 1],
  
  // Needs & Status (NeedsVector, from System 04)
  needs: {
    hunger: float [0, 1],
    fatigue: float [0, 1],
    social: float [0, 1],
    safety: float [0, 1],
    stimulation: float [0, 1],
    direction: float [0, 1],
  },
  
  // Inventory
  carried_materials: [MaterialStack, ...],
  equipped_items: [EquipmentPiece, ...],
  
  // Skills & Techniques (System 04)
  techniques: {
    [technique_id]: {
      skill_points: float,
      last_practiced_tick: int,
    }
  },
  
  // Mind (from System 03)
  faction_opinions: { [faction_id]: float },
  agent_opinions: { [agent_id]: float },
  
  // Memory (from System 17)
  episodic_memory: [EpisodicTrace, ...],
  spatial_memory: { [location_id]: SpatialMemoryEntry },
  learned_creature_abilities: {
    [species_id]: {
      ability_id: int,
      encounter_count: int,
      recent_use_frequency: float,
      confidence: float,
    }
  },
  
  // Position & Movement
  current_location: Location { x: float, y: float },
  current_activity: enum {
    Idle, Movement, Gathering, Crafting, Combat, Rest, Social, ...
  },
  current_activity_target_id: int or null,
  
  // Optional: Player Avatar
  is_player_avatar: bool,
  player_avatar_data: {
    lineage_id: int,
    career_primary: enum { Keeper, Scholar, Merchant, ... },
    captive_creature_lineages: [
      {
        species_id: int,
        current_population: [creature_id, ...],
        generations_of_selection: int,
      }
    ],
  } or null,
}
```

### Creature Entity

```
Creature {
  entity_id: int,
  type: "Creature",
  
  // Identity
  species_id: int,
  individual_id: int,  // Unique per species
  sex: enum { Male, Female },
  
  // Genetics (System 01)
  genotype: {
    genes: [
      {
        channel: int,
        magnitude: float,
        body_site: BodyVector,
      },
      ...
    ],
    regulatory_network: [Modifier, ...],
  },
  
  // Phenotype (derived from genotype)
  phenotype: {
    channel_values: [float; 18],  // One value per channel
    body_topology: {
      regions: [BodyRegion, ...],
      visual_features: [string, ...],  // Descriptive
    }
  },
  
  // State
  age_ticks: int,
  health: float [0, 1],
  energy: float [0, 1],
  
  // Behavior & AI (System 06)
  current_behavior: enum { Idle, Hunting, Fleeing, Resting, Mating, Migrating },
  target_creature_or_location_id: int or null,
  
  // Population context
  population_region_id: int,  // Biome region it belongs to (System 20)
  
  // Captivity (if player avatar owns it)
  captor_agent_id: int or null,
  is_captive: bool,
}
```

### Settlement Entity

```
Settlement {
  entity_id: int,
  type: "Settlement",
  
  // Identity
  name: string,
  faction_id: int,
  location: (x: float, y: float),
  
  // Population & Resources
  population_count: int,
  carrying_capacity: float,
  citizen_list: [agent_id, ...],  // Residents
  
  // Economy (System 04)
  stockpile: [MaterialStack, ...],
  active_markets: [Market, ...],
  
  // Infrastructure (System 04)
  facilities: [Facility, ...],
  
  // Health (System 16)
  disease_pressure: float [0, 1],
  sick_count: int,
  sanitation_level: float [0, 1],
  
  // Migration State (System 20)
  migration_state: {
    current_state: enum { Settled, MigrationPlanned, InMigration, ... },
    destination_settlement_id: int or null,
    migrating_population_count: int,
  },
  
  // Governance (System 03)
  government_type: enum { Tribal, Oligarchic, Monarchic, Democratic },
  leader_agent_id: int or null,
}
```

### BiomeCell Entity (for System 12 Ecology, System 15 Climate)

```
BiomeCell {
  entity_id: int,
  type: "BiomeCell",
  
  // Location
  location: (x: int, y: int),  // Grid coordinates
  
  // Terrain & Climate
  terrain_type: enum { Grassland, Forest, Mountain, Desert, Ocean, ... },
  climate: {
    temperature_C: float,
    precipitation_mm: float,
    wind_speed_mps: float,
  },
  
  // Ecology (System 12)
  ecology: {
    species_populations: {
      [species_id]: {
        count: int,
        total_biomass_kg: float,
        diet_composition: { [species_id]: float },
      }
    },
    vegetation_calories: float,
    vegetation_regeneration_rate: float,
  },
  
  // Resources (System 07 Exploration, System 04 Economy)
  resource_deposits: [ResourceDeposit, ...],
  
  // Dangers (System 06)
  hazard_list: [
    {
      hazard_type: enum { Predator, LandslideRisk, Flood, Disease, ... },
      severity: float [0, 1],
    }
  ],
}
```

### Faction Entity

```
Faction {
  entity_id: int,
  type: "Faction",
  
  // Identity
  name: string,
  color: (r: int, g: int, b: int),
  
  // Members & Settlements
  member_agent_ids: [agent_id, ...],
  settlement_ids: [settlement_id, ...],
  
  // Governance (System 03)
  government_type: enum { Tribal, Oligarchic, Monarchic, Democratic },
  leader_agent_id: int,
  opinion_network: {
    [faction_id]: {
      opinion_valence: float [−1, 1],
      recent_interactions_tick: int,
      alliance_status: enum { Allied, Neutral, Hostile },
    }
  },
  
  // Knowledge (System 03, F4)
  knowledge_facts: [KnowledgeFact, ...],
  
  // Language & Culture (System 18)
  primary_language_id: int,
  cultural_traits: [trait_id, ...],
  
  // Technology (System 19)
  discovered_technologies: [tech_id, ...],
  actively_practicing: [tech_id, ...],
  
  // Demographics (System 20)
  total_population: int,
  birth_rate_per_tick: float,
  death_rate_per_tick: float,
}
```

### Minimal Companion Records

For efficiency, some data is stored in separate arrays indexed by entity ID:

```
// Minimal companion records (lookup by entity_id)
MemoryCaches {
  [agent_id]: {
    cached_pathfind_route: [location, ...],
    cached_pathfind_destination: location,
    cached_pathfind_tick: int,
  }
}

PerformanceStats {
  per_system_tick_budget: {
    [system_name: string]: {
      average_tick_time_ms: float,
      peak_tick_time_ms: float,
      miss_count: int,  // How many ticks exceeded budget
    }
  }
}
```

---

## 4. Starter State & Scenario Format

Modders and scenario designers can create custom initial worlds:

```
ScenarioFile {
  format_version: "2.2.0",
  name: string,
  description: string,
  
  // Initial state
  map_configuration: {
    width: int,
    height: int,
    terrain_seed: int,  // Procedurally generate terrain or specify manually
    biome_placement: [
      { location: (x, y), terrain_type: enum, ... },
      ...
    ]
  },
  
  // Starting factions
  initial_factions: [
    {
      name: string,
      starting_settlement_location: (x, y),
      starting_population_count: int,
      government_type: enum,
      initial_technologies: [tech_id, ...],
      initial_cultural_traits: [trait_id, ...],
      initial_language_id: int,
    }
  ],
  
  // Starting creatures (species definitions + initial populations)
  initial_creature_species: [
    {
      species_id: int,
      name: string,
      starting_populations: [
        { location: (x, y), population_count: int },
        ...
      ],
      starting_genotype_profile: GenotypeTemplate,  // Average phenotype
    }
  ],
  
  // Starting technologies & knowledge
  initial_technologies: [Technology, ...],
  initial_knowledge_facts: [KnowledgeFact, ...],
  
  // Optional: Custom rules/modifications
  rule_modifications: {
    allow_player_avatar_creation: bool,
    initial_permadeath_mode: enum { Sandbox, Hardcore, Ironman },
  }
}
```

---

## 5. Serialization Update Rules

### Checkpoint & Delta Journal

```
function tick_and_checkpoint(world: World, tick: int):
  // Execute one tick of all systems
  tick_evolution(world)
  tick_ecology(world)
  tick_agents(world)
  tick_settlements(world)
  // ... all other systems ...
  
  world.current_tick = tick
  
  // Save checkpoint every N ticks (e.g., every 100 ticks)
  if tick % CHECKPOINT_INTERVAL == 0:
    checkpoint_world_state(world)

function checkpoint_world_state(world: World):
  save_file = serialize_world_to_save_file(world)
  save_file.save_checksum = compute_crc32(save_file.entities)
  
  # Compress and write to disk
  compressed = zstd_compress(json_serialize(save_file))
  write_to_disk("saves/world_tick_" + world.current_tick + ".sav", compressed)
  
  # Optionally: keep delta journal for crash recovery
  delta_journal_entry = {
    checkpoint_tick: world.current_tick,
    previous_checkpoint_tick: world.last_checkpoint_tick,
    delta_operations: [
      {
        operation_type: enum { Create, Update, Delete },
        entity_id: int,
        entity_data: Entity or null,  // null for Delete
      }
    ]
  }
  append_to_journal("saves/delta_journal.log", delta_journal_entry)

function load_world_from_save(save_path: string) -> World:
  compressed_data = read_from_disk(save_path)
  save_file = json_deserialize(zstd_decompress(compressed_data))
  
  # Validate schema version and apply migrations
  if save_file.format_version != CURRENT_SCHEMA_VERSION:
    save_file = apply_schema_migrations(save_file, CURRENT_SCHEMA_VERSION)
  
  # Validate checksum
  if compute_crc32(save_file.entities) != save_file.save_checksum:
    log_warning("Save file checksum mismatch; file may be corrupted")
  
  # Reconstruct world from save file
  world = World()
  world.current_tick = save_file.current_tick
  world.world_seed = save_file.world_seed
  world.rng_state = save_file.rng_state
  
  # Deserialize entities
  for agent in save_file.entities.agents:
    world.agents[agent.entity_id] = agent
  for creature in save_file.entities.creatures:
    world.creatures[creature.entity_id] = creature
  # ... and so on for all entity types ...
  
  return world
```

### Schema Migration Example

```
function apply_schema_migrations(save_file: SaveFile, target_version: string) -> SaveFile:
  current_version = save_file.format_version
  
  migrations = [
    ("2.0.0", "2.0.1", migrate_2_0_0_to_2_0_1),
    ("2.0.1", "2.1.0", migrate_2_0_1_to_2_1_0),
    ("2.1.0", "2.2.0", migrate_2_1_0_to_2_2_0),
  ]
  
  for (from_ver, to_ver, migration_func) in migrations:
    if current_version == from_ver:
      save_file = migration_func(save_file)
      current_version = to_ver
      log("Migrated from " + from_ver + " to " + to_ver)
  
  save_file.format_version = target_version
  return save_file

// Example migration: 2.0.0 → 2.0.1 added Agent.lineage_id field
function migrate_2_0_0_to_2_0_1(save_file: SaveFile) -> SaveFile:
  for agent in save_file.entities.agents:
    if agent.lineage_id == null:
      agent.lineage_id = generate_new_lineage_id()  // Default for old agents
  return save_file
```

### Deterministic Replay

```
function record_input_event(tick: int, input: PlayerInput):
  # Log all player actions with tick timestamp
  input_journal_entry = {
    tick: tick,
    player_id: input.player_id,
    action_type: input.action_type,
    action_target: input.target_id,
    action_parameters: input.params,
  }
  append_to_journal("saves/input_journal.log", input_journal_entry)

function replay_world(world_save_path: string, input_journal_path: string):
  # Load initial world state
  world = load_world_from_save(world_save_path)
  
  # Read input journal
  input_events = read_input_journal(input_journal_path)
  
  # Replay: reseed RNG, execute ticks, apply inputs at correct ticks
  world.rng_state = world.rng_state_at_save  // Restore RNG state from save
  
  for tick in range(world.current_tick, world.current_tick + N_REPLAY_TICKS):
    # Apply any inputs scheduled for this tick
    for input_event in input_events:
      if input_event.tick == tick:
        apply_player_input(world, input_event)
    
    # Tick simulation
    tick_and_checkpoint(world, tick)
    
    # Verify: check that replay matches recorded state
    if world.compute_state_hash() != recorded_state_hash[tick]:
      log_error("Replay diverged at tick " + tick)
      break
```

### SaveValidator: Schema Validation & Forbidden-Key Rejection

SaveValidator ensures save files conform to the canonical schema and cannot contain UI-ephemeral state that would break determinism on replay.

```rust
pub struct SaveValidator {
    schema: jsonschema::JSONSchema,
    forbidden_keys: HashSet<&'static str>,
}

impl SaveValidator {
    pub fn new() -> Result<Self> {
        // Load canonical save file schema (JSON Schema draft 2020-12)
        let schema_str = include_str!("../schemas/savefile.schema.json");
        let schema_json: serde_json::Value = serde_json::from_str(schema_str)?;
        let schema = jsonschema::JSONSchema::compile(&schema_json)?;
        
        // Forbidden keys that would corrupt deterministic replay
        let forbidden_keys = [
            "bestiary_discovered",  // Derived from observation_count; never serialized
            "ui_camera_position",
            "ui_open_tabs",
            "ui_filter_selections",
            "ui_scroll_position",
            "player_custom_notes",
        ].into_iter().collect();
        
        Ok(SaveValidator { schema, forbidden_keys })
    }
    
    /// Validate save file JSON against schema and check for forbidden keys
    pub fn validate(&self, json: &serde_json::Value) -> Result<()> {
        // 1. Validate against schema
        self.schema.validate(json).map_err(|e| {
            anyhow!("Save file schema validation failed: {}", e)
        })?;
        
        // 2. Check for forbidden keys (deep traversal)
        self.check_forbidden_keys(json, "")?;
        
        Ok(())
    }
    
    fn check_forbidden_keys(&self, value: &serde_json::Value, path: &str) -> Result<()> {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map.iter() {
                    // Check exact match (e.g., "bestiary_discovered")
                    if self.forbidden_keys.contains(key.as_str()) {
                        return Err(anyhow!(
                            "Forbidden key '{}' found at path '{}'",
                            key, path
                        ));
                    }
                    
                    // Check wildcard patterns (e.g., "ui_*")
                    if key.starts_with("ui_") {
                        return Err(anyhow!(
                            "Forbidden UI-state key '{}' found at path '{}'; UI state must not be serialized",
                            key, path
                        ));
                    }
                    
                    // Recursively check nested objects
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    self.check_forbidden_keys(val, &new_path)?;
                }
                Ok(())
            }
            serde_json::Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, i);
                    self.check_forbidden_keys(val, &new_path)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

pub fn load_game(path: &Path) -> Result<Simulation> {
    // 1. Read file
    let json_str = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&json_str)?;
    
    // 2. Validate with SaveValidator
    let validator = SaveValidator::new()?;
    validator.validate(&json)?;
    
    // 3. Deserialize
    let save_file: SaveFile = serde_json::from_value(json)?;
    
    // 4. Reconstruct simulation
    let sim = Simulation::from_save_file(save_file)?;
    Ok(sim)
}
```

**Test Requirement**: Hand-crafted invalid save file with forbidden keys must be rejected:

```rust
#[test]
fn test_savevalidator_rejects_forbidden_keys() {
    let invalid_json = serde_json::json!({
        "format_version": "2.2.0",
        "entities": { "creatures": [] },
        "bestiary_discovered": true,  // FORBIDDEN
    });
    
    let validator = SaveValidator::new().unwrap();
    assert!(validator.validate(&invalid_json).is_err());
    assert!(validator.validate(&invalid_json)
        .unwrap_err()
        .to_string()
        .contains("bestiary_discovered"));
}

#[test]
fn test_savevalidator_rejects_ui_keys() {
    let invalid_json = serde_json::json!({
        "format_version": "2.2.0",
        "entities": { "creatures": [] },
        "ui_camera_position": {"x": 100, "y": 200},  // FORBIDDEN
    });
    
    let validator = SaveValidator::new().unwrap();
    assert!(validator.validate(&invalid_json).is_err());
}

#[test]
fn test_savevalidator_accepts_valid_save() {
    let valid_json = serde_json::json!({
        "format_version": "2.2.0",
        "game_version": "1.0.0",
        "current_tick": 1000,
        "world_seed": 12345,
        "entities": {
            "creatures": [],
            "agents": [],
            "settlements": []
        },
        "rng_state": {
            "evolution_stream": [0, 0, 0, 0],
            "ecology_stream": [0, 0, 0, 0],
        }
    });
    
    let validator = SaveValidator::new().unwrap();
    assert!(validator.validate(&valid_json).is_ok());
}
```

---

## 6. Performance Budget Allocation

```
GlobalTickBudget {
  total_budget_ms: 16.0,  // 60 FPS
  
  per_system_budget: {
    "Evolution":           { budget_ms: 2.0, priority: 10, cooldown_ticks: 1 },
    "Ecology":             { budget_ms: 2.5, priority: 9, cooldown_ticks: 1 },
    "Agents":              { budget_ms: 4.0, priority: 8, cooldown_ticks: 1 },
    "Settlements":         { budget_ms: 2.0, priority: 7, cooldown_ticks: 5 },
    "Combat":              { budget_ms: 2.0, priority: 8, cooldown_ticks: 1 },
    "Crafting":            { budget_ms: 1.5, priority: 6, cooldown_ticks: 5 },
    "Climate":             { budget_ms: 0.5, priority: 4, cooldown_ticks: 100 },
    "Language Drift":      { budget_ms: 0.3, priority: 3, cooldown_ticks: 100 },
    "Technology Discovery": { budget_ms: 0.5, priority: 5, cooldown_ticks: 10 },
    "Migration":           { budget_ms: 1.0, priority: 7, cooldown_ticks: 10 },
    "Pathfinding":         { budget_ms: 1.5, priority: 6, cooldown_ticks: 5 },
    "Disease":             { budget_ms: 0.3, priority: 5, cooldown_ticks: 10 },
  },
}

function tick_all_systems(world: World):
  budget_remaining = 16.0  // ms
  systems_to_run = []
  
  for (system_name, system_config) in GlobalTickBudget.per_system_budget:
    if world.current_tick % system_config.cooldown_ticks == 0:
      systems_to_run.append((system_name, system_config.budget_ms, system_config.priority))
  
  # Sort by priority (descending)
  systems_to_run.sort(key=lambda s: s[2], reverse=true)
  
  for (system_name, budget_ms, priority) in systems_to_run:
    tick_start_time = get_time_ms()
    
    # Run system
    execute_system(world, system_name)
    
    tick_elapsed_ms = get_time_ms() - tick_start_time
    budget_remaining -= tick_elapsed_ms
    
    # Log if over budget
    if tick_elapsed_ms > budget_ms:
      log_warning(system_name + " exceeded budget: " + tick_elapsed_ms + " ms > " + budget_ms + " ms")
    
    # Stop if global budget exhausted
    if budget_remaining < 0:
      log_warning("Global tick budget exhausted; deferring " + (systems_to_run.length - systems_to_run.index - 1) + " systems")
      break
```

---

## 7. Cross-System Hooks

All systems depend on **System 22** for serialization:
- **Save/Load**: Every system stores its state in the SaveFile structure. Load restores it.
- **Determinism**: Every system uses the global RNG (seeded with world_seed) for any randomness.
- **Modularity**: Each system's component arrays are independent; adding a new system requires adding new component arrays (no refactoring of existing systems).

---

## 8. Tradeoff Matrix

| Dimension | Choice | Rationale |
|---|---|---|
| **Serialization Format** | JSON vs. binary | JSON is human-readable and debuggable; binary is compact. Chosen: JSON + compression (readable but compact). |
| **Checkpoint Frequency** | Every tick vs. every 100 ticks | Every tick is safe but slow; every 100 ticks is faster but riskier (100 ticks of progress lost on crash). Chosen: every 100 ticks (balance). |
| **Delta Journal** | Enabled vs. disabled | Enables fast incremental saves and crash recovery; disables saves space. Chosen: enabled (important for stability). |
| **Schema Versioning** | Strict (code version must match save version) vs. flexible (auto-migrate) | Strict is safer; flexible is forgiving. Chosen: flexible (auto-migrate with warnings). |
| **RNG Seeding** | Mersenne Twister vs. cryptographic RNG | MT is fast; crypto RNG is secure. Chosen: MT (sufficient for simulation). |
| **Determinism Verification** | Runtime replay checking vs. offline replay | Runtime is expensive; offline is useful for debugging. Chosen: offline replay mode (optional, for dev). |

---

## 9. Open Calibration Knobs

- **CHECKPOINT_INTERVAL**: Ticks between automatic saves (currently 100). Increase to reduce save overhead; decrease for more frequent checkpoints.

- **GLOBAL_TICK_BUDGET**: Total milliseconds per tick (currently 16.0 for 60 FPS). Increase to allow slower systems to run more; decrease for faster simulation.

- **SYSTEM_PRIORITIES**: Relative importance of systems. Increase priority for frequently-needed systems; decrease for background systems.

- **SYSTEM_COOLDOWN_TICKS**: How often each system runs (currently 1–100 ticks depending on system). Decrease for more frequent updates; increase to batch updates.

- **SAVE_COMPRESSION_LEVEL**: zstd compression level (currently default). Increase for smaller files; decrease for faster save/load.

- **DELTA_JOURNAL_ENABLED**: Whether to log delta changes (currently true). Disable if storage is limited.

- **CRASH_RECOVERY_ENABLED**: Whether to maintain a recovery journal (currently true). Disable to reduce I/O overhead.

