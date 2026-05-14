"""
pandas_vrp_v2_cross_market.py — Slice 113. Apply VRPCompression V2 (with
VVIX<0.40 as 3rd vol-regime gate) across NQ/SPY/IWM/DIA/GLD to confirm
cross-market portability is preserved.

V1 cross-market (Slice 112): NQ 3.33, SPY 6.22, GLD 5.33, IWM 1.80, DIA -1.45.
V2 hypothesis: adding VVIX<0.40 should preserve directional ranking but may
shift Sharpe levels per market. Care: VVIX measures *equity* vol-of-vol,
which is well-aligned for SPY/IWM/DIA but mismatched for GLD (gold has its
own vol regime via GVZ).
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

DATA_DIR = Path("user_data/data")
QQQ_IV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.iv.1d.10y.csv")
QQQ_HV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.hv.1d.10y.csv")
VVIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vvix.1d.10y.csv")
TRADING_DAYS = 252.0

MARKETS = [
    ("NQ/USD", DATA_DIR / "NQ_USD-15m.feather", pd.Timestamp("2018-01-01", tz="UTC"), pd.Timestamp("2025-12-31", tz="UTC")),
    ("SPY/USD", DATA_DIR / "SPY_USD-15m.feather", None, None),
    ("IWM/USD", DATA_DIR / "IWM_USD-15m.feather", None, None),
    ("DIA/USD", DATA_DIR / "DIA_USD-15m.feather", None, None),
    ("GLD/USD", DATA_DIR / "GLD_USD-15m.feather", None, None),
]

STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005
VVIX_THRESHOLD = 0.40


def load_close_series(csv_path: Path) -> pd.Series:
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def load_indicators(feather_path, start, end, iv_pr, hv_pr, vvix_pr):
    df = pd.read_feather(feather_path)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index()
    if start is not None:
        df = df.loc[start:]
    if end is not None:
        df = df.loc[:end]
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
            trades.append({"open_date": pd.Timestamp(ts[entry_idx]), "close_date": pd.Timestamp(ts[i]),
                           "profit_ratio": exit_price / entry_price - 1.0})
            in_pos = False; entry_idx = -1; entry_price = 0.0; peak = 0.0; trail = False
    return pd.DataFrame(trades)


def daily_pnl(t):
    if t.empty: return pd.Series(dtype=float)
    s = t.copy(); s["date"] = s["close_date"].dt.normalize()
    return s.groupby("date")["profit_ratio"].sum().sort_index()


def annual_metrics(d):
    if d.empty or d.std() == 0: return {"sharpe": 0.0, "max_dd": 0.0, "total_return": 0.0}
    sharpe = (d.mean() / d.std()) * np.sqrt(TRADING_DAYS) if d.std() > 0 else 0.0
    cum = (1.0 + d).cumprod(); dd = float((cum / cum.cummax() - 1.0).min())
    return {"sharpe": float(sharpe), "max_dd": dd, "total_return": float(cum.iloc[-1] - 1.0)}


def main() -> int:
    iv = load_close_series(QQQ_IV_CSV)
    hv = load_close_series(QQQ_HV_CSV)
    vvix = load_close_series(VVIX_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)
    print(f"VRPCompression V2 (VVIX<{VVIX_THRESHOLD}) cross-market:")
    print(f"{'market':10s}{'window':27s}{'trades':>8s}{'sharpe':>8s}{'maxdd':>9s}{'total':>9s}")
    print("-" * 80)
    for name, feather, start, end in MARKETS:
        if not feather.exists():
            print(f"{name:10s}MISSING"); continue
        df = load_indicators(feather, start, end, iv_pr, hv_pr, vvix_pr)
        if len(df) < 1000:
            print(f"{name:10s}TOO SHORT"); continue
        trades = simulate(df)
        if trades.empty:
            print(f"{name:10s}NO TRADES"); continue
        m = annual_metrics(daily_pnl(trades))
        window = f"{df.index.min().date()}->{df.index.max().date()}"
        print(f"{name:10s}{window:27s}{len(trades):>8d}{m['sharpe']:>8.3f}"
              f"{m['max_dd']:>9.2%}{m['total_return']:>9.2%}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
