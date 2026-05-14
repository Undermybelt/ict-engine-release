"""
pandas_vrp_v25d_cross_market.py — Slice 124. Cross-market validation of
V2.5d (only-trade if BBN pred_class in {flat, down}).

Retrain BBN on 2016-10 to 2024-12 (8 years training), predict for
2025-01 to 2026-05 (1Y+ test, covers cross-market 1Y feathers).

Apply V2 baseline + V2.5d on:
- NQ/USD 15m (2018-2026, but BBN test only 2025-2026 here for fair comparison)
- SPY/USD 15m (May 2025 - May 2026, ~1Y)
- IWM/USD 15m
- DIA/USD 15m
- GLD/USD 15m

Caveat: BBN trained on forward-20d NQ outcomes. SPY/IWM/DIA share US
equity vol regime (highly correlated to NQ). GLD has its own regime —
expect V2.5d to degrade most on GLD.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

DATA_DIR = Path("user_data/data")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
QQQ_IV_CSV = PROBE_DIR / "qqq.iv.1d.10y.csv"
VIX3M_CSV = PROBE_DIR / "vix3m.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
VIX_CSV = PROBE_DIR / "vix.1d.10y.csv"

NQ_15M = DATA_DIR / "NQ_USD-15m.feather"
BBN_TRAIN_START = pd.Timestamp("2016-10-01", tz="UTC")
BBN_TRAIN_END = pd.Timestamp("2024-12-31", tz="UTC")
TEST_START = pd.Timestamp("2025-01-01", tz="UTC")
TEST_END = pd.Timestamp("2026-05-31", tz="UTC")

FORWARD_DAYS = 20
N_BINS = 6
LAPLACE_ALPHA = 0.5
TRADING_DAYS = 252.0
STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005
CLASS_NAMES = ["crash", "down", "flat", "up", "strong_up"]

MARKETS = [
    ("NQ/USD",  DATA_DIR / "NQ_USD-15m.feather"),
    ("SPY/USD", DATA_DIR / "SPY_USD-15m.feather"),
    ("IWM/USD", DATA_DIR / "IWM_USD-15m.feather"),
    ("DIA/USD", DATA_DIR / "DIA_USD-15m.feather"),
    ("GLD/USD", DATA_DIR / "GLD_USD-15m.feather"),
]


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
    outcome = future_ret.apply(label_outcome)
    df = feats.copy(); df["outcome"] = outcome

    train = df.loc[df.index <= BBN_TRAIN_END].dropna()
    test_period = df.loc[(df.index >= TEST_START) & (df.index <= TEST_END)]
    # Test period may have NaN outcomes (forward-window), keep features
    test_period = test_period.dropna(subset=["qqq_hv_level", "nq_vs_200d_pct",
                                              "vix3m_level", "qqq_hv_pct_rank_252",
                                              "vvix_over_vix"])

    print(f"BBN train: {len(train)} samples ({train.index.min().date()} -> {train.index.max().date()})")
    print(f"BBN test predict: {len(test_period)} days ({test_period.index.min().date()} -> "
          f"{test_period.index.max().date()})")

    edges = {}
    for col in ["qqq_hv_level", "nq_vs_200d_pct", "vix3m_level",
                "qqq_hv_pct_rank_252", "vvix_over_vix"]:
        try:
            _, e = pd.qcut(train[col], q=N_BINS, retbins=True, duplicates="drop")
            edges[col] = e
        except Exception:
            edges[col] = np.linspace(train[col].min(), train[col].max(), N_BINS + 1)

    train_binned = pd.DataFrame(index=train.index)
    test_binned = pd.DataFrame(index=test_period.index)
    for col, e in edges.items():
        train_binned[col] = pd.cut(train[col], bins=e, labels=False, include_lowest=True)
        test_binned[col] = pd.cut(test_period[col], bins=e, labels=False, include_lowest=True)
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
    bbn_post = pd.DataFrame(post_arr, index=test_period.index,
                            columns=[f"p_{c}" for c in CLASS_NAMES])
    bbn_post["pred_class"] = post_arr.argmax(axis=1)
    return bbn_post


def build_indicators(feather_path, bbn_post):
    df = pd.read_feather(feather_path)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index().loc[TEST_START:TEST_END]
    if len(df) < 500:
        return None
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
    for col in bbn_post.columns:
        df[col] = pd.Series(cd.map(bbn_post[col]), index=df.index).ffill()
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


def main() -> int:
    bbn_post = train_bbn()
    print()
    print(f"Cross-market V2 vs V2.5d (BBN trained 2016-2024 NQ-outcome, applied OOS 2025-2026)")
    print()
    print(f"{'market':10s}{'variant':30s}{'window':25s}{'trades':>8s}{'sharpe':>8s}{'maxdd':>9s}{'total':>9s}")
    print("-" * 100)
    for name, feather in MARKETS:
        df = build_indicators(feather, bbn_post)
        if df is None:
            print(f"{name:10s}TOO SHORT")
            continue
        bbn_avail = df["p_up"].notna()
        pred = df["pred_class"]
        bull = df["bull_score"]
        window = f"{df.index.min().date()}->{df.index.max().date()}"
        for label, mask in [
            ("V2 baseline", df["v2_entry"]),
            ("V2.5b deny bull (0.6,0.8]",
             df["v2_entry"] & ~(bbn_avail & (bull > 0.6) & (bull <= 0.8))),
            ("V2.5d only pred_class<=2",
             df["v2_entry"] & bbn_avail & (pred <= 2)),
        ]:
            trades = simulate(df, mask)
            if trades.empty:
                print(f"{name:10s}{label:30s}{window:25s}{0:>8d}{0:>8.3f}{0:>9.2%}{0:>9.2%}")
                continue
            m = annual_metrics(daily_pnl(trades))
            print(f"{name:10s}{label:30s}{window:25s}{len(trades):>8d}"
                  f"{m['sharpe']:>8.3f}{m['max_dd']:>9.2%}{m['total']:>9.2%}")
        print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
