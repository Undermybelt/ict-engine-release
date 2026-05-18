use serde::{Deserialize, Serialize};

use crate::belief_core::ranking_label::StructuralPathRankerRuntimeSurface;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPlaybookBundle {
    pub artifact_version: String,
    pub symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_profile_data_contracts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_profile_track_statuses: Vec<String>,
    pub node: StructuralNodeArtifact,
    pub branch_set: StructuralBranchSetArtifact,
    pub scenario_playbook: StructuralScenarioPlaybookArtifact,
    pub path_plan: StructuralPathPlanArtifact,
    pub history_summary: StructuralHistorySummaryArtifact,
    pub node_history: StructuralNodeHistoryArtifact,
    pub branch_history: StructuralBranchHistoryArtifact,
    pub scenario_history: StructuralScenarioHistoryArtifact,
    pub path_history: StructuralPathHistoryArtifact,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_path_bundle: Option<StructuralRecommendedPathBundleArtifact>,
    pub feedback_template: StructuralFeedbackTemplateArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralNodeArtifact {
    pub node_id: String,
    pub node_family: String,
    pub node_label: String,
    pub focus_phase: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub market_context: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timeframe_scope: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supporting_evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalidating_evidence: Vec<String>,
    pub belief_prior: f64,
    pub belief_posterior: f64,
    pub posterior_confidence: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origin_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBranchSetArtifact {
    pub from_node_id: String,
    pub branches: Vec<StructuralBranchArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBranchArtifact {
    pub branch_id: String,
    pub target_node_id: String,
    pub branch_label: String,
    pub prior_probability: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_prior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_weighted_observation_mass: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_outcome_support: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_temporal_posterior_support: Option<f64>,
    pub posterior_probability: f64,
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
    pub composite_branch_score: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activation_conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supporting_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralScenarioPlaybookArtifact {
    pub scenarios: Vec<StructuralScenarioArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralScenarioArtifact {
    pub scenario_id: String,
    pub branch_id: String,
    pub scenario_label: String,
    pub narrative: String,
    pub prior_probability: f64,
    pub posterior_probability: f64,
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
    pub composite_scenario_score: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_confirmations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hard_invalidations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timing_constraints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathHistoryArtifact {
    pub summary: StructuralPathHistorySummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<StructuralPathOutcomeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralHistorySummaryArtifact {
    pub total_records: usize,
    pub distinct_nodes: usize,
    pub distinct_branches: usize,
    pub distinct_scenarios: usize,
    pub distinct_paths: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_branch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_scenario_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_path_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralNodeHistoryArtifact {
    pub summary: StructuralEntityHistorySummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<StructuralNodeOutcomeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBranchHistoryArtifact {
    pub summary: StructuralEntityHistorySummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<StructuralBranchOutcomeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralScenarioHistoryArtifact {
    pub summary: StructuralEntityHistorySummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<StructuralScenarioOutcomeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralEntityHistorySummary {
    pub total_records: usize,
    pub distinct_entities: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralNodeOutcomeSummary {
    pub node_id: String,
    pub total_records: usize,
    pub followed_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub not_followed: usize,
    pub avg_pnl: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_propensity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off_policy_exposure_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_realized_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBranchOutcomeSummary {
    pub node_id: String,
    pub branch_id: String,
    pub total_records: usize,
    pub followed_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub not_followed: usize,
    pub avg_pnl: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_propensity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off_policy_exposure_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_realized_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralScenarioOutcomeSummary {
    pub node_id: String,
    pub branch_id: String,
    pub scenario_id: String,
    pub total_records: usize,
    pub followed_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub not_followed: usize,
    pub avg_pnl: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_propensity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off_policy_exposure_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_realized_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathHistorySummary {
    pub total_records: usize,
    pub distinct_paths: usize,
    pub distinct_branches: usize,
    pub distinct_scenarios: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_path_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathOutcomeSummary {
    pub node_id: String,
    pub branch_id: String,
    pub scenario_id: String,
    pub path_id: String,
    pub total_records: usize,
    pub followed_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub invalidated: usize,
    pub abandoned: usize,
    pub not_followed: usize,
    pub avg_pnl: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_propensity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off_policy_exposure_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recommended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_realized_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralFeedbackTemplateArtifact {
    pub protocol_version: String,
    pub recommendation_id: String,
    pub recommended_at: String,
    pub symbol: String,
    pub node_id: String,
    pub branch_id: String,
    pub scenario_id: String,
    pub path_id: String,
    pub candidate_set_id: String,
    pub candidate_set_size: usize,
    pub selected_path_probability: f64,
    pub direction: String,
    pub entry_style: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_entry_quality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_entry_quality_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_bayes_gate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_posterior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbn_support_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_outcomes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feedback_fields: Vec<StructuralFeedbackField>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralFeedbackField {
    pub field_id: String,
    pub label: String,
    pub value_type: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralFeedbackSubmission {
    pub protocol_version: String,
    pub recommendation_id: String,
    pub recommended_at: String,
    pub symbol: String,
    pub node_id: String,
    pub branch_id: String,
    pub scenario_id: String,
    pub path_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_set_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_set_size: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_path_probability: Option<f64>,
    pub direction: String,
    pub entry_style: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_entry_quality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_entry_quality_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_bayes_gate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_posterior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbn_support_score: Option<f64>,
    pub followed_path: bool,
    pub realized_outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realized_pnl: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathPlanArtifact {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_data_contracts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_provider_tracks: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub candidate_set_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_paths: Vec<StructuralPathArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_runtime: Option<StructuralPathRankerRuntimeSurface>,
    pub paths: Vec<StructuralPathArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralTopPathCandidatesArtifact {
    pub symbol: String,
    pub candidate_set_id: String,
    pub candidate_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_runtime: Option<StructuralPathRankerRuntimeSurface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<StructuralTopPathCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralRecommendedPathBundleArtifact {
    pub symbol: String,
    pub rank: usize,
    pub candidate_set_id: String,
    pub candidate_set_size: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_runtime: Option<StructuralPathRankerRuntimeSurface>,
    pub selected_path_probability: f64,
    pub path_id: String,
    pub scenario_id: String,
    pub path_label: String,
    pub direction: String,
    pub experience_prior: f64,
    pub current_posterior: f64,
    pub composite_score: f64,
    #[serde(default)]
    pub historical_total_records: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_invalidation_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_raw_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_calibrated_path_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_path_prob_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_execution_gate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_runtime_source: Option<String>,
    pub why_this_path: String,
    pub trigger_summary: String,
    pub confirmation_summary: String,
    pub stop_summary: String,
    pub invalidation_summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralTopPathCandidate {
    pub rank: usize,
    pub candidate_set_id: String,
    pub behavior_policy_probability: f64,
    pub path_id: String,
    pub scenario_id: String,
    pub path_label: String,
    pub direction: String,
    pub experience_prior: f64,
    pub current_posterior: f64,
    pub composite_score: f64,
    #[serde(default)]
    pub historical_total_records: usize,
    #[serde(default)]
    pub historical_followed_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_invalidation_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_raw_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_calibrated_path_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_path_prob_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_execution_gate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_runtime_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralPathArtifact {
    pub path_id: String,
    pub scenario_id: String,
    pub path_label: String,
    pub direction: String,
    pub entry_style: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_entry_quality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_entry_quality_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_bayes_gate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_timeframe_direction_bias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_candidate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_candidate_artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_readiness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prediction_edge_share: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_edge_share: Option<f64>,
    #[serde(default)]
    pub historical_total_records: usize,
    #[serde(default)]
    pub historical_followed_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_propensity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_win_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_invalidation_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_avg_pnl: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trigger_conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub confirmation_conditions: Vec<String>,
    pub stop_definition: String,
    pub target_definition: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalidation_conditions: Vec<String>,
    pub expected_failure_mode: String,
    pub max_time_in_trade: String,
    pub path_prior: f64,
    pub path_posterior: f64,
    pub bbn_support_score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catboost_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_calibrated_path_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_path_prob_lower_bound: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_execution_gate_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_ranker_runtime_source: Option<String>,
    pub composite_preference_score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_command: Option<String>,
}
