"""
pandas_bbn_vrp_integration.py — Slice 119. Test the user's closed-loop
hypothesis: BBN posterior P(regime | evidence) used as soft conditioning
on VRP V2 execution.

Closed loop:
  evidence -> BBN posterior P(regime|evidence) -> execution-tree node
  -> trade allowed/denied/sized

Slice 118 saved 2023-2025 test posteriors to slice_118_bbn_predictions.csv.
This slice applies them as VRP V2 entry filters and measures Sharpe/DD
lift over the same test period.

Variants:
- baseline: V2 unrestricted (test period only, for fair comparison)
- v2_deny_crash_010:    deny entry if P(crash) > 0.10
- v2_deny_crash_020:    deny entry if P(crash) > 0.20
- v2_only_strong_up:    only enter if P(strong_up) > 0.10
- v2_deny_downside_040: deny if P(down) + P(crash) > 0.40
- v2_high_conv_up:      only enter if P(up) + P(strong_up) > 0.70

Metrics: aggregate Sharpe, max DD, total return, trade count, walk-forward
median 6M Sharpe (2023H1, 2023H2, 2024H1, 2024H2, 2025H1, 2025H2).
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))

NQ_15M = Path("user_data/data/NQ_USD-15m.feather")
PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")
QQQ_IV_CSV = PROBE_DIR / "qqq.iv.1d.10y.csv"
QQQ_HV_CSV = PROBE_DIR / "qqq.hv.1d.10y.csv"
VVIX_CSV = PROBE_DIR / "vvix.1d.10y.csv"
BBN_CSV = PROBE_DIR / "slice_118_bbn_predictions.csv"

START = pd.Timestamp("2023-01-01", tz="UTC")  # test period only
END = pd.Timestamp("2025-12-31", tz="UTC")
TRADING_DAYS = 252.0

STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.005


def load_close(csv_path):
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def build_indicators_with_bbn():
    df = pd.read_feather(NQ_15M)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index().loc[START:END]
    df["ema21"] = df["close"].ewm(span=21, adjust=False).mean()
    df["ema89"] = df["close"].ewm(span=89, adjust=False).mean()
    df["ema200"] = df["close"].ewm(span=200, adjust=False).mean()
    df["ema600"] = df["close"].ewm(span=600, adjust=False).mean()
    df["body_green"] = df["close"] > df["open"]
    df["hour_utc"] = df.index.hour

    iv = load_close(QQQ_IV_CSV)
    hv = load_close(QQQ_HV_CSV)
    vvix = load_close(VVIX_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    vvix_pr = vvix.rolling(252, min_periods=128).rank(pct=True)
    candle_dates = df.index.normalize()
    df["iv_pct_rank_252"] = pd.Series(candle_dates.map(iv_pr), index=df.index).ffill()
    df["hv_pct_rank_252"] = pd.Series(candle_dates.map(hv_pr), index=df.index).ffill()
    df["vvix_pct_rank_252"] = pd.Series(candle_dates.map(vvix_pr), index=df.index).ffill()

    df["liquid_window"] = (df["hour_utc"] >= 13) & (df["hour_utc"] <= 21)
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])

    # V2 base entry
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

    # Load BBN posteriors and align by date
    bbn = pd.read_csv(BBN_CSV, index_col=0, parse_dates=True)
    bbn.index = pd.to_datetime(bbn.index, utc=True).normalize()
    bbn = bbn[~bbn.index.duplicated(keep="last")]
    for col in ["p_crash", "p_down", "p_flat", "p_up", "p_strong_up"]:
        df[col] = pd.Series(candle_dates.map(bbn[col]), index=df.index).ffill()

    return df


def simulate_with_filter(df, entry_filter):
    closes = df["close"].to_numpy(); highs = df["high"].to_numpy(); lows = df["low"].to_numpy()
    es = entry_filter.to_numpy()
    xs = df["exit_signal"].to_numpy()
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
    if d.empty or d.std() == 0:
        return {"sharpe": 0.0, "max_dd": 0.0, "total": 0.0}
    sharpe = (d.mean() / d.std()) * np.sqrt(TRADING_DAYS) if d.std() > 0 else 0.0
    cum = (1.0 + d).cumprod(); dd = float((cum / cum.cummax() - 1.0).min())
    return {"sharpe": float(sharpe), "max_dd": dd, "total": float(cum.iloc[-1] - 1.0)}


def walk_forward(trades, start_year=2023, end_year=2026):
    if trades.empty: return None
    cur = pd.Timestamp(f"{start_year}-01-01", tz="UTC")
    end = pd.Timestamp(f"{end_year}-01-01", tz="UTC")
    sharpes = []
    while cur < end:
        nxt = cur + pd.DateOffset(months=6)
        wt = trades[(trades["open_date"] >= cur) & (trades["open_date"] < nxt)]
        if len(wt) >= 5:
            m = annual_metrics(daily_pnl(wt))
            sharpes.append(m["sharpe"])
        cur = nxt
    if not sharpes: return None
    return {
        "wf_median": float(np.median(sharpes)),
        "wf_pos_pct": float(sum(1 for s in sharpes if s > 0) / len(sharpes)),
        "wf_n": len(sharpes),
    }


def main() -> int:
    df = build_indicators_with_bbn()
    print(f"loaded {len(df)} 15m bars, BBN posterior coverage: "
          f"{df['p_crash'].notna().sum()} / {len(df)}")
    print()

    bbn_available = df["p_crash"].notna()

    variants = {
        "baseline V2 (test period)":      df["v2_entry"],
        "V2 + deny P(crash)>0.10":        df["v2_entry"] & bbn_available & (df["p_crash"] <= 0.10),
        "V2 + deny P(crash)>0.20":        df["v2_entry"] & bbn_available & (df["p_crash"] <= 0.20),
        "V2 + only P(strong_up)>0.10":    df["v2_entry"] & bbn_available & (df["p_strong_up"] > 0.10),
        "V2 + deny P(down|crash)>0.40":   df["v2_entry"] & bbn_available & ((df["p_down"] + df["p_crash"]) <= 0.40),
        "V2 + only P(up|strong_up)>0.70": df["v2_entry"] & bbn_available & ((df["p_up"] + df["p_strong_up"]) > 0.70),
    }

    rows = []
    for label, mask in variants.items():
        trades = simulate_with_filter(df, mask)
        if trades.empty:
            rows.append({"variant": label, "trades": 0, "sharpe": 0.0,
                         "max_dd": 0.0, "total": 0.0,
                         "wf_med": 0.0, "wf_pos%": 0.0, "wf_n": 0})
            continue
        m = annual_metrics(daily_pnl(trades))
        wf = walk_forward(trades)
        rows.append({
            "variant": label, "trades": int(len(trades)),
            "sharpe": m["sharpe"], "max_dd": m["max_dd"], "total": m["total"],
            "wf_med": wf["wf_median"] if wf else 0.0,
            "wf_pos%": (wf["wf_pos_pct"] * 100) if wf else 0.0,
            "wf_n": wf["wf_n"] if wf else 0,
        })

    print("=" * 105)
    print("BBN posterior → VRP V2 execution gating (test period 2023-2025)")
    print("=" * 105)
    print(f"{'variant':38s}{'trades':>8s}{'sharpe':>8s}{'maxdd':>9s}{'total':>9s}"
          f"{'wf_med':>9s}{'wf_pos':>9s}{'wf_n':>6s}")
    print("-" * 105)
    for r in rows:
        print(f"{r['variant']:38s}{r['trades']:>8d}{r['sharpe']:>8.3f}"
              f"{r['max_dd']:>9.2%}{r['total']:>9.2%}"
              f"{r['wf_med']:>9.3f}{r['wf_pos%']:>8.1f}%{r['wf_n']:>6d}")
    print()

    print("Interpretation:")
    base = rows[0]
    for r in rows[1:]:
        delta_sharpe = r["sharpe"] - base["sharpe"]
        delta_dd = r["max_dd"] - base["max_dd"]
        delta_trades = r["trades"] - base["trades"]
        verdict = ("LIFT" if delta_sharpe > 0.2 and r["trades"] >= 50
                   else "neutral" if abs(delta_sharpe) <= 0.2
                   else "drop")
        print(f"  {r['variant']:38s} ΔSharpe={delta_sharpe:+.3f}, "
              f"ΔDD={delta_dd*100:+.2f}%, Δtrades={delta_trades:+d}, {verdict}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
