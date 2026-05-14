from __future__ import annotations

import csv
import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_sidecar_pipeline as pipeline  # noqa: E402


class RegimeSidecarPipelineTests(unittest.TestCase):
    def _write_ohlcv(self, path: Path) -> None:
        rows = [
            ["timestamp", "open", "high", "low", "close", "volume"],
            ["t0", "100", "101", "99.5", "100.8", "1000"],
            ["t1", "100.8", "102", "100.5", "101.7", "1200"],
            ["t2", "101.7", "103.5", "101.2", "103.0", "1500"],
            ["t3", "103.0", "104.8", "102.7", "104.4", "1800"],
        ]
        with path.open("w", newline="", encoding="utf-8") as handle:
            csv.writer(handle).writerows(rows)

    def _write_aux(self, path: Path) -> None:
        rows = [
            ["timestamp", "qqq_hv_level", "nq_vs_200d_pct", "vix3m_level", "qqq_hv_pct_rank_252", "vvix_over_vix"],
            ["t0", "0.20", "0.05", "17.1", "0.55", "5.0"],
            ["t1", "0.21", "0.06", "17.5", "0.58", "5.1"],
            ["t2", "0.22", "0.07", "18.0", "0.60", "5.2"],
            ["t3", "0.23", "0.08", "18.5", "0.62", "5.3"],
        ]
        with path.open("w", newline="", encoding="utf-8") as handle:
            csv.writer(handle).writerows(rows)

    def _write_truth(self, path: Path) -> None:
        rows = [
            {"timestamp": "t0", "label_id": "primary::TrendExpansion"},
            {"timestamp": "t1", "label_id": "primary::TrendExpansion"},
            {"timestamp": "t2", "label_id": "primary::TrendExpansion"},
            {"timestamp": "t3", "label_id": "primary::TrendExpansion"},
        ]
        path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")

    def test_pipeline_runs_r2_to_r10_with_ohlcv(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ohlcv = tmp / "ohlcv.csv"
            aux = tmp / "aux.csv"
            truth = tmp / "truth.jsonl"
            out = tmp / "out"
            self._write_ohlcv(ohlcv)
            self._write_aux(aux)
            self._write_truth(truth)

            result = pipeline.run_pipeline(
                ohlcv_path=ohlcv,
                output_dir=out,
                auxiliary_evidence_path=aux,
                truth_path=truth,
                label_prefix="primary::Trend",
            )

            self.assertEqual(result["schema_version"], "regime-sidecar-pipeline/v1")
            self.assertEqual(result["status"], "ok")
            self.assertEqual(result["final_decision"]["decision_state"], "single_label_99")
            self.assertTrue(result["final_decision"]["trade_usable"])
            self.assertTrue((out / "regime_consumer_bundle.json").exists())
            self.assertTrue((out / "regime_high_confidence_decision.json").exists())
            self.assertEqual(result["bundle_path"], str(out / "regime_consumer_bundle.json"))
            self.assertEqual(result["truth_joined_rows"], 4)
            training_report = json.loads((out / "regime_expert_training_report.json").read_text(encoding="utf-8"))
            trend_summary = next(item for item in training_report["experts"] if item["label_id"] == "primary::TrendExpansion")
            self.assertEqual(trend_summary["support"], 4)
            with (out / "regime_features.csv").open(newline="", encoding="utf-8") as handle:
                feature_rows = list(csv.DictReader(handle))
            self.assertTrue(all(row["primary_label"] == "TrendExpansion" for row in feature_rows))

    def test_missing_ohlcv_returns_input_contract_without_repo_state(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            out = tmp / "out"
            result = pipeline.run_pipeline(ohlcv_path=None, output_dir=out)

            self.assertEqual(result["status"], "input_required")
            self.assertIn("--ohlcv", result["input_contract"]["required"])
            self.assertFalse(out.exists())

    def test_cli_success_and_missing_input_modes(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ohlcv = tmp / "ohlcv.csv"
            out = tmp / "out"
            self._write_ohlcv(ohlcv)

            ok_code = pipeline.main(["--ohlcv", str(ohlcv), "--output-dir", str(out), "--label-prefix", "primary::Trend"])
            missing_code = pipeline.main(["--output-dir", str(tmp / "missing")])

            self.assertEqual(ok_code, 0)
            self.assertEqual(missing_code, 2)


if __name__ == "__main__":
    unittest.main()
