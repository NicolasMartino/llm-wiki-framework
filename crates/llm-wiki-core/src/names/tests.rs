use super::{
    NEAR_MISS, SLUG_RULE, TitleError, edit_distance, is_slug, near_miss, slug_from_title, title,
};

#[test]
fn a_slug_is_lowercase_groups_joined_by_single_hyphens() {
    for slug in ["rent", "2026-taxes", "a-b-c", "x1", "9"] {
        assert!(is_slug(slug), "{slug}");
    }
    for slug in [
        "",
        "-rent",
        "rent-",
        "a--b",
        "Rent",
        "v1.2",
        "../../notes",
        "Pay Rent, May",
        "a b",
        "é",
        "a_b",
    ] {
        assert!(!is_slug(slug), "{slug}");
    }
    assert!(SLUG_RULE.contains("single hyphens"));
}

#[test]
fn a_title_gives_its_slug() {
    let cases = [
        ("Renew The Domain", "renew-the-domain"),
        ("  Pay rent: May, 2026!  ", "pay-rent-may-2026"),
        ("v1.2", "v1-2"),
        ("--x--", "x"),
        ("A", "a"),
    ];
    for (title, slug) in cases {
        assert_eq!(slug_from_title(title).as_deref(), Some(slug), "{title}");
    }
}

#[test]
fn a_title_with_no_ascii_slug_gives_none() {
    for title in ["!!!", "", "   ", "Café", "Ωmega", "日本"] {
        assert_eq!(slug_from_title(title), None, "{title}");
    }
}

#[test]
fn a_title_is_trimmed_not_empty_and_on_one_line() {
    assert_eq!(title(" Pay Rent "), Ok("Pay Rent"));
    assert_eq!(title(""), Err(TitleError::Empty));
    assert_eq!(title(" \t"), Err(TitleError::Empty));
    assert_eq!(title("Pay\nRent"), Err(TitleError::LineBreak));
    assert_eq!(title("Pay\rRent"), Err(TitleError::LineBreak));
    assert_eq!(title("Pay Rent\n"), Ok("Pay Rent"));
    assert_eq!(TitleError::Empty.reason(), "the title is empty");
    assert_eq!(
        TitleError::LineBreak.reason(),
        "the title holds a line break; it must be on one line"
    );
}

#[test]
fn the_edit_distance_counts_single_edits() {
    let cases = [
        ("", "", 0),
        ("", "abc", 3),
        ("abc", "", 3),
        ("deadline", "deadline", 0),
        ("dealine", "deadline", 1),
        ("deadlines", "deadline", 1),
        ("deadlinx", "deadline", 1),
        ("Deadline", "deadline", 1),
        ("Dedlin", "Deadline", 2),
        ("kitten", "sitting", 3),
        ("ab", "ba", 2),
    ];
    for (left, right, distance) in cases {
        assert_eq!(edit_distance(left, right), distance, "{left} -> {right}");
        assert_eq!(edit_distance(right, left), distance, "{right} -> {left}");
    }
}

#[test]
fn a_near_miss_is_one_or_two_edits_from_the_closest() {
    let known = ["Status", "Deadline", "Duration", "Importance", "Blocked by"];
    assert_eq!(near_miss("Dedline", known), Some("Deadline"));
    assert_eq!(near_miss("Blocked By", known), Some("Blocked by"));
    assert_eq!(near_miss("Statu", known), Some("Status"));
    assert_eq!(near_miss("Status", known), None);
    assert_eq!(near_miss("Team", known), None);
    assert_eq!(near_miss("Stat", known), Some("Status"));
    assert_eq!(near_miss("St", known), None);
    assert_eq!(NEAR_MISS, 2);
    assert_eq!(near_miss("ab", ["abcd", "abc"]), Some("abc"));
    assert_eq!(near_miss("ab", ["ax", "ay"]), Some("ax"));
}
