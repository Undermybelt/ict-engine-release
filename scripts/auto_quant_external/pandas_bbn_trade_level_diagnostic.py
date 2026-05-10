"""
pandas_bbn_trade_level_diagnostic.py — Slice 120. Test whether BBN posteriors
carry ANY residual trade-level information about VRP V2 outcomes beyond
what V2's hand-crafted gates already capture.

Setup:
- Run VRP V2 on test period 2023-2025 → 287 trades
- For each trade, look up entry-day BBN posterior (P(crash), P(down),
  P(flat), P(up), P(strong_up))
- Bucket trades by various posterior measures
- Compute per-bucket mean return, hit rate, sample Sharpe

Test buckets:
1. P(crash) bins: [0,0.05,0.10,0.15,1.0]
2. P(strong_up) bins: [0,0.02,0.05,0.10,1.0]
3. P(up)+P(strong_up) (bull score): [0,0.4,0.5,0.6,0.7,1.0]
4. P(down)+P(crash) (bear score): [0,0.15,0.30,0.50,1.0]
5. argmax class (categorical)

If any bucketing shows a monotonic relationship with trade outcome, BBN
has signal beyond V2 gates. If all are flat, BBN is fully redundant.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

NQ_15M = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather")
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


def load_close(csv_path):
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
    df["ema600"] = df["close"].ewm(span=600, adjust=False).mean()
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour

    iv = load_close(QQQ_IV_CSV); hv = load_close(QQQ_HV_CSV); vvix = load_close(VVIX_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)
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
    print(f"\n{label} bucketing:")
    print(f"  {'bucket':25s}{'n_trades':>10s}{'mean_ret':>11s}{'hit_rate':>11s}{'sample_sh':>11s}")
    print("  " + "-" * 70)
    bins = pd.cut(trades[key], bins=edges, include_lowest=True)
    for interval, group in trades.groupby(bins, observed=True):
        if len(group) == 0: continue
        ret = group["profit_ratio"]
        mean_r = ret.mean()
        hit = (ret > 0).mean()
        sample_sh = (mean_r / ret.std()) if ret.std() > 0 else 0.0
        print(f"  {str(interval):25s}{len(group):>10d}{mean_r:>11.4%}{hit:>10.1%}{sample_sh:>11.3f}")


def main() -> int:
    df = build_indicators()
    print(f"loaded {len(df)} 15m bars (test period 2023-2025)")
    trades = simulate(df)
    print(f"VRP V2 test trades: {len(trades)}")
    if trades.empty:
        return 1
    trades["entry_date"] = trades["open_date"].dt.normalize()

    # Attach BBN posteriors
    bbn = pd.read_csv(BBN_CSV, index_col=0, parse_dates=True)
    bbn.index = pd.to_datetime(bbn.index, utc=True).normalize()
    bbn = bbn[~bbn.index.duplicated(keep="last")]
    for col in ["p_crash", "p_down", "p_flat", "p_up", "p_strong_up", "pred_class"]:
        trades[col] = trades["entry_date"].map(bbn[col])
    trades["bull_score"] = trades["p_up"] + trades["p_strong_up"]
    trades["bear_score"] = trades["p_down"] + trades["p_crash"]

    # Overall stats
    overall_ret = trades["profit_ratio"].mean()
    overall_hit = (trades["profit_ratio"] > 0).mean()
    print(f"\nOverall: n={len(trades)}, mean_ret={overall_ret:.4%}, hit={overall_hit:.1%}")
    print(f"Posterior coverage: {trades['p_up'].notna().sum()} / {len(trades)} trades")

    # 1. P(crash) bucketing
    report_bucket("1. P(crash)", trades, [0, 0.02, 0.05, 0.10, 1.0], "p_crash")

    # 2. P(strong_up)
    report_bucket("2. P(strong_up)", trades, [0, 0.02, 0.05, 0.10, 1.0], "p_strong_up")

    # 3. bull_score
    report_bucket("3. P(up)+P(strong_up) bull_score", trades,
                  [0, 0.40, 0.50, 0.60, 0.70, 1.0], "bull_score")

    # 4. bear_score
    report_bucket("4. P(down)+P(crash) bear_score", trades,
                  [0, 0.10, 0.20, 0.30, 1.0], "bear_score")

    # 5. argmax class
    print(f"\n5. argmax-class bucketing:")
    print(f"  {'class':25s}{'n_trades':>10s}{'mean_ret':>11s}{'hit_rate':>11s}{'sample_sh':>11s}")
    print("  " + "-" * 70)
    for cls, name in enumerate(CLASS_NAMES):
        group = trades[trades["pred_class"] == cls]
        if len(group) == 0:
            continue
        ret = group["profit_ratio"]
        mean_r = ret.mean()
        hit = (ret > 0).mean()
        sample_sh = (mean_r / ret.std()) if ret.std() > 0 else 0.0
        print(f"  {name:25s}{len(group):>10d}{mean_r:>11.4%}{hit:>10.1%}{sample_sh:>11.3f}")

    # Pearson correlations
    print(f"\nPearson correlations (per-trade return vs BBN posterior):")
    for col in ["p_crash", "p_down", "p_flat", "p_up", "p_strong_up", "bull_score", "bear_score"]:
        corr = trades["profit_ratio"].corr(trades[col])
        print(f"  return vs {col:18s}: {corr:+.4f}")

    print(f"\n=== INTERPRETATION ===")
    print(f"If any bucket shows MONOTONIC relationship between BBN posterior and mean return:")
    print(f"  → BBN has residual signal → can drive position sizing")
    print(f"If all bucketings are FLAT (no monotonicity):")
    print(f"  → BBN is fully redundant with V2 gates → no further integration value")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
