from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from statistics import mean, pstdev
from typing import Any

OPTIONAL_FIELDS = [
    "vix_level",
    "vix3m_level",
    "vvix_level",
    "vvix_over_vix",
    "qqq_hv_level",
    "qqq_hv_pct_rank_252",
    "nq_vs_200d_pct",
    "iv_rank",
    "hv_rank",
    "vrp",
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


def _rolling_hv_from_close(rows: list[dict[str, Any]], lookback: int) -> list[float | None]:
    closes = [_to_float(row.get("close")) for row in rows]
    rets: list[float | None] = [None]
    for prev, curr in zip(closes, closes[1:]):
        if prev is None or curr is None or prev == 0.0:
            rets.append(None)
        else:
            rets.append((curr / prev) - 1.0)
    out: list[float | None] = []
    for idx in range(len(rows)):
        window = [value for value in rets[max(0, idx - lookback + 1) : idx + 1] if value is not None]
        if len(window) < 2:
            out.append(None)
        else:
            out.append(pstdev(window) * (252**0.5))
    return out


def _rank_percentile(values: list[float], value: float) -> float:
    if not values:
        return 0.0
    return sum(1 for item in values if item <= value) / len(values)


def _field_status(row: dict[str, Any]) -> tuple[dict[str, str], list[str]]:
    status: dict[str, str] = {}
    missing: list[str] = []
    for field in OPTIONAL_FIELDS:
        if _to_float(row.get(field)) is None:
            status[field] = "missing_optional"
            missing.append(field)
        else:
            status[field] = "present"
    return status, missing


def build_vrp_sidecar(
    *,
    rows: list[dict[str, Any]],
    symbol: str = "NQ",
    hv_lookback: int = 20,
    rank_lookback: int = 252,
) -> dict[str, Any]:
    hv_series = _rolling_hv_from_close(rows, hv_lookback)
    artifacts: list[dict[str, Any]] = []
    all_missing: set[str] = set()
    for idx, row in enumerate(rows):
        hv_estimate = _to_float(row.get("qqq_hv_level"))
        if hv_estimate is None:
            hv_estimate = hv_series[idx]
        vix = _to_float(row.get("vix_level"))
        vix3m = _to_float(row.get("vix3m_level"))
        vvix_over_vix = _to_float(row.get("vvix_over_vix"))
        if vvix_over_vix is None:
            vvix = _to_float(row.get("vvix_level"))
            if vvix is not None and vix not in (None, 0.0):
                vvix_over_vix = vvix / vix
        implied_vol = vix3m or vix
        vrp = _to_float(row.get("vrp"))
        if vrp is None and implied_vol is not None and hv_estimate is not None:
            vrp = implied_vol - hv_estimate
        hv_window = [value for value in hv_series[max(0, idx - rank_lookback + 1) : idx + 1] if value is not None]
        hv_rank = _to_float(row.get("hv_rank"))
        if hv_rank is None and hv_estimate is not None:
            hv_rank = _rank_percentile(hv_window, hv_estimate)
        iv_rank = _to_float(row.get("iv_rank"))
        if iv_rank is None and implied_vol is not None:
            iv_window: list[float] = []
            for prior in rows[max(0, idx - rank_lookback + 1) : idx + 1]:
                implied = _to_float(prior.get("vix3m_level")) or _to_float(prior.get("vix_level"))
                if implied is not None:
                    iv_window.append(implied)
            iv_rank = _rank_percentile(iv_window, implied_vol)
        field_status, missing = _field_status(row)
        all_missing.update(missing)
        available_score_parts = [
            part
            for part in [
                vrp,
                vix3m,
                vvix_over_vix,
                hv_rank,
                iv_rank,
                _to_float(row.get("nq_vs_200d_pct")),
            ]
            if part is not None
        ]
        confidence = min(1.0, len(available_score_parts) / 6.0)
        pressure = 0.0
        if vrp is not None:
            pressure += vrp
        if iv_rank is not None and hv_rank is not None:
            pressure += iv_rank - hv_rank
        if vvix_over_vix is not None:
            pressure += vvix_over_vix * 0.1
        if _to_float(row.get("nq_vs_200d_pct")) is not None:
            pressure += (_to_float(row.get("nq_vs_200d_pct")) or 0.0) * 0.01
        artifacts.append(
            {
                "timestamp": row.get("timestamp") or row.get("date") or str(idx),
                "symbol": symbol,
                "candidate_id": "vrp_pressure_qqq_v1",
                "family": "options_hedging",
                "vrp_pressure": pressure,
                "confidence": confidence,
                "missing_optional_fields": missing,
                "optional_input_status": field_status,
                "features": {
                    "vix_level": vix,
                    "vix3m_level": vix3m,
                    "vvix_over_vix": vvix_over_vix,
                    "qqq_hv_level": hv_estimate,
                    "qqq_hv_pct_rank_252": _to_float(row.get("qqq_hv_pct_rank_252")),
                    "nq_vs_200d_pct": _to_float(row.get("nq_vs_200d_pct")),
                    "iv_rank": iv_rank,
                    "hv_rank": hv_rank,
                    "vrp": vrp,
                },
                "bbn_targets": ["dealer_pressure", "factor_uncertainty", "crash_risk"],
                "promotion_gate": "probe",
            }
        )
    return {
        "schema_version": "qqq-nq-vrp-sidecar/v1",
        "candidate_id": "vrp_pressure_qqq_v1",
        "symbol": symbol,
        "row_count": len(artifacts),
        "missing_optional_policy": "emit_missing_optional_and_continue",
        "missing_optional_fields": sorted(all_missing),
        "zero_config_fallback": True,
        "artifacts": artifacts,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build QQQ/NQ VRP pressure sidecar artifact.")
    parser.add_argument("--input-csv", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--symbol", default="NQ")
    parser.add_argument("--hv-lookback", type=int, default=20)
    parser.add_argument("--rank-lookback", type=int, default=252)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    payload = build_vrp_sidecar(
        rows=_load_csv(Path(args.input_csv)),
        symbol=args.symbol,
        hv_lookback=args.hv_lookback,
        rank_lookback=args.rank_lookback,
    )
    out = Path(args.output_json)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    print(json.dumps({"ok": True, "output": str(out), "row_count": payload["row_count"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
