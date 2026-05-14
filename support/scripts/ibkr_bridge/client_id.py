"""Shared clientId fallback helpers for IBKR connections.

These helpers are deliberately pure / lightweight so they can be imported by
setup, bridge, and one-shot fetchers without pulling in ib_async or Redis.
"""

from __future__ import annotations

import contextlib
from collections.abc import Iterable

DEFAULT_FALLBACK_OFFSET = 100
DEFAULT_FALLBACK_COUNT = 5


def candidate_client_ids(
    preferred: int,
    *,
    fallback_offset: int = DEFAULT_FALLBACK_OFFSET,
    fallback_count: int = DEFAULT_FALLBACK_COUNT,
) -> list[int]:
    out = [preferred]
    out.extend(preferred + fallback_offset + index for index in range(fallback_count))
    return out


def is_client_id_conflict_error(exc: BaseException | str) -> bool:
    text = str(exc).lower()
    return (
        "client id is already in use" in text
        or "clientid is already in use" in text
        or "client id already in use" in text
        or ("326" in text and "client" in text and "use" in text)
    )


async def connect_with_client_id_fallback(
    ib,
    *,
    host: str,
    port: int,
    preferred_client_id: int,
    readonly: bool = True,
    candidates: Iterable[int] | None = None,
) -> tuple[int, list[tuple[int, str]]]:
    attempted_conflicts: list[tuple[int, str]] = []
    for client_id in list(candidates or candidate_client_ids(preferred_client_id)):
        try:
            await ib.connectAsync(
                host=host,
                port=port,
                clientId=client_id,
                readonly=readonly,
            )
            return client_id, attempted_conflicts
        except Exception as exc:  # noqa: BLE001
            with contextlib.suppress(Exception):
                if ib.isConnected():
                    ib.disconnect()
            if not is_client_id_conflict_error(exc):
                raise
            attempted_conflicts.append((client_id, str(exc)))
    raise RuntimeError(
        "all candidate clientIds are already in use: "
        + ", ".join(str(client_id) for client_id, _ in attempted_conflicts)
    )
