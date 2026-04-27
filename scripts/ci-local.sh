#!/usr/bin/env bash
# Run every CI gate from `.github/workflows/ci.yml` locally, in the same
# order, with the same flags. Fails fast on the first red step unless
# `--keep-going` is passed, in which case every step runs and the script
# exits non-zero if *any* step failed.
#
# Steps mirror the CI jobs:
#   1. cargo fmt --check                                      (lint-and-test)
#   2. cargo clippy --workspace --exclude {beast-render,beast-ui} --all-targets -D warnings
#   3. cargo test  --workspace --exclude {beast-render,beast-ui} --all-targets --locked
#   4. cargo test  --workspace --exclude {beast-render,beast-ui} --doc --locked
#   5. cargo clippy beast-render headless                     (lint-and-test, render lane)
#   6. cargo test  beast-render headless                      (lint-and-test, render lane)
#   7. cargo clippy beast-ui     headless                     (lint-and-test, ui lane)
#   8. cargo test  beast-ui     headless                      (lint-and-test, ui lane)
#   9. cargo deny check                                       (cargo-deny)        [skipped if not installed]
#  10. cargo llvm-cov summary                                 (coverage)          [skipped if not installed]
#  11. .github/scripts/run-quality-metrics.sh                 (quality-metrics)   [skipped if lizard missing]
#  12. cargo build --release --workspace --exclude {beast-render,beast-ui} --locked
#  13. cargo build --release -p beast-render --headless --locked
#  14. cargo build --release -p beast-ui     --headless --locked
#  15. cargo test --test determinism_test --release          (M1 determinism gate)
#
# Usage:
#   scripts/ci-local.sh                 # fail-fast
#   scripts/ci-local.sh --keep-going    # run everything, report failures at end
#   scripts/ci-local.sh --quick         # skip release builds + coverage + quality
#   scripts/ci-local.sh --no-render     # skip the SDL3-from-source render steps
#
# Exit codes:
#   0  all gates green
#   1  one or more gates failed
#   64 bad CLI usage

set -uo pipefail

KEEP_GOING=0
QUICK=0
NO_RENDER=0
for arg in "$@"; do
  case "$arg" in
    --keep-going) KEEP_GOING=1 ;;
    --quick)      QUICK=1 ;;
    --no-render)  NO_RENDER=1 ;;
    -h|--help)
      sed -n '2,32p' "$0"
      exit 0
      ;;
    *)
      echo "unknown flag: $arg" >&2
      exit 64
      ;;
  esac
done

# Match the CI environment exactly. `-D warnings` is propagated to every
# `cargo *` invocation through RUSTFLAGS so any new lint that lands later
# trips here too.
export CARGO_TERM_COLOR=always
export CARGO_INCREMENTAL=0
export RUSTFLAGS="${RUSTFLAGS:--D warnings}"
export RUST_BACKTRACE=short

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# ANSI helpers — degrade gracefully if stdout isn't a tty.
if [[ -t 1 ]]; then
  C_BOLD=$(printf '\033[1m')
  C_GREEN=$(printf '\033[32m')
  C_YELLOW=$(printf '\033[33m')
  C_RED=$(printf '\033[31m')
  C_DIM=$(printf '\033[2m')
  C_RESET=$(printf '\033[0m')
else
  C_BOLD=""; C_GREEN=""; C_YELLOW=""; C_RED=""; C_DIM=""; C_RESET=""
fi

# Explicit empty initialiser so `${#FAILED_STEPS[@]}` is safe under
# `set -u` even when no step has appended to the array yet.
FAILED_STEPS=()
TOTAL=0

# Run a named step. On failure either bail or, with --keep-going, record
# the step name and continue.
step() {
  local name="$1"; shift
  TOTAL=$((TOTAL+1))
  printf '\n%s=== [%d] %s%s\n' "$C_BOLD" "$TOTAL" "$name" "$C_RESET"
  printf '%s$ %s%s\n' "$C_DIM" "$*" "$C_RESET"
  local start ended dt rc
  start=$(date +%s)
  "$@"
  rc=$?
  ended=$(date +%s)
  dt=$((ended - start))
  if [[ $rc -eq 0 ]]; then
    printf '%s✓ %s — %ds%s\n' "$C_GREEN" "$name" "$dt" "$C_RESET"
  else
    printf '%s✗ %s — failed in %ds (exit %d)%s\n' "$C_RED" "$name" "$dt" "$rc" "$C_RESET"
    FAILED_STEPS+=("$name")
    if [[ $KEEP_GOING -eq 0 ]]; then
      summarize
      exit 1
    fi
  fi
}

# Run a step that may legitimately not be runnable on this host (e.g.
# cargo-deny not installed). On a missing-tool / not-applicable case
# print a clear "skipped" note instead of failing.
optional_step() {
  local name="$1"; shift
  local probe="$1"; shift   # command that exits 0 if the tool is available
  TOTAL=$((TOTAL+1))
  printf '\n%s=== [%d] %s%s\n' "$C_BOLD" "$TOTAL" "$name" "$C_RESET"
  if ! eval "$probe" >/dev/null 2>&1; then
    printf '%s↷ %s — skipped (missing prerequisite)%s\n' "$C_YELLOW" "$name" "$C_RESET"
    return 0
  fi
  printf '%s$ %s%s\n' "$C_DIM" "$*" "$C_RESET"
  local rc
  "$@"
  rc=$?
  if [[ $rc -eq 0 ]]; then
    printf '%s✓ %s%s\n' "$C_GREEN" "$name" "$C_RESET"
  else
    printf '%s✗ %s — failed (exit %d)%s\n' "$C_RED" "$name" "$rc" "$C_RESET"
    FAILED_STEPS+=("$name")
    if [[ $KEEP_GOING -eq 0 ]]; then
      summarize
      exit 1
    fi
  fi
}

summarize() {
  printf '\n%s── summary ──%s\n' "$C_BOLD" "$C_RESET"
  if [[ ${#FAILED_STEPS[@]} -eq 0 ]]; then
    printf '%s✓ all %d gates passed%s\n' "$C_GREEN" "$TOTAL" "$C_RESET"
  else
    printf '%s✗ %d / %d gates failed:%s\n' "$C_RED" "${#FAILED_STEPS[@]}" "$TOTAL" "$C_RESET"
    for s in "${FAILED_STEPS[@]}"; do
      printf '  - %s\n' "$s"
    done
  fi
}

# ----- 1. fmt -------------------------------------------------------------
step "cargo fmt --check" \
  cargo fmt --all -- --check

# ----- 2. clippy (workspace minus beast-render + beast-ui) ---------------
# beast-ui defaults to the `sdl` feature, which propagates to
# beast-render/sdl and triggers the SDL3 build-from-source path. Exclude
# both here and run them separately in headless mode below.
step "cargo clippy (workspace, exclude beast-render+beast-ui)" \
  cargo clippy --workspace --exclude beast-render --exclude beast-ui --all-targets -- -D warnings

# ----- 3. test (workspace minus beast-render + beast-ui) -----------------
step "cargo test (workspace, exclude beast-render+beast-ui)" \
  cargo test --workspace --exclude beast-render --exclude beast-ui --all-targets --locked

# ----- 4. doctests --------------------------------------------------------
step "cargo test --doc" \
  cargo test --workspace --exclude beast-render --exclude beast-ui --doc --locked

# ----- 5 + 6 + 7 + 8. beast-render + beast-ui headless -------------------
if [[ $NO_RENDER -eq 0 ]]; then
  step "cargo clippy beast-render (headless)" \
    cargo clippy -p beast-render --no-default-features --features headless --all-targets -- -D warnings
  step "cargo test beast-render (headless)" \
    cargo test -p beast-render --no-default-features --features headless --all-targets --locked
  step "cargo clippy beast-ui (headless)" \
    cargo clippy -p beast-ui --no-default-features --features headless --all-targets -- -D warnings
  step "cargo test beast-ui (headless)" \
    cargo test -p beast-ui --no-default-features --features headless --all-targets --locked
else
  printf '\n%s↷ skipping beast-render+beast-ui headless steps (--no-render)%s\n' "$C_YELLOW" "$C_RESET"
fi

# ----- 7. cargo-deny -----------------------------------------------------
optional_step "cargo deny check" \
  "command -v cargo-deny" \
  cargo deny check --hide-inclusion-graph

if [[ $QUICK -eq 0 ]]; then
  # ----- 10. coverage summary -------------------------------------------
  optional_step "cargo llvm-cov (summary)" \
    "command -v cargo-llvm-cov" \
    cargo llvm-cov --workspace --exclude beast-render --exclude beast-ui --all-targets --locked --summary-only --no-fail-fast

  # ----- 11. quality metrics --------------------------------------------
  optional_step "quality-metrics (lizard)" \
    "command -v lizard || python -m lizard --help" \
    .github/scripts/run-quality-metrics.sh

  # ----- 12 + 13 + 14. release build ------------------------------------
  step "cargo build --release (workspace, exclude beast-render+beast-ui)" \
    cargo build --workspace --exclude beast-render --exclude beast-ui --release --locked
  if [[ $NO_RENDER -eq 0 ]]; then
    step "cargo build --release beast-render (headless)" \
      cargo build -p beast-render --no-default-features --features headless --release --locked
    step "cargo build --release beast-ui (headless)" \
      cargo build -p beast-ui --no-default-features --features headless --release --locked
  fi
fi

# ----- 12. determinism gate ----------------------------------------------
# Skip silently if the test target doesn't exist yet — it lands once the
# 100-tick replay test is wired into a workspace integration target.
if cargo metadata --no-deps --format-version 1 2>/dev/null \
   | grep -q '"determinism_test"'; then
  step "cargo test --test determinism_test (release)" \
    cargo test --test determinism_test --release --locked -- --nocapture
else
  printf '\n%s↷ determinism_test target not present — skipping (will gate once it lands)%s\n' \
    "$C_YELLOW" "$C_RESET"
fi

summarize
[[ ${#FAILED_STEPS[@]} -eq 0 ]] || exit 1
