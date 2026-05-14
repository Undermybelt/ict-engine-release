"""
portfolio_diversity_scorecard.py — compute pairwise correlation and combined portfolio metrics
for a basket of dense+thin candidate strategies.

The post-regime portfolio-diversity rule from the TODO mandates ranking candidates not by
standalone Sharpe alone but by their incremental contribution to a combined portfolio:
pairwise correlation, payoff-shape complementarity, and combined Sharpe under equal-weight
or inverse-volatility weighting.

This script consumes the freqtrade `--export trades` zip artifacts that
`run_tomac_one.py STRATEGY TIMEFRAME EXPORT_PATH` produces (which freqtrade
actually writes to `~/Auto-Quant/user_data/backtest_results/backtest-result-*.zip`)
and reports a compact scorecard:

- per-candidate annualized standalone metrics (Sharpe / Sortino / Calmar / max drawdown)
- pairwise daily-return Pearson correlation matrix
- equal-weight portfolio combined metrics
- inverse-volatility-weighted portfolio combined metrics
- conclusion on whether the basket clears the "different not just stronger" bar

Usage:
    cd <auto-quant-root>
    uv run python <ict-engine-repo>/\\
        support/scripts/auto_quant_external/portfolio_diversity_scorecard.py
"""
from __future__ import annotations

import json
import zipfile
from pathlib import Path

import numpy as np
import pandas as pd

BACKTEST_RESULTS = Path("user_data/backtest_results")
TRADING_DAYS = 252.0

CANDIDATES: list[tuple[str, str]] = [
    ("TomacNQ_RegimeTrendPullbackDense15m", "trend continuation pullback"),
    ("TomacNQ_RegimePersistenceClusterDense15m", "trend continuation persistence"),
    ("TomacNQ_RegimeLiquiditySweepReclaim15mWide", "mean reversion / sweep"),
    ("TomacNQ_RegimeSweepLowVIX15m", "mean reversion / sweep + low-VIX gate"),
    ("TomacNQ_RegimeSweepHighVIX15m", "mean reversion / sweep + high-VIX gate"),
    ("TomacNQ_RegimeVIXShockReversal15m", "vol-shock mean reversion"),
    ("TomacNQ_RegimeVIXShockReversalWide15m", "vol-shock mean reversion (widened)"),
    ("TomacNQ_RegimeVVIXDivergence15m", "vol-of-vol divergence"),
    ("TomacNQ_RegimeVIXBackwardation15m", "vix term-structure inversion"),
    ("TomacNQ_RegimeVIXBackwardationWide15m", "vix term-structure inversion (widened)"),
    ("TomacNQ_RegimeVRPCompression15m", "iv-hv compression regime"),
]
MIN_TRADES_FOR_INVERSE_VOL = 10


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
        return pd.DataFrame(columns=["close_date", "profit_ratio"])
    df = pd.DataFrame(trades)
    df["close_date"] = pd.to_datetime(df["close_date"], utc=True, errors="coerce")
    df = df.dropna(subset=["close_date"])
    return df[["close_date", "profit_ratio"]]


def daily_pnl_series(trades: pd.DataFrame) -> pd.Series:
    if trades.empty:
        return pd.Series(dtype=float)
    s = trades.copy()
    s["date"] = s["close_date"].dt.normalize()
    return s.groupby("date")["profit_ratio"].sum().sort_index()


def reindex_to_union(series_map: dict[str, pd.Series]) -> pd.DataFrame:
    if not series_map:
        return pd.DataFrame()
    union_idx = sorted({d for s in series_map.values() for d in s.index})
    if not union_idx:
        return pd.DataFrame()
    full_idx = pd.date_range(min(union_idx), max(union_idx), freq="D", tz="UTC")
    df = pd.DataFrame(
        {label: s.reindex(full_idx).fillna(0.0) for label, s in series_map.items()}
    )
    return df


def metrics(daily_returns: pd.Series) -> dict[str, float]:
    if daily_returns.empty or daily_returns.std() == 0:
        return {
            "annualized_return": 0.0,
            "annualized_vol": 0.0,
            "sharpe": 0.0,
            "sortino": 0.0,
            "max_drawdown": 0.0,
            "calmar": 0.0,
            "total_return": 0.0,
            "trading_days": float(len(daily_returns)),
        }
    mean = daily_returns.mean()
    std = daily_returns.std()
    annualized_return = mean * TRADING_DAYS
    annualized_vol = std * np.sqrt(TRADING_DAYS)
    sharpe = (mean / std) * np.sqrt(TRADING_DAYS) if std > 0 else 0.0
    downside = daily_returns[daily_returns < 0]
    if len(downside) > 1 and downside.std() > 0:
        sortino = (mean / downside.std()) * np.sqrt(TRADING_DAYS)
    else:
        sortino = 0.0
    cum = (1.0 + daily_returns).cumprod()
    running_max = cum.cummax()
    drawdown = (cum / running_max) - 1.0
    max_dd = float(drawdown.min())
    calmar = annualized_return / abs(max_dd) if max_dd < 0 else 0.0
    total_return = float(cum.iloc[-1] - 1.0)
    return {
        "annualized_return": float(annualized_return),
        "annualized_vol": float(annualized_vol),
        "sharpe": float(sharpe),
        "sortino": float(sortino),
        "max_drawdown": max_dd,
        "calmar": float(calmar),
        "total_return": total_return,
        "trading_days": float(len(daily_returns)),
    }


def emit_scorecard(
    daily_df: pd.DataFrame,
    candidates: list[tuple[str, str]],
) -> None:
    print("=" * 78)
    print("Post-regime portfolio-diversity scorecard")
    print("=" * 78)
    print()

    print("Standalone candidate metrics (annualized):")
    print("-" * 78)
    print(f"{'candidate':40s}{'family':32s}{'sharpe':>7s}")
    for col, family in candidates:
        m = metrics(daily_df[col])
        print(f"{col:40s}{family:32s}{m['sharpe']:>7.3f}")
    print()
    print("Detailed standalone metrics:")
    print("-" * 78)
    print(f"{'candidate':40s}{'sharpe':>7s}{'sortino':>8s}{'calmar':>7s}{'maxdd':>8s}{'totret':>8s}")
    for col, _ in candidates:
        m = metrics(daily_df[col])
        print(
            f"{col:40s}{m['sharpe']:>7.3f}{m['sortino']:>8.3f}"
            f"{m['calmar']:>7.2f}{m['max_drawdown']:>8.2%}{m['total_return']:>8.2%}"
        )
    print()

    print("Pairwise daily-return Pearson correlation:")
    print("-" * 78)
    corr = daily_df.corr()
    print(corr.round(3).to_string())
    print()

    print("Equal-weight portfolio (1/N) and inverse-volatility-weight portfolio:")
    print("-" * 78)
    cols = list(daily_df.columns)
    eq_weights = pd.Series(1.0 / len(cols), index=cols)
    eq_returns = (daily_df * eq_weights).sum(axis=1)
    eq_metrics = metrics(eq_returns)

    trade_counts = pd.Series(
        {col: int((daily_df[col] != 0).sum()) for col in cols}
    )
    inv_vol_eligible = trade_counts >= MIN_TRADES_FOR_INVERSE_VOL
    if inv_vol_eligible.sum() < 2:
        inv_vol_eligible = trade_counts > 0
    eligible_cols = [c for c in cols if inv_vol_eligible[c]]
    excluded_cols = [c for c in cols if not inv_vol_eligible[c]]
    vol = daily_df[eligible_cols].std()
    raw = (1.0 / vol).where(vol > 0, 0.0)
    inv_vol_weights = pd.Series(0.0, index=cols)
    inv_vol_weights.loc[eligible_cols] = raw / raw.sum()
    iv_returns = (daily_df * inv_vol_weights).sum(axis=1)
    iv_metrics = metrics(iv_returns)

    print(f"{'portfolio':40s}{'sharpe':>7s}{'sortino':>8s}{'calmar':>7s}{'maxdd':>8s}{'totret':>8s}")
    print(
        f"{'equal weight (1/N)':40s}{eq_metrics['sharpe']:>7.3f}{eq_metrics['sortino']:>8.3f}"
        f"{eq_metrics['calmar']:>7.2f}{eq_metrics['max_drawdown']:>8.2%}{eq_metrics['total_return']:>8.2%}"
    )
    print(
        f"{'inverse vol':40s}{iv_metrics['sharpe']:>7.3f}{iv_metrics['sortino']:>8.3f}"
        f"{iv_metrics['calmar']:>7.2f}{iv_metrics['max_drawdown']:>8.2%}{iv_metrics['total_return']:>8.2%}"
    )
    print()
    print(f"Inverse-volatility weights (eligible: trade_count >= {MIN_TRADES_FOR_INVERSE_VOL}):")
    for col in cols:
        flag = "" if inv_vol_eligible[col] else "  (excluded — too few trades)"
        print(f"  {col:40s}{inv_vol_weights[col]:>7.3f}{flag}")
    if excluded_cols:
        print(f"  excluded from inverse-vol: {excluded_cols}")
    print()

    best_standalone_sharpe = max(metrics(daily_df[c])["sharpe"] for c, _ in candidates)
    print("Conclusion (different-not-just-stronger test):")
    print("-" * 78)
    print(f"best standalone Sharpe:         {best_standalone_sharpe:>7.3f}")
    print(f"equal-weight portfolio Sharpe:  {eq_metrics['sharpe']:>7.3f}")
    print(f"inverse-vol portfolio Sharpe:   {iv_metrics['sharpe']:>7.3f}")
    if iv_metrics["sharpe"] > best_standalone_sharpe:
        print("PASS: inverse-vol portfolio Sharpe exceeds best standalone candidate.")
        print("The basket adds something different, not just stronger.")
    elif eq_metrics["sharpe"] > best_standalone_sharpe:
        print("PARTIAL: equal-weight portfolio Sharpe exceeds best standalone candidate.")
    else:
        print("FAIL: portfolio Sharpe does not exceed best standalone candidate.")
        print("Either the candidates are too correlated or one dominates by edge.")


def main() -> int:
    series_map: dict[str, pd.Series] = {}
    for strategy, _ in CANDIDATES:
        zip_path = find_latest_zip_for_strategy(strategy)
        if zip_path is None:
            print(f"WARN: no backtest export found for {strategy}", flush=True)
            continue
        trades = load_trades_from_zip(zip_path, strategy)
        if trades.empty:
            print(f"WARN: zero trades in export for {strategy}", flush=True)
            continue
        series_map[strategy] = daily_pnl_series(trades)

    if len(series_map) < 2:
        print("ERROR: need at least 2 candidates with trade exports", flush=True)
        return 2

    daily_df = reindex_to_union(series_map)
    emit_scorecard(daily_df, [(s, fam) for s, fam in CANDIDATES if s in series_map])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
