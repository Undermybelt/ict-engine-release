"""
regime_conditional_basket.py — apply per-candidate regime filters and compare
conditional basket metrics to unconditional.

The post-regime portfolio-diversity rule says a regime classifier should improve
the deployable basket Sharpe by suppressing entries in regimes where each
candidate has negative or near-zero edge. Slice 94's per-regime attribution on
8Y data identified `BearishStress` (NQ drawdown < -7% + VIX >= 20) as the
universally-negative regime for trend candidates. This script tests whether
applying that rule lifts the conditional basket Sharpe over the unconditional.

Allowed-regimes rule (per Slice 95 8Y per-regime Sharpes):
    TrendPullbackDense15m:        all except BearishStress
    PersistenceClusterDense15m:   all except BearishStress
    LiquiditySweepReclaim15mWide: all (no regime is strongly negative)
    VRPCompression15m:            all (entry gates already filter BearishStress)

Output: unconditional vs conditional per-candidate Sharpe / total return / DD,
plus equal-weight and inverse-vol basket Sharpe in both modes.
"""
from __future__ import annotations

import json
import zipfile
from pathlib import Path

import numpy as np
import pandas as pd

import sys
sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table

BACKTEST_RESULTS = Path("/Users/thrill3r/Auto-Quant/user_data/backtest_results")
TRADING_DAYS = 252.0
MIN_TRADES_FOR_INVERSE_VOL = 10

CANDIDATES: list[tuple[str, str, set[str]]] = [
    (
        "TomacNQ_RegimeTrendPullbackDense15m",
        "trend continuation pullback",
        {"TrendingCalm", "TrendingNervous", "ChopRange", "Other"},
    ),
    (
        "TomacNQ_RegimePersistenceClusterDense15m",
        "trend continuation persistence",
        {"TrendingCalm", "TrendingNervous", "ChopRange", "Other"},
    ),
    (
        "TomacNQ_RegimeLiquiditySweepReclaim15mWide",
        "mean reversion / sweep",
        {"TrendingCalm", "TrendingNervous", "ChopRange", "BearishStress", "Other"},
    ),
    (
        "TomacNQ_RegimeVRPCompression15m",
        "iv-hv compression regime",
        {"TrendingCalm", "TrendingNervous", "ChopRange", "BearishStress", "Other"},
    ),
]


def find_latest_zip_for_strategy(strategy: str) -> Path | None:
    items: list[tuple[float, Path]] = []
    for meta_path in BACKTEST_RESULTS.glob("backtest-result-*.meta.json"):
        try:
            payload = json.loads(meta_path.read_text())
        except (OSError, json.JSONDecodeError):
            continue
        if strategy in payload:
            zip_path = meta_path.with_suffix("").with_suffix(".zip")
            if zip_path.exists():
                items.append((zip_path.stat().st_mtime, zip_path))
    if not items:
        return None
    items.sort(reverse=True)
    return items[0][1]


def load_trades_from_zip(zip_path: Path, strategy: str) -> pd.DataFrame:
    with zipfile.ZipFile(zip_path) as zf:
        result_name = next(
            n for n in zf.namelist()
            if n.endswith(".json") and "_config" not in n
        )
        with zf.open(result_name) as fh:
            payload = json.load(fh)
    strat = payload["strategy"][strategy]
    trades = strat.get("trades", [])
    if not trades:
        return pd.DataFrame(columns=["open_date", "close_date", "profit_ratio"])
    df = pd.DataFrame(trades)
    for col in ("open_date", "close_date"):
        df[col] = pd.to_datetime(df[col], utc=True, errors="coerce")
    df = df.dropna(subset=["open_date", "close_date"])
    return df[["open_date", "close_date", "profit_ratio"]]


def daily_pnl(trades: pd.DataFrame) -> pd.Series:
    if trades.empty:
        return pd.Series(dtype=float)
    s = trades.copy()
    s["date"] = s["close_date"].dt.normalize()
    return s.groupby("date")["profit_ratio"].sum().sort_index()


def annual_metrics(daily_returns: pd.Series) -> dict[str, float]:
    if daily_returns.empty or daily_returns.std() == 0:
        return {"sharpe": 0.0, "sortino": 0.0, "max_dd": 0.0, "calmar": 0.0,
                "total_return": 0.0, "trades_days": 0.0}
    mean = daily_returns.mean()
    std = daily_returns.std()
    sharpe = (mean / std) * np.sqrt(TRADING_DAYS) if std > 0 else 0.0
    downside = daily_returns[daily_returns < 0]
    sortino = (mean / downside.std()) * np.sqrt(TRADING_DAYS) if (len(downside) > 1 and downside.std() > 0) else 0.0
    cum = (1.0 + daily_returns).cumprod()
    running_max = cum.cummax()
    dd = (cum / running_max - 1.0).min()
    calmar = (mean * TRADING_DAYS) / abs(dd) if dd < 0 else 0.0
    total_return = float(cum.iloc[-1] - 1.0)
    return {
        "sharpe": float(sharpe),
        "sortino": float(sortino),
        "max_dd": float(dd),
        "calmar": float(calmar),
        "total_return": total_return,
        "trade_days": float((daily_returns != 0).sum()),
    }


def main() -> int:
    regimes = load_daily_regime_table()
    regime_lookup = regimes["regime"]

    series_uncond: dict[str, pd.Series] = {}
    series_cond: dict[str, pd.Series] = {}
    standalone_rows: list[dict] = []

    for strategy, family, allowed in CANDIDATES:
        zip_path = find_latest_zip_for_strategy(strategy)
        if zip_path is None:
            print(f"WARN: no zip for {strategy}", file=sys.stderr)
            continue
        trades = load_trades_from_zip(zip_path, strategy)
        if trades.empty:
            print(f"WARN: no trades for {strategy}")
            continue
        trades["entry_date"] = trades["open_date"].dt.normalize()
        trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
        denied = set(regimes["regime"].unique()) - allowed - {"unknown"}
        keep_mask = trades["regime"].isin(allowed)
        cond_trades = trades[keep_mask]

        uncond_daily = daily_pnl(trades)
        cond_daily = daily_pnl(cond_trades)
        series_uncond[strategy] = uncond_daily
        series_cond[strategy] = cond_daily

        u = annual_metrics(uncond_daily)
        c = annual_metrics(cond_daily)
        standalone_rows.append({
            "candidate": strategy,
            "family": family,
            "denied_regimes": ",".join(sorted(denied)) or "none",
            "uncond_trades": int(len(trades)),
            "cond_trades": int(len(cond_trades)),
            "uncond_sharpe": u["sharpe"],
            "cond_sharpe": c["sharpe"],
            "delta_sharpe": c["sharpe"] - u["sharpe"],
            "uncond_total": u["total_return"],
            "cond_total": c["total_return"],
            "uncond_max_dd": u["max_dd"],
            "cond_max_dd": c["max_dd"],
        })

    print("=" * 100)
    print("Per-candidate unconditional vs regime-conditional metrics on 8Y NQ/USD data")
    print("=" * 100)
    df = pd.DataFrame(standalone_rows)
    print(df.to_string(index=False, float_format=lambda x: f"{x:.4f}"))
    print()

    def reindex(series_map: dict[str, pd.Series]) -> pd.DataFrame:
        if not series_map:
            return pd.DataFrame()
        all_dates = sorted({d for s in series_map.values() for d in s.index})
        if not all_dates:
            return pd.DataFrame()
        idx = pd.date_range(min(all_dates), max(all_dates), freq="D", tz="UTC")
        return pd.DataFrame({k: s.reindex(idx).fillna(0.0) for k, s in series_map.items()})

    uncond_df = reindex(series_uncond)
    cond_df = reindex(series_cond)

    cols = list(uncond_df.columns)
    eq_w = pd.Series(1.0 / len(cols), index=cols)
    eq_uncond = annual_metrics((uncond_df * eq_w).sum(axis=1))
    eq_cond = annual_metrics((cond_df * eq_w).sum(axis=1))

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
        vol_u = uncond_df.loc[:, elig_u].std()
        raw = (1.0 / vol_u).where(vol_u > 0, 0.0)
        iv_u_w.loc[elig_u] = raw / raw.sum()
    if elig_c.any():
        vol_c = cond_df.loc[:, elig_c].std()
        raw = (1.0 / vol_c).where(vol_c > 0, 0.0)
        iv_c_w.loc[elig_c] = raw / raw.sum()
    iv_uncond = annual_metrics((uncond_df * iv_u_w).sum(axis=1))
    iv_cond = annual_metrics((cond_df * iv_c_w).sum(axis=1))

    print("Combined-portfolio comparison")
    print("-" * 100)
    print(f"{'mode':30s}{'sharpe':>8s}{'sortino':>9s}{'calmar':>8s}{'maxdd':>9s}{'totret':>9s}")
    print(f"{'unconditional, equal-weight':30s}{eq_uncond['sharpe']:>8.3f}{eq_uncond['sortino']:>9.3f}"
          f"{eq_uncond['calmar']:>8.2f}{eq_uncond['max_dd']:>9.2%}{eq_uncond['total_return']:>9.2%}")
    print(f"{'conditional, equal-weight':30s}{eq_cond['sharpe']:>8.3f}{eq_cond['sortino']:>9.3f}"
          f"{eq_cond['calmar']:>8.2f}{eq_cond['max_dd']:>9.2%}{eq_cond['total_return']:>9.2%}")
    print(f"{'unconditional, inverse-vol':30s}{iv_uncond['sharpe']:>8.3f}{iv_uncond['sortino']:>9.3f}"
          f"{iv_uncond['calmar']:>8.2f}{iv_uncond['max_dd']:>9.2%}{iv_uncond['total_return']:>9.2%}")
    print(f"{'conditional, inverse-vol':30s}{iv_cond['sharpe']:>8.3f}{iv_cond['sortino']:>9.3f}"
          f"{iv_cond['calmar']:>8.2f}{iv_cond['max_dd']:>9.2%}{iv_cond['total_return']:>9.2%}")
    print()
    print(f"Sharpe lift from regime filter (equal-weight): {eq_cond['sharpe'] - eq_uncond['sharpe']:+.3f}")
    print(f"Sharpe lift from regime filter (inverse-vol):  {iv_cond['sharpe'] - iv_uncond['sharpe']:+.3f}")
    print(f"Drawdown change   (equal-weight): {(eq_cond['max_dd'] - eq_uncond['max_dd']):+.2%}")
    print(f"Drawdown change   (inverse-vol):  {(iv_cond['max_dd'] - iv_uncond['max_dd']):+.2%}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
