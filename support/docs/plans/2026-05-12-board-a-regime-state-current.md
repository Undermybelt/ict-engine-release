# Board A Regime State Current

Updated: `2026-05-14 00:43:06 +0800`

Purpose: clean current authority for Board A. Live regime-confidence behavior
belongs in `ict-engine` runtime code, state artifacts, provider configs, and
compact evidence packets. Historical append-only prose is not a planning source,
status source, next-step source, or completion source.

Recovered 95+ confidence asset ledger:
`support/docs/plans/2026-05-12-95-confidence-asset-recovery.md`.

## Reader Contract

This file is the default Board A entrypoint. Do not enter historical logs during
normal iteration.

Use runtime/state surfaces first. Board A status updates must be terminal
decisions only: `keep`, `drop`, `incubate`, `blocked`, or `handoff`, with compact
evidence paths. Routine coordination and start claims must not be appended here.

Historical material may be opened only by exact artifact id, exact heading, or
exact evidence path when this file names a gap. It may never supply a new task,
claim, acceptance gate, route, or Board A/B boundary.

Do not lose or delete the recovered 95+ confidence assets just because they are
not Board B profit-factor candidate packs. Use the recovered asset ledger as the
Board A preservation index until a native regime-confidence entrypoint replaces
it.

## Role

Board A owns market-state identification and regime-confidence evidence. It does not own profitability-factor promotion. Board B may consume Board A regime context, but Board A remains responsible for whether a regime state is sufficiently calibrated and portable.

## Parallel Boundary

- Expected concurrency is high: assume about 20 agents may work Board A while
  about 20 agents work Board B. Agents must not use this board as a shared lock
  table or scratchpad.
- Start claims are ephemeral: use `/tmp/ict-engine-agent-claims/board-a/` or an
  equivalent process-local lock outside the repo. Do not create claim docs,
  claim rows, TODO sprawl, or sibling plan files in the repo for starting work.
- Repo writes are allowed only for durable terminal evidence: compact run-root
  packets and one current-board decision row after the idea has evidence.
- Board A work can run in parallel with Board B only when it stays on regime
  posterior, confidence, calibration, provider/context validation, and recovered
  regime-confidence assets.
- Board A must not claim, repair, promote, reject, or rerun Board B
  profitability-factor roots, Auto-Quant recipe packets, SMT factor training
  packets, or candidate-pack admission targets.
- Board B may read a frozen Board A context label as input, but that read does
  not transfer ownership of regime confidence or posterior repair.

## Idea Collision Rule

- If the chosen Board A idea is already claimed, active, done, or blocked, do not
  continue, repair, re-run, summarize, or "help" that idea.
- Write only a compact `duplicate_suppressed` or no-takeover note if useful,
  then pick a new unclaimed Board A idea from `Live Gates`.
- A new idea must be different by at least one real ownership axis: gate,
  symbol/instrument set, provider/window, artifact root, or calibration target.
- Do not create a sibling readback just to stay busy. If no unclaimed Board A
  idea exists, hand off the blocker in this file and stop.

## Code/Docs Isolation

- This file is not a runtime input. No Rust, Python, shell, provider, Auto-Quant,
  training, or workflow code may import, parse, grep, or depend on this markdown.
- Runtime state must live in code, configs, explicit state directories, provider
  manifests, candidate packs, JSON/CSV/JSONL artifacts, or CLI output.
- Markdown may cite artifact paths and commands for humans/agents, but it must
  not become a hidden dependency, router, data source, fixture, or model input.
- If code needs a rule currently written only here, promote that rule into a
  typed config, command flag, schema, or test fixture first; do not read the doc.

## Current Contract

- Use provider-backed broad-window evidence first.
- Local long-history data may be used for training and diagnostics, but promotion requires provider-backed reproduction or a portable manifest with `local_raw_dependency=false`.
- Regime acceptance still requires calibrated confidence at the stated Board A threshold, per-regime qualifying conditions, cross-instrument validation, cross-period/timeframe validation, and provider/context validation.
- Aggregate Sharpe or downstream profitability is not a substitute for regime accuracy.
- If the regime is incomplete, expose posterior probabilities instead of forcing a single label.

## Current Status

- Board A is not complete.
- Handoff `2026-05-14 / codex-provider-runtime-deps-refresh-v1`: current provider dependency readback under `support/docs/experiments/actionable-regime-confidence/runs/20260514T004004+0800-codex-provider-runtime-deps-refresh-v1/` shows the default shell still uses `/opt/homebrew/bin/python3` `3.14.4` and reports `market_data:1/7 ready`, but the existing isolated provider venv `/Users/thrill3r/.venvs/ict-engine-provider-py313` already imports `requests`, `pandas`, `ccxt`, `ib_async`, `redis`, `yaml`, `sklearn`, `pyarrow`, and `xgboost`. With `PATH=/Users/thrill3r/.venvs/ict-engine-provider-py313/bin:$PATH`, `cargo run --quiet -- provider-status --compact` exits `0` with `local_runtime:2/2 ready` and `market_data:6/7 ready`; provider-specific compact readbacks mark `ibkr`, `kraken_public`, `binance_public`, and `bybit_public` ready. `tradingview_mcp` remains fail-closed with `configured_runtime_unhealthy:tradingview_mcp_connectivity_probe_failed`. Next provider-backed Board A chains should inject this venv on `PATH`; this does not close Board A because no new regime-confidence chain was run and TradingViewRemix remains unhealthy. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260514T004004+0800-codex-provider-runtime-deps-refresh-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Blocked `2026-05-14 / codex-board-a-current-objective-audit-v1`: built a current prompt-to-artifact checklist under `support/docs/experiments/actionable-regime-confidence/runs/20260514T003314+0800-codex-board-a-current-objective-audit-v1/` against the operator objective after the latest terminal rows. The audit confirms Board A is still not complete: no artifact proves every regime reaches 95% calibrated confidence; compact crossmarket evidence observed only `TrendExpansion` and `RangeConsolidation`; yfinance/Kraken/IBKR evidence remains below threshold; TradingViewRemix remains fail-closed; execution-tree evidence remains `observe`/`transition_guardrail`/`execution_blocked`; Auto-Quant evidence is failed/negative/timed out/seed-blocked/not promotable; and the latest `LOW_HAZARD_RECLAIM` CatBoost maturity is demo-readback only with a refreshed structural candidate-set mismatch. Fresh `cargo run --quiet -- provider-status --compact` exited `0` but current market-data readiness is only `1/7` (`yfinance` ready; `ibkr`, `kraken_public`, and `tradingview_mcp` unhealthy), so cross-provider validation is currently blocked by provider runtime readiness. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260514T003314+0800-codex-board-a-current-objective-audit-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Blocked `2026-05-14 / codex-low-hazard-current-candidateset-score-refresh-v1`: completed the active `LOW_HAZARD_RECLAIM` current candidate-set CatBoost score refresh under `support/docs/experiments/actionable-regime-confidence/runs/20260513T230523+0800-codex-low-hazard-current-candidateset-score-refresh-v1/`. Offline `uv --offline --with pandas --with numpy --with catboost` generated `derived/current_candidate_scores.csv`; apply/register/enable succeeded, and `policy-training-status` reports `runtime_selection=enabled_candidate_set_ready`, `runtime_mode=candidate_set_only`, `runtime_source=candidate_set`, `score_model_family=catboost`, `score_source=external_model`, `runtime_matches=3`, `raw_scored_mature=11329/30`, `production_validation=11329/30`, and `observation_validation=150/30` with feedback outcomes `win=57`, `loss=90`, `breakeven=3`. This still does not pass Board A: the post-rescore `analyze` readback used demo three-frame inputs (`support/examples/demo/demo-15m.json`), workflow stayed fail-closed/observe (`closed_loop_branch_admission.status=fail_closed`, `candidate_status=execution_observe_only`, `execution_readiness=0.4612333809537747`), execution triage stayed `transition_guardrail`, Pre-Bayes quality was only `0.5822867835012198`, and the refreshed structural bundle generated a new candidate set `structural-candidates:LOW_HAZARD_RECLAIM:b16b0ca24f8cb2a2` with nested `path_ranker_runtime.status=enabled_no_matching_scores` despite the exported target remaining CatBoost-ready. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T230523+0800-codex-low-hazard-current-candidateset-score-refresh-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Done `2026-05-13 / hermes-feature-wiring-impl-v1`: implemented stable mainline propagation for `consumer_hints.user_vrp_nq_context` fields (`qqq_hv_level`, `nq_vs_200d_pct`, `vix3m_level`, `qqq_hv_pct_rank_252`, `vvix_over_vix`) into read-only BBN/pre-Bayes assignments, structural path-ranking target CSV/JSONL feature columns, and execution-tree path-ranker lineage. Verification passed: `cargo fmt --check`, `cargo test test_analyze_command_persists_regime_bundle_branch_path_on_execution_candidate -- --nocapture`, and `git diff --check` on touched files. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T183800+0800-hermes-feature-wiring-impl-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Blocked `2026-05-13 / hermes-board-a-provider-matrix-v1`: fresh isolated Board A provider matrix and runtime readback under `support/docs/experiments/actionable-regime-confidence/runs/20260513T201826+0800-hermes-board-a-provider-matrix-v1/` used yfinance, TradingViewRemix, IBKR, Kraken, ICT Engine `analyze-live`, `workflow-status`, `pre-bayes-status`, `policy-training-status`, structural path target export, and Auto-Quant prepare/factor-research. It is not promotable: yfinance QQQ 1d fetched `208` rows; Kraken PF_XBTUSD 1d fetched `378` rows; TradingViewRemix failed closed at `get_ohlcv` with provider status `configured_runtime_unhealthy`; IBKR was provider-ready but both 1Y harness and 60D direct historical fetch timed out/returned empty. Runtime stayed `observe/execution_blocked`: active regime `range`, posterior confidence `0.5455086354`, probabilities `range=0.7534400099 stress=0.1712159891 transition=0.075344001 trend=0.0`; path-ranker/CatBoost export had `rows=3`, `mature_rows=0`, `history_mature_rows=0`, `training_weight_rows=0`; Auto-Quant advanced from `dependency_ready_data_missing` to `data_ready=true` but remained blocked at `dependency_ready_seed_required`, and no Board B seed strategy was created. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T201826+0800-hermes-board-a-provider-matrix-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Blocked `2026-05-13 / hermes-board-a-crossmarket-regime95-validation-v1`: fresh isolated run under `support/docs/experiments/actionable-regime-confidence/runs/20260513T171059+0800-hermes-board-a-crossmarket-regime95-validation-v1/` executed provider fetches, high-confidence market-state validation, `analyze`, `workflow-status`, `pre-bayes-status`, `policy-training-status`, structural path target export, CatBoost score/apply/register/enable, execution-tree workflow readback, and managed Auto-Quant material dispatch/rank. Initial provider rows acquired: yfinance NQ 1d `1601`, NQ 1h `7770`, ES 1d `1601`, GC 1d `1600`, BTC 1h `11759`; Kraken PF_XBTUSD futures 1h `2000`, 1d `1513`. After operator correction to use denser IBKR/option evidence, reran IBKR via `uv run --with redis --with ib_async --with pandas`: QQQ 1m `5855`, 5m `5588`, 15m `3783`, 30m `3864`, 1d trades `251`, historical volatility `251`, option implied volatility `251`. IBKR validation still did not reach 95%: best QQQ 30m `avg_confidence=77.55%`, `high_confidence=66.32%`; QQQ 15m `77.12%/61.83%`; QQQ 5m `75.70%/56.83%`; QQQ 1m `74.81%/50.17%`. IBKR runtime after CatBoost stayed `Observe`, active `trend`, posterior confidence `0.5450`, probabilities `trend=0.4416 range=0.3117 stress=0.1714 transition=0.0753`; structural target remained immature (`rows=3`, `mature_rows=0`, `raw_scored_mature=0/30`, `production_validation=0/30`, `observation_validation=0/30`, `calibration=not_fitted`). Auto-Quant was used twice: first yfinance NQ 1h completed but was negative (`trade_count=2322`, `win_rate_pct=31.4815`, `sharpe=-8.4203`, `total_profit_pct=-43.0`); then IBKR 5m normalized `ts` -> `timestamp` and AQ batch/dispatch/rank exited `0`, but ranked unit status was `failed` with no usable metrics. TradingViewRemix `get_ohlcv` remained fail-closed. Terminal summaries: `support/docs/experiments/actionable-regime-confidence/runs/20260513T171059+0800-hermes-board-a-crossmarket-regime95-validation-v1/summaries/terminal_decision_summary.md` and `support/docs/experiments/actionable-regime-confidence/runs/20260513T171059+0800-hermes-board-a-crossmarket-regime95-validation-v1/summaries/ibkr_intraday_options_terminal_summary.md`. `update_goal=false`.
- Blocked `2026-05-13 / codex-board-a-kraken-btc-provider-posterior-chain-v1`: ran a new provider-backed Kraken BTC futures Board A chain under `support/docs/experiments/actionable-regime-confidence/runs/20260513T135235+0800-codex-board-a-kraken-btc-provider-posterior-chain-v1/` without reusing the yfinance chain root or rerunning the TVR root-cause lane. Real provider fetch used `fetch_external.py kraken-kline --market futures --pair PF_XBTUSD` for `1h`, `4h`, and `1d`, converted the CSVs to analyze JSON, then ran `analyze`, `workflow-status`, `pre-bayes-status`, `policy-training-status`, and `export-structural-path-ranking-target`. Provider rows were `1h=2000`, `4h=2000`, `1d=366`; the intraday Kraken futures fetches are capped and end at `2025-08-04`, so they are not current-market posterior evidence even though the `1d` frame reaches `2026-05-13`. Workflow exposes posterior distribution rather than forcing a label: active `range`, confidence `0.5616`, probabilities `range=0.7580 stress=0.1662 transition=0.0758 trend=0.0`. Pre-Bayes is `pass_neutralized` but risk flags include `low_directional_separation`; execution candidate is `no_trade/observe`; execution tree is `observe/promote_candidate=false` with `execution_guarded_due_to_high_transition_hazard`. Structural target export remains immature: `rows=3`, `mature_rows=0`, `history_rows=3`, `history_mature_rows=0`; policy training remains `raw_scored_mature=0/30`, `production_validation=0/30`, `observation_validation=0/30`, trainer artifact missing, runtime disabled. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T135235+0800-codex-board-a-kraken-btc-provider-posterior-chain-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Blocked `2026-05-13 / codex-live-structural-candidates-scorer-probe-v1`: repaired the incomplete live structural-candidates CatBoost scorer readback under `support/docs/experiments/actionable-regime-confidence/runs/20260513T133711+0800-codex-live-structural-candidates-scorer-probe-v1/` without taking over the Board B profitability root. Offline `uv --offline --with pandas --with numpy --with catboost` training on the `20260513T122932` history target succeeded with `41` history rows and `35` mature rows; latest scores were generated, applied via `apply-structural-path-ranking-external-scores`, and registered via `register-structural-path-ranking-trainer-artifact` using the JSON trainer metadata. `policy-training-status` now reports `trainer_status=runtime_eligible`, `runtime_selection=enabled_candidate_set_ready`, `runtime_mode=prefer_history`, `score_model_family=catboost`, `score_source=external_model`, `runtime_matches=3`, `raw_scored_mature=35/30`, and `production_validation=35/30`. This still does not pass Board A: latest target rows remain `mature_rows=0`, `rows_with_training_weight=0`, observation validation is `0/30`, workflow remains `Observe`, execution tree remains `observe/promote_candidate=false` with `execution_guarded_due_to_high_transition_hazard`, and current posterior confidence is below 95% (`trend=0.4550`, confidence `0.5823`; factor-research `research_iteration=0.4914`, confidence `0.5000`). Also, the workflow readback uses `support/examples/demo/demo-15m.json`, so this is not provider-backed promotion evidence. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T133711+0800-codex-live-structural-candidates-scorer-probe-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Blocked `2026-05-13 / codex-board-a-ibkr-kraken-fetch-matrix-v1`: fixed the IBKR harness fetch port mismatch and added terminal provider/context evidence under `support/docs/experiments/actionable-regime-confidence/runs/20260513T123600+0800-codex-board-a-ibkr-kraken-fetch-matrix-v1/`. `market-data-harness` now reuses `ICT_ENGINE_IBKR_GATEWAY_PORT` when set or auto-probes `7497/7496/4002/4001` and passes the reachable gateway port into `fetch_external.py`; child stdout/stderr is captured so harness JSON remains parseable. Fresh harness verification without manual `--port` returned `ok=true`, `rows=30`, `2026-03-31T00:00:00Z -> 2026-05-12T00:00:00Z` on IBKR QQQ 1d via reachable `IB Gateway paper:4002`; Kraken public XBTUSD 1h fetch returned `721` rows. Validation remains below the Board A acceptance target: IBKR QQQ 1d `avg_confidence=71.57% high_confidence=0.00%`; Kraken XBTUSD 1h `avg_confidence=59.56% high_confidence=6.06%`. Verification: `cargo fmt --check`, `cargo test ibkr_historical_args_include_configured_gateway_port -- --nocapture`, and the live `cargo run --quiet -- market-data-harness ...` fetch. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T123600+0800-codex-board-a-ibkr-kraken-fetch-matrix-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Blocked `2026-05-13 / codex-board-a-tvr-provider-root-cause-v1`: TradingViewRemix provider root-cause probe is terminal-blocked under `support/docs/experiments/actionable-regime-confidence/runs/20260513T123000+0800-codex-board-a-tvr-provider-root-cause-v1/`. Local config and key are present, but direct redacted `tools/list` against `https://tvremix.xyz/api/mcp/v1` returned HTTP `429` with `Rate limit exceeded`; normal `market-data-harness` fetch failed `get_ohlcv`; forced local stdio reported `local_stdio_ohlcv_available` but actual `get_ohlcv` failed with `SSL: UNEXPECTED_EOF_WHILE_READING`. This is not a missing local config issue. Keep TradingViewRemix fail-closed for Board A provider-backed validation until the remote rate-limit window clears and the local stdio OHLCV network path is repaired. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T123000+0800-codex-board-a-tvr-provider-root-cause-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Blocked `2026-05-13 / codex-board-a-provider-backed-chain-admission-v1`: real provider-backed chain readback ran through `provider-status`, `analyze-live`, `workflow-status`, `pre-bayes-status`, `policy-training-status`, and `export-structural-path-ranking-target` under `support/docs/experiments/actionable-regime-confidence/runs/20260513T121405+0800-codex-board-a-provider-backed-chain-admission-v1/`. Provider matrix was `market_data:6/7 ready` with `ibkr`, `kraken_public`, and `yfinance` ready, but `tradingview_mcp` unhealthy. Cross-instrument yfinance runs for `NQ`, `ES`, `GC`, and `CL` covered `1m,5m,15m,1h,4h,1d`; all stayed `observe/transition_guardrail`, none reached 95% calibrated confidence, and NQ path-ranker export had `rows=3` but `mature_rows=0`, `rows_with_raw_path_score=0`, `rows_with_calibrated_path_prob=0`, `rows_with_training_weight=0`. Terminal summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T121405+0800-codex-board-a-provider-backed-chain-admission-v1/summaries/terminal_decision_summary.md`. `update_goal=false`.
- Done `2026-05-13 / codex-regime-confidence-assets-runtime-readback-v1`: recovered 95%+ regime-confidence assets now have a native runtime entrypoint. `regime-confidence-assets --symbol REGIME_CONFIDENCE_ASSETS --state-dir /tmp/ict-engine-regime-confidence-assets --output-format human` writes `regime_confidence_asset_inventory.json`; `artifact-status --latest-only` sees `artifact_kind=regime_confidence_asset_inventory`, `status=ready_preserved`, `path_exists=true`; `policy-training-status` reads `Regime confidence assets: inventory=ready count=18 board_a_gate=11 direct_event=2 diagnostic=4 contrast_evidence=10 promotion_allowed=false runtime_selection=disabled`; `cargo test regime_confidence_asset -- --nocapture` passed. This closes asset recovery into a repo/runtime entrance, but not promotion or live workflow state.
- Latest visible terminal state keeps `update_goal=false`.
- Recent `221359` MACD six-provider AQ root is fail-closed for Board A: IBKR unavailable, only `5/6` provider rows acquired, dispatch timed out, no rank artifact, and no downstream admission.
- Recent direction correction says the issue is not a hard `15y 1m` requirement; the real requirement is the largest feasible provider/window density and enough K-line/trade evidence for the candidate being trained.
- Active provider-authority and Cargo/Rust work remains visible. Do not delete or rewrite artifacts owned by active claims.

## Live Gates

- `provider_backed_or_portable_manifest_required=true`
- `regime_confidence_asset_inventory_ready=true`
- `local_raw_dependency_allowed_for_training_only=true`
- `local_raw_dependency_allowed_for_promotion=false`
- `largest_feasible_provider_window_required=true`
- `tiny_daily_or_short_window_samples_not_training_evidence=true`
- `calibrated_regime_confidence_required=true`
- `cross_instrument_period_context_validation_required=true`
- `posterior_distribution_required_when_incomplete=true`
- `promotion_allowed=false_until_all_gates_pass`
- `update_goal=false`

## Iteration Protocol

For the next Board A iteration, use this order:

1. Select one unmet live gate from this file.
2. Create or reuse one compact run-root packet with `materials`, `summaries`,
   and `checks`.
3. Record only the terminal gate decision, artifact path, and fail-closed reason in
   this file.
4. Leave raw provider/AQ workspaces out of the decision path unless a compact
   manifest proves `local_raw_dependency=false`.
5. Before starting, check `Active Or Recent Roots To Preserve`; if a surface is
   occupied or belongs to Board B, record a no-takeover note here and choose a
   different Board A gate.
6. If another agent has the same idea, switch to a new unclaimed idea instead of
   competing for the same root.
7. Do not write a start claim, progress log, or routine readback into this file.

## Active Or Recent Roots To Preserve

- `20260512T221359+0800-codex-macd-current-six-provider-aq-v1`: fail-closed dispatch timeout and provider incompleteness evidence.
- `20260512T222644+0800-codex-215914-terminal-branch-path-readback-v1`: relevant to Board B branch-path propagation; preserve as cross-board evidence.
- `20260512T223650+0800-codex-provider-authority-preflight-no-aq-v1`: active/no-AQ provider authority preflight.
- Active Cargo/Rust/SMT test processes: do not clean generated state beneath their targets while running.

## Cleanup Rule

Historical Board A logs may not be deleted until:

1. All hard references to archival Board A prose are migrated or intentionally archived.
2. Compact extracted evidence covers every live gate above.
3. A dry-run reference audit shows no runtime or docs path relies on the old file.
4. A parity readback confirms Board A status is identical from this compact doc plus retained evidence packets.
