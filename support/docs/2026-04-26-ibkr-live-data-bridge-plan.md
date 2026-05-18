# IBKR live data bridge — single producer, multi-consumer

**Status**: planning, started 2026-04-26.
**Owner**: ict-engine repo (source of truth); deployed to Auto-Quant.
**Branch**: `green-baseline`.
**Companion**: `support/docs/2026-04-26-multi-exchange-data-source-integration-plan.md`
(prior session's bybit/kraken/binance/polymarket integration).

## Problem

Both Auto-Quant (Freqtrade fork, deploy target) and ict-engine (Rust/Python
research repo) need IBKR data. IBKR's TWS / IB Gateway is a single-login
session per account — so we cannot have N independent IBKR client processes
each holding their own login. **However**, a single TWS/Gateway accepts
multiple concurrent API client connections via distinct `clientId` values.

Constraints to honor:

1. **Rate limits** — IBKR enforces strict pacing (6s/contract historical,
   60-distinct-contracts / 10-min sliding window, 100 streaming market-data
   lines, ~50 outbound msg/sec). Cross-process coordination is required:
   Auto-Quant's freqtrade run + ict-engine's research script must not stack
   their token buckets independently.
2. **Open-source 0-config** — When ict-engine becomes public, fresh clones
   must work fully without IBKR. All IBKR features are opt-in.
3. **First-run consent + privacy disclaimer** — On first IBKR usage, the
   tool must (a) inform the user IBKR is optional, (b) state plainly what
   data leaves the machine (none), (c) ask for explicit opt-in, (d) persist
   that consent locally. No telemetry, no surprise.
4. **Auto-detect account capabilities** — every user's IBKR account is
   different (Pro vs Lite, Paper vs Live, single vs FAdvisor). No hardcoded
   defaults; identify statically + adapt at runtime.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  TWS / IB Gateway (single login session)            │
│   paper: localhost:7497   live: localhost:7496      │
└─────────────────────────────────────────────────────┘
                    ↑ 1 socket
                    │ clientId=20 (configurable)
┌─────────────────────────────────────────────────────┐
│  ibkr-bridge.py  (always-on producer)               │
│   • AccountProber  — reqManagedAccts (1-shot)       │
│   • RateLimiter    — Redis-backed, adaptive         │
│   • Subscriber     — priority watchlist             │
│   • Publisher      — XADD ticks/bars to Redis       │
└─────────────────────────────────────────────────────┘
                    ↓ Redis Streams + Hash
┌─────────────────────────────────────────────────────┐
│  Redis (localhost:6379, brew install redis)         │
│   • Streams ibkr:bars:{sym}:{bar}                   │
│   • Streams ibkr:ticks:{sym}                        │
│   • Hash    ibkr:snapshot:{sym}                     │
│   • ZSet    ibkr:rl:hist:global   (60/10min)        │
│   • String  ibkr:rl:hist:{conid}:{bar}  (6s lock)   │
│   • Counter ibkr:rl:lines:current                   │
└─────────────────────────────────────────────────────┘
       ↓ XREAD                       ↓ XREAD/HGETALL
┌──────────────────────────┐  ┌──────────────────────────┐
│  Auto-Quant strategy     │  │  ict-engine research     │
│  IbkrConsumer (read-only)│  │  IbkrConsumer (read-only)│
└──────────────────────────┘  └──────────────────────────┘
       ↑                                   ↑
       │  fetch_external.py ibkr-historical                 │
       │  short-lived connect, clientId=21+, reuses limiter │
       └────────────────────────────────────────────────────┘
```

The bridge owns the only persistent IBKR connection. Everything downstream
reads from Redis. Historical backfills are short-lived independent
connections that share the same Redis-backed RateLimiter so the account-level
pacing budget is correctly accounted across all sources.

## Tech selections

| Component | Choice | Reason | Alt considered |
| --- | --- | --- | --- |
| IBKR client lib | `ib_async` (PyPI, pkg `ib_async`) | maintained fork of archived `ib_insync`; async-native; auto-reconnect; widely used | raw `ibapi` (verbose), `ib_insync` (archived) |
| Message layer | Redis Streams | macOS one-line install; fan-out + durable backlog; cross-language clients; `MAXLEN ~10000` memory bound; doubles as rate-limit coordinator | NATS (extra server), plain TCP (fan-out reinvented), DuckDB watch (live latency poor) |
| Python Redis client | `redis-py` (>=5.0) | official, sync + async API, no extra deps | aioredis (merged into redis-py) |
| Config format | YAML via `pyyaml` | already in venv | TOML, env vars |
| Consumer API | importable module `ibkr_consumer` | single-file import; `from ibkr_consumer import IbkrConsumer`; works both in Freqtrade strategies and ict-engine notebooks | HTTP API (extra hop), CLI-only |

## Rate limit table (canonical IBKR pacing as of 2026)

### A. Historical bars (`reqHistoricalData`)

| Rule | Value | Error | Implementation |
| --- | --- | --- | --- |
| Same `(contract, bar_size, what_to_show)` interval | ≥ 6 sec | 162 | per-`(conid, bar, what)` token bucket, default 6.5s |
| 60 distinct contracts in 10-min sliding window | 60 reqs | 162 | Redis ZSET, ZADD timestamp, ZCOUNT current window |
| Simultaneous open historical reqs | ≤ 50 | — | semaphore |
| Identical hist req within 15s | violation | 162 | local CSV cache: skip remote on hit |

### B. Streaming market data (`reqMktData`)

| Rule | Value | Implementation |
| --- | --- | --- |
| Account streaming lines | default 100, scales with subscriptions | `max_lines: 80` default (20-line buffer); priority-ordered subscription queue |
| `reqRealTimeBars` | also counts as 1 line each | folded into same line accountant |

### C. Tick-by-tick (`reqTickByTickData`)

| Rule | Value | Implementation |
| --- | --- | --- |
| Simultaneous tick-by-tick | ≤ 5 (strict) | disabled by default; explicit opt-in per-symbol in config |

### D. Snapshot data

| Rule | Value | Implementation |
| --- | --- | --- |
| Per-contract snapshot | ≥ 11 sec | per-contract token bucket |
| Total snapshot rate | ~1/sec global | global token bucket |

### E. Account-wide

| Rule | Value | Implementation |
| --- | --- | --- |
| Outbound msg/sec to TWS | ~50 | 50ms minimum sender interval |
| Connection reset backoff | exponential | 5/15/45/120s reconnect ladder |

## Auto-detect strategy (zero hardcoded defaults)

### Static identification (1 RTT, on connect)

| Dimension | Source | Persisted |
| --- | --- | --- |
| Paper vs Live | port (7497/7496) + `reqManagedAccts` ID prefix (`DU*`=paper, `U*`=Pro, `DF*`/`F*`=FAdvisor, `I*`=Institutional) | `account_type` |
| Single vs FAdvisor | length of `reqManagedAccts` reply | `n_subaccounts` |
| Live vs delayed feed | `marketDataType` field on first `reqMktData` reply (1=live, 2=frozen, 3=delayed, 4=delayed-frozen) | `feeds_observed_delayed` per symbol |

### Runtime adaptation (learn from observed errors)

| Signal | Meaning | RateLimiter response |
| --- | --- | --- |
| Pacing violation 162 | hist requests too dense | +30s backoff; tighten `hist_min_interval` 6.5→8→10s; relax after 24h clean |
| Errors 354/322/1100 | streaming lines exhausted | record observed ceiling N-1 to capabilities; never exceed |
| Connection reset by TWS | outbound msg rate too high | drop sender to 30/sec, slow ramp back |
| `marketDataType=3` | no live subscription for that instrument | flag delayed, continue |
| 48h zero violations | safe headroom | relax 5% (historical only; lines never speculatively retried) |

### Persistence — `~/.ict-engine/ibkr_capabilities.json`

```json
{
  "version": 1,
  "first_seen": "2026-04-26T11:00:00+08:00",
  "last_updated": "2026-04-26T15:30:00+08:00",
  "account_type": "paper",
  "n_subaccounts": 1,
  "measured_max_lines": 92,
  "lines_safety_buffer": 0.85,
  "hist_min_interval_sec": 6.5,
  "hist_window_capacity": 60,
  "msg_outbound_rate": 50,
  "feeds_observed_delayed": ["AAPL"],
  "violations_24h": []
}
```

Initial defaults (no file present): conservative — `max_lines=80`,
`hist_min_interval=6.5s`, `hist_window=60/10min`, `msg_rate=50/sec`. **No
proactive probe at startup** — we do not waste user pacing budget. All
limits learned passively from real traffic.

## Consent flow (first-run UX)

### Trigger points

Any of the three IBKR-touching entries calls `require_ibkr_enabled()`:

1. `python support/scripts/ibkr_bridge/bridge.py ...`
2. `python support/scripts/auto_quant_external/fetch_external.py ibkr-historical ...`
3. `from ibkr_consumer import IbkrConsumer` — first call to a method that
   makes IBKR-bound traffic. (Pure read of an already-populated Redis is
   *not* gated; consumer-only users never see the prompt.)

### Gating logic

```python
def require_ibkr_enabled() -> None:
    consent = Path.home() / ".ict-engine" / "ibkr_consent.json"
    if consent.exists() and json.loads(consent.read_text())["opted_in"]:
        return
    if not sys.stdin.isatty():
        sys.exit(
            "IBKR not enabled. To enable interactively run:\n"
            "    python support/scripts/ibkr_bridge/setup.py --enable"
        )
    show_disclaimer_and_prompt()
```

### Bilingual disclaimer (verbatim)

```
IBKR live data — privacy & connectivity notice
──────────────────────────────────────────────
This feature reads real-time data from your *locally running* IB Gateway
or TWS application. Everything stays on this machine.

What this code DOES:
  • Connect to localhost:7497 (paper) or :7496 (live) — your local IB Gateway
  • Subscribe to instruments listed in ibkr_bridge/<your>_config.yaml
  • Write market data to your local Redis (localhost:6379)
  • Honor IBKR's pacing rules (6s/contract historical, 100 streaming lines)
  • Learn your account capabilities passively from observed errors;
    no proactive probing of your data quota at startup

What this code DOES NOT:
  • Send your IBKR credentials anywhere — they stay in IB Gateway/TWS
  • Contact ict-engine.com, OpenAlice, or any third-party server
  • Place orders, close positions, or modify your IBKR account state
  • Collect telemetry, analytics, or crash reports

Auditable source:  support/scripts/ibkr_bridge/{bridge,consumer,rate_limiter}.py
Capabilities file: ~/.ict-engine/ibkr_capabilities.json (gitignored, local)
Revoke any time:   rm ~/.ict-engine/ibkr_consent.json

═════════════════════════════════════════════════════════════════════
中文版
═════════════════════════════════════════════════════════════════════

IBKR 实时数据 — 隐私与连接说明
──────────────────────────────────
本功能从你**本机运行**的 IB Gateway 或 TWS 读取实时数据。所有内容均不离开本机。

本代码会做：
  • 连接 localhost:7497 (paper) 或 :7496 (live) — 你本地的 IB Gateway
  • 订阅 ibkr_bridge/<你的>_config.yaml 列出的合约
  • 写入你本地的 Redis (localhost:6379)
  • 遵守 IBKR 流控规则 (6 秒/合约 历史限制，100 条流式数据线)
  • 通过观察实际错误被动学习账户能力；启动时不做主动配额探测

本代码绝不会：
  • 把你的 IBKR 凭据传到任何地方 — 凭据一直在 IB Gateway/TWS 里
  • 联系 ict-engine.com、OpenAlice 或任何第三方服务
  • 下单、平仓或修改你的 IBKR 账户状态
  • 收集遥测、分析或崩溃报告

源代码可审：    support/scripts/ibkr_bridge/{bridge,consumer,rate_limiter}.py
能力文件：       ~/.ict-engine/ibkr_capabilities.json (gitignored, 本地)
随时撤回同意：   rm ~/.ict-engine/ibkr_consent.json

Do you understand and want to enable IBKR live data? 是否理解并启用 IBKR 实时数据？ [y/N]:
```

### Persisted consent — `~/.ict-engine/ibkr_consent.json`

```json
{
  "opted_in": true,
  "timestamp": "2026-04-26T11:00:00+08:00",
  "version": 1
}
```

## File layout

```
ict-engine/
├── support/docs/
│   ├── 2026-04-26-ibkr-live-data-bridge-plan.md         (this file)
│   └── external/auto-quant-multi-asset-data-2026-04-26.md  (+ IBKR section)
└── support/scripts/
    ├── auto_quant_external/
    │   └── fetch_external.py                            (+ ibkr-historical)
    └── ibkr_bridge/
        ├── __init__.py
        ├── setup.py              (consent + Redis ping + Gateway ping)
        ├── rate_limiter.py       (Redis-backed adaptive limiter)
        ├── account_prober.py     (one-time static identification)
        ├── bridge.py             (producer main loop)
        ├── consumer.py           (importable IbkrConsumer)
        ├── example_config.yaml   (subscriptions: [] empty default)
        └── README.md             (IBKR-optional callout, bilingual disclaimer)
```

User-local files (gitignored, **not** in repo):

```
~/.ict-engine/
├── ibkr_consent.json
└── ibkr_capabilities.json
```

## Dependencies

Python (Auto-Quant venv `~/Auto-Quant/.venv`):

```
ib_async>=1.0.5
redis>=5.0
pyyaml          # already present
pandas, requests, pyarrow   # already present
```

System (macOS):

```
brew install redis
brew services start redis
```

User-supplied (NOT installed by us, IP-licensed):

```
IB Gateway or TWS (Java app) — user downloads from interactivebrokers.com
Logged-in account (paper free, live with credentials)
```

## OSS user experience guarantees

**Verbatim text destined for `support/scripts/ibkr_bridge/README.md` top section**:

> ### ⚠️ IBKR is optional and disabled by default
>
> ict-engine works completely without IBKR. All free providers (Yahoo
> Finance, Kraken, Bybit, Binance) plus paid optional ones (Polygon, NSE)
> function on a fresh clone with zero IBKR setup.
>
> Enable IBKR only if you:
> 1. Have an IBKR account
> 2. Are running IB Gateway or TWS locally
> 3. Want real-time or historical data from IBKR feeds
>
> When enabled, all IBKR I/O is **localhost-only**. No data, credentials,
> or telemetry leaves your machine. See bilingual disclaimer below.

## Smoke-test plan

User actions required (one-time):

1. Install IB Gateway from interactivebrokers.com.
2. Log into a paper account. Enable API on port 7497, allow
   localhost. Set "Read-only API" toggle on.
3. `brew install redis && brew services start redis`.

Cascade actions:

1. `pip install ib_async redis` into Auto-Quant venv.
2. Run `python support/scripts/ibkr_bridge/setup.py --enable`. Verify:
   - Bilingual disclaimer renders.
   - `y` produces `~/.ict-engine/ibkr_consent.json`.
   - Redis ping success.
   - Gateway ping success (clientId=99 throwaway test connection).
3. Run `python support/scripts/ibkr_bridge/bridge.py --config example_config.yaml`
   with `subscriptions: [AAPL, SPY]`. Verify in second terminal:
   - `redis-cli XLEN ibkr:bars:AAPL:5sec` increases over time.
   - `redis-cli HGETALL ibkr:snapshot:AAPL` returns last quote.
4. From two separate Python processes, instantiate `IbkrConsumer`, call
   `c.bars("AAPL", "5 sec", lookback=20)` — both should return identical
   data (fan-out check).
5. Force a 162 violation: rapid-fire 70 distinct historical reqs in 5
   minutes via `fetch_external.py ibkr-historical`. Verify:
   - RateLimiter intercepts before TWS.
   - On any actual 162 (if leak), capabilities.json shows
     `hist_min_interval_sec` increased.
6. Restart bridge — capabilities.json is read, limits respected immediately.

## Sequencing

1. Plan committed (this file).
2. Deps installed (`pip install` + `brew install redis`).
3. Skeleton: `rate_limiter.py` → `account_prober.py` → `setup.py`.
4. `bridge.py` (uses skeleton).
5. `consumer.py`.
6. Config + README.
7. `fetch_external.py ibkr-historical` (reuses `rate_limiter.py`).
8. Update deploy doc.
9. Single feature commit on `green-baseline`.
10. User-driven smoke verification.

## Risks / open questions

1. **macOS Redis crash** during a run → bridge buffers in memory until
   reconnect; if bridge also crashes, latest 10s of ticks lost. v1 may add
   SQLite spool. Out of scope for v0.
2. **IB Gateway auto-logout** at 23:59 ET (paper) / weekly (live) → bridge
   must detect disconnect and either auto-reconnect (after user re-logins)
   or alert. v0: log clearly + exit code 4 so launchd / systemd can restart.
3. **`ib_async` is a community fork**, not officially endorsed by IBKR.
   Risk of upstream API changes. Pinned to `>=1.0.5,<2.0`.
4. **Redis exposure**: default `bind 127.0.0.1` only — `setup.py` verifies
   this at startup and refuses to launch bridge if Redis is bound to 0.0.0.0
   without `requirepass`.
5. **Pacing-violation 162 latency**: detection happens after the violation,
   so we can leak 1-2 reqs per spike. RateLimiter's pre-check prevents the
   bulk; adaptive `hist_min_interval` widens after each leak.
