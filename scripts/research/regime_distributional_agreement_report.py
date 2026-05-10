from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from statistics import mean
from typing import Any


FEATURE_FIELDS = ["atr_percentile", "directional_efficiency_3", "volume_percentile", "rsi_3", "range_position"]
USER_VRP_NQ_FIELDS = ["qqq_hv_level", "nq_vs_200d_pct", "vix3m_level", "qqq_hv_pct_rank_252", "vvix_over_vix"]

ARCHETYPES = {
    "primary::TrendExpansion": {"atr_percentile": 0.35, "directional_efficiency_3": 0.82, "volume_percentile": 0.72, "rsi_3": 62.0, "range_position": 0.78},
    "primary::RangeConsolidation": {"atr_percentile": 0.35, "directional_efficiency_3": 0.2, "volume_percentile": 0.45, "rsi_3": 50.0, "range_position": 0.5},
    "primary::ExtremeStress": {"atr_percentile": 0.9, "directional_efficiency_3": 0.65, "volume_percentile": 0.95, "rsi_3": 25.0, "range_position": 0.15},
    "primary::ReversalBrewing": {"atr_percentile": 0.7, "directional_efficiency_3": 0.25, "volume_percentile": 0.8, "rsi_3": 72.0, "range_position": 0.9},
}


def _to_float(value: Any, default: float = 0.0) -> float:
    try:
        if value in (None, ""):
            return default
        return float(value)
    except (TypeError, ValueError):
        return default


def _load_rows(path: Path) -> list[dict[str, Any]]:
    if path.suffix.lower() == ".csv":
        with path.open(newline="", encoding="utf-8") as handle:
            return [dict(row) for row in csv.DictReader(handle)]
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _latest_timestamp(rows: list[dict[str, Any]]) -> str:
    return str(rows[-1].get("timestamp", "")) if rows else ""


def _top_label(score_rows: list[dict[str, Any]], label_prefix: str = "") -> str:
    usable = [row for row in score_rows if not label_prefix or str(row.get("label_id", "")).startswith(label_prefix)]
    usable = [row for row in usable if not row.get("abstain_reason")]
    if not usable:
        return ""
    return str(max(usable, key=lambda row: _to_float(row.get("score"))).get("label_id", ""))


def _current_vector(features: list[dict[str, Any]], window: int) -> dict[str, float]:
    rows = features[-window:] if window > 0 else features
    return {
        field: mean(_to_float(row.get(field)) for row in rows)
        for field in FEATURE_FIELDS
        if any(field in row for row in rows)
    }


def _distance(left: dict[str, float], right: dict[str, float]) -> float:
    fields = [field for field in FEATURE_FIELDS if field in left and field in right]
    if not fields:
        return 1.0
    total = 0.0
    for field in fields:
        scale = 100.0 if field == "rsi_3" else 1.0
        total += abs(left[field] - right[field]) / scale
    return round(total / len(fields), 6)


def _feature_group_summaries(features: list[dict[str, Any]], window: int) -> dict[str, dict[str, Any]]:
    rows = features[-window:] if window > 0 else features
    def summarize(fields: list[str]) -> dict[str, Any]:
        return {
            field: {"latest": _to_float(rows[-1].get(field)) if rows else 0.0, "mean": round(mean(_to_float(row.get(field)) for row in rows), 6) if rows else 0.0}
            for field in fields
            if any(field in row for row in rows)
        }
    return {
        "core_regime": summarize(FEATURE_FIELDS),
        "user_vrp_nq": summarize(USER_VRP_NQ_FIELDS),
    }


def build_distributional_agreement_report(
    *,
    features_path: Path,
    scores_path: Path,
    conformal_report_path: Path,
    output_json: Path,
    label_prefix: str = "",
    window: int = 3,
) -> dict[str, Any]:
    features = _load_rows(features_path)
    score_rows = _load_rows(scores_path)
    conformal = _load_json(conformal_report_path)
    timestamp = _latest_timestamp(features)
    latest_scores = [row for row in score_rows if str(row.get("timestamp", "")) == timestamp] or score_rows
    top_label = _top_label(latest_scores, label_prefix=label_prefix)
    vector = _current_vector(features, window)
    archetypes = {label: archetype for label, archetype in ARCHETYPES.items() if not label_prefix or label.startswith(label_prefix)}
    if top_label and top_label not in archetypes and top_label.startswith("primary::"):
        archetypes[top_label] = ARCHETYPES.get(top_label, {})
    distances = {label: _distance(vector, archetype) for label, archetype in archetypes.items() if archetype}
    nearest_label = min(distances, key=distances.get) if distances else ""
    nearest_distance = distances.get(nearest_label, 1.0)
    conformal_set = conformal.get("sets_by_target_coverage", {}).get("0.99", {}).get(timestamp, [])
    agreement = "agree" if top_label and top_label == nearest_label and (not conformal_set or top_label in conformal_set) else "disagree"
    sorted_distances = sorted(distances.values())
    mixed_archetype = len(sorted_distances) >= 2 and abs(sorted_distances[1] - sorted_distances[0]) <= 0.08
    transitional_flag = bool(nearest_distance >= 0.35 or mixed_archetype or len(conformal_set) > 1)
    report = {
        "schema_version": "regime-distributional-agreement/v1",
        "timestamp": timestamp,
        "label_prefix": label_prefix,
        "method": "quantile_energy_proxy_fallback",
        "top_label": top_label,
        "nearest_archetype_label": nearest_label,
        "nearest_distance": nearest_distance,
        "label_distances": distances,
        "agreement": agreement,
        "transitional_flag": transitional_flag,
        "transitional_reasons": [
            reason for reason, active in {
                "high_distributional_distance": nearest_distance >= 0.35,
                "mixed_archetype_distance": mixed_archetype,
                "wide_conformal_set": len(conformal_set) > 1,
            }.items() if active
        ],
        "feature_group_summaries": _feature_group_summaries(features, window),
        "conformal_context": {
            "confidence_95": bool(conformal.get("confidence_95", False)),
            "confidence_99": bool(conformal.get("confidence_99", False)),
            "set_size": len(conformal_set),
        },
    }
    _write_json(output_json, report)
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Compare regime feature distribution against ICT label archetypes.")
    parser.add_argument("--features", required=True)
    parser.add_argument("--scores", required=True)
    parser.add_argument("--conformal-report", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--label-prefix", default="")
    parser.add_argument("--window", type=int, default=3)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_distributional_agreement_report(
        features_path=Path(args.features),
        scores_path=Path(args.scores),
        conformal_report_path=Path(args.conformal_report),
        output_json=Path(args.output_json),
        label_prefix=args.label_prefix,
        window=args.window,
    )
    print(json.dumps({"ok": True, "output": args.output_json, "top_label": report["top_label"], "agreement": report["agreement"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())