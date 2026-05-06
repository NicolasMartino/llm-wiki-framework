use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Paths {
    pub home: PathBuf,
}

impl Paths {
    pub fn from_env() -> Result<Self> {
        let home = env::var_os("HOME").context("HOME is not set")?;
        Ok(Self {
            home: PathBuf::from(home),
        })
    }

    pub fn claude_skill(&self, skill: &str) -> PathBuf {
        self.home
            .join(".claude/skills")
            .join(skill)
            .join("SKILL.md")
    }

    pub fn codex_skill(&self, skill: &str) -> PathBuf {
        self.home.join(".codex/skills").join(skill).join("SKILL.md")
    }

    pub fn codex_config(&self, skill: &str) -> PathBuf {
        self.home
            .join(".codex/skills")
            .join(skill)
            .join("agents/openai.yaml")
    }

    pub fn manifest(&self) -> PathBuf {
        self.home.join(".local/share/llm-wiki/manifest.json")
    }
}
