# Knowledge Research Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-06
- Category: Tooling
- Scope: The `knowledge-research` skill as the guided intake surface for gathering source material into `raw/research/` before ingest.
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md, wiki/decisions/knowledge-research-intake.decision.md, wiki/plans/knowledge-research-intake.plan.md

## What It Does

The knowledge-research skill gathers source material for a research task and
saves it as a coherent research bundle under `raw/research/`.

It is the framework's intake surface for pre-ingest research. When the user
starts with a broad or underspecified request, the skill asks a short scoping
exchange, gathers source material from local paths, explicit URLs, one
website, or broader web search, and writes a bundle with:

- raw source files or snapshots
- `manifest.md` for provenance and inventory
- `research-summary.md` for bundle-level synthesis

It does not update `wiki/` directly. Its output is staged source material that
can later be processed by `knowledge-ingest`.

## Scope

Single project only. Research bundles are written under the `raw/research/`
directory of the current working directory.

## Location

Canonical source: `skills/knowledge-research/SKILL.md`
Claude generated skill: `.claude/skills/knowledge-research/SKILL.md`
Codex generated skill: `.codex/skills/knowledge-research/SKILL.md`
Codex UI metadata source: `skills/knowledge-research/codex/openai.yaml`
Claude global access: symlinked to `~/.claude/skills/knowledge-research`
Codex global access: symlinked to `~/.codex/skills/knowledge-research`
Codex namespace alias: `$knowledge research`

## Invocation

In Codex, it can trigger from normal requests to research or gather sources,
or through explicit skill invocation:

- `Use $knowledge-research on path ./notes`
- `Use $knowledge-research on url https://example.com/spec`
- `Use $knowledge to research site docs.example.com auth tokens`
- `Use $knowledge to research web vector database benchmarks`

In Claude, it can trigger through direct skill invocation:

- `/knowledge-research ./notes`
- `/knowledge-research https://example.com/spec`
- `/knowledge-research docs.example.com auth tokens`

If the request is broad or does not make the source mode clear, the skill asks
one short intake exchange to determine:

1. the research question or topic
2. the goal of the research
3. any source or scope constraints
4. whether the user wants the agent to shortlist or auto-select sources

## Intake Workflow

1. Read `wiki/index.md` to understand existing project knowledge.
2. Read the 1-3 most relevant wiki pages for the topic when needed.
3. Clarify the research task if the request is broad or ambiguous.
4. Determine the source mode: `path`, `url`, `site`, or `web`.
5. Gather candidate sources.
6. For `site` and `web`, present a shortlist unless the user already provided
   a clear selection rule.
7. Create a dated research bundle under `raw/research/YYYY-MM-DD-topic-slug/`.
8. Save the approved source files or snapshots into `sources/`.
9. Write `manifest.md` and `research-summary.md`.
10. Report the saved bundle path and whether it is ready for ingest.

## Modes

| Mode | Behavior |
| --- | --- |
| `path` | Copy local files or folders into the bundle `sources/` directory unless already inside the target bundle |
| `url` | Fetch explicit URLs and save snapshots into the bundle `sources/` directory |
| `site` | Search within one domain, shortlist candidates, save approved pages into the bundle |
| `web` | Search broadly, shortlist candidates, save approved pages into the bundle |

## Bundle Shape

Each research run produces one bundle:

```text
raw/research/YYYY-MM-DD-topic-slug/
  manifest.md
  research-summary.md
  sources/
    01-source-name.md
    02-source-name.md
```

The bundle directory is the unit of handoff into later ingest or human review.

## Manifest Requirements

`manifest.md` records:

- research question
- research goal
- bundle date
- source mode or modes used
- source inventory with original location or URL
- retrieval date for fetched sources
- selection rationale for each source
- any important exclusions or gaps

## Research Summary Requirements

`research-summary.md` captures:

- the question being researched
- scope and constraints
- the source set reviewed
- key findings across the sources
- important disagreements, caveats, or open questions
- ingest recommendations or next steps

## Provenance Rules

The skill preserves provenance for every saved source. At minimum:

- local source path or original URL
- retrieval date when applicable
- acquisition method (`path`, `url`, `site`, or `web`)
- query and domain when applicable

This preserves the framework's raw-to-wiki provenance chain.

## Relationship to Ingest

Research is a pre-ingest acquisition step. It collects or stages source
material into `raw/research/`, while ingest compiles explicit raw sources into
typed wiki pages.

Requests to search the web or fetch site content should use `knowledge-research`
first, not `knowledge-ingest`.

## Key Behaviors

- **Guided intake** — asks a short scoping exchange for broad requests
- **Research bundles** — saves each run as a coherent package under
  `raw/research/`
- **Bundle-level synthesis** — writes one `research-summary.md` per run
- **Provenance** — tracks where each saved source came from and why it was kept
- **Explicit stop point** — does not ingest automatically unless the user
  asks for ingest separately

## Proven By

- Claude skill file exists at `.claude/skills/knowledge-research/SKILL.md`
- Codex skill file exists at `.codex/skills/knowledge-research/SKILL.md`
- Canonical skill source exists at `skills/knowledge-research/SKILL.md`
- `bash skills/build.sh` renders the Claude and Codex outputs
- Claude global symlink exists at `~/.claude/skills/knowledge-research`
- Codex UI metadata exists at `.codex/skills/knowledge-research/agents/openai.yaml`
- Codex global symlink exists at `~/.codex/skills/knowledge-research`
- The dispatcher namespace includes `$knowledge research`
- A research proof bundle exists under `raw/research/2026-05-02-knowledge-research-intake-proof/`

## Limitations

- Search result selection may still need user confirmation when the request is
  broad or ambiguous
- No autonomous crawl scheduling or recurring monitoring
- Bundle summaries can still be shallow if the source set is large or noisy
- The skill stages source material only; it does not update `wiki/` directly
