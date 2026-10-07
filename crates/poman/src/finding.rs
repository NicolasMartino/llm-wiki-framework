//! What `poman check` finds: an error or a warning, at a file's line.

use std::fmt;

use llm_wiki_core::mcp::object;
use serde_json::Value;

/// How much a finding matters.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    /// The check fails.
    Error,
    /// The check passes, but something is likely wrong.
    Warning,
}

impl Severity {
    /// The word a message uses for it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// One finding: the file, from the repository root, the line, counted from
/// 1, and what is wrong there.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// The file's path from the repository root, with `/` between its parts.
    pub path: String,
    /// The line, counted from 1; 1 for a finding about the whole file.
    pub line: usize,
    /// Whether it fails the check.
    pub severity: Severity,
    /// What is wrong.
    pub message: String,
}

impl Finding {
    /// An error at `path`, `line`.
    #[must_use]
    pub fn error(path: &str, line: usize, message: impl Into<String>) -> Self {
        Self {
            path: path.to_owned(),
            line,
            severity: Severity::Error,
            message: message.into(),
        }
    }

    /// A warning at `path`, `line`.
    #[must_use]
    pub fn warning(path: &str, line: usize, message: impl Into<String>) -> Self {
        Self {
            path: path.to_owned(),
            line,
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    /// The finding as `--json` writes it.
    #[must_use]
    pub fn to_json(&self) -> Value {
        object([
            ("path", Value::from(self.path.as_str())),
            ("line", Value::from(self.line)),
            ("severity", Value::from(self.severity.word())),
            ("message", Value::from(self.message.as_str())),
        ])
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}: {}",
            self.path,
            self.line,
            self.severity.word(),
            self.message
        )
    }
}
