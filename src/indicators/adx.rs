use crate::{math::wilder_smooth, types::Candle};

/// Compute Average Directional Index (ADX)
pub fn compute_adx(candles: &[Candle], period: usize) -> Vec<f64> {
    if candles.len() < period + 1 {
        return Vec::new();
    }

    let mut plus_dm = Vec::new();
    let mut minus_dm = Vec::new();
    let mut tr = Vec::new();

    // Calculate +DM, -DM, and TR
    for i in 1..candles.len() {
        let up_move = candles[i].high - candles[i - 1].high;
        let down_move = candles[i - 1].low - candles[i].low;

        let pdm = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        let mdm = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };

        plus_dm.push(pdm);
        minus_dm.push(mdm);
        tr.push(candles[i].true_range(&candles[i - 1]));
    }

    // Smooth the values
    let smooth_plus_dm = wilder_smooth(&plus_dm, period);
    let smooth_minus_dm = wilder_smooth(&minus_dm, period);
    let smooth_tr = wilder_smooth(&tr, period);

    if smooth_plus_dm.is_empty() {
        return Vec::new();
    }

    let _start_idx = period - 1;

    // Calculate +DI and -DI
    let mut plus_di = Vec::new();
    let mut minus_di = Vec::new();
    let mut dx_values = Vec::new();

    for i in 0..smooth_plus_dm.len() {
        let tr_val = smooth_tr[i];
        if tr_val == 0.0 {
            plus_di.push(0.0);
            minus_di.push(0.0);
            dx_values.push(0.0);
        } else {
            let pdi = smooth_plus_dm[i] / tr_val * 100.0;
            let mdi = smooth_minus_dm[i] / tr_val * 100.0;

            plus_di.push(pdi);
            minus_di.push(mdi);

            let sum = pdi + mdi;
            if sum == 0.0 {
                dx_values.push(0.0);
            } else {
                dx_values.push((pdi - mdi).abs() / sum * 100.0);
            }
        }
    }

    // Calculate ADX from DX
    if dx_values.len() < period {
        return Vec::new();
    }

    let mut adx_values = Vec::new();

    // First ADX is average of first period DX values
    let first_adx: f64 = dx_values[..period].iter().sum::<f64>() / period as f64;
    adx_values.push(first_adx);

    // Subsequent ADX values use Wilder's smoothing
    for value in dx_values.iter().skip(period) {
        let prev_adx = adx_values.last().unwrap();
        let adx = (prev_adx * (period - 1) as f64 + *value) / period as f64;
        adx_values.push(adx);
    }

    adx_values
}

/// Get the latest ADX value
pub fn latest_adx(candles: &[Candle], period: usize) -> f64 {
    let adx = compute_adx(candles, period);
    adx.last().copied().unwrap_or(0.0)
}
