use crate::{math, types::Candle};

/// Compute Exponential Moving Average on close prices
pub fn compute_ema(candles: &[Candle], period: usize) -> Vec<f64> {
    let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();
    math::ema(&prices, period)
}

/// Get the latest EMA value
pub fn latest_ema(candles: &[Candle], period: usize) -> f64 {
    let ema = compute_ema(candles, period);
    ema.last().copied().unwrap_or(0.0)
}

/// Check if price is above EMA
pub fn price_above_ema(candles: &[Candle], period: usize) -> bool {
    let ema = compute_ema(candles, period);
    if let (Some(&last_ema), Some(last_candle)) = (ema.last(), candles.last()) {
        last_candle.close > last_ema
    } else {
        false
    }
}
