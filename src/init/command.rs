use anyhow::Result;

use crate::cli::InitArgs;
use crate::init::answers::from_args;
use crate::init::compose::RenderPlan;
use crate::init::scaffold::{InitMode, create_project};
use crate::instance;
use crate::mcp_wiring;
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
    let init_mode = create_project(&args.path, &answers, &plan, &args.initial_sources)?;

    let mut registry_summary = None;
    let mut registered_id = None;
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
                registered_id = Some(registry::outcome_id(&outcome).to_string());
                registry_summary = Some(registry_outcome_summary(&outcome));
            }
            Err(error) => {
                context.diagnostic(format!("registration outcome: failed: {error}"));
                let bin = instance::binary_stem();
                registry_summary = Some(format!("failed (run `{bin} register <path>` to recover)"));
                eprintln!("Warning: project initialized but registry update failed: {error}");
                eprintln!(
                    "Run `{bin} register {}` to register it later.",
                    args.path.display()
                );
            }
        }
    }

    if let Some(summary) = registry_summary {
        let action = match init_mode {
            InitMode::Fresh => "initialized",
            InitMode::Rerun => "updated",
        };
        println!("Project {action}. Registry: {summary}.");
    }

    // MCP wiring is a side effect of registration: a registered project gets its
    // host `.mcp.json` wired via the shared core. When registration was skipped
    // (`--no-register`), respect the user's "don't touch hosts" intent and only
    // point at the fallback template path (unless `--no-mcp` opts out too).
    if let Some(id) = registered_id {
        registry::wire_registered_project_mcp(&id, args.no_mcp, context);
    } else if args.no_register
        && !args.no_mcp
        && let Ok(paths) = Paths::from_env()
    {
        println!("{}", mcp_wiring::staged_template_pointer(&paths));
    }

    Ok(())
}

fn registry_outcome_summary(outcome: &registry::RegisterOutcome) -> String {
    let id = registry::outcome_id(outcome);
    match outcome {
        registry::RegisterOutcome::Created(_) => format!("registered as {id}"),
        registry::RegisterOutcome::Updated(_) => format!("updated {id}"),
        registry::RegisterOutcome::Unchanged(_) => format!("already registered as {id}"),
    }
}
