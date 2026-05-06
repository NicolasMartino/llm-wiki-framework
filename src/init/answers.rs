use std::io::{self, Write};

use anyhow::{Result, bail};

use crate::cli::InitArgs;

pub(super) struct Answers {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) project_type: String,
    pub(super) scale: String,
    pub(super) existing: bool,
}

pub(super) fn from_args(args: &InitArgs) -> Result<Answers> {
    if args.non_interactive {
        return Ok(Answers {
            name: required_flag("--name", &args.name)?,
            description: required_flag("--description", &args.description)?,
            project_type: required_flag("--type", &args.project_type)?,
            scale: required_flag("--scale", &args.scale)?,
            existing: args.existing,
        });
    }

    Ok(Answers {
        name: args.name.clone().unwrap_or(prompt("Project name")?),
        description: args
            .description
            .clone()
            .unwrap_or(prompt("One-sentence description")?),
        project_type: args
            .project_type
            .clone()
            .unwrap_or(prompt("Project type (web/api/cli/ml/data/lib/other)")?),
        scale: args
            .scale
            .clone()
            .unwrap_or(prompt("Scale (small/medium/large)")?),
        existing: args.existing,
    })
}

fn required_flag(name: &'static str, value: &Option<String>) -> Result<String> {
    value
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required with --non-interactive"))
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim().to_string();
    if value.is_empty() {
        bail!("{label} is required");
    }
    Ok(value)
}
