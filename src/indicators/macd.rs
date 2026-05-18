use crate::{math::ema, types::Candle};

#[derive(Debug, Clone)]
pub struct MACD {
    pub macd_line: Vec<f64>,
    pub signal_line: Vec<f64>,
    pub histogram: Vec<f64>,
}

/// Compute MACD (12, 26, 9)
pub fn compute_macd(candles: &[Candle], fast: usize, slow: usize, signal: usize) -> MACD {
    let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();

    let ema_fast = ema(&prices, fast);
    let ema_slow = ema(&prices, slow);

    if ema_fast.is_empty() || ema_slow.is_empty() {
        return MACD {
            macd_line: Vec::new(),
            signal_line: Vec::new(),
            histogram: Vec::new(),
        };
    }

    // Align the EMAs (slow EMA starts later)
    let offset = slow - fast;
    let macd_line: Vec<f64> = ema_fast[offset..]
        .iter()
        .zip(ema_slow.iter())
        .map(|(fast, slow)| fast - slow)
        .collect();

    if macd_line.len() < signal {
        return MACD {
            macd_line,
            signal_line: Vec::new(),
            histogram: Vec::new(),
        };
    }

    // Signal line is EMA of MACD line
    let signal_line = ema(&macd_line, signal);
    if signal_line.is_empty() {
        return MACD {
            macd_line,
            signal_line,
            histogram: Vec::new(),
        };
    }

    // Histogram is MACD - Signal
    let signal_offset = signal - 1;
    let histogram: Vec<f64> = macd_line[signal_offset..]
        .iter()
        .zip(signal_line.iter())
        .map(|(macd, signal)| macd - signal)
        .collect();

    MACD {
        macd_line,
        signal_line,
        histogram,
    }
}

/// Get the latest MACD values
pub fn latest_macd(
    candles: &[Candle],
    fast: usize,
    slow: usize,
    signal: usize,
) -> Option<(f64, f64, f64)> {
    let macd = compute_macd(candles, fast, slow, signal);
    let macd_line = *macd.macd_line.last()?;
    let signal_line = *macd.signal_line.last()?;
    let histogram = *macd.histogram.last()?;
    Some((macd_line, signal_line, histogram))
}

/// Check if MACD histogram is bullish (positive and increasing)
pub fn is_macd_bullish(candles: &[Candle], fast: usize, slow: usize, signal: usize) -> bool {
    let macd = compute_macd(candles, fast, slow, signal);
    let Some(last) = macd.histogram.last().copied() else {
        return false;
    };
    let Some(prev) = macd.histogram.iter().rev().nth(1).copied() else {
        return false;
    };

    last > 0.0 && last > prev
}

/// Check if MACD histogram is bearish (negative and decreasing)
pub fn is_macd_bearish(candles: &[Candle], fast: usize, slow: usize, signal: usize) -> bool {
    let macd = compute_macd(candles, fast, slow, signal);
    let Some(last) = macd.histogram.last().copied() else {
        return false;
    };
    let Some(prev) = macd.histogram.iter().rev().nth(1).copied() else {
        return false;
    };

    last < 0.0 && last < prev
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn candle(close: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn latest_macd_returns_none_for_short_windows() {
        let candles = vec![candle(1.0), candle(1.1), candle(1.2), candle(1.3)];
        assert!(latest_macd(&candles, 12, 26, 9).is_none());
        assert!(!is_macd_bullish(&candles, 12, 26, 9));
        assert!(!is_macd_bearish(&candles, 12, 26, 9));
    }
}
