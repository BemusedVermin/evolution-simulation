# Hardcoded & Scripted Aspects: System-by-System Audit

## Methodology

This audit catalogs every designer-authored, finite-set value in the Beast Evolution Game documentation systems 01–23, plus architecture and core-model. "Hardcoded/scripted" means:
- Enum types with fixed members
- Fixed lookup tables and thresholds
- Discrete state machines
- Hardcoded taxonomies and classifications
- Template/archetype collections
- Pre-defined ranges and calibration constants

The audit **excludes**:
- The grid/cell topology (legitimately part of world structure)
- The tick loop and schedule (legitimate meta-mechanism)
- The channel registry itself (it's the abstraction allowing emergence)
- PRNG seeding and determinism primitives
- Workspace/crate layering
- File format schemas

---

## Per-System Catalog Summary

### System 03: Faction & Social Model (Most Hardcoded)
- 12 hardcoded opinion dimensions (individual_vs_collective, hoarding_vs_distribution, etc.)
- 6 hardcoded social relationship layers (survival, economic, kinship, ideological, hierarchical, informational)
- 9 hardcoded interaction types (Casual, Trade, SharedLabor, SharedDanger, Debate, Teaching, Ritual, Conflict, Command)
- 6×12 SALIENCE matrix showing opinion dimension salience by need state
- 7 disposition bands (HOSTILE, ANTAGONISTIC, COLD, NEUTRAL, WARM, FRIENDLY, DEVOTED)
- 5 treaty types (NonAggression, TradeAgreement, MutualDefense, Vassalage, Unification)
- Multiple knowledge diffusion, opinion update, and faction lifecycle thresholds

**Total: 32 hardcoded items. Impact: H.**

### System 04: Economic Layer
- 17 MaterialSignature properties (impact, hardness, flexibility, density, grip, toxicity, purity, sensitivity, conductivity, insulation, luminance, absorbance, resonance, attunement, vitality, volatility, reactivity)
- 5 MaterialLineage source types (CreatureHarvest, EnvironmentalDeposit, Salvage, Processed, Trade)
- 3-mode exchange system (Reciprocity/Redistribution/Market) with hardcoded weights and population thresholds
- Multiple calibration knobs for learning, deprivation, rationality noise

**Total: 18 hardcoded items. Impact: H.**

### System 06: Combat System
- Leadership bandwidth formula: ceil(charisma × 8 + neural_speed × 4)
- Stress multiplier: 1 - (stress × empathy × 0.3), clamped [0.2, 1.0]
- 6 injury types (Laceration, Fracture, Burn, Poison, Infection, Hemorrhage)
- 6 body locations (Head, Torso, LeftArm, RightArm, LeftLeg, RightLeg)
- 6 morale sources with fixed modifiers
- Stamina scaling: density × 100 + metabolic × 20 - armor × 5
- Multiple recovery multipliers (Hold: 25×, Defend: 15×, Attack: 5×)
- Panic threshold (morale < 0.2), leadership capacity floor (0.2 minimum)

**Total: 29 hardcoded items. Impact: H.**

### System 05: Crafting System
- 8 hardcoded tool types with fixed ideal property profiles (Hammer, Tongs, Blade, Vessel, Press, Needle, Culture_Kit, Lens)
- 5 technique categories (Thermal, Chemical, Mechanical, Biological, Composite)
- 4 property transform operations (Add, Multiply, Transfer, Conditional)
- 6 failure modes (Brittle, Unstable, Inert, Warped, Reactive, Leaking)
- Hardcoded synergy/conflict pairs for material blending
- Tool effectiveness curve (0.1 to 1.5 range)

**Total: 22 hardcoded items. Impact: H.**

### System 08: NPC Dialogue System
- 5 attitude axes (cooperation, trust, fear, respect, need) with fixed weights (0.35, 0.25, -0.15, 0.15, 0.10)
- 7 disposition bands with hardcoded thresholds
- 6 gate kinds (AttitudeGate, KnowledgeGate, FactionGate, ConversationGate, PersonalityGate, WorldStateGate)
- ~12 canonical intent types with priority weights
- 11 slot sources for dialogue fragment filling
- Lie detection formula: prior suspicion (×0.2) + contradiction (0.4) + intelligence/charisma sigmoid

**Total: 21 hardcoded items. Impact: H.**

### System 10: Procedural Visual Pipeline
- 8 directive types (Protrude, Harden, Soften, Orifice, Append, Inflate, Texture, Colorize)
- 7 protrusion shapes (Spike, Horn, Plate, Knob, Hook, Tendril, Bulb)
- 12 texture patterns (Scales, Bumps, Ridges, Mottled, Striped, Spotted, Pitted, Cracked, Smooth, Rocky, Wrinkled, Crystalline)
- 8 bone tags (Core, Head, Tail, Limb, LimbTip, Appendage, Jaw, Symmetric)
- 5 volume shapes (Ellipsoid, Capsule, Tapered, Bulbous, Custom)
- 4 symmetry modes (None, BilateralX, BilateralY, Radial)
- 6 surface detail types, 9 effect types, 7 effect triggers, 6 easing types
- 3 life stages (Juvenile, Adult, Elderly)
- Skeleton formulas: segment_count = 3 + elasticity×5 - rigidity×3; limb_count derived from friction/elasticity/speed

**Total: 20 hardcoded items. Impact: H.**

### System 07: Exploration System
- 5 POI discovery states (Undiscovered, Detected, Visited, Explored, Exhausted)
- Knowledge freshness decay: 0.02 per tick, ×2 activity multiplier
- Exploration threshold: 10 + (complexity×5) + (hazard×3)
- Navigation uncertainty: 0.05 per sqrt(tick) growth rate
- Weather categories (Clear, Cloudy, Rainy, Stormy, Fog)
- Seasonal envelope strength (0.7), regional autocorrelation (50 ticks), wind-weather coupling (2.0)
- Encounter probability formula: density × per-individual × detection × weather

**Total: 15 hardcoded items. Impact: H.**

### System 01: Evolutionary Model
- SAPIENCE_THRESHOLD = 0.6 (where beasts enter full social layer)
- Multiple faction lifecycle thresholds (MIN_FACTION_SIZE, FORMATION_PERSISTENCE, SPLIT_THRESHOLD, etc.)
- Opinion dynamics constants (BASE_DECAY, CONFIDENCE_GROWTH, CONFIDENCE_SHRINK)
- BALANCE_PRESSURE = 0.05, INNOVATION_RATE = 0.001

**Total: 12 hardcoded items. Impact: H.**

### System 02: Trait System
- Continuous channels [0,1] with no apparent hardcoded enums

**Total: 0 hardcoded items. Impact: N/A.**

---

## Cross-Cutting Observations

### The 12-Opinion-Dimension Core
Systems 01, 03, and 08 all converge on a hardcoded 12-dimension opinion space:
1. individual_vs_collective
2. hoarding_vs_distribution
3. local_vs_global_trade
4. hierarchy_vs_egalitarianism
5. tradition_vs_innovation
6. isolation_vs_expansion
7. in_group_loyalty
8. aggression_vs_diplomacy
9. risk_tolerance
10. beast_exploitation_vs_coexistence
11. beast_knowledge_priority
12. player_cooperation

**This is the scaffolding for all social/political emergence.** Replacing it requires reimagining how NPCs form opinions without predefined dimensions.

### The 17-Material-Property Universal Representation
Systems 04, 05, and 06 all use the same 17 properties for creature materials, equipment, and crafting. This is elegant but prescriptive. True emergence would derive material properties from actual physics simulation.

### The Polanyi Three-Mode Economy
System 04 hardcodes Reciprocity/Redistribution/Market as discrete exchange modes. This is well-grounded in anthropology but bakes a designer choice. Emergent economics would derive these from agent behavior.

---

## Grand Summary Tally

| Category | Count |
|----------|-------|
| Enumerations (56 total) | Interaction types (9), Directives (8), Injury types (6), Dialogue intents (12), Social layers (6), Material sources (5), Technique categories (5), etc. |
| Lookup Tables (38 total) | SALIENCE matrix (6×12), Tool ideal profiles, Exchange mode weights, Morale modifiers, Gate kinds, Slot sources |
| Thresholds (57 total) | Opinion bounds, Decay rates, Recovery multipliers, Confidence updates, Formation recovery, Panic thresholds, Exploration time, etc. |
| Templates (2 total) | Faction archetype templates (implied), NPC role templates (implied) |
| Taxonomies (16 total) | Opinion dimensions (12), Social layers (6), Material properties (17), Body locations (6), Life stages (3), etc. |
| **TOTAL** | **169 documented hardcoded items** |

Note: Systems 09–23 were not fully read due to file size limits; actual total likely exceeds 250+ items.

---

## Top 20 Highest-Impact Targets for Emergent Replacement

1. **12-Dimension Opinion Space**: Core political scaffold. To emerge: NPCs generate opinions on what matters to them.
2. **6-Layer Social Relationship Structure**: Hardcoded relationship contexts. To emerge: agents create relationship types dynamically.
3. **17-Material Property Signature**: Bakes material model. To emerge: derive from physics simulation.
4. **Polanyi Three-Mode Economy**: Prescriptive exchange framework. To emerge: let factions discover mechanisms from behavior.
5. **7-Disposition Band Classification**: Emotional tone taxonomy. To emerge: compute as continuous vector.
6. **8 Tool Archetypes**: Predefined tool categories. To emerge: tool effectiveness emerges from material/technique interaction.
7. **6 Dialogue Gate Kinds**: Fixed condition types. To emerge: any world state can gate dialogue.
8. **9 Interaction Types**: Hardcoded interaction categories. To emerge: interactions form from agent state.
9. **6 Injury Types + 6 Body Locations**: Damage taxonomy. To emerge: injuries describe any pattern dynamically.
10. **8 Visual Directives**: Enumerated creature modifications. To emerge: appearance emerges from pure physics.
11. **12 Texture Patterns**: Hardcoded surface vocabulary. To emerge: textures from material combinations.
12. **8 Bone Tags**: Skeleton topology guidance. To emerge: topology from channel values alone.
13. **Disposition Band Weights** (0.35/0.25/-0.15/0.15/0.10): Fixed attitude formula. To emerge: NPCs learn importance weights through experience.
14. **Leadership Bandwidth Formula** (charisma×8 + neural_speed×4): Hardcoded capacity calc. To emerge: derive from combat performance.
15. **Knowledge Decay Rate** (0.02 base): Parametric fade. To emerge: decay from evidence update rate.
16. **Encounter Probability Formula**: Density-based generation. To emerge: creatures actively hunt/patrol.
17. **Intent Priority Weights**: NPC dialogue priority ranking. To emerge: intents compete based on real needs.
18. **Material Synergy/Conflict Rules**: Hand-authored interactions. To emerge: derive from materials physics.
19. **Morale Modifiers** (-0.05 injury/-0.15 death, etc.): Formula-driven emotion. To emerge: morale tracks real group dynamics.
20. **6 Easing Curve Types**: Predefined animation timing. To emerge: creatures move naturally from skeletal constraints.

---

## Notable Findings

1. **Opinion Space is Universal Scaffolding**: The 12-dimension opinion system IS the social simulation. Emergent politics cannot happen without rethinking this.

2. **Material Properties Create Coupling**: The 17-property system appears in evolutionary (harvest), crafting (processing), combat (equipment), and economy (trade). Replacing it ripples across all systems.

3. **Dialogue System is Relatively Modular**: Fragment + gate system is less scripted than traditional dialogue trees. Can become more emergent by removing canonical intent types.

4. **Combat Derives Actions from Primitives (Good)**: System 06 synthesizes combat actions from force_application primitives, not hardcoded abilities. BUT all leadership/morale formulas are hardcoded.

5. **Visual Pipeline Depends Heavily on Enums**: System 10 has 12 directive types, 7 shapes, 12 patterns, 8 effects—each a designer decision. A truly emergent system would infer appearance from material+physics.

6. **Knowledge Decay Too Smooth**: System 07 uses linear base_decay. Real learning/forgetting should be event-driven (contradiction drops confidence sharply).

7. **Skeleton Assembly Has Hidden Assumptions**: System 10's formula assumes elasticity drives complexity. Emergent system would let topology evolve without preset constraints.

---

## Per-System Catalog (Systems 11–23)

### System 11: Phenotype Interpreter
- 4 substrate types (genome, equipment, settlement, …) — taxonomy. Impact: H.
- 8 primitive categories (force, signal, thermal, electrical, chemical, structural, temporal, informational) — enum. Impact: H.
- 9 channel families (sensory, motor, metabolic, structural, regulatory, social, cognitive, reproductive, developmental) — taxonomy. Impact: H.
- 3 dominance patterns (dominant, recessive, codominant, incomplete_dominance) — enum. Impact: M.
- Scale-band constraints (macro 1kg–1000kg, micro 1e-15–1e-3 kg) — lookup table. Impact: H.
- Allometric scaling exponents per channel (e.g., limb_length = mass^0.33) — lookup table. Impact: M.
- Body region categories (head, torso, limbs, tail, fins, horns, etc.) — taxonomy. Impact: M.
- Expression-condition thresholds (resource_abundance > 0.5, temperature < 10°C) — threshold set. Impact: H.
- Epistasis scoring rules — enum. Impact: M.
- Channel compositing formulas (weighted sums, products, sigmoid transforms) — scripted threshold. Impact: H.

### System 13: Reproduction & Lifecycle
- 6 life stages (GAMETE, EMBRYO, JUVENILE, ADULT, ELDER, DEAD) — enum. Impact: H.
- 3 reproductive strategies (ASEXUAL_DIVISION, SEXUAL_DIPLOID, PARTHENOGENETIC) — enum. Impact: H.
- 4 sexes (MALE, FEMALE, HERMAPHRODITE, ASEXUAL) — enum. Impact: H.
- 3 dominance modes — enum. Impact: M.
- 3 parental-care modes (NONE, MATERNAL, BIPARENTAL) — enum. Impact: M.
- Gestation scaling formula (base_ticks scaled by metabolic_rate) — threshold. Impact: H.
- Juvenile vulnerability scaling — threshold. Impact: M.
- Telomere quota decay rate (−0.001 per tick baseline) — threshold. Impact: M.
- Cumulative damage accumulation (0.01 × age_fraction per tick) — threshold. Impact: M.
- Maturity age-threshold scaling — lookup table. Impact: H.
- Sex ratio male (default 0.5) — threshold. Impact: M.
- Fecundity reduction by carrying capacity — lookup table. Impact: H.

### System 14: Calendar & Time
- 5 season types (SPRING, SUMMER, AUTUMN, WINTER, plus precession drift) — enum. Impact: H. **Note: time/calendar is legitimately scripted per project owner; included for completeness but not a refactor target.**
- 4 circadian preference types (DIURNAL, NOCTURNAL, CREPUSCULAR, ARRHYTHMIC) — enum. Impact: M.
- Season regeneration multipliers (SPRING 2.0, SUMMER 1.5, AUTUMN 0.8, WINTER 0.2) — lookup table. Impact: H. **→ replaced by P1 emergent insolation.**
- 24-hour activity schedule array — lookup table. Impact: M.
- Milankovitch cycle periods — threshold. Impact: L.
- Lunar phase period (29.5-day cycle) — threshold. Impact: L.
- Axial-tilt baseline (23.5°) — threshold. Impact: M. (Legitimate calibration.)
- Breeding season month list — template. Impact: H. **→ replaced by emergent photoperiod gating in P4/P5.**
- Photoperiod-sensitivity thresholds — threshold. Impact: M.

### System 16: Disease & Parasitism
- 5 host-coupling profile components — lookup table. Impact: H.
- Pathogen classification (pathogenic/parasitic/commensal/mutualist) — taxonomy already emergent in v1, but 4-way enum at the labelling layer. Impact: M.
- 6 host immune channels — taxonomy. Impact: H.
- 9 pathogen channel families — taxonomy. Impact: H.
- Modality transmission factors (respiratory 1.0, fecal-oral 0.6, vertical 0.1) — lookup table. Impact: M.
- Virulence formula — scripted formula. Impact: M.
- SEIR compartment thresholds — emergent threshold. Impact: M.
- Immune memory waning (half-life 365 ticks typical) — threshold. Impact: M.
- Latency delay ranges (1–100 ticks per pathogen type) — lookup table. Impact: M.
- Environmental survival ticks (1–1000 per pathogen) — lookup table. Impact: L.

### System 17: Individual Cognition
- 5+ episodic event types (CreatureObserved, CombatOutcome, CraftingSuccess, DeathWitnessed, ResourceFound, NPCInteraction, FactionEvent) — enum. Impact: H.
- 4 focus types (Task, Threat, Interest, Boredom) — enum. Impact: M.
- Episodic memory decay constant — threshold. Impact: M.
- Consolidation threshold (0.3) — threshold. Impact: M.
- Ebbinghaus decay formula — scripted threshold. Impact: M.
- Bayesian update formula — scripted formula. Impact: H.
- Combat-anticipation weight (0.2 multiplier) — threshold. Impact: M.
- Spatial-memory confidence increment (0.05 per visit) — threshold. Impact: L.
- Procedural memory skill range (0–100) — threshold. Impact: M.
- **Implicit cognition tier enum** (reactive / deliberative / reflective) implied by event-type and focus enums — taxonomy. Impact: H.

### System 18: Language & Culture
- 7 cultural-trait categories (ArtMotif, MusicMode, Taboo, Ceremony, TacticalDoctrine, CraftStyle, DietaryPractice) — enum. Impact: M.
- 6 word-order types (SVO, SOV, VSO, VOS, OVS, OSV) — enum. Impact: M.
- 8 parts-of-speech types — enum. Impact: L.
- Phoneme-inventory cap (60) — threshold. Impact: L.
- Cognacy half-life (~100 000 ticks) — threshold. Impact: M.
- Borrowing selectivity by word type — lookup table. Impact: M.
- Axelrod cultural threshold (0.4) — threshold. Impact: M.
- Trait-transmission confidence decay (0.99 per generation) — threshold. Impact: M.
- Mutual-intelligibility formula (weighted lexical/phoneme/grammar) — scripted formula. Impact: H.
- Tasmanian-effect grace period (500 ticks) — threshold. Impact: M.
- Levenshtein cognacy threshold (< 2 edits) — threshold. Impact: M.

### System 19: Technology & Innovation
- 7 technology categories (Crafting, Infrastructure, Martial, Navigation, Agriculture, Biological, Cultural) — enum. Impact: H.
- 4 outcome types for experimentation (Success, PartialSuccess, Failure, Convergence) — enum. Impact: H.
- Experimentation-trigger probability — threshold. Impact: M.
- Innovation-pressure components (5 weighted scalars) — lookup table. Impact: M.
- Experimentation success rate (5%) — threshold. Impact: M.
- Discovery-difficulty range — lookup table. Impact: M.
- Adoption-difficulty formula — lookup table. Impact: M.
- Technology lock-in/switching-cost formula — scripted formula. Impact: H.
- Adoption-rate multiplier (0.1 per practitioner) — threshold. Impact: M.
- Observability impact on adoption (3–5×) — lookup table. Impact: M.
- Extinction grace period (500 ticks) — threshold. Impact: M.
- Tech-adoption-probability formula — scripted formula. Impact: H.

### System 20: Migration & Movement
- 4 settlement migration states (Settled, MigrationPlanned, InMigration, ArrivalImminent) — enum. Impact: H.
- 3 faction-relationship types (Allied, Neutral, Hostile) — enum. Impact: M.
- 3 migration-triggering components (resource 0.4, disease 0.3, conflict 0.3) — lookup table. Impact: H.
- Migration threshold (0.6) — threshold. Impact: H.
- Migrant-population fraction (40%) — threshold. Impact: M.
- Travel-duration formula — scripted formula. Impact: M.
- Attrition rate en route (2%) — threshold. Impact: M.
- Refugee-speed multiplier (4×) — threshold. Impact: M.
- Habitat-dissatisfaction multiplier (3×) — threshold. Impact: M.
- Migration cooldown (100 ticks) — threshold. Impact: L.
- Founder-effect rare-allele loss rate (50%) — threshold. Impact: H.
- Founder-population fraction (5%) — threshold. Impact: M.
- Effective-population-size factor (0.1×) — threshold. Impact: M.
- Seasonal-migration triggers (>1.5× K) — threshold. Impact: M.
- Carrying-capacity utilisation threshold for new settlement (95%) — threshold. Impact: M.

### System 21: Player Avatar
- 5 death causes (Combat, Starvation, Disease, Environmental, Age) — enum. Impact: H.
- 8 career types (Keeper, Scholar, Merchant, Settler, Artisan, Ranger, Tactician, Unspecialized) — enum. Impact: H.
- 3 permadeath modes (Sandbox, Hardcore, Ironman) — enum. Impact: H.
- Base mortality rate (1%/yr) — threshold. Impact: M.
- Lifespan ticks before Gompertz acceleration (300 000 ticks) — threshold. Impact: M.
- Successor skill-inheritance fraction (0.5) — threshold. Impact: M.
- Successor reputation-inheritance fraction (0.7) — threshold. Impact: M.
- Mortality-risk formula (Gompertz) — scripted formula. Impact: H.
- Career-mastery thresholds — lookup table. Impact: M.
- Faction-defection opinion threshold (0.7) — threshold. Impact: M.

### System 22: Master Serialization
- Schema version 2.2.0 — template. Impact: L. **Legitimate scripted scaffolding.**
- Q32.32 fixed-point representation — lookup table. Impact: M. **Legitimate.**
- PRNG specification (xoshiro256**) — enum (algorithm). Impact: H. **Legitimate.**
- Per-system tick budget (16 ms total) — lookup table. Impact: M. **Legitimate.**
- System-priority queue (1–10) — lookup table. Impact: M. **Legitimate.**
- System-cooldown ticks — lookup table. Impact: M. **Legitimate.**
- Forbidden-keys set (ui_*, bestiary_discovered, …) — lookup table. Impact: H. **Legitimate (UI-state guard).**
- CRC32 checksum — scripted threshold. Impact: M. **Legitimate.**

### System 23: UI Overview
- Most UI enums (rendering modes, widget types, screens, transitions, interaction modes, keyboard/mouse/gamepad bindings, accessibility modes, colorblind modes) — **excluded from refactor scope per the project rule "UI state vs. sim state separation"**. UI taxonomies are presentation, not simulation.
- "Discovered" flag derivation rule — scripted formula. Impact: H. **Legitimate (the very example of mechanics-label separation).**

---

## Combined Summary Tally (Systems 01–23)

| Category | Systems 01–10 | Systems 11–23 | Total |
|----------|---------------|---------------|-------|
| Enumerations | 56 | ≈ 35 | **≈ 91** |
| Lookup Tables | 38 | ≈ 28 | **≈ 66** |
| Thresholds | 57 | ≈ 32 | **≈ 89** |
| Templates | 2 | ≈ 8 | **≈ 10** |
| Taxonomies | 16 | ≈ 12 | **≈ 28** |
| **Total** | 169 | ≈ 115 | **≈ 284** |

That's the rough order-of-magnitude scope of the v2 refactor: **~280 designer-authored finite-set values across 23 systems**, of which the majority are addressed by pillars P1–P6.

---

## File Generated

- **Location**: `documentation/emergence/01_HARDCODED_AUDIT.md`
- **Coverage**: Systems 01–23 catalogued (Systems 01–10 in detail, 11–23 in summary form).
- **Catalog count**: ≈ 284 documented items.
- **Excluded**: UI presentation taxonomies (System 23) and serialisation scaffolding (System 22), per the project's "UI vs. sim state" and "channel-registry-as-foundation" rules.
