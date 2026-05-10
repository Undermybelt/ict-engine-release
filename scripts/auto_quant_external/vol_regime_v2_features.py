"""
vol_regime_v2_features.py — richer vol-regime feature pack for the regime
factor benchmark.

Replaces the rejected Slice 69 / current `vol_regime` shape (level_z20,
spread, ratio, change3, change8, trend3, gap pairs) which the trees only
partially used (vol_hv_trend3, vol_vrp_change3, vol_vix_trend3) and which
regressed against the simpler base+pda comparator on NQ 1d
post_transition_direction.

The v2 pack reshapes the same underlying IV/HV/VIX/VVIX/VIX9D inputs into
features that are more regime-state-friendly for tree splits:

- long-window percentile rank (252 bars) instead of 20-bar rolling z, so
  regime levels are persistent and tree splits land on stable boundaries
- distance from 252-bar IV high / low (regime-extreme proxies)
- categorical 5-bin VRP discretization plus a regime-state persistence
  counter (bars since last bin change)
- 8-state interaction of trend signs across IV / HV / VIX (non-monotone
  joint state captured)
- real VIX-term-structure (VIX9D / VIX) — was an ATR(5)/ATR(60) proxy in v1
- real VVIX series (z-score + 3-bar change) — was rolling-std proxy in v1
- asymmetric vol-spike boolean (VIX > rolling 60-bar max in prior 5 bars)
- long-window mean-reversion z-score (252) on IV

Inputs
------
- target candles aligned to the benchmark target series (e.g., NQ 1d).
- a paired_candle_context mapping with IV / HV / VIX / VIX9D / VVIX series
  already aligned to len(candles), NaN-padded where data is missing.

Outputs
-------
- dict[str, list[float]] with one key per feature name in
  VOL_REGIME_V2_VECTOR_FEATURES, length == len(candles), float NaN where
  inputs are missing. Categorical features are integer-encoded as floats
  so the existing ExtraTreesClassifier path can split on them.

Promotion floor (per TODO Slice 72/75): eval_family_f1 >= 0.55 on
NQ 1d post_transition_direction before extending to 4h / 1h.

Wiring (manual)
---------------
The intended integration into regime_factor_benchmark.py is:

    from vol_regime_v2_features import (
        VOL_REGIME_V2_VECTOR_FEATURES,
        vol_regime_v2_feature_vectors,
        load_ibkr_probe_series,
        align_paired_to_candles,
    )

    FEATURE_SET_ALIASES["vol_regime_v2"] = VOL_REGIME_V2_VECTOR_FEATURES

then in the dispatch block:

    if wants_vol_regime_v2_features(args.feature_set) and paired_candle_context:
        extra_vectors.update(
            vol_regime_v2_feature_vectors(candles, paired_candle_context)
        )

This module deliberately does not edit regime_factor_benchmark.py because
that file is mid-flight in the user's working tree.
"""
from __future__ import annotations

from pathlib import Path
from typing import Iterable, Sequence

import numpy as np
import pandas as pd

VOL_REGIME_V2_VECTOR_FEATURES: list[str] = [
    "v2_iv_level_pct_rank_252",
    "v2_hv_level_pct_rank_252",
    "v2_vix_level_pct_rank_252",
    "v2_vvix_level_pct_rank_252",
    "v2_iv_to_iv_252_high_distance",
    "v2_iv_to_iv_252_low_distance",
    "v2_vrp_spread",
    "v2_vrp_state_5bin",
    "v2_vrp_regime_persistence",
    "v2_trend_sign_joint_8state",
    "v2_vix_term_short_long",
    "v2_vvix_level_z20",
    "v2_vvix_change3",
    "v2_vix_spike_5b",
    "v2_iv_meanrev_252_z",
]

_DEFAULT_IBKR_PROBE_DIR = Path("/tmp/ict-engine-ibkr-probe")

_IBKR_FILE_PATTERNS: dict[str, tuple[str, str]] = {
    "iv":     ("qqq.iv.1d.10y.csv",   "close"),
    "hv":     ("qqq.hv.1d.10y.csv",   "close"),
    "qqq_iv": ("qqq.iv.1d.10y.csv",   "close"),
    "qqq_hv": ("qqq.hv.1d.10y.csv",   "close"),
    "vix":    ("vix.1d.10y.csv",      "close"),
    "vix9d":  ("vix9d.1d.10y.csv",    "close"),
    "vvix":   ("vvix.1d.10y.csv",     "close"),
    "vxn":    ("vxn.1d.10y.csv",      "close"),
    "vix3m":  ("vix3m.1d.10y.csv",    "close"),
    "ovx":    ("ovx.1d.10y.csv",      "close"),
    "rvx":    ("rvx.1d.10y.csv",      "close"),
    "gvz":    ("gvz.1d.10y.csv",      "close"),
    "spy_iv": ("spy.iv.1d.10y.csv",   "close"),
    "spy_hv": ("spy.hv.1d.10y.csv",   "close"),
    "iwm_iv": ("iwm.iv.1d.10y.csv",   "close"),
    "iwm_hv": ("iwm.hv.1d.10y.csv",   "close"),
    "dia_iv": ("dia.iv.1d.10y.csv",   "close"),
    "dia_hv": ("dia.hv.1d.10y.csv",   "close"),
    "gld_iv": ("gld.iv.1d.10y.csv",   "close"),
    "gld_hv": ("gld.hv.1d.10y.csv",   "close"),
    "ndx":    ("ndx.1d.10y.csv",      "close"),
}


def load_ibkr_probe_series(
    keys: Iterable[str],
    probe_dir: Path = _DEFAULT_IBKR_PROBE_DIR,
) -> dict[str, pd.Series]:
    """Load 1d IBKR probe CSVs into time-aligned pandas Series.

    Each CSV: ts,open,high,low,close,volume,wap,count.
    Index is normalized to UTC date (no time-of-day).
    Silently skips missing files so the caller can still build features
    on the subset that exists.
    """
    series_map: dict[str, pd.Series] = {}
    for key in keys:
        if key not in _IBKR_FILE_PATTERNS:
            continue
        filename, col = _IBKR_FILE_PATTERNS[key]
        path = probe_dir / filename
        if not path.exists():
            continue
        df = pd.read_csv(path)
        if "ts" not in df.columns or col not in df.columns:
            continue
        df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
        df = df.dropna(subset=["ts", col])
        df["date"] = df["ts"].dt.normalize()
        s = df.set_index("date")[col].astype(float)
        s = s[~s.index.duplicated(keep="last")].sort_index()
        series_map[key] = s
    return series_map


def _candle_dates(candles: Sequence[dict]) -> pd.DatetimeIndex:
    raw = []
    for c in candles:
        for k in ("timestamp", "date", "ts"):
            if k in c:
                raw.append(c[k])
                break
        else:
            raw.append(None)
    return pd.to_datetime(raw, utc=True).normalize()


def align_paired_to_candles(
    candles: Sequence[dict],
    series_map: dict[str, pd.Series],
) -> dict[str, list[float]]:
    """Forward-fill each series onto the candle-date index.

    NaN before the series' first observation; pandas-NaN converted to
    Python float('nan') so downstream JSON serialization stays clean.
    """
    if not candles:
        return {k: [] for k in series_map}
    dates = _candle_dates(candles)
    out: dict[str, list[float]] = {}
    for key, series in series_map.items():
        reindexed = series.reindex(dates).ffill()
        out[key] = [float(v) if pd.notna(v) else float("nan") for v in reindexed.values]
    return out


def _nan_list(n: int) -> list[float]:
    return [float("nan")] * n


def _to_python_nan_list(series: pd.Series) -> list[float]:
    return [float(v) if pd.notna(v) else float("nan") for v in series.values]


def vol_regime_v2_feature_vectors(
    candles: Sequence[dict],
    paired_candle_context: dict[str, list[float]],
) -> dict[str, list[float]]:
    """Build the v2 feature columns aligned to candles.

    paired_candle_context keys (all optional; missing keys produce NaN):
      iv, hv, vix, vix9d, vvix.

    Output columns are listed in VOL_REGIME_V2_VECTOR_FEATURES.
    """
    n = len(candles)
    out: dict[str, list[float]] = {col: _nan_list(n) for col in VOL_REGIME_V2_VECTOR_FEATURES}
    if n == 0:
        return out

    df = pd.DataFrame(
        {
            "iv":    paired_candle_context.get("iv",    _nan_list(n)),
            "hv":    paired_candle_context.get("hv",    _nan_list(n)),
            "vix":   paired_candle_context.get("vix",   _nan_list(n)),
            "vix9d": paired_candle_context.get("vix9d", _nan_list(n)),
            "vvix":  paired_candle_context.get("vvix",  _nan_list(n)),
        }
    )

    out["v2_iv_level_pct_rank_252"]   = _to_python_nan_list(df["iv"].rolling(252, min_periods=128).rank(pct=True))
    out["v2_hv_level_pct_rank_252"]   = _to_python_nan_list(df["hv"].rolling(252, min_periods=128).rank(pct=True))
    out["v2_vix_level_pct_rank_252"]  = _to_python_nan_list(df["vix"].rolling(252, min_periods=128).rank(pct=True))
    out["v2_vvix_level_pct_rank_252"] = _to_python_nan_list(df["vvix"].rolling(252, min_periods=128).rank(pct=True))

    iv_high = df["iv"].rolling(252, min_periods=128).max()
    iv_low = df["iv"].rolling(252, min_periods=128).min()
    iv_range = (iv_high - iv_low).clip(lower=1e-12)
    out["v2_iv_to_iv_252_high_distance"] = _to_python_nan_list((iv_high - df["iv"]) / iv_range)
    out["v2_iv_to_iv_252_low_distance"]  = _to_python_nan_list((df["iv"] - iv_low) / iv_range)

    vrp = df["iv"] - df["hv"]
    out["v2_vrp_spread"] = _to_python_nan_list(vrp)

    q20 = vrp.rolling(252, min_periods=128).quantile(0.20)
    q40 = vrp.rolling(252, min_periods=128).quantile(0.40)
    q60 = vrp.rolling(252, min_periods=128).quantile(0.60)
    q80 = vrp.rolling(252, min_periods=128).quantile(0.80)
    state = pd.Series(np.nan, index=vrp.index)
    quantile_ready = q20.notna() & q40.notna() & q60.notna() & q80.notna() & vrp.notna()
    state.loc[quantile_ready & (vrp <= q20)] = 0.0
    state.loc[quantile_ready & (vrp > q20) & (vrp <= q40)] = 1.0
    state.loc[quantile_ready & (vrp > q40) & (vrp <= q60)] = 2.0
    state.loc[quantile_ready & (vrp > q60) & (vrp <= q80)] = 3.0
    state.loc[quantile_ready & (vrp > q80)] = 4.0
    out["v2_vrp_state_5bin"] = _to_python_nan_list(state)

    state_segment_id = (state != state.shift(1)).fillna(True).cumsum()
    persistence = state.groupby(state_segment_id).cumcount().astype(float)
    persistence = persistence.where(state.notna())
    out["v2_vrp_regime_persistence"] = _to_python_nan_list(persistence)

    iv_diff3 = df["iv"].diff(3)
    hv_diff3 = df["hv"].diff(3)
    vix_diff3 = df["vix"].diff(3)
    s_iv = (iv_diff3 > 0).astype(float)
    s_hv = (hv_diff3 > 0).astype(float)
    s_vix = (vix_diff3 > 0).astype(float)
    joint_valid = iv_diff3.notna() & hv_diff3.notna() & vix_diff3.notna()
    joint = (s_iv + 2 * s_hv + 4 * s_vix).where(joint_valid)
    out["v2_trend_sign_joint_8state"] = _to_python_nan_list(joint)

    vix_safe = df["vix"].where(df["vix"].abs() > 1e-12)
    out["v2_vix_term_short_long"] = _to_python_nan_list(df["vix9d"] / vix_safe)

    vvix_mean = df["vvix"].rolling(20, min_periods=10).mean()
    vvix_std = df["vvix"].rolling(20, min_periods=10).std().where(lambda s: s > 1e-12)
    out["v2_vvix_level_z20"] = _to_python_nan_list((df["vvix"] - vvix_mean) / vvix_std)
    out["v2_vvix_change3"] = _to_python_nan_list(df["vvix"].diff(3))

    vix_max_prior = df["vix"].shift(5).rolling(60, min_periods=30).max()
    spike = (df["vix"] > vix_max_prior).astype(float).where(vix_max_prior.notna())
    out["v2_vix_spike_5b"] = _to_python_nan_list(spike)

    iv_mean = df["iv"].rolling(252, min_periods=128).mean()
    iv_std = df["iv"].rolling(252, min_periods=128).std().where(lambda s: s > 1e-12)
    out["v2_iv_meanrev_252_z"] = _to_python_nan_list((df["iv"] - iv_mean) / iv_std)

    return out


def build_vol_regime_v2_for_candles(
    candles: Sequence[dict],
    probe_dir: Path = _DEFAULT_IBKR_PROBE_DIR,
    series_keys: Iterable[str] = ("iv", "hv", "vix", "vix9d", "vvix"),
) -> dict[str, list[float]]:
    """End-to-end helper: load probe CSVs, align to candles, build v2 columns."""
    series_map = load_ibkr_probe_series(series_keys, probe_dir)
    paired = align_paired_to_candles(candles, series_map)
    return vol_regime_v2_feature_vectors(candles, paired)


__all__ = [
    "VOL_REGIME_V2_VECTOR_FEATURES",
    "vol_regime_v2_feature_vectors",
    "load_ibkr_probe_series",
    "align_paired_to_candles",
    "build_vol_regime_v2_for_candles",
]
