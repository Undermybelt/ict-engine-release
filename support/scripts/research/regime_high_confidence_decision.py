from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

UNKNOWN_MARKERS = ("Unknown", "Neutral", "Transitional")


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _to_float(value: Any, default: float = 0.0) -> float:
    try:
        if value in (None, ""):
            return default
        return float(value)
    except (TypeError, ValueError):
        return default


def _latest_timestamp(score_rows: list[dict[str, Any]], label_prefix: str = "") -> str:
    timestamps = [str(row.get("timestamp", "")) for row in score_rows if not label_prefix or str(row.get("label_id", "")).startswith(label_prefix)]
    return sorted(timestamps)[-1] if timestamps else ""


def _latest_scores(score_rows: list[dict[str, Any]], timestamp: str, label_prefix: str = "") -> list[dict[str, Any]]:
    rows = [row for row in score_rows if str(row.get("timestamp", "")) == timestamp]
    if label_prefix:
        rows = [row for row in rows if str(row.get("label_id", "")).startswith(label_prefix)]
    return rows


def _top_score(rows: list[dict[str, Any]]) -> tuple[str, float]:
    usable = [row for row in rows if not row.get("abstain_reason")]
    if not usable:
        return "", 0.0
    best = max(usable, key=lambda row: _to_float(row.get("score")))
    return str(best.get("label_id", "")), _to_float(best.get("score"))


def _coverage_sets(conformal: dict[str, Any], timestamp: str) -> tuple[list[str], list[str]]:
    sets = conformal.get("sets_by_target_coverage", {})
    set95 = list(sets.get("0.95", {}).get(timestamp, []))
    set99 = list(sets.get("0.99", {}).get(timestamp, []))
    if not set95 and sets.get("0.95"):
        set95 = list(sets.get("0.95", {}).get(sorted(sets.get("0.95", {}))[-1], []))
    if not set99 and sets.get("0.99"):
        set99 = list(sets.get("0.99", {}).get(sorted(sets.get("0.99", {}))[-1], []))
    return set95, set99


def _is_unknown(label: str) -> bool:
    return any(marker in label for marker in UNKNOWN_MARKERS)


def _compact_reasons(*groups: list[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for group in groups:
        for reason in group:
            if reason and reason not in seen:
                seen.add(reason)
                result.append(reason)
    return result


def _user_vrp_nq_context(distribution: dict[str, Any]) -> dict[str, Any]:
    raw = distribution.get("feature_group_summaries", {}).get("user_vrp_nq", {})
    return {key: value.get("latest", value) if isinstance(value, dict) else value for key, value in raw.items()}


def build_high_confidence_decision(
    *,
    scores_path: Path,
    conformal_report_path: Path,
    distributional_report_path: Path,
    governor_report_path: Path,
    output_json: Path,
    label_prefix: str = "",
) -> dict[str, Any]:
    score_rows = _load_jsonl(scores_path)
    conformal = _load_json(conformal_report_path)
    distribution = _load_json(distributional_report_path)
    governor = _load_json(governor_report_path)

    timestamp = str(governor.get("timestamp") or distribution.get("timestamp") or _latest_timestamp(score_rows, label_prefix))
    latest_rows = _latest_scores(score_rows, timestamp, label_prefix=label_prefix)
    top_label, top_score = _top_score(latest_rows)
    current_label = str(governor.get("current_label") or top_label)
    set95, set99 = _coverage_sets(conformal, timestamp)
    primary_set = set99 or set95 or ([current_label] if current_label else [])

    governor_hint = str(governor.get("execution_tree_hint", ""))
    governor_reasons = list(governor.get("guardrail_reasons", []))
    distribution_reasons = list(distribution.get("transitional_reasons", []))
    reasons: list[str] = []
    if _is_unknown(current_label) or governor_hint == "unknown_abstain":
        reasons.append("unknown_label")
    if distribution.get("transitional_flag", False) or governor_hint == "transition_guardrail":
        reasons.append("transitional_or_guardrailed")
    if distribution.get("agreement") != "agree":
        reasons.append("distributional_disagreement")

    confidence95 = bool(conformal.get("confidence_95", False))
    confidence99 = bool(conformal.get("confidence_99", False))
    single95 = len(set95) == 1
    single99 = len(set99) == 1

    if "unknown_label" in reasons:
        decision_state = "unknown_abstain"
        trade_usable = False
    elif "transitional_or_guardrailed" in reasons:
        decision_state = "transitional"
        trade_usable = False
    elif confidence99 and single99 and set99[0] == current_label:
        decision_state = "single_label_99"
        trade_usable = True
    elif confidence95 and single95 and set95[0] == current_label:
        decision_state = "single_label_95"
        trade_usable = True
    elif len(primary_set) > 1:
        decision_state = "label_set"
        trade_usable = False
        reasons.append("wide_or_uncertain_label_set")
    else:
        decision_state = "unknown_abstain"
        trade_usable = False
        reasons.append("confidence_gate_failed")

    final_label = current_label if decision_state in ("single_label_95", "single_label_99") else ""
    abstain_reasons = [] if trade_usable else _compact_reasons(reasons, governor_reasons, distribution_reasons)
    transition_hazard = _to_float(governor.get("transition_hazard"), 0.0)

    execution_tree_hint = "accept_regime" if trade_usable else ("unknown_abstain" if decision_state == "unknown_abstain" else "transition_guardrail")
    bbn_evidence_hint = {
        "regime_decision_state": decision_state,
        "regime_trade_usable": trade_usable,
        "regime_label": final_label or current_label,
        "regime_label_set": primary_set,
        "regime_transition_hazard": transition_hazard,
        "regime_decision_reasons": abstain_reasons,
    }
    path_ranker_context = {
        "regime_label": final_label or current_label,
        "regime_label_set": primary_set,
        "regime_trade_usable": trade_usable,
        "regime_confidence_tier": decision_state,
        "regime_transition_hazard": transition_hazard,
    }

    report = {
        "schema_version": "regime-high-confidence-decision/v1",
        "timestamp": timestamp,
        "label_prefix": label_prefix,
        "decision_state": decision_state,
        "trade_usable": trade_usable,
        "final_label": final_label,
        "label_set": primary_set,
        "top_label": top_label,
        "top_score": top_score,
        "confidence_95": confidence95,
        "confidence_99": confidence99,
        "distributional_agreement": distribution.get("agreement", ""),
        "transition_hazard": transition_hazard,
        "abstain_reasons": abstain_reasons,
        "execution_tree_hint": execution_tree_hint,
        "bbn_evidence_hint": bbn_evidence_hint,
        "path_ranker_context": path_ranker_context,
        "user_vrp_nq_context": _user_vrp_nq_context(distribution),
        "consumer_contract": {
            "zero_config": True,
            "hotplug_scope": "label_prefix",
            "main_runtime_mutation": "none",
            "optional_for_consumers": True,
        },
    }
    _write_json(output_json, report)
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Aggregate regime sidecar evidence into a final high-confidence consumer decision.")
    parser.add_argument("--scores", required=True)
    parser.add_argument("--conformal-report", required=True)
    parser.add_argument("--distributional-report", required=True)
    parser.add_argument("--governor-report", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--label-prefix", default="")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_high_confidence_decision(
        scores_path=Path(args.scores),
        conformal_report_path=Path(args.conformal_report),
        distributional_report_path=Path(args.distributional_report),
        governor_report_path=Path(args.governor_report),
        output_json=Path(args.output_json),
        label_prefix=args.label_prefix,
    )
    print(json.dumps({"ok": True, "output": args.output_json, "decision_state": report["decision_state"], "trade_usable": report["trade_usable"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
