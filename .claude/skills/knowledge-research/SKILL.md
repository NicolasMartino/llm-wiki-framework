---
name: knowledge-research
description: Gather source material into raw/research through a guided intake flow. Use when the user asks what to research, wants web or site research, provides local files or URLs to collect, or wants to prepare a research bundle for later ingest.
---

# /knowledge-research

Gather source material into the project's `raw/research/` directory so it can
be ingested later.

## Usage

```text
/knowledge-research <topic-or-source>
```

Examples:

```text
/knowledge-research ./notes
/knowledge-research https://example.com/spec
/knowledge-research docs.example.com auth tokens
/knowledge-research vector database benchmarks
```

## Behavior

### Step 1: Locate the wiki

Check for `wiki/index.md` in the current working directory.

If it does not exist, tell the user:
> No wiki found in the current directory. Run /init-project first.

### Step 2: Read existing wiki state

Before gathering sources, read:

1. `wiki/index.md` — current catalog of all pages
2. `project_guidelines.md` if present, otherwise
   `project_guidelines.template.md` — documentation model and conventions
   when needed
3. the 1-3 most relevant wiki pages for the topic when the request is topical

Use that context to avoid redundant collection and to sharpen search terms.

### Step 3: Clarify the intake request

If the request is broad or ambiguous, ask one short intake exchange to
determine:

- the research question
- the goal of the research
- any source or scope constraints
- whether to shortlist or auto-select sources

Then determine the source mode:

- `path`
- `url`
- `site`
- `web`

### Step 4: Create the research bundle

Create one bundle for the research run:

```text
raw/research/YYYY-MM-DD-topic-slug/
  manifest.md
  research-summary.md
  sources/
```

This bundle is the research handoff artifact. It preserves provenance and
keeps the run coherent for later ingest.

### Step 5: Gather and save sources

**Path mode**

1. Copy the given file or directory into `sources/` using numbered,
   descriptive names.
2. Record the original path and selection rationale in `manifest.md`.

**URL mode**

1. Fetch each explicit URL.
2. Save each fetched page as markdown in `sources/`.
3. Prefix each saved file with:

   ```markdown
   # Source Snapshot

   - Title: ...
   - URL: ...
   - Retrieved: YYYY-MM-DD
   - Method: url
   ```

4. Record each URL in `manifest.md`.

**Site mode**

1. Search only within the specified domain.
2. Build a compact shortlist of relevant pages.
3. Show the shortlist with title, URL, and one-line reason each.
4. Ask which results to save unless the user already gave a clear selection
   rule.
5. Save approved pages in `sources/` with source metadata.
6. Record the query, domain, and selected pages in `manifest.md`.

**Web mode**

1. Search the web for the topic.
2. Prefer primary sources when source quality matters.
3. Build a shortlist of candidate pages.
4. Show the shortlist with title, URL, source, and one-line reason each.
5. Ask which results to save unless the user already gave a clear selection
   rule.
6. Save approved pages in `sources/` with source metadata.
7. Record the query and selected pages in `manifest.md`.

### Step 6: Write bundle files

Write `manifest.md` with:

- research question
- research goal
- date
- source mode or modes used
- source inventory with original path or URL
- retrieval date when applicable
- selection rationale for each source
- notable gaps or exclusions

Write `research-summary.md` with:

- research question
- scope and constraints
- source set reviewed
- key findings across the saved sources
- important disagreements, caveats, or open questions
- whether the bundle appears ready for ingest

### Step 7: Report

Tell the user:

- what research mode was used
- how many candidate sources were reviewed when applicable
- which bundle path was created or updated
- which source files were saved
- that `manifest.md` and `research-summary.md` were written
- any notable gaps or ambiguous sources
- whether the material is ready for `/knowledge-ingest`

## Rules

1. Keep `raw/` as the source of truth. Do not write research findings straight
   into `wiki/`.
2. Preserve provenance for every saved source.
3. Save only sources the user approved or clearly delegated you to select.
4. Prefer official docs, standards, maintainers, repositories, and first-party
   announcements over commentary when source quality matters.
5. Do not ingest automatically unless the user explicitly asks for ingest as a
   follow-up step.
6. Produce one `research-summary.md` per research run by default.
