use crate::types::HawkesParams;

/// Hawkes Process for event clustering
pub struct HawkesProcess {
    pub params: HawkesParams,
    pub events: Vec<f64>,
}

impl HawkesProcess {
    pub fn new(mu: f64, alpha: f64, beta: f64) -> Self {
        Self {
            params: HawkesParams { mu, alpha, beta },
            events: Vec::new(),
        }
    }

    /// Calculate intensity at time t
    pub fn intensity_at(&self, t: f64) -> f64 {
        let mut lam = self.params.mu;
        for &t_i in &self.events {
            if t_i < t {
                lam += self.params.alpha * (-self.params.beta * (t - t_i)).exp();
            }
        }
        lam
    }

    /// Incremental intensity calculation (O(1))
    pub fn intensity_incremental(lambda_prev: f64, delta_t: f64, params: &HawkesParams) -> f64 {
        (lambda_prev - params.mu) * (-params.beta * delta_t).exp() + params.mu + params.alpha
    }

    /// Branching ratio
    pub fn branching_ratio(&self) -> f64 {
        self.params.alpha / self.params.beta
    }

    /// Check if stationary
    pub fn is_stationary(&self) -> bool {
        self.params.alpha < self.params.beta
    }
}
