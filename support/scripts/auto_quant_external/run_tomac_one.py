"""
run_tomac_one.py — additive single-strategy wrapper around run_tomac.py with optional timeframe override.

Lives in the ict-engine repo so it does not modify the user's Auto-Quant
runtime layout. Imports run_tomac from the current Auto-Quant checkout; reuses
its _build_exchange_with_synthetic_pairs synthetic-market injection so
NQ/USD-style pseudo-pairs pass freqtrade's exchange validation.

The motivation is the freqtrade quirk surfaced in Slice 82: config.tomac.json
declares timeframe="1h" and freqtrade applies that to the data loader before
the strategy class's `timeframe = "5m"` attribute is read, which trips the
faster->slower @informative merge guard. Passing `--timeframe 5m` through
the args dict overrides the config-level timeframe up front and resolves it.

Usage (run from the Auto-Quant checkout so the synthetic-market path is
relative to the user's runtime data dir):

    cd <auto-quant-root>
    uv run python <ict-engine-repo>/\\
        support/scripts/auto_quant_external/run_tomac_one.py STRATEGY [TIMEFRAME] [EXPORT_PATH] [PAIRS] [TIMERANGE]

When EXPORT_PATH is provided the run enables `--export trades` and writes the
per-trade backtest result there for downstream portfolio-diversity scoring.

When PAIRS (comma-separated, e.g. "SPY/USD,IWM/USD") is provided it overrides
the config's pair_whitelist for cross-market validation. The synthetic-market
injection is rebuilt against the new pair list.

When TIMERANGE (freqtrade format "YYYYMMDD-YYYYMMDD") is provided it limits
the backtest window — used for train/test split validation.
"""
from __future__ import annotations

import sys
from pathlib import Path

AUTO_QUANT = Path.cwd()
if str(AUTO_QUANT) not in sys.path:
    sys.path.insert(0, str(AUTO_QUANT))

import run_tomac as rt  # noqa: E402

from freqtrade.configuration import Configuration  # noqa: E402
from freqtrade.enums import RunMode  # noqa: E402
from freqtrade.optimize.backtesting import Backtesting  # noqa: E402


def run(
    strategy: str,
    timeframe: str | None = None,
    export_path: str | None = None,
    pairs: list[str] | None = None,
    timerange: str | None = None,
) -> int:
    args = {
        "config": [str(rt.CONFIG)],
        "user_data_dir": str(rt.USER_DATA),
        "datadir": str(rt.DATA_DIR),
        "strategy": strategy,
        "strategy_path": str(rt.STRATEGIES_DIR),
        "export": "trades" if export_path else "none",
        "exportfilename": Path(export_path) if export_path else None,
        "cache": "none",
    }
    if timeframe:
        args["timeframe"] = timeframe
    if pairs:
        args["pairs"] = pairs
    if timerange:
        args["timerange"] = timerange
    config = Configuration(args, RunMode.BACKTEST).get_config()
    if pairs:
        config["exchange"]["pair_whitelist"] = pairs
    exchange = rt._build_exchange_with_synthetic_pairs(config)
    bt = Backtesting(config, exchange=exchange)
    bt.start()
    metrics = rt.extract_metrics(bt.results, strategy)
    rt.emit_block(
        strategy,
        rt.get_commit(),
        config["exchange"]["pair_whitelist"],
        metrics,
    )
    return 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(
            "Usage: run_tomac_one.py STRATEGY [TIMEFRAME] [EXPORT_PATH] [PAIRS] [TIMERANGE]",
            file=sys.stderr,
        )
        raise SystemExit(2)
    strategy = sys.argv[1]
    timeframe = sys.argv[2] if len(sys.argv) > 2 else None
    export_path = sys.argv[3] if len(sys.argv) > 3 else None
    pairs_arg = sys.argv[4] if len(sys.argv) > 4 else None
    pairs = [p.strip() for p in pairs_arg.split(",") if p.strip()] if pairs_arg else None
    timerange = sys.argv[5] if len(sys.argv) > 5 else None
    raise SystemExit(run(strategy, timeframe, export_path, pairs, timerange))
