# Faction & Social Dynamics: Emergent Politics & Cooperation

> **Superseded by emergence/56_relationship_graph_emergence.md (and emergence/50_social_emergence.md for the cultural/governance carriers).** Specifically:
> - `Agent.faction_id` (single-slot political membership) is **removed**. Memberships are derived from edge-clusters in the typed agent-pair multigraph; agents may sit in many overlapping clusters at multiple Leiden levels.
> - The 12-axis fixed opinion space is replaced by P5's continuous `cultural_trait_vector` (32 axes by default, axes labelled post-hoc per doc 58).
> - Treaty-type and governance-type enums become Chronicler 1-NN labels over continuous coalition / governance channels.
> - Faction-affinity probabilities used by this doc's interaction logic now read from doc 57's active-inference engine (per Invariant 9).
>
> This doc remains useful as a reference for what the v1 design specified; the listed mechanics should be re-read through the emergence-doc lens.

## 1. Overview

This document specifies how NPCs (humans and sapient beasts) form relationships, develop opinions, coalesce into factions, and make collective decisions. The system is grounded in real-world social dynamics research and designed for **emergent politics**: no faction types are hardcoded. Instead, factions crystallize from social network dynamics and opinion convergence.

The system operates on two tiers:

1. **Sapient Layer** (full opinion dynamics, political structures): humans + beasts with neural_speed > 0.6
2. **Beast Layer** (lightweight social structure): non-sapient beasts with < 0.6 neural_speed

Social dynamics are independent of the evolutionary model but *coupled* through knowledge transfer (NPCs observe beast traits, update their understanding, which affects hunting behavior, which affects beast fitness).

---

## 2. Research Basis

**Deffuant-Weisbuch Opinion Dynamics (Deffuant et al. 2000; Li et al. 2024)**
Pairwise interactions where agents move toward each other's opinions if they are within a confidence bound. Adaptive confidence bounds (widening with positive interaction, narrowing with conflict) produce rich dynamics: polarization, consensus, bridging across ideological divides. More tractable for games than Hegselmann-Krause (all-agents averaging).

**Weak Structural Balance (Davis 1967; Friedkin et al. 2017)**
Triadic relationships ("a friend of my friend is my friend") don't always hold. Weak balance allows 2+ factions rather than forcing two-faction splits. Empirical data show "enemy of my enemy is my friend" is weakly supported. Produces realistic multi-factional political landscapes.

**Axelrod's Culture Model (Axelrod 1997)**
Agents homophilic (interact more with culturally similar neighbors) and assimilate (copy one feature from interaction partner). Despite local convergence, global diversity emerges—stable heterogeneous regions. Maps to faction formation: local opinion convergence produces global factional structure.

**Multiplex Social Networks (Battiston et al. 2017; Kivelä et al. 2014)**
Different relationships exist on different layers (kinship, economic, ideological). The same pair can be economic partners yet ideological enemies. Multi-layer structure prevents total consensus: contradictions exist and create tension.

**Coalition Formation Theory (Ray & Vohra 2014; Acemoglu et al. 2008)**
Coalitions form when members are better off together than alone; split when internal disagreement exceeds cohesion. Governance emerges from power distributions (who makes decisions). Not game-theoretic equilibrium, but dynamic cycling.

**Information Diffusion & Cascades (Zhuang et al. 2025; PNAS 2025)**
Facts spread via SEIR model (epidemic-like). Ideas spread via opinion-modulated diffusion (must resonate with existing beliefs). Independent spreaders (ambitious agents) generate novel ideas autonomously, preventing opinion stagnation. Social reinforcement (hearing a fact multiple times increases confidence) plus fatigue (hearing too many sources causes skepticism) create realistic information dynamics.

**Sapience Gradient (Kokko & Heubel 2011; Reader & Laland 2002)**
Not a binary (sapient/non-sapient). Sapience is continuous—cognition and agency scale with neural capacity. Low-sapience agents follow simple rules; high-sapience agents hold abstract beliefs and adapt strategy to social context.

---

## 3. Entities & State

### Layer F1: Social Graph

```
SocialGraph {
    agents: list<AgentNode>                     // all NPCs
    edges: Map<(agent_id, agent_id), RelationshipEdge>
    spatial_index: SpatialHash               // for proximity queries
}

AgentNode {
    id: unique_id
    agent_type: enum { Human, SapientBeast }
    location: Vec3
    faction_id: faction_id or null           // if any
    sapience: float                          // 0..1, derived from neural_speed channel
    social_reach: float                      // max distance for interaction
    time_joined_simulation: tick              // when spawned/born into world
    alive: bool
}

RelationshipEdge {
    agent_a_id: id
    agent_b_id: id
    
    // Valence: -1 (enemies) to +1 (allies)
    valence: float
    
    // Familiarity: how well do they know each other?
    familiarity: float                       // 0..1
    
    // Trust: distinct from liking; can trust an enemy
    trust: float                             // 0..1
    
    last_interaction_tick: tick
    interaction_count: int
    
    // Multi-layer relationships (from Battiston et al. 2017)
    context_layers: float[NUM_SOCIAL_LAYERS]
    // NUM_SOCIAL_LAYERS = 6:
    // 0: survival (shared danger, mutual defense)
    // 1: economic (trade, resource sharing)
    // 2: kinship (family, clan, pack bonds)
    // 3: ideological (shared beliefs, worldview)
    // 4: hierarchical (dominance, deference, authority)
    // 5: informational (knowledge sharing, teaching)
}
```

**Relationship Decay:**

```pseudocode
function decay_relationship(edge, current_tick):
    time_since = current_tick - edge.last_interaction_tick
    
    // Decay rate depends on familiarity (close friends reconnect easily)
    decay_rate = BASE_DECAY * (1 - edge.familiarity * 0.5)
    
    // Exponential decay with half-life parameter
    decay_factor = exp(-decay_rate * time_since)
    
    edge.valence *= decay_factor
    edge.familiarity *= decay_factor ^ 0.5  // familiarity decays slower
    edge.trust *= decay_factor ^ 0.3        // trust decays slowest
    
    // Clamp to prevent negative familiarity
    edge.familiarity = max(0, edge.familiarity)
    edge.trust = max(0, edge.trust)
```

**Edge Formation:**

```pseudocode
function form_edge(agent_A, agent_B, interaction_type):
    if edge exists: return (update existing)
    if not co_located(A, B) or distance(A, B) > max(A.social_reach, B.social_reach):
        return (too far)
    
    // Probability of forming edge based on:
    // 1. Proximity (closer = higher)
    // 2. Cultural similarity (homophily)
    // 3. Triadic closure (mutual friends = higher)
    // 4. Faction membership (same faction = higher)
    
    p_form = 0.01  // base per-tick
    p_form *= (1 - distance(A, B) / max_social_reach)  // proximity boost
    p_form *= (1 + opinion_similarity(A, B) * 0.3)     // homophily
    
    if mutual_friends(A, B) > 0:
        p_form *= (1 + mutual_friends(A, B) * 0.1)  // triadic closure
    
    if A.faction_id == B.faction_id and A.faction_id != null:
        p_form *= 2.0  // same faction = faster bonding
    
    if random() < p_form:
        create_edge(A, B, initial_valence=0.2)
```

---

### Layer F2: Agent Mind

```
AgentMind {
    // 12 opinion dimensions (abstract, unnamed in code)
    opinions: float[NUM_OPINION_DIMS]    // each in [-1.0, +1.0]
    
    // Per-dimension openness to influence (adaptive BCM, Li et al. 2024)
    confidence: float[NUM_OPINION_DIMS]  // each in [MIN_CONF, MAX_CONF]
    
    // Knowledge about the world
    knowledge: KnowledgeStore
    
    // Current priorities (Maslow-like hierarchy)
    needs: NeedsVector
    
    // Stable personality traits
    personality: PersonalityVector
    
    // Behavioral record (for role detection)
    behavior_profile: BehaviorAccumulator
}

// Opinion dimensions (example set; tunable)
OPINION_DIMS = [
    0: individual_vs_collective,       // private property ↔ communal resources
    1: hoarding_vs_distribution,       // accumulate wealth ↔ redistribute
    2: local_vs_global_trade,          // protectionism ↔ free exchange
    3: hierarchy_vs_egalitarianism,    // strong leadership ↔ flat structure
    4: tradition_vs_innovation,        // preserve customs ↔ embrace change
    5: isolation_vs_expansion,         // defend borders ↔ explore/conquer
    6: in_group_loyalty,               // prioritize faction ↔ universal cooperation
    7: aggression_vs_diplomacy,        // force ↔ negotiation
    8: risk_tolerance,                 // cautious ↔ bold/experimental
    9: beast_exploitation_vs_coexistence, // dominate nature ↔ live alongside
    10: beast_knowledge_priority,      // study beasts ↔ ignore/destroy them
    11: player_cooperation,            // ally with player ↔ oppose player
]

Confidence bounds:
    MIN_CONFIDENCE = 0.05              // minimum openness to influence
    MAX_CONFIDENCE = 0.8               // maximum divergence tolerated

KnowledgeStore {
    // (fact_key) -> (fact_value, confidence, timestamp, source_trust, times_heard)
    world_facts: Map<string, KnowledgeFact>
    
    // Beast knowledge by species
    beast_knowledge: Map<species_id, BeastKnowledge>
    
    // Soft beliefs about other agents
    social_knowledge: Map<agent_id, AgentKnowledge>
    
    // Regional knowledge (dangers, resources, opportunities)
    region_knowledge: Map<biome_cell_id, RegionKnowledge>
}

KnowledgeFact {
    value: any
    confidence: float               // degrades over time
    freshness_tick: tick           // when last verified
    source_trust: float            // how much to trust the source
    times_heard: int               // social reinforcement counter
}

BeastKnowledge {
    threat_level: float            // how dangerous (0..1)
    observed_abilities: list<str>  // "poison", "fire", "regen", etc.
    weakness_hints: list<str>      // observed vulnerabilities
    location_history: Map<tick, Vec3>  // where sighted (temporal)
    last_sighting_tick: tick
    estimated_population: int or null
}

NeedsVector {
    survival: float               // food, shelter, safety
    economic: float              // trade, resources, wealth
    social: float                // belonging, status, reputation
    ideological: float           // desire to see beliefs enacted
    curiosity: float             // exploration, learning
    dominance: float             // authority, influence
}

PersonalityVector {
    stubbornness: float          // resistance to opinion change (0..1)
    charisma: float              // influence on others (0..1)
    empathy: float               // weight given to others' needs (0..1)
    ambition: float              // drive for status/power (0..1)
    volatility: float            // emotional reactivity (0..1)
    sociability: float           // seeking new interactions (0..1)
}

BehaviorAccumulator {
    // Exponentially weighted averages (recent actions weighted more)
    time_in_combat: float
    time_in_trade: float
    time_in_crafting: float
    time_in_diplomacy: float
    time_in_teaching: float
    time_in_exploration: float
    time_in_command: float
    time_in_ritual: float
    decisions_influenced: int
    resources_controlled: float
    followers: int
}
```

---

### Layer F3: Interaction Dynamics

**Interaction Types & Opinion Activation:**

```
InteractionType = enum {
    Casual,             // gossip, small talk → opinion_dims [0..2] (economic)
    Trade,              // exchange goods → opinion_dims [1..2] (economic + trade)
    SharedLabor,        // work together → opinion_dims [0, 3, 6] (collective, hierarchy, loyalty)
    SharedDanger,       // face threat → opinion_dims [3, 6, 7] (hierarchy, loyalty, aggression)
    Debate,             // discuss ideas → opinion_dims [*] (all dimensions, if salient)
    Teaching,           // knowledge transfer → opinion_dims [4, 10] (innovation, beast knowledge)
    Ritual,             // group ceremony → opinion_dims [3, 4, 6] (hierarchy, tradition, loyalty)
    Conflict,           // argument/fight → opinion_dims [7] (aggression) + relationship damage
    Command,            // hierarchical order → opinion_dims [3] (hierarchy) + edge update
}
```

**Opinion Update Rule** (modified Deffuant-Weisbuch with adaptive bounds):

```pseudocode
function update_opinions_on_interaction(agent_A, agent_B, interaction_type):
    active_dims = get_active_dimensions(interaction_type)
    
    for d in active_dims:
        distance = abs(A.opinions[d] - B.opinions[d])
        
        // Bounded confidence: do they listen?
        A_receptive = distance < A.confidence[d]
        B_receptive = distance < B.confidence[d]
        
        if A_receptive:
            // A moves toward B's opinion
            influence_strength = compute_influence(B, A, d, interaction_type)
            A.opinions[d] += influence_strength * (B.opinions[d] - A.opinions[d])
        
        if B_receptive:
            // B moves toward A's opinion
            influence_strength = compute_influence(A, B, d, interaction_type)
            B.opinions[d] += influence_strength * (A.opinions[d] - B.opinions[d])
        
        // Adaptive confidence bounds (Li et al. 2024)
        if A_receptive or B_receptive:
            // Positive interaction: widen confidence (become more open-minded)
            A.confidence[d] += CONFIDENCE_GROWTH * 0.01
            B.confidence[d] += CONFIDENCE_GROWTH * 0.01
        else:
            // Exposure to distant opinion: narrow confidence (become entrenched)
            A.confidence[d] -= CONFIDENCE_SHRINK * 0.005
            B.confidence[d] -= CONFIDENCE_SHRINK * 0.005
        
        // Clamp confidence bounds
        A.confidence[d] = clamp(A.confidence[d], MIN_CONFIDENCE, MAX_CONFIDENCE)
        B.confidence[d] = clamp(B.confidence[d], MIN_CONFIDENCE, MAX_CONFIDENCE)

function compute_influence(influencer, target, dim, interaction_type) -> float:
    base_influence = 0.05  // small per-interaction step
    
    // Personality modifiers
    base_influence *= influencer.personality.charisma
    base_influence *= (1 - target.personality.stubbornness)
    
    // Relationship strength modifier
    edge = get_edge(influencer, target)
    if edge != null:
        // Trust amplifies influence, but familiarity is also required
        base_influence *= (0.5 + edge.trust * 0.5)
        // Layer-specific: influence stronger on relevant layer
        relevant_layer = dim_to_layer(dim)
        base_influence *= (0.5 + edge.context_layers[relevant_layer] * 0.5)
    
    // Needs modifier: dimensions aligned with salient needs are more susceptible
    need_salience = get_dimension_salience(target, dim)
    base_influence *= need_salience
    
    // GROUP PRESSURE (higher-order interaction from PLoS ONE 2025)
    // In group settings (Debate, Ritual), group agreement amplifies influence
    if interaction_type in [Debate, Ritual]:
        nearby_agreement = count_nearby_agents_agreeing_with(influencer, dim)
        base_influence *= (1 + nearby_agreement * GROUP_PRESSURE_FACTOR)
    
    return clamp(base_influence, 0, MAX_INFLUENCE_PER_STEP)
```

**Dimension Salience Mapping** (which opinion dimensions are currently salient to each agent):

```
// Needs → Dimension Salience Matrix (6 needs × 12 dimensions)
SALIENCE[need][dim] =

Survival need (when health < 50% or starving):
    [1.0, 0.2, 0.3, 0.4, 0.1, 0.2, 0.5, 0.7, 0.3, 0.4, 0.1, 0.2]
    // high salience: collective aid (0), hierarchy-favoring (3), 
    //   aggression (7), beast danger (9), not ideological

Economic need (when resources < threshold):
    [0.8, 0.9, 0.8, 0.3, 0.2, 0.4, 0.2, 0.2, 0.4, 0.2, 0.2, 0.1]
    // high salience: economic dims (0, 1, 2)

Social need (when isolation > threshold):
    [0.3, 0.2, 0.3, 0.4, 0.5, 0.3, 0.9, 0.4, 0.2, 0.2, 0.1, 0.3]
    // high salience: loyalty (6), hierarchy (3), tradition (4)

Ideological need (when free & safe):
    [0.6, 0.5, 0.6, 0.5, 0.8, 0.7, 0.4, 0.5, 0.6, 0.8, 0.7, 0.3]
    // high salience: all political dims

Curiosity need (when safe & have time):
    [0.4, 0.3, 0.4, 0.3, 0.7, 0.8, 0.3, 0.2, 0.5, 0.4, 0.8, 0.2]
    // high salience: innovation (4), expansion (5), beast knowledge (10)

Dominance need (in competitive situations):
    [0.5, 0.4, 0.3, 0.9, 0.4, 0.6, 0.6, 0.7, 0.8, 0.5, 0.3, 0.4]
    // high salience: hierarchy (3), expansion (5), aggression (7), risk (8)
```

Rationale: Agents under threat prioritize collective security and dominance, not ideology. Comfortable agents debate ideas. This produces realistic conflict escalation and peace-building.

**Relationship Update After Interaction:**

```pseudocode
function update_relationship(A, B, interaction_type, outcome):
    edge = get_or_create_edge(A, B)
    
    // Valence (positive/negative): shared experience outcomes
    valence_delta = compute_valence_delta(interaction_type, outcome, A.opinions, B.opinions)
    edge.valence += valence_delta * VALENCE_LEARNING_RATE
    edge.valence = clamp(edge.valence, -1.0, 1.0)
    
    // Trust: predictable, honest behavior → trust
    //         betrayal → large trust decrease
    trust_delta = compute_trust_delta(interaction_type, outcome, edge.valence)
    edge.trust += trust_delta * TRUST_LEARNING_RATE
    edge.trust = clamp(edge.trust, 0.0, 1.0)
    
    // Familiarity: always increases with interaction
    edge.familiarity = min(1.0, edge.familiarity + (1 - edge.familiarity) * FAMILIARITY_GROWTH)
    edge.last_interaction_tick = current_tick
    edge.interaction_count += 1
    
    // Context layer: strengthen the relevant layer
    layer = interaction_type_to_layer(interaction_type)
    edge.context_layers[layer] = min(1.0, edge.context_layers[layer] + LAYER_STRENGTH_GROWTH)
    
    // STRUCTURAL BALANCE: check triads and apply corrective pressure
    apply_weak_structural_balance(A, B, edge)

function apply_weak_structural_balance(A, B, edge_AB):
    // Weak balance (Davis 1967): for each mutual acquaintance C,
    // check if A-B-C triad is balanced. If not, apply pressure.
    
    for C in mutual_acquaintances(A, B):
        edge_AC = get_edge(A, C)
        edge_BC = get_edge(B, C)
        
        if edge_AC == null or edge_BC == null:
            continue
        
        // Triadic product: positive = balanced, negative = unbalanced
        product = sign(edge_AB.valence) * sign(edge_AC.valence) * sign(edge_BC.valence)
        
        if product < 0:  // unbalanced
            // Find weakest edge (least familiar, lowest trust)
            edges = [edge_AB, edge_AC, edge_BC]
            weakest = min_by(edges, key=lambda e: e.familiarity + e.trust)
            
            // Push weakest toward balance
            // "A likes C and C likes B → A should like B"
            target_sign = sign(edge_AC.valence) * sign(edge_BC.valence)
            pressure_magnitude = BALANCE_PRESSURE * (1 - abs(weakest.valence))
            weakest.valence += target_sign * pressure_magnitude
            weakest.valence = clamp(weakest.valence, -1.0, 1.0)
```

Rationale: Weak (not strong) balance allows multi-faction structures. Triadic pressure gradually organizes random relationships into coherent faction-like clusters.

---

### Layer F4: Information & Idea Diffusion

**Knowledge Diffusion (SEIR Model):**

```pseudocode
function diffuse_knowledge(social_graph, tick):
    for each active_interaction(A, B):
        // A might share knowledge with B
        for fact in A.knowledge where fact.confidence > SHARE_THRESHOLD:
            if B has not yet encountered fact:
                // Probability of exposure
                p_expose = A.personality.charisma
                         * relevance_to_interaction(fact, interaction_type)
                         * B.needs.curiosity
                         * get_trust(A, B)
                
                if random() < p_expose:
                    B.knowledge[fact] = KnowledgeFact(
                        value=fact.value,
                        confidence=fact.confidence * TRANSFER_CONFIDENCE_LOSS,
                        source_trust=get_trust(A, B),
                        times_heard=1,
                        state=Exposed  // B has heard but not internalized
                    )
            
            elif B.knowledge[fact].state == Exposed:
                // SOCIAL REINFORCEMENT: hearing again increases confidence
                B.knowledge[fact].times_heard += 1
                B.knowledge[fact].confidence += REINFORCEMENT_PER_SOURCE * (get_trust(A, B) - 0.5)
                if B.knowledge[fact].confidence >= INTERNALIZATION_THRESHOLD:
                    B.knowledge[fact].state = Informed  // B can now share it
    
    // Decay: knowledge fades without reinforcement
    for agent in all_agents:
        for fact in agent.knowledge:
            time_since_verified = current_tick - fact.freshness_tick
            fact.confidence *= exp(-KNOWLEDGE_DECAY_RATE * time_since_verified)
            if fact.confidence < FORGET_THRESHOLD:
                fact.state = Stale
                // Stale knowledge can be re-exposed, re-learned

function update_beast_knowledge_from_observation(observer, beast, observed_ability):
    // Empirical evidence: observer sees beast use an ability
    species = beast.species_id
    if species not in observer.beast_knowledge:
        observer.beast_knowledge[species] = BeastKnowledge()
    
    knowledge = observer.beast_knowledge[species]
    knowledge.observed_abilities.add(observed_ability)
    knowledge.threat_level = estimate_threat_from_abilities(observed_abilities)
    knowledge.last_sighting_tick = current_tick
    
    // This knowledge will diffuse to other NPCs via information network
```

**Idea Diffusion (Opinion-Modulated):**

```pseudocode
Idea {
    id: unique_id
    origin_agent: agent_id        // who thought of this?
    opinion_delta: float[NUM_OPINION_DIMS]  // how it shifts opinions
    persuasiveness: float         // base transmission rate
    complexity: float             // requires sapience to understand
    novelty: float                // decays as idea spreads
    origin_tick: tick
}

function spread_ideas(social_graph, tick):
    for each active_interaction(A, B):
        for idea in A.active_ideas:
            if B has not encountered idea:
                // Agents are receptive to ideas aligned with existing opinions
                alignment = dot_product(idea.opinion_delta, B.opinions)
                // Agents with extreme opinions are polarized (resistant to cross-cutting ideas)
                receptivity = sigmoid(alignment * ALIGNMENT_SENSITIVITY)
                
                // Novel ideas spread faster (diminishing with oversaturation)
                idea.novelty *= NOVELTY_DECAY  // decay each time it's shared
                
                p_transmit = idea.persuasiveness
                           * receptivity
                           * idea.novelty
                           * get_trust(A, B)
                           * (B.sapience >= idea.complexity ? 1.0 : 0.3)
                
                if random() < p_transmit:
                    // B adopts the idea (opinion shift)
                    for d in 0..NUM_OPINION_DIMS:
                        if abs(idea.opinion_delta[d]) > EPSILON:
                            if B.confidence[d] wide enough:
                                B.opinions[d] += idea.opinion_delta[d] * IDEA_INFLUENCE
                    B.active_ideas.add(idea)

function generate_idea(agent) -> Idea:
    // Ambitious, ideological agents generate new ideas
    if agent.personality.ambition < INNOVATOR_THRESHOLD:
        return null
    if agent.needs.ideological < MOTIVATION_THRESHOLD:
        return null
    
    // Idea is a small perturbation of agent's current beliefs
    delta = float[NUM_OPINION_DIMS]
    
    // Pick 1–3 salient dimensions
    salient_dims = get_top_k_salient_dimensions(agent, k=randint(1, 3))
    for d in salient_dims:
        // Push the dimension further in direction agent already leans
        delta[d] = sign(agent.opinions[d]) * random(0.05, 0.2)
    
    return Idea {
        opinion_delta=delta,
        persuasiveness=agent.personality.charisma * 0.5 + random(0, 0.5),
        complexity=random(0.1, agent.sapience),
        novelty=1.0,
        origin_tick=current_tick,
        origin_agent=agent.id
    }

// INDEPENDENT SPREADERS: some agents autonomously generate ideas
function tick_independent_spreaders(all_agents, tick):
    for agent in all_agents where agent.sapience > 0.5:
        if agent.personality.ambition > INNOVATOR_THRESHOLD
           and agent.needs.ideological > MOTIVATION_THRESHOLD:
            if random() < INNOVATION_RATE * agent.sapience:
                new_idea = generate_idea(agent)
                if new_idea != null:
                    agent.active_ideas.add(new_idea)
```

---

### Layer F5: Faction Crystallization

**Faction Detection** (community detection on structure + opinions):

```pseudocode
function detect_factions(social_graph, agent_minds, tick) -> list<Faction>:
    // Step 1: Build combined similarity matrix
    // structure + opinion distance
    similarity = float[num_agents][num_agents]
    
    for each pair (A, B):
        edge = get_edge(A, B)
        social_weight = 0
        if edge != null:
            social_weight = edge.valence * edge.familiarity
        
        opinion_distance = euclidean_distance(A.opinions, B.opinions)
        opinion_similarity = 1 - (opinion_distance / sqrt(NUM_OPINION_DIMS * 4))
        
        similarity[A][B] = SOCIAL_WEIGHT * social_weight
                         + OPINION_WEIGHT * opinion_similarity
    
    // Step 2: Modularity-based community detection (Louvain or Leiden)
    // Signed edges (negative weights = repulsion between ideological enemies)
    communities = signed_community_detection(similarity)
    
    // Step 3: Filter by size
    factions = []
    for community in communities where len(community) >= MIN_FACTION_SIZE:
        members = community
        opinion_centroid = mean(agent.opinions for agent in members)
        
        faction = Faction(
            members=members,
            opinion_centroid=opinion_centroid,
            opinion_variance=variance(agent.opinions for agent in members),
            cohesion=internal_edge_density(community, social_graph),
            formation_tick=tick
        )
        factions.append(faction)
    
    // Step 4: Inheritance tracking (match to previous factions by member overlap)
    match_factions_to_previous_state(factions, PREVIOUS_FACTIONS)
    
    return factions
```

**Faction Lifecycle:**

```
FACTION_EVENTS:

Formation:
    Cluster of agents with high mutual affinity and similar opinions
    exceeds MIN_FACTION_SIZE for FORMATION_PERSISTENCE ticks.

Growth:
    New agent joins faction when:
    - Has positive edges to multiple members
    - Opinions within (centroid ± k * variance)
    - Spends significant time co-located

Split:
    If internal opinion variance exceeds SPLIT_THRESHOLD for SPLIT_PERSISTENCE ticks,
    run community detection on faction's internal subgraph.
    If subgraph fragments, split faction.

Merge:
    Two factions merge if centroids converge within MERGE_DISTANCE
    AND inter-faction edge density exceeds MERGE_AFFINITY_THRESHOLD.

Dissolution:
    Cohesion drops below MIN_COHESION for DISSOLVE_PERSISTENCE ticks,
    OR membership drops below MIN_FACTION_SIZE.
    Members become unaffiliated.

Absorption:
    Small faction absorbed by large faction if:
    - Small faction size < 2 × MIN_FACTION_SIZE
    - Large faction centroid within small faction's acceptance range
    - Many positive cross-faction relationships
```

**Emergent Faction Identity:**

```
IdentityProfile {
    // Which opinion dimensions define this faction?
    salient_dimensions: list<(dim_index, mean_value)>  // top 3–4
    
    // What do members do? (behavioral signature)
    economic_activity: float
    military_activity: float
    exploration_activity: float
    knowledge_activity: float
    ritual_frequency: float
    
    // Relations with other factions
    faction_relations: Map<faction_id, float>  // average valence
    
    // History (events and heroes)
    founding_event: optional<EventReference>
    key_ideas: list<idea_id>
    hero_agents: list<agent_id>
    
    // Computed governance profile (see Layer F6)
    governance: GovernanceProfile
}
```

---

### Layer F6: Political Structure & Decision-Making

**Emergent Governance:**

```
GovernanceProfile {
    // How decisions are actually made
    centralization: float       // 0 = consensus, 1 = single leader
    stability: float            // 1 / leadership_turnover_rate
    inclusiveness: float        // fraction of members in decision-making
    formality: float            // ratio of Command vs. Debate interactions
    legitimacy: float           // trust members have in leadership
}

function detect_role(agent) -> RoleProfile:
    ba = agent.behavior_profile
    total = sum_all(ba) + EPSILON
    
    return RoleProfile {
        authority=ba.time_in_command / total + ba.decisions_influenced * 0.01,
        economic=ba.time_in_trade / total + ba.resources_controlled * 0.01,
        military=ba.time_in_combat / total,
        intellectual=ba.time_in_teaching / total + len(ba.knowledge) * 0.001,
        diplomatic=ba.time_in_diplomacy / total,
        spiritual=ba.time_in_ritual / total,
        exploratory=ba.time_in_exploration / total,
    }
```

Examples:
- High authority + military = "warlord"
- High authority + spiritual = "priest-leader"
- High economic + diplomatic = "merchant-prince"

But names are *player-generated*, not in code.

**Collective Decision-Making:**

```pseudocode
function resolve_collective_decision(faction, decision_issue) -> float:
    governance = faction.governance_profile
    
    if governance.centralization > 0.7:
        // Autocratic: leader decides
        leader = highest_authority_member(faction)
        return leader.opinions[decision_issue]
    
    elif governance.centralization < 0.3:
        // Democratic: weighted median of engaged members
        votes = []
        for member in faction.members:
            if member.behavior_profile.decisions_influenced > 0:
                weight = member.behavior_profile.decisions_influenced
                votes.append((member.opinions[decision_issue], weight))
        return weighted_median(votes)
    
    else:
        // Oligarchic: top-k authority members decide
        elite = top_k_by(faction.members, key=authority, k=max(3, len(faction) * 0.1))
        return mean(m.opinions[decision_issue] for m in elite)
    
    // DISSENT TRACKING: members who strongly disagree reduce cohesion
    for member in faction.members:
        disagreement = abs(member.opinions[decision_issue] - outcome)
        if disagreement > member.confidence[decision_issue]:
            reduce_faction_cohesion(member, faction, disagreement)
```

**Inter-Faction Relations:**

```
FactionRelation {
    faction_a: id
    faction_b: id
    diplomatic: float           // -1 (war) to +1 (alliance)
    trade_volume: float
    treaty: optional<TreatyType>
    border_friction: float
    opinion_distance: float
}

TreatyType = enum {
    NonAggression,
    TradeAgreement,
    MutualDefense,
    Vassalage,   // asymmetric subordination
    Unification, // merging process
}

function update_inter_faction_relations(factions, tick):
    for each pair (F_a, F_b):
        rel = get_relation(F_a, F_b)
        
        // Bottom-up: aggregate member-level cross-faction relationships
        cross_edges = edges from F_a members to F_b members
        rel.diplomatic = mean(e.valence for e in cross_edges)
        
        // Ideological distance creates friction
        rel.opinion_distance = euclidean_distance(F_a.centroid, F_b.centroid)
        rel.diplomatic -= rel.opinion_distance * IDEOLOGY_FRICTION
        
        // Territorial overlap creates friction
        rel.border_friction = compute_territorial_overlap(F_a, F_b)
        rel.diplomatic -= rel.border_friction * TERRITORY_FRICTION
        
        // STRUCTURAL BALANCE at faction level
        for F_c in factions where F_c != F_a and F_c != F_b:
            rel_ac = get_relation(F_a, F_c)
            rel_bc = get_relation(F_b, F_c)
            triad = rel.diplomatic * rel_ac.diplomatic * rel_bc.diplomatic
            if triad < 0:  // unbalanced
                apply_faction_balance_pressure(rel, rel_ac, rel_bc)
```

---

### Layer F7: Beast Social Layer (Lightweight)

Non-sapient beasts (neural_speed < 0.6) use simplified social structure:

```
function tick_beast_social(beast_population):
    for species in species_list:
        if species.avg_neural_speed >= SAPIENCE_THRESHOLD:
            // Full sapient layer for this species
            continue
        
        // Lightweight: pack/herd/solitary dynamics
        gregariousness = species.avg_channel[CHEMICAL_SENSING] + species.avg_channel[VIBRATION_SENSING]
        
        if gregariousness > PACK_THRESHOLD:
            // Pack: dominance hierarchy
            packs = cluster_by_proximity_and_kinship(species.members)
            for pack in packs:
                pack.sort_by_dominance()
                alpha = pack[0]
                // Alpha influences movement, others follow
                for member in pack[1..]:
                    member.target_direction = toward(alpha)
        
        elif gregariousness > HERD_THRESHOLD:
            // Herd: loose grouping, alignment
            herds = cluster_by_proximity(species.members)
            for herd in herds:
                centroid = mean_position(herd)
                for member in herd:
                    member.target_direction = toward(centroid)
        
        // else: solitary
```

**Sapience Threshold:** If a beast species' average neural_speed evolves above 0.6, they can participate in the full sapient social layer:

```
if species.avg_neural_speed >= SAPIENCE_THRESHOLD:
    for beast in species.members:
        beast.sapience = beast.channels[NEURAL_SPEED]
        // Beast gains an AgentMind, participates in factions, etc.
        // This allows evolved intelligence to enable political complexity
```

---

## 4. Cross-System Hooks

**To Evolutionary Model:**
- `sapience_level` (derived from neural_speed channel) gates participation in full social layer
- Observed beast abilities update NPC beast_knowledge, which diffuses socially
- Hunting pressure from NPCs becomes empirical kill rates in biome cells

**To Economy/Territory (future system):**
- Factions claim territory, modify biomes (farming, deforestation)
- Territory changes affect biome cell_fitness and carrying_capacity
- Trade routes (edges between faction territories) become information highways

**To Phenotype Interpreter:**
- Beast abilities are observed by NPCs, update beast_knowledge
- NPC observation → confidence in fact increases with clarity_of_observation

---

## 5. Tradeoff Matrix

| Decision | Option A | Option B | Option C | Sim Fidelity | Implementability | Player Legibility | Choice & Why |
|---|---|---|---|---|---|---|---|
| **Opinion Representation** | Discrete ideology type | Continuous multidimensional | Hierarchical belief system | High (opt B) | Medium (B) | Low (opt A) | **Continuous (B)** — compose into unbounded political space. Discrete caps novelty. Hierarchical adds complexity. |
| **Confidence Bounds** | Fixed per agent | Adaptive per dimension | Fully dynamic (weighted) | High (opt B/C) | Medium (B) | Low (all) | **Adaptive per dimension (B)** — empirically grounded (Li et al. 2024). Agents become more/less open-minded through experience. |
| **Faction Formation** | Hand-declared | Detected via community algorithm | Explicit player choice | High (opt B) | Medium (B) | High (opt A/C) | **Detected (B)** — emerges from social dynamics. Hand-declared is scripted. Player choice removes emergence. |
| **Governance Emergence** | Define gov types (monarchy, etc.) | Measure from decision patterns | Hybrid (common types, rare variants) | High (opt B) | Medium (B) | High (opt C) | **Measured (B)** — governance is what government *does*, not a label. Produces novel structures. |
| **Beast Social Tier** | Full opinion dynamics for all | Lightweight pack/herd only | Sapience-gated transition | High (opt C) | High (opt C) | High (opt C) | **Sapience-gated (C)** — avoids absurdity of non-sapient politics. Allows evolved intelligence to unlock complexity. |
| **Player as Social Node** | Outside system (observer) | Inside system (one node) | Hybrid (mostly inside, special rules) | High (opt B) | Medium (B) | High (opt C) | **Inside system, mostly (C)** — player actions have real social consequences. But allow some special rules (player can't be betrayed by permanent-allegiance faction). |

---

## 6. Emergent Properties

**Political Polarization Without Authorial Antagonism**: Two initial random opinion clusters don't "fight" because it's authored. They drift apart through homophily (agents assimilate to their cluster) and structural balance (enemies of enemies distance further). Polarization emerges.

**Faction-Level Coevolution with Beasts**: NPCs observe beast tactics, encode in knowledge. Factions develop distinct hunting strategies. Beasts evolve defense against *discovered* strategies. Coevolution is between factions and beast species, not player and beasts.

**Ideological Drift Under External Pressure**: A faction faces a beast threat. Internal disagreement suppresses (threat unites). But if threat persists for too long, radical factions form (some members want aggression, others diplomacy). Factions split. Some seek alliance with player or other factions. Factional landscape reshapes under pressure.

**Knowledge-Opinion Feedback**: A faction learns beasts are weak to fire. They adopt "aggressive hunting" opinions. As their hunting success increases, they gain status, attract new members with similar aggressive opinions. Faction becomes more hawkish. This is pure feedback from ecology to politics to faction composition.

**Inter-Faction Equilibrium**: With weak structural balance, a 3-faction system can stabilize: Faction A and B are enemies, B and C are enemies, A and C are allies. Stable tripod structure. But if C grows too strong, A and B might ally against them (balance pressure pushes them together). Factional landscape is dynamic but stable at macro scale.

---

## 7. Open Calibration Knobs

| Parameter | Current Value | Range | Effect |
|---|---|---|---|
| `MIN_FACTION_SIZE` | 3 | 2–10 | Smaller = more factions, faster politics; larger = fewer, slower |
| `SAPIENCE_THRESHOLD` | 0.6 | 0.3–0.9 | Where beasts enter full social layer; higher = fewer sapient species |
| `CONFIDENCE_GROWTH` | 0.01 | 0.001–0.05 | Per positive interaction; higher = faster openness |
| `CONFIDENCE_SHRINK` | 0.005 | 0.001–0.02 | Per negative interaction; higher = faster entrenchment |
| `BALANCE_PRESSURE` | 0.05 | 0.01–0.2 | Triadic balance correction strength; higher = faster convergence |
| `INNOVATION_RATE` | 0.001 | 0.0001–0.01 | Base rate of idea generation; higher = faster ideological change |
| `KNOWLEDGE_DECAY_RATE` | 0.001 per tick | 0.0001–0.01 | Info fades without reinforcement |
| `SALIENCE_WEIGHTS` | Per dimension | — | How each need emphasizes each opinion dimension |

---

## 8. Notes on Implementation

**Performance:**
- Opinion updates: O(NUM_OPINION_DIMS) ≈ O(12) per interaction, fast
- Relationship decay: O(num_edges), runs once per tick on sparse graph
- Faction detection: O(N² × NUM_OPINION_DIMS), runs every K=200 ticks
- Knowledge diffusion: O(knowledge_store_size), capped per agent
- For 500 NPCs, ~20–50 interactions per tick, ~15 factions: < 5ms per tick

**Debug Tools:**
- Social graph visualizer: show agents (nodes), relationships (edges, color/thickness by valence/familiarity)
- Opinion plotter: radar chart per agent or faction centroid
- Faction browser: list factions, members, salient dimensions, governance profile
- Knowledge browser: per-agent knowledge store, trace information diffusion
- Idea tracker: log idea generation, spread, adoption

**Content Tuning:**
1. Run simulation, observe faction formation
2. If factions are too polarized, increase CONFIDENCE_GROWTH (more openness)
3. If factions are too stable, increase INNOVATION_RATE (more idea diversity)
4. If beasts are too threatened by NPCs, reduce KNOWLEDGE_DECAY_RATE (NPCs remember weaknesses too long)
5. Adjust SALIENCE_WEIGHTS to emphasize certain dimensions under stress
