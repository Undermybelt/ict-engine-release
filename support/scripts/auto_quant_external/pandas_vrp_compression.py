"""
pandas_vrp_compression.py — pandas drought-free implementation of VRPCompression15m.

Slice 91/95 found VRPCompression15m had the strongest standalone freqtrade
Sharpe (0.34 on 8Y per-trade, 3.34 daily-resampled annualized inflated by
sparse-trading bias). The candidate uses QQQ IV / HV percentile-rank cached
data — when both IV and HV are in the bottom 30% of their 252-day window
AND price holds above EMA89 AND 4h trend is up AND bar is bullish, enter
long. Exit when vol expands (iv_pct_rank > 0.55) or regime break or upper
target.

This script reimplements VRPCompression in the pandas drought-free harness
to get an honest 8Y Sharpe estimate, then compares to TrendPullback-NoRSI
to see if the 2-candidate basket exceeds 1.48 single-best.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table

NQ_15M_FEATHER = Path("user_data/data/NQ_USD-15m.feather")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
QQQ_IV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.iv.1d.10y.csv")
QQQ_HV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.hv.1d.10y.csv")
START = pd.Timestamp("2018-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
TRAIN_END = pd.Timestamp("2023-01-01", tz="UTC")
TRADING_DAYS = 252.0
MIN_TRADES_PER_CELL = 30

STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005


def load_close_series(csv_path: Path) -> pd.Series:
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def load_term_structure() -> pd.Series:
    vix9d = load_close_series(VIX9D_CSV)
    vix3m = load_close_series(VIX3M_CSV)
    common = vix9d.index.intersection(vix3m.index)
    return (vix9d.loc[common] / vix3m.loc[common].where(vix3m.loc[common] > 1e-9))


def classify_term(value: float) -> str:
    if not (value == value):
        return "unknown"
    if value < 0.92:
        return "DeepContango"
    if value <= 1.00:
        return "Contango"
    if value <= 1.05:
        return "FlatToBackward"
    return "Backwardation"


def load_indicators_with_vol() -> pd.DataFrame:
    df = pd.read_feather(NQ_15M_FEATHER)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index().loc[START:END]
    df["ema21"] = df["close"].ewm(span=21, adjust=False).mean()
    df["ema89"] = df["close"].ewm(span=89, adjust=False).mean()
    df["ema200"] = df["close"].ewm(span=200, adjust=False).mean()
    df["ema600"] = df["close"].ewm(span=600, adjust=False).mean()
    hl = df["high"] - df["low"]
    hc = (df["high"] - df["close"].shift(1)).abs()
    lc = (df["low"] - df["close"].shift(1)).abs()
    df["atr"] = pd.concat([hl, hc, lc], axis=1).max(axis=1).ewm(alpha=1 / 14, adjust=False).mean()
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour

    iv = load_close_series(QQQ_IV_CSV)
    hv = load_close_series(QQQ_HV_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    candle_dates = df.index.normalize()
    df["iv_pct_rank_252"] = pd.Series(candle_dates.map(iv_pr), index=df.index).ffill()
    df["hv_pct_rank_252"] = pd.Series(candle_dates.map(hv_pr), index=df.index).ffill()

    df["liquid_window"] = (df["hour_utc"] >= 13) & (df["hour_utc"] <= 21)
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])
    df["iv_compressed"] = df["iv_pct_rank_252"] < 0.30
    df["hv_compressed"] = df["hv_pct_rank_252"] < 0.30
    df["entry_signal"] = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & df["iv_compressed"]
        & df["hv_compressed"]
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    df["exit_signal"] = (
        (df["iv_pct_rank_252"] > 0.55)
        | (df["close"] < df["ema89"])
    )
    return df


def simulate(df: pd.DataFrame) -> pd.DataFrame:
    closes = df["close"].to_numpy()
    highs = df["high"].to_numpy()
    lows = df["low"].to_numpy()
    es = df["entry_signal"].to_numpy()
    xs = df["exit_signal"].to_numpy()
    ts = df.index.to_numpy()
    trades: list[dict] = []
    in_pos = False
    entry_idx = -1
    entry_price = 0.0
    peak = 0.0
    trail = False

    for i in range(len(df)):
        if not in_pos:
            if es[i]:
                in_pos = True
                entry_idx = i
                entry_price = closes[i]
                peak = closes[i]
                trail = False
            continue
        peak = max(peak, highs[i])
        gain = peak / entry_price - 1.0
        if not trail and gain >= TRAILING_OFFSET:
            trail = True
        sl = entry_price * (1.0 + STOPLOSS)
        tp = peak * (1.0 - TRAILING_STOP) if trail else 0.0
        eff = max(sl, tp)
        reason = None
        exit_price = closes[i]
        if lows[i] <= eff:
            reason = "stop"
            exit_price = eff
        elif xs[i]:
            reason = "exit_signal"
            exit_price = closes[i]
        if reason is not None:
            trades.append({
                "open_date": pd.Timestamp(ts[entry_idx]),
                "close_date": pd.Timestamp(ts[i]),
                "profit_ratio": exit_price / entry_price - 1.0,
            })
            in_pos = False
            entry_idx = -1
            entry_price = 0.0
            peak = 0.0
            trail = False
    return pd.DataFrame(trades)


def daily_pnl(trades: pd.DataFrame) -> pd.Series:
    if trades.empty:
        return pd.Series(dtype=float)
    s = trades.copy()
    s["date"] = s["close_date"].dt.normalize()
    return s.groupby("date")["profit_ratio"].sum().sort_index()


def annual_metrics(daily_returns: pd.Series) -> dict[str, float]:
    if daily_returns.empty or daily_returns.std() == 0:
        return {"sharpe": 0.0, "sortino": 0.0, "max_dd": 0.0, "total_return": 0.0}
    mean = daily_returns.mean()
    std = daily_returns.std()
    sharpe = (mean / std) * np.sqrt(TRADING_DAYS) if std > 0 else 0.0
    downside = daily_returns[daily_returns < 0]
    sortino = (mean / downside.std()) * np.sqrt(TRADING_DAYS) if (len(downside) > 1 and downside.std() > 0) else 0.0
    cum = (1.0 + daily_returns).cumprod()
    dd = float((cum / cum.cummax() - 1.0).min())
    return {"sharpe": float(sharpe), "sortino": float(sortino),
            "max_dd": dd, "total_return": float(cum.iloc[-1] - 1.0)}


def derive_deny(train_trades: pd.DataFrame) -> set[tuple[str, str]]:
    deny: set[tuple[str, str]] = set()
    if train_trades.empty:
        return deny
    for (regime, term), group in train_trades.groupby(["regime", "term"]):
        n = len(group)
        if n < MIN_TRADES_PER_CELL:
            continue
        returns = group["profit_ratio"].astype(float)
        if returns.std() == 0:
            continue
        sharpe = returns.mean() / returns.std()
        if sharpe < 0 and regime != "unknown" and term != "unknown":
            deny.add((regime, term))
    return deny


def main() -> int:
    print("Loading NQ 15m + QQQ IV/HV...")
    df = load_indicators_with_vol()
    print(f"  bars: {len(df):,}")
    print(f"  entry_signal active on: {df['entry_signal'].sum():,} bars ({df['entry_signal'].mean()*100:.2f}%)")
    print()

    trades = simulate(df)
    if trades.empty:
        print("ERROR: no trades")
        return 1
    print(f"trades: {len(trades)}, span {trades['open_date'].min().date()} -> {trades['close_date'].max().date()}")

    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term)
    trades["entry_date"] = trades["open_date"].dt.normalize()
    trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
    trades["term"] = trades["entry_date"].map(term_class).fillna("unknown")

    train = trades[trades["entry_date"] < TRAIN_END]
    test = trades[trades["entry_date"] >= TRAIN_END]
    deny = derive_deny(train)

    print()
    print(f"train trades: {len(train)}, test trades: {len(test)}")
    print(f"derived deny rules from train: {sorted(deny) if deny else 'none'}")
    print()

    cond_all = trades[~trades.apply(lambda r: (r["regime"], r["term"]) in deny, axis=1)]
    cond_train = train[~train.apply(lambda r: (r["regime"], r["term"]) in deny, axis=1)]
    cond_test = test[~test.apply(lambda r: (r["regime"], r["term"]) in deny, axis=1)]

    print("=" * 90)
    print("VRPCompression pandas drought-free results")
    print("=" * 90)
    print(f"{'window/mode':40s}{'trades':>8s}{'sharpe':>8s}{'sortino':>9s}{'maxdd':>9s}{'totret':>10s}")
    for label, t_set in [
        ("FULL 8Y unconditional", trades),
        ("FULL 8Y V3 conditional", cond_all),
        ("TRAIN 5Y unconditional", train),
        ("TRAIN 5Y V3 conditional", cond_train),
        ("TEST 3Y unconditional", test),
        ("TEST 3Y V3 conditional (rules from train)", cond_test),
    ]:
        if t_set.empty:
            continue
        m = annual_metrics(daily_pnl(t_set))
        print(f"{label:40s}{len(t_set):>8d}{m['sharpe']:>8.3f}{m['sortino']:>9.3f}{m['max_dd']:>9.2%}{m['total_return']:>10.2%}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
