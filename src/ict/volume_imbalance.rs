//! Volume Imbalance detector.
//!
//! Single-bar volume z-score outlier over a rolling baseline window.
//! Distinct from `PropulsionBlock` in that volume imbalance does not
//! require body / range structure — only the volume anomaly. ICT
//! treats VI as evidence of stop-runs and absorption, independent of
//! whether the bar happens to also be a propulsion bar.
//!
//! Forward-leak: baseline window is `[i - window .. i)` — excludes
//! the current bar so the z-score is unbiased.

use crate::types::{Candle, Direction, VolumeImbalance};

pub const DEFAULT_VOLUME_IMBALANCE_WINDOW: usize = 20;
pub const DEFAULT_VOLUME_IMBALANCE_Z_MIN: f64 = 2.5;

pub fn detect_volume_imbalances(
    candles: &[Candle],
    window: usize,
    z_threshold: f64,
) -> Vec<VolumeImbalance> {
    if candles.len() <= window || window == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in window..candles.len() {
        let candle = &candles[i];
        let baseline = &candles[i - window..i];
        let (mean, std_dev) = volume_stats(baseline);
        if std_dev <= f64::EPSILON {
            continue;
        }
        let z_score = (candle.volume - mean) / std_dev;
        if z_score < z_threshold {
            continue;
        }
        let direction = if candle.close > candle.open {
            Direction::Bull
        } else if candle.close < candle.open {
            Direction::Bear
        } else {
            Direction::Neutral
        };
        out.push(VolumeImbalance {
            bar_index: i,
            direction,
            volume: candle.volume,
            mean,
            std_dev,
            z_score,
        });
    }
    out
}

pub fn detect_volume_imbalances_default(candles: &[Candle]) -> Vec<VolumeImbalance> {
    detect_volume_imbalances(
        candles,
        DEFAULT_VOLUME_IMBALANCE_WINDOW,
        DEFAULT_VOLUME_IMBALANCE_Z_MIN,
    )
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

    fn candle(idx: i64, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: ts(idx),
            open: 100.0,
            high: 100.5,
            low: 99.5,
            close,
            volume,
        }
    }

    #[test]
    fn empty_yields_empty() {
        assert!(detect_volume_imbalances_default(&[]).is_empty());
    }

    #[test]
    fn flat_volume_yields_no_imbalance() {
        // All identical volumes ⇒ std = 0 ⇒ no detection.
        let candles: Vec<Candle> = (0..40).map(|i| candle(i as i64, 100.1, 1_000.0)).collect();
        let out = detect_volume_imbalances_default(&candles);
        assert!(out.is_empty());
    }

    #[test]
    fn isolated_volume_spike_is_detected() {
        let mut candles: Vec<Candle> = (0..40)
            .map(|i| {
                let v = 1_000.0 + ((i % 5) as f64) * 25.0;
                candle(i as i64, 100.1, v)
            })
            .collect();
        candles[30] = candle(30, 101.0, 12_000.0);
        let out = detect_volume_imbalances_default(&candles);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bar_index, 30);
        assert_eq!(out[0].direction, Direction::Bull);
        assert!(out[0].z_score >= 2.5);
    }

    #[test]
    fn bear_close_yields_bear_direction() {
        let mut candles: Vec<Candle> = (0..40)
            .map(|i| {
                let v = 1_000.0 + ((i % 5) as f64) * 25.0;
                candle(i as i64, 100.1, v)
            })
            .collect();
        candles[30] = candle(30, 99.0, 12_000.0);
        let out = detect_volume_imbalances_default(&candles);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].direction, Direction::Bear);
    }

    #[test]
    fn forward_leak_guard_holds() {
        let mut candles: Vec<Candle> = (0..50)
            .map(|i| {
                let v = 1_000.0 + ((i % 5) as f64) * 25.0;
                candle(i as i64, 100.1, v)
            })
            .collect();
        candles[35] = candle(35, 101.0, 12_000.0);

        let full = detect_volume_imbalances_default(&candles);
        let prefix = detect_volume_imbalances_default(&candles[..=35]);
        let f35: Vec<&VolumeImbalance> = full.iter().filter(|v| v.bar_index == 35).collect();
        let p35: Vec<&VolumeImbalance> = prefix.iter().filter(|v| v.bar_index == 35).collect();
        assert_eq!(f35.len(), p35.len());
        if !f35.is_empty() {
            assert!((f35[0].z_score - p35[0].z_score).abs() < 1e-9);
        }
    }
}
