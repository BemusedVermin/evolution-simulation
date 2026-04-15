# System 19: Technology & Innovation

## 1. Overview

Technologies are emergent cultural artifacts—not a pre-authored tech tree unlocking progression gates, but instead a living library of recipe-knowledge bundles that factions discover, lose, and transmit. A **Technology** is a bundle of:
- **Recipes** (from System 05): valid combinations of inputs, techniques, and outputs.
- **Facilities** (from System 04): workshop designs, capital infrastructure requirements.
- **Practices**: behavioral/tactical doctrines (e.g., "shield wall formation").

Technologies are discovered, not researched. When a faction faces suitable conditions—need (shortage of something), curiosity (idle agent time), opportunity (available materials, tools, role models)—agents trigger **experimentation events**. A novel technology is a viable combination of existing recipes/practices not yet in the faction's library. Once discovered, technologies diffuse through social knowledge networks (System 03, F4).

Technologies can be **forgotten** if no living agent practices them and transmission chains break (the Tasmanian effect, System 18). This creates real stakes: ruin a faction's crafters through plague or genocide, and they lose techniques forever. Convergent invention means independent factions may discover identical technologies given similar pressures—this creates plausible shared heritage without requiring a universal tech tree.

**Key principle**: Technologies emerge from agent behavior and environmental pressure, not designer intent. A player's actions (harvesting a new creature, introducing a new tool) can indirectly trigger faction innovation through changed material availability.

---

## 2. Research Basis

### Technological Evolution & Diffusion (Ziman, 2000; Rogers, 1995; Basalla, 1988)
Technology spreads via diffusion curves: early adopters, majority, laggards. Diffusion requires social learning—people must observe, understand, and adopt innovations. Rogers identifies five attributes affecting adoption: relative advantage (how much better), compatibility (fit with existing practices), complexity (effort to learn), trialability (can test), and observability (visible results). Technologies not practiced decay in cultural transmission; knowledge is lost if no practitioner survives.

- Ziman, J. (2000). *Technological Innovation as an Evolutionary Process*. Cambridge University Press.
- Rogers, E.M. (1995). *Diffusion of Innovations* (4th ed.). Free Press.
- Basalla, G. (1988). *The Evolution of Technology*. Cambridge University Press.

**Application**: Technologies are discovered and transmitted like cultural traits (System 18). New technologies have base adoption_difficulty; factions with relevant skills (e.g., metalworking) adopt metalworking-related tech faster. Observability (the technology's result is visible in daily life—e.g., metal tools are obviously better) increases adoption_rate.

### Convergent Evolution & Technological Convergence (Boyd & Richerson, 1985; Mesoudi et al., 2004)
Independent populations under similar selective pressures may develop similar technologies. This is convergent cultural evolution. Metallurgy was invented independently in multiple regions; agriculture emerged independently in ~10 locations. The convergence is not miraculous—similar problems invite similar solutions when physics and ecology are identical.

- Boyd, R. & Richerson, P.J. (1985). *Culture and the Evolutionary Process*. University of Chicago Press.
- Mesoudi, A., Whiten, A., & Laland, K.N. (2004). "Is Human Cultural Evolution Darwinian?" *Evolutionary Biology*, 34.

**Application**: Technologies are defined by their recipe and facility requirements, not by faction ownership. If two factions independently assemble the same recipe (copper ore + fire + skill > copper ingot), they discover the same technology. The game tracks whether the technology is "discovered" (known to at least one faction); newly discovering factions can either invent independently (high experimentation cost) or adopt from neighbors (social learning).

### Technological Dependence & Lock-in (Arthur, 1989; David, 1985)
Technological systems exhibit path dependence: early choices (e.g., adopting bronze over iron) constrain later innovations. Once a technology is entrenched (practitioners trained, facilities built, cultural norms established), switching is costly. Historical example: QWERTY keyboard persists despite inferior ergonomics due to training lock-in.

- Arthur, W.B. (1989). "Competing Technologies, Increasing Returns, and Lock-In by Historical Events." *Economic Journal*, 99(394).
- David, P.A. (1985). "Clio and the Economics of QWERTY." *American Economic Review*, 75(2).

**Application**: Factions with entrenched technologies (e.g., bronze-working practiced by 10+ agents, facilities across settlements) are reluctant to switch to alternative technologies (iron-working). Switching cost = (number of practitioners × retraining effort) + (facility rebuilding cost). This creates historical inertia: factions can be "stuck" in suboptimal technology states.

### Tasmanian Effect & Cultural Ratchet (Henrich, 2004; Boyd & Richerson, 1996)
The Tasmanian Aboriginals gradually lost technologies (e.g., bone tools, fishing) after isolation from mainland Australia, likely due to demographic bottleneck and transmission failure. Humans maintain culture via "ratcheting"—each generation adds to and refines what it inherits. Small populations or low transmission rates lead to loss. In contrast, large, interconnected populations accumulate and preserve complex technologies.

- Henrich, J. (2004). "Demography and Cultural Evolution." *Theoretical Population Biology*, 63(2).
- Boyd, R. & Richerson, P.J. (1996). "Why Culture Is Common, but Cultural Evolution Is Rare." *Proceedings of the British Academy*, 88.

**Application**: Each technology has practitioners_count. When a practitioner dies, practitioners_count decreases. If practitioners_count reaches 0, the technology enters a 500-tick grace period. If no new practitioner is trained, the technology is permanently lost and disappears from all factions' technology libraries. Small factions are vulnerable to losing complex technologies via bad luck (plague kills crafters).

---

## 3. Entities & State

### Technology Structure

```
Technology {
  technology_id: int,
  name: string,  // e.g., "Bronze Working", "Shield Wall Formation"
  category: enum {
    Crafting,      // Recipes + material transformation
    Infrastructure,  // Facility designs (System 04)
    Martial,       // Combat tactics (System 06)
    Navigation,    // Travel + exploration tech (System 07)
    Agriculture,   // Farming + food production (System 12)
    Biological,    // Creature domestication, breeding
    Cultural,      // Art, music, ceremony (System 18)
  },
  description: string,
  discovery_tick: int,           // When first discovered by any faction
  discovering_faction_id: int,   // Original discoverer (historical credit)
  
  // RECIPE BUNDLE
  recipe_ids: [recipe_id, ...],  // Associated crafting recipes (System 05)
  facility_designs: [facility_id, ...],  // Required or enabled facilities (System 04)
  prerequisite_technologies: [tech_id, ...],  // Must discover these first (e.g., fire → smelting)
  
  // DISCOVERY & ADOPTION
  discovery_difficulty: float [0, 1],    // How hard to independently discover
  adoption_difficulty: float [0, 1],     // How hard to learn from others
  base_adoption_rate: float [0, 1],      // How fast practitioners increase when adopted
  observability: float [0, 1],           // How visible/impressive the tech is
  
  // PRACTICE & TRANSMISSION
  practitioners_by_faction: {
    [faction_id]: {
      practitioner_count: int,
      skill_average: float [0, 100],
      last_practitioner_training_tick: int,
    }
  },
  extinction_grace_period_remaining_ticks: int or null,  // Countdown to permanent loss
  
  // IMPACT & FEEDBACK
  recipes_enabled: int,          // Number of recipes this unlocks
  material_productivity_modifier: float,  // Global efficiency boost (e.g., bronze tools 1.2x efficiency)
  settlement_carrying_capacity_modifier: float,  // Infrastructure boost (e.g., granaries 1.5x)
  cultural_prestige_value: float [−1, 1],  // How much the faction's reputation improves (−1 taboo tech)
}
```

### Experimentation Event

Agents trigger experimentation when they are:
1. **Facing a need**: shortage of critical resource or tool.
2. **Idle & curious**: not engaged in subsistence.
3. **In opportunity**: relevant materials and tools available, or witnessing a working prototype.

```
ExperimentationEvent {
  experimenter_agent_id: int,
  target_material_or_problem: string,  // "We need better weapons" or "Copper is too soft"
  
  // Available inputs
  available_materials: [MaterialStack, ...],
  available_recipes_to_recombine: [recipe_id, ...],
  available_agents_to_teach: [agent_id, ...],  // Skilled practitioners to observe
  
  // Outcome computation
  success_rate: float,  // Function of available_resources, agent_creativity, time_invested
  time_to_iterate: int,  // Ticks of experimentation
  
  // Result
  resulting_technology: Technology or null,  // New tech if successful
  feedback: enum {
    Success,          // New viable technology discovered
    PartialSuccess,   // Improvements to existing recipe, not new tech
    Failure,          // Dead end, material wasted
    Convergence,      // Independent rediscovery of known tech
  },
}
```

### Faction Technology Library

```
Faction.technologies = {
  discovered_technologies: [tech_id, ...],   // All known techs (whether practicing or not)
  actively_practicing: [tech_id, ...],       // Currently used/taught
  deprecated_technologies: [tech_id, ...],   // Known but abandoned
  
  technology_preferences: {
    [tech_id]: {
      cultural_alignment: float [−1, 1],  // Taboo? Prestigious? Neutral?
      adoption_priority: float [0, 1],    // How much faction wants to adopt
    }
  },
}
```

---

## 4. Update Rules

### Experimentation Triggering

Each tick, agents with low urgency (needs met, not in combat) consider experimentation:

```
function consider_experimentation(agent: Agent, tick: int):
  if agent.current_urgency < 0.3:  // Low needs satisfaction urgency
    need_for_innovation = compute_innovation_pressure(agent.faction, agent.location)
    if random() < 0.001 * need_for_innovation:  // Low probability per tick
      trigger_experimentation_event(agent, tick)
```

Experimentation pressure is computed from:
```
need_for_innovation = 
  + 0.3 * (resource_shortage_severity)  // Shortage of key resources
  + 0.2 * (quality_deficit vs. observed_enemies)  // Tools worse than rival factions
  + 0.2 * (presence_of_new_materials)  // New materials suggest new possibilities
  + 0.1 * (cultural_curiosity of population)  // Some factions are more experimental
  + 0.2 * (existence of partial_solutions)  // Agent sees incomplete technique and tries to finish it
```

### Experimentation Outcome

```
function resolve_experimentation(event: ExperimentationEvent) -> (success: bool, tech: Technology or null):
  // Estimate success rate
  material_availability_factor = len(event.available_materials) / len(materials_needed_for_typical_tech)
  agent_skill_factor = event.experimenter_agent.get_average_technique_skill()
  time_available_factor = event.available_time / MIN_EXPERIMENTATION_TIME  // At least 5–10 ticks
  
  success_rate = 0.05 * (  // Base 5% per tick of experimentation
    material_availability_factor * 0.4 +
    agent_skill_factor * 0.4 +
    time_available_factor * 0.2
  )
  
  // Iterate
  for attempt_tick in range(event.time_to_iterate):
    if random() < success_rate:
      outcome = determine_outcome_type(event)
      if outcome == Success:
        new_tech = synthesize_technology_from_recipes_and_materials(event)
        event.experimenter_agent.faction.discovered_technologies.add(new_tech)
        return (true, new_tech)
      elif outcome == Convergence:
        // Independent rediscovery
        return (true, already_known_tech)
      elif outcome == PartialSuccess:
        // Improvement to existing recipe
        for recipe in event.available_recipes_to_recombine:
          recipe.efficiency_modifier *= 1.05
        return (false, null)
  
  // Failure: materials wasted, agent gains experience but no new tech
  event.experimenter_agent.techniques[crafting].skill_points += 5  // Consolation prize
  return (false, null)

function synthesize_technology_from_recipes_and_materials(event: ExperimentationEvent) -> Technology:
  // Group available recipes by theme (e.g., all metalworking recipes)
  recipe_clusters = cluster_by_category(event.available_recipes_to_recombine)
  
  // Pick a cluster and combine inputs/outputs innovatively
  chosen_cluster = random_choice(recipe_clusters)
  base_inputs = union(recipe.inputs for recipe in chosen_cluster)
  base_outputs = union(recipe.outputs for recipe in chosen_cluster)
  
  // Create new tech: bundle of these recipes + novel combination
  new_recipe = synthesize_novel_recipe(base_inputs, base_outputs, event.target_material_or_problem)
  new_tech = Technology(
    name = generate_name(new_recipe, event.experimenter_agent.faction),
    recipe_ids = chosen_cluster + [new_recipe],
    discovery_difficulty = 0.3 + 0.4 * len(new_recipe.inputs),  // More complex = harder to discover independently
    adoption_difficulty = 0.15 + 0.2 * len(new_recipe.inputs),
    base_adoption_rate = 0.02 + 0.01 * observability_of_output,
    observability = 0.5 if new_recipe.output.is_visible else 0.1,
  )
  return new_tech
```

### Diffusion & Adoption

Once a technology is discovered, it spreads to neighboring factions via social networks:

```
function diffuse_technology(tech: Technology, source_faction: Faction, tick: int):
  for neighbor_faction in source_faction.opinion_neighbors():  // Factions with opinion links
    if neighbor_faction not in tech.practitioners_by_faction:
      // Neighbor hasn't discovered this tech yet
      
      // Adoption probability based on contact intensity and tech attractiveness
      contact_intensity = abs(source_faction.opinion[neighbor_faction])
      tech_advantage = tech.recipes_enabled + tech.observability  // How visible/useful
      knowledge_transfer_propensity = source_faction.get_knowledge_transfer_tendency()
      
      adoption_probability = 0.05 * contact_intensity * tech_advantage * knowledge_transfer_propensity
      
      if random() < adoption_probability:
        neighbor_faction.discovered_technologies.add(tech.technology_id)
        // Start adoption: spawn practitioners
        new_practitioners = max(1, int(contact_intensity))
        tech.practitioners_by_faction[neighbor_faction].practitioner_count = new_practitioners
        // Slower initial learning
        tech.practitioners_by_faction[neighbor_faction].skill_average = 0.2

function increase_practitioners_of_adopted_tech(tech: Technology, faction: Faction, tick: int):
  // Existing practitioners train new practitioners (agents choose to learn)
  if tech.practitioners_by_faction[faction].practitioner_count > 0:
    existing_practitioners = tech.practitioners_by_faction[faction].practitioner_count
    available_learners = [agent for agent in faction.agents if agent.can_learn_technique()]
    
    new_practitioners_rate = 0.1 * existing_practitioners  // 10% per existing practitioner per tick
    new_practitioners_count = int(min(new_practitioners_rate, len(available_learners)))
    
    for _ in range(new_practitioners_count):
      learner = random_choice(available_learners)
      learner.techniques[tech.category].skill_points += 0.5  // Apprenticeship
      tech.practitioners_by_faction[faction].practitioner_count += 1
```

### Transmission & Loss (Tasmanian Effect)

During lifecycle events (death, System 13):

```
function on_practitioner_death(agent: Agent, tech: Technology):
  tech.practitioners_by_faction[agent.faction].practitioner_count -= 1
  
  if tech.practitioners_by_faction[agent.faction].practitioner_count == 0:
    // Last practitioner dead; initiate grace period
    tech.extinction_grace_period_remaining_ticks = EXTINCTION_GRACE_PERIOD  // 500 ticks

function tick_extinction_grace_period(tech: Technology, tick: int):
  for (faction, practitioner_data) in tech.practitioners_by_faction:
    if practitioner_data.practitioner_count == 0:
      tech.extinction_grace_period_remaining_ticks -= 1
      
      if tech.extinction_grace_period_remaining_ticks <= 0:
        // No practitioner trained in time; technology extinct
        remove tech from all factions' technology_libraries
        # Lore event: "The ancient [tech name] was lost with the death of [last practitioner]"
        chronicle_extinction_event(tech, faction)

function transmit_technology_to_offspring(parent: Agent, child: Agent):
  for tech in parent.faction.discovered_technologies:
    if parent.knows_technology(tech):  // Parent is a practitioner
      if random() < 0.8:  // High transmission rate for skills
        child.faction.discovered_technologies.ensure_contains(tech)
        # Child doesn't start as practitioner but knows it's possible
```

### Technology Lock-in & Adoption Inertia

Technologies with many practitioners are resistant to replacement:

```
function compute_switching_cost(faction: Faction, old_tech: Technology, new_tech: Technology) -> float:
  retraining_cost = (
    old_tech.practitioners_by_faction[faction].practitioner_count *
    len(new_tech.recipe_ids) *  // Recipes to learn
    0.5  // Retraining effort multiplier
  )
  facility_rebuilding_cost = len(new_tech.facility_designs) * 100  // Facility redesign cost
  cultural_switching_cost = 1.0 if new_tech.cultural_prestige_value < old_tech.cultural_prestige_value else 0.0
  
  return retraining_cost + facility_rebuilding_cost + cultural_switching_cost

function adoption_faction_considers_switching(faction: Faction, old_tech: Technology, new_tech: Technology):
  switching_cost = compute_switching_cost(faction, old_tech, new_tech)
  benefit_upgrade = (new_tech.material_productivity_modifier - old_tech.material_productivity_modifier) * 100
  
  net_value = benefit_upgrade - switching_cost
  if net_value > 0 and not faction.is_too_stressed():
    faction.deprecate_technology(old_tech)
    faction.adopt_technology(new_tech)
```

---

## 5. Cross-System Hooks

**System 01 (Evolution)**: New materials from evolved creatures can trigger experimentation. A player hunts a creature with novel properties; the creature's harvested materials have unusual signatures (System 04), suggesting new recipes. Agents experimentally combine these materials.

**System 04 (Economy & Crafting)**: Technologies are bundles of recipes. When a technology is adopted, new recipes become available to crafters. Material productivity modifiers from technologies apply directly to recipe success rates and output quality.

**System 05 (Crafting)**: Technologies enable recipes. A recipe might be "unlocked" by discovering the required technology. Recipes can be improved (efficiency +5%) as agents experiment and refine. System 05 recipes maintain a base_discovery_tick and technology_required field.

**System 06 (Combat)**: Martial technologies (shield wall, cavalry tactics, magical incantations) unlock new combat abilities and formations. Factions with advanced martial tech have harder combats. Beasts can "discover" behaviors (System 01 evolution) that mimic discovered martial technologies (convergent evolution of tactics).

**System 07 (Exploration)**: Navigation technologies (boats, compasses, maps) enable exploration of new terrain types. Discovering ocean navigation opens archipelago travel. Technologies can require specific biome resources (System 15) or creature resources (System 01).

**System 09 (World History)**: Chronicle records tech discovery and loss. "The Bronze Age began when Clan Sacer discovered bronze working in tick 1,200." "The knowledge of glass was forever lost with the extinction of the Lume faction in tick 5,432, when their last craftsperson, old Mirga, passed away."

**System 12 (Ecology)**: Agriculture technologies increase carrying capacity of settlements (System 04). Domestication technologies reduce wild creature threat and increase food security.

**System 18 (Language & Culture)**: New technologies introduce vocabulary (System 18). When metalworking is discovered, words like "ore," "smelter," "alloy," "bronze" emerge in the discovering faction's language. Technology prestige affects cultural attractiveness.

**System 20 (Migration)**: Technologies influence migration decisions. Factions with advanced agricultural tech can support larger settlements and are less desperate to migrate. Loss of technology via plague can trigger migration as population starves.

---

## 6. Tradeoff Matrix

| Dimension | Choice | Rationale |
|---|---|---|
| **Discovery Mechanism** | Designed blueprints (tech tree) vs. emergent synthesis (this doc) | Emergent is less predictable but more novel. Chosen: emergent synthesis from recipes. |
| **Convergent Invention** | Allowed vs. prevented | Allowing it creates plausible independent discovery; preventing it requires careful tech-tree design. Chosen: allowed—same recipe bundle = same tech. |
| **Experimentation Cost** | Resource-consuming (materials wasted) vs. free (knowledge only) | Wasting resources creates scarcity and weight; free is faster. Chosen: resource-consuming (materials expended, time consumed). |
| **Transmission Fidelity** | Decay with each generation vs. perfect copies | Decay creates drift; perfect copies preserve tech unchanged. Chosen: decay (skill_average decays if not practiced) but recipes remain correct. |
| **Extinction Trigger** | Immediate (last practitioner dies) vs. grace period (500 ticks) | Grace period allows cultural memory and heroic saving; immediate is harsh. Chosen: grace period. |
| **Lock-in Mechanics** | Strong (switching is very costly) vs. weak (easy to switch) | Strong creates historical inertia; weak allows rapid adaptation. Chosen: strong (switching cost = practitioners × retraining + infrastructure). |
| **Observability Feedback** | Visible techs spread faster vs. all spread equally | Visible is realistic (people adopt what they can see working). Chosen: visible techs spread 3-5x faster. |

---

## 7. Emergent Properties

- **Technology Ratchet**: Factions accumulate technologies over time, rarely losing them (unless population crashes). Large, interconnected factions are "ratchet civilizations" that compound innovations; small, isolated factions risk losing technologies and regressing.

- **Path Dependence**: Early adoption of a technology (e.g., bronze) entrenches a faction on that path. Switching to iron working is costly. A faction might remain bronze-using long after iron is available elsewhere because of lock-in—even when inferior, it's the known path.

- **Convergent Vs. Divergent Evolution**: Two independent factions might discover identical bronze working (convergent), giving credit to both. But a third faction might discover a different metallurgical path (divergent), leading to technological diversity without a single "correct" progression.

- **Crisis-Driven Innovation**: A famine triggers experimentation with new food crops; a predator invasion triggers martial innovation. Historical crises become moments of rapid technological change, visible in chronicles.

- **Technological Prestige**: Factions with impressive, visible technologies (e.g., towering aqueducts, decorated armor) gain cultural prestige and attraction. Players learn that technology is not just functional—it's a status signal and reputation driver.

- **Knowledge Hoarding**: A faction with a monopoly on a valuable technology (e.g., metalworking) can leverage it politically. Refusing to teach rivals maintains superiority but also breeds resentment. Trade and spying become mechanisms to steal knowledge.

---

## 8. Open Calibration Knobs

- **EXPERIMENTATION_TRIGGER_PROBABILITY**: Base probability per tick of starting experimentation (currently 0.001 × innovation_pressure). Increase to speed up tech discovery; decrease to slow down technological change.

- **EXPERIMENTATION_SUCCESS_RATE_PER_TICK**: Base success rate during experimentation (currently 0.05). Increase for faster breakthroughs; decrease to require more time and resources.

- **EXTINCTION_GRACE_PERIOD**: Ticks before extinct tech is permanently lost (currently 500, ~1 game year). Increase to allow heroic last-minute recovery; decrease to make loss final quickly.

- **ADOPTION_RATE_MULTIPLIER**: How fast adopted technologies gain practitioners (currently 0.1 per existing practitioner per tick). Increase to speed technology spread; decrease to slow adoption.

- **OBSERVABILITY_IMPACT**: Multiplier on adoption rate for visible techs (currently 3-5x). Increase to make visible techs dominant; decrease to randomize adoption.

- **SWITCHING_COST_MULTIPLIER**: Scaling on retraining cost when switching techs (currently 0.5). Increase to create stronger lock-in; decrease to allow rapid technological switches.

- **KNOWLEDGE_TRANSFER_TENDENCY_BY_FACTION**: Per-faction parameter controlling how willing they are to teach neighbors (currently 0.3–0.9). Set per faction to model isolationism vs. openness.

