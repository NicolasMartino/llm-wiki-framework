<!-- CLAUDE -->
---
name: knowledge-query
description: Query the project wiki for knowledge. Use when the user asks a question about the project, its specs, decisions, features, roadmap, or any documented knowledge. Also use when the user says "check the wiki", "what do we know about", or "look up".
---

# /knowledge-query

Query the project's LLM Wiki knowledge base.

## Usage

```
/knowledge-query <question>
```

## Behavior

### Step 1: Locate the wiki

Check for `wiki/index.md` in the current working directory.

If it does not exist, tell the user:
> No wiki found in the current directory. This skill requires a project
> set up with the LLM Wiki framework (wiki/index.md must exist).

### Step 2: Read the index

Read `wiki/index.md` to get the full catalog of wiki pages.

### Step 3: Identify relevant pages

From the index, identify which pages are likely relevant to the question.
Consider:

- Document type relevance (specs for "how does X work?", decisions for
  "why did we choose X?", proposals for "what's planned for X?",
  roadmaps for "what's the status of X?", experiments for "what did we
  learn about X?")
- Title and summary keyword match
- Cross-references mentioned in page summaries

Select the most relevant pages. Start with 3-5 pages. If the answer seems
incomplete, read additional pages.

### Step 4: Check for QMD

If the question is complex or the index has many entries, check if QMD
is available:

```bash
which qmd
```

If QMD is available and there is a QMD collection for the wiki, use it
to supplement index-based navigation:

```bash
qmd query "<question>" -c wiki
```

Use QMD results to identify pages the index scan may have missed.

### Step 5: Read relevant pages

Read the full content of each relevant page identified in steps 3 and 4.

### Step 6: Synthesize the answer

Write a clear, direct answer to the question. Requirements:

1. **Cite wiki pages** — reference each claim with the wiki page path,
   e.g., "According to [wiki/specs/auth.spec.md], the service uses JWT..."
2. **Cite raw sources transitively** — if a wiki page cites a raw source
   that is key to the answer, include it: "...based on the original PRD
   (raw/specs/auth-prd.md)"
3. **Flag gaps** — if the wiki does not have enough information to fully
   answer the question, say so explicitly. Suggest what raw source might
   need to be ingested.
4. **Flag contradictions** — if two wiki pages disagree, surface the
   contradiction rather than picking one silently.
5. **Stay within the wiki** — do not speculate beyond what the wiki
   contains. The point of this skill is to surface documented knowledge,
   not to generate new reasoning.

### Step 7: Offer to save

If the synthesized answer produces durable new knowledge (not just a
lookup but a genuine synthesis across multiple pages), offer to save it:

> This answer synthesizes information from multiple wiki pages. Want me
> to save it as a wiki page? (It would go in wiki/references/ or
> wiki/specs/ depending on its nature)

If the user says yes:

1. Determine the correct document type (reference for synthesis of
   external evidence, spec if it validates current truth, decision if
   it clarifies a choice)
2. Write the page with proper metadata (Document Class, Status, Date,
   Category, Scope, Sources listing all wiki pages that contributed)
3. Update `wiki/index.md` to include the new page
4. Append to `wiki/log.md`:
   ```
   ## [YYYY-MM-DD] create | Query answer: <question summary>

   Synthesized answer from N wiki pages, saved as new page.
   Pages created: wiki/<type>/<slug>.md
   Source pages: [list of pages that contributed to the answer]
   ```

<!-- END -->
<!-- CODEX -->
---
name: knowledge-query
description: Query an LLM Wiki project knowledge base and answer with citations. Use when Codex is asked a question about the current project, specs, decisions, features, roadmap, plans, documented evidence, or when the user says to check the wiki, look up project knowledge, summarize what is known, find a decision, save a durable synthesis back into the wiki, or explicitly invokes `$knowledge-query` or `$knowledge`.
---

# Knowledge Query

Answer project questions from the compiled wiki instead of rediscovering
knowledge from the filesystem.

## Scope

Operate on one project at a time. The project must contain `wiki/index.md` in
the current working directory. If it does not, tell the user that the project
has not been initialized with the LLM Wiki framework.

## Invocation

Support normal requests and explicit invocation:

- `Use $knowledge-query to answer: what is D5 on the roadmap?`
- `Use $knowledge to query what is D5 on the roadmap?`

## Workflow

1. Read `wiki/index.md` first. Treat it as the catalog of all project
   knowledge.
2. Identify relevant pages from the index. Prefer:
   - Specs for current behavior, architecture, interfaces, and validated truth
   - Decisions for why a durable choice was made
   - Proposals for unaccepted future direction
   - Roadmaps for delivery order and status
   - Plans for tactical execution
   - Experiments and evals for measured findings
   - References for external evidence and source notes
3. Start with the 3-5 most relevant pages. Read more only when the answer is
   incomplete or cross-references point to important context.
4. If the index is large or the question is complex, check whether QMD is
   available with `which qmd`. If available and configured, use
   `qmd query "<question>" -c wiki` to supplement index navigation.
5. Synthesize a direct answer.
6. Cite wiki pages for claims using paths such as
   `wiki/specs/documentation-model.spec.md`.
7. If a cited wiki page depends on an important raw source, include the raw
   source path transitively.
8. Flag gaps explicitly when the wiki lacks enough information.
9. Flag contradictions instead of silently choosing one source over another.
10. Do not speculate beyond documented wiki knowledge unless the user asks for
    advice; clearly label advice as inference.

## Save-Back

If the answer creates durable new knowledge through synthesis across pages,
offer to save it as a wiki page.

When the user agrees:

1. Choose the document type by role:
   - `reference` for synthesis of source evidence
   - `spec` for validated current truth
   - `decision` for a durable choice and rationale
   - `proposal` for unaccepted direction
   - `plan` for tactical execution
2. Write a page with the required metadata block:
   - Document Class
   - Status
   - Date
   - Category
   - Scope
   - Sources
   - Related, when useful
3. Update `wiki/index.md`.
4. Append to `wiki/log.md` using:

```markdown
## [YYYY-MM-DD] create | Query answer: <question summary>

Synthesized answer from N wiki pages, saved as new page.
Pages created: wiki/<type>/<slug>.md
Source pages: [list of pages that contributed to the answer]
```

## Answer Shape

Lead with the answer, then cite supporting pages. Keep the response clear about
what is documented, what is missing, and what is inferred.

<!-- END -->
