# Research Summary: EV Battery Technologies Today and Future

## Scope

This bundle maps battery technologies relevant to electric cars in 2026 and plausible future technologies through roughly 2030-2035. It separates commercial traction-battery chemistries from emerging technologies, and flags technologies that are important for hybrids, 12 V systems, grid storage, or aviation but not likely to become mainstream car traction batteries.

## Source Set Reviewed

The research uses IEA Global EV Outlook 2024 and 2025 as the market anchor, government/lab sources from DOE, NHTSA, and Argonne as technical anchors, and primary company releases from CATL, Toyota, BMW, QuantumScape, Stellantis, Factorial, GM, LG Energy Solution, Group14, Panasonic, Lyten, and Zeta for commercialization claims.

## Key Findings

Lithium-ion remains the core EV battery platform. The commercially decisive split today is between lower-cost LFP/phosphate systems and higher-energy NMC/NCA/NMCA systems. IEA reports that LFP was nearly half of global EV battery sales in 2024, with strong China concentration, while NMC and related high-nickel chemistries remain more common in the United States and Europe.

LFP is no longer just a low-end chemistry. Cost, long cycle life, safety, and 100% usable charge behavior make it highly competitive for mainstream EVs. Its weaknesses are lower gravimetric and volumetric energy density and weaker cold-weather performance than high-nickel chemistries.

NMC, NCA, and NMCA retain value for long-range, high-performance, cold-weather, and packaging-constrained vehicles. Their strategic problem is exposure to nickel and cobalt cost, emissions, and supply-chain risk. They will likely keep a role in premium and long-range vehicles rather than disappear.

Manganese-rich chemistries are becoming a serious bridge category. GM and LG Energy Solution plan LMR commercial production in the United States by 2028, claiming LMR can deliver much higher energy density than LFP at comparable cost for trucks and large SUVs.

Sodium-ion is real but likely use-case specific. CATL announced a mass-producible sodium-ion product family in 2025, and IEA treats sodium-ion as a potentially important hedge against lithium price spikes and cold-weather limitations. Its lower energy density makes it most plausible for short-range, low-cost, cold-climate, or hybrid dual-chemistry packs rather than long-range premium cars.

Silicon-enhanced lithium-ion is one of the most likely near-term improvements because it modifies the anode inside the existing lithium-ion manufacturing ecosystem. It can raise energy density and charging performance, but swelling, cycle life, and heat durability remain the central engineering constraints.

Semi-solid and quasi-solid batteries are the likely first "solid-state" products most consumers will see. They can use gel or low-liquid electrolytes to gain some safety and energy-density benefits while avoiding the full manufacturing leap of all-solid-state cells. NIO/WeLion and Factorial/Stellantis show commercial or demonstration activity, but cost and scale remain unresolved.

All-solid-state lithium-metal batteries are technically promising but still early for mass-market EVs. Toyota targets 2027-2028 market launch, BMW is testing Solid Power cells in an i7, and QuantumScape shipped B1 QSE-5 samples in 2025. IEA classifies solid-state around large pilot stage and expects initial volumes to be limited.

Lithium-sulfur has high theoretical appeal because sulfur is cheap and abundant and the chemistry can avoid nickel, cobalt, manganese, and graphite. Stellantis has agreements with Lyten and Zeta, with Zeta-targeted vehicle use by 2030. The central uncertainty is whether cycle life, volumetric energy density, manufacturability, and automotive qualification can meet EV warranty needs.

Lithium-metal and anode-free liquid/gel electrolyte cells are a broader family that overlap with solid-state and semi-solid. They target high energy density by replacing graphite with lithium metal or forming lithium metal during charge. Dendrite control, first-cycle loss, fast-charge safety, and cycle life are the gating issues.

Legacy and niche chemistries still matter but mostly outside mainstream BEV traction packs. NiMH remains important in hybrids, lead-acid remains common for low-voltage auxiliary systems, LTO is useful for extreme power and long cycle life but low energy density, and metal-air/redox/iron-air are more relevant to grid or specialty applications than passenger EV traction.

## Disagreements and Caveats

Commercial timeline claims diverge sharply. Automakers and battery startups often announce aggressive dates, while IEA emphasizes that technologies like solid-state need pack-scale validation under realistic standardized conditions. "Commercialization" can mean anything from a concept car to a limited demo fleet to mass production.

The phrase solid-state is especially ambiguous. Some early products labeled solid-state may be semi-solid, quasi-solid, gel, or hybrid electrolyte systems. This matters because cost, safety, manufacturability, and residual-value impact will differ.

Battery technology alone does not determine used EV value. Pack size, charging curve, thermal management, warranty, replacement cost, brand support, software, and charging standard can matter as much as chemistry.

## Open Questions for Ingest

- Which chemistries should be treated as baseline "validated truth" versus hypotheses?
- Should depreciation analysis be organized by chemistry family, vehicle model, or market segment?
- How should the project handle China-only or limited-volume deployments when assessing likely global residual-value impact?
- What time horizon matters most for the user's decision: buying an EV now, holding value over 3-5 years, or structural market change by 2030?

## Ingest Readiness

Ready for ingest. Suggested wiki outputs:

- `wiki/references/ev-battery-technology-landscape.reference.md`
- `wiki/hypotheses/solid-state-residual-value-impact.hypothesis.md`
- `wiki/hypotheses/sodium-ion-low-cost-ev-impact.hypothesis.md`
- `wiki/hypotheses/lfp-mainstream-value-resilience.hypothesis.md`
- `wiki/literature/iea-global-ev-outlook-2025-batteries.literature-note.md`
