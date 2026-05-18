from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path
from typing import Any

DEFAULT_THRESHOLDS = {
    "min_entropy_improvement": 0.01,
    "min_logloss_improvement": 0.01,
    "min_contradiction_lift": 0.0,
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


def _float(row: dict[str, Any], key: str, default: float = 0.0) -> float:
    try:
        return float(row.get(key, default))
    except (TypeError, ValueError):
        return default


def _clamp_prob(value: float) -> float:
    return min(1.0 - 1e-12, max(1e-12, value))


def _binary_entropy(prob: float) -> float:
    p = _clamp_prob(prob)
    return -(p * math.log(p) + (1.0 - p) * math.log(1.0 - p))


def _logloss(prob: float, outcome: int) -> float:
    p = _clamp_prob(prob)
    return -math.log(p if outcome else 1.0 - p)


def _mean(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def _edge_metrics(rows: list[dict[str, Any]]) -> dict[str, Any]:
    prior_entropy = [_binary_entropy(_float(row, "prior_prob", 0.5)) for row in rows]
    posterior_entropy = [_binary_entropy(_float(row, "posterior_prob", 0.5)) for row in rows]
    prior_logloss: list[float] = []
    posterior_logloss: list[float] = []
    contradiction_improvements: list[float] = []

    for row in rows:
        outcome = int(_float(row, "outcome", 0.0) >= 0.5)
        prior_loss = _logloss(_float(row, "prior_prob", 0.5), outcome)
        posterior_loss = _logloss(_float(row, "posterior_prob", 0.5), outcome)
        prior_logloss.append(prior_loss)
        posterior_logloss.append(posterior_loss)
        if bool(row.get("contradiction", False)):
            contradiction_improvements.append(prior_loss - posterior_loss)

    return {
        "posterior_entropy_delta": _mean(posterior_entropy) - _mean(prior_entropy),
        "logloss_delta": _mean(posterior_logloss) - _mean(prior_logloss),
        "contradiction_lift": _mean(contradiction_improvements),
        "sample_count": len(rows),
    }


def build_evidence_value_report(
    *,
    rows: list[dict[str, Any]],
    candidate_id: str = "",
    thresholds: dict[str, float] | None = None,
) -> dict[str, Any]:
    merged = dict(DEFAULT_THRESHOLDS)
    if thresholds:
        merged.update(thresholds)

    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        edge_id = str(row.get("edge_id", "edge")).strip() or "edge"
        grouped[edge_id].append(row)

    edge_details: list[dict[str, Any]] = []
    accepted_edges: list[str] = []
    rejected_edges: list[str] = []
    for edge_id in sorted(grouped):
        metrics = _edge_metrics(grouped[edge_id])
        accepted = (
            metrics["posterior_entropy_delta"] <= -float(merged["min_entropy_improvement"])
            and metrics["logloss_delta"] <= -float(merged["min_logloss_improvement"])
            and metrics["contradiction_lift"] >= float(merged["min_contradiction_lift"])
        )
        if accepted:
            accepted_edges.append(edge_id)
        else:
            rejected_edges.append(edge_id)
        edge_details.append({"edge_id": edge_id, "accepted": accepted, **metrics})

    aggregate = _edge_metrics(rows) if rows else {
        "posterior_entropy_delta": 0.0,
        "logloss_delta": 0.0,
        "contradiction_lift": 0.0,
        "sample_count": 0,
    }
    return {
        "schema_version": "bbn-evidence-value-report/v1",
        "candidate_id": candidate_id,
        **aggregate,
        "accepted_edges": accepted_edges,
        "rejected_edges": rejected_edges,
        "edge_details": edge_details,
        "thresholds": merged,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Score BBN evidence edges by entropy, logloss, and contradiction lift.")
    parser.add_argument("--rows-jsonl", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--candidate-id", default="")
    parser.add_argument("--min-entropy-improvement", type=float, default=DEFAULT_THRESHOLDS["min_entropy_improvement"])
    parser.add_argument("--min-logloss-improvement", type=float, default=DEFAULT_THRESHOLDS["min_logloss_improvement"])
    parser.add_argument("--min-contradiction-lift", type=float, default=DEFAULT_THRESHOLDS["min_contradiction_lift"])
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    result = build_evidence_value_report(
        rows=_load_jsonl(Path(args.rows_jsonl)),
        candidate_id=args.candidate_id,
        thresholds={
            "min_entropy_improvement": args.min_entropy_improvement,
            "min_logloss_improvement": args.min_logloss_improvement,
            "min_contradiction_lift": args.min_contradiction_lift,
        },
    )
    _write_json(Path(args.output_json), result)
    print(json.dumps({"ok": True, "output": args.output_json, "accepted_edges": result["accepted_edges"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())