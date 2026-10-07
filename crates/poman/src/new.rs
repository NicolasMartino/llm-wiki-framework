//! `poman new deadline`: a deadline file written from flags, or from answers
//! on a terminal, after every check `poman check` would make of it.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::Path;

use llm_wiki_core::names::{SLUG_RULE, is_slug, slug_from_title, title};
use llm_wiki_core::types::FieldDefinition;
use llm_wiki_core::types::format::{ValueFormat, paths};
use llm_wiki_core::types::poman::DEADLINE;

use crate::check::is_deadline_file;
use crate::deadline::{BLOCKED_BY, check_text};
use crate::finding::Finding;
use crate::graph::{Graph, loop_through};
use crate::landing::{FILE, landing_branch};
use crate::repo::root_from;
use crate::{CANNOT_WORK, OUTPUT_FAILED, REFUSED};

/// What `poman new deadline` was given.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Request {
    /// The title, if given.
    pub title: Option<String>,
    /// The slug, if given.
    pub slug: Option<String>,
    /// Each field given, by its key.
    pub values: BTreeMap<&'static str, String>,
}

/// The file written, and the branch it lands on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Written {
    /// Its path from the repository root.
    pub path: String,
    /// The branch it lands on.
    pub branch: String,
}

/// Why nothing was written, and the exit code that says so.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refusal {
    /// The exit code.
    pub code: u8,
    /// Why.
    pub message: String,
}

impl Refusal {
    fn new(code: u8, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<io::Error> for Refusal {
    fn from(error: io::Error) -> Self {
        Self::new(
            OUTPUT_FAILED,
            format!("cannot ask on the terminal: {error}"),
        )
    }
}

/// A terminal to ask on: answers read from `input`, questions written to
/// `output`.
pub struct Terminal<'io> {
    /// Where the answers come from.
    pub input: &'io mut dyn BufRead,
    /// Where the questions go.
    pub output: &'io mut dyn Write,
}

/// The flag that gives the field `key`: `--blocked-by` for `Blocked by`.
#[must_use]
pub fn flag(key: &str) -> String {
    format!("--{}", key.to_ascii_lowercase().replace(' ', "-"))
}

/// Writes the deadline `request` asks for, in the repository holding `dir`,
/// asking on `terminal` for what is missing when there is one.
///
/// # Errors
///
/// Returns a [`Refusal`] when nothing is written: outside a Git repository,
/// a broken `poman.toml`, a title, slug or value out of its form, a field
/// missing off a terminal, a file already there, a broken reference, a loop
/// the new file would close, or a file that could not be written.
pub fn new_deadline(
    dir: &Path,
    mut request: Request,
    mut terminal: Option<Terminal<'_>>,
) -> Result<Written, Refusal> {
    let root = root_from(dir).ok_or_else(|| {
        Refusal::new(
            CANNOT_WORK,
            "not inside a Git repository; poman works in the nearest folder up that holds .git",
        )
    })?;
    let branch = landing_branch(&root).map_err(|findings| {
        let lines: Vec<String> = findings.iter().map(Finding::to_string).collect();
        Refusal::new(
            REFUSED,
            format!(
                "{FILE} is broken, so the branch the file lands on is not known:\n{}",
                lines.join("\n")
            ),
        )
    })?;
    check_given(&request)?;
    ask_missing(&mut request, terminal.as_mut())?;
    let title = request
        .title
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_owned();
    let slug = match (request.slug.take(), slug_from_title(&title)) {
        (Some(slug), _) | (None, Some(slug)) => slug,
        (None, None) => match terminal.as_mut() {
            Some(terminal) => ask(terminal, "Slug", slug_answer)?,
            None => {
                return Err(Refusal::new(
                    REFUSED,
                    format!(
                        "poman cannot make a slug from the title `{title}`; give one with --slug"
                    ),
                ));
            }
        },
    };
    let path = format!("{}/{slug}.{}", DEADLINE.folder, DEADLINE.suffix);
    if fs::symlink_metadata(root.join(&path)).is_ok() {
        return Err(Refusal::new(
            REFUSED,
            format!("{path} already exists; poman never overwrites a file"),
        ));
    }
    let blockers: Vec<String> = request
        .values
        .get(BLOCKED_BY)
        .and_then(|value| paths(value))
        .unwrap_or_default()
        .into_iter()
        .map(str::to_owned)
        .collect();
    check_blockers(&root, &path, &blockers)?;
    if !blockers.is_empty() {
        request.values.insert(BLOCKED_BY, blockers.join(", "));
    }
    let text = contents(&title, &request.values);
    write_new(&root, &path, &text)?;
    Ok(Written { path, branch })
}

/// Refuses every flag given out of its form, all named at once.
fn check_given(request: &Request) -> Result<(), Refusal> {
    let mut problems = Vec::new();
    if let Some(Err(error)) = request.title.as_deref().map(title) {
        problems.push(error.reason().to_owned());
    }
    if let Some(slug) = request.slug.as_deref().filter(|slug| !is_slug(slug)) {
        problems.push(format!("--slug `{slug}` is not {SLUG_RULE}"));
    }
    for field in DEADLINE.fields {
        if let Some(value) = request.values.get(field.key)
            && !field.format.accepts(value)
        {
            problems.push(format!(
                "{} `{value}` is not {}",
                flag(field.key),
                field.format.expected()
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(Refusal::new(REFUSED, problems.join("\n")))
    }
}

/// Asks on the terminal for the title and each mandatory field left out, or
/// refuses, naming every one, when there is no terminal.
fn ask_missing(request: &mut Request, terminal: Option<&mut Terminal<'_>>) -> Result<(), Refusal> {
    let fields: Vec<&FieldDefinition> = DEADLINE
        .fields
        .iter()
        .filter(|field| field.required && !request.values.contains_key(field.key))
        .collect();
    let Some(terminal) = terminal else {
        let mut missing: Vec<String> = fields.iter().map(|field| flag(field.key)).collect();
        if request.title.is_none() {
            missing.insert(0, "the title".to_owned());
        }
        if missing.is_empty() {
            return Ok(());
        }
        return Err(Refusal::new(
            REFUSED,
            format!(
                "missing, and poman asks for them only on a terminal: {}",
                missing.join(", ")
            ),
        ));
    };
    if request.title.is_none() {
        request.title = Some(ask(terminal, "Title", title_answer)?);
    }
    for field in fields {
        let format = field.format;
        let first_status = DEADLINE.statuses.first().copied().unwrap_or_default();
        let question = if field.key == "Status" {
            format!(
                "{} ({}; Enter for {first_status})",
                field.key,
                format.expected()
            )
        } else {
            format!("{} ({})", field.key, format.expected())
        };
        let answer = ask(terminal, &question, |answer| {
            if field.key == "Status" && answer.is_empty() {
                Ok(first_status.to_owned())
            } else {
                accepted(format, answer)
            }
        })?;
        request.values.insert(field.key, answer);
    }
    Ok(())
}

fn title_answer(answer: &str) -> Result<String, String> {
    title(answer)
        .map(str::to_owned)
        .map_err(|error| error.reason().to_owned())
}

fn slug_answer(answer: &str) -> Result<String, String> {
    if is_slug(answer) {
        Ok(answer.to_owned())
    } else {
        Err(format!("the slug `{answer}` is not {SLUG_RULE}"))
    }
}

fn accepted(format: ValueFormat, answer: &str) -> Result<String, String> {
    if format.accepts(answer) {
        Ok(answer.to_owned())
    } else {
        Err(format!("`{answer}` is not {}", format.expected()))
    }
}

/// Asks `question` until `check` takes the answer, saying why each answer it
/// refuses is wrong.
fn ask(
    terminal: &mut Terminal<'_>,
    question: &str,
    check: impl Fn(&str) -> Result<String, String>,
) -> Result<String, Refusal> {
    loop {
        write!(terminal.output, "{question}: ")?;
        terminal.output.flush()?;
        let mut line = String::new();
        terminal.input.read_line(&mut line)?;
        if line.is_empty() {
            return Err(Refusal::new(
                REFUSED,
                format!("no answer for {question}; nothing written"),
            ));
        }
        match check(line.trim_end_matches(['\n', '\r'])) {
            Ok(answer) => return Ok(answer),
            Err(reason) => writeln!(terminal.output, "{reason}")?,
        }
    }
}

/// Refuses a `Blocked by` path that is listed twice, is the new file, does
/// not exist or is not a deadline file, and a loop the new file would close.
fn check_blockers(root: &Path, path: &str, blockers: &[String]) -> Result<(), Refusal> {
    let mut problems = Vec::new();
    for (position, blocker) in blockers.iter().enumerate() {
        if blockers
            .iter()
            .take(position)
            .any(|earlier| earlier == blocker)
        {
            problems.push(format!("--blocked-by lists `{blocker}` twice"));
        } else if blocker == path {
            problems.push(format!("--blocked-by names the new file itself, `{path}`"));
        } else if !root.join(blocker).exists() {
            problems.push(format!(
                "--blocked-by names `{blocker}`, which does not exist"
            ));
        } else if !is_deadline_file(root, blocker) {
            problems.push(format!(
                "--blocked-by names `{blocker}`, which is not a deadline file in {}/",
                DEADLINE.folder
            ));
        }
    }
    if !problems.is_empty() {
        return Err(Refusal::new(REFUSED, problems.join("\n")));
    }
    let mut graph = existing_graph(root);
    graph.insert(path.to_owned(), blockers.to_vec());
    loop_through(&graph, path).map_or(Ok(()), |walk| {
        Err(Refusal::new(
            REFUSED,
            format!(
                "the new file would close a loop of `{BLOCKED_BY}`: {}",
                walk.join(" -> ")
            ),
        ))
    })
}

/// The `Blocked by` paths of every deadline file already in the folder.
fn existing_graph(root: &Path) -> Graph {
    let entries = fs::read_dir(root.join(DEADLINE.folder))
        .into_iter()
        .flatten()
        .filter_map(Result::ok);
    let mut graph = Graph::new();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{}/{name}", DEADLINE.folder);
        if !is_deadline_file(root, &path) {
            continue;
        }
        let text = fs::read_to_string(entry.path()).unwrap_or_default();
        if let (_, Some(blockers)) = check_text(&path, &text) {
            graph.insert(path, blockers.paths);
        }
    }
    graph
}

/// The file's text: the title, a blank line, and the fields in the type's
/// order, the optional ones only when given.
fn contents(title: &str, values: &BTreeMap<&'static str, String>) -> String {
    let fields: Vec<String> = DEADLINE
        .fields
        .iter()
        .filter_map(|field| {
            values
                .get(field.key)
                .map(|value| format!("- {}: {value}\n", field.key))
        })
        .collect();
    format!("# {title}\n\n{}", fields.concat())
}

fn write_new(root: &Path, path: &str, text: &str) -> Result<(), Refusal> {
    let cannot =
        |error: io::Error| Refusal::new(CANNOT_WORK, format!("cannot write {path}: {error}"));
    fs::create_dir_all(root.join(DEADLINE.folder)).map_err(cannot)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(path))
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(cannot)
}
