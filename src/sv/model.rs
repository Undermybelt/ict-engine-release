use rand::Rng;
use rand_distr::Normal;

/// Stochastic Volatility Model
/// log(sigma^2_t) = mu + phi * log(sigma^2_{t-1}) + eta_t, eta_t ~ N(0, sigma_eta^2)
/// r_t = sigma_t * epsilon_t, epsilon_t ~ N(0, 1)
pub struct SVModel {
    pub mu: f64,        // Long-term mean
    pub phi: f64,       // Persistence parameter
    pub sigma_eta: f64, // Volatility of volatility
}

impl SVModel {
    pub fn new(mu: f64, phi: f64, sigma_eta: f64) -> Self {
        Self { mu, phi, sigma_eta }
    }

    /// Fit SV model using simplified MCMC (Gibbs sampling)
    pub fn fit(&mut self, log_returns: &[f64], n_iter: usize) {
        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, 1.0).unwrap();

        // Initialize latent log variances
        let mut log_variances: Vec<f64> = log_returns
            .iter()
            .map(|_| self.mu + rng.sample(normal) * self.sigma_eta)
            .collect();

        for _ in 0..n_iter {
            // Sample log variances given parameters
            self.sample_log_variances(log_returns, &mut log_variances, &mut rng);

            // Update parameters given log variances
            self.update_parameters(&log_variances);
        }
    }

    /// Sample log variances using particle filter (simplified)
    fn sample_log_variances(
        &self,
        log_returns: &[f64],
        log_variances: &mut [f64],
        rng: &mut impl Rng,
    ) {
        let normal = Normal::new(0.0, 1.0).unwrap();

        for t in 1..log_variances.len() {
            // Prior: log_var_t ~ N(mu + phi * (log_var_{t-1} - mu), sigma_eta^2)
            let prior_mean = self.mu + self.phi * (log_variances[t - 1] - self.mu);
            let prior_var = self.sigma_eta * self.sigma_eta;

            // Likelihood: r_t ~ N(0, exp(log_var_t))
            let r_t = log_returns[t];
            let r_t_sq = r_t * r_t;

            // Simplified posterior sampling (Gaussian approximation)
            let likelihood_var = 0.5 * (1.0 / prior_var + r_t_sq / prior_var).recip();
            let likelihood_mean = likelihood_var * (prior_mean / prior_var);

            // Sample
            log_variances[t] = likelihood_mean + rng.sample(normal) * likelihood_var.sqrt();
        }
    }

    /// Update model parameters using OLS on log variances
    fn update_parameters(&mut self, log_variances: &[f64]) {
        if log_variances.len() < 2 {
            return;
        }

        // Estimate mu and phi using OLS
        let n = log_variances.len() - 1;
        let x: Vec<f64> = log_variances[..n].iter().map(|&v| v - self.mu).collect();
        let y: Vec<f64> = log_variances[1..].iter().map(|&v| v - self.mu).collect();

        let x_mean = x.iter().sum::<f64>() / n as f64;
        let y_mean = y.iter().sum::<f64>() / n as f64;

        let mut xy = 0.0;
        let mut xx = 0.0;

        for i in 0..n {
            xy += (x[i] - x_mean) * (y[i] - y_mean);
            xx += (x[i] - x_mean) * (x[i] - x_mean);
        }

        if xx.abs() > 1e-10 {
            self.phi = (xy / xx).clamp(-0.99, 0.99);
        }

        // Update sigma_eta from residuals
        let mut residuals = Vec::new();
        for i in 1..log_variances.len() {
            let pred = self.mu + self.phi * (log_variances[i - 1] - self.mu);
            residuals.push(log_variances[i] - pred);
        }

        if !residuals.is_empty() {
            let mean_resid = residuals.iter().sum::<f64>() / residuals.len() as f64;
            let var_resid = residuals
                .iter()
                .map(|&r| (r - mean_resid).powi(2))
                .sum::<f64>()
                / residuals.len() as f64;
            self.sigma_eta = var_resid.sqrt().max(1e-10);
        }
    }

    /// Compute log-likelihood
    pub fn log_likelihood(&self, log_returns: &[f64], log_variances: &[f64]) -> f64 {
        let mut ll = 0.0;

        for (r, &log_var) in log_returns.iter().zip(log_variances.iter()) {
            let var = log_var.exp();
            ll += -0.5 * (2.0 * std::f64::consts::PI).ln() - 0.5 * log_var - (r * r) / (2.0 * var);
        }

        ll
    }

    /// Transition log probability
    pub fn transition_log_prob(&self, log_var_prev: f64, log_var_curr: f64) -> f64 {
        let mean = self.mu + self.phi * (log_var_prev - self.mu);
        let diff = log_var_curr - mean;
        -0.5 * (2.0 * std::f64::consts::PI).ln()
            - self.sigma_eta.ln()
            - (diff * diff) / (2.0 * self.sigma_eta * self.sigma_eta)
    }
}
