<!-- CLAUDE -->
---
name: knowledge-ingest
description: Ingest raw source material into the project wiki. Use when the user adds new documents, drops files into raw/, asks to "ingest", "process", "add to the wiki", or provides new information that should be compiled into the knowledge base.
---

# /knowledge-ingest

Ingest raw source material into the project's LLM Wiki.

## Usage

```
/knowledge-ingest <file-or-directory>
/knowledge-ingest               (processes all un-ingested files in raw/)
```

## Behavior

### Step 1: Locate the wiki

Check for `wiki/index.md` in the current working directory.

If it does not exist, tell the user:
> No wiki found in the current directory. Run /init-project first.

### Step 2: Identify sources to ingest

If the user provides URLs or asks to search the web or a site, use
`knowledge-research` first so the fetched material is saved into `raw/`
before ingest. Do not fetch web sources directly from this ingest skill.

**If a path argument is provided:**

1. If the path is a file outside `raw/`: copy it into `raw/` under an
   appropriate subdirectory (by source type or date), then ingest it.
2. If the path is a file inside `raw/`: ingest it directly.
3. If the path is a directory: identify all files in it. Copy any that
   are outside `raw/` into `raw/`, then ingest each.

**If no argument is provided:**

1. Read `wiki/log.md` to determine which raw files have already been
   ingested (look for `ingest` entries that name raw/ paths).
2. Scan `raw/` for files not mentioned in any log entry.
3. If no new files found, tell the user: "All raw sources have been
   ingested. Nothing new to process."
4. Otherwise, list the unprocessed files and ask: "Found N new source(s)
   to ingest. Proceed with all, or select specific files?"

### Step 3: Read existing wiki state

Before ingesting, read:

1. `wiki/index.md` — current catalog of all pages
2. `project_guidelines.md` if present, otherwise
   `project_guidelines.template.md` — document types and conventions

This gives context for what already exists, so the ingest can update
existing pages rather than creating duplicates.

### Step 4: Process each source (per file)

For each raw source file, run a three-phase ingest:

**Phase 1: Extraction**

Read the source fully. Identify and extract:

- Key facts and claims
- Named entities (people, services, APIs, modules, libraries, concepts)
- Relationships between entities (depends-on, uses, contradicts, replaces)
- Decisions made or implied
- Open questions or uncertainties
- Status information (what is done, in progress, planned)

Produce a structured summary of what this source contributes.

**Phase 2: Page drafting**

For each significant finding from phase 1, determine the correct action:

1. **Does a wiki page already cover this entity or topic?**
   - Yes → read that page, merge the new information in, update the
     Sources field to include the new raw source, update the Date.
   - No → create a new page with the correct document type:

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
   | External evidence, synthesis | reference |

2. **Check for contradictions** with existing wiki content. If the new
   source contradicts an existing page:
   - Do NOT silently overwrite.
   - Flag the contradiction to the user:
     > Contradiction found: [new source] says X, but
     > [existing page] says Y. Which is correct?
   - Update based on user's answer. If the new information supersedes
     the old, update the existing page and note the supersession.

3. Write each page with proper metadata:
   ```
   - Document Class: [type]
   - Status: [status]
   - Date: [today]
   - Category: [label]
   - Scope: [one sentence]
   - Sources: [raw/ path of the source being ingested]
   - Related: [links to related wiki pages]
   ```

**Phase 3: Bookkeeping**

1. Update `wiki/index.md`:
   - Add entries for any new pages created
   - Update summaries for any pages that were modified
   - Verify all links are correct

2. Append to `wiki/log.md`:
   ```
   ## [YYYY-MM-DD] ingest | <source filename>

   Ingested <raw/path/to/source>.
   <Brief summary of what was extracted and what changed.>
   Pages created: [list]
   Pages updated: [list]
   Contradictions found: [list or "none"]
   ```

3. If QMD is available, trigger reindex:
   ```bash
   qmd embed -f
   ```

### Step 5: Report

After all sources are processed, tell the user:

- How many sources were ingested
- Pages created (with paths)
- Pages updated (with what changed)
- Contradictions found and resolved
- Any gaps identified (topics mentioned in sources but not yet covered)

## Handling Different Source Types

| Source type | How to process |
| --- | --- |
| Markdown (.md) | Read directly |
| Text (.txt) | Read directly |
| PDF (.pdf) | Read with the Read tool (supports PDF) |
| Images (.png, .jpg) | Read with the Read tool (multimodal), extract visible text and diagrams |
| URLs, sites, web search | Route to `knowledge-research` first so source snapshots are saved under `raw/` |
| Conversation transcripts | Extract decisions, action items, and facts; discard chatter |
| Code files | Extract architecture patterns, API surfaces, module boundaries |

## Rules

1. **One source at a time.** Process each raw file completely before
   moving to the next. This ensures cross-references are built correctly.
2. **Merge, don't duplicate.** Always check if a wiki page already
   covers the topic before creating a new one.
3. **Preserve provenance.** Every wiki page must cite which raw sources
   informed it.
4. **Flag, don't hide contradictions.** Never silently overwrite
   existing wiki content that disagrees with new sources.
5. **Use the correct document type.** Classify by the role of the
   information, not convenience. See `project_guidelines.md` or `project_guidelines.template.md`.
6. **Update the index.** wiki/index.md must reflect every change.
7. **Log everything.** wiki/log.md must record every ingest.

<!-- END -->
<!-- CODEX -->
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

<!-- END -->
