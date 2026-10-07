#!/usr/bin/env bash
#
# Runs the hand-run search eval (`tests/natural_language_search_eval.rs`, ignored
# by default) in a temporary home, so it needs nothing registered on the machine
# and leaves the machine's llm-wiki setup as it was. `just search-eval` runs it;
# see wiki/plans/search-eval-in-a-temporary-home.plan.md.
#
# The temporary home gets copies of the real managed home's small records: the
# install manifest (search refuses to run without one), the model records
# (`models/artifacts.toml`), the accepted licenses and the search settings. The
# model records name the real home's model files by absolute path, so index and
# search read those files where they are, never copied (1.6 GB) and never written.
# The eval's project is a copy of this checkout's wiki/ and AGENTS.MD (register
# wants an orientation file), with the home's default search settings as its
# own, registered with `--no-mcp` (no `.mcp.json` pointing at a binary that
# leaves with the home) and indexed. Then the ignored eval runs against it,
# writing its report under target/evals/, and the temporary directory goes,
# whether the eval passed or not.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
project_id=llm-wiki-framework-semantic-search
real=$HOME/.llm_wiki

missing() {
  echo "search-eval needs the managed search models with meaning-based search on: run \`llm-wiki install --configure-search\` first ($1)" >&2
  exit 1
}
command -v python3 >/dev/null 2>&1 || {
  echo "search-eval needs python3 (3.11 or newer) on PATH, to read search.toml" >&2
  exit 1
}
for file in manifest.json models/artifacts.toml accepted-licenses.toml search.toml; do
  [ -f "$real/$file" ] || missing "$real/$file is missing"
done
# Read as TOML, so a hand-edited file says the same here as it does to
# llm-wiki: `check` prints why search is off, `project` prints the home's
# default settings as a project file.
settings() { # check|project <search.toml>
  python3 - "$@" <<'PY'
import json, re, sys
try:
    import tomllib
except ImportError:
    sys.exit("search-eval needs python3 3.11 or newer, to read search.toml")
mode, path = sys.argv[1:]
try:
    with open(path, "rb") as f:
        home = tomllib.load(f)
except tomllib.TOMLDecodeError as error:
    print(f"{path} is not valid TOML: {error}")
    sys.exit(1)
default = home.get("project_default")
if mode == "check":
    if not isinstance(default, dict) or default.get("llm_search_enabled") is not True:
        print(f"{path} has meaning-based search off")
        sys.exit(1)
    sys.exit(0)
def value(v):
    if isinstance(v, (bool, int, float, str)):
        return json.dumps(v)
    sys.exit(f"search-eval cannot copy the value {v!r} from {path}")
for key in ("schema_version", "updated_at"):
    if key in home:
        print(f"{key} = {value(home[key])}")
print("\n[project]")
for key, v in default.items():
    name = key if re.fullmatch(r"[A-Za-z0-9_-]+", key) else json.dumps(key)
    print(f"{name} = {value(v)}")
PY
}
if ! why=$(settings check "$real/search.toml"); then
  [ -n "$why" ] || exit 1
  missing "$why"
fi

# The build and the eval must not take the test instance's names and folders.
unset LLM_WIKI_INSTANCE
# rustup and cargo find their homes from HOME unless told; keep the real ones.
export CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
export RUSTUP_HOME=${RUSTUP_HOME:-$HOME/.rustup}

cd "$repo"
cargo build --bin llm-wiki
# Through cargo, so the binary is the one just built wherever the target
# directory is.
wiki() { cargo run --quiet --bin llm-wiki -- "$@"; }

tmp=$(mktemp -d "${TMPDIR:-/tmp}/llm-wiki-search-eval.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

home=$tmp/home
managed=$home/.llm_wiki
mkdir -p "$managed/models"
cp "$real/models/artifacts.toml" "$managed/models/artifacts.toml"
for file in manifest.json accepted-licenses.toml search.toml search-thresholds.toml search-runtime-probes.toml; do
  [ ! -f "$real/$file" ] || cp "$real/$file" "$managed/$file"
done

# The project's own search settings: the home's default, as a project file
# (llm-wiki refuses a partial file).
root=$tmp/project
mkdir -p "$root/.llm_wiki"
cp -R "$repo/wiki" "$root/wiki"
cp "$repo/AGENTS.MD" "$root/AGENTS.MD"
settings project "$managed/search.toml" > "$root/.llm_wiki/search.toml"

export HOME=$home
export XDG_CACHE_HOME=$home/.cache
export XDG_DATA_HOME=$home/.local/share
export XDG_CONFIG_HOME=$home/.config
export XDG_STATE_HOME=$home/.local/state

echo "search-eval: registering and indexing $project_id in $home"
wiki register "$root" --id "$project_id" --name "$project_id" --no-mcp
wiki index --project "$project_id"

cargo test --test natural_language_search_eval -- --ignored --nocapture
