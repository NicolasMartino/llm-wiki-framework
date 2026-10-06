#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${LLM_WIKI_TEST_INSTANCE_PROOF_WORK:-$ROOT/target/test-instance-live-session}"

mkdir -p "$WORK_DIR"

select_python() {
  local candidate
  for candidate in "${PYTHON:-}" python3.13 python3.12 python3.11 python3.10 python3; do
    if [[ -n "$candidate" ]] && command -v "$candidate" >/dev/null 2>&1; then
      command -v "$candidate"
      return 0
    fi
  done
  echo "could not find a Python interpreter" >&2
  return 1
}

data_home() {
  printf '%s\n' "${XDG_DATA_HOME:-$HOME/.local/share}"
}

cache_home() {
  printf '%s\n' "${XDG_CACHE_HOME:-$HOME/.cache}"
}

test_state_paths=(
  "$HOME/.llm_wiki-test"
  "$HOME/.claude/skills/wiki-test"
  "$HOME/.claude/skills/wiki-init-test"
  "$HOME/.claude/skills/wiki-query-test"
  "$HOME/.claude/skills/wiki-ingest-test"
  "$HOME/.claude/skills/wiki-research-test"
  "$HOME/.claude/skills/wiki-lint-test"
  "$HOME/.codex/skills/wiki-test"
  "$HOME/.codex/skills/wiki-init-test"
  "$HOME/.codex/skills/wiki-query-test"
  "$HOME/.codex/skills/wiki-ingest-test"
  "$HOME/.codex/skills/wiki-research-test"
  "$HOME/.codex/skills/wiki-lint-test"
  "$(data_home)/llm-wiki-test"
  "$(cache_home)/llm-wiki-test"
)

for path in "${test_state_paths[@]}"; do
  if [[ -e "$path" || -L "$path" ]]; then
    echo "pre-existing test-instance state would be modified: $path" >&2
    echo "remove it intentionally, then rerun this proof" >&2
    exit 1
  fi
done

snapshot() {
  local out="$1"
  SNAPSHOT_OUT="$out" SNAPSHOT_HOME="$HOME" SNAPSHOT_DATA_HOME="$(data_home)" \
    SNAPSHOT_CACHE_HOME="$(cache_home)" "$PYTHON_BIN" - <<'PY'
import hashlib
import os
from pathlib import Path

home = Path(os.environ["SNAPSHOT_HOME"])
data_home = Path(os.environ["SNAPSHOT_DATA_HOME"])
cache_home = Path(os.environ["SNAPSHOT_CACHE_HOME"])
out = Path(os.environ["SNAPSHOT_OUT"])

roots = [
    home / ".llm_wiki",
    home / ".claude" / "skills",
    home / ".codex" / "skills",
    data_home / "llm-wiki",
    cache_home / "llm-wiki",
]

skill_prefixes = (
    "wiki",
    "wiki-init",
    "wiki-query",
    "wiki-ingest",
    "wiki-research",
    "wiki-lint",
)

def include(path: Path) -> bool:
    if path == home / ".claude" / "skills" or path == home / ".codex" / "skills":
        return True
    for parent in (home / ".claude" / "skills", home / ".codex" / "skills"):
        try:
            rel = path.relative_to(parent)
        except ValueError:
            continue
        if rel.parts and rel.parts[0].startswith(skill_prefixes):
            return True
        return False
    return True

def digest_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

records = []

def rel(path: Path) -> str:
    try:
        return str(path.relative_to(home))
    except ValueError:
        return str(path)

def record_path(path: Path) -> None:
    if not include(path):
        return
    if path.is_symlink():
        records.append(f"symlink {rel(path)} -> {os.readlink(path)}")
    elif path.is_file():
        records.append(f"file {rel(path)} {digest_file(path)}")
    elif path.is_dir():
        records.append(f"dir {rel(path)}")
    elif not path.exists():
        records.append(f"missing {rel(path)}")
    else:
        records.append(f"other {rel(path)}")

for root in roots:
    if not root.exists() and not root.is_symlink():
        record_path(root)
        continue
    if root.is_dir() and not root.is_symlink():
        for current, dirs, files in os.walk(root):
            current_path = Path(current)
            dirs[:] = sorted(dirs)
            for name in sorted(dirs):
                record_path(current_path / name)
            for name in sorted(files):
                record_path(current_path / name)
        record_path(root)
    else:
        record_path(root)

records = sorted(set(records))
summary = hashlib.sha256("\n".join(records).encode("utf-8")).hexdigest()
out.write_text(f"summary {summary}\n" + "\n".join(records) + "\n", encoding="utf-8")
PY
}

cleanup() {
  set +e
  if [[ "${INSTALLED_TEST_INSTANCE:-0}" == "1" ]]; then
    (cd "$ROOT" && LLM_WIKI_INSTANCE=test cargo build --bin llm-wiki >/dev/null 2>&1)
    (cd "$ROOT" && target/debug/llm-wiki uninstall >/dev/null 2>&1)
  fi
  (cd "$ROOT" && env -u LLM_WIKI_INSTANCE cargo build --bin llm-wiki >/dev/null 2>&1) || true
}
trap cleanup EXIT

PYTHON_BIN="$(select_python)"

CODEX_CONFIG="$HOME/.codex/config.toml"
CODEX_CONFIG_BEFORE="$WORK_DIR/codex-config.before"
CODEX_CONFIG_WAS_MISSING="$WORK_DIR/codex-config.before.missing"
rm -f "$CODEX_CONFIG_BEFORE" "$CODEX_CONFIG_WAS_MISSING"
if [[ -f "$CODEX_CONFIG" ]]; then
  cp "$CODEX_CONFIG" "$CODEX_CONFIG_BEFORE"
else
  : >"$CODEX_CONFIG_WAS_MISSING"
fi

snapshot "$WORK_DIR/pre.snapshot"
rm -rf "$WORK_DIR/build-output"

(cd "$ROOT" && LLM_WIKI_INSTANCE=test cargo build --bin llm-wiki --bin poman)
INSTALLED_TEST_INSTANCE=1
(cd "$ROOT" && target/debug/llm-wiki install --non-interactive --disable-llm-search --skip-path-guidance)

MANAGED_BIN="$HOME/.llm_wiki-test/bin/llm-wiki-test"
MANIFEST="$HOME/.llm_wiki-test/manifest.json"

test -x "$MANAGED_BIN"
test -f "$MANIFEST"
grep -qF "$MANAGED_BIN" "$MANIFEST"
grep -q '"installed_by": "llm-wiki-test"' "$MANIFEST"

"$MANAGED_BIN" --version >"$WORK_DIR/managed-version.txt"
grep -q '^llm-wiki-test ' "$WORK_DIR/managed-version.txt"
"$MANAGED_BIN" --help >"$WORK_DIR/managed-help.txt"
grep -q '^Usage: llm-wiki-test' "$WORK_DIR/managed-help.txt"
"$MANAGED_BIN" status >"$WORK_DIR/managed-status.txt"
grep -q '^llm-wiki-test ' "$WORK_DIR/managed-status.txt"
"$PYTHON_BIN" - "$MANAGED_BIN" "$WORK_DIR/managed-mcp-tools.jsonl" <<'PY'
import json
import subprocess
import sys

binary = sys.argv[1]
out = sys.argv[2]
requests = [
    {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "llm-wiki-proof", "version": "0"},
        },
    },
    {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
]
payload = "\n".join(json.dumps(request) for request in requests) + "\n"
completed = subprocess.run(
    [binary, "mcp", "serve"],
    input=payload,
    text=True,
    capture_output=True,
    timeout=30,
    check=True,
)
with open(out, "w", encoding="utf-8") as handle:
    handle.write(completed.stdout)
    if completed.stderr:
        handle.write("\n# stderr\n")
        handle.write(completed.stderr)
if "llm_wiki_read_test" not in completed.stdout:
    raise SystemExit("MCP tools/list did not include llm_wiki_read_test")
PY


CODEX_CONFIG="$HOME/.codex/config.toml"
test -f "$CODEX_CONFIG"
grep -q "llm-wiki-test" "$CODEX_CONFIG"
grep -qF "$MANAGED_BIN" "$CODEX_CONFIG"
if grep -q "llm-wiki" "$CODEX_CONFIG"; then
  grep -n "llm-wiki" "$CODEX_CONFIG" >"$WORK_DIR/production-mcp-entry-present.txt"
fi

(cd "$ROOT" && target/debug/llm-wiki uninstall)
INSTALLED_TEST_INSTANCE=0

if [[ -f "$CODEX_CONFIG_WAS_MISSING" ]]; then
  if [[ -e "$CODEX_CONFIG" ]]; then
    echo "Codex MCP config was created and not removed: $CODEX_CONFIG" >&2
    exit 1
  fi
elif ! cmp -s "$CODEX_CONFIG_BEFORE" "$CODEX_CONFIG"; then
  diff -u "$CODEX_CONFIG_BEFORE" "$CODEX_CONFIG" >"$WORK_DIR/codex-config.diff" || true
  echo "Codex MCP config changed after uninstall; see $WORK_DIR/codex-config.diff" >&2
  exit 1
fi

for path in "${test_state_paths[@]}"; do
  if [[ -e "$path" || -L "$path" ]]; then
    echo "test-instance state survived uninstall: $path" >&2
    exit 1
  fi
done

snapshot "$WORK_DIR/post.snapshot"
if ! cmp -s "$WORK_DIR/pre.snapshot" "$WORK_DIR/post.snapshot"; then
  diff -u "$WORK_DIR/pre.snapshot" "$WORK_DIR/post.snapshot" >"$WORK_DIR/snapshot.diff" || true
  echo "production pre/post snapshots differ; see $WORK_DIR/snapshot.diff" >&2
  exit 1
fi

{
  echo "# Test Instance Live Session Proof"
  echo
  echo "- home: $HOME"
  echo "- work_dir: $WORK_DIR"
  echo "- pre_snapshot: $(head -n 1 "$WORK_DIR/pre.snapshot")"
  echo "- post_snapshot: $(head -n 1 "$WORK_DIR/post.snapshot")"
  echo "- managed_version: $WORK_DIR/managed-version.txt"
  echo "- managed_help: $WORK_DIR/managed-help.txt"
  echo "- managed_status: $WORK_DIR/managed-status.txt"
  echo "- managed_mcp_tools: $WORK_DIR/managed-mcp-tools.jsonl"
  echo "- build_output: $WORK_DIR/build-output"
} >"$WORK_DIR/report.md"

echo "test-instance live-session proof OK"
echo "report: $WORK_DIR/report.md"
