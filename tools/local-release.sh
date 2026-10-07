#!/usr/bin/env bash
# The owner's local release (wiki/plans/poman-mcp-server.plan.md, "The local
# release and its revert"): build the release archives with cargo-dist for this
# machine (no tag, nothing published), save what is installed now, and run the
# archives' `llm-wiki install`, which puts llm-wiki and poman in
# ~/.llm_wiki/bin and registers both MCP servers. `revert` puts the newest
# saved state back exactly, but refuses while a file it would replace changed
# after the release; `--overwrite` reverts anyway, keeping a copy of each.
#
# Usage: tools/local-release.sh install [llm-wiki install flags...]
#        tools/local-release.sh revert [--overwrite]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SAVES="$HOME/.llm_wiki/local-release"
UNPACKED=""
trap 'if [ -n "$UNPACKED" ]; then rm -rf "$UNPACKED"; fi' EXIT

say() { printf '%s\n' "$*"; }
fail() { say "local-release: $*" >&2; exit 1; }

[ -z "${LLM_WIKI_INSTANCE:-}" ] || fail "refusing to run with LLM_WIKI_INSTANCE=${LLM_WIKI_INSTANCE}; the local release installs the real instance"
command -v python3 >/dev/null 2>&1 || fail "needs python3 (3.11 or newer); install it with your package manager"

# tools/local-release-state.py holds what is saved and how it is put back.
state() { python3 "$ROOT/tools/local-release-state.py" "$HOME" "$@"; }

install() {
  command -v dist >/dev/null 2>&1 || fail "needs cargo-dist; install it with: cargo install cargo-dist --locked --version 0.28.0"
  local target out stamp folder archive
  target="$(rustc -vV | sed -n 's/^host: //p')"
  out="$ROOT/target/distrib"
  say "== building the release archives for $target (dist build; no tag, nothing published)"
  (cd "$ROOT" && dist build --artifacts=local --target "$target")
  UNPACKED="$(mktemp -d)"
  for archive in "$out/llm-wiki-rs-$target.tar.xz" "$out/poman-$target.tar.xz"; do
    [ -f "$archive" ] || fail "dist did not build $archive"
    tar -xJf "$archive" -C "$UNPACKED" --strip-components=1
  done
  [ -x "$UNPACKED/llm-wiki" ] && [ -x "$UNPACKED/poman" ] || fail "the archives do not hold llm-wiki and poman"

  stamp="$(date +%Y%m%d-%H%M%S)"
  folder="$SAVES/$stamp"
  [ ! -e "$folder" ] || fail "$folder already exists; wait a second and run it again"
  say "== saving what is installed now in $folder"
  state save "$folder"
  printf 'built from %s\n' "$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo 'an unknown commit')" > "$folder/release.txt"

  say "== installing from the archives"
  "$UNPACKED/llm-wiki" install "$@"
  state installed "$folder"
  say ""
  say "Installed llm-wiki and poman from this checkout's release archives."
  say "What was installed before is saved in $folder."
  say "To go back to it: just local-release-revert"
  say "A project's .mcp.json changed later by \`llm-wiki register\` is not part of that saved state."
}

revert() {
  local folder
  folder="$(find "$SAVES" -mindepth 1 -maxdepth 1 -type d ! -name '*.reverted' 2>/dev/null | sort | tail -n 1)"
  [ -n "$folder" ] || fail "no saved state under $SAVES; nothing to revert"
  say "== putting back the state saved in $folder"
  state revert "$folder" "$@"
  mv "$folder" "$folder.reverted"
  say "Reverted; the saved state is kept as $folder.reverted."
  if [ -d "$folder.reverted/after-release" ]; then
    say "The files changed after the release are kept in $folder.reverted/after-release."
  fi
}

case "${1:-}" in
  install) shift; install "$@" ;;
  revert)
    shift
    [ "$#" -eq 0 ] || { [ "$#" -eq 1 ] && [ "$1" = --overwrite ]; } || fail "revert takes only --overwrite"
    revert "$@"
    ;;
  *) fail "usage: tools/local-release.sh install [llm-wiki install flags...] | revert [--overwrite]" ;;
esac
