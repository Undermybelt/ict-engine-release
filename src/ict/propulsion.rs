//! Propulsion Block detector.
//!
//! A propulsion block is a single high-conviction continuation candle:
//! - body / range ≥ `body_range_min` (body-dominant, not a wick)
//! - range / atr ≥ `range_atr_min` (range expansion)
//! - volume z-score over a rolling baseline ≥ `volume_z_min`
//!
//! ICT theory treats propulsion blocks as "displacement" candles —
//! they typically appear immediately after MSS / CISD events as the
//! market's confirmation that the new direction has institutional flow.
//!
//! Forward-leak: this detector consumes only `candles[..=i]` for the
//! event at bar `i`. The rolling volume baseline window is strictly
//! `[i.saturating_sub(window) .. i)` (excludes the current bar so the
//! z-score does not self-include).

use crate::types::{Candle, Direction, PropulsionBlock};

/// Default thresholds — tuned to land in the same order of magnitude
/// as `detect_rb`'s defaults (range/atr ≥ 1.0..2.0).
pub const DEFAULT_PROPULSION_BODY_RANGE_MIN: f64 = 0.65;
pub const DEFAULT_PROPULSION_RANGE_ATR_MIN: f64 = 1.5;
pub const DEFAULT_PROPULSION_VOLUME_WINDOW: usize = 20;
pub const DEFAULT_PROPULSION_VOLUME_Z_MIN: f64 = 1.5;

pub fn detect_propulsion_blocks(
    candles: &[Candle],
    atr: &[f64],
    body_range_min: f64,
    range_atr_min: f64,
    volume_window: usize,
    volume_z_min: f64,
) -> Vec<PropulsionBlock> {
    if candles.len() <= volume_window || volume_window == 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    for i in volume_window..candles.len() {
        let candle = &candles[i];
        let range = candle.range();
        if range <= f64::EPSILON {
            continue;
        }

        let body = candle.body();
        let body_ratio = body / range;
        if body_ratio < body_range_min {
            continue;
        }

        let atr_value = atr_at(atr, i, candles.len());
        if atr_value <= f64::EPSILON {
            continue;
        }
        let range_atr = range / atr_value;
        if range_atr < range_atr_min {
            continue;
        }

        let baseline_start = i - volume_window;
        let baseline = &candles[baseline_start..i];
        let (mean, std_dev) = volume_stats(baseline);
        if std_dev <= f64::EPSILON {
            continue;
        }
        let volume_z = (candle.volume - mean) / std_dev;
        if volume_z < volume_z_min {
            continue;
        }

        let direction = if candle.close > candle.open {
            Direction::Bull
        } else if candle.close < candle.open {
            Direction::Bear
        } else {
            // Doji on max-volume + max-range: treat as neutral; the
            // BBN evidence layer will filter it out via FactorRole.
            Direction::Neutral
        };

        out.push(PropulsionBlock {
            bar_index: i,
            direction,
            body_ratio,
            range_atr,
            volume_z,
        });
    }
    out
}

/// Convenience wrapper using module defaults.
pub fn detect_propulsion_blocks_default(candles: &[Candle], atr: &[f64]) -> Vec<PropulsionBlock> {
    detect_propulsion_blocks(
        candles,
        atr,
        DEFAULT_PROPULSION_BODY_RANGE_MIN,
        DEFAULT_PROPULSION_RANGE_ATR_MIN,
        DEFAULT_PROPULSION_VOLUME_WINDOW,
        DEFAULT_PROPULSION_VOLUME_Z_MIN,
    )
}

fn atr_at(atr: &[f64], bar_index: usize, total_bars: usize) -> f64 {
    if atr.is_empty() {
        return 0.0;
    }
    // `compute_atr` typically returns a vector aligned to the tail of
    // `candles`; mirror `detect_rb`'s indexing convention.
    let offset = total_bars.saturating_sub(atr.len());
    let idx = bar_index.saturating_sub(offset).min(atr.len() - 1);
    atr[idx]
}

fn volume_stats(window: &[Candle]) -> (f64, f64) {
    let n = window.len() as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let mean = window.iter().map(|c| c.volume).sum::<f64>() / n;
    let var = window
        .iter()
        .map(|c| {
            let diff = c.volume - mean;
            diff * diff
        })
        .sum::<f64>()
        / n;
    (mean, var.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn ts(n: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
    }

    fn candle(idx: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: ts(idx),
            open,
            high,
            low,
            close,
            volume,
        }
    }

    fn flat_baseline(len: usize) -> Vec<Candle> {
        // Flat range ~ 0.5, base volume ~ 1000, alternating direction
        (0..len)
            .map(|i| {
                let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
                let close = 100.0 + dir * 0.1;
                candle(i as i64, 100.0, 100.25, 99.75, close, 1_000.0)
            })
            .collect()
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert!(detect_propulsion_blocks_default(&[], &[]).is_empty());
    }

    #[test]
    fn flat_market_yields_no_propulsion() {
        let candles = flat_baseline(60);
        // ATR ~ 0.5
        let atr = vec![0.5; candles.len()];
        let blocks = detect_propulsion_blocks_default(&candles, &atr);
        assert!(blocks.is_empty(), "flat market must not propulse");
    }

    #[test]
    fn body_dominant_high_volume_bar_emits_bull_propulsion() {
        let mut candles = flat_baseline(40);
        // Inject a propulsion bar at index 30:
        // - range = 2.0 (4× baseline)
        // - body = 1.8 (body_ratio = 0.9)
        // - volume = 6_000 (z ~ 25 vs baseline std 0.0... need variance > 0)
        // Tweak baseline to have non-zero std first.
        for (i, c) in candles.iter_mut().enumerate().take(30) {
            c.volume = 1_000.0 + ((i % 5) as f64) * 50.0;
        }
        candles[30] = candle(30, 100.0, 102.0, 100.0, 101.8, 6_000.0);
        let atr = vec![0.5; candles.len()];

        let blocks = detect_propulsion_blocks_default(&candles, &atr);
        assert_eq!(blocks.len(), 1, "exactly one propulsion bar expected");
        let pb = &blocks[0];
        assert_eq!(pb.bar_index, 30);
        assert_eq!(pb.direction, Direction::Bull);
        assert!(pb.body_ratio >= 0.85);
        assert!(pb.range_atr >= 3.0);
        assert!(pb.volume_z >= 1.5);
    }

    #[test]
    fn wick_dominant_bar_does_not_emit() {
        let mut candles = flat_baseline(40);
        for (i, c) in candles.iter_mut().enumerate().take(30) {
            c.volume = 1_000.0 + ((i % 5) as f64) * 50.0;
        }
        // body = 0.1, range = 2.0 → body_ratio = 0.05 (below threshold)
        candles[30] = candle(30, 100.0, 102.0, 100.0, 100.1, 6_000.0);
        let atr = vec![0.5; candles.len()];
        let blocks = detect_propulsion_blocks_default(&candles, &atr);
        assert!(
            blocks.is_empty(),
            "wick-dominant bar must not be classified as propulsion"
        );
    }

    #[test]
    fn forward_leak_guard_holds() {
        // Detector must produce identical results when fed only the
        // prefix up to and including the bar of interest.
        let mut candles = flat_baseline(50);
        for (i, c) in candles.iter_mut().enumerate().take(35) {
            c.volume = 1_000.0 + ((i % 5) as f64) * 50.0;
        }
        candles[35] = candle(35, 100.0, 102.0, 100.0, 101.8, 6_000.0);
        let atr_full = vec![0.5; candles.len()];

        let full = detect_propulsion_blocks_default(&candles, &atr_full);
        let prefix_candles = &candles[..=35];
        let atr_prefix = vec![0.5; prefix_candles.len()];
        let prefix = detect_propulsion_blocks_default(prefix_candles, &atr_prefix);

        let full_at_35: Vec<&PropulsionBlock> = full.iter().filter(|b| b.bar_index == 35).collect();
        let prefix_at_35: Vec<&PropulsionBlock> =
            prefix.iter().filter(|b| b.bar_index == 35).collect();
        assert_eq!(
            full_at_35.len(),
            prefix_at_35.len(),
            "forward leak: prefix vs full disagree on bar 35"
        );
    }
}
