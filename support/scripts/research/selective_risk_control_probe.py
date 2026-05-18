#!/usr/bin/env python3
"""Selective bad-loss risk-control probe for execution-tree scan windows."""

from __future__ import annotations

import argparse
import csv
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class JoinedWindow:
    window: str
    timestamp: str
    bucket: str
    execution_readiness: float | None
    prediction_vote_score: float | None
    ranker_score: float | None
    ranker_score_used: bool
    transition_hazard: float | None
    duration_remaining: float | None
    label_id: str
    future_ret: float
    safe: bool


@dataclass(frozen=True)
class Rule:
    bucket: str
    score_min: float
    readiness_min: float
    transition_max: float | None
    duration_min: float | None
    require_ranker_used: bool

    def matches(self, row: JoinedWindow) -> bool:
        if self.bucket != "all" and row.bucket != self.bucket:
            return False
        score = row.ranker_score if row.ranker_score is not None else row.prediction_vote_score
        if score is None or score < self.score_min:
            return False
        if row.execution_readiness is None or row.execution_readiness < self.readiness_min:
            return False
        if self.transition_max is not None:
            if row.transition_hazard is None or row.transition_hazard > self.transition_max:
                return False
        if self.duration_min is not None:
            if row.duration_remaining is None or row.duration_remaining < self.duration_min:
                return False
        if self.require_ranker_used and not row.ranker_score_used:
            return False
        return True

    def as_dict(self) -> dict:
        return {
            "bucket": self.bucket,
            "score_min": self.score_min,
            "readiness_min": self.readiness_min,
            "transition_max": self.transition_max,
            "duration_min": self.duration_min,
            "require_ranker_used": self.require_ranker_used,
        }


def run_probe(
    *,
    scan_tsv: Path,
    windows_dir: Path,
    truth_jsonl: Path,
    output_json: Path,
    symbol: str,
    bad_loss_floor: float,
    alpha: float,
    calibration_fraction: float,
    min_calibration_support: int,
    min_test_support: int,
) -> dict:
    rows = join_scan_truth(
        scan_tsv=scan_tsv,
        windows_dir=windows_dir,
        truth_jsonl=truth_jsonl,
        symbol=symbol,
        bad_loss_floor=bad_loss_floor,
    )
    rows.sort(key=lambda row: int(row.window))
    split_index = max(1, min(len(rows) - 1, int(len(rows) * calibration_fraction)))
    calibration = rows[:split_index]
    test = rows[split_index:]

    accepted_rules = []
    for rule in candidate_rules(calibration):
        selected = [row for row in calibration if rule.matches(row)]
        if len(selected) < min_calibration_support:
            continue
        bad_losses = sum(1 for row in selected if not row.safe)
        risk = bad_losses / len(selected)
        risk_upper = wilson_upper(bad_losses, len(selected))
        if risk_upper <= alpha:
            accepted_rules.append(
                {
                    "rule": rule.as_dict(),
                    "calibration_support": len(selected),
                    "calibration_bad_losses": bad_losses,
                    "calibration_bad_loss_rate": risk,
                    "calibration_bad_loss_wilson_upper": risk_upper,
                }
            )

    accepted_rules.sort(
        key=lambda item: (
            item["calibration_bad_loss_wilson_upper"],
            -item["calibration_support"],
        )
    )
    selected_rule = accepted_rules[0] if accepted_rules else None
    test_selected: list[JoinedWindow] = []
    if selected_rule is not None:
        rule = Rule(**selected_rule["rule"])
        test_selected = [row for row in test if rule.matches(row)]
        if len(test_selected) < min_test_support:
            test_selected = []
            selected_rule = None
            accepted_rules = []

    test_bad_losses = sum(1 for row in test_selected if not row.safe)
    report = {
        "schema_version": "selective-risk-control-probe/v1",
        "symbol": symbol,
        "target": {
            "safe_rule": f"future_ret > {bad_loss_floor}",
            "alpha": alpha,
            "calibration_fraction": calibration_fraction,
            "min_calibration_support": min_calibration_support,
            "min_test_support": min_test_support,
        },
        "rows_joined": len(rows),
        "calibration_rows": len(calibration),
        "test_rows": len(test),
        "accepted_rules": accepted_rules,
        "selected_rule": selected_rule,
        "test": {
            "accepted_windows": len(test_selected),
            "bad_losses": test_bad_losses,
            "bad_loss_rate": test_bad_losses / len(test_selected)
            if test_selected
            else None,
            "bad_loss_wilson_upper": wilson_upper(test_bad_losses, len(test_selected))
            if test_selected
            else None,
            "windows": [row.window for row in test_selected],
        },
        "decision": "accepted_release_rule_found"
        if selected_rule is not None
        else "abstain_no_calibrated_release_rule",
    }
    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_json.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    return report


def join_scan_truth(
    *,
    scan_tsv: Path,
    windows_dir: Path,
    truth_jsonl: Path,
    symbol: str,
    bad_loss_floor: float,
) -> list[JoinedWindow]:
    del symbol
    truth_by_timestamp = {}
    with truth_jsonl.open(encoding="utf-8") as fh:
        for line in fh:
            if not line.strip():
                continue
            row = json.loads(line)
            truth_by_timestamp[str(row["timestamp"])] = row

    joined = []
    with scan_tsv.open(encoding="utf-8", newline="") as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            timestamp = window_last_timestamp(windows_dir, row["window"])
            truth = truth_by_timestamp.get(timestamp)
            if truth is None:
                continue
            transition = parse_float(row.get("hybrid_transition_hazard"))
            joined.append(
                JoinedWindow(
                    window=row["window"],
                    timestamp=timestamp,
                    bucket=transition_bucket(transition),
                    execution_readiness=parse_float(row.get("execution_readiness")),
                    prediction_vote_score=parse_float(row.get("prediction_vote_score")),
                    ranker_score=parse_float(row.get("ranker_score_raw_path_score")),
                    ranker_score_used=row.get("path_ranker_score_used_by_execution_tree")
                    == "true",
                    transition_hazard=transition,
                    duration_remaining=parse_float(
                        row.get("duration_remaining_expected_bars")
                    ),
                    label_id=str(truth.get("label_id", "")),
                    future_ret=float(truth["future_ret"]),
                    safe=float(truth["future_ret"]) > bad_loss_floor,
                )
            )
    return joined


def candidate_rules(rows: Iterable[JoinedWindow]) -> Iterable[Rule]:
    rows = list(rows)
    score_values = sorted(
        {
            value
            for row in rows
            for value in [row.ranker_score, row.prediction_vote_score]
            if value is not None
        }
    )
    readiness_values = sorted(
        {row.execution_readiness for row in rows if row.execution_readiness is not None}
    )
    transition_values = sorted(
        {row.transition_hazard for row in rows if row.transition_hazard is not None}
    )
    duration_values = sorted(
        {row.duration_remaining for row in rows if row.duration_remaining is not None}
    )
    score_thresholds = quantile_thresholds(score_values, [0.0, 0.25, 0.5, 0.75])
    readiness_thresholds = quantile_thresholds(readiness_values, [0.0, 0.25, 0.5, 0.75])
    transition_thresholds = [None] + quantile_thresholds(
        transition_values, [0.25, 0.5, 0.75, 1.0]
    )
    duration_thresholds = [None] + quantile_thresholds(duration_values, [0.0, 0.25, 0.5])
    buckets = ["all"] + sorted({row.bucket for row in rows})

    seen = set()
    for bucket in buckets:
        for score_min in score_thresholds:
            for readiness_min in readiness_thresholds:
                for transition_max in transition_thresholds:
                    for duration_min in duration_thresholds:
                        for require_ranker_used in [False, True]:
                            rule = Rule(
                                bucket=bucket,
                                score_min=score_min,
                                readiness_min=readiness_min,
                                transition_max=transition_max,
                                duration_min=duration_min,
                                require_ranker_used=require_ranker_used,
                            )
                            key = tuple(rule.as_dict().items())
                            if key in seen:
                                continue
                            seen.add(key)
                            yield rule


def quantile_thresholds(values: list[float], quantiles: list[float]) -> list[float]:
    if not values:
        return [0.0]
    result = []
    for quantile in quantiles:
        index = min(len(values) - 1, max(0, int(round((len(values) - 1) * quantile))))
        result.append(values[index])
    return sorted(set(result))


def transition_bucket(transition_hazard: float | None) -> str:
    if transition_hazard is None:
        return "transition_unknown"
    if transition_hazard >= 0.60:
        return "high_transition"
    return "stable_or_low_transition"


def window_last_timestamp(windows_dir: Path, window: str) -> str:
    candidates = [
        windows_dir / f"nq_15m_obs_{window}.json",
        windows_dir / f"NQ_15m_obs_{window}.json",
    ]
    for path in candidates:
        if path.exists():
            payload = json.loads(path.read_text(encoding="utf-8"))
            candles = payload.get("candles", [])
            if candles:
                return str(candles[-1]["timestamp"])
    raise FileNotFoundError(f"window file not found for {window} in {windows_dir}")


def parse_float(value: str | None) -> float | None:
    if value is None or value == "":
        return None
    try:
        return float(value)
    except ValueError:
        return None


def wilson_upper(bad_losses: int, total: int, z: float = 1.96) -> float:
    if total <= 0:
        return 1.0
    p_hat = bad_losses / total
    denom = 1.0 + z * z / total
    center = p_hat + z * z / (2.0 * total)
    radius = z * math.sqrt((p_hat * (1.0 - p_hat) + z * z / (4.0 * total)) / total)
    return min(1.0, (center + radius) / denom)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scan-tsv", required=True, type=Path)
    parser.add_argument("--windows-dir", required=True, type=Path)
    parser.add_argument("--truth-jsonl", required=True, type=Path)
    parser.add_argument("--output-json", required=True, type=Path)
    parser.add_argument("--symbol", default="NQ")
    parser.add_argument("--bad-loss-floor", type=float, default=-0.001)
    parser.add_argument("--alpha", type=float, default=0.05)
    parser.add_argument("--calibration-fraction", type=float, default=0.6)
    parser.add_argument("--min-calibration-support", type=int, default=30)
    parser.add_argument("--min-test-support", type=int, default=10)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    report = run_probe(
        scan_tsv=args.scan_tsv,
        windows_dir=args.windows_dir,
        truth_jsonl=args.truth_jsonl,
        output_json=args.output_json,
        symbol=args.symbol,
        bad_loss_floor=args.bad_loss_floor,
        alpha=args.alpha,
        calibration_fraction=args.calibration_fraction,
        min_calibration_support=args.min_calibration_support,
        min_test_support=args.min_test_support,
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
