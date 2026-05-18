from __future__ import annotations

import argparse
import json
from pathlib import Path
from statistics import mean
from typing import Any


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _top_result(benchmark: dict[str, Any]) -> dict[str, Any]:
    ranked = benchmark.get("ranked_results") or []
    return ranked[0] if ranked else {}


def build_regime_artifact_bundle(
    *,
    benchmarks: list[dict[str, Any]],
    candidate_id: str,
    display_name: str,
) -> dict[str, Any]:
    market_rows = []
    for benchmark in benchmarks:
        top = _top_result(benchmark)
        market_rows.append(
            {
                "symbol": benchmark.get("symbol"),
                "base_timeframe": benchmark.get("base_timeframe"),
                "bar_count": benchmark.get("bar_count"),
                "truth_mode": benchmark.get("truth_mode"),
                "best_model": top.get("name"),
                "eval_macro_f1": top.get("eval_macro_f1"),
                "eval_covered_precision": top.get("eval_covered_precision"),
                "eval_coverage": top.get("eval_coverage"),
                "transition_f1": top.get("transition_f1"),
                "resonance_4h": top.get("resonance_4h"),
                "resonance_1d": top.get("resonance_1d"),
                "flip_rate": top.get("flip_rate"),
            }
        )

    best_market = max(
        market_rows,
        key=lambda row: float(row.get("eval_macro_f1") or 0.0),
        default={},
    )
    macro_scores = [float(row.get("eval_macro_f1") or 0.0) for row in market_rows]
    covered_precision = [
        float(row.get("eval_covered_precision") or 0.0) for row in market_rows
    ]
    transition_scores = [float(row.get("transition_f1") or 0.0) for row in market_rows]
    resonance_4h = [float(row.get("resonance_4h") or 0.0) for row in market_rows]
    resonance_1d = [float(row.get("resonance_1d") or 0.0) for row in market_rows]

    return {
        "regime_classifier_summary": {
            "schema_version": "regime-classifier-summary/v1",
            "candidate_id": candidate_id,
            "display_name": display_name,
            "market_count": len(market_rows),
            "best_market": best_market.get("symbol"),
            "best_eval_macro_f1": best_market.get("eval_macro_f1"),
            "average_eval_macro_f1": round(mean(macro_scores), 6) if macro_scores else 0.0,
            "average_eval_covered_precision": (
                round(mean(covered_precision), 6) if covered_precision else 0.0
            ),
            "markets": market_rows,
        },
        "transition_summary": {
            "schema_version": "transition-summary/v1",
            "candidate_id": candidate_id,
            "market_count": len(market_rows),
            "best_transition_f1": max(transition_scores) if transition_scores else 0.0,
            "average_transition_f1": round(mean(transition_scores), 6)
            if transition_scores
            else 0.0,
            "markets": [
                {
                    "symbol": row["symbol"],
                    "transition_f1": row["transition_f1"],
                    "flip_rate": row["flip_rate"],
                }
                for row in market_rows
            ],
        },
        "resonance_summary": {
            "schema_version": "resonance-summary/v1",
            "candidate_id": candidate_id,
            "market_count": len(market_rows),
            "max_resonance_4h": max(resonance_4h) if resonance_4h else 0.0,
            "max_resonance_1d": max(resonance_1d) if resonance_1d else 0.0,
            "markets": [
                {
                    "symbol": row["symbol"],
                    "resonance_4h": row["resonance_4h"],
                    "resonance_1d": row["resonance_1d"],
                }
                for row in market_rows
            ],
        },
        "cross_market_summary": {
            "schema_version": "regime-cross-market-summary/v1",
            "candidate_id": candidate_id,
            "covered_markets": [row["symbol"] for row in market_rows if row.get("symbol")],
            "market_count": len(market_rows),
            "timeframes": sorted(
                {
                    row["base_timeframe"]
                    for row in market_rows
                    if row.get("base_timeframe")
                }
            ),
            "truth_modes": sorted(
                {row["truth_mode"] for row in market_rows if row.get("truth_mode")}
            ),
        },
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build a regime-only artifact bundle from existing regime benchmark JSON files."
    )
    parser.add_argument("--candidate-id", required=True)
    parser.add_argument("--display-name", required=True)
    parser.add_argument("--benchmark-json", action="append", required=True)
    parser.add_argument("--output-dir", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    benchmarks = [_load_json(Path(path)) for path in args.benchmark_json]
    bundle = build_regime_artifact_bundle(
        benchmarks=benchmarks,
        candidate_id=args.candidate_id,
        display_name=args.display_name,
    )
    output_dir = Path(args.output_dir).resolve()
    for name, payload in bundle.items():
        _write_json(output_dir / f"{name}.json", payload)
    print(
        json.dumps(
            {
                "ok": True,
                "candidate_id": args.candidate_id,
                "market_count": bundle["cross_market_summary"]["market_count"],
                "output_dir": str(output_dir),
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
