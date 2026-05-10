//! Breaker Block detector.
//!
//! Lifecycle:
//! 1. An Order Block forms at `origin_bar` (use the existing
//!    `crate::ict::ob::detect_order_blocks` for the upstream pass).
//! 2. Price closes through the OB on the *opposite* side at
//!    `violation_bar` — the OB is no longer valid as
//!    support/resistance in its original direction.
//! 3. Price returns to the OB level from the new side at
//!    `retest_bar` and rejects (wick into the level, close on the
//!    opposite side) — the failed support is now resistance, or
//!    vice versa.
//!
//! Forward-leak: emitted breakers have
//! `origin_bar < violation_bar < retest_bar`, all consumed only from
//! `candles[..=retest_bar]`.

use crate::ict::ob::detect_order_blocks;
use crate::types::{BreakerBlock, Candle, Direction};

pub fn detect_breaker_blocks(candles: &[Candle]) -> Vec<BreakerBlock> {
    if candles.len() < 4 {
        return Vec::new();
    }
    let obs = detect_order_blocks(candles);
    let mut out = Vec::new();
    for ob in &obs {
        let scan_start = ob.bar_index + 1;
        if scan_start >= candles.len() {
            continue;
        }
        let Some(violation_bar) =
            find_violation_bar(candles, ob.ob_type, ob.high, ob.low, scan_start)
        else {
            continue;
        };
        let retest_start = violation_bar + 1;
        if retest_start >= candles.len() {
            continue;
        }
        let Some(retest_bar) = find_retest_bar(candles, ob.ob_type, ob.high, ob.low, retest_start)
        else {
            continue;
        };
        out.push(BreakerBlock {
            high: ob.high,
            low: ob.low,
            original_direction: ob.ob_type,
            direction: invert_direction(ob.ob_type),
            origin_bar: ob.bar_index,
            violation_bar,
            retest_bar,
        });
    }
    out
}

fn find_violation_bar(
    candles: &[Candle],
    ob_direction: Direction,
    high: f64,
    low: f64,
    scan_start: usize,
) -> Option<usize> {
    for (offset, candle) in candles.iter().skip(scan_start).enumerate() {
        let bar = scan_start + offset;
        match ob_direction {
            // Bullish OB acts as support — violated when a bar closes
            // below the OB low.
            Direction::Bull => {
                if candle.close < low {
                    return Some(bar);
                }
            }
            // Bearish OB acts as resistance — violated when a bar
            // closes above the OB high.
            Direction::Bear => {
                if candle.close > high {
                    return Some(bar);
                }
            }
            Direction::Neutral => return None,
        }
    }
    None
}

fn find_retest_bar(
    candles: &[Candle],
    ob_direction: Direction,
    high: f64,
    low: f64,
    scan_start: usize,
) -> Option<usize> {
    for (offset, candle) in candles.iter().skip(scan_start).enumerate() {
        let bar = scan_start + offset;
        match ob_direction {
            // Was bullish support, now bearish resistance: price wicks
            // up to OB.low, closes back below.
            Direction::Bull => {
                if candle.high >= low && candle.close < low {
                    return Some(bar);
                }
            }
            // Was bearish resistance, now bullish support: price wicks
            // down to OB.high, closes back above.
            Direction::Bear => {
                if candle.low <= high && candle.close > high {
                    return Some(bar);
                }
            }
            Direction::Neutral => return None,
        }
    }
    None
}

fn invert_direction(d: Direction) -> Direction {
    match d {
        Direction::Bull => Direction::Bear,
        Direction::Bear => Direction::Bull,
        Direction::Neutral => Direction::Neutral,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

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

    #[test]
    fn empty_yields_empty() {
        assert!(detect_breaker_blocks(&[]).is_empty());
    }

    #[test]
    fn ob_without_violation_yields_no_breaker() {
        // Trending bull market — no closes back through the OB.
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 99.8),
            candle(1, 99.8, 100.0, 99.5, 99.6), // bearish candle (potential bull-OB origin)
            candle(2, 99.6, 102.0, 99.6, 101.8), // bullish expansion
            candle(3, 101.8, 102.5, 101.5, 102.3),
            candle(4, 102.3, 103.0, 102.2, 102.9),
            candle(5, 102.9, 103.5, 102.8, 103.2),
        ];
        assert!(detect_breaker_blocks(&candles).is_empty());
    }

    #[test]
    fn bullish_ob_violation_then_retest_emits_bearish_breaker() {
        // Build: bull-OB at bar 1, violation at bar 5 (close < OB.low),
        // retest at bar 7 (wick up to OB.low, close back below).
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 99.8), // baseline
            candle(1, 99.8, 100.0, 99.5, 99.6),  // bearish — bull-OB origin
            candle(2, 99.6, 102.0, 99.6, 101.8), // bullish expansion
            candle(3, 101.8, 102.5, 101.5, 102.3),
            candle(4, 102.3, 102.5, 101.0, 101.2), // pull back
            candle(5, 101.2, 101.3, 99.0, 99.2),   // VIOLATION: close 99.2 < OB.low 99.5
            candle(6, 99.2, 99.4, 98.5, 98.8),     // continued bearish
            candle(7, 98.8, 99.6, 98.7, 99.0), // RETEST: high 99.6 ≥ OB.low 99.5, close 99.0 < 99.5
        ];

        let out = detect_breaker_blocks(&candles);
        assert!(!out.is_empty(), "expected at least one breaker block");
        // The OB detector may produce multiple OBs; pick the bull one.
        let breaker = out
            .iter()
            .find(|b| b.original_direction == Direction::Bull)
            .expect("expected a bull-OB breaker");
        assert_eq!(breaker.direction, Direction::Bear);
        assert_eq!(breaker.origin_bar, 1);
        assert_eq!(breaker.violation_bar, 5);
        assert_eq!(breaker.retest_bar, 7);
    }

    #[test]
    fn bearish_ob_violation_then_retest_emits_bullish_breaker() {
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.2),
            candle(1, 100.2, 100.7, 100.0, 100.5), // bullish — bear-OB origin
            candle(2, 100.5, 100.7, 98.5, 98.8),   // bearish expansion
            candle(3, 98.8, 99.0, 98.0, 98.2),
            candle(4, 98.2, 99.5, 98.0, 99.4),
            candle(5, 99.4, 101.5, 99.3, 101.2), // VIOLATION: close 101.2 > OB.high 100.7
            candle(6, 101.2, 101.5, 101.0, 101.3),
            candle(7, 101.3, 101.5, 100.0, 101.0), // RETEST: low 100.0 ≤ OB.high 100.7, close 101.0 > 100.7
        ];

        let out = detect_breaker_blocks(&candles);
        assert!(!out.is_empty());
        let breaker = out
            .iter()
            .find(|b| b.original_direction == Direction::Bear)
            .expect("expected a bear-OB breaker");
        assert_eq!(breaker.direction, Direction::Bull);
    }

    #[test]
    fn violated_but_no_retest_yields_no_breaker() {
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 99.8),
            candle(1, 99.8, 100.0, 99.5, 99.6),
            candle(2, 99.6, 102.0, 99.6, 101.8),
            candle(3, 101.8, 102.5, 101.5, 102.3),
            candle(4, 102.3, 102.5, 101.0, 101.2),
            candle(5, 101.2, 101.3, 99.0, 99.2), // violation
            candle(6, 99.2, 99.4, 98.5, 98.6),
            candle(7, 98.6, 98.7, 98.0, 98.2),
            candle(8, 98.2, 98.3, 97.5, 97.7), // never wicks back to OB.low=99.5
        ];
        let out: Vec<_> = detect_breaker_blocks(&candles)
            .into_iter()
            .filter(|b| b.original_direction == Direction::Bull)
            .collect();
        assert!(
            out.is_empty(),
            "violation without retest must not emit a breaker"
        );
    }
}
