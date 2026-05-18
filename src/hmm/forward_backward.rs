use crate::types::HMMParams;

/// Forward-Backward algorithm for HMM
pub struct ForwardBackward;

impl ForwardBackward {
    /// Compute single-dimension Gaussian emission probability
    fn _gaussian_emit(x: f64, mean: f64, std: f64) -> f64 {
        let diff = x - mean;
        let exponent = -0.5 * (diff * diff) / (std * std);
        exponent.exp() / (std * (2.0 * std::f64::consts::PI).sqrt())
    }

    /// Compute log emission probability for full observation vector
    pub(crate) fn log_emission_prob(
        obs: &[f64],
        state: usize,
        means: &[Vec<f64>],
        stds: &[Vec<f64>],
    ) -> f64 {
        let mut log_p = 0.0;
        for ((&x, &mean), &std) in obs.iter().zip(means[state].iter()).zip(stds[state].iter()) {
            let diff = x - mean;
            log_p +=
                -0.5 * (diff / std).powi(2) - std.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln();
        }
        log_p
    }

    /// Forward algorithm (log space)
    /// Returns: log_alpha[T x K] and log_likelihood
    pub fn forward(observations: &[Vec<f64>], params: &HMMParams) -> (Vec<Vec<f64>>, f64) {
        let t = observations.len();
        let k = params.n_states;

        let mut log_alpha: Vec<Vec<f64>> = vec![vec![f64::NEG_INFINITY; k]; t];

        // Initialization
        for (state, alpha) in log_alpha[0].iter_mut().enumerate() {
            let log_emit = Self::log_emission_prob(
                &observations[0],
                state,
                &params.emission_means,
                &params.emission_stds,
            );
            *alpha = params.initial_probs[state].ln() + log_emit;
        }

        // Forward pass
        for tt in 1..t {
            for state in 0..k {
                let log_emit = Self::log_emission_prob(
                    &observations[tt],
                    state,
                    &params.emission_means,
                    &params.emission_stds,
                );

                let mut log_sum = f64::NEG_INFINITY;
                for (prev_state, prev_alpha) in log_alpha[tt - 1].iter().enumerate() {
                    let log_val = *prev_alpha + params.transition[prev_state][state].ln();
                    log_sum = Self::log_sum_exp(&[log_sum, log_val]);
                }

                log_alpha[tt][state] = log_sum + log_emit;
            }
        }

        // Log likelihood
        let log_likelihood = Self::log_sum_exp(&log_alpha[t - 1]);

        (log_alpha, log_likelihood)
    }

    /// Backward algorithm (log space)
    /// Returns: log_beta[T x K]
    pub fn backward(observations: &[Vec<f64>], params: &HMMParams) -> Vec<Vec<f64>> {
        let t = observations.len();
        let k = params.n_states;

        let mut log_beta: Vec<Vec<f64>> = vec![vec![f64::NEG_INFINITY; k]; t];

        // Initialization (last time step)
        for beta in &mut log_beta[t - 1] {
            *beta = 0.0; // log(1) = 0
        }

        // Backward pass
        for tt in (0..t - 1).rev() {
            for state in 0..k {
                let mut log_sum = f64::NEG_INFINITY;

                for (next_state, next_beta) in log_beta[tt + 1].iter().enumerate() {
                    let log_emit = Self::log_emission_prob(
                        &observations[tt + 1],
                        next_state,
                        &params.emission_means,
                        &params.emission_stds,
                    );

                    let log_val = params.transition[state][next_state].ln() + log_emit + *next_beta;
                    log_sum = Self::log_sum_exp(&[log_sum, log_val]);
                }

                log_beta[tt][state] = log_sum;
            }
        }

        log_beta
    }

    /// Compute gamma (state posterior probabilities)
    /// gamma[t][k] = P(S_t=k | O, lambda)
    pub fn compute_gamma(
        log_alpha: &[Vec<f64>],
        log_beta: &[Vec<f64>],
        log_likelihood: f64,
    ) -> Vec<Vec<f64>> {
        let t = log_alpha.len();
        let k = log_alpha[0].len();

        let mut gamma = vec![vec![0.0; k]; t];

        for tt in 0..t {
            for state in 0..k {
                gamma[tt][state] = log_alpha[tt][state] + log_beta[tt][state] - log_likelihood;
            }
        }

        gamma
    }

    /// Compute xi (state transition posterior probabilities)
    /// xi[t][i][j] = P(S_t=i, S_{t+1}=j | O, lambda)
    pub fn compute_xi(
        log_alpha: &[Vec<f64>],
        log_beta: &[Vec<f64>],
        observations: &[Vec<f64>],
        params: &HMMParams,
        log_likelihood: f64,
    ) -> Vec<Vec<Vec<f64>>> {
        let t = observations.len();
        let k = params.n_states;

        let mut xi = vec![vec![vec![f64::NEG_INFINITY; k]; k]; t - 1];

        for tt in 0..t - 1 {
            for i in 0..k {
                for j in 0..k {
                    let log_emit = Self::log_emission_prob(
                        &observations[tt + 1],
                        j,
                        &params.emission_means,
                        &params.emission_stds,
                    );

                    xi[tt][i][j] = log_alpha[tt][i]
                        + params.transition[i][j].ln()
                        + log_emit
                        + log_beta[tt + 1][j]
                        - log_likelihood;
                }
            }
        }

        xi
    }

    /// Log-sum-exp for numerical stability
    pub fn log_sum_exp(log_values: &[f64]) -> f64 {
        let max_val = log_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        if max_val == f64::NEG_INFINITY {
            return f64::NEG_INFINITY;
        }

        let sum: f64 = log_values.iter().map(|&x| (x - max_val).exp()).sum();
        max_val + sum.ln()
    }
}
