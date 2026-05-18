#!/usr/bin/env python3
"""
external_regime_changepoint_labels.py

Offline helper for generating independent regime-transition labels from candle
data. This lives outside the ict-engine runtime boundary so the factor-iteration
loop can evaluate change-point agreement without changing the Rust execution
path.
"""
from __future__ import annotations

import argparse
import json
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd


FAMILY_TREND = "trend"
FAMILY_RANGE = "range"
FAMILY_TRANSITION = "transition"
FAMILY_UNKNOWN = "unknown"


@dataclass(frozen=True)
class SegmentSummary:
    segment_id: int
    start: str
    end: str
    bars: int
    family: str
    mean_return: float
    realized_vol: float
    trend_efficiency: float
    max_drawdown: float


def load_candles(path: str | Path) -> pd.DataFrame:
    raw_path = Path(path)
    suffix = raw_path.suffix.lower()
    if suffix == ".feather":
        try:
            df = pd.read_feather(raw_path)
        except ImportError as exc:
            raise ImportError(
                "reading feather candles requires pyarrow; run with `uv run --with pyarrow ...`"
            ) from exc
    elif suffix == ".csv":
        df = pd.read_csv(raw_path)
    elif suffix == ".json":
        payload = json.loads(raw_path.read_text())
        if isinstance(payload, dict):
            for key in ("candles", "data", "rows"):
                if key in payload and isinstance(payload[key], list):
                    payload = payload[key]
                    break
        df = pd.DataFrame(payload)
    else:
        raise ValueError(f"unsupported candle format for {raw_path}")
    date_col = next((column for column in ("date", "timestamp", "ts") if column in df.columns), "ts")
    if pd.api.types.is_numeric_dtype(df[date_col]):
        df[date_col] = pd.to_datetime(df[date_col], unit="ms", utc=True, errors="coerce")
    else:
        df[date_col] = pd.to_datetime(df[date_col], utc=True, errors="coerce")
    df = df.rename(columns={date_col: "date"})
    required = {"date", "open", "high", "low", "close"}
    missing = required - set(df.columns)
    if missing:
        raise ValueError(f"missing required candle columns: {sorted(missing)}")
    if "volume" not in df.columns:
        df["volume"] = 0.0
    out = df[list(required | {"volume"})].dropna(subset=["date"]).copy()
    out = out.sort_values("date").drop_duplicates(subset=["date"], keep="last")
    out["volume"] = out["volume"].fillna(0.0)
    return out.set_index("date")


def compute_feature_matrix(candles: pd.DataFrame) -> pd.DataFrame:
    close = candles["close"].astype(float)
    high = candles["high"].astype(float)
    low = candles["low"].astype(float)
    volume = candles["volume"].astype(float)
    log_ret = np.log(close.replace(0, np.nan)).diff().fillna(0.0)
    true_range = pd.concat(
        [(high - low), (high - close.shift(1)).abs(), (low - close.shift(1)).abs()],
        axis=1,
    ).max(axis=1)
    atr = true_range.ewm(alpha=1 / 14, adjust=False).mean()
    range_norm = ((high - low) / close.replace(0, np.nan)).replace([np.inf, -np.inf], np.nan).fillna(0.0)
    realized_vol = log_ret.rolling(12, min_periods=6).std().fillna(0.0)
    trend_efficiency = (
        close.diff(20).abs()
        / close.diff().abs().rolling(20, min_periods=8).sum().replace(0, np.nan)
    ).replace([np.inf, -np.inf], np.nan).fillna(0.0)
    rolling_peak = close.rolling(60, min_periods=10).max()
    drawdown = (close / rolling_peak - 1.0).replace([np.inf, -np.inf], np.nan).fillna(0.0)
    volume_z = (
        (volume - volume.rolling(20, min_periods=10).mean())
        / volume.rolling(20, min_periods=10).std().replace(0, np.nan)
    ).replace([np.inf, -np.inf], np.nan).fillna(0.0)
    atr_ratio = (atr / close.replace(0, np.nan)).replace([np.inf, -np.inf], np.nan).fillna(0.0)
    return pd.DataFrame(
        {
            "log_ret": log_ret,
            "realized_vol": realized_vol,
            "atr_norm_range": range_norm,
            "trend_efficiency": trend_efficiency,
            "drawdown": drawdown,
            "volume_z": volume_z,
            "atr_ratio": atr_ratio,
        },
        index=candles.index,
    ).fillna(0.0)


def _require_ruptures():
    try:
        import ruptures as rpt  # type: ignore
    except ImportError as exc:
        raise ImportError(
            "ruptures is required for change-point detection; run with `uv run --with ruptures ...`"
        ) from exc
    return rpt


def detect_breakpoints(
    feature_matrix: pd.DataFrame,
    algorithms: Iterable[str],
    penalty: float = 12.0,
    min_size: int = 24,
    window_width: int = 24,
    max_breaks: int = 12,
) -> dict[str, list[int]]:
    rpt = _require_ruptures()
    signal = feature_matrix.to_numpy(dtype=float)
    results: dict[str, list[int]] = {}
    for algo in algorithms:
        name = algo.lower()
        if name == "pelt":
            detector = rpt.Pelt(model="rbf", min_size=min_size).fit(signal)
            raw = detector.predict(pen=penalty)
        elif name == "binseg":
            detector = rpt.Binseg(model="rbf", min_size=min_size).fit(signal)
            raw = detector.predict(n_bkps=max_breaks)
        elif name == "window":
            detector = rpt.Window(width=max(4, window_width), model="l2").fit(signal)
            raw = detector.predict(n_bkps=max_breaks)
        elif name == "kernelcpd":
            detector = rpt.KernelCPD(kernel="rbf", min_size=min_size).fit(signal)
            raw = detector.predict(n_bkps=max_breaks)
        else:
            raise ValueError(f"unsupported algorithm '{algo}'")
        cleaned = sorted({bp for bp in raw if 0 < bp < len(feature_matrix)})
        results[name] = cleaned
    return results


def cluster_breakpoints(
    breakpoints_by_algo: dict[str, Iterable[int]],
    tolerance: int = 3,
) -> list[dict[str, Any]]:
    entries: list[tuple[int, str]] = []
    for algo, breakpoints in breakpoints_by_algo.items():
        for bp in breakpoints:
            entries.append((int(bp), algo))
    entries.sort(key=lambda item: item[0])
    clusters: list[dict[str, Any]] = []
    for breakpoint, algo in entries:
        if not clusters or breakpoint - clusters[-1]["max_bar_index"] > tolerance:
            clusters.append(
                {
                    "bar_index": breakpoint,
                    "min_bar_index": breakpoint,
                    "max_bar_index": breakpoint,
                    "vote_count": 1,
                    "algorithms": [algo],
                    "members": [breakpoint],
                }
            )
            continue
        cluster = clusters[-1]
        cluster["members"].append(breakpoint)
        cluster["algorithms"].append(algo)
        cluster["vote_count"] += 1
        cluster["max_bar_index"] = breakpoint
        cluster["bar_index"] = int(round(sum(cluster["members"]) / len(cluster["members"])))
    for cluster in clusters:
        cluster["algorithms"] = sorted(set(cluster["algorithms"]))
    return clusters


def build_transition_proximity(
    index: pd.Index,
    breakpoints: Iterable[int],
    window: int = 6,
) -> pd.Series:
    proximity = np.zeros(len(index), dtype=float)
    for breakpoint in breakpoints:
        for pos in range(max(0, breakpoint - window), min(len(index), breakpoint + window + 1)):
            distance = abs(pos - breakpoint)
            score = max(0.0, 1.0 - distance / max(window, 1))
            proximity[pos] = max(proximity[pos], score)
    return pd.Series(proximity, index=index)


def infer_segment_family(segment: pd.DataFrame) -> str:
    if segment.empty:
        return FAMILY_UNKNOWN
    close = segment["close"].astype(float)
    log_ret = np.log(close.replace(0, np.nan)).diff().fillna(0.0)
    total_return = close.iloc[-1] / close.iloc[0] - 1.0 if close.iloc[0] else 0.0
    realized_vol = float(log_ret.std())
    denom = close.diff().abs().sum()
    trend_efficiency = float(abs(close.iloc[-1] - close.iloc[0]) / denom) if denom else 0.0
    running_peak = close.cummax()
    max_drawdown = float((close / running_peak - 1.0).min())
    if trend_efficiency >= 0.55 and abs(total_return) >= 0.015:
        return FAMILY_TREND
    if realized_vol <= 0.006 and abs(total_return) <= 0.01:
        return FAMILY_RANGE
    if max_drawdown <= -0.02 or realized_vol >= 0.012:
        return FAMILY_TRANSITION
    return FAMILY_UNKNOWN


def build_segment_rows(
    candles: pd.DataFrame,
    clustered_breakpoints: list[dict[str, Any]],
    transition_window: int,
) -> tuple[pd.DataFrame, list[SegmentSummary]]:
    breakpoints = [item["bar_index"] for item in clustered_breakpoints]
    proximity = build_transition_proximity(candles.index, breakpoints, window=transition_window)
    vote_map = {item["bar_index"]: item["vote_count"] for item in clustered_breakpoints}
    boundaries = [0] + breakpoints + [len(candles)]
    rows: list[pd.DataFrame] = []
    summaries: list[SegmentSummary] = []
    for segment_id, (start, end) in enumerate(zip(boundaries, boundaries[1:], strict=False)):
        segment = candles.iloc[start:end].copy()
        if segment.empty:
            continue
        family = infer_segment_family(segment)
        close = segment["close"].astype(float)
        log_ret = np.log(close.replace(0, np.nan)).diff().fillna(0.0)
        running_peak = close.cummax()
        max_drawdown = float((close / running_peak - 1.0).min())
        denom = close.diff().abs().sum()
        trend_efficiency = float(abs(close.iloc[-1] - close.iloc[0]) / denom) if denom else 0.0
        mean_return = float(close.iloc[-1] / close.iloc[0] - 1.0) if close.iloc[0] else 0.0
        realized_vol = float(log_ret.std())
        segment["segment_id"] = segment_id
        segment["segment_family"] = family
        segment["changepoint_transition_proximity"] = proximity.loc[segment.index]
        segment["changepoint_transition_flag"] = segment["changepoint_transition_proximity"] > 0.0
        segment["changepoint_vote_count"] = 0
        if end < len(candles):
            segment.iloc[-1, segment.columns.get_loc("changepoint_vote_count")] = vote_map.get(end, 0)
        rows.append(segment)
        summaries.append(
            SegmentSummary(
                segment_id=segment_id,
                start=segment.index[0].isoformat(),
                end=segment.index[-1].isoformat(),
                bars=len(segment),
                family=family,
                mean_return=mean_return,
                realized_vol=realized_vol,
                trend_efficiency=trend_efficiency,
                max_drawdown=max_drawdown,
            )
        )
    combined = pd.concat(rows) if rows else candles.copy()
    return combined, summaries


def build_output_payload(
    candles: pd.DataFrame,
    clustered_breakpoints: list[dict[str, Any]],
    segment_rows: pd.DataFrame,
    segment_summaries: list[SegmentSummary],
    args: argparse.Namespace,
) -> dict[str, Any]:
    output_rows = (
        segment_rows.reset_index()
        .rename(columns={"index": "date"})
        [["date", "segment_id", "segment_family", "changepoint_transition_flag", "changepoint_transition_proximity"]]
        .copy()
    )
    output_rows["date"] = output_rows["date"].astype(str)
    return {
        "metadata": {
            "input": args.input,
            "algorithms": args.algorithms,
            "penalty": args.penalty,
            "min_size": args.min_size,
            "window_width": args.window_width,
            "transition_window": args.transition_window,
            "bars": len(candles),
            "segments": len(segment_summaries),
        },
        "breakpoints": clustered_breakpoints,
        "segment_summaries": [summary.__dict__ for summary in segment_summaries],
        "rows": output_rows.to_dict(orient="records"),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Offline regime change-point labels")
    parser.add_argument("--input", required=True, help="Path to candle json/csv/feather")
    parser.add_argument("--output", help="Optional path to write JSON payload")
    parser.add_argument(
        "--algorithms",
        nargs="+",
        default=["pelt", "binseg", "window"],
        help="ruptures algorithms to run",
    )
    parser.add_argument("--penalty", type=float, default=12.0)
    parser.add_argument("--min-size", dest="min_size", type=int, default=24)
    parser.add_argument("--window-width", dest="window_width", type=int, default=24)
    parser.add_argument("--max-breaks", dest="max_breaks", type=int, default=12)
    parser.add_argument("--cluster-tolerance", type=int, default=3)
    parser.add_argument("--transition-window", type=int, default=6)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    candles = load_candles(args.input)
    features = compute_feature_matrix(candles)
    breakpoints = detect_breakpoints(
        features,
        algorithms=args.algorithms,
        penalty=args.penalty,
        min_size=args.min_size,
        window_width=args.window_width,
        max_breaks=args.max_breaks,
    )
    clustered = cluster_breakpoints(breakpoints, tolerance=args.cluster_tolerance)
    segment_rows, segment_summaries = build_segment_rows(
        candles,
        clustered_breakpoints=clustered,
        transition_window=args.transition_window,
    )
    payload = build_output_payload(candles, clustered, segment_rows, segment_summaries, args)
    rendered = json.dumps(payload, indent=2)
    if args.output:
        Path(args.output).write_text(rendered + "\n")
    else:
        print(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
