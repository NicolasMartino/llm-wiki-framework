use anyhow::Result;

use crate::cli::InitArgs;
use crate::init::answers::from_args;
use crate::init::scaffold::create_project;
use crate::registry;

pub fn run(args: &InitArgs, _context: &crate::cli::CliContext) -> Result<()> {
    let answers = from_args(args)?;
    create_project(&args.path, &answers, &args.initial_sources)?;

    let mut registry_summary = None;
    if !args.no_register {
        match registry::register_project(Some(&args.path), Some(answers.name.clone()), None, None) {
            Ok(outcome) => {
                registry_summary =
                    Some(format!("registered as {}", registry::outcome_id(&outcome)));
            }
            Err(error) => {
                registry_summary =
                    Some("failed (run `llm-wiki register <path>` to recover)".to_string());
                eprintln!("Warning: project initialized but registry update failed: {error}");
                eprintln!(
                    "Run `llm-wiki register {}` to register it later.",
                    args.path.display()
                );
            }
        }
    }

    if let Some(summary) = registry_summary {
        println!("Project initialized. Registry: {summary}.");
    }

    Ok(())
}
