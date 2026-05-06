---
name: knowledge-init
description: Create or update a software project with the LLM Wiki project management framework. Use when {runtime} is asked to initialize a new project, scaffold a project knowledge base, add the framework to an existing codebase, or run the framework bootstrap flow.
runtimes: [claude, codex]
operations: [init]
arguments:
  - name: path
    required: false
    description: Target project path. Defaults to the current working directory.
invocation_style: namespace
---
# Init Project

## Purpose

Collect project setup answers, then delegate deterministic scaffolding to the
`{llm_wiki_binary}` binary.

## Behavior

1. Determine the target path; default to the current working directory.
2. Ask for six values: project name, one-sentence description, project type
   (`web`, `api`, `cli`, `ml`, `data`, `lib`, `other`), whether this is an
   existing codebase, documentation scale (`small`, `medium`, `large`), and
   optional initial source paths.
3. Validate that required answers are present and normalize project type and
   scale to the accepted flag values.
4. Run:
   `{llm_wiki_binary} init <path> --non-interactive --name <name> --description <description> --type <type> --scale <scale>`
   Add `--existing` when appropriate. Add one `--initial-sources <path>` flag
   per supplied source.
5. Report the command result. If sources were copied, ask whether to run
   `knowledge-ingest` now; only ingest when the user agrees.

## Invocation

Use normal language or an explicit skill invocation:

- `<knowledge-init> /path/to/project`
- `<knowledge-init>` to initialize the current directory

## Notes

The binary owns scaffolding, template processing, collision checks, and source
copying. The agent owns conversation, validation, and the optional ingest
handoff.
