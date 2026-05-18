"""
pandas_alt_backtest.py — minimal pandas bar-by-bar backtester that bypasses
freqtrade's internal data pipeline.

Slice 100-102 established that the trend-pullback strategies' entry conditions
are met ~24% of 15m bars throughout 2018-2025 per pandas reproduction, but
freqtrade fires zero entries in 2024-2025 across multiple variants
(Original / Simple15m / NoRSI15m). The drought is a freqtrade-internal
artifact, not a strategy-gate artifact. This script tests that conclusion
directly: load the NQ 15m feather, compute the same indicators in pandas,
simulate max_open_trades=1 entry / stoploss / trailing-stop / exit-signal
logic bar-by-bar, output the trade list. If the pandas backtester produces
healthy 2024-2025 trade counts, the freqtrade-side drought is confirmed
and we now have a clean alternative harness for evaluating the candidates.

The simulated strategy is `TomacNQ_RegimeTrendPullbackNoRSI15m`:
- timeframe 15m, no informatives
- enter long when (long_trend OR local_trend) AND pullback_zone AND
  reacceleration AND liquid_window
- exit when close < ema200 (regime_break) OR stoploss -2.2% OR trailing
  stop (4 ATR after price moves +1% from entry)
- max_open_trades=1
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

NQ_15M_FEATHER = Path("user_data/data/NQ_USD-15m.feather")
START = pd.Timestamp("2018-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.004


def load_indicators() -> pd.DataFrame:
    df = pd.read_feather(NQ_15M_FEATHER)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index()
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
    return df.loc[START:END]


def simulate(df: pd.DataFrame) -> pd.DataFrame:
    closes = df["close"].to_numpy()
    highs = df["high"].to_numpy()
    lows = df["low"].to_numpy()
    entry_signal = df["entry_signal"].to_numpy()
    regime_break = df["regime_break"].to_numpy()
    timestamps = df.index.to_numpy()

    trades: list[dict] = []
    in_position = False
    entry_idx = -1
    entry_price = 0.0
    peak_price = 0.0
    trailing_active = False

    for i in range(len(df)):
        if not in_position:
            if entry_signal[i]:
                in_position = True
                entry_idx = i
                entry_price = closes[i]
                peak_price = closes[i]
                trailing_active = False
            continue

        cur_close = closes[i]
        peak_price = max(peak_price, highs[i])
        gain_from_entry = peak_price / entry_price - 1.0
        if not trailing_active and gain_from_entry >= TRAILING_OFFSET:
            trailing_active = True

        stoploss_price = entry_price * (1.0 + STOPLOSS)
        trailing_price = peak_price * (1.0 - TRAILING_STOP) if trailing_active else 0.0
        effective_stop = max(stoploss_price, trailing_price)

        exit_reason = None
        exit_price = cur_close
        if lows[i] <= effective_stop:
            exit_reason = "trailing_or_stop"
            exit_price = effective_stop
        elif regime_break[i]:
            exit_reason = "regime_break"
            exit_price = cur_close

        if exit_reason is not None:
            profit_ratio = exit_price / entry_price - 1.0
            trades.append(
                {
                    "open_date": pd.Timestamp(timestamps[entry_idx]),
                    "close_date": pd.Timestamp(timestamps[i]),
                    "open_price": entry_price,
                    "close_price": exit_price,
                    "profit_ratio": profit_ratio,
                    "exit_reason": exit_reason,
                    "bars_held": i - entry_idx,
                }
            )
            in_position = False
            entry_idx = -1
            entry_price = 0.0
            peak_price = 0.0
            trailing_active = False

    return pd.DataFrame(trades)


def metrics(trades: pd.DataFrame) -> dict[str, float]:
    if trades.empty:
        return {"trades": 0}
    n = len(trades)
    returns = trades["profit_ratio"]
    cum = (1.0 + returns).cumprod()
    total_return = float(cum.iloc[-1] - 1.0)
    sharpe = returns.mean() / returns.std() if returns.std() > 0 else 0.0
    win_rate = (returns > 0).mean()
    gross_win = returns[returns > 0].sum()
    gross_loss = -returns[returns < 0].sum()
    pf = gross_win / gross_loss if gross_loss > 1e-9 else float("inf") if gross_win > 0 else 0.0
    running_max = cum.cummax()
    dd = (cum / running_max - 1.0).min()
    return {
        "trades": int(n),
        "total_return": total_return,
        "sharpe_per_trade": float(sharpe),
        "win_rate": float(win_rate),
        "profit_factor": float(pf),
        "max_drawdown": float(dd),
        "first_open": str(trades["open_date"].min()),
        "last_close": str(trades["close_date"].max()),
    }


def main() -> int:
    df = load_indicators()
    print(f"loaded {len(df)} 15m bars from {df.index.min()} to {df.index.max()}")
    print(f"entry_signal active on {df['entry_signal'].sum():,} bars ({df['entry_signal'].mean()*100:.1f}%)")
    print()
    trades = simulate(df)
    if trades.empty:
        print("no trades simulated")
        return 0
    print("=" * 80)
    print("Pandas alt-backtest results — TomacNQ_RegimeTrendPullbackNoRSI15m equivalent")
    print("=" * 80)
    m = metrics(trades)
    for k, v in m.items():
        if isinstance(v, float):
            print(f"  {k:20s}: {v:+.4f}")
        else:
            print(f"  {k:20s}: {v}")
    print()
    print("Year-by-year trade distribution:")
    trades["year"] = trades["open_date"].dt.year
    yc = trades.groupby("year").size()
    for y, c in yc.items():
        print(f"  {y}: {c} trades")
    print()
    print("Year-by-year mean profit_ratio:")
    yp = trades.groupby("year")["profit_ratio"].mean() * 100
    for y, p in yp.items():
        print(f"  {y}: {p:+.4f}% per trade")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
