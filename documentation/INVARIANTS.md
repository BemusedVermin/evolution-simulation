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

## Audit Checklist

- [ ] All channels: Q32.32 fixed-point in schema / mutation / composition
- [ ] All PRNG: xoshiro256** seeded, one stream per subsystem
- [ ] Iteration order: sorted entity keys in all hot loops
- [ ] No floating-point in sim state; floats only in UI/render code
- [ ] Determinism CI test: replay divergence causes test failure + binary diff report
- [ ] Bestiary: _observations_ in sim, _discovered_ flag computed at UI layer
- [ ] Mechanic names: zero hardcoded ability names in systems 01–20
