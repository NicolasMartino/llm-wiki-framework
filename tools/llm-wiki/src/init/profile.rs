use anyhow::{Result, bail};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectProfile {
    pub include_ml_ai: bool,
    pub include_qmd: bool,
    pub is_existing: bool,
}

impl ProjectProfile {
    pub fn resolve(project_type: &str, scale: &str, is_existing: bool) -> Result<Self> {
        let project_type = project_type.to_ascii_lowercase();
        let scale = scale.to_ascii_lowercase();
        if !["web", "api", "cli", "ml", "data", "lib", "other"].contains(&project_type.as_str()) {
            bail!("unsupported project type: {project_type}");
        }
        if !["small", "medium", "large"].contains(&scale.as_str()) {
            bail!("unsupported scale: {scale}");
        }
        Ok(Self {
            include_ml_ai: matches!(project_type.as_str(), "ml" | "data"),
            include_qmd: matches!(scale.as_str(), "medium" | "large"),
            is_existing,
        })
    }
}
