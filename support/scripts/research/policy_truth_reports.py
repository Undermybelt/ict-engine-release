from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def _path_confidence_status(validation: dict[str, Any]) -> str:
    if validation.get("production_validation_ready"):
        return "ready_for_review"
    if validation.get("raw_scored_mature_rows", 0) > 0:
        return "partial_history_only"
    return "needs_more_history"


def build_policy_truth_report_bundle(
    *,
    symbol: str,
    policy_status: dict[str, Any],
    validation_summary: dict[str, Any],
    temporal_summary: dict[str, Any],
    recommended_path: dict[str, Any],
) -> dict[str, Any]:
    target = policy_status.get("structural_path_ranking_target", {})
    validation = policy_status.get("structural_path_ranking_validation", {})
    source_reliability = validation_summary.get("source_reliability", {})
    target_policy = validation_summary.get("target_policy", {})
    delayed_reward = validation_summary.get("delayed_reward")
    live_regime_truth_rule = validation_summary.get("live_regime_truth_rule", {})

    policy_correction_report = {
        "schema_version": "policy-correction-report/v1",
        "symbol": symbol,
        "status": _path_confidence_status(validation),
        "summary_line": policy_status.get("summary_line", ""),
        "target_export_summary": target.get("summary_line", ""),
        "validation_summary": validation.get("summary_line", ""),
        "rows": target.get("rows", 0),
        "history_rows": target.get("history_rows", 0),
        "rows_with_propensity_estimate": target.get("rows_with_propensity_estimate", 0),
        "rows_with_calibrated_path_prob": target.get("rows_with_calibrated_path_prob", 0),
        "calibration_ready": validation.get("calibration_ready", False),
        "calibration_quality_ready": validation.get("calibration_quality_ready", False),
        "production_validation_ready": validation.get(
            "production_validation_ready", False
        ),
        "production_validation_rows": validation.get("production_validation_rows", 0),
        "production_validation_min_rows": validation.get(
            "production_validation_min_rows", 0
        ),
        "production_validation_shortfall_rows": validation.get(
            "production_validation_shortfall_rows", 0
        ),
        "source_reliability_status": source_reliability.get("status"),
        "source_reliability_ready": source_reliability.get("ready", False),
    }

    ope_ci_ready = bool(
        validation.get("production_validation_ready")
        and target.get("rows_with_propensity_estimate", 0) > 0
    )
    ope_ci = {
        "schema_version": "ope-ci/v1",
        "symbol": symbol,
        "ready": ope_ci_ready,
        "reason": (
            "ready"
            if ope_ci_ready
            else "propensity_weighted_validation_missing"
        ),
        "calibration_status": validation.get("calibration_status"),
        "production_validation_rows": validation.get("production_validation_rows", 0),
        "production_validation_min_rows": validation.get(
            "production_validation_min_rows", 0
        ),
        "production_validation_shortfall_rows": validation.get(
            "production_validation_shortfall_rows", 0
        ),
        "raw_scored_mature_rows": validation.get("raw_scored_mature_rows", 0),
        "raw_scored_mature_min_rows": validation.get("raw_scored_mature_min_rows", 0),
        "raw_scored_mature_shortfall_rows": validation.get(
            "raw_scored_mature_shortfall_rows", 0
        ),
        "source_reliability_holdout_status": source_reliability.get("holdout_status"),
        "source_reliability_replay_status": source_reliability.get("replay_status"),
        "source_reliability_calibration_status": source_reliability.get(
            "calibration_status"
        ),
    }

    path_confidence_bounds = {
        "schema_version": "path-confidence-bounds/v1",
        "symbol": symbol,
        "current_path": {
            "path_id": recommended_path.get("path_id"),
            "path_label": recommended_path.get("path_label"),
            "direction": recommended_path.get("direction"),
            "experience_prior": recommended_path.get("experience_prior"),
            "current_posterior": recommended_path.get("current_posterior"),
            "selected_path_probability": recommended_path.get(
                "selected_path_probability"
            ),
            "path_prob_lower_bound": recommended_path.get("path_prob_lower_bound"),
        },
        "calibration_ready": validation.get("calibration_ready", False),
        "calibration_quality_ready": validation.get("calibration_quality_ready", False),
        "trainer_ready": target.get("trainer_artifact_ready", False),
        "history_rows_with_path_prob_lower_bound": target.get(
            "history_rows_with_path_prob_lower_bound", 0
        ),
        "target_policy_status": target_policy.get("status"),
        "target_policy_context_count": target_policy.get("context_count", 0),
    }

    duration_posterior = {
        "schema_version": "duration-posterior/v1",
        "symbol": symbol,
        "summary_line": temporal_summary.get("summary_line", ""),
        "duration_weighted_streak_mass": temporal_summary.get(
            "duration_weighted_streak_mass", 0.0
        ),
        "expected_dwell_steps": temporal_summary.get("expected_dwell_steps", 0.0),
        "remaining_dwell_steps": temporal_summary.get("remaining_dwell_steps", 0.0),
        "transition_weighted_observation_mass": temporal_summary.get(
            "transition_weighted_observation_mass", 0.0
        ),
        "transition_prior": temporal_summary.get("transition_prior", 0.0),
        "target_policy_status": target_policy.get("status"),
    }

    hazard_summary = {
        "schema_version": "hazard-summary/v1",
        "symbol": symbol,
        "break_hazard": temporal_summary.get("break_hazard", 0.0),
        "sequence_break_probability": temporal_summary.get(
            "sequence_break_probability", 0.0
        ),
        "sequence_reset_probability": temporal_summary.get(
            "sequence_reset_probability", 0.0
        ),
        "sticky_self_transition_strength": temporal_summary.get(
            "sticky_self_transition_strength", 0.0
        ),
        "delayed_reward": delayed_reward,
        "live_regime_truth_rule": live_regime_truth_rule,
    }

    return {
        "policy_correction_report": policy_correction_report,
        "ope_ci": ope_ci,
        "path_confidence_bounds": path_confidence_bounds,
        "duration_posterior": duration_posterior,
        "hazard_summary": hazard_summary,
    }


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(payload, indent=2, sort_keys=False) + "\n",
        encoding="utf-8",
    )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build reviewable policy/delayed-truth report artifacts from repo JSON surfaces."
    )
    parser.add_argument("--symbol", required=True)
    parser.add_argument("--policy-status-json", required=True)
    parser.add_argument("--validation-summary-json", required=True)
    parser.add_argument("--temporal-summary-json", required=True)
    parser.add_argument("--recommended-path-json", required=True)
    parser.add_argument("--output-dir", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    bundle = build_policy_truth_report_bundle(
        symbol=args.symbol,
        policy_status=_load_json(Path(args.policy_status_json)),
        validation_summary=_load_json(Path(args.validation_summary_json)),
        temporal_summary=_load_json(Path(args.temporal_summary_json)),
        recommended_path=_load_json(Path(args.recommended_path_json)),
    )
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    for artifact_name, payload in bundle.items():
        _write_json(output_dir / f"{artifact_name}.json", payload)
    print(
        json.dumps(
            {
                "ok": True,
                "symbol": args.symbol,
                "output_dir": str(output_dir),
                "artifacts": [f"{name}.json" for name in bundle],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
