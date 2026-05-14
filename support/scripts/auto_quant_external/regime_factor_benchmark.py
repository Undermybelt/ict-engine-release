#!/usr/bin/env python3
"""
Score non-trading regime factors against explicit regime labels.

This helper is intentionally outside the ict-engine runtime path. It lets the
factor-iteration loop evaluate regime / clustering descriptors even when they
do not produce trade entries.
"""
from __future__ import annotations

import argparse
import bisect
import json
import math
import random
from collections import Counter
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable, Iterable


LABELS = [
    "expansion",
    "manipulation",
    "reversion",
    "compression",
    "trend_continuation",
    "unknown",
]
REGIME_FAMILIES = ["trend", "range", "transition", "unknown"]
BASE_VECTOR_FEATURES = {
    "range_atr",
    "body_frac",
    "atr_pct",
    "bb_width",
    "bb_width_inv",
    "ema_gap",
    "abs_ema_gap",
    "rsi",
    "rsi_distance",
    "close_ema21_atr",
    "close_ema89_atr",
    "ema21_slope_atr",
    "mean_reclaim",
    "sweep_reject",
    "breakout_atr",
    "range_vs_mean20",
    "range_change5",
    "body_signed_atr",
    "close_pos_range",
    "upper_wick_frac",
    "lower_wick_frac",
    "prior_efficiency20",
    "chop20",
    "mean_distance_atr",
    "mean_reversion_pressure",
    "bb_width_ratio",
    "atr_pct_ratio",
    "ema_slope_change",
}
VOLUME_VECTOR_FEATURES = {
    "volume_z20",
    "volume_z50",
    "rel_volume20",
    "rel_volume50",
    "volume_trend",
    "obv_slope10",
}
INDICATOR_VECTOR_FEATURES = {
    "bb_pctb",
    "bb_pctb_extreme",
    "bb_width",
    "bb_width_inv",
    "bb_width_ratio",
    "bb_width_change5",
    "donchian_width_atr",
    "donchian_breakout_atr",
    "keltner_pos",
    "macd_hist_atr",
    "macd_hist_slope",
    "stoch_k",
    "stoch_d",
    "cci_scaled",
    "adx",
    "adx_slope",
    "rsi",
    "rsi_distance",
}
PDA_VECTOR_FEATURES = {
    "sweep_reject",
    "bull_fvg_gap_atr",
    "bear_fvg_gap_atr",
    "fvg_abs_atr",
    "order_block_touch_score",
    "order_block_age_score",
    "breaker_score",
    "sweep_displacement_score",
    "premium_discount_50",
    "premium_discount_edge",
    "engulfing_score",
    "pin_rejection_score",
    "propulsion_score",
}
PDA_DEEP_VECTOR_FEATURES = PDA_VECTOR_FEATURES | {
    "breaker_continuation_score",
    "sweep_reversal_score",
    "sweep_continuation_score",
    "fvg_mitigation_score",
    "fvg_failed_mitigation_score",
    "ob_post_mitigation_score",
}
PDA_SEQUENCE_VECTOR_FEATURES = {
    "seq_sweep_age8",
    "seq_fvg_age8",
    "seq_ob_age10",
    "seq_breaker_age8",
    "seq_sweep_then_fvg6",
    "seq_sweep_then_propulsion6",
    "seq_sweep_then_ob10",
    "seq_sweep_to_reversion6",
    "seq_sweep_to_efficiency6",
    "seq_fvg_then_mitigation8",
    "seq_fvg_then_failed_mitigation8",
    "seq_ob_then_breaker10",
    "seq_ob_then_propulsion10",
    "seq_breaker_then_continuation8",
    "seq_pda_recent_stack",
    "seq_pda_order_score",
}
POST_STATE_VECTOR_FEATURES = {
    "post_ret3_atr",
    "post_ret8_atr",
    "post_ret20_atr",
    "post_ret8_efficiency",
    "post_ret20_efficiency",
    "post_reversal_pressure",
    "post_absorption_pressure",
    "post_breakout_persistence",
    "post_sweep_reversal_bias",
    "post_sweep_continuation_bias",
    "post_trend_exhaustion",
    "post_range_absorb_chop",
    "post_direction_conflict",
}
CLUSTER_PROTO_VECTOR_FEATURES = {
    "cluster_proto_trend_prob",
    "cluster_proto_range_prob",
    "cluster_proto_transition_prob",
    "cluster_proto_margin",
    "cluster_proto_entropy",
    "cluster_proto_known",
    "cluster_proto_age20",
}
VOL_REGIME_VECTOR_FEATURES = {
    "vol_iv_level_z20",
    "vol_hv_level_z20",
    "vol_vix_level_z20",
    "vol_vrp_spread",
    "vol_vrp_ratio",
    "vol_vrp_spread_z20",
    "vol_vrp_change3",
    "vol_vrp_change8",
    "vol_vix_hv_gap",
    "vol_vix_iv_gap",
    "vol_iv_trend3",
    "vol_hv_trend3",
    "vol_vix_trend3",
}
HAZARD_VECTOR_FEATURES = {
    "hazard_range_shift_8_32",
    "hazard_body_shift_8_32",
    "hazard_chop_shift_8_32",
    "hazard_volume_shift_8_32",
    "hazard_sweep_shift_8_32",
    "hazard_slope_flip_5_20",
    "hazard_breakout_pressure",
    "hazard_compression_release",
    "hazard_regime_tension",
    "hazard_direction_instability",
}
BOCPD_LITE_VECTOR_FEATURES = {
    "bocpd_range_surprise",
    "bocpd_body_surprise",
    "bocpd_chop_surprise",
    "bocpd_volume_surprise",
    "bocpd_joint_surprise",
    "bocpd_joint_surprise_ema3",
    "bocpd_joint_surprise_ema8",
    "bocpd_hazard_prob",
    "bocpd_run_decay20",
    "bocpd_surprise_dispersion",
}
MS_REGIME_VECTOR_FEATURES = {
    "ms_regime_trend_prob",
    "ms_regime_range_prob",
    "ms_regime_transition_prob",
    "ms_regime_margin",
    "ms_regime_entropy",
    "ms_regime_known",
}
CLUSTER_VECTOR_FEATURES = (
    {f"wf_hmm_label_{label}" for label in LABELS}
    | {f"wf_hmm_family_{family}" for family in REGIME_FAMILIES}
    | {
        "wf_hmm_known",
        "wf_hmm_transition",
        "wf_hmm_segment_age20",
    }
)
CLUSTER_BRIDGE_BASE_FEATURES = [
    "range_atr",
    "atr_pct_ratio",
    "bb_width_ratio",
    "prior_efficiency20",
    "mean_reversion_pressure",
    "sweep_displacement_score",
    "premium_discount_edge",
    "propulsion_score",
]
CLUSTER_BRIDGE_VECTOR_FEATURES = (
    {
        f"wf_bridge_{family}_{feature}"
        for family in REGIME_FAMILIES
        for feature in CLUSTER_BRIDGE_BASE_FEATURES
    }
    | {
        "wf_bridge_transition_sweep",
        "wf_bridge_transition_propulsion",
        "wf_bridge_age_reversion",
        "wf_bridge_age_efficiency",
        "wf_bridge_known_range",
    }
)
FEATURE_SET_ALIASES = {
    "base": BASE_VECTOR_FEATURES,
    "volume": VOLUME_VECTOR_FEATURES,
    "indicator": INDICATOR_VECTOR_FEATURES,
    "pda": PDA_VECTOR_FEATURES,
    "pda_deep": PDA_DEEP_VECTOR_FEATURES,
    "pda_sequence": PDA_SEQUENCE_VECTOR_FEATURES,
    "post_state": POST_STATE_VECTOR_FEATURES,
    "cluster": CLUSTER_VECTOR_FEATURES,
    "cluster_static": CLUSTER_VECTOR_FEATURES,
    "cluster_kmeans": CLUSTER_VECTOR_FEATURES,
    "cluster_proto": CLUSTER_PROTO_VECTOR_FEATURES,
    "vol_regime": VOL_REGIME_VECTOR_FEATURES,
    "hazard": HAZARD_VECTOR_FEATURES,
    "bocpd_lite": BOCPD_LITE_VECTOR_FEATURES,
    "ms_regime": MS_REGIME_VECTOR_FEATURES,
    "cluster_bridge": CLUSTER_BRIDGE_VECTOR_FEATURES,
}
HMM_VECTOR_FEATURES = [
    "range_atr",
    "bb_width_ratio",
    "adx",
    "chop20",
    "mean_reversion_pressure",
    "sweep_displacement_score",
    "fvg_abs_atr",
    "volume_z50",
    "ema21_slope_atr",
    "prior_efficiency20",
    "bb_pctb_extreme",
    "premium_discount_edge",
]


@dataclass(frozen=True)
class Candle:
    timestamp: datetime
    open: float
    high: float
    low: float
    close: float
    volume: float


@dataclass
class FactorPrediction:
    label: str
    score: float


@dataclass(frozen=True)
class TrainedStump:
    label: str
    feature: str
    threshold: float
    direction: str
    f1: float
    precision: float
    recall: float


@dataclass(frozen=True)
class GaussianLabelModel:
    label: str
    prior: float
    means: dict[str, float]
    variances: dict[str, float]


@dataclass(frozen=True)
class HMMClusterModel:
    means: list[list[float]]
    variances: list[list[float]]
    transition: list[list[float]]
    start: list[float]
    state_labels: dict[int, str]


@dataclass(frozen=True)
class ExtraTreeNode:
    label: str
    confidence: float
    feature: str | None = None
    threshold: float = 0.0
    default_left: bool = True
    left: "ExtraTreeNode | None" = None
    right: "ExtraTreeNode | None" = None


def parse_timestamp(value: str) -> datetime:
    value = value.replace("Z", "+00:00")
    parsed = datetime.fromisoformat(value)
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def load_candles(path: Path) -> list[Candle]:
    raw = json.loads(path.read_text())
    rows = raw.get("candles", raw if isinstance(raw, list) else [])
    candles = [
        Candle(
            timestamp=parse_timestamp(row["timestamp"]),
            open=float(row["open"]),
            high=float(row["high"]),
            low=float(row["low"]),
            close=float(row["close"]),
            volume=float(row.get("volume", 0.0)),
        )
        for row in rows
    ]
    candles.sort(key=lambda item: item.timestamp)
    return candles


def manual_mece_labels(candles: list[Candle], lookback_len: int = 10) -> list[str]:
    out: list[str] = []
    for idx, candle in enumerate(candles):
        if idx < lookback_len:
            out.append("unknown")
            continue
        window = candles[idx - lookback_len : idx]
        avg_range = sum(max(0.0, c.high - c.low) for c in window) / lookback_len
        if not math.isfinite(avg_range) or avg_range <= 0.0:
            out.append("unknown")
            continue
        prev_max_high = max(c.high for c in window)
        prev_min_low = min(c.low for c in window)
        bar_range = max(0.0, candle.high - candle.low)
        body = abs(candle.close - candle.open)
        prev = candles[idx - 1]
        prev_dir = sign(prev.close - prev.open)
        curr_dir = sign(candle.close - candle.open)

        if bar_range > 0.0:
            swept_high = candle.high > prev_max_high and candle.close < (
                candle.high - 0.6 * bar_range
            )
            swept_low = candle.low < prev_min_low and candle.close > (
                candle.low + 0.6 * bar_range
            )
            if swept_high or swept_low:
                out.append("manipulation")
                continue

        if bar_range < 0.5 * avg_range:
            out.append("compression")
            continue

        if bar_range > 1.5 * avg_range and body > 0.6 * bar_range:
            out.append("expansion")
            continue

        if curr_dir != 0.0 and curr_dir == prev_dir and body > 0.5 * bar_range:
            out.append("trend_continuation")
            continue

        mean_close = sum(c.close for c in window) / lookback_len
        if (
            curr_dir != 0.0
            and curr_dir != prev_dir
            and abs(candle.close - mean_close) < abs(candle.open - mean_close)
        ):
            out.append("reversion")
            continue

        out.append("unknown")
    return out


def outcome_regime_labels(
    candles: list[Candle],
    lookback_len: int = 20,
    horizon: int = 8,
) -> list[str]:
    """Forward-outcome labels for offline validation only.

    These labels deliberately look forward, so they must never be used as live
    factors. They are an independent benchmark target for asking whether a
    current-state regime descriptor separates future behavior.
    """
    out: list[str] = []
    for idx, candle in enumerate(candles):
        if idx < lookback_len or idx + horizon >= len(candles):
            out.append("unknown")
            continue
        past = candles[idx - lookback_len : idx]
        future = candles[idx + 1 : idx + horizon + 1]
        avg_range = mean_range(past)
        if not math.isfinite(avg_range) or avg_range <= 0.0:
            out.append("unknown")
            continue

        prev_max_high = max(c.high for c in past)
        prev_min_low = min(c.low for c in past)
        mean_close = sum(c.close for c in past) / lookback_len
        future_high = max(c.high for c in future)
        future_low = min(c.low for c in future)
        future_close = future[-1].close
        future_range = max(0.0, future_high - future_low)
        forward_return = future_close - candle.close
        prior_return = candle.close - candles[idx - lookback_len].close
        displacement = candle.close - mean_close
        bar_range = max(0.0, candle.high - candle.low)

        if bar_range > 0.0:
            swept_high = candle.high > prev_max_high and candle.close < (
                candle.high - 0.6 * bar_range
            )
            swept_low = candle.low < prev_min_low and candle.close > (
                candle.low + 0.6 * bar_range
            )
            if swept_high and forward_return < -0.5 * avg_range:
                out.append("manipulation")
                continue
            if swept_low and forward_return > 0.5 * avg_range:
                out.append("manipulation")
                continue

        if future_range < 0.85 * avg_range and bar_range < 0.85 * avg_range:
            out.append("compression")
            continue

        if (
            abs(displacement) > 1.0 * avg_range
            and sign(displacement) != 0.0
            and sign(forward_return) == -sign(displacement)
            and abs(future_close - mean_close) < abs(displacement)
        ):
            out.append("reversion")
            continue

        if future_range > 1.8 * avg_range and abs(forward_return) > 0.8 * avg_range:
            out.append("expansion")
            continue

        prior_dir = sign(prior_return)
        future_dir = sign(forward_return)
        if prior_dir != 0.0 and future_dir == prior_dir and abs(forward_return) > 0.45 * avg_range:
            out.append("trend_continuation")
            continue

        out.append("unknown")
    return out


def behavior_regime_labels(
    candles: list[Candle],
    lookback_len: int = 20,
    horizon: int = 8,
) -> list[str]:
    """Forward behavior labels for regime-family validation.

    This is still an offline-only benchmark target, but it is less tied to the
    MECE bar taxonomy than `outcome_regime_labels`. It asks what the next path
    actually behaved like: efficient trend, expansion, range/compression,
    reversion, or unstable transition.
    """
    out: list[str] = []
    for idx, candle in enumerate(candles):
        if idx < lookback_len or idx + horizon >= len(candles):
            out.append("unknown")
            continue
        past = candles[idx - lookback_len : idx]
        future = candles[idx + 1 : idx + horizon + 1]
        avg_range = mean_range(past)
        if not math.isfinite(avg_range) or avg_range <= 0.0:
            out.append("unknown")
            continue

        mean_close = sum(c.close for c in past) / lookback_len
        future_high = max(c.high for c in future)
        future_low = min(c.low for c in future)
        future_close = future[-1].close
        future_range = max(0.0, future_high - future_low)
        forward_return = future_close - candle.close
        net_move = abs(forward_return)
        efficiency = net_move / max(future_range, 1e-12)
        prior_return = candle.close - candles[idx - lookback_len].close
        prior_dir = sign(prior_return)
        future_dir = sign(forward_return)
        displacement = candle.close - mean_close

        if future_range < 0.70 * avg_range:
            out.append("compression")
            continue

        if future_range > 1.40 * avg_range and efficiency < 0.25:
            out.append("manipulation")
            continue

        if (
            abs(displacement) > 0.80 * avg_range
            and sign(displacement) != 0.0
            and future_dir == -sign(displacement)
            and abs(future_close - mean_close) < abs(displacement)
        ):
            out.append("reversion")
            continue

        if future_range > 1.20 * avg_range and net_move > 0.55 * avg_range and efficiency >= 0.35:
            if prior_dir != 0.0 and future_dir == prior_dir:
                out.append("trend_continuation")
            else:
                out.append("expansion")
            continue

        if prior_dir != 0.0 and future_dir == prior_dir and net_move > 0.45 * avg_range:
            out.append("trend_continuation")
            continue

        out.append("unknown")
    return out


def transition_event_labels(
    candles: list[Candle],
    lookback_len: int = 20,
    horizon: int = 8,
) -> list[str]:
    """Offline target for future regime-family transition events."""
    structural = manual_mece_labels(candles)
    behavior = behavior_regime_labels(candles, lookback_len=lookback_len, horizon=horizon)
    out: list[str] = []
    for idx, candle in enumerate(candles):
        if idx < lookback_len or idx + horizon >= len(candles):
            out.append("unknown")
            continue
        past = candles[idx - lookback_len : idx]
        future = candles[idx + 1 : idx + horizon + 1]
        avg_range = mean_range(past)
        if not math.isfinite(avg_range) or avg_range <= 0.0:
            out.append("unknown")
            continue
        future_high = max(c.high for c in future)
        future_low = min(c.low for c in future)
        future_range = max(0.0, future_high - future_low)
        now_family = regime_family(structural[idx])
        future_family = regime_family(behavior[idx])

        changed_family = (
            now_family != "unknown"
            and future_family != "unknown"
            and now_family != future_family
        )
        if future_family == "transition" or (changed_family and future_range > avg_range):
            out.append("manipulation")
            continue
        if future_family == "trend":
            out.append("trend_continuation")
            continue
        if future_family == "range":
            if future_range < 0.80 * avg_range:
                out.append("compression")
            else:
                out.append("reversion")
            continue
        out.append("unknown")
    return out


def transition_binary_labels(
    candles: list[Candle],
    lookback_len: int = 20,
    horizon: int = 8,
) -> list[str]:
    """Binary transition-event target.

    A transition event is encoded as `manipulation` because that label belongs
    to the transition family in the existing report machinery. Non-events are
    left as `unknown` so this target tests event detection separately from
    post-event state classification.
    """
    structural = manual_mece_labels(candles)
    behavior = behavior_regime_labels(candles, lookback_len=lookback_len, horizon=horizon)
    out: list[str] = []
    for idx, _candle in enumerate(candles):
        if idx < lookback_len or idx + horizon >= len(candles):
            out.append("unknown")
            continue
        past = candles[idx - lookback_len : idx]
        future = candles[idx + 1 : idx + horizon + 1]
        avg_range = mean_range(past)
        if not math.isfinite(avg_range) or avg_range <= 0.0:
            out.append("unknown")
            continue
        future_range = max(c.high for c in future) - min(c.low for c in future)
        now_family = regime_family(structural[idx])
        future_family = regime_family(behavior[idx])
        changed_family = (
            now_family != "unknown"
            and future_family != "unknown"
            and now_family != future_family
        )
        transition_family = future_family == "transition"
        range_gate = future_range >= 0.85 * avg_range
        if (changed_family and range_gate) or transition_family:
            out.append("manipulation")
        else:
            out.append("unknown")
    return out


def post_transition_state_labels(
    candles: list[Candle],
    lookback_len: int = 20,
    horizon: int = 8,
) -> list[str]:
    """Post-transition state target after a binary transition gate."""
    structural = manual_mece_labels(candles)
    behavior = behavior_regime_labels(candles, lookback_len=lookback_len, horizon=horizon)
    out: list[str] = []
    for idx, _candle in enumerate(candles):
        if idx < lookback_len or idx + horizon >= len(candles):
            out.append("unknown")
            continue
        past = candles[idx - lookback_len : idx]
        future = candles[idx + 1 : idx + horizon + 1]
        avg_range = mean_range(past)
        if not math.isfinite(avg_range) or avg_range <= 0.0:
            out.append("unknown")
            continue
        future_range = max(c.high for c in future) - min(c.low for c in future)
        now_family = regime_family(structural[idx])
        future_label = behavior[idx]
        future_family = regime_family(future_label)
        changed_family = (
            now_family != "unknown"
            and future_family != "unknown"
            and now_family != future_family
        )
        if not changed_family and future_family != "transition":
            out.append("unknown")
            continue
        if future_family == "trend":
            out.append("trend_continuation")
        elif future_family == "range":
            out.append("compression" if future_range < 0.85 * avg_range else "reversion")
        elif future_family == "transition":
            out.append("manipulation")
        else:
            out.append("unknown")
    return out


def post_transition_state_balanced_labels(
    candles: list[Candle],
    lookback_len: int = 20,
    horizon: int = 8,
) -> list[str]:
    """Post-transition state target with a less brittle range split.

    The first post-transition target made `compression` nearly absent. This
    variant treats post-event range absorption as compression when the forward
    range remains below 1.5x the prior range, while larger range-state moves
    remain `reversion`.
    """
    structural = manual_mece_labels(candles)
    behavior = behavior_regime_labels(candles, lookback_len=lookback_len, horizon=horizon)
    out: list[str] = []
    for idx, _candle in enumerate(candles):
        if idx < lookback_len or idx + horizon >= len(candles):
            out.append("unknown")
            continue
        past = candles[idx - lookback_len : idx]
        future = candles[idx + 1 : idx + horizon + 1]
        avg_range = mean_range(past)
        if not math.isfinite(avg_range) or avg_range <= 0.0:
            out.append("unknown")
            continue
        future_range = max(c.high for c in future) - min(c.low for c in future)
        now_family = regime_family(structural[idx])
        future_family = regime_family(behavior[idx])
        changed_family = (
            now_family != "unknown"
            and future_family != "unknown"
            and now_family != future_family
        )
        if not changed_family and future_family != "transition":
            out.append("unknown")
            continue
        if future_family == "trend":
            out.append("trend_continuation")
        elif future_family == "range":
            out.append("compression" if future_range < 1.50 * avg_range else "reversion")
        elif future_family == "transition":
            out.append("manipulation")
        else:
            out.append("unknown")
    return out


def post_transition_direction_labels(
    candles: list[Candle],
    lookback_len: int = 20,
    horizon: int = 8,
) -> list[str]:
    """Post-event direction target after a transition gate.

    This target keeps only post-event directional resolution:
    trend-follow-through versus true reversion. Compression and still-chaotic
    transition outcomes are left as `unknown` so Stage 2 can answer a narrower
    question than the broad post-state target.
    """
    structural = manual_mece_labels(candles)
    behavior = behavior_regime_labels(candles, lookback_len=lookback_len, horizon=horizon)
    out: list[str] = []
    for idx, _candle in enumerate(candles):
        if idx < lookback_len or idx + horizon >= len(candles):
            out.append("unknown")
            continue
        now_family = regime_family(structural[idx])
        future_label = behavior[idx]
        future_family = regime_family(future_label)
        changed_family = (
            now_family != "unknown"
            and future_family != "unknown"
            and now_family != future_family
        )
        if not changed_family and future_family != "transition":
            out.append("unknown")
            continue
        if future_family == "trend":
            out.append("trend_continuation")
        elif future_label == "reversion":
            out.append("reversion")
        else:
            out.append("unknown")
    return out


def post_transition_absorption_labels(
    candles: list[Candle],
    lookback_len: int = 20,
    horizon: int = 8,
) -> list[str]:
    """Post-event range absorption target after a transition gate.

    This target asks only whether the post-event range resolves into compression
    or a larger reversion-style range. Trend and still-transition outcomes are
    left as `unknown`.
    """
    structural = manual_mece_labels(candles)
    behavior = behavior_regime_labels(candles, lookback_len=lookback_len, horizon=horizon)
    out: list[str] = []
    for idx, _candle in enumerate(candles):
        if idx < lookback_len or idx + horizon >= len(candles):
            out.append("unknown")
            continue
        past = candles[idx - lookback_len : idx]
        future = candles[idx + 1 : idx + horizon + 1]
        avg_range = mean_range(past)
        if not math.isfinite(avg_range) or avg_range <= 0.0:
            out.append("unknown")
            continue
        future_range = max(c.high for c in future) - min(c.low for c in future)
        now_family = regime_family(structural[idx])
        future_family = regime_family(behavior[idx])
        changed_family = (
            now_family != "unknown"
            and future_family != "unknown"
            and now_family != future_family
        )
        if not changed_family and future_family != "transition":
            out.append("unknown")
            continue
        if future_family != "range":
            out.append("unknown")
            continue
        out.append("compression" if future_range < 1.50 * avg_range else "reversion")
    return out


def hmm_viterbi_labels(candles: list[Candle], train_fraction: float = 0.70) -> list[str]:
    """Independent unsupervised HMM/Viterbi regime labels.

    The model is fit only on the training prefix, then decoded across the full
    series with fixed Gaussian emissions and transition probabilities. These
    labels are offline validation targets, not live factors.
    """
    if len(candles) < 500:
        return ["unknown"] * len(candles)
    features = build_features(candles)
    vectors = scalar_feature_vectors(candles, features)
    train_end = int(len(candles) * min(0.95, max(0.05, train_fraction)))
    observations, usable = hmm_observations(vectors, train_end)
    train_indices = [idx for idx in range(train_end) if usable[idx]]
    if len(train_indices) < 200:
        return ["unknown"] * len(candles)

    state_count = min(5, max(3, len(train_indices) // 2000))
    assignments = kmeans_assignments(observations, train_indices, state_count)
    model = fit_hmm_cluster_model(
        observations,
        train_indices,
        assignments,
        state_count,
    )
    decoded = viterbi_decode(observations, usable, model)
    return [
        model.state_labels.get(state, "unknown") if state >= 0 else "unknown"
        for state in decoded
    ]


def change_point_labels(
    candles: list[Candle],
    train_fraction: float = 0.70,
    window: int = 24,
) -> list[str]:
    """Offline change-point segmentation labels.

    Thresholds are learned from the training prefix. Segment labels are assigned
    from post-segmentation behavior, so this is validation truth only.
    """
    if len(candles) < window * 8:
        return ["unknown"] * len(candles)
    features = build_features(candles)
    vectors = scalar_feature_vectors(candles, features)
    observations, usable = hmm_observations(vectors, int(len(candles) * min(0.95, max(0.05, train_fraction))))
    scores = change_point_scores(observations, usable, window)
    train_end = int(len(candles) * min(0.95, max(0.05, train_fraction)))
    train_scores = [score for score in scores[window : max(window + 1, train_end - window)] if math.isfinite(score)]
    if len(train_scores) < 100:
        return ["unknown"] * len(candles)
    threshold = quantile(train_scores, 0.92)
    points = select_change_points(scores, threshold, max(6, window // 2), window)
    labels = ["unknown"] * len(candles)
    boundaries = [0] + points + [len(candles)]
    for start, end in zip(boundaries, boundaries[1:]):
        label = label_change_point_segment(candles, vectors, start, end)
        for idx in range(start, end):
            labels[idx] = label
    for point in points:
        for idx in range(max(0, point - 2), min(len(labels), point + 3)):
            labels[idx] = "manipulation"
    return labels


def walk_forward_hmm_labels(candles: list[Candle]) -> list[str]:
    """Walk-forward HMM labels fit only on prior windows.

    Each evaluation slice is labeled by an HMM/Viterbi model trained on the
    immediately preceding window. This is slower than `hmm_viterbi`, but it is a
    better stability check because future bars are not used to fit the labeler.
    """
    count = len(candles)
    if count < 1000:
        return ["unknown"] * count
    train_window = max(500, min(20000, count // 3))
    eval_window = max(200, train_window // 4)
    labels = ["unknown"] * count
    eval_start = train_window
    while eval_start < count:
        eval_end = min(count, eval_start + eval_window)
        train_start = max(0, eval_start - train_window)
        chunk = candles[train_start:eval_end]
        train_fraction = (eval_start - train_start) / max(1, len(chunk))
        chunk_labels = hmm_viterbi_labels(chunk, train_fraction=train_fraction)
        for idx in range(eval_start, eval_end):
            labels[idx] = chunk_labels[idx - train_start]
        eval_start = eval_end
    return labels


def walk_forward_hmm_labels_budgeted(
    candles: list[Candle],
    train_window_max: int | None = None,
    eval_window_override: int | None = None,
) -> list[str]:
    """Budgeted walk-forward HMM labels for runtime-heavy long-span probes."""
    count = len(candles)
    if count < 1000:
        return ["unknown"] * count
    train_window = max(500, min(20000, count // 3))
    if train_window_max is not None:
        train_window = max(500, min(train_window, train_window_max))
    eval_window = max(200, train_window // 4)
    if eval_window_override is not None:
        eval_window = max(200, eval_window_override)
    labels = ["unknown"] * count
    eval_start = train_window
    while eval_start < count:
        eval_end = min(count, eval_start + eval_window)
        train_start = max(0, eval_start - train_window)
        chunk = candles[train_start:eval_end]
        train_fraction = (eval_start - train_start) / max(1, len(chunk))
        chunk_labels = hmm_viterbi_labels(chunk, train_fraction=train_fraction)
        for idx in range(eval_start, eval_end):
            labels[idx] = chunk_labels[idx - train_start]
        eval_start = eval_end
    return labels


def walk_forward_hmm_feature_vectors(
    candles: list[Candle],
    vectors: dict[str, list[float]] | None = None,
    include_bridge: bool = False,
) -> dict[str, list[float]]:
    labels = walk_forward_hmm_labels(candles)
    out = {name: [] for name in CLUSTER_VECTOR_FEATURES}
    if include_bridge:
        if vectors is None:
            features = build_features(candles)
            vectors = scalar_feature_vectors(candles, features)
        for name in CLUSTER_BRIDGE_VECTOR_FEATURES:
            out[name] = []
    prev = "unknown"
    age = 0
    for idx, label in enumerate(labels):
        family = regime_family(label)
        known = label != "unknown"
        transition = known and prev != "unknown" and label != prev
        age = 0 if transition or not known else age + 1
        for item in LABELS:
            out[f"wf_hmm_label_{item}"].append(1.0 if label == item else 0.0)
        for item in REGIME_FAMILIES:
            out[f"wf_hmm_family_{item}"].append(1.0 if family == item else 0.0)
        out["wf_hmm_known"].append(1.0 if known else 0.0)
        out["wf_hmm_transition"].append(1.0 if transition else 0.0)
        out["wf_hmm_segment_age20"].append(min(1.0, age / 20.0) if known else 0.0)
        if include_bridge and vectors is not None:
            for item in REGIME_FAMILIES:
                gate = 1.0 if family == item else 0.0
                for feature in CLUSTER_BRIDGE_BASE_FEATURES:
                    out[f"wf_bridge_{item}_{feature}"].append(
                        gate * finite_vector_value(vectors[feature][idx])
                    )
            transition_gate = 1.0 if transition else 0.0
            age20 = min(1.0, age / 20.0) if known else 0.0
            out["wf_bridge_transition_sweep"].append(
                transition_gate * finite_vector_value(vectors["sweep_displacement_score"][idx])
            )
            out["wf_bridge_transition_propulsion"].append(
                transition_gate * finite_vector_value(vectors["propulsion_score"][idx])
            )
            out["wf_bridge_age_reversion"].append(
                age20 * finite_vector_value(vectors["mean_reversion_pressure"][idx])
            )
            out["wf_bridge_age_efficiency"].append(
                age20 * finite_vector_value(vectors["prior_efficiency20"][idx])
            )
            out["wf_bridge_known_range"].append(
                (1.0 if known else 0.0) * finite_vector_value(vectors["range_atr"][idx])
            )
        prev = label
    return out


def hmm_viterbi_feature_vectors(
    candles: list[Candle],
) -> dict[str, list[float]]:
    """Cheaper static HMM/Viterbi cluster features from one global fit."""
    labels = hmm_viterbi_labels(candles)
    out = {name: [] for name in CLUSTER_VECTOR_FEATURES}
    prev = "unknown"
    age = 0
    for label in labels:
        family = regime_family(label)
        known = label != "unknown"
        transition = known and prev != "unknown" and label != prev
        age = 0 if transition or not known else age + 1
        for item in LABELS:
            out[f"wf_hmm_label_{item}"].append(1.0 if label == item else 0.0)
        for item in REGIME_FAMILIES:
            out[f"wf_hmm_family_{item}"].append(1.0 if family == item else 0.0)
        out["wf_hmm_known"].append(1.0 if known else 0.0)
        out["wf_hmm_transition"].append(1.0 if transition else 0.0)
        out["wf_hmm_segment_age20"].append(min(1.0, age / 20.0) if known else 0.0)
        prev = label
    return out


def kmeans_cluster_feature_vectors(
    candles: list[Candle],
    vectors: dict[str, list[float]],
    train_fraction: float = 0.70,
) -> dict[str, list[float]]:
    """Cheaper static cluster features using one k-means pass on scalar vectors."""
    train_end = int(len(candles) * min(0.95, max(0.05, train_fraction)))
    observations, usable = hmm_observations(vectors, train_end)
    train_indices = [idx for idx in range(train_end) if usable[idx]]
    if len(train_indices) < 200:
        return {name: [0.0] * len(candles) for name in CLUSTER_VECTOR_FEATURES}

    state_count = min(5, max(3, len(train_indices) // 2000))
    assignments = kmeans_assignments(observations, train_indices, state_count)
    model = fit_hmm_cluster_model(
        observations,
        train_indices,
        assignments,
        state_count,
    )
    labels = [
        model.state_labels.get(assignments[idx], "unknown") if usable[idx] else "unknown"
        for idx in range(len(candles))
    ]
    out = {name: [] for name in CLUSTER_VECTOR_FEATURES}
    prev = "unknown"
    age = 0
    for label in labels:
        family = regime_family(label)
        known = label != "unknown"
        transition = known and prev != "unknown" and label != prev
        age = 0 if transition or not known else age + 1
        for item in LABELS:
            out[f"wf_hmm_label_{item}"].append(1.0 if label == item else 0.0)
        for item in REGIME_FAMILIES:
            out[f"wf_hmm_family_{item}"].append(1.0 if family == item else 0.0)
        out["wf_hmm_known"].append(1.0 if known else 0.0)
        out["wf_hmm_transition"].append(1.0 if transition else 0.0)
        out["wf_hmm_segment_age20"].append(min(1.0, age / 20.0) if known else 0.0)
        prev = label
    return out


def prototype_cluster_feature_vectors(
    candles: list[Candle],
    vectors: dict[str, list[float]],
    train_fraction: float = 0.70,
) -> dict[str, list[float]]:
    """Continuous cluster-family features from one global prototype fit."""
    train_end = int(len(candles) * min(0.95, max(0.05, train_fraction)))
    observations, usable = hmm_observations(vectors, train_end)
    train_indices = [idx for idx in range(train_end) if usable[idx]]
    out = {name: [] for name in CLUSTER_PROTO_VECTOR_FEATURES}
    if len(train_indices) < 200:
        for _ in candles:
            out["cluster_proto_trend_prob"].append(0.0)
            out["cluster_proto_range_prob"].append(0.0)
            out["cluster_proto_transition_prob"].append(0.0)
            out["cluster_proto_margin"].append(0.0)
            out["cluster_proto_entropy"].append(1.0)
            out["cluster_proto_known"].append(0.0)
            out["cluster_proto_age20"].append(0.0)
        return out

    state_count = min(5, max(3, len(train_indices) // 2000))
    assignments = kmeans_assignments(observations, train_indices, state_count)
    model = fit_hmm_cluster_model(
        observations,
        train_indices,
        assignments,
        state_count,
    )

    prev_family = "unknown"
    age = 0
    for idx in range(len(candles)):
        if not usable[idx]:
            out["cluster_proto_trend_prob"].append(0.0)
            out["cluster_proto_range_prob"].append(0.0)
            out["cluster_proto_transition_prob"].append(0.0)
            out["cluster_proto_margin"].append(0.0)
            out["cluster_proto_entropy"].append(1.0)
            out["cluster_proto_known"].append(0.0)
            out["cluster_proto_age20"].append(0.0)
            continue

        family_scores = {"trend": -float("inf"), "range": -float("inf"), "transition": -float("inf")}
        for state, mean in enumerate(model.means):
            family = regime_family(model.state_labels.get(state, "unknown"))
            if family == "unknown":
                continue
            score = gaussian_logp(observations[idx], mean, model.variances[state])
            family_scores[family] = max(family_scores[family], score)
        finite_scores = {family: score for family, score in family_scores.items() if math.isfinite(score)}
        if not finite_scores:
            out["cluster_proto_trend_prob"].append(0.0)
            out["cluster_proto_range_prob"].append(0.0)
            out["cluster_proto_transition_prob"].append(0.0)
            out["cluster_proto_margin"].append(0.0)
            out["cluster_proto_entropy"].append(1.0)
            out["cluster_proto_known"].append(0.0)
            out["cluster_proto_age20"].append(0.0)
            continue

        top_family, top_score = max(finite_scores.items(), key=lambda item: item[1])
        second_score = sorted(finite_scores.values(), reverse=True)[1] if len(finite_scores) > 1 else top_score
        shift = max(finite_scores.values())
        weights = {family: math.exp(score - shift) for family, score in finite_scores.items()}
        total = sum(weights.values())
        probs = {family: weights.get(family, 0.0) / total for family in ["trend", "range", "transition"]}
        entropy = -sum(prob * math.log(max(prob, 1e-12)) for prob in probs.values() if prob > 0.0)
        family_transition = top_family != prev_family and prev_family != "unknown"
        age = 0 if family_transition else age + 1
        out["cluster_proto_trend_prob"].append(probs["trend"])
        out["cluster_proto_range_prob"].append(probs["range"])
        out["cluster_proto_transition_prob"].append(probs["transition"])
        out["cluster_proto_margin"].append(max(0.0, min(1.0, (top_score - second_score) / 10.0)))
        out["cluster_proto_entropy"].append(min(1.5, entropy))
        out["cluster_proto_known"].append(1.0)
        out["cluster_proto_age20"].append(min(1.0, age / 20.0))
        prev_family = top_family
    return out


def vol_regime_feature_vectors(
    base_candles: list[Candle],
    paired_series: dict[str, list[Candle]],
    max_age_seconds: int = 172800,
) -> dict[str, list[float]]:
    """Volatility-regime descriptors from implied/realized vol proxy series."""
    out = {name: [] for name in VOL_REGIME_VECTOR_FEATURES}
    aligned: dict[str, list[float]] = {}
    for name, candles in paired_series.items():
        series_name = name.lower()
        timestamps = [candle.timestamp for candle in candles]
        values: list[float] = []
        for candle in base_candles:
            pos = bisect.bisect_right(timestamps, candle.timestamp) - 1
            if pos < 0 or (candle.timestamp - timestamps[pos]).total_seconds() > max_age_seconds:
                values.append(float("nan"))
            else:
                values.append(float(candles[pos].close))
        aligned[series_name] = values

    def first_series(*needles: str) -> list[float] | None:
        for name, values in aligned.items():
            if all(needle in name for needle in needles):
                return values
        return None

    iv = first_series("iv")
    hv = first_series("hv")
    vix = aligned.get("vix") or first_series("vix")

    spread_history: list[float] = []
    for idx in range(len(base_candles)):
        iv_value = iv[idx] if iv is not None else float("nan")
        hv_value = hv[idx] if hv is not None else float("nan")
        vix_value = vix[idx] if vix is not None else float("nan")
        spread = iv_value - hv_value if math.isfinite(iv_value) and math.isfinite(hv_value) else float("nan")
        spread_history.append(spread)

        out["vol_iv_level_z20"].append(rolling_z_value(iv, idx, 20))
        out["vol_hv_level_z20"].append(rolling_z_value(hv, idx, 20))
        out["vol_vix_level_z20"].append(rolling_z_value(vix, idx, 20))
        out["vol_vrp_spread"].append(spread)
        out["vol_vrp_ratio"].append(iv_value / max(hv_value, 1e-12) if math.isfinite(iv_value) and math.isfinite(hv_value) else float("nan"))
        out["vol_vrp_spread_z20"].append(rolling_z_from_history(spread_history, idx, 20))
        out["vol_vrp_change3"].append(series_change(spread_history, idx, 3))
        out["vol_vrp_change8"].append(series_change(spread_history, idx, 8))
        out["vol_vix_hv_gap"].append(vix_value - hv_value if math.isfinite(vix_value) and math.isfinite(hv_value) else float("nan"))
        out["vol_vix_iv_gap"].append(vix_value - iv_value if math.isfinite(vix_value) and math.isfinite(iv_value) else float("nan"))
        out["vol_iv_trend3"].append(series_change(iv, idx, 3))
        out["vol_hv_trend3"].append(series_change(hv, idx, 3))
        out["vol_vix_trend3"].append(series_change(vix, idx, 3))
    return out


def hazard_feature_vectors(
    candles: list[Candle],
    features: dict[str, list[float] | list[bool]],
    vectors: dict[str, list[float]],
) -> dict[str, list[float]]:
    """Continuous historical-only hazard descriptors for regime shifts."""
    out = {name: [] for name in HAZARD_VECTOR_FEATURES}
    range_history: list[float] = []
    body_history: list[float] = []
    chop_history: list[float] = []
    volume_history: list[float] = []
    sweep_history: list[float] = []
    for idx, candle in enumerate(candles):
        atr = max(float(features["atr"][idx]), 1e-12)  # type: ignore[index]
        bar_range = max(0.0, candle.high - candle.low) / atr
        body_signed = finite_vector_value(vectors["body_signed_atr"][idx])
        chop = finite_vector_value(vectors["chop20"][idx])
        volume_z = finite_vector_value(vectors["volume_z20"][idx])
        sweep = max(
            finite_vector_value(vectors["sweep_reject"][idx]),
            finite_vector_value(vectors["sweep_displacement_score"][idx]),
        )
        breakout = finite_vector_value(vectors["breakout_atr"][idx])
        bb_ratio = finite_vector_value(vectors["bb_width_ratio"][idx])
        atr_ratio = finite_vector_value(vectors["atr_pct_ratio"][idx])
        mean_revert = finite_vector_value(vectors["mean_reversion_pressure"][idx])
        efficiency = finite_vector_value(vectors["prior_efficiency20"][idx])
        ema_gap = finite_vector_value(vectors["ema_gap"][idx])
        ema_slope = finite_vector_value(vectors["ema21_slope_atr"][idx])
        premium_edge = finite_vector_value(vectors["premium_discount_edge"][idx])
        propulsion = finite_vector_value(vectors["propulsion_score"][idx])

        range_history.append(bar_range)
        body_history.append(abs(body_signed))
        chop_history.append(chop)
        volume_history.append(volume_z)
        sweep_history.append(sweep)

        out["hazard_range_shift_8_32"].append(window_shift(range_history, idx, 8, 32))
        out["hazard_body_shift_8_32"].append(window_shift(body_history, idx, 8, 32))
        out["hazard_chop_shift_8_32"].append(window_shift(chop_history, idx, 8, 32))
        out["hazard_volume_shift_8_32"].append(window_shift(volume_history, idx, 8, 32))
        out["hazard_sweep_shift_8_32"].append(window_shift(sweep_history, idx, 8, 32))
        short_range = window_mean(range_history, idx, 5)
        long_range = window_mean(range_history, idx, 20)
        slope_flip = sign(ema_slope) * sign(ema_gap)
        out["hazard_slope_flip_5_20"].append(
            (1.0 if slope_flip < 0.0 else 0.0) * abs(short_range - long_range)
            if math.isfinite(short_range) and math.isfinite(long_range)
            else float("nan")
        )
        out["hazard_breakout_pressure"].append(max(0.0, breakout) * max(0.0, 1.0 + volume_z))
        out["hazard_compression_release"].append(
            max(0.0, 1.15 - bb_ratio) * max(0.0, atr_ratio) * max(0.0, propulsion)
        )
        out["hazard_regime_tension"].append(
            abs(ema_gap) * mean_revert + premium_edge * max(0.0, 1.0 - efficiency)
        )
        out["hazard_direction_instability"].append(
            abs(body_signed) * max(0.0, 1.0 - efficiency) + sweep * mean_revert
        )
    return out


def bocpd_lite_feature_vectors(
    candles: list[Candle],
    features: dict[str, list[float] | list[bool]],
    vectors: dict[str, list[float]],
) -> dict[str, list[float]]:
    """Predictive-surprise and run-length proxies inspired by BOCPD."""
    out = {name: [] for name in BOCPD_LITE_VECTOR_FEATURES}
    series = {
        "range": [],
        "body": [],
        "chop": [],
        "volume": [],
        "joint": [],
    }
    last_break = -1000
    surprise_ema3 = 0.0
    surprise_ema8 = 0.0
    alpha3 = 2.0 / 4.0
    alpha8 = 2.0 / 9.0
    for idx, candle in enumerate(candles):
        atr = max(float(features["atr"][idx]), 1e-12)  # type: ignore[index]
        range_value = max(0.0, candle.high - candle.low) / atr
        body_value = abs(finite_vector_value(vectors["body_signed_atr"][idx]))
        chop_value = finite_vector_value(vectors["chop20"][idx])
        volume_value = finite_vector_value(vectors["volume_z20"][idx])
        series["range"].append(range_value)
        series["body"].append(body_value)
        series["chop"].append(chop_value)
        series["volume"].append(volume_value)

        range_surprise = predictive_surprise(series["range"], idx, 16, 64)
        body_surprise = predictive_surprise(series["body"], idx, 16, 64)
        chop_surprise = predictive_surprise(series["chop"], idx, 16, 64)
        volume_surprise = predictive_surprise(series["volume"], idx, 16, 64)
        joint_surprise = mean(
            [
                value
                for value in [range_surprise, body_surprise, chop_surprise, volume_surprise]
                if math.isfinite(value)
            ]
        ) if any(math.isfinite(value) for value in [range_surprise, body_surprise, chop_surprise, volume_surprise]) else float("nan")
        series["joint"].append(joint_surprise)

        if idx == 0 or not math.isfinite(joint_surprise):
            surprise_ema3 = joint_surprise if math.isfinite(joint_surprise) else 0.0
            surprise_ema8 = joint_surprise if math.isfinite(joint_surprise) else 0.0
        else:
            surprise_ema3 = alpha3 * joint_surprise + (1.0 - alpha3) * surprise_ema3
            surprise_ema8 = alpha8 * joint_surprise + (1.0 - alpha8) * surprise_ema8

        if math.isfinite(joint_surprise) and joint_surprise >= 1.8:
            last_break = idx
        run_decay = max(0.0, 1.0 - ((idx - last_break) / 20.0))
        hazard_prob = 1.0 - math.exp(-max(0.0, joint_surprise - 1.0)) if math.isfinite(joint_surprise) else 0.0
        dispersion = surprise_dispersion(series["joint"], idx, 8, 32)

        out["bocpd_range_surprise"].append(range_surprise)
        out["bocpd_body_surprise"].append(body_surprise)
        out["bocpd_chop_surprise"].append(chop_surprise)
        out["bocpd_volume_surprise"].append(volume_surprise)
        out["bocpd_joint_surprise"].append(joint_surprise)
        out["bocpd_joint_surprise_ema3"].append(surprise_ema3)
        out["bocpd_joint_surprise_ema8"].append(surprise_ema8)
        out["bocpd_hazard_prob"].append(min(1.0, hazard_prob))
        out["bocpd_run_decay20"].append(min(1.0, run_decay))
        out["bocpd_surprise_dispersion"].append(dispersion)
    return out


def ms_regime_feature_vectors(
    candles: list[Candle],
    vectors: dict[str, list[float]],
    train_fraction: float = 0.70,
) -> dict[str, list[float]]:
    """Markov-switching regression probabilities from statsmodels."""
    out = {name: [] for name in MS_REGIME_VECTOR_FEATURES}
    try:
        from statsmodels.tsa.regime_switching.markov_regression import MarkovRegression
    except Exception:
        for _ in candles:
            out["ms_regime_trend_prob"].append(0.0)
            out["ms_regime_range_prob"].append(0.0)
            out["ms_regime_transition_prob"].append(0.0)
            out["ms_regime_margin"].append(0.0)
            out["ms_regime_entropy"].append(1.0)
            out["ms_regime_known"].append(0.0)
        return out

    endog = [finite_vector_value(value) for value in vectors["body_signed_atr"]]
    train_end = int(len(candles) * min(0.95, max(0.05, train_fraction)))
    if train_end < 500:
        for _ in candles:
            out["ms_regime_trend_prob"].append(0.0)
            out["ms_regime_range_prob"].append(0.0)
            out["ms_regime_transition_prob"].append(0.0)
            out["ms_regime_margin"].append(0.0)
            out["ms_regime_entropy"].append(1.0)
            out["ms_regime_known"].append(0.0)
        return out

    try:
        model_train = MarkovRegression(
            endog[:train_end],
            k_regimes=3,
            trend="c",
            switching_variance=True,
        )
        result = model_train.fit(disp=False, maxiter=200)
        model_full = MarkovRegression(
            endog,
            k_regimes=3,
            trend="c",
            switching_variance=True,
        )
        filtered = model_full.filter(result.params)
    except Exception:
        for _ in candles:
            out["ms_regime_trend_prob"].append(0.0)
            out["ms_regime_range_prob"].append(0.0)
            out["ms_regime_transition_prob"].append(0.0)
            out["ms_regime_margin"].append(0.0)
            out["ms_regime_entropy"].append(1.0)
            out["ms_regime_known"].append(0.0)
        return out

    probs = filtered.filtered_marginal_probabilities
    state_count = probs.shape[1]
    state_stats: list[tuple[int, float, float]] = []
    for state in range(state_count):
        weights = [float(probs[idx, state]) for idx in range(train_end)]
        weight_total = sum(weights)
        if weight_total <= 1e-12:
            state_stats.append((state, 0.0, 0.0))
            continue
        mean = sum(weight * endog[idx] for idx, weight in enumerate(weights)) / weight_total
        variance = sum(weight * ((endog[idx] - mean) ** 2) for idx, weight in enumerate(weights)) / weight_total
        state_stats.append((state, mean, math.sqrt(max(variance, 0.0))))
    transition_state = max(state_stats, key=lambda item: item[2])[0]
    remaining = [item for item in state_stats if item[0] != transition_state]
    trend_state = max(remaining, key=lambda item: abs(item[1]))[0] if remaining else transition_state
    range_state = next((item[0] for item in remaining if item[0] != trend_state), trend_state)

    for idx in range(len(candles)):
        probs_row = [float(probs[idx, state]) for state in range(state_count)]
        trend_prob = probs_row[trend_state]
        range_prob = probs_row[range_state]
        transition_prob = probs_row[transition_state]
        ordered = sorted([trend_prob, range_prob, transition_prob], reverse=True)
        margin = ordered[0] - ordered[1] if len(ordered) > 1 else ordered[0]
        entropy = -sum(prob * math.log(max(prob, 1e-12)) for prob in [trend_prob, range_prob, transition_prob] if prob > 0.0)
        out["ms_regime_trend_prob"].append(trend_prob)
        out["ms_regime_range_prob"].append(range_prob)
        out["ms_regime_transition_prob"].append(transition_prob)
        out["ms_regime_margin"].append(margin)
        out["ms_regime_entropy"].append(entropy)
        out["ms_regime_known"].append(1.0)
    return out


def predictive_surprise(history: list[float], idx: int, short_window: int, long_window: int) -> float:
    if idx < long_window:
        return float("nan")
    baseline = [value for value in history[idx - long_window : idx] if math.isfinite(value)]
    recent = [value for value in history[max(0, idx - short_window) : idx] if math.isfinite(value)]
    current = history[idx]
    if not baseline or not recent or not math.isfinite(current):
        return float("nan")
    mean_base = sum(baseline) / len(baseline)
    variance_base = sum((value - mean_base) ** 2 for value in baseline) / max(1, len(baseline) - 1)
    std_base = math.sqrt(max(variance_base, 1e-12))
    mean_recent = sum(recent) / len(recent)
    z_current = abs(current - mean_base) / std_base
    z_recent = abs(mean_recent - mean_base) / std_base
    return 0.6 * z_current + 0.4 * z_recent


def surprise_dispersion(history: list[float], idx: int, short_window: int, long_window: int) -> float:
    if idx < long_window:
        return float("nan")
    recent = [value for value in history[idx - short_window : idx] if math.isfinite(value)]
    baseline = [value for value in history[idx - long_window : idx - short_window] if math.isfinite(value)]
    if len(recent) < 2 or len(baseline) < 2:
        return float("nan")
    mean_recent = sum(recent) / len(recent)
    mean_base = sum(baseline) / len(baseline)
    var_recent = sum((value - mean_recent) ** 2 for value in recent) / max(1, len(recent) - 1)
    var_base = sum((value - mean_base) ** 2 for value in baseline) / max(1, len(baseline) - 1)
    return math.sqrt(max(var_recent, 0.0)) - math.sqrt(max(var_base, 0.0))


def window_shift(history: list[float], idx: int, short: int, long: int) -> float:
    if idx + 1 < long:
        return float("nan")
    recent = [value for value in history[idx + 1 - short : idx + 1] if math.isfinite(value)]
    prior = [value for value in history[idx + 1 - long : idx + 1 - short] if math.isfinite(value)]
    if not recent or not prior:
        return float("nan")
    return (sum(recent) / len(recent)) - (sum(prior) / len(prior))


def window_mean(history: list[float], idx: int, window: int) -> float:
    if idx + 1 < window:
        return float("nan")
    values = [value for value in history[idx + 1 - window : idx + 1] if math.isfinite(value)]
    return sum(values) / len(values) if values else float("nan")


def rolling_z_value(values: list[float] | None, idx: int, window: int) -> float:
    if values is None or idx < window:
        return float("nan")
    history = [value for value in values[idx - window : idx] if math.isfinite(value)]
    current = values[idx]
    if not history or not math.isfinite(current):
        return float("nan")
    mean = sum(history) / len(history)
    variance = sum((value - mean) ** 2 for value in history) / max(1, len(history) - 1)
    std = math.sqrt(max(variance, 1e-12))
    return (current - mean) / std


def rolling_z_from_history(values: list[float], idx: int, window: int) -> float:
    if idx < window:
        return float("nan")
    history = [value for value in values[idx - window : idx] if math.isfinite(value)]
    current = values[idx]
    if not history or not math.isfinite(current):
        return float("nan")
    mean = sum(history) / len(history)
    variance = sum((value - mean) ** 2 for value in history) / max(1, len(history) - 1)
    std = math.sqrt(max(variance, 1e-12))
    return (current - mean) / std


def series_change(values: list[float] | None, idx: int, lookback: int) -> float:
    if values is None or idx < lookback:
        return float("nan")
    current = values[idx]
    prev = values[idx - lookback]
    if not math.isfinite(current) or not math.isfinite(prev):
        return float("nan")
    return current - prev


def walk_forward_hmm_feature_vectors_budgeted(
    candles: list[Candle],
    vectors: dict[str, list[float]] | None = None,
    include_bridge: bool = False,
    train_window_max: int | None = None,
    eval_window_override: int | None = None,
) -> dict[str, list[float]]:
    labels = walk_forward_hmm_labels_budgeted(
        candles,
        train_window_max=train_window_max,
        eval_window_override=eval_window_override,
    )
    out = {name: [] for name in CLUSTER_VECTOR_FEATURES}
    if include_bridge:
        if vectors is None:
            features = build_features(candles)
            vectors = scalar_feature_vectors(candles, features)
        for name in CLUSTER_BRIDGE_VECTOR_FEATURES:
            out[name] = []
    prev = "unknown"
    age = 0
    for idx, label in enumerate(labels):
        family = regime_family(label)
        known = label != "unknown"
        transition = known and prev != "unknown" and label != prev
        age = 0 if transition or not known else age + 1
        for item in LABELS:
            out[f"wf_hmm_label_{item}"].append(1.0 if label == item else 0.0)
        for item in REGIME_FAMILIES:
            out[f"wf_hmm_family_{item}"].append(1.0 if family == item else 0.0)
        out["wf_hmm_known"].append(1.0 if known else 0.0)
        out["wf_hmm_transition"].append(1.0 if transition else 0.0)
        out["wf_hmm_segment_age20"].append(min(1.0, age / 20.0))
        if include_bridge and vectors is not None:
            for item in REGIME_FAMILIES:
                gate = 1.0 if family == item else 0.0
                for feature in CLUSTER_BRIDGE_BASE_FEATURES:
                    out[f"wf_bridge_{item}_{feature}"].append(
                        gate * finite_vector_value(vectors[feature][idx])
                    )
            sweep = finite_vector_value(vectors["sweep_displacement_score"][idx])
            propulsion = finite_vector_value(vectors["propulsion_score"][idx])
            age_gate = min(1.0, age / 20.0)
            out["wf_bridge_transition_sweep"].append(
                (1.0 if family == "transition" else 0.0) * sweep
            )
            out["wf_bridge_transition_propulsion"].append(
                (1.0 if family == "transition" else 0.0) * propulsion
            )
            out["wf_bridge_age_reversion"].append(
                age_gate * finite_vector_value(vectors["mean_reversion_pressure"][idx])
            )
            out["wf_bridge_age_efficiency"].append(
                age_gate * finite_vector_value(vectors["prior_efficiency20"][idx])
            )
            out["wf_bridge_known_range"].append(
                (1.0 if known and family == "range" else 0.0)
                * finite_vector_value(vectors["range_atr"][idx])
            )
        prev = label if known else prev
    return out


def finite_vector_value(value: float) -> float:
    return value if math.isfinite(value) else 0.0


def event_decay(idx: int, last_idx: int | None, window: int) -> float:
    if last_idx is None:
        return 0.0
    age = idx - last_idx
    if age < 0 or age > window:
        return 0.0
    return max(0.0, 1.0 - age / max(window, 1))


def pda_sequence_feature_vectors(vectors: dict[str, list[float]]) -> dict[str, list[float]]:
    """Past-event PDA sequence features.

    These keep event order after sweeps / FVGs / order blocks without using
    future bars. The current bar can react to prior events, but the current
    event is only written into state after feature values are emitted.
    """
    length = len(next(iter(vectors.values()), []))
    out = {name: [] for name in PDA_SEQUENCE_VECTOR_FEATURES}
    last_sweep: int | None = None
    last_fvg: int | None = None
    last_ob: int | None = None
    last_breaker: int | None = None
    for idx in range(length):
        sweep = max(
            finite_vector_value(vectors["sweep_reject"][idx]),
            finite_vector_value(vectors["sweep_displacement_score"][idx]),
        )
        fvg = finite_vector_value(vectors["fvg_abs_atr"][idx])
        ob = finite_vector_value(vectors["order_block_touch_score"][idx])
        breaker = finite_vector_value(vectors["breaker_score"][idx])
        propulsion = finite_vector_value(vectors["propulsion_score"][idx])
        reversion = finite_vector_value(vectors["mean_reversion_pressure"][idx])
        efficiency = finite_vector_value(vectors["prior_efficiency20"][idx])
        fvg_mitigation = finite_vector_value(vectors["fvg_mitigation_score"][idx])
        fvg_failed = finite_vector_value(vectors["fvg_failed_mitigation_score"][idx])
        ob_cont = finite_vector_value(vectors["ob_post_mitigation_score"][idx])
        breaker_cont = finite_vector_value(vectors["breaker_continuation_score"][idx])

        sweep_gate = event_decay(idx, last_sweep, 8)
        fvg_gate = event_decay(idx, last_fvg, 8)
        ob_gate = event_decay(idx, last_ob, 10)
        breaker_gate = event_decay(idx, last_breaker, 8)
        recent_stack = sum(1.0 for gate in [sweep_gate, fvg_gate, ob_gate, breaker_gate] if gate > 0.0)

        out["seq_sweep_age8"].append(sweep_gate)
        out["seq_fvg_age8"].append(fvg_gate)
        out["seq_ob_age10"].append(ob_gate)
        out["seq_breaker_age8"].append(breaker_gate)
        out["seq_sweep_then_fvg6"].append(event_decay(idx, last_sweep, 6) * fvg)
        out["seq_sweep_then_propulsion6"].append(event_decay(idx, last_sweep, 6) * propulsion)
        out["seq_sweep_then_ob10"].append(sweep_gate * max(ob, ob_cont))
        out["seq_sweep_to_reversion6"].append(event_decay(idx, last_sweep, 6) * reversion)
        out["seq_sweep_to_efficiency6"].append(event_decay(idx, last_sweep, 6) * efficiency)
        out["seq_fvg_then_mitigation8"].append(fvg_gate * fvg_mitigation)
        out["seq_fvg_then_failed_mitigation8"].append(fvg_gate * fvg_failed)
        out["seq_ob_then_breaker10"].append(ob_gate * breaker)
        out["seq_ob_then_propulsion10"].append(ob_gate * propulsion)
        out["seq_breaker_then_continuation8"].append(breaker_gate * breaker_cont)
        out["seq_pda_recent_stack"].append(recent_stack / 4.0)
        out["seq_pda_order_score"].append(
            max(
                event_decay(idx, last_sweep, 6) * max(fvg, propulsion, reversion),
                fvg_gate * max(fvg_mitigation, fvg_failed),
                ob_gate * max(breaker, propulsion),
                breaker_gate * breaker_cont,
            )
        )

        if sweep >= 0.20:
            last_sweep = idx
        if fvg >= 0.15:
            last_fvg = idx
        if ob >= 0.15 or ob_cont >= 0.15:
            last_ob = idx
        if breaker >= 0.15 or breaker_cont >= 0.15:
            last_breaker = idx
    return out


def post_state_feature_vectors(
    candles: list[Candle],
    features: dict[str, list[float] | list[bool]],
    vectors: dict[str, list[float]],
) -> dict[str, list[float]]:
    """Stage-2 post-transition state features using current / historical bars."""
    out = {name: [] for name in POST_STATE_VECTOR_FEATURES}
    for idx, candle in enumerate(candles):
        atr = max(float(features["atr"][idx]), 1e-12)  # type: ignore[index]
        ret3 = (candle.close - candles[max(0, idx - 3)].close) / atr
        ret8 = (candle.close - candles[max(0, idx - 8)].close) / atr
        ret20 = (candle.close - candles[max(0, idx - 20)].close) / atr
        efficiency = finite_vector_value(vectors["prior_efficiency20"][idx])
        reversion = finite_vector_value(vectors["mean_reversion_pressure"][idx])
        sweep = max(
            finite_vector_value(vectors["sweep_reject"][idx]),
            finite_vector_value(vectors["sweep_displacement_score"][idx]),
        )
        bb_ratio = finite_vector_value(vectors["bb_width_ratio"][idx])
        chop = finite_vector_value(vectors["chop20"][idx])
        breakout = finite_vector_value(vectors["breakout_atr"][idx])
        ema_gap = finite_vector_value(vectors["ema_gap"][idx])
        body_signed = finite_vector_value(vectors["body_signed_atr"][idx])
        propulsion = finite_vector_value(vectors["propulsion_score"][idx])
        sweep_reversal = finite_vector_value(vectors["sweep_reversal_score"][idx])
        sweep_continuation = finite_vector_value(vectors["sweep_continuation_score"][idx])

        ret8_sign = sign(ret8)
        ema_sign = sign(ema_gap)
        body_sign = sign(body_signed)
        out["post_ret3_atr"].append(ret3)
        out["post_ret8_atr"].append(ret8)
        out["post_ret20_atr"].append(ret20)
        out["post_ret8_efficiency"].append(ret8 * efficiency)
        out["post_ret20_efficiency"].append(ret20 * efficiency)
        out["post_reversal_pressure"].append(sweep * reversion)
        out["post_absorption_pressure"].append(max(0.0, 1.25 - bb_ratio) * max(0.0, 1.0 - efficiency))
        out["post_breakout_persistence"].append(max(0.0, breakout) * max(0.0, efficiency))
        out["post_sweep_reversal_bias"].append(max(sweep_reversal, sweep * reversion))
        out["post_sweep_continuation_bias"].append(max(sweep_continuation, sweep * max(0.0, efficiency)))
        out["post_trend_exhaustion"].append(abs(ret20) * reversion * max(0.0, bb_ratio))
        out["post_range_absorb_chop"].append(chop * max(0.0, 1.0 - efficiency) * max(0.0, 1.25 - bb_ratio))
        conflict = 1.0 if ret8_sign != 0.0 and ema_sign != 0.0 and ret8_sign != ema_sign else 0.0
        if body_sign != 0.0 and ret8_sign != 0.0 and body_sign != ret8_sign:
            conflict = max(conflict, 0.5)
        out["post_direction_conflict"].append(conflict * min(2.0, abs(ret8)) + propulsion * max(0.0, efficiency))
    return out


def wants_cluster_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return (
        "cluster" in selected
        or "cluster_bridge" in selected
        or "cluster_static" in selected
        or "cluster_kmeans" in selected
        or "cluster_proto" in selected
    )


def wants_cluster_bridge_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "cluster_bridge" in selected


def wants_static_cluster_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "cluster_static" in selected


def wants_kmeans_cluster_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "cluster_kmeans" in selected


def wants_proto_cluster_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "cluster_proto" in selected


def wants_vol_regime_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "vol_regime" in selected


def wants_hazard_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "hazard" in selected


def wants_bocpd_lite_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "bocpd_lite" in selected


def wants_ms_regime_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "ms_regime" in selected


def wants_pda_sequence_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "pda_sequence" in selected


def wants_post_state_features(feature_sets: list[str] | None) -> bool:
    selected = normalize_feature_sets(feature_sets)
    return "post_state" in selected


def change_point_scores(
    observations: list[list[float]],
    usable: list[bool],
    window: int,
) -> list[float]:
    if not observations:
        return []
    dim = len(observations[0])
    prefix = [[0.0] * dim]
    counts = [0]
    running = [0.0] * dim
    count = 0
    for obs, ok in zip(observations, usable):
        if ok:
            count += 1
            for idx, value in enumerate(obs):
                running[idx] += value
        prefix.append(running[:])
        counts.append(count)
    scores = [float("nan")] * len(observations)
    for idx in range(window, len(observations) - window):
        prev_count = counts[idx] - counts[idx - window]
        next_count = counts[idx + window] - counts[idx]
        if prev_count <= window // 2 or next_count <= window // 2:
            continue
        total = 0.0
        for feature_idx in range(dim):
            prev_mean = (prefix[idx][feature_idx] - prefix[idx - window][feature_idx]) / prev_count
            next_mean = (prefix[idx + window][feature_idx] - prefix[idx][feature_idx]) / next_count
            diff = next_mean - prev_mean
            total += diff * diff
        scores[idx] = math.sqrt(total / dim)
    return scores


def quantile(values: list[float], q: float) -> float:
    if not values:
        return float("nan")
    ordered = sorted(values)
    pos = min(len(ordered) - 1, max(0, int(round((len(ordered) - 1) * q))))
    return ordered[pos]


def select_change_points(
    scores: list[float],
    threshold: float,
    min_spacing: int,
    edge: int,
) -> list[int]:
    if not math.isfinite(threshold):
        return []
    out: list[int] = []
    idx = edge
    limit = len(scores) - edge
    while idx < limit:
        score = scores[idx]
        if not math.isfinite(score) or score < threshold:
            idx += 1
            continue
        left = max(edge, idx - min_spacing)
        right = min(limit, idx + min_spacing + 1)
        if score >= max(value for value in scores[left:right] if math.isfinite(value)):
            out.append(idx)
            idx += min_spacing
        else:
            idx += 1
    return out


def label_change_point_segment(
    candles: list[Candle],
    vectors: dict[str, list[float]],
    start: int,
    end: int,
) -> str:
    if end - start < 6:
        return "unknown"
    segment = candles[start:end]
    net_move = segment[-1].close - segment[0].close
    path = sum(abs(cur.close - prev.close) for prev, cur in zip(segment, segment[1:]))
    efficiency = abs(net_move) / max(path, 1e-12)
    avg_bar_range = sum(max(0.0, candle.high - candle.low) for candle in segment) / len(segment)
    range_mean = finite_slice_mean(vectors["range_atr"], start, end)
    bb_mean = finite_slice_mean(vectors["bb_width_ratio"], start, end)
    chop_mean = finite_slice_mean(vectors["chop20"], start, end)
    sweep_mean = finite_slice_mean(vectors["sweep_displacement_score"], start, end)
    revert_mean = finite_slice_mean(vectors["mean_reversion_pressure"], start, end)
    volume_mean = finite_slice_mean(vectors["volume_z50"], start, end)
    atr_ratio_mean = finite_slice_mean(vectors["atr_pct_ratio"], start, end)

    if sweep_mean >= 0.45 or (range_mean >= 1.35 and efficiency <= 0.20 and volume_mean >= 0.40):
        return "manipulation"
    if range_mean >= 1.25 and (bb_mean >= 1.00 or atr_ratio_mean >= 1.05) and volume_mean >= -0.25:
        if efficiency >= 0.38 and abs(net_move) / max(avg_bar_range, 1e-12) >= 1.20:
            return "trend_continuation"
        return "expansion"
    if efficiency >= 0.45 and abs(net_move) / max(avg_bar_range, 1e-12) >= 1.20:
        return "trend_continuation"
    if range_mean <= 0.90 and (bb_mean <= 0.95 or chop_mean >= 1.40):
        return "compression"
    if revert_mean >= 0.65 and efficiency <= 0.35:
        return "reversion"
    return "unknown"


def finite_slice_mean(values: list[float], start: int, end: int) -> float:
    usable = [value for value in values[start:end] if math.isfinite(value)]
    return sum(usable) / len(usable) if usable else 0.0


def hmm_observations(
    vectors: dict[str, list[float]],
    train_end: int,
) -> tuple[list[list[float]], list[bool]]:
    stats: dict[str, tuple[float, float]] = {}
    for name in HMM_VECTOR_FEATURES:
        values = [float(value) for value in vectors[name][:train_end]]
        finite = [value for value in values if math.isfinite(value)]
        if not finite:
            stats[name] = (0.0, 1.0)
            continue
        mean = sum(finite) / len(finite)
        variance = sum((value - mean) ** 2 for value in finite) / max(1, len(finite) - 1)
        stats[name] = (mean, max(math.sqrt(max(variance, 0.0)), 1e-6))

    observations: list[list[float]] = []
    usable: list[bool] = []
    count = len(next(iter(vectors.values()))) if vectors else 0
    for idx in range(count):
        vector: list[float] = []
        finite_count = 0
        for name in HMM_VECTOR_FEATURES:
            value = float(vectors[name][idx])
            mean, std = stats[name]
            if math.isfinite(value):
                vector.append(max(-8.0, min(8.0, (value - mean) / std)))
                finite_count += 1
            else:
                vector.append(0.0)
        observations.append(vector)
        usable.append(idx >= 120 and finite_count >= max(6, len(HMM_VECTOR_FEATURES) // 2))
    return observations, usable


def kmeans_assignments(
    observations: list[list[float]],
    train_indices: list[int],
    state_count: int,
    iterations: int = 10,
) -> list[int]:
    seeds = [
        train_indices[int(round((len(train_indices) - 1) * rank / max(1, state_count - 1)))]
        for rank in range(state_count)
    ]
    centroids = [observations[idx][:] for idx in seeds]
    assignments = [0] * len(observations)
    for _ in range(iterations):
        buckets = [[0.0] * len(HMM_VECTOR_FEATURES) for _ in range(state_count)]
        counts = [0] * state_count
        for idx in train_indices:
            state = nearest_centroid(observations[idx], centroids)
            assignments[idx] = state
            counts[state] += 1
            for feature_idx, value in enumerate(observations[idx]):
                buckets[state][feature_idx] += value
        for state in range(state_count):
            if counts[state] == 0:
                continue
            centroids[state] = [value / counts[state] for value in buckets[state]]
    for idx in range(len(observations)):
        assignments[idx] = nearest_centroid(observations[idx], centroids)
    return assignments


def nearest_centroid(vector: list[float], centroids: list[list[float]]) -> int:
    best_state = 0
    best_distance = float("inf")
    for state, centroid in enumerate(centroids):
        distance = sum((value - center) ** 2 for value, center in zip(vector, centroid))
        if distance < best_distance:
            best_distance = distance
            best_state = state
    return best_state


def fit_hmm_cluster_model(
    observations: list[list[float]],
    train_indices: list[int],
    assignments: list[int],
    state_count: int,
) -> HMMClusterModel:
    dim = len(HMM_VECTOR_FEATURES)
    means = [[0.0] * dim for _ in range(state_count)]
    counts = [0] * state_count
    for idx in train_indices:
        state = assignments[idx]
        counts[state] += 1
        for feature_idx, value in enumerate(observations[idx]):
            means[state][feature_idx] += value
    for state in range(state_count):
        denom = max(1, counts[state])
        means[state] = [value / denom for value in means[state]]

    variances = [[0.25] * dim for _ in range(state_count)]
    sums = [[0.0] * dim for _ in range(state_count)]
    for idx in train_indices:
        state = assignments[idx]
        for feature_idx, value in enumerate(observations[idx]):
            diff = value - means[state][feature_idx]
            sums[state][feature_idx] += diff * diff
    for state in range(state_count):
        denom = max(1, counts[state] - 1)
        variances[state] = [max(0.20, value / denom) for value in sums[state]]

    transition_counts = [[1.0 for _ in range(state_count)] for _ in range(state_count)]
    for prev_idx, idx in zip(train_indices, train_indices[1:]):
        if idx == prev_idx + 1:
            transition_counts[assignments[prev_idx]][assignments[idx]] += 1.0
    transition = []
    for row in transition_counts:
        total = sum(row)
        transition.append([value / total for value in row])

    start_counts = [1.0 for _ in range(state_count)]
    for idx in train_indices[: min(200, len(train_indices))]:
        start_counts[assignments[idx]] += 1.0
    start_total = sum(start_counts)
    start = [value / start_total for value in start_counts]
    state_labels = hmm_state_labels(means, counts)
    return HMMClusterModel(means, variances, transition, start, state_labels)


def hmm_state_labels(means: list[list[float]], counts: list[int]) -> dict[int, str]:
    idx = {name: pos for pos, name in enumerate(HMM_VECTOR_FEATURES)}
    scored: list[tuple[float, int, str]] = []
    for state, vector in enumerate(means):
        range_z = vector[idx["range_atr"]]
        bb_z = vector[idx["bb_width_ratio"]]
        adx_z = vector[idx["adx"]]
        chop_z = vector[idx["chop20"]]
        mean_revert_z = vector[idx["mean_reversion_pressure"]]
        sweep_z = vector[idx["sweep_displacement_score"]]
        fvg_z = vector[idx["fvg_abs_atr"]]
        volume_z = vector[idx["volume_z50"]]
        slope_abs_z = abs(vector[idx["ema21_slope_atr"]])
        efficiency_z = vector[idx["prior_efficiency20"]]
        pctb_edge_z = vector[idx["bb_pctb_extreme"]]
        premium_edge_z = vector[idx["premium_discount_edge"]]
        label_scores = {
            "manipulation": 1.25 * sweep_z + 0.70 * fvg_z + 0.30 * volume_z + 0.20 * premium_edge_z,
            "compression": -0.90 * range_z - 0.85 * bb_z + 0.65 * chop_z - 0.25 * adx_z,
            "trend_continuation": 0.80 * adx_z + 0.70 * efficiency_z + 0.55 * slope_abs_z - 0.35 * chop_z,
            "reversion": 0.95 * mean_revert_z + 0.45 * premium_edge_z + 0.35 * pctb_edge_z - 0.20 * adx_z,
            "expansion": 0.85 * range_z + 0.65 * bb_z + 0.35 * volume_z + 0.25 * fvg_z,
        }
        for label, score in label_scores.items():
            scored.append((score + math.log(max(1, counts[state])), state, label))

    out: dict[int, str] = {}
    used_labels: set[str] = set()
    for _, state, label in sorted(scored, reverse=True):
        if state in out or label in used_labels:
            continue
        out[state] = label
        used_labels.add(label)
    for state in range(len(means)):
        if state in out:
            continue
        best = max((item for item in scored if item[1] == state), default=(0.0, state, "unknown"))
        out[state] = best[2]
    return out


def viterbi_decode(
    observations: list[list[float]],
    usable: list[bool],
    model: HMMClusterModel,
) -> list[int]:
    positions = [idx for idx, ok in enumerate(usable) if ok]
    if not positions:
        return [-1] * len(observations)
    state_count = len(model.means)
    log_transition = [
        [math.log(max(value, 1e-12)) for value in row]
        for row in model.transition
    ]
    log_start = [math.log(max(value, 1e-12)) for value in model.start]
    backpointers: list[list[int]] = []
    prev_scores: list[float] = []
    for pos_idx, obs_idx in enumerate(positions):
        emissions = [gaussian_logp(observations[obs_idx], model.means[state], model.variances[state]) for state in range(state_count)]
        current_scores: list[float] = []
        current_back: list[int] = []
        for state in range(state_count):
            if pos_idx == 0:
                current_scores.append(log_start[state] + emissions[state])
                current_back.append(0)
                continue
            best_prev = 0
            best_score = -float("inf")
            for prev_state in range(state_count):
                score = prev_scores[prev_state] + log_transition[prev_state][state]
                if score > best_score:
                    best_score = score
                    best_prev = prev_state
            current_scores.append(best_score + emissions[state])
            current_back.append(best_prev)
        prev_scores = current_scores
        backpointers.append(current_back)

    decoded_positions = [0] * len(positions)
    state = max(range(state_count), key=lambda item: prev_scores[item])
    for pos_idx in range(len(positions) - 1, -1, -1):
        decoded_positions[pos_idx] = state
        state = backpointers[pos_idx][state]
    decoded = [-1] * len(observations)
    for idx, state in zip(positions, decoded_positions):
        decoded[idx] = state
    return decoded


def gaussian_logp(vector: list[float], mean: list[float], variance: list[float]) -> float:
    total = 0.0
    for value, mu, var in zip(vector, mean, variance):
        safe_var = max(var, 1e-6)
        total += -0.5 * (math.log(2.0 * math.pi * safe_var) + ((value - mu) ** 2) / safe_var)
    return total


def sign(value: float) -> float:
    if value > 0:
        return 1.0
    if value < 0:
        return -1.0
    return 0.0


def ema(values: list[float], period: int) -> list[float]:
    if not values:
        return []
    alpha = 2.0 / (period + 1.0)
    out = [values[0]]
    for value in values[1:]:
        out.append(alpha * value + (1.0 - alpha) * out[-1])
    return out


def atr(candles: list[Candle], period: int = 14) -> list[float]:
    trs: list[float] = []
    prev_close = candles[0].close if candles else 0.0
    for candle in candles:
        tr = max(
            candle.high - candle.low,
            abs(candle.high - prev_close),
            abs(candle.low - prev_close),
        )
        trs.append(max(0.0, tr))
        prev_close = candle.close
    return rolling_mean(trs, period, default=0.0)


def rsi(candles: list[Candle], period: int = 14) -> list[float]:
    if not candles:
        return []
    gains = [0.0]
    losses = [0.0]
    for prev, cur in zip(candles, candles[1:]):
        delta = cur.close - prev.close
        gains.append(max(delta, 0.0))
        losses.append(max(-delta, 0.0))
    avg_gain = rolling_mean(gains, period, default=0.0)
    avg_loss = rolling_mean(losses, period, default=0.0)
    out = []
    for gain, loss in zip(avg_gain, avg_loss):
        if loss <= 1e-12:
            out.append(100.0 if gain > 0.0 else 50.0)
        else:
            rs = gain / loss
            out.append(100.0 - (100.0 / (1.0 + rs)))
    return out


def rolling_std(values: list[float], period: int, default: float) -> list[float]:
    out: list[float] = []
    running = 0.0
    running_sq = 0.0
    for idx, value in enumerate(values):
        running += value
        running_sq += value * value
        if idx >= period:
            old = values[idx - period]
            running -= old
            running_sq -= old * old
        if idx + 1 < period:
            out.append(default)
        else:
            mean = running / period
            variance = max(0.0, running_sq / period - mean * mean)
            out.append(math.sqrt(variance))
    return out


def rolling_mean(values: list[float], period: int, default: float) -> list[float]:
    out: list[float] = []
    running = 0.0
    for idx, value in enumerate(values):
        running += value
        if idx >= period:
            running -= values[idx - period]
        if idx + 1 < period:
            out.append(default)
        else:
            out.append(running / period)
    return out


def rolling_mean_finite(values: list[float], period: int, default: float) -> list[float]:
    out: list[float] = []
    for idx in range(len(values)):
        if idx + 1 < period:
            out.append(default)
            continue
        window = values[idx + 1 - period : idx + 1]
        if not all(math.isfinite(value) for value in window):
            out.append(default)
        else:
            out.append(sum(window) / period)
    return out


def safe_z_list(values: Iterable[float], means: Iterable[float], stds: Iterable[float]) -> list[float]:
    out = []
    for value, mean, std in zip(values, means, stds):
        if math.isfinite(mean) and math.isfinite(std) and std > 1e-12:
            out.append((value - mean) / std)
        else:
            out.append(float("nan"))
    return out


def safe_ratio_list(values: Iterable[float], bases: Iterable[float]) -> list[float]:
    out = []
    for value, base in zip(values, bases):
        if math.isfinite(base) and abs(base) > 1e-12:
            out.append(value / base)
        else:
            out.append(float("nan"))
    return out


def stochastic_k(candles: list[Candle], period: int = 14) -> list[float]:
    high = [c.high for c in candles]
    low = [c.low for c in candles]
    close = [c.close for c in candles]
    high_n = rolling_max(high, period)
    low_n = rolling_min(low, period)
    out = []
    for idx, value in enumerate(close):
        denom = high_n[idx] - low_n[idx]
        if idx + 1 < period or abs(denom) <= 1e-12:
            out.append(float("nan"))
        else:
            out.append(100.0 * (value - low_n[idx]) / denom)
    return out


def cci(candles: list[Candle], period: int = 20) -> list[float]:
    typical = [(c.high + c.low + c.close) / 3.0 for c in candles]
    mean = rolling_mean(typical, period, default=float("nan"))
    out = []
    for idx, value in enumerate(typical):
        if idx + 1 < period or not math.isfinite(mean[idx]):
            out.append(float("nan"))
            continue
        window = typical[idx + 1 - period : idx + 1]
        mad = sum(abs(item - mean[idx]) for item in window) / period
        out.append((value - mean[idx]) / (0.015 * mad) if mad > 1e-12 else 0.0)
    return out


def adx(candles: list[Candle], period: int = 14) -> list[float]:
    if not candles:
        return []
    plus_dm = [0.0]
    minus_dm = [0.0]
    true_ranges = [max(0.0, candles[0].high - candles[0].low)]
    for prev, cur in zip(candles, candles[1:]):
        up_move = cur.high - prev.high
        down_move = prev.low - cur.low
        plus_dm.append(up_move if up_move > down_move and up_move > 0.0 else 0.0)
        minus_dm.append(down_move if down_move > up_move and down_move > 0.0 else 0.0)
        true_ranges.append(
            max(
                cur.high - cur.low,
                abs(cur.high - prev.close),
                abs(cur.low - prev.close),
            )
        )
    smoothed_tr = rolling_mean(true_ranges, period, default=float("nan"))
    smoothed_plus = rolling_mean(plus_dm, period, default=float("nan"))
    smoothed_minus = rolling_mean(minus_dm, period, default=float("nan"))
    dx = []
    for tr, plus, minus in zip(smoothed_tr, smoothed_plus, smoothed_minus):
        if not all(math.isfinite(item) for item in [tr, plus, minus]) or tr <= 1e-12:
            dx.append(float("nan"))
            continue
        plus_di = 100.0 * plus / tr
        minus_di = 100.0 * minus / tr
        denom = plus_di + minus_di
        dx.append(100.0 * abs(plus_di - minus_di) / denom if denom > 1e-12 else 0.0)
    return rolling_mean_finite(dx, period, default=float("nan"))


def pda_proxy_features(
    candles: list[Candle],
    atr_values: list[float],
    volume_z50: list[float],
) -> dict[str, list[float]]:
    bull_fvg_gap_atr: list[float] = []
    bear_fvg_gap_atr: list[float] = []
    fvg_abs_atr: list[float] = []
    order_block_touch_score: list[float] = []
    order_block_age_score: list[float] = []
    breaker_score: list[float] = []
    breaker_continuation_score: list[float] = []
    sweep_displacement_score: list[float] = []
    sweep_reversal_score: list[float] = []
    sweep_continuation_score: list[float] = []
    fvg_mitigation_score: list[float] = []
    fvg_failed_mitigation_score: list[float] = []
    premium_discount_50: list[float] = []
    ob_post_mitigation_score: list[float] = []
    engulfing_score: list[float] = []
    pin_rejection_score: list[float] = []
    propulsion_score: list[float] = []

    bull_zone: tuple[float, float, int, float] | None = None
    bear_zone: tuple[float, float, int, float] | None = None
    bull_fvg_zone: tuple[float, float, int, float] | None = None
    bear_fvg_zone: tuple[float, float, int, float] | None = None
    for idx, candle in enumerate(candles):
        atr_den = max(atr_values[idx], 1e-12)
        bar_range = max(0.0, candle.high - candle.low)
        body = abs(candle.close - candle.open)
        body_frac = body / max(bar_range, 1e-12)
        vol_z = volume_z50[idx] if math.isfinite(volume_z50[idx]) else 0.0

        if idx >= 2:
            bull_gap = max(0.0, candle.low - candles[idx - 2].high) / atr_den
            bear_gap = max(0.0, candles[idx - 2].low - candle.high) / atr_den
        else:
            bull_gap = float("nan")
            bear_gap = float("nan")
        bull_fvg_gap_atr.append(bull_gap)
        bear_fvg_gap_atr.append(bear_gap)
        fvg_abs_atr.append(max(bull_gap, bear_gap) if math.isfinite(bull_gap) else float("nan"))

        touch = 0.0
        age_score = 0.0
        breaker = 0.0
        breaker_continuation = 0.0
        post_mitigation = 0.0
        if bull_zone is not None:
            low, high, zone_idx, score = bull_zone
            age = idx - zone_idx
            if age <= 60:
                decay = max(0.0, 1.0 - age / 60.0)
                age_score = max(age_score, decay * score)
                if candle.low <= high and candle.close >= low:
                    touch = max(touch, decay * score)
                    if candle.close > candle.open and (idx == 0 or candle.close > candles[idx - 1].close):
                        post_mitigation = max(post_mitigation, decay * score)
                if candle.close < low - 0.20 * atr_den:
                    breaker = max(breaker, decay * score)
                    if candle.close < candle.open and body_frac >= 0.45:
                        breaker_continuation = max(breaker_continuation, decay * score)
        if bear_zone is not None:
            low, high, zone_idx, score = bear_zone
            age = idx - zone_idx
            if age <= 60:
                decay = max(0.0, 1.0 - age / 60.0)
                age_score = max(age_score, decay * score)
                if candle.high >= low and candle.close <= high:
                    touch = max(touch, decay * score)
                    if candle.close < candle.open and (idx == 0 or candle.close < candles[idx - 1].close):
                        post_mitigation = max(post_mitigation, decay * score)
                if candle.close > high + 0.20 * atr_den:
                    breaker = max(breaker, decay * score)
                    if candle.close > candle.open and body_frac >= 0.45:
                        breaker_continuation = max(breaker_continuation, decay * score)
        order_block_touch_score.append(touch)
        order_block_age_score.append(age_score)
        breaker_score.append(breaker)
        breaker_continuation_score.append(breaker_continuation)
        ob_post_mitigation_score.append(post_mitigation)

        fvg_mitigation = 0.0
        fvg_failed = 0.0
        if bull_fvg_zone is not None:
            lower, upper, zone_idx, score = bull_fvg_zone
            age = idx - zone_idx
            if age <= 80:
                decay = max(0.0, 1.0 - age / 80.0)
                if candle.low <= upper and candle.close >= lower:
                    fvg_mitigation = max(fvg_mitigation, decay * score)
                if candle.close < lower - 0.15 * atr_den:
                    fvg_failed = max(fvg_failed, decay * score)
        if bear_fvg_zone is not None:
            lower, upper, zone_idx, score = bear_fvg_zone
            age = idx - zone_idx
            if age <= 80:
                decay = max(0.0, 1.0 - age / 80.0)
                if candle.high >= lower and candle.close <= upper:
                    fvg_mitigation = max(fvg_mitigation, decay * score)
                if candle.close > upper + 0.15 * atr_den:
                    fvg_failed = max(fvg_failed, decay * score)
        fvg_mitigation_score.append(fvg_mitigation)
        fvg_failed_mitigation_score.append(fvg_failed)

        if idx >= 20:
            prior = candles[idx - 20 : idx]
            prior_high = max(item.high for item in prior)
            prior_low = min(item.low for item in prior)
            swept_high = candle.high > prior_high
            swept_low = candle.low < prior_low
            closes_down_from_high = candle.close < candle.high - 0.65 * bar_range
            closes_up_from_low = candle.close > candle.low + 0.65 * bar_range
            continuation_up = swept_high and candle.close > prior_high + 0.15 * atr_den and body_frac >= 0.45
            continuation_down = swept_low and candle.close < prior_low - 0.15 * atr_den and body_frac >= 0.45
            if swept_high and closes_down_from_high:
                sweep_score = (bar_range / atr_den) * max(0.5, body_frac) * (1.0 + max(0.0, vol_z) / 4.0)
                reversal_score = sweep_score
                continuation_score = 0.0
            elif swept_low and closes_up_from_low:
                sweep_score = (bar_range / atr_den) * max(0.5, body_frac) * (1.0 + max(0.0, vol_z) / 4.0)
                reversal_score = sweep_score
                continuation_score = 0.0
            elif continuation_up or continuation_down:
                continuation_score = (bar_range / atr_den) * max(0.5, body_frac) * (1.0 + max(0.0, vol_z) / 4.0)
                sweep_score = continuation_score
                reversal_score = 0.0
            else:
                sweep_score = 0.0
                reversal_score = 0.0
                continuation_score = 0.0
        else:
            sweep_score = float("nan")
            reversal_score = float("nan")
            continuation_score = float("nan")
        sweep_displacement_score.append(sweep_score)
        sweep_reversal_score.append(reversal_score)
        sweep_continuation_score.append(continuation_score)

        if idx >= 50:
            recent = candles[idx - 50 : idx + 1]
            range_high = max(item.high for item in recent)
            range_low = min(item.low for item in recent)
            premium_discount_50.append((candle.close - range_low) / max(range_high - range_low, 1e-12))
        else:
            premium_discount_50.append(float("nan"))

        if idx >= 1:
            prev = candles[idx - 1]
            curr_dir = sign(candle.close - candle.open)
            prev_dir = sign(prev.close - prev.open)
            engulfed = candle.high >= prev.high and candle.low <= prev.low and curr_dir == -prev_dir and curr_dir != 0.0
            engulfing_score.append(body_frac * (bar_range / atr_den) if engulfed else 0.0)
        else:
            engulfing_score.append(float("nan"))

        upper_wick = candle.high - max(candle.open, candle.close)
        lower_wick = min(candle.open, candle.close) - candle.low
        pin_score = max(upper_wick, lower_wick) / max(bar_range, 1e-12)
        pin_rejection_score.append(pin_score if pin_score >= 0.55 else 0.0)
        propulsion_score.append((bar_range / atr_den) * body_frac * (1.0 + max(0.0, vol_z) / 3.0))

        if idx >= 1:
            prev = candles[idx - 1]
            displaced = bar_range / atr_den > 1.10 and body_frac > 0.55 and vol_z > 0.20
            if displaced and prev.close < prev.open and candle.close > prev.high:
                bull_zone = (
                    min(prev.open, prev.close, prev.low),
                    max(prev.open, prev.close, prev.high),
                    idx - 1,
                    min(2.0, (bar_range / atr_den) * (1.0 + vol_z / 4.0)),
                )

        if math.isfinite(bull_gap) and bull_gap > 0.0:
            bull_fvg_zone = (
                candles[idx - 2].high,
                candle.low,
                idx,
                min(2.0, bull_gap * (1.0 + max(0.0, vol_z) / 4.0)),
            )
        if math.isfinite(bear_gap) and bear_gap > 0.0:
            bear_fvg_zone = (
                candle.high,
                candles[idx - 2].low,
                idx,
                min(2.0, bear_gap * (1.0 + max(0.0, vol_z) / 4.0)),
            )
            if displaced and prev.close > prev.open and candle.close < prev.low:
                bear_zone = (
                    min(prev.open, prev.close, prev.low),
                    max(prev.open, prev.close, prev.high),
                    idx - 1,
                    min(2.0, (bar_range / atr_den) * (1.0 + vol_z / 4.0)),
                )

    return {
        "bull_fvg_gap_atr": bull_fvg_gap_atr,
        "bear_fvg_gap_atr": bear_fvg_gap_atr,
        "fvg_abs_atr": fvg_abs_atr,
        "order_block_touch_score": order_block_touch_score,
        "order_block_age_score": order_block_age_score,
        "breaker_score": breaker_score,
        "breaker_continuation_score": breaker_continuation_score,
        "sweep_displacement_score": sweep_displacement_score,
        "sweep_reversal_score": sweep_reversal_score,
        "sweep_continuation_score": sweep_continuation_score,
        "fvg_mitigation_score": fvg_mitigation_score,
        "fvg_failed_mitigation_score": fvg_failed_mitigation_score,
        "premium_discount_50": premium_discount_50,
        "ob_post_mitigation_score": ob_post_mitigation_score,
        "engulfing_score": engulfing_score,
        "pin_rejection_score": pin_rejection_score,
        "propulsion_score": propulsion_score,
    }


def rolling_quantile(values: list[float], period: int, q: float) -> list[float]:
    out: list[float] = []
    window: list[float] = []
    for idx, value in enumerate(values):
        bisect.insort(window, value)
        if idx >= period:
            old = values[idx - period]
            del window[bisect.bisect_left(window, old)]
        if idx + 1 < period:
            out.append(float("nan"))
        else:
            pos = int(round((len(window) - 1) * q))
            out.append(window[pos])
    return out


def build_features(candles: list[Candle]) -> dict[str, list[float] | list[bool]]:
    close = [c.close for c in candles]
    high = [c.high for c in candles]
    low = [c.low for c in candles]
    volume = [c.volume for c in candles]
    bar_range = [max(0.0, c.high - c.low) for c in candles]
    body = [abs(c.close - c.open) for c in candles]
    atr14 = atr(candles, 14)
    atr_pct = [a / max(c.close, 1e-12) for a, c in zip(atr14, candles)]
    range_mean20 = shift(rolling_mean(bar_range, 20, default=float("nan")), 1)
    volume_mean20 = shift(rolling_mean(volume, 20, default=float("nan")), 1)
    volume_std20 = shift(rolling_std(volume, 20, default=float("nan")), 1)
    volume_mean50 = shift(rolling_mean(volume, 50, default=float("nan")), 1)
    volume_std50 = shift(rolling_std(volume, 50, default=float("nan")), 1)
    volume_z20 = safe_z_list(volume, volume_mean20, volume_std20)
    volume_z50 = safe_z_list(volume, volume_mean50, volume_std50)
    rel_volume20 = safe_ratio_list(volume, volume_mean20)
    rel_volume50 = safe_ratio_list(volume, volume_mean50)
    volume_ema10 = ema(volume, 10)
    volume_ema30 = ema(volume, 30)
    volume_trend = safe_div_list(
        [fast - slow for fast, slow in zip(volume_ema10, volume_ema30)],
        volume_mean50,
    )
    obv = []
    running_obv = 0.0
    prev_close = close[0] if close else 0.0
    for idx, candle in enumerate(candles):
        if idx > 0:
            running_obv += sign(candle.close - prev_close) * candle.volume
        obv.append(running_obv)
        prev_close = candle.close
    obv_slope10 = safe_div_list(
        [value - obv[max(0, idx - 10)] for idx, value in enumerate(obv)],
        [max(sum(volume[max(0, idx - 10) : idx + 1]), 1e-12) for idx in range(len(volume))],
    )
    ema13 = ema(close, 13)
    ema21 = ema(close, 21)
    ema34 = ema(close, 34)
    ema55 = ema(close, 55)
    ema89 = ema(close, 89)
    rsi14 = rsi(candles, 14)
    ema12 = ema(close, 12)
    ema26 = ema(close, 26)
    macd_line = [fast - slow for fast, slow in zip(ema12, ema26)]
    macd_signal = ema(macd_line, 9)
    macd_hist = [line - signal for line, signal in zip(macd_line, macd_signal)]
    macd_hist_atr = safe_div_list(macd_hist, atr14)
    macd_hist_slope = [
        (value - macd_hist[max(0, idx - 3)]) / max(atr14[idx], 1e-12)
        for idx, value in enumerate(macd_hist)
    ]
    high_3 = rolling_max(high, 3)
    high_6 = rolling_max(high, 6)
    high_10 = rolling_max(high, 10)
    high_20 = rolling_max(high, 20)
    high_50 = rolling_max(high, 50)
    low_10 = rolling_min(low, 10)
    low_20 = rolling_min(low, 20)
    low_50 = rolling_min(low, 50)
    donchian_high20 = shift(high_20, 1)
    donchian_low20 = shift(low_20, 1)
    donchian_width_atr = safe_div_list(
        [h - l for h, l in zip(donchian_high20, donchian_low20)],
        atr14,
    )
    bb_mean20 = rolling_mean(close, 20, default=float("nan"))
    bb_std20 = rolling_std(close, 20, default=float("nan"))
    width = [
        (4.0 * std / mean) if math.isfinite(mean) and abs(mean) > 1e-12 else float("nan")
        for mean, std in zip(bb_mean20, bb_std20)
    ]
    bb_pctb = []
    for value, mean, std in zip(close, bb_mean20, bb_std20):
        if math.isfinite(mean) and math.isfinite(std) and std > 1e-12:
            lower = mean - 2.0 * std
            upper = mean + 2.0 * std
            bb_pctb.append((value - lower) / max(upper - lower, 1e-12))
        else:
            bb_pctb.append(float("nan"))
    bb_width_change5 = [
        width[idx] - width[max(0, idx - 5)] if math.isfinite(width[idx]) and math.isfinite(width[max(0, idx - 5)]) else float("nan")
        for idx in range(len(width))
    ]
    bb_width_p70 = rolling_quantile(width, 120, 0.70)
    bb_width_p20 = rolling_quantile(width, 120, 0.20)
    bb_width_ratio = safe_ratio_list(width, bb_width_p70)
    keltner_pos = [
        (candle.close - mid) / max(1.5 * atr_value, 1e-12)
        for candle, mid, atr_value in zip(candles, ema21, atr14)
    ]
    stoch14 = stochastic_k(candles, 14)
    stoch_d = rolling_mean_finite(stoch14, 3, default=float("nan"))
    cci20 = cci(candles, 20)
    adx14 = adx(candles, 14)
    adx_slope = [
        value - adx14[max(0, idx - 5)] if math.isfinite(value) and math.isfinite(adx14[max(0, idx - 5)]) else float("nan")
        for idx, value in enumerate(adx14)
    ]
    pda = pda_proxy_features(candles, atr14, volume_z50)
    features: dict[str, list[float] | list[bool]] = {
        "close": close,
        "range": bar_range,
        "range_mean20": range_mean20,
        "body": body,
        "atr": atr14,
        "atr_pct": atr_pct,
        "atr_p50": rolling_quantile(atr_pct, 120, 0.50),
        "atr_p65": rolling_quantile(atr_pct, 160, 0.65),
        "volume": volume,
        "volume_mean20": volume_mean20,
        "volume_z20": volume_z20,
        "volume_z50": volume_z50,
        "rel_volume20": rel_volume20,
        "rel_volume50": rel_volume50,
        "volume_trend": volume_trend,
        "obv_slope10": obv_slope10,
        "ema13": ema13,
        "ema21": ema21,
        "ema34": ema34,
        "ema55": ema55,
        "ema89": ema89,
        "rsi": rsi14,
        "macd_hist": macd_hist,
        "macd_hist_atr": macd_hist_atr,
        "macd_hist_slope": macd_hist_slope,
        "high_3": shift(high_3, 1),
        "high_6": shift(high_6, 1),
        "high_10": shift(high_10, 1),
        "high_20": donchian_high20,
        "high_50": shift(high_50, 1),
        "low_10": shift(low_10, 1),
        "low_20": donchian_low20,
        "low_50": shift(low_50, 1),
        "donchian_width_atr": donchian_width_atr,
        "bb_width": width,
        "bb_pctb": bb_pctb,
        "bb_width_change5": bb_width_change5,
        "bb_width_p70": bb_width_p70,
        "bb_width_p20": bb_width_p20,
        "bb_width_ratio": bb_width_ratio,
        "keltner_pos": keltner_pos,
        "stoch_k": stoch14,
        "stoch_d": stoch_d,
        "cci": cci20,
        "adx": adx14,
        "adx_slope": adx_slope,
        "ema_gap": safe_div_list([a - b for a, b in zip(ema21, ema89)], atr14),
    }
    features.update(pda)
    return features


def rolling_max(values: list[float], period: int) -> list[float]:
    out = []
    for idx in range(len(values)):
        start = max(0, idx + 1 - period)
        out.append(max(values[start : idx + 1]))
    return out


def rolling_min(values: list[float], period: int) -> list[float]:
    out = []
    for idx in range(len(values)):
        start = max(0, idx + 1 - period)
        out.append(min(values[start : idx + 1]))
    return out


def shift(values: list[float], periods: int) -> list[float]:
    return [float("nan")] * periods + values[: max(0, len(values) - periods)]


def safe_div_list(num: Iterable[float], den: Iterable[float]) -> list[float]:
    return [n / d if abs(d) > 1e-12 else 0.0 for n, d in zip(num, den)]


def bollinger_width(values: list[float], period: int) -> list[float]:
    out = []
    for idx in range(len(values)):
        if idx + 1 < period:
            out.append(float("nan"))
            continue
        window = values[idx + 1 - period : idx + 1]
        mean = sum(window) / period
        variance = sum((item - mean) ** 2 for item in window) / period
        std = math.sqrt(variance)
        out.append((4.0 * std) / mean if abs(mean) > 1e-12 else float("nan"))
    return out


def pred_unknown() -> FactorPrediction:
    return FactorPrediction("unknown", 0.0)


def build_factor_functions(
    candles: list[Candle],
    features: dict[str, list[float] | list[bool]],
) -> dict[str, Callable[[int], FactorPrediction]]:
    def valid(idx: int, *names: str) -> bool:
        for name in names:
            value = features[name][idx]  # type: ignore[index]
            if isinstance(value, float) and not math.isfinite(value):
                return False
        return True

    def manipulation_sweep_reject(idx: int) -> FactorPrediction:
        if idx < 10 or not valid(idx, "high_10", "low_10"):
            return pred_unknown()
        candle = candles[idx]
        bar_range = max(0.0, candle.high - candle.low)
        if bar_range <= 0.0:
            return pred_unknown()
        swept_high = candle.high > features["high_10"][idx] and candle.close < (
            candle.high - 0.6 * bar_range
        )
        swept_low = candle.low < features["low_10"][idx] and candle.close > (
            candle.low + 0.6 * bar_range
        )
        if swept_high or swept_low:
            return FactorPrediction("manipulation", min(1.0, bar_range / max(features["atr"][idx], 1e-12)))
        return pred_unknown()

    def compression_range_contract(idx: int) -> FactorPrediction:
        if idx < 10:
            return pred_unknown()
        avg_range = mean_range(candles[idx - 10 : idx])
        bar_range = max(0.0, candles[idx].high - candles[idx].low)
        if avg_range > 0.0 and bar_range < 0.5 * avg_range:
            return FactorPrediction("compression", max(0.0, 1.0 - bar_range / (0.5 * avg_range)))
        return pred_unknown()

    def expansion_body_range(idx: int) -> FactorPrediction:
        if idx < 10:
            return pred_unknown()
        avg_range = mean_range(candles[idx - 10 : idx])
        candle = candles[idx]
        bar_range = max(0.0, candle.high - candle.low)
        body = abs(candle.close - candle.open)
        if avg_range > 0.0 and bar_range > 1.5 * avg_range and body > 0.6 * bar_range:
            return FactorPrediction("expansion", min(1.0, bar_range / (1.5 * avg_range)))
        return pred_unknown()

    def trend_continuation_body(idx: int) -> FactorPrediction:
        if idx < 10:
            return pred_unknown()
        candle = candles[idx]
        prev = candles[idx - 1]
        bar_range = max(0.0, candle.high - candle.low)
        body = abs(candle.close - candle.open)
        curr_dir = sign(candle.close - candle.open)
        prev_dir = sign(prev.close - prev.open)
        if curr_dir != 0.0 and curr_dir == prev_dir and body > 0.5 * bar_range:
            return FactorPrediction("trend_continuation", body / max(bar_range, 1e-12))
        return pred_unknown()

    def reversion_mean_reclaim(idx: int) -> FactorPrediction:
        if idx < 10:
            return pred_unknown()
        candle = candles[idx]
        prev = candles[idx - 1]
        mean_close = sum(c.close for c in candles[idx - 10 : idx]) / 10.0
        curr_dir = sign(candle.close - candle.open)
        prev_dir = sign(prev.close - prev.open)
        before = abs(candle.open - mean_close)
        after = abs(candle.close - mean_close)
        if curr_dir != 0.0 and curr_dir != prev_dir and after < before:
            return FactorPrediction("reversion", max(0.0, 1.0 - after / max(before, 1e-12)))
        return pred_unknown()

    def mece_rule_baseline(idx: int) -> FactorPrediction:
        # Same priority order as the repo's manual MECE labeler. This is a
        # white-box regime baseline, not a trading factor. It exists so the
        # loop can separate "regime classifier quality" from "entry quality".
        for fn in [
            manipulation_sweep_reject,
            compression_range_contract,
            expansion_body_range,
            trend_continuation_body,
            reversion_mean_reclaim,
        ]:
            pred = fn(idx)
            if pred.label != "unknown":
                return pred
        return pred_unknown()

    def compression_release_dense(idx: int) -> FactorPrediction:
        if idx < 120 or not valid(idx, "bb_width", "bb_width_p70", "high_3", "ema89", "rsi"):
            return pred_unknown()
        width = features["bb_width"][idx]
        width_gate = features["bb_width_p70"][idx]
        compressed_recently = min(features["bb_width"][max(0, idx - 18) : idx + 1]) < width_gate
        if width < width_gate * 0.85:
            return FactorPrediction("compression", 1.0 - width / max(width_gate, 1e-12))
        release = candles[idx].close > features["high_3"][idx] and candles[idx].close > features["ema89"][idx]
        if compressed_recently and release and features["rsi"][idx] <= 78:
            return FactorPrediction("expansion", min(1.0, width / max(width_gate, 1e-12)))
        return pred_unknown()

    def persistence_cluster_dense(idx: int) -> FactorPrediction:
        if idx < 90 or not valid(idx, "ema13", "ema34", "ema89", "rsi", "atr"):
            return pred_unknown()
        ema_stack = features["ema13"][idx] > features["ema34"][idx] > features["ema89"][idx]
        slope = (features["ema13"][idx] - features["ema13"][max(0, idx - 3)]) / max(
            features["atr"][idx], 1e-12
        )
        rsi_ok = 43 <= features["rsi"][idx] <= 78
        if ema_stack and slope > -0.12 and rsi_ok:
            return FactorPrediction("trend_continuation", min(1.0, (slope + 0.12) / 0.6))
        return pred_unknown()

    def trend_pullback_dense(idx: int) -> FactorPrediction:
        if idx < 90 or not valid(idx, "ema21", "ema55", "ema89", "rsi", "atr"):
            return pred_unknown()
        candle = candles[idx]
        local_trend = features["ema21"][idx] > features["ema89"][idx] and candle.close > features["ema89"][idx]
        near_ema = abs(candle.close - features["ema21"][idx]) / max(features["atr"][idx], 1e-12)
        if local_trend and near_ema <= 2.4 and 35 <= features["rsi"][idx] <= 74:
            if candle.close > candle.open or candle.close > candles[idx - 1].close:
                return FactorPrediction("trend_continuation", max(0.0, 1.0 - near_ema / 2.4))
            return FactorPrediction("reversion", max(0.0, 1.0 - near_ema / 2.4))
        return pred_unknown()

    def volatility_transition_wide(idx: int) -> FactorPrediction:
        if idx < 120 or not valid(idx, "atr_pct", "atr_p50", "high_6", "ema89", "rsi"):
            return pred_unknown()
        active = features["atr_pct"][idx] > features["atr_p50"][idx]
        breakout = candles[idx].close > features["high_6"][idx]
        if active and breakout and candles[idx].close > features["ema89"][idx] and features["rsi"][idx] <= 74:
            score = features["atr_pct"][idx] / max(features["atr_p50"][idx], 1e-12)
            return FactorPrediction("expansion", min(1.0, score - 1.0))
        return pred_unknown()

    def transition_hazard(idx: int) -> FactorPrediction:
        if idx < 120 or not valid(idx, "ema_gap", "high_6", "rsi"):
            return pred_unknown()
        prior = features["ema_gap"][max(0, idx - 8)]
        gap = features["ema_gap"][idx]
        slope = gap - features["ema_gap"][max(0, idx - 5)]
        if prior <= 0.15 and gap > 0.0 and slope > 0.12 and candles[idx].close > features["high_6"][idx]:
            return FactorPrediction("expansion", min(1.0, slope))
        return pred_unknown()

    def volume_climax_regime(idx: int) -> FactorPrediction:
        if idx < 80 or not valid(idx, "volume_z50", "rel_volume50", "range", "atr", "bb_width_ratio"):
            return pred_unknown()
        candle = candles[idx]
        atr_den = max(features["atr"][idx], 1e-12)
        bar_range = max(0.0, candle.high - candle.low)
        body_frac = abs(candle.close - candle.open) / max(bar_range, 1e-12)
        volume_z = features["volume_z50"][idx]
        rel_volume = features["rel_volume50"][idx]
        if volume_z >= 2.0 and bar_range / atr_den >= 1.15:
            score = min(1.0, (volume_z - 1.0) / 3.0)
            if body_frac >= 0.60:
                return FactorPrediction("expansion", score)
            return FactorPrediction("manipulation", score)
        if rel_volume <= 0.55 and features["bb_width_ratio"][idx] < 0.90 and bar_range / atr_den <= 0.85:
            return FactorPrediction("compression", min(1.0, 1.0 - rel_volume))
        return pred_unknown()

    def indicator_bollinger_volume_cycle(idx: int) -> FactorPrediction:
        if idx < 160 or not valid(
            idx,
            "bb_width_ratio",
            "bb_width_change5",
            "bb_pctb",
            "rel_volume20",
            "volume_z20",
            "macd_hist_slope",
            "adx",
            "high_20",
            "low_20",
        ):
            return pred_unknown()
        width_ratio = features["bb_width_ratio"][idx]
        width_change = features["bb_width_change5"][idx]
        rel_volume = features["rel_volume20"][idx]
        adx_value = features["adx"][idx]
        if width_ratio <= 0.72 and rel_volume <= 1.05 and adx_value <= 22.0:
            return FactorPrediction("compression", min(1.0, 1.0 - width_ratio))
        breakout_up = candles[idx].close > features["high_20"][idx]
        breakout_down = candles[idx].close < features["low_20"][idx]
        release = width_ratio >= 0.85 and width_change > 0.0 and rel_volume >= 1.10
        macd_confirms = abs(features["macd_hist_slope"][idx]) >= 0.03
        if release and (breakout_up or breakout_down) and macd_confirms:
            return FactorPrediction("expansion", min(1.0, width_ratio * rel_volume / 1.8))
        if adx_value <= 20.0 and rel_volume >= 1.15 and (
            features["bb_pctb"][idx] >= 0.96 or features["bb_pctb"][idx] <= 0.04
        ):
            return FactorPrediction("reversion", min(1.0, rel_volume / 2.0))
        return pred_unknown()

    def indicator_adx_donchian_macd(idx: int) -> FactorPrediction:
        if idx < 120 or not valid(
            idx,
            "adx",
            "adx_slope",
            "macd_hist_atr",
            "macd_hist_slope",
            "donchian_width_atr",
            "high_20",
            "low_20",
            "rel_volume20",
        ):
            return pred_unknown()
        breakout_up = candles[idx].close > features["high_20"][idx]
        breakout_down = candles[idx].close < features["low_20"][idx]
        trend_strength = features["adx"][idx]
        if trend_strength >= 24.0 and features["donchian_width_atr"][idx] >= 2.0:
            macd = features["macd_hist_atr"][idx]
            macd_slope = features["macd_hist_slope"][idx]
            if (breakout_up and macd > 0.0 and macd_slope >= -0.02) or (
                breakout_down and macd < 0.0 and macd_slope <= 0.02
            ):
                score = min(1.0, (trend_strength - 18.0) / 24.0)
                if features["rel_volume20"][idx] >= 1.05:
                    return FactorPrediction("expansion", score)
                return FactorPrediction("trend_continuation", score)
        if trend_strength <= 17.0 and abs(features["macd_hist_atr"][idx]) <= 0.08:
            return FactorPrediction("compression", min(1.0, (17.0 - trend_strength) / 17.0))
        return pred_unknown()

    def indicator_cci_stoch_reversion(idx: int) -> FactorPrediction:
        if idx < 80 or not valid(
            idx,
            "cci",
            "stoch_k",
            "bb_pctb",
            "adx",
            "volume_z20",
            "keltner_pos",
        ):
            return pred_unknown()
        cci_value = features["cci"][idx]
        stoch_value = features["stoch_k"][idx]
        stretched_high = cci_value >= 140.0 and stoch_value >= 86.0 and features["bb_pctb"][idx] >= 0.92
        stretched_low = cci_value <= -140.0 and stoch_value <= 14.0 and features["bb_pctb"][idx] <= 0.08
        range_like = features["adx"][idx] <= 24.0
        volume_confirms = features["volume_z20"][idx] >= -0.25
        keltner_extreme = abs(features["keltner_pos"][idx]) >= 0.85
        if range_like and volume_confirms and keltner_extreme and (stretched_high or stretched_low):
            score = min(1.0, (abs(cci_value) - 100.0) / 120.0)
            return FactorPrediction("reversion", score)
        return pred_unknown()

    def pda_fvg_displacement(idx: int) -> FactorPrediction:
        if idx < 80 or not valid(
            idx,
            "fvg_abs_atr",
            "bull_fvg_gap_atr",
            "bear_fvg_gap_atr",
            "propulsion_score",
            "volume_z50",
            "ema_gap",
        ):
            return pred_unknown()
        fvg = features["fvg_abs_atr"][idx]
        propulsion = features["propulsion_score"][idx]
        if fvg >= 0.18 and propulsion >= 0.80:
            score = min(1.0, fvg * propulsion / 1.25)
            return FactorPrediction("expansion", score)
        if fvg >= 0.10 and abs(features["ema_gap"][idx]) >= 0.30 and features["volume_z50"][idx] >= -0.50:
            return FactorPrediction("trend_continuation", min(1.0, fvg + abs(features["ema_gap"][idx]) / 2.5))
        return pred_unknown()

    def pda_order_block_mitigation(idx: int) -> FactorPrediction:
        if idx < 120 or not valid(
            idx,
            "order_block_touch_score",
            "order_block_age_score",
            "breaker_score",
            "ema_gap",
            "volume_z20",
            "pin_rejection_score",
        ):
            return pred_unknown()
        if features["breaker_score"][idx] >= 0.35:
            return FactorPrediction("manipulation", min(1.0, features["breaker_score"][idx]))
        touch = features["order_block_touch_score"][idx]
        if touch >= 0.25 and abs(features["ema_gap"][idx]) >= 0.20:
            if features["pin_rejection_score"][idx] >= 0.55 or features["volume_z20"][idx] >= 0.0:
                return FactorPrediction("trend_continuation", min(1.0, touch))
        if features["order_block_age_score"][idx] >= 0.30 and abs(features["ema_gap"][idx]) <= 0.10:
            return FactorPrediction("compression", min(1.0, features["order_block_age_score"][idx]))
        return pred_unknown()

    def pda_sweep_displacement(idx: int) -> FactorPrediction:
        if idx < 80 or not valid(
            idx,
            "sweep_displacement_score",
            "volume_z50",
            "engulfing_score",
            "pin_rejection_score",
            "premium_discount_50",
        ):
            return pred_unknown()
        sweep = features["sweep_displacement_score"][idx]
        if sweep >= 0.75 and features["volume_z50"][idx] >= 0.0:
            if features["engulfing_score"][idx] > 0.0 or features["pin_rejection_score"][idx] >= 0.55:
                return FactorPrediction("manipulation", min(1.0, sweep / 2.0))
        premium = features["premium_discount_50"][idx]
        if sweep >= 0.45 and (premium >= 0.88 or premium <= 0.12):
            return FactorPrediction("reversion", min(1.0, sweep / 1.5))
        return pred_unknown()

    def pda_deep_structure(idx: int) -> FactorPrediction:
        if idx < 120 or not valid(
            idx,
            "sweep_reversal_score",
            "sweep_continuation_score",
            "fvg_mitigation_score",
            "fvg_failed_mitigation_score",
            "ob_post_mitigation_score",
            "breaker_continuation_score",
            "volume_z50",
            "ema_gap",
        ):
            return pred_unknown()
        if features["sweep_reversal_score"][idx] >= 0.60:
            return FactorPrediction("manipulation", min(1.0, features["sweep_reversal_score"][idx] / 2.0))
        continuation = max(
            features["sweep_continuation_score"][idx],
            features["fvg_mitigation_score"][idx],
            features["ob_post_mitigation_score"][idx],
        )
        if continuation >= 0.30 and abs(features["ema_gap"][idx]) >= 0.15:
            if features["volume_z50"][idx] >= -0.50:
                return FactorPrediction("trend_continuation", min(1.0, continuation))
        failure = max(
            features["fvg_failed_mitigation_score"][idx],
            features["breaker_continuation_score"][idx],
        )
        if failure >= 0.30:
            return FactorPrediction("manipulation", min(1.0, failure))
        return pred_unknown()

    factors = {
        "mece_rule_baseline_v1": mece_rule_baseline,
        "manipulation_sweep_reject": manipulation_sweep_reject,
        "compression_range_contract": compression_range_contract,
        "expansion_body_range": expansion_body_range,
        "trend_continuation_body": trend_continuation_body,
        "reversion_mean_reclaim": reversion_mean_reclaim,
        "regime_compression_release_dense": compression_release_dense,
        "regime_persistence_cluster_dense": persistence_cluster_dense,
        "regime_trend_pullback_dense": trend_pullback_dense,
        "regime_volatility_transition_wide": volatility_transition_wide,
        "regime_transition_hazard": transition_hazard,
        "volume_climax_regime_v1": volume_climax_regime,
        "indicator_bollinger_volume_cycle_v1": indicator_bollinger_volume_cycle,
        "indicator_adx_donchian_macd_v1": indicator_adx_donchian_macd,
        "indicator_cci_stoch_reversion_v1": indicator_cci_stoch_reversion,
        "pda_fvg_displacement_v1": pda_fvg_displacement,
        "pda_order_block_mitigation_v1": pda_order_block_mitigation,
        "pda_sweep_displacement_v1": pda_sweep_displacement,
        "pda_deep_structure_v1": pda_deep_structure,
    }

    def hybrid_vote(idx: int) -> FactorPrediction:
        priority = [
            factors["manipulation_sweep_reject"],
            factors["regime_compression_release_dense"],
            factors["regime_volatility_transition_wide"],
            factors["regime_transition_hazard"],
            factors["volume_climax_regime_v1"],
            factors["indicator_bollinger_volume_cycle_v1"],
            factors["indicator_adx_donchian_macd_v1"],
            factors["pda_sweep_displacement_v1"],
            factors["pda_fvg_displacement_v1"],
            factors["pda_order_block_mitigation_v1"],
            factors["regime_persistence_cluster_dense"],
            factors["regime_trend_pullback_dense"],
            factors["indicator_cci_stoch_reversion_v1"],
        ]
        for fn in priority:
            pred = fn(idx)
            if pred.label != "unknown":
                return pred
        return pred_unknown()

    factors["hybrid_regime_vote_v1"] = hybrid_vote
    return factors


def filter_feature_vectors(
    vectors: dict[str, list[float]],
    feature_sets: list[str] | None,
) -> dict[str, list[float]]:
    selected = normalize_feature_sets(feature_sets)
    if not selected or "all" in selected:
        return vectors
    names: set[str] = set()
    for feature_set in selected:
        if feature_set in FEATURE_SET_ALIASES:
            names.update(FEATURE_SET_ALIASES[feature_set])
        elif feature_set == "htf":
            names.update(name for name in vectors if name.startswith(("4h_", "1d_")))
        elif feature_set == "pair":
            names.update(name for name in vectors if name.startswith("pair_"))
        else:
            raise ValueError(f"unsupported feature set: {feature_set}")
    return {name: values for name, values in vectors.items() if name in names}


def normalize_feature_sets(feature_sets: list[str] | None) -> list[str]:
    if not feature_sets:
        return ["all"]
    out = []
    for item in feature_sets:
        for part in item.split(","):
            value = part.strip().lower()
            if value:
                out.append(value)
    return out or ["all"]


def build_trained_factor_functions(
    candles: list[Candle],
    features: dict[str, list[float] | list[bool]],
    truth: list[str],
    train_end: int,
    extra_vectors: dict[str, list[float]] | None = None,
    extra_tree_count: int = 9,
    extra_tree_depth: int = 5,
    extra_tree_min_leaf: int = 160,
    extra_tree_max_samples: int | None = None,
    feature_sets: list[str] | None = None,
    include_stumps: bool = True,
    include_gaussian: bool = True,
) -> dict[str, Callable[[int], FactorPrediction]]:
    vectors = scalar_feature_vectors(candles, features)
    if extra_vectors:
        vectors.update(extra_vectors)
    vectors = filter_feature_vectors(vectors, feature_sets)
    stumps = fit_label_stumps(vectors, truth, train_end) if include_stumps else []
    gaussian_models = fit_gaussian_models(vectors, truth, train_end) if include_gaussian else []
    extra_trees = fit_extra_trees(
        vectors,
        truth,
        train_end,
        LABELS,
        tree_count=extra_tree_count,
        max_depth=extra_tree_depth,
        min_leaf=extra_tree_min_leaf,
        max_samples=extra_tree_max_samples,
    )
    family_truth = [regime_family(label) for label in truth]
    family_stumps = (
        fit_label_stumps(vectors, family_truth, train_end, REGIME_FAMILIES)
        if include_stumps
        else []
    )
    family_gaussian_models = (
        fit_gaussian_models(
            vectors,
            family_truth,
            train_end,
            REGIME_FAMILIES,
        )
        if include_gaussian
        else []
    )
    family_extra_trees = fit_extra_trees(
        vectors,
        family_truth,
        train_end,
        REGIME_FAMILIES,
        tree_count=extra_tree_count,
        max_depth=extra_tree_depth,
        min_leaf=extra_tree_min_leaf,
        max_samples=extra_tree_max_samples,
        seed=2718,
    )

    def trained_scorecard(idx: int) -> FactorPrediction:
        best: tuple[float, TrainedStump] | None = None
        for stump in stumps:
            value = vectors[stump.feature][idx]
            if not math.isfinite(value):
                continue
            if stump.direction == ">=":
                passed = value >= stump.threshold
                margin = value - stump.threshold
            else:
                passed = value <= stump.threshold
                margin = stump.threshold - value
            if not passed:
                continue
            score = stump.f1 + 0.05 * max(0.0, margin / (abs(stump.threshold) + 1e-9))
            if best is None or score > best[0]:
                best = (score, stump)
        if best is None:
            return pred_unknown()
        score, stump = best
        return FactorPrediction(stump.label, min(1.0, score))

    def trained_gaussian_nb(idx: int) -> FactorPrediction:
        best, second_best = gaussian_best_label(idx, vectors, gaussian_models)
        if best is None or best[1] == "unknown":
            return pred_unknown()
        gap = best[0] - (second_best if second_best is not None else best[0])
        confidence = 1.0 / (1.0 + math.exp(-max(-20.0, min(20.0, gap))))
        return FactorPrediction(best[1], confidence)

    def trained_extra_trees(idx: int) -> FactorPrediction:
        return extra_forest_predict(idx, extra_trees, vectors)
    trained_extra_trees.feature_usage = extra_tree_feature_usage(extra_trees)  # type: ignore[attr-defined]

    def trained_family_scorecard(idx: int) -> FactorPrediction:
        pred = trained_stump_prediction(idx, vectors, family_stumps)
        if pred is None or pred.label == "unknown":
            return pred_unknown()
        return FactorPrediction(label_for_regime_family(pred.label), pred.score)

    def trained_family_gaussian_nb(idx: int) -> FactorPrediction:
        best, second_best = gaussian_best_label(idx, vectors, family_gaussian_models)
        if best is None or best[1] == "unknown":
            return pred_unknown()
        gap = best[0] - (second_best if second_best is not None else best[0])
        confidence = 1.0 / (1.0 + math.exp(-max(-20.0, min(20.0, gap))))
        return FactorPrediction(label_for_regime_family(best[1]), confidence)

    def trained_family_extra_trees(idx: int) -> FactorPrediction:
        pred = extra_forest_predict(idx, family_extra_trees, vectors)
        if pred.label == "unknown":
            return pred_unknown()
        return FactorPrediction(label_for_regime_family(pred.label), pred.score)
    trained_family_extra_trees.feature_usage = extra_tree_feature_usage(family_extra_trees)  # type: ignore[attr-defined]

    out: dict[str, Callable[[int], FactorPrediction]] = {}
    if stumps:
        out["trained_scorecard_v1"] = trained_scorecard
    if gaussian_models:
        out["trained_gaussian_nb_v1"] = trained_gaussian_nb
    if extra_trees:
        out["trained_extra_trees_v1"] = trained_extra_trees
    if family_stumps:
        out["trained_family_scorecard_v1"] = trained_family_scorecard
    if family_gaussian_models:
        out["trained_family_gaussian_nb_v1"] = trained_family_gaussian_nb
    if family_extra_trees:
        out["trained_family_extra_trees_v1"] = trained_family_extra_trees
    return out


def trained_stump_prediction(
    idx: int,
    vectors: dict[str, list[float]],
    stumps: list[TrainedStump],
) -> FactorPrediction | None:
    best: tuple[float, TrainedStump] | None = None
    for stump in stumps:
        value = vectors[stump.feature][idx]
        if not math.isfinite(value):
            continue
        if stump.direction == ">=":
            passed = value >= stump.threshold
            margin = value - stump.threshold
        else:
            passed = value <= stump.threshold
            margin = stump.threshold - value
        if not passed:
            continue
        score = stump.f1 + 0.05 * max(0.0, margin / (abs(stump.threshold) + 1e-9))
        if best is None or score > best[0]:
            best = (score, stump)
    if best is None:
        return None
    score, stump = best
    return FactorPrediction(stump.label, min(1.0, score))


def gaussian_best_label(
    idx: int,
    vectors: dict[str, list[float]],
    models: list[GaussianLabelModel],
) -> tuple[tuple[float, str] | None, float | None]:
    best: tuple[float, str] | None = None
    second_best: float | None = None
    for model in models:
        logp = math.log(max(model.prior, 1e-12))
        used = 0
        for feature, values in vectors.items():
            value = values[idx]
            mean = model.means.get(feature)
            variance = model.variances.get(feature)
            if mean is None or variance is None or not math.isfinite(value):
                continue
            variance = max(variance, 1e-9)
            delta = value - mean
            logp += -0.5 * (math.log(variance) + (delta * delta / variance))
            used += 1
        if used == 0:
            continue
        if best is None or logp > best[0]:
            second_best = best[0] if best is not None else None
            best = (logp, model.label)
        elif second_best is None or logp > second_best:
            second_best = logp
    return best, second_best


def label_for_regime_family(family: str) -> str:
    if family == "trend":
        return "trend_continuation"
    if family == "range":
        return "reversion"
    if family == "transition":
        return "manipulation"
    return "unknown"


def scalar_feature_vectors(
    candles: list[Candle],
    features: dict[str, list[float] | list[bool]],
) -> dict[str, list[float]]:
    out = {
        "range_atr": [],
        "body_frac": [],
        "atr_pct": [],
        "bb_width": [],
        "bb_width_inv": [],
        "ema_gap": [],
        "abs_ema_gap": [],
        "rsi": [],
        "rsi_distance": [],
        "close_ema21_atr": [],
        "close_ema89_atr": [],
        "ema21_slope_atr": [],
        "mean_reclaim": [],
        "sweep_reject": [],
        "breakout_atr": [],
        "range_vs_mean20": [],
        "range_change5": [],
        "body_signed_atr": [],
        "close_pos_range": [],
        "upper_wick_frac": [],
        "lower_wick_frac": [],
        "prior_efficiency20": [],
        "chop20": [],
        "mean_distance_atr": [],
        "mean_reversion_pressure": [],
        "bb_width_ratio": [],
        "atr_pct_ratio": [],
        "ema_slope_change": [],
        "volume_z20": [],
        "volume_z50": [],
        "rel_volume20": [],
        "rel_volume50": [],
        "volume_trend": [],
        "obv_slope10": [],
        "bb_pctb": [],
        "bb_pctb_extreme": [],
        "bb_width_change5": [],
        "donchian_width_atr": [],
        "donchian_breakout_atr": [],
        "keltner_pos": [],
        "macd_hist_atr": [],
        "macd_hist_slope": [],
        "stoch_k": [],
        "stoch_d": [],
        "cci_scaled": [],
        "adx": [],
        "adx_slope": [],
        "bull_fvg_gap_atr": [],
        "bear_fvg_gap_atr": [],
        "fvg_abs_atr": [],
        "order_block_touch_score": [],
        "order_block_age_score": [],
        "breaker_score": [],
        "breaker_continuation_score": [],
        "sweep_displacement_score": [],
        "sweep_reversal_score": [],
        "sweep_continuation_score": [],
        "fvg_mitigation_score": [],
        "fvg_failed_mitigation_score": [],
        "premium_discount_50": [],
        "premium_discount_edge": [],
        "ob_post_mitigation_score": [],
        "engulfing_score": [],
        "pin_rejection_score": [],
        "propulsion_score": [],
    }
    for idx, candle in enumerate(candles):
        atr_value = float(features["atr"][idx])  # type: ignore[index]
        atr_den = max(atr_value, 1e-12)
        bar_range = max(0.0, candle.high - candle.low)
        body = abs(candle.close - candle.open)
        bb_width = float(features["bb_width"][idx])  # type: ignore[index]
        ema21 = float(features["ema21"][idx])  # type: ignore[index]
        ema89 = float(features["ema89"][idx])  # type: ignore[index]
        ema_gap = float(features["ema_gap"][idx])  # type: ignore[index]
        rsi_value = float(features["rsi"][idx])  # type: ignore[index]
        high_10 = float(features["high_10"][idx])  # type: ignore[index]
        low_10 = float(features["low_10"][idx])  # type: ignore[index]

        out["range_atr"].append(bar_range / atr_den)
        out["body_frac"].append(body / max(bar_range, 1e-12))
        out["atr_pct"].append(float(features["atr_pct"][idx]))  # type: ignore[index]
        out["bb_width"].append(bb_width)
        out["bb_width_inv"].append(-bb_width if math.isfinite(bb_width) else float("nan"))
        out["ema_gap"].append(ema_gap)
        out["abs_ema_gap"].append(abs(ema_gap))
        out["rsi"].append(rsi_value)
        out["rsi_distance"].append(abs(rsi_value - 50.0) / 50.0)
        out["close_ema21_atr"].append((candle.close - ema21) / atr_den)
        out["close_ema89_atr"].append((candle.close - ema89) / atr_den)
        prev_ema21 = float(features["ema21"][max(0, idx - 5)])  # type: ignore[index]
        out["ema21_slope_atr"].append((ema21 - prev_ema21) / atr_den)

        if idx >= 10:
            mean_close = sum(c.close for c in candles[idx - 10 : idx]) / 10.0
            before = abs(candle.open - mean_close)
            after = abs(candle.close - mean_close)
            reclaim = max(0.0, 1.0 - after / max(before, 1e-12)) if after < before else 0.0
        else:
            reclaim = float("nan")
        out["mean_reclaim"].append(reclaim)

        if idx >= 10 and bar_range > 0.0 and math.isfinite(high_10) and math.isfinite(low_10):
            swept_high = candle.high > high_10 and candle.close < (candle.high - 0.6 * bar_range)
            swept_low = candle.low < low_10 and candle.close > (candle.low + 0.6 * bar_range)
            sweep = bar_range / atr_den if swept_high or swept_low else 0.0
        else:
            sweep = float("nan")
        out["sweep_reject"].append(sweep)

        if math.isfinite(high_10) and math.isfinite(low_10):
            upside = max(0.0, candle.close - high_10) / atr_den
            downside = max(0.0, low_10 - candle.close) / atr_den
            breakout = max(upside, downside)
        else:
            breakout = float("nan")
        out["breakout_atr"].append(breakout)

        if idx >= 20:
            recent = candles[idx - 20 : idx]
            mean_range20 = mean_range(recent)
            net20 = abs(candle.close - candles[idx - 20].close)
            path20 = sum(max(0.0, item.high - item.low) for item in recent)
            mean_close20 = sum(item.close for item in recent) / 20.0
            prior_efficiency = net20 / max(path20, 1e-12)
            chop = path20 / max(net20, 1e-12)
            mean_distance = (candle.close - mean_close20) / atr_den
            mean_pressure = abs(mean_distance) * (1.0 - prior_efficiency)
        else:
            mean_range20 = float("nan")
            prior_efficiency = float("nan")
            chop = float("nan")
            mean_distance = float("nan")
            mean_pressure = float("nan")
        out["range_vs_mean20"].append(bar_range / max(mean_range20, 1e-12))

        if idx >= 5:
            mean_range5 = mean_range(candles[idx - 5 : idx])
            prev_ema21_slope = (
                float(features["ema21"][max(0, idx - 5)])  # type: ignore[index]
                - float(features["ema21"][max(0, idx - 10)])  # type: ignore[index]
            ) / atr_den
            ema_slope_change = out["ema21_slope_atr"][-1] - prev_ema21_slope
        else:
            mean_range5 = float("nan")
            ema_slope_change = float("nan")
        out["range_change5"].append(bar_range / max(mean_range5, 1e-12))
        out["body_signed_atr"].append((candle.close - candle.open) / atr_den)
        out["close_pos_range"].append((candle.close - candle.low) / max(bar_range, 1e-12))
        upper_wick = candle.high - max(candle.open, candle.close)
        lower_wick = min(candle.open, candle.close) - candle.low
        out["upper_wick_frac"].append(max(0.0, upper_wick) / max(bar_range, 1e-12))
        out["lower_wick_frac"].append(max(0.0, lower_wick) / max(bar_range, 1e-12))
        out["prior_efficiency20"].append(prior_efficiency)
        out["chop20"].append(chop)
        out["mean_distance_atr"].append(mean_distance)
        out["mean_reversion_pressure"].append(mean_pressure)
        bb_width_p70 = float(features["bb_width_p70"][idx])  # type: ignore[index]
        atr_p50 = float(features["atr_p50"][idx])  # type: ignore[index]
        out["bb_width_ratio"].append(bb_width / max(bb_width_p70, 1e-12))
        out["atr_pct_ratio"].append(float(features["atr_pct"][idx]) / max(atr_p50, 1e-12))  # type: ignore[index]
        out["ema_slope_change"].append(ema_slope_change)
        volume_z20 = float(features["volume_z20"][idx])  # type: ignore[index]
        volume_z50 = float(features["volume_z50"][idx])  # type: ignore[index]
        rel_volume20 = float(features["rel_volume20"][idx])  # type: ignore[index]
        rel_volume50 = float(features["rel_volume50"][idx])  # type: ignore[index]
        bb_pctb = float(features["bb_pctb"][idx])  # type: ignore[index]
        high_20 = float(features["high_20"][idx])  # type: ignore[index]
        low_20 = float(features["low_20"][idx])  # type: ignore[index]
        donchian_breakout = float("nan")
        if math.isfinite(high_20) and math.isfinite(low_20):
            donchian_breakout = max(
                max(0.0, candle.close - high_20),
                max(0.0, low_20 - candle.close),
            ) / atr_den
        premium_discount = float(features["premium_discount_50"][idx])  # type: ignore[index]
        out["volume_z20"].append(volume_z20)
        out["volume_z50"].append(volume_z50)
        out["rel_volume20"].append(rel_volume20)
        out["rel_volume50"].append(rel_volume50)
        out["volume_trend"].append(float(features["volume_trend"][idx]))  # type: ignore[index]
        out["obv_slope10"].append(float(features["obv_slope10"][idx]))  # type: ignore[index]
        out["bb_pctb"].append(bb_pctb)
        out["bb_pctb_extreme"].append(abs(bb_pctb - 0.5) * 2.0 if math.isfinite(bb_pctb) else float("nan"))
        out["bb_width_change5"].append(float(features["bb_width_change5"][idx]))  # type: ignore[index]
        out["donchian_width_atr"].append(float(features["donchian_width_atr"][idx]))  # type: ignore[index]
        out["donchian_breakout_atr"].append(donchian_breakout)
        out["keltner_pos"].append(float(features["keltner_pos"][idx]))  # type: ignore[index]
        out["macd_hist_atr"].append(float(features["macd_hist_atr"][idx]))  # type: ignore[index]
        out["macd_hist_slope"].append(float(features["macd_hist_slope"][idx]))  # type: ignore[index]
        out["stoch_k"].append(float(features["stoch_k"][idx]))  # type: ignore[index]
        out["stoch_d"].append(float(features["stoch_d"][idx]))  # type: ignore[index]
        out["cci_scaled"].append(float(features["cci"][idx]) / 200.0)  # type: ignore[index]
        out["adx"].append(float(features["adx"][idx]))  # type: ignore[index]
        out["adx_slope"].append(float(features["adx_slope"][idx]))  # type: ignore[index]
        out["bull_fvg_gap_atr"].append(float(features["bull_fvg_gap_atr"][idx]))  # type: ignore[index]
        out["bear_fvg_gap_atr"].append(float(features["bear_fvg_gap_atr"][idx]))  # type: ignore[index]
        out["fvg_abs_atr"].append(float(features["fvg_abs_atr"][idx]))  # type: ignore[index]
        out["order_block_touch_score"].append(float(features["order_block_touch_score"][idx]))  # type: ignore[index]
        out["order_block_age_score"].append(float(features["order_block_age_score"][idx]))  # type: ignore[index]
        out["breaker_score"].append(float(features["breaker_score"][idx]))  # type: ignore[index]
        out["breaker_continuation_score"].append(float(features["breaker_continuation_score"][idx]))  # type: ignore[index]
        out["sweep_displacement_score"].append(float(features["sweep_displacement_score"][idx]))  # type: ignore[index]
        out["sweep_reversal_score"].append(float(features["sweep_reversal_score"][idx]))  # type: ignore[index]
        out["sweep_continuation_score"].append(float(features["sweep_continuation_score"][idx]))  # type: ignore[index]
        out["fvg_mitigation_score"].append(float(features["fvg_mitigation_score"][idx]))  # type: ignore[index]
        out["fvg_failed_mitigation_score"].append(float(features["fvg_failed_mitigation_score"][idx]))  # type: ignore[index]
        out["premium_discount_50"].append(premium_discount)
        out["premium_discount_edge"].append(abs(premium_discount - 0.5) * 2.0 if math.isfinite(premium_discount) else float("nan"))
        out["ob_post_mitigation_score"].append(float(features["ob_post_mitigation_score"][idx]))  # type: ignore[index]
        out["engulfing_score"].append(float(features["engulfing_score"][idx]))  # type: ignore[index]
        out["pin_rejection_score"].append(float(features["pin_rejection_score"][idx]))  # type: ignore[index]
        out["propulsion_score"].append(float(features["propulsion_score"][idx]))  # type: ignore[index]
    return out


def higher_timeframe_feature_vectors(
    base_candles: list[Candle],
    htf_candles: list[Candle],
    prefix: str,
) -> dict[str, list[float]]:
    if not htf_candles:
        return {}
    htf_features = build_features(htf_candles)
    htf_timestamps = [candle.timestamp for candle in htf_candles]
    out = {
        f"{prefix}_range_atr": [],
        f"{prefix}_ema_gap": [],
        f"{prefix}_atr_pct_ratio": [],
        f"{prefix}_bb_width_ratio": [],
        f"{prefix}_rsi": [],
        f"{prefix}_close_ema89_atr": [],
    }
    for candle in base_candles:
        pos = bisect.bisect_right(htf_timestamps, candle.timestamp) - 1
        if pos < 0:
            for values in out.values():
                values.append(float("nan"))
            continue
        htf = htf_candles[pos]
        atr_value = float(htf_features["atr"][pos])  # type: ignore[index]
        atr_den = max(atr_value, 1e-12)
        bar_range = max(0.0, htf.high - htf.low)
        atr_pct = float(htf_features["atr_pct"][pos])  # type: ignore[index]
        atr_p50 = float(htf_features["atr_p50"][pos])  # type: ignore[index]
        bb_width = float(htf_features["bb_width"][pos])  # type: ignore[index]
        bb_width_p70 = float(htf_features["bb_width_p70"][pos])  # type: ignore[index]
        ema89 = float(htf_features["ema89"][pos])  # type: ignore[index]
        out[f"{prefix}_range_atr"].append(bar_range / atr_den)
        out[f"{prefix}_ema_gap"].append(float(htf_features["ema_gap"][pos]))  # type: ignore[index]
        out[f"{prefix}_atr_pct_ratio"].append(atr_pct / max(atr_p50, 1e-12))
        out[f"{prefix}_bb_width_ratio"].append(bb_width / max(bb_width_p70, 1e-12))
        out[f"{prefix}_rsi"].append(float(htf_features["rsi"][pos]))  # type: ignore[index]
        out[f"{prefix}_close_ema89_atr"].append((htf.close - ema89) / atr_den)
    return out


def paired_market_feature_vectors(
    base_candles: list[Candle],
    base_features: dict[str, list[float] | list[bool]],
    paired_candles: list[Candle],
    pair_name: str,
    max_age_seconds: int = 7200,
) -> dict[str, list[float]]:
    if not paired_candles:
        return {}
    pair_features = build_features(paired_candles)
    pair_timestamps = [candle.timestamp for candle in paired_candles]
    prefix = f"pair_{pair_name.lower()}"
    out = {
        f"{prefix}_range_atr": [],
        f"{prefix}_ema_gap": [],
        f"{prefix}_rsi": [],
        f"{prefix}_close_ema89_atr": [],
        f"{prefix}_return_diff_3": [],
        f"{prefix}_return_diff_6": [],
        f"{prefix}_direction_agree_3": [],
        f"{prefix}_smt_diverge_3": [],
    }
    for idx, candle in enumerate(base_candles):
        pos = bisect.bisect_right(pair_timestamps, candle.timestamp) - 1
        if pos < 0 or (candle.timestamp - pair_timestamps[pos]).total_seconds() > max_age_seconds:
            for values in out.values():
                values.append(float("nan"))
            continue
        pair = paired_candles[pos]
        pair_atr = float(pair_features["atr"][pos])  # type: ignore[index]
        pair_atr_den = max(pair_atr, 1e-12)
        pair_range = max(0.0, pair.high - pair.low)
        pair_ema89 = float(pair_features["ema89"][pos])  # type: ignore[index]
        out[f"{prefix}_range_atr"].append(pair_range / pair_atr_den)
        out[f"{prefix}_ema_gap"].append(float(pair_features["ema_gap"][pos]))  # type: ignore[index]
        out[f"{prefix}_rsi"].append(float(pair_features["rsi"][pos]))  # type: ignore[index]
        out[f"{prefix}_close_ema89_atr"].append((pair.close - pair_ema89) / pair_atr_den)

        for lookback in [3, 6]:
            if idx >= lookback and pos >= lookback:
                base_atr = float(base_features["atr"][idx])  # type: ignore[index]
                base_ret = (candle.close - base_candles[idx - lookback].close) / max(base_atr, 1e-12)
                pair_ret = (pair.close - paired_candles[pos - lookback].close) / pair_atr_den
                if lookback == 3:
                    base_dir = sign(base_ret)
                    pair_dir = sign(pair_ret)
                    out[f"{prefix}_direction_agree_3"].append(1.0 if base_dir == pair_dir else 0.0)
                    out[f"{prefix}_smt_diverge_3"].append(1.0 if base_dir != 0.0 and pair_dir != 0.0 and base_dir != pair_dir else 0.0)
                out[f"{prefix}_return_diff_{lookback}"].append(base_ret - pair_ret)
            else:
                if lookback == 3:
                    out[f"{prefix}_direction_agree_3"].append(float("nan"))
                    out[f"{prefix}_smt_diverge_3"].append(float("nan"))
                out[f"{prefix}_return_diff_{lookback}"].append(float("nan"))
    return out


def fit_label_stumps(
    vectors: dict[str, list[float]],
    truth: list[str],
    train_end: int,
    label_set: list[str] | None = None,
) -> list[TrainedStump]:
    train_end = max(1, min(train_end, len(truth)))
    stumps: list[TrainedStump] = []
    min_support = max(20, int(train_end * 0.002))
    quantiles = [idx / 20.0 for idx in range(1, 20)]
    labels = label_set or LABELS
    for label in [item for item in labels if item != "unknown"]:
        positives = sum(1 for idx in range(train_end) if truth[idx] == label)
        if positives == 0:
            continue
        best: TrainedStump | None = None
        for feature, values in vectors.items():
            train_values = [values[idx] for idx in range(train_end) if math.isfinite(values[idx])]
            if len(train_values) < min_support:
                continue
            train_values.sort()
            thresholds = sorted(
                {
                    train_values[min(len(train_values) - 1, max(0, int(q * (len(train_values) - 1))))]
                    for q in quantiles
                }
            )
            for threshold in thresholds:
                for direction in [">=", "<="]:
                    tp = fp = fn = 0
                    for idx in range(train_end):
                        value = values[idx]
                        if not math.isfinite(value):
                            predicted = False
                        elif direction == ">=":
                            predicted = value >= threshold
                        else:
                            predicted = value <= threshold
                        actual = truth[idx] == label
                        if predicted and actual:
                            tp += 1
                        elif predicted and not actual:
                            fp += 1
                        elif not predicted and actual:
                            fn += 1
                    support = tp + fp
                    if support < min_support:
                        continue
                    precision = tp / support if support else 0.0
                    recall = tp / positives if positives else 0.0
                    denom = 2 * tp + fp + fn
                    f1 = 2 * tp / denom if denom else 0.0
                    candidate = TrainedStump(
                        label=label,
                        feature=feature,
                        threshold=threshold,
                        direction=direction,
                        f1=f1,
                        precision=precision,
                        recall=recall,
                    )
                    if best is None or (candidate.f1, candidate.precision) > (
                        best.f1,
                        best.precision,
                    ):
                        best = candidate
        if best is not None:
            stumps.append(best)
    return stumps


def fit_gaussian_models(
    vectors: dict[str, list[float]],
    truth: list[str],
    train_end: int,
    label_set: list[str] | None = None,
) -> list[GaussianLabelModel]:
    train_end = max(1, min(train_end, len(truth)))
    models: list[GaussianLabelModel] = []
    min_support = max(20, int(train_end * 0.001))
    labels = label_set or LABELS
    for label in labels:
        indices = [idx for idx in range(train_end) if truth[idx] == label]
        if len(indices) < min_support:
            continue
        means: dict[str, float] = {}
        variances: dict[str, float] = {}
        for feature, values in vectors.items():
            sample = [values[idx] for idx in indices if math.isfinite(values[idx])]
            if len(sample) < min_support:
                continue
            mean = sum(sample) / len(sample)
            variance = sum((value - mean) ** 2 for value in sample) / len(sample)
            means[feature] = mean
            variances[feature] = max(variance, 1e-9)
        if means:
            models.append(
                GaussianLabelModel(
                    label=label,
                    prior=len(indices) / train_end,
                    means=means,
                    variances=variances,
                )
            )
    return models


def fit_extra_trees(
    vectors: dict[str, list[float]],
    truth: list[str],
    train_end: int,
    label_set: list[str],
    *,
    tree_count: int = 9,
    max_depth: int = 5,
    min_leaf: int = 160,
    max_samples: int | None = None,
    feature_sample_count: int = 18,
    threshold_sample_count: int = 3,
    seed: int = 1729,
) -> list[ExtraTreeNode]:
    if tree_count <= 0:
        return []
    train_end = max(1, min(train_end, len(truth)))
    labels = [label for label in label_set if label in set(truth[:train_end])]
    if len(labels) <= 1:
        return []
    counts = Counter(truth[:train_end])
    label_weights = {
        label: math.sqrt(train_end / max(counts.get(label, 1), 1))
        for label in labels
    }
    candidate_features = usable_tree_features(vectors, train_end, min_leaf)
    if not candidate_features:
        return []
    indices = [idx for idx in range(train_end) if truth[idx] in labels]
    sample_size = len(indices)
    if max_samples is not None:
        sample_size = max(1, min(sample_size, max_samples))
    rng = random.Random(seed)
    trees = []
    for tree_idx in range(tree_count):
        sample = [rng.choice(indices) for _ in range(sample_size)]
        trees.append(
            build_extra_tree_node(
                sample,
                vectors,
                truth,
                labels,
                label_weights,
                candidate_features,
                rng,
                depth=0,
                max_depth=max_depth,
                min_leaf=min_leaf,
                feature_sample_count=feature_sample_count,
                threshold_sample_count=threshold_sample_count,
            )
        )
        rng.seed(seed + 1009 * (tree_idx + 1))
    return trees


def usable_tree_features(
    vectors: dict[str, list[float]],
    train_end: int,
    min_leaf: int,
) -> list[str]:
    out = []
    min_valid = max(min_leaf * 2, int(train_end * 0.35))
    for feature, values in vectors.items():
        finite = [values[idx] for idx in range(train_end) if math.isfinite(values[idx])]
        if len(finite) < min_valid:
            continue
        low = min(finite)
        high = max(finite)
        if math.isfinite(low) and math.isfinite(high) and abs(high - low) > 1e-12:
            out.append(feature)
    return out


def build_extra_tree_node(
    indices: list[int],
    vectors: dict[str, list[float]],
    truth: list[str],
    labels: list[str],
    label_weights: dict[str, float],
    candidate_features: list[str],
    rng: random.Random,
    *,
    depth: int,
    max_depth: int,
    min_leaf: int,
    feature_sample_count: int,
    threshold_sample_count: int,
) -> ExtraTreeNode:
    leaf_label, confidence = weighted_majority(indices, truth, labels, label_weights)
    if depth >= max_depth or len(indices) < min_leaf * 2 or label_purity(indices, truth) >= 0.985:
        return ExtraTreeNode(label=leaf_label, confidence=confidence)

    parent_impurity = weighted_gini(indices, truth, labels, label_weights)
    parent_weight = weighted_total(indices, truth, label_weights)
    best: tuple[float, str, float, list[int], list[int], bool] | None = None
    feature_count = min(feature_sample_count, len(candidate_features))
    for feature in rng.sample(candidate_features, feature_count):
        values = vectors[feature]
        finite_values = [values[idx] for idx in indices if math.isfinite(values[idx])]
        if len(finite_values) < min_leaf * 2:
            continue
        thresholds = random_thresholds(finite_values, threshold_sample_count, rng)
        for threshold in thresholds:
            left = []
            right = []
            for idx in indices:
                value = values[idx]
                if not math.isfinite(value):
                    continue
                if value <= threshold:
                    left.append(idx)
                else:
                    right.append(idx)
            if len(left) < min_leaf or len(right) < min_leaf:
                continue
            left_weight = weighted_total(left, truth, label_weights)
            right_weight = weighted_total(right, truth, label_weights)
            if left_weight <= 0.0 or right_weight <= 0.0:
                continue
            child_impurity = (
                left_weight / parent_weight * weighted_gini(left, truth, labels, label_weights)
                + right_weight / parent_weight * weighted_gini(right, truth, labels, label_weights)
            )
            gain = parent_impurity - child_impurity
            default_left = len(left) >= len(right)
            if best is None or gain > best[0]:
                best = (gain, feature, threshold, left, right, default_left)

    if best is None or best[0] <= 1e-7:
        return ExtraTreeNode(label=leaf_label, confidence=confidence)
    _, feature, threshold, left, right, default_left = best
    left_node = build_extra_tree_node(
        left,
        vectors,
        truth,
        labels,
        label_weights,
        candidate_features,
        rng,
        depth=depth + 1,
        max_depth=max_depth,
        min_leaf=min_leaf,
        feature_sample_count=feature_sample_count,
        threshold_sample_count=threshold_sample_count,
    )
    right_node = build_extra_tree_node(
        right,
        vectors,
        truth,
        labels,
        label_weights,
        candidate_features,
        rng,
        depth=depth + 1,
        max_depth=max_depth,
        min_leaf=min_leaf,
        feature_sample_count=feature_sample_count,
        threshold_sample_count=threshold_sample_count,
    )
    return ExtraTreeNode(
        label=leaf_label,
        confidence=confidence,
        feature=feature,
        threshold=threshold,
        default_left=default_left,
        left=left_node,
        right=right_node,
    )


def random_thresholds(values: list[float], count: int, rng: random.Random) -> list[float]:
    if not values:
        return []
    low = min(values)
    high = max(values)
    if not math.isfinite(low) or not math.isfinite(high) or abs(high - low) <= 1e-12:
        return []
    thresholds = set()
    for _ in range(count):
        thresholds.add(rng.uniform(low, high))
    sorted_values = sorted(values)
    for q in [0.20, 0.40, 0.60, 0.80]:
        if len(thresholds) >= count + 2:
            break
        pos = int(q * (len(sorted_values) - 1))
        thresholds.add(sorted_values[pos])
    return sorted(thresholds)


def weighted_majority(
    indices: list[int],
    truth: list[str],
    labels: list[str],
    label_weights: dict[str, float],
) -> tuple[str, float]:
    counts = {label: 0.0 for label in labels}
    for idx in indices:
        label = truth[idx]
        if label in counts:
            counts[label] += label_weights.get(label, 1.0)
    total = sum(counts.values())
    if total <= 0.0:
        return "unknown", 0.0
    label, score = max(counts.items(), key=lambda item: item[1])
    return label, score / total


def weighted_total(indices: list[int], truth: list[str], label_weights: dict[str, float]) -> float:
    return sum(label_weights.get(truth[idx], 1.0) for idx in indices)


def weighted_gini(
    indices: list[int],
    truth: list[str],
    labels: list[str],
    label_weights: dict[str, float],
) -> float:
    counts = {label: 0.0 for label in labels}
    total = 0.0
    for idx in indices:
        label = truth[idx]
        if label not in counts:
            continue
        weight = label_weights.get(label, 1.0)
        counts[label] += weight
        total += weight
    if total <= 0.0:
        return 0.0
    return 1.0 - sum((counts[label] / total) ** 2 for label in labels)


def label_purity(indices: list[int], truth: list[str]) -> float:
    if not indices:
        return 1.0
    counts = Counter(truth[idx] for idx in indices)
    return counts.most_common(1)[0][1] / len(indices)


def extra_tree_predict(idx: int, node: ExtraTreeNode, vectors: dict[str, list[float]]) -> FactorPrediction:
    current = node
    while current.feature is not None and current.left is not None and current.right is not None:
        value = vectors[current.feature][idx]
        if not math.isfinite(value):
            current = current.left if current.default_left else current.right
        elif value <= current.threshold:
            current = current.left
        else:
            current = current.right
    return FactorPrediction(current.label, current.confidence)


def extra_forest_predict(
    idx: int,
    trees: list[ExtraTreeNode],
    vectors: dict[str, list[float]],
) -> FactorPrediction:
    votes: dict[str, float] = {}
    for tree in trees:
        pred = extra_tree_predict(idx, tree, vectors)
        votes[pred.label] = votes.get(pred.label, 0.0) + max(0.01, pred.score)
    if not votes:
        return pred_unknown()
    label, score = max(votes.items(), key=lambda item: item[1])
    confidence = score / sum(votes.values())
    if label == "unknown":
        return pred_unknown()
    return FactorPrediction(label, confidence)


def extra_tree_feature_usage(trees: list[ExtraTreeNode]) -> dict[str, int]:
    counts: Counter[str] = Counter()

    def walk(node: ExtraTreeNode) -> None:
        if node.feature is not None:
            counts[node.feature] += 1
        if node.left is not None:
            walk(node.left)
        if node.right is not None:
            walk(node.right)

    for tree in trees:
        walk(tree)
    return dict(counts.most_common(20))


def mean_range(candles: list[Candle]) -> float:
    if not candles:
        return 0.0
    return sum(max(0.0, candle.high - candle.low) for candle in candles) / len(candles)


def evaluate_factor(
    name: str,
    fn: Callable[[int], FactorPrediction],
    candles: list[Candle],
    truth: list[str],
    htf_context: dict[str, tuple[list[datetime], list[str]]],
    features: dict[str, list[float] | list[bool]],
    eval_start_idx: int,
) -> dict[str, object]:
    predictions = [fn(idx) for idx in range(len(candles))]
    labels = [pred.label for pred in predictions]
    scores = [pred.score for pred in predictions]
    full_summary = label_summary(labels, truth, 0)
    eval_summary = label_summary(labels, truth, eval_start_idx)
    eta2 = separation_eta_squared(scores, truth)
    transition = transition_metrics(labels, truth)
    resonance = resonance_metrics(candles, labels, htf_context)
    stability = temporal_stability(labels)
    return {
        "name": name,
        "accuracy_all": round(full_summary["accuracy"], 6),
        "macro_f1": round(full_summary["macro_f1"], 6),
        "family_macro_f1": round(full_summary["family_macro_f1"], 6),
        "non_unknown_accuracy": round(full_summary["non_unknown_accuracy"], 6),
        "coverage": round(full_summary["coverage"], 6),
        "covered_precision": round(full_summary["covered_precision"], 6),
        "eval_macro_f1": round(eval_summary["macro_f1"], 6),
        "eval_family_macro_f1": round(eval_summary["family_macro_f1"], 6),
        "eval_non_unknown_accuracy": round(eval_summary["non_unknown_accuracy"], 6),
        "eval_coverage": round(eval_summary["coverage"], 6),
        "eval_covered_precision": round(eval_summary["covered_precision"], 6),
        "separation_eta2": round(eta2, 6),
        "transition_f1": round(transition["f1"], 6),
        "transition_precision": round(transition["precision"], 6),
        "transition_recall": round(transition["recall"], 6),
        "resonance_4h": round(resonance.get("4h", 0.0), 6),
        "resonance_1d": round(resonance.get("1d", 0.0), 6),
        "flip_rate": round(stability["flip_rate"], 6),
        "mean_segment_bars": round(stability["mean_segment_bars"], 3),
        "prediction_counts": count_labels(labels),
        "confusion_matrix": full_summary["confusion_matrix"],
    }


def label_summary(predicted: list[str], truth: list[str], start_idx: int) -> dict[str, object]:
    start_idx = max(0, min(start_idx, len(truth)))
    labels = predicted[start_idx:]
    real = truth[start_idx:]
    metric_all = classification_metrics(labels, real)
    family_metric = classification_metrics(
        [regime_family(label) for label in labels],
        [regime_family(label) for label in real],
        REGIME_FAMILIES,
    )
    covered_idx = [idx for idx, label in enumerate(labels) if label != "unknown"]
    covered_correct = sum(1 for idx in covered_idx if labels[idx] == real[idx])
    covered_precision = covered_correct / len(covered_idx) if covered_idx else 0.0
    coverage = len(covered_idx) / len(labels) if labels else 0.0
    non_unknown_truth = [idx for idx, label in enumerate(real) if label != "unknown"]
    non_unknown_correct = sum(1 for idx in non_unknown_truth if labels[idx] == real[idx])
    non_unknown_accuracy = (
        non_unknown_correct / len(non_unknown_truth) if non_unknown_truth else 0.0
    )
    return {
        "accuracy": metric_all["accuracy"],
        "macro_f1": metric_all["macro_f1"],
        "family_macro_f1": family_metric["macro_f1"],
        "non_unknown_accuracy": non_unknown_accuracy,
        "coverage": coverage,
        "covered_precision": covered_precision,
        "confusion_matrix": metric_all["confusion_matrix"],
    }


def classification_metrics(
    predicted: list[str],
    truth: list[str],
    label_set: list[str] | None = None,
) -> dict[str, object]:
    labels = label_set or LABELS
    total = len(truth)
    correct = sum(1 for pred, real in zip(predicted, truth) if pred == real)
    confusion = {real: {pred: 0 for pred in labels} for real in labels}
    for pred, real in zip(predicted, truth):
        confusion.setdefault(real, {item: 0 for item in labels})
        confusion[real][pred] = confusion[real].get(pred, 0) + 1
    f1_values = []
    for label in labels:
        tp = sum(1 for pred, real in zip(predicted, truth) if pred == label and real == label)
        fp = sum(1 for pred, real in zip(predicted, truth) if pred == label and real != label)
        fn = sum(1 for pred, real in zip(predicted, truth) if pred != label and real == label)
        denom = 2 * tp + fp + fn
        if denom > 0:
            f1_values.append(2 * tp / denom)
    return {
        "accuracy": correct / total if total else 0.0,
        "macro_f1": sum(f1_values) / len(f1_values) if f1_values else 0.0,
        "confusion_matrix": confusion,
    }


def separation_eta_squared(scores: list[float], truth: list[str]) -> float:
    usable = [(s, t) for s, t in zip(scores, truth) if t != "unknown" and math.isfinite(s)]
    if len(usable) < 2:
        return 0.0
    overall = sum(score for score, _ in usable) / len(usable)
    total_var = sum((score - overall) ** 2 for score, _ in usable)
    if total_var <= 1e-12:
        return 0.0
    between = 0.0
    for label in LABELS:
        group = [score for score, real in usable if real == label]
        if not group:
            continue
        mean = sum(group) / len(group)
        between += len(group) * (mean - overall) ** 2
    return max(0.0, min(1.0, between / total_var))


def transition_metrics(predicted: list[str], truth: list[str], window: int = 2) -> dict[str, float]:
    truth_events = transition_indices(truth)
    pred_events = transition_indices(predicted)
    matched_pred = sum(
        1 for pred_idx in pred_events if has_event_within(truth_events, pred_idx, window)
    )
    matched_truth = sum(
        1 for truth_idx in truth_events if has_event_within(pred_events, truth_idx, window)
    )
    precision = matched_pred / len(pred_events) if pred_events else 0.0
    recall = matched_truth / len(truth_events) if truth_events else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    return {"precision": precision, "recall": recall, "f1": f1}


def has_event_within(events: list[int], target: int, window: int) -> bool:
    pos = bisect.bisect_left(events, target)
    if pos < len(events) and abs(events[pos] - target) <= window:
        return True
    if pos > 0 and abs(events[pos - 1] - target) <= window:
        return True
    return False


def transition_indices(labels: list[str]) -> list[int]:
    out = []
    prev = labels[0] if labels else "unknown"
    for idx, label in enumerate(labels[1:], start=1):
        if label != "unknown" and prev != "unknown" and label != prev:
            out.append(idx)
        prev = label
    return out


def resonance_metrics(
    candles: list[Candle],
    predicted: list[str],
    htf_context: dict[str, tuple[list[datetime], list[str]]],
) -> dict[str, float]:
    out = {}
    for timeframe, (timestamps, labels) in htf_context.items():
        aligned = total = 0
        for candle, label in zip(candles, predicted):
            if label == "unknown":
                continue
            pos = bisect.bisect_right(timestamps, candle.timestamp) - 1
            if pos < 0:
                continue
            higher = labels[pos]
            if higher == "unknown":
                continue
            total += 1
            if regime_family(label) == regime_family(higher):
                aligned += 1
        out[timeframe] = aligned / total if total else 0.0
    return out


def regime_family(label: str) -> str:
    if label in {"expansion", "trend_continuation"}:
        return "trend"
    if label in {"compression", "reversion"}:
        return "range"
    if label == "manipulation":
        return "transition"
    return "unknown"


def temporal_stability(labels: list[str]) -> dict[str, float]:
    filtered = [label for label in labels if label != "unknown"]
    if not filtered:
        return {"flip_rate": 0.0, "mean_segment_bars": 0.0}
    segments = []
    current = filtered[0]
    length = 1
    flips = 0
    for label in filtered[1:]:
        if label == current:
            length += 1
        else:
            segments.append(length)
            current = label
            length = 1
            flips += 1
    segments.append(length)
    return {
        "flip_rate": flips / max(1, len(filtered) - 1),
        "mean_segment_bars": sum(segments) / len(segments),
    }


def count_labels(labels: list[str]) -> dict[str, int]:
    return {label: labels.count(label) for label in LABELS}


def split_metrics(results: list[dict[str, object]]) -> list[dict[str, object]]:
    return sorted(
        results,
        key=lambda item: (
            item["eval_family_macro_f1"],
            item["eval_macro_f1"],
            item["family_macro_f1"],
            item["macro_f1"],
            item["eval_covered_precision"],
            item["covered_precision"],
            item["separation_eta2"],
            item["transition_f1"],
        ),
        reverse=True,
    )


def write_markdown(path: Path, payload: dict[str, object]) -> None:
    rows = payload["ranked_results"]
    lines = [
        "# Regime Factor Benchmark",
        "",
        f"- symbol: `{payload['symbol']}`",
        f"- base timeframe: `{payload['base_timeframe']}`",
        f"- truth mode: `{payload['truth_mode']}`",
        f"- outcome horizon: `{payload['outcome_horizon']}`",
        f"- train fraction: `{payload['train_fraction']}`",
        f"- eval start index: `{payload['eval_start_index']}`",
        f"- HTF feature contexts: `{payload['htf_feature_contexts']}`",
        f"- paired feature contexts: `{payload['paired_feature_contexts']}`",
        f"- trained feature sets: `{payload['feature_sets']}`",
        f"- extra tree count/depth/min_leaf/max_samples: `{payload['extra_tree_count']}` / `{payload['extra_tree_depth']}` / `{payload['extra_tree_min_leaf']}` / `{payload['extra_tree_max_samples']}`",
        f"- skipped stumps/gaussian: `{payload['skip_stumps']}` / `{payload['skip_gaussian']}`",
        f"- bars: `{payload['bar_count']}`",
        f"- truth labels: `{payload['truth_label_counts']}`",
        "",
        "| rank | factor | eval_family_f1 | eval_macro_f1 | family_f1 | macro_f1 | eval_precision | precision | coverage | transition_f1 |",
        "|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for rank, item in enumerate(rows, start=1):
        lines.append(
            "| {rank} | `{name}` | {eval_family_macro_f1:.4f} | {eval_macro_f1:.4f} | "
            "{family_macro_f1:.4f} | {macro_f1:.4f} | {eval_covered_precision:.4f} | "
            "{covered_precision:.4f} | {coverage:.4f} | {transition_f1:.4f} |".format(
                rank=rank,
                **item,
            )
        )
    feature_usage = payload.get("model_feature_usage") or {}
    if feature_usage:
        lines.extend(["", "## Model Feature Usage", ""])
        for name, usage in feature_usage.items():  # type: ignore[union-attr]
            top = ", ".join(f"`{feature}`={count}" for feature, count in list(usage.items())[:12])
            lines.append(f"- `{name}`: {top}")
    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "- `macro_f1` and `non_unknown_accuracy` are classification scores against the selected truth-mode labels.",
            "- `family_f1` collapses fine labels into `trend`, `range`, `transition`, and `unknown` regime families.",
            "- `covered_precision` rewards a factor that is correct when it chooses to classify instead of forcing every bar into a label.",
            "- `separation_eta2` measures whether the factor score separates true regimes even when the factor is not a complete classifier.",
            "- `transition_f1` measures predicted label changes within a two-bar window of true regime transitions.",
            "- `resonance_4h` / `resonance_1d` compare predicted regime families against higher-timeframe truth-mode labels.",
            "- `eval_*` metrics are measured on the tail split after the configured train fraction.",
        ]
    )
    path.write_text("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--symbol", default="NQ")
    parser.add_argument("--base-timeframe", default="1h")
    parser.add_argument(
        "--truth-mode",
        choices=[
            "mece",
            "outcome",
            "behavior",
            "transition_event",
            "transition_binary",
            "post_transition_state",
            "post_transition_state_balanced",
            "post_transition_direction",
            "post_transition_absorption",
            "hmm_viterbi",
            "change_point",
            "walk_forward_hmm",
        ],
        default="mece",
    )
    parser.add_argument("--outcome-horizon", type=int, default=8)
    parser.add_argument("--train-fraction", type=float, default=0.70)
    parser.add_argument("--data", required=True)
    parser.add_argument("--data-4h")
    parser.add_argument("--data-1d")
    parser.add_argument(
        "--paired-data",
        action="append",
        default=[],
        help="paired market context as NAME=/path/to/candles.json; may be repeated",
    )
    parser.add_argument(
        "--feature-set",
        action="append",
        help="limit trained classifiers to one or more feature groups; may be comma-separated or repeated",
    )
    parser.add_argument("--extra-tree-count", type=int, default=9)
    parser.add_argument("--extra-tree-depth", type=int, default=5)
    parser.add_argument("--extra-tree-min-leaf", type=int, default=160)
    parser.add_argument("--extra-tree-max-samples", type=int)
    parser.add_argument("--wf-hmm-train-window-max", type=int)
    parser.add_argument("--wf-hmm-eval-window", type=int)
    parser.add_argument("--skip-stumps", action="store_true")
    parser.add_argument("--skip-gaussian", action="store_true")
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--output-md", required=True)
    args = parser.parse_args()

    train_fraction = min(0.95, max(0.05, args.train_fraction))
    candles = load_candles(Path(args.data))
    truth = labels_for_mode(
        candles,
        args.truth_mode,
        args.outcome_horizon,
        train_fraction,
        args.wf_hmm_train_window_max,
        args.wf_hmm_eval_window,
    )
    features = build_features(candles)
    eval_start_idx = int(len(candles) * train_fraction)
    htf_context: dict[str, tuple[list[datetime], list[str]]] = {}
    htf_candle_context: dict[str, list[Candle]] = {}
    for timeframe, path in [("4h", args.data_4h), ("1d", args.data_1d)]:
        if not path:
            continue
        htf_candles = load_candles(Path(path))
        htf_candle_context[timeframe] = htf_candles
        htf_context[timeframe] = (
            [c.timestamp for c in htf_candles],
            labels_for_mode(
                htf_candles,
                args.truth_mode,
                args.outcome_horizon,
                train_fraction,
                args.wf_hmm_train_window_max,
                args.wf_hmm_eval_window,
            ),
        )

    factors = build_factor_functions(candles, features)
    extra_vectors: dict[str, list[float]] = {}
    needs_scalar_vectors = (
        wants_cluster_features(args.feature_set)
        or wants_pda_sequence_features(args.feature_set)
        or wants_post_state_features(args.feature_set)
    )
    scalar_vectors = scalar_feature_vectors(candles, features) if needs_scalar_vectors else None
    if wants_pda_sequence_features(args.feature_set) and scalar_vectors is not None:
        extra_vectors.update(pda_sequence_feature_vectors(scalar_vectors))
    if wants_post_state_features(args.feature_set) and scalar_vectors is not None:
        extra_vectors.update(post_state_feature_vectors(candles, features, scalar_vectors))
    if wants_hazard_features(args.feature_set) and scalar_vectors is not None:
        extra_vectors.update(hazard_feature_vectors(candles, features, scalar_vectors))
    if wants_bocpd_lite_features(args.feature_set) and scalar_vectors is not None:
        extra_vectors.update(bocpd_lite_feature_vectors(candles, features, scalar_vectors))
    if wants_ms_regime_features(args.feature_set) and scalar_vectors is not None:
        extra_vectors.update(ms_regime_feature_vectors(candles, scalar_vectors, train_fraction=train_fraction))
    if wants_kmeans_cluster_features(args.feature_set) and scalar_vectors is not None:
        extra_vectors.update(
            kmeans_cluster_feature_vectors(
                candles,
                scalar_vectors,
                train_fraction=train_fraction,
            )
        )
    elif wants_static_cluster_features(args.feature_set):
        extra_vectors.update(hmm_viterbi_feature_vectors(candles))
    elif wants_cluster_features(args.feature_set) and scalar_vectors is not None:
        extra_vectors.update(
            walk_forward_hmm_feature_vectors_budgeted(
                candles,
                scalar_vectors,
                include_bridge=wants_cluster_bridge_features(args.feature_set),
                train_window_max=args.wf_hmm_train_window_max,
                eval_window_override=args.wf_hmm_eval_window,
            )
        )
    for timeframe, htf_candles in htf_candle_context.items():
        extra_vectors.update(higher_timeframe_feature_vectors(candles, htf_candles, timeframe))
    paired_contexts: list[str] = []
    paired_candle_context: dict[str, list[Candle]] = {}
    for spec in args.paired_data:
        pair_name, pair_path = parse_named_path(spec)
        pair_candles = load_candles(pair_path)
        paired_contexts.append(pair_name)
        paired_candle_context[pair_name] = pair_candles
        extra_vectors.update(
            paired_market_feature_vectors(candles, features, pair_candles, pair_name)
        )
    if wants_vol_regime_features(args.feature_set) and paired_candle_context:
        extra_vectors.update(vol_regime_feature_vectors(candles, paired_candle_context))
    factors.update(
        build_trained_factor_functions(
            candles,
            features,
            truth,
            eval_start_idx,
            extra_vectors=extra_vectors,
            extra_tree_count=args.extra_tree_count,
            extra_tree_depth=args.extra_tree_depth,
            extra_tree_min_leaf=args.extra_tree_min_leaf,
            extra_tree_max_samples=args.extra_tree_max_samples,
            feature_sets=args.feature_set,
            include_stumps=not args.skip_stumps,
            include_gaussian=not args.skip_gaussian,
        )
    )
    results = [
        evaluate_factor(name, fn, candles, truth, htf_context, features, eval_start_idx)
        for name, fn in factors.items()
    ]
    model_feature_usage = {
        name: usage
        for name, fn in factors.items()
        if (usage := getattr(fn, "feature_usage", None))
    }
    ranked = split_metrics(results)
    payload: dict[str, object] = {
        "symbol": args.symbol,
        "base_timeframe": args.base_timeframe,
        "truth_mode": args.truth_mode,
        "outcome_horizon": args.outcome_horizon,
        "train_fraction": train_fraction,
        "eval_start_index": eval_start_idx,
        "htf_feature_contexts": sorted(htf_candle_context),
        "paired_feature_contexts": sorted(paired_contexts),
        "feature_sets": normalize_feature_sets(args.feature_set),
        "extra_tree_count": args.extra_tree_count,
        "extra_tree_depth": args.extra_tree_depth,
        "extra_tree_min_leaf": args.extra_tree_min_leaf,
        "extra_tree_max_samples": args.extra_tree_max_samples,
        "wf_hmm_train_window_max": args.wf_hmm_train_window_max,
        "wf_hmm_eval_window": args.wf_hmm_eval_window,
        "skip_stumps": args.skip_stumps,
        "skip_gaussian": args.skip_gaussian,
        "bar_count": len(candles),
        "truth_label_counts": count_labels(truth),
        "model_feature_usage": model_feature_usage,
        "ranked_results": ranked,
    }
    out_json = Path(args.output_json)
    out_md = Path(args.output_md)
    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    write_markdown(out_md, payload)
    print(json.dumps({k: payload[k] for k in ["symbol", "base_timeframe", "bar_count"]}))
    for item in ranked[:5]:
        print(
            "{name}\teval_family_f1={eval_family_macro_f1:.4f}\t"
            "eval_macro_f1={eval_macro_f1:.4f}\tmacro_f1={macro_f1:.4f}\t"
            "covered_precision={covered_precision:.4f}\tcoverage={coverage:.4f}\t"
            "eta2={separation_eta2:.4f}\ttransition_f1={transition_f1:.4f}".format(
                **item
            )
        )
    return 0


def labels_for_mode(
    candles: list[Candle],
    mode: str,
    outcome_horizon: int,
    train_fraction: float = 0.70,
    wf_hmm_train_window_max: int | None = None,
    wf_hmm_eval_window: int | None = None,
) -> list[str]:
    if mode == "mece":
        return manual_mece_labels(candles)
    if mode == "outcome":
        return outcome_regime_labels(candles, horizon=outcome_horizon)
    if mode == "behavior":
        return behavior_regime_labels(candles, horizon=outcome_horizon)
    if mode == "transition_event":
        return transition_event_labels(candles, horizon=outcome_horizon)
    if mode == "transition_binary":
        return transition_binary_labels(candles, horizon=outcome_horizon)
    if mode == "post_transition_state":
        return post_transition_state_labels(candles, horizon=outcome_horizon)
    if mode == "post_transition_state_balanced":
        return post_transition_state_balanced_labels(candles, horizon=outcome_horizon)
    if mode == "post_transition_direction":
        return post_transition_direction_labels(candles, horizon=outcome_horizon)
    if mode == "post_transition_absorption":
        return post_transition_absorption_labels(candles, horizon=outcome_horizon)
    if mode == "hmm_viterbi":
        return hmm_viterbi_labels(candles, train_fraction=train_fraction)
    if mode == "change_point":
        return change_point_labels(candles, train_fraction=train_fraction)
    if mode == "walk_forward_hmm":
        return walk_forward_hmm_labels_budgeted(
            candles,
            train_window_max=wf_hmm_train_window_max,
            eval_window_override=wf_hmm_eval_window,
        )
    raise ValueError(f"unsupported truth mode: {mode}")


def parse_named_path(spec: str) -> tuple[str, Path]:
    if "=" not in spec:
        raise ValueError(f"bad paired-data spec {spec!r}; expected NAME=/path/to/file.json")
    name, path = spec.split("=", 1)
    name = name.strip()
    if not name:
        raise ValueError(f"bad paired-data spec {spec!r}; empty NAME")
    return name, Path(path).expanduser()


if __name__ == "__main__":
    raise SystemExit(main())
