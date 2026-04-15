# System 12: Ecology & Ecosystem Dynamics

## 1. Overview

The Ecology system models energy flow, trophic dynamics, and population-carrying capacity across spatial cells. It is the ENGINE that drives evolutionary fitness in System 01. Every cell tracks:
- **Trophic structure**: autotrophs (photosynthetic creatures, vegetation), primary consumers, secondary consumers, tertiary consumers, decomposers.
- **Energy pyramid**: calories flowing from producers → herbivores → carnivores, with realistic ~10% transfer efficiency.
- **Population-level carrying capacity** per species per cell: K emerges from prey availability, space, and metabolic demand.
- **Niche partitioning**: species coexistence when they occupy distinct ecological niches (diet, microhabitat, foraging time).
- **Keystone species detection**: species whose removal disproportionately collapses the local food web.
- **Prey/predator availability**: each cell tracks abundances of viable prey species for each consumer, used by predation AI and mate search in System 13.

This system is **read heavily by System 01 (Evolution)**, which uses carrying_capacity to compute fitness. It is **updated by System 13 (Reproduction & Lifecycle)** whenever births/deaths occur. It reads climate and biome distribution from System 15.

---

## 2. Research Basis

### Trophic Dynamics & Energy Flow
- **Lotka-Volterra predator-prey cycles** (Lotka 1925, Volterra 1926): Classic two-species oscillations; we generalize to food webs using the **Rosenzweig-MacArthur model** (1963), which includes carrying capacity and intraspecific competition.
- **Lindeman's energy law** (1942): ~10% of energy transfers between trophic levels; rest lost to respiration and heat.
- **Hutchinson's niche concept** (1957): species coexist if their realized niches differ sufficiently in resource axes (diet breadth, microhabitat, temporal activity).
- **MacArthur niche overlap model** (1970): overlap coefficient predicts coexistence vs. competitive exclusion.

### Carrying Capacity & Population Regulation
- **Logistic growth model**: dN/dt = rN(1 – N/K). K is limited by available resources (prey biomass, space, shelter).
- **Dietary breadth & specialist vs. generalist**: specialists have high fitness in their niche but low K when prey becomes scarce; generalists have lower maximum fitness but higher K resilience.

### Keystone Species Detection
- **Paine's starfish experiment** (1966): removal of Pisaster sea star collapsed rocky intertidal diversity.
- **Keystoneness metric**: we compute as (diversity_loss | species removed) / (baseline diversity). Species with high keystoneness deserve special attention in lore (System 09).

### Ecosystem Stability
- **Diversity-stability hypothesis** (MacArthur 1955): ecosystems with higher species diversity are more stable; corroborated by work on network topology (May 1972, stability depends on dimensionality and connectance).

---

## 3. Entities & State

### Per-Cell Ecology Data (attached to exploration cell from System 07)

```
Cell.ecology = {
  // Trophic energy pools (kCal per cell per tick)
  energy_photosynthesis: float,    // Solar input; depends on latitude, season (System 15)
  energy_chemosynthesis: float,    // Geothermal at volcanic cells
  
  // Species abundances in this cell
  species_populations = {
    [species_id]: {
      count: int,                                  // Number of individuals
      total_biomass_kg: float,                    // Sum of all individual masses
      average_mass_kg: float,                    // For quick per-capita metabolic demand
      trophic_level: float,                      // Computed: 1.0 = autotroph, avg of diet
      diet_composition: Dict[species_id, float], // Fraction of diet from each prey species
      niche_overlap_score: Dict[species_id, float], // Hutchinson overlap vs. each other species
      recent_birth_rate: float,                  // Births per tick, 10-tick rolling avg
      recent_death_rate: float,                  // Deaths per tick, 10-tick rolling avg
      recent_predation_loss: float,              // Deaths to predators per tick
      recent_starvation: float,                  // Deaths to hunger per tick
    }
  },
  
  // Vegetation (non-creature autotrophs)
  vegetation = {
    total_calories: float,         // Renewable pool, regenerates each tick
    regeneration_rate: float,      // kCal/tick; driven by climate (System 15)
  },
  
  // Decomposer activity
  decomposer_efficiency: float,  // Fraction of dead biomass recycled into next tick's vegetation
  
  // Derived metrics (recomputed each tick)
  trophic_structure: {
    autotrophs: [species_id, ...],
    primary_consumers: [species_id, ...],
    secondary_consumers: [species_id, ...],
    tertiary_consumers: [species_id, ...],
  },
  
  keystoneness: Dict[species_id, float], // Computed via removal simulation
  ecosystem_resilience: float,   // Eigenvalue of community matrix; >0 = stable
  ecosystem_diversity: float,    // Shannon entropy of species biomass
}
```

### Global Ecology Constants

```
ENERGY_TRANSFER_EFFICIENCY = 0.10   // ~10% from one trophic level to next
BASAL_METABOLIC_RATE_MULTIPLIER = 0.75  // Allometric scaling (West et al. 1997)
PREDATION_SEARCH_EFFICIENCY = 0.8   // Fraction of time predator successfully hunts in optimal cell
STARVATION_THRESHOLD_DAYS = 14      // Individual dies if unfed for >14 ticks
VEGETATION_REGENERATION_BASE = 100.0 // kCal/tick baseline; multiplied by climate biome fertility
```

---

## 4. Update Rules

### Each Tick: Energy Flow & Population Dynamics

**Phase 1: Primary Production**
```
For each cell:
  If cell.biome in [FOREST, GRASSLAND, OCEAN_SHALLOW]:
    photosynthesis_calories = 
      VEGETATION_REGENERATION_BASE 
      * latitude_insolation_factor(latitude, season)  // From System 15
      * biome_productivity(cell.biome)                // Lookup table
      * (1 + 0.1 * cell.ecology.ecosystem_diversity)  // Diversity bonus
    
    vegetation.total_calories += photosynthesis_calories
```

**Phase 2: Herbivore Consumption & Metabolic Cost**
```
For each species s in cell where trophic_level[s] ≈ 2.0 (primary consumers):
  For each individual creature c in s:
    metabolic_demand = basal_metabolic_rate(c.mass) * c.channels.metabolic_rate
    
    If vegetation.total_calories >= metabolic_demand:
      vegetation.total_calories -= metabolic_demand
      c.last_fed_tick = current_tick
    Else:
      starvation_counter += 1
      If (current_tick - c.last_fed_tick) > STARVATION_THRESHOLD_DAYS:
        kill(c)  // Notify System 13
        recent_starvation_rate += 1
```

**Phase 3: Carnivore Predation & Energy Transfer**
```
For each species s in cell where trophic_level[s] > 2.0:
  For each individual predator p in s:
    // Identify available prey
    viable_prey = filter(diet_composition[p.species], 
                         lambda prey_species: populations[prey_species].count > 0
                         AND populations[prey_species].average_mass < p.mass * 1.5)
    
    If viable_prey is non-empty:
      prey_species = select_by_encounter_rate(viable_prey)
      If random() < PREDATION_SEARCH_EFFICIENCY:
        prey_individual = sample(populations[prey_species])
        kill(prey_individual)  // Notify System 13
        energy_gained = prey_individual.mass * ENERGY_TRANSFER_EFFICIENCY
        p.last_fed_tick = current_tick
        recent_predation_loss[prey_species] += 1
    Else:
      starvation_counter += 1
      If (current_tick - p.last_fed_tick) > STARVATION_THRESHOLD_DAYS:
        kill(p)
        recent_starvation_rate += 1
```

**Phase 4: Decomposer Recycling**
```
For each cell:
  total_dead_biomass = sum of all creatures killed this tick
  recycled_calories = total_dead_biomass * decomposer_efficiency
  vegetation.total_calories += recycled_calories
```

**Phase 5: Compute Derived Metrics (every 10 ticks)**
```
For each cell:
  // Trophic level assignment
  For each species s:
    diet_species = species in diet_composition with nonzero fraction
    if all(diet == vegetation):
      trophic_level[s] = 1.0
    else:
      trophic_level[s] = 1.0 + mean(trophic_level[diet_species])
  
  // Niche overlap (Hutchinson)
  For each pair (s1, s2) in species:
    diet_overlap = |diet_composition[s1] ∩ diet_composition[s2]| 
                   / max(|diet_composition[s1]|, |diet_composition[s2]|)
    habitat_overlap = 1.0  // Both in same cell; refine with microhabitat in future
    niche_overlap[s1][s2] = diet_overlap * habitat_overlap
  
  // Keystoneness (recompute every 50 ticks; expensive)
  For each species s:
    diversity_baseline = shannon_entropy(biomass_distribution)
    // Simulate removal: temporarily set populations[s].count = 0
    // Rerun one tick of dynamics, compute new diversity
    diversity_without_s = shannon_entropy(biomass_distribution)
    keystoneness[s] = (diversity_baseline - diversity_without_s) / diversity_baseline
    // Restore populations[s]
  
  // Ecosystem resilience (stability)
  community_matrix = jacobian of (dN_i/dt) wrt (N_j) at equilibrium
  ecosystem_resilience = max_eigenvalue(community_matrix)
  
  // Diversity
  ecosystem_diversity = -sum(p_i * log(p_i)) where p_i = biomass[i] / total_biomass
```

**Phase 6: Carrying Capacity Computation (for System 01)**
```
For each species s in cell:
  // K emerges from available resources + niche space
  vegetable_food_calories = vegetation.total_calories * diet_fraction_to_vegetation[s]
  
  prey_calories = sum over all prey species p in s.diet:
    populations[p].total_biomass_kg * (p.channels.edibility / 100.0) * ENERGY_TRANSFER_EFFICIENCY
  
  available_calories = vegetable_food_calories + prey_calories
  
  per_capita_demand = basal_metabolic_rate(average_mass[s]) * s.channels.metabolic_rate
  
  // K from calories
  K_from_calories = available_calories / per_capita_demand
  
  // K from space (crude: cell has finite area)
  K_from_space = cell.area_m2 / (average_mass[s] * SPACE_MULTIPLIER)
  
  // Niche competition reduces K
  competition_factor = 1.0 - mean(niche_overlap[s][other_species]) * populations[other].count/K_other
  
  carrying_capacity[s] = min(K_from_calories, K_from_space) * competition_factor
```

---

## 5. Cross-System Hooks

### Reads From:
- **System 01 (Evolution)**: species channel values (diet_breadth, metabolic_rate, trophic_preference)
- **System 07 (Exploration)**: cell biome, altitude, latitude, area_m2
- **System 13 (Reproduction & Lifecycle)**: birth events, death events (starvation, predation, old age)
- **System 15 (Climate & Biome)**: latitude_insolation, season, biome_fertility, temperature for activity scheduling

### Writes To:
- **System 01 (Evolution)**: carrying_capacity[cell][species], fitness_from_abundance (used in System 01's final fitness)
- **System 13 (Reproduction & Lifecycle)**: availability metrics for mate search and population control logic

### Reads/Writes With:
- **System 09 (World History)**: keystone species and ecosystem collapse events become lore triggers
- **System 16 (Disease & Parasitism)**: pathogen transmission rates depend on host density (populations[species].count) and niche overlap (shared food sources = closer contact)

---

## 6. Tradeoff Matrix

| Decision | Options | Sim Fidelity | Implementability | Player Legibility | Emergent Power | Choice + Why |
|----------|---------|--------------|------------------|-------------------|-----------------|-------------|
| **Predation search success rate** | Deterministic (always hit) vs. probabilistic 0.8 vs. Foraging theory (encounter rate) | Encounter-rate highest | Probabilistic easiest | Probabilistic clear (hunt fails sometimes) | Probabilistic allows starvation dynamics, famine events | **Probabilistic 0.8**: biologically grounded, creates boom-bust cycles |
| **Niche overlap calculation** | Diet-only vs. (Diet + Habitat + Time) vs. Full multivariate Hutchinson | Full Hutchinson highest | Diet-only easiest (others need microhabitat tracking) | Diet-only most legible | Full Hutchinson enables subtle coexistence stories | **Diet-only initially, expand to habitat later**: scalable; diet differences are emergent from evolution |
| **Carrying capacity derivation** | Rosenzweig-MacArthur (glucose-based) vs. Space-based vs. Lotka-Volterra equilibrium | Rosenzweig-MacArthur | R-M moderate; others easy | All three hard to explain | R-M enables rich dynamics (e.g., prey-switching) | **R-M with space cap**: grounded in real ecology, avoids K spirals |
| **Decomposer recycling** | Instant (100%) vs. Slow (decay over days) vs. Agent-based (creatures do it) | Agent-based highest | Instant easiest | Instant clearest | Agent-based richest (decomposers as faction) | **Instant 70%**: keeps nutrients flowing; agents later if needed |
| **Keystoneness recomputation** | Every tick vs. Every 50 ticks vs. Never (static) | Every tick | Never easiest | Every 50 ticks clear | Every tick enables dynamic keystone shifts | **Every 50 ticks**: lore can reflect "X became keystone" events |

---

## 7. Emergent Properties

1. **Trophic cascades**: removal of top predator → herbivore boom → vegetation collapse → secondary collapse of herbivores (documentable in System 09 lore).
2. **Boom-bust oscillations**: probabilistic predation + delayed reproduction creates classic Lotka-Volterra cycles; these appear as "seasons of plenty and famine" in faction chronicles.
3. **Niche partitioning**: species with overlapping diets gradually diverge diet preferences (via selection in System 01) to reduce competition; observable as diet_composition arrays changing over generations.
4. **Ecosystem fragility threshold**: as diversity drops, eigenvalues of community matrix approach zero; system enters "tipping point" territory (e.g., one plague away from collapse).
5. **Keystone species emergence**: a species becomes critical not by being abundant, but by its unique role (e.g., single seed disperser, only pollinator). Its loss collapses the food web.

---

## 8. Open Calibration Knobs

- **ENERGY_TRANSFER_EFFICIENCY**: currently 0.10 (10%). Increase to 0.15 → ecosystems support 50% more biomass, less starvation. Decrease to 0.08 → harsher selection, larger K gaps between species.
- **VEGETATION_REGENERATION_BASE**: currently 100 kCal/tick. Tune per biome; forest higher, desert lower. Affects K and carrying-capacity variation.
- **PREDATION_SEARCH_EFFICIENCY**: currently 0.8. Decrease to 0.6 → predators must be smarter or specialize more. Increase to 0.95 → prey pressure is relentless, driving herbivore diversity.
- **STARVATION_THRESHOLD_DAYS**: currently 14. Increase to 21 → creatures tolerate scarcity better, less death noise. Decrease to 7 → harsher, boom-bust faster.
- **SPACE_MULTIPLIER**: factor converting average_mass to area requirement. Higher = creatures need more space, lower K. Affects role of cell size.
- **Decomposer efficiency**: currently 0.70 (70% recycled). Lower values → nutrient sink, ecosystems cycle through succession phases. Higher values → steady-state nutrient cycling.

