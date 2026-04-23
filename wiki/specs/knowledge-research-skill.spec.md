# Knowledge Research Skill

- Document Class: Spec
- Status: Active
- Date: 2026-04-23
- Category: Tooling
- Scope: The `knowledge-research` skill and `$knowledge research` namespace entry for gathering source material into `raw/` before ingest.
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md

## What It Does

The knowledge-research skill helps the user gather candidate sources from local
paths, explicit URLs, one website, or broader web search and save approved
material into the project's `raw/` layer.

It does not update `wiki/` directly. Its output is staged source material that
can later be processed by `knowledge-ingest`.

## Invocation

In Codex, it can trigger from normal requests to research or gather sources,
or through explicit skill invocation:

- `Use $knowledge-research on path ./notes`
- `Use $knowledge-research on url https://example.com/spec`
- `Use $knowledge to research site docs.example.com auth tokens`
- `Use $knowledge to research web vector database benchmarks`

If the request does not make the source mode clear, the skill asks one short
question to choose `path`, `url`, `site`, or `web`.

## Modes

| Mode | Behavior |
| --- | --- |
| `path` | Copy local files or folders into `raw/imports/` unless already under `raw/` |
| `url` | Fetch explicit URLs and save snapshots under `raw/web/<domain>/` |
| `site` | Search within one domain, shortlist candidates, save approved pages |
| `web` | Search broadly, shortlist candidates, save approved pages |

## Workflow

1. Read `wiki/index.md` to understand existing project knowledge.
2. Read the 1-3 most relevant wiki pages for the topic when needed.
3. Determine the research mode from the user request or ask once.
4. Gather candidate sources.
5. For `site` and `web`, present a shortlist unless the user already provided
   a selection rule.
6. Save approved sources into `raw/` with provenance metadata.
7. Report the saved raw paths and whether they are ready for ingest.

## Provenance Rules

Saved web snapshots must preserve:

- source URL
- retrieval date
- acquisition method (`url`, `site`, or `web`)
- originating query and domain when applicable

This preserves the framework's raw-to-wiki provenance chain.

## Relationship to Ingest

Research is a pre-ingest acquisition step. It collects or stages source
material into `raw/`, while ingest compiles explicit raw sources into typed
wiki pages.

Requests to search the web or fetch site content should use `knowledge-research`
first, not `knowledge-ingest`.

## Proven By

- Codex skill file exists at `.codex/skills/knowledge-research/SKILL.md`
- Codex UI metadata exists at `.codex/skills/knowledge-research/agents/openai.yaml`
- The dispatcher namespace includes `$knowledge research`

## Limitations

- Search result selection may still need user confirmation when the request is
  broad or ambiguous.
- No autonomous crawl scheduling or recurring monitoring
- Saves source snapshots into `raw/` but does not ingest automatically
