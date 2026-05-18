from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import regime_high_confidence_decision as decision  # noqa: E402


class RegimeHighConfidenceDecisionTests(unittest.TestCase):
    def _write_scores(self, path: Path, label: str = "primary::TrendExpansion") -> None:
        rows = []
        for idx in range(4):
            timestamp = f"t{idx}"
            rows.append({"timestamp": timestamp, "label_id": label, "score": 0.93, "threshold": 0.8, "decision": "positive", "abstain_reason": ""})
            rows.append({"timestamp": timestamp, "label_id": "primary::RangeConsolidation", "score": 0.11, "threshold": 0.8, "decision": "negative", "abstain_reason": ""})
        path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")

    def _write_conformal(self, path: Path, *, label_set: list[str] | None = None, confidence_95: bool = True, confidence_99: bool = True) -> None:
        labels = label_set or ["primary::TrendExpansion"]
        payload = {
            "schema_version": "regime-conformal-calibration/v1",
            "confidence_95": confidence_95,
            "confidence_99": confidence_99,
            "sets_by_target_coverage": {"0.95": {"t3": labels}, "0.99": {"t3": labels}},
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def _write_distribution(self, path: Path, *, agreement: str = "agree", transitional: bool = False) -> None:
        payload = {
            "schema_version": "regime-distributional-agreement/v1",
            "timestamp": "t3",
            "top_label": "primary::TrendExpansion",
            "nearest_archetype_label": "primary::TrendExpansion",
            "agreement": agreement,
            "transitional_flag": transitional,
            "transitional_reasons": ["mixed_archetype_distance"] if transitional else [],
            "feature_group_summaries": {
                "user_vrp_nq": {
                    "qqq_hv_level": {"latest": 0.22, "mean": 0.2},
                    "nq_vs_200d_pct": {"latest": 0.08, "mean": 0.07},
                    "vix3m_level": {"latest": 18.5, "mean": 18.0},
                    "qqq_hv_pct_rank_252": {"latest": 0.61, "mean": 0.6},
                    "vvix_over_vix": {"latest": 5.2, "mean": 5.1},
                }
            },
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def _write_governor(self, path: Path, *, hint: str = "accept_regime", reasons: list[str] | None = None) -> None:
        payload = {
            "schema_version": "regime-transition-governor/v1",
            "timestamp": "t3",
            "current_label": "primary::TrendExpansion",
            "transition_hazard": 0.0 if hint == "accept_regime" else 0.55,
            "guardrail_reasons": reasons or [],
            "execution_tree_hint": hint,
            "bbn_evidence_hint": {"regime_transition_hazard": 0.0 if hint == "accept_regime" else 0.55, "regime_governor_hint": hint, "regime_governor_reasons": reasons or []},
        }
        path.write_text(json.dumps(payload), encoding="utf-8")

    def test_single_label_99_trade_usable_when_all_gates_agree(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            governor = tmp / "governor.json"
            output = tmp / "decision.json"
            self._write_scores(scores)
            self._write_conformal(conformal)
            self._write_distribution(distribution)
            self._write_governor(governor)

            result = decision.build_high_confidence_decision(
                scores_path=scores,
                conformal_report_path=conformal,
                distributional_report_path=distribution,
                governor_report_path=governor,
                output_json=output,
                label_prefix="primary::",
            )

            self.assertEqual(result["schema_version"], "regime-high-confidence-decision/v1")
            self.assertEqual(result["decision_state"], "single_label_99")
            self.assertTrue(result["trade_usable"])
            self.assertEqual(result["final_label"], "primary::TrendExpansion")
            self.assertEqual(result["execution_tree_hint"], "accept_regime")
            self.assertEqual(result["path_ranker_context"]["regime_label"], "primary::TrendExpansion")
            self.assertIn("qqq_hv_level", result["user_vrp_nq_context"])
            self.assertTrue(output.exists())

    def test_single_label_95_when_99_gate_fails_but_95_accepts(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            governor = tmp / "governor.json"
            output = tmp / "decision.json"
            self._write_scores(scores)
            self._write_conformal(conformal, confidence_95=True, confidence_99=False)
            self._write_distribution(distribution)
            self._write_governor(governor)

            result = decision.build_high_confidence_decision(
                scores_path=scores,
                conformal_report_path=conformal,
                distributional_report_path=distribution,
                governor_report_path=governor,
                output_json=output,
            )

            self.assertEqual(result["decision_state"], "single_label_95")
            self.assertTrue(result["trade_usable"])

    def test_label_set_is_not_trade_usable_but_preserves_candidates(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            governor = tmp / "governor.json"
            output = tmp / "decision.json"
            labels = ["primary::TrendExpansion", "primary::RangeConsolidation"]
            self._write_scores(scores)
            self._write_conformal(conformal, label_set=labels, confidence_95=False, confidence_99=False)
            self._write_distribution(distribution)
            self._write_governor(governor)

            result = decision.build_high_confidence_decision(
                scores_path=scores,
                conformal_report_path=conformal,
                distributional_report_path=distribution,
                governor_report_path=governor,
                output_json=output,
            )

            self.assertEqual(result["decision_state"], "label_set")
            self.assertFalse(result["trade_usable"])
            self.assertEqual(result["label_set"], labels)
            self.assertIn("wide_or_uncertain_label_set", result["abstain_reasons"])

    def test_transitional_and_unknown_abstain_states(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            governor = tmp / "governor.json"
            output = tmp / "decision.json"
            self._write_scores(scores, label="primary::Unknown")
            self._write_conformal(conformal, label_set=["primary::Unknown"], confidence_95=False, confidence_99=False)
            self._write_distribution(distribution, agreement="disagree", transitional=True)
            self._write_governor(governor, hint="unknown_abstain", reasons=["unknown_label"])

            result = decision.build_high_confidence_decision(
                scores_path=scores,
                conformal_report_path=conformal,
                distributional_report_path=distribution,
                governor_report_path=governor,
                output_json=output,
            )

            self.assertEqual(result["decision_state"], "unknown_abstain")
            self.assertFalse(result["trade_usable"])
            self.assertIn("unknown_label", result["abstain_reasons"])

    def test_cli_writes_report(self) -> None:
        with TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            scores = tmp / "scores.jsonl"
            conformal = tmp / "conformal.json"
            distribution = tmp / "distribution.json"
            governor = tmp / "governor.json"
            output = tmp / "decision.json"
            self._write_scores(scores)
            self._write_conformal(conformal)
            self._write_distribution(distribution)
            self._write_governor(governor)

            exit_code = decision.main([
                "--scores", str(scores),
                "--conformal-report", str(conformal),
                "--distributional-report", str(distribution),
                "--governor-report", str(governor),
                "--output-json", str(output),
            ])

            self.assertEqual(exit_code, 0)
            self.assertIn('"decision_state"', output.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
