import csv
import json
import tempfile
import unittest
from pathlib import Path

from scripts.research.selective_risk_control_probe import run_probe


class SelectiveRiskControlProbeTests(unittest.TestCase):
    def test_probe_accepts_rule_with_calibrated_low_bad_loss_risk(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            scan = root / "scan.tsv"
            windows = root / "windows"
            truth = root / "truth.jsonl"
            output = root / "report.json"
            windows.mkdir()

            rows = []
            truth_rows = []
            for index in range(12):
                window = f"{index + 1:02d}"
                timestamp = str(1000 + index)
                (windows / f"nq_15m_obs_{window}.json").write_text(
                    json.dumps({"candles": [{"timestamp": timestamp}]}),
                    encoding="utf-8",
                )
                rows.append(
                    {
                        "window": window,
                        "execution_readiness": "0.60",
                        "prediction_vote_score": "0.80",
                        "ranker_score_raw_path_score": "0.90",
                        "path_ranker_score_used_by_execution_tree": "true",
                        "hybrid_transition_hazard": "0.40",
                        "duration_remaining_expected_bars": "4.0",
                    }
                )
                truth_rows.append(
                    {
                        "timestamp": timestamp,
                        "label_id": "payoff::ForwardWin",
                        "future_ret": 0.002,
                    }
                )

            _write_scan(scan, rows)
            _write_truth(truth, truth_rows)

            report = run_probe(
                scan_tsv=scan,
                windows_dir=windows,
                truth_jsonl=truth,
                output_json=output,
                symbol="NQ",
                bad_loss_floor=-0.001,
                alpha=0.80,
                calibration_fraction=0.5,
                min_calibration_support=5,
                min_test_support=5,
            )

            self.assertTrue(report["accepted_rules"])
            self.assertEqual(report["test"]["accepted_windows"], 6)
            self.assertEqual(report["test"]["bad_losses"], 0)
            self.assertEqual(report["decision"], "accepted_release_rule_found")

    def test_probe_abstains_when_calibration_risk_bound_is_too_high(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            scan = root / "scan.tsv"
            windows = root / "windows"
            truth = root / "truth.jsonl"
            output = root / "report.json"
            windows.mkdir()

            rows = []
            truth_rows = []
            for index in range(12):
                window = f"{index + 1:02d}"
                timestamp = str(2000 + index)
                (windows / f"nq_15m_obs_{window}.json").write_text(
                    json.dumps({"candles": [{"timestamp": timestamp}]}),
                    encoding="utf-8",
                )
                rows.append(
                    {
                        "window": window,
                        "execution_readiness": "0.60",
                        "prediction_vote_score": "0.80",
                        "ranker_score_raw_path_score": "0.90",
                        "path_ranker_score_used_by_execution_tree": "true",
                        "hybrid_transition_hazard": "0.40",
                        "duration_remaining_expected_bars": "4.0",
                    }
                )
                truth_rows.append(
                    {
                        "timestamp": timestamp,
                        "label_id": (
                            "payoff::Invalidated" if index % 2 == 0 else "payoff::ForwardWin"
                        ),
                        "future_ret": -0.002 if index % 2 == 0 else 0.002,
                    }
                )

            _write_scan(scan, rows)
            _write_truth(truth, truth_rows)

            report = run_probe(
                scan_tsv=scan,
                windows_dir=windows,
                truth_jsonl=truth,
                output_json=output,
                symbol="NQ",
                bad_loss_floor=-0.001,
                alpha=0.05,
                calibration_fraction=0.5,
                min_calibration_support=5,
                min_test_support=5,
            )

            self.assertFalse(report["accepted_rules"])
            self.assertEqual(report["decision"], "abstain_no_calibrated_release_rule")
            self.assertEqual(report["test"]["accepted_windows"], 0)


def _write_scan(path: Path, rows):
    fields = [
        "window",
        "execution_readiness",
        "prediction_vote_score",
        "ranker_score_raw_path_score",
        "path_ranker_score_used_by_execution_tree",
        "hybrid_transition_hazard",
        "duration_remaining_expected_bars",
    ]
    with path.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def _write_truth(path: Path, rows):
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row) + "\n")


if __name__ == "__main__":
    unittest.main()
