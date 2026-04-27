# Repository Scripts

Small utilities used by automation (review subagents, CI helpers). Each
script is **pre-approved** in `.claude/settings.local.json` so it runs
without a permission prompt.

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
  "body": "## Review summary\n\n- 1 HIGH, 2 MEDIUM, 1 LOW\n- ...",
  "event": "COMMENT",
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
- Set `event` to `COMMENT` unless the agent is explicitly authorized to
  approve / request-changes.

### Author conventions for reviewer subagents

When you (a reviewer subagent) need to post a review:

1. Build the payload as JSON in a temp file:
   `mktemp -t pr-review.XXXXXX.json` (or any path under `/tmp`).
2. Call `scripts/post-pr-review.sh <owner> <repo> <pr> <payload>`.
3. Parse the JSON output for the new review id if you need it (the
   script writes the API response to stdout).

Do **not** issue separate `pull_request_review_write` + `add_comment_to_pending_review` + `submit_pending` MCP calls. Each one of those is a permission-prompt landmine; the script is one approved call.
