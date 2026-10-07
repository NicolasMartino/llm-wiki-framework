//! The rules for the names a person writes: a slug, a title, and how close a
//! mistyped name is to a known one.

#[cfg(test)]
mod tests;

/// Whether `slug` is lowercase ASCII letters and digits in groups joined by
/// single hyphens: not empty, no hyphen at either end, no two together.
///
/// ```
/// use llm_wiki_core::names::is_slug;
///
/// assert!(is_slug("2026-taxes"));
/// assert!(!is_slug("-rent"));
/// assert!(!is_slug("v1.2"));
/// ```
#[must_use]
pub fn is_slug(slug: &str) -> bool {
    slug.split('-').all(|group| {
        !group.is_empty()
            && group
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

/// The slug rule, as a message names what it expected.
pub const SLUG_RULE: &str = "lowercase ASCII letters and digits in groups joined by single hyphens";

/// The slug a title gives.
///
/// ASCII letters are lowercased, digits kept, and every other run of
/// characters made one hyphen, trimmed at both ends. `None` when that leaves
/// nothing or the title holds a letter outside ASCII, where guessing would be
/// wrong.
///
/// ```
/// use llm_wiki_core::names::slug_from_title;
///
/// assert_eq!(slug_from_title("Renew The Domain").as_deref(), Some("renew-the-domain"));
/// assert_eq!(slug_from_title("!!!"), None);
/// assert_eq!(slug_from_title("Café"), None);
/// ```
#[must_use]
pub fn slug_from_title(title: &str) -> Option<String> {
    if title
        .chars()
        .any(|character| character.is_alphabetic() && !character.is_ascii())
    {
        return None;
    }
    let mut slug = String::new();
    let mut gap = false;
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            if gap && !slug.is_empty() {
                slug.push('-');
            }
            gap = false;
            slug.push(character.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    (!slug.is_empty()).then_some(slug)
}

/// Why a title cannot head a page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TitleError {
    /// Nothing is left once the spaces around it are trimmed.
    Empty,
    /// It holds a line break, which would end the field block under it.
    LineBreak,
}

impl TitleError {
    /// What is wrong, as a message says it.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Empty => "the title is empty",
            Self::LineBreak => "the title holds a line break; it must be on one line",
        }
    }
}

/// The title as a page heads it, trimmed, when it is not empty and on one
/// line.
///
/// # Errors
///
/// Returns the [`TitleError`] naming the rule the title breaks.
///
/// ```
/// use llm_wiki_core::names::{TitleError, title};
///
/// assert_eq!(title("  Pay Rent "), Ok("Pay Rent"));
/// assert_eq!(title(" "), Err(TitleError::Empty));
/// assert_eq!(title("Pay\nRent"), Err(TitleError::LineBreak));
/// ```
pub fn title(text: &str) -> Result<&str, TitleError> {
    let trimmed = text.trim();
    if trimmed.contains(['\n', '\r']) {
        Err(TitleError::LineBreak)
    } else if trimmed.is_empty() {
        Err(TitleError::Empty)
    } else {
        Ok(trimmed)
    }
}

/// The fewest single-character insertions, deletions and changes that turn
/// `left` into `right`; a change of case counts as one change.
///
/// ```
/// use llm_wiki_core::names::edit_distance;
///
/// assert_eq!(edit_distance("dealine", "deadline"), 1);
/// assert_eq!(edit_distance("Blocked By", "Blocked by"), 1);
/// ```
#[must_use]
pub fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, left_char) in left.chars().enumerate() {
        let mut current = Vec::with_capacity(previous.len());
        let mut last = row + 1;
        current.push(last);
        for ((right_char, diagonal), above) in
            right.iter().zip(&previous).zip(previous.iter().skip(1))
        {
            let change = diagonal + usize::from(left_char != *right_char);
            last = change.min(above + 1).min(last + 1);
            current.push(last);
        }
        previous = current;
    }
    previous.last().copied().unwrap_or_default()
}

/// The furthest a mistyped name may be from a known one and still be called a
/// near miss: two single-character edits.
pub const NEAR_MISS: usize = 2;

/// The known name `name` is a near miss of: one to [`NEAR_MISS`] edits away,
/// the closest first, the first known one among equals.
///
/// ```
/// use llm_wiki_core::names::near_miss;
///
/// assert_eq!(near_miss("Dedline", ["Deadline", "Duration"]), Some("Deadline"));
/// assert_eq!(near_miss("Deadline", ["Deadline"]), None);
/// assert_eq!(near_miss("Team", ["Deadline"]), None);
/// ```
#[must_use]
pub fn near_miss<'known>(
    name: &str,
    known: impl IntoIterator<Item = &'known str>,
) -> Option<&'known str> {
    known
        .into_iter()
        .map(|candidate| (edit_distance(name, candidate), candidate))
        .filter(|(distance, _)| (1..=NEAR_MISS).contains(distance))
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}
