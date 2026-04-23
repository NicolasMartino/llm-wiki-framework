---
name: knowledge-ingest
description: Ingest raw source material into the project wiki. Use when the user adds new documents, drops files into raw/, asks to "ingest", "process", "add to the wiki", or provides new information that should be compiled into the knowledge base.
---

# /knowledge-ingest

Ingest raw source material into the project's LLM Wiki.

## Usage

```
/knowledge-ingest <file-or-directory>
/knowledge-ingest               (processes all uninigested files in raw/)
```

## Behavior

### Step 1: Locate the wiki

Check for `wiki/index.md` in the current working directory.

If it does not exist, tell the user:
> No wiki found in the current directory. Run /init-project first.

### Step 2: Identify sources to ingest

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
2. `project_guidelines.md` — document types and conventions

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
| URLs | Fetch with WebFetch, save content to raw/ as markdown, then ingest |
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
   information, not convenience. See project_guidelines.md.
6. **Update the index.** wiki/index.md must reflect every change.
7. **Log everything.** wiki/log.md must record every ingest.
