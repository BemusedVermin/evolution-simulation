# 40 — Pillar P4: Ecology Emergence

**Replaces in v1:**
- the fixed `ENERGY_TRANSFER_EFFICIENCY = 0.10` constant (`systems/12`),
- integer trophic levels (`systems/12`),
- the `biome_productivity[biome]` lookup (`systems/12`, `systems/15`),
- the `compute_channel_modifiers(biome, ...)` if/else cascade (`systems/15`),
- diet-only niche overlap (`systems/12`),
- hardcoded predation success rate (`PREDATION_SEARCH_EFFICIENCY = 0.8`),
- fixed `STARVATION_THRESHOLD_DAYS = 14` and similar species-agnostic constants.

**Depends on:** P1 (insolation, temperature, precipitation, soil moisture), P2 (soil composition, nutrient pools), the existing creature-evolution carriers (genome / phenotype-interpreter pipeline) which v2 leaves untouched.

---

## 1. Overview

P4 makes population dynamics *emerge* from continuous traits and resource channels. The headline replacements:

1. **Lindeman's 10% rule → allometric trophic networks (ATN).** Consumption rate is a mechanistic function of predator-prey body-mass ratio, encounter geometry, and metabolic demand (Brose et al. 2006; Petchey et al. 2008). The "10%" emerges as an *aggregate* over the realised diet, not as an input.
2. **Integer trophic levels → continuous trophic position.** Computed post-hoc from realised diet history (Post 2002 stable-isotope analogue). Omnivores get fractional trophic positions; trophic cascades emerge naturally.
3. **`biome_productivity[]` lookup → vegetation growth from P1+P2 channels.** Net primary productivity is a function of `insolation`, `temperature_anom_K`, `soil_moisture`, `nitrogen_kgm2`, `phosphorus_kgm2`, `cation_exchange_capacity`. No biome enum involved.
4. **Williams-Martinez niche links → emergent food web.** Per cell, the food-web topology is recomputed from creature body-mass distribution; trophic links exist when prey body mass falls within predator's allometric feeding range.
5. **Diet-only niche overlap → multivariate trait niche.** Niche is a cluster in the creature trait space (continuous, from existing genome channels), not a diet-overlap scalar.
6. **Decomposers as a channel pool** rather than an instantaneous decomposer_efficiency multiplier — dead biomass enters the P2 SOM pools (Century model), nutrient cycling closes as a real loop.

---

## 2. Research basis

Full synthesis in `research/R20_ecology_social.md` (Part A). Key references:

- **Williams & Martinez (2000)** *Nature* 404 — niche model for emergent food-web topology.
- **Brose, U. et al. (2006)** *Ecology Letters* 9 — allometric trophic networks (ATN) framework.
- **Petchey, O. L. et al. (2008)** *PNAS* 105 — allometric diet breadth model.
- **Post, D. M. (2002)** *Ecology* 83 — continuous trophic position from realised diet.
- **Kooijman, S. (2010)** *Dynamic Energy Budget Theory* — optional individual-level metabolic model (deferred to a later phase).
- **Violle, C. et al. (2007)** *Oikos* 116 — trait-based community ecology.
- **Hairston, N. et al. (2005)** *Ecology Letters* 8 — eco-evolutionary feedback.
- **Parton, W. (1987)** Century — decomposer pool dynamics (shared with P2).

---

## 3. Channels & carriers

P4 mostly *uses* existing carriers (creatures, cells) plus a small number of new derived channels. No new top-level carrier is introduced; the "ecology state" of a cell is a slice of the cell carrier.

### Cell carrier — P4 context-gate channels

| Channel id | Range | Units | Role |
|------------|-------|-------|------|
| `vegetation_lai` | `[0, 8]` | m²/m² (leaf-area index) | Photosynthetic surface; emergent from light, water, nutrients |
| `vegetation_cover` | `[0, 1]` | dimensionless | Fractional ground cover; modifies P1 albedo |
| `npp_kgC_m2_yr` | `[0, 3]` | kgC/m²/yr | Net primary productivity (instantaneous) |
| `gpp_kgC_m2_yr` | `[0, 5]` | kgC/m²/yr | Gross primary productivity |
| `litter_kgm2` | `[0, 10]` | kgC/m² | Litter pool, feeds Century in P2 |
| `herbivore_biomass_kgm2` | `[0, 0.5]` | kg/m² | Aggregate, per-cell |
| `predator_biomass_kgm2` | `[0, 0.05]` | kg/m² | Aggregate |
| `decomposer_biomass_kgm2` | `[0, 0.1]` | kg/m² | Microbial pool (continuous) |

Per-species populations live on the existing per-creature carriers (no new state); these aggregates are derived but cached for cheap access.

### Creature carrier — channels read by P4 (already in v1)

`body_mass_kg`, `metabolic_rate_channel`, `diet_breadth`, `forage_speed`, `digestive_efficiency` — all continuous and registry-backed in v1. P4 *reads* them; the genome / phenotype-interpreter pipeline that produces them is untouched.

### Derived per-cell food-web data structure

Stored once per N ticks (N = 12 default), recomputed from creature distribution:

```
struct CellFoodWeb {
    species_in_cell: SortedVec<SpeciesId>,
    links: SparseMatrix<SpeciesId, SpeciesId, InteractionStrength>,  // attack & assimilation rates
    trophic_position: Map<SpeciesId, Q32_32>,                        // continuous TP, Post 2002
}
```

The matrix is sparse (typically O(S log S) entries from allometric link rules), bit-deterministic (sorted iteration), and recomputed deterministically.

---

## 4. Update rules

### 4.1 Vegetation growth (NPP)

```
fn vegetation_step(cell: &mut Cell, env: &EnvChannels) {
    // Light availability
    let light = env.insolation_W_m2 * (1 - exp(-K_BEER * cell.vegetation_lai));

    // Water stress (Monteith)
    let water_stress = monteith_water_stress(env.soil_moisture, env.specific_humidity);

    // Temperature limitation (parabolic; emergent zonation, no biome lookup)
    let T = env.temperature_anom_K + GLOBAL_T0;
    let temp_factor = max(0, 1 - ((T - T_OPT) / T_HALF)^2);

    // Nutrient limitation (Liebig: minimum of N, P, K availability)
    let nut_factor = liebig_min(env.nitrogen_kgm2, env.phosphorus_kgm2, env.potassium_kgm2);

    cell.gpp_kgC_m2_yr = K_GPP_MAX * light * water_stress * temp_factor * nut_factor;
    let respiration = K_AUTO_RESP * cell.vegetation_lai * temp_factor;
    cell.npp_kgC_m2_yr = max(0, cell.gpp_kgC_m2_yr - respiration);

    // LAI integrates NPP with senescence
    cell.vegetation_lai += DT * (cell.npp_kgC_m2_yr * F_LEAF_ALLOC - K_SENESC * cell.vegetation_lai);
    cell.vegetation_cover = 1 - exp(-K_BEER * cell.vegetation_lai);

    // Litter
    cell.litter_kgm2 += DT * K_SENESC * cell.vegetation_lai;
    // (Century in P2 then consumes this in soil_step)
}
```

This single update reproduces the latitudinal, water-limited, and nutrient-limited zonation that v1's `biome_productivity[]` table fakes — without enums.

### 4.2 Food-web emergence (Williams-Martinez + ATN)

Every N ticks, per cell:

```
fn rebuild_foodweb(cell: &Cell, species_set: &[SpeciesId]) -> CellFoodWeb {
    // Species sorted by id for determinism
    let species: Vec<_> = species_set.iter().sorted_by_id().collect();

    // Williams-Martinez niche position derived from log body mass (1-D niche axis)
    let niches: Vec<Q32_32> = species.iter()
        .map(|s| log10_q32(s.mean_body_mass_kg))
        .collect();

    // Allometric feeding range: predator i feeds on prey j if
    //    log(M_pred / M_prey) ∈ [LMR_LO, LMR_HI]
    // (typical values 0.5 to 4 — predators eat smaller, but not too small)
    let mut links = SparseMatrix::new();
    for (i, n_i) in niches.iter().enumerate() {
        for (j, n_j) in niches.iter().enumerate() {
            if i == j { continue; }
            let lmr = n_i - n_j;       // log mass ratio
            if lmr >= LMR_LO && lmr <= LMR_HI {
                // ATN attack rate ∝ M_pred^0.75 (allometric scaling, Brose 2006)
                let attack = K_ATTACK_BASE * pow_q32(species[i].mean_body_mass_kg, ALPHA_ATTACK);
                let assim  = K_ASSIM_BASE  * sigmoid(lmr - LMR_OPT, ASSIM_WIDTH);
                links.insert(species[i].id, species[j].id, InteractionStrength{attack, assim});
            }
        }
    }

    // Continuous trophic position (Post 2002): TP_i = 1 + Σ_j ω_ij * TP_j
    // where ω is the realised diet weight. Iterate to convergence (sorted, K = 16 sweeps).
    let tp = compute_continuous_tp(&links, K_TP_SWEEPS);

    CellFoodWeb { species_in_cell: species, links, trophic_position: tp }
}
```

`pow_q32` and `sigmoid` use Q32.32 lookup tables.

### 4.3 Predation step

For each predator individual in a cell, every tick:

```
let prey_options = foodweb.prey_of(predator.species_id);  // sorted by id
// Encounter rate from ATN: r_enc = a * N_prey^h, where a is attack rate and h is functional response
// (Holling type II by default; type III if predator has 'specialist' channel high)
let r = compute_holling(predator, prey_options, foodweb);
let kills = sample_poisson(r * DT, &mut rng_subsystem.predation);  // deterministic Xoshiro
for _ in 0..kills {
    let prey = sample_weighted(prey_options, weights, &mut rng);
    energy_gained += prey.body_mass_kg * foodweb.links[(pred.id, prey.id)].assim;
}
```

The single Xoshiro stream `rng_subsystem.predation` is per-cell-sorted; cross-cell determinism is preserved by per-cell stream forking (sorted cell id → forked stream).

### 4.4 Decomposers as a real pool

Dead biomass (predation losses, senescence, age-out) becomes `litter_kgm2` and is consumed by the P2 Century pools. The v1 `decomposer_efficiency = 0.70` constant is replaced by the actual Century turnover rates, which are temperature- and moisture-dependent (so cold cells genuinely retain dead biomass longer, exactly as in nature).

### 4.5 Carrying capacity

The v1 doc had a six-line manual computation for K. v2 has *no explicit K calculation* — populations grow until consumption equals supply, and supply is endogenous (vegetation NPP for herbivores, prey biomass for carnivores). K *emerges*; it is computed at UI render time as a derived statistic if needed for display.

---

## 5. Cross-pillar hooks

```mermaid
flowchart LR
    P1[P1 Environment]
    P2[P2 Materials/Soil]
    P3[P3 Biome labels]
    P4[P4 Ecology]
    P5[P5 Social]
    P6[P6 Culture/etc]

    P1 -->|insolation, T, precip, soil_moist| P4
    P2 -->|N, P, K, CEC| P4
    P4 -->|litter_kgm2| P2
    P4 -->|vegetation_cover (modifies albedo, ET)| P1
    P4 -->|vegetation_lai, npp| P3
    P4 -->|prey biomass, predator pressure| P5
    P5 -->|hunting effort, deforestation| P4
    P6 -->|disease (P6 pathogen channels infect P4 individuals)| P4
```

---

## 6. Tradeoff matrix

| Decision | Options | Sim Fidelity | Implementability | Player Legibility | Emergent Power | Choice + Why |
|----------|---------|--------------|------------------|-------------------|-----------------|-------------|
| Trophic transfer | Fixed 10% / **ATN allometric** / Full DEB | ATN strong | ATN moderate | ATN legible (predators big, prey small) | ATN strong (size-structured webs) | **ATN**. Mechanistic, Q32.32-friendly. |
| Trophic position | Integer levels / **Continuous (Post 2002)** | Continuous strictly higher | Same complexity | Continuous more legible (omnivores) | Continuous strong (cascade dynamics) | **Continuous**. |
| Food-web links | Hand-coded diet / **Williams-Martinez niche** / Full neutral assembly | W-M strong | Easy | W-M legible (size axis) | W-M strong (links emerge) | **Williams-Martinez** with allometric weighting. |
| Vegetation | Lookup biome_productivity / **Light × water × T × nutrient (Liebig minimum)** / DGVM (LPJ-GUESS) | Liebig strong | Moderate | Legible (each axis clear) | Strong (latitudinal zonation emerges) | **Liebig minimum**. Cheap, captures all main controls. |
| Decomposer model | Constant efficiency / **Century 3-pool** / Microbial-explicit | Century strong | Moderate (shared with P2) | Legible (litter visible) | Strong (climate-dependent decomposition) | **Century**. Already in P2; reuse. |
| Carrying capacity | Computed explicitly / **Emergent from supply-consumption balance** | Emergent stricter | Easier (no formula) | Both same | Emergent stronger (no spurious K spirals) | **Emergent**. |
| Predation success | Hardcoded 0.8 / **Holling type II/III from ATN** | Holling strong | Moderate | Legible | Strong (functional-response oscillations) | **Holling II default; III if specialist channel high**. |

---

## 7. Emergent properties

1. **Latitudinal productivity gradient.** No biome enum required: NPP drops with cold (temp_factor) and dryness (water_stress) and nutrient limitation; tropics become productive without scripting.
2. **Trophic cascades.** Top-predator removal → herbivore boom → vegetation collapse, recorded as continuous-TP shift, not as enum-class transitions.
3. **Boom-bust cycles.** Holling-II + delayed reproduction yields Lotka-Volterra-like oscillations endogenously.
4. **Niche partitioning under selection.** Two species with overlapping diet exert competition pressure that selects for trait divergence (eco-evolutionary feedback) — done by the existing creature evolution pipeline reading P4's competition signal.
5. **Soil exhaustion → ecosystem regime shift.** Heavy herbivory accelerates litter cycling but reduces vegetation cover; if soils are nutrient-limited, NPP collapses and the cell may transition to a different cluster (Chronicler relabels in P3).
6. **Keystone species emergence.** No designer marks a species as keystone; if removing it would collapse a large fraction of the food-web matrix, it *is* one — the metric is computed lazily for UI.
7. **Body-size-structured food webs.** Tiny pathogens, mid-mass herbivores, large predators all cohabitate without scale-band branching (`INVARIANT 5`).

---

## 8. Open calibration knobs

- `K_BEER` (light-attenuation through canopy).
- `T_OPT`, `T_HALF` (vegetation temperature optimum + half-width).
- `K_GPP_MAX` (potential GPP).
- `K_AUTO_RESP`, `K_SENESC`, `F_LEAF_ALLOC`.
- `LMR_LO`, `LMR_HI`, `LMR_OPT` (Williams-Martinez log-mass-ratio bounds).
- `K_ATTACK_BASE`, `K_ASSIM_BASE`, `ALPHA_ATTACK` (ATN scaling).
- `ASSIM_WIDTH` (assimilation efficiency width around optimal LMR).
- `K_TP_SWEEPS` (continuous-TP iterative depth).
- `N_TICKS_FOODWEB_REBUILD` (food-web matrix recompute cadence).

---

## 9. Determinism checklist

- ✅ All channel math Q32.32.
- ✅ Sorted iteration in food-web reconstruction.
- ✅ Per-cell forked Xoshiro stream for predation Poisson sampling.
- ✅ Continuous-TP Jacobi iteration uses fixed sweep count K_TP_SWEEPS.
- ✅ Litter → P2 Century pool coupling in same tick stage; no cross-stage write hazards.
- ✅ No HashMap iteration; all SparseMatrix iteration is sorted.

---

## 10. Sources

- Williams, R., Martinez, N. (2000). *Nature* 404.
- Brose, U. et al. (2006). *Ecology Letters* 9.
- Petchey, O. L. et al. (2008). *PNAS* 105.
- Post, D. M. (2002). *Ecology* 83.
- Kooijman, S. (2010). *Dynamic Energy Budget Theory* (Cambridge).
- Violle, C. et al. (2007). *Oikos* 116.
- Hairston, N. et al. (2005). *Ecology Letters* 8.
- Holling, C. S. (1959). "Some characteristics of simple types of predation and parasitism." *Canadian Entomologist* 91.
- Lieth, H. (1975). *Primary Productivity of the Biosphere*.
