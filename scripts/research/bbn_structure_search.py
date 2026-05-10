#!/usr/bin/env python3
"""Constrained external BBN structure-candidate generator.

This is deliberately minimal:
- reads explicit structure-learning rows JSONL
- keeps the current repo vocabulary fixed
- emits one review/import candidate artifact
- does not apply anything to runtime
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


CURRENT_EDGES = [
    ("market_regime", "entry_quality"),
    ("liquidity_context", "entry_quality"),
    ("factor_alignment", "entry_quality"),
    ("factor_uncertainty", "entry_quality"),
    ("multi_timeframe_resonance", "entry_quality"),
    ("entry_quality", "trade_outcome"),
    ("factor_alignment", "trade_outcome"),
    ("factor_uncertainty", "trade_outcome"),
]

OPTIONAL_TRADE_OUTCOME_PARENTS = [
    "multi_timeframe_resonance",
    "liquidity_context",
    "market_regime",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate a constrained review-only BBN structure candidate artifact."
    )
    parser.add_argument("--rows-jsonl", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--max-parent-count", type=int, default=3)
    parser.add_argument(
        "--backend",
        default="pgmpy_hc",
        choices=[
            "pgmpy_hc",
            "pgmpy_ges",
            "bnlearn_hc",
            "bnlearn_tabu",
            "gobnilp_oracle",
        ],
        help="Constrained offline search backend label",
    )
    return parser.parse_args()


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        rows.append(json.loads(line))
    return rows


def source_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def conditional_hit_rate(rows: list[dict[str, Any]], parent: str) -> float:
    if not rows:
        return 0.0
    supportive = 0
    total = 0
    for row in rows:
        outcome = str(row.get("trade_outcome", "")).strip()
        parent_value = str(row.get(parent, "")).strip()
        if not outcome or not parent_value:
            continue
        total += 1
        if (
            (parent_value in {"aligned", "favorable", "bull"} and outcome == "win")
            or (parent_value in {"dislocated", "hostile", "bear"} and outcome == "loss")
        ):
            supportive += 1
    return supportive / total if total else 0.0


def backend_status(backend: str) -> tuple[str, str]:
    if backend.startswith("pgmpy"):
        try:
            __import__("pgmpy")
            return backend, "available"
        except ImportError:
            return backend, "heuristic_fallback"
    if backend.startswith("bnlearn"):
        try:
            __import__("bnlearn")
            return backend, "available"
        except ImportError:
            return backend, "heuristic_fallback"
    if backend == "gobnilp_oracle":
        return backend, "heuristic_fallback"
    return backend, "heuristic_fallback"


def build_candidate(
    rows: list[dict[str, Any]], path: Path, max_parent_count: int, backend: str
) -> dict[str, Any]:
    edges = [{"parent": parent, "child": child} for parent, child in CURRENT_EDGES]
    score = 0.0
    best_optional_parent = None
    best_optional_score = 0.0
    for parent in OPTIONAL_TRADE_OUTCOME_PARENTS:
        value = conditional_hit_rate(rows, parent)
        if value > best_optional_score:
            best_optional_score = value
            best_optional_parent = parent
    if best_optional_parent and best_optional_score >= 0.55 and max_parent_count >= 4:
        edges.append({"parent": best_optional_parent, "child": "trade_outcome"})
    score = best_optional_score if best_optional_parent else 0.0
    backend_name, backend_mode = backend_status(backend)
    candidate = {
        "protocol_version": "bbn-structure-candidate-v1",
        "required_edges_satisfied": True,
        "forbidden_edges_violated": [],
        "max_parent_count": max_parent_count,
        "score_name": f"{backend_name}:{backend_mode}:conditional_hit_rate",
        "score_value": score,
        "structure_edges": edges,
        "cpt_overrides": {},
        "source_dataset_hash": source_hash(path),
    }
    return candidate


def main() -> None:
    args = parse_args()
    rows_path = Path(args.rows_jsonl).expanduser().resolve()
    if not rows_path.exists():
        raise SystemExit(f"rows jsonl not found: {rows_path}")
    rows = load_rows(rows_path)
    candidate = build_candidate(rows, rows_path, args.max_parent_count, args.backend)
    out_path = Path(args.out).expanduser().resolve()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(candidate, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
