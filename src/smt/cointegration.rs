/// Cointegration testing
pub struct Cointegration;

impl Cointegration {
    /// Engle-Granger two-step cointegration test
    pub fn engle_granger(x: &[f64], y: &[f64]) -> (f64, bool) {
        if x.len() != y.len() || x.len() < 10 {
            return (0.0, false);
        }

        // Step 1: OLS regression y = alpha + beta * x
        let n = x.len() as f64;
        let x_mean = x.iter().sum::<f64>() / n;
        let y_mean = y.iter().sum::<f64>() / n;

        let mut xy = 0.0;
        let mut xx = 0.0;
        for i in 0..x.len() {
            xy += (x[i] - x_mean) * (y[i] - y_mean);
            xx += (x[i] - x_mean) * (x[i] - x_mean);
        }

        let beta = if xx.abs() > 1e-10 { xy / xx } else { 0.0 };
        let alpha = y_mean - beta * x_mean;

        // Step 2: Calculate residuals
        let residuals: Vec<f64> = x
            .iter()
            .zip(y.iter())
            .map(|(&xi, &yi)| yi - (alpha + beta * xi))
            .collect();

        // Simplified ADF test (in practice, use proper statistical test)
        let resid_mean = residuals.iter().sum::<f64>() / n;
        let resid_var = residuals
            .iter()
            .map(|&r| (r - resid_mean).powi(2))
            .sum::<f64>()
            / n;

        let test_stat = if resid_var > 0.0 {
            resid_mean / resid_var.sqrt()
        } else {
            0.0
        };

        // Simplified: cointegrated if test_stat < -2.5
        (test_stat, test_stat < -2.5)
    }
}
