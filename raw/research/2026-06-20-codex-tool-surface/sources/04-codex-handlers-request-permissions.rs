// Partial capture: the ToolExecutor impl that binds RequestPermissionsHandler
// to the OpenAI tool name "request_permissions". The handler body
// (handle_call) is included verbatim from the upstream file; it is the basis
// for confirming this is a real, registered Codex tool whose tool_use name
// header carries the literal "request_permissions" string.

use codex_protocol::request_permissions::RequestPermissionsArgs;
use codex_sandboxing::policy_transforms::normalize_additional_permissions;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::parse_arguments;
use crate::tools::handlers::parse_arguments_with_base_path;
use crate::tools::handlers::resolve_tool_environment;
use crate::tools::handlers::shell_spec::create_request_permissions_tool;
use crate::tools::handlers::shell_spec::request_permissions_tool_description;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde::Deserialize;

pub struct RequestPermissionsHandler;

#[derive(Deserialize)]
struct RequestPermissionsEnvironmentArgs {
    #[serde(default, rename = "environment_id", alias = "environmentId")]
    environment_id: Option<String>,
}

impl ToolExecutor<ToolInvocation> for RequestPermissionsHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("request_permissions")
    }

    fn spec(&self) -> ToolSpec {
        create_request_permissions_tool(request_permissions_tool_description())
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(self.handle_call(invocation))
    }
}
