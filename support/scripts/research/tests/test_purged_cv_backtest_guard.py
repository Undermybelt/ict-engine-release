from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import purged_cv_backtest_guard as guard  # noqa: E402


class PurgedCvBacktestGuardTests(unittest.TestCase):
    def test_guard_reports_oos_lcb_and_pbo_for_stable_labels(self) -> None:
        labels = [
            {"entry_index": i * 3, "exit_index": i * 3 + 1, "realized_R": value}
            for i, value in enumerate([0.8, 0.7, -0.2, 0.9, 0.6, -0.1, 1.0, 0.5])
        ]

        report = guard.build_guard_report(labels=labels, nb_trials=12, embargo_bars=1, fold_count=4)

        self.assertEqual(report["schema_version"], "purged-cv-backtest-guard/v1")
        self.assertEqual(report["fold_count"], 4)
        self.assertGreaterEqual(report["pbo"], 0.0)
        self.assertLessEqual(report["pbo"], 1.0)
        self.assertIn(report["purged_cv_gate"], {"pass", "probe"})
        self.assertGreater(report["oos_sharpe_lcb"], 0.0)
        self.assertEqual(report["leakage_flags"], [])

    def test_guard_flags_overlap_and_insufficient_folds(self) -> None:
        labels = [
            {"entry_index": 0, "exit_index": 5, "realized_R": 1.0},
            {"entry_index": 2, "exit_index": 7, "realized_R": -0.5},
        ]

        report = guard.build_guard_report(labels=labels, nb_trials=2, embargo_bars=2, fold_count=4)

        self.assertEqual(report["purged_cv_gate"], "insufficient_data")
        self.assertIn("overlapping_labels", report["leakage_flags"])
        self.assertIn("insufficient_purged_folds", report["leakage_flags"])

    def test_cli_writes_guard_json(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            labels_jsonl = tmp / "labels.jsonl"
            output_json = tmp / "guard.json"
            labels_jsonl.write_text(
                "\n".join(
                    json.dumps({"entry_index": i * 2, "exit_index": i * 2 + 1, "realized_R": 0.5})
                    for i in range(8)
                )
                + "\n",
                encoding="utf-8",
            )

            exit_code = guard.main(
                [
                    "--labels-jsonl",
                    str(labels_jsonl),
                    "--output-json",
                    str(output_json),
                    "--nb-trials",
                    "8",
                    "--embargo-bars",
                    "1",
                    "--fold-count",
                    "4",
                ]
            )

            self.assertEqual(exit_code, 0)
            self.assertIn('"schema_version": "purged-cv-backtest-guard/v1"', output_json.read_text())


if __name__ == "__main__":
    unittest.main()