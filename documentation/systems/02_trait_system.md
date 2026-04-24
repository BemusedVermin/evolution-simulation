# Trait System: Channel Registry & Phenotype Expression

## 1. Overview

This document specifies the **Trait System**, which governs how monsters accumulate, express, and evolve physiological channels. The system operates in three layers:

1. **Channel Registry & Manifest**: Channels are declared in **manifests**—JSON documents specifying family membership, range, mutation behavior, composition rules, and expression conditions. A runtime registry assembles manifests from core, mods, and genesis-derived channels into a single searchable namespace.

2. **TraitGene & Body-Site Expression**: Each gene in the genome carries one channel id and a body-site location. TraitGenes aggregate into variable-length trait vectors (not fixed 18-channel enums), evaluated in a genome iteration to produce per-body-region channel profiles.

3. **Phenotype Interpreter**: Translates channel profiles into behavioral parameters (speed, damage, sensing ranges), visual directives (mesh, colors, particles), and abilities. This is where evolution meets gameplay.

The key principle: **channels are evolved building blocks, not named traits.** Two monsters with identical global channel profiles behave identically, even if they evolved those channels through completely different paths. Channels can be duplicated (genesis), diverge (manifest mutation), and reclassify into different families. Emergence arises from composition hooks: multiple channels crossing thresholds jointly unlock new behaviors (e.g., auditory + vocal + spatial > T ⟹ echolocation).

---

## 2. Research Basis

**Modular Evolution & Gene Duplication (Wagner 2010; Ohno 1970)**
Complex phenotypes evolve through duplication and divergence of regulatory elements and coding sequences. A paralog (duplicate channel) inherits the parent's function but can accumulate mutations, eventually achieving a distinct niche. This is channel genesis: duplication produces an exact manifest copy; subsequent mutations drift interaction coefficients, bounds, and eventually family membership. By this mechanism, the channel set itself evolves.

**Channel Families as Functional Constraint Classes (Gerstein et al. 2007; True & Carroll 2002)**
Channels cluster into families because they share developmental origins, regulatory logic, and interaction patterns. Sensory channels respond to environmental stimuli; motor channels execute movement; metabolic channels govern energy allocation. Family membership supplies defaults: sensory channels default to long-range detection rules; motor channels to force-based composition. This reduces design space complexity and ensures biological coherence.

**Phenotypic Plasticity & Expression Conditions (West-Eberhard 2003; Schilthuizen 2012)**
The same channel can manifest as different phenotypes under different environmental conditions. A channel is present in the genome but silent (expression_conditions unmet) until the biome affords its expression. This models developmental plasticity: a creature "can" have bioluminescence, but it's only useful in dark biomes.

**Compositional Morphogenesis (Fontana & Buss 1994; Wagner 2010)**
Complex morphologies are more evolvable when built from reusable, composable parts. Channels are primitives; body regions are modules; composition hooks are rules for their combination. The same channel set, distributed across different body sites, produces morphologically distinct outcomes—no need for hand-authored species categories.

**Emergence from Threshold Interactions (Schilthuizen 2012; Newman & Comper 1990)**
Qualitatively new behaviors arise when multiple independent systems cross activation thresholds simultaneously. High vibration output + high vibration sensing + neural speed → echolocation emerges. This captures real biological cases: eyes alone are useless without visual processing; sensory organs and neural circuits must coevolve to be effective.

**Arms Race Dynamics & Sensory Coevolution (Dawkins & Krebs 1979; Schilthuizen 2008)**
Predators and prey continuously escalate counter-adaptations. Predators evolving keen vision (light_sensing + neural_speed) drive prey toward camouflage (light_absorption). But a prey species that evolves bioluminescence (light_emission) for communication may become hypervisible to those same predators—creating an arms-race cost/benefit tradeoff captured in composition hooks.

---

## 3. Channel Registry & Manifest Schema

### Channel Manifest Format

Every channel is declared in a **manifest**—a JSON document specifying its identity, family, expression rules, and evolutionary behavior. Manifests are the canonical definition; all interpreter rules, composition, and mutation kernels derive from manifest fields.

```json
{
  "id": "kinetic_force",
  "family": "motor",
  "description": "Capacity to generate impact force in muscle contractions",
  "range": [0.0, 1.0],
  "units": "normalized",
  
  "mutation_kernel": {
    "sigma": 0.08,
    "bounds_policy": "reflect",
    "genesis_weight": 0.5,
    "correlation_with": ["structural_rigidity", "metabolic_rate"]
  },
  
  "composition_hooks": [
    {
      "with": "structural_rigidity",
      "kind": "multiplicative",
      "coefficient": 1.5,
      "description": "Rigid bodies can channel force more efficiently",
      "emits": [
        {
          "primitive_id": "apply_bite_force",
          "parameter_mapping": {
            "force": "kinetic_force * structural_rigidity * 1.5"
          }
        }
      ]
    },
    {
      "with": "mass_density",
      "kind": "additive",
      "coefficient": 0.3,
      "description": "Heavy impacts are more powerful",
      "emits": [
        {
          "primitive_id": "apply_impact_force",
          "parameter_mapping": {
            "impact": "kinetic_force * 0.3 + mass_density * 0.3"
          }
        }
      ]
    },
    {
      "with": "neural_speed",
      "kind": "threshold",
      "threshold": 0.6,
      "coefficient": 2.0,
      "description": "Neural coordination unlocks precise strikes; damage *= 2 if both > T",
      "emits": [
        {
          "primitive_id": "apply_bite_force",
          "parameter_mapping": {
            "force": "kinetic_force * neural_speed * 2.0"
          }
        }
      ]
    }
  ],
  
  "expression_conditions": {
    "biome_flags": ["any"],
    "scale_band": [0.1, 1000.0],
    "developmental_stages": ["juvenile", "adult"],
    "dormant_if_unmet": true
  },
  
  "body_site_applicable": true,
  "provenance": "core",
  "generation_born": 0
}
```

**Numerical Representation Note**: All channel `range` values and mutation parameters (`sigma`, `genesis_weight`, `coefficient`) in manifests are stored as **fixed-point Q32.32** in simulation state. The JSON schema shows decimal floats for readability, but internal storage uses 64-bit integers (Q32.32 format for [0, 1] quantities, i32 for counts). This ensures deterministic bit-identical replay across platforms. Serialization converts Q32.32 to/from JSON decimal representation; deserialization reconstructs exact fixed-point values.

**Manifest Fields:**

| Field | Type | Purpose |
|-------|------|---------|
| `id` | string (snake_case) | Stable identifier, unique within registry |
| `family` | enum: {sensory, motor, metabolic, structural, regulatory, social, cognitive, reproductive, developmental} | Family determines default interaction rules, mutation kernel, and composition defaults |
| `description` | string | For designers and UI |
| `range` | [float, float] | Min and max values; [0,1] is typical but not required |
| `units` | string | Semantic label (normalized, percentage, frequency, force, etc.) |
| `mutation_kernel.sigma` | float | Gaussian mutation standard deviation |
| `mutation_kernel.bounds_policy` | enum: {reflect, clamp, wrap} | Behavior when mutation exceeds range |
| `mutation_kernel.genesis_weight` | float [0,1] | When a paralog is created via duplication, start with sigma * genesis_weight (allows drift from parent). 0.5 = moderate drift allowed; 0.9 = stay close to parent. |
| `mutation_kernel.correlation_with` | list[string] | Other channel ids with which this channel's mutations are correlated (linked inheritance during evolution). Used by Evolution system (01). |
| `composition_hooks` | list[Hook] | Rules for how this channel combines with others |
| `composition_hooks[].with` | string | Target channel id |
| `composition_hooks[].kind` | enum: {additive, multiplicative, threshold, gating, antagonistic} | Interaction type. Additive: effect_A + effect_B. Multiplicative: effect_A * effect_B. Threshold: both must exceed T. Gating: A enables/disables B. Antagonistic: A and B suppress each other. |
| `composition_hooks[].coefficient` | float | Scaling factor or threshold value (context-dependent) |
| `composition_hooks[].threshold` | float (optional) | For threshold/gating kinds; channel values must exceed this to activate the interaction |
| `composition_hooks[].emits` | list[PrimitiveEmission] | **REQUIRED**: Which primitives fire when this hook triggers. Each emission specifies primitive_id and parameter_mapping expressions. |
| `composition_hooks[].emits[].primitive_id` | string | ID of primitive in primitive_registry that this hook emits |
| `composition_hooks[].emits[].parameter_mapping` | object | Map of (primitive_parameter_name → channel expression). Expressions use channel IDs and math operators. |
| `expression_conditions` | object | Predicates controlling when channel is expressed (active). |
| `expression_conditions.biome_flags` | list[string] | List of biome tags (e.g., "dark", "aquatic", "cave", "volcanic"); ["any"] means always expressible. |
| `scale_band` | object `{ min_kg, max_kg }` (top-level) | Applicable body-mass range. Authoritative, top-level field — not nested under `expression_conditions`. Allows pathogens (grams) and macro-beasts (tons) to share schema. |
| `expression_conditions.developmental_stages` | list[string] | Stages in which channel can be expressed: ["larval", "juvenile", "adult", "geriatric"] or ["any"] |
| `expression_conditions.dormant_if_unmet` | bool | If true, channel is present in genome but genetically silent until conditions are met; if false, channel causes developmental mismatch penalty (rare). |
| `body_site_applicable` | bool | Can this channel vary per body region (limbs, torso, head, tail, etc.)? If false, channel is always global. |
| `provenance` | enum: {core, mod:{mod_id}, genesis:{parent_id}:{generation}} | "core" = shipped with game; "mod:X" = from mod X; "genesis:..." = created by gene duplication from parent channel at generation N. |
| `generation_born` | int | Absolute generation when this channel was first created (for genesis channels, the duplication generation). |

### Channel Families & Defaults

Nine families form the backbone of channel organization. Each family carries default mutation kernels, composition patterns, and expression rules, reducing redundancy in manifest authorship.

#### **Family: Sensory** (input from environment)
Channels detect environmental stimuli (light, sound, chemicals, vibration). All sensory channels default to range-based detection, typically beneficial in specific biomes. Composition defaults: additive (more sensing = better detection).

**Core channels:** light_sensing, chemical_sensing, vibration_sensing, thermal_sensing, radiation_sensing
**Family Defaults:**
- Default mutation sigma: 0.10
- Default composition: additive (multiple senses are cumulative)
- Default expression: biome-dependent (light_sensing dormant in caves unless biome_flags="any")
- Body-site applicable: yes (e.g., eyes on head only)

#### **Family: Motor** (output of force/movement)
Channels generate movement and force application. Composition defaults: multiplicative with structural channels (force is channeled by rigidity), threshold with neural channels (coordination required for precision).

**Core channels:** kinetic_force, elastic_deformation, surface_friction, propulsion_method
**Family Defaults:**
- Default mutation sigma: 0.09
- Default composition: multiplicative with structural (rigidity enables force), threshold with cognitive (precision requires neural coordination)
- Default expression: scale and biome flexible
- Body-site applicable: yes (limbs, tail, torso)

#### **Family: Metabolic** (energy allocation & efficiency)
Channels govern energy budget: how much fuel consumed, how fast ATP is regenerated, heat efficiency. High metabolic channels increase action speed; low metabolic means slow but efficient (starvation-resistant).

**Core channels:** metabolic_rate, energy_recovery, thermal_efficiency, glucose_uptake
**Family Defaults:**
- Default mutation sigma: 0.07
- Default composition: antagonistic (high metabolic + high efficiency is unlikely; one up, one down is typical)
- Default expression: always (scale-band dependent; micro-scale pathogens need higher rates)
- Body-site applicable: no (global, whole-organism property)

#### **Family: Structural** (physical support & integrity)
Channels define body rigidity, density, and material properties. Rigid creatures are heavy and durable; elastic creatures are light and flexible. Composition defaults: multiplicative with motor (rigid channels force more effectively).

**Core channels:** structural_rigidity, mass_density, osmotic_regulation, exoskeleton_thickness
**Family Defaults:**
- Default mutation sigma: 0.08
- Default composition: multiplicative with motor (rigidity enables force transmission)
- Default expression: scale-band restricted (very light creatures are scale-appropriate only)
- Body-site applicable: yes (shell on back, soft belly)

#### **Family: Regulatory** (homeostasis & feedback control)
Channels govern immune response, temperature regulation, pH buffering, disease resistance. These are "meta" channels: they improve survival in harsh conditions but consume metabolic energy.

**Core channels:** immune_response, thermal_regulation, toxin_resistance, wound_healing
**Family Defaults:**
- Default mutation sigma: 0.06 (regulatory changes are slow; homeostasis is delicate)
- Default composition: additive (multiple regulatory systems help independently)
- Default expression: always (universal benefit)
- Body-site applicable: varies per channel (immune distributed; wound_healing localized)

#### **Family: Social** (communication & cooperation)
Channels enable signaling, pheromone production, alarm calls, dominance displays. Mostly multiplicative composition with cognitive (social behavior requires neural processing). Often dormant in solitary species.

**Core channels:** chemical_production, visual_signaling, vocal_ability, dominance_display
**Family Defaults:**
- Default mutation sigma: 0.11 (social channels are subject to runaway sexual selection, higher mutation)
- Default composition: multiplicative with cognitive (signals are more effective if receiver's neural speed is high)
- Default expression: conditional on group_living biome flag
- Body-site applicable: yes (colors on face/back; glands distributed)

#### **Family: Cognitive** (neural processing & learning)
Channels determine reaction speed, memory, decision sophistication, sapience. Higher cognitive channels gate NPC social behaviors and unlock learning from experience.

**Core channels:** neural_speed, pattern_recognition, long_term_memory, learning_rate
**Family Defaults:**
- Default mutation sigma: 0.05 (large neural mutations are often deleterious)
- Default composition: gating with social (neural_speed gates social behavior expression)
- Default expression: scale-band dependent (sapient behavior costs energy; only viable for large bodies)
- Body-site applicable: no (localized to brain)

#### **Family: Reproductive** (mating, fertility, sexual selection)
Channels govern fertility rate, mate-choice discrimination, parental investment, sexual dimorphism. High mutation in this family due to sexual selection runaway.

**Core channels:** fertility_rate, mating_preference_variance, gamete_quality, parental_investment
**Family Defaults:**
- Default mutation sigma: 0.13 (sexual selection driving rapid divergence)
- Default composition: threshold with condition (mating requires reaching sexual maturity, expression_stage = adult)
- Default expression: adult-only, biome-flexible
- Body-site applicable: no (reproductive traits are whole-organism)

#### **Family: Developmental** (growth rates, timing, heterochrony)
Channels alter developmental pace, allometry (relative growth rates), and life-stage transitions. Changes here produce dramatic morphological variation from small genotypic changes (heterochrony).

**Core channels:** growth_rate, developmental_heterochrony, allometric_head_size, allometric_limb_length
**Family Defaults:**
- Default mutation sigma: 0.09 (developmental timing is buffered but can shift)
- Default composition: multiplicative with structural (how fast you grow interacts with how dense you become)
- Default expression: all developmental stages (active during growth)
- Body-site applicable: yes (some body regions grow faster than others)


### Scale-Band Expression Conventions

Channels are expressed conditionally based on creature body mass. The scale_band field in expression_conditions gates which channels are active at a creature's current mass/scale.

**Conventions for Common Channel Scale Ranges:**

#### Macro-Only Channels (expressible at [1.0 kg, ∞] only)

These channels apply to large creatures and are dormant at micro scales. Example:

```json
{
  "id": "large_neural_integration",
  "family": "cognitive",
  "description": "Complex spatial cognition and learned behaviors. Viable only in creatures large enough to support large neural tissue.",
  "range": [0.0, 1.0],
  "expression_conditions": {
    "scale_band": [1.0, 1000000.0],
    "biome_flags": ["any"],
    "developmental_stages": ["juvenile", "adult"],
    "dormant_if_unmet": true
  }
}
```

**Rationale**: Creatures < 1 kg cannot support the metabolic burden of complex cognition. This channel dormant in all pathogens and parasites.

#### Micro-Only Channels (expressible at [1e-15 kg, 1e-3 kg] only)

These channels are expressed only in micro-scale organisms: pathogens, parasites, symbionts, and single-celled pathogens under ~1 gram.

Example:

```json
{
  "id": "host_attachment",
  "family": "structural",
  "description": "Strength of adhesion to host cell or tissue. Core channel for parasitic bonding. Dormant in macro creatures.",
  "range": [0.0, 1.0],
  "expression_conditions": {
    "scale_band": [1e-15, 1e-3],
    "biome_flags": ["any"],
    "developmental_stages": ["larval", "juvenile", "adult"],
    "dormant_if_unmet": true
  }
}
```

**Rationale**: Host attachment requires micro-scale surface interactions. Macro creatures cannot express parasitic bonding at the cellular level.

Another example:

```json
{
  "id": "cell_surface_antigen",
  "family": "structural",
  "description": "Antigenic proteins on pathogen surface. Determines immune recognition. Expressible only in micro-scale pathogens.",
  "range": [0.0, 1.0],
  "expression_conditions": {
    "scale_band": [1e-15, 1e-3],
    "biome_flags": ["any"],
    "developmental_stages": ["any"],
    "dormant_if_unmet": true
  }
}
```

#### Channels Expressible at All Scales (no scale_band constraint)

These channels are expressed across all size ranges, from micro-pathogens to macro-creatures:

```json
{
  "id": "metabolic_rate",
  "family": "metabolic",
  "description": "Energy consumption rate. Expressible at all scales; values calibrated per scale via mutation kernel bounds.",
  "range": [0.0, 1.0],
  "expression_conditions": {
    "biome_flags": ["any"],
    "developmental_stages": ["any"],
    "dormant_if_unmet": false
  },
  "body_site_applicable": false
}
```

**Note**: No `scale_band` field means the channel is viable at all masses. Metabolic_rate, immune_response_baseline, and regulatory channels typically have no scale_band constraint.

Another example:

```json
{
  "id": "immune_response_baseline",
  "family": "regulatory",
  "description": "Recognition threshold and response speed for immune activation. Expressible in both macro hosts and as regulatory output in micro pathogens.",
  "range": [0.0, 1.0],
  "expression_conditions": {
    "biome_flags": ["any"],
    "developmental_stages": ["any"],
    "dormant_if_unmet": false
  },
  "body_site_applicable": false
}
```

**Why this matters**: Host-coupling channels are ONLY expressible at micro scale (pathogens interpret them; macro creatures do not). The Phenotype Interpreter (System 11) applies scale-band filtering: any channel whose scale_band does not contain the creature's current body_mass_kg is zeroed out before stat computation and composition hook evaluation.

---
### Registry Architecture: Dual Registries (Channels & Primitives)

The system maintains **two parallel registries**, both following the same architectural pattern:

1. **Channel Registry**: Manifests for trait channels (sensory, motor, metabolic, etc.). Read by Evolution system (01) and Phenotype Interpreter (11).
2. **Primitive Registry**: Manifests for atomic effects (emit_acoustic_pulse, apply_bite_force, etc.). Read by Phenotype Interpreter (11) and Chronicler (09).

Both registries:
- Are manifest-declared (JSON documents specify all behavior)
- Are runtime-extensible (mods, genesis channels/primitives, and dynamic registration supported)
- Are snapshot-serialized (save states capture registry state for deterministic replay)
- Are deterministic (seeded, no randomness in registry structure itself)

### Channel Registry

The runtime **Channel Registry** is a searchable namespace assembling manifests from three sources:

1. **Core Channels**: ~17–20 channels shipped with the game, covering the 9 families. Listed in `data/channels/core.manifest`.
2. **Mod Channels**: Mods can extend the registry by contributing manifests. Each mod's channels are prefixed `mod:{mod_id}:channel_name` in the registry at runtime.
3. **Genesis Channels**: When a channel is duplicated (see Channel Genesis, below), the new paralog is added to the registry with provenance `genesis:{parent_id}:{generation}`.

**Registry API (pseudo-code):**

```
class ChannelRegistry:
    manifests: Dict[channel_id, ChannelManifest]
    families: Dict[family_name, List[channel_id]]  // index for family-level queries
    
    def register(manifest: ChannelManifest) -> None:
        // Called at startup for core + mod channels, and at runtime for genesis channels
        self.manifests[manifest.id] = manifest
        self.families[manifest.family].append(manifest.id)
    
    def get(channel_id: str) -> ChannelManifest:
        return self.manifests[channel_id] or raise UnknownChannel
    
    def get_family(family: str) -> List[ChannelManifest]:
        return [self.manifests[id] for id in self.families[family]]
    
    def all() -> List[ChannelManifest]:
        return list(self.manifests.values())
    
    def dump_for_evolution() -> Dict:
        // Export to Evolution system; used to initialize mutation kernels
        return {id: m.mutation_kernel for id, m in self.manifests.items()}
    
    def dump_for_interpreter() -> Dict:
        // Export to Phenotype Interpreter; used to render rules
        return {id: {composition_hooks, expression_conditions, range} for id, m in self.manifests.items()}
```

---

## 3B. The Primitive Registry

### Purpose

Primitives are the **atomic vocabulary of phenotype output**. Rather than declaring "ability: poison_bite", the Phenotype Interpreter emits primitive effects like `apply_bite_force(force=0.8)` and `emit_toxin(potency=0.5)`. This separation enables:

- **Composability**: Multiple channels can emit the same primitive, parameterized differently (e.g., kinetic_force emits apply_bite_force; elastic_deformation emits apply_impact_force; same primitive, different sources)
- **Emergence**: New abilities arise from combinations of primitives (fire + impact → ignite), not explicit ability definitions
- **Evolution**: Primitives diverge and reclassify like channels (future work, Section 4C of System 01)

### Primitive Manifest Shape

Each primitive declares its identity, functional category, parameters, composition compatibility, cost, and observable signature:

```json
{
  "id": "emit_acoustic_pulse",
  "category": "signal_emission",
  "description": "Emit a short-duration acoustic wave",
  
  "parameter_schema": {
    "frequency_hz": {
      "type": "float",
      "range": [100, 40000],
      "default": 5000,
      "description": "Pitch of the pulse"
    },
    "amplitude": {
      "type": "float",
      "range": [0.0, 1.0],
      "default": 0.5,
      "description": "Volume (0=silent, 1=max)"
    },
    "duration_ms": {
      "type": "float",
      "range": [1, 1000],
      "default": 100,
      "description": "Pulse length in milliseconds"
    }
  },
  
  "composition_compatibility": {
    "channel_families": ["motor", "cognitive"],
    "channel_ids": ["vibration_output", "neural_speed"],
    "description": "Acoustic pulse is more effective with motor control and neural coordination"
  },
  
  "cost_function": {
    "metabolic_cost": 0.05,
    "cooldown_ticks": 5,
    "description": "5% of metabolic budget; 5-tick recharge"
  },
  
  "observable_signature": {
    "modality": "vibration",
    "range": 50.0,
    "detectability": 0.8,
    "description": "Creatures can hear this pulse up to 50 units away; 80% detectability"
  },
  
  "provenance": "core",
  "generation_born": 0
}
```

**Key Fields:**

| Field | Type | Purpose |
|-------|------|---------|
| `id` | string | Unique identifier (e.g., "emit_acoustic_pulse", "genesis:emit_acoustic_pulse:1500") |
| `category` | enum | Functional category (8 types; see below) |
| `description` | string | UI/designer documentation |
| `parameter_schema` | object | JSON Schema for parameters (frequency, amplitude, duration, etc.). Channels parameterize primitives at invocation. |
| `composition_compatibility.channel_families` | list[string] | Families (sensory, motor, etc.) that enhance or are required for this primitive |
| `composition_compatibility.channel_ids` | list[string] | Specific channel ids that interact with this primitive |
| `cost_function.metabolic_cost` | float | Metabolic budget consumed per invocation (0..1) |
| `cost_function.cooldown_ticks` | int | Recharge time before primitive can be emitted again |
| `observable_signature.modality` | string | Sense channel ("vibration", "light", "chemical", "thermal", etc.) |
| `observable_signature.range` | float | Detection range (game units) |
| `observable_signature.detectability` | float [0,1] | Strength of signature (0=invisible, 1=unmissable) |
| `provenance` | enum | "core" = shipped; "mod:X" = from mod; "genesis:parent:generation" = evolved paralog |
| `generation_born` | int | Absolute generation of creation (0 for core) |

### Eight Primitive Categories

Primitives cluster into 8 functional categories, representing atomic phenotype vocabulary:

**1. Signal Emission** — Broadcast information outward
- Examples: emit_acoustic_pulse, emit_bioluminescent_flash, emit_pheromone_cloud, emit_thermal_radiation
- Driven by: vibration_output, light_emission, chemical_production, thermal_output channels
- Typical cost: moderate metabolic; scaled by emission intensity
- Observable signature: modality-specific (sound, light, smell, heat)

**2. Signal Reception** — Detect and transduce sensory information
- Examples: photoreception, phonoreception, chemoreception
- Driven by: light_sensing, vibration_sensing, chemical_sensing, thermal_sensing channels
- Typical cost: low (passive sensing)
- Observable signature: low (detection acts are internal)

**3. Force Application** — Exert mechanical work
- Examples: apply_bite_force, apply_impact_force, apply_injection_force, apply_grip_force
- Driven by: kinetic_force, structural_rigidity channels
- Typical cost: moderate to high (depends on force magnitude and target material)
- Observable signature: mechanical (vibration, impact sound)

**4. State Induction** — Alter internal or external behavioral state
- Examples: enter_predatory_mode, initiate_courtship, trigger_freeze_response, activate_hibernation
- Driven by: neural_speed, regulatory channels (immune_response, thermal_regulation)
- Typical cost: low to moderate (state change is metabolic; maintenance cost depends on state)
- Observable signature: behavioral (observable in creature actions)

**5. Spatial Integration** — Coordinate multi-body-part or multi-sensory actions
- Examples: echolocation_sweep, synchronized_pack_hunting, precise_aiming, formation_control
- Driven by: neural_speed, metabolic_rate, kinetic_force
- Typical cost: moderate (requires active neural coordination)
- Observable signature: spatial (visible in body positioning, movement synchronization)

**6. Mass Transfer** — Move matter (nutrients, gametes, pathogens, building materials)
- Examples: nutrient_absorption, spore_dispersal, larval_transport, parasite_transmission, web_production
- Driven by: metabolic_rate, reproductive_rate, chemical_production, structural_rigidity
- Typical cost: metabolic (depends on mass and distance moved)
- Observable signature: chemical/biological (spores, pheromones, visible constructs)

**7. Energy Modulation** — Store, release, or redirect energy
- Examples: enter_torpor, adrenaline_surge, temperature_regulation_feedback, bioluminescent_burst
- Driven by: metabolic_rate, thermal_output, thermal_regulation, light_emission
- Typical cost: variable (charge-up phase may be free; release phase is high cost)
- Observable signature: thermal, visual (glow, heat wave)

**8. Bond Formation** — Create temporary or permanent links between entities
- Examples: mate_pair_bonding_marker, symbiotic_immune_tolerance, web_anchoring, nest_construction
- Driven by: social channels (chemical_production, vocal_ability), reproductive_rate, structural_rigidity
- Typical cost: one-time or amortized (bonding has initial cost; maintenance varies)
- Observable signature: chemical (pheromones), visual (structures)

### How Channel Composition Hooks Reference Primitives

When a channel composition hook crosses a threshold, it emits one or more primitives, with parameters mapped from channel values. Example:

```json
Channel manifest (kinetic_force):
{
  "composition_hooks": [
    {
      "with": "structural_rigidity",
      "kind": "multiplicative",
      "coefficient": 1.5,
      "emits": [
        {
          "id": "apply_bite_force",
          "parameters": {
            "force": "kinetic_force * structural_rigidity * 1.5"
          }
        }
      ]
    },
    {
      "with": "neural_speed",
      "kind": "threshold",
      "threshold": 0.6,
      "emits": [
        {
          "id": "apply_bite_force",
          "parameters": {
            "force": "kinetic_force * neural_speed * 2.0"
          }
        }
      ]
    }
  ]
}
```

The `emits` field is **REQUIRED on all composition_hooks going forward**. It lists primitives and their parameter-mapping expressions. Parameter expressions use channel ids and math operators to compute runtime values.

**Migration Path for Existing Hooks (Without emits):**

Composition hooks in existing manifests that lack the `emits` field must be migrated before the next major version. Two strategies:

1. **Explicit mapping** (preferred): Designer analyzes the hook's kind and coefficient, infers which primitive(s) should fire, and populates `emits` with explicit parameter mappings.
   
   Example migration:
   ```json
   // OLD: multiplicative hook with no emits
   {
     "with": "structural_rigidity",
     "kind": "multiplicative",
     "coefficient": 1.5
   }
   
   // NEW: explicit primitive emission
   {
     "with": "structural_rigidity",
     "kind": "multiplicative",
     "coefficient": 1.5,
     "emits": [
       {
         "primitive_id": "apply_bite_force",
         "parameter_mapping": {
           "force": "kinetic_force * structural_rigidity * 1.5"
         }
       }
     ]
   }
   ```

2. **Fallback migration** (temporary): If `emits` is missing, the interpreter emits a generic aura primitive (backward compatibility). This is logged as a warning for designers to fix. Future versions will error on missing `emits`.

### Validation: Primitive Registry Linkage

**Invariant 1**: All primitives referenced in composition hooks (`emits[].primitive_id`) must exist in the Primitive Registry at interpreter evaluation time.

**Invariant 2**: Mods must register primitives before registering channel manifests that reference them.

**Invariant 3**: All composition_hooks MUST have a non-null `emits` field. No hook fires without declaring which primitives it emits.

**Validation at Startup**:
```
for each channel_manifest in channel_registry:
  for each composition_hook in channel_manifest.composition_hooks:
    if composition_hook.emits is not null:
      for each primitive_ref in composition_hook.emits:
        assert primitive_registry.get(primitive_ref.id) exists
        assert parameter_mappings are valid (all channel ids referenced exist in channel_registry)
```

### Determinism: Primitive Genesis & Save States

Primitive genesis (duplication, divergence, reclassification, loss) follows channel genesis (System 01, Section 2B) when implemented. Until then, the primitive registry is static (core + mods only). When primitive evolution is enabled:

- Duplication and mutation operators are seeded like channel genesis (deterministic)
- Primitive registry snapshots are serialized in save states alongside channel registry snapshots
- Registry grows over gameplay (new paralog manifests created by duplication), but growth is deterministic
- Primitive-to-channel parameter mappings are evaluated deterministically at interpretation time

---

## 4. Entities & State

### TraitGene & Genome Structure

A **TraitGene** is a variable-length record carrying one channel id and its body-site placement. The genome is no longer a fixed 18-element vector; it is a variable-length list of TraitGenes, supporting duplication, loss, and genesis.

```
TraitGene {
    id: string                          // Channel id (e.g., "kinetic_force", "genesis:kinetic_force:42")
    locus: int                          // Position in genome (for recombination)
    body_site: {
        region: enum {HEAD, TORSO, LIMBS_F, LIMBS_B, TAIL, FINS, WINGS, SPINES}
        bilateral_symmetry: bool        // if true, expressed on left and right; if false, unique placement
        surface_position: float [0,1]   // front (0) to back (1) along body axis
        depth_from_surface: float [0,1] // superficial (0) to internal (1)
    }
    expression_level: float [0,1]       // Dominance allele (if diploid). Always 1.0 for haploid.
    age_at_expression: int              // Developmental stage when this gene activates (generation units)
    mutation_history: {
        parent_channel_id: string       // For genesis channels, the parent id
        generation_created: int         // Absolute generation of creation/duplication event
        mutations_accumulated: int      // Count of non-genesis point mutations since creation
    }
}

Genome {
    genes: List[TraitGene]              // Variable-length; can grow via duplication, shrink via deletion
    ploidy: enum {haploid, diploid}     // Determines allele interpretation
    total_allele_count: int             // Sum of distinct channel ids across all genes
}
```

**TraitGene Fields:**

| Field | Purpose |
|-------|---------|
| `id` | Channel identifier from registry. Stable across evolution; a channel's identity is its id, not its location. |
| `locus` | Position in genome, used for recombination during sexual reproduction. Loci are assigned sequentially; duplication shifts loci downstream. |
| `body_site.region` | Which part of the body this gene is expressed in. Can be spatial (LIMBS_F, LIMBS_B) or unique (HEAD). Allows the same channel to appear in multiple body regions with different phenotypic effects. |
| `body_site.bilateral_symmetry` | If true, gene is expressed on both left and right symmetrically. If false, it's a unique left-side or right-side placement (creates morphological asymmetry). |
| `body_site.surface_position` | Front-to-back gradient (0 = head, 1 = tail). Allows fine-grained spatial variation. |
| `body_site.depth_from_surface` | Superficial (skin, visible) to internal (organ, hidden). Affects procgen visuals. |
| `expression_level` | Allele dominance. In diploid genomes, one allele is dominant (1.0), the other recessive (0.0–0.9). In haploid, always 1.0. Used by Evolution system during reproduction. |
| `age_at_expression` | When in development this gene "turns on." Allows temporal patterning: some traits active only in larvae, others only in adults. |
| `mutation_history.parent_channel_id` | For genesis channels, the channel id from which this paralog was duplicated. None for core or original channels. |
| `mutation_history.generation_created` | When was this channel (or this particular gene) created? For core channels, 0. For genesis channels, the duplication generation. |
| `mutation_history.mutations_accumulated` | Post-genesis point mutations; used to score divergence (see Channel Genesis, below). |

**Interpretation: Per-Body-Region Channel Profile**

From the genome, the Phenotype Interpreter aggregates channels per body region:

```
per_body_region_profile[LIMBS_F] = {
    kinetic_force: 0.72,      // Sum of expression_level across all kinetic_force genes in LIMBS_F
    elastic_deformation: 0.35,
    surface_friction: 0.61,
    ... all channels with body_site.region == LIMBS_F ...
}

global_channel_profile = {
    kinetic_force: 0.68,      // Average across all body regions (or weighted by surface area)
    metabolic_rate: 0.44,     // Channels not applicable to body_site are global (metabolic_rate.body_site_applicable=false)
    ...
}
```

**Key Insight**: Same channel, different body sites → same global behavioral effect, different visual manifestation. A creature with high kinetic_force in LIMBS_F and high kinetic_force in TAIL both have "strong bite," but one shows as powerful claws and the other as a whipping tail.

### Channel Genesis: Duplication & Divergence

**Channel genesis is a first-class evolutionary operator**—channels don't just accumulate variation; they can be duplicated, diverge, and eventually reclassify into new families. This is how the channel set itself evolves.

#### Duplication Event

When a gene undergoes a duplication event (modeled by Evolution system 01):

1. A new TraitGene is created with:
   - `id`: New unique id, `genesis:{parent_id}:{generation}`
   - `body_site`: Identical to parent (same region, position)
   - `mutation_history.parent_channel_id`: The parent's id
   - `mutation_history.generation_created`: Current generation
   - `mutation_history.mutations_accumulated`: 0
2. The manifest for this new channel is created by copying the parent's manifest and setting:
   - `provenance`: `genesis:{parent_id}:{generation}`
   - `generation_born`: Current generation
   - `mutation_kernel.sigma`: Scaled by `parent_manifest.mutation_kernel.genesis_weight`, allowing controlled drift from parent
3. The new channel is registered and becomes evolvable immediately.

#### Divergence Scoring

Over generations, the paralog accumulates mutations (point mutations on manifest fields: range, coefficients, correlations). Divergence is scored by:

```
divergence_score = sum(
    (mutated_field - parent_field) / (parent_field_range)
    for field in [range, composition_coefficients, correlation_weights]
)
```

When `divergence_score > THRESHOLD_SPECIATION` (e.g., 3.0), the paralog is considered "speciated":
- May be reclassified into a different family (if correlation structure has drifted sufficiently)
- Designers notified (for lore/naming)
- Treated as independent channel thereafter (reclassification recorded in manifest)

#### Family Reclassification

If a paralog's manifest drifts such that its interaction patterns no longer match its original family, it can reclassify:

```
Example: genesis:kinetic_force:42 (from motor family) mutates such that:
  - Its composition_hooks now primarily interact with sensory channels (not structural)
  - Its correlation_with list shifts to sensory channels
  - Its range compresses to [0, 0.3] (weak force expression)

Detection: compute_family_affinity(manifest) compares it to all family defaults.
If affinity to 'sensory' > affinity to 'motor', reclassify family: motor → sensory.

Side Effect: default mutation kernels, composition rules, and expression conditions update to sensory defaults.
```

Reclassification is rare (requires sustained selection for divergence over many generations) but creates narratively significant new channels.

#### Genesis Examples

**Example 1: Color Vision Specialization**
- Parent: light_sensing (sensory family, detects all wavelengths equally)
- Duplication at generation 1200: genesis:light_sensing:1200
- Mutations drift range to high sensitivity in red wavelengths only
- Eventually reclassifies to "red_vision" (sensory, but with narrower applicability)
- In high-predation red-colored biome, red_vision becomes selected; population diverges

**Example 2: Venom-Specific Toxin**
- Parent: chemical_output (social family, pheromone production)
- Duplication: genesis:chemical_output:850
- Mutations increase toxicity and decrease volatility (specialized for injection, not dispersal)
- Composition hooks shift: adds threshold interaction with kinetic_force (venom effective only when injected via bite)
- Reclassifies to motor-adjacent channel (now called "venom_injection")
- In predator arms race, venom becomes a major adaptive strategy

### Interpreted Phenotype

```
InterpretedPhenotype {
    // Movement & Body Plan
    base_speed: float                   // move distance per tick
    mass_total: float                   // kg equivalent
    max_health: float                   // HP
    
    // Combat Stats
    melee_attack_power: float           // damage per hit
    ranged_attack_power: float          // if applicable
    armor_value: float                  // damage reduction
    dodge_chance: float                 // probability to evade
    crit_chance: float                  // critical hit probability
    
    // Special Abilities
    abilities: list<Ability>            // derived from channel profiles
    
    // Sensory Ranges
    vision_range: float                 // distance can see
    hearing_range: float                // sound detection range
    smell_range: float                  // chemical sensing range
    tremor_sensitivity: float           // ground vibration detection
    
    // Regeneration & Survival
    healing_per_tick: float             // HP restored per game tick
    poison_resistance: float            // % damage reduction to toxins
    thermal_resistance: float           // tolerance to extreme temps
    
    // Social / Sapience
    sapience_level: float               // 0..1, gates full NPC social layer
    
    // Procgen Metadata
    body_morphology: MorphologyProfile  // visual parameters for mesh generation
    color_palette: ColorProfile         // RGB choices for shaders
    animation_params: AnimationProfile  // speed, weight, grace of motion
}

Ability {
    id: string                          // "poison_spit", "charge", "echolocation"
    trigger_type: enum { OnCommand, OnContact, OnDamage, Passive, Periodic }
    execution_type: enum { Melee, Ranged, Aura, SelfBuff, SelfHeal }
    power: float                        // effectiveness scaling
    cooldown_ticks: int                 // recharge time
    range: float                        // spatial extent
    description: string                 // for player UI (procedurally generated)
}

MorphologyProfile {
    body_length: float                  // 0.5–4.0 × base size
    limb_thickness: float               // proportion of limb to body
    head_size: float                    // relative to body
    spine_prominence: float             // how "spiky" (0..1)
    smoothness: float                   // curvature vs. angularity (0..1)
    bilateral_ratio: float              // left-right symmetry (0.5..1.0)
    
    // Per-region modifications
    region_overrides: Map<BodyRegion, RegionMorphology>
}

ColorProfile {
    hue_base: float                     // 0..360 degrees
    saturation: float                   // 0..1
    brightness: float                   // 0..1
    region_color_shifts: Map<BodyRegion, float>  // hue shift per region
    glow_color: optional<RGBColor>      // if light_emission > 0.3
    camouflage_pattern: enum { Solid, Stripes, Spots, Gradient }
}

AnimationProfile {
    movement_style: enum { Slither, Gallop, Bound, Crawl, Fly, Glide }
    attack_speed_multiplier: float      // how quick are animations
    idle_restlessness: float            // fidgeting, pacing intensity
    pain_response: enum { Thrash, Curl, Freeze, Dodge }
}
```

---

### Interpreter Rule System

The interpreter is a collection of **declarative rules** mapping channel profiles to behavioral parameters. Each rule operates independently; results compose additively or multiplicatively as specified.

#### Movement & Mobility

```
// Rules are declared per channel manifest; interpreter looks up composition_hooks at runtime.
// Manifest: elastic_deformation has composition_hook(kind=multiplicative, with=structural_rigidity, coeff=1.5)
// So: elastic deformation + rigidity interact multiplicatively.

// Base speed calculation
base_speed = BASE_MOVEMENT_SPEED
    * (1 + global_channels["elastic_deformation"] * 0.5)
    * (1 - global_channels["structural_rigidity"] * 0.3)
    * (1 + global_channels["metabolic_rate"] * 0.4)
    / (1 + global_channels["mass_density"] * 0.5)

// Total mass (affects momentum, falling, knockback)
mass_total = BASE_MASS * (1 + global_channels["mass_density"] * 2.0)

// Movement Style (animation choice)
// Channels are looked up by id from the registry; 'elastic_deformation' may be:
//  - core: "elastic_deformation"
//  - from mod: "mod:arachnid_pack:silk_elasticity"
//  - genesis: "genesis:elastic_deformation:1156"
// Interpreter handles all identically by looking up composition_hooks from manifest.

if global_channels["elastic_deformation"] > 0.7:
    movement_style = SLITHER or BOUND  // flexible creatures move fluidly
elif global_channels["structural_rigidity"] > 0.7:
    movement_style = GALLOP or CRAWL   // rigid creatures move stiffly
elif global_channels["kinetic_force"] > 0.6:
    movement_style = CHARGE            // aggressive creatures dash

// Climbing & vertical movement
// body_site_applicable: true channels have per-region profiles
if body_map[LIMBS]["surface_friction"] > 0.6:
    climbing_speed = base_speed * body_map[LIMBS]["surface_friction"]
    can_climb_walls = true
```

#### Defense & Survival

```
// Armor value (damage reduction percentage)
armor_value = 1 + global_channels["structural_rigidity"] * 5.0
// e.g., armor_value of 2.0 means take 50% damage (1 - 1/2)

// Dodge chance (evasion probability)
dodge_chance = global_channels["elastic_deformation"] * 0.6
// e.g., 0.5 elastic = 30% chance to evade

// Health points
max_health = BASE_HEALTH 
    * (1 + global_channels["mass_density"])
    * (1 + global_channels["structural_rigidity"] * 0.5)

// Regeneration
// Note: "wound_healing" is the regulatory family channel, not "regeneration_rate" (which may be deprecated or renamed)
// Interpreter looks up by id; actual channel name depends on registry contents
healing_per_tick = global_channels["wound_healing"] * 0.2
// If wound_healing = 0.5, heals 0.1 HP/tick

// Poison resistance (regulatory family)
poison_resistance = global_channels["toxin_resistance"] * 0.8
// % damage reduction to poison effects

// Thermal tolerance (regulatory family)
thermal_resistance = global_channels["thermal_regulation"] * 0.7
// range of temperatures survivable; outside = slow damage
```

#### Offense & Combat Abilities

**Melee/Contact Attacks:**

```
if any body_region has high(kinetic_force) AND timing == OnContact:
    melee_attack_power = BASE_DAMAGE * (1 + global_channels["kinetic_force"] * 2.0)
    
    // Body site determines where attack originates
    // Channels in different body_site regions create different phenotypes
    for region in body_map where region["kinetic_force"] > 0.4:
        attack_location = region
        // Front = jaw/bite, back = kick, sides = swipe, etc.
        if region.surface_position < 0.3:
            attack_type = BITE or GRAPPLE
        elif region.surface_position > 0.7:
            attack_type = KICK or TAIL_WHIP
        else:
            attack_type = CLAW or SWIPE

if global_channels["mass_density"] > 0.6:
    // Heavy creatures have powerful impact attacks
    knockback_power = global_channels["mass_density"]
    add_ability("CHARGE", trigger=OnCommand, power=knockback_power)
```

**Chemical Attacks (Poison, Secretions):**

```
if any body_region has high(chemical_production):
    // chemical_production is a social family channel for pheromones/signaling
    // genesis:chemical_production channels with venom_injection-like manifests deliver venom
    chemical_attack_exists = true
    
    // Manifest composition_hooks determine how chemical_production interacts with other channels
    // E.g., threshold: {with: kinetic_force, threshold: 0.6, kind: threshold} → venom only effective with strong bite
    
    if manifest[chemical_production].composition_hooks includes {with: kinetic_force, kind: threshold}:
        // Venom injection style
        venom_potency = global_channels["chemical_production"] * global_channels["kinetic_force"]
        add_ability("VENOMOUS_BITE", trigger=OnContact, power=venom_potency)
    
    if expression_periodic AND body_site.depth_from_surface > 0.5:
        // Internal production, diffuse release (aura style)
        poison_aura_damage = global_channels["chemical_production"] * 0.3
        add_ability("POISON_CLOUD", trigger=Passive, power=poison_aura_damage)
    
    // Visual: poison glands
    for region in body_map where region["chemical_production"] > 0.3:
        if region.depth_from_surface < 0.3:
            color_palette.region_color_shifts[region] += -30  // shift hue toward green/purple
            add_particles("gland_secretion", region)
```

**Thermal Attacks:**

```
if global_channels["thermal_output"] > 0.3:
    // thermal_output is typically social (visual signaling) or regulatory (heat dissipation)
    // Manifest composition_hooks determine behavior:
    // {with: light_emission, kind: additive} → glow + heat = intimidation
    // {with: structural_rigidity, kind: gating, threshold: 0.6} → heat only if body is rigid enough to contain it
    
    if body_map[HEAD]["thermal_output"] > body_map[TORSO]["thermal_output"]:
        // Facial heat source → breath attack (concentrates in head region)
        add_ability("FLAME_BREATH", trigger=OnCommand, power=global_channels["thermal_output"])
    else:
        // Whole-body heat → aura
        add_ability("HEAT_AURA", trigger=Passive, power=global_channels["thermal_output"] * 0.3)
    
    // Environmental effects: melt ice, ignite flammable objects
    environment_effects.add("heat_source")
```

**Sound/Vibration Attacks:**

```
// vibration_output is a sensory/motor hybrid, depending on whether expressed as input (sensing) or output
if global_channels["vibration_output"] > 0.4:
    // Manifest may include: {with: vocal_ability, kind: multiplicative, coeff: 1.8}
    //   → sound amplitude = vibration_output * vocal_ability * 1.8
    
    // Manifest expression_conditions gate activation by biome
    if current_biome in manifest["vibration_output"].expression_conditions.biome_flags:
        if expression_periodic:
            // Rhythmic ground stomp or sonic pulse
            add_ability("SHOCKWAVE", trigger=Periodic, cooldown=20, power=global_channels["vibration_output"])
        elif expression_on_contact:
            // Sonic screech or echolocation burst
            add_ability("SONIC_BLAST", trigger=OnCommand, power=global_channels["vibration_output"])
        
        // Animation: tremor effects, sound particles
        animation_params.movement_style = HEAVY_STOMP
```

#### Sensory Abilities

```
// All sensory channels (light_sensing, chemical_sensing, vibration_sensing, etc.) are from the sensory family
// Family defaults: additive composition, biome-dependent expression

// Vision
// Manifest for light_sensing may have composition_hook: {with: light_emission, kind: additive, coeff: 0.3}
vision_range = BASE_VISION * (1 + global_channels["light_sensing"] * 0.3)

// Some channels may have multiple manifests (canonical + mods)
// e.g., "light_absorption" (core) or "mod:cave_dweller:infrared_vision" (genesis from light_sensing)
if global_channels["light_absorption"] > 0.5:
    can_see_in_darkness = true
    vision_range *= (1 + global_channels["light_absorption"] * 0.5)

// Hearing
hearing_range = BASE_HEARING * (1 + global_channels["vibration_sensing"] * 0.6)

// Smell (chemical sensing is biome-gated; high efficiency in dark/aquatic biomes)
smell_range = BASE_SMELL * (1 + global_channels["chemical_sensing"] * 0.8)
// Smell is longer-range and more effective than vision; family defaults enable this

// Tremorsense
tremor_sensitivity = global_channels["vibration_sensing"] * 0.7
if tremor_sensitivity > 0.3:
    add_ability("TREMORSENSE", trigger=Passive)

// Echolocation (emergent behavior from composition hooks)
// Manifest for vibration_sensing may include: {with: vocal_ability, threshold: 0.4, kind: threshold}
// AND manifest for vocal_ability includes: {with: vibration_sensing, threshold: 0.4, kind: threshold}
// When BOTH conditions met, echolocation emerges (EXAMPLE of emergent combination)
if global_channels["vibration_output"] > 0.4 AND global_channels["vibration_sensing"] > 0.4:
    if manifest["vibration_output"].has_composition_hook(with="vibration_sensing"):
        add_ability("ECHOLOCATION", trigger=Periodic, range=hearing_range * 1.5)
```

#### Camouflage & Concealment

```
// Camouflage effectiveness (reduction in detection range to enemies)
// light_absorption is structural family, affects how much light is absorbed (dark coloration)
// Manifest expression_conditions gate this by biome: effective in forests/caves, useless in bright open areas
if global_channels["light_absorption"] > 0.4:
    if current_biome in manifest["light_absorption"].expression_conditions.biome_flags:
        camouflage_bonus = global_channels["light_absorption"] * 0.5
        // Enemies need to be closer to spot you; enemies' detection_range *= (1 - camouflage_bonus)
        detection_range_vs_you *= (1 - camouflage_bonus)
        
        // Visual effect: color and pattern shift toward background
        color_palette.camouflage_pattern = GRADIENT or SPOTS
        color_palette.match_to_biome_hues()

// Invisibility or transparency (extreme light absorption)
// May be from core channel or genesis variant with modified range (e.g., light_absorption max raised to 1.5)
if global_channels["light_absorption"] > 0.8:
    can_hide = true
    shader.transparency = global_channels["light_absorption"] * 0.5
```

#### Special Interactions

```
// Wound healing (regulatory family) triggers ability
// Manifest may gate this by the top-level scale_band field: large creatures heal faster (higher metabolic budget)
if global_channels["wound_healing"] > 0.3:
    healing_per_tick = base_healing * (1 + global_channels["wound_healing"] * 0.2)
    add_ability("RAPID_HEALING", trigger=Passive, power=healing_per_tick)

// High metabolism (metabolic family) → fast action economy
// Metabolic channels often have antagonistic composition with regulatory channels
//   {with: toxin_resistance, kind: antagonistic} → high metabolism leaves less energy for defense
if global_channels["metabolic_rate"] > 0.7:
    action_speed_multiplier = 1.0 + global_channels["metabolic_rate"] * 0.3
    // Can attack more often, move faster through action economy

// Sapience & Communication (cognitive family)
// Manifest: neural_speed gating social channels
// {with: vocal_ability, kind: gating, threshold: 0.5} → vocalizations only if neural_speed > T
if global_channels["neural_speed"] > 0.6:
    sapience_level = global_channels["neural_speed"]
    can_be_reasoned_with = true
    // Enters full NPC social layer (from NPC Social Model, system 12)
```

---

### Procgen Visuals

The body morphology, colors, and animations are procedurally generated from channel profiles and body-site maps. The interpreter looks up manifests from the registry to determine which channels affect which visual properties (all manifests carry visual hints in composition_hooks and family defaults).

```
function generate_morphology(monster) -> MorphologyProfile:
    profile = MorphologyProfile()
    registry = ChannelRegistry.instance()
    
    // Size
    // Look up manifest for mass_density to confirm its range and family
    mass_density_manifest = registry.get("mass_density")
    profile.body_length = 0.8 + global_channels["mass_density"] * 3.0
    
    // Limb proportions
    // kinetic_force is motor family; typically affects limb thickness
    profile.limb_thickness = 0.5 + global_channels["kinetic_force"] * 0.5
    if global_channels["elastic_deformation"] > 0.5:
        profile.limb_thickness *= 0.8  // flexible creatures are more slender
    if global_channels["structural_rigidity"] > 0.5:
        profile.limb_thickness *= 1.2  // rigid creatures are more stocky
    
    // Spikiness (from motor + structural channels)
    // surface_friction is motor; kinetic_force is motor; multiplicative composition
    profile.spine_prominence = max(0, global_channels["surface_friction"] * 0.8
                                      + global_channels["kinetic_force"] * 0.3)
    
    // Smoothness (structural channels)
    profile.smoothness = 1.0 - global_channels["structural_rigidity"] * 0.5
    if global_channels["elastic_deformation"] > 0.6:
        profile.smoothness += 0.3  // bendy creatures are curvy
    
    // Bilateral symmetry (from TraitGene body_site records)
    // Note: symmetry is now per-gene, not global; average across genome
    symmetry_count = 0
    for gene in monster.genome:
        if gene.body_site.bilateral_symmetry:
            symmetry_count += 1
    profile.bilateral_ratio = 0.5 + (symmetry_count / len(monster.genome)) * 0.45
    
    // Per-region overrides (body-site dependent)
    for region in body_map:
        if region["wound_healing"] > 0.3:
            profile.region_overrides[region].smoothness += 0.2  // healing tissue is round
        if region["chemical_production"] > 0.3:
            profile.region_overrides[region].gland_density = region["chemical_production"]
        if region["light_emission"] > 0.2:
            profile.region_overrides[region].glow_intensity = region["light_emission"]
    
    return profile

function generate_color(monster) -> ColorProfile:
    palette = ColorProfile()
    registry = ChannelRegistry.instance()
    
    // Base hue influenced by biome and traits
    if monster.biome.light_level < 0.3:
        // Dark biome: pale, glowing colors
        palette.hue_base = 200  // cool blues
        palette.brightness = 0.6
    else:
        palette.hue_base = random(0, 360)
        // light_emission is social family; higher values boost brightness
        palette.brightness = 0.5 + global_channels["light_emission"] * 0.3
    
    // Saturation: predators are vivid (warning colors or to appear strong)
    // Prey are dull (camouflage)
    // kinetic_force from motor family; high = aggressive predator phenotype
    if global_channels["kinetic_force"] > 0.6:
        palette.saturation = 0.8  // predator: vivid
    elif global_channels["light_absorption"] > 0.5:
        palette.saturation = 0.3  // camouflaged: dull
    else:
        palette.saturation = 0.6  // neutral
    
    // Glow color (if bioluminescent)
    if global_channels["light_emission"] > 0.3:
        // chemical_production (social) + light_emission (social) interaction
        if global_channels["chemical_production"] > 0.3:
            palette.glow_color = RGB(0.5, 1.0, 0.3)  // green (warning + light = eerie)
        else:
            palette.glow_color = RGB(1.0, 0.8, 0.2)  // yellow-white (warm)
    
    // Camouflage pattern choice
    // Composition: light_absorption (structural) + structural_rigidity (structural)
    if global_channels["light_absorption"] > 0.6:
        if global_channels["structural_rigidity"] > 0.6:
            palette.camouflage_pattern = STRIPES    // rigid patterns (sharp boundaries)
        else:
            palette.camouflage_pattern = GRADIENT   // soft blending (flexible bodies)
    
    // Per-region hue shifts (chemistry glands → greenish, heat-emitting → reddish)
    for region in body_map:
        if region["chemical_production"] > 0.3:
            palette.region_color_shifts[region] -= 120  // shift toward green
        if region["thermal_output"] > 0.3:
            palette.region_color_shifts[region] += 30   // shift toward red
    
    return palette

function generate_animation(monster) -> AnimationProfile:
    anim = AnimationProfile()
    registry = ChannelRegistry.instance()
    
    // Movement style
    // elastic_deformation (motor) vs. mass_density (structural) vs. kinetic_force (motor)
    if global_channels["elastic_deformation"] > 0.7 AND global_channels["mass_density"] < 0.3:
        anim.movement_style = SLITHER
    elif global_channels["mass_density"] > 0.7:
        anim.movement_style = CRAWL or TRAMPLE
    elif global_channels["kinetic_force"] > 0.6:
        anim.movement_style = GALLOP or BOUND  // athletic
    else:
        anim.movement_style = WALK
    
    // Attack speed (faster metabolisms = quicker attacks)
    // metabolic_rate from metabolic family; controls energy budget and action frequency
    anim.attack_speed_multiplier = 0.5 + global_channels["metabolic_rate"] * 0.8
    
    // Idle restlessness: high-energy creatures fidget
    anim.idle_restlessness = global_channels["metabolic_rate"] * 0.8
    
    // Pain response (emotion is cognitive family)
    // wound_healing (regulatory) vs. neural_speed (cognitive) vs. structural_rigidity (structural)
    if global_channels["wound_healing"] > 0.5:
        anim.pain_response = THRASH  // healing creatures are tough, thrash around
    elif global_channels["neural_speed"] > 0.7:
        anim.pain_response = DODGE   // smart creatures react quickly
    elif global_channels["structural_rigidity"] > 0.7:
        anim.pain_response = FREEZE  // hard shells don't move much when hit
    else:
        anim.pain_response = CURL    // vulnerable creatures curl up
    
    return anim
```

---

## 5. Update Rules & Caching

The interpreter is **stateless** and **deterministic**. Given the same genome, channel registry, and body-site assignments, it always produces identical behavioral parameters. This ensures reproducibility and makes debugging tractable.

**Update Timing:**
- Morphology, color, and animation are computed **once at monster creation** and **never updated** (visual form is locked to birth genotype).
- Behavioral parameters (speed, damage, sensing ranges) are recomputed **once per biome entry** and cached (not per-tick, for performance).
- Abilities are recomputed **once at monster creation**; only update if channels structurally change (gene duplication, family reclassification).
- Body-site profiles are recomputed **once per generation** if the genome changes (gene duplication, mutation); cached otherwise.

**Registry Lookups at Interpretation Time:**

When the interpreter evaluates a monster, it:
1. Locks the current channel registry state (snapshot to avoid mid-game changes from mods)
2. For each TraitGene in the genome, looks up its manifest from the registry
3. Aggregates per-body-region channel profiles using manifest fields and expression_conditions
4. Applies composition_hooks from manifests to compute final behavioral parameters
5. Caches the result for the biome/generation

**Fallback Rules (for uninterpreted channel combinations):**

If a channel combination has no explicit composition_hook, apply generic fallback:

```
// If a body region has high expression of a channel but no matching composition rules:
if any region has high(channel_value) AND registry.get(channel_id).composition_hooks.is_empty():
    apply_generic_aura(channel_value, body_region, family=registry.get(channel_id).family)
    // Emit a visual aura (family-color-coded), apply mild stat bonus, log for designer review
    
    // Designer can then write composition_hooks for this channel with others
    // OR extend an existing family default to cover this case
```

---

## 6. Cross-System Hooks

**To Evolutionary Model (System 01):**
- Evolutionary model mutates manifest fields (range, mutation_kernel, composition_hooks) for each channel per generation
- Trait system exports channel registry (manifests) to evolution for mutation kernel lookup
- Evolution system calls `channel_genesis()` when a duplication event occurs; trait system registers the paralog
- Evolution system tracks `mutations_accumulated` count per TraitGene (used to score divergence)
- Primitive genesis (future work): evolution system will call `primitive_genesis()` to register paralog primitives; trait system returns primitive registry to evolution for mutation kernel lookup
- No feedback loop; evolution drives changes, trait system reacts

**To Phenotype Interpreter (System 11):**
- Trait system aggregates genome → per-body-region channel profiles
- Trait system exports channel registry (manifests with composition_hooks and expression_conditions) to interpreter
- Trait system exports primitive registry (manifests with parameter_schema and composition_compatibility) to interpreter
- Interpreter evaluates composition_hooks at runtime; when a threshold/gating condition is met, interpreter emits referenced primitives with parameters computed from channel values
- Interpreter checks expression_conditions to gate channel activation by biome/scale/stage
- No feedback; interpreter reads both registries only

**To Chronicler / Emergent Labeling (System 09):**
- Chronicler monitors primitive emissions (which primitives, at what frequency, with what parameter values)
- Chronicler uses primitive observable_signature to map emitted effects to named patterns (e.g., echolocation = (vibration_output + vibration_sensing) > threshold)
- Primitive registry enables Chronicler to recognize emergent behaviors without explicit ability definitions
- As primitive genesis (future work) introduces new primitives, Chronicler learns new signatures dynamically

**To NPC Social Model (System 12):**
- `sapience_level` (derived from global_channels["neural_speed"]) gates NPC communication
- Observable abilities (combat skills, sensory acuity) update NPC beast_knowledge
- Beasts with high sapience can hold opinions and participate in social dynamics
- Social channels (vocal_ability, chemical_production) determine communication modality available to NPC

**To Combat Simulation:**
- Behavioral parameters (speed, attack_power, armor) are read by combat engine
- Sensory ranges determine what beasts can perceive and target
- Abilities determine what actions are available in combat
- Body-site profiles enable location-specific damage and armor calculations

**To Procgen Asset Pipeline:**
- Morphology profile (from generate_morphology) feeds into mesh generation
- Color profile (from generate_color) feeds into material/shader system
- Animation profile (from generate_animation) feeds into animation blending
- Body-site depth_from_surface determines which visual elements are shown (superficial glands visible, internal organs hidden)

**To Disease Model (System 16):**
- Pathogens share the same genome and channel schema as macro-scale beasts
- Trait system serves the same registry and interpreter logic to both; scalars differ only in scale_band
- Regulatory channels (immune_response, toxin_resistance) gate pathogen virulence and host resistance interaction
- Expression_conditions enforce scale_band: pathogens can only express channels in micro scale_band

---

## 7. Tradeoff Matrix: Registry & Primitive Design

| Decision | Option A | Option B | Option C | Sim Fidelity | Implementability | Extensibility | Choice & Why |
|---|---|---|---|---|---|---|---|
| **Registry Architecture** | Single unified registry (channels + primitives + abilities) | Dual registries (channels & primitives separate) | Implicit emergence (no registry, rules inferred from gameplay) | Medium (A) | Medium (A) | Low (A) ↔ High (B) ↔ Very High (C) | **Dual registries (B)** — Channels govern evolution (which traits spread); primitives govern phenotype output (which effects are emitted). Separation of concerns enables independent evolution of trait-space and effect-space. Unification (Option A) creates circular dependencies (evolution depends on interpreter rules; interpreter depends on evolved traits). Implicit emergence (Option C) is uncontrollable. |
| **Primitive Categories** | 8 functional categories (as specified in Section 3B) | Fine-grained per-modality categories (signal_acoustic, signal_visual, force_crushing, force_piercing, etc.) | No categorization (primitives are unstructured list) | High (A) | Medium (B) | Medium (A) ↔ Low (B) ↔ Low (C) | **8 functional categories (A)** — Anchors primitive taxonomy to functional roles (what work does the primitive do?), not implementation details (modality). Reclassification (System 01, Section 4C) matches primitives to functional archetypes. Fine-grained categories scale poorly; unstructured primitives are unmaintainable. |
| **Channel Representation** | Fixed enum (18 channels, shipped) | Family-based registry (core ~17–20, + mods + genesis) | Purely emergent graph (channels inferred from gameplay logs) | Medium (A) | High (A) | Low (A) ↔ High (B) ↔ Very High (C) | **Family-based registry (B)** — Maintains biological structure (9 families as constraint classes) while enabling extensibility. Genesis provides evolution-driven channel creation without designer overhead. Avoids enum brittleness and emergence-only chaos. |
| **Channel Genesis** | No genesis; fixed channel set | Genesis via duplication; limited divergence | Genesis with reclassification into new families | Low (A) | High (A) | Low (A) ↔ Medium (B) ↔ High (C) | **Genesis + reclassification (C)** — Channel set evolves alongside creatures. Rare reclassifications (after threshold divergence) create "discovery" moments (new channels unlock new niches). Tradeoff: requires divergence scoring and family affinity computation, but payoff is emergent complexity. |
| **Body-Site Overlay** | Global channels only (no per-region variation) | Manifest-driven: body_site_applicable flag | Unrestricted per-region, each gene carries full manifest | Medium (A) | High (A) | Low (A) ↔ High (B) ↔ Very High (C) | **Manifest-driven (B)** — Channels declare whether they vary per body region (body_site_applicable field). Keeps schema clean; avoids explosion of per-region manifests. Front limbs ≠ hind limbs phenotypically but share same channel id. |
| **TraitGene Structure** | Fixed vector [18 floats] | Variable-length: list[TraitGene] keyed by channel id | Fully diploid with Mendelian dominance per locus | High (A) | High (A) | Medium (A) ↔ High (B) ↔ Very High (C) | **Variable-length (B)** — Supports duplication/loss, allows genome growth, reduces memory for haploid pathogens. Diploid optionality (expression_level field) remains for future asexual→sexual transition. Loses fixed-vector simplicity but gains realism. |
| **Composition Rules** | Hard-coded per channel pair | Manifest composition_hooks (declarative) | Learned from fitness data (meta-learning) | Medium (A) | High (A) | Low (A) ↔ High (B) ↔ High (C) | **Manifest composition_hooks (B)** — Designers declare interactions (additive, multiplicative, threshold, gating, antagonistic) in manifest. Emergent behaviors arise from threshold/gating combos. Hard-coded is brittle; learned rules require extensive gameplay data we don't have. |
| **Expression Conditions** | Always express all channels | Biome + scale + developmental stage gates (expression_conditions) | Prediction model (which channels useful in which contexts) | Low (A) | High (A) | Medium (B) | **Expression_conditions gates (B)** — Dormant-if-unmet channels are genome-present but phenotypically silent until biome affords them. Enables creatures to "colonize" new biomes (dormant heat-sensitivity activates in volcanic biome). Clean, testable. |
| **Interpreter Rule Representation** | Hard-coded if-else chains per channel | Declarative rule engine (composition hooks + family defaults) | Neural network mapping channels → abilities | Medium (A) | High (A) | Medium (A) ↔ High (B) ↔ High (C) | **Declarative rules (B)** — Composition hooks are interpreted at runtime; rules are composable and testable. Designer-readable. NN would be overkill for the problem size; hardcoding duplicates rules across channel pairs. |

**Rationale for Registry + Families + Genesis:**

The fixed 18-channel enum (Option A) is simple to code but brittle: adding a mod channel requires code changes; removing a channel breaks compatibility. A pure emergent graph (Option C) has no designers' control and creates inconsistent semantics.

The family-based registry (Option B) strikes a balance:
- **Core channels**: ~17–20 channels, 9 families, shipped with game. Provide stable vocabulary for designers and evolution.
- **Mod channels**: Mods extend registry without code changes; inherit family defaults for composition and mutation.
- **Genesis channels**: Duplication + divergence allow the channel set to evolve. Reclassification (rare, high divergence threshold) creates emergence of new "species" of channels, parallel to creature speciation.

Channel genesis (Option C over B) is justified by:
- **Narrative**: "You've evolved a new type of venom!" is a discovery moment.
- **Fitness landscape**: Reclassified channels occupy new interaction niches, enabling escape from local optima.
- **Cost**: Divergence scoring is O(manifest_fields) per generation; reclassification check is O(families) per paralog. Acceptable.

Body-site manifest-driven (Option B) avoids per-region manifest explosion while preserving phenotypic diversity. A creature with kinetic_force in front limbs looks different (claws) from one with kinetic_force in tail (tail whip), but both have identical combat damage. This is biologically realistic (same protein, different morphology).

---

## 8. Emergent Properties

**Phenotypic Uncoupling from Genotype**: Two monsters with identical phenotypes (behavioral parameters and visuals) may have evolved them through completely different channel pathways. One has high chemical_sensing for smell-tracking; another has high vibration_sensing for echolocation. To the player, they "seem similar," but they're adapted to different sensory niches.

**Morphological Diversity**: Without hand-authored limits, monsters can achieve visually bizarre combinations: a massive, spikey, glowing, regenerating creature with weak limbs (high mass_density + light_emission + regeneration_rate + low kinetic_force). This is evolutionarily plausible but not "cool-looking." The interpreter gracefully degrades (generic aura) but doesn't prevent it.

**Niche Specialization**: Over time, populations segment into morphological niches—ambush predators (high light_absorption), active pursuit predators (high metabolic_rate), passive filter-feeders (high chemical_sensing), etc. These "archetypes" emerge from channel combinations, not from named categories.

**Visual Feedback Validates Evolution**: Players see monsters gradually change—becoming faster, spikier, glowing more. This visual feedback demonstrates that evolution is working, even though the mechanics are invisible to the player.

---

## 9. Open Calibration Knobs

| Parameter | Current Value | Range | Effect | How to Tune |
|---|---|---|---|---|
| `BASE_MOVEMENT_SPEED` | 1.0 | 0.5–2.0 | Absolute movement rate | Increase if combat feels slow |
| `BASE_DAMAGE` | 10 | 5–20 | Attack power baseline | Tune for combat balance |
| `BASE_HEALTH` | 100 | 50–200 | HP pool | Increase for longer fights |
| `BASE_ARMOR` | 1.0 | 0.5–1.5 | Defense multiplier | Increase if monsters trivialize player |
| `BASE_VISION` | 20 | 10–40 | Sight distance | Affect monster awareness |
| `BASE_HEARING` | 30 | 10–50 | Sound detection range | Longer = more responsive to noise |
| `BASE_SMELL` | 40 | 20–80 | Olfaction distance | Smell is longer-range than vision (real) |
| `CHANNEL_EFFECTIVENESS` (per-channel) | Varies | 0.3–2.0 | How much each channel contributes | Tune if one channel-type dominates |
| `Ability trigger rates` | Varies | Per ability | How often automated abilities trigger | Higher = more constant abilities |

All of the above are content-tuning parameters. No changes to the interpreter logic are required to balance—only slider adjustments in config files.

---

## 10. Notes on Implementation

**Performance:**
- Genome aggregation (TraitGenes → per-body-region profiles) is O(genome_size) ≈ O(100–1000 genes per creature) and happens once per generation change. Cached.
- Manifest lookups are O(1) hashmap access; registry is locked at interpretation time (no concurrent mutation).
- Behavioral parameter resolution is O(num_genes + num_composition_hooks) and cached. Only recomputed on biome change, generation advancement, or significant genome mutation.
- Ability triggering (in combat) is O(num_abilities) ≈ O(5–15), cheap per monster per tick.
- Registry size grows with mods and genesis channels; expected to plateau at O(100–500 channels) total (9 families × 10–50 channels per family).

**Testing & Validation:**
- Unit test each composition_hook in isolation (e.g., "if elastic_deformation=0.7 × structural_rigidity=0.4, is dodge_chance correct?")
- Verify manifest schema compliance: all channels in registry pass JSON schema validation, family membership is valid, composition_hooks reference existing channels.
- Generate 1000 random genomes, verify all produce valid behavioral parameters (no NaN, no out-of-range values).
- Spot-check procgen morphologies visually: do they match their channel profile?
- Test expression_conditions: create monsters with dormant channels; verify they activate in correct biome/scale/stage.
- Test genesis: create parasites, verify new manifests register correctly, divergence scoring works.
- A/B test ability balance: do high-kinetic_force monsters consistently outperform high-structural_rigidity monsters?

**Content Pipeline:**
1. Biologist/game designer writes high-level trait concepts ("venom," "armor," "echolocation")
2. Designer maps concepts to one or more channels from the registry (may require channel genesis if new niche)
3. If new channel needed, designer writes manifest (inherit family defaults)
4. Composition hooks are declared (interactions with other channels)
5. Interpreter uses hooks automatically; no additional rule writing required
6. Procgen produces visuals from body_site data and family defaults
7. Designers spot-check and polish (override color, add custom particles as needed)
8. Gameplay testing tunes base values (BASE_DAMAGE, metabolic costs, etc.) in config files

**Debug Tools:**
- "Channel Registry Viewer": Search, filter by family, display manifests with composition hooks. Diff registry state across game versions.
- "Channel Profile Viewer": Display active channels as radar chart per body region, per monster. Highlight dormant channels (unmet expression_conditions).
- "Genesis Inspector": Show all paralog channels, their parent lineages, divergence scores, and reclassification status.
- "Ability Inspector": Trace which composition hooks generated each ability. Show threshold values and activating channels.
- "Morphology Previewer": Render a test monster with any genome / any registry state.
- "Rule Trace": Log which composition hooks fired for a given channel profile, in order, with coefficients.
- "Expression Gate Report": List all channels and their expression_conditions; highlight which are currently active/dormant per biome.
- "Fallback Report": List all channels in a gameplay session that had empty composition_hooks (generic aura was applied).

---

## 11. Migration Notes: From Fixed Enum to Registry

This revision introduces significant architectural changes to support extensibility, channel evolution, and cleaner manifest-driven design. Projects upgrading from the prior version should follow these migration steps:

### What Changed

**Before:** Traits were a fixed enum of 18 channels (kinetic_force, mass_density, etc.). The Phenotype Interpreter hard-coded rules for each channel combination. Body sites were simple enums; traits were always global or per-region, with no middle ground.

**After:** Channels are declared in **manifests** (JSON documents), assembled into a **runtime registry**. Families (9 categories) provide defaults, reducing boilerplate. Genomes are variable-length lists of TraitGenes (not fixed vectors), supporting gene duplication and loss. Body-site placement is fine-grained (depth, position, symmetry). Channel genesis allows the channel set to evolve.

### Breaking Changes

1. **Genome Format**: Genomes are now `List[TraitGene]` instead of `float[18]`. Evolution system must track TraitGenes, not a fixed vector. Backward compatibility: write a one-time migration script to convert old fixed vectors to new genome format (assign each of the 18 channels to a canonical body region, set expression_level=1.0).

2. **Channel Identifiers**: Channels are now referred to by string id (e.g., `"kinetic_force"`, `"genesis:kinetic_force:1200"`) instead of array indices. Interpreter queries registry by id. If old code indexes channels by position, refactor to use registry lookups.

3. **Interpreter Input**: Phenotype Interpreter now receives:
   - Genome (list of TraitGenes with body_site data)
   - Channel Registry snapshot
   - Body region aggregation rules (implicit in manifest)
   
   Instead of pre-computed per-body-region float arrays. Interpreter now aggregates on-demand.

4. **Composition Hooks**: Hard-coded rules (if-else chains in interpreter) are replaced with declarative `composition_hooks` in manifests. Existing if-else rules must be translated to manifests. Priority: high-impact rules first (movement, combat, sensory).

### Non-Breaking Enhancements

1. **Backward Compatibility**: Core channels (the original 18) are registered with the same names and ranges. Existing interpreter rules (e.g., `global_channels["kinetic_force"]`) work without change. New code should use registry lookups for extensibility.

2. **Procgen Integration**: Morphology, color, and animation generation now reads body_site depth and position, enabling finer visual variety. Existing procgen code continues to work; enhanced code can leverage new fields.

3. **Biome Expression**: New `expression_conditions` field gates channel activation by biome. Dormant channels (unmet conditions) do not contribute to phenotype. Existing creatures use expression_conditions={"biome_flags": ["any"], ...} (always expressed).

### Migration Checklist

- [ ] Write manifest files for the original 18 core channels (template provided in `data/channels/core.manifest`)
- [ ] Test channel registry startup; verify all 18 channels register without errors
- [ ] Implement genome aggregation function (TraitGenes → per-body-region profiles)
- [ ] Update Evolution system to create/register paralog channels on duplication; track mutations_accumulated
- [ ] Refactor highest-impact interpreter rules to composition_hooks
- [ ] Test: generate 1000 random genomes, verify phenotypes match old interpreter output (within ~2% tolerance for rounding)
- [ ] Update debug tools (registry viewer, profile viewer, ability inspector)
- [ ] Write tests for channel genesis and divergence scoring
- [ ] Stress test: run gameplay with mods; verify new channels don't break existing rules
- [ ] Document family defaults and composition hook patterns for mod authors

### Design Decisions Made in This Revision

1. **Why Families?** The 9 families (sensory, motor, metabolic, etc.) group channels with similar evolutionary constraints and interaction patterns. Family defaults reduce manifest boilerplate. Mods inherit sensible defaults automatically.

2. **Why Variable-Length TraitGenes?** Fixed vectors don't support gene duplication or loss; variable-length genomes are standard in evolutionary algorithms. This enables in-game channel evolution and supports pathogens (which may have many duplicated immunity genes).

3. **Why Expression Conditions?** Allows a single channel to be "always in genome, sometimes in phenotype." Enables creatures to migrate to new biomes and unlock previously dormant traits. Realistic (many traits are environmentally contingent).

4. **Why Composition Hooks over Hard-Coded Rules?** Declarative composition is testable, designer-editable, and extensible by mods. Hard-coded rules are brittle and scale poorly. Composition hooks support emergence (threshold combinations creating new behaviors) without explicit "ability" code.

5. **Why Channel Genesis?** Duplication + divergence is the primary mechanism for new channel evolution in real biology. Allowing it in-game creates discovery moments ("You've evolved a new toxin type!") and allows the fitness landscape to expand (new channels = new niches).

### Example: Authoring a Mod Channel

A mod author wants to add a "silk_production" channel for web-spinning creatures:

```json
{
  "id": "mod:arachnid_pack:silk_production",
  "family": "motor",
  "description": "Ability to spin adhesive silk threads",
  "range": [0.0, 1.0],
  "units": "normalized",
  
  "mutation_kernel": {
    "sigma": 0.06,
    "bounds_policy": "clamp",
    "genesis_weight": 0.7,
    "correlation_with": ["kinetic_force", "spatial_awareness"]
  },
  
  "composition_hooks": [
    {
      "with": "structural_rigidity",
      "kind": "threshold",
      "threshold": 0.4,
      "coefficient": 1.5,
      "description": "Rigid spinnerets produce stronger threads"
    },
    {
      "with": "metabolic_rate",
      "kind": "antagonistic",
      "coefficient": 0.5,
      "description": "Silk production is metabolically expensive"
    }
  ],
  
  "expression_conditions": {
    "biome_flags": ["forest", "cave"],
    "scale_band": [0.05, 50.0],
    "developmental_stages": ["juvenile", "adult"],
    "dormant_if_unmet": true
  },
  
  "body_site_applicable": true,
  "provenance": "mod:arachnid_pack",
  "generation_born": 0
}
```

The mod ships this manifest in `mods/arachnid_pack/data/channels/silk_production.manifest`. At game startup, the registry loads it. Web-spinning mechanics can then be added via interpreter rules or additional composition hooks (e.g., silk_production + spatial_awareness = ability to navigate webs).

---

## 12. Migration Notes (Revision: Primitive Registry & Composition Hooks with Primitive Emission)

**From Previous Version (Channel Registry Only, Hard-Coded Abilities)**:

1. **Primitive Registry Introduction**: A new Primitive Registry (Section 3B) is introduced, parallel to the Channel Registry. Primitives are atomic effects (emit_acoustic_pulse, apply_bite_force, etc.) declared in manifests with parameter_schema, composition_compatibility, cost_function, and observable_signature. This separates phenotype vocabulary (what effects exist) from phenotype expression (how channels emit those effects).

2. **Composition Hooks Gain `emits` Field**: Channel manifests' composition_hooks gain an optional `emits` field (optional; future work for full parameter-mapping implementation). When a hook's condition is met (threshold, gating, etc.), listed primitives are emitted with parameters mapped from channel values. Example: `{with: kinetic_force, kind: threshold, threshold: 0.6, emits: [{id: "apply_bite_force", parameters: {force: "kinetic_force * 2.0"}}]}`. (This field is forward-compatible; current interpreter ignores it; future revision implements parameter mapping and primitive emission.)

3. **Registry Startup Validation**: At game startup, the trait system validates that all primitives referenced in composition hooks exist in the Primitive Registry. Mods must register primitives before channels that emit them.

4. **Primitive Manifests Carry Provenance**: Like channel manifests, primitive manifests carry provenance (core, mod:X, genesis:parent:generation) and generation_born. This prepares for primitive genesis (future work, System 01 Section 4C). Current builds initialize primitive registry from core manifests and mods; genesis primitives are created dynamically when duplication is implemented.

5. **Observable Signature for Chronicler**: Primitive manifests' observable_signature field (modality, range, detectability) is read by System 09 (Chronicler). Chronicler uses observable_signature to recognize emergent patterns in primitive emissions and label them as abilities without explicit ability definitions.

6. **Save State Expansion**: Save states now snapshot both channel registry and primitive registry. When primitive genesis is enabled, primitive-registry growth (new paralog manifests) is captured in snapshots, enabling deterministic replay.

7. **8-Category Taxonomy Committed**: The 8 primitive categories (signal_emission, signal_reception, force_application, state_induction, spatial_integration, mass_transfer, energy_modulation, bond_formation) are the stable taxonomy. Primitive reclassification (System 01, Section 4C, future work) will match primitive manifests to category archetypes; categorization is not subject to change.

8. **Backward Compatibility**: Existing channel manifests continue to work; composition_hooks without `emits` are interpreted by hard-coded rules (legacy behavior). New manifests can include `emits` for forward-compatibility. Interpreter gracefully degrades: if a hook has no `emits` field and no hard-coded rule covers it, a generic aura is applied (as before).
