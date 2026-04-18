/// SMT divergence detection
pub struct Divergence;

impl Divergence {
    /// Detect divergence between two series
    pub fn detect(x: &[f64], y: &[f64], lookback: usize) -> Vec<bool> {
        if x.len() != y.len() || x.len() < lookback {
            return Vec::new();
        }

        let mut divergences = vec![false; x.len()];

        for i in lookback..x.len() {
            let x_window = &x[i - lookback..=i];
            let y_window = &y[i - lookback..=i];

            // Check if price made new high/low but other didn't confirm
            let x_high = x_window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let x_low = x_window.iter().cloned().fold(f64::INFINITY, f64::min);
            let y_high = y_window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let y_low = y_window.iter().cloned().fold(f64::INFINITY, f64::min);

            // Bullish divergence: x makes lower low, y makes higher low
            // Bearish divergence: x makes higher high, y makes lower high
            let bullish_div = x[i] <= x_low && y[i] > y_low;
            let bearish_div = x[i] >= x_high && y[i] < y_high;

            divergences[i] = bullish_div || bearish_div;
        }

        divergences
    }
}
