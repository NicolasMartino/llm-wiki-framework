//! The file types of a project's wiki, starting with how a page's filename
//! names its type.

use std::error::Error;
use std::fmt;

#[cfg(test)]
mod tests;

/// A wiki page's filename split into its parts: `[slug].type.md`, or
/// `[index]-[slug].type.md` for a page kept in order.
///
/// ```
/// use llm_wiki_core::types::WikiFilename;
///
/// let name = WikiFilename::parse("03-strict-gates.plan.md")?;
/// assert_eq!(name.index(), Some("03"));
/// assert_eq!(name.slug(), "strict-gates");
/// assert_eq!(name.doc_type(), "plan");
/// assert_eq!(name.to_string(), "03-strict-gates.plan.md");
/// # Ok::<(), llm_wiki_core::types::FilenameError>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WikiFilename {
    index: Option<String>,
    slug: String,
    doc_type: String,
}

/// Why a filename is not a wiki page's filename.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilenameError {
    /// The name does not end in `.md`.
    NotMarkdown,
    /// The name holds a path separator; only the file's own name is split.
    HasPathSeparator,
    /// The name has no `.type` before `.md`, as `index.md` and `log.md`.
    MissingType,
    /// The name has more than one `.` before `.md`.
    ExtraDot,
    /// The type is empty or not lowercase ASCII letters.
    InvalidType,
    /// The slug is empty.
    EmptySlug,
}

impl fmt::Display for FilenameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::NotMarkdown => "does not end in .md",
            Self::HasPathSeparator => "holds a path separator",
            Self::MissingType => "has no .type before .md",
            Self::ExtraDot => "has more than one . before .md",
            Self::InvalidType => "has a type that is not lowercase letters",
            Self::EmptySlug => "has an empty slug",
        };
        write!(f, "the filename {reason}")
    }
}

impl Error for FilenameError {}

impl WikiFilename {
    /// Splits `filename` into its index, slug and type.
    ///
    /// Leading ASCII digits followed by `-` are the index, so `2026-goals`
    /// reads as index `2026` and slug `goals`.
    ///
    /// # Errors
    ///
    /// Returns the [`FilenameError`] naming the first rule the name breaks.
    pub fn parse(filename: &str) -> Result<Self, FilenameError> {
        if filename.contains(['/', '\\']) {
            return Err(FilenameError::HasPathSeparator);
        }
        let stem = filename
            .strip_suffix(".md")
            .ok_or(FilenameError::NotMarkdown)?;
        let (name, doc_type) = stem.rsplit_once('.').ok_or(FilenameError::MissingType)?;
        if name.contains('.') {
            return Err(FilenameError::ExtraDot);
        }
        if doc_type.is_empty() || !doc_type.chars().all(|c| c.is_ascii_lowercase()) {
            return Err(FilenameError::InvalidType);
        }
        let (index, slug) = match name.split_once('-') {
            Some((index, slug))
                if !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()) =>
            {
                (Some(index.to_owned()), slug)
            }
            _ => (None, name),
        };
        if slug.is_empty() {
            return Err(FilenameError::EmptySlug);
        }
        Ok(Self {
            index,
            slug: slug.to_owned(),
            doc_type: doc_type.to_owned(),
        })
    }

    /// The index that orders the page, as written (`"03"`), if it has one.
    #[must_use]
    pub fn index(&self) -> Option<&str> {
        self.index.as_deref()
    }

    /// The slug that names the page.
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The document type, as `plan` in `x.plan.md`.
    #[must_use]
    pub fn doc_type(&self) -> &str {
        &self.doc_type
    }
}

impl fmt::Display for WikiFilename {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(index) = &self.index {
            write!(f, "{index}-")?;
        }
        write!(f, "{}.{}.md", self.slug, self.doc_type)
    }
}
