"""
pandas_decision_tree_regime.py — Slice 126. Test whether the regime
classification ceiling (0.20-0.25 macro_F1 OOS) is due to the Naive
Bayes model's broken independence assumption, by replacing it with a
hand-rolled decision tree.

Decision trees handle correlated features naturally — each split picks
the single most-informative feature given the current node, then
recursive partitions explore conditional structure.

Same task as Slice 118/123: forward-20d 5-class direction outcome
(crash/down/flat/up/strong_up). Same features (top-5 from Slice 117).
Same train/test boundaries (2016-2019 / 2020-2025).

If decision tree macro_F1 > 0.30 OOS: model class WAS the bottleneck;
opens path to improving regime classification further.
If decision tree ≈ Naive Bayes: structural ceiling is real; need
different evidence categories, not different model.
"""
from __future__ import annotations

import sys
from pathlib import Path
from dataclasses import dataclass

import numpy as np
import pandas as pd

NQ_15M = Path("user_data/data/NQ_USD-15m.feather")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
VIX3M_CSV = PROBE_DIR / "vix3m.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
VIX_CSV = PROBE_DIR / "vix.1d.10y.csv"

BBN_TRAIN_START = pd.Timestamp("2016-10-01", tz="UTC")
BBN_TRAIN_END = pd.Timestamp("2019-12-31", tz="UTC")
TEST_START = pd.Timestamp("2020-01-01", tz="UTC")
TEST_END = pd.Timestamp("2025-12-31", tz="UTC")
FORWARD_DAYS = 20
N_CLASSES = 5
CLASS_NAMES = ["crash", "down", "flat", "up", "strong_up"]
MAX_DEPTH = 4
MIN_LEAF = 20


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
    df15 = df15.set_index("date").sort_index().loc[BBN_TRAIN_START:TEST_END]
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
    df = feats.copy(); df["outcome"] = outcome
    return df.dropna()


def gini(y, n_classes=N_CLASSES):
    if len(y) == 0:
        return 0.0
    counts = np.bincount(y, minlength=n_classes).astype(float)
    p = counts / len(y)
    return 1.0 - np.sum(p ** 2)


def gini_split(y_left, y_right):
    n = len(y_left) + len(y_right)
    if n == 0:
        return 0.0
    g_left = gini(y_left)
    g_right = gini(y_right)
    return (len(y_left) / n) * g_left + (len(y_right) / n) * g_right


def best_split(X, y, feature_names):
    """Find best (feature, threshold) for binary split."""
    best_gain = 0.0
    best_split_info = None
    parent_gini = gini(y)
    for f_idx, f_name in enumerate(feature_names):
        x = X[:, f_idx]
        # Try ~10 quantile thresholds
        thresholds = np.unique(np.quantile(x, np.linspace(0.1, 0.9, 9)))
        for thr in thresholds:
            left_mask = x <= thr
            right_mask = ~left_mask
            if left_mask.sum() < MIN_LEAF or right_mask.sum() < MIN_LEAF:
                continue
            g_split = gini_split(y[left_mask], y[right_mask])
            gain = parent_gini - g_split
            if gain > best_gain:
                best_gain = gain
                best_split_info = (f_idx, thr, gain)
    return best_split_info


@dataclass
class Node:
    is_leaf: bool
    prediction: int = 0
    class_probs: tuple = ()
    feature_idx: int = -1
    threshold: float = 0.0
    left: "Node | None" = None
    right: "Node | None" = None
    n_samples: int = 0


def build_tree(X, y, feature_names, depth=0, max_depth=MAX_DEPTH):
    n = len(y)
    counts = np.bincount(y, minlength=N_CLASSES).astype(float)
    probs = counts / max(n, 1)
    pred = int(np.argmax(counts))
    if depth >= max_depth or n < 2 * MIN_LEAF or gini(y) < 0.01:
        return Node(is_leaf=True, prediction=pred, class_probs=tuple(probs.tolist()), n_samples=n)
    split = best_split(X, y, feature_names)
    if split is None:
        return Node(is_leaf=True, prediction=pred, class_probs=tuple(probs.tolist()), n_samples=n)
    f_idx, thr, gain = split
    left_mask = X[:, f_idx] <= thr
    right_mask = ~left_mask
    left = build_tree(X[left_mask], y[left_mask], feature_names, depth + 1, max_depth)
    right = build_tree(X[right_mask], y[right_mask], feature_names, depth + 1, max_depth)
    return Node(
        is_leaf=False, prediction=pred, class_probs=tuple(probs.tolist()),
        feature_idx=f_idx, threshold=thr, left=left, right=right, n_samples=n,
    )


def predict_one(node, x):
    while not node.is_leaf:
        if x[node.feature_idx] <= node.threshold:
            node = node.left
        else:
            node = node.right
    return node.prediction, node.class_probs


def predict_batch(node, X):
    n = X.shape[0]
    preds = np.zeros(n, dtype=int)
    probs = np.zeros((n, N_CLASSES))
    for i in range(n):
        p, pr = predict_one(node, X[i])
        preds[i] = p
        probs[i] = pr
    return preds, probs


def macro_f1(y_true, y_pred, n_classes=N_CLASSES):
    f1s = []
    for cls in range(n_classes):
        tp = ((y_pred == cls) & (y_true == cls)).sum()
        fp = ((y_pred == cls) & (y_true != cls)).sum()
        fn = ((y_pred != cls) & (y_true == cls)).sum()
        if tp + fp == 0 or tp + fn == 0:
            f1s.append(0.0); continue
        p = tp / (tp + fp); r = tp / (tp + fn)
        f1s.append(2*p*r/(p+r) if (p+r) > 0 else 0.0)
    return float(np.mean(f1s)), f1s


def print_tree(node, feature_names, depth=0, side=""):
    indent = "  " * depth
    if node.is_leaf:
        probs_str = ", ".join(f"{p:.2f}" for p in node.class_probs)
        print(f"{indent}{side}leaf: pred={CLASS_NAMES[node.prediction]} "
              f"(probs=[{probs_str}], n={node.n_samples})")
    else:
        print(f"{indent}{side}{feature_names[node.feature_idx]} <= {node.threshold:.4f} "
              f"(n={node.n_samples})")
        print_tree(node.left, feature_names, depth + 1, "├L: ")
        print_tree(node.right, feature_names, depth + 1, "└R: ")


def main() -> int:
    df = build_features()
    train = df.loc[df.index <= BBN_TRAIN_END]
    test = df.loc[df.index >= TEST_START]
    print(f"Train: {len(train)} samples ({train.index.min().date()} -> {train.index.max().date()})")
    print(f"Test:  {len(test)} samples ({test.index.min().date()} -> {test.index.max().date()})")
    print()

    feature_names = ["qqq_hv_level", "nq_vs_200d_pct", "vix3m_level",
                     "qqq_hv_pct_rank_252", "vvix_over_vix"]
    X_train = train[feature_names].to_numpy()
    y_train = train["outcome"].astype(int).to_numpy()
    X_test = test[feature_names].to_numpy()
    y_test = test["outcome"].astype(int).to_numpy()

    print(f"Building decision tree (max_depth={MAX_DEPTH}, min_leaf={MIN_LEAF})...")
    tree = build_tree(X_train, y_train, feature_names)
    print()
    print("Tree structure:")
    print_tree(tree, feature_names)
    print()

    train_pred, _ = predict_batch(tree, X_train)
    test_pred, test_probs = predict_batch(tree, X_test)
    train_f1, _ = macro_f1(y_train, train_pred)
    test_f1, test_f1s = macro_f1(y_test, test_pred)
    train_acc = (train_pred == y_train).mean()
    test_acc = (test_pred == y_test).mean()

    print("=" * 75)
    print("Decision Tree regime classifier (forward-20d direction, 5-class)")
    print("=" * 75)
    print(f"{'split':>10s}{'samples':>10s}{'accuracy':>12s}{'macro_F1':>12s}")
    print("-" * 75)
    print(f"{'TRAIN':>10s}{len(y_train):>10d}{train_acc:>12.4f}{train_f1:>12.4f}")
    print(f"{'TEST':>10s}{len(y_test):>10d}{test_acc:>12.4f}{test_f1:>12.4f}")
    print()

    print("Per-class TEST F1:")
    for cls, lbl in enumerate(CLASS_NAMES):
        n_true = (y_test == cls).sum()
        n_pred = (test_pred == cls).sum()
        print(f"  {lbl:10s}: F1={test_f1s[cls]:.4f} (true={n_true}, pred={n_pred})")
    print()

    print("Reference points:")
    print(f"  uniform 5-class baseline:                    0.2000")
    print(f"  Naive Bayes direction (Slice 123, 6Y OOS):   0.1967")
    print(f"  Naive Bayes vol-regime (Slice 125, OOS):     0.2523")
    print(f"  THIS Decision Tree (TEST OOS):               {test_f1:.4f}")
    print()

    if test_f1 > 0.30:
        print("VERDICT: Decision Tree MATERIALLY OUTPERFORMS Naive Bayes.")
        print("  → Model class WAS the bottleneck. Naive Bayes' independence assumption was the limit.")
    elif test_f1 > 0.25:
        print("VERDICT: Decision Tree MODESTLY outperforms Naive Bayes.")
        print("  → Some lift from non-naive model, but not transformative.")
    elif test_f1 >= 0.18:
        print("VERDICT: Decision Tree COMPARABLE to Naive Bayes.")
        print("  → Structural ceiling confirmed. Need new evidence categories, not new model.")
    else:
        print("VERDICT: Decision Tree UNDERPERFORMS Naive Bayes (overfit).")

    out = pd.DataFrame(test_probs, index=test.index, columns=[f"dt_p_{c}" for c in CLASS_NAMES])
    out["dt_pred_class"] = test_pred
    out["true_class"] = y_test
    out_path = PROBE_DIR / "slice_126_decision_tree_predictions.csv"
    out.to_csv(out_path)
    print(f"\nSaved predictions to {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
