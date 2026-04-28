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
| ~~6.5~~ | ~~[55_multi_affiliation.md](55_multi_affiliation.md)~~ | **Superseded by 56.** Original idea: declared registry of group prototypes (kinship, settlement, polity, …) plus declared containment rules. Rejected on review for still authoring the hierarchy and the relationship-type vocabulary. Kept for design-evolution traceability. |
| 6.6 | [56_relationship_graph_emergence.md](56_relationship_graph_emergence.md) | Pillar 5a-extension (current). Replaces Agent.faction_id with a typed agent-pair multigraph driven by inter-agent primitive emissions. Edge-first link communities + hierarchical Leiden produce nested + overlapping clusters; cluster signatures (both edge-clusters and node-clusters) flow through the P7 naming pipeline, so labels like "kinship", "fealty", "kingdom", "guild" are emergent — etic galleries supply one of five candidate name sources, never the canonical name. Kingdoms emerge; they are not declared. |
| 6.7 | [57_agent_ai.md](57_agent_ai.md) | Pillar 6d-extension. Specifies the full agent AI architecture on top of P6d's seven cognitive channels: discrete factored POMDP, variational message passing for perception, MCTS-EFE for planning, reduced-nested theory-of-mind capped at depth 3, flat preference-channel registry, action-skill macros for hierarchical planning, smooth sapience scaling. One engine — same code path runs pathogen → beast → sapient via parameter degeneracy. Subsumes BT/GOAP/utility-AI/FSM as parameter-degenerate special cases. Strict Q32.32 with bounded per-agent runtime/space and graceful budget-exhaustion fallback. |
| 6.8 | [58_channel_genesis.md](58_channel_genesis.md) | Closes the last authoring carve-out. Channels themselves (preferences, factors, skills, primitives, cluster-label-gallery entries) are *born* at runtime from per-agent EFE-residual latent pressure that crosses a population threshold, via three combined mechanisms (combinatorial composition over registered operators per channel kind à la Brian Arthur 2009 + Lake 2015 BPL; latent-slot extraction à la Indian Buffet Process; schema mutation). New channels are registered with `genesis:<src_pop>:<tick>:<kind>:<sig_hash>` provenance (already reserved in the manifest schema); propagate via P6a iterated learning across populations; named by the P7 pipeline; bounded by Quality-Diversity per-niche archives + activity-score GC. Satisfies the four Soros-Stanley necessary conditions for open-ended evolution. |
| 7 | [60_culture_emergence.md](60_culture_emergence.md) | Pillar 5b: language, technology, economy, cognition, disease, migration. |
| 8 | [70_naming_and_discovery.md](70_naming_and_discovery.md) | Pillar 7: naming, discovery, and player presence. The player as in-world speaker; names propagate via P6a iterated learning. Replaces the `"uncategorised"` UX gap and the implicit "every named-thing has one canonical designer-authored name" assumption. |
| 9 | [90_topology_decision.md](90_topology_decision.md) | Voronoi vs. icosahedral-hex compatibility analysis and recommendation. |
| 10 | [99_integration_with_v1.md](99_integration_with_v1.md) | Map showing which v2 emergence pillars supersede which v1 systems 01–23. |

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
