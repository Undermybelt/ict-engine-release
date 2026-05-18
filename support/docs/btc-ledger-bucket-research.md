# BTC ledger bucket research

Artifacts
- script: `support/scripts/btc_ledger_bucket_research.py`
- runs: `state/btc_ledger/BTC_LEDGER/research_runs.json`
- bucket objectives: `state/btc_ledger/BTC_LEDGER/ledger_bucket_objectives.json`
- per-bucket payloads: `state/btc_ledger/BTC_LEDGER/ledger_bucket_*.json`

What this does
- slices `BTC-Trading-Since-2020` into 30-day buckets
- emits ICT-engine-like `research_runs.json`
- keeps two parallel surfaces per bucket:
  - `btc_ledger_ict_interpretability`
  - `btc_ledger_execution_native`

Why
- lets us test whether ICT reading is stable across time
- avoids forcing a single grand narrative on the full 6-year ledger
- preserves a ledger-native fallback when ICT language becomes too lossy

Current read
- bucket 0001:
  - best factor: `btc_ledger_ict_interpretability`
  - aggregate_return: `0.544135`
  - verdict: `ict-compatible execution logic`
- bucket 0002:
  - best factor: `btc_ledger_execution_native`
  - aggregate_return: `-1.115086`
  - verdict still ICT-compatible, but native surface scored higher
- bucket 0003:
  - ICT surface still slightly leads
  - aggression rose sharply (`0.3918`), so this is a stress bucket for the ICT mapping

Interpretation rule
- if ICT surface wins repeatedly and remains near or above native surface, keep ICT language as a valid explanatory layer
- if native surface starts dominating in stressed buckets, demote ICT to a soft annotation layer and trust ledger-native labels first

Use next
- compare bucket winners over the full run
- cluster buckets where `btc_ledger_execution_native` beats ICT
- inspect those periods for whether the account behavior is still explainable as post-liquidity-grab expansion capture or whether it becomes style-specific execution logic outside clean ICT framing
