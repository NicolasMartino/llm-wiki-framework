#!/usr/bin/env bash
#
# Runs the hand-run search eval (`tests/natural_language_search_eval.rs`, ignored
# by default) in a temporary home, so it needs nothing registered on the machine
# and leaves the machine's llm-wiki setup as it was. `just search-eval` runs it;
# see wiki/plans/search-eval-in-a-temporary-home.plan.md.
#
# The temporary home gets:
# - the real managed home's models, linked file by file and never copied (1.6 GB),
#   with their small records copied; search and index only read the models;
# - copies of the real home's install manifest (search refuses to run without
#   one), accepted licenses and search settings;
# - the eval's project: copies of this checkout's wiki/ and AGENTS.MD, with its
#   own search settings turning meaning-based search on, registered with
#   `--no-mcp` (no `.mcp.json` pointing at a binary that leaves with the home)
#   and indexed.
# Then the ignored eval runs against it, writing its report under target/evals/,
# and the temporary directory goes, whether the eval passed or not.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
project_id=llm-wiki-framework-semantic-search
real=$HOME/.llm_wiki

for file in manifest.json models/artifacts.toml accepted-licenses.toml search.toml; do
  [ -f "$real/$file" ] || {
    echo "search-eval needs the managed search models: run \`llm-wiki install --configure-search\` first ($real/$file is missing)" >&2
    exit 1
  }
done

# The build and the eval must not take the test instance's names and folders.
unset LLM_WIKI_INSTANCE
# rustup and cargo find their homes from HOME unless told; keep the real ones.
export CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
export RUSTUP_HOME=${RUSTUP_HOME:-$HOME/.rustup}

cd "$repo"
cargo build --bin llm-wiki
wiki=$repo/target/debug/llm-wiki

tmp=$(mktemp -d "${TMPDIR:-/tmp}/llm-wiki-search-eval.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

home=$tmp/home
managed=$home/.llm_wiki
mkdir -p "$managed"

# The models: every folder made, every model file linked, every record copied.
(cd "$real/models" && find . -type d) | while IFS= read -r dir; do
  mkdir -p "$managed/models/$dir"
done
(cd "$real/models" && find . -type f) | while IFS= read -r file; do
  case "$file" in
    *.toml | *.json) cp "$real/models/$file" "$managed/models/$file" ;;
    *) ln -s "$real/models/$file" "$managed/models/$file" ;;
  esac
done
for file in manifest.json accepted-licenses.toml search.toml search-thresholds.toml search-runtime-probes.toml; do
  [ ! -f "$real/$file" ] || cp "$real/$file" "$managed/$file"
done

# The project: its own wiki and orientation file (register wants one), with the
# home's default search settings as its own and meaning-based search on
# (llm-wiki refuses a partial file).
root=$tmp/project
mkdir -p "$root/.llm_wiki"
cp -R "$repo/wiki" "$root/wiki"
cp "$repo/AGENTS.MD" "$root/AGENTS.MD"
awk '/^\[/ { keep = ($0 == "[project]" || $0 == "[project_default]") } keep || /^(schema_version|updated_at) /' "$managed/search.toml" \
  | sed -e 's/^\[project_default\]$/[project]/' -e 's/^llm_search_enabled = false$/llm_search_enabled = true/' \
  > "$root/.llm_wiki/search.toml"

export HOME=$home
export XDG_CACHE_HOME=$home/.cache
export XDG_DATA_HOME=$home/.local/share
export XDG_CONFIG_HOME=$home/.config
export XDG_STATE_HOME=$home/.local/state

echo "search-eval: registering and indexing $project_id in $home"
"$wiki" register "$root" --id "$project_id" --name "$project_id" --no-mcp
"$wiki" index --project "$project_id"

cargo test --test natural_language_search_eval -- --ignored --nocapture
