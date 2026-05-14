# Structural Belief Execution Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** stabilize canonical market-structure anchors, land the literature artifacts as repo truth, and then execute structural belief learning in the order required for `BBN -> structural prior/posterior -> CatBoost path ranking`.

**Architecture:** keep the public `node -> branch -> scenario -> path` contract and the existing zero-config / opt-in-profile boundary, but stop letting downstream workflow phases overwrite canonical market-structure lineage. Upgrade the current heuristic bootstrap into principled belief learning in three layers: evidence math, structural state transition math, and final path-ranking calibration.

**Tech Stack:** Rust, `serde_json`, existing `workflow-status` / `update` / `analyze` / `factor-research` / `factor-backtest` pipelines, persisted `LearningState.structural_prior_state`, repo docs under `support/docs/`.

---

## Current State

Already true in repo:
- structural consumer contract exists for `node / branch / scenario / path`
- `structural-feedback-v1` can round-trip through `update --feedback-file`
- `FeedbackRecord`, `UpdateRunRecord`, and `WorkflowSnapshot.latest_update` carry structural lineage
- canonical structural posterior is preserved across:
  - `analyze`
  - `research`
  - `backtest`
  - `update`
  - workflow snapshot / ensemble / structural consumer surfaces
- `LearningState.structural_prior_state` exists and is fed by:
  - live structural feedback
  - analyze
  - research
  - backtest
  - mutation
  - artifact validation
- offline prior seeding already has:
  - source weighting
  - quality calibration
  - explicit `tempering_coefficient`
  - source-panel summaries before merge
- live feedback learning already has:
  - raw outcome preservation
  - unresolved / no-credit handling
  - fractional factor-credit semantics
- temporal state already includes:
  - `node_duration_priors`
  - `branch_transition_priors`
  - `node_temporal_posteriors`
  - `branch_temporal_posteriors`
  - persisted temporal maintenance weights
- belief packet evidence and workflow structural surfaces already expose temporal support / maintenance state

Current blocker:
- anchor leak is no longer the main blocker
- current blocker is deeper:
  - temporal support and discounted transition logic are still applied mostly as maintained support plus snapshot-time reweighting, not yet a single core `BBN` transition engine rule
  - offline source tempering is explicit enough to work, but not yet expressed as a fully formal power-prior-style posterior object
  - `CatBoost` target design has not started

Repo source-of-truth inputs for this plan:
- [2026-04-29-structural-playbook-belief-architecture-plan.md](/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/plans/2026-04-29-structural-playbook-belief-architecture-plan.md:1)
- [2026-04-30-structural-belief-literature-ingestion-plan.md](/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/plans/2026-04-30-structural-belief-literature-ingestion-plan.md:1)
- [structural-belief-learning-literature.md](/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/structural-belief-learning-literature.md:1)
- [structural-belief-learning-repo-map.md](/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/structural-belief-learning-repo-map.md:1)
- [support/docs/paper-code/structural_belief_learning/README.md](/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/paper-code/structural_belief_learning/README.md:1)
- [support/docs/paper-code/bayesian_nonparametric_hidden_semi_markov_models/README.md](/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/paper-code/bayesian_nonparametric_hidden_semi_markov_models/README.md:1)
- [support/docs/paper-code/self_calibrating_conformal_prediction/README.md](/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/paper-code/self_calibrating_conformal_prediction/README.md:1)

## Acceptance Gates

The plan is only considered complete when all of these are true:

1. `workflow-status --phase structural-node` returns canonical node families and labels that describe market structure, not workflow phase or support-reason leakage.
2. Downstream `research` / `backtest` / `update` runs can add evidence without mutating the analyze-time canonical node anchor.
3. Literature docs and paper-code readmes are committed as repo truth.
4. Live feedback posterior updates use explicit pseudo-count logic instead of only heuristic score blending.
5. Offline evidence seeding uses explicit tempered source contribution logic instead of only ad hoc support mixing.
6. `structural_prior_state` carries enough state to support duration and transition learning.
7. CatBoost path ranking is designed as a consumer of structural candidates, not a generator of hidden structure.

## File Map

Primary code surfaces for execution:
- Modify: `src/application/orchestration/structural_playbook.rs`
- Modify: `src/application/orchestration/workflow_status.rs`
- Modify: `src/main.rs`
- Modify: `src/state/types.rs`
- Modify: `src/factors/weight_updater.rs`
- Modify: `src/analyze_shared.rs`
- Modify: `src/factor_research_runtime.rs`
- Modify: `src/factor_backtest_runtime.rs`

Primary doc surfaces:
- Add/commit: `support/docs/plans/2026-04-30-structural-belief-literature-ingestion-plan.md`
- Add/commit: `support/docs/structural-belief-learning-literature.md`
- Add/commit: `support/docs/structural-belief-learning-repo-map.md`
- Add/commit: `support/docs/paper-code/structural_belief_learning/README.md`
- Add/commit: `support/docs/paper-code/bayesian_nonparametric_hidden_semi_markov_models/README.md`
- Add/commit: `support/docs/paper-code/self_calibrating_conformal_prediction/README.md`

Tests likely touched:
- `src/application/orchestration/workflow_status.rs` unit tests
- `src/main.rs` workflow snapshot / integration tests
- `src/state/types.rs` structural prior learning tests
- targeted command/integration tests around `analyze`, `factor-research`, `factor-backtest`, `update`

## Progress Snapshot

| Phase | Status | Notes |
|---|---|---|
| `P0` Repo truth | `done` | execution plan, literature docs, paper-code readmes, and patched repo-map are committed |
| `P1` Canonical structural anchor | `mostly done` | canonical anchor and cross-phase propagation are landed; a dedicated downstream smoke path is still worth keeping explicit |
| `P2` Live feedback posterior update | `mostly done` | delayed resolution, fractional pseudo-counts, compliance/off-policy exposure, clipped IPS counterfactual reward priors, candidate-set policy logging, feedback-time selected policy probability consumption, and clipped SNIPS/DR reward priors landed; deeper target-policy calibration remains |
| `P3` Offline evidence tempering | `mostly done` | source weighting, quality calibration, source panels, power-prior contribution objects, reusable source-reliability posteriors, compact outcome-confusion cells, and reliability-weighted panel aggregation landed |
| `P4` Structural prior state upgrade | `partial` | duration / transition / dwell-hazard fields / source panels / event ledger / temporal posterior state, separated prior-mass snapshots, and latest offline seed snapshot landed; fitted dwell-time theory remains |
| `P5` BBN node/branch posterior update | `mostly done` | discounted temporal maintenance and normalized outgoing transition posterior state exist; node/regime and complete/partial candidate-set branch adjustment now consume maintained posterior state |
| `P6` CatBoost path ranking target | `partial` | target surface, persisted export, empirical calibration utility, and calibration-quality evaluator exist; model training and production validation still require enough raw-scored rows |

## Current Todo Board

### Done

- [x] Commit literature ingestion docs, paper-code readmes, and patched repo-map as repo truth.
- [x] Preserve canonical structural anchor across `analyze / research / backtest / update`.
- [x] Propagate canonical structural posterior through workflow snapshot, ensemble surfaces, workflow-status, structural-playbook, and prompts.
- [x] Preserve raw structural outcomes instead of flattening everything at update ingress.
- [x] Split `unresolved / not_followed / abandoned / invalidated / breakeven` into explicit learning semantics.
- [x] Land fractional factor-credit semantics for structural feedback.
- [x] Feed offline evidence into `structural_prior_state` with source weighting, quality calibration, source panels, and explicit `tempering_coefficient`.
- [x] Persist `node_duration_priors`, `branch_transition_priors`, `node_temporal_posteriors`, and `branch_temporal_posteriors`.
- [x] Reuse persisted temporal posterior state across belief packet, workflow-status, and structural consumer surfaces.
- [x] Expose temporal support and maintenance weights as machine-readable consumer fields.
- [x] Surface not-followed feedback as execution-propensity / off-policy exposure evidence without changing reward posterior credit.
- [x] Add clipped IPS counterfactual reward prior and expose it on structural experience-prior surfaces.
- [x] Log candidate-set id, candidate-set size, and selected path behavior-policy probability on recommended path bundles.
- [x] Carry candidate-set policy context into structural feedback templates and execution contracts.
- [x] Consume submitted `selected_path_probability` as the feedback record's selected behavior-policy probability before legacy posterior fallbacks.
- [x] Persist clipped SNIPS and doubly robust reward priors from logged selected-path behavior-policy probability.
- [x] Store offline seed power-prior contribution objects in source panels.
- [x] Persist reusable source-reliability posteriors from offline seeds and live feedback.
- [x] Consume source-reliability posteriors in panel-derived prior aggregation.
- [x] Persist separated node / branch / scenario / path prior-mass snapshots and the latest offline seed snapshot object.
- [x] Persist normalized outgoing branch-transition posterior state and test repeated evidence without collapsing unrelated branches.
- [x] Make branch posterior adjustment consume complete normalized transition posterior state when available.

### Next

- [x] Add a dedicated smoke path proving `analyze -> research -> backtest -> structural-playbook` preserves canonical structural lineage end to end.
- [x] Finish remaining `P2` by adding DR/SNIPS-style correction after candidate-set policy logging exists.
- [x] Finish `P4` node-mass vs branch/path-mass separation and persist a clearer offline seed snapshot object.
- [x] Finish `P5` by moving richer node transition handling toward a more central maintained `BBN` posterior state instead of mostly snapshot-time reweighting.

### Not Yet

- [x] Add repeated-evidence tests proving one branch can strengthen without collapsing unrelated nodes.
- [ ] Define `CatBoost` path-ranking target math after `P1-P5` are stable.
- [ ] Add explicit target fields:
  - `raw_path_score`
  - `calibrated_path_prob`
  - `path_prob_lower_bound`
  - `pending_reward_state`
  - `propensity_estimate`
  - `regime_calibration_bucket`
- [ ] Keep `CatBoost` inside declared structural candidate sets only.

## Phases

### Phase 0: Repo Truth First

Purpose:
- turn the literature and repo-map docs into committed inputs before more code drift

Outcome:
- planning docs become canonical repo truth
- execution order is frozen before more implementation

### Phase 1: Structural Anchor Repair

Purpose:
- preserve an analyze-time canonical market-structure anchor
- prevent `research` / `backtest` / `support_reason` strings from becoming structural node identity

Outcome:
- `node_id`, `branch_id`, `scenario_id`, `path_id` remain structurally meaningful
- downstream runs enrich evidence, not ontology

### Phase 2: Principled Evidence Math

Purpose:
- replace remaining heuristic mixing with explicit formulas already selected in the literature docs

Outcome:
- offline evidence enters via tempered source contribution
- live feedback enters via explicit fractional pseudo-count posterior updates

### Phase 3: Structural State Math

Purpose:
- enrich `structural_prior_state` from “smoothed score bucket” into a real structural-state carrier

Outcome:
- node duration prior
- branch transition prior
- source-panel snapshots before canonical merge

### Phase 4: BBN Update Upgrade

Purpose:
- move node/branch posterior updates toward discounted dynamic Bayesian transition logic

Outcome:
- branch posterior updates stop being display-only summaries
- `BBN` becomes the actual structural transition engine

### Phase 5: CatBoost Target Design

Purpose:
- define the delayed-feedback, partial-compliance, calibrated path-ranking target after the structural state is trustworthy

Outcome:
- CatBoost consumes declared path candidates
- calibrated path probability and lower bound become gating surfaces

## Ordered TODO

### P0: Planning and Repo Truth

- [x] Commit the literature ingestion artifacts listed above so execution references versioned docs instead of chat memory.
- [x] Keep this execution plan and the older architecture plan aligned; if the scope changes, update both in the same slice.
- [x] Freeze the execution order to `anchor -> evidence math -> structural state math -> BBN upgrade -> CatBoost target`.

### P1: Canonical Structural Anchor

- [x] Audit `workflow_phase_snapshot_from_research_run(...)` in `src/main.rs` so downstream workflow snapshots carry evidence and execution metadata without redefining structural ontology.
- [x] Audit `workflow_phase_snapshot_from_backtest_run(...)` in `src/main.rs` with the same rule.
- [x] Refactor `build_structural_node_artifact_with_prior_state(...)` in `src/application/orchestration/structural_playbook.rs` so node family and label are chosen from canonical market-structure anchors first, not generic workflow phase or support strings.
- [x] Add a persisted analyze-time structural anchor field if the current snapshot surface cannot preserve canonical structure across later phases.
- [x] Add regression tests for real-world dirty inputs such as:
  - `posterior_active_regime = research_iteration`
  - `posterior_probabilities = { fallback, research_iteration }`
  - `blocking_truth.reason = market_policy=...`
- [x] Add a smoke path that proves `workflow-status --phase structural-playbook --agent` returns canonical market-structure lineage after analyze + research + backtest.

### P2: Live Feedback Posterior Update

- [x] Replace score-only path posterior refresh with explicit Beta-Binomial-style fractional pseudo-count updates in `src/state/types.rs` and `src/factors/weight_updater.rs`.
- [x] Split executed, not-followed, abandoned, invalidated, and delayed outcomes into explicit update semantics instead of one blended credit signal.
- [x] Track weighted exposure mass, weighted not-followed mass, execution propensity, and off-policy-adjusted prior separately from reward pseudo-counts.
- [x] Expose not-followed/off-policy history rates on structural history surfaces.
- [x] Add clipped IPS counterfactual reward prior on persisted stats and `structural-experience-priors`.
- [x] Add candidate-set policy logging on `structural-recommended-path-bundle` for later DR/SNIPS correction.
- [x] Surface logged candidate-set policy context in the structural feedback template / execution contract.
- [x] Consume `selected_path_probability` from structural feedback submissions so recorded feedback carries the logged behavior-policy probability when present.
- [x] Add clipped SNIPS / doubly robust reward priors on structural stats and `structural-experience-priors`.
- [x] Preserve zero-config CLI behavior: all new math must run behind existing flows, not new required flags.
- [x] Add tests for:
  - followed profitable path
  - followed invalidated path
  - not-followed recommendation
  - delayed outcome that resolves later

### P3: Offline Evidence Tempering

- [ ] Replace remaining heuristic support blending in `src/analyze_shared.rs`, `src/factor_research_runtime.rs`, and `src/factor_backtest_runtime.rs` with explicit tempered source contribution rules from the literature repo-map.
- [x] Carry source-panel snapshots into `structural_prior_state` before canonical merge so offline evidence is inspectable instead of irreversibly blended.
- [x] Store source-panel `StructuralPowerPriorContribution` objects with base source weight, tempering coefficient, entity mass scale, effective tau, and contribution masses.
- [x] Store reusable `StructuralSourceReliabilityPosterior` objects beyond the latest source-panel contribution.
- [x] Preserve compact source outcome-confusion cells from live feedback and offline seeds for later Dawid-Skene-style reliability learning.
- [x] Use source-reliability posteriors when deriving aggregate source-panel priors.
- [x] Keep the current source ordering, but make the effect formula explicit and testable.
- [x] Add tests that prove:
  - stronger source + good quality increases prior mass more than weaker source
  - break penalties and poor coverage reduce effective contribution
  - validation regression can reduce contribution rather than only cap it

### P4: Structural Prior State Upgrade

- [x] Extend `LearningState.structural_prior_state` in `src/state/types.rs` with explicit node duration and branch transition fields suggested by the repo map.
- [x] Separate node prior mass from branch/path prior mass so one noisy path does not mutate the whole node too aggressively.
- [x] Store last offline seed snapshots and per-source summaries for later audit and recalibration.
- [x] Surface duration expected dwell, remaining dwell, break hazard, and sticky self-transition strength through maintained temporal state.
- [x] Add tests that prove duration and transition state survives persistence and is reused by structural orchestration.

### P5: BBN Node/Branch Posterior Update

- [x] Identify the existing `BBN` update surfaces under `src/domain/belief/*` and `src/application/belief/*` that should consume discounted transition counts.
- [x] Introduce discounted transition-count updates for branch posterior maintenance.
- [x] Surface maintained node and branch temporal posterior state, including normalized transition posterior, through `workflow-status` temporal summary.
- [x] Make complete-candidate branch posterior adjustment read the maintained normalized transition posterior instead of reconstructing it from multipliers.
- [x] Collapse partial-candidate branch fallback onto the maintained normalized transition posterior state.
- [x] Collapse richer node transition handling onto the maintained `BBN` posterior state.
- [x] Add tests that prove repeated evidence can strengthen one branch posterior without collapsing unrelated nodes.

### P6: CatBoost Path Ranking Target

- [x] Define the training/eval target surface for path ranking only after P1-P5 are stable.
- [x] Add explicit fields for:
  - `raw_path_score`
  - `calibrated_path_prob`
  - `path_prob_lower_bound`
  - `pending_reward_state`
  - `propensity_estimate`
  - `regime_calibration_bucket`
- [x] Keep CatBoost inside the declared structural candidate set; it must not invent hidden nodes or branches.
- [x] Add a design-only doc slice before implementation if the target math exceeds one coding session: `support/docs/plans/2026-05-02-catboost-path-ranking-target-design.md`.
- [x] Add an empirical calibration utility that writes `calibrated_path_prob` and `path_prob_lower_bound` when raw-scored mature observations exist.
- [x] Add a training/export path for the target artifact once row semantics are stable.
- [x] Add a calibration-quality evaluator for exported raw-scored calibrated rows.
- [ ] Validate production calibration quality after enough exported raw-scored rows exist.

## Immediate Execution Backlog

Do these first, in order:

1. [x] Finish remaining `P2` by adding DR/SNIPS-style correction after candidate-set policy logging exists.
2. [x] Finish `P5` by moving richer node transition handling from mostly snapshot-time reweighting into a more central maintained `BBN` node/branch posterior state.
3. [x] Start `P6` CatBoost path-target design.
4. [x] Implement the P6 target artifact and workflow surface from `support/docs/plans/2026-05-02-catboost-path-ranking-target-design.md`.
5. [x] Implement persisted target-row export after the target rows have survived workflow verification.
6. [x] Implement empirical calibration from raw-scored mature observations.
7. [x] Add production calibration-quality evaluation plumbing for exported raw-scored rows.
8. [ ] Run production calibration validation once enough exported raw-scored rows exist.

## Out of Scope For The Next Execution Slice

- no new provider defaults
- no new required CLI flags
- no consumer-visible ontology expansion beyond current `node / branch / scenario / path`
- no full paper reproduction bundles
- no CatBoost training changes before the structural anchor and posterior math are stable

## Verification Checklist For Future Execution

- [ ] `cargo test --lib workflow_status_phase_structural_ -- --nocapture`
- [ ] `cargo test --bin ict-engine test_analyze_command_persists_analyze_run -- --nocapture`
- [ ] `cargo test --bin ict-engine test_run_factor_research_persists_rankings_and_run_record -- --nocapture`
- [ ] `cargo test --bin ict-engine test_run_factor_backtest_persists_backtest_run_and_agent_bundle -- --nocapture`
- [ ] `cargo test --bin ict-engine test_update_command_accepts_structural_feedback_file -- --nocapture`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] one real smoke run proving canonical structural lineage survives downstream phases

## Handoff

Plan intent:
- stop surface drift
- repair the structural anchor first
- then execute the literature-backed belief upgrade in the order already documented

Execution should not start until the operator explicitly chooses the next slice from the ordered backlog above.
