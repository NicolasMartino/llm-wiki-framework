#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SUFFIX=".disabled-for-mcp-eval.$$"
MOVED=()

restore() {
  local pair from to
  for pair in "${MOVED[@]}"; do
    from="${pair%%::*}"
    to="${pair##*::}"
    if [[ -e "$from" ]]; then
      mv "$from" "$to"
    fi
  done
}

trap restore EXIT

for rel in ".claude/skills" ".codex/skills"; do
  from="$ROOT/$rel"
  to="$ROOT/$rel$SUFFIX"
  if [[ -e "$from" ]]; then
    if [[ -e "$to" ]]; then
      echo "refusing to overwrite existing backup path: $to" >&2
      exit 1
    fi
    mv "$from" "$to"
    MOVED+=("$to::$from")
  fi
done

if [[ "$#" -eq 0 ]]; then
  echo "repo-local skills disabled for this process; pass a command to run under this posture" >&2
  exit 0
fi

cd "$ROOT"
"$@"
