use super::forward_backward::ForwardBackward;
use crate::types::HMMParams;

/// Viterbi algorithm for finding most likely state sequence
pub struct Viterbi;

impl Viterbi {
    /// Decode the most likely state sequence
    /// Returns: (state_sequence, log_likelihood)
    pub fn decode(observations: &[Vec<f64>], params: &HMMParams) -> (Vec<usize>, f64) {
        let t = observations.len();
        let k = params.n_states;

        let mut log_delta: Vec<Vec<f64>> = vec![vec![f64::NEG_INFINITY; k]; t];
        let mut psi: Vec<Vec<usize>> = vec![vec![0; k]; t];

        // Initialization
        for (state, delta) in log_delta[0].iter_mut().enumerate() {
            let log_emit = ForwardBackward::log_emission_prob(
                &observations[0],
                state,
                &params.emission_means,
                &params.emission_stds,
            );
            *delta = params.initial_probs[state].ln() + log_emit;
        }

        // Forward pass
        for tt in 1..t {
            for state in 0..k {
                let log_emit = ForwardBackward::log_emission_prob(
                    &observations[tt],
                    state,
                    &params.emission_means,
                    &params.emission_stds,
                );

                let mut best = f64::NEG_INFINITY;
                let mut best_prev = 0;

                for (prev_state, prev_delta) in log_delta[tt - 1].iter().enumerate() {
                    let val = *prev_delta + params.transition[prev_state][state].ln();
                    if val > best {
                        best = val;
                        best_prev = prev_state;
                    }
                }

                log_delta[tt][state] = best + log_emit;
                psi[tt][state] = best_prev;
            }
        }

        // Backtrack
        let mut path = vec![0; t];
        let mut log_likelihood = f64::NEG_INFINITY;

        for (state, &delta) in log_delta[t - 1].iter().enumerate() {
            if delta > log_likelihood {
                log_likelihood = delta;
                path[t - 1] = state;
            }
        }

        for tt in (0..t - 1).rev() {
            path[tt] = psi[tt + 1][path[tt + 1]];
        }

        (path, log_likelihood)
    }
}
