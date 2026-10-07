//! The file types of a project's wiki: how a page's filename names its type,
//! and each type's definition.

use std::error::Error;
use std::fmt;

use format::ValueFormat;

pub mod format;
pub mod llm_wiki;
pub mod poman;
#[cfg(test)]
mod tests;

/// A file type: how its pages are named, where they live, and what they
/// carry. llm-wiki's types are in [`llm_wiki`]; poman's, in [`poman`], are
/// kept apart, so each tool checks only its own.
///
/// ```
/// use llm_wiki_core::types::WikiFilename;
/// use llm_wiki_core::types::llm_wiki::PLAN;
///
/// let name = WikiFilename::parse("strict-gates.plan.md")?;
/// assert_eq!(format!("{}.md", name.doc_type()), PLAN.suffix);
/// assert_eq!((PLAN.name, PLAN.plural, PLAN.folder), ("Plan", "Plans", "wiki/plans"));
/// assert_eq!(PLAN.statuses.first(), Some(&"Draft"));
/// # Ok::<(), llm_wiki_core::types::FilenameError>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentType {
    /// The type's name, as a page's `Document Class` gives it: `Plan`.
    pub name: &'static str,
    /// The name of a list of its pages, as a wiki index or a status table
    /// heads it: `Plans`.
    pub plural: &'static str,
    /// The end of its pages' filenames, after the slug and its dot: `plan.md`.
    pub suffix: &'static str,
    /// The folder its pages live in, from the project's root: `wiki/plans`.
    pub folder: &'static str,
    /// Whether its filenames start with an index, as
    /// `[index]-[slug].proposal.md` does.
    pub indexed: bool,
    /// The fields its pages carry, in the order they are written.
    pub fields: &'static [FieldDefinition],
    /// The statuses its pages may have, in the order the guidelines give
    /// them.
    pub statuses: &'static [&'static str],
}

/// A field a type's pages carry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldDefinition {
    /// The field's key: `Document Class`.
    pub key: &'static str,
    /// Whether every page of the type must carry it.
    pub required: bool,
    /// The form its value must take.
    pub format: ValueFormat,
}

impl FieldDefinition {
    /// A field every page of the type must carry, its value free text.
    #[must_use]
    pub const fn required(key: &'static str) -> Self {
        Self {
            key,
            required: true,
            format: ValueFormat::FreeText,
        }
    }

    /// A field a page of the type may carry, its value free text.
    #[must_use]
    pub const fn optional(key: &'static str) -> Self {
        Self {
            key,
            required: false,
            format: ValueFormat::FreeText,
        }
    }

    /// The same field, its value held to `format`.
    ///
    /// ```
    /// use llm_wiki_core::types::FieldDefinition;
    /// use llm_wiki_core::types::format::ValueFormat;
    ///
    /// let who = FieldDefinition::optional("Who").with_format(ValueFormat::NonEmpty);
    /// assert!(!who.format.accepts(""));
    /// ```
    #[must_use]
    pub const fn with_format(self, format: ValueFormat) -> Self {
        Self { format, ..self }
    }
}

impl DocumentType {
    /// The field whose key is `key`, if the type has one.
    ///
    /// ```
    /// use llm_wiki_core::types::poman::DEADLINE;
    ///
    /// assert_eq!(DEADLINE.field("Who").map(|field| field.required), Some(false));
    /// assert_eq!(DEADLINE.field("Team"), None);
    /// ```
    #[must_use]
    pub fn field(&self, key: &str) -> Option<&'static FieldDefinition> {
        self.fields.iter().find(|field| field.key == key)
    }
}

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
        Self::split(filename, true)
    }

    /// Splits the filename of a type that takes no index: the whole name
    /// before the type is the slug, so `2026-taxes.deadline.md` is the slug
    /// `2026-taxes`.
    ///
    /// ```
    /// use llm_wiki_core::types::WikiFilename;
    ///
    /// let name = WikiFilename::parse_unindexed("2026-taxes.deadline.md")?;
    /// assert_eq!((name.index(), name.slug()), (None, "2026-taxes"));
    /// # Ok::<(), llm_wiki_core::types::FilenameError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the [`FilenameError`] naming the first rule the name breaks.
    pub fn parse_unindexed(filename: &str) -> Result<Self, FilenameError> {
        Self::split(filename, false)
    }

    fn split(filename: &str, indexed: bool) -> Result<Self, FilenameError> {
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
                if indexed && !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()) =>
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
