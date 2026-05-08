use anyhow::{Result, bail};
use inquire::{Confirm, MultiSelect, Select, Text};

use crate::cli::InitArgs;

use super::blueprints::Blueprint;
use super::packs::Pack;

pub(super) struct Answers {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) blueprint: Blueprint,
    // `None` means "use blueprint defaults"; `Some(vec![])` means
    // "explicitly select no packs".
    pub(super) packs: Option<Vec<Pack>>,
    pub(super) existing: bool,
}

pub(super) fn from_args(args: &InitArgs) -> Result<Answers> {
    reject_retired_flags(args)?;

    if args.non_interactive {
        return Ok(Answers {
            name: required_flag("--name", &args.name)?,
            description: required_flag("--description", &args.description)?,
            blueprint: required_flag("--blueprint", &args.blueprint)?.parse()?,
            packs: parse_cli_packs(&args.packs)?,
            existing: args.existing,
        });
    }

    let name = match &args.name {
        Some(name) if !name.trim().is_empty() => name.clone(),
        _ => Text::new("Project name").prompt()?,
    };
    let description = match &args.description {
        Some(description) if !description.trim().is_empty() => description.clone(),
        _ => Text::new("One-sentence description").prompt()?,
    };
    let blueprint = match &args.blueprint {
        Some(blueprint) => blueprint.parse()?,
        None => prompt_blueprint()?,
    };
    let packs = if args.packs.is_empty() {
        Some(prompt_packs(blueprint)?)
    } else {
        Some(parse_packs(&args.packs)?)
    };
    let existing = if args.existing {
        true
    } else {
        Confirm::new("Existing codebase?")
            .with_default(false)
            .prompt()?
    };

    Ok(Answers {
        name,
        description,
        blueprint,
        packs,
        existing,
    })
}

fn reject_retired_flags(args: &InitArgs) -> Result<()> {
    if args.project_type.is_some() || args.scale.is_some() {
        bail!("--type and --scale were replaced by --blueprint and repeatable --pack");
    }
    Ok(())
}

fn required_flag(name: &'static str, value: &Option<String>) -> Result<String> {
    value
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required with --non-interactive"))
}

fn parse_packs(values: &[String]) -> Result<Vec<Pack>> {
    values.iter().map(|value| value.parse()).collect()
}

fn parse_cli_packs(values: &[String]) -> Result<Option<Vec<Pack>>> {
    if values.is_empty() {
        Ok(None)
    } else {
        parse_packs(values).map(Some)
    }
}

fn prompt_blueprint() -> Result<Blueprint> {
    let options = Blueprint::ALL
        .iter()
        .map(|blueprint| format!("{} - {}", blueprint.name(), blueprint.description()))
        .collect();
    let selected = Select::new("Blueprint", options).prompt()?;
    parse_choice_name(&selected)?.parse()
}

fn prompt_packs(blueprint: Blueprint) -> Result<Vec<Pack>> {
    let defaults = default_pack_indexes(blueprint);
    let options = Pack::ALL
        .iter()
        .map(|pack| format!("{} - {}", pack.name(), pack.description()))
        .collect();
    let prompt = MultiSelect::new("Packs", options)
        .with_default(&defaults)
        .with_help_message("Space toggles a pack; enter accepts the selection.");
    prompt
        .prompt()?
        .iter()
        .map(|selected| parse_choice_name(selected)?.parse())
        .collect()
}

fn default_pack_indexes(blueprint: Blueprint) -> Vec<usize> {
    Blueprint::default_packs(blueprint)
        .iter()
        .filter_map(|default| Pack::ALL.iter().position(|pack| pack == default))
        .collect()
}

fn parse_choice_name(selected: &str) -> Result<&str> {
    selected
        .split_once(" - ")
        .map(|(name, _)| name)
        .ok_or_else(|| anyhow::anyhow!("invalid selection: {selected}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_descriptions_do_not_use_choice_delimiter() {
        for blueprint in Blueprint::ALL {
            assert!(
                !blueprint.description().contains(" - "),
                "blueprint description must not contain the choice delimiter"
            );
        }
        for pack in Pack::ALL {
            assert!(
                !pack.description().contains(" - "),
                "pack description must not contain the choice delimiter"
            );
        }
    }
}
