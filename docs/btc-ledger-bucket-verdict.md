# BTC ledger bucket verdict

Artifacts
- verdict script: `scripts/btc_ledger_bucket_verdict.py`
- verdict json: `state/btc_ledger/BTC_LEDGER/ledger_bucket_verdict.json`

Question answered
- Across bucketed runs, when does ICT interpretation beat ledger-native interpretation?
- Which buckets should be treated as mismatch buckets where ICT language should be demoted?

Read order
1. `ledger_bucket_verdict.json`
2. `research_runs.json`
3. individual `ledger_bucket_XXXX.json`

Operating rule
- If bucket winner is `ict`, ICT remains a valid explanatory layer.
- If bucket winner is `native`, keep ledger-native framing first and treat ICT as soft annotation only.
- Buckets in `native_mismatch_buckets` are the primary evidence for the boundary of ICT explanation.

Use next
- inspect top native mismatch buckets
- cluster them by aggression / drawdown / symbol dispersion
- define explicit demotion rules for when ledger-native should override ICT phrasing
