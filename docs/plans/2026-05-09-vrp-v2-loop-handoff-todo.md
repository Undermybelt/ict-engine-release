# 2026-05-09 VRP V2 Loop Handoff TODO

> Live handoff for the loop that explicitly pushed one candidate through factor artifact -> pre-bayes -> BBN -> CatBoost/path-ranking -> execution-tree. Keep this doc command/evidence focused and `/tmp/...` isolated.

## Scope

- Candidate: `VRPCompression_V2_NQ_15m`
- State dir: `/tmp/vrp-v2-loop-20260509`
- Candidate pack: `/tmp/vrp-v2-loop-20260509-candidate-pack/`
- Manifest: `/tmp/vrp_v2_strategy_library.json`
- Realized trades: `/tmp/vrp_v2_realized_trades.jsonl`
- Candidate spec: `/tmp/vrp-v2-loop-20260509-candidate-spec.json`
- Runtime boundary: no `ict-engine` runtime source changes for this slice.
- Repo pollution boundary: generated data stays under `/tmp`; this doc records paths and results only.

## Done

- [x] Created explicit candidate spec with downstream targets:
  - `pre_bayes_targets`
  - `belief_targets`
  - `path_ranking_targets`
  - `execution_tree_targets`
  - `structural_feedback_required=true`
- [x] Generated candidate pack with:
  - `/tmp/vrp-v2-loop-20260509-candidate-pack/factor_expression.json`
  - `/tmp/vrp-v2-loop-20260509-candidate-pack/factor_eval_grid_summary.json`
  - `/tmp/vrp-v2-loop-20260509-candidate-pack/transfer_score.json`
- [x] Ran pre-bayes status from the same isolated state:
  - command: `/Users/thrill3r/projects-ict-engine/ict-engine/target/debug/ict-engine pre-bayes-status --symbol NQ --state-dir /tmp/vrp-v2-loop-20260509 --human`
  - result: `gate=pass_neutralized`, `soft_evidence=yes`
  - bridge: `long=0.551`, `short=0.530`, `mtf=bullish`, `align=1.000`, `entry_align=0.860`
- [x] Ran BBN prior-init dry-run:
  - command: `/Users/thrill3r/projects-ict-engine/ict-engine/target/debug/ict-engine auto-quant-prior-init --symbol NQ --state-dir /tmp/vrp-v2-loop-20260509 --library /tmp/vrp_v2_strategy_library.json --dry-run`
  - strategy: `VRPCompression_V2_NQ_15m`
  - trades: `815` (`n_win=277`, `n_loss=538`, `n_breakeven=0`)
  - prior moved from `[0.34275405214176413, 0.001986731343802505, 0.6552592165144334]` to `[0.33990526417634764, 0.00001931209082675582, 0.6600754237328257]`
- [x] Checked realized-trade ingestion dry-run:
  - command: `/Users/thrill3r/projects-ict-engine/ict-engine/target/debug/ict-engine auto-quant-ingest-real-trades --symbol NQ --state-dir /tmp/vrp-v2-loop-20260509 --trades /tmp/vrp_v2_realized_trades.jsonl --dry-run`
  - result: refused duplicate content hash `fc131fe2cce235f5`
  - interpretation: feedback already exists in the copied state; do not force-reingest without rolling back BBN state.
- [x] Exported structural path-ranking target:
  - command: `/Users/thrill3r/projects-ict-engine/ict-engine/target/debug/ict-engine export-structural-path-ranking-target --symbol NQ --state-dir /tmp/vrp-v2-loop-20260509`
  - rows: `3`
  - mature_rows: `0`
  - rows_with_raw_path_score: `3`
  - rows_with_calibrated_path_prob: `0`
  - rows_with_propensity_estimate: `1`
  - output CSV: `/tmp/vrp-v2-loop-20260509/NQ/policy_training/structural_path_ranking_target.csv`
- [x] Checked CatBoost/path-ranking status:
  - command: `/Users/thrill3r/projects-ict-engine/ict-engine/target/debug/ict-engine policy-training-status --symbol NQ --state-dir /tmp/vrp-v2-loop-20260509 --human`
  - result: `trainer_artifact=ready`, `trainer_status=present_validation_insufficient`, `runtime_selection=enabled_candidate_set_ready`, `runtime_mode=candidate_set_only`, `runtime_source=candidate_set`, `runtime_matches=3`
  - blocker: `mature_rows=0`, `raw_scored_mature=0/30`, `production_validation=0/30`, `calibration=not_fitted`
- [x] Checked execution-tree/workflow status:
  - command: `/Users/thrill3r/projects-ict-engine/ict-engine/target/debug/ict-engine workflow-status --symbol NQ --state-dir /tmp/vrp-v2-loop-20260509 --human`
  - result: `analyze | pass_neutralized`, `quality=0.607`
  - ranker: `status=using_candidate_set_scores`, `source=candidate_set`, `applied=3`, `artifact=0`, `candidate=3`, `raw=0.470`
- [x] Checked workflow phases:
  - `ensemble-vote`: `action=Observe`, `confidence=0.464`, `comparable=no_previous_run`
  - `structural-playbook`: selected branch `trend_follow_through`, posterior about `0.464`, candidate-set path ranking active
  - `structural-recommended-path-bundle`: `trend_follow_through`, `posterior=0.464`, `selected_prob=0.370`

## Chain Verdict

- Verdict: `stopped_at_path_ranking`
- Factor stage: candidate has explicit pack and dense NQ evidence (`815` trades) plus cross-market evidence recorded in candidate spec.
- Pre-bayes stage: alive and pass-neutralized, not blocking.
- BBN stage: prior-init alive; realized-trade feedback already present in copied state.
- CatBoost/path-ranking stage: current blocker. The runtime can use candidate-set scores, but mature external ranker closure is not validated because `mature_rows=0`, `raw_scored_mature=0/30`, and `production_validation=0/30`.
- Execution-tree stage: alive and readable, but still observes via candidate-set ranker rather than a mature external CatBoost loop.

## Next

- [x] Do not force-reingest `/tmp/vrp_v2_realized_trades.jsonl` into the copied state unless intentionally rolling back BBN feedback first.
  - kept intact; no forced historical trade reingest was used.
- [x] Use the generated structural target CSV to produce a hot-plug external CatBoost/direct ranker artifact, then re-run `policy-training-status` to see whether runtime selection moves beyond `candidate_set_only`.
  - command: `python3 scripts/auto_quant_external/path_ranker_integration.py --state-dir /tmp/vrp-v2-loop-20260509 --symbol NQ --register-runtime-artifact --reuse-mode candidate_set_only`
  - result: `runtime_selection=enabled_registered_model_ready`, `runtime_source=registered_model_artifact`, `runtime_matches=3`.
  - CatBoost package was not installed in the current Python, so zero-config fallback emitted and registered `path_ranker_direct_model.json` instead of requiring dependency installation.
- [x] Keep the ranker artifact optional/hot-pluggable: consumer can use candidate-set scoring with zero config, or opt into the external ranker when enough validation rows exist.
  - runtime mode remains explicit opt-in under `/tmp/vrp-v2-loop-20260509/NQ/policy_training/structural_path_ranking_runtime_selection.json`.
- [x] If mature rows remain zero, the next practical slice should generate or import structural feedback rows rather than tuning the trainer again.
  - added `emit-probe` mode to `scripts/auto_quant_external/structural_feedback_trade_enricher.py`, which turns a selected structural target row into an explicit `structural-feedback-v1` JSON for `ict-engine update --feedback-file`.
  - command: `python3 scripts/auto_quant_external/structural_feedback_trade_enricher.py emit-probe --target-csv /tmp/vrp-v2-loop-20260509/NQ/policy_training/structural_path_ranking_target.csv --output /tmp/vrp-v2-loop-20260509/structural_feedback_probe_rank1.json --rank 1 --realized-outcome win --pnl 0.03 --exit-reason manual_probe_target_hit --notes "explicit opt-in probe from structural path-ranking target lineage"`
  - command: `ict-engine update --symbol NQ --outcome win --entry-signal medium --state-dir /tmp/vrp-v2-loop-20260509 --pnl 0.03 --feedback-file /tmp/vrp-v2-loop-20260509/structural_feedback_probe_rank1.json`
  - result after `export-structural-path-ranking-target`: `mature_rows=1`, `history_mature_rows=1`, `training_weight_rows=1`.
  - result after `policy-training-status`: `raw_scored_mature=1/30`, `production_validation=0/30`, `runtime_source=registered_model_artifact`.
- [x] Keep all generated ranker experiments under `/tmp/...` or explicit caller-owned state dirs; do not write model artifacts to repo root.
  - generated probe, model, scores, runtime selection, and target exports stayed under `/tmp/vrp-v2-loop-20260509`.
- [x] Next threshold: collect or replay 29 more honest structural-feedback observations before claiming external ranker validation (`raw_scored_mature >= 30`).
  - replay harness: `scripts/auto_quant_external/structural_feedback_replay_harness.py`
  - candles: `/Users/thrill3r/Downloads/Tomac/ict-cleaned-15m/nq.continuous-15m.json` (`28,909` cleaned 15m candles)
  - command: `python3 scripts/auto_quant_external/structural_feedback_replay_harness.py --candles /Users/thrill3r/Downloads/Tomac/ict-cleaned-15m/nq.continuous-15m.json --output-root /tmp/ict-engine-structural-replay-29 --symbol NQ --count 29 --lookback 52 --horizon 16 --threshold 0.001 --prior-state /tmp/vrp-v2-loop-20260509`
  - output summary: `/tmp/ict-engine-structural-replay-29/replay_summary.json`
  - state: `/tmp/ict-engine-structural-replay-29/state`
  - generated observations: `29` new semi-auto observations plus the prior one already in `/tmp/vrp-v2-loop-20260509`
  - `learning_state.feedback_history`: `30` structural-feedback records total; outcomes `loss=14`, `win=12`, `breakeven=4`; source `structural_feedback_submission=30`
  - each observation ran `ict-engine analyze`, `export-structural-path-ranking-target`, external ranker scoring, `apply-structural-path-ranking-external-scores`, `structural_feedback_trade_enricher.py emit-probe`, and `ict-engine update --feedback-file` on a replayed historical candle window.
  - initial blocker found: old `policy-training-status` reported `raw_scored_mature=2/30`, `production_validation=2/30`, because the engine validation counter was row-based over de-duplicated `structural_path_ranking_target_history.jsonl`, not observation-based over repeated `feedback_history` records.
  - code slice completed: `policy-training-status` now reports target-row validation and structural-feedback observation validation separately.
  - verified status on `/tmp/ict-engine-structural-replay-29/state`: `raw_scored_mature=2/30`, `production_validation=2/30`, `observation_validation=30/30`, `ready=true`.
  - honest conclusion: the requested 29 observations exist and are traceable; row-level target validation remains transparent at `2/30`, while observation-level structural-feedback validation is now explicitly `30/30`.
- [x] Next runtime/code slice: add an observation-level structural path-ranking evaluation export, or change validation status to count eligible structural-feedback observations separately from de-duplicated target rows.
  - implemented as observation validation status fields, not as inflated target rows.

## 2026-05-10 Current-Shell Refresh

- [x] Rebuilt and used the current repo binary:
  - command: `cargo build --bin ict-engine`
  - result: dev build completed successfully.
- [x] Rechecked the same replay state after structural target re-export:
  - command: `ict-engine export-structural-path-ranking-target --symbol NQ --state-dir /tmp/ict-engine-structural-replay-29/state`
  - result: `rows=1`, `history_rows=35`, `mature_rows=1`, `history_mature_rows=33`, `history_rows_with_raw_path_score=35`, `history_rows_with_calibrated_path_prob=33`, output under `/tmp/ict-engine-structural-replay-29/state/NQ/policy_training/`.
- [x] Rechecked CatBoost/path-ranker status from the same state:
  - command: `ict-engine policy-training-status --symbol NQ --state-dir /tmp/ict-engine-structural-replay-29/state --human`
  - result: `raw_scored_mature=33/30`, `production_validation=33/30`, `observation_validation=30/30`, `trainer_status=runtime_eligible`, `runtime_selection=enabled_registered_model_ready`, `runtime_source=registered_model_artifact`, `runtime_matches=1`.
  - correction to older note: the earlier `raw_scored_mature=2/30` / `production_validation=2/30` status is stale after the current re-export; the current replay state now passes the 30-row validation floor.
- [x] Rechecked pre-bayes / filter gate:
  - command: `ict-engine pre-bayes-status --symbol NQ --state-dir /tmp/ict-engine-structural-replay-29/state --human`
  - result: `gate=pass_neutralized`, `soft_evidence=yes`, `long=0.547`, `short=0.544`, `mtf=bullish`, `align=0.751`, `entry_align=0.883`.
- [x] Rechecked BBN prior-init effect:
  - command: `ict-engine auto-quant-prior-init --symbol NQ --state-dir /tmp/ict-engine-structural-replay-29/state --library /tmp/vrp_v2_strategy_library.json --dry-run`
  - result: `trade_count=815`, `n_win=277`, `n_loss=538`, `evidence_value_gate_passed=true`, probabilities moved from `[0.34275405214176413, 0.001986731343802505, 0.6552592165144334]` to `[0.33990526417634764, 0.00001931209082675582, 0.6600754237328257]`.
- [x] Rechecked execution-tree / workflow readback:
  - command: `ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-engine-structural-replay-29/state --human`
  - result: latest analyze remains `gate=pass_neutralized`, `quality=0.593`; ranker reads `status=using_registered_model_artifact`, `source=registered_model_artifact`, `applied=1`, `lb=0.521`, `gate=pass`.
  - command: `ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-engine-structural-replay-29/state --phase ensemble-vote --human`
  - result: `action=execute_follow_through`, `confidence=0.976`.
  - command: `ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-engine-structural-replay-29/state --phase structural-recommended-path-bundle --human`
  - result: `trend_follow_through`, `posterior=0.787`, `selected_prob=1.000`.
  - trace: `/tmp/ict-engine-structural-replay-29/state/NQ/execution_tree_trace.json`
  - trace output: `branch=transition_guardrail`, `execution_bias=guarded`, `gate_status=observe`, `execution_score=0.5736691669503992`, `decision_hint=execution_guarded_due_to_low_remaining_regime_duration`.
- [x] Rechecked requested provider lanes in the current process and saved fetch artifacts under `/tmp/ict-current-provider-probe-20260510/provider-probes/`:
  - provider catalog: `provider-status --agent` reports `entry_model:2/2 ready | live_runtime:1/3 ready | local_runtime:1/2 ready | market_data:5/7 ready`.
  - yfinance status: ready for live runtime and market data.
  - yfinance fetch: `python3 scripts/auto_quant_external/fetch_external.py yahoo --symbol QQQ --interval 1h --start 2026-04-25 --end 2026-05-10 --output /tmp/ict-current-provider-probe-20260510/provider-probes/yf_QQQ_1h.csv`; result `71` data rows after one HTTP 429 retry.
  - Kraken status: `kraken_cli` ready and `kraken_public` ready through provider catalog.
  - Kraken fetch: `python3 scripts/auto_quant_external/fetch_external.py kraken-kline --market futures --pair PF_XBTUSD --interval 1h --start 2026-04-25 --end 2026-05-10 --output /tmp/ict-current-provider-probe-20260510/provider-probes/kraken_pf_xbtusd_1h.csv`; result `360` data rows.
  - IBKR status: plain repo runtime still reports `configured_runtime_unhealthy` because `redis` is missing, but local gateway `127.0.0.1:4002` is reachable.
  - IBKR fetch: `uv run --offline --with redis --with ib_async --with pandas python scripts/auto_quant_external/fetch_external.py ibkr-historical --symbol SPY --sec-type STK --exchange SMART --currency USD --primary-exchange ARCA --bar-size '1 hour' --duration '10 D' --what-to-show TRADES --host 127.0.0.1 --port 4002 --client-id 25 --output /tmp/ict-current-provider-probe-20260510/provider-probes/ibkr_SPY_1h_10d.csv`; result `160` data rows.
  - TradingViewRemix status: current process is blocked by missing `ICT_ENGINE_TVREMIX_MCP_API_KEY`.
  - TradingViewRemix fetch attempt: `ict-engine market-data-harness --action fetch --market NQ --interval 1d --role etf_reference --provider etf_reference=tradingview_mcp --symbol-spec etf_reference=NASDAQ:QQQ`; result `fetch_failed: ICT_ENGINE_TVREMIX_MCP_API_KEY must be set for tradingview_mcp`.

## Current Chain Verdict After Refresh

- Verdict: `reached_execution_tree_registered_ranker_observe_guarded`
- Factor stage: `VRPCompression_V2_NQ_15m` remains explicit and dense on NQ (`815` trades), with candidate pack artifacts under `/tmp/vrp-v2-loop-20260509-candidate-pack/`.
- Pre-bayes / filter stage: alive, `pass_neutralized`, not blocking.
- BBN stage: alive; prior-init dry-run still produces measurable posterior movement and passes the evidence-value gate.
- CatBoost/path-ranking stage: current replay state now passes the validation floor (`raw_scored_mature=33/30`, `production_validation=33/30`, `observation_validation=30/30`) and workflow reads the registered ranker artifact.
- Execution-tree stage: reached and readable; ensemble wants `execute_follow_through`, structural path bundle selects `trend_follow_through`, but final execution trace remains `transition_guardrail` / `guarded` / `observe`.
- Current blocker: not provider absence and not CatBoost attachment. The live blocker is execution-readiness / remaining-regime-duration gating: `execution_readiness=0.4648`, `hybrid_transition_hazard=0.607`, `duration_remaining_expected_bars=0.667`, with `decision_hint=execution_guarded_due_to_low_remaining_regime_duration`.
- Provider boundary: yfinance, Kraken, and IBKR were physically fetched in this current shell; TradingViewRemix was physically attempted and is credential-blocked in this process. Do not claim `data_blocked` while these current/provider artifacts exist.

## Drift Check

- Scope: still serving the original loop request: factor -> pre-bayes/filter -> BBN -> CatBoost/path-ranking -> execution tree.
- Compatibility: zero-config remains intact; the current refresh is docs/evidence only.
- Pollution: generated artifacts are `/tmp` only; repo receives this handoff doc.
- Decision: path-ranking maturity is no longer the immediate stop layer for this replay state; continue by improving execution-readiness / temporal-duration / transition-guardrail evidence, while keeping the provider matrix broad.
