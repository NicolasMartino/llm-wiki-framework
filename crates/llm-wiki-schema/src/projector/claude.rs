use crate::SkillDoc;
use crate::projector::format::{description_for, yaml_scalar};
use crate::projector::idiom::{rewrite_invocation, supports_runtime};
use crate::projector::{ProjectError, Projector, RenderedSkill, TargetRuntime};
use askama::Template;

#[derive(Clone, Copy, Debug, Default)]
pub struct ClaudeProjector;

#[derive(Template)]
#[template(path = "skills/claude.md", escape = "none")]
struct ClaudeSkill<'a> {
    doc: &'a SkillDoc,
}

impl ClaudeSkill<'_> {
    fn name(&self) -> &str {
        &self.doc.frontmatter.name
    }

    fn name_yaml(&self) -> String {
        yaml_scalar(&self.doc.frontmatter.name)
    }

    fn description_yaml(&self) -> String {
        yaml_scalar(&description_for(
            &self.doc.frontmatter.description,
            TargetRuntime::Claude,
        ))
    }

    fn purpose(&self) -> &str {
        self.doc.body.purpose.trim()
    }

    fn behavior(&self) -> &str {
        self.doc.body.behavior.trim()
    }

    fn invocation(&self) -> String {
        rewrite_invocation(
            &self.doc.body.invocation,
            &self.doc.frontmatter.name,
            TargetRuntime::Claude,
        )
        .trim()
        .to_string()
    }

    fn dispatcher_aliases(&self) -> String {
        String::new()
    }

    fn notes_section(&self) -> String {
        self.doc.body.notes.as_ref().map_or_else(
            || "\n".to_string(),
            |notes| format!("\n\n## Notes\n\n{}\n", notes.trim()),
        )
    }
}

impl Projector for ClaudeProjector {
    fn project(&self, doc: &SkillDoc) -> Result<RenderedSkill, ProjectError> {
        let name = &doc.frontmatter.name;
        if !supports_runtime(&doc.frontmatter.runtimes, TargetRuntime::Claude) {
            return Err(ProjectError::UnsupportedRuntime {
                skill: name.clone(),
                runtime: TargetRuntime::Claude,
            });
        }

        let skill_md = ClaudeSkill { doc }
            .render()
            .map_err(|err| ProjectError::Render(err.to_string()))?;

        Ok(RenderedSkill {
            skill_md,
            runtime_config: None,
        })
    }
}
