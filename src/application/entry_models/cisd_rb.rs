use chrono::{DateTime, Timelike, Utc};
use chrono_tz::America::New_York;
use serde::{Deserialize, Serialize};

use crate::application::orchestration::PolicyFeatureVector;
use crate::domain::belief::BeliefEvidencePacket;
use crate::ict::{detect_cisd, detect_order_blocks};
use crate::indicators::{compute_atr, compute_ema};
use crate::state::PreBayesEvidenceFilter;
use crate::types::{Candle, Direction, RegimeProbs};

pub const CISD_RB_SETUP_MODEL_ID: &str = "cisd_rb_long_v1";
pub const CISD_RB_HMM_FEATURE_DIM: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CisdRbBestParams {
    pub cisd_bars: u8,
    pub wick_mult: f64,
    pub seq_window: u8,
    pub exit_ema: u8,
    pub roi: f64,
}

pub const CISD_RB_DEFAULT_BEST_PARAMS: CisdRbBestParams = CisdRbBestParams {
    cisd_bars: 3,
    wick_mult: 1.18,
    seq_window: 18,
    exit_ema: 19,
    roi: 0.0215,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CisdRbEntryModelPacket {
    pub setup_model_id: String,
    pub symbol: String,
    pub timeframe: String,
    pub direction: String,
    pub cisd_bars_required: u8,
    pub cisd_run_length_observed: u8,
    pub cisd_impulse_atr: f64,
    pub cisd_body_ratio_mean: f64,
    pub rb_wick_body_ratio: f64,
    pub rb_close_location_ratio: f64,
    pub rb_bullish: bool,
    pub bars_between_cisd_and_rb: u8,
    pub seq_window_limit: u8,
    pub seq_window_hit: bool,
    pub ema19_distance_bps: f64,
    pub atr14_bps: f64,
    pub realized_vol_zscore: f64,
    pub session_label: String,
    pub liquidity_swept: bool,
    pub mss_up: bool,
    pub filtered_market_regime_label: String,
    pub filtered_liquidity_context_label: String,
    pub filtered_resonance_label: String,
    pub evidence_quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CisdRbBbnEvidence {
    pub trend_alignment: String,
    pub liquidity_interaction_quality: String,
    pub trigger_confirmation_quality: String,
    pub session_quality: String,
    pub entry_quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CisdRbCatBoostFeatureRow {
    pub setup_model_id: String,
    pub setup_progress_state: String,
    pub hmm_accumulation_prob: f64,
    pub hmm_manipulation_expansion_prob: f64,
    pub hmm_distribution_prob: f64,
    pub bbn_trend_alignment: String,
    pub bbn_liquidity_interaction_quality: String,
    pub bbn_trigger_confirmation_quality: String,
    pub bbn_session_quality: String,
    pub bbn_entry_quality: String,
    pub cisd_run_length_observed: f64,
    pub cisd_impulse_atr: f64,
    pub cisd_body_ratio_mean: f64,
    pub rb_wick_body_ratio: f64,
    pub rb_close_location_ratio: f64,
    pub bars_between_cisd_and_rb: f64,
    pub seq_window_hit: bool,
    pub ema19_distance_bps: f64,
    pub atr14_bps: f64,
    pub realized_vol_zscore: f64,
    pub evidence_quality_score: f64,
    pub session_label: String,
}

pub fn build_cisd_rb_entry_model_packet(
    symbol: &str,
    timeframe: &str,
    candles: &[Candle],
    filter: &PreBayesEvidenceFilter,
) -> Option<CisdRbEntryModelPacket> {
    if candles.len() < 32 {
        return None;
    }

    let params = &CISD_RB_DEFAULT_BEST_PARAMS;
    let atr = compute_atr(candles, 14);
    let ema = compute_ema(candles, params.exit_ema as usize);
    let order_blocks = detect_order_blocks(candles);
    let mut cisd_confirm_bars = detect_cisd(candles, &order_blocks, params.cisd_bars as usize)
        .into_iter()
        .filter(|item| item.direction == Direction::Bull)
        .map(|item| item.confirm_bar)
        .collect::<Vec<_>>();
    cisd_confirm_bars.extend(fallback_bullish_cisd_confirm_bars(
        candles,
        params.cisd_bars as usize,
    ));
    cisd_confirm_bars.sort_unstable();
    cisd_confirm_bars.dedup();

    for &confirm_bar in cisd_confirm_bars.iter().rev() {
        let observed_run = bullish_run_length(candles, confirm_bar);
        if observed_run < params.cisd_bars as usize {
            continue;
        }

        let last_bar = (confirm_bar + params.seq_window as usize).min(candles.len() - 1);
        if let Some(rb_bar) =
            find_bullish_rejection_bar(candles, confirm_bar, last_bar, params.wick_mult)
        {
            let cisd_start = confirm_bar
                .saturating_add(1)
                .saturating_sub(params.cisd_bars as usize);
            let rb_candle = &candles[rb_bar];
            let bars_between = rb_bar.saturating_sub(confirm_bar) as u8;
            return Some(CisdRbEntryModelPacket {
                setup_model_id: CISD_RB_SETUP_MODEL_ID.to_string(),
                symbol: symbol.to_string(),
                timeframe: timeframe.to_string(),
                direction: "long".to_string(),
                cisd_bars_required: params.cisd_bars,
                cisd_run_length_observed: observed_run as u8,
                cisd_impulse_atr: impulse_atr(candles, &atr, cisd_start, confirm_bar),
                cisd_body_ratio_mean: mean_body_ratio(candles, cisd_start, confirm_bar),
                rb_wick_body_ratio: rb_candle.lower_wick() / rb_candle.body().max(f64::EPSILON),
                rb_close_location_ratio: (rb_candle.close - rb_candle.low)
                    / rb_candle.range().max(f64::EPSILON),
                rb_bullish: rb_candle.is_bullish(),
                bars_between_cisd_and_rb: bars_between,
                seq_window_limit: params.seq_window,
                seq_window_hit: bars_between <= params.seq_window,
                ema19_distance_bps: bps_distance_signed(
                    rb_candle.close,
                    ema_value_at(&ema, candles, rb_bar),
                ),
                atr14_bps: atr_value_at(&atr, candles, rb_bar) / rb_candle.close.max(f64::EPSILON)
                    * 10_000.0,
                realized_vol_zscore: realized_vol_zscore(candles, rb_bar),
                session_label: classify_session_label(rb_candle.timestamp),
                liquidity_swept: filter_label_contains_sweep(filter),
                mss_up: is_mss_up(candles, rb_bar, 5),
                filtered_market_regime_label: filter.filtered_market_regime_label.clone(),
                filtered_liquidity_context_label: filter.filtered_liquidity_context_label.clone(),
                filtered_resonance_label: filter.filtered_multi_timeframe_resonance_label.clone(),
                evidence_quality_score: filter.evidence_quality_score,
            });
        }
    }

    None
}

pub fn build_cisd_rb_hmm_features(
    packet: &CisdRbEntryModelPacket,
) -> [f64; CISD_RB_HMM_FEATURE_DIM] {
    [
        (packet.cisd_run_length_observed.min(5) as f64) / 5.0,
        (packet.cisd_impulse_atr / 3.0).clamp(0.0, 1.0),
        packet.cisd_body_ratio_mean.clamp(0.0, 1.0),
        (packet.rb_wick_body_ratio / 4.0).clamp(0.0, 1.0),
        packet.rb_close_location_ratio.clamp(0.0, 1.0),
        (packet.bars_between_cisd_and_rb as f64 / packet.seq_window_limit.max(1) as f64)
            .clamp(0.0, 1.0),
        (packet.ema19_distance_bps / 100.0).tanh(),
        (packet.realized_vol_zscore / 3.0).tanh(),
    ]
}

pub fn bin_cisd_rb_for_bbn(packet: &CisdRbEntryModelPacket) -> CisdRbBbnEvidence {
    let aligned_flags = [
        packet.filtered_market_regime_label == "bull",
        packet.filtered_resonance_label == "aligned",
        packet.ema19_distance_bps >= -15.0,
    ];
    let trend_alignment = if aligned_flags.iter().all(|flag| *flag) {
        "aligned"
    } else if aligned_flags.iter().filter(|flag| **flag).count() >= 2 {
        "mixed"
    } else {
        "opposed"
    };

    let liquidity_interaction_quality =
        if packet.liquidity_swept && packet.filtered_liquidity_context_label == "favorable" {
            "high"
        } else if packet.liquidity_swept || packet.filtered_liquidity_context_label == "favorable" {
            "medium"
        } else {
            "low"
        };

    let trigger_confirmation_quality = if packet.cisd_run_length_observed >= 3
        && packet.cisd_impulse_atr >= 0.8
        && packet.rb_wick_body_ratio >= 1.18
        && packet.rb_close_location_ratio >= 0.60
        && packet.bars_between_cisd_and_rb <= 6
    {
        "high"
    } else if packet.cisd_run_length_observed >= 3
        && packet.rb_wick_body_ratio >= 1.0
        && packet.bars_between_cisd_and_rb <= 12
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
        && trigger_confirmation_quality == "high"
        && session_quality != "low"
        && packet.evidence_quality_score >= 0.60
    {
        "high"
    } else if matches!(trigger_confirmation_quality, "high" | "medium")
        && packet.evidence_quality_score >= 0.50
    {
        "medium"
    } else {
        "low"
    };

    CisdRbBbnEvidence {
        trend_alignment: trend_alignment.to_string(),
        liquidity_interaction_quality: liquidity_interaction_quality.to_string(),
        trigger_confirmation_quality: trigger_confirmation_quality.to_string(),
        session_quality: session_quality.to_string(),
        entry_quality: entry_quality.to_string(),
    }
}

pub fn apply_cisd_rb_to_belief_packet(
    packet: &CisdRbEntryModelPacket,
    belief: &mut BeliefEvidencePacket,
) {
    if belief.entry_logic_id.is_none() {
        belief.entry_logic_id = Some(packet.setup_model_id.clone());
    }
    if belief.logic_family.is_none() {
        belief.logic_family = Some("entry_model_cisd_rb".to_string());
    }
    belief.factor_evidence.push(format!(
        "entry_model={} symbol={} timeframe={}",
        packet.setup_model_id, packet.symbol, packet.timeframe
    ));
    belief.factor_evidence.push(format!(
        "cisd_rb_trigger_quality run={} wick={:.4} bars={}",
        packet.cisd_run_length_observed, packet.rb_wick_body_ratio, packet.bars_between_cisd_and_rb
    ));
    belief
        .evidence_assignments
        .insert("setup_model_id".to_string(), packet.setup_model_id.clone());
    belief
        .evidence_assignments
        .insert("setup_direction".to_string(), packet.direction.clone());
    belief.timed_pda_summary.insert(
        "cisd_rb_seq_window_hit".to_string(),
        packet.seq_window_hit.to_string(),
    );
    belief.timed_pda_summary.insert(
        "cisd_rb_session_label".to_string(),
        packet.session_label.clone(),
    );
    belief.timed_pda_summary.insert(
        "cisd_rb_bars_between".to_string(),
        packet.bars_between_cisd_and_rb.to_string(),
    );
}

pub fn build_cisd_rb_catboost_feature_row(
    packet: &CisdRbEntryModelPacket,
    hmm_posterior: &RegimeProbs,
    bbn_bins: &CisdRbBbnEvidence,
) -> CisdRbCatBoostFeatureRow {
    CisdRbCatBoostFeatureRow {
        setup_model_id: packet.setup_model_id.clone(),
        setup_progress_state: if packet.seq_window_hit {
            "rb_confirmed".to_string()
        } else {
            "expired".to_string()
        },
        hmm_accumulation_prob: hmm_posterior.accumulation,
        hmm_manipulation_expansion_prob: hmm_posterior.manipulation_expansion,
        hmm_distribution_prob: hmm_posterior.distribution,
        bbn_trend_alignment: bbn_bins.trend_alignment.clone(),
        bbn_liquidity_interaction_quality: bbn_bins.liquidity_interaction_quality.clone(),
        bbn_trigger_confirmation_quality: bbn_bins.trigger_confirmation_quality.clone(),
        bbn_session_quality: bbn_bins.session_quality.clone(),
        bbn_entry_quality: bbn_bins.entry_quality.clone(),
        cisd_run_length_observed: packet.cisd_run_length_observed as f64,
        cisd_impulse_atr: packet.cisd_impulse_atr,
        cisd_body_ratio_mean: packet.cisd_body_ratio_mean,
        rb_wick_body_ratio: packet.rb_wick_body_ratio,
        rb_close_location_ratio: packet.rb_close_location_ratio,
        bars_between_cisd_and_rb: packet.bars_between_cisd_and_rb as f64,
        seq_window_hit: packet.seq_window_hit,
        ema19_distance_bps: packet.ema19_distance_bps,
        atr14_bps: packet.atr14_bps,
        realized_vol_zscore: packet.realized_vol_zscore,
        evidence_quality_score: packet.evidence_quality_score,
        session_label: packet.session_label.clone(),
    }
}

pub fn apply_cisd_rb_to_policy_features(
    packet: &CisdRbEntryModelPacket,
    hmm_posterior: &RegimeProbs,
    bbn_bins: &CisdRbBbnEvidence,
    features: &mut PolicyFeatureVector,
) {
    let row = build_cisd_rb_catboost_feature_row(packet, hmm_posterior, bbn_bins);
    features.setup_model_id = row.setup_model_id;
    features.setup_progress_state = row.setup_progress_state;
    features.factor_alignment = bbn_bins.trend_alignment.clone();
    features.selected_entry_quality = bbn_bins.entry_quality.clone();
    features.selected_direction = "Bull".to_string();
    features.setup_family = "cisd_rejection_block".to_string();
    features.entry_style = "sequence_confirmation".to_string();
    features.signal_bar_pattern = "rejection_block".to_string();
    features.session_model = packet.session_label.clone();
    features.setup_quality = bbn_bins.trigger_confirmation_quality.clone();
    features.liquidity_swept = packet.liquidity_swept;
    features.signal_bar_present = true;
    features.pda_signal_overlap = packet.seq_window_hit;
    features.displacement_strength = packet.cisd_impulse_atr.clamp(0.0, 5.0) / 5.0;
    features.sweep_depth_bps = if packet.liquidity_swept {
        packet.atr14_bps
    } else {
        0.0
    };
    features.entry_price_offset_bps = packet.ema19_distance_bps.abs();
    features.cisd_ltf_confirmed = packet.cisd_run_length_observed >= packet.cisd_bars_required;
    features.rb_pinbar_detected = packet.rb_wick_body_ratio >= 1.0;
    features.cisd_run_length_observed = row.cisd_run_length_observed;
    features.cisd_impulse_atr = row.cisd_impulse_atr;
    features.cisd_body_ratio_mean = row.cisd_body_ratio_mean;
    features.rb_wick_body_ratio = row.rb_wick_body_ratio;
    features.rb_close_location_ratio = row.rb_close_location_ratio;
    features.bars_between_cisd_and_rb = row.bars_between_cisd_and_rb;
    features.seq_window_hit = row.seq_window_hit;
    features.ema19_distance_bps = row.ema19_distance_bps;
    features.realized_vol_zscore = row.realized_vol_zscore;
    features.hmm_accumulation_prob = row.hmm_accumulation_prob;
    features.hmm_manipulation_expansion_prob = row.hmm_manipulation_expansion_prob;
    features.hmm_distribution_prob = row.hmm_distribution_prob;
}

fn bullish_run_length(candles: &[Candle], start: usize) -> usize {
    candles
        .iter()
        .skip(start)
        .take_while(|candle| candle.is_bullish())
        .count()
}

fn fallback_bullish_cisd_confirm_bars(candles: &[Candle], min_bars: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for idx in 2..candles.len() {
        let prev2 = &candles[idx - 2];
        let prev1 = &candles[idx - 1];
        let curr = &candles[idx];
        if prev2.is_bearish()
            && prev1.is_bearish()
            && curr.is_bullish()
            && bullish_run_length(candles, idx) >= min_bars
        {
            out.push(idx);
        }
    }
    out
}

fn find_bullish_rejection_bar(
    candles: &[Candle],
    start: usize,
    end: usize,
    wick_mult: f64,
) -> Option<usize> {
    (start..=end).find(|&idx| {
        let candle = &candles[idx];
        candle.is_bullish()
            && candle.body() > 0.0
            && candle.lower_wick() / candle.body().max(f64::EPSILON) >= wick_mult
    })
}

fn impulse_atr(candles: &[Candle], atr: &[f64], start: usize, end: usize) -> f64 {
    let end_candle = &candles[end];
    let start_open = candles[start].open;
    let atr_value = atr_value_at(atr, candles, end);
    (end_candle.close - start_open) / atr_value.max(f64::EPSILON)
}

fn mean_body_ratio(candles: &[Candle], start: usize, end: usize) -> f64 {
    let window = &candles[start..=end];
    let sum = window
        .iter()
        .map(|candle| candle.body() / candle.range().max(f64::EPSILON))
        .sum::<f64>();
    sum / window.len().max(1) as f64
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

fn ema_value_at(ema: &[f64], candles: &[Candle], candle_index: usize) -> f64 {
    if ema.is_empty() {
        return candles[candle_index].close;
    }
    if candles.len() <= ema.len() {
        return ema[candle_index.min(ema.len() - 1)];
    }
    let start_idx = candles.len() - ema.len();
    if candle_index < start_idx {
        ema.first().copied().unwrap_or(candles[candle_index].close)
    } else {
        ema[(candle_index - start_idx).min(ema.len() - 1)]
    }
}

fn realized_vol_zscore(candles: &[Candle], end: usize) -> f64 {
    if end < 25 {
        return 0.0;
    }
    let returns = candles
        .windows(2)
        .map(|pair| (pair[1].close / pair[0].close.max(f64::EPSILON)).ln())
        .collect::<Vec<_>>();
    if returns.len() < 40 {
        return 0.0;
    }
    let current_end = end.saturating_sub(1);
    let current_start = current_end.saturating_sub(19);
    let current = stddev(&returns[current_start..=current_end]);
    let baseline_start = current_start.saturating_sub(100);
    let mut baseline = Vec::new();
    for idx in baseline_start..current_start {
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

fn bps_distance_signed(price: f64, reference: f64) -> f64 {
    if reference.abs() < f64::EPSILON {
        0.0
    } else {
        (price - reference) / reference * 10_000.0
    }
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

fn filter_label_contains_sweep(filter: &PreBayesEvidenceFilter) -> bool {
    filter.raw_liquidity_context_label.contains("sweep")
        || filter.filtered_liquidity_context_label.contains("sweep")
        || filter
            .nearest_active_pda
            .as_deref()
            .map(|label| label.contains("sweep"))
            .unwrap_or(false)
}

fn is_mss_up(candles: &[Candle], bar_index: usize, lookback: usize) -> bool {
    if bar_index == 0 {
        return false;
    }
    let start = bar_index.saturating_sub(lookback);
    let max_high = candles[start..bar_index]
        .iter()
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    candles[bar_index].close > max_high
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PreBayesEvidencePolicy;
    use chrono::{Duration, TimeZone};

    fn sample_filter() -> PreBayesEvidenceFilter {
        PreBayesEvidenceFilter {
            policy: PreBayesEvidencePolicy::default(),
            raw_market_regime_label: "bull".to_string(),
            raw_liquidity_context_label: "sweep_reverted".to_string(),
            raw_factor_alignment: "aligned".to_string(),
            raw_factor_uncertainty: "low".to_string(),
            filtered_market_regime_label: "bull".to_string(),
            filtered_liquidity_context_label: "favorable".to_string(),
            filtered_factor_alignment: "aligned".to_string(),
            filtered_factor_uncertainty: "low".to_string(),
            filtered_multi_timeframe_resonance_label: "aligned".to_string(),
            evidence_quality_score: 0.72,
            gating_status: "pass_hard".to_string(),
            pass_to_bbn: true,
            ..Default::default()
        }
    }

    fn sample_candles() -> Vec<Candle> {
        let start = Utc.with_ymd_and_hms(2026, 1, 5, 14, 0, 0).unwrap();
        let mut candles = Vec::new();
        let mut price = 100.0;
        for i in 0..60 {
            let timestamp = start + Duration::minutes(i as i64 * 5);
            let mut open = price;
            let mut close = price + 0.05;
            let mut high = close + 0.10;
            let mut low = open - 0.10;
            if i == 20 || i == 21 {
                close = open - 0.60;
                high = open + 0.05;
                low = close - 0.15;
            }
            if i == 22 {
                open = price - 0.10;
                close = price + 0.90;
                high = close + 0.20;
                low = open - 0.05;
            }
            if i == 24 {
                open = price + 0.20;
                close = price + 0.35;
                high = close + 0.05;
                low = open - 0.40;
            }
            price = close;
            candles.push(Candle {
                timestamp,
                open,
                high,
                low,
                close,
                volume: 1000.0 + i as f64 * 10.0,
            });
        }
        candles
    }

    #[test]
    fn builds_cisd_rb_packet_from_synthetic_candles() {
        let packet =
            build_cisd_rb_entry_model_packet("NQ", "5m", &sample_candles(), &sample_filter())
                .expect("expected cisd->rb packet");
        assert_eq!(packet.setup_model_id, CISD_RB_SETUP_MODEL_ID);
        assert_eq!(packet.direction, "long");
        assert!(packet.seq_window_hit);
        assert!(packet.rb_wick_body_ratio >= 1.0);
    }

    #[test]
    fn bins_and_applies_to_policy_features() {
        let packet =
            build_cisd_rb_entry_model_packet("NQ", "5m", &sample_candles(), &sample_filter())
                .unwrap();
        let bins = bin_cisd_rb_for_bbn(&packet);
        let hmm = RegimeProbs {
            accumulation: 0.2,
            manipulation_expansion: 0.7,
            distribution: 0.1,
        };
        let mut features = PolicyFeatureVector::default();
        apply_cisd_rb_to_policy_features(&packet, &hmm, &bins, &mut features);
        assert_eq!(features.setup_model_id, CISD_RB_SETUP_MODEL_ID);
        assert_eq!(features.setup_family, "cisd_rejection_block");
        assert_eq!(features.selected_direction, "Bull");
    }

    #[test]
    fn appends_to_belief_packet() {
        let packet =
            build_cisd_rb_entry_model_packet("NQ", "5m", &sample_candles(), &sample_filter())
                .unwrap();
        let mut belief = BeliefEvidencePacket::default();
        apply_cisd_rb_to_belief_packet(&packet, &mut belief);
        assert_eq!(
            belief.entry_logic_id.as_deref(),
            Some(CISD_RB_SETUP_MODEL_ID)
        );
        assert!(belief
            .timed_pda_summary
            .contains_key("cisd_rb_seq_window_hit"));
    }
}
