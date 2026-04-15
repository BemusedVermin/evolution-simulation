# Exploration System: Simulation-First Navigation & Discovery

## 1. Overview & Sim-First Stance

Exploration is the **player's interface into an active simulation** of a flooded world with evolving ecosystems, drifting factions, and decaying old-world infrastructure. The player does not uncover a static map; they navigate through a world that changes based on ecological dynamics, NPC activity, and passage of time.

**Core simulation principle**: Every visible element on the map — creature presence, settlement position, POI state, weather patterns, ship position — is the output of continuous background simulation. The player's exploration decisions (which route to sail, where to land, how long to spend in a region) are meaningful because they have real consequences in a dynamic world.

**Player's primary resource**: Information. The map is the player's knowledge store. Fog of war is not a UI mechanic; it represents actual epistemic uncertainty. Map knowledge can be gathered through scouting, traded with NPCs, or discovered through encounters. Known routes are faster to sail because the player has charts. Unknown regions are dangerous because the player lacks information about hazards and creatures.

---

## 2. Research Basis

### Foundational Hexcrawl & Pointcrawl References

**Justin Alexander — Hexcrawl Series (The Alexandrian)**
The authoritative modern hexcrawl design framework. Key principles adopted:
- Hexes (or in our case, biome cells) are a GM-side organizational tool invisible to the player.
- Discovery is probabilistic: entering a hex does not automatically reveal its content.
- Navigation uncertainty (getting lost) is a core mechanic that creates exploration tension.
- Every region should have at least one reusable keyed location (POI).

- Alexander, J. "Hexcrawl." *The Alexandrian*. https://thealexandrian.net/wordpress/17308/roleplaying-games/hexcrawl

**Pointcrawl Design (The Alexandrian & Hill Cantons)**
Pointcrawls (node-and-edge networks) excel at creating meaningful route choices. Rather than 360-degree hex freedom, players choose between 2–3 significant destinations, each with known or suspected hazards. This produces information-rich decisions: "Do I take the safe route that's slow, the fast route that's dangerous, or the unknown route?"

- Alexander, J. "Pointcrawls." *The Alexandrian*. https://thealexandrian.net/wordpress/48666/roleplaying-games/pointcrawls
- Kutalik, C. "Hexcrawls vs Pointcrawls." *Hill Cantons*. 2016.

**Prismatic Wasteland — Hexcrawl-Pointcrawl Combo**
Proposes overlaying a pointcrawl (macro-scale navigation between regions) on a hexcrawl (micro-scale exploration within regions). In our project: archipelago-scale is pointcrawl (choose sea routes), island-scale is continuous exploration (free movement).

- Prismatic Wasteland. "Hexcrawl Checklist: Part Two." 2023.

### Video Game Exploration Models

**Darklands (MicroProse, 1992)**
Continuous overworld map with no visible grid. Party is an icon traversing painted terrain. Encounters and discoveries are probabilistically triggered based on position and time. Environmental factors (season, weather, time of day) dramatically affect encounter likelihood and travel speed. The design insight: a continuous-space map with procedural filling and authored anchors (major cities) feels real and vast while remaining hand-crafted.

- MicroProse. *Darklands*. 1992. DOS.

**Sunless Sea (Failbetter Games, 2015)**
Nautical exploration with semi-randomized map and fixed landmark anchors. Terror (morale/sanity) rises in darkness and open water, creating psychological pressure. Resource management (fuel, supplies, hull integrity) makes exploration costly. The inheritance for our project: exploration has real costs, and returning from an expedition is itself a meaningful decision.

- Failbetter Games. *Sunless Sea*. 2015.

**The Legend of Zelda: Breath of the Wild (Nintendo, 2017)**
The "Triangle Rule" for landmark placement: terrain features shaped as triangular peaks that obscure what's beyond and guide the eye toward points of interest. Three scales: large landmarks for wayfinding, medium terrain for view obstruction, small features for moment-to-moment rhythm. The design insight: the player's exploration loop (see something → investigate → spot something new → repeat) is driven by **visual landmarks**, not UI markers. Information is gathered by looking, not by consulting a quest log.

- Nintendo. CEDEC 2017 Talks on Breath of the Wild. https://www.youtube.com/watch?v=XOC3vixFSpY

**Fog of War Systems**
Three-state visibility model: Unexplored (unknown), Explored-but-Shrouded (known terrain, hidden dynamic elements), Visible (full information). Originated in *Empire* (1977), popularized by Warcraft II. The core insight: information asymmetry creates meaningful scouting decisions and makes discovery emotionally rewarding.

- Design the Game. "The Art and Science of Fog of War Systems in Video Games." 2026.

---

## 3. Entities & State

### World Structure: Continuous Terrain

The world is a heightmap-based continuous space, not a visible hex grid:

```
WorldMap {
    // Continuous geometry
    heightmap:          float[WIDTH][HEIGHT]        // elevation data
    sea_level:          float                       // global water line
    
    // Derived fields (computed once at load)
    land_mask:          bool[WIDTH][HEIGHT]         // true where elevation > sea_level
    depth_map:          float[WIDTH][HEIGHT]        // underwater depth
    shore_distance:     float[WIDTH][HEIGHT]        // signed distance to coastline
    
    // Authored structure
    archipelagos:       list<Archipelago>           // named island clusters
    ocean_currents:     list<OceanCurrent>          // directional flow vectors
    wind_patterns:      WindModel                   // seasonal wind patterns
    
    // Simulation substrate
    biome_grid:         BiomeCell[GRID_W][GRID_H]  // invisible grid from System 01
    
    // POI layer
    points_of_interest: list<PointOfInterest>       // all discoverable locations
}

Archipelago {
    id:                 unique_id
    name:               string
    center:             Vec2                        // world-space
    islands:            list<Island>
    sea_routes:         list<SeaRoute>              // connections to other archipelagos
    discovery_state:    DiscoveryState              // player knowledge of this cluster
}

Island {
    id:                 unique_id
    contour:            Polygon                     // derived from heightmap
    area:               float
    peak_elevation:     float
    terrain_profile:    TerrainProfile
    biome_cells:        list<cell_id>               // which biome cells overlap this island
    points_of_interest: list<poi_id>                // keyed locations on this island
    landing_sites:      list<LandingSite>
}
```

**Design rationale**: A continuous heightmap preserves the aesthetic of Darklands (a painted, seamless world) while the invisible BiomeCell grid (from System 01) provides the simulation substrate. The player never sees or thinks about cells; they navigate terrain and landmarks.

### POI Discovery State Machine

Points of interest progress through explicit state transitions, not random probability clouds:

```
DiscoveryState = enum {
    Undiscovered,       // Player has no knowledge of this POI's existence
    Detected,           // Player has observed the POI (via scouting/landmark visibility)
    Visited,            // Player has been within interaction range and chosen not to avoid
    Explored,           // Player has spent sufficient time in the POI to learn its major secrets
    Exhausted,          // For renewable POIs: retrievable resources dropped below threshold
}

function resolve_discovery_transition(poi: PointOfInterest, player_state: PlayerState):
    match poi.discovery_state:
        Undiscovered:
            // Transition to Detected when player can see it
            if player_can_see_poi(player_state, poi):
                poi.discovery_state = Detected
                notify_player(f"You spot {poi.description} on the horizon.")
                // Record in player's KnowledgeStore (System 03)
        
        Detected:
            // Transition to Visited when player moves within interaction range
            // AND has not been avoiding the POI
            distance_to_poi = distance(player_state.position, poi.position)
            if distance_to_poi < poi.interaction_range:
                if not player_is_avoiding(poi):
                    poi.discovery_state = Visited
                    player_state.knowledge_store.add_known_location(poi.id)
                    // Optional: trigger encounter or opening narration
        
        Visited:
            // Transition to Explored when player has spent enough time
            time_in_poi = current_tick - poi.last_entered_tick
            required_time = compute_exploration_threshold(poi)
            
            if time_in_poi > required_time:
                poi.discovery_state = Explored
                // Unlock lore fragments, map details
                unlock_poi_knowledge(poi)
        
        Explored:
            // Transition to Exhausted when renewable resources are depleted
            if poi.is_renewable:
                resource_fraction = current_resources / initial_resources
                if resource_fraction < 0.1:
                    poi.discovery_state = Exhausted
                    poi.respawn_timer = compute_respawn_time(poi)
        
        Exhausted:
            // For renewable POIs, wait for respawn timer
            if current_tick > poi.respawn_timer:
                reset_poi_resources(poi)
                poi.discovery_state = Visited  // cycle back

function compute_exploration_threshold(poi: PointOfInterest) -> int:
    // Exploration time depends on POI complexity
    base_threshold = 10  // ticks (min)
    
    complexity_factor = poi.size * poi.depth  // larger, deeper POIs take longer
    hazard_factor = len(poi.environmental_hazards) * 0.5  // hazards increase exploration time
    
    threshold = base_threshold + (complexity_factor * 5) + (hazard_factor * 3)
    
    return ceil(threshold)
```

**Design consequence**: Exploration is not binary (discovered/not discovered). The player must actively visit a POI, spend time there, and extract knowledge. A ruin can be "detected" from a distance (seen from a ship) but "exhausted" only after careful excavation. This makes exploration a real cost, not a checkbox.

### Fog of War: Freshness & Staleness

Map knowledge degrades over time as the world changes:

```
KnowledgeFact {
    // What we know
    subject:            entity_id or location_id    // what this fact is about
    fact_type:          enum { POI_exists, creature_presence, settlement_status, 
                               route_hazard, resource_location }
    content:            any                         // the actual knowledge
    
    // Knowledge quality
    freshness:          float                       // [0.0, 1.0]
    source:             InfoSource                  // direct observation vs rumor
    
    // Decay
    last_observed_tick: int
    decay_rate:         float                       // how fast this fact becomes stale
}

function decay_knowledge_freshness(fact: KnowledgeFact, world_state: WorldState):
    // Freshness decays based on activity in the region
    
    ticks_since_observation = current_tick - fact.last_observed_tick
    
    // Get the biome cell containing this fact's subject
    cell = world_state.biome_grid.cell_at(fact.subject.position)
    
    // Active areas (high creature activity, NPC presence, weather events)
    // have faster knowledge decay
    regional_activity = cell.creature_activity_level + cell.npc_presence_level
    
    // Decay curve: freshness drops over time, faster in active regions
    base_decay = 0.02  // per tick (knowledge "half-life" ~50 ticks)
    activity_multiplier = 1.0 + regional_activity * 2.0
    
    total_decay = base_decay * activity_multiplier * ticks_since_observation
    fact.freshness = max(0.0, fact.freshness - total_decay)
    
    return fact.freshness

function render_fact_to_player(fact: KnowledgeFact):
    if fact.freshness > 0.7:
        return f"{fact.content}"  // full knowledge
    elif fact.freshness > 0.4:
        return f"{fact.content} [last seen {ticks_ago(fact)} ticks ago]"
    else:
        return f"{fact.content}? [may be outdated; high uncertainty]"  // uncertain overlay
```

**Design rationale**: The player's map is not a completed, static thing. Returning to a region after many ticks reveals changes: creatures have moved, settlements have grown or declined, ruins have collapsed further. This makes exploration and revisit meaningful and preserves the world-turns-without-you principle.

### Navigation Uncertainty: Brownian Drift

Without proper navigation instruments, the player's displayed position drifts from true position:

```
NavigationState {
    true_position:      Vec2
    displayed_position: Vec2                    // what player sees
    position_uncertainty: float                 // σ of Brownian motion
    
    has_compass:        bool
    has_sextant:        bool
    has_detailed_charts: Map<route_id, bool>    // known routes have accurate charts
}

function resolve_navigation_uncertainty(vessel: Vessel, dt: float):
    // Position uncertainty grows as sqrt(time)
    // This is Brownian motion: noise accumulates
    
    if vessel.has_compass:
        // Compass prevents heading drift
        pass
    else:
        // Without compass, heading drifts randomly
        heading_drift = random_normal(0, 0.1) * dt
        vessel.heading += heading_drift
    
    if vessel.has_sextant:
        // Sextant fixes position, resetting uncertainty
        vessel.position_uncertainty = 0.0
    else:
        // Uncertainty grows with sqrt(time)
        uncertainty_growth_rate = 0.05  # per sqrt(tick)
        ticks_since_fix = current_tick - vessel.last_position_fix_tick
        vessel.position_uncertainty = sqrt(ticks_since_fix) * uncertainty_growth_rate
    
    // Apply uncertainty to displayed position
    // Player sees a fuzzy location, not exact position
    fuzzy_offset = random_in_circle(0, vessel.position_uncertainty)
    vessel.displayed_position = vessel.true_position + fuzzy_offset
    
    return vessel.displayed_position

function apply_position_fix(vessel: Vessel, fix_method: FixMethod):
    // Fix events: landmark sighting, sextant reading, arrival at known POI
    match fix_method:
        LandmarkSighting(landmark):
            vessel.true_position = snap_to_landmark(vessel.true_position, landmark)
            vessel.last_position_fix_tick = current_tick
            notify_player(f"You recognize {landmark.name}. Position confirmed.")
        
        SextantReading():
            // Sextant fixes position but takes time to use
            vessel.position_uncertainty = 0.0
            vessel.last_position_fix_tick = current_tick
        
        PoiArrival(poi):
            // Arriving at a known POI fixes position
            vessel.true_position = poi.position
            vessel.last_position_fix_tick = current_tick
```

**Anti-fudge #1**: Navigation uncertainty is **not** automatic random failure. Instead, it's Brownian drift: uncertainty accumulates predictably (proportional to sqrt(time)) and is fixable through landmarks, instruments, or known POIs. This is more simulation-coherent than random encounters.

### Encounter Generation: Emergent Presence

Encounters do **not** come from an abstract "encounter die." They come from actual creature presence in cells:

```
EncounterRoll {
    // REMOVE: abstract D20 roll against monster power
    // REPLACE: actual creature density computation
}

function compute_encounter_probability(cell: BiomeCell, party_state: PartyState, 
                                      weather: WeatherState, visibility: float) -> float:
    // Probability that an encounter triggers this tick
    
    // Base probability from creatures actually present in this cell
    // (from System 01: PopulationDynamics)
    total_encounter_prob = 0.0
    
    for creature_presence in cell.creature_populations:
        species = creature_presence.species_id
        density = creature_presence.population_count / cell.carrying_capacity
        
        // Encounter chance scales with density
        base_encounter_chance = creature_presence.encounter_probability_per_individual
        density_scaled = base_encounter_chance * density
        
        // Visibility affects whether creatures notice the party
        visibility_detection = visibility * party_state.party_stealth
        detection_scaling = visibility_detection * 0.8 + 0.2  // min 20% detection even hidden
        
        // Weather affects encounter rate
        weather_scaling = weather.encounter_modifier  // storms increase encounters, fog decreases
        
        encounter_prob_species = density_scaled * detection_scaling * weather_scaling
        total_encounter_prob += encounter_prob_species
    
    // Clamp to valid probability
    return clamp(total_encounter_prob, 0.0, 1.0)
```

**Design consequence**: Creature encounters are a consequence of actual creature populations. A cell with many evolved wolves has a high encounter rate. A cell with no creatures has zero encounters. The player can infer creature presence from encounter frequency, and spending time in a cell makes encountering creatures more likely (confirming the population).

### Sea-Route Fast Travel: Autonomous Sailing

There is **no "fast travel" convenience wrapper**. Travel is always physical, but the player can time-skip:

```
SeaRoute {
    id:                 unique_id
    origin_archipelago: archipelago_id
    destination_archipelago: archipelago_id
    distance:           float                       // world units
    base_travel_time:   float                       // ticks at standard speed
    difficulty:         float                       // [0.0, 1.0]
    
    // Simulation data
    current_alignment:  float                       // wind/current alignment [-1, 1]
    wind_alignment:     float
    hazards:            list<RouteHazard>
    known_by_factions:  set<faction_id>             // who has charts
    
    // Player knowledge
    player_known:       bool
    player_has_chart:   bool                        // good navigation information
    discovery_method:   enum { Sailed, Told, MapFound, Rumored }
}

function resolve_sea_voyage(vessel: Vessel, route: SeaRoute, player_time_skip: bool):
    // Sailing always happens in the simulation
    // The player can time-skip (accelerate ticks) while watching autonomously
    
    while not route_completed:
        if player_time_skip:
            // Accelerated time: each UI frame = multiple ticks
            # Vessel autonomously sails the route
            sailing_tick(vessel, route)
        else:
            // Real-time sailing: player controls vessel directly
            sailing_tick(vessel, route)
        
        // Hazard checks (occur whether time-skipping or not)
        for hazard in route.hazards:
            if hazard.position_along_route == vessel.position_fraction:
                trigger_hazard(hazard, vessel)
        
        // Encounter check
        if random() < route.encounter_probability_this_tick:
            trigger_encounter(vessel, route)
        
        // Resource consumption (fuel, supplies) happens regardless
        vessel.fuel -= fuel_consumption_rate(vessel.speed, vessel.mass)
        vessel.supplies -= supply_consumption_rate(vessel.crew_count)
        
        if vessel.fuel <= 0:
            trigger_shipwreck(vessel)
            break

function sailing_tick(vessel: Vessel, route: SeaRoute):
    // Autonomous sailing behavior
    
    if vessel.has_chart(route.id):
        // With a good chart, navigate directly
        heading_correction = optimal_heading_to_destination(vessel, route)
    else:
        // Without a chart, navigate by legend/rumor
        heading_correction = random_walk_toward_destination(vessel, route, std_dev=0.3)
    
    vessel.heading = lerp(vessel.heading, heading_correction, 0.3)  # smooth correction
    
    // Update position based on wind/current
    effective_thrust = vessel.engine_power
    effective_thrust += dot(wind, vessel.heading) * SAIL_COEFFICIENT
    effective_thrust += dot(current, vessel.heading) * CURRENT_COEFFICIENT
    
    vessel.speed = clamp(effective_thrust / vessel.mass, 0, vessel.max_speed)
    vessel.position += direction(vessel.heading) * vessel.speed * dt
    
    // Update resources
    morale_drain = (morale_drain_base 
                    - vessel.crew_morale * 0.1
                    + weather_severity * 0.2)
    vessel.crew_morale = max(0.0, vessel.crew_morale - morale_drain * dt)
    
    if vessel.crew_morale < 0.0:
        trigger_mutiny(vessel)
```

**Design consequence**: "Fast travel" is not a UI convenience. It's the player choosing to time-accelerate while the simulation continues. The vessel consumes resources, encounters can still trigger, and hazards still occur. The player saves time on input but doesn't skip the simulation.

### Route Danger Rating: Aggregated Hazard

```
function compute_route_danger_rating(route: SeaRoute) -> float:
    // Derived from aggregated hazard severity and frequency
    
    total_danger = 0.0
    
    for hazard in route.hazards:
        hazard_severity = hazard.severity * hazard.encounter_frequency
        
        if hazard.seasonal:
            // Seasonal hazards appear only certain times of year
            seasonal_presence_factor = compute_seasonal_presence(hazard, current_season)
        else:
            seasonal_presence_factor = 1.0
        
        total_danger += hazard_severity * seasonal_presence_factor
    
    // Normalize to [0.0, 1.0]
    danger_rating = clamp(total_danger / route.hazards.len(), 0.0, 1.0)
    
    return danger_rating
```

### Creature Territories: Population-Derived

Creature presence in a cell emerges from population dynamics, not fixed spawning tables:

```
function determine_creature_presence(cell: BiomeCell) -> list<CreatureInstance>:
    // Creatures in this cell come from PopulationDynamics (System 01)
    # No random encounter table; presence is computed from simulation
    
    present_creatures = []
    
    for species_population in cell.species_populations:
        # Population dynamics have already computed how many of this species
        # exist in the cell (this tick)
        population_count = species_population.current_population
        
        # Creatures are active/inactive based on the time of day, weather, visibility
        active_ratio = compute_activity_ratio(species_population.species,
                                              current_time_of_day,
                                              weather,
                                              visibility)
        
        active_count = ceil(population_count * active_ratio)
        
        for i in range(active_count):
            creature = spawn_creature_instance(species_population.species,
                                               cell,
                                               species_population.phenotype_seed)
            present_creatures.append(creature)
    
    return present_creatures
```

**Design consequence**: A cell that has evolved to contain many intelligent, organized creatures (e.g., high intelligence channel populations) will have "encounters" that are actually territorial patrols. A cell with solitary hunters will have rare, dangerous encounters.

### Salvage Availability: History-Derived

Salvageable materials emerge from world history, not loot tables:

```
function compute_salvage_in_region(cell: BiomeCell, poi: PointOfInterest) -> list<MaterialStack>:
    # REMOVE: abstract loot tables
    # REPLACE: salvage is the literal remains of old-world history
    
    salvage = []
    
    if poi.type == Ruin_Surface or poi.type == Ruin_Submerged:
        # Ruins contain materials based on their era
        # (era is defined in System 09: Lore & World History)
        era = poi.era
        
        # Each era has a fixed set of material signatures
        # (this defines what materials the old world made)
        era_materials = get_era_material_signatures(era)
        
        # Salvage quantity depends on ruin degradation
        degradation = poi.degradation_level
        base_yield = poi.initial_material_amount
        available_yield = base_yield * (1.0 - degradation)
        
        # Distribute among era materials
        for material_type in era_materials:
            quantity = available_yield * (material_type.frequency_in_era / sum_frequencies)
            material_stack = MaterialStack {
                material_signature: material_type.signature,
                quantity: quantity,
                freshness: compute_material_freshness(material_age),
            }
            salvage.append(material_stack)
    
    return salvage
```

**Design consequence**: Old-world ruins always yield "old-world materials" with specific material signatures reflecting their era. A 2050s-era industrial facility yields industrial-era materials (high conductivity, low vitality). A 2200s-era biotech facility yields biotech-era materials (high vitality, lower density). Salvage is not random; it tells the story of the world before it drowned.

---

## 4. Update Rules

### Exploration Tick

Each simulation tick (while in exploration mode), the following happen:

```
function exploration_tick(world: World, vessel_or_party: MovingEntity, dt: float):
    // Update creature populations in all cells (System 01)
    for cell in world.biome_grid.all_cells():
        update_population_dynamics(cell, dt)
    
    // Update NPC activity & faction state (System 03)
    update_faction_presence(world, dt)
    
    // Update weather (handled separately; see below)
    update_weather(world, dt)
    
    // Decay all knowledge facts
    for fact in world.knowledge_store.all_facts:
        decay_knowledge_freshness(fact, world)
    
    // Move the player entity (vessel or party)
    move_entity(vessel_or_party, player_input, dt)
    
    // Update navigation uncertainty
    resolve_navigation_uncertainty(vessel_or_party, dt)
    
    // Check for POI discovery
    for poi in world.nearby_pois(vessel_or_party.position, vessel_or_party.discovery_range):
        resolve_discovery_transition(poi, vessel_or_party)
    
    // Check for encounters
    current_cell = world.biome_grid.cell_at(vessel_or_party.position)
    encounter_prob = compute_encounter_probability(current_cell, vessel_or_party, 
                                                   world.weather, visibility_level)
    if random() < encounter_prob * dt:
        creatures = determine_creature_presence(current_cell)
        trigger_encounter(vessel_or_party, creatures, current_cell)
    
    // Update POI state (resource renewal, decay, NPC activity)
    for poi in world.nearby_pois(vessel_or_party.position, POI_UPDATE_RANGE):
        update_poi_state(poi, dt)
```

### Weather System: Hierarchical Stochastic

Weather is **not** randomly rolled. It's a continuous hierarchical model:

```
function update_weather(world: World, dt: float):
    // Layer 1: Global seasonal pattern
    seasonal_phase = (current_tick % YEAR_LENGTH) / YEAR_LENGTH
    global_weather_envelope = compute_seasonal_envelope(seasonal_phase)
    // Example: northern hemisphere → winter is cold/stormy, summer is warm/calm
    
    // Layer 2: Regional pattern (jet stream, monsoon proxies)
    for region_id in world.climate_regions:
        region = world.get_region(region_id)
        
        # Regional weather pattern has persistence/autocorrelation
        # It doesn't change every tick; it drifts slowly
        # This is modeled as an Ornstein-Uhlenbeck process
        
        region.weather_state += regional_weather_drift * dt
        region.weather_state = clamp(region.weather_state, -1.0, 1.0)
        
        # Constrain to seasonal envelope
        region.weather_state = clamp(region.weather_state,
                                     global_weather_envelope.min,
                                     global_weather_envelope.max)
    
    // Layer 3: Local stochastic perturbations
    for cell in world.biome_grid.all_cells():
        cell_region = world.region_containing_cell(cell)
        
        # Local weather is the regional weather + stochastic noise
        local_noise = random_normal(0, 0.1) * dt
        cell.weather_state = cell_region.weather_state + local_noise
        
        # Convert continuous weather_state to player-facing categories
        cell.current_weather = weather_state_to_category(cell.weather_state)
        # Returns: Clear, Cloudy, Rainy, Stormy, Fog, etc.
    
    // Wind field (persistent with autocorrelation)
    for cell in world.biome_grid.all_cells():
        # Wind direction persists and changes slowly
        cell.wind_direction += wind_drift_rate * dt
        cell.wind_direction = normalize_angle(cell.wind_direction)
        
        # Wind speed correlates with weather severity
        cell.wind_speed = base_wind_speed + weather_intensity(cell) * wind_weather_coupling
```

**Design consequence**: Weather is not a random encounter table roll. It's a continuous, persistent system where storms develop over time and move through regions. A player who sails into a storm front sees weather conditions worsen gradually, and can make course corrections to avoid the worst of it.

---

## 5. Cross-System Hooks

**To System 01 (Evolutionary Model)**:
- Creature presence in cells comes from PopulationDynamics
- Player's exploration activity (time spent in cell) feeds back to creature evolution as selection pressure
- Species observed feed the player's species browser

**To System 03 (Faction Social Model)**:
- Settlement placement and population come from faction territory control
- NPC presence on routes is faction-driven
- Exploration knowledge is traded between NPCs and factions

**To System 04 (Economy)**:
- Settlement resource availability comes from economy simulation
- Traveling between settlements enables trade
- Settlement demand drives creature hunting routes

**To System 05 (Crafting & Materials)**:
- Ruin era determines salvage material signatures
- Resource deposits yield materials tied to geological eras
- Harvesting success depends on equipment and technique

**To System 06 (Combat)**:
- Combat triggers from exploration encounters
- Creature loot feeds back to equipment crafting
- Combat outcomes affect NPC knowledge (deeds diffuse)

**To System 09 (Lore & World History)**:
- Ruin era and condition tell the world history
- POI placement reflects historical settlement patterns
- Old-world infrastructure decay reflects time-since-cataclysm

---

## 6. Tradeoff Matrix

| Tradeoff | Complexity | Player Agency | Simulation Purity | Adoption |
|----------|-----------|---------------|-------------------|----------|
| **Continuous terrain instead of hex grid** | +1 | +2 (organic navigation) | +3 | High |
| **POI discovery as state machine** | +1 | +1 (exploration has cost) | +3 | High |
| **Knowledge freshness decay** | +1 | +1 (map updates required) | +3 | Medium |
| **Navigation uncertainty (Brownian drift)** | +1 | +1 (fixes add gameplay) | +3 | High |
| **Encounters from creature presence** | +1 | +1 (density tells story) | +3 | High |
| **Routes with hazard aggregation** | +1 | +2 (emergent danger) | +2 | Medium |
| **Salvage from world history** | +2 | +1 (world-building) | +3 | Medium |
| **Hierarchical weather system** | +2 | +1 (weather is real) | +3 | Low |

---

## 7. Emergent Properties

- **Sparse-seeming world reveals pattern through repetition**: A region with low creature density will have long stretches without encounters, making the rare encounter feel consequential. A diverse ruin site with multiple creature niches will generate varied encounters.
- **Navigation knowledge becomes tradeable wealth**: The player who has sailed all routes has valuable chart knowledge that NPCs will pay for or want to steal.
- **Creature behavior shapes travel risk**: A cell with aggressive, high-neural_speed creatures is actively dangerous (territorial behavior). A cell with solitary, low-intelligence creatures is passively dangerous (unpredictable, environmental hazards).
- **Old-world ruins tell history through salvage**: The material signatures in ruins reflect their pre-flood construction era, creating a tangible connection to world history.
- **Exploration loops reinforce each other**: Discovering a creature type leads to hunting it → getting materials → crafting equipment → hunting more dangerous creatures → exploring more dangerous regions → discovering new creature types.

---

## 8. Open Calibration Knobs

```yaml
POI Discovery:
  exploration_threshold_base: 10            # min ticks to fully explore a POI
  complexity_exploration_factor: 5.0        # ticks per unit complexity
  hazard_exploration_factor: 3.0            # additional ticks per hazard
  
Knowledge Freshness:
  base_decay_rate: 0.02                     # per tick (freshness half-life ~50 ticks)
  activity_decay_multiplier: 2.0            # in active regions
  uncertainty_display_threshold: 0.4        # show "[may be outdated]" below this
  
Navigation Uncertainty:
  uncertainty_growth_rate: 0.05             # per sqrt(tick)
  compass_prevents_heading_drift: true
  sextant_fixes_position: true
  
Weather:
  seasonal_envelope_strength: 0.7           # how much season constrains weather
  regional_autocorrelation_timescale: 50    # ticks (weather drift speed)
  local_noise_std_dev: 0.1                  # per tick
  wind_weather_coupling: 2.0                # wind speed increases with storm severity
  
Encounters:
  base_encounter_rate_per_individual: 0.001 # per tick per creature
  visibility_detection_floor: 0.2           # min 20% detection even hidden
  weather_encounter_modifier_storm: 1.5     # storms increase encounters
  weather_encounter_modifier_fog: 0.5       # fog decreases encounters
```

---

## Appendix: Anti-Fudges Applied

1. **POI discovery is a state machine with explicit cost**: Discovering a POI requires sighting it (Detected), visiting it (Visited), and exploring it for a computed duration (Explored). This is not instant and not random.

2. **Encounters emerge from actual creature presence, not abstract dice rolls**: The "overloaded encounter die" is eliminated. Encounters happen when the player is near creatures that are actually present in the cell according to population dynamics simulation.

3. **Navigation uncertainty is Brownian drift, not random catastrophe**: Without instruments, the player's position drifts with sqrt(time) autocorrelation. Uncertainty is *fixable* through landmarks and instruments, creating a meaningful system rather than arbitrary failure.

