# Factor Catalog — Single-Page Index

> Source of truth for factor family → code → status traceability.
> Update this file when adding/removing/mutating any factor family.

## Active Rust Factors (5)

| # | Name | Category | Compute Entry | Subfactors (code) | Parameters |
|---|---|---|---|---|---|
| 1 | `trend_momentum` | TrendMomentum | `factor_definition.rs` L365 | EMA slope, RSI persistence, ADX trend | fast_period, slow_period, rsi_period, adx_period |
| 2 | `volatility_mean_reversion` | VolatilityMeanReversion | `factor_definition.rs` L380 | Bollinger displacement, ATR vol regime | bollinger_period, bollinger_std, adx_period, atr_period |
| 3 | `structure_ict` | StructureIct | `factor_definition.rs` L396 | ICT sweeps, OB, FVG, CISD, setups | lookback, expansion_threshold, sweep_atr_multiplier, sweep_return_bars, sweep_weight, ... |
| 4 | `cross_market_smt` | CrossMarketSmt | `factor_definition.rs` L420 | SMT divergence, relative strength | lookback |
| 5 | `options_hedging` | OptionsHedging | `factor_definition.rs` L431 | Dealer hedge proxy (requires auxiliary data) | atr_period |

## Design-Level Families (8, from execution-tree TODO)

| Family | Name | Rust Coverage | Missing Subfactors | Priority |
|---|---|---|---|---|
| A | Structure / Setup Quality | `structure_ict` partial | crowding pressure, setup recency quality, post-manipulation continuation | HIGH |
| B | Directionality / Persistence | `trend_momentum` partial | continuation-failure detection, exhaustion signs, momentum quality | HIGH |
| C | Cross-Market Confirmation | `cross_market_smt` partial | leader-laggard confirmation, correlation-consistency regime fit | MEDIUM |
| D | Stretch / Reversion Feasibility | `volatility_mean_reversion` partial | OU reversion speed, exhaustion after leg extension, bounce probability | HIGH |
| E | Crowding / Herding Execution Risk | `CrowdingHerding` (new) | same-direction herd intensity, crowding persistence, crowding collapse setup | CRITICAL → compute stub added |
| F | Spectral Rhythm / Chaos | `SpectralRhythm` (new) | spectral entropy factor, dominant cycle energy factor, rhythm stability | CRITICAL → compute stub added |
| G | Options / Dealer Positioning | `options_hedging` (data-limited) | gamma skew, hedge pressure, put/call OI imbalance, IV convexity | MEDIUM |
| H | Session / Liquidity Window Quality | `SessionLiquidity` (new) | session participation quality, kill-zone alignment, session transition risk | HIGH → compute stub added |

## Mutation / Autoresearch Surface

- CLI: `ict-engine factor-research --backend auto-quant`
- CLI: `ict-engine factor-autoresearch --backend auto-quant`
- CLI: `ict-engine factor-autoresearch-status --latest-only`
- Mutation routing: `src/application/factor_lifecycle/mutation_routing.rs`
- Mutation templates: `src/application/factor_lifecycle/mutation_templates.rs`
- Expansion scoring: `src/application/factor_lifecycle/expansion_scoring.rs`

## Execution-Tree Factor Consumers

| Tree Branch | Primary Factor Need | Secondary Factor Need |
|---|---|---|
| `block_crowded` | Family E (crowding) | Family F (spectral chaos) |
| `wait_for_reversion` | Family D (stretch/reversion) | Family F (spectral rhythm) |
| `fill_viable` | Family A (structure), Family B (directionality) | Family C (cross-market), Family H (session) |
| weak evidence gate | Family C (cross-market confirmation) | Family G (options evidence) |
| noisy environment gate | Family F (spectral chaos) | Family H (session quality) |
