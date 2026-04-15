<!--
Bug-fix PR template. Use this for fixes that address a filed bug or an
unreported defect noticed in review.

Activate via: ?template=bug_fix.md on the PR creation URL.
-->

## Summary

<!-- One sentence: what was broken, how is it fixed? -->

## Linked issue

<!-- Fixes #<bug-id>. If there's no issue yet, describe the bug here with repro. -->

## Root cause

<!--
Walk through the actual failure mode. Not "X was wrong" — "X was wrong
because we assumed Y and Z never happens, but it does when …".
Reviewers should be able to reason about whether the fix addresses the
real cause or only a symptom.
-->

## Fix

<!-- What changed and why that change addresses the root cause above. -->

## Regression coverage

<!--
A bug fix without a failing-before / passing-after test is a recipe
for the same bug reappearing. Link or inline the new test.
-->

- [ ] Added a regression test that fails on `master` and passes on this branch.
- [ ] Test is deterministic (no timing, no random without explicit seed).
- [ ] If this is a determinism-related fix, verify the test runs on all three CI OSes.

## Invariant impact

<!--
- [ ] No invariant touched (behaviour was already inside the contract).
- [ ] Tightened an invariant (what was silently relaxed, what now enforces it).
- [ ] Touches determinism — describe what changes in state/output, if anything.
-->

## Risk assessment

<!--
How confident are you that this doesn't introduce a new bug?
- What else called the buggy code path?
- Any behaviour change for callers that weren't hitting the bug?
-->

## Test plan

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets --locked`
- [ ] Regression test from above — demonstrably fails on `master`.
