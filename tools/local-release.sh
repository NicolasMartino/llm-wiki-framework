#!/usr/bin/env bash
# The owner's local release (wiki/plans/poman-mcp-server.plan.md, "The local
# release and its revert"): build the release archives with cargo-dist for this
# machine (no tag, nothing published), save what is installed now, and run the
# archives' `llm-wiki install`, which puts llm-wiki and poman in
# ~/.llm_wiki/bin and registers both MCP servers. `revert` puts the newest
# saved state back exactly.
#
# Usage: tools/local-release.sh install [llm-wiki install flags...]
#        tools/local-release.sh revert
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SAVES="$HOME/.llm_wiki/local-release"
UNPACKED=""
trap 'if [ -n "$UNPACKED" ]; then rm -rf "$UNPACKED"; fi' EXIT

say() { printf '%s\n' "$*"; }
fail() { say "local-release: $*" >&2; exit 1; }

[ -z "${LLM_WIKI_INSTANCE:-}" ] || fail "refusing to run with LLM_WIKI_INSTANCE=${LLM_WIKI_INSTANCE}; the local release installs the real instance"
command -v python3 >/dev/null 2>&1 || fail "needs python3 (3.11 or newer); install it with your package manager"

# state.py <save|revert> <folder>: what is saved, and how it is put back. The
# places it covers are the managed home (but its models, indexes and these
# saves), every file the manifest lists, and the Codex config with its backup.
state() {
  python3 - "$HOME" "$@" <<'PY'
import json, os, shutil, sys

home, action, folder = sys.argv[1], sys.argv[2], sys.argv[3]
managed = os.path.join(home, ".llm_wiki")
skipped = {"models", "indexes", "local-release"}
files = os.path.join(folder, "files")

def under_home(path):
    path = os.path.abspath(path)
    return os.path.relpath(path, home) if path.startswith(home + os.sep) else None

def places():
    found = set()
    if os.path.isdir(managed):
        for top in os.listdir(managed):
            if top in skipped:
                continue
            start = os.path.join(managed, top)
            if os.path.isdir(start) and not os.path.islink(start):
                for parent, dirs, names in os.walk(start):
                    for name in names + [d for d in dirs if os.path.islink(os.path.join(parent, d))]:
                        found.add(under_home(os.path.join(parent, name)))
            else:
                found.add(under_home(start))
    manifest = os.path.join(managed, "manifest.json")
    if os.path.isfile(manifest):
        data = json.load(open(manifest))
        listed = [data.get("binary") or {}, data.get("poman") or {}]
        listed += data.get("skills", []) + data.get("assets", [])
        for entry in listed:
            if isinstance(entry, dict) and entry.get("path"):
                found.add(under_home(entry["path"]))
    for name in (".codex/config.toml", ".codex/config.toml.llm-wiki-backup"):
        found.add(name)
    found.discard(None)
    return found

def exists(path):
    return os.path.lexists(path)

def copy(source, target):
    os.makedirs(os.path.dirname(target), exist_ok=True)
    if os.path.lexists(target) and not os.path.isdir(target):
        os.remove(target)
    if os.path.islink(source):
        os.symlink(os.readlink(source), target)
    else:
        shutil.copy2(source, target)

def lines(name):
    path = os.path.join(folder, name)
    return set(open(path).read().splitlines()) if os.path.exists(path) else set()

if action == "save":
    os.makedirs(files)
    existed, absent, dirs = [], [], set()
    for rel in sorted(places()):
        path = os.path.join(home, rel)
        if exists(path):
            copy(path, os.path.join(files, rel))
            existed.append(rel)
            parent = os.path.dirname(rel)
            while parent:
                dirs.add(parent)
                parent = os.path.dirname(parent)
        else:
            absent.append(rel)
    for name, items in (("existed.txt", existed), ("absent.txt", absent), ("dirs.txt", sorted(dirs))):
        with open(os.path.join(folder, name), "w") as out:
            out.write("".join(item + "\n" for item in items))
    print(f"saved {len(existed)} files; {len(absent)} places were empty")
else:
    existed, dirs = lines("existed.txt"), lines("dirs.txt")
    removed = 0
    for rel in sorted(places() | lines("absent.txt") - existed):
        path = os.path.join(home, rel)
        if rel not in existed and exists(path) and not os.path.isdir(path):
            os.remove(path)
            removed += 1
            parent = os.path.dirname(rel)
            while parent and parent not in dirs:
                try:
                    os.rmdir(os.path.join(home, parent))
                except OSError:
                    break
                parent = os.path.dirname(parent)
    for rel in sorted(existed):
        copy(os.path.join(files, rel), os.path.join(home, rel))
    for rel in sorted(existed):
        saved, now = os.path.join(files, rel), os.path.join(home, rel)
        same = (os.readlink(saved) == os.readlink(now)) if os.path.islink(saved) else (
            open(saved, "rb").read() == open(now, "rb").read()
            and os.stat(saved).st_mode == os.stat(now).st_mode)
        if not same:
            sys.exit(f"local-release: {rel} did not come back as saved")
    print(f"restored {len(existed)} files and removed {removed} the release added")
PY
}

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
  state revert "$folder"
  mv "$folder" "$folder.reverted"
  say "Reverted; the saved state is kept as $folder.reverted."
}

case "${1:-}" in
  install) shift; install "$@" ;;
  revert) shift; [ "$#" -eq 0 ] || fail "revert takes no argument"; revert ;;
  *) fail "usage: tools/local-release.sh install [llm-wiki install flags...] | revert" ;;
esac
