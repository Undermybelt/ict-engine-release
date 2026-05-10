"""
vol_regime_cross_asset_features.py — cross-asset vol-regime feature pack
that complements vol_regime_v2_features.py.

vol_regime_v2 is a within-asset descriptor: it characterizes one asset's
IV / HV / VIX / VVIX / VIX9D regime.

This module is cross-asset: every feature requires multiple assets'
IV / HV series simultaneously, plus the multi-vol-index basket. The
intent is to expose regime-concordance, regime-disagreement, and
cross-sectional VRP information that no within-asset shape can capture.

Designed to be wired into regime_factor_benchmark.py the same way as
vol_regime_v2 — the user can land it with a 3-line patch in
FEATURE_SET_ALIASES + the dispatch block when ready to consolidate. A
combined `vol_regime_v3` alias would be:

    FEATURE_SET_ALIASES["vol_regime_v3"] = (
        VOL_REGIME_V2_VECTOR_FEATURES + VOL_REGIME_CROSS_ASSET_VECTOR_FEATURES
    )

Inputs
------
- target candles aligned to the benchmark target series (e.g., NQ 1d).
- a paired_candle_context mapping with these keys (all optional, NaN-pad
  what is not available; missing series degrades gracefully):
    Equity HV basket:    qqq_hv, spy_hv, iwm_hv, dia_hv
    Commodity HV:        gld_hv
    Equity IV basket:    qqq_iv, spy_iv, iwm_iv, dia_iv
    Commodity IV:        gld_iv
    Vol indices basket:  vix, vxn, rvx, ovx, gvz
    Term-structure:      vix9d, vix3m

Outputs
-------
- dict[str, list[float]] with one key per feature name in
  VOL_REGIME_CROSS_ASSET_VECTOR_FEATURES, length == len(candles), float
  NaN where inputs are missing.

Feature catalog
---------------
- xa_hv_pct_rank_concordance: fraction of {qqq, spy, iwm, dia} HV with
  252-bar percentile rank > 0.7 (broad-vol regime detector)
- xa_iv_pct_rank_concordance: same for IV
- xa_hv_pct_rank_disagreement: std-dev of 252-bar pct ranks across the
  4-asset equity HV basket (regime-fragmentation detector)
- xa_iv_pct_rank_disagreement: same for IV
- xa_equity_vs_gold_hv_spread: mean(equity HV pct rank) - GLD HV pct
  rank (risk-on / risk-off proxy)
- xa_equity_vs_gold_iv_spread: same for IV
- xa_basket_iv_minus_hv_spread: mean(equity IV pct rank) - mean(equity
  HV pct rank) across basket (cross-sectional VRP)
- xa_vol_index_basket_z: z-score of mean(VIX, VXN, RVX, OVX, GVZ)
  against own 252-bar history
- xa_term_curvature_3pt: (VIX9D / VIX) - (VIX / VIX3M); positive =
  contango, negative = backwardation
- xa_vix9d_vix3m_ratio: VIX9D / VIX3M extreme term-structure ratio
"""
from __future__ import annotations

from typing import Sequence

import numpy as np
import pandas as pd

VOL_REGIME_CROSS_ASSET_VECTOR_FEATURES: list[str] = [
    "xa_hv_pct_rank_concordance",
    "xa_iv_pct_rank_concordance",
    "xa_hv_pct_rank_disagreement",
    "xa_iv_pct_rank_disagreement",
    "xa_equity_vs_gold_hv_spread",
    "xa_equity_vs_gold_iv_spread",
    "xa_basket_iv_minus_hv_spread",
    "xa_vol_index_basket_z",
    "xa_term_curvature_3pt",
    "xa_vix9d_vix3m_ratio",
]

_EQUITY_HV_KEYS = ("qqq_hv", "spy_hv", "iwm_hv", "dia_hv")
_EQUITY_IV_KEYS = ("qqq_iv", "spy_iv", "iwm_iv", "dia_iv")
_VOL_INDEX_KEYS = ("vix", "vxn", "rvx", "ovx", "gvz")


def _nan_list(n: int) -> list[float]:
    return [float("nan")] * n


def _to_python_nan_list(series: pd.Series) -> list[float]:
    return [float(v) if pd.notna(v) else float("nan") for v in series.values]


def _pct_rank_252(series: pd.Series) -> pd.Series:
    return series.rolling(252, min_periods=128).rank(pct=True)


def _basket_dataframe(
    paired_candle_context: dict[str, list[float]],
    keys: tuple[str, ...],
    n: int,
) -> pd.DataFrame:
    cols = {}
    for key in keys:
        cols[key] = paired_candle_context.get(key, _nan_list(n))
    return pd.DataFrame(cols)


def vol_regime_cross_asset_feature_vectors(
    candles: Sequence[dict],
    paired_candle_context: dict[str, list[float]],
) -> dict[str, list[float]]:
    """Build cross-asset regime feature columns aligned to candles.

    See module docstring for required paired_candle_context keys; each
    missing series degrades to NaN rather than crashing the build.
    """
    n = len(candles)
    out: dict[str, list[float]] = {col: _nan_list(n) for col in VOL_REGIME_CROSS_ASSET_VECTOR_FEATURES}
    if n == 0:
        return out

    hv_basket = _basket_dataframe(paired_candle_context, _EQUITY_HV_KEYS, n)
    iv_basket = _basket_dataframe(paired_candle_context, _EQUITY_IV_KEYS, n)
    gld_hv = pd.Series(paired_candle_context.get("gld_hv", _nan_list(n)))
    gld_iv = pd.Series(paired_candle_context.get("gld_iv", _nan_list(n)))
    vol_idx_basket = _basket_dataframe(paired_candle_context, _VOL_INDEX_KEYS, n)
    vix = pd.Series(paired_candle_context.get("vix", _nan_list(n)))
    vix9d = pd.Series(paired_candle_context.get("vix9d", _nan_list(n)))
    vix3m = pd.Series(paired_candle_context.get("vix3m", _nan_list(n)))

    hv_pr = hv_basket.apply(_pct_rank_252, axis=0)
    iv_pr = iv_basket.apply(_pct_rank_252, axis=0)
    gld_hv_pr = _pct_rank_252(gld_hv)
    gld_iv_pr = _pct_rank_252(gld_iv)

    out["xa_hv_pct_rank_concordance"] = _to_python_nan_list((hv_pr > 0.7).sum(axis=1) / hv_pr.notna().sum(axis=1).replace(0, np.nan))
    out["xa_iv_pct_rank_concordance"] = _to_python_nan_list((iv_pr > 0.7).sum(axis=1) / iv_pr.notna().sum(axis=1).replace(0, np.nan))

    out["xa_hv_pct_rank_disagreement"] = _to_python_nan_list(hv_pr.std(axis=1, skipna=True))
    out["xa_iv_pct_rank_disagreement"] = _to_python_nan_list(iv_pr.std(axis=1, skipna=True))

    equity_hv_mean = hv_pr.mean(axis=1, skipna=True)
    equity_iv_mean = iv_pr.mean(axis=1, skipna=True)
    out["xa_equity_vs_gold_hv_spread"] = _to_python_nan_list(equity_hv_mean - gld_hv_pr)
    out["xa_equity_vs_gold_iv_spread"] = _to_python_nan_list(equity_iv_mean - gld_iv_pr)
    out["xa_basket_iv_minus_hv_spread"] = _to_python_nan_list(equity_iv_mean - equity_hv_mean)

    vol_idx_mean = vol_idx_basket.mean(axis=1, skipna=True)
    vol_idx_mean_252_mean = vol_idx_mean.rolling(252, min_periods=128).mean()
    vol_idx_mean_252_std = vol_idx_mean.rolling(252, min_periods=128).std().where(lambda s: s > 1e-12)
    out["xa_vol_index_basket_z"] = _to_python_nan_list(
        (vol_idx_mean - vol_idx_mean_252_mean) / vol_idx_mean_252_std
    )

    vix_safe = vix.where(vix.abs() > 1e-12)
    vix3m_safe = vix3m.where(vix3m.abs() > 1e-12)
    short_long = vix9d / vix_safe
    long_far = vix / vix3m_safe
    out["xa_term_curvature_3pt"] = _to_python_nan_list(short_long - long_far)
    out["xa_vix9d_vix3m_ratio"] = _to_python_nan_list(vix9d / vix3m_safe)

    return out


def build_vol_regime_cross_asset_for_candles(
    candles: Sequence[dict],
    probe_dir=None,
) -> dict[str, list[float]]:
    """End-to-end helper: load probe CSVs and build cross-asset features.

    Reuses load_ibkr_probe_series + align_paired_to_candles from the
    sibling vol_regime_v2_features module so the file-pattern registry
    stays single-sourced.
    """
    from vol_regime_v2_features import (
        align_paired_to_candles,
        load_ibkr_probe_series,
        _DEFAULT_IBKR_PROBE_DIR,
    )

    series_keys = list(_EQUITY_HV_KEYS) + list(_EQUITY_IV_KEYS) + list(_VOL_INDEX_KEYS) + [
        "gld_hv",
        "gld_iv",
        "vix9d",
        "vix3m",
    ]
    series_map = load_ibkr_probe_series(series_keys, probe_dir or _DEFAULT_IBKR_PROBE_DIR)
    paired = align_paired_to_candles(candles, series_map)
    return vol_regime_cross_asset_feature_vectors(candles, paired)


__all__ = [
    "VOL_REGIME_CROSS_ASSET_VECTOR_FEATURES",
    "vol_regime_cross_asset_feature_vectors",
    "build_vol_regime_cross_asset_for_candles",
]
