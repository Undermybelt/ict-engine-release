from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_artifact_bundle as bundle  # noqa: E402


class RegimeArtifactBundleTests(unittest.TestCase):
    def test_build_bundle_summarizes_top_ranked_results(self) -> None:
        nq = {
            "symbol": "NQ",
            "base_timeframe": "1d",
            "bar_count": 2513,
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

        payload = bundle.build_regime_artifact_bundle(
            benchmarks=[nq, spy],
            candidate_id="regime_primary_gate_pending_v1",
            display_name="Primary Regime Gate",
        )

        self.assertEqual(payload["regime_classifier_summary"]["best_market"], "SPY")
        self.assertEqual(payload["regime_classifier_summary"]["market_count"], 2)
        self.assertEqual(
            payload["transition_summary"]["best_transition_f1"],
            0.0,
        )
        self.assertEqual(
            payload["resonance_summary"]["max_resonance_4h"],
            0.0,
        )
        self.assertEqual(
            payload["cross_market_summary"]["covered_markets"],
            ["NQ", "SPY"],
        )

    def test_main_writes_expected_artifacts(self) -> None:
        nq = {
            "symbol": "NQ",
            "base_timeframe": "1d",
            "bar_count": 2513,
            "truth_mode": "post_transition_direction",
            "ranked_results": [{"name": "trained_family_extra_trees_v1", "eval_macro_f1": 0.42}],
        }
        with TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            benchmark_path = root / "nq.json"
            output_dir = root / "out"
            benchmark_path.write_text(json.dumps(nq), encoding="utf-8")

            exit_code = bundle.main(
                [
                    "--candidate-id",
                    "regime_primary_gate_pending_v1",
                    "--display-name",
                    "Primary Regime Gate",
                    "--benchmark-json",
                    str(benchmark_path),
                    "--output-dir",
                    str(output_dir),
                ]
            )

            self.assertEqual(exit_code, 0)
            classifier = json.loads(
                (output_dir / "regime_classifier_summary.json").read_text(encoding="utf-8")
            )
            transition = json.loads(
                (output_dir / "transition_summary.json").read_text(encoding="utf-8")
            )
            resonance = json.loads(
                (output_dir / "resonance_summary.json").read_text(encoding="utf-8")
            )
            cross_market = json.loads(
                (output_dir / "cross_market_summary.json").read_text(encoding="utf-8")
            )

            self.assertEqual(classifier["candidate_id"], "regime_primary_gate_pending_v1")
            self.assertEqual(transition["market_count"], 1)
            self.assertEqual(resonance["market_count"], 1)
            self.assertEqual(cross_market["covered_markets"], ["NQ"])


if __name__ == "__main__":
    unittest.main()
