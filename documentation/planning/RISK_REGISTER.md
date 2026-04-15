# Beast Evolution Game: Risk Register & Mitigation Plan

This document identifies the top 10 risks to successful MVP delivery and specifies mitigation strategies.

---

## Risk Summary Table

| # | Risk | Probability | Impact | Score | Mitigation | Owner | Status |
|---|------|-------------|--------|-------|-----------|-------|--------|
| 1 | Determinism divergence at scale (1000+ ticks) | Medium (40%) | Critical (5) | 20 | Early determinism testing (E6); binary diff tool for debugging; property tests on all arithmetic | Solo+Claude | Active |
| 2 | Performance budget exceeded (> 20ms/tick) | Medium (50%) | High (4) | 20 | Profile early (S6); prioritize hot path; defer cold systems aggressively | Solo+Claude | Active |
| 3 | Mutation operators produce invalid genomes (values escape [0,1]) | Medium (40%) | High (4) | 16 | Rigorous bounds testing (E3); saturating arithmetic; property-based tests | Solo+Claude | Active |
| 4 | Complex composition hooks cause stack overflows or slow evaluation | Medium (35%) | High (4) | 14 | Limit hook depth; cache evaluated hooks; optimize fixed-point math | Solo+Claude | Active |
| 5 | Procedural visuals produce degenerate meshes (zero-area triangles) | Medium (40%) | Medium (3) | 12 | Validation pass on all meshes (E9); clamp minimum face sizes | Solo+Claude | Active |
| 6 | Chronicler label quality is poor (labels don't match emergent behaviors) | Low (25%) | Medium (3) | 7.5 | Manual playtesting; domain expert review; tune heuristics iteratively | Solo+Claude | Active |
| 7 | Combat balance is broken (creatures always die too fast or too slow) | High (70%) | Medium (3) | 21 | Playtesting early (S11); tune damage formula; gather user feedback on MVP | Solo+Claude | Active |
| 8 | ECS framework overhead (entity lookup, component access) degrades performance | Low (15%) | Medium (3) | 4.5 | Profile early; benchmark vs. naive arrays; use dense storage | Solo+Claude | Monitored |
| 9 | Save file format incompatibility across versions (migrations fail) | Low (15%) | High (4) | 6 | Version schema carefully; test migrations on real saves; plan forward compatibility | Solo+Claude | Active |
| 10 | UI event handling becomes bottleneck (1000 widgets, lag on input) | Low (20%) | Medium (3) | 6 | Lazy UI rendering; batch events; profile widget tree depth | Solo+Claude | Active |

---

## Risk #1: Determinism Divergence at Scale

**Probability**: Medium (40% chance of divergence by tick 1000)

**Impact**: Critical — Replay validation fails; modding/multiplayer impossible; bug reproduction unreliable

**Score**: 20 (high priority)

### Causes
- Floating-point arithmetic on different platforms produces different results
- PRNG state corruption or non-deterministic seeding
- Unordered map/set iteration in hot paths
- Temporal dependencies (wall-clock time instead of tick count)
- Rayon thread scheduling variability (work-stealing can change iteration order)

### Mitigation Strategy

**Early Detection (S1–S6)**:
- Implement Q3232 fixed-point type in E1; all property-based tests verify bit-identity
- PRNG tests (E1): same seed → identical sequence over 1M samples
- Build determinism test (E6) and run on every commit; fail if any hash diverges
- Sorted entity iteration (E5): enforce in all hot loops; test that iteration order is deterministic

**Debugging Tools (S6)**:
- Binary diff tool: when tick N diverges, identify first entity that differs
- Hash component-by-component (creature hash, pathogen hash, etc.) to localize divergence
- Snapshot system: save world state before/after each system in a stage; compare snapshots

**Safeguards (S3–S14)**:
- All mutation operators use saturating arithmetic (no wrapping overflow)
- All composition hooks evaluated in fixed-point (no floats in sim state)
- PRNG state isolated per subsystem (rng_evolution, rng_physics, etc. never cross-contaminate)
- No randomness post-hoc; all randomness seeded at world creation

**Testing (S6–S14)**:
- Determinism test: save at tick 50 → replay ticks 1–100 → all hashes must match (within rounding)
- Extended test (S14): save at tick 500 → replay all 500 ticks → perfect match
- CI test: every commit runs determinism test; failure blocks merge
- Fixture test: pre-generated save file + input journal must replay perfectly

### Acceptance Criteria
- [ ] 1000-tick replay produces bit-identical hashes (on same platform)
- [ ] Determinism test passes on multiple world seeds (5+ seeds)
- [ ] Binary diff tool identifies first divergence within 10 lines of code
- [ ] CI test runs and passes every commit

---

## Risk #2: Performance Budget Exceeded

**Probability**: Medium–High (50% chance of exceeding budget without optimization)

**Impact**: High — 60 FPS unachievable; gameplay unplayable; core features cut

**Score**: 20 (high priority)

### Causes
- Interpreter composition hook evaluation is O(N hooks × M operands); complex hooks slow
- ECS component storage overhead (sparse lookup on large worlds)
- Rayon thread spawning overhead (work-stealing adds latency)
- Chronicler pattern detection runs every tick (should defer)
- Spatial indexing (R-tree) build cost on every tick

### Mitigation Strategy

**Early Profiling (S6)**:
- After implementing tick loop (E6), profile first 100 ticks
- Identify budget offenders: which system/stage exceeds target?
- Record baseline per-stage times

**Hot Path Optimization (S7–S9)**:
- Composition hook evaluation: limit depth (max 3 levels); cache evaluated hooks per creature per tick
- Spatial indexing: build R-tree once per 10 ticks; reuse for physics/combat
- ECS iteration: use dense vector storage, not sparse sets
- PRNG sampling: use pre-computed lookup table for Gaussian Box-Muller (optional, measure first)

**Adaptive Quality (S13)**:
- If stage exceeds budget: skip "cold" system in next tick
- Priority tiers:
  - Tier 1 (every tick): Input, Genetics, Phenotype, Physics, Combat, Physiology
  - Tier 2 (every 10 ticks): Pathfinding, distant NPC behavior
  - Tier 3 (every 100 ticks): Chronicler pattern detection, speciation checks
  - Tier 4 (every 1000 ticks): Save checkpoints

**Batching & Caching (S8–S9)**:
- Batch render calls (world map: one draw call per tile layer, not per tile)
- Cache procgen results (creature meshes, once generated, don't regenerate)
- Defer UI updates (only re-layout widgets if data changed)

**Measurement (S6–S14)**:
- Profiling hooks in every system; elapsed time logged per-stage
- Budget violation triggers warning log + optional panic in debug builds
- Criterion benchmarks for hot functions (fixed-point multiply, PRNG sampling, hash computation)

### Acceptance Criteria
- [ ] 1000-tick run sustains < 15ms/tick on average (60 FPS feasible)
- [ ] Per-stage timing report shows all stages under budget
- [ ] Profiling identifies top 3 budget offenders; optimization plan drafted
- [ ] Rayon overhead < 10% of total per-stage time

---

## Risk #3: Mutation Operators Produce Invalid Genomes

**Probability**: Medium (40% chance of bounds violations without rigorous testing)

**Impact**: High — Evolution fails; creatures develop degenerate phenotypes; simulation breaks

**Score**: 16 (high priority)

### Causes
- Gaussian mutation drift pushes effect_vector values outside [0,1]
- Bounds clamping inconsistent (some operators clamp, others wrap)
- Regulatory modifier strength escapes [-1.0, 1.0]
- Body-site coverage escapes [0,1]
- Genesis channel values not properly initialized

### Mitigation Strategy

**Bounds Enforcement (E3)**:
- All mutation operators use saturating/clamping arithmetic
- Effect vector post-mutation: `clamp(value, 0.0, 1.0)` always
- Regulatory modifier strength: `clamp(strength, -1.0, 1.0)` always
- Body-site coverage: `clamp(coverage, 0.0, 1.0)` always
- Q3232 type ensures fixed-point saturation automatically

**Comprehensive Testing (E3)**:
- Property test: mutate random genome 10 times; all effects remain in [0,1] (10k samples)
- Edge case test: create genome with all values at bounds; mutate → ensure bounds respected
- Regulatory network test: build network with max depth; verify no infinite loops
- KS test: mutation kernel sample distribution matches expected Gaussian (p > 0.05)

**Validation System (E4)**:
- At phenotype interpretation: validate all channel values are in [0,1] before eval
- If channel escapes bounds: panic with clear message (will catch in testing)
- Logging: record any clamping events (should be zero in mutation, rare in interpreter)

**Monitoring (S3–S14)**:
- Run 1000-tick simulations with logging enabled; record all clamping events
- If any clamping detected: investigate and fix root cause
- Unit test new clamping cases

### Acceptance Criteria
- [ ] Mutate 1000 genomes 10 times each; zero bounds violations
- [ ] Property test passes (10k samples)
- [ ] Edge case test passes (bounds → mutate → bounds)
- [ ] Logging shows zero clamps in 1000-tick run

---

## Risk #4: Complex Composition Hooks Cause Slowdown or Stack Overflow

**Probability**: Medium (35% chance if hook evaluation is recursive without limits)

**Impact**: High — Interpreter becomes bottleneck; some creatures uninterpretable; crashes on deep networks

**Score**: 14 (high priority)

### Causes
- Composition hook evaluation is recursive (hook A depends on hook B, which depends on hook C)
- Deep dependency chains (10+ levels) cause stack overflow or exponential time
- No caching; same hook evaluated multiple times per creature per tick
- Parameter expressions are complex and slow to evaluate

### Mitigation Strategy

**Design Limits (E2–E4)**:
- Limit hook depth to 3 levels (composition_hooks can reference other hooks, but max depth = 3)
- Limit composition hooks per channel to 5 (a channel can have at most 5 composition_hooks)
- Validate manifests at load time (E2): reject hooks with depth > 3

**Caching (E4)**:
- Cache evaluated hooks per creature per tick in a HashMap: `(creature_id, hook_id) → evaluated_value`
- Clear cache at tick boundary (invalidate when creature genome changes)
- Measure cache hit rate; should be > 90%

**Expression Optimization (E4)**:
- Parameter expressions are fixed-point; pre-compile to bytecode if deep
- For MVP, simple expressions OK; optimize if profiling shows bottleneck

**Measurement (E6)**:
- Profiling: per-hook evaluation time logged
- Identify slow hooks (> 1µs per creature)
- Track interpreter wall time; target < 3ms/tick for 1000 creatures

**Safeguards (E4)**:
- Iterative evaluation: break evaluation into stages; if any operand is zero, return zero (short-circuit)
- Unit test: evaluate 100 randomly-generated composition hook networks; verify completion in < 100ms

### Acceptance Criteria
- [ ] Manifest loader rejects hooks with depth > 3
- [ ] 1000-creature interpretation completes in < 3ms
- [ ] Cache hit rate > 90%
- [ ] Zero stack overflow or infinite loops in fuzz test (100 random hook networks)

---

## Risk #5: Procedural Visuals Produce Degenerate Meshes

**Probability**: Medium (40% chance of visual glitches without validation)

**Impact**: Medium — Visual artifacts, but not gameplay-breaking; players tolerate quirky visuals

**Score**: 12 (medium priority)

### Causes
- Protrusion parameters generate 0-area triangles (e.g., spike at same position as spine)
- Shape generators produce NaN or infinity coordinates
- Mesh simplification removes necessary vertices
- Body region aggregation creates overlapping/inverted faces

### Mitigation Strategy

**Validation Pass (E9)**:
- After mesh generation: validate all triangles have area > epsilon (1e-6)
- Validate all vertices are finite (no NaN, no infinity)
- Clamp minimum spike size: `scale > 0.01` (prevent microscopic protrusions)

**Shape Generation (E9)**:
- Protrude: distribute N spikes evenly across region; avoid duplicate positions (deterministic spacing)
- Harden: thicken existing geometry proportionally; no new vertices
- Colorize: material assignment only; no geometry change
- Inflate: add uniform shell thickness; no self-intersections

**Testing (E9)**:
- Render 100 random creatures; validate all meshes pass degeneracy test
- Fuzz test: 1000 random visual directives → 100 creatures each → all meshes valid
- Visual inspection: spot-check 10 creatures for obvious glitches (degenerate spikes, inverted faces)

**Performance (E9)**:
- Mesh validation is fast (linear in triangle count); adds < 0.1ms per creature

### Acceptance Criteria
- [ ] 100 random creatures render without visual crashes
- [ ] All meshes pass degeneracy validation (zero degenerate triangles)
- [ ] Fuzz test passes: 1000 random directives produce valid meshes
- [ ] Visual inspection: no obvious glitches in 10 spot-checked creatures

---

## Risk #6: Chronicler Label Quality is Poor

**Probability**: Low (25% chance of unacceptable label quality)

**Impact**: Medium — Player doesn't understand emergent behaviors; labeling feels disconnected from mechanics

**Score**: 7.5 (lower priority, but address if time permits)

### Causes
- Heuristics for mapping primitive clusters to labels are simplistic
- Label generation is deterministic but not intuitive
- Confidence scoring doesn't match human perception of cluster quality
- Domain expertise needed to create good heuristics (team may lack biology knowledge)

### Mitigation Strategy

**Iterative Heuristic Development (E10–E12)**:
- Start with simple heuristics: high kinetic_force + jaw → "Biter"
- Add more heuristics as patterns emerge during testing
- Maintain heuristic registry in comments/docs for future maintainers

**Manual Playtesting (S12–S14)**:
- Run 1000-tick simulations; manually inspect labels
- Does "Biter" label match high kinetic_force creatures? If no, adjust heuristic.
- Gather feedback from first MVP users; iterate heuristics based on feedback

**Domain Expert Review (Post-MVP)**:
- If possible, consult with evolutionary biologist or game designer
- Refine heuristics based on expert feedback
- Update labels for v1.1

**Confidence Scoring (E10)**:
- Label confidence = cluster frequency (how many creatures in cluster) × stability (does cluster persist over time)
- Display only labels with confidence > 0.7 (hide low-confidence labels)
- This naturally filters out spurious labels

**Fallback (E10)**:
- If label quality is poor: show clusters by ID instead of name (e.g., "Cluster #3" instead of "Biter")
- Avoids misleading player; preserves functionality

### Acceptance Criteria
- [ ] Manual inspection of 10 evolved creatures shows labels that "make sense" (subjective, but documented)
- [ ] Confidence scores correlate with label quality (high confidence → good label)
- [ ] No labels assigned to clusters with confidence < 0.7
- [ ] Post-MVP: gather user feedback; iterate heuristics for v1.1

---

## Risk #7: Combat Balance is Broken

**Probability**: High (70% chance of imbalanced combat without extensive playtesting)

**Impact**: Medium — Combat is too easy or too hard; core gameplay loop breaks; player frustration

**Score**: 21 (critical priority)

### Causes
- Damage formula (offense_force × (1 − defense_rigidity)) not empirically tuned
- Starter creature genomes poorly balanced for MVP biomes
- Formation disruption mechanic too powerful or too weak
- Keeper stress scaling makes combat unpredictable

### Mitigation Strategy

**Early Balance Testing (S11)**:
- After implementing combat (E11), run 100 balanced encounters (Keeper vs. equivalent creatures)
- Measure: average damage per round, average combat duration, win/loss rate
- Target: 40–60% win rate for Keeper team (fair challenge)

**Iterative Tuning (S11–S14)**:
- If creatures die in < 3 rounds: reduce damage multiplier or increase creature health
- If creatures survive > 20 rounds: increase damage multiplier
- Starter genomes: tune to ~50% win rate against equivalent enemies
- Formation disruption: test if disrupted creatures too weak or too strong

**Playtesting (S13–S14)**:
- 5–10 players playtest combat for 30 min each
- Gather feedback: is combat fun? Fair? Challenging?
- Adjust based on feedback (e.g., "combat is too slow")

**Monitoring (Post-MVP)**:
- Log combat outcomes (win/loss rate, average duration, damage per round)
- If win rate > 70% or < 30%: rebalance for v1.1
- Community feedback loop: players report "combat is broken"; investigate and fix

### Acceptance Criteria
- [ ] 100 balanced encounters show 40–60% Keeper win rate
- [ ] Starter genomes survive 10-round combat (not instant death, not immortal)
- [ ] 5+ playtests show "combat feels fair and fun" (subjective, but documented)
- [ ] Damage formula has clear documentation (why values chosen)

---

## Risk #8: ECS Framework Overhead Degrades Performance

**Probability**: Low (15% chance if dense storage used from start)

**Impact**: Medium — Per-system iteration becomes bottleneck; interpreter slow; tick budget exceeded

**Score**: 4.5 (lower priority; well-mitigated by design)

### Causes
- specs World uses sparse sets (slow cache, poor locality)
- Entity lookup is O(log N) instead of O(1)
- Component access involves indirection (pointer chasing)

### Mitigation Strategy

**Design Choice (E5)**:
- Use dense vector storage from start (structure-of-arrays, not entity-centered)
- Prefer iteration over lookup (hot path is iterate-all-creatures, not get-creature-by-id)
- Sorted entity index (BTreeMap) is O(log N) lookup, but iteration is O(N) and cache-friendly

**Benchmarking (E5–E6)**:
- After ECS implementation, benchmark:
  - Entity iteration: measure time to iterate 1000 creatures, sum one component
  - Component access: measure time to get/set component for random entity
- Target: iteration < 1µs per entity, access < 10µs
- If target not met: profile and optimize storage layout

**Optimization (S6–S14)**:
- If profiling shows ECS overhead: switch to custom storage or PECS library (less overhead)
- Likely not needed (ECS overhead is usually < 5%)

### Acceptance Criteria
- [ ] Entity iteration: < 1µs per entity
- [ ] Component access: < 10µs per lookup
- [ ] 1000-creature iteration completes in < 1ms

---

## Risk #9: Save File Format Incompatibility

**Probability**: Low (15% chance if migrations not tested)

**Impact**: High — Old saves become unloadable; player progress lost; frustration

**Score**: 6 (lower priority, but important for long-term health)

### Causes
- Schema changes between versions without migrations
- Serialization format changes (e.g., bincode → serde_json) without conversion
- Forbidden-key rejection breaks old saves unexpectedly

### Mitigation Strategy

**Schema Versioning (E7)**:
- SaveFile includes schema_version field (e.g., "1.0.0")
- Manifest of migrations: schema 1.0 → 1.1, 1.1 → 1.2, etc.
- When loading old save: apply migrations iteratively

**Migration Testing (E7)**:
- Before each version change: create test save file in old schema
- Load old save into new code; apply migrations; verify load succeeds
- Run 10 ticks on loaded/migrated save; verify no crashes

**Forbidden-Key Handling (E7)**:
- SaveValidator rejects keys like `bestiary_discovered` (UI ephemeral state)
- Error message is clear: "Error: UI state cannot be saved. This is expected. Load the save file again; your creature data is intact."

**Forward Compatibility (E7)**:
- New code must accept old saves (load, migrate, run)
- Old code should reject new saves gracefully (or refuse to write incompatible saves)
- Plan: v1.0 saves must load in v2.0, but v2.0 saves may not load in v1.0

### Acceptance Criteria
- [ ] Load v1.0 save file in v1.1 code; migrations applied successfully
- [ ] Run 10 ticks on migrated save; no crashes
- [ ] SaveValidator rejects 5 malformed saves with clear error messages
- [ ] Forbidden-key test: save file with bestiary_discovered is rejected

---

## Risk #10: UI Event Handling Becomes Bottleneck

**Probability**: Low (20% chance if lazy rendering not implemented)

**Impact**: Medium — UI laggy on input; player frustration; 60 FPS not sustained

**Score**: 6 (lower priority, but address if needed)

### Causes
- Widget tree with 1000 widgets; event propagation O(N)
- Every event causes full re-layout of entire widget tree
- UI re-rendered every frame even if data unchanged

### Mitigation Strategy

**Lazy Rendering (E10)**:
- Only re-layout widgets if data changed
- Track dirty bits: widget knows if it needs layout update
- Skip layout for clean widgets

**Batch Events (E10)**:
- Accumulate input events for 1 frame; process in batch
- Avoids multiple layout passes per frame

**Event Propagation (E10)**:
- Use event capture/bubble model; stop propagation at first handler
- Avoid traversing entire widget tree for every event

**Profiling (E10)**:
- Measure time for layout, event dispatch, rendering separately
- If UI time > 2ms/frame: optimize (lazy rendering, batching)

### Acceptance Criteria
- [ ] UI responds to input within 1 frame (< 16ms latency)
- [ ] Widget tree with 100+ widgets renders at 60 FPS
- [ ] Layout time < 1ms per frame (lazy rendering reduces cost)

---

## Risk Monitoring & Escalation

**Weekly Check-In** (end of each sprint):
- Review Risk Register
- Update Probability/Impact based on progress
- Escalate any risk with Score > 15 (Critical or High)
- Adjust mitigation plan if needed

**Escalation Procedure**:
1. Risk score > 15: discuss with team; plan mitigation
2. Risk score > 20 AND mitigation not in progress: halt other work; focus on mitigation
3. Mitigation fails: call for emergency meeting; consider feature cuts or timeline extension

**Risk Closure**:
- Risk closed when acceptance criteria all met AND no regression in 3+ weeks
- Document lessons learned in post-project review
