# NPC Dialogue System: Simulation-First Conversation

## 1. Overview & Sim-First Stance

Dialogue is not a parallel system. It is a **high-frequency variant of social interaction**, using the same opinion dynamics as the faction social model (System 03) but applied to immediate conversation. NPCs do not have separate "dialogue trees" and "faction relations"—they have a unified AgentMind that drives both.

**Core simulation principle**: Everything an NPC says emerges from:
1. What they actually know (KnowledgeStore)
2. What they currently need (NeedsVector)
3. What they think of the player (AgentMind opinions)
4. What they're trying to accomplish right now (Intent, computed from state)

There are no named dialogue options authored in a tree. Instead, the system assembles dialogue from **semantic templates** that are gated on actual world state, not on player dialogue choices or build stats.

The **player's agency** lies in their past actions (which shaped NPC knowledge and opinions) and their present choices during conversation (pursuing or deflecting topics, sharing information, making promises). Dialogue outcomes are consequential because they affect the same opinion values that determine faction relations, trade availability, and NPC help in combat.

---

## 2. Research Basis

### Simulation-Based NPC Systems

**Disco Elysium (ZA/UM, 2019)**
Skills as active voices in your head, interjecting with personality. Dialogue is not gated on player stats (you can't "win" a conversation with high Charisma); rather, failure is interesting and reveals character. Passive checks reveal hidden NPC knowledge without the player asking. The design lesson: dialogue is a skill check, not a player choice. The system tells the player what their character would say/know, and the player experiences consequences.

- ZA/UM. *Disco Elysium: The Final Cut*. 2021. https://www.discoelysium.com/

**Fallout: New Vegas (Obsidian Entertainment, 2010)**
Faction reputation is a granular, multi-axis system (fame vs infamy, per-faction). Dialogue is gated on reputation (you can only make certain speeches if you have standing). Actions ripple through faction relations: helping one faction damages relations with enemies. The design lesson: the world reacts to your deeds, and dialogue reflects those reactions.

- Obsidian Entertainment. *Fallout: New Vegas*. 2010. https://fallout.bethesda.net/

**Baldur's Gate 3 (Larian Studios, 2023)**
Approval systems where companion reactions scale with how much they approve of the player. Approval affects persuasion difficulty, unlocks personal quests, and creates meaningful consequences (companions abandon you if approval is too low). The design lesson: approval/trust should drive behavior, not gating; the NPC should demonstrate their attitude through their actions and dialogue tone.

- Larian Studios. *Baldur's Gate 3*. 2023. https://baldursgates3.larian.com/

**Ekman's Emotion Recognition (Paul Ekman, 1970s–present)**
Research into facial expressions and emotional authenticity. Seven universal microexpressions (anger, fear, disgust, happiness, sadness, surprise, contempt) are hardwired across cultures. The design lesson for procedural dialogue: NPC emotional tone should map to a few authentic emotional primitives that the player can read from dialogue content. Avoid the uncanny valley of "too many named emotions."

- Ekman, P. & Friesen, W. V. "Unmasking the Face." 1975. Malor Books.

---

## 3. Entities & State

### Attitude Projection (Not a New Variable)

There are no separate "dialogue approval" scores. Attitude toward the player is a **computed projection** of the AgentMind:

```
AttitudeProjection {
    // All values in [-1.0, +1.0]
    
    cooperation:    float       // from opinions[11] (player_cooperation)
    trust:          float       // from social_knowledge[player].source_trust
    fear:           float       // derived from threat level × survival need
    respect:        float       // derived from competence estimate × alignment
    need:           float       // computed from how much NPC's needs align with player
    
    // Composite
    disposition:    float       // weighted combination; drives dialogue tone selector
    
    // Temporal
    recent_deeds:   list<DeedRecord>  // what NPC knows player did recently
    opinion_trajectory: float         // is attitude improving or worsening?
}

function project_attitude(npc: AgentMind, player_id: agent_id) -> AttitudeProjection:
    att = AttitudeProjection()
    
    // Direct opinion
    att.cooperation = npc.opinions[11]  // player_cooperation dimension
    
    // Trust from interaction history
    player_knowledge = npc.knowledge.get_social_knowledge(player_id)
    if player_knowledge != null:
        att.trust = player_knowledge.source_trust
        att.recent_deeds = player_knowledge.known_deeds
    else:
        att.trust = 0.0  // stranger
        att.recent_deeds = []
    
    // Fear: how much player's presence threatens NPC's survival
    if player_knowledge != null:
        threat_perceived = player_knowledge.threat_level
        att.fear = threat_perceived * npc.needs.survival
                   - npc.personality.volatility * 0.3  // volatile NPCs feel fear more
    else:
        att.fear = 0.0
    
    // Respect: did player demonstrate competence, or align with NPC's values?
    if player_knowledge != null:
        competence_observed = player_knowledge.competence_estimate
        ideological_alignment = alignment_bonus(npc, player_id)
        att.respect = competence_observed * 0.7 + ideological_alignment * 0.3
    else:
        att.respect = 0.0
    
    // Need: does NPC currently need the player?
    att.need = compute_need_alignment(npc.needs, player_knowledge)
    
    // Disposition: weighted combination
    weights = {
        cooperation: 0.35,
        trust: 0.25,
        fear: -0.15,        // fear often creates servility or hostility
        respect: 0.15,
        need: 0.10,
    }
    att.disposition = weighted_sum(
        [att.cooperation, att.trust, att.fear, att.respect, att.need],
        weights
    )
    att.disposition = clamp(att.disposition, -1.0, 1.0)
    
    // Opinion trajectory
    previous_att = npc.attitude_history.last()
    att.opinion_trajectory = att.disposition - previous_att.disposition
    
    return att
```

**Design consequence**: Changing an NPC's attitude requires changing their underlying opinions and knowledge, not spending dialogue "reputation points." If you want an NPC to like you, you need to do things they approve of (affecting `opinions[11]`), prove you're trustworthy (affecting `source_trust`), or demonstrate competence (affecting `competence_estimate`). Dialogue reflects these changes; it doesn't create them.

### Disposition Bands & Emotional Coloring

```
DISPOSITION_SCALE = [
    (-1.0, -0.7):   HOSTILE,        # attacks, threats, refuses
    (-0.7, -0.4):   ANTAGONISTIC,   # insults, obstruction, warnings
    (-0.4, -0.1):   COLD,           # terse, unhelpful, minimal engagement
    (-0.1,  0.1):   NEUTRAL,        # transactional, evaluating
    ( 0.1,  0.4):   WARM,           # polite, willing to trade, shares info
    ( 0.4,  0.7):   FRIENDLY,       # forthcoming, offers help
    ( 0.7,  1.0):   DEVOTED,        # deep loyalty, shares secrets, self-sacrifices
]

EMOTIONAL_MODIFIERS = {
    // Fear modulates the base disposition band
    high_fear: {
        HOSTILE:        SERVILE_HOSTILE,    // obeys but resentful
        ANTAGONISTIC:   NERVOUS_ANTAGONISTIC,
        NEUTRAL:        SUSPICIOUS,
        WARM:           ANXIOUS,
        FRIENDLY:       PROTECTIVE,
        DEVOTED:        SELF_SACRIFICING,
    },
    
    // Other personality dimensions affect tone
    high_empathy: prefers_compassionate_fragments,
    high_charisma: prefers_social_fragments,
    high_ambition: prefers_self_promoting_fragments,
}
```

### NPC Intents: Emergent Goals

Intents are not a fixed enum selection. They emerge from the NPC's current unmet needs and information mismatches:

```
function select_intent(npc: AgentMind, context: ConversationContext) -> NPCIntent:
    // Intents are generated from world state, not selected from a predefined menu
    
    candidates = []
    
    // URGENCY OVERRIDE
    if npc.urgent_concern != null and npc.urgent_concern.severity > 0.7:
        if context.attitude.disposition > -0.4:
            candidates.append((Intent.Warn, priority=10))
        elif context.attitude.need > 0.5:
            candidates.append((Intent.Plead, priority=10))
    
    // FIRST MEETING
    if context.relationship_age == 0:
        candidates.append((Intent.Greet, priority=8))
    
    // NEED-DRIVEN INTENTS
    // An NPC prioritizes conversation that helps meet their needs
    if npc.needs.survival > 0.7 and context.attitude.disposition > -0.1:
        candidates.append((Intent.RequestHelp, priority=npc.needs.survival * 7))
    
    if npc.needs.economic > 0.5 and context.attitude.disposition > 0.0:
        candidates.append((Intent.OfferTrade, priority=npc.needs.economic * 5))
    
    if npc.needs.ideological > 0.5 and context.attitude.respect > 0.2:
        candidates.append((Intent.Persuade, priority=npc.needs.ideological * 4))
    
    // DEED-REACTIVE INTENTS
    // NPC reacts to what they know the player has done
    for deed in context.recent_deeds:
        if deed.is_recent():
            valence = evaluate_deed_for_npc(npc, deed)
            if valence > 0.3:
                candidates.append((Intent.Praise, priority=valence * 6, data=deed))
            elif valence < -0.3:
                candidates.append((Intent.Reproach, priority=abs(valence) * 6, data=deed))
    
    // KNOWLEDGE MISMATCH INTENTS
    // NPC might want to share knowledge they have that player doesn't
    exclusive_knowledge = npc.knowledge.filter(k => player_doesnt_have(k))
    if len(exclusive_knowledge) > 0 and context.attitude.disposition > 0.1:
        candidates.append((Intent.Inform, priority=3, data=exclusive_knowledge[0]))
    
    // NPC might want to ask about things they suspect player knows
    if len(context.player_exclusive) > 0 and context.attitude.trust > 0.2:
        candidates.append((Intent.Inquire, priority=2))
    
    // TRUST-GATED INTENTS
    if context.attitude.trust > 0.6 and context.attitude.disposition > 0.5:
        secrets = get_npc_secrets(npc)
        if len(secrets) > 0:
            candidates.append((Intent.RevealSecret, priority=4, data=secrets[0]))
    
    // FACTION-DRIVEN INTENTS
    if npc.faction != null and npc.faction.should_recruit_player():
        if context.attitude.disposition > 0.3:
            candidates.append((Intent.Recruit, priority=3))
    
    // Select highest priority
    selected_intent = weighted_select(candidates, npc.personality)
    return selected_intent
```

**Design consequence**: An NPC's dialogue is driven by what they want and need right now, not by a dialogue tree branch. The Keeper can influence the topic by sharing information, but the NPC's intent remains their own priority. If an NPC needs help desperately, they'll try to recruit the player's aid even if the player steers the conversation toward trade.

### Dialogue Fragments: Semantic Templates

Dialogue is assembled from **fragments**—small, tagged, combinable pieces—rather than prewritten trees:

```
DialogueFragment {
    id:             fragment_id
    
    // What this fragment does
    intent:         NPCIntent          // which NPC intent it serves
    fragment_type:  enum { Opener, Body, Closer, Reaction, Interjection }
    
    // When it can be used
    gates: list<Gate>  // conditions that must be true
    
    // Content
    template:       string             // with {slot} placeholders
    slots:          list<SlotDef>      // how to fill slots
    
    // Tone variants
    variants: {
        HOSTILE: "...",
        ANTAGONISTIC: "...",
        // ... one variant per disposition band
        DEVOTED: "...",
    }
    
    // If the NPC says this, what opinion changes happen?
    // (consequences use same formulas as faction social model)
    social_consequences: {
        target_opinion: opinion_dimension,
        delta: float,                  // how much to shift opinion
        trigger_condition: "if_player_knows_is_false",
    }
}

Gate = union {
    AttitudeGate {
        axis: AttitudeAxis,     // cooperation, trust, fear, respect, need, disposition
        min: float,
        max: float,
    }
    KnowledgeGate {
        requires_fact: FactKey,  // NPC must know this fact
    }
    FactionGate {
        faction: faction_id,
        relation: "member" | "allied" | "neutral" | "hostile",
    }
    ConversationGate {
        requires_topic: TopicTag,     // must have already discussed this
        excludes_topic: TopicTag,     // must NOT have discussed this
    }
    PersonalityGate {
        trait: PersonalityTrait,      // high_empathy, high_ambition, etc.
        min: float,
        max: float,
    }
    WorldStateGate {
        condition: WorldCondition,    // time_of_day, season, recent_event, etc.
    }
}

SlotDef {
    name: string,           // e.g., "deed_verb", "beast_name", "location_name"
    source: SlotSource,     // where to pull the value from
    fallback: string,       // default if source is empty
}

SlotSource = enum {
    DeedDescription,        // from a DeedRecord the NPC knows about
    FactValue,              // from a KnowledgeFact being discussed
    NPCName,
    PlayerTitle,            // how NPC addresses the player (contextual)
    FactionName,
    LocationName,
    BeastSpeciesName,
    OpinionExpression,      // "I appreciate your {opinion_dimension}" → "I appreciate your courage"
    NeedExpression,         // NPC's current need as natural language
    RelationshipExpression, // "We've {relationship_age_band}" → "We've worked together for months"
    CustomComputed,         // game-specific computed value
}
```

**Fragment assembly example**:

```
Fragment {
    id: "praise_creature_kill_warm"
    intent: Praise
    gates: [
        AttitudeGate { axis: disposition, min: 0.1, max: 1.0 },
        KnowledgeGate { requires_fact: KnownDeed(type: killed_dangerous_creature) },
    ]
    variants: {
        WARM: "That {creature_name} was terrorizing {location_name}. You did us a real service.",
        FRIENDLY: "You brought down the {creature_name}! {location_name} sleeps easier because of you.",
        DEVOTED: "When I heard you faced the {creature_name} alone... I'll never forget that.",
    }
    slots: [
        { name: "creature_name", source: DeedDescription },
        { name: "location_name", source: LocationName },
    ]
    social_consequences: {
        target_opinion: player_cooperation,
        delta: +0.1,  // saying this increases NPC's player_cooperation opinion
        trigger_condition: "always",
    }
}
```

---

## 4. Update Rules

### Dialogue Assembly Pipeline

```
DIALOGUE ASSEMBLY:

Input: NPC, Player, World Context
       ↓
Stage 1: Context Evaluator
       ├─ Reads NPC state (knowledge, needs, personality, opinions)
       ├─ Reads player state (knowledge, recent deeds, faction affiliations)
       ├─ Reads world state (location, time, recent events)
       └─ Outputs: ConversationContext
       ↓
Stage 2: Intent Selector
       ├─ Evaluates NPC's current priorities
       ├─ Checks for urgent concerns
       └─ Outputs: NPCIntent + priority
       ↓
Stage 3: Content Assembler
       ├─ Gathers eligible fragments (matching intent + passing gates)
       ├─ Scores fragments by relevance to context
       ├─ Personality-weighted selection (high-empathy NPCs prefer emotional fragments)
       └─ Outputs: Selected DialogueFragment
       ↓
Stage 4: Slot Filler
       ├─ For each slot in the fragment template:
       │  ├─ Determine source (deed, fact, opinion, etc.)
       │  ├─ Pull value from NPC's knowledge store
       │  └─ Interpolate into template
       ├─ Select tone variant (based on disposition band)
       └─ Outputs: Final dialogue text
       ↓
Stage 5: Social Consequence Resolver
       ├─ Read fragment.social_consequences
       ├─ Apply opinion deltas (same system as faction social model)
       ├─ Record this interaction in NPC's memory
       └─ Outputs: Updated NPC state
       ↓
Output: Dialogue text + NPC state changes
```

### Stage 1: Context Evaluator

```
function evaluate_conversation_context(npc: AgentMind, player: Player, 
                                       world: World, time: float) -> ConversationContext:
    ctx = ConversationContext()
    
    // WHO
    ctx.npc = npc
    ctx.player = player
    ctx.attitude = project_attitude(npc, player.id)
    ctx.relationship_age = get_relationship_age(npc, player)
    
    // WHERE
    ctx.location = world.get_location(npc.current_location)
    ctx.location_owner = ctx.location.faction_control
    ctx.nearby_npcs = world.get_npcs_in_area(ctx.location, NEARBY_RANGE)
    ctx.weather = world.weather.sample(ctx.location)
    
    // WHAT'S HAPPENING
    ctx.npc_needs = npc.needs
    ctx.npc_urgent = npc.urgent_concern
    ctx.time_of_day = world.time_of_day
    ctx.season = world.season
    
    // KNOWLEDGE STATE
    ctx.player_knowledge = npc.knowledge.get_social_knowledge(player.id)
    ctx.shared_facts = npc.knowledge.intersection(player.knowledge)
    ctx.npc_exclusive = npc.knowledge.difference(player.knowledge)
    ctx.player_exclusive = player.knowledge.difference(npc.knowledge)
    
    // CONVERSATION HISTORY (this session)
    ctx.conversation_turns = []
    ctx.topics_covered = set()
    
    return ctx
```

### Stage 2: Intent Selector

(Detailed in Section 3 above.)

### Stage 3 & 4: Assembly & Filling

```
function assemble_npc_dialogue(ctx: ConversationContext, intent: NPCIntent) -> DialogueOutput:
    // Gather fragments matching intent
    candidates = fragment_library.query(intent=intent)
    eligible = []
    
    for frag in candidates:
        if all_gates_pass(frag.gates, ctx):
            eligible.append(frag)
    
    if len(eligible) == 0:
        // Fallback to generic fragments with no gates
        eligible = fragment_library.query(intent=intent, tags={generic})
    
    // Score fragments by relevance
    scored = []
    for frag in eligible:
        relevance_score = compute_fragment_relevance(frag, ctx)
        scored.append((frag, relevance_score))
    
    // Personality-weighted selection
    // High-empathy NPCs prefer compassionate fragments
    // High-charisma NPCs prefer social/dramatic fragments
    # This means the NPC's personality shapes their word choice
    selected = personality_weighted_select(scored, ctx.npc.personality)
    
    // Select disposition band variant
    band = attitude_to_disposition_band(ctx.attitude.disposition)
    if selected.variants.has_variant(band):
        template = selected.variants[band]
    else:
        template = selected.template
    
    // Fill slots
    filled_text = fill_slots(template, selected.slots, ctx)
    
    // Apply emotional modifiers based on fear level
    if ctx.attitude.fear > 0.5:
        filled_text = apply_fear_modifier(filled_text, ctx.npc.personality)
    
    return DialogueOutput {
        text: filled_text,
        fragment_id: selected.id,
        social_consequences: selected.social_consequences,
        player_dialogue_options: generate_player_responses(selected, ctx),
    }

function fill_slots(template: string, slots: list<SlotDef>, 
                    ctx: ConversationContext) -> string:
    filled = template
    
    for slot_def in slots:
        value = null
        
        match slot_def.source:
            DeedDescription:
                if ctx.player_knowledge != null and len(ctx.player_knowledge.known_deeds) > 0:
                    deed = ctx.player_knowledge.known_deeds[0]
                    value = describe_deed(deed)
            FactValue:
                if len(ctx.npc_exclusive) > 0:
                    fact = ctx.npc_exclusive[0]
                    value = fact.content_as_string()
            LocationName:
                value = ctx.location.name
            BeastSpeciesName:
                # determined from context (which creature was just discussed)
                value = ctx.current_beast_species.name if ctx.current_beast_species else "creature"
            OpinionExpression:
                dim = slot_def.opinion_dimension  # specified in template
                opinion_value = ctx.npc.opinions[dim]
                value = opinion_to_adjective(opinion_value)
                # e.g., if opinion is +0.8 → "courageous" or "bold"
            NeedExpression:
                high_need = ctx.npc_needs.highest_unmet_need()
                value = need_to_phrase(high_need)
                # e.g., "food" or "safety" or "revenge"
            RelationshipExpression:
                age_band = categorize_relationship_age(ctx.relationship_age)
                value = relationship_age_to_phrase(age_band)
                # e.g., "never met" → "just met" → "worked together" → "close allies"
        
        if value == null:
            value = slot_def.fallback
        
        filled = filled.replace("{" + slot_def.name + "}", value)
    
    return filled
```

### Stage 5: Social Consequence Resolver

```
function apply_social_consequences(npc: AgentMind, consequence: SocialConsequence):
    // This uses the SAME opinion change formulas as the faction social model
    
    opinion_dim = consequence.target_opinion
    delta = consequence.delta
    
    // Check trigger condition
    if consequence.trigger_condition == "always":
        pass
    elif consequence.trigger_condition == "if_player_knows_is_false":
        if npc.knowledge.has_fact(consequence.related_fact):
            return  // condition not met, don't apply consequence
    
    // Apply the opinion change
    // This is identical to the faction social model's opinion update rule
    npc.opinions[opinion_dim] += delta
    npc.opinions[opinion_dim] = clamp(npc.opinions[opinion_dim], -1.0, 1.0)
    
    // Record interaction in NPC's memory
    interaction = {
        type: "dialogue",
        fragment_id: consequence.fragment_id,
        player_id: player.id,
        tick: current_tick,
        opinion_change: delta,
        opinion_dimension: opinion_dim,
    }
    npc.interaction_history.append(interaction)
    
    // This interaction may diffuse through NPC network (witness rumors)
    // if other NPCs were nearby (from ctx.nearby_npcs)
    for witness in ctx.nearby_npcs:
        witness_confidence = compute_witness_confidence(witness, npc, ctx.location)
        if random() < witness_confidence:
            // Witness generates a KnowledgeFact about this interaction
            witness_fact = KnowledgeFact {
                subject: player.id,
                fact_type: "dialogue_with_npc",
                content: f"I saw {npc.name} talking with {player.name}.",
                source: DirectObservation,
                confidence: witness_confidence,
            }
            witness.knowledge.add_fact(witness_fact)
            // This fact will diffuse through the NPC network
```

### Lie Detection Mechanic

NPCs can detect if the player is lying (or believes the player is lying):

```
function evaluate_player_claim(npc: AgentMind, claim: PlayerStatement) -> float:
    // Returns probability that NPC believes the player is lying
    
    // Base suspicion from prior lies
    prior_lies = npc.knowledge.count_instances(player_has_lied_before)
    suspicion_prior = min(prior_lies * 0.2, 0.8)  // max 80% prior suspicion
    
    // Does the claim contradict known facts?
    contradicts_knowledge = npc.knowledge.contradicts(claim)
    if contradicts_knowledge:
        contradiction_weight = 0.4
    else:
        contradiction_weight = 0.0
    
    // Lie detection depends on NPC intelligence
    npc_intelligence = npc.stats.neural_speed  // from phenotype interpreter
    lie_detection_skill = npc_intelligence * 0.6  // intelligence contributes to lie detection
    
    // Player's presentation skill (Keeper personality)
    player_charisma = player.personality.charisma
    player_deception_skill = max(0.0, player_charisma - 0.5) * 0.6
    
    // Compute detection probability
    p_detect = sigmoid((lie_detection_skill - player_deception_skill) * 2.0)
    
    // Weight the factors
    p_lying = (suspicion_prior * 0.5 + 
               contradiction_weight * 0.5 +
               p_detect * 0.3)
    p_lying = clamp(p_lying, 0.0, 1.0)
    
    return p_lying

function npc_responds_to_claim(npc: AgentMind, claim: PlayerStatement) -> DialogueFragment:
    p_lying = evaluate_player_claim(npc, claim)
    
    if p_lying > 0.7:
        // NPC believes the player is lying
        return select_fragment(intent=Reproach, gates=[LyingGate], context=...)
    elif p_lying > 0.4:
        // NPC is suspicious but not certain
        return select_fragment(intent=Inquire, gates=[SuspiciousGate], context=...)
    else:
        // NPC accepts the claim
        return select_fragment(intent=Acknowledge, context=...)
```

### Witness Rumor Generation

Any NPC who observes a salient event (combat victory, gift exchange, hostile act) generates a KnowledgeFact that diffuses through the network:

```
function generate_rumor_from_observation(witness: AgentMind, event: SalientEvent, 
                                         location: Location):
    // Witness clarity: how well did they see what happened?
    distance_to_event = distance(witness.position, event.position)
    clarity = 1.0 - clamp(distance_to_event / VISIBILITY_RANGE, 0.0, 1.0)
    
    // Witness perception: can they even understand what they're seeing?
    perception = witness.stats.light_sensing  // from phenotype interpreter
    
    // Confidence in the rumor
    confidence = clarity * perception
    confidence = clamp(confidence, 0.2, 0.9)  // don't go to extremes
    
    // Generate the fact
    rumor = KnowledgeFact {
        subject: event.actor,  // who did the deed
        fact_type: event_type_to_deed_type(event),
        content: describe_event_from_perspective(event, witness),
        source: DirectObservation,
        confidence: confidence,
        tick: current_tick,
    }
    
    // Add to witness's knowledge
    witness.knowledge.add_fact(rumor)
    
    // Rumor will diffuse to other NPCs (from faction social model's SEIR network)
```

---

## 5. Cross-System Hooks

**To System 01 (Evolutionary Model)**:
- NPC neural_speed affects lie detection skill
- NPC personality affects dialogue fragment selection

**To System 02 (Traits & Channels)**:
- Keeper charisma affects deception resistance
- Keeper personality shapes how NPCs perceive them (high empathy → more likely to be trusted)

**To System 03 (Faction Social Model)**:
- Dialogue consequence opinion deltas use the same formulas as faction relations
- Social interaction (dialogue) is a high-frequency variant of faction dynamics
- Witnessed dialogue generates rumors that diffuse through NPC network

**To System 04 (Economy)**:
- NPC needs directly drive dialogue intents
- Trade requests emerge from economic needs
- Dialogue reveals NPC resource availability

**To System 06 (Combat)**:
- NPC crew trust affects rally success probability
- Combat outcomes (victory, death of allies) are high-severity deeds that generate rumors
- Crew morale is affected by witnessed deeds (Keeper behavior in combat)

**To System 07 (Exploration)**:
- NPC knowledge about locations feeds into route danger ratings
- Traded information affects player knowledge freshness
- Rumors about creature presence affect player encounter expectations

---

## 6. Tradeoff Matrix

| Tradeoff | Complexity | Player Agency | Simulation Purity | Adoption |
|----------|-----------|---------------|-------------------|----------|
| **No separate dialogue approval system** | -1 | +2 (opinions matter mechanically) | +3 | High |
| **Intents from needs, not menu** | +1 | +1 (NPC has own agenda) | +3 | Medium |
| **Fragments with semantic gates** | +2 | +1 (gates are transparent) | +2 | Low |
| **Slot-filling from knowledge** | +1 | +1 (content feels grounded) | +3 | High |
| **Lie detection probabilistic** | +0 | +1 (deception is risky) | +3 | High |
| **Witnessed rumors diffuse** | +1 | +2 (actions have consequences) | +3 | Medium |
| **Disposition bands + emotional coloring** | +0 | +0 (flavor only) | +2 | High |

---

## 7. Emergent Properties

- **NPCs react to deeds, not words**: The player's past actions (which made it into the rumor mill) are what NPCs react to. Talking well doesn't override deeds; it only works if your actions have already established credibility.
- **Information is power**: An NPC who knows something the player doesn't has leverage. Sharing (or withholding) knowledge shifts opinions and creates future dialogue hooks.
- **Personalities shape discourse style**: A high-charisma NPC will prefer social, dramatic dialogue. A high-empathy NPC will prefer compassionate framing. The same underlying conversation feels different based on NPC personality.
- **Dialogue cascades through NPC network**: A conversation witnessed by other NPCs spreads as rumor, affecting how those NPCs perceive the player before they meet.
- **Trust is hard to build, easy to lose**: An NPC's `source_trust` dimension decays if the player lies or breaks promises. It accumulates slowly if the player is reliable. This creates long-term relationship arcs.

---

## 8. Open Calibration Knobs

```yaml
Attitude Projection:
  cooperation_weight: 0.35              # how much direct opinion influences disposition
  trust_weight: 0.25
  fear_weight: -0.15                    # negative (fear creates servility/hostility)
  respect_weight: 0.15
  need_weight: 0.10
  
Intent Selection:
  urgency_override_threshold: 0.7       # how urgent must concern be to dominate
  recent_deed_reaction_window: 30       # ticks (how recent is "recent"?)
  
Fragment Assembly:
  personality_flavor_strength: 1.0      # how much personality affects fragment selection
  generic_fragment_fallback_enabled: true
  
Lie Detection:
  prior_lie_weight: 0.5
  contradiction_weight: 0.5
  intelligence_scaling: 0.6             # how much NPC intelligence affects detection
  charisma_scaling: 0.6                 # how much player charisma affects deception
  
Rumor Diffusion:
  witness_clarity_visibility_range: 50  # world units
  witness_confidence_floor: 0.2
  witness_confidence_ceiling: 0.9
  observation_distance_affects_confidence: true
```

---

## Appendix: Anti-Fudges Applied

1. **No separate dialogue approval variable**: Attitude is computed from actual AgentMind state (opinions, knowledge, needs). Changing dialogue outcomes requires changing underlying world state, not spending points.

2. **Intents are emergent, not enumerated menu**: Rather than "select from 6 dialogue options," the NPC's intent emerges from their current priorities (needs, urgent concerns, recent deeds they know about). The player doesn't choose what the NPC wants to talk about; they react to it.

3. **Lies are detectable but not perfectly**: Lie detection is probabilistic, scaled by NPC intelligence and player charisma. A sufficiently intelligent NPC with prior suspicion can detect deception, but smart players can sometimes get away with it. This creates risk without hard gatekeeping.

