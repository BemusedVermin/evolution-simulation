# System 21: Player Avatar & Embedded Agency

## 1. Overview

The Player Avatar is a special **Agent** (AgentMind) that the human player controls. It is fully embedded in the simulation: it ages, hungers, accumulates opinions, learns skills, breeds, and dies. The player is not a god-mode omniscient observer but a mortal actor subject to the same rules as NPCs.

**Core avatar components**:
1. **Life & Death**: Avatar ages (System 13 lifecycle), can take damage and die. Permadeath option: game ends. Successor mode: player respawns in same faction/world with a new avatar from the same lineage/faction.
2. **Reputation**: Avatar's witnessed actions trigger opinion updates in NPCs (System 08 dialogue, System 03 faction opinion). A reputation emerges naturally—not a "fame bar," but the result of NPCs remembering and sharing stories (System 09 chronicles, System 08 rumor).
3. **Faction & Social**: Avatar belongs to a faction; can defect to another (high relationship with target faction). Avatar's faction membership affects trade prices, combat alliances, and dialogue options.
4. **Career Arc**: Avatar can specialize in roles: Keeper (System 06 combat), Scholar (System 09 lore), Merchant (System 04 trade), Settler (System 20 migration, settlement founding), etc. Career is not a fixed class but emerges from playstyle choices.
5. **Breeding & Lineage**: Player can engage in selective breeding of captive creatures (System 01, System 13). Breeding is slow and reward-deferred—a creator arc for players who enjoy long-term husbandry.
6. **Permadeath Philosophy**: The world continues without the avatar. An NPC may recall the dead player avatar in dialogue ("Remember when [Your Name] saved us from [Beast]?"). Future avatars encounter the ghost of past actions.

**Key principle**: The player is subject to the simulation, not above it. This creates genuine stakes and immersion.

---

## 2. Research Basis

### Embedded Agency & First-Person Simulation (Bogost, 2007; Murray, 1997)
Interactive media creates meaning through constraint and agency—what the player *can* do shapes what they *want* to do. In simulations with embedded agency, the player's avatar is a constrained actor, not a god. This increases immersion and creates emergent narratives. Examples: Dwarf Fortress permadeath (losing a fortress is a story); Spelunky roguelike death (permanent but creates unique runs); hardcore Diablo permadeath (risk creates drama).

- Bogost, I. (2007). *Persuasive Games*. MIT Press.
- Murray, J.H. (1997). *Hamlet on the Holodeck*. MIT Press.

**Application**: Avatar is mortal, fallible, and subject to the same rules as NPCs. Permadeath is optional (sandbox mode allows respawn; hardcore mode does not). Death is final but creates closure: chronicles record the avatar's life; successors encounter the legacy.

### Sandbox Play & Emergence (Kelley, 2012; Adams & Dormans, 2012)
Open-ended sandbox play (Minecraft, Dwarf Fortress) derives engagement from player agency to set goals. Players decide what matters: building, optimization, storytelling, or destruction. The player's role is emergent, not assigned. This is distinct from narrative-driven games where role is scripted.

- Kelley, K. (2012). "The Aesthetics of Play." In *Game Design Reader*.
- Adams, E. & Dormans, J. (2012). *Game Mechanics*. Peachpit Press.

**Application**: Avatar can pursue multiple career arcs: combat specialist (Keeper), researcher (Scholar via System 09), entrepreneur (merchant via System 04), explorer, settler, etc. No scripted quests force a role. Playstyle shapes emergent role.

### Selective Breeding & Artificial Selection (Darwin, 1868; Falconer & Mackay, 1996)
Domestication involves selective breeding: humans choose breeding pairs based on desired traits, altering allele frequencies over generations. Domesticated animals diverge rapidly from wild ancestors. Archaeological evidence shows that domestication creates visible changes within 10–20 generations. Behavioral change is often the first step (tameness selects for neural crest cell variants affecting multiple traits—the "domestication syndrome").

- Darwin, C. (1868). *The Variation of Animals under Domestication*. John Murray.
- Falconer, D.S. & Mackay, T.F.C. (1996). *Introduction to Quantitative Genetics* (4th ed.). Longman.

**Application**: Player-captive creatures are subject to player-directed artificial selection. Player chooses breeding pairs (System 13). Each generation, offspring inherit filtered allele pools. Over 30–50 game-years, a carefully-managed captive lineage can diverge significantly from wild populations (visual changes, behavioral docility, novel traits). This is a long-term meta-goal for players who enjoy husbandry.

### Permadeath & Roguelike Design (Adams, 2013; Tarn Adams, DF Design)
Permadeath (character death is permanent, no reload) creates high stakes and emergent narratives. Roguelike games (Dwarf Fortress, NetHack, Spelunky) use permadeath to create unpredictability and loss aversion. Loss becomes emotionally meaningful precisely because it is final. Chronicles of fallen adventurers become lore.

- Adams, E. (2013). *Fundamentals of Game Design* (3rd ed.). Peachpit Press.
- Tarn Adams (Dwarf Fortress dev diary and design documents).

**Application**: Permadeath is optional (configurable at game start). Hardcore mode: avatar death ends game (but world is saved; can start new game in same world). Sandbox mode: avatar death triggers respawn with new avatar in same faction (less dramatic, more forgiving). Chronicles record deaths as historical events.

### Reputation & Rumor Propagation (Dunbar, 1992; Adler & Adler, 1989)
Social reputation emerges from gossip and witnessed behavior. Dunbar's research suggests that reputation is tracked socially (via conversation, not formal scores). Reputation is local and evolving—different people hold different opinions based on their social networks and information access.

- Dunbar, R.I.M. (1992). "Neocortex Size as a Constraint on Group Size in Primates." *Journal of Human Evolution*, 22(6).
- Adler, P.A. & Adler, P. (1989). "The Gloried Self." *Sociological Quarterly*, 30(3).

**Application**: Reputation is not a numeric "fame" variable. Instead, NPCs maintain opinions of the avatar (faction opinion system, System 03) based on witnessed actions. NPCs gossip (System 08 dialogue), sharing stories and updating opinions of third parties based on hearsay. Rumors drift and morph as they spread—the avatar may become legendary (reputation larger than reality) or notorious (feared but hated).

---

## 3. Entities & State

### Player Avatar (extends Agent)

```
PlayerAvatar extends Agent {
  avatar_id: int,
  player_id: int,
  creation_tick: int,
  
  // Identity
  name: string,  // Player-chosen
  faction_id: int,  // Starting faction; can defect
  origin_settlement_id: int,
  lineage_id: int,  // Links to predecessor/successor avatars and shared bloodline
  
  // Life & Death
  is_alive: bool,
  death_tick: int or null,
  death_cause: enum {
    Combat,
    Starvation,
    Disease,
    Environmental,
    Age,
  },
  
  // Reputation (emerges from system 03 faction_opinions)
  witnessed_actions: [
    {
      action_type: enum { CombatKill, Rescue, Trade, Craft, Explore, Steal, Betray, ... },
      target_id: id,
      witnesses: [agent_id, ...],
      tick: int,
      sentiment: float [−1, 1],  // Positive action = high sentiment
    }
  ],
  // Note: Reputation is queried via faction.opinion[avatar_id], not stored separately
  
  // Career & Specialization
  career_primary: enum {
    Keeper,        // Combat specialist
    Scholar,       // Lore & research
    Merchant,      // Trade & economics
    Settler,       // Founding & migration
    Artisan,       // Crafting & creation
    Ranger,        // Exploration & hunting
    Tactician,     // Warfare & tactics
    Unspecialized,
  },
  career_experience: {
    [career_type]: float [0, 1],  // Mastery level
  },
  
  // Breeding & Husbandry
  captive_creature_lineages: [
    {
      creature_species_id: int,
      founder_ancestors: [creature_id, ...],
      current_population: [creature_id, ...],
      intended_selection_pressure: [
        {
          trait_channel: channel_id,
          direction: enum { Increase, Decrease, Neutral },
          generation_started: int,
        }
      ],
      generations_of_selection: int,
      phenotypic_divergence_from_wild: float [0, 1],  // 0 = identical; 1 = fully domesticated
    }
  ],
  
  // Successor Avatar (if permadeath mode)
  successor_avatar_id: int or null,
  predecessor_avatar_id: int or null,
}
```

### Lineage & Dynasty (for successor tracking)

```
AvatarLineage {
  lineage_id: int,
  founding_faction_id: int,
  founding_tick: int,
  
  avatar_sequence: [avatar_id, ...],  // Linked list of avatars in this lineage
  shared_goal_or_theme: string or null,  // E.g., "Breeding the perfect SaberCat"
  
  // Lineage stats
  total_lifespan_ticks: float,  // Sum of all avatars in lineage
  total_wealth_accumulated: float,
  most_notable_achievement: string or null,
  
  // Dynasty prestige (subjective + objective)
  faction_prestige_contribution: float,  // How much lineage's actions raised faction reputation
}
```

### Career Specialization (per avatar)

Career is not a rigid class. Instead, avatar's actions implicitly build career_experience in categories. Career type is assigned when experience in one domain exceeds others.

```
CareerMastery {
  keeper_combat_kills: int,
  keeper_creatures_defeated: [species_id, ...],
  keeper_survival_ticks_in_combat: int,
  
  scholar_lore_entries_discovered: int,
  scholar_chronicles_written: int,
  scholar_languages_learned: int,
  
  merchant_trades_executed: int,
  merchant_profit_accumulated: float,
  merchant_reputation_with_factions: {
    [faction_id]: float,
  },
  
  settler_settlements_founded: int,
  settler_populations_established: int,
  settler_migration_distance_total: float,
  
  artisan_recipes_invented: int,
  artisan_items_crafted: int,
  artisan_reputation_with_crafters: float,
  
  ranger_biomes_explored: int,
  ranger_creatures_species_encountered: int,
  ranger_distance_traveled: float,
  
  // Compute career type from max domain
  primary_career = argmax(keeper, scholar, merchant, settler, artisan, ranger)
}
```

### Permadeath Configuration

At game start, player selects permadeath mode:

```
PermaDeathSettings {
  mode: enum {
    Sandbox,       // Death triggers respawn with new avatar in same world
    Hardcore,      // Death ends game; world saved for next playthrough
    Ironman,       // Death ends game; world is deleted (most extreme)
  },
  
  // Optional softening for Hardcore
  successor_inheritance: {
    inherits_skill_points: bool,     // Next avatar starts with same skills (learning ratchet)
    inherits_faction_memories: bool, // NPC opinions of lineage affect new avatar
    inherits_captive_lineages: bool, // Breeding progress continues
  },
}
```

---

## 4. Update Rules

### Avatar Life Cycle

Avatar is a normal Agent (System 13) with additional death handling:

```
function avatar_ages_and_ages(avatar: PlayerAvatar, tick: int):
  // Standard aging (System 13)
  avatar.age_ticks += 1
  
  // Mortality increases with age (Gompertz-like)
  mortality_risk = base_mortality_rate * exp(age_ticks / LIFESPAN_TICKS * 5)
  if random() < mortality_risk:
    avatar.is_alive = false
    avatar.death_tick = tick
    avatar.death_cause = Age
    trigger_avatar_death_event(avatar, tick)
```

### Avatar Death Event

```
function trigger_avatar_death_event(avatar: PlayerAvatar, tick: int):
  // Record death in chronicles
  chronicle_event = ChronicleEntry(
    event_type = "AvatarDeath",
    subject_id = avatar.avatar_id,
    tick = tick,
    description = "The legendary [Avatar Name], a [Career] of [Faction], died after [Lifespan] years of life."
  )
  world.history.add_entry(chronicle_event)
  
  // NPC reactions: NPCs who knew avatar express grief, remembrance
  for npc in avatar.faction.agents:
    if npc.memory.know_avatar(avatar):
      npc.memory.episodic_log.add(
        EpisodicTrace(
          event_type = PersonalGrief,
          subject_id = avatar.avatar_id,
          emotion_weight = 0.8 if npc.opinion[avatar] > 0.5 else 0.3,
          tick = tick
        )
      )
  
  // Handle permadeath setting
  if world.permadeath_mode == Hardcore:
    display_game_over_screen(avatar)
    # Game ends; world saved for successor playthrough
  
  elif world.permadeath_mode == Sandbox:
    # Spawn successor avatar automatically
    trigger_successor_avatar_spawn(avatar, tick)
  
  elif world.permadeath_mode == Ironman:
    display_game_over_screen(avatar)
    # Game ends; world deleted

function trigger_successor_avatar_spawn(predecessor: PlayerAvatar, tick: int):
  successor = new PlayerAvatar(
    faction_id = predecessor.faction_id,
    origin_settlement_id = predecessor.origin_settlement_id,
    lineage_id = predecessor.lineage_id,
    predecessor_avatar_id = predecessor.avatar_id,
  )
  
  # Inheritance (configurable)
  if world.successor_inheritance.inherits_skill_points:
    for (technique, skill_points) in predecessor.techniques.items():
      successor.techniques[technique] = skill_points * 0.5  # Half inheritance
  
  if world.successor_inheritance.inherits_faction_memories:
    # NPCs treat successor as related to predecessor
    for npc in world.agents:
      if npc.opinion[predecessor] > 0.0:
        npc.opinion[successor] = npc.opinion[predecessor] * 0.7  # Familial reputation transfer
  
  if world.successor_inheritance.inherits_captive_lineages:
    successor.captive_creature_lineages = predecessor.captive_creature_lineages.deepcopy()
    # Breeding continues seamlessly
  
  predecessor.successor_avatar_id = successor.avatar_id
  successor.predecessor_avatar_id = predecessor.avatar_id
  
  player.control_avatar(successor)
  display_succession_screen(predecessor, successor)
```

### Reputation Emergence (System 03 + System 08 feedback)

Avatar's actions automatically update NPC opinions via the standard System 03 mechanism (no special logic needed):

```
function avatar_witnessed_action_affects_opinions(avatar: PlayerAvatar, action: Action, witnesses: [Agent]):
  for witness in witnesses:
    opinion_delta = action.sentiment * 0.3  // Direct witness weight
    witness.faction.opinion[avatar] += opinion_delta
    
    // Witness gossips (System 08 dialogue)
    for listener in witness.nearby_agents():
      listener.faction.opinion[avatar] += opinion_delta * 0.1  // Hearsay discount
    
    // Record in witness memory (System 17)
    witness.memory.episodic_log.add(
      EpisodicTrace(
        event_type = WitnessedPlayerAction,
        subject_id = avatar.avatar_id,
        details = action,
        emotion_weight = abs(action.sentiment),  // Dramatic actions stick in memory
        tick = world.current_tick
      )
    )
```

### Career Specialization Tracking

During gameplay, avatar's actions increment career_experience counters:

```
function track_avatar_career_progress(avatar: PlayerAvatar, action: Action, tick: int):
  if action.type == Combat:
    avatar.career_mastery.keeper_combat_kills += 1
    if action.target_is_creature:
      avatar.career_mastery.keeper_creatures_defeated.add(target.species_id)
    avatar.career_mastery.keeper_survival_ticks_in_combat += action.duration
  
  elif action.type == Craft:
    avatar.career_mastery.artisan_items_crafted += 1
    if action.is_novel_recipe:
      avatar.career_mastery.artisan_recipes_invented += 1
  
  elif action.type == Trade:
    avatar.career_mastery.merchant_trades_executed += 1
    avatar.career_mastery.merchant_profit_accumulated += action.profit
  
  # ... similar for other actions ...
  
  # Recompute primary career
  avatar.career_primary = argmax(
    avatar.career_mastery.keeper,
    avatar.career_mastery.scholar,
    # ... etc ...
  )
```

### Selective Breeding Interface

Player can interact with captive creatures to designate breeding pairs:

```
function designate_breeding_pair(avatar: PlayerAvatar, individual_A: Creature, individual_B: Creature, tick: int):
  # Validate: both in avatar's captive population
  if individual_A not in avatar.captive_creature_lineages[species].current_population:
    return ERROR
  
  lineage = avatar.captive_creature_lineages[individual_A.species_id]
  
  # Log breeding intent
  lineage.intended_selection_pressure = [
    # Player-observable phenotypes of A and B
    {
      trait_channel = individual_A.phenotype.strongest_channel,
      direction = Increase,  // Increase this trait
      generation_started = tick,
    },
    {
      trait_channel = individual_B.phenotype.strongest_channel,
      direction = Increase,
      generation_started = tick,
    }
  ]
  
  # Queue breeding event (System 13)
  trigger_breeding_event(individual_A, individual_B, tick, captive=true, breeder=avatar)

function offspring_inherits_player_selection(parent_A: Creature, parent_B: Creature, offspring: Creature, avatar: PlayerAvatar, tick: int):
  lineage = avatar.captive_creature_lineages[offspring.species_id]
  
  # Player-directed selection: offspring inherits alleles more likely from stronger phenotype
  for allele in offspring.genotype.alleles:
    # Weighted inheritance: stronger parent's alleles more likely
    strength_A = parent_A.phenotype.channel_values[allele.channel]
    strength_B = parent_B.phenotype.channel_values[allele.channel]
    
    if strength_A > strength_B:
      inheritance_bias_toward_A = (strength_A - strength_B) / (strength_A + strength_B)
    else:
      inheritance_bias_toward_A = -(strength_B - strength_A) / (strength_A + strength_B)
    
    if random() < 0.5 + inheritance_bias_toward_A * 0.3:
      offspring.genotype[allele.locus] = parent_A.genotype[allele.locus]
    else:
      offspring.genotype[allele.locus] = parent_B.genotype[allele.locus]
  
  # Track divergence
  lineage.generations_of_selection += 1
  lineage.phenotypic_divergence_from_wild = compute_divergence(
    wild_population_traits,
    captive_lineage_traits
  )
```

### Faction Defection

Avatar can switch factions (if sufficiently accepted by target faction):

```
function attempt_faction_defection(avatar: PlayerAvatar, target_faction: Faction, tick: int):
  opinion_of_avatar = target_faction.opinion[avatar]
  
  if opinion_of_avatar < 0.7:
    return FAIL("Target faction does not trust you sufficiently")
  
  # Defection triggers opinion loss in origin faction
  origin_faction = world.get_faction(avatar.faction_id)
  for npc in origin_faction.agents:
    npc.faction.opinion[avatar] -= 0.5  // Betrayal
  
  # Avatar joins target faction
  avatar.faction_id = target_faction.faction_id
  
  # Chronicle records defection
  chronicle_event = ChronicleEntry(
    event_type = "FactionDefection",
    subject_id = avatar.avatar_id,
    tick = tick,
    description = "[Avatar Name] defected from [Origin Faction] to [Target Faction]."
  )
  world.history.add_entry(chronicle_event)
```

---

## 5. Cross-System Hooks

**System 01 (Evolution)**: Avatar can selectively breed captive creatures, altering allele frequencies. Captive lineages can diverge significantly from wild populations over 30+ generations.

**System 03 (Faction/Social)**: Avatar's faction opinion is maintained like any NPC's opinion. Avatar can defect to another faction. Avatar's actions ripple through social networks (gossip, System 08).

**System 04 (Economy)**: Avatar is a consumer and producer. Trading skills affect merchant-career experience. Avatar can accumulate wealth (optional wealth tracking).

**System 06 (Combat)**: Avatar engages in combat (Keeper career). Combat kills and survival ticks feed career_mastery.keeper.

**System 08 (Dialogue)**: Avatar converses with NPCs. NPCs may reference avatar's witnessed actions ("I heard you saved the village from a SaberCat"). Avatar's reputation affects dialogue options and NPC cooperation.

**System 09 (World History)**: Avatar's life and death are recorded in chronicles. Major actions (founding settlements, defeating legendary beasts) appear in historical records. Successors encounter the avatar's legend.

**System 13 (Lifecycle)**: Avatar ages, reproduces (optionally; player can marry and have biological offspring), and dies. Offspring inherit alleles + some skills (if inheritance enabled). Pregnancy/lactation mechanics apply to avatar (optional for realism).

**System 15 (Climate/Biome)**: Avatar is affected by climate (must seek shelter in extreme weather, System 06 environmental hazards). Climate change can affect avatar's captive creature populations.

**System 17 (Individual Cognition)**: Avatar maintains personal episodic memory like any NPC. Avatar can forget facts (memory decay), learn from observation (Bayesian updates), and anticipate opponent abilities in combat.

**System 18 (Language & Culture)**: Avatar learns NPC languages through dialogue. Avatar's cultural practices affect NPC opinions. Avatar can participate in faction ceremonies, festivals, etc.

**System 20 (Migration)**: Avatar can found new settlements (Settler career). Avatar can lead faction migration. Avatar can trigger refugee movements.

**System 22 (Master Serialization)**: Avatar's state is serialized as part of world state. Save/load preserves avatar's life, memories, reputation, breeding progress, etc.

---

## 6. Tradeoff Matrix

| Dimension | Choice | Rationale |
|---|---|---|
| **Permadeath Mode** | Hardcore (game ends) vs. Sandbox (respawn) | Hardcore creates emotional weight; Sandbox is forgiving. Chosen: both available; player picks at start. |
| **Successor Inheritance** | Full (all skills/rep transfer) vs. partial (50% transfer) vs. none | Full trivializes challenge; none removes meaning. Chosen: partial (50% skill transfer, 70% reputation transfer). |
| **Breeding Detail** | Simple allele inheritance vs. Mendelian inheritance | Simple is fast; Mendelian is realistic. Chosen: biased Mendelian (traits of stronger parent more likely inherited). |
| **Career Flexibility** | One career for lifetime vs. dynamic switching | One is focused; dynamic is explorable. Chosen: dynamic—career inferred from recent actions, can change over time. |
| **Faction Defection** | Allowed vs. locked to origin | Allowed creates emergent narratives; locked is simpler. Chosen: allowed (opinion threshold required). |
| **Reputation Visibility** | Explicit numeric "fame" vs. emergent gossip-based | Numeric is clear; emergent is immersive. Chosen: emergent—query NPC opinions to see reputation, not a separate score. |

---

## 7. Emergent Properties

- **Legacy & Dynasty**: Players develop emotional attachment to their avatars. A beloved avatar's death creates closure; a successor feels like continuing a lineage. Chronicles of a dynasty become personal mythology.

- **Risk-Taking Playstyle**: Permadeath creates meaningful risk. A player in Hardcore mode carefully avoids dangerous beasts; a Sandbox-mode player charges in. Playstyle emerges from risk tolerance.

- **Breeding Obsession**: Players pursuing Settler or Artisan careers often dedicate themselves to breeding pet creatures. A captive SaberCat lineage that diverges visually from wild SaberCats becomes a personal achievement. The player has *made* a new breed.

- **Infamous Reputation**: A villainous avatar who steals, betrays, and kills develops a fearful reputation. NPCs avoid the avatar or attack on sight. The avatar becomes an outlaw—not through quests, but through emergent consequences of witnessed actions.

- **Scholarly Legacy**: A Scholar avatar can found the world's first University (System 04 facility) and establish themselves as a Keeper of Lore. Future avatars may visit the library and encounter books written by a predecessor.

- **Multi-Avatar Playthroughs**: In Hardcore mode, a player might restart the world 5+ times, experiencing it from different faction perspectives and career paths. Each playthrough is a unique history.

- **Hero or Villain Cycles**: A legendary hero avatar dies; their successor is born into their shadow, praised but not yet tested. The player either lives up to the legend or shatters it (different emotional arcs).

---

## 8. Open Calibration Knobs

- **BASE_MORTALITY_RATE**: Baseline annual mortality (currently 0.01 or 1% per year for young adults). Increase for shorter lifespans; decrease for longer. Affects how many avatars a player controls in a single run.

- **LIFESPAN_TICKS**: Ticks until Gompertz mortality acceleration kicks in (currently 300,000 ticks ~ 50 game years). Increase to extend expected lifespan; decrease to shorten.

- **SUCCESSOR_SKILL_INHERITANCE_FRACTION**: Fraction of predecessor's skill_points inherited by successor (currently 0.5 or 50%). Increase to make successors powerful; decrease for harder resets.

- **SUCCESSOR_REPUTATION_INHERITANCE_FRACTION**: Fraction of NPC opinion toward predecessor that transfers to successor (currently 0.7 or 70%). Increase to make lineage reputation persistent; decrease to make each avatar start fresh.

- **BREEDING_GENERATION_TIME**: Ticks between breeding generations for captive creatures (currently inherited from species' reproduction_interval, System 13). Avatar can speed this up via magical means or selective pressure (future expansion).

- **CAREER_MASTERY_THRESHOLDS**: Minimum action counts to achieve career specialization (e.g., 10 kills → Keeper candidate, 50 kills → confirmed Keeper). Tune to control how long career specialization takes.

- **PERMADEATH_MODE_DEFAULT**: Which mode is presented as default (currently Sandbox for accessibility). Change to Hardcore to encourage permadeath playstyle.

- **FACTION_DEFECTION_OPINION_THRESHOLD**: Minimum opinion required to defect to target faction (currently 0.7 out of 1.0). Decrease to allow easier defection; increase to make it very difficult.

