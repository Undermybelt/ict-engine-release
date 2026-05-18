# Heuristic Learning Module Harvest Report

> Based on `support/docs/plans/2026-05-09-heuristic-learning-self-iteration-plan.md`.
> Goal: find existing papers/repos/formulas/modules that can be拆下來拼湊 into `ict-engine`.

## Executive Verdict

可直接拼。优先不是引入一个大框架，而是拆 8 个小模块：

1. Triple Barrier + Meta-labeling: 生成交易 outcome / path-ranker target
2. Purged CV + Embargo + DSR/PBO: 防止高夏普幻觉
3. Qlib Alpha158 / Alpha101: 公式种子库与 factor DSL
4. HMM + Markov Switching + ruptures/BOCD/river: regime posterior + transition evidence
5. MAPIE conformal + block bootstrap: 让“95% regime confidence”有校准含义
6. pgmpy/pyAgrum BBN search: 只在 entropy/log-loss 有增益时加 evidence edge
7. CatBoost/LightGBM ranking: execution path ranking
8. empyrical/pyfolio/Riskfolio/backtrader analyzers: payoff-shape + portfolio diversity report

Repo 已有基础：`support/paper2code/rammstein`、`support/paper2code/crowded_trades`、`support/paper2code/kyle_stochastic_liquidity`、`support/paper2code/red_queens_trap`。这些应先接入报告/门控层，不必重写。

---

## P0: 立刻可拆模块

### 1. Triple Barrier Labeling + Meta-labeling

Sources:
- mlfinlab: https://github.com/hudson-and-thames/mlfinlab
- López de Prado, Advances in Financial Machine Learning
- Meta-labeling reference: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3257419

Core formulas:

```text
upper_barrier = entry_price * (1 + pt * target_vol)
lower_barrier = entry_price * (1 - sl * target_vol)
vertical_barrier = t0 + max_holding_bars
label = first_hit(upper, lower, vertical) -> {+1, -1, 0}
meta_label = 1 if primary_side * net_return > 0 else 0
```

Direct ict-engine target:

```text
support/scripts/research/labeling_triple_barrier.py
support/scripts/research/factor_payoff_shape_report.py
support/scripts/auto_quant_external/pandas_path_ranker_trainer.py
/tmp/ict-hl/<symbol>/<session>/path_ranking/target.csv
```

Use:
- ICT setup gives `side` and `entry candidate`.
- Triple barrier gives realized path label: `hit_tp`, `hit_sl`, `timeout`.
- Meta-label decides whether setup should be taken.
- Path ranker target becomes `realized_R`, `mfe`, `mae`, `time_to_hit`, `meta_label`.

Risk:
- severe lookahead if barrier labels leak into features.
- overlapping labels require purged CV + embargo.

---

### 2. Purged K-Fold, Embargo, DSR, PBO

Sources:
- mlfinlab: https://github.com/hudson-and-thames/mlfinlab
- Deflated Sharpe Ratio: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2460551
- Probability of Backtest Overfitting: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253
- vectorbt DSR implementation: https://github.com/polakowo/vectorbt

Core formulas:

```text
PSR(SR*) = Phi(((SR - SR*) * sqrt(T - 1)) / sqrt(1 - skew*SR + ((kurt-1)/4)*SR^2))
DSR = PSR(SR*_multiple_trials)
PBO = fraction(CSCV splits where selected IS winner ranks below median OOS)
```

Direct ict-engine target:

```text
support/scripts/research/factor_payoff_shape_report.py
support/scripts/research/heuristic_chain_verdict.py
src/application/factor_lifecycle/expansion_scoring.rs
src/application/factor_lifecycle/expansion_evaluation.rs
```

Add fields to every candidate summary:

```json
{
  "raw_sharpe": 0.0,
  "oos_sharpe": 0.0,
  "psr": 0.0,
  "dsr": 0.0,
  "pbo": 0.0,
  "effective_trials": 0,
  "effective_sample_size": 0,
  "skew": 0.0,
  "kurtosis": 0.0,
  "promotion_gate": "reject|probe|promote"
}
```

Gate:

```text
promote only if oos_sharpe_lcb > 0 and dsr >= 0.8 and pbo <= 0.1
```

Risk:
- DSR requires真实 trial count；必须记录所有失败尝试。
- trade samples 非独立，T 要改成 effective sample size。

---

### 3. Qlib Alpha158 / Alpha360 + WorldQuant Alpha101

Sources:
- Qlib: https://github.com/microsoft/qlib
- Qlib Alpha loader: https://github.com/microsoft/qlib/blob/main/qlib/contrib/data/loader.py
- Alpha101 implementation: https://github.com/yli188/WorldQuant_alpha101_code

Extractable formulas:

Qlib Alpha158 K-line:

```text
KMID  = (close - open) / open
KLEN  = (high - low) / open
KMID2 = (close - open) / (high - low + eps)
KUP   = (high - max(open, close)) / open
KLOW  = (min(open, close) - low) / open
KSFT  = (2*close - high - low) / open
```

Rolling features:

```text
ROC_d  = Ref(close,d) / close
MA_d   = Mean(close,d) / close
STD_d  = Std(close,d) / close
BETA_d = Slope(close,d) / close
RSQR_d = Rsquare(close,d)
RSV_d  = (close - Min(low,d)) / (Max(high,d) - Min(low,d) + eps)
CORR_d = Corr(close, Log(volume+1), d)
```

WorldQuant operator set:

```text
ts_sum, sma, stddev, correlation, covariance, rolling_rank, ts_rank,
ts_min, ts_max, delta, delay, rank, scale, ts_argmax, ts_argmin, decay_linear
```

Direct ict-engine target:

```text
support/docs/factor-catalog.md
support/scripts/research/factor_formula_library.py
support/scripts/research/factor_candidate_pack.py
src/factor_lab/factor_definition.rs  (later only for promoted stable formulas)
```

Use:
- Treat Qlib/Alpha101 as formula seed pool.
- Convert to `factor_expression.json` DSL.
- Let HL mutation operate on operators/windows/normalization.

Risk:
- Qlib偏日频截面；NQ intraday 需转成 time-series bucket IC。
- Alpha zoo 会爆炸；必须配 compression retrospective。

---

### 4. Payoff-shape report metrics

Sources:
- empyrical: https://github.com/quantopian/empyrical
- pyfolio: https://github.com/quantopian/pyfolio
- backtrader analyzers: https://github.com/mementum/backtrader
- Riskfolio-Lib: https://github.com/dcajasn/Riskfolio-Lib

Extractable metrics:

```text
annual_return, cagr, annual_volatility, sharpe, sortino, calmar,
omega, max_drawdown, time_under_water, tail_ratio, VaR, CVaR,
hit_rate, avg_win, avg_loss, win_loss_ratio, payoff_ratio,
longest_win_streak, longest_loss_streak, exposure, turnover,
skew, kurtosis, HHI concentration, factor_return_corr,
incremental_portfolio_sharpe, incremental_CVaR
```

Direct ict-engine target:

```text
support/scripts/research/factor_payoff_shape_report.py
support/scripts/research/policy_truth_reports.py
support/scripts/auto_quant_external/portfolio_diversity_scorecard.py
```

Use:
- Turn “high Sharpe” into explainable source:
  - trend convexity
  - reversion snapback
  - carry/VRP
  - liquidity/session edge
  - cross-market lag/confirmation

Risk:
- Libraries have mixed maintenance/licenses. Best: reimplement formulas, do not copy code blindly.

---

## P0 Regime Confidence Stack

### 5. ruptures for offline changepoint labels

Source:
- ruptures: https://github.com/deepcharles/ruptures
- JOSS paper: https://joss.theoj.org/papers/10.21105/joss.02057

Algorithms:

```text
PELT: minimize sum(C(y_tau_i:tau_i+1)) + beta * num_changes
BinSeg: recursive binary segmentation
Window: local discrepancy window scan
Costs: l2, rbf, linear
```

Direct ict-engine target:

```text
support/scripts/research/regime_artifact_bundle.py
support/scripts/research/regime_confidence_report.py
/tmp/ict-hl/<session>/regime/regime_truth_sources.json
```

Use:
- independent ex-post regime truth source.
- not live trading signal.
- compare HMM/Markov/HF classifiers against transition windows.

Risk:
- ex-post lookahead. Use only for benchmark / labels, not realtime branch decision.

---

### 6. HMM + Markov Switching for regime posterior

Sources:
- hmmlearn: https://github.com/hmmlearn/hmmlearn
- statsmodels Markov switching: https://github.com/statsmodels/statsmodels

Core formulas:

```text
HMM forward posterior:
alpha_t(j) proportional to p(x_t | z_t=j) * sum_i alpha_{t-1}(i) A_ij
confidence_t = max_j P(z_t=j | x_1:t)
expected_duration_j = 1 / (1 - A_jj)
```

Direct ict-engine target:

```text
support/scripts/research/hmm_numeric_trainer.py        (exists)
support/scripts/research/regime_confidence_report.py   (new)
src/factors/regime_conditional.rs
src/application/regime/
```

Use:
- HMM for multivariate feature posterior.
- MarkovRegression/Autoregression for interpretable return/vol regimes.
- Map to BBN evidence:
  - `market_regime`
  - `factor_uncertainty = entropy(posterior)`
  - `transition_hazard`
  - `persistence_score`

Risk:
- HMM label switching; fix semantic mapping.
- Viterbi smoothing uses future data; realtime must use forward filter.

---

### 7. BOCD + river drift for online transition evidence

Sources:
- BOCD reference repo: https://github.com/gwgundersen/bocd
- Bayesian changepoint repo: https://github.com/hildensia/bayesian_changepoint_detection
- river: https://github.com/online-ml/river

Core formulas:

```text
BOCD run-length posterior: P(r_t | x_1:t)
changepoint_prob = P(r_t = 0 | x_1:t)
ADWIN/KSWIN/PageHinkley drift flags from river
```

Direct ict-engine target:

```text
support/scripts/research/regime_confidence_report.py
support/scripts/research/transition_evidence_aggregator.py
src/application/orchestration/execution_tree.rs
```

Use:
- `bocd_p_changepoint > 0.95` -> transition guardrail.
- river drift on features/residuals -> calibration degraded.
- feed `transition_alert_95` into execution tree.

Risk:
- BOCD hazard parameter sensitive.
- Drift does not say direction; only says regime evidence degraded.

---

### 8. MAPIE conformal + block bootstrap for 95% regime confidence

Sources:
- MAPIE: https://github.com/scikit-learn-contrib/MAPIE
- Conformalized Quantile Regression: https://arxiv.org/abs/1905.03222
- Adaptive conformal under distribution shift: https://arxiv.org/abs/2106.00170

Core formulas:

```text
split conformal score_i = 1 - p_model(y_i | x_i)
q = quantile(score_cal, ceil((n+1)(1-alpha))/n)
prediction_set = {y: 1 - p_model(y|x) <= q}
coverage >= 1 - alpha
ECE = sum_b |B_b|/n * |acc(B_b) - conf(B_b)|
```

Direct ict-engine target:

```text
support/scripts/research/regime_confidence_report.py
/tmp/ict-hl/<session>/regime/calibration_report.json
src/application/orchestration/execution_tree.rs
```

Define 95% confidence as:

```text
confidence_95 =
  conformal_set_size == 1
  and rolling_coverage >= 0.93
  and calibration_ece <= 0.05
  and bootstrap_ci_width <= max_width
  and transition_prob < threshold
  and flip_rate <= max_flip_rate
  and calibration_health == valid
```

Risk:
- conformal assumes exchangeability; use rolling/adaptive calibration.
- 95% coverage may produce too many ambiguous regimes; log ambiguity_rate.

---

## P0 BBN + Path Ranking

### 9. pgmpy / pyAgrum BBN structure + CPT learning

Sources:
- pgmpy: https://github.com/pgmpy/pgmpy
- pyAgrum: https://github.com/agrumery/aGrUM

Core formulas:

```text
P(X_1,...,X_n) = product_i P(X_i | Parents(X_i))
DeltaH = H(trade_outcome) - H(trade_outcome | evidence)
DeltaLL = logloss_baseline - logloss_with_evidence
```

Direct ict-engine target:

```text
support/scripts/research/bbn_structure_search.py       (exists)
src/bbn/evidence.rs
src/bbn/learning/structure_learner.rs
src/bbn/learning/cpt_updater.rs
/tmp/ict-hl/<session>/bbn/evidence_delta.json
```

Allowed node vocabulary:

```text
market_regime
liquidity_context
factor_alignment
factor_uncertainty
multi_timeframe_resonance
crowding_pressure
dealer_pressure
session_quality
entry_quality
trade_outcome
```

Gate:

```text
add evidence only if posterior_entropy_reduction > threshold
or OOS log-loss improves
or contradiction detection lift improves
```

Risk:
- structure learning finds correlation, not causality.
- CPT sparse; use Dirichlet smoothing.
- node explosion kills maintainability.

---

### 10. CatBoost / LightGBM learning-to-rank for execution paths

Sources:
- CatBoost: https://github.com/catboost/catboost
- LightGBM: https://github.com/microsoft/LightGBM

Core target:

```text
path_utility = realized_R
  - lambda1 * max_adverse_excursion
  - lambda2 * slippage
  - lambda3 * time_in_trade
  + lambda4 * regime_persistence
```

Ranking items:

```text
fill_now
wait_for_retest
skip
reduce_size
wider_stop
tighter_stop
```

Direct ict-engine target:

```text
support/scripts/auto_quant_external/pandas_path_ranker_trainer.py  (exists)
support/scripts/auto_quant_external/path_ranker_integration.py     (exists)
src/application/orchestration/execution_tree.rs
execution_tree_trace.json
```

Gate:

```text
mature_rows >= 30
raw_scored_mature >= 30
runtime_source = registered_model_artifact
execution_tree_trace includes ranker contribution
workflow recommendation changes on replayable setup
```

Risk:
- hindsight execution path leakage.
- raw PnL target biases to high vol; use risk-adjusted utility.

---

## P1 Feature / Diagnostics Sidecars

### 11. tsfresh / Kats / sktime feature extraction

Sources:
- tsfresh: https://github.com/blue-yonder/tsfresh
- Kats: https://github.com/facebookresearch/Kats
- sktime: https://github.com/sktime/sktime

Extractable features:

```text
autocorrelation, partial_autocorrelation, ADF, abs_energy, cid_ce,
mean_abs_change, linear_trend, entropy, sample_entropy,
approximate_entropy, permutation_entropy, fft_aggregated,
hurst, lumpiness, stability, level_shift_size, crossing_points,
trend_strength, seasonality_strength, spikiness
```

Direct ict-engine target:

```text
support/scripts/auto_quant_external/vol_regime_v2_features.py       (exists)
support/scripts/auto_quant_external/vol_regime_cross_asset_features.py (exists)
support/scripts/research/regime_artifact_bundle.py
support/scripts/research/factor_payoff_shape_report.py
```

Use:
- feature generator for regime classifier / path ranker.
- payoff stability diagnostics.

Risk:
- huge feature zoo; must use stability selection and DSR/PBO.

---

## Already in repo: should wire first

### support/paper2code/rammstein

Path:
```text
support/paper2code/rammstein/
```

Useful modules:
- OU MLE estimator: theta, mu, sigma
- 8-dim execution state vector
- regime-aware laziness score

Integration:
```text
support/scripts/research/regime_confidence_report.py
support/scripts/research/factor_payoff_shape_report.py
src/application/orchestration/execution_tree.rs
```

Use:
- OU theta -> reversion feasibility / wait-vs-fill.
- overextension -> BBN evidence.

### support/paper2code/crowded_trades

Path:
```text
support/paper2code/crowded_trades/
```

Useful modules:
- Jaccard overlap of trading patterns
- mean pairwise overlap
- Ising herding bias verification

Integration:
```text
src/factor_lab/factor_definition.rs::CrowdingHerding
src/application/orchestration/execution_tree.rs::block_crowded
src/bbn/evidence.rs::crowding_pressure
```

Use:
- factor agreement + Ising agreement = double crowding block.

### support/paper2code/kyle_stochastic_liquidity

Path:
```text
support/paper2code/kyle_stochastic_liquidity/
```

Useful modules:
- Kyle lambda: lambda = Cov(delta_price, order_flow) / Var(order_flow)
- market depth = 1 / lambda
- execution cost = lambda * order_size

Integration:
```text
src/application/orchestration/execution_tree.rs
support/scripts/research/factor_payoff_shape_report.py
```

Use:
- stop/fill/slippage realism.
- wait penalty if depth deteriorating.

### support/paper2code/red_queens_trap

Path:
```text
support/paper2code/red_queens_trap/
```

Useful modules:
- friction barrier
- survivor bias detector
- mode collapse monitor
- capital decay tracker

Integration:
```text
support/scripts/research/heuristic_retrospective.py
support/scripts/research/factor_payoff_shape_report.py
src/application/factor_lifecycle/expansion_evaluation.rs
```

Use:
- reject gross-positive/net-negative factors.
- detect all factors collapsing into same beta.

---

## Proposed New Files

```text
support/scripts/research/labeling_triple_barrier.py
support/scripts/research/factor_formula_library.py
support/scripts/research/factor_payoff_shape_report.py
support/scripts/research/regime_confidence_report.py
support/scripts/research/transition_evidence_aggregator.py
support/scripts/research/bbn_evidence_value_report.py
support/scripts/research/portfolio_diversity_scorecard.py
support/scripts/research/heuristic_chain_verdict.py
support/scripts/research/heuristic_retrospective.py
```

Optional later:

```text
support/scripts/research/adapters/qlib_alpha158.py
support/scripts/research/adapters/worldquant_alpha101.py
support/scripts/research/adapters/ruptures_changepoint.py
support/scripts/research/adapters/mapie_conformal.py
support/scripts/research/adapters/hmmlearn_regime.py
support/scripts/research/adapters/statsmodels_markov_switching.py
support/scripts/research/adapters/river_drift.py
```

---

## Recommended Build Order

### Slice 1: Labels + payoff truth

Implement:
```text
labeling_triple_barrier.py
factor_payoff_shape_report.py
```

Output:
```text
barrier_hit
realized_R
mfe
mae
time_to_hit
meta_label
psr/dsr/pbo
payoff_shape
```

### Slice 2: Regime confidence

Implement:
```text
regime_confidence_report.py
transition_evidence_aggregator.py
```

Use:
```text
hmmlearn/statsmodels posterior
ruptures benchmark labels
MAPIE conformal set
block bootstrap CI
river/BOCD transition alert
```

Output:
```text
confidence_95
conformal_set
calibration_ece
bootstrap_lcb
transition_prob
flip_rate
mean_segment_bars
```

### Slice 3: BBN evidence gate

Implement:
```text
bbn_evidence_value_report.py
```

Output:
```text
posterior_entropy_delta
logloss_delta
contradiction_lift
accepted_edges
rejected_edges
```

### Slice 4: Path ranking enhancement

Extend:
```text
support/scripts/auto_quant_external/pandas_path_ranker_trainer.py
```

Add:
```text
risk_adjusted_path_utility
ranking groups
calibration report
maturity report
trace contribution export
```

### Slice 5: Formula seed library

Implement:
```text
factor_formula_library.py
```

Sources:
```text
Qlib Alpha158
WorldQuant Alpha101 operators
TA-style indicators
existing paper2code modules
```

---

## Rejection / License Notes

Do not copy code blindly from:
- backtesting.py: AGPL-3.0
- backtrader: GPL-3.0
- archived Quantopian stack if license/maintenance unclear

Safe pattern:
- copy formulas only if license permits, or reimplement from papers/math.
- use repos as interface/metric references.
- keep external heavy deps in `support/scripts/research/` sidecar, not Rust runtime.

---

## Final Recommendation

The best immediate拼装路线:

```text
Triple Barrier labels
+ DSR/PBO high-Sharpe guard
+ Qlib/Alpha101 formula seeds
+ HMM/Markov/ruptures regime labels
+ MAPIE conformal 95% confidence
+ BBN entropy/log-loss evidence gate
+ CatBoost risk-adjusted path ranking
+ paper2code crowding/kyle/rammstein/red-queen modules
```

This converts the earlier HL plan from concept into reusable module inventory.
