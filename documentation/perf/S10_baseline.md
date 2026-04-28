# Sprint S10 — Chronicler Ingest Performance Baseline

> Tracking issue: [#210](https://github.com/BemusedVermin/evolution-simulation/issues/210) (S10.5).
> Run the perf test locally with:
>
> ```bash
> cargo test -p beast-chronicler --release --test cross_instance_determinism \
>     -- --ignored --nocapture ingest_1000_entities_100_ticks_under_50ms
> ```

This file records baseline numbers for the `Chronicler::ingest` hot path
so future PRs can regression-check against them. The dev-box numbers
below are from the same machine that authored the S9 baseline (Windows
11, x86_64-pc-windows-msvc, rustc 1.94.1, default `[profile.release]`).
The test is `#[ignore]`-gated so it doesn't run in the default
`cargo test` sweep — debug builds blow the budget by a wide margin.

## Pass criteria (S10.5)

The S10.5 issue body sets a hard budget: **1000 entities × 100 ticks
ingested in under 50 ms in release**. That number is encoded in the
test's `assert!`, so it's both the design constraint and the regression
gate.

| Test | Frame-budget limit | Regression gate | Why |
|---|---|---|---|
| `ingest_1000_entities_100_ticks_under_50ms` | < 50 ms / 100k snapshots | **< 50 ms** (issue-body budget) | Tighter than the 10× rule used in S9 because the issue body is the contract; the assert is the gate. |

> The 100k-snapshot run corresponds to a worst-case 100-tick chronicler
> sweep over a 1k-creature world. Real runs sample at lower cadence —
> see [`systems/09_world_history_lore.md`](../systems/09_world_history_lore.md)
> §3.2 — so this is the design ceiling, not the per-tick cost.

## Baseline (S10.5 dev-box, 8 release runs)

| Test | Min | Median | Mean | Max | Regression gate | Headroom-vs-gate (mean) |
|---|---|---|---|---|---|---|
| `ingest_1000_entities_100_ticks_under_50ms` | **13.2 ms** | **15.4 ms** | **16.0 ms** | **19.4 ms** | 50 ms | ≈ 3.1× |

Raw samples: 13.2, 14.7, 15.1, 15.4, 15.4, 17.4, 17.5, 19.4 ms.

Headroom is tighter than the S9 render benches (~10–12×). That's
expected: this is an end-to-end ingest test, not a tight-loop
microbenchmark, and the 50 ms ceiling comes from the issue body rather
than `~10× current mean`. If a perf-positive PR drops the mean
meaningfully, **don't** tighten the assert below the issue-body budget
without updating issue #210 or its successor.

## What's not measured here

* **Per-tick chronicler cost in a live tick loop** — needs the S6 sim
  loop to wire `PrimitiveSnapshot` emission into the schedule. Tracked
  with the rest of the tick-budget work in epic #22 (Phase 3 wrap-up).
* **`cluster()` query latency** — read-side perf, not ingest. Becomes
  load-bearing once the UI starts pulling from it (S10.7); add a bench
  there.
* **Persistence cost** — chronicler index is in-memory only until S12.

## How to compare against this baseline

1. Run the command at the top of this file on your local box.
2. Compare the printed `ingested 100000 snapshots in …ms` line to the
   `Mean` column above.
3. **Regression policy** — same shape as S9:
   * < 10 % slower → noise, no action.
   * 10 – 50 % slower → flag in PR description, gather a second sample.
   * \> 50 % slower **or** any sample over the 50 ms gate → block merge
     until investigated.
4. **Improvement policy**: speed-ups should be flagged in PR description
   so the table can be refreshed; the regression gate stays at the
   issue-body budget unless the issue itself is amended.

## How to update this file

If a perf-positive PR drops the mean meaningfully, update the table in
the same PR. The regression gate is the issue-body budget, not the
current best — leave it alone unless the issue is amended.
