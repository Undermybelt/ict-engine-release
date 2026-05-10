from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_conformal_calibration_report as conformal  # noqa: E402


class RegimeConformalCalibrationReportTests(unittest.TestCase):
    def _write_training_report(self, path: Path) -> None:
        payload = {
            "schema_version": "regime-expert-training/v1",
            "expert_count": 3,
            "experts": [
                {"label_id": "primary::TrendExpansion", "threshold": 0.8, "abstain_policy": "abstain_unless_singleton_conformal_set"},
                {"label_id": "primary::RangeConsolidation", "threshold": 0.8, "abstain_policy": "abstain_unless_singleton_conformal_set"},
                {"label_id": "primary::Unknown", "threshold": 1.0, "abstain_policy": "always_abstain"},
            ],
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def _write_scores(self, path: Path) -> None:
        rows = []
        for idx, truth in enumerate(["primary::TrendExpansion", "primary::TrendExpansion", "primary::RangeConsolidation", "primary::RangeConsolidation"]):
            rows.extend([
                {"timestamp": f"t{idx}", "label_id": "primary::TrendExpansion", "score": 0.94 if truth.endswith("TrendExpansion") else 0.08, "threshold": 0.8, "decision": "positive" if truth.endswith("TrendExpansion") else "negative", "abstain_reason": ""},
                {"timestamp": f"t{idx}", "label_id": "primary::RangeConsolidation", "score": 0.91 if truth.endswith("RangeConsolidation") else 0.11, "threshold": 0.8, "decision": "positive" if truth.endswith("RangeConsolidation") else "negative", "abstain_reason": ""},
                {"timestamp": f"t{idx}", "label_id": "primary::Unknown", "score": 0.01, "threshold": 1.0, "decision": "abstain", "abstain_reason": "always_abstain_label"},
            ])
        path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")

    def _write_truth(self, path: Path) -> None:
        rows = [
            {"timestamp": "t0", "label_id": "primary::TrendExpansion"},
            {"timestamp": "t1", "label_id": "primary::TrendExpansion"},
            {"timestamp": "t2", "label_id": "primary::RangeConsolidation"},
            {"timestamp": "t3", "label_id": "primary::RangeConsolidation"},
        ]
        path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")

    def _write_imbalanced_scores_and_truth(self, scores_path: Path, truth_path: Path) -> None:
        score_rows = []
        truth_rows = []
        for idx in range(100):
            timestamp = f"t{idx}"
            truth = "primary::RangeConsolidation" if idx == 99 else "primary::TrendExpansion"
            truth_rows.append({"timestamp": timestamp, "label_id": truth})
            score_rows.extend([
                {"timestamp": timestamp, "label_id": "primary::TrendExpansion", "score": 0.94, "threshold": 0.8, "decision": "positive", "abstain_reason": ""},
                {"timestamp": timestamp, "label_id": "primary::RangeConsolidation", "score": 0.08, "threshold": 0.8, "decision": "negative", "abstain_reason": ""},
                {"timestamp": timestamp, "label_id": "primary::Unknown", "score": 0.01, "threshold": 1.0, "decision": "abstain", "abstain_reason": "always_abstain_label"},
            ])
        scores_path.write_text("\n".join(json.dumps(row) for row in score_rows) + "\n", encoding="utf-8")
        truth_path.write_text("\n".join(json.dumps(row) for row in truth_rows) + "\n", encoding="utf-8")

    def test_report_emits_coverage_singleton_and_confidence_flags(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "regime_expert_scores.jsonl"
            training = tmp / "regime_expert_training_report.json"
            truth = tmp / "truth.jsonl"
            output = tmp / "regime_conformal_calibration_report.json"
            self._write_scores(scores)
            self._write_training_report(training)
            self._write_truth(truth)

            result = conformal.build_conformal_calibration_report(
                scores_path=scores,
                training_report_path=training,
                truth_path=truth,
                output_json=output,
            )

            self.assertEqual(result["schema_version"], "regime-conformal-calibration/v1")
            self.assertEqual(result["target_coverages"], [0.95, 0.99])
            self.assertEqual(result["singleton_rate"], 1.0)
            self.assertEqual(result["max_conformal_set_size"], 1)
            self.assertTrue(result["confidence_95"])
            self.assertTrue(result["confidence_99"])
            self.assertIn("primary::TrendExpansion", result["class_conditional_coverage"])
            self.assertTrue(output.exists())

    def test_label_prefix_filters_consumer_scope(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "regime_expert_scores.jsonl"
            training = tmp / "regime_expert_training_report.json"
            output = tmp / "regime_conformal_calibration_report.json"
            self._write_scores(scores)
            self._write_training_report(training)

            result = conformal.build_conformal_calibration_report(
                scores_path=scores,
                training_report_path=training,
                output_json=output,
                label_prefix="primary::Trend",
            )

            labels = {row for rows in result["sets_by_target_coverage"]["0.99"].values() for row in rows}
            self.assertTrue(labels)
            self.assertTrue(all(label.startswith("primary::Trend") for label in labels))
            self.assertEqual(result["truth_source"], "missing")
            self.assertFalse(result["confidence_95"])
            self.assertFalse(result["confidence_99"])
            self.assertIn("truth_labels_missing", result["warnings"])

    def test_unknown_labels_remain_non_trade_usable(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "regime_expert_scores.jsonl"
            training = tmp / "regime_expert_training_report.json"
            output = tmp / "regime_conformal_calibration_report.json"
            self._write_scores(scores)
            self._write_training_report(training)

            result = conformal.build_conformal_calibration_report(
                scores_path=scores,
                training_report_path=training,
                output_json=output,
            )

            unknown = result["label_contracts"]["primary::Unknown"]
            self.assertEqual(unknown["trade_usable"], False)
            self.assertEqual(unknown["abstain_policy"], "always_abstain")

    def test_confidence_requires_every_truth_class_to_meet_target_coverage(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "regime_expert_scores.jsonl"
            training = tmp / "regime_expert_training_report.json"
            truth = tmp / "truth.jsonl"
            output = tmp / "regime_conformal_calibration_report.json"
            self._write_imbalanced_scores_and_truth(scores, truth)
            self._write_training_report(training)

            result = conformal.build_conformal_calibration_report(
                scores_path=scores,
                training_report_path=training,
                truth_path=truth,
                output_json=output,
            )

            self.assertEqual(result["overall_coverage"], 0.99)
            self.assertEqual(result["class_conditional_coverage"]["primary::TrendExpansion"]["coverage"], 1.0)
            self.assertEqual(result["class_conditional_coverage"]["primary::RangeConsolidation"]["coverage"], 0.0)
            self.assertFalse(result["confidence_95"])
            self.assertFalse(result["confidence_99"])

    def test_cli_writes_report(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "regime_expert_scores.jsonl"
            training = tmp / "regime_expert_training_report.json"
            truth = tmp / "truth.jsonl"
            output = tmp / "regime_conformal_calibration_report.json"
            self._write_scores(scores)
            self._write_training_report(training)
            self._write_truth(truth)

            exit_code = conformal.main([
                "--scores", str(scores),
                "--training-report", str(training),
                "--truth", str(truth),
                "--output-json", str(output),
            ])

            self.assertEqual(exit_code, 0)
            self.assertIn('"confidence_95"', output.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
