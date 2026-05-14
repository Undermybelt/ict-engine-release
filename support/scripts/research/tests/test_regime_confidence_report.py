from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_confidence_report as report  # noqa: E402


class RegimeConfidenceReportTests(unittest.TestCase):
    def test_confidence_95_passes_for_singleton_well_calibrated_rows(self) -> None:
        rows = [
            {"timestamp": "t0", "truth": "trend", "posterior": {"trend": 0.96, "range": 0.04}, "transition_prob": 0.05},
            {"timestamp": "t1", "truth": "trend", "posterior": {"trend": 0.95, "range": 0.05}, "transition_prob": 0.08},
            {"timestamp": "t2", "truth": "range", "posterior": {"trend": 0.03, "range": 0.97}, "transition_prob": 0.04},
            {"timestamp": "t3", "truth": "range", "posterior": {"trend": 0.04, "range": 0.96}, "transition_prob": 0.05},
        ]

        result = report.build_confidence_report(rows=rows, candidate_id="regime-a")

        self.assertEqual(result["schema_version"], "regime-confidence-report/v1")
        self.assertEqual(result["candidate_id"], "regime-a")
        self.assertTrue(result["confidence_95"])
        self.assertEqual(result["conformal_set_size"], 1)
        self.assertGreaterEqual(result["rolling_coverage"], 0.93)
        self.assertLessEqual(result["calibration_ece"], 0.05)
        self.assertLessEqual(result["transition_prob"], 0.2)
        self.assertEqual(result["regime_confidence_gate"], "pass")

    def test_confidence_95_fails_for_ambiguous_or_flipping_rows(self) -> None:
        rows = [
            {"timestamp": "t0", "truth": "trend", "posterior": {"trend": 0.55, "range": 0.45}, "transition_prob": 0.4},
            {"timestamp": "t1", "truth": "range", "posterior": {"trend": 0.52, "range": 0.48}, "transition_prob": 0.5},
            {"timestamp": "t2", "truth": "trend", "posterior": {"trend": 0.51, "range": 0.49}, "transition_prob": 0.6},
        ]

        result = report.build_confidence_report(rows=rows, candidate_id="regime-b")

        self.assertFalse(result["confidence_95"])
        self.assertGreater(result["conformal_set_size"], 1)
        self.assertGreater(result["transition_prob"], 0.2)
        self.assertEqual(result["regime_confidence_gate"], "reject")

    def test_cli_writes_report_json(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            rows_jsonl = tmp / "rows.jsonl"
            output_json = tmp / "regime_confidence_report.json"
            rows_jsonl.write_text(
                json.dumps({"truth": "trend", "posterior": {"trend": 0.96, "range": 0.04}, "transition_prob": 0.05}) + "\n"
                + json.dumps({"truth": "range", "posterior": {"trend": 0.04, "range": 0.96}, "transition_prob": 0.05}) + "\n",
                encoding="utf-8",
            )

            exit_code = report.main(
                [
                    "--rows-jsonl",
                    str(rows_jsonl),
                    "--output-json",
                    str(output_json),
                    "--candidate-id",
                    "cli-regime",
                ]
            )

            self.assertEqual(exit_code, 0)
            self.assertIn('"candidate_id": "cli-regime"', output_json.read_text())


if __name__ == "__main__":
    unittest.main()