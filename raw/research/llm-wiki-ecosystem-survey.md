# LLM Wiki Ecosystem Survey

Source: GitHub search across llm-wiki implementations
Retrieved: 2026-04-23

## Scope

Comprehensive survey of implementations of Karpathy's LLM Wiki pattern
(published 2026-04-03). Covers repos, tools, SaaS products, and
architectural extensions found on GitHub as of 2026-04-23.

## Tier 1: Substantial / High-Star Projects

### SamurAIGPT/llm-wiki-agent (~1,965 stars)

Multi-platform wiki agent. Drop sources into raw/, agent reads them,
extracts knowledge, builds persistent interlinked wiki. Detects
contradictions at ingest time. Works with Claude Code, Codex, OpenCode,
Gemini CLI. No API key needed (uses the host agent's LLM).

### AgriciDaniel/claude-obsidian (~1,480 stars)

Claude + Obsidian knowledge companion. Most visually polished
implementation with 10 specialized skills, 2 parallel agents, and a hot
cache system. `/autoresearch` command for autonomous deep research loops.

### nashsu/llm_wiki (Desktop App)

Cross-platform desktop application built with Tauri v2 (Rust backend +
React/TypeScript frontend). Three-column layout: Knowledge Tree + Chat +
Preview. Supports OpenAI, Anthropic, Google, Ollama, custom endpoints.
LanceDB for vector search. 4-signal knowledge graph with Louvain community
detection for automatic cluster discovery. Two-step chain-of-thought
ingest. Automated builds for macOS (ARM + Intel), Windows, Linux.

### nvk/llm-wiki (Parallel Multi-Agent Research)

Parallel multi-agent research engine. Deploys 5-10 specialized parallel
agents (academic, technical, contrarian, etc.) for thesis-driven
investigation. `--plan` flag decomposes topics into 3-5 independent
research paths dispatched as parallel agent groups. Thesis mode collects
evidence for and against a claim, returns verdict. Has its own website
(llm-wiki.net).

### skyllwt/OmegaWiki (Research Lifecycle)

Full research lifecycle platform: paper ingestion -> knowledge graph ->
gap detection -> idea generation -> experiment design -> paper writing ->
peer review response. 23 skills. Failed experiments become
"anti-repetition memory" preventing dead-end re-exploration.

## Tier 2: Solid Implementations with Distinct Features

### Pratiyush/llm-wiki

CLI-centric. Commands: sync (JSONL to markdown), build (compile static
HTML + AI exports), serve (local HTTP server), graph, watch,
export-obsidian, export-qmd, export-marp, eval, lint. Multi-adapter:
Claude Code, Codex CLI, Copilot, Cursor, Gemini.

### lucasastorian/llmwiki (Web App)

Open-source web app hosted at llmwiki.app. Upload documents, connect
Claude account via MCP, Claude writes the wiki. Next.js frontend, FastAPI
backend, Supabase (Postgres), MCP server.

### atomicmemory/llm-wiki-compiler (Two-Phase Compilation)

Two-phase compilation pipeline. Phase 1: extract all concepts from all
sources. Phase 2: generate pages. Eliminates order-dependence. SHA-256
hash-based change detection for incremental builds. MCP server.
Compounding queries (query --save writes answer as wiki page and rebuilds
index).

### iamsashank09/llm-wiki-kit

Drop PDFs, URLs, YouTube videos. Modular pip install with optional extras:
`pip install "llm-wiki-kit[pdf]"`, `[web]`, `[youtube]`, `[all]`.
MCP-compatible.

### MehmetGoekce/llm-wiki (L1/L2 Cache Architecture)

L1/L2 cache architecture. L1 = Claude Code Memory (~14 files, auto-loaded
every session: rules, gotchas, identity). L2 = wiki layer (~46 pages,
queried on demand). First implementation with Logseq support (outliner
format = independently addressable blocks). Credential leak detection in
lint.

### MauricioPerera/llm-wiki-kit

Three-layer retrieval: grep on indexes, BM25 on pages, embeddings on
pages. Git-native substrate. TypeScript strict, Node 20+, Deno 2,
Cloudflare Workers. Explicit page supersession model.
Platform-agnostic (Node, Deno, Workers).

### hellohejinyu/llm-wiki

CLI with ReAct agent for deep query synthesis. Agent dives into source
files when concept pages cite them. Multi-language output (answers in
language of question). Static + semantic lint passes.

### kytmanov/obsidian-llm-wiki-local

100% local with Ollama. gemma4:e4b for fast analysis, qwen2.5:14b for
heavy tasks. Interactive setup wizard. Web clipper integration for
ingesting web content.

### kenhuangus/llm-wiki (Automated Feed Monitoring)

Stateful knowledge compilation engine. Automated monitoring of arXiv and
CVE feeds as continuous ingest sources. Multi-agent namespacing (shared
team wiki with per-agent write isolation). LM Studio + Gemma 4 (fully
local). RDF/OWL federation for formal semantic interoperability.

### domleca/llm-wiki (Obsidian Plugin)

Obsidian plugin (not external CLI). Reads vault, extracts
people/ideas/connections, lets you chat with notes. Background
re-extraction triggered on note save. Multi-turn conversations saved and
resumable. Privacy-first (everything local by default).

## Tier 3: Agent Skills / Smaller Projects

### Astro-Han/karpathy-llm-wiki

Agent Skills-compatible. Claims production use with 94 articles and 99
sources maintained daily. Claude Code, Cursor, Codex.

### Ar9av/obsidian-wiki

Skill-based framework. `/wiki-update` distills project knowledge into
Obsidian vault. `/wiki-query` searches wiki and synthesizes answers with
citations. Any AI coding agent.

### toolboxmd/karpathy-wiki

Two skills: general-purpose wiki and project wiki (reads source code,
docs, git history). Claude Code.

### kfchou/wiki-skills

Claude Code plugin with companion viewer (wikiclaudia). Go-based viewer.

### itsnauman/wikiclaudia

Local Wikipedia-style HTML reader for LLM wikis. Single Go binary.
Read-only viewer, companion to wiki-skills.

### llmrix/llm-wiki-skill (Multimodal Ingest)

Uses Claude's multimodal capability for semantic comprehension of
diagrams, charts, screenshots (not just OCR). Claude Code.

### lewislulu/llm-wiki-skill

Experimental OpenClaw/Codex skill. Obsidian plugin for in-page commenting
(writes anchored feedback to audit/).

### rarce/git-wiki

Claude Code skill with QMD for hybrid search. Wiki lives as a separate
GitHub repo (private by default). Auto-commits and pushes.

### ussumant/llm-wiki-compiler (Code Wiki)

Compiles knowledge from codebases (not just markdown). Architecture, API
contracts, decision records, deployment configs. `/wiki-visualize`
launches interactive knowledge graph. Reduces context costs by ~90%.

### ekadetov/llm-wiki

Claude Code plugin. Obsidian vault structure with Dataview-compatible
frontmatter.

### kothari-nikunj/llm-wiki (Personal Encyclopedia)

"Wikipedia about you." Reads your writing, tweets, messages, bookmarks.
Password-protected Next.js web viewer. Expects and handles LLM factual
errors as part of workflow.

### Ss1024sS/LLM-wiki (Scaffolding)

Bootstrap script generates 30 files including frontmatter templates,
manifests, stale reporting scripts, CI workflows, configs for 4 AI
platforms.

### hsuanguo/llm-wiki

Agent skill + Python CLI (lwiki) for scaffolding wiki trees and tracking
raw drift. Multi-wiki independence (each has own AGENTS.md).

### louiswang524/llm-knowledge-base (Fine-Tuning Vision)

9-skill Claude Code system. Envisions using wiki as training data to
fine-tune models that "genuinely know" the knowledge base.

### eugeniughelbur/obsidian-second-brain

4 autonomous agents running on schedule. `/obsidian-challenge` searches
vault for past failures on same topic, pushes back with your own words.
`/obsidian-daily` pulls calendar events, overdue tasks.

### NicholasSpisak/second-brain

LLM-maintained personal knowledge base for Obsidian. Install wizard for
naming, location, domain, tooling setup. 4 skills.

## Ecosystem / Meta Projects

### tjiahen/awesome-llm-wiki

Curated awesome-list of tools, schemas, and implementations. References
graphthulhu, mcp-obsidian, DAIR.AI Academy guide.

### ScrapingArt/Karpathy-LLM-Wiki-Stack

Comprehensive build-ready reference document. "Share with Claude Code and
ask it to set up an LLM Wiki based on this blueprint."

### skridlevsky/graphthulhu (MCP Server)

MCP server giving AI full read-write access to Logseq or Obsidian
knowledge graphs. 37 tools. Decisions as first-class objects. Go, Datalog
queries.

### Apify: openclawai/second-brain-builder

Apify actor that automates the LLM Wiki flow. Paste URLs or raw notes,
click Start, get Obsidian-compatible knowledge base. No-code approach.

## Gists

### redmizt — "Beyond the Wiki"

18 architectural extensions for multi-agent production: identity, security,
concurrency, active learning, knowledge graphs, framework
self-improvement. Born from running the pattern with multiple Claude Code
Opus agents simultaneously.
