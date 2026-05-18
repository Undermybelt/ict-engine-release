# BBN + Filter First Realignment

## Diagnosis
The recent path over-optimized execution/training surfaces before confirming the repo's intended order:
1. filter / denoise / gate
2. pre-bayes evidence filter
3. belief evidence packet / BBN nodes
4. only then policy / execution / downstream learners

That ordering is already visible in repo code:
- filter layer: `src/kalman/filter.rs`, `src/sv/particle_filter.rs`
- pre-bayes / belief bridge: `src/bbn/adapters/legacy_pre_bayes.rs`, `src/application/belief/*`
- BBN nodes: `src/bbn/trading/nodes.rs`, `src/bbn/evidence.rs`

## Existing canonical BBN surface
Current minimal trading BBN nodes:
- `market_regime` -> bull | bear | range
- `liquidity_context` -> favorable | neutral | hostile
- `entry_quality` -> high | medium | low
- `trade_outcome` -> win | breakeven | loss

Current pre-bayes-to-belief bridge already expects filter-style evidence such as:
- `filtered_market_regime_label`
- `filtered_liquidity_context_label`
- `filtered_multi_timeframe_resonance_label`
- `evidence_quality_score`
- timed PDA summary counts
- evidence assignments / rationale / conflict flags

## What the Tomac tables are good for
Tomac v4/v4_trainable are not execution trees. They are best treated as weak supervision / historical observation surfaces.
They currently provide:
- symbol
- direction_label
- result_label
- reason_label
- source_schema_type
- entry_kind / exit_kind
- pnl sign proxy

They do NOT yet provide canonical BBN evidence states like:
- market_regime
- liquidity_context
- entry_quality
- filtered resonance
- filter uncertainty / stress score
- timed PDA counts

## Filter-first rule
Before any belief-network training or execution-tree work:
1. produce denoised / gated observations
2. derive discrete evidence states
3. write a BBN evidence table
4. only then learn CPTs or downstream models

## Immediate next step
Create a BBN evidence dataset builder that maps available Tomac observations into the smallest valid packet:
- observed nodes:
  - `market_regime` (proxy / unknown if unavailable)
  - `liquidity_context` (proxy from reason/source family where possible)
- hidden-training target proxies:
  - `entry_quality` from source-specific heuristics
  - `trade_outcome` from TP/BE/SL/EOD
- metadata:
  - `direction_label`, `symbol`, `source_schema_type`, `reason_label`
- filter placeholders:
  - `filter_stage = raw_proxy`
  - `filter_confidence`
  - `filter_quality_bucket`

## Hard constraint
Do not call execution-tree work or CatBoost policy work "done" until a filter-aware BBN evidence surface exists.
