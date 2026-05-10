from __future__ import annotations

import csv
import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_feature_builder as builder  # noqa: E402


class RegimeFeatureBuilderTests(unittest.TestCase):
    def _write_ohlcv_csv(self, path: Path) -> None:
        rows = [
            {"timestamp": "t0", "open": 100, "high": 102, "low": 99, "close": 101, "volume": 1000},
            {"timestamp": "t1", "open": 101, "high": 104, "low": 100, "close": 103, "volume": 1200},
            {"timestamp": "t2", "open": 103, "high": 105, "low": 101, "close": 102, "volume": 900},
            {"timestamp": "t3", "open": 102, "high": 108, "low": 101, "close": 107, "volume": 1800},
        ]
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
            writer.writeheader()
            writer.writerows(rows)

    def test_zero_config_ohlcv_builds_core_feature_csv_and_quality_report(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ohlcv = tmp / "ohlcv.csv"
            out_csv = tmp / "regime_features.csv"
            report_json = tmp / "feature_quality_report.json"
            self._write_ohlcv_csv(ohlcv)

            result = builder.build_feature_artifacts(
                ohlcv_path=ohlcv,
                output_features=out_csv,
                output_report=report_json,
            )

            self.assertEqual(result["schema_version"], "regime-feature-builder/v1")
            self.assertEqual(result["row_count"], 4)
            with out_csv.open(encoding="utf-8") as handle:
                rows = list(csv.DictReader(handle))
            self.assertEqual(len(rows), 4)
            for column in [
                "return_1",
                "candle_range",
                "body_pct",
                "atr_3",
                "atr_percentile",
                "volume_percentile",
                "directional_efficiency_3",
                "rsi_3",
                "range_position",
                "mtf_alignment",
            ]:
                self.assertIn(column, rows[0])
            self.assertIn('"row_count": 4', report_json.read_text(encoding="utf-8"))

    def test_auxiliary_user_fields_pass_through_by_timestamp(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ohlcv = tmp / "ohlcv.csv"
            aux = tmp / "aux.jsonl"
            out_csv = tmp / "regime_features.csv"
            report_json = tmp / "feature_quality_report.json"
            self._write_ohlcv_csv(ohlcv)
            aux.write_text(
                json.dumps(
                    {
                        "timestamp": "t1",
                        "qqq_hv_level": 0.22,
                        "nq_vs_200d_pct": 0.07,
                        "vix3m_level": 18.5,
                        "qqq_hv_pct_rank_252": 0.81,
                        "vvix_over_vix": 5.6,
                    }
                )
                + "\n",
                encoding="utf-8",
            )

            builder.build_feature_artifacts(
                ohlcv_path=ohlcv,
                auxiliary_path=aux,
                output_features=out_csv,
                output_report=report_json,
            )

            with out_csv.open(encoding="utf-8") as handle:
                by_ts = {row["timestamp"]: row for row in csv.DictReader(handle)}
            self.assertEqual(by_ts["t1"]["qqq_hv_level"], "0.22")
            self.assertEqual(by_ts["t1"]["nq_vs_200d_pct"], "0.07")
            self.assertEqual(by_ts["t1"]["vix3m_level"], "18.5")
            self.assertEqual(by_ts["t1"]["qqq_hv_pct_rank_252"], "0.81")
            self.assertEqual(by_ts["t1"]["vvix_over_vix"], "5.6")

    def test_missing_optional_inputs_do_not_fail_and_report_missing_fields(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ohlcv = tmp / "ohlcv.csv"
            out_csv = tmp / "regime_features.csv"
            report_json = tmp / "feature_quality_report.json"
            self._write_ohlcv_csv(ohlcv)

            result = builder.build_feature_artifacts(
                ohlcv_path=ohlcv,
                output_features=out_csv,
                output_report=report_json,
            )

            self.assertTrue(out_csv.exists())
            self.assertIn("qqq_hv_level", result["missing_optional_fields"])
            self.assertEqual(result["optional_input_status"]["auxiliary_evidence"], "missing")
            self.assertEqual(result["optional_input_status"]["mtf_pda_events"], "missing")

    def test_cli_supports_jsonl_ohlcv_and_mtf_event_join(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            ohlcv = tmp / "ohlcv.jsonl"
            mtf = tmp / "mtf.jsonl"
            out_csv = tmp / "regime_features.csv"
            report_json = tmp / "feature_quality_report.json"
            ohlcv.write_text(
                "".join(
                    json.dumps(row) + "\n"
                    for row in [
                        {"timestamp": "t0", "open": 10, "high": 11, "low": 9, "close": 10.5, "volume": 100},
                        {"timestamp": "t1", "open": 10.5, "high": 12, "low": 10, "close": 11.5, "volume": 140},
                    ]
                ),
                encoding="utf-8",
            )
            mtf.write_text(json.dumps({"timestamp": "t1", "mtf_alignment": "aligned", "pda_event_count": 2}) + "\n", encoding="utf-8")

            exit_code = builder.main(
                [
                    "--ohlcv",
                    str(ohlcv),
                    "--mtf-pda-events",
                    str(mtf),
                    "--output-features",
                    str(out_csv),
                    "--output-report",
                    str(report_json),
                ]
            )

            self.assertEqual(exit_code, 0)
            with out_csv.open(encoding="utf-8") as handle:
                by_ts = {row["timestamp"]: row for row in csv.DictReader(handle)}
            self.assertEqual(by_ts["t1"]["mtf_alignment"], "aligned")
            self.assertEqual(by_ts["t1"]["pda_event_count"], "2")


if __name__ == "__main__":
    unittest.main()