# Economic Simulation Layer

> **Superseded by P6c (`emergence/60_culture_emergence.md`) and emergence/56 for settlement-as-cluster.** Specifically:
> - `SettlementEconomic.faction_id` is **removed**; a settlement is a derived spatial Leiden-level-1 cluster (doc 56) whose dominant polity is a *derived* query (`dominant_polity = polity-cluster with highest mean-allegiance among settlement members`), not a stored field.
> - Polanyi 3-mode exchange (reciprocity / redistribution / market) is replaced by ACE matching markets (P6c); the three named modes become Chronicler labels over a continuous mixing-weight vector.
> - The 17-property `MaterialSignature` is replaced by P2's emergent material-composition vector + derived signature.
> - Communal-share / redistribution formulas in this doc that read `faction_id` should be reframed to query the derived polity-cluster of the settlement.
>
> This doc remains useful as a reference for what the v1 design specified; the listed mechanics should be re-read through the emergence-doc lens.

## 1. Overview

The economic layer simulates resource production, consumption, exchange, and distribution across agents, settlements, and factions. It operates on a canonical **MaterialStack** representation: every unit of goods in the world is a structured stack of materials with 17-property signatures, quantities, freshness tags, and source lineage. This unified model eliminates the distinction between "resources" and "inventory items"—all are MaterialStacks—creating emergent economies from bottom-up agent behavior, Polanyian exchange modes (reciprocity, redistribution, market), and settlement-level aggregation.

**Core principle:** Simulation fidelity over convenience. The economy is embedded in social structure (faction governance, relationships, opinions) rather than separated from it. Economic activity feeds political conflict, which feeds economic policy, which cycles back. All exchange mechanisms (gift, redistribution, trade) coexist, weighted by governance profiles. No "capitalism" or "feudalism" presets—economics emerges.

---

## 2. Research Basis

### Substantivist Economics (Polanyi, 1944, 1957)

Karl Polanyi identified three "forms of integration"—reciprocity, redistribution, and market exchange—as mechanisms organizing pre-modern and modern economies. His central insight: economic activity in pre-industrial societies is *embedded* in social relationships and cannot be separated from kinship, politics, and ritual. Rather than assuming price-based markets and bolting on social effects, this model starts with socially-embedded provisioning and allows market behavior to emerge only when institutional conditions support it.

- Polanyi, K. (1944). *The Great Transformation*. Farrar & Rinehart.
- Polanyi, K., Arensberg, C., & Pearson, H. (1957). *Trade and Market in the Early Empires*. Free Press.

**Application:** The game's scattered archipelagos, medieval technology, small populations, and post-collapse setting mirror the societies Polanyi studied. The three exchange modes are derived from governance profiles and opinion dimensions—no new "economic system" variable is needed. Economic structure follows from political structure.

### Growing Artificial Societies (Sugarscape) (Epstein & Axtell, 1996)

Heterogeneous agents on spatially heterogeneous resource landscapes harvest, accumulate, and trade. When two resources exist, bilateral barter produces emergent price discovery, wealth inequality, and trade networks—all from local harvesting and exchange rules. Complex macro phenomena (Gini distributions, trade route formation, specialization) emerge from agents following simple local rules.

- Epstein, J.M. & Axtell, R.L. (1996). *Growing Artificial Societies: Social Science from the Bottom Up*. MIT Press.

**Application:** The game has spatially heterogeneous BiomeCells and heterogeneous AgentMinds with NeedsVectors. This model grows economic behavior from the existing substrate rather than imposing it top-down. The hybrid granularity—agents within settlements, factions between—extends the Sugarscape pattern for performance and setting appropriateness.

### Agent-Based Computational Economics (Tesfatsion & Judd, 2006)

Multi-agent systems with heterogeneous, boundedly rational agents reproduce macro phenomena (business cycles, wealth distributions) from microeconomic interaction rules. The MMO economy literature (Xu et al., 2025; Zhao et al., 2024) validates that such systems scale to game-relevant agent counts and produce realistic specialization and scarcity dynamics.

- Tesfatsion, L. & Judd, K., eds. (2006). *Handbook of Computational Economics*, Vol. 2. Elsevier.

**Application:** Validates the bottom-up approach. Heterogeneous-agent framework maps to AgentMind. The system captures bounded rationality through noise and heuristics, not optimization.

### Archaeological Trade Models (Brughmans & Poblome, 2016; Chliaoutakis & Chalkiadakis, 2020)

Agent-based models of trade in pre-industrial societies (Minoan Crete, Roman Mediterranean) demonstrate that inter-settlement trade emerges from resource heterogeneity and agent-level exchange rules. These models produce realistic signatures: trade volume distributions, settlement hierarchy patterns, and resource specialization.

- Chliaoutakis, A. & Chalkiadakis, G. (2020). "An Agent-Based Model for Simulating Inter-Settlement Trade in Past Societies." *J. Artificial Societies and Social Simulation*, 23(3).

**Application:** Directly validates hybrid granularity (agent-local, settlement-level networks). The archaeological setting closely matches the game's context.

### Bounded Rationality (Simon, 1956)

Human decision-making is bounded by cognitive limits. Agents use heuristics, satisfice rather than optimize, and make errors. These "imperfections" produce more realistic macro patterns than perfect rationality and create behavioral diversity.

- Simon, H.A. (1956). "Rational Choice and the Structure of the Environment." *Psychological Review*, 63(2).

**Application:** Activity selection and trade decisions use noise and urgency/skill heuristics, not optimization algorithms. This produces diverse specialization patterns and prevents convergence to single strategies.

### Reciprocity & Gift Exchange (Mauss, 1925; Sahlins, 1972)

Marcel Mauss analyzed the obligations created by gift exchange in non-monetary societies. A gift creates debt—not contractual, but social. Lewis Henry Sahlins distinguished gift exchange (strong social bonds), balanced reciprocity (weaker ties, delayed return), and negative reciprocity (haggling, theft). Gift exchange builds relationship capital; failure to reciprocate erodes it.

- Mauss, M. (1925). *The Gift*. Cohen & West (translated 1954).
- Sahlins, M.D. (1972). *Stone Age Economics*. Aldine-Atherton.

**Application:** Reciprocity exchange in EC5 tracks gift balances and relationship obligation. Persistent imbalance erodes relationship valence. This is concrete and measurable, not flavor.

---

## 3. Canonical Data Model: MaterialStack

Every material in the world—harvested resources, crafted goods, salvage, food, tools—is represented as a single data structure:

```
MaterialSignature {
    // 17 continuous properties (derived from creature channel profiles or environmental sources)
    impact:         float [0.0, 1.0]       // kinetic force, collision force
    hardness:       float [0.0, 1.0]       // structural rigidity, resistance to deformation
    flexibility:    float [0.0, 1.0]       // elastic deformation, bendability
    density:        float [0.0, 1.0]       // mass concentration, weight per unit
    grip:           float [0.0, 1.0]       // surface friction, hold traction
    toxicity:       float [0.0, 1.0]       // chemical danger, poison value
    purity:         float [0.0, 1.0]       // chemical homogeneity, resistance to contamination
    sensitivity:    float [0.0, 1.0]       // chemical sensing, reactivity to stimuli
    conductivity:   float [0.0, 1.0]       // thermal/electrical conductivity
    insulation:     float [0.0, 1.0]       // thermal/electrical resistance
    luminance:      float [0.0, 1.0]       // light emission, visibility
    absorbance:     float [0.0, 1.0]       // light absorption, darkness affinity
    resonance:      float [0.0, 1.0]       // vibration generation, acoustic projection
    attunement:     float [0.0, 1.0]       // vibration sensing, acoustic sensitivity
    vitality:       float [0.0, 1.0]       // regeneration rate, self-repair
    volatility:     float [0.0, 1.0]       // metabolic rate, instability
    reactivity:     float [0.0, 1.0]       // neural speed, eagerness to change
}

MaterialStack {
    signature:      MaterialSignature       // the 17-property vector
    quantity:       float                   // units of material (weight-based or count-based)
    freshness:      float [0.0, 1.0]      // 1.0 = fresh, decays over time
    lineage:        MaterialLineage         // source tracking
    acquired_tick:  int                     // when harvested/created
}

MaterialLineage {
    source_type:    enum {
        CreatureHarvest,
        EnvironmentalDeposit,
        Salvage,
        Processed,
        Trade,
    }
    source_id:      id                      // species_id, deposit_id, ruin_id, processor_id, etc.
    processing_chain: list<TechniqueId> or null  // if Processed, which techniques were applied
}
```

### Integration with Existing Systems

**Evolutionary Model:** Creature channel profiles (from Layer 3) are converted to MaterialSignatures via a canonical harvest mapping. The 17 properties are derived from the creature's channels: hardness comes from structural_rigidity, toxicity from chemical_output, vitality from regeneration_rate, etc. This creates direct novelty coupling: evolved species → new material signatures → new crafting possibilities → new equipment.

**Combat System:** Equipment uses the same MaterialSignature. An EquipmentPiece references two primary materials and their blend ratio; the equipment interpreter reads the material properties to compute combat stats. The same 17 properties mean no translation layer is needed.

**Crafting System:** Processing techniques operate on MaterialSignatures by applying transforms to individual properties. A technique specification says "multiply hardness by 1.4, multiply flexibility by 0.6, add 0.1 to conductivity if density is high." The crafting system maps material properties directly through the processing pipeline without any abstraction.

**Trade & Economics:** All inventory (agent, settlement, market) uses MaterialStacks. Surplus/deficit is computed per material per settlement. Trade is transfer of MaterialStacks. Prices are negotiated per MaterialStack type, using properties and availability as signals.

### Resource Channels as Derived Views

The "resource channels" of the original economic doc (subsistence:calories, tool:hard_material, etc.) are **derived views** over MaterialSignatures. The system doesn't compute or track channels—it tracks MaterialStacks. Channels are computed on-demand:

```
function estimate_subsistence_value(stack: MaterialStack) -> float:
    // Can this stack feed someone?
    organic_factor = stack.signature.vitality * 0.4 + stack.signature.volatility * 0.3
    calorific = (stack.signature.vitality + organic_factor) * stack.quantity
    return calorific

function estimate_tool_value_for_hardness(stack: MaterialStack) -> float:
    // Can this stack be made into a hard tool?
    toolability = stack.signature.hardness * 0.6 + stack.signature.density * 0.3
    return toolability * stack.quantity

function estimate_prestige_value(stack: MaterialStack) -> float:
    // Is this rare/exotic?
    exoticism = (stack.signature.luminance + stack.signature.resonance) * 0.5
    rarity_bonus = 1.0 if stack.signature.volatility > 0.7 else 0.5  // volatile = rare
    return exoticism * rarity_bonus * stack.quantity
```

This approach preserves the emergent economy philosophy: there are no predefined "goods categories." A MaterialStack's value in a given context emerges from its properties and what agents need. A stack with high luminance and high volatility (exotic, rare) is prestige in one context, dangerous-to-transport in another, and useless for feeding people in a third.

---

## 4. Update Rules / Layers

### Layer EC1: Resource Substrate (BiomeCells)

Each BiomeCell holds renewable and non-renewable resource stocks:

```
BiomeCellResources {
    current_stock:      MaterialStack[MAX_STACKS]    // harvestable materials
    regeneration_rate:  float[MAX_MATERIALS]         // per-tick regrowth
    non_renewable_deposit: MaterialStack[MAX_DEPOSITS] // salvage, mineral veins
}

function update_biome_cell_resources(cell, tick):
    for stack in cell.current_stock:
        // Renewable regeneration (logistic growth toward base)
        if stack.signature.vitality > 0:  // organic material
            headroom = cell.base_yield - stack.quantity
            regrowth = stack.signature.vitality * headroom * 0.001  // gentle growth
            stack.quantity += regrowth

    // Non-renewable deposits do not regenerate
    // Applied harvest pressure is tracked; depletion is permanent per-site
```

**Freshness decay in the world:** Materials in BiomeCells degrade slowly (exposed storage state, EC1 harvesting reduces freshness). Agents harvest fresh materials; older resources have lower freshness. This creates natural turnover—old harvests are gradually replaced by new growth.

### Layer EC2: Agent Economic State

Each agent has:

```
AgentEconomicState {
    inventory:          list<MaterialStack>
    capacity:           float                  // weight limit from physiology
    skill:              map<TechniqueId, float> // crafting skill; generic production skill
    consumption_rates:  map<Property, float>  // per-tick vitality burn, toxicity tolerance, etc.
    surplus:            map<MaterialStack, float> // computation from inventory vs. need horizon
    deficit:            map<MaterialStack, float> // shortfall below survival threshold
    role_history:       RoleAccumulator        // what the agent has been doing (from F2)
}

function compute_agent_surplus_deficit(agent):
    horizon_ticks = 20  // how many ticks ahead does agent plan?
    for stack in agent.inventory:
        estimated_need = agent.consumption_rates[stack] * horizon_ticks
        agent.surplus[stack] = stack.quantity - estimated_need
        agent.deficit[stack] = max(0, estimated_need - stack.quantity)

    // Feed into NeedsVector (F2 integration)
    total_subsistence_deficit = sum(
        agent.deficit[stack] for stack in agent.inventory
        where stack.signature.vitality > 0.3
    )
    agent.needs.survival = sigmoid(total_subsistence_deficit * SURVIVAL_PRESSURE)
```

### Layer EC3: Production & Consumption

**Production:** Agents choose activities (harvest, craft, salvage, idle) based on needs and personality:

```
function choose_production_activity(agent, settlement) -> Activity:
    scores = {}

    // Harvest: high deficit on organic materials → harvest urgency
    for harvestable_material in settlement.biome_cells:
        agent_deficit = agent.deficit.get(harvestable_material, 0)
        urgency = agent_deficit * NEED_WEIGHT
        skill = agent.skill.get("harvesting", 0.5)
        ambition_bonus = agent.personality.ambition * 0.1
        scores[Harvest(harvestable_material)] = urgency * 2.0 + skill + ambition_bonus

    // Craft: requires known recipes and input materials
    for recipe in agent.known_recipes:
        if agent_has_inputs(agent, recipe):
            output_urgency = sum(agent.deficit[stack] for stack in recipe.outputs)
            scores[Craft(recipe)] = output_urgency * 1.5 + agent.skill[recipe.id] * 0.8

    // Add bounded rationality noise
    for key in scores:
        scores[key] += random_uniform(-RATIONALITY_NOISE, RATIONALITY_NOISE)

    return argmax(scores) if scores else Idle

function agent_produce(agent, activity):
    match activity:
        Harvest(material):
            // Skill × material availability × crowding diminishing returns
            crowding = count_harvesters(material) / (material.current_stock + 1)
            yield = agent.skill["harvesting"] * material.current_stock / (1 + crowding * 0.5)
            harvested = MaterialStack {
                signature: material.signature,
                quantity: min(yield, material.current_stock),
                freshness: 1.0,  // fresh harvest
                lineage: CreatureHarvest { ... },
            }
            agent.inventory.append(harvested)
            material.current_stock -= harvested.quantity
            agent.skill["harvesting"] += LEARNING_RATE * yield  // practice improves skill

        Craft(recipe):
            // Consume inputs, produce outputs with property transforms
            for input_stack in recipe.inputs:
                agent.inventory.remove(input_stack)
            efficiency = agent.skill[recipe.technique_id]
            output = apply_technique(recipe.input_material, recipe.technique, efficiency)
            agent.inventory.append(output)
            agent.skill[recipe.technique_id] += LEARNING_RATE
```

**Consumption:**

```
function agent_consume(agent):
    for stack in agent.inventory:
        burn_rate = agent.consumption_rates.get(stack, 0)
        if stack.quantity > burn_rate:
            stack.quantity -= burn_rate
        else:
            shortfall = burn_rate - stack.quantity
            stack.quantity = 0
            // Apply deprivation effect
            if stack.signature.vitality > 0.3:  // food-like
                agent.needs.survival += shortfall * DEPRIVATION_SEVERITY
                if agent.deprivation_counter[stack] > LETHAL_THRESHOLD:
                    agent.alive = false
            else:
                agent.economic.skill[needed_skill] *= 0.98  // tool/material shortage degrades skill
```

### Layer EC4: Settlement Aggregation

Settlements aggregate agent state and determine macro properties:

```
SettlementEconomic {
    faction_id:     faction_id
    members:        list<agent_id>
    communal_stock: list<MaterialStack>          // shared reserves
    total_inventory: map<MaterialStack, float>   // sum of member inventories

    production_rate: map<MaterialStack, float>   // rolling avg of production/tick
    consumption_rate: map<MaterialStack, float>  // rolling avg of consumption/tick
    self_sufficiency: map<MaterialStack, float>  // production / consumption ratio
    exchange_mode:   ExchangeMode                 // reciprocity / redistribution / market weights
}

function update_settlement_economy(settlement, tick):
    // Aggregate
    settlement.total_inventory.clear()
    settlement.production_rate = rolling_avg(this_tick_production, settlement.production_rate)
    settlement.consumption_rate = rolling_avg(this_tick_consumption, settlement.consumption_rate)

    for agent in settlement.members:
        for stack in agent.inventory:
            settlement.total_inventory[stack.signature] += stack.quantity

    // Self-sufficiency per material signature
    for sig in ALL_MATERIAL_SIGNATURES:
        prod = settlement.production_rate.get(sig, 0)
        cons = settlement.consumption_rate.get(sig, 0)
        if cons > 0:
            settlement.self_sufficiency[sig] = prod / cons
        else:
            settlement.self_sufficiency[sig] = 1.0

    // Update exchange mode from governance
    settlement.exchange_mode = determine_exchange_mode(settlement)

    // Redistribute from agent → communal based on collectivism opinion
    for agent in settlement.members:
        for stack in agent.inventory where stack.quantity > 0:
            collectivism = get_faction(settlement.faction_id).centroid[0]  // opinion dim 0
            communal_share = clamp((collectivism + 1.0) / 2.0, 0.0, 0.8)
            communal_take = stack.quantity * communal_share
            if communal_take > 0:
                stack.quantity -= communal_take
                settlement.communal_stock.append(MaterialStack { ... communal_take ... })
```

### Layer EC5: Exchange Modes (Polanyi)

Three modes coexist, weighted by governance + opinions:

```
ExchangeMode {
    reciprocity_weight:    float
    redistribution_weight: float
    market_weight:         float
    // Sum to 1.0
}

function determine_exchange_mode(settlement) -> ExchangeMode:
    gov = get_faction(settlement.faction_id).governance_profile

    reciprocity = (
        (1.0 - gov.centralization) * 0.4
        + (1.0 - gov.formality) * 0.3
        + (1.0 if settlement.population < 50 else 0.0) * 0.3
    )

    redistribution = (
        gov.centralization * 0.4
        + (get_faction(settlement.faction_id).centroid[0] + 1.0) / 2.0 * 0.3
        + gov.formality * 0.2
        + gov.legitimacy * 0.1
    )

    market = (
        (1.0 - (get_faction(settlement.faction_id).centroid[0] + 1.0) / 2.0) * 0.3
        + (1.0 if settlement.population > 100 else 0.0) * 0.3
        + count_known_trade_partners(settlement) / 10.0 * 0.2
    )

    total = reciprocity + redistribution + market
    return ExchangeMode {
        reciprocity: reciprocity / total,
        redistribution: redistribution / total,
        market: market / total,
    }

function run_local_exchange(settlement, tick):
    mode = settlement.exchange_mode

    if mode.reciprocity_weight > 0.1:
        reciprocity_exchange(settlement)  // gifts

    if mode.redistribution_weight > 0.1:
        redistribution_exchange(settlement)  // authority collects and distributes

    if mode.market_weight > 0.1:
        market_exchange(settlement)  // bilateral barter
```

**Reciprocity exchange:** Agents with surplus gift to agents with deficit who they have strong relationships with. Gifts strengthen bonds. Failure to reciprocate erodes relationship (Mauss/Sahlins).

**Redistribution exchange:** Central authority (highest-legitimacy member) collects contributions proportional to surplus, distributes according to their hoarding_vs_distribution opinion. Unfair distribution erodes legitimacy and shifts opinions toward egalitarianism.

**Market exchange:** Agents post sell/buy orders. Orders match if a seller wants what a buyer has surplus of. Price is negotiated via Marginal Rate of Substitution (MRS) with attitude modifiers.

### Layer EC6: Inter-Settlement Trade

```
TradeRoute {
    settlement_a:   settlement_id
    settlement_b:   settlement_id
    distance:       float
    risk:           float  // hazards, faction conflict
    frequency:      float  // expeditions per K ticks
    price_differential: map<MaterialStack, float>
    knowledge_bandwidth: float  // information flow (F4 integration)
}

function discover_trade_routes(settlements):
    for pair (S_a, S_b):
        // Both must know each other exists
        if not (faction_knows(S_a.faction, S_b) and faction_knows(S_b.faction, S_a)):
            continue

        // Navigable path must exist
        path = find_path(S_a, S_b)
        if path is null:
            continue

        // Price differential must justify trip
        diff = compute_price_differential(S_a, S_b)
        if max(diff.values()) < MIN_INCENTIVE:
            continue

        // Diplomatic relations must permit trade
        rel = get_relation(S_a.faction, S_b.faction)
        if rel and rel.diplomatic < TRADE_THRESHOLD:
            continue

        create_trade_route(S_a, S_b, path)

function execute_inter_settlement_trade(route, tick):
    S_a = get_settlement(route.settlement_a)
    S_b = get_settlement(route.settlement_b)

    // For each material where S_a has surplus and S_b has deficit:
    for sig in ALL_MATERIAL_SIGNATURES:
        a_surplus = S_a.net_production[sig]
        b_deficit = S_b.deficit[sig]

        if a_surplus > 0 and b_deficit > 0:
            // Find best reciprocal material from B
            best_return_sig = find_best_return(S_a, S_b, sig)
            if best_return_sig:
                export_qty = min(a_surplus * 0.5, b_deficit, route.capacity)
                return_qty = export_qty * compute_exchange_ratio(S_a, S_b, sig, best_return_sig)

                // Execute transfer with risk-based loss
                loss_factor = 1.0 - route.risk * 0.15
                S_a.communal_stock[sig] -= export_qty
                S_b.communal_stock[sig] += export_qty * loss_factor
                S_b.communal_stock[best_return_sig] -= return_qty
                S_a.communal_stock[best_return_sig] += return_qty * loss_factor

                record_trade_price(S_a, S_b, sig, best_return_sig, return_qty / export_qty)

    // Trade = information highway (F4 integration)
    route.knowledge_bandwidth = route.capacity * KNOWLEDGE_PER_UNIT_TRADE
    transfer_knowledge_between_factions(S_a.faction, S_b.faction, route.knowledge_bandwidth)
```

---

## 5. Cross-System Hooks

### To Faction Social Model (F1–F6)

- **EC1 → F2:** Resource scarcity shifts NeedsVector.survival, which increases salience of resource-opinion dimensions.
- **EC2 → F2:** Agent production history feeds the BehaviorAccumulator; emergent roles (fisherman, smith) are detected from economic activity.
- **EC4 → F6:** Economic self-sufficiency influences faction stability and governance shifts.
- **EC5 → F6:** Exchange mode is derived from governance but also reinforces it. Successful redistribution increases legitimacy; market exchange empowers wealthy agents.
- **EC6 → F4:** Trade volume determines knowledge bandwidth between factions.
- **EC6 → F6:** Trade volume is a field on FactionRelation, now concretely driven by actual resource flows.

### To Evolutionary Model

- **EC1 ← Layer 4:** Creature populations affect resource availability (harvestable yields).
- **EC3 → Layer 4:** NPC harvesting pressure becomes selection pressure on evolved creatures.
- **EC1 ↔ EC3:** Salvage deposits (from ruin POIs) are economic motivations for exploration.

### To Crafting System (K1–K5)

- **EC1 ↔ K1:** Harvested materials are MaterialStacks; same representation.
- **EC3 ↔ K2:** Processing transforms material properties; recipes use MaterialSignatures.
- **EC4 ↔ K2:** Settlement facilities (forge, workshop) enable advanced techniques.
- **EC5/EC6 ↔ K2:** Faction traditions are technique libraries; learning techniques requires trade or social standing.

### To Combat System (C1–C3)

- **EC3 → C1:** Equipment crafted from materials and equipped by agents.
- **C3 → EC1:** Combat yields material drops (corpse harvest).
- **C3 → K4:** Equipment durability degrades from use, tracked in crafting inventory.

---

## 6. Tradeoff Matrix

| Decision | Options | Fidelity | Implementability | Legibility | Emergent Power | Why |
|----------|---------|----------|------------------|-----------|-----------------|-----|
| **Unified material representation** | A) Separate channels + items | Medium | Easy | High | Low | B) Single MaterialStack with properties |
| | B) MaterialStack with 17 properties | High | Medium | Medium | High | Resolves crafting/economy inconsistency; every material is the same type |
| **Exchange modes** | A) Discrete toggle (market/not market) | Low | Easy | High | Low | Doesn't match reality |
| | B) Continuous blend of Polanyi three | High | Medium | Medium | High | Matches ethnographic and historical patterns; governance naturally drives weighting |
| **Reciprocity mechanics** | A) Flavor text only | Low | Easy | High | Low | No gameplay consequence |
| | B) Explicit gift balance tracking + relationship erosion | High | Medium | Medium | High | Mauss/Sahlins framework; failed reciprocity has economic/political consequences |
| **Redistribution fairness** | A) Authority chooses allocations arbitrarily | Low | Easy | Medium | Low | Doesn't test legitimacy |
| | B) Fairness computed → opinion shift → potential rebellion | High | Medium | Medium | High | Corrupt redistribution erodes legitimacy through existing opinion mechanics |
| **Inter-settlement trade** | A) Player brokers all trades manually | Medium | Hard | High | Low | Too much player busywork; doesn't scale to multiple settlements |
| | B) Automated settlement-level expeditions; player can disrupt/enable | High | Medium | Medium | High | Faction organizes trade; player influences via exploration, diplomacy, or piracy |
| **Resource regeneration** | A) Infinite supply with scaling density | Low | Easy | High | Low | No scarcity pressure |
| | B) Logistic growth; non-renewables genuinely deplete | High | Easy | Medium | High | Creates meaningful harvest pressure and exploration motivation |
| **Skill system** | A) Generic "crafting skill" stat | Low | Easy | High | Low | No feedback to material demand |
| | B) Skill-by-technique learned through practice | High | Medium | Low | High | Specialization emerges; agents improve by doing |

---

## 7. Emergent Properties

1. **Specialization without assignment:** Agents who repeatedly harvest one material become skilled at it, producing more yield and lower freshness decay. This creates emergent roles (fisherman, smith, salvager) without role mechanics.

2. **Trade route formation:** When settlements are isolated, they synthesize what they can. As they discover trade partners with complementary production, routes form. Trade initially carries high risk but economic incentive drives continuous attempts until routes stabilize.

3. **Economic crisis cascades:** Over-harvesting depletes a resource → shortage → agent deprivation → population loss → reduced production → reduced trade → dependent settlements enter crisis → potential emigration/merger/conflict.

4. **Wealth inequality feedback:** Market-mode settlements with successful traders accumulate MaterialStack inventories. High-wealth agents can afford better tools (via crafting layer), produce better equipment, consolidate power. Without redistribution opinion support, inequality grows. With redistribution support, authority extracts for communal pool.

5. **Exchange mode transitions:** Small, decentralized factions lean reciprocal (tight kinship networks). As they grow and formalize, they shift toward redistribution (hierarchy, authority). If market opinion rises, they shift toward market exchange (large anonymous populations, price-based allocation). Modes can cycle as governance opinions shift.

6. **Salvage exhaustion crises:** High-dependency settlements exhaust local salvage. If salvage_dependency > 0.5 and available_salvage < consumption, settlement_health declines. Agents emigrate to settlements with salvage access. Failed salvage can trigger conflict over ruin access.

7. **Tribute systems from economics:** Vassal settlement regularly transfers MaterialStacks to overlord. If vassal can't meet tribute, deficit accumulates as "tribute_debt." Debt above threshold triggers overlord's choice: forgive (opinion gain, legitimacy cost) or escalate (political instability, potential rebellion).

---

## 8. Open Calibration Knobs

- **LEARNING_RATE:** How fast agents improve skill through practice.
- **DEPRIVATION_SEVERITY:** How much unmet need shifts opinions/triggers death.
- **RATIONALITY_NOISE:** Bounded rationality heuristic noise; higher = more diverse behavior, lower = specialization.
- **GIFT_FRACTION, GIFT_VALENCE_BOOST, GIFT_OBLIGATION_BOOST:** Reciprocity exchange parameters.
- **COLLECTION_RATE, BASE_COLLECTION_RATE:** Redistribution authority extraction rate.
- **TRADE_WILLINGNESS, TRADE_DIPLOMACY_THRESHOLD:** Market exchange willingness and diplomatic gating.
- **FRESHNESS_DEGRADATION_RATE:** How fast materials lose properties; per StorageState.
- **SETTLEMENT_POPULATION_THRESHOLDS:** Where reciprocity vs. market shifts activate.
- **SALVAGE_REGENERATION_RATE (0):** Ensures non-renewable salvage never regrows.
- **TRANSPORT_LOSS_FACTOR:** Risk-based loss on inter-settlement trade (shipwreck, banditry).
- **TRIBUTARY_RATE:** How much vassal owes overlord per tick.
- **OPINION_SHIFT_PER_UNFAIR_REDISTRIBUTION:** How much egalitarianism opinion shifts when distribution is hoarded.

---

## References

- Polanyi, K. (1944). *The Great Transformation*. Farrar & Rinehart.
- Polanyi, K., Arensberg, C., & Pearson, H. (1957). *Trade and Market in the Early Empires*. Free Press.
- Epstein, J.M. & Axtell, R.L. (1996). *Growing Artificial Societies: Social Science from the Bottom Up*. MIT Press.
- Simon, H.A. (1956). "Rational Choice and the Structure of the Environment." *Psychological Review*, 63(2).
- Mauss, M. (1925). *The Gift*. Cohen & West (1954 translation).
- Sahlins, M.D. (1972). *Stone Age Economics*. Aldine-Atherton.
- Brughmans, T. & Poblome, J. (2016). "MERCURY: An Agent-Based Model of Post-Bronze Age Collapse Trade." *Journal of Archaeological Method and Theory*, 23(3), 661–691.
- Chliaoutakis, A. & Chalkiadakis, G. (2020). "An Agent-Based Model for Simulating Inter-Settlement Trade in Past Societies." *Journal of Artificial Societies and Social Simulation*, 23(3).
