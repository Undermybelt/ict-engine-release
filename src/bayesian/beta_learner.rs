use crate::types::BetaParams;
use serde::{Deserialize, Serialize};

/// Beta distribution online learner for cascade layer likelihood ratios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeBetaLearner {
    pub bull: Vec<BetaParams>,
    pub bear: Vec<BetaParams>,
    pub base_rate: f64,
}

impl CascadeBetaLearner {
    pub fn new(num_layers: usize, base_rate: f64) -> Self {
        Self {
            bull: vec![
                BetaParams {
                    alpha: 1.0,
                    beta: 1.0
                };
                num_layers
            ],
            bear: vec![
                BetaParams {
                    alpha: 1.0,
                    beta: 1.0
                };
                num_layers
            ],
            base_rate,
        }
    }

    /// Update based on trade result
    pub fn update(
        &mut self,
        direction: crate::types::Direction,
        layer_results: &[bool],
        success: bool,
    ) {
        let params = match direction {
            crate::types::Direction::Bull => &mut self.bull,
            crate::types::Direction::Bear => &mut self.bear,
            _ => return,
        };

        for (i, &satisfied) in layer_results.iter().enumerate() {
            if i < params.len() && satisfied {
                params[i].update(success);
            }
        }
    }

    /// Get current LR estimates
    pub fn get_lr_estimates(&self, direction: crate::types::Direction) -> Vec<f64> {
        let params = match direction {
            crate::types::Direction::Bull => &self.bull,
            crate::types::Direction::Bear => &self.bear,
            _ => return vec![1.0; 7],
        };

        params
            .iter()
            .map(|p| p.lr_estimate(self.base_rate))
            .collect()
    }
}
