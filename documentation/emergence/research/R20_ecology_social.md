# Research Synthesis: Emergent Ecology and Emergent Social Structure Models
**Date:** April 2026  
**Status:** Literature synthesis for Beast Evolution Game architecture  
**Target:** Replace hardcoded parameters with mechanistic, emergent models

---

## PART A: EMERGENT ECOLOGY MODELS

### 1. Williams-Martinez Niche Model (2000)

**Description:**  
Food-web structure emerges from body-mass distributions via a one-dimensional niche axis. Each species occupies a niche position and has a feeding range; predator-prey links form probabilistically when prey fall within the predator's range.

**Key Paper:**  
Williams & Martinez, "Simple rules yield complex food webs," *Nature* (2000)

**Mechanism:**  
- Each species has three traits: niche position (0–1), diet center, and feeding range
- Predator *i* feeds on prey *j* if prey lies within predator's range
- Predator–prey mass ratios emerge: larger predators tend to feed on smaller prey
- No hardcoded energy transfer; link structure is purely geometric

**Input Channels:**
- Body mass distribution (organism birth/growth state)
- Environmental productivity (affects range sizes)

**Output Channels:**
- Trophic links (adjacency graph)
- Trophic level (derived from link structure, not assigned)

**Complexity:**  
Low. O(S²) for S species; purely geometric link formation.

**Determinism:**  
Fully deterministic. Requires only sorted entity IDs for link iteration.

**Game Application:**
Replace hardcoded diet rules with niche-position computation. Let body-mass distribution of creatures in a biome determine available predators/prey automatically. Trophic levels emerge post-hoc.

**Tradeoffs:**
- **Pro:** Emergent food-web structure, realistic allometry, deterministic
- **Con:** One-dimensional niche axis may miss functional-trait diversity; diet breadth does not capture prey-switching

---

### 2. Allometric Trophic Networks (ATN) & Diet Breadth Model

**Description:**  
Food-web links and interaction strengths scale with body mass via allometric relationships. Diet breadth (number of prey types) and consumption rates depend on predator size and prey availability.

**Key Papers:**  
Brose et al., ATN framework; Petchey et al., *Allometric Diet Breadth Model* (2008)

**Mechanism:**  
- Predator's feeding rate ∝ M^α (metabolic scaling, α ≈ 0.75)
- Diet breadth ∝ M^β (allometric diet expansion; β > 0)
- Attack rate (encounter) ∝ M_pred^0.5 × M_prey^(-0.5) (relative size difference)
- Optimal diet width emerges from foraging economics

**Input Channels:**
- Individual organism mass (Q32.32)
- Encounter rate (search volume, movement speed in fixed-point)
- Prey availability (population density per tile/region)

**Output Channels:**
- Realized diet composition (which prey types, how many)
- Consumption rate (J/tick; must be deterministic from fixed-point masses)
- Interaction strength (entry in S×S matrix)

**Complexity:**  
Medium. Requires per-organism diet calculation each tick; O(S × D) where D = mean diet breadth.

**Determinism:**  
Fully deterministic. All scaling exponents are constants; encounter rates depend only on mass and sorted density. Must iterate prey list in sorted entity-ID order to avoid HashSet non-determinism.

**Game Application:**
Replace the hardcoded 0.10 energy-transfer efficiency with mechanistic consumption rates derived from predator–prey mass ratios. Let diet breadth emerge from allometric scaling, not diet-type enumeration. Interaction strength becomes a continuous per-pair property.

**Tradeoffs:**
- **Pro:** Explains predator–prey body-size ratios, realistic diet expansion, handles ontogenetic diet shift
- **Con:** Computationally heavier; requires per-organism neighbor queries; assumes mass is sufficient to explain diet (misses trait convergence)

---

### 3. Dynamic Energy Budget (DEB) Theory

**Description:**  
Individual organisms partition assimilated energy into maintenance, growth, reproduction, and storage via mechanistic rate equations, replacing the top-down Lindeman 10% rule.

**Key Paper:**  
Kooijman, *Dynamic Energy Budget Theory for Metabolic Organisation* (Cambridge, 2010)

**Mechanism (simplified):**  
An organism's body compartments:
- **Structure**: fixed-size biomass that consumes maintenance power
- **Reserve**: variable energy depot (fed first, used for all work)
- **Reproduction buffer** (adults): energy reserved for offspring

Energy flow:
1. Ingestion rate = f(food availability, feeding efficiency)
2. Assimilation = η × ingestion (efficiency ~0.7–0.9)
3. Catabolism = maintenance power + movement + growth + reproduction
4. κ-rule: fraction κ of catabolism → growth; (1−κ) → maintenance + reproduction

**Input Channels:**
- Food density (J/L or J/tile)
- Organism structural mass (ontogenetic state)
- Reserve level (internal energy, Q32.32)

**Output Channels:**
- Metabolic rate (J/tick; deterministic from reserve and structure)
- Growth rate (dM_struct/dt)
- Reproductive output (# offspring, timing)

**Complexity:**  
High. Requires per-organism ODE integration (Euler step). Must track reserve state, growth history, and maturation schedule per entity.

**Determinism:**  
Fully deterministic if:
- Fixed-point Euler integration with constant dt
- Reserve level stored as Q32.32 (no floating-point)
- Food density is quantized (e.g., J/tile, sorted by entity proximity)

**Game Application:**
Embed a light DEB model in Stage 5 (Physiology). Each creature tracks structural mass and energy reserve; metabolism and growth emerge from energy balance, not stat sheets. Reproduction timing and clutch size emerge from reserve availability. Energy transfer efficiency is mechanistic, not hardcoded.

**Tradeoffs:**
- **Pro:** Mechanistic, explains ontogenetic growth trajectories and reproductive trade-offs, links individual metabolism to population dynamics
- **Con:** Adds per-tick ODE overhead; introduces small numerical sensitivity even with fixed-point (Euler error); requires careful reserve initialization

---

### 4. Continuous Trophic Position

**Description:**  
Replace integer trophic levels (herbivore=2, carnivore=3, omnivore=2.5) with continuous fractional trophic position computed from realized diet composition, tracked over a time window (e.g., 10 ticks).

**Key Paper:**  
Post, "Using stable isotopes to estimate trophic position: models, methods, and assumptions," *Ecology* (2002)

**Mechanism:**  
TP(consumer) = 1 + mean(TP(prey sources)) weighted by diet fraction

Example: a creature eating 60% herbivore (TP=2) and 40% carnivore (TP=3):
TP = 1 + 0.6×2 + 0.4×3 = 2.4

Requires tracking actual prey kills (or consumption events) over a rolling window, not diet archetype.

**Input Channels:**
- Prey consumption history (sorted list of prey eaten, last N ticks)
- Prey species trophic positions (computed recursively, base TP=1 for producers)

**Output Channels:**
- Continuous trophic position (Q16.16 or similar fixed-point)
- Trophic level class (derived: L1=1.0–1.5, L2=1.5–2.5, L3=2.5–3.5, etc.)

**Complexity:**  
Low to medium. O(S) per-tick update if food-web is cached and sorted.

**Determinism:**  
Fully deterministic. Requires only sorted consumption records and sorted TP lookup. No HashMaps.

**Game Application:**
Compute trophic position post-hoc in the Labeling stage (Stage 7). Store recent kill log (last 20 ticks) in sorted order; derive TP and emit it as a continuous stat. Use TP for ecosystem-level diagnostics (e.g., biome productivity vs. mean TP) rather than assigning it at birth.

**Tradeoffs:**
- **Pro:** Handles omnivory correctly, continuous (realistic), emerges from actual behavior, no hardcoded archetypes
- **Con:** Lag (TP stabilizes over ~20 ticks); requires kill log (small memory overhead per creature)

---

### 5. Eco-Evolutionary Feedback & Adaptive Dynamics

**Description:**  
Trait evolution and population dynamics are coupled: allele frequency change alters ecological conditions (resources, competition), which then feeds back to selection pressure.

**Key Papers:**  
Hairston et al., "Rapid evolution and community dynamics" (2005); Geritz, Metz, Diekmann, *Adaptive Dynamics* framework (1990s)

**Mechanism (simplified):**  
- Population X and allele frequencies shift on ecological timescale (1–100 ticks)
- Fitness landscape changes as food availability (N_food) and competitor density change
- New equilibrium frequency emerges
- Over longer timescale (~1000 ticks), mutation introduces a new allele
- If new allele has higher fitness at current population state, it sweeps

**Input Channels:**
- Allele frequencies (per locus, per population)
- Population densities (species-level)
- Resource distributions (food density per biome)

**Output Channels:**
- Trait distributions (mean and variance per species/population)
- Fitness surface (relative to population state)
- Population dynamics (births, deaths, emigration)

**Complexity:**  
Very high. Requires tracking genotype frequencies per subpopulation; multivariate fitness surface computation.

**Determinism:**  
Problematic. Adaptive dynamics relies on invasion analysis (mutant vs. resident fitness), which is mathematically sound but computationally requires stochastic simulation. Large population sizes and asynchronous breeding can break determinism.

**Game Application:**
Implement eco-evo feedback at the biome scale, not globally. For each biome:
1. Each tick, compute mean fitness = (births − deaths) / population
2. Allele frequencies change proportionally: p_new = p_old + ε × (fitness − mean_fitness)
3. ε is selection strength (e.g., 0.001 per tick for realism)
4. Use mutation replay (seed PRNG by entity ID and allele) to keep deterministic

**Tradeoffs:**
- **Pro:** Links evolution to ecology dynamically; explains why phenotypes shift in response to competition/predation
- **Con:** Computationally expensive; determinism requires careful PRNG seeding per locus; easily becomes a "chaos" system if populations are small

---

### 6. Trait-Based Niche Emergence (Functional Diversity)

**Description:**  
Replace diet-overlap-only niche competition with multivariate trait distance. Creatures compete if they are close in trait space (body size, feeding morphology, habitat preference, etc.); niche differentiation emerges from trait divergence.

**Key Paper:**  
Petchey & Gaston, "Functional diversity: back to basics and looking forward," *Ecology Letters* (2006)

**Mechanism:**  
Define a trait vector T = [log(mass), jaw_size, habitat_type, activity_time, ...].  
Niche overlap(species A, B) = 1 / (1 + d(T_A, T_B)) where d = Euclidean distance in trait space.  
Competition coefficient α_AB = niche_overlap × trophic_overlap

**Input Channels:**
- Per-creature trait vector (phenotype; derived from genome)
- Trophic overlap (from diet composition, previous section)

**Output Channels:**
- Pairwise competition coefficients (S×S symmetric matrix)
- Functional diversity index (branch length in trait dendrogram)

**Complexity:**  
Medium. O(S²) for pairwise comparisons; trait vectors are small (5–10 floats per creature). Can be cached per species/population.

**Determinism:**  
Fully deterministic if trait values are fixed-point and trait vectors are sorted by entity ID before distance computation.

**Game Application:**
In Stage 4 (Interaction & Combat), compute pairwise competition as a function of trait distance, not just diet. Allow creatures with very different trait suites (e.g., small nocturnal insectivore vs. large diurnal herbivore) to coexist even if they eat similar plants. Let niche differentiation emerge from trait divergence via mutation and selection.

**Tradeoffs:**
- **Pro:** Mechanistic niche separation, explains character displacement, allows realistic character radiation
- **Con:** Requires defining trait-relevance weights; computationally heavier; trait space is subjective

---

### 7. Decomposer Dynamics (CORPSE/MEND Models, simplified)

**Description:**  
Detritus decomposition is not a fixed-efficiency coefficient; it emerges from explicit microbial pools and enzyme kinetics. Carbon cycles through plant litter → fast detritus → microbial biomass → stabilized soil C.

**Key Papers:**  
Sulman et al., *CORPSE* model (2014); Wang et al., *MEND* model; simplified reviews in Chandel et al. (2023)

**Mechanism (game-scale simplification):**  
Each biome/tile tracks:
- Plant litter pool (newly shed biomass)
- Fast detritus pool (labile organic matter)
- Microbial biomass pool
- Stable soil pool (mineral-bound, very slow turnover)

Fluxes (per tick):
1. Litter input = Σ(dead organisms + shedding)
2. Litter decomposition = f(litter, microbe density, temperature) [Michaelis-Menten or similar]
3. Microbial respiration = fraction of consumption (inefficiency)
4. Microbial death → part goes back to fast detritus, part to stable pool
5. Stable pool loss = very slow (e.g., 1% per 100 ticks)

**Input Channels:**
- Dead organism biomass (J)
- Biome temperature (affects enzyme kinetics)
- Soil moisture (affects microbial activity)
- Microbial biomass (state variable)

**Output Channels:**
- Primary productivity available (tied to nutrient cycling)
- Soil organic matter (affects water retention, nutrient availability)
- CO2 respiration rate (ecosystem-level diagnostics)

**Complexity:**  
Medium. Per-tile/biome 4–5 state variables; simple ODEs, easily Euler-integrated.

**Determinism:**  
Fully deterministic if temperature and moisture are quantized (fixed-point) and pool fluxes are computed with sorted entity iteration.

**Game Application:**
In Stage 6 (Ecology), add decomposer pools to each biome. Primary productivity is no longer a hardcoded constant; it depends on soil organic matter and nutrient cycling. Dead creatures feed into litter, driving ecosystem productivity. Allows modeling of ecosystem collapse (all organisms die → no litter input → productivity crashes → starvation) and recovery.

**Tradeoffs:**
- **Pro:** Mechanistic nutrient cycling, links individual death to community productivity, allows ecosystem engineering
- **Con:** Adds per-biome state tracking; requires careful initialization; introduces new tuning parameters (litter decay rate, stable-pool half-life)

---

## PART B: EMERGENT SOCIAL STRUCTURE MODELS

### 8. Cultural Multilevel Selection (CMLS)

**Description:**  
Group-beneficial traits (cooperation, norm adherence, collective decision-making) spread via three mechanisms: (1) differential imitation (copying successful groups), (2) differential group proliferation/extinction (successful groups fission, unsuccessful merge/collapse), and (3) differential migration (individuals move to more successful groups).

**Key Papers:**  
Boyd & Richerson, *Culture and the Evolutionary Process* (1985); Henrich, *The Secret of Our Success* (2015)

**Mechanism:**  
- Each agent has a discrete set of cultural traits: {norm_honesty, punishment_tolerance, risk_aversion, ...}
- Traits spread via vertical (parent→child), oblique (elder→younger unrelated), and horizontal (peer→peer) transmission
- Group fitness = aggregate of member well-being (resource surplus, offspring survival)
- Every N ticks, high-fitness groups attract more immigrants; low-fitness groups lose members or fission

**Input Channels:**
- Cultural trait distribution (per agent: vector of discrete traits, each 0–K alleles)
- Group affiliation (agent → group ID)
- Group resource surplus (emergent from hunting/gathering/agriculture)

**Output Channels:**
- Group boundaries (emergent coalitions)
- Cultural trait frequency (per group, per biome)
- Group fission/fusion events (discrete demographic shocks)

**Complexity:**  
Medium-high. Requires per-agent trait vector + per-group aggregation. O(A + G) where A = agents, G = groups.

**Determinism:**  
Partially deterministic with stochastic transmission. To preserve determinism:
- Use replica-exchange MCMC or fixed mutation schedule (not random sampling)
- Seeded PRNG for vertical transmission based on (parent_id, locus)
- Group fission deterministic: highest-fitness group reproduces if size > threshold

**Game Application:**
Implement faction formation without hardcoded faction archetypes. Agents carry small trait vectors (5–10 binary cultural traits). Trait transmission uses tilted-urn models (Boyd & Richerson): agents copy traits from successful in-group members more often than random. Groups form via threshold clustering (agents with >70% trait similarity join a group). Factions emerge from geographic clustering + trait inheritance. No hardcoded governance types; governance evolves from trait combinations.

**Tradeoffs:**
- **Pro:** Emergent factions, explains cultural diversity, handles group extinction and fusion
- **Con:** Stochastic (cultural transmission is inherently random); requires careful PRNG seeding for determinism; groups can fragment/merge unexpectedly (not a bug, but complicates state representation)

---

### 9. Coalition & Alliance Emergence (ABM + Alignment)

**Description:**  
Groups (agents, tribes, factions) form coalitions opportunistically, driven by shared enemies, trade benefits, or kinship. Coalition structure emerges from pairwise alignment and is recomputed each epoch.

**Key Papers:**  
Axelrod, *Dissemination of Culture* (1997); Cederman, *Ethnic Conflict and State Formation* (ABM, 2002)

**Mechanism:**  
- Each agent/group has a continuous policy vector P = [militarism, openness, insularity, ...] ∈ [0,1]^d
- Alignment(A, B) = 1 - d(P_A, P_B) (Euclidean distance in policy space)
- If alignment > threshold and both groups have military surplus, they form a defense pact
- Pact benefits: combined military power, mutual defense against third parties
- Cost: commitment to aid (resources paid when ally attacked)
- Every epoch (~20 ticks), recompute coalitions: agents defect from misaligned pacts, form new ones

**Input Channels:**
- Group policy vector (emergent from cultural traits + leadership)
- Military power (population × training × morale)
- Enemy list (other groups that have attacked this group in recent history)

**Output Channels:**
- Coalition membership (graph: directed edges A→B mean "B defends A")
- Trade relationships (can emerge as byproduct of alliance)
- War duration & scale (coalitions engage larger conflicts)

**Complexity:**  
Medium. O(G²) pairwise alignment checks each epoch; coalition graph maintenance O(G) each epoch.

**Determinism:**  
Fully deterministic if:
- Policy vectors are quantized (Q8.8 fixed-point)
- Coalition formation rule is deterministic threshold (alignment > 0.7 AND military_surplus > 0)
- Iteration order is sorted (e.g., by group ID)

**Game Application:**
Every 20 ticks, recompute coalition graph. Groups with >0.6 alignment and overlapping enemies form pacts. Pacts persist until misalignment grows or an ally defects (strategic departure if pact costs > benefits). Wars are automatically declared when coalitions collide over territory. Victory redistributes territory and changes prestige (affects alignment for future recruitment).

**Tradeoffs:**
- **Pro:** Emergent warfare, stable multi-group politics, explains alliance-swapping
- **Con:** Determinism requires careful quantization of policy vectors; policy-space dimensionality can be hard to tune; coalitions can be unstable if alignment is near threshold

---

### 10. Kinship Emergence from Genealogy (Algebraic + ABM)

**Description:**  
Kinship terms and social rules (who can marry, inheritance rules, group membership) emerge from genealogical calculations and reproduction rules, not from hardcoded templates.

**Key Papers:**  
Read, "Kinship Algebra Expert System" (KAES); recent ABM work in *Proc. Roy. Soc. B* (2021) on kinship emergence

**Mechanism:**  
Store pedigree explicitly: each agent has (father_id, mother_id, birth_tick).  
Compute kinship distance recursively: kindist(A, B) = shortest path in pedigree.  
Define marriage rules algebraically:
- Patrilineal: agents share father → clan membership; marry outside clan → exogamy
- Matrilineal: agents share mother → group membership; residence patrilocal or matrilocal based on rule string
- Bilateral: all relatives weighted equally

**Input Channels:**
- Pedigree (father, mother per agent; updated at each birth)
- Residence rule (e.g., "postmarital residence: patrilocal" → bride moves to groom's household)
- Marriage rule (e.g., "cross-cousin marriage preferred" → cousin-distance weighted in mate selection)

**Output Channels:**
- Kinship coefficients (pairwise; used for preferential cooperation, nepotism)
- Household/clan boundaries (emergent from residence rules + pedigree)
- Inheritance schedules (property flows along kinship lines per rule)

**Complexity:**  
Medium. Pedigree is a DAG (acyclic). Kinship distance O(log N) with cached LCA (lowest common ancestor). Household aggregation O(N) per tick.

**Determinism:**  
Fully deterministic. Pedigree is deterministic (reproduction is seeded by parent IDs). Kinship computation is pure function of pedigree. Household rules are deterministic.

**Game Application:**
Embed genealogy tracking in the genetics system. Each agent records (father_id, mother_id). In the social reasoning stage (Stage 7), compute household membership using a residence rule (e.g., matrilocal = newlyweds live with bride's mother). Allow kinship-based provisioning (parents feed offspring, reciprocal altruism among cousins). Clans/lineages emerge as connected components of the pedigree DAG, clustered by shared ancestor and residence rule. No hardcoded clans; they are derived structures.

**Tradeoffs:**
- **Pro:** Fully generative, explains clan structure and exogamy, handles complex inheritance rules
- **Con:** Pedigree grows unboundedly (prune old ancestors to O(1000s)); kinship distance computation can be expensive if LCA cache misses; need careful integration with mating system

---

### 11. Continuous Governance Dimensions (No Enums)

**Description:**  
Replace {DEMOCRACY, MONARCHY, TRIBAL_COUNCIL} enums with continuous dimensions: centralization (0=fully decentralized, 1=absolute autocracy), formalization (0=custom/verbal, 1=codified law), succession-rule entropy, etc. Governance type emerges from position in policy space.

**Key Papers:**  
Polity IV dataset & V-Dem Project (Coppedge et al.); Turchin, *Ultrasociety* (cliodynamic perspective)

**Mechanism:**  
Define governance vector G = [centralization, formalization, transparency, succession_entropy, ...] ∈ [0,1]^d.  
Governance type is derived post-hoc from cluster in policy space:
- (high centralization, low succession_entropy) → monarchy
- (low centralization, low formalization) → tribal council
- (high transparency, high formalization) → democracy
- (high centralization, high formalization) → bureaucratic state

Governance evolves via cultural drift + selection:
1. Centralization increases if leaders have stable succession (low entropy)
2. Formalization increases if population > 1000 (scales need written law)
3. Transparency increases if groups are economically interdependent (trade)

**Input Channels:**
- Group size (population)
- Trade volume (inter-group exchange)
- Leadership stability (how many years since succession crisis)
- Economic surplus (affects capacity for bureaucracy)

**Output Channels:**
- Governance vector (5–10 continuous dimensions)
- Governance type (derived; for narrative/UI only)
- Institutional strength (efficiency of resource extraction, law enforcement)

**Complexity:**  
Low-medium. Per-group vector updates; O(G) per tick.

**Determinism:**  
Fully deterministic if policy transitions are deterministic functions of state (e.g., "if population > 1000 AND formalization < 0.5, increment formalization by 0.01 per tick").

**Game Application:**
Track centralization, formalization, succession_entropy for each faction. Update these per tick based on state (population size, leader age, trade partnerships). In narrative/UI layer, map (centralization, formalization) to a governance type string for storytelling. Institutions like "enforced trade tax" or "hereditary priesthood" emerge from high formalization + specific cultural traits. Revolutions happen when centralization grows too fast (population mismatch).

**Tradeoffs:**
- **Pro:** Captures governance diversity, emergent institutional complexity, explains state formation
- **Con:** Dimensionality is subjective; requires tuning which state variables feed which policy dimensions; risk of oversimplifying real governance

---

### 12. Dual Inheritance / Gene-Culture Coevolution

**Description:**  
Culture (transmitted socially) and genes (transmitted reproductively) coevolve. Cultural practices create novel selection pressures on genes; genetic evolution changes the capacity for cultural learning.

**Key Papers:**  
Cavalli-Sforza & Feldman, *Cultural Transmission and Evolution* (1981); Boyd & Richerson, *Culture and the Evolutionary Process* (1985)

**Mechanism:**  
- Genetic inheritance: sexual reproduction, Mendelian segregation, standard population genetics
- Cultural inheritance: vertical (parent→offspring), horizontal (peer→peer), oblique (elder→younger)
- Example: lactase persistence gene (LCT) spread rapidly in pastoral cultures (which practice dairying culturally) but slowly in non-dairy cultures
- Reverse: culturally-transmitted cooking reduces need for large teeth → genetic selection for smaller jaws

**Input Channels:**
- Allele frequencies (per locus, per population)
- Cultural trait frequencies (per group)
- Fitness function: f(genotype, culture) = compound function that couples both

**Output Channels:**
- Allele frequency shifts (gene-level evolution)
- Trait frequency shifts (culture-level evolution)
- Phenotype distribution (combined outcome)

**Complexity:**  
High. Requires tracking both genotype frequencies and cultural trait frequencies, plus cross-inheritance fitness tables.

**Determinism:**  
Fully deterministic if:
- Fitness function is deterministic (computed from quantized allele freq + trait freq)
- Mutation is seeded by entity ID and locus
- Selection via tilted-urn model (fixed PRNG seed per locus)

**Game Application:**
Implement joint evolution in the genetics + culture stages (Stages 1–2). For a few key loci (e.g., brain size, mate-preference genes), compute fitness as a function of both genotype and cultural practices (e.g., "high-brain-size alleles have higher fitness if literacy_trait is prevalent"). Use selection replay (same random seed for same locus each generation) to maintain determinism. Allows modeling of feedback loops like "complex tool-use culture → selection for larger brains → higher innovation rate → faster cultural evolution."

**Tradeoffs:**
- **Pro:** Explains rapid evolutionary change (culture speeds up evolution), explains genetic patterns in real populations (e.g., lactase persistence geographies)
- **Con:** Computationally expensive; requires defining cultural-locus fitness interactions; easily produces unexpected feedback loops

---

### 13. Chiefdom/State Formation via Circumscription + Cliodynamics

**Description:**  
Circumscribed resources (limited good land) + population growth → warfare → centralization → chiefdom/state formation. Formalizes Carneiro's theory with continuous variables and agent-based modeling.

**Key Papers:**  
Carneiro, "A theory of the origin of the state" (1970); Turchin, *Ultrasociety* (2016); recent ABM tests in *PNAS* (2024)

**Mechanism:**  
Per biome:
1. **Environmental circumscription** = edge_ratio (perimeter of productive land / productive area). High edge ratio (island, oasis, valley) → high circumscription; low ratio (continent) → low
2. **Population pressure** = current_population / biome_carrying_capacity
3. **Conquest probability** = f(circumscription, pressure, neighbor military power). If pressure > 0.8 AND circumscription > 0.6, warfare intensity increases
4. **Centralization** increases as warfare intensity increases (need military hierarchy)
5. **Group formation** occurs when multiple tribes are conquered and merged under one chief → chiefdom

State formation occurs when chiefdoms compete and one achieves dominance, plus a revenue system (taxation) emerges → monopoly on violence + institutionalized extraction.

**Input Channels:**
- Biome carrying capacity (fixed; determines max population before overpopulation)
- Biome edge ratio (environmental circumscription; fixed at map initialization)
- Population size (per group)
- Neighbor military power (inter-group threat)

**Output Channels:**
- Warfare intensity (war frequency, duration, casualties)
- Centralization index (leadership hierarchy depth)
- State-hood flag (true if: centralization > 0.7 AND taxation > 0.5 AND population > 5000)

**Complexity:**  
Medium-high. Requires per-biome carrying-capacity model + per-group warfare simulation + centralization tracking.

**Determinism:**  
Fully deterministic if:
- Carrying capacity is quantized (J32.32 total energy per biome per tick)
- Circumscription is precomputed (map property)
- Warfare outcome is seeded by (group_id_1, group_id_2, tick)

**Game Application:**
In each biome, compute carrying capacity from productivity (from decomposer pools + primary production). Track population pressure; when pressure > threshold, increase conflict probability. Use a simple ABM for warfare: two groups fight, winner annexes territory, loser disperses or becomes vassal. Centralization increases with each conquest; once centralization > 0.7 AND population > 5000, the group becomes a "state" (gains taxation abilities, creates structured hierarchy). Allows modeling of empire rise and fall without hardcoding the state as a separate entity type.

**Tradeoffs:**
- **Pro:** Explains state formation from first principles, predicts where states should form (circumscribed fertile regions), handles conquest and collapse
- **Con:** Warfare is inherently stochastic (outcome depends on morale, tactics); requires seeded randomness for determinism; can produce unexpected dynamics if carrying capacity is wrong

---

### 14. Iterated Learning & Cultural Transmission (Language/Norm Emergence)

**Description:**  
Norms and communication systems evolve through iterated learning: each generation learns by observing the previous generation's behavior, introducing noise (learning error) that biases the outcome. Over generations, this creates structure (e.g., phoneme inventories in language, simplified norm systems).

**Key Papers:**  
Kirby et al., "Iterated learning and the evolution of language," *PNAS* (2007); Smith & Kirby, *Iterated Learning Framework* (2008)

**Mechanism:**  
1. Agent A produces a behavior (e.g., a signal, a norm, a recipe)
2. Agent B learns from A's behavior + noise (perceptual error, memory loss, creative reinterpretation)
3. Agent B's version becomes the new exemplar
4. B teaches C, and so on

Over 20+ generations, the behavior space shrinks: noise is aligned toward stable attractor states (e.g., symmetric phonemes, simple kinship rules). Complex rules drop out; simple, regular structures remain.

**Input Channels:**
- Prototype behavior (per cultural trait: a signal, a norm, a recipe as a rule string)
- Learning error distribution (Gaussian noise on behavior space)
- Population size (affects transmission fidelity)
- Generation count (for novelty measurement)

**Output Channels:**
- Learned behavior (agent's version of the trait)
- Regularity (entropy of behavior distribution; decreases over time)
- Innovation (novel combinations that emerge from noise)

**Complexity:**  
Low-medium if behavior is discrete (rule strings, finite symbol inventory). O(A × traits) per tick.

**Determinism:**  
Fully deterministic if:
- Learning noise is seeded by (teacher_id, learner_id, locus)
- Behavior is discrete (e.g., rule ID, not continuous) or quantized

**Game Application:**
When an adult teaches a young agent a cultural trait, add noise proportional to trait complexity. Simple traits (e.g., "always hunt in groups") transmit accurately; complex traits (e.g., "hunt in groups *unless* prey density < 0.5 AND moon phase is new *AND* river is swollen") degrade quickly. Traits that survive >50 generations become "stable norms"; those that collapse are forgotten. Allows modeling of oral culture simplification and cultural punctuation (sudden adoption of simple, memorable norms after chaotic period). Norms like "endogamy" emerge from learning errors in the kinship system.

**Tradeoffs:**
- **Pro:** Explains why some norms are universal (emerge from iterated learning), predicts cultural structure, handles cultural drift
- **Con:** Requires defining behavior space (continuous or discrete); learning error is subjective; can produce unexpected stable states (local attractors)

---

## CROSS-MODEL TRADEOFF MATRIX

| Model | Complexity | Determinism | Emergence | Tuning Risk | Game Applicability |
|-------|-----------|-------------|-----------|------------|-------------------|
| **Williams-Martinez** | Low | Perfect | Food-web | Low | High (replace diet hardcoding) |
| **ATN/ADBM** | Medium | Perfect | Diet breadth + interaction strength | Medium | High (mechanism for energy flow) |
| **DEB Theory** | High | Perfect* | Metabolism + growth + reproduction | High | Medium (added state overhead) |
| **Continuous TP** | Low | Perfect | Trophic level | Low | High (replace trophic-level enum) |
| **Eco-Evo** | Very High | Risky** | Trait evolution + ecology coupling | High | Low (unless pop >> 1000) |
| **Trait-Based Niche** | Medium | Perfect | Niche differentiation | Medium | High (complement ATN) |
| **Decomposer** | Medium | Perfect | Nutrient cycling + productivity | Medium | High (replace fixed efficiency) |
| **CMLS** | Medium | Risky*** | Faction formation | Medium | High (replace faction enum) |
| **Coalition** | Medium | Perfect | Alliance structure | Medium | High (emergent politics) |
| **Kinship** | Medium | Perfect | Clan + inheritance | Low | High (replace clan enum) |
| **Governance** | Low | Perfect | Institutional complexity | Medium | High (replace governance enum) |
| **Dual Inheritance** | High | Perfect* | Gene-culture feedback | High | Medium (complex fitness) |
| **Chiefdom Formation** | Medium | Perfect* | State emergence | Medium | High (replace state enum) |
| **Iterated Learning** | Low | Perfect | Norm simplification | Low | High (replace hardcoded norms) |

**Legend:**  
*Perfect: Deterministic with fixed-point arithmetic and seeded PRNG.  
**Risky: Stochastic system; determinism requires careful PRNG seeding; small parameter changes cause large behavioral shifts.  
***Risky: Requires stochastic cultural transmission; determinism doable but fragile.

---

## INTEGRATION ROADMAP

### Phase 1 (Sprint S3–S4): Replace Hardcoded Parameters with Mechanistic Models
1. Implement **Williams-Martinez niche model** for diet formation (Stage 2)
2. Replace `ENERGY_TRANSFER_EFFICIENCY=0.10` with **ATN/ADBM** interaction strengths (Stage 2)
3. Add **continuous trophic position** calculation (Stage 7, Labeling)
4. Implement per-biome **decomposer pools** (Stage 6, Ecology)

### Phase 2 (Sprint S5–S6): Trait-Based Niche & Eco-Evo
5. Implement **trait-based niche competition** (Stage 4, Interaction)
6. Light **eco-evolutionary feedback** for key traits (Stage 2, Genetics)
7. Implement **DEB theory** for individual metabolism (Stage 5, Physiology) — *optional, if performance allows*

### Phase 3 (Sprint S7–S8): Social Structure Emergence
8. Replace faction archetypes with **CMLS** framework (Stage 7, Labeling)
9. Implement **genealogy tracking** for kinship emergence (Stage 1, input; Stage 7, Labeling)
10. Add **continuous governance dimensions** (Stage 7, Labeling)
11. Implement **coalition formation** ABM (Stage 7, Labeling; recompute every 20 ticks)

### Phase 4 (Optional, Sprint S9–S10): Advanced Models
12. **Chiefdom/state formation** via circumscription (long-timescale, Stage 7)
13. **Dual inheritance** gene-culture coevolution (Stages 1–2, Genetics)
14. **Iterated learning** for cultural transmission (Stage 7, when teaching)

---

## IMPLEMENTATION CONSTRAINTS

### Fixed-Point Arithmetic Requirement
All models must accept and produce Q32.32 (or other fixed-point) quantities. Key notes:
- Body mass, energy reserves, metabolic rates: Q32.32
- Allele frequencies, trait frequencies: Q8.8 or Q16.16
- Trophic position, policy dimensions: Q8.8 or Q16.16
- No floating-point in simulation state; floating-point only in render/UI

### Determinism Enforcement
- Every stochastic operation (mutation, learning error, warfare outcome) must be seeded by a deterministic function of entity IDs, locus IDs, and tick number
- Use Xoshiro256++ seeded per (subsystem, tick) at tick boundary; no per-operation reseeding
- Iterate sorted entity lists (by ID) to ensure state does not depend on HashMap iteration order

### Emergence vs. Specification
- No hardcoded faction archetypes, governance types, kinship terminologies, or trophic levels
- Hardcoding is permissible only for rules of transmission (e.g., "reproduction is sexual, vertical transmission is 50% per parent")
- All observable categories (faction type, governance type, trophic level) are derived, not stored

---

## REFERENCES

### Ecology

- Brose, U., Williams, R. B., & Martinez, N. D. (2006). Allometric food webs: Constraint and assembly rules. *Ecology Letters*, 9(7), 853–862.
- Chandel, R., et al. (2023). Microbial models for simulating soil carbon dynamics: A review. *Journal of Geophysical Research: Biogeosciences*, 128, e2023JG007436.
- Kooijman, S. A. L. M. (2010). *Dynamic Energy Budget Theory for Metabolic Organisation*. Cambridge University Press.
- Petchey, O. L., & Gaston, K. J. (2006). Functional diversity: back to basics and looking forward. *Ecology Letters*, 9(7), 741–758.
- Post, D. M. (2002). Using stable isotopes to estimate trophic position: Models, methods, and assumptions. *Ecology*, 83(3), 703–718.
- Williams, R. B., & Martinez, N. D. (2000). Simple rules yield complex food webs. *Nature*, 404, 180–183.

### Eco-Evolutionary Dynamics

- Hairston, N. G., et al. (2005). Rapid evolution and the convergence of species traits. *Ecology Letters*, 8(12), 1114–1124.
- Geritz, S. A. H., Metz, J. A. J., Kisdi, É., & Diekmann, O. (1998). Dynamics of adaptation and evolutionary branching. *Physical Review Letters*, 78(10), 2024.

### Social Structure

- Boyd, R., & Richerson, P. J. (1985). *Culture and the Evolutionary Process*. University of Chicago Press.
- Carneiro, R. L. (1970). A theory of the origin of the state. *Science*, 169(3947), 733–738.
- Cavalli-Sforza, L. L., & Feldman, M. W. (1981). *Cultural Transmission and Evolution: A Quantitative Approach*. Princeton University Press.
- Henrich, J. (2015). *The Secret of Our Success: How Culture Is Driving Human Evolution, Domesticating Our Species, and Making Us Smarter*. Princeton University Press.
- Kirby, S., Cornish, H., & Smith, K. (2008). Cumulative cultural evolution in the laboratory: An experimental approach to the origins of structure in human language. *PNAS*, 105(31), 10681–10686.
- Read, D. W. (2006). Kinship algebra expert system (KAES). *Mathematical Anthropology and Cultural Theory*, 2(4), 1–46.
- Smith, K., & Kirby, S. (2008). Iterated learning: A framework for the emergence of language. *Advances in Complex Systems*, 11(3), 331–346.
- Turchin, P. (2016). *Ultrasociety: How 10,000 Years of War Made Humans the Greatest Cooperators*. Beresta Books.

---

**Document Status:** Complete. Ready for architecture integration and crate-specific implementation planning.  
**Next Steps:** Create per-crate implementation specs (one per selected model); add to IMPLEMENTATION_ARCHITECTURE.md.
