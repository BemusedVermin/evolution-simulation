# Beast Evolution Game: Planning Documentation

This directory contains the **design-intent** planning documents for the Beast Evolution Game: scope, risks, dependency graph, branch options, milestone definitions. It is the reference you read when deciding *what* to build and *why*.

> **Live tracking is on GitHub, not in these markdown files.**
>
> - **[Project board](https://github.com/users/BemusedVermin/projects/1)** — the authoritative view of sprint/story/epic status (Sprint, Phase, Points, Status fields).
> - **[Sprint epics](https://github.com/BemusedVermin/evolution-simulation/issues?q=is%3Aissue+label%3Aepic)** — one per sprint (S1–S18), each with a story checklist derived from `SPRINTS.md`.
> - **Story issues** — opened per sprint via the Feature task template, labelled `story` + `sprint:sN` + `crate:*`, referencing the sprint epic.
>
> Status tables and Story tables in `EPICS.md` / `SPRINTS.md` reflect planned scope, not progress. **Do not edit them to reflect current progress** — update the board and issues instead. Scope *changes* (deferred stories, re-sized points, new risks) still land in these docs via PR.

---

## Documents Overview

### 1. IMPLEMENTATION_PLAN.md (73 KB) — **START HERE**
**Complete 1500-2500 line implementation plan covering:**
- Executive summary (total scope, sprint cadence, MVP definition)
- 14 detailed epics (E1–E14) with descriptions, dependencies, estimates, and DOD
- 14 sprint plans (S1–S14, MVP phase) with detailed stories, acceptance criteria, and risks
- 4 branching deep system options (Evolution, Disease, Economy, Culture) for S15–S18
- Risk register (top 10 risks with probability, impact, mitigation)
- Definition of Done (project-level, MVP, deep system)
- 5 exemplar story templates showing quality bar
- 5 major milestones and demos
- Velocity adjustment and replanning guidance

**Use this for:**
- Understanding complete scope and timeline
- Breaking down work for team
- Tracking progress against plans
- Reference for architectural decisions

---

### 2. EPICS.md (20 KB) — **Epic Tracking & Details**
**Detailed epic definitions with:**
- All 14 epics (E1–E14) with components, business value, dependencies
- Success criteria and definition of done for each epic
- Epic health tracking matrix (status, confidence, blockers)
- Risk profile per epic
- Health tracking section for velocity and burn-down

**Use this for:**
- Understanding what each epic delivers
- Tracking epic-level progress
- Identifying blockers between epics
- Reporting epic completion to stakeholders

---

### 3. SPRINTS.md (24 KB) — **Sprint Planning & Execution**
**Detailed sprint breakdown covering:**
- Full sprint calendar (S1–S14 MVP, S15–S18 deep systems)
- Sprint-by-sprint story breakdown with tables
- Points, owner, and status for each story
- Demo criteria and exit definitions of done for each sprint
- Detailed deep system branching plans (4 options, 3-4 sprints each)
- Dependency graph (critical path, parallel opportunities)
- Velocity tracking template
- Milestone gates and DoR/DoD checklists

**Use this for:**
- Planning individual sprints
- Daily standup reference
- Tracking story completion
- Understanding sprint dependencies
- Identifying parallel work opportunities

---

### 4. RISK_REGISTER.md (22 KB) — **Risk Management & Mitigation**
**Comprehensive risk register covering:**
- Top 10 risks by probability × impact score
- Risk summary table with all risks scored
- Detailed analysis for each risk (causes, mitigation strategy, acceptance criteria)
- Specific examples and safeguards
- Risk monitoring and escalation procedures
- Acceptance criteria for risk closure

**Key risks covered:**
1. Determinism divergence at scale (CRITICAL)
2. Performance budget exceeded (HIGH)
3. Mutation operators invalid genomes (HIGH)
4. Complex composition hooks slowdown (MEDIUM–HIGH)
5. Procedural meshes degenerate (MEDIUM)
6. Label quality poor (LOW–MEDIUM)
7. Combat balance broken (HIGH)
8. ECS overhead (LOW)
9. Save incompatibility (LOW)
10. UI event handling lag (LOW)

**Use this for:**
- Risk mitigation planning
- Early warning signs
- Escalation triggers
- Test strategy for high-risk areas
- Sprint-by-sprint risk focus

---

## How to Use These Documents

### For Solo Dev + Claude Planning
1. **Read IMPLEMENTATION_PLAN.md** section 0 (Executive Summary) for overview
2. **Skim EPICS.md** to understand 14 epics
3. **Follow SPRINTS.md** for sprint-by-sprint breakdown
4. **Check RISK_REGISTER.md** at start of each sprint to identify focus areas

### For Sprint Planning
1. Pick sprint from SPRINTS.md (e.g., S3: Genome & Mutation)
2. Read all stories in that sprint (points, owner, status)
3. Break stories into 1–4 hour Claude-pairable tasks
4. Check dependencies in SPRINTS.md Dependency Graph
5. Reference RISK_REGISTER.md for sprint-specific risks

### For Monitoring Progress
1. Move issues across columns on the [Project board](https://github.com/users/BemusedVermin/projects/1) as work progresses; tick story checkboxes in the sprint epic when a story lands
2. Track velocity: sum the `Points` field for items in the `Done` column at sprint end
3. Compare actual velocity to baseline 40 pts/sprint
4. If velocity drops, adjust timeline (see IMPLEMENTATION_PLAN.md "Velocity & Adjustment")
5. Escalate any risk with score > 15 (see RISK_REGISTER.md "Risk Monitoring")

Do **not** edit `SPRINTS.md` or `EPICS.md` status columns to track progress — those tables are historical scope, not live state.

### For Deep System Branching (Post-MVP)
1. At end of S14, choose one of four options (A/B/C/D)
2. Reference SPRINTS.md "Phase 4: Deep System Branching" for chosen option
3. Detailed sprint breakdowns provided for all 4 options
4. Continue tracking velocity, risks, and milestones in same format

---

## Key Metrics & Targets

### Scope
- **MVP**: 480 points = ~12 sprints = ~3.5 months (solo dev + Claude)
- **Deep System**: 150 points = ~3.75 sprints = ~1 month additional
- **Total Horizon**: ~630 points = ~15–16 weeks

### Velocity
- **Baseline**: 40 points/sprint (1 week part-time solo + Claude)
- **Sprint Sprint**: 1–4 hours/story (Claude-pairable)
- **Adjustment**: ±10% velocity triggers timeline extension/compression

### Determinism
- **Target**: Bit-identical replay validation after 1000+ ticks
- **Testing**: Determinism test runs on every commit; failure blocks merge
- **Validation**: Save at tick 50 → Replay ticks 1–100 → All hashes match

### Performance
- **Target**: 60 FPS sustained (16ms/tick)
- **Budget per stage**: ~2ms/stage (8 stages × 2ms = 16ms)
- **Profiling**: Per-system timing logged; budget overruns trigger deferred systems

### Risks
- **Critical (Score > 15)**: Determinism divergence (R1), Performance budget (R2), Combat balance (R7)
- **High (Score 10–15)**: Mutation bounds (R3), Composition slowdown (R4), Mesh quality (R5)
- **Medium (Score 5–10)**: Label quality (R6), ECS overhead (R8), Save compat (R9), UI lag (R10)

---

## Planning Artifacts

### Architecture References (Read FIRST)
- `/architecture/IMPLEMENTATION_ARCHITECTURE.md` — System design & crate layout
- `/architecture/ECS_SCHEDULE.md` — 8-stage tick loop specification
- `/architecture/CRATE_LAYOUT.md` — Workspace structure (17 crates)
- `/INVARIANTS.md` — Critical constraints (determinism, mechanics-label separation)

### Domain References
- `/systems/01_evolutionary_model.md` — Core genetics & fitness
- `/systems/02_trait_system.md` — Channels & phenotype expression
- `/systems/06_combat_system.md` — Formation & combat mechanics
- `/systems/11_phenotype_interpreter.md` — Genotype → primitive effects
- `/systems/16_disease_parasitism.md` — Pathogen coevolution
- `/systems/22_master_serialization.md` — Save/load & determinism
- `/systems/23_ui_overview.md` — UI modes & interaction

---

## Project Lifecycle

### Phase 1: Foundations & Core Sim (S1–S4, Weeks 1–4)
Build numerical primitives (fixed-point, PRNG), registries, genetics, interpreter.

### Phase 2: Evolution & ECS (S5–S9, Weeks 5–9)
Implement ECS, tick loop, save/load, world gen, rendering.

### Phase 3: UI & Combat (S10–S14, Weeks 10–14)
Add UI framework, combat, chronicler labels. **MVP Ships at end of S14.**

### Phase 4: Deep System (S15–S18, Weeks 15–18)
Choose one deep system (Evolution/Disease/Economy/Culture) and develop for 4 sprints.

---

## Success Criteria

### MVP (End of S14)
- [ ] All 13 MVP epics (E1–E13) completed
- [ ] 50+ creatures in playable world
- [ ] 1000-tick determinism validated (bit-identical replay)
- [ ] Bestiary shows 20+ discovered creatures with emergent labels
- [ ] One full encounter playable (5 friendly, 3 enemy, 10 combat rounds)
- [ ] Save/load functional; game persists and resumes correctly
- [ ] 60 FPS sustained on world map and encounter views
- [ ] CI passes: cargo build, cargo test, clippy clean
- [ ] Documentation complete (README, architecture guide, dev onboarding)

### Deep System (End of S18)
- [ ] Chosen deep system fully integrated
- [ ] New mechanics demonstrated in 1000-tick gameplay
- [ ] Determinism preserved with new systems
- [ ] New content visible in-game
- [ ] Documented and tested

---

## Reference Quick-Links

| Document | Purpose | Key Content |
|----------|---------|-------------|
| IMPLEMENTATION_PLAN.md | Complete roadmap | Epics, sprints, risks, story examples |
| EPICS.md | Epic tracking | Epic definitions, dependencies, status |
| SPRINTS.md | Sprint execution | Story breakdown, timeline, dependencies |
| RISK_REGISTER.md | Risk management | Top 10 risks, mitigation, monitoring |
| IMPLEMENTATION_ARCHITECTURE.md | System design | Crate layout, dependencies, layers |
| ECS_SCHEDULE.md | Tick loop spec | 8-stage schedule, parallelism rules |
| INVARIANTS.md | Critical constraints | Determinism, mechanics-label separation |

---

## Contact & Updates

**Planning Owner**: Solo Dev + Claude

**Last Updated**: April 14, 2026

**Next Review**: End of S1 (Week 1) — assess actual velocity; adjust sprints S2–S14 if needed

**Status**: Ready for implementation. All planning complete. Implementation begins S1.

---

## Notes for Dev

### Before Starting a Sprint
1. Open the sprint's epic issue (e.g. `Sprint S3: Genome & Mutation`) on GitHub
2. Confirm dependencies listed in the epic are closed / merged
3. Open a story issue per story in the epic's checklist using the **Feature task** template, labelled `story` + `sprint:sN` + `crate:*`, referencing the epic (`Part of #NN`)
4. Move the epic to `In Progress` on the [Project board](https://github.com/users/BemusedVermin/projects/1) and assign yourself

### During Each Sprint
1. Pick the next story issue; move it to `In Progress`
2. Work in a topic branch; open a PR using the appropriate PR template when ready
3. On merge: close the story issue, tick the matching checkbox in the sprint epic
4. At sprint end: move the epic to `Done` once the DoD checklist is complete; record velocity via the `Points` field

### Risk Management
1. At sprint start: review RISK_REGISTER.md "Sprint Risk Focus" (if provided)
2. Watch for early warning signs (listed in each risk's mitigation section)
3. If score > 15: escalate immediately; focus team effort
4. Weekly: update Risk Register with any new risks or mitigation status

---

## Good Luck!

This is a large, well-scoped project. The plan is detailed enough to execute without micromanagement, but flexible enough to adapt based on actual progress. Focus on determinism and emergent gameplay—these are the load-bearing constraints. Everything else is feature work that can be cut or extended as needed.

**Target**: MVP ships in ~14 weeks. Deep system in ~18 weeks. Beyond that: modding, multiplayer, and community-driven content.
