from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from statistics import mean, pstdev
from typing import Any

OPTIONAL_L2_FIELDS = [
    "bid_depth",
    "ask_depth",
    "bid_size_1",
    "ask_size_1",
    "signed_trade_volume",
    "buy_volume",
    "sell_volume",
    "spread",
    "session",
]


def _to_float(value: Any) -> float | None:
    if value in (None, ""):
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _load_csv(path: Path) -> list[dict[str, Any]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def _status(row: dict[str, Any]) -> tuple[dict[str, str], list[str]]:
    status: dict[str, str] = {}
    missing: list[str] = []
    for field in OPTIONAL_L2_FIELDS:
        if row.get(field) in (None, ""):
            status[field] = "missing_optional"
            missing.append(field)
        else:
            status[field] = "present"
    return status, missing


def _ret(prev: float | None, curr: float | None) -> float | None:
    if prev in (None, 0.0) or curr is None:
        return None
    return curr / prev - 1.0


def _rolling_z(values: list[float], value: float) -> float:
    if len(values) < 2:
        return 0.0
    sigma = pstdev(values)
    if sigma == 0.0:
        return 0.0
    return (value - mean(values)) / sigma


def _session_quality(row: dict[str, Any], volume_z: float, range_z: float, spread_proxy: float) -> float:
    session = str(row.get("session") or "").lower()
    session_bonus = 0.1 if session in {"ny_am", "ny", "london"} else 0.0
    return max(0.0, min(1.0, 0.45 + 0.15 * volume_z - 0.10 * abs(range_z) - spread_proxy + session_bonus))


def build_ofi_session_sidecar(
    *,
    rows: list[dict[str, Any]],
    symbol: str = "NQ",
    lookback: int = 20,
) -> dict[str, Any]:
    closes = [_to_float(row.get("close")) for row in rows]
    volumes = [_to_float(row.get("volume")) or 0.0 for row in rows]
    ranges = [
        max(0.0, (_to_float(row.get("high")) or 0.0) - (_to_float(row.get("low")) or 0.0))
        for row in rows
    ]
    artifacts: list[dict[str, Any]] = []
    all_missing: set[str] = set()
    for idx, row in enumerate(rows):
        status, missing = _status(row)
        all_missing.update(missing)
        bid_depth = _to_float(row.get("bid_depth"))
        ask_depth = _to_float(row.get("ask_depth"))
        if bid_depth is None:
            bid_depth = _to_float(row.get("bid_size_1"))
        if ask_depth is None:
            ask_depth = _to_float(row.get("ask_size_1"))
        signed_trade_volume = _to_float(row.get("signed_trade_volume"))
        buy_volume = _to_float(row.get("buy_volume"))
        sell_volume = _to_float(row.get("sell_volume"))
        if signed_trade_volume is None and buy_volume is not None and sell_volume is not None:
            signed_trade_volume = buy_volume - sell_volume
        depth_imbalance = None
        if bid_depth is not None and ask_depth is not None and bid_depth + ask_depth != 0.0:
            depth_imbalance = (bid_depth - ask_depth) / (bid_depth + ask_depth)
        prev_close = closes[idx - 1] if idx else None
        ret = _ret(prev_close, closes[idx])
        proxy_sign = 0.0
        if ret is not None:
            proxy_sign = 1.0 if ret > 0 else -1.0 if ret < 0 else 0.0
        flow_component = 0.0
        if signed_trade_volume is not None:
            denom = max(1.0, volumes[idx])
            flow_component = signed_trade_volume / denom
        elif ret is not None:
            flow_component = proxy_sign * min(1.0, abs(ret) * 100.0)
        if depth_imbalance is not None:
            pressure = 0.65 * depth_imbalance + 0.35 * flow_component
            confidence = 0.9 if signed_trade_volume is not None else 0.75
            fallback_mode = "l2_trade_flow"
        else:
            pressure = flow_component
            confidence = 0.35 if ret is not None else 0.1
            fallback_mode = "ohlcv_proxy_low_confidence"
        start = max(0, idx - lookback + 1)
        volume_z = _rolling_z(volumes[start : idx + 1], volumes[idx])
        range_z = _rolling_z(ranges[start : idx + 1], ranges[idx])
        spread = _to_float(row.get("spread"))
        spread_proxy = spread if spread is not None else max(0.0, ranges[idx] / max(1.0, closes[idx] or 1.0))
        session_quality = _session_quality(row, volume_z, range_z, spread_proxy)
        artifacts.append(
            {
                "timestamp": row.get("timestamp") or row.get("date") or str(idx),
                "symbol": symbol,
                "candidate_id": "ofi_book_pressure_v1",
                "family": "crowding_herding/session_liquidity",
                "ofi_pressure": pressure,
                "depth_imbalance": depth_imbalance,
                "flow_component": flow_component,
                "session_quality": session_quality,
                "confidence": confidence,
                "fallback_mode": fallback_mode,
                "missing_optional_fields": missing,
                "optional_input_status": status,
                "bbn_targets": ["crowding_pressure", "liquidity_context", "session_quality"],
                "execution_tree_targets": ["fill_viable", "block_crowded"],
                "promotion_gate": "probe",
            }
        )
    return {
        "schema_version": "ofi-session-sidecar/v1",
        "candidate_id": "ofi_book_pressure_v1",
        "symbol": symbol,
        "row_count": len(artifacts),
        "missing_optional_policy": "emit_missing_optional_and_continue",
        "missing_optional_fields": sorted(all_missing),
        "zero_config_fallback": True,
        "artifacts": artifacts,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build OFI/session liquidity sidecar artifact.")
    parser.add_argument("--input-csv", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--symbol", default="NQ")
    parser.add_argument("--lookback", type=int, default=20)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    payload = build_ofi_session_sidecar(
        rows=_load_csv(Path(args.input_csv)),
        symbol=args.symbol,
        lookback=args.lookback,
    )
    out = Path(args.output_json)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    print(json.dumps({"ok": True, "output": str(out), "row_count": payload["row_count"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
