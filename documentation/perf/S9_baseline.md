# Sprint S9 — Render Pipeline Performance Baseline

> Tracking issue: [#187](https://github.com/BemusedVermin/evolution-simulation/issues/187).
> Run criterion locally with:
>
> ```bash
> cargo bench -p beast-render --no-default-features --features headless
> ```

This file records baseline numbers for the headless render pipeline so
future PRs can regression-check against them. The dev-box numbers below
are from the machine that authored S9.8 (Windows 11, x86_64-pc-windows-msvc,
rustc 1.94.1, opt-level=3 + LTO=thin per `[profile.bench]`); CI numbers
will land once the SDL3 system deps are sorted on the runners
([#192](https://github.com/BemusedVermin/evolution-simulation/issues/192)).

## Pass criteria (S9.8)

The S9 issue body sets *frame-budget* limits (1 ms / 200 µs / 1 µs).
Those are the absolute design constraints. The thresholds below are
**regression gates** — set ~10× the current observed mean so the gate
catches a pathological 10× regression but tolerates ordinary noise on
slower CI hardware. If a perf-positive PR drops the mean meaningfully,
tighten the threshold in the same PR.

| Bench | Frame-budget limit | Regression gate | Why |
|---|---|---|---|
| `compile_blueprint/typical_phenotype` | < 1 ms | **< 50 µs** | ~11× current mean; flags any 10× slowdown immediately. |
| `rig_animations/typical` | < 200 µs | **< 25 µs** | ~12× current mean. |
| `animator_sample/walk_mid_t` | < 1 µs | **< 2.5 µs** | ~11× current mean. Per-frame 200-creature sample budget = 12 µs total. |

> 16.6 ms / 25 ms p99 frame-time targets from the issue body apply to the
> SDL render benches (`bench_world_map_200_creatures`,
> `bench_encounter_5_creatures`); those land alongside the renderers in
> S9.3 / S9.4 and reuse this file's structure.

## Baseline (S9.8 dev-box, criterion mean)

| Bench | Mean | p95-ish (high) | Regression gate | Headroom-vs-gate |
|---|---|---|---|---|
| `compile_blueprint/typical_phenotype` | **4.55 µs** | 4.61 µs | 50 µs | ≈ 11× |
| `compile_blueprint/random_phenotype` | **4.35 µs** | 4.61 µs | 50 µs | ≈ 11× |
| `rig_animations/typical` | **2.13 µs** | 2.19 µs | 25 µs | ≈ 12× |
| `animator_sample/walk_mid_t` | **222 ns** | 226 ns | 2.5 µs | ≈ 11× |
| `animator_sample/walk_t_zero` | **48 ns** | 49 ns | 2.5 µs | ≈ 52× |

Taken with `--warm-up-time 1 --measurement-time 2 --sample-size 30` for
stability without burning a long bench window. Default criterion (5s warm-up,
10s measurement, 100 samples) reports tighter intervals; run that locally
when investigating real regressions.

## What's not measured here

* **World-map render frame** (`bench_world_map_200_creatures`) — needs the
  `WorldMapRenderer` from S9.3. **Tracked as
  [#TBD-world-map-bench]** when the renderer lands.
* **Encounter-view render frame** (`bench_encounter_5_creatures`) — needs
  the `EncounterRenderer` from S9.4. **Tracked as
  [#TBD-encounter-bench]** when the renderer lands.
* **Per-tick HUD** (F3 frame-time overlay) — UI affordance, not a CI gate.
  Will live in `beast-app` when that crate exists.

## How to compare against this baseline

1. Run `cargo bench -p beast-render --no-default-features --features headless`
   on your local box.
2. Compare your `time:` line for each bench to this file's `Mean` column.
3. **Regression policy**:
   * < 10 % slower → noise, no action.
   * 10 – 50 % slower → flag in PR description, gather a second sample.
   * > 50 % slower → block merge until investigated.
4. **Improvement policy**: speed-ups should also be flagged. If a bench
   gets 2× faster the threshold may need re-tightening.

## How to update this file

If a perf-positive PR drops the mean meaningfully, update the table in
the same PR. Keep the threshold column conservative — it's the budget,
not the current best.
