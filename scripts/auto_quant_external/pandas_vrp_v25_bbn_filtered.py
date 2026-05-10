"""
pandas_vrp_v25_bbn_filtered.py — Slice 122. Deployable VRP V2.5 = V2 +
BBN bull_score death-zone filter, validated on multi-timeframe test
period 2023-2025.

Slice 121 found bull_score in (0.6, 0.7] is a "death zone" with mean
return -0.0146% across pooled n=976 trades, replicated on all 3 NQ
timeframes (5m/15m/1h). This slice tests whether removing those trades
lifts deployable Sharpe.

Variants:
- V2 baseline
- V2.5a: V2 + deny bull_score in (0.6, 0.7]
- V2.5b: V2 + deny bull_score in (0.6, 0.8] (broader)
- V2.5c: V2 + deny pred_class==up (aggressive: only-trade pred_class in {flat,down,strong_up,crash})
- V2.5d: V2 + only-trade if pred_class in {flat, down} (very aggressive)
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

DATA_DIR = Path("/Users/thrill3r/Auto-Quant/user_data/data")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_IV_CSV = PROBE_DIR / "qqq.iv.1d.10y.csv"
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
BBN_CSV = PROBE_DIR / "slice_118_bbn_predictions.csv"

START = pd.Timestamp("2023-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
TRADING_DAYS = 252.0
STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005

TIMEFRAMES = [
    ("5m", DATA_DIR / "NQ_USD-5m.feather"),
    ("15m", DATA_DIR / "NQ_USD-15m.feather"),
    ("1h", DATA_DIR / "NQ_USD-1h.feather"),
]


def load_close(csv_path):
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def build_indicators(feather_path, iv_pr, hv_pr, vvix_pr, bbn):
    df = pd.read_feather(feather_path)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index().loc[START:END]
    df["ema21"] = df["close"].ewm(span=21, adjust=False).mean()
    df["ema89"] = df["close"].ewm(span=89, adjust=False).mean()
    df["ema200"] = df["close"].ewm(span=200, adjust=False).mean()
    df["ema600"] = df["close"].ewm(span=600, adjust=False).mean()
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour
    cd = df.index.normalize()
    df["iv_pct_rank_252"] = pd.Series(cd.map(iv_pr), index=df.index).ffill()
    df["hv_pct_rank_252"] = pd.Series(cd.map(hv_pr), index=df.index).ffill()
    df["vvix_pct_rank_252"] = pd.Series(cd.map(vvix_pr), index=df.index).ffill()
    df["liquid_window"] = (df["hour_utc"] >= 13) & (df["hour_utc"] <= 21)
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])

    # Attach BBN posteriors
    for col in ["p_crash", "p_down", "p_flat", "p_up", "p_strong_up", "pred_class"]:
        df[col] = pd.Series(cd.map(bbn[col]), index=df.index).ffill()
    df["bull_score"] = df["p_up"] + df["p_strong_up"]

    df["v2_entry"] = (
        df["liquid_window"]
        & (df["long_trend"] | df["local_trend"])
        & (df["iv_pct_rank_252"] < 0.30)
        & (df["hv_pct_rank_252"] < 0.30)
        & (df["vvix_pct_rank_252"] < 0.40)
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    df["exit_signal"] = (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])
    return df


def simulate_with_filter(df, entry_filter):
    closes = df["close"].to_numpy(); highs = df["high"].to_numpy(); lows = df["low"].to_numpy()
    es = entry_filter.to_numpy(); xs = df["exit_signal"].to_numpy()
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
    if d.empty or d.std() == 0: return {"sharpe": 0.0, "max_dd": 0.0, "total": 0.0}
    sharpe = (d.mean() / d.std()) * np.sqrt(TRADING_DAYS) if d.std() > 0 else 0.0
    cum = (1.0 + d).cumprod(); dd = float((cum / cum.cummax() - 1.0).min())
    return {"sharpe": float(sharpe), "max_dd": dd, "total": float(cum.iloc[-1] - 1.0)}


def walk_forward(trades):
    if trades.empty: return None
    cur = pd.Timestamp("2023-01-01", tz="UTC"); end = pd.Timestamp("2026-01-01", tz="UTC")
    sharpes = []
    while cur < end:
        nxt = cur + pd.DateOffset(months=6)
        wt = trades[(trades["open_date"] >= cur) & (trades["open_date"] < nxt)]
        if len(wt) >= 5:
            m = annual_metrics(daily_pnl(wt))
            sharpes.append(m["sharpe"])
        cur = nxt
    if not sharpes: return None
    return {"wf_med": float(np.median(sharpes)),
            "wf_pos": float(sum(1 for s in sharpes if s > 0) / len(sharpes)),
            "wf_n": len(sharpes)}


def main() -> int:
    iv = load_close(QQQ_IV_CSV); hv = load_close(QQQ_HV_CSV); vvix = load_close(VVIX_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)
    bbn = pd.read_csv(BBN_CSV, index_col=0, parse_dates=True)
    bbn.index = pd.to_datetime(bbn.index, utc=True).normalize()
    bbn = bbn[~bbn.index.duplicated(keep="last")]

    print(f"VRP V2.5 BBN-filtered variants — test period 2023-2025")
    print(f"BBN trained on 2018-2022, applied as OOS conditioning on V2 entries\n")

    headers = (f"{'tf':>4s} {'variant':28s}{'trades':>8s}{'sharpe':>8s}{'maxdd':>9s}"
               f"{'total':>9s}{'wf_med':>9s}{'wf_pos%':>10s}")
    print(headers)
    print("-" * 92)

    for tf, feather in TIMEFRAMES:
        df = build_indicators(feather, iv_pr, hv_pr, vvix_pr, bbn)
        bbn_avail = df["p_up"].notna()
        bull = df["bull_score"]
        pred = df["pred_class"]

        variants = {
            "V2 baseline":               df["v2_entry"],
            "V2.5a deny bull (0.6,0.7]": df["v2_entry"] & ~(bbn_avail & (bull > 0.6) & (bull <= 0.7)),
            "V2.5b deny bull (0.6,0.8]": df["v2_entry"] & ~(bbn_avail & (bull > 0.6) & (bull <= 0.8)),
            "V2.5c deny pred_class=up":  df["v2_entry"] & ~(bbn_avail & (pred == 3)),
            "V2.5d only flat/down":      df["v2_entry"] & bbn_avail & ((pred == 1) | (pred == 2)),
        }

        for label, mask in variants.items():
            trades = simulate_with_filter(df, mask)
            if trades.empty:
                print(f"{tf:>4s} {label:28s}{0:>8d}{0:>8.3f}{0:>9.2%}{0:>9.2%}{0:>9.3f}{0:>9.1f}%")
                continue
            m = annual_metrics(daily_pnl(trades))
            wf = walk_forward(trades)
            wf_med = wf["wf_med"] if wf else 0.0
            wf_pos = wf["wf_pos"] * 100 if wf else 0.0
            print(f"{tf:>4s} {label:28s}{len(trades):>8d}{m['sharpe']:>8.3f}"
                  f"{m['max_dd']:>9.2%}{m['total']:>9.2%}"
                  f"{wf_med:>9.3f}{wf_pos:>9.1f}%")
        print()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
