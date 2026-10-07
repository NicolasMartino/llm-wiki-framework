//! Integration tests: `poman new deadline` writes the file its flags or its
//! answers give, which `poman check` accepts, and refuses, writing nothing,
//! each value, title, slug, reference or loop out of its form.

use std::fs;
use std::io::{self, Cursor};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn repo() -> io::Result<TempDir> {
    let repo = TempDir::new()?;
    fs::create_dir(repo.path().join(".git"))?;
    Ok(repo)
}

fn write(root: &Path, path: &str, text: &str) -> io::Result<()> {
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

/// Runs poman in `dir`, `input` its terminal when `terminal` is true.
fn poman(dir: &Path, args: &[&str], input: &str, terminal: bool) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut command_line = vec!["poman"];
    command_line.extend(args);
    let code = poman::run_in(
        command_line,
        dir,
        &mut Cursor::new(input),
        terminal,
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

const RENEW: [&str; 13] = [
    "new",
    "deadline",
    "Renew The Domain",
    "--status",
    "Todo",
    "--deadline",
    "2026-12-01",
    "--duration",
    "2 days",
    "--importance",
    "medium",
    "--blocked-by",
    "none",
];

const RENEW_TEXT: &str = "# Renew The Domain\n\n- Status: Todo\n- Deadline: 2026-12-01\n- Duration: 2 days\n- Importance: medium\n- Blocked by: none\n";

const OK: &str = "# Fine\n\n- Status: Todo\n- Deadline: none\n- Duration: 1 day\n- Importance: low\n- Blocked by: none\n";

fn assert_checks(root: &Path) {
    let (code, out, _) = poman(root, &["check"], "", false);
    assert_eq!(code, poman::SUCCESS, "{out}");
}

#[test]
fn flags_write_the_file_byte_for_byte() -> Result {
    let repo = repo()?;
    let (code, out, err) = poman(repo.path(), &RENEW, "", false);
    assert_eq!((code, err.as_str()), (poman::SUCCESS, ""));
    assert_eq!(
        out,
        "wrote wiki/deadlines/renew-the-domain.deadline.md; it lands on master\n"
    );
    let written = fs::read_to_string(
        repo.path()
            .join("wiki/deadlines/renew-the-domain.deadline.md"),
    )?;
    assert_eq!(written, RENEW_TEXT);
    assert_checks(repo.path());
    Ok(())
}

#[test]
fn every_field_and_blockers_are_written_in_the_type_order() -> Result {
    let repo = repo()?;
    write(repo.path(), "wiki/deadlines/a.deadline.md", OK)?;
    write(repo.path(), "wiki/deadlines/2026-b.deadline.md", OK)?;
    write(repo.path(), "poman.toml", "landing-branch = \"develop\"\n")?;
    let (code, out, _) = poman(
        repo.path(),
        &[
            "new",
            "deadline",
            "  Launch: The Site!  ",
            "--who",
            "Ana",
            "--track",
            "Company",
            "--blocked-by",
            "wiki/deadlines/a.deadline.md,wiki/deadlines/2026-b.deadline.md",
            "--importance",
            "high",
            "--duration",
            "1 day",
            "--deadline",
            "none",
            "--status",
            "Doing",
            "--slug",
            "2026-launch",
        ],
        "",
        false,
    );
    assert_eq!(code, poman::SUCCESS);
    assert_eq!(
        out,
        "wrote wiki/deadlines/2026-launch.deadline.md; it lands on develop\n"
    );
    let written = fs::read_to_string(repo.path().join("wiki/deadlines/2026-launch.deadline.md"))?;
    assert_eq!(
        written,
        "# Launch: The Site!\n\n- Status: Doing\n- Deadline: none\n- Duration: 1 day\n- Importance: high\n- Blocked by: wiki/deadlines/a.deadline.md, wiki/deadlines/2026-b.deadline.md\n- Track: Company\n- Who: Ana\n"
    );
    assert_checks(repo.path());
    Ok(())
}

#[test]
fn json_names_the_file_and_the_branch() -> Result {
    let repo = repo()?;
    let mut args = RENEW.to_vec();
    args.push("--json");
    let (code, out, _) = poman(repo.path(), &args, "", false);
    assert_eq!(code, poman::SUCCESS);
    let result: Value = serde_json::from_str(&out)?;
    assert_eq!(
        result.get("written"),
        Some(&Value::from("wiki/deadlines/renew-the-domain.deadline.md"))
    );
    assert_eq!(result.get("landing_branch"), Some(&Value::from("master")));
    let (code, out, err) = poman(repo.path(), &args, "", false);
    assert_eq!((code, err.as_str()), (poman::REFUSED, ""));
    let refused: Value = serde_json::from_str(&out)?;
    assert_eq!(refused.pointer("/error/code"), Some(&Value::from(4)));
    assert_eq!(
        refused.pointer("/error/message"),
        Some(&Value::from(
            "wiki/deadlines/renew-the-domain.deadline.md already exists; poman never overwrites a file"
        ))
    );
    Ok(())
}

#[test]
fn answers_on_a_terminal_give_the_same_file() -> Result {
    let repo = repo()?;
    let answers = "Renew The Domain\nlater\n\n2026-12-01\n0 days\n2 days\nmedium\nnone\n";
    let (code, out, err) = poman(repo.path(), &["new", "deadline"], answers, true);
    assert_eq!(code, poman::SUCCESS, "{err}");
    assert_eq!(
        out,
        "wrote wiki/deadlines/renew-the-domain.deadline.md; it lands on master\n"
    );
    assert_eq!(
        fs::read_to_string(
            repo.path()
                .join("wiki/deadlines/renew-the-domain.deadline.md")
        )?,
        RENEW_TEXT
    );
    assert_eq!(
        err,
        "Title: \
         Status (one of `Todo`, `Doing`, `Waiting` or `Done`; Enter for Todo): \
         `later` is not one of `Todo`, `Doing`, `Waiting` or `Done`\n\
         Status (one of `Todo`, `Doing`, `Waiting` or `Done`; Enter for Todo): \
         Deadline (a date written YYYY-MM-DD that exists on the calendar, or `none`): \
         Duration (a whole number of working days, `1 day` or `<n> days`, with no sign and no leading zero): \
         `0 days` is not a whole number of working days, `1 day` or `<n> days`, with no sign and no leading zero\n\
         Duration (a whole number of working days, `1 day` or `<n> days`, with no sign and no leading zero): \
         Importance (one of `low`, `medium` or `high`): \
         Blocked by (`none`, or paths from the repository root separated by commas, each with no leading `./` or `/` and no `..`): "
    );
    Ok(())
}

#[test]
fn a_bad_title_and_a_missing_slug_are_asked_again() -> Result {
    let repo = repo()?;
    let (code, out, err) = poman(
        repo.path(),
        &[
            "new",
            "deadline",
            "--status",
            "Done",
            "--deadline",
            "none",
            "--duration",
            "1 day",
            "--importance",
            "low",
            "--blocked-by",
            "none",
        ],
        " \n!!!\nBad Slug\nbangs\n",
        true,
    );
    assert_eq!(code, poman::SUCCESS, "{err}");
    assert_eq!(
        out,
        "wrote wiki/deadlines/bangs.deadline.md; it lands on master\n"
    );
    assert_eq!(
        err,
        "Title: the title is empty\nTitle: Slug: the slug `Bad Slug` is not lowercase ASCII letters and digits in groups joined by single hyphens\nSlug: "
    );
    assert!(
        fs::read_to_string(repo.path().join("wiki/deadlines/bangs.deadline.md"))?
            .starts_with("# !!!\n")
    );
    Ok(())
}

#[test]
fn input_that_ends_before_an_answer_writes_nothing() -> Result {
    let repo = repo()?;
    let (code, _, err) = poman(repo.path(), &["new", "deadline", "Rent"], "", true);
    assert_eq!(code, poman::REFUSED);
    assert!(err.ends_with("poman: no answer for Status (one of `Todo`, `Doing`, `Waiting` or `Done`; Enter for Todo); nothing written\n"), "{err}");
    assert!(!repo.path().join("wiki").exists());
    Ok(())
}

#[test]
fn off_a_terminal_every_missing_flag_is_named() -> Result {
    let repo = repo()?;
    let (code, out, err) = poman(
        repo.path(),
        &["new", "deadline", "--importance", "low"],
        "",
        false,
    );
    assert_eq!((code, out.as_str()), (poman::REFUSED, ""));
    assert_eq!(
        err,
        "poman: missing, and poman asks for them only on a terminal: the title, --status, --deadline, --duration, --blocked-by\n"
    );
    assert!(!repo.path().join("wiki").exists());
    Ok(())
}

/// Each refusal: the arguments, the exit code, and the message.
#[test]
fn each_refusal_writes_nothing_and_says_why() -> Result {
    let flags = [
        "--status",
        "Todo",
        "--deadline",
        "none",
        "--duration",
        "1 day",
        "--importance",
        "low",
        "--blocked-by",
        "none",
    ];
    let with = |front: &[&'static str]| -> Vec<&'static str> {
        let mut args = vec!["new", "deadline"];
        args.extend(front);
        args.extend(flags);
        args
    };
    let cases: Vec<(Vec<&str>, u8, &str)> = vec![
        (
            with(&["Notes", "--slug", "../../notes"]),
            4,
            "poman: --slug `../../notes` is not lowercase ASCII letters and digits in groups joined by single hyphens\n",
        ),
        (
            with(&["Version", "--slug", "v1.2"]),
            4,
            "poman: --slug `v1.2` is not lowercase ASCII letters and digits in groups joined by single hyphens\n",
        ),
        (with(&[""]), 4, "poman: the title is empty\n"),
        (
            with(&["Pay\nRent"]),
            4,
            "poman: the title holds a line break; it must be on one line\n",
        ),
        (
            with(&["!!!"]),
            4,
            "poman: poman cannot make a slug from the title `!!!`; give one with --slug\n",
        ),
        (
            with(&["Café"]),
            4,
            "poman: poman cannot make a slug from the title `Café`; give one with --slug\n",
        ),
        (
            vec![
                "new",
                "deadline",
                "Rent",
                "--status",
                "todo",
                "--deadline",
                "2026-1-5",
                "--duration",
                "+5 days",
                "--importance",
                "low",
                "--blocked-by",
                "none",
                "--track",
                "",
            ],
            4,
            "poman: --status `todo` is not one of `Todo`, `Doing`, `Waiting` or `Done`\n--deadline `2026-1-5` is not a date written YYYY-MM-DD that exists on the calendar, or `none`\n--duration `+5 days` is not a whole number of working days, `1 day` or `<n> days`, with no sign and no leading zero\n--track `` is not text on one line, not empty\n",
        ),
        (
            with(&["Rent", "--who", "**Ann**", "--track", "**Company"]),
            4,
            "poman: --track `**Company` would be read back as `Company`\n--who `**Ann**` would be read as a field written in bold, which poman check refuses\n",
        ),
        (
            with(&["Rent", "--who", " Ann"]),
            4,
            "poman: --who ` Ann` would be read back as `Ann`\n",
        ),
        (
            vec!["new", "deadlin", "Rent"],
            4,
            "poman: `deadlin` is not a type poman knows; it knows `deadline`; did you mean `deadline`?\n",
        ),
        (
            vec!["new", "task", "Rent"],
            4,
            "poman: `task` is not a type poman knows; it knows `deadline`\n",
        ),
    ];
    for (args, code, message) in cases {
        let repo = repo()?;
        let (got, out, err) = poman(repo.path(), &args, "", false);
        assert_eq!(
            (got, out.as_str(), err.as_str()),
            (code, "", message),
            "{args:?}"
        );
        assert!(!repo.path().join("wiki").exists(), "{args:?}");
    }
    Ok(())
}

#[test]
fn a_file_already_there_is_never_overwritten() -> Result {
    let repo = repo()?;
    write(
        repo.path(),
        "wiki/deadlines/renew-the-domain.deadline.md",
        "# Mine\n",
    )?;
    let (code, _, err) = poman(repo.path(), &RENEW, "", false);
    assert_eq!(code, poman::REFUSED);
    assert_eq!(
        err,
        "poman: wiki/deadlines/renew-the-domain.deadline.md already exists; poman never overwrites a file\n"
    );
    assert_eq!(
        fs::read_to_string(
            repo.path()
                .join("wiki/deadlines/renew-the-domain.deadline.md")
        )?,
        "# Mine\n"
    );
    Ok(())
}

#[test]
fn broken_references_are_refused() -> Result {
    let repo = repo()?;
    write(repo.path(), "wiki/deadlines/a.deadline.md", OK)?;
    write(repo.path(), "wiki/plans/p.plan.md", "# P\n")?;
    let mut args = RENEW.to_vec();
    args.truncate(11);
    args.extend([
        "--blocked-by",
        "wiki/deadlines/a.deadline.md, wiki/deadlines/a.deadline.md,wiki/deadlines/renew-the-domain.deadline.md,wiki/deadlines/gone.deadline.md,wiki/plans/p.plan.md,wiki/deadlines",
    ]);
    let (code, _, err) = poman(repo.path(), &args, "", false);
    assert_eq!(code, poman::REFUSED);
    assert_eq!(
        err,
        "poman: --blocked-by lists `wiki/deadlines/a.deadline.md` twice\n\
         --blocked-by names the new file itself, `wiki/deadlines/renew-the-domain.deadline.md`\n\
         --blocked-by names `wiki/deadlines/gone.deadline.md`, which does not exist\n\
         --blocked-by names `wiki/plans/p.plan.md`, which is not a deadline file in wiki/deadlines/\n\
         --blocked-by names `wiki/deadlines`, which is not a deadline file in wiki/deadlines/\n"
    );
    assert!(
        !repo
            .path()
            .join("wiki/deadlines/renew-the-domain.deadline.md")
            .exists()
    );
    Ok(())
}

#[test]
fn a_loop_the_new_file_would_close_is_refused() -> Result {
    let repo = repo()?;
    let names_b = OK.replace(
        "- Blocked by: none",
        "- Blocked by: wiki/deadlines/b.deadline.md",
    );
    write(repo.path(), "wiki/deadlines/a.deadline.md", &names_b)?;
    write(
        repo.path(),
        "wiki/deadlines/c.deadline.md",
        &OK.replace(
            "- Blocked by: none",
            "- Blocked by: wiki/deadlines/a.deadline.md",
        ),
    )?;
    write(
        repo.path(),
        "wiki/deadlines/notes.md",
        "- Blocked by: wiki/deadlines/b.deadline.md\n",
    )?;
    write(
        repo.path(),
        "wiki/deadlines/broken.deadline.md",
        "# Broken\n\n- Blocked by: ./x\n",
    )?;
    let args = [
        "new",
        "deadline",
        "B",
        "--status",
        "Todo",
        "--deadline",
        "none",
        "--duration",
        "1 day",
        "--importance",
        "low",
        "--blocked-by",
        "wiki/deadlines/c.deadline.md",
    ];
    let (code, _, err) = poman(repo.path(), &args, "", false);
    assert_eq!(code, poman::REFUSED);
    assert_eq!(
        err,
        "poman: the new file would close a loop of `Blocked by`: wiki/deadlines/b.deadline.md -> wiki/deadlines/c.deadline.md -> wiki/deadlines/a.deadline.md -> wiki/deadlines/b.deadline.md\n"
    );
    assert!(!repo.path().join("wiki/deadlines/b.deadline.md").exists());
    Ok(())
}

#[test]
fn a_broken_poman_toml_is_refused_with_the_check_message() -> Result {
    let repo = repo()?;
    write(
        repo.path(),
        "poman.toml",
        "landing-branch = \"two words\"\n",
    )?;
    let (code, _, err) = poman(repo.path(), &RENEW, "", false);
    assert_eq!(code, poman::REFUSED);
    assert_eq!(
        err,
        "poman: poman.toml is broken, so the branch the file lands on is not known:\npoman.toml:1: error: `landing-branch` must be a branch name in quotes, as `landing-branch = \"develop\"`\n"
    );
    assert!(!repo.path().join("wiki").exists());
    Ok(())
}

#[test]
fn where_poman_cannot_work_it_says_so() -> Result {
    let outside = TempDir::new()?;
    let (code, _, err) = poman(outside.path(), &RENEW, "", false);
    assert_eq!(code, poman::CANNOT_WORK);
    assert_eq!(
        err,
        "poman: not inside a Git repository; poman works in the nearest folder up that holds .git\n"
    );

    let blocked = repo()?;
    write(
        blocked.path(),
        "wiki/deadlines",
        "a file where the folder goes",
    )?;
    let (code, _, err) = poman(blocked.path(), &RENEW, "", false);
    assert_eq!(code, poman::CANNOT_WORK);
    assert!(
        err.starts_with("poman: cannot write wiki/deadlines/renew-the-domain.deadline.md: "),
        "{err}"
    );

    let locked = repo()?;
    let folder = locked.path().join("wiki/deadlines");
    fs::create_dir_all(&folder)?;
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o555))?;
    let (code, _, err) = poman(locked.path(), &RENEW, "", false);
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o755))?;
    assert_eq!(code, poman::CANNOT_WORK);
    assert_eq!(
        err,
        "poman: cannot write wiki/deadlines/renew-the-domain.deadline.md: Permission denied (os error 13)\n"
    );
    Ok(())
}
