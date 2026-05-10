from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


BASE_SEEDS: list[dict[str, Any]] = [
    {
        "seed_id": "qlib_alpha158_momentum_roc",
        "family": "momentum",
        "source": "qlib_alpha158_style",
        "expression": "roc(close, n) * rank(volume / mean(volume, n))",
        "required_fields": ["close", "volume"],
        "default_params": {"n": 20},
        "allowed_regimes": ["TrendExpansion", "HighLiquidity"],
        "mutation_hints": {"n": [10, 20, 40], "volume_weight": [0.5, 1.0, 1.5]},
        "hotplug_ready": True,
    },
    {
        "seed_id": "qlib_alpha158_vol_breakout",
        "family": "volatility_breakout",
        "source": "qlib_alpha158_style",
        "expression": "zscore(true_range, n) * sign(close - rolling_high(close, n))",
        "required_fields": ["high", "low", "close"],
        "default_params": {"n": 20},
        "allowed_regimes": ["TrendExpansion", "ExtremeStress"],
        "mutation_hints": {"n": [14, 20, 50], "z_threshold": [1.0, 1.5, 2.0]},
        "hotplug_ready": True,
    },
    {
        "seed_id": "alpha101_rank_decay_reversion",
        "family": "mean_reversion",
        "source": "alpha101_operator_skeleton",
        "expression": "-rank(decay_linear(delta(close, d), n))",
        "required_fields": ["close"],
        "default_params": {"d": 3, "n": 10},
        "allowed_regimes": ["RangeConsolidation", "ReversalBrewing"],
        "mutation_hints": {"d": [1, 3, 5], "n": [5, 10, 20]},
        "hotplug_ready": True,
    },
    {
        "seed_id": "alpha101_corr_liquidity_pressure",
        "family": "liquidity",
        "source": "alpha101_operator_skeleton",
        "expression": "-rank(correlation(rank(close), rank(volume), n))",
        "required_fields": ["close", "volume"],
        "default_params": {"n": 10},
        "allowed_regimes": ["ThinLiquidity", "HighLiquidity"],
        "mutation_hints": {"n": [5, 10, 20], "sign": [-1, 1]},
        "hotplug_ready": True,
    },
    {
        "seed_id": "vrp_compression_regime",
        "family": "options_vrp",
        "source": "ict_engine_vrp_v2",
        "expression": "zscore(vix3m_level, n) - zscore(qqq_hv_level, n) + rank(vvix_over_vix)",
        "required_fields": ["vix3m_level", "qqq_hv_level", "vvix_over_vix"],
        "default_params": {"n": 60},
        "allowed_regimes": ["RangeConsolidation", "ReversalBrewing"],
        "mutation_hints": {"n": [30, 60, 120], "vvix_weight": [0.5, 1.0, 1.5]},
        "hotplug_ready": True,
    },
    {
        "seed_id": "ict_fvg_reclaim_quality",
        "family": "structure_ict",
        "source": "ict_engine_structure_family",
        "expression": "fvg_reclaim_score * liquidity_sweep_score * mtf_alignment",
        "required_fields": ["fvg_reclaim_score", "liquidity_sweep_score", "mtf_alignment"],
        "default_params": {"min_alignment": 0.5},
        "allowed_regimes": ["TrendExpansion", "ReversalBrewing"],
        "mutation_hints": {"min_alignment": [0.3, 0.5, 0.7], "sweep_weight": [0.5, 1.0, 1.5]},
        "hotplug_ready": True,
    },
    {
        "seed_id": "crowding_reversal_pressure",
        "family": "crowding",
        "source": "crowded_trades_skeleton",
        "expression": "rank(volume_zscore) * rank(rsi_extreme) * -sign(recent_return)",
        "required_fields": ["volume", "rsi", "close"],
        "default_params": {"lookback": 14},
        "allowed_regimes": ["ReversalBrewing", "ExtremeStress"],
        "mutation_hints": {"lookback": [7, 14, 28], "extreme_threshold": [0.8, 0.9, 0.95]},
        "hotplug_ready": True,
    },
]


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=False) + "\n")


def build_formula_library(families: list[str] | None = None) -> dict[str, Any]:
    family_filter = {family.strip() for family in families or [] if family.strip()}
    seeds = [dict(seed) for seed in BASE_SEEDS if not family_filter or seed["family"] in family_filter]
    return {
        "schema_version": "factor-formula-library/v1",
        "seed_count": len(seeds),
        "families": sorted({seed["family"] for seed in seeds}),
        "seeds": seeds,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Export a hot-pluggable factor formula seed library.")
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--output-jsonl")
    parser.add_argument("--family", action="append", default=[])
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    payload = build_formula_library(families=args.family)
    _write_json(Path(args.output_json), payload)
    if args.output_jsonl:
        _write_jsonl(Path(args.output_jsonl), payload["seeds"])
    print(json.dumps({"ok": True, "output": args.output_json, "seed_count": payload["seed_count"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())