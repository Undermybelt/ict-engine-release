"""One-shot static identification of an IBKR account.

Connects briefly to TWS / IB Gateway via ``ib_async`` to learn:

* whether this is a paper or live account (port + managed-acct prefix)
* whether this is a single account or a Financial Advisor / multi-account
* the count of subaccounts (used to scale historical pacing)

The result is written to :class:`IbkrCapabilities` (which the rate limiter
reads). The probe consumes essentially no quota — only `reqManagedAccts`,
which is metadata, not market data.

The probe should run **once** per account / per setup. ``setup.py`` invokes
it from the consent flow. ``bridge.py`` re-runs it on first start if
``capabilities.account_type == "unknown"``.
"""

from __future__ import annotations

import asyncio
from dataclasses import asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Literal

from ib_async import IB

from .rate_limiter import (
    CAPABILITIES_PATH,
    AccountType,
    IbkrCapabilities,
)

PROBE_CLIENT_ID = 99  # throwaway id; very unlikely to clash with bridge (20)
DEFAULT_PROBE_TIMEOUT_SEC = 8.0


async def probe_account(host: str = "127.0.0.1",
                         port: int = 7497,
                         client_id: int = PROBE_CLIENT_ID,
                         timeout: float = DEFAULT_PROBE_TIMEOUT_SEC,
                         capabilities_path: Path = CAPABILITIES_PATH,
                         persist: bool = True
                         ) -> IbkrCapabilities:
    """Connect, read managed accounts, write capabilities, disconnect.

    Idempotent: if the capabilities file already exists, its adaptive fields
    are preserved; only static fields (account_type, n_subaccounts) are
    rewritten with fresh probe data.
    """
    caps = IbkrCapabilities.load_or_default(capabilities_path)

    ib = IB()
    try:
        await asyncio.wait_for(
            ib.connectAsync(host=host, port=port, clientId=client_id,
                             readonly=True),
            timeout=timeout,
        )
    except asyncio.TimeoutError as exc:
        raise ConnectionError(
            f"IBKR Gateway probe timed out connecting to {host}:{port} "
            f"(clientId={client_id}). Is IB Gateway / TWS running and "
            f"`Enable ActiveX and Socket Clients` toggled on?"
        ) from exc
    except OSError as exc:
        raise ConnectionError(
            f"Cannot reach IBKR Gateway at {host}:{port}: {exc}"
        ) from exc

    try:
        managed = list(ib.managedAccounts() or [])
        if not managed:
            # Some early-connection states return empty; wait briefly.
            await asyncio.sleep(1.0)
            managed = list(ib.managedAccounts() or [])
    finally:
        ib.disconnect()

    if not managed:
        raise RuntimeError(
            "IBKR returned no managed accounts. Make sure your account is "
            "fully logged in (Gateway 'Connection Status: Connected') and "
            "API is enabled in Settings → API → Settings."
        )

    caps.account_type = _classify(managed, port)
    caps.n_subaccounts = len(managed)
    if not caps.first_seen:
        caps.first_seen = datetime.now(timezone.utc).isoformat()

    if persist:
        caps.save(capabilities_path)

    return caps


def _classify(managed: list[str], port: int) -> AccountType:
    """Infer account type from managed-account ID prefixes and port.

    Conventions (subject to IBKR change, but stable since at least 2018):
        DU* → paper trading
        U*  → universal individual / Pro
        DF* → FAdvisor demo
        F*  → FAdvisor live (master)
        I*  → institutional
    """
    if port == 7497:
        # Paper port — even if prefixes match live-style codes the session
        # is paper.
        return "paper"
    prefixes = {a[:1] for a in managed if a}
    if any(a.startswith("DU") for a in managed):
        return "paper"
    if any(a.startswith("DF") for a in managed):
        return "paper"   # FA demo
    if any(a.startswith("F") for a in managed) and len(managed) > 1:
        return "fa"
    if any(a.startswith("I") for a in managed):
        return "institutional"
    if "U" in prefixes:
        return "live_pro"
    return "unknown"


# ---------------------------------------------------------------------------
# CLI entry point


async def _async_cli(args) -> int:
    try:
        caps = await probe_account(host=args.host, port=args.port,
                                    client_id=args.client_id,
                                    timeout=args.timeout,
                                    capabilities_path=Path(args.capabilities))
    except (ConnectionError, RuntimeError) as exc:
        print(f"probe failed: {exc}")
        return 2
    print(f"account_type:    {caps.account_type}")
    print(f"n_subaccounts:   {caps.n_subaccounts}")
    print(f"capabilities:    {args.capabilities}")
    print(f"hist_window cap: {caps.hist_window_capacity_for_account()} (effective)")
    return 0


def _cli() -> int:
    import argparse
    p = argparse.ArgumentParser(description="One-shot IBKR account identification probe")
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=7497, help="7497 paper, 7496 live")
    p.add_argument("--client-id", type=int, default=PROBE_CLIENT_ID)
    p.add_argument("--timeout", type=float, default=DEFAULT_PROBE_TIMEOUT_SEC)
    p.add_argument("--capabilities", default=str(CAPABILITIES_PATH))
    args = p.parse_args()
    return asyncio.run(_async_cli(args))


if __name__ == "__main__":
    raise SystemExit(_cli())
