// Partial capture: the submodule declarations and re-exports at the top of
// codex-rs/core/src/tools/handlers/mod.rs. The rest of the file (helper
// functions and tests) is omitted; the load-bearing evidence is the list of
// declared handler submodules and the bindings that expose them as the
// `ToolName::plain(...)` strings the OpenAI tool envelope carries.
//
// The omitted bottom portion of the upstream file contains helper functions
// (parse_arguments, normalize_and_validate_additional_permissions, etc.) and
// the #[cfg(test)] block. None of those declare additional tool names.

pub(crate) mod agent_jobs;
pub(crate) mod agent_jobs_spec;
pub(crate) mod apply_patch;
pub(crate) mod apply_patch_spec;
mod current_time;
mod dynamic;
pub(crate) mod extension_tools;
mod get_context_remaining;
pub(crate) mod get_context_remaining_spec;
mod list_available_plugins_to_install;
pub(crate) mod list_available_plugins_to_install_spec;
mod mcp;
mod mcp_resource;
pub(crate) mod mcp_resource_spec;
pub(crate) mod multi_agents;
pub(crate) mod multi_agents_common;
pub(crate) mod multi_agents_spec;
pub(crate) mod multi_agents_v2;
mod new_context_window;
pub(crate) mod new_context_window_spec;
mod plan;
pub(crate) mod plan_spec;
mod request_permissions;
mod request_plugin_install;
pub(crate) mod request_plugin_install_spec;
mod request_user_input;
pub(crate) mod request_user_input_spec;
mod shell;
pub(crate) mod shell_spec;
mod sleep;
mod test_sync;
pub(crate) mod test_sync_spec;
mod tool_search;
pub(crate) mod tool_search_spec;
pub(crate) mod unified_exec;
mod view_image;
pub(crate) mod view_image_spec;

// Handler re-exports (each binds a handler implementation to its tool spec):
pub use apply_patch::ApplyPatchHandler;
pub use current_time::CurrentTimeHandler;
pub use dynamic::DynamicToolHandler;
pub use get_context_remaining::GetContextRemainingHandler;
pub use list_available_plugins_to_install::ListAvailablePluginsToInstallHandler;
pub use mcp::McpHandler;
pub use mcp_resource::ListMcpResourceTemplatesHandler;
pub use mcp_resource::ListMcpResourcesHandler;
pub use mcp_resource::ReadMcpResourceHandler;
pub use new_context_window::NewContextWindowHandler;
pub use plan::PlanHandler;
pub use request_permissions::RequestPermissionsHandler;
pub use request_plugin_install::RequestPluginInstallHandler;
pub use request_user_input::RequestUserInputHandler;
pub use shell::ShellCommandHandler;
pub(crate) use shell::ShellCommandHandlerOptions;
pub use sleep::SleepHandler;
pub use test_sync::TestSyncHandler;
pub(crate) use tool_search::ToolSearchHandlerCache;
pub use unified_exec::ExecCommandHandler;
pub(crate) use unified_exec::ExecCommandHandlerOptions;
pub use unified_exec::WriteStdinHandler;
pub use view_image::ViewImageHandler;
