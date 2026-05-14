# 2026-04-26 auto-quant multi-asset data adapter

## Goal

Let Auto-Quant (FreqTrade-based) backtest non-crypto OHLCV data — stocks, ETFs,
futures, FX/CFDs — and ingest NSE option-chain snapshots, **without modifying
any FreqTrade source file**. Source-of-truth for the tooling lives here in
`ict-engine`; the Auto-Quant checkout is treated as a deploy target.

## Source-of-truth layout in this repo

```
support/scripts/auto_quant_external/
├── fetch_external.py          multi-source fetcher (yahoo / nse-options / polygon)
├── prepare_external.py        5-pass cleaner + CSV→feather adapter
├── run_tomac.py               runner with synthetic-market injection bypass
├── config.tomac.json          separate FreqTrade config (binance kept as ccxt placeholder)
└── strategies/
    └── TomacNQ_KillzoneBreakout.py    NQ futures example IStrategy
```

These files are intentionally **not** vendored under `src/` — they are operator
tooling, not part of the Rust binary surface. Treat them as a small companion
toolkit for whoever runs Auto-Quant locally.

## Deploy positions in an Auto-Quant checkout

Given an Auto-Quant checkout at `$AQ` (e.g. `~/Auto-Quant`):

```
$AQ/fetch_external.py                                ← support/scripts/auto_quant_external/fetch_external.py
$AQ/prepare_external.py                              ← support/scripts/auto_quant_external/prepare_external.py
$AQ/run_tomac.py                                     ← support/scripts/auto_quant_external/run_tomac.py
$AQ/config.tomac.json                                ← support/scripts/auto_quant_external/config.tomac.json
$AQ/user_data/strategies_external/                   ← support/scripts/auto_quant_external/strategies/
```

`strategies_external/` is intentionally a separate directory so that
`run_tomac.py` (which loads from it) never collides with Auto-Quant's master
`run.py` (which loads from `user_data/strategies/`). Both coexist with zero
edits to `run.py`, `prepare.py`, `config.json`, or any FreqTrade file.

A one-shot deploy is simply `cp -r` of the listed files; no symlinks required.

## End-to-end pipeline

1. **Fetch** raw OHLCV via `fetch_external.py yahoo --tickers ... --start ... --end ...`.
   Sub-commands: `yahoo`, `nse-options`, `polygon`. Yahoo path includes retry
   with exponential backoff and User-Agent rotation for 429/5xx mitigation.
2. **Clean + adapt** with `prepare_external.py --csv ... --pair NQ/USD --timeframes 1h,4h,1d`.
   The 5-pass cleaner handles: OHLC consistency, non-positive prices, negative
   volume, ghost bars, and configurable jump outliers. OTC instruments
   (forex/CFD where Yahoo reports volume=0 across the whole series) are
   detected by `median(volume) == 0` and the ghost-bar pass is skipped with a
   provenance note `ghost_bar_skipped: volume_series_all_zero_otc_instrument`.
   Output is FreqTrade feather under `user_data/data/{PAIR}-{TF}.feather`.
3. **Backtest** with `run_tomac.py`. The runner:
   - Loads `config.tomac.json` (whitelist contains pseudo-pairs like `NQ/USD`,
     exchange name kept as `binance` purely so CCXT validation passes).
   - Constructs the FreqTrade `Exchange` object **and then injects synthetic
     market entries** into `exchange._markets` matching the whitelist. This
     makes `IPairList._whitelist_for_active_markets` pass without contacting
     any real exchange and without touching FreqTrade source.
   - Hands the patched exchange to `Backtesting`, which accepts a
     pre-constructed exchange object (verified upstream behavior).

## Asset-class coverage (verified 2026-04-26 via Yahoo Finance free chart API)

| Class      | Symbol      | 1d bars | Range                       | Notes                      |
| ---------- | ----------- | ------- | --------------------------- | -------------------------- |
| stock      | `AAPL`      | 751     | 2023-01-03 .. 2025-12-30    |                            |
| etf        | `SPY`       | 751     | 2023-01-03 .. 2025-12-30    |                            |
| futures    | `ES=F`      | 754     | 2023-01-03 .. 2025-12-30    | continuous front-month     |
| cfd / fx   | `EURUSD=X`  | 779     | 2023-01-02 .. 2025-12-31    | OTC, volume=0 honored      |
| crypto     | `BTC-USD`   | 1096    | 2023-01-01 .. 2025-12-31    | full daily span            |

End-to-end backtest proof (previous session, databento NQ 1h+4h+1d feathers):
3 trades, win-rate 66.7%, sharpe 0.005, max-drawdown -1.10%, **0 FreqTrade
source modifications**.

## New providers (2026-04-26 iteration)

Four additional REST surfaces, all key-free for read:

| Sub-command | Asset class hit | Verified output |
| ----------- | --------------- | --------------- |
| `bybit-kline` | crypto spot, USDT/inverse perps, option contracts | 365 daily rows on `linear/BTCUSDT` |
| `bybit-options` | **BTC/ETH/SOL USDC options with delta/gamma/theta/vega** | 500 contracts × 9 expiries on BTC, 100% Greeks |
| `kraken-kline` (spot) | crypto, **xStocks tokenised U.S. equity (AAPLx/SPYx)**, fiat forex with **real volume** | 256–721 daily rows depending on instrument |
| `kraken-kline` (futures) | crypto perps + **PF_* equity-index perps (PF_SPXUSD…)** | 214 daily rows on `PF_SPXUSD` |
| `binance-kline` | crypto spot | 365 daily rows on `BTCUSDT` |
| `binance-options` | **BTC/ETH European options (`/eapi`) with Greeks** | 538 contracts × 12 expiries on BTC, 100% Greeks |
| `polymarket-markets` | prediction-market discovery | works from clean-routed exit; **GFW blocks** `gamma-api.polymarket.com` |
| `polymarket-history` | CLOB token mid-price history (alt-data implied probability) | same network constraint |

Key gotchas pinned during smoke tests:

- Bybit V5 option symbols are now `BASE-EXPIRY-STRIKE-SIDE-SETTLE` (5 segments).
  Older 4-segment form is also accepted.
- Kraken `/0/public/OHLC` requires `--asset-class tokenized_asset` for xStocks
  and `--asset-class forex` for fiat pairs; without it the endpoint replies
  `EGeneral:Invalid arguments`.
- xStocks pair names use a lowercase `x`: `AAPLxUSD`, `SPYxUSD`.
- All four new fetchers retry on `SSLError` / `ConnectionError` / `Timeout`
  with exponential backoff so the GFW's intermittent TLS resets are
  self-healing for kraken/bybit/binance. Polymarket from CN IP is not
  recoverable in code — use a clean-routed exit.

Auth-key acquisition surface (operator does this when ready to trade; data
endpoints above all work without it): see
`support/docs/2026-04-26-multi-exchange-data-source-integration-plan.md` for the
sign-up URL / scope checklist per provider.

## IBKR live data bridge (2026-04-26 iteration, opt-in)

**Disabled by default. Localhost-only. Bilingual disclaimer + explicit opt-in.**

Solves the "single TWS / IB Gateway login session, multiple Python consumers"
problem with a single producer + Redis Streams fan-out:

```
TWS / IB Gateway (single login)
       ↑ clientId=20
   ibkr-bridge.py  (this iteration)
       ↓ XADD bars / ticks / snapshot
   Redis (localhost:6379, loopback-bound + protected-mode)
       ↓ XREAD / HGETALL
  Auto-Quant strategy        ict-engine research script
  IbkrConsumer               IbkrConsumer
```

| Surface | Run as | Output |
| --- | --- | --- |
| `python -m support.scripts.ibkr_bridge.setup --enable` | one-time, interactive | bilingual disclaimer + opt-in + Redis/Gateway ping + account probe |
| `python -m support.scripts.ibkr_bridge.setup status` | non-interactive | current consent / capabilities / Redis / Gateway state |
| `python -m support.scripts.ibkr_bridge.setup revoke --clean-redis` | one-time | wipes consent + capabilities + ibkr:* Redis keys |
| `python -m support.scripts.ibkr_bridge.bridge --config <yaml>` | always-on producer | Redis streams `ibkr:bars:{sym}:5sec`, `ibkr:ticks:{sym}`, `ibkr:snapshot:{sym}` |
| `from ibkr_bridge.consumer import IbkrConsumer` | importable | snapshot/bars/ticks/stream_bars/stream_ticks |
| `fetch_external.py ibkr-historical --symbol AAPL --bar-size '1 hour' --duration '60 D' --output ...` | one-shot | canonical CSV (ts, open, high, low, close, volume, wap, count) |

Cross-process pacing: `bridge.py`, `fetch_external.py ibkr-historical`, and
any future ad-hoc IBKR script all share the same `IbkrRateLimiter` state in
Redis. Hitting IBKR's 6 s/contract or 60-distinct/10-min budgets is
impossible to do accidentally even when running 3 different processes
concurrently — each request waits in the same global queue.

OSS user experience guarantees:
* Default `subscriptions: []` in `example_config.yaml` — fresh clones do
  not consume any market-data lines if the bridge is launched.
* Consent gate (`require_ibkr_enabled()`) on all IBKR-touching entry
  points; non-interactive sessions get a clear instruction message rather
  than a silent failure.
* Read-only consumer never requires consent — Auto-Quant strategies that
  fall back gracefully when the bridge is offline can still be safely
  imported and CI-tested.
* No telemetry; no third-party network calls; no IBKR credentials handled
  by Python (they stay inside IB Gateway/TWS).

Full design + decision log: `support/docs/2026-04-26-ibkr-live-data-bridge-plan.md`.
Operator manual + bilingual disclaimer: `support/scripts/ibkr_bridge/README.md`.

## Option-chain coverage (NSE)

`fetch_external.py nse-options` implements the distilled NSE flow:

1. `GET /option-chain` warmup → cookies.
2. `GET /api/underlying-information` → list of indices + stocks.
3. `GET /api/option-chain-v3?type=...&expiry=...` → per-symbol chain.

Output is flattened to a wide CSV with columns:

```
snapshot_utc, expiry, strike,
call_oi, call_chng_oi, call_iv, call_ltp, call_volume,
put_oi,  put_chng_oi,  put_iv,  put_ltp,  put_volume
```

**Network constraint**: NSE's edge (Akamai) geofences non-Indian IPs and
returns 403/blocked HTML before reaching the API. Fetcher works as designed
when called from an Indian IP; from CN/elsewhere it requires VPN.

**Backtest constraint**: FreqTrade's `IStrategy` interface is single-instrument
OHLCV — it has no native dimension for strike, expiry, or Greeks. Option-chain
snapshots are therefore ingested as **context artifacts** (e.g. as features
for an underlying-asset strategy), not as a backtestable instrument. A
direct option-chain backtester would require a separate engine; this is out
of scope for the Auto-Quant integration.

## Auto-Quant remotes (post-fork)

The Auto-Quant checkout is wired with two remotes:

```
origin    git@github.com:Undermybelt/Auto-Quant.git    (your fork — read+write)
upstream  git@github.com:TraderAlice/Auto-Quant.git    (original — read only)
```

This separates "what the original ships" from "what we run". Your work branches
push to `origin`; you decide when to pull `upstream` changes.

## Upgrade workflow

When upstream releases changes:

```bash
cd ~/Auto-Quant
git fetch upstream                                     # see what changed
git log master..upstream/master --oneline              # review diffs
git checkout master && git merge upstream/master       # merge into your fork's master
git push origin master                                 # publish to your fork
git checkout autoresearch/apr26                        # back to working branch
git merge master                                       # bring upstream changes in
```

Then **always re-deploy + smoke-test**:

```bash
cd ~/projects-ict-engine/ict-engine                    # source-of-truth
cp support/scripts/auto_quant_external/*.py            ~/Auto-Quant/
cp support/scripts/auto_quant_external/config.tomac.json ~/Auto-Quant/
cp support/scripts/auto_quant_external/strategies/*.py ~/Auto-Quant/user_data/strategies_external/
cd ~/Auto-Quant
uv run run_tomac.py                                    # smoke-test the pipeline
```

If FreqTrade internals shifted (`Backtesting(exchange=...)` ctor, `_markets`
mutation point, or `IPairList._whitelist_for_active_markets`), the smoke-test
will fail loudly. Patch `run_tomac.py` in this repo, re-deploy.

## File-collision protection

The 5 deployed files live under unique paths and are listed in
`~/Auto-Quant/.git/info/exclude` (a local-only ignore that never travels to any
remote and never conflicts with upstream's tracked `.gitignore`). Effects:

- `git status` in Auto-Quant is clean — the deploy files are invisible.
- If upstream ever creates a tracked file at one of those exact paths,
  `git pull` aborts with "untracked working tree files would be overwritten by
  merge". You move the deploy file aside, pull, then re-deploy from ict-engine.
- `user_data/data/` is already covered by upstream's tracked `.gitignore`, so
  generated feathers are upstream-safe by design.

## Why this lives in `ict-engine`, not in Auto-Quant

`support/docs/auto-quant-ictengine-integration-guide.md` mandates: ict-engine is the
canonical decision-maker, Auto-Quant is an external workspace whose source
must not be polluted with ict-engine's experimental tooling. Storing this
toolkit under `support/scripts/auto_quant_external/` keeps the source-of-truth
versioned in our repo while the Auto-Quant checkout stays a clean upstream
mirror that anyone can reset to HEAD.
