pub const INSTANCE: &str = env!("LLM_WIKI_COMPILED_INSTANCE");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpInstance {
    Production,
    Test,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpToolName {
    base: &'static str,
    production: &'static str,
    test: &'static str,
}

impl McpToolName {
    pub const fn base(self) -> &'static str {
        self.base
    }

    pub const fn for_instance(self, instance: McpInstance) -> &'static str {
        match instance {
            McpInstance::Production => self.production,
            McpInstance::Test => self.test,
        }
    }
}

const MCP_INSTANCES: &[McpInstance] = &[McpInstance::Production, McpInstance::Test];
const MCP_TOOL_NAMES: &[McpToolName] = &[
    McpToolName {
        base: "llm_wiki_read",
        production: "llm_wiki_read",
        test: "llm_wiki_read_test",
    },
    McpToolName {
        base: "llm_wiki_search",
        production: "llm_wiki_search",
        test: "llm_wiki_search_test",
    },
    McpToolName {
        base: "llm_wiki_search_all",
        production: "llm_wiki_search_all",
        test: "llm_wiki_search_all_test",
    },
    McpToolName {
        base: "llm_wiki_index",
        production: "llm_wiki_index",
        test: "llm_wiki_index_test",
    },
    McpToolName {
        base: "llm_wiki_register",
        production: "llm_wiki_register",
        test: "llm_wiki_register_test",
    },
    McpToolName {
        base: "llm_wiki_status",
        production: "llm_wiki_status",
        test: "llm_wiki_status_test",
    },
];
const PRODUCTION_SERVER_NAME_VARIANTS: &[&str] = &["llm-wiki", "llm_wiki"];
const TEST_SERVER_NAME_VARIANTS: &[&str] = &["llm-wiki-test", "llm_wiki_test"];

pub fn is_test() -> bool {
    INSTANCE == "test"
}
pub fn binary_stem() -> &'static str {
    // Single source of truth with `src/cli.rs`'s `command(name = ...)`, which
    // must use the `env!` literal because attribute position cannot call a fn.
    // build.rs emits this from the same `LLM_WIKI_INSTANCE` input as `INSTANCE`.
    env!("LLM_WIKI_COMPILED_BINARY_STEM")
}

pub fn binary_name() -> &'static str {
    if cfg!(windows) {
        match INSTANCE {
            "" => "llm-wiki.exe",
            "test" => "llm-wiki-test.exe",
            _ => unreachable!("build.rs rejects unsupported LLM_WIKI_INSTANCE values"),
        }
    } else {
        binary_stem()
    }
}

pub fn managed_home_dir_name() -> &'static str {
    match INSTANCE {
        "" => ".llm_wiki",
        "test" => ".llm_wiki-test",
        _ => unreachable!("build.rs rejects unsupported LLM_WIKI_INSTANCE values"),
    }
}

pub fn mcp_server_name() -> &'static str {
    active_mcp_instance().server_name()
}

pub fn mcp_tool_name(base: &'static str) -> &'static str {
    active_mcp_instance().tool_name(base)
}

pub fn active_mcp_instance() -> McpInstance {
    if is_test() {
        McpInstance::Test
    } else {
        McpInstance::Production
    }
}

pub fn all_mcp_instances() -> &'static [McpInstance] {
    MCP_INSTANCES
}

pub fn mcp_tool_names() -> &'static [McpToolName] {
    MCP_TOOL_NAMES
}

impl McpInstance {
    pub fn server_name(self) -> &'static str {
        match self {
            Self::Production => "llm-wiki",
            Self::Test => "llm-wiki-test",
        }
    }

    pub fn server_name_variants(self) -> &'static [&'static str] {
        match self {
            Self::Production => PRODUCTION_SERVER_NAME_VARIANTS,
            Self::Test => TEST_SERVER_NAME_VARIANTS,
        }
    }

    pub fn tool_name(self, base: &'static str) -> &'static str {
        MCP_TOOL_NAMES
            .iter()
            .copied()
            .find(|tool| tool.base() == base)
            .unwrap_or_else(|| panic!("unknown MCP tool base: {base}"))
            .for_instance(self)
    }
}

pub fn mcp_read_tool_name() -> &'static str {
    mcp_tool_name("llm_wiki_read")
}

pub fn mcp_search_tool_name() -> &'static str {
    mcp_tool_name("llm_wiki_search")
}

pub fn mcp_search_all_tool_name() -> &'static str {
    mcp_tool_name("llm_wiki_search_all")
}

pub fn mcp_index_tool_name() -> &'static str {
    mcp_tool_name("llm_wiki_index")
}

pub fn mcp_register_tool_name() -> &'static str {
    mcp_tool_name("llm_wiki_register")
}

pub fn mcp_status_tool_name() -> &'static str {
    mcp_tool_name("llm_wiki_status")
}

#[cfg(windows)]
pub fn windows_managed_dir_name() -> &'static str {
    match INSTANCE {
        "" => "llm_wiki",
        "test" => "llm_wiki-test",
        _ => unreachable!("build.rs rejects unsupported LLM_WIKI_INSTANCE values"),
    }
}

pub fn xdg_dir_name() -> &'static str {
    match INSTANCE {
        "" => "llm-wiki",
        "test" => "llm-wiki-test",
        _ => unreachable!("build.rs rejects unsupported LLM_WIKI_INSTANCE values"),
    }
}
#[cfg(test)]
mod tests {
    use super::{
        McpInstance, all_mcp_instances, mcp_index_tool_name, mcp_read_tool_name,
        mcp_register_tool_name, mcp_search_all_tool_name, mcp_search_tool_name, mcp_server_name,
        mcp_status_tool_name, mcp_tool_names,
    };

    #[test]
    fn mcp_names_follow_active_instance() {
        if super::is_test() {
            assert_eq!(mcp_server_name(), "llm-wiki-test");
            assert_eq!(mcp_read_tool_name(), "llm_wiki_read_test");
            assert_eq!(mcp_search_tool_name(), "llm_wiki_search_test");
            assert_eq!(mcp_search_all_tool_name(), "llm_wiki_search_all_test");
            assert_eq!(mcp_index_tool_name(), "llm_wiki_index_test");
            assert_eq!(mcp_register_tool_name(), "llm_wiki_register_test");
            assert_eq!(mcp_status_tool_name(), "llm_wiki_status_test");
        } else {
            assert_eq!(mcp_server_name(), "llm-wiki");
            assert_eq!(mcp_read_tool_name(), "llm_wiki_read");
            assert_eq!(mcp_search_tool_name(), "llm_wiki_search");
            assert_eq!(mcp_search_all_tool_name(), "llm_wiki_search_all");
            assert_eq!(mcp_index_tool_name(), "llm_wiki_index");
            assert_eq!(mcp_register_tool_name(), "llm_wiki_register");
            assert_eq!(mcp_status_tool_name(), "llm_wiki_status");
        }
    }

    #[test]
    fn mcp_name_inventory_covers_production_and_test_instances() {
        assert_eq!(
            all_mcp_instances(),
            &[McpInstance::Production, McpInstance::Test]
        );
        assert_eq!(mcp_tool_names().len(), 6);
        assert_eq!(
            McpInstance::Production.server_name_variants(),
            &["llm-wiki", "llm_wiki"]
        );
        assert_eq!(
            McpInstance::Test.server_name_variants(),
            &["llm-wiki-test", "llm_wiki_test"]
        );
        for tool in mcp_tool_names() {
            assert_eq!(tool.for_instance(McpInstance::Production), tool.base());
            assert_eq!(
                McpInstance::Production.tool_name(tool.base()),
                tool.for_instance(McpInstance::Production)
            );
            assert_eq!(
                McpInstance::Test.tool_name(tool.base()),
                tool.for_instance(McpInstance::Test)
            );
            assert!(tool.for_instance(McpInstance::Test).ends_with("_test"));
        }
    }

    #[test]
    #[should_panic(expected = "unknown MCP tool base")]
    fn unknown_mcp_tool_base_panics_instead_of_falling_back() {
        McpInstance::Test.tool_name("llm_wiki_new_tool");
    }
}
