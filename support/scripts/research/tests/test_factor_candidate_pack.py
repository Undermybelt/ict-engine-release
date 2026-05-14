from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
import zipfile

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import factor_candidate_pack as pack  # noqa: E402


class FactorCandidatePackTests(unittest.TestCase):
    def test_build_manifest_from_freqtrade_backtest_zip_extracts_strategy_metrics(self) -> None:
        backtest_payload = {
            "strategy": {
                "TomacNQ_RegimeTrendPullbackDense15m": {
                    "strategy_name": "TomacNQ_RegimeTrendPullbackDense15m",
                    "results_per_pair": [
                        {
                            "key": "NQ/USD",
                            "trades": 103,
                            "winrate": 0.31,
                            "sharpe": 0.1211,
                            "profit_factor": 1.21,
                            "profit_total_pct": 3.92,
                            "max_drawdown_account": 0.0321,
                        },
                        {
                            "key": "TOTAL",
                            "trades": 103,
                            "winrate": 0.31,
                            "sharpe": 0.1211,
                            "profit_factor": 1.21,
                            "profit_total_pct": 3.92,
                            "max_drawdown_account": 0.0321,
                        },
                    ],
                    "total_trades": 103,
                    "wins": 32,
                    "losses": 71,
                    "draws": 0,
                    "sharpe": 0.1211,
                    "profit_factor": 1.21,
                    "profit_total": 0.0392,
                    "max_drawdown_account": 0.0321,
                    "backtest_start": "2023-01-01 00:00:00",
                    "backtest_end": "2025-12-31 00:00:00",
                    "timeframe": "15m",
                }
            }
        }
        config_payload = {"timeframe": "15m", "exchange": {"pair_whitelist": ["NQ/USD"]}}
        strategy_source = '''"""
Paradigm: regime-cluster trend pullback
Hypothesis: 15m pullback density unlock with 1h/4h resonance
Parent: TomacNQ_RegimeTrendPullbackDense
Status: density-via-timeframe probe
Uses MTF: yes
"""
'''

        with TemporaryDirectory() as tmpdir:
            zip_path = Path(tmpdir) / "backtest.zip"
            with zipfile.ZipFile(zip_path, "w") as archive:
                archive.writestr("backtest-result.json", json.dumps(backtest_payload))
                archive.writestr(
                    "backtest-result_config.json", json.dumps(config_payload)
                )
                archive.writestr(
                    "backtest-result_TomacNQ_RegimeTrendPullbackDense15m.py",
                    strategy_source,
                )

            manifest = pack.build_manifest_from_freqtrade_backtest_zip(zip_path)

        self.assertEqual(manifest["timeframe"], "15m")
        self.assertEqual(len(manifest["strategies"]), 1)
        strategy = manifest["strategies"][0]
        self.assertEqual(strategy["name"], "TomacNQ_RegimeTrendPullbackDense15m")
        self.assertEqual(strategy["metadata"]["paradigm"], "regime-cluster trend pullback")
        self.assertEqual(
            strategy["metadata"]["parent_strategy"],
            "TomacNQ_RegimeTrendPullbackDense",
        )
        self.assertTrue(strategy["metadata"]["uses_mtf"])
        self.assertEqual(strategy["validation_metrics"]["trade_count"], 103)
        self.assertEqual(strategy["validation_metrics"]["win_rate_pct"], 31.067961)
        self.assertEqual(
            strategy["per_pair_metrics"]["NQ/USD"]["max_drawdown_pct"],
            3.21,
        )

    def test_build_candidate_pack_uses_candidate_spec_and_cross_market_metrics(self) -> None:
        manifest = {
            "manifest_version": "1.0",
            "timeframe": "15m",
            "strategies": [
                {
                    "name": "TrendPullbackDense15m",
                    "status": "ok",
                    "metadata": {
                        "strategy": "TrendPullbackDense15m",
                        "mutation_id": "slice-083",
                        "base_factor": "trend_pullback",
                        "hypothesis": "pullback after higher timeframe trend continuation",
                        "paradigm": "trend",
                        "expected_regime": "expansion",
                        "factors_used": ["ema_fast", "ema_slow", "pullback_zone"],
                        "asset_class": "index_futures",
                    },
                    "validation_metrics": {
                        "sharpe": 1.42,
                        "trade_count": 87,
                        "win_rate_pct": 54.5,
                        "profit_factor": 1.85,
                        "total_profit_pct": 12.3,
                        "max_drawdown_pct": -3.2,
                    },
                    "per_pair_metrics": {
                        "NQ/USD": {"sharpe": 1.42, "trade_count": 87, "win_rate_pct": 54.5},
                        "SPY/USD": {"sharpe": 1.10, "trade_count": 50, "win_rate_pct": 56.0},
                        "GLD/USD": {"sharpe": 0.72, "trade_count": 34, "win_rate_pct": 53.0},
                    },
                }
            ],
        }
        candidate_spec = {
            "expression_text": "ema_fast > ema_slow and pullback_zone <= 0.4",
            "operator_set": ["ema", "pullback_zone", "trend_gate"],
            "complexity": 3,
            "target_market_hypothesis": ["NQ", "SPY", "GLD"],
            "base_timeframe": "15m",
            "context_timeframes": ["1h", "4h"],
            "pre_bayes_targets": ["filtered_resonance_label", "factor_uncertainty"],
            "belief_targets": ["entry_quality", "multi_timeframe_resonance"],
            "path_ranking_targets": ["experience_prior", "current_posterior"],
            "execution_tree_targets": ["execution_readiness", "prediction_vote_score"],
            "structural_feedback_required": True,
            "resonance_summary": {
                "base_timeframe": "15m",
                "context_stack": ["1h", "4h"],
                "resonance_by_timeframe": {"1h": "aligned", "4h": "aligned"},
            },
            "regime_role": "mixed",
            "cross_market_metrics": {
                "GLD/USD": {
                    "sharpe": 0.641,
                    "trade_count": 140,
                    "win_rate_pct": 54.0,
                    "profit_factor": 1.44,
                    "total_profit_pct": 6.7,
                    "max_drawdown_pct": 5.2,
                    "window": "2025-05-07->2026-05-06",
                },
                "SPY/USD": {
                    "sharpe": 0.605,
                    "trade_count": None,
                    "win_rate_pct": None,
                    "profit_factor": None,
                    "total_profit_pct": None,
                    "max_drawdown_pct": None,
                    "window": "2025-05-07->2026-05-06",
                },
            },
        }
        autoresearch_status = {
            "effective_status": "completed",
            "best_attempt": {
                "attempt_id": "attempt-3",
                "decision": {"score_delta": 0.19, "status": "keep"},
            },
            "decision_counts": {"keep": 2, "discard": 1},
            "failure_tag_counts": {"thin_trade_count": 1},
        }

        bundle = pack.build_factor_candidate_pack(
            manifest=manifest,
            strategy_name="TrendPullbackDense15m",
            candidate_spec=candidate_spec,
            autoresearch_status=autoresearch_status,
        )

        self.assertEqual(
            bundle["factor_expression"]["strategy_name"],
            "TrendPullbackDense15m",
        )
        self.assertEqual(
            bundle["factor_expression"]["expression_text"],
            "ema_fast > ema_slow and pullback_zone <= 0.4",
        )
        self.assertEqual(
            bundle["factor_expression"]["filter_belief_execution_mapping"]["pre_bayes_targets"],
            ["filtered_resonance_label", "factor_uncertainty"],
        )
        self.assertEqual(
            bundle["factor_expression"]["filter_belief_execution_mapping"]["execution_tree_targets"],
            ["execution_readiness", "prediction_vote_score"],
        )
        self.assertTrue(
            bundle["factor_expression"]["filter_belief_execution_mapping"]["structural_feedback_required"]
        )
        self.assertEqual(
            bundle["factor_eval_grid_summary"]["trade_density_summary"]["aggregate_label"],
            "preferred_density",
        )
        self.assertEqual(
            bundle["factor_eval_grid_summary"]["breadth_matrix"]["SPY/USD"]["status"],
            "covered",
        )
        self.assertEqual(
            bundle["factor_eval_grid_summary"]["breadth_matrix"]["GLD/USD"]["status"],
            "covered",
        )
        self.assertEqual(
            bundle["factor_eval_grid_summary"]["breadth_matrix"]["SPY/USD"]["status"],
            "covered",
        )
        self.assertEqual(
            bundle["transfer_score"]["status"],
            "cross_market_candidate",
        )
        self.assertGreater(bundle["transfer_score"]["overall_transfer_score"], 0.5)
        self.assertIn("GLD/USD", bundle["transfer_score"]["covered_markets"])
        self.assertEqual(bundle["transfer_score"]["markets_without_trade_counts"], [])

    def test_build_candidate_pack_falls_back_to_manifest_hypothesis(self) -> None:
        manifest = {
            "manifest_version": "1.0",
            "timeframe": "1h",
            "strategies": [
                {
                    "name": "VRPCarry",
                    "status": "ok",
                    "metadata": {
                        "strategy": "VRPCarry",
                        "mutation_id": "slice-140",
                        "base_factor": "vrp_carry",
                        "hypothesis": "carry-style compression regime harvest",
                        "paradigm": "carry",
                        "expected_regime": "compression",
                        "factors_used": ["rv_zscore", "value_zone"],
                        "asset_class": "index_futures",
                    },
                    "validation_metrics": {
                        "sharpe": 0.83,
                        "trade_count": 12,
                        "win_rate_pct": 58.0,
                        "profit_factor": 1.21,
                    },
                    "per_pair_metrics": {
                        "NQ/USD": {"sharpe": 0.83, "trade_count": 12, "win_rate_pct": 58.0}
                    },
                }
            ],
        }

        bundle = pack.build_factor_candidate_pack(manifest=manifest)

        self.assertEqual(
            bundle["factor_expression"]["expression_text"],
            "carry-style compression regime harvest",
        )
        self.assertEqual(
            bundle["factor_expression"]["operator_set"],
            ["rv_zscore", "value_zone"],
        )
        self.assertEqual(
            bundle["factor_expression"]["filter_belief_execution_mapping"]["pre_bayes_targets"],
            [],
        )
        self.assertFalse(
            bundle["factor_expression"]["filter_belief_execution_mapping"]["structural_feedback_required"]
        )
        self.assertEqual(
            bundle["factor_eval_grid_summary"]["trade_density_summary"]["aggregate_label"],
            "probe_only",
        )
        self.assertEqual(bundle["transfer_score"]["status"], "single_market_only")

    def test_build_strategy_library_manifest_from_freqtrade_backtest_zip(self) -> None:
        backtest_payload = {
            "strategy": {
                "TomacNQ_RegimeFVGRetrace": {
                    "strategy_name": "TomacNQ_RegimeFVGRetrace",
                    "results_per_pair": [
                        {
                            "key": "NQ/USD",
                            "trades": 12,
                            "winrate": 0.58333333,
                            "sharpe": 0.014993373176821853,
                            "profit_factor": 1.92,
                            "profit_total_pct": 0.57,
                            "max_drawdown_account": 0.00548,
                        },
                        {
                            "key": "TOTAL",
                            "trades": 12,
                            "winrate": 0.58333333,
                            "sharpe": 0.014993373176821853,
                            "profit_factor": 1.92,
                            "profit_total_pct": 0.57,
                            "max_drawdown_account": 0.00548,
                        },
                    ],
                    "total_trades": 12,
                    "wins": 7,
                    "losses": 5,
                    "draws": 0,
                    "sharpe": 0.014993373176821853,
                    "profit_factor": 1.92,
                    "profit_total": 0.0057,
                    "max_drawdown_account": 0.00548,
                    "backtest_start": "2018-01-01 00:00:00",
                    "backtest_end": "2025-12-31 00:00:00",
                    "timeframe": "1h",
                }
            }
        }
        config_payload = {"timeframe": "1h", "exchange": {"pair_whitelist": ["NQ/USD"]}}
        strategy_source = '''"""
Paradigm: structural retrace imbalance retest
Hypothesis: bullish fair-value-gap exists, later retraces into the gap, rejects back above the lower bound, and fires only when 4h trend remains aligned
Parent: TomacNQ_KillzoneBreakout
Status: active
External Data: no
Uses MTF: yes
"""
'''

        with TemporaryDirectory() as tmpdir:
            zip_path = Path(tmpdir) / "backtest.zip"
            with zipfile.ZipFile(zip_path, "w") as archive:
                archive.writestr("backtest-result.json", json.dumps(backtest_payload))
                archive.writestr(
                    "backtest-result_config.json", json.dumps(config_payload)
                )
                archive.writestr(
                    "backtest-result_TomacNQ_RegimeFVGRetrace.py",
                    strategy_source,
                )

            manifest = pack.build_strategy_library_manifest_from_freqtrade_backtest_zip(
                zip_path,
                repo_url="local-auto-quant",
                pinned_ref="abc123",
                config_path="config.tomac.json",
                log_path="run_tomac_fvg.log",
            )

        self.assertEqual(manifest["manifest_version"], "1.0")
        self.assertEqual(manifest["timeframe"], "1h")
        self.assertEqual(manifest["auto_quant_repo_url"], "local-auto-quant")
        self.assertEqual(manifest["auto_quant_pinned_ref"], "abc123")
        self.assertEqual(manifest["config_path"], "config.tomac.json")
        self.assertEqual(manifest["log_path"], "run_tomac_fvg.log")
        self.assertEqual(manifest["validation_errors"], [])
        self.assertEqual(len(manifest["strategies"]), 1)
        strategy = manifest["strategies"][0]
        self.assertEqual(strategy["name"], "TomacNQ_RegimeFVGRetrace")
        self.assertEqual(strategy["status"], "ok")
        self.assertEqual(strategy["metadata"]["parent"], "TomacNQ_KillzoneBreakout")
        self.assertEqual(strategy["validation_metrics"]["trade_count"], 12)
        self.assertEqual(strategy["per_pair_metrics"]["NQ/USD"]["trade_count"], 12)
        self.assertEqual(strategy["pairs"], ["NQ/USD"])

    def test_main_writes_artifacts(self) -> None:
        manifest = {
            "manifest_version": "1.0",
            "timeframe": "5m",
            "strategies": [
                {
                    "name": "SweepReclaimWide",
                    "status": "ok",
                    "metadata": {
                        "strategy": "SweepReclaimWide",
                        "mutation_id": "slice-086",
                        "base_factor": "sweep_reclaim",
                        "hypothesis": "wide liquidity sweep reclaim",
                        "paradigm": "reversal",
                        "expected_regime": "liquidity_sweep",
                        "factors_used": ["sweep_window", "reclaim_gate"],
                        "asset_class": "index_futures",
                    },
                    "validation_metrics": {
                        "sharpe": 1.12,
                        "trade_count": 31,
                        "win_rate_pct": 51.0,
                        "profit_factor": 1.33,
                    },
                    "per_pair_metrics": {
                        "NQ/USD": {"sharpe": 1.12, "trade_count": 31, "win_rate_pct": 51.0},
                        "ES/USD": {"sharpe": 0.65, "trade_count": 16, "win_rate_pct": 49.0},
                    },
                }
            ],
        }
        candidate_spec = {
            "base_timeframe": "5m",
            "context_timeframes": ["15m", "1h", "4h"],
            "regime_role": "execution_only",
        }

        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            manifest_path = root / "strategy_library.json"
            spec_path = root / "candidate_spec.json"
            output_dir = root / "out"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            spec_path.write_text(json.dumps(candidate_spec), encoding="utf-8")

            exit_code = pack.main(
                [
                    "--manifest-json",
                    str(manifest_path),
                    "--strategy-name",
                    "SweepReclaimWide",
                    "--candidate-spec-json",
                    str(spec_path),
                    "--output-dir",
                    str(output_dir),
                ]
            )

            self.assertEqual(exit_code, 0)
            expression = json.loads(
                (output_dir / "factor_expression.json").read_text(encoding="utf-8")
            )
            grid = json.loads(
                (output_dir / "factor_eval_grid_summary.json").read_text(
                    encoding="utf-8"
                )
            )
            transfer = json.loads(
                (output_dir / "transfer_score.json").read_text(encoding="utf-8")
            )

            self.assertEqual(expression["strategy_name"], "SweepReclaimWide")
            self.assertEqual(grid["selected_strategy"], "SweepReclaimWide")
            self.assertEqual(transfer["covered_market_count"], 2)

    def test_main_accepts_freqtrade_backtest_zip(self) -> None:
        backtest_payload = {
            "strategy": {
                "TomacNQ_RegimeVRPCompression15m": {
                    "strategy_name": "TomacNQ_RegimeVRPCompression15m",
                    "results_per_pair": [
                        {
                            "key": "NQ/USD",
                            "trades": 334,
                            "winrate": 0.34,
                            "sharpe": 0.339,
                            "profit_factor": 1.64,
                            "profit_total_pct": 28.95,
                            "max_drawdown_account": 0.041,
                        },
                        {
                            "key": "TOTAL",
                            "trades": 334,
                            "winrate": 0.34,
                            "sharpe": 0.339,
                            "profit_factor": 1.64,
                            "profit_total_pct": 28.95,
                            "max_drawdown_account": 0.041,
                        },
                    ],
                    "total_trades": 334,
                    "wins": 114,
                    "losses": 220,
                    "draws": 0,
                    "sharpe": 0.339,
                    "profit_factor": 1.64,
                    "profit_total": 0.2895,
                    "max_drawdown_account": 0.041,
                    "backtest_start": "2018-01-01 00:00:00",
                    "backtest_end": "2025-12-31 00:00:00",
                    "timeframe": "15m",
                }
            }
        }
        candidate_spec = {
            "candidate_id": "family_f_vrp_compression_v1",
            "display_name": "VRP Compression 15m",
            "family": "Family F",
            "status": "active",
            "promotion_state": "promotable",
            "expression_text": "iv_pct_rank_252 < 0.30 and hv_pct_rank_252 < 0.30",
            "operator_set": ["iv_pct_rank_252", "hv_pct_rank_252", "ema89", "ema_fast_4h"],
            "base_timeframe": "15m",
            "context_timeframes": ["4h", "1d"],
            "regime_role": "mixed",
            "pre_bayes_targets": ["volatility_compression_gate"],
            "belief_targets": ["bbn_vol_regime_evidence"],
            "path_ranking_targets": ["structural_path_confidence"],
            "execution_tree_targets": ["transition_guardrail", "observe_gate"],
            "structural_feedback_required": True,
        }

        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            zip_path = root / "backtest.zip"
            spec_path = root / "candidate_spec.json"
            output_dir = root / "out"
            with zipfile.ZipFile(zip_path, "w") as archive:
                archive.writestr("backtest-result.json", json.dumps(backtest_payload))
                archive.writestr(
                    "backtest-result_config.json", json.dumps({"timeframe": "15m"})
                )
                archive.writestr(
                    "backtest-result_TomacNQ_RegimeVRPCompression15m.py",
                    '"""\nParadigm: vol regime compression\nHypothesis: compressed IV/HV regime expansion\nUses MTF: yes\n"""',
                )
            spec_path.write_text(json.dumps(candidate_spec), encoding="utf-8")

            exit_code = pack.main(
                [
                    "--freqtrade-backtest-zip",
                    str(zip_path),
                    "--strategy-name",
                    "TomacNQ_RegimeVRPCompression15m",
                    "--candidate-spec-json",
                    str(spec_path),
                    "--output-dir",
                    str(output_dir),
                ]
            )

            self.assertEqual(exit_code, 0)
            expression = json.loads(
                (output_dir / "factor_expression.json").read_text(encoding="utf-8")
            )
            grid = json.loads(
                (output_dir / "factor_eval_grid_summary.json").read_text(
                    encoding="utf-8"
                )
            )

            self.assertEqual(expression["candidate_id"], "family_f_vrp_compression_v1")
            self.assertEqual(grid["trade_density_summary"]["aggregate_trade_count"], 334)
            self.assertEqual(
                grid["aggregate_metrics"]["max_drawdown_pct"],
                4.1,
            )

    def test_main_can_emit_strategy_library_manifest_from_freqtrade_backtest_zip(self) -> None:
        backtest_payload = {
            "strategy": {
                "TomacNQ_RegimeFVGRetrace": {
                    "strategy_name": "TomacNQ_RegimeFVGRetrace",
                    "results_per_pair": [
                        {
                            "key": "NQ/USD",
                            "trades": 12,
                            "winrate": 0.58333333,
                            "sharpe": 0.014993373176821853,
                            "profit_factor": 1.92,
                            "profit_total_pct": 0.57,
                            "max_drawdown_account": 0.00548,
                        },
                        {
                            "key": "TOTAL",
                            "trades": 12,
                            "winrate": 0.58333333,
                            "sharpe": 0.014993373176821853,
                            "profit_factor": 1.92,
                            "profit_total_pct": 0.57,
                            "max_drawdown_account": 0.00548,
                        },
                    ],
                    "total_trades": 12,
                    "wins": 7,
                    "losses": 5,
                    "draws": 0,
                    "sharpe": 0.014993373176821853,
                    "profit_factor": 1.92,
                    "profit_total": 0.0057,
                    "max_drawdown_account": 0.00548,
                    "backtest_start": "2018-01-01 00:00:00",
                    "backtest_end": "2025-12-31 00:00:00",
                    "timeframe": "1h",
                }
            }
        }

        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            zip_path = root / "backtest.zip"
            output_manifest = root / "strategy_library.json"
            with zipfile.ZipFile(zip_path, "w") as archive:
                archive.writestr("backtest-result.json", json.dumps(backtest_payload))
                archive.writestr(
                    "backtest-result_config.json", json.dumps({"timeframe": "1h"})
                )
                archive.writestr(
                    "backtest-result_TomacNQ_RegimeFVGRetrace.py",
                    '"""\nParadigm: structural retrace imbalance retest\nHypothesis: bullish fair-value-gap retest\nParent: TomacNQ_KillzoneBreakout\nStatus: active\nUses MTF: yes\n"""',
                )

            exit_code = pack.main(
                [
                    "--freqtrade-backtest-zip",
                    str(zip_path),
                    "--emit-strategy-library-json",
                    str(output_manifest),
                    "--repo-url",
                    "local-auto-quant",
                    "--pinned-ref",
                    "abc123",
                    "--config-path",
                    "config.tomac.json",
                    "--log-path",
                    "run_tomac_fvg.log",
                    "--output-dir",
                    str(root / "candidate-pack"),
                ]
            )

            self.assertEqual(exit_code, 0)
            manifest = json.loads(output_manifest.read_text(encoding="utf-8"))
            self.assertEqual(manifest["manifest_version"], "1.0")
            self.assertEqual(manifest["auto_quant_repo_url"], "local-auto-quant")
            self.assertEqual(manifest["strategies"][0]["name"], "TomacNQ_RegimeFVGRetrace")


if __name__ == "__main__":
    unittest.main()
