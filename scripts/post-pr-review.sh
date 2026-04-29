#!/usr/bin/env bash
# Post a complete GitHub PR review — overall body + every inline comment +
# event verdict — in a *single* atomic API call.
#
# Why this exists:
#   The rust-reviewer subagents otherwise fall back to multi-step Python /
#   Node scripts that each need a fresh approval. This script wraps the
#   one-shot `POST /repos/{owner}/{repo}/pulls/{pr}/reviews` endpoint so
#   the entire review lands in one already-approved `gh api` call.
#
# Usage:
#   scripts/post-pr-review.sh <owner> <repo> <pr> <payload.json>
#
# Payload JSON shape (per https://docs.github.com/en/rest/pulls/reviews):
#   {
#     "body": "Overall summary text...",
#     "event": "COMMENT",                     // or APPROVE / REQUEST_CHANGES
#     "comments": [
#       {
#         "path": "crates/foo/src/bar.rs",
#         "line": 42,                          // 1-based; line in the new
#                                              // file unless `side: LEFT`.
#         "side": "RIGHT",                     // optional; defaults to RIGHT
#         "body": "Inline comment text..."
#       },
#       {
#         "path": "...",
#         "start_line": 10,
#         "line": 14,
#         "side": "RIGHT",
#         "body": "Multi-line comment..."
#       }
#     ]
#   }
#
# Exit codes:
#   0   success — review posted, prints the new review JSON to stdout.
#   64  bad CLI usage.
#   65  payload `event` field missing or not in
#       {APPROVE, REQUEST_CHANGES, COMMENT}.
#   66  payload file missing.
#   non-zero (other) — `gh api` failure; stderr is forwarded.

set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <owner> <repo> <pr> <payload.json>" >&2
  exit 64
fi

OWNER="$1"
REPO="$2"
PR="$3"
PAYLOAD="$4"

if [[ ! -f "$PAYLOAD" ]]; then
  echo "error: payload file not found: $PAYLOAD" >&2
  exit 66
fi

# Validate the `event` field. GitHub's review-state rollup treats
# anything other than APPROVE / REQUEST_CHANGES / COMMENT as PENDING,
# which silently leaves the PR in whatever state the prior review put
# it in — observed on PR #240 round 2, where the agent posted with
# `event: COMMENT` for an APPROVE verdict and the prior round-1
# CHANGES_REQUESTED stayed active (issue #246). A loud failure here
# is much better than a silent merge-block.
event=$(jq -r '.event // empty' "$PAYLOAD")
case "$event" in
  APPROVE|REQUEST_CHANGES|COMMENT) ;;
  "")
    echo "error: payload is missing the 'event' field; must be one of APPROVE / REQUEST_CHANGES / COMMENT" >&2
    exit 65
    ;;
  *)
    echo "error: payload 'event' is '$event'; must be one of APPROVE / REQUEST_CHANGES / COMMENT (see scripts/README.md)" >&2
    exit 65
    ;;
esac

gh api \
  --method POST \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  "repos/$OWNER/$REPO/pulls/$PR/reviews" \
  --input "$PAYLOAD"
