"""
pandas_vrp_v25_long_oos.py — Slice 123. Extended OOS validation of
VRP V2.5b (V2 + BBN bull_score (0.6, 0.8] deny filter) over 6 years
including COVID, 2022 bear, and 2024-2025 recovery — much harder than
the Slice 122 3-year test.

Train BBN on 2016-05 to 2019-12 (~3.5 years, captures pre-COVID regime
including 2018 vol-spike and 2018 sell-off).
Test on 2020-01 to 2025-12 (~6 years).

Compare:
- V2 baseline on 2020-2025
- V2.5b (deny bull_score (0.6, 0.8]) on 2020-2025
- V2.5d (only pred_class in {flat, down}) on 2020-2025

Walk-forward distribution: 6M windows from 2020H1 to 2025H2 (12 windows).

This is the cleanest OOS test we can run without rolling-window BBN
training: BBN never sees test-period data, V2.5b filter is applied as
true OOS soft conditioning.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

NQ_5M = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-5m.feather")
NQ_15M = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather")
NQ_1H = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-1h.feather")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
QQQ_IV_CSV = PROBE_DIR / "qqq.iv.1d.10y.csv"
VIX3M_CSV = PROBE_DIR / "vix3m.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
VIX_CSV = PROBE_DIR / "vix.1d.10y.csv"

BBN_TRAIN_START = pd.Timestamp("2016-05-09", tz="UTC")
BBN_TRAIN_END = pd.Timestamp("2019-12-31", tz="UTC")
TEST_START = pd.Timestamp("2020-01-01", tz="UTC")
TEST_END = pd.Timestamp("2025-12-31", tz="UTC")
FORWARD_DAYS = 20
N_BINS = 6
LAPLACE_ALPHA = 0.5
TRADING_DAYS = 252.0
STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005
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


def build_daily_features():
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
    df = feats.copy()
    df["outcome"] = outcome
    df = df.dropna()
    return df, nq_daily


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


def fit_naive_bayes(train_binned, train_y, n_classes=5, alpha=LAPLACE_ALPHA):
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
        m = log_p.max()
        p = np.exp(log_p - m); p = p / p.sum()
        posteriors.append(p)
    return np.array(posteriors)


def build_intraday_with_bbn(feather_path, bbn_post_df):
    df = pd.read_feather(feather_path)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index().loc[TEST_START:TEST_END]
    df["ema21"] = df["close"].ewm(span=21, adjust=False).mean()
    df["ema89"] = df["close"].ewm(span=89, adjust=False).mean()
    df["ema200"] = df["close"].ewm(span=200, adjust=False).mean()
    df["ema600"] = df["close"].ewm(span=600, adjust=False).mean()
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour

    iv = load_close(QQQ_IV_CSV); hv = load_close(QQQ_HV_CSV); vvix = load_close(VVIX_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)
    cd = df.index.normalize()
    df["iv_pct_rank_252"] = pd.Series(cd.map(iv_pr), index=df.index).ffill()
    df["hv_pct_rank_252"] = pd.Series(cd.map(hv_pr), index=df.index).ffill()
    df["vvix_pct_rank_252"] = pd.Series(cd.map(vvix_pr), index=df.index).ffill()

    df["liquid_window"] = (df["hour_utc"] >= 13) & (df["hour_utc"] <= 21)
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])

    for col in bbn_post_df.columns:
        df[col] = pd.Series(cd.map(bbn_post_df[col]), index=df.index).ffill()
    df["bull_score"] = df["p_up"] + df["p_strong_up"]

    df["v2_entry"] = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & (df["iv_pct_rank_252"] < 0.30)
        & (df["hv_pct_rank_252"] < 0.30)
        & (df["vvix_pct_rank_252"] < 0.40)
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    df["exit_signal"] = (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])
    return df


def simulate(df, entry_filter):
    closes = df["close"].to_numpy(); highs = df["high"].to_numpy(); lows = df["low"].to_numpy()
    es = entry_filter.to_numpy(); xs = df["exit_signal"].to_numpy()
    ts = df.index.to_numpy()
    trades = []; in_pos = False; entry_idx = -1; entry_price = 0.0; peak = 0.0; trail = False
    for i in range(len(df)):
        if not in_pos:
            if es[i]:
                in_pos = True; entry_idx = i; entry_price = closes[i]; peak = closes[i]; trail = False
            continue
        peak = max(peak, highs[i])
        if not trail and (peak / entry_price - 1.0) >= TRAILING_OFFSET:
            trail = True
        sl = entry_price * (1.0 + STOPLOSS)
        tp = peak * (1.0 - TRAILING_STOP) if trail else 0.0
        eff = max(sl, tp)
        reason = None; exit_price = closes[i]
        if lows[i] <= eff:
            reason = "stop"; exit_price = eff
        elif xs[i]:
            reason = "exit"
        if reason is not None:
            trades.append({"open_date": pd.Timestamp(ts[entry_idx]),
                           "close_date": pd.Timestamp(ts[i]),
                           "profit_ratio": exit_price / entry_price - 1.0})
            in_pos = False; entry_idx = -1; entry_price = 0.0; peak = 0.0; trail = False
    return pd.DataFrame(trades)


def daily_pnl(t):
    if t.empty: return pd.Series(dtype=float)
    s = t.copy(); s["date"] = s["close_date"].dt.normalize()
    return s.groupby("date")["profit_ratio"].sum().sort_index()


def annual_metrics(d):
    if d.empty or d.std() == 0: return {"sharpe": 0.0, "max_dd": 0.0, "total": 0.0}
    sharpe = (d.mean() / d.std()) * np.sqrt(TRADING_DAYS) if d.std() > 0 else 0.0
    cum = (1.0 + d).cumprod(); dd = float((cum / cum.cummax() - 1.0).min())
    return {"sharpe": float(sharpe), "max_dd": dd, "total": float(cum.iloc[-1] - 1.0)}


def walk_forward(trades):
    if trades.empty: return None
    cur = pd.Timestamp("2020-01-01", tz="UTC"); end = pd.Timestamp("2026-01-01", tz="UTC")
    sharpes = []
    while cur < end:
        nxt = cur + pd.DateOffset(months=6)
        wt = trades[(trades["open_date"] >= cur) & (trades["open_date"] < nxt)]
        if len(wt) >= 5:
            m = annual_metrics(daily_pnl(wt))
            sharpes.append(m["sharpe"])
        cur = nxt
    if not sharpes: return None
    return {
        "wf_med": float(np.median(sharpes)),
        "wf_pos": float(sum(1 for s in sharpes if s > 0) / len(sharpes)),
        "wf_n": len(sharpes),
        "wf_min": float(min(sharpes)),
        "wf_max": float(max(sharpes)),
    }


def main() -> int:
    print("Building BBN training/test feature set...")
    df, _ = build_daily_features()
    train = df.loc[df.index <= BBN_TRAIN_END]
    test = df.loc[(df.index >= TEST_START) & (df.index <= TEST_END)]
    print(f"BBN train: {len(train)} samples ({train.index.min().date()} -> {train.index.max().date()})")
    print(f"BBN test:  {len(test)} samples ({test.index.min().date()} -> {test.index.max().date()})")
    print()

    edges = discretize(train.drop(columns=["outcome"]))
    train_binned = apply_bins(train.drop(columns=["outcome"]), edges)
    test_binned = apply_bins(test.drop(columns=["outcome"]), edges)
    train_y = train["outcome"].astype(int).to_numpy()
    test_y = test["outcome"].astype(int).to_numpy()

    prior, likelihoods = fit_naive_bayes(train_binned, train_y)
    test_post = predict_naive_bayes(test_binned, prior, likelihoods)
    test_pred = test_post.argmax(axis=1)

    # Macro F1
    f1s = []
    for cls in range(5):
        tp = ((test_pred == cls) & (test_y == cls)).sum()
        fp = ((test_pred == cls) & (test_y != cls)).sum()
        fn = ((test_pred != cls) & (test_y == cls)).sum()
        if tp + fp == 0 or tp + fn == 0:
            f1s.append(0.0); continue
        p = tp / (tp + fp); r = tp / (tp + fn)
        f1s.append(2*p*r/(p+r) if (p+r) > 0 else 0.0)
    print(f"BBN OOS macro_F1 (2020-2025): {np.mean(f1s):.4f}, accuracy: {(test_pred==test_y).mean():.4f}")
    print()

    bbn_post_df = pd.DataFrame(test_post, index=test.index, columns=[f"p_{c}" for c in CLASS_NAMES])
    bbn_post_df["pred_class"] = test_pred

    print("Running VRP V2 / V2.5b / V2.5d on 6Y OOS test (2020-2025)...")
    print()
    print(f"{'tf':>4s} {'variant':30s}{'trades':>8s}{'sharpe':>8s}{'maxdd':>9s}"
          f"{'total':>9s}{'wf_med':>9s}{'wf_pos':>10s}{'wf_min':>9s}")
    print("-" * 100)
    for tf, feather in [("5m", NQ_5M), ("15m", NQ_15M), ("1h", NQ_1H)]:
        df_tf = build_intraday_with_bbn(feather, bbn_post_df)
        bull = df_tf["bull_score"]
        pred = df_tf["pred_class"]
        bbn_avail = df_tf["p_up"].notna()

        variants = {
            "V2 baseline":               df_tf["v2_entry"],
            "V2.5b deny bull (0.6,0.8]": df_tf["v2_entry"] & ~(bbn_avail & (bull > 0.6) & (bull <= 0.8)),
            "V2.5d only pred_class<=2":  df_tf["v2_entry"] & bbn_avail & (pred <= 2),
        }

        for label, mask in variants.items():
            trades = simulate(df_tf, mask)
            if trades.empty:
                print(f"{tf:>4s} {label:30s}{0:>8d}{0:>8.3f}{0:>9.2%}{0:>9.2%}{0:>9.3f}{0:>9.1f}%{0:>9.3f}")
                continue
            m = annual_metrics(daily_pnl(trades))
            wf = walk_forward(trades)
            wf_med = wf["wf_med"] if wf else 0.0
            wf_pos = wf["wf_pos"] * 100 if wf else 0.0
            wf_min = wf["wf_min"] if wf else 0.0
            print(f"{tf:>4s} {label:30s}{len(trades):>8d}{m['sharpe']:>8.3f}"
                  f"{m['max_dd']:>9.2%}{m['total']:>9.2%}"
                  f"{wf_med:>9.3f}{wf_pos:>9.1f}%{wf_min:>9.3f}")
        print()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
