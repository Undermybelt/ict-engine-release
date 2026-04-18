/// Rolling correlation calculator
pub struct Correlation;

impl Correlation {
    /// Calculate rolling correlation
    pub fn rolling(x: &[f64], y: &[f64], window: usize) -> Vec<f64> {
        if x.len() != y.len() || x.len() < window {
            return Vec::new();
        }

        let mut correlations = Vec::new();

        for i in window..=x.len() {
            let x_window = &x[i - window..i];
            let y_window = &y[i - window..i];

            correlations.push(Self::pearson(x_window, y_window));
        }

        correlations
    }

    /// Pearson correlation coefficient
    pub fn pearson(x: &[f64], y: &[f64]) -> f64 {
        let n = x.len() as f64;
        let x_mean = x.iter().sum::<f64>() / n;
        let y_mean = y.iter().sum::<f64>() / n;

        let mut xy = 0.0;
        let mut xx = 0.0;
        let mut yy = 0.0;

        for i in 0..x.len() {
            let dx = x[i] - x_mean;
            let dy = y[i] - y_mean;
            xy += dx * dy;
            xx += dx * dx;
            yy += dy * dy;
        }

        let denom = (xx * yy).sqrt();
        if denom > 0.0 {
            xy / denom
        } else {
            0.0
        }
    }
}
