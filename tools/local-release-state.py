#!/usr/bin/env python3
"""What the local release saves, and how its revert puts it back.

Usage: local-release-state.py <home> save <folder>
       local-release-state.py <home> installed <folder>
       local-release-state.py <home> revert <folder> [--overwrite]

The places it covers are the managed home (but its models, indexes and these
saves), every file the manifest lists, and the Codex config with its backup.
`save` copies what is there before the install; `installed` records each
place's hash right after it, so `revert` can tell the release's own changes
from changes made since (a trusted project in the Codex config, an accepted
licence, a search setting). Those it never overwrites unless told to, and then
only after copying each one aside.
"""
import hashlib
import json
import os
import shutil
import sys

# The tools/local-release.sh command that reverts anyway; the refusal names it.
OVERWRITE = "just local-release-revert --overwrite"
SKIPPED = {"models", "indexes", "local-release"}

# abspath drops a trailing slash and doubled ones, so that every path under a
# HOME given as `/home/u/` is still found under it.
home = os.path.abspath(sys.argv[1])
action, folder = sys.argv[2], sys.argv[3]
options = sys.argv[4:]
managed = os.path.join(home, ".llm_wiki")
files = os.path.join(folder, "files")
aside = os.path.join(folder, "after-release")


def under_home(path):
    # A manifest may name a path through the real folder HOME links to.
    real = os.path.join(os.path.realpath(os.path.dirname(path)), os.path.basename(path))
    for candidate, base in ((os.path.abspath(path), home), (real, os.path.realpath(home))):
        rel = os.path.relpath(candidate, base)
        if rel != os.curdir and rel != os.pardir and not rel.startswith(os.pardir + os.sep):
            return rel
    return None


def places():
    found = set()
    if os.path.isdir(managed):
        for top in os.listdir(managed):
            if top in SKIPPED:
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
        with open(manifest) as source:
            data = json.load(source)
        listed = [data.get("binary") or {}, data.get("poman") or {}]
        listed += data.get("skills", []) + data.get("assets", [])
        for entry in listed:
            if isinstance(entry, dict) and entry.get("path"):
                found.add(under_home(entry["path"]))
    found.update((".codex/config.toml", ".codex/config.toml.llm-wiki-backup"))
    found.discard(None)
    return found


def signature(path):
    """What is at `path`, as one comparable string; None when nothing is."""
    if os.path.islink(path):
        return "link " + os.readlink(path)
    if os.path.isdir(path):
        return "folder"
    if not os.path.exists(path):
        return None
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return f"file {digest.hexdigest()} {os.stat(path).st_mode:o}"


def say(text):
    print(text, file=sys.stderr)


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
    if not os.path.exists(path):
        return None
    with open(path) as source:
        return source.read().splitlines()


def write_lines(name, items):
    with open(os.path.join(folder, name), "w") as out:
        out.write("".join(item + "\n" for item in items))


def save():
    os.makedirs(files)
    existed, absent, dirs = [], [], set()
    for rel in sorted(places()):
        path = os.path.join(home, rel)
        if os.path.lexists(path):
            copy(path, os.path.join(files, rel))
            existed.append(rel)
            parent = os.path.dirname(rel)
            while parent:
                dirs.add(parent)
                parent = os.path.dirname(parent)
        else:
            absent.append(rel)
    write_lines("existed.txt", existed)
    write_lines("absent.txt", absent)
    write_lines("dirs.txt", sorted(dirs))
    print(f"saved {len(existed)} files; {len(absent)} places were empty")


def saved_state():
    existed = set(lines("existed.txt") or [])
    return existed, set(lines("absent.txt") or []), set(lines("dirs.txt") or [])


def installed():
    existed, absent, _ = saved_state()
    record = []
    for rel in sorted(places() | existed | absent):
        record.append(f"{rel}\t{signature(os.path.join(home, rel)) or 'absent'}")
    write_lines("installed.txt", record)
    print(f"recorded {len(record)} places as the release left them")


def revert():
    if any(option != "--overwrite" for option in options):
        sys.exit(f"local-release: revert takes only --overwrite, not {' '.join(options)}")
    overwrite = bool(options)
    existed, absent, dirs = saved_state()
    record = lines("installed.txt")
    release = None
    if record is not None:
        release = {}
        for line in record:
            rel, _, state = line.partition("\t")
            release[rel] = None if state == "absent" else state
    every = places() | existed | absent | set(release or {})

    def as_saved(rel):
        now = signature(os.path.join(home, rel))
        saved = signature(os.path.join(files, rel)) if rel in existed else None
        # A folder where nothing was saved is left alone, as it always was.
        return now == saved or (now == "folder" and saved is None), now

    # What the revert would lose: a file added or changed after the release
    # recorded its state. A file deleted since loses nothing by coming back.
    to_change, since = [], []
    for rel in sorted(every):
        same, now = as_saved(rel)
        if same:
            continue
        to_change.append(rel)
        if now is not None and (release is None or now != release.get(rel)):
            added = release is None and rel not in existed or release is not None and release.get(rel) is None
            since.append((rel, "added" if added else "changed"))

    if since and not overwrite:
        if release is None:
            say("local-release: the release did not record what it installed (its install did not finish),")
            say("so these files differ from the saved state and may hold changes of yours:")
        else:
            say("local-release: these files changed after the release, and a revert would lose them:")
        for rel, how in since:
            say(f"  {how}: ~/{rel}")
        say("Nothing was reverted. To revert anyway, keeping a copy of each file above, run:")
        say(f"  {OVERWRITE}")
        sys.exit(1)

    for rel, _ in since:
        path = os.path.join(home, rel)
        if not os.path.isdir(path):
            copy(path, os.path.join(aside, rel))
    restored = removed = 0
    for rel in to_change:
        path = os.path.join(home, rel)
        if rel in existed:
            copy(os.path.join(files, rel), path)
            restored += 1
        elif not os.path.isdir(path):
            os.remove(path)
            removed += 1
            parent = os.path.dirname(rel)
            while parent and parent not in dirs:
                try:
                    os.rmdir(os.path.join(home, parent))
                except OSError:
                    break
                parent = os.path.dirname(parent)

    left = [rel for rel in sorted(every) if not as_saved(rel)[0]]
    if left:
        sys.exit("local-release: these did not come back as saved: " + ", ".join(left))
    print(f"put back {restored} files and removed {removed} that were not there before")
    if since:
        print(f"kept a copy of the {len(since)} files changed after the release")


{"save": save, "installed": installed, "revert": revert}[action]()
