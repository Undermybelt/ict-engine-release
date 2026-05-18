"""
pandas_walk_forward_vrp.py — walk-forward 6-month rolling test windows for
VRPCompression to honestly characterize the expected-Sharpe distribution.

Slice 108: VRPCompression 8Y aggregate Sharpe 3.33. Slice 109: best deployable
single config. But aggregate Sharpes can mask volatile per-period performance.
This script splits VRPCompression's 8Y trade history into non-overlapping 6M
test windows (Jan-Jun and Jul-Dec each year) and reports per-window metrics.
The distribution of per-window Sharpes is the honest expected-deployment range.

VRPCompression auto-derived no deny rules (all train cells positive — see
Slice 108), so walk-forward is just resampling the strategy's natural per-
window edge variability.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))
from pandas_vrp_compression import (
    load_indicators_with_vol,
    simulate,
    daily_pnl,
    annual_metrics,
)


def main() -> int:
    df = load_indicators_with_vol()
    trades = simulate(df)
    if trades.empty:
        print("ERROR: no trades")
        return 1
    trades["entry_date"] = trades["open_date"].dt.normalize()
    trades = trades.sort_values("close_date").reset_index(drop=True)
    print(f"total trades: {len(trades)}, span "
          f"{trades['open_date'].min().date()} -> {trades['close_date'].max().date()}")
    print()

    windows: list[tuple[str, pd.Timestamp, pd.Timestamp]] = []
    cur = pd.Timestamp("2020-01-01", tz="UTC")
    end_horizon = pd.Timestamp("2026-01-01", tz="UTC")
    while cur < end_horizon:
        nxt = cur + pd.DateOffset(months=6)
        label = f"{cur.year}-{('H1' if cur.month == 1 else 'H2')}"
        windows.append((label, cur, nxt))
        cur = nxt

    rows: list[dict] = []
    for label, start, end in windows:
        mask = (trades["open_date"] >= start) & (trades["open_date"] < end)
        window_trades = trades[mask]
        if window_trades.empty:
            rows.append({
                "window": label,
                "start": start.date(),
                "end": end.date(),
                "trades": 0,
                "sharpe": 0.0,
                "sortino": 0.0,
                "max_dd": 0.0,
                "total": 0.0,
            })
            continue
        m = annual_metrics(daily_pnl(window_trades))
        rows.append({
            "window": label,
            "start": start.date(),
            "end": end.date(),
            "trades": int(len(window_trades)),
            "sharpe": m["sharpe"],
            "sortino": m["sortino"],
            "max_dd": m["max_dd"],
            "total": m["total_return"],
        })

    print("=" * 90)
    print("VRPCompression walk-forward 6-month rolling test windows")
    print("=" * 90)
    df_rows = pd.DataFrame(rows)
    print(df_rows.to_string(index=False, float_format=lambda x: f"{x:+.4f}"))
    print()

    print("Distribution summary across test windows (excluding zero-trade windows):")
    valid = df_rows[df_rows["trades"] > 0]
    print(f"  windows with trades:        {len(valid)} of {len(df_rows)}")
    print(f"  Sharpe — mean:              {valid['sharpe'].mean():+.3f}")
    print(f"  Sharpe — median:            {valid['sharpe'].median():+.3f}")
    print(f"  Sharpe — std:               {valid['sharpe'].std():.3f}")
    print(f"  Sharpe — min:               {valid['sharpe'].min():+.3f}")
    print(f"  Sharpe — max:               {valid['sharpe'].max():+.3f}")
    print(f"  Sharpe — % positive:        {(valid['sharpe'] > 0).mean()*100:.1f}%")
    print(f"  Sharpe — % > 1.0:           {(valid['sharpe'] > 1.0).mean()*100:.1f}%")
    print(f"  Sharpe — % > 2.0:           {(valid['sharpe'] > 2.0).mean()*100:.1f}%")
    print(f"  trades — mean per window:   {valid['trades'].mean():.1f}")
    print(f"  trades — min per window:    {valid['trades'].min()}")
    print(f"  total return — mean:        {valid['total'].mean()*100:+.2f}%")
    print(f"  total return — % positive:  {(valid['total'] > 0).mean()*100:.1f}%")
    print(f"  max_dd — mean:              {valid['max_dd'].mean()*100:+.2f}%")
    print(f"  max_dd — worst:             {valid['max_dd'].min()*100:+.2f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
