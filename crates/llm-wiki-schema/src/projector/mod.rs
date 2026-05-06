mod claude;
mod codex;
pub mod idiom;

pub use claude::ClaudeProjector;
pub use codex::CodexProjector;

use crate::SkillDoc;
use thiserror::Error;

pub trait Projector {
    fn project(&self, doc: &SkillDoc) -> Result<RenderedSkill, ProjectError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedSkill {
    pub skill_md: String,
    pub runtime_config: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetRuntime {
    Claude,
    Codex,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ProjectError {
    #[error("skill {skill} does not support {runtime:?}")]
    UnsupportedRuntime {
        skill: String,
        runtime: TargetRuntime,
    },
}

pub(crate) fn render_frontmatter(name: &str, description: &str) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n",
        yaml_scalar(name),
        yaml_scalar(description)
    )
}

pub(crate) fn yaml_scalar(value: &str) -> String {
    serde_yaml::to_string(value)
        .expect("serializing scalar cannot fail")
        .trim()
        .to_string()
}

pub(crate) fn description_for(description: &str, runtime: TargetRuntime) -> String {
    let (runtime_name, verb) = match runtime {
        TargetRuntime::Claude => ("Claude Code", "invoke"),
        TargetRuntime::Codex => ("Codex", "use"),
    };
    description
        .replace("{runtime}", runtime_name)
        .replace("{verb}", verb)
}
