# Knowledge Query Skill

- Document Class: Spec
- Status: Active
- Date: 2026-04-23
- Category: Tooling
- Scope: The /knowledge-query skill for querying a project's wiki with citations and optional save-back.
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/init-project-skill.spec.md

## What It Does

`/knowledge-query <question>` searches the current project's wiki, synthesizes
an answer with citations, and optionally saves durable answers back as wiki pages.

## Scope

Single project only. Queries the `wiki/` directory in the current working
directory. Does not search across multiple projects.

## Location

Skill definition: `.claude/skills/knowledge-query/SKILL.md`
Global access: symlinked to `~/.claude/skills/knowledge-query`

## How It Works

1. Reads `wiki/index.md` to orient
2. Identifies relevant pages by document type and keyword match
3. Optionally uses QMD hybrid search if available and index is large
4. Reads full content of relevant pages (3-5 initially, more if needed)
5. Synthesizes answer with wiki page citations and transitive raw source citations
6. Flags gaps (missing information) and contradictions (disagreeing pages)
7. Offers to save durable synthesis as a new wiki page

## Key Behaviors

- **Stays within the wiki** — does not speculate beyond documented knowledge
- **Cites sources** — every claim references the wiki page it came from
- **Flags gaps** — explicitly says when the wiki lacks information
- **Flags contradictions** — surfaces disagreements rather than choosing silently
- **Save-back** — durable answers can become wiki pages (completes the knowledge compounding loop)

## Proven By

- Skill file exists at `.claude/skills/knowledge-query/SKILL.md`
- Global symlink exists at `~/.claude/skills/knowledge-query`
- Not yet tested with a real query (see roadmap D4)

## Limitations

- Single project scope only (no cross-project queries)
- QMD integration is optional (falls back to index.md scanning)
- Does not yet support query history or caching
