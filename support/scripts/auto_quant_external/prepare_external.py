"""
prepare_external.py — adapter that converts arbitrary OHLCV CSV files into
FreqTrade feather format, so the FreqTrade backtest engine can ingest stocks,
futures, CFDs, and ETFs without touching any FreqTrade source code.

Why this works without a FreqTrade fork:
  - FreqTrade's backtest path only reads `<datadir>/<PAIR>-<TF>.feather`
    with columns `[date, open, high, low, close, volume]`
  - The exchange.name in config.json is validated against ccxt at startup
    (so it must be a known ccxt name like "binance"), but no exchange API
    is called during backtest; data is purely local.
  - Pair names are arbitrary strings — they don't need to be real ccxt
    markets. We use synthetic pseudo-pairs like "NQ/USD" or "AAPL/USD".

Usage:
  uv run prepare_external.py \\
    --csv "/path/to/source.csv" \\
    --pair "NQ/USD" \\
    --timeframes 1h,4h,1d \\
    --timerange 20230101-20251231 \\
    --column-map ts_event:date,open_adj:open,high_adj:high,low_adj:low,close_adj:close,volume:volume \\
    --datadir user_data/data

Options is intentionally NOT supported: FreqTrade's single-instrument OHLCV
abstraction has no place for strike/expiry/Greeks/multi-leg positions.
For options use a real options-aware framework.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

import pandas as pd

REQUIRED_COLS = ["date", "open", "high", "low", "close", "volume"]
DEFAULT_MAX_JUMP_PCT = 5.0  # 1m close-to-close jump above this is treated as bad tick
TIMEFRAME_TO_PANDAS = {
    "1m": "1min",
    "5m": "5min",
    "15m": "15min",
    "30m": "30min",
    "1h": "1h",
    "2h": "2h",
    "4h": "4h",
    "6h": "6h",
    "12h": "12h",
    "1d": "1D",
    "1w": "1W",
}


def parse_column_map(spec: str) -> dict[str, str]:
    mapping: dict[str, str] = {}
    for chunk in spec.split(","):
        chunk = chunk.strip()
        if not chunk:
            continue
        if ":" not in chunk:
            raise ValueError(f"bad column-map entry {chunk!r}, expected src:dst")
        src, dst = chunk.split(":", 1)
        mapping[src.strip()] = dst.strip()
    return mapping


def load_source(csv_path: Path, column_map: dict[str, str]) -> tuple[pd.DataFrame, dict[str, int]]:
    df = pd.read_csv(csv_path, low_memory=False)
    raw_rows = len(df)
    rename = {src: dst for src, dst in column_map.items() if src in df.columns}
    df = df.rename(columns=rename)
    missing = [c for c in REQUIRED_COLS if c not in df.columns]
    if missing:
        raise ValueError(
            f"after applying column-map, source CSV is missing columns: {missing}. "
            f"Got columns: {list(df.columns)}"
        )
    df["date"] = pd.to_datetime(df["date"], utc=True, errors="coerce")
    bad_date = int(df["date"].isna().sum())
    df = df.dropna(subset=["date"])
    df = df.sort_values("date")
    before_dedup = len(df)
    df = df.drop_duplicates(subset=["date"], keep="first")
    duplicate_ts = before_dedup - len(df)
    for col in ("open", "high", "low", "close", "volume"):
        df[col] = pd.to_numeric(df[col], errors="coerce")
    before_nan_ohlc = len(df)
    df = df.dropna(subset=["open", "high", "low", "close"])
    nan_ohlc = before_nan_ohlc - len(df)
    df["volume"] = df["volume"].fillna(0.0)
    stats = {
        "raw_rows": raw_rows,
        "bad_date": bad_date,
        "duplicate_ts": duplicate_ts,
        "nan_ohlc": nan_ohlc,
        "after_load": len(df),
    }
    return df[REQUIRED_COLS].reset_index(drop=True), stats


def clean_bars(df: pd.DataFrame, max_jump_pct: float) -> tuple[pd.DataFrame, dict[str, int]]:
    """Five-pass cleaner; returns cleaned frame plus per-pass drop counts.

    Drop reasons (in order):
      1. ohlc_inconsistent  — high < max(open,close) or low > min(open,close)
      2. nonpositive_price  — any of open/high/low/close <= 0
      3. negative_volume    — volume < 0 (data error)
      4. ghost_bar          — volume == 0 AND |close-open| > 0 (no trades but price moved)
      5. jump_outlier       — |close[t]/close[t-1] - 1| > max_jump_pct/100 (bad tick at minute scale)
    """
    drops: dict[str, int] = {}
    work = df.copy()

    inconsistent = (work["high"] < work[["open", "close"]].max(axis=1)) | (
        work["low"] > work[["open", "close"]].min(axis=1)
    )
    drops["ohlc_inconsistent"] = int(inconsistent.sum())
    work = work.loc[~inconsistent]

    nonpositive = (work[["open", "high", "low", "close"]] <= 0).any(axis=1)
    drops["nonpositive_price"] = int(nonpositive.sum())
    work = work.loc[~nonpositive]

    negative_vol = work["volume"] < 0
    drops["negative_volume"] = int(negative_vol.sum())
    work = work.loc[~negative_vol]

    if len(work) > 0 and float(work["volume"].median()) == 0.0:
        drops["ghost_bar"] = 0
        drops["ghost_bar_skipped"] = "volume_series_all_zero_otc_instrument"
    else:
        ghost = (work["volume"] == 0) & ((work["close"] - work["open"]).abs() > 0)
        drops["ghost_bar"] = int(ghost.sum())
        work = work.loc[~ghost]

    if max_jump_pct > 0 and len(work) > 1:
        ratio = work["close"].pct_change().abs()
        outlier = ratio > (max_jump_pct / 100.0)
        drops["jump_outlier"] = int(outlier.sum())
        work = work.loc[~outlier]
    else:
        drops["jump_outlier"] = 0

    drops["after_clean"] = len(work)
    return work.reset_index(drop=True), drops


def slice_timerange(df: pd.DataFrame, timerange: str | None) -> pd.DataFrame:
    if not timerange:
        return df
    if "-" not in timerange:
        raise ValueError(f"timerange must be YYYYMMDD-YYYYMMDD, got {timerange!r}")
    start_str, end_str = timerange.split("-", 1)
    start = pd.Timestamp(start_str, tz="UTC") if start_str else None
    end = pd.Timestamp(end_str, tz="UTC") if end_str else None
    if start is not None:
        df = df[df["date"] >= start]
    if end is not None:
        df = df[df["date"] <= end]
    return df


def resample_ohlcv(df: pd.DataFrame, timeframe: str) -> pd.DataFrame:
    if timeframe not in TIMEFRAME_TO_PANDAS:
        raise ValueError(
            f"unsupported timeframe {timeframe!r}; supported: {sorted(TIMEFRAME_TO_PANDAS)}"
        )
    rule = TIMEFRAME_TO_PANDAS[timeframe]
    agg = (
        df.set_index("date")
        .resample(rule, label="left", closed="left")
        .agg(
            {
                "open": "first",
                "high": "max",
                "low": "min",
                "close": "last",
                "volume": "sum",
            }
        )
        .dropna(subset=["open", "high", "low", "close"])
        .reset_index()
    )
    return agg[REQUIRED_COLS]


def write_feather(df: pd.DataFrame, datadir: Path, pair: str, timeframe: str) -> Path:
    datadir.mkdir(parents=True, exist_ok=True)
    pair_filename = pair.replace("/", "_").replace(":", "_")
    out_path = datadir / f"{pair_filename}-{timeframe}.feather"
    out = df.copy()
    out["date"] = (out["date"].astype("int64") // 1_000_000).astype("int64")
    out.to_feather(out_path)
    return out_path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Convert arbitrary OHLCV CSV into FreqTrade feather files."
    )
    parser.add_argument("--csv", required=True, help="path to source OHLCV CSV")
    parser.add_argument("--pair", required=True, help="synthetic pair name, e.g. NQ/USD")
    parser.add_argument(
        "--timeframes",
        default="1h,4h,1d",
        help="comma-separated FreqTrade timeframes to emit (default 1h,4h,1d)",
    )
    parser.add_argument(
        "--timerange",
        default="",
        help="optional YYYYMMDD-YYYYMMDD slice (UTC); empty means full file",
    )
    parser.add_argument(
        "--datadir",
        default="user_data/data",
        help="destination dir for feather files (default user_data/data)",
    )
    parser.add_argument(
        "--column-map",
        default="ts_event:date,open:open,high:high,low:low,close:close,volume:volume",
        help=(
            "src:dst pairs separated by comma, mapping CSV columns to FreqTrade "
            "schema (date, open, high, low, close, volume)"
        ),
    )
    parser.add_argument(
        "--no-clean",
        action="store_true",
        help="skip the 5-pass cleaner (raw passthrough; only schema + dedup applied)",
    )
    parser.add_argument(
        "--max-jump-pct",
        type=float,
        default=DEFAULT_MAX_JUMP_PCT,
        help=(
            "reject 1m close-to-close jumps above this percent as bad ticks; "
            f"default {DEFAULT_MAX_JUMP_PCT}; set 0 to disable jump filter"
        ),
    )
    args = parser.parse_args()

    csv_path = Path(args.csv).expanduser().resolve()
    if not csv_path.exists():
        print(f"ERROR: csv not found: {csv_path}", file=sys.stderr)
        return 2
    column_map = parse_column_map(args.column_map)
    timeframes = [tf.strip() for tf in args.timeframes.split(",") if tf.strip()]
    datadir = Path(args.datadir).resolve()

    print(f"Loading {csv_path} ...")
    base, load_stats = load_source(csv_path, column_map)
    print(
        f"  raw_rows={load_stats['raw_rows']:,} bad_date={load_stats['bad_date']:,} "
        f"duplicate_ts={load_stats['duplicate_ts']:,} nan_ohlc={load_stats['nan_ohlc']:,} "
        f"after_load={load_stats['after_load']:,}"
    )
    if not base.empty:
        print(f"  range {base['date'].min()} -> {base['date'].max()}")
    base = slice_timerange(base, args.timerange or None)
    print(f"  after timerange slice: {len(base):,} rows")
    if base.empty:
        print("ERROR: no rows after slice; nothing to write", file=sys.stderr)
        return 3

    if args.no_clean:
        print("  cleaning: SKIPPED (--no-clean)")
    else:
        before = len(base)
        base, drops = clean_bars(base, args.max_jump_pct)
        total_dropped = before - len(base)
        pct = (total_dropped / before * 100) if before else 0.0
        print(
            f"  cleaning: dropped {total_dropped:,} ({pct:.4f}%) | "
            f"ohlc_inconsistent={drops['ohlc_inconsistent']:,} "
            f"nonpositive_price={drops['nonpositive_price']:,} "
            f"negative_volume={drops['negative_volume']:,} "
            f"ghost_bar={drops['ghost_bar']:,} "
            f"jump_outlier={drops['jump_outlier']:,} "
            f"-> after_clean={drops['after_clean']:,}"
        )
        if base.empty:
            print("ERROR: cleaner removed all rows; aborting", file=sys.stderr)
            return 4

    gaps = base["date"].diff().dt.total_seconds().dropna()
    if len(gaps) > 0:
        median_gap = gaps.median()
        max_gap_min = gaps.max() / 60.0
        big_gaps = int((gaps > median_gap * 60).sum())
        print(
            f"  gap_audit: median_gap={median_gap:.0f}s max_gap={max_gap_min:,.1f}min "
            f"big_gaps_gt_60x_median={big_gaps:,} (typical for futures session breaks/weekends)"
        )

    written = []
    for tf in timeframes:
        resampled = resample_ohlcv(base, tf)
        out_path = write_feather(resampled, datadir, args.pair, tf)
        written.append(out_path)
        print(f"  wrote {tf:>4}: {len(resampled):>7,} bars -> {out_path}")

    print(f"Done: {len(written)} feather files for pair {args.pair} in {datadir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
