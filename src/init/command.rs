use anyhow::Result;

use crate::cli::InitArgs;
use crate::init::answers::from_args;
use crate::init::compose::RenderPlan;
use crate::init::scaffold::create_project;
use crate::paths::Paths;
use crate::registry;

pub fn run(args: &InitArgs, context: &crate::cli::CliContext) -> Result<()> {
    context.diagnostic("command: init");
    context.diagnostic(format!("project path: {}", args.path.display()));
    context.diagnostic(format!("no register: {}", args.no_register));
    let answers = from_args(args)?;
    let plan = RenderPlan {
        name: answers.name.clone(),
        description: answers.description.clone(),
        blueprint: answers.blueprint,
        packs: answers.packs.clone(),
    };
    context.diagnostic(format!("blueprint: {}", plan.blueprint.name()));
    let resolved_packs = plan.resolved_packs();
    context.diagnostic(format!(
        "resolved packs: {}",
        resolved_packs
            .iter()
            .map(|pack| pack.name())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if args.initial_sources.is_empty() {
        context.diagnostic("initial sources: <none>");
    } else {
        for source in &args.initial_sources {
            context.diagnostic(format!("initial source: {}", source.display()));
        }
    }
    create_project(&args.path, &answers, &plan, &args.initial_sources)?;

    let mut registry_summary = None;
    if !args.no_register {
        match Paths::from_env() {
            Ok(paths) => {
                context.diagnostic(format!("registry: {}", paths.project_registry().display()))
            }
            Err(error) => context.diagnostic(format!("registry: unavailable: {error}")),
        }
        match registry::register_project_with_context(
            Some(&args.path),
            Some(answers.name.clone()),
            None,
            None,
            Some(context),
        ) {
            Ok(outcome) => {
                context.diagnostic(format!(
                    "registration outcome: {}",
                    registry::outcome_id(&outcome)
                ));
                registry_summary =
                    Some(format!("registered as {}", registry::outcome_id(&outcome)));
            }
            Err(error) => {
                context.diagnostic(format!("registration outcome: failed: {error}"));
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
