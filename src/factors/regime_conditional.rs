use crate::state::{regime_key, FactorLearningProfile, RegimeFactorStats};
use crate::types::Regime;
use serde::{Deserialize, Serialize};

/// Parameters for the 2-input EML fusion surface used in experiment-only regime scoring.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EmlParams {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub delta: f64,
    pub epsilon: f64,
}

impl Default for EmlParams {
    fn default() -> Self {
        Self {
            alpha: 0.55,
            beta: 0.80,
            gamma: 0.35,
            delta: 2.00,
            epsilon: 1e-3,
        }
    }
}

/// 2-input EML regime score kept only as rejected-PoC experiment surface.
/// Returns a winsorized scalar to guard against NaN/Inf from exp()/ln().
pub fn eml_regime_score(footprint: f64, displacement: f64, params: &EmlParams) -> f64 {
    let fp = footprint.clamp(0.0, 3.0);
    let disp = displacement.max(0.0);
    let exp_term = (params.beta * fp).exp();
    let ln_arg = params.delta * disp + params.epsilon;
    let ln_term = ln_arg.max(1e-9).ln();
    let raw = params.alpha * exp_term - params.gamma * ln_term;
    raw.clamp(-5.0, 5.0)
}

/// Regime-conditional factor evaluation and learning.
pub struct RegimeConditional;

impl RegimeConditional {
    pub fn evaluate(
        factor_value: f64,
        regime: Regime,
        profile: Option<&FactorLearningProfile>,
    ) -> f64 {
        factor_value * Self::multiplier_opt(profile, regime)
    }

    pub fn multiplier(profile: &FactorLearningProfile, regime: Regime) -> f64 {
        Self::multiplier_opt(Some(profile), regime)
    }

    pub fn multiplier_opt(profile: Option<&FactorLearningProfile>, regime: Regime) -> f64 {
        profile
            .and_then(|profile| profile.regime_stats.get(regime_key(regime)))
            .map(|stats| {
                if stats.multiplier.abs() <= f64::EPSILON {
                    1.0
                } else {
                    stats.multiplier
                }
            })
            .unwrap_or(1.0)
    }

    pub fn update_profile(
        profile: &mut FactorLearningProfile,
        regime: Regime,
        effective_success: bool,
        pnl: f64,
    ) {
        Self::update_profile_fractional(
            profile,
            regime,
            if effective_success { 1.0 } else { 0.0 },
            1.0,
            pnl,
        );
    }

    pub fn update_profile_fractional(
        profile: &mut FactorLearningProfile,
        regime: Regime,
        success_credit: f64,
        observation_weight: f64,
        pnl: f64,
    ) {
        let success_credit = success_credit.clamp(0.0, 1.0);
        let observation_weight = observation_weight.clamp(0.0, 1.0);
        if observation_weight <= f64::EPSILON {
            return;
        }
        let stats = profile
            .regime_stats
            .entry(regime_key(regime).to_string())
            .or_insert_with(|| RegimeFactorStats {
                multiplier: 1.0,
                ..RegimeFactorStats::default()
            });

        stats.observations += 1;
        if success_credit >= 1.0 - f64::EPSILON {
            stats.wins += 1;
        }
        let previous_weighted_observations = stats.weighted_observations;
        stats.weighted_observations += observation_weight;
        stats.weighted_successes += success_credit * observation_weight;

        let n = stats.weighted_observations;
        stats.avg_pnl = if previous_weighted_observations <= f64::EPSILON {
            pnl
        } else {
            ((stats.avg_pnl * previous_weighted_observations) + pnl * observation_weight) / n
        };
        let hit_rate = if stats.weighted_observations <= f64::EPSILON {
            0.5
        } else {
            (stats.weighted_successes / stats.weighted_observations).clamp(0.0, 1.0)
        };
        stats.multiplier = (0.5 + hit_rate + stats.avg_pnl.tanh() * 0.25).clamp(0.4, 1.6);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FactorLearningProfile;

    #[test]
    fn test_eml_regime_score_winsorized_and_monotonic() {
        let params = EmlParams::default();
        let low = eml_regime_score(0.1, 0.9, &params);
        let high = eml_regime_score(2.5, 0.1, &params);
        assert!(low.is_finite(), "low score should be finite");
        assert!(high.is_finite(), "high score should be finite");
        assert!(
            high > low,
            "higher footprint / lower displacement should raise score"
        );
    }

    #[test]
    fn test_eml_regime_score_extreme_inputs_no_nan() {
        let params = EmlParams::default();
        let s1 = eml_regime_score(100.0, 0.0, &params);
        let s2 = eml_regime_score(0.0, 100.0, &params);
        let s3 = eml_regime_score(f64::MAX, f64::MAX, &params);
        assert!(s1.is_finite());
        assert!(s2.is_finite());
        assert!(s3.is_finite());
    }

    #[test]
    fn test_multiplier_opt_is_neutral_without_profile_or_regime_stats() {
        assert_eq!(
            RegimeConditional::multiplier_opt(None, Regime::ManipulationExpansion),
            1.0
        );

        let profile = FactorLearningProfile::default();
        assert_eq!(
            RegimeConditional::multiplier_opt(Some(&profile), Regime::Distribution),
            1.0
        );
    }

    #[test]
    fn test_update_profile_positive_feedback_lifts_multiplier_above_neutral() {
        let mut profile = FactorLearningProfile::default();

        for _ in 0..5 {
            RegimeConditional::update_profile(
                &mut profile,
                Regime::ManipulationExpansion,
                true,
                1.0,
            );
        }

        let stats = profile
            .regime_stats
            .get(regime_key(Regime::ManipulationExpansion))
            .expect("manipulation_expansion stats");
        assert!(stats.multiplier > 1.0);
    }

    #[test]
    fn test_update_profile_fractional_breakeven_keeps_multiplier_near_neutral() {
        let mut profile = FactorLearningProfile::default();

        for _ in 0..5 {
            RegimeConditional::update_profile_fractional(
                &mut profile,
                Regime::ManipulationExpansion,
                0.5,
                1.0,
                0.0,
            );
        }

        let stats = profile
            .regime_stats
            .get(regime_key(Regime::ManipulationExpansion))
            .expect("manipulation_expansion stats");
        assert_eq!(stats.observations, 5);
        assert!((stats.weighted_observations - 5.0).abs() < 1e-9);
        assert!((stats.weighted_successes - 2.5).abs() < 1e-9);
        assert!((stats.multiplier - 1.0).abs() < 1e-9);
    }
}
