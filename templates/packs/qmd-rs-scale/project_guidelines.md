## qmd-rs Scale Pack Additions

After reading `wiki/index.md`, query workflows should search every project
query. When the host exposes the LLM Wiki MCP server, use the `llm_wiki_search`
MCP tool (and `llm_wiki_search_all` for cross-project retrieval); fall back to
shell `llm-wiki search --mode auto --format json` and `llm-wiki search-all` when
no MCP server is configured. Both are backed by qmd-rs stores managed by the
llm-wiki binary. Search is a discovery layer over `wiki/`; it does not replace
source citations, typed documents, or index maintenance.
