"""
regime_conditional_basket_v3_oos.py — derive deny rules from TRAIN period,
apply to held-out TEST period, validate that the V2 lift is not in-sample fit.

Slice 98's V2 lift over V1 (basket Sharpe 0.88 -> 1.06) came from refined
2D (regime, term-structure) deny rules. Those rules were derived by
inspecting per-cell Sharpes on 8Y data — exactly the data they were applied
to. This is feature selection on the evaluation set, so part of the lift
may be in-sample fit rather than a real generalizable edge.

This script does it honestly:
1. Split trades by entry-day timerange. Train: 2018-2022 (5Y, COVID + 2022
   bear regime mix). Test: 2023-2025 (3Y, the in-sample-favorable period).
2. On train trades, compute per-candidate per-cell (regime, term) Sharpe.
   Generate deny rules: cells where Sharpe < 0 AND trade count >= 10.
3. Apply those train-derived deny rules to TEST trades (as well as the
   universal "deny ALL Backwardation for Sweep" heuristic kept from Slice 97).
4. Report test-period basket Sharpe under: unconditional, V1 (regime only),
   V2 (full-data fitted from Slice 98), V3 (train-derived).

If V3 ~= V2 on test, the V2 lift is real. If V3 << V2 on test, V2 was
in-sample fit.
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

BACKTEST_RESULTS = Path("/Users/thrill3r/Auto-Quant/user_data/backtest_results")
VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
TRADING_DAYS = 252.0
MIN_TRADES_FOR_INVERSE_VOL = 10
MIN_TRAIN_TRADES_PER_CELL = 10
TRAIN_END = pd.Timestamp("2023-01-01", tz="UTC")
TEST_START = pd.Timestamp("2023-01-01", tz="UTC")

CANDIDATES: list[tuple[str, str]] = [
    ("TomacNQ_RegimeTrendPullbackDense15m", "trend continuation pullback"),
    ("TomacNQ_RegimePersistenceClusterDense15m", "trend continuation persistence"),
    ("TomacNQ_RegimeLiquiditySweepReclaim15mWide", "mean reversion / sweep"),
    ("TomacNQ_RegimeVRPCompression15m", "iv-hv compression regime"),
]

V1_DENY: dict[str, set] = {
    "TomacNQ_RegimeTrendPullbackDense15m": {"BearishStress"},
    "TomacNQ_RegimePersistenceClusterDense15m": {"BearishStress"},
    "TomacNQ_RegimeLiquiditySweepReclaim15mWide": set(),
    "TomacNQ_RegimeVRPCompression15m": set(),
}

V2_DENY_FULL: dict[str, set[tuple[str, str]]] = {
    "TomacNQ_RegimeTrendPullbackDense15m": {
        ("BearishStress", "Backwardation"),
        ("BearishStress", "FlatToBackward"),
        ("TrendingCalm", "Backwardation"),
    },
    "TomacNQ_RegimePersistenceClusterDense15m": {
        ("BearishStress", "Backwardation"),
        ("BearishStress", "DeepContango"),
        ("TrendingCalm", "Backwardation"),
        ("Other", "Contango"),
    },
    "TomacNQ_RegimeLiquiditySweepReclaim15mWide": set(),
    "TomacNQ_RegimeVRPCompression15m": set(),
}

V2_DENY_ALL_BACKWARDATION = {"TomacNQ_RegimeLiquiditySweepReclaim15mWide"}


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


def derive_train_deny_rules(
    train_trades: pd.DataFrame,
    deny_all_backward: bool = False,
) -> tuple[set[tuple[str, str]], list[dict]]:
    if train_trades.empty:
        return set(), []
    cells: list[dict] = []
    deny: set[tuple[str, str]] = set()
    for (regime, term), group in train_trades.groupby(["regime", "term"]):
        n = len(group)
        if n < MIN_TRAIN_TRADES_PER_CELL:
            continue
        returns = group["profit_ratio"].astype(float)
        if returns.std() == 0:
            sharpe = 0.0
        else:
            sharpe = returns.mean() / returns.std()
        cells.append({
            "regime": regime,
            "term": term,
            "trades": n,
            "sharpe_per_trade": sharpe,
        })
        if sharpe < 0 and not (regime == "unknown" or term == "unknown"):
            deny.add((regime, term))
    return deny, cells


def daily_pnl(trades: pd.DataFrame) -> pd.Series:
    if trades.empty:
        return pd.Series(dtype=float)
    s = trades.copy()
    s["date"] = s["close_date"].dt.normalize()
    return s.groupby("date")["profit_ratio"].sum().sort_index()


def annual_metrics(daily_returns: pd.Series) -> dict[str, float]:
    if daily_returns.empty or daily_returns.std() == 0:
        return {"sharpe": 0.0, "sortino": 0.0, "max_dd": 0.0, "calmar": 0.0,
                "total_return": 0.0}
    mean = daily_returns.mean()
    std = daily_returns.std()
    sharpe = (mean / std) * np.sqrt(TRADING_DAYS) if std > 0 else 0.0
    downside = daily_returns[daily_returns < 0]
    sortino = (mean / downside.std()) * np.sqrt(TRADING_DAYS) if (len(downside) > 1 and downside.std() > 0) else 0.0
    cum = (1.0 + daily_returns).cumprod()
    running_max = cum.cummax()
    dd = float((cum / running_max - 1.0).min())
    calmar = (mean * TRADING_DAYS) / abs(dd) if dd < 0 else 0.0
    total = float(cum.iloc[-1] - 1.0)
    return {"sharpe": float(sharpe), "sortino": float(sortino), "max_dd": dd,
            "calmar": float(calmar), "total_return": total}


def basket_metrics(series_map: dict[str, pd.Series]) -> tuple[dict, dict]:
    if not series_map:
        return {}, {}
    all_dates = sorted({d for s in series_map.values() for d in s.index})
    if not all_dates:
        return {}, {}
    idx = pd.date_range(min(all_dates), max(all_dates), freq="D", tz="UTC")
    df = pd.DataFrame({k: s.reindex(idx).fillna(0.0) for k, s in series_map.items()})
    cols = list(df.columns)
    eq_w = pd.Series(1.0 / len(cols), index=cols)
    eq = annual_metrics((df * eq_w).sum(axis=1))
    counts = pd.Series({c: int((df[c] != 0).sum()) for c in cols})
    elig = counts >= MIN_TRADES_FOR_INVERSE_VOL
    if elig.sum() < 2:
        elig = counts > 0
    iv_w = pd.Series(0.0, index=cols)
    if elig.any():
        vol = df.loc[:, elig].std()
        raw = (1.0 / vol).where(vol > 0, 0.0)
        iv_w.loc[elig] = raw / raw.sum()
    iv = annual_metrics((df * iv_w).sum(axis=1))
    return eq, iv


def main() -> int:
    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term).rename("term")

    derived_rules: dict[str, set[tuple[str, str]]] = {}
    test_uncond: dict[str, pd.Series] = {}
    test_v1: dict[str, pd.Series] = {}
    test_v2_full: dict[str, pd.Series] = {}
    test_v3_train: dict[str, pd.Series] = {}

    print("=" * 100)
    print("Slice 99 — train-derived deny rules out-of-sample validation")
    print("=" * 100)

    for strategy, family in CANDIDATES:
        zip_path = find_latest_zip_for_strategy(strategy)
        if zip_path is None:
            continue
        trades = load_trades_from_zip(zip_path, strategy)
        if trades.empty:
            continue
        trades["entry_date"] = trades["open_date"].dt.normalize()
        trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
        trades["term"] = trades["entry_date"].map(term_class).fillna("unknown")

        train_mask = trades["entry_date"] < TRAIN_END
        test_mask = trades["entry_date"] >= TEST_START
        train_trades = trades[train_mask]
        test_trades = trades[test_mask]

        deny_all_backward = strategy in V2_DENY_ALL_BACKWARDATION
        derived, cells = derive_train_deny_rules(train_trades, deny_all_backward)
        derived_rules[strategy] = derived

        v1_keep = ~test_trades["regime"].isin(V1_DENY[strategy])
        v2_full_keep = ~test_trades.apply(
            lambda r: ((r["regime"], r["term"]) in V2_DENY_FULL[strategy])
            or (deny_all_backward and r["term"] == "Backwardation"),
            axis=1,
        )
        v3_train_keep = ~test_trades.apply(
            lambda r: ((r["regime"], r["term"]) in derived)
            or (deny_all_backward and r["term"] == "Backwardation"),
            axis=1,
        )

        test_uncond[strategy] = daily_pnl(test_trades)
        test_v1[strategy] = daily_pnl(test_trades[v1_keep])
        test_v2_full[strategy] = daily_pnl(test_trades[v2_full_keep])
        test_v3_train[strategy] = daily_pnl(test_trades[v3_train_keep])

        print("-" * 100)
        print(f"{strategy} ({family})")
        print(f"  train trades: {len(train_trades)}  test trades: {len(test_trades)}")
        print(f"  train cell Sharpes (cells with >= {MIN_TRAIN_TRADES_PER_CELL} trades):")
        for c in sorted(cells, key=lambda x: x["sharpe_per_trade"]):
            print(f"    {c['regime']:>16s} x {c['term']:>16s}: trades={c['trades']:>5d}  sharpe={c['sharpe_per_trade']:+.4f}")
        print(f"  derived deny rules from train (Sharpe < 0): {sorted(derived) if derived else 'none'}")
        if deny_all_backward:
            print(f"  plus universal: deny ALL Backwardation (kept from Slice 97 heuristic)")
        u = annual_metrics(test_uncond[strategy])
        v1 = annual_metrics(test_v1[strategy])
        v2 = annual_metrics(test_v2_full[strategy])
        v3 = annual_metrics(test_v3_train[strategy])
        print(f"  test Sharpe — uncond={u['sharpe']:+.3f}  v1={v1['sharpe']:+.3f}  v2(full-fit)={v2['sharpe']:+.3f}  v3(train-derived)={v3['sharpe']:+.3f}")
        print()

    print("=" * 100)
    print("Test-period basket comparison")
    print("=" * 100)
    eq_u, iv_u = basket_metrics(test_uncond)
    eq_v1, iv_v1 = basket_metrics(test_v1)
    eq_v2, iv_v2 = basket_metrics(test_v2_full)
    eq_v3, iv_v3 = basket_metrics(test_v3_train)

    print(f"{'mode':40s}{'sharpe':>8s}{'sortino':>9s}{'maxdd':>9s}{'totret':>9s}")
    for label, eq in [("test unconditional, eq", eq_u),
                       ("test V1 (regime only), eq", eq_v1),
                       ("test V2 (full-data fitted), eq", eq_v2),
                       ("test V3 (TRAIN-derived), eq", eq_v3)]:
        print(f"{label:40s}{eq['sharpe']:>8.3f}{eq['sortino']:>9.3f}{eq['max_dd']:>9.2%}{eq['total_return']:>9.2%}")
    print()
    for label, iv in [("test unconditional, inv-vol", iv_u),
                       ("test V1 (regime only), inv-vol", iv_v1),
                       ("test V2 (full-data fitted), inv-vol", iv_v2),
                       ("test V3 (TRAIN-derived), inv-vol", iv_v3)]:
        print(f"{label:40s}{iv['sharpe']:>8.3f}{iv['sortino']:>9.3f}{iv['max_dd']:>9.2%}{iv['total_return']:>9.2%}")
    print()
    if eq_v3.get("sharpe", 0) > 0 and eq_v2.get("sharpe", 0) > 0:
        v3_to_v2 = eq_v3["sharpe"] / eq_v2["sharpe"] if eq_v2["sharpe"] > 0 else 0
        print(f"V3 / V2 Sharpe ratio (eq-weight, test): {v3_to_v2:.2%}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
