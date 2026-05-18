from __future__ import annotations

import csv
import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import payoff_to_path_ranker_target as exporter  # noqa: E402


class PayoffToPathRankerTargetTests(unittest.TestCase):
    def test_probe_payoff_exports_target_rows_and_bbn_gate(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            labels = tmp / "labels.jsonl"
            report = tmp / "payoff_report.json"
            out = tmp / "out"
            labels.write_text(
                json.dumps(
                    {
                        "entry_index": 7,
                        "entry_timestamp": "2026-05-09T14:30:00Z",
                        "side": 1,
                        "realized_R": 1.8,
                        "mfe": 0.025,
                        "mae": -0.004,
                        "time_to_hit": 3,
                        "meta_label": 1,
                        "qqq_hv_level": 0.22,
                        "vix3m_level": 18.5,
                        "vvix_over_vix": 5.1,
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            report.write_text(
                json.dumps(
                    {
                        "candidate_id": "vrp-compression",
                        "promotion_gate": "probe",
                        "dsr": 0.87,
                        "psr": 0.94,
                        "sharpe": 2.1,
                        "payoff_shape": "trend_convexity",
                        "failure_tags": ["thin_density"],
                    }
                ),
                encoding="utf-8",
            )

            summary = exporter.export_targets(
                labels_jsonl=labels,
                payoff_report_json=report,
                output_dir=out,
                symbol="NQ",
            )

            self.assertEqual(summary["bbn_gate"]["consume_by_regime_bbn"], True)
            self.assertEqual(summary["target_row_count"], 1)
            with (out / "path_ranker_target.csv").open(newline="", encoding="utf-8") as handle:
                target_rows = list(csv.DictReader(handle))
            self.assertEqual(target_rows[0]["candidate_id"], "vrp-compression")
            self.assertEqual(target_rows[0]["symbol"], "NQ")
            self.assertEqual(target_rows[0]["pending_reward_state"], "matured_success")
            self.assertEqual(target_rows[0]["payoff_gate"], "probe")
            self.assertEqual(target_rows[0]["qqq_hv_level"], "0.22")
            self.assertEqual(target_rows[0]["mae_penalty"], "0.004")
            self.assertEqual(target_rows[0]["time_penalty"], "0.03")
            self.assertEqual(target_rows[0]["regime_confidence_bonus"], "0.0")
            self.assertEqual(target_rows[0]["slippage_penalty"], "0.0")
            self.assertAlmostEqual(float(target_rows[0]["risk_adjusted_path_utility"]), 1.766)
            self.assertTrue((out / "bbn_gate.json").is_file())
            self.assertTrue((out / "path_ranker_target.jsonl").is_file())

    def test_risk_adjusted_utility_rewards_regime_confidence_and_penalizes_slippage(self) -> None:
        label = {
            "realized_R": 1.2,
            "mae": -0.25,
            "time_to_hit": 8,
            "meta_label": 1,
            "regime_confidence": 0.95,
            "slippage_R": 0.12,
        }
        report = {"candidate_id": "utility-factor", "promotion_gate": "promote", "payoff_shape": "right_tail"}

        row = exporter.build_target_row_for_test(label=label, report=report, symbol="NQ")

        self.assertEqual(row["mae_penalty"], 0.25)
        self.assertEqual(row["time_penalty"], 0.08)
        self.assertEqual(row["regime_confidence_bonus"], 0.095)
        self.assertEqual(row["slippage_penalty"], 0.12)
        self.assertAlmostEqual(row["risk_adjusted_path_utility"], 0.845)

    def test_reject_payoff_writes_only_failure_memory(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            labels = tmp / "labels.jsonl"
            report = tmp / "payoff_report.json"
            out = tmp / "out"
            labels.write_text('{"realized_R": -1.0, "meta_label": 0}\n', encoding="utf-8")
            report.write_text(
                json.dumps(
                    {
                        "candidate_id": "bad-factor",
                        "promotion_gate": "reject",
                        "dsr": 0.01,
                        "psr": 0.02,
                        "sharpe": -1.0,
                        "failure_tags": ["negative_edge"],
                    }
                ),
                encoding="utf-8",
            )

            summary = exporter.export_targets(
                labels_jsonl=labels,
                payoff_report_json=report,
                output_dir=out,
                symbol="NQ",
            )

            self.assertEqual(summary["bbn_gate"]["consume_by_regime_bbn"], False)
            self.assertEqual(summary["target_row_count"], 0)
            self.assertFalse((out / "path_ranker_target.csv").exists())
            failure = json.loads((out / "failure_memory.jsonl").read_text(encoding="utf-8").splitlines()[0])
            self.assertEqual(failure["candidate_id"], "bad-factor")
            self.assertEqual(failure["memory_type"], "payoff_reject")


if __name__ == "__main__":
    unittest.main()