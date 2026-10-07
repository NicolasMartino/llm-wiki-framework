#!/usr/bin/env bash
#
# Checks `just local-release` and its revert under a temporary HOME, so
# `just verify` goes red when the save or the revert breaks. The scripts run
# from a scratch copy, with stand-ins for cargo-dist and rustc on PATH: the
# stand-in archives hold an `llm-wiki` whose `install` writes what a release
# writes (the binaries, the manifest, the Codex config and its backup). No
# case touches the real ~/.llm_wiki. Runs under macOS's bash 3.2 too.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for tool in python3 tar xz; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "local-release-test needs $tool on PATH (your package manager)" >&2
    exit 1
  }
done

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export GIT_CEILING_DIRECTORIES="$TMP"
unset LLM_WIKI_INSTANCE

mkdir -p "$TMP/root/tools" "$TMP/bin"
cp "$ROOT/tools/local-release.sh" "$ROOT/tools/local-release-state.py" "$TMP/root/tools/"

cat > "$TMP/bin/rustc" <<'EOF'
#!/usr/bin/env bash
echo "host: test-target"
EOF
# The stand-in release: its `llm-wiki install` replaces llm-wiki, adds poman,
# rewrites the manifest and adds poman's server to the Codex config.
cat > "$TMP/bin/dist" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
work=$(mktemp -d)
mkdir -p "$work/llm-wiki" "$work/poman" target/distrib
cat > "$work/llm-wiki/llm-wiki" <<'TOOL'
#!/usr/bin/env bash
set -euo pipefail
[ "$1" = install ] || exit 2
bin="$HOME/.llm_wiki/bin"
mkdir -p "$bin" "$HOME/.codex"
echo "new llm-wiki" > "$bin/llm-wiki"
echo "poman" > "$bin/poman"
chmod 755 "$bin/llm-wiki" "$bin/poman"
printf '{"binary": {"path": "%s"}, "poman": {"path": "%s"}}\n' "$bin/llm-wiki" "$bin/poman" > "$HOME/.llm_wiki/manifest.json"
cp "$HOME/.codex/config.toml" "$HOME/.codex/config.toml.llm-wiki-backup"
printf '[mcp_servers.poman]\ncommand = "poman"\n' >> "$HOME/.codex/config.toml"
TOOL
echo "poman" > "$work/poman/poman"
chmod 755 "$work/llm-wiki/llm-wiki" "$work/poman/poman"
tar -cJf target/distrib/llm-wiki-rs-test-target.tar.xz -C "$work" llm-wiki
tar -cJf target/distrib/poman-test-target.tar.xz -C "$work" poman
rm -rf "$work"
EOF
chmod 755 "$TMP/bin/rustc" "$TMP/bin/dist"
export PATH="$TMP/bin:$PATH"

failures=0
out=""
code=0

fail() { echo "FAIL: $*"; printf '%s\n' "$out" | sed 's/^/    /'; failures=$((failures + 1)); }
pass() { echo "ok:   $*"; }

# Every file and link under a HOME, but the saves, with its mode and contents.
tree() {
  python3 - "$1" <<'PY'
import hashlib, os, sys
home = sys.argv[1]
for parent, dirs, names in os.walk(home):
    dirs[:] = sorted(d for d in dirs if os.path.join(parent, d) != os.path.join(home, ".llm_wiki", "local-release"))
    for name in sorted(names + [d for d in dirs if os.path.islink(os.path.join(parent, d))]):
        path = os.path.join(parent, name)
        rel = os.path.relpath(path, home)
        if os.path.islink(path):
            print(rel, "link", os.readlink(path))
        else:
            print(rel, oct(os.stat(path).st_mode), hashlib.sha256(open(path, "rb").read()).hexdigest())
PY
}

# A HOME with an install already there, as the owner's machine has.
new_home() {
  local home="$TMP/$1"
  mkdir -p "$home/.llm_wiki/bin" "$home/.llm_wiki/models" "$home/.codex"
  echo "old llm-wiki" > "$home/.llm_wiki/bin/llm-wiki"
  chmod 755 "$home/.llm_wiki/bin/llm-wiki"
  printf '{"binary": {"path": "%s"}}\n' "$home/.llm_wiki/bin/llm-wiki" > "$home/.llm_wiki/manifest.json"
  echo 'backend = "lexical"' > "$home/.llm_wiki/search.toml"
  echo "model" > "$home/.llm_wiki/models/m.gguf"
  echo '[mcp_servers.llm-wiki]' > "$home/.codex/config.toml"
  printf '%s\n' "$home"
}

release() { # <home as given> <args...>
  local home="$1"; shift
  code=0
  out=$(cd "$TMP" && HOME="$home" bash "$TMP/root/tools/local-release.sh" "$@" 2>&1) || code=$?
}

saves() { find "$1/.llm_wiki/local-release" -mindepth 1 -maxdepth 1 -type d | sort; }

# 1. Release, then revert: the HOME comes back file for file.
home=$(new_home plain)
before=$(tree "$home")
release "$home" install
if [ "$code" -eq 0 ] && [ "$(cat "$home/.llm_wiki/bin/llm-wiki")" = "new llm-wiki" ] && [ -f "$home/.llm_wiki/bin/poman" ]; then
  pass "the release installs both binaries"
else
  fail "the release installs both binaries (exit $code)"
fi
release "$home" revert
if [ "$code" -eq 0 ] && [ "$(tree "$home")" = "$before" ] && printf '%s' "$out" | grep -q '^Reverted'; then
  pass "the revert puts the HOME back exactly"
else
  fail "the revert puts the HOME back exactly (exit $code)"
fi
case "$(saves "$home")" in
  *.reverted) pass "the reverted save is kept, marked reverted" ;;
  *) fail "the reverted save is kept, marked reverted" ;;
esac

# 2. Changes made after the release: the revert refuses, names each one and
# the command that reverts anyway, and changes nothing.
home=$(new_home changed)
before=$(tree "$home")
release "$home" install
printf '[projects."/work/riseon"]\ntrust_level = "trusted"\n' >> "$home/.codex/config.toml"
echo "accepted" > "$home/.llm_wiki/accepted-licenses.toml"
rm "$home/.llm_wiki/search.toml"
after=$(tree "$home")
release "$home" revert
if [ "$code" -ne 0 ] \
  && printf '%s' "$out" | grep -qF "changed: ~/.codex/config.toml" \
  && printf '%s' "$out" | grep -qF "added: ~/.llm_wiki/accepted-licenses.toml" \
  && printf '%s' "$out" | grep -qF "just local-release-revert --overwrite" \
  && ! printf '%s' "$out" | grep -qF "search.toml" \
  && ! printf '%s' "$out" | grep -q '^Reverted'; then
  pass "the revert refuses, naming each file changed after the release"
else
  fail "the revert refuses, naming each file changed after the release (exit $code)"
fi
if [ "$(tree "$home")" = "$after" ] && case "$(saves "$home")" in *.reverted) false ;; *) true ;; esac; then
  pass "a refused revert changes nothing"
else
  fail "a refused revert changes nothing"
fi

# 3. --overwrite reverts anyway, after copying each of those files aside.
release "$home" revert --overwrite
kept="$(saves "$home")/after-release"
if [ "$code" -eq 0 ] && [ "$(tree "$home")" = "$before" ] \
  && grep -qF trust_level "$kept/.codex/config.toml" \
  && [ "$(cat "$kept/.llm_wiki/accepted-licenses.toml")" = accepted ] \
  && printf '%s' "$out" | grep -qF "kept in $kept"; then
  pass "--overwrite reverts and keeps a copy of each changed file, saying where"
else
  fail "--overwrite reverts and keeps a copy of each changed file, saying where (exit $code)"
fi

# 4. A HOME given with a trailing slash saves and reverts everything.
home=$(new_home slash)
before=$(tree "$home")
release "$home/" install
if [ "$code" -eq 0 ] && printf '%s' "$out" | grep -q '^saved 4 files'; then
  pass "a HOME ending in / saves every file"
else
  fail "a HOME ending in / saves every file (exit $code)"
fi
release "$home/" revert
if [ "$code" -eq 0 ] && [ "$(tree "$home")" = "$before" ]; then
  pass "a HOME ending in / reverts every file"
else
  fail "a HOME ending in / reverts every file (exit $code)"
fi

# 5. A revert that cannot put a file back fails, and never says it reverted.
home=$(new_home stuck)
release "$home" install
rm "$home/.llm_wiki/search.toml"
mkdir "$home/.llm_wiki/search.toml"
release "$home" revert --overwrite
if [ "$code" -ne 0 ] && printf '%s' "$out" | grep -qF "did not come back as saved" \
  && ! printf '%s' "$out" | grep -q '^Reverted'; then
  pass "a revert that leaves the release in place fails"
else
  fail "a revert that leaves the release in place fails (exit $code)"
fi

# 6. Neither command runs against a test instance.
code=0
out=$(cd "$TMP" && HOME="$TMP/plain" LLM_WIKI_INSTANCE=test bash "$TMP/root/tools/local-release.sh" revert 2>&1) || code=$?
if [ "$code" -ne 0 ] && printf '%s' "$out" | grep -qF "refusing to run with LLM_WIKI_INSTANCE=test"; then
  pass "it refuses to run with LLM_WIKI_INSTANCE set"
else
  fail "it refuses to run with LLM_WIKI_INSTANCE set (exit $code)"
fi

echo ""
if [ "$failures" -eq 0 ]; then
  echo "local-release-test: all cases pass."
else
  echo "local-release-test: $failures case(s) failed." >&2
  exit 1
fi
