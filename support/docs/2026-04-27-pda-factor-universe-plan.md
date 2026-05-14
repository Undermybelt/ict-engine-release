# PDA Factor Universe & Combinatorial Reduction Plan

Date: 2026-04-27
Status: implemented
Scope: ICT-style PDA factors + control-variable matrix for `auto-quant-factor-research`

Implementation status snapshot:
- P1 canonical setup matching is present in `src/pda_timeline/*` and consumed by `FactorDefinition::evaluate_structure_ict`, including cross-TF / paired-symbol context
- P2 PB(12) control-matrix runner is implemented in `src/application/backtest/control_matrix.rs` and `factor_research_command`
- P3 discovery-pool summary is implemented as a PB12 artifact surface, and `auto-quant-promote-canonical-setup` now persists repo-versioned promoted setup specs
- P4 provider readiness/install prompt surfaces plus direct IBKR / yfinance / TradingView MCP fetch/runtime consumption are implemented behind the market-data harness

## 1. Context

The user has a draft of nine `PDArrayType` detectors (`OrderBlock`,
`FairValueGap`, `InverseFVG`, `BreakerBlock`, `MitigationBlock`,
`RejectionBlock`, `PropulsionBlock`, `LiquidityVoid`,
`VolumeImbalance`) and wants to plug ordered combinations of them
into ict-engine's BBN evidence + execution-tree node priors. They
also want to control-vary across:

- 5 timeframes: 1m / 5m / 15m / 1h / 4h
- options exposure: Greeks / OI / IV (each on/off)
- cross-market reference: ETF / CFD (each on/off)
- volatility regime overlay: VIX (on/off)
- higher-timeframe structure overlay: daily / weekly (each on/off)

Markets are futures only: GC (gold), NQ (Nasdaq-100), YM (Dow), ES
(S&P 500), 6E (EUR/USD currency futures).

The user's heuristic comparison data (image, 2026-04-26) already
shows MSS (Market Structure Shift) and CISD (Change in State of
Delivery) as separate dimensions; those are not in the original
nine PDAs, so the universe is at least **eleven event types**.

This plan answers: *how do we cover this combinatorial space without
exploding the BBN with prior-dominated empty cells?*

## 2. Problem statement

### 2.1 Cell explosion under naive enumeration

Treating every "ordered subsequence of PDAs × control toggles ×
timeframes × symbols" as a distinct BBN cell:

```
P(11, k) ordered length-k sequences from 11 events:
  k=2 → 110     k=3 → 990     k=4 → 7,920
  k=5 → 55,440  k=6 → 332,640

Control toggles (8 binary + 1 four-level):
  256 conditions

Timeframes:    5
Symbols:       5

Length-4 cell count:
  7,920 × 256 × 5 × 5 = 50.7 M cells

Length-1..5 cumulative:
  ≈ 64,500 sequences × 256 × 5 × 5 ≈ 413 M cells
```

Beta-Binomial posteriors need on the order of **30 outcomes per
cell** to escape prior dominance. 50 M cells ⇒ 1.5 B labelled
trades — orders of magnitude beyond what 5 futures contracts'
multi-year history can supply, even unsanitised.

### 2.2 Why this matters for the existing BBN

`@/Users/thrill3r/projects-ict-engine/ict-engine/src/application/backtest/feedback.rs:122-163`
projects every realised trade onto a fixed three-axis evidence
tuple (`entry_quality`, `factor_alignment`, `factor_uncertainty`)
before calling `CPTUpdater::default().batch_update(...)`. The
trade-outcome CPT row count is therefore **bounded by the product
of those three axes' state counts**, not by `factor_name`. Adding
new factors does not multiply the CPT — it multiplies the
*upstream* feature space that maps into those three labels.

So the design constraint is not "fit 50 M factors into the CPT"
(impossible and unnecessary); it is **"choose a small, dense factor
universe whose label projections meaningfully spread the existing
three-axis CPT."** Anything else is decoration.

## 3. The eleven events

### 3.1 Detector contracts (target shape, not yet implemented)

Each detector returns `Vec<PdaEvent>` rather than `Option<PdaEvent>`
so multiple PDAs can co-fire on the same bar (an OB bar can
simultaneously be a RejectionBlock; the user's draft loses that
information by `if`/`else if` cascading).

```rust
pub struct PdaEvent {
    pub kind: PdaKind,
    pub bar_index: usize,
    pub bar_close_ts_ms: i64,
    pub range_high: f64,
    pub range_low: f64,
    pub strength: f64,             // 0..=1 — drives confidence
    pub timeframe: Timeframe,
    pub symbol: SymbolId,
}
```

### 3.2 The 11 kinds

| # | Kind | Bar window | Trigger summary |
|---|---|---|---|
| 1 | `OrderBlock` | 2 bars | Last opposite-direction candle followed by ≥1 ATR displacement |
| 2 | `FairValueGap` | 3 bars | Non-overlap between bar i-2 and bar i ranges (BISI / SIBI) |
| 3 | `InverseFVG` | ≥4 bars | Prior FVG fully traded through, then re-broken in opposite direction |
| 4 | `BreakerBlock` | ≥5 bars | OB violated then revisited — failure becomes structure |
| 5 | `MitigationBlock` | ≥4 bars | Failed swing (revisit without taking previous extreme) |
| 6 | `RejectionBlock` | 1 bar | Wick ≥ 2× body in dominant direction |
| 7 | `PropulsionBlock` | 1 bar | Body / range > θ (ATR-relative) AND volume > μ + 2σ |
| 8 | `LiquidityVoid` | 3 bars | ATR-normalised gap ≥ θ_void (replaces user's hard-coded 0.0030 forex pip) |
| 9 | `VolumeImbalance` | 1 bar + window | volume > 3σ over rolling N (window = 50, configurable) |
| 10 | `MarketStructureShift` (MSS) | swing-pivot | Higher-low broken (bearish) or lower-high broken (bullish) confirmed by close |
| 11 | `ChangeInStateOfDelivery` (CISD) | ≥3 bars | Three consecutive bars deliver opposite to prior trend (close beyond opposite extreme) |

### 3.3 Co-occurrence matrix (one bar)

`✓` = can co-fire on the *same* bar; `×` = mutually exclusive by
definition. The detector layer must respect this. (Cells inferred
from ICT literature; deviations to be flagged in code review.)

```
        OB FVG iFVG BB MB RB PB LV VI MSS CISD
 OB     —   ✓   ×    ×  ×  ✓  ✓  ✓  ✓  ✓   ×
 FVG    ✓   —   ×    ×  ×  ✓  ✓  ✓  ✓  ✓   ✓
 iFVG   ×   ×   —    ×  ×  ✓  ✓  ✓  ✓  ✓   ✓
 BB     ×   ×   ×    —  ×  ✓  ✓  ✓  ✓  ✓   ✓
 MB     ×   ×   ×    ×  —  ✓  ✓  ✓  ✓  ✓   ×
 RB     ✓   ✓   ✓    ✓  ✓  —  ×  ✓  ✓  ✓   ✓
 PB     ✓   ✓   ✓    ✓  ✓  ×  —  ✓  ✓  ✓   ✓
 LV     ✓   ✓   ✓    ✓  ✓  ✓  ✓  —  ✓  ✓   ✓
 VI     ✓   ✓   ✓    ✓  ✓  ✓  ✓  ✓  —  ✓   ✓
 MSS    ✓   ✓   ✓    ✓  ✓  ✓  ✓  ✓  ✓  —   ✓
 CISD   ×   ✓   ✓    ✓  ×  ✓  ✓  ✓  ✓  ✓   —
```

Key rules baked in:

- `OB` and `iFVG` cannot co-fire on the same bar — iFVG by
  definition consumes a prior FVG, so the bar in question is the
  destruction of structure rather than the creation of OB.
- `RejectionBlock` and `PropulsionBlock` are mutually exclusive at
  one bar (rejection is wick-dominant, propulsion is body-dominant).
- `OB`, `MitigationBlock`, and `CISD` are mutually exclusive at the
  reversal pivot (each is a different reading of "this is where
  the swing failed/succeeded").

### 3.4 Temporal precedence (which event can follow which)

A *valid* sequence respects ICT narrative ordering. Restricting to
sequences that respect this matrix collapses the 332,640 length-6
sequences to **fewer than 100** that ICT theory considers tradable.

Allowed predecessors per kind (read row → column means "row can
follow column"):

```
            OB FVG iFVG BB MB RB PB LV VI MSS CISD
 OB         —   —   —    —  —  ✓  ✓  ✓  ✓  ✓   ✓
 FVG        ✓   ✓   ✓    ✓  ✓  ✓  ✓  ✓  ✓  ✓   ✓
 iFVG       —   ✓   —    —  —  —  —  —  —  ✓   ✓
 BreakerBlk —   —   —    —  ✓  —  —  —  —  ✓   ✓
 MitigBlk   ✓   —   —    —  —  —  —  —  —  ✓   —
 RejBlk     ✓   ✓   ✓    ✓  ✓  —  —  ✓  ✓  ✓   ✓
 PropBlk    ✓   ✓   ✓    ✓  ✓  —  —  ✓  ✓  ✓   ✓
 LiqVoid    —   —   —    —  —  ✓  ✓  —  ✓  —   ✓
 VolImbal   ✓   ✓   ✓    ✓  ✓  ✓  ✓  ✓  —  ✓   ✓
 MSS        ✓   ✓   ✓    ✓  ✓  ✓  ✓  ✓  ✓  —   ✓
 CISD       ✓   ✓   ✓    ✓  —  ✓  ✓  ✓  ✓  ✓   —
```

Notable constraints:

- `iFVG` must be preceded by `FVG`, `MSS`, or `CISD` — anything
  else is by definition not an inverse FVG (no FVG to invert).
- `BreakerBlock` must be preceded by `MitigationBlock`, `MSS`, or
  `CISD` — i.e. there must be a structural failure before an OB
  can be re-classified as a breaker.
- `OB` cannot follow another `OB` directly — successive bars of OB
  formation collapse into a single OB (the second is noise).

## 4. Reduction strategy

### 4.1 PDA layer: ~30 canonical setups

We do **not** enumerate the full precedence-respecting space.
Instead we hard-code the canonical ICT setups as named entries in
a `CanonicalSetup` enum, ordered roughly by ICT-community priority:

```
1.  HtfMssLtfFvg                   // 4h MSS → 15m FVG retest
2.  HtfCisdLtfObRetest             // 4h CISD → 1h OB retest
3.  DailyHighSweepLtfMssFvg        // daily liquidity sweep → 15m MSS → 5m FVG
4.  DailyLowSweepLtfMssFvg         // mirror of #3
5.  WeeklyOpenSweepDailyMss        // weekly-open liquidity sweep → daily MSS
6.  AsiaRangeRaidLondonMss         // session-driven liquidity raid
7.  LondonRaidNyMssFvg             // London draws liquidity, NY confirms
8.  PowerOfThree (PO3)             // accumulation → manipulation → distribution
9.  TurtleSoupLiquidityGrab        // false break of obvious liquidity
10. ObRetestPropulsionConfirm      // OB retest + propulsion close
11. iFvgContinuation               // FVG → MSS → iFVG continuation
12. BreakerBlockRetest             // failed swing → breaker formation
13. MitigationBlockRetest          // failed swing → mitigation retest
14. RejectionBlockAtKeyLevel       // RB at HTF level (PDH/PDL/4h OB)
15. VolumeImbalanceFiller          // VI fills as continuation
16. LiquidityVoidContinuation      // LV continuation in trend direction
17. PropulsionPostMss              // propulsion bar immediately after MSS
18. CisdAfterDistribution          // CISD ending distribution phase
19. CisdAfterAccumulation          // CISD ending accumulation phase
20. OteWithFvgConfluence           // 0.62-0.79 retracement + FVG
21. OteWithObConfluence            // 0.62-0.79 retracement + OB
22. SmtDivergenceConfirm           // related-symbol SMT divergence + MSS
23. EquityFuturesSmt               // ES/NQ/YM SMT divergence
24. CurrencyFuturesSmt             // 6E vs DXY SMT divergence
25. GoldVixDivergence              // GC vs VIX divergence
26. SilverBulletWindow             // 10:00-11:00 NY FVG
27. SilverBulletAm                 // 03:00-04:00 NY FVG
28. JudasSwingReversal             // first-hour false move + reverse
29. UnicornModel                   // breaker + FVG overlap
30. OptimalTradeEntryWithCisd      // OTE retracement + CISD confirmation
```

Each setup is a hard-coded matcher over the `Vec<PdaEvent>`
timeline produced by §3.1's detectors. Setups can overlap on the
same bar (e.g., `HtfMssLtfFvg` and `OteWithFvgConfluence` may both
fire); each match emits an independent `FactorContribution`.

The remaining (precedence-valid but not-yet-canonical) sequences
become the **discovery pool** for `auto-quant-factor-research` to
explore. Discovery-pool factors do **not** seed BBN priors; they
only become canonical once a factor-research run shows statistically
significant edge over the prior-init baseline (Phase 1 path,
`@/Users/thrill3r/projects-ict-engine/ict-engine/src/application/auto_quant/results`).

### 4.2 Control variables: 8-binary + 1-four-level → PB(12)

The "control variables" the user listed are not domain factors;
they are *experimental conditions* whose presence/absence we must
isolate to know which add information. This is the textbook use
case for a fractional factorial design.

**Binary toggles (k = 8):**
1. `use_greeks` — include options Greeks (ICT-engine treats Δ, Γ, Θ, ν, ρ as one feature bundle; finer slicing comes after main-effect identification)
2. `use_oi` — include open interest
3. `use_iv` — include implied volatility
4. `use_etf` — include matched ETF reference (GLD/QQQ/DIA/SPY/FXE)
5. `use_cfd` — include matched CFD reference
6. `use_vix` — include VIX overlay
7. `use_daily_structure` — daily-level price structure + PDA
8. `use_weekly_structure` — weekly-level price structure + PDA

(The user's "daily/weekly" 4-level toggle is decomposed into two
binaries 7 + 8; their cross product reproduces the four states
none/daily/weekly/both, with the added benefit of letting PB
estimate `use_daily` and `use_weekly` main effects independently.)

**Plackett-Burman 12-run design (PB(12))** identifies up to 11
two-level main effects per run, so 8 binaries fit comfortably with
3 free columns left for two-factor confounding control. The
canonical PB(12) Hadamard-derived sign matrix is:

```
Run  T1 T2 T3 T4 T5 T6 T7 T8 — — —
 1   +  +  −  +  +  +  −  −  −  +  −
 2   −  +  +  −  +  +  +  −  −  −  +
 3   +  −  +  +  −  +  +  +  −  −  −
 4   −  +  −  +  +  −  +  +  +  −  −
 5   −  −  +  −  +  +  −  +  +  +  −
 6   −  −  −  +  −  +  +  −  +  +  +
 7   +  −  −  −  +  −  +  +  −  +  +
 8   +  +  −  −  −  +  −  +  +  −  +
 9   +  +  +  −  −  −  +  −  +  +  −
10   −  +  +  +  −  −  −  +  −  +  +
11   +  −  +  +  +  −  −  −  +  −  +
12   −  −  −  −  −  −  −  −  −  −  −
```

Run 12 is the all-off baseline ("no overlays"); the user explicitly
asked for this group. Each run is a separate
`auto-quant-factor-research` invocation with the implied toggle
state baked in, producing an independent `ResearchReport`.

After PB(12) main effects are fit (≈ 12 backtest runs per
{symbol, hierarchy, setup}), follow-up full-factorial only on the
2-3 toggles whose main effects clear a chosen significance
threshold (default 95% Bayesian credible interval excludes zero).

### 4.3 Timeframe layer: 3 hierarchies, not 5 independents

Bare 5-timeframe enumeration produces 5 × 4 = 20 (HTF, LTF) pairs;
ICT theory recognises only top-down nesting. We pin three canonical
hierarchies mirroring the user's image (1H → 15m, 15m → 5m):

```
H1: (4h HTF, 15m MTF, 5m  LTF)   // swing trader
H2: (1h HTF, 15m MTF, 1m  LTF)   // intraday
H3: (4h HTF, 1h  MTF, 15m LTF)   // position bias
```

Daily/weekly structure (toggles 7-8 in §4.2) overlays these three;
it does not become a fourth hierarchy.

### 4.4 Final cell budget

```
30 setups × 3 hierarchies × 12 PB runs × 5 symbols
= 5,400 cells
```

At 30 outcomes/cell minimum convergence, that is **162,000
labelled trades**. Distributed across 5 futures contracts × multi-
year unsanitised history, this is feasible (NQ alone has ~1M
5-minute bars over 5 years; even 1% trade-eligibility yields 10K
events per symbol). Realistically the discovery pool will surface
additional canonical setups, growing the count toward 7-8K cells —
still tractable.

## 5. Concrete integration points

The plan plugs into existing ict-engine surfaces; nothing new at
the BBN evidence layer is required, which is exactly the point of
§2.2.

| Stage | Existing surface | What plugs in |
|---|---|---|
| Detection | new `src/factors/pda_array.rs` (alongside `@/Users/thrill3r/projects-ict-engine/ict-engine/src/factors/registry.rs`) | 11 PDA detectors returning `Vec<PdaEvent>` |
| Timeline assembly | new `src/factors/pda_timeline.rs` | merge per-TF event streams, enforce co-occurrence + precedence matrices |
| Canonical setup matching | new `src/factors/canonical_setups.rs` | 30 named matchers over the timeline |
| Factor signal emission | `@/Users/thrill3r/projects-ict-engine/ict-engine/src/factor_lab/factor_definition.rs:786-848` `FactorDefinition::evaluate_structure_ict` | extend to consume canonical-setup matches; produces `FactorSignal` with `FactorCategory::StructureIct` |
| Diagnostics aggregation | `@/Users/thrill3r/projects-ict-engine/ict-engine/src/factor_lab/engine.rs:78-146` `FactorEngine::run` | unchanged — already aggregates across factors |
| Control-variable runner | new `src/factors/control_matrix.rs` + extension to `@/Users/thrill3r/projects-ict-engine/ict-engine/src/application/backtest/command_entry.rs:274` `factor_research_command` | PB(12) iteration driver |
| Per-cell prior init | existing Phase 1 `@/Users/thrill3r/projects-ict-engine/ict-engine/src/application/auto_quant/results/prior_init.rs` | unchanged — each setup contributes a tempered Beta-Binomial prior |
| Realised-trade feedback | existing Phase 3 `@/Users/thrill3r/projects-ict-engine/ict-engine/src/application/auto_quant/real_trades/ingest.rs` | unchanged — `FeedbackFactorUsage` already carries `factor_name` so per-setup posteriors update naturally |
| BBN CPT projection | `@/Users/thrill3r/projects-ict-engine/ict-engine/src/application/backtest/feedback.rs:122-163` `apply_feedback_to_trade_outcome_network` | unchanged — three-axis projection (entry_quality, factor_alignment, factor_uncertainty) |

`FactorCategory::StructureIct` permits roles `PriorAdjuster`,
`StateTransition`, `SetupClassifier`, `OutcomeValidator` per
`@/Users/thrill3r/projects-ict-engine/ict-engine/src/factor_lab/factor_definition.rs:50-55`.
PDA setups will be tagged as `SetupClassifier` (primary) +
`PriorAdjuster` (secondary). They are deliberately **not**
`Evidence` — that role is reserved for factors that the BBN can
update against directly, and we want PDA setups to flow through
the existing three-axis projection rather than open a new
evidence node.

## 6. Phased delivery plan

Each phase is an independently committable, testable artifact.
None of them are this doc; the doc is the contract.

**P0 — bug-fix the user's draft & land detectors**
- `src/factors/pda_array.rs`: 11 detectors with ATR-relative
  thresholds, forward-leak guard, `Vec` returns, full unit tests
- `src/factors/timeframe.rs`: `Timeframe` enum + canonical hierarchy
  triples (H1/H2/H3)

**P1 — timeline + canonical-setup matchers**
- `src/factors/pda_timeline.rs`: enforce co-occurrence + precedence
- `src/factors/canonical_setups.rs`: 30 setups, each with golden
  fixtures
- Wire into `evaluate_structure_ict` so existing
  `FactorEngine::run` picks them up automatically

**P2 — control-variable matrix runner**
- `src/factors/control_matrix.rs`: PB(12) sign-matrix + run
  iterator
- Extend `factor_research_command` with `--control-matrix pb12`
  switch; default remains the all-off baseline so existing
  callers are not broken
- New ledger artifact_kind `auto_quant_pb12_research_run`

**P3 — discovery-pool feedback loop**
- After each PB(12) sweep, surface the top-N
  precedence-valid-but-not-canonical sequences whose posterior
  win-rate beats the prior-init baseline by ≥ a Bayesian threshold
- Operator-driven promotion: a new
  `auto-quant-promote-canonical-setup` CLI command appends a
  named entry to `canonical_setups.rs` via a code-generation step
  (out of scope for this doc; tracked separately)

**P4 — ETF / CFD / VIX / Greeks / OI / IV ingestion**
- Each control-variable toggle requires a data source. Per-source
  adapters go under new `src/data_sources/<source>.rs` modules and
  are gated by their toggle so `use_etf=false` short-circuits the
  fetch path. Phase split: ETF + VIX first (cheapest, most public),
  Greeks/OI/IV second, CFD last.

## 7. Out of scope for this doc

- Real-time fill ingestion (Phase 3 already ships JSONL daily-
  reconciliation; intra-day fill streaming is a separate plan)
- Multi-strategy aggregation across canonical setups (each setup
  is its own factor; ensemble logic lives in
  `@/Users/thrill3r/projects-ict-engine/ict-engine/src/application/auto_quant/adoption.rs` already)
- Auth / signing of the control-matrix research artifacts
- Cross-symbol prior sharing (deliberately rejected — see §2.2)

## 8. Risks

| Risk | Mitigation |
|---|---|
| Forward-leak in detectors | Mandatory unit-test rule: every detector must be evaluated as a closure over `candles[..=index]` only; tests assert detectors built on `candles[..=i]` produce the same `PdaEvent` list as detectors built on the full slice and read at index `i` |
| ATR threshold drift across symbols | Per-symbol per-timeframe ATR baseline cached in state; recomputed on a documented cadence; baseline freshness recorded in ledger |
| PB(12) confounding with strong two-factor interaction | If the 95% credible interval for any main effect crosses zero AND a known interaction is plausible, fall back to full factorial on those 2-3 toggles only |
| Canonical setups frozen at 30 prematurely | Discovery pool is first-class; the list is meant to grow; promotion is operator-gated, not automatic |
| Cell budget creep as setups multiply | Hard cap (e.g. 50 canonical setups per `canonical_setups.rs`); breaching the cap requires a doc bump and a fresh budget review |
| `iFVG` / `BreakerBlock` precedence-validation overhead at scale | Timeline assembly is O(n × k) where k is the longest precedence chain we accept (≤ 6); benchmark before P2 lands |
| Detector co-occurrence matrix drift from ICT theory | The matrix in §3.3 is checked-in source code (`src/factors/pda_array.rs`); changes require a doc PR + at least one regression fixture |

## 9. Rollback recipe

The detection / canonical-setup / control-matrix layers introduce
**no in-place state mutation** — all state changes flow through
the existing Phase 1 prior-init and Phase 3 real-trade ingestion
paths, both of which already document content-hash-keyed rollback
(see `@/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/2026-04-26-auto-quant-real-trades-plan.md` §"Rollback recipe").

To roll back a PB(12) sweep that turned out to corrupt priors:

1. Identify the offending `auto_quant_pb12_research_run`
   ledger entries (`source_run_id` = sweep id).
2. Use the existing Phase 1 rollback: delete `bbn_network.json` +
   `learning_state.json` for the affected symbols.
3. Re-run `auto-quant-prior-init` (Phase 1) to rebuild from the
   strategy library minus the offending sweep.
4. Re-run any subsequent `auto-quant-ingest-real-trades` artifacts
   in original order.

Ledger entries from the bad sweep remain on disk as audit history.

## 10. References

- Existing factor universe & roles:
  - `@/Users/thrill3r/projects-ict-engine/ict-engine/src/factor_lab/factor_definition.rs:19-103`
  - `@/Users/thrill3r/projects-ict-engine/ict-engine/src/factor_lab/engine.rs:11-49`
  - `@/Users/thrill3r/projects-ict-engine/ict-engine/src/factors/registry.rs:11-23`
- BBN evidence projection:
  - `@/Users/thrill3r/projects-ict-engine/ict-engine/src/application/backtest/feedback.rs:122-173`
- Phase 1 / 2 / 3 docs (read these for ledger + state-machine conventions):
  - `@/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/2026-04-26-auto-quant-bbn-prior-init-plan.md`
  - `@/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/2026-04-26-auto-quant-live-signals-plan.md`
  - `@/Users/thrill3r/projects-ict-engine/ict-engine/support/docs/2026-04-26-auto-quant-real-trades-plan.md`
- ICT-community priors on canonical setups (informal, non-versioned):
  ICT 2022 Mentorship video set; The Inner Circle Trader YouTube
  archive — used as inspiration only; every numerical threshold
  in the implementation must be backed by a unit-test fixture, not
  a video timestamp.

## 11. Decision points pending operator review

Before P0 starts, the user should confirm or override:

1. The 30 canonical setups list (§4.1) — any to add / drop?
2. The 8 binary toggles (§4.2) — any missing / redundant?
3. The 3 timeframe hierarchies (§4.3) — any to swap?
4. The 5,400-cell budget — acceptable? Or compress further by
   dropping one of {hierarchy variants, control toggles, setups}?
5. Whether the discovery-pool feedback loop (§6 P3) is in scope
   for the immediate iteration or a later one.
