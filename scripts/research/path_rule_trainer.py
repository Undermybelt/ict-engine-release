#!/usr/bin/env python3
"""Build a tiny explicit path-ranker artifact from exported target rows.

This is intentionally minimal and deterministic:
- reads explicit target rows from JSONL or CSV
- derives a small rule list or shallow tree artifact
- writes one diff-friendly JSON artifact

It is a research harness only. Runtime stays read-only and consumes the
persisted artifact through ict-engine's existing opt-in registration path.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path
from statistics import mean
from typing import Any


NUMERIC_FEATURES = [
    "rank",
    "behavior_policy_probability",
    "execution_propensity",
    "target_policy_probability_confidence",
    "target_policy_probability_lower_bound",
    "target_policy_reward_prior",
    "target_policy_reward_lower_bound",
    "experience_prior",
    "current_posterior",
    "structural_baseline_score",
    "maturity_weight",
    "propensity_estimate",
    "ips_weight",
    "training_weight",
    "raw_path_score",
    "calibrated_path_prob",
    "path_prob_lower_bound",
]

CATEGORICAL_FEATURES = [
    "direction",
    "regime_calibration_bucket",
    "pending_reward_state",
    "execution_gate_status",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Train a tiny explicit structural path-ranker artifact from exported target rows."
    )
    parser.add_argument("--target-jsonl", help="Path to structural_path_ranking_target.jsonl")
    parser.add_argument("--target-csv", help="Path to structural_path_ranking_target.csv")
    parser.add_argument("--history-jsonl", help="Optional path to history rows jsonl")
    parser.add_argument(
        "--model-family",
        default="corels",
        choices=["corels", "gosdt", "ga_mask_tree"],
        help="Explicit artifact family to emit",
    )
    parser.add_argument("--out", required=True, help="Output JSON artifact path")
    return parser.parse_args()


def load_jsonl_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        rows.append(json.loads(line))
    return rows


def load_csv_rows(path: Path) -> list[dict[str, Any]]:
    with path.open(newline="") as handle:
        return list(csv.DictReader(handle))


def load_rows(args: argparse.Namespace) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    if not args.target_jsonl and not args.target_csv:
        raise SystemExit("one of --target-jsonl or --target-csv is required")
    current_path = Path(args.target_jsonl or args.target_csv)
    if not current_path.exists():
        raise SystemExit(f"target rows not found: {current_path}")
    current_rows = (
        load_jsonl_rows(current_path)
        if current_path.suffix.lower() == ".jsonl"
        else load_csv_rows(current_path)
    )
    history_rows: list[dict[str, Any]] = []
    if args.history_jsonl:
        history_path = Path(args.history_jsonl)
        if history_path.exists():
            history_rows = load_jsonl_rows(history_path)
    return current_rows, history_rows


def maybe_float(value: Any) -> float | None:
    if value is None or value == "":
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def label_for_row(row: dict[str, Any]) -> float | None:
    calibrated = maybe_float(row.get("calibrated_label"))
    if calibrated is not None:
        return max(0.0, min(1.0, calibrated))
    state = str(row.get("pending_reward_state", "")).strip()
    if state == "matured_success":
        return 1.0
    if state in {"matured_failure", "matured_invalidated"}:
        return 0.0
    return None


def numeric_feature(row: dict[str, Any], name: str) -> float | None:
    return maybe_float(row.get(name))


def categorical_feature(row: dict[str, Any], name: str) -> str | None:
    value = row.get(name)
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def labeled_rows(rows: list[dict[str, Any]]) -> list[tuple[dict[str, Any], float]]:
    out: list[tuple[dict[str, Any], float]] = []
    for row in rows:
        label = label_for_row(row)
        if label is not None:
            out.append((row, label))
    return out


def proxy_labeled_rows(rows: list[dict[str, Any]]) -> list[tuple[dict[str, Any], float]]:
    if not rows:
        return []
    def ordering_score(row: dict[str, Any]) -> tuple[float, float]:
        posterior = numeric_feature(row, "current_posterior") or 0.0
        prior = numeric_feature(row, "experience_prior") or 0.0
        return (posterior, prior)
    best_row = max(rows, key=ordering_score)
    out: list[tuple[dict[str, Any], float]] = []
    for row in rows:
        out.append((row, 1.0 if row is best_row else 0.0))
    return out


def candidate_thresholds(values: list[float]) -> list[float]:
    unique = sorted(set(values))
    if not unique:
        return []
    if len(unique) <= 8:
        return unique
    picks = [0.2, 0.4, 0.5, 0.6, 0.8]
    out: list[float] = []
    for ratio in picks:
        index = min(len(unique) - 1, max(0, int(round((len(unique) - 1) * ratio))))
        out.append(unique[index])
    return sorted(set(out))


def condition_matches(row: dict[str, Any], condition: dict[str, Any]) -> bool:
    feature = condition["feature"]
    operator = condition["operator"]
    if "numeric_value" in condition and condition["numeric_value"] is not None:
        value = numeric_feature(row, feature)
        if value is None:
            return False
        threshold = float(condition["numeric_value"])
        if operator == "ge":
            return value >= threshold
        if operator == "gt":
            return value > threshold
        if operator == "le":
            return value <= threshold
        if operator == "lt":
            return value < threshold
        if operator == "eq":
            return math.isclose(value, threshold)
        if operator == "neq":
            return not math.isclose(value, threshold)
        return False
    value = categorical_feature(row, feature)
    expected = condition.get("string_value")
    if value is None or expected is None:
        return False
    if operator == "eq":
        return value == expected
    if operator == "neq":
        return value != expected
    return False


def find_best_numeric_condition(rows: list[tuple[dict[str, Any], float]]) -> tuple[dict[str, Any], float] | None:
    best: tuple[dict[str, Any], float] | None = None
    total = len(rows)
    for feature in NUMERIC_FEATURES:
        values = [numeric_feature(row, feature) for row, _ in rows]
        usable = [value for value in values if value is not None]
        if len(usable) < 2:
            continue
        thresholds = candidate_thresholds(usable)
        for operator in ("ge", "le"):
            for threshold in thresholds:
                matched = [
                    label
                    for row, label in rows
                    if condition_matches(
                        row,
                        {
                            "feature": feature,
                            "operator": operator,
                            "numeric_value": threshold,
                        },
                    )
                ]
                if not matched or len(matched) == total:
                    continue
                probability = mean(matched)
                coverage = len(matched) / total
                edge = abs(probability - 0.5) * 2.0
                objective = coverage * edge
                if best is None or objective > best[1]:
                    best = (
                        {
                            "feature": feature,
                            "operator": operator,
                            "numeric_value": threshold,
                        },
                        objective,
                    )
    return best


def find_best_categorical_condition(
    rows: list[tuple[dict[str, Any], float]]
) -> tuple[dict[str, Any], float] | None:
    best: tuple[dict[str, Any], float] | None = None
    total = len(rows)
    for feature in CATEGORICAL_FEATURES:
        values = sorted({categorical_feature(row, feature) for row, _ in rows if categorical_feature(row, feature)})
        for value in values:
            matched = [
                label
                for row, label in rows
                if condition_matches(
                    row,
                    {
                        "feature": feature,
                        "operator": "eq",
                        "string_value": value,
                    },
                )
            ]
            if not matched or len(matched) == total:
                continue
            probability = mean(matched)
            coverage = len(matched) / total
            edge = abs(probability - 0.5) * 2.0
            objective = coverage * edge
            if best is None or objective > best[1]:
                best = (
                    {
                        "feature": feature,
                        "operator": "eq",
                        "string_value": value,
                    },
                    objective,
                )
    return best


def best_condition(rows: list[tuple[dict[str, Any], float]]) -> dict[str, Any] | None:
    numeric = find_best_numeric_condition(rows)
    categorical = find_best_categorical_condition(rows)
    candidates = [item for item in (numeric, categorical) if item is not None]
    if not candidates:
        return None
    return max(candidates, key=lambda item: item[1])[0]


def probability_for_condition(rows: list[tuple[dict[str, Any], float]], condition: dict[str, Any]) -> tuple[float, float]:
    matched = [label for row, label in rows if condition_matches(row, condition)]
    if not matched:
        return 0.5, 0.0
    return mean(matched), len(matched) / len(rows)


def default_probability(rows: list[tuple[dict[str, Any], float]]) -> float:
    return mean(label for _, label in rows) if rows else 0.5


def build_rule_list(rows: list[tuple[dict[str, Any], float]]) -> tuple[list[dict[str, Any]], list[str]]:
    condition = best_condition(rows)
    if condition is None:
        return (
            [
                {
                    "conditions": [],
                    "score": default_probability(rows),
                    "path_prob_lower_bound": max(0.0, default_probability(rows) - 0.1),
                    "execution_gate_status": "observe",
                }
            ],
            ["rank"],
        )
    probability, _coverage = probability_for_condition(rows, condition)
    default_score = default_probability(
        [(row, label) for row, label in rows if not condition_matches(row, condition)]
    )
    selected = [condition["feature"]]
    return (
        [
            {
                "conditions": [condition],
                "score": probability,
                "path_prob_lower_bound": max(0.0, probability - 0.1),
                "execution_gate_status": "pass" if probability >= 0.5 else "observe",
            },
            {
                "conditions": [],
                "score": default_score,
                "path_prob_lower_bound": max(0.0, default_score - 0.1),
                "execution_gate_status": "pass" if default_score >= 0.5 else "observe",
            },
        ],
        selected,
    )


def build_tree(rows: list[tuple[dict[str, Any], float]]) -> tuple[dict[str, Any], list[str]]:
    condition = best_condition(rows)
    if condition is None:
        score = default_probability(rows)
        return (
            {
                "score": score,
                "path_prob_lower_bound": max(0.0, score - 0.1),
                "execution_gate_status": "pass" if score >= 0.5 else "observe",
            },
            [],
        )
    matched = [(row, label) for row, label in rows if condition_matches(row, condition)]
    unmatched = [(row, label) for row, label in rows if not condition_matches(row, condition)]
    left_score = default_probability(matched)
    right_score = default_probability(unmatched)
    return (
        {
            **condition,
            "left": {
                "score": left_score,
                "path_prob_lower_bound": max(0.0, left_score - 0.1),
                "execution_gate_status": "pass" if left_score >= 0.5 else "observe",
            },
            "right": {
                "score": right_score,
                "path_prob_lower_bound": max(0.0, right_score - 0.1),
                "execution_gate_status": "pass" if right_score >= 0.5 else "observe",
            },
        },
        [condition["feature"]],
    )


def score_rows(rows: list[tuple[dict[str, Any], float]], model_family: str) -> tuple[list[float], list[str], list[dict[str, Any]], dict[str, Any] | None]:
    if model_family == "corels":
        rule_list, selected = build_rule_list(rows)
        predictions = []
        for row, _label in rows:
            for rule in rule_list:
                if all(condition_matches(row, cond) for cond in rule["conditions"]):
                    predictions.append(float(rule["score"]))
                    break
        return predictions, selected, rule_list, None
    tree_json, selected = build_tree(rows)
    predictions = []
    for row, _label in rows:
        node = tree_json
        while "score" not in node:
            if condition_matches(row, node):
                node = node["left"]
            else:
                node = node["right"]
        predictions.append(float(node["score"]))
    return predictions, selected, [], tree_json


def calibration_metrics(labels: list[float], predictions: list[float]) -> dict[str, Any]:
    if not labels:
        return {"eligible_rows": 0}
    errors = [(pred - label) ** 2 for pred, label in zip(predictions, labels)]
    abs_errors = [abs(pred - label) for pred, label in zip(predictions, labels)]
    return {
        "eligible_rows": len(labels),
        "brier_score": mean(errors),
        "propensity_weighted_brier_score": mean(errors),
        "expected_calibration_error": mean(abs_errors),
        "max_calibration_error": max(abs_errors),
    }


def build_artifact(
    current_rows: list[dict[str, Any]],
    history_rows: list[dict[str, Any]],
    model_family: str,
) -> dict[str, Any]:
    labeled = labeled_rows(current_rows)
    used_proxy_labels = False
    if not labeled:
        labeled = proxy_labeled_rows(current_rows)
        used_proxy_labels = True
    if not labeled:
        raise SystemExit("no usable rows found in the provided target rows")
    labels = [label for _, label in labeled]
    predictions, selected_features, rule_list, tree_json = score_rows(labeled, model_family)
    artifact: dict[str, Any] = {
        "protocol_version": "structural-path-ranking-trainer-artifact-v1",
        "dataset_role": "external_path_ranker_training_dataset",
        "model_family": model_family,
        "selected_features": selected_features,
        "trained_rows": len(labeled),
        "history_rows": len(history_rows) if history_rows else len(current_rows),
        "validation_metrics": {
            "raw_scored_mature_rows": len(labeled),
            "raw_scored_mature_min_rows": 30,
            "production_validation_rows": len(labeled),
            "production_validation_min_rows": 30,
        },
        "calibration_metrics": calibration_metrics(labels, predictions),
        "notes": [
            "trainer_family=minimal_explicit_rule_tree_harness",
            "runtime_boundary=offline_trainer_to_explicit_artifact",
        ],
    }
    if used_proxy_labels:
        artifact["notes"].append(
            "label_source=current_candidate_proxy because matured labels were unavailable"
        )
    if rule_list:
        artifact["rule_list"] = rule_list
    if tree_json is not None:
        artifact["tree_json"] = tree_json
    return artifact


def main() -> None:
    args = parse_args()
    current_rows, history_rows = load_rows(args)
    artifact = build_artifact(current_rows, history_rows, args.model_family)
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
