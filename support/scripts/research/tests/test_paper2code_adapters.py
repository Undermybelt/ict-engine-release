from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import paper2code_adapters as adapters  # noqa: E402


class Paper2CodeAdaptersTests(unittest.TestCase):
    def test_zero_config_report_scores_all_adapter_families(self) -> None:
        rows = [
            {"close": 100, "volume": 1000, "high": 101, "low": 99, "spread": 0.10, "signed_volume": 500},
            {"close": 101, "volume": 1300, "high": 102, "low": 100, "spread": 0.12, "signed_volume": 700},
            {"close": 99, "volume": 2100, "high": 103, "low": 98, "spread": 0.25, "signed_volume": -1600},
            {"close": 100, "volume": 1500, "high": 101, "low": 99, "spread": 0.14, "signed_volume": 400},
        ]

        result = adapters.build_adapter_report(rows=rows, candidate_id="paper-a")

        self.assertEqual(result["schema_version"], "paper2code-adapter-report/v1")
        self.assertEqual(result["candidate_id"], "paper-a")
        adapter_ids = {adapter["adapter_id"] for adapter in result["adapters"]}
        self.assertEqual(
            adapter_ids,
            {"rammstein_ou_reversion", "crowded_trades_pressure", "kyle_liquidity_slippage", "red_queens_friction"},
        )
        self.assertGreaterEqual(result["adapter_count"], 4)
        self.assertIn(result["execution_hint"], {"probe", "watch", "reject"})

    def test_kyle_adapter_penalizes_high_slippage_realism(self) -> None:
        rows = [
            {"close": 100, "volume": 100, "high": 102, "low": 98, "spread": 0.50, "signed_volume": 1000},
            {"close": 99, "volume": 90, "high": 101, "low": 97, "spread": 0.55, "signed_volume": -1200},
        ]

        result = adapters.build_adapter_report(rows=rows)
        kyle = next(adapter for adapter in result["adapters"] if adapter["adapter_id"] == "kyle_liquidity_slippage")

        self.assertGreater(kyle["risk_score"], 0.5)
        self.assertEqual(kyle["bbn_evidence_hint"], "liquidity_slippage_risk")

    def test_cli_writes_adapter_report_json(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            rows_jsonl = tmp / "rows.jsonl"
            output_json = tmp / "paper2code_report.json"
            rows_jsonl.write_text(
                json.dumps({"close": 100, "volume": 1000, "high": 101, "low": 99, "spread": 0.1, "signed_volume": 500}) + "\n"
                + json.dumps({"close": 101, "volume": 1200, "high": 102, "low": 100, "spread": 0.1, "signed_volume": 600}) + "\n",
                encoding="utf-8",
            )

            exit_code = adapters.main([
                "--rows-jsonl",
                str(rows_jsonl),
                "--output-json",
                str(output_json),
                "--candidate-id",
                "cli-paper",
            ])

            self.assertEqual(exit_code, 0)
            self.assertIn('"candidate_id": "cli-paper"', output_json.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()