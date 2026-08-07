## qmd-rs Scale Pack

- Search with the `llm_wiki_search` MCP tool (and `llm_wiki_search_all` across
  projects) when `wiki/index.md` no longer surfaces the needed page. Only fall
  back to the `llm-wiki search "<query>"` shell command when the MCP server is
  not configured.
- Re-run `llm-wiki index --force` after ingest or lint when search results may be stale.
- Treat qmd-rs-backed search as navigation support; answers still cite wiki pages.
