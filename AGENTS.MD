# AGENTS.md - Project Schema

This project defines and dogfoods a software project management framework
based on the LLM Wiki pattern. The framework is the product. The wiki
manages its own development.

## Agent Role

You own `wiki/`. You write, update, cross-link, and maintain all wiki content.
Humans curate `raw/` and make judgment calls. You handle the bookkeeping.

## How To Orient

1. Read `templates/base/project_guidelines.md` for the documentation model and rules.
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

- Document types: spec, decision, proposal, roadmap, plan, experiment,
  eval, checklist, reference.
- Use the type by role, not convenience. See `templates/base/project_guidelines.md`.
- Every wiki page has a metadata block (Document Class, Status, Date,
  Category, Scope, Sources).
- Filenames: `[slug].type.md` or `[index]-[slug].type.md`.
- Archived documents go to `wiki/archive/`.
- `wiki/log.md` uses format: `## [YYYY-MM-DD] operation | subject`.

## This Project Specifically

This project is self-referential. The framework is both the product and
the management system. When working here:

- Specs in `wiki/specs/` define the framework's own rules.
- Decisions in `wiki/decisions/` record why the framework works this way.
- The roadmap in `wiki/roadmaps/` coordinates the framework's deliverables.
- Every pain point is a bug in the product.
- The ultimate proof: a fresh agent in a fresh project, using only
  these specs, can bootstrap and maintain a self-managing wiki.
