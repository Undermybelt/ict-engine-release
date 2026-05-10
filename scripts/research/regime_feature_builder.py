from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from statistics import mean
from typing import Any

USER_AUX_FIELDS = [
    "qqq_hv_level",
    "nq_vs_200d_pct",
    "vix3m_level",
    "qqq_hv_pct_rank_252",
    "vvix_over_vix",
]

BASE_COLUMNS = ["timestamp", "open", "high", "low", "close", "volume"]


def _to_float(value: Any, default: float = 0.0) -> float:
    try:
        if value in (None, ""):
            return default
        return float(value)
    except (TypeError, ValueError):
        return default


def _load_rows(path: Path) -> list[dict[str, Any]]:
    suffix = path.suffix.lower()
    if suffix == ".csv":
        with path.open(newline="", encoding="utf-8") as handle:
            return [dict(row) for row in csv.DictReader(handle)]
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def _rows_by_timestamp(path: Path | None) -> dict[str, dict[str, Any]]:
    if path is None or not path.exists():
        return {}
    return {str(row.get("timestamp", "")): row for row in _load_rows(path) if row.get("timestamp") is not None}


def _percent_rank(values: list[float], index: int) -> float:
    if not values:
        return 0.0
    current = values[index]
    less_or_equal = sum(1 for value in values if value <= current)
    return less_or_equal / len(values)


def _rolling(values: list[float], index: int, window: int) -> list[float]:
    start = max(0, index - window + 1)
    return values[start : index + 1]


def _rsi(closes: list[float], index: int, window: int = 3) -> float:
    if index == 0:
        return 50.0
    gains: list[float] = []
    losses: list[float] = []
    start = max(1, index - window + 1)
    for cursor in range(start, index + 1):
        diff = closes[cursor] - closes[cursor - 1]
        if diff >= 0:
            gains.append(diff)
        else:
            losses.append(abs(diff))
    avg_gain = mean(gains) if gains else 0.0
    avg_loss = mean(losses) if losses else 0.0
    if avg_loss == 0.0:
        return 100.0 if avg_gain > 0.0 else 50.0
    rs = avg_gain / avg_loss
    return 100.0 - (100.0 / (1.0 + rs))


def _directional_efficiency(closes: list[float], index: int, window: int = 3) -> float:
    start = max(0, index - window + 1)
    if index <= start:
        return 0.0
    net = abs(closes[index] - closes[start])
    path = sum(abs(closes[cursor] - closes[cursor - 1]) for cursor in range(start + 1, index + 1))
    return net / path if path else 0.0


def _safe_div(numerator: float, denominator: float) -> float:
    return numerator / denominator if denominator else 0.0


def _compute_features(ohlcv_rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    opens = [_to_float(row.get("open")) for row in ohlcv_rows]
    highs = [_to_float(row.get("high")) for row in ohlcv_rows]
    lows = [_to_float(row.get("low")) for row in ohlcv_rows]
    closes = [_to_float(row.get("close")) for row in ohlcv_rows]
    volumes = [_to_float(row.get("volume")) for row in ohlcv_rows]
    ranges = [max(0.0, high - low) for high, low in zip(highs, lows)]
    true_ranges = []
    for index, candle_range in enumerate(ranges):
        if index == 0:
            true_ranges.append(candle_range)
        else:
            true_ranges.append(max(candle_range, abs(highs[index] - closes[index - 1]), abs(lows[index] - closes[index - 1])))

    rows: list[dict[str, Any]] = []
    for index, row in enumerate(ohlcv_rows):
        window_ranges = _rolling(true_ranges, index, 3)
        window_closes = _rolling(closes, index, 3)
        high_window = max(_rolling(highs, index, 5))
        low_window = min(_rolling(lows, index, 5))
        candle_range = ranges[index]
        body = abs(closes[index] - opens[index])
        ret = 0.0 if index == 0 else _safe_div(closes[index] - closes[index - 1], closes[index - 1])
        slope = closes[index] - window_closes[0] if window_closes else 0.0
        feature_row: dict[str, Any] = {
            "timestamp": str(row.get("timestamp", index)),
            "open": opens[index],
            "high": highs[index],
            "low": lows[index],
            "close": closes[index],
            "volume": volumes[index],
            "return_1": ret,
            "candle_range": candle_range,
            "body_pct": _safe_div(body, candle_range),
            "upper_wick_pct": _safe_div(highs[index] - max(opens[index], closes[index]), candle_range),
            "lower_wick_pct": _safe_div(min(opens[index], closes[index]) - lows[index], candle_range),
            "atr_3": mean(window_ranges) if window_ranges else 0.0,
            "atr_percentile": _percent_rank(true_ranges, index),
            "volume_percentile": _percent_rank(volumes, index),
            "directional_efficiency_3": _directional_efficiency(closes, index, 3),
            "slope_3": slope,
            "range_position": _safe_div(closes[index] - low_window, high_window - low_window),
            "rsi_3": _rsi(closes, index, 3),
            "realized_vol_3": mean(abs(closes[cursor] - closes[cursor - 1]) for cursor in range(max(1, index - 2), index + 1)) if index else 0.0,
            "mtf_alignment": "missing",
            "pda_event_count": 0,
        }
        rows.append(feature_row)
    return rows


def _join_optional(rows: list[dict[str, Any]], aux: dict[str, dict[str, Any]], mtf: dict[str, dict[str, Any]]) -> None:
    for row in rows:
        timestamp = str(row["timestamp"])
        aux_row = aux.get(timestamp, {})
        mtf_row = mtf.get(timestamp, {})
        for field in USER_AUX_FIELDS:
            if field in aux_row:
                row[field] = aux_row[field]
            else:
                row.setdefault(field, "")
        if "mtf_alignment" in mtf_row:
            row["mtf_alignment"] = mtf_row["mtf_alignment"]
        if "pda_event_count" in mtf_row:
            row["pda_event_count"] = mtf_row["pda_event_count"]


def _write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fieldnames: list[str] = []
    for row in rows:
        for key in row:
            if key not in fieldnames:
                fieldnames.append(key)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _quality_report(
    rows: list[dict[str, Any]],
    *,
    auxiliary_path: Path | None,
    mtf_path: Path | None,
) -> dict[str, Any]:
    columns = sorted({key for row in rows for key in row})
    missing_optional = [field for field in USER_AUX_FIELDS if all(row.get(field, "") in ("", None) for row in rows)]
    return {
        "schema_version": "regime-feature-builder/v1",
        "row_count": len(rows),
        "column_count": len(columns),
        "columns": columns,
        "feature_groups": [
            "price_geometry",
            "volatility",
            "liquidity",
            "structure_ict",
            "behavior_crowding",
            "distribution_shape",
            "mtf_resonance",
            "transition_history",
        ],
        "missing_optional_fields": missing_optional,
        "optional_input_status": {
            "auxiliary_evidence": "present" if auxiliary_path and auxiliary_path.exists() else "missing",
            "mtf_pda_events": "present" if mtf_path and mtf_path.exists() else "missing",
        },
    }


def build_feature_artifacts(
    *,
    ohlcv_path: Path,
    output_features: Path,
    output_report: Path,
    auxiliary_path: Path | None = None,
    mtf_pda_events_path: Path | None = None,
) -> dict[str, Any]:
    ohlcv_rows = _load_rows(ohlcv_path)
    features = _compute_features(ohlcv_rows)
    _join_optional(features, _rows_by_timestamp(auxiliary_path), _rows_by_timestamp(mtf_pda_events_path))
    _write_csv(output_features, features)
    report = _quality_report(features, auxiliary_path=auxiliary_path, mtf_path=mtf_pda_events_path)
    _write_json(output_report, report)
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build regime expert feature table from OHLCV plus optional evidence.")
    parser.add_argument("--ohlcv", required=True)
    parser.add_argument("--auxiliary-evidence")
    parser.add_argument("--mtf-pda-events")
    parser.add_argument("--output-features", required=True)
    parser.add_argument("--output-report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_feature_artifacts(
        ohlcv_path=Path(args.ohlcv),
        auxiliary_path=Path(args.auxiliary_evidence) if args.auxiliary_evidence else None,
        mtf_pda_events_path=Path(args.mtf_pda_events) if args.mtf_pda_events else None,
        output_features=Path(args.output_features),
        output_report=Path(args.output_report),
    )
    print(json.dumps({"ok": True, "row_count": report["row_count"], "output": args.output_features}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())