from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import ofi_session_sidecar as ofi  # noqa: E402


class OfiSessionSidecarTests(unittest.TestCase):
    def test_ohlcv_only_rows_emit_low_confidence_proxy_without_failing(self) -> None:
        rows = [
            {"timestamp": "t0", "open": "100", "high": "101", "low": "99", "close": "100", "volume": "1000"},
            {"timestamp": "t1", "open": "100", "high": "103", "low": "100", "close": "102", "volume": "1500"},
        ]

        payload = ofi.build_ofi_session_sidecar(rows=rows, symbol="NQ")
        artifact = payload["artifacts"][-1]

        self.assertEqual(payload["schema_version"], "ofi-session-sidecar/v1")
        self.assertEqual(payload["candidate_id"], "ofi_book_pressure_v1")
        self.assertTrue(payload["zero_config_fallback"])
        self.assertEqual(payload["missing_optional_policy"], "emit_missing_optional_and_continue")
        self.assertEqual(artifact["fallback_mode"], "ohlcv_proxy_low_confidence")
        self.assertLessEqual(artifact["confidence"], 0.35)
        self.assertIn("bid_depth", artifact["missing_optional_fields"])

    def test_l2_and_trade_flow_inputs_emit_high_confidence_pressure(self) -> None:
        rows = [
            {
                "timestamp": "t0",
                "open": "100",
                "high": "101",
                "low": "99",
                "close": "100",
                "volume": "1000",
                "bid_depth": "700",
                "ask_depth": "300",
                "signed_trade_volume": "250",
                "spread": "0.01",
                "session": "ny_am",
            }
        ]

        payload = ofi.build_ofi_session_sidecar(rows=rows, symbol="NQ")
        artifact = payload["artifacts"][0]

        self.assertEqual(artifact["optional_input_status"]["bid_depth"], "present")
        self.assertEqual(artifact["fallback_mode"], "l2_trade_flow")
        self.assertAlmostEqual(artifact["depth_imbalance"], 0.4)
        self.assertGreater(artifact["ofi_pressure"], 0.0)
        self.assertGreaterEqual(artifact["confidence"], 0.9)
        self.assertIn("session_quality", artifact["bbn_targets"])
        self.assertIn("fill_viable", artifact["execution_tree_targets"])

    def test_cli_writes_json(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            src = tmp / "ofi.csv"
            out = tmp / "ofi.json"
            src.write_text(
                "timestamp,open,high,low,close,volume,bid_depth,ask_depth,signed_trade_volume,session\n"
                "t0,100,101,99,100,1000,700,300,250,ny_am\n",
                encoding="utf-8",
            )

            exit_code = ofi.main(["--input-csv", str(src), "--output-json", str(out), "--symbol", "NQ"])

            self.assertEqual(exit_code, 0)
            payload = json.loads(out.read_text(encoding="utf-8"))
            self.assertEqual(payload["row_count"], 1)
            self.assertEqual(payload["artifacts"][0]["symbol"], "NQ")


if __name__ == "__main__":
    unittest.main()
