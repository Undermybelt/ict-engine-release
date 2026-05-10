"""
pandas_bbn_evidence_ranking.py — Slice 117. Rank BBN-style evidence
variables by mutual information with forward 20d NQ realized outcome
regime. Goal: identify the most informative evidence nodes for the next
regime classifier upgrade (the current 4-class NQ-200d+VIX classifier
has macro_f1 ~0.28-0.31 which is barely actionable).

Forward outcome regime (5-class):
- crash:      future_20d_return < -8%
- down:       -8% <= future_20d_return < -1%
- flat:       -1% <= future_20d_return < +1%
- up:         +1% <= future_20d_return < +8%
- strong_up:  future_20d_return >= +8%

Evidence variables (all daily, from /tmp/ict-engine-ibkr-probe/):
- vol levels: vix, vix9d, vix3m, vvix, vxn, rvx, gvz, ovx
- vol percentile-ranks (252d): each of the above
- vol ratios: vix9d/vix3m (term structure), vvix/vix, vxn/vix
- price-derived: nq_drawdown_pct (vs 200d high), nq_vs_200d_pct
- changes: vix_5d_chg, term_ratio_5d_chg

Output: ranked table of MI(evidence, forward_outcome) for each variable.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

NQ_15M = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")

EVIDENCE_CSVS = {
    "vix": "vix.1d.10y.csv",
    "vix9d": "vix9d.1d.10y.csv",
    "vix3m": "vix3m.1d.10y.csv",
    "vvix": "vvix.1d.10y.csv",
    "vxn": "vxn.1d.10y.csv",
    "rvx": "rvx.1d.10y.csv",
    "gvz": "gvz.1d.10y.csv",
    "ovx": "ovx.1d.10y.csv",
    "qqq_iv": "qqq.iv.1d.10y.csv",
    "qqq_hv": "qqq.hv.1d.10y.csv",
}

START = pd.Timestamp("2018-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
FORWARD_DAYS = 20


def load_close(csv_path):
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def label_outcome(future_ret):
    if future_ret < -0.08:
        return 0  # crash
    if future_ret < -0.01:
        return 1  # down
    if future_ret < 0.01:
        return 2  # flat
    if future_ret < 0.08:
        return 3  # up
    return 4  # strong_up


def main() -> int:
    # NQ daily close from 15m
    df15 = pd.read_feather(NQ_15M)
    df15["date"] = pd.to_datetime(df15["date"], unit="ms", utc=True)
    df15 = df15.set_index("date").sort_index().loc[START:END]
    nq_daily = df15["close"].resample("1D").last().dropna()
    nq_daily.index = nq_daily.index.normalize()

    # Forward 20d return
    future_ret = nq_daily.shift(-FORWARD_DAYS) / nq_daily - 1.0
    outcome = future_ret.apply(label_outcome)

    print(f"NQ daily bars: {len(nq_daily)}")
    print(f"Outcome distribution (5-class forward 20d):")
    counts = outcome.value_counts().sort_index()
    labels = ["crash", "down", "flat", "up", "strong_up"]
    for cls, lbl in enumerate(labels):
        n = counts.get(cls, 0)
        pct = n / len(outcome) * 100 if len(outcome) > 0 else 0
        print(f"  {lbl:10s}: {n:>5d} ({pct:>5.1f}%)")
    print()

    # Build evidence dataframe
    evidence = pd.DataFrame(index=nq_daily.index)

    for name, csv in EVIDENCE_CSVS.items():
        s = load_close(PROBE_DIR / csv)
        evidence[f"{name}_level"] = s.reindex(nq_daily.index).ffill()
        evidence[f"{name}_pct_rank_252"] = s.rolling(252, min_periods=128).rank(pct=True).reindex(nq_daily.index).ffill()
        evidence[f"{name}_5d_chg"] = (s / s.shift(5) - 1.0).reindex(nq_daily.index).ffill()

    # Vol ratios
    vix = load_close(PROBE_DIR / EVIDENCE_CSVS["vix"]).reindex(nq_daily.index).ffill()
    vix9d = load_close(PROBE_DIR / EVIDENCE_CSVS["vix9d"]).reindex(nq_daily.index).ffill()
    vix3m = load_close(PROBE_DIR / EVIDENCE_CSVS["vix3m"]).reindex(nq_daily.index).ffill()
    vvix = load_close(PROBE_DIR / EVIDENCE_CSVS["vvix"]).reindex(nq_daily.index).ffill()
    vxn = load_close(PROBE_DIR / EVIDENCE_CSVS["vxn"]).reindex(nq_daily.index).ffill()
    evidence["term_vix9d_vix3m"] = vix9d / vix3m.where(vix3m > 1e-9)
    evidence["term_vix9d_vix"] = vix9d / vix.where(vix > 1e-9)
    evidence["vvix_over_vix"] = vvix / vix.where(vix > 1e-9)
    evidence["vxn_over_vix"] = vxn / vix.where(vix > 1e-9)
    evidence["iv_minus_hv_pct"] = (
        load_close(PROBE_DIR / EVIDENCE_CSVS["qqq_iv"]).reindex(nq_daily.index).ffill()
        - load_close(PROBE_DIR / EVIDENCE_CSVS["qqq_hv"]).reindex(nq_daily.index).ffill()
    ) / load_close(PROBE_DIR / EVIDENCE_CSVS["qqq_iv"]).reindex(nq_daily.index).ffill()

    # NQ-derived
    nq200 = nq_daily.rolling(200, min_periods=100).max()
    evidence["nq_drawdown_pct"] = (nq_daily / nq200 - 1.0)
    sma200 = nq_daily.rolling(200, min_periods=100).mean()
    evidence["nq_vs_200d_pct"] = (nq_daily / sma200 - 1.0)

    # Align
    valid = evidence.dropna()
    valid_outcome = outcome.reindex(valid.index).dropna()
    valid = valid.loc[valid_outcome.index]
    print(f"Valid samples (after dropna + forward-window): {len(valid)}")
    print()

    # Mutual information per feature (manual, no sklearn)
    print("Computing MI for each evidence variable vs forward 20d outcome...")
    y = valid_outcome.astype(int).to_numpy()
    n = len(y)
    py = np.bincount(y, minlength=5).astype(float) / n

    mi_dict = {}
    n_bins = 8
    for col in valid.columns:
        x = valid[col].to_numpy()
        try:
            x_bins = pd.qcut(x, q=n_bins, labels=False, duplicates="drop")
        except Exception:
            mi_dict[col] = 0.0
            continue
        x_bins = np.asarray(x_bins)
        valid_mask = ~np.isnan(x_bins.astype(float))
        if valid_mask.sum() < 50:
            mi_dict[col] = 0.0
            continue
        xb = x_bins[valid_mask].astype(int)
        yb = y[valid_mask]
        n_eff = len(yb)
        n_x = xb.max() + 1
        joint = np.zeros((n_x, 5))
        for xi, yi in zip(xb, yb):
            joint[xi, yi] += 1
        joint /= n_eff
        px = joint.sum(axis=1)
        pyc = joint.sum(axis=0)
        mi = 0.0
        for i in range(n_x):
            for j in range(5):
                if joint[i, j] > 0 and px[i] > 0 and pyc[j] > 0:
                    mi += joint[i, j] * np.log(joint[i, j] / (px[i] * pyc[j]))
        mi_dict[col] = float(mi)
    mi_series = pd.Series(mi_dict).sort_values(ascending=False)

    print()
    print("=" * 70)
    print("BBN evidence variables — mutual information with forward 20d NQ outcome")
    print("=" * 70)
    print(f"{'rank':>5s}{'evidence':45s}{'MI(nats)':>12s}")
    print("-" * 70)
    for rank, (name, val) in enumerate(mi_series.items(), 1):
        print(f"{rank:>5d}  {name:43s}{val:>12.5f}")
    print()

    # Top 3 evidence variables
    top3 = mi_series.head(3)
    print(f"TOP 3 BBN evidence nodes (highest information about forward outcome):")
    for name, val in top3.items():
        print(f"  {name}: MI={val:.5f} nats")
    print()

    # Quick classification baseline using top-3 vs full feature set with manual decision rule
    print("Predictive sanity: top-K evidence -> 5-class accuracy (most-frequent-class within feature bin):")
    for k in [3, 5, len(valid.columns)]:
        feat_names = mi_series.head(k).index.tolist()
        # Build a simple multi-feature bin lookup
        binned = pd.DataFrame(index=valid.index)
        for f in feat_names:
            try:
                binned[f] = pd.qcut(valid[f], q=4, labels=False, duplicates="drop")
            except Exception:
                binned[f] = 0
        binned = binned.fillna(0).astype(int)
        binned["__outcome__"] = y
        # Compute most-frequent class per bin combination
        groups = binned.groupby(feat_names)["__outcome__"]
        most_freq = groups.agg(lambda s: s.value_counts().idxmax())
        binned["pred"] = binned[feat_names].apply(
            lambda r: most_freq.get(tuple(r), 2), axis=1
        )
        acc = (binned["pred"] == y).mean()
        # Macro-F1
        f1s = []
        for cls in range(5):
            tp = ((binned["pred"] == cls) & (y == cls)).sum()
            fp = ((binned["pred"] == cls) & (y != cls)).sum()
            fn = ((binned["pred"] != cls) & (y == cls)).sum()
            if tp + fp == 0 or tp + fn == 0:
                f1 = 0.0
            else:
                p = tp / (tp + fp)
                r = tp / (tp + fn)
                f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0.0
            f1s.append(f1)
        macro_f1 = np.mean(f1s)
        print(f"  top-{k:>3d} ({len(feat_names)} features): accuracy={acc:.4f}, "
              f"macro_f1={macro_f1:.4f}")
    print(f"(baseline uniform 5-class = 0.20; current 4-class regime classifier macro_f1 ≈ 0.28-0.31)")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
