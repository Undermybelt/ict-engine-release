from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any


def _as_float(row: dict[str, Any], key: str) -> float:
    return float(row[key])


def _side(row: dict[str, Any]) -> int:
    value = int(float(row.get("side", 0)))
    if value > 0:
        return 1
    if value < 0:
        return -1
    return 0


def _directional_return(side: int, entry: float, price: float) -> float:
    return side * ((price - entry) / entry)


def _bar_mfe_mae(side: int, entry: float, high: float, low: float) -> tuple[float, float]:
    returns = [
        _directional_return(side, entry, high),
        _directional_return(side, entry, low),
    ]
    return max(returns), min(returns)


def triple_barrier_labels(
    rows: list[dict[str, Any]],
    *,
    pt_mult: float,
    sl_mult: float,
    max_holding_bars: int,
    cost_bps: float = 0.0,
) -> list[dict[str, Any]]:
    """Label non-zero `side` rows with conservative triple-barrier outcomes.

    `realized_R` is net directional return divided by the stop distance. If a
    bar touches stop and take-profit simultaneously, stop wins to avoid optimistic
    intrabar path assumptions.
    """
    if pt_mult <= 0 or sl_mult <= 0:
        raise ValueError("pt_mult and sl_mult must be positive")
    if max_holding_bars < 1:
        raise ValueError("max_holding_bars must be >= 1")

    labels: list[dict[str, Any]] = []
    cost_return = cost_bps / 10_000.0

    for entry_index, row in enumerate(rows):
        side = _side(row)
        if side == 0:
            continue
        entry = _as_float(row, "close")
        if entry <= 0:
            continue

        horizon = min(len(rows) - 1, entry_index + max_holding_bars)
        barrier_hit = "vertical"
        exit_index = horizon
        exit_price = _as_float(rows[horizon], "close")
        mfe = 0.0
        mae = 0.0

        for i in range(entry_index + 1, horizon + 1):
            current = rows[i]
            high = _as_float(current, "high")
            low = _as_float(current, "low")
            bar_mfe, bar_mae = _bar_mfe_mae(side, entry, high, low)
            mfe = max(mfe, bar_mfe)
            mae = min(mae, bar_mae)

            hit_stop = bar_mae <= -sl_mult
            hit_take_profit = bar_mfe >= pt_mult
            if hit_stop:
                barrier_hit = "stop_loss"
                exit_index = i
                exit_price = entry * (1.0 - side * sl_mult)
                break
            if hit_take_profit:
                barrier_hit = "take_profit"
                exit_index = i
                exit_price = entry * (1.0 + side * pt_mult)
                break

        gross_return = _directional_return(side, entry, exit_price)
        net_return = gross_return - cost_return
        realized_r = net_return / sl_mult
        labels.append(
            {
                "entry_index": entry_index,
                "exit_index": exit_index,
                "entry_timestamp": row.get("timestamp"),
                "exit_timestamp": rows[exit_index].get("timestamp"),
                "side": side,
                "entry_price": entry,
                "exit_price": exit_price,
                "barrier_hit": barrier_hit,
                "gross_return": gross_return,
                "net_return": net_return,
                "realized_R": realized_r,
                "mfe": mfe,
                "mae": mae,
                "time_to_hit": exit_index - entry_index,
                "meta_label": 1 if realized_r > 0 else 0,
            }
        )
    return labels


def _read_csv(path: Path) -> list[dict[str, Any]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def _write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=False) + "\n")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build triple-barrier labels from OHLCV side rows.")
    parser.add_argument("--input-csv", required=True)
    parser.add_argument("--output-jsonl", required=True)
    parser.add_argument("--pt-mult", type=float, default=0.02)
    parser.add_argument("--sl-mult", type=float, default=0.01)
    parser.add_argument("--max-holding-bars", type=int, default=16)
    parser.add_argument("--cost-bps", type=float, default=0.0)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    labels = triple_barrier_labels(
        _read_csv(Path(args.input_csv)),
        pt_mult=args.pt_mult,
        sl_mult=args.sl_mult,
        max_holding_bars=args.max_holding_bars,
        cost_bps=args.cost_bps,
    )
    _write_jsonl(Path(args.output_jsonl), labels)
    print(json.dumps({"ok": True, "labels": len(labels), "output": args.output_jsonl}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())