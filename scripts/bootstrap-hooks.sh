#!/usr/bin/env sh
# Configure repository-local Git hooks path and ensure executables for POSIX hooks.
# Usage: ./scripts/bootstrap-hooks.sh

set -eu

# Resolve repo root (script directory/..)
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd -P)
cd "$REPO_ROOT"

printf "[bootstrap] Setting Git hooks path to .githooks\n"
git config core.hooksPath .githooks

# Make hooks executable on POSIX systems
if [ -d .githooks ]; then
  # Make all regular files under .githooks executable
  # Ignore errors if no files exist
  chmod +x .githooks/* 2>/dev/null || true
  # Also ensure pre-commit specifically is executable if present
  if [ -f .githooks/pre-commit ]; then
    chmod +x .githooks/pre-commit || true
  fi
fi

printf "[bootstrap] Done. Git will use hooks in .githooks\n"

