# 50 — Pillar P5: Social Emergence

> **Cross-reference (added with docs 56/57/58).** Two parts of this pillar are now superseded or reframed:
> - The Voronoi single-membership assignment in §4.2 (`ind.population_culture_id = nearest`) is **superseded by [56_relationship_graph_emergence.md](56_relationship_graph_emergence.md)**: agents are members of an arbitrary set of overlapping clusters at multiple Leiden levels, derived from the typed agent-pair multigraph. The `population_culture` carrier is retained, but it is now itself a *derived node-cluster* rather than the primary membership target.
> - The cultural-axis labelling pattern in §3.1 (axes labelled post-hoc by Chronicler if they stabilise) is **a special case of [58_channel_genesis.md](58_channel_genesis.md)**'s latent-extraction mechanism. The 32 pre-allocated `cultural_trait_vector` axes are seed slots; once a population's behaviour stabilises a coherent direction, the axis is registered with `genesis:<src_pop>:<tick>:cultural_axis:<sig_hash>` provenance and propagates through P6a iterated learning.
>
> Coalition formation, governance-policy drift, kinship-pedigree queries, and CMLS in this doc are all retained unchanged.

**Replaces in v1:**
- the **12-dimensional fixed opinion space** in `systems/01,03,08` (`individual_vs_collective`, `hoarding_vs_distribution`, `local_vs_global_trade`, `hierarchy_vs_egalitarianism`, `tradition_vs_innovation`, `isolation_vs_expansion`, `in_group_loyalty`, `aggression_vs_diplomacy`, `risk_tolerance`, `beast_exploitation_vs_coexistence`, `beast_knowledge_priority`, `player_cooperation`),
- the **6-layer social-relationship enum** (`survival | economic | kinship | ideological | hierarchical | informational`),
- the **9-element interaction-type enum** (`Casual | Trade | SharedLabor | SharedDanger | Debate | Teaching | Ritual | Conflict | Command`),
- the **6×12 SALIENCE matrix** (need-state × opinion dimension),
- the **7-band disposition classification** (`HOSTILE..DEVOTED`),
- the **5-element treaty-type enum** (`NonAggression | TradeAgreement | MutualDefense | Vassalage | Unification`),
- governance-type enums implicit in the v1 system 03,
- kinship-template scaffolding implicit in v1 reproduction logic.

**Depends on:** P4 (carrying-capacity surplus drives political organisation), the existing creature evolution pipeline (`systems/01`) for individual genome channels (charisma, neural_speed, etc. — unchanged).

---

## 1. Overview

P5 replaces the v1 opinion / faction / governance scaffolding with **continuous, learnable social state on three new carriers**:

- `population_culture` — a per-population (≈ per-faction) cultural-trait vector and policy vector,
- `coalition` — a runtime alliance object with continuous policy-distance similarity,
- a `pedigree_record` projection onto every individual creature for emergent kinship.

The 12 fixed opinion axes are gone. Cultural axes *emerge* as principal components of the cultural-trait vector (CMLS — Boyd & Richerson 1985; Henrich 2015), are tracked over time, and may be labelled by the Chronicler if they stabilise. There is no `Faction.governance: enum`; there is a `governance_policy_vector` of continuous scalars (centralisation, formalisation, succession-entropy, coercion, transparency, …) and the emergent label "monarchy" / "council" / "horde" is a Chronicler 1-NN cluster over that vector — exactly the same labelling pattern as P3 biomes.

Treaties and coalitions are not enum members; they are agent-formed bindings between populations whose terms are themselves channels (`coalition.shared_defense`, `coalition.trade_freeness`, …). The treaty taxonomy is post-hoc.

Kinship structures (clan, lineage, household) are not enums either: every individual carries an explicit pedigree projection (parent ids, residence-rule lineage), and Chronicler queries derive clans by transitive closure on a Q32.32 kinship-distance threshold. Read's "kinship algebra" (Read 2007) gives the formal grounding.

---

## 2. Research basis

Full synthesis in `research/R20_ecology_social.md` (Part B). Key references:

- **Boyd, R. & Richerson, P. (1985)** *Culture and the Evolutionary Process* — foundational CMLS.
- **Henrich, J. (2015)** *The Secret of Our Success* — modern synthesis of CMLS; group-level cultural variation.
- **Cavalli-Sforza, L., Feldman, M. (1981)** *Cultural Transmission and Evolution* — vertical & horizontal transmission rules.
- **Cederman, L.-E. (1997)** *Emergent Actors in World Politics* — agent-based nation formation.
- **Axelrod, R. (1997)** "The dissemination of culture" — cultural-similarity-based interaction.
- **Read, D. (2007)** "Kinship theory: a paradigm shift." *Ethnology* 46 — kinship as algebra rather than role enum.
- **Turchin, P. (2003)** *Historical Dynamics* — cliodynamics; multilevel selection on polities.
- **Turchin, P. (2016)** *Ages of Discord* — secular cycles, polity collapse.
- **Carneiro, R. (1970)** "A theory of the origin of the state." *Science* 169 — circumscription / population-pressure → centralisation.
- **Polity IV / V-Dem** — empirical continuous governance dimensions.

---

## 3. Channels & carriers

### 3.1 New carrier: `population_culture`

One per coherent population (typically 1-1 with v1 factions, but factions are now derived from cultural clustering rather than declared).

#### Cultural trait vector (continuous, registry-extensible)

```
cultural_trait_vector: [Q32_32; N_CULTURAL_AXES]   // default N = 32
```

The kernel does **not** assign semantics to these axes. Mods may register named axes via the registry:

```jsonc
{ "id": "cultural_axis.individualism", "carrier": "population_culture",
  "range": [0, 1], "operators": ["drift_gaussian", "imitation_blend"], ... }
```

But the kernel treats unnamed axes identically to named ones — drift and imitation operators don't care about meaning. The Chronicler labels axes post-hoc if they stabilise across populations.

#### Governance policy vector (continuous)

| Channel id | Range | Role |
|------------|-------|------|
| `gov_centralisation` | `[0, 1]` | How much decision-making concentrates in few agents |
| `gov_formalisation` | `[0, 1]` | Codified rules vs. ad-hoc norms |
| `gov_succession_entropy` | `[0, 1]` | How uncertain succession is (0 = strict primogeniture, 1 = full melee) |
| `gov_coercive_capacity` | `[0, 1]` | Ability to enforce decisions by force |
| `gov_transparency` | `[0, 1]` | How visible decisions are to membership |
| `gov_legibility` | `[0, 1]` | Ability of state to monitor population (Scott 1998) |
| `gov_redistribution` | `[0, 1]` | Wealth-redistribution policy intensity |

#### Population-level state (derived)

| Channel id | Range | Role |
|------------|-------|------|
| `population_size` | `[0, 1e7]` | Headcount (sum of attached individuals) |
| `cultural_cohesion` | `[0, 1]` | Inverse trait-vector variance among members |
| `legitimacy` | `[0, 1]` | Asabiyya / collective trust (Turchin) |
| `surplus_kgC_yr` | `[0, ∞]` | P4 net production minus subsistence demand |

### 3.2 New carrier: `coalition`

Runtime, lightweight. Created when two `population_culture` carriers fall within a similarity-and-context threshold; dissolved when policy distance exceeds a threshold. Channels:

| Channel id | Range | Role |
|------------|-------|------|
| `shared_defense` | `[0, 1]` | Defensive-treaty intensity |
| `trade_freeness` | `[0, 1]` | Goods-flow openness between members |
| `policy_alignment_score` | `[0, 1]` | Diagnosed from policy-vector L2 distance |
| `dominance` | `[0, 1]` | Whose policies dominate (0 = symmetric, 1 = one-sided) |
| `kinship_overlap` | `[0, 1]` | From pedigree projection of member populations |

The v1 `treaty_type: enum` is reproduced at the labelling layer as a 1-NN cluster on `(shared_defense, trade_freeness, dominance, kinship_overlap)`.

### 3.3 Pedigree projection on individual creatures

A small per-creature record (extends existing creature carrier):

| Channel id | Type | Role |
|------------|------|------|
| `parent_a_id` | EntityId | Father (or "monoecious mother A") |
| `parent_b_id` | EntityId | Mother (or B) |
| `birth_cell_id` | CellId | Locality of birth |
| `residence_rule` | enum (registry-backed: `patrilocal`, `matrilocal`, `neolocal`, `ambilocal`, …) | Where adults live relative to parents — **note**: this *is* a registered taxonomy, but it is mod-extensible and minimal; we accept it as a research-grounded design choice |

Kinship distance between two individuals is computed by graph BFS on parent-id edges with a dampened weight. Clan membership = transitive closure under threshold `KINSHIP_THRESHOLD`. Lineage = direct ancestral chain.

> **Why we keep `residence_rule` as a small enum.** It encodes a fundamental binary-ish choice (where do you live after marriage) that anthropologists have shown drives kinship structure (Murdock 1949). Making it continuous would be over-engineering. We register it as a small set with mod extensibility, in the same vein as the prototype gallery in P3, not as a hardcoded enum in system code.

---

## 4. Update rules

### 4.1 Cultural drift & imitation (CMLS)

Every M ticks (default M = 12), per population:

```
fn culture_step(pop: &mut PopulationCulture, neighbours: &[&PopulationCulture]) {
    // Vertical transmission: drift from individual reproduction
    let drift = sample_gaussian_q32(0, SIGMA_DRIFT, &mut rng_culture);
    pop.cultural_trait_vector += drift;

    // Horizontal transmission: weighted imitation toward nearby successful populations
    // "Success" = legitimacy * surplus
    let mut weighted_target = ZERO_VEC;
    let mut total_w = Q32_32::ZERO;
    for n in neighbours.iter().sorted_by_id() {  // sorted!
        let dist = l2(pop.cultural_trait_vector, n.cultural_trait_vector);
        let success = n.legitimacy * n.surplus_kgC_yr_normalised;
        let w = exp_q32(-dist / DIST_KERNEL) * success;
        weighted_target += n.cultural_trait_vector * w;
        total_w += w;
    }
    if total_w > Q32_32::EPS {
        let target = weighted_target / total_w;
        pop.cultural_trait_vector =
            pop.cultural_trait_vector * (1 - K_IMITATION) + target * K_IMITATION;
    }

    // Recompute cohesion as inverse-variance across members
    pop.cultural_cohesion = 1 / (1 + variance_among_members(pop));
}
```

CMLS in 30 lines. The "axes that emerge" are eigenvectors of the cross-population covariance — surfaceable by the Chronicler.

### 4.2 Faction formation / fission

A `population_culture` is *born* by clustering: when a sub-group's cultural distance from the parent population's mean exceeds `FISSION_THRESHOLD` for `T_FISSION` ticks, it splits. A single-link agglomerative algorithm runs in Stage 7 (deterministic, sorted-id):

```
fn faction_clustering_step(world: &mut World) {
    // Attach individuals to closest population_culture (Voronoi-like in cultural space)
    for ind in world.individuals.iter_sorted_by_id() {
        let nearest = nearest_population(ind.cultural_trait_vector);
        ind.population_culture_id = nearest;
    }
    // For each population, check internal variance; if too large, split
    for pop in world.populations.iter_sorted_by_id() {
        if internal_variance(pop) > FISSION_THRESHOLD {
            let (sub_a, sub_b) = bisect_population(pop);
            world.populations.spawn(sub_a);
            world.populations.spawn(sub_b);
            world.populations.retire(pop.id);
        }
    }
}
```

The v1 "faction archetypes" are reproduced at UI level by labelling `cultural_trait_vector` clusters. No archetype enum exists in sim.

### 4.3 Governance policy update

Every M_GOV ticks, each population's governance vector drifts under selection pressure:

```
fn governance_step(pop: &mut PopulationCulture, env: &EnvSignals) {
    // Carneiro circumscription: high pop pressure + spatial constraint → centralisation
    let pressure = pop.population_size / pop.carrying_capacity;
    let circumscription = 1 - pop.cells.len_q32() / pop.population_size;  // crowded → 1
    pop.gov_centralisation += K_GOV_LR * (pressure * circumscription
                                          - pop.gov_centralisation);

    // Surplus → formalisation (Polity IV-style: large surpluses fund bureaucracy)
    pop.gov_formalisation += K_GOV_LR * (sigmoid(pop.surplus_kgC_yr - SURPLUS_THRESH)
                                          - pop.gov_formalisation);

    // Succession entropy decays as formalisation grows
    pop.gov_succession_entropy += K_GOV_LR * (1 - pop.gov_formalisation
                                               - pop.gov_succession_entropy);
    // Coercive capacity scales with surplus + tech (P6 channels)
    pop.gov_coercive_capacity += K_GOV_LR * (pop.surplus_kgC_yr * tech_factor(P6)
                                              - pop.gov_coercive_capacity);
}
```

The Chronicler's 1-NN clustering over the resulting `(centralisation, formalisation, succession_entropy, coercive_capacity, transparency)` space yields labels like "tribal_council", "chiefdom", "feudal_monarchy", "bureaucratic_state" — registered in the same prototype-gallery JSON pattern as P3 biomes.

### 4.4 Coalition formation

```
fn coalition_step(world: &mut World) {
    // Pairwise: form coalition if policy distance below threshold AND both have > MIN_LEGITIMACY
    for (a, b) in world.populations.iter_sorted_pairs() {
        let pd = policy_distance(a, b);
        if pd < COALITION_FORM_THRESHOLD && existing_coalition(a, b).is_none() {
            world.coalitions.spawn(Coalition::new(a, b));
        }
        // Dissolve if drift makes them too distant
        if pd > COALITION_DISSOLVE_THRESHOLD {
            if let Some(c) = existing_coalition(a, b) { world.coalitions.retire(c); }
        }
    }
}
```

Wars are not a treaty enum: they are coalitions with `shared_defense > 0 ∧ kinship_overlap < threshold ∧ resource_competition_signal > threshold`, formalised by the Chronicler as labelled events.

### 4.5 Kinship queries

Stateless graph queries over the pedigree DAG. No special update rule — the DAG is append-only as births/deaths happen. Clan = transitive closure within `KINSHIP_THRESHOLD` Q32.32 distance under residence-rule-weighted edges.

---

## 5. Cross-pillar hooks

```mermaid
flowchart LR
    P4[P4 Ecology<br/>carrying capacity, prey biomass]
    P5[P5 Social]
    P6[P6 Culture/tech/econ/cog/disease/migration]
    EVO[v1 evolution<br/>(creatures, genome)]

    P4 -->|surplus_kgC_yr, density pressure| P5
    EVO -->|individual charisma,<br/>neural_speed,<br/>cooperation channels| P5
    P5 -->|cultural_trait_vector| P6
    P5 -->|governance_policy_vector| P6
    P6 -->|technology channels (boost coercive_capacity, formalisation)| P5
    P6 -->|disease pressure (P6 → mortality → faction stress)| P5
    P5 -->|coalition territory (cell membership)| P4
    P5 -->|hunting/foraging effort| P4
```

---

## 6. Tradeoff matrix

| Decision | Options | Sim Fidelity | Implementability | Player Legibility | Emergent Power | Choice + Why |
|----------|---------|--------------|------------------|-------------------|-----------------|-------------|
| Opinion space | Fixed 12 axes / **N continuous unnamed axes (CMLS)** / Topic-modelled | CMLS strong | CMLS moderate | Lower (axes shift) | Much higher | **CMLS, N = 32**. Axes labelled post-hoc by Chronicler. |
| Faction identity | Archetype enum / **Cultural cluster** / Network-community | Cluster strong | Cluster moderate | Cluster moderate | Strong (factions form / split / merge) | **Cultural cluster** with single-link agglomeration. |
| Treaty types | 5 enum / **Continuous coalition channels** | Continuous strong | Same complexity | Continuous less crisp | Strong (mixed treaties emerge) | **Continuous**. |
| Governance | Type enum / **Continuous policy vector** / Polity IV-grounded | Polity strong | Polity moderate | Polity moderate | Strong | **Polity-IV-style continuous**. |
| Kinship | Template archetypes (clan/lineage/household enum) / **Pedigree DAG with kinship algebra (Read)** | Pedigree strict | Pedigree moderate | Same | Pedigree much higher (matrilocal vs patrilocal effects emerge) | **Pedigree + algebra**. |
| Cultural transmission | Vertical-only / **Vertical + horizontal weighted by success** | Both | Same | Same | Horizontal much richer | **Both**. |
| Carneiro centralisation | Hardcoded thresholds / **Pressure × circumscription continuous drift** | Both | Same | Continuous legible | Continuous strong | **Continuous drift**. |
| Disposition bands | 7-band enum / **Continuous attitude vector** | Continuous strict | Same | Bands easier for UI | Continuous strong | **Continuous in sim, bands at UI**. |

---

## 7. Emergent properties

1. **Polity formation by circumscription.** High population density in a resource-rich, geographically constrained valley → centralisation drift → "chiefdom"-like Chronicler label. Diffuse low-density populations stay band-like. This is Carneiro 1970 falling out of the dynamics.
2. **Secular cycles (Turchin).** Surplus accumulation → elite proliferation (high `cultural_cohesion` drop) → factional fission → conflict → demographic crash → re-aggregation. The v1 "faction lifecycle thresholds" become emergent rather than scripted.
3. **Cultural drift between isolated populations.** Two sub-populations separated by P1 climate barriers diverge culturally over generations; the kinship DAG records the lineage; the Chronicler can write "the Northwood Clan and the Marsh Folk share ancestry but diverged 800 ticks ago" without any dialogue trigger.
4. **Treaty taxonomy emerges from coalition channels.** A high-`shared_defense` low-`trade_freeness` low-`dominance` coalition between similar-policy populations → "mutual defence pact". A high-`dominance` low-`shared_defense` coalition → "vassal/tributary". The Chronicler does the labelling.
5. **Kinship-mediated economy.** Trade flows preferentially within high-`kinship_overlap` coalitions; the v1 `local_vs_global_trade` axis emerges as a Chronicler-labelled principal component, not as input state.
6. **Religion-like ideologies.** Stable cultural axes that correlate with high `gov_legitimacy` are surfaced as "ideologies" by Chronicler clustering — e.g., a cluster of populations sharing a high `cultural_axis_27` value over many generations gets a name.

---

## 8. Open calibration knobs

- `N_CULTURAL_AXES` (default 32).
- `SIGMA_DRIFT` (vertical-transmission noise scale).
- `K_IMITATION`, `DIST_KERNEL` (horizontal-transmission rate and bandwidth).
- `FISSION_THRESHOLD`, `T_FISSION` (faction-split criterion).
- `COALITION_FORM_THRESHOLD`, `COALITION_DISSOLVE_THRESHOLD`.
- `K_GOV_LR` (governance-vector learning rate).
- `SURPLUS_THRESH` (surplus level above which formalisation drift kicks in).
- `KINSHIP_THRESHOLD` (clan-edge distance threshold).
- `MIN_LEGITIMACY` (gating for coalition formation).

---

## 9. Determinism checklist

- ✅ All cultural drift Q32.32 with seeded Xoshiro stream `rng_culture`.
- ✅ Sorted-id iteration over populations and individuals.
- ✅ Coalition formation iterates ordered pairs (sorted-id < sorted-id).
- ✅ Single-link agglomerative bisection uses stable sort on cultural-distance.
- ✅ Pedigree DAG is append-only; queries are read-only.
- ✅ Kinship-distance BFS uses sorted neighbour edges.

---

## 10. Sources

- Boyd, R., Richerson, P. (1985). *Culture and the Evolutionary Process*.
- Henrich, J. (2015). *The Secret of Our Success*.
- Cavalli-Sforza, L., Feldman, M. (1981). *Cultural Transmission and Evolution*.
- Cederman, L.-E. (1997). *Emergent Actors in World Politics*.
- Axelrod, R. (1997). "The dissemination of culture." *Journal of Conflict Resolution* 41.
- Read, D. (2007). "Kinship theory: a paradigm shift." *Ethnology* 46.
- Turchin, P. (2003). *Historical Dynamics*; (2016) *Ages of Discord*.
- Carneiro, R. (1970). *Science* 169.
- Murdock, G. P. (1949). *Social Structure*.
- Polity IV / V-Dem datasets (governance dimensions).
- Scott, J. C. (1998). *Seeing Like a State*.
