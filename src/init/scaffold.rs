use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

use crate::init::answers::Answers;
use crate::init::blueprints::Blueprint;
use crate::init::collision::refuse_framework_collision;
use crate::init::compose::{InitOutput, RenderPlan, compose, resolve_folders};
use crate::init::managed_block::{RootSchemaWrites, is_root_schema_file};
use crate::init::manifest::{InitManifest, ManagedBlocks};
use crate::init::packs::Pack;
use crate::init::runtime::RuntimeManifest;
use crate::init::sources::{copy_initial_sources, validate_initial_sources};
use crate::paths::Paths;
use crate::search_profile::{ProjectSearchConfig, SearchConfig};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InitMode {
    Fresh,
    Rerun,
}

pub(super) fn create_project(
    path: &Path,
    answers: &Answers,
    plan: &RenderPlan,
    initial_sources: &[PathBuf],
) -> Result<InitMode> {
    let mode = init_mode(path);
    let previous_manifest = match mode {
        InitMode::Fresh => None,
        InitMode::Rerun => read_previous_manifest(path),
    };
    if mode == InitMode::Fresh {
        refuse_framework_collision(path)?;
    }
    validate_initial_sources(initial_sources)?;

    let output = compose(plan)?;
    let previous_output = previous_manifest.as_ref().and_then(previous_render);
    let recorded_blocks = previous_manifest
        .as_ref()
        .map(|manifest| manifest.managed_blocks.clone())
        .unwrap_or_default();
    // Everything that can refuse the run happens above this line, before the
    // first write.
    let root_schema =
        RootSchemaWrites::plan(path, &output, previous_output.as_ref(), &recorded_blocks)?;
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;

    // Commit `.llm_wiki/init.toml` BEFORE the project files. If a later scaffold
    // step fails, the tool-owned `.llm_wiki/` marker is already present, so a
    // re-run resumes as a Rerun instead of tripping the collision guard and
    // trapping the user with a half-written scaffold they must hand-delete.
    write_manifest(
        path,
        answers,
        output.resolved_packs.clone(),
        output.folders.clone(),
        root_schema.hashes().clone(),
        mode,
    )?;

    for folder in &output.folders {
        fs::create_dir_all(path.join(folder))
            .with_context(|| format!("failed to create {}", path.join(folder).display()))?;
    }

    root_schema.apply(path)?;
    for file in &output.files {
        if is_root_schema_file(&file.path) {
            continue;
        }
        if mode == InitMode::Rerun && preserves_project_knowledge(&file.path) {
            let target = path.join(&file.path);
            if target.exists() {
                continue;
            }
        }
        fs::write(path.join(&file.path), &file.contents)
            .with_context(|| format!("failed to write {}", path.join(&file.path).display()))?;
    }

    if let Some(bundle) = copy_initial_sources(path, initial_sources)? {
        println!(
            "Sources copied to `{}`. Run `wiki-ingest` to compile them into the wiki.",
            bundle.display()
        );
    }

    if let Some(drift) = schema_drift(
        previous_manifest.as_ref(),
        answers.blueprint,
        &output.resolved_packs,
        &output.folders,
    ) {
        apply_schema_drift_audit(path, &drift)?;
    }

    match mode {
        InitMode::Fresh => println!("Initialized LLM Wiki project at {}", path.display()),
        InitMode::Rerun => println!("Updated LLM Wiki project at {}", path.display()),
    }
    Ok(mode)
}

fn init_mode(path: &Path) -> InitMode {
    // Any tool-owned `.llm_wiki/` directory (even one missing `init.toml` after an
    // interrupted init) counts as a resumable project, so a partial scaffold is
    // repaired by a Rerun rather than refused as a Fresh collision.
    if path.join(".llm_wiki").is_dir() {
        InitMode::Rerun
    } else {
        InitMode::Fresh
    }
}

/// Read the previous init manifest, tolerating a corrupt/partial `init.toml`
/// (treated as absent) so a re-run can overwrite it and recover instead of
/// aborting on the parse error.
fn read_previous_manifest(path: &Path) -> Option<InitManifest> {
    match InitManifest::read_from_project(path) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("Warning: ignoring unreadable .llm_wiki/init.toml: {error:#}");
            None
        }
    }
}

/// What init rendered from the answers recorded before this run, for the
/// migration of files written before init owned a block in them.
fn previous_render(manifest: &InitManifest) -> Option<InitOutput> {
    compose(&RenderPlan {
        name: manifest.project_name.clone()?,
        description: manifest.project_description.clone()?,
        blueprint: manifest.blueprint,
        packs: Some(manifest.packs.clone()),
    })
    .ok()
}

fn preserves_project_knowledge(path: &str) -> bool {
    matches!(path, "wiki/index.md" | "wiki/log.md")
}

fn write_manifest(
    path: &Path,
    answers: &Answers,
    packs: Vec<Pack>,
    resolved_folders: Vec<String>,
    managed_blocks: ManagedBlocks,
    mode: InitMode,
) -> Result<()> {
    let manifest_dir = path.join(".llm_wiki");
    fs::create_dir_all(&manifest_dir)
        .with_context(|| format!("failed to create {}", manifest_dir.display()))?;
    let manifest = InitManifest::new(
        answers.name.clone(),
        answers.description.clone(),
        answers.blueprint,
        packs,
        resolved_folders,
        managed_blocks,
    )
    .to_toml()?;
    fs::write(manifest_dir.join("init.toml"), manifest).with_context(|| {
        format!(
            "failed to write {}",
            manifest_dir.join("init.toml").display()
        )
    })?;
    let paths = Paths::from_env()?;
    let runtime = RuntimeManifest::from_paths(&paths)?;
    fs::write(manifest_dir.join("runtime.toml"), runtime.to_toml()?).with_context(|| {
        format!(
            "failed to write {}",
            manifest_dir.join("runtime.toml").display()
        )
    })?;
    if mode == InitMode::Fresh || !manifest_dir.join("search.toml").exists() {
        seed_project_search_config(&manifest_dir, &paths, runtime.install_id)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SchemaDrift {
    date: String,
    previous_blueprint: Blueprint,
    current_blueprint: Blueprint,
    added_packs: Vec<String>,
    removed_packs: Vec<String>,
    trigger: String,
    added_folders: Vec<String>,
    orphaned_folders: Vec<String>,
}

const SCHEMA_DRIFT_START_MARKER: &str = "<!-- llm-wiki:init-schema-drift:start -->\n";
const SCHEMA_DRIFT_END_MARKER: &str = "<!-- llm-wiki:init-schema-drift:end -->\n";

impl SchemaDrift {
    fn log_entry(&self) -> String {
        format!(
            "## [{}] init | schema drift | {} -> {}\n\n\
Added packs: {}\n\
Removed packs: {}\n\
Trigger: {}\n\
Added folders: {}\n\
Orphaned folders: {}\n\
Index update: init-owned schema drift section refreshed or appended; existing catalog entries preserved\n\n\
Orphan content is preserved on disk. Markdown under wiki/ remains searchable; non-wiki folders are preserved but not indexed by wiki search.\n",
            self.date,
            self.previous_blueprint.name(),
            self.current_blueprint.name(),
            csv_or_none(&self.added_packs),
            csv_or_none(&self.removed_packs),
            self.trigger,
            csv_or_none(&self.added_folders),
            csv_or_none(&self.orphaned_folders),
        )
    }

    fn index_section(&self) -> String {
        format!(
            "{SCHEMA_DRIFT_START_MARKER}\
## Schema Drift\n\n\
Latest drift: {}\n\
Newly claimed folders: {}\n\
Orphaned folders: {}\n\
Audit trail: `wiki/log.md`\n\n\
{SCHEMA_DRIFT_END_MARKER}\n",
            self.date,
            csv_or_none(&self.added_folders),
            csv_or_none(&self.orphaned_folders),
        )
    }
}

fn schema_drift(
    previous: Option<&InitManifest>,
    current_blueprint: Blueprint,
    current_packs: &[Pack],
    current_folders: &[String],
) -> Option<SchemaDrift> {
    let previous = previous?;
    let previous_pack_set = pack_set(&previous.packs);
    let current_pack_set = pack_set(current_packs);
    let pack_changed = previous_pack_set != current_pack_set;

    let previous_folders_were_recorded = !previous.resolved_folders.is_empty();
    let previous_folder_set = if previous_folders_were_recorded {
        string_set(&previous.resolved_folders)
    } else {
        string_set(&resolve_folders(&previous.packs))
    };
    let current_folder_set = string_set(current_folders);
    let folder_set_changed = previous_folder_set != current_folder_set;
    let composition_changed = previous_folders_were_recorded && folder_set_changed;

    if !pack_changed && !composition_changed {
        return None;
    }

    Some(SchemaDrift {
        date: Utc::now().date_naive().to_string(),
        previous_blueprint: previous.blueprint,
        current_blueprint,
        added_packs: pack_diff_names(&current_pack_set, &previous_pack_set),
        removed_packs: pack_diff_names(&previous_pack_set, &current_pack_set),
        trigger: drift_trigger(pack_changed, folder_set_changed),
        added_folders: string_diff(&current_folder_set, &previous_folder_set),
        orphaned_folders: string_diff(&previous_folder_set, &current_folder_set),
    })
}

fn pack_set(packs: &[Pack]) -> BTreeSet<Pack> {
    packs.iter().copied().collect()
}

fn string_set(values: &[String]) -> BTreeSet<String> {
    values.iter().cloned().collect()
}

fn pack_diff_names(left: &BTreeSet<Pack>, right: &BTreeSet<Pack>) -> Vec<String> {
    let mut names = left
        .difference(right)
        .map(|pack| pack.name().to_string())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn string_diff(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn drift_trigger(pack_changed: bool, folder_changed: bool) -> String {
    match (pack_changed, folder_changed) {
        (true, true) => "both".to_string(),
        (true, false) => "pack-set change".to_string(),
        (false, true) => "composition change".to_string(),
        (false, false) => "none".to_string(),
    }
}

fn csv_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn apply_schema_drift_audit(path: &Path, drift: &SchemaDrift) -> Result<()> {
    append_schema_drift_log(&path.join("wiki/log.md"), &drift.log_entry())?;
    refresh_schema_drift_section(&path.join("wiki/index.md"), &drift.index_section())?;
    Ok(())
}

fn append_schema_drift_log(path: &Path, entry: &str) -> Result<()> {
    if file_ends_with(path, entry.as_bytes())? {
        return Ok(());
    }

    let separator = log_append_separator(path)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {} for append", path.display()))?;
    if !separator.is_empty() {
        file.write_all(separator.as_bytes())
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    file.write_all(entry.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn file_ends_with(path: &Path, suffix: &[u8]) -> Result<bool> {
    if suffix.is_empty() || !path.exists() {
        return Ok(false);
    }
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let len = file
        .metadata()
        .with_context(|| format!("failed to stat {}", path.display()))?
        .len();
    if len < suffix.len() as u64 {
        return Ok(false);
    }
    file.seek(SeekFrom::End(-(suffix.len() as i64)))
        .with_context(|| format!("failed to seek {}", path.display()))?;
    let mut tail = vec![0; suffix.len()];
    file.read_exact(&mut tail)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(tail == suffix)
}

fn log_append_separator(path: &Path) -> Result<&'static str> {
    if !path.exists() {
        return Ok("");
    }
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let len = file
        .metadata()
        .with_context(|| format!("failed to stat {}", path.display()))?
        .len();
    if len == 0 {
        return Ok("");
    }
    let tail_len = len.min(2) as usize;
    file.seek(SeekFrom::End(-(tail_len as i64)))
        .with_context(|| format!("failed to seek {}", path.display()))?;
    let mut tail = vec![0; tail_len];
    file.read_exact(&mut tail)
        .with_context(|| format!("failed to read {}", path.display()))?;
    if tail.ends_with(b"\n\n") {
        Ok("")
    } else if tail.ends_with(b"\n") {
        Ok("\n")
    } else {
        Ok("\n\n")
    }
}

fn refresh_schema_drift_section(path: &Path, section: &str) -> Result<()> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let updated = replace_schema_drift_section(&contents, section);
    if updated != contents {
        fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

fn replace_schema_drift_section(contents: &str, section: &str) -> String {
    if let Some((start, end)) = init_owned_schema_drift_section_bounds(contents) {
        let mut output = String::new();
        output.push_str(&contents[..start]);
        output.push_str(section);
        output.push_str(&contents[end..]);
        return output;
    }

    let mut output = contents.to_string();
    if !output.is_empty() {
        if output.ends_with("\n\n") {
            // Already separated.
        } else if output.ends_with('\n') {
            output.push('\n');
        } else {
            output.push_str("\n\n");
        }
    }
    output.push_str(section);
    output
}

fn init_owned_schema_drift_section_bounds(contents: &str) -> Option<(usize, usize)> {
    let start = contents.find(SCHEMA_DRIFT_START_MARKER)?;
    let after_start = start + SCHEMA_DRIFT_START_MARKER.len();
    let marker_end = contents[after_start..].find(SCHEMA_DRIFT_END_MARKER)?
        + after_start
        + SCHEMA_DRIFT_END_MARKER.len();
    let mut end = marker_end;
    let bytes = contents.as_bytes();
    while end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    }
    Some((start, end))
}

fn seed_project_search_config(
    manifest_dir: &Path,
    paths: &Paths,
    source_install_id: Option<String>,
) -> Result<()> {
    let Some(global_config) = SearchConfig::read(&paths.search_config())? else {
        return Ok(());
    };
    let project_config =
        ProjectSearchConfig::from_project_default(&global_config, source_install_id);
    project_config.write_atomic(&manifest_dir.join("search.toml"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_drift_section_refreshes_marked_section_and_preserves_surrounding_index_content() {
        let index = format!(
            "# Wiki Index\n\n## Specs\n\n- keep this\n\n\
{SCHEMA_DRIFT_START_MARKER}\
## Schema Drift\n\nold\n\n\
{SCHEMA_DRIFT_END_MARKER}\n\
## Archive\n\n(none yet)\n"
        );
        let section = schema_drift_test_section();

        let updated = replace_schema_drift_section(&index, &section);

        assert!(updated.starts_with("# Wiki Index\n\n## Specs\n\n- keep this\n\n"));
        assert!(updated.contains(&section));
        assert!(updated.ends_with("## Archive\n\n(none yet)\n"));
        assert!(!updated.contains("\nold\n"));
    }

    #[test]
    fn schema_drift_section_appends_when_existing_section_is_unmarked() {
        let index = "# Wiki Index\n\n## Schema Drift\n\nManual catalog notes.\n\n## Archive\n\n(none yet)\n";
        let section = schema_drift_test_section();

        let updated = replace_schema_drift_section(index, &section);

        assert!(updated.contains("## Schema Drift\n\nManual catalog notes."));
        assert!(updated.contains("## Archive\n\n(none yet)\n"));
        assert_eq!(updated.matches("## Schema Drift").count(), 2);
        assert!(updated.ends_with(&section));
    }

    #[test]
    fn schema_drift_log_append_is_idempotent_for_latest_entry() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let log = temp.path().join("log.md");
        fs::write(
            &log,
            "# Wiki Log\n\n## [2026-05-13] update | marker\n\nKeep this.\n",
        )
        .expect("log");
        let entry =
            "## [2026-05-14] init | schema drift | generic -> web-product\n\nAdded packs: api\n";

        append_schema_drift_log(&log, entry).expect("first append");
        append_schema_drift_log(&log, entry).expect("second append");

        let contents = fs::read_to_string(log).expect("log");
        assert_eq!(contents.matches("init | schema drift").count(), 1);
        assert!(contents.ends_with(entry));
    }

    fn schema_drift_test_section() -> String {
        SchemaDrift {
            date: "2026-05-14".to_string(),
            previous_blueprint: Blueprint::Generic,
            current_blueprint: Blueprint::WebProduct,
            added_packs: vec!["api".to_string()],
            removed_packs: Vec::new(),
            trigger: "both".to_string(),
            added_folders: vec!["wiki/apis".to_string()],
            orphaned_folders: Vec::new(),
        }
        .index_section()
    }
}
