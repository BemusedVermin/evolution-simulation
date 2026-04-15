# System 15: Climate, Biome & Geology

## 1. Overview

The Climate, Biome & Geology system models the physical environment that creatures inhabit. It is the foundation for:
- **Climate**: global insolation, latitude bands, ocean currents (simplified), prevailing winds (Hadley/Ferrel/Polar cells), seasonal cycles from System 14.
- **Regional climate**: per-cell temperature, precipitation, humidity, wind derived from latitude, elevation, proximity to coast.
- **Biomes**: classified by climate (Whittaker diagram: temperature × precipitation) and geology (elevation, soil). Biomes determine base vegetation productivity and creature channel fitness.
- **Geology**: tectonic plates (slow motion), volcanism (thermal vents, ash), erosion (shape terrain over 100k+ ticks), hydrology (rivers from precipitation + topography).

Key constraint: **DO NOT duplicate biome cell data**. All biome types are referenced from System 07 (Exploration) cell.biome; this system computes derived climate values and productivity modifiers, reading/writing to the same cell objects.

This system is **read heavily by System 12 (Ecology)** for carrying_capacity and regeneration rates. It **reads from System 14 (Calendar)** for season and daylight. It **reads from System 01 (Evolution)** for channel fitness modifiers per biome.

---

## 2. Research Basis

### Solar Insolation & Latitude
- **Beer-Lambert law**: solar intensity ∝ sin(solar_zenith_angle). At equator, zenith angle = 0° (intense year-round). At poles, zenith varies 45°+ seasonally (extreme variation).
- **Hadley/Ferrel/Polar circulation cells** (Held & Hou 1980, simplified): air rises at equator (ITCZ—Intertropical Convergence Zone), sinks at ~30°N/S (subtropical high-pressure), rises at ~60° (polar front). Creates wind patterns and precipitation bands.
  - Hadley (0–30°): rising motion at equator, sinking at 30° → tropical rain at equator, desert at 30° (e.g., Sahara, Australian Outback).
  - Ferrel (30–60°): rising at 60°, sinking at 30° → temperate rain on western coasts (orographic lift from westerlies), rain shadow downwind.
  - Polar (60–90°): cold, dry, rising at pole edge, sinking at pole center.

### Regional Climate: Orographic Effect
- **Orographic precipitation**: air forced upslope cools adiabatically, precipitates on windward side. Leeward side is rain shadow (dry).
- **Elevation**: temperature decreases ~6.5°C per km altitude (adiabatic lapse rate).

### Biome Classification
- **Whittaker biome diagram** (Whittaker 1970): 2D space of mean annual temperature (x-axis) vs. mean annual precipitation (y-axis).
  - Low T, low P: tundra, polar desert.
  - Low T, high P: taiga (boreal forest).
  - Moderate T, low P: grassland, savanna, shrubland, desert.
  - Moderate T, high P: temperate forest, grassland.
  - High T, low P: hot desert, savanna.
  - High T, high P: tropical rainforest.
- **Köppen-Geiger classification** (Köppen 1900s, Geiger 1950s): climate zones based on monthly T/P thresholds; finer granularity than Whittaker.

### Primary Productivity
- **Productivity as function of (T, P)**: *Actual Evapotranspiration (AET)* is the limiting factor. Arid regions (low P) are limited by water. Polar regions (low T) are limited by energy. Tropical rainforests (high T, high P) are most productive.
- **Net Primary Productivity (NPP)** (Lieth 1975): rough formula NPP ≈ min(P_actual, 3000 mm/yr) × temperature_factor. Used to derive vegetation regeneration rate in System 12.

### Tectonic Plates & Geology
- **Plate tectonics** (Wegener 1912, Plate Tectonics Revolution 1960s): continental & oceanic plates move ~2–10 cm/year. Subduction, mid-ocean ridges, transform faults.
- **Volcanism**: at subduction zones (explosive) and mid-ocean ridges (effusive). Cooling lava forms new crust; thermal vents create chemosynthetic ecosystems (System 12 reads chemosynthesis).
- **Erosion & hydrology**: precipitation runs downslope → rivers. Rivers erode channels, deposit sediment in deltas. Slow process over geological timescales.

### Planetary Drift (from System 09 World History)
- Each Age (e.g., "Age of Awakening"), plates shift by a small amount (e.g., 0.5° longitude per Age). Landmasses slowly drift; islands move. Affects creature distribution and isolation.

---

## 3. Entities & State

### Per-Cell Climate Data (attached to System 07 Exploration cell)

```
Cell.climate = {
  cell_id: UUID,
  
  // Position (inherited from System 07)
  latitude_degrees: float,        // -90 (south pole) to +90 (north pole)
  longitude_degrees: float,       // Used for plate motion
  elevation_m: float,             // From System 07 topography
  
  // Derived climate values (recomputed each tick or each season)
  temperature_celsius: float,     // Influenced by latitude, elevation, season, ocean proximity
  precipitation_mm_per_year: float, // Influenced by circulation cells, orography, elevation
  humidity_percent: float,        // 0–100; affects drought stress
  wind_speed_mps: float,          // Meters per second; influenced by latitude band
  wind_direction: float,          // 0–360°; from dominant circulation cell
  
  // Derived from T & P: Biome assignment (Köppen/Whittaker)
  biome_type: enum {TROPICAL_RAINFOREST, SAVANNA, DESERT, GRASSLAND, TEMPERATE_FOREST, 
                     BOREAL_FOREST, TUNDRA, ALPINE, ESTUARY, CORAL_REEF, DEEP_OCEAN, 
                     VOLCANIC_VENT, COASTAL_SHELF, ...},  // Reference from System 07
  
  // Climate stability metrics
  temperature_variance_annual: float,  // Std dev of daily temps over year
  precipitation_seasonality: float,    // Coefficient of variation
  climate_stability_index: float,      // 0.0 = highly variable, 1.0 = stable
  
  // Geology
  geology = {
    rock_type: enum {BASALT, GRANITE, SEDIMENTARY, LIMESTONE, VOLCANIC_ASH},
    soil_fertility: float,          // 0.0 = barren, 1.0 = rich loam
    plate_boundary_type: enum {DIVERGENT, CONVERGENT, TRANSFORM, STABLE_INTERIOR},
    is_volcanic_active: bool,       // If true, thermal vents, ash output
    volcanic_ash_coverage: float,   // 0.0–1.0; affects light penetration, fertility
  },
  
  // Hydrology
  hydrology = {
    is_river_mouth: bool,          // Estuary
    upstream_watershed_area_km2: float,  // How much water drains into this cell
    groundwater_level_m: float,    // Depth to water table
    is_flooded: bool,              // Seasonal or permanent flooding
  },
  
  // Biome-derived productivity modifiers (fed to System 12)
  base_vegetation_regeneration_rate: float,  // kCal/tick baseline; scaled by AET
  base_carrying_capacity_modifier: float,    // Multiplier applied to K from System 12
  
  // Creature fitness modifiers by channel (read by System 01)
  channel_fitness_modifiers: {
    [channel_name]: float,  // e.g., cold_resistance: 1.5 in tundra, 0.5 in desert
  },
}
```

### Global Climate State

```
GlobalClimate = {
  // Ocean currents (simplified as latitudinal bands)
  ocean_current_strength: Dict[latitude_band, float],  // Modulates coastal temps
  
  // Plate configuration (slow-moving)
  tectonic_plates: [
    {
      plate_id: UUID,
      centroid_lon: float,
      centroid_lat: float,
      velocity_lon_cm_per_year: float,  // e.g., +2.5 cm/yr westward
      velocity_lat_cm_per_year: float,
      plate_vertices: [(lon, lat), ...],  // Boundary polygon
    }
  ],
  
  // Volcanic activity (linked to plate boundaries & vents)
  active_volcanoes: [
    {
      volcano_id: UUID,
      cell_id: UUID,
      eruption_intensity: enum {DORMANT, MILD, MODERATE, EXPLOSIVE},
      ash_column_km: float,
      ash_fallout_rate_kg_km2_per_day: float,  // Affects regional climate
    }
  ],
  
  // Global aerosol/dust from volcanism (affects insolation)
  atmospheric_optical_depth: float,  // 0.0 = clear, 0.1+ = significant dimming
}
```

### Per-Faction Geological Memory (from System 09 World History)

```
// In System 09 lore, major geological events (volcano, earthquake, tsunami) are recorded
// and tied to eras. System 15 references these to explain biome changes.
HistoricalEvent.geological_impact = {
  affected_cells: [cell_id, ...],
  climate_shift: float,  // Temporary T change (°C) from ash
  duration_ticks: int,   // How long ash persists in atmosphere
}
```

---

## 4. Update Rules

### Each Tick: Climate Computation (Global Climate Phase)

```
Function update_global_climate():
  // 1. Update solar insolation via System 14 (season, tilt, precession)
  season = System14.get_season()
  axial_tilt = System14.get_axial_tilt()
  
  // 2. Compute Hadley/Ferrel/Polar circulation (static model, tweaked by ocean currents)
  For each latitude band b in [-90, -60, -30, 0, 30, 60, 90]:
    if b in [0, ±30, ±60]:  // Rising/sinking nodes
      wind_direction[b] = compute_dominant_wind_direction(b, season)
```

### Each Season: Per-Cell Climate Update

```
For each cell c in world:
  // 1. Solar insolation (latitude + season)
  declination = 23.5 * sin(2π * (month - 1) / 12)
  solar_zenith = arccos(sin(latitude) * sin(declination) + cos(latitude) * cos(declination) * cos(hour_angle))
  
  insolation_fraction = max(0, sin(solar_zenith))  // Clamped to daytime
  daily_insolation = SOLAR_CONSTANT_MJ_m2 * insolation_fraction
  
  // 2. Base temperature (latitude + elevation)
  base_temp_sea_level = 
    30 - 0.5 * abs(latitude)  // Tropical ~30°C, polar ~0°C at sea level
  
  // Elevation effect (lapse rate = 6.5°C / km)
  lapse_rate = 0.0065  // °C per meter
  temp_sea_level = System14.get_global_temperature_baseline() + base_temp_sea_level
  temperature = temp_sea_level - (elevation_m / 1000.0) * 6.5
  
  // Ocean proximity: coast is warmer in winter, cooler in summer (moderating effect)
  if is_coastal:
    coast_moderation = 3.0 * (-season_progress)  // Negative = winter boost, positive = summer penalty
    temperature += coast_moderation
  
  // Volcanic ash: sunlight dimming
  if cell.geology.is_volcanic_active:
    c.temperature -= 1.0 + c.geology.volcanic_ash_coverage * 5.0
  
  c.temperature = temperature
  
  // 3. Precipitation (Hadley cell bands + orography)
  if latitude in [5°, 25°]:  // ITCZ — rising, high precip
    base_precip = 2000.0  // mm/yr
  elif latitude in [30°, 50°]:  // Sinking/rising boundaries — variable
    base_precip = 1000.0
  else:  // Subtropics sinking — deserts
    if latitude in [15°, 35°]:
      base_precip = 200.0
    else:  // Polar — very dry
      base_precip = 100.0
  
  // Orographic boost: windward slopes increase precip
  if elevation_m > 500 and is_windward:
    orographic_boost = (elevation_m / 1000.0) * 500.0  // +500mm per km on windward
    base_precip += orographic_boost
  
  // Rain shadow: leeward slopes decrease precip
  if elevation_m > 500 and is_leeward:
    rain_shadow_penalty = 0.5
    base_precip *= rain_shadow_penalty
  
  c.precipitation = max(0, base_precip)
  
  // 4. Humidity (function of precip, temperature, season)
  evaporation = temperature * 10.0  // Rough heuristic
  c.humidity = min(100, (c.precipitation / (evaporation + 1.0)) * 100.0)
  
  // 5. Wind (Hadley/Ferrel/Polar dominant direction + local terrain)
  c.wind_direction = dominant_wind_for_latitude(c.latitude)
  c.wind_speed = wind_speed_for_latitude(c.latitude) * (1.0 - 0.1 * elevation_m / 1000.0)
  
  // 6. Update volcanic ash (decay over time)
  if c.geology.volcanic_ash_coverage > 0:
    c.geology.volcanic_ash_coverage *= 0.95  // Decay each tick
  
  // 7. Update climate stability index
  c.climate_stability_index = compute_stability(
    temperature_variance=std_dev(historical_temps, window=365),
    precip_seasonality=std_dev(historical_precips) / mean(historical_precips),
  )
```

### Each Season: Biome Classification (Whittaker Diagram)

```
For each cell c:
  T = c.temperature
  P = c.precipitation
  
  // Simplified Whittaker rules (many transitions possible)
  if P < 250:
    if T < 0:
      c.biome = POLAR_DESERT
    elif T < 10:
      c.biome = TUNDRA
    else:
      c.biome = DESERT
      
  elif P < 1000:
    if T < 0:
      c.biome = ALPINE
    elif T < 10:
      c.biome = BOREAL_FOREST or GRASSLAND  // Depends on T+P balance
    else:
      c.biome = GRASSLAND or SAVANNA
      
  elif P < 2000:
    if T < 0:
      c.biome = TAIGA
    elif T < 15:
      c.biome = TEMPERATE_FOREST
    else:
      c.biome = SUBTROPICAL_FOREST
      
  else:  // P >= 2000
    if T < 5:
      c.biome = BOREAL_RAINFOREST
    else:
      c.biome = TROPICAL_RAINFOREST or MANGROVE
      
  // Special cases
  if c.elevation > 3000 and c.biome not in [ALPINE, POLAR_DESERT, TUNDRA]:
    c.biome = ALPINE
    
  if c.hydrology.is_flooded or c.hydrology.is_river_mouth:
    c.biome = ESTUARY or WETLAND
    
  // Assign derived productivity
  c.base_vegetation_regeneration_rate = compute_npp(T, P, c.climate_stability_index)
  c.base_carrying_capacity_modifier = npp_to_k_multiplier(c.base_vegetation_regeneration_rate)
  
  // Assign channel fitness modifiers (for System 01)
  c.channel_fitness_modifiers = compute_channel_modifiers(c.biome, T, P, c.elevation)
```

### Update Channel Fitness Modifiers by Biome

```
Function compute_channel_modifiers(biome, T, P, elevation):
  modifiers = {
    // All channels start at 1.0 (neutral)
  }
  
  // Temperature channels
  if T < -10:  // Extreme cold
    modifiers["cold_resistance"] = 1.5
    modifiers["metabolic_rate"] = 0.7  // Metabolism slowed in cold
  elif T < 0:  // Freezing
    modifiers["cold_resistance"] = 1.2
    modifiers["metabolic_rate"] = 0.85
  elif T > 35:  // Extreme heat
    modifiers["heat_resistance"] = 1.5
    modifiers["metabolic_rate"] = 0.8  // Heat stress
  elif T > 25:  // Warm
    modifiers["heat_resistance"] = 1.1
    
  // Precipitation/drought channels
  if P < 250:  // Arid
    modifiers["drought_tolerance"] = 1.4
    modifiers["digestion_efficiency"] = 0.9  // Sparse nutrition
  elif P > 1500:  // Wet
    modifiers["disease_resistance"] = 0.9  // More pathogens in wet
    modifiers["respiratory_efficiency"] = 1.1  // More O2 in air
    
  // Elevation channels
  if elevation > 2000:  // High altitude
    modifiers["oxygen_capacity"] = 1.2  // Need better lungs
    modifiers["metabolism"] = 0.8  // Energy expensive
    
  // Special biome traits
  if biome == AQUATIC or ESTUARY:
    modifiers["swimming_speed"] = 1.2
    modifiers["gill_efficiency"] = 1.3  // If creature has gills
    modifiers["salt_tolerance"] = varies_by_salinity
    
  if biome == VOLCANIC_VENT:
    modifiers["heat_resistance"] = 2.0
    modifiers["chemotaxis"] = 1.2  // Find chemical nutrients
    modifiers["cold_shock_recovery"] = 1.5  // Thermal vents fluctuate
    
  return modifiers
```

### Tectonic Plate Motion (Every 100 Ticks or Less Frequently)

```
For each plate p in world.tectonic_plates:
  // Advance plate position (slow)
  p.centroid_lon += p.velocity_lon_cm_per_year * TICK_TO_YEAR_CONVERSION  // Cm → degrees
  p.centroid_lat += p.velocity_lat_cm_per_year * TICK_TO_YEAR_CONVERSION
  
  // Update cell ownership (which cells are on this plate)
  for each cell c in cells_on_plate(p):
    c.longitude += velocity contribution
    // Biome may shift (move to different lat band) → recompute climate next season
    
  // Check for volcanic activity at convergent boundaries (subduction)
  for each boundary in plate_boundary(p):
    if boundary.type == CONVERGENT:
      // Probability of volcanic eruption (low, rare event)
      if random() < 0.01:  // 1% per tick at active zone
        volcano = trigger_eruption(boundary.location)
        System09.record_geological_event(volcano, "volcanic_eruption")
        System14.mark_era_boundary("volcanic_eruption")
```

---

## 5. Cross-System Hooks

### Reads From:
- **System 01 (Evolution)**: channel definitions and their allometric scaling; learns which channels matter in which biomes
- **System 07 (Exploration)**: cell location, elevation, biome_type (reference); reads topography for orographic effects
- **System 09 (World History)**: geological events (volcanoes, earthquakes) with climate impacts; long-term plate motion from Age progression
- **System 14 (Calendar & Time)**: season, month, axial tilt, precession for solar insolation; daylight hours per latitude
- **System 10 (Procgen Visuals)**: biome_type, temperature, precipitation for coloring (green forests, yellow deserts, white ice)

### Writes To:
- **System 01 (Evolution)**: channel_fitness_modifiers per cell (read during fitness calculation in System 01)
- **System 07 (Exploration)**: updates cell climate values; records cell biome_type
- **System 09 (World History)**: geological events (eruptions, earthquakes) become lore
- **System 12 (Ecology)**: base_vegetation_regeneration_rate, base_carrying_capacity_modifier per cell
- **System 13 (Reproduction & Lifecycle)**: gestation duration may scale with temperature stress; photoperiod (day_length) computed here
- **System 10 (Procgen Visuals)**: biome climate state for lighting, color scheme

### Reads/Writes With:
- **System 04 (Combat)**: climate affects combat performance (e.g., mud slows movement, snow visibility penalized)
- **System 07 (Exploration)**: weather events (storms, fog) drawn from wind, humidity, precipitation

---

## 6. Tradeoff Matrix

| Decision | Options | Sim Fidelity | Implementability | Player Legibility | Emergent Power | Choice + Why |
|----------|---------|--------------|------------------|-------------------|-----------------|-------------|
| **Circulation cells** | None (random climate) vs. Simplified Hadley/Ferrel/Polar vs. Full atmospheric GCM | Full GCM highest | Simplified easiest | Simplified intuitive (tropics wet, 30° dry) | Simplified enables predictable biome bands | **Simplified**: hard-coded 3-cell pattern; easily tunable; avoids heavy physics |
| **Orographic precipitation** | Ignore elevation vs. Linear elevation bonus vs. (Elevation + slope angle + wind) | Full coupling highest | Linear easiest | Linear clear ("mountains get rain") | Full coupling enables rain shadow speciation | **Linear boost/penalty**: 2–3x per km; refactor for slope later |
| **Biome classification** | Fixed lookup table vs. Whittaker 2D vs. Köppen 3-tier vs. Machine-learned clusters | Köppen/Whittaker highest | Lookup table easiest | Whittaker intuitive (can be visualized) | Whittaker continuous transitions, less discrete | **Whittaker with hard boundaries**: 2D space easy to visualize; refine transitions later |
| **Productivity model** | Fixed per biome vs. (T, P) function vs. Full NPP calculation vs. Nutrient cycling | Full nutrient cycling highest | Fixed easiest | (T, P) function intuitive | Full cycling enables soil depletion/recovery | **Simple (T, P) heuristic**: fast; refactor to nutrient state later |
| **Plate tectonics** | None (static world) vs. Drift (plates move, cells shift) vs. Active volcanism vs. (+ Earthquakes) | Full active system highest | None easiest | Drift visible over 1000s of ticks | Active volcanism triggers lore events, climate perturbations | **Plate drift + volcanism (no earthquakes initially)**: slow, deterministic; adds geological time scale |
| **Volcanic ash persistence** | Instant removal vs. Decay curve vs. (Decay + Wind dispersal) | Dispersal highest | Instant easiest | Decay curve visible as temporary climate | Decay curve allows volcano-triggered climate events (famine, migration) | **Exponential decay (~95% per tick)**: observable 10–20 tick impact; refactor for transport later |

---

## 7. Emergent Properties

1. **Biome-driven speciation**: creatures specialize to biome channel modifiers. A species that evolves high drought_tolerance dominates deserts, while cold_resistance specialists dominate tundra. Different biomes host different communities (observable biodiversity map).
2. **Rain shadow speciation**: populations on windward and leeward sides of mountains experience different precipitation regimes. Genetic divergence occurs despite close proximity (vicariance speciation).
3. **Volcanic perturbation cascades**: eruption → ash → temperature drop → vegetation decline → herbivore starvation → predator famine → social chaos (System 06 unrest, lore event). Documentable in System 09 as "Year of Ash."
4. **Milankovitch-driven climate drift**: over thousands of ticks, tilt and precession shift biome boundaries. Species must evolve or migrate. Some species "follow" their biome; others go extinct. Creates long-term evolutionary pressure distinct from short-term ecology.
5. **Coastal refugia**: coastal cells buffer temperature extremes. During harsh inland conditions, coastal populations survive. Evolution happens in refugia; species radiate back inland when conditions improve. Enables lineage diversity.

---

## 8. Open Calibration Knobs

- **HADLEY_CELL_PRECIPITATION_PEAKS**: currently [2000 mm at 0°, 200 mm at 30°]. Adjust peaks to increase/decrease desert aridity. Higher desert precip → deserts become grasslands.
- **OROGRAPHIC_PRECIPITATION_BOOST**: currently +500 mm/km elevation. Increase to +750 mm/km → windward mountains become wet forests. Decrease to +250 mm/km → mountains stay moderate.
- **LAPSE_RATE**: currently 6.5°C/km. Real Earth is 5.5–6.5°C/km depending on humidity. Increase to 8.0 → mountains cold faster, isolates high-altitude creatures. Decrease to 5.0 → easier high-altitude survival.
- **BASELINE_PRODUCTIVITY_SCALE**: currently ~100–3000 kCal/tick per cell. Increase 2x → all cells support larger K, less starvation pressure. Decrease 0.5x → tighter niches, faster K cycling.
- **VOLCANIC_ERUPTION_PROBABILITY**: currently 1% per tick at convergent zone. Increase to 3% → frequent eruptions, high perturbation, lore-rich. Decrease to 0.3% → rare catastrophes.
- **PLATE_VELOCITY**: currently ±2–10 cm/year. Increase to 50 cm/year → continents move noticeably over 500 ticks. Decrease to 1 cm/year → geological change imperceptible in short games.
- **ASH_DECAY_RATE**: currently 95% per tick (20-tick half-life). Decrease to 90% per tick → ash lingers longer, deeper climate impact. Increase to 98% → fast recovery.
- **CHANNEL_FITNESS_EXTREMES**: e.g., cold_resistance modifier in tundra. Currently 1.5x. Increase to 2.0x → strong selection for cold resistance. Decrease to 1.2x → weaker pressure, more generalism.

