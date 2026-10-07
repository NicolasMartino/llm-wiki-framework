#!/usr/bin/env bash
#
# Checks the quick stops of `tools/search-eval.sh`, so `just verify` and the
# fast check go red when they break: the eval itself needs the managed models
# and takes about 20 minutes, so nothing else runs the script. Each case gives
# the script a scratch HOME and must see it stop with exit 1 and one line
# naming `llm-wiki install`, before anything is built: a `cargo` on PATH that
# only says it was reached stands in for the real one. The script reads
# search.toml as TOML, so the cases spell it more than one way.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/bin"
# With PAST_BUILD set, the build passes and `register` prints the search
# settings the script wrote for the project, so a case can check them.
cat > "$TMP/bin/cargo" <<'CARGO'
#!/bin/sh
if [ "$1" = build ] && [ -n "${PAST_BUILD:-}" ]; then exit 0; fi
if [ "$1" = run ] && [ "$6" = register ]; then cat "$7/.llm_wiki/search.toml"; exit 98; fi
echo "cargo reached"
exit 99
CARGO
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
home() { # <name> <llm_search_enabled under [project_default]> [<that table's header> <flag line>]
  local managed="$TMP/$1/.llm_wiki"
  mkdir -p "$managed/models"
  : > "$managed/manifest.json"
  : > "$managed/models/artifacts.toml"
  : > "$managed/accepted-licenses.toml"
  printf '%s\n' 'schema_version = 1' 'updated_at = "2026-10-07T00:00:00Z"' '' \
    "${3:-[project_default]}" "${4:-llm_search_enabled = $2}" 'profile = "balanced"' \
    '' '[global_search]' 'llm_search_enabled = true' > "$managed/search.toml"
}
reaches_build() { # <description>
  if [ "$code" = 99 ] && [ "$out" = "cargo reached" ]; then
    echo "ok $1"
  else
    echo "FAILED $1: exit $code, expected the build to be reached"
    printf '%s\n' "--- output" "$out"
    failures=$((failures + 1))
  fi
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

home search-string - '[project_default]' 'llm_search_enabled = "true"'
run "$TMP/search-string"
stops "a home whose flag is the string \"true\" stops" 'has meaning-based search off'

home search-on true
run "$TMP/search-on"
reaches_build "a ready home gets past the checks to the build"

home hand-edited - '[ project_default ]  # edited' 'llm_search_enabled=true # on'
run "$TMP/hand-edited"
reaches_build "a hand-edited home with search on, spelled another TOML way, gets to the build"

code=0
out=$(PAST_BUILD=1 HOME="$TMP/hand-edited" PATH="$TMP/bin:$PATH" bash "$ROOT/tools/search-eval.sh" 2>&1) || code=$?
if [ "$code" = 98 ] && printf '%s\n' "$out" | sed -n '/^schema_version/,$p' | python3 -c '
import sys, tomllib
t = tomllib.loads(sys.stdin.read())
assert t == {"schema_version": 1, "updated_at": "2026-10-07T00:00:00Z",
             "project": {"llm_search_enabled": True, "profile": "balanced"}}, t
'; then
  echo "ok the project gets the hand-edited home's default settings as its own"
else
  echo "FAILED the project's settings: exit $code, expected 98 and the home's default as [project]"
  printf '%s\n' "--- output" "$out"
  failures=$((failures + 1))
fi

if [ "$failures" != 0 ]; then echo "search-eval-test: $failures case(s) failed" >&2; exit 1; fi
echo "search-eval-test: every case passed"
