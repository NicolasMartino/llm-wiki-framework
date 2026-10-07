//! The forms a field's value may take, each defined once so the code that
//! writes a value and the code that checks it read the same rule.

#[cfg(test)]
mod tests;

/// The form a field's value must take. Every form refuses a value holding a
/// line break, since a field's value is read from one line.
///
/// ```
/// use llm_wiki_core::types::format::ValueFormat;
///
/// assert!(ValueFormat::WorkingDays.accepts("10 days"));
/// assert!(!ValueFormat::WorkingDays.accepts("05 days"));
/// assert!(ValueFormat::DateOrNone.accepts("2028-02-29"));
/// assert!(!ValueFormat::DateOrNone.accepts("2026-02-29"));
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueFormat {
    /// Any text: llm-wiki's own fields.
    FreeText,
    /// Any text but an empty one.
    NonEmpty,
    /// One of these words, written exactly.
    OneOf(&'static [&'static str]),
    /// `none`, or a date written `YYYY-MM-DD` that exists on the calendar.
    DateOrNone,
    /// A whole number of working days: `1 day`, or `<n> days` for any other
    /// number, in ASCII digits with no sign and no leading zero.
    WorkingDays,
    /// `none`, or paths from the repository root separated by commas, each
    /// with no leading `./` or `/` and no `..`.
    PathsOrNone,
}

impl ValueFormat {
    /// Whether `value` is written in this form.
    #[must_use]
    pub fn accepts(self, value: &str) -> bool {
        if value.contains(['\n', '\r']) {
            return false;
        }
        match self {
            Self::FreeText => true,
            Self::NonEmpty => !value.trim().is_empty(),
            Self::OneOf(words) => words.contains(&value),
            Self::DateOrNone => value == NONE || is_date(value),
            Self::WorkingDays => is_working_days(value),
            Self::PathsOrNone => value == NONE || paths(value).is_some(),
        }
    }

    /// The form, as a message names what it expected.
    ///
    /// ```
    /// use llm_wiki_core::types::format::ValueFormat;
    ///
    /// let importance = ValueFormat::OneOf(&["low", "medium", "high"]);
    /// assert_eq!(importance.expected(), "one of `low`, `medium` or `high`");
    /// ```
    #[must_use]
    pub fn expected(self) -> String {
        match self {
            Self::FreeText => "any text on one line".to_owned(),
            Self::NonEmpty => "text on one line, not empty".to_owned(),
            Self::OneOf(words) => format!("one of {}", or_list(words)),
            Self::DateOrNone => {
                "a date written YYYY-MM-DD that exists on the calendar, or `none`".to_owned()
            }
            Self::WorkingDays => {
                "a whole number of working days, `1 day` or `<n> days`, with no sign and no leading zero"
                    .to_owned()
            }
            Self::PathsOrNone => "`none`, or paths from the repository root separated by commas, each with no leading `./` or `/` and no `..`".to_owned(),
        }
    }
}

/// The word a field holds when it names nothing: no deadline, no blocker.
pub const NONE: &str = "none";

/// The paths a [`ValueFormat::PathsOrNone`] value lists, in its order: none
/// for `none`, and `None` when the value is not written in that form.
///
/// ```
/// use llm_wiki_core::types::format::paths;
///
/// assert_eq!(paths("none"), Some(vec![]));
/// assert_eq!(paths("wiki/a.md,wiki/b.md, wiki/c.md"), Some(vec!["wiki/a.md", "wiki/b.md", "wiki/c.md"]));
/// assert_eq!(paths("./wiki/a.md"), None);
/// ```
#[must_use]
pub fn paths(value: &str) -> Option<Vec<&str>> {
    if value == NONE {
        return Some(Vec::new());
    }
    value
        .split(',')
        .enumerate()
        .map(|(position, path)| {
            let path = if position == 0 {
                path
            } else {
                path.strip_prefix(' ').unwrap_or(path)
            };
            is_root_path(path).then_some(path)
        })
        .collect()
}

/// A path written from the repository root: not empty, no space at either
/// end, no leading `/`, and no empty, `.` or `..` part.
fn is_root_path(path: &str) -> bool {
    path.trim() == path
        && !path.is_empty()
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn is_date(value: &str) -> bool {
    let mut parts = value.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let (Some(year), Some(month), Some(day)) = (digits(year, 4), digits(month, 2), digits(day, 2))
    else {
        return false;
    };
    (1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)
}

/// The number `text` writes in exactly `width` ASCII digits.
fn digits(text: &str, width: usize) -> Option<u32> {
    if text.len() != width || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn is_working_days(value: &str) -> bool {
    if value == "1 day" {
        return true;
    }
    let Some(number) = value.strip_suffix(" days") else {
        return false;
    };
    number != "1"
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && number.bytes().next().is_some_and(|first| first != b'0')
}

/// `` `a`, `b` or `c` ``.
fn or_list(words: &[&str]) -> String {
    let quoted: Vec<String> = words.iter().map(|word| format!("`{word}`")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => quoted.concat(),
    }
}
