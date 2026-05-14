from __future__ import annotations

import argparse
import json
from pathlib import Path
from statistics import mean
from typing import Any

DEFAULT_THRESHOLDS = {
    "alpha": 0.05,
    "min_rolling_coverage": 0.93,
    "max_calibration_ece": 0.05,
    "max_bootstrap_ci_width": 0.25,
    "max_transition_prob": 0.2,
    "max_flip_rate": 0.2,
}


def _load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _posterior(row: dict[str, Any]) -> dict[str, float]:
    raw = row.get("posterior", {})
    if isinstance(raw, str):
        raw = json.loads(raw)
    return {str(key): float(value) for key, value in raw.items()}


def _top_label_and_prob(row: dict[str, Any]) -> tuple[str, float]:
    posterior = _posterior(row)
    if not posterior:
        return "", 0.0
    label, probability = max(posterior.items(), key=lambda item: item[1])
    return label, probability


def _conformal_set_size(row: dict[str, Any], alpha: float) -> int:
    posterior = _posterior(row)
    threshold = 1.0 - alpha
    strong = [label for label, probability in posterior.items() if probability >= threshold]
    if strong:
        return 1
    # Ambiguous rows keep every non-trivial alternative in the set.
    return max(1, sum(1 for probability in posterior.values() if probability > alpha))


def _rolling_coverage(rows: list[dict[str, Any]]) -> float:
    scored = []
    for row in rows:
        truth = str(row.get("truth", ""))
        top_label, _ = _top_label_and_prob(row)
        if truth:
            scored.append(1.0 if truth == top_label else 0.0)
    return mean(scored) if scored else 0.0


def _calibration_ece(rows: list[dict[str, Any]], bin_count: int = 10) -> float:
    bins: list[list[tuple[float, float]]] = [[] for _ in range(bin_count)]
    for row in rows:
        truth = str(row.get("truth", ""))
        top_label, confidence = _top_label_and_prob(row)
        if not truth or not top_label:
            continue
        index = min(bin_count - 1, max(0, int(confidence * bin_count)))
        bins[index].append((confidence, 1.0 if truth == top_label else 0.0))
    total = sum(len(bucket) for bucket in bins)
    if total == 0:
        return 1.0
    error = 0.0
    for bucket in bins:
        if not bucket:
            continue
        avg_conf = mean(item[0] for item in bucket)
        avg_acc = mean(item[1] for item in bucket)
        error += len(bucket) / total * abs(avg_conf - avg_acc)
    return error


def _percentile(values: list[float], ratio: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, int(round((len(ordered) - 1) * ratio))))
    return ordered[index]


def _bootstrap_ci_width(rows: list[dict[str, Any]]) -> float:
    confidences = [_top_label_and_prob(row)[1] for row in rows if _top_label_and_prob(row)[0]]
    if len(confidences) < 2:
        return 1.0
    return _percentile(confidences, 0.95) - _percentile(confidences, 0.05)


def _transition_prob(rows: list[dict[str, Any]]) -> float:
    values = [float(row.get("transition_prob", 0.0)) for row in rows]
    return max(values) if values else 0.0


def _flip_rate(rows: list[dict[str, Any]]) -> float:
    labels = [_top_label_and_prob(row)[0] for row in rows if _top_label_and_prob(row)[0]]
    if len(labels) < 3:
        return 0.0
    # Count unstable A->B->A reversals, not one legitimate regime transition.
    reversals = sum(
        1
        for left, middle, right in zip(labels, labels[1:], labels[2:])
        if left == right and left != middle
    )
    return reversals / (len(labels) - 2)


def build_confidence_report(
    *,
    rows: list[dict[str, Any]],
    candidate_id: str,
    thresholds: dict[str, float] | None = None,
) -> dict[str, Any]:
    merged = dict(DEFAULT_THRESHOLDS)
    if thresholds:
        merged.update(thresholds)
    alpha = float(merged["alpha"])
    set_sizes = [_conformal_set_size(row, alpha) for row in rows]
    conformal_set_size = max(set_sizes) if set_sizes else 0
    rolling_coverage = _rolling_coverage(rows)
    calibration_ece = _calibration_ece(rows)
    bootstrap_width = _bootstrap_ci_width(rows)
    transition_prob = _transition_prob(rows)
    flip_rate = _flip_rate(rows)
    confidence_95 = (
        conformal_set_size == 1
        and rolling_coverage >= float(merged["min_rolling_coverage"])
        and calibration_ece <= float(merged["max_calibration_ece"])
        and bootstrap_width <= float(merged["max_bootstrap_ci_width"])
        and transition_prob <= float(merged["max_transition_prob"])
        and flip_rate <= float(merged["max_flip_rate"])
    )
    if confidence_95:
        gate = "pass"
    elif rows and rolling_coverage >= 0.75 and transition_prob <= 0.35:
        gate = "probe"
    else:
        gate = "reject"

    return {
        "schema_version": "regime-confidence-report/v1",
        "candidate_id": candidate_id,
        "row_count": len(rows),
        "confidence_95": confidence_95,
        "conformal_set_size": conformal_set_size,
        "rolling_coverage": rolling_coverage,
        "calibration_ece": calibration_ece,
        "bootstrap_ci_width": bootstrap_width,
        "transition_prob": transition_prob,
        "flip_rate": flip_rate,
        "regime_confidence_gate": gate,
        "thresholds": merged,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build a zero-config regime confidence report.")
    parser.add_argument("--rows-jsonl", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--candidate-id", required=True)
    parser.add_argument("--alpha", type=float, default=DEFAULT_THRESHOLDS["alpha"])
    parser.add_argument("--min-rolling-coverage", type=float, default=DEFAULT_THRESHOLDS["min_rolling_coverage"])
    parser.add_argument("--max-calibration-ece", type=float, default=DEFAULT_THRESHOLDS["max_calibration_ece"])
    parser.add_argument("--max-bootstrap-ci-width", type=float, default=DEFAULT_THRESHOLDS["max_bootstrap_ci_width"])
    parser.add_argument("--max-transition-prob", type=float, default=DEFAULT_THRESHOLDS["max_transition_prob"])
    parser.add_argument("--max-flip-rate", type=float, default=DEFAULT_THRESHOLDS["max_flip_rate"])
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    result = build_confidence_report(
        rows=_load_jsonl(Path(args.rows_jsonl)),
        candidate_id=args.candidate_id,
        thresholds={
            "alpha": args.alpha,
            "min_rolling_coverage": args.min_rolling_coverage,
            "max_calibration_ece": args.max_calibration_ece,
            "max_bootstrap_ci_width": args.max_bootstrap_ci_width,
            "max_transition_prob": args.max_transition_prob,
            "max_flip_rate": args.max_flip_rate,
        },
    )
    _write_json(Path(args.output_json), result)
    print(json.dumps({"ok": True, "output": args.output_json, "regime_confidence_gate": result["regime_confidence_gate"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
