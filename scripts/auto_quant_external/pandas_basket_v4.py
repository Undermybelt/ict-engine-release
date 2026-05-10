"""
pandas_basket_v4.py — multi-candidate drought-fixed pandas basket with V3
regime conditioning per candidate.

Slice 104 ran a single candidate (TrendPullback-NoRSI) through the pandas
alt-backtester and applied V3 regime conditioning, achieving 8Y Sharpe 1.480.
This script extends to a 2-candidate basket by adding a SweepReclaim15mWide-
equivalent in pandas, then aggregates per-candidate train-derived V3-
conditional trades into equal-weight and inverse-vol baskets.

The SweepReclaim implementation uses in-asset trend gates (EMA200 > EMA600)
instead of the original 1h/4h informatives to dodge the freqtrade drought
issue.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table

NQ_15M_FEATHER = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
START = pd.Timestamp("2018-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
TRAIN_END = pd.Timestamp("2023-01-01", tz="UTC")
TRADING_DAYS = 252.0
MIN_TRADES_PER_CELL = 30
MIN_TRADES_FOR_INVERSE_VOL = 30


def load_term_structure() -> pd.Series:
    def load(p: Path) -> pd.Series:
        df = pd.read_csv(p)
        df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
        df = df.dropna(subset=["ts", "close"])
        df["date"] = df["ts"].dt.normalize()
        s = df.set_index("date")["close"].astype(float)
        return s[~s.index.duplicated(keep="last")].sort_index()
    vix9d = load(VIX9D_CSV)
    vix3m = load(VIX3M_CSV)
    common = vix9d.index.intersection(vix3m.index)
    return (vix9d.loc[common] / vix3m.loc[common].where(vix3m.loc[common] > 1e-9)).rename("term_ratio")


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


def load_base_indicators() -> pd.DataFrame:
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
    df["liquid_window_8_23"] = (df["hour_utc"] >= 8) & (df["hour_utc"] <= 23)
    df["liquid_window_8_22"] = (df["hour_utc"] >= 8) & (df["hour_utc"] <= 22)
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])
    df["pullback_zone"] = df["near_ema21"] <= 2.4
    df["reacceleration"] = df["body_green"] | (df["close"] > df["close"].shift(1))
    df["regime_break_ema200"] = df["close"] < df["ema200"]
    df["low_12bar"] = df["low"].rolling(12).min().shift(1)
    df["low_2bar"] = df["low"].rolling(2).min()
    df["sweep_below"] = df["low_2bar"] < df["low_12bar"]
    df["reclaim_close"] = df["close"] > df["low_12bar"]
    df["body_strength"] = (df["close"] - df["open"]) / df["atr"]
    return df.loc[START:END]


def simulate_strategy(
    df: pd.DataFrame,
    entry_signal: pd.Series,
    exit_signal: pd.Series,
    stoploss: float,
    trailing_offset: float,
    trailing_stop: float,
) -> pd.DataFrame:
    closes = df["close"].to_numpy()
    highs = df["high"].to_numpy()
    lows = df["low"].to_numpy()
    es = entry_signal.to_numpy()
    xs = exit_signal.to_numpy()
    timestamps = df.index.to_numpy()

    trades: list[dict] = []
    in_position = False
    entry_idx = -1
    entry_price = 0.0
    peak_price = 0.0
    trailing_active = False

    for i in range(len(df)):
        if not in_position:
            if es[i]:
                in_position = True
                entry_idx = i
                entry_price = closes[i]
                peak_price = closes[i]
                trailing_active = False
            continue

        cur_close = closes[i]
        peak_price = max(peak_price, highs[i])
        gain_from_entry = peak_price / entry_price - 1.0
        if not trailing_active and gain_from_entry >= trailing_offset:
            trailing_active = True

        sl_price = entry_price * (1.0 + stoploss)
        trail_price = peak_price * (1.0 - trailing_stop) if trailing_active else 0.0
        eff_stop = max(sl_price, trail_price)

        exit_reason = None
        exit_price = cur_close
        if lows[i] <= eff_stop:
            exit_reason = "trailing_or_stop"
            exit_price = eff_stop
        elif xs[i]:
            exit_reason = "exit_signal"
            exit_price = cur_close

        if exit_reason is not None:
            profit_ratio = exit_price / entry_price - 1.0
            trades.append({
                "open_date": pd.Timestamp(timestamps[entry_idx]),
                "close_date": pd.Timestamp(timestamps[i]),
                "profit_ratio": profit_ratio,
                "exit_reason": exit_reason,
                "bars_held": i - entry_idx,
            })
            in_position = False
            entry_idx = -1
            entry_price = 0.0
            peak_price = 0.0
            trailing_active = False
    return pd.DataFrame(trades)


def daily_pnl(trades: pd.DataFrame) -> pd.Series:
    if trades.empty:
        return pd.Series(dtype=float)
    s = trades.copy()
    s["date"] = s["close_date"].dt.normalize()
    return s.groupby("date")["profit_ratio"].sum().sort_index()


def annual_metrics(daily_returns: pd.Series) -> dict[str, float]:
    if daily_returns.empty or daily_returns.std() == 0:
        return {"sharpe": 0.0, "sortino": 0.0, "max_dd": 0.0,
                "total_return": 0.0}
    mean = daily_returns.mean()
    std = daily_returns.std()
    sharpe = (mean / std) * np.sqrt(TRADING_DAYS) if std > 0 else 0.0
    downside = daily_returns[daily_returns < 0]
    sortino = (mean / downside.std()) * np.sqrt(TRADING_DAYS) if (len(downside) > 1 and downside.std() > 0) else 0.0
    cum = (1.0 + daily_returns).cumprod()
    running_max = cum.cummax()
    dd = float((cum / running_max - 1.0).min())
    return {"sharpe": float(sharpe), "sortino": float(sortino),
            "max_dd": dd, "total_return": float(cum.iloc[-1] - 1.0)}


def derive_deny(train_trades: pd.DataFrame) -> set[tuple[str, str]]:
    deny: set[tuple[str, str]] = set()
    if train_trades.empty:
        return deny
    for (regime, term), group in train_trades.groupby(["regime", "term"]):
        n = len(group)
        if n < MIN_TRADES_PER_CELL:
            continue
        returns = group["profit_ratio"].astype(float)
        if returns.std() == 0:
            continue
        sharpe = returns.mean() / returns.std()
        if sharpe < 0 and regime != "unknown" and term != "unknown":
            deny.add((regime, term))
    return deny


def main() -> int:
    df = load_base_indicators()

    trend_entry = (
        df["liquid_window_8_23"]
        & (df["long_trend"] | df["local_trend"])
        & df["pullback_zone"]
        & df["reacceleration"]
    )
    sweep_entry = (
        df["liquid_window_8_22"]
        & (df["long_trend"] | df["local_trend"])
        & df["sweep_below"]
        & df["reclaim_close"]
        & (df["body_strength"] > 0.25)
    )
    trend_exit = df["regime_break_ema200"]
    sweep_exit = df["close"] < df["low_12bar"]

    print("Simulating TrendPullback-NoRSI...")
    trend_trades = simulate_strategy(df, trend_entry, trend_exit,
                                      stoploss=-0.022, trailing_offset=0.010, trailing_stop=0.004)
    print(f"  trades: {len(trend_trades)}")
    print("Simulating SweepReclaim-Wide...")
    sweep_trades = simulate_strategy(df, sweep_entry, sweep_exit,
                                      stoploss=-0.018, trailing_offset=0.010, trailing_stop=0.004)
    print(f"  trades: {len(sweep_trades)}")

    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term).rename("term")

    candidates = {
        "TrendPullback-NoRSI": trend_trades,
        "SweepReclaim-Wide": sweep_trades,
    }
    for name, t in candidates.items():
        if t.empty:
            continue
        t["entry_date"] = t["open_date"].dt.normalize()
        t["regime"] = t["entry_date"].map(regime_lookup).fillna("unknown")
        t["term"] = t["entry_date"].map(term_class).fillna("unknown")

    series_uncond: dict[str, pd.Series] = {}
    series_cond: dict[str, pd.Series] = {}
    derived_rules: dict[str, set[tuple[str, str]]] = {}

    print()
    print("=" * 100)
    print("Per-candidate train-derived deny rules and standalone metrics")
    print("=" * 100)
    for name, t in candidates.items():
        train = t[t["entry_date"] < TRAIN_END]
        deny = derive_deny(train)
        derived_rules[name] = deny
        cond = t[~t.apply(lambda r: (r["regime"], r["term"]) in deny, axis=1)]
        series_uncond[name] = daily_pnl(t)
        series_cond[name] = daily_pnl(cond)

        u = annual_metrics(series_uncond[name])
        c = annual_metrics(series_cond[name])
        print(f"\n{name}")
        print(f"  trades: {len(t)} -> cond {len(cond)}")
        print(f"  deny rules: {sorted(deny) if deny else 'none'}")
        print(f"  uncond: Sharpe={u['sharpe']:+.3f}  Sortino={u['sortino']:+.3f}  MaxDD={u['max_dd']:+.2%}  Total={u['total_return']:+.2%}")
        print(f"  cond:   Sharpe={c['sharpe']:+.3f}  Sortino={c['sortino']:+.3f}  MaxDD={c['max_dd']:+.2%}  Total={c['total_return']:+.2%}")

    def reindex_basket(series_map: dict[str, pd.Series]) -> pd.DataFrame:
        if not series_map:
            return pd.DataFrame()
        all_dates = sorted({d for s in series_map.values() for d in s.index})
        if not all_dates:
            return pd.DataFrame()
        idx = pd.date_range(min(all_dates), max(all_dates), freq="D", tz="UTC")
        return pd.DataFrame({k: s.reindex(idx).fillna(0.0) for k, s in series_map.items()})

    uncond_df = reindex_basket(series_uncond)
    cond_df = reindex_basket(series_cond)
    print()
    print("=" * 100)
    print("Pairwise daily-PnL correlation (8Y unconditional)")
    print("=" * 100)
    print(uncond_df.corr().round(3).to_string())
    print()
    print("Pairwise daily-PnL correlation (8Y V3 conditional)")
    print(cond_df.corr().round(3).to_string())
    print()

    cols = list(uncond_df.columns)
    eq_w = pd.Series(1.0 / len(cols), index=cols)
    counts_u = pd.Series({c: int((uncond_df[c] != 0).sum()) for c in cols})
    counts_c = pd.Series({c: int((cond_df[c] != 0).sum()) for c in cols})
    elig_u = counts_u >= MIN_TRADES_FOR_INVERSE_VOL
    elig_c = counts_c >= MIN_TRADES_FOR_INVERSE_VOL
    if elig_u.sum() < 2:
        elig_u = counts_u > 0
    if elig_c.sum() < 2:
        elig_c = counts_c > 0
    iv_u_w = pd.Series(0.0, index=cols)
    iv_c_w = pd.Series(0.0, index=cols)
    if elig_u.any():
        vol = uncond_df.loc[:, elig_u].std()
        raw = (1.0 / vol).where(vol > 0, 0.0)
        iv_u_w.loc[elig_u] = raw / raw.sum()
    if elig_c.any():
        vol = cond_df.loc[:, elig_c].std()
        raw = (1.0 / vol).where(vol > 0, 0.0)
        iv_c_w.loc[elig_c] = raw / raw.sum()

    eq_uncond = annual_metrics((uncond_df * eq_w).sum(axis=1))
    eq_cond = annual_metrics((cond_df * eq_w).sum(axis=1))
    iv_uncond = annual_metrics((uncond_df * iv_u_w).sum(axis=1))
    iv_cond = annual_metrics((cond_df * iv_c_w).sum(axis=1))

    print("=" * 100)
    print("Multi-candidate basket (8Y full period, drought-fixed)")
    print("=" * 100)
    print(f"{'mode':36s}{'sharpe':>8s}{'sortino':>9s}{'maxdd':>9s}{'totret':>10s}")
    for label, m in [
        ("equal-weight unconditional",  eq_uncond),
        ("equal-weight V3 conditional", eq_cond),
        ("inverse-vol unconditional",   iv_uncond),
        ("inverse-vol V3 conditional",  iv_cond),
    ]:
        print(f"{label:36s}{m['sharpe']:>8.3f}{m['sortino']:>9.3f}{m['max_dd']:>9.2%}{m['total_return']:>10.2%}")
    print()
    print(f"Inverse-vol weights (cond): {dict(iv_c_w.round(3))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
