#!/usr/bin/env bash
# Shared driver for the quality-metrics CI gate.
#
# Called by both .github/workflows/ci.yml (`quality-metrics` job) and
# .githooks/pre-push. Keep this script the single source of truth for the
# lizard invocations and threshold enforcement — editing the CI YAML or the
# hook in isolation is a bug.
#
# Usage:
#   run-quality-metrics.sh [--artifact-dir DIR]
#
# Thresholds default to the values in .github/workflows/ci.yml and can be
# overridden via env vars so the CI job can set them in one place:
#   QUALITY_MAX_CCN                 (default: 10)
#   QUALITY_MAX_LENGTH              (default: 80)
#   QUALITY_MAX_DUPLICATE_RATE      (default: 5.0)
#   QUALITY_MIN_PUBLIC_DOC_COVERAGE (default: 80.0)
#
# --artifact-dir DIR: write quality-lizard.txt / quality-functions.csv /
# quality-duplicates.txt / quality-summary.md into DIR and leave them there
# (CI uses this so the upload-artifact step can pick them up). If omitted,
# artifacts go to a tmpdir that is cleaned up on exit.
#
# Exit status is 0 only when every lizard invocation and the summary script
# returned 0. On failure the summary is printed to stderr so callers that
# can't see the artifact (e.g. the pre-push hook) still get actionable
# feedback.

set -eu

# Resolve script dir so relative paths work from any caller CWD.
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

# --- Argument parsing ---------------------------------------------------------

artifact_dir=''
while [ $# -gt 0 ]; do
    case "$1" in
        --artifact-dir)
            artifact_dir="$2"
            shift 2
            ;;
        --artifact-dir=*)
            artifact_dir="${1#--artifact-dir=}"
            shift
            ;;
        *)
            echo "run-quality-metrics.sh: unknown argument '$1'" >&2
            exit 2
            ;;
    esac
done

if [ -z "$artifact_dir" ]; then
    artifact_dir="$(mktemp -d)"
    trap 'rm -rf "$artifact_dir"' EXIT
else
    mkdir -p "$artifact_dir"
fi

# --- Threshold defaults (overridable via env) ---------------------------------

: "${QUALITY_MAX_CCN:=10}"
: "${QUALITY_MAX_LENGTH:=80}"
: "${QUALITY_MAX_DUPLICATE_RATE:=5.0}"
: "${QUALITY_MIN_PUBLIC_DOC_COVERAGE:=80.0}"

# --- Lizard + summary script --------------------------------------------------
#
# The four invocations below mirror the ci.yml quality-metrics job verbatim;
# changing arguments here is a cross-job contract change. Each command's
# exit status is captured and aggregated at the end so we run the full
# report regardless of which step fails first — matching CI's behaviour.

cd "$repo_root"

set +e
python -m lizard -l rust -C"${QUALITY_MAX_CCN}" -L"${QUALITY_MAX_LENGTH}" \
    -x '*/tests/*' \
    -x '*/benches/*' \
    -W '.github/qa/whitelizard.txt' \
    crates > "$artifact_dir/quality-lizard.txt"
complexity_status=$?

python -m lizard --csv -l rust -C"${QUALITY_MAX_CCN}" -L"${QUALITY_MAX_LENGTH}" \
    -x '*/tests/*' \
    -x '*/benches/*' \
    -W '.github/qa/whitelizard.txt' \
    crates > "$artifact_dir/quality-functions.csv"
functions_status=$?

python -m lizard -l rust -Eduplicate \
    -x '*/tests/*' \
    -x '*/benches/*' \
    crates > "$artifact_dir/quality-duplicates.txt"
duplicates_status=$?

python .github/scripts/quality_metrics.py \
    --functions "$artifact_dir/quality-functions.csv" \
    --duplicates "$artifact_dir/quality-duplicates.txt" \
    --summary "$artifact_dir/quality-summary.md" \
    --workspace-root . \
    --max-duplicate-rate "${QUALITY_MAX_DUPLICATE_RATE}" \
    --max-ccn "${QUALITY_MAX_CCN}" \
    --max-length "${QUALITY_MAX_LENGTH}" \
    --min-public-doc-coverage "${QUALITY_MIN_PUBLIC_DOC_COVERAGE}"
summary_status=$?
set -e

# --- Aggregate & report ------------------------------------------------------

if [ "$complexity_status" -ne 0 ] || [ "$functions_status" -ne 0 ] \
        || [ "$duplicates_status" -ne 0 ] || [ "$summary_status" -ne 0 ]; then
    echo >&2
    echo 'run-quality-metrics: FAILED. Summary:' >&2
    echo >&2
    cat "$artifact_dir/quality-summary.md" >&2 2>/dev/null || true
    exit 1
fi

# Expose the artifact dir to the caller (CI) without polluting when we own it.
echo "$artifact_dir"
