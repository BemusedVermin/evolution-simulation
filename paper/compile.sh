#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# compile.sh - one-shot LaTeX -> PDF for channel_model.tex (POSIX)
#
# Tries, in order:
#   1. tectonic     (single self-contained binary; recommended)
#   2. latexmk      (TeX Live standard helper)
#   3. pdflatex     (raw fallback; runs twice for cross-references)
# ---------------------------------------------------------------------------
set -euo pipefail
cd "$(dirname "$0")"

TEXFILE=channel_model.tex
BASENAME=channel_model

echo
echo "=== compiling $TEXFILE ==="
echo

if command -v tectonic >/dev/null 2>&1; then
    echo "Using tectonic."
    tectonic "$TEXFILE"
elif command -v latexmk >/dev/null 2>&1; then
    echo "Using latexmk."
    latexmk -pdf -interaction=nonstopmode "$TEXFILE"
elif command -v pdflatex >/dev/null 2>&1; then
    echo "Using pdflatex (two passes for cross-references)."
    pdflatex -interaction=nonstopmode "$TEXFILE"
    pdflatex -interaction=nonstopmode "$TEXFILE"
else
    echo
    echo "ERROR: no LaTeX engine found on PATH."
    echo "Install one of:"
    echo "  - tectonic  (recommended, single binary): https://tectonic-typesetting.github.io"
    echo "  - TeX Live  (cross-platform): https://tug.org/texlive"
    exit 1
fi

if [[ -f "$BASENAME.pdf" ]]; then
    echo
    echo "=== compiled $BASENAME.pdf ==="
else
    echo
    echo "ERROR: compilation finished but $BASENAME.pdf was not produced."
    echo "Check the .log file for errors."
    exit 1
fi
