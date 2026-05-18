from __future__ import annotations

import csv
import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_distributional_agreement_report as dist  # noqa: E402


class RegimeDistributionalAgreementReportTests(unittest.TestCase):
    def _write_features(self, path: Path) -> None:
        rows = [
            {"timestamp": "t0", "atr_percentile": 0.22, "directional_efficiency_3": 0.86, "volume_percentile": 0.74, "rsi_3": 63, "qqq_hv_level": 0.18, "nq_vs_200d_pct": 0.12, "vix3m_level": 17, "qqq_hv_pct_rank_252": 0.31, "vvix_over_vix": 4.2},
            {"timestamp": "t1", "atr_percentile": 0.25, "directional_efficiency_3": 0.81, "volume_percentile": 0.69, "rsi_3": 59, "qqq_hv_level": 0.19, "nq_vs_200d_pct": 0.11, "vix3m_level": 18, "qqq_hv_pct_rank_252": 0.32, "vvix_over_vix": 4.1},
            {"timestamp": "t2", "atr_percentile": 0.34, "directional_efficiency_3": 0.18, "volume_percentile": 0.44, "rsi_3": 50, "qqq_hv_level": 0.20, "nq_vs_200d_pct": 0.08, "vix3m_level": 19, "qqq_hv_pct_rank_252": 0.35, "vvix_over_vix": 4.0},
        ]
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
            writer.writeheader()
            writer.writerows(rows)

    def _write_scores(self, path: Path) -> None:
        rows = [
            {"timestamp": "t2", "label_id": "primary::TrendExpansion", "score": 0.87, "threshold": 0.8, "decision": "positive", "abstain_reason": ""},
            {"timestamp": "t2", "label_id": "primary::RangeConsolidation", "score": 0.42, "threshold": 0.8, "decision": "negative", "abstain_reason": ""},
        ]
        path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")

    def _write_conformal(self, path: Path) -> None:
        payload = {
            "schema_version": "regime-conformal-calibration/v1",
            "sets_by_target_coverage": {"0.99": {"t2": ["primary::TrendExpansion"]}},
            "confidence_95": True,
            "confidence_99": True,
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def test_report_compares_current_window_to_label_archetypes(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            features = tmp / "regime_features.csv"
            scores = tmp / "regime_expert_scores.jsonl"
            conformal = tmp / "regime_conformal_calibration_report.json"
            output = tmp / "regime_distributional_agreement_report.json"
            self._write_features(features)
            self._write_scores(scores)
            self._write_conformal(conformal)

            result = dist.build_distributional_agreement_report(
                features_path=features,
                scores_path=scores,
                conformal_report_path=conformal,
                output_json=output,
                label_prefix="primary::",
            )

            self.assertEqual(result["schema_version"], "regime-distributional-agreement/v1")
            self.assertEqual(result["top_label"], "primary::TrendExpansion")
            self.assertIn("primary::TrendExpansion", result["label_distances"])
            self.assertIn(result["agreement"], {"agree", "disagree"})
            self.assertIn("transitional_flag", result)
            self.assertTrue(output.exists())

    def test_user_vrp_nq_fields_remain_visible_in_feature_group_summaries(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            features = tmp / "regime_features.csv"
            scores = tmp / "regime_expert_scores.jsonl"
            conformal = tmp / "regime_conformal_calibration_report.json"
            output = tmp / "regime_distributional_agreement_report.json"
            self._write_features(features)
            self._write_scores(scores)
            self._write_conformal(conformal)

            result = dist.build_distributional_agreement_report(
                features_path=features,
                scores_path=scores,
                conformal_report_path=conformal,
                output_json=output,
            )

            user_fields = result["feature_group_summaries"]["user_vrp_nq"]
            for field in ["qqq_hv_level", "nq_vs_200d_pct", "vix3m_level", "qqq_hv_pct_rank_252", "vvix_over_vix"]:
                self.assertIn(field, user_fields)

    def test_cli_writes_report(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            features = tmp / "regime_features.csv"
            scores = tmp / "regime_expert_scores.jsonl"
            conformal = tmp / "regime_conformal_calibration_report.json"
            output = tmp / "regime_distributional_agreement_report.json"
            self._write_features(features)
            self._write_scores(scores)
            self._write_conformal(conformal)

            exit_code = dist.main([
                "--features", str(features),
                "--scores", str(scores),
                "--conformal-report", str(conformal),
                "--output-json", str(output),
            ])

            self.assertEqual(exit_code, 0)
            self.assertIn('"top_label"', output.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()