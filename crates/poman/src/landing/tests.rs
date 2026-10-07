use super::is_branch_name;

#[test]
fn a_branch_name_is_one_git_takes() {
    for name in [
        "master",
        "develop",
        "release/2026-10",
        "a.b",
        "x@y",
        "feature/lock-in",
    ] {
        assert!(is_branch_name(name), "{name}");
    }
    for name in [
        "",
        "@",
        "-x",
        "/x",
        "x/",
        "x.",
        "a..b",
        "a@{b",
        "a//b",
        "two words",
        "a~b",
        "a^b",
        "a:b",
        "a?b",
        "a*b",
        "a[b",
        "a\\b",
        "a\tb",
        ".hidden",
        "feature/.hidden",
        "x.lock",
        "feature/x.lock",
    ] {
        assert!(!is_branch_name(name), "{name:?}");
    }
}
