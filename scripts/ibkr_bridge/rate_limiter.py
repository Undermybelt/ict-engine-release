"""Redis-backed adaptive rate limiter for IBKR.

Implements the canonical IBKR pacing rules and adapts limits at runtime based
on observed errors. State is held in Redis so that multiple processes
(`bridge.py`, `fetch_external.py ibkr-historical`, future ad-hoc scripts) all
share the same account-level pacing budget.

Pacing rules covered (default values, all adjustable via capabilities):
    * Historical bars: 6.5 s between identical (contract, bar_size,
      what_to_show); 60 distinct requests in any 10-minute sliding window;
      semaphore of 50 simultaneous historical requests.
    * Streaming market data: account-level line cap (default 80; learned).
    * Snapshot data: 11 s per contract; ~1/s global.
    * Outbound msg: ~50/s to TWS.

Adaptation triggers (driven by `observe_error()`):
    * 162 (pacing): increase `hist_min_interval_sec` by 25% (cap 30s),
      decrement `hist_window_capacity` after 3 violations / 24h.
    * 354 / 322 / 1100 (lines exhausted): record measured ceiling.
    * Connection reset: decrease `msg_outbound_rate` to 30, slow ramp.

Capabilities are persisted to ``~/.ict-engine/ibkr_capabilities.json`` so
they survive bridge restarts.

This module is async-friendly (uses ``asyncio.sleep`` for waits) but the
underlying Redis client is sync (operations are sub-millisecond).
"""

from __future__ import annotations

import asyncio
import json
import math
import time
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Literal

import redis

# ---------------------------------------------------------------------------
# Constants

CAPABILITIES_PATH = Path.home() / ".ict-engine" / "ibkr_capabilities.json"
CONSENT_PATH = Path.home() / ".ict-engine" / "ibkr_consent.json"

# Conservative defaults. Used on first run when no capabilities file exists.
# These are intentionally below documented IBKR ceilings so that fresh users
# never accidentally exceed and trigger 162.
DEFAULT_HIST_MIN_INTERVAL_SEC = 6.5
DEFAULT_HIST_WINDOW_CAPACITY = 55  # IBKR limit is 60; we leave 5-req buffer
DEFAULT_HIST_WINDOW_SEC = 600
DEFAULT_HIST_SIMULTANEOUS_CAP = 40  # IBKR limit is 50; buffer 10
DEFAULT_MAX_LINES = 80              # IBKR default tier is 100; buffer 20
DEFAULT_LINES_SAFETY_BUFFER = 0.85
DEFAULT_SNAPSHOT_INTERVAL_SEC = 11.0
DEFAULT_SNAPSHOT_GLOBAL_RATE = 1.0  # per second
DEFAULT_MSG_OUTBOUND_RATE = 45      # per second; IBKR enforces ~50

# Adaptation knobs
HIST_INTERVAL_BACKOFF_FACTOR = 1.25
HIST_INTERVAL_MAX_SEC = 30.0
HIST_WINDOW_DECREMENT_AFTER_N_VIOLATIONS = 3
MSG_RATE_FLOOR = 25                 # never go below this
RELAX_AFTER_HOURS = 48
RELAX_FACTOR = 0.95                 # tighten by 5% on each adverse signal

# Redis keys
KEY_HIST_CONTRACT_LOCK = "ibkr:rl:hist:lock:{conid}:{bar}:{what}"
KEY_HIST_GLOBAL_WINDOW = "ibkr:rl:hist:global"
KEY_HIST_SIMULTANEOUS = "ibkr:rl:hist:simultaneous"
KEY_LINES_CURRENT = "ibkr:rl:lines:current"
KEY_SNAPSHOT_CONTRACT = "ibkr:rl:snap:contract:{conid}"
KEY_SNAPSHOT_GLOBAL = "ibkr:rl:snap:global"
KEY_MSG_OUTBOUND = "ibkr:rl:msg:outbound"

AccountType = Literal["paper", "live_pro", "live_lite", "fa", "institutional", "unknown"]


# ---------------------------------------------------------------------------
# Capabilities (persisted state)


@dataclass
class IbkrCapabilities:
    """Adaptive limits learned from real account behaviour.

    Static fields (set once by AccountProber) describe what kind of IBKR
    account this is. Adaptive fields are nudged at runtime by
    ``IbkrRateLimiter.observe_error()`` and persisted across restarts.
    """

    version: int = 1
    first_seen: str = ""
    last_updated: str = ""

    # Static (set by AccountProber on first connect)
    account_type: AccountType = "unknown"
    n_subaccounts: int = 1

    # Adaptive
    measured_max_lines: int = DEFAULT_MAX_LINES
    lines_safety_buffer: float = DEFAULT_LINES_SAFETY_BUFFER
    hist_min_interval_sec: float = DEFAULT_HIST_MIN_INTERVAL_SEC
    hist_window_capacity: int = DEFAULT_HIST_WINDOW_CAPACITY
    hist_simultaneous_cap: int = DEFAULT_HIST_SIMULTANEOUS_CAP
    snapshot_interval_sec: float = DEFAULT_SNAPSHOT_INTERVAL_SEC
    snapshot_global_rate: float = DEFAULT_SNAPSHOT_GLOBAL_RATE
    msg_outbound_rate: int = DEFAULT_MSG_OUTBOUND_RATE

    # Observations
    feeds_observed_delayed: list[str] = field(default_factory=list)
    violations_24h: list[dict] = field(default_factory=list)

    @classmethod
    def load_or_default(cls, path: Path = CAPABILITIES_PATH) -> "IbkrCapabilities":
        if not path.exists():
            now = datetime.now(timezone.utc).isoformat()
            return cls(first_seen=now, last_updated=now)
        try:
            data = json.loads(path.read_text())
        except (json.JSONDecodeError, OSError) as exc:
            print(f"[rate_limiter] capabilities file corrupt ({exc}); starting from defaults",
                  flush=True)
            now = datetime.now(timezone.utc).isoformat()
            return cls(first_seen=now, last_updated=now)
        # Drop unknown keys so renaming a field doesn't crash the loader.
        valid_fields = {f for f in cls.__dataclass_fields__}
        filtered = {k: v for k, v in data.items() if k in valid_fields}
        return cls(**filtered)

    def save(self, path: Path = CAPABILITIES_PATH) -> None:
        self.last_updated = datetime.now(timezone.utc).isoformat()
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(asdict(self), indent=2))

    def effective_max_lines(self) -> int:
        return max(1, int(self.measured_max_lines * self.lines_safety_buffer))

    def hist_window_capacity_for_account(self) -> int:
        # FAdvisor / multi-subaccount accounts have effectively tighter
        # historical budgets because the same physical request multiplies.
        if self.n_subaccounts > 1:
            return max(10, self.hist_window_capacity // self.n_subaccounts)
        return self.hist_window_capacity


# ---------------------------------------------------------------------------
# Rate limiter


class IbkrRateLimiter:
    """Async-friendly, Redis-coordinated IBKR pacing limiter.

    Usage from a producer:
        limiter = IbkrRateLimiter(redis_url="redis://localhost:6379")
        await limiter.wait_for_historical(conid, "1 hour", "TRADES")
        await limiter.wait_for_outbound_msg()
        # ... call ib_async ...

    From an error handler / event listener:
        limiter.observe_error(error_code, message, contract_label)
    """

    def __init__(self, redis_url: str = "redis://localhost:6379",
                 capabilities: IbkrCapabilities | None = None,
                 capabilities_path: Path = CAPABILITIES_PATH) -> None:
        self._redis = redis.Redis.from_url(redis_url, decode_responses=True)
        # Verify Redis is reachable; fail-fast at construction.
        try:
            self._redis.ping()
        except redis.exceptions.RedisError as exc:
            raise RuntimeError(
                f"IbkrRateLimiter requires a reachable Redis at {redis_url!r}. "
                f"Install with `brew install redis && brew services start redis`. "
                f"Underlying error: {exc}"
            ) from exc
        self._caps_path = capabilities_path
        self.caps = capabilities or IbkrCapabilities.load_or_default(capabilities_path)

    # ----- Historical bars ------------------------------------------------

    async def wait_for_historical(self, conid: int | str, bar_size: str,
                                   what_to_show: str = "TRADES",
                                   max_wait_sec: float = 900.0) -> None:
        """Block until safe to issue a `reqHistoricalData` for this triple.

        Acquires the per-contract 6.5s lock and respects the 60-distinct-
        requests / 10-minute sliding window.

        Raises ``TimeoutError`` if not satisfied within ``max_wait_sec``.
        """
        contract_key = KEY_HIST_CONTRACT_LOCK.format(
            conid=conid, bar=bar_size.replace(" ", "_"),
            what=what_to_show,
        )
        global_key = KEY_HIST_GLOBAL_WINDOW
        interval = self.caps.hist_min_interval_sec
        window_cap = self.caps.hist_window_capacity_for_account()
        window_sec = DEFAULT_HIST_WINDOW_SEC

        deadline = time.monotonic() + max_wait_sec
        while time.monotonic() < deadline:
            now_s = time.time()
            # Trim global window
            self._redis.zremrangebyscore(global_key, "-inf", now_s - window_sec)
            window_count = int(self._redis.zcount(global_key, now_s - window_sec, "+inf"))

            # Acquire contract lock
            ttl_ms = int(interval * 1000) + 100
            acquired = self._redis.set(contract_key, "1", nx=True, px=ttl_ms)

            if acquired and window_count < window_cap:
                # Record this request in the global window
                member = f"{conid}:{bar_size}:{what_to_show}:{int(now_s*1e6)}"
                self._redis.zadd(global_key, {member: now_s})
                self._redis.expire(global_key, window_sec + 60)
                return

            # Compute wait
            if not acquired:
                # Contract lock held → wait remaining TTL
                pttl = self._redis.pttl(contract_key)
                wait_sec = max(0.05, (pttl or 200) / 1000.0)
            else:
                # Window full → release prematurely-acquired lock and wait
                # for oldest entry to expire.
                self._redis.delete(contract_key)
                oldest = self._redis.zrange(global_key, 0, 0, withscores=True)
                if oldest:
                    wait_sec = max(0.5, oldest[0][1] + window_sec - now_s + 0.1)
                else:
                    wait_sec = 1.0
            await asyncio.sleep(min(wait_sec, 10.0))

        raise TimeoutError(
            f"wait_for_historical(conid={conid}, bar={bar_size}, "
            f"what={what_to_show}) timed out after {max_wait_sec}s; "
            f"global window currently {window_count}/{window_cap}"
        )

    async def acquire_historical_slot(self) -> None:
        """Reserve one of the simultaneous historical-request slots."""
        cap = self.caps.hist_simultaneous_cap
        deadline = time.monotonic() + 300
        while time.monotonic() < deadline:
            current = int(self._redis.get(KEY_HIST_SIMULTANEOUS) or 0)
            if current < cap:
                # Best-effort INCR; race tolerated (cap has buffer baked in).
                self._redis.incr(KEY_HIST_SIMULTANEOUS)
                self._redis.expire(KEY_HIST_SIMULTANEOUS, 600)
                return
            await asyncio.sleep(0.5)
        raise TimeoutError(
            f"acquire_historical_slot timed out (cap={cap})"
        )

    def release_historical_slot(self) -> None:
        cur = int(self._redis.get(KEY_HIST_SIMULTANEOUS) or 0)
        if cur > 0:
            self._redis.decr(KEY_HIST_SIMULTANEOUS)

    # ----- Streaming lines ------------------------------------------------

    def acquire_streaming_line(self, symbol: str) -> bool:
        """Reserve 1 streaming market-data line. Returns False if at cap."""
        cap = self.caps.effective_max_lines()
        # Simple INCR-then-check pattern; race tolerated (off-by-one is OK
        # since we have a safety buffer).
        new_count = int(self._redis.incr(KEY_LINES_CURRENT))
        if new_count > cap:
            self._redis.decr(KEY_LINES_CURRENT)
            return False
        self._redis.sadd("ibkr:rl:lines:held", symbol)
        return True

    def release_streaming_line(self, symbol: str) -> None:
        was_held = self._redis.srem("ibkr:rl:lines:held", symbol)
        if was_held:
            cur = int(self._redis.get(KEY_LINES_CURRENT) or 0)
            if cur > 0:
                self._redis.decr(KEY_LINES_CURRENT)

    def held_streaming_lines(self) -> int:
        return int(self._redis.get(KEY_LINES_CURRENT) or 0)

    def reset_streaming_lines(self) -> None:
        """Called by bridge on graceful shutdown to clean up account."""
        self._redis.set(KEY_LINES_CURRENT, 0)
        self._redis.delete("ibkr:rl:lines:held")

    # ----- Snapshot data --------------------------------------------------

    async def wait_for_snapshot(self, conid: int | str,
                                 max_wait_sec: float = 60.0) -> None:
        contract_key = KEY_SNAPSHOT_CONTRACT.format(conid=conid)
        global_key = KEY_SNAPSHOT_GLOBAL
        deadline = time.monotonic() + max_wait_sec
        while time.monotonic() < deadline:
            ttl_ms = int(self.caps.snapshot_interval_sec * 1000) + 100
            acquired = self._redis.set(contract_key, "1", nx=True, px=ttl_ms)
            global_ok = self._global_token(global_key, self.caps.snapshot_global_rate)
            if acquired and global_ok:
                return
            await asyncio.sleep(0.5)
        raise TimeoutError(f"wait_for_snapshot(conid={conid}) timed out")

    # ----- Outbound message rate -----------------------------------------

    async def wait_for_outbound_msg(self, max_wait_sec: float = 30.0) -> None:
        """Throttle every outbound IBKR API call to caps.msg_outbound_rate/sec."""
        deadline = time.monotonic() + max_wait_sec
        while time.monotonic() < deadline:
            if self._global_token(KEY_MSG_OUTBOUND, self.caps.msg_outbound_rate):
                return
            await asyncio.sleep(1.0 / max(1, self.caps.msg_outbound_rate))
        raise TimeoutError("wait_for_outbound_msg timed out")

    def _global_token(self, key: str, rate_per_sec: float) -> bool:
        """Sliding-second token bucket. Returns True if a token was claimed."""
        now_s = time.time()
        cutoff = now_s - 1.0
        self._redis.zremrangebyscore(key, "-inf", cutoff)
        count = int(self._redis.zcount(key, cutoff, "+inf"))
        if count >= rate_per_sec:
            return False
        self._redis.zadd(key, {f"{int(now_s*1e6)}": now_s})
        self._redis.expire(key, 3)
        return True

    # ----- Adaptive feedback ---------------------------------------------

    def observe_error(self, code: int, message: str = "",
                       contract_label: str | None = None) -> None:
        """Adjust capabilities based on observed IBKR error feedback.

        Common pacing-relevant codes:
            162 — historical pacing violation
            354 / 322 / 1100 — streaming line / connectivity exhaustion
            165 — historical service connecting (transient, no-op)
            10092 — connection reset (rare; handled by reconnect loop)
        """
        now = datetime.now(timezone.utc).isoformat()
        violation = {"timestamp": now, "code": code, "message": message,
                     "contract": contract_label}

        if code == 162:
            old = self.caps.hist_min_interval_sec
            self.caps.hist_min_interval_sec = min(
                HIST_INTERVAL_MAX_SEC,
                old * HIST_INTERVAL_BACKOFF_FACTOR,
            )
            self.caps.violations_24h = self._prune_violations() + [violation]
            recent_162 = sum(1 for v in self.caps.violations_24h if v.get("code") == 162)
            if recent_162 >= HIST_WINDOW_DECREMENT_AFTER_N_VIOLATIONS:
                self.caps.hist_window_capacity = max(
                    20, self.caps.hist_window_capacity - 5
                )
            print(f"[rate_limiter] error 162 observed; hist_min_interval "
                  f"{old:.2f}s → {self.caps.hist_min_interval_sec:.2f}s, "
                  f"window cap → {self.caps.hist_window_capacity}", flush=True)
            self.caps.save(self._caps_path)
            return

        elif code in (354, 322, 1100):
            current_lines = self.held_streaming_lines()
            if current_lines and current_lines - 1 < self.caps.measured_max_lines:
                self.caps.measured_max_lines = max(1, current_lines - 1)
                print(f"[rate_limiter] error {code}: line ceiling lowered to "
                      f"{self.caps.measured_max_lines}", flush=True)
            self.caps.violations_24h = self._prune_violations() + [violation]
            self.caps.save(self._caps_path)
            return

        elif code == 10092:
            self.caps.msg_outbound_rate = max(
                MSG_RATE_FLOOR,
                int(self.caps.msg_outbound_rate * RELAX_FACTOR),
            )
            self.caps.violations_24h = self._prune_violations() + [violation]
            print(f"[rate_limiter] connection reset; msg_outbound_rate → "
                  f"{self.caps.msg_outbound_rate}/s", flush=True)
            self.caps.save(self._caps_path)
            return

        elif code == 165:
            # Historical service connecting; transient, do not adapt.
            return

        else:
            return  # Unknown / non-pacing code; do not adapt

    def observe_market_data_type(self, symbol: str, market_data_type: int) -> None:
        """`marketDataType` 3 means user has no live subscription for symbol."""
        if market_data_type == 3 and symbol not in self.caps.feeds_observed_delayed:
            self.caps.feeds_observed_delayed.append(symbol)
            self.caps.save(self._caps_path)

    def _prune_violations(self) -> list[dict]:
        cutoff = datetime.now(timezone.utc).timestamp() - 86400
        return [
            v for v in self.caps.violations_24h
            if datetime.fromisoformat(v["timestamp"]).timestamp() >= cutoff
        ]

    # ----- Diagnostics ---------------------------------------------------

    def status_snapshot(self) -> dict[str, Any]:
        now_s = time.time()
        return {
            "lines_current": self.held_streaming_lines(),
            "lines_cap": self.caps.effective_max_lines(),
            "hist_window_in_use": int(self._redis.zcount(
                KEY_HIST_GLOBAL_WINDOW, now_s - DEFAULT_HIST_WINDOW_SEC, "+inf",
            )),
            "hist_window_cap": self.caps.hist_window_capacity_for_account(),
            "hist_simultaneous_in_use": int(self._redis.get(KEY_HIST_SIMULTANEOUS) or 0),
            "hist_simultaneous_cap": self.caps.hist_simultaneous_cap,
            "hist_min_interval_sec": self.caps.hist_min_interval_sec,
            "msg_rate_per_sec": self.caps.msg_outbound_rate,
            "account_type": self.caps.account_type,
            "n_subaccounts": self.caps.n_subaccounts,
            "delayed_feeds": list(self.caps.feeds_observed_delayed),
            "violations_24h": len(self._prune_violations()),
        }


# ---------------------------------------------------------------------------
# Standalone diagnostic CLI


def _cli() -> int:
    """Print current capabilities + Redis-side counters and exit."""
    import argparse
    p = argparse.ArgumentParser(description="IBKR rate-limiter diagnostics")
    p.add_argument("--redis-url", default="redis://localhost:6379")
    args = p.parse_args()
    try:
        limiter = IbkrRateLimiter(redis_url=args.redis_url)
    except RuntimeError as exc:
        print(exc)
        return 2
    print(json.dumps({"capabilities": asdict(limiter.caps),
                       "live": limiter.status_snapshot()},
                      indent=2, default=str))
    return 0


if __name__ == "__main__":
    raise SystemExit(_cli())
