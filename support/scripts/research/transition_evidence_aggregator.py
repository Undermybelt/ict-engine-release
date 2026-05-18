from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

DEFAULT_THRESHOLDS = {
    "alert_transition_prob": 0.95,
    "block_transition_hazard": 0.8,
    "watch_transition_hazard": 0.35,
    "max_stable_transition_prob": 0.2,
    "max_stable_flip_rate": 0.2,
}


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_jsonl(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
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


def _drift_flags(drift_rows: list[dict[str, Any]], alert_threshold: float) -> list[str]:
    flags: list[str] = []
    for row in drift_rows:
        source = str(row.get("source", "drift")).strip() or "drift"
        if bool(row.get("drift_flag", False)) or _float(row, "transition_prob") >= alert_threshold:
            flags.append(source)
    return sorted(set(flags))


def _hazard(regime_report: dict[str, Any], drift_rows: list[dict[str, Any]]) -> float:
    values = [
        _float(regime_report, "transition_prob"),
        _float(regime_report, "flip_rate"),
    ]
    for row in drift_rows:
        values.append(_float(row, "transition_prob"))
        values.append(_float(row, "severity"))
    return max(values) if values else 0.0


def build_transition_evidence(
    *,
    regime_report: dict[str, Any],
    drift_rows: list[dict[str, Any]] | None = None,
    thresholds: dict[str, float] | None = None,
) -> dict[str, Any]:
    merged = dict(DEFAULT_THRESHOLDS)
    if thresholds:
        merged.update(thresholds)
    rows = drift_rows or []
    hazard = _hazard(regime_report, rows)
    flags = _drift_flags(rows, float(merged["alert_transition_prob"]))
    regime_confidence_ok = bool(regime_report.get("confidence_95", False))
    regime_gate = str(regime_report.get("regime_confidence_gate", ""))
    transition_alert_95 = hazard >= float(merged["alert_transition_prob"]) or (
        not regime_confidence_ok and hazard >= float(merged["block_transition_hazard"])
    )
    if transition_alert_95:
        block_hint = "transition_guardrail"
    elif not regime_confidence_ok or regime_gate == "reject" or hazard >= float(merged["watch_transition_hazard"]):
        block_hint = "watch_transition"
    else:
        block_hint = "none"

    return {
        "schema_version": "transition-evidence-aggregator/v1",
        "candidate_id": regime_report.get("candidate_id", ""),
        "transition_alert_95": transition_alert_95,
        "transition_hazard": hazard,
        "drift_flags": flags,
        "execution_tree_block_hint": block_hint,
        "regime_confidence_gate": regime_gate,
        "regime_confidence_95": regime_confidence_ok,
        "source_count": len(rows),
        "thresholds": merged,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Aggregate regime confidence and drift rows into transition evidence.")
    parser.add_argument("--regime-report-json", required=True)
    parser.add_argument("--drift-jsonl")
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--alert-transition-prob", type=float, default=DEFAULT_THRESHOLDS["alert_transition_prob"])
    parser.add_argument("--block-transition-hazard", type=float, default=DEFAULT_THRESHOLDS["block_transition_hazard"])
    parser.add_argument("--watch-transition-hazard", type=float, default=DEFAULT_THRESHOLDS["watch_transition_hazard"])
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    result = build_transition_evidence(
        regime_report=_load_json(Path(args.regime_report_json)),
        drift_rows=_load_jsonl(Path(args.drift_jsonl)) if args.drift_jsonl else [],
        thresholds={
            "alert_transition_prob": args.alert_transition_prob,
            "block_transition_hazard": args.block_transition_hazard,
            "watch_transition_hazard": args.watch_transition_hazard,
        },
    )
    _write_json(Path(args.output_json), result)
    print(json.dumps({"ok": True, "output": args.output_json, "execution_tree_block_hint": result["execution_tree_block_hint"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
