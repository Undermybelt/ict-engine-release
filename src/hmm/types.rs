use crate::types::HMMParams;

/// Initialize HMM with 3 states for regime detection
/// States: 0 = Accumulation, 1 = ManipulationExpansion, 2 = Distribution
pub fn init_hmm_params(obs_dim: usize) -> HMMParams {
    HMMParams::new_3state(obs_dim)
}

/// Get state name
pub fn state_name(state: usize) -> &'static str {
    match state {
        0 => "Accumulation",
        1 => "ManipulationExpansion",
        2 => "Distribution",
        _ => "Unknown",
    }
}
