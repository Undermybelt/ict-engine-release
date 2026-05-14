from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


UNKNOWN_MARKERS = ("Unknown", "Neutral", "Transitional")


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _to_float(value: Any, default: float = 0.0) -> float:
    try:
        if value in (None, ""):
            return default
        return float(value)
    except (TypeError, ValueError):
        return default


def _top_labels_by_timestamp(score_rows: list[dict[str, Any]], label_prefix: str = "") -> list[tuple[str, str, float]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in score_rows:
        label = str(row.get("label_id", ""))
        if label_prefix and not label.startswith(label_prefix):
            continue
        if row.get("abstain_reason"):
            continue
        grouped[str(row.get("timestamp", ""))].append(row)
    timeline: list[tuple[str, str, float]] = []
    for timestamp in sorted(grouped):
        best = max(grouped[timestamp], key=lambda row: _to_float(row.get("score")))
        timeline.append((timestamp, str(best.get("label_id", "")), _to_float(best.get("score"))))
    return timeline


def _latest_conformal_set(conformal: dict[str, Any], timestamp: str) -> list[str]:
    by_99 = conformal.get("sets_by_target_coverage", {}).get("0.99", {})
    if timestamp in by_99:
        return list(by_99.get(timestamp, []))
    if by_99:
        latest_key = sorted(by_99)[-1]
        return list(by_99.get(latest_key, []))
    return []


def _duration(timeline: list[tuple[str, str, float]], label: str) -> int:
    count = 0
    for _timestamp, current, _score in reversed(timeline):
        if current != label:
            break
        count += 1
    return count


def _flip_count(timeline: list[tuple[str, str, float]]) -> int:
    labels = [label for _timestamp, label, _score in timeline]
    return sum(1 for prev, cur in zip(labels, labels[1:]) if prev != cur)


def build_transition_governor_report(
    *,
    scores_path: Path,
    conformal_report_path: Path,
    distributional_report_path: Path,
    output_json: Path,
    hmm_report_path: Path | None = None,
    drift_rows_path: Path | None = None,
    label_prefix: str = "",
    min_duration: int = 3,
) -> dict[str, Any]:
    score_rows = _load_jsonl(scores_path)
    conformal = _load_json(conformal_report_path)
    distribution = _load_json(distributional_report_path)
    timeline = _top_labels_by_timestamp(score_rows, label_prefix=label_prefix)
    timestamp, current_label, current_score = timeline[-1] if timeline else ("", "", 0.0)
    conformal_set = _latest_conformal_set(conformal, timestamp)
    duration = _duration(timeline, current_label)
    flips = _flip_count(timeline)
    reasons: list[str] = []
    if any(marker in current_label for marker in UNKNOWN_MARKERS):
        reasons.append("unknown_label")
    if len(conformal_set) != 1:
        reasons.append("wide_conformal_set")
    if not conformal.get("confidence_95", False):
        reasons.append("confidence_95_failed")
    if distribution.get("agreement") != "agree":
        reasons.append("distributional_disagreement")
    if distribution.get("transitional_flag", False):
        reasons.append("distributional_transitional")
    if current_label and duration < min_duration:
        reasons.append("duration_below_minimum")
    if flips >= 2:
        reasons.append("flip_flop_labels")
    if drift_rows_path and drift_rows_path.exists():
        drift_rows = _load_jsonl(drift_rows_path)
        if any(bool(row.get("drift_flag", False)) for row in drift_rows):
            reasons.append("external_drift_flag")
    if hmm_report_path and hmm_report_path.exists():
        hmm = _load_json(hmm_report_path)
        if hmm.get("k_metrics", {}).get(str(hmm.get("best_k", "")), {}).get("transition_persistence", 1.0) < 0.5:
            reasons.append("low_hmm_transition_persistence")

    transition_hazard = min(1.0, round(0.15 * len(reasons) + 0.1 * flips, 6))
    if "unknown_label" in reasons or "wide_conformal_set" in reasons:
        hint = "unknown_abstain"
    elif reasons:
        hint = "transition_guardrail"
    else:
        hint = "accept_regime"
        transition_hazard = 0.0

    report = {
        "schema_version": "regime-transition-governor/v1",
        "timestamp": timestamp,
        "current_label": current_label,
        "current_score": current_score,
        "label_prefix": label_prefix,
        "min_duration": min_duration,
        "observed_duration": duration,
        "flip_count": flips,
        "conformal_set_size": len(conformal_set),
        "distributional_agreement": distribution.get("agreement", ""),
        "transition_hazard": transition_hazard,
        "guardrail_reasons": reasons,
        "execution_tree_hint": hint,
        "bbn_evidence_hint": {
            "regime_transition_hazard": transition_hazard,
            "regime_governor_hint": hint,
            "regime_governor_reasons": reasons,
        },
    }
    _write_json(output_json, report)
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Apply hysteresis and transition guardrails to regime sidecar outputs.")
    parser.add_argument("--scores", required=True)
    parser.add_argument("--conformal-report", required=True)
    parser.add_argument("--distributional-report", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--hmm-report")
    parser.add_argument("--drift-rows")
    parser.add_argument("--label-prefix", default="")
    parser.add_argument("--min-duration", type=int, default=3)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_transition_governor_report(
        scores_path=Path(args.scores),
        conformal_report_path=Path(args.conformal_report),
        distributional_report_path=Path(args.distributional_report),
        output_json=Path(args.output_json),
        hmm_report_path=Path(args.hmm_report) if args.hmm_report else None,
        drift_rows_path=Path(args.drift_rows) if args.drift_rows else None,
        label_prefix=args.label_prefix,
        min_duration=args.min_duration,
    )
    print(json.dumps({"ok": True, "output": args.output_json, "execution_tree_hint": report["execution_tree_hint"], "transition_hazard": report["transition_hazard"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())