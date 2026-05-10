from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from statistics import mean, pstdev
from typing import Any


def _load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _realized_r(row: dict[str, Any]) -> float:
    if "realized_R" in row:
        return float(row["realized_R"])
    return float(row.get("gross_R", 0.0)) - float(row.get("cost_R", 0.0))


def _sharpe(values: list[float]) -> float:
    if len(values) < 2:
        return 0.0
    sigma = pstdev(values)
    if sigma == 0.0:
        return 0.0
    return mean(values) / sigma * math.sqrt(len(values))


def _overlaps(left: dict[str, Any], right: dict[str, Any], embargo_bars: int) -> bool:
    left_start = int(left.get("entry_index", 0))
    left_end = int(left.get("exit_index", left_start)) + embargo_bars
    right_start = int(right.get("entry_index", 0))
    right_end = int(right.get("exit_index", right_start)) + embargo_bars
    return left_start <= right_end and right_start <= left_end


def _has_overlapping_labels(labels: list[dict[str, Any]], embargo_bars: int) -> bool:
    ordered = sorted(labels, key=lambda row: int(row.get("entry_index", 0)))
    for index, current in enumerate(ordered):
        for other in ordered[index + 1:]:
            if int(other.get("entry_index", 0)) > int(current.get("exit_index", current.get("entry_index", 0))) + embargo_bars:
                break
            if _overlaps(current, other, embargo_bars):
                return True
    return False


def _folds(labels: list[dict[str, Any]], fold_count: int) -> list[list[dict[str, Any]]]:
    ordered = sorted(labels, key=lambda row: int(row.get("entry_index", 0)))
    if fold_count <= 0:
        return []
    out: list[list[dict[str, Any]]] = []
    for fold_index in range(fold_count):
        start = round(len(ordered) * fold_index / fold_count)
        end = round(len(ordered) * (fold_index + 1) / fold_count)
        fold = ordered[start:end]
        if fold:
            out.append(fold)
    return out


def _purged_train_rows(
    labels: list[dict[str, Any]],
    test_fold: list[dict[str, Any]],
    embargo_bars: int,
) -> list[dict[str, Any]]:
    return [
        row
        for row in labels
        if row not in test_fold and not any(_overlaps(row, test_row, embargo_bars) for test_row in test_fold)
    ]


def _percentile(values: list[float], ratio: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, int(math.floor((len(ordered) - 1) * ratio))))
    return ordered[index]


def build_guard_report(
    *,
    labels: list[dict[str, Any]],
    nb_trials: int,
    embargo_bars: int = 1,
    fold_count: int = 4,
) -> dict[str, Any]:
    values = [_realized_r(label) for label in labels]
    folds = _folds(labels, fold_count)
    leakage_flags: list[str] = []
    if _has_overlapping_labels(labels, embargo_bars):
        leakage_flags.append("overlapping_labels")
    if len(folds) < 2 or any(not _purged_train_rows(labels, fold, embargo_bars) for fold in folds):
        leakage_flags.append("insufficient_purged_folds")

    fold_rows: list[dict[str, Any]] = []
    oos_sharpes: list[float] = []
    underperform_count = 0
    for fold_index, fold in enumerate(folds):
        train = _purged_train_rows(labels, fold, embargo_bars)
        train_values = [_realized_r(row) for row in train]
        test_values = [_realized_r(row) for row in fold]
        is_sharpe = _sharpe(train_values)
        oos_sharpe = _sharpe(test_values)
        oos_sharpes.append(oos_sharpe)
        if oos_sharpe < is_sharpe:
            underperform_count += 1
        fold_rows.append(
            {
                "fold_index": fold_index,
                "train_count": len(train),
                "test_count": len(fold),
                "is_sharpe": is_sharpe,
                "oos_sharpe": oos_sharpe,
                "oos_return_R": sum(test_values),
            }
        )

    pbo = underperform_count / len(fold_rows) if fold_rows else 1.0
    oos_lcb = _percentile(oos_sharpes, 0.05) if oos_sharpes else 0.0
    if "insufficient_purged_folds" in leakage_flags:
        gate = "insufficient_data"
    elif pbo <= 0.25 and oos_lcb > 0.0:
        gate = "pass"
    elif pbo <= 0.5 and sum(values) > 0.0:
        gate = "probe"
    else:
        gate = "reject"

    return {
        "schema_version": "purged-cv-backtest-guard/v1",
        "label_count": len(labels),
        "fold_count": len(folds),
        "requested_fold_count": fold_count,
        "embargo_bars": embargo_bars,
        "nb_trials": max(1, int(nb_trials)),
        "pbo": pbo,
        "oos_sharpe_lcb": oos_lcb,
        "oos_sharpe_mean": mean(oos_sharpes) if oos_sharpes else 0.0,
        "leakage_flags": leakage_flags,
        "purged_cv_gate": gate,
        "folds": fold_rows,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build purged CV / embargo / PBO guard report from payoff labels.")
    parser.add_argument("--labels-jsonl", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--nb-trials", type=int, default=1)
    parser.add_argument("--embargo-bars", type=int, default=1)
    parser.add_argument("--fold-count", type=int, default=4)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_guard_report(
        labels=_load_jsonl(Path(args.labels_jsonl)),
        nb_trials=args.nb_trials,
        embargo_bars=args.embargo_bars,
        fold_count=args.fold_count,
    )
    _write_json(Path(args.output_json), report)
    print(json.dumps({"ok": True, "output": args.output_json, "purged_cv_gate": report["purged_cv_gate"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
