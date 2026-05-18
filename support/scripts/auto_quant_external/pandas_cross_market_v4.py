"""
pandas_cross_market_v4.py — apply the drought-fixed pandas TrendPullback-NoRSI
strategy + V3 regime conditioning across SPY/IWM/DIA/GLD/NQ 15m markets.

Slice 104's drought-fixed full-8Y NQ V3 conditional Sharpe was 1.48. This
script tests whether the same methodology generalizes across markets:
- per-market: load 15m feather, compute strategy indicators, simulate trades
- attribute trades by NQ-derived daily regime + VIX9D/VIX3M term-structure
- apply train-derived V3 deny rules (use NQ-derived rules from Slice 99/104
  rather than re-derive per-market — the regime classifier itself is anchored
  on NQ + VIX, not the traded asset)
- compare per-market Sharpe / drawdown / total return

The cross-market 15m feathers cover only 1Y RTH (May 2025 - May 2026, ~6,490
bars per market). Cross-market regime conditioning is therefore conducted
across that 1Y window. Mostly-TrendingCalm sample but useful for testing
whether the strategy fires entries cleanly cross-market and whether the
classifier rules still help on a fresh dataset.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table

DATA_DIR = Path("user_data/data")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
TRADING_DAYS = 252.0

NQ_DERIVED_DENY: set[tuple[str, str]] = {
    ("BearishStress", "Backwardation"),
    ("BearishStress", "FlatToBackward"),
    ("ChopRange", "Contango"),
    ("ChopRange", "DeepContango"),
    ("TrendingCalm", "Contango"),
    ("TrendingNervous", "Backwardation"),
    ("TrendingNervous", "Contango"),
}

MARKETS = [
    ("NQ/USD", DATA_DIR / "NQ_USD-15m.feather", pd.Timestamp("2018-01-01", tz="UTC"), pd.Timestamp("2025-12-31", tz="UTC")),
    ("SPY/USD", DATA_DIR / "SPY_USD-15m.feather", None, None),
    ("IWM/USD", DATA_DIR / "IWM_USD-15m.feather", None, None),
    ("DIA/USD", DATA_DIR / "DIA_USD-15m.feather", None, None),
    ("GLD/USD", DATA_DIR / "GLD_USD-15m.feather", None, None),
]

STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.004


def load_term_structure() -> pd.Series:
    def load(p: Path) -> pd.Series:
        df = pd.read_csv(p)
        df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
        df = df.dropna(subset=["ts", "close"])
        df["date"] = df["ts"].dt.normalize()
        s = df.set_index("date")["close"].astype(float)
        return s[~s.index.duplicated(keep="last")].sort_index()
    vix9d = load(VIX9D_CSV)
    vix3m = load(VIX3M_CSV)
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


def load_indicators(feather_path: Path, start, end) -> pd.DataFrame:
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
    hl = df["high"] - df["low"]
    hc = (df["high"] - df["close"].shift(1)).abs()
    lc = (df["low"] - df["close"].shift(1)).abs()
    df["atr"] = pd.concat([hl, hc, lc], axis=1).max(axis=1).ewm(alpha=1 / 14, adjust=False).mean()
    df["near_ema21"] = (df["close"] - df["ema21"]).abs() / df["atr"]
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour
    df["liquid_window"] = (df["hour_utc"] >= 8) & (df["hour_utc"] <= 23)
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])
    df["pullback_zone"] = df["near_ema21"] <= 2.4
    df["reacceleration"] = df["body_green"] | (df["close"] > df["close"].shift(1))
    df["entry_signal"] = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & df["pullback_zone"]
        & df["reacceleration"]
    )
    df["regime_break"] = df["close"] < df["ema200"]
    return df


def simulate(df: pd.DataFrame) -> pd.DataFrame:
    closes = df["close"].to_numpy()
    highs = df["high"].to_numpy()
    lows = df["low"].to_numpy()
    es = df["entry_signal"].to_numpy()
    rb = df["regime_break"].to_numpy()
    timestamps = df.index.to_numpy()

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
        elif rb[i]:
            reason = "regime"
            exit_price = closes[i]
        if reason is not None:
            trades.append({
                "open_date": pd.Timestamp(timestamps[entry_idx]),
                "close_date": pd.Timestamp(timestamps[i]),
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


def main() -> int:
    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term)

    print("=" * 100)
    print("Cross-market drought-fixed pandas backtest with NQ-derived V3 deny rules")
    print("=" * 100)
    print(f"NQ-derived deny cells: {sorted(NQ_DERIVED_DENY)}")
    print()
    print(f"{'market':12s}{'window':25s}{'trades_uncond':>14s}{'trades_cond':>13s}{'sharpe_uncond':>15s}{'sharpe_cond':>13s}{'maxdd_cond':>12s}{'total_cond':>12s}")
    print("-" * 100)

    for name, feather, start, end in MARKETS:
        if not feather.exists():
            print(f"{name:12s}MISSING")
            continue
        df = load_indicators(feather, start, end)
        if len(df) < 800:
            print(f"{name:12s}TOO SHORT ({len(df)} bars)")
            continue
        trades = simulate(df)
        if trades.empty:
            print(f"{name:12s}NO TRADES")
            continue
        trades["entry_date"] = trades["open_date"].dt.normalize()
        trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
        trades["term"] = trades["entry_date"].map(term_class).fillna("unknown")
        deny_mask = trades.apply(lambda r: (r["regime"], r["term"]) in NQ_DERIVED_DENY, axis=1)
        cond = trades[~deny_mask]
        u = annual_metrics(daily_pnl(trades))
        c = annual_metrics(daily_pnl(cond))
        window = f"{df.index.min().date()}->{df.index.max().date()}"
        print(f"{name:12s}{window:25s}{len(trades):>14d}{len(cond):>13d}"
              f"{u['sharpe']:>15.3f}{c['sharpe']:>13.3f}{c['max_dd']:>11.2%}{c['total_return']:>11.2%}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
