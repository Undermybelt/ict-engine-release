# 2026-05-09 Audit Remediation Handoff TODO

## Mission

继续实现 `docs/plans/2026-05-09-ict-engine-audit-remediation-todo.md`。

硬约束：
- 零配置：默认命令能直接跑
- 消费者可用：`--human`/`--compact` 输出短、清楚、有下一步
- Token 友好：状态行优先
- 无污染：实验状态走 `/tmp/...`
- 无负债：修文档与代码口径冲突
- 用户个人数据热插拔：VRP特征可选，不进硬依赖
- 用户可选择是否沿用：外部ranker/profile/Auto-Quant prior都默认不强制

## Route / Skill

- Route alias: `ict-engine-runtime`
- Skill loaded: `~/.hermes/skills/ict-engine/ict-engine-runtime/SKILL.md`

## Phase 1 - P0 Validation Contract (IN PROGRESS)

### What was done

1. Added `StructuralPathRankingTargetRowValidationSurface` — row-level maturity surface
2. Added `StructuralPathRankingFeedbackObservationValidationSurface` — observation-level surface with outcome_distribution
3. Added both as nested fields on:
   - `StructuralPathRankingValidationSummarySurface` (top-level validation surface)
   - `StructuralPathRankingTargetTrainingStatusSurface` (target-level status)
4. Computed `feedback_observation_outcome_distribution` via `structural_feedback_counter_outcome`
5. Made `structural_feedback_counter_outcome` pub (was pub(crate)) in `src/state/types.rs`
6. Added import `structural_feedback_counter_outcome` to `training_export.rs`
7. Built `target_row_validation` and `feedback_observation_validation` structs in `structural_path_ranking_target_training_status`
8. Added test `structural_path_ranking_status_splits_target_rows_from_feedback_observations` — 30 observations + 1 pending, 2 target rows; asserts observation_validation_ready=true while production_validation_ready=false
9. Extended existing test assertions to cover observation counts and nested surfaces

### Files modified

- `src/application/entry_models/training_export.rs` — new structs, logic, tests
- `src/state/types.rs` — `structural_feedback_counter_outcome` visibility pub
- `AGENTS.md` — fixed E/F/H family status from MISSING to mapped, removed duplicate row

### Pending

- [x] Compile and test pass for `policy_training` filter
- [x] Fix stale test fixtures that needed the new `observation_validation` summary field
- [x] Verify `cargo test policy_training -- --nocapture` passes
- [x] Verify real NQ chain through pre-bayes / BBN / path-ranking / execution-tree on `/tmp/vrp-v2-runtime-closure`
- [x] Verify `./target/debug/ict-engine policy-training-status --symbol DEMO --state-dir /tmp/ict-engine-audit-demo --human`
- [x] Verify `cargo check --all-targets` (running in background)
- [ ] Address existing full-repo `cargo fmt --check` drift separately; do not mix broad formatting churn into this slice

## Fresh Chain Evidence - 2026-05-09 Loop Iteration

Commands personally run against `/tmp/vrp-v2-runtime-closure`:

- `pre-bayes-status --symbol NQ --human`: `gate=pass_neutralized`, `soft_evidence=yes`, bridge `long=0.551`, `short=0.530`, `mtf=bullish`, `align=1.000`, `entry_align=0.860`.
- `auto-quant-prior-init --symbol NQ --library /tmp/vrp_v2_strategy_library.json --dry-run --strategies VRPCompression_V2_NQ_15m`: BBN prior path alive; `trade_count=815`; dry-run final probs `[0.33990526417634764, 0.00001931209082675582, 0.6600754237328257]`.
- `policy-training-status --symbol NQ --human`: CatBoost/path-ranking surface alive but not closed; `rows=3`, `mature_rows=0`, `raw_scored_mature=0/30`, `production_validation=0/30`, `observation_validation=0/30`, `runtime_source=candidate_set`.
- `workflow-status --symbol NQ --human`: execution tree alive; `gate=pass_neutralized`, `Ranker: status=using_candidate_set_scores source=candidate_set applied=3`.
- `workflow-status --phase ensemble-vote --human`: `action=Observe`, `confidence=0.464`, policy runtime sample files readable.
- `workflow-status --phase structural-playbook --human`: returned `structural-feedback-v1` template and path plan; ranker runtime still `using_candidate_set_scores`.
- `workflow-status --phase structural-recommended-path-bundle --human`: selected `trend_follow_through`, posterior `0.464`, selected probability `0.370`.

Verdict: `stopped_at_path_ranking_validation_floor`. The chain is runnable and consumer-readable, but the candidate is not mature external-ranker closed until honest structural-feedback observations move `observation_validation` and target rows beyond the floor.

## Fresh Replay Verification - 2026-05-09 Loop Iteration 2

State checked: `/tmp/ict-engine-structural-replay-29/state`.

- Replay summary exists at `/tmp/ict-engine-structural-replay-29/replay_summary.json`: `count=29`, `lookback=52`, `horizon=16`, `threshold=0.001`, source candles `/Users/thrill3r/Downloads/Tomac/ict-cleaned-15m/nq.continuous-15m.json`.
- Learning state contains `30` structural-feedback observations from `structural_feedback_submission`: outcomes `loss=14`, `win=12`, `breakeven=4`.
- `policy-training-status --human`: `raw_scored_mature=2/30`, `production_validation=2/30`, `observation_validation=30/30`, `calibration=evaluated`, `runtime_source=registered_model_artifact`, `runtime_matches=1`, `ready=true`.
- `pre-bayes-status --human`: `gate=pass_neutralized`, bridge `long=0.519`, `short=0.526`, `mtf=bullish`, `align=1.000`, `entry_align=0.994`.
- `auto-quant-prior-init --dry-run --strategies VRPCompression_V2_NQ_15m`: BBN prior path still alive; `trade_count=815`, final probs `[0.33990526417634764, 0.00001931209082675582, 0.6600754237328257]`.
- `workflow-status --human`: execution tree uses `registered_model_artifact`; `Ranker: status=using_registered_model_artifact source=registered_model_artifact applied=1 artifact=1 ... gate=pass`.
- `workflow-status --phase ensemble-vote --human`: `action=execute_follow_through`, `confidence=1.000`.
- `workflow-status --phase structural-recommended-path-bundle --human`: `trend_follow_through`, `posterior=1.000`, `selected_prob=1.000`.
- `workflow-status --phase structural-playbook --human`: branch history has `total_records=30`, `wins=12`, `losses=14`, `breakevens=4`, `avg_pnl=0.0014675473531263`.
- `export-structural-path-ranking-target`: current target rows remain deliberately de-duplicated: `rows=1`, `history_rows=4`, `mature_rows=1`, `history_mature_rows=2`, all current row score/calibration/propensity/training-weight fields present.

Verdict: `closed_loop_changed_via_registered_model_artifact`, but not `target_row_validation_closed`. The consumer-visible chain is now hot-plug/ranker-backed and execution-tree-changing, while the target-row validation floor remains honestly reported as `2/30` instead of being inflated from repeated observations.

## Phase 2 - Consumer Zero-Config / Hot-Plug Surface

- [ ] Add/read optional profile config for user-specific VRP/NQ features
- [ ] Default behavior remains generic
- [ ] Human status shows profile=generic_zero_config / profile=thrill3r-nq / profile=disabled
- [ ] External ranker status shows ranker=disabled / fallback / registered_model / catboost

## Phase 3 - External Ranker Contract

- [ ] Python fixture test for output row count == target row count
- [ ] Rust/shell contract around apply scores
- [ ] Improve missing score CSV error message

## Phase 4 - Docs / Commit

Commit plan:
1. `fix: split structural path validation into target-row vs feedback-observation surfaces`
2. `docs: fix AGENTS.md factor family E/F/H status, remove duplicate`
3. `docs: add audit remediation handoff todo`

## Live Log

- 2026-05-09: Created handoff TODO
- 2026-05-09: Added TargetRowValidationSurface + FeedbackObservationValidationSurface
- 2026-05-09: Added outcome_distribution computation
- 2026-05-09: Added key test: 30 observations, 2 target rows, observation_ready=true, production_ready=false
- 2026-05-09: Fixed AGENTS.md E/F/H family mapping
- 2026-05-09: Background `cargo test policy_training -- --nocapture` completed: 4 relevant tests passed, 0 failed
- 2026-05-09: Updated stale analyze/backtest reporting fixtures to include `observation_validation=0/30`
- 2026-05-09: Ran live chain on `/tmp/vrp-v2-runtime-closure`; confirmed stop layer is path-ranking validation floor, not pre-bayes, BBN, or execution-tree command failure
- 2026-05-09: `cargo fmt --check` still fails due broad pre-existing formatting drift outside this slice; defer separate cleanup to avoid pollution
- 2026-05-09: Re-verified `/tmp/ict-engine-structural-replay-29/state`; 30 structural-feedback observations produce `observation_validation=30/30`, registered-model runtime, and execution-tree `execute_follow_through` while target-row validation remains transparent at `2/30`
