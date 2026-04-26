# Emergence Research Collection

This directory contains literature reviews and design analyses for replacing hard-coded systems with emergent, channel-based models.

## Documents

### R30: Culture, Economics, Cognition, Technology, Language, Disease, and Migration
**File:** `R30_culture_econ_etc.md`  
**Length:** ~6500 words  
**Status:** Complete  

Synthesizes recent research on:
- Iterated learning models for language emergence (Kirby, Smith, Tamariz)
- Combinatorial innovation and technology trees (Brian Arthur)
- Active inference and free energy principle for cognition (Friston)
- Predictive processing and hierarchical Bayesian agents
- Agent-based computational economics (Sugarscape, EURACE, ACE)
- Emergent monetary systems (Menger theory)
- Disease emergence in continuous antigenic space (SIR/SEIR multi-strain)
- Migration models (gravity models, agent-based frameworks)

**Recommendations:**
- Replace discrete language families with continuous phoneme/morphology channels.
- Replace tech trees with combinatorial primitives + derived compounds.
- Replace cognition tiers with continuous channels (model depth, horizon, theory-of-mind).
- Replace resource enums with continuous goods space.
- Model disease as points in antigenic space; strain competition emerges.
- Replace migration triggers with utility-driven movement + network effects.

**Implementation:** Phased rollout starting with technology and language (Phase 1), then economics, disease, cognition, and migration.

---

### R90: Voronoi, Hex, and Icosahedral Topology
**File:** `R90_voronoi_topology.md`  
**Length:** ~4000 words  
**Status:** Complete  

Evaluates five spherical grid topologies:
1. **Spherical Centroidal Voronoi Tessellations (SCVT)** — irregular, natural-looking, used in MPAS atmospheric models.
2. **Hexagonal grids** — regular, game-industry standard (Civ 6, Endless Legend).
3. **Icosahedral meshes** — highly regular triangular subdivision.
4. **HEALPix** — equal-area hierarchical diamonds (used in cosmology).
5. **Latitude-longitude grids** — simple but anisotropic at poles.

**Tradeoff matrix** covering determinism, neighbor uniformity, rendering clarity, fixed-point arithmetic, and implementation complexity.

**Recommendation:** **Icosahedral hexagonal grid**
- ✓ Fully deterministic in fixed-point arithmetic.
- ✓ Uniform 6-neighbor topology (except 12 poles).
- ✓ Proven in games and geophysics.
- ✓ Simple neighbor-graph based algorithms (no floating-point precision issues).
- ✗ Voronoi rejected unless determinism can be validated (risky; unproven in fixed-point).

**Why not Voronoi?**
- Lloyd's algorithm is iterative and convergence-dependent; fixed-point implementation unvalidated.
- Variable neighbor counts require branch-heavy algorithms.
- No mechanical or computational advantage over hex for culture/faction cellular automata.
- Game-industry precedent favors hex.

---

### RESEARCH_SYNTHESIS_SUMMARY.md
**File:** `RESEARCH_SYNTHESIS_SUMMARY.md`  
**Length:** ~500 words  
**Status:** Complete  

Executive summary covering both R30 and R90. Quick reference for:
- Key findings from each research area.
- Implementation phasing (MVP → Phases 1–4).
- Recommendation rationale.
- Next steps (validation, decision gates, Phase 1 kickoff).

---

## Sources and Citation

All documents include comprehensive references:
- **R30:** 13 primary sources (Smith 2003, Kirby et al., Arthur 2009, Friston et al., Epstein & Axtell 1996, Menger 1892, etc.).
- **R90:** 10 primary sources (Skamarock et al. MPAS, Du/Ju/Gunzburger CVT, Górski et al. HEALPix, game design references, etc.).

Full citations are provided as markdown hyperlinks in each document.

---

## Integration with Beast Core

### R30 Integration Points
1. **Channel manifests:** Add to `primitive_vocabulary/culture/` following the JSON schema.
2. **Interpreter hooks:** Stage 2 (Phenotype Resolution) reads culture channels → behavior parameters.
3. **Update logic:** Stages 1, 4, 5, 6 implement drift, transmission, innovation, diffusion.
4. **ECS schedule:** All within tick budget; validated against per-stage performance budget.

### R90 Integration Points
1. **Topology module:** New crate `beast-topology` (or fold into `beast-primitives`).
2. **Cell ID → (lat, lon) lookup:** For rendering and agent pathfinding.
3. **Neighbor graph:** Built once at world init; used by ecology, pathfinding, diffusion.
4. **Determinism:** Icosahedral hex is fully deterministic; no PRNG consumption for topology.

---

## Next Steps

1. **Validation gate:** Cross-check R30 and R90 against INVARIANTS.md and ECS_SCHEDULE.md.
2. **Design review:** Present recommendations to core team; approve phased rollout.
3. **Phase 1 implementation:**
   - Technology channels + combinatorial compounds.
   - Language channels + transmission bottleneck model.
   - Validation: test emergence on small populations over 1000+ ticks.
4. **Topology decision:** Approve icosahedral hex; begin module design.

---

**Last updated:** 2026-04-26  
**Version:** 1.0  
**Status:** Ready for design review and implementation planning.
