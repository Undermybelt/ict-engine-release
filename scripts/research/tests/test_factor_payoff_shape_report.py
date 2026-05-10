from __future__ import annotations

import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import factor_payoff_shape_report as payoff  # noqa: E402


class FactorPayoffShapeReportTests(unittest.TestCase):
    def test_report_identifies_positive_trend_convexity_after_costs(self) -> None:
        trades = [
            {"realized_R": 2.0, "side": 1},
            {"realized_R": -1.0, "side": 1},
            {"realized_R": 3.0, "side": 1},
            {"realized_R": -1.0, "side": -1},
            {"realized_R": 2.5, "side": 1},
        ]

        report = payoff.build_payoff_shape_report(
            candidate_id="trend-a",
            trades=trades,
            nb_trials=10,
            periods_per_year=252,
        )

        self.assertEqual(report["candidate_id"], "trend-a")
        self.assertEqual(report["trade_count"], 5)
        self.assertAlmostEqual(report["hit_rate"], 0.6)
        self.assertGreater(report["avg_win"], abs(report["avg_loss"]))
        self.assertEqual(report["payoff_shape"], "trend_convexity")
        self.assertGreater(report["sharpe"], 0.0)
        self.assertIn(report["promotion_gate"], {"probe", "promote"})

    def test_report_rejects_cost_blind_negative_edge(self) -> None:
        trades = [
            {"gross_R": 0.2, "cost_R": 0.3},
            {"gross_R": 0.1, "cost_R": 0.2},
            {"gross_R": 0.4, "cost_R": 0.6},
        ]

        report = payoff.build_payoff_shape_report(
            candidate_id="bad-carry",
            trades=trades,
            nb_trials=3,
            periods_per_year=252,
        )

        self.assertLess(report["net_return_R"], 0.0)
        self.assertIn("cost_blind_negative_edge", report["failure_tags"])
        self.assertEqual(report["promotion_gate"], "reject")

    def test_cli_writes_report_json(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            trades_jsonl = tmp / "trades.jsonl"
            output_json = tmp / "report.json"
            trades_jsonl.write_text(
                '{"realized_R": 1.5}\n{"realized_R": -0.5}\n',
                encoding="utf-8",
            )

            exit_code = payoff.main(
                [
                    "--candidate-id",
                    "cli-candidate",
                    "--trades-jsonl",
                    str(trades_jsonl),
                    "--output-json",
                    str(output_json),
                    "--nb-trials",
                    "2",
                ]
            )

            self.assertEqual(exit_code, 0)
            self.assertIn('"candidate_id": "cli-candidate"', output_json.read_text())


    def test_report_includes_probabilistic_and_deflated_sharpe(self) -> None:
        trades = [{"realized_R": value} for value in [1.0, 1.2, 0.8, -0.2, 1.1, 0.9]]

        report = payoff.build_payoff_shape_report(
            candidate_id="dsr-demo",
            trades=trades,
            nb_trials=25,
            periods_per_year=252,
        )

        self.assertGreaterEqual(report["psr"], 0.0)
        self.assertLessEqual(report["psr"], 1.0)
        self.assertGreaterEqual(report["dsr"], 0.0)
        self.assertLessEqual(report["dsr"], 1.0)
        self.assertGreater(report["deflated_sharpe_benchmark"], 0.0)
        self.assertEqual(report["effective_trials"], 25)

    def test_report_exposes_r23_payoff_gate_fields_and_failure_tags(self) -> None:
        trades = [
            {"realized_R": value}
            for value in [
                0.30,
                0.20,
                0.25,
                0.15,
                0.35,
                -0.10,
                0.22,
                0.18,
                -3.0,
            ]
        ]

        report = payoff.build_payoff_shape_report(
            candidate_id="tail-risk-candidate",
            trades=trades,
            nb_trials=200,
            periods_per_year=252,
        )

        for key in [
            "sortino",
            "calmar",
            "cvar_95",
            "tail_ratio",
            "profit_factor",
            "avg_rr",
            "oos_sharpe_lcb",
            "pbo",
            "effective_sample_size",
        ]:
            self.assertIn(key, report)
        self.assertIn("high_pbo", report["failure_tags"])
        self.assertIn("low_dsr", report["failure_tags"])
        self.assertIn("tail_risk_hidden", report["failure_tags"])


if __name__ == "__main__":
    unittest.main()