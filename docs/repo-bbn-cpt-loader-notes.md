# Repo BBN CPT Loader Notes

## Added
- `src/bbn/trading/cpt_init.rs`
  - loads `repo_bbn_trading_cpt_init.json`
  - validates node states/parents against live trading network
  - applies CPT entries to:
    - `market_regime`
    - `liquidity_context`
    - `entry_quality`
    - `trade_outcome`
- `src/bbn/trading/mod.rs`
  - exports `cpt_init`
- `src/bbn/trading/topology.rs`
  - attempts to auto-load:
    `/Users/thrill3r/projects-ict-engine/ict-engine/state/policy_training/repo_bbn_trading_cpt_init.json`
  - falls back to legacy in-code defaults if file not present

## Current blocker
Repo-wide test compile is blocked by unrelated pre-existing main.rs errors:
- `CascadeResult::default()` missing
- locations:
  - `src/main.rs:21405`
  - `src/main.rs:21406`

So CPT loader smoke test could not be verified through `cargo test` yet.

## What is already verified externally
- JSON init file exists and parses as structured data
- node state order matches repo trading node state order
- CPT entries were generated for:
  - root priors
  - `entry_quality | market_regime, liquidity_context`
  - `trade_outcome | entry_quality`

## Next safe move
Once unrelated `CascadeResult` compile issue is fixed, run:
- `cargo test bbn::trading::cpt_init::tests::loads_and_applies_tomac_cpt_init -- --nocapture`
- then a higher-level inference smoke test against `trade_evidence_from_labels`
