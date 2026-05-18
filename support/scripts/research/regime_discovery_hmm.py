from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import regime_discovery_cluster as cluster


def _hmm_metrics_from_cluster_metrics(metrics: dict[str, dict[str, Any]]) -> dict[str, dict[str, Any]]:
    adjusted: dict[str, dict[str, Any]] = {}
    for key, value in metrics.items():
        adjusted[key] = {
            "bic": value["bic"],
            "aic": value["aic"],
            "silhouette": value["silhouette"],
            "transition_persistence": value["transition_persistence"],
            "method": "gaussian_hmm_proxy_via_sequential_kmeans_fallback",
        }
    return adjusted


def build_hmm_discovery_report(*, features_path: Path, ontology_path: Path | None, output_json: Path) -> dict[str, Any]:
    rows = cluster.load_feature_rows(features_path)
    k_values, metrics, best_k, labels, columns = cluster.evaluate_k_range(rows)
    hmm_metrics = _hmm_metrics_from_cluster_metrics(metrics)
    # HMM sidecar is intentionally read-only against ontology; it proposes labels only.
    report = {
        "schema_version": "regime-discovery-hmm/v1",
        "method": "gaussian_hmm_or_sequential_kmeans_fallback",
        "row_count": len(rows),
        "feature_columns": columns,
        "ontology_labels_read_only": cluster.load_ontology_labels(ontology_path),
        "k_values": k_values,
        "k_metrics": hmm_metrics,
        "best_k": best_k,
        "states": cluster.state_summaries(rows, labels, columns),
    }
    cluster.write_json(output_json, report)
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Discover candidate HMM-like regime states from feature table.")
    parser.add_argument("--features", required=True)
    parser.add_argument("--ontology")
    parser.add_argument("--output-json", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_hmm_discovery_report(
        features_path=Path(args.features),
        ontology_path=Path(args.ontology) if args.ontology else None,
        output_json=Path(args.output_json),
    )
    print(json.dumps({"ok": True, "best_k": report["best_k"], "output": args.output_json}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())