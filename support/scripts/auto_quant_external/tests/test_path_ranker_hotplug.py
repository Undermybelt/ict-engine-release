from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import pandas as pd

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_ROOT))

import pandas_path_ranker_trainer as trainer  # noqa: E402
import path_ranker_integration as integration  # noqa: E402


class UserWeightsFallbackTests(unittest.TestCase):
    def test_weighted_sum_fallback_uses_user_weights_file(self) -> None:
        features = pd.DataFrame(
            {
                "evidence_quality_score": [1.0, 0.2],
                "risk_reward": [0.2, 1.0],
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            weights_path = Path(tmpdir) / "user_weights.json"
            weights_path.write_text(
                '{\n'
                '  "evidence_quality_score": 0.9,\n'
                '  "risk_reward": -0.4\n'
                '}\n',
                encoding="utf-8",
            )

            scores = trainer.weighted_sum_fallback(features, weights_path=weights_path)

        self.assertGreater(scores[0], scores[1])


class ReuseModelFlowTests(unittest.TestCase):
    def test_reuse_model_dir_skips_training_and_applies_existing_model(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            state_dir = Path(tmpdir) / "state"
            policy_dir = state_dir / "NQ" / "policy_training"
            policy_dir.mkdir(parents=True)
            target_csv = policy_dir / "structural_path_ranking_target.csv"
            target_csv.write_text("candidate_set_id,path_id\nset1,path1\n", encoding="utf-8")

            existing_model_dir = Path(tmpdir) / "existing_model"
            existing_model_dir.mkdir()

            with mock.patch.object(integration, "run_trainer") as run_trainer, mock.patch.object(
                integration, "run_apply"
            ) as run_apply:
                argv = [
                    "path_ranker_integration.py",
                    "--state-dir",
                    str(state_dir),
                    "--symbol",
                    "NQ",
                    "--reuse-model-dir",
                    str(existing_model_dir),
                ]
                with mock.patch.object(sys, "argv", argv):
                    integration.main()

            run_trainer.assert_not_called()
            run_apply.assert_called_once_with(
                str(existing_model_dir),
                str(target_csv),
                str(policy_dir / "scores.csv"),
                None,
                model_family="catboost",
                python_runner="auto",
                allow_direct_fallback=False,
            )

    def test_register_runtime_artifact_triggers_repo_cli_opt_in(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            state_dir = Path(tmpdir) / "state"
            policy_dir = state_dir / "NQ" / "policy_training"
            policy_dir.mkdir(parents=True)
            target_csv = policy_dir / "structural_path_ranking_target.csv"
            target_csv.write_text("candidate_set_id,path_id\nset1,path1\n", encoding="utf-8")
            output_dir = policy_dir / "path_ranker_model"
            output_dir.mkdir()
            (output_dir / "path_ranker_direct_model.json").write_text("{}", encoding="utf-8")

            with mock.patch.object(integration, "run_trainer") as run_trainer, mock.patch.object(
                integration, "run_apply"
            ) as run_apply, mock.patch.object(
                integration, "register_runtime_artifact"
            ) as register_runtime_artifact:
                argv = [
                    "path_ranker_integration.py",
                    "--state-dir",
                    str(state_dir),
                    "--symbol",
                    "NQ",
                    "--register-runtime-artifact",
                ]
                with mock.patch.object(sys, "argv", argv):
                    integration.main()

            run_trainer.assert_called_once()
            _, trainer_kwargs = run_trainer.call_args
            self.assertEqual(trainer_kwargs["python_runner"], "auto")
            self.assertFalse(trainer_kwargs["allow_direct_fallback"])
            run_apply.assert_called_once()
            _, apply_kwargs = run_apply.call_args
            self.assertEqual(apply_kwargs["python_runner"], "auto")
            self.assertFalse(apply_kwargs["allow_direct_fallback"])
            register_runtime_artifact.assert_called_once_with(
                state_dir=str(state_dir),
                symbol="NQ",
                model_dir=str(output_dir),
                target_csv=str(target_csv),
                score_column="raw_path_score",
                reuse_mode="candidate_set_only",
            )

    def test_register_runtime_artifact_uses_catboost_companion_family(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            state_dir = Path(tmpdir) / "state"
            model_dir = Path(tmpdir) / "catboost_model"
            model_dir.mkdir()
            target_csv = Path(tmpdir) / "target.csv"
            target_csv.write_text("candidate_set_id,path_id\nset1,path1\n", encoding="utf-8")
            companion_path = model_dir / "trainer_artifact.json"
            companion_path.write_text(
                json.dumps(
                    {
                        "protocol_version": "structural-path-ranking-trainer-artifact-v1",
                        "model_family": "catboost",
                        "artifact_uri": str(Path(tmpdir) / "scores.csv"),
                        "model_artifact_uri": str(model_dir / "catboost_model.cbm"),
                    }
                ),
                encoding="utf-8",
            )

            completed = subprocess_result()
            with mock.patch.object(integration.subprocess, "run", return_value=completed) as run:
                integration.register_runtime_artifact(
                    state_dir=str(state_dir),
                    symbol="NQ",
                    model_dir=str(model_dir),
                    target_csv=str(target_csv),
                )

            register_cmd = run.call_args_list[0].args[0]
            self.assertIn(str(companion_path), register_cmd)
            family_index = register_cmd.index("--model-family") + 1
            self.assertEqual(register_cmd[family_index], "catboost")

    def test_register_runtime_artifact_backfills_direct_model_for_legacy_model_dir(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            model_dir = Path(tmpdir) / "legacy_model"
            model_dir.mkdir()
            target_csv = Path(tmpdir) / "target.csv"
            target_csv.write_text(
                "candidate_set_id,path_id,rank,current_posterior,structural_baseline_score,maturity_mask,calibrated_label,training_weight\n"
                "set1,path1,1,0.7,0.8,true,1.0,1.0\n"
                "set1,path2,2,0.2,0.4,true,0.0,1.0\n",
                encoding="utf-8",
            )

            integration.ensure_runtime_artifact(model_dir=str(model_dir), target_csv=str(target_csv))

            artifact_path = model_dir / "path_ranker_direct_model.json"
            self.assertTrue(artifact_path.exists())
            artifact = json.loads(artifact_path.read_text(encoding="utf-8"))
            self.assertEqual(artifact["model_family"], "weighted_feature_sum_v1")

    def test_ensure_runtime_artifact_prefers_catboost_trainer_companion(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            model_dir = Path(tmpdir) / "catboost_model"
            model_dir.mkdir()
            companion_path = model_dir / "trainer_artifact.json"
            companion_path.write_text(
                json.dumps(
                    {
                        "model_family": "catboost",
                        "artifact_uri": str(Path(tmpdir) / "scores.csv"),
                        "model_artifact_uri": str(model_dir / "catboost_model.cbm"),
                    }
                ),
                encoding="utf-8",
            )
            target_csv = Path(tmpdir) / "target.csv"
            target_csv.write_text("candidate_set_id,path_id\nset1,path1\n", encoding="utf-8")

            artifact_path = integration.ensure_runtime_artifact(
                model_dir=str(model_dir),
                target_csv=str(target_csv),
            )

        self.assertEqual(artifact_path, str(companion_path))


class DirectModelArtifactTests(unittest.TestCase):
    def test_create_direct_model_artifact_emits_repo_runtime_contract(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir)
            trainer.create_direct_model_artifact(
                output_dir=output_dir,
                features=["rank", "current_posterior"],
                trained_rows=12,
                output_transform="sigmoid",
            )

            artifact = json.loads((output_dir / "path_ranker_direct_model.json").read_text(encoding="utf-8"))

        self.assertEqual(artifact["protocol_version"], "structural-path-ranking-direct-model-v1")
        self.assertEqual(artifact["model_family"], "weighted_feature_sum_v1")
        self.assertEqual(
            artifact["feature_schema_version"],
            "structural-path-ranking-trainer-manifest-v1",
        )
        self.assertEqual(artifact["output_transform"], "sigmoid")
        self.assertIn("current_posterior", artifact["numerical_feature_weights"])
        self.assertIn("rank", artifact["numerical_feature_weights"])

    def test_build_registered_artifact_prefers_direct_model_family_when_only_direct_model_exists(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir)
            trainer.create_direct_model_artifact(
                output_dir=output_dir,
                features=["rank"],
                trained_rows=7,
            )

            metadata = trainer.build_registered_artifact_metadata(
                output_dir=output_dir,
                scores_path=None,
                trained_rows=7,
                history_rows=9,
                calibration_rows=3,
                selected_features=["rank"],
            )

        self.assertEqual(metadata["protocol_version"], "structural-path-ranking-trainer-artifact-v1")
        self.assertEqual(metadata["model_family"], "weighted_feature_sum_v1")
        self.assertEqual(metadata["score_column"], "raw_path_score")
        self.assertEqual(metadata["trained_rows"], 7)
        self.assertEqual(metadata["history_rows"], 9)
        self.assertEqual(metadata["calibration_rows"], 3)
        self.assertTrue(str(metadata["artifact_uri"]).endswith("path_ranker_direct_model.json"))

    def test_build_registered_artifact_prefers_catboost_when_cbm_exists(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir)
            scores_path = Path(tmpdir) / "scores.csv"
            trainer.create_direct_model_artifact(
                output_dir=output_dir,
                features=["rank"],
                trained_rows=7,
            )
            (output_dir / "catboost_model.cbm").write_text("model", encoding="utf-8")

            metadata = trainer.build_registered_artifact_metadata(
                output_dir=output_dir,
                scores_path=scores_path,
                trained_rows=7,
                history_rows=9,
                calibration_rows=3,
                selected_features=["rank"],
            )

        self.assertEqual(metadata["model_family"], "catboost")
        self.assertEqual(metadata["artifact_uri"], str(scores_path))
        self.assertEqual(metadata["model_artifact_uri"], str(output_dir / "catboost_model.cbm"))
        self.assertIn("catboost_runtime_scores_uri=required", metadata["notes"])


def subprocess_result(stdout: str = "{}\n", stderr: str = "", returncode: int = 0):
    return mock.Mock(stdout=stdout, stderr=stderr, returncode=returncode)


if __name__ == "__main__":
    unittest.main()
