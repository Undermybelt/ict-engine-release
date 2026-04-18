use crate::{math::wilder_smooth, types::Candle};

/// Compute Average True Range (ATR) using Wilder's smoothing
pub fn compute_atr(candles: &[Candle], period: usize) -> Vec<f64> {
    if candles.len() < 2 {
        return Vec::new();
    }

    // Calculate True Range for each candle
    let tr_values: Vec<f64> = candles.windows(2).map(|w| w[1].true_range(&w[0])).collect();

    // Apply Wilder's smoothing
    wilder_smooth(&tr_values, period)
}

/// Get the latest ATR value
pub fn latest_atr(candles: &[Candle], period: usize) -> f64 {
    let atr = compute_atr(candles, period);
    atr.last().copied().unwrap_or(0.0)
}

/// Get ATR as percentage of close price
pub fn atr_percent(candles: &[Candle], period: usize) -> Vec<f64> {
    let atr = compute_atr(candles, period);
    let start_idx = candles.len() - atr.len();

    atr.iter()
        .enumerate()
        .map(|(i, &a)| a / candles[start_idx + i].close * 100.0)
        .collect()
}
