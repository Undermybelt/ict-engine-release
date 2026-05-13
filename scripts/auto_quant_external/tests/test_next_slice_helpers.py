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

    def test_emit_structural_feedback_probe_prefers_explicit_branch_path_fields(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            target_csv = tmp / "target.csv"
            output_path = tmp / "feedback.json"
            branch_path = "Bull -> ProviderTrend -> EmaRsiContinuation -> ProviderBtcEmaRsiHold12"
            target_csv.write_text(
                "symbol,candidate_set_id,candidate_set_size,rank,path_id,regime_profit_branch_path,main_regime,sub_regime,sub_sub_regime_or_profit_factor,profit_factor,direction,generated_at,behavior_policy_probability,current_posterior,raw_path_score\n"
                f"NQ,set-1,3,1,path:scenario:generic,{branch_path},Bull,ProviderTrend,EmaRsiContinuation,ProviderBtcEmaRsiHold12,Bull,2026-05-09T00:00:00Z,0.37,0.46,0.47\n",
                encoding="utf-8",
            )

            enricher.emit_structural_feedback_probe(
                target_csv=target_csv,
                output_path=output_path,
                realized_outcome="win",
                pnl=0.03,
            )

            payload = __import__("json").loads(output_path.read_text(encoding="utf-8"))
            self.assertEqual(payload["path_id"], branch_path)
            self.assertEqual(payload["regime_profit_branch_path"], branch_path)
            self.assertEqual(payload["main_regime"], "Bull")
            self.assertEqual(payload["sub_regime"], "ProviderTrend")
            self.assertEqual(payload["sub_sub_regime_or_profit_factor"], "EmaRsiContinuation")
            self.assertEqual(payload["profit_factor"], "ProviderBtcEmaRsiHold12")
            self.assertEqual(payload["branch_id"], "Bull -> ProviderTrend")
            self.assertEqual(payload["scenario_id"], "Bull -> ProviderTrend -> EmaRsiContinuation")

    def test_layer_contract_enrichment_overrides_stale_source_and_keeps_branch(self) -> None:
        trade = {
            "schema_version": "1.0",
            "symbol": "B2R_YAHOO_BTC_PULLBACK_PRECISION_104902",
            "trade_id": "trade-1",
            "strategy_name": "ProviderCryptoMomentumStateV1",
            "auto_quant_run_id": "stale-run",
            "open_ts_ms": 1,
            "close_ts_ms": 2,
            "direction": "Bull",
            "pnl": 0.01,
            "realized_outcome": "win",
            "regime_profit_branch_path": "Bull -> ProviderCryptoMomentum -> RsiMidlineExpansion -> ProviderCryptoMomentumStateV1",
        }

        enriched = enricher.enrich_trade_with_layer_contract(
            trade,
            auto_quant_run_id="20260512T115700+0800-codex-same-root-six-provider-1h-aq-v1",
            symbol="BTC_USDT",
            provider_provenance={
                "provider": "yfinance",
                "provider_symbol": "BTC-USD",
                "timeframe": "1h",
                "source_csv": "provider-csv/yfinance_btc_usd_1h.csv",
            },
            pre_bayes_filter_state={"gate": "pass_neutralized", "canonical_regime": "range"},
            bbn_posterior={"canonical_regime": "range", "confidence": 0.52},
            catboost_path_ranker_label={"score_model_family": "catboost", "label": "observed_win"},
            execution_tree_decision={"ready": False, "actionable": False, "review": "observe"},
            failure_reason="execution_tree_observe_only",
            quality_weight=0.25,
        )

        self.assertEqual(
            enriched["auto_quant_run_id"],
            "20260512T115700+0800-codex-same-root-six-provider-1h-aq-v1",
        )
        self.assertEqual(enriched["symbol"], "BTC_USDT")
        self.assertEqual(enriched["provider_provenance"]["provider"], "yfinance")
        self.assertEqual(enriched["pre_bayes_filter_state"]["gate"], "pass_neutralized")
        self.assertEqual(enriched["bbn_posterior"]["canonical_regime"], "range")
        self.assertEqual(enriched["catboost_path_ranker_label"]["label"], "observed_win")
        self.assertEqual(enriched["execution_tree_decision"]["review"], "observe")
        self.assertEqual(enriched["failure_reason"], "execution_tree_observe_only")
        self.assertEqual(enriched["quality_weight"], 0.25)
        self.assertEqual(enriched["main_regime"], "Bull")
        self.assertEqual(enriched["sub_regime"], "ProviderCryptoMomentum")
        self.assertEqual(enriched["sub_sub_regime_or_profit_factor"], "RsiMidlineExpansion")
        self.assertEqual(enriched["profit_factor"], "ProviderCryptoMomentumStateV1")
        self.assertNotIn("104902", enriched["symbol"])


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
