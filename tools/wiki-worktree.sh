#!/usr/bin/env bash
#
# Gives an Orca worktree its own wiki search, and takes it away again.
#
# `orca.yaml` runs `register` after Orca makes a worktree and `forget` before it
# removes one. Without this, `llm_wiki_search` fails in every worktree ("not
# registered"), so workers fall back to grepping the wiki.
#
# A worktree gets a word-match index only: it builds in about a second with
# about 5 MB, where the main checkout's meaning-based index takes minutes and
# more than 1 GB, which three workers starting at once cannot afford. Search
# rebuilds that word-match index itself once pages change.
#
# `register` also forgets this repository's registrations whose worktree is
# gone: a worktree removed without Orca's archive script leaves one behind.
#
# It does nothing unless the main checkout is registered with llm-wiki, and it
# always exits 0: a failed setup script fails the worker's start, and a failed
# archive script blocks the worktree's removal.
set -uo pipefail

wiki=$HOME/.llm_wiki/bin/llm-wiki
root=${ORCA_ROOT_PATH:-}
tree=${ORCA_WORKTREE_PATH:-}
[ -x "$wiki" ] && [ -n "$root" ] && [ -n "$tree" ] && [ "$tree" != "$root" ] || exit 0
id="$(basename "$root")--$(basename "$tree")"

main_registered() {
  "$wiki" projects 2>/dev/null | awk -F'\t' -v r="$root" '$3 == r { f = 1 } END { exit !f }'
}

forget_gone() {
  "$wiki" projects 2>/dev/null \
    | awk -F'\t' -v p="$(basename "$root")--" 'index($1, p) == 1 && $4 == "root-missing" { print $1 }' \
    | while IFS= read -r gone; do
        "$wiki" forget "$gone" --delete-cache >/dev/null 2>&1 && echo "wiki-worktree: forgot $gone (its worktree is gone)"
      done
}

case "${1:-}" in
  register)
    main_registered || { echo "wiki-worktree: $root is not registered with llm-wiki; nothing to do"; exit 0; }
    forget_gone
    [ -f "$tree/wiki/index.md" ] || exit 0
    # The main checkout's search settings (or the machine's default) with the
    # meaning-based search turned off; llm-wiki refuses a partial file.
    src=$root/.llm_wiki/search.toml
    [ -f "$src" ] || src=$HOME/.llm_wiki/search.toml
    mkdir -p "$tree/.llm_wiki"
    awk '/^\[/ { keep = ($0 == "[project]" || $0 == "[project_default]") } keep || /^(schema_version|updated_at) /' "$src" \
      | sed -e 's/^\[project_default\]$/[project]/' -e 's/^llm_search_enabled = true$/llm_search_enabled = false/' \
      > "$tree/.llm_wiki/search.toml"
    "$wiki" forget "$id" --delete-cache >/dev/null 2>&1
    if "$wiki" register "$tree" --id "$id" --name "$id" --no-mcp >/dev/null 2>&1 \
       && "$wiki" index --project "$id" >/dev/null 2>&1; then
      echo "wiki-worktree: registered and indexed $id"
    else
      echo "wiki-worktree: could not register $id; wiki search will not work in this worktree" >&2
    fi
    ;;
  forget)
    "$wiki" forget "$id" --delete-cache >/dev/null 2>&1 && echo "wiki-worktree: forgot $id"
    ;;
  *)
    echo "usage: $0 register|forget (run by orca.yaml)" >&2
    ;;
esac
exit 0
