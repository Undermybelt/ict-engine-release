use serde::{Deserialize, Serialize};

use crate::config::FrameFeatures;
use crate::types::Candle;

/// MECE ground-truth regime labels used to validate HMM Viterbi recovery
/// (Sprint 3 of the execution-first plan). The enum lists the variants;
/// `manual_mece_labeler` evaluates them in priority order so the output stays
/// mutually exclusive when a bar satisfies more than one weaker condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MeceRegimeLabel {
    Expansion,
    Manipulation,
    Reversion,
    Compression,
    TrendContinuation,
    Unknown,
}

const LOOKBACK: usize = 10;

pub fn manual_mece_labeler(
    candles: &[Candle],
    _frame_features: &FrameFeatures,
) -> Vec<MeceRegimeLabel> {
    let mut labels = Vec::with_capacity(candles.len());
    for (idx, candle) in candles.iter().enumerate() {
        if idx < LOOKBACK {
            labels.push(MeceRegimeLabel::Unknown);
            continue;
        }
        let lookback = &candles[idx - LOOKBACK..idx];
        let avg_range = average_range(lookback);
        if !avg_range.is_finite() || avg_range <= 0.0 {
            labels.push(MeceRegimeLabel::Unknown);
            continue;
        }
        let prev_max_high = lookback
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let prev_min_low = lookback.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);

        let range = (candle.high - candle.low).max(0.0);
        let body = (candle.close - candle.open).abs();
        let prev = &candles[idx - 1];
        let prev_dir = (prev.close - prev.open).signum();
        let curr_dir = (candle.close - candle.open).signum();

        // Priority 1: Manipulation — pierces a prior extreme then rejects (sweep + close back inside).
        if range > 0.0 {
            let swept_high_and_rejected =
                candle.high > prev_max_high && candle.close < (candle.high - 0.6 * range);
            let swept_low_and_rejected =
                candle.low < prev_min_low && candle.close > (candle.low + 0.6 * range);
            if swept_high_and_rejected || swept_low_and_rejected {
                labels.push(MeceRegimeLabel::Manipulation);
                continue;
            }
        }

        // Priority 2: Compression — tight range vs lookback baseline.
        if range < 0.5 * avg_range {
            labels.push(MeceRegimeLabel::Compression);
            continue;
        }

        // Priority 3: Expansion — wide range with a body that dominates the bar.
        if range > 1.5 * avg_range && body > 0.6 * range {
            labels.push(MeceRegimeLabel::Expansion);
            continue;
        }

        // Priority 4: TrendContinuation — directional body aligned with the prior bar.
        if curr_dir != 0.0 && curr_dir == prev_dir && body > 0.5 * range {
            labels.push(MeceRegimeLabel::TrendContinuation);
            continue;
        }

        // Priority 5: Reversion — closes back toward the lookback mean and against the prior direction.
        let lookback_mean = lookback.iter().map(|c| c.close).sum::<f64>() / LOOKBACK as f64;
        if curr_dir != 0.0
            && curr_dir != prev_dir
            && (candle.close - lookback_mean).abs() < (candle.open - lookback_mean).abs()
        {
            labels.push(MeceRegimeLabel::Reversion);
            continue;
        }

        labels.push(MeceRegimeLabel::Unknown);
    }
    labels
}

fn average_range(window: &[Candle]) -> f64 {
    if window.is_empty() {
        return 0.0;
    }
    window
        .iter()
        .map(|c| (c.high - c.low).max(0.0))
        .sum::<f64>()
        / window.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn frame() -> FrameFeatures {
        FrameFeatures::default()
    }

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

    fn flat_lookback(price: f64) -> Vec<Candle> {
        (0..LOOKBACK as i64)
            .map(|i| candle(i, price, price + 0.5, price - 0.5, price))
            .collect()
    }

    #[test]
    fn flags_unknown_for_insufficient_lookback() {
        let series: Vec<Candle> = (0..3)
            .map(|i| candle(i, 100.0, 100.5, 99.5, 100.0))
            .collect();
        let labels = manual_mece_labeler(&series, &frame());
        assert_eq!(labels.len(), series.len());
        assert!(labels
            .iter()
            .all(|label| *label == MeceRegimeLabel::Unknown));
    }

    #[test]
    fn flags_expansion_for_wide_directional_bar() {
        let mut series = flat_lookback(100.0);
        series.push(candle(LOOKBACK as i64, 100.0, 105.0, 99.5, 104.5));
        let labels = manual_mece_labeler(&series, &frame());
        assert_eq!(labels.last(), Some(&MeceRegimeLabel::Expansion));
    }

    #[test]
    fn flags_manipulation_for_sweep_and_reject() {
        let mut series = flat_lookback(100.0);
        series.push(candle(LOOKBACK as i64, 100.2, 102.0, 100.0, 100.3));
        let labels = manual_mece_labeler(&series, &frame());
        assert_eq!(labels.last(), Some(&MeceRegimeLabel::Manipulation));
    }

    #[test]
    fn flags_compression_for_tight_range() {
        let mut series = flat_lookback(100.0);
        series.push(candle(LOOKBACK as i64, 100.0, 100.05, 99.95, 100.02));
        let labels = manual_mece_labeler(&series, &frame());
        assert_eq!(labels.last(), Some(&MeceRegimeLabel::Compression));
    }

    #[test]
    fn flags_trend_continuation_for_aligned_directional_bars() {
        let series: Vec<Candle> = (0..=LOOKBACK as i64)
            .map(|i| {
                let base = 100.0 + i as f64 * 0.1;
                candle(i, base, base + 0.4, base - 0.1, base + 0.3)
            })
            .collect();
        let labels = manual_mece_labeler(&series, &frame());
        assert_eq!(labels.last(), Some(&MeceRegimeLabel::TrendContinuation));
    }

    #[test]
    fn label_count_matches_input_length() {
        let series: Vec<Candle> = (0..30)
            .map(|i| candle(i, 100.0, 100.5, 99.5, 100.0))
            .collect();
        let labels = manual_mece_labeler(&series, &frame());
        assert_eq!(labels.len(), series.len());
    }
}
