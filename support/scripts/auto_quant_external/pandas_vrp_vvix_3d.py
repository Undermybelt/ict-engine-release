"""
pandas_vrp_vvix_3d.py — Slice 113. Test VVIX as a 3rd vol-regime gate on top
of VRPCompression's QQQ IV/HV gates.

Baseline VRPCompression (Slice 108-112): 8Y aggregate Sharpe 3.33, median 6M
Sharpe 3.87, 815 trades, max DD -3.70%.

Hypothesis: VVIX (vol-of-vol) measures expected variance of VIX itself. When
VVIX is also compressed, the regime is "deeply calm" — both vol level and
vol-of-vol low. Adding VVIX < threshold as a 3rd gate may improve Sharpe by
filtering out trades made during quiet-but-fragile regimes (low VIX but high
VVIX = market expects vol shock soon).

Risk: narrowing the gate set always reduces trade count. If trade count drops
below ~250 over 8Y the per-window Sharpe distribution becomes noisy.

Variants tested:
- baseline (IV<0.30, HV<0.30 only)
- VVIX<0.50 (loose 3rd gate)
- VVIX<0.40 (medium 3rd gate)
- VVIX<0.30 (matches IV/HV severity)
- VVIX<0.20 (very tight)
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


def load_close_series(csv_path: Path) -> pd.Series:
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def add_vvix(df: pd.DataFrame) -> pd.DataFrame:
    vvix = load_close_series(VVIX_CSV)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)
    candle_dates = df.index.normalize()
    df["vvix_pct_rank_252"] = pd.Series(candle_dates.map(vvix_pr), index=df.index).ffill()
    return df


def build_entry(df: pd.DataFrame, vvix_threshold: float | None) -> pd.Series:
    base = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & (df["iv_pct_rank_252"] < 0.30)
        & (df["hv_pct_rank_252"] < 0.30)
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    if vvix_threshold is None:
        return base
    return base & (df["vvix_pct_rank_252"] < vvix_threshold)


def build_exit(df: pd.DataFrame) -> pd.Series:
    return (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])


def walk_forward_summary(trades: pd.DataFrame, label: str) -> dict:
    if trades.empty:
        return {"label": label, "trades": 0, "wf_median_sharpe": 0.0,
                "wf_pct_pos": 0.0, "wf_worst_dd": 0.0}
    trades = trades.copy()
    trades["entry_date"] = trades["open_date"].dt.normalize()
    cur = pd.Timestamp("2020-01-01", tz="UTC")
    end = pd.Timestamp("2026-01-01", tz="UTC")
    sharpes = []
    dds = []
    while cur < end:
        nxt = cur + pd.DateOffset(months=6)
        mask = (trades["open_date"] >= cur) & (trades["open_date"] < nxt)
        wt = trades[mask]
        if len(wt) >= 5:
            m = annual_metrics(daily_pnl(wt))
            sharpes.append(m["sharpe"])
            dds.append(m["max_dd"])
        cur = nxt
    if not sharpes:
        return {"label": label, "trades": int(len(trades)), "wf_median_sharpe": 0.0,
                "wf_pct_pos": 0.0, "wf_worst_dd": 0.0}
    return {
        "label": label,
        "trades": int(len(trades)),
        "wf_median_sharpe": float(np.median(sharpes)),
        "wf_pct_pos": float(sum(1 for s in sharpes if s > 0) / len(sharpes)),
        "wf_worst_dd": float(min(dds)),
    }


def main() -> int:
    df = load_indicators_with_vol()
    df = add_vvix(df)
    print(f"loaded {len(df)} 15m bars; vvix coverage "
          f"{df['vvix_pct_rank_252'].notna().sum()} / {len(df)} bars")
    print()

    variants: list[tuple[str, float | None]] = [
        ("baseline (no VVIX gate)", None),
        ("VVIX<0.50 (loose)", 0.50),
        ("VVIX<0.40 (medium)", 0.40),
        ("VVIX<0.30 (matches IV/HV)", 0.30),
        ("VVIX<0.20 (tight)", 0.20),
    ]

    rows = []
    wf_rows = []
    df["exit_signal"] = build_exit(df)
    for label, threshold in variants:
        df["entry_signal"] = build_entry(df, threshold)
        n_signal = int(df["entry_signal"].sum())
        trades = simulate(df)
        m = annual_metrics(daily_pnl(trades))
        rows.append({
            "variant": label,
            "signal_bars": n_signal,
            "trades": int(len(trades)),
            "sharpe": m["sharpe"],
            "sortino": m["sortino"],
            "max_dd": m["max_dd"],
            "total_return": m["total_return"],
        })
        wf_rows.append(walk_forward_summary(trades, label))

    print("=" * 100)
    print("VRPCompression + VVIX 3D — 8Y aggregate metrics")
    print("=" * 100)
    print(f"{'variant':30s}{'sig.bars':>10s}{'trades':>8s}{'sharpe':>8s}"
          f"{'sortino':>9s}{'maxdd':>9s}{'total':>9s}")
    print("-" * 100)
    for r in rows:
        print(f"{r['variant']:30s}{r['signal_bars']:>10d}{r['trades']:>8d}"
              f"{r['sharpe']:>8.3f}{r['sortino']:>9.3f}"
              f"{r['max_dd']:>9.2%}{r['total_return']:>9.2%}")
    print()

    print("=" * 100)
    print("VRPCompression + VVIX 3D — walk-forward 6M distribution")
    print("=" * 100)
    print(f"{'variant':30s}{'trades':>8s}{'wf_median':>11s}"
          f"{'wf_pct_pos':>11s}{'wf_worst_dd':>13s}")
    print("-" * 100)
    for w in wf_rows:
        print(f"{w['label']:30s}{w['trades']:>8d}{w['wf_median_sharpe']:>11.3f}"
              f"{w['wf_pct_pos']*100:>10.1f}%{w['wf_worst_dd']*100:>12.2f}%")
    print()

    print("Interpretation:")
    base = rows[0]
    for r in rows[1:]:
        delta_sharpe = r["sharpe"] - base["sharpe"]
        delta_trades = r["trades"] - base["trades"]
        verdict = ("LIFT" if delta_sharpe > 0.2 and r["trades"] >= 200
                   else "neutral" if abs(delta_sharpe) <= 0.2
                   else "drop")
        print(f"  {r['variant']:30s}: ΔSharpe={delta_sharpe:+.3f}, "
              f"Δtrades={delta_trades:+d}, verdict={verdict}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
