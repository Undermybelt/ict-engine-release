# 2026-05-09 Regime -> Execution Mainline Audit Handoff TODO

## Mission

继续审计并补齐 ICT Engine 从 regime 主/子分类到实战建议的主链闭环。

范围：
- regime 主分类 / 子分类
- BBN / Pre-Bayes 证据
- CatBoost / structural path ranker
- execution tree
- analyze / analyze-live / recommended command

硬约束：
- 不混入无关脏改。
- 实验状态只写 `/tmp/...`。
- 先证明 runtime 消费，再谈源码定义。
- 输出必须能给下个 agent 直接接手。

## Route / Skill

- Route alias: `ict-engine-runtime`
- Skill loaded: `~/.hermes/skills/ict-engine/ict-engine-runtime/SKILL.md`
- Reference used: `references/repo-audit-factor-to-execution-closure.md`
- Repo guide read: `AGENTS.md`

## Current git state

As of the latest handoff update, worktree was clean after the integration commits below.

Integration commits from this continuation:
- `cfb9d6a surface consumer reason in runtime outputs`
- `3b13667 feat: expose regime bundle trace on analyze`
- `0ef95c0 surface market-state in factor human reports`
- `b8fc1e8 feat: expose regime bundle trace on analyze-live`
- `c88a819 fix: load analyze regime bundle before state mutation`
- `5c799e2 document mature ranker validation`
- `d5458f6 map regime bundle to read-only bbn evidence`
- `53e768f format command wrappers`

Related regime-classifier handoff commit also exists:
- `2e7998b docs: add regime classifier r17 handoff`

Repo discipline for next agent:
- Run `git status --short` first.
- Keep experiment state in `/tmp/ict-mainline-regime-audit` or another `/tmp/...` path.
- Stage only files touched in the next slice.

## What was done

### 1. Previous implementation slice committed

Commit:
- `a0e9b4f thread 30m multi-timeframe PDA evidence`

Validated:
- `cargo check`
- `cargo test --bin ict-engine multi_timeframe -- --nocapture`
- `factor-research` / `factor-backtest` with `cleaned-30m`

Key runtime evidence from previous slice:
- `covered_intervals=1m,5m,15m,30m,1h,4h,1d`
- `structure_ict_pda_context_events=m1:239|m5:239|m15:239|m30:239|h1:239|h4:239|d1:239|w1:0`

### 2. Mainline audit replayed in isolated state

Audit root:
- `/tmp/ict-mainline-regime-audit`

Commands run and passed:
- `cargo check`
- `./target/debug/ict-engine validate-market-state --data /tmp/ict-mainline-regime-audit/cleaned-15m/nq.continuous-15m.json --window-size 40 --step-size 5 --profile high_confidence --compact`
- `./target/debug/ict-engine factor-research --symbol NQ ... --backend native --state-dir /tmp/ict-mainline-regime-audit/state --output-format json`
- `./target/debug/ict-engine factor-backtest --symbol NQ ... --state-dir /tmp/ict-mainline-regime-audit/state --output-format json`
- `./target/debug/ict-engine analyze --symbol NQ --data-root /tmp/ict-mainline-regime-audit --state-dir /tmp/ict-mainline-regime-audit/state --output-format json --inline-ledger`
- `./target/debug/ict-engine export-structural-path-ranking-target --symbol NQ --state-dir /tmp/ict-mainline-regime-audit/state`
- `./target/debug/ict-engine apply-structural-path-ranking-external-scores --symbol NQ --state-dir /tmp/ict-mainline-regime-audit/state --scores-file /tmp/ict-mainline-regime-audit/scores.csv`
- `./target/debug/ict-engine register-structural-path-ranking-trainer-artifact --symbol NQ --state-dir /tmp/ict-mainline-regime-audit/state --artifact-uri file:///tmp/ict-mainline-regime-audit/trainer_artifact.json --model-family catboost --score-column raw_path_score --trained-rows 3 --calibration-rows 0`
- `./target/debug/ict-engine enable-structural-path-ranking-runtime --symbol NQ --state-dir /tmp/ict-mainline-regime-audit/state --reuse-mode prefer_history`
- `./target/debug/ict-engine policy-training-status --symbol NQ --state-dir /tmp/ict-mainline-regime-audit/state --output-format json`
- `./target/debug/ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-mainline-regime-audit/state --output-format json`
- `./target/debug/ict-engine analyze-live --symbol NQ --state-dir /tmp/ict-mainline-regime-audit/state --output-format json`

Generated audit files:
- `/tmp/ict-mainline-regime-audit/validate-market-state.txt`
- `/tmp/ict-mainline-regime-audit/factor-research.json`
- `/tmp/ict-mainline-regime-audit/factor-backtest.json`
- `/tmp/ict-mainline-regime-audit/analyze.json`
- `/tmp/ict-mainline-regime-audit/analyze-after-ranker.json`
- `/tmp/ict-mainline-regime-audit/analyze-live.json`
- `/tmp/ict-mainline-regime-audit/policy-status-after-ranker.json`
- `/tmp/ict-mainline-regime-audit/workflow-status-after-ranker.json`
- `/tmp/ict-mainline-regime-audit/state/NQ/execution_tree_trace.json`

### 3. Field coverage result

| Stage | Evidence | Diagnostics | Lineage | Verdict |
|---|---:|---:|---:|---|
| regime primary/secondary | yes | yes | yes | reaches analyze/live/execution_tree |
| BBN / Pre-Bayes | yes | yes | partial | primary/secondary regime in filtered assignments |
| factor-research | yes | yes | yes | market_state primary/secondary now appears in JSON bundle + human |
| factor-backtest | yes | yes | yes | market_state primary/secondary now appears in JSON bundle + human |
| CatBoost/path ranker | yes | yes | yes | policy-status + execution_tree lineage + trace machine fields visible |
| execution tree | yes | yes | yes | market_state + path_ranker + consumer_reason reach trace |
| recommendation | yes | yes | partial | human/analyze reason line exists; structural path bundle text still does not change between registered vs disabled ranker in mature replay |
| regime consumer bundle | yes | yes | yes | analyze/analyze-live trace entries plus read-only BBN soft evidence adapter |

### 4. Runtime evidence found

`validate-market-state.txt`:
- `samples=9 avg_confidence=74.00% high_confidence=33.33% tradeable=100.00% primary_top=RangeConsolidation:6 secondary_top=WideRange:5`

`analyze-after-ranker.json`:
- `market_state_primary_regime=RangeConsolidation`
- `market_state_secondary_regime=WideRange`
- `pre_bayes_filtered_assignments.market_regime=range`
- `pre_bayes_filtered_assignments.liquidity_context=favorable`
- `pre_bayes_gate_status=pass_neutralized`

`analyze-live.json`:
- `market_state_primary_regime=TrendExpansion`
- `market_state_secondary_regime=BullTrendExhaustion`
- `pre_bayes_filtered_assignments.market_regime=bull`
- `pre_bayes_filtered_assignments.liquidity_context=neutral`
- execution triage:
  - `branch=transition_guardrail`
  - `gate_status=observe`
  - `execution_bias=guarded`
  - `decision_hint=execution_guarded_due_to_high_transition_hazard`

`policy-status-after-ranker.json`:
- `structural_path_ranking_runtime.enabled=true`
- `ready=true`
- `model_family=catboost`
- `active_match_count=3`
- `status=enabled_candidate_set_ready`
- validation still not mature:
  - `raw_scored_mature=0/30`
  - `production_validation=0/30`
  - `observation_validation=0/30`
  - `calibration=not_fitted`

`execution_tree_trace.json` lineage:
- `market_state=primary_regime=TrendExpansion secondary_regime=BullTrendExhaustion overall_confidence=0.553`
- `market_state=volatility=LowVol:0.779 liquidity=NormalLiquidity:0.363 structure=Trending:0.634 behavior=Neutral:0.450`
- `path_ranker=Ranker runtime: ... trainer_artifact=ready ... runtime_selection=enabled_candidate_set_ready runtime_mode=prefer_history runtime_source=candidate_set runtime_matches=3`
- `path_ranker=Ranker validation: calibration=false ... raw_scored_mature=0/30 ... ready=false`
- `hmm_posterior=(acc=0.286, manip=0.429, dist=0.286)`
- `hybrid_transition_hazard=0.607`

### 5. Source trace confirmed

Regime classification and BBN/Pre-Bayes bridge:
- `src/main.rs:4084-4094` classifies `MarketStateClassifier` and maps market state to BBN regime/liquidity labels.
- `src/main.rs:4177-4189` inserts `market_state_primary_regime` and `market_state_secondary_regime` into `pre_bayes_evidence_filter.evidence_assignments` and rationale.
- `src/main.rs:4191` converts Pre-Bayes filter into BBN evidence via `trade_evidence_from_pre_bayes_filter(...)`.

Execution tree bridge:
- `src/main.rs:4624-4634` reads path-ranker runtime/validation lineage from `policy_training_status`.
- `src/main.rs:4636-4644` passes `market_state_lineage` and `path_ranker_lineage` into `ExecutionTreeInput`.
- `src/application/orchestration/execution_tree.rs:344-352` writes market_state and path_ranker lineage into execution tree split reasons.

### 6. Continuation implementation summary

Consumer reason / analyze outputs:
- Commit `cfb9d6a surface consumer reason in runtime outputs`.
- `ExecutionTriage.consumer_reason` now reaches:
  - `report.supporting.execution_triage.consumer_reason`
  - `compact_report.execution_triage.consumer_reason`
  - `agent_report.execution_triage.consumer_reason`
  - `execution_tree_trace.json.output.consumer_reason`
  - first line of `--human` output
- Real output value:
  - `market_state=RangeConsolidation/WideRange | execution=observe/transition_guardrail/guarded | ranker=candidate_set/catboost/not_ready`
- Evidence files:
  - `/tmp/ict-mainline-regime-audit/analyze-consumer-reason.json`
  - `/tmp/ict-mainline-regime-audit/analyze-consumer-reason-human.txt`

Regime consumer bundle trace:
- Commits:
  - `3b13667 feat: expose regime bundle trace on analyze`
  - `b8fc1e8 feat: expose regime bundle trace on analyze-live`
  - `c88a819 fix: load analyze regime bundle before state mutation`
- CLI flags:
  - `analyze --regime-consumer-bundle <path> [--regime-consumer-bundle-strict]`
  - `analyze-live --regime-consumer-bundle <path> [--regime-consumer-bundle-strict]`
- Trace entries added to `artifact_action_summary`:
  - `regime_bundle_status=loaded|missing|invalid|disabled`
  - `regime_bundle_path=<path>`
  - `regime_decision_state=<state>`
  - `regime_trade_usable=<bool>`
  - `regime_final_label=<label>`
  - `regime_execution_tree_hint=<accept_regime|transition_guardrail|unknown_abstain>`
- Sample fixture used:
  - `/tmp/ict-mainline-regime-audit/regime-consumer-bundle-sample.json`
- Runtime evidence:
  - `/tmp/ict-mainline-regime-audit/analyze-regime-bundle.json`

Factor report market-state surfacing:
- Commit `0ef95c0 surface market-state in factor human reports`.
- `factor-research` and `factor-backtest` JSON already carry:
  - `market_state_primary_regime=RangeConsolidation`
  - `market_state_secondary_regime=WideRange`
  - `market_state_bbn_market_regime=range`
  - `market_state_bbn_liquidity_context=favorable`
- Human output now includes:
  - `Market State: RangeConsolidation/WideRange | bbn_regime=range | liquidity=favorable`
- Evidence files:
  - `/tmp/ict-mainline-regime-audit/factor-research-market-state-verify.json`
  - `/tmp/ict-mainline-regime-audit/factor-backtest-market-state-verify.json`
  - `/tmp/ict-mainline-regime-audit/factor-research-market-state-human.txt`
  - `/tmp/ict-mainline-regime-audit/factor-backtest-market-state-human.txt`

Regime bundle read-only BBN soft evidence:
- Commit `d5458f6 map regime bundle to read-only bbn evidence`.
- `RegimeConsumerBundleAdapter::to_read_only_bbn_soft_evidence()` maps:
  - `single_label_99 + trade_usable=true` -> `Strong`, weight `0.9`
  - `single_label_95 + trade_usable=true` -> `Moderate`, weight `0.65`
  - missing / invalid / abstain / transitional / unknown -> `Neutral`, weight `0.0`
- Important: this is currently an adapter surface, not yet injected into live BBN posterior math.
- Test file:
  - `tests/regime_consumer_bundle_adapter.rs`
- Validation:
  - `cargo test --test regime_consumer_bundle_adapter -- --nocapture` -> 11/11 pass

Mature ranker replay:
- Commit `5c799e2 document mature ranker validation`.
- Mature source state:
  - `/tmp/ict-engine-structural-replay-29/state`
- See P4 section below for the exact conclusion.

## Current verdict

Mainline status: runnable and materially connected.

Closed:
- regime primary/secondary reaches analyze/live Pre-Bayes assignments.
- regime evidence reaches BBN soft evidence path through Pre-Bayes.
- regime and path-ranker lineage reaches execution_tree_trace.
- `consumer_reason` reaches JSON, compact, agent, human, and trace output.
- `factor-research` and `factor-backtest` now surface market-state primary/secondary in JSON bundles and human output.
- CatBoost-labeled external ranker artifact can be registered, enabled, and surfaced in policy status / execution tree lineage.
- Regime consumer bundle can be loaded by analyze/analyze-live and surfaced as trace entries.
- Regime consumer bundle now has a read-only BBN soft evidence adapter.
- Mature ranker replay proves `registered_model_artifact` readiness can be achieved via observation validation (`30/30`) even when target-row validation is `2/30`.

Still open / next work:
- Read-only regime bundle BBN evidence is not yet injected into BBN posterior math; it is only mapped and test-covered.
- Regime consumer bundle currently surfaces trace entries, but does not yet alter execution tree branch/gate/bias.
- Structural path bundle recommendation text did not differ between registered-model runtime and disabled runtime in mature replay; if desired, add ranker-source reason to that human surface.
- CatBoost path remains external-score / registered-artifact integration, not proven Rust-native CatBoost inference.
- Need a final end-to-end smoke after any future BBN injection: bundle -> BBN soft evidence -> posterior -> execution trace -> human reason.

## TODO - should do next

### P0 - Repo discipline

- [x] Re-run `git status --short` before edits.
- [x] Keep experiment state in `/tmp/...`.
- [x] Do not format whole repo.
- [x] Stage only files touched in each slice.

### P1 - Close factor-research / factor-backtest market_state gap

- [x] Add market_state classification to `factor-research` runtime.
- [x] Add market_state classification to `factor-backtest` runtime.
- [x] Emit `market_state_primary_regime` / `market_state_secondary_regime` in:
  - [x] `report.multi_timeframe_summary`
  - [x] `agent_context_bundle.multi_timeframe_summary`
  - [x] human output line
- [x] Add tests proving market-state human output is surfaced.
- [x] Re-run:
  - [x] `cargo check`
  - [x] `cargo test --bin ict-engine multi_timeframe -- --nocapture`
  - [x] `cargo test --bin ict-engine test_market_state_summary_threads_primary_secondary_regime -- --nocapture`
  - [x] `cargo test --lib factor_research_human_output_is_short_text_not_json_dump -- --nocapture`

Evidence:
- `/tmp/ict-mainline-regime-audit/factor-research-market-state-verify.json`
- `/tmp/ict-mainline-regime-audit/factor-backtest-market-state-verify.json`
- `/tmp/ict-mainline-regime-audit/factor-research-market-state-human.txt`
- `/tmp/ict-mainline-regime-audit/factor-backtest-market-state-human.txt`

### P2 - Make recommendation explain why, not only what command

- [x] Add execution-tree short reason as `consumer_reason`.
- [x] Include:
  - [x] `market_state_primary_regime`
  - [x] `market_state_secondary_regime`
  - [x] execution tree branch/gate/bias
  - [x] path-ranker runtime source/model
  - [x] ranker validation readiness
- [x] Ensure `--human` / JSON output remains compact.
- [x] Verify `analyze --human` shows the reason line.

Evidence:
- `/tmp/ict-mainline-regime-audit/analyze-consumer-reason.json`
- `/tmp/ict-mainline-regime-audit/analyze-consumer-reason-human.txt`

Still optional:
- [ ] Add the same short reason into structural path bundle human text if desired.
- [ ] Re-run `analyze-live --human` with a live/backend-available state if needed.

### P3 - Make CatBoost/path-ranker consumption explicit

- [x] Add machine fields, not only text lineage:
  - [x] `path_ranker_score_used_by_execution_tree: bool`
  - [x] `path_ranker_model_family`
  - [x] `path_ranker_runtime_source`
  - [ ] `candidate_set_id`
  - [ ] `path_id` if selected
  - [x] `ranker_validation_ready`
  - [x] `consumer_reason`
- [x] Surface these fields in `execution_tree_trace.json`.
- [x] Surface compact consumer summary in analyze JSON / human.
- [x] Add/runtime-test proof that CatBoost-registered artifact appears in execution tree trace.

Evidence:
- `/tmp/ict-mainline-regime-audit/state/NQ/execution_tree_trace.json`

Still optional:
- [ ] Add explicit `candidate_set_id` / `path_id` if a stable ID source is available in runtime selection.

### P4 - Validate mature ranker path separately

- [x] Reuse mature state if available, e.g. prior handoff mentioned `/tmp/ict-engine-structural-replay-29/state`.
- [x] Run:
  - [x] `policy-training-status --symbol NQ --state-dir <mature-state> --human`
  - [x] `workflow-status --symbol NQ --state-dir <mature-state> --human`
  - [x] `workflow-status --phase structural-recommended-path-bundle --human`
- [x] Confirm whether registered-model artifact changes execution recommendation vs candidate-set fallback.
- [x] Record whether validation floor is target-row or observation-row.

P4 result (2026-05-09):
- Mature state exists: `/tmp/ict-engine-structural-replay-29/state`.
- Copied for comparison:
  - registered: `/tmp/ict-mainline-regime-audit/state-mature-ranker-registered`
  - disabled: `/tmp/ict-mainline-regime-audit/state-mature-ranker-disabled`
- Runtime registered-model status:
  - `runtime_selection=enabled_registered_model_ready`
  - `runtime_mode=candidate_set_only`
  - `runtime_source=registered_model_artifact`
  - `runtime_matches=1`
- Validation:
  - `raw_scored_mature=2/30`
  - `production_validation=2/30`
  - `observation_validation=30/30`
  - `calibration=evaluated`
  - `ready=true`
- Interpretation: readiness is satisfied by observation validation despite target-row / production validation remaining `2/30`. The `raw_scored_mature` floor is target-row style, but `quality_ready=true` and `ready=true` can pass when observation validation reaches `30/30`.
- Registered-model vs disabled runtime recommendation:
  - `workflow-status --phase structural-recommended-path-bundle --human` produced the same selected path and next command in this state.
  - Difference is visible in policy status/runtime source, not in the structural path bundle recommendation text.
- Evidence files:
  - `/tmp/ict-mainline-regime-audit/policy-status-mature-registered-human.txt`
  - `/tmp/ict-mainline-regime-audit/policy-status-mature-disabled-human.txt`
  - `/tmp/ict-mainline-regime-audit/path-bundle-mature-registered-human.txt`
  - `/tmp/ict-mainline-regime-audit/path-bundle-mature-disabled-human.txt`

### P5 - Regime consumer bundle closure

- [x] Add adapter load path for `analyze`.
- [x] Add adapter load path for `analyze-live`.
- [x] Surface trace entries into `artifact_action_summary`.
- [x] Strict mode errors before state mutation for missing/invalid bundle.
- [x] Add read-only BBN soft evidence mapping.
- [x] Test missing / invalid / loaded / strong / moderate / neutral mappings.

Validation:
- `cargo test --test regime_consumer_bundle_adapter -- --nocapture` -> 11/11 pass
- `cargo check` pass

Evidence:
- `/tmp/ict-mainline-regime-audit/regime-consumer-bundle-sample.json`
- `/tmp/ict-mainline-regime-audit/analyze-regime-bundle.json`

Still open:
- [ ] Feed `RegimeReadOnlyBbnSoftEvidence` into actual BBN evidence/posterior path as neutral/soft evidence, preserving read-only and non-destructive semantics.
- [ ] Add runtime smoke proving bundle changes a BBN diagnostic field or posterior rationale without silently overriding market_state classifier evidence.
- [ ] If bundle says abstain/transitional, ensure execution tree reason surfaces `transition_guardrail` or `unknown_abstain` rather than treating it as accepted regime.

### P6 - Recommended next implementation slice

Recommended next slice: `feed regime bundle soft evidence into BBN diagnostics`.

Target behavior:
- Given `--regime-consumer-bundle` with `single_label_99`, analyze/analyze-live should surface a BBN diagnostic line such as:
  - `regime_bundle_bbn_strength=strong`
  - `regime_bundle_bbn_weight=0.900`
  - `regime_bundle_bbn_label=<label>`
- If abstain/missing/invalid, output neutral diagnostics and do not alter posterior.
- Do not hard override existing `market_state` evidence. Treat bundle as additive read-only soft evidence unless explicitly promoted later.

Suggested steps:
1. Add a small conversion layer from `RegimeReadOnlyBbnSoftEvidence` to analyze supporting diagnostics / Pre-Bayes rationale first.
2. Add tests for strong/moderate/neutral diagnostics.
3. Run real CLI smoke with `/tmp/ict-mainline-regime-audit/regime-consumer-bundle-sample.json`.
4. Only then consider posterior math changes, behind a clearly named field or feature gate.

Suggested commands:
```bash
cargo test --test regime_consumer_bundle_adapter -- --nocapture
cargo test --bin ict-engine regime_bundle -- --nocapture
cargo check
cargo build --bin ict-engine
BASE=/tmp/ict-mainline-regime-audit
./target/debug/ict-engine analyze \
  --symbol NQ \
  --data-root $BASE \
  --state-dir $BASE/state \
  --output-format json \
  --inline-ledger \
  --regime-consumer-bundle $BASE/regime-consumer-bundle-sample.json \
  > $BASE/analyze-regime-bundle-bbn.json
```

### P7 - Optional cleanup / documentation

- [x] Add this handoff as a versioned repo doc.
- [x] Update skill reference for regime consumer bundle + mature ranker validation.
- [ ] Update `AGENTS.md` only if factor/regime routing changed.
- [ ] Add a stable `support/docs/audits/...` report if this handoff becomes too long.

## Reproduction notes

Audit root:
- `/tmp/ict-mainline-regime-audit`

Core replay chain:
1. validate market state
2. factor-research
3. factor-backtest
4. analyze
5. export structural path target
6. generate/apply `scores.csv`
7. register trainer artifact as `catboost`
8. enable runtime
9. analyze again
10. workflow-status
11. analyze-live if network/backend available

Useful existing replay files:
- `/tmp/ict-mainline-regime-audit/analyze-consumer-reason.json`
- `/tmp/ict-mainline-regime-audit/analyze-regime-bundle.json`
- `/tmp/ict-mainline-regime-audit/factor-research-market-state-verify.json`
- `/tmp/ict-mainline-regime-audit/factor-backtest-market-state-verify.json`
- `/tmp/ict-mainline-regime-audit/policy-status-mature-registered-human.txt`
- `/tmp/ict-mainline-regime-audit/policy-status-mature-disabled-human.txt`

Current clean verification set:
```bash
cargo check
cargo test --test regime_consumer_bundle_adapter -- --nocapture
cargo test --bin ict-engine test_market_state_summary_threads_primary_secondary_regime -- --nocapture
cargo test --bin ict-engine multi_timeframe -- --nocapture
cargo test --lib factor_research_human_output_is_short_text_not_json_dump -- --nocapture
```

Important pitfall:
- Use `python3`, not `python`, in CLI smoke scripts on this macOS host.
- `cargo check` alone is not enough before real CLI smoke; rebuild with `cargo build --bin ict-engine`.
