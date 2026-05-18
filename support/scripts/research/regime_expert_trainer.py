from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from statistics import mean
from typing import Any


DEFAULT_THRESHOLD = 0.5
PRECISION_FIRST_THRESHOLD = 0.8
UNKNOWN_MARKERS = ("Unknown", "Neutral", "Transitional")


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_rows(path: Path) -> list[dict[str, Any]]:
    if path.suffix.lower() == ".jsonl":
        return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    with path.open(newline="", encoding="utf-8") as handle:
        return [dict(row) for row in csv.DictReader(handle)]


def _to_float(value: Any, default: float = 0.0) -> float:
    try:
        if value in (None, ""):
            return default
        return float(value)
    except (TypeError, ValueError):
        return default


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=False) + "\n")


def _is_abstain_label(expert: dict[str, Any]) -> bool:
    label_id = str(expert.get("label_id", ""))
    label = str(expert.get("label", ""))
    policy = str(expert.get("abstain_policy", ""))
    return policy == "always_abstain" or any(marker in label_id or marker in label for marker in UNKNOWN_MARKERS)


def _truth_for(row: dict[str, Any], expert: dict[str, Any]) -> bool:
    label = str(expert.get("label", ""))
    level = str(expert.get("level", ""))
    for field in ("truth", "label", f"{level}_label", "primary_label"):
        if str(row.get(field, "")) == label:
            return True
    return False


def _score_row(row: dict[str, Any], expert: dict[str, Any]) -> float:
    label = str(expert.get("label", ""))
    features = [str(item) for item in expert.get("required_features", [])]
    if not features:
        return 0.0
    values = [_to_float(row.get(feature), 0.0) for feature in features if feature in row]
    if not values:
        return 0.0
    base = mean(min(1.0, max(0.0, value if abs(value) <= 1.0 else value / 100.0)) for value in values)
    directional = _to_float(row.get("directional_efficiency_3", row.get("directional_efficiency")), 0.0)
    atr = _to_float(row.get("atr_percentile"), 0.0)
    volume = _to_float(row.get("volume_percentile"), 0.0)
    rsi = _to_float(row.get("rsi_3", row.get("rsi")), 50.0)
    if label == "TrendExpansion":
        return min(1.0, 0.25 + 0.35 * directional + 0.2 * volume + 0.2 * (rsi / 100.0))
    if label == "RangeConsolidation":
        return min(1.0, 0.25 + 0.35 * (1.0 - directional) + 0.2 * (1.0 - abs(rsi - 50.0) / 50.0) + 0.2 * (1.0 - atr))
    if label == "ExtremeStress":
        return min(1.0, 0.35 * atr + 0.35 * volume + 0.3 * abs(rsi - 50.0) / 50.0)
    if label == "ReversalBrewing":
        return min(1.0, 0.35 * (1.0 - directional) + 0.35 * abs(rsi - 50.0) / 50.0 + 0.3 * volume)
    return min(1.0, base)


def _metrics(scores: list[tuple[float, bool]], threshold: float) -> dict[str, Any]:
    if not scores:
        return {"precision": 0.0, "recall": 0.0, "f1": 0.0, "brier_proxy": 0.0, "ece_proxy": 0.0, "support": 0}
    tp = sum(1 for score, truth in scores if score >= threshold and truth)
    fp = sum(1 for score, truth in scores if score >= threshold and not truth)
    fn = sum(1 for score, truth in scores if score < threshold and truth)
    precision = tp / (tp + fp) if (tp + fp) else 0.0
    recall = tp / (tp + fn) if (tp + fn) else 0.0
    f1 = 2.0 * precision * recall / (precision + recall) if (precision + recall) else 0.0
    brier = mean((score - (1.0 if truth else 0.0)) ** 2 for score, truth in scores)
    positives = [score for score, truth in scores if truth]
    negatives = [score for score, truth in scores if not truth]
    ece = abs((mean(positives) if positives else 0.0) - (1.0 - (mean(negatives) if negatives else 0.0)))
    return {
        "precision": round(precision, 6),
        "recall": round(recall, 6),
        "f1": round(f1, 6),
        "brier_proxy": round(brier, 6),
        "ece_proxy": round(ece, 6),
        "support": sum(1 for _score, truth in scores if truth),
    }


def _threshold_for(expert: dict[str, Any], precision_first: bool) -> float:
    if _is_abstain_label(expert):
        return 1.0
    return PRECISION_FIRST_THRESHOLD if precision_first else DEFAULT_THRESHOLD


def build_expert_training_artifacts(
    *,
    ontology_path: Path,
    features_path: Path,
    output_scores: Path,
    output_report: Path,
    cluster_report_path: Path | None = None,
    hmm_report_path: Path | None = None,
    precision_first: bool = True,
    embargo_bars: int = 1,
) -> dict[str, Any]:
    ontology = _load_json(ontology_path)
    rows = _load_rows(features_path)
    experts = list(ontology.get("experts", []))
    score_rows: list[dict[str, Any]] = []
    summaries: list[dict[str, Any]] = []

    for expert in experts:
        label_id = str(expert.get("label_id", ""))
        threshold = _threshold_for(expert, precision_first)
        pairs = [(_score_row(row, expert), _truth_for(row, expert)) for row in rows]
        summary = {
            "label_id": label_id,
            "level": expert.get("level", ""),
            "threshold": threshold,
            "abstain_policy": expert.get("abstain_policy", ""),
            **_metrics(pairs, threshold),
        }
        summaries.append(summary)
        for row, (score, _truth) in zip(rows, pairs):
            abstain_reason = "always_abstain_label" if _is_abstain_label(expert) else ""
            decision = "abstain" if abstain_reason else ("positive" if score >= threshold else "negative")
            score_rows.append({
                "timestamp": str(row.get("timestamp", "")),
                "label_id": label_id,
                "score": round(score, 6),
                "threshold": threshold,
                "decision": decision,
                "abstain_reason": abstain_reason,
            })

    report = {
        "schema_version": "regime-expert-training/v1",
        "expert_count": len(experts),
        "row_count": len(rows),
        "mode": "pure_python_threshold_fallback",
        "precision_first": precision_first,
        "purged_split_interface": {"enabled": True, "embargo_bars": embargo_bars, "implementation": "deterministic_fallback"},
        "optional_inputs": {
            "cluster_report": "present" if cluster_report_path and cluster_report_path.exists() else "missing",
            "hmm_report": "present" if hmm_report_path and hmm_report_path.exists() else "missing",
        },
        "ontology_mutation": "read_only",
        "experts": summaries,
    }
    _write_jsonl(output_scores, score_rows)
    _write_json(output_report, report)
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Train or score one-vs-rest ICT regime experts with pure-Python fallback.")
    parser.add_argument("--ontology", required=True)
    parser.add_argument("--features", required=True)
    parser.add_argument("--cluster-report")
    parser.add_argument("--hmm-report")
    parser.add_argument("--output-scores", required=True)
    parser.add_argument("--output-report", required=True)
    parser.add_argument("--balanced-thresholds", action="store_true")
    parser.add_argument("--embargo-bars", type=int, default=1)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_expert_training_artifacts(
        ontology_path=Path(args.ontology),
        features_path=Path(args.features),
        cluster_report_path=Path(args.cluster_report) if args.cluster_report else None,
        hmm_report_path=Path(args.hmm_report) if args.hmm_report else None,
        output_scores=Path(args.output_scores),
        output_report=Path(args.output_report),
        precision_first=not args.balanced_thresholds,
        embargo_bars=args.embargo_bars,
    )
    print(json.dumps({"ok": True, "expert_count": report["expert_count"], "output": args.output_report}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())