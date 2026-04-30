# Channel-model paper

Source for *A Mathematical Model of Channels, Carriers, and Runtime Vocabulary Genesis*. IEEE technical-report style (single-column `IEEEtran`).

## Files

- `channel_model.tex` — the manuscript. Self-contained: TikZ for the diagrams (no mermaid dependency), inline `thebibliography`, all math inline.
- `compile.bat` — Windows compile script.
- `compile.sh` — POSIX compile script.

## Compiling

The simplest path is **tectonic** — a single self-contained binary that downloads packages on first use. No full TeX distribution required, and it auto-resolves `IEEEtran` and everything else the paper needs.

Install once:

- Windows (winget): `winget install TectonicProject.Tectonic`
- macOS (homebrew): `brew install tectonic`
- Linux: see <https://tectonic-typesetting.github.io>

Then double-click `compile.bat` (or run `./compile.sh`). The script tries `tectonic` first, falls back to `latexmk`, then `pdflatex`. The output is `channel_model.pdf` next to the source.

If you already have **MiKTeX** or **TeX Live** installed, both can compile the paper too — but you may need to install `IEEEtran` if it isn't present (`tlmgr install IEEEtran` on TeX Live; the MiKTeX Console will prompt on first run).

## Packages used

Standard CTAN: `IEEEtran`, `geometry`, `amsmath`, `amssymb`, `amsthm`, `microtype`, `hyperref`, `cleveref`, `tikz` (with `positioning`, `arrows.meta`, `shapes.geometric`, `calc`, `fit`, `backgrounds` libraries), `booktabs`, `tabularx`, `longtable`, `ragged2e`, `enumitem`, `xcolor`. Tectonic resolves these automatically.

## Structure

1. **Background** — motivation and prior work (gene-regulatory networks, gameplay attribute systems, open-ended evolution).
2. **Notation** — comprehensive symbol table organised by topic (mathematical primitives, carriers, channels, state, gates, composition, drift, primitives/emissions, registries, genesis, revision).
3. **Channels and Carriers** — formal definitions.
4. **The State Space** — instance and world state.
5. **Gates and Effective Values** — gate kinds, activeness, effective value.
6. **Composition** — the within-tick fold.
7. **Drift** — the between-tick update.
8. **Primitives, Emission, and Merge** — world variables, primitives, the merge monoid.
9. **The Per-Tick State-Transition Operator** — Figure 1; formal $\mathcal{T}$.
10. **Runtime Vocabulary Genesis** — Figure 2; the four-step pipeline; lineage-closure theorem; **Retrospective Revision** as a subsection extending the lineage to branching forests.
11. **Properties** — determinism, monolithicism, emergence closure, mechanics--label separation.
12. **Design Rationale** — long table of choices and alternatives.
13. **Open Questions** — fitness function, semantic-distance metric, ancestor reactivation.
14. **Appendix A: Distributed Implementation (Sharded Lineage with Deterministic Merge)** — concrete algorithm for parallelising the model across multi-core CPU, distributed cluster, and hybrid CPU+GPU. Pseudocode for per-shard tick, deterministic merge tree, and distributed genesis with LSH bucketing and lex-sorted admission. Includes a determinism theorem (Theorem 3) showing replay invariance under arbitrary parallelism degree, complexity tables, and a hardware-tier mapping.
15. **References** — IEEE-style numeric citations.

## Notes

- The paper uses `\cref` cross-references throughout. Compile twice (or use `latexmk`/`tectonic`, both of which handle multi-pass automatically).
- Theorems are numbered sequentially across the paper: Theorem 1 is lineage closure under genesis; Theorem 2 is lineage closure under revision.
- The notation table spans multiple pages via `longtable`; this is expected and handled automatically.
