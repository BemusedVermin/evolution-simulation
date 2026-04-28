# Beast Evolution Game: ECS System Schedule & Execution Model

## Overview

The simulation executes in **8 ordered stages per tick**. Systems within a stage run in parallel (via rayon). Systems in different stages run sequentially. This design balances performance, determinism, and data consistency.

**Tick Duration**: ~16ms at 60 FPS; systems with budget overruns are deferred to next tick.

---

## System Schedule (Sequential Stages)

```
┌─────────────────────────────────────────────────────────────┐
│ TICK N BEGINS                                               │
└─────────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 0: INPUT & AGING                                  │
 │ (Serial: 1-2 systems)                                   │
 ├─────────────────────────────────────────────────────────┤
 │ • InputSystem (player avatar movement, breeding commands)
 │ • RandomEventSystem (migrations, plagues, resource booms)
 │ • AgingSystem (increment creature age, developmental transitions)
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 1: GENETICS + COGNITIVE PERCEPTION                │
 │ (Parallel: per-creature)                                │
 ├─────────────────────────────────────────────────────────┤
 │ • MutationSystem (point mutations, regulatory rewiring)
 │ • CognitivePerceptionSystem (per agent, doc 57):
 │     - VMP perceptual update on factor graph
 │     - ToM cache update for visible co-agents
 │ • CognitivePlanningSystem (off-cadence, doc 57):
 │     - MCTS-EFE expansion every PLANNING_CADENCE ticks
 │ • Note: ChannelGenesisSystem moved to Stage 7 per doc 58
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 2: PHENOTYPE RESOLUTION                           │
 │ (Parallel: per-creature interpretation)                 │
 ├─────────────────────────────────────────────────────────┤
 │ Sub-Stage 2a: ScaleBandFilterSystem (gate channels first)
 │ • Dormant channels output Q32.32::ZERO
 │ • Filter by creature scale_band before composition
 │
 │ Sub-Stage 2b: Composition Hooks & Interpretation
 │ • InterpreterSystem (genotype → primitive effects)
 │ • CompositionHooksSystem (evaluate hooks, emit primitives)
 │ • Composition hooks receiving zero from any operand → zero output
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 3: PHYSICS & MOVEMENT                             │
 │ (Parallel: per-creature)                                │
 ├─────────────────────────────────────────────────────────┤
 │ • PhysicsSystem (forces, collision detection)
 │ • MovementResolveSystem (pathfinding, position updates)
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 4: INTERACTION & COMBAT                           │
 │ (Parallel: per-creature-pair in spatial grid)           │
 ├─────────────────────────────────────────────────────────┤
 │ • ActionSamplingSystem (doc 57):
 │     - sample action tuple from policy_posterior
 │     - dispatch by primitive target_kind: self / broadcast / pair / group
 │ • RelationshipEdgeUpdateSystem (doc 56):
 │     - pair primitives → write into RelationshipEdge accum + ring buffer
 │ • LatentPressureSystem (doc 58):
 │     - per-agent EFE-residual → latent_slot accumulation
 │ • CombatResolutionSystem (melee, ranged, abilities)
 │     - subsumed: combat actions are policies sampled from agent.gm
 │ • PreyPredatorSystem (predation, herbivory checks)
 │ • ParasitismSystem (pathogen transmission, host damage)
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 5: PHYSIOLOGY                                     │
 │ (Parallel: per-creature)                                │
 ├─────────────────────────────────────────────────────────┤
 │ • MetabolismSystem (energy consumption, temp regulation)
 │ • InjuryHealingSystem (recovery from wounds)
 │ • ReproductionCycleSystem (fertility checks, spawning)
 │ • DeathCheckSystem (starvation, old age, severe injury)
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 6: ECOLOGY                                        │
 │ (Parallel: per-biome cell or per-species)               │
 ├─────────────────────────────────────────────────────────┤
 │ • PopulationDynamicsSystem (carrying capacity, migration)
 │ • BiomeEffectsSystem (hazards, resource regrowth, climate)
 │   See System 15 (Climate, Biome & Geology)
 │ • SpeciationSystem (reproductive isolation checks)
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 7: LABELING, EMERGENCE & PERSISTENCE              │
 │ (Serial or sampled, multiple registered cadences)       │
 ├─────────────────────────────────────────────────────────┤
 │ • EdgeDecaySystem (every D_EDGE ticks, doc 56):
 │     - apply lambda decay to RelationshipEdge.accum
 │     - GC fully-decayed edges
 │ • CommunityDetectionSystem (doc 56):
 │     - LinkCommunitySystem (every CADENCE_LINK ticks)
 │     - HierarchicalLeidenSystem (per-level cadences L0..L3)
 │     - ClusterCharacterizationSystem
 │ • ChannelGenesisSystem (every CADENCE_GENESIS ticks, doc 58):
 │     - bucket latent slots, run 3 proposal mechanisms,
 │       register winners with genesis: provenance,
 │       seed P6a propagation, GC retired channels
 │ • ChroniclerSystem (every N ticks):
 │     - pattern detection over primitive emissions
 │     - 1-NN labelling against etic galleries (doc 56 §5.5)
 │     - cluster-feature labelling (biomes, governance, …)
 │ • NamingSystem (doc 70):
 │     - phonotactic-gen for new cluster signatures
 │     - P6a iterated-learning lexical pass
 │ • SaveCheckpointSystem (every M ticks: write to disk)
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ Stage 8: RENDER PREP                                    │
 │ (Serial: snapshot creation)                             │
 ├─────────────────────────────────────────────────────────┤
 │ • RenderPrepSystem (copy immutable snapshot for render)
 └─────────────────────────────────────────────────────────┘
      ↓
 ┌─────────────────────────────────────────────────────────┐
 │ TICK N ENDS                                             │
 │ tick_counter += 1                                       │
 │ Render frame with snapshot                              │
 └─────────────────────────────────────────────────────────┘
```

---

## Stage Details

### Stage 0: Input & Aging

**Systems**:
1. `InputSystem` — Non-deterministic (reads user input from SDL3)
2. `RandomEventSystem` — Deterministic (RNG seeded from world_seed + tick)
3. `AgingSystem` — Deterministic

**Components Read**: Creature, Agent, Position
**Components Write**: Position (input movement), Age, DevelopmentalStage, HealthState (events)

**Parallelism**: Serial. Input is single-threaded; random events are processed in order.

**Notes**:
- Player avatar movement is logged in ReplayJournal for determinism testing
- Random events (plague outbreaks, migration waves) use `rng_input` stream
- Developmental stage transitions (juvenile → adult) checked here

---

### Stage 1: Genetics + Cognitive Perception

**Systems**:
1. `MutationSystem` — Mutate all creatures' genomes
2. `CognitivePerceptionSystem` (doc 57) — VMP perceptual update on per-agent factor graph; ToM cache update
3. `CognitivePlanningSystem` (doc 57) — MCTS-EFE policy posterior, off-cadence (every `PLANNING_CADENCE` ticks per agent)

**Components Read**: Genome (mutation), GenerativeModelState (cognition), Carriers, RelationshipEdge (read for ToM input)
**Components Write**: Genome, GenerativeModelState.beliefs, GenerativeModelState.tom_cache, GenerativeModelState.policy_posterior

**Parallelism**: Parallel per-creature (no inter-creature dependencies during perception)

**RNG Streams**: `rng_evolution` (mutation), `rng_belief` (VMP if randomised init), `rng_policy` (MCTS UCT)

**Notes**:
- Each creature's genome mutated independently
- Mutation rates: point ~1e-3, duplication ~5e-5, reclassification ~1e-5
- Cognitive perception: VMP iterates the agent's factor graph; bounded by `VMP_MAX_ITER` (default 8)
- Cognitive planning: MCTS expansions per agent capped by `cognitive_budget` (function of `neural_speed × resting_metabolic_rate`)
- ChannelGenesisSystem moved to Stage 7 (doc 58) — its trigger is per-population, not per-creature, and it runs at low cadence
- Duplication-only genesis events from MutationSystem still mark `Genome.provenance` as `genesis:parent_id:generation`; full channel-genesis events from doc 58 use the four-component form `genesis:<src_pop>:<tick>:<kind>:<sig_hash>`

---

### Stage 2: Phenotype Resolution

**Systems** (in order):
1. `ScaleBandFilterSystem` — Gate channels by scale_band constraint (FIRST)
2. `InterpreterSystem` — Genotype → primitive effect set
3. `CompositionHooksSystem` — Evaluate composition_hooks, emit primitives

**Components Read**: Genome, Environment (biome, season, creature_mass_kg), Mass
**Components Write**: Phenotype

**Parallelism**: Parallel per-creature

**Determinism**: Fully deterministic (fixed-point math, no randomness)

**Sub-Stage 2a: ScaleBandFilterSystem (Execution First)**

This system runs BEFORE composition hooks to filter channels that are dormant at this creature's scale:

```
For each creature C:
  creature_mass_kg = C.compute_mass_kg()
  For each channel ch in genome.channel_values:
    (min_kg, max_kg) = channel_registry[ch].scale_band
    if creature_mass_kg < min_kg OR creature_mass_kg > max_kg:
      // Dormant channel — set to Q32.32::ZERO
      filtered_channel_values[ch] = Q32.32::ZERO
    else:
      filtered_channel_values[ch] = genome.channel_values[ch]
```

**Output**: Filtered channel values that composition hooks will read. Dormant channels are **not** removed from the genome; they are simply clamped to zero during evaluation.

**Test Fixture Requirement**: A macro creature (e.g., 500kg) carrying a micro-only channel (scale_band = [1e-15kg, 1e-3kg]) must produce **zero primitives** for that channel. Composition hooks receiving zero from any operand must produce zero output (total evaluation, no errors).

**Notes**:
- Interpreter reads filtered channel_values (post-scale-band clamping)
- Evaluates expression_conditions (biome, season, developmental stage gates)
- Applies composition_hooks in sorted order (by hook ID)
- Parameter mapping evaluated via fixed-point expressions
- Output: Phenotype.primitive_effects (set of PrimitiveEffect IDs)
- **No errors on dormant channels**: Q32.32::ZERO is a valid output, not an error state

---

### Stage 3: Physics & Movement

**Systems**:
1. `PhysicsSystem` — Apply forces from primitive effects
2. `MovementResolveSystem` — Pathfinding, collision, position update

**Components Read**: Phenotype (primitive_effects), Position, Velocity
**Components Write**: Position, Velocity, CollisionState

**Parallelism**: Parallel per-creature (with spatial partitioning to avoid false positives)

**RNG Stream**: `rng_physics` (for stochastic collision outcomes)

**Notes**:
- Primitive effects with category `force_application` contribute to velocity
- Movement via pathfinding (A* with hierarchical nav mesh) or flocking
- Collision detection via spatial index (rstar R*-tree)
- Position updated at end of stage; no position-position read-write hazards within stage

---

### Stage 4: Interaction & Combat

**Systems**:
1. `CombatResolutionSystem` — Resolve creature-creature combat
2. `PreyPredatorSystem` — Herbivory, predation checks
3. `ParasitismSystem` — Pathogen transmission, host damage

**Components Read**: Phenotype, HealthState, Position, Genome
**Components Write**: HealthState, Injury, ThreatAssessment

**Parallelism**: Parallel per-creature-pair (spatial locality)

**RNG Stream**: `rng_combat`

**Notes**:
- Combat derived entirely from primitive effects (force_application, state_induction)
- No hardcoded damage formulas; force primitive parameter determines damage
- Predation: prey detection via `receive_acoustic_signal` + `signal_emission` primitives
- Parasitism: pathogen primitive effects determine transmission probability and virulence

**Combat Resolution**:
```
For each creature C1:
  Find nearby creatures C2 via spatial index
  For each C2 in engagement:
    Compute offense (C1 primitive effects)
    Compute defense (C2 primitive effects)
    Compute damage: (offense_force * (1 - defense_rigidity)) saturated
    Apply damage to C2.health
    C2.injuries updated
```

---

### Stage 5: Physiology

**Systems**:
1. `MetabolismSystem` — Energy consumption, thermoregulation
2. `InjuryHealingSystem` — Passive healing, scar formation
3. `ReproductionCycleSystem` — Fertility checks, offspring spawning
4. `DeathCheckSystem` — Starvation, old age, severe injury

**Components Read**: Phenotype (energy_modulation primitives), HealthState, Age, Genome
**Components Write**: HealthState (energy, health), Age, Offspring (spawned entities)

**Parallelism**: Parallel per-creature

**RNG Stream**: `rng_evolution` (offspring mutations)

**Notes**:
- Energy consumption: `energy_modulation` primitive `elevate_metabolic_rate` costs energy
- Thermoregulation: `state_induction` primitive `thermoregulate_self` prevents temperature extremes
- Reproduction: creatures with health > 0.7 && energy > 0.8 && age > min_breeding_age spawn 1–3 offspring
- Offspring mutation: inherited genome + mutations from `rng_evolution`
- Death: health <= 0 OR age > max_lifespan OR energy <= 0 for 3 consecutive ticks

---

### Stage 6: Ecology

**Systems**:
1. `PopulationDynamicsSystem` — Carrying capacity, migration, extinction
2. `BiomeEffectsSystem` — Resource regrowth, hazards, climate shifts
3. `SpeciationSystem` — Reproductive isolation (rare, asynchronous)

**Components Read**: Creature, Biome, Species
**Components Write**: Species membership (rare), Creature (extinction)

**Parallelism**: Parallel per-species or per-biome-cell

**RNG Stream**: `rng_ecology`

**Notes**:
- Carrying capacity: per-biome-cell, limits population size (excess creatures removed randomly)
- Migration: source-sink metapopulation (creatures migrate from high-fitness to low-fitness biomes)
- Extinction: species with <5 individuals in world removed with increasing probability each tick
- Speciation: rare (1e-6 per generation) event when reproductive isolation accumulates
- Biome hazards: volcanic eruption, wildfire, flooding — damage random creatures (low frequency, high impact)

**BiomeEffectsSystem Sub-Details** (See System 15 for full specification):
- **BiomeCell Component**: Each biome cell carries `resource_density`, `hazard_intensity`, `climate_state`, `season`
- **Resource Regrowth**: Vegetation regeneration driven by NPP (Net Primary Productivity) based on temperature + precipitation (System 15 climate model)
- **Seasonal Modifiers**: Season affects carrying capacity and resource availability; creatures express seasonal channels (winter coat, summer behavior)
- **Channel Fitness Modifiers**: `channel_fitness_modifiers` per cell (e.g., cold_resistance bonus in tundra) are read by evolution system during fitness evaluation
- **Hazard Application**: Rare hazard events (1% per tick at volcanic zones) damage random creatures via damage primitives or starvation
- **Climate Shifts**: Over geological timescales, plate drift and Milankovitch cycles gradually shift biome boundaries; species must evolve or migrate

---

### Stage 7: Labeling, Emergence & Persistence

**Systems** (each at its own registered cadence):
1. `EdgeDecaySystem` (doc 56) — apply Q32.32 decay to `RelationshipEdge.accum`; GC fully-decayed edges
2. `LinkCommunitySystem` (doc 56) — edge-first link communities (Ahn 2010 / Pelka & Skoulakis 2025); produces overlapping vertex memberships
3. `HierarchicalLeidenSystem` (doc 56) — recursive Leiden levels L0..L3 with per-level cadences
4. `ClusterCharacterizationSystem` (doc 56) — feature vectors over detected clusters (size, density, persistence, edge-type composition, internal hierarchy index, spatial extent, overlap fraction)
5. `LatentBucketingSystem` (doc 58) — cluster per-agent latent slots by signature similarity; identify candidate buckets
6. `ChannelGenesisSystem` (doc 58) — three proposal mechanisms (composition, latent extraction, mutation); QD-archive admission; register winners with `genesis:` provenance
7. `ChroniclerSystem` — etic 1-NN gallery match for cluster signatures (biomes, governance archetypes, edge-cluster labels, node-cluster labels); produces *layer-5* candidate names for the P7 chain
8. `NamingSystem` (doc 70) — phonotactic-gen seeded by `(cluster_sig, lang_id, world_seed)`; P6a iterated-learning lexical-transmission op; emits NamingEvents and processes player-typed alias inputs from the input log
9. `SaveCheckpointSystem` — checkpoint to disk

**Components Read**: All (immutable snapshot for labelling); RelationshipEdge (decay/community-detection); LatentSlotBuffer per agent
**Components Write**: RelationshipEdge.accum (decay), GroupClusters (derived; not saved), `genesis_registry`, `genesis_event_log`, `population.lexicon`

**Parallelism**: Serial within Stage 7 (Stage 7 carries cross-cutting state); each substage is internally parallelisable per-edge or per-agent or per-cluster as noted in doc 56/57/58

**Determinism**: All substages deterministic (Q32.32, sorted-id iteration, named PRNG streams: `rng_link`, `rng_leiden`, `rng_genesis`, `rng_naming`)

**Cadences (defaults — registered, mod-tunable)**:
| System | Cadence (ticks) |
|---|---|
| EdgeDecay | every `D_EDGE` (default 8) |
| LinkCommunity | `CADENCE_LINK` (64) |
| Leiden L0 | `CADENCE_LEIDEN_L0` (64) |
| Leiden L1 | `CADENCE_LEIDEN_L1` (256) |
| Leiden L2 | `CADENCE_LEIDEN_L2` (1024) |
| Leiden L3 | `CADENCE_LEIDEN_L3` (4096) |
| ClusterCharacterization | `CADENCE_CHARACTERIZE` (256) |
| ChannelGenesis | `CADENCE_GENESIS` (1024) |
| Chronicler | every 100 (existing) |
| Naming | every 60 (per doc 70 §3) |
| SaveCheckpoint | every 1000 |

**Notes**:
- Group memberships, containment, and edge-cluster types are recomputed every Stage 7; per Invariant 7 they are NOT saved.
- `genesis_registry` and `genesis_event_log` ARE saved (sim state, per Invariant 10).
- Chronicler 1-NN labels are *etic* candidate names (P7 layer 5) — they enter the bestiary alongside player aliases, loan-words from contact populations, and phonotactic-gen names; the literal `"uncategorised"` / `"unlabelled"` strings are never shown in player view.
- The naming pass is Q32.32-deterministic via the dedicated `rng_naming` stream and seeded phonotactic n-gram (doc 70 §4.3).

---

### Stage 8: Render Prep

**Systems**:
1. `RenderPrepSystem` — Copy immutable snapshot

**Components Read**: All (immutable)
**Components Write**: RenderQueue (not in ECS; lives in Renderer)

**Parallelism**: Serial snapshot creation

**Determinism**: Fully deterministic (pure read, deterministic iteration order)

**Notes**:
- Snapshot includes: entity positions, health, phenotype, visual directives
- Snapshot is passed to Renderer (off-main thread or main-thread-single-tasking)
- Rendering does NOT mutate sim state
- Snapshot stale after Stage 8; next tick invalidates it

---

## Parallelism & Determinism Rules

### Rule 1: Stage Isolation
Systems in different stages never execute simultaneously. A stage completes before the next begins.

### Rule 2: Per-Stage Parallelism
Within a stage, systems that operate on independent entities can run in parallel. Locks are used only to protect shared resources (registries, PRNG state).

**Safe Parallel Systems**:
- Per-creature mutations (no inter-creature dependencies)
- Per-creature phenotype interpretation
- Per-creature physics (with spatial grid partitioning)
- Per-creature metabolism

**Unsafe Parallel Systems** (run serial):
- Input handling (reads user input; serial by necessity)
- Random events (could be parallel but logically serial)
- Global statistics (chronicler, population counts)

### Rule 3: Sorted Iteration
All entity loops iterate in sorted EntityID order:
```rust
let entities = resources.sorted_entity_index.creatures();
for entity_id in entities {  // Ascending order
    // Process this entity
}
```

### Rule 4: PRNG Stream Isolation
Each subsystem has one PRNG stream (Xoshiro256PlusPlus). Streams never cross-contaminate:
```rust
// In Mutation System
let delta = sample_gaussian(&mut resources.rng_evolution, 0.0, 0.1);

// In Combat System (different stream)
let damage = (force * (1.0 - defense)).saturating_mul(resources.rng_combat.next_float());
```

### Rule 5: No Wall-Clock Timing
Timing decisions use tick_counter only:
```rust
// BAD (wall-clock dependent, non-deterministic):
let elapsed = std::time::Instant::now().elapsed();
if elapsed > 1000ms { ... }

// GOOD (tick-count dependent, deterministic):
if resources.tick_counter % 100 == 0 { ... }
```

---

## Performance Budget Allocation

**Total per-tick budget**: 16ms (60 FPS)

| Stage | System | Budget | Notes |
|-------|--------|--------|-------|
| 0 | Input | 1ms | Non-critical; overruns acceptable |
| 0 | Random Events | 1ms | Few events per tick |
| 1 | Mutation | 1ms | O(N creatures × genes) |
| 2 | Interpreter | 3ms | Hot path; fixed-point arithmetic optimized |
| 2 | Scale-Band Filter | 0.5ms | Trivial (conditional checks) |
| 2 | Composition Hooks | 2ms | Complex math; bottleneck for deep evolution |
| 3 | Physics | 2ms | Spatial grid partitioning |
| 3 | Movement | 1ms | Pathfinding cached per 10 ticks |
| 4 | Combat | 2ms | Spatial locality reduces checks |
| 4 | Predation | 0.5ms | Detection range limited |
| 4 | Parasitism | 0.5ms | Few pathogen interactions per tick |
| 5 | Metabolism | 1ms | Per-creature energy updates |
| 5 | Healing | 0.5ms | Passive (no RNG) |
| 5 | Reproduction | 1ms | Offspring spawn, mutations |
| 5 | Death | 0.5ms | Removal checks |
| 6 | Population Dynamics | 1ms | Per-species carrying capacity |
| 6 | Biome Effects | 0.5ms | Rare events |
| 6 | Speciation | 0.1ms | Very rare |
| 7 | Chronicler | 0ms (every 100 ticks) | Amortized to 0.1ms/tick |
| 7 | Save Checkpoint | 0ms (every 1000 ticks) | Amortized to 0.01ms/tick |
| 8 | Render Prep | 0.5ms | Snapshot creation |
| **Total** | | ~16ms | Tight; cold systems deferred |

**Adaptive Quality**: If a stage exceeds budget, subsequent ticks skip "cold" systems (low priority):
- Deferred to every 10 ticks: pathfinding (distant NPCs), far-biome events
- Deferred to every 100 ticks: chronicler pattern detection
- Deferred to every 1000 ticks: save checkpoints

---

## System Implementation Template

All systems follow this pattern:

```rust
pub struct MySystem {
    name: &'static str,
    stage: SystemStage,
    budget_ms: u32,
}

impl System for MySystem {
    fn run(&mut self, world: &World, resources: &mut Resources) -> Result<()> {
        let start = std::time::Instant::now();
        
        // Get storage
        let mut storage = world.write_storage::<MyComponent>();
        let other_storage = world.read_storage::<OtherComponent>();
        
        // Iterate in sorted order
        let entities = resources.sorted_entity_index.all_entities();
        for entity_id in entities {
            if let Ok(mut data) = storage.get_mut(entity_id) {
                // Deterministic computation (no floats, use Q32.32)
                // Use subsystem RNG if needed (e.g., rng_evolution)
                self.process_entity(&mut data, &other_storage)?;
            }
        }
        
        let elapsed = start.elapsed().as_millis() as u32;
        if elapsed > self.budget_ms {
            warn!("{} exceeded budget: {}ms > {}ms", self.name, elapsed, self.budget_ms);
        }
        
        Ok(())
    }
    
    fn stage(&self) -> SystemStage { self.stage }
    fn name(&self) -> &str { self.name }
}

impl MySystem {
    fn process_entity(&self, data: &mut MyComponent, other: &ReadStorage) -> Result<()> {
        // Your logic here
        Ok(())
    }
}
```

---

## Debugging & Profiling

**Per-Tick Profiling**:
```rust
pub struct TickProfiler {
    stage_times: HashMap<SystemStage, Duration>,
}

impl TickProfiler {
    pub fn profile_tick(&mut self, sim: &Simulation) {
        for system in &sim.systems {
            let start = Instant::now();
            system.run(&sim.world, &sim.resources)?;
            self.stage_times.insert(system.stage(), start.elapsed());
        }
    }
    
    pub fn print_summary(&self) {
        for (stage, duration) in &self.stage_times {
            println!("{:?}: {:.2}ms", stage, duration.as_secs_f32() * 1000.0);
        }
    }
}
```

**Determinism Debugging**:
If tick N diverges from replay:
```rust
// In determinism test
if hashes[i] != hashes_replay[i] {
    println!("Divergence at tick {}", i);
    println!("Original: {}", hashes[i]);
    println!("Replay:   {}", hashes_replay[i]);
    
    // Binary search to find first diverging entity
    for entity_id in entities {
        let hash_o = hash_entity(original_world, entity_id);
        let hash_r = hash_entity(replay_world, entity_id);
        if hash_o != hash_r {
            println!("First difference: entity {}", entity_id);
            // Print component diffs
            break;
        }
    }
}
```
