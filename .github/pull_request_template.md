<!--
Default PR template. GitHub picks one of the specialised templates under
.github/PULL_REQUEST_TEMPLATE/ when you pass `?template=<name>.md` on
the PR creation URL — e.g.
https://github.com/BemusedVermin/evolution-simulation/compare/master...my-branch?template=bug_fix.md

If your PR is a clean fit for Feature, Bug Fix, Security, or
Determinism, prefer the specialised template. Use this one for
anything else (infra, docs, CI tweaks, refactors).
-->

## Summary

<!-- One or two sentences. What changes, and why? -->

## Changes

<!-- Bullet list of the meaningful edits. File paths welcome. -->

-
-

## Linked issues

<!-- Closes #N / Part of #N / See #N. Or "none" if this is drive-by infra. -->

## Invariant impact

<!--
See documentation/INVARIANTS.md. Tick one.

- [ ] No invariant touched.
- [ ] Touches determinism (fixed-point, sorted iteration, PRNG streams).
- [ ] Touches mechanics-label separation.
- [ ] Touches channel registry monolithicism.
- [ ] Touches another invariant (explain below).
-->

## Test plan

<!--
Concrete commands a reviewer can run. Check each off as you verify.

- [ ] cargo fmt --all -- --check
- [ ] cargo clippy --workspace --all-targets -- -D warnings
- [ ] cargo test --workspace --all-targets --locked
- [ ] cargo test --workspace --doc --locked
- [ ] cargo deny check (if deps changed)
- [ ] Any manual verification steps the reviewer can't infer from tests
-->

## Notes for reviewer

<!-- Anything non-obvious: trade-offs taken, follow-ups deferred, surprising diffs. -->
