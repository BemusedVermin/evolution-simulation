# Formation Combat System: Simulation-First Design

## 1. Overview & Sim-First Stance

Combat is a **direct projection of underlying simulation state**, not a parallel gameplay system. The formation, crew capabilities, creature behaviors, and environmental interactions all emerge from continuous simulation rules. The player does not *play combat* in isolation; they observe and command an active simulation of their crew engaging threats.

**Core simulation principle**: Every number the player sees — damage, hit chance, formation safety, resource costs — is computed fresh from first principles each turn. There are no lookup tables, no "balance passes," no designer-authored numbers for specific matchups. Combat readouts are the inevitable output of:

1. Creature phenotypes (channel profiles → stat blocks → behavior trees)
2. Equipment material properties (creature materials → crew abilities)
3. Formation geometry (continuous slot properties modulated by terrain)
4. Concrete combat resolution (full damage formula, mobility checks, injury mechanics)

The **Keeper commands but doesn't fight**. The player's resource is observation bandwidth and decision-making capacity: each turn, the Keeper observes the formation state and the creatures' apparent capabilities, then issues orders constrained by a **leadership budget** derived from the Keeper's personality traits and current stress level.

---

## 2. Research Basis

### Simulation-First Design References

**Caves of Qud (Sproggiwood, 2015–present)**
Procedurally generated roguelike where nearly all game systems feed into one another through explicit simulation rules. Creature behavior emerges from their evolved traits, not from scripted AI. Items have properties that create emergent interactions. The key insight: a system that reuses simulation rules across all contexts (combat, NPC behavior, environmental interaction) becomes a content generation engine where novelty compounds. Our project inherits this: the creature channel interpreter, the material property system, and the behavior tree compiler are singular rule engines that drive both NPC behavior in exploration and creature tactics in combat.

- Sproggit. "Caves of Qud." 2015–present. https://www.cavesofqud.com/

**Dwarf Fortress (Tarn Adams, 2006–present)**
The ultimate simulation-first game: dwarves have labor preferences that emerge from personality traits, they make decisions (even tactical ones) based on visible information and their temperament, and environmental disasters cascade through the simulation without special scripting. Physics, fluid dynamics, and psychology are all real systems affecting play. The design lesson: when you implement one rule system for everything, complexity emerges naturally without explicitly programming it.

- Adams, T. & Adams, Z. "Dwarf Fortress." 2006–present. http://www.bay12games.com/dwarves/

**Ultima Underworld (Blue Sky Productions, 1992)**
Early 3D dungeon crawl with creature AI driven by actual pathfinding and visibility, not scripted routines. Monsters had facing, vision cones, and spatial awareness. The key innovation for our project: creatures have *spatial cognition* — they react to where things are, not to abstract slots. Our formation model (continuous engagement/exposure values) inherits this principle.

- Blue Sky Productions. *Ultima Underworld: The Stygian Abyss*. 1992.

**Into the Breach (Subset Games, 2018)**
Displacement-centric tactics where position matters more than damage. Telegraph system ensures perfect information. Players solve spatial puzzles knowing exactly what will happen. The design lesson adopted: make formation disruption (pushing, pulling, scattering) a primary threat track parallel to damage. A formation scattered is as dangerous as a formation damaged.

- Subset Games. *Into the Breach*. 2018. https://subsetgames.com/itb/

**Darkest Dungeon (Red Hook Studios, 2016)**
Positional combat where abilities have rank requirements. Stress system that creates decision weight. The lessons: (a) position enables and disables abilities elegantly, (b) non-combat resources (stress) should constrain combat effectiveness, (c) casualties create narrative weight.

- Red Hook Studios. *Darkest Dungeon*. 2016. https://www.darkestdungeon.com/

**Bartle's Player Research**
Richard Bartle's player archetypes (Achievers, Explorers, Socializers, Killers) and subsequent research on what drives engagement: autonomy (meaningful choices), mastery (seeing the system respond to skill), and purpose (stakes that matter). Combat design that exposes system state (all formulas transparent), creates asymmetric interesting decisions (formation slots with trade-offs), and makes losses consequential (injuries, casualties, morale damage) addresses all three drives.

- Bartle, R. "Hearts, Clubs, Diamonds, Spades: Players Who Suit MUDs." *The Journal of MUD Research*. 1996.

### Practical Combat Mechanics References

**Battle Brothers (Overhype Studios, 2017)**
Squad-based hex tactics with permadeath, equipment-as-identity, and morale as a resource. Zone-of-control and flanking are spatial mechanics, not abstractions. Injuries have lasting consequences. The design lesson: crew members are investment; they matter individually because losses are permanent.

- Overhype Studios. *Battle Brothers*. 2017. https://www.battlebrothers.net/

**Valkyria Chronicles (Sega, 2008)**
Command Point system where a commander has limited activation capacity each turn, forcing prioritization. The lesson: when the player can't do everything, every decision matters.

---

## 3. Entities & State

### The Keeper's Leadership Budget

The Keeper does not have generic "Command Points." Instead, the Keeper has a **Leadership Presence** derived from their personality and current state:

```
KeeperState {
    // Personality channels (from character creation; defined in System 01)
    charisma:               float   // persuasiveness, force of will
    neural_speed:           float   // decision-making quickness, reaction time
    empathy:                float   // attunement to crew morale/needs
    
    // Stress Accumulation (this session's combat)
    current_stress:         float   // [0.0, 1.0]
    stress_sources: {
        crew_injuries_witnessed:    float
        crew_casualties:            float
        formation_disruptions:      float
        creature_unknown_abilities: float
    }
    
    // Leadership Capacity (recomputed each round)
    base_observation_bandwidth:     int     // = ceil(charisma * 8 + neural_speed * 4)
    stress_reduction:               float   // = 1.0 - (current_stress * empathy * 0.3)
    active_orders_per_turn:         int     // = floor(base_observation_bandwidth * stress_reduction)
    
    // Keeper position (always in formation)
    formation_slot:         slot_id          // which slot contains the Keeper
    health:                 float            // Keeper is a real entity; gets damaged
    morale:                 float            // separate from crew morale; Keeper can panic
}

function compute_leadership_capacity(keeper: KeeperState) -> int:
    base_bandwidth = ceil(keeper.charisma * 8.0 + keeper.neural_speed * 4.0)
    
    // Stress multiplier: high stress reduces effective capacity
    stress_mult = 1.0 - (keeper.current_stress * keeper.empathy * 0.3)
    stress_mult = clamp(stress_mult, 0.2, 1.0)  // never drops below 20% even at max stress
    
    effective_orders = floor(base_bandwidth * stress_mult)
    
    return max(effective_orders, 1)  // always at least one order per turn
```

**Design consequence**: A Keeper with high charisma (8.0) and high neural_speed (8.0) starts with ~11-12 orders per turn. Under maximum stress with low empathy, this could drop to 2-3 orders. This creates a resource tension without arbitrary point pools — the Keeper's effectiveness is a real projection of their personality and psychological state.

**Stress accumulation mechanics**:

```
function accumulate_stress(keeper: KeeperState, event: CombatEvent):
    match event:
        CrewMemberInjured(severity):
            keeper.stress_sources.crew_injuries_witnessed += severity * (1.0 - keeper.empathy * 0.4)
        CrewMemberKilled(crew_member):
            keeper.stress_sources.crew_casualties += 0.15 * (1.0 - keeper.empathy * 0.5)
        FormationDisrupted(slots_displaced):
            keeper.stress_sources.formation_disruptions += (slots_displaced / crew_count) * 0.1
        CreatureUnknownAbilityTriggered():
            keeper.stress_sources.creature_unknown_abilities += 0.05
    
    keeper.current_stress = clamp(
        (keeper.stress_sources.crew_injuries_witnessed +
         keeper.stress_sources.crew_casualties +
         keeper.stress_sources.formation_disruptions +
         keeper.stress_sources.creature_unknown_abilities) / 4.0,
        0.0, 1.0
    )
    
    // Stress recovery: time at safe settlements (doc 04 economy) or Rally attempts
    // (Rally is now a continuous ability with success probability, see below)
```

### Formation Model: Continuous Slot Properties

(Largely unchanged from source, but with explicit mechanical grounding.)

A formation is a directed graph of **slots** with continuous properties that express positional meaning:

```
FormationSlot {
    id:             slot_id
    
    // Continuous positional properties [0.0, 1.0]
    // These properties modify ability validity and damage computation
    engagement:     float   // proximity to melee threats
                            // 0.0 = rear/protected, 1.0 = melee contact
    exposure:       float   // targetability by enemies
                            // 0.0 = fully shielded, 1.0 = fully exposed
    support_reach:  float   // ability to assist other slots
                            // 0.0 = isolated, 1.0 = can reach all positions
    lateral_spread: float   // flanking coordination capacity
                            // 0.0 = inline, 1.0 = wide flank
    
    // Graph relationships
    shields:        list<slot_id>   // this slot absorbs attacks aimed at these
    shielded_by:    list<slot_id>   // this slot is protected by these
    adjacent_to:    list<slot_id>   // proximity for flanking and AoE splash
    
    // Instance state
    crew_member:    crew_id or null  // who occupies this slot
    disruption_level: float          // [0.0, 1.0], how far from template (see below)
}
```

**Disruption recovery mechanism** (anti-fudge #1):

Formation disruption is **not** automatic. Disrupted slots drift back toward their template baseline at a rate proportional to crew mobility and formation cohesion:

```
function resolve_formation_recovery(formation: Formation, crew_count: int):
    // Each turn, disrupted slots move back toward equilibrium
    
    // Cohesion depends on crew health and morale
    avg_crew_health = mean(c.health for c in crew)
    avg_crew_morale = mean(c.morale for c in crew)
    cohesion_factor = (avg_crew_health + avg_crew_morale) / 2.0
    
    for slot in formation.slots where slot.disruption_level > 0.0:
        crew = slot.crew_member
        if crew == null:
            // Unmanned slot recovers faster
            recovery_rate = 0.15
        else:
            // Recovery rate = crew's mobility * cohesion * formation template recovery rate
            crew_mobility = crew.stats.mobility
            template_recovery = formation.template.recovery_rate  // authored constant, ~0.05
            recovery_rate = crew_mobility * cohesion_factor * template_recovery
        
        slot.disruption_level -= recovery_rate  // per turn
        slot.disruption_level = max(slot.disruption_level, 0.0)
    
    // Regenerate slot properties from template toward their baseline
    for slot in formation.slots:
        template_slot = formation.template.slots[slot.id]
        disruption_ratio = slot.disruption_level
        
        // Disrupted property values are a blend between current and baseline
        slot.engagement = lerp(
            template_slot.engagement,
            slot.engagement,
            disruption_ratio
        )
        slot.exposure = lerp(
            template_slot.exposure,
            slot.exposure,
            disruption_ratio
        )
        // ... same for support_reach, lateral_spread
```

**Design consequence**: If a push-ability shoves a front-line crew member backward, that slot's engagement drops and exposure rises. Over the next 3-4 turns (depending on crew mobility), the slot naturally drifts back to vanguard positioning. But this recovery costs crew labor; if the crew is heavily disrupted across multiple slots, they're spending their mobility trying to regroup instead of attacking. This is the positional equivalent of damage over time.

### Equipment & Material Properties

Equipment does not have named ability types. Instead:

```
EquipmentPiece {
    id:                 equipment_id
    
    // Material composition
    primary_material:   MaterialSignature    // main component
    secondary_material: MaterialSignature or null
    material_blend:     float                // [0.0, 1.0] = blend ratio
    
    // Form factor (the shape determines how properties are interpreted)
    form: {
        slot:           enum { Weapon, Armor, Tool, Accessory }
        reach:          float               // 0.0 = melee contact, 1.0 = full ranged
        coverage:       float               // 0.0 = accessory, 1.0 = full body
        complexity:     float               // affects ability precision/magnitude
        mass:           float               // affects stamina cost and speed
    }
    
    // Condition
    durability:         float               // [0.0, max_durability]
    max_durability:     float
}

MaterialSignature {
    // 17 properties (not 18, since reproductive_rate has no material analogue)
    // Derived from the creature's resolved channel profile
    
    // Impact & force
    impact:         float   // from kinetic_force channel
    hardness:       float   // from structural_rigidity channel
    flexibility:    float   // from elastic_deformation channel
    density:        float   // from mass_density channel
    grip:           float   // from surface_friction channel
    
    // Chemical
    toxicity:       float   // from chemical_output channel
    purity:         float   // from chemical_resistance channel
    sensitivity:    float   // from chemical_sensing channel
    
    // Thermal & Light
    conductivity:   float   // from thermal_output channel
    insulation:     float   // from thermal_resistance channel
    luminance:      float   // from light_emission channel
    absorbance:     float   // from light_absorption channel
    
    // Vibrational
    resonance:      float   // from vibration_output channel
    attunement:     float   // from vibration_sensing channel
    
    // Biological
    vitality:       float   // from regeneration_rate channel
    volatility:     float   // from metabolic_rate channel
    reactivity:     float   // from neural_speed channel
}
```

**Loot as creature anatomy** (anti-fudge #2):

When a creature dies, it does not drop a loot table. Instead, it decomposes into **MaterialStacks** corresponding to its body regions:

```
CreatureDeathEvent {
    creature:       MonsterInstance
    location:       position
}

function resolve_creature_death(creature: MonsterInstance, location: position):
    // The creature's anatomy determines what materials it yields
    // This is deterministic from its channel profile
    
    materials_yielded = []
    
    // Map body regions → material sources
    // The harvest mapping is defined in System 05 (Crafting)
    for region in creature.body_regions:
        // Each region has a harvesting material type
        harvest_type = region.harvest_material_type  // e.g., "hide", "bone", "organ"
        
        // The material signature is derived from the creature's channels,
        // processed through the harvest mapping
        material_sig = harvest_material_signature(
            creature.stats,
            creature.channel_profile,
            harvest_type
        )
        
        // Yield quality depends on harvest tool and Keeper's harvesting skill
        base_yield = region.base_material_amount
        yield_fraction = compute_harvest_yield(keeper, harvest_tool)
        material_freshness = 1.0 - time_since_death * FRESHNESS_DECAY_RATE
        
        material_stack = MaterialStack {
            material_signature: material_sig,
            quantity: base_yield * yield_fraction,
            freshness: material_freshness,  // affects crafting quality
            body_region: region.name,
        }
        materials_yielded.append(material_stack)
    
    // Create loot pickup event
    generate_loot_event(location, materials_yielded)
    
    // Observation: player identifies this creature species if unknown
    player.observe_species(creature.species_id, creature.stats, creature.channel_profile)
```

**Design consequence**: Loot is not random. A creature with high `toxicity` always yields toxic materials. A creature with high `density` always yields dense materials. The player who wants toxic weapons must either hunt creatures with high toxicity, or trade for their materials. This closes the hunt → craft → fight loop: evolution drives material properties, materials drive equipment capabilities, equipment drives combat tactics.

### Injury System: Persistent Damage

Injuries are not stat debuffs that heal on their own. They are damage_location + damage_severity + persistent_effect tuples:

```
Injury {
    body_location:      enum { Head, Torso, LeftArm, RightArm, LeftLeg, RightLeg }
    damage_severity:    float               // [0.0, 1.0]
    injury_type:        enum { Laceration, Fracture, Burn, Poison, Infection, Hemorrhage }
    
    // Effect on stats
    stat_penalties: {
        affected_stat:  enum { attack_power, evasion, armor, mobility, stamina }
        penalty_magnitude: float
    }
    
    // Healing requirements
    medical_materials_required: list<(material_type, quantity)>
    rest_ticks_required:        int
    
    // Progression
    current_ticks_treated:      int         // how much healing has been applied
    infection_risk:             float       // deteriorates if untreated
    
    // Outcome
    permanent_consequence:      bool        // crew can become permanently maimed
    consequence_description:    string
}
```

**Example**: A crew member takes a fracture to the left leg from being pushed by a creature. They suffer:
- -0.3 to mobility (can't reposition as easily)
- -0.2 to evasion (limping reduces dodge)
- Requires 10 ticks of rest at a settlement + 2 units of "regenerative material" (e.g., animal blood with high vitality)

If untreated for 5+ ticks, infection risk rises and the injury becomes permanent (permanent loss of 10% mobility even after healing).

### Stamina: Explicit Per-Turn Resource

Stamina is derived from the creature/crew's mass and metabolic properties:

```
function derive_max_stamina(stats: StatBlock, form: EquipmentForm) -> float:
    // max_stamina emerges from biological parameters, not designer tuning
    
    base_stamina = stats.mass_density * 100.0  // heavier creatures have more stamina
    
    // Metabolic rate affects endurance
    metabolic_bonus = stats.metabolic_rate * 20.0
    
    // Equipment load reduces stamina pool
    armor_load_penalty = form.mass * 5.0
    
    max_stamina = base_stamina + metabolic_bonus - armor_load_penalty
    return clamp(max_stamina, 10.0, 200.0)
```

Each turn:

```
function recover_stamina(crew: CrewMember, turn_action: Action):
    match turn_action:
        Hold():
            recovery = crew.stats.metabolic_rate * 25.0
        Defend():
            recovery = crew.stats.metabolic_rate * 15.0
        Attack(ability):
            recovery = crew.stats.metabolic_rate * 5.0  // attacking prevents stamina recovery
    
    crew.stamina = min(crew.stamina + recovery, crew.max_stamina)
```

### Combat Actions: Synthesized from Primitives

Combat actions are NOT read from an "Ability" object. Instead, they are **synthesized at encounter initialization** from the creature's primitive-effect set returned by the Phenotype Interpreter (System 11).

```
CombatAction {
    primitive_id: string                // e.g., "apply_bite_force", "inject_toxin"
    stamina_cost: float                 // from primitive.cost (NOT hardcoded)
    cooldown: float                     // from primitive.cooldown_hint (NOT hardcoded)
    damage_base: float                  // computed from primitive.parameters
    status_effect: StatusEffect or null // inferred from primitive category
    knockback_force: float or null      // for force_application primitives
    source_primitive: PrimitiveEffect   // reference to originating primitive
}
```

**Synthesis at encounter initialization:**

```
function synthesize_combat_actions(
    creature_primitives: Set<PrimitiveEffect>,
    creature_stats: StatBlock,
    primitive_registry: PrimitiveRegistry
) -> list<CombatAction>:
    
    actions = []
    
    // For each force_application primitive, create a combat action
    for primitive in creature_primitives where primitive.category == force_application:
        
        manifest = primitive_registry.manifests[primitive.primitive_id]
        
        // Damage/effect is derived from primitive parameters and stats
        damage_base = compute_primitive_damage(
            primitive.parameters,
            primitive.primitive_id,
            creature_stats
        )
        
        // Cost and cooldown come from primitive, NOT hardcoded per ability
        action = CombatAction {
            primitive_id: primitive.primitive_id,
            stamina_cost: primitive.cost,
            cooldown: primitive.cooldown_hint,
            damage_base: damage_base,
            status_effect: infer_status_from_primitive(primitive, manifest),
            knockback_force: infer_knockback_force(primitive),
            source_primitive: primitive
        }
        
        actions.append(action)
    
    return actions
```

**Cost and cooldown are from primitive manifests only:**

The primitive manifest's `cost_function` and `recovery_time` fields determine action costs:

```
function compute_primitive_damage(
    parameters: dict,
    primitive_id: string,
    stats: StatBlock
) -> float:
    // Example: apply_bite_force primitive
    match primitive_id:
        "apply_bite_force":
            // Damage scales with force parameter and attacker's kinetic_force stat
            base = parameters.get("force", 1.0)
            return base * stats.kinetic_force * 2.0
        
        "inject_toxin":
            // Damage from toxin volume and chemical_output stat
            volume = parameters.get("volume", 0.5)
            return volume * stats.chemical_output * 1.5
        
        // ... other primitives
        
        default:
            return parameters.get("magnitude", 1.0) * 2.0
```

**Design consequence**: Combat actions inherit biological grounding from primitives. Stamina costs and cooldowns are not balance knobs; they emerge from the creature's evolved channels and primitive parameter expressions. A creature with high chemical_output will have low-cost, low-cooldown toxin primitives because the manifest's cost_function scales with the potency parameter.

### Crew Morale & Panic

Crew morale accumulates from multiple sources and affects combat effectiveness:

```
CrewMorale {
    current_morale:         float       // [0.0, 1.0]
    
    morale_sources: {
        recent_victories:       float   // last combat outcome
        injuries_witnessed:     float   // negative
        casualties:             float   // negative
        formation_integrity:    float   // how disrupted the formation is
        keeper_stress_visible:  float   // crew can sense Keeper's stress
        leadership_capacity:    float   // positive if Keeper has spare orders
    }
}

function update_crew_morale(crew: CrewMember, keeper: KeeperState, round_events: list<Event>):
    morale = crew.morale
    
    for event in round_events:
        match event:
            CrewMemberInjured(target, severity):
                if target != crew:
                    morale -= severity * 0.05  // witnessing injury
            CrewMemberKilled(target):
                if target != crew:
                    morale -= 0.15  // witnessing death
            CrewMemberHealed(target):
                morale += 0.05  // morale boost from survival
            DefeatInflicted():
                morale -= 0.2   // losing is demoralizing
            VictoryInflicted():
                morale += 0.1   // winning is encouraging
    
    // Keeper's visible stress affects crew morale
    keeper_stress_signal = keeper.current_stress * (1.0 - keeper.charisma * 0.2)
    morale -= keeper_stress_signal * 0.1
    
    // Rally ability (continuous)
    if keeper_issued_rally_order:
        rally_success = compute_rally_success(keeper, crew)
        if rally_success:
            morale += 0.2  // temporary boost
        else:
            morale -= 0.05  // failed rally is demoralizing
    
    crew.morale = clamp(morale, 0.0, 1.0)
    
    // Panic threshold
    if crew.morale < 0.2:
        crew.status = Panicked
        // Panicked crew has reduced accuracy and may flee
```

**Rally mechanic** (anti-fudge #3):

Rally is not a "once per combat" convenience. It is a **continuous ability** with a computed success probability:

```
RallyAttempt {
    type:               "Rally"
    issuer:             keeper
    target:             crew_member or "all_crew"
    stamina_cost:       0  // special; doesn't cost Keeper stamina
    action_cost:        1  // costs 1 observation bandwidth
    cooldown:           0  // can attempt every turn
}

function compute_rally_success(keeper: KeeperState, target_crew: CrewMember) -> bool:
    // Success probability is derived from Keeper charisma and crew's opinion of Keeper
    
    base_success = keeper.charisma * 0.8
    
    // Crew's opinion of Keeper affects susceptibility to rally
    // Opinion is derived from faction social model (System 03)
    crew_trust_in_keeper = get_crew_trust(target_crew, keeper)
    crew_sees_keeper_as_leader = get_crew_leadership_perception(target_crew, keeper)
    
    opinion_factor = (crew_trust_in_keeper + crew_sees_keeper_as_leader) / 2.0
    
    success_probability = base_success * (0.5 + opinion_factor * 0.5)
    success_probability = clamp(success_probability, 0.2, 0.9)
    
    return random() < success_probability
```

**Design consequence**: A high-charisma Keeper can rally crew more reliably. A Keeper with poor relations with crew will have lower rally success. This ties combat effectiveness to the non-combat social simulation, ensuring that the Keeper's relationship with crew matters mechanically.

---

## 4. Update Rules

### Combat Round Flow

```
CombatRound {
    // Phase 1: Stress & Leadership Recalculation
    // Recompute Keeper's current stress and active orders
    compute_leadership_capacity(keeper)
    
    // Phase 2: Keeper Command Phase
    // Keeper issues orders to crew
    player_orders = get_player_input()  // see Phase 3
    
    // Validate orders against current formation state
    for order in player_orders:
        if not is_order_valid(order, current_formation):
            order = fallback_to_hold(order.crew_member)
    
    // Phase 3: Initiative & Turn Order
    turn_order = compute_initiative(all_combatants)
    
    // Phase 4: Execution
    for combatant in turn_order:
        if combatant is crew_member:
            crew_turn = player_orders[combatant] or default_crew_behavior(combatant)
            execute_crew_turn(combatant, crew_turn)
        elif combatant is creature:
            execute_creature_turn(combatant)  // uses interpreter behavior tree
    
    // Phase 5: End of Round
    resolve_ongoing_effects()       // DoT, passive abilities, environmental hazards
    resolve_formation_recovery()    // disrupted slots drift back
    resolve_stamina_recovery()      // all combatants recover some stamina
    check_panic_thresholds()        // crew with low morale may panic
    check_victory_defeat()          // win/lose conditions
    accumulate_keeper_stress()      // from round events
}
```

### Creature Turn Execution

```
function execute_creature_turn(creature: CreatureInstance, order: Order):
    if creature.status in [Downed, Panicked, Restrained]:
        apply_incapacity_effect(creature)
        return
    
    match order:
        Attack(target, action: CombatAction):
            // Cost stamina (from primitive, not hardcoded)
            if creature.stamina < action.stamina_cost:
                order = fallback_to_basic_attack(creature, target)
                return execute_creature_turn(creature, order)
            
            creature.stamina -= action.stamina_cost
            creature.action_cooldowns[action.primitive_id] = action.cooldown
            
            // Compute hit chance
            hit_chance = compute_hit_chance(creature, target, action)
            
            if random() < hit_chance:
                // Damage is derived from action's primitive parameters
                damage = action.damage_base + compute_stat_scaling(
                    creature.stats,
                    action.source_primitive.source_channels
                )
                
                apply_damage(target, damage)
                
                // Status effects (from primitive category)
                if action.status_effect:
                    apply_status(target, action.status_effect)
                
                // Knockback (for force_application primitives)
                if action.knockback_force > 0:
                    apply_knockback(target, action.knockback_force)
        
        Reposition(target_slot):
            // Attempt to move to different slot
            success = attempt_reposition(creature, target_slot)
            if success:
                move_creature_to_slot(creature, target_slot)
                creature.stamina -= 10
            else:
                creature.status = ZoneOfControlRestricted
        
        Hold():
            creature.stamina = min(
                creature.stamina + creature.stats.metabolic_rate * 25,
                creature.max_stamina
            )
            reduce_cooldowns(creature, 1)
        
        Defend():
            apply_defend_stance(creature)
```

**Key change**: Combat reads `CombatAction.primitive_id`, never ability names. All mechanics (damage, cost, cooldown, effects) are derived from the primitive's parameters and manifest, maintaining Mechanics-Label Separation.

### Damage Formula: From Primitives

```
function compute_damage_from_action(
    action: CombatAction,
    attacker_stats: StatBlock,
    target: Entity,
    attacker_position: Vec3
) -> float:
    
    // Step 1: Base damage from primitive parameters
    base_damage = action.damage_base
    
    // Step 2: Channel contribution
    // Damage scales with the source channels that triggered the primitive
    for channel_id in action.source_primitive.source_channels:
        channel_value = attacker_stats[channel_id]  // resolved stat
        // Channel's damage scaling comes from System 01
        channel_coefficient = get_channel_damage_coefficient(channel_id)
        base_damage += channel_value * channel_coefficient
    
    // Step 3: Positional modifier (creatures don't use equipment)
    // Damage varies based on creature position relative to target
    distance = magnitude(attacker_position - target.position)
    optimal_range = infer_optimal_range(action.source_primitive)
    range_delta = abs(distance - optimal_range)
    positional_modifier = max(1.0 - (range_delta * 0.2), 0.5)
    
    // Step 4: Area bonus (for creatures with allies nearby)
    area_bonus = 1.0
    if has_nearby_allies(attacker_position, target.position):
        area_bonus = 1.15
    
    // Step 5: Target defense
    target_armor = compute_target_armor(target)
    damage_reduction = target_armor  // [0.0, 1.0]
    
    // Step 6: Final damage
    total_damage = base_damage * positional_modifier * area_bonus
    damage_after_defense = total_damage * (1.0 - damage_reduction)
    
    return max(damage_after_defense, 1.0)
```

**Design consequence**: Damage emerges from primitive parameters and the channels that triggered them. No hardcoded ability values.

### Mobility & Zone-of-Control Checks

```
function attempt_reposition(crew: CrewMember, target_slot: FormationSlot) -> bool:
    // Moving between formation slots faces resistance from nearby enemies
    
    crew_mobility = crew.stats.mobility
    
    // Check if any adjacent enemies create zone-of-control
    nearby_enemies = find_adjacent_threats(crew.current_slot)
    
    if len(nearby_enemies) == 0:
        return true  // no blockers, reposition succeeds
    
    // Compute zone-of-control strength of all nearby enemies
    total_zoc_strength = sum(
        enemy.stats.zone_of_control_strength
        for enemy in nearby_enemies
    )
    
    // Success probability is sigmoid of (mobility - ZoC strength)
    mobility_advantage = crew_mobility - total_zoc_strength * 0.5
    
    success_probability = sigmoid(mobility_advantage * 2.0)
    success_probability = clamp(success_probability, 0.0, 1.0)
    
    return random() < success_probability
```

---

## 5. Ability Labels Are UI-Only: Mechanics-Label Separation

**Critical Invariant (Mechanics-Label Separation, Invariant 3.9):**

Combat mechanics **never read ability labels**. The following do NOT happen in combat code:

- No ability name lookup (e.g., "is this ability called 'poison_bite'?")
- No hardcoded ability branches (e.g., "if ability.name == 'echolocation' then...")
- No ability registry queries at runtime

**What actually happens:**

1. **Interpreter output** (System 11): Creature emits `Set<PrimitiveEffect>` with primitive_id strings like `"apply_bite_force"`, `"inject_toxin"`, `"induce_sleep"`.

2. **Combat synthesis** (System 06): For each `force_application` primitive, create a `CombatAction` struct at encounter init. Actions reference primitives, never ability names.

3. **Combat execution**: Read action.primitive_id, action.parameters, action.stamina_cost (from primitive manifest). Apply mechanics. Never use ability labels.

4. **UI/Chronicler** (System 09): Reads primitive-effect sets SEPARATELY. Chronicler assigns labels: `{emit_acoustic_pulse, receive_acoustic_signal, spatial_integrate} → "Echolocation"`. UI displays labels to player. Mechanics never see these labels.

**Consequence**: A creature's combat actions are deterministic based on:
- Evolved channel values
- Primitive parameter expressions
- Primitive manifest costs/cooldowns
- Not based on ability names, lookup tables, or designer-authored ability definitions

---

## 6. Cross-System Hooks

**To System 01 (Evolutionary Model)**:
- Creature phenotypes enter combat as MonsterInstance objects with resolved channel profiles
- Creature stat blocks and behavior trees come from the phenotype interpreter
- Creature death yields MaterialStacks with material signatures derived from channel profiles

**To System 02 (Traits & Channels)**:
- Keeper personality channels (charisma, neural_speed) directly feed leadership capacity
- Crew members are recruited from factions and inherit faction combat training bonuses

**To System 03 (Faction Social Model)**:
- Crew trust in Keeper affects rally success probability
- Combat against faction NPCs updates faction relations
- Witnessing Keeper in combat (victory/defeat) diffuses through NPC knowledge networks

**To System 04 (Economy)**:
- Crew casualties reduce active workforce
- Equipment damage drives repair costs and material demand
- Injuries require medical materials for healing
- Settlement safety levels affect Keeper stress recovery

**To System 05 (Crafting)**:
- Material harvesting is deterministic from creature anatomy and channel profile
- Equipment quality directly affects damage computation
- Durability loss from combat feeds replacement/repair cycle

**To System 07 (Exploration)**:
- Combat triggers from exploration encounters and POI entry
- Terrain environment modifies formation slot properties
- Combat outcomes feed exploration knowledge (species observed, abilities identified)

---

## 7. Tradeoff Matrix

| Tradeoff | Complexity | Player Agency | Simulation Purity | Adoption |
|----------|-----------|---------------|-------------------|----------|
| **Formation slot properties derived from terrain** | +2 | +1 (terrain-aware tactics) | +3 | Medium |
| **Loot as creature anatomy** | +1 | +3 (harvest choice matters) | +3 | High |
| **Stamina from metabolic properties** | +1 | +1 (resource management) | +3 | High |
| **Injuries as persistent tuples** | +2 | +2 (crew investment) | +2 | Medium |
| **Rally as probabilistic continuous ability** | +1 | +2 (morale as strategy) | +3 | High |
| **Leadership budget from personality** | +1 | +1 (personality consequences) | +3 | High |
| **Cooldowns from primitive manifests** | +1 | +0 (feels more grounded) | +3 | Medium |
| **Full damage formula spec** | +1 | +1 (understandable interactions) | +3 | High |

---

## 8. Emergent Properties

- **Formation geometry creates role differentiation**: Equipment properties and formation slot engagement values mean the same crew member in different positions fights with different actions and effectiveness, creating emergent tactical variety without rerolling stats.
- **Evolved creatures produce evolved combat tactics**: A creature that evolved high neural_speed produces high-intelligence behavior trees and primitive-driven actions, which interact with the formation model in unpredictable ways. Novel creature primitive combinations produce novel tactical challenges.
- **Material pipeline drives combat progression**: Players who hunt evolved creatures get better materials → craft better equipment → crew becomes more capable → can hunt harder creatures. This is the core loop.
- **Keeper personality shapes decision-making**: Charisma-based leadership capacity means different Keeper builds have fundamentally different command styles (high charisma = many orders, low charisma = must prioritize).
- **Stress cascades through system**: Keeper stress affects crew morale, which affects formation cohesion, which affects recovery rate, which affects next turn's position, which affects available actions. Tactical depth emerges from interconnection.

---

## 9. Open Calibration Knobs

```yaml
Leadership Capacity:
  charisma_multiplier: 8.0          # how much charisma contributes to base bandwidth
  neural_speed_multiplier: 4.0      # how much neural speed contributes
  stress_empathy_dampening: 0.3     # how much empathy reduces stress penalty
  minimum_capacity_floor: 0.2       # even under max stress, keep this fraction

Formation Recovery:
  template_recovery_rate: 0.05      # base recovery per turn
  cohesion_factor_weights: { health: 0.5, morale: 0.5 }
  
Stamina:
  mass_density_scaling: 100.0       # base stamina per unit mass_density
  metabolic_bonus: 20.0
  hold_recovery_multiplier: 25.0    # stamina recovery while holding
  attack_recovery_multiplier: 5.0   # stamina recovery while attacking

Injury Healing:
  freshness_decay_rate: 0.05        # per tick
  infection_risk_accumulation: 0.02 # per untreated tick
  
Rally:
  base_charisma_success: 0.8        # Keeper charisma → success floor
  trust_weight: 0.5
  leadership_perception_weight: 0.5
```

---

## 10. Appendix: Anti-Fudges Applied

1. **Formation disruption recovery is not automatic**: Disrupted slots drift back at a rate determined by crew mobility and formation cohesion. No instant resets. Recovery is interruptible and costs crew labor.

2. **Loot is creature anatomy, not loot tables**: Creatures yield deterministic material stacks based on body regions and channel profiles. No RNG for loot type or quantity (though harvest tool and technique affect yield fraction). This eliminates the "loot casino" and makes hunting strategy meaningful.

3. **Rally is probabilistic and continuous, not once-per-combat**: Rally attempts have a success probability derived from Keeper charisma and crew trust. They can be used every turn but aren't guaranteed to work. This is more simulation-coherent than a binary "once per combat" convenience.

