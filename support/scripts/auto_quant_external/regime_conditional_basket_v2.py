"""
regime_conditional_basket_v2.py — refined 2D (regime, term-structure) deny rules.

Slice 96's v1 used 1D regime-only deny rules and lifted basket Sharpe from
0.23 to 0.81 (equal-weight). Slice 97 identified that extending the rules
with the VIX9D/VIX3M term-structure dimension reveals additional structure
the v1 rules miss:

- LiquiditySweepReclaim15mWide should deny ALL Backwardation (not regime-conditional)
- TrendingCalm + Backwardation is bad for every trend candidate
- The blanket BearishStress deny is overly coarse — TrendPullback actually
  has positive Sharpe in BearishStress + Contango / DeepContango

This v2 implements 2D (regime, term) deny tuples per candidate. The expected
lift is +0.1 to +0.3 Sharpe over v1.
"""
from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table

BACKTEST_RESULTS = Path("user_data/backtest_results")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
TRADING_DAYS = 252.0
MIN_TRADES_FOR_INVERSE_VOL = 10

DenyTuple = tuple[str, str]

CANDIDATES_V2: list[tuple[str, str, set[DenyTuple], bool]] = [
    (
        "TomacNQ_RegimeTrendPullbackDense15m",
        "trend continuation pullback",
        {
            ("BearishStress", "Backwardation"),
            ("BearishStress", "FlatToBackward"),
            ("TrendingCalm", "Backwardation"),
        },
        False,
    ),
    (
        "TomacNQ_RegimePersistenceClusterDense15m",
        "trend continuation persistence",
        {
            ("BearishStress", "Backwardation"),
            ("BearishStress", "DeepContango"),
            ("TrendingCalm", "Backwardation"),
            ("Other", "Contango"),
        },
        False,
    ),
    (
        "TomacNQ_RegimeLiquiditySweepReclaim15mWide",
        "mean reversion / sweep",
        set(),
        True,
    ),
    (
        "TomacNQ_RegimeVRPCompression15m",
        "iv-hv compression regime",
        set(),
        False,
    ),
]


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
                "total_return": 0.0, "trade_days": 0.0}
    mean = daily_returns.mean()
    std = daily_returns.std()
    sharpe = (mean / std) * np.sqrt(TRADING_DAYS) if std > 0 else 0.0
    downside = daily_returns[daily_returns < 0]
    sortino = (mean / downside.std()) * np.sqrt(TRADING_DAYS) if (len(downside) > 1 and downside.std() > 0) else 0.0
    cum = (1.0 + daily_returns).cumprod()
    running_max = cum.cummax()
    dd = float((cum / running_max - 1.0).min())
    calmar = (mean * TRADING_DAYS) / abs(dd) if dd < 0 else 0.0
    total_return = float(cum.iloc[-1] - 1.0)
    return {
        "sharpe": float(sharpe),
        "sortino": float(sortino),
        "max_dd": dd,
        "calmar": float(calmar),
        "total_return": total_return,
        "trade_days": float((daily_returns != 0).sum()),
    }


def main() -> int:
    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term).rename("term")

    series_uncond: dict[str, pd.Series] = {}
    series_cond: dict[str, pd.Series] = {}
    standalone_rows: list[dict] = []

    for strategy, family, deny_tuples, deny_all_backwardation in CANDIDATES_V2:
        zip_path = find_latest_zip_for_strategy(strategy)
        if zip_path is None:
            continue
        trades = load_trades_from_zip(zip_path, strategy)
        if trades.empty:
            continue
        trades["entry_date"] = trades["open_date"].dt.normalize()
        trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
        trades["term"] = trades["entry_date"].map(term_class).fillna("unknown")
        deny_mask = trades.apply(
            lambda r: ((r["regime"], r["term"]) in deny_tuples)
            or (deny_all_backwardation and r["term"] == "Backwardation"),
            axis=1,
        )
        cond_trades = trades[~deny_mask]

        u = annual_metrics(daily_pnl(trades))
        c = annual_metrics(daily_pnl(cond_trades))
        series_uncond[strategy] = daily_pnl(trades)
        series_cond[strategy] = daily_pnl(cond_trades)

        rule_desc = []
        if deny_all_backwardation:
            rule_desc.append("ALL Backwardation")
        rule_desc.extend(f"{r}+{t}" for r, t in sorted(deny_tuples))
        standalone_rows.append({
            "candidate": strategy,
            "denied": "; ".join(rule_desc) or "none",
            "uncond_trades": int(len(trades)),
            "cond_trades": int(len(cond_trades)),
            "uncond_sharpe": u["sharpe"],
            "cond_sharpe": c["sharpe"],
            "delta_sharpe": c["sharpe"] - u["sharpe"],
            "uncond_max_dd": u["max_dd"],
            "cond_max_dd": c["max_dd"],
            "uncond_total": u["total_return"],
            "cond_total": c["total_return"],
        })

    print("=" * 110)
    print("V2 — refined 2D (regime, term-structure) deny rules")
    print("=" * 110)
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
        vol = uncond_df.loc[:, elig_u].std()
        raw = (1.0 / vol).where(vol > 0, 0.0)
        iv_u_w.loc[elig_u] = raw / raw.sum()
    if elig_c.any():
        vol = cond_df.loc[:, elig_c].std()
        raw = (1.0 / vol).where(vol > 0, 0.0)
        iv_c_w.loc[elig_c] = raw / raw.sum()
    iv_uncond = annual_metrics((uncond_df * iv_u_w).sum(axis=1))
    iv_cond = annual_metrics((cond_df * iv_c_w).sum(axis=1))

    print("Combined-portfolio comparison (v2 refined deny rules)")
    print("-" * 110)
    print(f"{'mode':32s}{'sharpe':>8s}{'sortino':>9s}{'calmar':>8s}{'maxdd':>9s}{'totret':>9s}")
    print(f"{'unconditional, equal-weight':32s}{eq_uncond['sharpe']:>8.3f}{eq_uncond['sortino']:>9.3f}"
          f"{eq_uncond['calmar']:>8.2f}{eq_uncond['max_dd']:>9.2%}{eq_uncond['total_return']:>9.2%}")
    print(f"{'conditional v2, equal-weight':32s}{eq_cond['sharpe']:>8.3f}{eq_cond['sortino']:>9.3f}"
          f"{eq_cond['calmar']:>8.2f}{eq_cond['max_dd']:>9.2%}{eq_cond['total_return']:>9.2%}")
    print(f"{'unconditional, inverse-vol':32s}{iv_uncond['sharpe']:>8.3f}{iv_uncond['sortino']:>9.3f}"
          f"{iv_uncond['calmar']:>8.2f}{iv_uncond['max_dd']:>9.2%}{iv_uncond['total_return']:>9.2%}")
    print(f"{'conditional v2, inverse-vol':32s}{iv_cond['sharpe']:>8.3f}{iv_cond['sortino']:>9.3f}"
          f"{iv_cond['calmar']:>8.2f}{iv_cond['max_dd']:>9.2%}{iv_cond['total_return']:>9.2%}")
    print()
    print(f"v2 Sharpe lift over unconditional (equal-weight): {eq_cond['sharpe'] - eq_uncond['sharpe']:+.3f}")
    print(f"v2 Sharpe lift over unconditional (inverse-vol):  {iv_cond['sharpe'] - iv_uncond['sharpe']:+.3f}")
    print()
    print("Slice 96 v1 reference (1D regime-only deny):")
    print("  conditional v1, equal-weight:  Sharpe 0.806  max_dd -4.76%  total +27.69%")
    print("  conditional v1, inverse-vol:   Sharpe 0.880  max_dd -4.31%  total +26.76%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
