use crate::SkillDoc;
use crate::projector::format::{description_for, render_frontmatter};
use crate::projector::idiom::{rewrite_invocation, supports_runtime};
use crate::projector::{ProjectError, Projector, RenderedSkill, TargetRuntime};

#[derive(Clone, Copy, Debug, Default)]
pub struct ClaudeProjector;

impl Projector for ClaudeProjector {
    fn project(&self, doc: &SkillDoc) -> Result<RenderedSkill, ProjectError> {
        let name = &doc.frontmatter.name;
        if !supports_runtime(&doc.frontmatter.runtimes, TargetRuntime::Claude) {
            return Err(ProjectError::UnsupportedRuntime {
                skill: name.clone(),
                runtime: TargetRuntime::Claude,
            });
        }

        let description = description_for(&doc.frontmatter.description, TargetRuntime::Claude);
        let mut skill_md = render_frontmatter(name, &description);
        skill_md.push_str(&format!("# /{name}\n\n"));
        skill_md.push_str("## Purpose\n\n");
        skill_md.push_str(doc.body.purpose.trim());
        skill_md.push_str("\n\n## Behavior\n\n");
        skill_md.push_str(doc.body.behavior.trim());
        skill_md.push_str("\n\n## Invocation\n\n");
        skill_md
            .push_str(rewrite_invocation(&doc.body.invocation, name, TargetRuntime::Claude).trim());
        if let Some(notes) = &doc.body.notes {
            skill_md.push_str("\n\n## Notes\n\n");
            skill_md.push_str(notes.trim());
        }
        skill_md.push('\n');

        Ok(RenderedSkill {
            skill_md,
            runtime_config: None,
        })
    }
}
