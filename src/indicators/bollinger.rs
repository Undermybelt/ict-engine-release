use crate::{
    math::{sma, std_dev},
    types::Candle,
};

#[derive(Debug, Clone)]
pub struct BollingerBands {
    pub upper: Vec<f64>,
    pub middle: Vec<f64>,
    pub lower: Vec<f64>,
}

/// Compute Bollinger Bands
pub fn compute_bollinger(candles: &[Candle], period: usize, num_std: f64) -> BollingerBands {
    let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let middle = sma(&prices, period);

    if middle.is_empty() {
        return BollingerBands {
            upper: Vec::new(),
            middle: Vec::new(),
            lower: Vec::new(),
        };
    }

    let mut upper = Vec::new();
    let mut lower = Vec::new();
    for i in 0..middle.len() {
        let window = &prices[i..i + period];
        let std = std_dev(window, middle[i]);

        upper.push(middle[i] + num_std * std);
        lower.push(middle[i] - num_std * std);
    }

    BollingerBands {
        upper,
        middle,
        lower,
    }
}

/// Get the latest Bollinger Bands values
pub fn latest_bollinger(
    candles: &[Candle],
    period: usize,
    num_std: f64,
) -> Option<(f64, f64, f64)> {
    let bands = compute_bollinger(candles, period, num_std);
    Some((
        *bands.upper.last()?,
        *bands.middle.last()?,
        *bands.lower.last()?,
    ))
}

/// Check if price is squeezing (bands are narrow)
pub fn is_squeeze(candles: &[Candle], period: usize, num_std: f64, threshold: f64) -> bool {
    let bands = compute_bollinger(candles, period, num_std);
    let (Some(last_upper), Some(last_lower), Some(last_middle)) =
        (bands.upper.last(), bands.lower.last(), bands.middle.last())
    else {
        return false;
    };
    let band_width = (last_upper - last_lower) / last_middle;

    band_width < threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|index| Candle {
                timestamp: Utc::now(),
                open: 100.0 + index as f64,
                high: 101.0 + index as f64,
                low: 99.0 + index as f64,
                close: 100.5 + index as f64,
                volume: 1_000.0,
            })
            .collect()
    }

    #[test]
    fn test_compute_bollinger_no_out_of_bounds() {
        let bands = compute_bollinger(&candles(50), 20, 2.0);
        assert_eq!(bands.middle.len(), 31);
        assert_eq!(bands.upper.len(), 31);
        assert_eq!(bands.lower.len(), 31);
    }
}
