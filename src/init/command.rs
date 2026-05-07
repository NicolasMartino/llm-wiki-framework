use anyhow::Result;

use crate::cli::InitArgs;
use crate::init::answers::from_args;
use crate::init::profile::ProjectProfile;
use crate::init::scaffold::create_project;
use crate::registry;

pub fn run(args: &InitArgs) -> Result<()> {
    let answers = from_args(args)?;
    let profile = ProjectProfile::resolve(&answers.project_type, &answers.scale, answers.existing)?;
    create_project(&args.path, &answers, &profile, &args.initial_sources)?;

    if !args.no_register {
        match registry::register_project(&args.path, Some(answers.name.clone()), None, None) {
            Ok(outcome) => {
                println!("Project registered: {}", registry::outcome_id(&outcome));
            }
            Err(error) => {
                eprintln!("Warning: project initialized but registry update failed: {error}");
                eprintln!(
                    "Run `llm-wiki register {}` to register it later.",
                    args.path.display()
                );
            }
        }
    }

    Ok(())
}
