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
        let structurally_resolved_feedback = feedback
            .iter()
            .filter(|record| {
                !crate::state::structural_feedback_outcome_is_unresolved(&record.realized_outcome)
            })
            .cloned()
            .collect::<Vec<_>>();
        let executed_feedback = structurally_resolved_feedback
            .iter()
            .filter(|record| crate::state::structural_feedback_counts_as_executed_trade(record))
            .cloned()
            .collect::<Vec<_>>();
        learning_state.apply_structural_feedback(&structurally_resolved_feedback);
        for record in &executed_feedback {
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
                let credit = factor_feedback_credit(record, supports_selected, factor.weight)
                    .unwrap_or(FactorFeedbackCredit {
                        success_credit: 0.5,
                        observation_weight: 1.0,
                        reliability_target: 0.5,
                        signed_delta: 0.0,
                    });
                let (base_weight, posterior_reliability) = {
                    let profile = learning_state.ensure_profile(&factor.factor_name);
                    profile.posterior_reliability = blend(
                        profile.posterior_reliability,
                        credit.reliability_target,
                        self.learning_rate,
                    );
                    let target_weight =
                        (profile.base_weight + credit.signed_delta).clamp(0.02, 1.0);
                    profile.base_weight =
                        blend(profile.base_weight, target_weight, self.learning_rate);
                    RegimeConditional::update_profile_fractional(
                        profile,
                        record.regime_at_entry,
                        credit.success_credit,
                        credit.observation_weight,
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

#[derive(Debug, Clone, Copy)]
struct FactorFeedbackCredit {
    success_credit: f64,
    observation_weight: f64,
    reliability_target: f64,
    signed_delta: f64,
}

fn factor_feedback_credit(
    record: &FeedbackRecord,
    supports_selected: bool,
    factor_weight: f64,
) -> Option<FactorFeedbackCredit> {
    let semantics = crate::state::structural_feedback_learning_semantics(record);
    if semantics.observation_weight <= f64::EPSILON {
        return None;
    }
    let base_success_credit = semantics.success_credit.clamp(0.0, 1.0);
    let observation_weight = semantics.observation_weight.max(0.0);
    let success_credit = if supports_selected {
        base_success_credit
    } else {
        1.0 - base_success_credit
    };
    let reliability_target = (0.25_f64 + 0.5_f64 * success_credit).clamp(0.25, 0.75);
    let signed_delta = if success_credit >= 0.5 {
        factor_weight.abs() * ((success_credit - 0.5) / 0.5) * 0.20 * observation_weight
    } else {
        -factor_weight.abs() * ((0.5 - success_credit) / 0.5) * 0.25 * observation_weight
    };
    Some(FactorFeedbackCredit {
        success_credit,
        observation_weight,
        reliability_target,
        signed_delta,
    })
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
            structural_feedback: None,
            reflection_mismatch_tags: Vec::new(),
        };

        updater.apply_feedback(&mut learning_state, &[feedback]);
        let profile = learning_state.profile("trend_momentum").unwrap();
        assert!(profile.posterior_reliability > 0.5);
        assert!(profile.base_weight > 0.2);
        assert!(profile.regime_stats.contains_key("manipulation_expansion"));
        assert!(learning_state.factor_rankings[0].weight > 0.2);
    }

    #[test]
    fn test_apply_feedback_updates_structural_prior_state() {
        let updater = WeightUpdater::default();
        let mut learning_state = LearningState::default();
        let feedback = FeedbackRecord {
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            source: "structural_feedback".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: vec![],
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
            structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-1".to_string(),
                recommended_at: "2026-04-29T00:00:00Z".to_string(),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                    .to_string(),
                path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                    .to_string(),
                followed_path: true,
                exit_reason: Some("target_hit".to_string()),
                notes: None,
            }),
            reflection_mismatch_tags: Vec::new(),
        };

        updater.apply_feedback(&mut learning_state, &[feedback]);

        let path = learning_state
            .structural_prior_state
            .paths
            .get("path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary")
            .expect("path prior state");
        assert_eq!(path.observations, 1);
        assert_eq!(path.followed_count, 1);
        assert_eq!(path.wins, 1);
        assert!(path.smoothed_prior > 0.5);
        assert!(learning_state
            .structural_prior_state
            .nodes
            .contains_key("NQ:belief_regime_node:trend"));
    }

    #[test]
    fn test_apply_feedback_skips_unresolved_outcomes() {
        let updater = WeightUpdater::default();
        let mut learning_state = LearningState {
            factor_rankings: vec![PersistedFactorRanking {
                factor_name: "trend_momentum".to_string(),
                weight: 0.2,
                ..PersistedFactorRanking::default()
            }],
            ..LearningState::default()
        };
        let mut feedback = FeedbackRecord {
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            source: "structural_feedback".to_string(),
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
            realized_outcome: "pending".to_string(),
            pnl: 0.0,
            regime_at_entry: Regime::ManipulationExpansion,
            structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-pending".to_string(),
                recommended_at: "2026-04-29T00:00:00Z".to_string(),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                    .to_string(),
                path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                    .to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            }),
            reflection_mismatch_tags: Vec::new(),
        };
        let before_weight = learning_state.factor_rankings[0].weight;

        updater.apply_feedback(&mut learning_state, &[feedback.clone()]);

        assert_eq!(learning_state.factor_rankings[0].weight, before_weight);
        assert!(learning_state.structural_prior_state.paths.is_empty());
        feedback.realized_outcome = "win".to_string();
        feedback.pnl = 0.02;
        updater.apply_feedback(&mut learning_state, &[feedback]);
        assert!(!learning_state.structural_prior_state.paths.is_empty());
        assert!(learning_state.factor_rankings[0].weight > before_weight);
    }

    #[test]
    fn test_apply_feedback_skips_not_followed_outcomes_for_trade_learning() {
        let updater = WeightUpdater::default();
        let mut learning_state = LearningState {
            factor_rankings: vec![PersistedFactorRanking {
                factor_name: "trend_momentum".to_string(),
                weight: 0.2,
                ..PersistedFactorRanking::default()
            }],
            ..LearningState::default()
        };
        let mut feedback = FeedbackRecord {
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            source: "structural_feedback".to_string(),
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
            realized_outcome: "not_followed".to_string(),
            pnl: 0.0,
            regime_at_entry: Regime::ManipulationExpansion,
            structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-not-followed".to_string(),
                recommended_at: "2026-04-29T00:00:00Z".to_string(),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                    .to_string(),
                path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                    .to_string(),
                followed_path: false,
                exit_reason: Some("user_skipped".to_string()),
                notes: None,
            }),
            reflection_mismatch_tags: Vec::new(),
        };
        let before_weight = learning_state.factor_rankings[0].weight;

        updater.apply_feedback(&mut learning_state, &[feedback.clone()]);

        assert_eq!(learning_state.factor_rankings[0].weight, before_weight);
        let path = learning_state
            .structural_prior_state
            .paths
            .get("path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary")
            .expect("path prior state");
        assert_eq!(path.not_followed, 1);
        assert_eq!(path.followed_count, 0);
        assert_eq!(path.weighted_followed_mass, 0.0);
        feedback.realized_outcome = "win".to_string();
        feedback.pnl = 0.02;
        if let Some(refs) = feedback.structural_feedback.as_mut() {
            refs.followed_path = true;
        }
        updater.apply_feedback(&mut learning_state, &[feedback]);
        assert!(learning_state.factor_rankings[0].weight > before_weight);
    }

    #[test]
    fn test_apply_feedback_treats_abandoned_as_fractional_negative_trade_learning() {
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
            source: "structural_feedback".to_string(),
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
            realized_outcome: "abandoned".to_string(),
            pnl: 0.0,
            regime_at_entry: Regime::ManipulationExpansion,
            structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-abandoned".to_string(),
                recommended_at: "2026-04-29T00:00:00Z".to_string(),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                    .to_string(),
                path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                    .to_string(),
                followed_path: true,
                exit_reason: Some("manual_exit".to_string()),
                notes: None,
            }),
            reflection_mismatch_tags: Vec::new(),
        };
        let before_weight = learning_state.factor_rankings[0].weight;

        updater.apply_feedback(&mut learning_state, &[feedback]);

        let profile = learning_state.profile("trend_momentum").unwrap();
        assert!(learning_state.factor_rankings[0].weight < before_weight);
        assert!(learning_state.factor_rankings[0].weight > 0.18);
        assert!(profile.posterior_reliability < 0.5);
        assert!(profile.posterior_reliability > 0.35);
        let regime_stats = profile
            .regime_stats
            .get("manipulation_expansion")
            .expect("regime stats");
        assert!((regime_stats.weighted_observations - 0.75).abs() < 1e-9);
        assert!((regime_stats.weighted_successes - 0.1875).abs() < 1e-9);
        let path = learning_state
            .structural_prior_state
            .paths
            .get("path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary")
            .expect("path prior state");
        assert_eq!(path.abandoned, 1);
    }
}
