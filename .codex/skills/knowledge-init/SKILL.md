---
name: knowledge-init
description: Create or update a software project with the LLM Wiki project management framework. Use when Codex is asked to initialize a new project, scaffold a project knowledge base, add the framework to an existing codebase, or run the framework bootstrap flow.
---

# Init Project

## Purpose

Collect project setup answers, then delegate deterministic scaffolding to the
`llm-wiki` binary.

## Behavior

1. Determine the target path; default to the current working directory.
2. Ask for six values: project name, one-sentence description, blueprint,
   optional pack overrides, whether this is an existing codebase, and optional
   initial source paths.
3. Validate that required answers are present. Accepted blueprints are
   `generic`, `web-product`, `library-sdk`, `ml-research`, `ops-infra`,
   `security`, `research`, and `custom`. Accepted packs are `api`,
   `frontend`, `library`, `ml`, `data`, `ops`, `ops-lite`, `security`,
   `research`, and `qmd-scale`.
4. Run:
   `llm-wiki init <path> --non-interactive --name <name> --description <description> --blueprint <blueprint>`
   Add one `--pack <pack>` flag per explicit pack override; omit `--pack` to
   use the blueprint defaults. Add `--existing` when appropriate. Add one
   `--initial-sources <path>` flag per supplied source.
5. Report the command result. If sources were copied, ask whether to run
   `knowledge-ingest` now; only ingest when the user agrees.

## Invocation

Use normal language or an explicit skill invocation:

- `$knowledge-init /path/to/project`
- `$knowledge-init` to initialize the current directory

Dispatcher aliases:
- `$knowledge init`


## Notes

The binary owns scaffolding, template processing, collision checks, and source
copying. The agent owns conversation, validation, and the optional ingest
handoff.
