# Electric Cars Search Eval

- Document Class: Eval
- Status: Active
- Date: 2026-05-12
- Category: Search infrastructure
- Scope: Domain retrieval eval for electric-car battery technology model comparison.
- Sources: raw/research/2026-05-10-ev-battery-technologies/manifest.md; raw/research/2026-05-10-ev-battery-technologies/research-summary.md

This eval gates retrieval quality on a frozen electric-car battery technology
corpus. The query table is intentionally hidden from the search index so
candidate models cannot retrieve the labels from the eval definition itself.

<!-- llm-wiki-search-ignore-start -->

| ID | Split | Query | Purpose | Applicable modes | Draft expected target pages |
| --- | --- | --- | --- | --- | --- |
| C1 | Calibration | `which battery chemistry is best established for affordable standard range EVs` | Conceptual LFP/phosphate retrieval | all | `wiki/references/lfp-lmfp-phosphate-lithium-ion.reference.md`, `wiki/specs/ev-battery-technology-landscape.spec.md` |
| C2 | Calibration | `why do high nickel NMC and NCA batteries still matter for electric cars` | High-energy lithium-ion tradeoff retrieval | all | `wiki/references/nmc-nca-high-nickel-lithium-ion.reference.md`, `wiki/specs/ev-battery-technology-landscape.spec.md` |
| C3 | Calibration | `GM LG LMR manganese rich cells for trucks and SUVs 2028` | Mixed exact/company/timeline retrieval | all | `wiki/references/lmr-high-manganese-lithium-ion.reference.md`, `wiki/roadmaps/battery-commercialization-timeline.roadmap.md` |
| C4 | Calibration | `sodium ion batteries cold climate low cost short range EVs` | Emerging chemistry use-case retrieval | all | `wiki/references/sodium-ion-batteries.reference.md`, `wiki/hypotheses/sodium-ion-low-cost-ev-impact.hypothesis.md` |
| C5 | Calibration | `silicon anode lithium ion charging performance swelling cycle life` | Anode improvement retrieval | all | `wiki/references/silicon-anode-lithium-ion.reference.md`, `wiki/specs/ev-battery-technology-landscape.spec.md` |
| C6 | Calibration | `how semi solid batteries differ from all solid state batteries` | Boundary between adjacent concepts | all | `wiki/references/semi-solid-quasi-solid-batteries.reference.md`, `wiki/references/all-solid-state-lithium-metal-batteries.reference.md`, `wiki/specs/ev-battery-technology-landscape.spec.md` |
| C7 | Calibration | `how battery technology shifts could affect used EV residual value` | Hypothesis/rationale retrieval | all | `wiki/hypotheses/battery-technology-residual-value-impact.hypothesis.md`, `wiki/hypotheses/lfp-mainstream-value-resilience.hypothesis.md` |
| C8 | Calibration | `PostgreSQL connection pooling shard planner` | Stable no expected match | all | none |
| H1 | Hold-out | `which EV battery chemistries avoid nickel and cobalt` | Cross-page material exposure query | all | `wiki/references/lfp-lmfp-phosphate-lithium-ion.reference.md`, `wiki/references/lithium-sulfur-batteries.reference.md`, `wiki/specs/ev-battery-technology-landscape.spec.md` |
| H2 | Hold-out | `Toyota 2027 2028 all solid state battery launch window` | Exact timeline retrieval | all | `wiki/references/all-solid-state-lithium-metal-batteries.reference.md`, `wiki/roadmaps/battery-commercialization-timeline.roadmap.md` |
| H3 | Hold-out | `lithium sulfur Stellantis Zeta vehicle use by 2030` | Mixed company/technology retrieval | all | `wiki/references/lithium-sulfur-batteries.reference.md`, `wiki/roadmaps/battery-commercialization-timeline.roadmap.md` |
| H4 | Hold-out | `anode free lithium metal dendrite first cycle loss fast charge safety` | Exact technical constraint retrieval | all | `wiki/references/lithium-metal-anode-free-batteries.reference.md` |
| H5 | Hold-out | `legacy battery chemistries not mainstream BEV traction packs` | Boundary technology retrieval | all | `wiki/references/legacy-niche-battery-chemistries.reference.md`, `wiki/specs/ev-battery-technology-landscape.spec.md` |
| H6 | Hold-out | `IEA LFP nearly half of global EV battery sales in 2024` | Market anchor retrieval | all | `wiki/literature/iea-global-ev-outlook-2025-batteries.literature-note.md`, `wiki/specs/ev-battery-technology-landscape.spec.md` |
| H7 | Hold-out | `LMFP raises voltage and energy density while preserving LFP advantages` | Exact chemistry variant retrieval | all | `wiki/references/lfp-lmfp-phosphate-lithium-ion.reference.md` |
| H8 | Hold-out | `solid state may affect expectations before mainstream used prices` | Hypothesis-specific retrieval | all | `wiki/hypotheses/solid-state-residual-value-impact.hypothesis.md`, `wiki/hypotheses/battery-technology-residual-value-impact.hypothesis.md` |
| H9 | Hold-out | `battery commercialization sequence through 2035` | Roadmap overview retrieval | all | `wiki/roadmaps/battery-commercialization-timeline.roadmap.md` |
| H10 | Hold-out | `browser automation screenshot plugin` | Stable no expected match | all | none |
| H11 | Hold-out | `Kubernetes ingress controller TLS certificate renewal` | Stable no expected match | all | none |
| H12 | Hold-out | `what evidence would falsify sodium ion low cost EV impact` | Hypothesis falsification retrieval | all | `wiki/hypotheses/sodium-ion-low-cost-ev-impact.hypothesis.md` |

<!-- llm-wiki-search-ignore-end -->
