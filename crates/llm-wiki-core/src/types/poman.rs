//! poman's own file types, kept apart from llm-wiki's so each tool checks
//! only its own: the deadline so far.

use super::format::ValueFormat;
use super::{DocumentType, FieldDefinition};

/// A deadline's statuses, in the order its work goes through them.
pub const STATUSES: &[&str] = &["Todo", "Doing", "Waiting", "Done"];

/// How much a deadline matters.
pub const IMPORTANCES: &[&str] = &["low", "medium", "high"];

/// A deadline's fields, in the order `poman new` writes them: the five every
/// deadline carries, then the two it may.
pub const DEADLINE_FIELDS: &[FieldDefinition] = &[
    FieldDefinition::required("Status").with_format(ValueFormat::OneOf(STATUSES)),
    FieldDefinition::required("Deadline").with_format(ValueFormat::DateOrNone),
    FieldDefinition::required("Duration").with_format(ValueFormat::WorkingDays),
    FieldDefinition::required("Importance").with_format(ValueFormat::OneOf(IMPORTANCES)),
    FieldDefinition::required("Blocked by").with_format(ValueFormat::PathsOrNone),
    FieldDefinition::optional("Track").with_format(ValueFormat::NonEmpty),
    FieldDefinition::optional("Who").with_format(ValueFormat::NonEmpty),
];

/// One deadline, in one file: `wiki/deadlines/<slug>.deadline.md`, its slug
/// the whole name, with no index.
///
/// ```
/// use llm_wiki_core::types::poman::DEADLINE;
///
/// assert_eq!((DEADLINE.suffix, DEADLINE.folder), ("deadline.md", "wiki/deadlines"));
/// assert!(!DEADLINE.indexed);
/// ```
pub const DEADLINE: DocumentType = DocumentType {
    name: "Deadline",
    plural: "Deadlines",
    suffix: "deadline.md",
    folder: "wiki/deadlines",
    indexed: false,
    fields: DEADLINE_FIELDS,
    statuses: STATUSES,
};

/// Every type poman reads.
pub const ALL: &[DocumentType] = &[DEADLINE];
