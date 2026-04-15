# Crafting & Material Processing System

## 1. Overview

The crafting system transforms MaterialStacks (the canonical material representation from the economic layer) through sequences of processing techniques, shapes them into equipment forms, and resolves their properties into gameplay-meaningful equipment. There are no recipes: the player selects materials, applies techniques that shift their 17-property signatures, shapes the result into a form, and the equipment interpreter converts the final material properties into combat stats and abilities.

**Core principle:** Craft is character design. Materials are the content. Evolution of creatures produces novel materials; novel materials enable novel equipment. Faction traditions teach processing techniques, creating social reasons to engage with multiple factions. Tool quality IS crafting skill—a masterwork hammer crafted from evolved creature materials is mechanically superior to a crude stone tool, creating a recursive improvement loop: hunt creatures → craft better tools → craft better equipment → hunt harder creatures.

---

## 2. Research Basis

### Property-Based Crafting (Ryzom, Dwarf Fortress, Monster Hunter)

**Ryzom** demonstrates that material stat profiles (quality tiers and property variations) drive crafting strategy more than recipe variation. The same weapon plan with different materials produces radically different results. **Dwarf Fortress** shows that material properties are simulation-meaningful—they resolve against combat physics, not abstraction layers. **Monster Hunter** establishes that hunt → harvest → craft → hunt loops are the primary progression mechanic.

- Ryzom Wiki. "Crafting." https://en.wiki.ryzom.com/wiki/Crafting
- Dwarf Fortress Wiki. "Material science." https://dwarffortresswiki.org/index.php/DF2014:Material_science
- Wikipedia. "Monster Hunter Series." https://en.wikipedia.org/wiki/Monster_Hunter

**Adoption:** Materials are the craft content, not recipes. Property-based transforms replace named item crafting.

### Parametric Crafting (DHQ, 2017; Envato Tuts+, 2015; bit-tech, 2018)

Crafting systems exist on a spectrum from abstract ("Combine A + B = C") to simulationist ("Adjust continuous parameters; resolve results through physics"). Parametric recipes specify *property requirements*, not specific ingredients, and carry input properties through to outputs with transforms. Failed crafting produces flawed-but-real objects, not total loss.

- Buruk, O. et al. "Crafting in Games." *Digital Humanities Quarterly*, 11(4), 2017.
- Engström, H. "5 Approaches to Crafting Systems in Games." *Envato Tuts+*, 2015.
- Lane, R. "Developers! Here's how to fix your stupid crafting system." *bit-tech*, 2018.

**Adoption:** Recipe-less system with property transforms. Failure produces artifacts.

### Archaeological Material Science (Kleiber's Law, Allometric Scaling)

Real crafting knowledge is embodied in tools and processes, not in abstract optimization. Tool quality (derived from material properties) directly predicts outcome quality. Specialized tools for specialized tasks create bottlenecks and progression.

- Kleiber, M. (1932). "Body Size as a Factor in Metabolism." *Hilgardia*, 6(11), 315–353.

**Adoption:** Tool quality as skill replacement. Tool ideal-profiles drive specialization.

---

## 3. Canonical Data Model: MaterialStack & Signatures

All crafting operates on MaterialStacks. Every stack has a 17-property MaterialSignature:

```
MaterialSignature {
    impact, hardness, flexibility, density, grip, toxicity, purity, sensitivity,
    conductivity, insulation, luminance, absorbance, resonance, attunement,
    vitality, volatility, reactivity
    // All float [0.0, 1.0]
}

MaterialStack {
    signature:      MaterialSignature
    quantity:       float
    freshness:      float [0.0, 1.0]   // 1.0 fresh, decays per storage state
    lineage:        MaterialLineage    // source: creature, deposit, salvage, processed
    acquired_tick:  int
}
```

**Derived from evolutionary channels:** When a creature is harvested, its channel profile is converted to a MaterialSignature:

```
function harvest_creature_region(creature, region) -> MaterialSignature:
    channels = creature.phenotype.body_map[region]
    return MaterialSignature {
        impact:       channels[KINETIC_FORCE] * 0.8 + channels[MASS_DENSITY] * 0.2,
        hardness:     channels[STRUCTURAL_RIGIDITY] * 0.9 + channels[MASS_DENSITY] * 0.1,
        flexibility:  channels[ELASTIC_DEFORMATION] * 0.85 + channels[REGENERATION] * 0.15,
        density:      channels[MASS_DENSITY],
        grip:         channels[SURFACE_FRICTION],
        toxicity:     channels[CHEMICAL_OUTPUT],
        // ... (remaining 11 properties mapped from creature channels)
    }
```

**Lineage tracking:** Every MaterialStack remembers its source—creature species + body region, deposit type, salvage era, or processing chain. The UI exposes this provenance. A stack labeled "Bright Horizon salvage" has different properties and availability than "Tidecaller-hunted abyssal shell."

---

## 4. Update Rules / Layers

### Layer K1: Material Acquisition & Storage

Materials enter the world from three sources:

**A) Creature Harvesting**

Killing a creature yields MaterialStacks from each harvestable body region (dorsal shell, venom gland, muscle fiber, etc.). Different regions have different MaterialSignatures derived from their channel profiles:

```
function harvest_creature(creature: MonsterInstance, region: BodyRegion) -> MaterialStack:
    signature = harvest_creature_region(creature, region)
    yield = compute_yield(creature.mass, region)
    return MaterialStack {
        signature: signature,
        quantity: yield,
        freshness: 1.0,
        lineage: CreatureHarvest { species_id: creature.species, body_region: region },
        acquired_tick: current_tick,
    }
```

**Design consequence:** A player hunting a shell-based creature learns that the hard dorsal region yields high-hardness material (good for armor) while the underbelly yields high-flexibility material (good for wrappings). Careful harvesting across regions yields material diversity from a single kill.

**B) Environmental Deposits**

Mineral veins, plant-rich biomes, and resin deposits yield environmental materials. Their base signatures are fixed but modulated by local biome conditions:

```
function harvest_deposit(deposit: EnvironmentalDeposit) -> MaterialStack:
    base_sig = DEPOSIT_BASE_SIGNATURES[deposit.type]
    biome = get_biome_cell(deposit.location)
    
    modifier = MaterialSignature {
        conductivity: biome.temperature * 0.1,    // hot = conductive
        insulation: (1.0 - biome.temperature) * 0.1,
        density: biome.elevation * 0.05,
        vitality: biome.moisture * 0.08,
    }
    
    return MaterialStack {
        signature: blend(base_sig, modifier, 0.8),
        quantity: compute_yield(deposit.size),
        freshness: 1.0,
        lineage: EnvironmentalDeposit { ... },
    }
```

**Design consequence:** Environmental materials are baseline for early-game crafting. They also serve as blending agents—pure materials with extreme properties can be blended with environmentals to shift toward desired profiles.

**C) Salvage from Ruins**

Non-renewable, era-specific materials from Bright Horizon, Synthesis, and earlier eras. Each era has distinct property profiles reflecting technology of that age:

```
ERA_SALVAGE_PROFILES = {
    BrightHorizon: { conductivity: 0.85, reactivity: 0.9, hardness: 0.6, ... },
    Synthesis:     { resonance: 0.8, attunement: 0.85, purity: 0.9, ... },
    SailAscendancy: { hardness: 0.9, grip: 0.7, flexibility: 0.5, ... },
}
```

Salvage materials are often extreme in multiple properties, making them powerful but unpredictable. Incorporating salvage into equipment produces extraordinary results or catastrophic failures.

### Freshness & Degradation

Materials degrade based on storage conditions:

```
function degrade_material(stack: MaterialStack, elapsed_ticks: int):
    storage_rate = DEGRADATION_RATES[stack.storage_state]
    
    organic_factor = 1.0 + stack.signature.vitality * 0.3 + stack.signature.volatility * 0.2
    
    stack.freshness -= storage_rate * organic_factor * elapsed_ticks
    
    if stack.freshness < 0.7:
        stack.signature.toxicity *= 0.95
        stack.signature.reactivity *= 0.97
        stack.signature.luminance *= 0.96
    
    if stack.freshness < 0.3:
        stack.signature.flexibility *= 0.98
        stack.signature.grip *= 0.97
```

**Design consequence:** Fresh creature materials with high vitality/volatility are time-sensitive. Using them immediately preserves exotic properties. Storing them delays but degrades them. Preservation techniques (from crafting) extend freshness at cost. This creates meaningful hunt rhythm—fresh kills → immediate crafting → urgency.

### Layer K2: Material Processing

Processing is the core mechanic. Techniques are functions that transform MaterialSignatures via property-level operations:

```
ProcessingTechnique {
    id:              technique_id
    name:            string              // "salt hardening", "venom infusion", etc.
    category:        enum { Thermal, Chemical, Mechanical, Biological, Composite }
    
    transforms:      list<PropertyTransform>  // what changes
    time_cost:       float
    fuel_cost:       float or null
    reagent:         MaterialRequirement or null
    
    tool_category:   ToolCategory        // which tool type is primary
    tool_scaling:    float               // how much tool quality improves outcome
    
    requirements:    TechniqueRequirements
}

PropertyTransform {
    target:          Property            // which of 17 properties
    operation:       enum {
        Add(value),
        Multiply(factor),
        Transfer { from: Property, to: Property, ratio: float },
        Conditional { condition_property, threshold, then_op },
    }
    magnitude:       float               // base strength
    scaling_source:  Property or null    // if set, magnitude *= input[scaling_source]
}
```

**Example techniques:**

```
forge_hardening:
    transforms:
        - target: hardness, op: Multiply(1.4)
        - target: flexibility, op: Multiply(0.6)
        - target: toxicity, op: Multiply(0.3)
        - target: conductivity, op: Add(0.1), scaling_source: density
    tool_category: Hammer
    facility: Forge

venom_infusion:
    transforms:
        - target: toxicity, op: Transfer { from: reactivity, to: toxicity, ratio: 0.5 }
        - target: grip, op: Multiply(0.8)
        - target: conductivity, op: Conditional {
              condition_property: toxicity, threshold: 0.5,
              then_op: Add(0.15)
          }
    reagent: { property_requirement: { toxicity: { min: 0.4 } }, blend_ratio: 0.3 }
    tool_category: Vessel

symbiotic_culture:
    transforms:
        - target: vitality, op: Multiply(2.0)
        - target: volatility, op: Add(0.2)
        - target: luminance, op: Conditional { condition_property: vitality, threshold: 0.4, then_op: Add(0.1) }
        - target: hardness, op: Multiply(0.7)
    environment: { moisture: { min: 0.5 }, temperature: { min: 0.4 } }
    time_cost: 50
    tool_category: Culture_Kit
```

### Technique Resolution

```
function apply_technique(
    material: MaterialStack,
    technique: ProcessingTechnique,
    tool: EquipmentPiece,           // the crafting tool
    facility_quality: float,
    reagent: MaterialStack or null
) -> ProcessingResult:

    output = copy(material.signature)
    
    // Tool quality IS skill
    tool_effectiveness = compute_tool_effectiveness(tool, technique)
    effectiveness = base_effectiveness(technique, tool_effectiveness, facility_quality)
    // Range: 0.3 (crude tool, poor facility) to 1.2 (masterwork tool, excellent facility)
    
    for transform in technique.transforms:
        magnitude = transform.magnitude * effectiveness
        if transform.scaling_source:
            magnitude *= material.signature[transform.scaling_source]
        
        match transform.operation:
            Add(v):
                output[transform.target] += v * magnitude
            Multiply(f):
                output[transform.target] *= lerp(1.0, f, magnitude)
            Transfer { from, to, ratio }:
                amount = output[from] * ratio * magnitude
                output[from] -= amount
                output[to] += amount
            Conditional { condition_property, threshold, then_op }:
                if output[condition_property] > threshold:
                    apply_op(output, transform.target, then_op, magnitude)
    
    // Clamp properties to [0, 1]
    for prop in output:
        output[prop] = clamp(output[prop], 0.0, 1.0)
    
    return ProcessingResult {
        output_signature: output,
        quality_modifier: effectiveness,
        time_taken: technique.time_cost / effectiveness,
        source: Processed { source: material.lineage, technique: technique.id },
    }
```

### Tool Quality Model

Tools are equipment made from materials. Tool effectiveness at a technique depends on how well the tool's material matches the technique's demands:

```
TOOL_IDEAL_PROFILES = {
    Hammer:    { impact: 0.8, hardness: 0.7, density: 0.6 },
    Tongs:     { insulation: 0.8, grip: 0.7, hardness: 0.5 },
    Blade:     { hardness: 0.8, grip: 0.6, flexibility: 0.5 },
    Vessel:    { purity: 0.8, insulation: 0.6, hardness: 0.4 },
    Press:     { hardness: 0.7, density: 0.7, grip: 0.5 },
    Needle:    { hardness: 0.6, flexibility: 0.7, reactivity: 0.5 },
    Culture_Kit: { vitality: 0.8, purity: 0.6, sensitivity: 0.5 },
    Lens:      { luminance: 0.6, sensitivity: 0.7, attunement: 0.6 },
}

function compute_tool_effectiveness(tool: EquipmentPiece, technique: ProcessingTechnique) -> float:
    ideal = TOOL_IDEAL_PROFILES[technique.tool_category]
    tool_sig = blend_materials(tool.primary_material, tool.secondary_material, tool.material_ratio)
    
    match_score = 0.0
    total_weight = 0.0
    for (prop, ideal_value) in ideal:
        weight = ideal_value
        actual = tool_sig[prop]
        if actual >= ideal_value:
            contribution = 1.0 + (actual - ideal_value) * 0.2
        else:
            contribution = actual / ideal_value
        match_score += contribution * weight
        total_weight += weight
    
    base = match_score / total_weight
    durability_factor = tool.durability / tool.max_durability
    base *= lerp(0.5, 1.0, durability_factor)  // worn tool is 50% effective
    base *= (1.0 + tool.form.complexity * 0.2)  // intricate tools enable finer work
    
    return clamp(base, 0.1, 1.5)
```

**Design consequence:** Better tools (made from evolved creature materials) enable better crafting outcomes. A crafter with a masterwork hammer outperforms one with a stone tool. This creates the bootstrap loop: starter tools are crude salvage seeded in starter biomes (worldgen guarantee). Players improve tools by hunting better creatures, which enables better equipment, which enables hunting better creatures.

### Technique Composition

Techniques compose. Output of one becomes input of next:

```
function process_chain(
    material: MaterialStack,
    techniques: list<(Technique, params)>,
    tool_loadout: map<ToolCategory, EquipmentPiece>,
    facilities: FacilitySet
) -> ProcessingResult:

    current = material
    cumulative_quality = 1.0
    
    for (technique, params) in techniques:
        tool = tool_loadout[technique.tool_category]
        result = apply_technique(current, technique, tool, params.facility_quality, params.reagent)
        current = result.as_material_stack()
        cumulative_quality *= result.quality_modifier
        current.freshness *= 0.95  // each step degrades freshness slightly
    
    return ProcessingResult {
        output_signature: current.signature,
        quality_modifier: cumulative_quality,
        processing_chain: techniques.map(t => t.id),
    }
```

**Example:** High-rigidity, high-chemical-output creature → harvest dorsal region → apply forge_hardening (boost hardness, burn toxicity) → apply venom_infusion using toxic reagent (preserve some toxicity, add conductivity) → result: extreme hardness + moderate toxicity + good purity + mild conductivity. A weapon that hits hard and poisons on contact. No recipe told the player to do this—they reasoned through material properties and technique effects.

### Faction Traditions

Factions teach technique libraries with bonuses and aesthetic modifiers:

```
FactionCraftingTradition {
    faction_id:         faction_id
    available_techniques: set<TechniqueId>
    technique_bonuses:   map<TechniqueId, float>  // +skill bonus
    aesthetic_modifiers: AestheticProfile
    forbidden_techniques: set<TechniqueId or pattern>
}

// Example: Tidecaller coastal faction
tidecaller_tradition:
    available_techniques:
        - salt_hardening         // unique to coastal
        - brine_quench           // uses seawater
        - coral_lamination       // with living coral
        - symbiotic_culture      // excellent at biological
    technique_bonuses:
        salt_hardening: +0.15
        symbiotic_culture: +0.2  // Tidecaller expertise
    aesthetic_modifiers:
        color_bias: blue-green
        texture_bias: organic, flowing
        decoration_style: wave patterns
    forbidden_techniques:
        - forge_hardening        // fire violates ecological harmony
```

**Design consequence:** Players who ally with Covenant (mountain faction) get thermal/mineral techniques. Players who ally with Tidecallers get biological/chemical techniques. To access full technique library, engage multiple factions. Using forbidden techniques damages standing.

### Layer K3: Form Shaping

Once processed, materials are shaped into equipment. The player sets continuous form parameters; the material constrains what's achievable:

```
FormParameters {
    slot:        EquipmentSlot  // Weapon, Armor, Tool, Accessory
    reach:       float [0.0, 1.0]
    coverage:    float [0.0, 1.0]
    complexity:  float [0.0, 1.0]
    mass:        float           // computed from density × form volume
}

function compute_form_constraints(material: MaterialSignature) -> FormConstraints:
    return FormConstraints {
        max_reach: clamp(
            material.hardness * 0.6 + material.flexibility * 0.4,
            0.1, 1.0
        ),
        max_coverage: clamp(
            material.flexibility * 0.5 + material.density * 0.3 + material.hardness * 0.2,
            0.1, 1.0
        ),
        max_complexity: clamp(
            material.reactivity * 0.3 + material.flexibility * 0.3 +
            material.grip * 0.2 + material.purity * 0.2,
            0.1, 1.0
        ),
        mass_per_unit: material.density * MASS_SCALING_FACTOR,
        durability_base: material.hardness * 0.4 + material.flexibility * 0.3 +
                         material.vitality * 0.3,
    }
```

**Design consequence:** Materials with high hardness and low flexibility can't achieve high reach (would snap). Materials with high reactivity, high flexibility, and high purity can achieve high complexity (intricate mechanisms work). This creates meaningful material hunting goals: "I need something hard and flexible for a long weapon."

### Dual-Material Blending

Equipment uses two materials with a blend ratio. Synergy/conflict rules apply:

```
function blend_for_equipment(
    primary: MaterialSignature,
    secondary: MaterialSignature,
    ratio: float,
    form: FormParameters
) -> MaterialSignature:
    blended = LinearBlend(primary, secondary, ratio)
    
    for prop in ALL_PROPERTIES:
        synergy = compute_synergy(prop, primary, secondary)
        blended[prop] *= (1.0 + synergy * SYNERGY_SCALE)
        
        conflict = compute_conflict(prop, primary, secondary)
        blended[prop] *= (1.0 - conflict * CONFLICT_SCALE)
    
    return clamp_all(blended, 0.0, 1.0)

function compute_synergy(prop, primary, secondary) -> float:
    if prop == hardness and primary.hardness > 0.7 and secondary.flexibility > 0.6:
        return 0.3  // steel + carbon fiber = superlinear hardness
    if prop == toxicity and primary.toxicity > 0.5 and secondary.conductivity > 0.6:
        return 0.4  // toxic + conductive = better poison delivery
    // ... more synergy rules
    return 0.0

function compute_conflict(prop, primary, secondary) -> float:
    if prop == vitality and primary.vitality > 0.6 and secondary.volatility > 0.6:
        return 0.5  // living + unstable = mutual destruction
    // ... more conflict rules
    return 0.0
```

**Design note:** Synergy/conflict rules are authored content, like interpreter assembly rules. They are tuned per property pair; new ones are data additions, not code changes.

### Layer K4: Quality & Outcome Resolution

Every crafted item receives a quality score that modifies stats:

```
function compute_quality(
    material: MaterialSignature,
    form: FormParameters,
    processing_chain: list<TechniqueId>,
    tool_effectiveness: float,
    facility_quality: float,
    form_constraint_usage: float
) -> QualityResult:

    // Material coherence: extreme properties in many dimensions = harder to work
    property_variance = variance(material.all_values())
    coherence = 1.0 / (1.0 + property_variance * COHERENCE_PENALTY)
    
    // Technique sequence synergy
    chain_bonus = compute_chain_synergy(processing_chain)
    
    // Tool contribution
    tool_factor = tool_effectiveness * (1.0 + facility_quality * 0.3)
    
    // Form constraint penalty: quadratic near edges
    limit_penalty = form_constraint_usage ^ 2
    
    quality = (coherence * 0.3 + chain_bonus * 0.2 + tool_factor * 0.4) * (1.0 - limit_penalty * 0.3)
    quality = clamp(quality, 0.05, 1.0)
    
    return QualityResult {
        quality_score: quality,
        durability_modifier: quality * 1.5,
        stat_modifier: 0.7 + quality * 0.6,  // 0.05 → 73%, 1.0 → 130%
        ability_threshold_modifier: quality,
    }
```

### Failure Modes

Poor choices produce flawed-but-real equipment:

```
FailureMode = enum {
    Brittle,    // high hardness, low flexibility → shatters on impact
    Unstable,   // high volatility, low purity → properties fluctuate
    Inert,      // processing killed active properties → minimal abilities
    Warped,     // exceeded form constraints → stat penalties
    Reactive,   // incompatible blend → random secondary effects
    Leaking,    // poorly contained toxicity/conductivity → harms wielder
}

function check_failure_modes(
    material: MaterialSignature,
    form: FormParameters,
    quality: QualityResult
) -> list<FailureMode>:
    failures = []
    if material.hardness > 0.8 and material.flexibility < 0.1:
        failures.append(Brittle)
    if material.volatility > 0.6 and material.purity < 0.3:
        failures.append(Unstable)
    if quality.quality_score < 0.15:
        failures.append(Inert)
    if form_exceeds_constraints(form, material):
        failures.append(Warped)
    // ... more checks
    return failures
```

**Design consequence:** Exotic equipment made at the edge of skill is powerful but risky. This is interesting, not punishing.

### Byproducts

Processing yields secondary materials:

```
function compute_byproducts(
    input: MaterialStack,
    technique: ProcessingTechnique,
    result: ProcessingResult
) -> list<MaterialStack>:
    byproducts = []
    
    if technique.category == Thermal:
        ash = MaterialStack {
            signature: residual_signature(input, technique, "thermal_residue"),
            quantity: input.quantity * 0.15,
            lineage: Processed { source: input.lineage, technique: technique.id },
        }
        byproducts.append(ash)
    
    if technique.reagent:
        spent = MaterialStack {
            signature: depleted_signature(technique.reagent_signature),
            quantity: technique.reagent_quantity * 0.6,
        }
        byproducts.append(spent)
    
    return byproducts
```

Byproducts are often low-value, but discovering that one technique's byproduct is ideal for another is a depth moment.

### Layer K5: Player Interface

The Workbench is the crafting UI. Four panels:

```
WorkbenchUI {
    material_selector:    MaterialPanel       // browse inventory
    process_chain:        ProcessPanel        // build technique sequences
    form_shaper:          FormPanel           // set form parameters
    outcome_preview:      PreviewPanel        // real-time stat preview
}
```

**Material Panel:** Lists materials sorted by property, filterable by source. Shows MaterialSignature as radar charts. Dual-select with blend ratio slider.

**Process Panel:** Available techniques from learned faction traditions. Drag to build sequence. Shows predicted property changes, cumulative quality, resource costs, warnings.

**Form Panel:** Sliders for reach/coverage/complexity (constrained by material). Visual silhouette preview. Predicted weight.

**Outcome Preview:** Full equipment interpreter pipeline on predicted output. Shows CrewStatBlock, predicted abilities, formation preferences, quality estimate, failure modes. Comparison overlay vs. current equipment. Range (best/expected/worst) based on tool wear uncertainty.

**Crafting Journal:** Automatically recorded. Searchable, filterable. Player can name creations and add notes. Highlights patterns: "You've made 5 high-toxicity items. Your best result used forge_hardening before venom_infusion."

---

## 5. Cross-System Integration

```
CRAFTING ↔ EVOLUTIONARY MODEL:
  K1 ← Layer 3 harvesting: creature channels → material signatures
  K1 → player behavior → Layer 4 player_activity selection pressure
  Layer 5 speciation → K1 novel materials

CRAFTING ↔ COMBAT SYSTEM:
  K3 → C2: EquipmentPiece fed to equipment interpreter
  C3 → K4: durability degradation from combat
  C3 → K1: material drops (corpse harvest)

CRAFTING ↔ FACTION SOCIAL MODEL:
  F5 faction identity → K2 technique libraries
  F3 relationship → K2 technique access
  K2 action → F3 opinion shift (using forbidden techniques)
  K2 → F5 cultural identity (faction aesthetic modifiers)
  K2 ↔ F4 knowledge diffusion (techniques are knowledge)

CRAFTING ↔ EXPLORATION SYSTEM:
  E4 discovery → K1 environmental deposits
  E4 → K1 salvage from ruins
  E4 → K2 facility access (ruins contain workshops)

CRAFTING ↔ ECONOMIC LAYER:
  K1 ↔ EC1: material inventory same representation
  K2 ↔ EC2: agents use same techniques
  EC4 → K2: settlement facilities enable advanced techniques
  EC6 ↔ K2: trade routes for technique knowledge

CRAFTING ↔ NPC DIALOGUE SYSTEM:
  Dialogue consequence → K2 technique unlock
  K2 technique → KnowledgeFact → F4 diffusion
  NPC opinions on techniques vary by faction alignment
```

---

## 6. Tradeoff Matrix

| Decision | Options | Fidelity | Implementability | Legibility | Emergent Power | Why |
|----------|---------|----------|------------------|-----------|-----------------|-----|
| **Recipe database vs. no recipes** | A) Finite recipe list | Low | Easy | High | Low | Caps design space |
| | B) Property transforms, no recipes | High | Medium | Medium | High | 17 properties × techniques × form params = effectively infinite |
| **Tool quality as skill** | A) Abstract crafting skill stat | Medium | Easy | High | Low | No feedback to material demand |
| | B) Tool material properties determine outcome quality | High | Medium | Medium | High | Bootstrap loop: hunt → craft tools → craft gear |
| **Material degradation** | A) Infinite freshness | Low | Easy | High | Low | Removes hunt urgency |
| | B) Freshness decay per storage state | High | Easy | Medium | High | Fresh materials are better; creates hunt rhythm |
| **Failure outcomes** | A) Crafting fails, materials consumed | Low | Easy | High | Low | Punishes experimentation |
| | B) Failure produces flawed artifact | High | Medium | Medium | High | Preserves investment, teaches through artifacts |
| **Faction traditions** | A) All techniques available to all players | Low | Easy | High | Low | No reason to engage multiple factions |
| | B) Factions teach technique subsets with bonuses | High | Medium | Low | High | Social progression drives crafting progression |
| **Form parameters** | A) Named weapon/armor types | Medium | Easy | High | Low | Violates project "no named types" principle |
| | B) Continuous form parameters | High | Medium | Medium | High | Lets equipment interpreter generate novel forms |
| **Outcome preview** | A) Approximate preview, full result at craft-time | Medium | Easy | Medium | Low | Player gambles, not engineers |
| | B) Full interpreter pipeline preview | High | Medium | Low | High | Player sees exact consequences before committing |

---

## 7. Emergent Properties

1. **Material novelty drives equipment novelty:** Evolutionary speciation produces creatures with novel channel profiles → novel material signatures → novel crafting possibilities → novel equipment. No designer recrafts recipe lists.

2. **Tool specialization bottleneck:** Different techniques demand different tool ideal-profiles. A crafter wants a high-hardness hammer, high-purity vessel, high-flexibility blade. These require hunting different creatures or trading. Specialized tool libraries become a meta-progression goal.

3. **Technique discovery chains:** Players discover that technique sequence A (technique 1 → technique 2) works better than sequence B (technique 2 → technique 1). They notice that byproduct of technique C is ideal reagent for technique D. These discoveries are player-driven, not designer-authored.

4. **Faction crafting identity:** Covenant crafters excel at thermal/mineral techniques; Meridian at experimental blends; Tidecallers at biological. A player's equipment aesthetic reflects faction alliances. Trading techniques across factions creates diplomatic tension (forbidden techniques being used).

5. **Tool wear as progression gating:** Worn tools produce lower-quality results. A crafter must periodically craft replacement tools, which requires hunting fresh creatures. This creates a natural pacing cycle: hunt → craft tools → craft gear → hunt again.

6. **Salvage as wild card:** Bright Horizon materials have extreme properties in unpredictable combinations. Crafting with salvage is high-variance—spectacular successes or failures. Players who understand synergy/conflict rules master salvage; novices avoid it or accept its risks.

7. **Form constraints drive material hunting:** A player wanting a long-reach weapon (reach=0.9) needs material with hardness + flexibility. This is a specific hunting goal: "Find creatures with both structural_rigidity and elastic_deformation." Evolution creates these niches.

---

## 8. Open Calibration Knobs

- **LEARNING_RATE:** How fast techniques improve skill.
- **COHERENCE_PENALTY:** How much property variance harms quality.
- **CHAIN_BONUS_WEIGHTS:** Technique sequence synergy bonus per sequence.
- **FORM_CONSTRAINT_SCALING:** How much exceeding limits penalizes quality.
- **TOOL_IDEAL_PROFILE_WEIGHTS:** How important each property is to each tool type.
- **TOOL_WEAR_EFFECTIVENESS_CURVE:** How durability degrades effectiveness (currently: lerp 0.5 to 1.0).
- **SYNERGY_SCALE, CONFLICT_SCALE:** Magnitude of material blend interactions.
- **FRESHNESS_DEGRADATION_RATE:** Per storage state.
- **BYPRODUCT_YIELD_RATIO:** What fraction of input becomes byproduct.
- **REAGENT_BLEND_RATIO:** How much secondary material is consumed per technique.
- **FACILITY_QUALITY_BONUS:** How much facility contributes to effectiveness.
- **FAILURE_MODE_THRESHOLD:** Per failure type.

---

## 9. Implementation Notes

**Performance:** Outcome preview runs equipment interpreter on predicted output at ~10Hz throttle. Processing resolution is O(techniques × properties) ≈ 5 × 17 = 85 operations per attempt—trivial.

**Modularity:** K1 outputs MaterialStacks. K2 transforms MaterialStack → MaterialStack. K3 sets form parameters. K4 computes quality/failure. K5 presents UI. Each layer is self-contained; extensions don't break others.

**Content pipeline:** New techniques are data files (YAML/JSON) with transform definitions. New synergy/conflict rules added to rule registry. New faction traditions assemble from existing technique IDs + aesthetic modifiers. System designed so content designer can add technique without touching code.

**Data export:** Crafting state serializes as material inventory (MaterialStack list), known techniques (TechniqueId set), tool loadout (ToolCategory → EquipmentPiece map), crafting journal, in-progress workbench state. Deterministic reproduction from saved inputs.

---

## References

- Ryzom Wiki. "Crafting." https://en.wiki.ryzom.com/wiki/Crafting
- Dwarf Fortress Wiki. "Material science." https://dwarffortresswiki.org/index.php/DF2014:Material_science
- Buruk, O. et al. "Crafting in Games." *Digital Humanities Quarterly*, 11(4), 2017.
- Engström, H. "5 Approaches to Crafting Systems in Games." *Envato Tuts+*, 2015.
- Lane, R. "Developers! Here's how to fix your stupid crafting system." *bit-tech*, 2018.
- Kleiber, M. (1932). "Body Size as a Factor in Metabolism." *Hilgardia*, 6(11), 315–353.
