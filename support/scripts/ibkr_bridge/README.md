# IBKR Bridge — out-of-the-box guide

A small set of opt-in tools that turn a locally-running IB Gateway / TWS into
a Redis-published market-data source for backtest dataset construction and
live factor research. Designed to work even on free / paper-only IBKR
accounts, with the bridge gracefully falling back to delayed data when a
live MD subscription is missing.

## Components

| File | Purpose |
|---|---|
| `setup.py` | One-shot interactive opt-in (consent + capabilities probe). Run first. |
| `bridge.py` | Long-running publisher. Connects to Gateway, subscribes to instruments, publishes ticks/bars/snapshots to Redis. |
| `consumer.py` | Read-side reference: consumes Redis streams as pandas DataFrames. |
| `rate_limiter.py` | Cross-process pacing manager (60 historical / 10 min, etc.). |
| `ibkr_errors.py` | Bilingual error-code translator with suggested fixes. |
| `account_prober.py` | Detects paper vs live, sub-account count, line cap. |
| `consent.py` | Stores opt-in state at `~/.ict-engine/ibkr_consent.json`. |

## Quick start (5 min)

```bash
# 1. Make sure IB Gateway / TWS is running locally with API enabled.
#    If you don't know the port, setup.py will auto-scan the common ports:
#    TWS 7497/7496, IB Gateway 4002/4001.
#    with API enabled and "Read-Only API" ticked.

# 2. One-time consent + capabilities probe (~5 sec, briefly connects to Gateway)
.venv/bin/python -m support.scripts.ibkr_bridge.setup enable

#    If multiple local runtimes are reachable, or your user runs a custom port,
#    pin it explicitly:
#    .venv/bin/python -m support.scripts.ibkr_bridge.setup enable --gateway-port 4002

# 3. Pick a starter config that matches your account state — see "Which
#    config to start from?" below — and run the bridge:
.venv/bin/python -m support.scripts.ibkr_bridge.bridge \
    --config support/scripts/ibkr_bridge/examples/safe_paper.yaml

# 4. From any other process / strategy file, consume the data:
.venv/bin/python -m support.scripts.ibkr_bridge.consumer SPY
```

Everything stays on `localhost`. The bridge only talks to **your** Gateway
and **your** Redis. No telemetry, no third-party servers.

## Which example config to start from?

| If your situation is… | Start with |
|---|---|
| Brand-new bridge user, want it to "just work" | `support/scripts/ibkr_bridge/examples/safe_paper.yaml` |
| Building 5-min factor research, market closed or open | `support/scripts/ibkr_bridge/examples/intraday_5min.yaml` |
| Weekend / overnight / want non-zero data right now | `support/scripts/ibkr_bridge/examples/crypto_24x7.yaml` |
| Forex mid-price kline research | `support/scripts/ibkr_bridge/examples/forex_idealpro.yaml` |
| Constructing a backtest dataset (CSV files) | `support/scripts/ibkr_bridge/examples/bulk_dataset.yaml` (used by `fetch_external.py ibkr-bulk`) |

## Feed types

A subscription declares one or more `feed:` entries:

| `feed` | Backed by | Best for | MD-subscription needed? |
|---|---|---|---|
| `bars_kup` | `reqHistoricalData(keepUpToDate=True)` | **Default for factor research** — any bar size, back-fills + streams updates | No (uses historical farm) |
| `market_data` | `reqMktData` | Full bid/ask/last/sizes tick stream | Yes for live; `market_data_type: 3` for delayed |
| `real_time_bars` | `reqRealTimeBars` | 5-second OHLCV bars only | Yes (live MD required) |

The smart default is **`bars_kup`** — it works at any bar size, doesn't
require a live MD subscription, and back-fills on first connect so consumers
replaying from `start_id='0'` see a full backtest window immediately.

## Redis schema

| Key | Type | Producer | Notes |
|---|---|---|---|
| `ibkr:bridge:status` | hash | bridge | `state`, `ts`, `gateway_host`, `gateway_port`, `market_data_type`, `client_id` (actual runtime), `configured_client_id`, `client_id_fallback_engaged`, `client_id_conflicts`, `subscriptions_active` |
| `ibkr:snapshot:<SYM>` | hash | bridge | Latest bid/ask/last/sizes + `last_bar_close` |
| `ibkr:ticks:<SYM>` | stream | bridge (`market_data`) | Tick events with bid/ask/last/sizes |
| `ibkr:bars:<SYM>:5sec` | stream | bridge (`real_time_bars`) | 5-second OHLCV bars |
| `ibkr:bars:<SYM>:5min` | stream | bridge (`bars_kup`) | 5-min OHLCV (suffix matches `_BAR_SIZE_SUFFIX` table) |
| `ibkr:bars:<SYM>:1h` etc. | stream | bridge (`bars_kup`) | One stream per bar size |
| `ibkr:rl:*` | various | rate_limiter | Pacing budget; do not write |

Streams are bounded by `publishing.stream_maxlen` (XADD with MAXLEN ~).

The consumer helper exposes two read paths:

- `bridge_status()` — raw/coerced hash view
- `bridge_runtime_summary()` — typed runtime summary with parsed `client_id`,
  `configured_client_id`, fallback boolean, conflict list, active subscription count,
  and gateway host/port
- `recommended_gateway_target()` — a small decision surface for downstream agents:
  current host/port, actual vs configured `client_id`, fallback/conflict state,
  and a one-line message

## Common errors and fixes

The bridge logs every IBKR error in **bilingual + remediation** form. A
quick reference for the codes you're most likely to see:

| Code | Severity | One-line fix |
|---|---|---|
| **10089** | error | Set `market_data_type: 3` to use delayed quotes |
| **10197** | error | Log out other live IBKR sessions (mobile app, web Portal); ensure live MD package subscribed |
| **420** | error | Switch to `market_data_type: 3` OR subscribe to the specific exchange MD package (NASDAQ TotalView, AMEX TOP, …) |
| **10299** | error | For crypto bars_kup use `what_to_show: MIDPOINT` (`TRADES` errors, `AGGTRADES` is incompatible with `keepUpToDate`) |
| **162** | error | Empty result; widen `duration` and/or use `--rth false` |
| **2104/2106/2158** | info | Farm-OK status; suppressed automatically |

Full catalog with English + 中文 explanations: `ibkr_errors.py`.

If you hit a code not in the catalog, the log line will say
`(uncatalogued — please file an issue)`; please paste the IBKR raw
message + the surrounding context into a GitHub issue and we'll add it.

## Free MD subscriptions to enable on your live account

If your account has zero MD subscriptions, subscribe these (all $0.00/mo
for IBKR-PRO clients) via Client Portal → User Settings → Market Data:

* **IBKR-PRO 非整合实时报价** / "IBKR-PRO Non-Consolidated Real-Time Quotes"
  — covers most US equities at non-NBBO routes
* **IDEALPRO 外汇** / "IDEALPRO Forex" — real-time spot FX
* **PAXOS IBLLC-美国（非美国）** — crypto MD (BTC/ETH/LTC/BCH)
* **CME 事件合约** / "CME Event Contracts"
* **美国共同基金（一级）** / "US Mutual Funds Level 1"

After subscribing, **restart Gateway** (File → Exit, wait 30s, relaunch)
so the new entitlements load. Then restart the bridge.

## Troubleshooting recipes

### "Bridge starts but ticks never arrive"

Check the snapshot:

```bash
redis-cli hgetall ibkr:snapshot:SPY
```

If `bid=-1, ask=-1`: market is closed for that asset class (FX weekend,
US equity overnight). Wait for the session, or switch to `crypto_24x7.yaml`.

If empty hash: bridge couldn't subscribe — check the bridge log for an
`IBKR error` block; the bilingual message includes the fix.

### "I don't want to wait for IBKR; I want a CSV right now"

Use `ibkr-bulk` for batch back-fill (no streaming):

```bash
.venv/bin/python fetch_external.py ibkr-bulk \
    --config support/scripts/ibkr_bridge/examples/bulk_dataset.yaml
```

Writes one CSV per `(symbol, bar_size)` to `user_data/data/ibkr_bulk/`.
Re-runs are idempotent (skip existing files unless `--force`).

### "Gateway disconnects the bridge after a few seconds"

Some IBKR account states (paper account on free tier, multiple sessions
elsewhere) cause IBKR to drop the API socket after the first request.
The bridge auto-reconnects with exponential backoff. If you want zero
drops:

1. Make sure all other IBKR sessions are logged out (mobile, web)
2. Enable at least one MD subscription on the live account
3. Wait 60–90s after subscribe for IBKR to propagate entitlements

### "Where do I revoke consent?"

```bash
.venv/bin/python -m support.scripts.ibkr_bridge.setup revoke
# or delete: ~/.ict-engine/ibkr_consent.json
```

## Architecture notes

* **State location**: capabilities at `~/.ict-engine/ibkr_capabilities.json`,
  consent at `~/.ict-engine/ibkr_consent.json`. Both gitignored.
* **Client IDs**: bridge=20, single-fetch=21, bulk-fetch=22 (avoid clash).
* **Read-only**: the bridge connects with `readonly=True` so it cannot
  place / cancel / modify orders even if your code somehow tried.
* **Cross-process pacing**: `rate_limiter.py` holds budget state in Redis
  so multiple bridges + ad-hoc fetches share the same 60-req/10-min window.
