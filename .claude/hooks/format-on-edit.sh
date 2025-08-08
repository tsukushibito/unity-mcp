#!/usr/bin/env bash
# Format Rust (.rs) and C# (.cs) files after Claude Code edits.
# Reads the full hook event JSON from stdin and extracts changed file paths.

set -euo pipefail

# Read entire event JSON from stdin
EVENT_JSON=$(cat || true)

if [[ -z "${EVENT_JSON}" ]]; then
  # Nothing to do
  exit 0
fi

# Helper: print to stderr
log() { printf "[claude-hook] %s\n" "$*" >&2; }

# Get unique list of candidate file paths from several possible payload shapes
readarray -t PATHS < <(
  printf '%s' "${EVENT_JSON}" | jq -r '
    [
      .tool_input.file_path?,
      (.tool_input.file_paths? // [])[],
      (.tool_input.files? // [])[] | .path?,
      (.tool_input.changes? // [])[] | .path?,
      (.tool_input.changes? // [])[] | .file_path?,
      (.tool_input.diff? // [])[] | .path?
    ]
    | map(select(type == "string"))
    | map(select(length > 0))
    | unique[]
  ' 2>/dev/null || true
)

if [[ ${#PATHS[@]} -eq 0 ]]; then
  # Fallback: if a simple string was piped instead of JSON, treat it as one path
  if [[ -n "${EVENT_JSON}" && ! "${EVENT_JSON}" =~ \{ ]]; then
    PATHS=("${EVENT_JSON}")
  fi
fi

# Nothing to format
if [[ ${#PATHS[@]} -eq 0 ]]; then
  exit 0
fi

format_rust() {
  local file="$1"
  if command -v rustfmt >/dev/null 2>&1; then
    rustfmt "$file" || true
  elif command -v cargo >/dev/null 2>&1; then
    # Run cargo fmt with file filtering from repo root when possible
    local root
    root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
    (cd "$root" && cargo fmt -- "$file") || true
  else
    log "Rust formatter not available (rustfmt/cargo not found). Skipping $file"
  fi
}

format_csharp() {
  local file="$1"
  # Prefer csharpier if available (supports single-file formatting)
  if command -v csharpier >/dev/null 2>&1; then
    csharpier "$file" >/dev/null 2>&1 || true
    return
  fi
  if command -v dotnet >/dev/null 2>&1; then
    # Try to find nearest workspace (sln/csproj)
    local dir ws wsdir
    dir=$(dirname "$file")
    ws=""
    while [[ "$dir" != "/" && -n "$dir" ]]; do
      ws=$(ls "$dir"/*.sln "$dir"/*.csproj 2>/dev/null | head -n1 || true)
      [[ -n "$ws" ]] && break
      dir=$(dirname "$dir")
    done
    if [[ -n "$ws" ]]; then
      wsdir=$(dirname "$ws")
      (cd "$wsdir" && dotnet format --verbosity quiet --include "$file" 2>/dev/null) || \
      (cd "$wsdir" && dotnet format whitespace --verbosity quiet --include "$file" 2>/dev/null) || true
    else
      # Fallback: format current folder and include the file
      local root
      root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
      (cd "$root" && dotnet format --folder --verbosity quiet --include "$file" 2>/dev/null) || true
    fi
  else
    log "C# formatter not available (dotnet/csharpier not found). Skipping $file"
  fi
}

# Iterate over paths safely (handle spaces)
for p in "${PATHS[@]}"; do
  # Only consider existing regular files
  if [[ ! -f "$p" ]]; then
    continue
  fi
  case "$p" in
    *.rs)
      format_rust "$p" ;;
    *.cs)
      format_csharp "$p" ;;
    *)
      # ignore other file types
      : ;;
  esac
done

exit 0
