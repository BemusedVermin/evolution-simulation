# Contributing

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
