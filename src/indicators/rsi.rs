use crate::types::Candle;

/// Compute Relative Strength Index (RSI) using Wilder's smoothing
pub fn compute_rsi(candles: &[Candle], period: usize) -> Vec<f64> {
    if candles.len() < period + 1 {
        return Vec::new();
    }

    let mut gains = Vec::new();
    let mut losses = Vec::new();

    // Calculate price changes
    for i in 1..candles.len() {
        let change = candles[i].close - candles[i - 1].close;
        if change >= 0.0 {
            gains.push(change);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(-change);
        }
    }

    // Calculate initial average gain and loss (SMA)
    let mut avg_gain: f64 = gains[..period].iter().sum::<f64>() / period as f64;
    let mut avg_loss: f64 = losses[..period].iter().sum::<f64>() / period as f64;

    let mut rsi_values = Vec::new();

    // First RSI
    if avg_loss == 0.0 {
        rsi_values.push(100.0);
    } else {
        let rs = avg_gain / avg_loss;
        rsi_values.push(100.0 - (100.0 / (1.0 + rs)));
    }

    // Subsequent RSI values using Wilder's smoothing
    for i in period..gains.len() {
        avg_gain = (avg_gain * (period - 1) as f64 + gains[i]) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + losses[i]) / period as f64;

        if avg_loss == 0.0 {
            rsi_values.push(100.0);
        } else {
            let rs = avg_gain / avg_loss;
            rsi_values.push(100.0 - (100.0 / (1.0 + rs)));
        }
    }

    rsi_values
}

/// Get the latest RSI value
pub fn latest_rsi(candles: &[Candle], period: usize) -> f64 {
    let rsi = compute_rsi(candles, period);
    rsi.last().copied().unwrap_or(50.0)
}
