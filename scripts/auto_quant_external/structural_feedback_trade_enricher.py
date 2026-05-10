from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import pandas as pd


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        rows.append(json.loads(line))
    return rows


def load_pending_templates(path: Path) -> list[dict[str, Any]]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(raw, list):
        return raw
    raise ValueError(f"pending update history at {path} must be a JSON array")


def attach_structural_feedback(
    trade: dict[str, Any],
    pending_template: dict[str, Any],
) -> dict[str, Any]:
    feedback = pending_template.get("template_feedback", {})
    structural_feedback = feedback.get("structural_feedback")
    if not structural_feedback:
        raise ValueError("pending template missing template_feedback.structural_feedback")

    enriched = dict(trade)
    enriched["structural_feedback"] = structural_feedback
    if "model_probabilities_before_trade" not in enriched and feedback.get(
        "model_probabilities_before_trade"
    ):
        enriched["model_probabilities_before_trade"] = feedback["model_probabilities_before_trade"]
    return enriched


def enrich_real_trades_jsonl(
    trades_path: Path,
    pending_update_history_path: Path,
    output_path: Path,
) -> dict[str, int]:
    trades = load_jsonl(trades_path)
    templates = load_pending_templates(pending_update_history_path)

    matched = min(len(trades), len(templates))
    enriched_rows = [
        attach_structural_feedback(trades[index], templates[index])
        for index in range(matched)
    ]
    output_path.write_text(
        "".join(json.dumps(row, ensure_ascii=True) + "\n" for row in enriched_rows),
        encoding="utf-8",
    )
    return {
        "total_trades": len(trades),
        "templates": len(templates),
        "matched": matched,
        "unmatched": max(0, len(trades) - matched),
    }


def _first_present(row: pd.Series, keys: list[str], default: Any = None) -> Any:
    for key in keys:
        if key in row and pd.notna(row[key]):
            return row[key]
    return default


def build_structural_feedback_from_target_row(
    row: pd.Series,
    *,
    realized_outcome: str,
    followed_path: bool = True,
    pnl: float = 0.0,
    exit_reason: str = "manual_probe",
    notes: str = "caller-owned structural feedback probe",
) -> dict[str, Any]:
    symbol = str(_first_present(row, ["symbol"], "NQ"))
    path_id = str(_first_present(row, ["path_id"], "path:unknown"))
    scenario_id = str(_first_present(row, ["scenario_id"], "scenario:unknown"))
    path_label = str(_first_present(row, ["path_label"], "unknown"))
    candidate_set_id = str(_first_present(row, ["candidate_set_id"], "structural-candidates:unknown"))
    recommended_at = str(
        _first_present(
            row,
            ["generated_at"],
            datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        )
    )
    selected_probability = float(
        _first_present(
            row,
            ["behavior_policy_probability", "current_posterior", "raw_path_score", "structural_baseline_score"],
            0.5,
        )
    )
    raw_path_score = float(_first_present(row, ["raw_path_score", "structural_baseline_score"], selected_probability))
    current_posterior = float(_first_present(row, ["current_posterior"], selected_probability))

    return {
        "protocol_version": "structural-feedback-v1",
        "recommendation_id": f"structural-feedback:{symbol}:{candidate_set_id}:{path_id}",
        "recommended_at": recommended_at,
        "symbol": symbol,
        "node_id": str(_first_present(row, ["regime_calibration_bucket"], f"{symbol}:belief_regime_node:unknown")),
        "branch_id": f"{symbol}:path_ranker:{path_label}",
        "scenario_id": scenario_id,
        "path_id": path_id,
        "direction": str(_first_present(row, ["direction"], "Observe")),
        "entry_style": "conditional_execution",
        "candidate_set_id": candidate_set_id,
        "candidate_set_size": int(_first_present(row, ["candidate_set_size"], 0) or 0),
        "selected_path_probability": selected_probability,
        "selected_entry_quality": "medium",
        "selected_entry_quality_probability": 1.0,
        "pre_bayes_gate_status": "pass_neutralized",
        "path_posterior": current_posterior,
        "bbn_support_score": current_posterior,
        "followed_path": followed_path,
        "realized_outcome": realized_outcome,
        "realized_pnl": pnl,
        "exit_reason": exit_reason,
        "notes": notes,
        "model_probabilities_before_trade": {
            "selected_direction": str(_first_present(row, ["direction"], "Observe")),
            "selected_probability": selected_probability,
            "long_score": raw_path_score,
            "short_score": max(0.0, 1.0 - raw_path_score),
            "win_prob_long": selected_probability,
            "win_prob_short": max(0.0, 1.0 - selected_probability),
            "uncertainty": max(0.0, 1.0 - abs(selected_probability - 0.5) * 2.0),
        },
    }


def emit_structural_feedback_probe(
    *,
    target_csv: Path,
    output_path: Path,
    rank: int = 1,
    path_id: str | None = None,
    realized_outcome: str = "win",
    followed_path: bool = True,
    pnl: float = 0.0,
    exit_reason: str = "manual_probe",
    notes: str = "caller-owned structural feedback probe",
) -> dict[str, Any]:
    df = pd.read_csv(target_csv)
    if path_id:
        matches = df[df["path_id"] == path_id]
    else:
        matches = df[df["rank"] == rank] if "rank" in df.columns else df.head(1)
    if matches.empty:
        raise ValueError(f"no structural target row matched rank={rank} path_id={path_id!r}")

    payload = build_structural_feedback_from_target_row(
        matches.iloc[0],
        realized_outcome=realized_outcome,
        followed_path=followed_path,
        pnl=pnl,
        exit_reason=exit_reason,
        notes=notes,
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return {
        "ok": True,
        "output_path": str(output_path),
        "path_id": payload["path_id"],
        "realized_outcome": realized_outcome,
        "followed_path": followed_path,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Structural feedback helper")
    subparsers = parser.add_subparsers(dest="command", required=True)

    enrich = subparsers.add_parser("enrich-trades")
    enrich.add_argument("--trades", required=True)
    enrich.add_argument("--pending-update-history", required=True)
    enrich.add_argument("--output", required=True)

    probe = subparsers.add_parser("emit-probe")
    probe.add_argument("--target-csv", required=True)
    probe.add_argument("--output", required=True)
    probe.add_argument("--rank", type=int, default=1)
    probe.add_argument("--path-id")
    probe.add_argument("--realized-outcome", default="win")
    probe.add_argument("--not-followed", action="store_true")
    probe.add_argument("--pnl", type=float, default=0.0)
    probe.add_argument("--exit-reason", default="manual_probe")
    probe.add_argument("--notes", default="caller-owned structural feedback probe")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if args.command == "enrich-trades":
        summary = enrich_real_trades_jsonl(
            trades_path=Path(args.trades),
            pending_update_history_path=Path(args.pending_update_history),
            output_path=Path(args.output),
        )
    else:
        summary = emit_structural_feedback_probe(
            target_csv=Path(args.target_csv),
            output_path=Path(args.output),
            rank=args.rank,
            path_id=args.path_id,
            realized_outcome=args.realized_outcome,
            followed_path=not args.not_followed,
            pnl=args.pnl,
            exit_reason=args.exit_reason,
            notes=args.notes,
        )
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
