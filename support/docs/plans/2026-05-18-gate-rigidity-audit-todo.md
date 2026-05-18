# Gate Rigidity Audit — 2026-05-18

Findings from a 9-round factor-iteration gate audit. The optimization
goal is "factors don't get killed by combat thresholds before live
deployment can teach the engine which gates were over-strict." The
audit surfaced 87 vulnerabilities across 17 classes, plus 9 recurring
meta-patterns. Two slices have already landed (see Status); the rest
is open work.

## Status

- **c8a45f12** `fix(gate): relax two unreachable hard gates so iteration can promote`
  - `MECE_RECOVERY_ACCURACY_GATE: 0.95 -> 0.55`
  - `STRUCTURAL_PATH_RANKING_EXECUTION_GATE_MIN_PATH_PROB: 0.5 -> 0.30`
- **a4d98718** `fix(gate): break OU overlay chicken-and-egg gate`
  - `OuOverlayState::regime_influence_enabled` activation:
    `>= EXECUTION_GATE_READY (0.65)` -> `>= EXECUTION_GATE_OBSERVE (0.45)`

Remaining: 84 findings, grouped below with concrete PR-shape proposals.

## Why the engine appears to kill good factors

Each iteration layer has a safety-first default; multiplied across 12
layers, the cumulative pass rate trends to ~0%. The audit identified
multiple constants that were *unreachable* on real-world data (95% MECE
accuracy on 5-class regime, 0.5 path-prob lower bound, 0.75 reliability
ceiling for "approved" status) and several chicken-and-egg structures
where an overlay that exists to lift readiness was gated off until
readiness was already lifted by other means.

## Meta-patterns (recurring root causes)

1. **Hard double-sided clamps** (~30 occurrences) — `clamp(0.4, 1.6)`,
   `clamp(0.25, 0.75)`, `clamp(0.0, 0.30)`, etc. truncate tails of
   legitimate signals. Proposal: introduce a `SoftClamp` trait using
   sigmoid/tanh shaping rather than hard cutoffs.
2. **Chicken-and-egg overlays** — overlay activations gated by the
   same readiness value the overlay is supposed to lift. One landed
   (OU). Spectral was already correct. Audit any new overlay against
   this pattern before merging.
3. **Safety-defaults compound across 9+ layers** — single-layer pass
   probabilities multiply; any unreachable layer (MECE 0.95,
   path_prob 0.5, reliability 0.75) zeroes the whole product.
4. **Magic-number scatter** — `0.65` appears in 4+ unrelated
   contexts; `0.55` in 5+. Pulling them into named `const`s would
   make calibration changes auditable.
5. **Bootstrap penalty** — new factors default to `stability=0.0`,
   `regime_break_penalty=0.10`, `approved=false`, `comparable=false`;
   composite is artificially low before any evidence exists.
6. **Activation function caps** — tanh coefficients, sigmoid without
   temperature, softmax with default T=1.0 — every activation has an
   uncalibrated scale that quietly damps signals.
7. **Catch-22 deadlocks** — options factor cannot raise confidence
   without `auxiliary` data, but cannot acquire `auxiliary` data
   without entering live mode, which requires raised confidence.
8. **Pad-with-zeros backtest contamination** — `pad_indicator(..., 0.0)`
   fills the first `lookback` bars with fake data; 252-day percentile
   factors on 1d series get ~1 year of artifacts treated as real.
9. **Mutation surface vs true parameter space mismatch** —
   `parameter()` exposes period sizes but compute-internal weights
   (e.g., `ema_edge * 12.0 + rsi_edge * 0.6`) are hardcoded;
   mutation cannot explore the actual signal-shape space.

## Findings by class

Each row: ID, file:line(s), one-line description, proposed fix.

### A. Global hard thresholds
- #1 `gates.rs:3` `EXECUTION_GATE_READY=0.65` not timeframe-aware.
  Proposal: `READY_BY_TF` table (1m/5m: 0.58, 15m/30m: 0.62, 1h+: 0.65).
- #2 `gates.rs:3-4` gap between READY (0.65) and OBSERVE (0.45) is
  20pp with no middle band. Proposal: add
  `EXECUTION_GATE_QUASI_READY=0.55` for paper-trade band.

### B. Sample maturity
- #5 `belief_core/ranking_label.rs` `min_rows default = 30`. Proposal:
  per-timeframe default; 1h+ uses 15.

### C. Cost/liquidity
- #7 Cost stress hardcoded 0/1/2/5bps. Proposal: per
  `instrument_class` table (US large-cap ETF 0.5/1/2bps, options
  proxy 5/10bps).
- #8 Sharpe/PF ignore turnover and capacity. Proposal: soft
  `capacity_warning` factor applied to readiness when
  `avg_trade_size_quote_vol > 0.05`.

### D. Regime conditioning
- #6 CatBoost fallback `weighted_feature_sum_v1` always
  `candidate_set_only`. Proposal: regime-conditional promotion when
  regime mix is degenerate (single regime year).
- #9 Aggregate Sharpe is the gate; regime-conditional cohort
  Sharpes are computed but not enforced. Proposal: gate requires
  `min_regime_bucket_sharpe > 1.0`.
- #10 BBN posterior not fed back into readiness. Proposal: add
  `bbn_pred_class` term to `derive_*_execution_fields`.

### E. Provider/parity
- #11 Provider window blockers pollute mutation Q-table. Proposal:
  add `provider_window_blocked_retry` factor state; excluded from
  Q-table updates.
- #12 Synthetic OHLCV positives have no auto-followup to native
  provider lane. Proposal: incubate-followup TODO injection.

### F. Composite scoring weights (state/types.rs:4429)
- #13 `ic_score = |ic|/0.08` — IC 0.05 is excellent on real data and
  only earns 0.625. Proposal: divisor 0.05.
- #14 `sharpe_score = (sharpe+0.2)/1.7` saturates at sharpe 1.5;
  walk-forward V2.5d at 5.13 maps to same 1.0 as 1.5. Proposal:
  soft upper bound `1.0 + log1p((s-1.5)) * 0.1`.
- #15 `return_score` uses total return not annualized. Proposal:
  use annualized.
- #16 `sample_score = min(20, trades)/20` caps density at 20.
  Proposal: log-density continuation above 20.
- #17 `regime_score < 0.34` flagged as weakness; SPY 8Y has ~0.33
  natural width. Proposal: threshold 0.20 with cohort-Sharpe
  compensator.

### G. Weakness flags (state/types.rs:4452-4492)
- #18 Eleven binary weakness flags; `keep` requires <=2 weaknesses;
  borderline factors trip 3 small ones. Proposal: severity-weighted
  Map with `sum(severity) <= 1.2`.
- #19 `win_rate < 0.48` flags trend-follow factors that are
  legitimately low-win-rate / high-PF. Proposal: low-win-rate not a
  weakness when `pf >= 1.50`.
- #20 `profit_factor < 1.05` hard cutoff. Proposal: graded
  mild/weak/severe band.

### H. Iteration and promotion
- #3 `derive_research_execution_fields` weights `approved=0.30`
  (human signoff); research-phase factors never have approval.
  Proposal: approved=0.10, walk_forward_positive=0.15,
  cost_stress_survives=0.10.
- #4 `derive_backtest_execution_fields` caps `trade_count.min(20)`.
  Proposal: log-density continuation.
- #21 `factor_iteration_action` `replace` requires `composite < 0.45`
  but grade D spans 0.40-0.55. Proposal: align grade and action
  bands.
- #22 `build_agent_prompt` hardcodes promotion increments 0.05/0.10
  /0.15 in strings. Proposal: `FactorPromotionThresholds` config.

### I. Overlay second-order gates
- #23 OU overlay readiness gate. **LANDED in a4d98718.**
- #24 Spectral overlay already correct (gated at OBSERVE).

### J. PDA / uncertainty / MECE
- #25 PDA `actionable_top` requires `silhouette>=0.50 AND
  consistency>=0.90`. Proposal: relax consistency to 0.85; add
  `actionable_medium` band.
- #26 `uncertainty >= 0.45` is single hard cutoff. Proposal: three
  bands (<0.30 certain, 0.30-0.55 mixed, >0.55 uncertain).
- #27 `MECE_RECOVERY_ACCURACY_GATE`. **LANDED in c8a45f12.**
- #28 `classify_mece_recovery_combined_gate` short-circuits on any
  sub-gate. Proposal: weighted fusion with
  `MECE_RECOVERY_OBSERVE_ONLY=0.45` middle band.

### K. Learning rate asymmetry (weight_updater.rs)
- #29 `signed_delta` step 0.20 on positive credit, 0.25 on negative.
  Proposal: symmetric 0.20 unless `pf_signal < 1.0`.
- #30 `reliability_target.clamp(0.25, 0.75)` ceiling 0.75 cannot
  reach `keep`-grade reliability. Proposal: `clamp(0.10, 0.95)`.
- #31 Fixed `learning_rate = 0.25`. Proposal:
  `lr = 1.0/(10.0+obs_count)` adaptive.

### L. Regime multiplier double clamp
- #32 `multiplier.clamp(0.4, 1.6)` caps both extremes. Proposal:
  `clamp(0.05, 2.5)` with soft `1.0 + log1p(hit_rate*10)` upper
  bound.

### M. RegimeV2 8-state default
- #33 HMM defaults to 3 states; `RegimeV2` 8-state exists in
  parallel but is opt-in. Proposal: promote `RegimeV2` to default;
  legacy as shim.

### N. Killzone UTC hardcode
- #34 `factor_definition.rs:2386` US RTH 14-15, 19-20 UTC; London
  7-8, 12-13 UTC — DST not handled. Proposal: `chrono_tz`-derived
  per-bar timezone.

### O. Walk-forward window defaults
- #35 `train_bars=60, test_bars=30` for all timeframes; on 1h that
  is 3.75 days train, on 5m that is 7.5 hours. Proposal: per-tf
  defaults (1d=252, 1h=2000, 15m=4000, 5m=8000); test = train/2.
- #36 `test_bars.clamp(5, ...)` allows 5-bar test windows. Proposal:
  minimum 30 or skip fold.

### P. Mutation local-minimum
- #37 `accepted: score_delta > 0.0` admits noise. Proposal:
  `score_delta > 0.01 OR (score_delta > 0.0 AND wf_stable)`.
- #38 `compare_hint_effectiveness` sorts by acceptance_rate first;
  small consistent wins beat occasional large wins. Proposal:
  primary key = `acceptance_rate * average_score_delta` (expected
  value).

### Q. Long-only bias in expansion scoring
- #39 `balanced_accuracy` only used when both bull AND bear samples
  exist; falls back to `directional_accuracy` for long-only.
  Proposal: long-only factors use `hit_rate_when_active`.

### R. Lifecycle single-point inheritance
- #40 `lifecycle.accepted = mutation.accepted`. Proposal: lifecycle
  requires `mutation.accepted AND wf.stable AND cost_stress.passed`.
- #41 `expansion_objective` mentions Pre-Bayes gate in prompt but
  not in score_delta. Proposal: add `pre_bayes_gate_acceptance_delta`
  to score.

### S. PDA cooccurrence/precedence window
- #42 Single `window_bars` for all event pairs. Proposal:
  per-event-pair window table or ATR-scaled adaptive.

### T. Hardcoded stoploss
- #43 `stop_loss_pct: 0.006` (0.6%) for all timeframes. Proposal:
  per-tf table (5m=0.5%, 1h=1.2%, 4h=2.0%, 1d=3.5%) or 1-sigma ATR.

### U. Oracle probe double-lock
- #44 `retrospective_only=true` AND `RequiresLiveNowcastBranch` both
  default. Proposal: expose `--probe-promotion-mode` CLI flag.
- #45 Tiny-leg `cluster_count=16/6` hardcoded. Proposal:
  `ceil(sqrt(leg_count)/2)` clamped to [4, 24].

### V. BBN node schema
- #47 4-state discretization on `entry_zone_quality`. Proposal:
  declare `discretization_strategy` per node; ordinal_8 for
  evidence-rich nodes.
- #48 `parent_candidates` static list. Proposal: runtime
  registration via `BbnEvidenceRegistry`.

### W. Conformal coverage threshold
- #49 `(cc - 0.45)/0.35` denominator + weakness `cc < 0.55`.
  Proposal: divisor 0.20, weakness threshold 0.40.

### X. Autoresearch staleness
- #50 `staleness_threshold = 10 min` for all timeframes. Proposal:
  `10 min * max(1, base_bar_minutes/5)`.

### Y. TimesFM concurrency
- #51 Shared `timesfm_input.json` temp file races. Proposal:
  per-process unique tempfile.

### Z. Mutation rejection history
- #52 Description-only; no negative-evidence memory. Proposal:
  `mutation_rejection_history` table; prior-acceptance lookup.

### AA. Killzone double source
- #53 BBN `execution_window` states vs `factor_definition` UTC
  windows are independent hardcodes. Proposal: shared
  `SessionContextProvider` trait.

### AB. Mutation match arms
- #54 `pre_bayes_gate_observe_only` replicated 23x; all map to
  `[rsi_period, adx_period]` regardless of factor family. Proposal:
  `mutation_params_by_family` Map.
- #55 Replicated match arms risk silent omission on new
  `FactorCategory`. Proposal: derive macro or exhaustive check.

### AC. Magic number scatter
- #56 `0.65` in 4+ unrelated contexts. Proposal: semantic constants
  (`EXECUTION_READY`, `TEXTURE_HEALTHY`, etc.) all referencing one
  source.
- #57 Confidence and reliability usable bands are narrow
  ([0.35,0.65] and [0.45,0.75]). Proposal: ordinal_4
  discretization.

### AD. Lévy VaR not wired
- #58 `levy_var.py` documents "hard gate" intent but no consumer.
  Proposal: `tail_risk_score = 1 - cvar_99/max_dd_tolerance` added
  to `derive_backtest_execution_fields`.

### AE. Risk tolerance double source
- #59 Texture uses `0.35 * atr_last`; stoploss uses fixed 0.006.
  Proposal: shared `RiskTolerance` struct.

### AF. Provider retry/timeout
- #60 yfinance 30s timeout doesn't scale with payload size.
  Proposal: `15s + 5s*years_requested`.
- #61 Fixed 700ms sleep between yfinance retries. Proposal:
  exponential backoff + Retry-After header.
- #67 Four providers share identical 30s timeout. Proposal: per
  provider timeout table.
- #68 yfinance MAX_ATTEMPTS=3, total wait ~2.1s vs Yahoo 429
  recovery time 30-90s. Proposal: MAX_ATTEMPTS=5 with backoff.
- #69 Hardcoded `50ms/60ms` micro-sleeps. Proposal: configurable.
- #77 Binance/Bybit 1000-bar cap without pagination. Proposal:
  cursor-based pagination.
- #78 TradingView MCP has no retry layer. Proposal: wrap with 429
  exponential backoff (5s/15s/45s).

### AG. HMM training
- #62 Comment-acknowledged "low iterations" in PDA HMM clustering.
  Proposal: `max_iter` default 50->200; expose CLI; record
  `convergence_achieved`.

### AH. Node transition recursion
- #63 `NODE_TRANSITION_RECURSIVE_MAX_DEPTH` and `STEP_DISCOUNT`
  hardcoded. Proposal: CLI override; regime-conditional defaults.

### AI. Bootstrap penalty
- #64 New factor's `regime_break_penalty=0.10` initial. Proposal:
  start at 0.0; update only after first observed break.

### AJ. CatBoost stub feedback pollution
- #65 Stub mode `confidence=0.500 weight=0.55`. Proposal: stub
  mode forces `weight=0.0` and `learning_enabled=false`.

### AK. regime_break normalization
- #66 `break_penalty_score = 1 - penalty/0.35`. Proposal: divisor
  0.50; weakness threshold 0.40; "capacity to recover" override.

### AL. Activation function caps
- #71 `softmax_temperature = 1.0` default produces near-uniform
  attention. Proposal: default 0.5.
- #72 `tanh(avg_pnl) * 0.25` coefficient damps PnL signal vs
  hit_rate. Proposal: coefficient 1.0 or switch to expectancy.
- #73 Sigmoid without temperature calibration on CatBoost output.
  Proposal: Platt or isotonic scaling.

### AM. Family compute hardcodes
- #74 `evaluate_trend_momentum` `ema*12.0 + rsi*0.6`, confidence
  `0.65/0.35`. Proposal: all magic numbers as `parameter()`.
- #75 `bollinger_std=2.0` default. Proposal: per vol regime.
- #76 `evaluate_spectral_rhythm` `(1.0 + variance.ln()/10.0)`
  saturates 0.1-0.5 for typical assets. Proposal: divisor 5.0 or
  switch to FFT bin entropy.
- #81 ICT internal hardcoded swing/CISD/pool constants (3, 1, 3, 2)
  not parameterized. Proposal: per-tf swing pivot table.
- #82 Options confidence accumulation `0.35+0.20+0.20+0.15`
  hardcoded. Proposal: `evidence_confidence_weight` Map.
- #84 Crowding `volume_spike_ratio=2.0` doesn't normalize by recent
  volatility. Proposal: `base_ratio * (1 + recent_vol_zscore*0.5)`.
- #87 `find_swing_highs(window, 3)` fixed for all tf. Proposal:
  `swing_pivot_bars_by_tf` table.

### AN. Catch-22 options factor
- #83 `evaluate_options_hedging` fallback confidence cap 0.30 below
  0.35 discard threshold. Proposal: fallback uses OHLCV-derived IV
  proxy floor 0.40; or "diagnostic mode" entry.

### AO. Pad-zero contamination
- #85 `pad_indicator(values, target_len, fill: 0.0)` for EMA/ATR.
  Proposal: skip first `lookback` bars in backtest evaluation; mark
  artifact with `effective_history_bars`.

### AP. Normalize_signed tail clamp
- #86 `normalize_signed(value, cap=1.0)` clamps extreme signals.
  Proposal: `value.tanh()` soft compression.

### AQ. Test suite freezes strict gates
- #79 `tests/.../structural_playbook.rs:6603` asserts
  `path_prob_lower_bound < 0.5`. Proposal: parameterize against
  current gate config.

### AR. Provider priority registry missing
- #80 Aggregator fallback order is implicit. Proposal:
  `ProviderPriorityRegistry` keyed by `(instrument_class, tf)`.

## Suggested next slice (after current 2 commits)

Priority order for follow-up PRs:

1. **Bootstrap penalty zero-out** (#64) — single-line in
   `state/persistence.rs:910` once that file is no longer dirty.
2. **Reliability ceiling lift** (#30) — `clamp(0.25, 0.75)` to
   `clamp(0.10, 0.95)` in `factors/weight_updater.rs`.
3. **Learning rate symmetry** (#29) — match positive/negative
   step in `factors/weight_updater.rs`.
4. **EXECUTION_GATE_QUASI_READY band** (#2) — adds the missing
   middle band in `domain/execution/gates.rs`.
5. **Composite score recalibration** (#13-#17) — bundle in
   `state/types.rs:4429` once dirty resolves.
6. **Walk-forward window per-tf** (#35-#36) — `factor_lab/backtest.rs`.
7. **Family-compute hardcode extraction** (#74, #81, #82) — biggest
   mutation-surface win, but touches `factor_lab/factor_definition.rs`
   in many places.

## Out of scope for this audit

- Real-time deployment of the relaxed gates (requires paper-trade
  validation that no edge case bypassed by relaxed gates produces
  actual losses).
- Calibration of all newly recommended thresholds (e.g., 0.55 for
  MECE) against fresh OOS data — current values are placeholders
  in the *reachable* band.

## How to extend this doc

Each finding above ends with a "Proposal:" line. New findings
appended below should follow the same shape: ID, file:line,
description, proposal. Keep one finding per row to make grep
auditable.
