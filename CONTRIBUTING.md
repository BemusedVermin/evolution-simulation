# Contributing

## Tracking work

All work is tracked on GitHub, not in the markdown planning docs.

- **[Project board](https://github.com/users/BemusedVermin/projects/1)** — single view of every sprint with Sprint / Phase / Points / Status fields.
- **Sprint epic issues** ([`label:epic`](https://github.com/BemusedVermin/evolution-simulation/issues?q=is%3Aissue+label%3Aepic)) — one per sprint (S1–S18) with the story checklist, demo criteria, DoD, and invariant notes.
- **Story issues** — opened when a sprint starts, via the **Feature task** issue template. Label each story with `story`, `sprint:sN`, and the relevant `crate:*` (e.g. `crate:beast-genome`). Reference the sprint epic (`Part of #NN`) so it shows up under the epic's task list.
- **Bugs, features, security reports, determinism regressions** — use the dedicated issue templates (blank issues are disabled).

The markdown docs under `documentation/planning/` (`SPRINTS.md`, `EPICS.md`, `RISK_REGISTER.md`) document *design intent*: scope, risks, branch options. Do **not** edit their status columns to reflect current progress — the board is the source of truth. Design-intent changes (scope, deferred stories, new risks) still land in those docs via PR.

`documentation/PROGRESS_LOG.md` remains a narrative diary: decisions taken, pitfalls, commit references. It is historical context, not a status tracker.

## Development setup

1. Clone the repo.
2. Activate the repo-tracked git hooks (**one-time, per clone**):
   ```
   git config core.hooksPath .githooks
   ```
   This enables `pre-push`, which blocks direct pushes to `master`. Work on a topic branch and open a PR instead.

## Branch policy

- `master` is the integration branch. Every change lands via a pull request.
- Topic branches are named by intent: `sprint-sN-scope`, `fix-...`, `ci-...`.
- CI (`.github/workflows/ci.yml`) must be green before merging. The gating jobs are:
  - `lint-and-test (ubuntu)` — `cargo fmt --check`, `clippy -D warnings`, full test suite, doctests.
  - `test (windows-latest)` / `test (macos-latest)` — cross-platform determinism sanity.
  - `release-build (ubuntu)` — `cargo build --release`.

## Push protection

- **Client-side** (this repo): the `pre-push` hook in `.githooks/` rejects direct pushes to `master`. Activation is opt-in via `git config core.hooksPath .githooks`. Bypass with `git push --no-verify` only in genuine emergencies, and note the reason in the next commit.
- **Server-side** (GitHub): branch protection rules require either a public repo or a GitHub Pro subscription. This repo is currently private/free, so server-side enforcement is not in place. Making the repo public, or subscribing to Pro, would let us require the CI checks server-side — see `.github/workflows/ci.yml` for the exact job names to register as required checks.

## Commit style

- Conventional commit prefixes: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci`.
- Subject under 72 characters; wrap the body at 80.
- Explain *why* in the body; the diff already shows *what*.

## Local verification before pushing

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
```

All four are run by CI on every PR, but running them locally first saves a round trip.
