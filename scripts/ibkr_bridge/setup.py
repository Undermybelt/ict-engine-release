"""First-run setup CLI for the IBKR live data bridge.

Walks the user through:
    1. Bilingual privacy disclaimer + explicit opt-in.
    2. Local Redis reachability + loopback-bind safety check.
    3. (Optional) IB Gateway / TWS reachability probe.
    4. (Optional) one-shot static account identification (writes
       ``~/.ict-engine/ibkr_capabilities.json``).

Subcommands:
    --enable   default; runs the full flow (idempotent).
    --revoke   deletes ``~/.ict-engine/ibkr_consent.json``; optionally
               wipes ``ibkr:*`` Redis keys with --clean-redis.
    --status   prints current consent / capabilities / Redis / Gateway state.

This is the **only** module the user is expected to invoke manually before
using IBKR features. After --enable succeeds, ``bridge.py``,
``fetch_external.py ibkr-historical``, and any other entry point can run
non-interactively.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import socket
import sys
from dataclasses import dataclass
from pathlib import Path

from .client_id import candidate_client_ids, is_client_id_conflict_error
from .consent import (
    CONSENT_PATH,
    is_opted_in,
    revoke as revoke_consent,
    show_disclaimer_and_prompt,
)

DEFAULT_REDIS_URL = "redis://localhost:6379"
DEFAULT_GATEWAY_HOST = "127.0.0.1"
DEFAULT_GATEWAY_PORT = None
PROBE_CLIENT_ID = 99
DEFAULT_PROBE_TIMEOUT_SEC = 8.0
CAPABILITIES_PATH = Path.home() / ".ict-engine" / "ibkr_capabilities.json"
COMMON_GATEWAY_PORTS = (
    ("TWS paper", 7497),
    ("TWS live", 7496),
    ("IB Gateway paper", 4002),
    ("IB Gateway live", 4001),
)

# ANSI colours; degraded gracefully on non-tty sinks
_GREEN = "\033[32m" if sys.stdout.isatty() else ""
_RED = "\033[31m" if sys.stdout.isatty() else ""
_YELLOW = "\033[33m" if sys.stdout.isatty() else ""
_DIM = "\033[2m" if sys.stdout.isatty() else ""
_RESET = "\033[0m" if sys.stdout.isatty() else ""

OK = f"{_GREEN}✓{_RESET}"
FAIL = f"{_RED}✗{_RESET}"
WARN = f"{_YELLOW}!{_RESET}"


@dataclass(frozen=True)
class GatewayCandidate:
    label: str
    port: int
    reachable: bool
    message: str


# ---------------------------------------------------------------------------
# Probes


def _ping_redis(url: str) -> tuple[bool, str]:
    try:
        redis = _load_redis_module()
    except ModuleNotFoundError as exc:
        return False, f"Redis client unavailable: {exc}. Install the python `redis` package first."
    try:
        r = redis.Redis.from_url(url, decode_responses=True,
                                  socket_connect_timeout=2.0)
        if not r.ping():
            return False, "PING returned falsy"
        bind = r.config_get("bind").get("bind", "")
        protected = r.config_get("protected-mode").get("protected-mode", "")
        version = r.info("server").get("redis_version", "?")
        external = any(part not in {"127.0.0.1", "::1"} and part
                       for part in bind.split())
        if external and protected != "yes":
            return False, (f"Redis bound to {bind!r} with protected-mode={protected!r}; "
                            "refusing to enable IBKR (would expose your data feed). "
                            "Edit /opt/homebrew/etc/redis.conf to bind 127.0.0.1 only.")
        return True, f"Redis {version}, bind={bind!r}, protected-mode={protected}"
    except redis.exceptions.RedisError as exc:
        return False, f"Redis unreachable: {exc}"


def _ping_gateway_socket(host: str, port: int, timeout: float = 2.0
                          ) -> tuple[bool, str]:
    """TCP-level reachability check; does not establish IBKR session."""
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True, f"TCP {host}:{port} accepting connections"
    except (socket.timeout, ConnectionRefusedError, OSError) as exc:
        return False, f"TCP {host}:{port} unreachable: {exc}"


def _gateway_label_for_port(port: int) -> str:
    for label, known_port in COMMON_GATEWAY_PORTS:
        if known_port == port:
            return label
    return f"Custom gateway port {port}"


def _scan_gateway_candidates(
    host: str,
    explicit_port: int | None,
    ping_socket=_ping_gateway_socket,
) -> list[GatewayCandidate]:
    specs = (
        [(_gateway_label_for_port(explicit_port), explicit_port)]
        if explicit_port is not None
        else list(COMMON_GATEWAY_PORTS)
    )
    candidates = []
    for label, port in specs:
        ok, msg = ping_socket(host, port)
        candidates.append(
            GatewayCandidate(
                label=label,
                port=port,
                reachable=ok,
                message=msg,
            )
        )
    return candidates


def _select_gateway_candidate(
    candidates: list[GatewayCandidate],
) -> GatewayCandidate | None:
    return next((candidate for candidate in candidates if candidate.reachable), None)


def _print_gateway_candidates(host: str, candidates: list[GatewayCandidate]) -> None:
    for candidate in candidates:
        marker = OK if candidate.reachable else WARN
        print(f"{marker} {candidate.label} ({host}:{candidate.port}) — {candidate.message}")


def _load_probe_account():
    try:
        from .account_prober import probe_account  # noqa: WPS433
    except ModuleNotFoundError as exc:
        if exc.name == "ib_async":
            raise RuntimeError(
                "IBKR account probe requires `ib_async`. Install it before probing, "
                "or rerun setup with --skip-probe."
            ) from exc
        raise
    return probe_account


def _load_redis_module():
    try:
        import redis  # noqa: WPS433
    except ModuleNotFoundError as exc:
        if exc.name == "redis":
            raise ModuleNotFoundError(
                "redis python package is required for IBKR setup/status"
            ) from exc
        raise
    return redis


def _load_ibkr_capabilities_class():
    from .rate_limiter import IbkrCapabilities  # noqa: WPS433

    return IbkrCapabilities


async def _probe_account_with_fallback(
    host: str,
    candidates: list[GatewayCandidate],
    client_id: int,
    timeout: float,
    probe_account_fn=None,
):
    probe_account = probe_account_fn or _load_probe_account()
    attempted_errors: list[str] = []
    for candidate in candidates:
        if not candidate.reachable:
            continue
        for candidate_client_id in candidate_client_ids(client_id):
            try:
                caps = await probe_account(
                    host=host,
                    port=candidate.port,
                    client_id=candidate_client_id,
                    timeout=timeout,
                )
                return candidate, candidate_client_id, caps, attempted_errors
            except (ConnectionError, RuntimeError) as exc:
                if is_client_id_conflict_error(exc):
                    attempted_errors.append(
                        f"{candidate.label} ({candidate.port}) clientId={candidate_client_id}: {exc}"
                    )
                    continue
                attempted_errors.append(
                    f"{candidate.label} ({candidate.port}) clientId={candidate_client_id}: {exc}"
                )
                break
    return None, None, None, attempted_errors


# ---------------------------------------------------------------------------
# Subcommands


async def cmd_enable(args: argparse.Namespace) -> int:
    print(f"{_DIM}── IBKR setup: enable ──{_RESET}\n")

    # 1. Consent
    if is_opted_in():
        print(f"{OK} Consent already on file at {CONSENT_PATH}")
    else:
        opted = show_disclaimer_and_prompt()
        if not opted:
            return 1

    # 2. Redis
    print(f"{_DIM}\n── Local Redis ──{_RESET}")
    ok, msg = _ping_redis(args.redis_url)
    print(f"{OK if ok else FAIL} {msg}")
    if not ok:
        print("\nTo install and start Redis on macOS:")
        print("    brew install redis")
        print("    brew services start redis")
        return 2

    # 3. Gateway TCP probe (optional but recommended)
    print(f"{_DIM}\n── IB Gateway / TWS ──{_RESET}")
    candidates = _scan_gateway_candidates(args.gateway_host, args.gateway_port)
    _print_gateway_candidates(args.gateway_host, candidates)
    selected = _select_gateway_candidate(candidates)
    if selected is not None:
        print(f"{OK} selected candidate: {selected.label} on {args.gateway_host}:{selected.port}")
        if args.gateway_port is None and sum(1 for item in candidates if item.reachable) > 1:
            print(f"{WARN} multiple local IBKR runtimes are reachable; using the first reachable candidate unless you pass --gateway-port explicitly.")
    else:
        print("    No reachable local TWS / Gateway endpoint detected.")
        print("    Setup will continue (you can launch TWS or Gateway later, or rerun with --gateway-port <port>).")
        if args.require_gateway:
            return 3

    # 4. Account probe (writes capabilities.json) — only if Gateway reachable
    if selected is not None and not args.skip_probe:
        print(f"{_DIM}\n── Account identification probe ──{_RESET}")
        candidate, selected_client_id, caps, attempted_errors = await _probe_account_with_fallback(
            args.gateway_host,
            candidates,
            args.client_id,
            args.probe_timeout,
        )
        if caps is not None and candidate is not None and selected_client_id is not None:
            print(f"{OK} account_type={caps.account_type}  "
                  f"n_subaccounts={caps.n_subaccounts}")
            print(f"  probe target: {candidate.label} ({args.gateway_host}:{candidate.port})")
            print(f"  probe clientId: {selected_client_id}")
            print(f"  capabilities written to {CAPABILITIES_PATH}")
        else:
            print(f"{WARN} probe skipped: no reachable candidate completed the IBKR probe")
            for line in attempted_errors:
                print(f"    {line}")
    elif args.skip_probe:
        print(f"{_DIM}── account probe skipped (--skip-probe) ──{_RESET}")

    print(f"\n{OK} IBKR setup complete. You can now run:")
    print(f"    python scripts/ibkr_bridge/bridge.py --config "
          f"scripts/ibkr_bridge/example_config.yaml")
    return 0


def cmd_revoke(args: argparse.Namespace) -> int:
    revoked = revoke_consent()
    if revoked:
        print(f"{OK} consent file removed: {CONSENT_PATH}")
    else:
        print(f"{WARN} no consent file to remove at {CONSENT_PATH}")

    # Capabilities file also goes
    if CAPABILITIES_PATH.exists():
        if args.keep_capabilities:
            print(f"{WARN} kept capabilities at {CAPABILITIES_PATH} "
                  f"(--keep-capabilities)")
        else:
            CAPABILITIES_PATH.unlink()
            print(f"{OK} capabilities file removed: {CAPABILITIES_PATH}")

    if args.clean_redis:
        try:
            redis = _load_redis_module()
            r = redis.Redis.from_url(args.redis_url, decode_responses=True)
            keys = list(r.scan_iter(match="ibkr:*"))
            if keys:
                r.delete(*keys)
                print(f"{OK} cleared {len(keys)} ibkr:* keys from Redis")
            else:
                print(f"{OK} no ibkr:* keys in Redis to clean")
        except ModuleNotFoundError as exc:
            print(f"{WARN} Redis clean skipped: {exc}")
        except redis.exceptions.RedisError as exc:
            print(f"{WARN} Redis clean skipped: {exc}")

    return 0


async def cmd_status(args: argparse.Namespace) -> int:
    print(f"{_DIM}── IBKR setup: status ──{_RESET}\n")

    if is_opted_in():
        ts = json.loads(CONSENT_PATH.read_text()).get("timestamp", "?")
        print(f"{OK} Consent: opted-in at {ts}")
    else:
        print(f"{FAIL} Consent: not opted-in (run `setup.py --enable`)")

    if CAPABILITIES_PATH.exists():
        try:
            caps = _load_ibkr_capabilities_class().load_or_default()
        except ModuleNotFoundError as exc:
            print(f"{WARN} Capabilities present but python deps are incomplete: {exc}")
        else:
            print(f"{OK} Capabilities at {CAPABILITIES_PATH}:")
            print(f"      account_type={caps.account_type}  "
                  f"n_subaccounts={caps.n_subaccounts}")
            print(f"      hist_min_interval={caps.hist_min_interval_sec:.2f}s  "
                  f"hist_window_cap={caps.hist_window_capacity_for_account()}  "
                  f"max_lines={caps.effective_max_lines()}")
    else:
        print(f"{WARN} Capabilities: not yet probed; defaults will apply")

    redis_ok, redis_msg = _ping_redis(args.redis_url)
    print(f"{OK if redis_ok else FAIL} Redis: {redis_msg}")

    candidates = _scan_gateway_candidates(args.gateway_host, args.gateway_port)
    _print_gateway_candidates(args.gateway_host, candidates)
    selected = _select_gateway_candidate(candidates)
    status_client_id = getattr(args, "client_id", PROBE_CLIENT_ID)
    if selected is not None:
        print(f"{OK} Selected candidate: {selected.label} ({args.gateway_host}:{selected.port})")
        print(f"      probe clientId policy: {candidate_client_ids(status_client_id)}")
    else:
        print(f"{WARN} No reachable IBKR TWS / Gateway candidate detected.")

    if redis_ok:
        try:
            redis = _load_redis_module()
        except ModuleNotFoundError:
            pass
        else:
            try:
                r = redis.Redis.from_url(args.redis_url, decode_responses=True)
                ibkr_keys = sum(1 for _ in r.scan_iter(match="ibkr:*", count=200))
                print(f"      ibkr:* keys in Redis: {ibkr_keys}")
            except redis.exceptions.RedisError:
                pass

    return 0 if (is_opted_in() and redis_ok) else 1


# ---------------------------------------------------------------------------
# Entry


def _build_parser() -> argparse.ArgumentParser:
    # Shared connection args — added both to the top-level parser and to
    # every subparser via parents=, so users can write either:
    #     setup --gateway-port 4002 enable
    #     setup enable --gateway-port 4002
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--redis-url", default=DEFAULT_REDIS_URL)
    common.add_argument("--gateway-host", default=DEFAULT_GATEWAY_HOST)
    common.add_argument(
        "--gateway-port", type=int, default=DEFAULT_GATEWAY_PORT,
        help=("If omitted, setup/status scan common local ports: "
              "TWS 7497 paper / 7496 live; "
              "Standalone IB Gateway 4002 paper / 4001 live"),
    )

    p = argparse.ArgumentParser(
        prog="ibkr_bridge.setup",
        parents=[common],
        description=("First-run setup for IBKR live data bridge. "
                     "Bilingual disclaimer + opt-in + environment checks."),
    )

    sub = p.add_subparsers(dest="action")
    # default action when no subcommand: enable
    p.set_defaults(action="enable")

    e = sub.add_parser("enable", parents=[common],
                        help="Run the full setup flow (consent, Redis, Gateway, probe)")
    e.add_argument("--client-id", type=int, default=PROBE_CLIENT_ID,
                    help="Throwaway clientId for the account probe")
    e.add_argument("--probe-timeout", type=float, default=DEFAULT_PROBE_TIMEOUT_SEC)
    e.add_argument("--skip-probe", action="store_true",
                    help="Skip the IBKR account probe (defer to first bridge run)")
    e.add_argument("--require-gateway", action="store_true",
                    help="Fail setup if Gateway/TWS is not reachable")

    r = sub.add_parser("revoke", parents=[common],
                       help="Delete consent + capabilities; optionally clean Redis")
    r.add_argument("--clean-redis", action="store_true",
                    help="Also wipe all ibkr:* keys from Redis")
    r.add_argument("--keep-capabilities", action="store_true",
                    help="Preserve ~/.ict-engine/ibkr_capabilities.json")

    sub.add_parser("status", parents=[common],
                    help="Print current consent / capabilities / Redis / Gateway")

    return p


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)

    if args.action == "enable":
        # `enable` defaults — fill missing fields if user invoked the bare CLI
        for attr, default in (("client_id", PROBE_CLIENT_ID),
                                ("probe_timeout", DEFAULT_PROBE_TIMEOUT_SEC),
                                ("skip_probe", False),
                                ("require_gateway", False)):
            if not hasattr(args, attr):
                setattr(args, attr, default)
        return asyncio.run(cmd_enable(args))
    if args.action == "revoke":
        for attr, default in (("clean_redis", False),
                                ("keep_capabilities", False)):
            if not hasattr(args, attr):
                setattr(args, attr, default)
        return cmd_revoke(args)
    if args.action == "status":
        return asyncio.run(cmd_status(args))

    parser.error(f"unknown action {args.action!r}")
    return 2  # unreachable


if __name__ == "__main__":
    raise SystemExit(main())
