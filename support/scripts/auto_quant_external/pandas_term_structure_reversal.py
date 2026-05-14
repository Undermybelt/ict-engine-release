"""
pandas_term_structure_reversal.py — Slice 116. Build a NEW factor family
genuinely orthogonal to VRPCompression by construction.

VRP fires during compressed-vol regimes (IV/HV both percentile<0.30).
This factor fires AFTER vol shocks, during the normalization phase.
Pairwise correlation should be near zero by mechanism.

Hypothesis (well-known in vol literature):
- VIX9D/VIX3M term inverted (>1.05) = front-end stress, often overshoot
- When ratio re-crosses back below 1.00 (return to contango), the spike
  is over and equity rallies as vol sellers re-engage
- Conditioned on NQ above EMA200 (regime not broken into bear)

Entry: today VIX9D/VIX3M crosses below 1.00 (yesterday >= 1.00) AND any of
the prior 5 days had ratio > 1.05 (true backwardation regime) AND NQ
above EMA200.
Exit: EMA89 break OR ratio drops below 0.92 (deep contango = vol-selling
overdone) OR stop -2.5%.

If this works AND has low correlation to VRP V2, it's a true diversifier
worth basket-combining.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

NQ_15M = Path("user_data/data/NQ_USD-15m.feather")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
QQQ_IV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.iv.1d.10y.csv")
QQQ_HV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.hv.1d.10y.csv")
VVIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vvix.1d.10y.csv")
START = pd.Timestamp("2018-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
TRADING_DAYS = 252.0

STOPLOSS = -0.025
TRAILING_OFFSET = 0.012
TRAILING_STOP = 0.006


def load_close_series(csv_path):
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def build_indicators():
    df = pd.read_feather(NQ_15M)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index().loc[START:END]
    df["ema21"] = df["close"].ewm(span=21, adjust=False).mean()
    df["ema89"] = df["close"].ewm(span=89, adjust=False).mean()
    df["ema200"] = df["close"].ewm(span=200, adjust=False).mean()
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour

    vix9d = load_close_series(VIX9D_CSV)
    vix3m = load_close_series(VIX3M_CSV)
    common = vix9d.index.intersection(vix3m.index)
    ratio = (vix9d.loc[common] / vix3m.loc[common].where(vix3m.loc[common] > 1e-9))

    backward_5d = (ratio > 1.05).rolling(5, min_periods=1).max().astype(bool)
    cross_below_100 = (ratio < 1.00) & (ratio.shift(1) >= 1.00)
    deep_contango = ratio < 0.92

    candle_dates = df.index.normalize()
    df["term_ratio"] = pd.Series(candle_dates.map(ratio), index=df.index).ffill()
    df["had_backward_5d"] = pd.Series(candle_dates.map(backward_5d), index=df.index).ffill().fillna(False).astype(bool)
    df["term_normalize_today"] = pd.Series(candle_dates.map(cross_below_100), index=df.index).ffill().fillna(False).astype(bool)
    df["deep_contango"] = pd.Series(candle_dates.map(deep_contango), index=df.index).ffill().fillna(False).astype(bool)

    df["liquid_window"] = (df["hour_utc"] >= 13) & (df["hour_utc"] <= 21)
    df["above_ema200"] = df["close"] > df["ema200"]

    df["entry_signal"] = (
        df["liquid_window"]
        & df["term_normalize_today"]
        & df["had_backward_5d"]
        & df["above_ema200"]
        & df["body_green"]
    )
    df["exit_signal"] = (
        (df["close"] < df["ema89"])
        | df["deep_contango"]
    )
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
    if d.empty or d.std() == 0: return {"sharpe": 0.0, "sortino": 0.0, "max_dd": 0.0, "total": 0.0}
    sharpe = (d.mean() / d.std()) * np.sqrt(TRADING_DAYS) if d.std() > 0 else 0.0
    downside = d[d < 0]
    sortino = (d.mean() / downside.std()) * np.sqrt(TRADING_DAYS) if (len(downside) > 1 and downside.std() > 0) else 0.0
    cum = (1.0 + d).cumprod(); dd = float((cum / cum.cummax() - 1.0).min())
    return {"sharpe": float(sharpe), "sortino": float(sortino),
            "max_dd": dd, "total": float(cum.iloc[-1] - 1.0)}


def vrp_v2_trades(df_base):
    """Re-derive VRP V2 trades for correlation comparison."""
    iv = load_close_series(QQQ_IV_CSV)
    hv = load_close_series(QQQ_HV_CSV)
    vvix = load_close_series(VVIX_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)
    df = df_base.copy()
    candle_dates = df.index.normalize()
    df["iv_pct_rank_252"] = pd.Series(candle_dates.map(iv_pr), index=df.index).ffill()
    df["hv_pct_rank_252"] = pd.Series(candle_dates.map(hv_pr), index=df.index).ffill()
    df["vvix_pct_rank_252"] = pd.Series(candle_dates.map(vvix_pr), index=df.index).ffill()
    df["ema600"] = df["close"].ewm(span=600, adjust=False).mean()
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])
    df["entry_signal"] = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & (df["iv_pct_rank_252"] < 0.30)
        & (df["hv_pct_rank_252"] < 0.30)
        & (df["vvix_pct_rank_252"] < 0.40)
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    df["exit_signal"] = (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])
    return simulate(df)


def main() -> int:
    df = build_indicators()
    print(f"loaded {len(df)} 15m bars")
    print(f"backward-5d days: {df['had_backward_5d'].sum()}")
    print(f"term-normalize days: {df['term_normalize_today'].sum()}")
    print(f"entry signal bars: {df['entry_signal'].sum()}")
    print()

    print("Simulating term-structure reversal factor...")
    tsr_trades = simulate(df)
    print(f"  trades: {len(tsr_trades)}")
    if tsr_trades.empty:
        print("ERROR: no trades")
        return 1
    m_tsr = annual_metrics(daily_pnl(tsr_trades))
    print(f"  Sharpe: {m_tsr['sharpe']:+.3f}, Sortino: {m_tsr['sortino']:+.3f}, "
          f"MaxDD: {m_tsr['max_dd']:+.2%}, Total: {m_tsr['total']:+.2%}")
    print()

    print("Simulating VRP V2 (for correlation reference)...")
    vrp_trades = vrp_v2_trades(df)
    print(f"  trades: {len(vrp_trades)}")
    m_vrp = annual_metrics(daily_pnl(vrp_trades))
    print(f"  Sharpe: {m_vrp['sharpe']:+.3f}, MaxDD: {m_vrp['max_dd']:+.2%}, Total: {m_vrp['total']:+.2%}")
    print()

    # Correlation
    tsr_pnl = daily_pnl(tsr_trades)
    vrp_pnl = daily_pnl(vrp_trades)
    all_dates = sorted(set(tsr_pnl.index) | set(vrp_pnl.index))
    if all_dates:
        idx = pd.date_range(min(all_dates), max(all_dates), freq="D", tz="UTC")
        tsr_r = tsr_pnl.reindex(idx).fillna(0.0)
        vrp_r = vrp_pnl.reindex(idx).fillna(0.0)
        corr = tsr_r.corr(vrp_r)
        print(f"Pairwise daily-PnL correlation (TermStructureReversal vs VRP V2): {corr:+.3f}")

        # Equal-weight basket
        eq = (tsr_r + vrp_r) / 2.0
        m_eq = annual_metrics(eq)
        print()
        print(f"Equal-weight 2-strategy basket:")
        print(f"  Sharpe: {m_eq['sharpe']:+.3f}, MaxDD: {m_eq['max_dd']:+.2%}, Total: {m_eq['total']:+.2%}")

        # Inverse-vol weight
        std_tsr = tsr_r.std(); std_vrp = vrp_r.std()
        if std_tsr > 0 and std_vrp > 0:
            w_tsr = (1/std_tsr) / ((1/std_tsr) + (1/std_vrp))
            w_vrp = 1.0 - w_tsr
            iv_basket = w_tsr * tsr_r + w_vrp * vrp_r
            m_iv = annual_metrics(iv_basket)
            print(f"Inverse-vol basket (weights TSR={w_tsr:.3f}, VRP={w_vrp:.3f}):")
            print(f"  Sharpe: {m_iv['sharpe']:+.3f}, MaxDD: {m_iv['max_dd']:+.2%}, Total: {m_iv['total']:+.2%}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
