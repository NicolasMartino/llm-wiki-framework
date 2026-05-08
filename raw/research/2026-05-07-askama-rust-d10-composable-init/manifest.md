# Research Manifest: Askama Rust for D10 Composable Init

- Date: 2026-05-07
- Research Question: How should the project use Askama in Rust to implement the D10 composable init plan safely and idiomatically?
- Goal: Prepare implementation by collecting primary Askama sources and translating them into concrete D10 design guidance.
- Source Modes: web, local context
- Selection Method: Auto-selected primary sources because the request asked for an in-depth preparation review.
- Local Context Reviewed: wiki/index.md, wiki/plans/composable-project-init.plan.md, wiki/decisions/composable-project-init.decision.md

## Source Inventory

1. `sources/001-askama-crate-latest.md` — docs.rs package page for Askama 0.16.0.
2. `sources/002-askama-introduction.md` — Askama stable introduction and quick start.
3. `sources/003-askama-template-derive.md` — Askama stable `Template` derive attributes.
4. `sources/004-askama-configuration.md` — Askama stable configuration, template dirs, whitespace, and escapers.
5. `sources/005-askama-template-syntax.md` — Askama syntax for escaping, loops, if, render-in-place, macros.
6. `sources/006-d10-local-context.md` — Local D10 implementation constraints from the wiki.

## Selection Rationale

The D10 plan depends on compile-time templates, Markdown output, stable file
locations, deterministic whitespace, and composed pack fragments. The selected
sources cover the Askama features that directly affect those requirements:
dependency/version choice, `Template` derive setup, template search paths,
escape behavior for `.md`, control-flow syntax, and composition options.

## Gaps

- Askama 0.16.0 docs.rs package metadata is current, but some generated docs
  URLs and search snippets still show 0.15.x or older example dependency
  versions. Prefer `askama = "0.16"` in implementation, then verify locally.
- This bundle does not research `inquire` or `toml`; it focuses on Askama as
  requested. D10's prompt and manifest work may merit separate source review if
  implementation hits API uncertainty.
- No source confirms D10-specific choices such as `AGENTS.md` vs `CLAUDE.md`;
  those come from local project decisions, captured in source 006.

## Ingest Readiness

Ready for ingest. Recommended document type: reference, with possible updates
to the D10 plan if implementation reveals new constraints.
