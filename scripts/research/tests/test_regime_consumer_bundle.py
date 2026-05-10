from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_consumer_bundle as bundle  # noqa: E402


class RegimeConsumerBundleTests(unittest.TestCase):
    def _write_json(self, path: Path, payload: dict) -> None:
        path.write_text(json.dumps(payload), encoding="utf-8")

    def _decision_payload(self) -> dict:
        return {
            "schema_version": "regime-high-confidence-decision/v1",
            "timestamp": "t3",
            "decision_state": "single_label_99",
            "trade_usable": True,
            "final_label": "primary::TrendExpansion",
            "label_set": ["primary::TrendExpansion"],
            "abstain_reasons": [],
            "execution_tree_hint": "accept_regime",
            "bbn_evidence_hint": {"regime_decision_state": "single_label_99", "regime_trade_usable": True},
            "path_ranker_context": {"regime_label": "primary::TrendExpansion", "regime_trade_usable": True},
            "user_vrp_nq_context": {"qqq_hv_level": 0.22, "nq_vs_200d_pct": 0.08, "vix3m_level": 18.5, "qqq_hv_pct_rank_252": 0.61, "vvix_over_vix": 5.2},
        }

    def test_builds_token_friendly_bundle_from_known_artifacts(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ontology = tmp / "ontology.json"
            features = tmp / "features_report.json"
            decision = tmp / "decision.json"
            output = tmp / "bundle.json"
            self._write_json(ontology, {"schema_version": "regime-ontology-manifest/v1", "expert_count": 53})
            self._write_json(features, {"schema_version": "regime-feature-quality/v1", "row_count": 4})
            self._write_json(decision, self._decision_payload())

            result = bundle.build_consumer_bundle(
                include_artifacts=[f"ontology={ontology}", f"feature_quality={features}", f"decision={decision}"],
                output_json=output,
            )

            self.assertEqual(result["schema_version"], "regime-consumer-bundle/v1")
            self.assertEqual(result["latest_decision"]["decision_state"], "single_label_99")
            self.assertTrue(result["latest_decision"]["trade_usable"])
            self.assertEqual(result["consumer_hints"]["execution_tree_hint"], "accept_regime")
            self.assertIn("decision", result["artifacts"])
            self.assertEqual(result["artifacts"]["ontology"]["schema_version"], "regime-ontology-manifest/v1")
            self.assertLess(len(json.dumps(result)), 7000)
            self.assertTrue(output.exists())

    def test_missing_artifacts_are_reported_not_fatal(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            decision = tmp / "decision.json"
            missing = tmp / "missing.json"
            output = tmp / "bundle.json"
            self._write_json(decision, self._decision_payload())

            result = bundle.build_consumer_bundle(
                include_artifacts=[f"decision={decision}", f"r7={missing}"],
                output_json=output,
            )

            self.assertEqual(result["missing_artifacts"], ["r7"])
            self.assertEqual(result["artifacts"]["r7"]["status"], "missing")
            self.assertTrue(result["consumer_contract"]["optional_for_consumers"])

    def test_auto_discovers_default_artifacts_in_artifact_dir(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            decision = tmp / "regime_high_confidence_decision.json"
            governor = tmp / "regime_transition_governor_report.json"
            output = tmp / "bundle.json"
            self._write_json(decision, self._decision_payload())
            self._write_json(governor, {"schema_version": "regime-transition-governor/v1", "execution_tree_hint": "accept_regime", "transition_hazard": 0.0})

            result = bundle.build_consumer_bundle(artifact_dir=tmp, include_artifacts=[], output_json=output)

            self.assertIn("decision", result["artifacts"])
            self.assertIn("transition_governor", result["artifacts"])
            self.assertEqual(result["latest_decision"]["final_label"], "primary::TrendExpansion")

    def test_cli_writes_bundle(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            decision = tmp / "decision.json"
            output = tmp / "bundle.json"
            self._write_json(decision, self._decision_payload())

            exit_code = bundle.main([
                "--include-artifact", f"decision={decision}",
                "--output-json", str(output),
            ])

            self.assertEqual(exit_code, 0)
            self.assertIn('"regime-consumer-bundle/v1"', output.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
