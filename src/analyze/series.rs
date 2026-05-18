use crate::types::Candle;

pub fn aligned_close_series(
    futures_candles: &[Candle],
    spot_candles: &[Candle],
) -> (Vec<f64>, Vec<f64>) {
    let len = futures_candles.len().min(spot_candles.len());
    let futures = futures_candles[futures_candles.len().saturating_sub(len)..]
        .iter()
        .map(|candle| candle.close)
        .collect();
    let spot = spot_candles[spot_candles.len().saturating_sub(len)..]
        .iter()
        .map(|candle| candle.close)
        .collect();
    (futures, spot)
}

pub fn close_to_returns(closes: &[f64]) -> Vec<f64> {
    closes
        .windows(2)
        .filter_map(|window| {
            let prev = window[0];
            let next = window[1];
            if prev.abs() <= f64::EPSILON {
                None
            } else {
                Some((next - prev) / prev)
            }
        })
        .collect()
}
