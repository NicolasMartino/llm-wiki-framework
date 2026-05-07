use std::collections::BTreeSet;

use anyhow::Result;
use chrono::Utc;

use super::blueprints::Blueprint;
use super::packs::{DocType, Pack, StatusEntry};
use super::profile::ProjectProfile;
use super::template::{
    render_agent_template_with_fragments, render_project_guidelines_with_fragments,
};

const SPINE_FOLDERS: &[&str] = &[
    "raw",
    "wiki/specs",
    "wiki/decisions",
    "wiki/proposals",
    "wiki/roadmaps",
    "wiki/plans",
    "wiki/checklists",
    "wiki/references",
    "wiki/archive",
];

const CODE_FOLDERS: &[&str] = &["src", "tests", "scripts", "infra"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderPlan {
    pub name: String,
    pub description: String,
    pub blueprint: Blueprint,
    pub packs: Vec<Pack>,
    pub is_existing: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitOutput {
    pub folders: Vec<String>,
    pub files: Vec<InitFile>,
    pub resolved_packs: Vec<Pack>,
    pub doc_types: Vec<DocType>,
    pub status_vocab: Vec<StatusEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitFile {
    pub path: String,
    pub contents: String,
}

impl RenderPlan {
    pub fn resolved_packs(&self) -> Vec<Pack> {
        let selected = if self.packs.is_empty() {
            self.blueprint.default_packs()
        } else {
            self.packs.as_slice()
        };
        dedupe_packs(selected)
    }
}

pub fn compose(plan: &RenderPlan) -> Result<InitOutput> {
    let packs = plan.resolved_packs();
    let profile = ProjectProfile {
        include_ml_ai: packs.contains(&Pack::Ml),
        include_qmd: packs.contains(&Pack::QmdScale),
        is_existing: plan.is_existing,
    };
    let agents_fragments = collect_agents_fragments(&packs)?;
    let guidelines_fragments = collect_guidelines_fragments(&packs)?;

    let mut folders = BTreeSet::new();
    folders.extend(SPINE_FOLDERS.iter().map(|folder| (*folder).to_string()));
    if !plan.is_existing {
        folders.extend(CODE_FOLDERS.iter().map(|folder| (*folder).to_string()));
    }
    for pack in &packs {
        folders.extend(pack.folders().iter().map(|folder| (*folder).to_string()));
    }

    let files = vec![
        InitFile {
            path: "project_guidelines.md".to_string(),
            contents: render_project_guidelines_with_fragments(
                &plan.name,
                &plan.description,
                &profile,
                &guidelines_fragments,
            )?,
        },
        InitFile {
            path: "AGENTS.md".to_string(),
            contents: render_agent_template_with_fragments(
                &plan.name,
                &plan.description,
                &profile,
                &agents_fragments,
            )?,
        },
        InitFile {
            path: "CLAUDE.md".to_string(),
            contents: "See @AGENTS.md.\n".to_string(),
        },
        InitFile {
            path: "wiki/index.md".to_string(),
            contents: index_md(&plan.name, &packs),
        },
        InitFile {
            path: "wiki/log.md".to_string(),
            contents: log_md(&plan.name),
        },
    ];

    Ok(InitOutput {
        folders: folders.into_iter().collect(),
        files,
        resolved_packs: packs.clone(),
        doc_types: dedupe_doc_types(&packs),
        status_vocab: dedupe_status_vocab(&packs),
    })
}

fn dedupe_packs(packs: &[Pack]) -> Vec<Pack> {
    let mut deduped = BTreeSet::new();
    deduped.extend(packs.iter().copied());
    deduped.into_iter().collect()
}

fn collect_agents_fragments(packs: &[Pack]) -> Result<Vec<String>> {
    let mut fragments = Vec::new();
    for pack in packs {
        if let Some(fragment) = pack.agents_fragment()? {
            fragments.push(fragment);
        }
    }
    Ok(fragments)
}

fn collect_guidelines_fragments(packs: &[Pack]) -> Result<Vec<String>> {
    let mut fragments = catalog_fragments(packs);
    for pack in packs {
        if let Some(fragment) = pack.guidelines_fragment()? {
            fragments.push(fragment);
        }
    }
    Ok(fragments)
}

fn catalog_fragments(packs: &[Pack]) -> Vec<String> {
    let mut fragments = Vec::new();
    let doc_types = dedupe_doc_types(packs);
    if !doc_types.is_empty() {
        let mut fragment =
            "## Pack Document Types\n\n| Document type | Filename suffix | Folder |\n| --- | --- | --- |\n"
                .to_string();
        for doc_type in doc_types {
            fragment.push_str(&format!(
                "| {} | `{}` | `{}` |\n",
                doc_type.name, doc_type.suffix, doc_type.folder
            ));
        }
        fragments.push(fragment);
    }

    let status_vocab = dedupe_status_vocab(packs);
    if !status_vocab.is_empty() {
        let mut fragment =
            "## Pack Status Vocabulary\n\n| Document class | Statuses |\n| --- | --- |\n"
                .to_string();
        for entry in status_vocab {
            let statuses = entry
                .statuses
                .iter()
                .map(|status| format!("`{status}`"))
                .collect::<Vec<_>>()
                .join(", ");
            fragment.push_str(&format!("| {} | {} |\n", entry.document_class, statuses));
        }
        fragments.push(fragment);
    }
    fragments
}

fn dedupe_doc_types(packs: &[Pack]) -> Vec<DocType> {
    let mut seen = BTreeSet::new();
    let mut doc_types = Vec::new();
    for pack in packs {
        for doc_type in pack.doc_types() {
            if seen.insert((doc_type.name, doc_type.suffix, doc_type.folder)) {
                doc_types.push(*doc_type);
            }
        }
    }
    doc_types
}

fn dedupe_status_vocab(packs: &[Pack]) -> Vec<StatusEntry> {
    let mut seen = BTreeSet::new();
    let mut status_vocab = Vec::new();
    for pack in packs {
        for entry in pack.status_vocab() {
            if seen.insert(entry.document_class) {
                status_vocab.push(*entry);
            }
        }
    }
    status_vocab
}

fn index_md(project_name: &str, packs: &[Pack]) -> String {
    let mut sections = vec![
        "Specs",
        "Decisions",
        "Roadmaps",
        "References",
        "Proposals",
        "Plans",
        "Checklists",
    ];
    if packs.contains(&Pack::Ml) {
        sections.push("Experiments");
        sections.push("Evals");
    }
    if packs.contains(&Pack::Ops) || packs.contains(&Pack::OpsLite) {
        sections.push("Runbooks");
    }
    if packs.contains(&Pack::Security) {
        sections.push("Threat Models");
        sections.push("Findings");
    }
    if packs.contains(&Pack::Research) {
        sections.push("Literature");
        sections.push("Hypotheses");
    }

    let mut output = format!(
        "# Wiki Index\n\nProject: {project_name}\nStage: Bootstrap\nUpdated: {}\n\n",
        Utc::now().date_naive()
    );
    for section in sections {
        output.push_str(&format!("## {section}\n\n(none yet)\n\n"));
    }
    output.push_str("## Archive\n\n(none yet)\n");
    output
}

fn log_md(project_name: &str) -> String {
    format!(
        "# Wiki Log\n\n## [{}] create | project bootstrap\n\nInitialized `{project_name}` with the LLM Wiki framework.\n",
        Utc::now().date_naive()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(blueprint: Blueprint, packs: Vec<Pack>) -> RenderPlan {
        RenderPlan {
            name: "Fixture Project".to_string(),
            description: "A fixture project.".to_string(),
            blueprint,
            packs,
            is_existing: false,
        }
    }

    #[test]
    fn blueprint_defaults_resolve_when_no_packs_are_provided() {
        let output = compose(&plan(Blueprint::MlResearch, Vec::new())).unwrap();

        assert!(output.folders.contains(&"wiki/experiments".to_string()));
        assert!(output.folders.contains(&"wiki/datasets".to_string()));
        assert!(output.folders.contains(&"wiki/literature".to_string()));
        assert!(
            output
                .files
                .iter()
                .any(|file| file.path == "AGENTS.md" && file.contents.contains("experiment, eval"))
        );
    }

    #[test]
    fn explicit_packs_override_blueprint_defaults() {
        let output = compose(&plan(Blueprint::MlResearch, vec![Pack::Ops])).unwrap();

        assert!(output.folders.contains(&"wiki/runbooks".to_string()));
        assert!(!output.folders.contains(&"wiki/experiments".to_string()));
    }

    #[test]
    fn folders_and_vocab_are_deduplicated() {
        let output = compose(&plan(
            Blueprint::Custom,
            vec![Pack::Ops, Pack::OpsLite, Pack::Ops],
        ))
        .unwrap();

        assert_eq!(
            output
                .folders
                .iter()
                .filter(|folder| folder.as_str() == "wiki/runbooks")
                .count(),
            1
        );
        assert_eq!(
            output
                .status_vocab
                .iter()
                .filter(|entry| entry.document_class == "Runbooks")
                .count(),
            1
        );
    }
}
