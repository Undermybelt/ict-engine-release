use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::belief_core::beta_dirichlet_update::{
    beta_posterior_mean, beta_update_factor, weighted_seed_beta_update,
    weighted_success_credit_beta_update, WeightedSeedBetaUpdateInput,
};
use crate::belief_core::changepoint_gate::rebuild_node_duration_priors_from_events;
use crate::belief_core::regime_filter::rebuild_transition_posteriors_from_events;
use crate::belief_core::source_reliability::{
    accumulate_structural_prior_source_summary_delayed_reward_observation,
    accumulate_structural_prior_stats_delayed_reward_observation,
    refresh_structural_prior_delayed_reward_metrics,
    refresh_structural_source_summary_delayed_reward_metrics,
};
use crate::types::{Direction, FactorIC, Regime, RegimeProbs};

#[cfg(test)]
use crate::belief_core::changepoint_gate::{
    structural_duration_break_hazard, structural_node_bocpd_recursive_run_length_fit,
};

const STRUCTURAL_IPS_WEIGHT_CLIP: f64 = 5.0;
const STRUCTURAL_SOURCE_CONFUSION_LAPLACE_ALPHA: f64 = 1.0;
pub const STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_MULTI_SOURCE_ITEMS: usize = 3;
pub const STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS: usize = 5;
pub const STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_CALIBRATION_OBSERVATIONS: usize = 6;
pub const STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_HOLDOUT_TRAIN_ITEMS: usize = 4;
pub const LEARNING_STATE_FILE: &str = "learning_state.json";
pub const TRADE_HISTORY_FILE: &str = "trade_history.json";
pub const TRAIN_RUNS_FILE: &str = "train_runs.json";
pub const ANALYZE_RUNS_FILE: &str = "analyze_runs.json";
pub const RESEARCH_RUNS_FILE: &str = "research_runs.json";
pub const FACTOR_MUTATION_RUNS_FILE: &str = "factor_mutation_runs.json";
pub const FACTOR_AUTORESEARCH_SESSIONS_FILE: &str = "factor_autoresearch_sessions.json";
pub const FACTOR_AUTORESEARCH_ATTEMPTS_FILE: &str = "factor_autoresearch_attempts.json";
pub const FACTOR_AUTORESEARCH_LIVE_FILE: &str = "factor_autoresearch_live.json";
pub const FACTOR_AUTORESEARCH_FINAL_FILE: &str = "factor_autoresearch_final.json";
pub const FACTOR_AUTORESEARCH_EXPERIMENTS_FILE: &str = "experiments.tsv";
pub const FACTOR_AUTORESEARCH_RETROSPECTIVE_FILE: &str = "factor_autoresearch_retrospective.md";
pub const BACKTEST_RUNS_FILE: &str = "backtest_runs.json";
pub const UPDATE_RUNS_FILE: &str = "update_runs.json";
pub const WORKFLOW_SNAPSHOT_FILE: &str = "workflow_snapshot.json";
pub const COMPACT_SNAPSHOT_FILE: &str = "compact_snapshot.json";
pub const PRE_BAYES_POLICY_HISTORY_FILE: &str = "pre_bayes_policy_history.json";
pub const PENDING_UPDATE_ARTIFACT_FILE: &str = "pending_update_feedback.json";
pub const PENDING_UPDATE_HISTORY_FILE: &str = "pending_update_history.json";
pub const EXECUTION_CANDIDATE_FILE: &str = "execution_candidate.json";
pub const EXECUTION_CANDIDATE_HISTORY_FILE: &str = "execution_candidate_history.json";
pub const ENSEMBLE_VOTE_FILE: &str = "ensemble_vote.json";
pub const ENSEMBLE_VOTE_HISTORY_FILE: &str = "ensemble_vote_history.json";
pub const ENSEMBLE_EXECUTOR_SCORECARDS_FILE: &str = "ensemble_executor_scorecards.json";
pub const ARTIFACT_LEDGER_FILE: &str = "artifact_ledger.json";
pub const BBN_STATE_FILE: &str = "bbn_network.json";

/// State persistence types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub hmm_params: Option<crate::types::HMMParams>,
    pub cascade_config: Option<crate::bayesian::CascadeConfig>,
    pub beta_learner: Option<crate::bayesian::CascadeBetaLearner>,
    pub sv_params: Option<SVParams>,
    pub learning_state: Option<LearningState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SVParams {
    pub mu: f64,
    pub phi: f64,
    pub sigma_eta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LearningState {
    pub factor_profiles: BTreeMap<String, FactorLearningProfile>,
    pub factor_rankings: Vec<PersistedFactorRanking>,
    pub feedback_history: Vec<FeedbackRecord>,
    #[serde(default)]
    pub structural_prior_state: StructuralPriorLearningState,
    pub last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPriorLearningState {
    pub nodes: BTreeMap<String, StructuralPriorStats>,
    pub branches: BTreeMap<String, StructuralPriorStats>,
    pub scenarios: BTreeMap<String, StructuralPriorStats>,
    pub paths: BTreeMap<String, StructuralPriorStats>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub node_prior_mass: BTreeMap<String, StructuralPriorMassSnapshot>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub branch_prior_mass: BTreeMap<String, StructuralPriorMassSnapshot>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub scenario_prior_mass: BTreeMap<String, StructuralPriorMassSnapshot>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub path_prior_mass: BTreeMap<String, StructuralPriorMassSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_offline_seed_snapshot: Option<StructuralOfflineSeedSnapshot>,
    #[serde(default)]
    pub event_ledger: Vec<StructuralPriorEvent>,
    #[serde(default)]
    pub node_duration_priors: BTreeMap<String, StructuralNodeDurationPrior>,
    #[serde(default)]
    pub branch_transition_priors: BTreeMap<String, StructuralBranchTransitionPrior>,
    #[serde(default)]
    pub node_temporal_posteriors: BTreeMap<String, StructuralNodeTemporalPosteriorState>,
    #[serde(default)]
    pub node_transition_posteriors: BTreeMap<String, StructuralNodeTransitionPosteriorState>,
    #[serde(default)]
    pub branch_temporal_posteriors: BTreeMap<String, StructuralBranchTemporalPosteriorState>,
    #[serde(default)]
    pub source_reliability_posteriors: BTreeMap<String, StructuralSourceReliabilityPosterior>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub source_reliability_em_summaries:
        BTreeMap<String, StructuralSourceReliabilityEmSourceSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_reliability_em_calibration: Option<StructuralSourceReliabilityEmCalibrationSummary>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub target_policy_context_posteriors: BTreeMap<String, StructuralTargetPolicyContextPosterior>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralTargetPolicyContextPosterior {
    pub observations: usize,
    #[serde(default)]
    pub weighted_observation_mass: f64,
    #[serde(default)]
    pub success_mass: f64,
    #[serde(default)]
    pub failure_mass: f64,
    #[serde(default)]
    pub behavior_policy_probability: f64,
    #[serde(default)]
    pub behavior_policy_probability_squared_mass: f64,
    #[serde(default)]
    pub behavior_policy_probability_variance: f64,
    #[serde(default)]
    pub learned_target_policy_probability: f64,
    #[serde(default)]
    pub learned_target_policy_probability_lower_bound: f64,
    #[serde(default)]
    pub learned_target_policy_probability_confidence: f64,
    #[serde(default)]
    pub calibrated_target_policy_probability: f64,
    #[serde(default)]
    pub calibrated_target_policy_probability_lower_bound: f64,
    #[serde(default)]
    pub target_policy_probability_brier_score_mass: f64,
    #[serde(default)]
    pub target_policy_probability_calibration_error_mass: f64,
    #[serde(default)]
    pub target_policy_probability_brier_score: f64,
    #[serde(default)]
    pub target_policy_probability_calibration_error: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommendation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPriorStats {
    pub observations: usize,
    pub followed_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub not_followed: usize,
    pub avg_pnl: f64,
    #[serde(default)]
    pub weighted_followed_mass: f64,
    #[serde(default)]
    pub weighted_success_mass: f64,
    #[serde(default)]
    pub weighted_failure_mass: f64,
    #[serde(default)]
    pub weighted_invalidation_mass: f64,
    #[serde(default)]
    pub weighted_exposure_mass: f64,
    #[serde(default)]
    pub weighted_not_followed_mass: f64,
    pub smoothed_prior: f64,
    #[serde(default)]
    pub execution_propensity: f64,
    #[serde(default)]
    pub ips_weight: f64,
    #[serde(default)]
    pub counterfactual_success_mass: f64,
    #[serde(default)]
    pub counterfactual_failure_mass: f64,
    #[serde(default)]
    pub counterfactual_reward_prior: f64,
    #[serde(default)]
    pub off_policy_adjusted_prior: f64,
    #[serde(default)]
    pub policy_weighted_observation_mass: f64,
    #[serde(default)]
    pub behavior_policy_probability: f64,
    #[serde(default)]
    pub behavior_policy_probability_squared_mass: f64,
    #[serde(default)]
    pub behavior_policy_probability_variance: f64,
    #[serde(default)]
    pub target_policy_probability_confidence: f64,
    #[serde(default)]
    pub target_policy_probability_lower_bound: f64,
    #[serde(default)]
    pub target_policy_probability_brier_score_mass: f64,
    #[serde(default)]
    pub target_policy_probability_calibration_error_mass: f64,
    #[serde(default)]
    pub target_policy_probability_brier_score: f64,
    #[serde(default)]
    pub target_policy_probability_calibration_error: f64,
    #[serde(default)]
    pub snips_weight_mass: f64,
    #[serde(default)]
    pub snips_weight_squared_mass: f64,
    #[serde(default)]
    pub snips_effective_sample_size: f64,
    #[serde(default)]
    pub snips_reward_mass: f64,
    #[serde(default)]
    pub snips_reward_prior: f64,
    #[serde(default)]
    pub doubly_robust_reward_mass: f64,
    #[serde(default)]
    pub doubly_robust_reward_prior: f64,
    #[serde(default)]
    pub target_policy_calibration_weight: f64,
    #[serde(default)]
    pub target_policy_reward_prior: f64,
    #[serde(default)]
    pub target_policy_variance_penalty: f64,
    #[serde(default)]
    pub target_policy_reward_lower_bound: f64,
    #[serde(default)]
    pub delayed_reward_resolution_probability: f64,
    #[serde(default)]
    pub delayed_reward_censoring_probability: f64,
    #[serde(default)]
    pub censoring_adjusted_reward_prior: f64,
    #[serde(default)]
    pub censoring_adjusted_reward_lower_bound: f64,
    #[serde(default)]
    pub delayed_reward_success_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_failure_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_competing_risk_entropy: f64,
    #[serde(default)]
    pub delayed_reward_elapsed_feedback_count: usize,
    #[serde(default)]
    pub delayed_reward_elapsed_hours_at_risk: f64,
    #[serde(default)]
    pub delayed_reward_avg_elapsed_hours: f64,
    #[serde(default)]
    pub delayed_reward_resolution_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_expected_resolution_hours: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_1h: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_4h: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_24h: f64,
    #[serde(default)]
    pub delayed_reward_success_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_failure_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_success_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_failure_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_1h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_1h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_1h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_4h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_4h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_4h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_24h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_24h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_24h: f64,
    #[serde(default)]
    pub source_panel_summaries: BTreeMap<String, StructuralPriorSourceSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_offline_seed_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPriorMassSnapshot {
    pub entity_id: String,
    pub entity_kind: String,
    pub observations: usize,
    pub followed_count: usize,
    pub weighted_followed_mass: f64,
    pub weighted_success_mass: f64,
    pub weighted_failure_mass: f64,
    pub weighted_invalidation_mass: f64,
    pub weighted_exposure_mass: f64,
    pub weighted_not_followed_mass: f64,
    pub smoothed_prior: f64,
    pub execution_propensity: f64,
    pub off_policy_adjusted_prior: f64,
    #[serde(default)]
    pub behavior_policy_probability: f64,
    #[serde(default)]
    pub behavior_policy_probability_squared_mass: f64,
    #[serde(default)]
    pub behavior_policy_probability_variance: f64,
    #[serde(default)]
    pub target_policy_probability_confidence: f64,
    #[serde(default)]
    pub target_policy_probability_lower_bound: f64,
    #[serde(default)]
    pub target_policy_probability_brier_score: f64,
    #[serde(default)]
    pub target_policy_probability_calibration_error: f64,
    #[serde(default)]
    pub snips_reward_prior: f64,
    #[serde(default)]
    pub doubly_robust_reward_prior: f64,
    #[serde(default)]
    pub target_policy_calibration_weight: f64,
    #[serde(default)]
    pub target_policy_reward_prior: f64,
    #[serde(default)]
    pub target_policy_variance_penalty: f64,
    #[serde(default)]
    pub target_policy_reward_lower_bound: f64,
    #[serde(default)]
    pub delayed_reward_resolution_probability: f64,
    #[serde(default)]
    pub delayed_reward_censoring_probability: f64,
    #[serde(default)]
    pub censoring_adjusted_reward_prior: f64,
    #[serde(default)]
    pub censoring_adjusted_reward_lower_bound: f64,
    #[serde(default)]
    pub delayed_reward_success_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_failure_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_competing_risk_entropy: f64,
    #[serde(default)]
    pub delayed_reward_elapsed_feedback_count: usize,
    #[serde(default)]
    pub delayed_reward_elapsed_hours_at_risk: f64,
    #[serde(default)]
    pub delayed_reward_avg_elapsed_hours: f64,
    #[serde(default)]
    pub delayed_reward_resolution_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_expected_resolution_hours: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_1h: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_4h: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_24h: f64,
    #[serde(default)]
    pub delayed_reward_success_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_failure_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_success_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_failure_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_1h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_1h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_1h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_4h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_4h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_4h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_24h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_24h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_24h: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_offline_seed_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPriorSeed {
    #[serde(default)]
    pub source_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tempering_coefficient: Option<f64>,
    pub observations: usize,
    pub followed_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub not_followed: usize,
    pub avg_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPriorSourceSummary {
    pub observations: usize,
    pub followed_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub not_followed: usize,
    pub avg_pnl: f64,
    #[serde(default)]
    pub weighted_followed_mass: f64,
    #[serde(default)]
    pub weighted_success_mass: f64,
    #[serde(default)]
    pub weighted_failure_mass: f64,
    #[serde(default)]
    pub weighted_invalidation_mass: f64,
    #[serde(default)]
    pub weighted_exposure_mass: f64,
    #[serde(default)]
    pub weighted_not_followed_mass: f64,
    pub smoothed_prior: f64,
    #[serde(default)]
    pub execution_propensity: f64,
    #[serde(default)]
    pub ips_weight: f64,
    #[serde(default)]
    pub counterfactual_success_mass: f64,
    #[serde(default)]
    pub counterfactual_failure_mass: f64,
    #[serde(default)]
    pub counterfactual_reward_prior: f64,
    #[serde(default)]
    pub off_policy_adjusted_prior: f64,
    #[serde(default)]
    pub policy_weighted_observation_mass: f64,
    #[serde(default)]
    pub behavior_policy_probability: f64,
    #[serde(default)]
    pub behavior_policy_probability_squared_mass: f64,
    #[serde(default)]
    pub behavior_policy_probability_variance: f64,
    #[serde(default)]
    pub target_policy_probability_confidence: f64,
    #[serde(default)]
    pub target_policy_probability_lower_bound: f64,
    #[serde(default)]
    pub target_policy_probability_brier_score_mass: f64,
    #[serde(default)]
    pub target_policy_probability_calibration_error_mass: f64,
    #[serde(default)]
    pub target_policy_probability_brier_score: f64,
    #[serde(default)]
    pub target_policy_probability_calibration_error: f64,
    #[serde(default)]
    pub snips_weight_mass: f64,
    #[serde(default)]
    pub snips_weight_squared_mass: f64,
    #[serde(default)]
    pub snips_effective_sample_size: f64,
    #[serde(default)]
    pub snips_reward_mass: f64,
    #[serde(default)]
    pub snips_reward_prior: f64,
    #[serde(default)]
    pub doubly_robust_reward_mass: f64,
    #[serde(default)]
    pub doubly_robust_reward_prior: f64,
    #[serde(default)]
    pub target_policy_calibration_weight: f64,
    #[serde(default)]
    pub target_policy_reward_prior: f64,
    #[serde(default)]
    pub target_policy_variance_penalty: f64,
    #[serde(default)]
    pub target_policy_reward_lower_bound: f64,
    #[serde(default)]
    pub delayed_reward_resolution_probability: f64,
    #[serde(default)]
    pub delayed_reward_censoring_probability: f64,
    #[serde(default)]
    pub censoring_adjusted_reward_prior: f64,
    #[serde(default)]
    pub censoring_adjusted_reward_lower_bound: f64,
    #[serde(default)]
    pub delayed_reward_success_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_failure_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_competing_risk: f64,
    #[serde(default)]
    pub delayed_reward_competing_risk_entropy: f64,
    #[serde(default)]
    pub delayed_reward_elapsed_feedback_count: usize,
    #[serde(default)]
    pub delayed_reward_elapsed_hours_at_risk: f64,
    #[serde(default)]
    pub delayed_reward_avg_elapsed_hours: f64,
    #[serde(default)]
    pub delayed_reward_resolution_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_expected_resolution_hours: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_1h: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_4h: f64,
    #[serde(default)]
    pub delayed_reward_survival_probability_24h: f64,
    #[serde(default)]
    pub delayed_reward_success_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_failure_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_hazard_per_hour: f64,
    #[serde(default)]
    pub delayed_reward_success_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_failure_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_invalidation_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_abandonment_cumulative_incidence_4h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_1h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_1h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_1h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_4h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_4h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_4h: f64,
    #[serde(default)]
    pub delayed_reward_resolution_horizon_24h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_within_24h_count: usize,
    #[serde(default)]
    pub delayed_reward_resolution_probability_24h: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_tempering_coefficient: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_power_prior_contribution: Option<StructuralPowerPriorContribution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommendation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPowerPriorContribution {
    pub source_label: String,
    pub base_source_weight: f64,
    pub tempering_coefficient: f64,
    pub entity_mass_scale: f64,
    pub effective_tau: f64,
    pub observation_mass: f64,
    pub success_mass: f64,
    pub failure_mass: f64,
    pub invalidation_mass: f64,
    pub not_followed_mass: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralOfflineSeedSnapshot {
    pub source_label: String,
    pub recommendation_id: String,
    pub recommended_at: String,
    pub node_id: String,
    pub branch_id: String,
    pub scenario_id: String,
    pub path_id: String,
    pub followed_path: bool,
    pub observations: usize,
    pub followed_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub not_followed: usize,
    pub avg_pnl: f64,
    pub node_contribution: StructuralPowerPriorContribution,
    pub branch_contribution: StructuralPowerPriorContribution,
    pub scenario_contribution: StructuralPowerPriorContribution,
    pub path_contribution: StructuralPowerPriorContribution,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralSourceReliabilityPosterior {
    pub source_label: String,
    pub observations: usize,
    #[serde(default)]
    pub weighted_observation_mass: f64,
    #[serde(default)]
    pub weighted_success_mass: f64,
    #[serde(default)]
    pub weighted_failure_mass: f64,
    #[serde(default)]
    pub weighted_invalidation_mass: f64,
    #[serde(default)]
    pub weighted_exposure_mass: f64,
    #[serde(default)]
    pub weighted_not_followed_mass: f64,
    #[serde(default)]
    pub posterior_reliability: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_tempering_coefficient: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_power_prior_contribution: Option<StructuralPowerPriorContribution>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub outcome_confusion: BTreeMap<String, StructuralSourceOutcomeConfusionCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralSourceOutcomeConfusionCell {
    pub observed_outcome: String,
    pub credit_class: String,
    pub observations: usize,
    #[serde(default)]
    pub weighted_observation_mass: f64,
    #[serde(default)]
    pub weighted_success_mass: f64,
    #[serde(default)]
    pub weighted_failure_mass: f64,
    #[serde(default)]
    pub credit_class_weighted_observation_mass: f64,
    #[serde(default)]
    pub credit_class_observed_outcome_count: usize,
    #[serde(default)]
    pub observed_given_credit_likelihood: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralSourceReliabilityEmSourceSummary {
    pub source_label: String,
    pub iteration_count: usize,
    pub latent_item_count: usize,
    pub distinct_label_count: usize,
    pub confusion_cell_count: usize,
    #[serde(default)]
    pub posterior_reliability: f64,
    #[serde(default)]
    pub min_diagonal_probability: f64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub confusion: BTreeMap<String, StructuralSourceReliabilityEmConfusionCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralSourceReliabilityEmConfusionCell {
    pub true_credit_class: String,
    pub observed_credit_class: String,
    pub probability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralSourceReliabilityEmCalibrationSummary {
    pub status: String,
    pub observation_count: usize,
    pub source_count: usize,
    pub min_observations: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_loss: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralSourceReliabilityEmHoldoutSummary {
    pub status: String,
    pub split_strategy: String,
    pub training_item_count: usize,
    pub evaluation_item_count: usize,
    pub observation_count: usize,
    pub source_count: usize,
    pub min_training_items: usize,
    pub min_observations: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_loss: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralSourceReliabilityEmReplaySummary {
    pub status: String,
    pub split_strategy: String,
    pub evaluation_item_count: usize,
    pub observation_count: usize,
    pub source_count: usize,
    pub min_training_items: usize,
    pub min_observations: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_loss: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct StructuralSourceReliabilityEmDiagnostics {
    pub candidate_item_count: usize,
    pub labeled_item_count: usize,
    pub multi_source_item_count: usize,
    pub distinct_source_count: usize,
    pub observed_label_count: usize,
    pub max_sources_per_item: usize,
    pub consensus_item_count: usize,
    pub conflict_item_count: usize,
    pub avg_consensus_confidence: Option<f64>,
    pub min_consensus_confidence: Option<f64>,
    pub fit: StructuralSourceReliabilityEmFit,
    pub persisted_source_summary_count: usize,
    pub persisted_confusion_cell_count: usize,
    pub avg_persisted_source_reliability: Option<f64>,
    pub min_persisted_source_reliability: Option<f64>,
    pub calibration: Option<StructuralSourceReliabilityEmCalibrationSummary>,
    pub holdout: Option<StructuralSourceReliabilityEmHoldoutSummary>,
    pub replay: Option<StructuralSourceReliabilityEmReplaySummary>,
}

#[derive(Debug, Clone, Default)]
pub struct StructuralSourceReliabilityEmFit {
    pub iteration_count: usize,
    pub latent_item_count: usize,
    pub distinct_label_count: usize,
    pub confusion_cell_count: usize,
    pub source_reliability: BTreeMap<String, f64>,
    pub avg_latent_confidence: Option<f64>,
    pub min_latent_confidence: Option<f64>,
    pub avg_source_reliability: Option<f64>,
    pub min_source_reliability: Option<f64>,
    pub(crate) confusion: StructuralSourceReliabilityEmConfusion,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPriorEvent {
    #[serde(default)]
    pub source_label: String,
    #[serde(default)]
    pub symbol: String,
    pub recommendation_id: String,
    pub recommended_at: String,
    pub node_id: String,
    pub branch_id: String,
    pub scenario_id: String,
    pub path_id: String,
    pub followed_path: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realized_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralNodeDurationPrior {
    pub observations: usize,
    pub streak_count: usize,
    #[serde(default)]
    pub weighted_streak_mass: f64,
    #[serde(default)]
    pub weighted_success_mass: f64,
    #[serde(default)]
    pub weighted_failure_mass: f64,
    pub total_streak_length: usize,
    pub avg_streak_length: f64,
    pub max_streak_length: usize,
    pub last_streak_length: usize,
    pub persistence_prior: f64,
    #[serde(default)]
    pub expected_dwell_steps: f64,
    #[serde(default)]
    pub remaining_dwell_steps: f64,
    #[serde(default)]
    pub break_hazard: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duration_distribution: Vec<StructuralNodeDurationBucket>,
    #[serde(default)]
    pub duration_distribution_entropy: f64,
    #[serde(default)]
    pub empirical_duration_survival: f64,
    #[serde(default)]
    pub empirical_duration_completion_hazard: f64,
    #[serde(default)]
    pub bocpd_duration_surprise: f64,
    #[serde(default)]
    pub bocpd_evidence_weight: f64,
    #[serde(default)]
    pub bocpd_raw_break_probability: f64,
    #[serde(default)]
    pub bocpd_break_probability: f64,
    #[serde(default)]
    pub bocpd_continue_probability: f64,
    #[serde(default)]
    pub bocpd_run_length_mode: usize,
    #[serde(default)]
    pub bocpd_run_length_mode_probability: f64,
    #[serde(default)]
    pub bocpd_run_length_tail_probability: f64,
    #[serde(default)]
    pub bocpd_run_length_observation_mass: f64,
    #[serde(default)]
    pub bocpd_recursive_reset_probability: f64,
    #[serde(default)]
    pub bocpd_recursive_run_length_mode: usize,
    #[serde(default)]
    pub bocpd_recursive_run_length_mode_probability: f64,
    #[serde(default)]
    pub bocpd_recursive_run_length_expected_value: f64,
    #[serde(default)]
    pub bocpd_recursive_run_length_entropy: f64,
    #[serde(default)]
    pub bocpd_sequence_change_intensity: f64,
    #[serde(default)]
    pub bocpd_sequence_break_probability: f64,
    #[serde(default)]
    pub bocpd_sequence_recursive_reset_probability: f64,
    #[serde(default)]
    pub bocpd_sequence_recursive_run_length_mode: usize,
    #[serde(default)]
    pub bocpd_sequence_recursive_run_length_mode_probability: f64,
    #[serde(default)]
    pub bocpd_sequence_recursive_run_length_expected_value: f64,
    #[serde(default)]
    pub bocpd_sequence_recursive_run_length_entropy: f64,
    #[serde(default)]
    pub sticky_self_transition_strength: f64,
    #[serde(default)]
    pub duration_outcome_support: f64,
    #[serde(default)]
    pub temporal_posterior_support: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralNodeDurationBucket {
    pub dwell_steps: usize,
    pub streak_count: usize,
    #[serde(default)]
    pub weighted_streak_mass: f64,
    #[serde(default)]
    pub probability: f64,
    #[serde(default)]
    pub survival_probability: f64,
    #[serde(default)]
    pub completion_hazard: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBranchTransitionPrior {
    pub from_node_id: String,
    pub to_node_id: String,
    pub from_branch_id: String,
    pub to_branch_id: String,
    pub observations: usize,
    pub weighted_observation_mass: f64,
    pub wins: usize,
    pub losses: usize,
    pub invalidated: usize,
    pub transition_prior: f64,
    #[serde(default)]
    pub transition_outcome_support: f64,
    #[serde(default)]
    pub temporal_posterior_support: f64,
    #[serde(default)]
    pub weighted_success_mass: f64,
    #[serde(default)]
    pub weighted_failure_mass: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralNodeTemporalPosteriorState {
    pub node_id: String,
    pub observations: usize,
    pub streak_count: usize,
    pub weighted_streak_mass: f64,
    #[serde(default)]
    pub expected_dwell_steps: f64,
    #[serde(default)]
    pub remaining_dwell_steps: f64,
    #[serde(default)]
    pub break_hazard: f64,
    #[serde(default)]
    pub sticky_self_transition_strength: f64,
    #[serde(default)]
    pub bocpd_recursive_reset_probability: f64,
    #[serde(default)]
    pub bocpd_recursive_run_length_mode: usize,
    #[serde(default)]
    pub bocpd_recursive_run_length_mode_probability: f64,
    #[serde(default)]
    pub bocpd_recursive_run_length_expected_value: f64,
    #[serde(default)]
    pub bocpd_recursive_run_length_entropy: f64,
    #[serde(default)]
    pub bocpd_sequence_change_intensity: f64,
    #[serde(default)]
    pub bocpd_sequence_break_probability: f64,
    #[serde(default)]
    pub bocpd_sequence_recursive_reset_probability: f64,
    #[serde(default)]
    pub bocpd_sequence_recursive_run_length_mode: usize,
    #[serde(default)]
    pub bocpd_sequence_recursive_run_length_mode_probability: f64,
    #[serde(default)]
    pub bocpd_sequence_recursive_run_length_expected_value: f64,
    #[serde(default)]
    pub bocpd_sequence_recursive_run_length_entropy: f64,
    pub duration_outcome_support: f64,
    pub temporal_posterior_support: f64,
    #[serde(default)]
    pub posterior_blend_weight: f64,
    #[serde(default)]
    pub summary_line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBranchTemporalPosteriorState {
    pub transition_key: String,
    pub from_branch_id: String,
    pub to_branch_id: String,
    pub observations: usize,
    pub weighted_observation_mass: f64,
    #[serde(default)]
    pub transition_prior: f64,
    pub transition_outcome_support: f64,
    pub temporal_posterior_support: f64,
    #[serde(default)]
    pub posterior_multiplier: f64,
    #[serde(default)]
    pub normalized_transition_posterior: f64,
    #[serde(default)]
    pub summary_line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralNodeTransitionPosteriorState {
    pub transition_key: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub observations: usize,
    pub weighted_observation_mass: f64,
    #[serde(default)]
    pub transition_prior: f64,
    #[serde(default)]
    pub weighted_success_mass: f64,
    #[serde(default)]
    pub weighted_failure_mass: f64,
    pub transition_outcome_support: f64,
    pub temporal_posterior_support: f64,
    #[serde(default)]
    pub posterior_multiplier: f64,
    #[serde(default)]
    pub normalized_transition_posterior: f64,
    #[serde(default)]
    pub summary_line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorLearningProfile {
    pub enabled: bool,
    pub base_weight: f64,
    pub posterior_reliability: f64,
    pub last_ic: f64,
    pub last_ir: f64,
    pub last_backtest_return: f64,
    pub last_stability: f64,
    pub parameters: BTreeMap<String, f64>,
    pub regime_stats: BTreeMap<String, RegimeFactorStats>,
}

impl Default for FactorLearningProfile {
    fn default() -> Self {
        Self {
            enabled: true,
            base_weight: 0.2,
            posterior_reliability: 0.5,
            last_ic: 0.0,
            last_ir: 0.0,
            last_backtest_return: 0.0,
            last_stability: 0.0,
            parameters: BTreeMap::new(),
            regime_stats: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeFactorStats {
    pub observations: usize,
    pub wins: usize,
    #[serde(default)]
    pub weighted_observations: f64,
    #[serde(default)]
    pub weighted_successes: f64,
    pub avg_pnl: f64,
    pub multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub source: String,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub trade_id: Option<String>,
    #[serde(default)]
    pub prompt_version: Option<String>,
    #[serde(default)]
    pub factor_version: Option<String>,
    #[serde(default)]
    pub data_fingerprint: Option<String>,
    pub factors_used: Vec<FeedbackFactorUsage>,
    pub model_probabilities_before_trade: ModelProbabilitySnapshot,
    pub realized_outcome: String,
    pub pnl: f64,
    pub regime_at_entry: Regime,
    #[serde(default)]
    pub structural_feedback: Option<StructuralFeedbackRefs>,
    #[serde(default)]
    pub reflection_mismatch_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralFeedbackRefs {
    pub protocol_version: String,
    pub recommendation_id: String,
    pub recommended_at: String,
    pub node_id: String,
    pub branch_id: String,
    pub scenario_id: String,
    pub path_id: String,
    pub followed_path: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

pub fn structural_feedback_outcome_is_unresolved(outcome: &str) -> bool {
    matches!(
        outcome.trim().to_ascii_lowercase().as_str(),
        "pending" | "delayed" | "unresolved" | "awaiting"
    )
}

pub fn structural_feedback_counts_as_executed_trade(record: &FeedbackRecord) -> bool {
    if structural_feedback_outcome_is_unresolved(&record.realized_outcome) {
        return false;
    }
    if record
        .realized_outcome
        .trim()
        .eq_ignore_ascii_case("not_followed")
    {
        return false;
    }
    record
        .structural_feedback
        .as_ref()
        .map(|refs| refs.followed_path)
        .unwrap_or(true)
}

pub fn structural_feedback_trade_outcome_proxy(record: &FeedbackRecord) -> Option<String> {
    use StructuralFeedbackLearningOutcome::{Negative, Neutral, Positive};

    structural_feedback_learning_outcome(record).map(|outcome| match outcome {
        Positive => "win".to_string(),
        Negative => "loss".to_string(),
        Neutral => "breakeven".to_string(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralFeedbackLearningOutcome {
    Positive,
    Neutral,
    Negative,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralFeedbackLearningSemantics {
    pub credit_class: String,
    pub success_credit: f64,
    pub observation_weight: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StructuralFeedbackPseudoCounts {
    pub(crate) success_credit: f64,
    pub(crate) observation_weight: f64,
}

pub fn structural_feedback_learning_outcome(
    record: &FeedbackRecord,
) -> Option<StructuralFeedbackLearningOutcome> {
    if !structural_feedback_counts_as_executed_trade(record) {
        return None;
    }
    match record.realized_outcome.trim().to_ascii_lowercase().as_str() {
        "win" | "profit" | "tp" | "take_profit" => {
            Some(StructuralFeedbackLearningOutcome::Positive)
        }
        "loss" | "lose" | "sl" | "stop" | "stop_loss" | "invalidated" => {
            Some(StructuralFeedbackLearningOutcome::Negative)
        }
        "breakeven" | "abandoned" => Some(StructuralFeedbackLearningOutcome::Neutral),
        _ if record.pnl > 1e-12 => Some(StructuralFeedbackLearningOutcome::Positive),
        _ if record.pnl < -1e-12 => Some(StructuralFeedbackLearningOutcome::Negative),
        _ => Some(StructuralFeedbackLearningOutcome::Neutral),
    }
}

pub fn structural_feedback_learning_semantics(
    record: &FeedbackRecord,
) -> StructuralFeedbackLearningSemantics {
    if structural_feedback_outcome_is_unresolved(&record.realized_outcome) {
        return StructuralFeedbackLearningSemantics {
            credit_class: "no_credit_unresolved".to_string(),
            success_credit: 0.0,
            observation_weight: 0.0,
        };
    }
    if !structural_feedback_counts_as_executed_trade(record) {
        return StructuralFeedbackLearningSemantics {
            credit_class: "no_credit_not_followed".to_string(),
            success_credit: 0.0,
            observation_weight: 0.0,
        };
    }
    match record.realized_outcome.trim().to_ascii_lowercase().as_str() {
        "win" | "profit" | "tp" | "take_profit" => StructuralFeedbackLearningSemantics {
            credit_class: "positive_executed".to_string(),
            success_credit: 1.0,
            observation_weight: 1.0,
        },
        "loss" | "lose" | "sl" | "stop" | "stop_loss" => StructuralFeedbackLearningSemantics {
            credit_class: "negative_executed".to_string(),
            success_credit: 0.0,
            observation_weight: 1.0,
        },
        "invalidated" => StructuralFeedbackLearningSemantics {
            credit_class: "negative_invalidated".to_string(),
            success_credit: 0.0,
            observation_weight: 1.25,
        },
        "abandoned" => StructuralFeedbackLearningSemantics {
            credit_class: "fractional_abandoned".to_string(),
            success_credit: 0.25,
            observation_weight: 0.75,
        },
        other => match structural_feedback_learning_outcome(record) {
            Some(StructuralFeedbackLearningOutcome::Positive) => {
                StructuralFeedbackLearningSemantics {
                    credit_class: format!("positive_proxy:{other}"),
                    success_credit: 1.0,
                    observation_weight: 1.0,
                }
            }
            Some(StructuralFeedbackLearningOutcome::Negative) => {
                StructuralFeedbackLearningSemantics {
                    credit_class: format!("negative_proxy:{other}"),
                    success_credit: 0.0,
                    observation_weight: 1.0,
                }
            }
            _ => StructuralFeedbackLearningSemantics {
                credit_class: "fractional_breakeven".to_string(),
                success_credit: 0.5,
                observation_weight: 1.0,
            },
        },
    }
}

fn structural_feedback_pseudo_counts(
    record: &FeedbackRecord,
    followed_path: bool,
) -> Option<StructuralFeedbackPseudoCounts> {
    if !followed_path || structural_feedback_outcome_is_unresolved(&record.realized_outcome) {
        return None;
    }
    let semantics = structural_feedback_learning_semantics(record);
    if semantics.observation_weight <= f64::EPSILON {
        None
    } else {
        Some(StructuralFeedbackPseudoCounts {
            success_credit: semantics.success_credit.clamp(0.0, 1.0),
            observation_weight: semantics.observation_weight.max(0.0),
        })
    }
}

pub(crate) fn structural_event_outcome_pseudo_counts(
    outcome: Option<&str>,
) -> Option<StructuralFeedbackPseudoCounts> {
    match outcome.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if matches!(value.as_str(), "win" | "profit" | "tp" | "take_profit") => {
            Some(StructuralFeedbackPseudoCounts {
                success_credit: 1.0,
                observation_weight: 1.0,
            })
        }
        Some(value)
            if matches!(
                value.as_str(),
                "loss" | "lose" | "sl" | "stop" | "stop_loss"
            ) =>
        {
            Some(StructuralFeedbackPseudoCounts {
                success_credit: 0.0,
                observation_weight: 1.0,
            })
        }
        Some(value) if value == "invalidated" => Some(StructuralFeedbackPseudoCounts {
            success_credit: 0.0,
            observation_weight: 1.25,
        }),
        Some(value) if value == "abandoned" => Some(StructuralFeedbackPseudoCounts {
            success_credit: 0.25,
            observation_weight: 0.75,
        }),
        Some(value) if value == "breakeven" => Some(StructuralFeedbackPseudoCounts {
            success_credit: 0.5,
            observation_weight: 1.0,
        }),
        _ => None,
    }
}

pub fn structural_feedback_counter_outcome(record: &FeedbackRecord) -> Option<&'static str> {
    match record.realized_outcome.trim().to_ascii_lowercase().as_str() {
        "win" | "profit" | "tp" | "take_profit" => Some("win"),
        "loss" | "lose" | "sl" | "stop" | "stop_loss" => Some("loss"),
        "invalidated" => Some("invalidated"),
        "abandoned" => Some("abandoned"),
        "breakeven" => Some("breakeven"),
        "not_followed" => Some("not_followed"),
        _ if record.pnl > 1e-12 => Some("win"),
        _ if record.pnl < -1e-12 => Some("loss"),
        _ => None,
    }
}

pub fn structural_learning_semantics_summary(
    credit_class: Option<&str>,
    success_credit: Option<f64>,
    observation_weight: Option<f64>,
) -> String {
    format!(
        "class={} success_credit={:.3} observation_weight={:.3}",
        credit_class.unwrap_or("unavailable"),
        success_credit.unwrap_or_default(),
        observation_weight.unwrap_or_default()
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdateArtifactDiff {
    pub previous_artifact_id: Option<String>,
    pub changed_fields: Vec<String>,
    pub quality_delta: i32,
    pub exact_duplicate: bool,
    pub comparable_same_data: bool,
    pub comparable_same_factor_version: bool,
    pub comparable_same_prompt_version: bool,
    pub selected_probability_delta: f64,
    pub top_factor_score_delta: f64,
    pub avg_family_score_delta: f64,
}

impl Default for PendingUpdateArtifactDiff {
    fn default() -> Self {
        Self {
            previous_artifact_id: None,
            changed_fields: Vec::new(),
            quality_delta: 0,
            exact_duplicate: false,
            comparable_same_data: false,
            comparable_same_factor_version: false,
            comparable_same_prompt_version: false,
            selected_probability_delta: 0.0,
            top_factor_score_delta: 0.0,
            avg_family_score_delta: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PendingUpdateArtifactDecision {
    pub status: String,
    pub reason: String,
    pub supersedes_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdateArtifact {
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub source_command: String,
    pub provenance: RunProvenance,
    pub decision_hint: String,
    pub entry_quality: String,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub selected_win_probability: f64,
    pub top_factor_score: f64,
    pub avg_family_score: f64,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub family_scores: BTreeMap<String, f64>,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub review_rule_snapshot: PendingUpdateReviewRules,
    #[serde(default)]
    pub pre_bayes_evidence_filter: Option<PreBayesEvidenceFilter>,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    pub template_feedback: FeedbackRecord,
    #[serde(default)]
    pub diff_from_previous: PendingUpdateArtifactDiff,
    #[serde(default)]
    pub review_decision: PendingUpdateArtifactDecision,
}

impl Default for PendingUpdateArtifact {
    fn default() -> Self {
        Self {
            artifact_id: String::new(),
            version: 0,
            generated_at: Utc::now(),
            symbol: String::new(),
            source_phase: String::new(),
            source_run_id: None,
            source_command: String::new(),
            provenance: RunProvenance::default(),
            decision_hint: String::new(),
            entry_quality: String::new(),
            factor_alignment: String::new(),
            factor_uncertainty: String::new(),
            selected_win_probability: 0.0,
            top_factor_score: 0.0,
            avg_family_score: 0.0,
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::new(),
            review_rule_version: String::new(),
            review_rule_snapshot: PendingUpdateReviewRules::default(),
            pre_bayes_evidence_filter: None,
            pre_bayes_entry_quality_bridge: None,
            multi_timeframe_summary: Vec::new(),
            template_feedback: FeedbackRecord {
                timestamp: Utc::now(),
                symbol: String::new(),
                source: String::new(),
                run_id: None,
                trade_id: None,
                prompt_version: None,
                factor_version: None,
                data_fingerprint: None,
                factors_used: Vec::new(),
                model_probabilities_before_trade: ModelProbabilitySnapshot {
                    selected_direction: Direction::Neutral,
                    selected_probability: 0.0,
                    long_score: 0.0,
                    short_score: 0.0,
                    win_prob_long: 0.0,
                    win_prob_short: 0.0,
                    uncertainty: 0.0,
                },
                realized_outcome: String::new(),
                pnl: 0.0,
                regime_at_entry: Regime::ManipulationExpansion,
                structural_feedback: None,
                reflection_mismatch_tags: Vec::new(),
            },
            diff_from_previous: PendingUpdateArtifactDiff::default(),
            review_decision: PendingUpdateArtifactDecision::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PendingUpdateArtifactSummary {
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub path: String,
    pub decision_hint: String,
    pub entry_quality: String,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub review_status: String,
    #[serde(default)]
    pub review_reason: String,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: String,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub quality_delta: i32,
    #[serde(default)]
    pub selected_probability_delta: f64,
    #[serde(default)]
    pub top_factor_score_delta: f64,
    #[serde(default)]
    pub avg_family_score_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCandidateArtifactDiff {
    pub previous_artifact_id: Option<String>,
    pub changed_fields: Vec<String>,
    pub posterior_delta: f64,
    pub win_probability_delta: f64,
    pub entry_delta: f64,
    pub exact_duplicate: bool,
}

impl Default for ExecutionCandidateArtifactDiff {
    fn default() -> Self {
        Self {
            previous_artifact_id: None,
            changed_fields: Vec::new(),
            posterior_delta: 0.0,
            win_probability_delta: 0.0,
            entry_delta: 0.0,
            exact_duplicate: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionCandidateArtifactDecision {
    pub status: String,
    pub reason: String,
    pub supersedes_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCandidateArtifact {
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub provenance: RunProvenance,
    pub decision_hint: String,
    pub selected_direction: Direction,
    pub trade_direction: Direction,
    pub actionable: bool,
    pub entry: f64,
    pub stop_loss: f64,
    pub take_profits: Vec<f64>,
    pub posterior: f64,
    pub win_probability: f64,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub candidate_status: String,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub family_scores: BTreeMap<String, f64>,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub review_rule_snapshot: ExecutionCandidateReviewRules,
    #[serde(default)]
    pub pre_bayes_evidence_filter: Option<PreBayesEvidenceFilter>,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub executor_scorecards: Vec<EnsembleExecutorScorecard>,
    #[serde(default)]
    pub diff_from_previous: ExecutionCandidateArtifactDiff,
    #[serde(default)]
    pub review_decision: ExecutionCandidateArtifactDecision,
}

impl Default for ExecutionCandidateArtifact {
    fn default() -> Self {
        Self {
            artifact_id: String::new(),
            version: 0,
            generated_at: Utc::now(),
            symbol: String::new(),
            source_phase: String::new(),
            source_run_id: None,
            provenance: RunProvenance::default(),
            decision_hint: String::new(),
            selected_direction: Direction::Neutral,
            trade_direction: Direction::Neutral,
            actionable: false,
            entry: 0.0,
            stop_loss: 0.0,
            take_profits: Vec::new(),
            posterior: 0.0,
            win_probability: 0.0,
            factor_alignment: String::new(),
            factor_uncertainty: String::new(),
            candidate_status: String::new(),
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::new(),
            review_rule_version: String::new(),
            review_rule_snapshot: ExecutionCandidateReviewRules::default(),
            pre_bayes_evidence_filter: None,
            pre_bayes_entry_quality_bridge: None,
            multi_timeframe_summary: Vec::new(),
            executor_scorecards: Vec::new(),
            diff_from_previous: ExecutionCandidateArtifactDiff::default(),
            review_decision: ExecutionCandidateArtifactDecision::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionCandidateArtifactSummary {
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub path: String,
    pub trade_direction: String,
    pub actionable: bool,
    pub candidate_status: String,
    pub decision_hint: String,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub review_status: String,
    #[serde(default)]
    pub review_reason: String,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: String,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub posterior_delta: f64,
    #[serde(default)]
    pub win_probability_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactLedgerEntry {
    pub entry_id: String,
    pub artifact_kind: String,
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub path: String,
    pub status: String,
    pub promote_candidate: bool,
    pub actionable: bool,
    pub decision_hint: String,
    pub review_reason: String,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub family_scores: BTreeMap<String, f64>,
    #[serde(default)]
    pub supersedes_artifact_id: Option<String>,
    #[serde(default)]
    pub quality_score: i32,
    pub consumed_by_update_run_id: Option<String>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub consumed_outcome: Option<String>,
    #[serde(default)]
    pub regraded_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub consumption_regrade_status: Option<String>,
    #[serde(default)]
    pub consumption_regrade_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdateReviewRules {
    pub min_probability_improvement: f64,
    pub min_top_factor_score_improvement: f64,
    pub min_avg_family_score_improvement: f64,
    pub require_same_data: bool,
    pub require_same_factor_version: bool,
    pub require_same_prompt_version: bool,
}

impl Default for PendingUpdateReviewRules {
    fn default() -> Self {
        Self {
            min_probability_improvement: 0.03,
            min_top_factor_score_improvement: 0.05,
            min_avg_family_score_improvement: 0.03,
            require_same_data: true,
            require_same_factor_version: true,
            require_same_prompt_version: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactReviewRuleSources {
    pub pending_update: BTreeMap<String, String>,
    pub execution_candidate: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCandidateReviewRules {
    pub min_posterior_improvement: f64,
    pub min_win_probability_improvement: f64,
    pub require_same_data: bool,
    pub require_same_factor_version: bool,
}

impl Default for ExecutionCandidateReviewRules {
    fn default() -> Self {
        Self {
            min_posterior_improvement: 0.03,
            min_win_probability_improvement: 0.03,
            require_same_data: true,
            require_same_factor_version: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactReviewRules {
    pub pending_update: PendingUpdateReviewRules,
    pub execution_candidate: ExecutionCandidateReviewRules,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnsembleExecutorScorecard {
    pub executor: String,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub validated_positive: usize,
    pub validated_negative: usize,
    pub cumulative_quality_score: i64,
    pub latest_weight_hint: Option<f64>,
    pub last_outcome: Option<String>,
    pub last_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnsembleVoteRecord {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub provenance: RunProvenance,
    pub dataset_comparability: DatasetComparability,
    pub ensemble_version: String,
    pub final_action: String,
    pub recommended_command: String,
    pub human_next_triage: String,
    #[serde(default)]
    pub hard_block: crate::application::orchestration::EnsembleHardBlockArtifact,
    pub confidence: f64,
    pub consensus_strength: f64,
    pub disagreement_flags: Vec<String>,
    pub executor_summaries: Vec<String>,
    pub split_explanations: Vec<String>,
    #[serde(default)]
    // DEPRECATED compatibility mirror. Canonical executor scorecards live in
    // ensemble_executor_scorecards.json and should be read through canonical loaders.
    pub executor_scorecards: Vec<EnsembleExecutorScorecard>,
    #[serde(default)]
    // Source of the scorecard surface used for this record: persisted or fallback.
    pub executor_scorecards_source: Option<String>,
    pub posterior_fingerprint: String,
    pub posterior_normalization_status: String,
    pub posterior_active_regime: String,
    pub posterior_confidence: Option<f64>,
    pub posterior_probabilities: BTreeMap<String, f64>,
    pub posterior_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactHistorySummary {
    pub total_entries: usize,
    pub pending_update_entries: usize,
    pub execution_candidate_entries: usize,
    #[serde(default)]
    pub ensemble_vote_entries: usize,
    pub promotable_entries: usize,
    pub actionable_entries: usize,
    pub consumed_entries: usize,
    pub average_quality_score: f64,
    pub latest_consumed_artifact_id: Option<String>,
    pub statuses_by_kind: BTreeMap<String, BTreeMap<String, usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactLineageSummary {
    pub artifact_kind: String,
    pub root_artifact_id: String,
    pub latest_artifact_id: String,
    pub artifact_count: usize,
    pub quality_delta: i32,
    pub consumed_count: usize,
    pub conclusion: String,
    #[serde(default)]
    pub distinct_review_rule_versions: Vec<String>,
    #[serde(default)]
    pub review_rule_break_count: usize,
    #[serde(default)]
    pub embedded_pre_bayes_change_count: usize,
    #[serde(default)]
    pub latest_pre_bayes_gate_status: String,
    #[serde(default)]
    pub latest_pre_bayes_bridge_selected_entry_quality: String,
    #[serde(default)]
    pub latest_pre_bayes_multi_timeframe_direction_bias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactRuleBreakFactorImpact {
    pub factor_name: String,
    pub break_count: usize,
    pub cumulative_quality_delta: i32,
    pub improving_breaks: usize,
    pub deteriorating_breaks: usize,
    #[serde(default)]
    pub consumed_breaks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactRuleBreakFamilyImpact {
    pub family: String,
    pub break_count: usize,
    pub cumulative_quality_delta: i32,
    pub improving_breaks: usize,
    pub deteriorating_breaks: usize,
    #[serde(default)]
    pub consumed_breaks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConsumedImpactPoint {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub consumed_at: Option<DateTime<Utc>>,
    pub consumed_outcome: Option<String>,
    pub quality_score: i32,
    pub regrade_status: Option<String>,
    pub quality_delta_from_previous_consumed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConsumedImpactWindow {
    pub label: String,
    pub count: usize,
    pub positive: usize,
    pub negative: usize,
    pub neutral: usize,
    pub average_quality_score: f64,
    pub cumulative_quality_delta: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConsumedImpactTrendComparison {
    pub label: String,
    pub recent: ArtifactConsumedImpactWindow,
    pub baseline: ArtifactConsumedImpactWindow,
    pub average_quality_score_delta: f64,
    pub cumulative_quality_delta_delta: i32,
    pub positive_rate_delta: f64,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConsumedImpactSummary {
    pub total_consumed: usize,
    pub positive_consumed: usize,
    pub negative_consumed: usize,
    pub neutral_consumed: usize,
    pub cumulative_quality_score: i32,
    pub points: Vec<ArtifactConsumedImpactPoint>,
    #[serde(default)]
    pub by_kind: BTreeMap<String, ArtifactConsumedImpactWindow>,
    #[serde(default)]
    pub recent_windows: Vec<ArtifactConsumedImpactWindow>,
    #[serde(default)]
    pub trend_comparisons: Vec<ArtifactConsumedImpactTrendComparison>,
    #[serde(default)]
    pub by_kind_trend_comparisons: BTreeMap<String, Vec<ArtifactConsumedImpactTrendComparison>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactRuleBreakEffect {
    pub artifact_kind: String,
    pub lineage_root_artifact_id: String,
    pub from_artifact_id: String,
    pub to_artifact_id: String,
    pub from_rule_version: String,
    pub to_rule_version: String,
    pub quality_delta: i32,
    pub consumed_delta: i32,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactDecisionSummary {
    pub actionable_artifact_count: usize,
    pub latest_promotable_artifact_id: Option<String>,
    pub artifact_rule_break_count: usize,
    pub highlighted_actions: Vec<String>,
    #[serde(default)]
    pub highlighted_factor_targets: Vec<String>,
    #[serde(default)]
    pub highlighted_family_targets: Vec<String>,
    #[serde(default)]
    pub promotion_strength: String,
    #[serde(default)]
    pub rollback_strength: String,
    #[serde(default)]
    pub consumed_trend_status: String,
    #[serde(default)]
    pub consumed_trend_reason: String,
    #[serde(default)]
    pub consumed_target_kinds: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactDecisionSection {
    pub summary: ArtifactDecisionSummary,
    pub action_summary: Vec<String>,
    pub top_factor_trends: Vec<ArtifactFactorTrendSummary>,
    pub top_family_trends: Vec<ArtifactFamilyTrendSummary>,
    pub top_rule_break_effects: Vec<ArtifactRuleBreakEffect>,
    #[serde(default)]
    pub top_consumed_trend_comparisons: Vec<ArtifactConsumedImpactTrendComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFactorTrendSummary {
    pub factor_name: String,
    pub entries: usize,
    pub promotable_entries: usize,
    pub consumed_entries: usize,
    pub average_quality_score: f64,
    pub latest_status: Option<String>,
    pub latest_action: Option<String>,
    #[serde(default)]
    pub decision_status: String,
    #[serde(default)]
    pub decision_reason: String,
    #[serde(default)]
    pub promotion_link_status: String,
    #[serde(default)]
    pub rollback_link_status: String,
    #[serde(default)]
    pub consumed_validation_status: String,
    #[serde(default)]
    pub consumed_validation_reason: String,
    #[serde(default)]
    pub consumed_validation_rank: i32,
    #[serde(default)]
    pub consumed_validation_score: f64,
}

impl Default for ArtifactFactorTrendSummary {
    fn default() -> Self {
        Self {
            factor_name: String::new(),
            entries: 0,
            promotable_entries: 0,
            consumed_entries: 0,
            average_quality_score: 0.0,
            latest_status: None,
            latest_action: None,
            decision_status: String::new(),
            decision_reason: String::new(),
            promotion_link_status: String::new(),
            rollback_link_status: String::new(),
            consumed_validation_status: String::new(),
            consumed_validation_reason: String::new(),
            consumed_validation_rank: 0,
            consumed_validation_score: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFamilyTrendSummary {
    pub family: String,
    pub entries: usize,
    pub promotable_entries: usize,
    pub consumed_entries: usize,
    pub average_quality_score: f64,
    pub latest_status: Option<String>,
    pub latest_score: Option<f64>,
    #[serde(default)]
    pub decision_status: String,
    #[serde(default)]
    pub decision_reason: String,
    #[serde(default)]
    pub promotion_link_status: String,
    #[serde(default)]
    pub rollback_link_status: String,
    #[serde(default)]
    pub consumed_validation_status: String,
    #[serde(default)]
    pub consumed_validation_reason: String,
    #[serde(default)]
    pub consumed_validation_rank: i32,
    #[serde(default)]
    pub consumed_validation_score: f64,
}

impl Default for ArtifactFamilyTrendSummary {
    fn default() -> Self {
        Self {
            family: String::new(),
            entries: 0,
            promotable_entries: 0,
            consumed_entries: 0,
            average_quality_score: 0.0,
            latest_status: None,
            latest_score: None,
            decision_status: String::new(),
            decision_reason: String::new(),
            promotion_link_status: String::new(),
            rollback_link_status: String::new(),
            consumed_validation_status: String::new(),
            consumed_validation_reason: String::new(),
            consumed_validation_rank: 0,
            consumed_validation_score: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackFactorUsage {
    pub factor_name: String,
    pub category: String,
    pub direction: Direction,
    pub value: f64,
    pub confidence: f64,
    pub weight: f64,
    pub long_support: f64,
    pub short_support: f64,
    pub uncertainty_contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProbabilitySnapshot {
    pub selected_direction: Direction,
    pub selected_probability: f64,
    pub long_score: f64,
    pub short_score: f64,
    pub win_prob_long: f64,
    pub win_prob_short: f64,
    pub uncertainty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeedbackHistorySummary {
    pub total_records: usize,
    pub wins: usize,
    pub losses: usize,
    pub avg_pnl: f64,
    pub factor_success_rates: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionThresholds {
    pub promotion_min_score_delta: f64,
    pub promotion_min_score: f64,
    pub rollback_score_delta: f64,
    pub rollback_probability_delta: f64,
    #[serde(default)]
    pub artifact_consumed_min_window: usize,
    #[serde(default)]
    pub artifact_consumed_improvement_quality_delta: f64,
    #[serde(default)]
    pub artifact_consumed_improvement_positive_rate_delta: f64,
    #[serde(default)]
    pub artifact_consumed_regression_quality_delta: f64,
    #[serde(default)]
    pub artifact_consumed_regression_positive_rate_delta: f64,
}

impl Default for DecisionThresholds {
    fn default() -> Self {
        Self {
            promotion_min_score_delta: 0.10,
            promotion_min_score: 0.70,
            rollback_score_delta: -0.10,
            rollback_probability_delta: 0.10,
            artifact_consumed_min_window: 3,
            artifact_consumed_improvement_quality_delta: 5.0,
            artifact_consumed_improvement_positive_rate_delta: 0.25,
            artifact_consumed_regression_quality_delta: -5.0,
            artifact_consumed_regression_positive_rate_delta: -0.25,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEvidencePolicy {
    pub version: String,
    pub source: String,
    pub min_directional_support_gap: f64,
    pub high_uncertainty_threshold: f64,
    pub min_multi_timeframe_alignment_score: f64,
    pub min_multi_timeframe_entry_alignment_score: f64,
    pub hard_pass_quality_threshold: f64,
    pub neutralized_quality_threshold: f64,
    pub directional_conflict_penalty: f64,
    pub mixed_alignment_penalty: f64,
    pub multi_timeframe_direction_conflict_penalty: f64,
    pub multi_timeframe_alignment_penalty: f64,
    pub multi_timeframe_entry_penalty: f64,
    pub multi_timeframe_alignment_bonus: f64,
    pub hostile_liquidity_penalty: f64,
    pub favorable_liquidity_bonus: f64,
    pub hostile_liquidity_forces_high_uncertainty: bool,
    #[serde(default)]
    pub market_overrides: BTreeMap<String, PreBayesMarketPolicyOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesMarketPolicyOverride {
    #[serde(default)]
    pub hostile_liquidity_penalty: Option<f64>,
    #[serde(default)]
    pub favorable_liquidity_bonus: Option<f64>,
    #[serde(default)]
    pub hostile_liquidity_forces_high_uncertainty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEntryQualityBridge {
    pub long_signal_probability: f64,
    pub short_signal_probability: f64,
    pub long_entry_bias: Vec<f64>,
    pub short_entry_bias: Vec<f64>,
    pub long_entry_quality: BTreeMap<String, f64>,
    pub short_entry_quality: BTreeMap<String, f64>,
    pub selected_entry_quality: BTreeMap<String, f64>,
    #[serde(default)]
    pub multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesPolicyDiff {
    pub previous_version: Option<String>,
    pub changed_fields: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesPolicyRecord {
    pub timestamp: DateTime<Utc>,
    pub run_id: String,
    pub source_command: String,
    pub policy: PreBayesEvidencePolicy,
    pub diff_from_previous: PreBayesPolicyDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesPolicyLineageSummary {
    pub latest_version: Option<String>,
    pub previous_version: Option<String>,
    pub total_versions: usize,
    pub changed_fields_union: Vec<String>,
    pub rollback_candidate_version: Option<String>,
    pub rollback_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesSoftEvidenceNodeDiff {
    pub node: String,
    pub filtered_state: String,
    pub dominant_soft_state: Option<String>,
    pub dominant_soft_probability: f64,
    pub entropy: f64,
    pub diverges_from_filtered_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEntryQualityBridgeDiff {
    pub dominant_long_entry_quality: Option<String>,
    pub dominant_long_entry_quality_probability: f64,
    pub dominant_short_entry_quality: Option<String>,
    pub dominant_short_entry_quality_probability: f64,
    pub selected_entry_quality: Option<String>,
    pub selected_entry_quality_probability: f64,
    pub long_short_signal_probability_gap: f64,
    #[serde(default)]
    pub multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub rationale_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorPipelineLabelSource {
    pub label: String,
    pub derivation: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEvidencePacket {
    pub filter: PreBayesEvidenceFilter,
    #[serde(default)]
    pub evidence_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub timed_pda_summary: crate::bbn::ICTStructureSummary,
    pub raw_market_regime_trace: FactorPipelineLabelSource,
    pub raw_liquidity_context_trace: FactorPipelineLabelSource,
    pub raw_multi_timeframe_resonance_trace: FactorPipelineLabelSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEvidenceFilter {
    #[serde(default)]
    pub policy: PreBayesEvidencePolicy,
    #[serde(default)]
    pub entry_logic_id: Option<String>,
    #[serde(default)]
    pub logic_family: Option<String>,
    pub raw_market_regime_label: String,
    pub raw_liquidity_context_label: String,
    pub raw_factor_alignment: String,
    pub raw_factor_uncertainty: String,
    #[serde(default)]
    pub raw_multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub raw_multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub raw_multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub raw_multi_timeframe_resonance_label: String,
    #[serde(default)]
    pub active_pda_count: usize,
    #[serde(default)]
    pub inversed_pda_count: usize,
    #[serde(default)]
    pub stale_pda_count: usize,
    #[serde(default)]
    pub nearest_active_pda: Option<String>,
    #[serde(default)]
    pub nearest_inversed_pda: Option<String>,
    pub filtered_market_regime_label: String,
    pub filtered_liquidity_context_label: String,
    pub filtered_factor_alignment: String,
    pub filtered_factor_uncertainty: String,
    #[serde(default)]
    pub filtered_multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub filtered_multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub filtered_multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub filtered_multi_timeframe_resonance_label: String,
    pub evidence_quality_score: f64,
    pub gating_status: String,
    pub pass_to_bbn: bool,
    #[serde(default)]
    pub uses_soft_evidence: bool,
    #[serde(default)]
    pub conflict_flags: Vec<String>,
    #[serde(default)]
    pub rationale: Vec<String>,
    #[serde(default)]
    pub evidence_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub soft_market_regime_distribution: BTreeMap<String, f64>,
    #[serde(default)]
    pub soft_liquidity_context_distribution: BTreeMap<String, f64>,
    #[serde(default)]
    pub soft_factor_alignment_distribution: BTreeMap<String, f64>,
    #[serde(default)]
    pub soft_factor_uncertainty_distribution: BTreeMap<String, f64>,
    #[serde(default)]
    pub soft_multi_timeframe_resonance_distribution: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RankingDiffItem {
    pub factor_name: String,
    pub previous_score: Option<f64>,
    pub new_score: f64,
    pub score_delta: f64,
    pub previous_weight: Option<f64>,
    pub new_weight: f64,
    pub weight_delta: f64,
    pub previous_action: Option<String>,
    pub new_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProbabilityDiff {
    pub state: String,
    pub previous: Option<f64>,
    pub new: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunProvenance {
    pub prompt_version: String,
    pub factor_version: String,
    pub config_hash: String,
    pub data_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatasetComparability {
    pub comparable: bool,
    pub previous_run_id: Option<String>,
    pub reason: String,
    #[serde(default)]
    pub comparison_class: String,
    #[serde(default)]
    pub same_data: bool,
    #[serde(default)]
    pub same_config: bool,
    #[serde(default)]
    pub same_prompt_version: bool,
    #[serde(default)]
    pub same_factor_version: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromotionDecision {
    pub approved: bool,
    pub status: String,
    pub reason: String,
    pub target_factors: Vec<String>,
    #[serde(default)]
    pub target_families: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RollbackRecommendation {
    pub should_rollback: bool,
    pub scope: String,
    pub reason: String,
    pub target_factors: Vec<String>,
    #[serde(default)]
    pub target_families: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub provenance: RunProvenance,
    pub dataset_comparability: DatasetComparability,
    pub source_command: String,
    pub data_path: String,
    pub epochs: usize,
    pub candles: usize,
    pub observations: usize,
    pub final_state: String,
    pub log_likelihood: f64,
    pub viterbi_log_likelihood: f64,
    #[serde(default)]
    pub workflow_state: WorkflowState,
    #[serde(default)]
    pub agent_action_plan: AgentActionPlan,
    #[serde(default)]
    pub recommended_commands: CommandRecommendations,
    #[serde(default)]
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    #[serde(default)]
    pub agent_context_bundle: AgentContextBundle,
    #[serde(default)]
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    #[serde(default)]
    pub agent_prompts: crate::agent::AgentPromptPack,
    #[serde(default)]
    pub prompt_workflow: String,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    #[serde(default)]
    pub research_objective: String,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub family_history_window: usize,
    pub data_path: String,
    pub paired_data_path: Option<String>,
    pub candles: usize,
    pub paired_candles: Option<usize>,
    pub config_name: String,
    pub source_command: String,
    pub factor_count: usize,
    pub best_factor: Option<String>,
    pub aggregate_return: f64,
    pub feedback_records_generated: usize,
    pub feedback_records_applied: usize,
    pub factor_score_deltas: Vec<RankingDiffItem>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    #[serde(default)]
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    #[serde(default)]
    pub factor_family_history: Vec<FactorFamilyHistory>,
    #[serde(default)]
    pub decision_history_summary: DecisionHistorySummary,
    #[serde(default)]
    pub workflow_state: WorkflowState,
    #[serde(default)]
    pub agent_action_plan: AgentActionPlan,
    #[serde(default)]
    pub recommended_commands: CommandRecommendations,
    #[serde(default)]
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    #[serde(default)]
    pub agent_context_bundle: AgentContextBundle,
    #[serde(default)]
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    #[serde(default)]
    pub feedback_history_summary: FeedbackHistorySummary,
    #[serde(default)]
    pub artifact_action_summary: Vec<String>,
    #[serde(default)]
    pub duration_sizing_scale: Option<f64>,
    #[serde(default)]
    pub hybrid_duration_model: Option<String>,
    #[serde(default)]
    pub hybrid_remaining_expected_bars: Option<f64>,
    #[serde(default)]
    pub backtest_conformal_coverage_1sigma: f64,
    #[serde(default)]
    pub backtest_trade_count: usize,
    #[serde(default)]
    pub canonical_structural_regime_posterior: Option<CanonicalStructuralRegimePosterior>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_decision_section: ArtifactDecisionSection,
    #[serde(default)]
    pub agent_prompts: crate::agent::AgentPromptPack,
    pub prompt_workflow: String,
    #[serde(default)]
    pub factor_mutation_evaluation: Option<FactorMutationEvaluation>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub execution_artifact_id: Option<String>,
    #[serde(default)]
    pub execution_edge_share: Option<f64>,
    #[serde(default)]
    pub prediction_edge_share: Option<f64>,
    #[serde(default)]
    pub execution_readiness: Option<f64>,
    #[serde(default)]
    pub execution_gate_status: Option<String>,
    #[serde(default)]
    pub pda_cluster_label: Option<String>,
    #[serde(default)]
    pub control_matrix_plan: Option<crate::application::backtest::ControlMatrixPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMutationSpec {
    #[serde(default)]
    pub mutation_id: String,
    #[serde(default)]
    pub base_factor: String,
    #[serde(default)]
    pub hypothesis: String,
    #[serde(default)]
    pub parameter_overrides: BTreeMap<String, f64>,
    #[serde(default)]
    pub direction_hints: BTreeMap<String, String>,
    #[serde(default)]
    pub step_size_hints: BTreeMap<String, f64>,
    #[serde(default)]
    pub enabled_overrides: BTreeMap<String, bool>,
    #[serde(default)]
    pub evaluate_expansion_preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMutationMetricSet {
    pub best_factor_composite_score: f64,
    pub aggregate_return: f64,
    pub feedback_records_generated: usize,
    pub feedback_records_applied: usize,
    #[serde(default)]
    pub top_factor_names: Vec<String>,
    #[serde(default)]
    pub expansion_selected_direction: Option<String>,
    #[serde(default)]
    pub expansion_selected_win_probability: Option<f64>,
    #[serde(default)]
    pub expansion_balanced_accuracy: Option<f64>,
    #[serde(default)]
    pub expansion_directional_accuracy: Option<f64>,
    #[serde(default)]
    pub pre_bayes_gate_status: Option<String>,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: Option<String>,
    #[serde(default)]
    pub pre_bayes_bridge_probability_gap: Option<f64>,
    #[serde(default)]
    pub pre_bayes_soft_evidence_divergence_count: usize,
    #[serde(default)]
    pub worst_market_balanced_accuracy: Option<f64>,
    #[serde(default)]
    pub worst_market_bridge_probability_gap: Option<f64>,
    #[serde(default)]
    pub regressed_markets: Vec<String>,
    #[serde(default)]
    pub regression_reasons_by_market: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub multi_timeframe_direction_bias: Option<String>,
    #[serde(default)]
    pub multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub multi_timeframe_entry_alignment_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMutationEvaluation {
    pub mutation_id: String,
    pub accepted: bool,
    pub score_before: f64,
    pub score_after: f64,
    pub score_delta: f64,
    pub baseline_available: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub failure_tags: Vec<String>,
    #[serde(default)]
    pub recommended_mutation_directions: Vec<String>,
    #[serde(default)]
    pub metrics_before: Option<FactorMutationMetricSet>,
    pub metrics_after: FactorMutationMetricSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMutationRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub source_command: String,
    pub data_path: String,
    #[serde(default)]
    pub paired_data_path: Option<String>,
    pub mutation_spec: FactorMutationSpec,
    pub evaluation: FactorMutationEvaluation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchDecision {
    pub status: String,
    pub reason: String,
    pub promoted_to_baseline: bool,
    pub baseline_score_before: f64,
    pub candidate_score: f64,
    pub score_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchAttempt {
    pub session_id: String,
    pub attempt_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub source_command: String,
    pub base_factor: String,
    #[serde(default)]
    pub baseline_mutation_id_before: Option<String>,
    pub candidate_mutation_spec: FactorMutationSpec,
    pub evaluation: FactorMutationEvaluation,
    pub decision: FactorAutoresearchDecision,
    #[serde(default)]
    pub branch_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchSession {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub symbol: String,
    pub objective: String,
    pub source_command: String,
    pub base_factor: String,
    #[serde(default)]
    pub baseline_mutation_id: Option<String>,
    pub baseline_score: f64,
    pub attempts_total: usize,
    pub kept_attempts: usize,
    pub discarded_attempts: usize,
    #[serde(default)]
    pub last_attempt_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchLiveSnapshot {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub symbol: String,
    pub objective: String,
    pub current_iteration: usize,
    pub attempts_total: usize,
    pub kept_attempts: usize,
    pub discarded_attempts: usize,
    #[serde(default)]
    pub current_stage: String,
    #[serde(default)]
    pub current_candidate_spec: Option<FactorMutationSpec>,
    #[serde(default)]
    pub latest_attempt_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchSummary {
    pub session: FactorAutoresearchSession,
    #[serde(default)]
    pub latest_attempt: Option<FactorAutoresearchAttempt>,
    #[serde(default)]
    pub next_mutation_spec_template: Option<FactorMutationSpec>,
    #[serde(default)]
    pub live_snapshot: Option<FactorAutoresearchLiveSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanonicalStructuralRegimePosterior {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_regime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub probabilities: BTreeMap<String, f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub family_history_window: usize,
    pub source_command: String,
    pub data_htf_path: Option<String>,
    pub data_mtf_path: Option<String>,
    pub data_ltf_path: Option<String>,
    pub live_data_source: Option<LiveDataSourceProvenance>,
    pub htf_bars: usize,
    pub mtf_bars: usize,
    pub ltf_bars: usize,
    pub selected_direction: Direction,
    pub selected_entry_quality: String,
    pub decision_hint: String,
    #[serde(default)]
    pub regime_probs: Option<RegimeProbs>,
    #[serde(default)]
    pub market_state_evidence: Vec<String>,
    #[serde(default)]
    pub entry_model_packets: crate::application::entry_models::EntryModelPacketStore,
    #[serde(default)]
    pub hybrid_regime_label: Option<String>,
    #[serde(default)]
    pub hybrid_regime_age_bars: Option<usize>,
    #[serde(default)]
    pub hybrid_duration_model: Option<String>,
    #[serde(default)]
    pub hybrid_remaining_expected_bars: Option<f64>,
    #[serde(default)]
    pub pre_bayes_evidence_filter: PreBayesEvidenceFilter,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge: PreBayesEntryQualityBridge,
    #[serde(default)]
    pub canonical_structural_regime_posterior: Option<CanonicalStructuralRegimePosterior>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    pub factor_family_history: Vec<FactorFamilyHistory>,
    pub decision_history_summary: DecisionHistorySummary,
    pub workflow_state: WorkflowState,
    pub agent_action_plan: AgentActionPlan,
    pub recommended_commands: CommandRecommendations,
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    pub agent_context_bundle: AgentContextBundle,
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    pub feedback_history_summary: FeedbackHistorySummary,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub execution_artifact_id: Option<String>,
    #[serde(default)]
    pub execution_edge_share: Option<f64>,
    #[serde(default)]
    pub prediction_edge_share: Option<f64>,
    #[serde(default)]
    pub execution_readiness: Option<f64>,
    #[serde(default)]
    pub execution_gate_status: Option<String>,
    #[serde(default)]
    pub artifact_action_summary: Vec<String>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_decision_section: ArtifactDecisionSection,
    pub agent_prompts: crate::agent::AgentPromptPack,
    pub prompt_workflow: String,
}

impl Default for AnalyzeRunRecord {
    fn default() -> Self {
        Self {
            run_id: String::new(),
            timestamp: Utc::now(),
            symbol: String::new(),
            provenance: RunProvenance::default(),
            decision_thresholds: DecisionThresholds::default(),
            dataset_comparability: DatasetComparability::default(),
            promotion_decision: PromotionDecision::default(),
            rollback_recommendation: RollbackRecommendation::default(),
            family_history_window: 0,
            source_command: String::new(),
            data_htf_path: None,
            data_mtf_path: None,
            data_ltf_path: None,
            live_data_source: None,
            htf_bars: 0,
            mtf_bars: 0,
            ltf_bars: 0,
            selected_direction: Direction::Neutral,
            selected_entry_quality: String::new(),
            decision_hint: String::new(),
            regime_probs: None,
            market_state_evidence: Vec::new(),
            entry_model_packets: crate::application::entry_models::EntryModelPacketStore::default(),
            hybrid_regime_label: None,
            hybrid_regime_age_bars: None,
            hybrid_duration_model: None,
            hybrid_remaining_expected_bars: None,
            pre_bayes_evidence_filter: PreBayesEvidenceFilter::default(),
            pre_bayes_entry_quality_bridge: PreBayesEntryQualityBridge::default(),
            canonical_structural_regime_posterior: None,
            factor_family_decisions: Vec::new(),
            factor_family_outcomes: Vec::new(),
            factor_family_diffs: Vec::new(),
            factor_family_history: Vec::new(),
            decision_history_summary: DecisionHistorySummary::default(),
            workflow_state: WorkflowState::default(),
            agent_action_plan: AgentActionPlan::default(),
            recommended_commands: CommandRecommendations::default(),
            recommended_next_command: String::new(),
            recommended_next_command_meta: RecommendedNextCommandMeta::default(),
            agent_context_bundle: AgentContextBundle::default(),
            agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
            feedback_history_summary: FeedbackHistorySummary::default(),
            multi_timeframe_summary: Vec::new(),
            execution_artifact_id: None,
            execution_edge_share: None,
            prediction_edge_share: None,
            execution_readiness: None,
            execution_gate_status: None,
            artifact_action_summary: Vec::new(),
            artifact_decision_summary: ArtifactDecisionSummary::default(),
            artifact_decision_section: ArtifactDecisionSection::default(),
            agent_prompts: crate::agent::AgentPromptPack::default(),
            prompt_workflow: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    #[serde(default)]
    pub ensemble_executor_scorecards: Vec<EnsembleExecutorScorecard>,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub family_history_window: usize,
    pub source_command: String,
    pub normalized_entry_quality: String,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub realized_outcome: String,
    #[serde(default)]
    pub structural_learning_credit_class: Option<String>,
    #[serde(default)]
    pub structural_learning_success_credit: Option<f64>,
    #[serde(default)]
    pub structural_learning_observation_weight: Option<f64>,
    pub feedback_records_applied: usize,
    pub duplicate_feedback_skipped: bool,
    #[serde(default)]
    pub consumed_pending_update_artifact_id: Option<String>,
    #[serde(default)]
    pub consumed_execution_candidate_artifact_id: Option<String>,
    #[serde(default)]
    pub consumed_artifact_path: Option<String>,
    #[serde(default)]
    pub consumed_analyze_run_id: Option<String>,
    #[serde(default)]
    pub consumed_pre_bayes_evidence_filter: Option<PreBayesEvidenceFilter>,
    #[serde(default)]
    pub consumed_pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    #[serde(default)]
    pub consumed_canonical_structural_regime_posterior: Option<CanonicalStructuralRegimePosterior>,
    #[serde(default)]
    pub consumed_multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub structural_feedback: Option<StructuralFeedbackRefs>,
    pub trade_outcome_deltas: Vec<ProbabilityDiff>,
    pub factor_score_deltas: Vec<RankingDiffItem>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    #[serde(default)]
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    #[serde(default)]
    pub factor_family_history: Vec<FactorFamilyHistory>,
    #[serde(default)]
    pub decision_history_summary: DecisionHistorySummary,
    #[serde(default)]
    pub workflow_state: WorkflowState,
    #[serde(default)]
    pub agent_action_plan: AgentActionPlan,
    #[serde(default)]
    pub recommended_commands: CommandRecommendations,
    #[serde(default)]
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    #[serde(default)]
    pub agent_context_bundle: AgentContextBundle,
    #[serde(default)]
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    #[serde(default)]
    pub feedback_history_summary: FeedbackHistorySummary,
    #[serde(default)]
    pub artifact_action_summary: Vec<String>,
    #[serde(default)]
    pub duration_sizing_scale: Option<f64>,
    #[serde(default)]
    pub hybrid_duration_model: Option<String>,
    #[serde(default)]
    pub hybrid_remaining_expected_bars: Option<f64>,
    #[serde(default)]
    pub execution_artifact_id: Option<String>,
    #[serde(default)]
    pub execution_edge_share: Option<f64>,
    #[serde(default)]
    pub prediction_edge_share: Option<f64>,
    #[serde(default)]
    pub execution_readiness: Option<f64>,
    #[serde(default)]
    pub execution_gate_status: Option<String>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_decision_section: ArtifactDecisionSection,
    #[serde(default)]
    pub agent_prompts: crate::agent::AgentPromptPack,
    pub prompt_workflow: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BacktestRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub family_history_window: usize,
    pub data_path: String,
    pub paired_data_path: Option<String>,
    pub candles: usize,
    pub paired_candles: Option<usize>,
    pub warmup_bars: usize,
    pub hold_bars: usize,
    pub online_learning: bool,
    pub source_command: String,
    pub total_return: f64,
    pub trade_count: usize,
    #[serde(default)]
    pub conformal_coverage_1sigma: f64,
    #[serde(default)]
    pub conformal_miscoverage_1sigma: f64,
    #[serde(default)]
    pub mean_prediction_interval_half_width: f64,
    #[serde(default)]
    pub worst_window_miscoverage: f64,
    #[serde(default)]
    pub regime_break_penalty: f64,
    #[serde(default)]
    pub structural_break_score: f64,
    #[serde(default)]
    pub structural_break_index: Option<usize>,
    #[serde(default)]
    pub structural_break_detected: bool,
    #[serde(default)]
    pub signal_structural_break_score: f64,
    #[serde(default)]
    pub signal_structural_break_index: Option<usize>,
    #[serde(default)]
    pub signal_structural_break_detected: bool,
    #[serde(default)]
    pub residual_structural_break_score: f64,
    #[serde(default)]
    pub residual_structural_break_index: Option<usize>,
    #[serde(default)]
    pub residual_structural_break_detected: bool,
    #[serde(default)]
    pub rolling_ic_structural_break_score: f64,
    #[serde(default)]
    pub rolling_ic_structural_break_index: Option<usize>,
    #[serde(default)]
    pub rolling_ic_structural_break_detected: bool,
    pub factor_score_deltas: Vec<RankingDiffItem>,
    pub trade_outcome_deltas: Vec<ProbabilityDiff>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    #[serde(default)]
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    #[serde(default)]
    pub factor_family_history: Vec<FactorFamilyHistory>,
    #[serde(default)]
    pub decision_history_summary: DecisionHistorySummary,
    #[serde(default)]
    pub workflow_state: WorkflowState,
    #[serde(default)]
    pub agent_action_plan: AgentActionPlan,
    #[serde(default)]
    pub recommended_commands: CommandRecommendations,
    #[serde(default)]
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    #[serde(default)]
    pub agent_context_bundle: AgentContextBundle,
    #[serde(default)]
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    #[serde(default)]
    pub feedback_history_summary: FeedbackHistorySummary,
    #[serde(default)]
    pub artifact_action_summary: Vec<String>,
    #[serde(default)]
    pub duration_sizing_scale: Option<f64>,
    #[serde(default)]
    pub hybrid_duration_model: Option<String>,
    #[serde(default)]
    pub hybrid_remaining_expected_bars: Option<f64>,
    #[serde(default)]
    pub execution_artifact_id: Option<String>,
    #[serde(default)]
    pub execution_edge_share: Option<f64>,
    #[serde(default)]
    pub prediction_edge_share: Option<f64>,
    #[serde(default)]
    pub execution_readiness: Option<f64>,
    #[serde(default)]
    pub execution_gate_status: Option<String>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_decision_section: ArtifactDecisionSection,
    #[serde(default)]
    pub agent_prompts: crate::agent::AgentPromptPack,
    pub prompt_workflow: String,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub objective_market_credibility_shrink:
        Option<crate::domain::belief::ObjectiveMarketCredibilityShrink>,
    #[serde(default)]
    pub canonical_structural_regime_posterior: Option<CanonicalStructuralRegimePosterior>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedFactorRanking {
    pub factor_name: String,
    pub regime: String,
    pub ic: f64,
    pub ir: f64,
    pub backtest_return: f64,
    pub sharpe: f64,
    pub stability: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub trade_count: usize,
    #[serde(default)]
    pub conformal_coverage_1sigma: f64,
    #[serde(default)]
    pub conformal_miscoverage_1sigma: f64,
    #[serde(default)]
    pub mean_prediction_interval_half_width: f64,
    #[serde(default)]
    pub worst_window_miscoverage: f64,
    #[serde(default)]
    pub regime_break_penalty: f64,
    pub weight: f64,
    pub regime_scores: BTreeMap<String, f64>,
    pub composite_score: f64,
    pub score_breakdown: BTreeMap<String, f64>,
    pub grade: String,
    pub iteration_action: String,
    pub replacement_candidate: bool,
    pub weaknesses: Vec<String>,
    pub agent_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorIterationPrompt {
    pub factor_name: String,
    pub composite_score: f64,
    pub grade: String,
    pub iteration_action: String,
    pub replacement_candidate: bool,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorFamilyDecision {
    pub family: String,
    pub factor_count: usize,
    pub avg_score: f64,
    pub actions: Vec<String>,
    pub replacement_candidates: Vec<String>,
    #[serde(default)]
    pub dominant_action: String,
    #[serde(default)]
    pub decision_status: String,
    #[serde(default)]
    pub decision_reason: String,
    #[serde(default)]
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorFamilyOutcome {
    pub family: String,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorFamilyDiff {
    pub family: String,
    pub previous_avg_score: Option<f64>,
    pub new_avg_score: f64,
    pub avg_score_delta: f64,
    pub previous_replacement_count: usize,
    pub new_replacement_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionHistorySummary {
    pub total_runs: usize,
    pub promotion_approved_runs: usize,
    pub rollback_recommended_runs: usize,
    pub latest_promotion_status: Option<String>,
    pub latest_rollback_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentActionItem {
    pub stage: String,
    pub blocking: bool,
    pub priority: String,
    pub title: String,
    pub rationale: String,
    pub expected_output: String,
    pub expected_state_changes: Vec<ExpectedStateChange>,
    pub suggested_files: Vec<String>,
    pub suggested_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExpectedStateChange {
    pub target: String,
    pub direction: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentActionPlan {
    pub summary: String,
    pub items: Vec<AgentActionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecommendedCommand {
    pub command: String,
    pub ready: bool,
    #[serde(default)]
    pub missing_inputs: Vec<String>,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub user_data_selection_required: bool,
    #[serde(default)]
    pub user_data_selection_prompt: String,
    #[serde(default)]
    pub recorded_data_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RecommendedNextCommandKind {
    IctEngine,
    AskUser,
    Blocked,
    Unavailable,
    Other,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecommendedNextCommandMeta {
    #[serde(default)]
    pub kind: RecommendedNextCommandKind,
    #[serde(default)]
    pub requires_user_input: bool,
    #[serde(default)]
    pub blocked: bool,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub executable_command: Option<String>,
    #[serde(default)]
    pub recorded_data_paths: Vec<String>,
}

pub fn recommended_next_command_meta(raw: &str) -> RecommendedNextCommandMeta {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "recommended_command_unavailable" {
        return RecommendedNextCommandMeta {
            kind: RecommendedNextCommandKind::Unavailable,
            ..RecommendedNextCommandMeta::default()
        };
    }
    if let Some(rest) = trimmed.strip_prefix("ask-user: ") {
        let prompt = rest
            .split(" | blocked until ")
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let executable_command = rest
            .split("| then ")
            .nth(1)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let recorded_data_paths = rest
            .split("recorded_paths=")
            .nth(1)
            .and_then(|tail| tail.split('|').next())
            .map(|paths| {
                paths
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        return RecommendedNextCommandMeta {
            kind: RecommendedNextCommandKind::AskUser,
            requires_user_input: true,
            blocked: true,
            prompt,
            executable_command,
            recorded_data_paths,
        };
    }
    if trimmed.starts_with("blocked:") {
        return RecommendedNextCommandMeta {
            kind: RecommendedNextCommandKind::Blocked,
            blocked: true,
            ..RecommendedNextCommandMeta::default()
        };
    }
    if trimmed.starts_with("ict-engine ") {
        return RecommendedNextCommandMeta {
            kind: RecommendedNextCommandKind::IctEngine,
            executable_command: Some(trimmed.to_string()),
            ..RecommendedNextCommandMeta::default()
        };
    }
    RecommendedNextCommandMeta {
        kind: RecommendedNextCommandKind::Other,
        executable_command: Some(trimmed.to_string()),
        ..RecommendedNextCommandMeta::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandRecommendations {
    pub analyze: RecommendedCommand,
    pub research: RecommendedCommand,
    pub backtest: RecommendedCommand,
    pub update: RecommendedCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiveDataSourceProvenance {
    pub futures_backend: String,
    pub aux_backend: String,
    pub futures_base_url: String,
    pub aux_base_url: String,
    pub futures_symbol: String,
    pub spot_symbol: String,
    pub options_symbol: String,
    pub spot_kind: String,
    pub fetched_at: DateTime<Utc>,
    #[serde(default)]
    pub persisted_htf_path: Option<String>,
    #[serde(default)]
    pub persisted_h4_path: Option<String>,
    #[serde(default)]
    pub persisted_mtf_path: Option<String>,
    #[serde(default)]
    pub persisted_m5_path: Option<String>,
    #[serde(default)]
    pub persisted_ltf_path: Option<String>,
    #[serde(default)]
    pub persisted_m1_path: Option<String>,
    #[serde(default)]
    pub persisted_spot_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorFamilyHistory {
    pub family: String,
    pub window_size: usize,
    pub recent_run_ids: Vec<String>,
    pub recent_timestamps: Vec<DateTime<Utc>>,
    pub recent_avg_scores: Vec<f64>,
    pub recent_replacement_counts: Vec<usize>,
    pub score_trend: String,
    pub replacement_trend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowState {
    pub phase: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPhaseSnapshot {
    pub phase: String,
    pub source_command: String,
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub workflow_phase: String,
    pub workflow_reason: String,
    pub promotion_status: String,
    pub rollback_scope: String,
    pub comparable_to_previous: bool,
    pub comparison_class: String,
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    pub phase_summary: String,
    pub top_actions: Vec<String>,
    pub risk_flags: Vec<String>,
    #[serde(default)]
    pub selected_direction: Option<String>,
    #[serde(default)]
    pub selected_entry_quality: Option<String>,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_uses_soft_evidence: bool,
    #[serde(default)]
    pub pre_bayes_policy_version: String,
    #[serde(default)]
    pub pre_bayes_evidence_quality_score: f64,
    #[serde(default)]
    pub pre_bayes_conflict_flags: Vec<String>,
    #[serde(default)]
    pub pre_bayes_filtered_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub pre_bayes_soft_evidence: BTreeMap<String, BTreeMap<String, f64>>,
    #[serde(default)]
    pub market_state_evidence: Vec<String>,
    #[serde(default)]
    pub canonical_structural_active_regime: Option<String>,
    #[serde(default)]
    pub canonical_structural_confidence: Option<f64>,
    #[serde(default)]
    pub canonical_structural_probabilities: BTreeMap<String, f64>,
    #[serde(default)]
    pub pre_bayes_long_signal_probability: Option<f64>,
    #[serde(default)]
    pub pre_bayes_short_signal_probability: Option<f64>,
    #[serde(default)]
    pub pre_bayes_selected_entry_quality_probability: Option<f64>,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: Option<String>,
    #[serde(default)]
    pub pre_bayes_bridge_probability_gap: Option<f64>,
    #[serde(default)]
    pub pre_bayes_bridge_rationale_summary: Vec<String>,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub realized_outcome: Option<String>,
    #[serde(default)]
    pub structural_learning_credit_class: Option<String>,
    #[serde(default)]
    pub structural_learning_success_credit: Option<f64>,
    #[serde(default)]
    pub structural_learning_observation_weight: Option<f64>,
    #[serde(default)]
    pub family_states: Vec<String>,
    #[serde(default)]
    pub factor_actions: Vec<String>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub structural_feedback: Option<StructuralFeedbackRefs>,
    #[serde(default)]
    pub family_score_map: BTreeMap<String, f64>,
    #[serde(default)]
    pub factor_score_map: BTreeMap<String, f64>,
    #[serde(default)]
    pub objective_market_credibility_shrink:
        Option<crate::domain::belief::ObjectiveMarketCredibilityShrink>,
    #[serde(default)]
    pub execution_edge_share: Option<f64>,
    #[serde(default)]
    pub prediction_edge_share: Option<f64>,
    #[serde(default)]
    pub execution_readiness: Option<f64>,
    #[serde(default)]
    pub execution_gate_status: Option<String>,
    #[serde(default)]
    pub pda_cluster_label: Option<String>,
    #[serde(default)]
    pub hybrid_duration_model: Option<String>,
    #[serde(default)]
    pub hybrid_remaining_expected_bars: Option<f64>,
    /// Round 2 §3.4: spectral entropy from the latest execution_artifact. None
    /// when the spectral layer did not fit (window too short) or when the
    /// artifact has not been populated yet.
    #[serde(default)]
    pub spectral_entropy: Option<f64>,
    /// Round 2 §3.4: softshrink sparsity ratio from the latest mece_recovery
    /// artifact. None when MECE recovery has not been run.
    #[serde(default)]
    pub sparsity_ratio: Option<f64>,
    /// Round 2 §3.4: "promote" / "blocked" verdict from the MECE recovery
    /// segments gate. None when recovery segments are empty or unrun.
    #[serde(default)]
    pub segments_gate: Option<String>,
}

impl Default for WorkflowPhaseSnapshot {
    fn default() -> Self {
        Self {
            phase: String::new(),
            source_command: String::new(),
            run_id: String::new(),
            timestamp: Utc::now(),
            workflow_phase: String::new(),
            workflow_reason: String::new(),
            promotion_status: String::new(),
            rollback_scope: String::new(),
            comparable_to_previous: false,
            comparison_class: String::new(),
            recommended_next_command: String::new(),
            recommended_next_command_meta: RecommendedNextCommandMeta::default(),
            phase_summary: String::new(),
            top_actions: Vec::new(),
            risk_flags: Vec::new(),
            selected_direction: None,
            selected_entry_quality: None,
            pre_bayes_gate_status: String::new(),
            pre_bayes_uses_soft_evidence: false,
            pre_bayes_policy_version: String::new(),
            pre_bayes_evidence_quality_score: 0.0,
            pre_bayes_conflict_flags: Vec::new(),
            pre_bayes_filtered_assignments: BTreeMap::new(),
            pre_bayes_soft_evidence: BTreeMap::new(),
            market_state_evidence: Vec::new(),
            canonical_structural_active_regime: None,
            canonical_structural_confidence: None,
            canonical_structural_probabilities: BTreeMap::new(),
            pre_bayes_long_signal_probability: None,
            pre_bayes_short_signal_probability: None,
            pre_bayes_selected_entry_quality_probability: None,
            pre_bayes_bridge_selected_entry_quality: None,
            pre_bayes_bridge_probability_gap: None,
            pre_bayes_bridge_rationale_summary: Vec::new(),
            pre_bayes_multi_timeframe_direction_bias: String::new(),
            pre_bayes_multi_timeframe_alignment_score: None,
            pre_bayes_multi_timeframe_entry_alignment_score: None,
            realized_outcome: None,
            structural_learning_credit_class: None,
            structural_learning_success_credit: None,
            structural_learning_observation_weight: None,
            family_states: Vec::new(),
            factor_actions: Vec::new(),
            multi_timeframe_summary: Vec::new(),
            structural_feedback: None,
            family_score_map: BTreeMap::new(),
            factor_score_map: BTreeMap::new(),
            objective_market_credibility_shrink: None,
            execution_edge_share: None,
            prediction_edge_share: None,
            execution_readiness: None,
            execution_gate_status: None,
            pda_cluster_label: None,
            hybrid_duration_model: None,
            hybrid_remaining_expected_bars: None,
            spectral_entropy: None,
            sparsity_ratio: None,
            segments_gate: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowConflictSource {
    pub scope: String,
    pub subject: String,
    pub left_phase: String,
    pub left_value: String,
    pub right_phase: String,
    pub right_value: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowFieldDiff {
    pub left_phase: String,
    pub right_phase: String,
    pub field: String,
    pub left_value: String,
    pub right_value: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDisagreement {
    pub id: String,
    pub severity: String,
    pub summary: String,
    pub phases: Vec<String>,
    pub recommended_action: String,
    pub evidence: Vec<String>,
    #[serde(default)]
    pub sources: Vec<WorkflowConflictSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    pub symbol: String,
    pub generated_at: DateTime<Utc>,
    pub current_focus_phase: String,
    pub current_focus_reason: String,
    #[serde(default)]
    pub blocking_truth: WorkflowBlockingTruth,
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    pub pending_actions: Vec<String>,
    pub risk_flags: Vec<String>,
    #[serde(default)]
    pub latest_train: Option<WorkflowPhaseSnapshot>,
    pub latest_analyze: Option<WorkflowPhaseSnapshot>,
    pub latest_research: Option<WorkflowPhaseSnapshot>,
    pub latest_backtest: Option<WorkflowPhaseSnapshot>,
    pub latest_update: Option<WorkflowPhaseSnapshot>,
    #[serde(default)]
    pub latest_pre_bayes_policy: Option<PreBayesEvidencePolicy>,
    #[serde(default)]
    pub latest_pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    #[serde(default)]
    pub latest_pre_bayes_entry_quality_bridge_diff: Option<PreBayesEntryQualityBridgeDiff>,
    #[serde(default)]
    pub latest_pre_bayes_policy_diff: Option<PreBayesPolicyDiff>,
    #[serde(default)]
    pub latest_pre_bayes_policy_lineage: Option<PreBayesPolicyLineageSummary>,
    #[serde(default)]
    pub latest_pre_bayes_soft_evidence_diff: Vec<PreBayesSoftEvidenceNodeDiff>,
    #[serde(default)]
    pub recent_pre_bayes_policies: Vec<PreBayesPolicyRecord>,
    #[serde(default)]
    pub latest_pending_update: Option<PendingUpdateArtifactSummary>,
    #[serde(default)]
    pub recent_pending_updates: Vec<PendingUpdateArtifactSummary>,
    #[serde(default)]
    pub latest_execution_candidate: Option<ExecutionCandidateArtifactSummary>,
    #[serde(default)]
    pub recent_execution_candidates: Vec<ExecutionCandidateArtifactSummary>,
    #[serde(default)]
    pub latest_ensemble_vote: Option<EnsembleVoteRecord>,
    #[serde(default)]
    pub recent_ensemble_votes: Vec<EnsembleVoteRecord>,
    #[serde(default)]
    pub recent_artifacts: Vec<ArtifactLedgerEntry>,
    #[serde(default)]
    pub actionable_artifacts: Vec<ArtifactLedgerEntry>,
    #[serde(default)]
    pub latest_promotable_artifact: Option<ArtifactLedgerEntry>,
    #[serde(default)]
    pub artifact_history_summary: ArtifactHistorySummary,
    #[serde(default)]
    pub artifact_factor_trends: Vec<ArtifactFactorTrendSummary>,
    #[serde(default)]
    pub artifact_family_trends: Vec<ArtifactFamilyTrendSummary>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_review_rules: ArtifactReviewRules,
    #[serde(default)]
    pub artifact_review_rule_sources: ArtifactReviewRuleSources,
    #[serde(default)]
    pub artifact_lineage_summaries: Vec<ArtifactLineageSummary>,
    #[serde(default)]
    pub artifact_rule_break_effects: Vec<ArtifactRuleBreakEffect>,
    #[serde(default)]
    pub artifact_factor_rule_break_impacts: Vec<ArtifactRuleBreakFactorImpact>,
    #[serde(default)]
    pub artifact_family_rule_break_impacts: Vec<ArtifactRuleBreakFamilyImpact>,
    #[serde(default)]
    pub artifact_consumed_impact_summary: ArtifactConsumedImpactSummary,
    #[serde(default)]
    pub field_diffs: Vec<WorkflowFieldDiff>,
    #[serde(default)]
    pub disagreements: Vec<WorkflowDisagreement>,
}

impl Default for WorkflowSnapshot {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            generated_at: Utc::now(),
            current_focus_phase: String::new(),
            current_focus_reason: String::new(),
            blocking_truth: WorkflowBlockingTruth::default(),
            recommended_next_command: String::new(),
            recommended_next_command_meta: RecommendedNextCommandMeta::default(),
            pending_actions: Vec::new(),
            risk_flags: Vec::new(),
            latest_train: None,
            latest_analyze: None,
            latest_research: None,
            latest_backtest: None,
            latest_update: None,
            latest_pre_bayes_policy: None,
            latest_pre_bayes_entry_quality_bridge: None,
            latest_pre_bayes_entry_quality_bridge_diff: None,
            latest_pre_bayes_policy_diff: None,
            latest_pre_bayes_policy_lineage: None,
            latest_pre_bayes_soft_evidence_diff: Vec::new(),
            recent_pre_bayes_policies: Vec::new(),
            latest_pending_update: None,
            recent_pending_updates: Vec::new(),
            latest_execution_candidate: None,
            recent_execution_candidates: Vec::new(),
            latest_ensemble_vote: None,
            recent_ensemble_votes: Vec::new(),
            recent_artifacts: Vec::new(),
            actionable_artifacts: Vec::new(),
            latest_promotable_artifact: None,
            artifact_history_summary: ArtifactHistorySummary::default(),
            artifact_factor_trends: Vec::new(),
            artifact_family_trends: Vec::new(),
            artifact_decision_summary: ArtifactDecisionSummary::default(),
            artifact_review_rules: ArtifactReviewRules::default(),
            artifact_review_rule_sources: ArtifactReviewRuleSources::default(),
            artifact_lineage_summaries: Vec::new(),
            artifact_rule_break_effects: Vec::new(),
            artifact_factor_rule_break_impacts: Vec::new(),
            artifact_family_rule_break_impacts: Vec::new(),
            artifact_consumed_impact_summary: ArtifactConsumedImpactSummary::default(),
            field_diffs: Vec::new(),
            disagreements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowBlockingTruth {
    pub stage: String,
    pub status: String,
    pub reason: String,
    pub evidence: Vec<String>,
    pub next_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentContextBundle {
    pub workflow_state: WorkflowState,
    pub decision_hint: String,
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    pub recommended_commands: CommandRecommendations,
    pub family_history_window: usize,
    pub comparable_to_last_run: bool,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_uses_soft_evidence: bool,
    #[serde(default)]
    pub pre_bayes_evidence_quality_score: f64,
    #[serde(default)]
    pub pre_bayes_conflict_flags: Vec<String>,
    #[serde(default)]
    pub pre_bayes_filtered_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub pre_bayes_soft_evidence: BTreeMap<String, BTreeMap<String, f64>>,
    #[serde(default)]
    pub pre_bayes_policy_version: String,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge_summary: Vec<String>,
    #[serde(default)]
    pub pre_bayes_soft_evidence_diff: Vec<PreBayesSoftEvidenceNodeDiff>,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge_diff: Option<PreBayesEntryQualityBridgeDiff>,
    #[serde(default)]
    pub factor_mutation_evaluation: Option<FactorMutationEvaluation>,
    #[serde(default)]
    pub factor_mutation_priority_markets: Vec<String>,
    #[serde(default)]
    pub factor_mutation_priority_reasons: Vec<String>,
    #[serde(default)]
    pub factor_mutation_recommended_focus: Vec<String>,
    #[serde(default)]
    pub factor_mutation_direction_hints: Vec<String>,
    #[serde(default)]
    pub factor_mutation_step_size_hints: Vec<String>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub artifact_consumed_gate_status: String,
    #[serde(default)]
    pub artifact_consumed_gate_reason: String,
    #[serde(default)]
    pub artifact_consumed_gate_targets: Vec<String>,
    #[serde(default)]
    pub pda_sequence_summary: Option<String>,
    #[serde(default)]
    pub pda_cluster_label: Option<String>,
    #[serde(default)]
    pub pda_cluster_confidence: Option<f64>,
    pub top_factor_actions: Vec<String>,
    pub family_actions: Vec<String>,
    pub stage_views: Vec<StageAgentContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageAgentContext {
    pub stage: String,
    pub blocking_items: usize,
    pub recommended_command: String,
    pub actions: Vec<String>,
    #[serde(default)]
    pub gate_status: String,
    #[serde(default)]
    pub gate_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentContextBundleMinimal {
    pub workflow_phase: String,
    pub recommended_next_command: String,
    #[serde(default)]
    pub recommended_next_command_meta: RecommendedNextCommandMeta,
    pub family_history_window: usize,
    pub comparable_to_last_run: bool,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_uses_soft_evidence: bool,
    #[serde(default)]
    pub pre_bayes_policy_version: String,
    #[serde(default)]
    pub pre_bayes_soft_evidence_divergence_count: usize,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: String,
    #[serde(default)]
    pub factor_mutation_acceptance_status: String,
    #[serde(default)]
    pub factor_mutation_failure_tags: Vec<String>,
    #[serde(default)]
    pub factor_mutation_priority_markets: Vec<String>,
    #[serde(default)]
    pub factor_mutation_priority_reasons: Vec<String>,
    #[serde(default)]
    pub factor_mutation_direction_hints: Vec<String>,
    #[serde(default)]
    pub factor_mutation_step_size_hints: Vec<String>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub artifact_consumed_gate_status: String,
    #[serde(default)]
    pub pda_cluster_label: Option<String>,
    pub top_factor_actions: Vec<String>,
    pub stage_views: Vec<StageAgentContextMinimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageAgentContextMinimal {
    pub stage: String,
    pub recommended_command: String,
    #[serde(default)]
    pub gate_status: String,
}

impl LearningState {
    pub fn ensure_profile(&mut self, factor_name: &str) -> &mut FactorLearningProfile {
        self.factor_profiles
            .entry(factor_name.to_string())
            .or_default()
    }

    pub fn profile(&self, factor_name: &str) -> Option<&FactorLearningProfile> {
        self.factor_profiles.get(factor_name)
    }

    pub fn feedback_key(record: &FeedbackRecord) -> String {
        let factors = record
            .factors_used
            .iter()
            .map(|factor| {
                format!(
                    "{}:{:?}:{:.6}:{:.6}:{:.6}",
                    factor.factor_name,
                    factor.direction,
                    factor.value,
                    factor.confidence,
                    factor.weight
                )
            })
            .collect::<Vec<_>>()
            .join("|");

        format!(
            "{}|{}|{}|{:?}|{}|{:.8}|{}|{:?}|{:?}|{:.6}|{:.6}|{}|{}",
            record.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true),
            record.symbol,
            record.source,
            record.trade_id,
            record.realized_outcome,
            record.pnl,
            regime_key(record.regime_at_entry),
            record.model_probabilities_before_trade.selected_direction,
            record
                .model_probabilities_before_trade
                .selected_probability
                .to_bits(),
            record.model_probabilities_before_trade.long_score,
            record.model_probabilities_before_trade.short_score,
            factors,
            record
                .structural_feedback
                .as_ref()
                .map(|refs| {
                    format!(
                        "{}|{}|{}|{}|{}|{}|{:?}",
                        refs.protocol_version,
                        refs.recommendation_id,
                        refs.node_id,
                        refs.branch_id,
                        refs.scenario_id,
                        refs.path_id,
                        refs.followed_path
                    )
                })
                .unwrap_or_else(|| "no_structural_feedback".to_string())
        )
    }

    pub fn merge_feedback_records(&mut self, feedback: &[FeedbackRecord]) -> Vec<FeedbackRecord> {
        let mut existing = self
            .feedback_history
            .iter()
            .map(Self::feedback_key)
            .collect::<BTreeSet<_>>();
        let mut unresolved_by_resolution: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        let mut resolved_by_resolution: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for (index, existing_record) in self.feedback_history.iter().enumerate() {
            if let Some(resolution_key) = feedback_resolution_key(existing_record) {
                if structural_feedback_outcome_is_unresolved(&existing_record.realized_outcome) {
                    unresolved_by_resolution
                        .entry(resolution_key)
                        .or_default()
                        .insert(index);
                } else {
                    resolved_by_resolution
                        .entry(resolution_key)
                        .or_default()
                        .insert(index);
                }
            }
        }
        let mut inserted = Vec::new();

        for record in feedback {
            let key = Self::feedback_key(record);
            if existing.insert(key.clone()) {
                let resolution_key = feedback_resolution_key(record);
                let unresolved_outcome =
                    structural_feedback_outcome_is_unresolved(&record.realized_outcome);
                if !unresolved_outcome {
                    if let Some(resolution_key) = resolution_key.as_deref() {
                        if let Some(index) = unresolved_by_resolution
                            .get(resolution_key)
                            .and_then(|indexes| indexes.iter().next().copied())
                        {
                            let old_key = Self::feedback_key(&self.feedback_history[index]);
                            self.feedback_history[index] = record.clone();
                            existing.remove(&old_key);
                            existing.insert(Self::feedback_key(record));
                            let remove_empty_entry =
                                match unresolved_by_resolution.get_mut(resolution_key) {
                                    Some(indexes) => {
                                        indexes.remove(&index);
                                        indexes.is_empty()
                                    }
                                    None => false,
                                };
                            if remove_empty_entry {
                                unresolved_by_resolution.remove(resolution_key);
                            }
                            resolved_by_resolution
                                .entry(resolution_key.to_string())
                                .or_default()
                                .insert(index);
                            inserted.push(record.clone());
                            continue;
                        }
                    }
                } else if let Some(resolution_key) = resolution_key.as_deref() {
                    let resolved_exists = resolved_by_resolution
                        .get(resolution_key)
                        .map(|indexes| !indexes.is_empty())
                        .unwrap_or(false);
                    if resolved_exists {
                        existing.remove(&key);
                        continue;
                    }
                }
                let index = self.feedback_history.len();
                self.feedback_history.push(record.clone());
                if let Some(resolution_key) = resolution_key {
                    if unresolved_outcome {
                        unresolved_by_resolution
                            .entry(resolution_key)
                            .or_default()
                            .insert(index);
                    } else {
                        resolved_by_resolution
                            .entry(resolution_key)
                            .or_default()
                            .insert(index);
                    }
                }
                inserted.push(record.clone());
            }
        }

        inserted
    }

    pub fn replace_feedback_records(&mut self, feedback: &[FeedbackRecord]) {
        let replacements = feedback
            .iter()
            .map(|record| (Self::feedback_key(record), record.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        for existing in &mut self.feedback_history {
            if let Some(replacement) = replacements.get(&Self::feedback_key(existing)) {
                *existing = replacement.clone();
            }
        }
    }

    pub fn apply_structural_feedback(&mut self, feedback: &[FeedbackRecord]) {
        let mut appended = false;
        for record in feedback {
            if structural_feedback_outcome_is_unresolved(&record.realized_outcome) {
                continue;
            }
            let Some(refs) = record.structural_feedback.as_ref() else {
                continue;
            };
            update_structural_source_reliability_from_feedback(
                &mut self.structural_prior_state,
                record,
                refs.followed_path,
            );
            update_structural_target_policy_context_posterior(
                &mut self.structural_prior_state,
                record,
                refs.followed_path,
            );
            update_structural_prior_stats(
                self.structural_prior_state
                    .nodes
                    .entry(refs.node_id.clone())
                    .or_default(),
                record,
                refs.followed_path,
                StructuralPriorEntityKind::Node,
            );
            update_structural_prior_stats(
                self.structural_prior_state
                    .branches
                    .entry(refs.branch_id.clone())
                    .or_default(),
                record,
                refs.followed_path,
                StructuralPriorEntityKind::Branch,
            );
            update_structural_prior_stats(
                self.structural_prior_state
                    .scenarios
                    .entry(refs.scenario_id.clone())
                    .or_default(),
                record,
                refs.followed_path,
                StructuralPriorEntityKind::Scenario,
            );
            update_structural_prior_stats(
                self.structural_prior_state
                    .paths
                    .entry(refs.path_id.clone())
                    .or_default(),
                record,
                refs.followed_path,
                StructuralPriorEntityKind::Path,
            );
            refresh_structural_prior_mass_snapshots_for_refs(
                &mut self.structural_prior_state,
                refs,
            );
            appended |= append_structural_prior_event(
                &mut self.structural_prior_state,
                StructuralPriorEvent {
                    source_label: record.source.clone(),
                    symbol: record.symbol.clone(),
                    recommendation_id: refs.recommendation_id.clone(),
                    recommended_at: refs.recommended_at.clone(),
                    node_id: refs.node_id.clone(),
                    branch_id: refs.branch_id.clone(),
                    scenario_id: refs.scenario_id.clone(),
                    path_id: refs.path_id.clone(),
                    followed_path: refs.followed_path,
                    realized_outcome: Some(record.realized_outcome.clone()),
                },
            );
        }
        if appended {
            rebuild_structural_sequence_priors(&mut self.structural_prior_state);
        }
    }

    pub fn apply_structural_prior_seed(
        &mut self,
        refs: &StructuralFeedbackRefs,
        seed: &StructuralPriorSeed,
    ) {
        apply_structural_prior_seed_to_source_reliability(&mut self.structural_prior_state, seed);
        apply_structural_prior_seed_to_stats(
            self.structural_prior_state
                .nodes
                .entry(refs.node_id.clone())
                .or_default(),
            refs,
            seed,
            StructuralPriorEntityKind::Node,
        );
        apply_structural_prior_seed_to_stats(
            self.structural_prior_state
                .branches
                .entry(refs.branch_id.clone())
                .or_default(),
            refs,
            seed,
            StructuralPriorEntityKind::Branch,
        );
        apply_structural_prior_seed_to_stats(
            self.structural_prior_state
                .scenarios
                .entry(refs.scenario_id.clone())
                .or_default(),
            refs,
            seed,
            StructuralPriorEntityKind::Scenario,
        );
        apply_structural_prior_seed_to_stats(
            self.structural_prior_state
                .paths
                .entry(refs.path_id.clone())
                .or_default(),
            refs,
            seed,
            StructuralPriorEntityKind::Path,
        );
        if seed.observations > 0 {
            self.structural_prior_state.last_offline_seed_snapshot =
                Some(structural_offline_seed_snapshot(refs, seed));
            refresh_structural_prior_mass_snapshots_for_refs(
                &mut self.structural_prior_state,
                refs,
            );
        }
        if append_structural_prior_event(
            &mut self.structural_prior_state,
            StructuralPriorEvent {
                source_label: seed.source_label.clone(),
                symbol: structural_prior_symbol_from_node_id(&refs.node_id),
                recommendation_id: refs.recommendation_id.clone(),
                recommended_at: refs.recommended_at.clone(),
                node_id: refs.node_id.clone(),
                branch_id: refs.branch_id.clone(),
                scenario_id: refs.scenario_id.clone(),
                path_id: refs.path_id.clone(),
                followed_path: refs.followed_path,
                realized_outcome: structural_prior_seed_representative_outcome(seed),
            },
        ) {
            rebuild_structural_sequence_priors(&mut self.structural_prior_state);
        }
    }

    pub fn summary(&self) -> FeedbackHistorySummary {
        if self.feedback_history.is_empty() {
            return FeedbackHistorySummary::default();
        }

        let mut summary = FeedbackHistorySummary::default();
        let mut counts = BTreeMap::<String, (usize, usize)>::new();

        for record in &self.feedback_history {
            summary.total_records += 1;
            summary.avg_pnl += record.pnl;
            if record.pnl > 0.0 || record.realized_outcome == "win" {
                summary.wins += 1;
            } else if record.pnl < 0.0 || record.realized_outcome == "loss" {
                summary.losses += 1;
            }

            for factor in &record.factors_used {
                let entry = counts.entry(factor.factor_name.clone()).or_insert((0, 0));
                entry.0 += 1;

                let selected_direction = record.model_probabilities_before_trade.selected_direction;
                let aligned = match selected_direction {
                    Direction::Bull => factor.long_support >= factor.short_support,
                    Direction::Bear => factor.short_support >= factor.long_support,
                    Direction::Neutral => false,
                };
                let effective_success = if aligned {
                    record.pnl >= 0.0
                } else {
                    record.pnl < 0.0
                };
                if effective_success {
                    entry.1 += 1;
                }
            }
        }

        summary.avg_pnl /= self.feedback_history.len() as f64;
        summary.factor_success_rates = counts
            .into_iter()
            .map(|(factor, (count, success))| {
                (
                    factor,
                    if count > 0 {
                        success as f64 / count as f64
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        summary
    }

    pub fn iteration_queue(&self) -> Vec<FactorIterationPrompt> {
        let mut queue = self
            .factor_rankings
            .iter()
            .filter(|ranking| ranking.iteration_action != "keep" || ranking.replacement_candidate)
            .map(FactorIterationPrompt::from)
            .collect::<Vec<_>>();

        queue.sort_by(|a, b| {
            iteration_priority(&b.iteration_action)
                .cmp(&iteration_priority(&a.iteration_action))
                .then_with(|| {
                    a.composite_score
                        .partial_cmp(&b.composite_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        queue
    }

    pub fn family_decisions(&self) -> Vec<FactorFamilyDecision> {
        let mut grouped = BTreeMap::<String, Vec<&PersistedFactorRanking>>::new();
        for ranking in &self.factor_rankings {
            grouped
                .entry(factor_family(&ranking.factor_name).to_string())
                .or_default()
                .push(ranking);
        }

        let mut decisions = grouped
            .into_iter()
            .map(|(family, items)| {
                let avg_score = items.iter().map(|item| item.composite_score).sum::<f64>()
                    / items.len().max(1) as f64;
                let actions = items
                    .iter()
                    .map(|item| format!("{}:{}", item.factor_name, item.iteration_action))
                    .collect::<Vec<_>>();
                let replacement_candidates = items
                    .iter()
                    .filter(|item| item.replacement_candidate)
                    .map(|item| item.factor_name.clone())
                    .collect::<Vec<_>>();
                let dominant_action = family_dominant_action(&items);
                let avg_score_band = factor_grade(avg_score).to_ascii_lowercase();
                let risk_flags = family_risk_flags(&items, avg_score, &replacement_candidates);

                FactorFamilyDecision {
                    family,
                    factor_count: items.len(),
                    avg_score,
                    actions,
                    replacement_candidates,
                    dominant_action: dominant_action.to_string(),
                    decision_status: family_decision_status(&items, avg_score).to_string(),
                    decision_reason: family_decision_reason(
                        dominant_action,
                        avg_score_band.as_str(),
                        &risk_flags,
                    ),
                    risk_flags,
                }
            })
            .collect::<Vec<_>>();

        decisions.sort_by(|a, b| {
            a.avg_score
                .partial_cmp(&b.avg_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        decisions
    }
}

impl FactorLearningProfile {
    pub fn regime_multiplier(&self, regime: Regime) -> f64 {
        self.regime_stats
            .get(regime_key(regime))
            .map(|stats| {
                if stats.multiplier.abs() <= f64::EPSILON {
                    1.0
                } else {
                    stats.multiplier
                }
            })
            .unwrap_or(1.0)
    }
}

impl From<&FactorIC> for PersistedFactorRanking {
    fn from(value: &FactorIC) -> Self {
        let mut ranking = Self {
            factor_name: value.factor_name.clone(),
            regime: regime_key(value.regime).to_string(),
            ic: value.mean_ic,
            ir: value.ir,
            backtest_return: value.backtest_return,
            sharpe: value.sharpe,
            stability: value.stability,
            win_rate: value.win_rate,
            profit_factor: value.profit_factor,
            trade_count: value.trade_count,
            conformal_coverage_1sigma: 0.0,
            conformal_miscoverage_1sigma: 0.0,
            mean_prediction_interval_half_width: 0.0,
            worst_window_miscoverage: 0.0,
            regime_break_penalty: 0.0,
            weight: value.weight,
            regime_scores: value
                .regime_scores
                .iter()
                .map(|(key, value)| (key.clone(), *value))
                .collect(),
            composite_score: 0.0,
            score_breakdown: BTreeMap::new(),
            grade: String::new(),
            iteration_action: String::new(),
            replacement_candidate: false,
            weaknesses: Vec::new(),
            agent_prompt: String::new(),
        };
        ranking.refresh_scorecard();
        ranking
    }
}

impl From<&PersistedFactorRanking> for FactorIterationPrompt {
    fn from(value: &PersistedFactorRanking) -> Self {
        Self {
            factor_name: value.factor_name.clone(),
            composite_score: value.composite_score,
            grade: value.grade.clone(),
            iteration_action: value.iteration_action.clone(),
            replacement_candidate: value.replacement_candidate,
            prompt: value.agent_prompt.clone(),
        }
    }
}

impl PersistedFactorRanking {
    pub fn refresh_scorecard(&mut self) {
        let regime_breadth = if self.regime_scores.is_empty() {
            0.0
        } else {
            self.regime_scores
                .values()
                .filter(|score| **score > 0.0)
                .count() as f64
                / self.regime_scores.len() as f64
        };
        let sample_score = (self.trade_count as f64 / 20.0).clamp(0.0, 1.0);
        let ic_score = (self.ic.abs() / 0.08).clamp(0.0, 1.0);
        let ir_score = (self.ir / 1.5).clamp(0.0, 1.0);
        let return_score = ((self.backtest_return + 0.02) / 0.12).clamp(0.0, 1.0);
        let sharpe_score = ((self.sharpe + 0.2) / 1.7).clamp(0.0, 1.0);
        let stability_score = self.stability.clamp(0.0, 1.0);
        let win_rate_score = ((self.win_rate - 0.45) / 0.20).clamp(0.0, 1.0);
        let profit_factor_score = ((self.profit_factor - 0.95) / 0.55).clamp(0.0, 1.0);
        let regime_score = regime_breadth.clamp(0.0, 1.0);
        let conformal_score = ((self.conformal_coverage_1sigma - 0.45) / 0.35).clamp(0.0, 1.0);
        let break_penalty_score = (1.0 - (self.regime_break_penalty / 0.35)).clamp(0.0, 1.0);

        self.score_breakdown = BTreeMap::from([
            ("ic".to_string(), ic_score),
            ("ir".to_string(), ir_score),
            ("return".to_string(), return_score),
            ("sharpe".to_string(), sharpe_score),
            ("stability".to_string(), stability_score),
            ("win_rate".to_string(), win_rate_score),
            ("profit_factor".to_string(), profit_factor_score),
            ("regime_coverage".to_string(), regime_score),
            ("conformal_coverage".to_string(), conformal_score),
            ("regime_break_resilience".to_string(), break_penalty_score),
            ("sample".to_string(), sample_score),
        ]);

        self.composite_score = 0.16 * ic_score
            + 0.13 * ir_score
            + 0.16 * return_score
            + 0.10 * sharpe_score
            + 0.10 * stability_score
            + 0.10 * win_rate_score
            + 0.08 * profit_factor_score
            + 0.06 * regime_score
            + 0.07 * conformal_score
            + 0.05 * break_penalty_score
            + 0.05 * sample_score;
        self.composite_score = self.composite_score.clamp(0.0, 1.0);

        self.weaknesses = factor_weaknesses(self, regime_score, sample_score);
        self.grade = factor_grade(self.composite_score).to_string();
        self.iteration_action = factor_iteration_action(self, sample_score).to_string();
        self.replacement_candidate = self.iteration_action == "replace"
            && self.trade_count >= 8
            && self.composite_score < 0.45;
        self.agent_prompt = build_agent_prompt(self);
    }
}

fn factor_weaknesses(
    ranking: &PersistedFactorRanking,
    regime_score: f64,
    sample_score: f64,
) -> Vec<String> {
    let mut weaknesses = Vec::new();
    if sample_score < 0.40 {
        weaknesses.push("insufficient_sample".to_string());
    }
    if ranking.backtest_return <= 0.0 {
        weaknesses.push("negative_or_flat_return".to_string());
    }
    if ranking.ic.abs() < 0.03 {
        weaknesses.push("weak_ic".to_string());
    }
    if ranking.ir < 0.5 {
        weaknesses.push("weak_ir".to_string());
    }
    if ranking.sharpe < 0.5 {
        weaknesses.push("low_sharpe".to_string());
    }
    if ranking.stability < 0.45 {
        weaknesses.push("unstable_walk_forward".to_string());
    }
    if ranking.win_rate < 0.48 {
        weaknesses.push("low_win_rate".to_string());
    }
    if ranking.profit_factor < 1.05 {
        weaknesses.push("weak_profit_factor".to_string());
    }
    if ranking.conformal_coverage_1sigma < 0.55 {
        weaknesses.push("low_conformal_coverage".to_string());
    }
    if ranking.regime_break_penalty > 0.20 {
        weaknesses.push("high_regime_break_penalty".to_string());
    }
    if regime_score < 0.34 {
        weaknesses.push("narrow_regime_coverage".to_string());
    }
    weaknesses
}

fn factor_grade(score: f64) -> &'static str {
    if score >= 0.85 {
        "A"
    } else if score >= 0.70 {
        "B"
    } else if score >= 0.55 {
        "C"
    } else if score >= 0.40 {
        "D"
    } else {
        "F"
    }
}

fn factor_iteration_action(ranking: &PersistedFactorRanking, sample_score: f64) -> &'static str {
    if sample_score < 0.40 || ranking.trade_count < 8 {
        "observe"
    } else if ranking.composite_score >= 0.75 && ranking.weaknesses.len() <= 2 {
        "keep"
    } else if ranking.composite_score >= 0.45 {
        "tune"
    } else {
        "replace"
    }
}

fn build_agent_prompt(ranking: &PersistedFactorRanking) -> String {
    let weaknesses = if ranking.weaknesses.is_empty() {
        "none".to_string()
    } else {
        ranking.weaknesses.join(", ")
    };

    match ranking.iteration_action.as_str() {
        "keep" => format!(
            "Keep factor '{}' as the benchmark. score={:.2} grade={}. Only promote a new variant if composite score improves by >=0.05 without reducing stability below {:.2} or profit_factor below {:.2}.",
            ranking.factor_name,
            ranking.composite_score,
            ranking.grade,
            ranking.stability.max(0.50),
            ranking.profit_factor.max(1.05)
        ),
        "tune" => format!(
            "Tune factor '{}'. score={:.2} grade={}. Weaknesses: {}. Agent should iterate parameters/evidence mapping and only accept a variant if composite score improves by >=0.10 and walk-forward stability does not fall.",
            ranking.factor_name,
            ranking.composite_score,
            ranking.grade,
            weaknesses
        ),
        "replace" => format!(
            "Replace factor '{}'. score={:.2} grade={}. Weaknesses: {}. Agent should design a new factor for this slot, benchmark against the current factor, and only promote it if composite score improves by >=0.15 with trade_count >= 8 and profit_factor >= 1.10.",
            ranking.factor_name,
            ranking.composite_score,
            ranking.grade,
            weaknesses
        ),
        _ => format!(
            "Observe factor '{}'. score={:.2} grade={}. Weaknesses: {}. Agent should gather more data or expand sample coverage before replacing it.",
            ranking.factor_name,
            ranking.composite_score,
            ranking.grade,
            weaknesses
        ),
    }
}

fn iteration_priority(action: &str) -> u8 {
    match action {
        "replace" => 3,
        "tune" => 2,
        "observe" => 1,
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy)]
enum StructuralPriorEntityKind {
    Node,
    Branch,
    Scenario,
    Path,
}

fn structural_prior_entity_mass_scale(kind: StructuralPriorEntityKind) -> f64 {
    match kind {
        StructuralPriorEntityKind::Node => 0.50,
        StructuralPriorEntityKind::Branch => 0.75,
        StructuralPriorEntityKind::Scenario => 0.90,
        StructuralPriorEntityKind::Path => 1.0,
    }
}

fn structural_prior_entity_kind_label(kind: StructuralPriorEntityKind) -> &'static str {
    match kind {
        StructuralPriorEntityKind::Node => "node",
        StructuralPriorEntityKind::Branch => "branch",
        StructuralPriorEntityKind::Scenario => "scenario",
        StructuralPriorEntityKind::Path => "path",
    }
}

fn structural_prior_mass_snapshot(
    entity_id: &str,
    kind: StructuralPriorEntityKind,
    stats: &StructuralPriorStats,
) -> StructuralPriorMassSnapshot {
    StructuralPriorMassSnapshot {
        entity_id: entity_id.to_string(),
        entity_kind: structural_prior_entity_kind_label(kind).to_string(),
        observations: stats.observations,
        followed_count: stats.followed_count,
        weighted_followed_mass: stats.weighted_followed_mass,
        weighted_success_mass: stats.weighted_success_mass,
        weighted_failure_mass: stats.weighted_failure_mass,
        weighted_invalidation_mass: stats.weighted_invalidation_mass,
        weighted_exposure_mass: stats.weighted_exposure_mass,
        weighted_not_followed_mass: stats.weighted_not_followed_mass,
        smoothed_prior: stats.smoothed_prior,
        execution_propensity: stats.execution_propensity,
        off_policy_adjusted_prior: stats.off_policy_adjusted_prior,
        behavior_policy_probability: stats.behavior_policy_probability,
        behavior_policy_probability_squared_mass: stats.behavior_policy_probability_squared_mass,
        behavior_policy_probability_variance: stats.behavior_policy_probability_variance,
        target_policy_probability_confidence: stats.target_policy_probability_confidence,
        target_policy_probability_lower_bound: stats.target_policy_probability_lower_bound,
        target_policy_probability_brier_score: stats.target_policy_probability_brier_score,
        target_policy_probability_calibration_error: stats
            .target_policy_probability_calibration_error,
        snips_reward_prior: stats.snips_reward_prior,
        doubly_robust_reward_prior: stats.doubly_robust_reward_prior,
        target_policy_calibration_weight: stats.target_policy_calibration_weight,
        target_policy_reward_prior: stats.target_policy_reward_prior,
        target_policy_variance_penalty: stats.target_policy_variance_penalty,
        target_policy_reward_lower_bound: stats.target_policy_reward_lower_bound,
        delayed_reward_resolution_probability: stats.delayed_reward_resolution_probability,
        delayed_reward_censoring_probability: stats.delayed_reward_censoring_probability,
        censoring_adjusted_reward_prior: stats.censoring_adjusted_reward_prior,
        censoring_adjusted_reward_lower_bound: stats.censoring_adjusted_reward_lower_bound,
        delayed_reward_success_competing_risk: stats.delayed_reward_success_competing_risk,
        delayed_reward_failure_competing_risk: stats.delayed_reward_failure_competing_risk,
        delayed_reward_invalidation_competing_risk: stats
            .delayed_reward_invalidation_competing_risk,
        delayed_reward_abandonment_competing_risk: stats.delayed_reward_abandonment_competing_risk,
        delayed_reward_competing_risk_entropy: stats.delayed_reward_competing_risk_entropy,
        delayed_reward_elapsed_feedback_count: stats.delayed_reward_elapsed_feedback_count,
        delayed_reward_elapsed_hours_at_risk: stats.delayed_reward_elapsed_hours_at_risk,
        delayed_reward_avg_elapsed_hours: stats.delayed_reward_avg_elapsed_hours,
        delayed_reward_resolution_hazard_per_hour: stats.delayed_reward_resolution_hazard_per_hour,
        delayed_reward_expected_resolution_hours: stats.delayed_reward_expected_resolution_hours,
        delayed_reward_survival_probability_1h: stats.delayed_reward_survival_probability_1h,
        delayed_reward_survival_probability_4h: stats.delayed_reward_survival_probability_4h,
        delayed_reward_survival_probability_24h: stats.delayed_reward_survival_probability_24h,
        delayed_reward_success_hazard_per_hour: stats.delayed_reward_success_hazard_per_hour,
        delayed_reward_failure_hazard_per_hour: stats.delayed_reward_failure_hazard_per_hour,
        delayed_reward_invalidation_hazard_per_hour: stats
            .delayed_reward_invalidation_hazard_per_hour,
        delayed_reward_abandonment_hazard_per_hour: stats
            .delayed_reward_abandonment_hazard_per_hour,
        delayed_reward_success_cumulative_incidence_4h: stats
            .delayed_reward_success_cumulative_incidence_4h,
        delayed_reward_failure_cumulative_incidence_4h: stats
            .delayed_reward_failure_cumulative_incidence_4h,
        delayed_reward_invalidation_cumulative_incidence_4h: stats
            .delayed_reward_invalidation_cumulative_incidence_4h,
        delayed_reward_abandonment_cumulative_incidence_4h: stats
            .delayed_reward_abandonment_cumulative_incidence_4h,
        delayed_reward_resolution_horizon_1h_count: stats
            .delayed_reward_resolution_horizon_1h_count,
        delayed_reward_resolution_within_1h_count: stats.delayed_reward_resolution_within_1h_count,
        delayed_reward_resolution_probability_1h: stats.delayed_reward_resolution_probability_1h,
        delayed_reward_resolution_horizon_4h_count: stats
            .delayed_reward_resolution_horizon_4h_count,
        delayed_reward_resolution_within_4h_count: stats.delayed_reward_resolution_within_4h_count,
        delayed_reward_resolution_probability_4h: stats.delayed_reward_resolution_probability_4h,
        delayed_reward_resolution_horizon_24h_count: stats
            .delayed_reward_resolution_horizon_24h_count,
        delayed_reward_resolution_within_24h_count: stats
            .delayed_reward_resolution_within_24h_count,
        delayed_reward_resolution_probability_24h: stats.delayed_reward_resolution_probability_24h,
        last_offline_seed_source: stats.last_offline_seed_source.clone(),
    }
}

fn refresh_structural_prior_mass_snapshots_for_refs(
    state: &mut StructuralPriorLearningState,
    refs: &StructuralFeedbackRefs,
) {
    if let Some(snapshot) = state.nodes.get(&refs.node_id).map(|stats| {
        structural_prior_mass_snapshot(&refs.node_id, StructuralPriorEntityKind::Node, stats)
    }) {
        state.node_prior_mass.insert(refs.node_id.clone(), snapshot);
    }
    if let Some(snapshot) = state.branches.get(&refs.branch_id).map(|stats| {
        structural_prior_mass_snapshot(&refs.branch_id, StructuralPriorEntityKind::Branch, stats)
    }) {
        state
            .branch_prior_mass
            .insert(refs.branch_id.clone(), snapshot);
    }
    if let Some(snapshot) = state.scenarios.get(&refs.scenario_id).map(|stats| {
        structural_prior_mass_snapshot(
            &refs.scenario_id,
            StructuralPriorEntityKind::Scenario,
            stats,
        )
    }) {
        state
            .scenario_prior_mass
            .insert(refs.scenario_id.clone(), snapshot);
    }
    if let Some(snapshot) = state.paths.get(&refs.path_id).map(|stats| {
        structural_prior_mass_snapshot(&refs.path_id, StructuralPriorEntityKind::Path, stats)
    }) {
        state.path_prior_mass.insert(refs.path_id.clone(), snapshot);
    }
}

fn update_structural_prior_stats(
    stats: &mut StructuralPriorStats,
    record: &FeedbackRecord,
    followed_path: bool,
    kind: StructuralPriorEntityKind,
) {
    let source_weight =
        structural_prior_source_weight(&record.source) * structural_prior_entity_mass_scale(kind);
    let not_followed_path = !followed_path
        || record
            .realized_outcome
            .trim()
            .eq_ignore_ascii_case("not_followed");
    stats.observations += 1;
    stats.weighted_exposure_mass += source_weight;
    if stats.observations == 1 {
        stats.avg_pnl = record.pnl;
    } else {
        let previous = stats.observations - 1;
        stats.avg_pnl =
            ((stats.avg_pnl * previous as f64) + record.pnl) / stats.observations as f64;
    }
    if followed_path {
        stats.followed_count += 1;
    }
    if not_followed_path {
        stats.not_followed += 1;
        stats.weighted_not_followed_mass += source_weight;
    }
    accumulate_structural_prior_stats_delayed_reward_observation(stats, record, followed_path);
    if let Some(pseudo_counts) = structural_feedback_pseudo_counts(record, followed_path) {
        let mass_update = weighted_success_credit_beta_update(
            pseudo_counts.success_credit,
            source_weight,
            pseudo_counts.observation_weight,
            1.0,
        );
        let weighted_observation = mass_update.observation_mass;
        mass_update.apply_to(
            &mut stats.weighted_followed_mass,
            &mut stats.weighted_success_mass,
            &mut stats.weighted_failure_mass,
        );
        if matches!(
            structural_feedback_counter_outcome(record),
            Some("invalidated")
        ) {
            stats.weighted_invalidation_mass += weighted_observation;
        }
        if let Some(contribution) =
            structural_policy_correction_contribution(record, pseudo_counts, weighted_observation)
        {
            update_structural_policy_correction_stats(
                StructuralPolicyCorrectionStatsAccumulator {
                    policy_weighted_observation_mass: &mut stats.policy_weighted_observation_mass,
                    behavior_policy_probability: &mut stats.behavior_policy_probability,
                    behavior_policy_probability_squared_mass: &mut stats
                        .behavior_policy_probability_squared_mass,
                    target_policy_probability_brier_score_mass: &mut stats
                        .target_policy_probability_brier_score_mass,
                    target_policy_probability_calibration_error_mass: &mut stats
                        .target_policy_probability_calibration_error_mass,
                    snips_weight_mass: &mut stats.snips_weight_mass,
                    snips_weight_squared_mass: &mut stats.snips_weight_squared_mass,
                    snips_reward_mass: &mut stats.snips_reward_mass,
                    doubly_robust_reward_mass: &mut stats.doubly_robust_reward_mass,
                },
                contribution,
            );
        }
    }
    let source_summary = stats
        .source_panel_summaries
        .entry(record.source.clone())
        .or_default();
    update_structural_prior_source_summary_from_feedback(
        source_summary,
        record,
        followed_path,
        kind,
    );
    refresh_structural_smoothed_prior(stats);
}

fn apply_structural_prior_seed_to_stats(
    stats: &mut StructuralPriorStats,
    refs: &StructuralFeedbackRefs,
    seed: &StructuralPriorSeed,
    kind: StructuralPriorEntityKind,
) {
    if seed.observations == 0 {
        return;
    }
    let source_weight =
        structural_prior_seed_effective_weight(seed) * structural_prior_entity_mass_scale(kind);
    let previous_observations = stats.observations;
    stats.observations += seed.observations;
    stats.followed_count += seed.followed_count;
    stats.wins += seed.wins;
    stats.losses += seed.losses;
    stats.breakevens += seed.breakevens;
    stats.invalidated += seed.invalidated;
    stats.abandoned += seed.abandoned;
    stats.not_followed += seed.not_followed;
    let mass_update = weighted_seed_beta_update(WeightedSeedBetaUpdateInput {
        followed_observation_count: seed.followed_count,
        wins: seed.wins,
        losses: seed.losses,
        breakevens: seed.breakevens,
        invalidated: seed.invalidated,
        abandoned: seed.abandoned,
        source_weight,
        quality_weight: 1.0,
        recency_weight: 1.0,
    });
    mass_update.apply_to(
        &mut stats.weighted_followed_mass,
        &mut stats.weighted_success_mass,
        &mut stats.weighted_failure_mass,
    );
    stats.weighted_invalidation_mass += source_weight * seed.invalidated as f64;
    stats.weighted_exposure_mass += source_weight * seed.observations as f64;
    stats.weighted_not_followed_mass += source_weight * seed.not_followed as f64;
    let new_total = previous_observations + seed.observations;
    stats.avg_pnl = if new_total == 0 {
        0.0
    } else {
        ((stats.avg_pnl * previous_observations as f64) + (seed.avg_pnl * seed.observations as f64))
            / new_total as f64
    };
    stats.last_offline_seed_source = Some(seed.source_label.clone());
    let source_summary = stats
        .source_panel_summaries
        .entry(seed.source_label.clone())
        .or_default();
    apply_structural_prior_seed_to_source_summary(source_summary, refs, seed, kind);
    refresh_structural_smoothed_prior(stats);
}

pub(crate) fn structural_prior_source_weight(source: &str) -> f64 {
    match source.trim() {
        "structural_feedback_submission" | "update_structural_feedback" | "live_feedback" => 1.0,
        "artifact_validation" => 0.90,
        "backtest" | "backtest_run_structural_prior_seed" => 0.75,
        "research" | "research_run_structural_prior_seed" => 0.55,
        "factor_mutation_structural_prior_seed" | "factor_mutation" => 0.40,
        "analyze" | "analyze_run_structural_prior_seed" => 0.30,
        _ => 0.50,
    }
}

fn structural_prior_seed_tempering_coefficient(seed: &StructuralPriorSeed) -> f64 {
    seed.tempering_coefficient.unwrap_or(1.0).clamp(0.0, 1.0)
}

fn structural_prior_seed_effective_weight(seed: &StructuralPriorSeed) -> f64 {
    beta_update_factor(
        structural_prior_source_weight(&seed.source_label),
        structural_prior_seed_tempering_coefficient(seed),
        1.0,
    )
    .clamp(0.0, 1.0)
}

fn structural_power_prior_contribution(
    seed: &StructuralPriorSeed,
    kind: StructuralPriorEntityKind,
) -> StructuralPowerPriorContribution {
    structural_power_prior_contribution_with_entity_scale(
        seed,
        structural_prior_entity_mass_scale(kind),
    )
}

fn structural_power_prior_contribution_with_entity_scale(
    seed: &StructuralPriorSeed,
    entity_mass_scale: f64,
) -> StructuralPowerPriorContribution {
    let base_source_weight = structural_prior_source_weight(&seed.source_label);
    let tempering_coefficient = structural_prior_seed_tempering_coefficient(seed);
    let effective_tau =
        beta_update_factor(base_source_weight, tempering_coefficient, entity_mass_scale);
    let mass_update = weighted_seed_beta_update(WeightedSeedBetaUpdateInput {
        followed_observation_count: seed.followed_count,
        wins: seed.wins,
        losses: seed.losses,
        breakevens: seed.breakevens,
        invalidated: seed.invalidated,
        abandoned: seed.abandoned,
        source_weight: base_source_weight,
        quality_weight: tempering_coefficient,
        recency_weight: entity_mass_scale,
    });
    StructuralPowerPriorContribution {
        source_label: seed.source_label.clone(),
        base_source_weight,
        tempering_coefficient,
        entity_mass_scale,
        effective_tau,
        observation_mass: effective_tau * seed.observations as f64,
        success_mass: mass_update.success_mass,
        failure_mass: mass_update.failure_mass,
        invalidation_mass: effective_tau * seed.invalidated as f64,
        not_followed_mass: effective_tau * seed.not_followed as f64,
    }
}

fn structural_offline_seed_snapshot(
    refs: &StructuralFeedbackRefs,
    seed: &StructuralPriorSeed,
) -> StructuralOfflineSeedSnapshot {
    StructuralOfflineSeedSnapshot {
        source_label: seed.source_label.clone(),
        recommendation_id: refs.recommendation_id.clone(),
        recommended_at: refs.recommended_at.clone(),
        node_id: refs.node_id.clone(),
        branch_id: refs.branch_id.clone(),
        scenario_id: refs.scenario_id.clone(),
        path_id: refs.path_id.clone(),
        followed_path: refs.followed_path,
        observations: seed.observations,
        followed_count: seed.followed_count,
        wins: seed.wins,
        losses: seed.losses,
        breakevens: seed.breakevens,
        invalidated: seed.invalidated,
        abandoned: seed.abandoned,
        not_followed: seed.not_followed,
        avg_pnl: seed.avg_pnl,
        node_contribution: structural_power_prior_contribution(
            seed,
            StructuralPriorEntityKind::Node,
        ),
        branch_contribution: structural_power_prior_contribution(
            seed,
            StructuralPriorEntityKind::Branch,
        ),
        scenario_contribution: structural_power_prior_contribution(
            seed,
            StructuralPriorEntityKind::Scenario,
        ),
        path_contribution: structural_power_prior_contribution(
            seed,
            StructuralPriorEntityKind::Path,
        ),
    }
}

fn structural_followed_exposure_mass(
    weighted_exposure_mass: f64,
    weighted_not_followed_mass: f64,
    followed_count: usize,
) -> f64 {
    if weighted_exposure_mass > f64::EPSILON {
        (weighted_exposure_mass - weighted_not_followed_mass.max(0.0)).max(0.0)
    } else {
        followed_count as f64
    }
}

fn structural_not_followed_exposure_mass(
    weighted_exposure_mass: f64,
    weighted_not_followed_mass: f64,
    not_followed: usize,
) -> f64 {
    if weighted_exposure_mass > f64::EPSILON {
        weighted_not_followed_mass.max(0.0)
    } else {
        not_followed as f64
    }
}

fn structural_propensity_estimate(followed_exposure_mass: f64, not_followed_mass: f64) -> f64 {
    let followed = followed_exposure_mass.max(0.0);
    let not_followed = not_followed_mass.max(0.0);
    let exposure = followed + not_followed;
    if exposure <= f64::EPSILON {
        0.5
    } else {
        ((1.0 + followed) / (2.0 + exposure)).clamp(0.0, 1.0)
    }
}

fn structural_ips_weight(execution_propensity: f64) -> f64 {
    if execution_propensity <= f64::EPSILON {
        0.0
    } else {
        (1.0 / execution_propensity).min(STRUCTURAL_IPS_WEIGHT_CLIP)
    }
}

#[derive(Debug, Clone, Copy)]
struct StructuralPolicyCorrectionContribution {
    weighted_observation: f64,
    behavior_policy_probability: f64,
    behavior_policy_probability_squared_mass: f64,
    target_policy_probability_brier_score_mass: f64,
    target_policy_probability_calibration_error_mass: f64,
    snips_weighted_mass: f64,
    snips_weighted_squared_mass: f64,
    snips_reward_mass: f64,
    doubly_robust_reward: f64,
}

fn structural_feedback_behavior_policy_probability(record: &FeedbackRecord) -> Option<f64> {
    let probability = record
        .model_probabilities_before_trade
        .selected_probability
        .clamp(0.0, 1.0);
    (probability > f64::EPSILON).then_some(probability)
}

fn structural_feedback_reward_baseline(record: &FeedbackRecord) -> f64 {
    let probabilities = &record.model_probabilities_before_trade;
    match probabilities.selected_direction {
        Direction::Bull => probabilities.win_prob_long,
        Direction::Bear => probabilities.win_prob_short,
        Direction::Neutral => probabilities.selected_probability,
    }
    .clamp(0.0, 1.0)
}

fn structural_policy_correction_contribution(
    record: &FeedbackRecord,
    pseudo_counts: StructuralFeedbackPseudoCounts,
    weighted_observation: f64,
) -> Option<StructuralPolicyCorrectionContribution> {
    if weighted_observation <= f64::EPSILON {
        return None;
    }
    let behavior_policy_probability = structural_feedback_behavior_policy_probability(record)?;
    let snips_weight = structural_ips_weight(behavior_policy_probability);
    if snips_weight <= f64::EPSILON {
        return None;
    }
    let reward = pseudo_counts.success_credit.clamp(0.0, 1.0);
    let baseline = structural_feedback_reward_baseline(record);
    let doubly_robust_reward = (baseline + snips_weight * (reward - baseline)).clamp(0.0, 1.0);
    let snips_weighted_mass = weighted_observation * snips_weight;
    Some(StructuralPolicyCorrectionContribution {
        weighted_observation,
        behavior_policy_probability,
        behavior_policy_probability_squared_mass: weighted_observation
            * behavior_policy_probability
            * behavior_policy_probability,
        target_policy_probability_brier_score_mass: weighted_observation
            * (behavior_policy_probability - reward).powi(2),
        target_policy_probability_calibration_error_mass: weighted_observation
            * (behavior_policy_probability - reward).abs(),
        snips_weighted_mass,
        snips_weighted_squared_mass: snips_weighted_mass * snips_weighted_mass,
        snips_reward_mass: snips_weighted_mass * reward,
        doubly_robust_reward,
    })
}

struct StructuralPolicyCorrectionStatsAccumulator<'a> {
    policy_weighted_observation_mass: &'a mut f64,
    behavior_policy_probability: &'a mut f64,
    behavior_policy_probability_squared_mass: &'a mut f64,
    target_policy_probability_brier_score_mass: &'a mut f64,
    target_policy_probability_calibration_error_mass: &'a mut f64,
    snips_weight_mass: &'a mut f64,
    snips_weight_squared_mass: &'a mut f64,
    snips_reward_mass: &'a mut f64,
    doubly_robust_reward_mass: &'a mut f64,
}

fn update_structural_policy_correction_stats(
    stats: StructuralPolicyCorrectionStatsAccumulator<'_>,
    contribution: StructuralPolicyCorrectionContribution,
) {
    let StructuralPolicyCorrectionStatsAccumulator {
        policy_weighted_observation_mass,
        behavior_policy_probability,
        behavior_policy_probability_squared_mass,
        target_policy_probability_brier_score_mass,
        target_policy_probability_calibration_error_mass,
        snips_weight_mass,
        snips_weight_squared_mass,
        snips_reward_mass,
        doubly_robust_reward_mass,
    } = stats;
    let previous_mass = (*policy_weighted_observation_mass).max(0.0);
    let next_mass = previous_mass + contribution.weighted_observation;
    if next_mass > f64::EPSILON {
        *behavior_policy_probability = ((*behavior_policy_probability * previous_mass)
            + contribution.behavior_policy_probability * contribution.weighted_observation)
            / next_mass;
    }
    *policy_weighted_observation_mass = next_mass;
    *behavior_policy_probability_squared_mass +=
        contribution.behavior_policy_probability_squared_mass;
    *target_policy_probability_brier_score_mass +=
        contribution.target_policy_probability_brier_score_mass;
    *target_policy_probability_calibration_error_mass +=
        contribution.target_policy_probability_calibration_error_mass;
    *snips_weight_mass += contribution.snips_weighted_mass;
    *snips_weight_squared_mass += contribution.snips_weighted_squared_mass;
    *snips_reward_mass += contribution.snips_reward_mass;
    *doubly_robust_reward_mass +=
        contribution.weighted_observation * contribution.doubly_robust_reward;
}

fn structural_target_policy_context_key(record: &FeedbackRecord) -> String {
    let symbol = record.symbol.trim();
    let symbol = if symbol.is_empty() { "unknown" } else { symbol };
    format!(
        "{}:{}:{}",
        symbol,
        structural_regime_context_label(record.regime_at_entry),
        structural_direction_context_label(
            record.model_probabilities_before_trade.selected_direction,
        )
    )
}

fn structural_regime_context_label(regime: Regime) -> &'static str {
    match regime {
        Regime::Accumulation => "accumulation",
        Regime::ManipulationExpansion => "manipulation_expansion",
        Regime::Distribution => "distribution",
    }
}

fn structural_direction_context_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Bull => "bull",
        Direction::Bear => "bear",
        Direction::Neutral => "neutral",
    }
}

fn update_structural_target_policy_context_posterior(
    state: &mut StructuralPriorLearningState,
    record: &FeedbackRecord,
    followed_path: bool,
) {
    let Some(pseudo_counts) = structural_feedback_pseudo_counts(record, followed_path) else {
        return;
    };
    let mass_update = weighted_success_credit_beta_update(
        pseudo_counts.success_credit,
        structural_prior_source_weight(&record.source),
        pseudo_counts.observation_weight,
        1.0,
    );
    let weighted_observation = mass_update.observation_mass;
    let Some(contribution) =
        structural_policy_correction_contribution(record, pseudo_counts, weighted_observation)
    else {
        return;
    };
    let key = structural_target_policy_context_key(record);
    let posterior = state
        .target_policy_context_posteriors
        .entry(key)
        .or_default();
    let previous_mass = posterior.weighted_observation_mass.max(0.0);
    let next_mass = previous_mass + contribution.weighted_observation;
    if next_mass > f64::EPSILON {
        posterior.behavior_policy_probability = ((posterior.behavior_policy_probability
            * previous_mass)
            + contribution.behavior_policy_probability * contribution.weighted_observation)
            / next_mass;
    }
    posterior.observations += 1;
    posterior.weighted_observation_mass = next_mass;
    posterior.success_mass += mass_update.success_mass;
    posterior.failure_mass += mass_update.failure_mass;
    posterior.behavior_policy_probability_squared_mass +=
        contribution.behavior_policy_probability_squared_mass;
    posterior.target_policy_probability_brier_score_mass +=
        contribution.target_policy_probability_brier_score_mass;
    posterior.target_policy_probability_calibration_error_mass +=
        contribution.target_policy_probability_calibration_error_mass;
    posterior.last_updated_at = Some(record.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true));
    posterior.last_recommendation_id = record
        .structural_feedback
        .as_ref()
        .map(|refs| refs.recommendation_id.clone());
    refresh_structural_target_policy_context_posterior(posterior);
}

fn refresh_structural_target_policy_context_posterior(
    posterior: &mut StructuralTargetPolicyContextPosterior,
) {
    posterior.behavior_policy_probability = posterior.behavior_policy_probability.clamp(0.0, 1.0);
    posterior.behavior_policy_probability_variance =
        structural_behavior_policy_probability_variance(
            posterior.weighted_observation_mass,
            posterior.behavior_policy_probability,
            posterior.behavior_policy_probability_squared_mass,
        );
    posterior.target_policy_probability_brier_score = structural_policy_probability_average_error(
        posterior.weighted_observation_mass,
        posterior.target_policy_probability_brier_score_mass,
    );
    posterior.target_policy_probability_calibration_error =
        structural_policy_probability_average_error(
            posterior.weighted_observation_mass,
            posterior.target_policy_probability_calibration_error_mass,
        );
    posterior.learned_target_policy_probability =
        structural_beta_mean(posterior.success_mass, posterior.failure_mass);
    posterior.learned_target_policy_probability_lower_bound =
        structural_learned_target_policy_probability_lower_bound(
            posterior.learned_target_policy_probability,
            posterior.weighted_observation_mass,
        );
    posterior.learned_target_policy_probability_confidence =
        structural_learned_target_policy_probability_confidence(
            posterior.weighted_observation_mass,
            posterior.target_policy_probability_brier_score,
        );
    posterior.calibrated_target_policy_probability =
        structural_calibrated_target_policy_probability(
            posterior.behavior_policy_probability,
            posterior.learned_target_policy_probability,
            posterior.learned_target_policy_probability_confidence,
        );
    posterior.calibrated_target_policy_probability_lower_bound =
        structural_calibrated_target_policy_probability_lower_bound(
            posterior.behavior_policy_probability,
            posterior.behavior_policy_probability_variance,
            posterior.weighted_observation_mass,
            posterior.learned_target_policy_probability_lower_bound,
            posterior.learned_target_policy_probability_confidence,
        );
}

fn structural_learned_target_policy_probability_lower_bound(probability: f64, mass: f64) -> f64 {
    if mass <= f64::EPSILON {
        return 0.0;
    }
    let probability = probability.clamp(0.0, 1.0);
    let penalty = ((probability * (1.0 - probability)) / (1.0 + mass.max(0.0)))
        .sqrt()
        .clamp(0.0, 1.0);
    (probability - penalty).clamp(0.0, 1.0)
}

fn structural_learned_target_policy_probability_confidence(mass: f64, brier_score: f64) -> f64 {
    if mass <= f64::EPSILON {
        return 0.0;
    }
    let mass_confidence = (mass / (1.0 + mass)).clamp(0.0, 1.0);
    let calibration_confidence = (1.0 - brier_score.clamp(0.0, 1.0).sqrt()).clamp(0.0, 1.0);
    (mass_confidence * calibration_confidence).clamp(0.0, 1.0)
}

fn structural_calibrated_target_policy_probability(
    behavior_policy_probability: f64,
    learned_target_policy_probability: f64,
    learned_target_policy_probability_confidence: f64,
) -> f64 {
    let confidence = learned_target_policy_probability_confidence.clamp(0.0, 1.0);
    (learned_target_policy_probability.clamp(0.0, 1.0) * confidence
        + behavior_policy_probability.clamp(0.0, 1.0) * (1.0 - confidence))
        .clamp(0.0, 1.0)
}

fn structural_calibrated_target_policy_probability_lower_bound(
    behavior_policy_probability: f64,
    behavior_policy_probability_variance: f64,
    weighted_observation_mass: f64,
    learned_target_policy_probability_lower_bound: f64,
    learned_target_policy_probability_confidence: f64,
) -> f64 {
    let behavior_lower_bound = structural_target_policy_probability_lower_bound(
        behavior_policy_probability,
        weighted_observation_mass,
        behavior_policy_probability_variance,
    );
    structural_calibrated_target_policy_probability(
        behavior_lower_bound,
        learned_target_policy_probability_lower_bound,
        learned_target_policy_probability_confidence,
    )
}

fn structural_snips_effective_sample_size(
    snips_weight_mass: f64,
    snips_weight_squared_mass: f64,
) -> f64 {
    if snips_weight_mass <= f64::EPSILON || snips_weight_squared_mass <= f64::EPSILON {
        0.0
    } else {
        ((snips_weight_mass * snips_weight_mass) / snips_weight_squared_mass).max(0.0)
    }
}

fn structural_target_policy_calibration_weight(snips_effective_sample_size: f64) -> f64 {
    let effective_sample_size = snips_effective_sample_size.max(0.0);
    if effective_sample_size <= f64::EPSILON {
        0.0
    } else {
        (effective_sample_size / (1.0 + effective_sample_size)).clamp(0.0, 1.0)
    }
}

fn structural_behavior_policy_probability_variance(
    policy_weighted_observation_mass: f64,
    behavior_policy_probability: f64,
    behavior_policy_probability_squared_mass: f64,
) -> f64 {
    if policy_weighted_observation_mass <= f64::EPSILON {
        return 0.0;
    }
    let mean = behavior_policy_probability.clamp(0.0, 1.0);
    let mean_square = (behavior_policy_probability_squared_mass.max(0.0)
        / policy_weighted_observation_mass)
        .clamp(0.0, 1.0);
    (mean_square - mean * mean).max(0.0).clamp(0.0, 1.0)
}

fn structural_target_policy_probability_penalty(
    policy_weighted_observation_mass: f64,
    behavior_policy_probability_variance: f64,
) -> f64 {
    if policy_weighted_observation_mass <= f64::EPSILON {
        return 0.0;
    }
    (behavior_policy_probability_variance.clamp(0.0, 1.0)
        / (1.0 + policy_weighted_observation_mass.max(0.0)))
    .sqrt()
    .clamp(0.0, 1.0)
}

fn structural_target_policy_probability_confidence(
    policy_weighted_observation_mass: f64,
    behavior_policy_probability_variance: f64,
) -> f64 {
    if policy_weighted_observation_mass <= f64::EPSILON {
        return 0.0;
    }
    let mass_confidence = (policy_weighted_observation_mass
        / (1.0 + policy_weighted_observation_mass))
        .clamp(0.0, 1.0);
    let dispersion_confidence =
        (1.0 - behavior_policy_probability_variance.clamp(0.0, 1.0).sqrt()).clamp(0.0, 1.0);
    (mass_confidence * dispersion_confidence).clamp(0.0, 1.0)
}

fn structural_target_policy_probability_lower_bound(
    behavior_policy_probability: f64,
    policy_weighted_observation_mass: f64,
    behavior_policy_probability_variance: f64,
) -> f64 {
    let penalty = structural_target_policy_probability_penalty(
        policy_weighted_observation_mass,
        behavior_policy_probability_variance,
    );
    (behavior_policy_probability.clamp(0.0, 1.0) - penalty).clamp(0.0, 1.0)
}

fn structural_policy_probability_average_error(
    policy_weighted_observation_mass: f64,
    error_mass: f64,
) -> f64 {
    if policy_weighted_observation_mass <= f64::EPSILON {
        0.0
    } else {
        (error_mass.max(0.0) / policy_weighted_observation_mass).clamp(0.0, 1.0)
    }
}

fn structural_target_policy_variance_penalty(
    reward_prior: f64,
    snips_effective_sample_size: f64,
) -> f64 {
    if snips_effective_sample_size <= f64::EPSILON {
        return 0.0;
    }
    let reward_prior = reward_prior.clamp(0.0, 1.0);
    ((reward_prior * (1.0 - reward_prior)) / (1.0 + snips_effective_sample_size))
        .sqrt()
        .clamp(0.0, 1.0)
}

fn structural_target_policy_reward_prior(
    smoothed_prior: f64,
    doubly_robust_reward_prior: f64,
    target_policy_calibration_weight: f64,
) -> f64 {
    if target_policy_calibration_weight <= f64::EPSILON {
        0.0
    } else {
        ((doubly_robust_reward_prior.clamp(0.0, 1.0) * target_policy_calibration_weight)
            + (smoothed_prior.clamp(0.0, 1.0) * (1.0 - target_policy_calibration_weight)))
            .clamp(0.0, 1.0)
    }
}

fn structural_beta_mean(success_mass: f64, failure_mass: f64) -> f64 {
    beta_posterior_mean(success_mass, failure_mass)
}

fn refresh_structural_smoothed_prior(stats: &mut StructuralPriorStats) {
    stats.smoothed_prior = if stats.weighted_followed_mass <= f64::EPSILON {
        0.5
    } else {
        structural_beta_mean(stats.weighted_success_mass, stats.weighted_failure_mass)
    };
    let followed_exposure_mass = structural_followed_exposure_mass(
        stats.weighted_exposure_mass,
        stats.weighted_not_followed_mass,
        stats.followed_count,
    );
    let not_followed_mass = structural_not_followed_exposure_mass(
        stats.weighted_exposure_mass,
        stats.weighted_not_followed_mass,
        stats.not_followed,
    );
    stats.execution_propensity =
        structural_propensity_estimate(followed_exposure_mass, not_followed_mass);
    stats.ips_weight = structural_ips_weight(stats.execution_propensity);
    stats.counterfactual_success_mass = stats.weighted_success_mass * stats.ips_weight;
    stats.counterfactual_failure_mass = stats.weighted_failure_mass * stats.ips_weight;
    stats.counterfactual_reward_prior = structural_beta_mean(
        stats.counterfactual_success_mass,
        stats.counterfactual_failure_mass,
    );
    stats.off_policy_adjusted_prior =
        (stats.counterfactual_reward_prior * stats.execution_propensity).clamp(0.0, 1.0);
    stats.behavior_policy_probability = stats.behavior_policy_probability.clamp(0.0, 1.0);
    stats.behavior_policy_probability_variance = structural_behavior_policy_probability_variance(
        stats.policy_weighted_observation_mass,
        stats.behavior_policy_probability,
        stats.behavior_policy_probability_squared_mass,
    );
    stats.target_policy_probability_confidence = structural_target_policy_probability_confidence(
        stats.policy_weighted_observation_mass,
        stats.behavior_policy_probability_variance,
    );
    stats.target_policy_probability_lower_bound = structural_target_policy_probability_lower_bound(
        stats.behavior_policy_probability,
        stats.policy_weighted_observation_mass,
        stats.behavior_policy_probability_variance,
    );
    stats.target_policy_probability_brier_score = structural_policy_probability_average_error(
        stats.policy_weighted_observation_mass,
        stats.target_policy_probability_brier_score_mass,
    );
    stats.target_policy_probability_calibration_error = structural_policy_probability_average_error(
        stats.policy_weighted_observation_mass,
        stats.target_policy_probability_calibration_error_mass,
    );
    stats.snips_reward_prior = if stats.snips_weight_mass > f64::EPSILON {
        (stats.snips_reward_mass / stats.snips_weight_mass).clamp(0.0, 1.0)
    } else {
        0.0
    };
    stats.snips_effective_sample_size = structural_snips_effective_sample_size(
        stats.snips_weight_mass,
        stats.snips_weight_squared_mass,
    );
    stats.doubly_robust_reward_prior = if stats.policy_weighted_observation_mass > f64::EPSILON {
        (stats.doubly_robust_reward_mass / stats.policy_weighted_observation_mass).clamp(0.0, 1.0)
    } else {
        0.0
    };
    stats.target_policy_calibration_weight =
        structural_target_policy_calibration_weight(stats.snips_effective_sample_size);
    stats.target_policy_reward_prior = structural_target_policy_reward_prior(
        stats.smoothed_prior,
        stats.doubly_robust_reward_prior,
        stats.target_policy_calibration_weight,
    );
    stats.target_policy_variance_penalty = structural_target_policy_variance_penalty(
        stats.target_policy_reward_prior,
        stats.snips_effective_sample_size,
    );
    stats.target_policy_reward_lower_bound =
        (stats.target_policy_reward_prior - stats.target_policy_variance_penalty).clamp(0.0, 1.0);
    refresh_structural_prior_delayed_reward_metrics(stats);
}

fn update_structural_prior_source_summary_from_feedback(
    summary: &mut StructuralPriorSourceSummary,
    record: &FeedbackRecord,
    followed_path: bool,
    kind: StructuralPriorEntityKind,
) {
    let source_weight =
        structural_prior_source_weight(&record.source) * structural_prior_entity_mass_scale(kind);
    let not_followed_path = !followed_path
        || record
            .realized_outcome
            .trim()
            .eq_ignore_ascii_case("not_followed");
    summary.last_tempering_coefficient = Some(1.0);
    summary.observations += 1;
    summary.weighted_exposure_mass += source_weight;
    if summary.observations == 1 {
        summary.avg_pnl = record.pnl;
    } else {
        let previous = summary.observations - 1;
        summary.avg_pnl =
            ((summary.avg_pnl * previous as f64) + record.pnl) / summary.observations as f64;
    }
    if followed_path {
        summary.followed_count += 1;
    }
    if not_followed_path {
        summary.not_followed += 1;
        summary.weighted_not_followed_mass += source_weight;
    }
    accumulate_structural_prior_source_summary_delayed_reward_observation(
        summary,
        record,
        followed_path,
    );
    if let Some(pseudo_counts) = structural_feedback_pseudo_counts(record, followed_path) {
        let mass_update = weighted_success_credit_beta_update(
            pseudo_counts.success_credit,
            source_weight,
            pseudo_counts.observation_weight,
            1.0,
        );
        let weighted_observation = mass_update.observation_mass;
        mass_update.apply_to(
            &mut summary.weighted_followed_mass,
            &mut summary.weighted_success_mass,
            &mut summary.weighted_failure_mass,
        );
        if matches!(
            structural_feedback_counter_outcome(record),
            Some("invalidated")
        ) {
            summary.weighted_invalidation_mass += weighted_observation;
        }
        if let Some(contribution) =
            structural_policy_correction_contribution(record, pseudo_counts, weighted_observation)
        {
            update_structural_policy_correction_stats(
                StructuralPolicyCorrectionStatsAccumulator {
                    policy_weighted_observation_mass: &mut summary.policy_weighted_observation_mass,
                    behavior_policy_probability: &mut summary.behavior_policy_probability,
                    behavior_policy_probability_squared_mass: &mut summary
                        .behavior_policy_probability_squared_mass,
                    target_policy_probability_brier_score_mass: &mut summary
                        .target_policy_probability_brier_score_mass,
                    target_policy_probability_calibration_error_mass: &mut summary
                        .target_policy_probability_calibration_error_mass,
                    snips_weight_mass: &mut summary.snips_weight_mass,
                    snips_weight_squared_mass: &mut summary.snips_weight_squared_mass,
                    snips_reward_mass: &mut summary.snips_reward_mass,
                    doubly_robust_reward_mass: &mut summary.doubly_robust_reward_mass,
                },
                contribution,
            );
        }
    }
    if let Some(refs) = record.structural_feedback.as_ref() {
        summary.last_recommendation_id = Some(refs.recommendation_id.clone());
        summary.last_recommended_at = Some(refs.recommended_at.clone());
        summary.last_note = refs.notes.clone();
    }
    refresh_structural_prior_source_summary(summary);
}

fn apply_structural_prior_seed_to_source_summary(
    summary: &mut StructuralPriorSourceSummary,
    refs: &StructuralFeedbackRefs,
    seed: &StructuralPriorSeed,
    kind: StructuralPriorEntityKind,
) {
    if seed.observations == 0 {
        return;
    }
    let source_weight =
        structural_prior_seed_effective_weight(seed) * structural_prior_entity_mass_scale(kind);
    summary.last_tempering_coefficient = Some(structural_prior_seed_tempering_coefficient(seed));
    summary.last_power_prior_contribution = Some(structural_power_prior_contribution(seed, kind));
    let previous_observations = summary.observations;
    summary.observations += seed.observations;
    summary.followed_count += seed.followed_count;
    summary.wins += seed.wins;
    summary.losses += seed.losses;
    summary.breakevens += seed.breakevens;
    summary.invalidated += seed.invalidated;
    summary.abandoned += seed.abandoned;
    summary.not_followed += seed.not_followed;
    let mass_update = weighted_seed_beta_update(WeightedSeedBetaUpdateInput {
        followed_observation_count: seed.followed_count,
        wins: seed.wins,
        losses: seed.losses,
        breakevens: seed.breakevens,
        invalidated: seed.invalidated,
        abandoned: seed.abandoned,
        source_weight,
        quality_weight: 1.0,
        recency_weight: 1.0,
    });
    mass_update.apply_to(
        &mut summary.weighted_followed_mass,
        &mut summary.weighted_success_mass,
        &mut summary.weighted_failure_mass,
    );
    summary.weighted_invalidation_mass += source_weight * seed.invalidated as f64;
    summary.weighted_exposure_mass += source_weight * seed.observations as f64;
    summary.weighted_not_followed_mass += source_weight * seed.not_followed as f64;
    let new_total = previous_observations + seed.observations;
    summary.avg_pnl = if new_total == 0 {
        0.0
    } else {
        ((summary.avg_pnl * previous_observations as f64)
            + (seed.avg_pnl * seed.observations as f64))
            / new_total as f64
    };
    summary.last_recommendation_id = Some(refs.recommendation_id.clone());
    summary.last_recommended_at = Some(refs.recommended_at.clone());
    summary.last_note = refs.notes.clone();
    refresh_structural_prior_source_summary(summary);
}

fn refresh_structural_prior_source_summary(summary: &mut StructuralPriorSourceSummary) {
    summary.smoothed_prior = if summary.weighted_followed_mass <= f64::EPSILON {
        0.5
    } else {
        structural_beta_mean(summary.weighted_success_mass, summary.weighted_failure_mass)
    };
    let followed_exposure_mass = structural_followed_exposure_mass(
        summary.weighted_exposure_mass,
        summary.weighted_not_followed_mass,
        summary.followed_count,
    );
    let not_followed_mass = structural_not_followed_exposure_mass(
        summary.weighted_exposure_mass,
        summary.weighted_not_followed_mass,
        summary.not_followed,
    );
    summary.execution_propensity =
        structural_propensity_estimate(followed_exposure_mass, not_followed_mass);
    summary.ips_weight = structural_ips_weight(summary.execution_propensity);
    summary.counterfactual_success_mass = summary.weighted_success_mass * summary.ips_weight;
    summary.counterfactual_failure_mass = summary.weighted_failure_mass * summary.ips_weight;
    summary.counterfactual_reward_prior = structural_beta_mean(
        summary.counterfactual_success_mass,
        summary.counterfactual_failure_mass,
    );
    summary.off_policy_adjusted_prior =
        (summary.counterfactual_reward_prior * summary.execution_propensity).clamp(0.0, 1.0);
    summary.behavior_policy_probability = summary.behavior_policy_probability.clamp(0.0, 1.0);
    summary.behavior_policy_probability_variance = structural_behavior_policy_probability_variance(
        summary.policy_weighted_observation_mass,
        summary.behavior_policy_probability,
        summary.behavior_policy_probability_squared_mass,
    );
    summary.target_policy_probability_confidence = structural_target_policy_probability_confidence(
        summary.policy_weighted_observation_mass,
        summary.behavior_policy_probability_variance,
    );
    summary.target_policy_probability_lower_bound =
        structural_target_policy_probability_lower_bound(
            summary.behavior_policy_probability,
            summary.policy_weighted_observation_mass,
            summary.behavior_policy_probability_variance,
        );
    summary.target_policy_probability_brier_score = structural_policy_probability_average_error(
        summary.policy_weighted_observation_mass,
        summary.target_policy_probability_brier_score_mass,
    );
    summary.target_policy_probability_calibration_error =
        structural_policy_probability_average_error(
            summary.policy_weighted_observation_mass,
            summary.target_policy_probability_calibration_error_mass,
        );
    summary.snips_reward_prior = if summary.snips_weight_mass > f64::EPSILON {
        (summary.snips_reward_mass / summary.snips_weight_mass).clamp(0.0, 1.0)
    } else {
        0.0
    };
    summary.snips_effective_sample_size = structural_snips_effective_sample_size(
        summary.snips_weight_mass,
        summary.snips_weight_squared_mass,
    );
    summary.doubly_robust_reward_prior = if summary.policy_weighted_observation_mass > f64::EPSILON
    {
        (summary.doubly_robust_reward_mass / summary.policy_weighted_observation_mass)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    summary.target_policy_calibration_weight =
        structural_target_policy_calibration_weight(summary.snips_effective_sample_size);
    summary.target_policy_reward_prior = structural_target_policy_reward_prior(
        summary.smoothed_prior,
        summary.doubly_robust_reward_prior,
        summary.target_policy_calibration_weight,
    );
    summary.target_policy_variance_penalty = structural_target_policy_variance_penalty(
        summary.target_policy_reward_prior,
        summary.snips_effective_sample_size,
    );
    summary.target_policy_reward_lower_bound = (summary.target_policy_reward_prior
        - summary.target_policy_variance_penalty)
        .clamp(0.0, 1.0);
    refresh_structural_source_summary_delayed_reward_metrics(summary);
}

fn update_structural_source_reliability_from_feedback(
    state: &mut StructuralPriorLearningState,
    record: &FeedbackRecord,
    followed_path: bool,
) {
    let source_weight = structural_prior_source_weight(&record.source);
    let not_followed_path = !followed_path
        || record
            .realized_outcome
            .trim()
            .eq_ignore_ascii_case("not_followed");
    let posterior = state
        .source_reliability_posteriors
        .entry(record.source.clone())
        .or_insert_with(|| StructuralSourceReliabilityPosterior {
            source_label: record.source.clone(),
            ..StructuralSourceReliabilityPosterior::default()
        });
    posterior.observations += 1;
    posterior.weighted_exposure_mass += source_weight;
    if not_followed_path {
        posterior.weighted_not_followed_mass += source_weight;
    }
    if let Some(pseudo_counts) = structural_feedback_pseudo_counts(record, followed_path) {
        let mass_update = weighted_success_credit_beta_update(
            pseudo_counts.success_credit,
            source_weight,
            pseudo_counts.observation_weight,
            1.0,
        );
        let weighted_observation = mass_update.observation_mass;
        mass_update.apply_to(
            &mut posterior.weighted_observation_mass,
            &mut posterior.weighted_success_mass,
            &mut posterior.weighted_failure_mass,
        );
        if matches!(
            structural_feedback_counter_outcome(record),
            Some("invalidated")
        ) {
            posterior.weighted_invalidation_mass += weighted_observation;
        }
    }
    let semantics = structural_feedback_learning_semantics(record);
    update_structural_source_outcome_confusion(
        posterior,
        &record.realized_outcome,
        &semantics.credit_class,
        source_weight * semantics.observation_weight.max(0.0),
        semantics.success_credit,
    );
    refresh_structural_source_reliability_posterior(posterior);
}

fn apply_structural_prior_seed_to_source_reliability(
    state: &mut StructuralPriorLearningState,
    seed: &StructuralPriorSeed,
) {
    if seed.observations == 0 {
        return;
    }
    let source_weight = structural_prior_seed_effective_weight(seed);
    let posterior = state
        .source_reliability_posteriors
        .entry(seed.source_label.clone())
        .or_insert_with(|| StructuralSourceReliabilityPosterior {
            source_label: seed.source_label.clone(),
            ..StructuralSourceReliabilityPosterior::default()
        });
    posterior.observations += seed.observations;
    let mass_update = weighted_seed_beta_update(WeightedSeedBetaUpdateInput {
        followed_observation_count: seed.followed_count,
        wins: seed.wins,
        losses: seed.losses,
        breakevens: seed.breakevens,
        invalidated: seed.invalidated,
        abandoned: seed.abandoned,
        source_weight,
        quality_weight: 1.0,
        recency_weight: 1.0,
    });
    mass_update.apply_to(
        &mut posterior.weighted_observation_mass,
        &mut posterior.weighted_success_mass,
        &mut posterior.weighted_failure_mass,
    );
    posterior.weighted_invalidation_mass += source_weight * seed.invalidated as f64;
    posterior.weighted_exposure_mass += source_weight * seed.observations as f64;
    posterior.weighted_not_followed_mass += source_weight * seed.not_followed as f64;
    posterior.last_tempering_coefficient = Some(structural_prior_seed_tempering_coefficient(seed));
    posterior.last_power_prior_contribution = Some(
        structural_power_prior_contribution_with_entity_scale(seed, 1.0),
    );
    update_structural_source_outcome_confusion_from_seed(posterior, seed, source_weight);
    refresh_structural_source_reliability_posterior(posterior);
}

fn update_structural_source_outcome_confusion_from_seed(
    posterior: &mut StructuralSourceReliabilityPosterior,
    seed: &StructuralPriorSeed,
    source_weight: f64,
) {
    for (observed_outcome, count, success_credit, observation_weight) in [
        ("win", seed.wins, 1.0, 1.0),
        ("loss", seed.losses, 0.0, 1.0),
        ("breakeven", seed.breakevens, 0.5, 1.0),
        ("invalidated", seed.invalidated, 0.0, 1.25),
        ("abandoned", seed.abandoned, 0.25, 0.75),
        ("not_followed", seed.not_followed, 0.0, 0.0),
    ] {
        if count == 0 {
            continue;
        }
        let credit_class = structural_source_credit_class(observed_outcome, success_credit);
        update_structural_source_outcome_confusion_with_count(
            posterior,
            observed_outcome,
            &credit_class,
            count,
            source_weight * observation_weight * count as f64,
            success_credit,
        );
    }
}

fn update_structural_source_outcome_confusion(
    posterior: &mut StructuralSourceReliabilityPosterior,
    observed_outcome: &str,
    credit_class: &str,
    weighted_observation: f64,
    success_credit: f64,
) {
    update_structural_source_outcome_confusion_with_count(
        posterior,
        observed_outcome,
        credit_class,
        1,
        weighted_observation,
        success_credit,
    );
}

fn update_structural_source_outcome_confusion_with_count(
    posterior: &mut StructuralSourceReliabilityPosterior,
    observed_outcome: &str,
    credit_class: &str,
    observations: usize,
    weighted_observation: f64,
    success_credit: f64,
) {
    let observed_outcome = structural_source_observed_outcome_label(observed_outcome);
    let credit_class = credit_class.trim().to_string();
    let key = format!("{observed_outcome}->{credit_class}");
    let entry = posterior.outcome_confusion.entry(key).or_insert_with(|| {
        StructuralSourceOutcomeConfusionCell {
            observed_outcome,
            credit_class,
            ..StructuralSourceOutcomeConfusionCell::default()
        }
    });
    entry.observations += observations;
    let weighted_observation = weighted_observation.max(0.0);
    entry.weighted_observation_mass += weighted_observation;
    entry.weighted_success_mass += weighted_observation * success_credit.clamp(0.0, 1.0);
    entry.weighted_failure_mass += weighted_observation * (1.0 - success_credit.clamp(0.0, 1.0));
}

fn structural_source_observed_outcome_label(observed_outcome: &str) -> String {
    let normalized = observed_outcome.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        "unknown".to_string()
    } else {
        normalized
    }
}

pub fn structural_source_observed_outcome_likelihood(
    posterior: &StructuralSourceReliabilityPosterior,
    credit_class: &str,
    observed_outcome: &str,
) -> f64 {
    let observed_outcome = structural_source_observed_outcome_label(observed_outcome);
    let credit_class = credit_class.trim();
    let key = format!("{observed_outcome}->{credit_class}");
    let (row_mass, row_count) = structural_source_confusion_row_stats(posterior, credit_class);
    if let Some(cell) = posterior.outcome_confusion.get(&key) {
        if row_mass <= f64::EPSILON {
            return cell
                .observed_given_credit_likelihood
                .clamp(0.0, 1.0)
                .max(0.5);
        }
        let denominator =
            row_mass + STRUCTURAL_SOURCE_CONFUSION_LAPLACE_ALPHA * row_count.max(1) as f64;
        return ((cell.weighted_observation_mass.max(0.0)
            + STRUCTURAL_SOURCE_CONFUSION_LAPLACE_ALPHA)
            / denominator)
            .clamp(0.0, 1.0);
    }

    if row_mass <= f64::EPSILON {
        return 0.5;
    }
    let denominator =
        row_mass + STRUCTURAL_SOURCE_CONFUSION_LAPLACE_ALPHA * (row_count.saturating_add(1) as f64);
    if denominator <= f64::EPSILON {
        0.5
    } else {
        (STRUCTURAL_SOURCE_CONFUSION_LAPLACE_ALPHA / denominator).clamp(0.0, 1.0)
    }
}

fn structural_source_credit_class(observed_outcome: &str, success_credit: f64) -> String {
    match observed_outcome {
        "win" => "positive_executed".to_string(),
        "loss" => "negative_executed".to_string(),
        "breakeven" => "fractional_breakeven".to_string(),
        "invalidated" => "negative_invalidated".to_string(),
        "abandoned" => "fractional_abandoned".to_string(),
        "not_followed" => "no_credit_not_followed".to_string(),
        _ if success_credit >= 0.999 => "positive_proxy".to_string(),
        _ if success_credit <= 0.001 => "negative_proxy".to_string(),
        _ => "fractional_proxy".to_string(),
    }
}

fn refresh_structural_source_reliability_posterior(
    posterior: &mut StructuralSourceReliabilityPosterior,
) {
    posterior.posterior_reliability = if posterior.weighted_observation_mass <= f64::EPSILON {
        0.5
    } else {
        structural_beta_mean(
            posterior.weighted_success_mass,
            posterior.weighted_failure_mass,
        )
    };
    refresh_structural_source_outcome_likelihoods(posterior);
}

fn refresh_structural_source_outcome_likelihoods(
    posterior: &mut StructuralSourceReliabilityPosterior,
) {
    let mut row_stats = BTreeMap::<String, (f64, usize)>::new();
    for cell in posterior.outcome_confusion.values() {
        let entry = row_stats.entry(cell.credit_class.clone()).or_default();
        entry.0 += cell.weighted_observation_mass.max(0.0);
        if cell.weighted_observation_mass > f64::EPSILON || cell.observations > 0 {
            entry.1 += 1;
        }
    }

    for cell in posterior.outcome_confusion.values_mut() {
        let (row_mass, row_count) = row_stats
            .get(&cell.credit_class)
            .copied()
            .unwrap_or((0.0, 0));
        let row_count = row_count.max(1);
        cell.credit_class_weighted_observation_mass = row_mass;
        cell.credit_class_observed_outcome_count = row_count;
        let denominator = row_mass + STRUCTURAL_SOURCE_CONFUSION_LAPLACE_ALPHA * row_count as f64;
        cell.observed_given_credit_likelihood = if denominator <= f64::EPSILON {
            1.0 / row_count as f64
        } else {
            ((cell.weighted_observation_mass.max(0.0) + STRUCTURAL_SOURCE_CONFUSION_LAPLACE_ALPHA)
                / denominator)
                .clamp(0.0, 1.0)
        };
    }
}

fn structural_source_confusion_row_stats(
    posterior: &StructuralSourceReliabilityPosterior,
    credit_class: &str,
) -> (f64, usize) {
    let mut row_mass = 0.0;
    let mut row_count = 0;
    for cell in posterior.outcome_confusion.values() {
        if cell.credit_class == credit_class {
            row_mass += cell.weighted_observation_mass.max(0.0);
            if cell.weighted_observation_mass > f64::EPSILON || cell.observations > 0 {
                row_count += 1;
            }
        }
    }
    (row_mass, row_count)
}

type StructuralSourceReliabilityEmConfusion =
    BTreeMap<String, BTreeMap<String, BTreeMap<String, f64>>>;

pub fn structural_source_reliability_em_diagnostics(
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralSourceReliabilityEmDiagnostics {
    crate::belief_core::source_reliability::structural_source_reliability_em_diagnostics(
        structural_prior_state,
    )
}

pub fn structural_source_reliability_em_fit_from_state(
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralSourceReliabilityEmFit {
    crate::belief_core::source_reliability::structural_source_reliability_em_fit_from_state(
        structural_prior_state,
    )
}

pub fn refresh_structural_source_reliability_em_state(
    structural_prior_state: &mut StructuralPriorLearningState,
) {
    crate::belief_core::source_reliability::refresh_structural_source_reliability_em_state(
        structural_prior_state,
    )
}

fn structural_prior_symbol_from_node_id(node_id: &str) -> String {
    node_id.split(':').next().unwrap_or_default().to_string()
}

fn feedback_resolution_key(record: &FeedbackRecord) -> Option<String> {
    record
        .trade_id
        .as_ref()
        .map(|trade_id| format!("trade:{trade_id}"))
        .or_else(|| {
            record.structural_feedback.as_ref().map(|refs| {
                format!(
                    "structural:{}:{}:{}",
                    refs.recommendation_id, refs.path_id, refs.followed_path
                )
            })
        })
        .or_else(|| record.run_id.as_ref().map(|run_id| format!("run:{run_id}")))
}

fn structural_prior_seed_representative_outcome(seed: &StructuralPriorSeed) -> Option<String> {
    if seed.invalidated > 0 {
        Some("invalidated".to_string())
    } else if seed.losses > 0 {
        Some("loss".to_string())
    } else if seed.abandoned > 0 {
        Some("abandoned".to_string())
    } else if seed.wins > 0 {
        Some("win".to_string())
    } else if seed.breakevens > 0 {
        Some("breakeven".to_string())
    } else if seed.not_followed > 0 {
        Some("not_followed".to_string())
    } else {
        None
    }
}

fn structural_prior_event_key(event: &StructuralPriorEvent) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        event.source_label,
        event.symbol,
        event.recommendation_id,
        event.recommended_at,
        event.node_id,
        event.branch_id,
        event.path_id
    )
}

fn append_structural_prior_event(
    state: &mut StructuralPriorLearningState,
    event: StructuralPriorEvent,
) -> bool {
    let key = structural_prior_event_key(&event);
    if state
        .event_ledger
        .iter()
        .any(|existing| structural_prior_event_key(existing) == key)
    {
        return false;
    }
    state.event_ledger.push(event);
    true
}

pub(crate) fn structural_sorted_prior_events(
    event_ledger: &[StructuralPriorEvent],
) -> Vec<StructuralPriorEvent> {
    let mut events = event_ledger.to_vec();
    events.sort_by(|left, right| {
        left.symbol
            .cmp(&right.symbol)
            .then_with(|| left.recommended_at.cmp(&right.recommended_at))
            .then_with(|| left.recommendation_id.cmp(&right.recommendation_id))
            .then_with(|| left.branch_id.cmp(&right.branch_id))
    });
    events
}

fn rebuild_structural_sequence_priors(state: &mut StructuralPriorLearningState) {
    state.source_reliability_em_summaries.clear();
    rebuild_node_duration_priors_from_events(
        &mut state.node_duration_priors,
        &mut state.node_temporal_posteriors,
        &state.event_ledger,
    );
    rebuild_transition_posteriors_from_events(
        &mut state.branch_transition_priors,
        &mut state.branch_temporal_posteriors,
        &mut state.node_transition_posteriors,
        &state.event_ledger,
    );
    refresh_structural_source_reliability_em_state(state);
}

fn family_dominant_action(items: &[&PersistedFactorRanking]) -> &'static str {
    if items.iter().any(|item| item.iteration_action == "replace") {
        "replace"
    } else if items.iter().any(|item| item.iteration_action == "tune") {
        "tune"
    } else if items.iter().any(|item| item.iteration_action == "observe") {
        "observe"
    } else {
        "keep"
    }
}

fn family_decision_status(items: &[&PersistedFactorRanking], avg_score: f64) -> &'static str {
    if items.iter().any(|item| item.replacement_candidate) {
        "review_replace"
    } else if avg_score >= 0.75 && items.iter().all(|item| item.iteration_action == "keep") {
        "stable_keep"
    } else if items.iter().any(|item| item.iteration_action == "tune") {
        "needs_tuning"
    } else if items.iter().any(|item| item.iteration_action == "observe") {
        "needs_observation"
    } else {
        "hold"
    }
}

fn family_risk_flags(
    items: &[&PersistedFactorRanking],
    avg_score: f64,
    replacement_candidates: &[String],
) -> Vec<String> {
    let mut flags = Vec::new();
    if avg_score < 0.45 {
        flags.push("low_family_score".to_string());
    }
    if !replacement_candidates.is_empty() {
        flags.push("contains_replacement_candidates".to_string());
    }
    let unique_actions = items
        .iter()
        .map(|item| item.iteration_action.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if unique_actions.len() > 1 {
        flags.push("mixed_iteration_actions".to_string());
    }
    if items.iter().any(|item| {
        item.weaknesses
            .iter()
            .any(|w| w == "narrow_regime_coverage")
    }) {
        flags.push("narrow_regime_coverage".to_string());
    }
    flags
}

fn family_decision_reason(
    dominant_action: &str,
    avg_score_band: &str,
    risk_flags: &[String],
) -> String {
    if !risk_flags.is_empty() {
        format!(
            "dominant_action={} avg_score_band={} risk_flags={}",
            dominant_action,
            avg_score_band,
            risk_flags.join(",")
        )
    } else {
        format!(
            "dominant_action={} avg_score_band={} no_material_family_risks",
            dominant_action, avg_score_band
        )
    }
}

fn factor_family(name: &str) -> &'static str {
    match name {
        "trend_momentum" => "trend_momentum",
        "volatility_mean_reversion" => "volatility_mean_reversion",
        "structure_ict" => "structure_ict",
        "cross_market_smt" => "cross_market_smt",
        "options_hedging" => "options_hedging",
        _ => "other",
    }
}

pub fn regime_key(regime: Regime) -> &'static str {
    match regime {
        Regime::Accumulation => "accumulation",
        Regime::ManipulationExpansion => "manipulation_expansion",
        Regime::Distribution => "distribution",
    }
}

/// RegimeV2 key for extended 8-state regime system
pub fn regime_v2_key(regime: crate::types::RegimeV2) -> &'static str {
    match regime {
        crate::types::RegimeV2::TrendUpStrong => "trend_up_strong",
        crate::types::RegimeV2::TrendUpWeak => "trend_up_weak",
        crate::types::RegimeV2::RangeVolatile => "range_volatile",
        crate::types::RegimeV2::RangeQuiet => "range_quiet",
        crate::types::RegimeV2::TrendDownWeak => "trend_down_weak",
        crate::types::RegimeV2::TrendDownStrong => "trend_down_strong",
        crate::types::RegimeV2::Transition => "transition",
        crate::types::RegimeV2::CrashRecovery => "crash_recovery",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct RankingInput<'a> {
        name: &'a str,
        mean_ic: f64,
        ir: f64,
        backtest_return: f64,
        sharpe: f64,
        stability: f64,
        win_rate: f64,
        profit_factor: f64,
        trade_count: usize,
    }

    fn ranking(input: RankingInput<'_>) -> PersistedFactorRanking {
        PersistedFactorRanking::from(&FactorIC {
            factor_name: input.name.to_string(),
            regime: Regime::ManipulationExpansion,
            ic_values: vec![0.1, 0.2],
            mean_ic: input.mean_ic,
            std_ic: 0.02,
            ir: input.ir,
            weight: 0.2,
            backtest_return: input.backtest_return,
            sharpe: input.sharpe,
            stability: input.stability,
            win_rate: input.win_rate,
            profit_factor: input.profit_factor,
            trade_count: input.trade_count,
            regime_scores: HashMap::from([("manipulation_expansion".to_string(), 0.02)]),
        })
    }

    fn sample_feedback() -> FeedbackRecord {
        FeedbackRecord {
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            source: "test".to_string(),
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
                weight: 0.2,
                long_support: 0.3,
                short_support: 0.0,
                uncertainty_contribution: 0.1,
            }],
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: Direction::Bull,
                selected_probability: 0.7,
                long_score: 0.4,
                short_score: 0.1,
                win_prob_long: 0.6,
                win_prob_short: 0.3,
                uncertainty: 0.2,
            },
            realized_outcome: "win".to_string(),
            pnl: 0.02,
            regime_at_entry: Regime::ManipulationExpansion,
            structural_feedback: None,
            reflection_mismatch_tags: Vec::new(),
        }
    }

    #[test]
    fn test_merge_feedback_records_deduplicates() {
        let feedback = sample_feedback();
        let mut state = LearningState::default();

        let first = state.merge_feedback_records(std::slice::from_ref(&feedback));
        let second = state.merge_feedback_records(&[feedback]);

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(state.feedback_history.len(), 1);
    }

    #[test]
    fn test_merge_feedback_records_keeps_distinct_structural_refs() {
        let mut first = sample_feedback();
        first.structural_feedback = Some(StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-1".to_string(),
            recommended_at: "2026-04-29T00:00:00Z".to_string(),
            node_id: "node-1".to_string(),
            branch_id: "branch-1".to_string(),
            scenario_id: "scenario-1".to_string(),
            path_id: "path-1".to_string(),
            followed_path: true,
            exit_reason: Some("target_hit".to_string()),
            notes: None,
        });
        let mut second = first.clone();
        second.structural_feedback = Some(StructuralFeedbackRefs {
            path_id: "path-2".to_string(),
            recommendation_id: "rec-2".to_string(),
            ..first.structural_feedback.clone().unwrap()
        });

        let mut state = LearningState::default();
        let inserted = state.merge_feedback_records(&[first, second]);

        assert_eq!(inserted.len(), 2);
        assert_eq!(state.feedback_history.len(), 2);
    }

    #[test]
    fn test_merge_feedback_records_replaces_pending_structural_feedback_with_resolved_outcome() {
        let mut pending = sample_feedback();
        pending.realized_outcome = "pending".to_string();
        pending.pnl = 0.0;
        pending.structural_feedback = Some(StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-pending".to_string(),
            recommended_at: "2026-04-29T00:00:00Z".to_string(),
            node_id: "node-1".to_string(),
            branch_id: "branch-1".to_string(),
            scenario_id: "scenario-1".to_string(),
            path_id: "path-1".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        });
        let mut resolved = pending.clone();
        resolved.realized_outcome = "win".to_string();
        resolved.pnl = 0.02;

        let mut state = LearningState::default();
        let first = state.merge_feedback_records(&[pending]);
        let second = state.merge_feedback_records(&[resolved.clone()]);

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_eq!(state.feedback_history.len(), 1);
        assert_eq!(state.feedback_history[0].realized_outcome, "win");
        assert_eq!(state.feedback_history[0].pnl, 0.02);
    }

    #[test]
    fn test_merge_feedback_records_large_resolved_structural_batch_is_indexed() {
        let base_timestamp = Utc::now();
        let feedback = (0..3_000)
            .map(|index| {
                let mut record = sample_feedback();
                record.timestamp = base_timestamp + chrono::Duration::milliseconds(index as i64);
                record.trade_id = Some(format!("trade-{index}"));
                record.realized_outcome = if index % 2 == 0 {
                    "win".to_string()
                } else {
                    "loss".to_string()
                };
                record.pnl = if index % 2 == 0 { 0.01 } else { -0.01 };
                record.structural_feedback = Some(StructuralFeedbackRefs {
                    protocol_version: "structural-feedback-v1".to_string(),
                    recommendation_id: format!("rec-{index}"),
                    recommended_at: "2026-04-29T00:00:00Z".to_string(),
                    node_id: format!("node-{}", index % 4),
                    branch_id: format!("branch-{}", index % 8),
                    scenario_id: format!("scenario-{}", index % 16),
                    path_id: format!("path-{index}"),
                    followed_path: true,
                    exit_reason: Some("target_hit".to_string()),
                    notes: None,
                });
                record
            })
            .collect::<Vec<_>>();

        let mut state = LearningState::default();
        let started = std::time::Instant::now();
        let inserted = state.merge_feedback_records(&feedback);
        let elapsed = started.elapsed();

        assert_eq!(inserted.len(), feedback.len());
        assert_eq!(state.feedback_history.len(), feedback.len());
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "merge_feedback_records should use indexed resolution lookup for large resolved structural batches; elapsed={elapsed:?}"
        );
    }

    #[test]
    fn test_structural_prior_seed_source_weight_affects_smoothed_prior() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-1".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-1".to_string(),
            branch_id: "branch-1".to_string(),
            scenario_id: "scenario-1".to_string(),
            path_id: "path-1".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        };
        let analyze_seed = StructuralPriorSeed {
            source_label: "analyze".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.01,
        };
        let backtest_seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            ..analyze_seed.clone()
        };

        let mut analyze_state = LearningState::default();
        analyze_state.apply_structural_prior_seed(&refs, &analyze_seed);
        let mut backtest_state = LearningState::default();
        backtest_state.apply_structural_prior_seed(&refs, &backtest_seed);

        assert!(
            backtest_state
                .structural_prior_state
                .paths
                .get("path-1")
                .unwrap()
                .smoothed_prior
                > analyze_state
                    .structural_prior_state
                    .paths
                    .get("path-1")
                    .unwrap()
                    .smoothed_prior
        );
    }

    #[test]
    fn test_structural_prior_seed_tempering_coefficient_affects_smoothed_prior() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-temper".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-temper".to_string(),
            branch_id: "branch-temper".to_string(),
            scenario_id: "scenario-temper".to_string(),
            path_id: "path-temper".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        };
        let weak_seed = StructuralPriorSeed {
            source_label: "research".to_string(),
            tempering_coefficient: Some(0.25),
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.01,
        };
        let strong_seed = StructuralPriorSeed {
            tempering_coefficient: Some(0.90),
            ..weak_seed.clone()
        };

        let mut weak_state = LearningState::default();
        weak_state.apply_structural_prior_seed(&refs, &weak_seed);
        let mut strong_state = LearningState::default();
        strong_state.apply_structural_prior_seed(&refs, &strong_seed);

        let weak_path = weak_state
            .structural_prior_state
            .paths
            .get("path-temper")
            .expect("weak path prior state");
        let strong_path = strong_state
            .structural_prior_state
            .paths
            .get("path-temper")
            .expect("strong path prior state");

        assert!(strong_path.weighted_success_mass > weak_path.weighted_success_mass);
        assert!(strong_path.smoothed_prior > weak_path.smoothed_prior);
        assert_eq!(
            strong_path.source_panel_summaries["research"].last_tempering_coefficient,
            Some(0.90)
        );
    }

    #[test]
    fn test_apply_structural_feedback_skips_unresolved_outcomes() {
        let mut feedback = sample_feedback();
        feedback.source = "structural_feedback_submission".to_string();
        feedback.realized_outcome = "pending".to_string();
        feedback.pnl = 0.0;
        feedback.structural_feedback = Some(StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-pending".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-pending".to_string(),
            branch_id: "branch-pending".to_string(),
            scenario_id: "scenario-pending".to_string(),
            path_id: "path-pending".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        });

        let mut state = LearningState::default();
        state.apply_structural_feedback(&[feedback]);

        assert!(state.structural_prior_state.paths.is_empty());
        assert!(state.structural_prior_state.nodes.is_empty());
        assert!(state.structural_prior_state.event_ledger.is_empty());
    }

    #[test]
    fn test_delayed_structural_feedback_resolution_applies_only_resolved_pseudo_count() {
        let mut pending = sample_feedback();
        pending.source = "structural_feedback_submission".to_string();
        pending.realized_outcome = "delayed".to_string();
        pending.pnl = 0.0;
        pending.structural_feedback = Some(StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-delayed".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-delayed".to_string(),
            branch_id: "branch-delayed".to_string(),
            scenario_id: "scenario-delayed".to_string(),
            path_id: "path-delayed".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        });
        let mut resolved = pending.clone();
        resolved.realized_outcome = "win".to_string();
        resolved.pnl = 0.03;

        let mut state = LearningState::default();
        let first = state.merge_feedback_records(&[pending]);
        state.apply_structural_feedback(&first);
        let second = state.merge_feedback_records(&[resolved]);
        state.apply_structural_feedback(&second);

        assert_eq!(state.feedback_history.len(), 1);
        assert_eq!(state.feedback_history[0].realized_outcome, "win");
        let path = state
            .structural_prior_state
            .paths
            .get("path-delayed")
            .expect("resolved path prior state");
        assert_eq!(path.observations, 1);
        assert_eq!(path.wins, 1);
        assert_eq!(path.weighted_failure_mass, 0.0);
        assert!(path.weighted_success_mass > 0.0);
        assert_eq!(state.structural_prior_state.event_ledger.len(), 1);
        assert_eq!(
            state.structural_prior_state.event_ledger[0]
                .realized_outcome
                .as_deref(),
            Some("win")
        );
    }

    #[test]
    fn test_live_structural_feedback_weights_more_than_analyze_seed() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-1".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-1".to_string(),
            branch_id: "branch-1".to_string(),
            scenario_id: "scenario-1".to_string(),
            path_id: "path-1".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        };
        let analyze_seed = StructuralPriorSeed {
            source_label: "analyze".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.01,
        };
        let mut live_feedback = sample_feedback();
        live_feedback.source = "structural_feedback_submission".to_string();
        live_feedback.structural_feedback = Some(refs.clone());

        let mut analyze_state = LearningState::default();
        analyze_state.apply_structural_prior_seed(&refs, &analyze_seed);
        let mut live_state = LearningState::default();
        live_state.apply_structural_feedback(&[live_feedback]);

        assert!(
            live_state
                .structural_prior_state
                .paths
                .get("path-1")
                .unwrap()
                .smoothed_prior
                > analyze_state
                    .structural_prior_state
                    .paths
                    .get("path-1")
                    .unwrap()
                    .smoothed_prior
        );
    }

    #[test]
    fn test_live_structural_feedback_loss_adds_failure_mass_and_pushes_prior_below_neutral() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-loss".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-loss".to_string(),
            branch_id: "branch-loss".to_string(),
            scenario_id: "scenario-loss".to_string(),
            path_id: "path-loss".to_string(),
            followed_path: true,
            exit_reason: Some("stop_hit".to_string()),
            notes: None,
        };
        let mut feedback = sample_feedback();
        feedback.source = "structural_feedback_submission".to_string();
        feedback.realized_outcome = "loss".to_string();
        feedback.pnl = -0.02;
        feedback.structural_feedback = Some(refs);

        let mut state = LearningState::default();
        state.apply_structural_feedback(&[feedback]);

        let path = state
            .structural_prior_state
            .paths
            .get("path-loss")
            .expect("path prior state");
        assert!(path.weighted_failure_mass > 0.0);
        assert!(path.smoothed_prior < 0.5);
    }

    #[test]
    fn test_structural_feedback_node_mass_updates_less_than_path_mass() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-scale".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-scale".to_string(),
            branch_id: "branch-scale".to_string(),
            scenario_id: "scenario-scale".to_string(),
            path_id: "path-scale".to_string(),
            followed_path: true,
            exit_reason: Some("target_hit".to_string()),
            notes: None,
        };
        let mut feedback = sample_feedback();
        feedback.source = "structural_feedback_submission".to_string();
        feedback.structural_feedback = Some(refs);

        let mut state = LearningState::default();
        state.apply_structural_feedback(&[feedback]);

        let node = state
            .structural_prior_state
            .nodes
            .get("node-scale")
            .expect("node prior state");
        let branch = state
            .structural_prior_state
            .branches
            .get("branch-scale")
            .expect("branch prior state");
        let scenario = state
            .structural_prior_state
            .scenarios
            .get("scenario-scale")
            .expect("scenario prior state");
        let path = state
            .structural_prior_state
            .paths
            .get("path-scale")
            .expect("path prior state");

        assert!(node.weighted_success_mass < branch.weighted_success_mass);
        assert!(branch.weighted_success_mass < scenario.weighted_success_mass);
        assert!(scenario.weighted_success_mass < path.weighted_success_mass);
    }

    #[test]
    fn test_invalidated_feedback_counts_more_failure_mass_than_plain_loss() {
        let mut invalidated_feedback = sample_feedback();
        invalidated_feedback.source = "structural_feedback_submission".to_string();
        invalidated_feedback.realized_outcome = "invalidated".to_string();
        invalidated_feedback.pnl = -0.01;
        invalidated_feedback.structural_feedback = Some(StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-invalidated".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-invalidated".to_string(),
            branch_id: "branch-invalidated".to_string(),
            scenario_id: "scenario-invalidated".to_string(),
            path_id: "path-invalidated".to_string(),
            followed_path: true,
            exit_reason: Some("invalidation".to_string()),
            notes: None,
        });
        let mut loss_feedback = invalidated_feedback.clone();
        if let Some(refs) = loss_feedback.structural_feedback.as_mut() {
            refs.recommendation_id = "rec-loss".to_string();
            refs.node_id = "node-loss".to_string();
            refs.branch_id = "branch-loss".to_string();
            refs.scenario_id = "scenario-loss".to_string();
            refs.path_id = "path-loss".to_string();
        }
        loss_feedback.realized_outcome = "loss".to_string();

        let mut invalidated_state = LearningState::default();
        invalidated_state.apply_structural_feedback(&[invalidated_feedback]);
        let mut loss_state = LearningState::default();
        loss_state.apply_structural_feedback(&[loss_feedback]);

        let invalidated_path = invalidated_state
            .structural_prior_state
            .paths
            .get("path-invalidated")
            .expect("invalidated path prior state");
        let loss_path = loss_state
            .structural_prior_state
            .paths
            .get("path-loss")
            .expect("loss path prior state");
        assert!(invalidated_path.weighted_failure_mass > loss_path.weighted_failure_mass);
        assert!(invalidated_path.smoothed_prior < loss_path.smoothed_prior);
    }

    #[test]
    fn test_abandoned_feedback_uses_fractional_pseudo_counts_in_structural_prior() {
        let mut feedback = sample_feedback();
        feedback.source = "structural_feedback_submission".to_string();
        feedback.realized_outcome = "abandoned".to_string();
        feedback.pnl = 0.0;
        feedback.structural_feedback = Some(StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-abandoned".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-abandoned".to_string(),
            branch_id: "branch-abandoned".to_string(),
            scenario_id: "scenario-abandoned".to_string(),
            path_id: "path-abandoned".to_string(),
            followed_path: true,
            exit_reason: Some("manual_exit".to_string()),
            notes: None,
        });

        let mut state = LearningState::default();
        state.apply_structural_feedback(&[feedback]);

        let path = state
            .structural_prior_state
            .paths
            .get("path-abandoned")
            .expect("path prior state");
        assert_eq!(path.abandoned, 1);
        assert_eq!(path.followed_count, 1);
        assert!((path.weighted_followed_mass - 0.75).abs() < 1e-9);
        assert!((path.weighted_success_mass - 0.1875).abs() < 1e-9);
        assert!((path.weighted_failure_mass - 0.5625).abs() < 1e-9);
        assert!(path.smoothed_prior > 0.40);
        assert!(path.smoothed_prior < 0.50);
    }

    #[test]
    fn test_not_followed_feedback_updates_propensity_without_reward_credit() {
        let mut followed = sample_feedback();
        followed.source = "structural_feedback_submission".to_string();
        followed.realized_outcome = "win".to_string();
        followed.pnl = 0.02;
        followed.structural_feedback = Some(StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-followed".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-propensity".to_string(),
            branch_id: "branch-propensity".to_string(),
            scenario_id: "scenario-propensity".to_string(),
            path_id: "path-propensity".to_string(),
            followed_path: true,
            exit_reason: Some("target_hit".to_string()),
            notes: None,
        });

        let mut not_followed = followed.clone();
        not_followed.realized_outcome = "not_followed".to_string();
        not_followed.pnl = 0.0;
        if let Some(refs) = not_followed.structural_feedback.as_mut() {
            refs.recommendation_id = "rec-not-followed".to_string();
            refs.recommended_at = "2026-04-30T00:05:00Z".to_string();
            refs.followed_path = false;
            refs.exit_reason = Some("skipped".to_string());
        }

        let mut state = LearningState::default();
        state.apply_structural_feedback(&[followed]);
        let prior_after_followed = state
            .structural_prior_state
            .paths
            .get("path-propensity")
            .expect("path prior state")
            .smoothed_prior;
        state.apply_structural_feedback(&[not_followed]);

        let path = state
            .structural_prior_state
            .paths
            .get("path-propensity")
            .expect("path prior state");
        assert_eq!(path.observations, 2);
        assert_eq!(path.followed_count, 1);
        assert_eq!(path.not_followed, 1);
        assert!((path.weighted_followed_mass - 1.0).abs() < 1e-9);
        assert!((path.weighted_exposure_mass - 2.0).abs() < 1e-9);
        assert!((path.weighted_not_followed_mass - 1.0).abs() < 1e-9);
        assert!((path.smoothed_prior - prior_after_followed).abs() < 1e-9);
        assert!((path.execution_propensity - 0.5).abs() < 1e-9);
        assert!((path.ips_weight - 2.0).abs() < 1e-9);
        assert!((path.counterfactual_success_mass - 2.0).abs() < 1e-9);
        assert!(path.counterfactual_failure_mass.abs() < 1e-9);
        assert!((path.counterfactual_reward_prior - 0.75).abs() < 1e-9);
        assert!((path.off_policy_adjusted_prior - 0.375).abs() < 1e-9);
        let posterior = state
            .structural_prior_state
            .source_reliability_posteriors
            .get("structural_feedback_submission")
            .expect("live feedback source reliability posterior");
        assert_eq!(posterior.observations, 2);
        assert!((posterior.weighted_exposure_mass - 2.0).abs() < 1e-9);
        assert!((posterior.weighted_not_followed_mass - 1.0).abs() < 1e-9);
        assert!((posterior.weighted_success_mass - 1.0).abs() < 1e-9);
        assert!((posterior.posterior_reliability - (2.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn test_structural_feedback_records_snips_and_dr_policy_priors() {
        let mut win = sample_feedback();
        win.timestamp = DateTime::parse_from_rfc3339("2026-04-30T00:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        win.source = "structural_feedback_submission".to_string();
        win.realized_outcome = "win".to_string();
        win.model_probabilities_before_trade.selected_probability = 0.50;
        win.model_probabilities_before_trade.win_prob_long = 0.60;
        win.structural_feedback = Some(StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-policy-win".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-policy".to_string(),
            branch_id: "branch-policy".to_string(),
            scenario_id: "scenario-policy".to_string(),
            path_id: "path-policy".to_string(),
            followed_path: true,
            exit_reason: Some("target_hit".to_string()),
            notes: None,
        });

        let mut loss = win.clone();
        loss.timestamp = DateTime::parse_from_rfc3339("2026-04-30T01:05:00Z")
            .unwrap()
            .with_timezone(&Utc);
        loss.realized_outcome = "loss".to_string();
        loss.pnl = -0.01;
        loss.model_probabilities_before_trade.selected_probability = 0.25;
        if let Some(refs) = loss.structural_feedback.as_mut() {
            refs.recommendation_id = "rec-policy-loss".to_string();
            refs.recommended_at = "2026-04-30T00:05:00Z".to_string();
            refs.exit_reason = Some("stop_loss".to_string());
        }

        let mut state = LearningState::default();
        state.apply_structural_feedback(&[win, loss]);

        let path = state
            .structural_prior_state
            .paths
            .get("path-policy")
            .expect("path policy correction stats");
        assert!((path.policy_weighted_observation_mass - 2.0).abs() < 1e-9);
        assert!((path.behavior_policy_probability - 0.375).abs() < 1e-9);
        assert!((path.behavior_policy_probability_squared_mass - 0.3125).abs() < 1e-9);
        assert!((path.behavior_policy_probability_variance - 0.015625).abs() < 1e-9);
        let expected_probability_confidence = (2.0 / 3.0) * (1.0 - 0.015625_f64.sqrt());
        let expected_probability_lower_bound = 0.375 - (0.015625_f64 / 3.0).sqrt();
        assert!(
            (path.target_policy_probability_confidence - expected_probability_confidence).abs()
                < 1e-9
        );
        assert!(
            (path.target_policy_probability_lower_bound - expected_probability_lower_bound).abs()
                < 1e-9
        );
        assert!((path.target_policy_probability_brier_score_mass - 0.3125).abs() < 1e-9);
        assert!((path.target_policy_probability_calibration_error_mass - 0.75).abs() < 1e-9);
        assert!((path.target_policy_probability_brier_score - 0.15625).abs() < 1e-9);
        assert!((path.target_policy_probability_calibration_error - 0.375).abs() < 1e-9);
        assert!((path.snips_weight_mass - 6.0).abs() < 1e-9);
        assert!((path.snips_weight_squared_mass - 20.0).abs() < 1e-9);
        assert!((path.snips_effective_sample_size - 1.8).abs() < 1e-9);
        assert!((path.snips_reward_mass - 2.0).abs() < 1e-9);
        assert!((path.snips_reward_prior - (1.0 / 3.0)).abs() < 1e-9);
        assert!((path.doubly_robust_reward_mass - 1.0).abs() < 1e-9);
        assert!((path.doubly_robust_reward_prior - 0.5).abs() < 1e-9);
        let expected_target_policy_weight = 1.8 / 2.8;
        let expected_target_policy_variance_penalty = (0.25_f64 / 2.8).sqrt();
        assert!(
            (path.target_policy_calibration_weight - expected_target_policy_weight).abs() < 1e-9
        );
        assert!((path.target_policy_reward_prior - 0.5).abs() < 1e-9);
        assert!(
            (path.target_policy_variance_penalty - expected_target_policy_variance_penalty).abs()
                < 1e-9
        );
        assert!(
            (path.target_policy_reward_lower_bound
                - (0.5 - expected_target_policy_variance_penalty))
                .abs()
                < 1e-9
        );
        assert!((path.delayed_reward_resolution_probability - 0.75).abs() < 1e-9);
        assert!((path.delayed_reward_censoring_probability - 0.25).abs() < 1e-9);
        assert!((path.censoring_adjusted_reward_prior - 0.5).abs() < 1e-9);
        assert!(
            (path.censoring_adjusted_reward_lower_bound
                - ((0.5 - expected_target_policy_variance_penalty) * 0.75 + 0.0625))
                .abs()
                < 1e-9
        );
        let expected_competing_risks: [f64; 4] = [2.0 / 6.0, 2.0 / 6.0, 1.0 / 6.0, 1.0 / 6.0];
        let expected_competing_risk_entropy: f64 = expected_competing_risks
            .iter()
            .map(|risk| -*risk * (*risk).ln())
            .sum();
        assert!(
            (path.delayed_reward_success_competing_risk - expected_competing_risks[0]).abs() < 1e-9
        );
        assert!(
            (path.delayed_reward_failure_competing_risk - expected_competing_risks[1]).abs() < 1e-9
        );
        assert!(
            (path.delayed_reward_invalidation_competing_risk - expected_competing_risks[2]).abs()
                < 1e-9
        );
        assert!(
            (path.delayed_reward_abandonment_competing_risk - expected_competing_risks[3]).abs()
                < 1e-9
        );
        assert!(
            (path.delayed_reward_competing_risk_entropy - expected_competing_risk_entropy).abs()
                < 1e-9
        );
        assert_eq!(path.delayed_reward_elapsed_feedback_count, 2);
        assert!((path.delayed_reward_elapsed_hours_at_risk - 1.5).abs() < 1e-9);
        assert!((path.delayed_reward_avg_elapsed_hours - 0.75).abs() < 1e-9);
        let expected_resolution_hazard = 2.0 / 1.5;
        assert!(
            (path.delayed_reward_resolution_hazard_per_hour - expected_resolution_hazard).abs()
                < 1e-9
        );
        assert!((path.delayed_reward_expected_resolution_hours - 0.75).abs() < 1e-9);
        assert!(
            (path.delayed_reward_survival_probability_1h
                - (-expected_resolution_hazard * 1.0).exp())
            .abs()
                < 1e-9
        );
        assert!(
            (path.delayed_reward_survival_probability_4h
                - (-expected_resolution_hazard * 4.0).exp())
            .abs()
                < 1e-9
        );
        assert!(
            (path.delayed_reward_survival_probability_24h
                - (-expected_resolution_hazard * 24.0).exp())
            .abs()
                < 1e-9
        );
        assert!((path.delayed_reward_success_hazard_per_hour - (1.0 / 1.5)).abs() < 1e-9);
        assert!((path.delayed_reward_failure_hazard_per_hour - (1.0 / 1.5)).abs() < 1e-9);
        assert!(path.delayed_reward_invalidation_hazard_per_hour.abs() < 1e-9);
        assert!(path.delayed_reward_abandonment_hazard_per_hour.abs() < 1e-9);
        let expected_total_incidence_4h = 1.0 - (-expected_resolution_hazard * 4.0).exp();
        assert!(
            (path.delayed_reward_success_cumulative_incidence_4h
                - expected_total_incidence_4h * 0.5)
                .abs()
                < 1e-9
        );
        assert!(
            (path.delayed_reward_failure_cumulative_incidence_4h
                - expected_total_incidence_4h * 0.5)
                .abs()
                < 1e-9
        );
        assert!(
            path.delayed_reward_invalidation_cumulative_incidence_4h
                .abs()
                < 1e-9
        );
        assert!(
            path.delayed_reward_abandonment_cumulative_incidence_4h
                .abs()
                < 1e-9
        );
        assert_eq!(path.delayed_reward_resolution_horizon_1h_count, 2);
        assert_eq!(path.delayed_reward_resolution_within_1h_count, 2);
        assert!((path.delayed_reward_resolution_probability_1h - 0.75).abs() < 1e-9);
        assert_eq!(path.delayed_reward_resolution_horizon_4h_count, 2);
        assert_eq!(path.delayed_reward_resolution_within_4h_count, 2);
        assert!((path.delayed_reward_resolution_probability_4h - 0.75).abs() < 1e-9);
        assert_eq!(path.delayed_reward_resolution_horizon_24h_count, 2);
        assert_eq!(path.delayed_reward_resolution_within_24h_count, 2);
        assert!((path.delayed_reward_resolution_probability_24h - 0.75).abs() < 1e-9);
        let context = state
            .structural_prior_state
            .target_policy_context_posteriors
            .get("NQ:manipulation_expansion:bull")
            .expect("contextual target-policy probability posterior");
        assert_eq!(context.observations, 2);
        assert!((context.weighted_observation_mass - 2.0).abs() < 1e-9);
        assert!((context.success_mass - 1.0).abs() < 1e-9);
        assert!((context.failure_mass - 1.0).abs() < 1e-9);
        assert!((context.behavior_policy_probability - 0.375).abs() < 1e-9);
        assert!((context.behavior_policy_probability_variance - 0.015625).abs() < 1e-9);
        assert!((context.learned_target_policy_probability - 0.5).abs() < 1e-9);
        let expected_context_lower_bound = 0.5 - (0.25_f64 / 3.0).sqrt();
        assert!(
            (context.learned_target_policy_probability_lower_bound - expected_context_lower_bound)
                .abs()
                < 1e-9
        );
        let expected_context_confidence =
            (2.0 / 3.0) * (1.0 - context.target_policy_probability_brier_score.sqrt());
        assert!(
            (context.learned_target_policy_probability_confidence - expected_context_confidence)
                .abs()
                < 1e-9
        );
        let expected_calibrated_context_probability =
            0.5 * expected_context_confidence + 0.375 * (1.0 - expected_context_confidence);
        assert!(
            (context.calibrated_target_policy_probability
                - expected_calibrated_context_probability)
                .abs()
                < 1e-9
        );
        let expected_calibrated_context_lower_bound = expected_context_lower_bound
            * expected_context_confidence
            + expected_probability_lower_bound * (1.0 - expected_context_confidence);
        assert!(
            (context.calibrated_target_policy_probability_lower_bound
                - expected_calibrated_context_lower_bound)
                .abs()
                < 1e-9
        );
        assert!((context.target_policy_probability_brier_score - 0.15625).abs() < 1e-9);
        assert!((context.target_policy_probability_calibration_error - 0.375).abs() < 1e-9);
        assert_eq!(
            context.last_recommendation_id.as_deref(),
            Some("rec-policy-loss")
        );
        assert_eq!(
            context.last_updated_at.as_deref(),
            Some("2026-04-30T01:05:00Z")
        );
        let source_summary = path
            .source_panel_summaries
            .get("structural_feedback_submission")
            .expect("path source policy correction stats");
        assert!((source_summary.behavior_policy_probability - 0.375).abs() < 1e-9);
        assert!((source_summary.behavior_policy_probability_squared_mass - 0.3125).abs() < 1e-9);
        assert!((source_summary.behavior_policy_probability_variance - 0.015625).abs() < 1e-9);
        assert!(
            (source_summary.target_policy_probability_confidence - expected_probability_confidence)
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.target_policy_probability_lower_bound
                - expected_probability_lower_bound)
                .abs()
                < 1e-9
        );
        assert!((source_summary.target_policy_probability_brier_score - 0.15625).abs() < 1e-9);
        assert!((source_summary.target_policy_probability_calibration_error - 0.375).abs() < 1e-9);
        assert!((source_summary.snips_weight_squared_mass - 20.0).abs() < 1e-9);
        assert!((source_summary.snips_effective_sample_size - 1.8).abs() < 1e-9);
        assert!((source_summary.snips_reward_prior - (1.0 / 3.0)).abs() < 1e-9);
        assert!((source_summary.doubly_robust_reward_prior - 0.5).abs() < 1e-9);
        assert!(
            (source_summary.target_policy_calibration_weight - expected_target_policy_weight).abs()
                < 1e-9
        );
        assert!((source_summary.target_policy_reward_prior - 0.5).abs() < 1e-9);
        assert!(
            (source_summary.target_policy_variance_penalty
                - expected_target_policy_variance_penalty)
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.target_policy_reward_lower_bound
                - (0.5 - expected_target_policy_variance_penalty))
                .abs()
                < 1e-9
        );
        assert!((source_summary.delayed_reward_resolution_probability - 0.75).abs() < 1e-9);
        assert!((source_summary.delayed_reward_censoring_probability - 0.25).abs() < 1e-9);
        assert!((source_summary.censoring_adjusted_reward_prior - 0.5).abs() < 1e-9);
        assert!(
            (source_summary.censoring_adjusted_reward_lower_bound
                - ((0.5 - expected_target_policy_variance_penalty) * 0.75 + 0.0625))
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_success_competing_risk - expected_competing_risks[0])
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_failure_competing_risk - expected_competing_risks[1])
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_invalidation_competing_risk
                - expected_competing_risks[2])
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_abandonment_competing_risk
                - expected_competing_risks[3])
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_competing_risk_entropy
                - expected_competing_risk_entropy)
                .abs()
                < 1e-9
        );
        assert_eq!(source_summary.delayed_reward_elapsed_feedback_count, 2);
        assert!((source_summary.delayed_reward_elapsed_hours_at_risk - 1.5).abs() < 1e-9);
        assert!((source_summary.delayed_reward_avg_elapsed_hours - 0.75).abs() < 1e-9);
        assert!(
            (source_summary.delayed_reward_resolution_hazard_per_hour - expected_resolution_hazard)
                .abs()
                < 1e-9
        );
        assert!((source_summary.delayed_reward_expected_resolution_hours - 0.75).abs() < 1e-9);
        assert!(
            (source_summary.delayed_reward_survival_probability_1h
                - (-expected_resolution_hazard * 1.0).exp())
            .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_survival_probability_4h
                - (-expected_resolution_hazard * 4.0).exp())
            .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_survival_probability_24h
                - (-expected_resolution_hazard * 24.0).exp())
            .abs()
                < 1e-9
        );
        assert!((source_summary.delayed_reward_success_hazard_per_hour - (1.0 / 1.5)).abs() < 1e-9);
        assert!((source_summary.delayed_reward_failure_hazard_per_hour - (1.0 / 1.5)).abs() < 1e-9);
        assert!(
            source_summary
                .delayed_reward_invalidation_hazard_per_hour
                .abs()
                < 1e-9
        );
        assert!(
            source_summary
                .delayed_reward_abandonment_hazard_per_hour
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_success_cumulative_incidence_4h
                - expected_total_incidence_4h * 0.5)
                .abs()
                < 1e-9
        );
        assert!(
            (source_summary.delayed_reward_failure_cumulative_incidence_4h
                - expected_total_incidence_4h * 0.5)
                .abs()
                < 1e-9
        );
        assert!(
            source_summary
                .delayed_reward_invalidation_cumulative_incidence_4h
                .abs()
                < 1e-9
        );
        assert!(
            source_summary
                .delayed_reward_abandonment_cumulative_incidence_4h
                .abs()
                < 1e-9
        );
        assert_eq!(source_summary.delayed_reward_resolution_horizon_1h_count, 2);
        assert_eq!(source_summary.delayed_reward_resolution_within_1h_count, 2);
        assert!((source_summary.delayed_reward_resolution_probability_1h - 0.75).abs() < 1e-9);
        assert_eq!(source_summary.delayed_reward_resolution_horizon_4h_count, 2);
        assert_eq!(source_summary.delayed_reward_resolution_within_4h_count, 2);
        assert!((source_summary.delayed_reward_resolution_probability_4h - 0.75).abs() < 1e-9);
        assert_eq!(
            source_summary.delayed_reward_resolution_horizon_24h_count,
            2
        );
        assert_eq!(source_summary.delayed_reward_resolution_within_24h_count, 2);
        assert!((source_summary.delayed_reward_resolution_probability_24h - 0.75).abs() < 1e-9);
    }

    #[test]
    fn test_structural_prior_seed_records_source_panel_summary() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-panel".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-panel".to_string(),
            branch_id: "branch-panel".to_string(),
            scenario_id: "scenario-panel".to_string(),
            path_id: "path-panel".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        };
        let analyze_seed = StructuralPriorSeed {
            source_label: "analyze".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.01,
        };
        let backtest_seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            tempering_coefficient: None,
            observations: 2,
            followed_count: 2,
            wins: 1,
            losses: 1,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.015,
        };

        let mut state = LearningState::default();
        state.apply_structural_prior_seed(&refs, &analyze_seed);
        state.apply_structural_prior_seed(&refs, &backtest_seed);

        let path = state
            .structural_prior_state
            .paths
            .get("path-panel")
            .expect("path prior state");
        let analyze_panel = path
            .source_panel_summaries
            .get("analyze")
            .expect("analyze source panel");
        let backtest_panel = path
            .source_panel_summaries
            .get("backtest")
            .expect("backtest source panel");

        assert_eq!(analyze_panel.observations, 1);
        assert_eq!(analyze_panel.wins, 1);
        assert_eq!(backtest_panel.observations, 2);
        assert_eq!(backtest_panel.losses, 1);
        let backtest_power_prior = backtest_panel
            .last_power_prior_contribution
            .as_ref()
            .expect("backtest power-prior contribution");
        assert_eq!(backtest_power_prior.source_label, "backtest");
        assert!((backtest_power_prior.base_source_weight - 0.75).abs() < 1e-9);
        assert!((backtest_power_prior.tempering_coefficient - 1.0).abs() < 1e-9);
        assert!((backtest_power_prior.entity_mass_scale - 1.0).abs() < 1e-9);
        assert!((backtest_power_prior.effective_tau - 0.75).abs() < 1e-9);
        assert!((backtest_power_prior.observation_mass - 1.5).abs() < 1e-9);
        assert!((backtest_power_prior.success_mass - 0.75).abs() < 1e-9);
        assert!((backtest_power_prior.failure_mass - 0.75).abs() < 1e-9);
        assert_eq!(path.last_offline_seed_source.as_deref(), Some("backtest"));
    }

    #[test]
    fn test_structural_prior_seed_updates_reusable_source_reliability_posterior() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-source-reliability".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-source-reliability".to_string(),
            branch_id: "branch-source-reliability".to_string(),
            scenario_id: "scenario-source-reliability".to_string(),
            path_id: "path-source-reliability".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        };
        let seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            tempering_coefficient: None,
            observations: 2,
            followed_count: 2,
            wins: 1,
            losses: 1,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.015,
        };

        let mut state = LearningState::default();
        state.apply_structural_prior_seed(&refs, &seed);

        let posterior = state
            .structural_prior_state
            .source_reliability_posteriors
            .get("backtest")
            .expect("backtest source reliability posterior");
        assert_eq!(posterior.source_label, "backtest");
        assert_eq!(posterior.observations, 2);
        assert!((posterior.weighted_observation_mass - 1.5).abs() < 1e-9);
        assert!((posterior.weighted_success_mass - 0.75).abs() < 1e-9);
        assert!((posterior.weighted_failure_mass - 0.75).abs() < 1e-9);
        assert!((posterior.posterior_reliability - 0.5).abs() < 1e-9);
        let win_cell = posterior
            .outcome_confusion
            .get("win->positive_executed")
            .expect("win confusion cell");
        let loss_cell = posterior
            .outcome_confusion
            .get("loss->negative_executed")
            .expect("loss confusion cell");
        assert_eq!(win_cell.observations, 1);
        assert!((win_cell.weighted_success_mass - 0.75).abs() < 1e-9);
        assert_eq!(loss_cell.observations, 1);
        assert!((loss_cell.weighted_failure_mass - 0.75).abs() < 1e-9);
        assert!(posterior.last_power_prior_contribution.is_some());
    }

    #[test]
    fn test_source_outcome_confusion_derives_smoothed_likelihoods() {
        let mut posterior = StructuralSourceReliabilityPosterior {
            source_label: "backtest".to_string(),
            ..StructuralSourceReliabilityPosterior::default()
        };

        update_structural_source_outcome_confusion_with_count(
            &mut posterior,
            "tp",
            "positive_executed",
            2,
            2.0,
            1.0,
        );
        update_structural_source_outcome_confusion_with_count(
            &mut posterior,
            "take_profit",
            "positive_executed",
            1,
            1.0,
            1.0,
        );
        refresh_structural_source_reliability_posterior(&mut posterior);

        let tp_cell = posterior
            .outcome_confusion
            .get("tp->positive_executed")
            .expect("tp confusion cell");
        let take_profit_cell = posterior
            .outcome_confusion
            .get("take_profit->positive_executed")
            .expect("take-profit confusion cell");

        assert!((tp_cell.credit_class_weighted_observation_mass - 3.0).abs() < 1e-9);
        assert_eq!(tp_cell.credit_class_observed_outcome_count, 2);
        assert!((tp_cell.observed_given_credit_likelihood - 0.6).abs() < 1e-9);
        assert!((take_profit_cell.observed_given_credit_likelihood - 0.4).abs() < 1e-9);
        assert!(
            (structural_source_observed_outcome_likelihood(&posterior, "positive_executed", "tp")
                - 0.6)
                .abs()
                < 1e-9
        );
        assert!(
            (structural_source_observed_outcome_likelihood(
                &posterior,
                "negative_executed",
                "loss"
            ) - 0.5)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn test_structural_prior_seed_persists_source_reliability_em_summaries() {
        fn seed_with_outcome(source_label: &str, outcome: &str) -> StructuralPriorSeed {
            let mut seed = StructuralPriorSeed {
                source_label: source_label.to_string(),
                observations: 1,
                followed_count: 1,
                avg_pnl: 0.01,
                ..StructuralPriorSeed::default()
            };
            match outcome {
                "win" => seed.wins = 1,
                "loss" => seed.losses = 1,
                _ => seed.breakevens = 1,
            }
            seed
        }

        fn refs_for_rec(recommendation_id: &str) -> StructuralFeedbackRefs {
            StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: recommendation_id.to_string(),
                recommended_at: format!("2026-05-02T00:00:0{}Z", &recommendation_id[4..]),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: "scenario-em".to_string(),
                path_id: format!("path-{recommendation_id}"),
                followed_path: true,
                exit_reason: None,
                notes: None,
            }
        }

        let mut state = LearningState::default();
        for (
            recommendation_id,
            backtest_outcome,
            live_outcome,
            research_outcome,
            analyze_outcome,
        ) in [
            ("rec-1", "win", "win", "win", "loss"),
            ("rec-2", "loss", "loss", "loss", "win"),
            ("rec-3", "win", "win", "win", "loss"),
        ] {
            let refs = refs_for_rec(recommendation_id);
            state.apply_structural_prior_seed(
                &refs,
                &seed_with_outcome("backtest", backtest_outcome),
            );
            state.apply_structural_prior_seed(&refs, &seed_with_outcome("live", live_outcome));
            state.apply_structural_prior_seed(
                &refs,
                &seed_with_outcome("research", research_outcome),
            );
            state
                .apply_structural_prior_seed(&refs, &seed_with_outcome("analyze", analyze_outcome));
        }

        let summaries = &state.structural_prior_state.source_reliability_em_summaries;
        assert_eq!(summaries.len(), 4);
        let backtest = summaries
            .get("backtest")
            .expect("backtest persisted EM summary");
        let live = summaries.get("live").expect("live persisted EM summary");
        let analyze = summaries
            .get("analyze")
            .expect("analyze persisted EM summary");
        assert_eq!(
            backtest.iteration_count,
            STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS
        );
        assert_eq!(backtest.latent_item_count, 3);
        assert_eq!(backtest.distinct_label_count, 2);
        assert_eq!(backtest.confusion_cell_count, 4);
        assert!(backtest.posterior_reliability > analyze.posterior_reliability);
        assert!(live.posterior_reliability > analyze.posterior_reliability);
        assert!(backtest.confusion["positive_executed->positive_executed"].probability > 0.5);
        assert!(analyze.confusion["positive_executed->negative_executed"].probability > 0.5);
        let calibration = state
            .structural_prior_state
            .source_reliability_em_calibration
            .as_ref()
            .expect("persisted EM calibration summary");
        assert_eq!(calibration.status, "ready");
        assert_eq!(calibration.observation_count, 12);
        assert_eq!(calibration.source_count, 4);
        assert_eq!(
            calibration.min_observations,
            STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_CALIBRATION_OBSERVATIONS
        );
        assert!(calibration.brier_score.unwrap() < 0.6);
        assert!(calibration.log_loss.unwrap() < 0.8);
        let diagnostics =
            structural_source_reliability_em_diagnostics(&state.structural_prior_state);
        let holdout = diagnostics
            .holdout
            .as_ref()
            .expect("holdout diagnostics summary");
        assert!(holdout.training_item_count > 0);
        assert!(holdout.evaluation_item_count <= holdout.training_item_count);
        assert_eq!(
            holdout.min_training_items,
            STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_HOLDOUT_TRAIN_ITEMS
        );
        assert!(matches!(
            holdout.status.as_str(),
            "ready" | "needs_larger_panel" | "needs_more_items"
        ));
    }

    #[test]
    fn test_structural_source_reliability_em_holdout_prefers_chronological_split() {
        let mut state = StructuralPriorLearningState::default();
        for idx in 0..6 {
            let recommendation_id = format!("rec-{idx}");
            for (source_label, realized_outcome) in [("backtest", "win"), ("live", "loss")] {
                state.event_ledger.push(StructuralPriorEvent {
                    source_label: source_label.to_string(),
                    symbol: "NQ".to_string(),
                    recommendation_id: recommendation_id.clone(),
                    recommended_at: format!("2026-05-0{}T00:00:00Z", idx + 1),
                    node_id: "node-em".to_string(),
                    branch_id: "branch-em".to_string(),
                    scenario_id: "scenario-em".to_string(),
                    path_id: format!("path-{recommendation_id}"),
                    followed_path: true,
                    realized_outcome: Some(realized_outcome.to_string()),
                });
            }
        }

        let summary = structural_source_reliability_em_diagnostics(&state)
            .holdout
            .expect("chronological holdout summary");
        assert_eq!(summary.split_strategy, "chronological_recommended_at");
        assert_eq!(summary.training_item_count, 4);
        assert_eq!(summary.evaluation_item_count, 2);
        assert!(matches!(
            summary.status.as_str(),
            "ready" | "needs_larger_panel" | "needs_multiple_sources"
        ));
    }

    #[test]
    fn test_structural_prior_seed_node_mass_updates_less_than_path_mass() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-seed-scale".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-seed-scale".to_string(),
            branch_id: "branch-seed-scale".to_string(),
            scenario_id: "scenario-seed-scale".to_string(),
            path_id: "path-seed-scale".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        };
        let seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.02,
        };

        let mut state = LearningState::default();
        state.apply_structural_prior_seed(&refs, &seed);

        let node = state
            .structural_prior_state
            .nodes
            .get("node-seed-scale")
            .expect("node prior state");
        let branch = state
            .structural_prior_state
            .branches
            .get("branch-seed-scale")
            .expect("branch prior state");
        let scenario = state
            .structural_prior_state
            .scenarios
            .get("scenario-seed-scale")
            .expect("scenario prior state");
        let path = state
            .structural_prior_state
            .paths
            .get("path-seed-scale")
            .expect("path prior state");

        assert!(node.weighted_success_mass < branch.weighted_success_mass);
        assert!(branch.weighted_success_mass < scenario.weighted_success_mass);
        assert!(scenario.weighted_success_mass < path.weighted_success_mass);
    }

    #[test]
    fn test_structural_prior_seed_persists_separated_mass_snapshot() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-seed-snapshot".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-seed-snapshot".to_string(),
            branch_id: "branch-seed-snapshot".to_string(),
            scenario_id: "scenario-seed-snapshot".to_string(),
            path_id: "path-seed-snapshot".to_string(),
            followed_path: true,
            exit_reason: None,
            notes: None,
        };
        let seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            tempering_coefficient: None,
            observations: 2,
            followed_count: 2,
            wins: 1,
            losses: 1,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.015,
        };

        let mut state = LearningState::default();
        state.apply_structural_prior_seed(&refs, &seed);

        let structural_state = &state.structural_prior_state;
        let node_mass = structural_state
            .node_prior_mass
            .get("node-seed-snapshot")
            .expect("node prior mass");
        let path_mass = structural_state
            .path_prior_mass
            .get("path-seed-snapshot")
            .expect("path prior mass");

        assert_eq!(node_mass.entity_kind, "node");
        assert_eq!(path_mass.entity_kind, "path");
        assert!((node_mass.weighted_success_mass - 0.375).abs() < 1e-9);
        assert!((path_mass.weighted_success_mass - 0.75).abs() < 1e-9);
        assert!(node_mass.weighted_success_mass < path_mass.weighted_success_mass);

        let snapshot = structural_state
            .last_offline_seed_snapshot
            .as_ref()
            .expect("last offline seed snapshot");
        assert_eq!(snapshot.source_label, "backtest");
        assert_eq!(snapshot.recommendation_id, "rec-seed-snapshot");
        assert_eq!(snapshot.node_id, "node-seed-snapshot");
        assert_eq!(snapshot.path_id, "path-seed-snapshot");
        assert!((snapshot.node_contribution.effective_tau - 0.375).abs() < 1e-9);
        assert!((snapshot.branch_contribution.effective_tau - 0.5625).abs() < 1e-9);
        assert!((snapshot.scenario_contribution.effective_tau - 0.675).abs() < 1e-9);
        assert!((snapshot.path_contribution.effective_tau - 0.75).abs() < 1e-9);

        let serialized = serde_json::to_string(&structural_state).expect("serialize state");
        assert!(serialized.contains("last_offline_seed_snapshot"));
        let round_tripped: StructuralPriorLearningState =
            serde_json::from_str(&serialized).expect("deserialize state");
        assert!(round_tripped.last_offline_seed_snapshot.is_some());
        assert!(round_tripped
            .node_prior_mass
            .contains_key("node-seed-snapshot"));
    }

    #[test]
    fn test_live_feedback_records_separate_source_panel_summary() {
        let refs = StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: "rec-live".to_string(),
            recommended_at: "2026-04-30T00:00:00Z".to_string(),
            node_id: "node-live".to_string(),
            branch_id: "branch-live".to_string(),
            scenario_id: "scenario-live".to_string(),
            path_id: "path-live".to_string(),
            followed_path: true,
            exit_reason: Some("target_hit".to_string()),
            notes: None,
        };
        let analyze_seed = StructuralPriorSeed {
            source_label: "analyze".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.01,
        };
        let mut feedback = sample_feedback();
        feedback.source = "structural_feedback_submission".to_string();
        feedback.structural_feedback = Some(refs.clone());

        let mut state = LearningState::default();
        state.apply_structural_prior_seed(&refs, &analyze_seed);
        state.apply_structural_feedback(&[feedback]);

        let path = state
            .structural_prior_state
            .paths
            .get("path-live")
            .expect("path prior state");
        assert!(path.source_panel_summaries.contains_key("analyze"));
        assert!(path
            .source_panel_summaries
            .contains_key("structural_feedback_submission"));
        assert_eq!(
            path.source_panel_summaries["structural_feedback_submission"].wins,
            1
        );
        let posterior = state
            .structural_prior_state
            .source_reliability_posteriors
            .get("structural_feedback_submission")
            .expect("live source reliability posterior");
        let cell = posterior
            .outcome_confusion
            .get("win->positive_executed")
            .expect("live win confusion cell");
        assert_eq!(cell.observations, 1);
        assert!((cell.weighted_success_mass - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_structural_prior_seed_rebuilds_node_duration_priors() {
        let mut state = LearningState::default();
        let seed = StructuralPriorSeed {
            source_label: "analyze".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.01,
        };

        for (recommendation_id, recommended_at, node_id, branch_id) in [
            (
                "rec-1",
                "2026-04-30T00:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-2",
                "2026-04-30T01:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-3",
                "2026-04-30T02:00:00Z",
                "NQ:belief_regime_node:range",
                "NQ:belief_regime_node:range:range_mean_reversion",
            ),
            (
                "rec-4",
                "2026-04-30T03:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
        ] {
            state.apply_structural_prior_seed(
                &StructuralFeedbackRefs {
                    protocol_version: "structural-prior-seed-v1".to_string(),
                    recommendation_id: recommendation_id.to_string(),
                    recommended_at: recommended_at.to_string(),
                    node_id: node_id.to_string(),
                    branch_id: branch_id.to_string(),
                    scenario_id: format!("scenario:{branch_id}"),
                    path_id: format!("path:scenario:{branch_id}:primary"),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                },
                &seed,
            );
        }

        let trend = state
            .structural_prior_state
            .node_duration_priors
            .get("NQ:belief_regime_node:trend")
            .expect("trend duration prior");
        let range = state
            .structural_prior_state
            .node_duration_priors
            .get("NQ:belief_regime_node:range")
            .expect("range duration prior");

        assert_eq!(trend.observations, 3);
        assert_eq!(trend.streak_count, 2);
        assert!(trend.weighted_streak_mass > range.weighted_streak_mass);
        assert_eq!(trend.max_streak_length, 2);
        assert!((trend.avg_streak_length - 1.5).abs() < 1e-9);
        assert!(trend.expected_dwell_steps > range.expected_dwell_steps);
        assert!(trend.remaining_dwell_steps >= 0.0);
        assert!(trend.break_hazard > 0.0);
        assert_eq!(trend.duration_distribution.len(), 2);
        assert_eq!(trend.duration_distribution[0].dwell_steps, 1);
        assert_eq!(trend.duration_distribution[0].streak_count, 1);
        assert!((trend.duration_distribution[0].probability - (1.0 / 1.85)).abs() < 1e-9);
        assert_eq!(trend.duration_distribution[1].dwell_steps, 2);
        assert!((trend.duration_distribution[1].probability - (0.85 / 1.85)).abs() < 1e-9);
        assert!(trend.duration_distribution_entropy > 0.0);
        assert!((trend.empirical_duration_survival - 1.0).abs() < 1e-9);
        assert!((trend.empirical_duration_completion_hazard - (1.0 / 1.85)).abs() < 1e-9);
        assert!(trend.bocpd_evidence_weight > 0.0);
        assert!(trend.bocpd_evidence_weight < 1.0);
        assert!(trend.bocpd_raw_break_probability > 0.0);
        assert_eq!(trend.bocpd_run_length_mode, 1);
        assert!((trend.bocpd_run_length_mode_probability - (1.0 / 1.85)).abs() < 1e-9);
        assert!((trend.bocpd_run_length_tail_probability - 1.0).abs() < 1e-9);
        assert!((trend.bocpd_run_length_observation_mass - 1.85).abs() < 1e-9);
        assert!(trend.bocpd_recursive_reset_probability > trend.bocpd_break_probability);
        assert_eq!(trend.bocpd_recursive_run_length_mode, 0);
        let expected_recursive_fit = structural_node_bocpd_recursive_run_length_fit(
            &trend.duration_distribution,
            trend.bocpd_evidence_weight,
            trend.bocpd_break_probability,
        );
        assert_eq!(
            trend.bocpd_recursive_run_length_mode,
            expected_recursive_fit.run_length_mode
        );
        assert!(
            (trend.bocpd_recursive_run_length_mode_probability
                - expected_recursive_fit.run_length_mode_probability)
                .abs()
                < 1e-9
        );
        assert!(trend.bocpd_recursive_run_length_expected_value > 0.0);
        assert!(trend.bocpd_recursive_run_length_entropy > 0.0);
        assert!(trend.bocpd_sequence_change_intensity > 0.0);
        assert!(trend.bocpd_sequence_break_probability > 0.0);
        assert!(trend.bocpd_sequence_recursive_reset_probability > 0.0);
        assert!(trend.bocpd_sequence_recursive_run_length_mode_probability > 0.0);
        assert!(trend.bocpd_sequence_recursive_run_length_expected_value >= 0.0);
        assert!(trend.bocpd_sequence_recursive_run_length_entropy >= 0.0);
        let parametric_break_hazard =
            structural_duration_break_hazard(trend.last_streak_length, trend.expected_dwell_steps);
        assert!(trend.bocpd_break_probability > 0.0);
        assert!(trend.bocpd_break_probability < 1.0);
        assert!(
            trend.bocpd_break_probability
                > parametric_break_hazard.min(trend.bocpd_raw_break_probability)
        );
        assert!(
            trend.bocpd_break_probability
                < parametric_break_hazard.max(trend.bocpd_raw_break_probability)
                || (trend.bocpd_break_probability
                    - parametric_break_hazard.max(trend.bocpd_raw_break_probability))
                .abs()
                    < 1e-9
        );
        assert!(
            (trend.bocpd_continue_probability - (1.0 - trend.bocpd_break_probability)).abs() < 1e-9
        );
        assert!(trend.sticky_self_transition_strength > 0.0);
        assert!(trend.persistence_prior > range.persistence_prior);
        assert_eq!(range.observations, 1);
        assert_eq!(range.streak_count, 1);
        let temporal = state
            .structural_prior_state
            .node_temporal_posteriors
            .get("NQ:belief_regime_node:trend")
            .expect("trend temporal posterior");
        assert_eq!(temporal.node_id, "NQ:belief_regime_node:trend");
        assert_eq!(temporal.streak_count, 2);
        assert_eq!(temporal.weighted_streak_mass, trend.weighted_streak_mass);
        assert_eq!(temporal.expected_dwell_steps, trend.expected_dwell_steps);
        assert_eq!(temporal.break_hazard, trend.break_hazard);
        assert_eq!(
            temporal.sticky_self_transition_strength,
            trend.sticky_self_transition_strength
        );
        assert_eq!(
            temporal.bocpd_recursive_reset_probability,
            trend.bocpd_recursive_reset_probability
        );
        assert_eq!(
            temporal.bocpd_recursive_run_length_mode,
            trend.bocpd_recursive_run_length_mode
        );
        assert_eq!(
            temporal.bocpd_recursive_run_length_expected_value,
            trend.bocpd_recursive_run_length_expected_value
        );
        assert_eq!(
            temporal.bocpd_sequence_change_intensity,
            trend.bocpd_sequence_change_intensity
        );
        assert_eq!(
            temporal.bocpd_sequence_break_probability,
            trend.bocpd_sequence_break_probability
        );
        assert_eq!(
            temporal.bocpd_sequence_recursive_reset_probability,
            trend.bocpd_sequence_recursive_reset_probability
        );
        assert_eq!(
            temporal.bocpd_sequence_recursive_run_length_mode,
            trend.bocpd_sequence_recursive_run_length_mode
        );
        assert_eq!(
            temporal.bocpd_sequence_recursive_run_length_expected_value,
            trend.bocpd_sequence_recursive_run_length_expected_value
        );
        assert_eq!(
            temporal.temporal_posterior_support,
            trend.temporal_posterior_support
        );
    }

    #[test]
    fn test_structural_node_duration_priors_discount_older_streaks() {
        let mut state = LearningState::default();
        let seed = StructuralPriorSeed {
            source_label: "analyze".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.01,
        };

        for (recommendation_id, recommended_at, node_id, branch_id) in [
            (
                "rec-1",
                "2026-04-30T00:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-2",
                "2026-04-30T01:00:00Z",
                "NQ:belief_regime_node:range",
                "NQ:belief_regime_node:range:range_mean_reversion",
            ),
            (
                "rec-3",
                "2026-04-30T02:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-4",
                "2026-04-30T03:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
        ] {
            state.apply_structural_prior_seed(
                &StructuralFeedbackRefs {
                    protocol_version: "structural-prior-seed-v1".to_string(),
                    recommendation_id: recommendation_id.to_string(),
                    recommended_at: recommended_at.to_string(),
                    node_id: node_id.to_string(),
                    branch_id: branch_id.to_string(),
                    scenario_id: format!("scenario:{branch_id}"),
                    path_id: format!("path:scenario:{branch_id}:primary"),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                },
                &seed,
            );
        }

        let trend = state
            .structural_prior_state
            .node_duration_priors
            .get("NQ:belief_regime_node:trend")
            .expect("trend duration prior");
        let range = state
            .structural_prior_state
            .node_duration_priors
            .get("NQ:belief_regime_node:range")
            .expect("range duration prior");

        assert_eq!(trend.streak_count, 2);
        assert_eq!(range.streak_count, 1);
        assert!(trend.weighted_streak_mass > 1.0);
        assert!(trend.weighted_streak_mass > range.weighted_streak_mass);
        assert!(trend.persistence_prior > range.persistence_prior);
    }

    #[test]
    fn test_structural_node_duration_outcome_support_penalizes_recent_negative_streaks() {
        let mut state = LearningState::default();
        let mut baseline_state = LearningState::default();
        let positive_seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.02,
        };
        let negative_seed = StructuralPriorSeed {
            wins: 0,
            losses: 1,
            avg_pnl: -0.02,
            ..positive_seed.clone()
        };

        state.apply_structural_prior_seed(
            &StructuralFeedbackRefs {
                protocol_version: "structural-prior-seed-v1".to_string(),
                recommendation_id: "rec-1".to_string(),
                recommended_at: "2026-04-30T00:00:00Z".to_string(),
                node_id: "node-duration-quality".to_string(),
                branch_id: "branch-duration-quality:trend".to_string(),
                scenario_id: "scenario:branch-duration-quality:trend".to_string(),
                path_id: "path:scenario:branch-duration-quality:trend:primary".to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            },
            &positive_seed,
        );
        baseline_state.apply_structural_prior_seed(
            &StructuralFeedbackRefs {
                protocol_version: "structural-prior-seed-v1".to_string(),
                recommendation_id: "rec-1".to_string(),
                recommended_at: "2026-04-30T00:00:00Z".to_string(),
                node_id: "node-duration-quality".to_string(),
                branch_id: "branch-duration-quality:trend".to_string(),
                scenario_id: "scenario:branch-duration-quality:trend".to_string(),
                path_id: "path:scenario:branch-duration-quality:trend:primary".to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            },
            &positive_seed,
        );
        state.apply_structural_prior_seed(
            &StructuralFeedbackRefs {
                protocol_version: "structural-prior-seed-v1".to_string(),
                recommendation_id: "rec-2".to_string(),
                recommended_at: "2026-04-30T01:00:00Z".to_string(),
                node_id: "other-node".to_string(),
                branch_id: "other-branch".to_string(),
                scenario_id: "scenario:other-branch".to_string(),
                path_id: "path:scenario:other-branch:primary".to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            },
            &positive_seed,
        );
        state.apply_structural_prior_seed(
            &StructuralFeedbackRefs {
                protocol_version: "structural-prior-seed-v1".to_string(),
                recommendation_id: "rec-3".to_string(),
                recommended_at: "2026-04-30T02:00:00Z".to_string(),
                node_id: "node-duration-quality".to_string(),
                branch_id: "branch-duration-quality:trend".to_string(),
                scenario_id: "scenario:branch-duration-quality:trend".to_string(),
                path_id: "path:scenario:branch-duration-quality:trend:primary".to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            },
            &negative_seed,
        );
        baseline_state.apply_structural_prior_seed(
            &StructuralFeedbackRefs {
                protocol_version: "structural-prior-seed-v1".to_string(),
                recommendation_id: "rec-3".to_string(),
                recommended_at: "2026-04-30T02:00:00Z".to_string(),
                node_id: "node-duration-quality".to_string(),
                branch_id: "branch-duration-quality:trend".to_string(),
                scenario_id: "scenario:branch-duration-quality:trend".to_string(),
                path_id: "path:scenario:branch-duration-quality:trend:primary".to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            },
            &positive_seed,
        );

        let prior = state
            .structural_prior_state
            .node_duration_priors
            .get("node-duration-quality")
            .expect("node duration prior");
        let baseline = baseline_state
            .structural_prior_state
            .node_duration_priors
            .get("node-duration-quality")
            .expect("baseline node duration prior");
        assert!(prior.weighted_success_mass > 0.0);
        assert!(prior.weighted_failure_mass > 0.0);
        assert!(prior.duration_outcome_support < baseline.duration_outcome_support);
        assert!(prior.bocpd_break_probability > baseline.bocpd_break_probability);
    }

    #[test]
    fn test_structural_prior_seed_rebuilds_branch_transition_priors() {
        let mut state = LearningState::default();
        let seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.02,
        };

        for (recommendation_id, recommended_at, node_id, branch_id) in [
            (
                "rec-a",
                "2026-04-30T00:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-b",
                "2026-04-30T01:00:00Z",
                "NQ:belief_regime_node:range",
                "NQ:belief_regime_node:range:range_mean_reversion",
            ),
            (
                "rec-c",
                "2026-04-30T02:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-d",
                "2026-04-30T03:00:00Z",
                "NQ:belief_regime_node:transition",
                "NQ:belief_regime_node:transition:transition_confirmation",
            ),
        ] {
            state.apply_structural_prior_seed(
                &StructuralFeedbackRefs {
                    protocol_version: "structural-prior-seed-v1".to_string(),
                    recommendation_id: recommendation_id.to_string(),
                    recommended_at: recommended_at.to_string(),
                    node_id: node_id.to_string(),
                    branch_id: branch_id.to_string(),
                    scenario_id: format!("scenario:{branch_id}"),
                    path_id: format!("path:scenario:{branch_id}:primary"),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                },
                &seed,
            );
        }

        let transition_ab = state
            .structural_prior_state
            .branch_transition_priors
            .get(
                "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:range:range_mean_reversion",
            )
            .expect("trend to range transition");
        let transition_ac = state
            .structural_prior_state
            .branch_transition_priors
            .get(
                "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation",
            )
            .expect("trend to transition transition");

        assert_eq!(transition_ab.observations, 1);
        assert_eq!(transition_ac.observations, 1);
        assert!(transition_ac.weighted_observation_mass > transition_ab.weighted_observation_mass);
        assert!(transition_ac.transition_prior > transition_ab.transition_prior);
        let temporal = state
            .structural_prior_state
            .branch_temporal_posteriors
            .get(
                "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation",
            )
            .expect("transition temporal posterior");
        assert_eq!(
            temporal.transition_key,
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation"
        );
        assert_eq!(temporal.observations, transition_ac.observations);
        assert_eq!(
            temporal.temporal_posterior_support,
            transition_ac.temporal_posterior_support
        );
        let node_temporal = state
            .structural_prior_state
            .node_transition_posteriors
            .get("NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition")
            .expect("node transition temporal posterior");
        assert_eq!(
            node_temporal.transition_key,
            "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition"
        );
        assert_eq!(node_temporal.from_node_id, "NQ:belief_regime_node:trend");
        assert_eq!(node_temporal.to_node_id, "NQ:belief_regime_node:transition");
        assert!(node_temporal.weighted_observation_mass > 0.0);
        assert!(node_temporal.normalized_transition_posterior > 0.0);
    }

    #[test]
    fn test_structural_transition_priors_discount_older_transitions() {
        let mut state = LearningState::default();
        let seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.02,
        };

        for (recommendation_id, recommended_at, node_id, branch_id) in [
            (
                "rec-1",
                "2026-04-30T00:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-2",
                "2026-04-30T01:00:00Z",
                "NQ:belief_regime_node:range",
                "NQ:belief_regime_node:range:range_mean_reversion",
            ),
            (
                "rec-3",
                "2026-04-30T02:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-4",
                "2026-04-30T03:00:00Z",
                "NQ:belief_regime_node:transition",
                "NQ:belief_regime_node:transition:transition_confirmation",
            ),
            (
                "rec-5",
                "2026-04-30T04:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-6",
                "2026-04-30T05:00:00Z",
                "NQ:belief_regime_node:transition",
                "NQ:belief_regime_node:transition:transition_confirmation",
            ),
        ] {
            state.apply_structural_prior_seed(
                &StructuralFeedbackRefs {
                    protocol_version: "structural-prior-seed-v1".to_string(),
                    recommendation_id: recommendation_id.to_string(),
                    recommended_at: recommended_at.to_string(),
                    node_id: node_id.to_string(),
                    branch_id: branch_id.to_string(),
                    scenario_id: format!("scenario:{branch_id}"),
                    path_id: format!("path:scenario:{branch_id}:primary"),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                },
                &seed,
            );
        }

        let trend_to_range = state
            .structural_prior_state
            .branch_transition_priors
            .get(
                "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:range:range_mean_reversion",
            )
            .expect("trend to range transition");
        let trend_to_transition = state
            .structural_prior_state
            .branch_transition_priors
            .get(
                "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation",
            )
            .expect("trend to transition transition");

        assert_eq!(trend_to_range.observations, 1);
        assert_eq!(trend_to_transition.observations, 2);
        assert!(
            trend_to_transition.weighted_observation_mass
                > trend_to_range.weighted_observation_mass
        );
        assert!(trend_to_transition.transition_prior > trend_to_range.transition_prior);
    }

    #[test]
    fn test_repeated_branch_evidence_strengthens_without_collapsing_unrelated_nodes() {
        let mut state = LearningState::default();
        let seed = StructuralPriorSeed {
            source_label: "backtest".to_string(),
            tempering_coefficient: None,
            observations: 1,
            followed_count: 1,
            wins: 1,
            losses: 0,
            breakevens: 0,
            invalidated: 0,
            abandoned: 0,
            not_followed: 0,
            avg_pnl: 0.02,
        };

        for (recommendation_id, recommended_at, node_id, branch_id) in [
            (
                "rec-1",
                "2026-04-30T00:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-2",
                "2026-04-30T01:00:00Z",
                "NQ:belief_regime_node:range",
                "NQ:belief_regime_node:range:range_mean_reversion",
            ),
            (
                "rec-3",
                "2026-04-30T02:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-4",
                "2026-04-30T03:00:00Z",
                "NQ:belief_regime_node:transition",
                "NQ:belief_regime_node:transition:transition_confirmation",
            ),
            (
                "rec-5",
                "2026-04-30T04:00:00Z",
                "NQ:belief_regime_node:trend",
                "NQ:belief_regime_node:trend:trend_follow_through",
            ),
            (
                "rec-6",
                "2026-04-30T05:00:00Z",
                "NQ:belief_regime_node:transition",
                "NQ:belief_regime_node:transition:transition_confirmation",
            ),
        ] {
            state.apply_structural_prior_seed(
                &StructuralFeedbackRefs {
                    protocol_version: "structural-prior-seed-v1".to_string(),
                    recommendation_id: recommendation_id.to_string(),
                    recommended_at: recommended_at.to_string(),
                    node_id: node_id.to_string(),
                    branch_id: branch_id.to_string(),
                    scenario_id: format!("scenario:{branch_id}"),
                    path_id: format!("path:scenario:{branch_id}:primary"),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                },
                &seed,
            );
        }

        let trend_to_range_key =
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:range:range_mean_reversion";
        let trend_to_transition_key =
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation";
        let range_to_trend_key =
            "NQ:belief_regime_node:range:range_mean_reversion=>NQ:belief_regime_node:trend:trend_follow_through";
        let trend_to_range = state
            .structural_prior_state
            .branch_temporal_posteriors
            .get(trend_to_range_key)
            .expect("trend to range posterior");
        let trend_to_transition = state
            .structural_prior_state
            .branch_temporal_posteriors
            .get(trend_to_transition_key)
            .expect("trend to transition posterior");
        let range_to_trend = state
            .structural_prior_state
            .branch_temporal_posteriors
            .get(range_to_trend_key)
            .expect("range to trend posterior");

        assert!(trend_to_transition.transition_prior > trend_to_range.transition_prior);
        assert!(
            trend_to_transition.normalized_transition_posterior
                > trend_to_range.normalized_transition_posterior
        );
        assert!(
            (trend_to_transition.normalized_transition_posterior
                + trend_to_range.normalized_transition_posterior
                - 1.0)
                .abs()
                < 1e-9
        );
        let node_trend_to_transition = state
            .structural_prior_state
            .node_transition_posteriors
            .get("NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition")
            .expect("trend to transition node posterior");
        let node_trend_to_range = state
            .structural_prior_state
            .node_transition_posteriors
            .get("NQ:belief_regime_node:trend=>NQ:belief_regime_node:range")
            .expect("trend to range node posterior");
        assert!(
            node_trend_to_transition.normalized_transition_posterior
                > node_trend_to_range.normalized_transition_posterior
        );
        assert!(
            (node_trend_to_transition.normalized_transition_posterior
                + node_trend_to_range.normalized_transition_posterior
                - 1.0)
                .abs()
                < 1e-9
        );
        assert!((range_to_trend.normalized_transition_posterior - 1.0).abs() < 1e-9);
        assert!(range_to_trend.weighted_observation_mass > 0.0);
    }

    #[test]
    fn test_scorecard_assigns_keep_vs_replace_actions() {
        let keep = ranking(RankingInput {
            name: "trend_momentum",
            mean_ic: 0.08,
            ir: 1.2,
            backtest_return: 0.14,
            sharpe: 1.4,
            stability: 0.72,
            win_rate: 0.58,
            profit_factor: 1.35,
            trade_count: 18,
        });
        let replace = ranking(RankingInput {
            name: "weak_factor",
            mean_ic: 0.01,
            ir: 0.1,
            backtest_return: -0.03,
            sharpe: -0.2,
            stability: 0.20,
            win_rate: 0.42,
            profit_factor: 0.82,
            trade_count: 14,
        });

        assert_eq!(keep.iteration_action, "keep");
        assert!(keep.composite_score > replace.composite_score);
        assert_eq!(replace.iteration_action, "replace");
        assert!(replace.replacement_candidate);
    }

    #[test]
    fn test_iteration_queue_prioritizes_low_scoring_replace_items() {
        let state = LearningState {
            factor_rankings: vec![
                ranking(RankingInput {
                    name: "good_factor",
                    mean_ic: 0.07,
                    ir: 1.0,
                    backtest_return: 0.12,
                    sharpe: 1.1,
                    stability: 0.70,
                    win_rate: 0.55,
                    profit_factor: 1.28,
                    trade_count: 16,
                }),
                ranking(RankingInput {
                    name: "bad_factor",
                    mean_ic: 0.01,
                    ir: 0.0,
                    backtest_return: -0.04,
                    sharpe: -0.1,
                    stability: 0.25,
                    win_rate: 0.43,
                    profit_factor: 0.80,
                    trade_count: 14,
                }),
            ],
            ..LearningState::default()
        };

        let queue = state.iteration_queue();
        assert_eq!(queue[0].factor_name, "bad_factor");
        assert_eq!(queue[0].iteration_action, "replace");
    }

    #[test]
    fn test_family_decisions_group_by_factor_family() {
        let mut state = LearningState::default();
        let mut a = ranking(RankingInput {
            name: "trend_momentum",
            mean_ic: 0.08,
            ir: 1.2,
            backtest_return: 0.10,
            sharpe: 1.2,
            stability: 0.70,
            win_rate: 0.56,
            profit_factor: 1.3,
            trade_count: 15,
        });
        let mut b = ranking(RankingInput {
            name: "structure_ict",
            mean_ic: 0.02,
            ir: 0.3,
            backtest_return: -0.02,
            sharpe: 0.1,
            stability: 0.30,
            win_rate: 0.45,
            profit_factor: 0.95,
            trade_count: 12,
        });
        a.iteration_action = "keep".to_string();
        b.iteration_action = "replace".to_string();
        b.replacement_candidate = true;
        state.factor_rankings = vec![a, b];

        let families = state.family_decisions();
        assert_eq!(families.len(), 2);
        assert!(families
            .iter()
            .any(|family| family.family == "structure_ict"
                && !family.replacement_candidates.is_empty()
                && family.decision_status == "review_replace"
                && family.dominant_action == "replace"
                && family
                    .risk_flags
                    .iter()
                    .any(|flag| flag == "contains_replacement_candidates")));
    }
}
