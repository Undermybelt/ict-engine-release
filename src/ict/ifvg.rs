//! Inverse Fair Value Gap (iFVG) detector.
//!
//! Lifecycle of an iFVG:
//! 1. A regular FVG forms at bar `origin_bar`.
//! 2. Price returns and *fully fills* the gap at `fill_bar` (the
//!    first bar after `origin_bar + 1` whose body penetrates the
//!    opposite edge of the gap).
//! 3. Within `confirm_window` bars after the fill, price *closes
//!    through* the original far edge in the opposite direction at
//!    `confirm_bar`. This is the inversion — the bullish FVG is now
//!    bearish resistance, or vice versa.
//!
//! Forward-leak: every emitted iFVG has `confirm_bar >= fill_bar` so
//! consumers can filter `confirm_bar <= current_bar` cleanly.
//!
//! See `crate::ict::pda_state` for the lifecycle machinery operating
//! over `TimedPdaState`; this detector is the simpler stateless
//! "did an iFVG event happen in this candle window" projection.

use crate::ict::fvg::detect_fvg;
use crate::types::{Candle, Direction, InverseFairValueGap};

pub const DEFAULT_IFVG_CONFIRM_WINDOW: usize = 8;

pub fn detect_inverse_fvgs(candles: &[Candle], confirm_window: usize) -> Vec<InverseFairValueGap> {
    if candles.len() < 4 {
        return Vec::new();
    }
    let fvgs = detect_fvg(candles);
    let mut out = Vec::new();
    for fvg in &fvgs {
        let scan_start = fvg.start_bar + 2;
        if scan_start >= candles.len() {
            continue;
        }
        let Some(fill_bar) = find_fill_bar(candles, fvg.direction, fvg.top, fvg.bottom, scan_start)
        else {
            continue;
        };
        let confirm_end = (fill_bar + confirm_window).min(candles.len() - 1);
        if confirm_end <= fill_bar {
            continue;
        }
        let Some(confirm_bar) = find_confirm_bar(
            candles,
            fvg.direction,
            fvg.top,
            fvg.bottom,
            fill_bar + 1,
            confirm_end,
        ) else {
            continue;
        };
        out.push(InverseFairValueGap {
            top: fvg.top,
            bottom: fvg.bottom,
            original_direction: fvg.direction,
            direction: invert_direction(fvg.direction),
            origin_bar: fvg.start_bar,
            fill_bar,
            confirm_bar,
        });
    }
    out
}

pub fn detect_inverse_fvgs_default(candles: &[Candle]) -> Vec<InverseFairValueGap> {
    detect_inverse_fvgs(candles, DEFAULT_IFVG_CONFIRM_WINDOW)
}

fn find_fill_bar(
    candles: &[Candle],
    fvg_direction: Direction,
    top: f64,
    bottom: f64,
    scan_start: usize,
) -> Option<usize> {
    for (offset, candle) in candles.iter().skip(scan_start).enumerate() {
        let bar = scan_start + offset;
        match fvg_direction {
            // Bullish FVG: filled when price comes back down and trades
            // through the bottom edge.
            Direction::Bull => {
                if candle.low <= bottom {
                    return Some(bar);
                }
            }
            // Bearish FVG: filled when price comes back up and trades
            // through the top edge.
            Direction::Bear => {
                if candle.high >= top {
                    return Some(bar);
                }
            }
            Direction::Neutral => return None,
        }
    }
    None
}

fn find_confirm_bar(
    candles: &[Candle],
    fvg_direction: Direction,
    top: f64,
    bottom: f64,
    range_start: usize,
    range_end_inclusive: usize,
) -> Option<usize> {
    for bar in range_start..=range_end_inclusive {
        let candle = candles.get(bar)?;
        match fvg_direction {
            // Bullish-FVG inversion to bearish: close back below the
            // gap bottom on a separate bar after the fill.
            Direction::Bull => {
                if candle.close < bottom {
                    return Some(bar);
                }
            }
            // Bearish-FVG inversion to bullish: close back above the
            // gap top on a separate bar after the fill.
            Direction::Bear => {
                if candle.close > top {
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
        assert!(detect_inverse_fvgs_default(&[]).is_empty());
    }

    #[test]
    fn unfilled_bullish_fvg_yields_no_ifvg() {
        // Bullish FVG forms at bar 1, price keeps trending up — no fill.
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.4),
            candle(1, 100.4, 101.0, 100.4, 100.9),
            candle(2, 102.0, 102.5, 102.0, 102.3),
            candle(3, 102.3, 103.0, 102.2, 102.8),
            candle(4, 102.8, 103.5, 102.7, 103.2),
        ];
        assert!(detect_inverse_fvgs_default(&candles).is_empty());
    }

    #[test]
    fn filled_but_unconfirmed_yields_no_ifvg() {
        // Filled but no close-through afterwards.
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.4),
            candle(1, 100.4, 101.0, 100.4, 100.9),
            candle(2, 102.0, 102.5, 102.0, 102.3),
            candle(3, 102.3, 102.5, 100.5, 100.6),
            candle(4, 100.6, 101.0, 100.5, 100.7),
            candle(5, 100.7, 101.0, 100.5, 100.6),
        ];
        // Bullish FVG: bar0.high=100.5, bar2.low=102.0 → gap [100.5, 102.0]
        // Bar 3 fills (low=100.5) but bar 3..5 close at 100.6+, never below 100.5.
        let out = detect_inverse_fvgs_default(&candles);
        assert!(
            out.is_empty(),
            "fill without close-through must not yield iFVG"
        );
    }

    #[test]
    fn fill_then_close_through_emits_bearish_ifvg() {
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.4),
            candle(1, 100.4, 101.0, 100.4, 100.9),
            candle(2, 102.0, 102.5, 102.0, 102.3),
            candle(3, 102.3, 102.5, 100.0, 100.2), // fills the gap (low=100.0)
            candle(4, 100.2, 100.4, 99.0, 99.2),   // close 99.2 < bottom 100.5
        ];
        let out = detect_inverse_fvgs_default(&candles);
        assert_eq!(out.len(), 1);
        let ifvg = &out[0];
        assert_eq!(ifvg.original_direction, Direction::Bull);
        assert_eq!(ifvg.direction, Direction::Bear);
        assert_eq!(ifvg.origin_bar, 1);
        assert_eq!(ifvg.fill_bar, 3);
        assert_eq!(ifvg.confirm_bar, 4);
    }

    #[test]
    fn fill_then_close_through_emits_bullish_ifvg_for_bearish_fvg() {
        let candles = vec![
            candle(0, 100.0, 100.5, 100.0, 100.0),
            candle(1, 100.0, 100.0, 99.5, 99.6),
            candle(2, 98.0, 98.5, 98.0, 98.2),
            candle(3, 98.2, 100.5, 98.2, 100.4), // fills (high=100.5)
            candle(4, 100.4, 101.5, 100.3, 101.2), // close 101.2 > top 100.0
        ];
        // Bearish FVG: bar0.low=100.0, bar2.high=98.5 → gap top=100.0, bottom=98.5
        let out = detect_inverse_fvgs_default(&candles);
        assert_eq!(out.len(), 1);
        let ifvg = &out[0];
        assert_eq!(ifvg.original_direction, Direction::Bear);
        assert_eq!(ifvg.direction, Direction::Bull);
        assert_eq!(ifvg.origin_bar, 1);
        assert_eq!(ifvg.fill_bar, 3);
        assert_eq!(ifvg.confirm_bar, 4);
    }

    #[test]
    fn confirm_window_caps_lookahead() {
        // Same as the bullish-iFVG fixture but with the close-through
        // bar pushed past the confirm window.
        let mut candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.4),
            candle(1, 100.4, 101.0, 100.4, 100.9),
            candle(2, 102.0, 102.5, 102.0, 102.3),
            candle(3, 102.3, 102.5, 100.0, 100.2),
        ];
        // 10 sideways bars
        for i in 4..14 {
            candles.push(candle(i as i64, 100.6, 100.7, 100.5, 100.6));
        }
        // Close-through bar at index 14 — outside a confirm_window=4
        candles.push(candle(14, 100.6, 100.7, 99.0, 99.2));

        let out = detect_inverse_fvgs(&candles, 4);
        assert!(
            out.is_empty(),
            "close-through outside confirm_window must not emit"
        );

        let out_wide = detect_inverse_fvgs(&candles, 12);
        assert_eq!(
            out_wide.len(),
            1,
            "close-through inside a wider window must emit"
        );
    }
}
