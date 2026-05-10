# 2026-05-08 Paper Search Outline And Matrix

## Purpose

- This document translates the current repo limitations, pain points, and known design direction into a paper-search outline.
- It is intentionally **not** a literature review.
- It is designed so a later agent or human can use the keywords directly for paper / repo / benchmark search.

## Search Principles

- Default to white-box, interpretable, explicit-artifact approaches.
- Prefer methods that can map into `offline trainer -> explicit artifact -> runtime read-only consume`.
- Prefer work that improves consumer usability, zero-config defaults, or low-pollution public surfaces.
- Prefer techniques that strengthen factor iteration, belief evidence quality, or execution-tree action selection without forcing repo-specific ontology into public CLI surfaces.

## Search Matrix

| Problem Area | Why It Matters Here | Search Goal | Primary Keywords | Secondary Keywords | Likely Output Shape To Look For |
|---|---|---|---|---|---|
| Zero-config historical data bootstrap | Current historical-data workflow still needs manual file-path selection | Find methods or systems for automatic dataset discovery / retrieval / normalization | zero-config market data bootstrap; historical dataset auto-discovery; symbol-to-dataset resolution | local market data cache; data catalog inference; self-bootstrapping data pipeline | dataset resolver, catalog scorer, retrieval planner, normalized artifact generator |
| Adaptive first-run routing | Empty-state workflow routing is still generic | Find ways to infer best next step from user intent, symbol family, and available artifacts | workflow intent inference; adaptive first-run routing; progressive disclosure CLI UX | intent classification for developer tools; workflow recommendation systems; self-describing next-step schema | intent router, next-step policy, route ranking model, CLI recommendation contract |
| Persisted profile/session context | Profile reuse still depends on explicit selector propagation | Find clean approaches to opt-in context persistence without polluting public defaults | profile/session context persistence; opt-in context propagation | workflow identity persistence; session-scoped configuration; optional runtime context | session context artifact, scoped config store, explicit profile ledger |
| Semantic symbol/profile matching | Current profile filtering is contract-label based, not semantic | Find better symbol-family matching without leaking repo ontology into user surfaces | semantic symbol matching; market family ontology mapping | asset alias resolution; ticker normalization; contract-based symbol intelligence | symbol embedding or taxonomy, alias matcher, family classifier |
| Provider capability negotiation | Provider surfaces explain missing tracks but do not resolve them automatically | Find approaches for runtime capability matching and fallback negotiation | provider capability negotiation; external runtime orchestration | capability discovery; runtime fallback planning; provider readiness modeling | capability graph, provider planner, fallback policy artifact |
| Auto-Quant handoff boundary | Current Auto-Quant path is still handoff-centric | Find better explicit artifact contracts for cross-runtime strategy iteration | artifact-first execution contracts; external runtime orchestration | handoff protocol design; inter-process research loop; agent-tool boundary | strategy handoff artifact, execution manifest, adoption/review protocol |
| Cold-start research bootstrap | Fresh state dirs have sparse research truth | Find warm-start priors or bootstrap methods that remain honest and explicit | warm-start research bootstrap; cold-start structural priors | prior elicitation; bootstrapped belief initialization; few-shot policy priors | prior artifact, bootstrap dataset, cold-start policy estimator |
| Structural path ranking calibration | Path ranking is central but still calibration-sensitive | Find methods for calibrated path scoring and explicit confidence bounds | structural path ranking calibration; conformal execution gating | calibrated ranking models; confidence-aware path selection; uncertainty-aware routing | calibrated ranker, lower-bound estimator, path confidence artifact |
| Policy correction / off-policy learning | Structural policy correction uses explicit weighting and reward correction | Find principled off-policy estimators that fit the repo’s artifact style | contextual bandit; off-policy evaluation; doubly robust estimation | SNIPS; inverse propensity scoring; policy calibration | policy correction report, calibration artifact, confidence interval surface |
| Delayed reward replay truth | Delayed reward and replay validation remain important evidence surfaces | Find robust delayed-outcome modeling for execution and belief updates | delayed reward replay validation; changepoint-aware duration modeling | competing risks; survival analysis; hazard calibration | replay validation artifact, duration posterior, hazard summary |
| HMM / regime continuity | HMM regime continuity is part of the repo’s truth model | Find interpretable regime models that complement explicit belief artifacts | HMM regime continuity; changepoint-aware duration modeling | semi-Markov model; regime persistence; BOCPD | regime posterior artifact, duration prior, transition prior |
| Multi-timeframe resonance filtering | Multi-timeframe resonance is a key decision surface | Find explicit, testable resonance/consistency models | multi-timeframe resonance filtering | hierarchical signal alignment; timeframe agreement scoring | resonance score artifact, timeframe alignment summary |
| Factor breadth and generalization | Factor work must generalize across markets and avoid low-trade overclaiming | Find research on breadth-first factor screening and family-level proof | factor family breadth screening; cross-market generalization; trade-density-aware search | robust factor validation; market transferability; sparse-trade rejection | factor family evaluation grid, transfer metric, trade-density gate |
| White-box factor generation | Repo direction prefers white-box factor evolution over opaque search | Find interpretable factor proposal or refinement methods | white-box factor generation; factor-to-evidence pipeline design | symbolic factor search; grammar-based alpha generation; explicit feature crafting | factor generator, factor grammar, explicit hypothesis artifact |
| Belief evidence composition | Repo routes factors into evidence, then belief, then ranking/execution | Find methods for explicit evidence weighting in belief networks | belief network evidence weighting; factor filtering before belief node evidence | evidential reasoning; probabilistic graphical model features; causal evidence scoring | evidence weighting rule set, node contribution artifact, evidence calibration |
| CatBoost path ranking | Repo explicitly uses CatBoost-style ranking paths | Find ranking/selection methods that fit explicit artifact output | CatBoost path ranking | LambdaMART; calibrated GBDT ranking; ranking under uncertainty | ranker artifact, feature importance report, ranked path manifest |
| Execution-tree arbitration | Final decision still resolves at execution-tree/action layer | Find explicit action arbitration methods after upstream evidence aggregation | execution-tree action arbitration | decision policy graphs; action gating; hierarchical controller | execution decision contract, action policy artifact, branch selector |

## Search Bundles

### Bundle A: User-Bootstrap / Consumer Experience

- zero-config market data bootstrap
- historical dataset auto-discovery
- workflow intent inference
- adaptive first-run routing
- progressive disclosure CLI UX
- provider capability negotiation
- semantic symbol matching

Use when:
- You want to improve first-run experience, reduce manual file-path friction, or make workflow routing smarter for users and user-agents.

### Bundle B: Research / Belief / Calibration

- cold-start structural priors
- delayed reward replay validation
- structural path ranking calibration
- contextual bandit
- off-policy evaluation
- doubly robust estimation
- belief network evidence weighting
- HMM regime continuity
- changepoint-aware duration modeling

Use when:
- You want to improve truth quality, calibration, or evidence reliability inside the belief and structural path layers.

### Bundle C: Factor Iteration / Execution

- white-box factor generation
- factor family breadth screening
- cross-market generalization
- trade-density-aware search
- factor-to-evidence pipeline design
- factor filtering before belief node evidence
- CatBoost path ranking
- execution-tree action arbitration

Use when:
- You want to improve factor iteration throughput, factor family quality, and downstream action quality without switching to black-box search.

## Search Questions

### Q1: Better Zero-Config Bootstrap

- How can a system infer or acquire the best historical dataset for a symbol without requiring a manual file path?
- How can the next-step router choose between replay, factor iteration, and live bootstrap from current repo state plus user intent?

### Q2: Better Prior / Calibration

- What explicit, interpretable methods exist for cold-start priors in action-selection or belief systems?
- What delayed-reward and survival-style methods fit explicit artifacts and replay validation better than naive outcome counting?

### Q3: Better Factor Pipeline

- What white-box methods exist for generating or refining factor candidates while preserving interpretability and cross-market generalization?
- What methods best filter candidate factors before they become evidence inside belief network nodes?

### Q4: Better Final Arbitration

- What ranking / action-selection approaches best fit “multiple explicit evidence surfaces -> calibrated path ranking -> execution-tree node selection”?
- What uncertainty-aware ranking methods preserve a clean public runtime contract?

## Exclusions

- Black-box end-to-end policy optimization with no explicit artifact boundary
- Methods that require repo-specific ontology leakage into public CLI surfaces
- Approaches that assume a fully managed cloud service or mandatory external platform dependence
- Papers that optimize only a single-symbol backtest without any transfer / breadth / evidence-calibration discussion

## Suggested Search Order

1. Bundle A if the next goal is user/consumer usability improvement
2. Bundle B if the next goal is belief / calibration / truth quality
3. Bundle C if the next goal is factor iteration and action quality
4. Then cross-link findings back into the repo using the existing limitation document:
   - `docs/audits/2026-05-08-whole-repo-limitations-and-keywords.md`
