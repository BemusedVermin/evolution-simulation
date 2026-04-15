# Channel Manifest Schema & Documentation

This directory contains the formal schema and examples for **Channel Manifests** in the Beast Evolution Game—the core mechanism by which genetic traits evolve, mutate, and compose into emergent abilities.

## Overview

A **Channel** is a regulated genetic trait (e.g., auditory sensitivity, muscle fiber density, social bonding strength). Channels are:

- **Organized by family** (9 families: sensory, motor, metabolic, structural, regulatory, social, cognitive, reproductive, developmental)
- **Quantitative**, with continuous numeric values and physical units
- **Mutable** via Gaussian kernels with configurable bounds policies
- **Composable**, combining with other channels to influence phenotype
- **Conditional**, expressing only under specific biome, developmental, or density regimes
- **Scalable**, with body-mass ranges defining macro/meso/micro applicability
- **Extensible**, registered by mods or created via channel genesis

A **Channel Manifest** is the complete formal definition of a single channel: its family, range, mutation kernel, composition rules, and expression conditions.

---

## Field Reference

### `id` (string, required)
Unique snake_case identifier. Alphanumeric + underscore; must start with letter or underscore. Examples: `auditory_sensitivity`, `leg_muscle_density`, `host_coupling`.

### `family` (enum, required)
Biological family—determines typical mutation breadth, composition patterns, and expression logic:

| Family | Typical σ | Typical Composition | Notes |
|--------|-----------|-------------------|-------|
| **sensory** | 0.05–0.15 | Multiplicative (threshold gates) | Perception thresholds, acuity ranges. Often gates other channels. |
| **motor** | 0.08–0.20 | Multiplicative, additive | Strength, speed, precision. Interact with metabolic cost channels. |
| **metabolic** | 0.10–0.25 | Additive, multiplicative | Energy, digestion rate, body mass. High variance; foundational. |
| **structural** | 0.06–0.18 | Additive (cumulative) | Bone density, hide thickness. Compounding effects. |
| **regulatory** | 0.04–0.12 | Gating, threshold | Hormone levels, internal clocks. Direct ability gates. |
| **social** | 0.12–0.30 | Threshold, gating | Group size preference, bonding strength. Contextual expression. |
| **cognitive** | 0.07–0.22 | Multiplicative (compositing) | Learning speed, memory, problem-solving. Often partners with sensory. |
| **reproductive** | 0.05–0.15 | Gating (binary expression) | Fertility, mate choice, parental investment. Sparse composition. |
| **developmental** | 0.09–0.20 | Threshold (stage-gated) | Growth rate, body plan variation. Sensitive to expression conditions. |

**Genesis defaults**: Most families start with `genesis_weight: 1.0`. High-impact channels (metabolic, regulatory) often reduced to 0.5–0.7.

### `description` (string, required)
Human-readable explanation (10–500 chars). Example: *"Auditory perception threshold in dB; lower values = more sensitive"*.

### `range` (object, required)
Defines the numeric domain:

- **`min`**: Minimum valid value.
- **`max`**: Maximum valid value.
- **`units`**: Physical units (e.g., `"dB"`, `"kg"`, `"Hz"`, `"dimensionless"`).

Example:
```json
"range": {
  "min": 20,
  "max": 100,
  "units": "dB"
}
```

### `mutation_kernel` (object, required)

- **`sigma`** (number > 0): Standard deviation of Gaussian mutations, scaled by the channel's range width at runtime. Example: σ=0.1 on range [0, 1] means typical step ≈ 0.1. Higher σ → bolder mutations.

- **`bounds_policy`** (enum: `"clamp"`, `"reflect"`, `"wrap"`):
  - `clamp`: Truncate to boundary (e.g., 1.5 → 1.0 if max is 1.0).
  - `reflect`: Bounce off boundary (e.g., 1.5 → 0.5 if max is 1.0).
  - `wrap`: Periodic (e.g., 1.5 → 0.5 if range is [0, 1.0]).

- **`genesis_weight`** (≥ 0): Relative probability this channel is chosen for duplication during channel genesis. Sum across all channels in a family sets selection odds.

- **`correlation_with`** (array of objects, optional):
  ```json
  [
    { "channel": "leg_muscle_density", "coefficient": 0.8 },
    { "channel": "metabolic_rate", "coefficient": -0.6 }
  ]
  ```
  Declares shared mutation directions. Coefficient ∈ [-1, 1]:
  - Positive: codirection (both increase together).
  - Negative: antagonistic (one up, one down).
  - Zero: no correlation.

  Example: `auditory_sensitivity` and `vocal_modulation` might correlate with coefficient 0.7, representing the plausibility that more sensitive hearing enables better vocalization control.

### `composition_hooks` (array, required)

Each hook defines how this channel interacts with others:

```json
{
  "with": "spatial_cognition",
  "kind": "threshold",
  "coefficient": 1.0,
  "threshold": 0.5
}
```

- **`with`** (string): Channel ID or `"self"` (auto-interaction, e.g., squared term).

- **`kind`** (enum):
  - **`additive`**: Sum contributions. `result += coefficient × other_channel`.
  - **`multiplicative`**: Product. `result *= (1 + coefficient × other_channel)`.
  - **`threshold`**: Activation gate. If `other_channel >= threshold`, apply this channel.
  - **`gating`**: Binary switch. If `other_channel >= threshold`, gate is ON; else OFF.
  - **`antagonistic`**: Subtraction. `result -= coefficient × other_channel`.

- **`coefficient`** (number): Scaling factor.

- **`threshold`** (number, required if `kind ∈ {threshold, gating}`): Activation value.

**Example—Emergent Echolocation:**
- `auditory_sensitivity`: threshold hook with `spatial_cognition` (coefficient 1.0, threshold 0.6).
- `vocal_modulation`: threshold hook with `spatial_cognition` (coefficient 1.0, threshold 0.6).
- Both activate only if `spatial_cognition ≥ 0.6`, allowing the organism to integrate sound emission with directional processing.

**Composition patterns by family:**
- **Sensory**: Often gating or threshold (gates expression of motor/cognitive channels).
- **Motor**: Multiplicative or additive (scales with strength, cost).
- **Metabolic**: Additive (stacks to compute total energy demand).
- **Regulatory**: Gating (binary control of physiological states).
- **Social**: Threshold (density-dependent activation).
- **Cognitive**: Multiplicative with sensory (composites perception into decision-making).

### `expression_conditions` (array, required)

All conditions must hold for the channel to express. Empty array = always express.

Discriminated union by `kind`:

#### `biome_flag`
```json
{ "kind": "biome_flag", "flag": "aquatic" }
```
Channel expresses only in named biome/environment.

#### `scale_band`
```json
{ "kind": "scale_band", "min_kg": 0.1, "max_kg": 100 }
```
Channel expresses only if organism body mass is in range.

#### `season`
```json
{ "kind": "season", "season": "spring" }
```
Channel expresses only during named season.

#### `developmental_stage`
```json
{ "kind": "developmental_stage", "stage": "breeding_adult" }
```
Channel expresses only in named life stage (e.g., juvenile, adult, elder).

#### `social_density`
```json
{ "kind": "social_density", "min_per_km2": 1, "max_per_km2": 100 }
```
Channel expresses only if population density is within range.

### `scale_band` (object, required)
Applicable body mass range (kg):
```json
"scale_band": { "min_kg": 0.001, "max_kg": 100 }
```

Macro scale: 100+ kg. Meso scale: 1–100 kg. Micro scale: <1 kg (pathogens, symbiotes).

### `body_site_applicable` (boolean, required)
Whether this channel can vary across body locations (e.g., hide thickness on back vs belly). True for most structural and sensory channels; false for systemic regulatory channels.

### `provenance` (string, required)
Origin tracking via regex `^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$`:

- **`core`**: Canonical channel from Beast Evolution core.
- **`mod:my_mod_id`**: Registered by mod `my_mod_id`.
- **`genesis:parent_id:3`**: Duplicated from `parent_id` at generation 3 (see Channel Genesis below).

---

## Channel Genesis

**Channel Genesis** is the evolution of the genetic architecture itself—not just mutation, but creation of new channels via duplication and divergence.

### Genesis Process

1. **Duplication**: At a Genesis event (e.g., every N generations), one channel is selected for copy based on `genesis_weight`. Higher weight → higher selection probability.

2. **Reclassification** (optional): The duplicated channel may shift into a different family (e.g., an auditory sensory channel becomes a cognitive processing channel). This resets typical mutation σ and default composition hooks.

3. **Divergence**: The child channel mutates under its new or existing family's rules, drifting from the parent.

4. **Provenance chain**: Each child records `provenance: "genesis:parent_id:generation"`, enabling family tree reconstruction.

### Example: Echolocation Emergence

Starting state:
- `auditory_sensitivity` (sensory, σ=0.1): thresholds on `spatial_cognition`.
- `vocal_modulation` (motor, σ=0.15): thresholds on `spatial_cognition`.
- `spatial_cognition` (cognitive, σ=0.12): gates both above.

Generation 50: Genesis event duplicates `auditory_sensitivity` and reclassifies as cognitive.
- New channel: `auditory_integration` (cognitive, provenance: `genesis:auditory_sensitivity:50`).
- Inherits threshold rules but mutates σ, composition weights.
- Result: Organism develops a specialized cognitive module for integrating auditory + directional data.

Generation 75: `vocal_modulation` duplicates → reclassified regulatory.
- New channel: `vocal_timing` (regulatory, provenance: `genesis:vocal_modulation:75`).
- Gates vocal output timing to match feedback latency.
- Result: Echolocation stabilizes and becomes cost-effective.

This demonstrates **emergence**: no explicit "echolocation" gene exists; instead, the interplay of three family-diverse channels, shaped by duplication and reclassification, produces the ability.

---

## Mod Registration & Extensibility

### How Mods Register Channels

1. **Define manifest JSON** in mod package (e.g., `mod-my-creatures/channels/my_new_channel.json`).

2. **Ensure family is set**. Family membership is mandatory—determines mutation defaults, composition patterns, expression logic. Cannot be inferred.

3. **Validate against schema** using `jsonschema` Python package:
   ```bash
   pip install jsonschema
   python3 -c "import json, jsonschema; s=json.load(open('channel_manifest.schema.json')); jsonschema.validate(json.load(open('my_channel.json')), s)"
   ```

4. **Set `provenance: "mod:my_mod_id"`** (where `my_mod_id` matches mod package name).

5. **Register with mod loader**:
   ```python
   # In mod init
   from beast_evolution.channels import register_channel
   register_channel(json.load(open("my_new_channel.json")))
   ```

### Why Family is Mandatory

Family is **not inferred** from numeric properties; it's a semantic classification that governs:
- Typical mutation breadth (σ defaults).
- Composition patterns (gates vs additive stacks).
- Expression conditions (e.g., regulatory channels rarely scale with body mass).
- Genesis selection weights (rare channels get low weight).

Mods must explicitly state family to integrate correctly with the mutator and composer.

---

## The Primitive Registry

**Primitives** are atomic phenotype outputs—the vocabulary of effects that the phenotype interpreter emits during organism evaluation. A primitive is never itself a named ability (e.g., "echolocation", "venom"). Rather, named abilities are Chronicler-assigned **labels** over recurring clusters of primitives. The interpreter produces structured `PrimitiveEffect` instances containing `primitive_id`, parameters, body site, conditions, and provenance; the Chronicler observes these patterns and assigns semantic ability names post-hoc.

### The Eight Primitive Categories

Primitives are organized into eight functional categories:

| Category | Role | Example |
|----------|------|---------|
| **signal_emission** | Broadcasting info to environment | Acoustic pulses, chemical markers, electrical discharges |
| **signal_reception** | Passive sensing from environment | Hearing, vision, electroreception, olfaction |
| **force_application** | Mechanical action on environment | Bite force, locomotor thrust, limb impact |
| **state_induction** | Physiological state change in self or target | Paralysis toxins, thermoregulation, metabolic boost |
| **spatial_integration** | Fusing multi-sensory signals into maps | Echolocation coordinate fusion, visual-olfactory fusion |
| **mass_transfer** | Moving substances between spaces | Venom injection, nutrient absorption, pheromone secretion |
| **energy_modulation** | Controlling metabolic rate and energy budget | Burst metabolism, torpor entry, energy distribution |
| **bond_formation** | Establishing behavioral/physiological attachments | Pair bonding, host-parasite coupling, kin recognition |

### Relationship to Channel Manifests' `emits` Field

Each **Channel Manifest** may declare an `emits` array of primitives that the phenotype interpreter can trigger when that channel is active. For example:

```json
{
  "id": "vocal_modulation",
  "family": "motor",
  "emits": [
    { "primitive_id": "emit_acoustic_pulse", "parameter_source": { "frequency_hz": "vocal_frequency_hz", "amplitude_db": "vocal_amplitude_db" } }
  ]
}
```

This declares that when `vocal_modulation` is expressed, the interpreter may emit the `emit_acoustic_pulse` primitive with parameters sourced from the channel's own properties. Similarly, a `spatial_cognition` channel might emit `spatial_integrate` with multi-modal fusion rules.

**Key design pattern**: Channels define *potential* (genetic traits that can express), while primitives define *action* (what the phenotype actually outputs). The composition_hooks on channels determine *when* primitives fire and with what parameters. The Chronicler then clusters recurring primitive signatures—e.g., regular co-firing of `emit_acoustic_pulse` (high frequency), `receive_acoustic_signal` (same band), and `spatial_integrate` (acoustic modality)—and assigns the label "echolocation".

### Starter Primitive Vocabulary

The `primitive_vocabulary/` directory contains 16 starter primitives covering all eight categories (2 each):

**signal_emission**: `emit_acoustic_pulse`, `emit_chemical_marker`  
**signal_reception**: `receive_acoustic_signal`, `receive_photic_signal`  
**force_application**: `apply_bite_force`, `apply_locomotive_thrust`  
**state_induction**: `induce_paralysis`, `thermoregulate_self`  
**spatial_integration**: `spatial_integrate`, `temporal_integrate`  
**mass_transfer**: `inject_substance`, `absorb_substance`  
**energy_modulation**: `elevate_metabolic_rate`, `enter_torpor`  
**bond_formation**: `form_pair_bond`, `form_host_attachment`

Each primitive manifest includes:
- **parameter_schema**: Formal specification of inputs (e.g., frequency_hz for acoustic_pulse).
- **composition_compatibility**: Which channel families can emit this primitive.
- **cost_function**: Base metabolic cost plus parameter-dependent scaling (often power laws, e.g., bite force costs scale as force^1.5).
- **observable_signature**: Modality (acoustic, chemical, mechanical, etc.), detection range, and pattern_key (the signature string the Chronicler uses to infer ability labels).
- **provenance**: Origin tracking (core, mod, or genesis).

---

## System References

- **System 01 (Genotype)**: Stores channel values for each individual. Channels are the atoms of genotype.
- **System 02 (Phenotype & Environment)**: Evaluates `expression_conditions` to determine active channels. Computes phenotype by folding `composition_hooks` into a trait expression engine.
- **System 11 (Mutation & Drift)**: Uses `mutation_kernel` (σ, bounds_policy) to drive channel value change. References `correlation_with` for correlated evolution.
- **System 16 (Modularity & Genesis)**: Implements channel duplication, reclassification, and provenance tracking. Selects channels by `genesis_weight`.

---

## Schema Validation

The schema enforces:
- `family` must be one of nine enums.
- `kind` (composition_hooks, expression_conditions) must be valid enum values.
- `threshold` is conditionally required: mandatory if `kind ∈ {threshold, gating}`.
- `provenance` matches regex `^(core|mod:[a-z_][a-z0-9_]*|genesis:[a-z_][a-z0-9_]*:[0-9]+)$`.
- All numeric ranges sensible (min ≤ max, σ > 0, etc.).
- Every field has a description.

---

## Example Manifests

See `examples/` for five worked examples:

1. **`kinetic_force.json`** (motor, macro scale): Leg strength. Multiplicative interaction with metabolic cost.
2. **`auditory_sensitivity.json`** (sensory): Hearing threshold; gates with `spatial_cognition`.
3. **`vocal_modulation.json`** (motor): Voice control; gates with `spatial_cognition`. Together with auditory_sensitivity, enables echolocation.
4. **`host_coupling.json`** (social, micro scale): Pathogen/symbiont bonding to host. Scale-band expression condition.
5. **`structural_rigidity.json`** (structural, macro scale): Bone density. Additive stacking.

All examples are valid against the schema.
