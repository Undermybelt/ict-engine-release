"""
pandas_regime_conditional_v3.py — apply the V3 train-derived regime classifier
to the pandas alt-backtest trade list to get the most honest deployable
basket Sharpe estimate on full 8Y data.

Slice 99 derived deny rules from a 5Y train period, applied to a 3Y test
period, validated that the rules generalize. Slice 103 demonstrated that
the freqtrade backtest had a 2024-2025 drought; the pandas alt-backtester
runs cleanly through 2025-12-30. This script combines both: re-run pandas
backtest, split trades into train (2018-2022) and test (2023-2025), auto-
derive deny rules from train, apply to test, report basket metrics.

The key difference from Slice 99: Slice 99 used freqtrade's truncated trade
list (which had zero entries in 2024-2025). This script uses the pandas
trade list which fires entries throughout 2018-2025. The conditional Sharpe
estimate is therefore the first drought-fixed honest number.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent))
from regime_attribution import load_daily_regime_table
from pandas_alt_backtest import load_indicators as pandas_load_indicators, simulate as pandas_simulate

VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
TRADING_DAYS = 252.0
MIN_TRADES_PER_CELL = 30
TRAIN_END = pd.Timestamp("2023-01-01", tz="UTC")


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
    total = float(cum.iloc[-1] - 1.0)
    return {
        "sharpe": float(sharpe),
        "sortino": float(sortino),
        "max_dd": dd,
        "calmar": float(calmar),
        "total_return": total,
        "trade_days": float((daily_returns != 0).sum()),
    }


def derive_deny_rules(train_trades: pd.DataFrame) -> tuple[set[tuple[str, str]], list[dict]]:
    cells: list[dict] = []
    deny: set[tuple[str, str]] = set()
    if train_trades.empty:
        return deny, cells
    for (regime, term), group in train_trades.groupby(["regime", "term"]):
        n = len(group)
        if n < MIN_TRADES_PER_CELL:
            continue
        returns = group["profit_ratio"].astype(float)
        sharpe = returns.mean() / returns.std() if returns.std() > 0 else 0.0
        cells.append({
            "regime": regime,
            "term": term,
            "trades": n,
            "sharpe_per_trade": float(sharpe),
        })
        if sharpe < 0 and regime != "unknown" and term != "unknown":
            deny.add((regime, term))
    return deny, cells


def main() -> int:
    print("Generating pandas trade list...")
    df = pandas_load_indicators()
    trades = pandas_simulate(df)
    if trades.empty:
        print("ERROR: pandas backtest produced no trades")
        return 1

    regimes = load_daily_regime_table()
    term = load_term_structure()
    regime_lookup = regimes["regime"]
    term_class = term.apply(classify_term).rename("term")

    trades["entry_date"] = trades["open_date"].dt.normalize()
    trades["regime"] = trades["entry_date"].map(regime_lookup).fillna("unknown")
    trades["term"] = trades["entry_date"].map(term_class).fillna("unknown")

    train_mask = trades["entry_date"] < TRAIN_END
    test_mask = trades["entry_date"] >= TRAIN_END
    train_trades = trades[train_mask]
    test_trades = trades[test_mask]

    print(f"\ntotal trades: {len(trades)}")
    print(f"  train (2018-2022): {len(train_trades)}")
    print(f"  test  (2023-2025): {len(test_trades)}")
    print()

    deny, cells = derive_deny_rules(train_trades)
    print(f"Train cell Sharpes (cells with >= {MIN_TRADES_PER_CELL} trades), sorted:")
    for c in sorted(cells, key=lambda x: x["sharpe_per_trade"]):
        print(f"  {c['regime']:>16s} x {c['term']:>16s}: trades={c['trades']:>5d}  sharpe={c['sharpe_per_trade']:+.4f}")
    print()
    print(f"Auto-derived deny rules from train (cells with Sharpe < 0):")
    if deny:
        for r, t in sorted(deny):
            print(f"  {r} x {t}")
    else:
        print("  (none)")
    print()

    cond_train = train_trades[~train_trades.apply(lambda r: (r["regime"], r["term"]) in deny, axis=1)]
    cond_test = test_trades[~test_trades.apply(lambda r: (r["regime"], r["term"]) in deny, axis=1)]
    cond_all = trades[~trades.apply(lambda r: (r["regime"], r["term"]) in deny, axis=1)]

    print("=" * 90)
    print("Pandas alt-backtest — drought-fixed regime-conditional basket comparison")
    print("=" * 90)
    print(f"{'window/mode':40s}{'trades':>8s}{'sharpe':>8s}{'sortino':>9s}{'maxdd':>9s}{'totret':>9s}")
    for label, t_set in [
        ("FULL 8Y unconditional", trades),
        ("FULL 8Y V3 conditional", cond_all),
        ("TRAIN 5Y unconditional", train_trades),
        ("TRAIN 5Y V3 conditional", cond_train),
        ("TEST 3Y unconditional", test_trades),
        ("TEST 3Y V3 conditional (rules from train)", cond_test),
    ]:
        if t_set.empty:
            continue
        m = annual_metrics(daily_pnl(t_set))
        print(f"{label:40s}{len(t_set):>8d}{m['sharpe']:>8.3f}{m['sortino']:>9.3f}{m['max_dd']:>9.2%}{m['total_return']:>9.2%}")
    print()

    test_uncond = annual_metrics(daily_pnl(test_trades))
    test_cond = annual_metrics(daily_pnl(cond_test))
    print(f"Sharpe lift on TEST from regime filter: {test_cond['sharpe'] - test_uncond['sharpe']:+.3f}")
    print(f"Drawdown change on TEST:                {test_cond['max_dd'] - test_uncond['max_dd']:+.2%}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
