---
name: knowledge-ingest
description: Ingest explicit raw source material into an LLM Wiki project. Use when Codex is asked to process files or directories into the wiki, add new information from raw/ to project knowledge, compile notes, PRDs, specs, research, transcripts, code observations, images, or PDFs into typed wiki pages, or update wiki/index.md and wiki/log.md after new raw sources arrive.
---

# Knowledge Ingest

Compile source material into the project wiki so future answers can use
organized, cited knowledge instead of re-reading raw sources.

## Scope

Operate on the current project only. Require `wiki/index.md` in the current
working directory. If it is missing, tell the user to initialize the project
with the LLM Wiki framework first.

## Invocation

Support normal requests and explicit invocation:

- `Use $knowledge-ingest to process new raw sources`
- `Use $knowledge to ingest raw/product/prd.md`

## Source Selection

If the user provides URLs or asks to search the web or a site, use
`knowledge-research` first so the fetched material is saved into `raw/`
before ingest.

If the user provides a path:

1. If it is outside `raw/`, copy it into `raw/` under an appropriate
   subdirectory by source type or date.
2. If it is already inside `raw/`, ingest it in place.
3. If it is a directory, identify contained sources and ingest them one by
   one. Copy outside files into `raw/` first.

If the user provides no path:

1. Read `wiki/log.md`.
2. Identify raw files that do not appear in prior ingest log entries.
3. If no new files exist, report that there is nothing new to process.
4. If new files exist, list them and ask whether to ingest all or a subset.

## Required Context

Before processing sources, read:

1. `wiki/index.md` for existing pages
2. `project_guidelines.md` or `project_guidelines.template.md` for document
   roles and conventions
3. Existing wiki pages that appear to cover the source topic

Use the index as the entry point. Avoid filesystem browsing for project
knowledge beyond the explicit source-selection step.

## Three-Phase Pipeline

Process one source completely before moving to the next.

### Phase 1: Extraction

Read the source fully. Extract:

- Key facts and claims
- Named entities, systems, APIs, modules, people, and concepts
- Relationships such as depends-on, uses, replaces, contradicts, or supersedes
- Decisions made or implied
- Open questions and uncertainties
- Status information

Produce a compact working summary before drafting pages.

### Phase 2: Page Drafting

For each significant finding, decide whether to update an existing page or
create a new page.

Update an existing page when it already covers the entity or topic. Merge the
new information, add the raw source to `Sources`, update `Date`, and preserve
valid older context.

Create a new page when no existing page covers the topic. Choose document type
by role:

| Finding type | Document type |
| --- | --- |
| Validated fact, architecture, behavior | spec |
| Lasting choice with rationale | decision |
| Future direction, not yet accepted | proposal |
| Ordered work coordination | roadmap |
| Bounded execution steps | plan |
| Investigation with hypothesis | experiment |
| Performance measurement | eval |
| Repeatable procedure | checklist |
| External evidence or source synthesis | reference |

Every wiki page must include:

```markdown
- Document Class: [type]
- Status: [status]
- Date: [YYYY-MM-DD]
- Category: [label]
- Scope: [one sentence]
- Sources: [raw/ path]
- Related: [related wiki paths]
```

### Phase 3: Bookkeeping

1. Update `wiki/index.md` for every created or modified page.
2. Append to `wiki/log.md`:

```markdown
## [YYYY-MM-DD] ingest | <source filename>

Ingested <raw/path/to/source>.
<Brief summary of what was extracted and what changed.>
Pages created: [list]
Pages updated: [list]
Contradictions found: [list or "none"]
```

3. If QMD is available and configured, run `qmd embed -f`.

## Contradictions

Do not silently overwrite conflicting wiki content. If the new source
contradicts an existing page, show the conflict clearly and ask the user which
claim is correct. After resolution, update the relevant page and note the
supersession or correction.

## Source Types

| Source type | Processing |
| --- | --- |
| Markdown or text | Read directly |
| PDF | Extract readable text and cite the PDF path |
| Images | Extract visible text and diagram meaning |
| Transcripts | Extract decisions, action items, facts, and discard chatter |
| Code files | Extract architecture, API surfaces, and module boundaries |

## Final Report

Report the number of sources ingested, pages created, pages updated,
contradictions found and resolved, and any gaps that need more source material.
