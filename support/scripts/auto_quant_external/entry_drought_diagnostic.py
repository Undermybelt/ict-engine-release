"""
entry_drought_diagnostic.py — diagnose why trend candidates stop firing entries
after mid-2023 in the 8Y NQ/USD 15m backtest.

Slice 95 evidence:
- TrendPullbackDense15m last entry: 2023-06-01 (out of 8Y window 2018-01 to 2025-12)
- PersistenceClusterDense15m last entry: 2022-11-07
- LiquiditySweepReclaim15mWide last entry: 2023-05-04
- VRPCompression15m last entry: 2023-09-08

These are different stop dates per candidate, suggesting candidate-specific
gate failures rather than a global data issue. This script reproduces each
candidate's entry conditions on the live NQ 15m feather, computes the boolean
for each individual gate per bar, and reports the monthly fraction of bars
meeting each gate over the full 8Y. The gate whose fraction collapses near
the candidate's last-entry date is the binding constraint that needs
re-tuning.

Output: per-candidate monthly tables showing the fraction of 15m bars in
each month that meet each gate, plus the final "all gates met" fraction.
"""
from __future__ import annotations

from pathlib import Path

import numpy as np
import pandas as pd

NQ_15M_FEATHER = Path("user_data/data/NQ_USD-15m.feather")
NQ_1H_FEATHER = Path("user_data/data/NQ_USD-1h.feather")
NQ_4H_FEATHER = Path("user_data/data/NQ_USD-4h.feather")


def load_feather(path: Path) -> pd.DataFrame:
    df = pd.read_feather(path)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    return df.set_index("date").sort_index()


def ema(series: pd.Series, period: int) -> pd.Series:
    return series.ewm(span=period, adjust=False).mean()


def rsi(close: pd.Series, period: int = 14) -> pd.Series:
    delta = close.diff()
    gain = delta.clip(lower=0).ewm(alpha=1 / period, adjust=False).mean()
    loss = (-delta.clip(upper=0)).ewm(alpha=1 / period, adjust=False).mean()
    rs = gain / loss.replace(0, np.nan)
    return 100 - (100 / (1 + rs))


def atr(df: pd.DataFrame, period: int = 14) -> pd.Series:
    high_low = df["high"] - df["low"]
    high_close = (df["high"] - df["close"].shift(1)).abs()
    low_close = (df["low"] - df["close"].shift(1)).abs()
    true_range = pd.concat([high_low, high_close, low_close], axis=1).max(axis=1)
    return true_range.ewm(alpha=1 / period, adjust=False).mean()


def merge_higher_tf(base: pd.DataFrame, higher: pd.DataFrame, suffix: str) -> pd.DataFrame:
    h = higher.copy()
    h["ema_fast"] = ema(h["close"], 21)
    h["ema_slow"] = ema(h["close"], 89)
    out = base.copy()
    out[f"ema_fast_{suffix}"] = h["ema_fast"].reindex(base.index, method="ffill")
    out[f"ema_slow_{suffix}"] = h["ema_slow"].reindex(base.index, method="ffill")
    return out


def trend_pullback_dense_gates(df: pd.DataFrame) -> dict[str, pd.Series]:
    df = df.copy()
    df["ema21"] = ema(df["close"], 21)
    df["ema89"] = ema(df["close"], 89)
    df["rsi"] = rsi(df["close"], 14)
    df["atr"] = atr(df, 14)
    df["near_ema21"] = (df["close"] - df["ema21"]).abs() / df["atr"]
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour
    return {
        "liquid_window": (df["hour_utc"] >= 8) & (df["hour_utc"] <= 23),
        "trend_or": (df["ema_fast_4h"] > df["ema_slow_4h"])
        | (df["ema_fast_1h"] > df["ema_slow_1h"])
        | ((df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])),
        "pullback_zone": df["near_ema21"] <= 2.4,
        "reacceleration": df["body_green"] | (df["close"] > df["close"].shift(1)),
        "not_exhausted": (df["rsi"] >= 35) & (df["rsi"] <= 74),
    }


def liquidity_sweep_reclaim_15m_wide_gates(df: pd.DataFrame) -> dict[str, pd.Series]:
    df = df.copy()
    df["ema21"] = ema(df["close"], 21)
    df["ema89"] = ema(df["close"], 89)
    df["atr"] = atr(df, 14)
    df["low_12bar"] = df["low"].rolling(12).min().shift(1)
    df["low_2bar"] = df["low"].rolling(2).min()
    df["sweep_below"] = df["low_2bar"] < df["low_12bar"]
    df["reclaim_close"] = df["close"] > df["low_12bar"]
    df["body_strength"] = (df["close"] - df["open"]) / df["atr"]
    df["hour_utc"] = df.index.hour
    return {
        "liquid_window": (df["hour_utc"] >= 8) & (df["hour_utc"] <= 22),
        "trend_or": (df["ema_fast_4h"] > df["ema_slow_4h"])
        | (df["ema_fast_1h"] > df["ema_slow_1h"]),
        "sweep_and_reclaim": df["sweep_below"] & df["reclaim_close"],
        "body_strength_25": df["body_strength"] > 0.25,
    }


def persistence_cluster_dense_15m_gates(df: pd.DataFrame) -> dict[str, pd.Series]:
    df = df.copy()
    df["ema13"] = ema(df["close"], 13)
    df["ema34"] = ema(df["close"], 34)
    df["ema89"] = ema(df["close"], 89)
    df["rsi"] = rsi(df["close"], 14)
    df["atr"] = atr(df, 14)
    df["ema13_slope"] = df["ema13"].diff(3) / df["atr"]
    df["above_ema13_count"] = (df["close"] > df["ema13"]).rolling(5).sum()
    df["hour_utc"] = df.index.hour
    return {
        "liquid_window": (df["hour_utc"] >= 8) & (df["hour_utc"] <= 23),
        "trend_or": (df["ema_fast_4h"] > df["ema_slow_4h"])
        | (df["ema_fast_1h"] > df["ema_slow_1h"])
        | ((df["ema13"] > df["ema34"]) & (df["ema34"] > df["ema89"])),
        "persistence": (df["above_ema13_count"] >= 2) & (df["ema13_slope"] > -0.12),
        "not_exhausted": (df["rsi"] >= 43) & (df["rsi"] <= 78),
    }


def report(name: str, gates: dict[str, pd.Series]) -> None:
    df = pd.DataFrame(gates).fillna(False)
    df["all_gates"] = df.all(axis=1)
    monthly = df.resample("MS").mean() * 100.0
    print("=" * 110)
    print(f"{name}: per-month fraction (%) of 15m bars meeting each gate")
    print("=" * 110)
    pd.set_option("display.max_columns", None)
    pd.set_option("display.width", 200)
    monthly = monthly.copy()
    monthly.index = monthly.index.strftime("%Y-%m")
    print(monthly.round(1).to_string())
    print()


def main() -> int:
    base = load_feather(NQ_15M_FEATHER)
    h1 = load_feather(NQ_1H_FEATHER)
    h4 = load_feather(NQ_4H_FEATHER)
    df = merge_higher_tf(base, h1, "1h")
    df = merge_higher_tf(df, h4, "4h")
    df = df.loc["2018-01-01":"2025-12-31"]

    report("TomacNQ_RegimeTrendPullbackDense15m", trend_pullback_dense_gates(df))
    report("TomacNQ_RegimeLiquiditySweepReclaim15mWide", liquidity_sweep_reclaim_15m_wide_gates(df))
    report("TomacNQ_RegimePersistenceClusterDense15m", persistence_cluster_dense_15m_gates(df))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
