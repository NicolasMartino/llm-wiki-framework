pub struct ExpectedDoc {
    pub path: &'static str,
    pub class: &'static str,
    pub status: &'static str,
}

pub const EXPECTED: &[ExpectedDoc] = &[
    ExpectedDoc {
        path: "archive/archived.proposal.md",
        class: "Proposal",
        status: "Archived",
    },
    ExpectedDoc {
        path: "checklists/fixture.checklist.md",
        class: "Checklist",
        status: "Active",
    },
    ExpectedDoc {
        path: "decisions/fixture.decision.md",
        class: "Decision",
        status: "Accepted",
    },
    ExpectedDoc {
        path: "plans/fixture.plan.md",
        class: "Plan",
        status: "Active",
    },
    ExpectedDoc {
        path: "proposals/fixture.proposal.md",
        class: "Proposal",
        status: "Proposed",
    },
    ExpectedDoc {
        path: "references/fixture.reference.md",
        class: "Reference",
        status: "Sourced",
    },
    ExpectedDoc {
        path: "roadmaps/fixture.roadmap.md",
        class: "Roadmap",
        status: "Active",
    },
    ExpectedDoc {
        path: "specs/fixture.spec.md",
        class: "Spec",
        status: "Active",
    },
];
