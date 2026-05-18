# 2026-05-09 Regime -> Execution Mainline Implementation Handoff TODO

## Mission

继续把 regime -> factor reports -> BBN/Pre-Bayes -> path ranker/CatBoost -> execution tree -> practical recommendation 做成可消费主链。

约束：
- 零配置：默认 CLI 可跑，不要求用户先配权重/模型。
- 消费者可用：machine JSON 有显式字段，human/agent/compact 有短理由。
- token 友好：输出短线索，不倾倒长 trace。
- 无污染：实跑 state 只用 `/tmp/...`；不全仓 fmt；只提交本轮触碰文件。
- 热插拔：path ranker / hotplug / market_state 都保持可启停、可缺省。
- 用户个人数据内容优先：VRP/NQ 辅助字段、primary/secondary market_state、ranker readiness、execution branch/gate/bias 必须能被消费。

## Route / Skill

- Route alias: `ict-engine/ict-engine-runtime`
- Runtime skill: `~/.hermes/skills/ict-engine/ict-engine-runtime/SKILL.md`
- Process skill: `software-development/test-driven-dev`
- Repo guide: `AGENTS.md`
- Global project-router: missing/empty; no override used

## Commits this turn

- `abcb9fa thread market state through factor reports`

Notes:
- First commit attempt accidentally included pre-staged external changes; immediately soft-reset and recommitted only `src/main.rs`, `src/factor_research_runtime.rs`, `src/factor_backtest_runtime.rs`.
- External dirty files remain untouched.

## Implemented after commit, not yet committed

### P2/P3 slice: explicit execution-tree/ranker consumer fields

Files touched:
- `src/application/orchestration/execution_tree.rs`
- `src/application/reflection/execution_tree_bundle.rs`
- `src/main.rs`

Changes:
- `ExecutionTreeOutput` now carries machine fields:
  - `path_ranker_score_used_by_execution_tree: bool`
  - `path_ranker_model_family: Option<String>`
  - `path_ranker_runtime_source: Option<String>`
  - `ranker_validation_ready: bool`
- `ExecutionTriage` now carries `reason_summary: Vec<String>`.
- Triage reason summary keeps compact selected lineage:
  - market_state lines
  - path_ranker lines
  - branch line
  - hybrid transition hazard line
- `build_analyze_report` now adds compact `ranker_machine=...` lineage into execution-tree input:
  - `source`
  - `model_family`
  - `validation_ready`
  - `active_match_count`

TDD evidence:
- RED verified: `cargo test --lib triage_reason_summary_includes_regime_and_ranker_context -- --nocapture` failed because `ExecutionTriage.reason_summary` field was missing.
- RED verified: `cargo test --lib execution_tree_surfaces_path_ranker_machine_fields -- --nocapture` failed because `ExecutionTreeOutput.path_ranker_*` fields were missing.
- GREEN:
  - `cargo test --lib path_ranker -- --nocapture` PASS
  - `cargo test --lib triage_reason_summary -- --nocapture` PASS

Validation:
- `cargo check` PASS
- `cargo test --bin ict-engine test_market_state_summary_threads_primary_secondary_regime -- --nocapture` PASS

## Current dirty state to respect

Known unrelated/external dirty files before/around this slice:
- `support/docs/plans/2026-05-09-regime-classifier-r5-handoff-todo.md`
- `support/docs/plans/2026-05-09-regime-classifier-research-and-99-confidence-todo.md`
- `src/auto_quant_command.rs`
- `src/validate_market_state_command.rs`
- `support/docs/plans/2026-05-09-regime-classifier-r6-handoff-todo.md`
- `support/scripts/research/regime_conformal_calibration_report.py`
- `support/scripts/research/tests/test_regime_conformal_calibration_report.py`

This handoff file itself is new and should be included only if committing this slice.

## Next actions

### P0 - Before editing/committing

- [ ] Run `git status --short`.
- [ ] Stage only this slice:
  - [ ] `src/application/orchestration/execution_tree.rs`
  - [ ] `src/application/reflection/execution_tree_bundle.rs`
  - [ ] `src/main.rs`
  - [ ] `support/docs/plans/2026-05-09-regime-to-execution-mainline-implementation-handoff-todo.md`
- [ ] Do not stage unrelated files listed above.

### P1 - Commit current P2/P3 slice

Recommended commit:
- `surface path ranker fields in execution trace`

Before commit, run:
- [ ] `cargo check`
- [ ] `cargo test --lib path_ranker -- --nocapture`
- [ ] `cargo test --lib triage_reason_summary -- --nocapture`
- [ ] `cargo test --bin ict-engine test_market_state_summary_threads_primary_secondary_regime -- --nocapture`

### P2 - Runtime smoke in isolated state

Use `/tmp/ict-mainline-regime-audit` if still present; else recreate from prior sample root.

Run:
- [ ] `./target/debug/ict-engine analyze --symbol NQ --data-root /tmp/ict-mainline-regime-audit --state-dir /tmp/ict-mainline-regime-audit/state --output-format json --inline-ledger`
- [ ] Check JSON/report has:
  - [ ] `supporting.execution_triage.reason_summary`
  - [ ] `supporting.execution_artifact.features` still present
  - [ ] execution tree trace `output.path_ranker_score_used_by_execution_tree`
  - [ ] execution tree trace `output.path_ranker_runtime_source`
  - [ ] execution tree trace `output.path_ranker_model_family` when ranker artifact exists
  - [ ] execution tree trace `output.ranker_validation_ready`
- [ ] Run `--human` and confirm short execution line + reason line is not too verbose.

### P3 - Recommendation reason text

Still to improve:
- [ ] `recommended_next_command_meta` or adjacent output should expose a compact reason object/line that joins:
  - [ ] market_state primary/secondary
  - [ ] execution branch/gate/bias
  - [ ] path-ranker source/model/validation readiness
- [ ] Keep machine fields separate from prose.
- [ ] Keep `--compact` / `--agent` low token.

### P4 - Mature ranker validation

Need separate mature-state proof:
- [ ] Try `/tmp/ict-engine-structural-replay-29/state` if present.
- [ ] Run `policy-training-status --symbol NQ --state-dir <state> --human`.
- [ ] Run `workflow-status --symbol NQ --state-dir <state> --human`.
- [ ] Confirm target-row vs observation-row validation floor.
- [ ] Record whether registered model changes recommendation vs candidate-set fallback.

## Latest update

### Consumer reason field

Implemented in `src/application/orchestration/execution_tree.rs`:
- `ExecutionTriage.consumer_reason`
- Format:
  - `market_state=<primary>/<secondary> | execution=<gate>/<branch>/<bias> | ranker=<source>/<model>/<ready|not_ready>`
- Example:
  - `market_state=TrendExpansion/BullTrendExhaustion | execution=ready/fill_viable/aggressive | ranker=registered_artifact/catboost/ready`

TDD evidence:
- RED verified: `cargo test --lib triage_consumer_reason_merges_market_execution_and_ranker -- --nocapture` failed because `ExecutionTriage.consumer_reason` was missing.
- GREEN verified: same test passed after implementation.

Regression checks:
- `cargo test --lib path_ranker -- --nocapture` PASS
- `cargo test --lib triage_reason_summary -- --nocapture` PASS
- `cargo check` PASS

## Current verdict

- P1 from old audit closed and committed: factor-research/backtest now report market_state primary/secondary.
- P2/P3 closed: execution tree trace has explicit path-ranker machine fields; triage has short reason summary and clean `consumer_reason`.
- Remaining optional gap: runtime smoke on `/tmp/ict-mainline-regime-audit` to prove JSON/human surfaces show the new field in a full analyze run.
