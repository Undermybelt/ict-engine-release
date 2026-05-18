# High-Sharpe Factor Harvest and Infinite Iteration Contract

Goal: harvest papers and open-source repos for regime-conditioned win-rate edges, then turn them into a zero-config, hot-plug, consumer-safe `ict-engine` learning loop.

Direction correction, 2026-05-10:

```text
Do not optimize for aggregate Sharpe as the primary target.
Primary target = regime is correct first, then strategy-regime fit, then conditioned win-rate/payoff/tail quality.
High win rate is not enough when regime is unknown, transitioning, or mismatched.
Aggregate Sharpe stays as a diagnostic field only.
```

Scope: all liquid markets are allowed: index futures, equities, ETFs, FX, rates, commodities, crypto, volatility, options, dealer/flow proxies.

Non-goal: do not import large frameworks into runtime. Reimplement small formulas as sidecar artifacts, validate out-of-sample, then promote only through gates.

---

## Operating rule

Every candidate starts as a sidecar experiment:

```text
paper/repo idea
-> formula spec
-> sidecar JSON/CSV artifact in /tmp/ict-hl/...
-> payoff-shape report
-> purged OOS/DSR/PBO gate
-> regime slice check
-> BBN evidence value check
-> path-ranker target contribution
-> execution-tree recommendation delta
-> promote/probe/reject
-> compression retrospective
-> next mutation batch
```

Do not call anything "promotable" until the regime gate, OOS gate, overfit guards, BBN value gate, path-ranker evidence, and execution-tree recommendation delta are all visible.

Runtime closure is mandatory after any promising candidate:

```text
Auto-Quant real run
-> filter/analyze market_state
-> auto-quant-results-import
-> auto-quant-prior-init with non-empty strategies_applied
-> export-structural-path-ranking-target
-> CatBoost or explicitly named fallback ranker
-> apply-structural-path-ranking-external-scores
-> register/enable ranker runtime
-> workflow-status / execution-tree evidence
```

Latest real closure evidence:

```text
run_root=/tmp/ict-high-sharpe-real-20260509-234554
Auto-Quant strategies=MomentumMTFConfluence,RegimeAdaptiveBNB
BBN strategies_applied=2
BBN prior_final=[0.6734197006771924,0.000000013279761567917304,0.326580286043046]
CatBoost train=uv run --with catboost ... pandas_path_ranker_trainer.py
CatBoost model=/tmp/ict-high-sharpe-real-20260509-234554/path_ranker_catboost/catboost_model.cbm
CatBoost scores=/tmp/ict-high-sharpe-real-20260509-234554/path_scores_catboost_after_analyze.csv
workflow_ranker=using_candidate_set_scores source=candidate_set applied=1 raw=0.751
analyze_ranker=candidate_set/catboost/not_ready
structural_path=trend_follow_through posterior=1.000 selected_prob=1.000
execution=observe/transition_guardrail/guarded
validation_boundary=mature_rows=0/30, production_validation=0/30, calibration=not_fitted
```

Rule: if CatBoost is unavailable, name the fallback (`weighted_feature_sum_v1`) and do not call it CatBoost. If CatBoost is available but rows are immature, call it a real CatBoost smoke only, not production validation.

Latest regime-first real closure evidence, 2026-05-10:

```text
real_chain_run_root=/tmp/ict-real-regime-selector-20260510T020632Z
provider_matrix_run_root=/tmp/ict-r36-full-market-selector-20260510T023200Z
Auto-Quant real run=logs/09_auto_quant_run.log
Auto-Quant strategies=MomentumMTFConfluence,RegimeAdaptiveBNB
filter/regime bundle=/tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state/regime_quality_matrix.json
BBN import=logs/10_auto_quant_results_import.log
BBN prior init=logs/11_auto_quant_prior_init.log, strategies_applied=2
CatBoost train=logs/16_catboost_train.log, samples=1367
CatBoost apply=logs/17_catboost_apply_current.log
execution_tree_trace=/tmp/ict-real-regime-selector-20260510T020632Z/structural-replay-36/state/NQ/execution_tree_trace.json
path_ranker_model_family=catboost
execution=observe/transition_guardrail/guarded
```

Provider evidence:

```text
yfinance=QQQ 1h/1d and AAPL 1h/1d fetched
IBKR=QQQ 1h 30D and AAPL 1d 60D fetched
TradingViewRemix=QQQ 1h/1d fetched after local credential recovery
Kraken=PF_XBTUSD 1h/1d fetched
provider_matrix_rows=10
providers=ibkr,kraken_public,tradingview_mcp,yfinance
timeframes=1d,1h
symbols=AAPL,BTCUSD,NQ
```

Regime/selector decision:

```text
regime_quality_matrix=/tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state/regime_quality_matrix.json
strategy_regime_matrix=/tmp/ict-r36-full-market-selector-20260510T023200Z/repo-state/strategy_regime_matrix.json
current_regime=unknown_abstain
current_regime_trade_usable=false
selected_strategy=none
recommendation=observe_no_trade
reason=all current provider/timeframe regime bundles are unknown_abstain / trade_usable=false
```

Strategy-regime metrics currently say:

```text
MomentumMTFConfluence: trades=854, win_rate=34.7775, win_rate_lcb=31.6578, pf=1.1682, max_dd=-23.1801, sharpe_diagnostic_only=0.3993, disabled_regime=unknown_abstain
RegimeAdaptiveBNB: trades=115 full_5y, win_rate=69.5652, win_rate_lcb=60.6358, pf=1.4262, max_dd=-4.6742, sharpe_diagnostic_only=0.1380, disabled_regime=unknown_abstain
Decision: RegimeAdaptiveBNB is the better win-rate candidate, but it is not enabled until the current regime classifier becomes reliable.
```

TradingViewRemix consumer credential fix:

```text
root_cause=ict-engine used process env only and ignored the existing local credential owner
canonical_local_fallback=~/.ict-engine/tvremix_mcp.json
code_owner=src/application/data_sources/control_matrix_providers.rs and provider_fetch.rs
env_order=ICT_ENGINE_TVREMIX_MCP_API_KEY / ICT_ENGINE_TVREMIX_MCP_URL first, then local file, then default URL
secret_policy=never print or write the key into repo docs
optional_options_probe_failure=degrade options lanes, do not hide usable OHLCV access
final_no_env_run_root=/tmp/ict-tvremix-local-config-final-20260510T034555Z
no_env_provider_status=market_data:1/1 ready, reason=mcp_url_and_api_key_available
no_env_fetch=ok=true, rows=21
```

---

## Promotion gates

### Factor gate

```text
trade_count >= 80 on one liquid lane, or probe-only if 30-79
OOS Sharpe LCB > 0
DSR >= 0.80
PBO <= 0.10
profit_factor improves vs baseline
max_drawdown and CVaR do not degrade beyond configured floor
return-correlation <= 0.65 vs accepted factors in same regime
incremental_portfolio_sharpe > replacement baseline
execution_tree recommendation changes in a useful direction
```

### Regime gate

```text
coverage >= 0.95 if conformal mode is used
calibration_ece <= 0.05
flip_rate below lane floor
mean_segment_bars above persistence floor
transition hazard exposed, not hidden
validated by an independent label source
```

### Options/VRP gate

```text
stress PnL includes 1987/2008/2020-style gap scenarios or synthetic equivalents
tail_loss_p99 / CVaR is explicit
margin and gap risk are modeled
bid-ask and stale quote filters active
gamma/vega/theta attribution present if options chain data exists
```

---

## Personal priority fields

These fields matter for the current user workflow and should be first-class if data exists:

```text
symbol=NQ / QQQ first, then ES/SPY, RTY/IWM, YM/DIA, GC, CL, BTC/ETH/SOL
qqq_hv_level
qqq_hv_pct_rank_252
nq_vs_200d_pct
vix3m_level
vvix_over_vix
vrp = implied_variance - realized_variance
iv_rank / hv_rank
session = Asia/London/NY AM/NY PM
mtf_resonance = m1/m5/m15/m30/h1/h4/d1/w1
dealer_pressure_proxy
crowding_pressure
liquidity_thinness
```

Default must still be zero-config. If these fields are missing, emit `missing_optional`, not failure.

---

## Paper harvest: P0 factor families

### 1. Time-Series Momentum / Trend Following

Sources:
- Moskowitz, Ooi, Pedersen, 2012, "Time Series Momentum". https://doi.org/10.1016/j.jfineco.2011.11.003
- Hurst, Ooi, Pedersen, 2017, "A Century of Evidence on Trend-Following Investing". https://www.aqr.com/Insights/Research/White-Papers/A-Century-of-Evidence-on-Trend-Following-Investing

Idea:
```text
signal = sign(return over 1m/3m/12m windows)
weight = signal / realized_vol
```

Implement:
- `tsmom_1m_3m_12m`
- `trend_convexity_score`
- `mtf_trend_resonance`
- `crisis_alpha_flag`

Map:
- `TrendMomentum`
- `market_state.structure`
- path-ranker features: `trend_persistence`, `mtf_alignment`, `vol_targeted_momentum`

Caveat: trend crowding and post-crisis reversals. Must pair with crowding/liquidity gate.

---

### 2. Cross-Asset Value + Momentum

Source:
- Asness, Moskowitz, Pedersen, 2013, "Value and Momentum Everywhere". https://doi.org/10.1111/jofi.12021

Idea:
```text
momentum = 12-1M return
value = asset-specific long-horizon cheapness proxy
portfolio = value + momentum with correlation control
```

Implement:
- `cross_asset_momentum_rank`
- `relative_value_rank`
- `value_momentum_diversifier_score`

Map:
- `CrossMarketSmt`
- `TrendMomentum`
- portfolio diversity report

Caveat: value definitions differ by asset class; treat as adapter-specific, not global formula.

---

### 3. Carry / Roll Yield

Source:
- Koijen, Moskowitz, Pedersen, Vrugt, 2018, "Carry". https://doi.org/10.1111/jofi.12643

Idea:
```text
futures_carry = log(near_price / far_price) / tenor
fx_carry = short_rate_base - short_rate_quote
options_carry = implied_vol - realized_vol
```

Implement:
- `futures_carry_percentile`
- `carry_momentum_blend`
- `carry_crash_risk_gate`

Map:
- `TrendMomentum`
- `CrossMarketSmt`
- `OptionsHedging`
- BBN: `carry_pressure`, `crash_risk`

Caveat: carry Sharpe often hides left-tail risk; require CVaR and stress replay.

---

### 4. Low Beta / Betting Against Beta

Source:
- Frazzini, Pedersen, 2014, "Betting Against Beta". https://doi.org/10.1016/j.jfineco.2013.10.005

Idea:
```text
beta_i = cov(r_i, r_market) / var(r_market)
long low_beta, short high_beta, beta-neutralized
```

Implement:
- `rolling_beta_rank`
- `low_beta_quality_sleeve`
- `beta_stability_score`

Map:
- equity/ETF adapter
- BBN: `factor_alignment`, `factor_uncertainty`

Caveat: financing/leverage cost and high-rate regimes can erase edge.

---

### 5. Quality Minus Junk

Source:
- Asness, Frazzini, Pedersen, 2019, "Quality Minus Junk". https://ssrn.com/abstract=2312432

Idea:
```text
quality = profitability + growth + safety + payout
```

Implement:
- `quality_score_equity_daily`
- `quality_momentum_overlay`

Map:
- hot-plug equity fundamental sidecar only

Caveat: needs lagged fundamentals; not for intraday NQ unless using ETF constituent proxy.

---

### 6. Momentum Crash Filter

Source:
- Daniel, Moskowitz, 2016, "Momentum Crashes". https://doi.org/10.1016/j.jfineco.2015.12.002

Idea:
```text
crash_risk = bear_market && high_realized_vol && sharp_market_rebound
if crash_risk: reduce/flip momentum exposure
```

Implement:
- `momentum_crash_gate`
- `trend_reversal_hazard`

Map:
- `ReversalBrewing`
- `ExtremeStress`
- execution tree: `transition_guardrail`

Caveat: this is a protection layer, not a standalone alpha.

---

### 7. FX Carry + FX Momentum

Sources:
- Menkhoff, Sarno, Schmeling, Schrimpf, 2012, "Currency Momentum Strategies". https://ssrn.com/abstract=1809776
- Menkhoff, Sarno, Schmeling, Schrimpf, 2012, "Carry Trades and Global Foreign Exchange Volatility". https://doi.org/10.1111/j.1540-6261.2012.01728.x
- Lustig, Roussanov, Verdelhan, 2011, "Common Risk Factors in Currency Markets". https://doi.org/10.1093/rfs/hhr068

Idea:
```text
fx_momentum = rank(spot_return_1m/3m/6m/12m)
fx_carry = high_rate_currency - low_rate_currency
global_fx_vol_gate = avg_fx_realized_vol or implied vol proxy
```

Implement:
- `fx_momentum_rank`
- `hml_fx_carry`
- `global_fx_vol_stress_gate`

Map:
- `CrossMarketSmt`
- `market_state.volatility`
- `market_state.liquidity`

Caveat: crisis correlation spike and EM liquidity.

---

### 8. Variance Risk Premium / Volatility Risk Premium

Sources:
- Bollerslev, Tauchen, Zhou, 2009, "Expected Stock Returns and Variance Risk Premia". https://doi.org/10.1093/rfs/hhp008
- Bakshi, Kapadia, 2003, "Delta-Hedged Gains and the Negative Market Volatility Risk Premium". https://doi.org/10.1093/rfs/hhg002
- Coval, Shumway, 2001, "Expected Option Returns". https://doi.org/10.1111/0022-1082.00352

Idea:
```text
VRP = implied_variance - realized_variance
delta_hedged_pnl = option_price_change - delta * underlying_price_change - financing
```

Implement:
- `vrp_rank_252`
- `qqq_vrp_pressure`
- `vvix_over_vix_stress`
- `short_vol_carry_candidate`
- `delta_hedged_option_pnl_attribution`

Map:
- `OptionsHedging`
- `VolatilityMeanReversion`
- BBN: `dealer_pressure`, `factor_uncertainty`, `crash_risk`

Caveat: high Sharpe can be short-tail illusion. Require CVaR, gap stress, margin model.

---

### 9. Volatility Spread / Option Momentum

Sources:
- Bali, Hovakimian, 2009, "Volatility Spreads and Expected Stock Returns". https://doi.org/10.1287/mnsc.1090.1063
- Heston, Jones, Khorram, 2023, "Option Momentum". https://doi.org/10.1111/jofi.13279

Idea:
```text
vol_spread = ATM_IV - realized_vol
option_momentum = past option return by moneyness/maturity bucket
```

Implement:
- `option_momentum_bucket_rank`
- `iv_minus_hv_spread_rank`
- `moneyness_liquidity_filter`

Map:
- `OptionsHedging`
- options sidecar artifact, not core runtime dependency

Caveat: stale quotes, bid-ask bounce, survivorship and chain selection.

---

### 10. Crypto Momentum / Size / Liquidity

Sources:
- Liu, Tsyvinski, Wu, 2022, "Common Risk Factors in Cryptocurrency". https://doi.org/10.1111/jofi.13119
- Liu, Tsyvinski, 2018/2021, "Risks and Returns of Cryptocurrency". https://doi.org/10.3386/w24877

Idea:
```text
crypto_market_factor
crypto_size = log(market_cap)
crypto_momentum = past 1w/4w return
crypto_liquidity = volume/market_cap or Amihud |ret|/volume
```

Implement:
- `crypto_momentum_rank`
- `crypto_liquidity_quality`
- `funding_pressure_proxy`

Map:
- `TrendMomentum`
- `SessionLiquidity`
- `CrowdingHerding`

Caveat: fake volume, exchange survivorship, 24/7 day boundary, funding effects.

---

### 11. Order Flow Imbalance / Book Pressure

Sources:
- Cont, Kukanov, Stoikov, 2014, "The Price Impact of Order Book Events". https://doi.org/10.1016/j.jfineco.2013.10.006
- Generic OFI/open LOB repos listed below.

Idea:
```text
book_imbalance = (bid_depth - ask_depth) / (bid_depth + ask_depth)
ofi = signed changes in best bid/ask size and price levels
trade_imbalance = buy_initiated_volume - sell_initiated_volume
```

Implement:
- `ofi_book_imbalance`
- `depth_thinning_score`
- `liquidity_sweep_pressure`

Map:
- `CrowdingHerding`
- `SessionLiquidity`
- execution tree: `fill_viable` vs `block_crowded`

Caveat: needs high-quality L2/trade sign data. If unavailable, use proxy-only and mark confidence low.

---

### 12. Statistical Arbitrage / Residual Mean Reversion

Source:
- "Deep Learning Statistical Arbitrage", arXiv:2106.04028. https://arxiv.org/abs/2106.04028
- Hudson & Thames ArbitrageLab repo references below.

Idea:
```text
residual = asset_return - conditional_factor_model_return
zscore_residual = (residual - rolling_mean) / rolling_std
trade reversion if residual extreme and regime stable
```

Implement:
- `residual_reversion_zscore`
- `ou_half_life_score`
- `cointegration_stability_score`

Map:
- `VolatilityMeanReversion`
- `CrossMarketSmt`
- BBN: `factor_alignment`, `liquidity_context`

Caveat: factor model instability and crowding. Needs walk-forward and turnover/slippage realism.

---

## Open-source repo harvest

Use repos as references. Copy only if license permits and attribution is preserved. Prefer formula reimplementation.

| Repo | License posture | Harvest | Runtime stance |
|---|---|---|---|
| https://github.com/microsoft/qlib | MIT | Alpha158/Alpha360 expressions, feature DSL, IC pipeline | reference expressions only |
| https://github.com/STHSF/alpha101 | MIT | WorldQuant Alpha101 pandas primitives | reimplement primitives/tests |
| https://github.com/quantopian/empyrical | Apache-2.0 | Sharpe, Sortino, Calmar, drawdown, tail ratio | reimplement metrics or sidecar |
| https://github.com/ranaroussi/quantstats | Apache-2.0 | report metrics, rolling stats | reference metric names |
| https://github.com/pmorissette/bt | MIT | rebalance algorithms, strategy tree | reference portfolio UX |
| https://github.com/hudson-and-thames/mlfinlab | restrictive/proprietary visible license | triple barrier, CUSUM, purged CV, bet sizing concepts | do not copy; use papers |
| https://github.com/hudson-and-thames/arbitragelab | BSD-3-Clause | OU, cointegration, Kalman, copula stat-arb | sidecar formulas allowed with attribution |
| https://github.com/robcarver17/pysystemtrade | GPL-3.0 | futures carry/trend/vol targeting concepts | do not copy/import |
| https://github.com/robcarver17/systematictradingexamples | GPL-2.0 | systematic futures examples | do not copy/import |
| https://github.com/mansoor-mamnoon/limit-order-book | MIT shown in README badge | LOB replay, OFI, imbalance, microstructure metrics | sidecar only |
| https://github.com/kernc/backtesting.py | AGPL-3.0 | simple backtest UX/stat naming | do not copy/import |
| https://github.com/quantrocket-llc/zipline | Apache-2.0 | calendars, slippage/commission abstraction | reference only |

---

## First 16 candidate specs

Each candidate writes a `factor_expression.json`, `candidate_spec.json`, `summary.json`, and `chain_verdict.json`.

| ID | Family | Formula seed | Primary lane | Verdict default |
|---|---|---|---|---|
| `tsmom_mtf_convexity_v1` | TrendMomentum | 1m/3m/12m sign blend + vol target | NQ/ES/GC/CL/BTC | probe |
| `trend_crash_guard_v1` | TrendMomentum/Regime | bear + high vol + rebound hazard | NQ/QQQ | probe |
| `carry_momentum_blend_v1` | CrossMarketSmt | carry percentile + 3m/12m momentum | futures basket | probe |
| `vrp_pressure_qqq_v1` | OptionsHedging | VIX/VIX3M/HV/VRP/VVIX over VIX | QQQ/NQ | probe |
| `iv_hv_spread_rank_v1` | OptionsHedging | ATM_IV - HV rank | SPY/QQQ liquid options | probe |
| `option_momentum_bucket_v1` | OptionsHedging | option return by moneyness/maturity bucket | SPY/QQQ | probe |
| `ofi_book_pressure_v1` | CrowdingHerding | bid/ask depth imbalance + signed flow | futures/crypto L2 | probe |
| `session_liquidity_quality_v1` | SessionLiquidity | range, volume, spread, sweep proxy by session | NQ intraday | probe |
| `alpha101_ts_rank_delta_v1` | TrendMomentum | ts_rank(delta(close,n),m) | all OHLCV | probe |
| `alpha101_corr_vol_price_v1` | CrowdingHerding | corr(rank(volume), rank(price move)) | all OHLCV | probe |
| `qlib_kline_shape_v1` | StructureIct | KMID/KLEN/KUP/KLOW/KSFT | all OHLCV | probe |
| `qlib_slope_bundle_v1` | TrendMomentum | MA/BETA/RSQR/ROC windows | all OHLCV | probe |
| `residual_ou_reversion_v1` | VolatilityMeanReversion | residual zscore + OU half-life | pairs/baskets | probe |
| `fx_hml_carry_v1` | CrossMarketSmt | high-rate minus low-rate basket | FX | probe |
| `crypto_mom_liquidity_v1` | TrendMomentum/SessionLiquidity | momentum + turnover/Amihud | crypto | probe |
| `low_beta_stability_v1` | equity sidecar | rolling beta rank + stability | equities/ETF | probe |

---

## Artifact schema

### `candidate_spec.json`

```json
{
  "candidate_id": "tsmom_mtf_convexity_v1",
  "source_refs": ["doi:10.1016/j.jfineco.2011.11.003"],
  "family": "trend_momentum",
  "markets": ["NQ", "ES", "GC", "CL", "BTC"],
  "timeframes": ["15m", "1h", "4h", "1d"],
  "required_fields": ["open", "high", "low", "close", "volume"],
  "optional_fields": ["qqq_hv_pct_rank_252", "vix3m_level", "vvix_over_vix"],
  "missing_optional_policy": "emit_missing_optional_and_continue",
  "promotion_gate": "probe"
}
```

### `factor_expression.json`

```json
{
  "expression_version": 1,
  "operators": ["delay", "delta", "rolling_mean", "rolling_std", "ts_rank", "rank", "zscore"],
  "formula": "zscore(sign(ret_63) + sign(ret_126) + sign(ret_252)) / realized_vol_63",
  "normalization": "rolling_zscore",
  "lookahead_safe": true
}
```

### `chain_verdict.json`

```json
{
  "candidate_id": "...",
  "verdict": "reject|probe|promote",
  "trade_count": 0,
  "oos_sharpe_lcb": 0.0,
  "dsr": 0.0,
  "pbo": 1.0,
  "max_drawdown": 0.0,
  "cvar_95": 0.0,
  "regime_slices_passed": [],
  "bbn_entropy_reduction": 0.0,
  "path_ranker_delta": 0.0,
  "execution_tree_changed": false,
  "failure_tags": []
}
```

---

## Infinite iteration loop

Run this forever in small batches. Commit only stable support/docs/tests/specs, never raw `/tmp` state.

```text
1. Select 3-5 candidates from the backlog.
2. Generate zero-config sidecar specs.
3. Run on NQ/QQQ first, then ES/SPY, then cross-market basket.
4. Write payoff report: Sharpe, Sortino, Calmar, CVaR, tail ratio, profit factor, hit rate, avg R/R, turnover.
5. Run purged OOS with embargo and DSR/PBO.
6. Split by regime: TrendExpansion, RangeConsolidation, ExtremeStress, ReversalBrewing, Unknown.
7. Check optional personal fields: QQQ HV, NQ vs 200D, VIX3M, VVIX/VIX, VRP.
8. Add BBN evidence only if entropy/log-loss improves.
9. Export path-ranker target and check contribution.
10. Run analyze/workflow-status and require recommendation delta.
11. Tag failures.
12. Promote only if gates pass.
13. Compress every 10-20 attempts: merge duplicates, prune dead params, update support/docs/factor-catalog.md.
14. Create next batch from failure tags and source backlog.
```

Failure tags:

```text
under_trades
thin_density
lookahead_risk
oos_decay
high_pbo
low_dsr
tail_risk_hidden
regime_confused
transition_late
high_flip_rate
bbn_no_uncertainty_reduction
path_ranker_no_delta
execution_tree_no_change
duplicate_payoff_source
data_dependency_too_heavy
license_copy_risk
```

---

## Implementation order

### R21: source registry doc and seed candidates

- Create this document.
- Add first candidate pack list.
- No runtime code.

### R22: formula sidecar seed library

- Add `support/scripts/research/factor_formula_seed_library.py`.
- Emit candidate specs for the first 16 IDs.
- No external framework dependency.
- Validate JSON schema.

### R23: payoff gate expansion

- Extend payoff report with DSR/PBO/CVaR/tail ratio if missing.
- Add OOS LCB and effective sample size.

### R24: options/VRP sidecar

- Add optional input schema for VIX/VIX3M/VVIX/HV/IV.
- Missing optional fields stay non-fatal.
- Emit `vrp_pressure_qqq_v1`.

### R25: OFI/session sidecar

- Add L2 optional schema.
- Fallback to OHLCV proxy if L2 missing.
- Emit `ofi_book_pressure_v1` with low confidence if proxy-only.

### R26: BBN value gate

- Only admit new factor evidence if entropy/log-loss/contradiction lift improves.

### R27: path-ranker and execution-tree closure

- Require target row, score contribution, and visible `workflow-status` reason before promotion.

---

## Verification commands

Use targeted commands; do not full-format the repo unless asked.

```bash
git status --short
cargo check
python3 support/scripts/research/factor_formula_seed_library.py --output /tmp/ict-hl/factor_seed_candidates.json
python3 support/scripts/research/factor_payoff_shape_report.py --help
./target/debug/ict-engine analyze --demo --symbol NQ --state-dir /tmp/ict-hl-smoke --human
./target/debug/ict-engine workflow-status --symbol NQ --state-dir /tmp/ict-hl-smoke --human
```

---

## Standing caveats

- High in-sample Sharpe is usually a bug, leakage, selection bias, short-tail exposure, or duplicate payoff source until proven otherwise.
- Options income must be judged by CVaR, margin stress, and gap replay, not Sharpe alone.
- GPL/AGPL/restrictive repos are idea sources only; do not copy code into runtime.
- Qlib/Alpha101 formulas are seeds, not truth. Mutate and validate by lane/regime.
- Runtime remains zero-config. User-specific auxiliary fields are opt-in and hot-plug.
