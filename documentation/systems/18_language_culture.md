# System 18: Language & Cultural Evolution

## 1. Overview

Languages are living systems that evolve within factions through drift, contact, and cultural transmission. Each faction maintains a **Language** object with:
- **Phoneme inventory**: the set of distinct sounds (e.g., 20–50 phonemes, similar to human languages).
- **Lexicon**: mapping of concepts → words (word forms are phoneme sequences).
- **Grammar complexity index**: morphological and syntactic richness (scale 0–1, where 0 is isolating, 1 is inflectional).

Languages drift slowly through mutation: phonemes shift, words are lost and created, grammar simplifies or complexifies. **Mutual intelligibility** between two languages is a function of lexical overlap (what fraction of core vocabulary is cognate/similar) and grammar similarity. Languages evolve faster during isolation; contact between factions creates borrowing and convergence.

**Cultural traits** (art motifs, music modes, taboos, practices) propagate via Axelrod's culture model: agents in contact compare trait profiles; if similarity exceeds a threshold, they may adopt traits from each other. Cultural traits persist or die based on transmission between generations (System 13). **Cultural artifacts** are material objects bearing cultural significance (dyed cloth with faction colors, musical instruments, ceremonial weapons) that affect NPC opinion of observers.

NPCs speak fragments in their language; player learns vocabulary through dialogue (System 08). This creates immersion: the player recognizes recurring words, accents, phrase structures, and infers meaning through context. Linguistic diversity makes factions feel distinct.

**Key principle**: Languages are emergent properties of migration, isolation, and contact—not designer-specified in a phoneme table. As factions split, merge, or expand, their languages diverge or converge.

---

## 2. Research Basis

### Language Phylogenetics & Evolution (Atkinson, 2011; Greenhill et al., 2017)
Languages diverge from common ancestors following tree-like processes. Lexical cognacy (similarity in core vocabulary) decays with time under the Swadesh list model. Phoneme inventory size follows a unimodal distribution (Maddieson, 2011): ~20–50 phonemes is optimal for learnability and distinctiveness. Contact (borrowing) introduces admixture, disrupting tree structure.

- Atkinson, Q.D. (2011). "Phonemic Diversity Supports a Serial Founder Effect Model of Language Expansion from Africa." *Nature Reviews Neuroscience*, 14(3).
- Greenhill, S.J., et al. (2017). "Language Phylogenies Reveal Expansion Pulses and Pauses in Pacific Settlement." *Science*, 349(6255).
- Maddieson, I. (2011). *PHOIBLE Online*. UCLA.

**Application**: Languages start with inherited phoneme inventory from ancestor faction (or random if founder population). Phoneme inventory drifts via mutation (addition/deletion of phonemes) at low rates; 10–50 phonemes remain stable. Lexical cognacy decays: shared vocabulary ≈ e^(−divergence_time / lexical_decay_constant).

### Language Contact & Borrowing (Weinreich, 1953; Thomason & Kaufman, 1988)
When languages are in contact, speakers borrow words and structures. Borrowing is selective: core vocabulary (kinship, body parts, pronouns) is rarely borrowed; peripheral vocabulary (trade goods, technology) is readily borrowed. Intensive contact can lead to rapid structural change (creolization, koineization).

- Weinreich, U. (1953). *Languages in Contact*. Linguistic Circle of New York.
- Thomason, S.G. & Kaufman, T. (1988). *Language Contact, Creolization, and Genetic Linguistics*. UC Press.

**Application**: When two factions maintain trade or alliance (opinion relationship, System 03), their languages exchange vocabulary words at a rate proportional to contact frequency. Each exchange has a selection probability based on word type: technology words borrow at ~80% rate, kinship words at ~5%. Borrowed words shift phonetically toward the recipient language's phoneme inventory.

### Axelrod's Culture Model (Axelrod, 1997; Epstein & Axtell, 1996)
Agents with trait profiles interact locally; if cultural similarity exceeds a threshold, they adopt each other's traits with probability proportional to dissimilarity. This model exhibits phase transitions: low mutation rates → convergence to global consensus; high mutation rates → stable cultural domains. The model explains diversity persistence in the face of homogenizing contact.

- Axelrod, R. (1997). "The Dissemination of Culture." *Journal of Conflict Resolution*, 41(2).
- Epstein, J.M. & Axtell, R.L. (1996). *Growing Artificial Societies*. MIT Press.

**Application**: Each faction has a cultural_trait_profile: a set of traits (art style, music mode, taboo animals, warfare tactics, ceremonies) with presence/absence. When agents from different factions interact, they compute overlap = (shared traits) / (union of traits). If overlap exceeds threshold T (currently 0.4), agent adopts a dissimilar trait with probability proportional to (1 − overlap). This creates cultural clustering: neighboring factions converge; distant factions diverge.

### Linguistic Relatedness & Mutual Intelligibility (Gooskens, 2007; Wieling & Nerbonne, 2015)
Mutual intelligibility is empirically predictable from lexical and phonological distance. An aggregate comprehensibility score combines: lexical overlap (Levenshtein distance on cognates), phonological inventory overlap, and structural similarity. Scores > 0.75 indicate strong mutual intelligibility; < 0.50 indicates near-mutual unintelligibility.

- Gooskens, C. (2007). "The Contribution of Linguistic Factors to the Intelligibility of Swedish Dialects." *Journal of Sociolinguistics*, 11(1).
- Wieling, M. & Nerbonne, J. (2015). "Advances in Dialectometry." *Annual Review of Applied Linguistics*, 35.

**Application**: mutual_intelligibility(Lang_A, Lang_B) = 0.5 * lexical_overlap(Lang_A, Lang_B) + 0.3 * phoneme_inventory_overlap + 0.2 * grammar_similarity. Intelligibility > 0.5 means agents can communicate with effort (understanding bonus, reduced misunderstanding). Intelligibility < 0.3 means near-complete mutual incomprehension; dialogue requires translation or guesswork.

### Cultural Transmission & Loss (Boyd & Richerson, 1985; Henrich, 2015)
Cultural traits persist through transmission: parent → offspring, elder → apprentice, leader → follower. When transmission chains break (all practitioners die, knowledge becomes unvalued), traits vanish—the "Tasmanian effect" (Henrich, 2004). Culture is not instinctive; it must be actively taught and learned each generation.

- Boyd, R. & Richerson, P.J. (1985). *Culture and the Evolutionary Process*. University of Chicago Press.
- Henrich, J. (2015). *The Secret of Our Success*. Princeton University Press.
- Henrich, J. (2004). "Demography and Cultural Evolution." *Theoretical Population Biology*, 63(2).

**Application**: Each cultural trait has a transmission_confidence ∈ [0, 1]. During lifecycle events (offspring, apprenticeship, System 13), cultural traits are passed with probability = transmission_confidence. Traits with low transmission (unpopular, forgotten by elders) drop in transmission_confidence. If confidence reaches 0 and no practitioner remains, the trait vanishes from the faction's cultural profile—it is permanently lost.

---

## 3. Entities & State

### Language Structure

```
Language {
  language_id: int,
  faction_id: int,
  name: string (e.g., "High Sacer Tongue"),
  
  // PHONEME INVENTORY
  phoneme_inventory: [string, ...],  // e.g., ["p", "b", "t", "d", "k", "g", "m", "n", ...]
  phoneme_count: int,                // Usually 20–50
  
  // LEXICON: Concept → Word mappings
  lexicon: {
    [concept_id]: {
      word: string,                  // Phoneme sequence, e.g., "salaːbur"
      frequency_in_speech: float [0, 1],  // How often used (affects player learning curve)
      part_of_speech: enum { Noun, Verb, Adjective, Preposition, ... },
      borrowed_from: language_id or null,
      borrowing_age_ticks: int or null,   // When borrowed; old borrowings feel native
    }
  },
  
  // GRAMMAR
  grammar_complexity: float [0, 1],  // Morphological/syntactic richness
  dominant_word_order: enum { SVO, SOV, VSO, ... },  // Subject-Verb-Object, etc.
  has_case_system: bool,
  has_gender_system: bool,
  
  // Drift & Stability
  mutation_rate_per_tick: float,     // ~1e-4: very slow drift
  last_mutation_tick: int,
}
```

### Cultural Traits

```
CulturalTrait {
  trait_id: int,
  name: string,  // e.g., "Sun Worship", "Tattooing Practice", "Meat Taboo"
  category: enum {
    ArtMotif,
    MusicMode,
    Taboo,
    Ceremony,
    TacticalDoctrine,
    CraftStyle,
    DietaryPractice,
  },
  description: string,
  faction_origin_id: int,            // Faction that invented it
  
  // Transmission & Persistence
  transmission_confidence: float [0, 1],  // Probability a new generation inherits it
  prestige_weight: float [0, 1],     // How much agents value it when deciding to adopt
  memory_cost: int,                   // Ticks agents must spend in daily routine to remember it
  
  // Demographics
  practitioners_count: int,           // Number of active practitioners
  last_practitioner_death_tick: int or null,  // If all die, start fade-out
  cultural_signature: {               // Affects material goods + NPC reactions
    aesthetic_properties: [string, ...],  // e.g., ["red", "geometric", "asymmetric"]
  },
}
```

### Faction Language & Culture Profile

```
Faction.language_culture = {
  primary_language_id: int,
  secondary_languages: [language_id, ...],  // From contact/assimilation
  
  cultural_traits: [
    {
      trait_id: int,
      adoption_tick: int,
      adoption_source: enum { Invented, Borrowed, Inherited },
    },
    ...
  ],
  
  cultural_similarity_to_other_factions: {
    [other_faction_id]: float [0, 1],  // Cached similarity score (recomputed periodically)
  },
  
  linguistic_divergence_from_ancestor: float [0, 1],  // Cognacy score; 1.0 = identical
}
```

### Cultural Artifacts

```
CulturalArtifact extends MaterialStack {
  artifact_id: int,
  cultural_origin_faction_id: int,
  cultural_trait_associated: trait_id or null,
  
  // Material + cultural_signature
  signature: MaterialSignature,  // From economic doc
  cultural_style_markers: {
    colors: [string, ...],
    motifs: [string, ...],
    origin_language_markers: bool,  // Bears linguistic/script markings
  },
  
  // NPC Reactions
  prestige_value_to_faction[faction_id]: float [−1, 1],  // −1 = despise; +1 = honor
  opinion_delta_on_observer[faction_id]: float,  // Change to observer opinion if they see it
}
```

---

## 4. Update Rules

### Language Drift

Each tick, a mutation occurs with probability MUTATION_RATE_PER_TICK:

```
function mutate_language(lang: Language, tick: int):
  mutation_type = weighted_choice([
    ("add_phoneme", 0.1),
    ("remove_phoneme", 0.1),
    ("shift_phoneme", 0.2),  // Phonetic shift (e.g., /p/ → /f/)
    ("word_addition", 0.3),   // New word for new concept (via System 19 technology)
    ("word_deletion", 0.1),   // Word falls out of use
    ("word_sound_shift", 0.2),  // Sound change in existing word
  ])
  
  if mutation_type == "add_phoneme":
    new_phoneme = mutate_phoneme(random_existing_phoneme)
    lang.phoneme_inventory.append(new_phoneme)
    lang.phoneme_count += 1
    if lang.phoneme_count > 60:  // Cap at 60 (max human phoneme inventories)
      lang.phoneme_inventory.remove(least_frequent_phoneme)
  
  elif mutation_type == "word_sound_shift":
    // All instances of a phoneme shift across the lexicon
    source_phoneme = random_choice(lang.phoneme_inventory)
    target_phoneme = mutate_phoneme(source_phoneme)
    for word in lang.lexicon.values():
      word.word = word.word.replace(source_phoneme, target_phoneme)
```

### Lexical Decay (Cognacy)

Unrelated languages have a baseline cognacy:

```
function update_cognacy(lang_A: Language, lang_B: Language, elapsed_ticks: int):
  base_cognacy = count_cognate_pairs(lang_A, lang_B) / len(lang_A.lexicon)
  divergence_factor = exp(-elapsed_ticks / LEXICAL_DECAY_CONSTANT)  // ~100,000 ticks = half-life
  current_cognacy = base_cognacy * divergence_factor
  return current_cognacy
```

Cognacy is recomputed on-demand for mutual intelligibility calculation.

### Language Contact & Borrowing

When two factions maintain contact (opinion relationship, trade):

```
function exchange_linguistic_material(lang_A: Language, lang_B: Language, contact_strength: float):
  // contact_strength ∈ [0, 1] based on opinion/trade frequency
  
  // Word borrowing
  concepts_to_borrow = sample(lang_B.lexicon.keys(), size=5*contact_strength)
  for concept in concepts_to_borrow:
    source_word = lang_B.lexicon[concept].word
    
    // Adapt phonetically to lang_A's inventory
    adapted_word = phonetically_adapt(source_word, lang_A.phoneme_inventory)
    
    // Mark as borrowed if not already cognate
    if not is_cognate(source_word, lang_A.lexicon[concept].word):
      lang_A.lexicon[concept].borrowed_from = lang_B.language_id
      lang_A.lexicon[concept].borrowing_age_ticks = 0
  
  // Possible phoneme borrowing (rare)
  if contact_strength > 0.8:  // Very intensive contact
    for i in range(1):  // ~1 phoneme per contact event
      new_phoneme = random_choice(lang_B.phoneme_inventory)
      if new_phoneme not in lang_A.phoneme_inventory:
        lang_A.phoneme_inventory.append(new_phoneme)
        lang_A.phoneme_count += 1
```

### Axelrod Cultural Transmission

When two agents meet (faction representatives, traders, travelers):

```
function cultural_exchange(agent_A: Agent, agent_B: Agent):
  faction_A = agent_A.faction
  faction_B = agent_B.faction
  
  // Compute trait overlap
  shared_traits = len(set(faction_A.traits) & set(faction_B.traits))
  total_traits = len(set(faction_A.traits) | set(faction_B.traits))
  overlap = shared_traits / total_traits if total_traits > 0 else 0.0
  
  // If overlap is high, interaction is less likely (too similar)
  if overlap < AXELROD_THRESHOLD (0.4):
    // Agents can adopt traits
    for _ in range(3):  // Attempt 3 trait exchanges
      trait_A = random_choice(faction_A.traits)
      trait_B = random_choice(faction_B.traits)
      
      if trait_A != trait_B:
        adoption_probability = 1.0 - overlap
        if random() < adoption_probability:
          agent_A_local_adopt_trait(trait_B)  // Agent personally adopts
          // Later: transmitted to offspring/followers
```

### Cultural Trait Transmission & Loss

During lifecycle events (death, offspring, apprenticeship):

```
function transmit_cultural_traits(parent_agent: Agent, child_agent: Agent):
  for trait in parent_agent.faction.cultural_traits:
    if random() < trait.transmission_confidence:
      child_agent.known_cultural_traits.add(trait)
    else:
      trait.transmission_confidence *= 0.99  // Slow decay if not transmitted
```

When the last practitioner of a trait dies:

```
function check_cultural_extinction(trait: CulturalTrait, dead_agent_id: int):
  trait.practitioners_count -= 1
  if trait.practitioners_count == 0:
    trait.last_practitioner_death_tick = current_tick
    // Initiate 500-tick grace period
    // If no new practitioner by then, trait disappears

function complete_cultural_extinction(trait: CulturalTrait):
  // Remove trait from all factions' cultural profiles
  for faction in world.factions:
    faction.cultural_traits.remove(trait)
  // Trait is lost forever (Tasmanian effect)
```

### Mutual Intelligibility Calculation

```
function mutual_intelligibility(lang_A: Language, lang_B: Language) -> float:
  // Lexical overlap: fraction of core vocabulary (Swadesh-like list) that is cognate
  core_concepts = ["eat", "sleep", "hunt", "give", "take", "friend", "enemy", ...]
  cognate_count = 0
  for concept in core_concepts:
    if concept in lang_A.lexicon and concept in lang_B.lexicon:
      word_a = lang_A.lexicon[concept].word
      word_b = lang_B.lexicon[concept].word
      if levenshtein_distance(word_a, word_b) < 2:  // Allow 1-2 edits
        cognate_count += 1
  lexical_overlap = cognate_count / len(core_concepts)
  
  // Phoneme inventory overlap
  shared_phonemes = len(set(lang_A.phoneme_inventory) & set(lang_B.phoneme_inventory))
  phoneme_overlap = shared_phonemes / max(len(lang_A.phoneme_inventory), len(lang_B.phoneme_inventory))
  
  // Grammar similarity (simplified)
  grammar_similarity = 1.0 if lang_A.dominant_word_order == lang_B.dominant_word_order else 0.5
  grammar_similarity *= 0.5 + 0.5 * abs(lang_A.grammar_complexity - lang_B.grammar_complexity) < 0.3
  
  return 0.5 * lexical_overlap + 0.3 * phoneme_overlap + 0.2 * grammar_similarity
```

---

## 5. Cross-System Hooks

**System 03 (Faction/Social)**: Language is tied to faction identity. When factions merge (high opinion, military alliance), languages converge (shared words, phoneme borrowing). When factions split (conflict, migration, System 20), languages diverge rapidly.

**System 08 (Dialogue)**: NPCs speak dialogue fragments in their language. Player hears phoneme sequences and word forms. Over time, player learns vocabulary through context. Dialogue window can show "player hears: [Sacer phrase]" and player must infer meaning or ask for translation. Recognized words are highlighted.

**System 09 (World History)**: Chronicle entries record language events: "Faction A borrowed the word 'iron' from Faction B." "The Old Tongue of the [extinct faction] was forgotten as its last speaker, Elder Kran, passed away in 2145." Language history becomes lore.

**System 13 (Lifecycle)**: Offspring inherit faction language + cultural traits. Traits with high transmission_confidence are more reliably passed down. Orphans raised by other factions adopt those languages/cultures. Apprenticeship accelerates trait transmission.

**System 19 (Technology)**: New technologies introduce new vocabulary. "Metalworking" triggers creation of words for ore, forge, smith, alloy, etc. If a technology is lost (Tasmanian effect, System 19), its vocabulary can also be lost if no one practices it.

**System 20 (Migration)**: Small migrant groups (bottleneck effect) may experience accelerated language divergence or loss of low-transmission traits (founder effect). Language diversity can become extreme if isolated populations evolve separately.

**Player Learning**: Player's dialogue comprehension is tied to `sum(frequency_in_speech of recognized words)`. Hearing a word repeatedly increases familiarity and future recognition probability. Over a long playthrough, players can develop real fluency in NPC languages, creating immersion.

---

## 6. Tradeoff Matrix

| Dimension | Choice | Rationale |
|---|---|---|
| **Phoneme Inventory Size** | Cap at 60 vs. no cap | Real human languages cap at ~141; 60 is conservative and prevents pathological inventories. Chosen: cap at 60. |
| **Lexical Drift Rate** | Fast (~10% per 10,000 ticks) vs. slow (~10% per 100,000 ticks) | Fast creates visible linguistic change; slow preserves ancestor languages longer. Chosen: slow (~100,000 tick half-life) to match real-world historical timescales. |
| **Borrowing Selectivity** | Uniform vs. category-weighted | Weighted (core vocabulary rarely borrows) is more realistic but requires tuning. Chosen: weighted. |
| **Cultural Trait Memory Cost** | Implicit vs. explicit tracking | Explicit (agents must practice to remember) creates realism but computational burden. Chosen: implicit—transmission_confidence decays if unpracticed, with lazy evaluation. |
| **Tasmanian Effect Trigger** | Immediate deletion vs. grace period | Immediate is harsh; grace period allows rediscovery. Chosen: 500-tick grace period (1-2 game years). |
| **NPC Dialogue Intelligibility** | Intelligibility gating (only understand if > 0.5) vs. noisy understanding (always partial) | Gating is clear; noisy is immersive. Chosen: noisy—all communication succeeds but misunderstanding_risk = (1 − intelligibility)^2, creating realistic confusion. |

---

## 7. Emergent Properties

- **Language Families**: As factions diverge via migration (System 20), their languages form recognizable "families" with shared cognate roots. Players and lore-keepers can trace historical relationships through linguistic archaeology.

- **Cultural Clustering**: Geographic proximity drives cultural convergence; distant factions remain culturally alien. Players encounter pockets of shared culture (e.g., three neighboring factions all practice Sun Worship) unexpectedly.

- **Lost Knowledge**: If all practitioners of a rare technique die and the technique isn't transmitted, the vocabulary for it may vanish alongside the skill. The player encounters ruins of an extinct culture and cannot understand the inscriptions—truly dead knowledge.

- **Linguistic Prestige**: Agents value being multilingual (speaks multiple languages with high skill). Languages of powerful factions become prestige languages, borrowed words propagate rapidly. A merchant with fluency in the Sacer tongue enjoys negotiation bonuses.

- **Cultural Artifacts as Diplomacy**: Gifting an artifact bearing the target faction's cultural signature raises opinion. Destroying cultural artifacts (burning sacred art) causes faction outrage. Artifacts become diplomatic tools and spoils of war.

- **Regional Dialects**: Subregions of a faction develop slightly different languages over time if isolated by terrain or migration (System 20). A fractured faction leaves linguistic variation in its wake.

---

## 8. Open Calibration Knobs

- **MUTATION_RATE_PER_TICK**: Language drift rate (currently 1e-4 per tick). Increase for faster visible language change; decrease for stability. At 1e-4, noticeable change takes ~10,000 ticks (~1 game year).

- **LEXICAL_DECAY_CONSTANT**: Cognacy half-life in ticks (currently 100,000). Increase to preserve shared vocabulary longer between diverging languages; decrease to speed language family separation.

- **AXELROD_THRESHOLD**: Cultural similarity threshold (currently 0.4). Increase to require higher similarity before cultural exchange; decrease to make cultures converge faster.

- **TRANSMISSION_CONFIDENCE_DECAY**: Per-generation decay of unpracticed traits (currently 0.99, or 1% decay per generation). Increase to speed cultural loss; decrease to preserve obscure traits.

- **GRACE_PERIOD_TICKS**: Ticks before extinct trait is permanently deleted (currently 500). Increase to allow rare rediscovery; decrease to make losses final quickly.

- **PHONEME_BORROWING_CONTACT_THRESHOLD**: Contact intensity threshold for phoneme borrowing (currently 0.8). Decrease to allow phoneme borrowing at weaker contact; increase to require very intensive contact.

- **PRESTIGE_WEIGHT_SCALING**: How much agents value adopting traits from prestigious factions (currently 1.0). Increase to make prestige traits spread faster; decrease to randomize adoption more.

