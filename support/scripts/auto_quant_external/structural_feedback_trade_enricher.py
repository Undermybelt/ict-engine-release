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


def _clean_text(value: Any) -> str | None:
    if value is None or pd.isna(value):
        return None
    text = str(value).strip()
    return text or None


def _branch_segments(branch_path: str | None) -> list[str]:
    if not branch_path:
        return []
    return [part.strip() for part in branch_path.split(" -> ") if part.strip()]


def _branch_path_from_row(row: pd.Series) -> str | None:
    explicit = _clean_text(_first_present(row, ["regime_profit_branch_path"]))
    if explicit:
        return explicit
    main = _clean_text(_first_present(row, ["main_regime", "parent_regime_root"]))
    sub = _clean_text(_first_present(row, ["sub_regime"]))
    sub_sub = _clean_text(_first_present(row, ["sub_sub_regime_or_profit_factor"]))
    profit = _clean_text(_first_present(row, ["profit_factor"]))
    if main and sub and sub_sub and profit:
        return f"{main} -> {sub} -> {sub_sub} -> {profit}"
    path_id = _clean_text(_first_present(row, ["path_id"]))
    if path_id and " -> " in path_id:
        return path_id
    return None


def _is_present(value: Any) -> bool:
    return value is not None and value != "" and value != [] and value != {}


def _branch_path_from_mapping(row: dict[str, Any]) -> str | None:
    explicit = row.get("regime_profit_branch_path") or row.get("branch_path")
    if explicit:
        return str(explicit)
    main = row.get("main_regime") or row.get("parent_regime_root")
    sub = row.get("sub_regime")
    sub_sub = row.get("sub_sub_regime_or_profit_factor")
    profit = row.get("profit_factor")
    if main and sub and sub_sub and profit:
        return f"{main} -> {sub} -> {sub_sub} -> {profit}"
    return None


def _fill_branch_fields(row: dict[str, Any]) -> dict[str, Any]:
    branch_path = _branch_path_from_mapping(row)
    if not branch_path:
        return row
    parts = _branch_segments(branch_path)
    row["regime_profit_branch_path"] = branch_path
    row.setdefault("branch_path", branch_path)
    if parts:
        row.setdefault("main_regime", parts[0])
        row.setdefault("parent_regime_root", parts[0])
    if len(parts) > 1:
        row.setdefault("sub_regime", parts[1])
    if len(parts) > 2:
        row.setdefault("sub_sub_regime_or_profit_factor", parts[2])
    if len(parts) > 3:
        row.setdefault("profit_factor", " -> ".join(parts[3:]))
    return row


def enrich_trade_with_layer_contract(
    trade: dict[str, Any],
    *,
    auto_quant_run_id: str,
    symbol: str | None = None,
    provider_provenance: dict[str, Any],
    pre_bayes_filter_state: dict[str, Any],
    bbn_posterior: dict[str, Any],
    catboost_path_ranker_label: dict[str, Any],
    execution_tree_decision: dict[str, Any],
    failure_reason: str,
    quality_weight: float,
) -> dict[str, Any]:
    required = {
        "auto_quant_run_id": auto_quant_run_id,
        "provider_provenance": provider_provenance,
        "pre_bayes_filter_state": pre_bayes_filter_state,
        "bbn_posterior": bbn_posterior,
        "catboost_path_ranker_label": catboost_path_ranker_label,
        "execution_tree_decision": execution_tree_decision,
        "failure_reason": failure_reason,
        "quality_weight": quality_weight,
    }
    missing = [key for key, value in required.items() if not _is_present(value)]
    if missing:
        raise ValueError(f"missing layer contract fields: {', '.join(missing)}")

    enriched = dict(trade)
    enriched["auto_quant_run_id"] = auto_quant_run_id
    if symbol:
        enriched["symbol"] = symbol
    enriched["provider_provenance"] = provider_provenance
    enriched["pre_bayes_filter_state"] = pre_bayes_filter_state
    enriched["bbn_posterior"] = bbn_posterior
    enriched["catboost_path_ranker_label"] = catboost_path_ranker_label
    enriched["execution_tree_decision"] = execution_tree_decision
    enriched["failure_reason"] = failure_reason
    enriched["quality_weight"] = quality_weight
    enriched["layer_contract_version"] = "board-b-layered-feedback-v1"
    return _fill_branch_fields(enriched)


def _json_arg(value: str) -> dict[str, Any]:
    path = Path(value)
    if path.exists():
        return json.loads(path.read_text(encoding="utf-8"))
    loaded = json.loads(value)
    if not isinstance(loaded, dict):
        raise ValueError("JSON argument must decode to an object")
    return loaded


def enrich_real_trades_jsonl_with_layer_contract(
    *,
    trades_path: Path,
    output_path: Path,
    auto_quant_run_id: str,
    symbol: str | None,
    provider_provenance: dict[str, Any],
    pre_bayes_filter_state: dict[str, Any],
    bbn_posterior: dict[str, Any],
    catboost_path_ranker_label: dict[str, Any],
    execution_tree_decision: dict[str, Any],
    failure_reason: str,
    quality_weight: float,
) -> dict[str, Any]:
    trades = load_jsonl(trades_path)
    enriched_rows = [
        enrich_trade_with_layer_contract(
            trade,
            auto_quant_run_id=auto_quant_run_id,
            symbol=symbol,
            provider_provenance=provider_provenance,
            pre_bayes_filter_state=pre_bayes_filter_state,
            bbn_posterior=bbn_posterior,
            catboost_path_ranker_label=catboost_path_ranker_label,
            execution_tree_decision=execution_tree_decision,
            failure_reason=failure_reason,
            quality_weight=quality_weight,
        )
        for trade in trades
    ]
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        "".join(json.dumps(row, ensure_ascii=True) + "\n" for row in enriched_rows),
        encoding="utf-8",
    )
    return {
        "total_trades": len(trades),
        "enriched": len(enriched_rows),
        "output_path": str(output_path),
    }


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
    branch_path = _branch_path_from_row(row)
    branch_parts = _branch_segments(branch_path)
    main_regime = _clean_text(_first_present(row, ["main_regime", "parent_regime_root"]))
    sub_regime = _clean_text(_first_present(row, ["sub_regime"]))
    sub_sub_regime = _clean_text(_first_present(row, ["sub_sub_regime_or_profit_factor"]))
    profit_factor = _clean_text(_first_present(row, ["profit_factor"]))
    if branch_parts:
        main_regime = main_regime or branch_parts[0]
        sub_regime = sub_regime or (branch_parts[1] if len(branch_parts) > 1 else None)
        sub_sub_regime = sub_sub_regime or (branch_parts[2] if len(branch_parts) > 2 else None)
        profit_factor = profit_factor or (" -> ".join(branch_parts[3:]) if len(branch_parts) > 3 else None)

    path_id = branch_path or str(_first_present(row, ["path_id"], "path:unknown"))
    branch_id = (
        f"{main_regime} -> {sub_regime}"
        if main_regime and sub_regime
        else f"{symbol}:path_ranker:{_first_present(row, ['path_label'], 'unknown')}"
    )
    scenario_id = (
        f"{branch_id} -> {sub_sub_regime}"
        if main_regime and sub_regime and sub_sub_regime
        else str(_first_present(row, ["scenario_id"], "scenario:unknown"))
    )
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

    payload = {
        "protocol_version": "structural-feedback-v1",
        "recommendation_id": f"structural-feedback:{symbol}:{candidate_set_id}:{path_id}",
        "recommended_at": recommended_at,
        "symbol": symbol,
        "node_id": main_regime or str(_first_present(row, ["regime_calibration_bucket"], f"{symbol}:belief_regime_node:unknown")),
        "branch_id": branch_id,
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
    if branch_path:
        payload.update(
            {
                "regime_profit_branch_path": branch_path,
                "parent_regime_root": main_regime,
                "main_regime": main_regime,
                "sub_regime": sub_regime,
                "sub_sub_regime_or_profit_factor": sub_sub_regime,
                "profit_factor": profit_factor,
            }
        )
    return payload


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

    layer = subparsers.add_parser("enrich-layer-contract")
    layer.add_argument("--trades", required=True)
    layer.add_argument("--output", required=True)
    layer.add_argument("--auto-quant-run-id", required=True)
    layer.add_argument("--symbol")
    layer.add_argument("--provider-provenance-json", required=True)
    layer.add_argument("--pre-bayes-filter-state-json", required=True)
    layer.add_argument("--bbn-posterior-json", required=True)
    layer.add_argument("--catboost-path-ranker-label-json", required=True)
    layer.add_argument("--execution-tree-decision-json", required=True)
    layer.add_argument("--failure-reason", required=True)
    layer.add_argument("--quality-weight", type=float, required=True)

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
    elif args.command == "enrich-layer-contract":
        summary = enrich_real_trades_jsonl_with_layer_contract(
            trades_path=Path(args.trades),
            output_path=Path(args.output),
            auto_quant_run_id=args.auto_quant_run_id,
            symbol=args.symbol,
            provider_provenance=_json_arg(args.provider_provenance_json),
            pre_bayes_filter_state=_json_arg(args.pre_bayes_filter_state_json),
            bbn_posterior=_json_arg(args.bbn_posterior_json),
            catboost_path_ranker_label=_json_arg(args.catboost_path_ranker_label_json),
            execution_tree_decision=_json_arg(args.execution_tree_decision_json),
            failure_reason=args.failure_reason,
            quality_weight=args.quality_weight,
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
