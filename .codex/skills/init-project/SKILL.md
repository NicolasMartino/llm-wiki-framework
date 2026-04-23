---
name: init-project
description: Create or update a software project with the LLM Wiki project management framework. Use when Codex is asked to initialize a new project, scaffold a project knowledge base, add the framework to an existing codebase, update a project's framework files, or translate the framework bootstrap flow from Claude skills into Codex usage.
---

# Init Project

Create or update a project so an agent can manage project knowledge through
`raw/`, `wiki/`, and project-specific agent instructions.

## Invocation

Treat normal user requests as invocations. Support explicit `$init-project`
or `$knowledge` invocation.

Examples:

- "Initialize this repo with the LLM Wiki framework"
- "Set up `/path/to/project` as a self-managing wiki project"
- "Update this project to the latest framework template"
- "Use $init-project to set up /path/to/project"
- "Use $knowledge to init /path/to/project"

If no target path is provided, use the current working directory.

## Framework Source

This skill is maintained inside the framework repo at:

`/Users/nicolasmartino/Documents/local_llm_wiki/software_project_management`

Use that repo's `project_guidelines.template.md` as the template source. When
running from a symlinked skill, resolve the symlink target if needed; do not
assume `~/.codex/skills` is the framework repo.

## State File

Persist progress in `<target-path>/init.json`. Create it before asking the
first question and update it after every answer so interrupted setup can
resume.

```json
{
  "status": "in_progress | completed | failed",
  "current_step": 1,
  "created_at": "YYYY-MM-DD",
  "updated_at": "YYYY-MM-DD",
  "answers": {
    "project_name": null,
    "description": null,
    "project_type": null,
    "existing_or_new": null,
    "scale": null,
    "initial_sources": null
  },
  "profile": {
    "INCLUDE_ML_AI": null,
    "INCLUDE_QMD": null,
    "IS_EXISTING": null
  },
  "generated_files": []
}
```

## Mode Selection

1. Check whether `<target-path>/init.json` exists.
2. If `status` is `in_progress`, resume from `current_step`.
3. If `status` is `completed`, use update mode.
4. If there is no state file, check for `wiki/` or `project_guidelines.md`.
5. If framework files already exist, use update mode; otherwise use create mode.

## Create Mode

1. Create the target directory if needed.
2. Create `init.json` with `status: in_progress`, `current_step: 1`, and
   today's date.
3. Ask only missing questions, one at a time:
   - Project name
   - One-sentence project description
   - Project type: `web`, `api`, `cli`, `ml`, `data`, `lib`, or `other`
   - Whether this is a new project or an existing codebase
   - Expected documentation scale: `small`, `medium`, or `large`
   - Optional initial raw sources to ingest
4. After each answer, update `init.json` and increment `current_step`.
5. Compute profile flags:
   - `INCLUDE_ML_AI`: true for `ml` or `data`
   - `INCLUDE_QMD`: true for `medium` or `large`
   - `IS_EXISTING`: true when adding to an existing codebase
6. Read `project_guidelines.template.md` from the framework source repo.
7. Generate `project_guidelines.md` by replacing template variables and
   removing inactive conditional sections.
8. Generate `CLAUDE.md` with project-specific agent instructions. If the
   target project should support Codex directly, also generate `AGENTS.md`
   with the same operational rules adapted for Codex.
9. Scaffold:
   - `raw/`
   - `wiki/specs/`
   - `wiki/decisions/`
   - `wiki/proposals/`
   - `wiki/roadmaps/`
   - `wiki/plans/`
   - `wiki/checklists/`
   - `wiki/references/`
   - `wiki/archive/`
10. If `INCLUDE_ML_AI`, also create `wiki/experiments/`, `wiki/evals/`,
    `models/`, `data/`, `notebooks/`, and `evals/`.
11. For new projects, create `src/`, `tests/`, `scripts/`, and `infra/`.
    For existing codebases, leave existing source layout alone.
12. Create `wiki/index.md` with empty sections for every active document type.
13. Create `wiki/log.md` with a project bootstrap entry.
14. Initialize git only if the target is not already in a git repository.
15. Create or update `.gitignore` with `.wiki/`, `*.sqlite`, and `.cache/`.
16. If initial raw sources were provided, copy them into `raw/` and run an
    ingest operation on each source.
17. Set `init.json.status` to `completed`, update `updated_at`, and record
    generated files.

## Template Processing

Replace:

- `{{PROJECT_NAME}}` with the project name
- `{{PROJECT_DESCRIPTION}}` with the project description
- `{{DATE}}` with today's date

Conditional sections:

- If `INCLUDE_ML_AI` is false, remove content between
  `<!-- SECTION:ML_AI -->` and `<!-- END:ML_AI -->`, and remove lines with
  `<!-- CONDITIONAL:ML_AI -->`.
- If `INCLUDE_QMD` is false, remove content between
  `<!-- SECTION:QMD -->` and `<!-- END:QMD -->`.
- Remove remaining HTML comment markers and clean extra blank lines.

## Update Mode

1. Read the target project's `init.json` if present.
2. Read `project_guidelines.md`, `CLAUDE.md`, `AGENTS.md` if present, and
   `wiki/index.md`.
3. Read the latest `project_guidelines.template.md` from the framework repo.
4. Compare current files with the template and identify missing sections,
   outdated conventions, and structural drift.
5. Summarize proposed changes and ask before applying them.
6. Preserve project-specific customizations while applying approved updates.
7. Append the update to `wiki/log.md`.

## Completion Report

Report the generated or updated files, profile flags, folder structure, and
any initial sources ingested. If QMD was included, provide the setup commands:

```bash
npm install -g @tobilu/qmd
qmd collection add wiki/ --name wiki
qmd embed
```
