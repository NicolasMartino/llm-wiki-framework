---
name: knowledge-lint
description: Run a lint pass on an LLM Wiki project. Use when the user asks to lint the wiki, scan for contradictions, stale claims, orphan pages, missing cross-references, or fix wiki bookkeeping issues directly.
---

# /knowledge-lint

Run a lint pass over the current project's wiki and fix real bookkeeping
problems directly.

## Invocation

Support normal language and explicit invocation:

```text
/knowledge-lint
```

## Scope

Operate on one project at a time. Require `wiki/index.md` in the current
working directory. If it is missing, tell the user the project has not been
initialized with the LLM Wiki framework.

## Required Context

Before changing anything, read:

1. `wiki/index.md`
2. `project_guidelines.md` if present, otherwise `project_guidelines.template.md`
3. Relevant wiki pages implicated by the issues you find

Use the index as the entry point. Read more pages only when needed to confirm
or fix a specific issue.

## Checks

Look for:

1. Contradictions between wiki pages
2. Stale statuses or claims that no longer match related pages
3. Orphan pages not linked from `wiki/index.md`
4. Missing cross-references between closely related pages
5. Index entries that are missing, wrong, or stale

## Workflow

1. Read `wiki/index.md` and identify candidate pages to inspect.
2. Read the minimum set of pages needed to confirm each issue.
3. If an issue is clear and mechanical, fix it directly.
4. If pages make conflicting substantive claims and the correct answer is not
   documented anywhere, stop and ask the user to resolve the conflict.
5. Update `wiki/index.md` if the catalog needs correction.
6. Append the lint pass to `wiki/log.md`:

```markdown
## [YYYY-MM-DD] lint | wiki consistency pass

Scanned the wiki for contradictions, stale claims, orphan pages, and missing
cross-references.
Issues found: [list or "none"]
Pages updated: [list or "none"]
Outstanding questions: [list or "none"]
```

## Rules

1. Fix bookkeeping issues directly.
2. Do not silently choose between contradictory substantive claims when the
   source of truth is unclear.
3. Prefer narrow edits over broad rewrites.
4. Keep the index aligned with the actual wiki contents.
5. Report residual risk or unanswered conflicts clearly.

## Final Report

Report issues found, pages updated, unresolved contradictions or questions,
and whether the wiki now appears consistent.
