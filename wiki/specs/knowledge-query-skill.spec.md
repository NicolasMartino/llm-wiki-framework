# Knowledge Query Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-06
- Category: Tooling
- Scope: The `knowledge-query` skill for querying a project's wiki with citations and optional save-back.
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/init-project-skill.spec.md, wiki/decisions/single-source-skills.decision.md

## What It Does

The knowledge-query skill searches the current project's wiki, synthesizes an
answer with citations, and optionally saves durable answers back as wiki pages.
In Codex, it triggers from normal requests to check the wiki or answer from
documented project knowledge. It also supports explicit invocation through
`$knowledge-query` and the namespace alias `$knowledge query`.

## Scope

Single project only. Queries the `wiki/` directory in the current working
directory. Does not search across multiple projects.

## Location

Canonical source: `skills/knowledge-query/SKILL.md`
Claude generated skill: `.claude/skills/knowledge-query/SKILL.md`
Codex generated skill: `.codex/skills/knowledge-query/SKILL.md`
Codex UI metadata source: `skills/knowledge-query/codex/openai.yaml`
Claude global access: symlinked to `~/.claude/skills/knowledge-query`
Codex global access: symlinked to `~/.codex/skills/knowledge-query`
Codex namespace alias: `$knowledge query`

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

- Claude skill file exists at `.claude/skills/knowledge-query/SKILL.md`
- Codex skill file exists at `.codex/skills/knowledge-query/SKILL.md`
- Canonical skill source exists at `skills/knowledge-query/SKILL.md`
- `bash skills/build.sh` renders the Claude and Codex outputs
- Codex global symlink exists at `~/.codex/skills/knowledge-query`
- Codex dispatcher skill exists at `.codex/skills/knowledge/SKILL.md`
- Claude global symlink exists at `~/.claude/skills/knowledge-query`
- Not yet tested with a real query (see roadmap D4)

## Limitations

- Single project scope only (no cross-project queries)
- QMD integration is optional (falls back to index.md scanning)
- Does not yet support query history or caching
