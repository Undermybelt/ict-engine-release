//! Liquidity Void detector.
//!
//! A liquidity void is a 3-bar gap (same geometry as a Fair Value Gap)
//! whose magnitude relative to ATR exceeds `min_gap_atr`. ICT
//! treats LVs as zones the market expects to revisit and rebalance.
//! By definition every LV is also an FVG; the converse is not true.
//!
//! We deliberately do **not** call `detect_fvg` here so the LV
//! detector remains independent: rebuilding the 3-bar geometry is
//! cheap and keeps each detector self-contained for unit testing.

use crate::types::{Candle, Direction, LiquidityVoid};

pub const DEFAULT_LIQUIDITY_VOID_MIN_GAP_ATR: f64 = 0.75;

pub fn detect_liquidity_voids(
    candles: &[Candle],
    atr: &[f64],
    min_gap_atr: f64,
) -> Vec<LiquidityVoid> {
    if candles.len() < 3 || atr.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 1..candles.len() - 1 {
        let prev = &candles[i - 1];
        let next = &candles[i + 1];
        let atr_value = atr_at(atr, i, candles.len());
        if atr_value <= f64::EPSILON {
            continue;
        }

        // Bullish LV: next.low > prev.high
        if next.low > prev.high {
            let gap = next.low - prev.high;
            let gap_atr = gap / atr_value;
            if gap_atr >= min_gap_atr {
                let mut void = LiquidityVoid {
                    top: next.low,
                    bottom: prev.high,
                    direction: Direction::Bull,
                    start_bar: i,
                    gap_atr,
                    filled: false,
                };
                void.filled = is_void_filled(candles, &void);
                out.push(void);
            }
        }

        // Bearish LV: next.high < prev.low
        if next.high < prev.low {
            let gap = prev.low - next.high;
            let gap_atr = gap / atr_value;
            if gap_atr >= min_gap_atr {
                let mut void = LiquidityVoid {
                    top: prev.low,
                    bottom: next.high,
                    direction: Direction::Bear,
                    start_bar: i,
                    gap_atr,
                    filled: false,
                };
                void.filled = is_void_filled(candles, &void);
                out.push(void);
            }
        }
    }
    out
}

pub fn detect_liquidity_voids_default(candles: &[Candle], atr: &[f64]) -> Vec<LiquidityVoid> {
    detect_liquidity_voids(candles, atr, DEFAULT_LIQUIDITY_VOID_MIN_GAP_ATR)
}

pub fn find_unfilled_liquidity_voids(
    candles: &[Candle],
    atr: &[f64],
    min_gap_atr: f64,
) -> Vec<LiquidityVoid> {
    let mut voids = detect_liquidity_voids(candles, atr, min_gap_atr);
    voids.retain(|v| !v.filled);
    voids
}

fn is_void_filled(candles: &[Candle], void: &LiquidityVoid) -> bool {
    // The 3-bar gap spans bars [start_bar - 1, start_bar, start_bar + 1].
    // It is filled the first time price closes past the far edge.
    for candle in candles.iter().skip(void.start_bar + 2) {
        match void.direction {
            Direction::Bull => {
                if candle.low <= void.bottom {
                    return true;
                }
            }
            Direction::Bear => {
                if candle.high >= void.top {
                    return true;
                }
            }
            Direction::Neutral => return false,
        }
    }
    false
}

fn atr_at(atr: &[f64], bar_index: usize, total_bars: usize) -> f64 {
    let offset = total_bars.saturating_sub(atr.len());
    let idx = bar_index.saturating_sub(offset).min(atr.len() - 1);
    atr[idx]
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
    fn empty_input_yields_empty() {
        assert!(detect_liquidity_voids_default(&[], &[]).is_empty());
    }

    #[test]
    fn small_fvg_below_atr_threshold_is_not_a_void() {
        // Tiny 3-bar gap of 0.1 with ATR=1.0 → gap_atr=0.1 < 0.75
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.0),
            candle(1, 100.5, 101.0, 100.4, 100.9),
            candle(2, 100.7, 101.2, 100.6, 101.0),
        ];
        let atr = vec![1.0; 3];
        let voids = detect_liquidity_voids_default(&candles, &atr);
        assert!(voids.is_empty(), "small FVG must not register as LV");
    }

    #[test]
    fn large_bullish_gap_is_detected_as_void() {
        // bar0 high=100.5, bar2 low=102.0 → gap=1.5 with ATR=1.0 → gap_atr=1.5
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.4),
            candle(1, 100.4, 101.0, 100.4, 100.9),
            candle(2, 102.0, 102.5, 102.0, 102.3),
        ];
        let atr = vec![1.0; 3];
        let voids = detect_liquidity_voids_default(&candles, &atr);
        assert_eq!(voids.len(), 1);
        let v = &voids[0];
        assert_eq!(v.direction, Direction::Bull);
        assert_eq!(v.start_bar, 1);
        assert!((v.top - 102.0).abs() < 1e-9);
        assert!((v.bottom - 100.5).abs() < 1e-9);
        assert!((v.gap_atr - 1.5).abs() < 1e-9);
        assert!(!v.filled);
    }

    #[test]
    fn large_bearish_gap_is_detected_as_void() {
        let candles = vec![
            candle(0, 100.0, 100.5, 100.0, 100.0),
            candle(1, 100.0, 100.0, 99.5, 99.6),
            candle(2, 98.0, 98.5, 98.0, 98.2),
        ];
        let atr = vec![1.0; 3];
        let voids = detect_liquidity_voids_default(&candles, &atr);
        assert_eq!(voids.len(), 1);
        let v = &voids[0];
        assert_eq!(v.direction, Direction::Bear);
        assert!((v.top - 100.0).abs() < 1e-9);
        assert!((v.bottom - 98.5).abs() < 1e-9);
    }

    #[test]
    fn fill_check_marks_void_as_filled_when_price_returns() {
        // Bullish LV from bars 0-2; bar 5 returns into the void.
        // Bars 1 and 3 are deliberately wicked to suppress collateral
        // gaps that would otherwise form at bars 2 / 4.
        let candles = vec![
            candle(0, 100.0, 100.5, 99.5, 100.4),
            candle(1, 100.4, 101.0, 100.4, 100.9),
            candle(2, 102.0, 102.5, 102.0, 102.3),
            candle(3, 102.3, 102.5, 100.5, 101.5),
            candle(4, 101.5, 101.5, 101.0, 101.2),
            candle(5, 101.2, 101.2, 99.5, 99.8),
        ];
        let atr = vec![1.0; candles.len()];
        let voids = detect_liquidity_voids_default(&candles, &atr);
        assert_eq!(voids.len(), 1, "fixture should produce exactly one LV");
        assert!(voids[0].filled);
        assert!(
            find_unfilled_liquidity_voids(&candles, &atr, DEFAULT_LIQUIDITY_VOID_MIN_GAP_ATR)
                .is_empty()
        );
    }
}
