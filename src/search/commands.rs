use anyhow::{Context, Result, bail};
use serde_json::json;

use crate::cli::{IndexArgs, OutputFormat, SearchArgs};
use crate::paths::Paths;
use crate::registry::{ProjectRegistry, RegisteredProject};
use crate::search::adapter::{BackendState, Freshness, SearchBackend, SearchFilters, SearchResult};
use crate::search::project::discover_from_cwd;
use crate::search::qmd_rs::QmdRsBackend;

pub fn index(args: &IndexArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    let mut registry = ProjectRegistry::read(&registry_path)?;
    let project = select_project(&registry, args.project.as_deref())?;
    let store_path = paths.qmd_rs_store_path(&project.id);
    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create search index dir {}", parent.display()))?;
    }

    let backend = QmdRsBackend::new();
    let status =
        backend.rebuild_or_recover(&project.id, &project.wiki_root(), &store_path, args.force)?;
    if matches!(status.state, BackendState::FeatureDisabled) {
        bail!("{}", feature_disabled_message());
    }
    if !matches!(status.state, BackendState::Ready) {
        bail!(
            "index failed for project {}: {}",
            project.id,
            status
                .message
                .unwrap_or_else(|| format!("{:?}", status.state))
        );
    }

    registry.record_index_success(&project.id, status.indexed_files, &project.wiki_root())?;
    registry.write_atomic(&registry_path)?;
    println!(
        "Indexed project: {} ({} files)",
        project.id, status.indexed_files
    );
    Ok(())
}

pub fn search(args: &SearchArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let registry = ProjectRegistry::read(&paths.project_registry())?;
    let project = select_project(&registry, args.project.as_deref())?;
    let backend = QmdRsBackend::new();
    let store_path = paths.qmd_rs_store_path(&project.id);
    let wiki_root = project.wiki_root();
    let status = backend.status(&store_path, &wiki_root)?;

    match status.state {
        BackendState::FeatureDisabled => bail!("{}", feature_disabled_message()),
        BackendState::Missing => bail!(
            "search index missing for project {}; run `llm-wiki index --project {}`",
            project.id,
            project.id
        ),
        BackendState::Corrupt | BackendState::SchemaMismatch => bail!(
            "search index unusable for project {}; run `llm-wiki index --project {} --force`",
            project.id,
            project.id
        ),
        BackendState::Ready | BackendState::Stale => {}
    }

    let filters = SearchFilters {
        document_class: args.document_class.clone(),
        status: args.status.clone(),
    };
    let mut results =
        backend.search_project(&store_path, &wiki_root, &args.query, &filters, args.limit)?;
    for result in &mut results {
        result.project_id = project.id.clone();
        result.project_name = Some(project.name.clone());
    }
    let warning = if matches!(status.state, BackendState::Stale) {
        Some(format!(
            "search index stale for project {}; run `llm-wiki index --project {}`",
            project.id, project.id
        ))
    } else {
        None
    };

    match args.format {
        OutputFormat::Text => print_search_text(&project, warning.as_deref(), &results),
        OutputFormat::Json => {
            print_search_json(&project, &args.query, warning.as_deref(), &results)
        }
    }
    Ok(())
}

fn select_project(
    registry: &ProjectRegistry,
    requested_id: Option<&str>,
) -> Result<RegisteredProject> {
    if let Some(project_id) = requested_id {
        return registry
            .project_by_id(project_id)
            .cloned()
            .with_context(|| format!("project id {project_id} is not registered"));
    }

    let Some(discovered) = discover_from_cwd()? else {
        bail!("not inside a wiki project; pass --project <id>");
    };
    registry
        .project_by_root(&discovered.project_root)
        .cloned()
        .with_context(|| {
            format!(
                "current project {} is not registered; run `llm-wiki register {}`",
                discovered.project_root.display(),
                discovered.project_root.display()
            )
        })
}

fn print_search_text(project: &RegisteredProject, warning: Option<&str>, results: &[SearchResult]) {
    if let Some(warning) = warning {
        println!("Warning: {warning}");
    }
    if results.is_empty() {
        println!("No results.");
        return;
    }
    for (index, result) in results.iter().enumerate() {
        println!(
            "{}. [{}] {} ({}) score={:.3} freshness={}",
            index + 1,
            project.id,
            result.title,
            result.path.display(),
            result.score.0,
            freshness_label(result.freshness)
        );
        if let Some(snippet) = &result.snippet {
            println!("   {snippet}");
        }
    }
}

fn print_search_json(
    project: &RegisteredProject,
    query: &str,
    warning: Option<&str>,
    results: &[SearchResult],
) {
    let results = results
        .iter()
        .map(|result| {
            json!({
                "project_id": project.id,
                "project_name": project.name,
                "path": result.path,
                "title": result.title,
                "document_class": result.document_class,
                "status": result.status,
                "score": result.score.0,
                "snippet": result.snippet,
                "backend": result.backend,
                "mode": result.mode.to_string(),
                "freshness": freshness_label(result.freshness),
            })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "query": query,
            "project_id": project.id,
            "project_name": project.name,
            "warning": warning,
            "results": results,
        }))
        .expect("serialize search json")
    );
}

fn freshness_label(freshness: Freshness) -> &'static str {
    match freshness {
        Freshness::Fresh => "fresh",
        Freshness::Stale => "stale",
        Freshness::Unknown => "unknown",
    }
}

fn feature_disabled_message() -> &'static str {
    "qmd-rs-feature-disabled: qmd-rs backend feature is disabled; rebuild with --features qmd-rs"
}
