"""IBKR live data consumer — read-only helper for downstream scripts.

Auto-Quant strategies and ict-engine research scripts import :class:`IbkrConsumer`
to read whatever ``bridge.py`` has published into Redis. Consumers never
touch IBKR directly, never need consent, and can run without IB Gateway.

Typical usage in a Freqtrade strategy ``populate_indicators``::

    from ibkr_bridge.consumer import IbkrConsumer

    c = IbkrConsumer()
    snap = c.snapshot("AAPL")
    last_bid = snap.get("bid")
    bars = c.bars("AAPL", bar_size="5sec", lookback=300)  # pandas DataFrame
    if bars.empty:
        return df  # bridge not running yet; fall back to local source

Streaming consumption (async)::

    async for bar in c.stream_bars(["AAPL", "SPY"]):
        handle(bar)

Failure modes:
    * Redis unreachable      -> RuntimeError at construction.
    * Symbol absent          -> snapshot() returns {}; bars/ticks return empty
                                DataFrame.
    * Bridge offline         -> stream methods block until bridge wakes up
                                (XREAD with BLOCK=...).
"""

from __future__ import annotations

import asyncio
import math
from typing import Any, AsyncIterator

import pandas as pd

DEFAULT_REDIS_URL = "redis://localhost:6379"

BAR_FLOAT_COLS = ("open", "high", "low", "close", "volume", "wap")
BAR_INT_COLS = ("count",)
TICK_FLOAT_COLS = ("bid", "ask", "last", "bid_size", "ask_size", "last_size")


class IbkrConsumer:
    """Read-only consumer of bridge-produced Redis streams + snapshots."""

    def __init__(self, redis_url: str = DEFAULT_REDIS_URL) -> None:
        self._url = redis_url
        redis = _load_redis_sync_module()
        self._sync = redis.Redis.from_url(redis_url, decode_responses=True,
                                            socket_connect_timeout=2.0)
        # Verify Redis is reachable; surface a clear error if not.
        try:
            self._sync.ping()
        except redis.exceptions.RedisError as exc:
            raise RuntimeError(
                f"IbkrConsumer requires a reachable Redis at {redis_url!r}. "
                f"Is the bridge running? Underlying error: {exc}"
            ) from exc
        self._async: Any | None = None

    # ----- Snapshot ------------------------------------------------------

    def snapshot(self, symbol: str) -> dict[str, Any]:
        """Latest known values for ``symbol`` (HGETALL ibkr:snapshot:SYM)."""
        raw = self._sync.hgetall(f"ibkr:snapshot:{symbol}")
        return _coerce_snapshot(raw)

    def all_snapshots(self) -> dict[str, dict[str, Any]]:
        out: dict[str, dict[str, Any]] = {}
        for key in self._sync.scan_iter(match="ibkr:snapshot:*", count=200):
            sym = key.split(":", 2)[-1]
            out[sym] = _coerce_snapshot(self._sync.hgetall(key))
        return out

    # ----- Historical reads from streams ---------------------------------

    def bars(self, symbol: str, bar_size: str = "5sec",
              lookback: int = 200) -> pd.DataFrame:
        """Most-recent ``lookback`` bars for ``symbol`` as DataFrame.

        Columns: ts (UTC datetime), open, high, low, close, volume, wap, count.
        Empty DataFrame when stream has no data.
        """
        key = f"ibkr:bars:{symbol}:{bar_size}"
        entries = self._sync.xrevrange(key, count=lookback)
        if not entries:
            return _empty_bars()
        rows: list[dict[str, Any]] = []
        for stream_id, fields in reversed(entries):
            row = dict(fields)
            row["_id"] = stream_id
            rows.append(row)
        df = pd.DataFrame(rows)
        return _coerce_bars(df)

    def ticks(self, symbol: str, lookback: int = 500) -> pd.DataFrame:
        key = f"ibkr:ticks:{symbol}"
        entries = self._sync.xrevrange(key, count=lookback)
        if not entries:
            return _empty_ticks()
        rows = [{"_id": sid, **fields} for sid, fields in reversed(entries)]
        return _coerce_ticks(pd.DataFrame(rows))

    # ----- Bridge liveness ----------------------------------------------

    def bridge_status(self) -> dict[str, Any]:
        raw = self._sync.hgetall("ibkr:bridge:status")
        if not raw:
            return {"state": "absent"}
        return _coerce_snapshot(raw)

    def bridge_runtime_summary(self) -> dict[str, Any]:
        raw = self._sync.hgetall("ibkr:bridge:status")
        if not raw:
            return {"state": "absent"}
        return _coerce_bridge_status(raw)

    def recommended_gateway_target(self) -> dict[str, Any]:
        return _recommended_gateway_target(self.bridge_runtime_summary())

    # ----- Streaming ------------------------------------------------------

    async def stream_bars(self, symbols: list[str], bar_size: str = "5sec",
                           start_id: str = "$",
                           block_ms: int = 5000
                           ) -> AsyncIterator[dict[str, Any]]:
        """Async iterator yielding new bars as the bridge publishes them.

        ``start_id="$"`` means start at the latest entry (live tail).
        ``start_id="0"`` replays the full backlog first.
        """
        async for entry in self._stream(
            keys=[f"ibkr:bars:{s}:{bar_size}" for s in symbols],
            start_id=start_id,
            block_ms=block_ms,
            float_cols=BAR_FLOAT_COLS,
            int_cols=BAR_INT_COLS,
            kind="bar",
        ):
            yield entry

    async def stream_ticks(self, symbols: list[str],
                            start_id: str = "$",
                            block_ms: int = 5000
                            ) -> AsyncIterator[dict[str, Any]]:
        async for entry in self._stream(
            keys=[f"ibkr:ticks:{s}" for s in symbols],
            start_id=start_id,
            block_ms=block_ms,
            float_cols=TICK_FLOAT_COLS,
            int_cols=(),
            kind="tick",
        ):
            yield entry

    async def _stream(self, keys: list[str], start_id: str, block_ms: int,
                       float_cols: tuple[str, ...], int_cols: tuple[str, ...],
                       kind: str) -> AsyncIterator[dict[str, Any]]:
        if self._async is None:
            redis_async = _load_redis_async_module()
            self._async = redis_async.from_url(self._url, decode_responses=True)
        cursors = {key: start_id for key in keys}
        while True:
            try:
                results = await self._async.xread(streams=cursors,
                                                    block=block_ms, count=200)
            except asyncio.CancelledError:
                raise
            except _redis_error_types() as exc:
                # Transient — pause briefly and retry
                await asyncio.sleep(1.0)
                continue
            if not results:
                continue
            for key, entries in results:
                for stream_id, fields in entries:
                    cursors[key] = stream_id
                    sym = _symbol_from_key(key)
                    yield _coerce_entry(sym, kind, stream_id, fields,
                                          float_cols, int_cols)

    async def aclose(self) -> None:
        if self._async is not None:
            await self._async.aclose()
            self._async = None


# ---------------------------------------------------------------------------
# Coercion helpers


def _empty_bars() -> pd.DataFrame:
    cols = ("ts",) + BAR_FLOAT_COLS + BAR_INT_COLS
    return pd.DataFrame(columns=cols)


def _empty_ticks() -> pd.DataFrame:
    cols = ("ts",) + TICK_FLOAT_COLS + ("market_data_type",)
    return pd.DataFrame(columns=cols)


def _coerce_bars(df: pd.DataFrame) -> pd.DataFrame:
    df = df.copy()
    if "ts" in df.columns:
        df["ts"] = pd.to_datetime(pd.to_numeric(df["ts"], errors="coerce"),
                                    unit="s", utc=True)
    for col in BAR_FLOAT_COLS:
        if col in df.columns:
            df[col] = pd.to_numeric(df[col], errors="coerce")
    for col in BAR_INT_COLS:
        if col in df.columns:
            df[col] = pd.to_numeric(df[col], errors="coerce").astype("Int64")
    return df


def _coerce_ticks(df: pd.DataFrame) -> pd.DataFrame:
    df = df.copy()
    if "ts" in df.columns:
        df["ts"] = pd.to_datetime(pd.to_numeric(df["ts"], errors="coerce"),
                                    unit="s", utc=True)
    for col in TICK_FLOAT_COLS:
        if col in df.columns:
            df[col] = pd.to_numeric(df[col], errors="coerce")
    if "market_data_type" in df.columns:
        df["market_data_type"] = pd.to_numeric(df["market_data_type"],
                                                  errors="coerce").astype("Int64")
    return df


def _coerce_snapshot(raw: dict[str, str]) -> dict[str, Any]:
    out: dict[str, Any] = dict(raw)
    for col in (set(BAR_FLOAT_COLS) | set(TICK_FLOAT_COLS) | {"ts", "last_bar_close",
                                                                 "last_bar_ts"}):
        if col in out and out[col] != "":
            try:
                v = float(out[col])
                if math.isnan(v):
                    out[col] = None
                else:
                    out[col] = v
            except (TypeError, ValueError):
                pass
    return out


def _coerce_bridge_status(raw: dict[str, str]) -> dict[str, Any]:
    out = _coerce_snapshot(raw)
    for key in (
        "client_id",
        "configured_client_id",
        "subscriptions_active",
        "gateway_port",
        "market_data_type",
    ):
        if key in out and out[key] not in ("", None):
            try:
                out[key] = int(out[key])
            except (TypeError, ValueError):
                pass
    if "client_id_fallback_engaged" in out:
        out["client_id_fallback_engaged"] = str(out["client_id_fallback_engaged"]).lower() == "true"
    if "client_id_conflicts" in out:
        conflicts = str(out["client_id_conflicts"]).strip()
        if not conflicts:
            out["client_id_conflicts"] = []
        else:
            out["client_id_conflicts"] = [
                int(item) for item in conflicts.split(",") if item.strip()
            ]
    return out


def _recommended_gateway_target(status: dict[str, Any]) -> dict[str, Any]:
    state = status.get("state", "absent")
    if state == "absent":
        return {
            "status": "bridge_absent",
            "message": "IBKR bridge is not publishing status yet; run the bridge or inspect setup/status first.",
        }

    host = status.get("gateway_host")
    port = status.get("gateway_port")
    client_id = status.get("client_id")
    configured_client_id = status.get("configured_client_id")
    market_data_type = status.get("market_data_type")
    fallback = bool(status.get("client_id_fallback_engaged", False))
    conflicts = status.get("client_id_conflicts", [])
    if host is None or port is None:
        return {
            "status": "incomplete_status",
            "message": "IBKR bridge status is present but missing gateway host/port details.",
        }

    state_status = "ready" if str(state) == "running" else "not_running"
    if fallback:
        message = (
            f"IBKR bridge is using {host}:{port} with fallback clientId={client_id} "
            f"(configured={configured_client_id}, conflicts={conflicts})."
        )
    else:
        message = (
            f"IBKR bridge is using {host}:{port} with clientId={client_id}."
        )
    if market_data_type is not None:
        message += f" market_data_type={market_data_type}."
    if state_status != "ready":
        message += f" current_state={state}."
    return {
        "status": state_status,
        "state": state,
        "host": host,
        "port": port,
        "client_id": client_id,
        "configured_client_id": configured_client_id,
        "client_id_fallback_engaged": fallback,
        "client_id_conflicts": conflicts,
        "market_data_type": market_data_type,
        "message": message,
    }


def _coerce_entry(symbol: str, kind: str, stream_id: str, fields: dict[str, str],
                   float_cols: tuple[str, ...], int_cols: tuple[str, ...]
                   ) -> dict[str, Any]:
    out: dict[str, Any] = {"symbol": symbol, "kind": kind,
                              "_id": stream_id, **fields}
    if "ts" in out:
        try:
            out["ts"] = float(out["ts"])
        except (TypeError, ValueError):
            pass
    for col in float_cols:
        if col in out and out[col] != "":
            try:
                out[col] = float(out[col])
            except (TypeError, ValueError):
                pass
    for col in int_cols:
        if col in out and out[col] != "":
            try:
                out[col] = int(out[col])
            except (TypeError, ValueError):
                pass
    return out


def _symbol_from_key(key: str) -> str:
    # ibkr:bars:AAPL:5sec  -> AAPL
    # ibkr:ticks:AAPL      -> AAPL
    parts = key.split(":")
    if len(parts) >= 3 and parts[0] == "ibkr":
        return parts[2]
    return key


def _load_redis_sync_module():
    try:
        import redis  # noqa: WPS433
    except ModuleNotFoundError as exc:
        if exc.name == "redis":
            raise RuntimeError(
                "IbkrConsumer requires the python `redis` package. Install it before consuming bridge state."
            ) from exc
        raise
    return redis


def _load_redis_async_module():
    try:
        import redis.asyncio as redis_async  # noqa: WPS433
    except ModuleNotFoundError as exc:
        if exc.name == "redis":
            raise RuntimeError(
                "IbkrConsumer async streaming requires the python `redis` package."
            ) from exc
        raise
    return redis_async


def _redis_error_types():
    redis = _load_redis_sync_module()
    return (redis.exceptions.RedisError,)


# ---------------------------------------------------------------------------
# Diagnostic CLI


def _cli() -> int:
    import argparse
    import json
    p = argparse.ArgumentParser(description="Inspect IBKR Redis bridge state")
    p.add_argument("--redis-url", default=DEFAULT_REDIS_URL)
    p.add_argument("symbol", nargs="?", help="Inspect this symbol's snapshot + last bar/tick")
    args = p.parse_args()

    try:
        c = IbkrConsumer(redis_url=args.redis_url)
    except RuntimeError as exc:
        print(exc)
        return 2

    print("bridge status:", json.dumps(c.bridge_runtime_summary(), indent=2, default=str))
    print("recommended target:", json.dumps(c.recommended_gateway_target(), indent=2, default=str))
    if args.symbol:
        print(f"\nsnapshot[{args.symbol}]:",
              json.dumps(c.snapshot(args.symbol), indent=2, default=str))
        bars = c.bars(args.symbol, lookback=5)
        print(f"\nlast 5 bars for {args.symbol}:")
        print(bars.to_string(index=False) if not bars.empty else "  (none)")
        ticks = c.ticks(args.symbol, lookback=5)
        print(f"\nlast 5 ticks for {args.symbol}:")
        print(ticks.to_string(index=False) if not ticks.empty else "  (none)")
    else:
        snaps = c.all_snapshots()
        print(f"\nactive symbols: {sorted(snaps)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(_cli())
