from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPT_ROOT.parents[2]
sys.path.insert(0, str(SCRIPT_ROOT))

import market_data_resolver as resolver  # noqa: E402


class MarketDataResolverTests(unittest.TestCase):
    def test_build_resolution_for_market_alias_without_profile(self) -> None:
        bundle = resolver.build_resolution_bundle(
            repo_root=REPO_ROOT,
            market_selector="NASDAQ_FUTURES",
        )

        self.assertEqual(bundle["symbol_resolution"]["market_key"], "NQ")
        self.assertEqual(
            bundle["symbol_resolution"]["live_defaults"]["spot_symbol"],
            "QQQ",
        )
        self.assertEqual(
            bundle["data_catalog"]["summary"]["default_provider_candidates"],
            ["yfinance", "tradingview_mcp", "ibkr"],
        )
        self.assertTrue(bundle["normalized_dataset_summary"]["resolution_ready"])
        self.assertFalse(bundle["normalized_dataset_summary"]["dataset_available"])
        self.assertEqual(
            bundle["normalized_dataset_summary"]["selection_mode"],
            "generic_zero_config",
        )

    def test_build_resolution_bundle_includes_opt_in_profile_lane(self) -> None:
        bundle = resolver.build_resolution_bundle(
            repo_root=REPO_ROOT,
            market_selector="NQ",
            profile_selector="thrill3r_nq_closed_loop_v1",
        )

        self.assertEqual(
            bundle["symbol_resolution"]["selected_profile"]["profile_id"],
            "thrill3r_nq_closed_loop_v1",
        )
        self.assertEqual(
            bundle["normalized_dataset_summary"]["selection_mode"],
            "profile_opt_in",
        )
        self.assertIn(
            "thrilL3r_nq_closed_loop_v1".lower(),
            bundle["normalized_dataset_summary"]["selection_label"].lower(),
        )
        historical_entries = [
            entry
            for entry in bundle["data_catalog"]["datasets"]
            if entry["category"] == "historical"
        ]
        self.assertEqual(len(historical_entries), 1)
        self.assertTrue(historical_entries[0]["opt_in_only"])
        self.assertEqual(
            historical_entries[0]["path_hint"],
            "<cleaned-mtf-root>",
        )

    def test_main_writes_expected_artifacts(self) -> None:
        with TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir)

            exit_code = resolver.main(
                [
                    "--repo-root",
                    str(REPO_ROOT),
                    "--market",
                    "GC",
                    "--output-dir",
                    str(output_dir),
                    "--bar-count",
                    "750",
                    "--timeframe",
                    "15m",
                    "--timeframe",
                    "1h",
                ]
            )

            self.assertEqual(exit_code, 0)
            catalog = json.loads(
                (output_dir / "data_catalog.json").read_text(encoding="utf-8")
            )
            symbol_resolution = json.loads(
                (output_dir / "symbol_resolution.json").read_text(encoding="utf-8")
            )
            dataset_summary = json.loads(
                (output_dir / "normalized_dataset_summary.json").read_text(
                    encoding="utf-8"
                )
            )

            self.assertEqual(symbol_resolution["market_key"], "GC")
            self.assertEqual(
                catalog["summary"]["requested_timeframes"],
                ["15m", "1h"],
            )
            self.assertEqual(dataset_summary["requested_bar_count"], 750)
            self.assertEqual(
                dataset_summary["requested_timeframes"],
                ["15m", "1h"],
            )


if __name__ == "__main__":
    unittest.main()
