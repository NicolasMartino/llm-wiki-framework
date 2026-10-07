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
for file in manifest.json models/artifacts.toml accepted-licenses.toml search.toml; do
  [ -f "$real/$file" ] || missing "$real/$file is missing"
done
awk '/^\[/ { section = $0 } section == "[project_default]" && $0 == "llm_search_enabled = true" { on = 1 } END { exit !on }' "$real/search.toml" \
  || missing "$real/search.toml has meaning-based search off"

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
awk '/^\[/ { keep = ($0 == "[project]" || $0 == "[project_default]") } keep || /^(schema_version|updated_at) /' "$managed/search.toml" \
  | sed -e 's/^\[project_default\]$/[project]/' \
  > "$root/.llm_wiki/search.toml"

export HOME=$home
export XDG_CACHE_HOME=$home/.cache
export XDG_DATA_HOME=$home/.local/share
export XDG_CONFIG_HOME=$home/.config
export XDG_STATE_HOME=$home/.local/state

echo "search-eval: registering and indexing $project_id in $home"
wiki register "$root" --id "$project_id" --name "$project_id" --no-mcp
wiki index --project "$project_id"

cargo test --test natural_language_search_eval -- --ignored --nocapture
