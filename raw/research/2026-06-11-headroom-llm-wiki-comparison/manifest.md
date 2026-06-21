# Headroom And LLM Wiki Comparison Research Manifest

- Research Date: 2026-06-11
- Mode: URL research plus temporary source archive inspection
- Question: Could Headroom be used in a similar way to this LLM Wiki framework, and how does Headroom work?
- Goal: Decide whether Headroom should replace, augment, or simply inform the LLM Wiki framework.
- Intake Note: The user supplied the research question, goal, and primary URL in the request, so no separate intake exchange was needed.

## Source Inventory

Primary external source:

- `https://github.com/chopratejas/headroom`

Saved sources:

1. `sources/01-headroom-readme.md` - repository README from the downloaded `main` archive.
2. `sources/02-headroom-llms.txt` - machine-readable docs index.
3. `sources/03-headroom-docs-architecture.mdx` - architecture docs.
4. `sources/04-headroom-docs-ccr.mdx` - reversible compression docs.
5. `sources/05-headroom-docs-mcp.mdx` - MCP tools docs.
6. `sources/06-headroom-docs-memory.mdx` - persistent memory docs.
7. `sources/07-headroom-docs-limitations.mdx` - limitations and safety gates.
8. `sources/08-headroom-docs-failure-learning.mdx` - `headroom learn` docs.
9. `sources/09-headroom-source-transform-pipeline.py` - transform pipeline implementation.
10. `sources/10-headroom-source-content-router.py` - content router implementation.
11. `sources/11-headroom-source-smart-crusher.py` - SmartCrusher wrapper implementation.
12. `sources/12-headroom-source-compression-store.py` - CCR compression store implementation.
13. `sources/13-headroom-source-mcp-server.py` - Headroom MCP server implementation.
14. `sources/14-headroom-source-proxy-server.py`
17. `sources/17-headroom-config-excerpt.py` - partial capture of `headroom/config.py` taken 2026-06-15 to evidence the literal `DEFAULT_EXCLUDE_TOOLS` and `DEFAULT_TOOL_PROFILES` shapes after the original bundle's coverage was found insufficient. Includes an explicit note on the upstream comment vs. literal inconsistency around Bash. - Headroom proxy server implementation.
15. `sources/15-headroom-source-learn-cli.py` - failure-learning CLI implementation.
16. `sources/16-headroom-pyproject.toml` - package metadata and dependency surface.

## Local Project Context Reviewed

- `templates/base/project_guidelines.md`
- `wiki/index.md`
- `wiki/specs/documentation-model.spec.md`
- `wiki/references/llm-wiki-pattern.reference.md`
- `wiki/references/three-phase-ingest-pipeline.reference.md`
- `wiki/decisions/three-layer-architecture.decision.md`
- `wiki/decisions/llm-wiki-binary-distribution.decision.md`
- `wiki/specs/wiki-query-skill.spec.md`
- `wiki/decisions/wiki-query-search-first.decision.md`
- `wiki/decisions/semantic-hybrid-search-mode.decision.md`

## Retrieval And Verification Notes

- The Headroom repository was opened through GitHub and downloaded as a temporary source archive under `/private/tmp`.
- No Headroom package install, proxy run, or test suite execution was performed.
- The repository README and `pyproject.toml` identify Headroom as `headroom-ai` version `0.24.0` in the inspected archive.
- GitHub displayed latest release `v0.24.0` dated 2026-06-09 at review time.
- Research is ready for ingest into a `reference` page and, if desired, a `proposal` or `decision` page.
