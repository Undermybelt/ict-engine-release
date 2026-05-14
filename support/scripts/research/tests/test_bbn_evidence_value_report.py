from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import bbn_evidence_value_report as report  # noqa: E402


class BbnEvidenceValueReportTests(unittest.TestCase):
    def test_accepts_evidence_edge_when_entropy_and_logloss_improve(self) -> None:
        rows = [
            {
                "edge_id": "vrp_regime_to_fill_viable",
                "prior_prob": 0.55,
                "posterior_prob": 0.82,
                "outcome": 1,
                "contradiction": False,
            },
            {
                "edge_id": "vrp_regime_to_fill_viable",
                "prior_prob": 0.45,
                "posterior_prob": 0.21,
                "outcome": 0,
                "contradiction": True,
            },
        ]

        result = report.build_evidence_value_report(rows=rows, candidate_id="bbn-a")

        self.assertEqual(result["schema_version"], "bbn-evidence-value-report/v1")
        self.assertEqual(result["candidate_id"], "bbn-a")
        self.assertLess(result["posterior_entropy_delta"], 0.0)
        self.assertLess(result["logloss_delta"], 0.0)
        self.assertGreater(result["contradiction_lift"], 0.0)
        self.assertEqual(result["accepted_edges"], ["vrp_regime_to_fill_viable"])
        self.assertEqual(result["rejected_edges"], [])

    def test_rejects_edge_when_posterior_adds_noise(self) -> None:
        rows = [
            {"edge_id": "noisy_edge", "prior_prob": 0.80, "posterior_prob": 0.51, "outcome": 1},
            {"edge_id": "noisy_edge", "prior_prob": 0.20, "posterior_prob": 0.49, "outcome": 0},
        ]

        result = report.build_evidence_value_report(rows=rows)

        self.assertGreater(result["posterior_entropy_delta"], 0.0)
        self.assertGreater(result["logloss_delta"], 0.0)
        self.assertEqual(result["accepted_edges"], [])
        self.assertEqual(result["rejected_edges"], ["noisy_edge"])

    def test_cli_writes_bbn_evidence_value_json(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            rows_jsonl = tmp / "rows.jsonl"
            output_json = tmp / "bbn_value.json"
            rows_jsonl.write_text(
                json.dumps({"edge_id": "edge-a", "prior_prob": 0.6, "posterior_prob": 0.9, "outcome": 1}) + "\n"
                + json.dumps({"edge_id": "edge-a", "prior_prob": 0.4, "posterior_prob": 0.1, "outcome": 0}) + "\n",
                encoding="utf-8",
            )

            exit_code = report.main(
                [
                    "--rows-jsonl",
                    str(rows_jsonl),
                    "--output-json",
                    str(output_json),
                    "--candidate-id",
                    "cli-bbn",
                ]
            )

            self.assertEqual(exit_code, 0)
            self.assertIn('"candidate_id": "cli-bbn"', output_json.read_text())
            self.assertIn('"accepted_edges": [', output_json.read_text())


if __name__ == "__main__":
    unittest.main()