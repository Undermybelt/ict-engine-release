"""
pandas_bbn_vol_regime.py — Slice 125. Build a BBN classifier with
forward VOL-REGIME target instead of forward DIRECTION target.

Direction is intrinsically hard (Slice 118: macro_F1 0.20-0.26 OOS).
Vol persists strongly — vol-regime classification should achieve much
higher OOS accuracy.

Target definition: forward 20d max VIX percentile (252d window), binned:
- very_low:  pct < 0.20
- low:       0.20 <= pct < 0.40
- medium:    0.40 <= pct < 0.65
- high:      pct >= 0.65

Features: same top-5 from Slice 117 (qqq_hv_level, nq_vs_200d_pct,
vix3m_level, qqq_hv_pct_rank_252, vvix_over_vix). All vol-state
indicators — should map cleanly to forward vol regime.

Train: 2016-10 to 2019-12
Test:  2020-01 to 2025-12

Comparison:
- BBN direction target (Slice 118/123): OOS macro_F1 ~0.20
- BBN vol-regime target (this slice):   target ≥0.40 macro_F1
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

NQ_15M = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather")
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
N_BINS = 6
LAPLACE_ALPHA = 0.5
CLASS_NAMES = ["very_low", "low", "medium", "high"]


def load_close(csv_path):
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def label_vol_regime(future_max_pct):
    if future_max_pct < 0.20: return 0  # very_low
    if future_max_pct < 0.40: return 1  # low
    if future_max_pct < 0.65: return 2  # medium
    return 3                              # high


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

    # Forward target: MEAN VIX percentile in next 20 days (less event-dominated than MAX)
    vix_pct_252 = vix.rolling(252, min_periods=128).rank(pct=True)
    rolled = vix_pct_252.rolling(FORWARD_DAYS, min_periods=10).mean()
    fwd_mean = rolled.shift(-FORWARD_DAYS)
    outcome = fwd_mean.apply(label_vol_regime)

    df = feats.copy()
    df["outcome"] = outcome
    df = df.dropna()
    return df


def discretize(train_df, n_bins=N_BINS):
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
    out = pd.DataFrame(index=df.index)
    for col, e in edges.items():
        out[col] = pd.cut(df[col], bins=e, labels=False, include_lowest=True)
    return out.fillna(0).astype(int)


def fit_naive_bayes(train_binned, train_y, n_classes=4, alpha=LAPLACE_ALPHA):
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


def predict_naive_bayes(test_binned, prior, likelihoods, n_classes=4):
    log_prior = np.log(prior + 1e-12)
    posteriors = []
    for _, row in test_binned.iterrows():
        log_p = log_prior.copy()
        for col, like in likelihoods.items():
            xi = int(row[col])
            xi = max(0, min(xi, like.shape[0] - 1))
            log_p = log_p + np.log(like[xi] + 1e-12)
        m = log_p.max()
        p = np.exp(log_p - m); p = p / p.sum()
        posteriors.append(p)
    return np.array(posteriors)


def macro_f1(y_true, y_pred, n_classes=4):
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


def main() -> int:
    df = build_features()
    print(f"Total: {len(df)} samples ({df.index.min().date()} -> {df.index.max().date()})")
    train = df.loc[df.index <= BBN_TRAIN_END]
    test = df.loc[df.index >= TEST_START]
    print(f"Train: {len(train)} ({train.index.min().date()} -> {train.index.max().date()})")
    print(f"Test:  {len(test)} ({test.index.min().date()} -> {test.index.max().date()})")
    print()
    print(f"Train vol-regime distribution:")
    for cls, lbl in enumerate(CLASS_NAMES):
        n = (train["outcome"] == cls).sum()
        pct = n / len(train) * 100 if len(train) > 0 else 0
        print(f"  {lbl:10s}: {n:>5d} ({pct:>5.1f}%)")
    print()
    print(f"Test vol-regime distribution:")
    for cls, lbl in enumerate(CLASS_NAMES):
        n = (test["outcome"] == cls).sum()
        pct = n / len(test) * 100 if len(test) > 0 else 0
        print(f"  {lbl:10s}: {n:>5d} ({pct:>5.1f}%)")
    print()

    edges = discretize(train.drop(columns=["outcome"]))
    train_binned = apply_bins(train.drop(columns=["outcome"]), edges)
    test_binned = apply_bins(test.drop(columns=["outcome"]), edges)
    train_y = train["outcome"].astype(int).to_numpy()
    test_y = test["outcome"].astype(int).to_numpy()

    prior, likelihoods = fit_naive_bayes(train_binned, train_y)
    train_post = predict_naive_bayes(train_binned, prior, likelihoods)
    train_pred = train_post.argmax(axis=1)
    test_post = predict_naive_bayes(test_binned, prior, likelihoods)
    test_pred = test_post.argmax(axis=1)

    train_f1, train_f1s = macro_f1(train_y, train_pred)
    test_f1, test_f1s = macro_f1(test_y, test_pred)
    train_acc = (train_pred == train_y).mean()
    test_acc = (test_pred == test_y).mean()

    print("=" * 75)
    print("BBN VOL-REGIME classifier (forward 20d max VIX percentile, 4 classes)")
    print("=" * 75)
    print(f"{'split':>10s}{'samples':>10s}{'accuracy':>12s}{'macro_F1':>12s}")
    print("-" * 75)
    print(f"{'TRAIN':>10s}{len(train):>10d}{train_acc:>12.4f}{train_f1:>12.4f}")
    print(f"{'TEST':>10s}{len(test):>10d}{test_acc:>12.4f}{test_f1:>12.4f}")
    print()
    print(f"Per-class TEST F1:")
    for cls, lbl in enumerate(CLASS_NAMES):
        n_true = (test_y == cls).sum()
        n_pred = (test_pred == cls).sum()
        print(f"  {lbl:10s}: F1={test_f1s[cls]:.4f} (true={n_true}, pred={n_pred})")
    print()

    cm = np.zeros((4, 4), dtype=int)
    for t, p in zip(test_y, test_pred):
        cm[t, p] += 1
    print(f"TEST confusion matrix (rows=true, cols=pred):")
    header = "          " + "".join(f"{n[:8]:>10s}" for n in CLASS_NAMES)
    print(header)
    for i, lbl in enumerate(CLASS_NAMES):
        row = f"{lbl[:8]:>8s}  " + "".join(f"{cm[i,j]:>10d}" for j in range(4))
        print(row)
    print()

    print("Reference points:")
    print(f"  uniform 4-class baseline:                    0.2500")
    print(f"  BBN DIRECTION classifier (Slice 123, 6Y):   ~0.1967")
    print(f"  THIS BBN VOL-REGIME classifier (TEST OOS):    {test_f1:.4f}")
    print()

    if test_f1 > 0.40:
        print(f"VERDICT: vol-regime classifier is MATERIALLY BETTER than direction.")
        print(f"  Lift over direction classifier: {test_f1/0.20:.2f}x")
        print(f"  This addresses the user's 'regime判断更对' preference concretely.")
    elif test_f1 > 0.30:
        print(f"VERDICT: vol-regime classifier is BETTER than direction (modest lift).")
    else:
        print(f"VERDICT: vol-regime classifier comparable to direction. Vol classification harder than expected.")

    # Save predictions for next-slice VRP integration
    out = pd.DataFrame(test_post, index=test.index, columns=[f"vp_{c}" for c in CLASS_NAMES])
    out["vol_pred_class"] = test_pred
    out["vol_true_class"] = test_y
    out_path = PROBE_DIR / "slice_125_vol_regime_predictions.csv"
    out.to_csv(out_path)
    print(f"Saved test predictions to {out_path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
