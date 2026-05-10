from __future__ import annotations

import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import pandas as pd

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import entry_drought_diagnostic_v2 as drought  # noqa: E402
import external_regime_changepoint_labels as changepoint  # noqa: E402
import structural_feedback_trade_enricher as enricher  # noqa: E402
import structural_feedback_replay_harness as replay  # noqa: E402


class ChangePointHelperTests(unittest.TestCase):
    def test_cluster_breakpoints_merges_nearby_votes(self) -> None:
        clusters = changepoint.cluster_breakpoints(
            {
                "pelt": [10, 30, 60],
                "binseg": [11, 29, 61],
                "window": [30, 89],
            },
            tolerance=2,
        )

        self.assertEqual([item["bar_index"] for item in clusters], [10, 30, 60, 89])
        self.assertEqual(clusters[0]["vote_count"], 2)
        self.assertEqual(clusters[1]["vote_count"], 3)

    def test_transition_proximity_peaks_around_breakpoints(self) -> None:
        index = pd.date_range("2025-01-01", periods=8, freq="h", tz="UTC")
        proximity = changepoint.build_transition_proximity(index, [3], window=2)

        self.assertEqual(proximity.iloc[3], 1.0)
        self.assertEqual(proximity.iloc[1], 0.0)
        self.assertGreater(proximity.iloc[2], 0.0)
        self.assertGreater(proximity.iloc[4], 0.0)

    def test_load_candles_accepts_timestamp_column(self) -> None:
        with TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "candles.csv"
            path.write_text(
                "timestamp,open,high,low,close,volume\n"
                "1740502800000,1,2,0.5,1.5,10\n",
                encoding="utf-8",
            )

            candles = changepoint.load_candles(path)

        self.assertEqual(len(candles), 1)
        self.assertEqual(candles.index.name, "date")
        self.assertEqual(float(candles.iloc[0]["close"]), 1.5)


class EntryDroughtHelperTests(unittest.TestCase):
    def test_gate_ablations_flag_density_bottleneck(self) -> None:
        gate_df = pd.DataFrame(
            {
                "session": [True] * 8,
                "trend": [True] * 8,
                "strict_gate": [True, True, True, True, False, False, False, False],
            },
            index=pd.date_range("2025-01-01", periods=8, freq="D", tz="UTC"),
        )

        ablations = drought.analyze_gate_ablations(gate_df)
        suspect_gates = [item["gate"] for item in drought.find_suspect_gates(ablations)]

        self.assertEqual(ablations[0]["gate"], "strict_gate")
        self.assertIn("strict_gate", suspect_gates)
        self.assertEqual(drought.classify_density_issue(gate_df, ablations), "over_gating_issue")


class StructuralFeedbackEnricherTests(unittest.TestCase):
    def test_attach_structural_feedback_maps_trade_to_template(self) -> None:
        trade = {
            "trade_id": "t-1",
            "symbol": "NQ",
            "realized_outcome": "win",
            "pnl": 0.02,
            "close_ts_ms": 1_745_427_900_000,
        }
        template = {
            "template_feedback": {
                "structural_feedback": {
                    "protocol_version": "structural-feedback-v1",
                    "recommendation_id": "structural-feedback:NQ:node:path",
                    "recommended_at": "2026-05-07T09:56:50Z",
                    "node_id": "node-1",
                    "branch_id": "branch-1",
                    "scenario_id": "scenario-1",
                    "path_id": "path-1",
                    "followed_path": True,
                },
                "model_probabilities_before_trade": {
                    "selected_direction": "Bull",
                    "selected_probability": 0.62,
                    "long_score": 0.62,
                    "short_score": 0.38,
                    "win_prob_long": 0.62,
                    "win_prob_short": 0.38,
                    "uncertainty": 0.10,
                },
            }
        }

        enriched = enricher.attach_structural_feedback(trade, template)

        self.assertEqual(enriched["structural_feedback"]["path_id"], "path-1")
        self.assertEqual(
            enriched["model_probabilities_before_trade"]["selected_probability"],
            0.62,
        )
        self.assertEqual(enriched["realized_outcome"], "win")

    def test_enrich_jsonl_round_trip_writes_only_matched_records(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            trades_path = tmp / "trades.jsonl"
            pending_path = tmp / "pending_update_history.json"
            output_path = tmp / "enriched.jsonl"

            trades_path.write_text(
                "\n".join(
                    [
                        '{"trade_id":"t-1","symbol":"NQ","realized_outcome":"win","pnl":0.02,"close_ts_ms":1745427900000}',
                        '{"trade_id":"t-2","symbol":"NQ","realized_outcome":"loss","pnl":-0.01,"close_ts_ms":1745427901000}',
                    ]
                )
                + "\n",
                encoding="utf-8",
            )
            pending_path.write_text(
                '[{"template_feedback":{"structural_feedback":{"protocol_version":"structural-feedback-v1","recommendation_id":"rec-1","recommended_at":"2026-05-07T09:56:50Z","node_id":"node-1","branch_id":"branch-1","scenario_id":"scenario-1","path_id":"path-1","followed_path":true},"model_probabilities_before_trade":{"selected_direction":"Bull","selected_probability":0.62,"long_score":0.62,"short_score":0.38,"win_prob_long":0.62,"win_prob_short":0.38,"uncertainty":0.1}}}]',
                encoding="utf-8",
            )

            summary = enricher.enrich_real_trades_jsonl(
                trades_path=trades_path,
                pending_update_history_path=pending_path,
                output_path=output_path,
            )

            self.assertEqual(summary["matched"], 1)
            self.assertEqual(summary["unmatched"], 1)
            lines = [line for line in output_path.read_text(encoding="utf-8").splitlines() if line.strip()]
            self.assertEqual(len(lines), 1)
            payload = pd.Series([lines[0]]).apply(lambda x: __import__("json").loads(x)).iloc[0]
            self.assertEqual(payload["structural_feedback"]["path_id"], "path-1")

    def test_emit_structural_feedback_probe_uses_target_lineage(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            target_csv = tmp / "target.csv"
            output_path = tmp / "feedback.json"
            target_csv.write_text(
                "symbol,candidate_set_id,candidate_set_size,rank,path_id,scenario_id,path_label,direction,generated_at,behavior_policy_probability,current_posterior,raw_path_score\n"
                "NQ,set-1,3,1,path-1,scenario-1,trend_follow,Observe,2026-05-09T00:00:00Z,0.37,0.46,0.47\n",
                encoding="utf-8",
            )

            summary = enricher.emit_structural_feedback_probe(
                target_csv=target_csv,
                output_path=output_path,
                realized_outcome="win",
                pnl=0.03,
            )

            payload = __import__("json").loads(output_path.read_text(encoding="utf-8"))
            self.assertTrue(summary["ok"])
            self.assertEqual(payload["protocol_version"], "structural-feedback-v1")
            self.assertEqual(payload["path_id"], "path-1")
            self.assertEqual(payload["scenario_id"], "scenario-1")
            self.assertEqual(payload["candidate_set_id"], "set-1")
            self.assertEqual(payload["realized_outcome"], "win")
            self.assertEqual(payload["model_probabilities_before_trade"]["selected_probability"], 0.37)


class StructuralFeedbackReplayHarnessTests(unittest.TestCase):
    def test_outcome_from_forward_window_labels_directional_move(self) -> None:
        candles = [
            {"close": 100.0, "high": 100.0, "low": 100.0},
            {"close": 100.1, "high": 100.4, "low": 99.9},
            {"close": 100.4, "high": 100.5, "low": 100.0},
        ]

        outcome, pnl, exit_close = replay.outcome_from_forward_window(
            candles,
            entry_index=0,
            horizon=2,
            threshold=0.001,
        )

        self.assertEqual(outcome, "win")
        self.assertAlmostEqual(pnl, 0.004)
        self.assertEqual(exit_close, 100.4)

    def test_load_candles_accepts_wrapped_payload(self) -> None:
        with TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "candles.json"
            path.write_text(
                '{"symbol":"NQ","candles":[{"timestamp":"t","open":1,"high":1,"low":1,"close":1,"volume":1}]}',
                encoding="utf-8",
            )

            candles = replay.load_candles(path)

        self.assertEqual(len(candles), 1)
        self.assertEqual(candles[0]["timestamp"], "t")


if __name__ == "__main__":
    unittest.main()
