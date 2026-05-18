/// Metropolis-Hastings sampler
pub struct MetropolisHastings {
    pub n_samples: usize,
    pub burn_in: usize,
    pub step_size: f64,
}

impl MetropolisHastings {
    pub fn new(n_samples: usize, burn_in: usize, step_size: f64) -> Self {
        Self {
            n_samples,
            burn_in,
            step_size,
        }
    }

    /// Sample from target distribution
    pub fn sample<F>(&self, log_target: F, initial: Vec<f64>) -> Vec<Vec<f64>>
    where
        F: Fn(&[f64]) -> f64,
    {
        let dim = initial.len();
        let mut samples = Vec::with_capacity(self.n_samples);
        let mut current = initial;
        let mut current_ll = log_target(&current);

        for i in 0..self.n_samples + self.burn_in {
            // Propose: random walk
            let mut candidate = current.clone();
            for value in candidate.iter_mut().take(dim) {
                let noise: f64 = rand::random::<f64>() * 2.0 - 1.0;
                *value += self.step_size * noise;
            }

            let candidate_ll = log_target(&candidate);
            let log_alpha = candidate_ll - current_ll;

            if log_alpha > 0.0 || rand::random::<f64>().ln() < log_alpha {
                current = candidate;
                current_ll = candidate_ll;
            }

            if i >= self.burn_in {
                samples.push(current.clone());
            }
        }

        samples
    }
}
