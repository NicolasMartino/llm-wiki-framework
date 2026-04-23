---
name: knowledge-research
description: Research and collect candidate source material into `raw/` before ingest. Use when the user asks to search the web, search one site, gather URLs, import local files, or save approved sources into the project's raw knowledge layer.
---

# Knowledge Research

Gather source material into `raw/` so it can be ingested later. This skill
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

If the user asks for research without a clear mode, ask one short question to
choose among `path`, `url`, `site`, or `web`.

## Required Context

Before gathering sources:

1. Read `wiki/index.md` to understand what the project already knows.
2. Read the 1-3 most relevant wiki pages for the topic when the request is
   topical enough to identify them.
3. Use that context to avoid redundant source collection and to sharpen search
   terms.

## Modes

### Path

Use when the user gives a local file or directory.

1. If the path is already under `raw/`, report that it is ready for ingest.
2. If it is outside `raw/`, copy it into `raw/imports/` using a dated,
   descriptive filename or folder name.
3. Report what was staged into `raw/`.

### URL

Use when the user provides one or more explicit URLs.

1. Fetch each URL.
2. Save each fetched page as markdown under `raw/web/<domain>/`.
3. Prefix the saved file with source metadata:

```markdown
# Source Snapshot

- Title: ...
- URL: ...
- Retrieved: YYYY-MM-DD
- Method: url
```

4. Report the saved raw paths.

### Site

Use when the user wants research constrained to one website.

1. Search only within the specified domain.
2. Collect a compact shortlist of the most relevant pages.
3. Show the shortlist with title, URL, and one-line reason each.
4. Ask which results to save unless the user already said `top N`, `all
   relevant`, or named specific pages.
5. Save approved pages under `raw/web/<domain>/` with source metadata:

```markdown
# Source Snapshot

- Title: ...
- URL: ...
- Retrieved: YYYY-MM-DD
- Method: site
- Domain: ...
- Query: ...
```

### Web

Use when the user wants broader web research.

1. Search the web for the topic.
2. Prefer primary sources when the topic is technical, legal, financial, or
   otherwise sensitive.
3. Build a shortlist of relevant candidate pages.
4. Show the shortlist with title, URL, source, and one-line reason each.
5. Ask which results to save unless the user already specified a selection
   rule such as `top 3`.
6. Save approved pages under `raw/web/<domain>/` with source metadata:

```markdown
# Source Snapshot

- Title: ...
- URL: ...
- Retrieved: YYYY-MM-DD
- Method: web
- Query: ...
```

## Rules

1. Keep `raw/` as the source of truth. Do not write research findings straight
   into `wiki/`.
2. Preserve provenance for every saved source: URL when applicable, retrieval
   date, and the search mode that found it.
3. For `site` and `web` modes, treat search as curation support. Save only
   sources the user approved or clearly delegated you to select.
4. Prefer official docs, standards, maintainers, repositories, and first-party
   announcements over commentary when source quality matters.
5. Do not ingest automatically unless the user explicitly asks for ingest as a
   follow-up step.

## Final Report

Report:

- research mode used
- number of candidate sources reviewed when applicable
- raw files saved
- any notable gaps or ambiguous sources
- whether the material is ready for `$knowledge ingest` or `knowledge-ingest`
