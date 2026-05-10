# Regime-Conditional Auto-Quant Profitability TODO Board

Date: 2026-05-10

Purpose: this board is responsible only for deciding whether an Auto-Quant recipe is stably profitable inside a regime that Board A has already accepted. It does not revalidate regime confidence.

Authority inputs:
- Board A accepted regime packet: `docs/plans/2026-05-10-actionable-regime-confidence-todo.md`
- Research scan: `docs/market-regime-profitable-strategy-research-2026-05-10.md`
- Auto-Quant recipe and backtest artifacts
- ict-engine downstream consumption: pre-Bayes gate, BBN, CatBoost/path-ranker, execution tree, feedback updates

## Loop Prompt

Use this prompt for a `/goal` loop that works only on Board B:

```text
You are running Board B: Regime-Conditional Auto-Quant Profitability.

Mission:
Given one accepted regime packet from Board A, determine whether one Auto-Quant recipe is stably profitable inside that regime after costs, stress, and downstream consumption checks.

Scope:
- Work only on recipe profitability inside an accepted regime.
- Do not re-score or redefine regime confidence.
- Do not claim stable profitability without a Board A accepted regime packet.
- Do not start an additional heavy Auto-Quant training process if two factor-training processes are already running locally.

Required loop order:
1. Update the Current Cursor in this file before starting.
2. Confirm the accepted regime packet from Board A.
3. Select exactly one Auto-Quant recipe for this loop.
4. Run or consume one backtest/replay artifact.
5. Score the recipe with RC-SPA.
6. If the recipe passes, push the artifact through pre-Bayes, BBN, CatBoost/path-ranker, and execution tree consumption checks.
7. Write artifact paths, scores, gate states, blocker, and next action back into this file.

Completion standard:
- One Evidence Ledger row is added or updated.
- The Current Cursor is updated.
- A single next action remains.
- If the recipe fails, write a hard failure reason instead of changing the metric after the fact.
```

## Current Cursor

| Field | Current Value | Update Rule |
|---|---|---|
| board_state | waiting_for_board_a | `waiting_for_board_a`, `active`, `research_watch`, `stable_candidate`, `pilot_candidate`, `rejected`, `blocked` |
| last_loop_id | 2026-05-10-board-b-bootstrap | Use timestamp or run id |
| accepted_regime_id | none_yet | Must come from Board A |
| accepted_regime_artifact | none_yet | Path to Board A packet |
| auto_quant_recipe | none_selected_yet | Recipe id, hash, or artifact path |
| backtest_run_root | none_recorded_yet | Use `/tmp/...` or `/private/tmp/...` |
| stable_profit_score | not_scored_yet | RC-SPA score from 0 to 100 |
| hard_gate_result | waiting_for_board_a | `pass` or `fail:<reason>` |
| downstream_consumption | not_started | `not_started`, `pre_bayes`, `bbn`, `catboost`, `execution_tree`, `feedback_updated` |
| blocker | waiting for Board A accepted regime packet | Single blocking reason |
| next_action | B1: wait for Board A accepted regime packet | Keep exactly one next action |

## Stable Profitability Algorithm: RC-SPA

RC-SPA means `Regime-Conditional Stable Profitability Algorithm`.

Inputs:
- `accepted_regime_id`: accepted by Board A.
- `recipe_id`: Auto-Quant falsifiable recipe id or hash.
- `trades_or_returns`: chronological trades or bar-level strategy returns.
- `cost_model`: commission, spread, slippage, borrow or margin, funding, assignment or tail cost.
- `market_context`: market, timeframe, session, liquidity bucket, holding horizon.

Splitting rules:
- Use chronological walk-forward evaluation.
- Use purged CV plus embargo when labels or trades overlap.
- Use net returns only.
- Do not put the same macro/event shock window in both train and test.

Hard gates:
- `accepted_regime_id` is present and traceable to Board A.
- `min_total_trades >= 100` for liquid intraday strategies such as NQ, ES, BTC, or ETH.
- `min_total_trades >= 50` only for lower-frequency, options, gold, or commodity swing strategies.
- `min_test_folds >= 4`
- `min_trades_per_test_fold >= 10`
- `fold_positive_rate >= 0.75`
- `bootstrap_edge_lcb_5pct > 0`
- `cost_stress_survival = true` under 2x cost/slippage.
- `pbo <= 0.25`
- `dsr > 0`
- `tail_loss_p95 <= configured_risk_budget`
- `regime_specificity_ratio >= 1.20`

Direct rejection states:
- `reject_missing_accepted_regime`
- `reject_thin_trades`
- `reject_no_positive_edge`
- `reject_cost_fragile`
- `reject_overfit_risk`
- `reject_tail_risk`
- `reject_no_regime_specificity`
- `research_watch_downstream_not_consumed`

Score:

```text
edge_score        = clamp(bootstrap_edge_lcb_5pct / target_edge, 0, 1)
fold_score        = fold_positive_rate
depth_score       = clamp(total_trades / required_trades, 0, 1)
dsr_score         = clamp(dsr / target_dsr, 0, 1)
pbo_score         = 1 - clamp(pbo / 0.25, 0, 1)
cost_score        = 1 if 2x cost stress survives else 0
drawdown_score    = 1 - clamp(max_drawdown_p95 / drawdown_budget, 0, 1)
specificity_score = clamp((regime_specificity_ratio - 1.0) / 0.5, 0, 1)

RC_SPA =
  100 * (
    0.20 * edge_score +
    0.15 * fold_score +
    0.15 * depth_score +
    0.15 * dsr_score +
    0.10 * pbo_score +
    0.10 * cost_score +
    0.10 * drawdown_score +
    0.05 * specificity_score
  )
```

Promotion levels:
- `reject`: any hard gate fails, or `RC_SPA < 60`.
- `research_watch`: hard gates pass and `60 <= RC_SPA < 75`; collect more evidence, do not enter execution tree.
- `stable_candidate`: hard gates pass and `75 <= RC_SPA < 85`; may enter pre-Bayes, BBN, and CatBoost/path-ranker paper path.
- `pilot_candidate`: hard gates pass and `RC_SPA >= 85`; may enter execution tree simulation or small pilot only after downstream consumption is verified.

Strategy-specific stress add-ons:
- VRP, put-writing, or short-vol: require gap risk, margin stress, VIX/VIX3M, VVIX/VIX, and event filter checks.
- Crypto: require liquidity cliff, funding, weekend/session, and exchange outage checks.
- Trend following: require whipsaw and volatility-target drawdown checks.
- Mean reversion: require trend-break loss and stop-execution checks.

## Output Contract To ict-engine

Board B may hand off only a profitability packet with these fields:

```text
accepted_regime_id
recipe_id
recipe_artifact_path
backtest_artifact_path
total_trades
test_folds
fold_positive_rate
bootstrap_edge_lcb_5pct
pbo
dsr
cost_stress_result
tail_loss_p95
regime_specificity_ratio
rc_spa
promotion_level
downstream_consumption_status
```

If downstream consumption is not verified, the packet cannot be treated as execution-tree-ready.

## Done

- [x] Board B is split into its own English file.
- [x] Scope is limited to stable profitability inside an already accepted regime.
- [x] RC-SPA defines stable profitability with net returns, trade depth, fold consistency, cost stress, PBO/DSR, drawdown, and regime specificity.
- [x] Low trade count is an explicit rejection state, not weak evidence.

## Next

- [ ] B1. Wait for Board A to produce an accepted regime packet.
- [ ] B2. Select exactly one Auto-Quant recipe for the accepted regime.
- [ ] B3. Run or consume one backtest artifact with net returns, costs, slippage, trades, and fold metadata.
- [ ] B4. Compute all hard gates and `RC_SPA`.
- [ ] B5. If `stable_candidate` or `pilot_candidate`, pass the artifact through pre-Bayes, BBN, CatBoost/path-ranker, and execution tree consumption checks.
- [ ] B6. If execution tree abstains, classify the blocker as regime evidence, profitability evidence, or downstream consumption evidence.
- [ ] B7. After pilot or replay feedback exists, update BBN priors and CatBoost/path-ranker training data before searching for another recipe.

## Not Yet

- [ ] B8. Add a dedicated tail-risk stress pack for VRP and short-vol recipes.
- [ ] B9. Add a dedicated liquidity/funding stress pack for crypto recipes.
- [ ] B10. Add capacity and correlation-crash stress for cross-asset or network-momentum recipes.
- [ ] B11. Maintain a recipe leaderboard only for recipes that pass hard gates.

## Evidence Ledger

| Loop ID | Accepted Regime | Recipe | Trades | RC-SPA | Gate Result | Downstream | Artifact | Next |
|---|---|---|---:|---:|---|---|---|---|
| 2026-05-10-board-b-bootstrap | none | none | 0 | 0 | waiting_for_board_a | not_started | this file | B1 |
