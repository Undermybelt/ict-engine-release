"""
regime_attribution.py — split each candidate's trade history by entry-day regime
and report per-regime trade count, Sharpe, win rate, and total return.

The Slice 93 train/test split showed that the in-sample 2023-2025 basket Sharpe
of 2.78 was overfit to a regime-favorable window: 3 of 4 candidates collapsed
on the 2018-2022 train period. The natural follow-up is to characterize WHICH
regimes each candidate's edge concentrates in. If TrendPullbackDense15m is the
only regime-stable candidate, what regime feature is it stable on? If
SweepReclaim collapses on 2018-2022, what regime difference makes 2023-2025
favorable? The answers are the regime descriptors needed to make the pack
conditionally deployable.

Approach:
- load NQ daily candles from feather to define daily regime features
- load daily VIX close from /tmp/ict-engine-ibkr-probe/vix.1d.10y.csv
- compute regime features per day: NQ above 200d SMA, NQ-200d slope, VIX level,
  NQ drawdown from rolling 60d high
- classify each day into one of:
    TrendingCalm:    above 200d + slope rising + VIX low
    TrendingNervous: above 200d + VIX elevated
    ChopRange:       within 5% of 200d, low slope
    BearishStress:   below 200d + drawdown
- load each candidate's trade export from the latest backtest zip
- label each trade by its entry-day regime
- compute per-regime metrics: trade count, Sharpe, win rate, profit factor,
  total return

This is a regime characterizer, not a deployable classifier. The output is a
description of where each candidate's edge concentrates, useful for
designing the next round of candidates and as a precursor to a regime-
conditional allocator.
"""
from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

import numpy as np
import pandas as pd

BACKTEST_RESULTS = Path("user_data/backtest_results")
NQ_DAILY_FEATHER = Path("user_data/data/NQ_USD-1d.feather")
VIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vix.1d.10y.csv")

CANDIDATES: list[tuple[str, str]] = [
    ("TomacNQ_RegimeTrendPullbackDense15m", "trend continuation pullback"),
    ("TomacNQ_RegimePersistenceClusterDense15m", "trend continuation persistence"),
    ("TomacNQ_RegimeLiquiditySweepReclaim15mWide", "mean reversion / sweep"),
    ("TomacNQ_RegimeVRPCompression15m", "iv-hv compression regime"),
]


def load_daily_regime_table() -> pd.DataFrame:
    nq = pd.read_feather(NQ_DAILY_FEATHER)
    nq["date"] = pd.to_datetime(nq["date"], unit="ms", utc=True).dt.normalize()
    nq = nq.set_index("date").sort_index()

    vix_df = pd.read_csv(VIX_CSV)
    vix_df["ts"] = pd.to_datetime(vix_df["ts"], utc=True, errors="coerce")
    vix_df = vix_df.dropna(subset=["ts", "close"])
    vix_df["date"] = vix_df["ts"].dt.normalize()
    vix = vix_df.set_index("date")["close"].astype(float)
    vix = vix[~vix.index.duplicated(keep="last")].sort_index()

    df = pd.DataFrame(index=nq.index)
    df["nq_close"] = nq["close"]
    df["nq_sma200"] = nq["close"].rolling(200).mean()
    df["nq_above_sma200"] = (df["nq_close"] > df["nq_sma200"]).astype(int)
    df["nq_sma200_slope"] = df["nq_sma200"].pct_change(20)
    df["nq_60d_high"] = nq["close"].rolling(60).max()
    df["nq_drawdown_60d"] = df["nq_close"] / df["nq_60d_high"] - 1.0
    df["vix"] = vix.reindex(df.index).ffill()

    def classify(row: pd.Series) -> str:
        if not (np.isfinite(row["nq_above_sma200"]) and np.isfinite(row["vix"])):
            return "unknown"
        if row["nq_above_sma200"] == 1 and row["nq_sma200_slope"] > 0.005 and row["vix"] < 20:
            return "TrendingCalm"
        if row["nq_above_sma200"] == 1 and row["vix"] >= 20:
            return "TrendingNervous"
        if row["nq_drawdown_60d"] < -0.07 and row["vix"] >= 20:
            return "BearishStress"
        if abs(row["nq_close"] / row["nq_sma200"] - 1.0) < 0.05:
            return "ChopRange"
        if row["nq_above_sma200"] == 0 and row["nq_sma200_slope"] < 0:
            return "BearishStress"
        return "Other"

    df["regime"] = df.apply(classify, axis=1)
    return df


def find_latest_zip_for_strategy(strategy: str) -> Path | None:
    candidates: list[tuple[float, Path]] = []
    for meta_path in BACKTEST_RESULTS.glob("backtest-result-*.meta.json"):
        try:
            payload = json.loads(meta_path.read_text())
        except (OSError, json.JSONDecodeError):
            continue
        if strategy in payload:
            zip_path = meta_path.with_suffix("").with_suffix(".zip")
            if zip_path.exists():
                candidates.append((zip_path.stat().st_mtime, zip_path))
    if not candidates:
        return None
    candidates.sort(reverse=True)
    return candidates[0][1]


def load_trades_from_zip(zip_path: Path, strategy: str) -> pd.DataFrame:
    with zipfile.ZipFile(zip_path) as zf:
        result_name = next(
            name for name in zf.namelist()
            if name.endswith(".json") and "_config" not in name
        )
        with zf.open(result_name) as fh:
            payload = json.load(fh)
    strat = payload["strategy"][strategy]
    trades = strat.get("trades", [])
    if not trades:
        return pd.DataFrame(columns=["close_date", "open_date", "profit_ratio"])
    df = pd.DataFrame(trades)
    for col in ("open_date", "close_date"):
        df[col] = pd.to_datetime(df[col], utc=True, errors="coerce")
    df = df.dropna(subset=["open_date", "close_date"])
    return df[["open_date", "close_date", "profit_ratio"]]


def attribute_trades(trades: pd.DataFrame, regime_table: pd.DataFrame) -> pd.DataFrame:
    if trades.empty:
        return pd.DataFrame()
    trades = trades.copy()
    trades["entry_date"] = trades["open_date"].dt.normalize()
    regime_lookup = regime_table["regime"]
    trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
    return trades


def per_regime_metrics(trades: pd.DataFrame) -> pd.DataFrame:
    if trades.empty:
        return pd.DataFrame()
    rows: list[dict] = []
    for regime, group in trades.groupby("regime"):
        n = len(group)
        if n == 0:
            continue
        returns = group["profit_ratio"].astype(float)
        win = (returns > 0).sum()
        loss = (returns < 0).sum()
        total_return = float((1.0 + returns).prod() - 1.0)
        gross_win = float(returns[returns > 0].sum())
        gross_loss = float(-returns[returns < 0].sum())
        pf = gross_win / gross_loss if gross_loss > 1e-9 else float("inf") if gross_win > 0 else 0.0
        rows.append({
            "regime": regime,
            "trades": n,
            "win_rate": win / n if n > 0 else 0.0,
            "mean_return_per_trade": returns.mean(),
            "total_return": total_return,
            "profit_factor": pf,
            "sharpe_per_trade": returns.mean() / returns.std() if returns.std() > 0 else 0.0,
        })
    return pd.DataFrame(rows).sort_values("trades", ascending=False).reset_index(drop=True)


def main() -> int:
    regime_table = load_daily_regime_table()
    print("=" * 78)
    print("Per-candidate regime attribution (NQ/USD 15m, latest backtest export)")
    print("=" * 78)
    print()
    print("Daily regime distribution over 2018-2025:")
    print(regime_table.loc["2018":"2025", "regime"].value_counts().to_string())
    print()

    for strategy, family in CANDIDATES:
        zip_path = find_latest_zip_for_strategy(strategy)
        if zip_path is None:
            print(f"WARN: no zip found for {strategy}", file=sys.stderr)
            continue
        trades = load_trades_from_zip(zip_path, strategy)
        if trades.empty:
            print(f"  {strategy}: no trades")
            continue
        labeled = attribute_trades(trades, regime_table)
        per_regime = per_regime_metrics(labeled)
        print("-" * 78)
        print(f"{strategy} ({family})")
        print(f"trades total: {len(labeled)}, span "
              f"{trades['open_date'].min().date()} -> {trades['open_date'].max().date()}")
        print(per_regime.to_string(index=False))
        print()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
