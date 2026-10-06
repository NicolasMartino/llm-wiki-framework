//! llm-wiki's nine document types: the seven every project gets ([`CORE`])
//! and the ML pack's two ([`ML`]).
//!
//! llm-wiki's project guidelines describe the same types for people and
//! agents; a test in llm-wiki renders the guidelines and fails when they and
//! these definitions stop saying the same thing.

use super::{DocumentType, FieldDefinition};

/// The fields every llm-wiki page carries: the six of its metadata block,
/// then the optional ones.
pub const FIELDS: &[FieldDefinition] = &[
    FieldDefinition::required("Document Class"),
    FieldDefinition::required("Status"),
    FieldDefinition::required("Date"),
    FieldDefinition::required("Category"),
    FieldDefinition::required("Scope"),
    FieldDefinition::required("Sources"),
    FieldDefinition::optional("Owner"),
    FieldDefinition::optional("Supersedes"),
    FieldDefinition::optional("Superseded By"),
    FieldDefinition::optional("Related"),
    FieldDefinition::optional("Promotion Target"),
];

/// Validated truth.
pub const SPEC: DocumentType = DocumentType {
    name: "Spec",
    plural: "Specs",
    suffix: "spec.md",
    folder: "wiki/specs",
    indexed: false,
    fields: FIELDS,
    statuses: &["Active", "Superseded"],
};

/// A durable choice.
pub const DECISION: DocumentType = DocumentType {
    name: "Decision",
    plural: "Decisions",
    suffix: "decision.md",
    folder: "wiki/decisions",
    indexed: false,
    fields: FIELDS,
    statuses: &["Accepted", "Superseded"],
};

/// A direction not yet accepted.
pub const PROPOSAL: DocumentType = DocumentType {
    name: "Proposal",
    plural: "Proposals",
    suffix: "proposal.md",
    folder: "wiki/proposals",
    indexed: true,
    fields: FIELDS,
    statuses: &["Proposed", "Accepted", "Deferred", "Rejected", "Superseded"],
};

/// The order deliverables come in.
pub const ROADMAP: DocumentType = DocumentType {
    name: "Roadmap",
    plural: "Roadmaps",
    suffix: "roadmap.md",
    folder: "wiki/roadmaps",
    indexed: false,
    fields: FIELDS,
    statuses: &["Draft", "Active", "Paused", "Completed", "Superseded"],
};

/// How one deliverable is carried out.
pub const PLAN: DocumentType = DocumentType {
    name: "Plan",
    plural: "Plans",
    suffix: "plan.md",
    folder: "wiki/plans",
    indexed: false,
    fields: FIELDS,
    statuses: &["Draft", "Active", "Blocked", "Completed", "Superseded"],
};

/// An investigation, the ML pack's.
pub const EXPERIMENT: DocumentType = DocumentType {
    name: "Experiment",
    plural: "Experiments",
    suffix: "experiment.md",
    folder: "wiki/experiments",
    indexed: true,
    fields: FIELDS,
    statuses: &["Planned", "Running", "Recorded"],
};

/// How a candidate performed, the ML pack's.
pub const EVAL: DocumentType = DocumentType {
    name: "Eval",
    plural: "Evals",
    suffix: "eval.md",
    folder: "wiki/evals",
    indexed: true,
    fields: FIELDS,
    statuses: &[
        "Planned",
        "Baseline",
        "Candidate",
        "Accepted",
        "Rejected",
        "Superseded",
    ],
};

/// A repeatable procedure.
pub const CHECKLIST: DocumentType = DocumentType {
    name: "Checklist",
    plural: "Checklists",
    suffix: "checklist.md",
    folder: "wiki/checklists",
    indexed: false,
    fields: FIELDS,
    statuses: &["Active", "Superseded"],
};

/// External evidence.
pub const REFERENCE: DocumentType = DocumentType {
    name: "Reference",
    plural: "References",
    suffix: "reference.md",
    folder: "wiki/references",
    indexed: false,
    fields: FIELDS,
    statuses: &["Draft", "Sourced", "Archived"],
};

/// The seven types every project gets, in the guidelines' order.
pub const CORE: &[DocumentType] = &[
    SPEC, DECISION, PROPOSAL, ROADMAP, PLAN, CHECKLIST, REFERENCE,
];

/// The ML pack's two types.
pub const ML: &[DocumentType] = &[EXPERIMENT, EVAL];
