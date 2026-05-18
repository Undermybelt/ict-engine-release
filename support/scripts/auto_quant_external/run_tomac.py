"""
run_tomac.py — sibling to run.py for backtesting NON-CRYPTO assets via the
same FreqTrade engine, with zero modifications to FreqTrade source.

Reads strategies from `user_data/strategies_external/` (separate dir so the
master run.py never tries to load these), uses `config.tomac.json` (separate
config so master config.json stays untouched), and treats whatever feathers
are present in user_data/data/ matching the configured pair_whitelist as
the universe.

This script is purely additive — it does NOT modify run.py, config.json,
prepare.py, _template.py.example, or anything else under the program.md
"do not modify" contract.
"""
from __future__ import annotations

import json
import subprocess
import sys
import traceback
from pathlib import Path
from typing import Any

from freqtrade.configuration import Configuration
from freqtrade.enums import RunMode
from freqtrade.optimize.backtesting import Backtesting
from freqtrade.resolvers import ExchangeResolver

PROJECT_DIR = Path(__file__).parent.resolve()
USER_DATA = PROJECT_DIR / "user_data"
STRATEGIES_DIR = USER_DATA / "strategies_external"
DATA_DIR = USER_DATA / "data"
CONFIG = PROJECT_DIR / "config.tomac.json"


def _synthetic_market(pair: str, trading_mode: str) -> dict[str, Any]:
    """Build a minimal ccxt-shaped market entry for a non-crypto pseudo-pair.

    FreqTrade's IPairList._whitelist_for_active_markets requires every
    whitelisted pair to be present in exchange.markets and to satisfy
    market_is_tradable (active=True, quote==stake_currency, spot type).
    These keys cover those checks without modifying freqtrade source.
    """
    base, quote = pair.split("/", 1)
    is_futures = trading_mode == "futures"
    return {
        "id": pair.replace("/", ""),
        "symbol": pair,
        "base": base,
        "quote": quote,
        "active": True,
        "type": "swap" if is_futures else "spot",
        "spot": not is_futures,
        "margin": is_futures,
        "swap": is_futures,
        "future": False,
        "option": False,
        "contract": is_futures,
        "linear": True if is_futures else None,
        "inverse": False if is_futures else None,
        "settle": quote if is_futures else None,
        "settleId": quote if is_futures else None,
        "expiry": None,
        "expiryDatetime": None,
        "strike": None,
        "optionType": None,
        "taker": 0.0,
        "maker": 0.0,
        "percentage": True,
        "tierBased": False,
        "feeSide": "get",
        # Freqtrade may run this synthetic market through ccxt tick-size
        # precision semantics. Decimal-place integers like 8 can then be
        # interpreted as an 8-unit amount tick and round BTC-sized orders to 0.
        "precision": {"amount": 0.00000001, "price": 0.01, "base": 0.00000001, "quote": 0.01},
        "limits": {
            "amount": {"min": 0, "max": None},
            "price": {"min": 0, "max": None},
            "cost": {"min": 0, "max": None},
            "leverage": {"min": 1, "max": 20 if is_futures else 1},
        },
        "info": {},
    }


def _build_exchange_with_synthetic_pairs(config: dict[str, Any]):
    # Synthetic non-crypto backtests should not block on remote exchange
    # market metadata. We populate the minimal market map locally instead.
    exchange = ExchangeResolver.load_exchange(
        config,
        validate=False,
        load_leverage_tiers=False,
    )
    synthetic = config["exchange"].get("pair_whitelist", [])
    trading_mode = config.get("trading_mode", "spot")
    if exchange._api.markets is None:
        exchange._api.markets = {}
    if exchange._api_async.markets is None:
        exchange._api_async.markets = {}
    for pair in synthetic:
        market = _synthetic_market(pair, trading_mode)
        exchange._markets[pair] = market
        exchange._api.markets[pair] = market
        exchange._api_async.markets[pair] = market
    return exchange


def _get(d: dict[str, Any], *keys: str, default: float = 0.0) -> float:
    for k in keys:
        if k in d and d[k] is not None:
            try:
                return float(d[k])
            except (TypeError, ValueError):
                continue
    return default


def _entry_metrics(entry: dict[str, Any]) -> dict[str, float]:
    return {
        "sharpe": _get(entry, "sharpe", "sharpe_ratio"),
        "sortino": _get(entry, "sortino", "sortino_ratio"),
        "calmar": _get(entry, "calmar", "calmar_ratio"),
        "total_profit_pct": _get(entry, "profit_total_pct"),
        "max_drawdown_pct": -abs(_get(entry, "max_drawdown_account")) * 100,
        "trade_count": int(_get(entry, "trades", "total_trades")),
        "win_rate_pct": _get(entry, "winrate") * 100,
        "profit_factor": _get(entry, "profit_factor"),
    }


def get_commit() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=str(PROJECT_DIR),
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except Exception:
        return "unknown"


def discover_strategies() -> list[str]:
    if not STRATEGIES_DIR.exists():
        return []
    names = []
    for path in sorted(STRATEGIES_DIR.glob("*.py")):
        if path.stem.startswith("_"):
            continue
        names.append(path.stem)
    return names


def run_backtest(strategy_name: str) -> dict[str, Any]:
    args = {
        "config": [str(CONFIG)],
        "user_data_dir": str(USER_DATA),
        "datadir": str(DATA_DIR),
        "strategy": strategy_name,
        "strategy_path": str(STRATEGIES_DIR),
        "export": "none",
        "exportfilename": None,
        "cache": "none",
    }
    config = Configuration(args, RunMode.BACKTEST).get_config()
    exchange = _build_exchange_with_synthetic_pairs(config)
    bt = Backtesting(config, exchange=exchange)
    bt.start()
    return bt.results


def extract_metrics(results: dict[str, Any], strategy_name: str) -> dict[str, Any]:
    strat = results.get("strategy", {}).get(strategy_name, {}) or {}
    per_pair_list = strat.get("results_per_pair", []) or []
    aggregate: dict[str, float] = {}
    per_pair: dict[str, dict[str, float]] = {}
    for entry in per_pair_list:
        key = entry.get("key", "")
        metrics = _entry_metrics(entry)
        if key == "TOTAL":
            aggregate = metrics
        elif key:
            per_pair[key] = metrics
    if not aggregate:
        aggregate = _entry_metrics(strat)
    return {"aggregate": aggregate, "per_pair": per_pair}


def emit_block(strategy_name: str, commit: str, config_pairs: list[str], metrics: dict[str, Any]) -> None:
    agg = metrics["aggregate"]
    print("---")
    print(f"strategy:         {strategy_name}")
    print(f"commit:           {commit}")
    print(f"config:           {CONFIG.name}")
    print(f"pairs:            {','.join(config_pairs)}")
    print(f"sharpe:           {agg['sharpe']:.4f}")
    print(f"sortino:          {agg['sortino']:.4f}")
    print(f"calmar:           {agg['calmar']:.4f}")
    print(f"total_profit_pct: {agg['total_profit_pct']:.4f}")
    print(f"max_drawdown_pct: {agg['max_drawdown_pct']:.4f}")
    print(f"trade_count:      {agg['trade_count']}")
    print(f"win_rate_pct:     {agg['win_rate_pct']:.4f}")
    print(f"profit_factor:    {agg['profit_factor']:.4f}")
    if metrics["per_pair"]:
        print("per_pair:")
        for pair, m in metrics["per_pair"].items():
            print(
                f"  {pair}: sharpe={m['sharpe']:.4f} trades={m['trade_count']} "
                f"profit_pct={m['total_profit_pct']:.2f} dd_pct={m['max_drawdown_pct']:.2f} "
                f"wr={m['win_rate_pct']:.1f} pf={m['profit_factor']:.2f}"
            )
    print()


def main() -> int:
    if not CONFIG.exists():
        print(f"ERROR: {CONFIG} not found", file=sys.stderr)
        return 2
    if not STRATEGIES_DIR.exists():
        print(f"ERROR: {STRATEGIES_DIR} not found; create it and add at least one strategy", file=sys.stderr)
        return 2
    config_pairs = json.loads(CONFIG.read_text())["exchange"]["pair_whitelist"]
    commit = get_commit()
    strategies = discover_strategies()
    if not strategies:
        print(f"ERROR: no strategies discovered in {STRATEGIES_DIR}", file=sys.stderr)
        return 2

    succeeded = failed = 0
    for name in strategies:
        try:
            results = run_backtest(name)
            metrics = extract_metrics(results, name)
            emit_block(name, commit, config_pairs, metrics)
            succeeded += 1
        except Exception as exc:  # noqa: BLE001 — mirror run.py's per-strategy isolation
            failed += 1
            print("---")
            print(f"strategy:         {name}")
            print(f"commit:           {commit}")
            print(f"status:           ERROR")
            print(f"error_type:       {type(exc).__name__}")
            print(f"error_msg:        {exc}")
            print("traceback:")
            traceback.print_exc()
            print()
    print(f"Done: {succeeded} succeeded, {failed} failed.")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
