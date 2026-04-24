# The Phenotype Interpreter: Primitive-Effect Engine

## 1. Overview

The Phenotype Interpreter is the **formal composition law** that converts an evolved genotype into a concrete primitive-effect output used by all downstream systems. **Formally:**

```
interpret(genome, environment, channel_registry, primitive_registry) -> Set<PrimitiveEffect>
```

Where:
- `genome`: global channels [0,1] × body map
- `environment`: biome, time, season, social density, developmental stage
- `channel_registry`: dynamically mutable set of channels and composition hooks (enables genesis)
- `primitive_registry`: dynamically mutable set of primitive effect manifests with composition hooks

**Output specification**: Each `PrimitiveEffect` in the returned set carries:
- `primitive_id`: identifier in primitive_registry
- `parameters`: derived from hook.emits[].parameter_mapping expressions evaluated against channel values
- `cost`: computed from primitive manifest's cost_function, applied to parameters (NOT hardcoded per ability)
- `cooldown_hint`: from primitive manifest's recovery_time (NOT hardcoded)
- `source_channels`: which channels triggered this emission (for provenance)

**Core commitment**: Same genome + same environment + same channel_registry state + same primitive_registry state → identical primitive-effect set. The interpreter is fully deterministic. All variance emerges from evolved differences, registry mutations, and environmental affordances — never from interpreter randomness.

**Three sources of novel phenotypes** (detached from scripted content):
- **(A) Emergent Primitive Combinations**: Threshold and gating composition hooks fire when multiple channels jointly cross thresholds, causing matched primitives to emit from the registry.
- **(B) Channel Genesis**: Paralog channels arise via gene duplication and divergence; the interpreter handles unknown channels at runtime (registry-driven).
- **(C) Environmental Affordances**: The same genome in different biomes/seasons/social contexts emits different primitive effects. Expression conditions gate which primitives fire.

The interpreter executes five integrated stages, now outputting primitive effects instead of named abilities. Composition hooks have a new `emits` field that lists which primitives fire when the hook triggers, with parameter-mapping expressions.

### Key Principles

1. **Primitive-effect output**: The interpreter's only output is a set of `PrimitiveEffect` objects. No named abilities ("echolocation", "pack_hunting_bond") ever appear in interpreter output. Naming is the Chronicler's job.
2. **Determinism**: Every output is a pure function of (genome, environment, channel_registry, primitive_registry). No random generation happens here. Novelty is emergent, not random.
3. **Registry-driven mutability**: Composition hooks, channels, and primitives live in mutable registries. No recompilation on genesis or hook drift — interpreter reads registries each tick.
4. **Cross-stage compositionality**: Primitive effects feed into stats, behavior compilation, visual directives, and interaction handlers.
5. **Body region tiling**: Primitives are evaluated per body region; resolution algorithms synthesize a coherent MonsterInstance.
6. **Invariant 3.8 (Primitive Foundation)**: The interpreter is a primitive-effect generator, not a rule engine or named-ability factory. Downstream systems (combat, UI, behavior) are responsible for reading primitives and assigning semantics.
7. **Invariant 3.9 (Mechanics-Label Separation)**: Composition_hooks' `emits` field is the ONLY path to primitive emission. No hardcoded ability inference or residual label-based branching in interpreter logic. Combat systems read PrimitiveEffect sets, never ability names. Labels are UI-only and assigned downstream by the Chronicler.

---

## 2. Research Basis

**Tinbergen's Four Questions (Ethology)**: The interpreter is structured around Tinbergen's framework for animal behavior:
- *Causation* (mechanistic): What channels trigger this behavior? → Primitive-effect generation
- *Development* (ontogeny): How do these channels co-regulate? → Regulatory channels, composition hooks
- *Evolution* (phylogeny): How did these traits evolve? → Channel definitions in evolution doc
- *Function* (adaptive value): Why does this morphology work? → Combat/ecology systems consume primitives

**Unreliable Narrator (Wolfe, Book of the New Sun)**: The interpreter outputs raw primitive effects; meaning is assigned downstream. A high LIGHT_ABSORPTION channel emits `activate_camouflage(strength=f(LIGHT_ABSORPTION))`. The Chronicler or renderer interprets what camouflage "looks like" in a given biome.

**Behavior Compilation (Game AI)**: Primitive effects drive behavior tree construction. Instead of hand-authored combat systems, behavior emerges from the primitive effects a creature can emit.

---

## 3. Entities & State

### 3.0 Numerical Representation & Determinism

All parameter computations in the interpreter use **fixed-point arithmetic** for deterministic, bit-identical replay. Saturation, clamping, and channel-value operations apply fixed-point operations, not floating-point.

**Parameter Computation Example**:
```
intensity = ch[VOCAL_MODULATION] * ch[NEURAL_SPEED]  // both Q32.32
        = (ch1 >> 16) * (ch2 >> 16) >> 16  // fixed-point multiply
range = ch[AUDITORY_SENSITIVITY] * 8  // fixed-point scalar mult
      = (ch[AUDITORY_SENSITIVITY] << 3)  // left-shift = multiply by 2^3
```

All channel values are Q32.32 fixed-point; parameter expressions are evaluated in fixed-point. No floats appear in the interpreter hot path. Visualization/rendering (UI layer) may convert Q32.32 to float for graphics, but the interpreter itself is deterministic integer-only.

**Saturation & Clamping** (deterministic):
- Clamp to range: `clamp(value, min, max)` uses integer comparisons.
- Overflow: Fixed-point multiply may overflow; use saturating arithmetic (clamp to signed 64-bit bounds) rather than wrapping.

---

### 3.1 Resolved Phenotype (Input)

```
ResolvedPhenotype {
    // From evolution Layer 2 (the channel resolver)
    global_channels: list<float>        // N channels (varies with genesis), each [0, 1]
    channel_names: dict<int, string>    // maps channel_id -> "auditory_sensitivity", etc.
    body_map: list<BodyRegion>          // spatial distribution of channels
    life_stage: enum {Juvenile, Adult, Elderly}
    genotype_id: uint64                 // for determinism verification
    mutation_tick: uint64               // when this genotype was first expressed
    
    // Environmental context (gates expression and affordances)
    biome: BiomeID                      // affects expression_conditions filtering
    biome_tags: list<BiomeTag>          // semantic tags (aquatic, arboreal, volcanic, etc.)
    season: enum                        // spring, summer, fall, winter
    local_light_level: float            // [0, 1] nocturnal → diurnal
    local_temperature: float            // Celsius
    local_population_density: float     // creatures per 100 sq units
    current_age_ticks: uint64           // for age-gating behavior
    position: Vec3
}

BodyRegion {
    id: int
    body_site: enum {Head, Jaw, Core, Limb_L, Limb_R, Tail, Appendage}
    surface_vs_internal: float          // 0 = deep, 1 = surface
    channel_amplitudes: list<float>     // per-region channel values (post-resolution)
}
```

### 3.2 Primitive Effect

A primitive effect is an atomic phenotype output in a small verb vocabulary:

```
PrimitiveEffect {
    id: unique_id                       // for deduplication
    primitive_id: string                // references primitive_registry entry
    category: enum {
        signal_emission,                // emit_acoustic_pulse, emit_pheromone, emit_light
        signal_reception,               // receive_acoustic_signal, detect_pheromone
        force_application,              // apply_bite_force, apply_impact_force
        state_induction,                // elevate_metabolic_rate, induce_sleep
        spatial_integration,            // spatial_integrate, terrain_map
        mass_transfer,                  // inject_substance, consume_material
        energy_modulation,              // modulate_temperature, modulate_pH
        bond_formation                  // form_host_attachment, form_symbiosis
    }
    parameters: dict<string, float>     // derived from trigger_channel_values
    trigger_channels: list<int>         // which channels drove this primitive
    trigger_values: list<float>         // their values when it fired
    body_region: BodyRegion             // which region emits this
    activation_cost: float              // metabolic or stamina cost (if any)
}
```

### 3.3 Primitive Manifest

A manifest defines a primitive effect's shape, constraints, and compatibility:

```
PrimitiveManifest {
    id: string                          // e.g., "emit_acoustic_pulse", "receive_acoustic_signal"
    category: enum {signal_emission, signal_reception, ...}
    
    // SCHEMA: What parameters does this primitive accept?
    parameter_schema: dict<string, {
        name: string,                   // e.g., "range", "frequency", "intensity"
        type: enum {float, int, bool},
        range: [min, max],              // e.g., [0.5, 20.0] for range in units
        derivation: string              // how to compute from channel values, e.g., "ch[5] * 8"
    }>
    
    // COMPATIBILITY: Which channel families can drive this primitive?
    composition_compatibility: list<int>  // channel_ids that can trigger this primitive
    
    // COST: Metabolic or stamina penalty
    cost_function: fn(parameter_dict) -> float  // e.g., (range * freq) * 0.1
    
    // OBSERVABILITY: How does this primitive appear in the game world?
    observable_signature: {
        visual: string,                 // "acoustic_ripple", "pheromone_cloud", etc.
        audio: string,                  // "high_pitched_chirp", null if silent
        particle_effect: string         // null if no visuals
    }
    
    // PROVENANCE: Where did this primitive come from?
    provenance: enum {authored, emerged, genesis}  // authored = handcrafted; emerged = born from composition; genesis = created via paralog event
}
```

### 3.4 Composition Hook (Extended)

```
CompositionHook {
    id: string                          // unique identifier
    kind: enum { Additive, Multiplicative, Threshold, Gating, Antagonistic }
    channel_ids: list<int>              // which channels combine?
    threshold_values: list<float>       // for Threshold kind
    
    // NEW: Which primitives fire when this hook triggers?
    emits: list<{
        primitive_id: string,           // references primitive_registry entry
        parameter_mapping: dict<string, string>  // maps parameter names to expressions
                                                 // e.g., {"range": "ch[2] * 8", "freq": "ch[1] * 100"}
    }>
    
    modifier_coefficient: float         // how much does this hook affect primitive parameters?
    cache_key: string                   // for invalidation on genesis
    expression_conditions: ExpressionCondition or null  // when does this hook activate?
}
```

---

## 4. The Primitive Effect Vocabulary

The interpreter has access to a curated vocabulary of 8 primitive-effect categories. These are the only outputs the interpreter can produce. Downstream systems (Chronicler, combat, UI) consume these primitives and assign names/labels/behaviors.

### 4.1 Signal Emission (sensory broadcast)

Creatures emit information into their environment:
- `emit_acoustic_pulse(range, frequency, intensity, waveform)` — sonic detection signal
- `emit_pheromone(distance, molecular_type, concentration)` — chemical signal
- `emit_light(wavelength, intensity, duration)` — bioluminescent signal
- `emit_electrical_discharge(voltage, radius)` — electrical signal

**Parameter derivation**: e.g., for `emit_acoustic_pulse`, range = ch[AUDITORY] * 8, frequency = ch[VOCAL] * 100, intensity = sqrt(ch[VOCAL] * ch[NEURAL_SPEED]).

### 4.2 Signal Reception (sensory intake)

Creatures receive and process environmental signals:
- `receive_acoustic_signal(sensitivity, resolution, frequency_range)` — sound perception
- `detect_pheromone(sensitivity, molecular_type_range)` — olfactory perception
- `detect_light(sensitivity, wavelength_range)` — visual perception
- `detect_electrical_field(sensitivity)` — electroreception

**Parameter derivation**: sensitivity = ch[AUDITORY_SENSITIVITY] * 0.8, resolution = ch[SPATIAL_COGNITION] * 0.5.

### 4.3 Force Application (physical interaction)

Creatures apply force to their environment or targets:
- `apply_bite_force(magnitude, location, sharpness)` — melee attack
- `apply_impact_force(magnitude, direction, radius)` — area damage
- `apply_grapple_force(magnitude, hold_duration)` — grappling

**Parameter derivation**: magnitude = ch[KINETIC_FORCE] * 10, sharpness = ch[STRUCTURAL_RIGIDITY] * 0.8.

### 4.4 State Induction (affect living systems)

Creatures induce physiological states in themselves or targets:
- `elevate_metabolic_rate(duration, intensity)` — temporary metabolism boost
- `induce_fatigue(duration, intensity)` — debuff target
- `induce_sleep(duration, resistance_threshold)` — stun effect
- `inject_substance(substance_type, volume, duration)` — poison, venom, antibiotic

**Parameter derivation**: intensity = ch[CHEMICAL_OUTPUT] * 0.7, duration = ch[METABOLIC_RATE] * 100.

### 4.5 Spatial Integration (perceiving and mapping space)

Creatures build mental or physical models of their environment:
- `spatial_integrate(resolution, range, update_frequency)` — build cognitive map
- `terrain_map(coverage, detail_level)` — learn terrain layout
- `predict_trajectory(accuracy, lookahead_time)` — predict movement

**Parameter derivation**: resolution = ch[SPATIAL_COGNITION] * 0.3, range = ch[PERCEPTION] * 5.

### 4.6 Mass Transfer (moving material)

Creatures move mass in/out of themselves or their environment:
- `inject_parasite(virulence, reproduction_rate)` — parasitism
- `secrete_adhesive(strength, stickiness)` — webbing, mucus
- `consume_material(type, rate, nutritional_value)` — feeding behavior
- `deposit_pheromone_mark(persistence, intensity)` — territorial marking

**Parameter derivation**: reproduction_rate = ch[CHEMICAL_OUTPUT] * ch[REPRODUCTION_DRIVE].

### 4.7 Energy Modulation (environmental chemistry/physics)

Creatures alter their immediate environment:
- `modulate_temperature(delta, radius, duration)` — heating/cooling
- `modulate_pH(delta, radius, duration)` — acidification/alkalization
- `modulate_osmotic_pressure(delta, radius)` — water manipulation
- `emit_photons(wavelength, intensity)` — bioluminescence (alternative to signal_emission)

**Parameter derivation**: delta = ch[THERMAL_OUTPUT] * 50, radius = ch[MASS_DENSITY] * 0.2.

### 4.8 Bond Formation (building relationships/attachments)

Creatures form or strengthen bonds with other entities:
- `form_host_attachment(host_compatibility, strength, duration)` — parasitism anchor
- `form_symbiotic_bond(partner_type, benefit_ratio)` — mutualism
- `form_pack_bond(group_cohesion, coordination_strength)` — social bonding
- `form_reproductive_bond(partner_gender, fertility_bonus)` — mating bond

**Parameter derivation**: group_cohesion = ch[SOCIAL_DRIVE] * 0.6, coordination_strength = ch[NEURAL_SPEED] * 0.8.

---

## 5. The Three Operators: Sources of Novelty

The interpreter realizes emergence through three operators that produce novel primitive-effect sets without hand-authored content. All three are deterministic and registry-driven.

### 5.0a Operator A: Emergent Primitive Combinations (Threshold Composition)

**Signature:**
```
EvaluateThresholdHooks(
    global_channels: list<float>,
    body_map: list<BodyRegion>,
    environment: Environment,
    channel_registry: ChannelRegistry,
    primitive_registry: PrimitiveRegistry,
    hook_registry: dict<string, CompositionHook>
) -> list<PrimitiveEffect>
```

**Formal definition:**
A threshold hook fires when multiple channels jointly cross a combined threshold. When a hook fires, the interpreter emits the primitives listed in the hook's `emits` field, with parameters derived from the trigger channel values.

**Example:**
```
CompositionHook {
    id: "echolocation_threshold"
    kind: Threshold
    channel_ids: [AUDITORY_SENSITIVITY, VOCAL_MODULATION, SPATIAL_COGNITION]
    threshold_values: [0.6, 0.5, 0.7]
    
    // NEW: Which primitives fire when all channels exceed thresholds?
    emits: [
        {
            primitive_id: "emit_acoustic_pulse",
            parameter_mapping: {
                "range": "ch[VOCAL_MODULATION] * 8",
                "frequency": "ch[AUDITORY_SENSITIVITY] * 100",
                "intensity": "sqrt(ch[VOCAL_MODULATION])"
            }
        },
        {
            primitive_id: "receive_acoustic_signal",
            parameter_mapping: {
                "sensitivity": "ch[AUDITORY_SENSITIVITY] * 0.8",
                "resolution": "ch[SPATIAL_COGNITION] * 0.5",
                "frequency_range": "[ch[AUDITORY_SENSITIVITY] * 50, ch[AUDITORY_SENSITIVITY] * 500]"
            }
        },
        {
            primitive_id: "spatial_integrate",
            parameter_mapping: {
                "resolution": "ch[SPATIAL_COGNITION] * 0.3",
                "range": "ch[VOCAL_MODULATION] * 5"
            }
        }
    ]
}
```

**Pseudocode:**
```
function evaluate_threshold_hooks(
    resolved_phenotype: ResolvedPhenotype,
    channel_registry: ChannelRegistry,
    primitive_registry: PrimitiveRegistry,
    hook_registry: dict<string, CompositionHook>,
    primitive_cache: dict
) -> list<PrimitiveEffect>:
    
    emitted_primitives = []
    
    for (hook_id, hook) in hook_registry:
        // Skip if environment doesn't afford this hook
        if hook.expression_conditions != null:
            if not evaluate_expression_condition(hook.expression_conditions, resolved_phenotype.environment):
                continue
        
        // Check if all channels exist in registry
        channel_ids_valid = true
        for channel_id in hook.channel_ids:
            if channel_id not in channel_registry.active_channels:
                channel_ids_valid = false
                break
        
        if not channel_ids_valid:
            continue  // Skip genesis-dependent hooks until channels exist
        
        // Evaluate threshold condition
        threshold_met = true
        channel_values = []
        for i, channel_id in enumerate(hook.channel_ids):
            ch_val = resolved_phenotype.global_channels[channel_id]
            threshold = hook.threshold_values[i]
            if ch_val < threshold:
                threshold_met = false
                break
            channel_values.append(ch_val)
        
        if threshold_met:
            // Hook fires: emit all primitives in hook.emits
            for primitive_spec in hook.emits:
                primitive_id = primitive_spec.primitive_id
                
                // Load manifest
                manifest = primitive_registry.manifests[primitive_id]
                
                // Evaluate parameter expressions
                params = {}
                for (param_name, expression) in primitive_spec.parameter_mapping:
                    params[param_name] = evaluate_expression(expression, resolved_phenotype.global_channels)
                
                // Instantiate primitive effect
                effect = PrimitiveEffect {
                    id: unique_id(),
                    primitive_id: primitive_id,
                    category: manifest.category,
                    parameters: params,
                    trigger_channels: hook.channel_ids,
                    trigger_values: channel_values,
                    body_region: infer_primary_region(hook),  // which region emits this
                    activation_cost: manifest.cost_function(params)
                }
                
                emitted_primitives.append(effect)
    
    return emitted_primitives
```

**Combinatorial explosion bounding:**
- Only evaluate hooks whose participating channels are declared in at least one rule's matcher.
- Cache invalidation: When a new paralog channel is created (genesis), recompute hooks involving that channel.
- Max hooks evaluated per tick: soft limit (warn if >50 active hooks per creature).
- Hook registry: emergent primitives are stored in primitive_registry, not invented ad hoc.

---

### 5.0b Operator B: Channel Genesis (Paralog Emergence)

**Signature:**
```
HandleChannelGenesis(
    paralog_event: GeneParalogEvent,
    channel_registry: ChannelRegistry,
    hook_registry: dict<string, CompositionHook>
) -> (new_channel_id: int, composition_hooks_invalidated: list<string>)
```

**Formal definition:**
When gene duplication produces a paralog channel, the new channel is added to the registry with an initial manifest identical to its progenitor. Over time, mutations alter the paralog's manifest. The interpreter treats the paralog as a distinct channel for composition purposes. Sister hooks are cloned and their parameter expressions updated to reference the new channel.

**Example:**
```
// Base channel exists:
AUDITORY_SENSITIVITY_A (id=5)

// Gene duplication event fires:
paralog_event = GeneParalogEvent {
    source_channel_id: 5,
    new_channel_id: 21,
    mutation_tick: 10000,
    initial_divergence: 0.1  // 10% coefficient drift
}

// Interpreter actions:
1. Add channel 21 to channel_registry
2. Clone all hooks containing channel 5 → new hooks with channel 21
3. Mark composition_hooks[21] as "pending divergence"
4. Next generation: mutations can independently tune channel 21's manifests and parameter expressions
```

**Pseudocode:**
```
function handle_channel_genesis(
    paralog_event: GeneParalogEvent,
    channel_registry: ChannelRegistry,
    hook_registry: dict<string, CompositionHook>
) -> (int, list<string>):
    
    source_id = paralog_event.source_channel_id
    new_id = paralog_event.new_channel_id
    
    // 1. Register new channel
    new_channel = channel_registry.channels[source_id].clone()
    new_channel.id = new_id
    new_channel.progenitor_id = source_id
    new_channel.divergence_magnitude = paralog_event.initial_divergence
    channel_registry.active_channels[new_id] = new_channel
    
    // 2. Clone all hooks referencing source_id
    invalidated_hooks = []
    for (hook_id, hook) in hook_registry:
        if source_id in hook.channel_ids:
            sister_hook = hook.clone()
            sister_hook.id = unique_id()
            sister_hook.cache_key = hook.cache_key + "_paralog_" + str(new_id)
            
            // Replace source_id with new_id in channel list
            idx = sister_hook.channel_ids.index(source_id)
            sister_hook.channel_ids[idx] = new_id
            
            // Update parameter expressions to reference new channel
            // e.g., "ch[5] * 8" becomes "ch[21] * 8"
            for primitive_spec in sister_hook.emits:
                for (param_name, expr) in primitive_spec.parameter_mapping:
                    sister_hook.emits[param_name].expression = 
                        expr.replace("ch[" + str(source_id) + "]", "ch[" + str(new_id) + "]")
            
            hook_registry[sister_hook.id] = sister_hook
            invalidated_hooks.append(sister_hook.cache_key)
    
    // 3. Mark divergence state
    channel_registry.divergence_state[new_id] = {
        "progenitor_id": source_id,
        "time_to_full_divergence": 500,
        "created_at_generation": current_generation
    }
    
    return (new_id, invalidated_hooks)
```

**Mutability contract:**
- Paralog channels are mutable: their coefficients and parameter expressions can drift independently.
- Sister hooks inherit the parent hook's `emits` list but can diverge.
- Composition hooks are lazily re-evaluated: on each tick, check if all participating channels exist; skip if not yet diverged.

---

### 5.0c Operator C: Environmental Affordances

**Signature:**
```
EvaluateEnvironmentalAffordances(
    resolved_phenotype: ResolvedPhenotype,
    hook_registry: dict<string, CompositionHook>
) -> list<string>  // active hook IDs
```

**Formal definition:**
The same genome emits different primitives in different environments. Expression conditions on hooks gate their activation. A hook only fires if the environment affords it.

**Example:**
```
CompositionHook {
    id: "nocturnal_echolocation",
    kind: Threshold,
    channel_ids: [AUDITORY_SENSITIVITY, VOCAL_MODULATION, SPATIAL_COGNITION],
    threshold_values: [0.6, 0.5, 0.7],
    expression_conditions: {
        light_level_range: [0.0, 0.3]  // only in darkness
    },
    emits: [
        { primitive_id: "emit_acoustic_pulse", ... },
        { primitive_id: "receive_acoustic_signal", ... },
        { primitive_id: "spatial_integrate", ... }
    ]
}

// Genome has AUDITORY=0.7, VOCAL=0.6, SPATIAL=0.8 (would fire echolocation)
// In daytime (light=0.9): expression_conditions block hook → NO primitives emitted
// At night (light=0.15): expression_conditions allow hook → echolocation primitives emitted
```

**Pseudocode:**
```
function filter_hooks_by_affordances(
    resolved_phenotype: ResolvedPhenotype,
    hook_registry: dict<string, CompositionHook>
) -> list<string>:
    
    active_hooks = []
    environment = resolved_phenotype.environment
    
    for (hook_id, hook) in hook_registry:
        if hook.expression_conditions != null:
            if not evaluate_expression_condition(hook.expression_conditions, environment):
                continue
        
        active_hooks.append(hook_id)
    
    return active_hooks
```

**Caching and performance:**
- Cache affordance evaluations per biome/season/density trio.
- Invalidate cache on seasonal transitions or major ecological shifts.
- Affordance predicates are precompiled to bitmaps for O(1) lookup.

---

## 6. Execution Flow: Six Stages (with Scale-Band Filtering)

The interpreter executes six integrated stages, now outputting only primitive effects and stats/visuals/behavior derived from them. **Stage 1A performs scale-band filtering before stat resolution.**

### 6.0 Stage 1A: Scale-Band Filtering

**Purpose**: Gate channel expression by creature body mass. Channels with `scale_band` constraints in their manifests are dormant outside their valid mass range. Channels without `scale_band` constraints are expressible at all scales.

**Critical for micro-scale pathogens**: Host-attachment, antigen, and other micro-only channels (scale_band [0, 0.001] kg) remain dormant in macro creatures (scale_band [1.0, inf] kg), and vice versa.

```
function apply_scale_band_filtering(
    resolved_phenotype: ResolvedPhenotype,
    channel_registry: ChannelRegistry
) -> list<float>:
    
    filtered_channels = resolved_phenotype.global_channels.clone()
    creature_body_mass_kg = resolved_phenotype.creature_body_mass_kg
    
    for channel_id in channel_registry.active_channels:
        channel_manifest = channel_registry.manifests[channel_id]

        if channel_manifest.scale_band == null:
            // No scale_band constraint; expressible at all scales
            continue

        min_kg, max_kg = channel_manifest.scale_band
        out_of_band = creature_body_mass_kg < min_kg or creature_body_mass_kg > max_kg

        if out_of_band:
            // Creature's mass is outside this channel's valid range
            filtered_channels[channel_id] = 0.0  // Channel is dormant
        // else: within range; channel expresses normally

        // Per-body-site channels: filter each site independently when
        // out-of-band. Uses the same top-level scale_band bounds — no
        // per-site scale bands exist in the schema.
        if channel_manifest.body_site_applicable and out_of_band:
            for body_region in resolved_phenotype.body_map:
                body_region.channel_amplitudes[channel_id] = 0.0

    return filtered_channels
```

**Key behavior**:
- **Default (no scale_band)**: Channel is expressible at all scales. Regulatory channels like `immune_response_baseline` have no scale_band and express in both micro and macro creatures.
- **Micro-only** (e.g., `host_attachment`): scale_band = [1e-15, 1e-3] kg. Expressible only in pathogens, parasites, symbionts under ~1 gram.
- **Macro-only** (e.g., `large_neural_integration`): scale_band = [1.0, ∞] kg. Dormant in micro-scale pathogens.
- **Both scales** (e.g., `metabolic_rate`): scale_band = [1e-15, ∞] kg or no constraint. Active everywhere.

**This stage executes BEFORE Stage 1 (Stat Resolver)** so that dormant channels contribute zero to stat calculations and composition hooks. Filtering is per-channel; body-site channels are filtered per-site-per-channel.

**Authoritative `scale_band` field**: the top-level `ChannelManifest.scale_band` is the only source this stage consults. Any `ExpressionCondition::ScaleBand { min_kg, max_kg }` variants that appear on a composition hook's `expression_conditions` list are *additional runtime gates* evaluated later in Stage 2A (affordance filter) — not a second definition of the channel's scale band.

### 6.0B. Body-Site Aggregation (Issue #9)

**Purpose**: Distribute channel values to body-region-specific primitive effects when channels are marked `body_site_applicable=true`. For channels without this flag, values remain global.

**Key Rule**:
- **Body-Site Channels** (e.g., bite_force, claw_sharpness): Channels with `body_site_applicable=true` generate **per-site PrimitiveEffect instances**. A single composition hook referencing `apply_bite_force` on `limb_front_left` vs. `limb_front_right` creates separate PrimitiveEffect objects, each with `site_id` metadata.
- **Global Channels** (e.g., metabolic_rate): Channels without body_site_applicable produce a single global PrimitiveEffect. Value is broadcast to all sites if needed.

**Composition Hook Cross-Body-Site Behavior**:
When a composition hook references multiple channels:
- **Both body-site**: Hook fires per-site; each site receives its own pair of channel values.
- **One body-site, one global**: Hook fires per-site; global channel value is broadcast to all invocations (e.g., `apply_bite_force` uses per-site kinetic_force × global damage_multiplier).
- **Both global**: Hook fires once, globally.

**Aggregation Function for Combat & UI Summary**:

```
function aggregate_to_global(
    channel_id: string,
    strategy: enum{max, mean, sum},
    body_site_values: dict<site_id, float>
) -> float:
    
    match strategy:
        max:   return max(body_site_values.values())
        mean:  return mean(body_site_values.values())
        sum:   return sum(body_site_values.values())
    
    // Example: kinetic_force uses 'max' (strongest limb's impact)
    //          claw_sharpness uses 'mean' (average across all claws)
    //          surface_area uses 'sum' (total area)
```

Aggregation strategy is defined per-channel in the channel manifest. When the UI or combat system needs a single global value for a body-site channel, it invokes `aggregate_to_global()` with the appropriate strategy.

---

### 6.1 Stage 1: Stat Resolver

Converts global channels (post-filtering) → `StatBlock`. Each stat is computed as a weighted sum of filtered channels with optional nonlinear transforms. **Dormant channels (scale_band-filtered to 0.0) contribute zero to stats.**

### 6.1 Stage 1: Stat Resolver

Converts global channels → `StatBlock`. Each stat is computed as a weighted sum of channels with optional nonlinear transforms. **This stage is unchanged from prior versions.**

```
function resolve_stats(resolved_phenotype: ResolvedPhenotype) -> StatBlock:
    stats = new StatBlock()
    
    for stat_rule in STAT_RULES:
        value = stat_rule.base_value
        for term in stat_rule.terms:
            channel_val = resolved_phenotype.global_channels[term.channel]
            adjusted = apply_transform(channel_val, term.transform, term.threshold)
            value += term.weight * adjusted
        
        stats[stat_rule.output_stat] = clamp(value, stat_rule.clamp.min, stat_rule.clamp.max)
    
    return stats
```

### 6.2 Stage 2: Primitive Effect Emission

Evaluates threshold composition hooks and emits the primitive effects they declare.

```
function emit_primitives(
    resolved_phenotype: ResolvedPhenotype,
    channel_registry: ChannelRegistry,
    primitive_registry: PrimitiveRegistry,
    hook_registry: dict<string, CompositionHook>,
    environment: Environment
) -> list<PrimitiveEffect>:
    
    all_primitives = []
    
    // STAGE 2A: Filter hooks by environmental affordances
    active_hooks = filter_hooks_by_affordances(resolved_phenotype, hook_registry)
    
    // STAGE 2B: Evaluate active hooks and emit primitives
    for hook_id in active_hooks:
        hook = hook_registry[hook_id]
        
        // Check if all channels exist in registry
        channel_ids_valid = true
        for channel_id in hook.channel_ids:
            if channel_id not in channel_registry.active_channels:
                channel_ids_valid = false
                break
        
        if not channel_ids_valid:
            continue
        
        // Evaluate threshold condition
        threshold_met = true
        channel_values = []
        for i, channel_id in enumerate(hook.channel_ids):
            ch_val = resolved_phenotype.global_channels[channel_id]
            threshold = hook.threshold_values[i]
            
            // CRITICAL: Dormant channels are Q32.32::ZERO; no error state
            // If any operand is zero, this threshold automatically fails (total evaluation)
            if ch_val == Q32.32::ZERO or ch_val < threshold:
                threshold_met = false
                break
            channel_values.append(ch_val)
        
        if threshold_met:
            // Hook fires: emit all primitives in hook.emits
            for primitive_spec in hook.emits:
                primitive_id = primitive_spec.primitive_id
                manifest = primitive_registry.manifests[primitive_id]
                
                // Evaluate parameter expressions
                params = {}
                for (param_name, expression) in primitive_spec.parameter_mapping:
                    // Expression evaluation is total: dormant channels (value=0) propagate zero
                    params[param_name] = evaluate_expression(expression, resolved_phenotype.global_channels)
                
                // Instantiate primitive effect
                effect = PrimitiveEffect {
                    id: unique_id(),
                    primitive_id: primitive_id,
                    category: manifest.category,
                    parameters: params,
                    trigger_channels: hook.channel_ids,
                    trigger_values: channel_values,
                    body_region: resolve_body_region(hook, resolved_phenotype),
                    activation_cost: manifest.cost_function(params)
                }
                
                all_primitives.append(effect)
    
    return all_primitives
```

### 6.2B Stage 2B: Primitive Deduplication & Merging

**Critical Rule**: Multiple composition hooks may emit the same `primitive_id` from different channels or body sites. These must be **deduplicated and merged** using a deterministic strategy.

```
function deduplicate_and_merge_primitives(
    all_primitives: list<PrimitiveEffect>
) -> dict<(primitive_id, site_id), MergedPrimitive>:
    
    // Group primitives by (primitive_id, site_id)
    grouped = defaultdict<(primitive_id, site_id), list<PrimitiveEffect>>()
    
    for effect in all_primitives:
        key = (effect.primitive_id, effect.body_region.id)
        grouped[key].append(effect)
    
    // Merge each group using manifest-specified merge_strategy
    merged_primitives = {}
    for (key, effects) in grouped.items():
        primitive_id, site_id = key
        
        if len(effects) == 1:
            // No merge needed; single emission
            merged_primitives[key] = effects[0]
        else:
            // Multiple hooks emitted the same primitive; merge parameters
            merged = merge_primitive_group(
                effects,
                primitive_registry.manifests[primitive_id],
                key
            )
            merged_primitives[key] = merged
    
    return merged_primitives


function merge_primitive_group(
    effects: list<PrimitiveEffect>,
    manifest: PrimitiveManifest,
    key: (primitive_id, site_id)
) -> MergedPrimitive:
    
    // Manifest declares merge_strategy per parameter
    merged_params = {}
    
    for param_name in manifest.parameter_schema.keys():
        merge_strategy = manifest.merge_strategy.get(param_name, "max")
        
        param_values = [e.parameters[param_name] for e in effects if param_name in e.parameters]
        
        if not param_values:
            continue
        
        match merge_strategy:
            case "sum":
                // Additive quantities (force, magnitude, intensity)
                merged_params[param_name] = sum(param_values)
            
            case "max":
                // Intensity-like quantities (concentration, frequency)
                merged_params[param_name] = max(param_values)
            
            case "mean":
                // Averaging behavior
                merged_params[param_name] = mean(param_values)
            
            case "union":
                // Set-valued (tags, molecular types)
                merged_params[param_name] = union(param_values)
    
    // Merged effect
    merged = MergedPrimitive {
        primitive_id: effects[0].primitive_id,
        parameters: merged_params,
        source_hooks: [e.trigger_channels for e in effects],  // Provenance
        activation_cost: sum(e.activation_cost for e in effects),  // Sum costs
        body_region: effects[0].body_region  // All share same site
    }
    
    return merged
```

**Example**:
```
CompositionHook A: auditory_sensitivity + vocal_modulation → emit_acoustic_pulse(range=0.6, intensity=0.4)
CompositionHook B: spatial_cognition → emit_acoustic_pulse(range=0.3, intensity=0.8)

Both hooks fire in same creature/site.

Manifest declares:
  merge_strategy: {
    "range": "max",        // Use max (0.6)
    "intensity": "max"     // Use max (0.8)
  }

Result: Single merged emit_acoustic_pulse(range=0.6, intensity=0.8)
```

**Manifest Declaration**:
```json
{
  "id": "emit_acoustic_pulse",
  "parameter_schema": {
    "range": { "type": "float", "range": [0.5, 20.0] },
    "frequency": { "type": "float", "range": [50, 500] },
    "intensity": { "type": "float", "range": [0.0, 1.0] }
  },
  "merge_strategy": {
    "range": "max",
    "frequency": "max",
    "intensity": "max"
  }
}
```
```

### 6.3 Stage 3: Behavior Compilation

Takes stats and primitive effects → produces a priority-ordered `BehaviorTree`. Behavior is inferred from which primitives the creature can emit.

```
function compile_behavior(
    stats: StatBlock,
    primitives: list<PrimitiveEffect>
) -> BehaviorTree:
    
    tree = new BehaviorTree()
    
    // Analyze which primitive categories are available
    has_signal_emission = any(p.category == signal_emission for p in primitives)
    has_force_application = any(p.category == force_application for p in primitives)
    has_signal_reception = any(p.category == signal_reception for p in primitives)
    has_state_induction = any(p.category == state_induction for p in primitives)
    has_spatial_integration = any(p.category == spatial_integration for p in primitives)
    
    // SURVIVAL PRIORITY (always highest)
    tree.add_priority(
        Priority.CRITICAL,
        SurvivalBehavior(
            health_threshold: 0.2,
            has_defensive_primitives: (has_state_induction or has_force_application),
            flee_speed_bonus: stats.move_speed * 0.3
        )
    )
    
    // FLEE THRESHOLD (derived from stats and primitives)
    flee_threshold = compute_flee_threshold(stats, primitives)
    tree.add_priority(
        Priority.HIGH,
        FleeBehavior(
            trigger_threshold: flee_threshold
        )
    )
    
    // COMBAT (if has force application primitives)
    if has_force_application:
        tree.add_priority(
            Priority.MEDIUM,
            CombatBehavior(
                melee_enabled: any(p.primitive_id.contains("bite") for p in primitives),
                ranged_enabled: any(p.primitive_id.contains("discharge") for p in primitives),
                preferred_range: infer_preferred_range(primitives, stats)
            )
        )
    
    // SENSORY BEHAVIORS (if has signal reception/spatial integration)
    if has_signal_reception or has_spatial_integration:
        tree.add_priority(
            Priority.MEDIUM_LOW,
            SensoryBehavior(
                active_sensing: has_signal_emission,
                passive_sensing: has_signal_reception
            )
        )
    
    // FORAGE / PATROL (default, lowest priority)
    tree.add_priority(
        Priority.LOWEST,
        PatrolBehavior(
            perception_range: stats.perception_range
        )
    )
    
    return tree
```

### 6.4 Stage 4: Visual Directive Generator

Produces `VisualDirective` tuples that the procgen pipeline consumes. Directives are inferred from primitive effects or specified in primitive manifests.

```
function generate_visual_directives(
    resolved_phenotype: ResolvedPhenotype,
    primitives: list<PrimitiveEffect>,
    primitive_registry: PrimitiveRegistry
) -> list<VisualDirective>:
    
    directives = []
    
    for primitive in primitives:
        manifest = primitive_registry.manifests[primitive.primitive_id]
        
        // Use the manifest's observable_signature to generate directives
        if manifest.observable_signature.visual != null:
            directive = VisualDirective {
                body_region: primitive.body_region,
                directive_type: infer_directive_type(manifest.observable_signature.visual),
                parameters: {
                    animation: manifest.observable_signature.visual,
                    intensity: primitive.parameters.get("intensity", 1.0),
                    scale: infer_scale_from_parameters(primitive.parameters)
                }
            }
            directives.append(directive)
    
    return directives
```

### 6.5 Stage 5: Primitive Effect Instantiation & Cost Resolution

**Critical: This is the ONLY stage where primitives are instantiated.** All parameter binding, cost derivation, and cooldown hints happen here. No hardcoded ability inference occurs elsewhere in the interpreter.

Takes triggered composition hooks and instantiates fully-parameterized `PrimitiveEffect` objects with costs and cooldowns derived from primitive manifests.

```
function instantiate_primitives_from_hooks(
    resolved_phenotype: ResolvedPhenotype,
    channel_registry: ChannelRegistry,
    primitive_registry: PrimitiveRegistry,
    triggered_hooks: list<CompositionHook>
) -> Set<PrimitiveEffect>:
    
    instantiated_effects = Set<PrimitiveEffect>()
    
    for hook in triggered_hooks:
        // For each primitive declared in the hook's emits field
        for primitive_spec in hook.emits:
            primitive_id = primitive_spec.primitive_id
            
            // Verify primitive exists in registry
            if primitive_id not in primitive_registry.manifests:
                log_warning("Primitive not in registry: " + primitive_id)
                continue
            
            manifest = primitive_registry.manifests[primitive_id]
            
            // === PARAMETER BINDING ===
            // Evaluate all parameter expressions from primitive_spec.parameter_mapping
            // against current channel values
            parameters = {}
            for (param_name, expression) in primitive_spec.parameter_mapping.items():
                // Expression uses channel IDs and math operators
                // e.g., "kinetic_force * 0.8 + 0.2" → evaluate against global_channels
                param_value = evaluate_expression(
                    expression,
                    resolved_phenotype.global_channels,
                    resolved_phenotype.body_map
                )
                
                // Clamp parameter to manifest's parameter_schema constraints
                schema = manifest.parameter_schema[param_name]
                param_value = clamp(param_value, schema.min, schema.max)
                
                parameters[param_name] = param_value
            
            // === COST DERIVATION ===
            // Cost comes from primitive manifest's cost_function, NOT hardcoded
            // cost_function takes parameters as input
            activation_cost = manifest.cost_function(parameters)
            
            // === COOLDOWN HINT ===
            // Cooldown also from manifest recovery_time, parameterized if needed
            cooldown_hint = manifest.recovery_time
            if manifest.cooldown_is_parametric:
                // Some primitives have cooldowns that scale with parameters
                cooldown_hint = manifest.compute_cooldown(parameters)
            
            // === INSTANTIATE PRIMITIVE EFFECT ===
            effect = PrimitiveEffect {
                id: generate_unique_id(),
                primitive_id: primitive_id,
                category: manifest.category,
                parameters: parameters,
                cost: activation_cost,
                cooldown_hint: cooldown_hint,
                trigger_channels: hook.channel_ids,
                trigger_values: [resolved_phenotype.global_channels[ch] for ch in hook.channel_ids],
                body_region: resolve_body_region(hook, resolved_phenotype),
                source_channels: hook.channel_ids
            }
            
            instantiated_effects.add(effect)
    
    return instantiated_effects
```

**Parameter Derivation Strategy:**

Parameters are derived from channel values via expressions in `primitive_spec.parameter_mapping`:
- **Linear mapping**: `strength = kinetic_force * 2.0` → channel value multiplied by scalar
- **Range clamping**: Parameter value is clamped to [schema.min, schema.max] before instantiation
- **Multi-channel composition**: `potency = (chemical_output + chemical_resistance) / 2.0` → weighted average or sum of channels
- **Nonlinear transforms**: `magnitude = sqrt(kinetic_force * density)` → arbitrary math expressions supported

**Cost and Cooldown Contract:**

The primitive manifest's `cost_function` and `recovery_time` are the ONLY sources of cost/cooldown data. Combat systems must NOT hardcode cost values per primitive_id. If a primitive's cost needs to scale with parameters, the manifest declares this:

```
{
  "primitive_id": "emit_acoustic_pulse",
  "cost_function": "0.2 * intensity + 0.1",  // cost scales with intensity parameter
  "recovery_time": 2.0,                       // base cooldown in turns
  "cooldown_is_parametric": false
}
```

Or with parameterized cooldown:

```
{
  "primitive_id": "inject_toxin",
  "cost_function": "0.5 * volume",            // stamina cost from volume
  "recovery_time": 3.0,                       // base cooldown
  "cooldown_is_parametric": true,
  "compute_cooldown": "2.0 + (potency / 4.0)" // cooldown increases with potency
}
```

---

### 6.6 Downstream: Interaction Handler Generation (Behavior)

After all primitive effects are instantiated, downstream systems (combat, behavior, UI) consume the PrimitiveEffect set:

```
function generate_combat_actions_from_primitives(
    primitives: Set<PrimitiveEffect>,
    creature_stats: StatBlock
) -> list<CombatAction>:
    
    // Combat system synthesizes CombatAction structs from primitives
    // (This is in System 06; interpreter output ends at Stage 5)
    // 
    // For each force_application primitive:
    //   - primitive_id → which attack type
    //   - parameters → damage/effect magnitude
    //   - cost → stamina_cost
    //   - cooldown_hint → action cooldown
    //   - This is purely derived from PrimitiveEffect; never from ability labels
```

---

## 7. What the Interpreter Does Not Do

**Critical boundaries:**

1. **The interpreter does NOT emit named abilities.** No "echolocation", "pack_hunting_bond", "bioluminescence" strings appear in interpreter output. Only `PrimitiveEffect` objects.

2. **The interpreter does NOT assign labels or names.** That is the Chronicler's job (System 09). The Chronicler reads the set of primitives emitted and labels it: `{emit_acoustic_pulse, receive_acoustic_signal, spatial_integrate} → "echolocation"`.

3. **The interpreter does NOT have hardcoded ability inference branches.** All primitive emission flows through `composition_hooks[].emits` field. No if-else chains like "if channels > threshold then activate_ability(name)" exist in interpreter logic. (Invariant 3.9)

4. **The interpreter does NOT hardcode cost or cooldown values.** Stamina costs and action cooldowns come from primitive manifest's `cost_function` and `recovery_time` fields, parameterized by instantiated effect parameters. Stage 5 reads manifests; downstream systems never invent cost values.

5. **The interpreter does NOT execute combat logic.** Combat reads `PrimitiveEffect` objects and decides how to apply them. The interpreter only produces the effects.

6. **The interpreter does NOT build the UI.** The UI reads Chronicler-assigned labels, not primitives. (Downstream decision: UI reads Chronicler; game mechanics read Primitives.)

7. **The interpreter does NOT handle stat modifications from named abilities.** Stats come from Stage 1. Primitives are independent outputs. If a primitive should modify a stat, that's a downstream concern (behavior tree or combat system) that reads the primitive and applies the stat bonus.

8. **The interpreter does NOT generate new primitives at runtime.** Primitives are authored or created via paralog genesis (Option B). Primitive genesis (Option C) is future work and will be specified separately.

---

## 8. Primitive Registry Mutability and Determinism

The primitive registry is part of the save state. Like the channel registry, it is:

- **Unbounded in size**: New primitives can be created via genesis events (future work), but the count is deterministic.
- **Fully serializable**: Registry snapshots are stored as JSON or binary alongside channel registry snapshots.
- **Deterministic**: Given a save state (genome + channel_registry + primitive_registry + hook_registry), the same interpreter invocation always produces the same primitive-effect set.

**Save contract:**
```
SaveState {
    genotype_id: uint64,
    channel_registry: ChannelRegistry,    // dynamically mutable
    primitive_registry: PrimitiveRegistry, // dynamically mutable
    hook_registry: dict<string, CompositionHook>,  // dynamically mutable
    timestamp: uint64,
    environment_snapshot: Environment
}
```

---

## 9. Cross-System Hooks

The interpreter's output feeds into:

1. **Combat System (System 06)**: `PrimitiveEffect` objects are consumed by combat directly. Combat reads `{apply_bite_force(magnitude=8), induce_fatigue(...)}` and synthesizes `CombatAction` structs at encounter init. Combat NEVER reads ability names; all parameters, costs, and cooldowns come from PrimitiveEffect manifests.

2. **Chronicler (System 09)**: Reads primitive-effect sets and assigns semantic labels. `{emit_acoustic_pulse, receive_acoustic_signal, spatial_integrate} → "echolocation"`. These labels appear in creature descriptions and dialogue.

3. **UI/Dialogue (System ???)**: Reads Chronicler-assigned labels, not primitives. Displays "This creature uses echolocation" based on Chronicler output.

4. **Procgen Visual (Doc 10)**: Reads visual directives derived from primitives. Stage 4 output feeds into procgen blueprint generation.

5. **Crafting System (Doc 05)**: Harvested materials come from Stage 5 interaction handlers, parameterized by the same channels that shaped the creature.

6. **Behavior/AI System**: BehaviorTree output (Stage 3) drives monster decision-making. Tree is built from primitive-effect capabilities.

---

## 10. Tradeoff Matrix: Primitives vs. Named Abilities

| Dimension | Primitives First | Named Abilities | Hybrid (This Design) |
|-----------|-----|-----|--------|
| **Interpreter Output** | Atomic effects only | Named ability modules | Primitives + Chronicler labels |
| **Naming Authority** | Downstream (Chronicler) | Interpreter | Chronicler owns labels, primitives own semantics |
| **Game Mechanics Reading** | PrimitiveEffect directly | Named ability objects | PrimitiveEffect directly; UI reads Chronicler labels |
| **New Primitive Addition** | Registry + future genesis operators | Code change + rule authoring | Registry mutation (deterministic) |
| **Composition Hook Clarity** | Direct: hook → emits primitives | Indirect: hook → emits named ability → contains primitives | Direct: hook → emits primitives |
| **UI Decoupling** | UI must know primitive semantics | UI tightly coupled to ability names | UI decoupled: reads Chronicler labels |
| **Complexity** | Simpler interpreter, Chronicler overhead | Simpler overall, less flexibility | Moderate: two systems with clear boundary |

**Winner**: Primitives-first hybrid. This design:
- Keeps the interpreter simple (output is atoms, not molecules).
- Decouples UI from game mechanics (UI reads Chronicler; mechanics read Primitives).
- Enables future primitive genesis (primitives are extensible; named abilities are not).
- Maintains determinism (primitive registry is fully serializable and deterministic).

---

## 11. Formal Commitment: Invariant 3.8 (Primitive Foundation)

**The interpreter is a primitive-effect generator, not a named-ability factory.**

```
primitive_effects = Interpreter(genome, environment, channel_registry, primitive_registry)
```

Where:
- **genome** = channel values [0, 1] × body map (evolved by selection)
- **environment** = biome, season, light, temperature, density, time (from world state)
- **channel_registry** = dynamically mutable set of channels and composition hooks
- **primitive_registry** = dynamically mutable set of primitive manifests

**Guarantees:**
1. Same genome + same environment + same registries → identical primitive-effect set (deterministic).
2. Variance across populations = variance in genotypes and/or registry states, never variance in interpretation logic.
3. Primitives are **atomic** (cannot be decomposed further by the interpreter) and **elementary** (named only by downstream systems).
4. Emergence is automatic: design the registry once, evolution populates the phenotype space.

**Core claim:** The interpreter is the formal substrate for all evolved phenotypes. It is a pure function with bounded, deterministic outputs. Meaning is assigned downstream, maintaining coherence across game systems without tight coupling.

---

## 12. Environmental Affordances in Depth

The interpreter evaluates expression conditions in order on each tick. This means phenotypes shift dynamically as the creature moves, seasons change, or population density rises. **Crucially, primitive effects respond to environmental gating, not just stats.**

### 12a. Predicate Language

```
ExpressionCondition {
    biome_flags: list<BiomeTag> or null
        // Examples: AQUATIC, ARBOREAL, VOLCANIC, CORAL_REEF, UNDERGROUND
    
    season: enum or null
        // SPRING, SUMMER, FALL, WINTER
    
    developmental_stage: enum or null
        // JUVENILE, ADULT, ELDERLY
    
    light_level_range: [float, float] or null
        // [0.0, 0.3] = nocturnal
        // [0.7, 1.0] = diurnal
    
    temperature_range: [float, float] or null
        // Celsius
    
    population_density_range: [float, float] or null
        // creatures per 100 sq units
    
    must_have_channels: list<int> or null
        // Require these channels to exist in the registry
}
```

### 12b. Evaluation and Caching

- **Affordance bitmap**: Precompute biome_flag intersections as bit vectors for O(1) lookup.
- **Scale band cache**: Recompute only when local_population_density changes by >1.
- **Light level cache**: Recompute on season transition or time-of-day change.
- **Cache invalidation**: On interpreter entry, check if environment has changed; invalidate affected hook lists.

**Result:** A creature can emit completely different primitive sets in different niches. A nocturnal genome in a cave emits echolocation primitives; the same genome in daytime forest does not. Same interpreter, different outputs, no hand-authored variants.

---

## 13. Emergent Properties

1. **Primitive-driven behavior**: Creatures with high force-application primitives default to aggressive strategies. Creatures with high spatial-integration primitives default to tactical strategies. Behavior emerges without explicit rules.

2. **Predator-prey arms race**: Prey species evolve primitives for camouflage and perception. Predators evolve primitives for force application. Coevolution is mediated entirely through primitive registries.

3. **Form-function feedback loops**: A visual directive (e.g., "spiky appearance") comes from the same primitive that deals damage. Selection for damage automatically refines appearance; evolution doesn't need to specify appearance separately.

4. **Regulatory evolution**: Populations can shift how aggressively they express primitives without changing channel counts — just shifting regulatory channel values that modulate composition hook thresholds.

5. **Ecological roles**: Niches (e.g., "ambush predator") emerge from the combination of primitive capabilities without being explicitly defined.

6. **Threshold composition synergies**: Populations can independently evolve how tightly their channels must synergize. Population A: echolocation primitives fire at mild auditory + strong vocal. Population B: strict auditory requirement, loose vocal. Same primitives, different evolutionary requirement profiles.

7. **Paralog divergence**: A duplicated channel initially drives identical hooks and primitives as its progenitor. Over time, parameter expressions drift. Ancient paralogs emit completely different primitive sets.

8. **Environmental specialization**: No hand-authored "forest variant." Evolution selects for genotypes whose expression conditions match their local environment.

---

## 14. Open Calibration Knobs

1. **Primitive vocabulary size**: How many primitive manifests should exist? Currently ~20 (2-3 per category). More = finer-grained expression control.

2. **Composition hook density**: How many active composition hooks per creature? Soft cap ~50. More hooks = more potential emergent behavior.

3. **Expression condition granularity**: Biome-only vs. biome+season+light? Finer = more environmental nuance but more cache entries.

4. **Parameter expression complexity**: How complex can parameter-mapping expressions be? Currently simple (linear combinations). Could allow nonlinear functions.

5. **Primitive cost functions**: How expensive are primitives? Currently estimated per primitive. Could make cost scale with parameter values (e.g., larger range = higher cost).

6. **Affordance caching strategy**: Lazy (recompute on first use) vs. eager (recompute each tick). Lazy = fewer updates, more stale data. Eager = consistent but higher CPU.

7. **Channel genesis mutation rate**: How aggressively do sister hooks diverge from parents? Higher = faster speciation.

8. **Environmental transition latency**: Immediate effect vs. fade-in when moving between biomes? Fade-in prevents ability flickering.

---

## 15. Migration Notes v2.0

### Changes from v1 (Named Abilities)

**Change 1: Output signature**
- Old: `MonsterInstance { abilities: list<Ability> }` where Ability.name ∈ {"echolocation", "pack_hunting_bond", ...}
- New: `list<PrimitiveEffect>` where PrimitiveEffect.primitive_id ∈ primitive_registry
- Impact: Game mechanics must be refactored to read primitives instead of named abilities.

**Change 2: Composition hooks**
- Old: `CompositionHook.emerging_ability_id: string` (e.g., "echolocation")
- New: `CompositionHook.emits: list<{primitive_id, parameter_mapping}>`
- Impact: Hook definitions must now specify which primitives fire, with parameter expressions.

**Change 3: Interpreter function signature**
- Old: `phenotype = Interpreter(genome, environment, channel_registry)`
- New: `primitive_effects = Interpreter(genome, environment, channel_registry, primitive_registry)`
- Impact: Callers must pass primitive_registry.

**Change 4: Ability naming is decoupled**
- Old: Interpreter output names abilities; UI uses those names directly.
- New: Interpreter outputs primitives; Chronicler (System 09) assigns names; UI reads Chronicler labels.
- Impact: System 09 (Chronicler) must be implemented to read primitive patterns and assign semantic labels.

**Change 5: Primitive registry lifecycle**
- New responsibility: Manage primitive_registry.manifests (add/remove manifests), manage hook_registry.emits (which primitives each hook declares).
- Old: No primitive registry existed.
- Impact: Serialization code must snapshot primitive_registry alongside channel_registry.

### Downstream System Impacts

**System 06 (Combat)**
- Old: Read `Ability` objects, match by `ability.name` ("bite", "poison", etc.).
- New: Read `PrimitiveEffect` objects, match by `primitive.primitive_id` ("apply_bite_force", "inject_substance", etc.).
- Action: Refactor combat application logic to dispatch on primitive_id instead of ability_name.

**System 09 (Chronicler)** [NEW]
- Old: Did not exist.
- New: Reads primitive-effect sets, pattern-matches against known synergies, outputs semantic labels.
- Example: `{emit_acoustic_pulse, receive_acoustic_signal, spatial_integrate} → "echolocation"`.
- Action: Implement pattern matching and label assignment logic.

**System 10 (Procgen Visual)**
- Old: Reads MonsterInstance.visual_directives directly from interpreter output.
- New: No change. Stage 4 of interpreter still produces directives, now derived from primitives' observable_signatures.
- Action: Minor refactor: directives now come from primitive manifests instead of hardcoded rule outputs.

**System 08 (Evolution)**
- Old: Fitness derived from Ability.effect values and behavior outcomes.
- New: Fitness derived from PrimitiveEffect.parameters and behavior outcomes.
- Action: Update fitness calculation to read primitive parameters instead of ability stats.

**System 05 (Crafting)**
- Old: No change expected.
- New: No change. Interaction handlers still infer materials from channel profiles.

**System 14 (Calendar/Time)**
- Old: Life-stage gates controlled ability assembly.
- New: Life-stage gates control hook firing.
- Action: Update expression_conditions in hooks to include life_stage gates.

### Implementation Checklist

- [ ] Design primitive vocabulary: 8 categories, ~20 manifests total.
- [ ] Implement `PrimitiveManifest` and `PrimitiveRegistry` data structures.
- [ ] Update `CompositionHook` to include `emits: list<{primitive_id, parameter_mapping}>`.
- [ ] Implement `evaluate_expression(expression: string, channels: list<float>) -> float` for parameter evaluation.
- [ ] Refactor Stage 2 (Ability Assembler) into Stage 2 (Primitive Effect Emission).
- [ ] Refactor Stage 4 (Visual Directive Generator) to derive directives from primitive manifests.
- [ ] Implement `apply_primitive_to_target(primitive, target, context)` in interaction handlers.
- [ ] Add serialization for primitive_registry (alongside channel_registry).
- [ ] Update all example scenarios to output primitives instead of named abilities.
- [ ] Implement System 09 (Chronicler) pattern matching and label assignment.
- [ ] Refactor System 06 (Combat) to read and apply primitives.
- [ ] Test: same genome in two environments emits different primitives.
- [ ] Test: paralog channel genesis clones hooks and updates parameter expressions.
- [ ] Test: threshold composition hook fires and emits all declared primitives.

### Backward Compatibility

- Non-primitive systems (evolution, stats, visual directives, behavior trees) are largely unchanged; they operate on downstream outputs (stats, visuals, behavior), not on the interpreter's primitive output.
- Hand-authored composition hooks must be migrated: `emerging_ability_id` → `emits[{primitive_id, parameter_mapping}]`.
- Game mechanics code must be refactored to read primitives instead of abilities, but the refactor is localized to System 06.
- The Chronicler (System 09) is new and must be implemented from scratch.

---

## Appendix A: Primitive Vocabulary Reference

| Primitive | Category | Example Parameters | Observable |
|-----------|----------|-----|---------|
| `emit_acoustic_pulse` | signal_emission | range, frequency, intensity | acoustic ripple animation |
| `emit_pheromone` | signal_emission | distance, molecular_type, concentration | odor cloud particle |
| `emit_light` | signal_emission | wavelength, intensity, duration | glow effect |
| `receive_acoustic_signal` | signal_reception | sensitivity, resolution, frequency_range | (passive, no visual) |
| `detect_pheromone` | signal_reception | sensitivity, molecular_type_range | (passive) |
| `apply_bite_force` | force_application | magnitude, sharpness | impact particle, blood splatter |
| `apply_impact_force` | force_application | magnitude, radius | explosion effect |
| `induce_fatigue` | state_induction | duration, intensity | debuff indicator |
| `elevate_metabolic_rate` | state_induction | duration, intensity | energy aura |
| `inject_substance` | mass_transfer | substance_type, volume, duration | poison cloud, injection animation |
| `spatial_integrate` | spatial_integration | resolution, range | (passive, affects behavior) |
| `modulate_temperature` | energy_modulation | delta, radius, duration | heat wave or frost effect |
| `form_pack_bond` | bond_formation | group_cohesion, coordination_strength | social link animation |

---

## Appendix B: Example Trace (Refactored to Primitives)

### Scenario: Forest Creature at Night

**Input**:
```
Genome: AUDITORY_SENSITIVITY 0.65, VOCAL_MODULATION 0.55, SPATIAL_COGNITION 0.70,
         KINETIC_FORCE 0.8, LIGHT_ABSORPTION 0.7, NEURAL_SPEED 0.6, MASS_DENSITY 0.3
Environment: Biome=FOREST, Season=SUMMER, Light_Level=0.15 (night),
             Temperature=18°C, Population_Density=1.5 (sparse)
Channel Registry: Standard 20 channels
Primitive Registry: Standard manifests (20 primitives)
Hook Registry: echolocation_threshold [AUDITORY, VOCAL, SPATIAL] at [0.6, 0.5, 0.7]
```

**Stage 1 (Stat Resolver)**:
```
move_speed = 6.5 units/tick
max_health = 120 HP
perception_range = 8.0 units
neural_speed = 0.6 (for behavioral responsiveness)
```

**Stage 2A (Affordance Filtering)**:
```
echolocation_threshold.expression_conditions = { light_level_range: [0.0, 0.3] }
Light_Level = 0.15 → INSIDE [0.0, 0.3] ✓ → hook is ACTIVE
```

**Stage 2B (Threshold Composition)**:
```
echolocation_threshold channel values:
  AUDITORY_SENSITIVITY = 0.65 > 0.6 ✓
  VOCAL_MODULATION = 0.55 > 0.5 ✓
  SPATIAL_COGNITION = 0.70 > 0.7 ✓
  All thresholds met → hook FIRES

Emitted primitives:
  {
    primitive_id: "emit_acoustic_pulse",
    parameters: {
      range: 0.55 * 8 = 4.4 units,
      frequency: 0.65 * 100 = 65 Hz,
      intensity: sqrt(0.55) ≈ 0.74
    }
  },
  {
    primitive_id: "receive_acoustic_signal",
    parameters: {
      sensitivity: 0.65 * 0.8 = 0.52,
      resolution: 0.70 * 0.5 = 0.35,
      frequency_range: [32.5, 325] Hz
    }
  },
  {
    primitive_id: "spatial_integrate",
    parameters: {
      resolution: 0.70 * 0.3 = 0.21,
      range: 0.55 * 5 = 2.75 units
    }
  }
```

**Output**:
```
PrimitiveEffects: [emit_acoustic_pulse(...), receive_acoustic_signal(...), spatial_integrate(...)]
Body Region: Head (inferred)
Observable Signature: "acoustic_ripple" (from manifest)
```

**Downstream (Chronicler)**:
```
Pattern Match: {emit_acoustic_pulse, receive_acoustic_signal, spatial_integrate}
  → Matched pattern: "echolocation"
  → Label: "This creature can echolocate."
```

**Downstream (Game Mechanics)**:
```
Combat system reads apply_bite_force primitive (if present):
  magnitude = 0.8 * 10 = 8 damage
  sharpness = ??? (no structural rigidity in this hook)
  
Behavior system reads spatial_integrate primitive:
  → enables "use spatial awareness in decision-making"
  → preferred_range computed from primitive parameters
  → engage_range = 2.75 units (spatial_integrate.range)
```

**Key Insight**: The interpreter emits raw primitives. The Chronicler labels them "echolocation" for dialogue. Combat reads primitives directly to apply effects. UI shows Chronicler's label.

---

## Appendix C: Determinism Guarantee

**Claim:** Given identical inputs (genome, environment, channel_registry state, primitive_registry state), the interpreter always produces identical primitive-effect sets.

**Proof sketch:**
1. All channel values are deterministic (resolved from fixed genome).
2. All expression conditions are evaluated purely (biome tags, light level, season are deterministic).
3. All threshold evaluations are deterministic (channel_value vs. threshold_value is boolean).
4. All parameter expressions are deterministic (mathematical functions of channels with no RNG).
5. Primitive instantiation is deterministic (unique_id() can be seeded from hook_id + iteration counter).
6. Registry lookups are deterministic (primitive_registry is a fixed data structure).

**Guarantee scope**: Primitive outputs are deterministic. Downstream systems (combat, behavior, UI) may add randomness, but the interpreter's output is not random.

**Save-state implications**: To restore a creature's exact primitive-effect set, snapshot: (genotype_id, environment, channel_registry, primitive_registry, hook_registry). No additional randomness seed is needed.

---

**End of Document (1632 lines)**
