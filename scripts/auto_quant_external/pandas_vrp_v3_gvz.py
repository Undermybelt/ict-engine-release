"""
pandas_vrp_v3_gvz.py — Slice 114. Test GVZ (gold's VIX) as 4th vol-regime
gate on top of VRPCompression V2 (already has QQQ IV/HV + VVIX gates).

Hypothesis: cross-asset vol regime confirmation should add diversification.
When equity vol AND gold vol are BOTH compressed, the global vol regime is
deeply calm — entries should be even higher quality. If it doesn't help,
that's also informative — it means equity vol gates already capture all
the relevant regime signal and gold vol is redundant.

SKEW (CBOE) was the originally-intended target but IBKR returned IP-conflict
and yfinance/PyPI is unreachable. GVZ is the cleanest available
cross-asset substitute (different asset class, same data shape).

Variants tested:
- V2 baseline (no GVZ gate; VVIX<0.40 only)
- V3 with GVZ<0.60 (very loose)
- V3 with GVZ<0.50 (loose)
- V3 with GVZ<0.40 (matches VVIX)
- V3 with GVZ<0.30 (tight)
- V3-alt: GVZ<0.40 INSTEAD of VVIX<0.40 (test if GVZ alone >  VVIX alone)
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

VVIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vvix.1d.10y.csv")
GVZ_CSV = Path("/tmp/ict-engine-ibkr-probe/gvz.1d.10y.csv")


def load_close_series(csv_path: Path) -> pd.Series:
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def add_vol_indices(df: pd.DataFrame) -> pd.DataFrame:
    vvix = load_close_series(VVIX_CSV)
    gvz = load_close_series(GVZ_CSV)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)
    gvz_pr = gvz.rolling(252, min_periods=128).rank(pct=True)
    candle_dates = df.index.normalize()
    df["vvix_pct_rank_252"] = pd.Series(candle_dates.map(vvix_pr), index=df.index).ffill()
    df["gvz_pct_rank_252"] = pd.Series(candle_dates.map(gvz_pr), index=df.index).ffill()
    return df


def build_entry(df, vvix_thr, gvz_thr):
    base = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & (df["iv_pct_rank_252"] < 0.30)
        & (df["hv_pct_rank_252"] < 0.30)
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    if vvix_thr is not None:
        base = base & (df["vvix_pct_rank_252"] < vvix_thr)
    if gvz_thr is not None:
        base = base & (df["gvz_pct_rank_252"] < gvz_thr)
    return base


def build_exit(df):
    return (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])


def walk_forward_summary(trades, label):
    if trades.empty:
        return {"label": label, "trades": 0, "wf_median": 0.0, "wf_pos_pct": 0.0, "wf_worst_dd": 0.0}
    trades = trades.copy()
    cur = pd.Timestamp("2020-01-01", tz="UTC")
    end = pd.Timestamp("2026-01-01", tz="UTC")
    sharpes, dds = [], []
    while cur < end:
        nxt = cur + pd.DateOffset(months=6)
        wt = trades[(trades["open_date"] >= cur) & (trades["open_date"] < nxt)]
        if len(wt) >= 5:
            m = annual_metrics(daily_pnl(wt))
            sharpes.append(m["sharpe"])
            dds.append(m["max_dd"])
        cur = nxt
    if not sharpes:
        return {"label": label, "trades": int(len(trades)), "wf_median": 0.0,
                "wf_pos_pct": 0.0, "wf_worst_dd": 0.0}
    return {
        "label": label, "trades": int(len(trades)),
        "wf_median": float(np.median(sharpes)),
        "wf_pos_pct": float(sum(1 for s in sharpes if s > 0) / len(sharpes)),
        "wf_worst_dd": float(min(dds)),
    }


def main() -> int:
    df = load_indicators_with_vol()
    df = add_vol_indices(df)
    print(f"loaded {len(df)} 15m bars; vvix coverage "
          f"{df['vvix_pct_rank_252'].notna().sum()}, gvz coverage "
          f"{df['gvz_pct_rank_252'].notna().sum()}")
    print()

    variants = [
        ("V2 baseline (VVIX<0.40 only)",     0.40, None),
        ("V3 VVIX<0.40 + GVZ<0.60",          0.40, 0.60),
        ("V3 VVIX<0.40 + GVZ<0.50",          0.40, 0.50),
        ("V3 VVIX<0.40 + GVZ<0.40",          0.40, 0.40),
        ("V3 VVIX<0.40 + GVZ<0.30",          0.40, 0.30),
        ("V3-alt GVZ<0.40 (no VVIX)",        None, 0.40),
        ("V3-alt GVZ<0.50 (no VVIX)",        None, 0.50),
    ]

    df["exit_signal"] = build_exit(df)
    rows = []
    wf_rows = []
    for label, vthr, gthr in variants:
        df["entry_signal"] = build_entry(df, vthr, gthr)
        n_signal = int(df["entry_signal"].sum())
        trades = simulate(df)
        m = annual_metrics(daily_pnl(trades))
        rows.append({
            "variant": label, "signal_bars": n_signal,
            "trades": int(len(trades)),
            "sharpe": m["sharpe"], "sortino": m["sortino"],
            "max_dd": m["max_dd"], "total": m["total_return"],
        })
        wf_rows.append(walk_forward_summary(trades, label))

    print("=" * 105)
    print("VRPCompression V3 (GVZ as 4th vol-axis) — 8Y aggregate")
    print("=" * 105)
    print(f"{'variant':36s}{'sig.bars':>10s}{'trades':>8s}{'sharpe':>8s}"
          f"{'sortino':>9s}{'maxdd':>9s}{'total':>9s}")
    print("-" * 105)
    for r in rows:
        print(f"{r['variant']:36s}{r['signal_bars']:>10d}{r['trades']:>8d}"
              f"{r['sharpe']:>8.3f}{r['sortino']:>9.3f}"
              f"{r['max_dd']:>9.2%}{r['total']:>9.2%}")
    print()

    print("=" * 105)
    print("Walk-forward 6M distribution")
    print("=" * 105)
    print(f"{'variant':36s}{'trades':>8s}{'wf_med':>9s}{'wf_pos%':>10s}{'wf_worst_dd':>13s}")
    print("-" * 105)
    for w in wf_rows:
        print(f"{w['label']:36s}{w['trades']:>8d}{w['wf_median']:>9.3f}"
              f"{w['wf_pos_pct']*100:>9.1f}%{w['wf_worst_dd']*100:>12.2f}%")
    print()

    print("Interpretation:")
    base = rows[0]
    for r in rows[1:]:
        delta_sharpe = r["sharpe"] - base["sharpe"]
        delta_trades = r["trades"] - base["trades"]
        verdict = ("LIFT" if delta_sharpe > 0.2 and r["trades"] >= 200
                   else "neutral" if abs(delta_sharpe) <= 0.2
                   else "drop")
        print(f"  {r['variant']:36s} ΔSharpe={delta_sharpe:+.3f}, "
              f"Δtrades={delta_trades:+d}, verdict={verdict}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
