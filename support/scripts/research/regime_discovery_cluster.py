from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path
from typing import Any

DEFAULT_K_VALUES = list(range(3, 13))
NON_FEATURE_COLUMNS = {"timestamp", "label", "truth", "regime", "mtf_alignment"}


def load_feature_rows(path: Path) -> list[dict[str, Any]]:
    if path.suffix.lower() == ".csv":
        with path.open(newline="", encoding="utf-8") as handle:
            return [dict(row) for row in csv.DictReader(handle)]
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def numeric_feature_columns(rows: list[dict[str, Any]]) -> list[str]:
    columns: list[str] = []
    for row in rows:
        for key, value in row.items():
            if key in NON_FEATURE_COLUMNS or key in columns:
                continue
            try:
                float(value)
            except (TypeError, ValueError):
                continue
            columns.append(key)
    return columns


def matrix_from_rows(rows: list[dict[str, Any]], columns: list[str]) -> list[list[float]]:
    matrix: list[list[float]] = []
    for row in rows:
        values = []
        for column in columns:
            try:
                values.append(float(row.get(column, 0.0)))
            except (TypeError, ValueError):
                values.append(0.0)
        matrix.append(values)
    return matrix


def standardize(matrix: list[list[float]]) -> list[list[float]]:
    if not matrix:
        return []
    width = len(matrix[0])
    means = [sum(row[col] for row in matrix) / len(matrix) for col in range(width)]
    stds = []
    for col in range(width):
        variance = sum((row[col] - means[col]) ** 2 for row in matrix) / max(1, len(matrix) - 1)
        stds.append(math.sqrt(variance) or 1.0)
    return [[(row[col] - means[col]) / stds[col] for col in range(width)] for row in matrix]


def distance(left: list[float], right: list[float]) -> float:
    return math.sqrt(sum((a - b) ** 2 for a, b in zip(left, right)))


def mean_vector(points: list[list[float]], width: int) -> list[float]:
    if not points:
        return [0.0] * width
    return [sum(point[col] for point in points) / len(points) for col in range(width)]


def kmeans(matrix: list[list[float]], k: int, iterations: int = 20) -> tuple[list[int], list[list[float]]]:
    if not matrix:
        return [], []
    k = min(k, len(matrix))
    width = len(matrix[0])
    seeds = [round(i * (len(matrix) - 1) / max(1, k - 1)) for i in range(k)]
    centroids = [list(matrix[index]) for index in seeds]
    labels = [0] * len(matrix)
    for _ in range(iterations):
        next_labels = [min(range(k), key=lambda cluster_id: distance(row, centroids[cluster_id])) for row in matrix]
        if next_labels == labels:
            break
        labels = next_labels
        for cluster_id in range(k):
            points = [row for row, label in zip(matrix, labels) if label == cluster_id]
            if points:
                centroids[cluster_id] = mean_vector(points, width)
    return labels, centroids


def silhouette_score(matrix: list[list[float]], labels: list[int]) -> float:
    if len(set(labels)) < 2 or len(matrix) < 3:
        return 0.0
    scores = []
    for index, row in enumerate(matrix):
        same = [distance(row, other) for cursor, other in enumerate(matrix) if labels[cursor] == labels[index] and cursor != index]
        other_clusters = sorted(set(labels) - {labels[index]})
        a = sum(same) / len(same) if same else 0.0
        b_values = []
        for cluster_id in other_clusters:
            distances = [distance(row, other) for cursor, other in enumerate(matrix) if labels[cursor] == cluster_id]
            if distances:
                b_values.append(sum(distances) / len(distances))
        b = min(b_values) if b_values else 0.0
        denominator = max(a, b)
        scores.append((b - a) / denominator if denominator else 0.0)
    return sum(scores) / len(scores) if scores else 0.0


def sse(matrix: list[list[float]], labels: list[int], centroids: list[list[float]]) -> float:
    return sum(distance(row, centroids[label]) ** 2 for row, label in zip(matrix, labels))


def information_criteria(matrix: list[list[float]], labels: list[int], centroids: list[list[float]]) -> tuple[float, float]:
    n = max(1, len(matrix))
    dimensions = len(matrix[0]) if matrix else 1
    k = max(1, len(centroids))
    error = max(sse(matrix, labels, centroids), 1e-9)
    params = k * dimensions
    log_likelihood_proxy = n * math.log(error / n)
    bic = log_likelihood_proxy + params * math.log(n)
    aic = log_likelihood_proxy + 2 * params
    return bic, aic


def transition_persistence(labels: list[int]) -> float:
    if len(labels) < 2:
        return 1.0
    same = sum(1 for left, right in zip(labels, labels[1:]) if left == right)
    return same / (len(labels) - 1)


def centroid_profile(rows: list[dict[str, Any]], labels: list[int], cluster_id: int, columns: list[str]) -> dict[str, float]:
    selected = [row for row, label in zip(rows, labels) if label == cluster_id]
    profile: dict[str, float] = {}
    for column in columns:
        values = []
        for row in selected:
            try:
                values.append(float(row.get(column, 0.0)))
            except (TypeError, ValueError):
                pass
        profile[column] = sum(values) / len(values) if values else 0.0
    return profile


def candidate_label(profile: dict[str, float]) -> str:
    atr = profile.get("atr_percentile", profile.get("atr_3", 0.0))
    de = profile.get("directional_efficiency_3", 0.0)
    vol = profile.get("volume_percentile", 0.0)
    rsi = profile.get("rsi_3", 50.0)
    pos = profile.get("range_position", 0.5)
    if atr >= 0.75 and (vol >= 0.7 or rsi <= 30 or rsi >= 70):
        return "primary::ExtremeStress"
    if de >= 0.6 and atr < 0.8:
        return "primary::TrendExpansion"
    if de <= 0.35 and 0.25 <= atr <= 0.7:
        return "primary::RangeConsolidation"
    if rsi <= 35 or rsi >= 65 or pos <= 0.2 or pos >= 0.8:
        return "primary::ReversalBrewing"
    return "primary::Unknown"


def state_summaries(rows: list[dict[str, Any]], labels: list[int], columns: list[str]) -> list[dict[str, Any]]:
    summaries = []
    for cluster_id in sorted(set(labels)):
        profile = centroid_profile(rows, labels, cluster_id, columns)
        summaries.append(
            {
                "state_id": cluster_id,
                "support": sum(1 for label in labels if label == cluster_id),
                "candidate_label": candidate_label(profile),
                "profile": profile,
            }
        )
    return summaries


def evaluate_k_range(rows: list[dict[str, Any]]) -> tuple[list[int], dict[str, dict[str, Any]], int, list[int], list[str]]:
    columns = numeric_feature_columns(rows)
    raw_matrix = matrix_from_rows(rows, columns)
    matrix = standardize(raw_matrix)
    metrics: dict[str, dict[str, Any]] = {}
    best_k = DEFAULT_K_VALUES[0]
    best_labels: list[int] = []
    best_score = -999.0
    for k in DEFAULT_K_VALUES:
        labels, centroids = kmeans(matrix, k)
        score = silhouette_score(matrix, labels)
        bic, aic = information_criteria(matrix, labels, centroids)
        persistence = transition_persistence(labels)
        metrics[str(k)] = {
            "silhouette": score,
            "bic": bic,
            "aic": aic,
            "transition_persistence": persistence,
            "method": "deterministic_kmeans_fallback",
        }
        if score > best_score:
            best_score = score
            best_k = k
            best_labels = labels
    return DEFAULT_K_VALUES, metrics, best_k, best_labels, columns


def load_ontology_labels(path: Path | None) -> list[str]:
    if path is None or not path.exists():
        return []
    payload = json.loads(path.read_text(encoding="utf-8"))
    return [str(expert.get("label_id")) for expert in payload.get("experts", [])]


def build_cluster_discovery_report(*, features_path: Path, ontology_path: Path | None, output_json: Path) -> dict[str, Any]:
    rows = load_feature_rows(features_path)
    k_values, metrics, best_k, labels, columns = evaluate_k_range(rows)
    report = {
        "schema_version": "regime-discovery-cluster/v1",
        "method": "kmeans_or_deterministic_fallback",
        "row_count": len(rows),
        "feature_columns": columns,
        "ontology_labels_read_only": load_ontology_labels(ontology_path),
        "k_values": k_values,
        "k_metrics": metrics,
        "best_k": best_k,
        "states": state_summaries(rows, labels, columns),
    }
    write_json(output_json, report)
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Discover candidate regime clusters from feature table.")
    parser.add_argument("--features", required=True)
    parser.add_argument("--ontology")
    parser.add_argument("--output-json", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_cluster_discovery_report(
        features_path=Path(args.features),
        ontology_path=Path(args.ontology) if args.ontology else None,
        output_json=Path(args.output_json),
    )
    print(json.dumps({"ok": True, "best_k": report["best_k"], "output": args.output_json}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())