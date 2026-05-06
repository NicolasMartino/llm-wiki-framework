---
name: knowledge-research
description: Gather source material into `raw/research/` through a guided intake flow. Use when the user asks what to research, wants web or site research, provides local files or URLs to collect, or wants to prepare a research bundle for later ingest.
---

# Knowledge Research

Gather source material into `raw/research/` so it can be ingested later. This
skill is the framework's guided intake surface for pre-ingest research. It
does research and source acquisition; it does not update `wiki/` directly.

## Scope

Operate on one project at a time. Require `wiki/index.md` and `raw/` in the
current working directory. If they are missing, tell the user to initialize
the project first.

## Invocation

Support natural language and explicit `$knowledge-research` or `$knowledge`
invocation:

- `Use $knowledge-research on path ./notes`
- `Use $knowledge-research on url https://example.com/spec`
- `Use $knowledge to research site docs.example.com auth tokens`
- `Use $knowledge to research web vector database benchmarks`

If the request is broad or the source mode is unclear, ask one short intake
exchange to determine:

- the research question
- the goal or deliverable
- any source or scope constraints
- whether to shortlist or auto-select sources

## Required Context

Before gathering sources:

1. Read `wiki/index.md` to understand what the project already knows.
2. Read the 1-3 most relevant wiki pages for the topic when the request is
   topical enough to identify them.
3. Use that context to avoid redundant source collection and to sharpen search
   terms.

## Bundle Output

Each research run creates one bundle:

```text
raw/research/YYYY-MM-DD-topic-slug/
  manifest.md
  research-summary.md
  sources/
    01-source-name.md
    02-source-name.md
```

Use a short topic slug derived from the research question. If a closely
matching bundle already exists for the same day and topic, extend it rather
than creating a duplicate only when the user clearly intends the same run.

## Modes

### Path

Use when the user gives a local file or directory.

1. Create the research bundle.
2. Copy the provided file or directory into `sources/` using numbered,
   descriptive names.
3. Record the original path and acquisition method in `manifest.md`.
4. Include the source in `research-summary.md`.

### URL

Use when the user provides one or more explicit URLs.

1. Create the research bundle.
2. Fetch each URL.
3. Save each fetched page as markdown in `sources/`.
4. Prefix each saved file with source metadata:

```markdown
# Source Snapshot

- Title: ...
- URL: ...
- Retrieved: YYYY-MM-DD
- Method: url
```

5. Record each source in `manifest.md`.
6. Include the source in `research-summary.md`.

### Site

Use when the user wants research constrained to one website.

1. Search only within the specified domain.
2. Collect a compact shortlist of the most relevant pages.
3. Show the shortlist with title, URL, and one-line reason each.
4. Ask which results to save unless the user already said `top N`, `all
   relevant`, or named specific pages.
5. Create the research bundle.
6. Save approved pages in `sources/` with source metadata:

```markdown
# Source Snapshot

- Title: ...
- URL: ...
- Retrieved: YYYY-MM-DD
- Method: site
- Domain: ...
- Query: ...
```

7. Record the query, domain, and selected pages in `manifest.md`.
8. Synthesize the run in `research-summary.md`.

### Web

Use when the user wants broader web research.

1. Search the web for the topic.
2. Prefer primary sources when the topic is technical, legal, financial, or
   otherwise sensitive.
3. Build a shortlist of relevant candidate pages.
4. Show the shortlist with title, URL, source, and one-line reason each.
5. Ask which results to save unless the user already specified a selection
   rule such as `top 3`.
6. Create the research bundle.
7. Save approved pages in `sources/` with source metadata:

```markdown
# Source Snapshot

- Title: ...
- URL: ...
- Retrieved: YYYY-MM-DD
- Method: web
- Query: ...
```

8. Record the query and selected pages in `manifest.md`.
9. Synthesize the run in `research-summary.md`.

## Required Bundle Files

### manifest.md

Record:

- research question
- research goal
- date
- source mode or modes used
- source inventory with original path or URL
- retrieval date when applicable
- selection rationale for each source
- notable gaps or exclusions

### research-summary.md

Capture:

- research question
- scope and constraints
- source set reviewed
- key findings across the saved sources
- notable disagreements, caveats, or open questions
- whether the bundle appears ready for ingest

## Rules

1. Keep `raw/` as the source of truth. Do not write research findings straight
   into `wiki/`.
2. Preserve provenance for every saved source: original path or URL,
   retrieval date when applicable, and the acquisition method that found it.
3. For `site` and `web` modes, treat search as curation support. Save only
   sources the user approved or clearly delegated you to select.
4. Prefer official docs, standards, maintainers, repositories, and first-party
   announcements over commentary when source quality matters.
5. Do not ingest automatically unless the user explicitly asks for ingest as a
   follow-up step.
6. Produce one `research-summary.md` per research run, not one summary per
   source by default.

## Final Report

Report:

- research mode used
- number of candidate sources reviewed when applicable
- bundle path created or updated
- source files saved
- manifest and summary files written
- any notable gaps or ambiguous sources
- whether the material is ready for `$knowledge ingest` or `knowledge-ingest`
