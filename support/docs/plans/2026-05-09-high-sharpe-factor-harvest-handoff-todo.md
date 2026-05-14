# Regime-Conditioned Win-Rate Strategy Selection Handoff TODO

Live board for paper/repo factor harvest and infinite iteration.

Goal: turn external factor/strategy references into zero-config, hot-plug candidates that can be selected only when the current regime is reliable enough and the candidate's regime-conditioned win rate/payoff profile is acceptable.

Direction correction: this board is no longer a "find high Sharpe" lane. Sharpe remains a useful diagnostic for smoothness and tail-adjusted stability, but it is not the north-star promotion gate. The next loop should prove:

1. Regime classification is accurate/calibrated enough to trust.
2. Each candidate has an explicit regime match or regime exclusion rule.
3. The selected strategy has acceptable regime-conditioned win rate, trade density, profit factor / avg R:R, drawdown, and tail behavior.
4. If regime confidence is low or no strategy matches the regime, execution should stay in `observe` / no-trade.

---

## Done

- [x] Routed through `ict-engine-runtime`.
- [x] Loaded heuristic learning harvest reference.
- [x] Checked current dirty worktree; unrelated dirty files preserved.
- [x] Reviewed existing heuristic learning docs:
  - `support/docs/plans/2026-05-09-heuristic-learning-module-harvest-report.md`
  - `support/docs/plans/2026-05-09-heuristic-learning-self-iteration-plan.md`
  - `support/docs/plans/2026-05-09-regime-classifier-r20-handoff-todo.md`
- [x] Searched papers and repos via arXiv/Semantic Scholar/GitHub API plus delegated research.
- [x] Created source registry and iteration contract:
  - `support/docs/plans/2026-05-09-high-sharpe-factor-harvest-and-infinite-iteration.md`

Note: the source-registry filename is historical. This handoff board now overrides the old high-Sharpe framing for subsequent execution.

---

## Source harvest

Papers/families captured as strategy/factor raw material, not as automatic high-Sharpe promotion targets:

- Time-Series Momentum / managed futures trend following
- Value and Momentum Everywhere
- Carry / roll yield
- Betting Against Beta
- Quality Minus Junk
- Momentum Crash filter
- FX carry / FX momentum
- Variance Risk Premium / delta-hedged options
- Volatility spread / option momentum
- Crypto momentum/size/liquidity
- Order flow imbalance / book pressure
- Residual statistical arbitrage / OU reversion

Repos captured:

- `microsoft/qlib`
- `STHSF/alpha101`
- `quantopian/empyrical`
- `ranaroussi/quantstats`
- `pmorissette/bt`
- `hudson-and-thames/mlfinlab`
- `hudson-and-thames/arbitragelab`
- `robcarver17/pysystemtrade`
- `robcarver17/systematictradingexamples`
- `mansoor-mamnoon/limit-order-book`
- `kernc/backtesting.py`
- `quantrocket-llc/zipline`

---

## Consumer contract

- Zero-config default remains unchanged.
- Runtime must not import large research frameworks.
- Candidate factors are sidecar artifacts first.
- Regime quality is the first decision gate:
  - classify current regime with confidence/calibration metadata;
  - reject or downgrade strategy selection when regime confidence is weak;
  - keep "unknown / transition / mixed" regimes as first-class no-trade or observe states.
- Strategy promotion is regime-conditional:
  - do not judge a strategy only by aggregate Sharpe;
  - require per-regime hit rate / win-rate lower bound, trade density, profit factor, avg R:R, drawdown, and tail checks;
  - require explicit "enabled regimes" and "disabled regimes" in the candidate artifact;
  - prefer a lower-Sharpe strategy with stable, regime-matched win rate over a high-Sharpe strategy that wins only in an unidentified or unstable regime.
- User-specific fields are optional and hot-plug:
  - `qqq_hv_level`
  - `qqq_hv_pct_rank_252`
  - `nq_vs_200d_pct`
  - `vix3m_level`
  - `vvix_over_vix`
  - `vrp`
  - `iv_rank`
  - `hv_rank`
- Missing optional fields must emit `missing_optional`, not fail.
- Promotion requires regime calibration, regime-conditioned OOS win-rate/payoff evidence, DSR/PBO/tail checks, BBN value lift, path-ranker readback, and execution-tree closure.

---

## Historical implementation queue

### R22: factor formula seed library

- [x] Add `support/scripts/research/factor_formula_seed_library.py`.
- [x] Add tests: `support/scripts/research/tests/test_factor_formula_seed_library.py`.
- [x] Emit JSON candidate specs for first 16 candidates in the harvest doc.
- [x] Include `source_refs`, `family`, `required_fields`, `optional_fields`, `missing_optional_policy`.
- [x] Keep no third-party heavy dependency.
- [x] Validate JSON output.
- [x] Preserve user-specific fields as optional hot-plug fields, never required.

Observed artifact:

```text
/tmp/ict-hl/factor_seed_candidates.json
schema=factor-formula-seed-library/v1
candidate_count=16
first=tsmom_mtf_convexity_v1
vrp_optional_ok=True
```

### R23: payoff gate expansion

- [x] Ensure payoff report exposes:
  - Sharpe / Sortino / Calmar
  - max drawdown
  - CVaR / tail ratio
  - profit factor
  - hit rate
  - avg R/R
  - OOS Sharpe LCB
  - DSR / PBO
  - effective sample size
- [x] Add failure tags for `high_pbo`, `low_dsr`, `tail_risk_hidden`.
- [x] Add regression test: `test_report_exposes_r23_payoff_gate_fields_and_failure_tags`.
- [x] Validate CLI JSON output.

Observed artifact:

```text
/tmp/ict-hl/r23_payoff_report.json
sharpe=-2.5303377710824577
sortino=-2.5561399172110284
calmar=-0.48333333333333334
cvar_95=-3.0
tail_ratio=0.09999999999999999
profit_factor=0.532258064516129
avg_rr=0.152073732718894
oos_sharpe_lcb=-3.183671104415791
dsr=0.0
pbo=1.0
effective_sample_size=9
failure_tags=['thin_density', 'negative_edge', 'low_dsr', 'high_pbo', 'tail_risk_hidden']
promotion_gate=reject
```

### R24: QQQ/NQ VRP sidecar

- [x] Add optional auxiliary schema for VIX/VIX3M/VVIX/HV/IV.
- [x] Emit `vrp_pressure_qqq_v1`.
- [x] Keep zero-config fallback when fields missing.
- [x] Add sidecar script: `support/scripts/research/qqq_nq_vrp_sidecar.py`.
- [x] Add tests: `support/scripts/research/tests/test_qqq_nq_vrp_sidecar.py`.
- [x] Validate CLI JSON output.

Observed artifact:

```text
/tmp/ict-hl/r24_vrp_sidecar.json
schema=qqq-nq-vrp-sidecar/v1
candidate=vrp_pressure_qqq_v1
rows=3
missing_policy=emit_missing_optional_and_continue
zero_config_fallback=True
last_vrp=7.0
last_pressure=7.899047619047619
last_confidence=1.0
bbn_targets=dealer_pressure,factor_uncertainty,crash_risk
```

### R25: OFI/session sidecar

- [ ] Add optional L2/trade-flow schema.
- [ ] Emit `ofi_book_pressure_v1`.
- [ ] Add OHLCV proxy mode with low confidence if L2 missing.

Status: paused after user correction. Do not continue sidecar-only work before proving regime-first strategy selection through Auto-Quant -> filter/analyze/regime -> BBN -> ranker -> execution tree.

### R26: BBN evidence value gate

- [x] Run managed Auto-Quant workspace, not just synthetic sidecar JSON.
- [x] Import a real Auto-Quant run manifest through `auto-quant-results-import`.
- [x] Apply strategy evidence through `auto-quant-prior-init` and verify `strategies_applied` is non-empty.
- [x] Admit factor evidence only if entropy/log-loss/contradiction lift improves.
- [x] Persist `bbn_entropy_reduction` and `bbn_log_loss_delta`.

Observed real closure slice:

```text
run_root=/tmp/ict-high-sharpe-real-20260509-234554
provider_status=market_data ready 5/7; yfinance+kraken_public ready; ibkr/tradingview_mcp pending
analyze_before=market_state TrendExpansion/BullTrendAcceleration; execution observe/transition_guardrail/guarded
Auto-Quant bootstrap=healthy pinned_ref=34ba6b6ee6aa69813a50a72158d4c089d97afb96
Auto-Quant prepare=data_ready true
Auto-Quant run log=/tmp/ict-high-sharpe-real-20260509-234554/logs/11_auto_quant_run.log
strategy_library=/tmp/ict-high-sharpe-real-20260509-234554/strategy_library_after_real_auto_quant_run_v3.json
import_artifact=auto_quant_strategy_library_NQ_20260509T155207.539452000Z
import_n_ok=2
prior_artifact=auto_quant_prior_init_NQ_20260509T155207.769438000Z
prior_initial=[0.999956,0.000022,0.000022]
prior_final=[0.6734197006771924,0.000000013279761567917304,0.326580286043046]
strategies_applied=MomentumMTFConfluence,RegimeAdaptiveBNB
MomentumMTFConfluence=854 trades, sharpe 0.3993, win_rate 34.7775, profit 53.24%, max_dd -23.1801%, pf 1.1682
RegimeAdaptiveBNB=115 trades, sharpe 0.1380, win_rate 69.5652, profit 16.41%, max_dd -4.6742%, pf 1.4262
```

Important: earlier manifest parse attempts v1/v2 produced `n_ok=0` and `strategies_applied=[]`; those are rejected evidence. v3 is the accepted run.

Observed live closure slice after user correction:

```text
run_root=/tmp/ict-high-sharpe-live-20260510-000946

provider matrix:
  yfinance provider-status: live_runtime 1/1 ready, market_data 1/1 ready
  yfinance actual fetch: QQQ 1h, 190 rows, success after one HTTP 429 retry
  kraken_public provider-status: market_data 1/1 ready
  kraken_public actual fetch: XBTUSD 1h, 721 rows, success
  ibkr provider-status: configured_runtime_unhealthy in default runtime because redis/ib_async missing, gateway reachable on port 4002
  ibkr actual fetch: QQQ 1h 30d, 480 rows via uv run --with redis --with ib_async --with pandas and gateway port 4002
  tradingview_mcp provider-status: install_required, missing ICT_ENGINE_TVREMIX_MCP_API_KEY
  tradingview_mcp actual fetch: attempted NASDAQ:QQQ, blocked by missing key; no TradingView data was used

Auto-Quant bootstrap:
  source=/Users/thrill3r/Auto-Quant
  managed_copy=/tmp/ict-high-sharpe-live-20260510-000946/auto-quant/auto-quant/.deps/auto-quant
  pinned_ref=34ba6b6ee6aa69813a50a72158d4c089d97afb96
  prepare=data_ready true
  run_exit=0
  run_log=/tmp/ict-high-sharpe-live-20260510-000946/logs/12_auto_quant_run.log
  manifest=/tmp/ict-high-sharpe-live-20260510-000946/strategy_library_from_live_run.json

Auto-Quant results:
  MomentumMTFConfluence full: 854 trades, sharpe 0.3993, win_rate 34.7775, profit 53.2400, max_dd -23.1801, pf 1.1682
  RegimeAdaptiveBNB full_5y: 115 trades, sharpe 0.1380, win_rate 69.5652, profit 16.4100, max_dd -4.6742, pf 1.4262
  RegimeAdaptiveBNB bull_2021: 16 trades, sharpe 0.3226
  RegimeAdaptiveBNB winter_2022: 25 trades, sharpe 0.2359
  RegimeAdaptiveBNB recovery_23_25: 72 trades, sharpe 0.0967

ict-engine filter/analyze:
  analyze_live_yfinance=TrendExpansion/BullTrendExhaustion; execution observe/transition_guardrail/guarded; gate pass_neutralized; quality 0.561
  analyze_demo_filter=TrendExpansion/BullTrendAcceleration; execution observe/transition_guardrail/guarded; gate pass_neutralized; quality 0.582
  persisted_paths=analyze_live htf/mtf/ltf/m1/m5/h4/spot JSONs under run_root/repo-state/NQ

BBN prior init:
  import_artifact=auto_quant_strategy_library_NQ_20260509T161635.677766000Z
  import_n_ok=2
  prior_artifact=auto_quant_prior_init_NQ_20260509T161711.472076000Z
  prior_initial=[0.999956,0.000022,0.000022]
  prior_final=[0.6734197006771924,0.000000013279761567917304,0.326580286043046]
  strategies_applied=MomentumMTFConfluence,RegimeAdaptiveBNB
  effects=854 trades -> 297 win/557 loss; 115 trades -> 80 win/35 loss
```

Observed R26 value-gate implementation slice:

```text
state_dir=/tmp/ict-r26-bbn-value-gate-20260510
source_manifest=/tmp/ict-high-sharpe-live-20260510-000946/strategy_library_from_live_run.json
source_log=/tmp/ict-high-sharpe-live-20260510-000946/logs/12_auto_quant_run.log
import_summary=/tmp/ict-r26-bbn-value-gate-20260510/import.json
import_n_ok=2
import_log_cross_check=matched 2, mismatches [], manifest_only [], log_only []
prior_summary=/tmp/ict-r26-bbn-value-gate-20260510/prior_init.json
prior_artifact=auto_quant_prior_init_NQ_20260509T171431.268234000Z
prior_state=/tmp/ict-r26-bbn-value-gate-20260510/auto-quant/NQ/auto_quant_prior_init_NQ_20260509T171431.268234000Z.json
prior_history=/tmp/ict-r26-bbn-value-gate-20260510/auto-quant/NQ/auto_quant_prior_init_history.json
evidence_value_gate_passed=true
bbn_entropy_reduction=0.018056766371967514
bbn_log_loss_delta=6.588649375209126
bbn_contradiction_lift=1.931483401354385
strategies_applied=MomentumMTFConfluence,RegimeAdaptiveBNB
strategies_skipped=[]
MomentumMTFConfluence gate=true entropy_reduction=0.0 log_loss_delta=6.348640598279494 contradiction_lift=1.292299795823666
RegimeAdaptiveBNB gate=true entropy_reduction=0.018056766371967514 log_loss_delta=0.24000877692963196 contradiction_lift=0.639183605530719
```

Implementation evidence:

```text
code=src/application/auto_quant/results/prior_init.rs
persistence=src/application/auto_quant/results/persistence.rs
summary_surface=src/application/auto_quant/command_entry.rs
test=cargo test --lib prior_init -- --nocapture
test_result=14 passed
test=cargo test --lib persistence -- --nocapture
test_result=46 passed
real_import=cargo run --quiet -- auto-quant-results-import --symbol NQ --state-dir /tmp/ict-r26-bbn-value-gate-20260510 --library /tmp/ict-high-sharpe-live-20260510-000946/strategy_library_from_live_run.json --log /tmp/ict-high-sharpe-live-20260510-000946/logs/12_auto_quant_run.log
real_prior_init=cargo run --quiet -- auto-quant-prior-init --symbol NQ --state-dir /tmp/ict-r26-bbn-value-gate-20260510
```

Boundary: this closes the BBN evidence-value admission/persistence gap for the current Auto-Quant strategy-library prior-init path. It does not promote the strategies to production; R27 still needs registered CatBoost runtime artifact support and mature scored rows.

### R27: path-ranker / execution-tree closure

- [x] Export target rows for promoted/probe candidates.
- [x] Apply external ranker scores and make contribution visible.
- [x] Enable registered ranker runtime artifact explicitly.
- [x] Require `workflow-status --human` to explain practical recommendation delta.
- [x] Train and apply actual CatBoost package against exported structural path target rows.
- [x] Feed CatBoost-produced raw scores back into ict-engine candidate-set runtime and verify `workflow-status --human` readback.
- [x] Add a supported registered CatBoost runtime companion path:
  - CatBoost `.cbm` stays as `model_artifact_uri` metadata.
  - Runtime artifact URI points to CatBoost-produced scored rows.
  - Weighted direct fallback remains `weighted_feature_sum_v1` and is not called CatBoost.
- [ ] Collect enough mature scored rows for production validation (`raw_scored_mature >= 30`).

Observed real closure slice:

```text
export_target=/tmp/ict-high-sharpe-real-20260509-234554/repo-state-v3/NQ/policy_training/structural_path_ranking_target.csv
rows=3
mature_rows=0
raw_scored_mature_before=0/30
trainer=/tmp/ict-high-sharpe-real-20260509-234554/path_ranker/trainer_artifact.json
catboost=not_installed
actual_model_family=weighted_feature_sum_v1
scores=/tmp/ict-high-sharpe-real-20260509-234554/path_scores.csv
rows_with_raw_path_score_after=3
runtime_selection=enabled_registered_model_ready
runtime_source=registered_model_artifact
runtime_matches=3
analyze_after=ranker registered_model_artifact/weighted_feature_sum_v1/not_ready
workflow_structural=trend_follow_through posterior=0.452 selected_prob=0.369
workflow_final_ranker=status using_registered_model_artifact source registered_model_artifact applied=3 artifact=3 lb=0.489 gate=observe
execution=observe/transition_guardrail/guarded
```

Boundary: this is a real command-level closure through Auto-Quant, filter/analyze, BBN, ranker registration, and execution tree. It is not a production high-confidence claim: CatBoost was unavailable, ranker used weighted fallback, and mature rows remain 0/30.

Observed live CatBoost / execution-tree slice after dependency install:

```text
run_root=/tmp/ict-high-sharpe-live-20260510-000946
export_target=/tmp/ict-high-sharpe-live-20260510-000946/repo-state/NQ/policy_training/structural_path_ranking_target.csv
target_rows=3
mature_rows=0
catboost_train=success
catboost_model=/tmp/ict-high-sharpe-live-20260510-000946/path_ranker_catboost/catboost_model.cbm
catboost_scores=/tmp/ict-high-sharpe-live-20260510-000946/path_scores_catboost.csv
features_used=structural_baseline_score fallback only
labels=pseudo-labels from structural_baseline_score because mature labels are unavailable
apply_scores=rows_with_raw_path_score 3
register_directory=failed Is a directory
register_cbm=failed stream did not contain valid UTF-8
register_json_companion_as_catboost=failed family mismatch source=weighted_feature_sum_v1
runtime_selection=enabled_candidate_set_ready
runtime_mode=candidate_set_only
runtime_source=candidate_set
runtime_matches=3
policy_status=raw_scored_mature 0/30; production_validation 0/30; observation_validation 0/30; calibration not_fitted
analyze_after=execution observe/transition_guardrail/guarded
workflow_ranker=status using_candidate_set_scores source candidate_set applied=3 artifact=0 candidate=3 raw=0.751 gate=n/a
```

Boundary correction: the 2026-05-10 slice did use the CatBoost dependency for training and scoring, but it did not produce a registered CatBoost runtime artifact. Runtime currently consumes candidate-set scores derived from CatBoost output.

Observed R27 registered CatBoost companion slice:

```text
run_root=/tmp/ict-r27-catboost-runtime-20260510021154
source_state=/tmp/ict-high-sharpe-live-20260510-000946/repo-state copied into isolated /tmp state
source_catboost_model=/tmp/ict-high-sharpe-live-20260510-000946/path_ranker_catboost/catboost_model.cbm
refreshed_target=/tmp/ict-r27-catboost-runtime-20260510021154/state/NQ/policy_training/structural_path_ranking_target.csv
candidate_set=structural-candidates:NQ:c5c555570db0f226
target_rows=1
catboost_apply_current=/tmp/ict-r27-catboost-runtime-20260510021154/path_scores_catboost_current.csv
catboost_score=trend_follow_through raw_path_score 0.862256
companion=/tmp/ict-r27-catboost-runtime-20260510021154/path_ranker_catboost/trainer_artifact.json
companion_model_family=catboost
companion_artifact_uri=/tmp/ict-r27-catboost-runtime-20260510021154/path_scores_catboost_current.csv
companion_model_artifact_uri=/tmp/ict-r27-catboost-runtime-20260510021154/path_ranker_catboost/catboost_model.cbm
register_cli=success
enable_cli=success
policy_status=runtime_selection enabled_registered_artifact_ready; runtime_source registered_artifact; runtime_matches 1
workflow_structural=using_registered_artifact_scores; source registered_artifact; applied 1; artifact 1; raw 0.862256
workflow_human_ranker=status using_registered_artifact_scores source registered_artifact applied=1 artifact=1 candidate=0 history=0 raw=0.862 gate=n/a
validation_boundary=raw_scored_mature 0/30, production_validation 0/30, observation_validation 0/30, calibration not_fitted
```

Implementation note: this is registered CatBoost score-row runtime consumption, not Rust-side `.cbm` inference. The binary model is recorded in the companion as metadata and remains optional/hot-plug; runtime defaults are unchanged.

### R28: Auto-Quant log cross-check parser hardening

- [x] Reproduce false drift from the live run where `SUMMARY` blocks reused strategy names and overwrote metric-bearing blocks.
- [x] Add RED regression for `SUMMARY` duplicate strategy names.
- [x] Add RED regression for same-strategy multi-timerange blocks where the manifest points at the full-window block.
- [x] Fix cross-check matching to choose the block that best matches the manifest status, timerange, and metrics.
- [x] Re-run parser tests.
- [x] Re-run live `auto-quant-results-import` against the same manifest and log in an isolated parser-check state dir.

Observed parser closure:

```text
original_import_log=/tmp/ict-high-sharpe-live-20260510-000946/logs/15_auto_quant_results_import.log
original_cross_check=matched 0, mismatches 4, false zero metrics from SUMMARY blocks
first_regression=SUMMARY duplicate selected log metrics 0 instead of full metrics
second_regression=RegimeAdaptiveBNB bull_2021 selected instead of manifest full_5y
test=cargo test --lib log_parser -- --nocapture
test_result=10 passed
live_recheck_state=/tmp/ict-high-sharpe-live-20260510-000946/repo-state-parser-check-2
live_recheck=matched 2, mismatches [], manifest_only [], log_only []
```

### R29: fresh end-to-end no-imagination chain rerun

- [x] Re-run provider status and actual provider fetches in a fresh `/tmp` root.
- [x] Re-bootstrap and prepare the managed Auto-Quant workspace from `/Users/thrill3r/Auto-Quant`.
- [x] Seed the managed workspace with archived Auto-Quant strategy files from its own versioned strategy folders.
- [x] Run Auto-Quant backtests, not a synthetic sidecar-only report.
- [x] Import the fresh strategy library through `auto-quant-results-import` with log cross-check.
- [x] Apply BBN prior initialization through `auto-quant-prior-init`.
- [x] Run `analyze-live` and `analyze --demo` to force the filter/analyze and execution-tree surfaces.
- [x] Export structural path target rows.
- [x] Train CatBoost from local `uv` cache, apply scores, register the CatBoost companion artifact, and enable runtime reuse.
- [x] Re-run analyze/workflow/policy status and inspect persisted `execution_tree_trace.json`.

Observed fresh chain:

```text
run_root=/tmp/ict-aq-filter-bbn-catboost-exectree-20260510022525

provider matrix:
  yfinance provider-status: live_runtime 1/1 ready, market_data 1/1 ready
  yfinance actual fetch: QQQ 1h, 210 rows -> provider-data/yf_QQQ_1h.csv
  kraken_public provider-status: market_data 1/1 ready
  kraken_public actual fetch: XBTUSD 1h, 721 rows -> provider-data/kraken_XBTUSD_1h.csv
  ibkr provider-status: configured_runtime_unhealthy in default runtime because redis/ib_async missing, gateway reachable on port 4002
  ibkr actual fetch: QQQ 1h 30d, 210 rows via uv run --with ib_async --with pandas --with redis, gateway port 4002 -> provider-data/ibkr_QQQ_1h_30d.csv
  tradingviewremix provider-status: install_required, missing ICT_ENGINE_TVREMIX_MCP_API_KEY
  tradingviewremix actual fetch: attempted NASDAQ:QQQ, blocked by missing key; no TradingViewRemix data was used

Auto-Quant bootstrap:
  source=/Users/thrill3r/Auto-Quant
  managed_copy=/tmp/ict-aq-filter-bbn-catboost-exectree-20260510022525/auto-quant/auto-quant/.deps/auto-quant
  pinned_ref=34ba6b6ee6aa69813a50a72158d4c089d97afb96
  prepare=data_ready true
  seed_source=managed versions/0.4.0/strategies/MomentumMTFConfluence.py and versions/0.4.1/strategies/RegimeAdaptiveBNB.py
  run_log=/tmp/ict-aq-filter-bbn-catboost-exectree-20260510022525/logs/06_auto_quant_run.log
  run_result=5 backtests succeeded, 0 failed

Auto-Quant results:
  MomentumMTFConfluence full: 854 trades, sharpe 0.3993, sortino 1.0760, calmar 2.4190, profit 53.2400%, max_dd -23.1801%, pf 1.1682
  RegimeAdaptiveBNB bull_2021: 16 trades, sharpe 0.3226, pf 2.4178
  RegimeAdaptiveBNB winter_2022: 25 trades, sharpe 0.2359, pf 1.7511
  RegimeAdaptiveBNB recovery_23_25: 72 trades, sharpe 0.0967, pf 1.2912
  RegimeAdaptiveBNB full_5y: 115 trades, sharpe 0.1380, profit 16.4100%, max_dd -4.6742%, pf 1.4262

Import and BBN:
  manifest=/tmp/ict-aq-filter-bbn-catboost-exectree-20260510022525/strategy_library_from_fresh_run.json
  import_artifact=auto_quant_strategy_library_NQ_20260509T183007.447791000Z
  import_n_ok=2
  log_cross_check=matched 2, mismatches [], manifest_only [], log_only []
  prior_artifact=auto_quant_prior_init_NQ_20260509T183007.498689000Z
  evidence_value_gate_passed=true
  prior_initial=[0.999956,0.000022,0.000022]
  prior_final=[0.6734197006771924,0.000000013279761567917304,0.326580286043046]
  bbn_entropy_reduction=0.018056766371967514
  bbn_log_loss_delta=6.588649375209126
  bbn_contradiction_lift=1.931483401354385
  strategies_applied=MomentumMTFConfluence,RegimeAdaptiveBNB
  strategies_skipped=[]

Filter/analyze and execution tree before ranker:
  analyze_live_yfinance=TrendExpansion/BullTrendExhaustion; execution observe/transition_guardrail/guarded; gate pass_neutralized; quality 0.568
  analyze_demo_filter=TrendExpansion/BullTrendAcceleration; execution observe/transition_guardrail/guarded; gate pass_neutralized; quality 0.582
  pre_bayes=gate pass_neutralized; soft_evidence=yes; long 0.554 short 0.534
  structural_target=3 rows, mature_rows 0, rows_with_raw_path_score 0

CatBoost and execution tree after ranker:
  first_catboost_attempt=failed PyPI TLS fetch; not accepted as evidence
  catboost_train=success using local uv offline cache
  catboost_model=/tmp/ict-aq-filter-bbn-catboost-exectree-20260510022525/path_ranker_catboost/catboost_model.cbm
  catboost_scores=/tmp/ict-aq-filter-bbn-catboost-exectree-20260510022525/path_scores_catboost.csv
  score_rows=3
  top_score=trend_follow_through raw_path_score 0.7506586567765241
  apply_scores=rows_with_raw_path_score 3, history_rows_with_raw_path_score 3
  register_runtime=enabled_registered_artifact_ready
  runtime_source=registered_artifact
  runtime_matches=3
  policy_status=raw_scored_mature 0/30; production_validation 0/30; observation_validation 0/30; calibration not_fitted
  analyze_after=market_state TrendExpansion/BullTrendAcceleration; execution observe/transition_guardrail/guarded; ranker registered_artifact/catboost/not_ready
  execution_tree_trace=path_ranker_model_family catboost, path_ranker_runtime_source registered_artifact, ranker_validation_ready false
```

Boundary: this proves the current candidate chain was actually driven through provider probing, Auto-Quant, filter/analyze, BBN prior initialization, CatBoost score production, registered ranker runtime, and execution-tree readback. It does not promote either strategy to production: TradingViewRemix is still key-blocked, NQ structural rows are immature, CatBoost labels are pseudo-label/fallback-only, and execution remains `observe/transition_guardrail/guarded`.

### R30: R27 mature-row harvest attempt and blocker

- [x] Re-use the fresh provider matrix in a new `/tmp` root instead of assuming data availability.
- [x] Fetch/attempt every requested provider lane: IBKR, TradingViewRemix, YF/yfinance, and Kraken.
- [x] Replay IBKR candles through the real structural feedback harness.
- [x] Seed a second replay from the IBKR state and replay YF candles through the same harness.
- [x] Train CatBoost from the mature target history, apply CatBoost scores, register the CatBoost companion artifact, and re-read runtime/policy status.
- [ ] Unblock target-row production validation: either add a real observation/run identity to structural path-ranking target history, widen candidate/path generation enough to produce `raw_scored_mature >= 30`, or do both with tests.

Observed mature-row harvest:

```text
run_root=/tmp/ict-r27-mature-row-harvest-20260510023136

provider matrix:
  yfinance actual fetch: QQQ 1h, 344 rows after one HTTP 429 retry -> provider-data/yf_QQQ_1h.csv
  kraken_public actual fetch: XBTUSD 1h, 721 rows -> provider-data/kraken_XBTUSD_1h.csv
  ibkr actual fetch: QQQ 1h 45d, 720 rows via uv run --with redis --with ib_async --with pandas, gateway port 4002 -> provider-data/ibkr_QQQ_1h_45d.csv
  tradingviewremix actual fetch: attempted NASDAQ:QQQ, blocked by missing ICT_ENGINE_TVREMIX_MCP_API_KEY; no TradingViewRemix data was used

replay_ibkr:
  command_surface=support/scripts/auto_quant_external/structural_feedback_replay_harness.py
  candles=candles/ibkr_QQQ_1h_45d.json
  observations=31
  final_current_mature_rows=1
  after_catboost_policy_status=raw_scored_mature 6/30; production_validation 6/30; observation_validation 31/30
  runtime_selection=enabled_registered_artifact_ready
  runtime_source=registered_artifact
  model_family=catboost

replay_yfinance_after_ibkr:
  seed_state=replay_ibkr/state copied forward
  candles=candles/yf_QQQ_1h.json
  additional_observations=31
  total_update_runs=62
  total_mature_observations=62/30
  final_policy_status=raw_scored_mature 6/30; production_validation 6/30; observation_validation 62/30
  runtime_selection=enabled_registered_artifact_ready
  runtime_source=registered_artifact
  model_family=catboost

target_history_diagnosis:
  history_jsonl=/tmp/ict-r27-mature-row-harvest-20260510023136/replay_yfinance_after_ibkr/state/NQ/policy_training/structural_path_ranking_target_history.jsonl
  history_rows=12
  unique_candidate_path_keys=12
  mature_scored_rows=6
  feedback_observations_total=62
  status_line=target_rows raw_scored_mature=6/30 production_validation=6/30 ready=false
  observation_line=observations mature=62/30 pending=0 total=62 ready=true
```

Root cause: more replay observations are reaching the feedback/policy layer, but they are not becoming distinct production-validation target rows. The canonical upsert owner is `src/belief_core/ranking_label.rs`; `structural_path_ranking_target_row_history_key` currently keys history rows as `candidate_set_id|path_id`, while the target row payload has no persisted observation/run timestamp. Repeated observations for the same candidate/path therefore overwrite the history row instead of adding independent supervised samples. Blind replay is now low yield until the target-row identity contract or candidate-path diversity is fixed.

Boundary: R27 is still open. This run proves actual operation through provider probing, replay feedback, filter/analyze state updates, CatBoost companion registration, and runtime readback, but it does not satisfy `raw_scored_mature >= 30` or production validation.

---

## Direction Pivot After R30

Current diagnosis:

- The chain is no longer blocked on "can we produce a high-Sharpe-looking candidate." The latest accepted evidence already shows one candidate with weak aggregate Sharpe but higher win rate (`RegimeAdaptiveBNB`) and one candidate with more trades but weaker win rate (`MomentumMTFConfluence`).
- The real blocker is selector correctness: if the regime layer is wrong or under-calibrated, the system cannot know which strategy should be enabled.
- Therefore, the next work should optimize for regime-conditioned strategy selection, not standalone Sharpe harvest.
- Sidecar factor work is still allowed, but only when it improves one of the selector inputs: regime identification, regime transition warning, strategy/regime match quality, or conditioned win-rate stability.

Decision lock:

- Do not rank candidates by aggregate Sharpe alone.
- Do not promote a strategy that lacks per-regime evidence, even if its aggregate metrics look acceptable.
- Do not continue blind replay only to increase observation count if the target-row identity contract still collapses repeated observations into too few production-validation rows.
- Treat `observe` as a valid output when the regime classifier is uncertain, transitioning, or mismatched against all available strategies.
- Keep Auto-Quant freedom: factors/strategies may be synthesized or hardcoded in the Auto-Quant workspace, but repo runtime changes must remain minimal and justified by a validated artifact contract.

---

## Next Implementation Queue

### R31: regime classifier quality gate

- [x] Define the minimum regime evidence needed before any strategy-selection claim:
  - regime label;
  - confidence / calibration score;
  - transition-risk flag;
  - lookback window and provider source;
  - known failure mode when labels conflict across providers or timeframes.
- [x] Run the regime classifier across the same provider matrix used in R29/R30:
  - yfinance / Yahoo;
  - IBKR;
  - Kraken for crypto cross-check where relevant;
  - TradingViewRemix if key becomes available.
- [x] Compare regime labels across timeframes:
  - intraday;
  - daily / swing context;
  - higher-timeframe trend context.
- [x] Emit a regime-quality artifact under `/tmp/<run_root>/repo-state/<SYMBOL>/...` and import/read it through existing repo surfaces rather than chat-only notes.
- [x] Add/extend tests only if a repo contract changes. A support/docs/runbook-only run does not need code tests.

Acceptance:

```text
regime_quality_artifact exists
regime_label is present
regime_confidence is present
transition_risk is present
provider/timeframe disagreements are visible
low-confidence regime produces observe/no-trade guidance
```

Observed R31 closure:

```text
source_run_root=/tmp/ict-r36-full-market-selector-20260510T023200Z
aggregate_artifact=/tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state/regime_quality_matrix.json
NQ_artifact=/tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state/NQ/regime_quality_artifact.json
BTCUSD_artifact=/tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state/BTCUSD/regime_quality_artifact.json
AAPL_artifact=/tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state/AAPL/regime_quality_artifact.json
provider_matrix_rows=10
providers=ibkr,kraken_public,tradingview_mcp,yfinance
timeframes=1d,1h
symbols=AAPL,BTCUSD,NQ
decision_states_seen=unknown_abstain
global_guidance=observe_no_trade
json_validation=passed for aggregate and all per-symbol artifacts
```

NQ readback:

```text
artifact_symbol=NQ
provider_timeframe_count=5
regime_label=""
all_confidence_95=false
all_confidence_99=false
all_trade_usable=false
max_transition_hazard=0.6
distributional_agreements_seen=disagree
low_confidence_guidance=observe_no_trade
known_failure_modes=confidence_95_failed,distributional_disagreement,distributional_transitional,high_distributional_distance,transitional_or_guardrailed,unknown_label,wide_conformal_set
repo_surface_readback=analyze --demo --regime-consumer-bundle /tmp/ict-r36-full-market-selector-20260510T023200Z/regime/yf_QQQ_1h/regime_consumer_bundle.json --state-dir /tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state
repo_surface_result=market_state TrendExpansion/BullTrendAcceleration; execution observe/transition_guardrail/guarded; plan no_trade_due_to_insufficient_edge
```

### R32: strategy-regime match matrix

- [x] Build a matrix for each candidate strategy:
  - enabled regimes;
  - disabled regimes;
  - uncertain regimes;
  - per-regime trade count;
  - per-regime win rate / hit rate;
  - win-rate lower confidence bound;
  - profit factor;
  - avg R:R;
  - max drawdown;
  - tail / CVaR check;
  - failure tags.
- [x] Re-score existing accepted strategies first:
  - `MomentumMTFConfluence`;
  - `RegimeAdaptiveBNB`.
- [x] Require sufficient trade density per enabled regime. A high win rate from a tiny regime slice is not enough.
- [x] Preserve aggregate Sharpe as a secondary diagnostic field, not a selector gate.
- [x] Store the matrix as an explicit artifact that downstream BBN/ranker/execution-tree steps can read.

Acceptance:

```text
strategy_regime_matrix exists
each strategy has enabled/disabled/uncertain regime sets
each enabled regime has win_rate_lcb and trade_count
aggregate Sharpe is present only as diagnostic
at least one strategy has a clear enable/disable recommendation by regime
```

Observed R32 closure:

```text
source_run_root=/tmp/ict-r36-full-market-selector-20260510T023200Z
strategy_regime_matrix=/tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state/strategy_regime_matrix.json
json_validation=passed
strategies=MomentumMTFConfluence,RegimeAdaptiveBNB
provider_matrix_rows=10
providers=ibkr,kraken_public,tradingview_mcp,yfinance
timeframes=1d,1h
symbols=AAPL,BTCUSD,NQ
current_regime=unknown_abstain
current_regime_trade_usable=false
matrix_recommendation=observe_no_trade
selected_strategy=none
aggregate_sharpe_role=diagnostic_only
```

Strategy-regime decision summary:

```text
MomentumMTFConfluence:
  enabled_regimes=[]
  disabled_regimes=unknown_abstain
  aggregate_diagnostics: trades=854, win_rate=34.7775, win_rate_lcb=31.6578, pf=1.1682, max_dd=-23.1801, sharpe_diagnostic_only=0.3993
  reason=low hit rate and no current reliable regime; do not promote on aggregate Sharpe.

RegimeAdaptiveBNB:
  enabled_regimes=[]
  disabled_regimes=unknown_abstain
  historical/probe rows:
    bull_2021: trades=16, win_rate=81.25, win_rate_lcb=56.9906, pf=2.4178, max_dd=-4.1163
    winter_2022: trades=25, win_rate=68.0, win_rate_lcb=48.4099, pf=1.7511, max_dd=-2.3833
    recovery_23_25: trades=72, win_rate=68.0556, win_rate_lcb=56.6074, pf=1.2912, max_dd=-4.6744
    full_5y: trades=115, win_rate=69.5652, win_rate_lcb=60.6358, pf=1.4262, max_dd=-4.6742
  reason=hit-rate evidence is better than MomentumMTFConfluence, but current regime quality is unknown_abstain / trade_usable=false, so it remains disabled.

Missing optional fields:
  avg_rr=missing_optional
  tail_cvar_check=missing_optional
```

### R33: regime-first selector contract

- [ ] Define selector order:
  1. read current regime quality;
  2. reject if regime confidence is below threshold or transition risk is too high;
  3. load strategy-regime matrix;
  4. select only strategies enabled for the current regime;
  5. rank by conditioned win-rate/payoff/tail profile;
  6. pass selected strategy evidence to BBN/ranker/execution tree.
- [ ] Make the selector output explain why a strategy is enabled, disabled, or held in observe.
- [ ] Add a no-trade/observe path for:
  - unknown regime;
  - unstable regime transition;
  - provider/timeframe disagreement;
  - no strategy with acceptable conditioned win-rate/payoff.
- [ ] Keep zero-config behavior: missing optional strategy-regime artifacts should degrade to current observe/guarded behavior, not crash.

Acceptance:

```text
selector_output shows current_regime
selector_output shows regime_confidence
selector_output shows selected_strategy or observe reason
disabled strategies include human-readable regime mismatch reasons
missing optional matrix does not fail zero-config runtime
```

### R34: Auto-Quant loop target rewrite

- [ ] Change the Auto-Quant research prompt from "find high Sharpe" to "find regime-conditioned win-rate edges."
- [ ] Ask Auto-Quant to generate or mutate candidates that improve one of:
  - regime detection quality;
  - regime transition warning;
  - strategy/regime matching;
  - conditioned win-rate lower bound;
  - payoff profile inside the enabled regime.
- [ ] Keep candidate artifacts explicit:
  - required fields;
  - optional hot-plug fields;
  - enabled/disabled regimes;
  - missing optional policy;
  - expected BBN targets.
- [ ] Reject candidates that only improve aggregate Sharpe while weakening regime-conditioned win rate or selector clarity.

Acceptance:

```text
auto_quant_prompt mentions regime-conditioned win-rate target
candidate artifact includes enabled_regimes and disabled_regimes
candidate artifact includes win_rate_lcb or enough raw fields to compute it
candidate artifact includes fallback behavior for unknown regime
```

### R35: BBN and path-ranker evidence rewrite

- [ ] Feed BBN with regime-conditioned evidence, not only aggregate strategy metrics:
  - `regime_reliability`;
  - `strategy_regime_fit`;
  - `conditioned_win_rate_edge`;
  - `conditioned_payoff_quality`;
  - `transition_risk`;
  - `strategy_disabled_reason` when applicable.
- [ ] Make contradiction lift sensitive to regime mismatch:
  - high aggregate performance but wrong current regime should increase contradiction / uncertainty;
  - lower aggregate Sharpe but strong current-regime win-rate evidence may be admitted.
- [ ] Path-ranker targets should include regime identity and selector decision reason so mature rows do not collapse across materially different observations.
- [ ] Revisit the R30 target-row identity blocker before more replay:
  - current owner: `src/belief_core/ranking_label.rs`;
  - current issue: `candidate_set_id|path_id` can collapse repeated observations;
  - next fix should preserve observation/run identity if production-validation rows are meant to represent independent supervised samples.

Acceptance:

```text
BBN prior init reports regime-conditioned strategy evidence
path-ranker target rows include regime/selector context
raw_scored_mature growth is not blocked by repeated observation overwrite
workflow-status can explain regime mismatch vs selected strategy
```

### R36: full-market / full-timeframe validation pass

- [x] Validate the selector on more than NQ before claiming family-level success.
- [x] Include at least:
  - index futures / QQQ/NQ lane;
  - crypto lane via Kraken/public data;
  - one equity or ETF lane through yfinance/IBKR if data is available.
- [x] Run multiple timeframes for each lane and log disagreement, not just best-case results.
- [x] Keep provider attempts explicit. Do not call the lane data-blocked after one provider failure.
- [x] Produce a final table with:
  - symbol;
  - provider;
  - timeframe;
  - regime label/confidence;
  - selected strategy;
  - enabled/disabled reason;
  - conditioned win rate;
  - trade count;
  - payoff/tail status;
  - execution-tree action.

Acceptance:

```text
full_market_selector_table exists
provider attempts are enumerated
at least 3 market lanes are attempted
multiple timeframes are attempted per lane
failures are tagged as provider_blocked, regime_uncertain, strategy_mismatch, or insufficient_trade_density
```

---

### R37: real no-imagination chain rerun after user correction

- [x] Build current repo binary before running the chain:
  - `cargo build --bin ict-engine`
  - result: `Finished dev profile`
- [x] Create isolated run root:
  - `run_root=/tmp/ict-real-regime-selector-20260510T020632Z`
  - logs under `run_root/logs/`
  - provider data under `run_root/provider-data/`
  - repo state under `run_root/repo-state` and `run_root/structural-replay-36/state`
- [x] Actually attempt every requested provider lane:
  - YF/yfinance: `QQQ 1h`, 148 data rows -> `provider-data/yf_QQQ_1h.csv`
  - Kraken: `XBTUSD 1h`, 721 data rows -> `provider-data/kraken_XBTUSD_1h.csv`
  - IBKR: `QQQ STK 1 hour 30 D` via `uv run --with redis --with ib_async --with pandas` and gateway port `4002`, 480 data rows -> `provider-data/ibkr_QQQ_1h_30d.csv`
  - TradingViewRemix: actual `market-data-harness fetch` attempted for `NASDAQ:QQQ`; blocked at environment boundary because `ICT_ENGINE_TVREMIX_MCP_API_KEY` is unset. Evidence log: `logs/05_tradingviewremix_fetch_attempt.log`
- [x] Bootstrap and prepare managed Auto-Quant from the local checkout:
  - `auto-quant-bootstrap --repo-url /Users/thrill3r/Auto-Quant`
  - managed copy: `run_root/repo-state/auto-quant/.deps/auto-quant`
  - pinned ref: `34ba6b6ee6aa69813a50a72158d4c089d97afb96`
  - `auto-quant-prepare`: `data_ready=true`
- [x] Seed and run real Auto-Quant strategies from managed versioned strategy files:
  - `versions/0.4.0/strategies/MomentumMTFConfluence.py`
  - `versions/0.4.1/strategies/RegimeAdaptiveBNB.py`
  - command log: `logs/09_auto_quant_run.log`
  - run result: `5 backtests succeeded, 0 failed`
- [x] Import Auto-Quant output through ict-engine:
  - manifest: `strategy_library_from_current_auto_quant_run.json`
  - note: the managed copy did not contain canonical `export_strategy_library.py`; the manifest was parsed from the just-produced Auto-Quant run log, then validated by ict-engine against the same log.
  - `auto-quant-results-import`: `n_ok=2`, `log_cross_check matched=2`, `mismatches=[]`, `manifest_only=[]`, `log_only=[]`
  - library artifact: `auto_quant_strategy_library_NQ_20260510T021121.644863000Z`
- [x] Apply BBN prior initialization:
  - command log: `logs/11_auto_quant_prior_init.log`
  - `evidence_value_gate_passed=true`
  - `strategies_applied=MomentumMTFConfluence,RegimeAdaptiveBNB`
  - `bbn_entropy_reduction=0.018056766371967514`
  - `bbn_log_loss_delta=6.588649375209126`
  - `bbn_contradiction_lift=1.931483401354385`
- [x] Run regime/filter/analyze layer:
  - NQ source: `/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather`
  - extracted 2500 latest 15m candles:
    - `input/nq_auto_quant_15m_ohlcv.csv`
    - `input/nq_auto_quant_15m_candles.json`
    - range: `2025-11-20T18:45:00Z -> 2025-12-31T21:45:00Z`
  - regime sidecar bundle: `regime-sidecar/regime_consumer_bundle.json`
  - regime result: `decision_state=transitional`, `trade_usable=false`, `label_set=primary::TrendExpansion`, `execution_tree_hint=transition_guardrail`, `confidence_95=false`
  - `analyze-live` with yfinance and the regime bundle:
    - log: `logs/13_analyze_live_yfinance_with_regime.log`
    - result: `market_state=TrendExpansion/BullTrendExhaustion`, `execution=observe/transition_guardrail/guarded`, `gate=pass_neutralized`, `quality=0.561`
  - `analyze --demo` with the regime bundle:
    - log: `logs/14_analyze_demo_with_regime.log`
    - result: `market_state=TrendExpansion/BullTrendAcceleration`, `execution=observe/transition_guardrail/guarded`, `gate=pass_neutralized`, `quality=0.582`
- [x] Run structural replay through ict-engine update/export/analyze surfaces:
  - command log: `logs/15_structural_feedback_replay_36.log`
  - replay result: `observations=36`, `final_mature_rows=1`
  - state root: `structural-replay-36/state`
  - target summary: `state/NQ/policy_training/structural_path_ranking_target_summary.json`
  - target history: `history_rows=1372`, `history_mature_rows=1367`
- [x] Train and apply real CatBoost path ranker:
  - train log: `logs/16_catboost_train.log`
  - input: `structural_path_ranking_target_history.csv`
  - training samples: `1367`
  - model: `catboost-path-ranker/catboost_model.cbm`
  - selected feature: `structural_baseline_score`
  - apply log: `logs/17_catboost_apply_current.log`
  - score file: `path_scores_catboost_current.csv`
  - current score: `raw_path_score=0.5234782652629066`
- [x] Feed CatBoost output back into ict-engine runtime:
  - `apply-structural-path-ranking-external-scores`: `rows=1`, `history_rows_with_raw_path_score=1367`
  - companion artifact: `catboost-path-ranker/catboost_trainer_companion_scores.json`
  - `register-structural-path-ranking-trainer-artifact`: `trainer_artifact_model_family=catboost`, `runtime_artifact_match_count=1`
  - `enable-structural-path-ranking-runtime`: `runtime_selection=enabled_registered_artifact_ready`, `runtime_source=registered_artifact`, `runtime_matches=1`
  - validation: `raw_scored_mature=1367/30`, `production_validation=1367/30`, `observation_validation=36/30`
- [x] Verify execution-tree and workflow readback:
  - `workflow-status --refresh --human` log: `logs/22_workflow_status_refresh_human_after_catboost.log`
  - human ranker readback: `Ranker: status=using_registered_artifact_scores source=registered_artifact applied=1 artifact=1 raw=0.523`
  - `analyze --demo` after CatBoost runtime:
    - log: `logs/24_analyze_after_catboost_runtime.log`
    - result: `market_state=TrendExpansion/BullTrendAcceleration | execution=observe/transition_guardrail/guarded | ranker=registered_artifact/catboost/ready`
  - execution-tree trace: `structural-replay-36/state/NQ/execution_tree_trace.json`
  - trace confirms:
    - `path_ranker_score_visible_to_execution_tree=true`
    - `path_ranker_score_used_by_execution_tree=true`
    - `path_ranker_model_family=catboost`
    - `path_ranker_runtime_source=registered_artifact`
    - `ranker_validation_ready=true`

Observed strategy metrics from the real Auto-Quant run:

```text
MomentumMTFConfluence full:
  trades=854
  win_rate=34.7775
  sharpe=0.3993
  profit=53.2400
  max_dd=-23.1801
  pf=1.1682

RegimeAdaptiveBNB full_5y:
  trades=115
  win_rate=69.5652
  sharpe=0.1380
  profit=16.4100
  max_dd=-4.6742
  pf=1.4262
```

Prompt-to-artifact checklist:

```text
same TODO updated: yes, this R37 block
actual Auto-Quant operated: yes, logs/09_auto_quant_run.log and strategy_library_from_current_auto_quant_run.json
ict-engine filter/analyze operated: yes, logs/13_analyze_live_yfinance_with_regime.log and logs/14_analyze_demo_with_regime.log
belief network operated: yes, logs/11_auto_quant_prior_init.log and auto_quant_prior_init state artifacts
CatBoost operated: yes, logs/16_catboost_train.log, catboost_model.cbm, logs/17_catboost_apply_current.log
execution tree operated: yes, logs/24_analyze_after_catboost_runtime.log and structural-replay-36/state/NQ/execution_tree_trace.json
YF/yfinance used: yes, provider-data/yf_QQQ_1h.csv plus analyze-live yfinance path
Kraken used: yes, provider-data/kraken_XBTUSD_1h.csv
IBKR used: yes, provider-data/ibkr_QQQ_1h_30d.csv via gateway 4002
TradingViewRemix used/attempted: attempted, blocked by missing ICT_ENGINE_TVREMIX_MCP_API_KEY; see logs/05_tradingviewremix_fetch_attempt.log
repo pollution check: accidental catboost_info/ removed after CatBoost run
```

Boundary:

- This is a real operated chain, not a speculative plan.
- It still does not authorize a production trade. The regime sidecar says `transitional` / `trade_usable=false`, and the execution tree remains `observe/transition_guardrail/guarded`.
- TradingViewRemix is not counted as successful data coverage because the API key is absent in the current environment.
- The current CatBoost fit is real and runtime-consumed, but its selected feature is only `structural_baseline_score`; next work should add regime/selector context features before treating it as a high-quality strategy selector.

### R38: TradingViewRemix credential recovery and current-shell refresh

- [x] Rechecked the local credential owner before treating TradingViewRemix as data-blocked:
  - local credential file exists: `~/.ict-engine/tvremix_mcp.json`
  - file contains `api_key` and `url`
  - root cause of the latest block: current shell had no `ICT_ENGINE_TVREMIX_MCP_API_KEY`, and the local file's key value had diverged from the newly supplied key
  - fixed the local persisted credential without writing the secret into repo files
  - tightened local secret permissions: `~/.ict-engine` is `700`, `~/.ict-engine/tvremix_mcp.json` is `600`
- [x] Rebuilt the current repo binary:
  - command: `cargo build --bin ict-engine`
  - log: `/tmp/ict-tvremix-credential-refresh-20260510T022759Z/logs/00_cargo_build.log`
  - result: `Finished dev profile`
- [x] Re-ran TradingViewRemix with credentials injected only into the child process from the local credential file:
  - run root: `/tmp/ict-tvremix-credential-refresh-20260510T022759Z`
  - provider status artifact: `/tmp/ict-tvremix-credential-refresh-20260510T022759Z/provider-probes/tradingview_provider_status_agent.txt`
  - status: `market_data:1/1 ready`
  - reason: `mcp_url_and_api_key_available`
  - missing-key check: `missing_tradingview_mcp_api_key=false`
- [x] Re-ran the actual TradingViewRemix fetch:
  - command: `ict-engine market-data-harness --action fetch --market NQ --interval 1d --role etf_reference --provider etf_reference=tradingview_mcp --symbol-spec etf_reference=NASDAQ:QQQ`
  - output artifact: `/tmp/ict-tvremix-credential-refresh-20260510T022759Z/provider-probes/tradingview_qqq_1d_fetch.json`
  - summary artifact: `/tmp/ict-tvremix-credential-refresh-20260510T022759Z/provider-probes/tradingview_refresh_summary.json`
  - result: `ok=true`, provider `tradingview_mcp`, operation `ohlcv.fetch`
  - rows: `21`
  - range: `2026-04-10T13:30:00Z -> 2026-05-08T13:30:00Z`

Updated provider boundary after R38:

```text
TradingViewRemix current shell status: ready when child process loads ~/.ict-engine/tvremix_mcp.json
TradingViewRemix current fetch: succeeded, NASDAQ:QQQ 1d, 21 rows
R37 key-blocked note is now superseded for the current credentialed child-process lane
Do not call the provider matrix data-blocked while YF, Kraken, IBKR, and TradingViewRemix artifacts exist
```

### R39: R36 full-market / full-timeframe selector matrix pass

- [x] Produce `full_market_selector_table` as an artifact, not a chat-only table.
- [x] Attempt at least three market lanes:
  - index / QQQ-NQ lane;
  - crypto lane;
  - equity or ETF lane.
- [x] Attempt multiple timeframes per lane.
- [x] Enumerate provider evidence across YF/yfinance, Kraken, IBKR, and TradingViewRemix.
- [x] Run regime/analyze/selector surfaces where the current runtime supports the provider lane.
- [x] Tag unsupported or failed lanes explicitly as `provider_blocked`, `regime_uncertain`, `strategy_mismatch`, or `insufficient_trade_density`.

Live run root:

```text
/tmp/ict-r36-full-market-selector-20260510T023200Z
```

Provider and timeframe evidence:

```text
YF/yfinance:
  QQQ 1h: 148 rows, 2026-04-10 13:30:00+00:00 -> 2026-05-08 20:00:00+00:00
  QQQ 1d: 21 rows, 2026-04-10 13:30:00+00:00 -> 2026-05-08 13:30:00+00:00
  AAPL 1h: 148 rows, 2026-04-10 13:30:00+00:00 -> 2026-05-08 20:00:00+00:00
  AAPL 1d: 21 rows, 2026-04-10 13:30:00+00:00 -> 2026-05-08 13:30:00+00:00

Kraken:
  PF_XBTUSD 1h: 697 rows, 2026-04-10 00:00:00+00:00 -> 2026-05-09 00:00:00+00:00
  PF_XBTUSD 1d: 30 rows, 2026-04-10 00:00:00+00:00 -> 2026-05-09 00:00:00+00:00

TradingViewRemix:
  NASDAQ:QQQ 1h: 147 rows, 2026-04-10T13:30:00Z -> 2026-05-08T19:30:00Z
  NASDAQ:QQQ 1d: 21 rows, 2026-04-10T13:30:00Z -> 2026-05-08T13:30:00Z

IBKR:
  QQQ STK 1h 30D: 480 rows, 2026-03-27T08:00:00+00:00 -> 2026-05-08T23:00:00+00:00
  AAPL STK 1d 60D: 60 rows, 2026-02-12T00:00:00+00:00 -> 2026-05-08T00:00:00+00:00
```

Artifacts:

```text
full_market_selector_table_csv=/tmp/ict-r36-full-market-selector-20260510T023200Z/selector/full_market_selector_table.csv
full_market_selector_table_json=/tmp/ict-r36-full-market-selector-20260510T023200Z/selector/full_market_selector_table.json
summary=/tmp/ict-r36-full-market-selector-20260510T023200Z/selector/full_market_selector_summary.json
provider_data_dir=/tmp/ict-r36-full-market-selector-20260510T023200Z/provider-data
regime_bundle_dir=/tmp/ict-r36-full-market-selector-20260510T023200Z/regime
```

Selector table summary:

```text
row_count=10
market_lanes=crypto_xbtusd,equity_aapl,index_qqq_nq
providers=ibkr,kraken_public,tradingview_mcp,yfinance
timeframes=1d,1h
selected_strategies=none
failure_tags=provider_blocked+regime_uncertain,regime_uncertain,strategy_mismatch+regime_uncertain
```

Runtime/analyze notes:

```text
NQ/yfinance analyze-live succeeded:
  market_state=TrendExpansion/BullTrendExhaustion
  execution=observe/transition_guardrail/guarded
  gate=pass_neutralized
  quality=0.561

NQ/TradingViewRemix provider fetch succeeded, but analyze-live does not currently accept `tradingview_mcp` as a live auxiliary backend:
  tag=strategy_mismatch+regime_uncertain

AAPL/yfinance provider fetch succeeded, but generic equity analyze-live tried the futures-style `AAPL=F` path and failed:
  tag=provider_blocked+regime_uncertain
```

Decision:

```text
No strategy is promoted from this R36/R39 pass.
Every sidecar bundle returned latest_decision=unknown_abstain and trade_usable=false.
RegimeAdaptiveBNB remains an observed candidate with full_5y win_rate=69.5652, trades=115, pf=1.4262, max_dd=-4.6742, but it is disabled because the current regime gate is not reliable enough.
This validates the corrected direction: provider breadth is now present; the blocking problem is regime/selector confidence, not Sharpe hunting and not missing data.
```

### R40: TradingViewRemix local credential fallback fix

- [x] Root cause:
  - `provider-status` and `market-data-harness` treated the current process env as the only TradingViewRemix credential owner.
  - The real local owner already exists at `~/.ict-engine/tvremix_mcp.json`, but ict-engine did not read it unless an external wrapper injected `ICT_ENGINE_TVREMIX_MCP_API_KEY`.
  - Consumer impact: every fresh shell or child process could report `missing_tradingview_mcp_api_key` even when the key was correctly stored locally.
- [x] Code fix:
  - added a shared TradingViewRemix runtime config resolver:
    - env key/url first;
    - fallback to `~/.ict-engine/tvremix_mcp.json`;
    - fallback URL remains `https://tvremix.xyz/api/mcp/v1`;
    - secret value is never printed in provider status.
  - `provider-status` now marks `tradingview_mcp` ready when local config is present and the probe passes.
  - `market-data-harness fetch` now uses the same resolver, so consumers do not need to export the key manually in every shell.
  - legacy provider preference inference now also sees the local config instead of env only.
  - optional TradingViewRemix options-probe failure no longer kills OHLCV readiness; if OHLCV is usable but options smoke fails, the provider remains available for OHLCV and options lanes are degraded.
- [x] Regression test:
  - command: `cargo test --lib control_matrix_providers::tests::tradingview_provider -- --nocapture`
  - result: `4 passed`
  - covered behavior: local `~/.ict-engine/tvremix_mcp.json` with no env makes TradingViewRemix provider ready, and the secret remains redacted.
  - covered behavior: an options-probe failure with OHLCV still usable returns `ready_degraded` instead of making the whole provider unavailable.
- [x] Build:
  - command: `cargo build --bin ict-engine`
  - result: `Finished dev profile`
- [x] Real no-env consumer verification:
  - run root: `/tmp/ict-tvremix-local-config-fix-20260510T025728Z`
  - status command: `env -u ICT_ENGINE_TVREMIX_MCP_API_KEY -u ICT_ENGINE_TVREMIX_MCP_URL ict-engine provider-status --provider tradingview_mcp --agent`
  - status result: `market_data:1/1 ready`, `reason=mcp_url_and_api_key_available`
  - fetch command: `env -u ICT_ENGINE_TVREMIX_MCP_API_KEY -u ICT_ENGINE_TVREMIX_MCP_URL ict-engine market-data-harness --action fetch --market NQ --interval 1d --role etf_reference --provider etf_reference=tradingview_mcp --symbol-spec etf_reference=NASDAQ:QQQ`
  - fetch result: `ok=true`, `21` rows, `2026-04-10T13:30:00Z -> 2026-05-08T13:30:00Z`
- [x] Final rebuilt-binary no-env recheck after provider-support prompt cleanup:
  - run root: `/tmp/ict-tvremix-local-config-fix-after-prompt-20260510T030151Z`
  - status result: `market_data:1/1 ready`, provider `tradingview_mcp`, `reason=mcp_url_and_api_key_available`
  - fetch result: `ok=true`, `21` rows, `2026-04-10T13:30:00Z -> 2026-05-08T13:30:00Z`
- [x] Final rebuilt-binary no-env recheck after optional-options degradation fix:
  - run root: `/tmp/ict-tvremix-local-config-final-20260510T034555Z`
  - status artifact: `/tmp/ict-tvremix-local-config-final-20260510T034555Z/provider-probes/tradingview_provider_status_no_env.txt`
  - fetch artifact: `/tmp/ict-tvremix-local-config-final-20260510T034555Z/provider-probes/tradingview_qqq_1d_fetch_no_env.json`
  - summary artifact: `/tmp/ict-tvremix-local-config-final-20260510T034555Z/provider-probes/tradingview_no_env_summary.json`
  - status result: `market_data:1/1 ready`, provider `tradingview_mcp`, `ready=true`, `reason=mcp_url_and_api_key_available`
  - fetch result: `ok=true`, `21` rows, `2026-04-10T13:30:00Z -> 2026-05-08T13:30:00Z`

Consumer boundary after R40:

```text
TradingViewRemix key is no longer considered "missing" merely because the current shell lacks ICT_ENGINE_TVREMIX_MCP_API_KEY.
The canonical local fallback is ~/.ict-engine/tvremix_mcp.json.
Provider status and fetch now use the same credential chain.
Optional options smoke failures no longer hide usable OHLCV access from consumers.
Agents should still not write the secret into repo files or TODO docs.
```

### R41: branch-specific factor priority queue

- [x] Direction lock: the next Auto-Quant loop should generate / backtest branch-specific factor evidence under the current regime before feeding BBN and CatBoost/path-ranking.
- [x] Range mean-reversion viability bucket:
  - Bollinger / ATR stretch;
  - distance from VWAP / session midpoint;
  - prior sweep + failed continuation;
  - chop / volatility compression vs expansion.
- [x] Transition confirmation bucket:
  - range break with displacement;
  - volume expansion after compression;
  - multi-timeframe alignment persistence;
  - failed mean-reversion after boundary break.
- [x] Stress de-risk bucket:
  - volatility spike;
  - liquidity thinning;
  - wide-range continuation hazard;
  - FOMO / crowding risk.
- [x] Chain lock: these buckets should become Auto-Quant factor/candidate evidence first, then feed BBN evidence and CatBoost/path-ranker features. Do not turn the next slice into another pseudo-label model training endpoint.
- [x] Downstream mapping:
  - range mean-reversion viability -> selector evidence for `wait_for_reversion` vs disabled continuation; BBN targets `reversion_viability`, `factor_uncertainty`, and `liquidity_context`;
  - transition confirmation -> selector evidence for enabling transition / continuation branches; BBN targets `transition_confirmed`, `evidence_quality`, and `mtf_alignment`;
  - stress de-risk -> observe / no-trade / size-reduction evidence; BBN targets `crash_risk`, `crowding_pressure`, and `liquidity_risk`.
- [ ] Next safe Auto-Quant slice: create an external feature pack or candidate-spec artifact outside the Rust runtime that emits all three buckets with `enabled_regimes`, `disabled_regimes`, BBN targets, and CatBoost/path-ranker feature fields.
- [ ] Backtest order:
  1. start from the current NQ/QQQ `15m/1h/1d` provider matrix proven in R39/R40;
  2. test range mean-reversion only when regime quality supports range / chop / transition-guardrail context;
  3. test transition confirmation when compression -> break evidence appears;
  4. run stress de-risk as veto / size-reduction evidence, not as standalone entry alpha.
- [ ] Promotion gate: require conditioned win-rate/payoff inside enabled regimes plus BBN value lift and CatBoost/readback feature contribution. Aggregate Sharpe and pseudo-label accuracy remain diagnostics only.

Slice boundary: this R41 update is a docs-only queue capture. No runtime source was modified for this slice.

---

## Verification floor

```bash
git status --short
python3 support/scripts/research/factor_formula_seed_library.py --output /tmp/ict-hl/factor_seed_candidates.json
python3 -m json.tool /tmp/ict-hl/factor_seed_candidates.json >/dev/null
cargo check
./target/debug/ict-engine analyze --demo --symbol NQ --state-dir /tmp/ict-hl-smoke --human
./target/debug/ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-hl-smoke --human
```

---

## Files in this slice

This direction-correction slice should stage only:

- `support/docs/plans/2026-05-09-high-sharpe-factor-harvest-handoff-todo.md`

Unrelated dirty files remain outside this slice.
