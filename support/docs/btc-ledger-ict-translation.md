# BTC ledger ICT translation

Artifacts
- factor snapshot: `state/btc_ledger/BTC/ledger_factor_snapshot.json`
- ICT translation: `state/btc_ledger/BTC/ledger_ict_objective.json`
- translator: `support/scripts/btc_ledger_ict_translate.py`

Verdict
- Current evidence says the account history is ICT-compatible enough to interpret with liquidity-grab -> expansion language.
- Do not overclaim bar-by-bar ICT reconstruction yet.
- Keep dual reading:
  - ICT reading for hypothesis generation
  - ledger-native reading as fallback truth

Why ICT mapping currently holds
- mild positive execution aggression, not extreme chase:
  - `execution_aggression_bias=0.0867`
- high order completion pressure:
  - `fill_completion_pressure=0.7261`
- BTC concentration dominates top traded symbols:
  - `btc_top_symbol_share=0.6652`
- wealth compounding is persistent:
  - `latest_adjusted_wealth_multiple=52.3973`
- drawdown floor still leaves the long-horizon curve intact:
  - `trough_adjusted_wealth_multiple=0.5502`

Working interpretation
- likely not random overtrading
- more consistent with repeated participation after decisive liquidity events and expansion follow-through
- usable as prior / regime-style annotation source
- not yet usable as direct signal oracle

Safe engineering use
1. use as offline labels for execution style and regime state
2. use ICT wording only in derived metadata / objective surfaces
3. keep original ledger metrics beside translated scores
4. if future bucketed replay contradicts ICT mapping, demote ICT layer and keep ledger-native layer

Do not do yet
- do not wire this directly into live `analyze`
- do not pretend fills reconstruct exact BOS/CHOCH/FVG events
- do not replace existing ICT factors with account-ledger heuristics
