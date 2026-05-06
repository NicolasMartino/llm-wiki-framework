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
        self.managed_home().join("manifest.json")
    }

    pub fn managed_home(&self) -> PathBuf {
        self.home.join(".llm_wiki")
    }

    pub fn managed_bin_dir(&self) -> PathBuf {
        self.managed_home().join("bin")
    }

    pub fn managed_binary(&self) -> PathBuf {
        self.managed_bin_dir().join(managed_binary_name())
    }

    pub fn partial_install(&self) -> PathBuf {
        self.managed_home().join("install.partial.json")
    }
}

pub fn managed_binary_name() -> &'static str {
    if cfg!(windows) {
        "llm-wiki.exe"
    } else {
        "llm-wiki"
    }
}
