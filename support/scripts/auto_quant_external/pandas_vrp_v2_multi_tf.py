"""
pandas_vrp_v2_multi_tf.py — Slice 115. Test VRPCompression V2 across NQ
timeframes (5m / 15m / 1h) to address "时间周期够多" preference and
verify cross-timeframe stability of the edge.

Hypothesis: if the IV/HV/VVIX compression-regime edge is real, it should
show up across multiple bar resolutions because the daily vol gates are
timeframe-agnostic and the EMA structure is bar-count-based (the same
21/89/200/600 EMA structure relative to recent bars). If the edge is
purely a 15m microstructure artifact, 5m and 1h will collapse.

Trade hour gating: liquid window 13-21 UTC (RTH). Same percentages for
stop/trailing.
"""
from __future__ import annotations

from pathlib import Path
import sys

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

DATA_DIR = Path("user_data/data")
QQQ_IV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.iv.1d.10y.csv")
QQQ_HV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.hv.1d.10y.csv")
VVIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vvix.1d.10y.csv")
START = pd.Timestamp("2018-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
TRADING_DAYS = 252.0

STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005
VVIX_THRESHOLD = 0.40

TIMEFRAMES = [
    ("5m", DATA_DIR / "NQ_USD-5m.feather"),
    ("15m", DATA_DIR / "NQ_USD-15m.feather"),
    ("1h", DATA_DIR / "NQ_USD-1h.feather"),
]


def load_close_series(csv_path):
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def load_indicators(feather_path, iv_pr, hv_pr, vvix_pr):
    df = pd.read_feather(feather_path)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index().loc[START:END]
    df["ema21"] = df["close"].ewm(span=21, adjust=False).mean()
    df["ema89"] = df["close"].ewm(span=89, adjust=False).mean()
    df["ema200"] = df["close"].ewm(span=200, adjust=False).mean()
    df["ema600"] = df["close"].ewm(span=600, adjust=False).mean()
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour
    candle_dates = df.index.normalize()
    df["iv_pct_rank_252"] = pd.Series(candle_dates.map(iv_pr), index=df.index).ffill()
    df["hv_pct_rank_252"] = pd.Series(candle_dates.map(hv_pr), index=df.index).ffill()
    df["vvix_pct_rank_252"] = pd.Series(candle_dates.map(vvix_pr), index=df.index).ffill()
    df["liquid_window"] = (df["hour_utc"] >= 13) & (df["hour_utc"] <= 21)
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])
    df["entry_signal"] = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & (df["iv_pct_rank_252"] < 0.30)
        & (df["hv_pct_rank_252"] < 0.30)
        & (df["vvix_pct_rank_252"] < VVIX_THRESHOLD)
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    df["exit_signal"] = (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])
    return df


def simulate(df):
    closes = df["close"].to_numpy(); highs = df["high"].to_numpy(); lows = df["low"].to_numpy()
    es = df["entry_signal"].to_numpy(); xs = df["exit_signal"].to_numpy()
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
    if d.empty or d.std() == 0:
        return {"sharpe": 0.0, "sortino": 0.0, "max_dd": 0.0, "total": 0.0}
    sharpe = (d.mean() / d.std()) * np.sqrt(TRADING_DAYS) if d.std() > 0 else 0.0
    downside = d[d < 0]
    sortino = (d.mean() / downside.std()) * np.sqrt(TRADING_DAYS) if (len(downside) > 1 and downside.std() > 0) else 0.0
    cum = (1.0 + d).cumprod(); dd = float((cum / cum.cummax() - 1.0).min())
    return {"sharpe": float(sharpe), "sortino": float(sortino),
            "max_dd": dd, "total": float(cum.iloc[-1] - 1.0)}


def walk_forward(trades):
    if trades.empty: return None
    cur = pd.Timestamp("2020-01-01", tz="UTC"); end = pd.Timestamp("2026-01-01", tz="UTC")
    sharpes, dds = [], []
    while cur < end:
        nxt = cur + pd.DateOffset(months=6)
        wt = trades[(trades["open_date"] >= cur) & (trades["open_date"] < nxt)]
        if len(wt) >= 5:
            m = annual_metrics(daily_pnl(wt))
            sharpes.append(m["sharpe"]); dds.append(m["max_dd"])
        cur = nxt
    if not sharpes: return None
    return {
        "wf_median": float(np.median(sharpes)),
        "wf_mean": float(np.mean(sharpes)),
        "wf_pos_pct": float(sum(1 for s in sharpes if s > 0) / len(sharpes)),
        "wf_min": float(min(sharpes)),
        "wf_n_windows": len(sharpes),
    }


def main() -> int:
    iv = load_close_series(QQQ_IV_CSV)
    hv = load_close_series(QQQ_HV_CSV)
    vvix = load_close_series(VVIX_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)

    print(f"VRPCompression V2 (VVIX<{VVIX_THRESHOLD}) multi-timeframe NQ 8Y (2018-2025):")
    print()
    print(f"{'tf':>4s}{'bars':>10s}{'sig.bars':>10s}{'trades':>8s}"
          f"{'sharpe':>8s}{'sortino':>9s}{'maxdd':>9s}{'total':>9s}"
          f"{'wf_med':>9s}{'wf_pos':>9s}{'wf_n':>6s}")
    print("-" * 96)

    for tf, feather in TIMEFRAMES:
        df = load_indicators(feather, iv_pr, hv_pr, vvix_pr)
        n_signal = int(df["entry_signal"].sum())
        trades = simulate(df)
        if trades.empty:
            print(f"{tf:>4s}{len(df):>10d}{n_signal:>10d}      0    NO TRADES")
            continue
        m = annual_metrics(daily_pnl(trades))
        wf = walk_forward(trades)
        wf_med = wf["wf_median"] if wf else float("nan")
        wf_pos = wf["wf_pos_pct"] if wf else float("nan")
        wf_n = wf["wf_n_windows"] if wf else 0
        print(f"{tf:>4s}{len(df):>10d}{n_signal:>10d}{len(trades):>8d}"
              f"{m['sharpe']:>8.3f}{m['sortino']:>9.3f}"
              f"{m['max_dd']:>9.2%}{m['total']:>9.2%}"
              f"{wf_med:>9.3f}{wf_pos*100:>8.1f}%{wf_n:>6d}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
