# BTC ledger factor intake

Source:
- `/tmp/ict_repo_intake/BTC-Trading-Since-2020`
- generated snapshot: `state/btc_ledger/BTC/ledger_factor_snapshot.json`
- extractor: `support/scripts/btc_ledger_factor_extract.py`

Absorption verdict:
- Learn + absorb.
- Not a runtime dependency.
- Treat as offline research corpus for factor ideation, regime labels, execution-style priors, and equity-state supervision.

What was extracted
- trade ledger summary from `api-v1-execution-tradeHistory.csv`
- order lifecycle summary from `api-v1-order.csv`
- wallet PnL / cashflow summary from `api-v1-user-walletHistory.csv`
- equity state summary from `derived-equity-curve.csv`

Immediate factor candidates
1. `execution_aggression_bias`
   - removed-liquidity share minus added-liquidity share
   - maps trader style: chase vs passive absorb
2. `symbol_concentration_entropy`
   - concentration / diffusion across traded symbols
   - maps BTC-focus vs alt dispersion regimes
3. `wallet_realized_pnl_pulse`
   - daily realized PnL pulse from wallet history
   - candidate supervision label for state transitions
4. `fill_completion_pressure`
   - average fill ratio by session
   - candidate proxy for liquidity / urgency / order quality
5. `equity_drawdown_state`
   - adjusted wealth multiple or drawdown bucket
   - candidate regime state label, not price predictor

Use guidance
- Prefer this corpus for offline labeling and hypothesis generation first.
- Do not force-fit it into live analyze path yet.
- Best near-term path: build a separate research script that emits time-bucketed features and labels, then compare against existing factor-research outputs.
- Current one-shot translation says this ledger is ICT-compatible enough to read as post-liquidity-grab expansion capture, but keep ledger-native interpretation alongside ICT language.
- ICT-first only while evidence stays stable; if later bucketed reconstruction breaks the mapping, fall back to ledger-native execution-style labels.

Non-goals
- not HFT order-book modeling
- not direct copying of account behavior as trading policy
- not replacing existing ICT / BBN logic with account-ledger heuristics
