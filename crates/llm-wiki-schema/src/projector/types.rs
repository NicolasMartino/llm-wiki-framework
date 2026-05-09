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
    #[error("failed to render skill: {0}")]
    Render(String),
}
