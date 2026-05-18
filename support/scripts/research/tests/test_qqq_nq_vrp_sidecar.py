from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import qqq_nq_vrp_sidecar as vrp  # noqa: E402


class QqqNqVrpSidecarTests(unittest.TestCase):
    def test_missing_optional_inputs_do_not_fail_and_emit_status(self) -> None:
        rows = [
            {"timestamp": "t0", "close": "100"},
            {"timestamp": "t1", "close": "101"},
            {"timestamp": "t2", "close": "99"},
        ]

        payload = vrp.build_vrp_sidecar(rows=rows, symbol="NQ", hv_lookback=2)

        self.assertEqual(payload["schema_version"], "qqq-nq-vrp-sidecar/v1")
        self.assertEqual(payload["candidate_id"], "vrp_pressure_qqq_v1")
        self.assertTrue(payload["zero_config_fallback"])
        self.assertEqual(payload["missing_optional_policy"], "emit_missing_optional_and_continue")
        self.assertEqual(payload["row_count"], 3)
        self.assertIn("vix3m_level", payload["missing_optional_fields"])
        self.assertIn("vvix_over_vix", payload["missing_optional_fields"])
        last = payload["artifacts"][-1]
        self.assertEqual(last["optional_input_status"]["vix3m_level"], "missing_optional")
        self.assertGreaterEqual(last["confidence"], 0.0)

    def test_vrp_pressure_uses_optional_vix_vix3m_vvix_hv_iv_fields(self) -> None:
        rows = [
            {
                "timestamp": "t0",
                "close": "100",
                "vix_level": "16",
                "vix3m_level": "18",
                "vvix_level": "96",
                "qqq_hv_level": "12",
                "qqq_hv_pct_rank_252": "0.45",
                "nq_vs_200d_pct": "8",
                "iv_rank": "0.70",
                "hv_rank": "0.40",
            }
        ]

        payload = vrp.build_vrp_sidecar(rows=rows, symbol="NQ")
        artifact = payload["artifacts"][0]

        self.assertEqual(artifact["candidate_id"], "vrp_pressure_qqq_v1")
        self.assertEqual(artifact["optional_input_status"]["vix3m_level"], "present")
        self.assertAlmostEqual(artifact["features"]["vvix_over_vix"], 6.0)
        self.assertAlmostEqual(artifact["features"]["vrp"], 6.0)
        self.assertGreater(artifact["vrp_pressure"], 0.0)
        self.assertGreater(artifact["confidence"], 0.8)
        self.assertIn("dealer_pressure", artifact["bbn_targets"])

    def test_cli_writes_json(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            src = tmp / "features.csv"
            out = tmp / "vrp.json"
            src.write_text(
                "timestamp,close,vix3m_level,qqq_hv_level,vvix_over_vix\n"
                "t0,100,18,12,5.5\n"
                "t1,101,19,13,5.8\n",
                encoding="utf-8",
            )

            exit_code = vrp.main(["--input-csv", str(src), "--output-json", str(out), "--symbol", "NQ"])

            self.assertEqual(exit_code, 0)
            payload = json.loads(out.read_text(encoding="utf-8"))
            self.assertEqual(payload["row_count"], 2)
            self.assertEqual(payload["artifacts"][0]["symbol"], "NQ")


if __name__ == "__main__":
    unittest.main()
