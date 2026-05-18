"""IBKR live data bridge — single producer, Redis Streams fan-out.

Connects to TWS / IB Gateway via ``ib_async``, subscribes to instruments
listed in a YAML config, and publishes every bar / tick / snapshot to local
Redis as Streams + Hash entries. Multiple consumers (Auto-Quant strategies,
ict-engine research scripts, ad-hoc notebooks) read from Redis without
touching IBKR directly.

Redis key layout
----------------

    ibkr:bars:{symbol}:5sec        Stream  — 5-second OHLCV bars
    ibkr:ticks:{symbol}            Stream  — bid/ask/last quote ticks
    ibkr:snapshot:{symbol}         Hash    — latest known values
    ibkr:bridge:status             Hash    — bridge liveness for consumers

Consumers should call ``XREAD`` with cursor IDs for streams and ``HGETALL``
for snapshots. See ``consumer.py`` for the canonical helper.

The bridge respects ``IbkrRateLimiter`` for every IBKR API call (so that
``fetch_external.py ibkr-historical`` running in parallel cannot accidentally
exceed the account budget).
"""

from __future__ import annotations

import argparse
import asyncio
import contextlib
import json
import math
import signal
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import redis
import yaml
from ib_async import (
    IB,
    Contract,
    Crypto,
    Forex,
    Future,
    Index,
    Option,
    RealTimeBar,
    Stock,
)

from .account_prober import probe_account
from .client_id import connect_with_client_id_fallback
from .consent import require_ibkr_enabled
from .ibkr_errors import format_for_log, is_info
from .rate_limiter import (
    CAPABILITIES_PATH,
    IbkrCapabilities,
    IbkrRateLimiter,
)

DEFAULT_REDIS_URL = "redis://localhost:6379"
DEFAULT_GATEWAY_HOST = "127.0.0.1"
DEFAULT_GATEWAY_PORT = 7497
DEFAULT_BRIDGE_CLIENT_ID = 20
DEFAULT_STREAM_MAXLEN = 10000

RECONNECT_BACKOFF_S = [5, 15, 45, 120]


# ---------------------------------------------------------------------------
# Config


@dataclass
class SubscriptionSpec:
    symbol: str
    sec_type: str = "STK"
    exchange: str = "SMART"
    currency: str = "USD"
    feed: list[str] = field(default_factory=lambda: ["real_time_bars"])
    # Optional contract qualifiers
    last_trade_date: str | None = None      # FUT, OPT
    strike: float | None = None             # OPT
    right: str | None = None                # OPT 'C'/'P'
    multiplier: str | None = None           # FUT, OPT
    primary_exchange: str | None = None     # STK with disambiguation
    # bars_kup feed (reqHistoricalData keepUpToDate=True). Works at any
    # bar size and does NOT require a live MD subscription — it uses the
    # historical-data plumbing which IBKR makes more permissive than
    # live tick streams. Ideal for 5-min intraday factor research.
    bar_size: str = "5 mins"                # IBKR-formatted, see docs
    what_to_show: str = "TRADES"            # TRADES | MIDPOINT | BID | ASK
    duration: str = "1 D"                   # initial backfill window
    use_rth: bool = False                   # filter to Regular Trading Hours


@dataclass
class GatewayConfig:
    host: str = DEFAULT_GATEWAY_HOST
    port: int = DEFAULT_GATEWAY_PORT
    client_id: int = DEFAULT_BRIDGE_CLIENT_ID
    # IBKR market-data type:
    #   1 = live   (requires active MD subscription on the live account)
    #   2 = frozen (last known live values when market closed)
    #   3 = delayed   (~15 min delay; free for most accounts; great fallback)
    #   4 = delayed-frozen
    # Use 3 for paper-account smoke tests where the live MD subscription is
    # absent or in conflict with another logged-in session.
    market_data_type: int = 1


@dataclass
class RedisConfig:
    url: str = DEFAULT_REDIS_URL


@dataclass
class PublishingConfig:
    stream_maxlen: int = DEFAULT_STREAM_MAXLEN
    snapshot_ttl_sec: int = 0  # 0 = no expire


@dataclass
class BridgeConfig:
    gateway: GatewayConfig = field(default_factory=GatewayConfig)
    redis: RedisConfig = field(default_factory=RedisConfig)
    publishing: PublishingConfig = field(default_factory=PublishingConfig)
    subscriptions: list[SubscriptionSpec] = field(default_factory=list)

    @classmethod
    def from_yaml(cls, path: Path) -> "BridgeConfig":
        if not path.exists():
            raise FileNotFoundError(f"bridge config not found at {path}")
        raw = yaml.safe_load(path.read_text()) or {}
        gw = GatewayConfig(**(raw.get("gateway") or {}))
        rd = RedisConfig(**(raw.get("redis") or {}))
        pb = PublishingConfig(**(raw.get("publishing") or {}))
        subs = [SubscriptionSpec(**s) for s in (raw.get("subscriptions") or [])]
        return cls(gateway=gw, redis=rd, publishing=pb, subscriptions=subs)


# ---------------------------------------------------------------------------
# Contract construction


def _build_contract(spec: SubscriptionSpec) -> Contract:
    """Translate a SubscriptionSpec into an ib_async Contract."""
    sec = spec.sec_type.upper()
    if sec == "STK":
        c = Stock(spec.symbol, spec.exchange, spec.currency)
        if spec.primary_exchange:
            c.primaryExchange = spec.primary_exchange
        return c
    if sec == "CASH":
        return Forex(spec.symbol)  # 'EURUSD' style
    if sec == "FUT":
        return Future(spec.symbol, spec.last_trade_date or "",
                       spec.exchange, currency=spec.currency,
                       multiplier=spec.multiplier or "")
    if sec == "IND":
        return Index(spec.symbol, spec.exchange, spec.currency)
    if sec == "CRYPTO":
        # IBKR-PAXOS crypto: BTC, ETH, LTC, BCH on exchange='PAXOS' currency='USD'
        return Crypto(spec.symbol, spec.exchange or "PAXOS",
                       spec.currency or "USD")
    if sec == "OPT":
        if spec.last_trade_date is None or spec.strike is None or spec.right is None:
            raise ValueError(
                f"OPT subscription for {spec.symbol!r} needs "
                "last_trade_date, strike, and right"
            )
        return Option(spec.symbol, spec.last_trade_date, spec.strike,
                       spec.right, spec.exchange, currency=spec.currency,
                       multiplier=spec.multiplier or "100")
    raise ValueError(f"unsupported sec_type {spec.sec_type!r} for {spec.symbol!r}")


_BAR_SIZE_SUFFIX = {
    "1 secs": "1sec", "5 secs": "5sec", "10 secs": "10sec",
    "15 secs": "15sec", "30 secs": "30sec",
    "1 min": "1min", "2 mins": "2min", "3 mins": "3min",
    "5 mins": "5min", "10 mins": "10min", "15 mins": "15min",
    "20 mins": "20min", "30 mins": "30min",
    "1 hour": "1h", "2 hours": "2h", "3 hours": "3h",
    "4 hours": "4h", "8 hours": "8h",
    "1 day": "1d", "1W": "1w", "1M": "1mo",
}


def _bar_size_to_key_suffix(bar_size: str) -> str:
    """Translate an IBKR bar-size string to a Redis-friendly key suffix.

    Falls back to a sanitised version of the original string for any size
    that is not in the canonical IBKR table.
    """
    suffix = _BAR_SIZE_SUFFIX.get(bar_size)
    if suffix:
        return suffix
    return bar_size.lower().replace(" ", "")


# ---------------------------------------------------------------------------
# Bridge


class IbkrBridge:
    """Owns the persistent IBKR connection and the Redis publisher.

    Lifecycle:
        b = IbkrBridge(config)
        await b.start()       # connects, subscribes, runs until cancelled
        await b.stop()        # graceful disconnect, line release, save caps
    """

    def __init__(self, config: BridgeConfig,
                 capabilities_path: Path = CAPABILITIES_PATH) -> None:
        self.config = config
        self.ib = IB()
        self._capabilities_path = capabilities_path
        self._redis = redis.Redis.from_url(config.redis.url, decode_responses=True)
        self._limiter = IbkrRateLimiter(redis_url=config.redis.url,
                                          capabilities_path=capabilities_path)
        self._active_subscriptions: dict[str, dict[str, Any]] = {}
        self._stop_event = asyncio.Event()
        self._selected_client_id: int | None = None
        self._attempted_client_id_conflicts: list[int] = []

        # Wire IBKR error -> rate-limiter feedback
        self.ib.errorEvent += self._on_ib_error
        self.ib.disconnectedEvent += self._on_disconnected

    # ----- Public lifecycle ----------------------------------------------

    async def start(self) -> None:
        """Run forever: connect, subscribe, pump events, reconnect on drop."""
        self._publish_status("starting")
        attempt = 0
        while not self._stop_event.is_set():
            disconnect_was_clean = False
            try:
                await self._connect_and_subscribe()
                attempt = 0
                self._publish_status("running")
                await self._run_until_disconnect()
                # _run_until_disconnect() returns cleanly when IBKR dropped
                # the socket — treat as a transient failure and reconnect.
                disconnect_was_clean = True
            except asyncio.CancelledError:
                break
            except Exception as exc:  # noqa: BLE001
                # ib_async raises a wide variety of types (ApiException,
                # HandshakeError, asyncio.IncompleteReadError, etc.). Catch
                # them all so the supervisor never dies — the user can SIGINT.
                self._log(f"connect/subscribe error: {type(exc).__name__}: {exc}")

            # Reset per-connection state so the next connect/subscribe
            # cycle starts from a clean slate (don't keep stale ib handles).
            self._active_subscriptions.clear()
            try:
                self._limiter.reset_streaming_lines()
            except Exception:  # noqa: BLE001
                pass

            wait = RECONNECT_BACKOFF_S[min(attempt, len(RECONNECT_BACKOFF_S) - 1)]
            attempt += 1
            reason = "IBKR dropped socket" if disconnect_was_clean else "exception"
            self._publish_status(f"reconnecting in {wait}s ({reason})")
            self._log(f"reconnect in {wait}s ({reason}; attempt {attempt})")
            try:
                await asyncio.wait_for(self._stop_event.wait(), timeout=wait)
            except asyncio.TimeoutError:
                pass

    async def stop(self) -> None:
        """Graceful shutdown — release lines, disconnect, save caps."""
        self._stop_event.set()
        self._publish_status("stopping")
        for symbol, sub in list(self._active_subscriptions.items()):
            try:
                self._cancel_subscription(symbol, sub)
            except Exception as exc:  # noqa: BLE001
                self._log(f"unsubscribe {symbol} failed: {exc}")
        self._limiter.reset_streaming_lines()
        if self.ib.isConnected():
            self.ib.disconnect()
        self._limiter.caps.save(self._capabilities_path)
        self._publish_status("stopped")

    # ----- Connection + subscription -------------------------------------

    async def _connect_and_subscribe(self) -> None:
        cfg = self.config
        self._log(f"connecting to {cfg.gateway.host}:{cfg.gateway.port} "
                  f"as clientId={cfg.gateway.client_id} "
                  f"market_data_type={cfg.gateway.market_data_type}")
        await self._limiter.wait_for_outbound_msg()
        selected_client_id, attempted_conflicts = await connect_with_client_id_fallback(
            self.ib,
            host=cfg.gateway.host,
            port=cfg.gateway.port,
            preferred_client_id=cfg.gateway.client_id,
            readonly=True,
        )
        self._selected_client_id = selected_client_id
        self._attempted_client_id_conflicts = [
            client_id for client_id, _ in attempted_conflicts
        ]
        if attempted_conflicts:
            self._log(
                "clientId fallback engaged: "
                + "; ".join(
                    f"{client_id} conflicted" for client_id, _ in attempted_conflicts
                )
                + f"; selected clientId={selected_client_id}"
            )
        elif selected_client_id != cfg.gateway.client_id:
            self._log(f"selected alternate clientId={selected_client_id}")

        # Set the market-data type early so every subsequent reqMktData /
        # reqRealTimeBars on this connection inherits the choice. IBKR will
        # transparently substitute frozen / delayed quotes when the requested
        # contract is not on a live subscription.
        if cfg.gateway.market_data_type != 1:
            await self._limiter.wait_for_outbound_msg()
            self.ib.reqMarketDataType(cfg.gateway.market_data_type)

        # First-connect probe if capabilities are unknown
        if self._limiter.caps.account_type == "unknown":
            try:
                managed = list(self.ib.managedAccounts() or [])
                if not managed:
                    await asyncio.sleep(1.0)
                    managed = list(self.ib.managedAccounts() or [])
                if managed:
                    from .account_prober import _classify
                    self._limiter.caps.account_type = _classify(managed, cfg.gateway.port)
                    self._limiter.caps.n_subaccounts = len(managed)
                    self._limiter.caps.save(self._capabilities_path)
                    self._log(f"detected account_type={self._limiter.caps.account_type} "
                              f"n_subaccounts={self._limiter.caps.n_subaccounts}")
            except Exception as exc:  # noqa: BLE001
                self._log(f"managed-accounts probe skipped: {exc}")

        if not cfg.subscriptions:
            self._log("no subscriptions configured (idle bridge)")
            return

        # Subscribe in priority order (config order), respecting line cap
        for spec in cfg.subscriptions:
            if not self._limiter.acquire_streaming_line(spec.symbol):
                self._log(f"line cap reached; skipping {spec.symbol}")
                continue
            try:
                await self._subscribe_one(spec)
            except Exception as exc:  # noqa: BLE001
                self._limiter.release_streaming_line(spec.symbol)
                self._log(f"subscribe {spec.symbol} failed: {exc}")

    async def _subscribe_one(self, spec: SubscriptionSpec) -> None:
        contract = _build_contract(spec)
        await self._limiter.wait_for_outbound_msg()
        qualified = await self.ib.qualifyContractsAsync(contract)
        if not qualified:
            raise RuntimeError(f"contract not resolved: {spec.symbol}")
        contract = qualified[0]

        active: dict[str, Any] = {"contract": contract, "feeds": {}}
        feeds = [f.lower() for f in (spec.feed or ["real_time_bars"])]

        if "real_time_bars" in feeds:
            await self._limiter.wait_for_outbound_msg()
            bars = self.ib.reqRealTimeBars(contract, 5, "TRADES", useRTH=False)
            bars.updateEvent += self._make_bar_handler(spec.symbol)
            active["feeds"]["real_time_bars"] = bars

        if "market_data" in feeds:
            await self._limiter.wait_for_outbound_msg()
            ticker = self.ib.reqMktData(contract, "", snapshot=False,
                                          regulatorySnapshot=False)
            ticker.updateEvent += self._make_tick_handler(spec.symbol)
            active["feeds"]["market_data"] = ticker

        if "bars_kup" in feeds:
            await self._subscribe_bars_kup(spec, contract, active)

        self._active_subscriptions[spec.symbol] = active
        self._log(f"subscribed {spec.symbol} feeds={list(active['feeds'])}")

    async def _subscribe_bars_kup(self, spec: SubscriptionSpec,
                                    contract: Contract,
                                    active: dict[str, Any]) -> None:
        """Request keepUpToDate historical bars and stream updates to Redis.

        keepUpToDate=True is IBKR's most permissive way to obtain N-minute
        bars: it back-fills the requested duration AND keeps the most-recent
        bar updated as new ticks arrive. Unlike reqRealTimeBars (which is
        locked to 5 s and demands a live MD subscription), this works at any
        bar size and uses the historical-data farm.
        """
        bar_size = spec.bar_size or "5 mins"
        what = (spec.what_to_show or "TRADES").upper()
        duration = spec.duration or "1 D"
        if what not in ("TRADES", "MIDPOINT", "BID", "ASK"):
            raise ValueError(
                f"bars_kup whatToShow must be one of TRADES/MIDPOINT/BID/ASK; "
                f"got {what!r} for {spec.symbol!r}"
            )
        suffix = _bar_size_to_key_suffix(bar_size)

        # Historical-side rate-limit gating (initial backfill counts toward
        # the 60-req/10-min budget; the streaming half does not).
        await self._limiter.wait_for_historical(contract.conId, bar_size, what)
        await self._limiter.acquire_historical_slot()
        try:
            await self._limiter.wait_for_outbound_msg()
            bars = await self.ib.reqHistoricalDataAsync(
                contract,
                endDateTime="",
                durationStr=duration,
                barSizeSetting=bar_size,
                whatToShow=what,
                useRTH=bool(spec.use_rth),
                formatDate=2,
                keepUpToDate=True,
            )
        finally:
            self._limiter.release_historical_slot()

        # Backlog publish — emit every bar that came back in the initial
        # response so consumers replaying from start_id='0' get the full
        # backtest dataset shape on first connection. We pass batch=True
        # to skip per-bar snapshot updates during the bulk write.
        for bar in bars or []:
            self._publish_kup_bar(spec.symbol, suffix, bar, batch=True)

        # Final snapshot from the latest backlog bar so consumers see a
        # populated `ibkr:snapshot:<SYM>` immediately after subscribe.
        if bars:
            last_bar = bars[-1]
            ts_val = _bar_ts(last_bar.date)
            self._update_snapshot(spec.symbol, {
                "last_bar_close": str(last_bar.close),
                "last_bar_ts": f"{ts_val:.3f}",
                "bar_size": bar_size,
                "market_data_type": str(self.config.gateway.market_data_type),
            })

        # Live stream side — every subsequent bar update lands here.
        bars.updateEvent += self._make_kup_bar_handler(spec.symbol, suffix)
        active["feeds"]["bars_kup"] = {"bars": bars, "suffix": suffix,
                                          "bar_size": bar_size,
                                          "what_to_show": what}
        self._log(f"  bars_kup {spec.symbol} {bar_size} {what} "
                  f"backfill={len(bars or [])} bars -> ibkr:bars:{spec.symbol}:{suffix}")

    def _cancel_subscription(self, symbol: str, sub: dict[str, Any]) -> None:
        feeds = sub.get("feeds", {})
        bars = feeds.get("real_time_bars")
        if bars is not None:
            with contextlib.suppress(Exception):
                self.ib.cancelRealTimeBars(bars)
        ticker = feeds.get("market_data")
        if ticker is not None and "contract" in sub:
            with contextlib.suppress(Exception):
                self.ib.cancelMktData(sub["contract"])
        kup = feeds.get("bars_kup")
        if kup is not None:
            with contextlib.suppress(Exception):
                self.ib.cancelHistoricalData(kup["bars"])
        self._limiter.release_streaming_line(symbol)
        self._active_subscriptions.pop(symbol, None)

    async def _run_until_disconnect(self) -> None:
        """Pump events until either user signals stop or IBKR disconnects."""
        while self.ib.isConnected() and not self._stop_event.is_set():
            try:
                await asyncio.wait_for(self._stop_event.wait(), timeout=1.0)
            except asyncio.TimeoutError:
                pass

    # ----- Event handlers (publish to Redis) -----------------------------

    def _make_bar_handler(self, symbol: str):
        publish = self._publish_bar
        def _handler(bars, hasNewBar) -> None:  # noqa: ANN001
            if not hasNewBar or not bars:
                return
            bar = bars[-1]
            publish(symbol, bar)
        return _handler

    def _publish_bar(self, symbol: str, bar: RealTimeBar) -> None:
        ts = bar.time
        if hasattr(ts, "timestamp"):
            ts_val = ts.timestamp()
        else:
            ts_val = float(ts)
        fields = {
            "ts": f"{ts_val:.3f}",
            "open": str(bar.open_),
            "high": str(bar.high),
            "low": str(bar.low),
            "close": str(bar.close),
            "volume": str(bar.volume),
            "wap": str(getattr(bar, "wap", "")),
            "count": str(getattr(bar, "count", "")),
        }
        key = f"ibkr:bars:{symbol}:5sec"
        try:
            self._redis.xadd(key, fields,
                             maxlen=self.config.publishing.stream_maxlen,
                             approximate=True)
            self._update_snapshot(symbol, {"last_bar_close": fields["close"],
                                            "last_bar_ts": fields["ts"]})
        except redis.exceptions.RedisError as exc:
            self._log(f"redis xadd bars/{symbol} failed: {exc}")

    def _make_kup_bar_handler(self, symbol: str, suffix: str):
        publish = self._publish_kup_bar
        def _handler(bars, hasNewBar) -> None:  # noqa: ANN001
            # ib_async fires updateEvent for *every* tick rebuild of the
            # currently-forming bar. We always publish the LAST bar so the
            # consumer sees the most recent OHLCV state. hasNewBar=True
            # signals the bar boundary rolled (previous bar finalised);
            # both transitions are interesting for downstream consumers.
            if not bars:
                return
            publish(symbol, suffix, bars[-1], batch=False)
        return _handler

    def _publish_kup_bar(self, symbol: str, suffix: str,
                          bar, batch: bool = False) -> None:  # noqa: ANN001
        ts_val = _bar_ts(getattr(bar, "date", None))
        fields = {
            "ts": f"{ts_val:.3f}",
            "open": str(getattr(bar, "open", "")),
            "high": str(getattr(bar, "high", "")),
            "low": str(getattr(bar, "low", "")),
            "close": str(getattr(bar, "close", "")),
            "volume": str(getattr(bar, "volume", "")),
            "wap": str(getattr(bar, "average", "")),
            "count": str(getattr(bar, "barCount", "")),
        }
        key = f"ibkr:bars:{symbol}:{suffix}"
        try:
            self._redis.xadd(key, fields,
                             maxlen=self.config.publishing.stream_maxlen,
                             approximate=True)
            if not batch:
                self._update_snapshot(symbol, {
                    "last_bar_close": fields["close"],
                    "last_bar_ts": fields["ts"],
                })
        except redis.exceptions.RedisError as exc:
            self._log(f"redis xadd bars/{symbol}/{suffix} failed: {exc}")

    def _make_tick_handler(self, symbol: str):
        publish = self._publish_tick
        def _handler(ticker) -> None:  # noqa: ANN001
            publish(symbol, ticker)
        return _handler

    def _publish_tick(self, symbol: str, ticker) -> None:  # noqa: ANN001
        # ticker.marketDataType ∈ {1=live, 2=frozen, 3=delayed, 4=delayed-frozen}
        mdt = getattr(ticker, "marketDataType", None)
        if mdt is not None:
            self._limiter.observe_market_data_type(symbol, mdt)
        bid = _safe(ticker.bid)
        ask = _safe(ticker.ask)
        last = _safe(ticker.last)
        bid_size = _safe(getattr(ticker, "bidSize", None))
        ask_size = _safe(getattr(ticker, "askSize", None))
        last_size = _safe(getattr(ticker, "lastSize", None))
        ts_val = time.time()
        fields = {
            "ts": f"{ts_val:.3f}",
            "bid": _fmt(bid),
            "ask": _fmt(ask),
            "last": _fmt(last),
            "bid_size": _fmt(bid_size),
            "ask_size": _fmt(ask_size),
            "last_size": _fmt(last_size),
            "market_data_type": str(mdt) if mdt is not None else "",
        }
        key = f"ibkr:ticks:{symbol}"
        try:
            self._redis.xadd(key, fields,
                             maxlen=self.config.publishing.stream_maxlen,
                             approximate=True)
            self._update_snapshot(symbol, fields)
        except redis.exceptions.RedisError as exc:
            self._log(f"redis xadd ticks/{symbol} failed: {exc}")

    def _update_snapshot(self, symbol: str, fields: dict[str, str]) -> None:
        key = f"ibkr:snapshot:{symbol}"
        try:
            self._redis.hset(key, mapping=fields)
            ttl = self.config.publishing.snapshot_ttl_sec
            if ttl > 0:
                self._redis.expire(key, ttl)
        except redis.exceptions.RedisError as exc:
            self._log(f"redis snapshot/{symbol} failed: {exc}")

    # ----- IBKR error / disconnect handling ------------------------------

    def _on_ib_error(self, reqId, errorCode, errorString, contract) -> None:  # noqa: ANN001
        # Suppress purely informational codes (per ibkr_errors catalog).
        if is_info(errorCode):
            return
        contract_label = None
        if contract is not None and getattr(contract, "symbol", None):
            contract_label = f"{contract.symbol}/{contract.secType}"
        # Render the multi-line bilingual form for catalogued codes;
        # uncatalogued codes still come through with a clear marker so
        # users can file an issue for us to add a translation.
        formatted = format_for_log(errorCode, errorString or "", contract_label)
        for line in formatted.splitlines():
            self._log(line)
        self._limiter.observe_error(errorCode, errorString or "", contract_label)

    def _on_disconnected(self) -> None:
        self._publish_status("disconnected")
        self._log("disconnected from IBKR")

    # ----- Misc ----------------------------------------------------------

    def _publish_status(self, state: str) -> None:
        try:
            active_client_id = self._selected_client_id or self.config.gateway.client_id
            self._redis.hset("ibkr:bridge:status", mapping={
                "state": state,
                "ts": f"{time.time():.3f}",
                "gateway_host": self.config.gateway.host,
                "gateway_port": str(self.config.gateway.port),
                "market_data_type": str(self.config.gateway.market_data_type),
                "client_id": str(active_client_id),
                "configured_client_id": str(self.config.gateway.client_id),
                "client_id_fallback_engaged": str(
                    active_client_id != self.config.gateway.client_id
                ).lower(),
                "client_id_conflicts": ",".join(
                    str(client_id) for client_id in self._attempted_client_id_conflicts
                ),
                "subscriptions_active": str(len(self._active_subscriptions)),
            })
        except redis.exceptions.RedisError:
            pass

    def _log(self, msg: str) -> None:
        ts = datetime.now(timezone.utc).isoformat(timespec="seconds")
        print(f"[{ts}] [bridge] {msg}", flush=True)


# ---------------------------------------------------------------------------
# Helpers


def _safe(v: Any) -> float | None:
    if v is None:
        return None
    try:
        f = float(v)
    except (TypeError, ValueError):
        return None
    if math.isnan(f) or math.isinf(f):
        return None
    return f


def _fmt(v: float | None) -> str:
    return "" if v is None else f"{v:.6f}"


def _bar_ts(ts: Any) -> float:
    """Normalise an ib_async BarData.date value to a UTC epoch float.

    IBKR returns:
      * `datetime.datetime` for intraday bars (with formatDate=2; usually UTC,
        but tzinfo is sometimes missing — assume UTC then)
      * `datetime.date`     for daily / weekly / monthly bars
      * an epoch int/string when something exotic happens
    """
    if isinstance(ts, datetime):
        return (ts if ts.tzinfo else ts.replace(tzinfo=timezone.utc)).timestamp()
    if hasattr(ts, "isoformat"):  # datetime.date — promote to UTC midnight
        return datetime(ts.year, ts.month, ts.day,
                          tzinfo=timezone.utc).timestamp()
    try:
        return float(ts)
    except (TypeError, ValueError):
        return datetime.now(timezone.utc).timestamp()


# ---------------------------------------------------------------------------
# CLI entry


async def _amain(args: argparse.Namespace) -> int:
    require_ibkr_enabled()
    cfg = BridgeConfig.from_yaml(Path(args.config))
    bridge = IbkrBridge(cfg)

    loop = asyncio.get_running_loop()
    stop = asyncio.Event()

    def _signal() -> None:
        stop.set()

    for sig in (signal.SIGINT, signal.SIGTERM):
        with contextlib.suppress(NotImplementedError):
            loop.add_signal_handler(sig, _signal)

    runner = asyncio.create_task(bridge.start())
    await stop.wait()
    await bridge.stop()
    runner.cancel()
    with contextlib.suppress(asyncio.CancelledError):
        await runner
    return 0


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="ibkr_bridge.bridge",
        description="IBKR live data producer publishing to Redis Streams",
    )
    p.add_argument("--config", required=True,
                    help="Path to YAML config (see example_config.yaml)")
    return p


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    return asyncio.run(_amain(args))


if __name__ == "__main__":
    raise SystemExit(main())
