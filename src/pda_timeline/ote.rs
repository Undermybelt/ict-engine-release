//! Fibonacci OTE (Optimal Trade Entry) zone calculator.
//!
//! ICT theory defines the OTE retracement zone as the 0.62-0.79
//! Fibonacci band relative to the most recent **swing leg** — i.e.
//! the move from the second-to-last extreme to the last extreme.
//! For a bullish leg (low → high), the OTE zone sits above the low
//! by 62-79% of the leg height; entries are taken **on retracement
//! into** the zone with continuation in the leg direction.
//!
//! This module is the canonical home of the OTE math. Three
//! canonical-setup matchers consume it (FVG, OB, CISD confluence).

use crate::ict::{find_swing_highs, find_swing_lows};
use crate::types::{Candle, Direction, SwingPoint};

/// Lower bound of the OTE retracement window (62%).
pub const OTE_LOW: f64 = 0.62;
/// Upper bound of the OTE retracement window (79%).
pub const OTE_HIGH: f64 = 0.79;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OteZone {
    /// Inclusive lower price bound of the OTE band.
    pub low: f64,
    /// Inclusive upper price bound of the OTE band.
    pub high: f64,
    /// Direction of the underlying swing leg. `Bull` means the leg
    /// went from a swing low up to a swing high; OTE is reached on
    /// a retracement back **down** into the band, with continuation
    /// expected back up.
    pub direction: Direction,
    /// Bar index of the swing extreme that anchors the leg's high
    /// for `Bull` legs and low for `Bear` legs (the "100%" of the
    /// retracement).
    pub leg_end_bar: usize,
}

impl OteZone {
    /// Returns true if `price` lies inside the OTE band [low, high].
    pub fn contains(&self, price: f64) -> bool {
        price >= self.low && price <= self.high
    }
}

/// Compute the OTE zone for the most recent **completed** swing leg
/// in `candles`. Returns `None` when fewer than two swing extremes
/// of opposing types are available.
///
/// "Most recent leg" is defined as the segment from the most recent
/// extreme back to the previous extreme of the opposing type. This
/// matches the leg an ICT trader would visually annotate as "the
/// last move".
///
/// The leg's direction (`Bull` = low → high, `Bear` = high → low)
/// determines on which side of the leg the OTE band sits:
///   - `Bull` leg: band is above the swing low, in (62%, 79%] of
///     the leg height.
///   - `Bear` leg: band is below the swing high, in (62%, 79%] of
///     the leg height (measured downward).
pub fn most_recent_ote_zone(candles: &[Candle], swing_strength: usize) -> Option<OteZone> {
    let highs = find_swing_highs(candles, swing_strength);
    let lows = find_swing_lows(candles, swing_strength);

    let last_high = highs.last();
    let last_low = lows.last();

    match (last_high, last_low) {
        (Some(h), Some(l)) if h.index > l.index => Some(bull_leg(l, h)),
        (Some(h), Some(l)) if l.index > h.index => Some(bear_leg(h, l)),
        _ => None,
    }
}

fn bull_leg(swing_low: &SwingPoint, swing_high: &SwingPoint) -> OteZone {
    let leg = swing_high.price - swing_low.price;
    let band_low = swing_low.price + leg * (1.0 - OTE_HIGH);
    let band_high = swing_low.price + leg * (1.0 - OTE_LOW);
    // band_low < band_high because OTE_LOW < OTE_HIGH and the
    // retracement-from-the-top math inverts the multiplier.
    OteZone {
        low: band_low,
        high: band_high,
        direction: Direction::Bull,
        leg_end_bar: swing_high.index,
    }
}

fn bear_leg(swing_high: &SwingPoint, swing_low: &SwingPoint) -> OteZone {
    let leg = swing_high.price - swing_low.price;
    let band_low = swing_high.price - leg * (1.0 - OTE_LOW);
    let band_high = swing_high.price - leg * (1.0 - OTE_HIGH);
    OteZone {
        low: band_low,
        high: band_high,
        direction: Direction::Bear,
        leg_end_bar: swing_low.index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn ts(n: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
    }

    fn candle(n: i64, low: f64, high: f64) -> Candle {
        Candle {
            timestamp: ts(n),
            open: (low + high) / 2.0,
            high,
            low,
            close: (low + high) / 2.0,
            volume: 1000.0,
        }
    }

    fn synthetic_bull_leg() -> Vec<Candle> {
        // 30 bars: a clear V from low @ bar 10 up to high @ bar 25.
        // Pre-bars descend smoothly to the low, post-bars ascend to
        // the high. Swing detection with strength=3 should pick out
        // index 10 as a swing low and index 25 as a swing high.
        let mut out = Vec::new();
        for i in 0..30 {
            let (lo, hi) = if i < 10 {
                let v = 100.0 - i as f64; // 100, 99, ..., 91
                (v - 0.5, v + 0.5)
            } else if i == 10 {
                (89.0, 90.0) // strict swing low
            } else if i < 25 {
                let v = 90.0 + (i - 10) as f64; // 91, 92, ..., 104
                (v - 0.5, v + 0.5)
            } else if i == 25 {
                (105.0, 106.0) // strict swing high
            } else {
                let v = 106.0 - (i - 25) as f64; // 105, 104, ..., 101
                (v - 0.5, v + 0.5)
            };
            out.push(candle(i as i64, lo, hi));
        }
        out
    }

    #[test]
    fn bull_leg_band_lies_between_62_and_79_pct_retracement() {
        let candles = synthetic_bull_leg();
        let zone = most_recent_ote_zone(&candles, 3).expect("expected zone");
        assert_eq!(zone.direction, Direction::Bull);
        // Leg = 106-89 = 17. Retracement 62%→79% measured from the top:
        //   band_high = 89 + 17 * (1 - 0.62) = 89 + 6.46  = 95.46
        //   band_low  = 89 + 17 * (1 - 0.79) = 89 + 3.57  = 92.57
        assert!((zone.low - 92.57).abs() < 0.05, "got low={}", zone.low);
        assert!((zone.high - 95.46).abs() < 0.05, "got high={}", zone.high);
        assert!(zone.low < zone.high);
    }

    #[test]
    fn empty_when_swings_are_missing() {
        // Flat candles -> no swings -> no zone.
        let flat: Vec<Candle> = (0..20).map(|i| candle(i as i64, 100.0, 100.5)).collect();
        assert!(most_recent_ote_zone(&flat, 3).is_none());
    }

    #[test]
    fn contains_test_is_inclusive_at_boundaries() {
        let zone = OteZone {
            low: 90.0,
            high: 95.0,
            direction: Direction::Bull,
            leg_end_bar: 25,
        };
        assert!(zone.contains(90.0));
        assert!(zone.contains(95.0));
        assert!(zone.contains(92.5));
        assert!(!zone.contains(89.99));
        assert!(!zone.contains(95.01));
    }
}
