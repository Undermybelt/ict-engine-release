"""
pandas_v2_position_sizing.py — Slice 128. Test continuous position
sizing on V2 trades based on BBN bull_score, vs hard binary gating.

Slices 121-127 established that bull_score (0.5-0.6) is the V2 sweet
spot, and bull_score (0.6-0.8) is a "death zone" where V2 trades are
unprofitable. Hard gating (V2.5b/d) deletes trades entirely.

Continuous sizing keeps some exposure during borderline regimes:
- baseline:        weight = 1 always
- linear_decay:    weight = max(0, 1 - 1.5 * max(0, bull - 0.5))
                   = 1 for bull <= 0.5, linearly to 0 at bull = 1.17
- inverted_U:      weight = 1 for bull in [0.4, 0.6], 0.5 in [0, 0.4],
                   decays from 0.6 to 1.0 to zero
- aggressive_decay: weight = exp(-4 * max(0, bull - 0.45))
                    = 1 for bull <= 0.45, 0.45 at bull=0.65, 0.20 at 0.85

Test on NQ 5m 2020-2025 OOS (BBN trained on 2016-2019).
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

NQ_5M = Path("user_data/data/NQ_USD-5m.feather")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_IV_CSV = PROBE_DIR / "qqq.iv.1d.10y.csv"
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
VIX3M_CSV = PROBE_DIR / "vix3m.1d.10y.csv"
VIX_CSV = PROBE_DIR / "vix.1d.10y.csv"
NQ_15M = Path("user_data/data/NQ_USD-15m.feather")

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
    bbn_post = pd.DataFrame(post_arr, index=test.index, columns=["p_crash", "p_down", "p_flat", "p_up", "p_strong_up"])
    bbn_post["bull_score"] = bbn_post["p_up"] + bbn_post["p_strong_up"]
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
    df["exit_signal"] = (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])
    for col in bbn_post.columns:
        df[col] = pd.Series(cd.map(bbn_post[col]), index=df.index).ffill()
    return df


def simulate_v2(df):
    """Generate V2 trades; record entry-day bull_score per trade."""
    closes = df["close"].to_numpy(); highs = df["high"].to_numpy(); lows = df["low"].to_numpy()
    es = df["v2_entry"].to_numpy(); xs = df["exit_signal"].to_numpy()
    bull = df["bull_score"].to_numpy()
    pred = (df.get("pred_class", pd.Series(0, index=df.index))).to_numpy()
    p_crash = df["p_crash"].to_numpy()
    ts = df.index.to_numpy()
    trades = []; in_pos = False; entry_idx = -1; entry_price = 0.0; peak = 0.0; trail = False
    entry_bull = 0.0; entry_pcrash = 0.0
    for i in range(len(df)):
        if not in_pos:
            if es[i]:
                in_pos = True; entry_idx = i; entry_price = closes[i]; peak = closes[i]; trail = False
                entry_bull = float(bull[i]) if not np.isnan(bull[i]) else 0.5
                entry_pcrash = float(p_crash[i]) if not np.isnan(p_crash[i]) else 0.0
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
                           "profit_ratio": exit_price / entry_price - 1.0,
                           "entry_bull": entry_bull,
                           "entry_pcrash": entry_pcrash})
            in_pos = False; entry_idx = -1; entry_price = 0.0; peak = 0.0; trail = False
    return pd.DataFrame(trades)


def weighting_schemes():
    """Return dict of name -> weight function (takes bull_score, returns weight in [0,1])."""
    def w_baseline(b): return 1.0
    def w_linear(b):
        return max(0.0, 1.0 - 1.5 * max(0.0, b - 0.5))
    def w_inverted_u(b):
        if b < 0.4: return 0.5
        if b <= 0.6: return 1.0
        return max(0.0, 1.0 - (b - 0.6) / 0.4)
    def w_aggressive(b):
        return float(np.exp(-4.0 * max(0.0, b - 0.45)))
    def w_hard_v25b(b):
        return 0.0 if 0.6 < b <= 0.8 else 1.0
    def w_hard_v25d_proxy(b):
        return 0.0 if b > 0.5 else 1.0  # only-trade if bull <= 0.5
    return {
        "1.baseline V2":         w_baseline,
        "2.linear_decay":        w_linear,
        "3.inverted_U":          w_inverted_u,
        "4.aggressive_decay":    w_aggressive,
        "5.hard V2.5b proxy":    w_hard_v25b,
        "6.hard bull<=0.5":      w_hard_v25d_proxy,
    }


def weighted_metrics(trades, w_fn):
    if trades.empty:
        return {"trades_active": 0, "trades_total": 0, "sharpe": 0.0,
                "max_dd": 0.0, "total": 0.0, "mean_weight": 0.0}
    w = trades["entry_bull"].apply(w_fn).clip(lower=0, upper=1)
    weighted_ret = trades["profit_ratio"] * w
    n_active = int((w > 0.05).sum())
    s = trades.copy()
    s["weighted_ret"] = weighted_ret
    s["close_date_n"] = s["close_date"].dt.normalize()
    daily = s.groupby("close_date_n")["weighted_ret"].sum().sort_index()
    if daily.empty or daily.std() == 0:
        return {"trades_active": n_active, "trades_total": len(trades), "sharpe": 0.0,
                "max_dd": 0.0, "total": 0.0, "mean_weight": float(w.mean())}
    sharpe = (daily.mean() / daily.std()) * np.sqrt(TRADING_DAYS) if daily.std() > 0 else 0.0
    cum = (1.0 + daily).cumprod()
    dd = float((cum / cum.cummax() - 1.0).min())
    return {"trades_active": n_active, "trades_total": len(trades),
            "sharpe": float(sharpe), "max_dd": dd,
            "total": float(cum.iloc[-1] - 1.0), "mean_weight": float(w.mean())}


def main() -> int:
    print("Training BBN on 2016-2019, predicting 2020-2025...")
    bbn_post = train_bbn()
    print(f"BBN posteriors: {len(bbn_post)} days")
    print()

    print("Generating V2 trades on NQ 5m 2020-2025...")
    df = build_indicators(bbn_post)
    trades = simulate_v2(df)
    print(f"V2 trades: {len(trades)}")
    print()

    schemes = weighting_schemes()
    print(f"{'scheme':28s}{'trades_act':>11s}{'mean_w':>9s}{'sharpe':>8s}{'maxdd':>9s}{'total':>9s}")
    print("-" * 75)
    for name, fn in schemes.items():
        m = weighted_metrics(trades, fn)
        print(f"{name:28s}{m['trades_active']:>11d}{m['mean_weight']:>9.3f}"
              f"{m['sharpe']:>8.3f}{m['max_dd']:>9.2%}{m['total']:>9.2%}")
    print()
    print("Note: weighted Sharpe assumes capital is reallocated; trades_active counts")
    print("trades with weight > 0.05; mean_w is the average weighting applied to V2 trades")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
