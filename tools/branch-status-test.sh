#!/usr/bin/env bash
#
# Checks `just branch-status` against a scratch origin, so `just verify` goes
# red when the recipe breaks. Each case below is a row the recipe must print,
# and the exit code it must give.
#
# The scratch repository has its own "statuses moved into PRs" commit, passed
# to the recipe with `--set`, where one plan (old.plan.md) already names its
# branch. The recipe itself is copied from this repository's justfile.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for tool in git just; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "branch-status-test needs $tool on PATH (git: your package manager; just: https://just.systems)" >&2
    exit 1
  }
done

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.invalid
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.invalid

failures=0
out=""
code=0
moved=""

plan() { # <file> <status> [branch]
  { echo "# Plan"; echo ""; echo "- Document Class: Plan"; echo "- Status: $2"
    [ -z "${3:-}" ] || echo "- Branch: \`$3\`"; echo "- Date: 2026-10-06"; } > "wiki/plans/$1"
}
commit() { git add -A && git commit -q -m "$1"; }
run() { # [dir]
  code=0
  out=$(cd "${1:-$TMP/work}" && just --set statuses_moved_into_prs "$moved" branch-status 2>&1) || code=$?
}
has() { # <description> <regex>
  if printf '%s\n' "$out" | grep -qE -- "$2"; then echo "ok    $1"
  else echo "FAILED $1: no line matches /$2/"; failures=$((failures + 1)); fi
}
lacks() { # <description> <regex>
  if printf '%s\n' "$out" | grep -qE -- "$2"; then
    echo "FAILED $1: a line matches /$2/"; failures=$((failures + 1))
  else echo "ok    $1"; fi
}
exits() { # <description> <code>
  if [ "$code" = "$2" ]; then echo "ok    $1"
  else echo "FAILED $1: exit $code, expected $2"; failures=$((failures + 1)); fi
}
dump() { [ "$failures" = 0 ] || printf '%s\n' "--- output of the last run" "$out"; }

git init -q --bare "$TMP/origin.git"
git init -q -b develop "$TMP/work"
cd "$TMP/work"
git remote add origin "$TMP/origin.git"
cp "$ROOT/justfile" .
mkdir -p wiki/plans
plan old.plan.md Active old-branch
plan a.plan.md Draft
plan c.plan.md Draft
commit "statuses move into PRs"
moved=$(git rev-parse HEAD)
git push -q origin develop develop:old-branch develop:no-plan-branch

# Work started the new way: each branch names its plan on itself.
git switch -q -c feat-a && plan a.plan.md Active feat-a && commit a && git push -q origin feat-a
git switch -q -c feat-c develop && plan c.plan.md "Completed (develop)" feat-c && commit c && git push -q origin feat-c
git switch -q develop

# A Branch line written on develop after the move, its branch still on origin.
plan late.plan.md Active late-branch && commit late && git push -q origin develop develop:late-branch

echo "from develop"
run
has "a branch's own Active plan is listed" '^  ok +feat-a +-> a\.plan\.md \[Active\]'
has "a branch's own plan under another status is STALE" '^  STALE +feat-c +-> c\.plan\.md \[Completed \(develop\)\]'
has "a branch no plan names is listed" '^  no plan +no-plan-branch'
has "a line from before the move, its branch on origin, is in flight" '^  in flight old\.plan\.md +old-branch'
has "its branch's row points to it" '^  ok +old-branch +-> old\.plan\.md'
has "a line written on develop after the move is STALE" '^  STALE +late\.plan\.md +names late-branch on this checkout'
exits "STALE lines fail the command" 1

plan late.plan.md Active && commit "late: drop the line" && git push -q origin develop
git push -q origin --delete feat-c
run
exits "with those fixed, it passes" 0

git push -q origin --delete old-branch
run
has "a line from before the move, its branch gone, is STALE" '^  STALE +old\.plan\.md +names old-branch, which origin does not have'
exits "and fails the command" 1
git push -q origin develop:old-branch

echo "from a worker's branch, detached as a PR checkout leaves it"
git switch -q --detach origin/feat-a
run
has "its own plan is its own" '^  ok +a\.plan\.md +feat-a \[Active\], this branch.s own plan'
lacks "and not a leftover" '^  (STALE|in flight) a\.plan\.md'
exits "and it passes" 0
git switch -q develop

echo "from a single-branch clone"
git clone -q --single-branch --branch develop "$TMP/origin.git" "$TMP/single"
run "$TMP/single"
has "it still sees every branch" '^  ok +feat-a +-> a\.plan\.md'
has "and the line from before the move is in flight" '^  in flight old\.plan\.md'

dump
if [ "$failures" != 0 ]; then echo "branch-status-test: $failures case(s) failed" >&2; exit 1; fi
echo "branch-status-test: every case passed"
