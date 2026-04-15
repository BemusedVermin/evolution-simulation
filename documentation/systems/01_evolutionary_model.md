# Evolutionary Model: Open-Ended Trait Channels

## 1. Overview

This document describes the core evolutionary dynamics that drive monster adaptation in the Beast Evolution Game. The system is founded on a principle: **simulation first, gameplay convenience second**. Emergent behavioral complexity should arise from first-principles physical and biological rules, not from authored mechanics.

The evolutionary model consists of five interconnected layers:

```
┌─────────────────────────────────────────────────────┐
│  Layer 5: POPULATION DYNAMICS                       │
│  (selection, reproduction, speciation, migration)   │
├─────────────────────────────────────────────────────┤
│  Layer 4: ENVIRONMENT INTERFACE                     │
│  (biome pressures, resource density, player impact) │
├─────────────────────────────────────────────────────┤
│  Layer 3: PHENOTYPE INTERPRETER                     │
│  (channel profiles → game behaviors & visuals)      │
├─────────────────────────────────────────────────────┤
│  Layer 2: GENE NETWORK + CHANNEL AGGREGATION        │
│  (regulatory resolution, epistasis, interference)   │
├─────────────────────────────────────────────────────┤
│  Layer 1: GENOTYPE                                  │
│  (trait genes, channels, body sites, timing)        │
└─────────────────────────────────────────────────────┘
```

Importantly: **the Phenotype Interpreter is the game engine's responsibility, not this model's.** This document specifies the channel substrate and how evolution produces channel profiles. The interpreter (how channels map to visuals and mechanics) is documented separately and can evolve independently of the evolutionary model.

---

## 2. Research Basis

**Fisher's Geometric Model (Fisher 1930; Orr 1998; Matuszewski et al. 2014)**
Organisms optimize their position in a high-dimensional phenotype space relative to an environmental optimum. Smaller mutations are more likely beneficial. A moving optimum (shifting environment) predicts adaptation dynamics as a race between environmental change and evolutionary response. This justifies continuous trait values and the fitness-landscape perspective.

**Kauffman's NK Model (Kauffman 1993; Weinreich et al. 2018)**
Gene interactions create rugged fitness landscapes. K (average epistatic partners per locus) controls landscape complexity; K=0 is smooth, higher K is rugged. Higher-order epistasis declines (pairwise > three-way). This informs the sparse regulatory network design and justifies global epistasis penalties.

**Global Epistasis (Diaz-Colunga et al. 2023; Weinreich et al. 2018)**
The fitness effect of mutations depends on background genetic fitness—a pattern called global epistasis. This produces diminishing returns: as organisms become fitter, beneficial mutations have smaller effects. Prevents runaway optimization and maintains evolvability.

**Allometric Scaling: Kleiber's Law (Kleiber 1932; West et al. 1997; Glazier 2005)**
Metabolic rate scales as mass^(3/4) across a vast range of organisms, from bacteria to whales. This is one of biology's most robust scaling laws, emerging from space-filling network optimization. Critically, it holds across body size scales: elephants and bacteria both obey the 3/4 rule when normalized to their mass. This enables a unified evolutionary model for macro hosts and micro pathogens.

**Source-Sink Metapopulation Dynamics (Hanski 1998; Pulliam 1988)**
Populations in favorable habitats (sources) produce surplus emigrants that colonize marginal habitats (sinks). Networks of connected populations show different dynamics than isolated populations. Justifies migration rates based on biome adjacency and resource density rather than frustration-valve immigration.

**Axelrod's Culture Model (Axelrod 1997)**
Local homophily (assimilation to neighbors) produces global diversity (stable heterogeneous regions). Small mutation rates prevent complete convergence. Adapted here to show how monsters evolve locally distinctive traits rather than a single optimal form.

**Frequency-Dependent Selection (Maynard Smith 1982; Kokko & Heubel 2011)**
Rare phenotypes may enjoy fitness advantages (opponents unprepared for them, or frequency-dependent resources). This is a real biological phenomenon, not a game-design fudge. Can emerge naturally if prey/predator strategies are modeled explicitly.

**Life-Dinner Principle vs. Trophic Asymmetry (McLean et al. 2024)**
Predator-prey arms races don't always favor prey, despite prey having more to lose. Predator rarity, attack difficulty, and defense costliness all affect who evolves faster. Relevant for modeling player as selective pressure.

**Evolutionary Dynamics of Parasites and Pathogens (Anderson & May 1982; Read & Taylor 2001)**
Parasites and pathogens evolve under the same fitness landscape as hosts: transmission success, virulence, and within-host replication rates are selected. At micro body scales, host_coupling channels (which remain dormant at macro scale) become expressible and under selection. Parasite/pathogen/commensal/symbiont distinctions arise from the sign and magnitude of host_coupling channel outputs—not from separate evolutionary machinery.

---

## 3. Entities & State

### Layer 1: Genotype

Each monster's genome is a variable-length list of **Trait Genes**. A trait gene is not a named type—it is a compositional structure:

```
TraitGene {
    // WHAT the trait produces
    effect: EffectVector {
        channel: float[NUM_CHANNELS]         // contribution to each interaction channel (e.g., [0.3, 0.0, 0.8, ...])
        magnitude: float                     // expression strength (0..1 after network resolution)
        radius: float                        // spatial range (0 = self-only, 1 = area-of-effect, continuous)
        timing: enum { Passive, OnContact, OnDamage, OnCooldown, Periodic }
        target: enum { Self, TouchedEntity, AreaFriend, AreaFoe, Environment }
    }

    // WHERE on the body it manifests
    body_site: BodyVector {
        surface_vs_internal: float           // 0 = deep organ, 1 = surface protrusion
        body_region: float                   // 0..1, mapped to procedural body topology (anterior/posterior/lateral)
        bilateral_symmetry: bool             // mirrored on other side?
        coverage: float                      // fraction of region affected (0..1)
    }

    // HOW it relates to other genes
    regulatory: list<Modifier>               // outgoing regulatory connections
    enabled: bool                            // can be silenced independently
    lineage_tag: phylo_id                    // unique ancestor identifier (for speciation)
}

Modifier {
    target_gene_index: int                   // index into genome
    effect_type: enum { Activate, Suppress, Modulate }
    strength: float                          // [-1.0, 1.0]
}

NUM_CHANNELS = 18  // see below
```

**Interaction Channels** (the periodic table of traits):

```
Physical (channels 0-4):
  0: kinetic_force           // push, charge, strike magnitude
  1: structural_rigidity     // armor, shell, bone density
  2: elastic_deformation     // flexibility, dodge, squeeze
  3: mass_density            // weight, momentum, sinking vs. floating
  4: surface_friction        // grip, adhesion, slipperiness

Chemical (channels 5-7):
  5: chemical_output         // secretion, spray, cloud production
  6: chemical_resistance     // immunity, neutralization, detoxification
  7: chemical_sensing        // smell, taste, olfaction sensitivity

Thermal (channels 8-9):
  8: thermal_output          // heat generation, cold aura
  9: thermal_resistance      // insulation, temperature tolerance

Electromagnetic & Vibrational (channels 10-13):
  10: light_emission         // bioluminescence, flash brightness
  11: light_absorption       // camouflage, darkness, photosynthesis
  12: vibration_output       // sound generation, tremor, echolocation pings
  13: vibration_sensing      // hearing acuity, tremorsense sensitivity

Biological (channels 14-17):
  14: regeneration_rate      // healing, regrowth speed
  15: metabolic_rate         // energy efficiency, speed of all processes
  16: reproductive_rate      // fecundity, egg/offspring production
  17: neural_speed           // reaction time, information processing, sapience
```

**Mutation Model** (per reproduction event):

| Mutation Type | Per-Locus Rate | Description |
|---|---|---|
| Point mutation (allele) | 1.0e-3 | Shift `magnitude` by N(0, 0.1) |
| Regulatory rewiring | 5.0e-4 | Add, remove, or mutate a Modifier |
| Channel shift | 5.0e-4 | Shuffle channel contributions by N(0, 0.15) |
| Duplication | 5.0e-5 | Copy a random gene → paralog; provenance marked |
| Divergence | per-generation | Paralog's manifest drifts independently; selection acts on contribution |
| Reclassification | 1.0e-5 | Paralog reclassifies to new family if manifest distance threshold met |
| Deletion (Loss) | 1.0e-4 | Remove zero-contribution gene; stochastic, fitness-gated |
| Silencing toggle | 1.0e-3 | Flip `enabled` on a random gene |
| Body site shift | 1.0e-3 | Drift `surface_vs_internal`, `body_region` by N(0, 0.1) |

**Rationale**: These rates are biologically plausible. The per-genome mutation rate (Σ per-locus × genome length) is what determines visible evolutionary pace. A typical monster with ~15 genes sees ~2-4 mutations per generation, matching empirical observation that evolution is visible but not instantaneous. Gene duplication, divergence, reclassification, and loss are discussed in detail in Section 2B.

---

### Numerical Representation & Determinism

All channel values and continuous quantities in simulation state are stored as **fixed-point Q32.32** (64-bit signed integers) representing values in [0, 1] canonical range. This ensures bit-identical replay across platforms and compilers.

**Fixed-Point Encoding**:
- Continuous value `v ∈ [0, 1]` is encoded as integer `v_fixed = round(v × 2^32)` in [0, 2^32].
- All channel-to-channel operations (Modulate, composition hooks) use fixed-point arithmetic.
- Mutation and Box-Muller gaussian sampling are deterministic via seeded xoshiro256** PRNG.

**Mutation Kernel** (deterministic):
```
function mutate_channel_magnitude(current: Q32.32, sigma: Q32.32, rng_stream: xoshiro256**) -> Q32.32:
  u1, u2 = rng_stream.next() % (2^32)  // uniform [0, 2^32)
  // Box-Muller: convert two uniform variates to standard normal
  r = sqrt(-2 * ln(u1 / 2^32))  // fixed-point sqrt
  theta = 2 * PI * (u2 / 2^32)  // angle in [0, 2*PI)
  z = r * cos(theta)  // fixed-point multiply
  new_magnitude = current + sigma * z
  return clamp(new_magnitude, 0, 2^32)  // reflect boundary condition
```

**PRNG Stream Ownership**:
- Evolution system: one xoshiro256** stream seeded from (world_seed + subsystem_id_evolution).
- Ecology system: one xoshiro256** stream seeded from (world_seed + subsystem_id_ecology).
- (All other systems follow the same pattern.)
- Stream state is serialized in save files; replay restores exact state.

### Channel Correlation Application (Issue #11)

When a point mutation alters a channel, correlated channels are automatically adjusted to maintain biological pleiotropy. This reflects the reality that many morphological traits share common genetic control (e.g., increased metabolic_rate → increased thermal_output due to shared endocrine pathways).

**Algorithm**:
```
When point-mutating channel A with N(0, sigma):
  delta_A = sample from N(0, sigma)
  channels[A] += delta_A
  
  for corr in A.manifest.correlation_with:
    channels[corr.id] += delta_A * corr.coefficient * sigma_factor
  
  // Transitive correlations NOT applied (prevents feedback loops)
```

**Design**: Each channel manifest lists its correlations via `correlation_with` array, specifying target channel IDs and correlation coefficients (signed floats in [-1.0, 1.0]). When A mutates, each correlated channel receives a proportional update. Coefficients can be negative (antagonistic pleiotropy: increasing armor reduces speed) or positive (synergistic: more sensory neurons → faster processing).

**Transitive Correlation Prevention**: Only first-order correlations are applied. If A→B and B→C, a mutation to A affects B directly but NOT C indirectly. This prevents runaway feedback loops and maintains computational tractability.

---

### Layer 2: Gene Network & Channel Aggregation

The gene network is a **directed weighted graph** where nodes are genes and edges are Modifiers.

**Network Resolution** (run once per monster, cached until mutation):

```pseudocode
function resolve_network(genome, max_iterations=5) -> float[NUM_CHANNELS]:
    // Step 1: Initialize expression levels from allele magnitudes
    expression = {}
    for gene in genome where gene.enabled:
        expression[gene.index] = gene.effect.magnitude

    // Step 2: Iterative relaxation to resolve regulatory interactions
    for iteration in 1..max_iterations:
        delta = 0.0
        new_expression = copy(expression)
        for gene in genome where gene.enabled:
            for modifier in gene.regulatory:
                target_idx = modifier.target_gene_index
                if target_idx < len(genome) and genome[target_idx].enabled:
                    influence = expression[gene.index] * modifier.strength
                    match modifier.effect_type:
                        Activate:  new_expression[target_idx] += influence * 0.3
                        Suppress:  new_expression[target_idx] -= influence * 0.3
                        Modulate:  new_expression[target_idx] *= (1.0 + influence * 0.2)
            delta += abs(new_expression[gene.index] - expression[gene.index])
        
        // Clamp to prevent unbounded growth
        for i in 0..len(genome):
            new_expression[i] = clamp(new_expression[i], 0.0, 1.5)
        
        expression = new_expression
        if delta < 0.001:
            break  // convergence
    
    // Step 3: Aggregate into channel profiles
    channels = float[NUM_CHANNELS]
    for gene in genome where gene.enabled:
        for ch in 0..NUM_CHANNELS:
            channels[ch] += gene.effect.channel[ch] * expression[gene.index]
    
    return channels

function resolve_channel(channel_idx, genome, global_channels) -> float:
    // Applies global epistasis (diminishing returns) within a channel
    contributions = []
    for gene in genome where gene.enabled and gene.effect.channel[channel_idx] > EPSILON:
        effective_magnitude = expression[gene.index]  // from resolve_network
        contribution = gene.effect.channel[channel_idx] * effective_magnitude
        contributions.append(contribution)
    
    if len(contributions) == 0:
        return 0.0
    elif len(contributions) == 1:
        return contributions[0]
    else:
        // Diminishing returns: first source gets full value, subsequent sources get discounted
        sorted_desc = sort_descending(contributions)
        total = sorted_desc[0]
        DIMINISHING_FACTOR = 0.7  // each additional source contributes 70% of previous
        for i in 1..len(sorted_desc):
            total += sorted_desc[i] * (DIMINISHING_FACTOR ^ i)
        return clamp(total, 0.0, 2.0)  // allow synergistic peak, but capped
```

**Cyclic Network Detection and Damping**:

```pseudocode
function detect_and_resolve_cycles(graph, max_iterations=5):
    // Use Tarjan's SCC (Strongly Connected Component) algorithm
    sccs = tarjan_scc(graph)
    
    for scc in sccs where len(scc) > 1:  // only cyclic SCCs
        // Cyclic subgraph — apply fixed-point iteration
        apply_damped_iteration(scc, damping=0.95, max_iterations=3)
```

**Design Rationale**: Network resolution via iterative relaxation mirrors gene regulatory network dynamics (Schlitt & Brazma 2007, Kauffman 1969) while staying computationally tractable. The 1.5 upper clamp allows emergent synergies without unbounded growth. Diminishing returns at the channel level implement global epistasis and prevent degenerate strategies (stacking ten "armor genes" doesn't give 10x armor).

---

## 2B. Gene Duplication, Divergence, Reclassification, and Loss

### Gene Duplication and Channel Speciation

**Overview**: Gene duplication (Ohno 1970) is the primary mechanism by which new functional channels arise. When a gene is duplicated, the paralog initially carries the parent's manifest verbatim. Over generations, the paralog's manifest drifts independently under mutation and selection. Divergent paralogs that contribute to fitness are retained; non-contributing paralogs are pruned.

**Formal Signature**:

```
DUPLICATION: Parent Gene → Paralog + Parent
  rate: genome_level duplication_rate (itself mutable)
  provenance: "genesis:{parent_channel_id}:{generation}"
  
Parent manifest retained verbatim.
Paralog initialized with:
  - Same channel profile as parent
  - Same regulatory structure as parent
  - Composition hooks initially identical (redundancy, no fitness gain yet)
  - composition_hooks[...] initially point to parent channel value
```

**Rates and Dynamics**:

```pseudocode
function apply_duplication(genome) -> modified_genome:
    duplication_rate = genome.params.duplication_rate
    
    if random() < duplication_rate:
        parent_idx = random_gene_index(genome)
        parent_gene = genome[parent_idx]
        
        // Create paralog
        paralog = deep_copy(parent_gene)
        paralog.id = new_unique_id()
        paralog.provenance = "genesis:{parent_gene.id}:{current_generation}"
        
        // Add noise to paralog to distinguish from parent immediately
        for ch in paralog.effect.channel:
            ch += normal(0, 0.05)
        paralog.effect.magnitude += normal(0, 0.03)
        
        genome.genes.append(paralog)
        
        return genome

// Genome-level duplication_rate evolves alongside other traits
function mutate_duplication_rate(genome):
    if random() < 1.0e-4:  // rare per-genome mutation
        genome.params.duplication_rate += normal(0, 5.0e-6)
        genome.params.duplication_rate = clamp(genome.params.duplication_rate, 1.0e-6, 1.0e-3)
```

**Rationale (Ohno 1970)**: Gene duplication allows one copy to be "freed" for neofunctionalization (new function) while the other maintains the ancestral function (subfunctionalization). This is how organisms expand their functional toolkit without losing essential genes. Biological examples: immunoglobulin gene family, olfactory receptors, developmental toolkit genes.

---

### Divergence

**Overview**: After duplication, paralogs drift independently over generations. Their mutation kernels may shift, ranges may shift, composition hook coefficients drift. Selection acts on the composite phenotype; if the divergent paralog contributes to fitness, it is retained.

**Formal Signature**:

```
DIVERGENCE: Paralog Manifest → Diverged Manifest
  driven by: standard point mutation + channel shift operators
  selection pressure: fitness contribution of the paralog's output
  outcome: manifest drifts away from parent in channel profile, regulatory targets, body site
```

**Dynamics**:

```pseudocode
function diverge_paralog(paralog, generations_since_duplication):
    // Standard mutation operators apply to paralog independently
    // Over time, cumulative drift causes paralog to differ from parent
    
    // Example: after 10 generations, mutation accumulation is ~0.1-0.3 units of drift
    // If paralog contributes to a channel that is under strong selection,
    // beneficial mutations are retained and divergence accelerates
    
    // Measure contribution of paralog to resolved channel profile
    contribution = measure_paralog_contribution(paralog, genome, resolved_channels)
    
    if contribution > MINIMUM_CONTRIBUTION_THRESHOLD:
        // Paralog is expressed and selected for
        // Mutations that increase its contribution are retained
        diversifying_selection = true
    else:
        // Paralog is redundant or harmful; mutations are nearly neutral
        mutation_drift = true  // non-adaptive drift, slower change
    
    return paralog  // with accumulated mutations

function measure_paralog_contribution(paralog, genome, resolved_channels) -> float:
    // Sum the absolute effect of this paralog on all channels
    contribution = 0.0
    for ch in 0..NUM_CHANNELS:
        contribution += abs(paralog.effect.channel[ch] * paralog_expression_level)
    return contribution
```

**Rationale**: Divergence mirrors the molecular clock and adaptive radiation. Paralogs that acquire new functions (higher contribution to different channels or body sites) expand faster. Those that remain redundant drift slowly under nearly-neutral mutation. This is observable in real genomes: olfactory receptor pseudogenes diverge slowly; immune receptor gene clusters diverge rapidly.

---

### Reclassification

**Overview**: When a paralog's manifest and interaction patterns drift sufficiently far from its original family's defaults, it may reclassify into a different family (or, rarely, found a new family). This is rare but captures the biological notion that functions can shift roles as they evolve.

**Formal Signature**:

```
RECLASSIFICATION: Paralog{family_A} → Paralog{family_B}
  trigger: manifest distance from family_A baseline > threshold
           && manifest distance from family_B baseline < threshold
  rate: depends on epistasis density and family overlap
  outcome: channel now belongs to different family
```

**Dynamics**:

```pseudocode
function reclassify_if_diverged(paralog, genome):
    original_family = paralog.family
    
    // Measure distance from original family's defaults
    distance_from_original = family_manifest_distance(paralog, original_family)
    
    if distance_from_original > RECLASSIFICATION_THRESHOLD:  // e.g., 0.5
        // Check if paralog is now closer to a different family
        for candidate_family in [SENSORY, MOTOR, ..., DEVELOPMENTAL]:
            distance_from_candidate = family_manifest_distance(paralog, candidate_family)
            
            if distance_from_candidate < distance_from_original:
                // Reclassify
                paralog.family = candidate_family
                paralog.reclassification_history.append({
                    from: original_family,
                    to: candidate_family,
                    generation: current_generation
                })
                return
    
    // Very rare: found new family if no existing family is a good fit
    if min_distance_across_families > 0.7 and random() < 1.0e-5:
        new_family = create_new_family(paralog)
        paralog.family = new_family

function family_manifest_distance(paralog, family) -> float:
    // Distance metric: compare channel profile, regulatory structure, body site applicability
    distance = 0.0
    baseline = get_family_baseline_manifest(family)
    
    for ch in paralog.effect.channel:
        distance += abs(ch - baseline.channel[ch])
    
    distance += edit_distance(paralog.regulatory, baseline.regulatory) * 0.3
    distance += abs(paralog.body_site.surface_vs_internal - baseline.body_site.surface_vs_internal)
    
    return distance
```

**Rationale**: Reclassification is rare because families are broad categories tied to fundamental biology (sensory, motor, metabolic, etc.). However, it can happen: a gene duplicated from a motor channel might evolve to become a sensory organ if selection favors that function. Real example: the lens of the eye is crystallin proteins, some of which are repurposed heat-shock proteins (HSP members). This operator captures that possibility without requiring explicit design.

---

### Gene Loss (Pruning)

**Overview**: A channel with zero effective contribution (composition hooks drift toward zero, or expression conditions are never met) can be pruned with low probability, reducing genome bloat.

**Formal Signature**:

```
LOSS: Paralog → ∅
  trigger: effective_contribution < LOSS_THRESHOLD (0.01)
           AND mutations would increase fitness if pruned
  rate: 1.0e-4 per gene per generation
  outcome: gene is deleted from genome
```

**Dynamics**:

```pseudocode
function prune_if_useless(gene, genome, fitness_before):
    contribution = measure_gene_contribution(gene, genome)
    
    if contribution < LOSS_THRESHOLD:  // 0.01
        // Estimate fitness if gene is pruned
        genome_without = deep_copy(genome)
        genome_without.genes.remove(gene)
        fitness_after = fitness_total(monster_with_genome(genome_without))
        
        if fitness_after >= fitness_before - TOLERANCE:  // -1% tolerance
            // Pruning doesn't hurt, and reduces complexity cost
            if random() < GENE_LOSS_RATE:  // 1.0e-4
                genome.genes.remove(gene)
                return true
    
    return false
```

**Rationale**: Biological genomes accumulate pseudogenes and junk DNA, but strong purifying selection removes useless genes. In our system, a gene with zero contribution adds metabolic overhead (complexity tax). Pruning occurs stochastically when the fitness benefit of reduced complexity exceeds the fitness cost of losing the gene entirely.

---

### Layer 3: Phenotype Interpreter

The Phenotype Interpreter translates resolved channel profiles into concrete game behaviors and visuals. **This is the game engine's responsibility.** The evolutionary model is agnostic to interpretation. The interpreter is a set of declarative rules:

```
PhenotypeProfile {
    global_channels: float[NUM_CHANNELS]      // whole-body aggregate
    body_map: Map<BodyRegion, float[NUM_CHANNELS]>  // per-region breakdown
    total_metabolic_cost: float               // fitness penalty
    behavioral_params: BehaviorProfile        // derived from channels
}

// EXAMPLE INTERPRETER RULES (game engine defines these):

// Movement
speed = BASE_SPEED
    * (1 + channels[ELASTIC_DEFORMATION] * 0.5)
    * (1 - channels[STRUCTURAL_RIGIDITY] * 0.3)
    * (1 + channels[METABOLIC_RATE] * 0.4)
    / (1 + channels[MASS_DENSITY] * 0.5)

// Defense
armor_value = 1.0 + channels[STRUCTURAL_RIGIDITY] * 5.0
dodge_chance = channels[ELASTIC_DEFORMATION] * 0.6
heal_per_tick = channels[REGENERATION_RATE] * 0.1

// Attack behaviors (from timing + target)
if any body_region has high(KINETIC_FORCE) AND timing == OnContact:
    → melee_attack_power = base_damage * (1 + channels[KINETIC_FORCE] * 2.0)
if any body_region has high(CHEMICAL_OUTPUT) AND timing == OnContact:
    → contact_poison_per_hit = channels[CHEMICAL_OUTPUT] * 0.5
if radius > 0.5 AND target == AreaFoe:
    → area_denial_effect = true

// Sensory
smell_range = BASE_SMELL * (1 + channels[CHEMICAL_SENSING])
hearing_range = BASE_HEARING * (1 + channels[VIBRATION_SENSING])
vision_in_darkness = channels[LIGHT_ABSORPTION] * 0.8  // see in dark

// Procgen visuals
for each body_region:
    if channels[STRUCTURAL_RIGIDITY] > 0.5:
        add_shell_mesh(coverage=coverage)
    if channels[SURFACE_FRICTION] > 0.7:
        add_spines_or_texture(intensity=channels[SURFACE_FRICTION])
    if channels[CHEMICAL_OUTPUT] > 0.3:
        add_gland_particles()
    if channels[LIGHT_EMISSION] > 0.2:
        add_glow_shader(intensity=channels[LIGHT_EMISSION])
    if channels[ELASTIC_DEFORMATION] > 0.6:
        soften_meshes()
    if channels[MASS_DENSITY] > 0.7:
        scale_up_model(), heavy_walk_animation()
    if channels[REGENERATION_RATE] > 0.4:
        add_shimmer_aura()
```

The interpreter is the bridge between the abstract channel system and the concrete game. It is designer-authored but modular—adding new visual or behavioral responses to channel combinations does not require changes to the evolutionary model.

---

### Layer 4: Environment Interface

Biome cells express selective pressure as **channel fitness modifiers**:

```
BiomeCell {
    // Channel-level fitness modifiers (multipliers)
    channel_fitness: float[NUM_CHANNELS]      // e.g., [0.2, -0.1, 0.8, ...]
    
    // Resource availability (drives carrying capacity and reproduction)
    resource_density: float                   // 0..1, food availability
    
    // Environmental stressors
    hazard_level: float                       // environmental damage per tick
    temperature: float                        // affects thermal channels
    light_level: float                        // affects light channels
    
    // Predation pressure (crucial for defensive channel fitness)
    predation_pressure: float                 // = (kill_rate_by_all_agents / total_population) * scaling
    player_activity: float                    // rolling average over last 365 ticks
    
    // Carrying capacity (from resource density + environmental harshness)
    carrying_capacity: function(resource_density, hazard_level)
}

function compute_predation_pressure(cell, tick):
    // Empirical kill rate by all predators (beasts, NPCs, player) in this cell
    kill_count = count_kills_in_cell(cell, tick - 365, tick)  // rolling 365-tick window
    population = count_agents_in_cell(cell)
    if population == 0:
        return 0.0
    
    kill_rate = kill_count / population
    // Scale to fitness landscape: high kill rate = strong selection for defense
    return clamp(kill_rate * 100.0, 0.0, 1.0)  // tunable scaling

function carrying_capacity(cell):
    base = 50  // baseline
    return base * (1 + cell.resource_density * 10) * (1 - cell.hazard_level * 0.5)
```

**Fitness Calculation**:

```pseudocode
function fitness_total(monster, biome_cell) -> float:
    f_metabolic = compute_metabolic_fitness(monster)
    f_environment = compute_environmental_fitness(monster, biome_cell)
    f_survival = monster.health / monster.max_health  // current condition
    
    // Defensive channel fitness scales with local predation pressure
    f_defense = 0.0
    defensive_channels = [STRUCTURAL_RIGIDITY, REGENERATION_RATE, NEURAL_SPEED, 
                          CHEMICAL_RESISTANCE, THERMAL_RESISTANCE, ELASTIC_DEFORMATION]
    for ch in defensive_channels:
        f_defense += monster.channels[ch] * biome_cell.predation_pressure
    
    // Total fitness is multiplicative to enforce constraint satisfaction
    fitness = f_metabolic * f_environment * f_survival * (1.0 + f_defense)
    return clamp(fitness, 0.0, 1.0)

function compute_metabolic_fitness(monster) -> float:
    // Allometric scaling: Kleiber's law (mass ^ 0.75)
    base_metabolic_cost = BASE_RATE * (monster.total_mass ^ 0.75)
    
    // Complexity tax: cost per active gene (maintenance burden)
    complexity_penalty = COMPLEXITY_COEFFICIENT * count_enabled_genes(monster)
    
    // Channel expression costs
    channel_costs = 0.0
    for ch in 0..NUM_CHANNELS:
        channel_costs += CHANNEL_BASE_COST[ch] * (monster.channels[ch] ^ 1.5)
    
    total_metabolic_cost = base_metabolic_cost + complexity_penalty + channel_costs
    
    // Return fitness: full fitness if cost is zero, declines to zero if cost >= metabolic_budget
    return max(0.0, 1.0 - total_metabolic_cost)

function compute_environmental_fitness(monster, biome_cell) -> float:
    fitness = 0.5  // baseline
    for ch in 0..NUM_CHANNELS:
        fitness += biome_cell.channel_fitness[ch] * monster.channels[ch] * 0.1
    return clamp(fitness, 0.0, 1.0)

function fitness_total_multiscale(monster, biome_cell) -> float:
    // Extended fitness function accounting for multi-scale ecology
    f_metabolic = compute_metabolic_fitness(monster)
    f_environment = compute_environmental_fitness(monster, biome_cell)
    f_survival = monster.health / monster.max_health
    
    // Scale-band dependent selection
    if monster.body_size_scale == MACRO:
        // Macro organisms: standard channel fitness
        f_defense = 0.0
        defensive_channels = [STRUCTURAL_RIGIDITY, REGENERATION_RATE, NEURAL_SPEED,
                              CHEMICAL_RESISTANCE, THERMAL_RESISTANCE, ELASTIC_DEFORMATION]
        for ch in defensive_channels:
            f_defense += monster.channels[ch] * biome_cell.predation_pressure
        
        return f_metabolic * f_environment * f_survival * (1.0 + f_defense)
    
    elif monster.body_size_scale == MICRO:
        // Micro organisms (pathogens): host-coupling channels dominate
        f_within_host = 0.0
        host_coupling_channels = [HOST_COUPLING_TRANSMISSION, HOST_COUPLING_VIRULENCE, 
                                  HOST_COUPLING_SUPPRESSION]
        for ch in host_coupling_channels:
            if ch == HOST_COUPLING_VIRULENCE:
                f_within_host -= monster.channels[ch] * 0.5  // virulence is a trade-off
            else:
                f_within_host += monster.channels[ch]  // transmission and suppression benefit
        
        host_availability = estimate_host_density_and_susceptibility(biome_cell)
        return f_metabolic * (1.0 + f_within_host) * host_availability
    
    else:  // MESO scale (rare parasites on macro bodies)
        // Blend: some environmental fitness, some host-coupling
        f_defense = 0.0
        for ch in defensive_channels:
            f_defense += monster.channels[ch] * biome_cell.predation_pressure
        f_coupling = monster.channels[HOST_COUPLING_TRANSMISSION] * 0.1  // weak effect
        return f_metabolic * f_environment * f_survival * (1.0 + f_defense + f_coupling)
```

**Design Rationale**: Predation pressure is derived from empirical kill rates, not a static value. This makes player-pressure adaptation a special case of general predator-pressure adaptation—pure simulation, not a separate game mechanic. Resource-dependent carrying capacity makes population density responsive to ecology, not fixed. Allometric scaling (Kleiber's law) is empirically grounded.

---

## 4B. Disease and Parasitism as Multi-Scale Evolutionary Outcomes

### Unified Model: No Separate Pathogen System

**Overview**: Diseases, parasites, commensals, and symbionts are not driven by separate evolutionary machinery. Instead, they emerge from the same evolutionary operators applied at different body-size scales. A **scale-band** parameter (defined in System 16) determines which monsters can physically interact with which. At the macro scale (elephants, dragons), pathogens remain dormant. At the micro scale (bacteria, viruses), host-interaction channels (dormant in large organisms) become the primary fitness landscape.

**Key Insight**: Parasites are simply monsters that:
1. Evolve at the micro body-size scale
2. Have high host_coupling channel values (negative or positive)
3. Exist within a host biome cell, competing for reproductive success
4. Spread through host-to-host transmission when conditions allow

**Scale-Band Architecture**:

```
Body Size Scale Band | Example Organism | Primary Channels Active | Scale Context |
---|---|---|---|
Macro | Elephant, Dragon | sensory, motor, metabolic (standard) | Free-living, environmental |
Meso | Large insects, fish | sensory, motor, metabolic + weak host_coupling | Rare parasites/symbionts |
Micro | Bacteria, viruses | host_coupling, metabolic (miniaturized) | Within-host, host-dependent |

// Kleiber scaling works across all bands:
base_metabolic_cost = BASE_RATE * (mass ^ 0.75)
// For a macro beast: mass = 100 kg → cost ∝ 100^0.75 ≈ 31
// For a micro pathogen: mass = 1e-9 kg → cost ∝ (1e-9)^0.75 ≈ 1e-7 (proportionally similar metabolic burden)
```

**Host-Coupling Channels**:

The channel registry includes three dormant channels that become expressible at micro scale:

```
Dormant at Macro Scale (value ≈ 0):
  host_coupling_transmission    // how readily parasite spreads host→host
  host_coupling_virulence       // damage or metabolic drain caused by parasite
  host_coupling_suppression     // immune evasion, antibiotic resistance
```

At the micro scale:

```pseudocode
function fitness_micro_parasite(parasite, host_biome_cell) -> float:
    // Same fitness function as macroscale, but different channel selection
    f_metabolic = compute_metabolic_fitness_microbe(parasite)  // Kleiber-scaled
    
    // Host-coupling channels now dominate fitness
    transmission_success = parasite.channels[HOST_COUPLING_TRANSMISSION] * host_density
    virulence_damage = parasite.channels[HOST_COUPLING_VIRULENCE]  // trade-off with transmission
    immune_evasion = parasite.channels[HOST_COUPLING_SUPPRESSION]
    
    // Reproduction within host (faster generation times due to small size)
    within_host_fitness = (transmission_success + immune_evasion - virulence_damage * 0.5)
    
    // Total fitness: metabolic feasibility × within-host replication × transmission opportunity
    fitness = f_metabolic * within_host_fitness * host_availability
    return clamp(fitness, 0.0, 1.0)

function reproduction_rate_microbe(parasite, host_biome_cell, tick) -> float:
    // Generation time much shorter for microbes (hours to days, not seasons)
    fitness = fitness_micro_parasite(parasite, host_biome_cell)
    host_load = estimate_parasite_load_in_host(host_biome_cell)
    
    generation_length = MICRO_BASE_GENERATION_TICKS / (parasite.channels[METABOLIC_RATE] * SCALE_SPEEDUP)
    // SCALE_SPEEDUP ≈ 1000, so micro generations happen 1000× faster
    
    lambda = fitness * (1 - host_load / MAX_PARASITE_PER_HOST) / generation_length
    return 1.0 - exp(-lambda)
```

**Parasite Diversity: From the Same Registry**:

All monsters (macro hosts and micro pathogens) are drawn from the same trait registry (System 02). A pathogen is instantiated as:

```
Pathogen {
    is_pathogen: bool = true
    body_size_scale: enum = MICRO
    parent_host_id: uuid  // reference to infecting host instance
    location: within_host  // not a world coordinate; "within bloodstream" etc.
    channels: float[NUM_CHANNELS]  // same 18 channels; mostly zero except host_coupling_*
}
```

Selection, mutation, and speciation operate identically:

```pseudocode
function coevolve_host_parasite(host, parasites_in_host, tick):
    // Host fitness affected by parasites
    total_parasite_damage = sum(p.channels[HOST_COUPLING_VIRULENCE] for p in parasites_in_host)
    host_fitness *= (1.0 - total_parasite_damage * 0.01)  // parasites reduce host fitness
    
    // Parasites reproduce and mutate
    for parasite in parasites_in_host:
        p_repr = reproduction_rate_microbe(parasite, host)
        if random() < p_repr:
            child_parasite = apply_mutations(parasite.genome)
            parasites_in_host.append(child_parasite)
    
    // If parasite load exceeds threshold, some parasites burst out (transmission)
    if len(parasites_in_host) > MAX_PARASITE_PER_HOST:
        excess = len(parasites_in_host) - MAX_PARASITE_PER_HOST
        transmit_to_neighbors(host, excess_parasites)
    
    // Host selection and speciation apply to both host and parasite populations
    select_population(parasites_in_host, tick)  // remove lowest-fitness parasites
```

---

### Emergence of Pathogenic Strategies

**Virulence-Transmission Trade-off** (Anderson & May 1982):

High virulence (large HOST_COUPLING_VIRULENCE channel value) damages hosts but may release parasites faster. High transmission (HOST_COUPLING_TRANSMISSION) spreads widely but requires host survival. Optimal virulence is an evolutionary outcome, not a tunable parameter:

```pseudocode
// Parasite A: high virulence, kills host in 10 days, transmits to 100 neighbors
// Parasite B: low virulence, keeps host alive 100 days, transmits to 50 neighbors
// Parasite B spreads farther; selection favors it UNLESS hosts evolve rapid recovery
// → coevolutionary spiral without explicit design
```

**Immune Evasion**:

Hosts can evolve CHEMICAL_RESISTANCE and THERMAL_RESISTANCE, which reduce parasite fitness. Parasites evolve HOST_COUPLING_SUPPRESSION to counter. These arms races emerge naturally from the fitness function.

**Symbiosis Emergence**:

If a parasite's HOST_COUPLING_VIRULENCE is negative (i.e., it provides metabolic benefit to the host), the relationship is symbiotic. Mutualism emerges when both parties benefit: host provides resources (high carrying capacity in bloodstream), parasite provides a channel benefit (e.g., enhanced digestion). No special code—just the sign of the host-coupling channel.

---

### Scale-Band Integration

System 16 ("Disease as Scale-Band Evolution") defines:
- Body size scale bands (macro, meso, micro)
- Channel expressibility constraints per scale
- Transmission mechanics (how pathogens jump hosts)
- Timescale acceleration (micro monsters reproduce 1000× faster)

This document (System 01) ensures that selection, mutation, duplication, and divergence work identically at all scales. The unified model removes the need for separate pathogen logic.

---

### Layer 5: Population Dynamics

**Speciation Metric** (updated for channel representation):

```pseudocode
function genetic_distance(monster_A, monster_B) -> float:
    // Phylo-tagged lineage marker: each gene has a unique ancestor ID
    // Two monsters from the same mutation lineage are closer than those from divergent lineages
    
    distance = 0.0
    
    // Gene lineage distance
    gene_pairs = align_genes_by_lineage_tag(A, B)
    for (geneA, geneB) in gene_pairs:
        if geneA.lineage_tag != geneB.lineage_tag:
            distance += 2.0  // high cost for different lineage (duplication/loss)
        else:
            // Same lineage — measure expression distance
            distance += abs(geneA.effect.magnitude - geneB.effect.magnitude) * 0.5
            // Channel profile distance
            distance += l2_norm(geneA.effect.channel - geneB.effect.channel)
    
    // Regulatory network distance
    for (modA, modB) in zip(A.genome, B.genome):
        distance += edit_distance(modA.regulatory, modB.regulatory) * 0.5
    
    // Normalize
    distance /= max(len(A.genome), len(B.genome))
    
    return clamp(distance, 0.0, 1.0)

SPECIATION_THRESHOLD = 0.4  // if distance > 0.4, different species
```

**Time Scales** (canonical for the entire system):

```
ONE TICK = ONE GAME-DAY (24 hours in-world)

ONE GENERATION = species-specific, derived from reproductive interval:
    generation_length_ticks = (base_generation_ticks / channels[METABOLIC_RATE])
    // Fast-metabolism species reproduce faster, slower-metabolism species reproduce slower
    // e.g., base_generation_ticks = 50, so metabolic_rate of 1.0 = 50 ticks/gen
    //       metabolic_rate of 0.5 = 100 ticks/gen

ROLLING WINDOW for selection pressure = 365 ticks (one game-year)

EXPONENTIALLY WEIGHTED MOVING AVERAGE (for player_activity, predation_pressure):
    ewma[t] = alpha * value[t] + (1 - alpha) * ewma[t-1]
    half_life = 60 ticks (tunable)  // 2-month "memory" of player activity
    alpha = 2 / (half_life + 1) = 0.0317
```

**Reproduction**:

```pseudocode
function reproduction_rate(monster, biome_cell, tick) -> float:
    fitness = fitness_total(monster, biome_cell)
    
    // Resource intake (determined by feeding success in biome)
    resource_intake = estimate_feeding_success(monster, biome_cell)
    
    generation_length = base_generation_ticks / monster.channels[METABOLIC_RATE]
    
    // Poisson process: rate = lambda
    lambda = fitness * resource_intake / generation_length
    
    // In discrete time: P(reproduction this tick) = 1 - exp(-lambda)
    return 1.0 - exp(-lambda)

function apply_mutations(parent_genome) -> child_genome:
    child = deep_copy(parent_genome)
    
    for gene in child.genome:
        // Point mutation
        if random() < 1.0e-3:
            gene.effect.magnitude += normal(0, 0.1)
            
            // Apply channel correlations (Issue #11: Channel Correlation Application)
            delta_A = gene.effect.magnitude - parent_genome[gene.index].effect.magnitude
            for corr in gene.manifest.correlation_with:
                target_channel_id = corr.id
                sigma_factor = 0.1  // mutation kernel sigma
                correlation_delta = delta_A * corr.coefficient * sigma_factor
                // Find target channel in active genes and apply correlation
                for target_gene in child.genome:
                    if target_gene.manifest.channel_id == target_channel_id and target_gene.enabled:
                        target_gene.effect.magnitude += correlation_delta
                        break
            // Note: Transitive correlations NOT applied (prevents feedback loops)
        
        // Channel shift
        if random() < 5.0e-4:
            for ch in 0..NUM_CHANNELS:
                gene.effect.channel[ch] += normal(0, 0.15)
        
        // Body site drift
        if random() < 1.0e-3:
            gene.body_site.surface_vs_internal += normal(0, 0.1)
            gene.body_site.body_region += normal(0, 0.1)
        
        // Silencing toggle
        if random() < 1.0e-3:
            gene.enabled = !gene.enabled
    
    // Regulatory rewiring
    if random() < 5.0e-4:
        add_or_remove_modifier(child)
    
    // Duplication
    if random() < 5.0e-5:
        duplicate_gene(child)
    
    // Deletion
    if random() < 5.0e-5:
        delete_gene(child)
    
    return child
```

**Selection**:

```pseudocode
function select_population(cell, tick):
    monsters = get_monsters_in_cell(cell)
    
    // Environmental + metabolic selection: remove unviable
    viable = [m for m in monsters if fitness_total(m, cell) > 0.1]
    
    if len(viable) < 2:
        // Near-extinction: enable source-sink migration from adjacent cells
        enable_immigration(cell, immigration_rate=0.5)
    
    // Intraspecific competition: if population exceeds carrying capacity
    if len(viable) > cell.carrying_capacity:
        excess = len(viable) - int(cell.carrying_capacity)
        // Cull lowest-fitness individuals (stochastically)
        candidates = sort_by(viable, key=fitness_total)
        for i in 0..excess:
            p_cull = 0.3 + 0.7 * (i / excess)  // increasing probability
            if random() < p_cull:
                monsters.remove(candidates[i])
```

**Migration** (source-sink metapopulation):

```pseudocode
function migration(cell, tick):
    // Adjacency-based: only adjacent biome cells contribute emigrants
    for adjacent_cell in neighbors(cell):
        // Emigration rate based on source quality and distance
        source_fitness = mean(fitness_total(m, adjacent_cell) for m in adjacent_cell.monsters)
        sink_fitness = mean(fitness_total(m, cell) for m in cell.monsters)
        
        if source_fitness > sink_fitness:
            // Source cell, sink cell — migration flows from source to sink
            emigration_rate = source_fitness * 0.05  // 5% of advantage
            num_emigrants = int(len(adjacent_cell.monsters) * emigration_rate)
            
            for i in 0..num_emigrants:
                emigrant = select_random_from(adjacent_cell.monsters, weight_by=fitness_total)
                child = apply_mutations(emigrant.genome)
                cell.add_monster(child, location=random_location_in_cell())
```

---

## 4C. Primitive-Genesis Operators (Future Work)

### Overview: Primitives Are Evolvable

The Phenotype Interpreter emits atomic effects (primitives) rather than named abilities. Primitives are declared in manifests in a **Primitive Registry** (parallel to the Channel Registry in System 02) and are fully evolvable: they can be duplicated, diverge, reclassified, and lost by the same mechanisms that govern channel evolution.

In this revision, primitive evolution is **deferred to future work**. We document the registry structure, outline the operators by analogy with channel genesis, and identify open research questions.

### Primitive Manifest Shape

Each primitive is declared in a manifest with the following structure:

```json
{
  "id": "emit_acoustic_pulse",
  "category": "signal_emission",
  "description": "Emit a short-duration acoustic wave",
  
  "parameter_schema": {
    "frequency_hz": {
      "type": "float",
      "range": [100, 40000],
      "default": 5000
    },
    "amplitude": {
      "type": "float",
      "range": [0.0, 1.0],
      "default": 0.5
    },
    "duration_ms": {
      "type": "float",
      "range": [1, 1000],
      "default": 100
    }
  },
  
  "composition_compatibility": {
    "channel_families": ["motor", "cognitive"],
    "channel_ids": ["vibration_output", "neural_speed"],
    "description": "Acoustic pulse is more effective with motor control and neural coordination"
  },
  
  "cost_function": {
    "metabolic_cost": 0.05,
    "cooldown_ticks": 5
  },
  
  "observable_signature": {
    "modality": "vibration",
    "range": 50.0,
    "detectability": 0.8,
    "description": "Players and creatures can hear/sense this pulse"
  },
  
  "provenance": "core",
  "generation_born": 0
}
```

**Manifest Fields:**

| Field | Purpose |
|-------|---------|
| `id` | Stable identifier (e.g., "emit_acoustic_pulse", "genesis:emit_acoustic_pulse:1500") |
| `category` | One of 8 categories: signal_emission, signal_reception, force_application, state_induction, spatial_integration, mass_transfer, energy_modulation, bond_formation |
| `parameter_schema` | JSON Schema for parameters this primitive accepts (frequency, amplitude, duration, etc.). Allows fine-grained phenotypic variation without new primitives. |
| `composition_compatibility` | List of channel families and channel ids that, when high, increase the effectiveness or availability of this primitive. Used by composition hooks in channel manifests. |
| `cost_function` | Metabolic cost and cooldown. Allows cost to scale with parameter values. |
| `observable_signature` | Detection range, modality (acoustic, visual, chemical), and strength. Used by Chronicler (System 09) for emergent labeling and by creatures for sensory detection. |
| `provenance` | "core" = shipped, "mod:X" = from mod X, "genesis:parent:generation" = evolved from paralog |
| `generation_born` | Absolute generation when created (0 for core) |

### Eight Primitive Categories

Primitives cluster into 8 functional categories, each representing an atomic vocabulary of phenotype output:

**1. Signal Emission** (broadcast information outward)
- Emit acoustic pulses, bioluminescent flashes, pheromone clouds, thermal radiation
- Used by motor (vibration_output) and social (chemical_production, light_emission) channels
- Examples: vocalization, alarm call, mating display

**2. Signal Reception** (detect information from environment)
- Transduce sensory modalities into neural signals (light → vision, vibration → hearing)
- Paired with sensory channels; usually not evolved directly
- Examples: photoreception, phonoreception

**3. Force Application** (exert mechanical work)
- Apply bite force, injection force, striking force, grip force
- Paired with motor (kinetic_force) and structural (structural_rigidity) channels
- Examples: crushing bite, venomous injection, headbutt

**4. State Induction** (alter internal or external state)
- Transition between behavioral states (e.g., "aggression," "hiding," "mating")
- Triggered by thresholds in cognitive or regulatory channels
- Examples: enter predatory mode, initiate courtship ritual

**5. Spatial Integration** (organize and coordinate multi-body-part actions)
- Synchronize locomotion, target aiming, formation control
- Depend on neural_speed and kinetic_force
- Examples: echolocation sweep, coordinated pack hunting, precise flinch response

**6. Mass Transfer** (move matter: nutrients, gametes, pathogens)
- Feeding mechanics, spore dispersal, larval transport, parasite transmission
- Interact with metabolic_rate and reproductive_rate
- Examples: nutrient absorption, pollen dispersal, vertical transmission

**7. Energy Modulation** (store, release, or redirect energy)
- Hibernation, adrenaline surge, temperature regulation feedback, bioluminescent burst
- Interact with metabolic_rate and thermal channels
- Examples: torpor entry, flight capacity surge, defensive flash

**8. Bond Formation** (create temporary or permanent links)
- Pheromone bonding (mating), symbiotic integration, web-spinning, nesting
- Interact with social, reproductive, and motor channels
- Examples: mate-pair bonding marker, parasitic immune tolerance, architectural material

### Primitive Genesis Operators (Sketch)

By analogy with channel genesis (Section 2B), primitives undergo:

**Duplication**: A primitive manifest is copied, assigned a new id (genesis:parent:generation), and registered. Composition_compatibility and parameters initially match the parent. The new primitive is immediately selectable by composition hooks.

**Divergence**: Over generations, parameter_schema bounds drift (mutation on min/max/default), observable_signature drift (effective range increases, modality changes), cost_function mutates (metabolic cost decreases through efficiency selection). Selection acts on creatures using the primitive; if a divergent primitive is more effective, it spreads.

**Reclassification**: A primitive's `category` may shift if its parameter drift and composition hooks suggest it now serves a different functional role. Example: a signal_emission primitive (emit_acoustic_pulse) mutates parameters and cost_function until it becomes indistinguishable from a force_application primitive (mechanical shockwave). Detection criteria: compare composition_compatibility and cost_function to category archetypes; reclassify if best match shifts.

**Loss**: A primitive with zero invocations over N generations (no composition hook references it, no interpreted ability uses it) is stochastically pruned. Reduces primitive registry bloat.

### Research Anchors

Primitive evolution is grounded in three biological phenomena:

**1. Exaptation (Gould & Vrba 1982)**
Traits evolve for one function, then are co-opted for another. A feather evolved for thermoregulation; exapted for flight. In our model, a primitive emitted for one channel composition (e.g., vibration for hearing) is exapted for another (e.g., vibration for attack via shockwave). Reclassification captures this.

**2. Promiscuous Enzyme Theory (Khersonsky & Tawfik 2010)**
Proteins evolve new functions through duplication and divergence. A metabolic enzyme (narrow specificity) is duplicated; one copy mutates to degrade a novel substrate (broad specificity, poor catalysis). This promiscuous binding is then refined by selection. Our primitives follow the same logic: duplicated primitives (initially identical to parent) are promiscuous; selection refines them via parameter drift.

**3. Modular Evolution (Wagner 2007)**
Complex systems evolve through modular rearrangement. Primitives are modules; composition hooks are connections. A new primitive can plug into an existing composition network without redesign (as long as it's compatible with channel families). This reduces evolutionary constraint.

### Determinism & State Snapshots

Primitive genesis, like channel genesis, is deterministic:
- Duplication rate, parameter drift mutation kernels, and reclassification thresholds are seeded
- Primitive registry snapshots are serialized alongside channel registry snapshots in save states
- Registry is unbounded (grows with gameplay), but all randomness is deterministic

### Open Questions (Future Revision)

1. **Mutation Rates**: How fast do primitive parameters drift relative to channel manifests? Should primitives mutate faster (they are "downstream") or slower (they are lower-level constraints)?

2. **Cost Function Drift**: Primitives have metabolic costs; as cost_function mutates, efficiency landscapes shift. Do creatures evolve toward cheap primitives, or is selection driven entirely by effectiveness? How do we avoid degenerate low-cost, high-effect primitives?

3. **Observable Signature Drift**: As observable_signature (range, detectability) drifts, the Chronicler's ability to recognize emergent patterns (System 09) may degrade. Does the system learn new patterns? Or do we cap signature drift within a "legibility corridor" to ensure gameplay clarity?

4. **Genesis Rates & Registry Bloat**: How many primitives should be in the registry by late game? Thresholds for loss (invocation count, generation age) need tuning to prevent explosion without over-pruning useful variants.

---

## 4. Cross-System Hooks

**To Phenotype Interpreter (System 11):**
- Outputs channel profile per body region
- References: "The game engine receives `resolved_channels` and applies interpreter rules to determine visuals and mechanics"
- Scale-band determines visual rendering: macro beasts have detailed organs; micro pathogens are schematic/abstract

**To Trait Registry (System 02):**
- All monsters (macro and micro) draw from the same channel manifest registry (core, mod, and genesis channels)
- Gene duplication and divergence create new channel manifest variants and may found new families
- Provenance tracking enables phylogenetic reconstruction and speciation metrics
- Primitive registry (parallel to channel registry) holds atomic effect manifests; primitives are emitted by composition hooks on channel threshold crossing
- Primitive genesis (future work) follows channel genesis operators; primitive registry snapshots join channel snapshots in save states

**To Faction/Social Model:**
- `neural_speed` channel affects sapience threshold
- High `neural_speed` beasts can participate in social layer
- Evolved beast traits become observable to NPCs, updating their `beast_knowledge`

**To Ecology / Trophic Dynamics (System 12):**
- Macro predation is represented as predators (other beasts, NPCs, player) culling prey
- Micro parasitism is represented as within-host population dynamics
- Host-parasite interactions are trophic edges in the food web: parasite draws resources from host; high virulence reduces host fitness
- Carrying capacity now accounts for both macro resource density and micro parasite load per host

**To Disease / Scale-Band Mechanics (System 16):**
- Evolutionary dynamics are scale-band agnostic; System 16 defines when parasites can infect, transmission mechanics, and timescale acceleration
- Fitness function (Section 4) accepts body_size_scale parameter and routes to appropriate channel weights
- Host-coupling channels are dormant at macro scale; activated at micro scale by System 16 logic

**To Economy/Ecology:**
- `resource_density` in biome cell is set by economy layer
- Player/NPC actions modify biome cells (farming, deforestation, construction)
- `carrying_capacity` is a function of `resource_density` (macro) and parasite load (micro)

---

## 5. Tradeoff Matrix

| Decision | Option A | Option B | Option C | Sim Fidelity | Implementability | Player Legibility | Choice & Why |
|---|---|---|---|---|---|---|---|
| **Trait Representation** | Named enum (Speed, Armor, Venom) | Channels + body sites | Procedural morphogenesis | High (opt C) | Low (opt C) | High (opt A) | **Channels (B)** — unbounded design space while remaining implementable. Enum caps novelty. Procedural morphogenesis is overkill complexity. |
| **Channel Genesis** | Fixed 18-channel registry | Gene duplication → new channels | Procedural registry expansion | High (opt B) | Medium (opt B) | Medium (opt B) | **Gene duplication (B)** — paralogs speciate into new functional channels without explicit "new channel creation" API. Emerges from duplication + divergence. Maintains tractability. |
| **Allometric Scaling** | Fixed 1.5 exponent | Kleiber's 3/4 law unified across scales | Tunable exponent per scale | High (opt B) | Medium (opt B) | Medium (opt A) | **Kleiber 3/4 unified (B)** — empirically grounded, works for elephants to bacteria. Eliminates need for separate pathogen metabolic model. |
| **Disease/Parasite Modeling** | Separate pathogen subsystem | Unified scale-band model | Scripted disease events | High (opt B) | Medium (opt B) | High (opt A) | **Unified scale-band (B)** — parasites evolve under identical fitness & mutation operators as hosts. Same registry, different body-size scale. Emerges from ecology, no separate machinery. |
| **Migration Logic** | Fixed immigration bonus (frustration valve) | Source-sink metapopulation | Complete ecosystem model | High (opt B/C) | Medium (opt B) | Low (all) | **Metapopulation (B)** — realistic and avoids "emergency relief" feeling. Simulates natural biogeography. |
| **Metabolic Cost** | Linear | 1.5 exponent | Allometric base + complexity tax | High (opt C) | Medium (opt C) | Low (opt C) | **Allometric + complexity (C)** — Kleiber for base (empirical), plus complexity tax for active traits (makes sense: more genes = more maintenance). |
| **Predation Pressure** | Fixed constant per biome | Empirical kill-rate tracking | Player-specific only | High (opt B) | High (opt B) | High (opt B) | **Empirical kill-rate (B)** — player pressure becomes a special case of general predator pressure. Pure simulation, no separate "anti-player" system. |
| **Reproductive Timing** | Fixed generation length | Poisson per individual | Age-based senescence | High (opt B) | Medium (opt B) | Low (opt C) | **Poisson per individual (B)** — matches real breeding asynchrony. Simpler than senescence. |
| **Network Cycles** | Ignore (allow infinity loops) | Fixed-point iteration | SCC damping | Medium (opt B/C) | Medium (opt B) | High (none!) | **SCC detection + damping (C)** — correct treatment of cyclic regulation. Biological networks do exhibit cycles; should resolve gracefully. |
| **Speciation Metric** | Trait set distance only | Phylogenetic + channel distance | Reproductive isolation | High (opt B/C) | Medium (opt B) | High (opt A) | **Phylo-tagged lineage (B)** — ancestors matter; evolution is tree-structured. Lineage tags make speciation meaningful. |
| **Family Reclassification Frequency** | Disabled (static 9 families) | Rare reclassification (threshold-driven) | Fluid family membership | High (opt C) | Medium (opt B) | Low (opt C) | **Rare reclassification (B)** — preserves family structure while allowing paralogs to shift roles. Families remain recognizable to players; emergence is rare enough to feel like evolution, not randomness. |
| **Primitive Evolution Timing** | Implement now (full operators) | Document structure, defer operators to future pass | No primitives at all | High (opt A) | Low (opt A) | Medium (opt B) | **Defer to future pass (B)** — Primitive registry structure and categories are stable (Section 4C). Genesis operators (duplication, divergence, reclassification, loss) are sketched and grounded in research (exaptation, promiscuous enzymes, modularity). Implementing full operators now blocks channel-registry completion; defer mutation kernels and reclassification thresholds to next revision after live testing. Primitives are evolvable by design; enabling evolution is future work. |

---

## 6. Emergent Properties

**Adaptive Radiation**: When a monster population colonizes a new biome, initial channel profiles are random. Selection rapidly culls unfit variants. Within 5-10 generations, distinct ecological morphs emerge without explicit guild assignment. Herbivores converge on chemical-sensing + low-defense, predators converge on kinetic-force + speed, etc.

**Coevolutionary Spirals**: If a dominant predator strategy (high kinetic_force, OnContact) eliminates all vulnerable prey, selection relaxes on prey defense channels. Later, when predator numbers crash from lack of prey, anti-predator strategies become rare and expensive. This creates population cycles of predator and prey without any explicit oscillation logic.

**Frequency-Dependent Persistence**: A common phenotype (say, high structural_rigidity) dominates until predators/players learn to exploit it (exploit weakness). Rare variants (high elastic_deformation) are ignored by predators and can increase. Frequency-dependent advantage emerges from the interpreter (predator strategy adapts to what it sees) without explicit "rare bonus" in the evolutionary model.

**Spatial Diversity**: Without global optimum (Axelrod principle), regional biomes select for distinct channel profiles. Mountain biomes favor high mass_density + thermal_resistance. Swamps favor elastic_deformation + chemical_resistance. Forests favor light_absorption + camouflage. No two regions converge to the same strategy because each has unique channel_fitness weights.

**Evolutionary Arms Races with Beasts**: When beasts are sapient enough to communicate with NPCs (neural_speed > 0.6), their observable traits update NPC beast_knowledge. NPCs develop hunting tactics. Monsters that evolve counter-tactics (e.g., high thermal_output against fire-wielding hunters) gain fitness. This is pure feedback: no "anti-player" system, just normal ecology with humans as apex predators.

---

## 6B. Calibration Targets & Telemetry (Issue #6)

### Mutation Rate Calibration Targets

**Goal**: The mutation system should produce observable evolutionary dynamics matching empirical timescales:
- **Phenotypic Variant Production**: A typical species under moderate selection produces **2–3 observable phenotypic variants per 10 generations** (variants distinguished by visibly different channel combinations).
- **Speciation Timescale**: New stable lineage emerges every ~**500 generations** in a given biome (measured as fixation of distinct channel profiles in isolated populations).

These targets ensure that evolution feels neither too slow (imperceptible change) nor too fast (unrealistic bursts of novelty).

### Required Telemetry Metrics

The simulation must expose the following metrics for tuning and observation:

1. **`mean_channel_diversity_per_species_per_biome`**: Average Shannon entropy of the 18-channel distribution across all organisms in a species in a biome. High value = high within-species diversity; low = specialized population.

2. **`new_lineages_per_1000_ticks`**: Count of newly emerged lineages (genetic distance from nearest ancestor > SPECIATION_THRESHOLD) per 1000 simulation ticks, reported per biome.

3. **`extinction_rate`**: Fraction of species present at tick T that are extinct by tick T+1000, reported per biome.

4. **`mean_fitness_trajectory`**: For a given species and biome, the mean fitness of the population at each generation, averaged over the last 100 generations. Used to detect convergence vs. ongoing selection.

### Primitive Genesis Operators (Channel Genesis) — Currently Disabled for v0

**Status**: Gene duplication, divergence, reclassification, and loss operators are fully documented in Section 2B but are **disabled for v0 balance**. The engine will instantiate duplications at baseline rates (1.0e-5) and track provenance, but the ecosystem has not been calibrated for the phenotypic explosion that genesis produces.

**Future Work (v1)**: Once telemetry confirms stable baseline evolution under the core 9 families, genesis operators will be re-enabled with calibrated rates. This allows for a two-stage rollout: validate mutation/selection in v0, then introduce family-level novelty in v1.

### Mutation Rate Tuning Knobs

The table below lists every mutation-rate parameter, its default value, and observable effect. Adjust these to match the calibration targets above.

---

## 7. Open Calibration Knobs

| Parameter | Current Value | Range | Effect | How to Tune |
|---|---|---|---|---|
| `BASE_RATE` (metabolic baseline) | 0.1 | 0.05–0.2 | Higher = shorter lifespans, faster reproduction, more pressure to specialize | Increase if evolution feels too slow; decrease if population explodes |
| `COMPLEXITY_COEFFICIENT` | 0.05 | 0.01–0.1 | Cost per active gene | Higher = monsters simplify (fewer genes). Use to prevent bloat. |
| `DIMINISHING_FACTOR` (global epistasis) | 0.7 | 0.5–0.9 | Second source of a channel contributes 70% of first | Lower = more extreme single-channel specialists. Higher = generalists encouraged. |
| `DUPLICATION_RATE` (genome-level) | 1.0e-5 | 1.0e-6 to 1.0e-4 | Per-generation probability of gene duplication | Lower = slower channel genesis, less novelty. Higher = rapid lineage expansion, more bloat. |
| `RECLASSIFICATION_THRESHOLD` | 0.5 | 0.3–0.8 | Manifest distance from original family before reclassification | Lower = more families emerge, higher organizational complexity. Higher = families remain stable. |
| `GENE_LOSS_RATE` | 1.0e-4 | 1.0e-5 to 1.0e-3 | Stochastic deletion of zero-contribution genes | Higher = leaner genomes, less redundancy. Lower = more pseudogenes, evolutionary flexibility. |
| `SPECIATION_THRESHOLD` | 0.4 | 0.2–0.6 | Genetic distance for species boundary | Higher = fewer species, longer before divergence. Lower = more species, faster speciation. |
| `PLAYER_ACTIVITY_HALF_LIFE` | 60 ticks | 20–200 ticks | Exponential memory of player activity | Shorter = faster adaptation to player's current tactics. Longer = slower, more "ecological" feel. |
| `FORMATION_PERSISTENCE` | 4 ticks | 1–10 ticks | Ticks a faction must remain stable to persist | Lower = faster fission. Higher = more stable large empires. |
| `Channel base costs` (per channel) | Varies (see code) | 0.001–0.1 | Metabolic cost to express a channel | Tune per-channel to prevent any one from dominating |
| `base_generation_ticks` | 50 | 20–100 | Baseline generation length (ticks) | Affects absolute speed of evolution. Increasing slows it down. |
| `MICRO_SCALE_SPEEDUP` | 1000 | 100–10000 | Generation time acceleration at micro scale | Higher = faster pathogen evolution relative to hosts. Tune for coevolutionary balance. |
| `MAX_PARASITE_PER_HOST` | 1000 | 100–10000 | Carrying capacity for parasites within a host | Lower = transmission-limited; higher = load-limited. Affects epidemic dynamics. |

### 7A. Mutation Rate Tuning Knobs (Issue #6 — Detailed Table)

| Mutation Type | Per-Locus Rate | Parameter Name | Observable Effect | Calibration Link |
|---|---|---|---|---|
| Point mutation (allele) | 1.0e-3 | `POINT_MUTATION_RATE` | Shift in channel magnitude N(0, 0.1). Higher rate = faster channel drift, more intraspecific variation. | Track: `mean_channel_diversity_per_species_per_biome` |
| Regulatory rewiring | 5.0e-4 | `REGULATORY_REWIRING_RATE` | Add/remove modifier connections. Higher = more complex regulatory networks. | Indirect: affects phenotypic plasticity |
| Channel shift | 5.0e-4 | `CHANNEL_SHIFT_RATE` | Shuffle channel contributions N(0, 0.15). Higher = broader phenotypic exploration. | Track: phenotypic variance per generation |
| Gene duplication | 1.0e-5 | `DUPLICATION_RATE` (mutable) | Copy gene → paralog with provenance. **Currently disabled for v0**. | Calibrate for v1: target `new_lineages_per_1000_ticks` |
| Gene divergence | per-generation | `DIVERGENCE_DRIFT_SIGMA` | Paralog manifest drifts at sigma per generation. | Monitor: paralog contribution divergence |
| Gene reclassification | 1.0e-5 | `RECLASSIFICATION_RATE` | Paralog reclassifies to new family if distance > threshold. **Currently disabled for v0**. | Calibrate for v1: family novelty rate |
| Deletion (Loss) | 1.0e-4 | `DELETION_RATE` | Stochastic removal of zero-contribution genes. Higher = leaner genomes. | Monitor: average genome size, deletion bias |
| Silencing toggle | 1.0e-3 | `SILENCING_TOGGLE_RATE` | Flip `enabled` on random gene. Higher = more regulatory variation. | Track: silent vs. expressed gene ratio |
| Body site shift | 1.0e-3 | `BODY_SITE_DRIFT_RATE` | Drift surface_vs_internal and body_region N(0, 0.1). | Monitor: body-site channel distribution |

**Tuning Strategy**: Start with defaults; monitor telemetry over 10,000 ticks. If `mean_channel_diversity_per_species_per_biome` is too low, increase point mutation and channel shift rates. If speciation is too fast (> 5 lineages per 1000 ticks), reduce duplication_rate when enabled in v1.

---

## 8. Notes on Implementation

**Performance Optimization:**
- Cache `resolve_network()` per monster; only recompute on mutation
- Pre-compute channel aggregation once per frame, not per decision
- Population updates can run on a slower tick (every 4–10 game ticks) than individual behavior
- Faction detection (section 5) runs every K=200 ticks, not every tick

**Debug Tools:**
- Channel profile visualizer: show each monster's 18-channel vector as a radar chart
- Speciation browser: show family tree, genetic distance heatmap
- Fitness landscape plotter: plot fitness as function of two salient channels for a given biome
- Mutation reporter: log all mutations, filterable by type and effect

**Content Pipeline:**
- The Phenotype Interpreter (Layer 3) is where designers author channel→behavior mappings
- Adding a new visual effect (glow, particles, deformation) is an interpreter change, not a model change
- Adding a new channel (if game expands to include new physics) is straightforward: increment NUM_CHANNELS, define CHANNEL_BASE_COST, add interpreter rules

---

## 9. Migration Notes

**From Previous Version (Fixed 18 Channels, Separate Pathogen System)**:

1. **Gene Duplication is Now First-Class**: The previous "duplication" operator (5.0e-5) is now expanded into a full gene duplication machinery with provenance tracking. Existing genomes are unaffected, but new duplications will mark paralogs with `provenance = "genesis:..."`. Backwards compatibility: old genes are treated as genesis generation 0 (no provenance).

2. **New Paralog Divergence Tracking**: The paralog struct gains two fields:
   - `provenance: string` (e.g., "genesis:channel_123:generation_456")
   - `generations_since_duplication: int`
   
   These enable tracking of neo-/subfunctionalization and speciation of new functional channels.

3. **Reclassification Logic**: Rare but implemented. Existing monsters' genes will never reclassify (probability is near zero over typical playthroughs). If a designer wants to seed a new family, they can manually set `paralog.family = NEW_FAMILY` and the model will respect it.

4. **Scale-Band Awareness in Fitness**: The `fitness_total()` function now checks `monster.body_size_scale` and routes to appropriate channel weights (macro vs. micro). Legacy calls to `fitness_total(monster, biome_cell)` continue to work; assume `body_size_scale = MACRO` if unspecified. Micro-scale fitness requires System 16 to instantiate micro monsters correctly.

5. **Multi-Scale Kleiber Scaling**: The metabolic cost function now uses mass^0.75 universally (unchanged from v1). Micro organisms are instantiated with mass ~1e-9 kg; their metabolic cost is therefore tiny (~1e-7) relative to body-size. This is intentional and correct: bacteria have lower absolute metabolic cost, not different scaling laws.

6. **Host-Coupling Channels**: Three new channels added to the registry (System 02):
   - `host_coupling_transmission` (channel 18)
   - `host_coupling_virulence` (channel 19)
   - `host_coupling_suppression` (channel 20)
   
   These are dormant (near-zero value) for macro monsters. At micro scale (System 16 logic), they become the primary fitness drivers. Old genomes have implicit zeros for these channels; no migration needed.

7. **Duplication Rate is Mutable**: `genome.params.duplication_rate` is now a parameter that evolves. This allows populations to evolve higher or lower duplication rates over time. Initial value: 1.0e-5. If existing genomes lack this field, initialize to 1.0e-5.

8. **System 02 Registry Updates**: Add `provenance` field to trait manifests. Add `genesis_weight` parameter to channel mutation_kernel. System 02 schema is now a prerequisite; see System 02 for details.

9. **System 16 Integration**: Parasitism and disease are no longer a separate subsystem but a scale-band mode of the same evolutionary engine. System 16 ("Disease as Scale-Band Evolution") now reads as "disease mechanics are scale-band manifestations of the evolutionary model in System 01." No changes to System 01 for System 16 compatibility; System 16 instantiates micro organisms via the standard genome-to-monster pipeline.

10. **Backward Compatibility**: Existing saves with macro monsters are fully compatible. Existing monsters' genomes will work unchanged. Duplication, divergence, reclassification, and loss apply only to new duplications and mutations post-load. No data migration needed.

**Key Ambiguities Resolved**:
- **Channel Genesis**: New channels emerge via gene duplication + divergence, not from a separate "new trait" system. Families remain the 9 core families; reclassification is rare enough (~1e-5) that it's an evolutionary event, not noise.
- **Unified Scaling**: Kleiber's law works identically at all scales; parasites and hosts share the same metabolic physics. Scale-band logic (System 16) determines expressibility of host-coupling channels, not metabolic differences.
- **No Separate Pathogen Logic**: Parasites, pathogens, commensals, and symbionts are all monsters instantiated at the micro scale. Host-coupling channel values determine the relationship (negative = virulence, positive = mutualism). Transmission and within-host dynamics emerge from standard fitness + population mechanics.
- **Primitive Evolution**: Primitives are the atomic vocabulary of phenotype output. The Primitive Registry (System 02) parallels the Channel Registry. Primitive manifests carry provenance, composition_compatibility, and observable_signature. Genesis operators (duplication, divergence, reclassification, loss) are deferred to future work but are architecturally committed. Primitive evolution is deterministic and seeded like channel genesis.

---

## 10. Migration Notes (Revision: Primitive-Genesis Operators)

**From Previous Version (Channel Genesis, No Primitives)**:

1. **Primitive Registry Introduction**: The Phenotype Interpreter now emits primitives (atomic effects) rather than named abilities. A new Primitive Registry is introduced (parallel to Channel Registry in System 02) with manifest schema defined in Section 4C. Existing gameplay unaffected; primitives are emitted by composition hooks at runtime.

2. **Composition Hooks Emit Primitives**: Channel manifests (System 02) gain an optional `emits` field listing primitives fired on threshold crossing, with parameter-mapping expressions (future work). Example: `{with: kinetic_force, kind: threshold, threshold: 0.6, emits: [{id: "apply_bite_force", params: {force: "kinetic_force * 2.0"}}]}`.

3. **Primitive Genesis Deferred**: Section 4C outlines primitive genesis operators (duplication, divergence, reclassification, loss) by analogy with channel genesis. Implementation is future work. The primitive registry is deterministic and will be seeded like channel genesis when operators are implemented. No immediate code changes required.

4. **Open Questions Documented**: Mutation rates, cost-function drift, observable-signature drift, and registry-bloat thresholds are identified as future research (Section 4C, "Open Questions"). These will inform the next revision when primitive evolution is enabled.

5. **Backward Compatibility**: Existing monsters and channels are unaffected. The primitive registry is initialized with core primitives (to be defined in System 02 and System 11); new primitives are registered on demand. Save states now include a primitive registry snapshot alongside the channel registry snapshot.

---
