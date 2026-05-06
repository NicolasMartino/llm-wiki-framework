use crate::SkillDoc;
use crate::projector::idiom::{rewrite_invocation, supports_runtime};
use crate::projector::{
    ProjectError, Projector, RenderedSkill, TargetRuntime, description_for, render_frontmatter,
};

#[derive(Clone, Debug, Default)]
pub struct CodexProjector {
    runtime_config_template: Option<String>,
}

impl CodexProjector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime_config_template(template: impl Into<String>) -> Self {
        Self {
            runtime_config_template: Some(template.into()),
        }
    }
}

impl Projector for CodexProjector {
    fn project(&self, doc: &SkillDoc) -> Result<RenderedSkill, ProjectError> {
        let name = &doc.frontmatter.name;
        if !supports_runtime(&doc.frontmatter.runtimes, TargetRuntime::Codex) {
            return Err(ProjectError::UnsupportedRuntime {
                skill: name.clone(),
                runtime: TargetRuntime::Codex,
            });
        }

        let description = description_for(&doc.frontmatter.description, TargetRuntime::Codex);
        let mut skill_md = render_frontmatter(name, &description);
        skill_md.push_str(&format!("# {}\n\n", doc.body.title.trim()));
        skill_md.push_str("## Purpose\n\n");
        skill_md.push_str(doc.body.purpose.trim());
        skill_md.push_str("\n\n## Behavior\n\n");
        skill_md.push_str(doc.body.behavior.trim());
        skill_md.push_str("\n\n## Invocation\n\n");
        skill_md
            .push_str(rewrite_invocation(&doc.body.invocation, name, TargetRuntime::Codex).trim());
        if !doc.frontmatter.operations.is_empty() && name != "knowledge" {
            skill_md.push_str("\n\nDispatcher aliases:\n");
            for operation in &doc.frontmatter.operations {
                skill_md.push_str(&format!("- `$knowledge {operation}`\n"));
            }
        }
        if let Some(notes) = &doc.body.notes {
            skill_md.push_str("\n\n## Notes\n\n");
            skill_md.push_str(notes.trim());
        }
        skill_md.push('\n');

        let runtime_config = self
            .runtime_config_template
            .clone()
            .unwrap_or_else(|| "interface:\n  display_name: \"{skill_name}\"\n".to_string())
            .replace("{skill_name}", name);

        Ok(RenderedSkill {
            skill_md,
            runtime_config: Some(runtime_config),
        })
    }
}
