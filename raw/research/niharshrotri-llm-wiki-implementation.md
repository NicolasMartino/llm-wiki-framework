# NiharShrotri/llm-wiki - Full Implementation

Source: https://github.com/NiharShrotri/llm-wiki
Retrieved: 2026-04-23

## Overview

Full implementation of Karpathy's LLM Wiki pattern. 100% local. Uses
Ollama + Qwen3-14B for reasoning, QMD for hybrid search, Obsidian for
graph visualization.

## Architecture

```
raw/ (immutable sources) → LLM Agent (Qwen3-14B) → wiki/ (compiled markdown)
                              ↓                          ↓
                         schema/AGENTS.md          Obsidian vault
                         (formatting rules)        (graph visualization)
```

## Technology Stack

| Layer | Component | Purpose |
|-------|-----------|---------|
| LLM | Ollama + Qwen3-14B (Q4_K_M, ~9.3GB) | Reasoning, 40K context, thinking mode |
| Search | QMD (BM25 + vector + rerank) | Local hybrid retrieval, SQLite-backed |
| Embeddings | EmbeddingGemma-300M (via QMD) | Compact, high-quality vectors |
| Reranker | Qwen3-Reranker-0.6B (via QMD) | Cross-encoder ranking |
| CLI | Typer + Rich | Progress bars, colored output |
| Parsers | pypdf, python-docx, beautifulsoup4 | PDF, DOCX, HTML, MD, TXT support |
| Vault | Obsidian | Graph view, backlinks, native wikilinks |

## Folder Structure

```
project-root/
├── raw/
├── wiki/
│   ├── sources/           # One page per ingested file
│   ├── entities/          # People, orgs, models extracted
│   ├── concepts/          # Thematic pages
│   ├── synthesis/         # User-saved answers
│   ├── index.md           # Auto-rebuilt
│   └── log.md             # Activity record
├── schema/
│   └── AGENTS.md          # Format conventions & merge rules
└── .wiki/                 # Internal state (git-ignored)
    ├── ingest_history.db  # SQLite tracking
    ├── search_index/      # QMD index
    └── config.yaml
```

## 3-Pass Ingest Pipeline

Pass 1 - Extraction (thinking mode enabled):
- Qwen3 reads source document
- Returns structured JSON: summary, key takeaways, named entities, concepts, tags
- Identifies what this document contributes

Pass 2 - Page Drafting (thinking mode off, streaming):
- One LLM call per entity/concept
- Decision: create new page OR merge into existing page
- Preserves prior content, appends to sources: frontmatter, updates dates
- Maintains provenance through source references

Pass 3 - Source Summary:
- Generates sources/<slug>.md listing every wiki page touched
- Complete audit trail of which sources influenced which pages

Post-ingest automation:
- Rebuilds index.md
- Appends to log.md
- Auto-updates QMD search index

## Query Pipeline

1. Hybrid search via QMD — BM25 + vector + LLM reranking (all local)
2. Top-K hydration — Load full content of top 5-8 ranked pages
3. Synthesis — Qwen3 writes cited markdown answer using [[wikilinks]]
4. Optional save-back — --save-as files answer as new synthesis/ page

Query scopes: Wiki (compiled pages only), Raw (original documents), Hybrid (both)

## CLI Commands

| Command | Function |
|---------|----------|
| wiki init [path] | Scaffold new project |
| wiki add <file> [-r] | Register sources for ingest |
| wiki ingest [source_id] | Run 3-pass LLM pipeline |
| wiki query "<q>" [--scope wiki|raw|hybrid] [--save-as slug] | Search + synthesize |
| wiki lint [--deep] [--fix] | Health checks + auto-repair |
| wiki reindex | Force QMD index rebuild |
| wiki serve [--port N] | Launch FastAPI web UI |

## Web UI (FastAPI)

Seven pages:
- Dashboard — stats and activity
- Sources — list, inspect, delete, re-ingest
- Ingest — drag-and-drop with live progress
- Jobs — persistent history
- Query — chat interface with streaming, scope toggle, save-as
- Lint — interactive report with one-click auto-fix
- Graph — D3 force-directed visualization

## Lint System

Detects:
- Broken wikilinks
- Orphan pages (zero inbound links)
- Malformed frontmatter
- Noise (overly long summaries, empty sections)
- Deep contradiction detection (optional, LLM-powered)

Most issues auto-fixable with --fix flag.

## Key Attributes

- 100% local — no cloud, no API keys
- All search on-device via QMD
- Streaming ingest with interactive entity confirmation
- Auto-reindex after ingest/lint
- Write-back: good query answers become synthesis pages
- Obsidian native with wikilinks and graph view
- v0.8.1, production-ready for personal use
