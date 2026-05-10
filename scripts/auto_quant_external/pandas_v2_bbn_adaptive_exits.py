"""
pandas_v2_bbn_adaptive_exits.py — Slice 129. Test V2 with EXIT-side BBN
integration (instead of entry filtering).

V2's standard exits: IV percentile rank > 0.55 OR close < EMA89.
Both fire AFTER vol has started expanding or price has broken regime.

Hypothesis: BBN posterior may detect regime shift earlier than vol
gates. Adding "exit when P(crash) > threshold" or "exit when bull_score
shifts upward into death-zone" could pre-empt drawdown.

Variants:
- V2 baseline (entry-only filters as in Slice 128)
- V2 + exit when P(crash) > 0.30
- V2 + exit when P(crash) > 0.50 (less aggressive)
- V2 + exit when bull_score crosses into (0.6, 0.8] death zone
- V2 + exit when bull_score > 0.7 (post-trade momentum exhaustion)
- V2 + exit when pred_class changes to up (regime shifted to bullish-extension)

Test on NQ 5m 2020-2025 OOS. Compare to baseline + Slice 128 best
(continuous aggressive-decay sizing 3.66 Sharpe).
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

NQ_5M = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-5m.feather")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_IV_CSV = PROBE_DIR / "qqq.iv.1d.10y.csv"
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
VIX3M_CSV = PROBE_DIR / "vix3m.1d.10y.csv"
VIX_CSV = PROBE_DIR / "vix.1d.10y.csv"
NQ_15M = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather")

TEST_START = pd.Timestamp("2020-01-01", tz="UTC")
TEST_END = pd.Timestamp("2025-12-31", tz="UTC")
BBN_TRAIN_START = pd.Timestamp("2016-10-01", tz="UTC")
BBN_TRAIN_END = pd.Timestamp("2019-12-31", tz="UTC")
TRADING_DAYS = 252.0
STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005
FORWARD_DAYS = 20
N_BINS = 6
LAPLACE_ALPHA = 0.5


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


def train_bbn():
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
    feats["outcome"] = future_ret.apply(label_outcome)
    df_all = feats.dropna(subset=["qqq_hv_level", "nq_vs_200d_pct", "vix3m_level",
                                   "qqq_hv_pct_rank_252", "vvix_over_vix"])
    train = df_all.loc[(df_all.index <= BBN_TRAIN_END) & df_all["outcome"].notna()]
    test = df_all.loc[df_all.index >= TEST_START]
    feature_cols = ["qqq_hv_level", "nq_vs_200d_pct", "vix3m_level",
                     "qqq_hv_pct_rank_252", "vvix_over_vix"]
    edges = {}
    for col in feature_cols:
        try:
            _, e = pd.qcut(train[col], q=N_BINS, retbins=True, duplicates="drop")
            edges[col] = e
        except Exception:
            edges[col] = np.linspace(train[col].min(), train[col].max(), N_BINS + 1)
    train_binned = pd.DataFrame(index=train.index)
    test_binned = pd.DataFrame(index=test.index)
    for col, e in edges.items():
        train_binned[col] = pd.cut(train[col], bins=e, labels=False, include_lowest=True)
        test_binned[col] = pd.cut(test[col], bins=e, labels=False, include_lowest=True)
    train_binned = train_binned.fillna(0).astype(int)
    test_binned = test_binned.fillna(0).astype(int)
    train_y = train["outcome"].astype(int).to_numpy()
    n = len(train_y)
    counts = np.bincount(train_y, minlength=5).astype(float)
    prior = (counts + LAPLACE_ALPHA) / (n + LAPLACE_ALPHA * 5)
    likelihoods = {}
    for col in train_binned.columns:
        x = train_binned[col].to_numpy()
        n_bins_col = int(x.max()) + 1
        like = np.full((n_bins_col, 5), LAPLACE_ALPHA, dtype=float)
        for xi, yi in zip(x, train_y):
            like[xi, yi] += 1.0
        like = like / like.sum(axis=0, keepdims=True)
        likelihoods[col] = like

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
    post_arr = np.array(posteriors)
    bbn_post = pd.DataFrame(post_arr, index=test.index,
                            columns=["p_crash", "p_down", "p_flat", "p_up", "p_strong_up"])
    bbn_post["bull_score"] = bbn_post["p_up"] + bbn_post["p_strong_up"]
    bbn_post["pred_class"] = post_arr.argmax(axis=1)
    return bbn_post


def build_indicators(bbn_post):
    df = pd.read_feather(NQ_5M)
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
    df["v2_entry"] = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & (df["iv_pct_rank_252"] < 0.30)
        & (df["hv_pct_rank_252"] < 0.30)
        & (df["vvix_pct_rank_252"] < 0.40)
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    df["base_exit"] = (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])
    for col in bbn_post.columns:
        df[col] = pd.Series(cd.map(bbn_post[col]), index=df.index).ffill()
    return df


def simulate(df, exit_extra):
    """Simulate V2 entries with extra exit signal added (bool series)."""
    closes = df["close"].to_numpy(); highs = df["high"].to_numpy(); lows = df["low"].to_numpy()
    es = df["v2_entry"].to_numpy()
    base_x = df["base_exit"].to_numpy()
    extra_x = exit_extra.to_numpy() if exit_extra is not None else np.zeros(len(df), dtype=bool)
    ts = df.index.to_numpy()
    trades = []; in_pos = False; entry_idx = -1; entry_price = 0.0; peak = 0.0; trail = False
    for i in range(len(df)):
        if not in_pos:
            if es[i]:
                in_pos = True; entry_idx = i; entry_price = closes[i]; peak = closes[i]; trail = False
            continue
        peak = max(peak, highs[i])
        if not trail and (peak / entry_price - 1.0) >= TRAILING_OFFSET: trail = True
        sl = entry_price * (1.0 + STOPLOSS)
        tp = peak * (1.0 - TRAILING_STOP) if trail else 0.0
        eff = max(sl, tp)
        reason = None; exit_price = closes[i]
        if lows[i] <= eff: reason = "stop"; exit_price = eff
        elif base_x[i]: reason = "base_exit"
        elif extra_x[i]: reason = "bbn_exit"
        if reason is not None:
            trades.append({"open_date": pd.Timestamp(ts[entry_idx]),
                           "close_date": pd.Timestamp(ts[i]),
                           "profit_ratio": exit_price / entry_price - 1.0,
                           "reason": reason})
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


def main() -> int:
    print("Training BBN...")
    bbn_post = train_bbn()
    print(f"BBN posteriors: {len(bbn_post)}")
    df = build_indicators(bbn_post)
    print(f"5m bars: {len(df)}")
    print()

    bull = df["bull_score"]
    p_crash = df["p_crash"]
    pred = df["pred_class"]
    bull_prev = bull.shift(1)
    pred_prev = pred.shift(1)

    variants = {
        "V2 baseline (no BBN exit)":           None,
        "V2 + exit P(crash) > 0.30":           p_crash > 0.30,
        "V2 + exit P(crash) > 0.50":           p_crash > 0.50,
        "V2 + exit bull in (0.6, 0.8]":        (bull > 0.6) & (bull <= 0.8),
        "V2 + exit bull > 0.7":                bull > 0.7,
        "V2 + exit pred_class crossed to up":  (pred == 3) & (pred_prev != 3),
        "V2 + exit bull crossed into death":   (bull > 0.6) & (bull <= 0.8) & (bull_prev <= 0.6),
    }

    print(f"{'variant':40s}{'trades':>8s}{'sharpe':>8s}{'maxdd':>9s}{'total':>9s}{'bbn_ex%':>9s}")
    print("-" * 85)
    for label, exit_extra in variants.items():
        trades = simulate(df, exit_extra)
        if trades.empty:
            print(f"{label:40s}{0:>8d}{0:>8.3f}{0:>9.2%}{0:>9.2%}{0:>9.1f}%"); continue
        m = annual_metrics(daily_pnl(trades))
        bbn_pct = (trades["reason"] == "bbn_exit").mean() * 100 if "reason" in trades.columns else 0
        print(f"{label:40s}{len(trades):>8d}{m['sharpe']:>8.3f}"
              f"{m['max_dd']:>9.2%}{m['total']:>9.2%}{bbn_pct:>8.1f}%")
    print()

    print("Reference points:")
    print(f"  V2 baseline (Slice 123):           Sharpe 3.51, DD -3.75%")
    print(f"  V2.5d (Slice 123 best entry filter): Sharpe 5.13, DD -1.55%")
    print(f"  V2 + aggressive_decay (Slice 128): Sharpe 3.66, DD -2.89%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
