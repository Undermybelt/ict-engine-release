#!/usr/bin/env python3
"""
Path Ranker 一键集成脚本
========================

从导出 target → 训练模型 → 应用 scores 的完整流程。

用法：
    # 完整流程
    python path_ranker_integration.py --state-dir /tmp/vrp-v2-runtime-closure --symbol NQ

    # 仅训练
    python path_ranker_integration.py --train-only --state-dir /tmp/state --symbol NQ

    # 仅应用
    python path_ranker_integration.py --apply-only --model-dir /tmp/model --target-csv target.csv

零配置：默认行为可直接运行
热插拔：用户可通过 user_weights.json 自定义
"""

import argparse
import json
import shutil
import subprocess
import sys
from importlib import util as importlib_util
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
TRAINER_SCRIPT = SCRIPT_DIR / "pandas_path_ranker_trainer.py"
REPO_ROOT = SCRIPT_DIR.parents[1]
ICT_ENGINE_BIN = REPO_ROOT / "target" / "debug" / "ict-engine"

sys.path.insert(0, str(SCRIPT_DIR))
import pandas_path_ranker_trainer as trainer  # noqa: E402


def find_target_csv(state_dir: str, symbol: str) -> Path:
    """查找导出的 target CSV"""
    target_path = Path(state_dir) / symbol / "policy_training" / "structural_path_ranking_target.csv"
    if target_path.exists():
        return target_path
    
    # 备用路径
    alt_path = Path(state_dir) / symbol / "structural_path_ranking_target.csv"
    if alt_path.exists():
        return alt_path
    
    raise FileNotFoundError(f"No target CSV found in {state_dir}/{symbol}")


def current_python_has_modules(modules: list[str]) -> bool:
    return all(importlib_util.find_spec(module) is not None for module in modules)


def python_runner_command(model_family: str, python_runner: str) -> list[str]:
    if python_runner == "system":
        return [sys.executable]
    if python_runner not in {"auto", "uv"}:
        raise ValueError(f"unsupported python runner: {python_runner}")
    required = ["pandas", "numpy"]
    if model_family in {"catboost", "both"}:
        required.append("catboost")
    if model_family in {"xgboost", "both"}:
        required.append("xgboost")
    if python_runner == "auto" and current_python_has_modules(required):
        return [sys.executable]
    uv = shutil.which("uv")
    if not uv:
        raise RuntimeError(
            "CatBoost/path-ranker dependencies are missing from this Python and `uv` is not available; "
            "install uv or run with a Python environment containing pandas, numpy, and catboost"
        )
    cmd = [uv, "run", "--with", "pandas", "--with", "numpy"]
    if "catboost" in required:
        cmd.extend(["--with", "catboost"])
    if "xgboost" in required:
        cmd.extend(["--with", "xgboost"])
    cmd.append("python")
    return cmd


def run_trainer(
    target_csv: str,
    output_dir: str,
    model_family: str = "catboost",
    output_scores: str | None = None,
    python_runner: str = "auto",
    allow_direct_fallback: bool = False,
):
    """运行训练器"""
    cmd = [
        *python_runner_command(model_family, python_runner),
        str(TRAINER_SCRIPT),
        "--target-csv", target_csv,
        "--output-dir", output_dir,
        "--model-family", model_family,
    ]
    if output_scores:
        cmd.extend(["--output-scores", output_scores])
    if allow_direct_fallback:
        cmd.append("--allow-direct-fallback")

    print(f"[run] {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        print(f"[error] {result.stderr or result.stdout}")
        raise RuntimeError("Trainer failed")

    print(result.stdout)
    return result


def run_apply(
    model_dir: str,
    target_csv: str,
    output_scores: str,
    user_weights: str | None = None,
    model_family: str = "catboost",
    python_runner: str = "auto",
    allow_direct_fallback: bool = False,
):
    """运行应用"""
    cmd = [
        *python_runner_command(model_family, python_runner),
        str(TRAINER_SCRIPT),
        "--apply",
        "--model-dir", model_dir,
        "--target-csv", target_csv,
        "--output-scores", output_scores,
    ]
    if user_weights:
        cmd.extend(["--user-weights", user_weights])
    if allow_direct_fallback:
        cmd.append("--allow-direct-fallback")

    print(f"[run] {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        print(f"[error] {result.stderr or result.stdout}")
        raise RuntimeError("Apply failed")

    print(result.stdout)
    return result


def ensure_runtime_artifact(model_dir: str, target_csv: str) -> str:
    """Backfill a direct-model artifact for legacy model dirs before repo registration."""
    model_path = Path(model_dir)
    trainer_artifact_path = model_path / "trainer_artifact.json"
    if trainer_artifact_path.exists():
        return str(trainer_artifact_path)

    artifact_path = model_path / "path_ranker_direct_model.json"
    if artifact_path.exists():
        return str(artifact_path)

    df = trainer.load_target_csv(target_csv)
    _, _, _, features = trainer.prepare_features(df)
    trainer.create_direct_model_artifact(
        output_dir=model_path,
        features=features,
        trained_rows=len(df),
    )
    return str(artifact_path)


def runtime_artifact_model_family(artifact_path: str) -> str:
    """Read the repo runtime artifact family without exposing user-specific paths."""
    try:
        with open(artifact_path, "r", encoding="utf-8") as handle:
            payload = json.load(handle)
    except (json.JSONDecodeError, OSError):
        return "weighted_feature_sum_v1"
    family = str(payload.get("model_family", "")).strip()
    return family or "weighted_feature_sum_v1"


def register_runtime_artifact(
    state_dir: str,
    symbol: str,
    model_dir: str,
    target_csv: str,
    score_column: str = "raw_path_score",
    reuse_mode: str = "candidate_set_only",
):
    """Opt in to repo-side runtime reuse using the emitted direct-model artifact."""
    artifact_path = ensure_runtime_artifact(model_dir=model_dir, target_csv=target_csv)
    model_family = runtime_artifact_model_family(artifact_path)
    register_cmd = [
        str(ICT_ENGINE_BIN),
        "register-structural-path-ranking-trainer-artifact",
        "--symbol",
        symbol,
        "--state-dir",
        state_dir,
        "--artifact-uri",
        str(artifact_path),
        "--model-family",
        model_family,
        "--score-column",
        score_column,
    ]
    enable_cmd = [
        str(ICT_ENGINE_BIN),
        "enable-structural-path-ranking-runtime",
        "--symbol",
        symbol,
        "--state-dir",
        state_dir,
        "--reuse-mode",
        reuse_mode,
    ]
    for cmd in (register_cmd, enable_cmd):
        print(f"[run] {' '.join(cmd)}")
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"[error] {result.stderr}")
            raise RuntimeError("Runtime artifact registration failed")
        print(result.stdout)


def main():
    parser = argparse.ArgumentParser(description="Path Ranker Integration")
    parser.add_argument("--state-dir", required=False, help="State directory")
    parser.add_argument("--symbol", default="NQ", help="Symbol")
    parser.add_argument("--target-csv", help="Direct path to target CSV")
    parser.add_argument("--output-dir", default=None, help="Model output directory")
    parser.add_argument("--model-dir", default=None, help="Existing model directory (for apply)")
    parser.add_argument("--output-scores", default=None, help="Scores output path")
    parser.add_argument("--model-family", default="catboost", choices=["catboost", "xgboost", "both"])
    parser.add_argument(
        "--python-runner",
        default="auto",
        choices=["auto", "uv", "system"],
        help="Python runner for trainer/apply; auto provisions CatBoost via uv when needed",
    )
    parser.add_argument(
        "--allow-direct-fallback",
        action="store_true",
        help="Allow weighted_feature_sum_v1 fallback when no CatBoost/XGBoost model is available",
    )
    parser.add_argument("--train-only", action="store_true", help="Only train, skip apply")
    parser.add_argument("--apply-only", action="store_true", help="Only apply, skip train")
    parser.add_argument(
        "--reuse-model-dir",
        help="Reuse an existing model directory and skip training",
    )
    parser.add_argument(
        "--user-weights",
        help="Optional user_weights.json used when fallback scoring is needed",
    )
    parser.add_argument(
        "--register-runtime-artifact",
        action="store_true",
        help="Opt in to register the emitted direct-model artifact and enable repo runtime reuse",
    )
    parser.add_argument(
        "--reuse-mode",
        default="candidate_set_only",
        help="Runtime reuse mode used with --register-runtime-artifact",
    )
    
    args = parser.parse_args()
    
    # 确定路径
    if args.target_csv:
        target_csv = args.target_csv
    elif args.state_dir:
        target_csv = str(find_target_csv(args.state_dir, args.symbol))
    else:
        print("ERROR: Need --state-dir or --target-csv")
        sys.exit(1)
    
    if args.output_dir:
        output_dir = args.output_dir
    elif args.state_dir:
        output_dir = str(Path(args.state_dir) / args.symbol / "policy_training" / "path_ranker_model")
    else:
        output_dir = "./path_ranker_model"
    
    if args.output_scores:
        output_scores = args.output_scores
    elif args.state_dir:
        output_scores = str(Path(args.state_dir) / args.symbol / "policy_training" / "scores.csv")
    else:
        output_scores = "./scores.csv"
    
    # 执行
    if args.apply_only:
        model_dir = args.model_dir or output_dir
        run_apply(
            model_dir,
            target_csv,
            output_scores,
            args.user_weights,
            model_family=args.model_family,
            python_runner=args.python_runner,
            allow_direct_fallback=args.allow_direct_fallback,
        )
    elif args.reuse_model_dir:
        run_apply(
            args.reuse_model_dir,
            target_csv,
            output_scores,
            args.user_weights,
            model_family=args.model_family,
            python_runner=args.python_runner,
            allow_direct_fallback=args.allow_direct_fallback,
        )
    else:
        # 训练
        run_trainer(
            target_csv,
            output_dir,
            args.model_family,
            output_scores,
            python_runner=args.python_runner,
            allow_direct_fallback=args.allow_direct_fallback,
        )

        if not args.train_only:
            # 应用
            run_apply(
                output_dir,
                target_csv,
                output_scores,
                args.user_weights,
                model_family=args.model_family,
                python_runner=args.python_runner,
                allow_direct_fallback=args.allow_direct_fallback,
            )

    if args.register_runtime_artifact:
        if not args.state_dir:
            print("ERROR: --register-runtime-artifact requires --state-dir")
            sys.exit(1)
        register_runtime_artifact(
            state_dir=args.state_dir,
            symbol=args.symbol,
            model_dir=output_dir,
            target_csv=target_csv,
            score_column="raw_path_score",
            reuse_mode=args.reuse_mode,
        )
    
    print(f"\n[done] Model: {output_dir}")
    print(f"[done] Scores: {output_scores}")
    print(f"\n[next] Apply to runtime:")
    print(f"  ./target/debug/ict-engine apply-structural-path-ranking-external-scores \\")
    print(f"    --symbol {args.symbol} --state-dir {args.state_dir} --scores-file {output_scores}")


if __name__ == "__main__":
    main()
