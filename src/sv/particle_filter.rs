use super::model::SVModel;
use crate::types::Particle;
use rand::Rng;
use rand_distr::Normal;

/// Particle Filter for Stochastic Volatility estimation
pub struct SVParticleFilter {
    pub n_particles: usize,
    pub particles: Vec<Particle>,
}

impl SVParticleFilter {
    pub fn new(n_particles: usize) -> Self {
        Self {
            n_particles,
            particles: Vec::with_capacity(n_particles),
        }
    }

    /// Initialize particles from prior distribution
    pub fn initialize(&mut self, prior_mean: f64, prior_var: f64) {
        let mut rng = rand::thread_rng();
        let normal = Normal::new(prior_mean, prior_var.sqrt()).unwrap();

        self.particles.clear();
        for _ in 0..self.n_particles {
            let log_var = rng.sample(normal);
            self.particles.push(Particle {
                state: vec![log_var],
                weight: 1.0 / self.n_particles as f64,
            });
        }
    }

    /// Predict step: propagate particles according to SV transition
    pub fn predict(&mut self, model: &SVModel) {
        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, model.sigma_eta).unwrap();

        for particle in &mut self.particles {
            let log_var_old = particle.state[0];
            let log_var_new = model.mu + model.phi * (log_var_old - model.mu) + rng.sample(normal);
            particle.state[0] = log_var_new;
        }
    }

    /// Update step: update weights based on observation
    pub fn update(&mut self, log_return: f64, _model: &SVModel) {
        let mut total_weight = 0.0;

        for particle in &mut self.particles {
            let log_var = particle.state[0];
            let variance = log_var.exp();

            // Gaussian likelihood
            let likelihood = (-0.5 * (log_return * log_return) / variance).exp()
                / (2.0 * std::f64::consts::PI * variance).sqrt();

            particle.weight *= likelihood;
            total_weight += particle.weight;
        }

        // Normalize weights
        if total_weight > 0.0 {
            for particle in &mut self.particles {
                particle.weight /= total_weight;
            }
        }
    }

    /// Resample particles using systematic resampling
    pub fn resample(&mut self) {
        let ess = self.ess();

        // Only resample if ESS < N/2
        if ess >= self.n_particles as f64 / 2.0 {
            return;
        }

        let mut rng = rand::thread_rng();
        let mut new_particles = Vec::with_capacity(self.n_particles);

        // Cumulative weights
        let mut cum_weights = Vec::with_capacity(self.n_particles);
        let mut cum_sum = 0.0;
        for particle in &self.particles {
            cum_sum += particle.weight;
            cum_weights.push(cum_sum);
        }

        // Systematic resampling
        let step = 1.0 / self.n_particles as f64;
        let start: f64 = rng.gen::<f64>() * step;

        let mut i = 0;
        for j in 0..self.n_particles {
            let threshold = start + j as f64 * step;
            while i < cum_weights.len() - 1 && cum_weights[i] < threshold {
                i += 1;
            }

            new_particles.push(Particle {
                state: self.particles[i].state.clone(),
                weight: 1.0 / self.n_particles as f64,
            });
        }

        self.particles = new_particles;
    }

    /// Complete step: predict + update + resample
    /// Returns current implied volatility estimate
    pub fn step(&mut self, log_return: f64, model: &SVModel) -> f64 {
        self.predict(model);
        self.update(log_return, model);
        self.resample();

        // Weighted average of log variance
        let mut weighted_sum = 0.0;
        for particle in &self.particles {
            weighted_sum += particle.state[0] * particle.weight;
        }

        weighted_sum.exp().sqrt() // Return volatility (not variance)
    }

    /// Filter entire series
    pub fn filter_series(&mut self, log_returns: &[f64], model: &SVModel) -> Vec<f64> {
        let mut implied_vols = Vec::with_capacity(log_returns.len());

        for &log_return in log_returns {
            let vol = self.step(log_return, model);
            implied_vols.push(vol);
        }

        implied_vols
    }

    /// Compute Effective Sample Size
    fn ess(&self) -> f64 {
        let sum_sq: f64 = self.particles.iter().map(|p| p.weight * p.weight).sum();
        1.0 / sum_sq
    }
}
