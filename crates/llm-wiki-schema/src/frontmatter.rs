use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    Claude,
    Codex,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationStyle {
    Slash,
    Namespace,
    Dispatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkillArgument {
    pub name: String,
    pub required: bool,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    pub runtimes: Vec<Runtime>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub arguments: Vec<SkillArgument>,
    pub invocation_style: Option<InvocationStyle>,
    #[serde(default)]
    pub dispatcher_for: Vec<String>,
}

pub fn schema_field_names() -> &'static [&'static str] {
    &[
        "name",
        "description",
        "runtimes",
        "operations",
        "arguments",
        "invocation_style",
        "dispatcher_for",
    ]
}
