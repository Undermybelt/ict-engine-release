use crate::types::Candle;

/// Classify market as premium, discount, or equilibrium
pub fn classify_premium_discount(candles: &[Candle], lookback: usize) -> (bool, bool, bool) {
    if candles.len() < lookback {
        return (false, false, false);
    }

    let start_idx = candles.len() - lookback;
    let range = &candles[start_idx..];

    // Find swing high and low
    let high = range
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = range.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let midpoint = (high + low) / 2.0;

    let Some(current_price) = candles.last().map(|candle| candle.close) else {
        return (false, false, false);
    };

    // Premium: above midpoint
    let is_premium = current_price > midpoint;

    // Discount: below midpoint
    let is_discount = current_price < midpoint;

    // Equilibrium: near midpoint (within 20% of range)
    let range_size = high - low;
    let is_equilibrium = (current_price - midpoint).abs() < range_size * 0.2;

    (is_premium, is_discount, is_equilibrium)
}
