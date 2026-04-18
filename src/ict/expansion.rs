use crate::indicators::atr::compute_atr;
use crate::types::{Candle, Direction};

/// Check if a bullish expansion exists
pub fn check_bull_expansion_exists(
    candles: &[Candle],
    lookback: usize,
    atr_multiplier: f64,
) -> bool {
    if candles.len() < lookback {
        return false;
    }

    let atr = compute_atr(candles, 14);
    if atr.is_empty() {
        return false;
    }

    let current_atr = *atr.last().unwrap();
    let start_idx = candles.len() - lookback;

    // Find the lowest low in the lookback period
    let lowest_low = candles[start_idx..]
        .iter()
        .map(|c| c.low)
        .fold(f64::INFINITY, f64::min);

    // Check if current price has moved significantly above the low
    let current_price = candles.last().unwrap().close;
    let move_size = current_price - lowest_low;

    move_size > current_atr * atr_multiplier
}

/// Check if a bearish expansion exists
pub fn check_bear_expansion_exists(
    candles: &[Candle],
    lookback: usize,
    atr_multiplier: f64,
) -> bool {
    if candles.len() < lookback {
        return false;
    }

    let atr = compute_atr(candles, 14);
    if atr.is_empty() {
        return false;
    }

    let current_atr = *atr.last().unwrap();
    let start_idx = candles.len() - lookback;

    // Find the highest high in the lookback period
    let highest_high = candles[start_idx..]
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);

    // Check if current price has moved significantly below the high
    let current_price = candles.last().unwrap().close;
    let move_size = highest_high - current_price;

    move_size > current_atr * atr_multiplier
}

/// Check if expansion exists in either direction
pub fn check_expansion_exists(
    candles: &[Candle],
    lookback: usize,
    atr_multiplier: f64,
) -> Option<Direction> {
    if check_bull_expansion_exists(candles, lookback, atr_multiplier) {
        Some(Direction::Bull)
    } else if check_bear_expansion_exists(candles, lookback, atr_multiplier) {
        Some(Direction::Bear)
    } else {
        None
    }
}

/// Calculate expansion strength (ratio of move to ATR)
pub fn expansion_strength(candles: &[Candle], lookback: usize) -> f64 {
    if candles.len() < lookback {
        return 0.0;
    }

    let atr = compute_atr(candles, 14);
    if atr.is_empty() {
        return 0.0;
    }

    let current_atr = *atr.last().unwrap();
    let start_idx = candles.len() - lookback;

    let highest_high = candles[start_idx..]
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);

    let lowest_low = candles[start_idx..]
        .iter()
        .map(|c| c.low)
        .fold(f64::INFINITY, f64::min);

    let range = highest_high - lowest_low;
    range / current_atr
}
