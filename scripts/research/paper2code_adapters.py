from __future__ import annotations

import argparse
import json
import statistics
from pathlib import Path
from typing import Any


def _load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _float(row: dict[str, Any], key: str, default: float = 0.0) -> float:
    try:
        return float(row.get(key, default))
    except (TypeError, ValueError):
        return default


def _clamp(value: float, low: float = 0.0, high: float = 1.0) -> float:
    return max(low, min(high, value))


def _series(rows: list[dict[str, Any]], key: str) -> list[float]:
    return [_float(row, key) for row in rows]


def _returns(rows: list[dict[str, Any]]) -> list[float]:
    closes = _series(rows, "close")
    return [closes[i] - closes[i - 1] for i in range(1, len(closes))]


def _mean_abs(values: list[float]) -> float:
    return sum(abs(value) for value in values) / len(values) if values else 0.0


def _rammstein_ou(rows: list[dict[str, Any]]) -> dict[str, Any]:
    closes = _series(rows, "close")
    if len(closes) < 2:
        score = 0.0
    else:
        mean_close = statistics.fmean(closes)
        deviations = [close - mean_close for close in closes]
        directional_reverts = sum(1 for prev, cur in zip(deviations, deviations[1:]) if prev * cur < 0)
        score = directional_reverts / max(1, len(deviations) - 1)
    return {
        "adapter_id": "rammstein_ou_reversion",
        "paper_family": "ou_reversion_feasibility",
        "edge_score": round(score, 6),
        "risk_score": round(1.0 - score, 6),
        "bbn_evidence_hint": "ou_reversion_feasibility",
    }


def _crowded_trades(rows: list[dict[str, Any]]) -> dict[str, Any]:
    volumes = _series(rows, "volume")
    rets = _returns(rows)
    mean_volume = statistics.fmean(volumes) if volumes else 0.0
    volume_spike = max(volumes) / mean_volume - 1.0 if mean_volume else 0.0
    one_way_pressure = abs(sum(1 if ret > 0 else -1 if ret < 0 else 0 for ret in rets)) / max(1, len(rets))
    risk = _clamp((volume_spike * 0.5) + (one_way_pressure * 0.5))
    return {
        "adapter_id": "crowded_trades_pressure",
        "paper_family": "crowding_pressure",
        "edge_score": round(1.0 - risk, 6),
        "risk_score": round(risk, 6),
        "bbn_evidence_hint": "crowding_execution_risk",
    }


def _kyle_liquidity(rows: list[dict[str, Any]]) -> dict[str, Any]:
    ranges = [_float(row, "high") - _float(row, "low") for row in rows]
    signed_volume = _series(rows, "signed_volume")
    spreads = _series(rows, "spread")
    avg_volume_impact = _mean_abs(signed_volume) / 1000.0
    avg_range = statistics.fmean(ranges) if ranges else 0.0
    avg_spread = statistics.fmean(spreads) if spreads else 0.0
    risk = _clamp((avg_spread * 1.5) + (avg_range * 0.05) + (avg_volume_impact * 0.25))
    return {
        "adapter_id": "kyle_liquidity_slippage",
        "paper_family": "liquidity_slippage_realism",
        "edge_score": round(1.0 - risk, 6),
        "risk_score": round(risk, 6),
        "bbn_evidence_hint": "liquidity_slippage_risk",
    }


def _red_queens(rows: list[dict[str, Any]]) -> dict[str, Any]:
    rets = _returns(rows)
    churn = sum(1 for prev, cur in zip(rets, rets[1:]) if prev * cur < 0) / max(1, len(rets) - 1)
    low_progress = 1.0 - _clamp(abs(sum(rets)) / max(1.0, _mean_abs(rets) * max(1, len(rets))))
    friction = _clamp((churn + low_progress) * 0.5)
    return {
        "adapter_id": "red_queens_friction",
        "paper_family": "friction_mode_collapse",
        "edge_score": round(1.0 - friction, 6),
        "risk_score": round(friction, 6),
        "bbn_evidence_hint": "friction_mode_collapse_risk",
    }


def build_adapter_report(rows: list[dict[str, Any]], candidate_id: str = "") -> dict[str, Any]:
    adapters = [_rammstein_ou(rows), _crowded_trades(rows), _kyle_liquidity(rows), _red_queens(rows)]
    max_risk = max((adapter["risk_score"] for adapter in adapters), default=0.0)
    if max_risk >= 0.8:
        execution_hint = "reject"
    elif max_risk >= 0.5:
        execution_hint = "watch"
    else:
        execution_hint = "probe"
    return {
        "schema_version": "paper2code-adapter-report/v1",
        "candidate_id": candidate_id,
        "adapter_count": len(adapters),
        "execution_hint": execution_hint,
        "max_risk_score": round(max_risk, 6),
        "adapters": adapters,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run sidecar paper2code adapter reports over market rows.")
    parser.add_argument("--rows-jsonl", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--candidate-id", default="")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_adapter_report(rows=_load_jsonl(Path(args.rows_jsonl)), candidate_id=args.candidate_id)
    _write_json(Path(args.output_json), report)
    print(json.dumps({"ok": True, "output": args.output_json, "execution_hint": report["execution_hint"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())