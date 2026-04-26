# 20 — Pillar P2: Materials Emergence

**Replaces in v1:**
- the 5-value `rock_type` enum (`BASALT | GRANITE | SEDIMENTARY | LIMESTONE | VOLCANIC_ASH`) and `soil_fertility` scalar in `systems/15_climate_biome_geology.md`,
- the 17-property `MaterialSignature` shared by `systems/04_economic_layer.md`, `systems/05_crafting_system.md`, and `systems/06_combat_system.md`,
- the 5 `MaterialLineage` source-type enum (`CreatureHarvest | EnvironmentalDeposit | Salvage | Processed | Trade`).

**Depends on:** P1 environmental channels (temperature, precipitation, soil moisture, runoff). The weathering kinetics and soil-moisture coupling read these directly.

**Sibling pillars:** P3 reads composition for biome labelling; P4 reads soil composition for primary-productivity emergence; P5/P6 read material-instance composition for technology, economy, crafting.

---

## 1. Overview

Materials in v2 are **continuous composition vectors plus a small set of derived physical channels**, never enums. A "rock" is a 6-fraction composition vector (quartz, feldspar, mafic, clay, carbonate, organic). A "soil" is a soil-texture triad (sand, silt, clay) plus organic-matter pool plus nutrient pools. A "tool material" is the same composition vector plus a derived signature (hardness, density, …) that emerges from composition rather than being authored.

Weathering — the conversion of rock to soil — is a deterministic kinetic update driven by P1 atmosphere and hydrology. New minerals form, old ones dissolve, organic matter accumulates and decomposes, and the resulting soil composition determines the cell's nutrient pools that feed P4's primary productivity.

The labels "basalt", "granite", "sandstone", etc. survive only as Chronicler outputs in P3 (or as decorative bestiary text), never as control-flow branches.

---

## 2. Research basis

Full synthesis in `research/R10_environment_materials.md`. Key references:

- **Mineralogical composition vectors (QAPF approach).** Streckeisen (1976). Igneous rocks are classified by quartz (Q), alkali feldspar (A), plagioclase (P), feldspathoid (F) percentages — *as a continuous space*. We adopt a simplified 6-component basis suitable for game-scale chemistry.
- **Soil texture triangle.** USDA soil taxonomy. Sand/silt/clay fractions sum to 1; soil "type" is a label over this 2-simplex. Soil composition determines water-holding capacity, infiltration, cation exchange.
- **Century soil-organic-matter model.** Parton et al. (1987, 1988). Three pools (active / slow / passive) with first-order decay rates modulated by temperature and moisture. Bit-deterministic. We adopt directly.
- **Simplified weathering kinetics.** Goddéris & Brantley (2013); West (2012). Arrhenius-style temperature-dependence on mineral dissolution rates; runoff carries dissolved species. We adopt a coarse 6-mineral kinetic model (one rate per mineral pair).
- **Reactive transport (CrunchFlow / PHREEQC families).** Steefel & Lasaga (1994). Production-grade reactive transport. We do *not* adopt; we capture the same topology with a coarse equilibrium-thermodynamics shortcut.
- **Material derived properties from composition.** Mishra & Mathew (2012). Empirical relations of hardness, density, etc. to mineralogy. We use a small set of linear combinations.

---

## 3. Channels & carriers

### 3.1 New carrier: `material_instance`

Used for any chunk of physical matter the simulation tracks separately — a tool, an ore deposit, a piece of equipment, a corpse, a harvested resource. Replaces the v1 `MaterialSignature` everywhere it appears.

#### Composition vector (6 fractions, sum = 1)

| Channel id | Range | Role |
|------------|-------|------|
| `frac_quartz` | `[0, 1]` | SiO₂ family — hard, inert, abrasive |
| `frac_feldspar` | `[0, 1]` | Al-silicates — moderate hardness, moderate weathering |
| `frac_mafic` | `[0, 1]` | Fe-Mg silicates — dense, dark, weathers fast |
| `frac_clay` | `[0, 1]` | Layered silicates — soft, plastic, water-retaining |
| `frac_carbonate` | `[0, 1]` | CaCO₃ family — soluble, biogenic |
| `frac_organic` | `[0, 1]` | Plant/animal-derived carbon — combustible, fertile |

Sum-to-one constraint enforced by the operator (renormalise on every drift step). This is an example of the v1 "channels can model categorical enums as multiple correlated channels" idiom (`core-model/01_channels.md`).

#### Physical-state channels (derived but stored to avoid recomputation)

| Channel id | Range | Units | Role |
|------------|-------|-------|------|
| `grain_size_mm` | `[1e-3, 100]` | mm | Mean grain size; weathering shrinks |
| `bulk_density_kg_m3` | `[500, 7000]` | kg/m³ | Mass per unit volume; computed from composition + porosity |
| `porosity` | `[0, 0.6]` | dimensionless | Void fraction; affects permeability and weathering rate |
| `permeability_log_md` | `[-6, 4]` | log₁₀(millidarcies) | For groundwater coupling |
| `cohesion_kPa` | `[0, 5e5]` | kPa | Resistance to shear; affects mining, building |
| `temperature_K` | `[150, 2000]` | K | Material temperature (for ore smelting, etc.) |

#### Derived signature (replaces v1 17-property `MaterialSignature`)

Computed by an operator, *not stored* — pure functions of composition + state. The 17 v1 properties are reproduced as derived channels:

```
hardness        = 9*frac_quartz + 6*frac_feldspar + 5*frac_mafic + 1*frac_carbonate + 0.5*frac_clay
density         = bulk_density_kg_m3 / 1000
flexibility     = 1 - hardness/9
toxicity        = f(trace_elements, frac_organic_decomposing)
purity          = max(frac_*) / sum(frac_*)        // dominant-component dominance
conductivity_th = 0.7*frac_quartz + 0.5*frac_feldspar + 1.5*frac_mafic + 0.2*frac_clay + ...
... and so on.
```

The Chronicler can still label a material "obsidian" (high `frac_quartz` + glassy `grain_size_mm` < 0.1) or "limestone" (high `frac_carbonate`) — same mechanics-label separation.

### 3.2 Cell-attached soil & regolith channels

These are channels on the existing `cell` carrier (introduced in P1), gated by a "soil" context gate.

| Channel id | Range | Role |
|------------|-------|------|
| `soil_sand_frac` | `[0, 1]` | Texture component |
| `soil_silt_frac` | `[0, 1]` | Texture component |
| `soil_clay_frac` | `[0, 1]` | Texture component (sum-to-one operator) |
| `soil_organic_carbon_kgm2` | `[0, 50]` | Surface organic-matter pool |
| `som_active_kgm2` | `[0, 5]` | Century active SOM pool |
| `som_slow_kgm2` | `[0, 30]` | Century slow SOM pool |
| `som_passive_kgm2` | `[0, 50]` | Century passive SOM pool |
| `nitrogen_kgm2` | `[0, 5]` | Plant-available N |
| `phosphorus_kgm2` | `[0, 1]` | Plant-available P |
| `potassium_kgm2` | `[0, 5]` | Plant-available K |
| `cation_exchange_capacity_cmol_kg` | `[0, 100]` | Soil chemistry: derived from clay + organic |
| `regolith_depth_m` | `[0, 50]` | Weathered-rock layer depth |
| `bedrock_composition_*` | (six fractions) | Underlying rock composition (rare changes; tectonic only) |

The `soil_fertility` scalar in v1 is replaced by the *combination* of nutrient channels and CEC. P4's primary-productivity computation reads them directly.

---

## 4. Update rules

### 4.1 Weathering kinetics

Runs every K ticks (default K = 4) in Stage 5 (Physiology) — soil treated as a slow physiological field of the cell.

```
fn weathering_step(cell: &mut Cell, env: &EnvChannels) {
    // Arrhenius temperature dependence
    let T = env.temperature_K();
    let k_temp = arrhenius_lookup(T);  // Q32.32 lookup table

    // Moisture dependence — wet rock weathers faster
    let k_moist = sigmoid(env.soil_moisture, KW_M0, KW_W);

    // Per-mineral dissolution rate
    let k_mafic = K_MAFIC_BASE * k_temp * k_moist;
    let k_feldspar = K_FELDSPAR_BASE * k_temp * k_moist;
    let k_carbonate = K_CARBONATE_BASE * k_temp * k_moist
                       * carbonate_acidity(env.specific_humidity);
    // quartz, clay, organic have their own rates

    // Apply to bedrock_composition: mafic and feldspar weather to clay
    let dt = K_TICKS as Q32_32 * TICK_SECONDS;
    let dq = k_quartz   * cell.bedrock_quartz   * dt;
    let df = k_feldspar * cell.bedrock_feldspar * dt;
    let dm = k_mafic    * cell.bedrock_mafic    * dt;
    let dc = k_carbonate* cell.bedrock_carbonate* dt;

    cell.bedrock_quartz    -= dq;
    cell.bedrock_feldspar  -= df;
    cell.bedrock_mafic     -= dm;
    cell.bedrock_carbonate -= dc;

    // Products: feldspar + mafic → clay (most of mass), carbonate → solute (lost to runoff)
    cell.soil_clay_frac    += (df + dm) * (1 - LEACHED_FRAC) / cell.regolith_mass();
    cell.soil_sand_frac    += dq * (1 - LEACHED_FRAC) / cell.regolith_mass();
    let leached_kg         = (df + dm + dc) * LEACHED_FRAC;
    env.runoff_solutes_kgm2 += leached_kg / cell.area_m2;

    cell.regolith_depth_m   += (df + dm + dc + dq) / cell.regolith_density() * dt;
    renormalise_soil_texture(cell);
}
```

All operations are Q32.32 fixed-point; the Arrhenius `exp(-Ea/RT)` uses a pre-computed 1024-entry lookup table indexed by `temperature_K - 250 K` (covers a 250–500 K range with < 0.1 % error).

### 4.2 Century soil-organic-matter model

Three pools, first-order decay, modulated by temperature and moisture (Parton et al. 1987):

```
fn som_step(cell: &mut Cell, env: &EnvChannels) {
    let f = som_climate_factor(env.temperature_anom_K, env.soil_moisture);
    let dt = K_TICKS as Q32_32 * TICK_SECONDS;

    // Inputs: litter from P4 (vegetation senescence + animal mortality)
    let litter_in = cell.litter_input_kgm2_per_step();  // from P4

    // Active pool: fast turnover
    let dec_active  = K_ACTIVE_DEC  * cell.som_active_kgm2  * f * dt;
    let dec_slow    = K_SLOW_DEC    * cell.som_slow_kgm2    * f * dt;
    let dec_passive = K_PASSIVE_DEC * cell.som_passive_kgm2 * f * dt;

    cell.som_active_kgm2  += litter_in   - dec_active;
    cell.som_slow_kgm2    += dec_active  * F_ACTIVE_TO_SLOW   - dec_slow;
    cell.som_passive_kgm2 += dec_slow    * F_SLOW_TO_PASSIVE  - dec_passive;

    // Mineralised nutrients released as decomposition occurs
    cell.nitrogen_kgm2  += dec_active * N_RATIO_ACTIVE
                         + dec_slow   * N_RATIO_SLOW
                         + dec_passive* N_RATIO_PASSIVE;
    // P, K analogous
}
```

The CN/CP/CK ratios are calibration constants (legitimate). The soil-organic-carbon channel `soil_organic_carbon_kgm2` is the sum of the three pools.

### 4.3 Material-instance physics (tools, equipment, ore)

Material instances detached from a cell (e.g., a stone axe) run a much lighter update — temperature equilibrates with surroundings, derived channels recomputed if composition changes (rare; only when crafted/processed). P5/P6 crafting reads the derived channels.

---

## 5. Cross-pillar hooks

```mermaid
flowchart LR
    P1[P1 Environment]
    P2[P2 Materials]
    P3[P3 Biome labels]
    P4[P4 Ecology]
    P5[P5 Social]
    P6[P6 Culture/etc]

    P1 -->|temperature, soil_moisture, runoff| P2
    P2 -->|albedo of bare soil/rock| P1
    P2 -->|soil composition + nutrient pools| P3
    P2 -->|nutrient channels (N/P/K), CEC| P4
    P4 -->|litter input| P2
    P2 -->|material_instance composition| P5
    P2 -->|composition| P6
    P6 -->|recombinant tech (smelting, alloying) modifies composition| P2
    P5 -->|mining, deforestation modify cell composition| P2
```

---

## 6. Tradeoff matrix

| Decision | Options | Sim Fidelity | Implementability | Player Legibility | Emergent Power | Choice + Why |
|----------|---------|--------------|------------------|-------------------|-----------------|-------------|
| Mineralogy | Enum / **6-fraction composition** / Full thermodynamics (CrunchFlow) | Composition strong; full tx highest | Composition moderate | Composition legible (ternary diagrams) | Composition strong | **6-fraction**. Reproduces useful diversity at game cost. |
| Soil model | scalar fertility / **Texture triad + Century 3-pool SOM** / Full pedogenesis | Century strong | Moderate | Texture triangle is intuitive | Strong (decadal-timescale soil change) | **Texture + Century**. Standard in ecosystem science. |
| Weathering | None / Linear / **Arrhenius kinetics with moisture coupling** / Full reactive transport | Arrhenius strong | Moderate (lookup tables) | Visible: rocks weather to soil over time | Strong (regional regolith patterns) | **Arrhenius**. Realistic; cheap with lookup. |
| Material derived properties | 17 authored numbers / **Linear functions of composition** / Full QSPR | Linear mid; QSPR highest | Linear easiest | Linear legible | Mid (composition determines behaviour) | **Linear from composition**. Removes designer-authored numbers. |
| Material instance update freq | Every tick / **K-ticks with K = 4** / Event-driven | Same | K-tick easiest | Same | Same | **K = 4**. Soil & material physics are slow. |

---

## 7. Emergent properties

1. **Latitudinal regolith patterns.** Tropical wet cells weather fast → thick clay-rich regolith (laterite-like) → high water-retention. Polar cells barely weather → thin rocky soils. Mid-latitudes get "fertile loams". Nothing scripts this; it falls out of Arrhenius temperature-dependence + moisture coupling + Century SOM input from P4 vegetation.
2. **Karst-like landscapes.** High-`frac_carbonate` bedrock + high precipitation → fast carbonate dissolution → caves and sinkholes (modeled as elevated `permeability_log_md` and lowered `regolith_depth_m`).
3. **Fertile floodplains.** D∞ runoff carries dissolved nutrients downhill; deposition zones (low slope) accumulate them. P4's primary productivity follows automatically.
4. **Soil exhaustion.** Heavy P4 vegetation extracts N/P/K faster than weathering replaces them in nutrient-poor cells. Long-running settlements deplete soil unless P5/P6 introduces fallow periods or fertilisation. Emergent agronomy.
5. **Material lineage as composition history.** The v1 5-enum `MaterialLineage` (`CreatureHarvest | EnvironmentalDeposit | Salvage | Processed | Trade`) is replaced by *compositional fingerprint* — bone-derived materials have high `frac_organic` + characteristic Ca/P, ore-derived materials have high `frac_mafic`, etc. The Chronicler can label without an enum.

---

## 8. Open calibration knobs

- `K_MAFIC_BASE`, `K_FELDSPAR_BASE`, `K_CARBONATE_BASE`, `K_QUARTZ_BASE` — base mineral dissolution rates.
- `LEACHED_FRAC` — fraction of weathered mass exported as solute.
- Century pool turnover rates `K_ACTIVE_DEC`, `K_SLOW_DEC`, `K_PASSIVE_DEC` and inter-pool fractions `F_ACTIVE_TO_SLOW`, `F_SLOW_TO_PASSIVE`.
- Nutrient ratios `N_RATIO_*`, `P_RATIO_*`, `K_RATIO_*`.
- Bedrock-composition prior at world creation (sets dominant rock types per cell).

---

## 9. Determinism checklist

- ✅ Composition fractions Q32.32; sum-to-one operator runs every step.
- ✅ Arrhenius via 1024-entry Q32.32 lookup table, no live `exp` calls.
- ✅ K-tick weathering and Century SOM update at fixed sub-cycle.
- ✅ Sorted-cell iteration in soil/runoff dependency.
- ✅ Material-instance ids sorted in any global pass.

---

## 10. Sources

- Streckeisen, A. (1976). "To each plutonic rock its proper name." *Earth-Science Reviews* 12.
- Parton, W. et al. (1987). "Analysis of factors controlling soil organic matter levels in Great Plains grasslands." *SSSAJ* 51.
- Parton, W. et al. (1988). "Dynamics of C, N, P and S in grassland soils: a model." *Biogeochemistry* 5.
- Goddéris, Y., Brantley, S. (2013). "Earthcasting the future critical zone." *Elementa* 1.
- West, A. J. (2012). "Thickness of the chemical weathering zone and implications for erosional and climatic drivers of weathering and for carbon-cycle feedbacks." *Geology* 40.
- Steefel, C., Lasaga, A. (1994). "A coupled model for transport of multiple chemical species and kinetic precipitation/dissolution reactions with application to reactive flow in single phase hydrothermal systems." *Am. J. Sci.* 294.
- Mishra, A., Mathew, R. (2012). "Empirical correlations for hardness from mineralogy." *J. Mat. Sci.* 47.
