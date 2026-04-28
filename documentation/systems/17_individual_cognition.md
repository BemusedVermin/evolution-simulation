# System 17: Individual Cognition, Memory & Learning

> **Subsumed under emergence/57_agent_ai.md and emergence/60_culture_emergence.md (P6d).** The memory subsystems described here — episodic, spatial, procedural, Bayesian belief updating, Ebbinghaus forgetting, combat-ability anticipation — remain valid in their behaviour, but the *implementation* is reframed as factor-graph marginals on the agent's discrete factored POMDP (doc 57):
> - `episodic_log` → marginals on the "past-event" factor; consolidation = pruning low-belief entries.
> - `spatial_map` → marginals on per-cell factors of the spatial layer of the factor graph.
> - `known_techniques` → entries in the agent's `action_skill` proficiency vector; skill acquisition = Bayesian update on the proficiency factor.
> - `learned_creature_abilities` → reduced generative models in the `tom_cache` for non-sapient species.
> - `local_knowledge_facts` → KnowledgeFacts are observations on the corresponding factor's likelihood; **the Bayesian update in §4.2 below IS the active-inference perceptual update step**.
>
> The cognition-tier enum (reactive / deliberative / reflective) is removed per P6d; tiers are 1-NN Chronicler labels over `(predictive_horizon_ticks, model_depth, theory_of_mind_order)`. Per Invariant 9, all decision-making routes through doc 57's policy posterior; this doc's memory APIs remain user-facing.

## 1. Overview

Individual cognition captures how agents (both human NPCs and sapient beasts) learn, remember, and adapt from direct experience. Unlike the faction-level knowledge diffusion (System 03, F4), this system models *per-agent* memory structures and online learning. An agent's memory forms through three independent systems:

1. **Episodic Memory**: discrete events with emotional weight and decay over time.
2. **Spatial Memory**: known locations, paths, hazards, resource caches—persistent mental maps.
3. **Procedural Memory**: learned skills and techniques (techniques[][skill_points]), plus learned creature-ability patterns.

The central principle: **agents learn from observation**. When an agent witnesses an event (predation, crafting technique, combat ability, creature behavior), they update internal KnowledgeFacts (System 03) via Bayesian belief updates. This creates online adaptation: agents encountering a beast species for the first time form initial beliefs; repeated encounters refine those beliefs. In combat, agents develop anticipatory expectations: *"This species has used acid spray the last three encounters; I should expect it again."*

Memory consolidation occurs during rest (System 13, lifecycle). Episodic memories relevant to spatial/procedural contexts are tagged for long-term retention. Forgetting follows Ebbinghaus curves: recent, emotionally charged events persist; mundane facts decay to noise over weeks.

**Key design principle**: No "god-mode" NPC omniscience. NPCs and sapient creatures have bounded, biased, evolving mental models of the world. This creates realism and player agency: the player can mislead factions through misdirection, can exploit NPC misunderstandings, can surprise even known enemies with novel tactics.

---

## 2. Research Basis

### Episodic Memory & Consolidation (Tulving, 1983; Dudai, 2002)
Episodic memory stores *what happened, where, when*—context-rich traces of experiences. Consolidation—the process of stabilizing memories—occurs especially during sleep, where emotional weighting determines retention priority. High-emotion events are rehearsed more during consolidation, creating stronger long-term memories.

- Tulving, E. (1983). *Elements of Episodic Memory*. Oxford University Press.
- Dudai, Y. (2002). "Memory from A to Z." Oxford University Press.

**Application**: Agents form episodic memories of witnessed events with an emotion_weight ∈ [0, 1]. During rest ticks, memories with high emotion_weight are consolidated into long-term storage; others decay. A death in an agent's faction (high emotion_weight) is consolidated and persists. Routine resource gathering (low emotion_weight) fades within days.

### Spatial Cognition & Mental Maps (Tolman, 1948; O'Keefe & Nadel, 1978)
Animals (and humans) construct mental maps of spatial environments. Place cells in the hippocampus fire at specific locations; grid cells encode relative distance and direction. Mental maps support navigation, route planning, and spatial memory for hazards. Maps are updated online as exploration reveals new areas.

- Tolman, E.C. (1948). "Cognitive Maps in Rats and Men." *Psychological Review*, 55(4).
- O'Keefe, J. & Nadel, L. (1978). *The Hippocampus as a Cognitive Map*. Oxford University Press.

**Application**: Each agent maintains a spatial_memory_map with entries: `(location_id, confidence, resource_signature, hazard_level, visited_count, last_visited_tick)`. The agent updates this online during exploration. Routes between familiar locations are memoized; when a new path is found, the agent updates distance estimates. High-confidence locations (visited many times) are used for long-term planning; low-confidence entries may be forgotten or revisited.

### Procedural Memory & Skill Learning (Cohen & Squire, 1980; Doyon & Ungerleider, 2002)
Procedural memory is the memory of *how* to do things—motor skills, techniques, action sequences. Unlike episodic memory, procedural memories are implicit; they are retrieved automatically during performance. Skill learning involves repetition and error correction, encoded in cerebellum and basal ganglia circuits, not hippocampus.

- Cohen, N.L. & Squire, L.R. (1980). "Preserved Learning and Retention of Pattern Analyzing Skill in Amnesia." *Science*, 210(4466).
- Doyon, J. & Ungerleider, L.G. (2002). "Functional Brain Imaging of Motor Performance." *Current Opinion in Neurobiology*, 12(2).

**Application**: Agent.techniques[][skill_points] grows through practice. Agents practicing a technique (e.g., ironworking) gain incremental skill_points. Unrelated to episodic memory—the agent doesn't consciously recall *learning* the skill; they simply perform it better. After extended disuse (weeks in-game), procedural memories degrade slightly.

### Bayesian Belief Updating & Observer Learning (Rescorla & Wagner, 1972; Dayan et al., 2000)
Animals update beliefs about environmental relationships through experience. The Rescorla-Wagner model quantifies belief update: prediction error (observed − expected) drives learning. Modern Bayesian formulations extend this: when an agent observes data, they update a prior belief distribution over hypotheses (e.g., "Does this creature use poison?") using the likelihood of the data under each hypothesis.

- Rescorla, R.A. & Wagner, A.R. (1972). "A Theory of Pavlovian Conditioning." In *Classical Conditioning: Current Research and Theory*.
- Dayan, P., Kakade, S., & Montague, P.R. (2000). "Learning and Selective Attention." *Nature Reviews Neuroscience*, 1(3).

**Application**: When an agent observes an event (creature uses ability, technique succeeds, NPC trades), they perform Bayesian update on a KnowledgeFact. A KnowledgeFact `"SaberCat_A uses SonicScreech"` has a confidence [0, 1]. Observing the ability increases confidence; repeated use reinforces high confidence. Absence of observation over time (weeks) reduces confidence toward base-rate prior. Prediction error (expected ability not observed; unexpected ability observed) drives larger updates.

### Ebbinghaus Forgetting Curve (Ebbinghaus, 1885; Cepeda et al., 2006)
The Ebbinghaus curve models memory retention decay: retention ≈ e^(-t/S), where t is elapsed time and S is a strength constant. Rehearsal (encountering information again) resets the curve, allowing rapid re-learning. The curve is non-linear: early forgetting is steep; later forgetting flattens.

- Ebbinghaus, H. (1885). *Memory: A Contribution to Experimental Psychology*. Dover (1964 reprint).
- Cepeda, N.J., et al. (2006). "Distributed Practice in Verbal Recall Tasks: A Review and Quantitative Synthesis." *Psychological Bulletin*, 132(3).

**Application**: Episodic memories decay via `memory_strength(t) = strength_0 × e^(−t / consolidation_time_ticks)`. Consolidation time is proportional to emotion_weight. A traumatic event (emotion_weight = 0.9) may consolidate over 3 ticks; a mundane fact over 30 ticks. Re-encounter resets the consolidation timer.

### Combatant Ability Anticipation (Heyes, 2012; Wolpert et al., 2001)
Predicting opponents' actions requires learned forward models of their behavior. Repeated interaction builds implicit models: "When opponent is at low health, they use desperate move X." This is a form of procedural learning applied to social prediction. In sports, athletes develop opponent-specific game plans through repeated play.

- Heyes, C.M. (2012). "New Thinking: The Evolution of Human Cognition." *Philosophical Transactions of the Royal Society B*, 367(1599).
- Wolpert, D.M., et al. (2001). "Computational Principles of Movement Neuroscience." *Nature Reviews Neuroscience*, 2(12).

**Application**: Each combat encounter logs observed abilities used. On rematch (same creature species or individual), agents maintain learned_ability_patterns: `(ability_id, encounter_count, recent_frequency, predicted_next_use_tick)`. If a creature has used ability X three times in recent encounters, the agent anticipates ability X and adjusts defense or positioning accordingly.

---

## 3. Entities & State

### Agent Memory Structures

```
AgentMind.memory = {
  // EPISODIC MEMORY: Recent events with decay
  episodic_log: [
    {
      event_id: int,
      tick_observed: int,
      event_type: enum {
        CreatureObserved,
        CombatOutcome,
        CraftingSuccess,
        DeathWitnessed,
        ResourceFound,
        NPCInteraction,
        FactionEvent,
      },
      subject_id: id,             // creature_id, npc_id, etc.
      details: {
        // Contextual data varies by event_type
        creature_species?: species_id,
        ability_used?: ability_id,
        outcome?: enum { Success, Failure, Unknown },
        location_id?: location_id,
        witnesses?: [agent_id, ...],
      },
      emotion_weight: float [0, 1],  // Importance for consolidation
      strength: float [0, 1],        // Current mnemonic strength; decays over time
      tags: [string, ...],           // For search (e.g., "predation", "technique_X")
    }
  ],
  episodic_consolidation_queue: [
    {
      memory_id: int,
      consolidation_deadline_tick: int,  // tick when consolidation decision made
      likelyhood_of_retention: float,    // (emotion_weight); >0.5 → likely retained
    }
  ],

  // SPATIAL MEMORY: Known locations and routes
  spatial_map: {
    [location_id]: {
      location_name: string,
      coords: (x, y),
      confidence: float [0, 1],        // 1.0 = visited many times; 0.5 = heard rumors
      resource_signature: MaterialSignature or null,  // If known
      hazard_level: float [0, 1],      // 0 = safe; 1 = deadly
      visited_count: int,
      last_visited_tick: int,
      distance_estimates_to_other_locations: Dict[location_id, float],
    }
  },
  active_route: {
    from_location_id: id,
    to_location_id: id,
    waypoints: [location_id, ...],
    estimated_cost_ticks: int,
  } or null,

  // PROCEDURAL MEMORY: Learned techniques and abilities
  known_techniques: [
    {
      technique_id: id,
      skill_points: float [0, 100],  // 0 = novice; 100 = master
      last_practiced_tick: int,       // For decay if unused
      efficiency_modifier: float,      // (1.0 + 0.01*skill_points); derived during use
    }
  ],
  learned_creature_abilities: {
    [species_id]: {
      ability_id: ability_id,
      encounter_count: int,
      recent_use_frequency: float [0, 1],  // Frequency in last 10 encounters
      confidence: float [0, 1],
      predicted_next_use_tick: int or null,  // Forecast for combat
    }
  },

  // KNOWLEDGE FACTS (shared with faction, but individual copies + local updates)
  local_knowledge_facts: [
    KnowledgeFact, ...
  ],
  confidence_overrides: {
    [fact_id]: float [0, 1],  // Agent-specific confidence (may differ from faction consensus)
  },

  // WORKING MEMORY / ATTENTION
  current_focus: {
    focus_type: enum { Task, Threat, Interest, Boredom },
    target_id: id,
    attention_strength: float [0, 1],
    ticks_focused: int,
  } or null,
}
```

### Memory Update Events

Whenever an agent takes an action (move, observe, craft, combat, dialogue), the memory subsystem processes updates:

```
function on_agent_observes_event(agent: Agent, event: Event):
  // 1. Create episodic trace
  ep_trace = EpisodicTrace(
    event_type = event.type,
    subject_id = event.subject_id,
    details = event.details,
    emotion_weight = compute_emotion_weight(event, agent.faction_opinion, agent.needs),
    tick_observed = world.current_tick
  )
  agent.memory.episodic_log.append(ep_trace)

  // 2. Update spatial memory if location observed
  if event.location_id:
    location_entry = agent.memory.spatial_map[event.location_id]
    location_entry.confidence = min(1.0, location_entry.confidence + 0.05)
    location_entry.last_visited_tick = world.current_tick
    if event.type == CreatureObserved:
      location_entry.hazard_level = max(location_entry.hazard_level, creature.danger_rating)

  // 3. Bayesian update on KnowledgeFacts
  for fact in agent.memory.local_knowledge_facts:
    if fact.relates_to(event.subject_id):
      prior_confidence = agent.memory.confidence_overrides[fact.id] or fact.global_confidence
      likelihood = fact.compute_likelihood(event)  // P(observation | fact_true)
      posterior = prior_confidence * likelihood / (posterior_normalizer)
      agent.memory.confidence_overrides[fact.id] = posterior

  // 4. Update procedural patterns (combat)
  if event.type == CombatOutcome:
    for ability in event.creature_abilities_used:
      pattern = agent.memory.learned_creature_abilities[event.creature_species][ability]
      pattern.encounter_count += 1
      pattern.recent_use_frequency = smooth_exponential(
        alpha=0.15,
        new_observation=1.0,
        old_frequency=pattern.recent_use_frequency
      )
      pattern.confidence = min(1.0, pattern.confidence + 0.1)
```

### Consolidation During Rest

During sleep/rest ticks (System 13), episodic memories are consolidated:

```
function consolidate_episodic_memory(agent: Agent, tick: int):
  // Iterate over memories in consolidation queue
  for queued_memory in agent.memory.episodic_consolidation_queue:
    elapsed_ticks = tick - queued_memory.queued_at_tick
    decay_rate = exp(-elapsed_ticks / 3)  // ~3-tick consolidation baseline
    memory_object = agent.memory.episodic_log[queued_memory.memory_id]
    
    if memory_object.emotion_weight * decay_rate > CONSOLIDATION_THRESHOLD (0.3):
      // Move to long-term storage
      agent.memory.long_term_episodic_store.append(memory_object)
      mark_for_rehearsal = true
    else:
      // Forget: decay strength to near-zero
      memory_object.strength *= 0.1
      if memory_object.strength < 0.01:
        agent.memory.episodic_log.remove(memory_object)
  
  // Rehearse memories related to current goals/contexts
  for long_term_memory in agent.memory.long_term_episodic_store:
    if is_contextually_relevant(long_term_memory, agent.current_focus):
      long_term_memory.strength = min(1.0, long_term_memory.strength + 0.2)
```

---

## 4. Update Rules

### Episodic Decay During Normal Ticks

Every tick, episodic memories decay unless rehearsed:

```
for memory in agent.memory.episodic_log:
  if memory.id not in recently_rehearsed_set:
    memory.strength *= exp(-1.0 / memory.consolidation_strength_ticks)
    if memory.strength < MEMORY_DISAPPEAR_THRESHOLD (0.01):
      remove memory from log
```

Consolidation strength is initially `consolidation_strength_ticks = 5 + 20 * memory.emotion_weight`, creating a range of 5–25 ticks baseline.

### Bayesian Update Formula

When an agent observes an event related to a KnowledgeFact:

```
fact_prior_confidence = C_old
event_likelihood = likelihood(observation | fact == true)  // Manually computed per fact type
posterior_confidence = (fact_prior_confidence * event_likelihood) / Z

where Z = fact_prior_confidence * event_likelihood + (1 - fact_prior_confidence) * (1 - event_likelihood)
```

Example: Agent believes "SaberCat uses SonicScreech" with confidence 0.6. Agent observes SaberCat without using SonicScreech. Likelihood of observation given fact is true: 0.3 (it has the ability, but doesn't always use it). Posterior:

```
C_new = (0.6 * 0.3) / (0.6 * 0.3 + 0.4 * 0.7) = 0.18 / 0.46 ≈ 0.39
```

Confidence drops, but not drastically—a single non-use is weak evidence against the fact.

### Combat Ability Anticipation

Before combat with a known creature species/individual, the agent computes anticipation_bonus:

```
for ability in learned_creature_abilities[species_id]:
  ability_score = ability.recent_use_frequency * ability.confidence
  defense_modifier_vs_ability = 1.0 + 0.2 * ability_score
  
apply defense_modifier_vs_ability to dodge/parry against that specific ability
```

---

## 5. Cross-System Hooks

**System 03 (Faction/Social)**: Individual memory feeds faction KnowledgeStore. Agents observe events; each agent's local confidence overrides feed into faction consensus confidence via (sum of local confidences) / (number of observers). Agents who have high-confidence facts propagate them more strongly during faction meetings (System 03, F4).

**System 06 (Combat)**: Before combat initiation, combatants query their learned_creature_abilities. Known opponents trigger ability anticipation; unknown opponents use species base-rates (from faction knowledge or procedural defaults). Combat moves update episodic memory with high emotion_weight; near-death experiences create strong memories.

**System 08 (Dialogue)**: NPC dialogue can reference episodic memories. *"I remember when your faction nearly starved in 2142. I still have nightmares about it."* References are probabilistic—NPCs don't recall events with perfect accuracy, but they recall with confidence proportional to emotion_weight. This creates emergent storytelling.

**System 13 (Reproduction/Lifecycle)**: Rest periods trigger consolidation. Agents in extreme stress (high threat) or high happiness (successful reproduction) consolidate memories more aggressively. Sleep duration affects consolidation depth.

**System 14 (Calendar/Time)**: Seasonal changes and long-term climate shifts reset some spatial memory confidence (e.g., "This forest was dense; now it's sparse"). Decades-old memories have baseline confidence degradation.

**Faction-level learning vs. Individual learning**: Agents feed *high-confidence* local facts into faction knowledge network. But agents also maintain private, possibly idiosyncratic beliefs that differ from faction consensus. This creates friction: an NPC might secretly believe a creature is more dangerous than the faction officially recognizes, causing them to avoid certain hunts.

---

## 6. Tradeoff Matrix

| Dimension | Choice | Rationale |
|---|---|---|
| **Episodic Decay Function** | Ebbinghaus exponential vs. simple linear decay | Exponential is more realistic (early forgetting steep); linear is faster to compute. Chosen: exponential, but with lookup table caching. |
| **Consolidation Timing** | Every rest tick vs. dedicated consolidation events | Every rest tick is seamless and couples to lifecycle. Chosen: every rest tick. |
| **Bayesian Update Detail** | Full likelihood computation vs. heuristic update | Full likelihood requires hand-tuned per-fact-type likelihood functions. Heuristic (e.g., increase confidence by fixed amount per observation) is faster but less realistic. Chosen: hybrid—pre-computed likelihood tables for common fact types. |
| **Spatial Memory Granularity** | Per-location vs. per-tile | Per-location (from System 07) avoids explosion of memory. Chosen: per-location with tile-level hazard annotations. |
| **Combat Anticipation Scope** | Species-level patterns vs. individual-creature tracking | Species-level patterns scale; individual tracking is detailed but memory-heavy. Chosen: species-level + individual overrides for named creatures. |
| **Conflict Resolution** | Faction consensus vs. individual local override | Faction consensus allows knowledge diffusion; individual override allows personality divergence. Both coexist; agents weight their own observations more heavily than faction rumors. |

---

## 7. Emergent Properties

- **NPC Learning Curve**: Players encounter NPCs who visibly improve against them over time. A vendor who initially underestimates the player learns their combat style and adjusts prices/trades. This mirrors real-world dynamics without explicit "difficulty scaling."

- **False Memories & Confirmation Bias**: Agents can develop strong false beliefs if they receive misleading observations early (high emotion_weight) and then update Bayesianly. A creature mistakenly believed dangerous will be approached cautiously even if it's harmless, until strong counter-evidence accumulates.

- **Knowledge Fragmentation**: The same fact (e.g., "This cave is safe") has different confidences across agents and the faction, creating realistic disagreement and debate. Factions are not omniscient monoliths.

- **Combat Surprise**: Even known enemies can surprise players if they learn new abilities (mutation from System 01) or if the player's anticipation models are stale (long time since last encounter, forgetting curves kicked in).

- **Procedural Skill Transfer**: Agents practicing ironworking don't consciously learn theory; they just get better. Later, when they try a related technique (e.g., metalcasting), they start with a skill_point bonus (transfer of procedural learning). This emerges without explicit "cross-skill inheritance"—simply the reality that procedural knowledge is flexible.

---

## 8. Open Calibration Knobs

- **CONSOLIDATION_THRESHOLD**: Probability threshold determining whether episodic memory is retained (currently 0.3). Raising it (e.g., 0.5) makes agents forgetful; lowering it (e.g., 0.15) makes them retain more. Tune to control how "forgetful" the world feels.

- **MEMORY_DECAY_CONSTANT**: Ebbinghaus decay rate constant S (currently 5–25 ticks depending on emotion_weight). Increase for slower forgetting; decrease for faster. Affects long-term replayability: players can hide from old mistakes or become infamous forever.

- **CONSOLIDATION_BASELINE**: Baseline consolidation time in ticks (currently 3). Increase if episodic consolidation is bottlenecking computation; decrease to speed up memory formation.

- **BAYESIAN_LIKELIHOOD_STRENGTH**: Scaling factor on likelihood contributions (currently 1.0). Reduce if confidence updates are too volatile.

- **COMBAT_ANTICIPATION_WEIGHT**: Multiplier on ability_score for defense bonus (currently 0.2). Increase to make learned opponents significantly harder; decrease if players should be able to brute-force through opponent learning.

- **SPATIAL_CONFIDENCE_INCREMENT**: How much each visit increases location confidence (currently 0.05). Increase to make locations "feel known" faster; decrease for slow familiarity.

