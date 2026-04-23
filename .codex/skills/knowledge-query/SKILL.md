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
