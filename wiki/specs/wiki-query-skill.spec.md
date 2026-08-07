# Wiki Query Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-08
- Category: Tooling
- Scope: Query the project wiki and answer with citations.
- Sources: .claude/skills/wiki-query/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/decisions/wiki-query-search-first.decision.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/wiki-ingest-skill.spec.md, wiki/evals/natural-language-search.eval.md

## Contract

`wiki-query` answers from the compiled wiki. It reads `wiki/index.md`
first for orientation, uses `llm_wiki_search` for every query when the MCP
server is exposed and the current project is registered, falls back to
`llm-wiki search --mode auto --format json` only when MCP is unavailable, reads
relevant wiki pages through `llm_wiki_read` when possible, cites wiki paths,
flags gaps and contradictions, and offers save-back when a synthesized answer
creates durable knowledge.

The skill must not browse the filesystem for project knowledge. The index is
the entry point, but search is the default retrieval attempt after index
orientation when the current project is registered.

For every query, the skill queries with the `llm_wiki_search` MCP tool when the
host exposes the MCP server, falling back to shell
`llm-wiki search --mode auto --format json "<question>"` otherwise. It must
inspect `selected_mode`, `readiness_reason`, `fallback_reason`,
`zero_result_reason`, and per-result mode/backend metadata before trusting the
result set. Search snippets are navigation aids only; the skill still reads and
cites the returned wiki pages directly (via `llm_wiki_read`).

If semantic/hybrid readiness is missing, the project is unregistered, the index
is missing or stale, or search returns no useful result, the skill continues
from index-based navigation instead of treating the search failure or
zero-result outcome as an answer.

When the skill uses `search-all` (or the `llm_wiki_search_all` MCP tool) as an
explicit cross-project navigation supplement, it must inspect the JSON
`projects` array as well as the top-level mode fields so skipped projects,
lexical fallback, and mixed readiness do not get mistaken for corpus-wide
absence.

## Skill Source

Authored source: `.claude/skills/wiki-query/SKILL.md`, a repo-local Claude Code
skill. The MCP-first surface no longer renders Claude/Codex runtime variants or
an `openai.yaml` dispatcher. Hosts read and search the wiki through the
`llm_wiki_read`, `llm_wiki_search`, and `llm_wiki_search_all` MCP tools.

Invocation:

- Claude Code: `/wiki-query <question>` (repo-local skill)

## Proven By

- The `llm_wiki_search` / `llm_wiki_search_all` MCP tools return the readiness,
  fallback, and zero-result metadata the skill inspects.
- `wiki/evals/natural-language-search.eval.md` measures the underlying search.
