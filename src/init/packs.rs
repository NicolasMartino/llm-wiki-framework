use std::fmt;
use std::str::FromStr;

use anyhow::{Result, bail};
use askama::Template;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pack {
    Api,
    Frontend,
    Library,
    Ml,
    Data,
    Ops,
    OpsLite,
    Security,
    Research,
    QmdRsScale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocType {
    pub name: &'static str,
    pub suffix: &'static str,
    pub folder: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusEntry {
    pub document_class: &'static str,
    pub statuses: &'static [&'static str],
}

const API_DOC_TYPES: &[DocType] = &[DocType {
    name: "API Spec",
    suffix: "api.md",
    folder: "wiki/apis",
}];

const FRONTEND_DOC_TYPES: &[DocType] = &[DocType {
    name: "Design",
    suffix: "design.md",
    folder: "wiki/designs",
}];

const LIBRARY_DOC_TYPES: &[DocType] = &[DocType {
    name: "Compatibility Note",
    suffix: "compat.md",
    folder: "wiki/compatibility",
}];

const ML_DOC_TYPES: &[DocType] = &[
    DocType {
        name: "Experiment",
        suffix: "experiment.md",
        folder: "wiki/experiments",
    },
    DocType {
        name: "Eval",
        suffix: "eval.md",
        folder: "wiki/evals",
    },
    DocType {
        name: "Model Card",
        suffix: "model-card.md",
        folder: "wiki/model-cards",
    },
];

const DATA_DOC_TYPES: &[DocType] = &[
    DocType {
        name: "Dataset Card",
        suffix: "dataset-card.md",
        folder: "wiki/datasets",
    },
    DocType {
        name: "Transformation Spec",
        suffix: "transform.spec.md",
        folder: "wiki/data",
    },
];

const OPS_DOC_TYPES: &[DocType] = &[
    DocType {
        name: "Runbook",
        suffix: "runbook.md",
        folder: "wiki/runbooks",
    },
    DocType {
        name: "SLO",
        suffix: "slo.md",
        folder: "wiki/slos",
    },
    DocType {
        name: "Postmortem",
        suffix: "postmortem.md",
        folder: "wiki/postmortems",
    },
];

const OPS_LITE_DOC_TYPES: &[DocType] = &[DocType {
    name: "Runbook",
    suffix: "runbook.md",
    folder: "wiki/runbooks",
}];

const SECURITY_DOC_TYPES: &[DocType] = &[
    DocType {
        name: "Threat Model",
        suffix: "threat-model.md",
        folder: "wiki/threat-models",
    },
    DocType {
        name: "Finding",
        suffix: "finding.md",
        folder: "wiki/findings",
    },
];

const RESEARCH_DOC_TYPES: &[DocType] = &[
    DocType {
        name: "Literature Note",
        suffix: "literature-note.md",
        folder: "wiki/literature",
    },
    DocType {
        name: "Hypothesis",
        suffix: "hypothesis.md",
        folder: "wiki/hypotheses",
    },
];

const EMPTY_DOC_TYPES: &[DocType] = &[];

const ML_STATUS: &[StatusEntry] = &[StatusEntry {
    document_class: "Evals",
    statuses: &[
        "Planned",
        "Baseline",
        "Candidate",
        "Accepted",
        "Rejected",
        "Superseded",
    ],
}];

const OPS_STATUS: &[StatusEntry] = &[
    StatusEntry {
        document_class: "Runbooks",
        statuses: &["Draft", "Active", "Retired"],
    },
    StatusEntry {
        document_class: "Incidents",
        statuses: &["Open", "Mitigated", "Resolved", "Reviewed"],
    },
];

const SECURITY_STATUS: &[StatusEntry] = &[StatusEntry {
    document_class: "Findings",
    statuses: &[
        "Open",
        "Triaged",
        "Accepted Risk",
        "Fixed",
        "False Positive",
    ],
}];

const RESEARCH_STATUS: &[StatusEntry] = &[StatusEntry {
    document_class: "Hypotheses",
    statuses: &["Proposed", "Testing", "Supported", "Rejected", "Superseded"],
}];

const EMPTY_STATUS: &[StatusEntry] = &[];

macro_rules! fragment_template {
    ($name:ident, $path:literal) => {
        #[derive(Template)]
        #[template(path = $path, escape = "none")]
        struct $name;
    };
}

fragment_template!(ApiAgentsTemplate, "packs/api/agents.md");
fragment_template!(ApiGuidelinesTemplate, "packs/api/project_guidelines.md");
fragment_template!(FrontendAgentsTemplate, "packs/frontend/agents.md");
fragment_template!(
    FrontendGuidelinesTemplate,
    "packs/frontend/project_guidelines.md"
);
fragment_template!(LibraryAgentsTemplate, "packs/library/agents.md");
fragment_template!(
    LibraryGuidelinesTemplate,
    "packs/library/project_guidelines.md"
);
fragment_template!(MlAgentsTemplate, "packs/ml/agents.md");
fragment_template!(MlGuidelinesTemplate, "packs/ml/project_guidelines.md");
fragment_template!(DataAgentsTemplate, "packs/data/agents.md");
fragment_template!(DataGuidelinesTemplate, "packs/data/project_guidelines.md");
fragment_template!(OpsAgentsTemplate, "packs/ops/agents.md");
fragment_template!(OpsGuidelinesTemplate, "packs/ops/project_guidelines.md");
fragment_template!(OpsLiteAgentsTemplate, "packs/ops-lite/agents.md");
fragment_template!(
    OpsLiteGuidelinesTemplate,
    "packs/ops-lite/project_guidelines.md"
);
fragment_template!(SecurityAgentsTemplate, "packs/security/agents.md");
fragment_template!(
    SecurityGuidelinesTemplate,
    "packs/security/project_guidelines.md"
);
fragment_template!(ResearchAgentsTemplate, "packs/research/agents.md");
fragment_template!(
    ResearchGuidelinesTemplate,
    "packs/research/project_guidelines.md"
);
fragment_template!(QmdRsScaleAgentsTemplate, "packs/qmd-rs-scale/agents.md");
fragment_template!(
    QmdRsScaleGuidelinesTemplate,
    "packs/qmd-rs-scale/project_guidelines.md"
);

impl Pack {
    pub const ALL: &'static [Pack] = &[
        Pack::Api,
        Pack::Frontend,
        Pack::Library,
        Pack::Ml,
        Pack::Data,
        Pack::Ops,
        Pack::OpsLite,
        Pack::Security,
        Pack::Research,
        Pack::QmdRsScale,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Pack::Api => "api",
            Pack::Frontend => "frontend",
            Pack::Library => "library",
            Pack::Ml => "ml",
            Pack::Data => "data",
            Pack::Ops => "ops",
            Pack::OpsLite => "ops-lite",
            Pack::Security => "security",
            Pack::Research => "research",
            Pack::QmdRsScale => "qmd-rs-scale",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Pack::Api => "API specs, schema conventions, stability and deprecation tracking",
            Pack::Frontend => "Design docs, component inventory, and screenshot conventions",
            Pack::Library => "Public surface tracking, compatibility notes, changelog discipline",
            Pack::Ml => "Experiments, evals, model cards, and model lineage",
            Pack::Data => "Dataset cards, schema docs, lineage, and transformation specs",
            Pack::Ops => "Runbooks, SLOs, on-call, incidents, and postmortems",
            Pack::OpsLite => "Runbooks without full SLO/on-call/postmortem machinery",
            Pack::Security => "Threat models, findings, audits, and controls",
            Pack::Research => "Literature notes, hypotheses, citations, and lab-note conventions",
            Pack::QmdRsScale => "qmd-rs-backed llm-wiki search guidance for large wikis",
        }
    }

    pub fn folders(self) -> &'static [&'static str] {
        match self {
            Pack::Api => &["wiki/apis", "raw/api"],
            Pack::Frontend => &["wiki/designs", "wiki/components", "raw/screenshots"],
            Pack::Library => &["wiki/compatibility", "examples"],
            Pack::Ml => &[
                "wiki/experiments",
                "wiki/evals",
                "wiki/model-cards",
                "models",
                "notebooks",
                "evals",
            ],
            Pack::Data => &["wiki/datasets", "wiki/data", "raw/data", "data"],
            Pack::Ops => &[
                "wiki/runbooks",
                "wiki/slos",
                "wiki/postmortems",
                "raw/incidents",
            ],
            Pack::OpsLite => &["wiki/runbooks"],
            Pack::Security => &[
                "wiki/threat-models",
                "wiki/findings",
                "wiki/controls",
                "raw/audits",
            ],
            Pack::Research => &["wiki/literature", "wiki/hypotheses", "raw/papers"],
            Pack::QmdRsScale => &[],
        }
    }

    pub fn doc_types(self) -> &'static [DocType] {
        match self {
            Pack::Api => API_DOC_TYPES,
            Pack::Frontend => FRONTEND_DOC_TYPES,
            Pack::Library => LIBRARY_DOC_TYPES,
            Pack::Ml => ML_DOC_TYPES,
            Pack::Data => DATA_DOC_TYPES,
            Pack::Ops => OPS_DOC_TYPES,
            Pack::OpsLite => OPS_LITE_DOC_TYPES,
            Pack::Security => SECURITY_DOC_TYPES,
            Pack::Research => RESEARCH_DOC_TYPES,
            Pack::QmdRsScale => EMPTY_DOC_TYPES,
        }
    }

    pub fn status_vocab(self) -> &'static [StatusEntry] {
        match self {
            Pack::Ml => ML_STATUS,
            Pack::Ops | Pack::OpsLite => OPS_STATUS,
            Pack::Security => SECURITY_STATUS,
            Pack::Research => RESEARCH_STATUS,
            Pack::Api | Pack::Frontend | Pack::Library | Pack::Data | Pack::QmdRsScale => {
                EMPTY_STATUS
            }
        }
    }

    pub fn agents_fragment(self) -> Result<String> {
        let rendered = match self {
            Pack::Api => render_fragment(ApiAgentsTemplate)?,
            Pack::Frontend => render_fragment(FrontendAgentsTemplate)?,
            Pack::Library => render_fragment(LibraryAgentsTemplate)?,
            Pack::Ml => render_fragment(MlAgentsTemplate)?,
            Pack::Data => render_fragment(DataAgentsTemplate)?,
            Pack::Ops => render_fragment(OpsAgentsTemplate)?,
            Pack::OpsLite => render_fragment(OpsLiteAgentsTemplate)?,
            Pack::Security => render_fragment(SecurityAgentsTemplate)?,
            Pack::Research => render_fragment(ResearchAgentsTemplate)?,
            Pack::QmdRsScale => render_fragment(QmdRsScaleAgentsTemplate)?,
        };
        Ok(rendered)
    }

    pub fn guidelines_fragment(self) -> Result<String> {
        let rendered = match self {
            Pack::Api => render_fragment(ApiGuidelinesTemplate)?,
            Pack::Frontend => render_fragment(FrontendGuidelinesTemplate)?,
            Pack::Library => render_fragment(LibraryGuidelinesTemplate)?,
            Pack::Ml => render_fragment(MlGuidelinesTemplate)?,
            Pack::Data => render_fragment(DataGuidelinesTemplate)?,
            Pack::Ops => render_fragment(OpsGuidelinesTemplate)?,
            Pack::OpsLite => render_fragment(OpsLiteGuidelinesTemplate)?,
            Pack::Security => render_fragment(SecurityGuidelinesTemplate)?,
            Pack::Research => render_fragment(ResearchGuidelinesTemplate)?,
            Pack::QmdRsScale => render_fragment(QmdRsScaleGuidelinesTemplate)?,
        };
        Ok(rendered)
    }
}

fn render_fragment(template: impl Template) -> Result<String> {
    let rendered = template.render()?;
    Ok(format!("{}\n", rendered.trim()))
}

impl fmt::Display for Pack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Pack {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        for pack in Pack::ALL {
            if pack.name() == normalized {
                return Ok(*pack);
            }
        }
        bail!("unknown pack: {value}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_packs_round_trip_through_name() {
        for pack in Pack::ALL {
            assert_eq!(pack.name().parse::<Pack>().unwrap(), *pack);
            assert_eq!(pack.to_string(), pack.name());
        }
    }

    #[test]
    fn pack_catalog_accessors_are_populated() {
        for pack in Pack::ALL {
            assert!(!pack.name().is_empty());
            assert!(!pack.description().is_empty());
            if *pack != Pack::QmdRsScale {
                assert!(!pack.folders().is_empty(), "{} has no folders", pack.name());
                assert!(
                    !pack.doc_types().is_empty(),
                    "{} has no doc types",
                    pack.name()
                );
            }
            assert!(!pack.agents_fragment().unwrap().is_empty());
            assert!(!pack.guidelines_fragment().unwrap().is_empty());
        }
    }
}
