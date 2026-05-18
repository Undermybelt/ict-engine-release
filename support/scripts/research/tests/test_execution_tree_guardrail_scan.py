from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import execution_tree_guardrail_scan as scan  # noqa: E402


class ExecutionTreeGuardrailScanTests(unittest.TestCase):
    def test_summarize_trace_extracts_guardrail_metrics(self) -> None:
        trace = {
            "output": {
                "execution_score": 0.49695555545028114,
                "branch": "transition_guardrail",
                "execution_bias": "guarded",
                "gate_status": "observe",
                "branch_probability": 0.0,
                "decision_hint": "execution_guarded_due_to_low_remaining_regime_duration",
                "consumer_reason": "market_state=TrendExpansion/BullTrendExhaustion | execution=observe/transition_guardrail/guarded | ranker=history/catboost/ready",
                "path_ranker_score_used_by_execution_tree": True,
                "path_ranker_score_visible_to_execution_tree": True,
                "path_ranker_model_family": "catboost",
                "path_ranker_runtime_source": "history",
                "ranker_validation_ready": True,
                "execution_shap_top_k": [
                    {
                        "feature": "cycle_phase_alignment",
                        "contribution": -0.9383,
                        "feature_value": "-0.9383",
                    },
                    {
                        "feature": "branch_probability",
                        "contribution": 0.5,
                        "feature_value": "1.0000",
                    },
                    {
                        "feature": "pythagorean_overstretch",
                        "contribution": -0.3,
                        "feature_value": "1.0000",
                    },
                ],
                "split_reason_lineage": [
                    "execution_readiness=0.3631 \u2192 gate_status=blocked",
                    "prediction_vote_score=0.4188 (medium) \u00d7 execution_readiness=0.3631 (weak) \u2192 bias=skip",
                    "hybrid_transition_hazard=0.627",
                    "duration_remaining_expected_bars=0.000",
                    "ranker_score=path_id=path:scenario:NQ:belief_regime_node:range:range_mean_reversion:primary runtime_source=registered_artifact_path raw_path_score=0.807741 calibrated_path_prob=none path_prob_lower_bound=none execution_gate_status=observe",
                ],
            }
        }

        row = scan.summarize_window("80", {}, trace)

        self.assertEqual(row["window"], "80")
        self.assertEqual(row["gate_status"], "observe")
        self.assertEqual(row["branch"], "transition_guardrail")
        self.assertEqual(row["path_ranker_runtime_source"], "history")
        self.assertEqual(row["path_ranker_model_family"], "catboost")
        self.assertEqual(row["ranker_validation_ready"], "true")
        self.assertEqual(row["path_ranker_score_used_by_execution_tree"], "true")
        self.assertEqual(row["path_ranker_score_visible_to_execution_tree"], "true")
        self.assertEqual(
            row["ranker_score_path_id"],
            "path:scenario:NQ:belief_regime_node:range:range_mean_reversion:primary",
        )
        self.assertEqual(row["ranker_score_runtime_source"], "registered_artifact_path")
        self.assertEqual(row["ranker_score_raw_path_score"], "0.807741")
        self.assertEqual(row["ranker_score_calibrated_path_prob"], "")
        self.assertEqual(row["ranker_score_path_prob_lower_bound"], "")
        self.assertEqual(row["ranker_score_execution_gate_status"], "observe")
        self.assertEqual(row["execution_readiness"], "0.363100")
        self.assertEqual(row["prediction_vote_score"], "0.418800")
        self.assertEqual(row["hybrid_transition_hazard"], "0.627000")
        self.assertEqual(row["duration_remaining_expected_bars"], "0.000000")
        self.assertEqual(row["branch_probability"], "0.000000")
        self.assertEqual(row["readiness_gap_to_observe"], "0.086900")
        self.assertEqual(row["readiness_gap_to_ready"], "0.286900")
        self.assertEqual(row["top_positive_feature"], "branch_probability")
        self.assertEqual(row["top_positive_contribution"], "0.500000")
        self.assertEqual(row["top_negative_feature"], "cycle_phase_alignment")
        self.assertEqual(row["top_negative_contribution"], "-0.938300")
        self.assertEqual(row["pythagorean_overstretch"], "1.000000")

    def test_run_scan_saves_per_window_trace_and_tsv_metrics(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            windows = tmp / "windows"
            state = tmp / "state"
            output = tmp / "scan"
            windows.mkdir()
            (windows / "nq_15m_obs_01.json").write_text("{}", encoding="utf-8")
            (windows / "nq_15m_obs_02.json").write_text("{}", encoding="utf-8")

            calls: list[list[str]] = []

            class Result:
                returncode = 0
                stderr = ""

                def __init__(self, index: int) -> None:
                    self.stdout = json.dumps(
                        {
                            "execution_triage": {
                                "gate_status": "observe",
                                "branch": "transition_guardrail",
                                "execution_bias": "guarded",
                                "execution_score": 0.5 + index / 100.0,
                                "branch_probability": 0.0,
                                "decision_hint": "execution_guarded_due_to_high_transition_hazard",
                            }
                        }
                    )

            def fake_runner(command: list[str], **_: object) -> Result:
                calls.append(command)
                index = len(calls)
                trace_dir = state / "NQ"
                trace_dir.mkdir(parents=True, exist_ok=True)
                (trace_dir / "execution_tree_trace.json").write_text(
                    json.dumps(
                        {
                            "output": {
                                "gate_status": "observe",
                                "branch": "transition_guardrail",
                                "execution_bias": "guarded",
                                "execution_score": 0.5 + index / 100.0,
                                "branch_probability": 0.0,
                                "decision_hint": "execution_guarded_due_to_high_transition_hazard",
                                "split_reason_lineage": [
                                    f"execution_readiness=0.{index}000 \u2192 gate_status=blocked",
                                    f"hybrid_transition_hazard=0.{index}500",
                                    f"duration_remaining_expected_bars={index}.250",
                                ],
                            }
                        }
                    ),
                    encoding="utf-8",
                )
                return Result(index)

            summary = scan.run_scan(
                ict_engine_bin=Path("/fake/ict-engine"),
                windows_dir=windows,
                state_dir=state,
                symbol="NQ",
                output_dir=output,
                runner=fake_runner,
            )

            self.assertEqual(summary["windows_scanned"], 2)
            self.assertEqual(summary["metric_summary"]["execution_readiness"]["count"], 2)
            self.assertEqual(summary["metric_summary"]["execution_readiness"]["min"], 0.1)
            self.assertEqual(summary["metric_summary"]["execution_readiness"]["max"], 0.2)
            self.assertEqual(summary["metric_summary"]["hybrid_transition_hazard"]["count"], 2)
            self.assertEqual(summary["metric_summary"]["duration_remaining_expected_bars"]["max"], 2.25)
            self.assertEqual(len(calls), 2)
            self.assertTrue((output / "execution_tree_trace_01.json").exists())
            self.assertTrue((output / "execution_tree_trace_02.json").exists())
            tsv = (output / "scan.tsv").read_text(encoding="utf-8")
            self.assertIn("execution_readiness", tsv)
            self.assertIn("hybrid_transition_hazard", tsv)
            self.assertIn("duration_remaining_expected_bars", tsv)
            self.assertIn("top_negative_feature", tsv)
            self.assertIn("readiness_gap_to_observe", tsv)
            self.assertIn("0.100000", tsv)
            self.assertIn("0.250000", tsv)


if __name__ == "__main__":
    unittest.main()
