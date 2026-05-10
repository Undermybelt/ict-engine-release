# Actionable Regime Confidence TODO Board

Date: 2026-05-10

Purpose: this board is responsible only for deciding whether the system can identify an actionable market regime with 95%-99% calibrated confidence. It does not decide whether any Auto-Quant recipe is profitable.

Authority inputs:
- `docs/market-regime-profitable-strategy-research-2026-05-10.md`
- Provider paths: IBKR, TradingView MCP, yfinance, Kraken, local cached data, Auto-Quant cached data
- Downstream consumers: pre-Bayes gate, BBN evidence, CatBoost/path-ranker context, execution tree readiness fields

## Loop Prompt

Use this prompt for a `/goal` loop that works only on Board A:

```text
You are running Board A: Actionable Regime Confidence.

Mission:
Determine whether a market regime/filter can be identified with 95%-99% calibrated confidence and can be consumed by the downstream ict-engine chain.

Scope:
- Work only on regime/filter confidence.
- Do not evaluate Auto-Quant recipe profitability.
- Do not claim a tradable strategy exists.
- Do not start an additional heavy Auto-Quant training process if two factor-training processes are already running locally.

Required loop order:
1. Update the Current Cursor in this file before starting.
2. Check provider/local data availability: IBKR, TradingView MCP, yfinance, Kraken, local cache, Auto-Quant cache.
3. Select at most two markets and two timeframes for this loop.
4. Produce one regime-confidence evidence packet.
5. Calibrate the candidate using chronological calibration/test splits.
6. Write the artifact path, support, calibrated confidence, acceptance state, blocker, and next action back into this file.

Completion standard:
- One Evidence Ledger row is added or updated.
- The Current Cursor is updated.
- A single next action remains.
- If no rule passes, write an explicit abstain reason instead of lowering thresholds.
```

## Current Cursor

| Field | Current Value | Update Rule |
|---|---|---|
| board_state | active | `active`, `blocked`, `accepted_95`, `accepted_99`, `abstain` |
| last_loop_id | 2026-05-10-board-a-bootstrap | Use timestamp or run id |
| active_market_set | none_selected_yet | Max two markets per loop |
| active_timeframes | none_selected_yet | Max two timeframes per loop |
| current_run_root | none_recorded_yet | Use `/tmp/...` or `/private/tmp/...` |
| provider_status | research_scan_only | Record IBKR, TradingView MCP, yfinance, Kraken, local cache, Auto-Quant cache |
| candidate_regime | none_selected_yet | Regime label plus horizon |
| confidence_lane | 95_first_then_99 | `95`, `99`, or `abstain` |
| accepted_gate | none | Write the exact accepted gate or `none` |
| blocker | no live calibration packet recorded yet | Single blocking reason |
| next_action | A1: build provider availability packet | Keep exactly one next action |

## Regime Definition

Board A accepts only execution-native regime events:

```text
event = market + timeframe + regime_id + horizon + allowed_action
```

Recommended regime labels:
- `TrendExpansion`
- `RangeConsolidation`
- `ExtremeStress`
- `ReversalBrewing`
- `ThinLiquidity`
- `Unknown`

The event must map to downstream truth labels:
- `ReleaseAllowed`: the future window allows action without bad-loss or tail-loss breach.
- `LowTransitionHazard`: the regime is unlikely to switch before the minimum holding horizon.
- `ReadinessObserveOrReady`: execution tree readiness is not hard-blocked.
- `DurationViable`: expected regime duration covers the strategy's minimum holding horizon.
- `PathSpecificEdge`: the relevant downstream path has positive forward payoff after cost.

## Confidence Standard

Do not accept raw model probability as confidence. Confidence must be calibrated by time-ordered out-of-sample evidence.

95% acceptance:
- `precision_wilson_lcb_95 >= 0.95`
- `calibration_support >= 120 windows`
- `test_support >= 60 windows`
- `ece <= 0.05`
- `coverage >= 0.03`
- Evidence covers at least two different time periods, markets, or timeframe combinations.

99% acceptance:
- `precision_wilson_lcb_99 >= 0.99`
- `calibration_support >= 300 windows`
- `test_support >= 120 windows`
- `ece <= 0.02`
- `coverage >= 0.02`
- Evidence covers at least three different time periods, markets, or timeframe combinations.

Valid failure states:
- `abstain_no_calibrated_release_rule`
- `target_schema_bad`
- `support_too_thin`
- `provider_gap`
- `transition_hazard_high`
- `duration_not_viable`
- `execution_relevance_missing`

## Output Contract For Board B

Board A may hand off only an accepted regime packet with these fields:

```text
accepted_regime_id
market
timeframe
horizon
allowed_action
confidence_lane
precision_wilson_lcb
calibration_support
test_support
ece
coverage
transition_hazard
duration_viable
downstream_evidence_fields
artifact_path
```

If Board A does not produce this packet, Board B must remain blocked.

## Done

- [x] Board A is split into its own English file.
- [x] Scope is limited to actionable regime confidence, not profitability.
- [x] The board rejects raw model probability and requires calibrated chronological evidence.
- [x] The board records provider availability instead of treating one failed provider as a data blocker.

## Next

- [ ] A1. Create the provider availability packet for IBKR, TradingView MCP, yfinance, Kraken, local cache, and Auto-Quant cache.
- [ ] A2. Select the smallest useful market/timeframe set for the first packet.
- [ ] A3. Generate candidate regime windows with `regime_id`, `horizon`, `transition_hazard`, `duration_viable`, and `allowed_action`.
- [ ] A4. Run chronological calibration/test evaluation and compute Wilson lower confidence bounds, ECE, coverage, and support.
- [ ] A5. Emit an accepted regime packet only if the 95% or 99% gate passes.
- [ ] A6. If two consecutive loops return `abstain_no_calibrated_release_rule`, change the target schema or regime family instead of relaxing thresholds.

## Not Yet

- [ ] A7. Add realized covariance and correlation eigenstructure as `ExtremeStress` or `ReversalBrewing` evidence.
- [ ] A8. Add jump-model or persistence-penalty features to `transition_hazard`.
- [ ] A9. Normalize TradingView regime formulas into feature recipes without trusting script profitability.
- [ ] A10. Add order-flow entropy only after tick or order-flow data is available.

## Evidence Ledger

| Loop ID | Run Root | Provider Status | Market/TF | Regime | Support | Gate Result | Artifact | Next |
|---|---|---|---|---|---:|---|---|---|
| 2026-05-10-board-a-bootstrap | none | research_scan_only | none | none | 0 | board_created | this file | A1 |
