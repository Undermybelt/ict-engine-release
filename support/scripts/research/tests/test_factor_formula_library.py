from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import factor_formula_library as library  # noqa: E402


class FactorFormulaLibraryTests(unittest.TestCase):
    def test_zero_config_library_contains_hotplug_seed_pool(self) -> None:
        result = library.build_formula_library()

        self.assertEqual(result["schema_version"], "factor-formula-library/v1")
        self.assertGreaterEqual(result["seed_count"], 6)
        seed_ids = {seed["seed_id"] for seed in result["seeds"]}
        self.assertIn("qlib_alpha158_momentum_roc", seed_ids)
        self.assertIn("alpha101_rank_decay_reversion", seed_ids)
        self.assertIn("vrp_compression_regime", seed_ids)
        first = result["seeds"][0]
        self.assertIn("expression", first)
        self.assertIn("required_fields", first)
        self.assertIn("mutation_hints", first)
        self.assertTrue(first["hotplug_ready"])

    def test_family_filter_returns_only_requested_factor_family(self) -> None:
        result = library.build_formula_library(families=["mean_reversion"])

        self.assertGreaterEqual(result["seed_count"], 1)
        self.assertEqual({seed["family"] for seed in result["seeds"]}, {"mean_reversion"})

    def test_cli_writes_json_and_jsonl_artifacts(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            output_json = tmp / "formula_library.json"
            output_jsonl = tmp / "formula_library.jsonl"

            exit_code = library.main(
                [
                    "--output-json",
                    str(output_json),
                    "--output-jsonl",
                    str(output_jsonl),
                    "--family",
                    "momentum",
                ]
            )

            self.assertEqual(exit_code, 0)
            payload = json.loads(output_json.read_text(encoding="utf-8"))
            self.assertGreaterEqual(payload["seed_count"], 1)
            self.assertIn('"family": "momentum"', output_jsonl.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()