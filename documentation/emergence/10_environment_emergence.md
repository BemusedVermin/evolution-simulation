# 10 — Pillar P1: Environment Emergence

**Replaces in v1:** large parts of `systems/15_climate_biome_geology.md` (atmospheric circulation, climate computation, hydrology) and the cell-attached climate scalars referenced from `systems/07_exploration_system.md`.

**Depends on:** topology decision (`90_topology_decision.md`) — assumes spherical centroidal Voronoi (SCVT) with ≈ 20 000 cells and a frozen integer neighbour graph stored in the world artefact.

**Sibling pillars:** `20_materials_emergence.md` (soils & weathering read P1), `40_ecology_emergence.md` (carrying capacity reads P1).

---

## 1. Overview

P1 replaces the v1 hardcoded environment scaffolding — `biome_type` enum, the static 3-cell Hadley/Ferrel/Polar pattern, `compute_channel_modifiers(biome, T, P, ...)` if/else, `wind_speed_mps`, fixed precipitation bands, lapse-rate-based temperature lookup — with **continuous environmental channels per cell** updated by a deterministic moist energy-balance atmosphere coupled to a runoff/groundwater hydrology and a surface-energy-budget land model.

Hadley/Ferrel/Polar circulation patterns *emerge* from the meridional heat-transport closure of the EBM. The ITCZ migrates seasonally because of the seasonal insolation forcing on a heat-capacity field that includes ocean-vs-land contrast. Deserts emerge in subtropical sinking regions; rainforests emerge in the ITCZ; rain-shadows emerge from the orographic-uplift parameterisation. Volcanic ash, ocean currents, and Milankovitch drift are inputs that feed the same channel set rather than special cases.

> **Headline:** v1 had ≈ 14 enumerated biome types and a static circulation pattern. v2 has 11 environmental channels per cell and circulation emerges. Biome names are *labels* applied by the Chronicler in pillar P3.

---

## 2. Research basis

The full literature synthesis is in `research/R10_environment_materials.md`. Headline references:

- **EBM (Energy-Balance Model).** Budyko (1969); Sellers (1969). One-dimensional zonal-mean energy budget with diffusive meridional transport. Bit-deterministic, runs in fixed-point at < 1 op/cell/timestep. Reproduces the basic equator-to-pole temperature gradient and the seasonal cycle from solar geometry alone.
- **Moist EBM.** Frierson, Held & Zurita-Gotor (2006); Hill et al. (2017). Adds a Clausius-Clapeyron-saturated moist static energy budget; reproduces ITCZ, monsoon migration, polar amplification — *all of which are hardcoded in v1's circulation table*.
- **MPAS-A on SCVT.** Skamarock et al. (2012). Conservative finite-volume non-hydrostatic atmospheric core on spherical Voronoi cells. We do not adopt MPAS-A wholesale — far too expensive for game scale — but we follow its *discretisation pattern*: cell-centred scalars (T, q, p), edge-centred fluxes, Voronoi-cell-area-weighted divergence.
- **D∞ flow routing.** Tarboton (1997). Generalises D8 to allow flow to be partitioned among up to two neighbours. Works directly on irregular Voronoi neighbour graphs because it operates on edge-incident slope rather than fixed compass directions.
- **Boussinesq groundwater.** Marçais et al. (2017). Single-layer unconfined aquifer; cell-scalar groundwater head, lateral flux ∝ head gradient × transmissivity. Conservative, deterministic.
- **Land-surface scheme (TESSEL-style).** Balsamo et al. (2009). Surface-energy-budget closure with bare soil + vegetation tile fractions, soil moisture as a function of precipitation, evapotranspiration, runoff, drainage. We use a heavily simplified single-layer variant.
- **Sea-ice albedo feedback.** North & Coakley (1979). Ice line emerges from temperature when albedo is a step function of `temperature_anom_K`.

---

## 3. Channels & carriers

All channels follow the v1 `core-model/01_channels.md` schema. They are registered in the standard registry; mods can extend them.

### New carrier: `cell`

The cell carrier holds all per-Voronoi-cell environmental state. Channels are sorted into three context-gate groups (P1 surface, P1 atmosphere column, P1 hydrology) but live on a single carrier for simplicity.

#### Surface and atmospheric channels (P1)

| Channel id | Range | Units | Role |
|------------|-------|-------|------|
| `temperature_anom_K` | `[-50, +50]` | K | Surface temperature anomaly from a global-mean baseline. Primary state of the EBM. |
| `surface_albedo` | `[0, 1]` | dimensionless | Fraction of insolation reflected. Updated from snow/ice/vegetation/soil. |
| `cloud_fraction` | `[0, 1]` | dimensionless | Diagnosed from humidity + lapse rate. Modifies effective albedo and longwave back-radiation. |
| `specific_humidity` | `[0, 0.04]` | kg(water)/kg(air) | Column-integrated water vapour, normalised by Clausius-Clapeyron at `temperature_anom_K`. |
| `precipitation_kg_m2_s` | `[0, 1e-3]` | kg/m²/s | Net precipitation rate. Diagnosed from convergence of moist static energy + orographic uplift. |
| `wind_u`, `wind_v` | `[-50, +50]` | m/s | Eastward / northward wind components, derived from horizontal-pressure-gradient closure. |
| `insolation_W_m2` | `[0, 500]` | W/m² | Top-of-atmosphere insolation. Function of `latitude`, `axial_tilt`, `eccentricity`, `time_of_year`. |
| `aerosol_optical_depth` | `[0, 1]` | dimensionless | Volcanic ash + dust; modifies insolation. |

#### Hydrology channels (P1, hydrology context gate)

| Channel id | Range | Units | Role |
|------------|-------|-------|------|
| `soil_moisture` | `[0, 1]` | dimensionless (fraction of saturation) | Single-layer soil-water content. |
| `groundwater_head_m` | `[-200, 0]` | m below surface | Boussinesq aquifer head. |
| `runoff_kg_m2_s` | `[0, 1e-3]` | kg/m²/s | Outgoing surface runoff routed to neighbours via D∞. |
| `river_discharge_m3_s` | `[0, 1e6]` | m³/s | Cumulative routed discharge through this cell. |

#### Solid-earth channels (P1, geology context gate)

| Channel id | Range | Units | Role |
|------------|-------|-------|------|
| `elevation_m` | `[-11000, +9000]` | m | Topographic elevation. World-gen + slow tectonic drift. |
| `topographic_slope` | `[0, 1.5]` | rad | Gradient magnitude over neighbour graph; used by orographic precipitation and runoff. |
| `lithosphere_age_Myr` | `[0, 4500]` | Myr | Used by P2 for weathering kinetics. |
| `volcanism_intensity` | `[0, 1]` | dimensionless | Set by tectonic events; decays over time; feeds `aerosol_optical_depth` and P2. |

#### Constants (registered as `provenance: core` and immutable per cell)

| Channel id | Source | Role |
|------------|--------|------|
| `latitude_rad` | World-gen | Deterministic from cell centroid. Read-only. |
| `longitude_rad` | World-gen | Deterministic from cell centroid. Read-only. |
| `cell_area_m2` | World-gen (Lloyd) | Read-only; enforces area-aware density math. |

### No new carrier for "biome"

There is no `biome` carrier. Biomes are *labels* over the channels above (and the P2 soil/material channels), assigned by the Chronicler in `30_biomes_emergence.md`.

---

## 4. Update rules

### 4.1 Tick-loop placement

P1 work fits inside the existing 8-stage schedule (`architecture/ECS_SCHEDULE.md`). Concretely:

| Tick stage | P1 sub-step | Frequency |
|------------|-------------|-----------|
| Stage 0 (Input & Aging) | — | — |
| Stage 5 (Physiology) | Surface energy budget per cell (T_anom, albedo) | every tick |
| Stage 5 | Soil-moisture update (precip → infiltration → runoff) | every tick |
| Stage 6 (Ecology) | River routing (D∞ over neighbour graph) | every tick |
| Stage 6 | Boussinesq groundwater step | every K ticks (default K = 4) |
| Stage 7 (Labelling & Persistence) | EBM atmospheric step (heat & moisture transport) | every M ticks (default M = 24, "1 game day") |

The EBM atmospheric step is the most expensive (≈ 30 fixed-point ops × cell × neighbour). Sub-stepping at M = 24 keeps the tick budget within 16 ms even at 80 000 cells.

### 4.2 EBM atmospheric step (every M ticks)

Sketched in pseudocode; see `40_ecology_emergence.md` for the analogous ecology pattern.

```
fn ebm_step(world: &mut World) {
    // 1. Update insolation from time of year (Stage 14 calendar)
    for cell in world.cells.iter_sorted_by_id() {
        cell.insolation_W_m2 = solar_top_of_atm(
            cell.latitude_rad, world.axial_tilt, world.day_of_year);
    }

    // 2. Local energy budget: net radiation + latent + sensible
    for cell in world.cells.iter_sorted_by_id() {
        let net_sw = cell.insolation_W_m2 * (1 - cell.surface_albedo)
                       * (1 - cell.aerosol_optical_depth);
        let net_lw = STEFAN_BOLTZMANN_FP * pow4(cell.temperature_K())
                       * (1 - cell.cloud_fraction * GREENHOUSE_EFFECTIVE);
        let latent  = LATENT_HEAT_VAP * cell.evapotranspiration_kg_s();
        let sensible = SENSIBLE_HEAT_COEF * (cell.temperature_anom_K - air_anom);
        cell.dT_local = (net_sw - net_lw - latent - sensible) / heat_capacity(cell);
    }

    // 3. Meridional + zonal transport (Voronoi finite-volume diffusion)
    for cell in world.cells.iter_sorted_by_id() {
        let mut flux_in = Q32_32::ZERO;
        for &nbr_id in cell.neighbours.iter() {  // already sorted
            let edge = world.edge_between(cell.id, nbr_id);
            let grad = (world.cells[nbr_id].temperature_anom_K
                       - cell.temperature_anom_K) / edge.length_m;
            flux_in += DIFFUSIVITY_THERMAL * grad * edge.length_m;
        }
        cell.dT_transport = flux_in / cell.area_m2;
    }

    // 4. Apply
    for cell in world.cells.iter_sorted_by_id() {
        cell.temperature_anom_K += M as Q32_32 * (cell.dT_local + cell.dT_transport);
    }

    // 5. Moisture: same pattern with specific_humidity, capped at Clausius-Clapeyron saturation
    moisture_step(world, M);

    // 6. Wind diagnosis from pressure gradient (geostrophic + drag)
    diagnose_winds(world);
}
```

**Determinism notes.**
- `iter_sorted_by_id()` is mandatory.
- All multiplications are Q32.32 fixed-point. `pow4` and `exp` (for Clausius-Clapeyron) use the pre-computed lookup tables in `beast-primitives`.
- `DIFFUSIVITY_THERMAL` and similar coefficients are calibration constants (legitimate, not hardcoded taxonomy).
- The flux is *symmetric* (same coefficient at edge between A and B regardless of which cell evaluates it), so the energy budget closes exactly.

### 4.3 Hydrology

```
fn hydrology_step(world: &mut World) {
    for cell in world.cells.iter_sorted_by_id() {
        let infiltration = min(cell.precipitation_kg_m2_s,
                               max_infiltration_rate(cell.soil_texture));
        let surface_runoff = cell.precipitation_kg_m2_s - infiltration;
        cell.soil_moisture += infiltration / cell.soil_capacity_kg();
        cell.runoff_kg_m2_s = surface_runoff;
    }
    // D∞ routing: each cell distributes runoff to its lower neighbours,
    // partitioned by edge slope. Iterate over cells in topologically-sorted
    // (highest elevation first) order to compute river_discharge_m3_s in one pass.
    let order = world.cells_sorted_by_elevation_desc();
    for cell_id in order {
        let cell = &world.cells[cell_id];
        let outgoing = cell.runoff_kg_m2_s * cell.area_m2 / RHO_WATER;
        for (nbr_id, share) in cell.d_inf_flow_partition() {
            world.cells[nbr_id].river_discharge_m3_s += outgoing * share;
        }
    }
}
```

The `cells_sorted_by_elevation_desc()` ordering is bit-deterministic (stable sort on Q32.32 elevation, tie-break on cell id). D∞ partition is computed once at world creation from elevation + neighbour graph and cached.

### 4.4 Where the hardcoded v1 outputs come from in v2

| v1 hardcoded item | v2 replacement |
|-------------------|----------------|
| `biome_type: enum` | Cluster label assigned post-hoc by Chronicler (P3); never appears in sim code |
| Whittaker thresholds | Implicit in the prototype gallery used by the Chronicler |
| 3-cell Hadley/Ferrel/Polar pattern | Emerges from EBM meridional transport + insolation gradient |
| `compute_channel_modifiers(biome, ...)` | Removed; creature fitness reads the cell's continuous channels directly |
| `base_vegetation_regeneration_rate` | Computed in P4 from cell's `temperature_anom_K`, `precipitation_kg_m2_s`, `soil_moisture`, `soil_organic_carbon` (P2) |
| `lapse_rate` constant 6.5°C/km | Emerges in moist EBM from Clausius-Clapeyron + adiabatic cooling |
| `is_coastal`, `coast_moderation` | Coast effect emerges from sea heat capacity in the heat-capacity field |
| `is_windward` / `is_leeward` rain shadow | Emerges from orographic uplift parameterisation: `precipitation += k * max(0, w · ∇h)` |

The v1 doc has 7 "Open Calibration Knobs" — every one is preserved in v2 as a registered constant (e.g., `DIFFUSIVITY_THERMAL`, orographic coefficient, sea-ice albedo). Calibration constants are not the same as hardcoded taxonomies; the user explicitly accepted them.

---

## 5. Cross-pillar hooks

```mermaid
flowchart LR
    P1[P1 Environment]
    P2[P2 Materials]
    P3[P3 Biomes (labels)]
    P4[P4 Ecology]
    P5[P5 Social]
    P6[P6 Culture/etc]

    P1 -- "temperature_anom_K, precipitation, soil_moisture, insolation" --> P4
    P1 -- "temperature_anom_K, precipitation, runoff, river_discharge" --> P2
    P2 -- "surface_albedo (snow), evapotranspiration capacity, dust" --> P1
    P1 -- "all surface channels" --> P3
    P2 -- "soil composition" --> P3
    P4 -- "vegetation_lai, vegetation_cover" --> P3
    P4 -- "vegetation_cover (modifies albedo & evapotranspiration)" --> P1
    P1 -- "river_discharge_m3_s, climate stress" --> P6
    P5 -- "settlement footprint (LULC change)" --> P1
```

Reads and writes follow the standard `core-model/03_operators_and_composition.md` pattern: each system declares its read/write set on cell channels; the scheduler validates non-overlap of writes within a stage.

---

## 6. Tradeoff matrix

| Decision | Options | Sim Fidelity | Implementability | Player Legibility | Emergent Power | Choice + Why |
|----------|---------|--------------|------------------|-------------------|-----------------|-------------|
| Atmosphere | None / static 3-cell / **Moist EBM** / Shallow-water / Full GCM | EBM strong; GCM highest | EBM moderate | EBM legible (zonal bands emerge) | EBM strong | **Moist EBM**. Cheap; emergent ITCZ; matches sim-first philosophy. |
| Hydrology | Static rivers / **D∞ + Boussinesq groundwater** / Full SVAT | D∞ + Boussinesq strong | Moderate | River basins are visually clear | Strong (lakes form/dry, rivers shift with elevation) | **D∞ + simplified groundwater**; closes mass balance; emerges flood plains. |
| Sea ice | None / **Step-function albedo** / Thermodynamic ice | Mid; thermo highest | Step easy | Step legible (ice line visible) | Step gives ice-albedo feedback | **Step-function** at temperature_anom_K threshold. Reuse EBM machinery. |
| Cloud / radiation | Constant cloud / **Diagnosed cloud_fraction from RH** / Prognostic clouds | Diagnostic mid; prognostic highest | Diagnostic moderate | Both legible | Diagnostic gives wet/dry feedback | **Diagnostic** clouds; prognostic deferred. |
| Atmosphere update frequency | Every tick / Sub-cycled / **Multi-rate (M ticks per atmosphere step)** | Every tick highest fidelity | Multi-rate easiest at scale | Identical from outside | Identical | **Multi-rate (M = 24)**. Keeps within tick budget at 20 k+ cells. |
| Volcanism | None / Lookup / **Tectonic-event-driven `volcanism_intensity` channel feeding aerosol_optical_depth** | Tectonic mid | Mid | Visible: "year of ash" cooling | Strong: ash-driven climate perturbations | **Tectonic-event-driven**. Reuses P2 + atmospheric channels; no special case. |

---

## 7. Emergent properties

What v2 *exhibits* without scripting:

1. **ITCZ migration.** Seasonal latitude of peak precipitation tracks solar zenith. Tropics-belt rainforests appear naturally.
2. **Subtropical deserts.** Sinking-region dryness emerges from moisture-flux convergence pattern of the moist EBM, *not* from a `if 15° < lat < 35°: dry` rule.
3. **Rain shadows.** Orographic-uplift parameterisation makes the leeward slope of every mountain dry, deterministically.
4. **Coastal climate moderation.** High-heat-capacity ocean cells diffuse temperature to coastal land cells; the hardcoded `coast_moderation` term is gone.
5. **Ice-albedo feedback.** Sea-ice expansion at low temperature increases albedo, lowers absorbed shortwave, lowers temperature — bistable cold-Earth states are reachable. Snowball-earth-like regimes are possible from extreme volcanism.
6. **Milankovitch drift.** Slowly varying axial tilt and eccentricity (registered as global constants updated by the calendar system) shift biome belts on long timescales — entirely from the same EBM.
7. **River-driven biomes.** River cells have higher local soil moisture → wetlands / riparian forests as Chronicler-assigned labels in P3; nothing scripts this.
8. **Volcanic winters.** A high-`volcanism_intensity` event raises `aerosol_optical_depth`, lowers absorbed shortwave globally, drops temperatures — a plausible world-history event without a hardcoded "Year of Ash" trigger.

---

## 8. Open calibration knobs

These are calibration constants, not hardcoded taxonomies. All are registry-backed.

- `DIFFUSIVITY_THERMAL` — controls the equator-to-pole gradient. Higher → flatter gradient.
- `OROGRAPHIC_PRECIP_COEF` — strength of windward-uplift precipitation.
- `RUNOFF_TO_INFILTRATION_RATIO` — calibrates floodiness.
- `SEA_ICE_ALBEDO_THRESHOLD_K` — sea-ice line.
- `CLOUD_DIAGNOSTIC_GAIN` — RH-to-cloud-fraction sigmoid steepness.
- `EBM_M_TICKS_PER_STEP` — multi-rate sub-cycle.
- `GROUNDWATER_TRANSMISSIVITY_DEFAULT` — global aquifer flow scale.
- `AEROSOL_DECAY_PER_TICK` — how long volcanic ash lingers.

---

## 9. Determinism checklist

- ✅ All channel math in Q32.32.
- ✅ All neighbour iteration in sorted-id order.
- ✅ Multi-rate atmosphere step uses the same `M` for every cell; no skew.
- ✅ D∞ routing uses a topologically-sorted cell pass (deterministic by stable sort).
- ✅ One Xoshiro256++ stream per subsystem (`atmosphere`, `hydrology`, `volcanism`); no cross-contamination.
- ✅ No wall-clock dependencies.
- ✅ No floating point.
- ✅ Voronoi mesh is static after world creation; topology never re-computed.

---

## 10. Sources

- Budyko, M. I. (1969). "The effect of solar radiation variations on the climate of the Earth." *Tellus* 21.
- Sellers, W. D. (1969). "A global climatic model based on the energy balance of the earth-atmosphere system." *J. Appl. Met.* 8.
- Frierson, D., Held, I., Zurita-Gotor, P. (2006). "A gray-radiation aquaplanet moist GCM." *J. Atmos. Sci.* 63.
- Hill, S., Bordoni, S., Mitchell, J. (2017). "Solsticial Hadley cell ascending edge theory from supercriticality." *J. Atmos. Sci.* 74.
- North, G. R., Coakley, J. A. (1979). "Differences between seasonal and mean annual energy balance model calculations of climate and climate sensitivity." *J. Atmos. Sci.* 36.
- Skamarock, W. et al. (2012). MPAS-A. *MWR* 140.
- Ringler, T. et al. (2010). MPAS-O. *Ocean Modelling* 33.
- Tarboton, D. (1997). "A new method for the determination of flow directions and upslope areas in grid digital elevation models." *Water Resources Research* 33.
- Marçais, J., de Dreuzy, J.-R., Erhel, J. (2017). "Dynamic coupling of subsurface and seepage flows solved within a regularized partition formulation." *Advances in Water Resources* 109.
- Balsamo, G. et al. (2009). "A revised hydrology for the ECMWF model: verification from field site to terrestrial water storage and impact in the integrated forecast system." *J. Hydromet.* 10.
