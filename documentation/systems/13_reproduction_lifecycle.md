# System 13: Reproduction & Lifecycle

## 1. Overview

The Reproduction & Lifecycle system models how creatures breed, develop, and age. It is tightly coupled to System 01 (Evolution), System 12 (Ecology), and System 10 (Procgen Visuals).

Each creature progresses through a sequence of life stages:
1. **Conception** (choice of reproductive strategy, mating, or asexual division).
2. **Gestation/Incubation** (timer-driven; duration scales with metabolic_rate and complexity).
3. **Birth** (inheritance of alleles from parents via Mendelian genetics for sexual reproducers, or direct copy for asexual).
4. **Juvenile development** (gradual morphological interpolation from juvenile to adult shape; performance traits scale up; vulnerability to predation/starvation remains high).
5. **Adulthood** (reproductive eligibility, display channels activate).
6. **Senescence** (fitness decline; telomere-style aging).
7. **Death** (natural, starvation, predation, disease, or age).

Reproductive strategies (asexual, sexual, parthenogenesis) emerge from genotype; different strategies incur different energy costs and genetic mixing strategies. This system **drives population dynamics in System 12** (births/deaths propagate) and **reads carrying capacity feedback to modulate reproductive success**.

---

## 2. Research Basis

### Reproductive Strategies & Life-History Theory
- **Pianka's r/K-selection continuum** (1970): r-selected species (fast maturation, high fecundity, short lifespan, low parental care) vs. K-selected species (slow maturation, low fecundity, long lifespan, high parental care). On a spectrum, shaped by environmental stability.
- **Trivers' parental investment theory** (1972): investment in fewer, higher-quality offspring (K) vs. many low-investment offspring (r).
- **Asexual vs. sexual reproduction** (Rose & Otto 2000): asexual cloning has 2x per capita growth (no male cost) but zero genetic recombination; sexual has male cost but genetic shuffling for parasite/pathogen evasion (Red Queen hypothesis).

### Mating & Sexual Selection
- **Fisherian runaway** (Fisher 1930): display channels (coloration, song, size dimorphism) amplify via mate choice because females prefer them, even if costly.
- **Handicap principle** (Zahavi 1975): costly displays signal health/quality; only high-fitness males can bear the cost.
- **Sperm competition** (Parker 1970): in polyandrous species, male reproductive anatomy/behavior competes post-copulation.

### Mendelian Inheritance & Polygenic Traits
- **Biallelic loci**: each gene has two alleles (A/a); offspring inherit one from each parent. Heterozygotes may be dominant or show blending.
- **Polygenic inheritance** (Falconer 1960): phenotypic traits (e.g., size, speed) controlled by many loci; heritability h² relates parent-offspring resemblance. Offspring phenotype ≈ midpoint of parents' phenotypes + mutation + random noise.
- **Covariance & linkage disequilibrium** (Lynch & Walsh 1998): traits may be genetically correlated (e.g., large body size and metabolic cost); selection on one trait indirectly selects on others.

### Development & Ontogeny
- **Von Bertalanffy growth curve** (1938): L(t) = L_∞ (1 – e^(-k(t – t₀))). Creatures grow rapidly post-birth, then plateau.
- **Allometric scaling** (Huxley 1932): trait_i scales with body_mass^(b_i). E.g., limb length scales as mass^0.33 to maintain agility across sizes.

### Senescence & Aging
- **Williams' antagonistic pleiotropy hypothesis** (1957): alleles beneficial early in life may be deleterious late (e.g., rapid growth → cancer risk). No single "aging gene"; rather, trade-offs accumulate.
- **Telomere shortening** (Hayflick 1961): linear DNA caps shorten with each replication; eventually cells stop dividing (Hayflick limit). We use as metaphor: creatures accumulate a cumulative_damage counter that increases with age, stress, and reproduction.
- **Gompertz mortality law**: mortality rate increases exponentially with age (μ(t) ∝ e^(αt)). We model as fitness_multiplier declining from 1.0 toward 0 over lifespan.

---

## 3. Entities & State

### Per-Creature Reproductive & Lifecycle Data

```
Creature = {
  // Identity & lineage
  creature_id: UUID,
  species_id: species_id,
  parent_ids: [mother_id, father_id | father_id | None],  // Asexual has one parent or None
  generation: int,                                          // Distance to founding ancestor
  
  // Life stage
  life_stage: enum {GAMETE, EMBRYO, JUVENILE, ADULT, ELDER, DEAD},
  age_ticks: int,                    // Time since birth (or conception for embryos)
  maturity_age_ticks: int,           // Age at which life_stage transitions from JUVENILE to ADULT
  
  // Morphology & growth
  current_mass_kg: float,
  target_mass_kg: float,             // Genetically determined adult size
  morphological_state: float,        // 0.0 = newborn juvenile, 1.0 = full adult (scales ALL size-dependent traits)
  
  // Genetics: allele pairs per locus
  // Example locus: diet_breadth_loci = [allele_mother, allele_father]
  genotype = {
    [locus_name]: [allele_m, allele_f],
    ...
  },
  
  // Phenotype (computed from genotype + morphological_state + environment)
  channels: {  // From System 01; recomputed with morphological_state scaling
    // ... all channel values, scaled by morphological_state
    metabolic_rate: float,
    speed: float,
    strength: float,
    display_coloration_r: int,   // RGB for display (sexual selection)
    display_coloration_g: int,
    display_coloration_b: int,
    // ... etc
  },
  
  // Reproductive state
  reproductive_strategy: enum {ASEXUAL_DIVISION, SEXUAL_DIPLOID, PARTHENOGENETIC},
  sex: enum {MALE, FEMALE, HERMAPHRODITE, ASEXUAL} | None,  // Determined at birth by genotype
  is_fertile: bool,                // True if adult, not pregnant, not recently-bred
  gestation_progress_ticks: int,  // 0 if not pregnant; counts up toward gestation_duration
  gestation_duration_ticks: int,  // Scales with metabolic_rate and complexity
  
  // For sexual reproducers: recent mate
  recent_mate_id: UUID | None,
  breeding_cooldown_ticks: int,   // Prevents over-breeding in single tick
  
  // Health & aging
  cumulative_damage: float,       // Accumulates with age, starvation, injury. Increases senescence.
  telomere_quota: float,          // 0.0 = senescent, 1.0 = newborn. Decreases with age and reproduction.
  hunger_counter_ticks: int,      // Time since last meal; >threshold triggers starvation death
  
  // Historical
  num_offspring: int,
  total_offspring_mass: float,
  last_reproduction_tick: int,
}
```

### Per-Species Reproductive Configuration (in species definition)

```
Species = {
  // ... (other system data)
  
  reproduction = {
    // Strategy
    primary_strategy: enum {ASEXUAL_DIVISION, SEXUAL_DIPLOID, PARTHENOGENETIC},
    
    // Loci defining traits (System 01 reads these)
    locus_definitions = {
      [locus_name]: {
        allele_dominance: enum {DOMINANT, RECESSIVE, CODOMINANT, INCOMPLETE_DOMINANCE},
        allele_map: {allele_value: phenotypic_contribution},
        heritability_h2: float,  // Fraction of phenotypic variance attributable to genetics
      }
    },
    
    // Sexual reproduction parameters
    mate_preference_channels: [channel_1, channel_2, ...],  // Sexual selection on these channels
    sexual_dimorphism_threshold: float,  // How different males/females are in display channels
    mate_search_radius_m: float,  // How far individuals will search for mates (from System 12 availability)
    
    // Breeding costs
    gestation_base_ticks: int,    // e.g., 20; scaled by metabolic_rate
    asexual_division_cost_pct: float,  // % of energy/biomass expended to produce offspring
    sexual_reproduction_male_cost_pct: float,
    sexual_reproduction_female_cost_pct: float,
    parental_care_mode: enum {NONE, MATERNAL, BIPARENTAL},
    parental_care_duration_ticks: int,  // Reduces offspring starvation if parent present
    
    // Development timescales
    juvenile_duration_base_ticks: int,  // e.g., 50; scales with metabolic_rate
    elder_threshold_pct: float,  // % of max lifespan; e.g., 80% → last 20% is elder phase
    max_lifespan_ticks: int,  // Hard cap; creature dies of old age
    
    // Fecundity
    fecundity_base: float,  // Avg offspring per breeding event
    sex_ratio_male: float,  // Fraction of offspring that are male (0.5 typical)
  }
}
```

---

## 4. Update Rules

### Phase 1: Age All Creatures & Update Life Stages (Each Tick)

```
For each creature c:
  age_ticks += 1
  
  // Update life stage based on age and reproductive status
  If c.life_stage == EMBRYO:
    c.gestation_progress_ticks += 1
    If c.gestation_progress_ticks >= c.gestation_duration_ticks:
      // BIRTH EVENT
      c.life_stage = JUVENILE
      c.morphological_state = 0.05  // Start very small
      c.age_ticks = 0
      System12.record_birth(c.species_id, c.current_mass_kg)  // For carrying capacity
      
  Else if c.life_stage == JUVENILE:
    If c.age_ticks >= c.maturity_age_ticks:
      c.life_stage = ADULT
      c.morphological_state = 1.0
      
  Else if c.life_stage == ADULT:
    elder_threshold_age = c.species.max_lifespan * c.species.elder_threshold_pct
    If c.age_ticks >= elder_threshold_age:
      c.life_stage = ELDER
      
  Else if c.life_stage == ELDER:
    If c.age_ticks >= c.species.max_lifespan:
      c.life_stage = DEAD
      System12.record_death(c, cause=NATURAL_AGE)
      return  // Skip remaining updates
      
  // Update damage (senescence)
  c.cumulative_damage += 0.01 * (c.age_ticks / c.species.max_lifespan)
  c.telomere_quota -= 0.001 * (1.0 + c.cumulative_damage)
  If c.telomere_quota <= 0.0:
    c.life_stage = DEAD
    System12.record_death(c, cause=TELOMERE_EXHAUSTION)
    return
```

### Phase 2: Growth & Morphology (Each Tick)

```
For each creature c where life_stage in [EMBRYO, JUVENILE, ADULT]:
  // Von Bertalanffy growth
  growth_rate = c.species.growth_coefficient * (1.0 - c.current_mass_kg / c.target_mass_kg)
  c.current_mass_kg += growth_rate
  
  // Interpolate morphological state (juvenile → adult)
  If c.life_stage == JUVENILE:
    juvenile_progress = c.age_ticks / c.maturity_age_ticks
    c.morphological_state = min(juvenile_progress, 1.0)
  Else if c.life_stage == ADULT or ELDER:
    c.morphological_state = 1.0
    
  // Recompute phenotype channels with allometry
  For each channel ch in c.channels:
    base_value = c.genotype_phenotype(ch)  // From System 01
    allometric_scaling = (c.current_mass_kg / c.target_mass_kg) ^ allometric_exponent[ch]
    c.channels[ch] = base_value * allometric_scaling * c.morphological_state
    
  // Hunger & starvation check
  c.hunger_counter_ticks += 1
  If c.hunger_counter_ticks > STARVATION_THRESHOLD:
    c.life_stage = DEAD
    System12.record_death(c, cause=STARVATION)
    return
```

### Phase 3: Reproduction Decision & Mating (Each Tick)

```
For each creature c where life_stage == ADULT and age_ticks >= maturity_age:
  is_fertile = (c.gestation_progress_ticks == 0)  // Not pregnant
  is_ready = (current_tick - c.last_reproduction_tick) > breeding_cooldown_ticks
  
  If is_fertile and is_ready:
    // Check carrying capacity signal from System 12
    population_ratio = System12.get_abundance_ratio(c.species_id, c.cell_id)
    // If ratio > 1.0, K exceeded; reduce reproduction via Poisson breeding
    
    // Asexual strategy
    If c.reproductive_strategy == ASEXUAL_DIVISION:
      If random() < fecundity_base * (1.0 - population_ratio * 0.5):
        // Division: clone parent genotype (with mutation)
        offspring = create_creature(
          parents=[c],
          genotype=c.genotype + mutation_noise(),
          reproductive_strategy=ASEXUAL_DIVISION
        )
        
        // Energy cost
        c.current_mass_kg *= (1.0 - asexual_division_cost_pct)
        c.cumulative_damage += 0.05  // Reproduction ages you
        c.telomere_quota -= 0.05
        c.last_reproduction_tick = current_tick
        
        System12.record_birth(offspring, c.cell_id)
        
    // Sexual strategy
    Else if c.reproductive_strategy == SEXUAL_DIPLOID:
      // Determine sex if not yet assigned
      If c.sex == None:
        c.sex = sample_from_sex_ratio(c.species.sex_ratio_male)
        
      If c.sex == MALE:
        // Males broadcast mate-search; females select
        // (mating happens in female branch below)
        pass
        
      Else if c.sex == FEMALE:
        // Search for mates in vicinity
        available_males = System12.find_creatures_near(
          species_id=c.species_id,
          cell_id=c.cell_id,
          radius=c.species.mate_search_radius_m,
          sex=MALE,
          is_fertile=True
        )
        
        If available_males:
          // Mate choice via sexual selection
          mate = select_mate_by_display(available_males, c.species.mate_preference_channels)
          
          // Fertilization: create embryo
          offspring_genotype = mendelian_cross(c.genotype, mate.genotype, c.species)
          
          embryo = create_creature(
            parents=[c, mate],
            genotype=offspring_genotype,
            life_stage=EMBRYO,
            gestation_progress_ticks=0
          )
          c.gestation_progress_ticks = 1  // Mark as pregnant
          c.gestation_duration_ticks = compute_gestation_duration(
            c.species,
            metabolic_rate=c.channels.metabolic_rate
          )
          
          // Energy cost to female
          c.current_mass_kg *= (1.0 - sexual_reproduction_female_cost_pct)
          c.cumulative_damage += 0.03
          mate.cumulative_damage += 0.01  // Male pays smaller cost
          c.last_reproduction_tick = current_tick
          
    // Parthenogenetic strategy
    Else if c.reproductive_strategy == PARTHENOGENETIC:
      // Like asexual but requires (slower) meiosis
      If random() < (fecundity_base * 0.6) * (1.0 - population_ratio * 0.5):
        offspring = create_creature(
          parents=[c],
          genotype=c.genotype + larger_mutation_noise(),  // More mutation without sexual mixing
          reproductive_strategy=PARTHENOGENETIC
        )
        c.current_mass_kg *= (1.0 - asexual_division_cost_pct * 0.8)
        c.cumulative_damage += 0.02
        c.last_reproduction_tick = current_tick
        System12.record_birth(offspring, c.cell_id)
```

### Phase 4: Mendelian Cross & Offspring Genotype (Subroutine)

```
Function mendelian_cross(genotype_mom, genotype_dad, species):
  offspring_genotype = {}
  
  For each locus L in species.locus_definitions:
    // Each parent contributes one allele at locus L
    [allele_m1, allele_m2] = genotype_mom[L]
    [allele_d1, allele_d2] = genotype_dad[L]
    
    inherited_mom = sample([allele_m1, allele_m2])  // Random choice
    inherited_dad = sample([allele_d1, allele_d2])
    
    // Apply mutation during meiosis (System 01 mutation rates)
    If random() < mutation_rate:
      inherited_mom = mutate_allele(inherited_mom, species, L)
    If random() < mutation_rate:
      inherited_dad = mutate_allele(inherited_dad, species, L)
      
    offspring_genotype[L] = [inherited_mom, inherited_dad]
    
  Return offspring_genotype
```

### Phase 5: Mate Choice via Sexual Selection (Subroutine)

```
Function select_mate_by_display(candidates, preference_channels):
  // Females choose males with high display values
  fitness_scores = {}
  
  For each male m in candidates:
    score = 0.0
    For each channel ch in preference_channels:
      // Display channels: coloration, song, size dimorphism, etc.
      score += m.channels[ch]
    
    // Handicap principle: costly displays signal quality
    // Males with high display must be healthier to afford it
    if m.telomere_quota < 0.5:
      score *= 0.5  // Discount unhealthy males
      
    fitness_scores[m] = score
    
  // Probabilistic selection (not deterministic best)
  selected_male = sample_by_fitness(fitness_scores)
  Return selected_male
```

### Phase 6: Parental Care (If Enabled, Each Tick)

```
For each creature c where parental_care_mode != NONE:
  If c.life_stage == ADULT and has_dependent_offspring:
    for each offspring o in c.dependents:
      If o.life_stage == JUVENILE:
        // Parent mitigates offspring starvation
        protection_duration_remaining = parental_care_duration - (current_tick - o.birth_tick)
        If protection_duration_remaining > 0:
          o.hunger_counter_ticks = max(0, o.hunger_counter_ticks - 2)  // Parent "feeds" offspring
          o.cumulative_damage -= 0.01  // Reduced stress
```

---

## 5. Cross-System Hooks

### Reads From:
- **System 01 (Evolution)**: genotype_to_phenotype mapping, channel allometry exponents, species locus definitions, mutation rates
- **System 10 (Procgen Visuals)**: juvenile_morph vs. adult_morph visual templates; applies morphological_state interpolation
- **System 12 (Ecology)**: carrying_capacity, abundance_ratio for population-based fertility modulation, food availability for hunger tracking
- **System 15 (Climate & Biome)**: gestation duration may scale with season/temperature (energetic stress)
- **System 09 (World History)**: landmark births/deaths of notable creatures become lore

### Writes To:
- **System 01 (Evolution)**: cumulative_damage, telomere_quota as fitness modifiers; offspring as new individuals with genotypes
- **System 12 (Ecology)**: record_birth, record_death events; drives population dynamics
- **System 09 (World History)**: significant reproduction/death events (e.g., birth of legendary hero, extinction of population)
- **System 10 (Procgen Visuals)**: morphological_state, display channels for rendering juvenile-to-adult transitions and sexual selection displays

### Reads/Writes With:
- **System 04 (Combat)**: damage from combat injury increases cumulative_damage; affects reproduction vigor
- **System 16 (Disease & Parasitism)**: pathogen infection increases cumulative_damage, reduces fertility, increases gestation duration

---

## 6. Tradeoff Matrix

| Decision | Options | Sim Fidelity | Implementability | Player Legibility | Emergent Power | Choice + Why |
|----------|---------|--------------|------------------|-------------------|-----------------|-------------|
| **Inheritance model** | Asexual cloning vs. Mendelian biallelic vs. Polygenic quantitative | Polygenic highest | Biallelic easiest | Biallelic clearest (2 parents, 1 allele each) | Polygenic enables sexual selection runaway | **Biallelic initially**: fundamental, extensible to polygenic without refactor |
| **Sex determination** | Genetic (XY/ZW) vs. Environmental vs. Hermaphrodite-biased | Genetic highest | Hermaphrodite easiest | Genetic most intuitive | Genetic enables sexual dimorphism arms race | **Genetic XY**: coupled with sexual selection, drives display evolution |
| **Mate choice** | Random vs. Prefer-by-display vs. Complex female choice algorithm | Complex algorithm highest | Random easiest | Prefer-by-display clear (see coloration) | Complex algorithm enables Fisherian runaway | **Prefer-by-display with handicap check**: emergent arms race; high replay value |
| **Gestation scaling** | Fixed duration vs. Metabolic-scaled vs. (Metabolic + Season + Health) | Full scaling highest | Fixed easiest | Metabolic-scaled intuitive | Full scaling enables generation-time diversity | **Metabolic-scaled**: fast creatures breed faster, slower creatures more strategic |
| **Senescence model** | No aging vs. Linear decline vs. Gompertz exponential vs. Telomere quota | Telomere quota highest | No aging easiest | Linear decline clearest | Telomere quota enables aging diversity (fast-living short-lived vs. conservative long-lived) | **Telomere quota with cumulative damage**: linked to reproduction, creates trade-offs |
| **Parental care** | None vs. Maternal only vs. Biparental vs. Alloparental (helper breeding) | Alloparental highest | None easiest | Maternal-only intuitive | Alloparental enables eusocial evolution | **Maternal-only initially**: simpler; refactor for biparental later if needed |

---

## 7. Emergent Properties

1. **Sexual selection runaway**: females prefer males with costly displays (e.g., bright coloration). Males evolve more exaggerated displays. Displays become arbitrarily extreme (Fisherian feedback loop); documentable in lore as "flamboyant heroes."
2. **Generation-time diversity**: asexual creatures breed fast (low cost); sexual creatures breed slower (mate search, gestation). This drives differential species composition: asexual dominates early game, sexual diversifies late (frequency-dependent selection).
3. **Life-history diversity**: r-selected species (small, fast, short-lived) vs. K-selected species (large, slow, long-lived) emerge from carrying capacity feedback and initial reproductive strategy. Observable as different "life strategies" across creatures.
4. **Grandmother effect**: long-lived species with biparental care benefit from grandparent survival (more kin to care for). This selects for longer lifespans and slower reproduction (K-selection).
5. **Senescence heterogeneity**: creatures face trade-off between early fertility (breed young, die sooner) vs. delayed reproduction (live longer). Species with high damage accumulation have shorter lifespans; this creates temporal niche partitioning.

---

## 8. Open Calibration Knobs

- **GESTATION_BASE_TICKS**: currently ~20. Increase to 40 → breeding slower, generation time longer. Decrease to 10 → rapid generations, evolution faster.
- **JUVENILE_DURATION_BASE_TICKS**: currently ~50. Higher values → creatures are vulnerable longer, lower juvenile survival. Affects K for different species.
- **ASEXUAL_DIVISION_COST_PCT**: currently 0.20 (20% biomass). Increase to 0.40 → asexual breeding becomes expensive, sexual becomes competitive. Decrease to 0.10 → asexual dominates.
- **SEXUAL_DIMORPHISM_THRESHOLD**: currently 0.15. Higher → males and females look more different; enables stronger sexual selection but reduces population flexibility.
- **MATE_SEARCH_RADIUS_M**: currently species-tuned. Higher → mates easier to find, higher breeding rate. Lower → isolation causes speciation pressure.
- **TELOMERE_QUOTA_DECAY_RATE**: currently 0.001 per tick. Increase → shorter lifespans overall. Affects balance between fast-breeding and long-living strategies.
- **PARENTAL_CARE_HUNGER_REDUCTION**: currently –2 ticks per parental tick. Increase to –4 → offspring survive longer under care; bigger payoff to parental investment.

