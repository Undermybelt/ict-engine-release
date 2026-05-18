"""
pandas_binary_crash_detector.py — Slice 127. Pivot to binary
crash-detection task instead of 5-class direction (which saturated at
macro_F1 ~0.20 OOS in Slices 118/123/126).

Target: will next 20 days see a peak-to-trough drawdown >= 5%?
This is a cleaner risk-management question:
- Binary instead of 5-class
- More balanced (15-30% of windows have a crash event)
- Direct utility for VRP execution — deny entries when P(crash) high

Same 5 BBN features (Slice 117 top-5).
Train: 2016-10 to 2019-12
Test:  2020-01 to 2025-12

Metrics: accuracy, F1, precision, recall, AUC.

If precision > 0.5 at recall > 0.3, this is a useful crash-detection
signal. Final test: V2 baseline vs V2 + deny when P(crash) > 0.5 on
2020-2025 6Y OOS — does crash-deny improve Sharpe / DD?
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

NQ_15M = Path("user_data/data/NQ_USD-15m.feather")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
QQQ_IV_CSV = PROBE_DIR / "qqq.iv.1d.10y.csv"
VIX3M_CSV = PROBE_DIR / "vix3m.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
VIX_CSV = PROBE_DIR / "vix.1d.10y.csv"

BBN_TRAIN_END = pd.Timestamp("2019-12-31", tz="UTC")
TEST_START = pd.Timestamp("2020-01-01", tz="UTC")
TEST_END = pd.Timestamp("2025-12-31", tz="UTC")
FORWARD_DAYS = 20
DD_THRESHOLD = 0.05  # 5% drawdown defines "crash"
N_BINS = 6
LAPLACE_ALPHA = 0.5
TRADING_DAYS = 252.0
STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005


def load_close(csv_path):
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def build_features():
    df15 = pd.read_feather(NQ_15M)
    df15["date"] = pd.to_datetime(df15["date"], unit="ms", utc=True)
    df15 = df15.set_index("date").sort_index().loc[pd.Timestamp("2016-10-01", tz="UTC"):TEST_END]
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

    # Binary target: forward 20d max peak-to-trough drawdown >= DD_THRESHOLD
    n = len(nq_daily)
    crash = pd.Series(np.nan, index=nq_daily.index)
    prices = nq_daily.values
    for i in range(n - FORWARD_DAYS):
        window = prices[i:i + FORWARD_DAYS + 1]
        peak = window[0]
        max_dd = 0.0
        for v in window[1:]:
            if v > peak:
                peak = v
            dd = (peak - v) / peak if peak > 0 else 0.0
            if dd > max_dd:
                max_dd = dd
        crash.iloc[i] = 1 if max_dd >= DD_THRESHOLD else 0

    df = feats.copy()
    df["crash"] = crash
    return df.dropna()


def discretize(train_df, n_bins=N_BINS):
    edges = {}
    for col in train_df.columns:
        if col == "crash": continue
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


def fit_naive_bayes_binary(train_binned, train_y, alpha=LAPLACE_ALPHA):
    n = len(train_y)
    counts = np.bincount(train_y, minlength=2).astype(float)
    prior = (counts + alpha) / (n + alpha * 2)
    likelihoods = {}
    for col in train_binned.columns:
        x = train_binned[col].to_numpy()
        n_bins_col = int(x.max()) + 1
        like = np.full((n_bins_col, 2), alpha, dtype=float)
        for xi, yi in zip(x, train_y):
            like[xi, int(yi)] += 1.0
        like = like / like.sum(axis=0, keepdims=True)
        likelihoods[col] = like
    return prior, likelihoods


def predict_binary(test_binned, prior, likelihoods):
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
        posteriors.append(p[1])  # P(crash=1)
    return np.array(posteriors)


def auc_score(y_true, y_score):
    """ROC AUC via Mann-Whitney U statistic."""
    pos = y_score[y_true == 1]
    neg = y_score[y_true == 0]
    if len(pos) == 0 or len(neg) == 0:
        return 0.5
    n_pos, n_neg = len(pos), len(neg)
    rank_sum = 0
    all_scores = np.concatenate([pos, neg])
    ranks = pd.Series(all_scores).rank().to_numpy()
    rank_sum = ranks[:n_pos].sum()
    auc = (rank_sum - n_pos * (n_pos + 1) / 2) / (n_pos * n_neg)
    return float(auc)


def precision_recall_at(y_true, y_score, threshold):
    pred = (y_score >= threshold).astype(int)
    tp = ((pred == 1) & (y_true == 1)).sum()
    fp = ((pred == 1) & (y_true == 0)).sum()
    fn = ((pred == 0) & (y_true == 1)).sum()
    p = tp / max(tp + fp, 1)
    r = tp / max(tp + fn, 1)
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0.0
    n_pred = (pred == 1).sum()
    return p, r, f1, n_pred


def vrp_v2_filter_test(post_df):
    """Apply V2 baseline + V2 + crash-deny on 6Y OOS."""
    print("\n=== V2 + crash-deny integration test on 6Y OOS (NQ 5m) ===")
    df = pd.read_feather(Path("user_data/data/NQ_USD-5m.feather"))
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
    df["exit_signal"] = (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])
    df["p_crash"] = pd.Series(cd.map(post_df["p_crash"]), index=df.index).ffill()

    def sim(df, mask):
        closes = df["close"].to_numpy(); highs = df["high"].to_numpy(); lows = df["low"].to_numpy()
        es = mask.to_numpy(); xs = df["exit_signal"].to_numpy()
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
            elif xs[i]: reason = "exit"
            if reason is not None:
                trades.append({"open_date": pd.Timestamp(ts[entry_idx]),
                               "close_date": pd.Timestamp(ts[i]),
                               "profit_ratio": exit_price / entry_price - 1.0})
                in_pos = False; entry_idx = -1; entry_price = 0.0; peak = 0.0; trail = False
        return pd.DataFrame(trades)

    def annual_metrics(d):
        if d.empty or d.std() == 0: return {"sharpe": 0.0, "max_dd": 0.0, "total": 0.0}
        sharpe = (d.mean() / d.std()) * np.sqrt(TRADING_DAYS) if d.std() > 0 else 0.0
        cum = (1.0 + d).cumprod(); dd = float((cum / cum.cummax() - 1.0).min())
        return {"sharpe": float(sharpe), "max_dd": dd, "total": float(cum.iloc[-1] - 1.0)}

    def daily_pnl(t):
        if t.empty: return pd.Series(dtype=float)
        s = t.copy(); s["date"] = s["close_date"].dt.normalize()
        return s.groupby("date")["profit_ratio"].sum().sort_index()

    bbn_avail = df["p_crash"].notna()
    print(f"{'variant':30s}{'trades':>8s}{'sharpe':>8s}{'maxdd':>9s}{'total':>9s}")
    print("-" * 65)
    for label, mask in [
        ("V2 baseline", df["v2_entry"]),
        ("V2 + deny p_crash > 0.30", df["v2_entry"] & ~(bbn_avail & (df["p_crash"] > 0.30))),
        ("V2 + deny p_crash > 0.40", df["v2_entry"] & ~(bbn_avail & (df["p_crash"] > 0.40))),
        ("V2 + deny p_crash > 0.50", df["v2_entry"] & ~(bbn_avail & (df["p_crash"] > 0.50))),
        ("V2 + deny p_crash > 0.60", df["v2_entry"] & ~(bbn_avail & (df["p_crash"] > 0.60))),
    ]:
        trades = sim(df, mask)
        if trades.empty:
            print(f"{label:30s}{0:>8d}{0:>8.3f}{0:>9.2%}{0:>9.2%}"); continue
        m = annual_metrics(daily_pnl(trades))
        print(f"{label:30s}{len(trades):>8d}{m['sharpe']:>8.3f}{m['max_dd']:>9.2%}{m['total']:>9.2%}")


def main() -> int:
    print(f"Building features with binary crash target (DD >= {DD_THRESHOLD*100:.0f}% in next {FORWARD_DAYS}d)")
    df = build_features()
    train = df.loc[df.index <= BBN_TRAIN_END]
    test = df.loc[df.index >= TEST_START]
    print(f"Train: {len(train)} samples")
    print(f"Test:  {len(test)} samples")
    print()
    print(f"Train crash rate: {train['crash'].mean()*100:.1f}% ({int(train['crash'].sum())} of {len(train)})")
    print(f"Test crash rate:  {test['crash'].mean()*100:.1f}% ({int(test['crash'].sum())} of {len(test)})")
    print()

    edges = discretize(train.drop(columns=["crash"]))
    train_binned = apply_bins(train.drop(columns=["crash"]), edges)
    test_binned = apply_bins(test.drop(columns=["crash"]), edges)
    train_y = train["crash"].astype(int).to_numpy()
    test_y = test["crash"].astype(int).to_numpy()

    prior, likelihoods = fit_naive_bayes_binary(train_binned, train_y)
    train_score = predict_binary(train_binned, prior, likelihoods)
    test_score = predict_binary(test_binned, prior, likelihoods)

    print(f"Test score distribution:")
    for q in [0.05, 0.10, 0.25, 0.50, 0.75, 0.90, 0.95]:
        print(f"  {q*100:>4.0f}th percentile: {np.quantile(test_score, q):.4f}")
    print()

    print(f"Test AUC: {auc_score(test_y, test_score):.4f}")
    print(f"Train AUC: {auc_score(train_y, train_score):.4f}")
    print()

    print(f"{'threshold':>10s}{'precision':>11s}{'recall':>9s}{'F1':>7s}{'n_pred':>8s}")
    print("-" * 50)
    for thr in [0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80]:
        p, r, f1, n_pred = precision_recall_at(test_y, test_score, thr)
        print(f"{thr:>10.2f}{p:>11.4f}{r:>9.4f}{f1:>7.4f}{n_pred:>8d}")
    print()

    # Save predictions for V2 integration
    post_df = pd.DataFrame({"p_crash": test_score}, index=test.index)
    out_path = PROBE_DIR / "slice_127_crash_predictions.csv"
    post_df.to_csv(out_path)

    # V2 integration
    vrp_v2_filter_test(post_df)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
