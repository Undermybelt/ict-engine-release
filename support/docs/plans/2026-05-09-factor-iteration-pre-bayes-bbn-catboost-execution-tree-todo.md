# Factor Iteration -> Pre-Bayes -> BBN -> CatBoost -> Execution-Tree TODO

> Authoritative bridge board for the next pure-iteration loop.  
> This board does **not** replace the existing factor board or the post-factor runtime-closure board.  
> It exists to sequence them correctly so factor iteration does not stop at backtest metrics and runtime closure does not begin from a vague handoff.

**Goal:** run the next iteration as one explicit regime-rooted branch-ranking chain:

1. market data
2. market state / regime inference
3. regime-rooted execution-tree branch candidates
4. Auto-Quant factor iteration and backtest by `regime + branch`
5. imported `strategy_library` / realized trades
6. BBN prior init/update
7. structural path target export
8. CatBoost / structural path-ranking surface
9. execution-tree / workflow outcome

and make every handoff explicit, reviewable, and `/tmp/...` isolated.

**Architecture:** keep runtime code unchanged unless a real middle-layer defect is proven. Use existing public CLI/runtime surfaces first. Treat factor iteration as external/additive work, then push only explicit artifacts downstream. The root node is the current market state / regime; branch candidates are the allowed operation paths under that regime. Do not mix new factor search, provider bootstrap, and runtime-surface refactors into one pass.

**Tech Stack:** `support/docs/plans/2026-05-08-factor-iteration-filter-belief-catboost-execution-tree-board.md`, `support/docs/plans/2026-05-07-auto-quant-post-factor-runtime-closure-todo.md`, `support/docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`, `support/docs/202605071246nextstep`, `./target/debug/ict-engine factor-research`, `auto-quant-results-import`, `auto-quant-prior-init`, `auto-quant-ingest-real-trades`, `pre-bayes-status`, `policy-training-status`, `export-structural-path-ranking-target`, `workflow-status`, `execution_tree_trace.json`, explicit `/tmp/...` state dirs, Auto-Quant `strategy_library.json`, realized-trades JSONL.

**Baseline / Authority Refs:**
- `support/docs/plans/2026-05-08-factor-iteration-filter-belief-catboost-execution-tree-board.md`
- `support/docs/plans/2026-05-07-auto-quant-post-factor-runtime-closure-todo.md`
- `support/docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md`
- `support/docs/202605071246nextstep`
- `/tmp/vrp-v2-runtime-closure/`
- `/tmp/vrp_v2_strategy_library.json`
- `/tmp/vrp_v2_realized_trades.jsonl`

**Compatibility Boundary:** pure iteration first. No repo runtime edits unless a public command or persisted artifact surface is actually broken on `green-baseline`. Use repo-native terms in this board:
- `绿波` = `pre-bayes / filter gate`
- `信念网络` = `BBN prior/posterior evidence`
- `cat boost` = `structural path-ranking / external ranker surface`
- `执行树` = `execution_tree_trace.json` + `workflow-status` downstream outcome

**Verification:** all claims on this board must be backed by a fresh command or a persisted artifact from the same `/tmp/...` state dir.

---

## Decision Lock

- [x] This board is orchestration/guidance only. It should tell the next agent exactly how to move a candidate through the full chain.
- [x] The root of the chain is **current market state / regime**, not a detached high-Sharpe factor search.
- [x] The execution tree candidates under that regime are the first-class branch targets:
  - `range_mean_reversion`
  - `transition_confirmation`
  - `de_risk`
  - `trend_continuation`
  - `exhaustion_reversal`
  - `breakout_continuation`
  - `liquidity_sweep_failure`
- [x] Every branch row must carry three separate scoring/evidence surfaces:
  - `BBN prior`: historical / Auto-Quant / realized-trade evidence for the branch
  - `market likelihood`: current market-state evidence for that branch
  - `CatBoost score`: factor/path-result score for branch ranking
- [x] Auto-Quant is not allowed to optimize a generic high-Sharpe strategy first. It must first train factors that separate **branch win probability inside a known regime bucket**.
- [x] Do not reopen the pseudo-label direction-model lane as the main path. The next loop is branch-specific evidence generation for BBN and CatBoost.
- [x] Factor iteration still starts on the factor board, not here.
- [x] Runtime closure still lands on the post-factor board, not here.
- [x] This board is the sequencing contract between the two.
- [x] This board is not allowed to become a third competing execution owner:
  - factor generation / candidate-pack truth stays on `2026-05-08-factor-iteration-filter-belief-catboost-execution-tree-board.md`
  - runtime mutation / before-after closure stays on `2026-05-07-auto-quant-post-factor-runtime-closure-todo.md`
  - this board only labels the chain, the stopping layer, and the exact next handoff
- [x] No code reopening is justified yet just from this audit.
- [x] One real surface drift was found and normalized here:
  - `support/docs/plans/2026-05-07-auto-quant-post-factor-runtime-closure-todo.md` still mentions `auto-quant-results-import --dry-run`
  - current `green-baseline` binary does **not** support `--dry-run` on that command
  - therefore import rehearsal on current mainline must use an isolated copied `/tmp/...` state dir instead of a non-existent dry-run flag

## Regime-Rooted Branch Ranker Contract

### Branch-ranking row shape

Every candidate row exported to BBN / CatBoost must be keyed by:

- `symbol`
- `timeframe`
- `run_id`
- `regime_label`
- `market_state_primary`
- `market_state_secondary`
- `branch_id`
- `branch_family`
- `factor_family`
- `factor_variant`
- `bbn_prior_evidence`
- `market_likelihood_evidence`
- `catboost_feature_vector`
- `realized_branch_outcome`
- `stopping_layer`

This is the row identity contract for branch scoring. A factor candidate is incomplete if it only reports standalone strategy metrics and cannot be joined back to `regime_label + branch_id`.

### Current QQQ run anchor

- Current state: `range_choppy / RangeConsolidation + WideRange`.
- Current structural branch: `range_mean_reversion`.
- Current downstream outcome: execution blocked / observe.
- Current interpretation: the engine should not ask "what high-Sharpe strategy works on QQQ?" first. It should ask "inside this range-choppy state, what evidence distinguishes range mean-reversion viability from transition confirmation or stress de-risk?"

### First branch-specific factor queue

1. `range_mean_reversion_viability`
   - Bollinger / ATR stretch
   - distance from VWAP / session midpoint
   - prior sweep plus failed continuation
   - chop / volatility compression versus expansion
2. `transition_confirmation`
   - range break with displacement
   - volume expansion after compression
   - multi-timeframe alignment persistence
   - failed mean-reversion after boundary break
3. `stress_de_risk`
   - volatility spike
   - liquidity thinning
   - wide-range continuation hazard
   - FOMO / crowding risk

### Regime-bucket questions Auto-Quant must answer

- In `range_choppy` / `range_consolidation`, which factors separate `range_mean_reversion` from `transition_confirmation`?
- In `trend_expansion`, which factors separate `trend_continuation` from `exhaustion_reversal`?
- In `manipulation_expansion`, which factors separate `breakout_continuation` from `liquidity_sweep_failure`?

Auto-Quant output must feed two downstream lanes from the same artifact pack:

- BBN lane: branch/node prior evidence.
- CatBoost lane: branch scoring training rows.

## Current Closed-Loop Diagnosis

### What is alive on `green-baseline`

- [x] `pre-bayes-status` is alive and readable.
  - latest checked state:
    - `Pre-Bayes | gate=pass_neutralized | soft_evidence=yes`
    - `Bridge: long=0.551 | short=0.530 | mtf=bullish | align=1.000 | entry_align=0.860`
- [x] `auto-quant-prior-init` is alive and still consumes imported strategy-library evidence.
  - latest checked state:
    - `trade_count=815`
    - `final_probs=[0.3462936184690158, 0.00000021385176184690159, 0.6537061676792224]`
- [x] `auto-quant-ingest-real-trades` is alive and still parses the realized-trades artifact.
  - latest checked state:
    - `trades_total=815`
    - `trades_applied=815`
    - `trades_invalid=0`
- [x] `export-structural-path-ranking-target` is alive and still exports the same target surface.
  - latest checked state:
    - `rows=3`
    - `mature_rows=0`
    - `rows_with_raw_path_score=3`
    - `rows_with_calibrated_path_prob=0`
    - `production_validation=0/30`
- [x] `policy-training-status` is alive and reports the current CatBoost/path-ranking state honestly.
  - latest checked state:
    - `trainer_artifact=ready`
    - `trainer_status=present_validation_insufficient`
    - `runtime_selection=enabled_candidate_set_ready`
    - `runtime_source=candidate_set`
    - `runtime_matches=3`
- [x] `workflow-status` and `execution_tree_trace.json` are alive and still close the chain.
  - latest checked state:
    - `workflow-status`: `current_focus_phase=analyze`
    - `workflow-status`: `recommended_next_command=ict-engine factor-research --symbol NQ --data support/examples/demo/demo-15m.json --state-dir /tmp/vrp-v2-runtime-closure --backend native`
    - execution tree:
      - `branch=transition_guardrail`
      - `execution_bias=guarded`
      - `gate_status=observe`
      - `execution_score=0.5806074494341393`

### What is not broken, but still not closed

- [x] The factor -> pre-bayes -> BBN -> execution-tree path is not dead.
- [x] The actual current blocker is **not** “command missing” or “binary crashed”.
- [x] The actual current blocker is that the CatBoost / structural path-ranking layer still lacks mature, structural-lineage-backed rows:
  - `mature_rows=0`
  - `raw_scored_mature=0/30`
  - `production_validation=0/30`
- [x] The execution tree is therefore still running from candidate-set scores / current evidence, not from a validated mature external ranker loop.

### What is the only confirmed surface mismatch

- [x] `auto-quant-results-import --dry-run` is documented in the older runtime-closure board, but unsupported by the current mainline binary.
- [x] Treat this as a guidance mismatch, not yet a code-reopen trigger by itself.
- [x] For pure iteration on current mainline:
  - rehearse import on a copied `/tmp/...` state dir
  - then run the real import only in an isolated throwaway state

## Current Todo Board

### Done

- [x] Normalized user shorthand into repo terms:
  - `濾波` -> `pre-bayes / filter gate`
  - `信念网络` -> `BBN prior/posterior evidence`
  - `cat boost` -> `structural path ranking / external ranker surface`
  - `执行树` -> `workflow-status` + `execution_tree_trace.json`
- [x] Re-read the existing factor board, runtime-closure board, and next-step diagnosis doc.
- [x] Audited the current mainline public command surface.
- [x] Confirmed the middle layers are alive on `green-baseline`.
- [x] Isolated the actual current closure blocker to the CatBoost/path-ranking maturity layer, not to factor import, BBN prior-init, or execution-tree readback.
- [x] Confirmed one guidance drift:
  - `auto-quant-results-import` has no `--dry-run` on current mainline

### Next Slice

- [x] Build the next Auto-Quant candidate pack around `regime_label + branch_id`, not around standalone Sharpe.
  - evidence: `/tmp/ict-regime-branch-iteration-20260510-1/candidate-pack/`
  - candidate: `qqq_regime_branch_transition_confirmation_tomac_20260510`
  - selected strategy: `TomacNQ_KillzoneBreakout`
  - branch family: `transition_confirmation`
- [x] For the current QQQ anchor, start with `range_choppy / RangeConsolidation + WideRange` and compare:
  - `range_mean_reversion`
  - `transition_confirmation`
  - `stress_de_risk`
  - evidence: `/tmp/ict-regime-branch-iteration-20260510-1/branch_ranking_rows_qqq_20260510.json`
  - correction: current provider evidence is split, not a single clean range label:
    - IBKR: `RangeConsolidation/WideRange`, `observe/transition_guardrail/guarded`
    - YF: `TrendExpansion/BullTrendAcceleration`, `observe/fill_viable/passive`
    - TradingViewRemix/MCP: `TrendExpansion/BullTrendAcceleration`, `observe/fill_viable/passive`
    - Kraken: provider reachability only for `PF_XBTUSD`, not QQQ branch-performance evidence
- [x] Require each factor result to state which branch it separates and which downstream lane it supports:
  - BBN prior evidence
  - CatBoost branch scoring row
  - both
  - evidence: `/tmp/ict-regime-branch-iteration-20260510-1/candidate-pack/factor_expression.json`
  - evidence: `/tmp/ict-regime-branch-iteration-20260510-1/candidate-pack/factor_eval_grid_summary.json`
  - evidence: `/tmp/ict-regime-branch-iteration-20260510-1/candidate-pack/transfer_score.json`
- [x] Export one branch-ranking row set with explicit `market_likelihood_evidence` before calling the loop closed.
  - evidence: `/tmp/ict-regime-branch-iteration-20260510-1/branch_ranking_rows_qqq_20260510.json`
- [x] Run the next candidate through the chain in this order, without skipping layers:
  - build/refresh explicit factor candidate artifact
  - check pre-bayes / bridge state from the same `/tmp/...` state
  - apply or inspect BBN prior-init effect
  - inspect whether structural path-ranking target rows grew in a meaningful way
  - inspect whether execution-tree / workflow output changed
  - evidence: `support/docs/plans/2026-05-09-vrp-v2-loop-handoff-todo.md`
- [x] Do not call a factor “closed” just because its standalone backtest is good.
  - Slice result: `VRPCompression_V2_NQ_15m` is chain-readable, but not mature external-ranker closed.
- [x] Do not hand off to runtime closure until the candidate artifact explicitly states:
  - `pre_bayes_targets`
  - `belief_targets`
  - `path_ranking_targets`
  - `execution_tree_targets`
  - `structural_feedback_required`
  - evidence: `/tmp/vrp-v2-loop-20260509-candidate-pack/factor_expression.json`
- [x] For each next candidate, write one explicit chain-level judgment:
  - `stopped_at_factor_iteration`
  - `stopped_at_pre_bayes`
  - `stopped_at_bbn`
  - `stopped_at_path_ranking`
  - `stopped_at_execution_tree`
  - `closed_loop_changed`
  - Slice verdict: `stopped_at_path_ranking` because `mature_rows=0`, `raw_scored_mature=0/30`, `production_validation=0/30`, while workflow still uses `candidate_set_only` scores.
- [x] Next practical slice: generate or import structural feedback rows / hot-plug ranker evidence so path-ranking can move beyond candidate-set scoring without breaking zero-config fallback.
  - hot-plug evidence: `runtime_selection=enabled_registered_model_ready`, `runtime_source=registered_model_artifact`, `runtime_matches=3` after `path_ranker_integration.py --register-runtime-artifact`.
  - structural feedback evidence: `structural_feedback_trade_enricher.py emit-probe` generated `structural-feedback-v1` from the rank-1 target row; `ict-engine update --feedback-file` consumed it; target export moved to `mature_rows=1`, `raw_scored_mature=1/30`.
  - remaining stop layer: `stopped_at_path_ranking_validation_floor` until 29 more honest structural-feedback observations exist.

### Not Yet

- [ ] Training a pseudo-label direction model as the main path
- [ ] Calling a generic high-Sharpe strategy a valid factor if it cannot separate two branches inside the same regime
- [ ] Treating `range_mean_reversion` as accepted just because the current structural branch names it while the execution tree remains observe/blocked
- [ ] Reopening runtime code just to make the loop look cleaner
- [ ] Treating `trainer_artifact=ready` as equivalent to a validated path-ranker loop
- [ ] Treating `candidate_set` runtime scoring as equivalent to mature external CatBoost closure
- [ ] Mixing new provider bootstrap or generic UX work into this loop board

## Ordered Execution Checklist

1. Capture market data for the target symbol/timeframe and record the provider path.
2. Infer the current market state / regime and write the regime anchor into the candidate pack.
3. Enumerate the branch candidates allowed under that regime.
4. Pick one branch pair to separate, for example `range_mean_reversion` versus `transition_confirmation`.
5. Generate or import one explicit factor candidate for that branch-pair separation.
6. Materialize or refresh its explicit candidate artifact pack in `/tmp/...`.
7. Record the factor-stage truth:
   - trade-density bucket
   - breadth / market coverage
   - resonance stack
   - claimed downstream targets
   - `regime_label`
   - `branch_id`
   - branch-pair discrimination result
8. Push it into the `pre-bayes` stage and record:
   - `pre-bayes-status --human`
   - `bridge` line
   - whether the gate is blocked, neutralized, or supportive
9. Push it into the `BBN` stage and record:
   - `auto-quant-prior-init` diff or applied result
   - if real trades exist, `auto-quant-ingest-real-trades`
   - whether the BBN layer actually changed any downstream prior/posterior belief worth keeping
10. Push it into the `CatBoost / path-ranking` stage and record:
   - `export-structural-path-ranking-target`
   - `policy-training-status --human`
   - whether the lane is blocked by:
     - no target rows
     - no mature rows
     - no calibration
     - no structural lineage
11. Push it into the `execution-tree` stage and record:
   - `workflow-status --human`
   - `workflow-status --phase ensemble-vote --human`
   - `workflow-status --phase structural-playbook --human`
   - `workflow-status --phase structural-recommended-path-bundle --human`
   - `execution_tree_trace.json`
12. Write one final chain verdict:
   - where the candidate stopped
   - what exact artifact/metric blocked it
   - whether the blocker is:
     - candidate quality
     - regime / pre-bayes
     - BBN evidence
     - path-ranking maturity
     - execution-tree behavior

## Real Command Floor

Use these exact current-mainline commands as the minimal closure floor.

### Factor / candidate handoff

```bash
python3 support/scripts/research/factor_candidate_pack.py \
  --manifest-json <strategy_library.json> \
  --strategy-name <strategy> \
  --candidate-spec-json <candidate_spec.json> \
  --autoresearch-status-json <autoresearch_status.json> \
  --output-dir /tmp/<candidate-pack>
```

### Pre-Bayes / 濾波

```bash
./target/debug/ict-engine pre-bayes-status \
  --symbol <SYMBOL> \
  --state-dir /tmp/<state> \
  --human
```

### BBN

```bash
./target/debug/ict-engine auto-quant-prior-init \
  --symbol <SYMBOL> \
  --state-dir /tmp/<state> \
  --library <strategy_library.json> \
  --dry-run
```

```bash
./target/debug/ict-engine auto-quant-ingest-real-trades \
  --symbol <SYMBOL> \
  --state-dir /tmp/<state> \
  --trades <realized_trades.jsonl> \
  --dry-run
```

### CatBoost / structural path-ranking

```bash
./target/debug/ict-engine export-structural-path-ranking-target \
  --symbol <SYMBOL> \
  --state-dir /tmp/<state>
```

```bash
./target/debug/ict-engine policy-training-status \
  --symbol <SYMBOL> \
  --state-dir /tmp/<state> \
  --human
```

### Execution tree

```bash
./target/debug/ict-engine workflow-status \
  --symbol <SYMBOL> \
  --state-dir /tmp/<state> \
  --human
```

```bash
./target/debug/ict-engine workflow-status \
  --symbol <SYMBOL> \
  --state-dir /tmp/<state> \
  --phase structural-playbook \
  --human
```

```bash
./target/debug/ict-engine workflow-status \
  --symbol <SYMBOL> \
  --state-dir /tmp/<state> \
  --phase structural-recommended-path-bundle \
  --human
```

## Current Known Blockers

### Blocker A: import rehearsal surface drift

- `auto-quant-results-import` on current `green-baseline` does **not** support `--dry-run`.
- Therefore this board must treat import rehearsal as:
  - copy a `/tmp/...` state dir
  - run the real import there
  - discard that state if the manifest/handoff is wrong

### Blocker B: path-ranking maturity gap

- Historical note: the first VRP state stopped here with `mature_rows=0`, `raw_scored_mature=0/30`, and `production_validation=0/30`.
- Current refresh on `/tmp/ict-engine-structural-replay-29/state` no longer stops here:
  - `export-structural-path-ranking-target`: `history_rows=35`, `history_mature_rows=33`, `history_rows_with_raw_path_score=35`, `history_rows_with_calibrated_path_prob=33`
  - `policy-training-status`: `raw_scored_mature=33/30`, `production_validation=33/30`, `observation_validation=30/30`
  - runtime: `runtime_selection=enabled_registered_model_ready`, `runtime_source=registered_model_artifact`
- Current interpretation: path-ranking maturity remains a required gate for new candidates, but this replay state has passed the 30-observation / 30-row floor.

### Blocker C: execution tree still reads candidate-set-level path support

- Historical note: the first VRP state read candidate-set path support.
- Current refresh no longer has that reader issue:
  - `workflow-status --human`: `Ranker: status=using_registered_model_artifact source=registered_model_artifact applied=1`
  - `workflow-status --phase ensemble-vote --human`: `action=execute_follow_through`, `confidence=0.976`
  - `workflow-status --phase structural-recommended-path-bundle --human`: `trend_follow_through`, `posterior=0.787`, `selected_prob=1.000`
  - `execution_tree_trace.json`: `branch=transition_guardrail`, `execution_bias=guarded`, `gate_status=observe`, `execution_score=0.5736691669503992`
- Current stop layer: execution-tree guardrail, not CatBoost attachment. The trace names low remaining regime duration / transition hazard as the immediate reason:
  - `execution_readiness=0.4648`
  - `hybrid_transition_hazard=0.607`
  - `duration_remaining_expected_bars=0.667`
  - `decision_hint=execution_guarded_due_to_low_remaining_regime_duration`

### Provider Matrix Requirement

- Current provider refresh used `/tmp/ict-current-provider-probe-20260510/provider-probes/`.
- YF/yfinance: actual QQQ 1h fetch succeeded with `71` rows after one HTTP 429 retry.
- Kraken: actual `PF_XBTUSD` 1h futures fetch succeeded with `360` rows.
- IBKR: plain repo runtime still lacks `redis`, but local gateway `127.0.0.1:4002` was reachable and an offline-`uv` IBKR SPY 1h fetch succeeded with `160` rows.
- TradingViewRemix: current process has no `ICT_ENGINE_TVREMIX_MCP_API_KEY`; the `market-data-harness` fetch was attempted and failed at that credential boundary.
- Rule for the next candidate: do not claim `data_blocked` from one provider; log YF, Kraken, IBKR, TradingViewRemix, local caches, and any reusable auxiliary artifacts separately.

## 2026-05-10 QQQ Regime-Branch Chain Evidence

Run root: `/tmp/ict-regime-branch-iteration-20260510-1`

### Provider / regime root

- YF QQQ 1h fetch and analyze succeeded:
  - candles: `/tmp/ict-regime-branch-iteration-20260510-1/candles/yf_QQQ_1h.json`
  - state: `/tmp/ict-regime-branch-iteration-20260510-1/state`
  - result: `TrendExpansion/BullTrendAcceleration`, `observe/fill_viable/passive`
- TradingViewRemix/MCP QQQ 1h fetch and analyze succeeded:
  - candles: `/tmp/ict-regime-branch-iteration-20260510-1/candles/tv_QQQ_1h.json`
  - state: `/tmp/ict-regime-branch-iteration-20260510-1/state_tv_QQQ_1h`
  - result: `TrendExpansion/BullTrendAcceleration`, `observe/fill_viable/passive`
- IBKR QQQ 1h fetch and analyze succeeded through the local gateway:
  - candles: `/tmp/ict-regime-branch-iteration-20260510-1/candles/ibkr_QQQ_1h.json`
  - state: `/tmp/ict-regime-branch-iteration-20260510-1/state_ibkr_QQQ_1h`
  - result: `RangeConsolidation/WideRange`, `observe/transition_guardrail/guarded`
- Kraken futures fetch succeeded for `PF_XBTUSD`:
  - candles: `/tmp/ict-regime-branch-iteration-20260510-1/candles/kraken_PF_XBTUSD_1h.json`
  - interpretation: provider reachability evidence only, not QQQ branch-performance evidence.

Current root is therefore not a single clean `range_choppy` label. It is `range_consolidation_with_provider_disagreement`: IBKR supports the range/guardrail anchor, while YF and TradingView support trend expansion. This disagreement is encoded as `market_likelihood_evidence`, not hidden.

### Auto-Quant candidate and branch rows

- Auto-Quant command evidence:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/10_auto_quant_run_tomac_hist.out`
  - strategy: `TomacNQ_KillzoneBreakout`
  - pair: `QQQ/USD`
  - trades: `74`
  - total profit: `+6.98%`
  - Sharpe: `0.2207`
  - Sortino: `0.3825`
  - max drawdown: `-4.2049%`
  - win rate: `52.7027%`
  - profit factor: `1.2501`
- Strategy library artifact:
  - `/tmp/ict-regime-branch-iteration-20260510-1/strategy_library_qqq_tomac_hist.json`
  - import log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/11_auto_quant_results_import.out`
  - import result: `n_ok=1`, `matched=1`, `manifest_only=[]`, `log_only=[]`
- Candidate pack:
  - `/tmp/ict-regime-branch-iteration-20260510-1/candidate-pack/factor_expression.json`
  - `/tmp/ict-regime-branch-iteration-20260510-1/candidate-pack/factor_eval_grid_summary.json`
  - `/tmp/ict-regime-branch-iteration-20260510-1/candidate-pack/transfer_score.json`
  - aggregate density: `trade_count=74`, label `thin`
  - transfer status: `single_market_only`, score `0.0`
- Branch-ranking rows:
  - `/tmp/ict-regime-branch-iteration-20260510-1/branch_ranking_rows_qqq_20260510.json`
  - rows:
    - `range_mean_reversion`: native control evidence only, `volatility_mean_reversion` return `-0.86%`, stopped at `factor_iteration`
    - `transition_confirmation`: Auto-Quant breakout evidence, positive but weak, stopped at `path_ranking_or_execution_tree_pending`
    - `stress_de_risk`: provider disagreement + observe/guardrail evidence, stopped at `execution_tree_observe`

Interpretation: the first real Auto-Quant candidate is useful branch evidence for `transition_confirmation`, not proof that `range_mean_reversion` is viable and not a promotable high-Sharpe strategy.

### Pre-Bayes / BBN

- Pre-Bayes command:
  - `/tmp/ict-regime-branch-iteration-20260510-1/logs/12_pre_bayes_status.out`
  - result: `gate=pass_neutralized`, `soft_evidence=yes`
  - bridge: `long=0.551`, `short=0.537`, `mtf=bullish`, `align=1.000`, `entry_align=0.839`
- BBN dry-run:
  - `/tmp/ict-regime-branch-iteration-20260510-1/logs/13_auto_quant_prior_init_dry_run.out`
  - `evidence_value_gate_passed=true`
  - `n_win=39`, `n_loss=35`, `trade_count=74`
  - final probabilities preview: `[0.5731664390243902, 0.000002146341463414634, 0.42683141463414637]`
- BBN apply in isolated `/tmp` state:
  - `/tmp/ict-regime-branch-iteration-20260510-1/logs/21_auto_quant_prior_init_apply.out`
  - `dry_run=false`
  - same final probabilities as preview
  - artifact: `/tmp/ict-regime-branch-iteration-20260510-1/state/auto-quant/QQQ/auto_quant_prior_init_QQQ_20260510T093548.477591000Z.json`

No `realized_trades.jsonl` was produced in this slice, so `auto-quant-ingest-real-trades` was not run. The BBN evidence here comes from the imported strategy-library aggregate wins/losses, not realized live trades.

### CatBoost / path-ranking / execution tree

- Structural path target export before and after BBN apply stayed at:
  - logs:
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/14_export_structural_path_ranking_target.out`
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/22_export_structural_path_ranking_target_after_bbn_apply.out`
  - `rows=3`
  - `history_rows=3`
  - `mature_rows=0`
  - `history_mature_rows=0`
  - `rows_with_raw_path_score=0`
  - `rows_with_calibrated_path_prob=0`
  - `rows_with_training_weight=0`
- Policy training / CatBoost status:
  - logs:
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/15_policy_training_status.out`
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/23_policy_training_status_after_bbn_apply.out`
  - `raw_scored_mature=0/30`
  - `production_validation=0/30`
  - `observation_validation=0/30`
  - `trainer_artifact=missing`
  - `runtime_selection=disabled`
  - `score_model_family=unknown`

CatBoost branch score is therefore not available for this run. Do not fabricate it from the weak Auto-Quant Sharpe; the correct stop layer is `stopped_at_path_ranking_validation_floor`.

- Workflow / execution-tree outcome after BBN apply:
  - `/tmp/ict-regime-branch-iteration-20260510-1/logs/24_workflow_status_after_bbn_apply.out`
  - `/tmp/ict-regime-branch-iteration-20260510-1/logs/25_workflow_structural_recommended_path_bundle_after_bbn_apply.out`
  - `/tmp/ict-regime-branch-iteration-20260510-1/logs/26_workflow_ensemble_vote_after_bbn_apply.out`
  - `workflow-status --human`: `action_blocked`
  - block: `user_selected_historical_data_missing`
  - structural path: `trend_follow_through`, `posterior=0.568`, `selected_prob=0.367`
  - ensemble vote: `action=Observe`, `confidence=0.568`
  - trace summary: `branch=fill_viable`, `execution_bias=passive`, `gate_status=observe`, `ranker_validation_ready=false`

### Slice verdict

- Real chain order executed:
  - provider-backed market data
  - regime inference
  - regime-rooted branch rows
  - Auto-Quant backtest evidence
  - strategy-library import
  - pre-bayes check
  - BBN prior preview and apply
  - structural path target export
  - policy-training / CatBoost readiness check
  - workflow-status / execution-tree readback
- Candidate judgment:
  - `TomacNQ_KillzoneBreakout` is branch evidence for `transition_confirmation`.
  - It is not a promoted factor and not a generic high-Sharpe strategy.
  - `range_mean_reversion` remains unproven for this QQQ slice.
  - `stress_de_risk` remains live because all workflow surfaces are observe/blocked and providers disagree.
- Stopping layer:
  - `stopped_at_path_ranking_validation_floor`
  - execution tree also remains `observe/action_blocked`
- Next smallest honest action:
  - generate more branch-labeled structural feedback rows for the same `regime_label + branch_id` surface, or run additional branch-specific Auto-Quant candidates for `range_mean_reversion_viability` and `stress_de_risk` before attempting CatBoost training.

## 2026-05-10 QQQ Structural Feedback + CatBoost Replay Evidence

Run root: `/tmp/ict-regime-branch-iteration-20260510-1`

### Completion-audit checklist for the user objective

- Named board updated:
  - target file: `support/docs/plans/2026-05-09-factor-iteration-pre-bayes-bbn-catboost-execution-tree-todo.md`
  - status: covered by this section and the prior `2026-05-10 QQQ Regime-Branch Chain Evidence` section.
- Real provider use:
  - IBKR: `/tmp/ict-regime-branch-iteration-20260510-1/candles/ibkr_QQQ_1h.json`
  - TradingViewRemix/MCP: `/tmp/ict-regime-branch-iteration-20260510-1/candles/tv_QQQ_1h.json`
  - YF: `/tmp/ict-regime-branch-iteration-20260510-1/candles/yf_QQQ_1h.json` and `/tmp/ict-regime-branch-iteration-20260510-1/candles/yf_QQQ_1h_2024_2025.json`
  - Kraken: `/tmp/ict-regime-branch-iteration-20260510-1/candles/kraken_PF_XBTUSD_1h.json`
  - status: covered. Kraken remains provider reachability / cross-market evidence, not QQQ branch-performance evidence.
- Auto-Quant operated:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/10_auto_quant_run_tomac_hist.out`
  - imported library: `/tmp/ict-regime-branch-iteration-20260510-1/strategy_library_qqq_tomac_hist.json`
  - status: covered. The imported candidate is branch evidence for `transition_confirmation`, not a promoted high-Sharpe factor.
- Filter / pre-bayes operated:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/12_pre_bayes_status.out`
  - status: covered, `gate=pass_neutralized`.
- BBN operated:
  - dry-run log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/13_auto_quant_prior_init_dry_run.out`
  - apply log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/21_auto_quant_prior_init_apply.out`
  - status: covered, applied once in isolated `/tmp` state.
- CatBoost operated:
  - replay target: `/tmp/ict-regime-branch-iteration-20260510-1/structural-replay-qqq-36/state/QQQ/policy_training/structural_path_ranking_target_history.csv`
  - mature-only trainer input: `/tmp/ict-regime-branch-iteration-20260510-1/structural-replay-qqq-36/state/QQQ/policy_training/structural_path_ranking_target_history_mature_only.csv`
  - trained model: `/tmp/ict-regime-branch-iteration-20260510-1/catboost-path-ranker-qqq-replay-36-mature-only/catboost_model.cbm`
  - trainer artifact: `/tmp/ict-regime-branch-iteration-20260510-1/catboost-path-ranker-qqq-replay-36-mature-only/trainer_artifact.json`
  - status: covered for path-ranking training/runtime eligibility; direct execution-tree trace still does not mark the CatBoost score as visible.
- Execution tree / workflow operated:
  - workflow logs:
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/38_workflow_status_after_catboost_enable_qqq.out`
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/42_workflow_status_refresh_after_catboost_enable_qqq.out`
  - trace summary:
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/46_execution_tree_trace_after_workflow_refresh_qqq.out`
  - status: covered; final action remains observe/guarded at execution-tree level.

### Structural feedback replay

- Command:
  - `python3 support/scripts/auto_quant_external/structural_feedback_replay_harness.py --candles /tmp/ict-regime-branch-iteration-20260510-1/candles/yf_QQQ_1h_2024_2025.json --output-root /tmp/ict-regime-branch-iteration-20260510-1/structural-replay-qqq-36 --symbol QQQ --count 36 --lookback 120 --horizon 16 --threshold 0.001 --prior-state /tmp/ict-regime-branch-iteration-20260510-1/state`
- Log:
  - `/tmp/ict-regime-branch-iteration-20260510-1/logs/27_structural_feedback_replay_qqq_36.out`
- Summary:
  - observations: `36`
  - outcomes: `24 win`, `11 loss`, `1 invalidated`
  - final path from workflow: `range_mean_reversion`
  - path history: `total=36`, `wins=24`, `losses=11`, `invalidated=1`, `avg_pnl=0.0037`
- Target export after replay:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/28_export_structural_path_ranking_target_replay_qqq.out`
  - `history_rows=1375`
  - `history_mature_rows=1367`
  - `history_rows_with_raw_path_score=1366`
  - `history_rows_with_calibrated_path_prob=1366`
  - `history_rows_with_training_weight=1367`
- Policy status after replay:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/29_policy_training_status_replay_qqq.out`
  - `raw_scored_mature=1366/30`
  - `production_validation=1366/30`
  - `observation_validation=36/30`
  - `calibration=evaluated`
  - `quality_ready=true`

This replay removes the prior `path-ranking_validation_floor` blocker for the replay state.

### CatBoost training and runtime wiring

- Non-authoritative first trainer run:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/30_catboost_path_ranker_train_qqq_replay_36.out`
  - rejected reason: the unfiltered history CSV contained censored blank labels, and the trainer fell back to pseudo-labels from `structural_baseline_score`.
  - status: do not use this as CatBoost evidence.
- Mature-only input:
  - file: `/tmp/ict-regime-branch-iteration-20260510-1/structural-replay-qqq-36/state/QQQ/policy_training/structural_path_ranking_target_history_mature_only.csv`
  - rows: `1367`
  - label counts: `1.0=881`, `0.0=486`
- Authoritative CatBoost training:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/32_catboost_path_ranker_train_qqq_replay_36_mature_only_offline.out`
  - model: `/tmp/ict-regime-branch-iteration-20260510-1/catboost-path-ranker-qqq-replay-36-mature-only/catboost_model.cbm`
  - trained rows: `1367`
  - label distribution: `0.0=486`, `1.0=881`
  - feature set: `structural_baseline_score`
  - note: this is a real CatBoost fit on mature calibrated labels, but the feature set is still thin.
- Apply current CatBoost scores:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/33_catboost_path_ranker_apply_qqq_current.out`
  - scores file: `/tmp/ict-regime-branch-iteration-20260510-1/catboost-path-ranker-qqq-replay-36-mature-only/path_scores_catboost_current.csv`
  - note: the apply helper still prints a label fallback line for the one-row current target; that affects helper diagnostics, not the already-trained model.
- Import CatBoost scores into ict-engine:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/34_apply_catboost_scores_qqq_replay_state.out`
- Register trainer artifact:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/35_register_catboost_trainer_artifact_qqq_replay_state.out`
  - `trainer_artifact_status=runtime_eligible`
  - `trainer_artifact_model_family=catboost`
  - `trainer_artifact_trained_rows=1367`
  - `production_validation_ready=true`
  - `observation_validation_ready=true`
- Enable runtime reuse:
  - logs:
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/36_enable_structural_path_ranking_runtime_qqq_replay_state.out`
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/41_enable_structural_path_ranking_runtime_prefer_history_qqq.out`
  - final status:
    - `runtime_selection=enabled_candidate_set_ready`
    - `runtime_mode=prefer_history`
    - `runtime_source=candidate_set`
    - `score_model_family=catboost`
    - `score_source=external_model`
    - `runtime_matches=1`
- Policy status after runtime enable:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/37_policy_training_status_after_runtime_enable_qqq.out`
  - `raw_scored_mature=1367/30`
  - `production_validation=1367/30`
  - `observation_validation=36/30`
  - `trainer_artifact=ready`
  - `trainer_status=runtime_eligible`
  - `runtime_selection=enabled_candidate_set_ready`
  - `score_model_family=catboost`
  - `score_source=external_model`
  - `runtime_matches=1`

### Workflow / execution-tree after CatBoost

- Workflow status after CatBoost runtime enable:
  - logs:
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/38_workflow_status_after_catboost_enable_qqq.out`
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/42_workflow_status_refresh_after_catboost_enable_qqq.out`
  - result:
    - `Block: none`
    - `Ranker: status=using_candidate_set_scores source=candidate_set applied=1 artifact=0 candidate=1 history=0 lb=0.567 gate=pass`
    - `Path: ...range_mean_reversion... total=36 wins=24 losses=11 invalidated=1 avg_pnl=0.0037`
- Structural recommended path:
  - logs:
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/39_workflow_structural_path_after_catboost_enable_qqq.out`
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/43_workflow_structural_path_refresh_after_catboost_enable_qqq.out`
  - result:
    - `range_mean_reversion`
    - `posterior=0.787`
    - `selected_prob=1.000`
- Ensemble vote:
  - logs:
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/40_workflow_ensemble_after_catboost_enable_qqq.out`
    - `/tmp/ict-regime-branch-iteration-20260510-1/logs/44_workflow_ensemble_refresh_after_catboost_enable_qqq.out`
  - result:
    - `action=favor_mean_reversion_only`
    - `confidence=0.787`
- Execution tree trace:
  - log: `/tmp/ict-regime-branch-iteration-20260510-1/logs/46_execution_tree_trace_after_workflow_refresh_qqq.out`
  - result:
    - `branch=transition_guardrail`
    - `execution_bias=guarded`
    - `gate_status=observe`
    - `decision_hint=execution_guarded_due_to_low_remaining_regime_duration`
    - `ranker_validation_ready=true`
    - `path_ranker_score_visible_to_execution_tree=false`
    - `path_ranker_score_used_by_execution_tree=false`

### Updated slice verdict

- The first QQQ slice stopped at `path-ranking_validation_floor`.
- This replay slice moved past that floor:
  - structural feedback maturity is ready
  - CatBoost trained on mature labels
  - trainer artifact is registered
  - runtime reuse is enabled
  - workflow reads ranker support through candidate-set scores
- New stop layer:
  - `stopped_at_execution_tree_guardrail_and_trace_visibility_gap`
- Interpretation:
  - `range_mean_reversion` now has replay-backed branch evidence in this QQQ state.
  - This is still retrospective replay evidence, not live production proof.
  - The direct `execution_tree_trace.json` still does not expose CatBoost as used by the execution tree, even though `policy-training-status` shows `score_model_family=catboost` and workflow ranker support is active.
- Next honest action:
  - either inspect/fix the execution-tree trace visibility gap if direct trace consumption is required, or run another provider-backed forward slice to check whether the CatBoost-backed `range_mean_reversion` branch survives outside YF replay.

## Success Standard

This board is successful only if a later iteration can say all of the following with explicit artifacts:

- the factor candidate is explicit and reviewable
- the current regime root is explicit
- the branch candidate or branch pair is explicit
- the market likelihood evidence for that branch is explicit
- the pre-bayes / filter gate result is explicit
- the BBN prior/posterior effect is explicit
- the structural path-ranking maturity state is explicit
- the execution-tree outcome is explicit
- the exact stopping layer is explicit

If a candidate stops before the execution tree changes, that still counts as a successful iteration **only if the stopping layer is honestly identified and recorded**.
