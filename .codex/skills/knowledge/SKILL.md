---
name: knowledge
description: Dispatch knowledge operations for the LLM Wiki framework. Use when the user invokes `$knowledge`, asks for knowledge init/query/ingest/research/lint, or wants one explicit command-like entry point for framework operations in Codex.
---

# Knowledge

Use `$knowledge` as the explicit command-like namespace for framework
operations in Codex.

## Invocation

Support both explicit `$knowledge` invocation and normal language.

Examples:

- `Use $knowledge to init /path/to/project`
- `Use $knowledge to query what is D5 on the roadmap?`
- `Use $knowledge to ingest raw/meeting-notes.md`
- `Use $knowledge to research docs.example.com auth tokens`
- `Use $knowledge to lint the wiki`

If the user invokes bare `$knowledge`, list the supported operations and ask
which one they want.

## Routing

Parse the requested operation after `$knowledge` and route as follows:

- `init`: read `../init-project/SKILL.md` and follow it. Treat remaining text
  as the optional target path or setup request.
- `query`: read `../knowledge-query/SKILL.md` and follow it. Treat remaining
  text as the required question.
- `ingest`: read `../knowledge-ingest/SKILL.md` and follow it. Treat remaining
  text as an optional explicit source path.
- `research`: read `../knowledge-research/SKILL.md` and follow it. Treat
  remaining text as the research request, mode, or query.
- `lint`: read `../knowledge-lint/SKILL.md` and follow it.

If the subcommand is unknown, report the valid options: `init`, `query`,
`ingest`, `research`, `lint`.

## Notes

- Direct skill names such as `init-project`, `knowledge-query`, and
  `knowledge-ingest`, and `knowledge-lint` remain valid.
- Normal language requests remain valid.
- `$knowledge` exists to give the framework one predictable explicit command
  surface without relying on unsupported product slash commands.
