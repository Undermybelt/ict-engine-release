use chrono::Utc;

use crate::factors::regime_conditional::RegimeConditional;
use crate::state::{FeedbackRecord, LearningState, PersistedFactorRanking};
use crate::types::{Direction, FactorIC};

/// Factor weight updater for backtest and feedback learning.
pub struct WeightUpdater {
    pub learning_rate: f64,
}

impl Default for WeightUpdater {
    fn default() -> Self {
        Self {
            learning_rate: 0.25,
        }
    }
}

impl WeightUpdater {
    /// Update weights based on IC/IR plus realized backtest quality.
    pub fn update_weights(factors: &mut [FactorIC]) {
        let scorecards = factors
            .iter()
            .map(|factor| {
                let mut scorecard = PersistedFactorRanking::from(factor);
                scorecard.conformal_coverage_1sigma = factor.weight;
                scorecard.refresh_scorecard();
                scorecard
            })
            .collect::<Vec<_>>();
        let raw_scores = scorecards
            .iter()
            .map(|scorecard| scorecard.composite_score.max(0.0))
            .collect::<Vec<_>>();

        let total_score: f64 = raw_scores.iter().sum();
        if total_score <= f64::EPSILON {
            let equal = if factors.is_empty() {
                0.0
            } else {
                1.0 / factors.len() as f64
            };
            for factor in factors.iter_mut() {
                factor.weight = equal;
            }
            return;
        }

        for (factor, score) in factors.iter_mut().zip(raw_scores.into_iter()) {
            factor.weight = score / total_score;
        }
    }

    pub fn apply_rankings(&self, learning_state: &mut LearningState, factors: &[FactorIC]) {
        let mut persisted = Vec::with_capacity(factors.len());
        for factor in factors {
            let profile = learning_state.ensure_profile(&factor.factor_name);
            profile.base_weight = blend(profile.base_weight, factor.weight, self.learning_rate);
            profile.posterior_reliability = blend(
                profile.posterior_reliability,
                (0.5 + factor.stability * 0.5).clamp(0.1, 0.99),
                self.learning_rate,
            );
            profile.last_ic = factor.mean_ic;
            profile.last_ir = factor.ir;
            profile.last_backtest_return = factor.backtest_return;
            profile.last_stability = factor.stability;
            let mut ranking = PersistedFactorRanking::from(factor);
            ranking.weight = profile.base_weight;
            ranking.stability = profile.posterior_reliability;
            ranking.refresh_scorecard();
            profile.enabled =
                ranking.iteration_action != "replace" || ranking.composite_score >= 0.30;
            persisted.push(ranking);
        }
        persisted.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        learning_state.factor_rankings = persisted;
        learning_state.last_updated = Some(Utc::now());
    }

    pub fn apply_feedback(&self, learning_state: &mut LearningState, feedback: &[FeedbackRecord]) {
        for record in feedback {
            for factor in &record.factors_used {
                let ranking_exists = learning_state
                    .factor_rankings
                    .iter()
                    .any(|ranking| ranking.factor_name == factor.factor_name);
                let selected_direction = record.model_probabilities_before_trade.selected_direction;
                let supports_selected = match selected_direction {
                    Direction::Bull => factor.long_support >= factor.short_support,
                    Direction::Bear => factor.short_support >= factor.long_support,
                    Direction::Neutral => false,
                };
                let effective_success = if supports_selected {
                    record.pnl >= 0.0 || record.realized_outcome == "win"
                } else {
                    record.pnl < 0.0 || record.realized_outcome == "loss"
                };
                let (base_weight, posterior_reliability) = {
                    let profile = learning_state.ensure_profile(&factor.factor_name);
                    let reliability_target = if effective_success { 0.75 } else { 0.25 };
                    profile.posterior_reliability = blend(
                        profile.posterior_reliability,
                        reliability_target,
                        self.learning_rate,
                    );
                    let signed_delta = if effective_success {
                        factor.weight.abs() * 0.20
                    } else {
                        -factor.weight.abs() * 0.25
                    };
                    let target_weight = (profile.base_weight + signed_delta).clamp(0.02, 1.0);
                    profile.base_weight =
                        blend(profile.base_weight, target_weight, self.learning_rate);
                    RegimeConditional::update_profile(
                        profile,
                        record.regime_at_entry,
                        effective_success,
                        record.pnl,
                    );
                    (profile.base_weight, profile.posterior_reliability)
                };

                if !ranking_exists {
                    learning_state.factor_rankings.push(PersistedFactorRanking {
                        factor_name: factor.factor_name.clone(),
                        weight: base_weight,
                        stability: posterior_reliability,
                        ..PersistedFactorRanking::default()
                    });
                }
            }
        }
        sync_rankings_from_profiles(learning_state);
        learning_state.last_updated = Some(Utc::now());
    }
}

fn blend(previous: f64, target: f64, learning_rate: f64) -> f64 {
    let lr = learning_rate.clamp(0.0, 1.0);
    (1.0 - lr) * previous + lr * target
}

fn sync_rankings_from_profiles(learning_state: &mut LearningState) {
    let profile_snapshot = learning_state
        .factor_profiles
        .iter()
        .map(|(name, profile)| {
            (
                name.clone(),
                (profile.base_weight, profile.posterior_reliability),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    for ranking in &mut learning_state.factor_rankings {
        if let Some((weight, reliability)) = profile_snapshot.get(&ranking.factor_name) {
            ranking.weight = *weight;
            ranking.stability = *reliability;
            ranking.refresh_scorecard();
        }
    }
    learning_state.factor_rankings.sort_by(|a, b| {
        b.composite_score
            .partial_cmp(&a.composite_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{FeedbackFactorUsage, ModelProbabilitySnapshot};
    use crate::types::Regime;
    use std::collections::HashMap;

    #[test]
    fn test_update_weights_and_learning_state() {
        let mut rankings = vec![
            FactorIC {
                factor_name: "trend_momentum".to_string(),
                regime: Regime::ManipulationExpansion,
                ic_values: vec![0.2, 0.3],
                mean_ic: 0.25,
                std_ic: 0.05,
                ir: 5.0,
                weight: 0.0,
                backtest_return: 0.12,
                sharpe: 1.4,
                stability: 0.7,
                win_rate: 0.55,
                profit_factor: 1.6,
                trade_count: 8,
                regime_scores: HashMap::from([("manipulation_expansion".to_string(), 0.15)]),
            },
            FactorIC {
                factor_name: "structure_ict".to_string(),
                regime: Regime::ManipulationExpansion,
                ic_values: vec![0.1, 0.0],
                mean_ic: 0.05,
                std_ic: 0.05,
                ir: 1.0,
                weight: 0.0,
                backtest_return: 0.03,
                sharpe: 0.5,
                stability: 0.4,
                win_rate: 0.50,
                profit_factor: 1.1,
                trade_count: 4,
                regime_scores: HashMap::new(),
            },
        ];

        WeightUpdater::update_weights(&mut rankings);
        assert!(rankings[0].weight > rankings[1].weight);

        let updater = WeightUpdater::default();
        let mut learning_state = LearningState::default();
        updater.apply_rankings(&mut learning_state, &rankings);
        assert_eq!(learning_state.factor_rankings.len(), 2);
        assert!(
            learning_state
                .profile("trend_momentum")
                .unwrap()
                .base_weight
                > learning_state.profile("structure_ict").unwrap().base_weight
        );
    }

    #[test]
    fn test_apply_feedback_updates_reliability_and_regime_stats() {
        let updater = WeightUpdater::default();
        let mut learning_state = LearningState {
            factor_rankings: vec![PersistedFactorRanking {
                factor_name: "trend_momentum".to_string(),
                weight: 0.2,
                ..PersistedFactorRanking::default()
            }],
            ..LearningState::default()
        };
        let feedback = FeedbackRecord {
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            source: "backtest".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: vec![FeedbackFactorUsage {
                factor_name: "trend_momentum".to_string(),
                category: "trend_momentum".to_string(),
                direction: Direction::Bull,
                value: 0.8,
                confidence: 0.7,
                weight: 0.3,
                long_support: 0.4,
                short_support: 0.0,
                uncertainty_contribution: 0.1,
            }],
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: Direction::Bull,
                selected_probability: 0.6,
                long_score: 0.6,
                short_score: 0.3,
                win_prob_long: 0.58,
                win_prob_short: 0.41,
                uncertainty: 0.2,
            },
            realized_outcome: "win".to_string(),
            pnl: 0.02,
            regime_at_entry: Regime::ManipulationExpansion,
        };

        updater.apply_feedback(&mut learning_state, &[feedback]);
        let profile = learning_state.profile("trend_momentum").unwrap();
        assert!(profile.posterior_reliability > 0.5);
        assert!(profile.base_weight > 0.2);
        assert!(profile.regime_stats.contains_key("manipulation_expansion"));
        assert!(learning_state.factor_rankings[0].weight > 0.2);
    }
}
