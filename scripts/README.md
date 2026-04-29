# Repository Scripts

Small utilities used by automation (review subagents, CI helpers). Each
script is **pre-approved** in `.claude/settings.local.json` so it runs
without a permission prompt.

## `ci-local.sh`

Run every CI gate from `.github/workflows/ci.yml` locally, in the same order, with the same flags. Mirror of:

1. `cargo fmt --check`
2. `cargo clippy --workspace --exclude beast-render --all-targets -- -D warnings`
3. `cargo test --workspace --exclude beast-render --all-targets --locked`
4. `cargo test --workspace --exclude beast-render --doc --locked`
5. `cargo clippy -p beast-render --no-default-features --features headless --all-targets -- -D warnings`
6. `cargo test  -p beast-render --no-default-features --features headless --all-targets --locked`
7. `cargo deny check`                                                   *(skipped if cargo-deny isn't installed)*
8. `cargo llvm-cov` summary                                             *(skipped if cargo-llvm-cov isn't installed)*
9. `.github/scripts/run-quality-metrics.sh`                             *(skipped if `lizard` isn't installed)*
10. `cargo build --release --workspace --exclude beast-render --locked`
11. `cargo build --release -p beast-render --headless --locked`
12. `cargo test --test determinism_test --release` *(once the M1 target lands)*

### Usage

```bash
scripts/ci-local.sh                  # fail-fast (recommended for pre-push)
scripts/ci-local.sh --keep-going     # run every gate, then list failures
scripts/ci-local.sh --quick          # skip release builds, coverage, quality
scripts/ci-local.sh --no-render      # skip the SDL3-from-source render steps
```

The script forces `RUSTFLAGS=-D warnings` and `CARGO_INCREMENTAL=0` so the local run matches CI bit-for-bit. Optional gates (cargo-deny, cargo-llvm-cov, lizard) print a clear "skipped" line when the tool isn't installed instead of failing.

---

## `post-pr-review.sh`

Post a complete GitHub PR review — overall body + every inline comment +
event verdict (`COMMENT` / `APPROVE` / `REQUEST_CHANGES`) — in a single
`gh api` POST.

**Use this instead of multi-step Python / Node calls when posting reviews
from a subagent.** The pending-review → add-comment → submit flow needs a
fresh approval per call; this is one already-approved invocation.

### Usage

```bash
scripts/post-pr-review.sh <owner> <repo> <pr> <payload.json>
```

### Payload JSON

Mirrors the GitHub
[Create a review for a pull request](https://docs.github.com/en/rest/pulls/reviews#create-a-review-for-a-pull-request)
endpoint:

```json
{
  "body": "**REQUEST_CHANGES** — round 1 — 0 CRITICAL · 1 HIGH · 2 MEDIUM · 1 LOW",
  "event": "REQUEST_CHANGES",
  "comments": [
    {
      "path": "crates/beast-ui/src/widget.rs",
      "line": 35,
      "side": "RIGHT",
      "body": "**LOW** — Add `Serialize`/`Deserialize` derives so a future widget tree save..."
    },
    {
      "path": "crates/beast-ui/src/widget/list.rs",
      "start_line": 50,
      "line": 55,
      "side": "RIGHT",
      "body": "**MEDIUM** — Multi-line comment example..."
    }
  ]
}
```

Notes:
- `line` is 1-based and refers to the line in the **new** file (`side:
  RIGHT`) by default.
- For multi-line comments, `start_line` and `line` together describe a
  range; both refer to the same `side`.
- Omit `body` if the review is purely inline; omit `comments` if it's
  purely a top-level note.

#### Picking `event`

The `event` field drives GitHub's per-reviewer review-state rollup —
it is what determines whether the PR appears as
`approved` / `changes_requested` / `commented` in the merge gate. It
MUST match the verdict the agent states in the body's first line:

| Verdict in body line | Required `event` value | When to use |
| -------------------- | ---------------------- | ----------- |
| `**APPROVE** — ...` (any round, advisory or not) | `"APPROVE"` | No CRITICAL/HIGH (round 1: also no MEDIUM). Supersedes any prior `CHANGES_REQUESTED` from the same `github-actions[bot]` reviewer. |
| `**REQUEST_CHANGES** — ...` | `"REQUEST_CHANGES"` | Any new finding at the round's blocking tier (round 1: MEDIUM+; round 2+: HIGH+). |
| `**COMMENT** — ...` | `"COMMENT"` | Explicit fallback only — diff too large to evaluate, or change is purely config without a rubric. Does NOT supersede a prior `CHANGES_REQUESTED`. |

`COMMENT` is **not** the safe default. Sending `event: "COMMENT"` while
the body says `**APPROVE**` leaves any prior `CHANGES_REQUESTED` from
the same reviewer active, blocking the merge despite the verdict —
this is the regression tracked in
[#246](https://github.com/BemusedVermin/evolution-simulation/issues/246).

`post-pr-review.sh` rejects payloads whose `event` is not one of
`APPROVE` / `REQUEST_CHANGES` / `COMMENT`, so a typo (e.g.
`"APPROVED"`) fails loudly rather than silently degrading to
GitHub's default of `PENDING`.

#### APPROVE example (clean diff or round-2 advisory)

```json
{
  "body": "**APPROVE** — round 2 advisory — 0 CRITICAL · 0 HIGH · 1 MEDIUM · 1 LOW (non-blocking)",
  "event": "APPROVE",
  "comments": []
}
```

A round-2 advisory APPROVE may still carry inline `comments[]`
entries for the non-blocking MEDIUM/LOW findings — the
`event: "APPROVE"` value is what supersedes the prior bot review.

### Author conventions for reviewer subagents

When you (a reviewer subagent) need to post a review:

1. Build the payload as JSON in a temp file:
   `mktemp -t pr-review.XXXXXX.json` (or any path under `/tmp`).
2. Call `scripts/post-pr-review.sh <owner> <repo> <pr> <payload>`.
3. Parse the JSON output for the new review id if you need it (the
   script writes the API response to stdout).

Do **not** issue separate `pull_request_review_write` + `add_comment_to_pending_review` + `submit_pending` MCP calls. Each one of those is a permission-prompt landmine; the script is one approved call.
