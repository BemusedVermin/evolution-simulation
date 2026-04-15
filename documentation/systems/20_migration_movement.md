# System 20: Population Migration & Macro-Movement

## 1. Overview

Migration captures macro-scale population flows: species range expansion, seasonal creature movements, human settlement relocation, and refugee dynamics. This system operates at population and settlement granularity, not individual movement (which is handled by pathfinding). It couples settlement carrying capacity (System 04), ecological resources (System 12), climate (System 15), conflict (System 03), and genetics (System 01, metapopulation dynamics).

**Beast Migration** is driven by:
- **Dispersal rates** per species (function of mobility channel from System 01 + habitat suitability from System 12).
- **Seasonal migrations**: creatures move following resource availability (System 12 vegetation cycles).
- **Range expansion/contraction**: species expand into favorable habitats; contract during unfavorable periods.
- Population abundance tracking at the metapopulation level (per biome cell or region).

**Human Settlement Migration** is driven by:
- **Settlement stress**: resource shortage, disease, conflict.
- **Pull factors**: better settlement prospects, lower conflict, available land.
- **Decision process**: settlement council (faction leader + agents) evaluates relocation need; if need exceeds threshold, migration event is triggered.
- **Refugee dynamics**: military defeats force population flight; receiving settlements incur crowding costs.

**Founder Events** (small migrant groups establishing new settlements) create population bottlenecks that amplify genetic drift in creature species and cultural trait loss in humans (System 18 Tasmanian effect).

Cross-coupling: migration from conflict drives genetic flow (species interbreeding at new locations), technology/language/cultural trait diffusion as factions merge, and trade route formation (successful migration corridors become trade paths, System 04).

**Key principle**: Populations are not teleported; they move via corridors defined by terrain and safety. A migration event takes ticks proportional to distance × difficulty, during which the moving population is vulnerable to predation and starvation.

---

## 2. Research Basis

### Dispersal & Range Expansion (Lomolino, 2005; Shigesada & Kawasaki, 1997)
Species expand their range via diffusion—individuals move into adjacent suitable habitats. Dispersal rate depends on mobility and habitat suitability. The "velocity of range expansion" (km/year) is empirically predictable from dispersal rate and generation time. Species with good dispersal (e.g., flying insects, fast-moving vertebrates) expand rapidly; poor dispersers (e.g., earthworms, amphibians) spread slowly.

- Lomolino, M.V. (2005). *Biogeography* (3rd ed.). Sinauer.
- Shigesada, N. & Kawasaki, K. (1997). *Biological Invasions: Theory and Practice*. Oxford University Press.

**Application**: Each creature species has a dispersal_rate (frac of population per tick that disperses to adjacent cells). Habitat suitability (from System 12 carrying capacity) modulates dispersal: creatures leave poor habitats faster. Range expansion creates visible range maps in the world history (System 09).

### Seasonal Migration (Dingle & Drake, 2007; Wilcove & Wikelski, 2008)
Animals migrate seasonally to track resources (e.g., herbivores following vegetation growth, predators following prey). Migration is phenotypically plastic but triggered by reliable environmental cues (day length, temperature, rainfall). Round-trip migrations involve high mortality risk but access to seasonal abundance.

- Dingle, H. & Drake, V.A. (2007). "What Is Migration?" *BioScience*, 57(2).
- Wilcove, D.S. & Wikelski, M. (2008). "Going, Going, Gone." *Nature*, 432.

**Application**: Each species has a migration_trigger and migration_route based on seasonal climate shifts (System 15, calendar System 14). Creatures leave a biome cell when resources drop below threshold; return when resources recover. This is encoded in creature behavior (System 06 AI) and tracked at population scale in System 12.

### Human Migration & Settlement Dynamics (Lee, 1966; Ravenstein, 1889; Boyd & Richerson, 1985)
Migration is driven by push (harsh conditions) and pull (opportunity) factors. Lee's push-pull model quantifies that migration probability is proportional to dissatisfaction_at_origin × attractiveness_of_destination. People migrate in groups (families, clans) rather than individuals. Historical studies show that migration distance follows gravity models (decreases with distance squared).

- Lee, E.S. (1966). "A Theory of Migration." *Demography*, 3(1).
- Ravenstein, E.G. (1889). "The Laws of Migration." *Journal of Statistical Society*, 52.
- Boyd, R. & Richerson, P.J. (1985). *Culture and the Evolutionary Process*. University of Chicago Press.

**Application**: Settlement migration is triggered when settlement stress exceeds threshold (resource shortage, disease, conflict). Agents score destination attractiveness (available land, lower conflict, better resources). Migration route is chosen via path-of-least-resistance (terrain, enemies). Small migrant group (founder event) arrives at destination; if population too low, founding settlement forms (bottleneck drift).

### Founder Events & Bottleneck Effects (Mayr, 1954; Carson, 1968; Nei et al., 1975)
Small founding populations experience rapid genetic drift: rare alleles may be lost by chance; common alleles may become fixed. Effective population size (Ne) in founder groups is much smaller than census size. Over a few generations, founder populations diverge genetically despite ongoing gene flow from source population. Cultural traits show similar bottlenecking: small populations lose rare variants rapidly (Tasmanian effect).

- Mayr, E. (1954). "Change of Genetic Environment and Evolution." In *Evolution as a Process*.
- Carson, H.A. (1968). "The Population Flush and Its Genetic Consequences." In *Population Biology and Evolution*.
- Nei, M., et al. (1975). "The Bottleneck Effect and Genetic Variability in Populations." *Evolution*, 29(1).

**Application**: When a small migrant group establishes a new settlement, apply founder effect: randomly remove ~50% of rare alleles (frequency < 0.2) from creature populations in the new location; cultural traits with low transmission_confidence (System 18) have 50% chance of being lost. Founding genetic drift is visible in chronicles (System 09) and affects evolution trajectories.

### Metapopulation Dynamics (Levins, 1970; Hanski, 1998)
A metapopulation is a network of populations in distinct habitat patches, connected by dispersal. Dynamics: patches with high carrying capacity are "sources" (produce surplus emigrants); poor patches are "sinks" (sustained only by immigration). Interconnected networks (high dispersal rate) show synchronized fluctuations; isolated populations fluctuate independently.

- Levins, R. (1970). "Extinction." In *Some Mathematical Questions in Biology*.
- Hanski, I. (1998). "Metapopulation Dynamics." *Oxford University Press*.

**Application**: Track creature abundance in a coarse grid (e.g., each biome region). Dispersal from high-abundance regions to low-abundance regions occurs each tick. Settlement populations are also modeled as metapopulation sources/sinks.

### Refugee Dynamics & Forced Migration (Lischer, 2005; Oliver-Smith, 2004)
Military defeat causes sudden forced migration. Refugees flee to allied or neutral territory, straining receiving communities' carrying capacity. Refugees may establish new settlements if land is available. Historical example: Armenian Genocide forced migration; receiver communities experienced crowding-driven famine.

- Lischer, S.K. (2005). "The Global Spread of Ethnic Conflict." *Princeton University Press*.
- Oliver-Smith, A. (2004). "Involuntary Relocation, Livelihood, and Traumatic Stress." In *Risks and Reconstruction*.

**Application**: When a settlement is defeated in combat (System 06), surviving population flees. They are shunted to nearest allied settlement, increasing carrying capacity load. Receiving settlement must absorb refugees or faces starvation. This creates cascading crises: one lost war can trigger famine in multiple settlements.

---

## 3. Entities & State

### Creature Population at Biome/Region Level

```
CreaturePopulationByRegion {
  species_id: int,
  region_id: int,  // Biome cell or larger region (from System 07 exploration)
  
  // Population
  census_size: int,            // Number of individuals
  effective_population_size: int,  // For genetic drift calculation (usually 0.1-0.2 × census)
  
  // Genetic composition (simplified)
  allele_frequency_map: {       // Tracks common alleles per locus
    [locus_id]: {
      allele: string,
      frequency: float [0, 1],
    }
  },
  
  // Dynamics
  recent_births: int,           // Rolling 10-tick average
  recent_deaths: int,
  recent_immigration: int,      // From adjacent regions
  recent_emigration: int,       // To adjacent regions
  
  // Range & Habitat
  habitat_suitability: float [0, 1],  // From System 12 (carrying capacity)
  is_expanding_range: bool,
  is_contracting_range: bool,
  year_range_expanded_tick: int or null,
  
  // Migration
  in_seasonal_migration: bool,
  migration_destination_region_id: int or null,
  migration_duration_ticks_remaining: int,
}
```

### Settlement Migration State

```
Settlement.migration_state = {
  current_state: enum {
    Settled,           // Normal operations
    MigrationPlanned,  // Decision made; logistics underway
    InMigration,       // Population is moving
    ArrivalImminent,   // Will arrive next tick
  },
  
  destination_settlement_id: int or null,   // If migrating
  migration_origin_settlement_id: int or null,  // Source
  origin_coordinates: (x, y),
  destination_coordinates: (x, y),
  
  // Movement progress
  migrating_population_count: int,  // People on the road
  migration_duration_ticks: int,     // Total ticks for full journey
  migration_ticks_elapsed: int,      // Ticks completed
  
  // Risks during migration
  attrition_rate_per_tick: float,     // Deaths en route
  supplies_carried: [MaterialStack, ...],
  
  // Refugee-specific
  is_refugee_migration: bool,
  source_conflict_settlement_id: int or null,
}
```

### Settlement Stress & Migration Decision

```
Settlement.migration_need = {
  resource_stress: float [0, 1],    // (deficit / carrying_capacity)
  disease_stress: float [0, 1],     // (sick_count / population)
  conflict_stress: float [0, 1],    // (enemy_proximity + recent_losses / morale)
  
  total_push_score: float,          // Weighted sum of stresses
  migration_threshold: float,       // ~0.6; if total_push_score > threshold, migration triggered
  
  // Candidate destinations
  evaluated_destinations: [
    {
      settlement_id: int,
      distance: float,
      attractiveness_score: float,  // Resources + security − distance
      faction_relationship: enum { Allied, Neutral, Hostile },
    }
  ],
  preferred_destination_id: int or null,
  
  last_migration_attempt_tick: int,
  migration_cooldown_ticks: int,    // Prevent thrashing; must wait 100 ticks between attempts
}
```

### Founder Event (New Settlement from Migration)

```
FounderEvent {
  founding_tick: int,
  source_settlement_id: int,
  source_faction_id: int,
  
  founder_population_count: int,  // Small group (5–30% of source)
  founder_genetic_sample: {       // Gene frequencies from source (bottlenecked)
    creature_populations: [CreaturePopulationByRegion, ...],  // Reduced diversity
  },
  founder_cultural_sample: {      // Traits + technologies from source (probabilistically lost)
    technologies: [tech_id, ...],
    cultural_traits: [trait_id, ...],
    languages: [language_id, ...],
  },
  
  founding_location: (x, y),
  new_settlement_id: int,          // ID of established settlement
}
```

---

## 4. Update Rules

### Creature Dispersal & Range Expansion

Each tick, populations disperse to adjacent regions:

```
function disperse_creature_population(pop: CreaturePopulationByRegion, tick: int):
  species = get_creature_species(pop.species_id)
  
  // Dispersal rate depends on mobility (System 01) and habitat dissatisfaction
  base_dispersal_rate = species.channels[mobility] * 0.001  // Mobility channel → dispersal
  
  // Habitat suitability modulates: poor suitability increases leaving rate
  habitat_dissatisfaction = 1.0 - pop.habitat_suitability
  effective_dispersal_rate = base_dispersal_rate * (1.0 + 3.0 * habitat_dissatisfaction)
  
  emigrants = int(pop.census_size * effective_dispersal_rate)
  
  // Distribute emigrants to adjacent regions with higher suitability
  for adjacent_region in get_adjacent_regions(pop.region_id):
    adjacent_pop = get_creature_population(pop.species_id, adjacent_region)
    adjacent_suitability = adjacent_pop.habitat_suitability
    
    if adjacent_suitability > pop.habitat_suitability:
      // Higher suitability attracts emigrants
      attraction_factor = adjacent_suitability - pop.habitat_suitability
      emigrant_fraction = effective_dispersal_rate * attraction_factor / sum(all_attractions)
      emigrant_count = int(emigrants * emigrant_fraction)
      
      transfer_population(pop, adjacent_pop, emigrant_count)
      adjacent_pop.recent_immigration += emigrant_count
```

### Seasonal Migration Trigger

When climate conditions shift (System 14, 15), creatures migrate:

```
function evaluate_seasonal_migration(pop: CreaturePopulationByRegion, tick: int):
  species = get_creature_species(pop.species_id)
  
  // Check if seasonal migration is beneficial
  current_carrying_capacity = pop.habitat_suitability * MAX_CARRYING_CAPACITY[species]
  nearby_better_region = find_best_seasonal_destination(species, tick)
  
  if nearby_better_region and nearby_better_region.carrying_capacity > current_carrying_capacity * 1.5:
    // Migration is worthwhile
    pop.in_seasonal_migration = true
    pop.migration_destination_region_id = nearby_better_region.region_id
    pop.migration_duration_ticks_remaining = distance(pop.region_id, nearby_better_region) * 5
    
    // Partial population migrates (not all; some stay)
    migration_fraction = 0.6  // 60% of population migrates
    migrants = int(pop.census_size * migration_fraction)
    pop.recent_emigration += migrants
    // Migrants will arrive at destination next tick or after travel time
```

### Human Settlement Migration Decision

Each tick, settlements with high stress evaluate migration:

```
function evaluate_settlement_migration_need(settlement: Settlement, tick: int):
  settlement.migration_need.resource_stress = max(0.0, 
    settlement.total_deficit / settlement.carrying_capacity
  )
  
  settlement.migration_need.disease_stress = (
    settlement.sick_population_count / settlement.population_count
  )
  
  settlement.migration_need.conflict_stress = (
    settlement.enemy_threat_proximity_factor +
    settlement.recent_military_casualties / settlement.population_count
  )
  
  settlement.migration_need.total_push_score = (
    0.4 * settlement.migration_need.resource_stress +
    0.3 * settlement.migration_need.disease_stress +
    0.3 * settlement.migration_need.conflict_stress
  )
  
  // Evaluate destinations
  if settlement.migration_need.total_push_score > settlement.migration_need.migration_threshold:
    candidate_destinations = find_candidate_destinations(settlement, settlement.faction)
    
    for dest in candidate_destinations:
      distance = path_distance(settlement.location, dest.location)
      
      dest.attractiveness_score = (
        (1.0 - dest.carrying_capacity_utilization) * 0.4 +  // Available land
        (1.0 - dest.enemy_threat) * 0.4 +                    // Safety
        (1.0 - (distance / MAX_MIGRATION_DISTANCE)) * 0.2    // Proximity
      )
    
    settlement.migration_need.evaluated_destinations = candidate_destinations
    best_dest = max(candidate_destinations, key=lambda d: d.attractiveness_score)
    settlement.migration_need.preferred_destination_id = best_dest.settlement_id

function execute_settlement_migration(settlement: Settlement, destination: Settlement, tick: int):
  // Sanity check: not migrated recently
  if tick - settlement.migration_need.last_migration_attempt_tick < settlement.migration_need.migration_cooldown_ticks:
    return
  
  // Partial population migrates
  migrant_count = int(settlement.population_count * 0.4)  // 40% migrate
  
  // Calculate travel time
  distance = path_distance(settlement.location, destination.location)
  travel_duration = int(distance / 2.0) + 5  // Distance/2 + base time
  
  settlement.migration_state.current_state = MigrationPlanned
  settlement.migration_state.destination_settlement_id = destination.settlement_id
  settlement.migration_state.migrating_population_count = migrant_count
  settlement.migration_state.migration_duration_ticks = travel_duration
  settlement.migration_state.migration_ticks_elapsed = 0
  
  // Remove from source settlement
  settlement.population_count -= migrant_count
  settlement.migration_need.last_migration_attempt_tick = tick

function tick_settlement_in_migration(settlement: Settlement, destination: Settlement, tick: int):
  state = settlement.migration_state
  state.migration_ticks_elapsed += 1
  
  // Attrition: starvation, predation en route
  attrition_rate = 0.02 + terrain_hazard(current_path)  // 2% base + terrain hazard
  casualties = int(state.migrating_population_count * attrition_rate)
  state.migrating_population_count -= casualties
  
  if state.migration_ticks_elapsed >= state.migration_duration_ticks:
    // Arrival
    state.current_state = ArrivalImminent
    state.migration_ticks_elapsed = 0  // Reset for arrival tick

function complete_settlement_migration(settlement: Settlement, destination: Settlement, tick: int):
  state = settlement.migration_state
  
  // Add migrants to destination
  destination.population_count += state.migrating_population_count
  destination.carrying_capacity_utilization = destination.population_count / destination.carrying_capacity
  
  // Check if new settlement should be founded (if migrants don't fit)
  if destination.carrying_capacity_utilization > 0.95 and state.migrating_population_count > 5:
    // Found new settlement instead
    trigger_founder_event(
      source_settlement_id=settlement.settlement_id,
      founding_location=destination.location + random_offset(),
      founder_population=state.migrating_population_count
    )
  
  state.current_state = Settled
  state.destination_settlement_id = null
  state.migrating_population_count = 0
```

### Refugee Migration (Forced by Military Defeat)

When a settlement is defeated in System 06:

```
function trigger_refugee_migration(defeated_settlement: Settlement, tick: int):
  // Estimate survivor population (survivors flee)
  survivor_population = int(defeated_settlement.population_count * 0.6)  // 40% killed, 60% flee
  
  // Find nearest allied settlement
  allied_candidates = [
    s for s in defeated_settlement.faction.settlements
    if s.settlement_id != defeated_settlement.settlement_id
  ]
  
  if not allied_candidates:
    // No allied settlements; refugees disperse (some lost, some form new settlement)
    return
  
  nearest_ally = min(allied_candidates, key=lambda s: distance(defeated_settlement, s))
  
  # Create refugee migration event
  migration_state = Settlement.migration_state
  migration_state.current_state = MigrationPlanned
  migration_state.destination_settlement_id = nearest_ally.settlement_id
  migration_state.migrating_population_count = survivor_population
  migration_state.is_refugee_migration = true
  migration_state.source_conflict_settlement_id = defeated_settlement.settlement_id
  
  # Refugees bypass normal travel time (fleeing urgency)
  migration_state.migration_duration_ticks = int(distance(...) / 4.0)  // Faster escape
  
  # Record population loss
  defeated_settlement.population_count = defeated_settlement.population_count * 0.1  // Garrison remains
```

### Founder Event Triggering & Bottleneck

When a migrant group arrives at a region with no established settlement:

```
function trigger_founder_event(source_settlement_id: int, founding_location: (x, y), founder_population: int, tick: int):
  source = get_settlement(source_settlement_id)
  source_faction = source.faction
  
  # Create new settlement
  new_settlement = Settlement(
    name = generate_settlement_name(source_faction),
    faction_id = source_faction.faction_id,
    location = founding_location,
    population_count = founder_population,
    parent_settlement = source_settlement_id,
  )
  
  # Bottleneck genetic diversity in creature populations
  for creature_species in world.creatures:
    source_pop = get_creature_population(creature_species.species_id, source.biome_region_id)
    
    # Founder effect: reduce rare alleles
    new_pop = CreaturePopulationByRegion(
      species_id = creature_species.species_id,
      region_id = new_settlement.biome_region_id,
      census_size = int(source_pop.census_size * 0.05),  // Few founders
      effective_population_size = int(census_size * 0.1),  // Very small Ne
    )
    
    for locus in source_pop.allele_frequency_map:
      for allele in locus.alleles:
        freq = allele.frequency
        
        # Bottleneck: rare alleles likely lost, common alleles likely retained
        if freq < 0.2 and random() > freq:
          # Rare allele lost in founder population
          pass
        else:
          new_pop.allele_frequency_map[locus][allele] = freq + random(−0.1, 0.1)
  
  # Bottleneck cultural traits
  founder_event = FounderEvent(
    source_settlement_id = source_settlement_id,
    source_faction_id = source_faction.faction_id,
    founder_population_count = founder_population,
    founding_location = founding_location,
    new_settlement_id = new_settlement.settlement_id,
  )
  
  for trait in source_faction.cultural_traits:
    if random() < trait.transmission_confidence * 0.5:  // 50% loss rate in founder event
      founder_event.founder_cultural_sample.cultural_traits.append(trait)
  
  for tech in source_faction.technologies:
    if random() < 0.8:  // 80% tech retention (most knowledge retained)
      founder_event.founder_cultural_sample.technologies.append(tech)
  
  new_settlement.founding_event = founder_event
  chronicle_founder_event(founder_event, tick)
```

---

## 5. Cross-System Hooks

**System 01 (Evolution)**: Dispersal from overcrowded populations creates gene flow. Founder events trigger bottleneck drift. Range expansion into new biomes exposes populations to new selection pressures (System 12). Dispersal rate is modulated by mobility channel (System 01).

**System 03 (Faction/Social)**: Migration between settlements diffuses faction culture and language (Systems 18, 19). Founder events create new settlements that may eventually form independent factions (if isolated long enough). Refugee dynamics affect faction relationships (receiving factions incur costs; sending factions owe political debt).

**System 04 (Economy)**: Refugee population strains carrying capacity, triggering shortages and price spikes. Migration corridors become trade routes as successful paths are repeatedly used.

**System 06 (Combat)**: Military defeat triggers refugee migration. Large conquering armies may move to occupy new territory (settlement migration). Defeated creatures (invasive species from System 01) may withdraw from regions via dispersal.

**System 12 (Ecology)**: Habitat suitability (carrying capacity) drives creature dispersal and migration. Seasonal resource fluctuations trigger seasonal migration. Population abundance at per-region granularity feeds evolutionary fitness in System 01.

**System 13 (Lifecycle)**: Offspring born in new settlements inherit founder effects (genetic drift). Migration during pregnancy may affect newborn survival. Migration events consume ticks, delaying reproduction and economic output.

**System 14 (Calendar)**: Seasonal triggers (day-length, temperature) activate seasonal migration in System 14 calendar logic. Migration events are recorded in calendar (e.g., "Year 215: The Sacer Herd migrated north" appears in chronicle).

**System 15 (Climate/Biome)**: Climate shifts cause range contraction/expansion. Resource density drives creature dispersal. Seasonal climate patterns trigger seasonal migration.

**System 18 (Language & Culture)**: Founder events are bottleneck events for language/trait transmission. Isolated populations develop dialectal divergence. Refugees introduce new languages/traits to receiving settlements.

**System 20 (This system)**: Interconnected migrations create metapopulation dynamics. Successful trade routes correlate with successful migration corridors.

---

## 6. Tradeoff Matrix

| Dimension | Choice | Rationale |
|---|---|---|
| **Creature Migration Granularity** | Per-individual vs. per-population | Per-population is faster; per-individual is detailed. Chosen: per-population with emergent per-individual behavior from differential death rates. |
| **Settlement Migration Cost** | Attrition en route vs. instant teleport | Attrition creates risk/reward; instant is fast. Chosen: attrition (casualties en route). |
| **Bottleneck Severity** | Extreme (50% loss of rare alleles) vs. mild (10% loss) | Extreme creates visible genetic divergence; mild is conservative. Chosen: 50% loss rate for rare alleles (realistic founder effect). |
| **Refugee Cooldown** | Strict (100-tick cooldown) vs. flexible (threshold-based) | Strict prevents thrashing; flexible is more dynamic. Chosen: strict (prevents pathological settling/resettling). |
| **Founder Population Size** | Fixed (e.g., 5% of source) vs. variable (proportional to destination available space) | Fixed is predictable; variable is adaptive. Chosen: variable (if destination has space, more settle; else smaller founder event). |
| **Destination Evaluation** | Long-distance (entire world) vs. local (adjacent 5 cells) | Long-distance is more realistic (scouts find opportunities); local is faster. Chosen: medium (up to 20-cell distance). |

---

## 7. Emergent Properties

- **Metapopulation Dynamics**: Creature populations show synchronized cycles in connected regions; isolated populations fluctuate independently. Ecosystems with good dispersal (low barriers) are homogeneous; fragmented ecosystems are diverse.

- **Genetic Differentiation**: Small isolated populations (founder effect) diverge rapidly from source populations in allele frequencies. Over multiple generations, isolated populations may speciate. Players notice creatures in remote islands are distinctly different.

- **Cultural Divergence via Founder Effect**: Small migrant groups lose rare cultural traits, creating divergence from source faction. A settled colony may lose the source faction's taboos, developing independent culture. After 50+ years, the colony may be culturally distinct and form its own faction.

- **Refugee Cascades**: One defeat forces refugees into allied settlements; overcrowding triggers starvation in receiving settlements; starvation causes those settlements to migrate, triggering cascading crises across the map. A single lost war can unravel a faction's network.

- **Trade Route Formation**: Successful migration corridors become trade routes. Players notice that the primary trade path matches the historical migration path, creating a visible relationship between human movement and economic flow.

- **Invasive Species Dynamics**: A creature species expands its range into new biomes; it outcompetes native species and establishes dominance. Players can track the "invasion wave" of a new predator species as it disperses across the world. System 01 evolution allows adaptation; System 20 allows range expansion—together they create biological invasions.

- **Strategic Relocation**: A player faction can influence migration by improving settlement conditions, attracting rival factions' refugees, or increasing rival factions' stress (via conflict or resource denial). Macro-scale population management becomes a strategic lever.

---

## 8. Open Calibration Knobs

- **BASE_DISPERSAL_RATE**: Base dispersal fraction per tick (currently 0.001 × mobility channel). Increase for faster range expansion; decrease for slower.

- **HABITAT_DISSATISFACTION_MULTIPLIER**: How much poor habitat increases emigration (currently 3.0). Increase to make creatures leave bad habitats more readily; decrease for stubbornness.

- **MIGRATION_THRESHOLD**: Settlement stress threshold for triggering migration decision (currently 0.6 out of 1.0). Increase for reluctance to migrate; decrease for eagerness.

- **MIGRANT_FRACTION**: Fraction of settlement population that migrates (currently 0.4 or 40%). Increase for larger migrations; decrease for smaller.

- **MIGRATION_ATTRITION_RATE_BASE**: Base death rate en route (currently 0.02 or 2% per tick). Increase to make migration dangerous; decrease for safer travel.

- **REFUGEE_SPEED_MULTIPLIER**: How much faster refugees travel vs. normal migrants (currently 4.0, so 1/4 the travel time). Increase for faster refugee escape; decrease to make refugees slow.

- **FOUNDER_POPULATION_FRACTION**: Fraction of source settlement that can found new settlement (currently 0.4 or 40%). Increase to allow larger founder events; decrease for smaller.

- **BOTTLENECK_RARE_ALLELE_LOSS_RATE**: Fraction of rare alleles (freq < 0.2) lost in founder event (currently 0.5 or 50%). Increase for severe bottlenecking; decrease for milder.

- **MIGRATION_COOLDOWN_TICKS**: Ticks before settlement can attempt migration again (currently 100). Increase to prevent thrashing; decrease for more dynamic movement.

