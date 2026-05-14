from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


TARGET_COVERAGES = [0.95, 0.99]
UNKNOWN_MARKERS = ("Unknown", "Neutral", "Transitional")


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_jsonl(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _truth_by_timestamp(truth_rows: list[dict[str, Any]]) -> dict[str, str]:
    mapping: dict[str, str] = {}
    for row in truth_rows:
        timestamp = str(row.get("timestamp", ""))
        label = str(row.get("label_id", row.get("label", row.get("primary_label", ""))))
        if timestamp and label:
            mapping[timestamp] = label
    return mapping


def _label_contracts(training_report: dict[str, Any], label_prefix: str = "") -> dict[str, dict[str, Any]]:
    contracts: dict[str, dict[str, Any]] = {}
    for expert in training_report.get("experts", []):
        label_id = str(expert.get("label_id", ""))
        if label_prefix and not label_id.startswith(label_prefix):
            continue
        policy = str(expert.get("abstain_policy", ""))
        always_abstain = policy == "always_abstain" or any(marker in label_id for marker in UNKNOWN_MARKERS)
        contracts[label_id] = {
            "abstain_policy": policy,
            "threshold": expert.get("threshold", 1.0 if always_abstain else 0.8),
            "trade_usable": not always_abstain,
        }
    return contracts


def _group_scores_by_timestamp(score_rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in score_rows:
        grouped[str(row.get("timestamp", ""))].append(row)
    return grouped


def _conformal_set(rows: list[dict[str, Any]], contracts: dict[str, dict[str, Any]], target_coverage: float) -> list[str]:
    # Deterministic split-conformal proxy: higher coverage relaxes threshold slightly.
    relax = 0.04 if target_coverage >= 0.99 else 0.0
    labels: list[str] = []
    for row in rows:
        label_id = str(row.get("label_id", ""))
        if label_id not in contracts:
            continue
        contract = contracts[label_id]
        if not contract.get("trade_usable", True):
            continue
        threshold = float(contract.get("threshold", row.get("threshold", 0.8))) - relax
        if float(row.get("score", 0.0)) >= threshold:
            labels.append(label_id)
    if labels:
        return sorted(labels)
    best = [row for row in rows if str(row.get("label_id", "")) in contracts and contracts[str(row.get("label_id", ""))].get("trade_usable", True)]
    if not best:
        return []
    return [str(max(best, key=lambda item: float(item.get("score", 0.0))).get("label_id", ""))]


def _class_coverage(
    conformal_sets: dict[str, list[str]],
    truth_map: dict[str, str],
) -> dict[str, dict[str, Any]]:
    totals: dict[str, int] = defaultdict(int)
    hits: dict[str, int] = defaultdict(int)
    for timestamp, truth in truth_map.items():
        totals[truth] += 1
        if truth in conformal_sets.get(timestamp, []):
            hits[truth] += 1
    return {
        label: {"coverage": round(hits[label] / total, 6) if total else 0.0, "support": total}
        for label, total in sorted(totals.items())
    }


def _all_truth_classes_meet_coverage(class_cov: dict[str, dict[str, Any]], target_coverage: float) -> bool:
    if not class_cov:
        return False
    return all(float(stats.get("coverage", 0.0)) >= target_coverage for stats in class_cov.values())


def build_conformal_calibration_report(
    *,
    scores_path: Path,
    training_report_path: Path,
    output_json: Path,
    truth_path: Path | None = None,
    target_coverages: list[float] | None = None,
    label_prefix: str = "",
) -> dict[str, Any]:
    score_rows = _load_jsonl(scores_path)
    training_report = _load_json(training_report_path)
    truth_map = _truth_by_timestamp(_load_jsonl(truth_path))
    coverages = target_coverages or TARGET_COVERAGES
    contracts = _label_contracts(training_report, label_prefix=label_prefix)
    grouped = _group_scores_by_timestamp(score_rows)

    sets_by_coverage: dict[str, dict[str, list[str]]] = {}
    for coverage in coverages:
        sets_by_coverage[str(coverage)] = {
            timestamp: _conformal_set(rows, contracts, coverage)
            for timestamp, rows in grouped.items()
        }

    primary_sets = sets_by_coverage[str(max(coverages))] if coverages else {}
    set_sizes = [len(labels) for labels in primary_sets.values()]
    singleton_rate = sum(1 for size in set_sizes if size == 1) / len(set_sizes) if set_sizes else 0.0
    max_set_size = max(set_sizes) if set_sizes else 0
    class_cov = _class_coverage(primary_sets, truth_map)
    has_truth = bool(truth_map)
    overall_coverage = 0.0
    warnings: list[str] = []
    if has_truth:
        overall_coverage = sum(1 for ts, truth in truth_map.items() if truth in primary_sets.get(ts, [])) / len(truth_map)
    else:
        warnings.append("truth_labels_missing")
    class_coverage_95 = _all_truth_classes_meet_coverage(class_cov, 0.95)
    class_coverage_99 = _all_truth_classes_meet_coverage(class_cov, 0.99)

    report = {
        "schema_version": "regime-conformal-calibration/v1",
        "target_coverages": coverages,
        "label_prefix": label_prefix,
        "truth_source": "provided" if has_truth else "missing",
        "row_count": len(grouped),
        "score_row_count": len(score_rows),
        "singleton_rate": round(singleton_rate, 6),
        "max_conformal_set_size": max_set_size,
        "average_conformal_set_size": round(sum(set_sizes) / len(set_sizes), 6) if set_sizes else 0.0,
        "overall_coverage": round(overall_coverage, 6),
        "class_conditional_coverage": class_cov,
        "confidence_95": bool(has_truth and overall_coverage >= 0.95 and class_coverage_95 and singleton_rate == 1.0 and max_set_size == 1),
        "confidence_99": bool(has_truth and overall_coverage >= 0.99 and class_coverage_99 and singleton_rate == 1.0 and max_set_size == 1),
        "sets_by_target_coverage": sets_by_coverage,
        "label_contracts": contracts,
        "warnings": warnings,
    }
    _write_json(output_json, report)
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build class-conditional conformal calibration report from regime expert scores.")
    parser.add_argument("--scores", required=True)
    parser.add_argument("--training-report", required=True)
    parser.add_argument("--truth")
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--target-coverage", action="append", type=float)
    parser.add_argument("--label-prefix", default="", help="Optional hot-plug label scope, e.g. primary:: or volatility::")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_conformal_calibration_report(
        scores_path=Path(args.scores),
        training_report_path=Path(args.training_report),
        truth_path=Path(args.truth) if args.truth else None,
        output_json=Path(args.output_json),
        target_coverages=args.target_coverage,
        label_prefix=args.label_prefix,
    )
    print(json.dumps({"ok": True, "output": args.output_json, "confidence_95": report["confidence_95"], "confidence_99": report["confidence_99"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
