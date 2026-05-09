---
name: wiki-query
description: Query an LLM Wiki project knowledge base and answer with citations. Use when {runtime} is asked a question about the current project, specs, decisions, features, roadmap, plans, documented evidence, or when the user says to check the wiki, look up project knowledge, summarize what is known, find a decision, save a durable synthesis back into the wiki, or explicitly invokes the skill.
runtimes: [claude, codex]
operations: [query]
arguments:
  - name: question
    required: true
    description: Project question to answer from the wiki.
invocation_style: namespace
---
# Wiki Query

## Purpose

Answer project questions from the compiled wiki instead of rediscovering
knowledge from the filesystem.

## Behavior

1. Require `wiki/index.md` in the current working directory. If it is missing,
   tell the user that the project has not been initialized with the LLM Wiki
   framework.
2. Read `wiki/index.md` first. Treat it as the catalog of all project
   knowledge.
3. Identify relevant pages from the index. Prefer specs for current truth,
   decisions for rationale, proposals for unaccepted direction, roadmaps for
   delivery status, plans for tactical work, experiments/evals for measured
   findings, and references for external evidence.
4. Start with the three to five most relevant pages. Read more only when the
   answer is incomplete or cross-references point to important context.
5. If the index is large or the question is complex, check whether QMD is
   available with `which qmd`. If available and configured, use
   `qmd query "<question>" -c wiki` to supplement index navigation.
6. Synthesize a direct answer with citations to wiki page paths. Include raw
   source paths transitively when a cited wiki page depends on a key raw
   source.
7. Flag gaps explicitly when the wiki lacks enough information. Flag
   contradictions instead of silently choosing one source over another.
8. Do not speculate beyond documented wiki knowledge unless the user asks for
   advice; clearly label advice as inference.
9. If the answer creates durable new knowledge through synthesis across pages,
   offer to save it as a wiki page. When the user agrees, choose the document
   type by role, write the page with the required metadata block, update
   `wiki/index.md`, and append a `create` entry to `wiki/log.md`.

## Invocation

Use normal language or an explicit skill invocation:

- `<wiki-query> what is D8 on the roadmap?`
- `<wiki-query> summarize the binary distribution decision`

## Notes

Lead with the answer, then cite supporting pages. Keep the response clear
about what is documented, what is missing, and what is inferred.
