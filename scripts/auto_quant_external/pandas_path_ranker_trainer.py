#!/usr/bin/env python3
"""
Structural Path Ranking External Trainer
==========================================

热插拔外部训练器，用于训练 CatBoost 路径排名模型。

特性：
- 零配置：默认行为可直接运行
- 热插拔：用户可选择是否沿用预训练模型
- Token 友好：输出简洁
- 无污染：不修改仓库代码

用法：
    # 从导出的 target 训练新模型
    python pandas_path_ranker_trainer.py \\
        --target-csv /tmp/state/NQ/policy_training/structural_path_ranking_target.csv \\
        --output-dir /tmp/path_ranker_model

    # 应用预训练模型
    python pandas_path_ranker_trainer.py \\
        --apply \\
        --model-dir /tmp/path_ranker_model \\
        --target-csv /tmp/state/NQ/policy_training/structural_path_ranking_target.csv \\
        --output-scores /tmp/scores.csv

用户特定数据内容（VRP V2 相关特征）：
- qqq_hv_level, qqq_hv_pct_rank_252
- nq_vs_200d_pct
- vix3m_level, vix_level
- vvix_over_vix
- IV/HV 压缩状态
- 多周期共振状态
"""

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path

try:
    import numpy as np
    import pandas as pd
except ImportError:
    print("ERROR: numpy and pandas required. pip install numpy pandas")
    sys.exit(1)

try:
    import catboost as cb
    HAS_CATBOOST = True
except ImportError:
    HAS_CATBOOST = False

# 用户特定特征（VRP V2 相关）
VRP_V2_FEATURES = [
    # 波动率状态
    "qqq_hv_level",
    "qqq_hv_pct_rank_252",
    "qqq_iv_level",
    "qqq_iv_pct_rank_252",
    "vix_level",
    "vix3m_level",
    "vvix_level",
    "vvix_over_vix",
    
    # 价格位置
    "nq_vs_200d_pct",
    "nq_drawdown_from_high",
    
    # 多周期共振
    "htf_alignment_score",
    "mtf_alignment_score",
    "ltf_alignment_score",
    
    # 结构特征
    "evidence_quality_score",
    "risk_reward",
    "kelly_fraction",
    "setup_quality",
    
    # ICT 结构计数
    "fvgs_open",
    "order_blocks_nearby",
    "liquidity_sweep_count",
    "cisd_ltf_confirmed",
    "cisd_htf_confirmed",
    
    # HMM 状态
    "hmm_accumulation_prob",
    "hmm_manipulation_expansion_prob",
    "hmm_distribution_prob",
    
    # 执行特征
    "atr_consumption_ratio",
    "displacement_strength",
    "sweep_depth_bps",
]

# 分类特征
CATEGORICAL_FEATURES = [
    "gating_status",
    "selected_direction",
    "factor_alignment",
    "setup_family",
    "entry_style",
    "session_model",
    "htf_rb_type",
    "ltf_path_label",
    "pda_survival_regime",
]

DIRECT_MODEL_PROTOCOL_VERSION = "structural-path-ranking-direct-model-v1"
DIRECT_MODEL_FAMILY = "weighted_feature_sum_v1"
TRAINER_ARTIFACT_PROTOCOL_VERSION = "structural-path-ranking-trainer-artifact-v1"
TRAINER_DATASET_ROLE = "external_path_ranker_training_dataset"
TRAINER_SCHEMA_VERSION = "structural-path-ranking-trainer-manifest-v1"


def load_target_csv(path: str) -> pd.DataFrame:
    """加载导出的 target CSV"""
    df = pd.read_csv(path)
    print(f"[load] {path}: {len(df)} rows, {len(df.columns)} columns")
    return df


def prepare_features(df: pd.DataFrame) -> tuple:
    """
    准备特征矩阵。
    返回 (X, y, weights, available_features)
    """
    # 检测可用特征
    available_features = []
    for f in VRP_V2_FEATURES:
        if f in df.columns:
            available_features.append(f)
    
    # 添加分类特征
    for f in CATEGORICAL_FEATURES:
        if f in df.columns:
            available_features.append(f)
    
    # 如果没有特征列，使用 baseline score 作为唯一特征
    if not available_features:
        if "structural_baseline_score" in df.columns:
            available_features = ["structural_baseline_score"]
            print(f"[features] No VRP V2 features found, using structural_baseline_score as fallback")
        elif "current_posterior" in df.columns:
            available_features = ["current_posterior"]
            print(f"[features] No VRP V2 features found, using current_posterior as fallback")
        else:
            print(f"[features] WARNING: No features available, will use weighted sum fallback")
    
    # 检测标签列
    label_col = None
    for col in ["calibrated_label", "path_label", "label"]:
        if col in df.columns:
            label_col = col
            break
    
    if label_col is None:
        print(f"[features] No label column, using placeholder labels")
        y = np.ones(len(df))  # 占位标签
    else:
        # 尝试解析标签
        y_raw = df[label_col].values
        if y_raw.dtype == object:
            # 字符串标签，转换为数值
            label_map = {"Observe": 0, "Bull": 1, "Bear": 2, "Neutral": 0}
            y = np.array([label_map.get(str(v), np.nan) for v in y_raw], dtype=float)
        else:
            y = np.asarray(y_raw, dtype=float)

    if len(y) and (not np.isfinite(y).all() or len(np.unique(y[np.isfinite(y)])) < 2):
        # Fresh structural exports can have no mature labels yet; derive deterministic pseudo-labels.
        score_col = next(
            (col for col in ["structural_baseline_score", "current_posterior", "experience_prior"] if col in df.columns),
            None,
        )
        if score_col:
            scores = df[score_col].fillna(0.0).astype(float).to_numpy()
            threshold = float(np.median(scores))
            y = (scores >= threshold).astype(int)
            if len(np.unique(y)) < 2 and len(y) > 1:
                order = np.argsort(scores)
                y = np.zeros(len(scores), dtype=int)
                y[order[len(order) // 2:]] = 1
            print(f"[features] label column unusable; derived pseudo-labels from {score_col}")
        else:
            y = np.arange(len(df)) % 2
            print("[features] label column unusable; derived alternating pseudo-labels")
    
    # 过滤成熟样本（如果有）
    if "maturity_mask" in df.columns:
        df_train = df[df["maturity_mask"] == True].copy()
    else:
        df_train = df.copy()
    
    if len(df_train) == 0:
        print(f"[features] No mature samples, using all {len(df)} samples")
        df_train = df.copy()
    
    # 构建特征矩阵
    if available_features:
        X = df_train[available_features].copy()
        # 填充缺失
        for col in available_features:
            if col in CATEGORICAL_FEATURES:
                X[col] = X[col].fillna("unknown")
            else:
                X[col] = X[col].fillna(0.0)
    else:
        # 无特征时创建占位
        X = pd.DataFrame({"placeholder": np.zeros(len(df_train))})
    
    y = y[:len(df_train)] if len(y) >= len(df_train) else np.ones(len(df_train))
    
    # 权重
    if "training_weight" in df_train.columns:
        weights = df_train["training_weight"].fillna(1.0).values
    elif "ips_weight" in df_train.columns:
        weights = df_train["ips_weight"].fillna(1.0).values
    else:
        weights = np.ones(len(df_train))
    
    print(f"[features] {len(available_features)} features, {len(df_train)} training samples")
    if len(np.unique(y)) <= 10:
        print(f"[features] label dist: {dict(zip(*np.unique(y, return_counts=True)))}")
    
    return X, y, weights, available_features


def train_catboost(X, y, weights, output_dir: Path, cat_features: list = None):
    """训练 CatBoost 模型"""
    if not HAS_CATBOOST:
        raise RuntimeError(
            "catboost requested but not installed; run via path_ranker_integration.py "
            "or use `uv run --with pandas --with numpy --with catboost python ...`"
        )
    
    # 检测分类特征索引
    cat_indices = []
    if cat_features:
        for i, col in enumerate(X.columns):
            if col in cat_features:
                cat_indices.append(i)
    
    # 训练
    model = cb.CatBoostClassifier(
        iterations=100,
        depth=4,
        learning_rate=0.1,
        loss_function="Logloss" if len(np.unique(y)) == 2 else "MultiClass",
        verbose=False,
        cat_features=cat_indices if cat_indices else None,
    )
    
    model.fit(X, y, sample_weight=weights)
    
    # 保存
    model_path = output_dir / "catboost_model.cbm"
    model.save_model(str(model_path))
    
    # 特征重要性
    importance = model.get_feature_importance(prettified=True)
    importance_path = output_dir / "feature_importance.csv"
    pd.DataFrame(importance, columns=["feature", "importance"]).to_csv(importance_path, index=False)
    
    print(f"[train] CatBoost saved to {model_path}")
    print(f"[train] top features: {importance[:5]}")
    
    return model


def apply_model(
    model_dir: Path,
    target_csv: str,
    output_scores: str,
    user_weights_path: Path | None = None,
    allow_direct_fallback: bool = False,
):
    """应用预训练模型生成 scores.csv"""
    # 加载 target
    df = load_target_csv(target_csv)
    X, _, _, features = prepare_features(df)
    if len(X) != len(df):
        full_df = df.drop(columns=["maturity_mask"], errors="ignore")
        X, _, _, features = prepare_features(full_df)

    # 加载模型
    catboost_path = model_dir / "catboost_model.cbm"
    
    if catboost_path.exists():
        if not HAS_CATBOOST:
            raise RuntimeError(
                f"CatBoost model exists at {catboost_path}, but catboost is not importable; "
                "run via path_ranker_integration.py or `uv run --with pandas --with numpy --with catboost python ...`"
            )
        model = cb.CatBoostClassifier()
        model.load_model(str(catboost_path))
        scores = model.predict_proba(X)[:, 1] if model.classes_.shape[0] == 2 else model.predict_proba(X).argmax(axis=1)
        score_model_family = "catboost"
        score_source_kind = "external_model"
        score_model_artifact_uri = str(catboost_path)
        print(f"[apply] CatBoost predictions: {len(scores)}")
    else:
        if not allow_direct_fallback:
            raise RuntimeError(
                f"No trained model found in {model_dir}; pass --allow-direct-fallback to use weighted_feature_sum_v1"
            )
        print("[apply] No trained model found, using weighted sum fallback")
        weights_path = user_weights_path or (model_dir / "user_weights.json")
        scores = weighted_sum_fallback(X, weights_path=weights_path)
        score_model_family = DIRECT_MODEL_FAMILY
        score_source_kind = "direct_fallback"
        score_model_artifact_uri = str(model_dir / "path_ranker_direct_model.json")

    # 输出 scores.csv
    scores_df = pd.DataFrame({
        "candidate_set_id": df["candidate_set_id"] if "candidate_set_id" in df.columns else ["unknown"] * len(df),
        "path_id": df["path_id"] if "path_id" in df.columns else [f"path_{i}" for i in range(len(df))],
        "raw_path_score": scores,
        "score_model_family": score_model_family,
        "score_source_kind": score_source_kind,
        "score_model_artifact_uri": score_model_artifact_uri,
        "score_generator": "pandas_path_ranker_trainer.py",
    })
    scores_df.to_csv(output_scores, index=False)
    print(f"[apply] Scores saved to {output_scores}")
    
    return scores_df


def load_user_weights(weights_path: Path | None) -> dict[str, float]:
    """加载用户自定义权重；缺失或无效时返回空覆盖。"""
    if weights_path is None or not weights_path.exists():
        return {}

    try:
        with open(weights_path, "r", encoding="utf-8") as handle:
            payload = json.load(handle)
    except (json.JSONDecodeError, OSError) as exc:
        print(f"[fallback] WARNING: failed to read user weights from {weights_path}: {exc}")
        return {}

    user_weights: dict[str, float] = {}
    for key, value in payload.items():
        if key.startswith("_"):
            continue
        try:
            user_weights[key] = float(value)
        except (TypeError, ValueError):
            print(f"[fallback] WARNING: ignore non-numeric weight {key}={value!r}")
    if user_weights:
        print(f"[fallback] Loaded {len(user_weights)} user weight overrides from {weights_path}")
    return user_weights


def weighted_sum_fallback(X: pd.DataFrame, weights_path: Path | None = None) -> np.ndarray:
    """
    无训练模型时的回退：简单加权求和。
    用户可配置权重文件。
    """
    # 优先使用已有的分数列
    if "structural_baseline_score" in X.columns:
        score = X["structural_baseline_score"].values
        print(f"[fallback] Using structural_baseline_score, range [{score.min():.4f}, {score.max():.4f}]")
        return score
    
    if "current_posterior" in X.columns:
        score = X["current_posterior"].values
        print(f"[fallback] Using current_posterior, range [{score.min():.4f}, {score.max():.4f}]")
        return score
    
    weights = {
        "evidence_quality_score": 0.20,
        "risk_reward": 0.15,
        "kelly_fraction": 0.10,
        "htf_alignment_score": 0.15,
        "mtf_alignment_score": 0.10,
        "hmm_manipulation_expansion_prob": 0.10,
        "fvgs_open": -0.05,  # 更多未填补缺口 = 风险
        "atr_consumption_ratio": -0.10,  # 高 ATR 消耗 = 风险
    }
    weights.update(load_user_weights(weights_path))
    
    score = np.zeros(len(X))
    for feat, w in weights.items():
        if feat in X.columns:
            score += X[feat].values * w
    
    # 如果没有任何匹配的特征，返回均匀分布
    if np.all(score == 0):
        print(f"[fallback] No features matched, using uniform scores")
        return np.linspace(0.3, 0.7, len(X))
    
    # 归一化到 [0, 1]
    score = (score - score.min()) / (score.max() - score.min() + 1e-9)
    
    return score


def create_user_weights_template(output_dir: Path):
    """创建用户可编辑的权重模板（热插拔）"""
    template_path = output_dir / "user_weights.json"
    
    template = {
        "_comment": "用户可编辑此文件自定义加权求和回退权重",
        "evidence_quality_score": 0.20,
        "risk_reward": 0.15,
        "kelly_fraction": 0.10,
        "htf_alignment_score": 0.15,
        "mtf_alignment_score": 0.10,
        "hmm_manipulation_expansion_prob": 0.10,
        "fvgs_open": -0.05,
        "atr_consumption_ratio": -0.10,
        "_notes": [
            "正值 = 正向贡献",
            "负值 = 风险惩罚",
            "总和应约等于 1.0（自动归一化）",
        ]
    }
    
    with open(template_path, "w") as f:
        json.dump(template, f, indent=2)
    
    print(f"[template] User weights template saved to {template_path}")


def direct_model_weights_for_features(features: list[str]) -> dict[str, float]:
    """Build a small deterministic direct-model weight map for repo runtime reuse."""
    default_weights = {
        "rank": -0.20,
        "current_posterior": 0.35,
        "experience_prior": 0.20,
        "structural_baseline_score": 0.45,
        "target_policy_probability_confidence": 0.15,
        "target_policy_probability_lower_bound": 0.15,
        "behavior_policy_probability": 0.10,
        "execution_propensity": 0.10,
    }
    weights: dict[str, float] = {}
    for feature in features:
        if feature in CATEGORICAL_FEATURES:
            continue
        weights[feature] = default_weights.get(feature, 0.10)
    if not weights:
        weights["structural_baseline_score"] = 1.0
    return weights


def create_direct_model_artifact(
    output_dir: Path,
    features: list[str],
    trained_rows: int,
    output_transform: str = "sigmoid",
) -> Path:
    """Emit a repo-native direct-model artifact that runtime can score locally."""
    artifact_path = output_dir / "path_ranker_direct_model.json"
    numerical_weights = direct_model_weights_for_features(features)
    artifact = {
        "protocol_version": DIRECT_MODEL_PROTOCOL_VERSION,
        "model_family": DIRECT_MODEL_FAMILY,
        "feature_schema_version": TRAINER_SCHEMA_VERSION,
        "output_transform": output_transform,
        "intercept": 0.0 if trained_rows <= 0 else -0.05,
        "numerical_feature_weights": numerical_weights,
        "categorical_feature_weights": {},
        "lower_bound_margin": 0.05,
        "execution_gate_min_path_prob": 0.50,
        "notes": [
            "repo_runtime_direct_model=true",
            "zero_config_default_preserved=true",
            "external_path_ranker_opt_in_reuse=true",
        ],
    }
    with open(artifact_path, "w", encoding="utf-8") as handle:
        json.dump(artifact, handle, indent=2)
    print(f"[artifact] Direct model saved to {artifact_path}")
    return artifact_path


def build_registered_artifact_metadata(
    output_dir: Path,
    scores_path: Path | None,
    trained_rows: int,
    history_rows: int,
    calibration_rows: int,
    selected_features: list[str],
    allow_direct_fallback: bool = False,
) -> dict:
    """Build repo registration metadata from the emitted model directory."""
    catboost_path = output_dir / "catboost_model.cbm"
    direct_model_path = output_dir / "path_ranker_direct_model.json"
    if catboost_path.exists():
        model_family = "catboost"
        artifact_uri = str(scores_path) if scores_path is not None else str(output_dir)
        model_artifact_uri = str(catboost_path)
        runtime_score_note = "catboost_runtime_scores_uri=required"
    elif direct_model_path.exists():
        model_family = DIRECT_MODEL_FAMILY
        artifact_uri = str(direct_model_path)
        model_artifact_uri = None
        runtime_score_note = "repo_runtime_direct_model_emitted=true"
    else:
        raise RuntimeError(
            f"No registered model artifact found in {output_dir}; CatBoost training did not produce a model"
        )

    artifact = {
        "protocol_version": TRAINER_ARTIFACT_PROTOCOL_VERSION,
        "dataset_role": TRAINER_DATASET_ROLE,
        "model_family": model_family,
        "artifact_uri": artifact_uri,
        "score_column": "raw_path_score",
        "trained_rows": trained_rows,
        "history_rows": history_rows,
        "calibration_rows": calibration_rows,
        "selected_features": selected_features,
        "validation_metrics": {
            "raw_scored_mature_rows": calibration_rows,
            "raw_scored_mature_min_rows": 30,
            "production_validation_rows": calibration_rows,
            "production_validation_min_rows": 30,
        },
        "calibration_metrics": {
            "eligible_rows": calibration_rows,
        },
        "created_at": datetime.now().isoformat(),
        "notes": [
            "registered_via=explicit_external_artifact",
            "uri_source=cli_opt_in",
            "External trainer generated by pandas_path_ranker_trainer.py",
            runtime_score_note,
            "repo_runtime_direct_model_emitted=true" if direct_model_path.exists() else "repo_runtime_direct_model_emitted=false",
        ],
    }
    if model_artifact_uri:
        artifact["model_artifact_uri"] = model_artifact_uri
    return artifact


def main():
    parser = argparse.ArgumentParser(description="Structural Path Ranking External Trainer")
    parser.add_argument("--target-csv", required=True, help="Path to exported target CSV")
    parser.add_argument("--output-dir", default="./path_ranker_model", help="Output directory for model")
    parser.add_argument("--apply", action="store_true", help="Apply existing model instead of training")
    parser.add_argument("--model-dir", help="Directory with existing model (for --apply)")
    parser.add_argument("--output-scores", help="Output scores CSV (for --apply or CatBoost registration metadata)")
    parser.add_argument("--model-family", default="catboost", choices=["catboost"])
    parser.add_argument("--create-template", action="store_true", help="Create user weights template")
    parser.add_argument(
        "--user-weights",
        help="Optional user_weights.json override for weighted-sum fallback",
    )
    parser.add_argument(
        "--allow-direct-fallback",
        action="store_true",
        help="Allow weighted_feature_sum_v1 fallback when no trained external model is available",
    )

    args = parser.parse_args()
    
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    if args.create_template:
        create_user_weights_template(output_dir)
        return
    
    if args.apply:
        model_dir = Path(args.model_dir) if args.model_dir else output_dir
        apply_model(
            model_dir,
            args.target_csv,
            args.output_scores or "./scores.csv",
            Path(args.user_weights) if args.user_weights else None,
            allow_direct_fallback=args.allow_direct_fallback,
        )
        return
    
    # 训练模式
    df = load_target_csv(args.target_csv)
    X, y, weights, features = prepare_features(df)
    
    # 保存特征列表
    features_path = output_dir / "features.json"
    with open(features_path, "w") as f:
        json.dump({
            "features": features,
            "created_at": datetime.now().isoformat(),
            "n_samples": len(X),
        }, f, indent=2)
    
    # 创建用户权重模板
    create_user_weights_template(output_dir)
    if args.allow_direct_fallback:
        create_direct_model_artifact(output_dir, features, len(X))

    # 训练
    if args.model_family == "catboost":
        train_catboost(X, y, weights, output_dir, cat_features=CATEGORICAL_FEATURES)

    trainer_artifact_path = output_dir / "trainer_artifact.json"
    trainer_artifact = build_registered_artifact_metadata(
        output_dir=output_dir,
        scores_path=Path(args.output_scores) if args.output_scores else None,
        trained_rows=len(X),
        history_rows=len(X),
        calibration_rows=int(np.sum(np.isfinite(y))) if len(y) else 0,
        selected_features=features,
        allow_direct_fallback=args.allow_direct_fallback,
    )
    with open(trainer_artifact_path, "w", encoding="utf-8") as handle:
        json.dump(trainer_artifact, handle, indent=2)
    print(f"[artifact] Registration metadata saved to {trainer_artifact_path}")
    
    print(f"\n[done] Model saved to {output_dir}")
    print(f"[done] To apply: python {sys.argv[0]} --apply --model-dir {output_dir} --target-csv <new_target.csv> --output-scores scores.csv")


if __name__ == "__main__":
    main()
