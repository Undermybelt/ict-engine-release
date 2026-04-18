use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use super::loader::{
    aggregate_candles_by_minutes, CandleSessionMode, SessionAwareAggregationSummary,
    SessionAwareCleaningSummary,
};
use crate::types::Candle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentedRegimeState {
    BearishExpansion,
    BullishExpansion,
    Consolidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeSegment {
    pub state: SegmentedRegimeState,
    pub start_index: usize,
    pub end_index: usize,
    pub start_price: f64,
    pub end_price: f64,
    pub extremum_price: f64,
    pub bar_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeSegmentationOutput {
    pub timeframe_minutes: i64,
    pub segments: Vec<RegimeSegment>,
    pub latest_state: SegmentedRegimeState,
    pub bullish_share: f64,
    pub bearish_share: f64,
    pub consolidation_share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MultiTimeframeRegimeSummary {
    pub frames: Vec<RegimeSegmentationOutput>,
}

pub fn aggregate_to_multi_timeframe_frames(
    candles_1m: &[Candle],
    intervals: &[i64],
) -> Result<Vec<(i64, Vec<Candle>)>> {
    let mut frames = Vec::new();
    for &interval in intervals {
        if interval <= 0 {
            bail!("interval must be positive");
        }
        let aggregated = if interval == 1 {
            candles_1m.to_vec()
        } else {
            aggregate_candles_by_minutes(candles_1m, interval)?
        };
        frames.push((interval, aggregated));
    }
    Ok(frames)
}

pub fn build_multi_timeframe_regime_snapshot(
    candles_1m: &[Candle],
    intervals: &[i64],
) -> Result<MultiTimeframeRegimeSummary> {
    let mut frames = Vec::new();
    for (interval, candles) in aggregate_to_multi_timeframe_frames(candles_1m, intervals)? {
        frames.push(segment_regimes_lightweight(&candles, interval)?);
    }
    Ok(MultiTimeframeRegimeSummary { frames })
}

pub fn aggregate_session_aware_minutes(
    candles_1m: &[Candle],
    intervals: &[i64],
    session_mode: CandleSessionMode,
) -> Result<SessionAwareAggregationSummary> {
    let mut snapshot_frames = Vec::new();
    let mut frame_sizes = Vec::new();
    for (interval, candles) in aggregate_to_multi_timeframe_frames(candles_1m, intervals)? {
        if candles.len() < 20 {
            continue;
        }
        frame_sizes.push((interval, candles.len()));
        snapshot_frames.push(segment_regimes_lightweight(&candles, interval)?);
    }
    if snapshot_frames.is_empty() {
        let fallback = segment_regimes_lightweight(candles_1m, 1)?;
        frame_sizes.push((1, candles_1m.len()));
        snapshot_frames.push(fallback);
    }
    Ok(SessionAwareAggregationSummary {
        session_mode,
        requested_intervals: intervals.to_vec(),
        frame_sizes,
        regime_snapshot: MultiTimeframeRegimeSummary {
            frames: snapshot_frames,
        },
        cleaning: SessionAwareCleaningSummary {
            timezone: "America/New_York".to_string(),
            session_mode,
            ..SessionAwareCleaningSummary::default()
        },
    })
}

pub fn segment_regimes_lightweight(
    candles: &[Candle],
    timeframe_minutes: i64,
) -> Result<RegimeSegmentationOutput> {
    if candles.len() < 20 {
        bail!("need at least 20 candles for lightweight regime segmentation");
    }

    let closes: Vec<f64> = candles.iter().map(|candle| candle.close).collect();
    let mut rolling_mean = Vec::with_capacity(closes.len());
    let mut rolling_slope = Vec::with_capacity(closes.len());
    let mut rolling_volatility = Vec::with_capacity(closes.len());
    for index in 0..closes.len() {
        let start = index.saturating_sub(19);
        let window = &closes[start..=index];
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        let variance = window
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / window.len() as f64;
        rolling_mean.push(mean);
        rolling_volatility.push(variance.sqrt());
        if index == 0 {
            rolling_slope.push(0.0);
        } else {
            rolling_slope.push(mean - rolling_mean[index - 1]);
        }
    }

    let average_volatility =
        rolling_volatility.iter().sum::<f64>() / rolling_volatility.len() as f64;
    let slope_threshold = average_volatility.max(1e-6) * 0.15;

    let mut states = Vec::with_capacity(candles.len());
    for index in 0..candles.len() {
        let slope = rolling_slope[index];
        let state = if slope > slope_threshold {
            SegmentedRegimeState::BullishExpansion
        } else if slope < -slope_threshold {
            SegmentedRegimeState::BearishExpansion
        } else {
            SegmentedRegimeState::Consolidation
        };
        states.push(state);
    }

    let mut segments = Vec::new();
    let mut start_index = 0usize;
    let mut current_state = states[0];
    for index in 1..states.len() {
        if states[index] != current_state {
            segments.push(build_segment(
                candles,
                current_state,
                start_index,
                index - 1,
            ));
            current_state = states[index];
            start_index = index;
        }
    }
    segments.push(build_segment(
        candles,
        current_state,
        start_index,
        states.len() - 1,
    ));

    let total = candles.len() as f64;
    let bullish_bars = states
        .iter()
        .filter(|state| **state == SegmentedRegimeState::BullishExpansion)
        .count() as f64;
    let bearish_bars = states
        .iter()
        .filter(|state| **state == SegmentedRegimeState::BearishExpansion)
        .count() as f64;
    let consolidation_bars = total - bullish_bars - bearish_bars;

    Ok(RegimeSegmentationOutput {
        timeframe_minutes,
        segments,
        latest_state: *states
            .last()
            .ok_or_else(|| anyhow!("missing segmented state"))?,
        bullish_share: bullish_bars / total,
        bearish_share: bearish_bars / total,
        consolidation_share: consolidation_bars / total,
    })
}

fn build_segment(
    candles: &[Candle],
    state: SegmentedRegimeState,
    start_index: usize,
    end_index: usize,
) -> RegimeSegment {
    let slice = &candles[start_index..=end_index];
    let start_price = slice.first().map(|candle| candle.open).unwrap_or(0.0);
    let end_price = slice.last().map(|candle| candle.close).unwrap_or(0.0);
    let extremum_price = match state {
        SegmentedRegimeState::BullishExpansion => slice
            .iter()
            .map(|candle| candle.high)
            .fold(f64::NEG_INFINITY, f64::max),
        SegmentedRegimeState::BearishExpansion => slice
            .iter()
            .map(|candle| candle.low)
            .fold(f64::INFINITY, f64::min),
        SegmentedRegimeState::Consolidation => {
            slice.iter().map(|candle| candle.close).sum::<f64>() / slice.len() as f64
        }
    };

    RegimeSegment {
        state,
        start_index,
        end_index,
        start_price,
        end_price,
        extremum_price,
        bar_count: end_index - start_index + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_1m(count: usize, drift: f64) -> Vec<Candle> {
        (0..count)
            .map(|index| {
                let base = 100.0 + drift * index as f64;
                Candle {
                    timestamp: Utc
                        .timestamp_opt(1_700_000_000 + index as i64 * 60, 0)
                        .unwrap(),
                    open: base,
                    high: base + 0.5,
                    low: base - 0.5,
                    close: base + drift,
                    volume: 1.0,
                }
            })
            .collect()
    }

    #[test]
    fn test_aggregate_to_multi_timeframe_frames_builds_requested_intervals() {
        let candles = sample_1m(30, 0.1);
        let frames = aggregate_to_multi_timeframe_frames(&candles, &[1, 5, 15]).unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].1.len(), 30);
        assert_eq!(frames[1].1.len(), 7);
        assert_eq!(frames[2].1.len(), 3);
    }

    #[test]
    fn test_segment_regimes_lightweight_detects_latest_bullish_state() {
        let candles = sample_1m(40, 0.25);
        let segmented = segment_regimes_lightweight(&candles, 1).unwrap();
        assert_eq!(
            segmented.latest_state,
            SegmentedRegimeState::BullishExpansion
        );
        assert!(segmented.bullish_share > segmented.bearish_share);
        assert!(!segmented.segments.is_empty());
    }
}
