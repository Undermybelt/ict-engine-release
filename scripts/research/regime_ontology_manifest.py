from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

PRIMARY_LABELS = [
    "TrendExpansion",
    "RangeConsolidation",
    "ExtremeStress",
    "ReversalBrewing",
    "Unknown",
]

SECONDARY_LABELS = [
    "BullTrendAcceleration",
    "BearTrendAcceleration",
    "BullTrendExhaustion",
    "BearTrendExhaustion",
    "TightRange",
    "WideRange",
    "Accumulation",
    "Distribution",
    "VolatilitySpike",
    "LiquidityCrunch",
    "PanicSelling",
    "PanicBuying",
    "TrendFatigue",
    "SentimentExtreme",
    "StructureBreakdown",
    "Unknown",
]

DIMENSION_LABELS = {
    "volatility": ["LowVol", "NormalVol", "ElevatedVol", "CrisisVol", "Unknown"],
    "liquidity": ["HighLiquidity", "NormalLiquidity", "ThinLiquidity", "Unknown"],
    "structure": [
        "Trending",
        "MeanReverting",
        "Ranging",
        "Accumulation",
        "Distribution",
        "Breakout",
        "Breakdown",
        "Unknown",
    ],
    "behavior": ["Crowding", "Exhaustion", "FOMO", "Capitulation", "RiskOn", "RiskOff", "Neutral"],
}

TRANSITION_LABELS = [
    "StayTrendExpansion",
    "TrendExpansionToRangeConsolidation",
    "TrendExpansionToReversalBrewing",
    "RangeConsolidationToTrendExpansion",
    "RangeConsolidationToExtremeStress",
    "ReversalBrewingToTrendExpansion",
    "ExtremeStressToNormalizing",
    "AnyToUnknownTransitional",
]

FEATURES_BY_LEVEL = {
    "primary": [
        "atr_percentile",
        "directional_efficiency",
        "adx",
        "volume_percentile",
        "rsi",
        "mtf_alignment",
        "transition_hazard",
    ],
    "secondary": [
        "slope_r2",
        "range_percentile",
        "vol_of_vol",
        "liquidity_sweep_score",
        "fvg_reclaim_score",
        "momentum_fade",
        "sentiment_extreme_score",
    ],
    "dimension": [
        "ohlcv",
        "atr_percentile",
        "volume_percentile",
        "range_percentile",
        "adx",
        "rsi",
    ],
    "transition": [
        "previous_regime",
        "current_regime_probability",
        "hmm_transition_probability",
        "drift_flag",
        "duration_bars",
        "entropy",
    ],
}

PARENT_BY_SECONDARY = {
    "BullTrendAcceleration": "TrendExpansion",
    "BearTrendAcceleration": "TrendExpansion",
    "BullTrendExhaustion": "TrendExpansion",
    "BearTrendExhaustion": "TrendExpansion",
    "TightRange": "RangeConsolidation",
    "WideRange": "RangeConsolidation",
    "Accumulation": "RangeConsolidation",
    "Distribution": "RangeConsolidation",
    "VolatilitySpike": "ExtremeStress",
    "LiquidityCrunch": "ExtremeStress",
    "PanicSelling": "ExtremeStress",
    "PanicBuying": "ExtremeStress",
    "TrendFatigue": "ReversalBrewing",
    "SentimentExtreme": "ReversalBrewing",
    "StructureBreakdown": "ReversalBrewing",
    "Unknown": "Unknown",
}


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=False) + "\n")


def _confidence_contract(label: str, level: str) -> dict[str, Any]:
    is_unknown = label in {"Unknown", "Neutral"} or "Unknown" in label or "Transitional" in label
    return {
        "target_coverage": 0.99 if level == "transition" and not is_unknown else 0.95,
        "abstain_policy": "always_abstain" if is_unknown else "abstain_unless_singleton_conformal_set",
        "min_support": 0 if is_unknown else 100,
        "promotion_gates": [
            "purged_cv_embargo",
            "class_conditional_conformal_coverage",
            "low_entropy",
            "top1_top2_margin",
            "distributional_agreement",
            "bbn_logloss_delta_negative",
        ],
    }


def _expert(
    *,
    label_id: str,
    level: str,
    label: str,
    parent_label: str = "",
    required_features: list[str] | None = None,
) -> dict[str, Any]:
    contract = _confidence_contract(label, level)
    return {
        "label_id": label_id,
        "level": level,
        "label": label,
        "parent_label": parent_label,
        "positive_definition": f"one-vs-rest detector for {label_id}",
        "negative_definition": f"all non-{label} labels at level {level}, excluding abstain rows when configured",
        "required_features": required_features or FEATURES_BY_LEVEL[level],
        "allowed_data_sources": ["ohlcv", "auxiliary_evidence", "mtf_pda_events", "hmm_state", "distributional_archetype"],
        **contract,
    }


def _primary_experts() -> list[dict[str, Any]]:
    return [
        _expert(label_id=f"primary::{label}", level="primary", label=label)
        for label in PRIMARY_LABELS
    ]


def _secondary_experts() -> list[dict[str, Any]]:
    return [
        _expert(
            label_id=f"secondary::{label}",
            level="secondary",
            label=label,
            parent_label=PARENT_BY_SECONDARY[label],
        )
        for label in SECONDARY_LABELS
    ]


def _dimension_experts() -> list[dict[str, Any]]:
    experts: list[dict[str, Any]] = []
    for dimension, labels in DIMENSION_LABELS.items():
        for label in labels:
            experts.append(
                _expert(
                    label_id=f"{dimension}::{label}",
                    level="dimension",
                    label=label,
                    parent_label=dimension,
                )
            )
    return experts


def _transition_experts() -> list[dict[str, Any]]:
    return [
        _expert(label_id=f"transition::{label}", level="transition", label=label)
        for label in TRANSITION_LABELS
    ]


def build_manifest() -> dict[str, Any]:
    primary = _primary_experts()
    secondary = _secondary_experts()
    dimension = _dimension_experts()
    transition = _transition_experts()
    experts = primary + secondary + dimension + transition
    return {
        "schema_version": "regime-ontology-manifest/v1",
        "source_runtime_files": [
            "src/market_state/mod.rs",
            "src/market_state/volatility.rs",
            "src/market_state/liquidity.rs",
            "src/market_state/structure.rs",
            "src/market_state/behavior.rs",
        ],
        "counts": {
            "primary": len(primary),
            "secondary": len(secondary),
            "dimension": len(dimension),
            "transition": len(transition),
        },
        "expert_count": len(experts),
        "experts": experts,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Export ICT regime ontology and high-confidence expert bank manifest.")
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--output-jsonl", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    payload = build_manifest()
    _write_json(Path(args.output_json), payload)
    _write_jsonl(Path(args.output_jsonl), payload["experts"])
    print(json.dumps({"ok": True, "expert_count": payload["expert_count"], "output": args.output_json}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
