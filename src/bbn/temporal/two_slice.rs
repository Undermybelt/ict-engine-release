use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransitionKernel {
    pub regime_key: String,
    pub transition_bias: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemporalBeliefModel2Slice {
    pub active_regime_t: String,
    pub active_regime_t1: String,
    pub transition_kernel: TransitionKernel,
}

impl TemporalBeliefModel2Slice {
    pub fn new(active_regime: &str) -> Self {
        Self {
            active_regime_t: active_regime.to_string(),
            active_regime_t1: active_regime.to_string(),
            transition_kernel: TransitionKernel {
                regime_key: active_regime.to_string(),
                transition_bias: 0.5,
            },
        }
    }
}
