"""
pandas_bbn_diagnostic_multi_tf.py — Slice 121. Validate Slice 120's
anti-momentum hypothesis (return vs bull_score inverted-U) by pooling
VRP V2 trades from NQ 5m + 15m + 1h on test period 2023-2025 (where
BBN posteriors are available).

Slice 120 found |r|<0.10 at n=287 — borderline statistical power. This
slice targets >600 pooled trades by aggregating the 3 timeframes that
all showed structural edge (Slice 115). If the pattern replicates, the
anti-momentum residual is real; if it goes flat, Slice 120 was noise.
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
STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005
CLASS_NAMES = ["crash", "down", "flat", "up", "strong_up"]

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


def build_indicators(feather_path, iv_pr, hv_pr, vvix_pr):
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


def report_bucket(label, trades, edges, key):
    print(f"\n{label}:")
    print(f"  {'bucket':25s}{'n_trades':>10s}{'mean_ret':>11s}{'hit_rate':>11s}{'sample_sh':>11s}")
    print("  " + "-" * 70)
    bins = pd.cut(trades[key], bins=edges, include_lowest=True)
    for interval, group in trades.groupby(bins, observed=True):
        if len(group) == 0: continue
        ret = group["profit_ratio"]
        sh = (ret.mean() / ret.std()) if ret.std() > 0 else 0.0
        print(f"  {str(interval):25s}{len(group):>10d}{ret.mean():>11.4%}"
              f"{(ret > 0).mean():>10.1%}{sh:>11.3f}")


def main() -> int:
    iv = load_close(QQQ_IV_CSV); hv = load_close(QQQ_HV_CSV); vvix = load_close(VVIX_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)

    bbn = pd.read_csv(BBN_CSV, index_col=0, parse_dates=True)
    bbn.index = pd.to_datetime(bbn.index, utc=True).normalize()
    bbn = bbn[~bbn.index.duplicated(keep="last")]

    all_trades = []
    print("Per-timeframe trade counts (test period 2023-2025):")
    for tf, feather in TIMEFRAMES:
        df = build_indicators(feather, iv_pr, hv_pr, vvix_pr)
        trades = simulate(df)
        if trades.empty:
            print(f"  {tf}: 0 trades"); continue
        trades["tf"] = tf
        trades["entry_date"] = trades["open_date"].dt.normalize()
        for col in ["p_crash", "p_down", "p_flat", "p_up", "p_strong_up", "pred_class"]:
            trades[col] = trades["entry_date"].map(bbn[col])
        n_with_bbn = trades["p_up"].notna().sum()
        print(f"  {tf}: {len(trades)} trades, {n_with_bbn} with BBN posterior")
        all_trades.append(trades.dropna(subset=["p_up"]))
    pooled = pd.concat(all_trades, ignore_index=True)
    print(f"\nPooled total: {len(pooled)} trades with BBN posterior")
    if len(pooled) < 200:
        print("ERROR: insufficient pooled sample"); return 1

    pooled["bull_score"] = pooled["p_up"] + pooled["p_strong_up"]
    pooled["bear_score"] = pooled["p_down"] + pooled["p_crash"]

    overall_ret = pooled["profit_ratio"].mean()
    overall_sh = (overall_ret / pooled["profit_ratio"].std()) if pooled["profit_ratio"].std() > 0 else 0.0
    print(f"Overall pooled: mean_ret={overall_ret:.4%}, hit={(pooled['profit_ratio']>0).mean():.1%}, "
          f"per-trade Sharpe={overall_sh:.3f}")

    print(f"\nPearson correlations (pooled n={len(pooled)}):")
    for col in ["p_crash", "p_down", "p_flat", "p_up", "p_strong_up", "bull_score", "bear_score"]:
        r = pooled["profit_ratio"].corr(pooled[col])
        # Standard error of correlation: 1/sqrt(n-2)
        se = 1.0 / np.sqrt(len(pooled) - 2)
        # Significance threshold at 95%: |r| > 1.96 * se
        sig = "**" if abs(r) > 1.96 * se else ("*" if abs(r) > se else "")
        print(f"  return vs {col:18s}: {r:+.4f}  SE={se:.4f}  {sig}")

    # 1. P(crash) bucketing
    report_bucket("1. P(crash) bucketing", pooled, [0, 0.02, 0.05, 0.10, 1.0], "p_crash")

    # 3. bull_score
    report_bucket("3. bull_score = P(up)+P(strong_up)", pooled,
                  [0, 0.40, 0.50, 0.60, 0.70, 1.0], "bull_score")

    # 4. bear_score
    report_bucket("4. bear_score = P(down)+P(crash)", pooled,
                  [0, 0.10, 0.20, 0.30, 1.0], "bear_score")

    # 5. argmax class
    print(f"\n5. argmax-class bucketing:")
    print(f"  {'class':25s}{'n_trades':>10s}{'mean_ret':>11s}{'hit_rate':>11s}{'sample_sh':>11s}")
    print("  " + "-" * 70)
    for cls, name in enumerate(CLASS_NAMES):
        group = pooled[pooled["pred_class"] == cls]
        if len(group) == 0: continue
        ret = group["profit_ratio"]
        sh = (ret.mean() / ret.std()) if ret.std() > 0 else 0.0
        print(f"  {name:25s}{len(group):>10d}{ret.mean():>11.4%}"
              f"{(ret > 0).mean():>10.1%}{sh:>11.3f}")

    # Per-tf consistency check on bull_score buckets
    print(f"\nBull_score 0.5-0.6 bucket per timeframe (the 'sweet spot' from Slice 120):")
    print(f"  {'tf':>4s}{'n_trades':>10s}{'mean_ret':>11s}{'sample_sh':>11s}")
    sweet_mask = (pooled["bull_score"] > 0.5) & (pooled["bull_score"] <= 0.6)
    for tf in ["5m", "15m", "1h"]:
        g = pooled[(pooled["tf"] == tf) & sweet_mask]
        if len(g) == 0: continue
        ret = g["profit_ratio"]
        sh = (ret.mean() / ret.std()) if ret.std() > 0 else 0.0
        print(f"  {tf:>4s}{len(g):>10d}{ret.mean():>11.4%}{sh:>11.3f}")

    print(f"\nBull_score 0.6-0.7 bucket per timeframe (the 'death zone' from Slice 120):")
    death_mask = (pooled["bull_score"] > 0.6) & (pooled["bull_score"] <= 0.7)
    for tf in ["5m", "15m", "1h"]:
        g = pooled[(pooled["tf"] == tf) & death_mask]
        if len(g) == 0: continue
        ret = g["profit_ratio"]
        sh = (ret.mean() / ret.std()) if ret.std() > 0 else 0.0
        print(f"  {tf:>4s}{len(g):>10d}{ret.mean():>11.4%}{sh:>11.3f}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
