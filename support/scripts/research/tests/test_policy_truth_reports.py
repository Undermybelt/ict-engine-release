from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import policy_truth_reports as reports  # noqa: E402


class PolicyTruthReportTests(unittest.TestCase):
    def test_build_report_bundle_marks_missing_validation_as_pending(self) -> None:
        policy_status = {
            "symbol": "DEMO",
            "structural_path_ranking_target": {
                "rows": 0,
                "history_rows": 0,
                "rows_with_propensity_estimate": 0,
                "rows_with_calibrated_path_prob": 0,
                "raw_scored_mature_rows": 0,
                "raw_scored_mature_min_rows": 30,
                "raw_scored_mature_shortfall_rows": 30,
                "production_validation_ready": False,
                "production_validation_rows": 0,
                "production_validation_min_rows": 30,
                "production_validation_shortfall_rows": 30,
                "summary_line": "target export missing",
            },
            "structural_path_ranking_validation": {
                "calibration_ready": False,
                "calibration_quality_ready": False,
                "calibration_status": "not_fitted",
                "raw_scored_mature_rows": 0,
                "raw_scored_mature_min_rows": 30,
                "raw_scored_mature_shortfall_rows": 30,
                "production_validation_ready": False,
                "production_validation_rows": 0,
                "production_validation_min_rows": 30,
                "production_validation_shortfall_rows": 30,
                "summary_line": "validation missing",
            },
            "summary_line": "entry-model training modules mixed",
        }
        validation_summary = {
            "source_reliability": {
                "status": "needs_multiple_sources",
                "ready": False,
                "multi_source_item_count": 0,
                "holdout_status": None,
                "replay_status": None,
                "calibration_status": None,
            },
            "target_policy": {
                "status": "bucket_posterior_empty",
                "context_count": 0,
                "current_model": "symbol:regime:direction_bucket_posterior",
            },
            "delayed_reward": None,
            "live_regime_truth_rule": {
                "status": "enforced",
                "summary": "retrospective outputs are not sufficient",
                "current_state_branch": "temporal_hmm_pre_bayes_nowcast",
            },
        }
        temporal_summary = {
            "summary_line": "duration_mass=0.000 expected_dwell=0.000 break_hazard=0.000",
            "duration_weighted_streak_mass": 0.0,
            "expected_dwell_steps": 0.0,
            "remaining_dwell_steps": 0.0,
            "break_hazard": 0.0,
            "sequence_break_probability": 0.0,
            "sequence_reset_probability": 0.0,
            "sticky_self_transition_strength": 0.0,
            "transition_weighted_observation_mass": 0.0,
            "transition_prior": 0.0,
        }
        recommended_path = {
            "path_id": "path:demo",
            "path_label": "bootstrap_readiness",
            "direction": "observe",
            "experience_prior": 0.5,
            "current_posterior": 0.0,
            "selected_path_probability": 1.0,
        }

        bundle = reports.build_policy_truth_report_bundle(
            symbol="DEMO",
            policy_status=policy_status,
            validation_summary=validation_summary,
            temporal_summary=temporal_summary,
            recommended_path=recommended_path,
        )

        self.assertEqual(
            bundle["policy_correction_report"]["status"],
            "needs_more_history",
        )
        self.assertFalse(bundle["ope_ci"]["ready"])
        self.assertEqual(
            bundle["ope_ci"]["reason"],
            "propensity_weighted_validation_missing",
        )
        self.assertEqual(
            bundle["path_confidence_bounds"]["current_path"]["path_id"],
            "path:demo",
        )
        self.assertEqual(
            bundle["duration_posterior"]["expected_dwell_steps"],
            0.0,
        )
        self.assertEqual(
            bundle["hazard_summary"]["live_regime_truth_rule"]["status"],
            "enforced",
        )

    def test_build_report_bundle_preserves_ready_metrics(self) -> None:
        policy_status = {
            "symbol": "NQ",
            "structural_path_ranking_target": {
                "rows": 80,
                "history_rows": 140,
                "rows_with_propensity_estimate": 65,
                "rows_with_calibrated_path_prob": 60,
                "history_rows_with_path_prob_lower_bound": 50,
                "raw_scored_mature_rows": 42,
                "raw_scored_mature_min_rows": 30,
                "raw_scored_mature_shortfall_rows": 0,
                "production_validation_ready": True,
                "production_validation_rows": 38,
                "production_validation_min_rows": 30,
                "production_validation_shortfall_rows": 0,
                "summary_line": "target ready",
                "trainer_artifact_ready": True,
            },
            "structural_path_ranking_validation": {
                "calibration_ready": True,
                "calibration_quality_ready": True,
                "calibration_status": "ready",
                "raw_scored_mature_rows": 42,
                "raw_scored_mature_min_rows": 30,
                "raw_scored_mature_shortfall_rows": 0,
                "production_validation_ready": True,
                "production_validation_rows": 38,
                "production_validation_min_rows": 30,
                "production_validation_shortfall_rows": 0,
                "summary_line": "validation ready",
            },
            "summary_line": "ready",
        }
        validation_summary = {
            "source_reliability": {
                "status": "ready",
                "ready": True,
                "multi_source_item_count": 18,
                "holdout_status": "ready",
                "replay_status": "ready",
                "calibration_status": "ready",
                "holdout_brier_score": 0.18,
                "replay_brier_score": 0.2,
                "calibration_brier_score": 0.17,
            },
            "target_policy": {
                "status": "bucket_posterior_live",
                "context_count": 12,
                "current_model": "symbol:regime:direction_bucket_posterior",
            },
            "delayed_reward": {
                "status": "ready",
                "resolution_brier_score": 0.21,
                "resolution_observation_count": 34,
            },
            "live_regime_truth_rule": {
                "status": "enforced",
                "summary": "retrospective outputs are not sufficient",
                "current_state_branch": "temporal_hmm_pre_bayes_nowcast",
            },
        }
        temporal_summary = {
            "summary_line": "duration_mass=3.000 expected_dwell=5.000 break_hazard=0.125",
            "duration_weighted_streak_mass": 3.0,
            "expected_dwell_steps": 5.0,
            "remaining_dwell_steps": 2.0,
            "break_hazard": 0.125,
            "sequence_break_probability": 0.22,
            "sequence_reset_probability": 0.11,
            "sticky_self_transition_strength": 0.64,
            "transition_weighted_observation_mass": 8.0,
            "transition_prior": 0.44,
        }
        recommended_path = {
            "path_id": "path:nq",
            "path_label": "long_bias",
            "direction": "Bull",
            "experience_prior": 0.61,
            "current_posterior": 0.72,
            "selected_path_probability": 0.68,
            "path_prob_lower_bound": 0.57,
        }

        bundle = reports.build_policy_truth_report_bundle(
            symbol="NQ",
            policy_status=policy_status,
            validation_summary=validation_summary,
            temporal_summary=temporal_summary,
            recommended_path=recommended_path,
        )

        self.assertEqual(bundle["policy_correction_report"]["status"], "ready_for_review")
        self.assertTrue(bundle["ope_ci"]["ready"])
        self.assertEqual(bundle["ope_ci"]["production_validation_rows"], 38)
        self.assertEqual(
            bundle["path_confidence_bounds"]["current_path"]["path_prob_lower_bound"],
            0.57,
        )
        self.assertEqual(bundle["duration_posterior"]["expected_dwell_steps"], 5.0)
        self.assertEqual(bundle["hazard_summary"]["break_hazard"], 0.125)


if __name__ == "__main__":
    unittest.main()
