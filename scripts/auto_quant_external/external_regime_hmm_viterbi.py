#!/usr/bin/env python3
"""
HMM/Viterbi independent regime label source.

This helper creates regime labels using Hidden Markov Models,
providing an independent validation source for the existing
regime classifier (base+pda + ExtraTrees).

Per TODO:
- HMM labels must be evaluated as independent validation, not used as the only truth.
- Promote only if current regime classifier agrees with HMM family labels better than baseline.
"""
from __future__ import annotations

import argparse
import json
import warnings
from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import numpy as np
import pandas as pd
from hmmlearn import hmm

warnings.filterwarnings("ignore", category=DeprecationWarning)


@dataclass
class HMMRegimeConfig:
    n_states: int = 4
    covariance_type: str = "full"
    n_iter: int = 100
    random_state: int = 42
    train_ratio: float = 0.7


@dataclass
class HMMRegimeResult:
    state_id: int
    family_label: str
    count: int
    pct: float
    mean_ret: float
    std_ret: float
    sharpe_proxy: float
    duration_bars: float


REGIME_FAMILY_MAP = {
    "trend_up": "trend",
    "trend_down": "trend",
    "range_volatile": "range",
    "range_quiet": "range",
    "transition": "transition",
    "crash": "transition",
    "recovery": "transition",
}

# V2: 8-state granular mapping for factor runtime
REGIME_FAMILY_MAP_V2 = {
    0: {"label": "TrendUpStrong", "family": "trend"},
    1: {"label": "TrendUpWeak", "family": "trend"},
    2: {"label": "TrendDownStrong", "family": "trend"},
    3: {"label": "TrendDownWeak", "family": "trend"},
    4: {"label": "RangeQuiet", "family": "range"},
    5: {"label": "RangeVolatile", "family": "range"},
    6: {"label": "Transition", "family": "transition"},
    7: {"label": "CrashRecovery", "family": "transition"},
}

# State ID to V2 label for 8-state model
STATE_TO_V2_LABEL = {
    0: "trend_up_strong",
    1: "trend_up_weak",
    2: "trend_down_strong",
    3: "trend_down_weak",
    4: "range_quiet",
    5: "range_volatile",
    6: "transition",
    7: "crash_recovery",
}


def load_candles(path: Path) -> pd.DataFrame:
    """Load OHLCV from feather or CSV."""
    if path.suffix == ".feather":
        df = pd.read_feather(path)
    else:
        df = pd.read_csv(path)
    # Handle various date formats
    if "date" in df.columns:
        if df["date"].dtype == "int64":
            # Assume epoch ms
            df["date"] = pd.to_datetime(df["date"], unit="ms")
        else:
            df["date"] = pd.to_datetime(df["date"])
        df = df.set_index("date")
    elif "timestamp" in df.columns:
        if df["timestamp"].dtype == "int64":
            df["timestamp"] = pd.to_datetime(df["timestamp"], unit="ms")
        else:
            df["timestamp"] = pd.to_datetime(df["timestamp"])
        df = df.set_index("timestamp")
    return df


def compute_features(df: pd.DataFrame) -> pd.DataFrame:
    """Compute regime-relevant features for HMM."""
    df = df.copy()
    df["log_ret"] = np.log(df["close"] / df["close"].shift(1))
    df["range_atr"] = (df["high"] - df["low"]) / df["close"].rolling(20).std()
    df["body_frac"] = abs(df["close"] - df["open"]) / (df["high"] - df["low"] + 1e-9)
    df["close_vs_high"] = (df["close"] - df["low"]) / (df["high"] - df["low"] + 1e-9)
    
    # Rolling stats
    df["ret_mean_20"] = df["log_ret"].rolling(20).mean()
    df["ret_std_20"] = df["log_ret"].rolling(20).std()
    df["ret_skew_20"] = df["log_ret"].rolling(20).skew()
    df["vol_ratio"] = df["ret_std_20"] / df["ret_std_20"].rolling(60).mean()
    
    # Trend efficiency
    df["trend_eff"] = abs(df["close"] - df["close"].shift(20)) / (
        (df["high"].rolling(20).max() - df["low"].rolling(20).min()) + 1e-9
    )
    
    # Drop NaN
    df = df.dropna()
    return df


def prepare_hmm_inputs(df: pd.DataFrame) -> np.ndarray:
    """Prepare feature matrix for HMM."""
    # Alternative: use features closer to classifier logic
    df["return_20d"] = df["close"].pct_change(20)
    df["return_60d"] = df["close"].pct_change(60)
    df["volatility_20d"] = df["close"].pct_change().rolling(20).std()
    df["above_sma200_proxy"] = (df["return_60d"] > 0).astype(float)
    
    # Use returns + vol instead of range/body features
    feature_cols = [
        "log_ret",
        "return_20d",
        "return_60d", 
        "volatility_20d",
        "above_sma200_proxy",
        "vol_ratio",
        "trend_eff",
    ]
    X = df[feature_cols].values.copy()
    # Handle NaN/inf
    X = np.nan_to_num(X, nan=0.0, posinf=0.0, neginf=0.0)
    # Standardize
    mean = X.mean(axis=0)
    std = X.std(axis=0)
    std[std == 0] = 1.0  # avoid div/0
    X = (X - mean) / std
    return X


def fit_hmm_model(X: np.ndarray, config: HMMRegimeConfig) -> hmm.GaussianHMM:
    """Fit Gaussian HMM."""
    model = hmm.GaussianHMM(
        n_components=config.n_states,
        covariance_type=config.covariance_type,
        n_iter=config.n_iter,
        random_state=config.random_state,
    )
    model.fit(X)
    return model


def decode_states(model: hmm.GaussianHMM, X: np.ndarray) -> np.ndarray:
    """Viterbi decode to get state sequence."""
    return model.predict(X)


def interpret_states(
    df: pd.DataFrame, states: np.ndarray, n_states: int
) -> Tuple[Dict[int, HMMRegimeResult], List[str]]:
    """Map hidden states to interpretable regime families."""
    df = df.copy()
    df["hmm_state"] = states
    
    results = {}
    family_labels = []
    
    # Sort states by sharpe to assign meaningful labels
    state_sharpes = []
    for state_id in range(n_states):
        mask = df["hmm_state"] == state_id
        state_df = df[mask]
        if len(state_df) > 0:
            mean_ret = state_df["log_ret"].mean() * 100
            std_ret = state_df["log_ret"].std() * 100
            sharpe = mean_ret / (std_ret + 1e-9)
            state_sharpes.append((state_id, sharpe, len(state_df)))
    
    # Sort by sharpe descending
    state_sharpes.sort(key=lambda x: x[1], reverse=True)
    
    # Assign labels based on sharpe ranking for 8-state model
    if n_states == 8:
        # Top 2 sharpe -> TrendUpStrong/Weak
        # Bottom 2 sharpe -> TrendDownStrong/Weak
        # Middle by vol -> RangeQuiet/Volatile/Transition/CrashRecovery
        state_to_label = {}
        for i, (state_id, sharpe, count) in enumerate(state_sharpes):
            if i == 0:
                label = "trend_up_strong"
            elif i == 1:
                label = "trend_up_weak"
            elif i == 6:
                label = "trend_down_weak"
            elif i == 7:
                label = "trend_down_strong"
            elif i == 2:
                label = "range_quiet"
            elif i == 3:
                label = "range_volatile"
            elif i == 4:
                label = "transition"
            else:  # i == 5
                label = "crash_recovery"
            state_to_label[state_id] = label
    else:
        # Fallback for non-8 states
        state_to_label = {}
        for state_id, sharpe, count in state_sharpes:
            if sharpe > 0.1:
                state_to_label[state_id] = "trend_up"
            elif sharpe < -0.1:
                state_to_label[state_id] = "trend_down"
            else:
                state_to_label[state_id] = "range"
    
    for state_id in range(n_states):
        mask = df["hmm_state"] == state_id
        state_df = df[mask]
        count = len(state_df)
        pct = count / len(df) * 100
        
        mean_ret = state_df["log_ret"].mean() * 100
        std_ret = state_df["log_ret"].std() * 100
        sharpe_proxy = mean_ret / (std_ret + 1e-9)
        
        # Duration (run length)
        state_runs = (df["hmm_state"] == state_id).astype(int)
        run_changes = np.diff(state_runs, prepend=0)
        run_starts = np.where(run_changes == 1)[0]
        durations = []
        for start in run_starts:
            end = start + 1
            while end < len(df) and df["hmm_state"].iloc[end] == state_id:
                end += 1
            durations.append(end - start)
        duration_bars = np.mean(durations) if durations else 0
        
        family = state_to_label.get(state_id, "range")
        # Extract family (trend/range/transition)
        if "trend" in family:
            family_label = "trend"
        elif "range" in family:
            family_label = "range"
        else:
            family_label = "transition"
        
        results[state_id] = HMMRegimeResult(
            state_id=state_id,
            family_label=family,  # Granular label
            count=count,
            pct=pct,
            mean_ret=mean_ret,
            std_ret=std_ret,
            sharpe_proxy=sharpe_proxy,
            duration_bars=duration_bars,
        )
        family_labels.append(family)
    
    return results, family_labels


def compute_transition_metrics(
    states: np.ndarray, family_labels: List[str]
) -> Dict:
    """Compute transition frequency and flip rate."""
    transitions = Counter()
    for i in range(len(states) - 1):
        src = states[i]
        dst = states[i + 1]
        if src != dst:
            transitions[(src, dst)] += 1
    
    total_transitions = sum(transitions.values())
    flip_rate = total_transitions / len(states)
    
    return {
        "total_transitions": total_transitions,
        "flip_rate": flip_rate,
        "transition_counts": {f"{k[0]}->{k[1]}": v for k, v in transitions.items()},
    }


def evaluate_against_existing_labels(
    df: pd.DataFrame, hmm_states: np.ndarray, existing_labels: Optional[List[str]] = None
) -> Dict:
    """Compare HMM states to existing regime labels (if available)."""
    if existing_labels is None or len(existing_labels) != len(df):
        return {"agreement": None, "note": "no existing labels to compare"}
    
    df = df.copy()
    df["hmm_state"] = hmm_states
    df["existing_label"] = existing_labels
    
    # Map existing labels to families
    existing_families = [REGIME_FAMILY_MAP.get(l, l) for l in existing_labels]
    
    # Agreement
    agreement_scores = []
    for hmm_state in df["hmm_state"].unique():
        hmm_mask = df["hmm_state"] == hmm_state
        hmm_family = df.loc[hmm_mask, "existing_label"].mode().iloc[0] if hmm_mask.any() else "unknown"
        existing_in_state = df.loc[hmm_mask, "existing_label"]
        if len(existing_in_state) > 0:
            agreement = (existing_in_state == hmm_family).sum() / len(existing_in_state)
            agreement_scores.append(agreement)
    
    return {
        "mean_agreement": np.mean(agreement_scores) if agreement_scores else 0,
        "per_state_agreement": agreement_scores,
    }


def main():
    parser = argparse.ArgumentParser(description="HMM/Viterbi regime labels")
    parser.add_argument("--candle-path", required=True, help="Path to OHLCV feather/CSV")
    parser.add_argument("--n-states", type=int, default=4, choices=[3, 4, 5, 6, 7, 8])
    parser.add_argument("--train-ratio", type=float, default=0.7)
    parser.add_argument("--output-dir", default="/tmp/hmm_regime_output")
    parser.add_argument("--existing-labels-path", default=None, help="Path to existing labels for comparison")
    args = parser.parse_args()
    
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Load data
    print(f"Loading candles from {args.candle_path}")
    df = load_candles(Path(args.candle_path))
    print(f"Loaded {len(df)} bars, date range: {df.index[0]} to {df.index[-1]}")
    
    # Compute features
    print("Computing features...")
    df_feat = compute_features(df)
    X = prepare_hmm_inputs(df_feat)
    
    # Train/test split
    split_idx = int(len(X) * args.train_ratio)
    X_train = X[:split_idx]
    X_test = X[split_idx:]
    
    # Fit HMM
    print(f"Fitting HMM with {args.n_states} states...")
    config = HMMRegimeConfig(n_states=args.n_states, train_ratio=args.train_ratio)
    model = fit_hmm_model(X_train, config)
    
    # Decode all
    states_train = decode_states(model, X_train)
    states_test = decode_states(model, X_test)
    states_all = np.concatenate([states_train, states_test])
    
    # Interpret
    results, family_labels = interpret_states(df_feat, states_all, args.n_states)
    
    # Transition metrics
    trans_metrics = compute_transition_metrics(states_all, family_labels)
    
    # Compare with existing labels if provided
    existing_labels = None
    if args.existing_labels_path:
        with open(args.existing_labels_path) as f:
            existing_labels = json.load(f)
    comparison = evaluate_against_existing_labels(df_feat, states_all, existing_labels)
    
    # Build per-bar labels
    per_bar_labels = []
    for i, (ts, state) in enumerate(zip(df_feat.index, states_all)):
        family = results[state].family_label if state in results else "unknown"
        per_bar_labels.append({
            "ts": ts.isoformat() if hasattr(ts, 'isoformat') else str(ts),
            "state": int(state),
            "family": family,
        })
    
    # Output
    output = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "config": {
            "n_states": args.n_states,
            "train_ratio": args.train_ratio,
            "n_train": len(X_train),
            "n_test": len(X_test),
        },
        "model_metrics": {
            "log_prob_train": model.score(X_train),
            "log_prob_test": model.score(X_test),
            "converged": model.monitor_.converged,
        },
        "labels": per_bar_labels,  # per-bar state/family labels
        "state_interpretation": {
            str(k): {
                "state_id": v.state_id,
                "family_label": v.family_label,
                "count": v.count,
                "pct": round(v.pct, 2),
                "mean_ret_ann": round(v.mean_ret, 4),
                "std_ret_ann": round(v.std_ret, 4),
                "sharpe_proxy": round(v.sharpe_proxy, 2),
                "duration_bars": round(v.duration_bars, 1),
            }
            for k, v in results.items()
        },
        "transition_metrics": trans_metrics,
        "existing_label_comparison": comparison,
        "family_distribution": dict(Counter(family_labels)),
    }
    
    output_path = output_dir / f"hmm_regime_{args.n_states}states.json"
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"Saved to {output_path}")
    
    # Summary
    print("\n=== HMM Regime Summary ===")
    print(f"States: {args.n_states}")
    print(f"Train bars: {len(X_train)}, Test bars: {len(X_test)}")
    print(f"Log prob (train): {output['model_metrics']['log_prob_train']:.2f}")
    print(f"Log prob (test): {output['model_metrics']['log_prob_test']:.2f}")
    print(f"Converged: {output['model_metrics']['converged']}")
    print(f"Flip rate: {trans_metrics['flip_rate']:.4f}")
    print("\nState interpretation:")
    for state_id, data in output["state_interpretation"].items():
        print(f"  State {state_id}: {data['family_label']} ({data['pct']:.1f}%, sharpe={data['sharpe_proxy']:.2f})")
    
    return output


if __name__ == "__main__":
    main()
