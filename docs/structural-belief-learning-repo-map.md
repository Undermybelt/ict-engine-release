# Structural Belief Learning Repo Map

Date: 2026-05-01
Status: patched_against_execution_plan
Scope: map literature mechanisms into concrete `ict-engine` belief surfaces, and mark what is already implemented vs still missing.

Aligned source docs:
- [2026-04-30-structural-belief-execution-plan.md](/Users/thrill3r/projects-ict-engine/ict-engine/docs/plans/2026-04-30-structural-belief-execution-plan.md:1)
- [20260501repo.md](/Users/thrill3r/projects-ict-engine/ict-engine/docs/plans/20260501repo.md:1)
- [2026-05-02-catboost-path-ranking-target-design.md](/Users/thrill3r/projects-ict-engine/ict-engine/docs/plans/2026-05-02-catboost-path-ranking-target-design.md:1)

## Status Key

- `已实现`: versioned in repo and already used by production flows
- `部分实现`: repo has real code and tests, but the literature mechanism is only partially landed
- `未实现`: only planned / documented, not yet in the canonical runtime path

## Execution-Plan Alignment

| Execution phase | Repo status | Notes |
|---|---|---|
| `P0` Repo truth | `已实现` | execution plan, literature docs, and paper-code readmes are committed |
| `P1` Canonical structural anchor | `已实现` | downstream phases no longer redefine canonical structural lineage |
| `P2` Live feedback posterior update | `基本实现` | delayed resolution, fractional pseudo-count updates, compliance/off-policy exposure fields, clipped IPS counterfactual reward priors, candidate-set policy logging, feedback-time selected policy probability consumption, clipped SNIPS/DR reward priors, SNIPS effective-sample diagnostics, compact target-policy variance/Brier/calibration-error diagnostics, delayed-reward resolution/censoring adjustment diagnostics, compact competing-risk outcome probabilities, elapsed-hour hazard diagnostics, compact event-time survival diagnostics, 1h/4h/24h resolution horizon probabilities, compact 4h cause-specific cumulative-incidence diagnostics, and compact online target-policy context posteriors with confidence-calibrated probability scalars exist; deeper learned target-policy and full delayed-reward competing-risk calibration remains |
| `P3` Offline evidence tempering | `部分实现` | source weighting, quality calibration, source panels, power-prior contribution objects, reusable source-reliability posteriors, compact outcome-confusion profiles, persisted EM source-confusion summaries, compact EM calibration diagnostics, and reliability-weighted panel aggregation exist |
| `P4` Structural prior state upgrade | `部分实现` | duration, transition, dwell/hazard fields, compact BOCPD run-length, sequence-change, and recursive sequence posterior diagnostics, source panels, event ledger, separated prior-mass snapshots, and latest offline seed snapshot exist; fitted dwell-time theory remains |
| `P5` BBN node/branch posterior update | `基本实现` | temporal priors adjust belief snapshots and branch surfaces, normalized outgoing branch and node-transition posterior state persists, and node/regime plus complete/partial candidate-set branch adjustment consume it directly |
| `P6` CatBoost path ranking target | `部分实现` | target surface contract, explicit row fields, workflow surface, persisted target-row export, empirical calibration utility, and calibration-quality evaluator exist; production validation on sufficient raw-scored rows is still not landed |

## Repo Targets

Allowed target surfaces:
- `structural_prior_state`
- `BBN node/branch posterior update`
- `CatBoost path ranking target`
- `artifact-validation prior source`
- `live feedback posterior update`

---

## 1. structural_prior_state

Status: `部分实现`

Primary papers
- `Sequential Bayesian Learning for Hidden Semi-Markov Models`
- `A sticky HDP-HMM with application to speaker diarization`
- `Using Bayesian Model Averaging to Calibrate Forecast Ensembles`

Already in repo
- persisted `LearningState.structural_prior_state`
- `source_panel_summaries`
- `last_offline_seed_source`
- `last_offline_seed_snapshot`
- `node_prior_mass`
- `branch_prior_mass`
- `scenario_prior_mass`
- `path_prior_mass`
- `event_ledger`
- `node_duration_priors`
- `branch_transition_priors`
- duration `expected_dwell_steps`, `remaining_dwell_steps`, empirical dwell distribution, fitted completion hazard, BOCPD evidence weight/raw break probability, compact run-length mode/probability/tail/mass diagnostics, compact one-step recursive run-length posterior diagnostics, sequence-change intensity, sequence break probability, compact recursive sequence run-length diagnostics, and `sticky_self_transition_strength`
- `structural-temporal-summary` exposes compact duration-distribution entropy, survival, completion-hazard, BOCPD evidence-weight, raw break, calibrated break/continue, empirical run-length, recursive run-length posterior, sequence-break, and sequence-reset diagnostics without dumping the full histogram
- panel-derived prior reconstruction before structural display / ranking surfaces

Literature mechanisms still worth importing
- full production-grade BOCPD changepoint filtering over richer sequence history and emissions beyond the current compact duration-surprise, empirical run-length, one-step recursive run-length, evidence-weighted break/continue probabilities, adjacent-streak sequence-change scalars, and compact recursive sequence posterior diagnostics
- source-panel posterior aggregation written as explicit panel likelihood / prior math, not only weighted summary blending
- clearer node-level prior mass separation from branch/path-level prior mass

Suggested state fields
- `node_prior_mass`
- `node_duration_prior`
- `branch_transition_prior`
- `source_panel_summaries`
- `last_offline_seed_snapshot`

Current repo gap
- `node_duration_priors` and `branch_transition_priors` are real; duration state now carries expected dwell, remaining dwell, empirical dwell distribution, completion hazard, BOCPD evidence weight, raw/calibrated break probability, compact empirical and one-step recursive run-length diagnostics, adjacent-streak sequence-change/break diagnostics, compact recursive sequence run-length diagnostics, break hazard, and sticky self-transition strength; `node_prior_mass` / `branch_prior_mass` / `scenario_prior_mass` / `path_prior_mass` keep entity-scaled prior mass auditable outside the generic stats maps
- `last_offline_seed_snapshot` is formalized as a persistent theory object for the latest offline seed, but deeper snapshot history / recalibration policy remains future work

Upgrade path
1. calibrate the compact BOCPD-style break, one-step recursive run-length, adjacent-streak sequence-change, and recursive sequence run-length diagnostics against richer sequence history once enough observations exist
2. treat source panels as pre-merge posterior contributors, not only audit surfaces

---

## 2. BBN node/branch posterior update

Status: `部分实现`

Primary papers
- `Dynamic Bayesian Networks: Representation, Inference and Learning`
- `A New Approach to the Economic Analysis of Nonstationary Time Series and the Business Cycle`
- `Online Learning of Order Flow and Market Impact with Bayesian Change-Point Detection Methods`

Already in repo
- canonical belief snapshot consumes structural priors
- node duration prior adjusts regime confidence in belief snapshot
- branch transition prior adjusts canonical regime probabilities
- branch temporal posterior state stores `transition_prior`, `posterior_multiplier`, and normalized outgoing `normalized_transition_posterior`
- node transition posterior state stores node-to-node transition prior, posterior multiplier, and normalized outgoing node posterior for compact Hamilton/DBN-style filtering
- `regime_posterior`, `belief_posteriors["market_regime"]`, `gate_decision`, `strategy_recommendation`, and selected market subgraph are synchronized after adjustment
- workflow snapshot and ensemble surfaces reuse canonical structural regime posteriors across phases
- `workflow-status` temporal summary exposes the maintained normalized branch and node transition posterior for consumer agents
- node/regime posterior adjustment prefers maintained node transition posterior state, then uses a bounded discounted recursive node-transition fallback when direct node evidence is missing, then falls back to branch-transition aggregation and finally unadjusted probabilities; complete/partial candidate-set branch posterior adjustment reads maintained branch transition posterior state and now uses bounded discounted recursive branch fallback when direct branch evidence is missing

Literature mechanisms still worth importing
- discounted transition-count updates:
  - `N_ij(t) = lambda * N_ij(t-1) + P(z_(t-1)=i, z_t=j | x_1:t)`
- Hamilton/DBN-style recursive node/branch posterior maintenance beyond the compact bounded discounted recursive transition surfaces
- BOCPD-style hazard handling for branch birth / node break
- moving branch posterior maintenance out of display-layer blending and into core belief-state updates

Suggested formulas
- discounted transition counts:
  - `N_ij(t) = lambda * N_ij(t-1) + P(z_(t-1)=i, z_t=j | x_1:t)`
- normalized branch posterior:
  - `pi_ij(t) = N_ij(t) / sum_j N_ij(t)`

Suggested implementation hooks
- `src/domain/belief/*`
- `src/application/belief/*`
- `src/state/*`

Current repo gap
- branch transition priors already affect branch prior/posterior surfaces and belief snapshots, maintained branch temporal posterior state carries normalized outgoing posterior mass, and maintained node transition posterior state now carries normalized outgoing node posterior mass
- node/regime posterior adjustment plus complete and partial candidate sets now consume maintained node/branch transition posterior state directly, including bounded discounted recursive node and branch fallback when direct evidence is absent; remaining work is deeper multi-step DBN/Hamilton filtering rather than display-layer fallback cleanup

---

## 3. artifact-validation prior source

Status: `部分实现`

Primary papers
- `Power Prior Distributions for Regression Models`
- `Maximum Likelihood Estimation of Observer Error-Rates Using the EM Algorithm`
- `Using Bayesian Model Averaging to Calibrate Forecast Ensembles`

Already in repo
- artifact validation feeds `structural_prior_state`
- source-specific weighting exists
- quality calibration exists
- validation regression can reduce effective contribution
- source panels preserve inspectable pre-merge evidence instead of only final aggregate prior
- source panels store the latest `StructuralPowerPriorContribution` with source rank, tempering coefficient, entity scale, effective tau, and weighted contribution masses
- `structural_prior_state.source_reliability_posteriors` stores reusable source-level reliability posteriors from offline seeds and live feedback
- source reliability posteriors preserve compact `observed_outcome -> credit_class` outcome-confusion cells with weighted success/failure mass
- outcome-confusion cells derive smoothed `P(observed_outcome | credit_class, source)` likelihoods for downstream source-reliability consumers
- panel-derived aggregate priors consume source-reliability posteriors and outcome-confusion likelihood concentration so low-reliability or diffuse high-mass panels shrink toward neutral instead of dominating by raw mass
- `structural-experience-priors` exposes compact Dawid-Skene / EM readiness counts from the structural prior event ledger: candidate items, multi-source overlap, distinct sources, observed labels, and readiness status
- `structural-experience-priors` also surfaces compact latent-label consensus diagnostics for cross-source items: consensus item count, conflict item count, average consensus confidence, and minimum consensus confidence
- `structural-experience-priors` runs a dependency-free fixed-iteration Dawid-Skene-style EM fit over multi-source event-ledger items, exposing latent item confidence and learned source-reliability summary diagnostics without dumping confusion matrices
- `structural_prior_state.source_reliability_em_summaries` persists the learned source-specific EM confusion matrices with source reliability and compact matrix cell counts
- `structural-experience-priors` exposes compact persisted EM summary counts and reliability ranges without dumping the full matrices
- `structural_prior_state.source_reliability_em_calibration` persists a compact leave-source-out calibration check over persisted EM matrices, including status, observation count, source count, Brier score, and log loss
- `structural-experience-priors` exposes those EM calibration diagnostics as scalar readiness/error fields
- panel-derived priors prefer persisted EM source reliability as a conservative multiplier, blending it with persisted source posterior reliability when both are available

Literature mechanisms still worth importing
- richer aggregate power-prior / tempered likelihood composition across source-panel contributions:
  - `posterior(theta) propto prior(theta) * product_s L_s(theta)^(tau_s)`
- validating persisted EM source-specific confusion matrices on larger real cross-source panels and out-of-sample windows
- clearer split between source rank, evidence quality, recency, and drift penalty

Suggested formula
- `posterior(theta) propto prior(theta) * product_s L_s(theta)^(tau_s)`

Suggested `tau_s` ingredients
- base source rank
- execution readiness
- aggregate return quality
- score delta stability
- mutation acceptance rate
- conformal coverage quality
- break penalty

Current repo gap
- source reliability now has compact outcome-confusion likelihood cells, cross-source EM-readiness diagnostics, latent-label consensus telemetry, fixed-iteration EM fit diagnostics, persisted source-specific EM confusion summaries, persisted EM calibration diagnostics, and persisted EM source-reliability consumption in panel-derived priors; remaining work is validation on larger real cross-source panels and out-of-sample windows

---

## 4. live feedback posterior update

Status: `部分实现`

Primary papers
- `The Beta-Binomial Bayesian Model`
- `Counterfactual Risk Minimization: Learning from Logged Bandit Feedback`
- `Modeling Delayed Feedback in Display Advertising`

Already in repo
- `structural-feedback-v1` round-trips through `update --feedback-file`
- structural lineage survives in `FeedbackRecord`, `UpdateRunRecord`, `WorkflowSnapshot.latest_update`
- feedback updates persisted `structural_prior_state`
- explicit success/failure mass exists
- invalidated / abandoned / breakeven outcomes no longer look like pure wins
- not-followed feedback updates weighted exposure mass, weighted not-followed mass, execution propensity, and an off-policy-adjusted prior without adding reward pseudo-counts
- structural history surfaces expose execution propensity and off-policy exposure rate for consumer agents
- persisted structural stats and `structural-experience-priors` expose clipped IPS weight and counterfactual reward prior
- recommended path bundles log candidate-set id, candidate-set size, and selected path behavior-policy probability for later DR/SNIPS correction
- structural feedback templates and inline execution contracts carry the logged candidate-set policy context without adding required flags
- structural feedback submissions consume `selected_path_probability` as the recorded selected behavior-policy probability before legacy posterior fallbacks
- structural prior stats/source summaries persist behavior-policy probability, probability variance, probability confidence/lower-bound diagnostics, compact target-policy probability Brier score and absolute calibration error, SNIPS reward prior, SNIPS effective sample size, doubly robust reward prior, target-policy calibration weight, variance penalty, calibrated reward prior, conservative reward lower bound, delayed-reward resolution/censoring probabilities, censoring-adjusted reward prior/lower-bound diagnostics, compact delayed-reward competing-risk probabilities, elapsed-hour hazard diagnostics, event-time resolution hazard / expected-resolution / fixed-horizon survival diagnostics, 1h/4h/24h resolution horizon probabilities, and compact 4h cause-specific cumulative-incidence diagnostics; `structural-experience-priors` exposes the compact correction fields
- `StructuralPriorLearningState.target_policy_context_posteriors` now persists compact online context-keyed target-policy probability posteriors from resolved structural feedback only, including confidence-calibrated probability and lower-bound scalars that blend learned context probability with logged behavior probability; `structural-experience-priors` caps that surface to the highest-support context summaries
- `structural-experience-priors` now exposes compact maturity/censoring diagnostics from existing counters: matured feedback count, unresolved feedback count, maturity coverage, censoring rate, delayed-reward resolution/censoring probability, censoring-adjusted reward prior/lower bound, success/failure/invalidation/abandonment competing-risk probability, competing-risk entropy, elapsed feedback count, elapsed hours at risk, average elapsed hours, resolution hazard, expected resolution hours, fixed-horizon survival probabilities, per-hour outcome hazards, compact 1h/4h/24h delayed-resolution horizon support/probability fields, and compact success/failure/invalidation/abandonment 4h cumulative-incidence fields

Literature mechanisms still worth importing
- deeper learned/contextual target-policy probability calibration beyond the current compact logged-probability variance/confidence, Brier/calibration-error, ESS-weighted reward prior diagnostics, online context posterior, and confidence-calibrated context probability scalars
- full elapsed-time competing-risk delayed-reward model beyond compact resolved/unresolved counters, smoothed resolution/censoring probabilities, censoring-adjusted reward blending, counter-derived competing-risk probabilities, elapsed-hour hazards, fixed-horizon survival, fixed-horizon resolution CDF diagnostics, and fixed-horizon cause-specific cumulative incidence
- compliance / propensity correction before updating execution value

Suggested formulas
- path success posterior:
  - `p_k ~ Beta(alpha_k, beta_k)`
  - `alpha_k += w * q * c * reward_credit`
  - `beta_k += w * q * c * loss_credit`
- optional OPE correction:
  - `V_DR = q(x, pi_e) + [pi_e(a|x)/pi_b(a|x)] * (r - q(x,a))`

Suggested outcome decomposition
- realized return component
- execution readiness realized component
- user compliance component
- invalidation cleanliness component

Current repo gap
- feedback learning is real and no longer heuristic-only; delayed outcomes now resolve into one posterior event and abandoned/invalidated outcomes use explicit pseudo-count weights
- not-followed recommendations now carry separate propensity/off-policy exposure fields and clipped IPS counterfactual reward priors; recommended path bundles, feedback templates, and execution contracts log behavior-policy probability; submitted feedback consumes that probability and structural stats expose compact probability variance/confidence/lower-bound/Brier/calibration-error diagnostics, clipped SNIPS/DR reward priors, ESS-weighted target-policy reward, variance diagnostics, compact maturity/censoring coverage, smoothed delayed-reward resolution/censoring probabilities, censoring-adjusted reward prior/lower-bound diagnostics, counter-derived competing-risk probabilities, elapsed-hour hazards, fixed-horizon survival diagnostics, fixed-horizon resolution CDF diagnostics, compact 4h cause-specific cumulative-incidence diagnostics, and compact online context-keyed target-policy probability posteriors with confidence-calibrated probability scalars; deeper target-policy probability modeling and full delayed-reward competing-risk models remain

---

## 5. CatBoost path ranking target

Status: `部分实现`

Primary papers
- `Adaptive Conformal Inference Under Distribution Shift`
- `Modeling Delayed Feedback in Display Advertising`
- `Counterfactual Risk Minimization: Learning from Logged Bandit Feedback`

Already in repo
- structural candidate contract exists for `node / branch / scenario / path`
- path / branch / scenario / node history surfaces exist
- recommended path bundle and top-path candidate surfaces already exist for consumption
- `structural-path-ranking-target` workflow surface exists and reuses the declared structural candidate set
- target rows expose `raw_path_score`, `calibrated_path_prob`, `path_prob_lower_bound`, lower-bound execution-gate fields, `pending_reward_state`, `maturity_mask`, `maturity_weight`, `calibrated_label`, `propensity_estimate`, `ips_weight`, `training_weight`, `regime_calibration_bucket`, and compact target-policy confidence/lower-bound/reward-prior diagnostics for external trainers
- target rows export to `policy_training/structural_path_ranking_target.csv` and `.jsonl` with a summary file during the normal update flow
- `export-structural-path-ranking-target` provides the same export contract on demand from persisted workflow state, which is useful for isolated collection without relying on update-side effects
- `policy-training-status` reports structural path-ranking export readiness, mature-row availability, and calibration readiness without requiring a new CLI flag
- empirical Beta-smoothed calibration writes `calibrated_path_prob` and `path_prob_lower_bound` only when a regime bucket has raw-scored mature outcome observations
- `policy-training-status` evaluates exported mature calibrated rows with compact Brier / calibration-error fields when enough rows exist
- `policy-training-status` also reports a clipped-IPS propensity-weighted Brier score for calibrated mature rows when `propensity_estimate` is available
- `policy-training-status` also reports raw-scored mature-row sufficiency and shortfall separately from propensity-weighted production-validation rows, so missing score history versus missing calibrated propensity coverage stay distinguishable
- `policy-training-status` summary lines now include the path-ranking readiness shortfalls directly, so consumer agents do not need to inspect nested fields to see the blocking counts
- `policy-training-status` now derives its history-side score/calibration/propensity/training-weight counters from the accumulated dataset itself when present, so top-line readiness and nested counters no longer mix snapshot-only and history-backed bases
- path-ranking export now maintains an upserted history JSONL alongside the latest snapshot export, and `policy-training-status` evaluates production-validation readiness from that accumulated history instead of a single latest candidate set
- path-ranking export also writes a matching accumulated history CSV, so external trainers can consume the same accumulated dataset without translating the JSONL stream
- calibrated target rows include advisory execution-gate status fields from `path_prob_lower_bound` and a fixed repo threshold, without blocking zero-config uncalibrated flows
- target rows now carry mature reward labels plus clipped IPS/sample weights so a downstream ranker can train without treating censored rows as negatives
- `policy-training-status` separates calibration-quality readiness from production-validation readiness by requiring enough propensity-weighted calibrated rows before declaring the target production-validatable
- the target summary carries an external trainer manifest describing group, label, weight, target-policy feature, calibration, and guardrail columns without adding a CatBoost runtime dependency
- `policy-training-status` surfaces trainer manifest readiness with protocol/dataset role plus compact feature/calibration/guardrail column counts, and warns on incomplete manifests without loading an external trainer
- `policy-training-status` also recognizes an optional `policy_training/structural_path_ranking_trainer_artifact.json` handoff file, reporting artifact readiness, protocol/dataset role, model family, score column, trained/calibration row counts, feature-column count, and URI presence without dumping a user-specific artifact URI
- `register-structural-path-ranking-trainer-artifact` lets a user explicitly register an external artifact URI into that handoff file without auto-loading personal paths or requiring manual JSON edits
- registered external trainer artifacts now default their trained/calibration row counts from accumulated history when that history exists, so the handoff metadata matches the dataset that `policy-training-status` validates
- `clear-structural-path-ranking-trainer-artifact` lets a user explicitly remove that personal artifact wiring and return the status surface to repo-default behavior
- externally applied raw scores now persist through later target re-exports when the candidate-set/path key still matches, so users do not need to reapply the same scores after every export
- `policy-training-status` now distinguishes pending update templates, legacy feedback rows without structural refs, and applied structural feedback history, which makes “no mature rows yet” easier to diagnose on real local state
- `apply-structural-path-ranking-external-scores` lets a user explicitly merge external raw scores back into the latest export and accumulated history datasets, after which repo-native calibration/gating surfaces recompute from the updated rows

Not yet in repo
- propensity-aware model training loop
- trained-ranker production calibration validation over real exported raw-scored rows

Suggested target stack
1. train raw ranking score on realized / corrected outcomes
2. debias eval under partial execution
3. apply probability calibration
4. maintain regime-conditional conformal coverage

Suggested fields
- `raw_path_score`
- `calibrated_path_prob`
- `path_prob_lower_bound`
- lower-bound execution-gate fields
- `pending_reward_state`
- `maturity_mask`
- `maturity_weight`
- `calibrated_label`
- `propensity_estimate`
- `ips_weight`
- `training_weight`
- `regime_calibration_bucket`

Current repo gap
- CatBoost training and production calibration validation are still downstream work; current path ranking surfaces are structural-orchestration target rows, empirical calibration/evaluation utilities, and an explicit external artifact registration/status boundary, not a learned calibrated ranker runtime
- the P6 target design is versioned in `docs/plans/2026-05-02-catboost-path-ranking-target-design.md`; next implementation needs a real trained external ranker artifact/service and enough real exported raw-scored rows, not more candidate-surface expansion

---

## Recommended implementation order

### Phase 1
- `artifact-validation prior source`
- `live feedback posterior update`

Reason
- the repo already has working source weighting and feedback learning; the next gain is to make the math explicit and versionable

### Phase 2
- `structural_prior_state`
- `BBN node/branch posterior update`

Reason
- duration / transition state now exists; the next step is to move from persistence + surface adjustment into proper transition-count-driven posterior maintenance

### Phase 3
- `CatBoost path ranking target`

Reason
- ranking calibration should consume trustworthy structural candidates and posterior state, not race ahead of them

---

## Minimal Viable Theory Bundle

If only 5 mechanisms are implemented next, use these:
1. HSMM node duration prior
2. discounted DBN branch transition update
3. tempered offline pseudo-likelihood by source
4. Beta-Binomial fractional pseudo-count path outcome update
5. calibrated path probability + lower bound for execution gating

---

## Practical Reading Of Current Repo State

Use this summary when deciding the next coding slice:

- `canonical structural anchor`: `已实现`
- `canonical structural posterior propagation across phases`: `已实现`
- `offline source weighting + quality calibration`: `基本实现`
- `live structural feedback learning`: `基本实现`
- `source-panel inspectability`: `已实现`
- `duration / transition persistence`: `已实现`
- `duration / transition as core BBN transition engine`: `部分实现`
- `CatBoost calibrated path target`: `部分实现`

The repo is no longer blocked on surface drift. The highest-value remaining work is now:
1. collect or opt into larger real cross-source panels, then inspect the persisted Dawid-Skene / EM-style calibration diagnostics over out-of-sample windows
2. richer BOCPD posterior calibration on top of the current HSMM-style empirical dwell distribution and compact evidence-weighted break/continue plus empirical/recursive run-length, adjacent-streak sequence, and recursive sequence-posterior telemetry
3. deeper target-policy probability calibration and delayed-reward competing-risk modeling beyond the current clipped IPS / SNIPS / DR, ESS-weighted reward, variance, Brier/calibration-error, compact context posterior, context-confidence-calibrated probability scalars, compact censoring-adjusted diagnostics, counter-derived competing-risk probabilities, elapsed-hour hazards, fixed-horizon survival diagnostics, fixed-horizon resolution CDF diagnostics, and compact 4h cause-specific cumulative-incidence diagnostics
4. CatBoost training and production validation on top of exported P6 target rows once raw-scored history exists
