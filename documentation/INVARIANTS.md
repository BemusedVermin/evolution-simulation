# Beast Evolution Game: Engineering Invariants

This document consolidates critical invariants that define the simulation's contract and non-negotiable constraints.

---

## 1. Determinism (Critical Issue #3)

**Statement**: Given identical initial state, seed, and input sequence, the simulation must produce bit-identical world state at every tick. Replay validation in CI must pass: save N ticks → snapshot → replay → verify tick-by-tick sim state hash.

**Rationale**: Multiplayer, modding, replay analysis, and bug reproduction all depend on deterministic replay. Floating-point arithmetic is not bit-identical across platforms/compilers; this cannot be relied upon for sim state.

**Commitment**:
- All simulation-state math uses fixed-point arithmetic: Q32.32 format for continuous quantities in [0, 1], i32 for counts.
- PRNG: xoshiro256** seeded once at world creation; one stream per subsystem to prevent cross-contamination.
- Iteration order: always over sorted entity keys; no unordered maps or set iteration in hot paths.
- Timing: no wall-clock dependencies; only tick-count-based logic.
- RNG: never use OS RNG (no libc random, no std::mt19937 if uninitialized).

**Validation Approach**:
- CI test: run save → replay 100 ticks → snapshot every tick → hash compare (identical hashes → pass).
- Test fixture: determinism_test.json provides seed + initial state + input journal.
- Failure protocol: any divergence triggers binary diff of tick data; review includes numerical analysis (overflow, saturation, rounding mode).

---

## 2. Mechanics-Label Separation (System Invariant 3.9)

**Statement**: Gameplay mechanics derive only from primitive emissions; no hand-authored ability names ("Echolocation", "Pack Hunting") appear in sim code. Naming is the Chronicler's responsibility.

**Rationale**: Decouples evolution from art/narrative; allows emergent behaviors to surface without bloating the codebase.

**Validation Approach**: Static analysis: grep for quoted ability names in systems 01–20. All occurrence are documentation-only, not control flow.

---

## 3. Channel Registry Monolithicism

**Statement**: At runtime, a single authoritative channel registry (core + mod + genesis-derived entries) defines all available channels. Code never hardcodes channel assumptions; all composition rules live in manifest hooks.

**Rationale**: Enables modding and genesis without recompilation; evolution system is agnostic to channel set.

**Validation Approach**: Registry load test: parse all manifest JSON; ensure id uniqueness; verify family membership. Reject malformed entries at load time.

---

## 4. Emergence Closure (Invariant 3.6)

**Statement**: All named behaviors and emergent capabilities must trace back to primitive emissions. No ghost mechanics.

**Rationale**: Ensures the system is understandable and auditable; prevents hidden gameplay rules.

**Validation Approach**: Behavior traceability audit: for each documented behavior, identify which primitives enable it. Document in System 11 appendix.

---

## 5. Scale-Band Unification

**Statement**: All evolutionary dynamics, predator–prey interactions, and metabolic scaling apply uniformly across body-size scales (macro hosts to micro pathogens). No scale-specific hardcoding.

**Rationale**: Reduces design surface; Kleiber's Law (metabolic rate ∝ mass^0.75) provides principled scaling.

**Validation Approach**: Allometric test: run evolution at three scales (10g, 1kg, 100kg); verify mutation distribution, fitness distribution, and equilibrium population sizes match expected scaling curves.

---

## 6. UI State vs. Sim State Separation

**Statement**: Bestiary "discovered" flag is DERIVED from sim observation counts (bestiary_observations >= 1); it is never written directly. Camera filters, notes, and sort order are pure UI state. Sim state includes only: creatures, agents, settlements, biomes, and their evolution/ecology.

**Rationale**: Ensures save/load is auditable; UI cosmetics do not pollute versioning.

**Validation Approach**: Schema validation: bestiary_observations is in Creature entity; bestiary_discovered is computed at load. Serialize check: save file never contains bestiary_discovered key; verification at parse time rejects any file that does.

---

## 7. Groups Are Derived (from `emergence/56_relationship_graph_emergence.md`)

**Statement**: A group (faction, settlement, polity, religion, guild, household, pack, …) is a *derived view* over the typed agent-pair multigraph; it is never authoritative state on disk. Save state contains the relationship-edge accumulators and the per-agent carriers; cluster memberships, containment hierarchies, and edge-cluster types are recomputed by Stage-7 community detection on load.

**Rationale**: Preserves emergence-closure for social structure: every group traces back to the primitive emissions that produced its edges. Save files are smaller and don't carry stale derived state; replay reconstructs all groups exactly.

**Validation Approach**: Save-format check: deny any save key matching `*.faction_id`, `*.settlement_id`, `*.cluster_id`, `*.polity_id` etc. on agent / settlement entities. Load test: snapshot post-load membership-set; verify identical to pre-save derivation.

---

## 8. No Authored Relationship-Type or Group-Type Vocabulary (from `emergence/56_relationship_graph_emergence.md`)

**Statement**: The kernel must not declare relationship-type names (no `kinship`, `fealty`, `trade` enums in code) or group-type names (no `polity`, `kingdom`, `guild` enums). Etic vocabularies live only in JSON galleries (`relationship_label_gallery.json`, `cluster_label_gallery.json`); they are layer-5 inputs into the P7 naming pipeline (`70_naming_and_discovery.md`), never canonical names. Per-population emic names are sim state in `population.lexicon`.

**Rationale**: Strict generalisation of Mechanics-Label Separation from creature abilities to social structure. Kingdoms emerge from cluster signatures; they are not declared shapes in code.

**Validation Approach**: Static analysis: grep for the strings `"kingdom"`, `"town"`, `"faction"`, `"guild"`, `"religion"`, `"clan"`, `"household"`, `"pack"`, `"kinship"`, `"fealty"`, `"trade"` in sim-path code (everything outside `documentation/` and `*_gallery*.json`). Zero hits required.

---

## 9. All Agent Decision-Making Runs Active Inference (from `emergence/57_agent_ai.md`)

**Statement**: Every agent — pathogen, beast, proto-sapient, sapient — selects actions by sampling from a policy posterior produced by Expected-Free-Energy planning over a discrete factored POMDP, using variational message passing for perception. There is no per-system FSM, behaviour tree, GOAP, utility-AI, or LLM-driven decision logic in the sim path. Combat AI, dialogue intent, foraging, migration, social interaction are all policies sampled from the same engine over different parts of the same factor graph.

**Rationale**: Forces emergence-first decision-making with one proven-correct deterministic mechanism. Tier labels (reactive / deliberative / reflective / Machiavellian / compulsive) are 1-NN Chronicler labels over `(predictive_horizon_ticks, model_depth, theory_of_mind_order)` — never authored modes.

**Validation Approach**: Static analysis: grep for `if creature.tier ==`, `match agent.behavior_state`, `select_behavior(`, `dialogue_intent ==` patterns in sim code. All decision points must route through `agent.gm.policy_posterior`.

---

## 10. Genesis Lineage Closure (from `emergence/58_channel_genesis.md`)

**Statement**: Any channel id of the form `genesis:*` (preference, factor, skill, primitive-mutation, cluster-label-gallery entry) MUST appear in the `genesis_event_log` with parent lineage. The kernel and mod registries are read-only at runtime; runtime additions go to the `genesis_registry` namespace whose entries every trace back to seed `core` or `mod:*` entries through a finite chain of recorded events.

**Rationale**: Strict generalisation of Emergence Closure from primitive emissions to channel vocabularies themselves. Any channel that has ever existed in a running world is recoverable from the seed registries plus the genesis event log.

**Validation Approach**: Load-time check: for every active channel with `provenance` matching `genesis:*`, verify a corresponding entry exists in the `genesis_event_log` and that the entry's parent ids resolve. Reject the save if any genesis-id has no event-log entry.

---

## Audit Checklist

- [ ] All channels: Q32.32 fixed-point in schema / mutation / composition
- [ ] All PRNG: xoshiro256** seeded, one stream per subsystem
- [ ] Iteration order: sorted entity keys in all hot loops
- [ ] No floating-point in sim state; floats only in UI/render code
- [ ] Determinism CI test: replay divergence causes test failure + binary diff report
- [ ] Bestiary: _observations_ in sim, _discovered_ flag computed at UI layer
- [ ] Mechanic names: zero hardcoded ability names in systems 01–20
- [ ] Groups: no `*.faction_id`/`*.cluster_id` keys in save files; recomputed by Stage 7
- [ ] Relationship/group vocabularies: zero hardcoded `"kingdom"`/`"fealty"`/`"guild"`-class strings in sim path
- [ ] Decision-making: every action routes through `agent.gm.policy_posterior` (active inference)
- [ ] Genesis: every `genesis:*` channel id resolves to a `genesis_event_log` entry with parent lineage
