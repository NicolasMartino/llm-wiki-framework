<!-- CLAUDE -->
---
name: init-project
description: Create or update a software project with the LLM Wiki project management framework. Use when the user wants to start a new project, scaffold a project, or update an existing project's framework.
---

# /init-project

Create or update a software project with the LLM Wiki project management framework.

## Usage

```
/init-project [path]
/init-project [path] --update
```

If no path is provided, use the current working directory.

## State File: init.json

All progress is persisted in `<target-path>/init.json`. This file is written
before the first question and updated after every answer. If the skill is
interrupted and re-invoked, it resumes from where it left off.

### init.json schema

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

## Behavior

### Step 0: Check for existing state

1. Check if `<target-path>/init.json` exists.
2. If it exists and `status` is `"in_progress"`: **resume mode** — skip to
   the question indicated by `current_step`, pre-filled answers are already
   saved.
3. If it exists and `status` is `"completed"`: **update mode**.
4. If it does not exist: **create mode** — check if `wiki/` or
   `project_guidelines.md` exist at the target path. If they do, switch to
   update mode. Otherwise, proceed with create mode.

---

### CREATE MODE

#### Step 1: Initialize init.json

Create `init.json` at the target path with all fields set to null, `status`
set to `"in_progress"`, `current_step` set to `1`, and `created_at` set to
today's date.

If the target directory does not exist, create it first.

#### Step 2: Ask questions

Ask questions one at a time. After each answer, update `init.json` with the
answer value and increment `current_step`. This ensures resumability.

If resuming, skip questions that already have non-null answers in init.json.
Show the user what was already answered:
> Resuming project setup. Previous answers:
> - Project name: X
> - Description: Y
> Continuing from question N...

**Q1: Project name** → saves to `answers.project_name`
> What is the project name?

**Q2: Description** → saves to `answers.description`
> One-sentence description of what this project does.

**Q3: Project type** → saves to `answers.project_type`
> What type of project is this?
> - web — Web application (frontend, backend, or full-stack)
> - api — API service or backend
> - cli — Command-line tool
> - ml — ML/AI project (models, training, inference)
> - data — Data pipeline or analytics
> - lib — Library or package
> - other — Something else (describe it)

**Q4: Existing code** → saves to `answers.existing_or_new`
> Is this a new project or are you adding the framework to an existing codebase?
> - new — Starting from scratch
> - existing — Adding to existing code

**Q5: Scale expectation** → saves to `answers.scale`
> How much project documentation do you expect over the project's lifetime?
> - small — Under 50 wiki pages (index.md navigation only)
> - medium — 50-200 pages (index.md + QMD search)
> - large — 200+ pages (QMD-primary navigation)

**Q6: Initial raw sources** (optional) → saves to `answers.initial_sources`
> Do you have any initial documents to ingest? (PRDs, specs, research, meeting notes)
> Provide file paths or say "none".

#### Step 3: Determine project profile

Based on answers, compute and save to `profile` in init.json:

- `INCLUDE_ML_AI`: true if project type is `ml` or `data`
- `INCLUDE_QMD`: true if scale is `medium` or `large`
- `IS_EXISTING`: true if existing_or_new is `existing`

Update `current_step` to `7` (generation phase).

#### Step 4: Read the template

Resolve the framework root from this skill file location; do not use a
hardcoded absolute path. If this skill is reached through a global symlink,
resolve the symlink target first. From `<framework-root>/.claude/skills/init-project/SKILL.md`, the framework root is three directories up.

Read the file:

```text
<framework-root>/project_guidelines.template.md
```

#### Step 5: Generate project_guidelines.md

Generate `project_guidelines.md` in the target path by processing the template:

1. Replace `{{PROJECT_NAME}}` with `answers.project_name`
2. Replace `{{PROJECT_DESCRIPTION}}` with `answers.description`
3. Replace `{{DATE}}` with today's date
4. If `INCLUDE_ML_AI` is false: remove all content between `<!-- SECTION:ML_AI -->` and `<!-- END:ML_AI -->` markers, and remove lines with `<!-- CONDITIONAL:ML_AI -->`
5. If `INCLUDE_QMD` is false: remove all content between `<!-- SECTION:QMD -->` and `<!-- END:QMD -->` markers
6. Remove all remaining HTML comment markers (`<!-- ... -->`) from the output
7. Clean up any double blank lines left by removals

Add `"project_guidelines.md"` to `generated_files` in init.json.

#### Step 6: Generate CLAUDE.md

Create `CLAUDE.md` in the target path, tailored to the project:

```markdown
# CLAUDE.md - Project Schema

This is {{PROJECT_NAME}}: {{PROJECT_DESCRIPTION}}

## Agent Role

You own `wiki/`. You write, update, cross-link, and maintain all wiki content.
Humans curate `raw/` and make judgment calls. You handle the bookkeeping.

## How To Orient

1. Read `project_guidelines.md` for the documentation model and rules.
2. Read `wiki/index.md` for the catalog of all project knowledge.
3. Read specific wiki pages identified from the index.
4. Read `raw/` sources only when wiki content is insufficient.

Never browse the filesystem to find information. `wiki/index.md` is your
entry point.

## Operations

### Ingest

When new material appears in `raw/`:

1. Read the raw source fully.
2. Identify facts, entities, relationships, decisions.
3. Write or update wiki pages (use the correct document type).
4. Check for contradictions with existing wiki content.
5. Update `wiki/index.md`.
6. Append to `wiki/log.md`.

### Query

When answering questions:

1. Read `wiki/index.md` to find relevant pages.
2. Read those pages.
3. Synthesize an answer with citations.
4. If the answer is durable new knowledge, file it as a wiki page.

### Lint

Periodically or on request:

1. Scan for contradictions between pages.
2. Find stale statuses or outdated claims.
3. Identify orphan pages not linked from index.
4. Check for missing cross-references.
5. Fix issues directly.
6. Log all changes in `wiki/log.md`.

## Conventions

- Document types: spec, decision, proposal, roadmap, plan, checklist, reference{{ML_AI_TYPES}}.
- Use the type by role, not convenience. See `project_guidelines.md`.
- Every wiki page has a metadata block (Document Class, Status, Date,
  Category, Scope, Sources).
- Filenames: `[slug].type.md` or `[index]-[slug].type.md`.
- Archived documents go to `wiki/archive/`.
- `wiki/log.md` uses format: `## [YYYY-MM-DD] operation | subject`.
```

Where `{{ML_AI_TYPES}}` is `, experiment, eval` if `INCLUDE_ML_AI` is true, otherwise empty.

Add `"CLAUDE.md"` to `generated_files` in init.json.

#### Step 7: Scaffold folder structure

Create the directory structure:

```
raw/
wiki/
  specs/
  decisions/
  proposals/
  roadmaps/
  plans/
  checklists/
  references/
  archive/
```

If `INCLUDE_ML_AI`: also create `wiki/experiments/`, `wiki/evals/`, and
root-level `models/`, `data/`, `notebooks/`, `evals/`.

If `IS_EXISTING`: do NOT create `src/`, `tests/`, etc. — they already exist.
If new project: create `src/`, `tests/`, `scripts/`, `infra/`.

#### Step 8: Create wiki/index.md

```markdown
# Wiki Index

Project: {{PROJECT_NAME}}
Stage: Bootstrap
Updated: {{DATE}}

## Specs

(none yet)

## Decisions

(none yet)

## Roadmaps

(none yet)

## Proposals

(none yet)

## Plans

(none yet)

## Checklists

(none yet)

## References

(none yet)
```

Add Experiments and Evals sections if `INCLUDE_ML_AI`.

#### Step 9: Create wiki/log.md

```markdown
# Wiki Log

## [{{DATE}}] create | Project bootstrap

Project initialized with LLM Wiki framework.
Project type: {{PROJECT_TYPE}}
Pages created: wiki/index.md, wiki/log.md
```

#### Step 10: Initialize git (if not already a repo)

```bash
git init
```

Create a `.gitignore` if one doesn't exist, including:
```
.wiki/
*.sqlite
.cache/
```

#### Step 11: Ingest initial sources (if provided)

If `answers.initial_sources` is not null and not "none":

1. Copy or move them into `raw/`
2. Run a full ingest operation on each source
3. Update index.md and log.md accordingly

#### Step 12: Finalize

1. Set `status` to `"completed"` in init.json
2. Set `updated_at` to today's date
3. Record all generated files in `generated_files`

Tell the user what was created. List:
- Files generated
- Folder structure
- Profile applied (which sections included/excluded)
- Next steps: "Drop raw sources into `raw/` and ask the agent to ingest them"

If `INCLUDE_QMD`, also tell the user:
```
To enable QMD search (recommended for your expected scale):
  npm install -g @tobilu/qmd
  qmd collection add wiki/ --name wiki
  qmd embed
```

---

### UPDATE MODE

#### Step 1: Read current state

Read the target project's:
- `init.json` (if it exists, for profile info)
- `project_guidelines.md`
- `CLAUDE.md`
- `wiki/index.md`

#### Step 2: Read latest template

Resolve the framework root from this skill file and read `<framework-root>/project_guidelines.template.md`.

#### Step 3: Compare and identify gaps

Compare the project's current guidelines against the latest template.
Identify:
- Missing sections (new document types, new operations, new rules)
- Outdated conventions
- Structural differences

#### Step 4: Propose changes

Show the user a summary of what would change and why.
Ask for confirmation before applying.

#### Step 5: Apply updates

Update `project_guidelines.md` and `CLAUDE.md` with the approved changes.
Preserve any project-specific customizations.
Log the update in `wiki/log.md`.
Update `updated_at` in init.json.

<!-- END -->
<!-- CODEX -->
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

Resolve the framework root from this skill file location; do not use a
hardcoded absolute path. If this skill is reached through a global symlink,
resolve the symlink target first. From either runtime output path:

```text
<framework-root>/.claude/skills/init-project/SKILL.md
<framework-root>/.codex/skills/init-project/SKILL.md
```

the framework root is three directories up. Confirm the resolved root contains
`project_guidelines.template.md`, then use:

```text
<framework-root>/project_guidelines.template.md
```

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
6. Resolve the framework root from this skill file and read `<framework-root>/project_guidelines.template.md`.
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
3. Resolve the framework root from this skill file and read the latest `<framework-root>/project_guidelines.template.md`.
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

<!-- END -->
