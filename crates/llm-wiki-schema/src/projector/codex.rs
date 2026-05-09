use crate::SkillDoc;
use crate::projector::format::{description_for, yaml_double_quoted, yaml_scalar};
use crate::projector::idiom::{rewrite_invocation, supports_runtime};
use crate::projector::{ProjectError, Projector, RenderedSkill, TargetRuntime};
use askama::Template;
use serde::Deserialize;

#[derive(Clone, Debug, Default)]
pub struct CodexProjector {
    runtime_config: Option<CodexRuntimeConfig>,
}

impl CodexProjector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime_config(runtime_config: CodexRuntimeConfig) -> Self {
        Self {
            runtime_config: Some(runtime_config),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CodexRuntimeConfig {
    pub interface: CodexInterfaceConfig,
}

impl CodexRuntimeConfig {
    pub fn from_yaml(input: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(input)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CodexInterfaceConfig {
    pub display_name: String,
    pub short_description: String,
    pub default_prompt: String,
}

#[derive(Template)]
#[template(path = "skills/codex.md", escape = "none")]
struct CodexSkill<'a> {
    doc: &'a SkillDoc,
}

impl CodexSkill<'_> {
    fn name_yaml(&self) -> String {
        yaml_scalar(&self.doc.frontmatter.name)
    }

    fn description_yaml(&self) -> String {
        yaml_scalar(&description_for(
            &self.doc.frontmatter.description,
            TargetRuntime::Codex,
        ))
    }

    fn title(&self) -> &str {
        self.doc.body.title.trim()
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
            TargetRuntime::Codex,
        )
        .trim()
        .to_string()
    }

    fn dispatcher_aliases(&self) -> String {
        if self.doc.frontmatter.operations.is_empty() || self.doc.frontmatter.name == "wiki" {
            return String::new();
        }

        let mut aliases = String::from("\n\nDispatcher aliases:\n");
        for operation in &self.doc.frontmatter.operations {
            aliases.push_str(&format!("- `$wiki {operation}`\n"));
        }
        aliases
    }

    fn notes_section(&self) -> String {
        self.doc.body.notes.as_ref().map_or_else(
            || "\n".to_string(),
            |notes| format!("\n\n## Notes\n\n{}\n", notes.trim()),
        )
    }
}

#[derive(Template)]
#[template(path = "skills/codex_runtime_config.yaml", escape = "none")]
struct CodexRuntimeConfigTemplate<'a> {
    config: &'a CodexRuntimeConfig,
}

impl CodexRuntimeConfigTemplate<'_> {
    fn display_name_yaml(&self) -> String {
        yaml_double_quoted(&self.config.interface.display_name)
    }

    fn short_description_yaml(&self) -> String {
        yaml_double_quoted(&self.config.interface.short_description)
    }

    fn default_prompt_yaml(&self) -> String {
        yaml_double_quoted(&self.config.interface.default_prompt)
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

        let skill_md = CodexSkill { doc }
            .render()
            .map_err(|err| ProjectError::Render(err.to_string()))?;
        let runtime_config = self
            .runtime_config
            .clone()
            .unwrap_or_else(|| default_runtime_config(doc));
        let mut runtime_config = CodexRuntimeConfigTemplate {
            config: &runtime_config,
        }
        .render()
        .map_err(|err| ProjectError::Render(err.to_string()))?;
        if !runtime_config.ends_with('\n') {
            runtime_config.push('\n');
        }

        Ok(RenderedSkill {
            skill_md,
            runtime_config: Some(runtime_config),
        })
    }
}

fn default_runtime_config(doc: &SkillDoc) -> CodexRuntimeConfig {
    CodexRuntimeConfig {
        interface: CodexInterfaceConfig {
            display_name: doc.body.title.trim().to_string(),
            short_description: description_for(&doc.frontmatter.description, TargetRuntime::Codex),
            default_prompt: format!("Use ${} in Codex.", doc.frontmatter.name),
        },
    }
}
