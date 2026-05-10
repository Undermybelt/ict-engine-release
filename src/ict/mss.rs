//! Market Structure Shift (MSS) detector.
//!
//! MSS is ICT's stricter cousin of CHoCH (change of character):
//! - **CHoCH** in `crate::ict::bos_choch` triggers on a wick break
//!   (`candle.high > prior_swing_high.price`).
//! - **MSS** requires a *close-through* in the new direction.
//!
//! The detector tracks the running trend and only emits on a trend
//! reversal — successive same-direction breaks (which are BOS, not
//! MSS) are skipped.
//!
//! Forward-leak: every emitted MSS uses only `candles[..=bar_index]`.

use crate::ict::swing::{find_swing_highs, find_swing_lows};
use crate::types::{Candle, Direction, MarketStructureShift, SwingPoint};

pub const DEFAULT_MSS_SWING_LOOKBACK: usize = 3;

pub fn detect_market_structure_shifts(
    candles: &[Candle],
    swing_lookback: usize,
) -> Vec<MarketStructureShift> {
    if candles.len() < swing_lookback * 2 + 2 {
        return Vec::new();
    }
    let highs = find_swing_highs(candles, swing_lookback);
    let lows = find_swing_lows(candles, swing_lookback);
    let mut out = Vec::new();
    let mut last_trend = Direction::Neutral;

    for (i, candle) in candles.iter().enumerate() {
        // Most recent swing high strictly before bar `i`.
        if let Some(sh) = latest_swing_before(&highs, i) {
            if candle.close > sh.price && last_trend != Direction::Bull {
                out.push(MarketStructureShift {
                    bar_index: i,
                    direction: Direction::Bull,
                    broken_swing_index: sh.index,
                    broken_swing_price: sh.price,
                });
                last_trend = Direction::Bull;
                continue;
            }
        }
        if let Some(sl) = latest_swing_before(&lows, i) {
            if candle.close < sl.price && last_trend != Direction::Bear {
                out.push(MarketStructureShift {
                    bar_index: i,
                    direction: Direction::Bear,
                    broken_swing_index: sl.index,
                    broken_swing_price: sl.price,
                });
                last_trend = Direction::Bear;
            }
        }
    }
    out
}

pub fn detect_market_structure_shifts_default(candles: &[Candle]) -> Vec<MarketStructureShift> {
    detect_market_structure_shifts(candles, DEFAULT_MSS_SWING_LOOKBACK)
}

fn latest_swing_before(swings: &[SwingPoint], index: usize) -> Option<&SwingPoint> {
    swings
        .iter()
        .filter(|sp| sp.index < index)
        .max_by_key(|sp| sp.index)
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
        assert!(detect_market_structure_shifts_default(&[]).is_empty());
    }

    #[test]
    fn pure_uptrend_yields_at_most_one_bull_mss() {
        // Monotone bull market: at most one MSS (the very first
        // close-through of the first swing high). After that it's
        // BOS continuation.
        let candles: Vec<Candle> = (0..40)
            .map(|i| {
                let mid = 100.0 + 0.1 * i as f64;
                candle(i as i64, mid, mid + 0.2, mid - 0.05, mid + 0.15)
            })
            .collect();
        let out = detect_market_structure_shifts_default(&candles);
        let bull = out
            .iter()
            .filter(|m| m.direction == Direction::Bull)
            .count();
        assert!(
            bull <= 1,
            "monotone uptrend must not emit repeated bull MSS"
        );
    }

    #[test]
    fn down_to_up_reversal_emits_bull_mss_on_close_through() {
        // V-shape: rise → swing high at bar 5 → drop → bull reversal
        // that closes through the swing high.
        let mut candles = vec![
            candle(0, 100.0, 100.5, 99.8, 100.3),
            candle(1, 100.3, 101.0, 100.2, 100.8),
            candle(2, 100.8, 102.0, 100.6, 101.8),
            candle(3, 101.8, 103.0, 101.5, 102.8),
            candle(4, 102.8, 104.0, 102.6, 103.8),
            candle(5, 103.8, 110.0, 103.6, 109.5), // swing high at 110
        ];
        // Bear leg: bar 6..13
        for i in 6..14 {
            let close = 109.5 - 1.0 * (i - 5) as f64;
            let open = close + 1.0;
            candles.push(candle(i as i64, open, open + 0.2, close - 0.2, close));
        }
        // Bull reversal: bar 14..22 — close climbs from 102.5 to 110.5
        for i in 14..22 {
            let close = 101.0 + 1.5 * (i - 13) as f64;
            let open = close - 1.5;
            candles.push(candle(i as i64, open, close + 0.3, open - 0.2, close));
        }
        // Tail
        for i in 22..30 {
            candles.push(candle(i as i64, 113.0, 113.4, 112.5, 113.2));
        }

        let out = detect_market_structure_shifts_default(&candles);
        let bull: Vec<&MarketStructureShift> = out
            .iter()
            .filter(|m| m.direction == Direction::Bull)
            .collect();
        assert!(
            !bull.is_empty(),
            "down-to-up reversal must emit at least one bullish MSS"
        );
        // The first bull MSS bar must close above the broken swing high.
        let first = bull[0];
        assert!(
            candles[first.bar_index].close > first.broken_swing_price,
            "MSS bar must close strictly through the swing"
        );
        assert!((first.broken_swing_price - 110.0).abs() < 1e-6);
    }

    #[test]
    fn wick_through_without_close_does_not_emit() {
        // Swing high at bar 4 = 110, later bars wick to 110 but never
        // close above. Should yield zero MSS.
        let mut candles = vec![
            candle(0, 100.0, 100.5, 99.8, 100.3),
            candle(1, 100.3, 101.0, 100.2, 100.8),
            candle(2, 100.8, 101.5, 100.6, 101.3),
            candle(3, 101.3, 102.0, 101.1, 101.8),
            candle(4, 101.8, 110.0, 101.7, 109.5), // SH wick at 110
        ];
        for i in 5..10 {
            candles.push(candle(i as i64, 109.5, 109.7, 108.0, 108.5));
        }
        // Wick to 110 but close 109.9 — wick-through, not close-through
        candles.push(candle(10, 108.5, 110.0, 108.4, 109.9));
        for i in 11..20 {
            candles.push(candle(i as i64, 109.9, 110.0, 109.0, 109.5));
        }

        let out = detect_market_structure_shifts_default(&candles);
        let bull = out
            .iter()
            .filter(|m| m.direction == Direction::Bull)
            .count();
        assert_eq!(
            bull, 0,
            "wick-through alone must not register as MSS (use bos_choch for that)"
        );
    }
}
