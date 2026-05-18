from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_transition_governor as governor  # noqa: E402


class RegimeTransitionGovernorTests(unittest.TestCase):
    def _write_scores(self, path: Path, labels: list[str]) -> None:
        rows = []
        for idx, label in enumerate(labels):
            timestamp = f"t{idx}"
            rows.append({"timestamp": timestamp, "label_id": label, "score": 0.91, "threshold": 0.8, "decision": "positive", "abstain_reason": ""})
            other = "primary::RangeConsolidation" if label != "primary::RangeConsolidation" else "primary::TrendExpansion"
            rows.append({"timestamp": timestamp, "label_id": other, "score": 0.2, "threshold": 0.8, "decision": "negative", "abstain_reason": ""})
        path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")

    def _write_conformal(self, path: Path, latest_label: str = "primary::TrendExpansion", set_size: int = 1) -> None:
        label_set = [latest_label] if set_size == 1 else [latest_label, "primary::RangeConsolidation"]
        payload = {
            "schema_version": "regime-conformal-calibration/v1",
            "confidence_95": set_size == 1,
            "confidence_99": set_size == 1,
            "sets_by_target_coverage": {"0.99": {"t3": label_set}},
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def _write_distribution(self, path: Path, agreement: str = "agree", transitional: bool = False) -> None:
        payload = {
            "schema_version": "regime-distributional-agreement/v1",
            "timestamp": "t3",
            "top_label": "primary::TrendExpansion",
            "nearest_archetype_label": "primary::TrendExpansion",
            "agreement": agreement,
            "transitional_flag": transitional,
            "transitional_reasons": ["mixed_archetype_distance"] if transitional else [],
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def test_accepts_stable_regime_when_confidence_and_distribution_agree(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            output = tmp / "governor.json"
            self._write_scores(scores, ["primary::TrendExpansion"] * 4)
            self._write_conformal(conformal)
            self._write_distribution(distribution)

            result = governor.build_transition_governor_report(
                scores_path=scores,
                conformal_report_path=conformal,
                distributional_report_path=distribution,
                output_json=output,
            )

            self.assertEqual(result["schema_version"], "regime-transition-governor/v1")
            self.assertEqual(result["execution_tree_hint"], "accept_regime")
            self.assertEqual(result["transition_hazard"], 0.0)
            self.assertTrue(output.exists())

    def test_blocks_flip_flop_with_transition_guardrail(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            output = tmp / "governor.json"
            self._write_scores(scores, ["primary::TrendExpansion", "primary::RangeConsolidation", "primary::TrendExpansion", "primary::RangeConsolidation"])
            self._write_conformal(conformal, latest_label="primary::RangeConsolidation")
            self._write_distribution(distribution, agreement="disagree", transitional=True)

            result = governor.build_transition_governor_report(
                scores_path=scores,
                conformal_report_path=conformal,
                distributional_report_path=distribution,
                output_json=output,
                min_duration=3,
            )

            self.assertEqual(result["execution_tree_hint"], "transition_guardrail")
            self.assertGreater(result["transition_hazard"], 0.0)
            self.assertIn("duration_below_minimum", result["guardrail_reasons"])
            self.assertIn("distributional_disagreement", result["guardrail_reasons"])

    def test_unknown_or_wide_conformal_set_abstains(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            output = tmp / "governor.json"
            self._write_scores(scores, ["primary::Unknown"] * 4)
            self._write_conformal(conformal, latest_label="primary::Unknown", set_size=2)
            self._write_distribution(distribution, agreement="disagree", transitional=True)

            result = governor.build_transition_governor_report(
                scores_path=scores,
                conformal_report_path=conformal,
                distributional_report_path=distribution,
                output_json=output,
            )

            self.assertEqual(result["execution_tree_hint"], "unknown_abstain")
            self.assertIn("wide_conformal_set", result["guardrail_reasons"])

    def test_cli_writes_report(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            output = tmp / "governor.json"
            self._write_scores(scores, ["primary::TrendExpansion"] * 4)
            self._write_conformal(conformal)
            self._write_distribution(distribution)

            exit_code = governor.main([
                "--scores", str(scores),
                "--conformal-report", str(conformal),
                "--distributional-report", str(distribution),
                "--output-json", str(output),
            ])

            self.assertEqual(exit_code, 0)
            self.assertIn('"execution_tree_hint"', output.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()