# Board B Profit Factor Current

Updated: `2026-05-13 20:03:32 +0800`

Purpose: clean current authority for Board B. Live profitability-factor
behavior belongs in `ict-engine` runtime code, state artifacts, provider configs,
candidate packs, admission targets, and compact evidence packets; old
append-only prose is archival only. Historical append-only prose is not a
planning source, status source, next-step source, or completion source.

Factor ingestion instruction surface: `support/docs/plans/2026-05-12-factor-candidate-ingestion-instructions.md`.

## Reader Contract

This file is the default Board B entrypoint. Do not enter historical logs during
normal iteration.

Use runtime/state surfaces first. Board B status updates must be terminal
decisions only: `keep`, `drop`, `incubate`, `blocked`, or `handoff`, with compact
evidence paths. Routine coordination and start claims must not be appended here.

Historical material may be opened only by exact artifact id, exact heading, or
exact evidence path when this file names a gap. It may never supply a new task,
claim, acceptance gate, route, or Board A/B boundary.

Useful factor findings must enter through the repo-local candidate-pack and
admission commands named in the factor ingestion instruction surface. This file
only records compact decisions and pointers to evidence packets.

## Role

Board B owns profitability-factor discovery, training, and downstream admission. It must root every candidate in:

`main_regime -> sub_regime -> sub_sub_regime_or_profit_factor -> profit_factor`

Board B must not take over Board A regime work, and must not touch non-owner roots such as `141000`.

## Parallel Boundary

- Expected concurrency is high: assume about 20 agents may work Board B while
  about 20 agents work Board A. Agents must not use this board as a shared lock
  table or scratchpad.
- Start claims are ephemeral: use `/tmp/ict-engine-agent-claims/board-b/` or an
  equivalent process-local lock outside the repo. Do not create claim docs,
  claim rows, TODO sprawl, or sibling plan files in the repo for starting work.
- Repo writes are allowed only for durable terminal evidence: compact run-root
  packets and one current-board decision row after the idea has evidence.
- Board B work can run in parallel with Board A only when it stays on
  profitability-factor discovery, candidate packs, provider/AQ provenance,
  branch-keyed profitability statistics, admission targets, and downstream
  handoff evidence for an already-frozen Board A context label.
- Board B must not claim, repair, promote, reject, relabel, or rerun Board A
  regime-confidence roots, posterior state, market-state labels, provider
  authority preflights owned by Board A, or recovered regime-confidence assets.
- Board A context fields inside Board B artifacts are attribution keys only.
  They are not Board B deliverables and do not authorize regime work.

## Idea Collision Rule

- If the chosen Board B idea is already claimed, active, done, or blocked, do not
  continue, repair, re-run, summarize, or "help" that idea.
- Write only a compact `duplicate_suppressed` or no-takeover note if useful,
  then pick a new unclaimed Board B idea from `Factor Gate Model` or
  `Live Gates`.
- A new idea must be different by at least one real ownership axis: factor,
  root regime, symbol/instrument set, provider/window, artifact root, or gate
  being tested.
- Do not create a sibling readback just to stay busy. If no unclaimed Board B
  idea exists, hand off the blocker in this file and stop.

## Code/Docs Isolation

- This file is not a runtime input. No Rust, Python, shell, provider, Auto-Quant,
  training, or workflow code may import, parse, grep, or depend on this markdown.
- Runtime state must live in code, configs, explicit state directories, provider
  manifests, candidate packs, admission targets, JSON/CSV/JSONL artifacts, or
  CLI output.
- Markdown may cite artifact paths and commands for humans/agents, but it must
  not become a hidden dependency, router, data source, fixture, or model input.
- If code needs a rule currently written only here, promote that rule into a
  typed config, command flag, schema, or test fixture first; do not read the doc.

## Current Contract

- Discovery is `profit_seed_first`: find a concrete profitable seed inside one clearly identified regime branch before running broad promotion audits.
- Factor training is isolated by default. Do not make every factor prove the full
  provider -> Auto-Quant -> Pre-Bayes -> BBN -> CatBoost -> execution-tree chain
  before it is useful.
- The normal Board B unit is one factor plus one gate, optionally conditioned on
  one root regime. The question is: "does this factor help in this root regime?"
- Promotion is stricter than seed discovery, but promotion should still be
  staged: first factor gate, then optional root-regime fit, then portability or
  downstream handoff only if the first two are worth keeping.
- Branch path must survive as first-class data at least as
  `root_regime -> factor`. Deeper branch detail is optional unless the current
  idea explicitly needs it.
- Sidecar CSV branch paths are not enough when a packet claims root-regime fit
  or downstream handoff.
- `exit=0` is not success unless the artifact covers the actual gate.

## Factor Gate Model

Use the simplest gate that answers the current isolated factor question.

- Gate 1, factor viability: enough observations, positive expectancy or useful
  negative classification, cost/slippage sanity, and no obvious leakage.
- Gate 2, root-regime fit: compare the factor inside one frozen Board A root
  regime versus outside it; keep the factor only if it improves that root or is
  clearly marked as root-agnostic.
- Gate 3, portability check: provider/window/instrument reproduction only after
  Gate 1 or Gate 2 says the factor is worth carrying.
- Gate 4, downstream handoff: Pre-Bayes, BBN, CatBoost/path-ranker, execution
  tree, and feedback/update are optional consumer-admission checks, not the
  default requirement for every isolated factor training run.

Default answer format for each Board B idea:

`factor_id`, `claimed_gate`, `root_regime` or `root_agnostic`,
`evidence_path`, `decision=keep/drop/incubate/blocked`,
`next_unclaimed_idea`.

## Current Status

- Board B is not complete.
- Blocked `2026-05-14 / codex-family-f-vrp-regime-branch-chain-v1`: exact VRP branch `TrendExpansion -> VolatilityCompression -> iv_hv_compression_regime -> vrp_compression_long_v1` preserved through agent-material batch/dispatch/rank, but the factor failed closed before downstream admission because required IV/HV provider inputs are missing. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T225914+0800-codex-family-f-vrp-regime-branch-chain-v1/summaries/family_f_vrp_regime_branch_chain_blocked_readback.md`. Provider probes were real: IBKR `QQQ OPTION_IMPLIED_VOLATILITY`, `QQQ HISTORICAL_VOLATILITY`, `VIX`, and `QQQ 15m` all timed out/empty; yfinance produced `NQ=F 15m` (`3765` rows), `QQQ 15m` (`1066` rows), and `^VIX 1d` (`2604` rows); Kraken `PF_XBTUSD 15m` produced `2000` rows; TradingViewRemix `get_ohlcv` failed through `market-data-harness`. Auto-Quant first failed on missing `NQ/USD 4h`, then same-workspace 4h repair wrote `254` bars and rerun reached the exact strategy's IV loader, where it failed on missing `/tmp/ict-engine-ibkr-probe/qqq.iv.1d.10y.csv`. No Pre-Bayes/BBN/CatBoost/execution-tree promotion was run because there is no valid Auto-Quant result, manifest, or realized-trade export to consume. Decision: `blocked`, not negative profitability; next unclaimed work is real QQQ IV/HV acquisition or a new non-IV/HV compression factor id that must not claim this VRP branch.
- Incubate `2026-05-13 / hermes-low-hazard-reclaim-new-factor-v1`: new low-hazard split factor `low_hazard_reclaim_long_v1` on branch `TrendExpansion -> SessionLiquidity -> dense_kline_low_hazard_reclaim -> low_hazard_reclaim_long_v1`, not a rerun of dense upbar reclaim. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T193423+0800-hermes-low-hazard-reclaim-new-factor-v1/summaries/low_hazard_reclaim_new_factor_readback.md`. Ran 5 Auto-Quant material units using existing provider K-lines from IBKR, yfinance, and Kraken. Rank positives: `IBKR QQQ 5m` (`696` trades, win rate `43.2471%`, Sharpe `38.4095`, total profit `5.63%`), `YF NQ=F 5m` (`193`, `41.4508%`, `46.4326`, `0.92%`), `IBKR QQQ 1m` (`722`, `36.5651%`, `26.2328`, `0.37%`), and `Kraken XBTUSD 1m` (`244`, `35.2459%`, `340.7426`, `1.14%`); `YF NQ=F 1m` was negative (`963`, `34.7871%`, `-22.0932`, `-0.22%`). Auto-Quant import accepted `5/5`; BBN prior applied all 5 with `evidence_value_gate_passed=true`; CatBoost trained/applied/registered/enabled, and execution-tree trace consumed the ranker (`visible=true`, `used=true`, `model=catboost`, `runtime_source=candidate_set`, `raw_path_score=0.750659`). Promotion remains blocked: CatBoost was trained from a 3-row pseudo-label structural target with `mature_rows=0`, `raw_scored_mature=0/30`, `production_validation=0/30`, `observation_validation=0/30`, and final execution stayed `observe/transition_guardrail/guarded` with `execution_guarded_due_to_high_transition_hazard`. Decision: `incubate`, not promotion; next unclaimed work is real per-trade export/replay for this exact low-hazard branch, then re-check validation and transition hazard.
- Handoff `2026-05-13 / hermes-low-hazard-reclaim-real-trade-feedback-replay-v1`: real per-trade replay completed for `low_hazard_reclaim_long_v1`. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T193423+0800-hermes-low-hazard-reclaim-new-factor-v1/summaries/low_hazard_reclaim_real_trade_feedback_replay_readback.md`. Exported `2818` real Freqtrade rows from IBKR QQQ 1m/5m, Kraken XBTUSD 1m, and YF NQ=F 1m/5m with exact `profit_abs_sum == profit_total_abs` on each provider slice, then replayed `150` actual trades into structural feedback. Runtime readback now shows `observation_validation=150/30 ready=true` and `raw_scored_mature=11180/30 ready=true`, but live ranker surface still sits at `enabled_no_matching_scores` and execution remains `execution_guarded_due_to_high_transition_hazard`. Decision: `incubate`, not promotion; next unclaimed work is current-score adapter repair or a tighter branch split, not aggregate replay.
- Handoff `2026-05-13 / hermes-family-h-dense-kline-trade-export-v1`: actual per-trade export for dense positive 1m/5m branch completed before any feedback replay. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T184800+0800-hermes-family-h-dense-kline-trade-export-v1/summaries/dense_kline_trade_export_readback.md`. Exported real `bt.results["strategy"][strategy]["trades"]` rows from the same Freqtrade/Auto-Quant workspaces for `IBKR QQQ 5m` (`1351` rows), `IBKR QQQ 1m` (`1272`), `YF NQ=F 5m` (`270`), `YF NQ=F 1m` (`1318`), and `Kraken XBTUSD 1m` (`348`). `profit_abs` row sums exactly match Freqtrade `profit_total_abs`, proving these are real per-trade rows rather than aggregate summary reconstruction. No feedback replay or promotion was claimed. Next unclaimed work may replay only these exported trade CSV rows into structural feedback with branch path + provider/timeframe provenance; do not use the earlier synthetic feedback files from `hermes-family-h-dense-kline-feedback-maturity-v1`.
- Blocked `2026-05-13 / hermes-family-h-dense-kline-feedback-maturity-v1`: halted before claiming maturity because dense Auto-Quant rank metrics are aggregate Freqtrade summaries, not individual realized trade records. `total_profit_pct` is account-level total profit percent from `100000 USD` starting balance (example: `IBKR QQQ 5m` final balance `106889.438 USD`, total profit `6.89%`, CAGR `80.96%` over 41 days), while Sharpe is Freqtrade's annualized summary metric and is inflated/unstable on dense intraday short windows and tiny per-trade edge. Evidence under `support/docs/experiments/actionable-regime-confidence/runs/20260513T183600+0800-hermes-family-h-dense-kline-feedback-maturity-v1`; partial synthetic feedback generation is non-authoritative and must not be used for promotion. Next unclaimed work should export actual per-trade results/trades CSV from Auto-Quant/Freqtrade, then replay only real realized trade rows into structural feedback.
- Incubate `2026-05-13 / hermes-family-h-dense-kline-regime-branch-v1`: user-requested dense K-line continuation for branch `TrendExpansion -> SessionLiquidity -> dense_kline_upbar_reclaim -> dense_kline_upbar_reclaim_long_v1`. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T181830+0800-hermes-family-h-dense-kline-regime-branch-v1/summaries/dense_kline_regime_branch_terminal_readback.md`. Ran 11 Auto-Quant material units across 1m/5m/15m/30m where data was available: `YF NQ=F 1m/5m/15m/30m`, `IBKR QQQ 1m/5m/15m/30m`, and `Kraken XBTUSD 1m/15m/30m`; TradingViewRemix/TVR QQQ `1m/5m/15m/30m` was attempted but `tradingview_mcp get_ohlcv` failed for all four intervals. Branch fields survived batch/dispatch/rank as first-class `main_regime/sub_regime/sub_sub_regime_or_profit_factor/profit_factor`. Best dense positives: `IBKR QQQ 5m` (`1351` trades, win rate `40.3405%`, Sharpe `34.83`, total profit `6.89%`), `IBKR QQQ 1m` (`1272`, `38.2862%`, `112.3034`, `1.87%`), `YF NQ=F 5m` (`270`, `41.8519%`, `93.5579`, `1.71%`), `YF NQ=F 1m` (`1318`, `37.2534%`, `87.6718`, `0.87%`), and `Kraken XBTUSD 1m` (`348`, `34.7701%`, `246.3706`, `0.88%`). 15m/30m slices were mostly negative. Auto-Quant import accepted `11/11`, BBN prior init applied `11` with evidence gate passed, CatBoost trained/applied/registered/enabled, and execution-tree trace shows `path_ranker_score_visible_to_execution_tree=true`, `used=true`, `model=catboost`. Promotion remains blocked: ranker validation is still `raw_scored_mature=0/30`, `production_validation=0/30`, `observation_validation=0/30`, and final execution stayed `observe/transition_guardrail/guarded`. Decision: `incubate`, not promotion; next unclaimed work should create comparable structural-feedback observations for the positive 1m/5m dense branch, repair TVR dense fetch, or split the 15m/30m negative branch instead of promoting the mixed packet.
- Incubate `2026-05-13 / hermes-family-h-vwap-regime-branch-chain-v1`: Gate 4 regime-branch runtime chain for `runtime_density_upbar_reclaim_long_v1` on branch `TrendExpansion -> SessionLiquidity -> runtime_density_upbar_reclaim -> runtime_density_upbar_reclaim_long_v1`. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T171220+0800-hermes-family-h-vwap-regime-branch-chain-v1/summaries/hermes_family_h_regime_branch_chain_terminal_readback.md`. Auto-Quant material dispatch completed across `yfinance/YF SPY 1h`, `IBKR SPY 1h`, `TradingViewRemix/TVR QQQ 1h`, and `Kraken XBTUSD 1h`; the branch path survived batch/dispatch/rank as first-class `main_regime/sub_regime/sub_sub_regime_or_profit_factor/profit_factor` fields. Only TradingViewRemix was mildly positive (`21` trades, win rate `42.8571%`, Sharpe `0.7103`, total profit `0.39%`); Kraken/YF/IBKR were negative. Auto-Quant strategy library import accepted `4/4`, BBN prior init applied all `4` strategies with `evidence_value_gate_passed=true`, CatBoost trained and was registered/enabled, and execution-tree trace shows `path_ranker_score_visible_to_execution_tree=true`, `used=true`, `model=catboost`. Promotion is blocked: ranker validation remains `raw_scored_mature=0/30`, `production_validation=0/30`, `observation_validation=0/30`, final execution stayed `observe/transition_guardrail/guarded`, and provider portability is not positive. Decision: `incubate`, not promotion; next unclaimed work should either generate matured structural-feedback rows for this exact branch or test a stricter SessionLiquidity leaf that avoids the YF/IBKR/Kraken negative-density failure.
- Blocked `2026-05-13 / codex-live-structural-candidates-scorer-probe-v1`: Gate 4 first-class live `structural-candidates:*` scorer probe for the incubating Family A packet. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T133711+0800-codex-live-structural-candidates-scorer-probe-v1/summaries/live_structural_candidates_scorer_probe_terminal_readback.md`. Exported the live target and confirmed `rows=3`, `candidate_set_id=structural-candidates:FAMILY_A_KILLZONE_MTF_122932_NATIVE:2ce054df90e47839`; current live rows remain unobserved (`mature_rows=0`, `training_weight_rows=0`, `calibrated_rows=0`, `execution_gate_rows=0`) while mature supervised signal exists only in history/admission rows (`history_mature_rows=35`). A direct fallback scorer closed the live candidate-set consumption bridge but all scores were flat `0.5` and explicitly non-promotional. A later offline history CatBoost run trained on `35` mature history rows and registered successfully: `runtime_selection=enabled_candidate_set_ready`, `runtime_mode=prefer_history`, `runtime_source=candidate_set`, `score_model_family=catboost`, `score_source=external_model`, `runtime_matches=3`; however it selected only `structural_baseline_score`, produced flat current live scores (`0.05891652998482881` for all 3 paths), and workflow/policy readback still blocks promotion with `observation_validation=0/30`, no structural-feedback observations, `no_previous_run` risk flags, final action `Observe`, and guarded execution tree. Decision: `blocked`, not promotion; next unclaimed work should create real live structural-feedback observations/comparable candidate-set history that breaks `no_previous_run` and `observation_validation=0/30`, not rerun flat current-row scoring.
- Handoff `2026-05-13 / codex-factor-research-after-live-score-adapter-v1`: Gate 4 structural-feedback probe after live execution-tree candidate-set score consumption. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T125242+0800-codex-factor-research-after-live-score-adapter-v1/summaries/factor_research_after_live_score_adapter_terminal_readback.md`. Ran the workflow-recommended native `factor-research` command against the persisted Family A state after the live path-id adapter probe; no Auto-Quant rerun and no promotion claim. The run wrote `research:FAMILY_A_KILLZONE_MTF_122932_NATIVE:20260513T045342.052Z`, objective `expansion_manipulation`, best factor `trend_momentum`, aggregate return `0.002891322311496955`, and generated/applied `113/113` feedback records. It kept path-ranker consumption on live candidate-set scores (`using_candidate_set_scores`, `runtime_matches=3`) but promotion stayed held: `approved=false`, `status=hold`, reason includes `no_previous_run` and `artifact_consumed_trend_status=no_consumed_validation`; execution gate remained `execution_observe_only`, execution readiness `0.45`, and agent action plan still starts with `Block Promotion` / `OBSERVE trend_momentum`. Decision: `handoff`; next unclaimed work should create comparable research/backtest history for this same live scored candidate set, implement/train a real live `structural-candidates:*` scorer, or repair provider portability before further downstream promotion attempts.
- Handoff `2026-05-13 / codex-live-exectree-score-id-adapter-probe-v1`: Gate 4 downstream score-consumption ID-match probe for the incubating Family A packet. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T124630+0800-codex-live-exectree-score-id-adapter-probe-v1/summaries/live_exectree_score_id_adapter_probe_terminal_readback.md`. The probe did not rerun Auto-Quant or claim promotion. It exported the post-analyze live execution-tree structural target and found `rows=3`, `candidate_set_id=structural-candidates:FAMILY_A_KILLZONE_MTF_122932_NATIVE:2ce054df90e47839`, while the prior CatBoost/admission scores had targeted `factor-candidate-admission:FAMILY_A_KILLZONE_MTF_122932_NATIVE:curated-auto-quant-v1`. A 3-row adapter-probe score file using the live `candidate_set_id/path_id` values changed workflow consumption from the previous `enabled_no_matching_scores` failure mode to `path_ranker_summary.status=using_candidate_set_scores`, with `applied_path_count=3`, `candidate_set_match_count=3`, `runtime_source=candidate_set`, and `latest_structural_execution_candidate.path_ranker_raw_score=0.557601`. This proves the live execution-tree ID bridge shape, but it is not a trained CatBoost profitability model and not trade-usable: live target rows are still unobserved with `mature_rows=0`, `calibrated_rows=0`, `execution_gate_rows=0`, `training_weight_rows=0`, final action remains `Observe`, and provider portability remains incomplete. Decision: `handoff`; next unclaimed implementation-grade work should make live `structural-candidates:*` scoring first-class and keep admission candidate-pack scores separate from live execution-tree path scores.
- Incubate `2026-05-13 / codex-family-a-killzone-mtf-provider-repair-v1`: Gate 1/provider-window MTF repair for `family_a_killzone_breakout_1h_v1` on branch `TrendExpansion -> KillzoneBreakout -> family_a_killzone_breakout_1h_v1 -> nq_am_breakout_long`. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T122932+0800-codex-family-a-killzone-mtf-provider-repair-v1/summaries/family_a_killzone_mtf_provider_repair_terminal_readback.md`. Branch fields survived material/dispatch/rank, initial Auto-Quant failed on missing NQ/USD 4h informative data, and a manual same-workspace MTF repair derived `NQ_USD-4h.feather` before a successful `TomacNQ_KillzoneBreakout` rerun: 91 trades, win rate 70.3297%, profit factor 1.3449, total profit 12.40%, Sharpe 0.0489, max drawdown 8.61%. Native admission wrote 35 structural-path rows; CatBoost model/scores were generated and applied, and ranker runtime enabled with 35 admission-set matches. Downstream chain did not close: provider matrix remains incomplete, analyze/workflow final action stayed `Observe`, execution was guarded by `execution_guarded_due_to_high_transition_hazard`, and live workflow path-ranker status was `enabled_no_matching_scores` because admission candidate-pack scores did not match the analyze execution-tree path ids. Decision: `incubate`, not promotion; next unclaimed work should target paired timeframe material generation or a live execution-tree candidate-id score adapter rather than rerunning the same packet unchanged.
- Blocked `2026-05-13 / codex-family-d-liquidity-sweep-gate1-v1`: non-duplicate Gate 1/downstream smoke for `family_d_liquidity_sweep_reclaim_15m_wide_v1` on branch `TrendTransition -> LiquidityReclaim -> family_d_liquidity_sweep_reclaim_15m_wide_v1 -> liquidity_sweep_reclaim_long`. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T121613+0800-codex-family-d-liquidity-sweep-gate1-v1/summaries/family_d_liquidity_sweep_gate1_terminal_readback.md`. Native candidate/admission/training surfaces wrote durable 35-row structural targets and reported `raw_scored_mature=35/30`, `production_validation=35/30`, `trainer_status=runtime_eligible`, but `runtime_selection=disabled`. Auto-Quant material batch/dispatch/rank ran and preserved `regime_profit_branch_path`, `main_regime`, `sub_regime`, `sub_sub_regime_or_profit_factor`, `profit_factor`, and provider provenance into dispatch/rank artifacts. The packet failed before Pre-Bayes/BBN/CatBoost runtime/execution-tree closure because Freqtrade had Yahoo NQ 1h material but no required NQ/USD 4h informative dataframe: `ValueError: Informative dataframe for (NQ/USD, 4h, spot) is empty`. Decision: `blocked`, not promotion; do not rerun the same packet without paired 1h/4h provider material or a parity-preserving material generator.
- Done `2026-05-13 / codex-profit-factor-trace-live-path-v1`: tried one profitable factor candidate, `family_f_trend_pullback_dense_15m_v1`, through the runtime chain. Evidence summary: `support/docs/experiments/actionable-regime-confidence/runs/20260513T113009+0800-codex-profit-factor-trace-live-path-v1/summaries/profit_factor_trace_live_path_readback.md`. The factor reached candidate inventory, admission targets, policy-training/ranker artifact, explicitly enabled ranker runtime, analyze/analyze-live, Pre-Bayes soft evidence, ensemble vote, execution candidate, and execution tree. It did not reach practical execution: final ensemble action stayed `Observe`, execution tree blocked with readiness `0.4446 < 0.45`, `execution_guarded_due_to_high_transition_hazard`, and live candidate review stayed observe due to `candidate_not_comparable_same_data_factor_required`. This is useful negative execution-admission evidence, not promotion.
- Done `2026-05-13 00:24:35 +0800 / codex-candidate-pack-runtime-loop-v1`: seven curated Auto-Quant-derived candidates are visible through `python3 support/scripts/research/factor_candidate_resolver.py --repo-root . --list-buildable --output-format human`; generated pack indexes use relative `pack_dir` and repo-relative `source_candidate_pack_dir`; leak scan over generated packs/registry/index/list output found no `/Users`, `/tmp`, `/private`, or `Auto-Quant`; `python3 -m unittest support.scripts.research.tests.test_factor_candidate_resolver support.scripts.research.tests.test_factor_candidate_pack` passed with 16 tests. Remaining promotion gates still apply; this only closes the reusable candidate-pack inspection loop.
- Done `2026-05-13 / codex-native-candidate-pack-cli-v1`: native `cargo run -- factor-candidate-packs` lists the seven repo-local curated packs without Python or Board docs; `--state-dir /tmp/ict-engine-candidates --symbol FACTOR_CANDIDATES` writes `factor_candidate_pack_inventory.json`, appends an `artifact_kind=factor_candidate_pack_inventory` ledger row with `status=ready`, and refreshes `workflow_snapshot.json`; `artifact-status --latest-only` reads the entry with `path_exists=true`; `workflow-status` exposes it under `recent_artifacts`; `cargo test factor_candidate_pack -- --nocapture` passed 3 native candidate-pack tests; Python resolver/pack tests still pass 16 tests. This makes the moved results visible in the normal CLI/state loop, but not yet trade-usable promotion.
- Done `2026-05-13 / codex-candidate-pack-policy-training-visibility-v1`: `policy-training-status --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates --output-format human` now reports `Factor candidate packs: inventory=ready count=7 preferred_density=6 cross_market=6`; JSON exposes `factor_candidate_packs.inventory_ready=true`, `candidate_pack_count=7`, `preferred_density_count=6`, and `cross_market_candidate_count=6`. This moves the curated成果 into the normal training-readiness inspection surface. It does not promote them into path-ranker/runtime execution; structural path ranking remains `runtime_selection=disabled` until Pre-Bayes/BBN/CatBoost/execution-tree gates are satisfied.
- Done `2026-05-13 / codex-candidate-pack-structural-admission-target-v1`: new native `factor-candidate-admission-targets --symbol FACTOR_CANDIDATES --state-dir /tmp/ict-engine-candidates-admission` writes the inventory, standard `policy_training/structural_path_ranking_target.*`, `factor_candidate_ranker_direct_model.json`, and `structural_path_ranking_trainer_artifact.json`; smoke output reports 35 admission rows and `policy-training-status` reports both `inventory=ready count=7 preferred_density=6 cross_market=6` and `structural_path_ranking_target rows=35 history_rows=35 mature_rows=35 history_mature_rows=35`; `policy-training-status` also reports `trainer_artifact=ready trainer_status=present_validation_insufficient`; `artifact-status --latest-only` sees `factor_candidate_pack_inventory status=ready path_exists=true`, `structural_path_ranking_target status=admission_pending path_exists=true`, and `structural_path_ranking_trainer_artifact status=ready_observation_only path_exists=true`; `workflow-status` exposes all three under redacted `recent_artifacts`. Rows are `direction=Observe`; candidate aggregate rows plus cross-market evidence rows become offline matured target observations with `training_weight_rows=35` and `raw_scored_mature=35/30`; `calibrated_rows=0`, `execution_gate_rows=0`, `production_validation=0/30`, and `runtime_selection=disabled`, so this is normal training/admission visibility with an observation-only trainer artifact, not trade-usable promotion. `cargo test factor_candidate_admission -- --nocapture` passed.
- Latest objective audit says the core blocker is first-class branch-path survival plus positive provider-portable profitability.
- `215914` proved six-provider acquisition and Auto-Quant rank can run, but failed because material/rank artifacts lost branch-path fields and profitability was not mature/provider-portable.
- `221359` failed via IBKR unavailable, missing material rows, dispatch timeout, no rank.
- `220702` exact branch readback remains blocked at Pre-Bayes/BBN/entry-model readiness.
- `223253` profit-seed closed-artifact scan is active; do not duplicate it.
- `223410` SMT strict-density expansion is active; do not duplicate it.
- Active Cargo/Rust branch-path/SMT owners are still visible; do not overwrite their source or target state.

## Live Gates

- `profit_seed_first=true`
- `single_factor_single_gate_default=true`
- `root_regime_fit_optional_max_default=true`
- `full_chain_required_for_every_factor=false`
- `branch_path_required_from_first_artifact=root_regime_plus_factor`
- `material_json_branch_path_required=only_for_root_fit_or_handoff`
- `rank_artifact_branch_path_required=only_for_handoff`
- `provider_matrix_required_for_accepted_branch=true`
- `positive_provider_portable_profitability_required=true`
- `pre_bayes_filter_required_after_seed=false_by_default`
- `bbn_learning_required_after_filter=false_by_default`
- `catboost_path_ranker_required_after_bbn=false_by_default`
- `execution_tree_non_observe_required=false_by_default`
- `feedback_update_learning_required=false_by_default`
- `promotion_allowed=false_until_all_gates_pass`
- `trade_usable=false_until_all_gates_pass`
- `update_goal=false`

## Iteration Protocol

For the next Board B iteration, use this order:

1. Pick one branch path and one unmet gate from this file.
2. Prove a profitable seed before broad promotion: meaningful trade count,
   positive expectancy, branch path present, and provider/window provenance.
3. Keep the first pass isolated: one factor, one gate, and at most one frozen
   root regime.
4. Preserve at least `root_regime` and `factor_id` in the first material. Add
   deeper `main_regime`, `sub_regime`, `sub_sub_regime_or_profit_factor`,
   `profit_factor`, and `regime_profit_branch_path` only when the idea claims
   root-fit or downstream handoff.
5. Only after Gate 1 or Gate 2 is worth keeping, run provider portability or the
   Pre-Bayes/filter -> BBN -> CatBoost/path-ranker -> execution-tree chain.
6. Record the terminal decision here as keep/drop/incubate/blocked/handoff with artifact paths;
   do not append routine status to archival Board B prose.
7. Before starting, check `Current Status` and active roots; if a surface is
   occupied or belongs to Board A, record `duplicate_suppressed` or no-takeover
   coordination here and choose a different profitability-factor gate.
8. If another agent has the same idea, switch to a new unclaimed idea instead of
   competing for the same root.
9. Do not write a start claim, progress log, or routine readback into this file.

## SMT Boundary

SMT is a Board B confirmation factor only. It is not a Board A regime task, not generic correlation, not relative strength, and not standalone actionable.

Current SMT gates:

- same timeframe, same overlapping session, same swing/liquidity event;
- relationship stability is only a comparable-symbol gate;
- inverse relationships must emit normalized and raw comparison structure;
- every signal requires base and comparison levels;
- trade evidence requires later MSS/CISD or displacement plus PDA entry model;
- current strict rows are insufficient, with `stress` and `other` still sparse.

## Cleanup Rule

Historical Board B logs may not be deleted until:

1. All hard references to archival Board B prose are migrated or intentionally archived.
2. Compact extracted evidence covers every live gate above.
3. Active claims such as `223253`, `223410`, provider-authority work, and Cargo/Rust owner work are closed or handed off.
4. A dry-run reference audit shows no runtime or docs path relies on the old file.
5. A parity readback confirms Board B status is identical from this compact doc plus retained evidence packets.
