from __future__ import annotations

import csv
import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_discovery_cluster as cluster  # noqa: E402
import regime_discovery_hmm as hmm  # noqa: E402


class RegimeDiscoveryTests(unittest.TestCase):
    def _write_features_csv(self, path: Path) -> None:
        rows = []
        for index in range(36):
            bucket = index % 3
            if bucket == 0:
                row = {"timestamp": f"t{index}", "atr_percentile": 0.18, "directional_efficiency_3": 0.82, "volume_percentile": 0.72, "rsi_3": 61, "range_position": 0.8}
            elif bucket == 1:
                row = {"timestamp": f"t{index}", "atr_percentile": 0.34, "directional_efficiency_3": 0.18, "volume_percentile": 0.46, "rsi_3": 50, "range_position": 0.5}
            else:
                row = {"timestamp": f"t{index}", "atr_percentile": 0.91, "directional_efficiency_3": 0.64, "volume_percentile": 0.93, "rsi_3": 24, "range_position": 0.1}
            rows.append(row)
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
            writer.writeheader()
            writer.writerows(rows)

    def _write_ontology(self, path: Path) -> None:
        payload = {
            "schema_version": "regime-ontology-manifest/v1",
            "experts": [
                {"label_id": "primary::TrendExpansion"},
                {"label_id": "primary::RangeConsolidation"},
                {"label_id": "primary::ExtremeStress"},
                {"label_id": "primary::ReversalBrewing"},
                {"label_id": "primary::Unknown"},
            ],
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def test_cluster_discovery_evaluates_k_range_and_maps_candidate_labels(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            features = tmp / "regime_features.csv"
            ontology = tmp / "regime_ontology_manifest.json"
            output = tmp / "cluster_regime_discovery_report.json"
            self._write_features_csv(features)
            self._write_ontology(ontology)

            result = cluster.build_cluster_discovery_report(features_path=features, ontology_path=ontology, output_json=output)

            self.assertEqual(result["schema_version"], "regime-discovery-cluster/v1")
            self.assertEqual(result["k_values"], list(range(3, 13)))
            self.assertIn("silhouette", result["k_metrics"]["3"])
            self.assertIn("candidate_label", result["states"][0])
            self.assertTrue(output.exists())
            self.assertEqual(json.loads(ontology.read_text(encoding="utf-8"))["schema_version"], "regime-ontology-manifest/v1")

    def test_hmm_discovery_reports_information_criteria_and_transition_persistence(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            features = tmp / "regime_features.csv"
            ontology = tmp / "regime_ontology_manifest.json"
            output = tmp / "hmm_regime_discovery_report.json"
            self._write_features_csv(features)
            self._write_ontology(ontology)

            result = hmm.build_hmm_discovery_report(features_path=features, ontology_path=ontology, output_json=output)

            self.assertEqual(result["schema_version"], "regime-discovery-hmm/v1")
            self.assertEqual(result["k_values"], list(range(3, 13)))
            self.assertIn("bic", result["k_metrics"]["3"])
            self.assertIn("aic", result["k_metrics"]["3"])
            self.assertIn("transition_persistence", result["k_metrics"]["3"])
            self.assertIn("candidate_label", result["states"][0])
            self.assertTrue(output.exists())
            self.assertEqual(json.loads(ontology.read_text(encoding="utf-8"))["schema_version"], "regime-ontology-manifest/v1")

    def test_discovery_clis_write_reports(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            features = tmp / "regime_features.csv"
            ontology = tmp / "regime_ontology_manifest.json"
            cluster_output = tmp / "cluster_regime_discovery_report.json"
            hmm_output = tmp / "hmm_regime_discovery_report.json"
            self._write_features_csv(features)
            self._write_ontology(ontology)

            cluster_exit = cluster.main(["--features", str(features), "--ontology", str(ontology), "--output-json", str(cluster_output)])
            hmm_exit = hmm.main(["--features", str(features), "--ontology", str(ontology), "--output-json", str(hmm_output)])

            self.assertEqual(cluster_exit, 0)
            self.assertEqual(hmm_exit, 0)
            self.assertIn('"best_k"', cluster_output.read_text(encoding="utf-8"))
            self.assertIn('"best_k"', hmm_output.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()