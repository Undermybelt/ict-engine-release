from __future__ import annotations

import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import labeling_triple_barrier as labeling  # noqa: E402


class TripleBarrierLabelingTests(unittest.TestCase):
    def test_long_event_hits_take_profit_before_stop(self) -> None:
        rows = [
            {"timestamp": "t0", "open": 100, "high": 101, "low": 99, "close": 100, "side": 1},
            {"timestamp": "t1", "open": 100, "high": 103, "low": 99.5, "close": 102, "side": 0},
            {"timestamp": "t2", "open": 102, "high": 104, "low": 98, "close": 99, "side": 0},
        ]

        labels = labeling.triple_barrier_labels(
            rows,
            pt_mult=0.02,
            sl_mult=0.01,
            max_holding_bars=2,
            cost_bps=0.0,
        )

        self.assertEqual(len(labels), 1)
        event = labels[0]
        self.assertEqual(event["barrier_hit"], "take_profit")
        self.assertEqual(event["exit_index"], 1)
        self.assertAlmostEqual(event["realized_R"], 2.0)
        self.assertEqual(event["meta_label"], 1)
        self.assertGreater(event["mfe"], 0.0)
        self.assertLessEqual(event["mae"], 0.0)

    def test_short_event_hits_stop_loss_and_accounts_for_cost(self) -> None:
        rows = [
            {"timestamp": "t0", "open": 100, "high": 100.5, "low": 99.5, "close": 100, "side": -1},
            {"timestamp": "t1", "open": 100, "high": 101.5, "low": 98.0, "close": 99, "side": 0},
        ]

        labels = labeling.triple_barrier_labels(
            rows,
            pt_mult=0.02,
            sl_mult=0.01,
            max_holding_bars=1,
            cost_bps=10.0,
        )

        event = labels[0]
        self.assertEqual(event["barrier_hit"], "stop_loss")
        self.assertEqual(event["meta_label"], 0)
        self.assertAlmostEqual(event["realized_R"], -1.1, places=6)

    def test_cli_writes_jsonl_labels(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            input_csv = tmp / "events.csv"
            output_jsonl = tmp / "labels.jsonl"
            input_csv.write_text(
                "timestamp,open,high,low,close,side\n"
                "t0,100,101,99,100,1\n"
                "t1,100,103,99.5,102,0\n",
                encoding="utf-8",
            )

            exit_code = labeling.main(
                [
                    "--input-csv",
                    str(input_csv),
                    "--output-jsonl",
                    str(output_jsonl),
                    "--pt-mult",
                    "0.02",
                    "--sl-mult",
                    "0.01",
                    "--max-holding-bars",
                    "1",
                ]
            )

            self.assertEqual(exit_code, 0)
            self.assertIn('"barrier_hit": "take_profit"', output_jsonl.read_text())


if __name__ == "__main__":
    unittest.main()