<!--
Determinism PR template. Use this for fixes to replay-hash divergences,
PRNG seeding bugs, float leaks into sim state, and iteration-order bugs.

Activate via: ?template=determinism.md on the PR creation URL.

See documentation/INVARIANTS.md §1 for the full determinism contract.
-->

## Summary

<!-- What divergence did we have? What's the fix in one sentence? -->

## Linked issue

<!-- Fixes #<determinism-regression-issue>. -->

## Divergence class

<!--
Tick the observed class:

- [ ] Same seed, same commit, same platform → different output across runs.
- [ ] Same seed, different platform (linux/windows/macOS) → different output.
- [ ] Save → load → replay doesn't match original (state-hash mismatch).
- [ ] Tick-by-tick hash diverges partway through a run.
-->

## Root cause

<!--
Determinism bugs fall into three buckets (in the order INVARIANTS.md
says to suspect them):

1. Numerical precision — float leaking into sim state, Q3232 saturation,
   rounding asymmetry.
2. PRNG seeding — cross-subsystem stream contamination, re-seeding from
   derived state, missing `long_jump()` on split.
3. Iteration order — HashMap/HashSet leaking into state, non-sorted
   iteration of entity keys.

Describe which bucket this fits (or whether it's a new class) and the
exact mechanism that made the output diverge.
-->

## Fix

<!-- What changed. Reference the specific invariant clause if applicable. -->

## Regression coverage

<!--
Determinism regressions *must* come with a hash-comparison test that:
- Runs a deterministic scenario for a fixed tick count.
- Compares the state hash against a committed golden, or runs twice
  and compares, whichever is appropriate.
- Runs on all three CI OSes.
-->

- [ ] Added hash-comparison regression test.
- [ ] Test fails on `master`, passes on this branch.
- [ ] Test runs under the cross-platform matrix (windows + macos + ubuntu).
- [ ] Test is free of wall-clock reads and OS RNG.

## Invariant confirmation

<!-- Tick everything you verified: -->

- [ ] All sim-state math uses `Q3232` (no `f32`/`f64` in the changed code path).
- [ ] PRNG usage goes through a subsystem stream (no `rand::thread_rng`, no `std::random`).
- [ ] Hot-path iteration uses `BTreeMap` / `BTreeSet` / explicitly-sorted collections.
- [ ] No wall-clock reads (`SystemTime`, `Instant`) on the sim path.

## Test plan

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets --locked`
- [ ] Cross-platform CI (windows + macos) green.
- [ ] When the formal determinism test lands in S6, this regression is included in its corpus.
