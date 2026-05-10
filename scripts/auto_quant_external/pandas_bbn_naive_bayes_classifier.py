"""
pandas_bbn_naive_bayes_classifier.py — Slice 118. Build a Naive-Bayes-style
BBN regime classifier using the top-5 evidence features identified in
Slice 117. Validate train/test split for honest OOS performance.

BBN aggregation:
  P(regime | features) ∝ P(regime) × ∏ P(feature_i | regime)

The "naive" part: assume features are conditionally independent given
regime. Reasonable here since they capture different aspects of vol
regime (level vs price-vs-trend vs forward-vol vs vol-of-vol-ratio).

Train: 2018-01-01 to 2022-12-31 (5 years)
Test:  2023-01-01 to 2025-12-31 (3 years)

Output: per-test-day posterior probabilities P(class | features), saved
to /tmp/ict-engine-ibkr-probe/slice_118_bbn_predictions.csv for use in
Slice 119+ VRP integration.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

NQ_15M = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
VIX3M_CSV = PROBE_DIR / "vix3m.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
VIX_CSV = PROBE_DIR / "vix.1d.10y.csv"

START = pd.Timestamp("2018-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
TRAIN_END = pd.Timestamp("2023-01-01", tz="UTC")
FORWARD_DAYS = 20
N_BINS = 6
LAPLACE_ALPHA = 0.5
CLASS_NAMES = ["crash", "down", "flat", "up", "strong_up"]


def load_close(csv_path):
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def label_outcome(future_ret):
    if future_ret < -0.08: return 0
    if future_ret < -0.01: return 1
    if future_ret < 0.01: return 2
    if future_ret < 0.08: return 3
    return 4


def build_features():
    df15 = pd.read_feather(NQ_15M)
    df15["date"] = pd.to_datetime(df15["date"], unit="ms", utc=True)
    df15 = df15.set_index("date").sort_index().loc[START:END]
    nq_daily = df15["close"].resample("1D").last().dropna()
    nq_daily.index = nq_daily.index.normalize()

    sma200 = nq_daily.rolling(200, min_periods=100).mean()

    qqq_hv = load_close(QQQ_HV_CSV).reindex(nq_daily.index).ffill()
    vix3m = load_close(VIX3M_CSV).reindex(nq_daily.index).ffill()
    vvix = load_close(VVIX_CSV).reindex(nq_daily.index).ffill()
    vix = load_close(VIX_CSV).reindex(nq_daily.index).ffill()

    feats = pd.DataFrame(index=nq_daily.index)
    feats["qqq_hv_level"] = qqq_hv
    feats["nq_vs_200d_pct"] = nq_daily / sma200 - 1.0
    feats["vix3m_level"] = vix3m
    feats["qqq_hv_pct_rank_252"] = qqq_hv.rolling(252, min_periods=128).rank(pct=True)
    feats["vvix_over_vix"] = vvix / vix.where(vix > 1e-9)

    future_ret = nq_daily.shift(-FORWARD_DAYS) / nq_daily - 1.0
    outcome = future_ret.apply(label_outcome)

    df = feats.copy()
    df["outcome"] = outcome
    df = df.dropna()
    return df


def discretize_train(train_df, n_bins=N_BINS):
    """Return per-feature train-quantile bin edges."""
    edges = {}
    for col in train_df.columns:
        if col == "outcome": continue
        try:
            _, e = pd.qcut(train_df[col], q=n_bins, retbins=True, duplicates="drop")
            edges[col] = e
        except Exception:
            edges[col] = np.linspace(train_df[col].min(), train_df[col].max(), n_bins + 1)
    return edges


def apply_bins(df, edges):
    binned = pd.DataFrame(index=df.index)
    for col, e in edges.items():
        binned[col] = pd.cut(df[col], bins=e, labels=False, include_lowest=True)
    binned = binned.fillna(0).astype(int)
    return binned


def fit_naive_bayes(train_binned, train_y, n_classes=5, alpha=LAPLACE_ALPHA):
    """
    Returns:
      prior: P(class), shape (n_classes,)
      likelihoods: dict[feature] -> P(bin | class), shape (n_bins_for_feature, n_classes)
    """
    n = len(train_y)
    counts = np.bincount(train_y, minlength=n_classes).astype(float)
    prior = (counts + alpha) / (n + alpha * n_classes)

    likelihoods = {}
    for col in train_binned.columns:
        x = train_binned[col].to_numpy()
        n_bins_col = int(x.max()) + 1
        like = np.full((n_bins_col, n_classes), alpha, dtype=float)
        for xi, yi in zip(x, train_y):
            like[xi, yi] += 1.0
        like = like / like.sum(axis=0, keepdims=True)
        likelihoods[col] = like
    return prior, likelihoods


def predict_naive_bayes(test_binned, prior, likelihoods, n_classes=5):
    log_prior = np.log(prior + 1e-12)
    posteriors = []
    for _, row in test_binned.iterrows():
        log_p = log_prior.copy()
        for col, like in likelihoods.items():
            xi = int(row[col])
            xi = max(0, min(xi, like.shape[0] - 1))
            log_p = log_p + np.log(like[xi] + 1e-12)
        # Normalize
        m = log_p.max()
        p = np.exp(log_p - m)
        p = p / p.sum()
        posteriors.append(p)
    return np.array(posteriors)


def macro_f1(y_true, y_pred, n_classes=5):
    f1s = []
    for cls in range(n_classes):
        tp = ((y_pred == cls) & (y_true == cls)).sum()
        fp = ((y_pred == cls) & (y_true != cls)).sum()
        fn = ((y_pred != cls) & (y_true == cls)).sum()
        if tp + fp == 0 or tp + fn == 0:
            f1s.append(0.0)
            continue
        p = tp / (tp + fp)
        r = tp / (tp + fn)
        f1s.append(2 * p * r / (p + r) if (p + r) > 0 else 0.0)
    return float(np.mean(f1s)), f1s


def main() -> int:
    df = build_features()
    print(f"Total samples: {len(df)} ({df.index.min().date()} -> {df.index.max().date()})")
    train = df.loc[df.index < TRAIN_END]
    test = df.loc[df.index >= TRAIN_END]
    print(f"Train: {len(train)} ({train.index.min().date()} -> {train.index.max().date()})")
    print(f"Test:  {len(test)} ({test.index.min().date()} -> {test.index.max().date()})")
    print()

    print("Train outcome distribution:")
    for cls, lbl in enumerate(CLASS_NAMES):
        n = (train["outcome"] == cls).sum()
        print(f"  {lbl:10s}: {n:>5d} ({n/len(train)*100:>5.1f}%)")
    print()

    edges = discretize_train(train.drop(columns=["outcome"]))
    train_binned = apply_bins(train.drop(columns=["outcome"]), edges)
    test_binned = apply_bins(test.drop(columns=["outcome"]), edges)
    train_y = train["outcome"].astype(int).to_numpy()
    test_y = test["outcome"].astype(int).to_numpy()

    prior, likelihoods = fit_naive_bayes(train_binned, train_y)
    print(f"Train prior P(class):")
    for cls, lbl in enumerate(CLASS_NAMES):
        print(f"  {lbl:10s}: {prior[cls]:.4f}")
    print()

    train_post = predict_naive_bayes(train_binned, prior, likelihoods)
    train_pred = train_post.argmax(axis=1)
    train_f1, train_f1s = macro_f1(train_y, train_pred)
    train_acc = (train_pred == train_y).mean()

    test_post = predict_naive_bayes(test_binned, prior, likelihoods)
    test_pred = test_post.argmax(axis=1)
    test_f1, test_f1s = macro_f1(test_y, test_pred)
    test_acc = (test_pred == test_y).mean()

    print("=" * 75)
    print("BBN Naive Bayes regime classifier — top-5 evidence features")
    print("=" * 75)
    print(f"{'split':>10s}{'samples':>10s}{'accuracy':>12s}{'macro_F1':>12s}")
    print("-" * 75)
    print(f"{'TRAIN':>10s}{len(train):>10d}{train_acc:>12.4f}{train_f1:>12.4f}")
    print(f"{'TEST':>10s}{len(test):>10d}{test_acc:>12.4f}{test_f1:>12.4f}")
    print()
    print("Per-class F1 (TEST):")
    for cls, lbl in enumerate(CLASS_NAMES):
        n_true = (test_y == cls).sum()
        n_pred = (test_pred == cls).sum()
        print(f"  {lbl:10s}: F1={test_f1s[cls]:.4f} (true={n_true}, pred={n_pred})")
    print()

    print("Confusion matrix (TEST, rows=true, cols=pred):")
    cm = np.zeros((5, 5), dtype=int)
    for t, p in zip(test_y, test_pred):
        cm[t, p] += 1
    header = "         " + "".join(f"{n[:6]:>8s}" for n in CLASS_NAMES)
    print(header)
    for i, lbl in enumerate(CLASS_NAMES):
        row = f"{lbl[:8]:>8s} " + "".join(f"{cm[i,j]:>8d}" for j in range(5))
        print(row)
    print()

    print("Reference points:")
    print(f"  uniform baseline (5-class):       0.2000")
    print(f"  current 4-class classifier:    ~0.2800-0.3100")
    print(f"  Slice 117 in-sample lookup:       0.4900")
    print(f"  THIS classifier (TEST OOS):       {test_f1:.4f}")
    print()

    if test_f1 > 0.31:
        print(f"VERDICT: BBN Naive Bayes classifier IS BETTER than existing 4-class.")
        print(f"  Lift: {test_f1/0.30:.2f}x over baseline 0.30")
    elif test_f1 > 0.24:
        print(f"VERDICT: BBN Naive Bayes classifier is COMPARABLE to existing 4-class.")
    else:
        print(f"VERDICT: BBN Naive Bayes classifier is WORSE than existing — overfitting likely.")
    print()

    # Save predictions for Slice 119+ VRP integration
    out = pd.DataFrame(test_post, index=test.index, columns=[f"p_{c}" for c in CLASS_NAMES])
    out["pred_class"] = test_pred
    out["true_class"] = test_y
    out_path = PROBE_DIR / "slice_118_bbn_predictions.csv"
    out.to_csv(out_path)
    print(f"Saved test predictions to {out_path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
