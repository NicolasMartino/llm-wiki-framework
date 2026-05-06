#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
root_dir="$(cd "${script_dir}/.." && pwd -P)"

render_skill() {
  local target="$1"
  local src="$2"
  local dst="$3"

  mkdir -p "$(dirname "$dst")"
  awk -v target="$target" '
    /^<!-- CLAUDE -->$/ { mode = "CLAUDE"; next }
    /^<!-- CODEX -->$/ { mode = "CODEX"; next }
    /^<!-- END -->$/ { mode = ""; next }
    mode == "" || mode == target {
      if ($0 == "") {
        pending = pending "\n"
        next
      }
      printf "%s%s\n", pending, $0
      pending = ""
    }
  ' "$src" > "$dst"
}

copy_codex_config() {
  local skill_name="$1"
  local src="${root_dir}/skills/${skill_name}/codex/openai.yaml"
  local dst="${root_dir}/.codex/skills/${skill_name}/agents/openai.yaml"

  if [[ -f "$src" ]]; then
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
  fi
}

for skill_dir in "${root_dir}"/skills/*; do
  [[ -d "$skill_dir" ]] || continue
  skill_name="$(basename "$skill_dir")"
  src="${skill_dir}/SKILL.md"
  [[ -f "$src" ]] || continue

  if [[ "$skill_name" != "knowledge" ]]; then
    render_skill "CLAUDE" "$src" "${root_dir}/.claude/skills/${skill_name}/SKILL.md"
  fi

  render_skill "CODEX" "$src" "${root_dir}/.codex/skills/${skill_name}/SKILL.md"
  copy_codex_config "$skill_name"
done
