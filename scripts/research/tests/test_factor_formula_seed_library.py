from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import factor_formula_seed_library as seeds  # noqa: E402


class FactorFormulaSeedLibraryTests(unittest.TestCase):
    def test_zero_config_seed_library_contains_first_16_candidates(self) -> None:
        payload = seeds.build_seed_library()

        self.assertEqual(payload["schema_version"], "factor-formula-seed-library/v1")
        self.assertEqual(payload["candidate_count"], 16)
        candidate_ids = {candidate["candidate_id"] for candidate in payload["candidates"]}
        self.assertIn("tsmom_mtf_convexity_v1", candidate_ids)
        self.assertIn("vrp_pressure_qqq_v1", candidate_ids)
        self.assertIn("ofi_book_pressure_v1", candidate_ids)
        self.assertIn("low_beta_stability_v1", candidate_ids)

    def test_candidates_are_hotplug_safe_and_keep_personal_fields_optional(self) -> None:
        payload = seeds.build_seed_library()

        for candidate in payload["candidates"]:
            self.assertEqual(
                candidate["missing_optional_policy"],
                "emit_missing_optional_and_continue",
            )
            self.assertEqual(candidate["promotion_gate"], "probe")
            self.assertIn("candidate_spec.json", candidate["artifact_contract"]["writes"])
            self.assertIn("factor_expression.json", candidate["artifact_contract"]["writes"])
            self.assertTrue(candidate["factor_expression"]["lookahead_safe"])
            self.assertIn("qqq_hv_pct_rank_252", candidate["optional_fields"])
            self.assertIn("vvix_over_vix", candidate["optional_fields"])

    def test_cli_writes_json_artifact(self) -> None:
        with TemporaryDirectory() as tmpdir:
            output = Path(tmpdir) / "factor_seed_candidates.json"

            exit_code = seeds.main(["--output", str(output)])

            self.assertEqual(exit_code, 0)
            payload = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(payload["candidate_count"], 16)
            self.assertEqual(
                payload["runtime_dependency_policy"],
                "sidecar_only_no_large_framework_import",
            )


if __name__ == "__main__":
    unittest.main()