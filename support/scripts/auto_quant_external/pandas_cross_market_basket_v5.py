"""
pandas_cross_market_basket_v5.py — equal-weight basket of NQ + SPY + GLD with
V3 regime conditioning, using a common 1Y window where all three markets
have 15m data.

Slice 106 found the TrendPullback-NoRSI strategy ports cleanly to NQ / SPY /
GLD with V3 conditional Sharpes 1.50 / 2.26 / 2.23 respectively. This script
builds the 3-asset cross-market basket on the common 2025-05-07 → 2026-05-06
window and computes basket metrics. If basket Sharpe meaningfully exceeds
the best single-market Sharpe (2.26 SPY), the diversification is validated.

The 1Y common window biases all numbers high (favorable regime mix), but
the relative comparison of single vs basket is meaningful regardless.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table

DATA_DIR = Path("user_data/data")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
TRADING_DAYS = 252.0
COMMON_START = pd.Timestamp("2025-05-07", tz="UTC")
COMMON_END = pd.Timestamp("2026-05-06", tz="UTC")

NQ_DERIVED_DENY: set[tuple[str, str]] = {
    ("BearishStress", "Backwardation"),
    ("BearishStress", "FlatToBackward"),
    ("ChopRange", "Contango"),
    ("ChopRange", "DeepContango"),
    ("TrendingCalm", "Contango"),
    ("TrendingNervous", "Backwardation"),
    ("TrendingNervous", "Contango"),
}

MARKETS = [
    ("NQ/USD", DATA_DIR / "NQ_USD-15m.feather"),
    ("SPY/USD", DATA_DIR / "SPY_USD-15m.feather"),
    ("GLD/USD", DATA_DIR / "GLD_USD-15m.feather"),
]

STOPLOSS = -0.022
TRAILING_OFFSET = 0.010
TRAILING_STOP = 0.004


def load_term_structure() -> pd.Series:
    def load(p: Path) -> pd.Series:
        df = pd.read_csv(p)
        df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
        df = df.dropna(subset=["ts", "close"])
        df["date"] = df["ts"].dt.normalize()
        return df.set_index("date")["close"].astype(float)
    vix9d = load(VIX9D_CSV)
    vix3m = load(VIX3M_CSV)
    common = vix9d.index.intersection(vix3m.index)
    return (vix9d.loc[common] / vix3m.loc[common].where(vix3m.loc[common] > 1e-9))


def classify_term(value: float) -> str:
    if not (value == value):
        return "unknown"
    if value < 0.92:
        return "DeepContango"
    if value <= 1.00:
        return "Contango"
    if value <= 1.05:
        return "FlatToBackward"
    return "Backwardation"


def load_indicators(feather_path: Path) -> pd.DataFrame:
    df = pd.read_feather(feather_path)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index()
    df = df.loc[COMMON_START:COMMON_END]
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
    return df


def simulate(df: pd.DataFrame) -> pd.DataFrame:
    closes = df["close"].to_numpy()
    highs = df["high"].to_numpy()
    lows = df["low"].to_numpy()
    es = df["entry_signal"].to_numpy()
    rb = df["regime_break"].to_numpy()
    ts = df.index.to_numpy()
    trades: list[dict] = []
    in_pos = False
    entry_idx = -1
    entry_price = 0.0
    peak = 0.0
    trail = False
    for i in range(len(df)):
        if not in_pos:
            if es[i]:
                in_pos = True
                entry_idx = i
                entry_price = closes[i]
                peak = closes[i]
                trail = False
            continue
        peak = max(peak, highs[i])
        gain = peak / entry_price - 1.0
        if not trail and gain >= TRAILING_OFFSET:
            trail = True
        sl = entry_price * (1.0 + STOPLOSS)
        tp = peak * (1.0 - TRAILING_STOP) if trail else 0.0
        eff = max(sl, tp)
        reason = None
        exit_price = closes[i]
        if lows[i] <= eff:
            reason = "stop"
            exit_price = eff
        elif rb[i]:
            reason = "regime"
            exit_price = closes[i]
        if reason is not None:
            trades.append({
                "open_date": pd.Timestamp(ts[entry_idx]),
                "close_date": pd.Timestamp(ts[i]),
                "profit_ratio": exit_price / entry_price - 1.0,
            })
            in_pos = False
            entry_idx = -1
            entry_price = 0.0
            peak = 0.0
            trail = False
    return pd.DataFrame(trades)


def daily_pnl(trades: pd.DataFrame) -> pd.Series:
    if trades.empty:
        return pd.Series(dtype=float)
    s = trades.copy()
    s["date"] = s["close_date"].dt.normalize()
    return s.groupby("date")["profit_ratio"].sum().sort_index()


def annual_metrics(daily_returns: pd.Series) -> dict[str, float]:
    if daily_returns.empty or daily_returns.std() == 0:
        return {"sharpe": 0.0, "sortino": 0.0, "max_dd": 0.0, "total_return": 0.0}
    mean = daily_returns.mean()
    std = daily_returns.std()
    sharpe = (mean / std) * np.sqrt(TRADING_DAYS) if std > 0 else 0.0
    downside = daily_returns[daily_returns < 0]
    sortino = (mean / downside.std()) * np.sqrt(TRADING_DAYS) if (len(downside) > 1 and downside.std() > 0) else 0.0
    cum = (1.0 + daily_returns).cumprod()
    dd = float((cum / cum.cummax() - 1.0).min())
    return {"sharpe": float(sharpe), "sortino": float(sortino),
            "max_dd": dd, "total_return": float(cum.iloc[-1] - 1.0)}


def main() -> int:
    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term)

    series_uncond: dict[str, pd.Series] = {}
    series_cond: dict[str, pd.Series] = {}
    standalone_rows: list[dict] = []

    for name, feather in MARKETS:
        if not feather.exists():
            print(f"WARN: {name} missing", file=sys.stderr)
            continue
        df = load_indicators(feather)
        if len(df) < 800:
            print(f"WARN: {name} too short ({len(df)} bars)", file=sys.stderr)
            continue
        trades = simulate(df)
        if trades.empty:
            continue
        trades["entry_date"] = trades["open_date"].dt.normalize()
        trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
        trades["term"] = trades["entry_date"].map(term_class).fillna("unknown")
        deny_mask = trades.apply(lambda r: (r["regime"], r["term"]) in NQ_DERIVED_DENY, axis=1)
        cond = trades[~deny_mask]

        u = annual_metrics(daily_pnl(trades))
        c = annual_metrics(daily_pnl(cond))
        series_uncond[name] = daily_pnl(trades)
        series_cond[name] = daily_pnl(cond)
        standalone_rows.append({
            "market": name,
            "trades_uncond": len(trades),
            "trades_cond": len(cond),
            "sharpe_uncond": u["sharpe"],
            "sharpe_cond": c["sharpe"],
            "max_dd_cond": c["max_dd"],
            "total_cond": c["total_return"],
        })

    print("=" * 100)
    print(f"3-asset cross-market basket — common window {COMMON_START.date()} -> {COMMON_END.date()}")
    print("=" * 100)
    print()
    print("Per-market standalone:")
    df_rows = pd.DataFrame(standalone_rows)
    print(df_rows.to_string(index=False, float_format=lambda x: f"{x:.4f}"))
    print()

    def reindex_basket(series_map: dict[str, pd.Series]) -> pd.DataFrame:
        if not series_map:
            return pd.DataFrame()
        all_dates = sorted({d for s in series_map.values() for d in s.index})
        idx = pd.date_range(min(all_dates), max(all_dates), freq="D", tz="UTC")
        return pd.DataFrame({k: s.reindex(idx).fillna(0.0) for k, s in series_map.items()})

    uncond_df = reindex_basket(series_uncond)
    cond_df = reindex_basket(series_cond)

    print("Pairwise daily-PnL correlation (V3 conditional):")
    print(cond_df.corr().round(3).to_string())
    print()

    cols = list(cond_df.columns)
    eq_w = pd.Series(1.0 / len(cols), index=cols)
    eq_uncond = annual_metrics((uncond_df * eq_w).sum(axis=1))
    eq_cond = annual_metrics((cond_df * eq_w).sum(axis=1))
    vol = cond_df.std()
    raw = (1.0 / vol).where(vol > 0, 0.0)
    iv_w = raw / raw.sum()
    iv_cond = annual_metrics((cond_df * iv_w).sum(axis=1))

    print("Basket comparison:")
    print(f"{'mode':36s}{'sharpe':>8s}{'sortino':>9s}{'maxdd':>9s}{'totret':>9s}")
    print(f"{'equal-weight unconditional':36s}{eq_uncond['sharpe']:>8.3f}{eq_uncond['sortino']:>9.3f}{eq_uncond['max_dd']:>9.2%}{eq_uncond['total_return']:>9.2%}")
    print(f"{'equal-weight V3 conditional':36s}{eq_cond['sharpe']:>8.3f}{eq_cond['sortino']:>9.3f}{eq_cond['max_dd']:>9.2%}{eq_cond['total_return']:>9.2%}")
    print(f"{'inverse-vol V3 conditional':36s}{iv_cond['sharpe']:>8.3f}{iv_cond['sortino']:>9.3f}{iv_cond['max_dd']:>9.2%}{iv_cond['total_return']:>9.2%}")
    print()
    print("Inverse-vol weights:")
    for c in cols:
        print(f"  {c:12s}{iv_w[c]:>7.3f}")
    best_single = max(r["sharpe_cond"] for r in standalone_rows)
    print()
    print(f"best single-market V3 cond Sharpe:  {best_single:.3f}")
    print(f"basket equal-weight V3 cond Sharpe: {eq_cond['sharpe']:.3f}")
    print(f"basket inverse-vol V3 cond Sharpe:  {iv_cond['sharpe']:.3f}")
    if eq_cond['sharpe'] > best_single or iv_cond['sharpe'] > best_single:
        print("PASS: basket exceeds best single-market — cross-market diversification works")
    else:
        print("PARTIAL: basket below best single (best may have been favorable-regime tail)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
