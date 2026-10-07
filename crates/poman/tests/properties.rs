//! Property tests: any word poman does not know is refused, with nothing on
//! standard output.

use proptest::prelude::{prop_assert, prop_assert_eq};
use proptest::test_runner::{TestError, TestRunner};

#[test]
fn an_unknown_word_is_refused() -> Result<(), TestError<String>> {
    TestRunner::default().run(&"[a-z][a-z0-9-]{0,16}", |word| {
        if ["check", "new", "mcp", "help"].contains(&word.as_str()) {
            return Ok(());
        }
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = poman::run(["poman", word.as_str()], &mut out, &mut err);
        prop_assert_eq!(code, 2);
        prop_assert!(out.is_empty());
        prop_assert!(!err.is_empty());
        Ok(())
    })
}

fn repo() -> std::io::Result<tempfile::TempDir> {
    let repo = tempfile::TempDir::new()?;
    std::fs::create_dir(repo.path().join(".git"))?;
    Ok(repo)
}

fn run(dir: &std::path::Path, args: &[String]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut command_line = vec!["poman".to_owned()];
    command_line.extend_from_slice(args);
    let code = poman::run_in(
        command_line,
        dir,
        &mut std::io::empty(),
        false,
        &mut out,
        &mut err,
    );
    (code, String::from_utf8_lossy(&out).into_owned())
}

/// A title, then each flag and its value.
type Deadline = (String, Vec<(&'static str, String)>);

/// Valid values for every field.
fn valid_deadline() -> impl proptest::strategy::Strategy<Value = Deadline> {
    use proptest::prelude::{Just, Strategy, prop_oneof};
    use proptest::sample::select;
    let date = (1000_u32..=9999, 1_u32..=12, 1_u32..=28)
        .prop_map(|(year, month, day)| format!("{year}-{month:02}-{day:02}"));
    let deadline = prop_oneof![Just("none".to_owned()), date];
    let duration = (1_u32..=999).prop_map(|days| {
        if days == 1 {
            "1 day".to_owned()
        } else {
            format!("{days} days")
        }
    });
    let optional = proptest::option::of("[A-Za-z][A-Za-z ]{0,10}[A-Za-z]");
    (
        "[A-Z][a-z]{0,8}( [A-Z0-9][a-z]{0,8}){0,3}",
        select(&["Todo", "Doing", "Waiting", "Done"][..]),
        deadline,
        duration,
        select(&["low", "medium", "high"][..]),
        optional.clone(),
        optional,
    )
        .prop_map(
            |(title, status, deadline, duration, importance, track, who)| {
                let mut flags = vec![
                    ("--status", status.to_owned()),
                    ("--deadline", deadline),
                    ("--duration", duration),
                    ("--importance", importance.to_owned()),
                    ("--blocked-by", "none".to_owned()),
                ];
                flags.extend(track.map(|track| ("--track", track)));
                flags.extend(who.map(|who| ("--who", who)));
                (title, flags)
            },
        )
}

#[test]
fn any_deadline_written_from_valid_values_passes_the_check() -> Result<(), TestError<Deadline>> {
    TestRunner::default().run(&valid_deadline(), |(title, flags)| {
        let repo = repo()
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let mut args = vec!["new".to_owned(), "deadline".to_owned(), title];
        for (flag, value) in flags {
            args.push(format!("{flag}={value}"));
        }
        let (code, out) = run(repo.path(), &args);
        prop_assert_eq!(code, poman::SUCCESS, "{}", out);
        let (code, out) = run(repo.path(), &["check".to_owned()]);
        prop_assert_eq!(code, poman::SUCCESS, "{}", out);
        prop_assert!(
            out.ends_with("deadline files checked: 1, errors: 0, warnings: 0\n"),
            "{}",
            out
        );
        Ok(())
    })
}

/// One field of a valid deadline, by its line, made invalid.
fn one_invalid_field() -> impl proptest::strategy::Strategy<Value = (usize, &'static str, String)> {
    use proptest::prelude::{Strategy, prop_oneof};
    prop_oneof![
        "[a-z]{1,8}".prop_map(|value| (3, "Status", value)),
        "[0-9]{1,3}-[0-9]{2}-[0-9]{2}|[0-9]{4}-(1[3-9]|00)-[0-9]{2}"
            .prop_map(|value| (4, "Deadline", value)),
        "0[0-9]{0,2} days|[1-9] dayz|\\+[1-9] days".prop_map(|value| (5, "Duration", value)),
        "[A-Z][a-z]{0,6}".prop_map(|value| (6, "Importance", value)),
        "(/|\\./)[a-z]{1,8}\\.md".prop_map(|value| (7, "Blocked by", value)),
        "|[ ]{1,3}".prop_map(|value| (8, "Track", value)),
    ]
}

#[test]
fn any_one_invalid_field_fails_at_its_line() -> Result<(), TestError<(usize, &'static str, String)>>
{
    TestRunner::default().run(&one_invalid_field(), |(line, key, value)| {
        let repo = repo()
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let mut fields = vec![
            ("Status", "Todo".to_owned()),
            ("Deadline", "none".to_owned()),
            ("Duration", "1 day".to_owned()),
            ("Importance", "low".to_owned()),
            ("Blocked by", "none".to_owned()),
            ("Track", "Company".to_owned()),
        ];
        for field in &mut fields {
            if field.0 == key {
                field.1.clone_from(&value);
            }
        }
        let lines: Vec<String> = fields
            .iter()
            .map(|(field_key, field_value)| format!("- {field_key}: {field_value}\n"))
            .collect();
        let text = format!("# A Deadline\n\n{}", lines.concat());
        let folder = repo.path().join("wiki/deadlines");
        std::fs::create_dir_all(&folder)
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        std::fs::write(folder.join("a-deadline.deadline.md"), text)
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let (code, out) = run(repo.path(), &["check".to_owned()]);
        prop_assert_eq!(code, poman::CHECK_FAILED);
        let expected = format!("wiki/deadlines/a-deadline.deadline.md:{line}: error: `{key}` is ");
        prop_assert!(out.starts_with(&expected), "{}", out);
        prop_assert!(out.ends_with("errors: 1, warnings: 0\n"), "{}", out);
        Ok(())
    })
}
