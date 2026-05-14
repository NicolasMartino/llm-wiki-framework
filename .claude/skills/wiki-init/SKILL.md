---
name: wiki-init
description: Create or update a software project with the LLM Wiki project management framework. Use when Claude Code is asked to initialize a new project, scaffold a project knowledge base, add the framework to an existing codebase, or run the framework bootstrap flow.
---

# /wiki-init

## Purpose

Collect project setup answers, then delegate deterministic scaffolding to the
`llm-wiki` binary.

## Behavior

1. Determine the target path; default to the current working directory.
2. Ask for five values: project name, one-sentence description, blueprint,
   optional pack overrides, and optional initial source paths. If the target
   already has `.llm_wiki/init.toml`, use the recorded project values as the
   default answers and leave any newly introduced questions empty/defaulted.
   If the user changes blueprint during a rerun, use the new blueprint's pack
   defaults rather than the previous pack set.
3. Validate that required answers are present. Accepted blueprints are
   `generic`, `web-product`, `library-sdk`, `cli-tool`, `ml-research`,
   `ops-infra`, `security`, `research`, and `custom`. Accepted packs are
   `api`, `frontend`, `library`, `ml`, `data`, `ops`, `ops-lite`, `security`,
   `research`, `qmd-rs-scale`, and `code`.
4. Run:
   `llm-wiki init <path> --non-interactive --name <name> --description <description> --blueprint <blueprint>`
   Add one `--pack <pack>` flag per explicit pack override; omit `--pack` to
   use the blueprint defaults. Add one `--initial-sources <path>` flag per
   supplied source.
5. Report the command result. If sources were copied, ask whether to run
   `wiki-ingest` now; only ingest when the user agrees.

## Invocation

Use normal language or an explicit skill invocation:

- `/wiki-init /path/to/project`
- `/wiki-init` to initialize the current directory

## Notes

The binary owns scaffolding, template processing, collision checks, and source
copying. The agent owns conversation, validation, and the optional ingest
handoff.
