# System 14: Calendar & Time

## 1. Overview

The Calendar & Time system is the **canonical clock** for the simulation. It defines:
- **TICK**: one game-day. All simulation updates use ticks as the unit of time.
- **Day/Month/Year cycles**: configurable period length (e.g., 10 ticks per day, 30 days per month, 12 months per year).
- **Seasonal cycles**: driven by axial tilt and Milankovitch-like orbital variations (System 15 reads these).
- **Time-of-day scheduling**: creatures have activity schedules (nocturnal, diurnal, crepuscular) that modulate foraging, mating, and migration based on hour_of_day.
- **Per-faction calendars**: each sapient faction (System 06) maintains its own calendar with custom naming, epochs, and era detection (shared with System 09 World History).

This system is **read by all others** to drive ticks, seasons, and temporal behavior. It is **written to only at initialization** and **passively incremented each tick**.

---

## 2. Research Basis

### Astronomical Time & Orbital Mechanics
- **Milankovitch cycles** (Milankovitch 1930s): Earth's climate varies with three orbital parameters: eccentricity (~100k yr), axial tilt/obliquity (~41k yr), precession (~23k yr). We model simplified precession for season shift.
- **Solar insolation**: varies by latitude (cosine of solar zenith angle) and season (axial tilt). Peak at equator year-round; poles vary 45°+ between summer/winter.
- **Lunar cycles**: approximately 29.5 days. Tides, creature behavior (spawning, hunting), tide-pool dynamics.

### Chronobiology & Circadian Rhythms
- **Entrainment**: creature activity is entrained to light/dark cycles; some species are diurnal, others nocturnal, others crepuscular (active at dawn/dusk).
- **Photoperiod**: day length varies by latitude and season. Triggers reproductive readiness, migration, hibernation (System 13, System 07).
- **Circadian clock**: internal 24-hour rhythm; slightly varies per individual (genetic). We model simple: activity_level[hour] = high if creature_preference_time matches hour_of_day.

### Chronological Record-Keeping
- **Calendar systems**: humans (and sapient creatures) track time with epochs (founding events). Gregorian, Islamic, Jewish, Chinese, Mayan calendars all have distinct starting points.
- **Era detection**: historical events (plagues, wars, discoveries) mark the boundary between epochs. System 09 (World History) uses this to structure lore.

---

## 3. Entities & State

### Global Time State

```
GlobalTime = {
  current_tick: int,              // Monotonically increasing; tick 0 = game start
  
  // Calendar configuration (set at initialization)
  ticks_per_day: int,             // e.g., 1 or 10 (granularity choice)
  days_per_month: int,            // e.g., 30
  months_per_year: int,           // e.g., 12
  
  // Derived cycles (read-only, computed)
  ticks_per_month: int,           // ticks_per_day * days_per_month
  ticks_per_year: int,            // ticks_per_month * months_per_year
  
  // Current time-of-day
  day_of_month: int,              // 1..days_per_month
  month_of_year: int,             // 1..months_per_year
  year_of_epoch: int,             // Years since epoch start
  hour_of_day: float,             // 0.0..23.999; subdivides day into 24 hours
  
  // Orbital parameters (Milankovitch-inspired)
  axial_tilt_degrees: float,      // Obliquity; ~23.5° for Earth. Varies ±1.3° over ~41k years
  orbital_precession_phase: float, // 0.0..1.0; cycles over ~23k years
  
  // Lunar phase (simple sine wave)
  lunar_phase: float,             // 0.0..1.0; 0=new, 0.5=full, back to 0
  
  // Season (derived from month + tilt)
  season: enum {SPRING, SUMMER, AUTUMN, WINTER},
  season_progress: float,         // 0.0 = start, 1.0 = end of season
  
  // Temperature & daylight (derived from latitude + season)
  // These are computed per-cell in System 15; stored here as global baseline
  global_temperature_baseline: float,  // Celsius; used as offset for biome temp
  daylight_hours_at_equator: float,   // Should be 12; varies at poles
  
  // Historical events & eras (shared with System 09)
  recorded_eras: [
    {
      era_name: str,
      start_tick: int,
      end_tick: int | null,
      trigger_event: str,  // "plague_peaked", "war_victory", "crop_failure", etc.
      discoverer_faction: faction_id,
    }
  ],
}
```

### Per-Faction Calendar (linked to System 06 Faction)

```
Faction.calendar = {
  faction_id: faction_id,
  
  // Custom naming
  epoch_name: str,               // e.g., "Age of Awakening"
  epoch_start_tick: int,         // When this faction "began counting time"
  
  // Month & day names (flavor; lore-only)
  month_names: [str, ...],       // e.g., ["Seedtime", "Bloomtide", ...]
  day_names: [str, ...],         // e.g., ["Moonday", "Starday", ...]
  
  // Era tracking (unique per faction; may diverge from global eras)
  cultural_eras: [
    {
      era_name: str,
      start_tick: int,
      trigger: str,
      description: str,          // Written by System 09 chronicler
    }
  ],
  
  // Observed events (holidays, festivals)
  holidays: [
    {
      name: str,
      month_of_year: int,
      day_of_month: int,
      significance: str,
    }
  ],
}
```

### Per-Creature Activity Schedule (linked to System 13 Creature)

```
Creature.activity_schedule = {
  creature_id: UUID,
  
  // Chronotype (genetic, from System 01 channels)
  circadian_preference: enum {DIURNAL, NOCTURNAL, CREPUSCULAR, ARRHYTHMIC},
  
  // Circadian phase offset (individual variation)
  phase_offset_hours: float,     // ±2 hours; causes individual variation in sleep
  
  // Activity level by hour (drives foraging, mating, migration)
  activity_level: [float; 24],   // 0.0..1.0 for each hour; sum may exceed 1
  
  // Reproductive seasonality (if any)
  breeding_season_months: [int, ...],  // e.g., [5, 6, 7] = May–July
  
  // Migration schedule (set by System 07)
  planned_migration_tick: int | None,
}
```

---

## 4. Update Rules

### Each Tick: Advance Clock (Global Phase 0)

```
Function advance_time():
  current_tick += 1
  
  // Compute time-of-day (hour within the day)
  tick_within_day = current_tick % ticks_per_day
  hour_of_day = (tick_within_day / ticks_per_day) * 24.0
  
  // Compute day, month, year
  total_days_elapsed = current_tick / ticks_per_day
  day_of_month = (total_days_elapsed % days_per_month) + 1
  month_of_year = ((total_days_elapsed / days_per_month) % months_per_year) + 1
  year_of_epoch = total_days_elapsed / (days_per_month * months_per_year)
  
  // Update orbital parameters (slow drift)
  // Precession: ~23,000 ticks per full cycle (if ticks_per_day = 1)
  orbital_precession_phase = (current_tick / 23000.0) % 1.0
  
  // Axial tilt variation (subtle; ±1.3° over ~41k years)
  tilt_variation = 1.3 * sin(2.0 * pi * (current_tick / 41000.0))
  axial_tilt_degrees = 23.5 + tilt_variation
  
  // Lunar phase (~29.5 day cycle)
  lunar_phase = (total_days_elapsed / 29.5) % 1.0
  
  // Season (derived from month + tilt)
  // Northern hemisphere: spring = months 3–5, summer = 6–8, autumn = 9–11, winter = 12–2
  // Shift by precession_phase for slow climate drift
  precession_shift = orbital_precession_phase * 2.0  // ±2 months drift
  effective_month = (month_of_year + precession_shift - 1.0) % months_per_year
  
  if effective_month in [2, 3, 4]:
    season = SPRING
    season_progress = effective_month - 2.0
  elif effective_month in [5, 6, 7]:
    season = SUMMER
    season_progress = effective_month - 5.0
  elif effective_month in [8, 9, 10]:
    season = AUTUMN
    season_progress = effective_month - 8.0
  else:  // [11, 12, 1]
    season = WINTER
    season_progress = (effective_month - 11.0) % 12.0
    
  // Global temperature baseline (higher in summer, lower in winter)
  global_temperature_baseline = 
    BASE_TEMPERATURE 
    + SEASONAL_AMPLITUDE * sin(2.0 * pi * (effective_month - 1.0) / 12.0)
```

### Update Creature Activity (Each Tick, Driven by Time-of-Day)

```
For each creature c in all cells:
  // Circadian entrainment
  phase_adjusted_hour = (hour_of_day + c.activity_schedule.phase_offset_hours) % 24.0
  
  // Look up activity level for this hour
  hour_index = floor(phase_adjusted_hour)
  activity_level = c.activity_schedule.activity_level[hour_index]
  
  // Adjust foraging, mate search, migration by activity level
  If activity_level < 0.2:
    c.foraging_rate *= 0.1  // Nearly asleep
  Else if activity_level > 0.8:
    c.foraging_rate *= 1.5  // Peak activity
  Else:
    c.foraging_rate *= 1.0 + (activity_level - 0.5)
    
  // Reproductive seasonality (check breeding_season_months)
  If month_of_year not in c.activity_schedule.breeding_season_months:
    c.fertility *= 0.5  // Reduced fertility outside breeding season
    
  // Migration triggers (set by System 07 based on photoperiod)
  If current_tick == c.activity_schedule.planned_migration_tick:
    System07.initiate_migration(c)
```

### Season-Driven Vegetation Regeneration (System 12 reads this)

```
For each cell:
  biome_type = cell.biome
  
  // Regeneration varies by season
  If season == SPRING:
    regeneration_multiplier = 2.0  // New growth
  Else if season == SUMMER:
    regeneration_multiplier = 1.5  // Peak growth
  Else if season == AUTUMN:
    regeneration_multiplier = 0.8  // Senescence
  Else:  // WINTER
    regeneration_multiplier = 0.2  // Dormancy
    
  System12.update_vegetation_regeneration(cell, regeneration_multiplier)
```

### Era Detection & Historical Recording (System 09 Chronicler calls this)

```
Function mark_era_boundary(trigger_event: str, affected_faction_id: faction_id | None):
  // Create a global era record
  new_era = {
    era_name: generate_era_name(trigger_event),  // e.g., "Year of the Plague"
    start_tick: current_tick,
    end_tick: None,
    trigger_event: trigger_event,
    discoverer_faction: affected_faction_id,
  }
  
  global_time.recorded_eras.append(new_era)
  
  // If affected_faction exists, also record in its cultural calendar
  If affected_faction_id:
    faction = System06.get_faction(affected_faction_id)
    cultural_era = {
      era_name: new_era.era_name,
      start_tick: current_tick,
      trigger: trigger_event,
      description: System09.generate_era_description(trigger_event),
    }
    faction.calendar.cultural_eras.append(cultural_era)
```

### Daylight Calculation by Latitude (System 15 uses this)

```
Function compute_daylight_hours(latitude_degrees: float, season: enum) -> float:
  // Simplified latitude effect (declination angle from season)
  declination = 23.5 * sin(2.0 * pi * (effective_month - 1.0) / 12.0)
  
  // Latitude + declination → sunset hour angle
  latitude_rad = latitude_degrees * pi / 180.0
  declination_rad = declination * pi / 180.0
  
  cos_hour_angle = -tan(latitude_rad) * tan(declination_rad)
  cos_hour_angle = clamp(cos_hour_angle, -1.0, 1.0)
  
  hour_angle = arccos(cos_hour_angle)
  daylight_hours = 24.0 * (hour_angle / pi)
  
  return daylight_hours
```

---

## 5. Cross-System Hooks

### Reads From:
- **System 07 (Exploration)**: creature latitude for daylight calculation; biome type for season response
- **System 09 (World History)**: historical events that trigger new eras (e.g., "plague ended", "war started")
- **System 15 (Climate & Biome)**: temperature baseline for season validation

### Writes To:
- **System 01 (Evolution)**: fitness penalties/bonuses for creatures not active during their preferred time (diurnal creature active at night = lower fitness)
- **System 07 (Exploration)**: daylight_hours affects creature activity ranges; migration photoperiod triggers
- **System 09 (World History)**: era_boundary events become chronicle entries
- **System 12 (Ecology)**: season_progress drives vegetation regeneration rates
- **System 13 (Reproduction & Lifecycle)**: breeding_season_months gate fertility; photoperiod (day_length) triggers developmental cues
- **System 15 (Climate & Biome)**: season + orbital parameters drive climate variation

### Reads/Writes With:
- **System 06 (Factions & Social)**: per-faction calendars, holidays, cultural era naming

---

## 6. Tradeoff Matrix

| Decision | Options | Sim Fidelity | Implementability | Player Legibility | Emergent Power | Choice + Why |
|----------|---------|--------------|------------------|-------------------|-----------------|-------------|
| **Tick granularity** | 1 tick = 1 day vs. 1 tick = 1 hour vs. 1 tick = 1 month | 1 hour highest detail | 1 day easiest | 1 day most intuitive | 1 day enables circadian rhythm, 1 hour enables seasonal tide | **1 tick = 1 day**: balances detail and computational load; 1 hour adds 24x overhead |
| **Year length** | 365 ticks vs. 360 ticks vs. 12 months x 30 days (360) vs. Configurable | 365 ticks Earth-like | 360 ticks (cleaner division) | 360 clearest | 365 allows Milankovitch cycles to sync realistically | **360 ticks/year (12×30)**: clean cycles; easy to tune without floating-point drift |
| **Lunar phase relevance** | No moon vs. Cosmetic only vs. Affects tide/creature behavior vs. Second astronomical body (orbital mechanics) | Second body orbital highest | No moon easiest | Cosmetic intuitive | Second body enables tidal migration, spawning events | **Cosmetic ~30-day cycle**: visible in lore; refactor to behavioral coupling later |
| **Per-faction calendars** | Global only vs. Faction-unique naming vs. Faction-unique epochs (divergent eras) | Unique epochs highest | Global only easiest | Naming-only clear | Unique epochs enable cultural drift (factions interpret history differently) | **Unique naming + epochs**: lore richness; enables divergent histories in System 09 |
| **Photoperiod entrainment** | None (activity level constant) vs. Fixed schedule per chronotype vs. Day-length-dependent vs. (Day-length + Social cues) | Full coupling highest | None easiest | Fixed schedule per type clear | Day-length-dependent enables migration triggers, breeding seasonality | **Fixed schedule + day-length penalties**: simple; flexible for creatures to adapt |

---

## 7. Emergent Properties

1. **Photoperiod-driven migration**: creatures with photoperiod-sensitive physiology automatically trigger migration when day length crosses thresholds. Different latitudes have different triggers. This creates seasonal pulses of movement (documented in System 07).
2. **Breeding season bottlenecks**: creatures that breed only in narrow months face population synchrony. This creates "generation cohorts" with synchronized lifespans and senescence. Visible as synchronized death waves.
3. **Nocturnal/diurnal niches**: creatures specialize to active times. Nocturnal predators hunt nocturnal prey; diurnal food chains are separate. Emergent temporal partitioning with less spatial overlap.
4. **Era-driven lore**: major events (plagues, famines, discoveries) mark era boundaries. Factions with different epochs tell different historical narratives (e.g., one faction counts years from a plague, another from a victory). Enables factional mythology.
5. **Milankovitch climate wobble**: over long timescales (thousands of ticks), axial tilt and precession cause slow climate drift. This selects for creatures that tolerate variation. Creates "climate epochs" in lore (e.g., "the slow warming age").

---

## 8. Open Calibration Knobs

- **TICKS_PER_DAY**: currently 1. Increase to 10 → finer hourly resolution, but 10x slower simulation. Decrease to 0.25 → 4 days per tick (very coarse; only if monthly-scale game).
- **MONTHS_PER_YEAR**: currently 12. Increase to 24 → shorter seasons, more frequent breeding windows. Decrease to 4 → longer seasons, slower ecological cycles.
- **DAYS_PER_MONTH**: currently 30. Increase to 45 → longer months, fewer breeding seasons per year. Change affects seasonal resource cycling.
- **SEASONAL_AMPLITUDE**: currently ±10°C from baseline. Higher → more extreme seasons, stronger selection for cold/heat tolerance. Lower → stable climate, diversity-homogenizing.
- **AXIAL_TILT_VARIATION_MAX**: currently ±1.3°. Higher → wilder climate swings, paleoclimate-like variation. Lower → stable poles, less polar activity.
- **BREEDING_SEASON_WINDOW**: species-tunable (months_list). Narrow (1–2 months) → population bottlenecks, high synchrony. Wide (all year) → steady breeding, no seasonal cohorts.
- **CIRCADIAN_PHASE_OFFSET_VARIATION**: currently ±2 hours. Higher → more individual variation in sleep timing. Lower → synchronized groups, more collective behavior.

