use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "repforge-test")]
#[command(about = "Unified test runner for RepForge", long_about = None)]
pub struct Cli {
    /// Test categories to run (default: rust)
    #[arg(value_enum)]
    pub categories: Vec<Category>,

    /// Test name filter (passed to nextest)
    #[arg(last = true)]
    pub filter: Vec<String>,

    /// Skip infrastructure startup (assume already running)
    #[arg(long)]
    pub skip_infra: bool,

    /// Keep infrastructure running after tests
    #[arg(long)]
    pub keep_infra: bool,

    /// Stop on first test failure
    #[arg(long)]
    pub fail_fast: bool,

    /// Run browser-based e2e tests in headed mode (sets HEADED=1)
    #[arg(long)]
    pub headed: bool,

    /// Output directory for reports
    #[arg(long, default_value = "target/test-reports")]
    pub output_dir: PathBuf,

    /// Print JSON to stdout
    #[arg(long)]
    pub stdout: bool,

    /// List available test suites
    #[arg(long)]
    pub list: bool,

    /// Show full subprocess output
    #[arg(long, short)]
    pub verbose: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug)]
pub enum Category {
    Rust,
    E2e,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Rust => "rust",
            Category::E2e => "e2e",
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Cli {
    pub fn effective_categories(&self) -> Vec<Category> {
        if self.categories.is_empty() {
            vec![Category::Rust]
        } else {
            self.categories.clone()
        }
    }

    pub fn filter_args(&self) -> Vec<String> {
        self.filter.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_categories() {
        let cli = Cli {
            categories: vec![],
            filter: vec![],
            skip_infra: false,
            keep_infra: false,
            fail_fast: false,
            headed: false,
            output_dir: PathBuf::from("target/test-reports"),
            stdout: false,
            list: false,
            verbose: false,
        };
        assert_eq!(cli.effective_categories(), vec![Category::Rust]);
    }

    #[test]
    fn test_explicit_categories() {
        let cli = Cli {
            categories: vec![Category::E2e],
            filter: vec![],
            skip_infra: false,
            keep_infra: false,
            fail_fast: false,
            headed: false,
            output_dir: PathBuf::from("target/test-reports"),
            stdout: false,
            list: false,
            verbose: false,
        };
        assert_eq!(cli.effective_categories(), vec![Category::E2e]);
    }

    #[test]
    fn test_filter_args() {
        let cli = Cli {
            categories: vec![],
            filter: vec!["test_name".to_string(), "--".to_string(), "arg".to_string()],
            skip_infra: false,
            keep_infra: false,
            fail_fast: false,
            headed: false,
            output_dir: PathBuf::from("target/test-reports"),
            stdout: false,
            list: false,
            verbose: false,
        };
        assert_eq!(
            cli.filter_args(),
            vec!["test_name".to_string(), "--".to_string(), "arg".to_string()]
        );
    }
}
