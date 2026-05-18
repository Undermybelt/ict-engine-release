use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::belief_core::structural_state::{
    StructuralBranchOutcomeSummary, StructuralNodeOutcomeSummary, StructuralPathOutcomeSummary,
    StructuralScenarioOutcomeSummary,
};
use crate::state::{
    structural_feedback_outcome_is_unresolved, structural_source_observed_outcome_likelihood,
    FeedbackRecord, StructuralPriorEvent, StructuralPriorLearningState,
    StructuralPriorSourceSummary, StructuralPriorStats,
    StructuralSourceReliabilityEmCalibrationSummary, StructuralSourceReliabilityEmConfusionCell,
    StructuralSourceReliabilityEmDiagnostics, StructuralSourceReliabilityEmFit,
    StructuralSourceReliabilityEmHoldoutSummary, StructuralSourceReliabilityEmReplaySummary,
    StructuralSourceReliabilityEmSourceSummary, StructuralSourceReliabilityPosterior,
    StructuralTargetPolicyContextPosterior, STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS,
    STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_CALIBRATION_OBSERVATIONS,
    STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_HOLDOUT_TRAIN_ITEMS,
    STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_MULTI_SOURCE_ITEMS,
};

pub const STRUCTURAL_TARGET_POLICY_CONTEXT_SURFACE_LIMIT: usize = 3;
pub const STRUCTURAL_DELAYED_REWARD_REPLAY_MIN_TRAIN_RECORDS: usize = 3;

const STRUCTURAL_SOURCE_RELIABILITY_EM_LAPLACE_ALPHA: f64 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralExperiencePriorSurfaceArtifact {
    pub symbol: String,
    #[serde(default)]
    pub source_reliability_em: StructuralSourceReliabilityEmReadiness,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_policy_contexts: Vec<StructuralTargetPolicyContextSurface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<StructuralExperiencePriorEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<StructuralExperiencePriorEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<StructuralExperiencePriorEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<StructuralExperiencePriorEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralTargetPolicyContextSurface {
    pub context_key: String,
    pub observations: usize,
    pub weighted_observation_mass: f64,
    pub behavior_policy_probability: f64,
    pub behavior_policy_probability_variance: f64,
    pub learned_target_policy_probability: f64,
    pub learned_target_policy_probability_lower_bound: f64,
    pub learned_target_policy_probability_confidence: f64,
    pub calibrated_target_policy_probability: f64,
    pub calibrated_target_policy_probability_lower_bound: f64,
    pub target_policy_probability_brier_score: f64,
    pub target_policy_probability_calibration_error: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommendation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralDelayedRewardReplayValidationSurface {
    pub status: String,
    pub training_record_count: usize,
    pub evaluation_record_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_training_recommended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_evaluation_recommended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_evaluation_recommended_at: Option<String>,
    pub resolution_observation_count: usize,
    pub resolution_1h_observation_count: usize,
    pub resolution_4h_observation_count: usize,
    pub resolution_24h_observation_count: usize,
    pub min_training_records: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_1h_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_4h_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_24h_brier_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralSourceReliabilityEmReadiness {
    pub ready: bool,
    pub status: String,
    pub candidate_item_count: usize,
    pub labeled_item_count: usize,
    pub multi_source_item_count: usize,
    pub distinct_source_count: usize,
    pub observed_label_count: usize,
    pub max_sources_per_item: usize,
    pub min_multi_source_items: usize,
    #[serde(default)]
    pub consensus_item_count: usize,
    #[serde(default)]
    pub conflict_item_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_consensus_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_consensus_confidence: Option<f64>,
    #[serde(default)]
    pub em_iteration_count: usize,
    #[serde(default)]
    pub em_latent_item_count: usize,
    #[serde(default)]
    pub em_distinct_label_count: usize,
    #[serde(default)]
    pub em_confusion_cell_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_em_latent_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_em_latent_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_em_source_reliability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_em_source_reliability: Option<f64>,
    #[serde(default)]
    pub persisted_source_summary_count: usize,
    #[serde(default)]
    pub persisted_confusion_cell_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_persisted_source_reliability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_persisted_source_reliability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_calibration_status: Option<String>,
    #[serde(default)]
    pub em_calibration_observation_count: usize,
    #[serde(default)]
    pub em_calibration_source_count: usize,
    #[serde(default)]
    pub em_calibration_min_observations: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_calibration_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_calibration_log_loss: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_holdout_status: Option<String>,
    pub em_holdout_split_strategy: Option<String>,
    #[serde(default)]
    pub em_holdout_training_item_count: usize,
    #[serde(default)]
    pub em_holdout_evaluation_item_count: usize,
    #[serde(default)]
    pub em_holdout_observation_count: usize,
    #[serde(default)]
    pub em_holdout_source_count: usize,
    #[serde(default)]
    pub em_holdout_min_training_items: usize,
    #[serde(default)]
    pub em_holdout_min_observations: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_holdout_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_holdout_log_loss: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_holdout_observation_coverage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_replay_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_replay_split_strategy: Option<String>,
    #[serde(default)]
    pub em_replay_evaluation_item_count: usize,
    #[serde(default)]
    pub em_replay_observation_count: usize,
    #[serde(default)]
    pub em_replay_source_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_replay_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_replay_log_loss: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub em_replay_observation_coverage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralExperiencePriorEntry {
    pub entity_kind: String,
    pub entity_id: String,
    #[serde(default)]
    pub historical_total_records: usize,
    #[serde(default)]
    pub historical_followed_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_win_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_invalidation_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_avg_pnl: Option<f64>,
    pub experience_prior: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_posterior: Option<f64>,
    pub composite_score: f64,
    #[serde(default)]
    pub source_panel_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_offline_seed_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dominant_source_panel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dominant_source_share: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dominant_source_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_propensity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ips_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counterfactual_reward_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off_policy_adjusted_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_policy_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_policy_probability_variance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_probability_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_probability_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_probability_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_probability_calibration_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snips_weight_mass: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snips_weight_squared_mass: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snips_effective_sample_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snips_reward_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doubly_robust_reward_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_calibration_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_reward_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_variance_penalty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_policy_reward_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matured_feedback_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_feedback_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maturity_coverage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub censoring_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_censoring_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub censoring_adjusted_reward_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub censoring_adjusted_reward_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_success_competing_risk: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_failure_competing_risk: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_invalidation_competing_risk: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_abandonment_competing_risk: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_competing_risk_entropy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_elapsed_feedback_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_elapsed_hours_at_risk: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_avg_elapsed_hours: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_hazard_per_hour: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_expected_resolution_hours: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_survival_probability_1h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_survival_probability_4h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_survival_probability_24h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_success_hazard_per_hour: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_failure_hazard_per_hour: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_invalidation_hazard_per_hour: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_abandonment_hazard_per_hour: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_success_cumulative_incidence_4h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_failure_cumulative_incidence_4h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_invalidation_cumulative_incidence_4h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_abandonment_cumulative_incidence_4h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_horizon_1h_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_within_1h_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_probability_1h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_horizon_4h_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_within_4h_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_probability_4h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_horizon_24h_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_within_24h_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_resolution_probability_24h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed_reward_replay_validation: Option<StructuralDelayedRewardReplayValidationSurface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_streak_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_avg_streak_length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_persistence_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_weighted_streak_mass: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_weighted_observation_mass: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_outcome_support: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_temporal_posterior_support: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_outcome_support: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_temporal_posterior_support: Option<f64>,
}

#[derive(Debug, Clone, Default)]
struct StructuralSourceReliabilityEmItem {
    last_recommended_at: Option<String>,
    sources: BTreeSet<String>,
    observed_labels: usize,
    observed_credit_classes: BTreeMap<String, usize>,
    source_credit_classes: BTreeMap<String, BTreeMap<String, usize>>,
}

#[derive(Debug, Default)]
struct StructuralSourceReliabilityEmLedger {
    items: BTreeMap<String, StructuralSourceReliabilityEmItem>,
    distinct_sources: BTreeSet<String>,
    observed_label_count: usize,
}

type StructuralSourceReliabilityEmPosteriors = BTreeMap<String, BTreeMap<String, f64>>;
type StructuralSourceReliabilityEmConfusion =
    BTreeMap<String, BTreeMap<String, BTreeMap<String, f64>>>;

pub fn structural_source_reliability_em_diagnostics(
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralSourceReliabilityEmDiagnostics {
    let ledger = structural_source_reliability_em_ledger(structural_prior_state);
    let items = ledger.items;
    let candidate_item_count = items.len();
    let labeled_item_count = items
        .values()
        .filter(|item| item.observed_labels > 0)
        .count();
    let multi_source_item_count = items
        .values()
        .filter(|item| item.sources.len() >= 2 && item.observed_labels >= 2)
        .count();
    let max_sources_per_item = items
        .values()
        .map(|item| item.sources.len())
        .max()
        .unwrap_or_default();
    let consensus_confidences = items
        .values()
        .filter_map(structural_source_reliability_em_consensus_confidence)
        .collect::<Vec<_>>();
    let consensus_item_count = consensus_confidences.len();
    let conflict_item_count = items
        .values()
        .filter(|item| item.observed_credit_classes.len() > 1)
        .count();
    let avg_consensus_confidence = structural_source_reliability_em_avg(&consensus_confidences);
    let min_consensus_confidence = structural_source_reliability_em_min(&consensus_confidences);
    let fit = structural_source_reliability_em_fit(&items);
    let persisted_source_reliabilities = structural_prior_state
        .source_reliability_em_summaries
        .values()
        .map(|summary| summary.posterior_reliability.clamp(0.0, 1.0))
        .collect::<Vec<_>>();

    StructuralSourceReliabilityEmDiagnostics {
        candidate_item_count,
        labeled_item_count,
        multi_source_item_count,
        distinct_source_count: ledger.distinct_sources.len(),
        observed_label_count: ledger.observed_label_count,
        max_sources_per_item,
        consensus_item_count,
        conflict_item_count,
        avg_consensus_confidence,
        min_consensus_confidence,
        fit,
        persisted_source_summary_count: structural_prior_state
            .source_reliability_em_summaries
            .len(),
        persisted_confusion_cell_count: structural_prior_state
            .source_reliability_em_summaries
            .values()
            .map(|summary| summary.confusion_cell_count)
            .sum(),
        avg_persisted_source_reliability: structural_source_reliability_em_avg(
            &persisted_source_reliabilities,
        ),
        min_persisted_source_reliability: structural_source_reliability_em_min(
            &persisted_source_reliabilities,
        ),
        calibration: structural_prior_state
            .source_reliability_em_calibration
            .clone(),
        holdout: structural_source_reliability_em_holdout_summary(&items),
        replay: structural_source_reliability_em_replay_summary(&items),
    }
}

pub fn structural_source_reliability_em_fit_from_state(
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralSourceReliabilityEmFit {
    if let Some(fit) =
        structural_source_reliability_em_fit_from_persisted_state(structural_prior_state)
    {
        return fit;
    }
    let ledger = structural_source_reliability_em_ledger(structural_prior_state);
    structural_source_reliability_em_fit(&ledger.items)
}

pub fn refresh_structural_source_reliability_em_state(
    structural_prior_state: &mut StructuralPriorLearningState,
) {
    structural_prior_state
        .source_reliability_em_summaries
        .clear();
    structural_prior_state.source_reliability_em_calibration = None;
    let ledger = structural_source_reliability_em_ledger(structural_prior_state);
    let fit = structural_source_reliability_em_fit(&ledger.items);
    if fit.iteration_count == 0 {
        return;
    }
    for (source_label, source_confusion) in &fit.confusion {
        let diagonal = source_confusion
            .iter()
            .filter_map(|(true_label, row)| row.get(true_label).copied())
            .collect::<Vec<_>>();
        let confusion = source_confusion
            .iter()
            .flat_map(|(true_label, row)| {
                row.iter().map(move |(observed_label, probability)| {
                    let key = format!("{true_label}->{observed_label}");
                    (
                        key,
                        StructuralSourceReliabilityEmConfusionCell {
                            true_credit_class: true_label.clone(),
                            observed_credit_class: observed_label.clone(),
                            probability: probability.clamp(0.0, 1.0),
                        },
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();
        structural_prior_state
            .source_reliability_em_summaries
            .insert(
                source_label.clone(),
                StructuralSourceReliabilityEmSourceSummary {
                    source_label: source_label.clone(),
                    iteration_count: fit.iteration_count,
                    latent_item_count: fit.latent_item_count,
                    distinct_label_count: fit.distinct_label_count,
                    confusion_cell_count: confusion.len(),
                    posterior_reliability: fit
                        .source_reliability
                        .get(source_label)
                        .copied()
                        .unwrap_or_default()
                        .clamp(0.0, 1.0),
                    min_diagonal_probability: structural_source_reliability_em_min(&diagonal)
                        .unwrap_or_default()
                        .clamp(0.0, 1.0),
                    confusion,
                },
            );
    }
    structural_prior_state.source_reliability_em_calibration =
        structural_source_reliability_em_calibration_summary(
            &ledger.items,
            &structural_prior_state.source_reliability_em_summaries,
        );
}

fn structural_source_reliability_em_calibration_summary(
    items: &BTreeMap<String, StructuralSourceReliabilityEmItem>,
    summaries: &BTreeMap<String, StructuralSourceReliabilityEmSourceSummary>,
) -> Option<StructuralSourceReliabilityEmCalibrationSummary> {
    if summaries.is_empty() {
        return None;
    }
    let mut weighted_brier = 0.0;
    let mut weighted_log_loss = 0.0;
    let mut observation_count = 0usize;
    let mut calibrated_sources = BTreeSet::<String>::new();

    for item in items.values() {
        if item.sources.len() < 2 || item.observed_labels < 2 {
            continue;
        }
        for (source_label, observed_counts) in &item.source_credit_classes {
            let Some(consensus_label) =
                structural_source_reliability_em_leave_source_out_consensus(item, source_label)
            else {
                continue;
            };
            let Some(summary) = summaries.get(source_label) else {
                continue;
            };
            let row = summary
                .confusion
                .values()
                .filter(|cell| cell.true_credit_class == consensus_label)
                .map(|cell| {
                    (
                        cell.observed_credit_class.clone(),
                        cell.probability.clamp(0.0, 1.0),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if row.is_empty() {
                continue;
            }
            for (observed_label, count) in observed_counts {
                if *count == 0 {
                    continue;
                }
                let mut brier = row
                    .iter()
                    .map(|(row_label, probability)| {
                        let target = if row_label == observed_label {
                            1.0
                        } else {
                            0.0
                        };
                        (probability - target) * (probability - target)
                    })
                    .sum::<f64>();
                if !row.contains_key(observed_label) {
                    brier += 1.0;
                }
                let observed_probability = row
                    .get(observed_label)
                    .copied()
                    .unwrap_or(1e-12)
                    .clamp(1e-12, 1.0);
                weighted_brier += brier * *count as f64;
                weighted_log_loss += -observed_probability.ln() * *count as f64;
                observation_count += *count;
                calibrated_sources.insert(source_label.clone());
            }
        }
    }

    if observation_count == 0 {
        return None;
    }
    let source_count = calibrated_sources.len();
    let status = if source_count < 2 {
        "needs_multiple_sources"
    } else if observation_count < STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_CALIBRATION_OBSERVATIONS {
        "needs_larger_panel"
    } else {
        "ready"
    };
    Some(StructuralSourceReliabilityEmCalibrationSummary {
        status: status.to_string(),
        observation_count,
        source_count,
        min_observations: STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_CALIBRATION_OBSERVATIONS,
        brier_score: Some((weighted_brier / observation_count as f64).clamp(0.0, 2.0)),
        log_loss: Some((weighted_log_loss / observation_count as f64).max(0.0)),
    })
}

fn structural_source_reliability_em_holdout_summary(
    items: &BTreeMap<String, StructuralSourceReliabilityEmItem>,
) -> Option<StructuralSourceReliabilityEmHoldoutSummary> {
    let mut fit_items = items
        .iter()
        .filter(|(_, item)| item.sources.len() >= 2 && item.observed_labels >= 2)
        .collect::<Vec<_>>();
    if fit_items.is_empty() {
        return None;
    }
    fit_items.sort_by(|(left_key, left), (right_key, right)| {
        left.last_recommended_at
            .cmp(&right.last_recommended_at)
            .then_with(|| left_key.cmp(right_key))
    });
    let min_training_items = STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_HOLDOUT_TRAIN_ITEMS;
    let min_observations = STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_CALIBRATION_OBSERVATIONS;
    if fit_items.len() <= min_training_items {
        return Some(StructuralSourceReliabilityEmHoldoutSummary {
            status: "needs_more_items".to_string(),
            split_strategy: "chronological_recommended_at".to_string(),
            training_item_count: fit_items.len(),
            evaluation_item_count: 0,
            observation_count: 0,
            source_count: 0,
            min_training_items,
            min_observations,
            brier_score: None,
            log_loss: None,
        });
    }
    let split_index = fit_items.len().saturating_mul(2) / 3;
    let split_index = split_index.clamp(min_training_items, fit_items.len().saturating_sub(1));
    let training_items = fit_items
        .iter()
        .take(split_index)
        .map(|(key, item)| ((*key).clone(), (**item).clone()))
        .collect::<BTreeMap<_, _>>();
    let evaluation_items = fit_items
        .iter()
        .skip(split_index)
        .map(|(key, item)| ((*key).clone(), (**item).clone()))
        .collect::<BTreeMap<_, _>>();
    if evaluation_items.is_empty() {
        return Some(StructuralSourceReliabilityEmHoldoutSummary {
            status: "needs_more_items".to_string(),
            split_strategy: "chronological_recommended_at".to_string(),
            training_item_count: training_items.len(),
            evaluation_item_count: 0,
            observation_count: 0,
            source_count: 0,
            min_training_items,
            min_observations,
            brier_score: None,
            log_loss: None,
        });
    }
    let fit = structural_source_reliability_em_fit(&training_items);
    if fit.iteration_count == 0 {
        return Some(StructuralSourceReliabilityEmHoldoutSummary {
            status: "needs_multiple_sources".to_string(),
            split_strategy: "chronological_recommended_at".to_string(),
            training_item_count: training_items.len(),
            evaluation_item_count: evaluation_items.len(),
            observation_count: 0,
            source_count: fit.source_reliability.len(),
            min_training_items,
            min_observations,
            brier_score: None,
            log_loss: None,
        });
    }
    let (observation_count, source_count, weighted_brier, weighted_log_loss) =
        structural_source_reliability_em_score_items(&evaluation_items, &fit.confusion);
    let status = if source_count < 2 {
        "needs_multiple_sources"
    } else if observation_count < min_observations {
        "needs_larger_panel"
    } else {
        "ready"
    };
    Some(StructuralSourceReliabilityEmHoldoutSummary {
        status: status.to_string(),
        split_strategy: "chronological_recommended_at".to_string(),
        training_item_count: training_items.len(),
        evaluation_item_count: evaluation_items.len(),
        observation_count,
        source_count,
        min_training_items,
        min_observations,
        brier_score: (observation_count > 0)
            .then_some((weighted_brier / observation_count as f64).clamp(0.0, 2.0)),
        log_loss: (observation_count > 0)
            .then_some((weighted_log_loss / observation_count as f64).max(0.0)),
    })
}

fn structural_source_reliability_em_score_items(
    items: &BTreeMap<String, StructuralSourceReliabilityEmItem>,
    confusion: &StructuralSourceReliabilityEmConfusion,
) -> (usize, usize, f64, f64) {
    let mut weighted_brier = 0.0;
    let mut weighted_log_loss = 0.0;
    let mut observation_count = 0usize;
    let mut calibrated_sources = BTreeSet::<String>::new();

    for item in items.values() {
        if item.sources.len() < 2 || item.observed_labels < 2 {
            continue;
        }
        for (source_label, observed_counts) in &item.source_credit_classes {
            let Some(consensus_label) =
                structural_source_reliability_em_leave_source_out_consensus(item, source_label)
            else {
                continue;
            };
            let Some(source_confusion) = confusion.get(source_label) else {
                continue;
            };
            let row = source_confusion
                .get(&consensus_label)
                .cloned()
                .unwrap_or_default();
            if row.is_empty() {
                continue;
            }
            for (observed_label, count) in observed_counts {
                if *count == 0 {
                    continue;
                }
                let mut brier = row
                    .iter()
                    .map(|(row_label, probability)| {
                        let target = if row_label == observed_label {
                            1.0
                        } else {
                            0.0
                        };
                        (probability - target) * (probability - target)
                    })
                    .sum::<f64>();
                if !row.contains_key(observed_label) {
                    brier += 1.0;
                }
                let observed_probability = row
                    .get(observed_label)
                    .copied()
                    .unwrap_or(1e-12)
                    .clamp(1e-12, 1.0);
                weighted_brier += brier * *count as f64;
                weighted_log_loss += -observed_probability.ln() * *count as f64;
                observation_count += *count;
                calibrated_sources.insert(source_label.clone());
            }
        }
    }

    (
        observation_count,
        calibrated_sources.len(),
        weighted_brier,
        weighted_log_loss,
    )
}

fn structural_source_reliability_em_replay_summary(
    items: &BTreeMap<String, StructuralSourceReliabilityEmItem>,
) -> Option<StructuralSourceReliabilityEmReplaySummary> {
    let mut fit_items = items
        .iter()
        .filter(|(_, item)| item.sources.len() >= 2 && item.observed_labels >= 2)
        .collect::<Vec<_>>();
    if fit_items.len() <= STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_HOLDOUT_TRAIN_ITEMS {
        return None;
    }
    fit_items.sort_by(|(left_key, left), (right_key, right)| {
        left.last_recommended_at
            .cmp(&right.last_recommended_at)
            .then_with(|| left_key.cmp(right_key))
    });

    let min_training_items = STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_HOLDOUT_TRAIN_ITEMS;
    let min_observations = STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_CALIBRATION_OBSERVATIONS;
    let mut evaluation_item_count = 0usize;
    let mut observation_count = 0usize;
    let mut scored_source_labels = BTreeSet::<String>::new();
    let mut weighted_brier = 0.0;
    let mut weighted_log_loss = 0.0;

    for split_index in min_training_items..fit_items.len() {
        let training_items = fit_items
            .iter()
            .take(split_index)
            .map(|(key, item)| ((*key).clone(), (**item).clone()))
            .collect::<BTreeMap<_, _>>();
        let evaluation_item = fit_items[split_index];
        let mut evaluation_items = BTreeMap::new();
        evaluation_items.insert((*evaluation_item.0).clone(), (*evaluation_item.1).clone());

        let fit = structural_source_reliability_em_fit(&training_items);
        if fit.iteration_count == 0 {
            continue;
        }
        let (item_observations, item_source_count, item_brier, item_log_loss) =
            structural_source_reliability_em_score_items(&evaluation_items, &fit.confusion);
        if item_observations == 0 {
            continue;
        }
        evaluation_item_count += 1;
        observation_count += item_observations;
        weighted_brier += item_brier;
        weighted_log_loss += item_log_loss;
        if item_source_count > 0 {
            scored_source_labels.extend(evaluation_item.1.source_credit_classes.keys().cloned());
        }
    }

    if evaluation_item_count == 0 {
        return None;
    }
    let source_count = scored_source_labels.len();
    let status = if source_count < 2 {
        "needs_multiple_sources"
    } else if observation_count < min_observations {
        "needs_larger_panel"
    } else {
        "ready"
    };
    Some(StructuralSourceReliabilityEmReplaySummary {
        status: status.to_string(),
        split_strategy: "expanding_window_recommended_at".to_string(),
        evaluation_item_count,
        observation_count,
        source_count,
        min_training_items,
        min_observations,
        brier_score: (observation_count > 0)
            .then_some((weighted_brier / observation_count as f64).clamp(0.0, 2.0)),
        log_loss: (observation_count > 0)
            .then_some((weighted_log_loss / observation_count as f64).max(0.0)),
    })
}

fn structural_source_reliability_em_leave_source_out_consensus(
    item: &StructuralSourceReliabilityEmItem,
    source_label: &str,
) -> Option<String> {
    let mut other_counts = item.observed_credit_classes.clone();
    if let Some(source_counts) = item.source_credit_classes.get(source_label) {
        for (label, count) in source_counts {
            let entry = other_counts.get_mut(label)?;
            *entry = entry.saturating_sub(*count);
        }
    }
    other_counts.retain(|_, count| *count > 0);
    let max_count = other_counts.values().copied().max()?;
    if other_counts
        .values()
        .filter(|count| **count == max_count)
        .count()
        != 1
    {
        return None;
    }
    other_counts
        .into_iter()
        .find_map(|(label, count)| (count == max_count).then_some(label))
}

fn structural_source_reliability_em_fit_from_persisted_state(
    structural_prior_state: &StructuralPriorLearningState,
) -> Option<StructuralSourceReliabilityEmFit> {
    if structural_prior_state
        .source_reliability_em_summaries
        .is_empty()
    {
        return None;
    }
    let source_reliability = structural_prior_state
        .source_reliability_em_summaries
        .iter()
        .map(|(source_label, summary)| {
            (
                source_label.clone(),
                summary.posterior_reliability.clamp(0.0, 1.0),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let source_reliabilities = source_reliability.values().copied().collect::<Vec<_>>();
    Some(StructuralSourceReliabilityEmFit {
        iteration_count: structural_prior_state
            .source_reliability_em_summaries
            .values()
            .map(|summary| summary.iteration_count)
            .max()
            .unwrap_or_default(),
        latent_item_count: structural_prior_state
            .source_reliability_em_summaries
            .values()
            .map(|summary| summary.latent_item_count)
            .max()
            .unwrap_or_default(),
        distinct_label_count: structural_prior_state
            .source_reliability_em_summaries
            .values()
            .map(|summary| summary.distinct_label_count)
            .max()
            .unwrap_or_default(),
        confusion_cell_count: structural_prior_state
            .source_reliability_em_summaries
            .values()
            .map(|summary| summary.confusion_cell_count)
            .sum(),
        source_reliability,
        avg_source_reliability: structural_source_reliability_em_avg(&source_reliabilities),
        min_source_reliability: structural_source_reliability_em_min(&source_reliabilities),
        ..StructuralSourceReliabilityEmFit::default()
    })
}

fn structural_source_reliability_em_item_key(event: &StructuralPriorEvent) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        event.symbol,
        event.recommendation_id,
        event.node_id,
        event.branch_id,
        event.scenario_id,
        event.path_id
    )
}

fn structural_source_reliability_em_ledger(
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralSourceReliabilityEmLedger {
    let mut ledger = StructuralSourceReliabilityEmLedger::default();
    for event in &structural_prior_state.event_ledger {
        let source_label = event.source_label.trim();
        if source_label.is_empty() {
            continue;
        }
        let item = ledger
            .items
            .entry(structural_source_reliability_em_item_key(event))
            .or_default();
        item.sources.insert(source_label.to_string());
        item.last_recommended_at = Some(event.recommended_at.clone());
        ledger.distinct_sources.insert(source_label.to_string());
        if let Some(credit_class) = event
            .realized_outcome
            .as_deref()
            .and_then(structural_source_reliability_em_credit_class)
        {
            item.observed_labels += 1;
            *item
                .observed_credit_classes
                .entry(credit_class.to_string())
                .or_default() += 1;
            *item
                .source_credit_classes
                .entry(source_label.to_string())
                .or_default()
                .entry(credit_class.to_string())
                .or_default() += 1;
            ledger.observed_label_count += 1;
        }
    }
    ledger
}

fn structural_source_reliability_em_fit(
    items: &BTreeMap<String, StructuralSourceReliabilityEmItem>,
) -> StructuralSourceReliabilityEmFit {
    let fit_items = items
        .iter()
        .filter(|(_, item)| item.sources.len() >= 2 && item.observed_labels >= 2)
        .collect::<BTreeMap<_, _>>();
    let latent_item_count = fit_items.len();
    let mut labels = BTreeSet::<String>::new();
    let mut sources = BTreeSet::<String>::new();
    for item in fit_items.values() {
        labels.extend(item.observed_credit_classes.keys().cloned());
        sources.extend(item.source_credit_classes.keys().cloned());
    }
    let labels = labels.into_iter().collect::<Vec<_>>();
    let distinct_label_count = labels.len();
    let confusion_cell_count = sources.len() * distinct_label_count * distinct_label_count;
    if latent_item_count == 0 || sources.len() < 2 || distinct_label_count < 2 {
        return StructuralSourceReliabilityEmFit {
            latent_item_count,
            distinct_label_count,
            confusion_cell_count,
            ..StructuralSourceReliabilityEmFit::default()
        };
    }

    let mut posteriors = structural_source_reliability_em_initial_posteriors(&fit_items, &labels);
    let mut confusion = StructuralSourceReliabilityEmConfusion::new();
    for _ in 0..STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS {
        confusion =
            structural_source_reliability_em_confusion(&fit_items, &posteriors, &labels, &sources);
        let class_prior = structural_source_reliability_em_class_prior(&posteriors, &labels);
        posteriors = structural_source_reliability_em_update_posteriors(
            &fit_items,
            &labels,
            &class_prior,
            &confusion,
        );
    }

    let latent_confidences = posteriors
        .values()
        .filter_map(|posterior| {
            posterior
                .values()
                .copied()
                .max_by(|left, right| left.total_cmp(right))
        })
        .collect::<Vec<_>>();
    let source_reliability = sources
        .iter()
        .filter_map(|source| {
            let source_confusion = confusion.get(source)?;
            let diagonal = labels
                .iter()
                .filter_map(|label| source_confusion.get(label)?.get(label).copied())
                .collect::<Vec<_>>();
            Some((
                source.clone(),
                structural_source_reliability_em_avg(&diagonal).unwrap_or(0.5),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let source_reliabilities = source_reliability.values().copied().collect::<Vec<_>>();

    StructuralSourceReliabilityEmFit {
        iteration_count: STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS,
        latent_item_count,
        distinct_label_count,
        confusion_cell_count,
        source_reliability,
        avg_latent_confidence: structural_source_reliability_em_avg(&latent_confidences),
        min_latent_confidence: structural_source_reliability_em_min(&latent_confidences),
        avg_source_reliability: structural_source_reliability_em_avg(&source_reliabilities),
        min_source_reliability: structural_source_reliability_em_min(&source_reliabilities),
        confusion,
    }
}

fn structural_source_reliability_em_initial_posteriors(
    items: &BTreeMap<&String, &StructuralSourceReliabilityEmItem>,
    labels: &[String],
) -> StructuralSourceReliabilityEmPosteriors {
    items
        .iter()
        .map(|(item_key, item)| {
            let mut posterior = BTreeMap::<String, f64>::new();
            let denominator = item.observed_labels as f64
                + STRUCTURAL_SOURCE_RELIABILITY_EM_LAPLACE_ALPHA * labels.len() as f64;
            for label in labels {
                let count = item
                    .observed_credit_classes
                    .get(label)
                    .copied()
                    .unwrap_or_default() as f64;
                posterior.insert(
                    label.clone(),
                    ((count + STRUCTURAL_SOURCE_RELIABILITY_EM_LAPLACE_ALPHA) / denominator)
                        .clamp(0.0, 1.0),
                );
            }
            ((*item_key).clone(), posterior)
        })
        .collect()
}

fn structural_source_reliability_em_confusion(
    items: &BTreeMap<&String, &StructuralSourceReliabilityEmItem>,
    posteriors: &StructuralSourceReliabilityEmPosteriors,
    labels: &[String],
    sources: &BTreeSet<String>,
) -> StructuralSourceReliabilityEmConfusion {
    let mut confusion = StructuralSourceReliabilityEmConfusion::new();
    for source in sources {
        let mut source_rows = BTreeMap::<String, BTreeMap<String, f64>>::new();
        for true_label in labels {
            let mut observed_counts = labels
                .iter()
                .map(|label| {
                    (
                        label.clone(),
                        STRUCTURAL_SOURCE_RELIABILITY_EM_LAPLACE_ALPHA,
                    )
                })
                .collect::<BTreeMap<_, _>>();
            for (item_key, item) in items {
                let true_probability = posteriors
                    .get(*item_key)
                    .and_then(|posterior| posterior.get(true_label))
                    .copied()
                    .unwrap_or_default();
                if true_probability <= f64::EPSILON {
                    continue;
                }
                if let Some(source_labels) = item.source_credit_classes.get(source) {
                    for (observed_label, count) in source_labels {
                        *observed_counts.entry(observed_label.clone()).or_default() +=
                            true_probability * *count as f64;
                    }
                }
            }
            let denominator = observed_counts.values().sum::<f64>();
            let row = observed_counts
                .into_iter()
                .map(|(observed_label, count)| {
                    let probability = if denominator <= f64::EPSILON {
                        1.0 / labels.len() as f64
                    } else {
                        (count / denominator).clamp(0.0, 1.0)
                    };
                    (observed_label, probability)
                })
                .collect::<BTreeMap<_, _>>();
            source_rows.insert(true_label.clone(), row);
        }
        confusion.insert(source.clone(), source_rows);
    }
    confusion
}

fn structural_source_reliability_em_class_prior(
    posteriors: &StructuralSourceReliabilityEmPosteriors,
    labels: &[String],
) -> BTreeMap<String, f64> {
    let mut prior = labels
        .iter()
        .map(|label| {
            (
                label.clone(),
                STRUCTURAL_SOURCE_RELIABILITY_EM_LAPLACE_ALPHA,
            )
        })
        .collect::<BTreeMap<_, _>>();
    for posterior in posteriors.values() {
        for label in labels {
            *prior.entry(label.clone()).or_default() +=
                posterior.get(label).copied().unwrap_or_default();
        }
    }
    let denominator = prior.values().sum::<f64>();
    if denominator <= f64::EPSILON {
        return labels
            .iter()
            .map(|label| (label.clone(), 1.0 / labels.len() as f64))
            .collect();
    }
    prior
        .into_iter()
        .map(|(label, value)| (label, (value / denominator).clamp(0.0, 1.0)))
        .collect()
}

fn structural_source_reliability_em_update_posteriors(
    items: &BTreeMap<&String, &StructuralSourceReliabilityEmItem>,
    labels: &[String],
    class_prior: &BTreeMap<String, f64>,
    confusion: &StructuralSourceReliabilityEmConfusion,
) -> StructuralSourceReliabilityEmPosteriors {
    let mut posteriors = StructuralSourceReliabilityEmPosteriors::new();
    for (item_key, item) in items {
        let mut log_scores = Vec::<(String, f64)>::new();
        for true_label in labels {
            let mut log_score = class_prior
                .get(true_label)
                .copied()
                .unwrap_or(1.0 / labels.len() as f64)
                .clamp(1e-12, 1.0)
                .ln();
            for (source, observed_labels) in &item.source_credit_classes {
                for (observed_label, count) in observed_labels {
                    let likelihood = confusion
                        .get(source)
                        .and_then(|source_rows| source_rows.get(true_label))
                        .and_then(|row| row.get(observed_label))
                        .copied()
                        .unwrap_or(1.0 / labels.len() as f64)
                        .clamp(1e-12, 1.0);
                    log_score += *count as f64 * likelihood.ln();
                }
            }
            log_scores.push((true_label.clone(), log_score));
        }
        let max_log_score = log_scores
            .iter()
            .map(|(_, score)| *score)
            .fold(f64::NEG_INFINITY, f64::max);
        let denominator = log_scores
            .iter()
            .map(|(_, score)| (*score - max_log_score).exp())
            .sum::<f64>();
        let posterior = log_scores
            .into_iter()
            .map(|(label, score)| {
                let probability = if denominator <= f64::EPSILON {
                    1.0 / labels.len() as f64
                } else {
                    ((score - max_log_score).exp() / denominator).clamp(0.0, 1.0)
                };
                (label, probability)
            })
            .collect::<BTreeMap<_, _>>();
        posteriors.insert((*item_key).clone(), posterior);
    }
    posteriors
}

fn structural_source_reliability_em_avg(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some((values.iter().sum::<f64>() / values.len() as f64).clamp(0.0, 1.0))
    }
}

fn structural_source_reliability_em_min(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .min_by(|left, right| left.total_cmp(right))
}

fn structural_source_reliability_em_credit_class(outcome: &str) -> Option<&'static str> {
    let outcome = outcome.trim().to_ascii_lowercase();
    if outcome.is_empty() || structural_feedback_outcome_is_unresolved(&outcome) {
        return None;
    }
    match outcome.as_str() {
        "win" | "profit" | "tp" | "take_profit" => Some("positive_executed"),
        "loss" | "lose" | "sl" | "stop" | "stop_loss" | "invalidated" => Some("negative_executed"),
        "breakeven" | "abandoned" => Some("neutral_executed"),
        "not_followed" => Some("no_credit_not_followed"),
        _ => Some("other_observed"),
    }
}

fn structural_source_reliability_em_consensus_confidence(
    item: &StructuralSourceReliabilityEmItem,
) -> Option<f64> {
    if item.observed_labels < 2 {
        return None;
    }
    let max_class_count = item.observed_credit_classes.values().copied().max()?;
    Some((max_class_count as f64 / item.observed_labels as f64).clamp(0.0, 1.0))
}

pub fn structural_resolved_smoothed_prior(
    prior_stats: Option<&StructuralPriorStats>,
    structural_prior_state: &StructuralPriorLearningState,
    fallback: f64,
) -> f64 {
    prior_stats
        .map(|stats| {
            structural_panel_derived_smoothed_prior(stats, structural_prior_state)
                .unwrap_or(stats.smoothed_prior)
        })
        .unwrap_or(fallback)
}

pub fn structural_resolved_observations(
    prior_stats: Option<&StructuralPriorStats>,
    fallback: usize,
) -> usize {
    prior_stats
        .map(|stats| stats.observations)
        .unwrap_or(fallback)
}

pub fn structural_resolved_followed_count(
    prior_stats: Option<&StructuralPriorStats>,
    fallback: usize,
) -> usize {
    prior_stats
        .map(|stats| stats.followed_count)
        .unwrap_or(fallback)
}

pub fn structural_prior_stats_win_rate(prior_stats: Option<&StructuralPriorStats>) -> Option<f64> {
    let stats = prior_stats?;
    if stats.followed_count == 0 {
        None
    } else {
        Some(stats.wins as f64 / stats.followed_count as f64)
    }
}

pub fn structural_prior_stats_invalidation_rate(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    let stats = prior_stats?;
    if stats.followed_count == 0 {
        None
    } else {
        Some(stats.invalidated as f64 / stats.followed_count as f64)
    }
}

pub fn structural_resolved_avg_pnl(
    prior_stats: Option<&StructuralPriorStats>,
    fallback: Option<f64>,
) -> Option<f64> {
    prior_stats.map(|stats| stats.avg_pnl).or(fallback)
}

pub fn structural_resolved_node_win_rate(
    prior_stats: Option<&StructuralPriorStats>,
    historical_summary: Option<&StructuralNodeOutcomeSummary>,
) -> Option<f64> {
    structural_prior_stats_win_rate(prior_stats)
        .or_else(|| structural_node_history_win_rate(historical_summary))
}

pub fn structural_resolved_node_invalidation_rate(
    prior_stats: Option<&StructuralPriorStats>,
    historical_summary: Option<&StructuralNodeOutcomeSummary>,
) -> Option<f64> {
    structural_prior_stats_invalidation_rate(prior_stats)
        .or_else(|| structural_node_history_invalidation_rate(historical_summary))
}

pub fn structural_resolved_branch_win_rate(
    prior_stats: Option<&StructuralPriorStats>,
    historical_summary: Option<&StructuralBranchOutcomeSummary>,
) -> Option<f64> {
    structural_prior_stats_win_rate(prior_stats)
        .or_else(|| structural_branch_history_win_rate(historical_summary))
}

pub fn structural_resolved_branch_invalidation_rate(
    prior_stats: Option<&StructuralPriorStats>,
    historical_summary: Option<&StructuralBranchOutcomeSummary>,
) -> Option<f64> {
    structural_prior_stats_invalidation_rate(prior_stats)
        .or_else(|| structural_branch_history_invalidation_rate(historical_summary))
}

pub fn structural_resolved_scenario_win_rate(
    prior_stats: Option<&StructuralPriorStats>,
    historical_summary: Option<&StructuralScenarioOutcomeSummary>,
) -> Option<f64> {
    structural_prior_stats_win_rate(prior_stats)
        .or_else(|| structural_scenario_history_win_rate(historical_summary))
}

pub fn structural_resolved_scenario_invalidation_rate(
    prior_stats: Option<&StructuralPriorStats>,
    historical_summary: Option<&StructuralScenarioOutcomeSummary>,
) -> Option<f64> {
    structural_prior_stats_invalidation_rate(prior_stats)
        .or_else(|| structural_scenario_history_invalidation_rate(historical_summary))
}

pub fn structural_resolved_path_win_rate(
    prior_stats: Option<&StructuralPriorStats>,
    historical_summary: Option<&StructuralPathOutcomeSummary>,
) -> Option<f64> {
    structural_prior_stats_win_rate(prior_stats)
        .or_else(|| structural_history_win_rate(historical_summary))
}

pub fn structural_resolved_path_invalidation_rate(
    prior_stats: Option<&StructuralPriorStats>,
    historical_summary: Option<&StructuralPathOutcomeSummary>,
) -> Option<f64> {
    structural_prior_stats_invalidation_rate(prior_stats)
        .or_else(|| structural_history_invalidation_rate(historical_summary))
}

pub fn structural_history_adjusted_branch_prior(
    base_prior: f64,
    historical_summary: Option<&StructuralBranchOutcomeSummary>,
) -> f64 {
    structural_history_adjusted_prior(
        base_prior,
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary.map(|summary| summary.wins).unwrap_or(0),
        historical_summary
            .map(|summary| summary.breakevens)
            .unwrap_or(0),
    )
}

pub fn structural_history_adjusted_node_prior(
    base_prior: f64,
    historical_summary: Option<&StructuralNodeOutcomeSummary>,
) -> f64 {
    structural_history_adjusted_prior(
        base_prior,
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary.map(|summary| summary.wins).unwrap_or(0),
        historical_summary
            .map(|summary| summary.breakevens)
            .unwrap_or(0),
    )
}

pub fn structural_history_adjusted_scenario_prior(
    base_prior: f64,
    historical_summary: Option<&StructuralScenarioOutcomeSummary>,
) -> f64 {
    structural_history_adjusted_prior(
        base_prior,
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary.map(|summary| summary.wins).unwrap_or(0),
        historical_summary
            .map(|summary| summary.breakevens)
            .unwrap_or(0),
    )
}

pub fn structural_history_adjusted_prior(
    base_prior: f64,
    followed_count: usize,
    wins: usize,
    breakevens: usize,
) -> f64 {
    if followed_count == 0 {
        return base_prior;
    }
    let empirical_success = (wins as f64 + breakevens as f64 * 0.5) / followed_count as f64;
    let sample_weight = (followed_count as f64 / 5.0).min(1.0);
    (base_prior * (1.0 - sample_weight) + empirical_success * sample_weight).clamp(0.0, 1.0)
}

pub fn structural_history_win_rate_from_counts(followed_count: usize, wins: usize) -> Option<f64> {
    if followed_count == 0 {
        None
    } else {
        Some(wins as f64 / followed_count as f64)
    }
}

pub fn structural_history_invalidation_rate_from_counts(
    followed_count: usize,
    invalidated: usize,
) -> Option<f64> {
    if followed_count == 0 {
        None
    } else {
        Some(invalidated as f64 / followed_count as f64)
    }
}

pub fn structural_node_history_win_rate(
    historical_summary: Option<&StructuralNodeOutcomeSummary>,
) -> Option<f64> {
    structural_history_win_rate_from_counts(
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary.map(|summary| summary.wins).unwrap_or(0),
    )
}

pub fn structural_node_history_invalidation_rate(
    historical_summary: Option<&StructuralNodeOutcomeSummary>,
) -> Option<f64> {
    structural_history_invalidation_rate_from_counts(
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary
            .map(|summary| summary.invalidated)
            .unwrap_or(0),
    )
}

pub fn structural_branch_history_win_rate(
    historical_summary: Option<&StructuralBranchOutcomeSummary>,
) -> Option<f64> {
    structural_history_win_rate_from_counts(
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary.map(|summary| summary.wins).unwrap_or(0),
    )
}

pub fn structural_branch_history_invalidation_rate(
    historical_summary: Option<&StructuralBranchOutcomeSummary>,
) -> Option<f64> {
    structural_history_invalidation_rate_from_counts(
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary
            .map(|summary| summary.invalidated)
            .unwrap_or(0),
    )
}

pub fn structural_scenario_history_win_rate(
    historical_summary: Option<&StructuralScenarioOutcomeSummary>,
) -> Option<f64> {
    structural_history_win_rate_from_counts(
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary.map(|summary| summary.wins).unwrap_or(0),
    )
}

pub fn structural_scenario_history_invalidation_rate(
    historical_summary: Option<&StructuralScenarioOutcomeSummary>,
) -> Option<f64> {
    structural_history_invalidation_rate_from_counts(
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary
            .map(|summary| summary.invalidated)
            .unwrap_or(0),
    )
}

pub fn structural_history_adjusted_path_prior(
    base_prior: f64,
    historical_summary: Option<&StructuralPathOutcomeSummary>,
) -> f64 {
    let Some(summary) = historical_summary else {
        return base_prior;
    };
    structural_history_adjusted_prior(
        base_prior,
        summary.followed_count,
        summary.wins,
        summary.breakevens,
    )
}

pub fn structural_composite_preference_score(
    bbn_support_score: f64,
    history_adjusted_prior: f64,
) -> f64 {
    (bbn_support_score * 0.70 + history_adjusted_prior * 0.30).clamp(0.0, 1.0)
}

pub fn structural_history_win_rate(
    historical_summary: Option<&StructuralPathOutcomeSummary>,
) -> Option<f64> {
    structural_history_win_rate_from_counts(
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary.map(|summary| summary.wins).unwrap_or(0),
    )
}

pub fn structural_history_invalidation_rate(
    historical_summary: Option<&StructuralPathOutcomeSummary>,
) -> Option<f64> {
    structural_history_invalidation_rate_from_counts(
        historical_summary
            .map(|summary| summary.followed_count)
            .unwrap_or(0),
        historical_summary
            .map(|summary| summary.invalidated)
            .unwrap_or(0),
    )
}

pub fn structural_panel_derived_smoothed_prior(
    stats: &StructuralPriorStats,
    structural_prior_state: &StructuralPriorLearningState,
) -> Option<f64> {
    let em_fit = structural_source_reliability_em_fit_from_state(structural_prior_state);
    let em_source_reliability = if em_fit.iteration_count == 0 {
        None
    } else {
        Some(&em_fit.source_reliability)
    };
    let success_mass: f64 = stats
        .source_panel_summaries
        .iter()
        .map(|(source_label, summary)| {
            summary.weighted_success_mass.max(0.0)
                * structural_source_reliability_multiplier(
                    structural_prior_state,
                    source_label,
                    em_source_reliability,
                )
        })
        .sum();
    let failure_mass: f64 = stats
        .source_panel_summaries
        .iter()
        .map(|(source_label, summary)| {
            summary.weighted_failure_mass.max(0.0)
                * structural_source_reliability_multiplier(
                    structural_prior_state,
                    source_label,
                    em_source_reliability,
                )
        })
        .sum();
    if success_mass <= f64::EPSILON && failure_mass <= f64::EPSILON {
        return None;
    }
    let alpha = 1.0 + success_mass;
    let beta = 1.0 + failure_mass;
    Some((alpha / (alpha + beta)).clamp(0.0, 1.0))
}

pub fn structural_source_reliability_multiplier(
    structural_prior_state: &StructuralPriorLearningState,
    source_label: &str,
    em_source_reliability: Option<&BTreeMap<String, f64>>,
) -> f64 {
    let posterior = structural_prior_state
        .source_reliability_posteriors
        .get(source_label);
    let posterior_multiplier = posterior
        .map(|posterior| {
            if posterior.observations == 0 && posterior.weighted_observation_mass <= f64::EPSILON {
                1.0
            } else {
                posterior.posterior_reliability.clamp(0.0, 1.0)
                    * structural_source_confusion_concentration_multiplier(posterior).unwrap_or(1.0)
            }
        })
        .unwrap_or(1.0);
    let Some(em_multiplier) = em_source_reliability
        .and_then(|source_reliability| source_reliability.get(source_label))
        .copied()
        .map(|value| value.clamp(0.0, 1.0))
    else {
        return posterior_multiplier;
    };

    if posterior.is_some() {
        (posterior_multiplier * 0.5 + em_multiplier * 0.5).clamp(0.0, 1.0)
    } else {
        em_multiplier
    }
}

pub fn structural_source_confusion_concentration_multiplier(
    posterior: &StructuralSourceReliabilityPosterior,
) -> Option<f64> {
    let mut weighted_likelihood_mass = 0.0;
    let mut weighted_observation_mass = 0.0;
    for cell in posterior.outcome_confusion.values() {
        let cell_mass = cell.weighted_observation_mass.max(0.0);
        if cell_mass <= f64::EPSILON {
            continue;
        }
        weighted_likelihood_mass += cell_mass
            * structural_source_observed_outcome_likelihood(
                posterior,
                &cell.credit_class,
                &cell.observed_outcome,
            );
        weighted_observation_mass += cell_mass;
    }

    if weighted_observation_mass <= f64::EPSILON {
        None
    } else {
        Some((weighted_likelihood_mass / weighted_observation_mass).clamp(0.0, 1.0))
    }
}

pub fn structural_source_reliability_em_readiness(
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralSourceReliabilityEmReadiness {
    let diagnostics = structural_source_reliability_em_diagnostics(structural_prior_state);
    let ready = diagnostics.distinct_source_count >= 2
        && diagnostics.multi_source_item_count
            >= STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_MULTI_SOURCE_ITEMS;
    let status = if ready {
        "ready"
    } else if diagnostics.distinct_source_count < 2 {
        "needs_multiple_sources"
    } else {
        "needs_multi_source_overlap"
    };
    StructuralSourceReliabilityEmReadiness {
        ready,
        status: status.to_string(),
        candidate_item_count: diagnostics.candidate_item_count,
        labeled_item_count: diagnostics.labeled_item_count,
        multi_source_item_count: diagnostics.multi_source_item_count,
        distinct_source_count: diagnostics.distinct_source_count,
        observed_label_count: diagnostics.observed_label_count,
        max_sources_per_item: diagnostics.max_sources_per_item,
        min_multi_source_items: STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_MULTI_SOURCE_ITEMS,
        consensus_item_count: diagnostics.consensus_item_count,
        conflict_item_count: diagnostics.conflict_item_count,
        avg_consensus_confidence: diagnostics.avg_consensus_confidence,
        min_consensus_confidence: diagnostics.min_consensus_confidence,
        em_iteration_count: diagnostics.fit.iteration_count,
        em_latent_item_count: diagnostics.fit.latent_item_count,
        em_distinct_label_count: diagnostics.fit.distinct_label_count,
        em_confusion_cell_count: diagnostics.fit.confusion_cell_count,
        avg_em_latent_confidence: diagnostics.fit.avg_latent_confidence,
        min_em_latent_confidence: diagnostics.fit.min_latent_confidence,
        avg_em_source_reliability: diagnostics.fit.avg_source_reliability,
        min_em_source_reliability: diagnostics.fit.min_source_reliability,
        persisted_source_summary_count: diagnostics.persisted_source_summary_count,
        persisted_confusion_cell_count: diagnostics.persisted_confusion_cell_count,
        avg_persisted_source_reliability: diagnostics.avg_persisted_source_reliability,
        min_persisted_source_reliability: diagnostics.min_persisted_source_reliability,
        em_calibration_status: diagnostics
            .calibration
            .as_ref()
            .map(|calibration| calibration.status.clone()),
        em_calibration_observation_count: diagnostics
            .calibration
            .as_ref()
            .map(|calibration| calibration.observation_count)
            .unwrap_or_default(),
        em_calibration_source_count: diagnostics
            .calibration
            .as_ref()
            .map(|calibration| calibration.source_count)
            .unwrap_or_default(),
        em_calibration_min_observations: diagnostics
            .calibration
            .as_ref()
            .map(|calibration| calibration.min_observations)
            .unwrap_or_default(),
        em_calibration_brier_score: diagnostics
            .calibration
            .as_ref()
            .and_then(|calibration| calibration.brier_score),
        em_calibration_log_loss: diagnostics
            .calibration
            .as_ref()
            .and_then(|calibration| calibration.log_loss),
        em_holdout_status: diagnostics
            .holdout
            .as_ref()
            .map(|holdout| holdout.status.clone()),
        em_holdout_split_strategy: diagnostics
            .holdout
            .as_ref()
            .map(|holdout| holdout.split_strategy.clone()),
        em_holdout_training_item_count: diagnostics
            .holdout
            .as_ref()
            .map(|holdout| holdout.training_item_count)
            .unwrap_or_default(),
        em_holdout_evaluation_item_count: diagnostics
            .holdout
            .as_ref()
            .map(|holdout| holdout.evaluation_item_count)
            .unwrap_or_default(),
        em_holdout_observation_count: diagnostics
            .holdout
            .as_ref()
            .map(|holdout| holdout.observation_count)
            .unwrap_or_default(),
        em_holdout_source_count: diagnostics
            .holdout
            .as_ref()
            .map(|holdout| holdout.source_count)
            .unwrap_or_default(),
        em_holdout_min_training_items: diagnostics
            .holdout
            .as_ref()
            .map(|holdout| holdout.min_training_items)
            .unwrap_or(STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_HOLDOUT_TRAIN_ITEMS),
        em_holdout_min_observations: diagnostics
            .holdout
            .as_ref()
            .map(|holdout| holdout.min_observations)
            .unwrap_or_default(),
        em_holdout_brier_score: diagnostics
            .holdout
            .as_ref()
            .and_then(|holdout| holdout.brier_score),
        em_holdout_log_loss: diagnostics
            .holdout
            .as_ref()
            .and_then(|holdout| holdout.log_loss),
        em_holdout_observation_coverage: diagnostics.holdout.as_ref().and_then(|holdout| {
            (holdout.min_observations > 0).then_some(
                (holdout.observation_count as f64 / holdout.min_observations as f64)
                    .clamp(0.0, 1.0),
            )
        }),
        em_replay_status: diagnostics
            .replay
            .as_ref()
            .map(|replay| replay.status.clone()),
        em_replay_split_strategy: diagnostics
            .replay
            .as_ref()
            .map(|replay| replay.split_strategy.clone()),
        em_replay_evaluation_item_count: diagnostics
            .replay
            .as_ref()
            .map(|replay| replay.evaluation_item_count)
            .unwrap_or_default(),
        em_replay_observation_count: diagnostics
            .replay
            .as_ref()
            .map(|replay| replay.observation_count)
            .unwrap_or_default(),
        em_replay_source_count: diagnostics
            .replay
            .as_ref()
            .map(|replay| replay.source_count)
            .unwrap_or_default(),
        em_replay_brier_score: diagnostics
            .replay
            .as_ref()
            .and_then(|replay| replay.brier_score),
        em_replay_log_loss: diagnostics
            .replay
            .as_ref()
            .and_then(|replay| replay.log_loss),
        em_replay_observation_coverage: diagnostics.replay.as_ref().and_then(|replay| {
            (replay.min_observations > 0).then_some(
                (replay.observation_count as f64 / replay.min_observations as f64).clamp(0.0, 1.0),
            )
        }),
    }
}

pub fn structural_target_policy_context_surfaces(
    structural_prior_state: &StructuralPriorLearningState,
) -> Vec<StructuralTargetPolicyContextSurface> {
    let mut contexts = structural_prior_state
        .target_policy_context_posteriors
        .iter()
        .filter(|(_, posterior)| posterior.weighted_observation_mass > f64::EPSILON)
        .collect::<Vec<_>>();
    contexts.sort_by(|(left_key, left), (right_key, right)| {
        right
            .weighted_observation_mass
            .total_cmp(&left.weighted_observation_mass)
            .then_with(|| left_key.cmp(right_key))
    });
    contexts
        .into_iter()
        .take(STRUCTURAL_TARGET_POLICY_CONTEXT_SURFACE_LIMIT)
        .map(|(context_key, posterior)| {
            structural_target_policy_context_surface(context_key, posterior)
        })
        .collect()
}

pub fn structural_delayed_reward_replay_validation(
    records: &[&FeedbackRecord],
) -> Option<StructuralDelayedRewardReplayValidationSurface> {
    let mut followed_records = records
        .iter()
        .filter_map(|record| {
            let refs = record.structural_feedback.as_ref()?;
            if !refs.followed_path {
                return None;
            }
            let recommended_at = DateTime::parse_from_rfc3339(refs.recommended_at.trim())
                .ok()?
                .with_timezone(&Utc);
            let elapsed_hours = record
                .timestamp
                .signed_duration_since(recommended_at)
                .num_seconds()
                .max(0) as f64
                / 3600.0;
            Some((*record, elapsed_hours, recommended_at))
        })
        .collect::<Vec<_>>();
    followed_records.sort_by_key(|(record, _, _)| record.timestamp);
    if followed_records.is_empty() {
        return None;
    }

    let min_training_records = STRUCTURAL_DELAYED_REWARD_REPLAY_MIN_TRAIN_RECORDS;
    if followed_records.len() <= min_training_records {
        return Some(StructuralDelayedRewardReplayValidationSurface {
            status: "needs_more_history".to_string(),
            training_record_count: followed_records.len(),
            evaluation_record_count: 0,
            latest_training_recommended_at: followed_records
                .last()
                .map(|(_, _, recommended_at)| recommended_at.to_rfc3339()),
            first_evaluation_recommended_at: None,
            last_evaluation_recommended_at: None,
            resolution_observation_count: 0,
            resolution_1h_observation_count: 0,
            resolution_4h_observation_count: 0,
            resolution_24h_observation_count: 0,
            min_training_records,
            resolution_brier_score: None,
            resolution_1h_brier_score: None,
            resolution_4h_brier_score: None,
            resolution_24h_brier_score: None,
        });
    }

    let split_index =
        ((followed_records.len() * 2) / 3).clamp(min_training_records, followed_records.len() - 1);
    let training = &followed_records[..split_index];
    let evaluation = &followed_records[split_index..];

    #[derive(Default)]
    struct HorizonStats {
        observations: usize,
        within: usize,
    }

    let mut matured_train = 0usize;
    let mut horizon_1h = HorizonStats::default();
    let mut horizon_4h = HorizonStats::default();
    let mut horizon_24h = HorizonStats::default();

    for (record, elapsed_hours, _) in training {
        let matured = !structural_feedback_outcome_is_unresolved(&record.realized_outcome);
        if matured {
            matured_train += 1;
        }
        for (horizon_hours, stats) in [
            (1.0, &mut horizon_1h),
            (4.0, &mut horizon_4h),
            (24.0, &mut horizon_24h),
        ] {
            if *elapsed_hours >= horizon_hours || matured {
                stats.observations += 1;
                if matured && *elapsed_hours <= horizon_hours {
                    stats.within += 1;
                }
            }
        }
    }

    let predicted_resolution =
        ((1.0 + matured_train as f64) / (2.0 + training.len() as f64)).clamp(0.0, 1.0);
    let predicted_horizon = |stats: &HorizonStats| -> Option<f64> {
        (stats.observations > 0).then_some(
            ((1.0 + stats.within as f64) / (2.0 + stats.observations as f64)).clamp(0.0, 1.0),
        )
    };
    let predicted_1h = predicted_horizon(&horizon_1h);
    let predicted_4h = predicted_horizon(&horizon_4h);
    let predicted_24h = predicted_horizon(&horizon_24h);

    let mut resolution_brier = 0.0;
    let mut resolution_count = 0usize;
    let mut resolution_1h_brier = 0.0;
    let mut resolution_1h_count = 0usize;
    let mut resolution_4h_brier = 0.0;
    let mut resolution_4h_count = 0usize;
    let mut resolution_24h_brier = 0.0;
    let mut resolution_24h_count = 0usize;

    for (record, elapsed_hours, _) in evaluation {
        let matured = !structural_feedback_outcome_is_unresolved(&record.realized_outcome);
        let actual_resolution = if matured { 1.0 } else { 0.0 };
        resolution_brier += (predicted_resolution - actual_resolution).powi(2);
        resolution_count += 1;

        for (horizon_hours, predicted, brier, count) in [
            (
                1.0,
                predicted_1h,
                &mut resolution_1h_brier,
                &mut resolution_1h_count,
            ),
            (
                4.0,
                predicted_4h,
                &mut resolution_4h_brier,
                &mut resolution_4h_count,
            ),
            (
                24.0,
                predicted_24h,
                &mut resolution_24h_brier,
                &mut resolution_24h_count,
            ),
        ] {
            let Some(predicted) = predicted else {
                continue;
            };
            if *elapsed_hours >= horizon_hours || matured {
                let actual = if matured && *elapsed_hours <= horizon_hours {
                    1.0
                } else {
                    0.0
                };
                *brier += (predicted - actual).powi(2);
                *count += 1;
            }
        }
    }

    let status = if evaluation.is_empty() {
        "needs_more_history"
    } else {
        "ready"
    };
    Some(StructuralDelayedRewardReplayValidationSurface {
        status: status.to_string(),
        training_record_count: training.len(),
        evaluation_record_count: evaluation.len(),
        latest_training_recommended_at: training
            .last()
            .map(|(_, _, recommended_at)| recommended_at.to_rfc3339()),
        first_evaluation_recommended_at: evaluation
            .first()
            .map(|(_, _, recommended_at)| recommended_at.to_rfc3339()),
        last_evaluation_recommended_at: evaluation
            .last()
            .map(|(_, _, recommended_at)| recommended_at.to_rfc3339()),
        resolution_observation_count: resolution_count,
        resolution_1h_observation_count: resolution_1h_count,
        resolution_4h_observation_count: resolution_4h_count,
        resolution_24h_observation_count: resolution_24h_count,
        min_training_records,
        resolution_brier_score: (resolution_count > 0)
            .then_some((resolution_brier / resolution_count as f64).clamp(0.0, 1.0)),
        resolution_1h_brier_score: (resolution_1h_count > 0)
            .then_some((resolution_1h_brier / resolution_1h_count as f64).clamp(0.0, 1.0)),
        resolution_4h_brier_score: (resolution_4h_count > 0)
            .then_some((resolution_4h_brier / resolution_4h_count as f64).clamp(0.0, 1.0)),
        resolution_24h_brier_score: (resolution_24h_count > 0)
            .then_some((resolution_24h_brier / resolution_24h_count as f64).clamp(0.0, 1.0)),
    })
}

pub fn structural_target_policy_context_surface(
    context_key: &str,
    posterior: &StructuralTargetPolicyContextPosterior,
) -> StructuralTargetPolicyContextSurface {
    StructuralTargetPolicyContextSurface {
        context_key: context_key.to_string(),
        observations: posterior.observations,
        weighted_observation_mass: posterior.weighted_observation_mass,
        behavior_policy_probability: posterior.behavior_policy_probability,
        behavior_policy_probability_variance: posterior.behavior_policy_probability_variance,
        learned_target_policy_probability: posterior.learned_target_policy_probability,
        learned_target_policy_probability_lower_bound: posterior
            .learned_target_policy_probability_lower_bound,
        learned_target_policy_probability_confidence: posterior
            .learned_target_policy_probability_confidence,
        calibrated_target_policy_probability: posterior.calibrated_target_policy_probability,
        calibrated_target_policy_probability_lower_bound: posterior
            .calibrated_target_policy_probability_lower_bound,
        target_policy_probability_brier_score: posterior.target_policy_probability_brier_score,
        target_policy_probability_calibration_error: posterior
            .target_policy_probability_calibration_error,
        last_recommendation_id: posterior.last_recommendation_id.clone(),
    }
}

pub fn structural_source_panel_count(prior_stats: Option<&StructuralPriorStats>) -> usize {
    prior_stats
        .map(|stats| stats.source_panel_summaries.len())
        .unwrap_or(0)
}

pub fn structural_last_offline_seed_source(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<String> {
    prior_stats.and_then(|stats| stats.last_offline_seed_source.clone())
}

pub fn structural_prior_positive_value(
    prior_stats: Option<&StructuralPriorStats>,
    value: impl Fn(&StructuralPriorStats) -> f64,
) -> Option<f64> {
    prior_stats
        .map(value)
        .filter(|candidate| *candidate > f64::EPSILON)
}

pub fn structural_prior_positive_count(
    prior_stats: Option<&StructuralPriorStats>,
    value: impl Fn(&StructuralPriorStats) -> usize,
) -> Option<usize> {
    prior_stats.map(value).filter(|candidate| *candidate > 0)
}

pub fn structural_prior_execution_propensity(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.execution_propensity)
}

pub fn structural_prior_ips_weight(prior_stats: Option<&StructuralPriorStats>) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.ips_weight)
}

pub fn structural_prior_counterfactual_reward_prior(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.counterfactual_reward_prior)
}

pub fn structural_prior_off_policy_adjusted_prior(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.off_policy_adjusted_prior)
}

pub fn structural_prior_behavior_policy_probability(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.behavior_policy_probability)
}

pub fn structural_prior_behavior_policy_probability_variance(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.behavior_policy_probability_variance
    })
}

pub fn structural_prior_target_policy_probability_confidence(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.target_policy_probability_confidence
    })
}

pub fn structural_prior_target_policy_probability_lower_bound(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.target_policy_probability_lower_bound
    })
}

pub fn structural_prior_target_policy_probability_brier_score(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.target_policy_probability_brier_score
    })
}

pub fn structural_prior_target_policy_probability_calibration_error(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.target_policy_probability_calibration_error
    })
}

pub fn structural_prior_snips_weight_mass(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.snips_weight_mass)
}

pub fn structural_prior_snips_weight_squared_mass(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.snips_weight_squared_mass)
}

pub fn structural_prior_snips_effective_sample_size(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.snips_effective_sample_size)
}

pub fn structural_prior_snips_reward_prior(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.snips_reward_prior)
}

pub fn structural_prior_doubly_robust_reward_prior(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.doubly_robust_reward_prior)
}

pub fn structural_prior_target_policy_calibration_weight(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.target_policy_calibration_weight)
}

pub fn structural_prior_target_policy_reward_prior(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.target_policy_reward_prior)
}

pub fn structural_prior_target_policy_variance_penalty(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.target_policy_variance_penalty)
}

pub fn structural_prior_target_policy_reward_lower_bound(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.target_policy_reward_lower_bound)
}

fn structural_matured_feedback_count_value(stats: &StructuralPriorStats) -> usize {
    structural_delayed_reward_matured_feedback_count(
        stats.wins,
        stats.losses,
        stats.breakevens,
        stats.invalidated,
        stats.abandoned,
    )
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StructuralDelayedRewardCompetingRisks {
    pub(crate) success: f64,
    pub(crate) failure: f64,
    pub(crate) invalidation: f64,
    pub(crate) abandonment: f64,
    pub(crate) entropy: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StructuralDelayedRewardElapsedHazards {
    pub(crate) success: f64,
    pub(crate) failure: f64,
    pub(crate) invalidation: f64,
    pub(crate) abandonment: f64,
}

pub(crate) fn structural_delayed_reward_matured_feedback_count(
    wins: usize,
    losses: usize,
    breakevens: usize,
    invalidated: usize,
    abandoned: usize,
) -> usize {
    wins + losses + breakevens + invalidated + abandoned
}

pub(crate) fn structural_delayed_reward_censoring_rate(
    followed_count: usize,
    matured_feedback_count: usize,
) -> Option<f64> {
    if followed_count == 0 {
        None
    } else {
        Some(
            (followed_count.saturating_sub(matured_feedback_count) as f64 / followed_count as f64)
                .clamp(0.0, 1.0),
        )
    }
}

pub(crate) fn structural_delayed_reward_resolution_probability(
    followed_count: usize,
    matured_feedback_count: usize,
) -> f64 {
    if followed_count == 0 {
        return 0.0;
    }
    let matured = matured_feedback_count.min(followed_count) as f64;
    ((1.0 + matured) / (2.0 + followed_count as f64)).clamp(0.0, 1.0)
}

pub(crate) fn structural_delayed_reward_censoring_probability(
    followed_count: usize,
    matured_feedback_count: usize,
) -> f64 {
    if followed_count == 0 {
        return 0.0;
    }
    let unresolved = followed_count.saturating_sub(matured_feedback_count) as f64;
    ((1.0 + unresolved) / (2.0 + followed_count as f64)).clamp(0.0, 1.0)
}

pub(crate) fn structural_delayed_reward_elapsed_hours(
    record: &FeedbackRecord,
    followed_path: bool,
) -> Option<f64> {
    if !followed_path
        || record
            .realized_outcome
            .trim()
            .eq_ignore_ascii_case("not_followed")
    {
        return None;
    }
    let refs = record.structural_feedback.as_ref()?;
    let recommended_at = DateTime::parse_from_rfc3339(refs.recommended_at.trim())
        .ok()?
        .with_timezone(&Utc);
    let elapsed_seconds = record
        .timestamp
        .signed_duration_since(recommended_at)
        .num_seconds();
    (elapsed_seconds >= 0).then_some(elapsed_seconds as f64 / 3600.0)
}

pub(crate) fn structural_delayed_reward_avg_elapsed_hours(count: usize, elapsed_hours: f64) -> f64 {
    if count == 0 {
        0.0
    } else {
        (elapsed_hours.max(0.0) / count as f64).max(0.0)
    }
}

pub(crate) fn structural_delayed_reward_resolution_hazard_per_hour(
    matured_feedback_count: usize,
    elapsed_hours_at_risk: f64,
) -> f64 {
    if matured_feedback_count == 0 || elapsed_hours_at_risk <= f64::EPSILON {
        0.0
    } else {
        (matured_feedback_count as f64 / elapsed_hours_at_risk.max(f64::EPSILON)).max(0.0)
    }
}

pub(crate) fn structural_delayed_reward_expected_resolution_hours(
    resolution_hazard_per_hour: f64,
) -> f64 {
    if resolution_hazard_per_hour <= f64::EPSILON {
        0.0
    } else {
        (1.0 / resolution_hazard_per_hour).max(0.0)
    }
}

pub(crate) fn structural_delayed_reward_survival_probability(
    resolution_hazard_per_hour: f64,
    horizon_hours: f64,
) -> f64 {
    if resolution_hazard_per_hour <= f64::EPSILON || horizon_hours <= f64::EPSILON {
        return 0.0;
    }
    (-resolution_hazard_per_hour * horizon_hours)
        .exp()
        .clamp(0.0, 1.0)
}

pub(crate) fn structural_delayed_reward_cumulative_incidence(
    cause_hazard_per_hour: f64,
    resolution_hazard_per_hour: f64,
    horizon_hours: f64,
) -> f64 {
    if cause_hazard_per_hour <= f64::EPSILON
        || resolution_hazard_per_hour <= f64::EPSILON
        || horizon_hours <= f64::EPSILON
    {
        return 0.0;
    }
    let cause_share = (cause_hazard_per_hour.max(0.0)
        / resolution_hazard_per_hour.max(f64::EPSILON))
    .clamp(0.0, 1.0);
    let event_probability = (1.0
        - structural_delayed_reward_survival_probability(
            resolution_hazard_per_hour,
            horizon_hours,
        ))
    .clamp(0.0, 1.0);
    (cause_share * event_probability).clamp(0.0, 1.0)
}

pub(crate) fn structural_delayed_reward_update_resolution_horizon(
    elapsed_hours: f64,
    matured: bool,
    horizon_hours: f64,
    horizon_count: &mut usize,
    within_count: &mut usize,
) {
    if elapsed_hours >= horizon_hours || matured {
        *horizon_count += 1;
        if matured && elapsed_hours <= horizon_hours {
            *within_count += 1;
        }
    }
}

struct StructuralDelayedRewardCounters<'a> {
    wins: &'a mut usize,
    losses: &'a mut usize,
    breakevens: &'a mut usize,
    invalidated: &'a mut usize,
    abandoned: &'a mut usize,
    delayed_reward_elapsed_feedback_count: &'a mut usize,
    delayed_reward_elapsed_hours_at_risk: &'a mut f64,
    delayed_reward_resolution_horizon_1h_count: &'a mut usize,
    delayed_reward_resolution_within_1h_count: &'a mut usize,
    delayed_reward_resolution_horizon_4h_count: &'a mut usize,
    delayed_reward_resolution_within_4h_count: &'a mut usize,
    delayed_reward_resolution_horizon_24h_count: &'a mut usize,
    delayed_reward_resolution_within_24h_count: &'a mut usize,
}

fn apply_structural_delayed_reward_feedback(
    counters: StructuralDelayedRewardCounters<'_>,
    record: &FeedbackRecord,
    followed_path: bool,
) {
    let StructuralDelayedRewardCounters {
        wins,
        losses,
        breakevens,
        invalidated,
        abandoned,
        delayed_reward_elapsed_feedback_count,
        delayed_reward_elapsed_hours_at_risk,
        delayed_reward_resolution_horizon_1h_count,
        delayed_reward_resolution_within_1h_count,
        delayed_reward_resolution_horizon_4h_count,
        delayed_reward_resolution_within_4h_count,
        delayed_reward_resolution_horizon_24h_count,
        delayed_reward_resolution_within_24h_count,
    } = counters;
    let counter_outcome = crate::state::structural_feedback_counter_outcome(record);
    match counter_outcome {
        Some("win") => {
            *wins += 1;
        }
        Some("loss") => {
            *losses += 1;
        }
        Some("breakeven") => {
            *breakevens += 1;
        }
        Some("invalidated") => {
            *invalidated += 1;
        }
        Some("abandoned") => {
            *abandoned += 1;
        }
        Some("not_followed") | Some(_) | None => {}
    }
    if let Some(elapsed_hours) = structural_delayed_reward_elapsed_hours(record, followed_path) {
        *delayed_reward_elapsed_feedback_count += 1;
        *delayed_reward_elapsed_hours_at_risk += elapsed_hours;
        let matured = counter_outcome.is_some();
        structural_delayed_reward_update_resolution_horizon(
            elapsed_hours,
            matured,
            1.0,
            delayed_reward_resolution_horizon_1h_count,
            delayed_reward_resolution_within_1h_count,
        );
        structural_delayed_reward_update_resolution_horizon(
            elapsed_hours,
            matured,
            4.0,
            delayed_reward_resolution_horizon_4h_count,
            delayed_reward_resolution_within_4h_count,
        );
        structural_delayed_reward_update_resolution_horizon(
            elapsed_hours,
            matured,
            24.0,
            delayed_reward_resolution_horizon_24h_count,
            delayed_reward_resolution_within_24h_count,
        );
    }
}

pub(crate) fn accumulate_structural_prior_stats_delayed_reward_observation(
    stats: &mut StructuralPriorStats,
    record: &FeedbackRecord,
    followed_path: bool,
) {
    apply_structural_delayed_reward_feedback(
        StructuralDelayedRewardCounters {
            wins: &mut stats.wins,
            losses: &mut stats.losses,
            breakevens: &mut stats.breakevens,
            invalidated: &mut stats.invalidated,
            abandoned: &mut stats.abandoned,
            delayed_reward_elapsed_feedback_count: &mut stats.delayed_reward_elapsed_feedback_count,
            delayed_reward_elapsed_hours_at_risk: &mut stats.delayed_reward_elapsed_hours_at_risk,
            delayed_reward_resolution_horizon_1h_count: &mut stats
                .delayed_reward_resolution_horizon_1h_count,
            delayed_reward_resolution_within_1h_count: &mut stats
                .delayed_reward_resolution_within_1h_count,
            delayed_reward_resolution_horizon_4h_count: &mut stats
                .delayed_reward_resolution_horizon_4h_count,
            delayed_reward_resolution_within_4h_count: &mut stats
                .delayed_reward_resolution_within_4h_count,
            delayed_reward_resolution_horizon_24h_count: &mut stats
                .delayed_reward_resolution_horizon_24h_count,
            delayed_reward_resolution_within_24h_count: &mut stats
                .delayed_reward_resolution_within_24h_count,
        },
        record,
        followed_path,
    );
}

pub(crate) fn accumulate_structural_prior_source_summary_delayed_reward_observation(
    summary: &mut StructuralPriorSourceSummary,
    record: &FeedbackRecord,
    followed_path: bool,
) {
    apply_structural_delayed_reward_feedback(
        StructuralDelayedRewardCounters {
            wins: &mut summary.wins,
            losses: &mut summary.losses,
            breakevens: &mut summary.breakevens,
            invalidated: &mut summary.invalidated,
            abandoned: &mut summary.abandoned,
            delayed_reward_elapsed_feedback_count: &mut summary
                .delayed_reward_elapsed_feedback_count,
            delayed_reward_elapsed_hours_at_risk: &mut summary.delayed_reward_elapsed_hours_at_risk,
            delayed_reward_resolution_horizon_1h_count: &mut summary
                .delayed_reward_resolution_horizon_1h_count,
            delayed_reward_resolution_within_1h_count: &mut summary
                .delayed_reward_resolution_within_1h_count,
            delayed_reward_resolution_horizon_4h_count: &mut summary
                .delayed_reward_resolution_horizon_4h_count,
            delayed_reward_resolution_within_4h_count: &mut summary
                .delayed_reward_resolution_within_4h_count,
            delayed_reward_resolution_horizon_24h_count: &mut summary
                .delayed_reward_resolution_horizon_24h_count,
            delayed_reward_resolution_within_24h_count: &mut summary
                .delayed_reward_resolution_within_24h_count,
        },
        record,
        followed_path,
    );
}

pub(crate) fn structural_delayed_reward_resolution_horizon_probability(
    within_count: usize,
    horizon_count: usize,
) -> f64 {
    if horizon_count == 0 {
        0.0
    } else {
        ((1.0 + within_count as f64) / (2.0 + horizon_count as f64)).clamp(0.0, 1.0)
    }
}

pub(crate) fn structural_delayed_reward_competing_risks(
    wins: usize,
    losses: usize,
    breakevens: usize,
    invalidated: usize,
    abandoned: usize,
) -> Option<StructuralDelayedRewardCompetingRisks> {
    if structural_delayed_reward_matured_feedback_count(
        wins,
        losses,
        breakevens,
        invalidated,
        abandoned,
    ) == 0
    {
        return None;
    }
    let success_mass = wins as f64 + breakevens as f64 * 0.5;
    let failure_mass = losses as f64 + breakevens as f64 * 0.5;
    let invalidation_mass = invalidated as f64;
    let abandonment_mass = abandoned as f64;
    let denominator = success_mass + failure_mass + invalidation_mass + abandonment_mass + 4.0;
    if denominator <= f64::EPSILON {
        return None;
    }
    let success = ((1.0 + success_mass) / denominator).clamp(0.0, 1.0);
    let failure = ((1.0 + failure_mass) / denominator).clamp(0.0, 1.0);
    let invalidation = ((1.0 + invalidation_mass) / denominator).clamp(0.0, 1.0);
    let abandonment = ((1.0 + abandonment_mass) / denominator).clamp(0.0, 1.0);
    let entropy = [success, failure, invalidation, abandonment]
        .into_iter()
        .filter(|risk| *risk > f64::EPSILON)
        .map(|risk| -risk * risk.ln())
        .sum();
    Some(StructuralDelayedRewardCompetingRisks {
        success,
        failure,
        invalidation,
        abandonment,
        entropy,
    })
}

pub(crate) fn structural_delayed_reward_elapsed_hazards(
    wins: usize,
    losses: usize,
    breakevens: usize,
    invalidated: usize,
    abandoned: usize,
    elapsed_hours_at_risk: f64,
) -> Option<StructuralDelayedRewardElapsedHazards> {
    if structural_delayed_reward_matured_feedback_count(
        wins,
        losses,
        breakevens,
        invalidated,
        abandoned,
    ) == 0
        || elapsed_hours_at_risk <= f64::EPSILON
    {
        return None;
    }
    let denominator = elapsed_hours_at_risk.max(f64::EPSILON);
    Some(StructuralDelayedRewardElapsedHazards {
        success: (wins as f64 + breakevens as f64 * 0.5) / denominator,
        failure: (losses as f64 + breakevens as f64 * 0.5) / denominator,
        invalidation: invalidated as f64 / denominator,
        abandonment: abandoned as f64 / denominator,
    })
}

pub(crate) fn structural_censoring_adjusted_reward_prior(
    target_policy_reward_prior: f64,
    smoothed_prior: f64,
    delayed_reward_resolution_probability: f64,
) -> f64 {
    let resolution = delayed_reward_resolution_probability.clamp(0.0, 1.0);
    (target_policy_reward_prior.clamp(0.0, 1.0) * resolution
        + smoothed_prior.clamp(0.0, 1.0) * (1.0 - resolution))
        .clamp(0.0, 1.0)
}

pub(crate) fn structural_censoring_adjusted_reward_lower_bound(
    target_policy_reward_lower_bound: f64,
    smoothed_prior: f64,
    delayed_reward_resolution_probability: f64,
    delayed_reward_censoring_probability: f64,
) -> f64 {
    let resolution = delayed_reward_resolution_probability.clamp(0.0, 1.0);
    let censoring = delayed_reward_censoring_probability.clamp(0.0, 1.0);
    (target_policy_reward_lower_bound.clamp(0.0, 1.0) * resolution
        + smoothed_prior.clamp(0.0, 1.0) * 0.5 * censoring)
        .clamp(0.0, 1.0)
}

pub(crate) fn refresh_structural_prior_delayed_reward_metrics(stats: &mut StructuralPriorStats) {
    let matured_feedback_count = structural_delayed_reward_matured_feedback_count(
        stats.wins,
        stats.losses,
        stats.breakevens,
        stats.invalidated,
        stats.abandoned,
    );
    stats.delayed_reward_resolution_probability = structural_delayed_reward_resolution_probability(
        stats.followed_count,
        matured_feedback_count,
    );
    stats.delayed_reward_censoring_probability = structural_delayed_reward_censoring_probability(
        stats.followed_count,
        matured_feedback_count,
    );
    stats.censoring_adjusted_reward_prior = structural_censoring_adjusted_reward_prior(
        stats.target_policy_reward_prior,
        stats.smoothed_prior,
        stats.delayed_reward_resolution_probability,
    );
    stats.censoring_adjusted_reward_lower_bound = structural_censoring_adjusted_reward_lower_bound(
        stats.target_policy_reward_lower_bound,
        stats.smoothed_prior,
        stats.delayed_reward_resolution_probability,
        stats.delayed_reward_censoring_probability,
    );
    let competing_risks = structural_delayed_reward_competing_risks(
        stats.wins,
        stats.losses,
        stats.breakevens,
        stats.invalidated,
        stats.abandoned,
    )
    .unwrap_or_default();
    stats.delayed_reward_success_competing_risk = competing_risks.success;
    stats.delayed_reward_failure_competing_risk = competing_risks.failure;
    stats.delayed_reward_invalidation_competing_risk = competing_risks.invalidation;
    stats.delayed_reward_abandonment_competing_risk = competing_risks.abandonment;
    stats.delayed_reward_competing_risk_entropy = competing_risks.entropy;
    stats.delayed_reward_avg_elapsed_hours = structural_delayed_reward_avg_elapsed_hours(
        stats.delayed_reward_elapsed_feedback_count,
        stats.delayed_reward_elapsed_hours_at_risk,
    );
    stats.delayed_reward_resolution_hazard_per_hour =
        structural_delayed_reward_resolution_hazard_per_hour(
            matured_feedback_count,
            stats.delayed_reward_elapsed_hours_at_risk,
        );
    stats.delayed_reward_expected_resolution_hours =
        structural_delayed_reward_expected_resolution_hours(
            stats.delayed_reward_resolution_hazard_per_hour,
        );
    stats.delayed_reward_survival_probability_1h = structural_delayed_reward_survival_probability(
        stats.delayed_reward_resolution_hazard_per_hour,
        1.0,
    );
    stats.delayed_reward_survival_probability_4h = structural_delayed_reward_survival_probability(
        stats.delayed_reward_resolution_hazard_per_hour,
        4.0,
    );
    stats.delayed_reward_survival_probability_24h = structural_delayed_reward_survival_probability(
        stats.delayed_reward_resolution_hazard_per_hour,
        24.0,
    );
    let elapsed_hazards = structural_delayed_reward_elapsed_hazards(
        stats.wins,
        stats.losses,
        stats.breakevens,
        stats.invalidated,
        stats.abandoned,
        stats.delayed_reward_elapsed_hours_at_risk,
    )
    .unwrap_or_default();
    stats.delayed_reward_success_hazard_per_hour = elapsed_hazards.success;
    stats.delayed_reward_failure_hazard_per_hour = elapsed_hazards.failure;
    stats.delayed_reward_invalidation_hazard_per_hour = elapsed_hazards.invalidation;
    stats.delayed_reward_abandonment_hazard_per_hour = elapsed_hazards.abandonment;
    stats.delayed_reward_success_cumulative_incidence_4h =
        structural_delayed_reward_cumulative_incidence(
            elapsed_hazards.success,
            stats.delayed_reward_resolution_hazard_per_hour,
            4.0,
        );
    stats.delayed_reward_failure_cumulative_incidence_4h =
        structural_delayed_reward_cumulative_incidence(
            elapsed_hazards.failure,
            stats.delayed_reward_resolution_hazard_per_hour,
            4.0,
        );
    stats.delayed_reward_invalidation_cumulative_incidence_4h =
        structural_delayed_reward_cumulative_incidence(
            elapsed_hazards.invalidation,
            stats.delayed_reward_resolution_hazard_per_hour,
            4.0,
        );
    stats.delayed_reward_abandonment_cumulative_incidence_4h =
        structural_delayed_reward_cumulative_incidence(
            elapsed_hazards.abandonment,
            stats.delayed_reward_resolution_hazard_per_hour,
            4.0,
        );
    stats.delayed_reward_resolution_probability_1h =
        structural_delayed_reward_resolution_horizon_probability(
            stats.delayed_reward_resolution_within_1h_count,
            stats.delayed_reward_resolution_horizon_1h_count,
        );
    stats.delayed_reward_resolution_probability_4h =
        structural_delayed_reward_resolution_horizon_probability(
            stats.delayed_reward_resolution_within_4h_count,
            stats.delayed_reward_resolution_horizon_4h_count,
        );
    stats.delayed_reward_resolution_probability_24h =
        structural_delayed_reward_resolution_horizon_probability(
            stats.delayed_reward_resolution_within_24h_count,
            stats.delayed_reward_resolution_horizon_24h_count,
        );
}

pub(crate) fn refresh_structural_source_summary_delayed_reward_metrics(
    summary: &mut StructuralPriorSourceSummary,
) {
    let matured_feedback_count = structural_delayed_reward_matured_feedback_count(
        summary.wins,
        summary.losses,
        summary.breakevens,
        summary.invalidated,
        summary.abandoned,
    );
    summary.delayed_reward_resolution_probability =
        structural_delayed_reward_resolution_probability(
            summary.followed_count,
            matured_feedback_count,
        );
    summary.delayed_reward_censoring_probability = structural_delayed_reward_censoring_probability(
        summary.followed_count,
        matured_feedback_count,
    );
    summary.censoring_adjusted_reward_prior = structural_censoring_adjusted_reward_prior(
        summary.target_policy_reward_prior,
        summary.smoothed_prior,
        summary.delayed_reward_resolution_probability,
    );
    summary.censoring_adjusted_reward_lower_bound =
        structural_censoring_adjusted_reward_lower_bound(
            summary.target_policy_reward_lower_bound,
            summary.smoothed_prior,
            summary.delayed_reward_resolution_probability,
            summary.delayed_reward_censoring_probability,
        );
    let competing_risks = structural_delayed_reward_competing_risks(
        summary.wins,
        summary.losses,
        summary.breakevens,
        summary.invalidated,
        summary.abandoned,
    )
    .unwrap_or_default();
    summary.delayed_reward_success_competing_risk = competing_risks.success;
    summary.delayed_reward_failure_competing_risk = competing_risks.failure;
    summary.delayed_reward_invalidation_competing_risk = competing_risks.invalidation;
    summary.delayed_reward_abandonment_competing_risk = competing_risks.abandonment;
    summary.delayed_reward_competing_risk_entropy = competing_risks.entropy;
    summary.delayed_reward_avg_elapsed_hours = structural_delayed_reward_avg_elapsed_hours(
        summary.delayed_reward_elapsed_feedback_count,
        summary.delayed_reward_elapsed_hours_at_risk,
    );
    summary.delayed_reward_resolution_hazard_per_hour =
        structural_delayed_reward_resolution_hazard_per_hour(
            matured_feedback_count,
            summary.delayed_reward_elapsed_hours_at_risk,
        );
    summary.delayed_reward_expected_resolution_hours =
        structural_delayed_reward_expected_resolution_hours(
            summary.delayed_reward_resolution_hazard_per_hour,
        );
    summary.delayed_reward_survival_probability_1h = structural_delayed_reward_survival_probability(
        summary.delayed_reward_resolution_hazard_per_hour,
        1.0,
    );
    summary.delayed_reward_survival_probability_4h = structural_delayed_reward_survival_probability(
        summary.delayed_reward_resolution_hazard_per_hour,
        4.0,
    );
    summary.delayed_reward_survival_probability_24h =
        structural_delayed_reward_survival_probability(
            summary.delayed_reward_resolution_hazard_per_hour,
            24.0,
        );
    let elapsed_hazards = structural_delayed_reward_elapsed_hazards(
        summary.wins,
        summary.losses,
        summary.breakevens,
        summary.invalidated,
        summary.abandoned,
        summary.delayed_reward_elapsed_hours_at_risk,
    )
    .unwrap_or_default();
    summary.delayed_reward_success_hazard_per_hour = elapsed_hazards.success;
    summary.delayed_reward_failure_hazard_per_hour = elapsed_hazards.failure;
    summary.delayed_reward_invalidation_hazard_per_hour = elapsed_hazards.invalidation;
    summary.delayed_reward_abandonment_hazard_per_hour = elapsed_hazards.abandonment;
    summary.delayed_reward_success_cumulative_incidence_4h =
        structural_delayed_reward_cumulative_incidence(
            elapsed_hazards.success,
            summary.delayed_reward_resolution_hazard_per_hour,
            4.0,
        );
    summary.delayed_reward_failure_cumulative_incidence_4h =
        structural_delayed_reward_cumulative_incidence(
            elapsed_hazards.failure,
            summary.delayed_reward_resolution_hazard_per_hour,
            4.0,
        );
    summary.delayed_reward_invalidation_cumulative_incidence_4h =
        structural_delayed_reward_cumulative_incidence(
            elapsed_hazards.invalidation,
            summary.delayed_reward_resolution_hazard_per_hour,
            4.0,
        );
    summary.delayed_reward_abandonment_cumulative_incidence_4h =
        structural_delayed_reward_cumulative_incidence(
            elapsed_hazards.abandonment,
            summary.delayed_reward_resolution_hazard_per_hour,
            4.0,
        );
    summary.delayed_reward_resolution_probability_1h =
        structural_delayed_reward_resolution_horizon_probability(
            summary.delayed_reward_resolution_within_1h_count,
            summary.delayed_reward_resolution_horizon_1h_count,
        );
    summary.delayed_reward_resolution_probability_4h =
        structural_delayed_reward_resolution_horizon_probability(
            summary.delayed_reward_resolution_within_4h_count,
            summary.delayed_reward_resolution_horizon_4h_count,
        );
    summary.delayed_reward_resolution_probability_24h =
        structural_delayed_reward_resolution_horizon_probability(
            summary.delayed_reward_resolution_within_24h_count,
            summary.delayed_reward_resolution_horizon_24h_count,
        );
}

pub fn structural_prior_matured_feedback_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    prior_stats.map(structural_matured_feedback_count_value)
}

pub fn structural_prior_unresolved_feedback_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    prior_stats.map(|stats| {
        stats
            .followed_count
            .saturating_sub(structural_matured_feedback_count_value(stats))
    })
}

pub fn structural_prior_maturity_coverage(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    let stats = prior_stats?;
    if stats.followed_count == 0 {
        None
    } else {
        Some(
            (structural_matured_feedback_count_value(stats) as f64 / stats.followed_count as f64)
                .clamp(0.0, 1.0),
        )
    }
}

pub fn structural_prior_censoring_rate(prior_stats: Option<&StructuralPriorStats>) -> Option<f64> {
    let stats = prior_stats?;
    structural_delayed_reward_censoring_rate(
        stats.followed_count,
        structural_matured_feedback_count_value(stats),
    )
}

pub fn structural_prior_delayed_reward_resolution_probability(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    let stats = prior_stats?;
    (stats.followed_count > 0).then_some(structural_delayed_reward_resolution_probability(
        stats.followed_count,
        structural_matured_feedback_count_value(stats),
    ))
}

pub fn structural_prior_delayed_reward_censoring_probability(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    let stats = prior_stats?;
    (stats.followed_count > 0).then_some(structural_delayed_reward_censoring_probability(
        stats.followed_count,
        structural_matured_feedback_count_value(stats),
    ))
}

pub fn structural_prior_censoring_adjusted_reward_prior(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    let stats = prior_stats?;
    let resolution = structural_prior_delayed_reward_resolution_probability(Some(stats))?;
    Some(structural_censoring_adjusted_reward_prior(
        stats.target_policy_reward_prior,
        stats.smoothed_prior,
        resolution,
    ))
}

pub fn structural_prior_censoring_adjusted_reward_lower_bound(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    let stats = prior_stats?;
    let resolution = structural_prior_delayed_reward_resolution_probability(Some(stats))?;
    let censoring = structural_prior_delayed_reward_censoring_probability(Some(stats))?;
    Some(structural_censoring_adjusted_reward_lower_bound(
        stats.target_policy_reward_lower_bound,
        stats.smoothed_prior,
        resolution,
        censoring,
    ))
}

fn structural_prior_delayed_reward_competing_risks(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<StructuralDelayedRewardCompetingRisks> {
    let stats = prior_stats?;
    structural_delayed_reward_competing_risks(
        stats.wins,
        stats.losses,
        stats.breakevens,
        stats.invalidated,
        stats.abandoned,
    )
}

pub fn structural_prior_delayed_reward_success_competing_risk(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_delayed_reward_competing_risks(prior_stats).map(|risks| risks.success)
}

pub fn structural_prior_delayed_reward_failure_competing_risk(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_delayed_reward_competing_risks(prior_stats).map(|risks| risks.failure)
}

pub fn structural_prior_delayed_reward_invalidation_competing_risk(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_delayed_reward_competing_risks(prior_stats).map(|risks| risks.invalidation)
}

pub fn structural_prior_delayed_reward_abandonment_competing_risk(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_delayed_reward_competing_risks(prior_stats).map(|risks| risks.abandonment)
}

pub fn structural_prior_delayed_reward_competing_risk_entropy(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_delayed_reward_competing_risks(prior_stats).map(|risks| risks.entropy)
}

pub fn structural_prior_delayed_reward_elapsed_feedback_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    structural_prior_positive_count(prior_stats, |stats| {
        stats.delayed_reward_elapsed_feedback_count
    })
}

pub fn structural_prior_delayed_reward_elapsed_hours_at_risk(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_elapsed_hours_at_risk
    })
}

pub fn structural_prior_delayed_reward_avg_elapsed_hours(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| stats.delayed_reward_avg_elapsed_hours)
}

pub fn structural_prior_delayed_reward_resolution_hazard_per_hour(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_resolution_hazard_per_hour
    })
}

pub fn structural_prior_delayed_reward_expected_resolution_hours(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_expected_resolution_hours
    })
}

pub fn structural_prior_delayed_reward_survival_probability_1h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_survival_probability_1h
    })
}

pub fn structural_prior_delayed_reward_survival_probability_4h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_survival_probability_4h
    })
}

pub fn structural_prior_delayed_reward_survival_probability_24h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_survival_probability_24h
    })
}

pub fn structural_prior_delayed_reward_success_hazard_per_hour(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_success_hazard_per_hour
    })
}

pub fn structural_prior_delayed_reward_failure_hazard_per_hour(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_failure_hazard_per_hour
    })
}

pub fn structural_prior_delayed_reward_invalidation_hazard_per_hour(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_invalidation_hazard_per_hour
    })
}

pub fn structural_prior_delayed_reward_abandonment_hazard_per_hour(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_abandonment_hazard_per_hour
    })
}

pub fn structural_prior_delayed_reward_success_cumulative_incidence_4h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_success_cumulative_incidence_4h
    })
}

pub fn structural_prior_delayed_reward_failure_cumulative_incidence_4h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_failure_cumulative_incidence_4h
    })
}

pub fn structural_prior_delayed_reward_invalidation_cumulative_incidence_4h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_invalidation_cumulative_incidence_4h
    })
}

pub fn structural_prior_delayed_reward_abandonment_cumulative_incidence_4h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_abandonment_cumulative_incidence_4h
    })
}

pub fn structural_prior_delayed_reward_resolution_horizon_1h_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    structural_prior_positive_count(prior_stats, |stats| {
        stats.delayed_reward_resolution_horizon_1h_count
    })
}

pub fn structural_prior_delayed_reward_resolution_within_1h_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    structural_prior_positive_count(prior_stats, |stats| {
        stats.delayed_reward_resolution_within_1h_count
    })
}

pub fn structural_prior_delayed_reward_resolution_probability_1h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_resolution_probability_1h
    })
}

pub fn structural_prior_delayed_reward_resolution_horizon_4h_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    structural_prior_positive_count(prior_stats, |stats| {
        stats.delayed_reward_resolution_horizon_4h_count
    })
}

pub fn structural_prior_delayed_reward_resolution_within_4h_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    structural_prior_positive_count(prior_stats, |stats| {
        stats.delayed_reward_resolution_within_4h_count
    })
}

pub fn structural_prior_delayed_reward_resolution_probability_4h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_resolution_probability_4h
    })
}

pub fn structural_prior_delayed_reward_resolution_horizon_24h_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    structural_prior_positive_count(prior_stats, |stats| {
        stats.delayed_reward_resolution_horizon_24h_count
    })
}

pub fn structural_prior_delayed_reward_resolution_within_24h_count(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<usize> {
    structural_prior_positive_count(prior_stats, |stats| {
        stats.delayed_reward_resolution_within_24h_count
    })
}

pub fn structural_prior_delayed_reward_resolution_probability_24h(
    prior_stats: Option<&StructuralPriorStats>,
) -> Option<f64> {
    structural_prior_positive_value(prior_stats, |stats| {
        stats.delayed_reward_resolution_probability_24h
    })
}

pub fn structural_dominant_source_panel(
    prior_stats: Option<&StructuralPriorStats>,
) -> (Option<String>, Option<f64>, Option<f64>) {
    let Some(stats) = prior_stats else {
        return (None, None, None);
    };
    let total_mass: f64 = stats
        .source_panel_summaries
        .values()
        .map(|summary| summary.weighted_followed_mass.max(0.0))
        .sum();
    let dominant = stats
        .source_panel_summaries
        .iter()
        .max_by(|a, b| {
            a.1.weighted_followed_mass
                .partial_cmp(&b.1.weighted_followed_mass)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(b.0))
        })
        .map(|(label, summary)| {
            let share = if total_mass <= f64::EPSILON {
                None
            } else {
                Some((summary.weighted_followed_mass / total_mass).clamp(0.0, 1.0))
            };
            (Some(label.clone()), share, Some(summary.smoothed_prior))
        });
    dominant.unwrap_or((None, None, None))
}

pub fn structural_experience_prior_runtime_metrics(
    prior_stats: Option<&StructuralPriorStats>,
    delayed_reward_replay_validation: Option<StructuralDelayedRewardReplayValidationSurface>,
) -> StructuralExperiencePriorEntry {
    StructuralExperiencePriorEntry {
        source_panel_count: structural_source_panel_count(prior_stats),
        last_offline_seed_source: structural_last_offline_seed_source(prior_stats),
        execution_propensity: structural_prior_execution_propensity(prior_stats),
        ips_weight: structural_prior_ips_weight(prior_stats),
        counterfactual_reward_prior: structural_prior_counterfactual_reward_prior(prior_stats),
        off_policy_adjusted_prior: structural_prior_off_policy_adjusted_prior(prior_stats),
        behavior_policy_probability: structural_prior_behavior_policy_probability(prior_stats),
        behavior_policy_probability_variance: structural_prior_behavior_policy_probability_variance(
            prior_stats,
        ),
        target_policy_probability_confidence: structural_prior_target_policy_probability_confidence(
            prior_stats,
        ),
        target_policy_probability_lower_bound:
            structural_prior_target_policy_probability_lower_bound(prior_stats),
        target_policy_probability_brier_score:
            structural_prior_target_policy_probability_brier_score(prior_stats),
        target_policy_probability_calibration_error:
            structural_prior_target_policy_probability_calibration_error(prior_stats),
        snips_weight_mass: structural_prior_snips_weight_mass(prior_stats),
        snips_weight_squared_mass: structural_prior_snips_weight_squared_mass(prior_stats),
        snips_effective_sample_size: structural_prior_snips_effective_sample_size(prior_stats),
        snips_reward_prior: structural_prior_snips_reward_prior(prior_stats),
        doubly_robust_reward_prior: structural_prior_doubly_robust_reward_prior(prior_stats),
        target_policy_calibration_weight: structural_prior_target_policy_calibration_weight(
            prior_stats,
        ),
        target_policy_reward_prior: structural_prior_target_policy_reward_prior(prior_stats),
        target_policy_variance_penalty: structural_prior_target_policy_variance_penalty(
            prior_stats,
        ),
        target_policy_reward_lower_bound: structural_prior_target_policy_reward_lower_bound(
            prior_stats,
        ),
        matured_feedback_count: structural_prior_matured_feedback_count(prior_stats),
        unresolved_feedback_count: structural_prior_unresolved_feedback_count(prior_stats),
        maturity_coverage: structural_prior_maturity_coverage(prior_stats),
        censoring_rate: structural_prior_censoring_rate(prior_stats),
        delayed_reward_resolution_probability:
            structural_prior_delayed_reward_resolution_probability(prior_stats),
        delayed_reward_censoring_probability: structural_prior_delayed_reward_censoring_probability(
            prior_stats,
        ),
        censoring_adjusted_reward_prior: structural_prior_censoring_adjusted_reward_prior(
            prior_stats,
        ),
        censoring_adjusted_reward_lower_bound:
            structural_prior_censoring_adjusted_reward_lower_bound(prior_stats),
        delayed_reward_success_competing_risk:
            structural_prior_delayed_reward_success_competing_risk(prior_stats),
        delayed_reward_failure_competing_risk:
            structural_prior_delayed_reward_failure_competing_risk(prior_stats),
        delayed_reward_invalidation_competing_risk:
            structural_prior_delayed_reward_invalidation_competing_risk(prior_stats),
        delayed_reward_abandonment_competing_risk:
            structural_prior_delayed_reward_abandonment_competing_risk(prior_stats),
        delayed_reward_competing_risk_entropy:
            structural_prior_delayed_reward_competing_risk_entropy(prior_stats),
        delayed_reward_elapsed_feedback_count:
            structural_prior_delayed_reward_elapsed_feedback_count(prior_stats),
        delayed_reward_elapsed_hours_at_risk: structural_prior_delayed_reward_elapsed_hours_at_risk(
            prior_stats,
        ),
        delayed_reward_avg_elapsed_hours: structural_prior_delayed_reward_avg_elapsed_hours(
            prior_stats,
        ),
        delayed_reward_resolution_hazard_per_hour:
            structural_prior_delayed_reward_resolution_hazard_per_hour(prior_stats),
        delayed_reward_expected_resolution_hours:
            structural_prior_delayed_reward_expected_resolution_hours(prior_stats),
        delayed_reward_survival_probability_1h:
            structural_prior_delayed_reward_survival_probability_1h(prior_stats),
        delayed_reward_survival_probability_4h:
            structural_prior_delayed_reward_survival_probability_4h(prior_stats),
        delayed_reward_survival_probability_24h:
            structural_prior_delayed_reward_survival_probability_24h(prior_stats),
        delayed_reward_success_hazard_per_hour:
            structural_prior_delayed_reward_success_hazard_per_hour(prior_stats),
        delayed_reward_failure_hazard_per_hour:
            structural_prior_delayed_reward_failure_hazard_per_hour(prior_stats),
        delayed_reward_invalidation_hazard_per_hour:
            structural_prior_delayed_reward_invalidation_hazard_per_hour(prior_stats),
        delayed_reward_abandonment_hazard_per_hour:
            structural_prior_delayed_reward_abandonment_hazard_per_hour(prior_stats),
        delayed_reward_success_cumulative_incidence_4h:
            structural_prior_delayed_reward_success_cumulative_incidence_4h(prior_stats),
        delayed_reward_failure_cumulative_incidence_4h:
            structural_prior_delayed_reward_failure_cumulative_incidence_4h(prior_stats),
        delayed_reward_invalidation_cumulative_incidence_4h:
            structural_prior_delayed_reward_invalidation_cumulative_incidence_4h(prior_stats),
        delayed_reward_abandonment_cumulative_incidence_4h:
            structural_prior_delayed_reward_abandonment_cumulative_incidence_4h(prior_stats),
        delayed_reward_resolution_horizon_1h_count:
            structural_prior_delayed_reward_resolution_horizon_1h_count(prior_stats),
        delayed_reward_resolution_within_1h_count:
            structural_prior_delayed_reward_resolution_within_1h_count(prior_stats),
        delayed_reward_resolution_probability_1h:
            structural_prior_delayed_reward_resolution_probability_1h(prior_stats),
        delayed_reward_resolution_horizon_4h_count:
            structural_prior_delayed_reward_resolution_horizon_4h_count(prior_stats),
        delayed_reward_resolution_within_4h_count:
            structural_prior_delayed_reward_resolution_within_4h_count(prior_stats),
        delayed_reward_resolution_probability_4h:
            structural_prior_delayed_reward_resolution_probability_4h(prior_stats),
        delayed_reward_resolution_horizon_24h_count:
            structural_prior_delayed_reward_resolution_horizon_24h_count(prior_stats),
        delayed_reward_resolution_within_24h_count:
            structural_prior_delayed_reward_resolution_within_24h_count(prior_stats),
        delayed_reward_resolution_probability_24h:
            structural_prior_delayed_reward_resolution_probability_24h(prior_stats),
        delayed_reward_replay_validation,
        ..StructuralExperiencePriorEntry::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        structural_delayed_reward_replay_validation,
        STRUCTURAL_DELAYED_REWARD_REPLAY_MIN_TRAIN_RECORDS,
    };
    use crate::state::{FeedbackRecord, ModelProbabilitySnapshot, StructuralFeedbackRefs};
    use crate::types::{Direction, Regime};
    use chrono::{DateTime, Utc};

    fn replay_record(
        recommendation_id: &str,
        recommended_at: &str,
        feedback_at: &str,
        path_id: &str,
        realized_outcome: &str,
    ) -> FeedbackRecord {
        FeedbackRecord {
            timestamp: DateTime::parse_from_rfc3339(feedback_at)
                .unwrap()
                .with_timezone(&Utc),
            symbol: "NQ".to_string(),
            source: "live_feedback".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: Vec::new(),
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: Direction::Bull,
                selected_probability: 0.6,
                long_score: 0.6,
                short_score: 0.4,
                win_prob_long: 0.6,
                win_prob_short: 0.4,
                uncertainty: 0.2,
            },
            realized_outcome: realized_outcome.to_string(),
            pnl: 1.0,
            regime_at_entry: Regime::Accumulation,
            structural_feedback: Some(StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: recommendation_id.to_string(),
                recommended_at: recommended_at.to_string(),
                node_id: "node".to_string(),
                branch_id: "branch".to_string(),
                scenario_id: "scenario".to_string(),
                path_id: path_id.to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            }),
            reflection_mismatch_tags: Vec::new(),
        }
    }

    #[test]
    fn delayed_reward_replay_validation_scores_future_resolution_horizons() {
        let path_id = "path:scenario:NQ:test:primary";
        let records = [
            replay_record(
                "rec-1",
                "2026-04-30T00:00:00Z",
                "2026-04-30T00:30:00Z",
                path_id,
                "win",
            ),
            replay_record(
                "rec-2",
                "2026-04-30T01:00:00Z",
                "2026-04-30T03:00:00Z",
                path_id,
                "loss",
            ),
            replay_record(
                "rec-3",
                "2026-04-30T02:00:00Z",
                "2026-04-30T08:00:00Z",
                path_id,
                "invalidated",
            ),
            replay_record(
                "rec-4",
                "2026-04-30T03:00:00Z",
                "2026-04-30T03:45:00Z",
                path_id,
                "win",
            ),
            replay_record(
                "rec-5",
                "2026-04-30T04:00:00Z",
                "2026-04-30T10:00:00Z",
                path_id,
                "loss",
            ),
        ];
        let refs = records.iter().collect::<Vec<_>>();
        let summary =
            structural_delayed_reward_replay_validation(&refs).expect("replay validation");
        assert_eq!(summary.status, "ready");
        assert_eq!(
            summary.min_training_records,
            STRUCTURAL_DELAYED_REWARD_REPLAY_MIN_TRAIN_RECORDS
        );
        assert_eq!(summary.training_record_count, 3);
        assert_eq!(summary.evaluation_record_count, 2);
        assert_eq!(
            summary.latest_training_recommended_at.as_deref(),
            Some("2026-04-30T03:00:00+00:00")
        );
        assert_eq!(
            summary.first_evaluation_recommended_at.as_deref(),
            Some("2026-04-30T02:00:00+00:00")
        );
        assert_eq!(
            summary.last_evaluation_recommended_at.as_deref(),
            Some("2026-04-30T04:00:00+00:00")
        );
        assert_eq!(summary.resolution_observation_count, 2);
        assert_eq!(summary.resolution_1h_observation_count, 2);
        assert_eq!(summary.resolution_4h_observation_count, 2);
        assert_eq!(summary.resolution_24h_observation_count, 2);
        let overall = summary.resolution_brier_score.unwrap();
        let horizon_1h = summary.resolution_1h_brier_score.unwrap();
        let horizon_4h = summary.resolution_4h_brier_score.unwrap();
        let horizon_24h = summary.resolution_24h_brier_score.unwrap();
        assert!((0.0..=1.0).contains(&overall));
        assert!((0.0..=1.0).contains(&horizon_1h));
        assert!((0.0..=1.0).contains(&horizon_4h));
        assert!((0.0..=1.0).contains(&horizon_24h));
        assert!(horizon_1h > overall);
        assert!(horizon_4h > overall);
        assert!(horizon_24h <= horizon_1h);
        assert!(horizon_24h <= horizon_4h);
    }
}
