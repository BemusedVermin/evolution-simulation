# World History & Lore: The Drowned Earth

## 1. Overview

The Drowned Earth is a dying post-human world where player exploration uncovers layered ruins from nine historical ages. **But the world is not static.** The Chronicler System records significant simulation events (faction founding, extinctions, technological breakthroughs, plagues) and clusters them into emergent eras. In-world NPCs debate interpretations of these events, recount them with bias, and transmit them with decay — creating the lore the player encounters.

**Core design principle**: Nine seed ages (written pre-history) provide the world's foundation and tone. Chronicler outputs become the post-simulation lore. Narrators distort what they've recorded based on faction bias and time elapsed. Player encounters multiple contradictory accounts of the same event. The *truth* is always just out of reach — but plausible patterns emerge.

The lore system enforces that *simulation drives narrative*. No designer decides what "really happened" in the simulation; the Chronicler does. Contradictions are not bugs — they're features that embody how knowledge decays and refocuses across generations.

---

## 2. Research Basis

**Unreliable Narrator (Wolfe, Book of the New Sun; McCarthy, Blood Meridian)**: The most compelling fictional worlds are viewed through characters who misunderstand them. Narrators conflate, forget, reinterpret, and advocate. This doc implements that as a formal system.

**Telephone Game / Transmission Decay**: Classic folklore studies (Bartlett, 1932) showed that stories mutate systematically — details fade, explanations fill gaps, new details get added to make sense of remnants. We implement this as a confidence vector that degrades over simulation ticks.

**Emergent Periodization (Historical Semantics)**: Historians don't *find* eras — they *define* them in hindsight by looking for major axes of variance. The Chronicler does this: sliding-window analysis of recorded events, detect when dominant faction types shift (theocracy → merchant republic) or technology levels jump, declare a new era.

**Dwarf Fortress Legends Mode**: The gold standard for simulation-driven lore. Events are recorded procedurally; NPCs remember and misremember them; player can read raw history or distorted legends. We adopt that model.

**Cacogen as Irreducible Mystery (Wolfe, Barker, Leiber)**: Aliens are most interesting when incomprehensible. Caches of contradictory evidence about them (were they merchants? harvesters? colonizers?) should never fully resolve. The Watchers trope (a presence continuously observing Earth across multiple eras) is borrowed from Wolfe's Autarch.

---

## 3. Entities & State

### 3.1 Seed History: The Nine Ages

Pre-written, foundational lore. These exist before simulation starts and provide the world's tone and technology baseline.

```
SeedAge {
    age_number: int                      // 1-9, descending (9 = oldest)
    name: string                         // "Age of the Bright Horizon", "The Binding"
    approximate_duration: uint64         // ticks (if this age were to repeat now)
    defining_characteristics: string     // prose description
    
    // Gameplay hooks
    ruin_era: RuinEra                    // maps to procgen ruin templates
    technology_level: int                // [0-5] for NPC tech assumptions
    dominant_faction_types: list<string> // "theocracy", "merchant_republic", "tribe"
    key_locations: list<Location>        // major sites associated with age
    seed_lore_fragments: list<LoreFragment>
}

RuinEra = enum {
    // Pre-Harvest (Age 9)
    BrightHorizon,          // high-tech human
    BrightHorizon_Cacogen,  // alien-built
    
    // Post-Harvest (Ages 8-1)
    Receding,               // Age 8: desperate hybrid
    CradleKingdom,          // Age 7: early medieval
    SailAscendancy,         // Age 6: advanced medieval
    LongSilence,            // Age 5: sparse
    Synthesis,              // Age 4: quasi-industrial
    Binding,                // Age 3: religious monumental
    Unraveling,             // Age 2: militarized
    PresentTide,            // Age 1: contemporary
    
    Palimpsest,             // multi-era, common in settlements
}
```

### 3.2 Chronicler: Event Recording System

Detects significant simulation events and records them.

```
Chronicler {
    event_log: list<ChronicleEntry>
    last_consolidation_tick: uint64
    current_era: SimulationEra or null
}

ChronicleEntry {
    id: unique_id
    event_type: EventType
    timestamp: uint64                    // absolute simulation tick
    location: WorldLocation
    agents_involved: list<EntityID>      // factions, creatures, landmarks
    description: string                  // procedurally generated
    
    // Confidence vector: how much do narrators trust this account?
    confidence: Confidence {
        epistemology: float              // [0,1] "how directly observed?"
        persistence: float               // [0,1] "how many sources mention it?"
        resolution: float                // [0,1] "how specific vs. vague?"
    }
    
    // Narrative distortion: tracks how this event gets told
    narrative_variants: list<NarrativeVariant>
    salience: float                      // [0,1] population_affected × novelty × persistence
}

EventType = enum {
    // Ecological
    SpeciesExtinction,
    SpeciesEmergence,
    MasseMigration,
    EcosystemShift,
    
    // Social
    FactionFounding,
    FactionCollapse,
    FactionMerger,
    ConflictEruption,
    TreatyForged,
    
    // Technological
    TechnologyAdopted,
    TechnologyLost,
    ArtifactDiscovered,
    ArtifactDesecrated,
    
    // Biological/Epidemic
    PlagueOutbreak,
    PlagueCured,
    
    // Environmental
    DisasterStrike,      // volcanic, tectonic, weather
    GeographicShift,     // the Drifting, sea level change
    
    // Cacogenic
    CacogenSighting,
    CacogenInteraction,
    CacogenArtifactActivated,
    
    // Cultural
    ReligionEmerges,
    ReligionSplits,
    TraditionLost,
    NamedIndividualDeath,
    NamedIndividualAccomplishment,
}

NarrativeVariant {
    narrator_faction_id: EntityID or null  // who told this version?
    content: string                         // alternative wording
    conviction: float                       // [0,1] how sure is this narrator?
    divergence_reasons: list<string>       // why does it differ? (bias, decay, etc.)
}

Confidence {
    // Each component ranges [0,1]
    epistemology: float
    // 1.0 = witnessed firsthand
    // 0.8 = from reliable actor
    // 0.5 = hearsay
    // 0.2 = fragmentary evidence
    // 0.0 = pure speculation
    
    persistence: float
    // 1.0 = mentioned by 5+ independent sources
    // 0.8 = mentioned by 2-4 sources
    // 0.5 = mentioned by 1 source
    // 0.0 = no corroboration
    
    resolution: float
    // 1.0 = specific details, causation clear
    // 0.8 = specific but causation fuzzy
    // 0.5 = vague outline
    // 0.0 = almost purely inferential
}
```

### 3.3 Era Detection: Emergent Periodization

The Chronicler analyzes its event log and clusters events into eras.

```
SimulationEra {
    era_number: int                      // 0, 1, 2, ... (post-seed eras)
    name: string                         // assigned by a narrator or historian faction
    start_tick: uint64
    end_tick: uint64 or null             // null = ongoing
    dominant_axis_of_change: string      // "faction_type_shift", "technology_surge", "climate_epoch"
    
    // Statistical summary
    event_count: int
    salience_sum: float                  // total importance of era
    key_events: list<ChronicleEntry>    // top 5-10 by salience
    
    // Detected archetypes
    faction_types_present: list<string>
    technology_level_trend: float        // increasing, stable, or decreasing
    ecosystem_state: string              // "stable", "disrupted", "recovering"
}

function detect_new_era(chronicler: Chronicler) -> bool:
    // Sliding window analysis: last N ticks
    window_size = 10000  // ticks (~1 in-game year at default speed)
    recent_events = chronicler.event_log.filter(e => e.timestamp > current_tick - window_size)
    
    if len(recent_events) < 5:
        return false  // not enough events to define an era
    
    // Compute key metrics
    dominant_faction_type = most_common_faction_type(recent_events)
    avg_technology_level = average_technology_mentioned(recent_events)
    ecosystem_volatility = std_dev_of_ecosystem_events(recent_events)
    
    // Compare to current era (if one exists)
    if chronicler.current_era == null:
        // First era
        chronicler.current_era = new SimulationEra(
            era_number: 0,
            start_tick: recent_events.min_timestamp,
            dominant_axis_of_change: infer_axis(recent_events)
        )
        return true
    
    // Check if we've crossed a threshold
    faction_shift = abs(dominant_faction_type - chronicler.current_era.dominant_faction)
    tech_shift = abs(avg_technology_level - chronicler.current_era.avg_tech_level)
    ecosystem_shift = ecosystem_volatility > chronicler.current_era.avg_volatility * 1.5
    
    if faction_shift > 0.3 or tech_shift > 1.0 or ecosystem_shift:
        // Significant change detected — declare a new era
        chronicler.current_era.end_tick = current_tick - window_size / 2
        chronicler.current_era = new SimulationEra(
            era_number: chronicler.current_era.era_number + 1,
            start_tick: current_tick - window_size / 2,
            dominant_axis_of_change: infer_axis(recent_events)
        )
        return true
    
    return false
```

### 3.4 Narrator: Faction-Biased Lore Transmitter

```
Narrator {
    narrator_id: EntityID               // linked to a faction or individual
    knowledge_store: set<ChronicleEntry> // which events does this narrator know about?
    personality: NarratorPersonality
    
    faction_opinion_bias: OpinionVector // affects how events are interpreted
    temporal_decay_rate: float          // how fast does memory fade? [0,1]
    
    transmitted_accounts: list<NarrativeVariant>  // lore the narrator has told
}

NarratorPersonality = enum {
    Scholar,         // seeks accuracy, cites sources
    Mystic,          // allegorical, spiritual interpretation
    Merchant,        // pragmatic, focuses on trade/resources
    Warrior,         // emphasizes conflict, strength
    Keeper,          // preserves heritage, resists change
}

function narrator_transmit(
    narrator: Narrator,
    chronicle_entry: ChronicleEntry
) -> NarrativeVariant:
    
    // 1. Apply temporal decay
    age_in_ticks = current_tick - chronicle_entry.timestamp
    decay_factor = pow(narrator.temporal_decay_rate, age_in_ticks / 1000.0)
    
    adjusted_confidence = chronicle_entry.confidence * decay_factor
    
    // 2. Apply bias based on personality and faction opinions
    bias = compute_bias(narrator, chronicle_entry)
    
    // 3. Apply noise (simulating memory errors and reinterpretation)
    noise_amount = 1.0 - adjusted_confidence.resolution
    noise_injected = apply_narrative_noise(chronicle_entry.description, noise_amount)
    
    // 4. Reinterpret based on personality
    narrated_description = personalize_narrative(
        noise_injected,
        narrator.personality,
        chronicle_entry.event_type
    )
    
    // 5. Build variant
    variant = NarrativeVariant(
        narrator_faction_id: narrator.narrator_id,
        content: narrated_description,
        conviction: adjusted_confidence.epistemology * (1.0 - bias),
        divergence_reasons: [
            if decay_factor < 0.5: "time has faded details",
            if bias > 0.3: "narrator's faction has interest in reframing",
            if noise_amount > 0.3: "contradictory source accounts"
        ]
    )
    
    chronicle_entry.narrative_variants.append(variant)
    return variant

function compute_bias(narrator: Narrator, entry: ChronicleEntry) -> float:
    // Narrator skews interpretation based on their faction's opinions
    
    agent_faction = faction_of_agent(entry.agents_involved[0])
    opinion_delta = magnitude(narrator.faction_opinion_bias - agent_faction.opinion_vector)
    
    // Stronger bias = narrator reinterprets event to fit their worldview
    return clamp(opinion_delta * 0.5, 0.0, 1.0)

function apply_narrative_noise(text: string, noise_level: float) -> string:
    // Simulates transmission errors: drop details, add invented details, shift causation
    
    // Drop specific details (high noise)
    if noise_level > 0.5:
        text = text.remove_random_clauses(count: floor(noise_level * 3))
    
    // Generalize specific names → categories
    if noise_level > 0.3:
        text = text.replace_entities_with_categories()  // "Thraxor" → "a warlord"
    
    // Invert or obscure causation
    if noise_level > 0.7:
        text = text.permute_causal_clauses()
    
    return text
```

### 3.5 Lore Fragment System

Atomic units of lore delivered to the player.

```
LoreFragment {
    id: unique_id
    content_key: string                  // lookup key for localized text
    era: RuinEra or null                 // which age does this illuminate?
    cross_refs: list<RuinEra>           // other ages mentioned
    category: LoreCategory
    reliability: float                   // [0,1] author-assigned base reliability
    source_chronicle_entry: ChronicleEntry or null
    
    // Narrative variants from different sources
    variants: list<NarrativeVariant>
    
    // Gatefold: player must know certain things to unlock this
    knowledge_gate: list<ChronicleEntry> or null
    rarity: enum { Common, Uncommon, Rare, Unique }
    
    // Contradicts/complements other fragments
    contradicts: list<LoreFragment>
    complements: list<LoreFragment>
}

LoreCategory = enum {
    HistoricalNarrative,    // "what happened"
    TechnicalKnowledge,     // "how things work"
    CacogenRecord,          // alien-related
    CulturalPractice,       // customs, beliefs
    GeographicRecord,       // maps, routes, terrain
    BiologicalRecord,       // species, ecology
    PersonalAccount,        // individual stories
    ReligiousText,          // doctrine, prophecy
    PoliticalRecord,        // governance, law
    KeeperArchive,          // Keeper-lineage records
}
```

### 3.6 Keeper Tradition & Faction Seed

The Keeper tradition is a heritable cultural archetype, seeded into the faction social system.

```
KeepersArchetype {
    // Applied to factions during crystallization
    
    tradition_name: string               // "Orthodox Keepers", "Scholar Keepers", etc.
    opinion_anchor_point: OpinionVector
    
    // Resists drift on certain dimensions
    value_drift_resistance: {
        tradition_vs_innovation: float   // high = keeps values stable
        hierarchy_vs_egalitarian: float
        isolation_vs_expansion: float
    }
    
    // Starting practices
    starting_artifact_collection: list<ArtifactType>
    knowledge_preservation_rate: float   // how well do they transmit lore?
}

// Seed Keeper archetypes (inserted into initial faction population)
SEED_KEEPERS = [
    KeepersArchetype(
        tradition_name: "Orthodox Keepers",
        opinion_anchor_point: { tradition: 0.9, hierarchy: 0.8 },
        value_drift_resistance: { tradition_vs_innovation: 0.8 }
    ),
    KeepersArchetype(
        tradition_name: "Scholar Keepers",
        opinion_anchor_point: { tradition: 0.7, innovation: 0.8 },
        value_drift_resistance: { tradition_vs_innovation: 0.5 }
    ),
    // ... more archetypes
]
```

### 3.7 Emergent Labels: Primitive-Cluster Naming

With the Phenotype Interpreter (System 11) refactored to emit only PrimitiveEffect sets, the Chronicler must discover human-readable labels for stable, recurring patterns of primitive combinations. Labels are *emergent* — discovered by the Chronicler, not pre-authored — and serve the UI and lore layer, while game mechanics read primitives directly.

```
EmergentLabel {
    id: unique_id
    label_string: string                 // "echolocation", "venom_injection", "bioluminescence"
    pattern_signature: PatternSignature  // canonical form of primitive cluster
    
    // Metadata
    first_observed_tick: uint64
    discovery_tick: uint64               // when stability threshold was crossed
    discoverer_faction: EntityID or null // which faction has this in their lexicon?
    
    // Stability evidence
    observed_count: int                  // number of organisms exhibiting pattern
    lineages_exhibiting: set<LineageID>  // how many distinct evolutionary lineages?
    tick_persistence: uint64             // duration of observation (contiguous)
    
    // Naming provenance
    naming_source: enum {
        ThesaurusMatch,                  // matched hand-seeded biological thesaurus
        TemplateComposition,             // built from primitive categories
        FactionCoin                      // faction's language system coined term
    }
    thesaurus_match_confidence: float or null  // [0,1] if ThesaurusMatch
    
    // Persistence in lore
    is_extinct: bool                     // pattern population fell below floor
    extinction_tick: uint64 or null      // when did it go extinct?
    retained_in_lore: bool               // keep label in historical records?
}

PatternSignature {
    id: unique_id
    primitive_cluster: list<string>      // canonical set of primitive effects
    // e.g., ["emit_acoustic_pulse", "receive_acoustic_signal", "spatial_integrate"]
    
    // Parameter ranges (tolerance windows)
    parameter_bounds: map<string, Bound> // e.g., frequency: [20kHz, 100kHz]
    
    // Body-site configuration: which body regions must carry these primitives?
    body_site_requirements: list<string> // e.g., ["head", "sensory_organs"]
    
    // Conditions for recognition
    co_occurrence_threshold: float       // [0,1] must have at least this fraction
    // of primitives present to match signature
    
    // Canonical tolerance for matching
    tolerance: {
        parameter_variance: float        // [0,1] allowed deviation from bounds
        site_variance: float             // [0,1] allowed deviation from site config
    }
}

LabelThesaurus {
    // Hand-seeded biological patterns with known signatures
    entries: list<ThesaurusEntry>
}

ThesaurusEntry {
    biological_term: string              // "echolocation", "electroreception", "venom"
    pattern_signature_template: PatternSignature
    description: string                  // biological explanation
    examples_in_nature: list<string>     // real animals: "bats", "dolphins", "snakes"
    research_tags: list<string>          // for discovery ranking
}
```

### 3.8 Data-Driven Label Manifests

**Critical Rule**: All label definitions are loaded from JSON manifests at startup. Hardcoded heuristics for label naming are **forbidden** in simulation code. The Chronicler reads manifest entries and uses them to match emerging primitive patterns against canonical signatures.

**LabelManifest JSON Format** (stored in `assets/manifests/labels/`):

```json
[
  {
    "label_id": "echolocation",
    "canonical_label": "echolocation",
    "primitive_signature": [
      "emit_acoustic_pulse",
      "receive_acoustic_signal",
      "spatial_integrate"
    ],
    "confidence_threshold": 0.7,
    "parameter_constraints": {
      "emit_acoustic_pulse": {
        "frequency": {"min": 20000, "max": 200000}
      },
      "receive_acoustic_signal": {
        "sensitivity": {"min": 0.6, "max": 1.0}
      }
    },
    "body_site_requirements": ["head", "sensory_organs"],
    "etymology_hint": "biological: bats, dolphins use sound echolocation",
    "research_tags": ["sensory_system", "acoustic_communication"],
    "discovery_rank": 2
  },
  {
    "label_id": "venom_injection",
    "canonical_label": "venom injection",
    "primitive_signature": [
      "inject_substance",
      "state_induction"
    ],
    "confidence_threshold": 0.6,
    "parameter_constraints": {
      "inject_substance": {
        "substance_type": ["toxin", "protein_complex"],
        "volume": {"min": 0.001, "max": 1.0}
      },
      "state_induction": {
        "duration": {"min": 10, "max": 1000}
      }
    },
    "body_site_requirements": ["fangs", "stingers", "claws"],
    "etymology_hint": "biological: snakes, spiders, wasps use venom",
    "research_tags": ["chemical_defense", "predation"],
    "discovery_rank": 1
  },
  {
    "label_id": "bioluminescence",
    "canonical_label": "bioluminescence",
    "primitive_signature": [
      "emit_light"
    ],
    "confidence_threshold": 0.5,
    "parameter_constraints": {
      "emit_light": {
        "wavelength": {"min": 400, "max": 700},
        "intensity": {"min": 0.3, "max": 1.0}
      }
    },
    "body_site_requirements": null,
    "etymology_hint": "biological: fireflies, jellyfish, anglerfish emit light",
    "research_tags": ["visual_signaling", "camouflage"],
    "discovery_rank": 3
  }
]
```

**Loading & Validation at Startup**:

```rust
pub struct LabelManifest {
    pub entries: Vec<LabelManifestEntry>,
}

pub struct LabelManifestEntry {
    pub label_id: String,
    pub canonical_label: String,
    pub primitive_signature: Vec<String>,
    pub confidence_threshold: f32,
    pub parameter_constraints: Option<HashMap<String, ParameterConstraint>>,
    pub body_site_requirements: Option<Vec<String>>,
    pub etymology_hint: Option<String>,
    pub research_tags: Option<Vec<String>>,
    pub discovery_rank: u32,
}

impl LabelManifest {
    /// Load label manifest from JSON at startup; validate against schema
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let json_str = std::fs::read_to_string(path)?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)?;
        
        // Validate against JSON Schema draft 2020-12
        let schema_str = include_str!("../schemas/label_manifest.schema.json");
        let schema_value: serde_json::Value = serde_json::from_str(schema_str)?;
        let schema = jsonschema::JSONSchema::compile(&schema_value)?;
        schema.validate(&json_value).map_err(|e| {
            anyhow!("Label manifest validation failed: {}", e)
        })?;
        
        // Deserialize
        let entries: Vec<LabelManifestEntry> = serde_json::from_value(json_value)?;
        
        Ok(LabelManifest { entries })
    }
    
    /// Find label matching a primitive pattern signature
    pub fn find_label_for_signature(
        &self,
        primitive_signature: &[String],
        parameters: &HashMap<String, f32>,
    ) -> Option<LabelManifestEntry> {
        // Match against manifest entries in discovery_rank order (higher rank = prefer)
        let mut matching_entries: Vec<_> = self.entries
            .iter()
            .filter(|entry| {
                // Check if signature matches
                let sig_match = Self::signature_matches(
                    &entry.primitive_signature,
                    primitive_signature,
                    entry.confidence_threshold,
                );
                
                // Check parameter constraints
                let param_match = entry.parameter_constraints.as_ref()
                    .map_or(true, |constraints| {
                        Self::parameters_match(constraints, parameters)
                    });
                
                sig_match && param_match
            })
            .collect();
        
        // Sort by discovery_rank (descending); return highest
        matching_entries.sort_by_key(|e| -(e.discovery_rank as i32));
        matching_entries.first().map(|e| (*e).clone())
    }
    
    fn signature_matches(
        manifest_sig: &[String],
        actual_sig: &[String],
        confidence_threshold: f32,
    ) -> bool {
        // Compute Jaccard similarity or cosine similarity of signatures
        let intersection = manifest_sig.iter()
            .filter(|p| actual_sig.contains(p))
            .count();
        let union = manifest_sig.iter().chain(actual_sig.iter())
            .collect::<std::collections::HashSet<_>>()
            .len();
        
        let similarity = intersection as f32 / union as f32;
        similarity >= confidence_threshold
    }
    
    fn parameters_match(
        constraints: &HashMap<String, ParameterConstraint>,
        parameters: &HashMap<String, f32>,
    ) -> bool {
        for (param_name, constraint) in constraints.iter() {
            if let Some(value) = parameters.get(param_name) {
                if !constraint.validate(*value) {
                    return false;
                }
            }
        }
        true
    }
}
```

**Faction-Specific Label Aliases** (loaded separately):

Factions can coin their own labels via the Chronicler's faction-coined pipeline. Faction-specific aliases are stored in the Chronicler state, not in the global manifest:

```rust
pub struct FactionCoinedLabel {
    pub faction_id: EntityID,
    pub coined_label: String,
    pub maps_to_canonical: String,  // e.g., "venom_injection"
    pub coined_at_tick: u64,
    pub usage_count: u32,
}

// Chronicler maintains faction-coined labels
pub struct Chronicler {
    pub label_manifest: LabelManifest,
    pub faction_coined_labels: Vec<FactionCoinedLabel>,
    // ... other fields
}
```

**Critical Invariant**: No simulation code (interpreter, combat system, ecology system) reads label strings. Labels are UI-only. Game mechanics read primitive effects directly and are unaffected by label definitions.

---

## 4. Update Rules

### 4.1 Event Detection & Recording

The Chronicler monitors simulation and records significant events.

```
function chronicler_update(world: World) -> void:
    // Every tick, scan for noteworthy events
    
    new_extinctions = world.ecology.extinct_species_this_tick
    for species_id in new_extinctions:
        record_event(
            event_type: SpeciesExtinction,
            agents_involved: [species_id],
            location: species_last_sighting_location,
            salience: population_affected * 0.5  // moderate importance
        )
    
    new_factions = world.factions.founded_this_tick
    for faction_id in new_factions:
        record_event(
            event_type: FactionFounding,
            agents_involved: [faction_id],
            location: faction_capital,
            salience: 0.7  // notable
        )
    
    new_conflicts = world.social.conflicts_begun_this_tick
    for conflict in new_conflicts:
        record_event(
            event_type: ConflictEruption,
            agents_involved: conflict.parties,
            location: conflict.location,
            salience: len(conflict.parties) * 0.3
        )
    
    // ... scan for tech adoption, plague, artifacts discovered, etc.
    
    // Consolidate and detect eras every N ticks
    if current_tick % 5000 == 0:
        if detect_new_era(chronicler):
            name_new_era()
            notify_narrators()

function record_event(
    event_type: EventType,
    agents_involved: list<EntityID>,
    location: WorldLocation,
    salience: float
) -> void:
    
    entry = ChronicleEntry(
        event_type: event_type,
        timestamp: current_tick,
        location: location,
        agents_involved: agents_involved,
        salience: salience,
        confidence: estimate_initial_confidence(event_type, agents_involved),
        description: generate_event_description(event_type, agents_involved, location)
    )
    
    chronicler.event_log.append(entry)
    
    // Notify narrators within knowledge range
    for narrator in world.narrators:
        if distance(narrator.location, location) < narrator.knowledge_range:
            narrator.knowledge_store.add(entry)

function estimate_initial_confidence(event_type: EventType, agents: list<EntityID>) -> Confidence:
    // Initial confidence based on how clear and direct the event is
    
    epistemology = 1.0  // directly observed by system
    persistence = 0.2   // only recorded once, needs corroboration
    resolution = match event_type:
        SpeciesExtinction: 0.9,  // clear cutoff point
        ConflictEruption: 0.7,   // multiple causes, unclear origins
        CacogenSighting: 0.4,    // inherently mysterious
        // ... etc.
    
    return Confidence { epistemology, persistence, resolution }
```

### 4.2 Narrator Knowledge Decay

Over time, narrators' confidence in their knowledge fades.

```
function narrator_memory_decay() -> void:
    for narrator in world.narrators:
        for chronicle_entry in narrator.knowledge_store:
            age_ticks = current_tick - chronicle_entry.timestamp
            decay = pow(narrator.temporal_decay_rate, age_ticks / 1000.0)
            
            chronicle_entry.confidence.epistemology *= decay
            chronicle_entry.confidence.resolution *= decay
            
            // If confidence drops below threshold, forget entirely
            if confidence_total(chronicle_entry.confidence) < 0.1:
                narrator.knowledge_store.remove(chronicle_entry)
```

### 4.3 Era Naming

When a new simulation era is detected, NPCs assign it a name.

```
function name_simulation_era(era: SimulationEra) -> void:
    // Ask historian factions to name it
    historians = world.factions.filter(f => f.primary_role == Historian)
    
    if len(historians) == 0:
        // Auto-generate name
        era.name = generate_default_era_name(era.dominant_axis_of_change)
        return
    
    // Each historian proposes a name based on their values
    proposals = []
    for historian in historians:
        proposal = generate_era_name_proposal(historian, era)
        proposals.append(proposal)
    
    // Consensus: the name that most historians propose wins
    // (Ties broken randomly or by faction size)
    era.name = proposals.max_by(count).name
```

---

## 5. Cross-System Hooks

### 5.1 Knowledge Storage Integration

The lore system integrates with the player's KnowledgeStore (faction system F2).

```
function unlock_lore_fragments(player_knowledge: KnowledgeStore) -> list<LoreFragment>:
    // Player can access lore fragments if they've learned prerequisites
    
    accessible_fragments = []
    
    for fragment in world.all_lore_fragments:
        if fragment.knowledge_gate == null:
            // No gating
            accessible_fragments.append(fragment)
        else:
            // Check if player knows all gating entries
            if all(player_knowledge.contains(entry) for entry in fragment.knowledge_gate):
                accessible_fragments.append(fragment)
    
    return accessible_fragments
```

### 5.2 NPC Dialogue Integration

NPCs reference lore and recount events based on what they know.

```
function generate_npc_dialogue_topic(npc: NPC, topic: DialogueTopic) -> string:
    // NPC draws from their knowledge store to answer questions about lore
    
    relevant_fragments = npc.narrator.knowledge_store
        .filter(e => topic_matches(e, topic))
    
    if len(relevant_fragments) == 0:
        return "I know nothing of this."
    
    // Pick the most salient and confident fragment
    chosen = relevant_fragments.max_by(e => e.salience * confidence_total(e.confidence))
    
    // Generate variant based on narrator personality
    variant = narrator_transmit(npc.narrator, chosen)
    
    return variant.content
```

### 5.3 Environmental Storytelling

Ruins and artifacts encode lore via their era assignment.

```
function ruin_dialogue_tooltip(ruin: Ruin) -> string:
    // When player examines a ruin, show lore about its era
    
    era_fragments = world.all_lore_fragments.filter(f => f.era == ruin.era)
    
    reliable = era_fragments.filter(f => f.reliability > 0.6)
    unreliable = era_fragments.filter(f => f.reliability < 0.4)
    
    text = ""
    if len(reliable) > 0:
        text += "This appears to be from " + ruin.era.name + ": " + reliable[0].content
    
    if len(unreliable) > 0:
        text += "\n(Some say: " + unreliable[0].content + ")"
    
    return text
```

### 5.4 Keeper Tradition Propagation

Keeper-lineage factions maintain low value drift on specific dimensions.

```
function faction_opinion_drift(faction: Faction, dt: float) -> void:
    // Normal factions' opinions drift toward neighbors in opinion space
    
    drift = compute_opinion_drift(faction, dt)
    
    // If faction has Keeper heritage, resist drift on certain dimensions
    if faction.has_keeper_tradition:
        keeper_archetype = faction.keeper_archetype
        
        for dimension in drift.dimensions:
            if dimension in keeper_archetype.value_drift_resistance:
                resistance = keeper_archetype.value_drift_resistance[dimension]
                drift[dimension] *= (1.0 - resistance)  // dampen drift
    
    faction.opinions += drift
```

### 5.5 Cacogen Indigenization

Evolved creatures can carry "alien lineage tags" from the initial gene pool.

```
function monster_inheritance_from_cacogen_ancestry(
    ancestor: Monster,
    offspring: Monster
) -> void:
    
    // If ancestor has alien_lineage_tag, offspring may inherit it
    if ancestor.genotype.alien_lineage_tag != null:
        inheritance_chance = 0.7  // high heritability
        
        if random() < inheritance_chance:
            offspring.genotype.alien_lineage_tag = ancestor.genotype.alien_lineage_tag
            
            // Alien tags provide slow-evolving regulatory traits
            // (e.g., unusual metabolic pathways, sensory modes)
            offspring.genotype.regulatory_channels[ALIEN_METABOLISM] = 0.1
            offspring.genotype.regulatory_channels[EXOTIC_SENSING] = 0.05
```

---

## 6. How the Chronicler Serves Downstream Systems

The Chronicler is a *data layer*, not a game mechanic. It produces records that other systems consume. Understanding the boundary between Chronicler output and downstream consumption prevents design confusion.

### 6.1 Combat & AI (Never Read Labels)

**Constraint**: Combat systems and creature AI read only PrimitiveEffects. They never read EmergentLabels.

Why? Labels are compressed, post-hoc summaries. Game mechanics require the full fidelity of atomic primitives. A creature doesn't "use echolocation" — it emits acoustic pulses, receives signals, integrates space. The combat system computes range-of-effect, accuracy, stamina cost, and feedback based on these primitives directly.

EmergentLabels are UI abstractions. If we allowed combat to read labels, we'd create two sources of truth: the primitives (which change dynamically) and the label (which is stable but coarse). This leads to sync bugs.

```
function creature_turn_in_combat(creature: Creature) -> Action:
    // Combat AI reads primitives
    available_actions = []
    
    for primitive in creature.phenotype.active_primitives:
        // Compute action options from primitive effects directly
        action_option = compute_action_from_primitive(primitive, creature, combat_state)
        if action_option != null:
            available_actions.append(action_option)
    
    // Choose action with highest expected value
    chosen_action = available_actions.max_by(expected_value)
    return chosen_action

function compute_action_from_primitive(
    primitive: PrimitiveEffect,
    creature: Creature,
    combat_state: CombatState
) -> Action:
    match primitive.effect_type:
        EmitAcousticPulse:
            // Compute acoustic pressure, frequency, cone
            action = Action(
                type: Echolocation,  // action is labeled for UI
                range: compute_range(primitive.power),
                accuracy: compute_accuracy(primitive.frequency, creature.sensory_sharpness),
                cooldown: compute_cooldown(primitive),
                stamina_cost: compute_stamina(primitive.power)
            )
            return action
        ApplyBiteForce:
            action = Action(
                type: Bite,
                damage: compute_damage(primitive.force),
                range: 1,
                cooldown: 0,
                stamina_cost: compute_stamina(primitive.force)
            )
            return action
        // ... etc.
```

Note: The Action is *labeled* "Echolocation" for UI display. This label comes from the combat system's action taxonomy, not from the Chronicler. This is intentional separation: the Chronicler labels *biological patterns over evolutionary time*; the combat system labels *actions available this turn*.

### 6.2 NPC Dialogue & Lore References (Read Labels)

**Permission**: NPCs and narrators can cite EmergentLabels in dialogue and lore.

NPCs reference labels when describing creatures, abilities, factions, and history. An NPC might say: *"Aye, the echo-beasts nearly drove us from the deep caves — their sound-sense is uncanny."* This is a reference to the echolocation label.

```
function generate_npc_dialogue_about_creature(
    npc: NPC,
    creature_species: Species
) -> string:
    // Draw on labels the NPC's faction has observed
    
    known_labels = npc.faction.observed_emergent_labels.filter(
        label => label_observed_in_species(label, creature_species)
    )
    
    if len(known_labels) == 0:
        return "I know not what manner of beast that is."
    
    // Choose a label at random (or by salience/familiarity)
    label = known_labels[random() % len(known_labels)]
    
    // Use faction's coined term if available, else default label
    label_name = npc.faction.language_system.get_term_for_label(label) or label.label_string
    
    // Insert into template
    return "Beware the " + label_name + " — they hunt with " + label.describe_mechanism() + "."
```

### 6.3 Bestiary & Creature Codex

**Permission**: The player-facing bestiary shows both labels and primitive breakdown.

A bestiary entry for a creature might look like:

```
[CREATURE: Deepcaller]
[SIZE: Large]
[Rarity: Uncommon]

[OBSERVED ABILITIES]:
  - Echolocation (Emergent Label)
  - Cooperative Hunting
  
[MECHANICAL DETAIL - Primitives]:
  - emit_acoustic_pulse (frequency: 40-60 kHz, power: 8-10W)
  - receive_acoustic_signal (directional accuracy: ±5 degrees)
  - spatial_integrate (map resolution: 0.5m at 10m range)
  - form_cooperative_bond (with up to 4 siblings)
  
[EVOLUTIONARY ORIGIN]:
  This label was first observed in Simulation Era 2, in 47 distinct organisms
  across the Deepmaw lineage and the Vocalic lineage, marking convergent
  evolution of acoustic sensing.
```

The labels make the entry readable to the player. The primitives explain the mechanics to a designer or advanced player. Both are true; they serve different audiences.

### 6.4 Per-Faction Calendars & Songs (Dialect & Reinterpretation)

**Permission**: Factions interpret and reuse labels within their own cultural products.

The Faction Language System (system 18) can generate per-faction variants of label names. A faction might coin a local term for "echolocation" that reflects their cosmology:

- **Orthodox Keepers**: "The Echoing Sense" (scientific)
- **Mystic Cultists**: "The Song of the Watcher-Below" (mystical)
- **Merchant Consortium**: "The Trade-Sense" (pragmatic, because they used echolocating creatures as navigators)

Per-faction calendars (system 14) might mark the year an important label was discovered locally, or incorporate lore about the first faction member to master a labeled ability. Labels become part of the cultural narrative layer.

```
function faction_calendar_entry_for_label(
    faction: Faction,
    label: EmergentLabel
) -> CalendarEntry or null:
    
    // Did this faction first observe this label?
    if faction.id not in label.observing_factions:
        return null
    
    observation_tick = faction.label_observation_log[label.id].first_tick
    era = world.simulation_eras_at_tick(observation_tick)
    
    entry = CalendarEntry(
        era: era,
        description: "Discovery of the " + faction.get_label_term(label) + \
                     " — a new form of sensory mastery.",
        salience: 0.3
    )
    
    return entry
```

### 6.5 Cross-Chronicler Index: Labels as Indexed Events

Labels are a first-class component of the Chronicler's event index, alongside ChronicleEntry. When a label is discovered or goes extinct, it is recorded as an event that appears in the world's history.

Example query:
```
labels_discovered_in_era = chronicler.emergent_labels.filter(
    l => l.discovery_tick >= era.start_tick and l.discovery_tick <= era.end_tick
)

// In narrative generation:
narrative = "In " + era.name + ", the world saw the rise of " + \
            human_readable_list(labels_discovered_in_era) + "."
```

---

## 7. Emergent Label Discovery: From Primitives to Named Abilities

**Core Principle**: The Phenotype Interpreter emits only PrimitiveEffect sets (atomic verbs: emit_acoustic_pulse, receive_acoustic_signal, spatial_integrate, apply_bite_force, inject_substance, form_host_attachment, etc.). The Chronicler watches these effects across the population and *discovers* human-readable labels for stable, recurring primitive patterns. These labels are NOT produced by the Interpreter — they emerge from the Chronicler's observation of patterns.

Named abilities like "echolocation," "venom injection," or "bioluminescence" are Chronicler output, not Phenotype output. Game mechanics (combat, AI) read only PrimitiveEffects; labels appear in UI, NPC dialogue, bestiary, and lore.

**Research Basis**: Gould & Vrba (1982) on exaptation — evolution produces structures whose current function differs from their original function or origin. Importantly, *names follow observation*. Darwin wrote of "organs of extreme perfection" precisely because nature does not pre-name its inventions; humans apply names after the fact, recognizing patterns. Similarly, the Chronicler does not hand-author "echolocation" for a bat-like creature; it observes the co-occurrence of acoustic emission and spatial-integration primitives and assigns a label retrospectively.

### 6.1 Data Flow: Primitive Snapshots to Pattern Recognition

Each tick (or at sampled intervals for performance), the Phenotype Interpreter produces per-organism PrimitiveEffect snapshots. These are streamed to the Chronicler.

```
function chronicler_ingest_primitive_effects(tick: uint64, organism: Organism) -> void:
    // Sample organisms at controlled rate to avoid processing overhead
    sample_interval = 100  // process 1% of organisms per tick at default density
    
    if random() < 1.0 / sample_interval:
        snapshot = PrimitiveEffectSnapshot(
            organism_id: organism.id,
            lineage_id: organism.lineage.id,
            timestamp: tick,
            active_primitives: organism.phenotype.active_primitives,
            body_site_map: organism.phenotype.body_site_map,
            parameter_state: organism.phenotype.parameter_state
        )
        
        // Queue snapshot for pattern analysis
        chronicler.primitive_snapshot_queue.append(snapshot)

function chronicler_analyze_primitive_patterns(window_size: uint64 = 500) -> void:
    // Process accumulated snapshots; compute pattern signatures
    // Called every ~N ticks (e.g., every 500 ticks)
    
    for signature in chronicler.known_pattern_signatures:
        organisms_matching = []
        lineages_matching = set()
        
        for snapshot in chronicler.primitive_snapshot_queue:
            if pattern_signature_matches(snapshot, signature):
                organisms_matching.append(snapshot.organism_id)
                lineages_matching.add(snapshot.lineage_id)
        
        signature.recent_match_count = len(organisms_matching)
        signature.recent_lineages = lineages_matching
        
        // Check for label stability
        check_label_stability(signature)
    
    // Try to discover NEW pattern signatures from unlabeled snapshots
    discover_novel_signatures(chronicler.primitive_snapshot_queue)
    
    chronicler.primitive_snapshot_queue.clear()

function pattern_signature_matches(
    snapshot: PrimitiveEffectSnapshot,
    signature: PatternSignature
) -> bool:
    // Check if snapshot's primitive cluster maps onto signature within tolerance
    
    snapshot_primitives = set(snapshot.active_primitives)
    canonical_primitives = set(signature.primitive_cluster)
    
    overlap = len(snapshot_primitives ∩ canonical_primitives)
    required = ceil(len(canonical_primitives) * signature.co_occurrence_threshold)
    
    if overlap < required:
        return false
    
    // Check parameter bounds
    for (param_name, bounds) in signature.parameter_bounds:
        if param_name not in snapshot.parameter_state:
            return false
        
        value = snapshot.parameter_state[param_name]
        allowed_variance = bounds.range * signature.tolerance.parameter_variance
        
        if abs(value - bounds.center) > allowed_variance:
            return false
    
    // Check body-site configuration
    site_coverage = compute_site_coverage(snapshot.body_site_map, signature.body_site_requirements)
    required_site_coverage = 1.0 - signature.tolerance.site_variance
    
    if site_coverage < required_site_coverage:
        return false
    
    return true
```

### 6.2 Pattern Signature: The Canonical Form

A PatternSignature is the invariant representation of a cluster. Two organisms are said to share a pattern if their primitive sets and body-site configurations map onto the same signature within tolerance windows.

**Example: Echolocation Signature**
```
echolocation_signature = PatternSignature(
    id: "sig_echolocation_v1",
    primitive_cluster: [
        "emit_acoustic_pulse",
        "receive_acoustic_signal",
        "spatial_integrate"
    ],
    parameter_bounds: {
        "frequency": Bound(center: 50kHz, range: 30kHz),  // 20-80 kHz
        "pulse_rate": Bound(center: 10, range: 5),        // 5-15 pulses/sec
        "reception_threshold": Bound(center: 0.1, range: 0.05)
    },
    body_site_requirements: ["head", "sensory_organs"],
    co_occurrence_threshold: 0.8,  // must have 2+ of the 3 primitives
    tolerance: {
        parameter_variance: 0.3,    // 30% allowed deviation from bounds
        site_variance: 0.2          // 80% of required sites must carry primitives
    }
)

echolocation_in_bats = snapshot {
    active_primitives: ["emit_acoustic_pulse", "receive_acoustic_signal", "spatial_integrate"],
    parameter_state: { "frequency": 45kHz, "pulse_rate": 12Hz, "reception_threshold": 0.08 },
    body_site_map: { "head": [emit, receive], "wing_margin": [spatial_integrate] }
}
// ✓ Matches echolocation_signature within tolerance

echolocation_in_shrews = snapshot {
    active_primitives: ["emit_acoustic_pulse", "receive_acoustic_signal"],  // missing spatial_integrate
    parameter_state: { "frequency": 55kHz, "pulse_rate": 18Hz },
    body_site_map: { "head": [emit, receive] }
}
// ✓ Matches (has 2/3 primitives; meets co_occurrence_threshold of 0.8 for 2/3 = 0.67... wait, let me recalculate)
// Actually: 2 of 3 = 0.667, threshold 0.8 requires 2.4, so this does NOT match
// Must have tighter thresholds or lower threshold
```

### 6.3 Stability Thresholds: When a Pattern Becomes Real

A pattern is considered "stable" and thus a candidate for labeling when it meets ALL of:

1. **Population Threshold (N)**: The pattern is observed in at least N distinct organisms across the sampled population. Default: N = 20.
   - *Rationale*: A single freak mutation is not a pattern; 20 independent occurrences across different lineages shows heritable evolution.

2. **Temporal Persistence (M)**: The pattern persists across at least M contiguous observation ticks. Default: M = 200 ticks.
   - *Rationale*: A transient mutation that appears once and vanishes is noise; 200 ticks (~20 sim-days at default tick speed) shows that the pattern maintains itself across generations.

3. **Lineage Diversity (K)**: The pattern is observed in at least K distinct evolutionary lineages. Default: K = 2.
   - *Rationale*: Convergent evolution. If the same pattern appears in two unrelated lineages, it is a stable attractor in the adaptive landscape, not a lineage-specific quirk.

```
function check_label_stability(signature: PatternSignature) -> void:
    N_THRESHOLD = 20   // tunable
    M_THRESHOLD = 200  // tunable
    K_THRESHOLD = 2    // tunable
    
    if (len(signature.recent_match_count) >= N_THRESHOLD and
        signature.temporal_persistence >= M_THRESHOLD and
        len(signature.recent_lineages) >= K_THRESHOLD):
        
        // Pattern is stable
        if signature.label == null:
            // Create new label
            create_emergent_label(signature)
        else:
            // Existing label; increment stability evidence
            signature.label.observed_count += len(signature.recent_match_count)
            signature.label.tick_persistence += M_THRESHOLD

function create_emergent_label(signature: PatternSignature) -> EmergentLabel:
    // Three-source pipeline for label assignment
    
    // Source 1: Hand-Seeded Thesaurus
    thesaurus_match = match_thesaurus(signature)
    if thesaurus_match != null:
        label_string = thesaurus_match.biological_term
        naming_source = ThesaurusMatch
        confidence = compute_match_confidence(signature, thesaurus_match)
        
        label = EmergentLabel(
            label_string: label_string,
            pattern_signature: signature,
            naming_source: naming_source,
            thesaurus_match_confidence: confidence,
            discovery_tick: current_tick
        )
        
        signature.label = label
        chronicler.emergent_labels.add(label)
        return label
    
    // Source 2: Template-Compositional Naming
    // If no thesaurus match, build a name from primitives and body sites
    primitive_categories = [categorize_primitive(p) for p in signature.primitive_cluster]
    body_sites = signature.body_site_requirements
    
    // e.g., ["acoustic", "spatial"] + ["head"] = "acoustic_spatial_perception"
    label_string = compose_label_name(primitive_categories, body_sites)
    naming_source = TemplateComposition
    
    label = EmergentLabel(
        label_string: label_string,
        pattern_signature: signature,
        naming_source: naming_source,
        discovery_tick: current_tick
    )
    
    signature.label = label
    chronicler.emergent_labels.add(label)
    
    // Source 3: Faction Coining (optional override)
    // If a faction observes this pattern, they may coin a term
    observing_factions = find_factions_with_label_observations(label)
    for faction in observing_factions:
        if faction.language_system != null:
            faction_term = faction.language_system.coin_term_for_pattern(signature)
            if faction_term != null:
                // Faction's label overrides default in faction dialogue/lore
                label.faction_coins[faction.id] = faction_term
    
    return label

function match_thesaurus(signature: PatternSignature) -> ThesaurusEntry or null:
    // Try to match signature against hand-seeded thesaurus entries
    // Ranking: highest match confidence wins
    
    matches = []
    for entry in LABEL_THESAURUS.entries:
        confidence = compute_signature_similarity(
            signature,
            entry.pattern_signature_template
        )
        if confidence > 0.6:  // tunable threshold
            matches.append((entry, confidence))
    
    if len(matches) == 0:
        return null
    
    return matches.max_by(confidence)[0]

function compute_signature_similarity(sig1, sig2) -> float:
    // Return [0, 1] similarity score
    // Considers primitive overlap, parameter ranges, body sites
    
    primitive_overlap = len(set(sig1.primitive_cluster) ∩ set(sig2.primitive_cluster)) / \
                       len(set(sig1.primitive_cluster) ∪ set(sig2.primitive_cluster))
    
    site_overlap = len(set(sig1.body_site_requirements) ∩ set(sig2.body_site_requirements)) / \
                   max(len(sig1.body_site_requirements), len(sig2.body_site_requirements))
    
    return (primitive_overlap * 0.6 + site_overlap * 0.4)  // weighted average

function compose_label_name(primitive_categories: list<string>, body_sites: list<string>) -> string:
    // Build a template-compositional name
    // e.g., ["thermal_sense", "directional_hunt"] + ["head"] = "thermal_directional_head_sensing"
    
    category_string = "_".join(primitive_categories)
    site_string = "_".join(body_sites)
    
    return category_string + "_" + site_string
```

### 6.4 Label Persistence & Extinction Handling

Once created, an EmergentLabel is stored as part of the save state. Labels persist even if the pattern's population shrinks below thresholds.

```
function monitor_label_extinction() -> void:
    // Called during Chronicler update
    // If a label's population drops below extinction floor, mark it extinct
    
    EXTINCTION_FLOOR = 5  // if fewer than 5 organisms exhibit pattern, mark extinct
    
    for label in chronicler.emergent_labels:
        if label.is_extinct:
            continue  // already marked
        
        current_pop = count_organisms_matching_signature(label.pattern_signature)
        
        if current_pop < EXTINCTION_FLOOR:
            label.is_extinct = true
            label.extinction_tick = current_tick
            
            // Historical record: retain label in lore even after extinction
            label.retained_in_lore = true
            
            // Notify narrators: this pattern is now gone from the world
            record_event(
                event_type: SpeciesExtinction,  // repurposed for label extinction
                agents_involved: [label.id],
                description: "The last of the " + label.label_string + " have perished."
            )
```

### 6.5 Determinism & Seeding

Label creation is deterministic. Identical simulation runs (same seed) produce identical labels at identical ticks.

```
function deterministic_label_creation() -> void:
    // All random decisions in pattern matching and label naming are seeded by:
    // - Current tick
    // - Signature ID
    // - Organism lineage IDs in the match set
    
    // This ensures that replaying a sim produces the same labels in the same order
    seed = hash(current_tick, signature.id, sorted(signature.recent_lineages))
    prng = SeededRandom(seed)
    
    // Use prng for any stochastic label-assignment choices
    if thesaurus_matches_are_ambiguous:
        chosen_match = select_by_rng(matches, prng)
    else:
        chosen_match = matches[0]
```

### 6.6 Integration with Event Indexing

Emergent labels are a new class of indexed events. When a label stabilizes, the Chronicler records a "LabelDiscovery" event. When a label goes extinct, a "LabelExtinction" event is recorded.

```
EventType additions:
    LabelDiscovery,      // A new emergent ability/pattern becomes stable
    LabelExtinction,     // A previously common pattern falls below floor
    
ChronicleEntry {
    // existing fields ...
    
    // New optional field for label-related entries
    associated_label: EmergentLabel or null
}

function record_label_discovery(label: EmergentLabel) -> void:
    // When a pattern crosses stability threshold, record it as a major event
    
    entry = ChronicleEntry(
        event_type: LabelDiscovery,
        timestamp: current_tick,
        location: centroid_of_organisms_matching_signature(label.pattern_signature),
        agents_involved: [organisms_in_label_match_set],
        associated_label: label,
        description: "A new pattern emerges: " + label.label_string,
        salience: 0.4  // moderate importance
    )
    
    chronicler.event_log.append(entry)
```

### 6.3 Composite Observable Signatures (Issue #12)

**Definition**: When a stable primitive cluster becomes a labeled ability (e.g., "echolocation", "venom injection"), its composite signature is derived from its constituent primitive effects.

**Signature Composition Rules**:

1. **Modalities Union**: The composite signature's modality set is the union of all sub-primitive modalities. If a cluster contains primitives with modalities {acoustic_output, spatial_integration}, the composite's modality = {acoustic_output, spatial_integration}.

2. **Range Maximum**: For each modality dimension (e.g., frequency, intensity), the composite range is the maximum range across all constituent primitives. Example: if emit_acoustic_pulse has frequency ∈ [1 kHz, 50 kHz] and receive_acoustic_signal has ∈ [0.5 kHz, 100 kHz], the composite's frequency range is [0.5 kHz, 100 kHz].

3. **Frequency Intersection**: Where frequency ranges overlap across multiple primitives, the composite's "effective frequency" is the intersection (frequencies where both emission and reception are possible). This defines the "sweet spot" for the ability.

**Detectability of Composites**:

```
composite_detectability = min(sub_primitive_detectabilities)
```

The hardest-to-detect component dominates. Example: echolocation = min(detectability of acoustic_output, detectability of spatial_integration). If spatial_integration is nearly undetectable but acoustic_output is loud, echolocation remains detectable overall because the acoustic pulse is observable.

**Faction Knowledge Diffusion**:

Once a composite label is assigned to a primitive cluster, factions observe instances of organisms performing that composite ability. Through knowledge diffusion (System 03), NPCs propagate names for the ability:
- **Scholar factions**: Use compositional labels ("high-frequency acoustic ranging").
- **Mystic factions**: Use evocative names ("spirit-echo", "phantom-voice").
- **Keeper factions**: Preserve canonical thesaurus names ("echolocation").

All three refer to the same underlying mechanics; divergence in naming is purely NPC-cultural.

---

## 8. Tradeoff Matrix

| Dimension | Simulated Lore | Hand-Authored | Winner |
|-----------|---|---|---|
| **Authenticity** | Emerges from actual events | Carefully designed | Hand-Authored (short-term) |
| **Coherence** | Internally consistent by construction | Must manually edit contradictions | Simulated |
| **Scalability** | Millions of micro-events auto-clustered | Manual authoring of each age | Simulated |
| **Player Agency** | Lore changes based on player choices | Static backdrop | Simulated |
| **Surprise** | Actual future differs from seed lore | All futures written in advance | Simulated |
| **Locality** | Different in different regions/playthroughs | Canonical | Simulated |
| **Emotional Resonance** | "I shaped this world" | "I witnessed art" | Hand-Authored (immediate) |

**Winner**: Simulated. Dynamism and scope outweigh certainty.

**Label-specific tradeoffs**:

| Dimension | Thesaurus Match | Compositional | Faction Coining |
|---|---|---|---|
| **Authorial Control** | High (curated terms) | Low (algorithmic) | Medium (faction-driven) |
| **Narrative Surprise** | Low (known patterns) | Medium (novel terms) | High (unexpected local names) |
| **Mechanical Clarity** | High (clear intent) | Medium (descriptive) | Low (may obscure mechanics) |
| **Cultural Immersion** | Low (generic) | Medium (logical) | High (faction-specific dialect) |
| **Performance** | Fast (lookup) | Moderate (composition) | Moderate (language system) |

**Recommendation**: Deploy all three sources. Thesaurus provides landmarks; compositional handles novel combinations; faction coining enriches faction dialogue. In a single playthrough, a label will be discovered once (deterministically), but NPCs may reference it using faction-specific names.

---

## 9. Emergent Properties

1. **Contested History**: Multiple narrators produce contradictory accounts of the same event. Scholar vs. Mystic vs. Keeper factions each emphasize different aspects. Player must triangulate truth.

2. **Era Redefinition**: Player learning in Age 3 might discover that what was labeled "Age 6" actually contained TWO distinct eras (a civilization rise and fall within). Narrators disagree on periodization.

3. **Technology Regression**: If simulation enters an era of conflict and isolation, technology_level can drop, despite seed history saying it "should" progress. This breaks player preconceptions.

4. **Lore Contradictions as Features**: Narrator A says cacogens "came to harvest" (extractive interpretation). Narrator B says they "came to study" (scientific). Narrator C says they "came to warn" (mystical). No narrator is lying — they're interpreting fragmentary evidence through different lenses.

5. **Cacogen Indigenization**: Over thousands of ticks, creatures with alien lineage tags become common predators, treated as native fauna by NPCs. "Have they always been here?" becomes unanswerable.

6. **Keeper Authority Paradox**: Keeper factions claim knowledge preservation, but if simulation contradicts seed lore, Keepers face a crisis of authority. Do they adapt their teachings or insist on inherited truth?

7. **Era Naming Wars**: When a new simulation era is detected, different factions propose different names, each emphasizing their interpretation. The winning name becomes canonical only by consensus.

---

## 10. Open Calibration Knobs

1. **Salience Weighting**: How is event importance calculated? Currently: `population_affected × novelty × persistence`. Adjust weights to make ecological events vs. political events more/less prominent.

2. **Confidence Decay Rate**: How fast do narrators forget? Default: 50% decay per 1000 ticks. Shorter decay = oral history fades quickly; longer = legends persist.

3. **Noise Injection Threshold**: At what confidence level do narrative errors appear? Currently: errors when `resolution < 0.3`. Lower = earlier degradation.

4. **Era Detection Window**: How many ticks of history to analyze? Default: 10,000 ticks. Shorter = more era transitions (finer periodization); longer = slower era changes.

5. **Bias Amplitude**: How strongly do narrator personalities reinterpret events? Default: bias scales from 0.0 to 1.0 based on faction opinion delta. Could make Scholar narrators more resistant to bias.

6. **Keeper Value Drift Resistance**: How much do Keeper factions resist opinion drift? Default: 80% resistance on tradition_vs_innovation. Can tune per archetype.

7. **Cacogen Abundance**: What fraction of initial creatures have alien lineage tags? Default: 1-2%. Higher = more exotic fauna appears quickly.

8. **Knowledge Gate Strictness**: Should lore fragments be gated on knowledge of previous events, or always accessible? Gate early/late to control discovery pacing.

**Label Discovery Knobs**:

9. **Label Stability Thresholds** (N, M, K):
   - **N** (population): Minimum organisms exhibiting pattern. Default: 20. Lower = labels discovered faster but more fragile; higher = only robust patterns labeled.
   - **M** (temporal persistence): Minimum contiguous ticks for stability. Default: 200. Lower = volatile labeling; higher = stable lore but slower discovery.
   - **K** (lineage diversity): Minimum lineages showing pattern. Default: 2. Set to 1 for rapid labeling; 3+ for strict convergence-only labels.

10. **Thesaurus Match Confidence Threshold**: Minimum similarity score for hand-seeded thesaurus matching. Default: 0.6. Higher = more compositional labels (less familiar); lower = more thesaurus hits (canonical terminology).

11. **Pattern Sampling Interval**: How many organisms to sample per tick for primitive analysis. Default: 1%. Lower = expensive but finer-grained discovery; higher = coarse but performant.

12. **Label Extinction Floor**: Minimum population below which label is marked extinct. Default: 5 organisms. Controls how "recently alive" patterns must be.

13. **Faction Coining Rate**: Probability a faction's language system coins a term for a newly discovered label. Default: 0.5 per faction per label. Controls dialect density.

---

## 11. The Nine Seed Ages

### Age 9: The Bright Horizon (the first golden age)

**Designers' knowledge**: A pre-Harvest human civilization that achieved space travel and genetic engineering. The present-day ruins are orbital platforms, genetic research facilities, and cities built with advanced technology.

**Key features**:
- Technology level 5 (post-human, incomprehensible)
- Dominant faction types: Technocracy, Meritocracy
- Genetic engineering created the biological diversity that "evolved" post-Harvest
- The Harvest was a first-contact event; cacogens came for resources and biological samples

**Lore hooks**:
- Ruins of orbital platforms (pristine but dead, AI systems dormant)
- Genetic vaults: sealed chambers containing genetic templates
- The Synthesis activation site (located in a deep-ocean facility)

**Cultural memory**: "The time before the Drowning" or "The Age of Sky." Most contemporary cultures have completely mythologized this era.

---

### Age 8: The Receding (desperate hybrid age)

**Designers' knowledge**: The immediate post-Harvest collapse. Humans with remaining technology tried to maintain civilization while the Harvest cacogens withdrew. Coastal settlements abandoned as oceans rose. Pockets of high-tech holdouts eventually failed (power grids collapsed, supply chains broke).

**Key features**:
- Technology level 3-4 (declining)
- Dominant faction types: Warlord states, Technological enclaves
- Rapid population decline as food systems failed
- Beast species diversified as human dominion receded

**Lore hooks**:
- Ruins of hybrid construction: old-world tech buildings repurposed as forts
- Keeper vaults established during this age (preservation efforts as tech failed)
- Beast domestication attempts (some species were briefly enslaved before going wild again)

**Cultural memory**: Almost entirely lost. Treated as a dark age; no oral tradition preserved.

---

### Age 7: The Cradle Kingdoms (early medieval age)

**Designers' knowledge**: Humans reorganized into agrarian kingdoms. Technology largely forgotten except as magical artifacts. Population slowly rebounded. Inter-island maritime trade began. This age saw the longest peace until the Sail Ascendancy.

**Key features**:
- Technology level 1 (stone, bronze, sailing)
- Dominant faction types: Kingdoms, Tribal confederacies, Merchant guilds
- Keeper tradition formalized as sacred preservation (not yet understood)
- Beast species became fundamental to ecology (hunted for food, domesticated as beasts of burden)

**Lore hooks**:
- Ruins of stone cities with artistic masonry
- Keeper vaults from this era contain early attempts at documentation
- Named individuals: legendary founders (some historical, many mythical)

**Cultural memory**: This is the "baseline" that many present-day factions trace their lineage to. Many false genealogies claim Cradle Kingdom origins.

---

### Age 6: The Sail Ascendancy (the first golden age of the post-Harvest)

**Designers' knowledge**: First inter-archipelago civilization. Advanced sailing technology enabled trade and communication. Population growth, specialization, the emergence of large cities. Keeper-scholars began systematic study of old-world ruins. 

**Critical event: The Cacogen Echo** — A post-Harvest cacogen vessel appeared in the sky, observed the largest city for 11 days, and departed. It communicated nothing and took nothing. The event traumatized the civilization and contributed to its collapse.

**Key features**:
- Technology level 2-3 (advanced medieval)
- Dominant faction types: Merchant republics, Maritime kingdoms, Scholar guilds
- First systematic study of pre-Harvest technology
- Population boom → overfishing → resource depletion

**Lore hooks**:
- The Cacogen Echo (primary mystery event)
- Ruins of harbor cities with sophisticated maritime engineering
- Academic texts and early maps (fragmentary)

**Cultural memory**: Remembered as the golden age ("when all the islands were one"). The Cacogen Echo is mythologized as divine visitation or warning.

---

### Age 5: The Long Silence (the first dark age)

**Designers' knowledge**: Millennium of fragmentation and regression. Inter-archipelago networks collapsed. Individual islands reverted to subsistence agriculture. Keeper tradition went underground (protecting knowledge from those who might misuse it). Beast species diversified rapidly as human hunting pressure eased.

**Key features**:
- Technology level 0-1 (subsistence)
- Dominant faction types: Tribes, Hermit settlements, Outlaw bands
- Written language lost in most regions
- Few records from this age exist

**Lore hooks**:
- Ruins from this age are sparse and temporary (wood, thatch structures don't survive)
- Cryptic Keeper records about "guarding against the return"
- Oral traditions about specific events (isolated to individual islands)

**Cultural memory**: Mostly forgotten. Called "the Gap" or "the Forgetting." Few cultures preserve stories from this era.

---

### Age 4: The Synthesis (the second golden age, doomed by ambition)

**Designers' knowledge**: A cluster of archipelagos independently redeveloped advanced metallurgy, mathematics, and preserved old-world technical documents. Synthesis scholars reverse-engineered elements of Bright Horizon technology, achieving:
- Desalination at scale
- Mechanical computing devices
- Partial understanding of genetic engineering
- Signal transmission using old-world infrastructure

**Critical event: The Awakening** — A Synthesis research team activated something in a deep-submerged cacogen installation. For 18 months, the installation broadcast a signal. Non-Earth entities responded, passed through, and left. The region experienced inexplicable phenomena (geometry that moved, sound that altered biology). The surviving population, traumatized, rejected systematic inquiry and burned their libraries.

**Key features**:
- Technology level 3-4 (quasi-industrial, then crashed)
- Dominant faction types: Scholarly oligarchy, Priesthood
- The Awakening trauma → anti-intellectual backlash
- Beast genetic engineering was understood (partially)

**Lore hooks**:
- Ruins of Synthesis cities: clean, geometric, recognizably "modern"
- Destroyed Keeper vaults (post-Synthesis purges)
- Computing devices (partially intact)
- The deep-submerged installation (still active, broadcasting faintly)

**Cultural memory**: The most vivid cautionary tale. "They woke the deep" is the idiom for reckless ambition. The Awakening is the world's central trauma, comparable to the Fall or Pandora's Box.

---

### Age 3: The Binding (age of faith)

**Designers' knowledge**: Civilizational overcorrection. Surviving cultures elevated spiritual authority over inquiry. Keeper lineages were co-opted by religions and transformed from scholars into custodians of mystery. Old-world artifacts were worshipped, not studied. Functional technology was monopolized by priestly elites to reinforce authority.

**Key features**:
- Technology level 1 (intentionally constrained)
- Dominant faction types: Theocracies, Mystery cults, Priestly hierarchies
- The longest-lived post-Harvest civilization (over a millennium)
- Elaborate cacogen theology (angels vs. demons)

**Lore hooks**:
- Monumental temple complexes (often built around Keeper vaults)
- Religious texts blending Synthesis wisdom with mysticism
- Cacogen theology documents (wild speculation)
- Hierarchical artifacts and reliquaries

**Cultural memory**: The most directly remembered age. Religious frameworks from the Binding persist in diluted form. The tension "sacred mystery vs. dangerous knowledge" still defines how people view technology.

---

### Age 2: The Unraveling (the second dark age)

**Designers' knowledge**: Theocracies rotted from within. Priestly doctrine drifted from observable reality. Competing theocracies warred. The Binding's monopoly on functional technology couldn't be maintained without understanding it — corruption and hypocrisy eroded authority.

**Critical event: The Drifting** — Tectonic instability and sea level change physically reshaped the archipelago over two centuries. Islands drifted apart, new ones surfaced, old ones submerged. The map of the world changed. This exposed unprecedented quantities of pre-Harvest ruins and destabilized every territorial claim.

**Key features**:
- Technology level 1 (priesthood lost understanding)
- Dominant faction types: Warlord states, Wandering Keepers, Secular strongholds
- Replacement of religious authority with military authority
- Ecological disruption from The Drifting

**Lore hooks**:
- Ruins showing militarization: fortifications, watchtowers
- Portable Keeper vaults (Drifting Keepers carrying collections)
- Chronicles of The Drifting itself (geological records, navigation journals)
- Artifacts exposed by geological shifts

**Cultural memory**: Within living cultural memory (4-5 generations). Remembered as the time of warlords and wandering holy men. The Drifting is remembered vividly.

---

### Age 1: The Present Tide (the player's age)

**Designers' knowledge**: A new tentative equilibrium. Population small (few hundred thousand). Technology broadly medieval with local variation. The Keeper tradition fragmented: some Keepers are respected scholars, some are feared sorcerers, some are peddlers of relics, some are charlatans.

**Key features**:
- Technology level 1-2 (medieval, with pockets of preserved tech)
- Dominant faction types: Merchant republics, City-states, Tribal confederacies, Keeper lineages
- Ingredients for a third golden age exist (salvage abundant, trade reestablishing)
- Ingredients for a third dark age also exist (conflict over salvage, ecological instability)

**Lore hooks**:
- Layered ruins (every settlement sits on older ruins)
- Living Keeper lineages with competing claims to authentic tradition
- Cacogen encounters (ongoing, mysterious)
- Possibility: the Drifting continues; geology is still active

**Cultural memory**: The present is being historicized in real-time. Narrators are still debating what the Unraveling meant and what the present portends.

---

## 12. The Watchers: Irreducible Mystery

Scattered across multiple ages, in sources that don't reference each other, the same geometric symbol appears:
- In Bright Horizon orbital facility blueprints
- Carved into a Cradle Kingdom temple foundation
- In Synthesis technical documents (margins, never in official text)
- In Binding-era religious art (as a mandala, meditation focus)

The pattern suggests a presence continuously observing Earth since before the Harvest. Not a species, not a system — a *something* that watches and never acts.

Whether the Watchers are real or coincidental pattern is deliberately unresolved. They exist to give the deepest lore an asymptotic mystery: the more the player learns, the more the pattern seems to cohere, but it never fully resolves. This is the Wolfe technique — the mystery that deepens with scrutiny rather than dissolving.

---

## 13. Implementation Notes

- **Determinism**: Chronicle entry generation is seeded by location and tick, so the same location at the same tick always produces the same event (if the preconditions are met).

- **Narrator Proliferation**: Don't create a narrator for every NPC. Instead, associate narrators with factions (one historian per faction, for instance). Multiple NPCs can reference the same narrator's knowledge.

- **Fragment Gating**: Use sparingly. Gate fragments on discovering previous events to create a sense of "unlocking" history. But don't gate so much that players feel blocked from lore discovery.

- **Contradiction Tolerance**: Embrace contradictions. Don't patch over them. Contradictions are where players engage with lore depth.

---

## 13. Cataloging & Naming: The Chronicler as UI Data Source

The Chronicler system is not merely a lore engine; it is the authoritative source for all user-facing catalogs. As simulation data accrues, the Chronicler indexes it into queryable structures that the UI layer consults directly, never bypassing into raw mechanical data. This section specifies the catalogs, their contents, naming pipelines, and the read-only Query API through which the UI accesses them.

### 13.1 Catalog Surfaces

The Chronicler produces and maintains five primary catalogs, each serving a distinct UI layer need:

#### Bestiary Index

Indexed collection of all discovered creature lineages, discoverable through encounters, ecological observation, and lore.

```
BestiaryEntry {
    id: unique_id
    lineage_id: LineageID                    // primary key to phylogeny
    chronicler_label: string                 // canonical name from EmergentLabel discovery
    
    // Mechanical signature (opaque to UI; UI never reads primitives)
    primitive_signature: PatternSignature    // canonical form; UI does not parse
    observed_ecology: EcologySnapshot {
        biome_distribution: map<Biome, float>  // probability by region
        diet_classification: string             // herbivore/omnivore/etc.
        threat_level: enum { Harmless, Dangerous, Apex }
        population_trend: enum { Growing, Stable, Declining, Extinct }
        last_sighting_tick: uint64
    }
    
    // Lore & discovery
    first_discovered_tick: uint64
    discovery_location: WorldLocation
    discoverer_faction: EntityID or null
    
    // Alternate names (faction-coined or player custom)
    alternate_names: list<AlternateName> {
        name: string
        source: enum { FactionCoined, PlayerJournal, Thesaurus }
        faction_id: EntityID or null         // if FactionCoined
        confidence: float                    // how widespread is this name?
    }
}
```

#### Material Index

Canonical catalog of all discovered material types and crafting inputs. Indexed by material signature and thesaurus classification.

```
MaterialEntry {
    id: unique_id
    material_stack_canonical: MaterialStack  // the primary mechanical form
    thesaurus_name: string                   // "dense red mineral", "supple hide"
    
    // Provenance & discovery
    first_discovered_tick: uint64
    discovery_locations: list<WorldLocation>
    sources: list<MaterialSource> {
        source_type: enum { Creature, Deposit, Craft, Harvest }
        source_entity: EntityID or null
        abundance: enum { Rare, Uncommon, Common, Abundant }
    }
    
    // Recipes & economics
    discovered_recipes: list<RecipeReference> {
        recipe_id: RecipeID
        discoverer_faction: EntityID or null
        first_crafted_tick: uint64
    }
    market_data: MarketSnapshot {
        factions_trading: list<FactionID>
        price_trend: float                   // relative stability
        demand_level: enum { Low, Moderate, High, Critical }
    }
    
    // Faction terminology
    faction_names: list<AlternateName>      // faction-specific material terms
}
```

#### Event Index

All Chronicler-indexed events with rich filtering and navigation. Serves timeline views, event browsing, and lore discovery gates.

```
EventIndexEntry {
    id: unique_id
    chronicle_entry: ChronicleEntry         // reference to original event
    
    // Searchable metadata
    event_type: EventType
    era_tags: list<int>                     // which simulation eras is this relevant to?
    faction_tags: list<FactionID>           // which factions were involved?
    region_tag: WorldRegion                 // geographic pivot
    biome_tags: list<Biome>                 // ecological context
    
    // Narrative accessibility
    indexed_description: string              // searchable prose
    salience_rank: float                    // for sorting by "importance"
    confidence_summary: float               // aggregate of confidence vector
    narrator_count: int                     // how many factions have accounts?
    
    // Content gates
    discovery_locked: bool                  // requires knowledge_gate?
    knowledge_gates: list<ChronicleEntry>  // what must player know first?
    discovery_tick: uint64 or null          // when was this discovered by player?
}
```

#### Faction Index

Catalog of all known factions, their relationships, practices, and reputation with the player.

```
FactionEntry {
    id: FactionID (primary key)
    self_name: string                       // faction's own name (System 18)
    player_custom_name: string or null      // player's journal override
    
    // Internal structure & knowledge
    faction_type: string                    // "merchant_republic", "theocracy", etc.
    cultural_practices: list<CulturalPractice>
    faction_language: LanguageDialect       // language system (System 18)
    territory: list<WorldLocation>
    
    // Relations & politics
    relations_matrix: map<FactionID, float> // [-1, 1] opinion vectors
    treaties: list<Treaty>
    conflicts: list<ConflictRecord>
    
    // Reputation with player
    reputation_with_player: float           // [-1, 1]
    player_interaction_log: list<Interaction> {
        interaction_type: enum { Trade, Dialogue, Combat, Ritual, Diplomacy }
        tick: uint64
        outcome: string
    }
    
    // Lore & presence
    founding_event: ChronicleEntry or null
    key_individuals: list<NamedIndividual>
    ritual_practices: list<Ritual>
}
```

#### Lineage Tree

The phylogenetic structure of all discovered lineages, produced from speciation events in the ecological simulation.

```
LineageNode {
    lineage_id: LineageID
    parent_lineage: LineageID or null       // null for root
    children: list<LineageID>
    
    // Tree metadata
    depth: int                              // generation count from root
    branch_tick: uint64                     // when did this lineage branch?
    
    // Lineage properties
    chronicler_label: string                // emergent ability/form name
    phenotype_summary: string               // descriptive form
    population_count: int                   // living members
    extinction_status: enum { Alive, Extinct, Unknown }
    
    // UI presentation
    ecological_niche: string                // brief ecological role
    visual_silhouette: string               // reference to sprite/icon
    
    // Lore attachment
    lore_fragments: list<LoreFragmentID>    // which story beats mention this lineage?
}
```

### 13.2 Naming Pipeline

Each catalog entry acquires a canonical name through a deterministic pipeline. Names are indexical (in Kripke's 1980 sense of the baptism-at-first-observation chain): they refer to observed simulation state, and the naming is fixed at discovery time.

#### Lineage Naming

**Pipeline**:
1. **Chronicler Label Discovery** (from pattern signature matching, as specified in Section 3.7)
   - If primitive pattern matches a thesaurus entry (confidence > threshold): use thesaurus name + discovery tick.
   - If no match: generate compositional name from primitive categories (e.g., "acoustic-sensory flyer").
2. **Auto-Code Fallback** (deterministic encoding)
   - If label discovery fails: assign code of form "α-###" where α is the biome/ecology class and ### is a deterministic hash of the lineage's genetic signature.
   - Example: "ρ-042" for a rainforest-class creature, where ρ is the biome letter and 042 is hash(genotype).
3. **Per-Faction Coined Terms** (System 18 Language)
   - Each faction may independently coin a name for a discovered lineage. Faction language system generates a term rooted in their morphological / phonological conventions.
   - Example: same creature called "echo-thing" by merchant republic, "singing-fang" by theocracy.
   - These terms override the Chronicler label in faction-specific dialogue but the Chronicler label remains canonical in bestiary.

**Determinism**:
- Labeling depends only on (primitive_signature, discovery_tick, biome_context). Same simulation state at the same tick produces the same label.
- Thesaurus matching is deterministic (confidence scores computed from signature distance metrics).
- Faction coining is deterministic (faction language system is seeded; same lineage ID and faction seed produce same term).

**Research anchor**: 
Following Kripke (1980) on rigid designation and Putnam (1975) on the division of linguistic labor, names are anchored to the moment and place of first observation. The Chronicler is the "baptizer" — it fixes the reference of a name to a pattern in the world. Subsequent uses of the name (by NPCs, in dialogue, in bestiary entries) all refer back to that same pattern, even if the pattern's details become clearer or the population's phenotype drifts.

#### Material Naming

**Pipeline**:
1. **Thesaurus Match by Property Signature**
   - Each material's chemical/structural properties (density, color, malleability, elemental composition) are hashed into a property signature.
   - Thesaurus lookup: does this signature match a known real-world material (copper, chitin, obsidian)? If confidence > threshold, use the thesaurus name.
2. **Compositional Name Generation**
   - If no thesaurus match: generate a descriptive name from property keywords: "dense red mineral", "translucent chitinous plate", "supple hide with iridescent sheen".
   - Composition is rule-based and deterministic.
3. **Faction-Coined Term Override**
   - Faction language system generates an alternative term (e.g., "red-blood stone" vs. "cinnabar ore").
   - Like lineage naming, faction names are deterministic and do not override the canonical name.

#### Event Naming

**Pipeline**:
- Events are procedurally named as compositional sentences built from (actor, verb, outcome).
- Example: "The Theocracy of the Drift Shores declares war on the Merchant Syndicate" (FactionFounding + ConflictEruption).
- Narrators may distort or abbreviate event names (decay, bias); the Chronicler stores the canonical form.

#### Faction Naming

**Pipeline**:
- Factions are self-named (System 18: factions generate their own names via cultural identity subsystem).
- Player may override faction names in their personal journal (PlayerJournal scope).
- Canonical name is the faction's self-name; player customization is stored separately in GameState, never in Chronicler data.

### 13.3 Query API for UI

The UI layer never reads mechanical data (primitives, genotypes, recipes) directly through the Chronicler. Instead, it queries the Chronicler via a read-only API that returns UI-safe, label-backed structures. All queries are deterministic and produce consistent results across playthroughs (given the same simulation state).

```rust
impl Chronicler {
    /// Retrieve bestiary entries, filterable by discovery status, lineage, region, threat level.
    /// Returns entries in alphabetical or by-salience order.
    fn get_bestiary_entries(
        &self,
        filter: BestiaryFilter {
            discovered_only: bool,
            lineage_id: Option<LineageID>,
            region: Option<WorldRegion>,
            threat_level: Option<ThreatLevel>,
            sort_by: enum { Alphabetical, DiscoveryDate, PopulationTrend },
        }
    ) -> Vec<BestiaryEntry>;
    
    /// Retrieve material index entries, filterable by material type, abundance, recipe availability.
    fn get_material_entries(
        &self,
        filter: MaterialFilter {
            material_id: Option<MaterialID>,
            discovered_only: bool,
            has_recipe: bool,
            availability: Option<Abundance>,
            sort_by: enum { Name, Discovery, Abundance },
        }
    ) -> Vec<MaterialEntry>;
    
    /// Retrieve event feed with temporal and thematic filters.
    /// window: (tick_start, tick_end) or (era_start, era_end)
    fn get_event_feed(
        &self,
        filter: EventFilter {
            era: Option<int>,
            faction: Option<FactionID>,
            event_type: Option<EventType>,
            region: Option<WorldRegion>,
            locked_only: bool,           // show only undiscovered events?
            sort_by: enum { Chronological, Salience, RecentFirst },
        },
        window: TimeWindow,
    ) -> Vec<EventIndexEntry>;
    
    /// Retrieve faction list with reputation and relation info.
    fn get_faction_list(
        &self,
        filter: FactionFilter {
            region: Option<WorldRegion>,
            faction_type: Option<String>,
            allied_to_player: bool,
            sort_by: enum { Name, Reputation, Founding },
        }
    ) -> Vec<FactionEntry>;
    
    /// Retrieve lineage tree rooted at a specific lineage, up to specified depth.
    /// depth: how many generations to traverse
    fn get_lineage_tree(
        &self,
        root: LineageID,
        depth: int,
    ) -> LineageNode;
    
    /// Retrieve string labels (names, descriptions) for any entity.
    /// Never expose mechanics; only labels.
    fn get_label_for_entity(
        &self,
        entity_id: EntityID,
        context: enum { Bestiary, Merchant, Faction, Scholar },
    ) -> Option<String>;
}
```

All queries are **read-only** and **deterministic**. The Chronicler is the authoritative source for UI-facing strings. No other system may produce or override bestiary names, event descriptions, or faction labels accessed by the UI.

### 13.4 Invariant: UI Never Reads Primitives

**Invariant 3.9** (from System 3 / this document):
The UI reads only the Chronicler's Query API outputs (catalogs, labels, narrative descriptions). The UI NEVER parses or displays mechanical data such as:
- Primitive effects (internal mechanical representation)
- Genotype details (genetic code)
- Recipe mechanics (crafting probability, yield)
- Faction opinion vectors (internal political state)

Instead:
- Primitive sets → Chronicler labels (bestiary names)
- Genotype drift → Chronicler lineage updates
- Recipe discovery → MaterialEntry.discovered_recipes
- Faction relations → FactionEntry.relations_matrix (opinions as symbolic cardinal directions, not numeric vectors)

This separation preserves **mechanical transparency**: players can understand the rules through UI, but cannot directly manipulate or exploit mechanical internals through UI queries.

### 13.5 Tradeoff Matrix: Cataloging Responsibility

| Dimension | Single Chronicler | Per-Domain Catalogs | Winner |
|---|---|---|---|
| **Consistency** | Single source of truth | Potential divergence | Single Chronicler |
| **Modularity** | Tight coupling to lore system | Independent maintenance | Per-Domain |
| **Performance** | Unified indexing overhead | Domain-specific optimization | Per-Domain |
| **Player Experience** | Unified discovery pace | Decoupled discovery | Single Chronicler |
| **Determinism** | Single RNG seed for all catalogs | Per-domain seeds (more fragile) | Single Chronicler |
| **UI Coupling** | UI directly queries Chronicler | UI depends on multiple subsystems | Single Chronicler |
| **Flexibility** | Harder to specialize catalog behavior | Easier to tune by domain | Per-Domain |

**Winner**: Single Chronicler. The coherence, determinism, and UI simplicity benefits outweigh modularity concerns. Cataloging responsibility belongs in the lore system because catalogs *are* lore — they're how players construct a narrative understanding of the world.

---

## 15. Migration Notes: Integration with Phenotype Interpreter Refactor

### 15.1 Version History

**v1.3 Changes (this version)**:
- Added Section 13: "Cataloging & Naming" — formalizes the Chronicler as the authoritative source for all UI-facing data surfaces (bestiary, materials, events, factions, lineages).
- Specified the five primary catalog types and their contents.
- Documented the naming pipelines for each catalog domain (lineages, materials, events, factions) with research grounding in Kripke's indexical reference and Putnam's division of linguistic labor.
- Formalized the Query API through which UI reads from the Chronicler (read-only, deterministic).
- Codified Invariant 3.9: UI never reads primitives; it reads only Chronicler labels.
- Added tradeoff matrix for single Chronicler vs. per-domain catalogs.

**v1.2 Changes**:
The Phenotype Interpreter (System 11) was refactored to emit only PrimitiveEffect sets instead of named abilities. This document describes the new responsibility for the Chronicler to *discover* labels for stable primitive clusters.

**Before (v1.1)**:
- Phenotype Interpreter produced: `Ability { name: "echolocation", range: 50m, frequency: 45kHz, ... }`
- Chronicler recorded: Events and eras only.
- Labels were authored; abilities were fixed per phenotype.

**After**:
- Phenotype Interpreter produces: `[PrimitiveEffect { emit_acoustic_pulse, receive_acoustic_signal, spatial_integrate }, ...]`
- Chronicler watches primitives and *discovers* labels: `EmergentLabel { label_string: "echolocation", pattern_signature: ..., ...}`
- Labels are emergent; patterns are flexible.

### 15.2 Data Migration

Existing save states will not have EmergentLabel records. Migration strategy:

```
function migrate_old_save_to_new_labels(old_save: OldSaveState) -> void:
    // On load of pre-refactor save:
    
    // 1. Clear old ability records
    for creature in old_save.creatures:
        creature.abilities.clear()
    
    // 2. Synthesize primitive effects from old abilities
    for creature in old_save.creatures:
        for old_ability in creature.old_abilities:
            new_primitives = synthesize_primitives_from_ability(old_ability)
            creature.phenotype.active_primitives.extend(new_primitives)
    
    // 3. Run initial label-discovery pass
    chronicler.discover_initial_labels_from_population()
    
    // 4. Re-run era detection on event log
    for old_event in old_save.chronicle_entries:
        if can_interpret_as_new_event_type(old_event):
            re_classify_event(old_event)
    
    save.upgrade_version = CURRENT_VERSION
```

### 15.3 Behavioral Changes

1. **Abilities are no longer static**: A creature's effective "abilities" may change if its phenotype drifts. The Chronicler will notice if new primitive patterns emerge or old ones fade.

2. **Labels are population-level**: A label appears when many organisms exhibit a pattern. A single freak mutation with a novel primitive doesn't get a label — it's a noise.

3. **Labels are shared**: If bat-like creatures and shrews both evolve echolocation, they share a label. This was impossible before when abilities were creature-type specific.

4. **Factions can coin dialects**: The same label may be known by different names in different factions. This adds to lore depth but complicates bestiary consistency — design choice: show all known names in bestiary, or faction-specific names in faction dialogue only?

### 15.4 Ambiguities & Design Decisions

**Q1: What if a creature's primitives change mid-simulation (via new mutations)?**
A: The Chronicler will detect the change. If the new primitive set matches an existing label's signature, the creature now "exhibits" that label. If it's novel, a new label may eventually be discovered. Creatures can be labeled differently over time.

**Q2: Can a single organism exhibit multiple labels?**
A: Yes. A creature might have both echolocation (acoustic) and electroreception (electrical) primitives. Both labels apply. In dialogue, NPCs may mention the creature's "dual senses."

**Q3: How do we prevent label proliferation?**
A: The stability thresholds (N, M, K) act as filters. Only robust patterns get labels. In a small world, fewer labels will stabilize.

**Q4: What happens if the player's understanding of creatures changes?**
A: Lore fragments (which cite labels) remain consistent, but bestiary entries update. If a new label is discovered mid-game, NPCs who previously said "I know not what that creature is" can now be more specific. This is intentional — the world's knowledge evolves.

**Q5: Should extinct labels appear in NPCs' dialogue?**
A: Yes, in historical/archival context. An old NPC might say: "In my youth, we feared the * [extinct label]*, but none have been sighted in decades." This enriches lore. Control via `label.retained_in_lore` flag.

### 15.5 Testing Strategy

1. **Synthetic Population Test**: Create a hand-designed set of creatures with known primitive patterns. Verify the Chronicler discovers and labels them correctly.

2. **Label Stability Sweep**: Run simulations with different (N, M, K) thresholds. Verify that labels stabilize at expected times and match thesaurus entries when appropriate.

3. **Determinism Check**: Run identical simulations twice (same seed). Verify labels are created at identical ticks with identical IDs and names.

4. **Narrative Coherence**: Generate lore fragments that reference labels. Verify that extinct labels still appear in historical accounts; that NPC dialogue cites labels appropriately; that bestiary entries are mechanically accurate.

5. **Faction Dialect**: Verify that faction language systems successfully coin alternate names for discovered labels.

### 15.6 Performance Considerations

Primitive-snapshot processing is expensive at scale. Recommend:

- **Sampling**: Process only a fraction of organisms per tick (default: 1%). Adjust via `pattern_sampling_interval` knob.
- **Batch Analysis**: Group pattern-matching into discrete analysis phases (every 500 ticks) rather than continuous processing.
- **Signature Caching**: Pre-compute signature matches for common patterns (echolocation, venom, etc.) to avoid redundant similarity checks.
- **Lazy Labeling**: Only compute label names when a signature stabilizes, not during candidate phase.

---

## 16. Query API — Formal Contract

**Scope**: System 23 (UI) queries System 09 (Chronicler) for all user-facing data. All queries are **read-only snapshots** against the currently committed simulation tick. Queries never modify sim state.

```rust
// Formal Chronicler Query API
trait ChroniclerQuery {
    /// Get all bestiary entries matching a filter.
    fn get_bestiary_entries(&self, filter: BestiaryFilter) -> Vec<BestiaryEntry>;
    
    /// Get a single bestiary entry by species ID.
    fn get_bestiary_entry(&self, species_id: SpeciesId) -> Option<BestiaryEntry>;
    
    /// Get all material entries matching a filter.
    fn get_material_entries(&self, filter: MaterialFilter) -> Vec<MaterialEntry>;
    
    /// Get paginated chronicle events (events feed).
    fn get_event_feed(&self, cursor: EventCursor, limit: usize) -> Vec<ChronicleEntry>;
    
    /// Get all factions as summary records.
    fn get_faction_list(&self) -> Vec<FactionSummary>;
    
    /// Get a lineage tree rooted at a given lineage ID.
    fn get_lineage_tree(&self, root: LineageId, depth: usize) -> LineageNode;
    
    /// Get the canonical label for a primitive fingerprint (if one exists).
    fn get_label_for_primitive_cluster(&self, cluster: PrimitiveFingerprint) -> Option<Label>;
    
    /// Get all labels that apply to a species.
    fn get_labels_for_species(&self, species_id: SpeciesId) -> Vec<Label>;
}

// ===== Supporting Data Structures =====

struct BestiaryFilter {
    /// Show only species with observation_count >= threshold.
    discovered_only: bool,
    /// Filter by region(s).
    regions: Option<Vec<RegionId>>,
    /// Filter by threat level: [0-5].
    threat_level_min: Option<u8>,
    threat_level_max: Option<u8>,
    /// Text search on canonical label or aliases.
    search_query: Option<String>,
    /// Sort order: "name", "threat", "discovery_date", "observation_count".
    sort_by: Option<String>,
}

struct BestiaryEntry {
    /// Canonical species identifier.
    species_id: SpeciesId,
    /// The primary label assigned by Chronicler (e.g., "Bat-like").
    canonical_label: Label,
    /// Simulation tick when first observation recorded.
    discovered_at_tick: u64,
    /// Total observations of this species.
    observation_count: u32,
    /// The primitive pattern this species stabilized under.
    primitive_fingerprint: PrimitiveFingerprint,
    /// Summary of observed active channels (e.g., "Acoustic, Kinetic").
    channel_summary: Vec<ChannelName>,
    /// Alternative names: faction-coined names, synonyms, historical names.
    aliases: Vec<Label>,
}

struct Label {
    /// Human-readable text (e.g., "echolocation", "fire-breather").
    text: String,
    /// Confidence [0.0, 1.0]; how likely is this label stable?
    confidence: f32,
    /// Provenance: where did this label originate?
    provenance: Provenance,
    /// Tick when label first achieved stability threshold.
    first_seen_tick: u64,
}

enum Provenance {
    /// Label derived from thesaurus (hand-curated mapping).
    Thesaurus,
    /// Label constructed compositionally (e.g., "fire" + "breath").
    Compositional,
    /// Label coined by a faction's language system.
    FactionCoined { faction_id: FactionId },
}

struct MaterialFilter {
    material_type: Option<String>,
    discovered_only: bool,
    has_recipe: Option<bool>,
    sort_by: Option<String>,
}

struct MaterialEntry {
    material_id: MaterialId,
    canonical_label: Label,
    discovered_at_tick: u64,
    observation_count: u32,
    aliases: Vec<Label>,
}

struct EventCursor {
    /// Pagination: how many events back from present to start.
    offset: usize,
}

struct FactionSummary {
    faction_id: FactionId,
    name: String,
    settlement_count: u32,
    population: u32,
    government_type: String,
    relation_to_player: enum { Allied, Neutral, Hostile },
}

struct LineageId {
    /// Lineage = clade (internal node in evolutionary tree).
    /// Distinct from SpeciesId, which is a leaf (extant species).
    id: u32,
}

struct SpeciesId {
    /// Species = extant or extinct leaf node.
    /// A species may exhibit multiple labels if it has multiple primitive patterns.
    id: u32,
}

struct LineageNode {
    lineage_id: LineageId,
    label: Option<Label>,
    /// Direct child clades.
    children: Vec<LineageNode>,
    /// Leaf species in this clade (if depth limit reached).
    species: Vec<(SpeciesId, Label)>,
}

struct PrimitiveFingerprint {
    /// Sorted vector of (primitive_id, frequency_bucket) pairs.
    /// Represents co-emission within a 10-tick observation window.
    /// Example: [(acoustic_pulse, 3), (spatial_integrate, 3), (receive_signal, 2)]
    signature: Vec<(PrimitiveId, FrequencyBucket)>,
}

type FrequencyBucket = u8;  // 0-5: absent to very-frequent
```

### 16.1 BestiaryEntry Semantics

- **species_id**: Globally unique identifier for this species (leaf node in phylogeny).
- **canonical_label**: The primary name assigned when the species' primitive pattern stabilized.
- **discovered_at_tick**: The first tick at which an organism of this species was sighted.
- **observation_count**: Incremented each time a creature of this species is encountered or observed. Used to determine "discovered" status in UI.
- **primitive_fingerprint**: A sorted vector of primitive IDs and their co-emission frequencies observed within 10-tick windows. This is the structural pattern that uniquely identifies the species.
- **channel_summary**: High-level description of which channels (acoustic, kinetic, chemical, etc.) are actively expressed.
- **aliases**: Faction-coined names, historical synonyms, or alternate canonical names (if multiple factions discovered the same species independently).

### 16.2 Label Semantics

- **text**: The human-readable name.
- **confidence**: How stable/reliable is this label? Once confidence crosses the stability threshold (Section 17.3), the label becomes canonical and is frozen.
- **provenance**: Where the label came from:
  - **Thesaurus**: Hand-authored name mapping (designer provides common descriptive names for fingerprints).
  - **Compositional**: Built from primitives (e.g., "flame" + "throw" = "fire-breather").
  - **FactionCoined**: Generated by a faction's language system (System 18); may differ from canonical.
- **first_seen_tick**: When the label first stabilized (confidence crossed threshold).

### 16.3 Lineage vs. Species Relationship

- **LineageId** represents a **clade** (internal node in the evolutionary tree). A clade is an ancestral lineage that may have diverged into multiple descendant species.
- **SpeciesId** represents a **species** (leaf node in the tree at the current moment). An extant species is a population of organisms exhibiting a stable primitive pattern.
- **Extinct species** remain in the phylogeny as lineage nodes but are no longer extant (no living organisms).
- A query for `get_lineage_tree(root, depth)` returns a tree structure: each node can represent either a clade (with children) or a leaf species.

---

## 17. Labeling Algorithm — Formal Spec

**Goal**: Assign human-readable labels to primitive patterns that emerge from the evolution system. Labels must be stable, unambiguous, and consistent across independent evolutionary instances.

### 17.1 Primitive Fingerprint Definition

A **PrimitiveFingerprint** is a structural signature of a creature's active primitive effects, invariant to lineage or species identity.

```
PrimitiveFingerprint {
    /// Sorted vector of (primitive_id, frequency_bucket) pairs.
    /// Frequency bucket: [0 = absent, 1 = rare, 2 = occasional, 3 = common, 4 = very common, 5 = always].
    signature: Vec<(PrimitiveId, u8)>,
}

/// Compute fingerprint from observed creatures in a 10-tick window.
fn compute_fingerprint_from_window(creatures: Vec<Creature>, window_ticks: usize) -> PrimitiveFingerprint {
    let mut primitive_counts: Map<PrimitiveId, u32> = Map::new();
    let mut window_count: u32 = 0;
    
    for creature in creatures {
        window_count += 1;
        for primitive in creature.phenotype.active_primitives {
            primitive_counts[primitive.id] += 1;
        }
    }
    
    // Compute frequency buckets
    let mut signature: Vec<(PrimitiveId, u8)> = Vec::new();
    for (prim_id, count) in primitive_counts {
        let ratio = count as f32 / window_count as f32;
        let bucket = match ratio {
            r if r == 0.0 => 0,
            r if r < 0.1 => 1,
            r if r < 0.3 => 2,
            r if r < 0.7 => 3,
            r if r < 0.95 => 4,
            _ => 5,
        };
        signature.push((prim_id, bucket));
    }
    
    // Sort by primitive ID for determinism
    signature.sort_by_key(|x| x.0);
    PrimitiveFingerprint { signature }
}
```

### 17.2 Stability Thresholds

A primitive fingerprint becomes **stable** (eligible for labeling) when it satisfies all three thresholds:

- **N ≥ 20**: At least 20 distinct creatures in the current population exhibit this fingerprint.
- **M ≥ 200**: The fingerprint has been observed co-emitted consistently for at least 200 simulation ticks (no major disruptions).
- **K ≥ 2**: The fingerprint appears in at least 2 independent lineages (convergent evolution). This ensures names are structural, not reference-dependent.

```
struct StabilityThresholds {
    N: u32 = 20,   // Population cohort size
    M: u64 = 200,  // Ticks of continuous co-emission
    K: u32 = 2,    // Independent lineages
}

fn is_stable(fingerprint: PrimitiveFingerprint, stats: FingerprintStats) -> bool {
    stats.population_count >= N 
    && stats.continuous_ticks >= M 
    && stats.lineage_count >= K
}
```

### 17.3 Three-Source Naming Pipeline

When a fingerprint stabilizes, the Chronicler attempts to assign a label in this priority order:

#### Phase 1: Thesaurus Lookup (Priority 1)
Check if the fingerprint matches a hand-curated pattern in the **Thesaurus**. The thesaurus maps known patterns to canonical names.

```
Thesaurus {
    [fingerprint] -> {
        canonical_name: "echolocation",
        description: "Acoustic emission and reception, tight feedback loop",
        category: "sense",
    }
}

if thesaurus.contains(fingerprint):
    label = thesaurus[fingerprint].canonical_name
    confidence = 0.95  // High confidence; matches known pattern
    return Label { text: label, confidence, provenance: Thesaurus, first_seen_tick: now }
```

#### Phase 2: Compositional Naming (Priority 2)
If not in thesaurus, construct a name from the primitives themselves.

```
fn compositional_name(fingerprint: PrimitiveFingerprint) -> String {
    // Extract semantic meaning from each primitive
    let parts: Vec<String> = fingerprint.signature
        .iter()
        .map(|(prim_id, _)| describe_primitive_semantically(*prim_id))
        .collect();
    
    // Join with hyphens or other joiner
    parts.join("-")
    // Example: "acoustic-pulse-receive-spatial-integrate" -> "echolocation-sense" (after curation)
}

label = compositional_name(fingerprint)
confidence = 0.65  // Medium confidence; composite names are less intuitive
return Label { text: label, confidence, provenance: Compositional, first_seen_tick: now }
```

#### Phase 3: Faction-Coined Names (Priority 3)
If still unmatched, delegate to faction language systems (System 18). Each faction may coin its own name based on cultural context.

```
if faction_has_discovered_this_fingerprint:
    faction_label = faction.language_system.coin_name(fingerprint, cultural_context)
    // Register as a faction-specific alias, not canonical
    register_alias(fingerprint, faction_label, provenance: FactionCoined { faction_id })
```

### 17.4 Label Immutability & Alias Management

Once a label's **confidence** crosses the stability threshold (default: 0.90), the **canonical_label** is **frozen**. Future updates to the label are stored as **aliases**.

```
struct LabelRegistry {
    // Map fingerprint -> canonical label
    canonical_labels: Map<PrimitiveFingerprint, Label>,
    // Map fingerprint -> list of aliases (faction-coined, historical, etc.)
    aliases: Map<PrimitiveFingerprint, Vec<Label>>,
}

fn on_label_stabilize(fingerprint: PrimitiveFingerprint, label: Label) -> void {
    if label.confidence >= STABILITY_CONFIDENCE_THRESHOLD (0.90):
        canonical_labels[fingerprint] = label
        label.frozen = true
    
    fn on_new_faction_name_generated(fingerprint: PrimitiveFingerprint, faction_label: Label) -> void:
        if canonical_labels.contains(fingerprint):
            // New faction name becomes an alias
            aliases[fingerprint].push(faction_label)
        else:
            // No canonical yet; add as candidate
            aliases[fingerprint].push(faction_label)
}
```

### 17.5 Convergent Evolution: Structural Naming Independence

**Key principle**: If two independent lineages evolve the same primitive fingerprint, they receive the **same label**. Names are structural (based on the pattern), not genealogical (based on ancestry).

```
// Scenario: Bat-like creatures (lineage A) and Shrews (lineage B) both evolve echolocation.
fingerprint_A = compute_fingerprint(creatures_from_lineage_A)
fingerprint_B = compute_fingerprint(creatures_from_lineage_B)

if fingerprint_A == fingerprint_B:
    // Same fingerprint -> same label
    label = label_registry.get_or_create(fingerprint_A)
    assign_label_to_species(species_A, label)
    assign_label_to_species(species_B, label)
    
    // Both are now "echolocation" despite different ancestry
    // Lore: "The shrews, like the bats, developed a sense of sound."
```

### 17.6 Fallback: Numeric IDs & Provisional Labels

If all three naming phases fail to produce a label (rare edge case), assign a provisional **numeric fingerprint ID**:

```
if not thesaurus.contains(fingerprint) && 
   not compositional_name_acceptable(fingerprint) &&
   not faction_name_available(fingerprint):
    label_text = f"FP-{fingerprint_hash:X8}"
    confidence = 0.1  // Very low confidence; this is a placeholder
    label = Label { text: label_text, confidence, provenance: Thesaurus, first_seen_tick: now }
    
    // Flag for later curation: human designer should review and rename
    CURATION_QUEUE.add(fingerprint, label)
```

---

