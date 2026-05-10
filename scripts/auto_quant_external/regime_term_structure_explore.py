"""
regime_term_structure_explore.py — extend the daily regime taxonomy with a
VIX9D/VIX3M term-structure feature and test whether it adds discriminative
power beyond the 4-class regime.

The Slice 96 classifier (TrendingCalm / TrendingNervous / ChopRange /
BearishStress) lifts the conditional basket Sharpe from 0.23 to 0.81. The
question now: does adding VIX term-structure (VIX9D / VIX3M) split each of
those classes into sub-regimes with materially different per-candidate edge?

Term-structure semantics:
- vix9d / vix3m < 0.92: "DeepContango" (calm forward, complacent market)
- vix9d / vix3m in [0.92, 1.00]: "Contango" (normal)
- vix9d / vix3m in (1.00, 1.05]: "FlatToBackward" (mild stress)
- vix9d / vix3m > 1.05: "Backwardation" (front-month vol spike)

Output: 4-regime × 4-term-structure 2D Sharpe matrix per candidate. If a row /
column shows clear monotonic separation, we have an extra usable filter.
"""
from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

import pandas as pd

import sys as _sys
_sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table

BACKTEST_RESULTS = Path("/Users/thrill3r/Auto-Quant/user_data/backtest_results")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")

CANDIDATES: list[tuple[str, str]] = [
    ("TomacNQ_RegimeTrendPullbackDense15m", "trend continuation pullback"),
    ("TomacNQ_RegimePersistenceClusterDense15m", "trend continuation persistence"),
    ("TomacNQ_RegimeLiquiditySweepReclaim15mWide", "mean reversion / sweep"),
    ("TomacNQ_RegimeVRPCompression15m", "iv-hv compression regime"),
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


def main() -> int:
    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term).rename("term")

    print("Term-structure distribution over 2018-2025:")
    common_dates = term_class.reindex(regimes.index).dropna()
    print(common_dates.value_counts().to_string())
    print()
    print("Joint regime x term-structure distribution:")
    joint = pd.DataFrame({"regime": regime_lookup, "term": term_class}).dropna()
    pivot = joint.value_counts().unstack(fill_value=0)
    print(pivot.to_string())
    print()

    for strategy, family in CANDIDATES:
        zip_path = find_latest_zip_for_strategy(strategy)
        if zip_path is None:
            print(f"WARN: no zip for {strategy}", file=sys.stderr)
            continue
        trades = load_trades_from_zip(zip_path, strategy)
        if trades.empty:
            continue
        trades["entry_date"] = trades["open_date"].dt.normalize()
        trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
        trades["term"] = trades["entry_date"].map(term_class).fillna("unknown")
        print("-" * 90)
        print(f"{strategy} ({family})")
        print(f"trades total: {len(trades)}")
        rows: list[dict] = []
        for (regime, t), group in trades.groupby(["regime", "term"]):
            n = len(group)
            if n < 5:
                continue
            returns = group["profit_ratio"].astype(float)
            sharpe = returns.mean() / returns.std() if returns.std() > 0 else 0.0
            rows.append({
                "regime": regime,
                "term": t,
                "trades": n,
                "win_rate": (returns > 0).mean(),
                "mean_return": returns.mean(),
                "sharpe_per_trade": sharpe,
            })
        if not rows:
            print("  (no cells with >=5 trades)")
            continue
        df = pd.DataFrame(rows).sort_values(["regime", "term"]).reset_index(drop=True)
        print(df.to_string(index=False, float_format=lambda x: f"{x:.4f}"))
        print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
