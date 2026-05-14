from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

DEFAULT_AUXILIARY_FIELDS = [
    "qqq_hv_level",
    "nq_vs_200d_pct",
    "vix3m_level",
    "qqq_hv_pct_rank_252",
    "vvix_over_vix",
]

TARGET_FIELDS = [
    "schema_version",
    "symbol",
    "candidate_id",
    "entry_index",
    "entry_timestamp",
    "side",
    "realized_R",
    "mfe",
    "mae",
    "time_to_hit",
    "risk_adjusted_path_utility",
    "mae_penalty",
    "time_penalty",
    "regime_confidence_bonus",
    "slippage_penalty",
    "meta_label",
    "calibrated_label",
    "pending_reward_state",
    "payoff_gate",
    "dsr",
    "psr",
    "sharpe",
    "payoff_shape",
    "bbn_consume",
    *DEFAULT_AUXILIARY_FIELDS,
]


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=False) + "\n")


def _write_csv(path: Path, rows: list[dict[str, Any]], fields: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)


def _pending_reward_state(label: dict[str, Any]) -> str:
    return "matured_success" if int(label.get("meta_label", 0)) == 1 else "matured_failure"


def _float(row: dict[str, Any], key: str, default: float = 0.0) -> float:
    try:
        return float(row.get(key, default))
    except (TypeError, ValueError):
        return default


def _risk_adjusted_utility(label: dict[str, Any]) -> dict[str, float]:
    realized_r = _float(label, "realized_R")
    mae_penalty = abs(min(0.0, _float(label, "mae")))
    time_penalty = max(0.0, _float(label, "time_to_hit")) * 0.01
    regime_confidence_bonus = max(0.0, min(1.0, _float(label, "regime_confidence"))) * 0.10
    slippage_penalty = abs(_float(label, "slippage_R"))
    utility = realized_r - mae_penalty - time_penalty + regime_confidence_bonus - slippage_penalty
    return {
        "risk_adjusted_path_utility": round(utility, 6),
        "mae_penalty": round(mae_penalty, 6),
        "time_penalty": round(time_penalty, 6),
        "regime_confidence_bonus": round(regime_confidence_bonus, 6),
        "slippage_penalty": round(slippage_penalty, 6),
    }


def _target_row(
    *,
    label: dict[str, Any],
    report: dict[str, Any],
    symbol: str,
    auxiliary_fields: list[str],
) -> dict[str, Any]:
    row = {
        "schema_version": "payoff-path-ranker-target/v1",
        "symbol": symbol,
        "candidate_id": report["candidate_id"],
        "entry_index": label.get("entry_index", ""),
        "entry_timestamp": label.get("entry_timestamp", ""),
        "side": label.get("side", ""),
        "realized_R": label.get("realized_R", 0.0),
        "mfe": label.get("mfe", 0.0),
        "mae": label.get("mae", 0.0),
        "time_to_hit": label.get("time_to_hit", ""),
        **_risk_adjusted_utility(label),
        "meta_label": label.get("meta_label", 0),
        "calibrated_label": max(0.0, min(1.0, float(label.get("meta_label", 0)))),
        "pending_reward_state": _pending_reward_state(label),
        "payoff_gate": report.get("promotion_gate", "reject"),
        "dsr": report.get("dsr", 0.0),
        "psr": report.get("psr", 0.0),
        "sharpe": report.get("sharpe", 0.0),
        "payoff_shape": report.get("payoff_shape", "unknown"),
        "bbn_consume": True,
    }
    for field in auxiliary_fields:
        row[field] = label.get(field, "")
    return row


def build_target_row_for_test(label: dict[str, Any], report: dict[str, Any], symbol: str) -> dict[str, Any]:
    return _target_row(label=label, report=report, symbol=symbol, auxiliary_fields=DEFAULT_AUXILIARY_FIELDS)


def _bbn_gate(report: dict[str, Any]) -> dict[str, Any]:
    gate = str(report.get("promotion_gate", "reject"))
    return {
        "schema_version": "payoff-bbn-gate/v1",
        "candidate_id": report.get("candidate_id", ""),
        "payoff_gate": gate,
        "consume_by_regime_bbn": gate in {"probe", "promote"},
        "reason": "payoff_gate_allows_consumption" if gate in {"probe", "promote"} else "payoff_reject_failure_memory_only",
        "dsr": report.get("dsr", 0.0),
        "psr": report.get("psr", 0.0),
        "failure_tags": report.get("failure_tags", []),
    }


def _failure_memory(report: dict[str, Any], symbol: str) -> dict[str, Any]:
    return {
        "schema_version": "payoff-failure-memory/v1",
        "memory_type": "payoff_reject",
        "symbol": symbol,
        "candidate_id": report.get("candidate_id", ""),
        "payoff_gate": report.get("promotion_gate", "reject"),
        "dsr": report.get("dsr", 0.0),
        "psr": report.get("psr", 0.0),
        "sharpe": report.get("sharpe", 0.0),
        "failure_tags": report.get("failure_tags", []),
        "next_action": "do_not_route_to_regime_bbn_or_path_ranker",
    }


def export_targets(
    *,
    labels_jsonl: Path,
    payoff_report_json: Path,
    output_dir: Path,
    symbol: str,
    auxiliary_fields: list[str] | None = None,
) -> dict[str, Any]:
    report = _load_json(payoff_report_json)
    labels = _load_jsonl(labels_jsonl)
    fields = auxiliary_fields or DEFAULT_AUXILIARY_FIELDS
    gate = _bbn_gate(report)
    output_dir.mkdir(parents=True, exist_ok=True)

    target_rows: list[dict[str, Any]] = []
    artifact_paths: dict[str, str] = {}
    if gate["consume_by_regime_bbn"]:
        target_rows = [_target_row(label=label, report=report, symbol=symbol, auxiliary_fields=fields) for label in labels]
        csv_fields = [field for field in TARGET_FIELDS if field in target_rows[0]] if target_rows else TARGET_FIELDS
        csv_path = output_dir / "path_ranker_target.csv"
        jsonl_path = output_dir / "path_ranker_target.jsonl"
        _write_csv(csv_path, target_rows, csv_fields)
        _write_jsonl(jsonl_path, target_rows)
        artifact_paths["path_ranker_target_csv"] = str(csv_path)
        artifact_paths["path_ranker_target_jsonl"] = str(jsonl_path)
    else:
        failure_path = output_dir / "failure_memory.jsonl"
        _write_jsonl(failure_path, [_failure_memory(report, symbol)])
        artifact_paths["failure_memory_jsonl"] = str(failure_path)

    gate_path = output_dir / "bbn_gate.json"
    summary_path = output_dir / "path_ranker_handoff_summary.json"
    _write_json(gate_path, gate)
    summary = {
        "ok": True,
        "schema_version": "payoff-path-ranker-handoff/v1",
        "symbol": symbol,
        "candidate_id": report.get("candidate_id", ""),
        "bbn_gate": gate,
        "target_row_count": len(target_rows),
        "artifact_paths": {"bbn_gate": str(gate_path), **artifact_paths, "summary": str(summary_path)},
        "auxiliary_fields": fields,
    }
    _write_json(summary_path, summary)
    return summary


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Export payoff-gated path-ranker targets and BBN gate artifacts.")
    parser.add_argument("--labels-jsonl", required=True)
    parser.add_argument("--payoff-report-json", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--symbol", required=True)
    parser.add_argument("--auxiliary-fields", default=",".join(DEFAULT_AUXILIARY_FIELDS))
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    fields = [field.strip() for field in args.auxiliary_fields.split(",") if field.strip()]
    summary = export_targets(
        labels_jsonl=Path(args.labels_jsonl).resolve(),
        payoff_report_json=Path(args.payoff_report_json).resolve(),
        output_dir=Path(args.output_dir).resolve(),
        symbol=args.symbol,
        auxiliary_fields=fields,
    )
    print(json.dumps(summary, indent=2, sort_keys=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
