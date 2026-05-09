---
name: wiki
description: Dispatch wiki operations for the LLM Wiki framework. Use when the user invokes the wiki namespace, asks for wiki init/query/ingest/research/lint, or wants one explicit command-like entry point for framework operations in Codex.
runtimes: [codex]
operations: [dispatch]
invocation_style: dispatch
dispatcher_for: [init, query, ingest, research, lint]
---
# Wiki

## Purpose

Provide one explicit command-like namespace for LLM Wiki framework operations
in Codex.

## Behavior

1. Support explicit wiki-namespace invocation and normal language.
2. If the user invokes the namespace without an operation, list the supported
   operations and ask which one they want.
3. Route `init` to the `wiki-init` skill and treat remaining text as the
   optional target path or setup request.
4. Route `query` to `wiki-query` and treat remaining text as the required
   project question.
5. Route `ingest` to `wiki-ingest` and treat remaining text as an
   optional explicit source path.
6. Route `research` to `wiki-research` and treat remaining text as the
   research request, source mode, or query.
7. Route `lint` to `wiki-lint`.
8. If the operation is unknown, report the valid options: `init`, `query`,
   `ingest`, `research`, and `lint`.

## Invocation

Use the dispatcher explicitly:

- `<wiki> init /path/to/project`
- `<wiki> query what is D5 on the roadmap?`
- `<wiki> ingest raw/meeting-notes.md`
- `<wiki> research docs.example.com auth tokens`
- `<wiki> lint the wiki`

## Notes

Direct skill names remain valid. The namespace exists to give the framework a
predictable explicit command surface without relying on unsupported product
slash commands.
