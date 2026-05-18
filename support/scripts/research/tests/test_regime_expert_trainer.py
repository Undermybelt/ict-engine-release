from __future__ import annotations

import csv
import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_expert_trainer as trainer  # noqa: E402


class RegimeExpertTrainerTests(unittest.TestCase):
    def _write_ontology(self, path: Path) -> None:
        payload = {
            "schema_version": "regime-ontology-manifest/v1",
            "experts": [
                {
                    "label_id": "primary::TrendExpansion",
                    "label": "TrendExpansion",
                    "level": "primary",
                    "abstain_policy": "abstain_unless_singleton_conformal_set",
                    "required_features": ["atr_percentile", "directional_efficiency_3", "volume_percentile", "rsi_3"],
                },
                {
                    "label_id": "primary::RangeConsolidation",
                    "label": "RangeConsolidation",
                    "level": "primary",
                    "abstain_policy": "abstain_unless_singleton_conformal_set",
                    "required_features": ["atr_percentile", "directional_efficiency_3", "volume_percentile", "rsi_3"],
                },
                {
                    "label_id": "primary::Unknown",
                    "label": "Unknown",
                    "level": "primary",
                    "abstain_policy": "always_abstain",
                    "required_features": ["atr_percentile"],
                },
            ],
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def _write_features(self, path: Path) -> None:
        rows = [
            {"timestamp": "t0", "atr_percentile": 0.22, "directional_efficiency_3": 0.86, "volume_percentile": 0.74, "rsi_3": 63, "primary_label": "TrendExpansion"},
            {"timestamp": "t1", "atr_percentile": 0.26, "directional_efficiency_3": 0.80, "volume_percentile": 0.69, "rsi_3": 59, "primary_label": "TrendExpansion"},
            {"timestamp": "t2", "atr_percentile": 0.34, "directional_efficiency_3": 0.18, "volume_percentile": 0.44, "rsi_3": 50, "primary_label": "RangeConsolidation"},
            {"timestamp": "t3", "atr_percentile": 0.36, "directional_efficiency_3": 0.22, "volume_percentile": 0.48, "rsi_3": 51, "primary_label": "RangeConsolidation"},
        ]
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
            writer.writeheader()
            writer.writerows(rows)

    def test_trainer_loads_ontology_and_reports_one_summary_per_label(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ontology = tmp / "regime_ontology_manifest.json"
            features = tmp / "regime_features.csv"
            scores = tmp / "regime_expert_scores.jsonl"
            report = tmp / "regime_expert_training_report.json"
            self._write_ontology(ontology)
            self._write_features(features)

            result = trainer.build_expert_training_artifacts(
                ontology_path=ontology,
                features_path=features,
                output_scores=scores,
                output_report=report,
            )

            self.assertEqual(result["schema_version"], "regime-expert-training/v1")
            self.assertEqual(result["expert_count"], 3)
            self.assertEqual(len(result["experts"]), 3)
            self.assertTrue(scores.exists())
            self.assertTrue(report.exists())

    def test_scores_include_required_contract_fields_and_unknown_abstains(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ontology = tmp / "regime_ontology_manifest.json"
            features = tmp / "regime_features.csv"
            scores = tmp / "regime_expert_scores.jsonl"
            report = tmp / "regime_expert_training_report.json"
            self._write_ontology(ontology)
            self._write_features(features)

            trainer.build_expert_training_artifacts(
                ontology_path=ontology,
                features_path=features,
                output_scores=scores,
                output_report=report,
            )

            rows = [json.loads(line) for line in scores.read_text(encoding="utf-8").splitlines()]
            self.assertGreater(len(rows), 0)
            for field in ["timestamp", "label_id", "score", "threshold", "decision", "abstain_reason"]:
                self.assertIn(field, rows[0])
            unknown_rows = [row for row in rows if row["label_id"] == "primary::Unknown"]
            self.assertTrue(unknown_rows)
            self.assertTrue(all(row["decision"] == "abstain" for row in unknown_rows))

    def test_precision_first_thresholding_raises_threshold_for_ambiguous_labels(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ontology = tmp / "regime_ontology_manifest.json"
            features = tmp / "regime_features.csv"
            scores = tmp / "regime_expert_scores.jsonl"
            report = tmp / "regime_expert_training_report.json"
            self._write_ontology(ontology)
            self._write_features(features)

            result = trainer.build_expert_training_artifacts(
                ontology_path=ontology,
                features_path=features,
                output_scores=scores,
                output_report=report,
                precision_first=True,
            )

            thresholds = {row["label_id"]: row["threshold"] for row in result["experts"]}
            self.assertGreaterEqual(thresholds["primary::TrendExpansion"], 0.8)
            for expert in result["experts"]:
                for field in ["precision", "recall", "f1", "brier_proxy", "ece_proxy", "support", "threshold"]:
                    self.assertIn(field, expert)

    def test_cli_writes_report_and_scores(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ontology = tmp / "regime_ontology_manifest.json"
            features = tmp / "regime_features.csv"
            scores = tmp / "regime_expert_scores.jsonl"
            report = tmp / "regime_expert_training_report.json"
            self._write_ontology(ontology)
            self._write_features(features)

            exit_code = trainer.main([
                "--ontology", str(ontology),
                "--features", str(features),
                "--output-scores", str(scores),
                "--output-report", str(report),
            ])

            self.assertEqual(exit_code, 0)
            self.assertIn('"expert_count"', report.read_text(encoding="utf-8"))
            self.assertIn("primary::TrendExpansion", scores.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()