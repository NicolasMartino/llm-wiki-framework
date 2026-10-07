#!/usr/bin/env bash
#
# Checks the quick stops of `tools/search-eval.sh`, so `just verify` and the
# fast check go red when they break: the eval itself needs the managed models
# and takes about 20 minutes, so nothing else runs the script. Each case gives
# the script a scratch HOME and must see it stop with exit 1 and one line
# naming `llm-wiki install`, before anything is built: a `cargo` on PATH that
# only says it was reached stands in for the real one.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/bin"
printf '#!/bin/sh\necho "cargo reached"\nexit 99\n' > "$TMP/bin/cargo"
chmod +x "$TMP/bin/cargo"

failures=0
out=""
code=0

run() { # <home>
  code=0
  out=$(HOME="$1" PATH="$TMP/bin:$PATH" bash "$ROOT/tools/search-eval.sh" 2>&1) || code=$?
}
stops() { # <description> <regex>
  if [ "$code" = 1 ] && [ "$(printf '%s\n' "$out" | wc -l)" -eq 1 ] \
     && printf '%s\n' "$out" | grep -q 'run `llm-wiki install --configure-search` first' \
     && printf '%s\n' "$out" | grep -Eq -- "$2"; then
    echo "ok $1"
  else
    echo "FAILED $1: exit $code, expected 1 and one line matching /$2/"
    printf '%s\n' "--- output" "$out"
    failures=$((failures + 1))
  fi
}
home() { # <name> <llm_search_enabled under [project_default]>
  local managed="$TMP/$1/.llm_wiki"
  mkdir -p "$managed/models"
  : > "$managed/manifest.json"
  : > "$managed/models/artifacts.toml"
  : > "$managed/accepted-licenses.toml"
  printf '%s\n' 'schema_version = 1' '' '[project_default]' "llm_search_enabled = $2" \
    '' '[global_search]' 'llm_search_enabled = true' > "$managed/search.toml"
}

mkdir -p "$TMP/empty"
run "$TMP/empty"
stops "an empty home stops on its first missing file" 'manifest\.json is missing'

home no-licenses true
rm "$TMP/no-licenses/.llm_wiki/accepted-licenses.toml"
run "$TMP/no-licenses"
stops "a home without accepted licenses stops" 'accepted-licenses\.toml is missing'

home search-off false
run "$TMP/search-off"
stops "a home with meaning-based search off stops, whatever other sections say" 'has meaning-based search off'

home search-on true
run "$TMP/search-on"
if [ "$code" = 99 ] && [ "$out" = "cargo reached" ]; then
  echo "ok a ready home gets past the checks to the build"
else
  echo "FAILED a ready home: exit $code, expected the build to be reached"
  printf '%s\n' "--- output" "$out"
  failures=$((failures + 1))
fi

if [ "$failures" != 0 ]; then echo "search-eval-test: $failures case(s) failed" >&2; exit 1; fi
echo "search-eval-test: every case passed"
