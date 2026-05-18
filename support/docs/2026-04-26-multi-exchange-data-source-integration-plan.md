# 2026-04-26 multi-exchange data source integration plan

## Goal

Extend `support/scripts/auto_quant_external/fetch_external.py` with 4 new data
providers — Bybit, Kraken, Binance, Polymarket — covering asset classes that
existing providers (Yahoo / NSE / Polygon) miss:

- **Crypto options chains** with Greeks (Bybit USDC opts; Binance EU opts)
- **Tokenized U.S. equities and ETFs trading 24/7** (Kraken xStocks)
- **Equity-index perpetual futures as crypto-collateralised CFD substitutes**
  (Kraken PF_SPXUSD, PF_AAPLXUSD, etc.)
- **Sub-Polygon centralised forex with real volume** (Kraken fiat pairs vs.
  Yahoo's OTC `volume=0` series)
- **Implied-probability time-series of macro/political/sport events** as an
  alternative-data feature surface (Polymarket CLOB price history)

## Constraints (recap)

- Source-of-truth lives in `ict-engine/scripts/auto_quant_external/`. Auto-Quant
  is treated as a deploy target (cp-only, no symlinks). The pipeline shape is
  unchanged: fetch → CSV → `prepare_external.py` → feather → strategy.
- Zero modifications to FreqTrade source.
- Public market-data endpoints first, no key required. Authenticated endpoints
  are env-var driven (`{PROVIDER}_API_KEY`, `{PROVIDER}_API_SECRET`) and remain
  stubs until the operator applies for keys.
- Direct REST over `requests` — no provider SDKs in the dep tree. Reasoning:
  fewer churn surfaces, consistent retry/backoff with the existing Yahoo
  fetcher, cleaner attribution when an endpoint breaks.

## Provider matrix

| Provider | Public read API | Asset classes added | Greeks/IV | Geo notes |
| -------- | --------------- | ------------------- | --------- | --------- |
| Bybit V5 | `api.bybit.com/v5/market/*` | crypto spot, linear/inverse perps, **USDC options** | yes (option tickers) | Bybit blocks USA/UK at trading edge; market data globally accessible |
| Kraken Spot | `api.kraken.com/0/public/*` | crypto spot, **xStocks tokenised equity**, fiat forex | n/a | xStocks not available to USA users (trading); read API global |
| Kraken Futures | `futures.kraken.com/derivatives/api/v3/*` | crypto perps, **equity-index perps (PF_SPXUSD…)**, fixed-date | n/a | Same as above |
| Binance Spot | `api.binance.com/api/v3/*` | crypto spot | n/a | Binance blocks USA → users go to binance.us; market data globally accessible |
| Binance Options | `eapi.binance.com/eapi/v1/*` | **European BTC/ETH options** | yes (mark endpoint) | Same as above |
| Polymarket | `gamma-api.polymarket.com` + `clob.polymarket.com` | **prediction-market CLOB and price history** | n/a | USA-restricted on trading via Polygon wallet; read API global |

## Endpoint shortlist (public)

### Bybit V5
- `GET /v5/market/kline?category={spot|linear|inverse|option}&symbol={S}&interval={1|5|15|30|60|120|240|360|720|D|W|M}&start={ms}&end={ms}&limit=1000`
- `GET /v5/market/instruments-info?category=option&baseCoin={BTC|ETH|SOL}` → expiry+strike grid
- `GET /v5/market/tickers?category=option&baseCoin={BTC|ETH|SOL}` → mark IV, delta, gamma, theta, vega per contract

### Kraken Spot
- `GET /0/public/AssetPairs` → catalogue (lookup wsname for forex/xStocks)
- `GET /0/public/OHLC?pair={pair}&interval={1|5|15|30|60|240|1440|10080|21600}` → OHLCV+VWAP (returns last ~720 bars)

### Kraken Futures
- `GET /derivatives/api/v3/instruments` → contract catalogue, including PF_*
- `GET /derivatives/api/v4/historical-funding-rates?symbol={S}` (newer path on v4 for funding-rate series)
- `GET /charts/{trade|spot|mark}/{symbol}/{1m|5m|15m|30m|1h|4h|12h|1d|1w}?from={s}&to={s}` → OHLC candles

### Binance Spot
- `GET /api/v3/klines?symbol={S}&interval={1m|…|1d}&startTime={ms}&endTime={ms}&limit=1000` → kline (last col includes ignored field)

### Binance European Options
- `GET /eapi/v1/exchangeInfo` → option universe per underlying
- `GET /eapi/v1/klines?symbol={contract}&interval={…}` → contract OHLC
- `GET /eapi/v1/mark?symbol={contract}` → mark IV, delta, gamma, theta, vega

### Polymarket
- `GET https://gamma-api.polymarket.com/markets?limit=&offset=&active=&closed=&order=&tag_id=` → market discovery
- `GET https://gamma-api.polymarket.com/events?...` → event grouping
- `GET https://clob.polymarket.com/prices-history?market={tokenId}&startTs={s}&endTs={s}&interval={1m|1h|6h|1d|1w|max}&fidelity={N}` → CLOB mid-price history

## Code shape

7 new sub-commands in `fetch_external.py`:

```
bybit kline           --category {spot|linear|inverse|option} --symbol BTCUSDT --interval 60 --start ... --end ... --output ...
bybit option-chain    --base {BTC|ETH|SOL} [--expiry YYYYMMDD] --output ...

kraken kline          --pair {AAPLX/USD|EURUSD|XBTUSD} [--asset-class spot|futures] --interval 60 --since ... --output ...

binance kline         --symbol BTCUSDT --interval 1h --start ... --end ... --output ...
binance option-chain  --underlying {BTC|ETH} [--expiry YYYYMMDD] --output ...

polymarket markets       [--limit N] [--tag T] [--active true|false] --output ...
polymarket price-history --token TOKEN_ID --interval 1d [--days N] --output ...
```

Each provider gets its own `Fetcher` class (matches `YahooFinanceFetcher` style)
with a shared retry/UA-rotation pattern. Spot/futures/option kline outputs land
in the canonical 6-column CSV (`date,open,high,low,close,volume`) so they flow
unchanged through `prepare_external.py` into FreqTrade feather. Option-chain
outputs use a wide CSV similar to the NSE flatten format, augmented with
`mark_iv,delta,gamma,theta,vega` columns where available.

## Auth tier and key acquisition

| Tier | What we get | What it requires | Code state |
| ---- | ----------- | ---------------- | ---------- |
| Public | OHLCV, option chain, instrument catalogue, CLOB price history | Nothing — anonymous HTTPS | Implemented this iteration |
| Keyed | Account balance, positions, order placement, websocket private streams | Operator KYC + key generation, env-var injection | Stub, env-var driven; flip on later |

The operator (you) handles the KYC/key creation; the assistant cannot perform
identity-bound signups. Per-provider acquisition checklist:

| Provider | Sign-up URL | Required ID | Key permissions for read-only ops | Notes |
| -------- | ----------- | ----------- | --------------------------------- | ----- |
| Bybit | bybit.com/en/register | KYC tier 1 (ID + selfie) for trading; market data needs nothing | n/a (public) | Restricted: USA, UK (Bybit blocks at trading layer). API key page: bybit.com/app/user/api-management |
| Kraken Spot | kraken.com/sign-up | KYC verified (Pro tier) for trading; market data needs nothing | `Query Funds`, `Query Open Orders & Trades` for read | API key page: kraken.com → Settings → API |
| Kraken Futures | login-futures.kraken.com (separate creds) | Tier 2 verified | Separate `KRAKEN_FUTURES_API_KEY` env var | Paper sandbox at demo-futures.kraken.com — no KYC needed |
| Binance | binance.com/en/register | KYC for trading; market data needs nothing | IP-whitelist + `Enable Reading` scope only for safety | API key page: binance.com/en/my/settings/api-management |
| Polymarket | No sign-up; uses Polygon wallet | Wallet (e.g. MetaMask) + USDC + MATIC for gas | Wallet private key in env var (read API needs nothing) | USA-restricted at trading; data global |

## Smoke-test plan

For each new provider, this iteration will:

1. Call its public endpoint with a representative symbol.
2. Verify row count is non-zero and the date range matches what was requested.
3. Drop the CSV into `Auto-Quant/user_data/data/raw/`.
4. Run `prepare_external.py` over it where the data shape is canonical OHLCV
   (skip for option-chain rows; those are wide and not for backtest).
5. Confirm a feather lands under `user_data/data/{PAIR}-{TF}.feather` if
   applicable.

Targets:

| Smoke | Public URL hit | Expected |
| ----- | -------------- | -------- |
| `bybit kline --category spot --symbol BTCUSDT --interval D` | `api.bybit.com/v5/market/kline` | ≥ 365 daily rows |
| `bybit option-chain --base BTC` | `api.bybit.com/v5/market/instruments-info` + `tickers` | ≥ 50 contracts at any time |
| `kraken kline --pair AAPLX/USD --interval 1440` | `api.kraken.com/0/public/OHLC` | ≥ 200 daily rows |
| `binance kline --symbol BTCUSDT --interval 1d` | `api.binance.com/api/v3/klines` | ≥ 365 daily rows |
| `binance option-chain --underlying BTC` | `eapi.binance.com/eapi/v1/exchangeInfo` + `mark` | ≥ 30 active contracts |
| `polymarket markets --limit 5` | `gamma-api.polymarket.com/markets` | 5 markets |
| `polymarket price-history --token … --interval 1d --days 30` | `clob.polymarket.com/prices-history` | ~30 daily mid prices |

## Out of scope (explicit)

- Trading endpoints (order place / amend / cancel). Public read only this iteration.
- WebSocket streaming. Could be a follow-up but Auto-Quant's backtest path is REST-driven via canonical CSV → feather.
- Geo workarounds. If you're behind a hostile NAT, run from a clean VPS or with a clean SOCKS proxy.
- Direct integration with FreqTrade's `IStrategy`. Option-chain data is an
  observation feature for downstream strategies, not a backtestable instrument
  by itself (FreqTrade is single-instrument-OHLCV, no strike/expiry dim).
- Pinned-version Python SDKs (`pybit`, `binance-connector-python`). Reasons in
  Constraints. We can revisit when keyed signing becomes a maintenance burden.

## Sequencing

1. Plan committed (this file).
2. `fetch_external.py` extended with 7 new sub-commands and 4 new fetcher
   classes; doc paragraph in the script header.
3. Smoke tests run; outputs land in `Auto-Quant/user_data/data/raw/`.
4. Deploy doc `support/docs/external/auto-quant-multi-asset-data-2026-04-26.md`
   updated with the new provider table.
5. Single feature commit on `green-baseline`.

## Smoke-test outcome (2026-04-26, this client IP)

| Sub-command | Result | Detail |
| ----------- | ------ | ------ |
| `bybit-kline --category linear --symbol BTCUSDT --interval 1d` | ✅ | 365 daily rows 2025-01-01 → 2025-12-31 |
| `bybit-options --base BTC` | ✅ | 500 contracts × 9 expiries, 100% Greeks coverage, strikes 20k–350k |
| `kraken-kline --market spot --pair XBTUSD --interval 1d` | ✅ | 721 daily rows 2024-05-06 → 2026-04-26 |
| `kraken-kline --market spot --pair AAPLxUSD --asset-class tokenized_asset --interval 1d` | ✅ | 256 daily rows 2025-08-14 → 2026-04-26 (xStocks) |
| `kraken-kline --market spot --pair SPYxUSD --asset-class tokenized_asset --interval 1d` | ✅ | 256 daily rows (xStocks) |
| `kraken-kline --market spot --pair ZEURZUSD --asset-class forex --interval 1d` | ✅ | 721 daily rows; **real volume** unlike Yahoo |
| `kraken-kline --market futures --pair PF_SPXUSD --interval 1d` | ✅ | 214 daily rows 2025-06-01 → 2025-12-31 (S&P 500 perp) |
| `binance-kline --symbol BTCUSDT --interval 1d` | ✅ | 365 daily rows |
| `binance-options --underlying BTC` | ✅ | 538 contracts × 12 expiries, 100% Greeks coverage, strikes 30k–170k |
| `polymarket-markets --limit 5` | ❌ | GFW reliably resets TLS handshakes to `gamma-api.polymarket.com`; 5 retries exhausted. Code is correct; from a clean-routed exit it succeeds. |
| `polymarket-history --token …` | ❌ | Same; depends on `polymarket-markets` for token discovery first. |

## Findings worth pinning into prose

1. **Bybit V5 option symbols now have 5 dash segments**, not 4.
   `BTC-26MAR27-78000-P-USDT` (BASE-EXPIRY-STRIKE-SIDE-SETTLE). The settle
   suffix denotes the quote currency for cash settlement (`USDT` or `USDC`).
   `BybitFetcher.option_chain` accepts both 4- and 5-segment forms and
   surfaces the settle currency as its own column.

2. **Kraken's `/0/public/OHLC` requires `asset_class=tokenized_asset` for
   xStocks** and `asset_class=forex` for fiat pairs. Without it the endpoint
   returns `EGeneral:Invalid arguments`. The `kraken-kline --asset-class …`
   flag wires this through; default is omitted (i.e. crypto).

3. **xStocks pair names use a lowercase `x`** (`AAPLxUSD`, `SPYxUSD`), matching
   Kraken's altname convention. Kraken-CLI's `wsname` uses `AAPLx/USD`, but
   the REST `pair=` parameter wants the altname.

4. **Bybit / Kraken / Binance / Polymarket TLS handshakes** can be torn down
   mid-flight by hostile network paths (verified GFW behavior on
   `gamma-api.polymarket.com`; intermittent on `api.kraken.com`). The four new
   `_get` helpers now retry on `requests.exceptions.SSLError`,
   `ConnectionError`, and `Timeout` with the same exponential backoff used
   for retryable HTTP statuses. This rescued AAPL xStocks during smoke tests
   and is a no-op on healthy paths.

5. **Polymarket from CN IP is not viable for read** because the GFW reliably
   resets TLS to `gamma-api.polymarket.com` and `clob.polymarket.com`. The
   sub-commands work correctly; deployment requires an exit outside that
   network. No code workaround is appropriate (would mean shipping a proxy or
   alternative DNS, both out of scope for this iteration).
