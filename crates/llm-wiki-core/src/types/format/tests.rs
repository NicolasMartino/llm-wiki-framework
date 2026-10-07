use super::{NONE, ValueFormat, paths};

const STATUS: ValueFormat = ValueFormat::OneOf(&["Todo", "Doing", "Waiting", "Done"]);

#[test]
fn free_text_takes_anything_on_one_line() {
    assert!(ValueFormat::FreeText.accepts(""));
    assert!(ValueFormat::FreeText.accepts("anything at all"));
    assert!(!ValueFormat::FreeText.accepts("two\nlines"));
    assert!(!ValueFormat::FreeText.accepts("two\rlines"));
}

#[test]
fn non_empty_text_refuses_blank() {
    assert!(ValueFormat::NonEmpty.accepts("Company"));
    assert!(!ValueFormat::NonEmpty.accepts(""));
    assert!(!ValueFormat::NonEmpty.accepts("   "));
}

#[test]
fn one_of_takes_each_word_exactly() {
    for word in ["Todo", "Doing", "Waiting", "Done"] {
        assert!(STATUS.accepts(word), "{word}");
    }
    for word in ["todo", "TODO", "Todo ", "Done!", ""] {
        assert!(!STATUS.accepts(word), "{word}");
    }
}

#[test]
fn a_date_exists_on_the_calendar() {
    let date = ValueFormat::DateOrNone;
    for value in [
        NONE,
        "2026-01-05",
        "2026-12-31",
        "2028-02-29",
        "2000-02-29",
        "2026-04-30",
    ] {
        assert!(date.accepts(value), "{value}");
    }
    for value in [
        "None",
        "2026-02-29",
        "1900-02-29",
        "2026-02-30",
        "2026-04-31",
        "2026-13-01",
        "2026-00-10",
        "2026-01-00",
        "2026-01-32",
        "2026-1-5",
        "+2026-01-05",
        "02026-01-05",
        "2026-01-05-",
        "2026-01",
        "2026-0a-05",
        "2026-01-05 ",
        "",
    ] {
        assert!(!date.accepts(value), "{value}");
    }
}

#[test]
fn every_month_has_its_length() {
    let lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (month, length) in (1..=12).zip(lengths) {
        let last = format!("2026-{month:02}-{length:02}");
        let after = format!("2026-{month:02}-{:02}", length + 1);
        assert!(ValueFormat::DateOrNone.accepts(&last), "{last}");
        assert!(!ValueFormat::DateOrNone.accepts(&after), "{after}");
    }
}

#[test]
fn a_duration_is_whole_working_days() {
    let days = ValueFormat::WorkingDays;
    for value in ["1 day", "2 days", "10 days", "120 days"] {
        assert!(days.accepts(value), "{value}");
    }
    for value in [
        "1 days", "0 days", "05 days", "+5 days", "-5 days", "5 day", "5days", "5 Days", " days",
        "days", "1.5 days", "",
    ] {
        assert!(!days.accepts(value), "{value}");
    }
}

#[test]
fn paths_are_none_or_a_comma_list() {
    let format = ValueFormat::PathsOrNone;
    assert!(format.accepts(NONE));
    assert!(format.accepts("wiki/deadlines/a.deadline.md"));
    assert!(format.accepts("wiki/a.md,wiki/b.md"));
    assert!(format.accepts("wiki/a.md, wiki/b.md"));
    for value in [
        "",
        "./wiki/a.md",
        "/wiki/a.md",
        "wiki/../a.md",
        "wiki/./a.md",
        "wiki//a.md",
        "wiki/a.md/",
        "wiki/a.md,",
        "wiki/a.md,  wiki/b.md",
        " wiki/a.md",
        "wiki/a.md ",
    ] {
        assert!(!format.accepts(value), "{value:?}");
    }
}

#[test]
fn paths_lists_each_path_in_order() {
    assert_eq!(paths(NONE), Some(Vec::new()));
    assert_eq!(paths("a.md, b.md,c.md"), Some(vec!["a.md", "b.md", "c.md"]));
    assert_eq!(paths("a.md,,b.md"), None);
}

#[test]
fn each_format_names_what_it_expects() {
    assert_eq!(ValueFormat::FreeText.expected(), "any text on one line");
    assert_eq!(
        ValueFormat::NonEmpty.expected(),
        "text on one line, not empty"
    );
    assert_eq!(
        STATUS.expected(),
        "one of `Todo`, `Doing`, `Waiting` or `Done`"
    );
    assert_eq!(ValueFormat::OneOf(&["only"]).expected(), "one of `only`");
    assert_eq!(ValueFormat::OneOf(&[]).expected(), "one of ");
    assert_eq!(
        ValueFormat::DateOrNone.expected(),
        "a date written YYYY-MM-DD that exists on the calendar, or `none`"
    );
    assert_eq!(
        ValueFormat::WorkingDays.expected(),
        "a whole number of working days, `1 day` or `<n> days`, with no sign and no leading zero"
    );
    assert_eq!(
        ValueFormat::PathsOrNone.expected(),
        "`none`, or paths from the repository root separated by commas, each with no leading `./` or `/` and no `..`"
    );
}
