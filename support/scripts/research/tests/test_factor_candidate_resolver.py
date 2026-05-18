from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPT_ROOT.parents[2]
sys.path.insert(0, str(SCRIPT_ROOT))

import factor_candidate_resolver as resolver  # noqa: E402


class FactorCandidateResolverTests(unittest.TestCase):
    def test_build_candidate_registry_marks_invalid_freqtrade_zip_unbuildable(self) -> None:
        with TemporaryDirectory() as tmpdir:
            repo_root = Path(tmpdir)
            (repo_root / "config").mkdir(parents=True, exist_ok=True)
            broken_zip = repo_root / "broken.zip"
            broken_zip.write_text("not-a-real-zip", encoding="utf-8")
            (repo_root / "config" / "factor_candidate_harness_presets.json").write_text(
                json.dumps(
                    {
                        "schema_version": "factor-candidate-harness-presets/v1",
                        "candidates": [
                            {
                                "candidate_id": "broken_freqtrade_zip_candidate",
                                "display_name": "Broken Zip Candidate",
                                "artifact_source": {
                                    "freqtrade_backtest_zip": str(broken_zip)
                                },
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            bundle = resolver.build_candidate_registry(repo_root=repo_root)

            candidate = bundle["candidates"][0]
            self.assertFalse(candidate["artifact_ready"])
            self.assertEqual(candidate["evidence_status"], "missing_reusable_artifact")
            self.assertEqual(candidate["curation_decision"], "discard_until_reusable_artifact")
            self.assertEqual(candidate["artifact_kind"], "freqtrade_backtest_zip")
            self.assertTrue(
                candidate["pack_build_reason"].startswith("invalid_artifact:")
            )

    def test_build_candidate_registry_without_profile_stays_generic(self) -> None:
        bundle = resolver.build_candidate_registry(repo_root=REPO_ROOT)

        self.assertEqual(bundle["summary"]["selection_mode"], "generic_zero_config")
        self.assertEqual(bundle["summary"]["candidate_count"], 13)
        self.assertEqual(bundle["summary"]["buildable_count"], 7)
        self.assertEqual(
            bundle["summary"]["naming_contract_version"],
            "factor-artifact-naming/v1",
        )
        self.assertIsNone(bundle["selected_profile"])
        buildable = [candidate for candidate in bundle["candidates"] if candidate["artifact_ready"]]
        self.assertEqual(len(buildable), 7)
        self.assertTrue(
            all(candidate["artifact_kind"] == "candidate_pack_dir" for candidate in buildable)
        )
        deferred = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "regime_primary_gate_pending_v1"
        )
        self.assertEqual(
            deferred["pack_readiness_reason"],
            "opt_in_regime_benchmark_profile_required",
        )
        self.assertEqual(deferred["evidence_status"], "deferred")
        self.assertEqual(deferred["curation_decision"], "needs_named_prerequisite")
        self.assertEqual(deferred["artifact_kind"], "regime_benchmark_json")
        self.assertEqual(deferred["archive_evidence_status"], "not_runtime_input")
        self.assertEqual(deferred["archive_refs"], [])
        for cid in [
            "family_a_killzone_breakout_15m_v1",
            "family_a_killzone_breakout_1d_regime_v1",
            "family_a_killzone_breakout_1m_v1",
            "family_a_es_killzone_breakout_1h_v1",
            "family_a_eur_killzone_breakout_1h_v1",
        ]:
            candidate = next(item for item in bundle["candidates"] if item["candidate_id"] == cid)
            self.assertNotIn("/tmp/", candidate["strategy_source"])
            self.assertTrue(all("/tmp/" not in ref for ref in candidate["reusable_input_refs"]))

    def test_build_candidate_registry_with_profile_marks_reusable_artifacts(self) -> None:
        bundle = resolver.build_candidate_registry(
            repo_root=REPO_ROOT,
            profile_selector="thrill3r_nq_auto_quant_v1",
        )

        self.assertEqual(bundle["summary"]["selection_mode"], "profile_opt_in")
        self.assertEqual(bundle["selected_profile"]["profile_id"], "thrill3r_nq_auto_quant_v1")
        self.assertGreaterEqual(bundle["summary"]["buildable_count"], 5)
        vrp = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_f_vrp_compression_15m_v1"
        )
        self.assertTrue(vrp["artifact_ready"])
        self.assertIn("GLD/USD", vrp["cross_market_metrics"])
        killzone = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_killzone_breakout_1h_v1"
        )
        self.assertTrue(killzone["artifact_ready"])
        self.assertEqual(killzone["family"], "Family A")
        self.assertEqual(killzone["evidence_status"], "buildable")
        self.assertEqual(killzone["artifact_kind"], "candidate_pack_dir")
        self.assertEqual(killzone["curation_decision"], "promote_to_candidate_pack")
        self.assertEqual(killzone["archive_evidence_status"], "not_runtime_input")
        self.assertIn("candidate_pack", killzone["naming_contract"]["artifact_layers"])
        self.assertGreaterEqual(len(killzone["reusable_input_refs"]), 1)
        self.assertTrue(any("support/examples/factor_candidate_packs" in ref for ref in killzone["reusable_input_refs"]))
        displacement = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_killzone_displacement_pending_v1"
        )
        self.assertTrue(displacement["artifact_ready"])
        self.assertEqual(displacement["evidence_status"], "buildable")
        self.assertEqual(displacement["artifact_kind"], "candidate_pack_dir")
        fvg_retrace = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_fvg_retrace_1h_v1"
        )
        self.assertTrue(fvg_retrace["artifact_ready"])
        self.assertEqual(fvg_retrace["family"], "Family A")
        self.assertIn("GLD/USD", fvg_retrace["cross_market_metrics"])
        fvg_retrace_5m = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_fvg_retrace_5m_v1"
        )
        self.assertTrue(fvg_retrace_5m["artifact_ready"])
        self.assertEqual(fvg_retrace_5m["base_timeframe"], "5m")
        killzone_15m = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_killzone_breakout_15m_v1"
        )
        self.assertFalse(killzone_15m["artifact_ready"])
        self.assertEqual(killzone_15m["artifact_kind"], "strategy_library_json")
        self.assertEqual(killzone_15m["curation_decision"], "discard_until_reusable_artifact")
        one_day_regime = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_killzone_breakout_1d_regime_v1"
        )
        self.assertFalse(one_day_regime["artifact_ready"])
        self.assertEqual(one_day_regime["artifact_kind"], "strategy_library_json")
        one_minute = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_killzone_breakout_1m_v1"
        )
        self.assertFalse(one_minute["artifact_ready"])
        self.assertEqual(one_minute["base_timeframe"], "1m")
        es_breakout = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_es_killzone_breakout_1h_v1"
        )
        self.assertFalse(es_breakout["artifact_ready"])
        self.assertEqual(es_breakout["artifact_kind"], "strategy_library_json")
        eur_breakout = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "family_a_eur_killzone_breakout_1h_v1"
        )
        self.assertFalse(eur_breakout["artifact_ready"])
        self.assertEqual(eur_breakout["artifact_kind"], "strategy_library_json")
        regime = next(
            item
            for item in bundle["candidates"]
            if item["candidate_id"] == "regime_primary_gate_pending_v1"
        )
        if regime["artifact_ready"]:
            self.assertEqual(regime["artifact_kind"], "regime_benchmark_json")
            self.assertEqual(regime["evidence_status"], "buildable")
            self.assertGreaterEqual(len(regime["reusable_input_refs"]), 4)

    def test_main_writes_specs_and_builds_packs(self) -> None:
        with TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir)

            exit_code = resolver.main(
                [
                    "--repo-root",
                    str(REPO_ROOT),
                    "--profile",
                    "thrill3r_nq_auto_quant_v1",
                    "--build-packs",
                    "--output-dir",
                    str(output_dir),
                ]
            )

            self.assertEqual(exit_code, 0)
            registry = json.loads(
                (output_dir / "candidate_registry.json").read_text(encoding="utf-8")
            )
            pack_index = json.loads(
                (output_dir / "candidate_pack_index.json").read_text(encoding="utf-8")
            )

            self.assertEqual(registry["summary"]["selection_mode"], "profile_opt_in")
            self.assertEqual(
                pack_index["summary"]["built_count"],
                registry["summary"]["buildable_count"],
            )
            self.assertEqual(
                pack_index["summary"]["skipped_count"],
                registry["summary"]["candidate_count"] - registry["summary"]["buildable_count"],
            )
            self.assertEqual(
                registry["summary"]["naming_contract_version"],
                "factor-artifact-naming/v1",
            )
            self.assertTrue(
                all(not item["pack_dir"].startswith("/") for item in pack_index["built_candidates"])
            )
            skipped = {
                item["candidate_id"]: item["reason"]
                for item in pack_index["skipped_candidates"]
            }
            built = {
                item["candidate_id"]: item
                for item in pack_index["built_candidates"]
            }
            if "family_f_vrp_compression_15m_v1" in built:
                vrp_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_f_vrp_compression_15m_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(vrp_expression["family"], "Family F")
                self.assertEqual(
                    vrp_expression["strategy_name"],
                    "TomacNQ_RegimeVRPCompression15m",
                )
            if "family_a_killzone_breakout_1h_v1" in built:
                killzone_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_killzone_breakout_1h_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(killzone_expression["family"], "Family A")
            if "family_a_killzone_displacement_pending_v1" in built:
                displacement_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_killzone_displacement_pending_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    displacement_expression["strategy_name"],
                    "TomacNQ_KillzoneBreakoutDisplacement",
                )
            if "family_a_fvg_retrace_1h_v1" in built:
                fvg_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_fvg_retrace_1h_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    fvg_expression["strategy_name"],
                    "TomacNQ_RegimeFVGRetrace",
                )
                fvg_transfer = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_fvg_retrace_1h_v1"
                        / "transfer_score.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(fvg_transfer["status"], "cross_market_candidate")
            if "family_a_fvg_retrace_5m_v1" in built:
                fvg_5m_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_fvg_retrace_5m_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    fvg_5m_expression["strategy_name"],
                    "TomacNQ_RegimeFVGRetrace5m",
                )
                fvg_5m_summary = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_fvg_retrace_5m_v1"
                        / "factor_eval_grid_summary.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    fvg_5m_summary["trade_density_summary"]["aggregate_label"],
                    "preferred_density",
                )
            if "family_a_killzone_breakout_15m_v1" in built:
                killzone_15m_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_killzone_breakout_15m_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    killzone_15m_expression["strategy_name"],
                    "TomacNQKillzoneBreakout15m",
                )
            if "family_a_killzone_breakout_1d_regime_v1" in built:
                one_day_regime_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_killzone_breakout_1d_regime_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    one_day_regime_expression["strategy_name"],
                    "TomacNQ_KillzoneBreakout1dRegime",
                )
            if "family_a_killzone_breakout_1m_v1" in built:
                one_minute_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_killzone_breakout_1m_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    one_minute_expression["strategy_name"],
                    "TomacNQKillzoneBreakout1m",
                )
            if "family_a_es_killzone_breakout_1h_v1" in built:
                es_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_es_killzone_breakout_1h_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    es_expression["strategy_name"],
                    "TomacKillzoneBreakout",
                )
            if "family_a_eur_killzone_breakout_1h_v1" in built:
                eur_expression = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "family_a_eur_killzone_breakout_1h_v1"
                        / "factor_expression.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    eur_expression["strategy_name"],
                    "TomacKillzoneBreakout",
                )
            if "regime_primary_gate_pending_v1" in built:
                classifier_summary = json.loads(
                    (
                        output_dir
                        / "packs"
                        / "regime_primary_gate_pending_v1"
                        / "regime_classifier_summary.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(
                    classifier_summary["candidate_id"],
                    "regime_primary_gate_pending_v1",
                )
                self.assertGreaterEqual(classifier_summary["market_count"], 1)
            else:
                self.assertIn("regime_primary_gate_pending_v1", skipped)

    def test_list_buildable_candidates_surfaces_curated_pack_metrics(self) -> None:
        registry = resolver.build_candidate_registry(repo_root=REPO_ROOT)

        payload = resolver.list_buildable_candidates(
            repo_root=REPO_ROOT,
            candidates=registry["candidates"],
        )

        self.assertEqual(payload["summary"]["buildable_count"], 7)
        vrp = next(
            item
            for item in payload["buildable_candidates"]
            if item["candidate_id"] == "family_f_vrp_compression_15m_v1"
        )
        self.assertEqual(vrp["aggregate_trade_count"], 334)
        self.assertEqual(vrp["aggregate_label"], "preferred_density")
        self.assertEqual(vrp["transfer_status"], "cross_market_candidate")
        self.assertTrue(
            vrp["reusable_input_refs"][0].startswith("support/examples/factor_candidate_packs/")
        )

    def test_build_candidate_packs_supports_regime_benchmark_bundle(self) -> None:
        nq = {
            "symbol": "NQ",
            "base_timeframe": "1d",
            "bar_count": 4651,
            "truth_mode": "post_transition_direction",
            "ranked_results": [
                {
                    "name": "trained_family_extra_trees_v1",
                    "eval_macro_f1": 0.427327,
                    "eval_covered_precision": 0.433515,
                    "eval_coverage": 0.393266,
                    "transition_f1": 0.0,
                    "resonance_4h": 0.0,
                    "resonance_1d": 0.0,
                    "flip_rate": 0.0,
                }
            ],
        }
        spy = {
            "symbol": "SPY",
            "base_timeframe": "1d",
            "bar_count": 2513,
            "truth_mode": "post_transition_direction",
            "ranked_results": [
                {
                    "name": "trained_extra_trees_v1",
                    "eval_macro_f1": 0.449186,
                    "eval_covered_precision": 0.42623,
                    "eval_coverage": 0.404509,
                    "transition_f1": 0.0,
                    "resonance_4h": 0.0,
                    "resonance_1d": 0.0,
                    "flip_rate": 0.0,
                }
            ],
        }

        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            nq_path = root / "nq.json"
            spy_path = root / "spy.json"
            nq_path.write_text(json.dumps(nq), encoding="utf-8")
            spy_path.write_text(json.dumps(spy), encoding="utf-8")
            output_dir = root / "out"

            pack_index = resolver.build_candidate_packs(
                repo_root=root,
                output_dir=output_dir,
                candidates=[
                    {
                        "candidate_id": "regime_primary_gate_pending_v1",
                        "display_name": "Primary Regime Classifier Gate",
                        "artifact_source": {
                            "regime_benchmark_jsons": [
                                str(nq_path),
                                str(spy_path),
                            ]
                        },
                        "reusable_input_kind": "regime_benchmark_json",
                    }
                ],
            )

            self.assertEqual(pack_index["summary"]["built_count"], 1)
            self.assertEqual(pack_index["summary"]["skipped_count"], 0)
            self.assertEqual(
                pack_index["built_candidates"][0]["artifact_family"],
                "regime_artifact_bundle",
            )
            classifier_summary = json.loads(
                (
                    output_dir
                    / "packs"
                    / "regime_primary_gate_pending_v1"
                    / "regime_classifier_summary.json"
                ).read_text(encoding="utf-8")
            )
            self.assertEqual(classifier_summary["market_count"], 2)

    def test_build_candidate_packs_skips_invalid_freqtrade_zip(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            broken_zip = root / "broken.zip"
            broken_zip.write_text("not-a-real-zip", encoding="utf-8")
            output_dir = root / "out"

            pack_index = resolver.build_candidate_packs(
                repo_root=root,
                output_dir=output_dir,
                candidates=[
                    {
                        "candidate_id": "broken_freqtrade_zip_candidate",
                        "artifact_source": {
                            "freqtrade_backtest_zip": str(broken_zip)
                        },
                    }
                ],
            )

            self.assertEqual(pack_index["summary"]["built_count"], 0)
            self.assertEqual(pack_index["summary"]["skipped_count"], 1)
            self.assertTrue(
                pack_index["skipped_candidates"][0]["reason"].startswith(
                    "invalid_artifact:"
                )
            )

    def test_build_candidate_registry_marks_strategy_library_json_buildable(self) -> None:
        with TemporaryDirectory() as tmpdir:
            repo_root = Path(tmpdir)
            (repo_root / "config").mkdir(parents=True, exist_ok=True)
            strategy_library = repo_root / "strategy_library.json"
            strategy_library.write_text(
                json.dumps(
                    {
                        "manifest_version": "1.0",
                        "timeframe": "15m",
                        "strategies": [
                            {
                                "name": "TomacNQKillzoneBreakout15m",
                                "status": "ok",
                                "metadata": {"hypothesis": "15m breakout lane"},
                                "validation_metrics": {
                                    "sharpe": 0.0746,
                                    "trade_count": 22,
                                    "profit_factor": 1.1272,
                                    "total_profit_pct": 1.18,
                                },
                                "per_pair_metrics": {
                                    "NQ/USD": {
                                        "sharpe": 0.0746,
                                        "trade_count": 22,
                                        "profit_factor": 1.1272,
                                        "total_profit_pct": 1.18,
                                    }
                                },
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            (repo_root / "config" / "factor_candidate_harness_presets.json").write_text(
                json.dumps(
                    {
                        "schema_version": "factor-candidate-harness-presets/v1",
                        "candidates": [
                            {
                                "candidate_id": "strategy_library_candidate",
                                "display_name": "Strategy Library Candidate",
                                "artifact_source": {
                                    "strategy_library_json": str(strategy_library)
                                },
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            bundle = resolver.build_candidate_registry(repo_root=repo_root)

            candidate = bundle["candidates"][0]
            self.assertTrue(candidate["artifact_ready"])
            self.assertEqual(candidate["evidence_status"], "buildable")
            self.assertEqual(candidate["curation_decision"], "promote_to_candidate_pack")
            self.assertEqual(candidate["artifact_kind"], "strategy_library_json")
            self.assertEqual(
                candidate["pack_build_reason"],
                "buildable_from_reusable_artifact",
            )

    def test_build_candidate_packs_supports_strategy_library_json(self) -> None:
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            strategy_library = root / "strategy_library.json"
            strategy_library.write_text(
                json.dumps(
                    {
                        "manifest_version": "1.0",
                        "timeframe": "15m",
                        "strategies": [
                            {
                                "name": "TomacNQKillzoneBreakout15m",
                                "status": "ok",
                                "metadata": {"hypothesis": "15m breakout lane"},
                                "validation_metrics": {
                                    "sharpe": 0.0746,
                                    "trade_count": 22,
                                    "profit_factor": 1.1272,
                                    "total_profit_pct": 1.18,
                                },
                                "per_pair_metrics": {
                                    "NQ/USD": {
                                        "sharpe": 0.0746,
                                        "trade_count": 22,
                                        "profit_factor": 1.1272,
                                        "total_profit_pct": 1.18,
                                    }
                                },
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            output_dir = root / "out"

            pack_index = resolver.build_candidate_packs(
                repo_root=root,
                output_dir=output_dir,
                candidates=[
                    {
                        "candidate_id": "strategy_library_candidate",
                        "display_name": "Strategy Library Candidate",
                        "strategy_name": "TomacNQKillzoneBreakout15m",
                        "artifact_source": {
                            "strategy_library_json": str(strategy_library)
                        },
                        "base_timeframe": "15m",
                        "context_timeframes": ["1h", "4h"],
                        "pre_bayes_targets": ["session_breakout_gate"],
                        "belief_targets": ["entry_quality"],
                        "path_ranking_targets": ["recommended_path_bundle"],
                        "execution_tree_targets": ["execution_readiness"],
                    }
                ],
            )

            self.assertEqual(pack_index["summary"]["built_count"], 1)
            self.assertEqual(pack_index["summary"]["skipped_count"], 0)
            expression = json.loads(
                (
                    output_dir
                    / "packs"
                    / "strategy_library_candidate"
                    / "factor_expression.json"
                ).read_text(encoding="utf-8")
            )
            self.assertEqual(
                expression["strategy_name"],
                "TomacNQKillzoneBreakout15m",
            )


if __name__ == "__main__":
    unittest.main()
