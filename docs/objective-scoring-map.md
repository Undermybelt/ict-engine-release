# Objective Scoring Map

## Goal

This document explains how ict-engine objective scoring currently works, where scores come from, and which fields can mislead research if they are null, hardcoded, or contaminated by state.

## Current objectives

`ResearchObjectiveMode` currently has:
- `generic`
- `expansion_manipulation`

Most current factor work uses:
- `expansion_manipulation`

## Key distinction: two scoring layers

There are two related but distinct score surfaces:

### 1. Factor scorecard composite score
Stored in ranking / learning state as:
- `best_factor_composite_score`
- `scorecard.composite_score`

Under `expansion_manipulation`, this score is overwritten by an objective-specific score. It is not the generic IC/IR/backtest composite.

### 2. Mechanical mutation score
Used by mutation evaluation:
- `score_before`
- `score_after`
- `score_delta`

This score wraps selected fields from `FactorMutationMetricSet`.

## Expansion-manipulation factor scorecard formula

The objective scorecard roughly follows:

```text
objective_score = (
    expansion_balanced_accuracy      * 0.45
  + expansion_directional_accuracy   * 0.20
  + expansion_fit_score              * 0.15
  + bridge_gap_score                 * 0.10
  + selected_win_probability         * 0.10
  + gate_adjustment
)
* objective_jump_weight
* objective_market_shrink_weight
```

Where:
- `bridge_gap_score = (bridge_gap / 0.25).clamp(0.0, 1.0)`
- `gate_adjustment`:
  - `pass_hard` -> `+0.10`
  - `pass_neutralized` -> `+0.03`
  - `observe_only` -> `-0.12`
- `objective_market_shrink_weight` comes from evidence credibility / market-family shrink logic

## Mechanical mutation score formula

For `expansion_manipulation`:

```text
mechanical_score =
    best_factor_composite_score             * 0.60
  + expansion_balanced_accuracy             * 0.20
  + expansion_directional_accuracy          * 0.10
  + expansion_selected_win_probability      * 0.05
  + bridge_gap_score                        * 0.03
  + multi_timeframe_alignment_score         * 0.04
  + multi_timeframe_entry_alignment_score   * 0.03
  - pre_bayes_soft_evidence_divergence_count * 0.05
  + directional_bias_bonus                  * 0.02
```

Important:
- If `evaluate_expansion_preview=false`, many expansion fields may be null and contribute zero.
- This creates dead-weight unless explicitly understood.

## Known historical pitfalls already fixed

### 1. `evaluate_expansion_preview=false` dead weights
Problem:
- A large part of `mechanical_mutation_score` was null/default because expansion preview fields were not populated.

Fix / current rule:
- For objective-sensitive mutation evaluation, use `evaluate_expansion_preview=true` when you need full score decomposition.
- For pure parameter sensitivity, `false` may be useful, but label it as such.

### 2. hardcoded expansion scoring params
Problem:
- Some expansion scoring used hardcoded `(lookback=20, expansion_threshold=1.5)`.
- This rewarded baseline defaults and obscured whether mutation params were truly being evaluated.

Current rule:
- Check that scoring derives from the mutated registry parameters where appropriate.
- Any future hardcoded scoring values must be surfaced in reports.

## Current bottlenecks found empirically

Recent experiments show:
- `expansion_balanced_accuracy` and `expansion_directional_accuracy` are near saturation (`~0.98`) for baseline defaults.
- `selected_win_probability` is around `0.54` and has limited sensitivity to structure params.
- `bridge_gap` is very small (`~0.007`), so its score contribution is tiny.
- `objective_market_shrink_weight` can pin score down to around `0.55` multiplier.

Therefore the strongest remaining bottlenecks are:
1. `evidence_quality_score`
2. `pre_bayes_gate_status`
3. `objective_market_shrink_weight`
4. `bridge_gap`

## Evidence-quality / shrink logic

`objective_market_shrink_weight` is driven by credibility:

```text
raw_shrink = (1.0 - credibility_score) * (1.0 + objective_bias + market_bias)
shrink_weight = (1.0 - raw_shrink).clamp(0.55, 1.0)
```

For `expansion_manipulation` and futures-index markets, low credibility can pin shrink at the floor.

Practical effect:
- Even strong expansion metrics can be heavily discounted if evidence quality is low.

## Gate status effect

Gate status affects objective score via `gate_adjustment`:
- `pass_hard` is meaningfully better than `pass_neutralized`
- `observe_only` is heavily penalized

Research implication:
- A mutation that slightly improves factor metrics but regresses gate status should usually be rejected.

## How to read a mutation result

Do not read only `score_delta`.

Read in this order:
1. `accepted`
2. `score_delta`
3. `failure_tags`
4. `metrics_before.best_factor_composite_score`
5. `metrics_after.best_factor_composite_score`
6. `pre_bayes_gate_status`
7. `pre_bayes_bridge_probability_gap`
8. `top_factor_names`
9. `evaluate_expansion_preview`

## Dead-weight warning rules

A result is suspicious if:
- expansion metrics are null while objective is `expansion_manipulation`
- bridge gap is null or always zero
- gate status is missing
- selected win probability is null

In those cases, label the run as:
- `objective_surface_degraded`

## Baseline contamination warning

If many runs share one state dir, score deltas may reflect cumulative accepted updates rather than independent comparison.

Signs:
- accepted runs appear in a long streak
- later deltas plateau at the same value
- best score appears after several promotions in same state dir

Correct comparison method:
- use isolated state dirs for each parameter set

## Current baseline truth

For current NQ cleaned multi-timeframe data:
- `structure_ict` baseline defaults remain the strongest confirmed parameter set under isolated comparison.
- Local sweeps around lookback / expansion threshold did not beat baseline when run with isolated state.
- `cross_market_smt` has shown keep in autoresearch but direct paired-data sweep was flat or invalid for some pair quality cases.

## Recommended next scoring work

1. Implement `evidence-quality-breakdown`
   - base
   - support gap contribution
   - uncertainty penalty
   - conflict penalties
   - multi-timeframe bonus/penalty
   - liquidity penalty/bonus

2. Implement `research-verdict`
   - summarize whether results imply continue / pivot / stop / structural change

3. Add objective score decomposition to mutation outputs
   - contribution by component
   - dead/null component flags
   - parameter source annotation
