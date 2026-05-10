use chrono::{DateTime, Timelike, Utc};
use chrono_tz::America::New_York;
use serde::{Deserialize, Serialize};

use crate::ict::detect_breaker_blocks;
use crate::indicators::{compute_atr, compute_ema};
use crate::state::PreBayesEvidenceFilter;
use crate::types::{Candle, Direction, RegimeProbs};

pub const BREAKER_RB_SETUP_MODEL_ID: &str = "breaker_rb_long_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BreakerRbBestParams {
    pub seq_window: u8,
    pub wick_mult: f64,
    pub exit_ema: u8,
    pub roi: f64,
}

pub const BREAKER_RB_DEFAULT_BEST_PARAMS: BreakerRbBestParams = BreakerRbBestParams {
    seq_window: 14,
    wick_mult: 2.0,
    exit_ema: 19,
    roi: 0.0225,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BreakerRbEntryModelPacket {
    pub setup_model_id: String,
    pub symbol: String,
    pub timeframe: String,
    pub direction: String,
    pub origin_bar: u32,
    pub violation_bar: u32,
    pub retest_bar: u32,
    pub bars_between_violation_and_retest: u8,
    pub seq_window_limit: u8,
    pub seq_window_hit: bool,
    pub breaker_width_bps: f64,
    pub retest_reclaim_bps: f64,
    pub rb_wick_body_ratio: f64,
    pub rb_close_location_ratio: f64,
    pub ema19_distance_bps: f64,
    pub atr14_bps: f64,
    pub realized_vol_zscore: f64,
    pub session_label: String,
    pub filtered_market_regime_label: String,
    pub filtered_liquidity_context_label: String,
    pub filtered_resonance_label: String,
    pub evidence_quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BreakerRbBbnEvidence {
    pub trend_alignment: String,
    pub breaker_retest_quality: String,
    pub session_quality: String,
    pub entry_quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BreakerRbCatBoostFeatureRow {
    pub setup_model_id: String,
    pub setup_progress_state: String,
    pub hmm_accumulation_prob: f64,
    pub hmm_manipulation_expansion_prob: f64,
    pub hmm_distribution_prob: f64,
    pub bbn_trend_alignment: String,
    pub bbn_breaker_retest_quality: String,
    pub bbn_session_quality: String,
    pub bbn_entry_quality: String,
    pub bars_between_violation_and_retest: f64,
    pub breaker_width_bps: f64,
    pub retest_reclaim_bps: f64,
    pub rb_wick_body_ratio: f64,
    pub rb_close_location_ratio: f64,
    pub ema19_distance_bps: f64,
    pub atr14_bps: f64,
    pub realized_vol_zscore: f64,
    pub evidence_quality_score: f64,
    pub session_label: String,
}

pub fn build_breaker_rb_entry_model_packet(
    symbol: &str,
    timeframe: &str,
    candles: &[Candle],
    filter: &PreBayesEvidenceFilter,
) -> Option<BreakerRbEntryModelPacket> {
    if candles.len() < 32 {
        return None;
    }
    let params = &BREAKER_RB_DEFAULT_BEST_PARAMS;
    let atr = compute_atr(candles, 14);
    let ema = compute_ema(candles, params.exit_ema as usize);
    let breakers = detect_breaker_blocks(candles);

    for breaker in breakers
        .iter()
        .rev()
        .filter(|item| item.direction == Direction::Bull)
    {
        let retest_bar = breaker.retest_bar;
        let bars_between = retest_bar.saturating_sub(breaker.violation_bar) as u8;
        let candle = &candles[retest_bar];
        let wick_ratio = candle.lower_wick() / candle.body().max(f64::EPSILON);
        if wick_ratio < params.wick_mult {
            continue;
        }
        return Some(BreakerRbEntryModelPacket {
            setup_model_id: BREAKER_RB_SETUP_MODEL_ID.to_string(),
            symbol: symbol.to_string(),
            timeframe: timeframe.to_string(),
            direction: "long".to_string(),
            origin_bar: breaker.origin_bar as u32,
            violation_bar: breaker.violation_bar as u32,
            retest_bar: retest_bar as u32,
            bars_between_violation_and_retest: bars_between,
            seq_window_limit: params.seq_window,
            seq_window_hit: bars_between <= params.seq_window,
            breaker_width_bps: bps_distance(midpoint(breaker.high, breaker.low), breaker.high),
            retest_reclaim_bps: bps_distance_signed(candle.close, breaker.high),
            rb_wick_body_ratio: wick_ratio,
            rb_close_location_ratio: (candle.close - candle.low) / candle.range().max(f64::EPSILON),
            ema19_distance_bps: ema_distance_bps(&ema, candles, retest_bar),
            atr14_bps: atr_value_at(&atr, candles, retest_bar) / candle.close.max(f64::EPSILON)
                * 10_000.0,
            realized_vol_zscore: realized_vol_zscore(candles, retest_bar),
            session_label: classify_session_label(candle.timestamp),
            filtered_market_regime_label: filter.filtered_market_regime_label.clone(),
            filtered_liquidity_context_label: filter.filtered_liquidity_context_label.clone(),
            filtered_resonance_label: filter.filtered_multi_timeframe_resonance_label.clone(),
            evidence_quality_score: filter.evidence_quality_score,
        });
    }
    None
}

pub fn bin_breaker_rb_for_bbn(packet: &BreakerRbEntryModelPacket) -> BreakerRbBbnEvidence {
    let trend_alignment = if packet.filtered_market_regime_label == "bull"
        && packet.filtered_resonance_label == "aligned"
    {
        "aligned"
    } else if packet.filtered_market_regime_label == "bull"
        || packet.filtered_resonance_label == "aligned"
    {
        "mixed"
    } else {
        "opposed"
    };
    let breaker_retest_quality = if packet.bars_between_violation_and_retest <= 6
        && packet.rb_wick_body_ratio >= 2.0
        && packet.retest_reclaim_bps > 0.0
    {
        "high"
    } else if packet.bars_between_violation_and_retest <= packet.seq_window_limit
        && packet.rb_wick_body_ratio >= 1.2
    {
        "medium"
    } else {
        "low"
    };
    let session_quality = match packet.session_label.as_str() {
        "ny_open" => "high",
        "london" | "ny_mid" => "medium",
        _ => "low",
    };
    let entry_quality = if trend_alignment == "aligned"
        && breaker_retest_quality == "high"
        && packet.evidence_quality_score >= 0.60
    {
        "high"
    } else if matches!(breaker_retest_quality, "high" | "medium")
        && packet.evidence_quality_score >= 0.50
    {
        "medium"
    } else {
        "low"
    };
    BreakerRbBbnEvidence {
        trend_alignment: trend_alignment.to_string(),
        breaker_retest_quality: breaker_retest_quality.to_string(),
        session_quality: session_quality.to_string(),
        entry_quality: entry_quality.to_string(),
    }
}

pub fn build_breaker_rb_catboost_feature_row(
    packet: &BreakerRbEntryModelPacket,
    hmm_posterior: &RegimeProbs,
    bbn: &BreakerRbBbnEvidence,
) -> BreakerRbCatBoostFeatureRow {
    BreakerRbCatBoostFeatureRow {
        setup_model_id: packet.setup_model_id.clone(),
        setup_progress_state: if packet.seq_window_hit {
            "rb_confirmed".to_string()
        } else {
            "expired".to_string()
        },
        hmm_accumulation_prob: hmm_posterior.accumulation,
        hmm_manipulation_expansion_prob: hmm_posterior.manipulation_expansion,
        hmm_distribution_prob: hmm_posterior.distribution,
        bbn_trend_alignment: bbn.trend_alignment.clone(),
        bbn_breaker_retest_quality: bbn.breaker_retest_quality.clone(),
        bbn_session_quality: bbn.session_quality.clone(),
        bbn_entry_quality: bbn.entry_quality.clone(),
        bars_between_violation_and_retest: packet.bars_between_violation_and_retest as f64,
        breaker_width_bps: packet.breaker_width_bps,
        retest_reclaim_bps: packet.retest_reclaim_bps,
        rb_wick_body_ratio: packet.rb_wick_body_ratio,
        rb_close_location_ratio: packet.rb_close_location_ratio,
        ema19_distance_bps: packet.ema19_distance_bps,
        atr14_bps: packet.atr14_bps,
        realized_vol_zscore: packet.realized_vol_zscore,
        evidence_quality_score: packet.evidence_quality_score,
        session_label: packet.session_label.clone(),
    }
}

fn midpoint(high: f64, low: f64) -> f64 {
    (high + low) * 0.5
}

fn bps_distance(anchor: f64, target: f64) -> f64 {
    if anchor.abs() <= f64::EPSILON {
        0.0
    } else {
        ((target - anchor).abs() / anchor.abs()) * 10_000.0
    }
}

fn bps_distance_signed(price: f64, reference: f64) -> f64 {
    if reference.abs() <= f64::EPSILON {
        0.0
    } else {
        (price - reference) / reference * 10_000.0
    }
}

fn atr_value_at(atr: &[f64], candles: &[Candle], candle_index: usize) -> f64 {
    if atr.is_empty() {
        return 0.0;
    }
    if candles.len() <= atr.len() {
        return atr[candle_index.min(atr.len() - 1)];
    }
    let start_idx = candles.len() - atr.len();
    if candle_index < start_idx {
        atr.first().copied().unwrap_or(0.0)
    } else {
        atr[(candle_index - start_idx).min(atr.len() - 1)]
    }
}

fn ema_distance_bps(ema: &[f64], candles: &[Candle], candle_index: usize) -> f64 {
    if ema.is_empty() {
        return 0.0;
    }
    let ema_value = if candles.len() <= ema.len() {
        ema[candle_index.min(ema.len() - 1)]
    } else {
        let start_idx = candles.len() - ema.len();
        if candle_index < start_idx {
            ema.first().copied().unwrap_or(candles[candle_index].close)
        } else {
            ema[(candle_index - start_idx).min(ema.len() - 1)]
        }
    };
    bps_distance_signed(candles[candle_index].close, ema_value)
}

fn realized_vol_zscore(candles: &[Candle], end: usize) -> f64 {
    if end < 25 {
        return 0.0;
    }
    let returns = candles[..=end]
        .windows(2)
        .filter_map(|pair| {
            let prev = pair[0].close;
            let next = pair[1].close;
            if prev.abs() <= f64::EPSILON {
                None
            } else {
                Some((next / prev).ln())
            }
        })
        .collect::<Vec<_>>();
    if returns.len() < 40 {
        return 0.0;
    }
    let current_end = returns.len() - 1;
    let current_start = current_end.saturating_sub(19);
    let current = stddev(&returns[current_start..=current_end]);
    let baseline_end = current_start;
    let baseline_start = baseline_end.saturating_sub(100);
    let mut baseline = Vec::new();
    for idx in baseline_start..baseline_end {
        if idx + 19 >= returns.len() {
            break;
        }
        baseline.push(stddev(&returns[idx..=idx + 19]));
    }
    if baseline.len() < 5 {
        return 0.0;
    }
    let mean = baseline.iter().sum::<f64>() / baseline.len() as f64;
    let std = stddev(&baseline).max(f64::EPSILON);
    (current - mean) / std
}

fn stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn classify_session_label(timestamp: DateTime<Utc>) -> String {
    let ny = timestamp.with_timezone(&New_York);
    match ny.hour() {
        3..=7 => "london".to_string(),
        9 if ny.minute() >= 30 => "ny_open".to_string(),
        10 => "ny_open".to_string(),
        11..=13 => "ny_mid".to_string(),
        20..=23 | 0..=2 => "asia".to_string(),
        _ => "dead_zone".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn ts(n: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
    }

    fn candle(idx: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: ts(idx),
            open,
            high,
            low,
            close,
            volume: 1_000.0,
        }
    }

    fn sample_filter() -> PreBayesEvidenceFilter {
        PreBayesEvidenceFilter {
            filtered_market_regime_label: "bull".to_string(),
            filtered_liquidity_context_label: "favorable".to_string(),
            filtered_multi_timeframe_resonance_label: "aligned".to_string(),
            evidence_quality_score: 0.71,
            ..Default::default()
        }
    }

    #[test]
    fn builds_breaker_rb_packet_from_synthetic_candles() {
        let mut candles = (0..24)
            .map(|idx| {
                let base = 100.0 + idx as f64 * 0.1;
                candle(idx, base, base + 0.3, base - 0.3, base + 0.1)
            })
            .collect::<Vec<_>>();
        candles.extend(vec![
            candle(0, 100.0, 100.5, 99.5, 100.2),
            candle(1, 100.2, 100.7, 100.0, 100.5),
            candle(2, 100.5, 100.7, 98.5, 98.8),
            candle(3, 98.8, 99.0, 98.0, 98.2),
            candle(4, 98.2, 99.5, 98.0, 99.4),
            candle(5, 99.4, 101.5, 99.3, 101.2),
            candle(6, 101.2, 101.5, 101.0, 101.3),
            candle(7, 101.3, 101.5, 100.0, 101.0),
        ]);
        let packet =
            build_breaker_rb_entry_model_packet("NQ", "5m", &candles, &sample_filter()).unwrap();
        assert_eq!(packet.setup_model_id, BREAKER_RB_SETUP_MODEL_ID);
        assert_eq!(packet.direction, "long");
        assert!(packet.rb_wick_body_ratio >= 1.0);
    }
}
