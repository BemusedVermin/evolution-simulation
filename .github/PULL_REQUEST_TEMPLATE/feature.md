<!--
Feature PR template. Use this for new capabilities (matches the
Feature Request / Feature Task issue templates).

Activate via: ?template=feature.md on the PR creation URL.
-->

## Summary

<!-- What new capability does this land? One paragraph, reader-facing. -->

## Linked issues

<!--
- Closes #<feature-request>
- Implements #<feature-task-1>, #<feature-task-2>, …
- Sprint: SPRINTS.md#S?
-->

## Design notes

<!--
Anything a reviewer should know about the shape of the change that
isn't obvious from the diff. API decisions, data-structure choices,
invariants added / relaxed, future-proofing deliberately deferred.
-->

## Invariant impact

<!--
See documentation/INVARIANTS.md. Tick one:

- [ ] No invariant touched.
- [ ] Determinism — describe what keeps bit-identical replay intact.
- [ ] Mechanics-label separation — confirm no named-ability logic in sim code.
- [ ] Channel registry monolithicism — all channel lookups go through the registry.
- [ ] Other invariant (explain).
-->

## Test plan

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets --locked`
- [ ] `cargo test --workspace --doc --locked`
- [ ] Feature-specific tests listed here:
  -
- [ ] `cargo deny check` (if deps changed)

## Docs

- [ ] `documentation/PROGRESS_LOG.md` session entry added.
- [ ] Crate `README.md` updated (if public surface changed).
- [ ] Inline rustdoc on new public items.

## Out of scope (deliberate)

<!-- What this PR is NOT doing. Tracked follow-ups with issue links. -->

-
