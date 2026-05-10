"""
pandas_vix_shock_basket.py — VIX-shock reversal candidate in pandas, plus
3-candidate basket combining TrendPullback-NoRSI + VRPCompression + VIXShock.

Slice 109 found that adding TrendPullback to VRPCompression's basket failed
to exceed VRPCompression standalone (correlation 0.28 too high, Sharpe gap
too wide). Need a candidate with low correlation to VRPCompression to make
the basket lift work.

VIX-shock reversal candidate (different vol-regime axis than VRPCompression):
- Entry: vix_z20 > 1.2 (VIX spikes 1.2 std-dev above 20-day mean) AND
  NQ pulled back >0.5% from rolling 5-day high AND first bullish 15m bar
  AND NQ not collapsing (close > ema89 * 0.97)
- Exit: vix_z20 normalizes (<0.3) OR regime break OR upper target

VIX-shock entries fire after volatility events; VRPCompression entries fire
during compressed vol. The two should have very low correlation by design.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table

NQ_15M_FEATHER = Path("/Users/thrill3r/Auto-Quant/user_data/data/NQ_USD-15m.feather")
VIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vix.1d.10y.csv")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
QQQ_IV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.iv.1d.10y.csv")
QQQ_HV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.hv.1d.10y.csv")
START = pd.Timestamp("2018-01-01", tz="UTC")
END = pd.Timestamp("2025-12-31", tz="UTC")
TRAIN_END = pd.Timestamp("2023-01-01", tz="UTC")
TRADING_DAYS = 252.0
MIN_TRADES_PER_CELL = 30


def load_close_series(csv_path: Path) -> pd.Series:
    df = pd.read_csv(csv_path)
    df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
    df = df.dropna(subset=["ts", "close"])
    df["date"] = df["ts"].dt.normalize()
    s = df.set_index("date")["close"].astype(float)
    return s[~s.index.duplicated(keep="last")].sort_index()


def load_term_structure() -> pd.Series:
    vix9d = load_close_series(VIX9D_CSV)
    vix3m = load_close_series(VIX3M_CSV)
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


def load_base() -> pd.DataFrame:
    df = pd.read_feather(NQ_15M_FEATHER)
    df["date"] = pd.to_datetime(df["date"], unit="ms", utc=True)
    df = df.set_index("date").sort_index().loc[START:END]
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
    df["liquid_window_13_21"] = (df["hour_utc"] >= 13) & (df["hour_utc"] <= 21)
    df["long_trend"] = df["ema200"] > df["ema600"]
    df["local_trend"] = (df["ema21"] > df["ema89"]) & (df["close"] > df["ema89"])
    df["pullback_zone"] = df["near_ema21"] <= 2.4
    df["reacceleration"] = df["body_green"] | (df["close"] > df["close"].shift(1))
    df["high_5d_15m"] = df["high"].rolling(96 * 5).max().shift(1)
    df["pullback_pct"] = df["close"] / df["high_5d_15m"] - 1.0
    df["bullish_body"] = df["close"] > df["open"]

    iv = load_close_series(QQQ_IV_CSV)
    hv = load_close_series(QQQ_HV_CSV)
    iv_pr = iv.rolling(252, min_periods=128).rank(pct=True)
    hv_pr = hv.rolling(252, min_periods=128).rank(pct=True)
    candle_dates = df.index.normalize()
    df["iv_pct_rank_252"] = pd.Series(candle_dates.map(iv_pr), index=df.index).ffill()
    df["hv_pct_rank_252"] = pd.Series(candle_dates.map(hv_pr), index=df.index).ffill()

    vix = load_close_series(VIX_CSV)
    vix_mean = vix.rolling(20, min_periods=10).mean()
    vix_std = vix.rolling(20, min_periods=10).std()
    vix_z20 = (vix - vix_mean) / vix_std.where(vix_std > 1e-9)
    df["vix_z20"] = pd.Series(candle_dates.map(vix_z20), index=df.index).ffill()
    return df


def simulate(df: pd.DataFrame, entry_signal: pd.Series, exit_signal: pd.Series,
             stoploss: float, trailing_offset: float, trailing_stop: float) -> pd.DataFrame:
    closes = df["close"].to_numpy()
    highs = df["high"].to_numpy()
    lows = df["low"].to_numpy()
    es = entry_signal.to_numpy()
    xs = exit_signal.to_numpy()
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
        if not trail and gain >= trailing_offset:
            trail = True
        sl = entry_price * (1.0 + stoploss)
        tp = peak * (1.0 - trailing_stop) if trail else 0.0
        eff = max(sl, tp)
        reason = None
        exit_price = closes[i]
        if lows[i] <= eff:
            reason = "stop"
            exit_price = eff
        elif xs[i]:
            reason = "exit"
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
    df = load_base()
    print(f"loaded {len(df)} 15m bars")

    trend_entry = (
        df["liquid_window_8_23"]
        & (df["long_trend"] | df["local_trend"])
        & df["pullback_zone"]
        & df["reacceleration"]
    )
    trend_exit = df["close"] < df["ema200"]

    vrp_entry = (
        df["liquid_window_13_21"]
        & (df["long_trend"] | df["local_trend"])
        & (df["iv_pct_rank_252"] < 0.30)
        & (df["hv_pct_rank_252"] < 0.30)
        & df["body_green"]
        & (df["close"] > df["ema89"])
    )
    vrp_exit = (df["iv_pct_rank_252"] > 0.55) | (df["close"] < df["ema89"])

    shock_entry = (
        df["liquid_window_13_21"]
        & (df["vix_z20"] > 1.2)
        & (df["pullback_pct"] < -0.005)
        & (df["close"] > df["ema89"] * 0.97)
        & df["bullish_body"]
        & (df["close"] > df["close"].shift(1))
    )
    shock_exit = (df["vix_z20"] < 0.3) | (df["close"] < df["ema89"] * 0.97)

    print("Simulating TrendPullback-NoRSI..."); trend_trades = simulate(df, trend_entry, trend_exit, -0.022, 0.010, 0.004)
    print(f"  trades: {len(trend_trades)}")
    print("Simulating VRPCompression..."); vrp_trades = simulate(df, vrp_entry, vrp_exit, -0.022, 0.010, 0.005)
    print(f"  trades: {len(vrp_trades)}")
    print("Simulating VIXShockReversal..."); shock_trades = simulate(df, shock_entry, shock_exit, -0.025, 0.012, 0.005)
    print(f"  trades: {len(shock_trades)}")
    print()

    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term)

    candidates = {
        "TrendPullback-NoRSI": trend_trades,
        "VRPCompression": vrp_trades,
        "VIXShockReversal": shock_trades,
    }
    series_uncond: dict[str, pd.Series] = {}
    series_cond: dict[str, pd.Series] = {}
    for name, t in candidates.items():
        if t.empty:
            print(f"WARN: {name} no trades")
            continue
        t["entry_date"] = t["open_date"].dt.normalize()
        t["regime"] = t["entry_date"].map(regime_lookup).fillna("unknown")
        t["term"] = t["entry_date"].map(term_class).fillna("unknown")
        train = t[t["entry_date"] < TRAIN_END]
        deny = derive_deny(train)
        cond = t[~t.apply(lambda r: (r["regime"], r["term"]) in deny, axis=1)]
        series_uncond[name] = daily_pnl(t)
        series_cond[name] = daily_pnl(cond)
        u = annual_metrics(series_uncond[name])
        c = annual_metrics(series_cond[name])
        print(f"{name}")
        print(f"  trades: {len(t)} -> cond {len(cond)}")
        print(f"  deny rules: {sorted(deny) if deny else 'none'}")
        print(f"  uncond: Sharpe={u['sharpe']:+.3f}  MaxDD={u['max_dd']:+.2%}  Total={u['total_return']:+.2%}")
        print(f"  cond:   Sharpe={c['sharpe']:+.3f}  MaxDD={c['max_dd']:+.2%}  Total={c['total_return']:+.2%}")
        print()

    def reindex(series_map: dict[str, pd.Series]) -> pd.DataFrame:
        all_dates = sorted({d for s in series_map.values() for d in s.index})
        idx = pd.date_range(min(all_dates), max(all_dates), freq="D", tz="UTC")
        return pd.DataFrame({k: s.reindex(idx).fillna(0.0) for k, s in series_map.items()})

    cond_df = reindex(series_cond)
    print("Pairwise daily-PnL correlation (V3 conditional):")
    print(cond_df.corr().round(3).to_string())
    print()

    cols = list(cond_df.columns)
    eq_w = pd.Series(1.0 / len(cols), index=cols)
    eq_cond = annual_metrics((cond_df * eq_w).sum(axis=1))
    vol = cond_df.std()
    raw = (1.0 / vol).where(vol > 0, 0.0)
    iv_w = raw / raw.sum()
    iv_cond = annual_metrics((cond_df * iv_w).sum(axis=1))

    print("3-candidate basket (8Y full period, V3 conditional):")
    print(f"{'mode':32s}{'sharpe':>8s}{'sortino':>9s}{'maxdd':>9s}{'totret':>9s}")
    print(f"{'eq-weight':32s}{eq_cond['sharpe']:>8.3f}{eq_cond['sortino']:>9.3f}{eq_cond['max_dd']:>9.2%}{eq_cond['total_return']:>9.2%}")
    print(f"{'inverse-vol':32s}{iv_cond['sharpe']:>8.3f}{iv_cond['sortino']:>9.3f}{iv_cond['max_dd']:>9.2%}{iv_cond['total_return']:>9.2%}")
    print()
    print(f"Inverse-vol weights: {dict(iv_w.round(3))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
