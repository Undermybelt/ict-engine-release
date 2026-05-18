# Entry Model Factor Backlog

Date: 2026-04-28
Status: active backlog
Goal: replace weak primitive-only factors with stronger ICT-style entry models and accumulate the right feature base for HMM / MCMC / BBN / CatBoost to serve live trade suggestions.

## Core stance

- Stop treating isolated primitives as the end-state.
- Treat them as building blocks inside entry models.
- Optimize for live decision usefulness, not concept purity.
- Prioritize factors that can become:
  - stable entry models
  - low-cardinality regime/evidence states for BBN
  - numeric/categorical feature packets for CatBoost
  - sequential observations for HMM
  - probabilistic priors/likelihood components for MCMC

## External reference models

These are reference inspirations, not source-of-truth:

- ICT OTE / Optimal Trade Entry
  - https://innercircletrader.net/tutorials/ict-optimal-trade-entry-ote-pattern/
- ICT Unicorn Model
  - https://innercircletrader.net/id/tutorials/ict-unicorn-model/
- ICT Judas Swing
  - https://innercircletrader.net/tutorials/ict-judas-swing-complete-guide/
- ICT Silver Bullet
  - https://innercircletrader.net/tutorials/ict-silver-bullet-strategy/
- ICT IOFED
  - https://innercircletrader.net/id/tutorials/ict-institutional-order-flow-entry-drill/

## What to train instead

### A. First-class entry models

- [ ] `MSS -> FVG retest -> continuation`
  - Long and short separately
  - Trend-following only
  - This is the first priority model

- [ ] `CISD -> FVG retest`
  - Long and short separately
  - Allow both reversal and continuation labels, but do not mix them in the same run
  - This is the second priority model

- [ ] `Breaker + FVG overlap (Unicorn)`
  - Entry model, not just a pattern tag
  - Validate whether overlap actually improves entry timing vs standalone breaker / standalone FVG

- [ ] `OTE + MSS`
  - OTE zone as retracement constraint
  - MSS as trigger
  - FVG as execution refinement only if it improves win rate

- [ ] `OTE + CISD`
  - Same structure as above, but with delivery-state shift trigger

- [ ] `Judas Swing -> MSS/FVG`
  - Treat as a time-windowed false-move entry model
  - Especially useful for NY session research

- [ ] `Silver Bullet FVG`
  - Explicit session-window entry model
  - Separate AM / PM windows

- [ ] `IOFED / CE entry drill`
  - Entry refinement model
  - Only worth keeping if it improves execution quality, not just concept elegance

### B. Multi-timeframe resonance factors

- [ ] `HTF bias alignment`
  - Daily / 4H / 1H directional agreement
  - Not as a setup, but as a context factor

- [ ] `HTF dealing range location`
  - Premium / discount / equilibrium
  - Where the entry happens inside the higher-timeframe range

- [ ] `LTF trigger quality inside HTF bias`
  - Example: 15m MSS inside 4H bullish structure

- [ ] `Session-window alignment`
  - Asia / London / NY / specific kill-zone
  - Separate “in-window” from “out-of-window”

- [ ] `Resonance score`
  - Count how many of these line up:
    - HTF bias
    - session window
    - trigger event
    - retracement quality
    - liquidity map location

### C. Options / volatility / derivatives context

- [ ] `IV regime factor`
  - IV percentile / IV expansion / IV crush context

- [ ] `OI pressure factor`
  - OI expansion / contraction around the entry zone

- [ ] `Greeks pressure factor`
  - Gamma concentration
  - Delta imbalance
  - Vega sensitivity

- [ ] `Options wall proximity`
  - Distance to gamma wall / high OI strike cluster

- [ ] `Put/Call skew factor`
  - OI skew
  - volume skew
  - IV skew

- [ ] `Volatility expansion context`
  - ATR regime
  - session range expansion
  - realized volatility percentile

### D. Cross-market / SMT / related-symbol context

- [ ] `Index SMT factor`
  - NQ vs ES / YM

- [ ] `Risk proxy divergence`
  - index futures vs volatility proxy / sector ETF / breadth proxy

- [ ] `Leader-laggard factor`
  - which market breaks structure first
  - whether laggard catches up or diverges

- [ ] `Cross-market confirmation score`
  - not a setup; a context strength factor

## What to accumulate as foundation

### 1. Entry packet truth

Every candidate trade/event should persist:

- setup model id
- primitive sequence
- symbol
- timeframe
- direction
- session tag
- HTF bias
- HTF dealing range location
- trigger bar timestamp
- entry bar timestamp
- invalidation rule
- target logic

### 2. Evidence packet

Persist these as raw features, not just final scores:

- displacement size vs ATR
- FVG size vs ATR
- retracement depth
- distance to equilibrium / OTE
- liquidity sweep distance
- sweep reclaimed or not
- MSS/CISD confirmation delay
- session range percentile
- volatility percentile
- OI / IV / Greeks snapshots when available
- cross-market divergence values when available

### 3. Outcome packet

Need multi-horizon outcome labels, not just one PnL:

- `outcome_1`
- `outcome_2`
- `outcome_4`
- `outcome_8`
- `MFE`
- `MAE`
- `time_to_target`
- `time_to_invalidation`

This is what lets us build:
- HMM state observations
- BBN categorical bins
- CatBoost feature table
- MCMC likelihood updates

## How to make HMM / MCMC / BBN / CatBoost useful

### HMM

Use HMM for latent regime/state sequences, not for final entry decision directly.

Best inputs:
- volatility state
- session state
- trend / range state
- liquidity interaction state
- cross-market divergence state

Do not feed HMM with “setup name only”.

### BBN

BBN wants compact, interpretable bins.

Good BBN factor families:
- trend alignment
- retracement quality
- liquidity interaction quality
- trigger confirmation quality
- session quality
- options pressure quality
- cross-market confirmation quality

### CatBoost

CatBoost should be the rich scorer on top of the same packet.

Best features:
- all numeric packet fields
- categorical setup model id
- categorical session
- categorical HTF state
- availability flags for options / OI / IV / Greeks

### MCMC

Use MCMC to update uncertainty around:
- which entry model is truly better
- whether edge is regime-dependent
- whether options context actually changes expected outcome

## Priority order for next validation

### Wave 1: stop wasting time on weak primitives

- [ ] `MSS -> FVG retest -> continuation`
- [ ] `CISD -> FVG retest`
- [ ] `Unicorn`
- [ ] `OTE + MSS`

### Wave 2: context multipliers

- [ ] HTF alignment
- [ ] session-window alignment
- [ ] resonance score
- [ ] volatility regime filter

### Wave 3: derivatives / options

- [ ] IV regime
- [ ] OI pressure
- [ ] Greeks pressure
- [ ] options wall proximity

### Wave 4: cross-market

- [ ] index SMT
- [ ] leader-laggard divergence
- [ ] cross-market confirmation score

## Immediate todo

- [ ] Replace primitive-first iteration with entry-model-first iteration
- [ ] Start with `MSS -> FVG retest` and `CISD -> FVG retest`
- [ ] Keep long and short fully separated
- [ ] Keep each run at `one model × one symbol × one timeframe`
- [ ] Add outcome windows `1/2/4/8`
- [ ] Add `MFE/MAE`
- [ ] Add HTF location tags
- [ ] Add session tags
- [ ] Add evidence availability flags
- [ ] Add options / OI / IV / Greeks only when provider truth exists

## Reference benchmark notes

User-supplied benchmark, for orientation only:

- `E3 + FVGR_TOP + 4H Bull + 1H Transition`
  - `count = 238`
  - `winrate_4 = 77.31%`
  - `avg_return_4 = +33.74bp`
  - `median_return_4 = +35.08bp`
  - `MFE/MAE = 4.257`

- `E3 + FVGR_TOP + 4H Transition + 1H Transition`
  - `count = 720`
  - `winrate_4 = 68.06%`
  - `avg_return_4 = +45.04bp`
  - `median_return_4 = +31.75bp`
  - `MFE/MAE = 3.384`

Interpretation:

- the market is likely paying more for structured entry models with HTF/MTF context
- this strengthens the case for prioritizing:
  - `MSS/CISD -> FVG retest`
  - HTF bias / transition state
  - multi-timeframe resonance
  - execution-window context
