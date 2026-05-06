pub mod body;
pub mod doc;
pub mod frontmatter;
pub mod projector;
pub mod validation;

pub use crate::body::SkillBody;
pub use crate::doc::{ParseError, SkillDoc, parse};
pub use crate::frontmatter::{
    InvocationStyle, Runtime, SkillArgument, SkillFrontmatter, schema_field_names,
};
pub use crate::projector::{
    ClaudeProjector, CodexProjector, ProjectError, Projector, RenderedSkill, TargetRuntime,
};
