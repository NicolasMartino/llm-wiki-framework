use std::fs;
use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Result, bail};
use inquire::{MultiSelect, Select, Text};

use crate::cli::InitArgs;

use super::blueprints::Blueprint;
use super::manifest::InitManifest;
use super::packs::Pack;

pub(super) struct Answers {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) blueprint: Blueprint,
    // `None` means "use blueprint defaults"; `Some(vec![])` means
    // "explicitly select no packs".
    pub(super) packs: Option<Vec<Pack>>,
}

pub(super) fn from_args(args: &InitArgs) -> Result<Answers> {
    reject_retired_flags(args)?;

    if args.non_interactive {
        return Ok(Answers {
            name: required_flag("--name", &args.name)?,
            description: required_flag("--description", &args.description)?,
            blueprint: required_flag("--blueprint", &args.blueprint)?.parse()?,
            packs: parse_cli_packs(&args.packs)?,
        });
    }

    // Guard interactive prompts behind a TTY check so a non-terminal invocation
    // gets a clear hint instead of a cryptic inquire "not a terminal" error.
    if !std::io::stdin().is_terminal() {
        bail!(
            "interactive init requires a terminal; re-run with --non-interactive and \
             provide --name, --description, and --blueprint (see `init --help`)"
        );
    }

    let defaults = ExistingAnswers::from_project(&args.path)?;
    let name = match &args.name {
        Some(name) if !name.trim().is_empty() => name.clone(),
        _ => prompt_text("Project name", defaults.name.as_deref())?,
    };
    let description = match &args.description {
        Some(description) if !description.trim().is_empty() => description.clone(),
        _ => prompt_text("One-sentence description", defaults.description.as_deref())?,
    };
    let blueprint = match &args.blueprint {
        Some(blueprint) => blueprint.parse()?,
        None => prompt_blueprint(defaults.blueprint)?,
    };
    let packs = if args.packs.is_empty() {
        Some(prompt_packs(
            blueprint,
            prompt_pack_defaults(blueprint, &defaults),
        )?)
    } else {
        Some(parse_packs(&args.packs)?)
    };

    Ok(Answers {
        name,
        description,
        blueprint,
        packs,
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ExistingAnswers {
    name: Option<String>,
    description: Option<String>,
    blueprint: Option<Blueprint>,
    packs: Option<Vec<Pack>>,
}

impl ExistingAnswers {
    fn from_project(path: &Path) -> Result<Self> {
        let Some(manifest) = InitManifest::read_from_project(path)? else {
            return Ok(Self::default());
        };

        let name = clean_optional(manifest.project_name).or_else(|| read_project_name(path));
        let description = clean_optional(manifest.project_description)
            .or_else(|| read_project_description(path, name.as_deref()));
        Ok(Self {
            name,
            description,
            blueprint: Some(manifest.blueprint),
            packs: Some(manifest.packs),
        })
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_project_name(path: &Path) -> Option<String> {
    let index = fs::read_to_string(path.join("wiki/index.md")).ok()?;
    index.lines().find_map(|line| {
        line.strip_prefix("Project:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn read_project_description(path: &Path, project_name: Option<&str>) -> Option<String> {
    read_agent_description(path, project_name).or_else(|| read_guidelines_description(path))
}

fn read_agent_description(path: &Path, project_name: Option<&str>) -> Option<String> {
    let agents = fs::read_to_string(path.join("AGENTS.md")).ok()?;
    let prefix = match project_name {
        Some(name) if !name.trim().is_empty() => format!("This is {}: ", name.trim()),
        _ => "This is ".to_string(),
    };
    agents.lines().find_map(|line| {
        line.strip_prefix(&prefix)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn read_guidelines_description(path: &Path) -> Option<String> {
    let guidelines = fs::read_to_string(path.join("project_guidelines.md")).ok()?;
    let mut lines = guidelines.lines();
    for line in lines.by_ref() {
        if line.trim() == "## Purpose" {
            break;
        }
    }
    lines
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with("This file defines ") {
                None
            } else {
                Some(line.to_string())
            }
        })
        .next()
}

fn prompt_text(message: &'static str, default: Option<&str>) -> Result<String> {
    let prompt = Text::new(message);
    match default {
        Some(default) if !default.trim().is_empty() => Ok(prompt
            .with_initial_value(default)
            .with_default(default)
            .prompt()?),
        _ => Ok(prompt.prompt()?),
    }
}

fn prompt_blueprint(default: Option<Blueprint>) -> Result<Blueprint> {
    let options = Blueprint::ALL
        .iter()
        .map(|blueprint| format!("{} - {}", blueprint.name(), blueprint.description()))
        .collect();
    let starting_cursor = default
        .and_then(|default| {
            Blueprint::ALL
                .iter()
                .position(|blueprint| *blueprint == default)
        })
        .unwrap_or(0);
    let selected = Select::new("Blueprint", options)
        .with_starting_cursor(starting_cursor)
        .prompt()?;
    parse_choice_name(&selected)?.parse()
}

fn prompt_packs(blueprint: Blueprint, default_packs: Option<&[Pack]>) -> Result<Vec<Pack>> {
    let defaults = default_packs
        .map(pack_indexes)
        .unwrap_or_else(|| default_pack_indexes(blueprint));
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

fn prompt_pack_defaults(
    selected_blueprint: Blueprint,
    existing: &ExistingAnswers,
) -> Option<&[Pack]> {
    if existing.blueprint == Some(selected_blueprint) {
        existing.packs.as_deref()
    } else {
        None
    }
}

fn default_pack_indexes(blueprint: Blueprint) -> Vec<usize> {
    pack_indexes(Blueprint::default_packs(blueprint))
}

fn pack_indexes(packs: &[Pack]) -> Vec<usize> {
    packs
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
    use std::fs;

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

    #[test]
    fn existing_answers_read_current_manifest_values() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join(".llm_wiki")).expect("manifest dir");
        fs::write(
            temp.path().join(".llm_wiki/init.toml"),
            r#"framework_version = "1.2.3"
project_name = "Saved Project"
project_description = "A saved project."
blueprint = "web-product"
packs = ["api", "frontend"]
"#,
        )
        .expect("manifest");

        let defaults = ExistingAnswers::from_project(temp.path()).expect("defaults");
        assert_eq!(defaults.name.as_deref(), Some("Saved Project"));
        assert_eq!(defaults.description.as_deref(), Some("A saved project."));
        assert_eq!(defaults.blueprint, Some(Blueprint::WebProduct));
        assert_eq!(defaults.packs, Some(vec![Pack::Api, Pack::Frontend]));
    }

    #[test]
    fn existing_answers_fall_back_to_generated_project_files() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join(".llm_wiki")).expect("manifest dir");
        fs::create_dir_all(temp.path().join("wiki")).expect("wiki dir");
        fs::write(
            temp.path().join(".llm_wiki/init.toml"),
            r#"framework_version = "1.2.3"
blueprint = "research"
packs = ["research", "qmd-rs-scale"]
"#,
        )
        .expect("manifest");
        fs::write(
            temp.path().join("wiki/index.md"),
            "# Wiki Index\n\nProject: Legacy Project\nStage: Bootstrap\n",
        )
        .expect("index");
        fs::write(
            temp.path().join("AGENTS.md"),
            "# AGENTS.md - Project Schema\n\nThis is Legacy Project: A legacy description.\n",
        )
        .expect("agents");

        let defaults = ExistingAnswers::from_project(temp.path()).expect("defaults");
        assert_eq!(defaults.name.as_deref(), Some("Legacy Project"));
        assert_eq!(
            defaults.description.as_deref(),
            Some("A legacy description.")
        );
        assert_eq!(defaults.blueprint, Some(Blueprint::Research));
        assert_eq!(defaults.packs, Some(vec![Pack::Research, Pack::QmdRsScale]));
    }

    #[test]
    fn prompt_pack_defaults_follow_current_blueprint_only() {
        let existing = ExistingAnswers {
            name: None,
            description: None,
            blueprint: Some(Blueprint::Generic),
            packs: Some(vec![Pack::Api]),
        };

        assert_eq!(
            prompt_pack_defaults(Blueprint::Generic, &existing),
            Some(&[Pack::Api][..])
        );
        assert_eq!(prompt_pack_defaults(Blueprint::MlResearch, &existing), None);
        assert_eq!(
            default_pack_indexes(Blueprint::MlResearch),
            pack_indexes(Blueprint::MlResearch.default_packs())
        );
    }
}
