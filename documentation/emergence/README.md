# Emergence (v2 Design Track)

This folder contains the **post-MVP `v2` design track** for Beast Evolution Game. Its purpose is to systematically replace every hardcoded enum, lookup table, threshold, and template in the v1 system specs (`documentation/systems/01–23`) with **emergent dynamics** built on the existing channel/carrier/registry abstraction in `documentation/core-model/`.

The project rule is that the simulation comes first and gameplay emerges from it. The v1 specs took us most of the way; this folder finishes the job by removing the remaining designer-imposed taxonomies (biome enums, faction archetypes, tech-tree nodes, language families, disease classes, opinion dimensions, material types, …) in favor of continuous channel spaces and post-hoc Chronicler labeling.

> **Invariant alignment.** Every design in this folder honors `documentation/INVARIANTS.md` — determinism (Q32.32 fixed-point, sorted iteration, one PRNG stream per subsystem), mechanics-label separation, channel-registry monolithicism, emergence closure, scale-band unification, and UI-state-vs-sim-state separation.

---

## Reading order

| # | Document | Purpose |
|---|----------|---------|
| 0 | [00_MASTER_SYNTHESIS.md](00_MASTER_SYNTHESIS.md) | Design philosophy, unifying architecture, master tradeoff matrix. **Start here.** |
| 1 | [01_HARDCODED_AUDIT.md](01_HARDCODED_AUDIT.md) | Catalogue of every hardcoded/scripted aspect found in the v1 specs. |
| 2 | [10_environment_emergence.md](10_environment_emergence.md) | Pillar 1: continuous environmental fields (climate, atmosphere, ocean, hydrology). |
| 3 | [20_materials_emergence.md](20_materials_emergence.md) | Pillar 2: emergent materials (mineralogy, soils, weathering). |
| 4 | [30_biomes_emergence.md](30_biomes_emergence.md) | Pillar 3: biomes as emergent labels over channel clusters (replaces `biome_type` enum). |
| 5 | [40_ecology_emergence.md](40_ecology_emergence.md) | Pillar 4: emergent food webs, niches, energy flow. |
| 6 | [50_social_emergence.md](50_social_emergence.md) | Pillar 5a: factions, kinship, governance, coalitions emerge from continuous policy spaces. |
| 7 | [60_culture_emergence.md](60_culture_emergence.md) | Pillar 5b: language, technology, economy, cognition, disease, migration. |
| 8 | [90_topology_decision.md](90_topology_decision.md) | Voronoi vs. icosahedral-hex compatibility analysis and recommendation. |
| 9 | [99_integration_with_v1.md](99_integration_with_v1.md) | Map showing which v2 emergence pillars supersede which v1 systems 01–23. |

The `research/` subfolder contains the supporting literature syntheses (R10, R20, R30, R90).

---

## Design philosophy in one sentence

> *Everything is a channel: environmental fields, material composition, social policies, language structure, cognitive parameters. Discrete categories are post-hoc Chronicler labels assigned to clusters in continuous channel space, never primary state.*

This is a strict generalization of the existing `INVARIANT 2: Mechanics-Label Separation` from creature abilities to every other taxonomy in the simulation.

---

## Status

- v2 design track. **Not yet integrated** into the sprint plan or GitHub project board.
- Migration path from v1 → v2 is intentionally deferred per project-owner direction.
- Once an emergence pillar is approved, its v1 counterpart in `documentation/systems/` will get a "Superseded by emergence/Nx" header and its sprint allocation will be revisited.
