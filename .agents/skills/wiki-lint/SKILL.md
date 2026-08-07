---
name: wiki-lint
description: Run a lint pass on an LLM Wiki project. Use when Codex is asked to lint the wiki, scan for contradictions, stale claims, orphan pages, missing cross-references, or fix wiki bookkeeping issues directly.
---

# /wiki-lint

## Purpose

Run a lint pass over the current project's wiki and fix real bookkeeping
problems directly.

## MCP Routing

When the host exposes the LLM Wiki MCP server, use the framework-owned MCP
tools for wiki/raw access: `llm_wiki_read` for full `wiki/` or `raw/` files,
`llm_wiki_search` for current-project retrieval, `llm_wiki_search_all` only for
cross-project retrieval, and `llm_wiki_index` after lint changes wiki content.
Shell `llm-wiki ...` commands and direct wiki/raw file reads are fallback only
when MCP is unavailable. Direct file edits remain the mutation mechanism for
fixing wiki pages, `wiki/index.md`, and `wiki/log.md`.

## Behavior

1. Require `wiki/index.md` in the current working directory. If it is missing,
   tell the user the project has not been initialized with the LLM Wiki
   framework.
2. Before changing anything, read `wiki/index.md` and relevant wiki pages via
   `llm_wiki_read` when MCP is exposed. Use `llm_wiki_search` to identify
   candidate pages when the project is registered. Read `project_guidelines.md`
   from an MCP resource if exposed, otherwise directly.
3. Look for contradictions between wiki pages, stale statuses or claims,
   orphan pages not linked from `wiki/index.md`, missing cross-references
   between related pages, and index entries that are missing, wrong, or stale.
4. Read the minimum set of pages needed to confirm each issue.
5. If an issue is clear and mechanical, fix it directly. If pages make
   conflicting substantive claims and the correct answer is not documented
   anywhere, stop and ask the user to resolve the conflict.
6. Update `wiki/index.md` when the catalog needs correction.
7. Append a `lint` entry to `wiki/log.md` listing issues found, pages updated,
   and outstanding questions.
8. Refresh the search index with `llm_wiki_index` when MCP is exposed and wiki
   content changed; use shell `llm-wiki index --force` only as fallback.

## Invocation

Use normal language or an explicit skill invocation:

- `/wiki-lint lint the wiki`
- `/wiki-lint scan for stale statuses`

## Notes

Prefer narrow edits over broad rewrites. Report residual risk or unanswered
conflicts clearly.
