use crate::types::RegimeProbs;

/// Bayesian fusion of regime prior and downstream trade probability.
/// ICT/cascade structures should influence win probability upstream as evidence,
/// not act as deterministic trade triggers here.
pub struct BayesianFusion;

impl BayesianFusion {
    /// Combine manipulation/expansion prior with trade win probability.
    pub fn fuse_trade_probability(regime_probs: &RegimeProbs, win_probability: f64) -> f64 {
        regime_probs.manipulation_expansion * win_probability.clamp(0.0, 1.0)
    }

    /// Determine if a trade should be taken
    pub fn should_trade(
        bull_combined: f64,
        bear_combined: f64,
        threshold: f64,
    ) -> Option<crate::types::Direction> {
        if bull_combined > threshold && bull_combined > bear_combined {
            Some(crate::types::Direction::Bull)
        } else if bear_combined > threshold {
            Some(crate::types::Direction::Bear)
        } else {
            None
        }
    }
}
