use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::application::orchestration::{
    apply_structural_path_ranking_external_scores,
    evaluate_structural_path_probability_calibration_rows,
    export_structural_path_ranking_target_with_agent_material_rank,
    StructuralPathProbabilityCalibrationEvaluationReport, StructuralPathRankingExternalScoreInput,
    StructuralPathRankingTargetExportSummary, StructuralPathRankingTargetRow,
    StructuralPathRankingTrainerManifest, STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE,
};
use crate::application::provider_catalog::provider_status_agent_surface;
use crate::belief_core::ranking_label::{
    load_structural_path_ranker_direct_model_artifact,
    load_structural_path_ranker_runtime_artifact_rows,
    load_structural_path_ranking_runtime_selection,
    score_structural_path_ranker_runtime_rows_with_direct_model,
    score_structural_path_ranker_runtime_rows_with_explicit_family,
    score_structural_path_ranker_runtime_rows_with_service,
    structural_path_ranker_supports_direct_model_family,
    structural_path_ranker_supports_explicit_family,
    structural_path_ranker_supports_service_family, structural_path_ranking_runtime_selection_path,
    StructuralPathRankerCalibrationMetrics, StructuralPathRankerRule, StructuralPathRankerTreeNode,
    StructuralPathRankerValidationMetrics, StructuralPathRankingRuntimeSelection,
    STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
    STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY,
    STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_PROTOCOL_VERSION,
};
#[cfg(test)]
use crate::state::{append_artifact_ledger_entry, ArtifactLedgerEntry};
use crate::state::{
    load_artifact_ledger, load_learning_state, load_pending_update_history, load_state_or_default,
    load_workflow_snapshot, save_text_state, structural_feedback_counter_outcome,
    structural_feedback_outcome_is_unresolved, AnalyzeRunRecord, UpdateRunRecord,
    ANALYZE_RUNS_FILE, PENDING_UPDATE_ARTIFACT_FILE, UPDATE_RUNS_FILE,
};
use crate::types::RegimeProbs;

use super::{
    bin_breaker_rb_for_bbn, bin_cisd_rb_for_bbn, build_breaker_rb_catboost_feature_row,
    build_cisd_rb_catboost_feature_row, decode_entry_model_packet, entry_model_providers,
    BreakerRbBbnEvidence, BreakerRbEntryModelPacket, CisdRbBbnEvidence, CisdRbEntryModelPacket,
    ConsumerDefaultMode, EntryModelProvider, EntryModelTrainingRows, BREAKER_RB_SETUP_MODEL_ID,
    CISD_RB_SETUP_MODEL_ID,
};

pub const POLICY_TRAINING_DIR: &str = "policy_training";
pub const CISD_RB_BBN_TRAINING_FILE: &str = "cisd_rb_bbn_training.csv";
pub const CISD_RB_CATBOOST_TRAINING_FILE: &str = "cisd_rb_catboost_training.csv";
pub const CISD_RB_TRAINING_SUMMARY_FILE: &str = "cisd_rb_training_export_summary.json";
pub const BREAKER_RB_BBN_TRAINING_FILE: &str = "breaker_rb_bbn_training.csv";
pub const BREAKER_RB_CATBOOST_TRAINING_FILE: &str = "breaker_rb_catboost_training.csv";
pub const BREAKER_RB_TRAINING_SUMMARY_FILE: &str = "breaker_rb_training_export_summary.json";
pub const STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE: &str =
    "structural_path_ranking_trainer_artifact.json";
const STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION: &str =
    "structural-path-ranking-trainer-artifact-v1";
const STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct CisdRbCollectedTrainingRows {
    analyze_runs: usize,
    update_runs: usize,
    bbn_rows: Vec<CisdRbBbnTrainingRow>,
    catboost_rows: Vec<CisdRbCatBoostTrainingRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct BreakerCollectedTrainingRows {
    analyze_runs: usize,
    update_runs: usize,
    bbn_rows: Vec<BreakerRbBbnTrainingRow>,
    catboost_rows: Vec<BreakerRbCatBoostTrainingRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CisdRbBbnTrainingRow {
    pub analyze_run_id: String,
    pub update_run_id: String,
    pub symbol: String,
    pub timeframe: String,
    pub setup_model_id: String,
    pub trend_alignment: String,
    pub liquidity_interaction_quality: String,
    pub trigger_confirmation_quality: String,
    pub session_quality: String,
    pub entry_quality: String,
    pub evidence_quality_score: f64,
    pub gating_status: String,
    pub realized_outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CisdRbCatBoostTrainingRow {
    pub analyze_run_id: String,
    pub update_run_id: String,
    pub symbol: String,
    pub timeframe: String,
    pub setup_model_id: String,
    pub setup_progress_state: String,
    pub hmm_accumulation_prob: f64,
    pub hmm_manipulation_expansion_prob: f64,
    pub hmm_distribution_prob: f64,
    pub bbn_trend_alignment: String,
    pub bbn_liquidity_interaction_quality: String,
    pub bbn_trigger_confirmation_quality: String,
    pub bbn_session_quality: String,
    pub bbn_entry_quality: String,
    pub cisd_run_length_observed: f64,
    pub cisd_impulse_atr: f64,
    pub cisd_body_ratio_mean: f64,
    pub rb_wick_body_ratio: f64,
    pub rb_close_location_ratio: f64,
    pub bars_between_cisd_and_rb: f64,
    pub seq_window_hit: bool,
    pub ema19_distance_bps: f64,
    pub atr14_bps: f64,
    pub realized_vol_zscore: f64,
    pub evidence_quality_score: f64,
    pub session_label: String,
    pub realized_outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CisdRbTrainingExportSummary {
    pub symbol: String,
    pub analyze_runs: usize,
    pub update_runs: usize,
    pub matched_rows: usize,
    pub bbn_training_path: String,
    pub catboost_training_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BreakerRbTrainingExportSummary {
    pub symbol: String,
    pub analyze_runs: usize,
    pub update_runs: usize,
    pub matched_rows: usize,
    pub bbn_training_path: String,
    pub catboost_training_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct NumericRangeSummary {
    pub min: f64,
    pub max: f64,
    pub span: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbnTrainingStatusSurface {
    pub ready: bool,
    pub rows: usize,
    pub outcome_counts: BTreeMap<String, usize>,
    pub entry_quality_counts: BTreeMap<String, usize>,
    pub trigger_confirmation_quality_counts: BTreeMap<String, usize>,
    pub session_quality_counts: BTreeMap<String, usize>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CatBoostTrainingStatusSurface {
    pub ready: bool,
    pub rows: usize,
    pub outcome_counts: BTreeMap<String, usize>,
    pub numeric_ranges: BTreeMap<String, NumericRangeSummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CisdRbTrainingStatusSurface {
    pub symbol: String,
    pub analyze_runs: usize,
    pub update_runs: usize,
    pub matched_rows: usize,
    pub setup_model_ids: BTreeMap<String, usize>,
    pub bbn: BbnTrainingStatusSurface,
    pub catboost: CatBoostTrainingStatusSurface,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BreakerRbTrainingStatusSurface {
    pub symbol: String,
    pub analyze_runs: usize,
    pub update_runs: usize,
    pub matched_rows: usize,
    pub setup_model_ids: BTreeMap<String, usize>,
    pub bbn: BbnTrainingStatusSurface,
    pub catboost: CatBoostTrainingStatusSurface,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BreakerRbBbnTrainingRow {
    pub analyze_run_id: String,
    pub update_run_id: String,
    pub symbol: String,
    pub timeframe: String,
    pub setup_model_id: String,
    pub trend_alignment: String,
    pub breaker_retest_quality: String,
    pub session_quality: String,
    pub entry_quality: String,
    pub evidence_quality_score: f64,
    pub gating_status: String,
    pub realized_outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BreakerRbCatBoostTrainingRow {
    pub analyze_run_id: String,
    pub update_run_id: String,
    pub symbol: String,
    pub timeframe: String,
    pub setup_model_id: String,
    pub setup_progress_state: String,
    pub hmm_accumulation_prob: f64,
    pub hmm_manipulation_expansion_prob: f64,
    pub hmm_distribution_prob: f64,
    pub bbn_trend_alignment: String,
    pub bbn_breaker_retest_quality: String,
    pub bbn_session_quality: String,
    pub bbn_entry_quality: String,
    pub bars_between_violation_and_retest: f64,
    pub breaker_width_bps: f64,
    pub retest_reclaim_bps: f64,
    pub rb_wick_body_ratio: f64,
    pub rb_close_location_ratio: f64,
    pub ema19_distance_bps: f64,
    pub atr14_bps: f64,
    pub realized_vol_zscore: f64,
    pub evidence_quality_score: f64,
    pub session_label: String,
    pub realized_outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PolicyTrainingProviderStatusSurface {
    #[serde(rename = "entry_model_id")]
    pub provider_id: String,
    pub consumer_adopted_by_default: bool,
    pub consumer_effect: String,
    pub ready: bool,
    pub matched_rows: usize,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PolicyTrainingStatusSurface {
    pub symbol: String,
    pub analyze_runs: usize,
    pub update_runs: usize,
    #[serde(rename = "entry_models")]
    pub providers: Vec<PolicyTrainingProviderStatusSurface>,
    pub factor_candidate_packs: FactorCandidatePackTrainingStatusSurface,
    pub regime_confidence_assets: RegimeConfidenceAssetTrainingStatusSurface,
    pub structural_path_ranking_runtime: StructuralPathRankingRuntimeSummarySurface,
    pub structural_path_ranking_validation: StructuralPathRankingValidationSummarySurface,
    pub structural_path_ranking_target: StructuralPathRankingTargetTrainingStatusSurface,
    pub structural_path_ranking_runtime_summary: String,
    pub structural_path_ranking_validation_summary: String,
    pub factor_hotplug_summary: String,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FactorCandidatePackTrainingStatusSurface {
    pub inventory_ready: bool,
    pub inventory_status: String,
    pub candidate_pack_count: usize,
    pub preferred_density_count: usize,
    pub cross_market_candidate_count: usize,
    pub inventory_path: String,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RegimeConfidenceAssetTrainingStatusSurface {
    pub inventory_ready: bool,
    pub inventory_status: String,
    pub asset_count: usize,
    pub board_a_regime_gate_count: usize,
    pub direct_event_overlay_count: usize,
    pub diagnostic_after_source_control_unlock_count: usize,
    pub contrast_evidence_count: usize,
    pub recovered_not_candidate_pack_count: usize,
    pub promotion_allowed: bool,
    pub runtime_selection_enabled: bool,
    pub inventory_path: String,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingRuntimeSummarySurface {
    pub enabled: bool,
    pub ready: bool,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub active_match_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_artifact_uri: Option<String>,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingTargetRowValidationSurface {
    pub raw_scored_mature_rows: usize,
    pub raw_scored_mature_min_rows: usize,
    pub raw_scored_mature_shortfall_rows: usize,
    pub production_validation_ready: bool,
    pub production_validation_rows: usize,
    pub production_validation_min_rows: usize,
    pub production_validation_shortfall_rows: usize,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingFeedbackObservationValidationSurface {
    pub ready: bool,
    pub mature_observations: usize,
    pub min_observations: usize,
    pub shortfall_observations: usize,
    pub pending_observations: usize,
    pub total_observations: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub outcome_distribution: BTreeMap<String, usize>,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingValidationSummarySurface {
    pub calibration_ready: bool,
    pub calibration_quality_ready: bool,
    pub calibration_status: String,
    pub raw_scored_mature_rows: usize,
    pub raw_scored_mature_min_rows: usize,
    pub raw_scored_mature_shortfall_rows: usize,
    pub production_validation_ready: bool,
    pub production_validation_rows: usize,
    pub production_validation_min_rows: usize,
    pub production_validation_shortfall_rows: usize,
    #[serde(default)]
    pub observation_validation_ready: bool,
    #[serde(default)]
    pub observation_validation_rows: usize,
    #[serde(default)]
    pub observation_validation_min_rows: usize,
    #[serde(default)]
    pub observation_validation_shortfall_rows: usize,
    #[serde(default)]
    pub target_row_validation: StructuralPathRankingTargetRowValidationSurface,
    #[serde(default)]
    pub feedback_observation_validation: StructuralPathRankingFeedbackObservationValidationSurface,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingTargetTrainingStatusSurface {
    pub export_ready: bool,
    pub calibration_ready: bool,
    #[serde(default)]
    pub calibration_quality_ready: bool,
    #[serde(default)]
    pub trainer_manifest_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trainer_manifest_protocol_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trainer_manifest_dataset_role: Option<String>,
    #[serde(default)]
    pub trainer_feature_columns: usize,
    #[serde(default)]
    pub trainer_calibration_columns: usize,
    #[serde(default)]
    pub trainer_guardrail_columns: usize,
    #[serde(default)]
    pub trainer_artifact_ready: bool,
    #[serde(default)]
    pub trainer_artifact_status: String,
    #[serde(default)]
    pub trainer_artifact_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trainer_artifact_protocol_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trainer_artifact_dataset_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trainer_artifact_model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trainer_artifact_score_column: Option<String>,
    #[serde(default)]
    pub trainer_artifact_trained_rows: usize,
    #[serde(default)]
    pub trainer_artifact_history_rows: usize,
    #[serde(default)]
    pub trainer_artifact_calibration_rows: usize,
    #[serde(default)]
    pub trainer_artifact_feature_columns: usize,
    #[serde(default)]
    pub trainer_artifact_uri_present: bool,
    #[serde(default)]
    pub runtime_selection_enabled: bool,
    #[serde(default)]
    pub runtime_selection_ready: bool,
    #[serde(default)]
    pub runtime_selection_status: String,
    #[serde(default)]
    pub runtime_selection_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_selection_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_source_kind: Option<String>,
    #[serde(default)]
    pub runtime_active_match_count: usize,
    #[serde(default)]
    pub runtime_artifact_match_count: usize,
    #[serde(default)]
    pub runtime_candidate_set_match_count: usize,
    #[serde(default)]
    pub runtime_history_match_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_artifact_uri: Option<String>,
    pub rows: usize,
    pub candidate_set_id: Option<String>,
    pub candidate_set_size: usize,
    #[serde(default)]
    pub mature_rows: usize,
    pub rows_with_propensity_estimate: usize,
    pub rows_with_calibrated_path_prob: usize,
    #[serde(default)]
    pub rows_with_execution_gate_status: usize,
    #[serde(default)]
    pub rows_with_training_weight: usize,
    #[serde(default)]
    pub history_rows: usize,
    #[serde(default)]
    pub history_mature_rows: usize,
    #[serde(default)]
    pub history_rows_with_raw_path_score: usize,
    #[serde(default)]
    pub history_rows_with_calibrated_path_prob: usize,
    #[serde(default)]
    pub history_rows_with_path_prob_lower_bound: usize,
    #[serde(default)]
    pub history_rows_with_propensity_estimate: usize,
    #[serde(default)]
    pub history_rows_with_training_weight: usize,
    #[serde(default)]
    pub update_runs_with_structural_feedback: usize,
    #[serde(default)]
    pub feedback_rows_with_structural_feedback: usize,
    #[serde(default)]
    pub feedback_rows_total: usize,
    #[serde(default)]
    pub feedback_rows_without_structural_feedback: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_rows_without_structural_feedback_dominant_source: Option<String>,
    #[serde(default)]
    pub feedback_rows_without_structural_feedback_dominant_count: usize,
    #[serde(default)]
    pub feedback_rows_matured: usize,
    #[serde(default)]
    pub feedback_rows_pending: usize,
    #[serde(default)]
    pub pending_update_artifact_present: bool,
    #[serde(default)]
    pub pending_update_history_rows: usize,
    #[serde(default)]
    pub pending_update_templates_with_structural_feedback: usize,
    #[serde(default)]
    pub calibration_evaluation_rows: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_brier_score: Option<f64>,
    #[serde(default)]
    pub calibration_propensity_weighted_rows: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_propensity_weighted_brier_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_expected_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_max_error: Option<f64>,
    #[serde(default)]
    pub raw_scored_mature_rows: usize,
    #[serde(default)]
    pub raw_scored_mature_min_rows: usize,
    #[serde(default)]
    pub raw_scored_mature_shortfall_rows: usize,
    #[serde(default)]
    pub production_validation_ready: bool,
    #[serde(default)]
    pub production_validation_rows: usize,
    #[serde(default)]
    pub production_validation_min_rows: usize,
    #[serde(default)]
    pub production_validation_shortfall_rows: usize,
    #[serde(default)]
    pub observation_validation_ready: bool,
    #[serde(default)]
    pub observation_validation_rows: usize,
    #[serde(default)]
    pub observation_validation_min_rows: usize,
    #[serde(default)]
    pub observation_validation_shortfall_rows: usize,
    #[serde(default)]
    pub target_row_validation: StructuralPathRankingTargetRowValidationSurface,
    #[serde(default)]
    pub feedback_observation_validation: StructuralPathRankingFeedbackObservationValidationSurface,
    pub summary_path: String,
    pub csv_path: Option<String>,
    pub jsonl_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_csv_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_jsonl_path: Option<String>,
    pub warnings: Vec<String>,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StructuralPathRankingTrainerArtifact {
    #[serde(default)]
    pub protocol_version: String,
    #[serde(default)]
    pub dataset_role: String,
    #[serde(default)]
    pub model_family: String,
    #[serde(default)]
    pub artifact_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_artifact_uri: Option<String>,
    #[serde(default)]
    pub score_column: String,
    #[serde(default)]
    pub trained_rows: usize,
    #[serde(default)]
    pub history_rows: usize,
    #[serde(default)]
    pub calibration_rows: usize,
    #[serde(
        default,
        alias = "feature_columns",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub selected_features: Vec<String>,
    #[serde(default)]
    pub validation_metrics: StructuralPathRankerValidationMetrics,
    #[serde(default)]
    pub calibration_metrics: StructuralPathRankerCalibrationMetrics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_list: Vec<StructuralPathRankerRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_json: Option<StructuralPathRankerTreeNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CisdRbEntryModelProvider;

#[derive(Debug, Clone, Copy, Default)]
pub struct BreakerRbEntryModelProvider;

impl EntryModelProvider for CisdRbEntryModelProvider {
    fn provider_id(&self) -> &'static str {
        CISD_RB_SETUP_MODEL_ID
    }

    fn consumer_default_mode(&self) -> ConsumerDefaultMode {
        ConsumerDefaultMode::InternalTrainingOnly
    }

    fn build_analyze_packet(
        &self,
        symbol: &str,
        timeframe: &str,
        candles: &[crate::types::Candle],
        filter: &crate::state::PreBayesEvidenceFilter,
    ) -> Option<Value> {
        super::build_cisd_rb_entry_model_packet(symbol, timeframe, candles, filter)
            .and_then(|packet| serde_json::to_value(packet).ok())
    }

    fn training_rows(&self, state_dir: &str, symbol: &str) -> Result<EntryModelTrainingRows> {
        build_cisd_rb_training_rows(state_dir, symbol)
    }

    fn status_surface(
        &self,
        state_dir: &str,
        symbol: &str,
    ) -> Result<PolicyTrainingProviderStatusSurface> {
        let cisd_rb = cisd_rb_training_status(state_dir, symbol)?;
        Ok(PolicyTrainingProviderStatusSurface {
            provider_id: self.provider_id().to_string(),
            consumer_adopted_by_default: self.consumer_default_mode().adopted_by_default(),
            consumer_effect: self.consumer_default_mode().effect_label().to_string(),
            ready: cisd_rb.bbn.ready && cisd_rb.catboost.ready,
            matched_rows: cisd_rb.matched_rows,
            summary_line: cisd_rb.summary_line,
        })
    }
}

impl EntryModelProvider for BreakerRbEntryModelProvider {
    fn provider_id(&self) -> &'static str {
        BREAKER_RB_SETUP_MODEL_ID
    }

    fn consumer_default_mode(&self) -> ConsumerDefaultMode {
        ConsumerDefaultMode::InternalTrainingOnly
    }

    fn build_analyze_packet(
        &self,
        symbol: &str,
        timeframe: &str,
        candles: &[crate::types::Candle],
        filter: &crate::state::PreBayesEvidenceFilter,
    ) -> Option<Value> {
        super::build_breaker_rb_entry_model_packet(symbol, timeframe, candles, filter)
            .and_then(|packet| serde_json::to_value(packet).ok())
    }

    fn training_rows(&self, state_dir: &str, symbol: &str) -> Result<EntryModelTrainingRows> {
        build_breaker_rb_training_rows(state_dir, symbol)
    }

    fn status_surface(
        &self,
        state_dir: &str,
        symbol: &str,
    ) -> Result<PolicyTrainingProviderStatusSurface> {
        let breaker_rb = breaker_rb_training_status(state_dir, symbol)?;
        Ok(PolicyTrainingProviderStatusSurface {
            provider_id: self.provider_id().to_string(),
            consumer_adopted_by_default: self.consumer_default_mode().adopted_by_default(),
            consumer_effect: self.consumer_default_mode().effect_label().to_string(),
            ready: breaker_rb.bbn.ready && breaker_rb.catboost.ready,
            matched_rows: breaker_rb.matched_rows,
            summary_line: breaker_rb.summary_line,
        })
    }
}

pub fn export_cisd_rb_training_tables(
    state_dir: &str,
    symbol: &str,
) -> Result<CisdRbTrainingExportSummary> {
    let rows = build_cisd_rb_training_rows(state_dir, symbol)?;
    persist_training_rows(state_dir, symbol, &rows)?;
    let summary: CisdRbTrainingExportSummary = serde_json::from_str(&rows.summary_json)?;
    Ok(summary)
}

pub fn export_breaker_rb_training_tables(
    state_dir: &str,
    symbol: &str,
) -> Result<BreakerRbTrainingExportSummary> {
    let rows = build_breaker_rb_training_rows(state_dir, symbol)?;
    persist_training_rows(state_dir, symbol, &rows)?;
    let summary: BreakerRbTrainingExportSummary = serde_json::from_str(&rows.summary_json)?;
    Ok(summary)
}

fn build_breaker_rb_training_rows(state_dir: &str, symbol: &str) -> Result<EntryModelTrainingRows> {
    let rows = collect_breaker_training_rows(state_dir, symbol)?;
    let bbn_csv = render_breaker_bbn_training_csv(&rows.bbn_rows);
    let catboost_csv = render_breaker_catboost_training_csv(&rows.catboost_rows);
    let summary = BreakerRbTrainingExportSummary {
        symbol: symbol.to_string(),
        analyze_runs: rows.analyze_runs,
        update_runs: rows.update_runs,
        matched_rows: rows.bbn_rows.len(),
        bbn_training_path: Path::new(state_dir)
            .join(symbol)
            .join(POLICY_TRAINING_DIR)
            .join(BREAKER_RB_BBN_TRAINING_FILE)
            .to_string_lossy()
            .to_string(),
        catboost_training_path: Path::new(state_dir)
            .join(symbol)
            .join(POLICY_TRAINING_DIR)
            .join(BREAKER_RB_CATBOOST_TRAINING_FILE)
            .to_string_lossy()
            .to_string(),
    };
    Ok(EntryModelTrainingRows {
        provider_id: BREAKER_RB_SETUP_MODEL_ID.to_string(),
        matched_rows: rows.bbn_rows.len(),
        bbn_training_filename: BREAKER_RB_BBN_TRAINING_FILE.to_string(),
        bbn_csv,
        catboost_training_filename: BREAKER_RB_CATBOOST_TRAINING_FILE.to_string(),
        catboost_csv,
        summary_filename: BREAKER_RB_TRAINING_SUMMARY_FILE.to_string(),
        summary_json: serde_json::to_string_pretty(&summary)?,
    })
}

pub fn export_policy_training_tables(state_dir: &str, symbol: &str) -> Result<()> {
    for provider in entry_model_providers() {
        let rows = provider.training_rows(state_dir, symbol)?;
        persist_training_rows(state_dir, symbol, &rows)?;
    }
    Ok(())
}

fn build_cisd_rb_training_rows(state_dir: &str, symbol: &str) -> Result<EntryModelTrainingRows> {
    let rows = collect_training_rows(state_dir, symbol)?;
    let bbn_csv = render_bbn_training_csv(&rows.bbn_rows);
    let catboost_csv = render_catboost_training_csv(&rows.catboost_rows);
    let summary = CisdRbTrainingExportSummary {
        symbol: symbol.to_string(),
        analyze_runs: rows.analyze_runs,
        update_runs: rows.update_runs,
        matched_rows: rows.bbn_rows.len(),
        bbn_training_path: Path::new(state_dir)
            .join(symbol)
            .join(POLICY_TRAINING_DIR)
            .join(CISD_RB_BBN_TRAINING_FILE)
            .to_string_lossy()
            .to_string(),
        catboost_training_path: Path::new(state_dir)
            .join(symbol)
            .join(POLICY_TRAINING_DIR)
            .join(CISD_RB_CATBOOST_TRAINING_FILE)
            .to_string_lossy()
            .to_string(),
    };
    Ok(EntryModelTrainingRows {
        provider_id: CISD_RB_SETUP_MODEL_ID.to_string(),
        matched_rows: rows.bbn_rows.len(),
        bbn_training_filename: CISD_RB_BBN_TRAINING_FILE.to_string(),
        bbn_csv,
        catboost_training_filename: CISD_RB_CATBOOST_TRAINING_FILE.to_string(),
        catboost_csv,
        summary_filename: CISD_RB_TRAINING_SUMMARY_FILE.to_string(),
        summary_json: serde_json::to_string_pretty(&summary)?,
    })
}

pub fn cisd_rb_training_status(
    state_dir: &str,
    symbol: &str,
) -> Result<CisdRbTrainingStatusSurface> {
    let rows = collect_training_rows(state_dir, symbol)?;
    let setup_model_ids =
        rows.catboost_rows
            .iter()
            .fold(BTreeMap::<String, usize>::new(), |mut acc, row| {
                *acc.entry(row.setup_model_id.clone()).or_insert(0) += 1;
                acc
            });
    let bbn = build_bbn_status(&rows.bbn_rows);
    let catboost = build_catboost_status(&rows.catboost_rows);
    let summary_line = if bbn.ready && catboost.ready {
        format!(
            "policy training looks healthy for BBN and CatBoost: matched_rows={} outcomes={}",
            rows.bbn_rows.len(),
            format_counts(&catboost.outcome_counts)
        )
    } else if !bbn.ready && !catboost.ready {
        format!(
            "policy training is not ready for either BBN or CatBoost: matched_rows={} bbn_warnings={} catboost_warnings={}",
            rows.bbn_rows.len(),
            bbn.warnings.join("; "),
            catboost.warnings.join("; ")
        )
    } else if !bbn.ready {
        format!(
            "policy training is CatBoost-usable but BBN-weak: matched_rows={} bbn_warnings={}",
            rows.bbn_rows.len(),
            bbn.warnings.join("; ")
        )
    } else {
        format!(
            "policy training is BBN-usable but CatBoost-weak: matched_rows={} catboost_warnings={}",
            rows.catboost_rows.len(),
            catboost.warnings.join("; ")
        )
    };
    Ok(CisdRbTrainingStatusSurface {
        symbol: symbol.to_string(),
        analyze_runs: rows.analyze_runs,
        update_runs: rows.update_runs,
        matched_rows: rows.bbn_rows.len(),
        setup_model_ids,
        bbn,
        catboost,
        summary_line,
    })
}

pub fn breaker_rb_training_status(
    state_dir: &str,
    symbol: &str,
) -> Result<BreakerRbTrainingStatusSurface> {
    let rows = collect_breaker_training_rows(state_dir, symbol)?;
    let setup_model_ids =
        rows.catboost_rows
            .iter()
            .fold(BTreeMap::<String, usize>::new(), |mut acc, row| {
                *acc.entry(row.setup_model_id.clone()).or_insert(0) += 1;
                acc
            });
    let outcome_counts = count_strings(
        rows.bbn_rows
            .iter()
            .map(|row| row.realized_outcome.as_str()),
    );
    let bbn = build_generic_bbn_status(
        rows.bbn_rows.len(),
        outcome_counts.clone(),
        count_strings(rows.bbn_rows.iter().map(|row| row.entry_quality.as_str())),
        count_strings(
            rows.bbn_rows
                .iter()
                .map(|row| row.breaker_retest_quality.as_str()),
        ),
        count_strings(rows.bbn_rows.iter().map(|row| row.session_quality.as_str())),
    );
    let catboost = build_generic_catboost_status(
        rows.catboost_rows.len(),
        outcome_counts,
        BTreeMap::from([
            (
                "bars_between_violation_and_retest".to_string(),
                range_of(
                    rows.catboost_rows
                        .iter()
                        .map(|row| row.bars_between_violation_and_retest),
                ),
            ),
            (
                "breaker_width_bps".to_string(),
                range_of(rows.catboost_rows.iter().map(|row| row.breaker_width_bps)),
            ),
            (
                "retest_reclaim_bps".to_string(),
                range_of(rows.catboost_rows.iter().map(|row| row.retest_reclaim_bps)),
            ),
            (
                "rb_wick_body_ratio".to_string(),
                range_of(rows.catboost_rows.iter().map(|row| row.rb_wick_body_ratio)),
            ),
            (
                "realized_vol_zscore".to_string(),
                range_of(rows.catboost_rows.iter().map(|row| row.realized_vol_zscore)),
            ),
        ]),
    );
    let summary_line =
        build_provider_summary_line("Breaker RB", rows.bbn_rows.len(), &bbn, &catboost);
    Ok(BreakerRbTrainingStatusSurface {
        symbol: symbol.to_string(),
        analyze_runs: rows.analyze_runs,
        update_runs: rows.update_runs,
        matched_rows: rows.bbn_rows.len(),
        setup_model_ids,
        bbn,
        catboost,
        summary_line,
    })
}

pub fn cisd_rb_training_status_command(state_dir: &str, symbol: &str) -> Result<()> {
    let surface = cisd_rb_training_status(state_dir, symbol)?;
    println!("{}", serde_json::to_string_pretty(&surface)?);
    Ok(())
}

pub fn policy_training_status(
    state_dir: &str,
    symbol: &str,
    provider_filter: Option<&str>,
) -> Result<PolicyTrainingStatusSurface> {
    if let Some(filter) = provider_filter {
        if entry_model_providers()
            .into_iter()
            .all(|provider| provider.provider_id() != filter)
        {
            bail!(
                "unsupported policy training provider '{}'; available: {}",
                filter,
                entry_model_providers()
                    .into_iter()
                    .map(|provider| provider.provider_id())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    let cisd_rb = cisd_rb_training_status(state_dir, symbol)?;
    let structural_path_ranking_target =
        structural_path_ranking_target_training_status(state_dir, symbol)?;
    let factor_candidate_packs = factor_candidate_pack_training_status(state_dir, symbol)?;
    let regime_confidence_assets = regime_confidence_asset_training_status(state_dir, symbol)?;
    let providers = entry_model_providers()
        .into_iter()
        .filter(|provider| {
            provider_filter
                .map(|filter| filter == provider.provider_id())
                .unwrap_or(true)
        })
        .map(|provider| provider.status_surface(state_dir, symbol))
        .collect::<Result<Vec<_>>>()?;
    let provider_summary_line = if providers.iter().all(|provider| provider.ready) {
        format!(
            "all entry-model training modules ready: {}",
            providers
                .iter()
                .map(|provider| format!("{}={}", provider.provider_id, provider.matched_rows))
                .collect::<Vec<_>>()
                .join(",")
        )
    } else {
        let ready = providers
            .iter()
            .filter(|provider| provider.ready)
            .map(|provider| provider.provider_id.clone())
            .collect::<Vec<_>>();
        let pending = providers
            .iter()
            .filter(|provider| !provider.ready)
            .map(|provider| provider.provider_id.clone())
            .collect::<Vec<_>>();
        format!(
            "entry-model training modules mixed: ready=[{}] pending=[{}]",
            ready.join(","),
            pending.join(",")
        )
    };
    let summary_line = format!(
        "{} | {} | {} | {}",
        provider_summary_line,
        factor_candidate_packs.summary_line,
        regime_confidence_assets.summary_line,
        structural_path_ranking_target.summary_line
    );
    let structural_path_ranking_runtime_summary = format!(
        "Ranker runtime: {}",
        structural_path_ranking_target.summary_line
    );
    let structural_path_ranking_ready = structural_path_ranking_target.production_validation_ready
        || structural_path_ranking_target.observation_validation_ready;
    let structural_path_ranking_validation_summary = format!(
        "Ranker validation: calibration={} quality_ready={} raw_scored_mature={}/{} production_validation={}/{} observation_validation={}/{} ready={}",
        structural_path_ranking_target.calibration_ready,
        structural_path_ranking_target.calibration_quality_ready,
        structural_path_ranking_target.raw_scored_mature_rows,
        structural_path_ranking_target.raw_scored_mature_min_rows,
        structural_path_ranking_target.production_validation_rows,
        structural_path_ranking_target.production_validation_min_rows,
        structural_path_ranking_target.observation_validation_rows,
        structural_path_ranking_target.observation_validation_min_rows,
        structural_path_ranking_ready
    );
    let factor_hotplug_summary = crate::factors::hotplug::FactorHotplugConfig::load(state_dir)
        .map(|config| match config {
            Some(config) => {
                let disabled = config
                    .families
                    .iter()
                    .filter_map(|(name, enabled)| (!enabled).then_some(name.as_str()))
                    .collect::<Vec<_>>();
                if disabled.is_empty() {
                    "Factor hotplug: config=present disabled=[]".to_string()
                } else {
                    format!(
                        "Factor hotplug: config=present disabled=[{}]",
                        disabled.join(",")
                    )
                }
            }
            None => "Factor hotplug: config=absent all_default_enabled".to_string(),
        })
        .unwrap_or_else(|err| format!("Factor hotplug: config=invalid error={}", err));
    Ok(PolicyTrainingStatusSurface {
        symbol: symbol.to_string(),
        analyze_runs: cisd_rb.analyze_runs,
        update_runs: cisd_rb.update_runs,
        providers,
        factor_candidate_packs,
        regime_confidence_assets,
        structural_path_ranking_runtime: StructuralPathRankingRuntimeSummarySurface {
            enabled: structural_path_ranking_target.runtime_selection_enabled,
            ready: structural_path_ranking_target.runtime_selection_ready,
            status: if structural_path_ranking_target
                .runtime_selection_status
                .trim()
                .is_empty()
            {
                "disabled".to_string()
            } else {
                structural_path_ranking_target
                    .runtime_selection_status
                    .clone()
            },
            reuse_mode: structural_path_ranking_target
                .runtime_selection_mode
                .clone(),
            source_kind: structural_path_ranking_target.runtime_source_kind.clone(),
            active_match_count: structural_path_ranking_target.runtime_active_match_count,
            model_family: structural_path_ranking_target.score_model_family.clone(),
            score_model_family: structural_path_ranking_target.score_model_family.clone(),
            score_source_kind: structural_path_ranking_target.score_source_kind.clone(),
            score_model_artifact_uri: structural_path_ranking_target
                .score_model_artifact_uri
                .clone(),
            summary_line: structural_path_ranking_runtime_summary.clone(),
        },
        structural_path_ranking_validation: StructuralPathRankingValidationSummarySurface {
            calibration_ready: structural_path_ranking_target.calibration_ready,
            calibration_quality_ready: structural_path_ranking_target.calibration_quality_ready,
            calibration_status: if !structural_path_ranking_target.calibration_ready {
                "not_fitted".to_string()
            } else if !structural_path_ranking_target.calibration_quality_ready {
                "pending_eval".to_string()
            } else {
                "evaluated".to_string()
            },
            raw_scored_mature_rows: structural_path_ranking_target.raw_scored_mature_rows,
            raw_scored_mature_min_rows: structural_path_ranking_target.raw_scored_mature_min_rows,
            raw_scored_mature_shortfall_rows: structural_path_ranking_target
                .raw_scored_mature_shortfall_rows,
            production_validation_ready: structural_path_ranking_target.production_validation_ready,
            production_validation_rows: structural_path_ranking_target.production_validation_rows,
            production_validation_min_rows: structural_path_ranking_target
                .production_validation_min_rows,
            production_validation_shortfall_rows: structural_path_ranking_target
                .production_validation_shortfall_rows,
            observation_validation_ready: structural_path_ranking_target
                .observation_validation_ready,
            observation_validation_rows: structural_path_ranking_target.observation_validation_rows,
            observation_validation_min_rows: structural_path_ranking_target
                .observation_validation_min_rows,
            observation_validation_shortfall_rows: structural_path_ranking_target
                .observation_validation_shortfall_rows,
            target_row_validation: structural_path_ranking_target.target_row_validation.clone(),
            feedback_observation_validation: structural_path_ranking_target
                .feedback_observation_validation
                .clone(),
            summary_line: structural_path_ranking_validation_summary.clone(),
        },
        structural_path_ranking_runtime_summary,
        structural_path_ranking_validation_summary,
        factor_hotplug_summary,
        structural_path_ranking_target,
        summary_line,
    })
}

fn factor_candidate_pack_training_status(
    state_dir: &str,
    symbol: &str,
) -> Result<FactorCandidatePackTrainingStatusSurface> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let Some(entry) = ledger
        .iter()
        .rev()
        .find(|entry| entry.artifact_kind == "factor_candidate_pack_inventory")
    else {
        return Ok(FactorCandidatePackTrainingStatusSurface {
            inventory_status: "missing".to_string(),
            summary_line: "Factor candidate packs: inventory=missing count=0".to_string(),
            ..FactorCandidatePackTrainingStatusSurface::default()
        });
    };
    let path = Path::new(&entry.path);
    if !path.exists() {
        return Ok(FactorCandidatePackTrainingStatusSurface {
            inventory_status: "missing_file".to_string(),
            inventory_path: entry.path.clone(),
            summary_line: "Factor candidate packs: inventory=missing_file count=0".to_string(),
            ..FactorCandidatePackTrainingStatusSurface::default()
        });
    }
    let raw = fs::read_to_string(path)?;
    let inventory: serde_json::Value = serde_json::from_str(&raw)?;
    let candidates = inventory
        .get("candidates")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let candidate_pack_count = inventory
        .pointer("/summary/candidate_pack_count")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(candidates.len());
    let preferred_density_count = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .get("aggregate_label")
                .and_then(serde_json::Value::as_str)
                == Some("preferred_density")
        })
        .count();
    let cross_market_candidate_count = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .get("transfer_status")
                .and_then(serde_json::Value::as_str)
                == Some("cross_market_candidate")
        })
        .count();
    Ok(FactorCandidatePackTrainingStatusSurface {
        inventory_ready: candidate_pack_count > 0,
        inventory_status: "ready".to_string(),
        candidate_pack_count,
        preferred_density_count,
        cross_market_candidate_count,
        inventory_path: entry.path.clone(),
        summary_line: format!(
            "Factor candidate packs: inventory=ready count={} preferred_density={} cross_market={}",
            candidate_pack_count, preferred_density_count, cross_market_candidate_count
        ),
    })
}

fn regime_confidence_asset_training_status(
    state_dir: &str,
    symbol: &str,
) -> Result<RegimeConfidenceAssetTrainingStatusSurface> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let Some(entry) = ledger
        .iter()
        .rev()
        .find(|entry| entry.artifact_kind == "regime_confidence_asset_inventory")
    else {
        return Ok(RegimeConfidenceAssetTrainingStatusSurface {
            inventory_status: "missing".to_string(),
            summary_line: "Regime confidence assets: inventory=missing count=0".to_string(),
            ..RegimeConfidenceAssetTrainingStatusSurface::default()
        });
    };
    let path = Path::new(&entry.path);
    if !path.exists() {
        return Ok(RegimeConfidenceAssetTrainingStatusSurface {
            inventory_status: "missing_file".to_string(),
            inventory_path: entry.path.clone(),
            summary_line: "Regime confidence assets: inventory=missing_file count=0".to_string(),
            ..RegimeConfidenceAssetTrainingStatusSurface::default()
        });
    }
    let raw = fs::read_to_string(path)?;
    let inventory: serde_json::Value = serde_json::from_str(&raw)?;
    let assets = inventory
        .get("assets")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let asset_count = inventory
        .pointer("/summary/asset_count")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(assets.len());
    let board_a_regime_gate_count = inventory
        .pointer("/summary/board_a_regime_gate_count")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let direct_event_overlay_count = inventory
        .pointer("/summary/direct_event_overlay_count")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let diagnostic_after_source_control_unlock_count = inventory
        .pointer("/summary/diagnostic_after_source_control_unlock_count")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let contrast_evidence_count = inventory
        .pointer("/summary/contrast_evidence_count")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let recovered_not_candidate_pack_count = inventory
        .pointer("/summary/recovered_not_candidate_pack_count")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let promotion_allowed = inventory
        .pointer("/summary/promotion_allowed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let runtime_selection_enabled = inventory
        .pointer("/summary/runtime_selection_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    Ok(RegimeConfidenceAssetTrainingStatusSurface {
        inventory_ready: asset_count > 0,
        inventory_status: "ready".to_string(),
        asset_count,
        board_a_regime_gate_count,
        direct_event_overlay_count,
        diagnostic_after_source_control_unlock_count,
        contrast_evidence_count,
        recovered_not_candidate_pack_count,
        promotion_allowed,
        runtime_selection_enabled,
        inventory_path: entry.path.clone(),
        summary_line: format!(
            "Regime confidence assets: inventory=ready count={} board_a_gate={} direct_event={} diagnostic={} contrast_evidence={} promotion_allowed={} runtime_selection={}",
            asset_count,
            board_a_regime_gate_count,
            direct_event_overlay_count,
            diagnostic_after_source_control_unlock_count,
            contrast_evidence_count,
            promotion_allowed,
            if runtime_selection_enabled { "enabled" } else { "disabled" }
        ),
    })
}

pub fn structural_path_ranking_target_training_status(
    state_dir: &str,
    symbol: &str,
) -> Result<StructuralPathRankingTargetTrainingStatusSurface> {
    let summary_path = Path::new(state_dir)
        .join(symbol)
        .join(POLICY_TRAINING_DIR)
        .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE);
    if !summary_path.exists() {
        return Ok(StructuralPathRankingTargetTrainingStatusSurface {
            summary_path: summary_path.to_string_lossy().to_string(),
            warnings: vec!["structural_path_ranking_target_export_missing".to_string()],
            runtime_selection_status: "disabled".to_string(),
            summary_line:
                "structural path ranking target export missing runtime_selection=disabled runtime_source=none runtime_matches=0".to_string(),
            ..StructuralPathRankingTargetTrainingStatusSurface::default()
        });
    }
    let raw = fs::read_to_string(&summary_path)?;
    let summary: StructuralPathRankingTargetExportSummary = serde_json::from_str(&raw)?;
    let evaluation_jsonl_path = if !summary.history_jsonl_path.trim().is_empty() {
        summary.history_jsonl_path.as_str()
    } else {
        summary.jsonl_path.as_str()
    };
    let update_runs: Vec<UpdateRunRecord> =
        load_state_or_default(state_dir, symbol, UPDATE_RUNS_FILE).unwrap_or_default();
    let pending_update_history = load_pending_update_history(state_dir, symbol).unwrap_or_default();
    let learning_state = load_learning_state(state_dir, symbol).unwrap_or_default();
    let pending_update_artifact_present = Path::new(state_dir)
        .join(symbol)
        .join(PENDING_UPDATE_ARTIFACT_FILE)
        .exists();
    let pending_update_history_rows = pending_update_history.len();
    let pending_update_templates_with_structural_feedback = pending_update_history
        .iter()
        .filter(|artifact| artifact.template_feedback.structural_feedback.is_some())
        .count();
    let update_runs_with_structural_feedback = update_runs
        .iter()
        .filter(|run| run.structural_feedback.is_some())
        .count();
    let feedback_rows_total = learning_state.feedback_history.len();
    let feedback_rows_with_structural_feedback = learning_state
        .feedback_history
        .iter()
        .filter(|record| record.structural_feedback.is_some())
        .count();
    let feedback_rows_without_structural_feedback =
        feedback_rows_total.saturating_sub(feedback_rows_with_structural_feedback);
    let feedback_rows_without_structural_feedback_by_source = learning_state
        .feedback_history
        .iter()
        .filter(|record| record.structural_feedback.is_none())
        .fold(BTreeMap::<String, usize>::new(), |mut acc, record| {
            *acc.entry(record.source.clone()).or_insert(0) += 1;
            acc
        });
    let (
        feedback_rows_without_structural_feedback_dominant_source,
        feedback_rows_without_structural_feedback_dominant_count,
    ) = feedback_rows_without_structural_feedback_by_source
        .iter()
        .max_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(right.0)))
        .map(|(source, count)| (Some(source.clone()), *count))
        .unwrap_or((None, 0));
    let structural_feedback_records = learning_state
        .feedback_history
        .iter()
        .filter(|record| record.structural_feedback.is_some())
        .collect::<Vec<_>>();
    let feedback_rows_pending = structural_feedback_records
        .iter()
        .filter(|record| structural_feedback_outcome_is_unresolved(&record.realized_outcome))
        .count();
    let feedback_rows_matured = structural_feedback_records
        .iter()
        .filter(|record| !structural_feedback_outcome_is_unresolved(&record.realized_outcome))
        .count();
    let feedback_observation_outcome_distribution = structural_feedback_records
        .iter()
        .filter_map(|record| structural_feedback_counter_outcome(record))
        .fold(BTreeMap::<String, usize>::new(), |mut acc, outcome| {
            *acc.entry(outcome.to_string()).or_insert(0) += 1;
            acc
        });
    let history_rows = load_structural_path_ranking_target_rows_from_jsonl(evaluation_jsonl_path)?;
    let history_row_count = history_rows.len().max(summary.history_rows);
    let history_mature_rows = history_rows
        .iter()
        .filter(|row| row.maturity_mask)
        .count()
        .max(summary.history_mature_rows);
    let history_rows_with_calibrated_path_prob = history_rows
        .iter()
        .filter(|row| row.calibrated_path_prob.is_some())
        .count()
        .max(summary.history_rows_with_calibrated_path_prob)
        .max(summary.rows_with_calibrated_path_prob);
    let history_rows_with_path_prob_lower_bound = summary
        .history_rows_with_path_prob_lower_bound
        .max(
            history_rows
                .iter()
                .filter(|row| row.path_prob_lower_bound.is_some())
                .count(),
        )
        .max(summary.rows_with_path_prob_lower_bound);
    let history_rows_with_raw_path_score = history_rows
        .iter()
        .filter(|row| row.raw_path_score.is_some())
        .count()
        .max(summary.history_rows_with_raw_path_score)
        .max(summary.rows_with_raw_path_score);
    let history_rows_with_propensity_estimate = summary
        .history_rows_with_propensity_estimate
        .max(
            history_rows
                .iter()
                .filter(|row| row.propensity_estimate.is_some())
                .count(),
        )
        .max(summary.rows_with_propensity_estimate);
    let history_rows_with_training_weight = summary
        .history_rows_with_training_weight
        .max(
            history_rows
                .iter()
                .filter(|row| row.training_weight.is_some())
                .count(),
        )
        .max(summary.rows_with_training_weight);
    let current_rows = load_structural_path_ranking_target_rows_from_jsonl(&summary.jsonl_path)?;
    let calibration_ready =
        history_rows_with_calibrated_path_prob > 0 && history_rows_with_path_prob_lower_bound > 0;
    let trainer_manifest = &summary.trainer_manifest;
    let trainer_manifest_ready = structural_path_ranking_trainer_manifest_ready(trainer_manifest);
    let trainer_artifact_path = summary_path
        .parent()
        .map(|parent| parent.join(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE))
        .unwrap_or_else(|| {
            Path::new(state_dir)
                .join(symbol)
                .join(POLICY_TRAINING_DIR)
                .join(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE)
        });
    let (trainer_artifact, trainer_artifact_file_present, trainer_artifact_warning) =
        structural_path_ranking_trainer_artifact(&trainer_artifact_path);
    let trainer_artifact_ready = trainer_artifact.as_ref().is_some_and(|artifact| {
        structural_path_ranking_trainer_artifact_ready(artifact, trainer_manifest)
    });
    let runtime_selection_path = structural_path_ranking_runtime_selection_path(state_dir, symbol);
    let runtime_selection = load_structural_path_ranking_runtime_selection(state_dir, symbol);
    let direct_model_candidate_rows = current_rows
        .iter()
        .filter(|row| row.candidate_set_id == summary.candidate_set_id)
        .cloned()
        .collect::<Vec<_>>();
    let runtime_direct_model_rows = trainer_artifact
        .as_ref()
        .and_then(|artifact| {
            if !structural_path_ranker_supports_direct_model_family(&artifact.model_family) {
                return None;
            }
            score_structural_path_ranker_runtime_rows_with_direct_model(
                state_dir,
                symbol,
                &artifact.artifact_uri,
                &artifact.model_family,
                &direct_model_candidate_rows,
            )
            .ok()
        })
        .unwrap_or_default();
    let runtime_direct_model_ready = !runtime_direct_model_rows.is_empty();
    let runtime_explicit_rows = trainer_artifact
        .as_ref()
        .and_then(|artifact| {
            if !structural_path_ranker_supports_explicit_family(&artifact.model_family) {
                return None;
            }
            score_structural_path_ranker_runtime_rows_with_explicit_family(
                state_dir,
                symbol,
                &artifact.model_family,
                &direct_model_candidate_rows,
            )
            .ok()
        })
        .unwrap_or_default();
    let runtime_explicit_ready = !runtime_explicit_rows.is_empty();
    let runtime_service_rows = trainer_artifact
        .as_ref()
        .and_then(|artifact| {
            if !structural_path_ranker_supports_service_family(&artifact.model_family) {
                return None;
            }
            score_structural_path_ranker_runtime_rows_with_service(
                symbol,
                &artifact.artifact_uri,
                &artifact.score_column,
                &artifact.model_family,
                &direct_model_candidate_rows,
            )
            .ok()
        })
        .unwrap_or_default();
    let runtime_service_ready = !runtime_service_rows.is_empty();
    let runtime_direct_model_loadable = trainer_artifact
        .as_ref()
        .and_then(|artifact| {
            if !structural_path_ranker_supports_direct_model_family(&artifact.model_family) {
                return None;
            }
            load_structural_path_ranker_direct_model_artifact(
                state_dir,
                symbol,
                &artifact.artifact_uri,
                &artifact.model_family,
            )
            .ok()
            .flatten()
        })
        .is_some();
    let runtime_service_declared = trainer_artifact.as_ref().is_some_and(|artifact| {
        structural_path_ranker_supports_service_family(&artifact.model_family)
    });
    let runtime_artifact_rows = if runtime_direct_model_ready {
        runtime_direct_model_rows
    } else if runtime_explicit_ready {
        runtime_explicit_rows
    } else if runtime_service_ready {
        runtime_service_rows
    } else if runtime_service_declared {
        Vec::new()
    } else {
        trainer_artifact
            .as_ref()
            .and_then(|artifact| {
                load_structural_path_ranker_runtime_artifact_rows(
                    state_dir,
                    symbol,
                    &artifact.artifact_uri,
                    &artifact.score_column,
                )
                .ok()
            })
            .unwrap_or_default()
    };
    let runtime_artifact_match_count = runtime_artifact_rows
        .iter()
        .filter(|row| {
            row.candidate_set_id == summary.candidate_set_id && row.raw_path_score.is_some()
        })
        .map(|row| row.path_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let runtime_candidate_set_match_count = history_rows
        .iter()
        .chain(current_rows.iter())
        .filter(|row| {
            row.candidate_set_id == summary.candidate_set_id && row.raw_path_score.is_some()
        })
        .map(|row| row.path_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let runtime_history_match_count = history_rows
        .iter()
        .chain(current_rows.iter())
        .filter(|row| row.raw_path_score.is_some())
        .map(|row| row.path_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let calibration_evaluation =
        structural_path_ranking_target_calibration_evaluation(evaluation_jsonl_path)?;
    let calibration_quality_ready = calibration_evaluation.status == "evaluated";
    let raw_scored_mature_rows =
        structural_path_ranking_target_raw_scored_mature_rows(evaluation_jsonl_path)?;
    let raw_scored_mature_min_rows = STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS;
    let raw_scored_mature_shortfall_rows =
        raw_scored_mature_min_rows.saturating_sub(raw_scored_mature_rows);
    let production_validation_rows = calibration_evaluation.propensity_weighted_rows;
    let production_validation_min_rows = STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS;
    let production_validation_shortfall_rows =
        production_validation_min_rows.saturating_sub(production_validation_rows);
    let production_validation_ready =
        calibration_quality_ready && production_validation_rows >= production_validation_min_rows;
    let target_row_validation_summary = format!(
        "target_rows raw_scored_mature={}/{} production_validation={}/{} ready={}",
        raw_scored_mature_rows,
        raw_scored_mature_min_rows,
        production_validation_rows,
        production_validation_min_rows,
        production_validation_ready
    );
    let observation_validation_rows = feedback_rows_matured;
    let observation_validation_min_rows = STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS;
    let observation_validation_shortfall_rows =
        observation_validation_min_rows.saturating_sub(observation_validation_rows);
    let observation_validation_ready =
        observation_validation_rows >= observation_validation_min_rows;
    let feedback_observation_validation_summary = format!(
        "observations mature={}/{} pending={} total={} ready={}",
        observation_validation_rows,
        observation_validation_min_rows,
        feedback_rows_pending,
        feedback_rows_with_structural_feedback,
        observation_validation_ready
    );
    let target_row_validation = StructuralPathRankingTargetRowValidationSurface {
        raw_scored_mature_rows,
        raw_scored_mature_min_rows,
        raw_scored_mature_shortfall_rows,
        production_validation_ready,
        production_validation_rows,
        production_validation_min_rows,
        production_validation_shortfall_rows,
        summary_line: target_row_validation_summary,
    };
    let feedback_observation_validation =
        StructuralPathRankingFeedbackObservationValidationSurface {
            ready: observation_validation_ready,
            mature_observations: observation_validation_rows,
            min_observations: observation_validation_min_rows,
            shortfall_observations: observation_validation_shortfall_rows,
            pending_observations: feedback_rows_pending,
            total_observations: feedback_rows_with_structural_feedback,
            outcome_distribution: feedback_observation_outcome_distribution,
            summary_line: feedback_observation_validation_summary,
        };
    let mut warnings = Vec::new();
    if summary.rows == 0 {
        warnings.push("structural_path_ranking_target_rows_empty".to_string());
    }
    if summary.mature_rows == 0 {
        warnings.push("structural_path_ranking_target_mature_rows_missing".to_string());
    }
    if update_runs.is_empty() {
        warnings.push("structural_path_ranking_target_update_runs_missing".to_string());
    }
    if update_runs_with_structural_feedback == 0 && feedback_rows_with_structural_feedback == 0 {
        warnings.push("structural_path_ranking_target_structural_feedback_missing".to_string());
        if pending_update_history_rows > 0 || pending_update_artifact_present {
            warnings.push(
                "structural_path_ranking_target_pending_update_templates_present".to_string(),
            );
        }
    }
    if feedback_rows_total > 0 && feedback_rows_with_structural_feedback == 0 {
        if let Some(source) = feedback_rows_without_structural_feedback_dominant_source.as_deref() {
            warnings.push(format!(
                "structural_path_ranking_target_feedback_rows_missing_structural_refs:dominant_source={} count={}",
                source,
                feedback_rows_without_structural_feedback_dominant_count
            ));
        } else {
            warnings.push(
                "structural_path_ranking_target_feedback_rows_missing_structural_refs".to_string(),
            );
        }
    }
    if history_rows_with_propensity_estimate == 0 {
        warnings.push("structural_path_ranking_target_propensity_missing".to_string());
    }
    if !calibration_ready {
        warnings.push("structural_path_ranking_target_calibration_not_fitted".to_string());
    }
    if !trainer_manifest_ready {
        warnings.push("structural_path_ranking_target_trainer_manifest_incomplete".to_string());
    }
    if let Some(warning) = trainer_artifact_warning {
        warnings.push(warning);
    }
    if !trainer_artifact_ready {
        if trainer_artifact_file_present {
            warnings.push("structural_path_ranking_target_trainer_artifact_incomplete".to_string());
        } else {
            warnings.push("structural_path_ranking_target_trainer_artifact_missing".to_string());
        }
    }
    if !calibration_quality_ready {
        warnings.extend(calibration_evaluation.warnings.clone());
    }
    if raw_scored_mature_shortfall_rows > 0 {
        warnings.push(format!(
            "structural_path_ranking_target_raw_scored_mature_rows_insufficient:min={} observed={}",
            raw_scored_mature_min_rows, raw_scored_mature_rows
        ));
    }
    if !production_validation_ready {
        warnings.push(format!(
            "structural_path_ranking_target_production_validation_insufficient_rows:min={} observed={}",
            production_validation_min_rows, production_validation_rows
        ));
    }
    if !observation_validation_ready {
        warnings.push(format!(
            "structural_path_ranking_observation_validation_insufficient_rows:min={} observed={}",
            observation_validation_min_rows, observation_validation_rows
        ));
    }
    let calibration_status = if !calibration_ready {
        "not_fitted"
    } else if !calibration_quality_ready {
        "pending_eval"
    } else {
        "evaluated"
    };
    let trainer_status = if trainer_artifact_ready {
        "ready"
    } else if trainer_artifact_file_present {
        "incomplete"
    } else {
        "missing"
    };
    let trainer_artifact_status = if !trainer_artifact_file_present {
        "missing".to_string()
    } else if !trainer_artifact_ready {
        "present_validation_insufficient".to_string()
    } else if calibration_quality_ready && production_validation_ready {
        "runtime_eligible".to_string()
    } else {
        "present_validation_insufficient".to_string()
    };
    let runtime_selection_enabled = runtime_selection
        .as_ref()
        .map(|selection| selection.enabled)
        .unwrap_or(false);
    let runtime_selection_mode = runtime_selection
        .as_ref()
        .and_then(|selection| non_empty_string(&selection.reuse_mode));
    let runtime_selection_status = match runtime_selection.as_ref() {
        None => "disabled".to_string(),
        Some(selection) if !selection.enabled => "disabled".to_string(),
        Some(_) if runtime_direct_model_ready => "enabled_registered_model_ready".to_string(),
        Some(_) if runtime_explicit_ready => {
            "enabled_registered_explicit_artifact_ready".to_string()
        }
        Some(_) if runtime_service_ready => "enabled_registered_service_ready".to_string(),
        Some(_)
            if trainer_artifact.as_ref().is_some_and(|artifact| {
                structural_path_ranker_supports_direct_model_family(&artifact.model_family)
            }) && !runtime_direct_model_loadable =>
        {
            "enabled_registered_model_invalid".to_string()
        }
        Some(_) if runtime_service_declared => "enabled_registered_service_invalid".to_string(),
        Some(_)
            if trainer_artifact.as_ref().is_some_and(|artifact| {
                structural_path_ranker_supports_explicit_family(&artifact.model_family)
            }) =>
        {
            "enabled_registered_explicit_artifact_invalid".to_string()
        }
        Some(_) if runtime_artifact_match_count > 0 => {
            "enabled_registered_artifact_ready".to_string()
        }
        Some(_) if runtime_candidate_set_match_count > 0 => {
            "enabled_candidate_set_ready".to_string()
        }
        Some(selection)
            if selection.reuse_mode == STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY
                && runtime_history_match_count > 0 =>
        {
            "enabled_history_ready".to_string()
        }
        Some(_) => "enabled_no_matching_scores".to_string(),
    };
    let runtime_selection_ready = matches!(
        runtime_selection_status.as_str(),
        "enabled_registered_model_ready"
            | "enabled_registered_explicit_artifact_ready"
            | "enabled_registered_service_ready"
            | "enabled_registered_artifact_ready"
            | "enabled_candidate_set_ready"
            | "enabled_history_ready"
    );
    let runtime_source_kind =
        structural_path_ranking_runtime_source_kind(&runtime_selection_status).map(str::to_string);
    let runtime_active_match_count = structural_path_ranking_runtime_active_match_count(
        runtime_source_kind.as_deref(),
        runtime_artifact_match_count,
        runtime_candidate_set_match_count,
        runtime_history_match_count,
    );
    let score_model_family = runtime_row_score_model_family(
        runtime_source_kind.as_deref(),
        &runtime_artifact_rows,
        &history_rows,
        &current_rows,
        &summary.candidate_set_id,
        trainer_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(&artifact.model_family)),
    );
    let score_source_kind = runtime_row_score_source_kind(
        runtime_source_kind.as_deref(),
        &history_rows,
        &current_rows,
        &summary.candidate_set_id,
    );
    let score_model_artifact_uri = runtime_row_score_model_artifact_uri(
        runtime_source_kind.as_deref(),
        &history_rows,
        &current_rows,
        &summary.candidate_set_id,
        trainer_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(&artifact.artifact_uri)),
    );
    let summary_line = format!(
        "structural_path_ranking_target rows={} history_rows={} mature_rows={} history_mature_rows={} raw_scored_mature={}/{} production_validation={}/{} observation_validation={}/{} calibration={} trainer_artifact={} trainer_status={} runtime_selection={} runtime_mode={} runtime_source={} score_model_family={} score_source={} runtime_matches={}",
        summary.rows,
        summary.history_rows,
        summary.mature_rows,
        summary.history_mature_rows,
        raw_scored_mature_rows,
        raw_scored_mature_min_rows,
        production_validation_rows,
        production_validation_min_rows,
        observation_validation_rows,
        observation_validation_min_rows,
        calibration_status,
        trainer_status,
        trainer_artifact_status,
        runtime_selection_status,
        runtime_selection_mode.as_deref().unwrap_or("none"),
        runtime_source_kind.as_deref().unwrap_or("none"),
        score_model_family.as_deref().unwrap_or("unknown"),
        score_source_kind.as_deref().unwrap_or("unknown"),
        runtime_active_match_count
    );
    Ok(StructuralPathRankingTargetTrainingStatusSurface {
        export_ready: summary.rows > 0,
        calibration_ready,
        calibration_quality_ready,
        trainer_manifest_ready,
        trainer_manifest_protocol_version: non_empty_string(&trainer_manifest.protocol_version),
        trainer_manifest_dataset_role: non_empty_string(&trainer_manifest.dataset_role),
        trainer_feature_columns: trainer_manifest.feature_columns.len(),
        trainer_calibration_columns: trainer_manifest.calibration_columns.len(),
        trainer_guardrail_columns: trainer_manifest.guardrail_columns.len(),
        trainer_artifact_ready,
        trainer_artifact_status,
        trainer_artifact_path: trainer_artifact_path.to_string_lossy().to_string(),
        trainer_artifact_protocol_version: trainer_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(&artifact.protocol_version)),
        trainer_artifact_dataset_role: trainer_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(&artifact.dataset_role)),
        trainer_artifact_model_family: trainer_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(&artifact.model_family)),
        trainer_artifact_score_column: trainer_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(&artifact.score_column)),
        trainer_artifact_trained_rows: trainer_artifact
            .as_ref()
            .map(|artifact| artifact.trained_rows)
            .unwrap_or_default(),
        trainer_artifact_calibration_rows: trainer_artifact
            .as_ref()
            .map(|artifact| artifact.calibration_rows)
            .unwrap_or_default(),
        trainer_artifact_feature_columns: trainer_artifact
            .as_ref()
            .map(|artifact| artifact.selected_features.len())
            .unwrap_or_default(),
        trainer_artifact_history_rows: trainer_artifact
            .as_ref()
            .map(|artifact| artifact.history_rows)
            .unwrap_or_default(),
        trainer_artifact_uri_present: trainer_artifact
            .as_ref()
            .and_then(|artifact| non_empty_string(&artifact.artifact_uri))
            .is_some(),
        runtime_selection_enabled,
        runtime_selection_ready,
        runtime_selection_status,
        runtime_selection_path,
        runtime_selection_mode,
        runtime_source_kind,
        runtime_active_match_count,
        runtime_artifact_match_count,
        runtime_candidate_set_match_count,
        runtime_history_match_count,
        score_model_family: score_model_family.clone(),
        score_source_kind: score_source_kind.clone(),
        score_model_artifact_uri: score_model_artifact_uri.clone(),
        rows: summary.rows,
        candidate_set_id: Some(summary.candidate_set_id),
        candidate_set_size: summary.candidate_set_size,
        mature_rows: summary.mature_rows,
        rows_with_propensity_estimate: summary.rows_with_propensity_estimate,
        rows_with_calibrated_path_prob: summary.rows_with_calibrated_path_prob,
        rows_with_execution_gate_status: summary.rows_with_execution_gate_status,
        rows_with_training_weight: summary.rows_with_training_weight,
        history_rows: history_row_count,
        history_mature_rows,
        history_rows_with_raw_path_score,
        history_rows_with_calibrated_path_prob,
        history_rows_with_path_prob_lower_bound,
        history_rows_with_propensity_estimate,
        history_rows_with_training_weight,
        update_runs_with_structural_feedback,
        feedback_rows_total,
        feedback_rows_with_structural_feedback,
        feedback_rows_without_structural_feedback,
        feedback_rows_without_structural_feedback_dominant_source,
        feedback_rows_without_structural_feedback_dominant_count,
        feedback_rows_matured,
        feedback_rows_pending,
        pending_update_artifact_present,
        pending_update_history_rows,
        pending_update_templates_with_structural_feedback,
        calibration_evaluation_rows: calibration_evaluation.eligible_rows,
        calibration_brier_score: calibration_evaluation.brier_score,
        calibration_propensity_weighted_rows: calibration_evaluation.propensity_weighted_rows,
        calibration_propensity_weighted_brier_score: calibration_evaluation
            .propensity_weighted_brier_score,
        calibration_expected_error: calibration_evaluation.expected_calibration_error,
        calibration_max_error: calibration_evaluation.max_calibration_error,
        raw_scored_mature_rows,
        raw_scored_mature_min_rows,
        raw_scored_mature_shortfall_rows,
        production_validation_ready,
        production_validation_rows,
        production_validation_min_rows,
        production_validation_shortfall_rows,
        observation_validation_ready,
        observation_validation_rows,
        observation_validation_min_rows,
        observation_validation_shortfall_rows,
        target_row_validation,
        feedback_observation_validation,
        summary_path: summary.summary_path,
        csv_path: Some(summary.csv_path),
        jsonl_path: Some(summary.jsonl_path),
        history_csv_path: non_empty_string(&summary.history_csv_path),
        history_jsonl_path: non_empty_string(&summary.history_jsonl_path),
        warnings,
        summary_line,
    })
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn structural_path_ranking_runtime_source_kind(status: &str) -> Option<&'static str> {
    match status {
        "enabled_registered_model_ready" | "enabled_registered_model_invalid" => {
            Some("registered_model_artifact")
        }
        "enabled_registered_explicit_artifact_ready"
        | "enabled_registered_explicit_artifact_invalid" => Some("registered_explicit_artifact"),
        "enabled_registered_service_ready" | "enabled_registered_service_invalid" => {
            Some("registered_service")
        }
        "enabled_registered_artifact_ready" => Some("registered_artifact"),
        "enabled_candidate_set_ready" => Some("candidate_set"),
        "enabled_history_ready" => Some("history"),
        _ => None,
    }
}

fn structural_path_ranking_runtime_active_match_count(
    runtime_source_kind: Option<&str>,
    runtime_artifact_match_count: usize,
    runtime_candidate_set_match_count: usize,
    runtime_history_match_count: usize,
) -> usize {
    match runtime_source_kind {
        Some("registered_model_artifact" | "registered_service" | "registered_artifact") => {
            runtime_artifact_match_count
        }
        Some("candidate_set") => runtime_candidate_set_match_count,
        Some("history") => runtime_history_match_count,
        _ => 0,
    }
}

fn unique_non_empty_values(values: impl Iterator<Item = Option<String>>) -> BTreeSet<String> {
    values
        .filter_map(|value| value.and_then(|value| non_empty_string(&value)))
        .collect()
}

fn single_or_mixed(values: BTreeSet<String>) -> Option<String> {
    if values.is_empty() {
        None
    } else if values.len() == 1 {
        values.into_iter().next()
    } else {
        Some("mixed".to_string())
    }
}

fn runtime_row_score_model_family(
    runtime_source_kind: Option<&str>,
    runtime_artifact_rows: &[crate::belief_core::ranking_label::StructuralPathRankerRuntimeRow],
    history_rows: &[StructuralPathRankingTargetRow],
    current_rows: &[StructuralPathRankingTargetRow],
    candidate_set_id: &str,
    trainer_artifact_family: Option<String>,
) -> Option<String> {
    match runtime_source_kind {
        Some("registered_model_artifact" | "registered_service" | "registered_artifact") => {
            single_or_mixed(unique_non_empty_values(
                runtime_artifact_rows
                    .iter()
                    .filter(|row| {
                        row.candidate_set_id == candidate_set_id && row.raw_path_score.is_some()
                    })
                    .map(|row| row.score_model_family.clone()),
            ))
            .or(trainer_artifact_family)
        }
        Some("candidate_set") => single_or_mixed(unique_non_empty_values(
            history_rows
                .iter()
                .chain(current_rows.iter())
                .filter(|row| {
                    row.candidate_set_id == candidate_set_id && row.raw_path_score.is_some()
                })
                .map(|row| row.score_model_family.clone()),
        )),
        Some("history") => single_or_mixed(unique_non_empty_values(
            history_rows
                .iter()
                .chain(current_rows.iter())
                .filter(|row| row.raw_path_score.is_some())
                .map(|row| row.score_model_family.clone()),
        )),
        _ => None,
    }
}

fn runtime_row_score_source_kind(
    runtime_source_kind: Option<&str>,
    history_rows: &[StructuralPathRankingTargetRow],
    current_rows: &[StructuralPathRankingTargetRow],
    candidate_set_id: &str,
) -> Option<String> {
    match runtime_source_kind {
        Some("candidate_set") => single_or_mixed(unique_non_empty_values(
            history_rows
                .iter()
                .chain(current_rows.iter())
                .filter(|row| {
                    row.candidate_set_id == candidate_set_id && row.raw_path_score.is_some()
                })
                .map(|row| row.score_source_kind.clone()),
        )),
        Some("history") => single_or_mixed(unique_non_empty_values(
            history_rows
                .iter()
                .chain(current_rows.iter())
                .filter(|row| row.raw_path_score.is_some())
                .map(|row| row.score_source_kind.clone()),
        )),
        Some("registered_model_artifact") => Some("direct_model".to_string()),
        Some("registered_service") => Some("service".to_string()),
        Some("registered_artifact") => Some("external_artifact".to_string()),
        _ => None,
    }
}

fn runtime_row_score_model_artifact_uri(
    runtime_source_kind: Option<&str>,
    history_rows: &[StructuralPathRankingTargetRow],
    current_rows: &[StructuralPathRankingTargetRow],
    candidate_set_id: &str,
    trainer_artifact_uri: Option<String>,
) -> Option<String> {
    match runtime_source_kind {
        Some("candidate_set") => single_or_mixed(unique_non_empty_values(
            history_rows
                .iter()
                .chain(current_rows.iter())
                .filter(|row| {
                    row.candidate_set_id == candidate_set_id && row.raw_path_score.is_some()
                })
                .map(|row| row.score_model_artifact_uri.clone()),
        )),
        Some("history") => single_or_mixed(unique_non_empty_values(
            history_rows
                .iter()
                .chain(current_rows.iter())
                .filter(|row| row.raw_path_score.is_some())
                .map(|row| row.score_model_artifact_uri.clone()),
        )),
        Some("registered_model_artifact" | "registered_service" | "registered_artifact") => {
            trainer_artifact_uri
        }
        _ => None,
    }
}

fn structural_path_ranking_trainer_manifest_ready(
    manifest: &StructuralPathRankingTrainerManifest,
) -> bool {
    [
        manifest.protocol_version.as_str(),
        manifest.dataset_role.as_str(),
        manifest.group_id_column.as_str(),
        manifest.label_column.as_str(),
        manifest.weight_column.as_str(),
        manifest.maturity_column.as_str(),
        manifest.raw_score_column.as_str(),
    ]
    .iter()
    .all(|value| !value.trim().is_empty())
        && !manifest.feature_columns.is_empty()
        && !manifest.calibration_columns.is_empty()
        && !manifest.guardrail_columns.is_empty()
}

fn structural_path_ranking_trainer_artifact(
    artifact_path: &Path,
) -> (
    Option<StructuralPathRankingTrainerArtifact>,
    bool,
    Option<String>,
) {
    if !artifact_path.exists() {
        return (None, false, None);
    }
    let raw = match fs::read_to_string(artifact_path) {
        Ok(raw) => raw,
        Err(_) => {
            return (
                None,
                true,
                Some("structural_path_ranking_target_trainer_artifact_unreadable".to_string()),
            );
        }
    };
    match serde_json::from_str::<StructuralPathRankingTrainerArtifact>(&raw) {
        Ok(artifact) => (Some(artifact), true, None),
        Err(_) => (
            None,
            true,
            Some("structural_path_ranking_target_trainer_artifact_invalid_json".to_string()),
        ),
    }
}

fn structural_path_ranking_trainer_artifact_ready(
    artifact: &StructuralPathRankingTrainerArtifact,
    manifest: &StructuralPathRankingTrainerManifest,
) -> bool {
    artifact.protocol_version.trim() == STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION
        && artifact.dataset_role.trim() == manifest.dataset_role.trim()
        && !artifact.model_family.trim().is_empty()
        && !artifact.score_column.trim().is_empty()
        && artifact.trained_rows > 0
        && artifact.history_rows > 0
        && !artifact.selected_features.is_empty()
        && if structural_path_ranker_supports_explicit_family(&artifact.model_family) {
            !artifact.rule_list.is_empty() || artifact.tree_json.is_some()
        } else {
            !artifact.artifact_uri.trim().is_empty()
        }
}

fn structural_path_ranking_source_artifact_path(artifact_uri: &str) -> Option<std::path::PathBuf> {
    let trimmed = artifact_uri.trim();
    if trimmed.is_empty() || trimmed.contains("://") && !trimmed.starts_with("file://") {
        return None;
    }
    if let Some(path) = trimmed.strip_prefix("file://") {
        return Some(std::path::PathBuf::from(path));
    }
    Some(std::path::PathBuf::from(trimmed))
}

fn merge_structural_path_ranking_source_artifact(
    artifact: &mut StructuralPathRankingTrainerArtifact,
    artifact_uri: &str,
) -> Result<bool> {
    let Some(path) = structural_path_ranking_source_artifact_path(artifact_uri) else {
        return Ok(false);
    };
    if !path.exists() {
        return Ok(false);
    }
    let raw = fs::read_to_string(path)?;
    let source = match serde_json::from_str::<StructuralPathRankingTrainerArtifact>(&raw) {
        Ok(source) => source,
        Err(_) => return Ok(false),
    };
    if let Some(source_family) = non_empty_string(&source.model_family) {
        if source_family != artifact.model_family {
            bail!(
                "structural path ranking trainer artifact family mismatch: cli='{}' source='{}'",
                artifact.model_family,
                source_family
            );
        }
    }
    if let Some(score_column) = non_empty_string(&source.score_column) {
        artifact.score_column = score_column;
    }
    if let Some(source_artifact_uri) = non_empty_string(&source.artifact_uri) {
        artifact.artifact_uri = source_artifact_uri;
    }
    if source.model_artifact_uri.is_some() {
        artifact.model_artifact_uri = source.model_artifact_uri;
    }
    if source.trained_rows > 0 {
        artifact.trained_rows = source.trained_rows;
    }
    if source.history_rows > 0 {
        artifact.history_rows = source.history_rows;
    }
    if source.calibration_rows > 0 {
        artifact.calibration_rows = source.calibration_rows;
    }
    if !source.selected_features.is_empty() {
        artifact.selected_features = source.selected_features;
    }
    if !source.rule_list.is_empty() {
        artifact.rule_list = source.rule_list;
    }
    if source.tree_json.is_some() {
        artifact.tree_json = source.tree_json;
    }
    if !source.notes.is_empty() {
        artifact.notes.extend(source.notes);
    }
    Ok(true)
}

fn structural_path_ranking_target_calibration_evaluation(
    jsonl_path: &str,
) -> Result<StructuralPathProbabilityCalibrationEvaluationReport> {
    let rows = load_structural_path_ranking_target_rows_from_jsonl(jsonl_path)?;
    if rows.is_empty() && !Path::new(jsonl_path).exists() {
        return Ok(StructuralPathProbabilityCalibrationEvaluationReport {
            status: "calibration_evaluation_export_missing".to_string(),
            warnings: vec!["structural_path_ranking_target_jsonl_missing".to_string()],
            summary_line:
                "structural_path_probability_calibration_evaluation status=export_missing"
                    .to_string(),
            ..StructuralPathProbabilityCalibrationEvaluationReport::default()
        });
    }
    Ok(evaluate_structural_path_probability_calibration_rows(&rows))
}

fn load_structural_path_ranking_target_rows_from_jsonl(
    jsonl_path: &str,
) -> Result<Vec<StructuralPathRankingTargetRow>> {
    if !Path::new(jsonl_path).exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(jsonl_path)?;
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<StructuralPathRankingTargetRow>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn structural_path_ranking_target_raw_scored_mature_rows(jsonl_path: &str) -> Result<usize> {
    let rows = load_structural_path_ranking_target_rows_from_jsonl(jsonl_path)?;
    Ok(rows
        .iter()
        .filter(|row| row.raw_path_score.is_some() && row.calibrated_label.is_some())
        .count())
}

fn load_structural_path_ranking_external_scores(
    scores_path: &str,
) -> Result<Vec<StructuralPathRankingExternalScoreInput>> {
    if scores_path.ends_with(".jsonl") {
        let raw = fs::read_to_string(scores_path)?;
        return raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str::<StructuralPathRankingExternalScoreInput>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into);
    }
    let mut reader = csv::Reader::from_path(scores_path)?;
    let mut rows = Vec::new();
    for row in reader.deserialize::<StructuralPathRankingExternalScoreInput>() {
        rows.push(row?);
    }
    Ok(rows)
}

pub fn policy_training_status_command(
    state_dir: &str,
    symbol: &str,
    provider_filter: Option<&str>,
) -> Result<()> {
    let surface = policy_training_status(state_dir, symbol, provider_filter)?;
    println!("{}", serde_json::to_string_pretty(&surface)?);
    Ok(())
}

fn register_structural_path_ranking_trainer_artifact(
    state_dir: &str,
    symbol: &str,
    artifact_uri: &str,
    model_family: &str,
    score_column: Option<&str>,
    trained_rows: Option<usize>,
    calibration_rows: Option<usize>,
) -> Result<(String, StructuralPathRankingTrainerArtifact)> {
    let artifact_uri = artifact_uri.trim();
    if artifact_uri.is_empty() {
        bail!("artifact uri must not be empty");
    }
    let model_family = model_family.trim();
    if model_family.is_empty() {
        bail!("model family must not be empty");
    }
    let summary_path = Path::new(state_dir)
        .join(symbol)
        .join(POLICY_TRAINING_DIR)
        .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE);
    if !summary_path.exists() {
        bail!(
            "structural path ranking target export missing at {}; export target rows before registering an external trainer artifact",
            summary_path.to_string_lossy()
        );
    }
    let raw = fs::read_to_string(&summary_path)?;
    let summary: StructuralPathRankingTargetExportSummary = serde_json::from_str(&raw)?;
    let evaluation_jsonl_path = if !summary.history_jsonl_path.trim().is_empty() {
        summary.history_jsonl_path.as_str()
    } else {
        summary.jsonl_path.as_str()
    };
    if !structural_path_ranking_trainer_manifest_ready(&summary.trainer_manifest) {
        bail!(
            "structural path ranking trainer manifest incomplete at {}; export target rows with the current repo before registering an external trainer artifact",
            summary_path.to_string_lossy()
        );
    }
    let raw_scored_mature_rows =
        structural_path_ranking_target_raw_scored_mature_rows(evaluation_jsonl_path)?;
    let calibration_evaluation =
        structural_path_ranking_target_calibration_evaluation(evaluation_jsonl_path)?;
    let trained_row_default = if summary.history_rows > 0 {
        summary.history_rows
    } else if summary.rows_with_training_weight > 0 {
        summary.rows_with_training_weight
    } else if summary.mature_rows > 0 {
        summary.mature_rows
    } else {
        summary.rows
    };
    let calibration_row_default = if summary.history_mature_rows > 0 {
        summary.history_mature_rows
    } else {
        raw_scored_mature_rows.max(summary.rows_with_calibrated_path_prob)
    };
    let validation_metrics = StructuralPathRankerValidationMetrics {
        raw_scored_mature_rows,
        raw_scored_mature_min_rows: STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS,
        production_validation_rows: calibration_evaluation.propensity_weighted_rows,
        production_validation_min_rows: STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS,
    };
    let calibration_metrics = StructuralPathRankerCalibrationMetrics {
        eligible_rows: calibration_evaluation.eligible_rows,
        brier_score: calibration_evaluation.brier_score,
        propensity_weighted_brier_score: calibration_evaluation.propensity_weighted_brier_score,
        expected_calibration_error: calibration_evaluation.expected_calibration_error,
        max_calibration_error: calibration_evaluation.max_calibration_error,
    };
    let artifact = StructuralPathRankingTrainerArtifact {
        protocol_version: STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION.to_string(),
        dataset_role: summary.trainer_manifest.dataset_role.clone(),
        model_family: model_family.to_string(),
        artifact_uri: artifact_uri.to_string(),
        model_artifact_uri: None,
        score_column: score_column
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(summary.trainer_manifest.raw_score_column.as_str())
            .to_string(),
        trained_rows: trained_rows.unwrap_or(trained_row_default),
        history_rows: summary.history_rows.max(trained_row_default),
        calibration_rows: calibration_rows.unwrap_or(calibration_row_default),
        selected_features: summary.trainer_manifest.feature_columns.clone(),
        validation_metrics,
        calibration_metrics,
        rule_list: Vec::new(),
        tree_json: None,
        created_at: Some(Utc::now().to_rfc3339()),
        notes: vec![
            "registered_via=explicit_external_artifact".to_string(),
            "uri_source=cli_opt_in".to_string(),
        ],
    };
    let mut artifact = artifact;
    let merged_source = merge_structural_path_ranking_source_artifact(&mut artifact, artifact_uri)?;
    if structural_path_ranker_supports_explicit_family(model_family)
        && (!merged_source || (artifact.rule_list.is_empty() && artifact.tree_json.is_none()))
    {
        bail!(
            "explicit path-ranker family '{}' requires a readable JSON artifact with either rule_list or tree_json",
            model_family
        );
    }
    let artifact_filename =
        format!("{POLICY_TRAINING_DIR}/{STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE}");
    save_text_state(
        state_dir,
        symbol,
        &artifact_filename,
        &serde_json::to_string_pretty(&artifact)?,
    )?;
    let artifact_path = Path::new(state_dir)
        .join(symbol)
        .join(&artifact_filename)
        .to_string_lossy()
        .to_string();
    Ok((artifact_path, artifact))
}

pub fn register_structural_path_ranking_trainer_artifact_command(
    state_dir: &str,
    symbol: &str,
    artifact_uri: &str,
    model_family: &str,
    score_column: Option<&str>,
    trained_rows: Option<usize>,
    calibration_rows: Option<usize>,
) -> Result<()> {
    register_structural_path_ranking_trainer_artifact(
        state_dir,
        symbol,
        artifact_uri,
        model_family,
        score_column,
        trained_rows,
        calibration_rows,
    )?;
    let surface = structural_path_ranking_target_training_status(state_dir, symbol)?;
    println!("{}", serde_json::to_string_pretty(&surface)?);
    Ok(())
}

fn clear_structural_path_ranking_trainer_artifact(state_dir: &str, symbol: &str) -> Result<bool> {
    let artifact_path = Path::new(state_dir)
        .join(symbol)
        .join(POLICY_TRAINING_DIR)
        .join(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE);
    if !artifact_path.exists() {
        return Ok(false);
    }
    fs::remove_file(&artifact_path)?;
    Ok(true)
}

pub fn clear_structural_path_ranking_trainer_artifact_command(
    state_dir: &str,
    symbol: &str,
) -> Result<()> {
    clear_structural_path_ranking_trainer_artifact(state_dir, symbol)?;
    let surface = structural_path_ranking_target_training_status(state_dir, symbol)?;
    println!("{}", serde_json::to_string_pretty(&surface)?);
    Ok(())
}

fn normalize_structural_path_ranking_runtime_reuse_mode(mode: &str) -> Result<&'static str> {
    let mode = mode.trim();
    match mode {
        STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY => {
            Ok(STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY)
        }
        STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY => {
            Ok(STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY)
        }
        _ => bail!(
            "unsupported runtime reuse mode '{}'; expected '{}' or '{}'",
            mode,
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY
        ),
    }
}

fn set_structural_path_ranking_runtime_selection(
    state_dir: &str,
    symbol: &str,
    reuse_mode: &str,
) -> Result<String> {
    let summary_path = Path::new(state_dir)
        .join(symbol)
        .join(POLICY_TRAINING_DIR)
        .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE);
    if !summary_path.exists() {
        bail!(
            "structural path ranking target export missing at {}; export target rows before enabling runtime reuse",
            summary_path.to_string_lossy()
        );
    }
    let reuse_mode = normalize_structural_path_ranking_runtime_reuse_mode(reuse_mode)?;
    let selection = StructuralPathRankingRuntimeSelection {
        protocol_version: STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_PROTOCOL_VERSION.to_string(),
        enabled: true,
        reuse_mode: reuse_mode.to_string(),
        selected_at: Some(Utc::now().to_rfc3339()),
        notes: vec![
            "opt_in_runtime_reuse=true".to_string(),
            "zero_config_default_preserved=true".to_string(),
        ],
    };
    let relative_path = format!(
        "{POLICY_TRAINING_DIR}/{}",
        crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_FILE
    );
    save_text_state(
        state_dir,
        symbol,
        &relative_path,
        &serde_json::to_string_pretty(&selection)?,
    )?;
    Ok(structural_path_ranking_runtime_selection_path(
        state_dir, symbol,
    ))
}

pub fn enable_structural_path_ranking_runtime_command(
    state_dir: &str,
    symbol: &str,
    reuse_mode: &str,
) -> Result<()> {
    set_structural_path_ranking_runtime_selection(state_dir, symbol, reuse_mode)?;
    let surface = structural_path_ranking_target_training_status(state_dir, symbol)?;
    println!("{}", serde_json::to_string_pretty(&surface)?);
    Ok(())
}

fn clear_structural_path_ranking_runtime_selection(state_dir: &str, symbol: &str) -> Result<bool> {
    let selection_path = Path::new(state_dir)
        .join(symbol)
        .join(POLICY_TRAINING_DIR)
        .join(crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_FILE);
    if !selection_path.exists() {
        return Ok(false);
    }
    fs::remove_file(&selection_path)?;
    Ok(true)
}

pub fn disable_structural_path_ranking_runtime_command(
    state_dir: &str,
    symbol: &str,
) -> Result<()> {
    clear_structural_path_ranking_runtime_selection(state_dir, symbol)?;
    let surface = structural_path_ranking_target_training_status(state_dir, symbol)?;
    println!("{}", serde_json::to_string_pretty(&surface)?);
    Ok(())
}

fn export_structural_path_ranking_target_from_state_dir(
    state_dir: &str,
    symbol: &str,
) -> Result<StructuralPathRankingTargetExportSummary> {
    let snapshot = load_workflow_snapshot(state_dir, symbol)?;
    let learning_state = load_learning_state(state_dir, symbol)?;
    let provider_status_agent = provider_status_agent_surface(None, None, None).unwrap_or_default();
    let agent_material_rank = load_latest_agent_material_rank_artifact(state_dir, symbol)?;
    export_structural_path_ranking_target_with_agent_material_rank(
        state_dir,
        symbol,
        &snapshot,
        &provider_status_agent,
        &learning_state.feedback_history,
        &learning_state.structural_prior_state,
        agent_material_rank.as_ref(),
    )
}

fn load_latest_agent_material_rank_artifact(
    state_dir: &str,
    symbol: &str,
) -> Result<Option<crate::application::auto_quant::AgentMaterialRankArtifact>> {
    let ledger = load_artifact_ledger(state_dir, symbol)?;
    let Some(entry) = ledger
        .iter()
        .rev()
        .find(|entry| entry.artifact_kind == "auto_quant_agent_material_rank")
    else {
        return Ok(None);
    };
    let raw = fs::read_to_string(&entry.path)?;
    serde_json::from_str::<crate::application::auto_quant::AgentMaterialRankArtifact>(&raw)
        .map(Some)
        .map_err(Into::into)
}

pub fn export_structural_path_ranking_target_command(state_dir: &str, symbol: &str) -> Result<()> {
    let summary = export_structural_path_ranking_target_from_state_dir(state_dir, symbol)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

pub fn apply_structural_path_ranking_external_scores_command(
    state_dir: &str,
    symbol: &str,
    scores_path: &str,
) -> Result<()> {
    let scores = load_structural_path_ranking_external_scores(scores_path)?;
    if scores.is_empty() {
        bail!(
            "no structural path ranking external scores found in '{}'",
            scores_path
        );
    }
    let summary = apply_structural_path_ranking_external_scores(state_dir, symbol, &scores)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn collect_training_rows(state_dir: &str, symbol: &str) -> Result<CisdRbCollectedTrainingRows> {
    let analyze_runs: Vec<AnalyzeRunRecord> =
        load_state_or_default(state_dir, symbol, ANALYZE_RUNS_FILE)?;
    let update_runs: Vec<UpdateRunRecord> =
        load_state_or_default(state_dir, symbol, UPDATE_RUNS_FILE)?;
    let legacy_packets_by_run_id = load_legacy_cisd_rb_packets(state_dir, symbol)?;
    let analyze_by_id = analyze_runs
        .iter()
        .map(|run| (run.run_id.clone(), run))
        .collect::<BTreeMap<_, _>>();

    let mut bbn_rows = Vec::new();
    let mut catboost_rows = Vec::new();

    for update in &update_runs {
        let Some(analyze_run_id) = update.consumed_analyze_run_id.as_deref() else {
            continue;
        };
        let Some(analyze) = analyze_by_id.get(analyze_run_id) else {
            continue;
        };
        let packet = decode_entry_model_packet::<CisdRbEntryModelPacket>(
            &analyze.entry_model_packets,
            CISD_RB_SETUP_MODEL_ID,
        )
        .or_else(|| legacy_packets_by_run_id.get(analyze_run_id).cloned());
        let Some(packet) = packet else {
            continue;
        };
        let bins = bin_cisd_rb_for_bbn(&packet);
        let hmm = analyze.regime_probs.unwrap_or(RegimeProbs {
            accumulation: 0.0,
            manipulation_expansion: 0.0,
            distribution: 0.0,
        });
        bbn_rows.push(build_bbn_training_row(analyze, update, &packet, &bins));
        catboost_rows.push(build_catboost_training_row(
            analyze, update, &packet, &bins, &hmm,
        ));
    }

    Ok(CisdRbCollectedTrainingRows {
        analyze_runs: analyze_runs.len(),
        update_runs: update_runs.len(),
        bbn_rows,
        catboost_rows,
    })
}

fn collect_breaker_training_rows(
    state_dir: &str,
    symbol: &str,
) -> Result<BreakerCollectedTrainingRows> {
    let analyze_runs: Vec<AnalyzeRunRecord> =
        load_state_or_default(state_dir, symbol, ANALYZE_RUNS_FILE)?;
    let update_runs: Vec<UpdateRunRecord> =
        load_state_or_default(state_dir, symbol, UPDATE_RUNS_FILE)?;
    let analyze_by_id = analyze_runs
        .iter()
        .map(|run| (run.run_id.clone(), run))
        .collect::<BTreeMap<_, _>>();

    let mut bbn_rows = Vec::new();
    let mut catboost_rows = Vec::new();

    for update in &update_runs {
        let Some(analyze_run_id) = update.consumed_analyze_run_id.as_deref() else {
            continue;
        };
        let Some(analyze) = analyze_by_id.get(analyze_run_id) else {
            continue;
        };
        let Some(packet) = decode_entry_model_packet::<BreakerRbEntryModelPacket>(
            &analyze.entry_model_packets,
            BREAKER_RB_SETUP_MODEL_ID,
        ) else {
            continue;
        };
        let bins = bin_breaker_rb_for_bbn(&packet);
        let hmm = analyze.regime_probs.unwrap_or(RegimeProbs {
            accumulation: 0.0,
            manipulation_expansion: 0.0,
            distribution: 0.0,
        });
        bbn_rows.push(build_breaker_bbn_training_row(
            analyze, update, &packet, &bins,
        ));
        catboost_rows.push(build_breaker_catboost_training_row(
            analyze, update, &packet, &bins, &hmm,
        ));
    }

    Ok(BreakerCollectedTrainingRows {
        analyze_runs: analyze_runs.len(),
        update_runs: update_runs.len(),
        bbn_rows,
        catboost_rows,
    })
}

fn persist_training_rows(
    state_dir: &str,
    symbol: &str,
    rows: &EntryModelTrainingRows,
) -> Result<()> {
    let symbol_dir = Path::new(state_dir).join(symbol).join(POLICY_TRAINING_DIR);
    fs::create_dir_all(&symbol_dir)?;
    save_text_state(
        state_dir,
        symbol,
        &format!("{POLICY_TRAINING_DIR}/{}", rows.bbn_training_filename),
        &rows.bbn_csv,
    )?;
    save_text_state(
        state_dir,
        symbol,
        &format!("{POLICY_TRAINING_DIR}/{}", rows.catboost_training_filename),
        &rows.catboost_csv,
    )?;
    save_text_state(
        state_dir,
        symbol,
        &format!("{POLICY_TRAINING_DIR}/{}", rows.summary_filename),
        &rows.summary_json,
    )?;
    Ok(())
}

fn load_legacy_cisd_rb_packets(
    state_dir: &str,
    symbol: &str,
) -> Result<BTreeMap<String, CisdRbEntryModelPacket>> {
    let path = Path::new(state_dir).join(symbol).join(ANALYZE_RUNS_FILE);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let raw = fs::read_to_string(&path)?;
    let value: Value = serde_json::from_str(&raw)?;
    let Some(items) = value.as_array() else {
        return Ok(BTreeMap::new());
    };
    let mut out = BTreeMap::new();
    for item in items {
        let Some(run_id) = item.get("run_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(packet_value) = item.get("cisd_rb_entry_model_packet") else {
            continue;
        };
        let Some(packet) =
            serde_json::from_value::<CisdRbEntryModelPacket>(packet_value.clone()).ok()
        else {
            continue;
        };
        out.insert(run_id.to_string(), packet);
    }
    Ok(out)
}

fn build_bbn_status(rows: &[CisdRbBbnTrainingRow]) -> BbnTrainingStatusSurface {
    let outcome_counts = count_strings(rows.iter().map(|row| row.realized_outcome.as_str()));
    let entry_quality_counts = count_strings(rows.iter().map(|row| row.entry_quality.as_str()));
    let trigger_confirmation_quality_counts = count_strings(
        rows.iter()
            .map(|row| row.trigger_confirmation_quality.as_str()),
    );
    let session_quality_counts = count_strings(rows.iter().map(|row| row.session_quality.as_str()));
    build_generic_bbn_status(
        rows.len(),
        outcome_counts,
        entry_quality_counts,
        trigger_confirmation_quality_counts,
        session_quality_counts,
    )
}

fn build_generic_bbn_status(
    rows_len: usize,
    outcome_counts: BTreeMap<String, usize>,
    entry_quality_counts: BTreeMap<String, usize>,
    trigger_confirmation_quality_counts: BTreeMap<String, usize>,
    session_quality_counts: BTreeMap<String, usize>,
) -> BbnTrainingStatusSurface {
    let mut warnings = Vec::new();
    if rows_len < 30 {
        warnings.push(format!("matched_rows_below_minimum: {}", rows_len));
    }
    if outcome_counts.len() < 2 {
        warnings.push("outcome_labels_do_not_cover_win_loss".to_string());
    }
    if entry_quality_counts.len() < 2 {
        warnings.push("entry_quality_bins_have_low_diversity".to_string());
    }
    if trigger_confirmation_quality_counts.len() < 2 {
        warnings.push("trigger_confirmation_bins_have_low_diversity".to_string());
    }
    let ready = warnings.is_empty();
    BbnTrainingStatusSurface {
        ready,
        rows: rows_len,
        outcome_counts,
        entry_quality_counts,
        trigger_confirmation_quality_counts,
        session_quality_counts,
        warnings,
    }
}

fn build_catboost_status(rows: &[CisdRbCatBoostTrainingRow]) -> CatBoostTrainingStatusSurface {
    let outcome_counts = count_strings(rows.iter().map(|row| row.realized_outcome.as_str()));
    let numeric_ranges = BTreeMap::from([
        (
            "cisd_impulse_atr".to_string(),
            range_of(rows.iter().map(|row| row.cisd_impulse_atr)),
        ),
        (
            "rb_wick_body_ratio".to_string(),
            range_of(rows.iter().map(|row| row.rb_wick_body_ratio)),
        ),
        (
            "bars_between_cisd_and_rb".to_string(),
            range_of(rows.iter().map(|row| row.bars_between_cisd_and_rb)),
        ),
        (
            "ema19_distance_bps".to_string(),
            range_of(rows.iter().map(|row| row.ema19_distance_bps)),
        ),
        (
            "realized_vol_zscore".to_string(),
            range_of(rows.iter().map(|row| row.realized_vol_zscore)),
        ),
    ]);
    build_generic_catboost_status(rows.len(), outcome_counts, numeric_ranges)
}

fn build_generic_catboost_status(
    rows_len: usize,
    outcome_counts: BTreeMap<String, usize>,
    numeric_ranges: BTreeMap<String, NumericRangeSummary>,
) -> CatBoostTrainingStatusSurface {
    let varying_features = numeric_ranges
        .values()
        .filter(|range| range.span > 0.0)
        .count();
    let mut warnings = Vec::new();
    if rows_len < 50 {
        warnings.push(format!(
            "matched_rows_below_recommended_minimum: {}",
            rows_len
        ));
    }
    if outcome_counts.len() < 2 {
        warnings.push("outcome_labels_do_not_cover_win_loss".to_string());
    }
    if varying_features < 4 {
        warnings.push(format!(
            "numeric_feature_variation_too_low: {varying_features}/5 varying"
        ));
    }
    let ready = warnings.is_empty();
    CatBoostTrainingStatusSurface {
        ready,
        rows: rows_len,
        outcome_counts,
        numeric_ranges,
        warnings,
    }
}

fn build_provider_summary_line(
    label: &str,
    matched_rows: usize,
    bbn: &BbnTrainingStatusSurface,
    catboost: &CatBoostTrainingStatusSurface,
) -> String {
    if bbn.ready && catboost.ready {
        format!(
            "{label} policy training looks healthy for BBN and CatBoost: matched_rows={} outcomes={}",
            matched_rows,
            format_counts(&catboost.outcome_counts)
        )
    } else if !bbn.ready && !catboost.ready {
        format!(
            "{label} policy training is not ready for either BBN or CatBoost: matched_rows={} bbn_warnings={} catboost_warnings={}",
            matched_rows,
            bbn.warnings.join("; "),
            catboost.warnings.join("; ")
        )
    } else if !bbn.ready {
        format!(
            "{label} policy training is CatBoost-usable but BBN-weak: matched_rows={} bbn_warnings={}",
            matched_rows,
            bbn.warnings.join("; ")
        )
    } else {
        format!(
            "{label} policy training is BBN-usable but CatBoost-weak: matched_rows={} catboost_warnings={}",
            matched_rows,
            catboost.warnings.join("; ")
        )
    }
}

fn count_strings<'a>(values: impl Iterator<Item = &'a str>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0) += 1;
    }
    counts
}

fn range_of(values: impl Iterator<Item = f64>) -> NumericRangeSummary {
    let vals = values.collect::<Vec<_>>();
    if vals.is_empty() {
        return NumericRangeSummary::default();
    }
    let min = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let max = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    NumericRangeSummary {
        min,
        max,
        span: max - min,
    }
}

fn format_counts(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(label, count)| format!("{label}={count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn build_bbn_training_row(
    analyze: &AnalyzeRunRecord,
    update: &UpdateRunRecord,
    packet: &CisdRbEntryModelPacket,
    bins: &CisdRbBbnEvidence,
) -> CisdRbBbnTrainingRow {
    CisdRbBbnTrainingRow {
        analyze_run_id: analyze.run_id.clone(),
        update_run_id: update.run_id.clone(),
        symbol: analyze.symbol.clone(),
        timeframe: packet.timeframe.clone(),
        setup_model_id: packet.setup_model_id.clone(),
        trend_alignment: bins.trend_alignment.clone(),
        liquidity_interaction_quality: bins.liquidity_interaction_quality.clone(),
        trigger_confirmation_quality: bins.trigger_confirmation_quality.clone(),
        session_quality: bins.session_quality.clone(),
        entry_quality: bins.entry_quality.clone(),
        evidence_quality_score: packet.evidence_quality_score,
        gating_status: analyze.pre_bayes_evidence_filter.gating_status.clone(),
        realized_outcome: update.realized_outcome.clone(),
    }
}

fn build_breaker_bbn_training_row(
    analyze: &AnalyzeRunRecord,
    update: &UpdateRunRecord,
    packet: &BreakerRbEntryModelPacket,
    bins: &BreakerRbBbnEvidence,
) -> BreakerRbBbnTrainingRow {
    BreakerRbBbnTrainingRow {
        analyze_run_id: analyze.run_id.clone(),
        update_run_id: update.run_id.clone(),
        symbol: analyze.symbol.clone(),
        timeframe: packet.timeframe.clone(),
        setup_model_id: packet.setup_model_id.clone(),
        trend_alignment: bins.trend_alignment.clone(),
        breaker_retest_quality: bins.breaker_retest_quality.clone(),
        session_quality: bins.session_quality.clone(),
        entry_quality: bins.entry_quality.clone(),
        evidence_quality_score: packet.evidence_quality_score,
        gating_status: analyze.pre_bayes_evidence_filter.gating_status.clone(),
        realized_outcome: update.realized_outcome.clone(),
    }
}

fn build_catboost_training_row(
    analyze: &AnalyzeRunRecord,
    update: &UpdateRunRecord,
    packet: &CisdRbEntryModelPacket,
    bins: &CisdRbBbnEvidence,
    hmm: &RegimeProbs,
) -> CisdRbCatBoostTrainingRow {
    let row = build_cisd_rb_catboost_feature_row(packet, hmm, bins);
    CisdRbCatBoostTrainingRow {
        analyze_run_id: analyze.run_id.clone(),
        update_run_id: update.run_id.clone(),
        symbol: analyze.symbol.clone(),
        timeframe: packet.timeframe.clone(),
        setup_model_id: row.setup_model_id,
        setup_progress_state: row.setup_progress_state,
        hmm_accumulation_prob: row.hmm_accumulation_prob,
        hmm_manipulation_expansion_prob: row.hmm_manipulation_expansion_prob,
        hmm_distribution_prob: row.hmm_distribution_prob,
        bbn_trend_alignment: row.bbn_trend_alignment,
        bbn_liquidity_interaction_quality: row.bbn_liquidity_interaction_quality,
        bbn_trigger_confirmation_quality: row.bbn_trigger_confirmation_quality,
        bbn_session_quality: row.bbn_session_quality,
        bbn_entry_quality: row.bbn_entry_quality,
        cisd_run_length_observed: row.cisd_run_length_observed,
        cisd_impulse_atr: row.cisd_impulse_atr,
        cisd_body_ratio_mean: row.cisd_body_ratio_mean,
        rb_wick_body_ratio: row.rb_wick_body_ratio,
        rb_close_location_ratio: row.rb_close_location_ratio,
        bars_between_cisd_and_rb: row.bars_between_cisd_and_rb,
        seq_window_hit: row.seq_window_hit,
        ema19_distance_bps: row.ema19_distance_bps,
        atr14_bps: row.atr14_bps,
        realized_vol_zscore: row.realized_vol_zscore,
        evidence_quality_score: row.evidence_quality_score,
        session_label: row.session_label,
        realized_outcome: update.realized_outcome.clone(),
    }
}

fn build_breaker_catboost_training_row(
    analyze: &AnalyzeRunRecord,
    update: &UpdateRunRecord,
    packet: &BreakerRbEntryModelPacket,
    bins: &BreakerRbBbnEvidence,
    hmm: &RegimeProbs,
) -> BreakerRbCatBoostTrainingRow {
    let row = build_breaker_rb_catboost_feature_row(packet, hmm, bins);
    BreakerRbCatBoostTrainingRow {
        analyze_run_id: analyze.run_id.clone(),
        update_run_id: update.run_id.clone(),
        symbol: analyze.symbol.clone(),
        timeframe: packet.timeframe.clone(),
        setup_model_id: row.setup_model_id,
        setup_progress_state: row.setup_progress_state,
        hmm_accumulation_prob: row.hmm_accumulation_prob,
        hmm_manipulation_expansion_prob: row.hmm_manipulation_expansion_prob,
        hmm_distribution_prob: row.hmm_distribution_prob,
        bbn_trend_alignment: row.bbn_trend_alignment,
        bbn_breaker_retest_quality: row.bbn_breaker_retest_quality,
        bbn_session_quality: row.bbn_session_quality,
        bbn_entry_quality: row.bbn_entry_quality,
        bars_between_violation_and_retest: row.bars_between_violation_and_retest,
        breaker_width_bps: row.breaker_width_bps,
        retest_reclaim_bps: row.retest_reclaim_bps,
        rb_wick_body_ratio: row.rb_wick_body_ratio,
        rb_close_location_ratio: row.rb_close_location_ratio,
        ema19_distance_bps: row.ema19_distance_bps,
        atr14_bps: row.atr14_bps,
        realized_vol_zscore: row.realized_vol_zscore,
        evidence_quality_score: row.evidence_quality_score,
        session_label: row.session_label,
        realized_outcome: update.realized_outcome.clone(),
    }
}

fn render_bbn_training_csv(rows: &[CisdRbBbnTrainingRow]) -> String {
    let mut out = String::from(
        "analyze_run_id,update_run_id,symbol,timeframe,setup_model_id,trend_alignment,liquidity_interaction_quality,trigger_confirmation_quality,session_quality,entry_quality,evidence_quality_score,gating_status,realized_outcome\n",
    );
    for row in rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{:.6},{},{}\n",
            row.analyze_run_id,
            row.update_run_id,
            row.symbol,
            row.timeframe,
            row.setup_model_id,
            row.trend_alignment,
            row.liquidity_interaction_quality,
            row.trigger_confirmation_quality,
            row.session_quality,
            row.entry_quality,
            row.evidence_quality_score,
            row.gating_status,
            row.realized_outcome
        ));
    }
    out
}

fn render_catboost_training_csv(rows: &[CisdRbCatBoostTrainingRow]) -> String {
    let mut out = String::from(
        "analyze_run_id,update_run_id,symbol,timeframe,setup_model_id,setup_progress_state,hmm_accumulation_prob,hmm_manipulation_expansion_prob,hmm_distribution_prob,bbn_trend_alignment,bbn_liquidity_interaction_quality,bbn_trigger_confirmation_quality,bbn_session_quality,bbn_entry_quality,cisd_run_length_observed,cisd_impulse_atr,cisd_body_ratio_mean,rb_wick_body_ratio,rb_close_location_ratio,bars_between_cisd_and_rb,seq_window_hit,ema19_distance_bps,atr14_bps,realized_vol_zscore,evidence_quality_score,session_label,realized_outcome\n",
    );
    for row in rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{:.6},{:.6},{:.6},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{:.6},{:.6},{:.6},{:.6},{},{}\n",
            row.analyze_run_id,
            row.update_run_id,
            row.symbol,
            row.timeframe,
            row.setup_model_id,
            row.setup_progress_state,
            row.hmm_accumulation_prob,
            row.hmm_manipulation_expansion_prob,
            row.hmm_distribution_prob,
            row.bbn_trend_alignment,
            row.bbn_liquidity_interaction_quality,
            row.bbn_trigger_confirmation_quality,
            row.bbn_session_quality,
            row.bbn_entry_quality,
            row.cisd_run_length_observed,
            row.cisd_impulse_atr,
            row.cisd_body_ratio_mean,
            row.rb_wick_body_ratio,
            row.rb_close_location_ratio,
            row.bars_between_cisd_and_rb,
            row.seq_window_hit,
            row.ema19_distance_bps,
            row.atr14_bps,
            row.realized_vol_zscore,
            row.evidence_quality_score,
            row.session_label,
            row.realized_outcome
        ));
    }
    out
}

fn render_breaker_bbn_training_csv(rows: &[BreakerRbBbnTrainingRow]) -> String {
    let mut out = String::from(
        "analyze_run_id,update_run_id,symbol,timeframe,setup_model_id,trend_alignment,breaker_retest_quality,session_quality,entry_quality,evidence_quality_score,gating_status,realized_outcome\n",
    );
    for row in rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{:.6},{},{}\n",
            row.analyze_run_id,
            row.update_run_id,
            row.symbol,
            row.timeframe,
            row.setup_model_id,
            row.trend_alignment,
            row.breaker_retest_quality,
            row.session_quality,
            row.entry_quality,
            row.evidence_quality_score,
            row.gating_status,
            row.realized_outcome
        ));
    }
    out
}

fn render_breaker_catboost_training_csv(rows: &[BreakerRbCatBoostTrainingRow]) -> String {
    let mut out = String::from(
        "analyze_run_id,update_run_id,symbol,timeframe,setup_model_id,setup_progress_state,hmm_accumulation_prob,hmm_manipulation_expansion_prob,hmm_distribution_prob,bbn_trend_alignment,bbn_breaker_retest_quality,bbn_session_quality,bbn_entry_quality,bars_between_violation_and_retest,breaker_width_bps,retest_reclaim_bps,rb_wick_body_ratio,rb_close_location_ratio,ema19_distance_bps,atr14_bps,realized_vol_zscore,evidence_quality_score,session_label,realized_outcome\n",
    );
    for row in rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{:.6},{:.6},{:.6},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{}\n",
            row.analyze_run_id,
            row.update_run_id,
            row.symbol,
            row.timeframe,
            row.setup_model_id,
            row.setup_progress_state,
            row.hmm_accumulation_prob,
            row.hmm_manipulation_expansion_prob,
            row.hmm_distribution_prob,
            row.bbn_trend_alignment,
            row.bbn_breaker_retest_quality,
            row.bbn_session_quality,
            row.bbn_entry_quality,
            row.bars_between_violation_and_retest,
            row.breaker_width_bps,
            row.retest_reclaim_bps,
            row.rb_wick_body_ratio,
            row.rb_close_location_ratio,
            row.ema19_distance_bps,
            row.atr14_bps,
            row.realized_vol_zscore,
            row.evidence_quality_score,
            row.session_label,
            row.realized_outcome
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::entry_models::{
        insert_entry_model_packet, EntryModelPacketStore, CISD_RB_SETUP_MODEL_ID,
    };
    use crate::state::{
        save_learning_state, save_state, AnalyzeRunRecord, FeedbackRecord, LearningState,
        ModelProbabilitySnapshot, StructuralFeedbackRefs, UpdateRunRecord,
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn sample_packet() -> CisdRbEntryModelPacket {
        CisdRbEntryModelPacket {
            setup_model_id: CISD_RB_SETUP_MODEL_ID.to_string(),
            symbol: "NQ".to_string(),
            timeframe: "5m".to_string(),
            direction: "long".to_string(),
            cisd_bars_required: 3,
            cisd_run_length_observed: 3,
            cisd_impulse_atr: 1.2,
            cisd_body_ratio_mean: 0.7,
            rb_wick_body_ratio: 1.3,
            rb_close_location_ratio: 0.7,
            rb_bullish: true,
            bars_between_cisd_and_rb: 4,
            seq_window_limit: 18,
            seq_window_hit: true,
            ema19_distance_bps: 12.0,
            atr14_bps: 25.0,
            realized_vol_zscore: 0.4,
            session_label: "ny_open".to_string(),
            liquidity_swept: true,
            mss_up: true,
            filtered_market_regime_label: "bull".to_string(),
            filtered_liquidity_context_label: "favorable".to_string(),
            filtered_resonance_label: "aligned".to_string(),
            evidence_quality_score: 0.72,
        }
    }

    fn serve_http_response_with_method(
        path: &str,
        body: String,
        request_count: usize,
        method: &str,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("listener addr");
        let expected_path = format!("/{path}");
        let expected_method = method.to_string();
        let response_path = expected_path.clone();
        thread::spawn(move || {
            for _ in 0..request_count {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buffer = [0_u8; 4096];
                    let read = stream.read(&mut buffer).unwrap_or_default();
                    let request = String::from_utf8_lossy(&buffer[..read]);
                    assert!(request.starts_with(&format!("{expected_method} ")));
                    assert!(request.contains(&expected_path));
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
            }
        });
        format!("http://{address}{response_path}")
    }

    fn structural_path_ranking_row(
        path_id: &str,
        calibrated_path_prob: f64,
        pending_reward_state: &str,
    ) -> StructuralPathRankingTargetRow {
        StructuralPathRankingTargetRow {
            rank: 1,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            path_id: path_id.to_string(),
            scenario_id: format!("scenario:{path_id}"),
            path_label: path_id.to_string(),
            regime_profit_branch_path: None,
            parent_regime_root: None,
            main_regime: None,
            sub_regime: None,
            sub_sub_regime_or_profit_factor: None,
            profit_factor: None,
            direction: "bull".to_string(),
            raw_path_score: Some(calibrated_path_prob),
            calibrated_path_prob: Some(calibrated_path_prob),
            path_prob_lower_bound: Some((calibrated_path_prob - 0.1).clamp(0.0, 1.0)),
            execution_gate_status: None,
            execution_gate_min_path_prob: None,
            execution_gate_reason: None,
            pending_reward_state: pending_reward_state.to_string(),
            maturity_mask: matches!(
                pending_reward_state,
                "matured_success" | "matured_failure" | "matured_invalidated"
            ),
            maturity_weight: if matches!(
                pending_reward_state,
                "matured_success" | "matured_failure" | "matured_invalidated"
            ) {
                1.0
            } else {
                0.0
            },
            calibrated_label: match pending_reward_state {
                "matured_success" => Some(1.0),
                "matured_failure" | "matured_invalidated" => Some(0.0),
                _ => None,
            },
            propensity_estimate: Some(0.5),
            ips_weight: Some(2.0),
            training_weight: if matches!(
                pending_reward_state,
                "matured_success" | "matured_failure" | "matured_invalidated"
            ) {
                Some(2.0)
            } else {
                None
            },
            regime_calibration_bucket: "NQ:trend".to_string(),
            behavior_policy_probability: 0.5,
            execution_propensity: Some(0.5),
            target_policy_probability_confidence: Some(0.55),
            target_policy_probability_lower_bound: Some(0.30),
            target_policy_reward_prior: Some(0.58),
            target_policy_reward_lower_bound: Some(0.28),
            experience_prior: 0.5,
            current_posterior: 0.5,
            structural_baseline_score: 0.5,
            regime_aux_qqq_hv_level: None,
            regime_aux_nq_vs_200d_pct: None,
            regime_aux_vix3m_level: None,
            regime_aux_qqq_hv_pct_rank_252: None,
            regime_aux_vvix_over_vix: None,
            score_model_family: None,
            score_source_kind: None,
            score_model_artifact_uri: None,
            score_generator: None,
        }
    }

    fn structural_path_ranking_trainer_manifest_for_test() -> StructuralPathRankingTrainerManifest {
        StructuralPathRankingTrainerManifest {
            protocol_version: "structural-path-ranking-trainer-manifest-v1".to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            group_id_column: "candidate_set_id".to_string(),
            label_column: "calibrated_label".to_string(),
            weight_column: "training_weight".to_string(),
            maturity_column: "maturity_mask".to_string(),
            raw_score_column: "raw_path_score".to_string(),
            feature_columns: vec!["rank".to_string(), "raw_path_score".to_string()],
            calibration_columns: vec!["calibrated_path_prob".to_string()],
            guardrail_columns: vec!["candidate_set_size".to_string()],
            notes: Vec::new(),
        }
    }

    fn structural_path_ranking_trainer_artifact_for_test() -> StructuralPathRankingTrainerArtifact {
        StructuralPathRankingTrainerArtifact {
            protocol_version: STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION.to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: "catboost".to_string(),
            artifact_uri: "/opt/external/path-ranker/model.cbm".to_string(),
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "raw_path_score".to_string()],
            validation_metrics: StructuralPathRankerValidationMetrics::default(),
            calibration_metrics: StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: Some("2026-05-02T00:00:00Z".to_string()),
            notes: Vec::new(),
        }
    }

    #[test]
    fn exports_training_tables_from_matched_histories() {
        let temp = tempfile::tempdir().unwrap();
        let mut entry_model_packets = EntryModelPacketStore::default();
        insert_entry_model_packet(
            &mut entry_model_packets,
            CISD_RB_SETUP_MODEL_ID,
            &sample_packet(),
        )
        .unwrap();
        let analyze = AnalyzeRunRecord {
            run_id: "analyze:1".to_string(),
            symbol: "NQ".to_string(),
            regime_probs: Some(RegimeProbs {
                accumulation: 0.1,
                manipulation_expansion: 0.8,
                distribution: 0.1,
            }),
            entry_model_packets,
            pre_bayes_evidence_filter: crate::state::PreBayesEvidenceFilter {
                gating_status: "pass_hard".to_string(),
                ..Default::default()
            },
            ..AnalyzeRunRecord::default()
        };
        let update = UpdateRunRecord {
            run_id: "update:1".to_string(),
            symbol: "NQ".to_string(),
            consumed_analyze_run_id: Some("analyze:1".to_string()),
            realized_outcome: "win".to_string(),
            ..UpdateRunRecord::default()
        };
        save_state(temp.path(), "NQ", ANALYZE_RUNS_FILE, &[analyze]).unwrap();
        save_state(temp.path(), "NQ", UPDATE_RUNS_FILE, &[update]).unwrap();

        let summary = export_cisd_rb_training_tables(temp.path().to_str().unwrap(), "NQ").unwrap();
        assert_eq!(summary.matched_rows, 1);
        assert!(Path::new(&summary.bbn_training_path).exists());
        assert!(Path::new(&summary.catboost_training_path).exists());
    }

    #[test]
    fn builds_status_surface_from_matched_histories() {
        let temp = tempfile::tempdir().unwrap();
        let mut entry_model_packets = EntryModelPacketStore::default();
        insert_entry_model_packet(
            &mut entry_model_packets,
            CISD_RB_SETUP_MODEL_ID,
            &sample_packet(),
        )
        .unwrap();
        let analyze = AnalyzeRunRecord {
            run_id: "analyze:1".to_string(),
            symbol: "NQ".to_string(),
            regime_probs: Some(RegimeProbs {
                accumulation: 0.1,
                manipulation_expansion: 0.8,
                distribution: 0.1,
            }),
            entry_model_packets,
            pre_bayes_evidence_filter: crate::state::PreBayesEvidenceFilter {
                gating_status: "pass_hard".to_string(),
                ..Default::default()
            },
            ..AnalyzeRunRecord::default()
        };
        let update_win = UpdateRunRecord {
            run_id: "update:1".to_string(),
            symbol: "NQ".to_string(),
            consumed_analyze_run_id: Some("analyze:1".to_string()),
            realized_outcome: "win".to_string(),
            ..UpdateRunRecord::default()
        };
        let update_loss = UpdateRunRecord {
            run_id: "update:2".to_string(),
            symbol: "NQ".to_string(),
            consumed_analyze_run_id: Some("analyze:1".to_string()),
            realized_outcome: "loss".to_string(),
            ..UpdateRunRecord::default()
        };
        save_state(temp.path(), "NQ", ANALYZE_RUNS_FILE, &[analyze]).unwrap();
        save_state(
            temp.path(),
            "NQ",
            UPDATE_RUNS_FILE,
            &[update_win, update_loss],
        )
        .unwrap();

        let status = cisd_rb_training_status(temp.path().to_str().unwrap(), "NQ").unwrap();
        assert_eq!(status.matched_rows, 2);
        assert_eq!(status.setup_model_ids.get(CISD_RB_SETUP_MODEL_ID), Some(&2));
        assert!(status.summary_line.contains("matched_rows=2"));
        assert!(!status.bbn.ready);
        assert!(!status.catboost.ready);
    }

    #[test]
    fn policy_training_status_lists_registered_providers() {
        let temp = tempfile::tempdir().unwrap();
        let status = policy_training_status(temp.path().to_str().unwrap(), "NQ", None).unwrap();
        let provider_ids = status
            .providers
            .iter()
            .map(|provider| provider.provider_id.as_str())
            .collect::<Vec<_>>();
        assert!(provider_ids.contains(&CISD_RB_SETUP_MODEL_ID));
        assert!(provider_ids.contains(&BREAKER_RB_SETUP_MODEL_ID));
        assert!(!status.structural_path_ranking_target.export_ready);
        assert!(!status.structural_path_ranking_runtime.enabled);
        assert_eq!(status.structural_path_ranking_runtime.status, "disabled");
        assert!(status.structural_path_ranking_runtime.source_kind.is_none());
        assert!(!status.structural_path_ranking_validation.calibration_ready);
        assert_eq!(
            status.structural_path_ranking_validation.calibration_status,
            "not_fitted"
        );
        assert!(
            !status
                .structural_path_ranking_validation
                .production_validation_ready
        );
        assert!(status
            .structural_path_ranking_target
            .warnings
            .contains(&"structural_path_ranking_target_export_missing".to_string()));
        assert!(status
            .structural_path_ranking_runtime_summary
            .contains("Ranker runtime:"));
        assert!(status
            .structural_path_ranking_validation_summary
            .contains("Ranker validation:"));
        assert_eq!(status.factor_candidate_packs.inventory_status, "missing");
        assert!(status
            .factor_candidate_packs
            .summary_line
            .contains("inventory=missing"));
        assert!(status
            .summary_line
            .contains("structural path ranking target export missing"));
    }

    #[test]
    fn policy_training_status_reads_factor_candidate_pack_inventory() {
        let temp = tempfile::tempdir().unwrap();
        let inventory = serde_json::json!({
            "schema_version": "factor-candidate-pack-inventory/v1",
            "summary": {
                "candidate_pack_count": 2
            },
            "candidates": [
                {
                    "candidate_id": "a",
                    "aggregate_label": "preferred_density",
                    "transfer_status": "cross_market_candidate"
                },
                {
                    "candidate_id": "b",
                    "aggregate_label": "probe_only",
                    "transfer_status": "single_market_only"
                }
            ]
        });
        crate::state::save_state(
            temp.path(),
            "FACTOR_CANDIDATES",
            "factor_candidate_pack_inventory.json",
            &inventory,
        )
        .unwrap();
        append_artifact_ledger_entry(
            temp.path(),
            "FACTOR_CANDIDATES",
            ArtifactLedgerEntry {
                entry_id: "ledger:factor-candidate-pack-inventory:test".to_string(),
                artifact_kind: "factor_candidate_pack_inventory".to_string(),
                artifact_id: "factor-candidate-pack-inventory:test".to_string(),
                version: 1,
                generated_at: Utc::now(),
                symbol: "FACTOR_CANDIDATES".to_string(),
                source_phase: "factor-candidate-packs".to_string(),
                source_run_id: None,
                path: temp
                    .path()
                    .join("FACTOR_CANDIDATES")
                    .join("factor_candidate_pack_inventory.json")
                    .to_string_lossy()
                    .to_string(),
                status: "ready".to_string(),
                promote_candidate: false,
                actionable: false,
                decision_hint: "inspect_candidate_packs_before_admission".to_string(),
                review_reason: "candidate_pack_count=2".to_string(),
                review_rule_version: "factor-candidate-pack-inventory/v1".to_string(),
                top_factor_name: None,
                top_factor_action: Some("inspect".to_string()),
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 2,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();

        let status =
            policy_training_status(temp.path().to_str().unwrap(), "FACTOR_CANDIDATES", None)
                .unwrap();

        assert!(status.factor_candidate_packs.inventory_ready);
        assert_eq!(status.factor_candidate_packs.candidate_pack_count, 2);
        assert_eq!(status.factor_candidate_packs.preferred_density_count, 1);
        assert_eq!(
            status.factor_candidate_packs.cross_market_candidate_count,
            1
        );
        assert!(status
            .summary_line
            .contains("Factor candidate packs: inventory=ready count=2"));
    }

    #[test]
    fn structural_path_ranking_target_training_status_reads_summary() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let symbol_dir = temp.path().join("NQ");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 3,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 3,
            mature_rows: 0,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 0,
            rows_with_path_prob_lower_bound: 0,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: summary_dir
                .join("structural_path_ranking_target.jsonl")
                .to_string_lossy()
                .to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            summary_line: "structural_path_ranking_target rows=3".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let mut pending_artifact = crate::state::PendingUpdateArtifact {
            symbol: "NQ".to_string(),
            ..crate::state::PendingUpdateArtifact::default()
        };
        pending_artifact.template_feedback.structural_feedback =
            Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-1".to_string(),
                recommended_at: "2026-05-02T00:00:00Z".to_string(),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                    .to_string(),
                path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                    .to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            });
        std::fs::write(
            symbol_dir.join(crate::state::PENDING_UPDATE_ARTIFACT_FILE),
            serde_json::to_string_pretty(&pending_artifact).unwrap(),
        )
        .unwrap();
        std::fs::write(
            symbol_dir.join(crate::state::PENDING_UPDATE_HISTORY_FILE),
            serde_json::to_string_pretty(&vec![pending_artifact]).unwrap(),
        )
        .unwrap();
        let legacy_feedback = crate::state::FeedbackRecord {
            timestamp: chrono::Utc::now(),
            symbol: "NQ".to_string(),
            source: "legacy_feedback".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: Vec::new(),
            model_probabilities_before_trade: crate::state::ModelProbabilitySnapshot {
                selected_direction: crate::types::Direction::Neutral,
                selected_probability: 0.0,
                long_score: 0.0,
                short_score: 0.0,
                win_prob_long: 0.0,
                win_prob_short: 0.0,
                uncertainty: 0.0,
            },
            realized_outcome: "win".to_string(),
            pnl: 1.0,
            regime_at_entry: crate::types::Regime::ManipulationExpansion,
            structural_feedback: None,
            reflection_mismatch_tags: Vec::new(),
        };
        crate::state::save_learning_state(
            temp.path(),
            "NQ",
            &crate::state::LearningState {
                feedback_history: vec![legacy_feedback],
                ..crate::state::LearningState::default()
            },
        )
        .unwrap();

        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();

        assert!(status.export_ready);
        assert!(!status.calibration_ready);
        assert_eq!(status.rows, 3);
        assert_eq!(status.mature_rows, 0);
        assert_eq!(status.history_rows, 0);
        assert_eq!(status.history_mature_rows, 0);
        assert_eq!(status.history_rows_with_raw_path_score, 0);
        assert_eq!(status.history_rows_with_calibrated_path_prob, 0);
        assert_eq!(status.history_rows_with_path_prob_lower_bound, 0);
        assert_eq!(status.history_rows_with_propensity_estimate, 2);
        assert_eq!(status.update_runs_with_structural_feedback, 0);
        assert_eq!(status.feedback_rows_with_structural_feedback, 0);
        assert_eq!(status.feedback_rows_matured, 0);
        assert_eq!(status.feedback_rows_pending, 0);
        assert!(status.pending_update_artifact_present);
        assert_eq!(status.pending_update_history_rows, 1);
        assert_eq!(status.pending_update_templates_with_structural_feedback, 1);
        assert_eq!(status.feedback_rows_total, 1);
        assert_eq!(status.feedback_rows_without_structural_feedback, 1);
        assert_eq!(
            status
                .feedback_rows_without_structural_feedback_dominant_source
                .as_deref(),
            Some("legacy_feedback")
        );
        assert_eq!(
            status.feedback_rows_without_structural_feedback_dominant_count,
            1
        );
        assert_eq!(
            status.candidate_set_id.as_deref(),
            Some("structural-candidates:NQ:test")
        );
        assert_eq!(status.rows_with_propensity_estimate, 2);
        assert!(status.summary_line.contains("raw_scored_mature=0/30"));
        assert!(status.summary_line.contains("production_validation=0/30"));
        assert!(status.summary_line.contains("observation_validation=0/30"));
        assert_eq!(status.observation_validation_rows, 0);
        assert_eq!(status.observation_validation_min_rows, 30);
        assert!(!status.observation_validation_ready);
        assert_eq!(
            status.feedback_observation_validation.mature_observations,
            0
        );
        assert_eq!(status.feedback_observation_validation.total_observations, 0);
        assert_eq!(
            status.feedback_observation_validation.pending_observations,
            0
        );
        assert_eq!(
            status
                .feedback_observation_validation
                .outcome_distribution
                .len(),
            0
        );
        assert!(status
            .target_row_validation
            .summary_line
            .contains("target_rows raw_scored_mature=0/30 production_validation=0/30 ready=false"));
        assert!(status
            .feedback_observation_validation
            .summary_line
            .contains("observations mature=0/30 pending=0 total=0 ready=false"));
        assert!(status.summary_line.contains("calibration=not_fitted"));
        assert!(status.summary_line.contains("trainer_artifact=missing"));
        assert!(status.history_csv_path.is_none());
        assert!(status.history_jsonl_path.is_none());
        assert!(!status.trainer_manifest_ready);
        assert_eq!(status.trainer_feature_columns, 0);
        assert_eq!(status.trainer_calibration_columns, 0);
        assert_eq!(status.trainer_guardrail_columns, 0);
        assert!(!status.trainer_artifact_ready);
        assert!(status
            .trainer_artifact_path
            .ends_with(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE));
        assert_eq!(status.trainer_artifact_trained_rows, 0);
        assert!(!status.trainer_artifact_uri_present);
        assert!(status
            .warnings
            .contains(&"structural_path_ranking_target_calibration_not_fitted".to_string()));
        assert!(status
            .warnings
            .contains(&"structural_path_ranking_target_trainer_manifest_incomplete".to_string()));
        assert!(status
            .warnings
            .contains(&"structural_path_ranking_target_trainer_artifact_missing".to_string()));
        assert!(status
            .warnings
            .contains(&"structural_path_ranking_target_update_runs_missing".to_string()));
        assert!(status
            .warnings
            .contains(&"structural_path_ranking_target_structural_feedback_missing".to_string()));
        assert!(status.warnings.contains(
            &"structural_path_ranking_target_pending_update_templates_present".to_string()
        ));
        assert!(status
            .warnings
            .iter()
            .any(|warning| warning.contains(
                "structural_path_ranking_target_feedback_rows_missing_structural_refs:dominant_source=legacy_feedback count=1"
            )));
    }

    fn structural_feedback_record_for_status(
        outcome: &str,
        pnl: f64,
        followed_path: bool,
    ) -> FeedbackRecord {
        FeedbackRecord {
            timestamp: chrono::Utc::now(),
            symbol: "NQ".to_string(),
            source: "structural_feedback_replay".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: Vec::new(),
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: crate::types::Direction::Neutral,
                selected_probability: 0.0,
                long_score: 0.0,
                short_score: 0.0,
                win_prob_long: 0.0,
                win_prob_short: 0.0,
                uncertainty: 0.0,
            },
            realized_outcome: outcome.to_string(),
            pnl,
            regime_at_entry: crate::types::Regime::ManipulationExpansion,
            structural_feedback: Some(StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-test".to_string(),
                recommended_at: chrono::Utc::now().to_rfc3339(),
                node_id: "node-test".to_string(),
                branch_id: "branch-test".to_string(),
                scenario_id: "scenario-test".to_string(),
                path_id: "path-test".to_string(),
                followed_path,
                exit_reason: None,
                notes: None,
            }),
            reflection_mismatch_tags: Vec::new(),
        }
    }

    #[test]
    fn structural_path_ranking_status_splits_target_rows_from_feedback_observations() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let history_jsonl_path = summary_dir.join("structural_path_ranking_target_history.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            history_rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            history_mature_rows: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 2,
            rows_with_path_prob_lower_bound: 2,
            rows_with_training_weight: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            history_jsonl_path: history_jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2 history_rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.8,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.2,
                "matured_failure",
            ))
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(&history_jsonl_path, format!("{jsonl}\n")).unwrap();
        let mut feedback_history = Vec::new();
        for index in 0..STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS {
            let (outcome, pnl) = match index % 3 {
                0 => ("win", 1.0),
                1 => ("loss", -1.0),
                _ => ("breakeven", 0.0),
            };
            feedback_history.push(structural_feedback_record_for_status(outcome, pnl, true));
        }
        feedback_history.push(structural_feedback_record_for_status("pending", 0.0, true));
        save_learning_state(
            temp.path(),
            "NQ",
            &LearningState {
                feedback_history,
                ..LearningState::default()
            },
        )
        .unwrap();

        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        let full_status =
            policy_training_status(temp.path().to_str().unwrap(), "NQ", None).unwrap();

        assert_eq!(status.raw_scored_mature_rows, 2);
        assert_eq!(status.production_validation_rows, 2);
        assert!(!status.production_validation_ready);
        assert_eq!(status.observation_validation_rows, 30);
        assert!(status.observation_validation_ready);
        assert_eq!(
            status.feedback_observation_validation.mature_observations,
            30
        );
        assert_eq!(
            status.feedback_observation_validation.pending_observations,
            1
        );
        assert_eq!(
            status.feedback_observation_validation.total_observations,
            31
        );
        assert_eq!(
            status
                .feedback_observation_validation
                .outcome_distribution
                .get("win"),
            Some(&10)
        );
        assert_eq!(
            status
                .feedback_observation_validation
                .outcome_distribution
                .get("loss"),
            Some(&10)
        );
        assert_eq!(
            status
                .feedback_observation_validation
                .outcome_distribution
                .get("breakeven"),
            Some(&10)
        );
        assert!(status.summary_line.contains("raw_scored_mature=2/30"));
        assert!(status.summary_line.contains("production_validation=2/30"));
        assert!(status.summary_line.contains("observation_validation=30/30"));
        assert!(full_status
            .structural_path_ranking_validation_summary
            .contains("observation_validation=30/30"));
        assert!(
            full_status
                .structural_path_ranking_validation
                .feedback_observation_validation
                .ready
        );
    }

    #[test]
    fn structural_path_ranking_target_training_status_reports_calibration_quality() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let history_csv_path = summary_dir.join("structural_path_ranking_target_history.csv");
        let history_jsonl_path = summary_dir.join("structural_path_ranking_target_history.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 2,
            rows_with_path_prob_lower_bound: 2,
            rows_with_execution_gate_status: 2,
            rows_with_training_weight: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            history_csv_path: history_csv_path.to_string_lossy().to_string(),
            history_jsonl_path: history_jsonl_path.to_string_lossy().to_string(),
            history_rows: 2,
            history_mature_rows: 2,
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE),
            serde_json::to_string_pretty(&structural_path_ranking_trainer_artifact_for_test())
                .unwrap(),
        )
        .unwrap();
        let jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.8,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.2,
                "matured_failure",
            ))
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(&history_csv_path, "header\n").unwrap();
        std::fs::write(&history_jsonl_path, format!("{jsonl}\n")).unwrap();

        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();

        assert!(status.export_ready);
        assert!(status.calibration_ready);
        assert!(status.calibration_quality_ready);
        assert!(!status.production_validation_ready);
        assert_eq!(status.production_validation_rows, 2);
        assert_eq!(status.history_rows, 2);
        assert_eq!(status.history_mature_rows, 2);
        assert_eq!(status.history_rows_with_raw_path_score, 2);
        assert_eq!(status.history_rows_with_calibrated_path_prob, 2);
        assert_eq!(status.history_rows_with_path_prob_lower_bound, 2);
        assert_eq!(status.history_rows_with_propensity_estimate, 2);
        assert_eq!(status.history_rows_with_training_weight, 2);
        assert_eq!(status.update_runs_with_structural_feedback, 0);
        assert_eq!(status.feedback_rows_with_structural_feedback, 0);
        assert_eq!(status.feedback_rows_total, 0);
        assert_eq!(status.feedback_rows_without_structural_feedback, 0);
        assert!(!status.pending_update_artifact_present);
        assert_eq!(status.pending_update_history_rows, 0);
        assert_eq!(
            status.production_validation_min_rows,
            STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS
        );
        assert_eq!(status.mature_rows, 2);
        assert_eq!(status.rows_with_execution_gate_status, 2);
        assert_eq!(status.rows_with_training_weight, 2);
        assert!(status.trainer_manifest_ready);
        assert_eq!(
            status.trainer_manifest_protocol_version.as_deref(),
            Some("structural-path-ranking-trainer-manifest-v1")
        );
        assert_eq!(
            status.trainer_manifest_dataset_role.as_deref(),
            Some("external_path_ranker_training_dataset")
        );
        assert_eq!(status.trainer_feature_columns, 2);
        assert_eq!(status.trainer_calibration_columns, 1);
        assert_eq!(status.trainer_guardrail_columns, 1);
        assert!(status.trainer_artifact_ready);
        assert_eq!(
            status.trainer_artifact_protocol_version.as_deref(),
            Some(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION)
        );
        assert_eq!(
            status.trainer_artifact_dataset_role.as_deref(),
            Some("external_path_ranker_training_dataset")
        );
        assert_eq!(
            status.trainer_artifact_model_family.as_deref(),
            Some("catboost")
        );
        assert_eq!(
            status.trainer_artifact_score_column.as_deref(),
            Some("raw_path_score")
        );
        assert_eq!(status.trainer_artifact_trained_rows, 42);
        assert_eq!(status.trainer_artifact_calibration_rows, 12);
        assert_eq!(status.trainer_artifact_feature_columns, 2);
        assert!(status.trainer_artifact_uri_present);
        assert_eq!(status.calibration_evaluation_rows, 2);
        assert_eq!(status.calibration_propensity_weighted_rows, 2);
        assert_eq!(status.raw_scored_mature_rows, 2);
        let expected_history_csv_path = jsonl_path
            .with_file_name("structural_path_ranking_target_history.csv")
            .to_string_lossy()
            .to_string();
        let expected_history_jsonl_path = jsonl_path
            .with_file_name("structural_path_ranking_target_history.jsonl")
            .to_string_lossy()
            .to_string();
        assert_eq!(
            status.history_csv_path.as_deref(),
            Some(expected_history_csv_path.as_str())
        );
        assert_eq!(
            status.history_jsonl_path.as_deref(),
            Some(expected_history_jsonl_path.as_str())
        );
        assert_eq!(
            status.raw_scored_mature_min_rows,
            STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS
        );
        assert_eq!(
            status.raw_scored_mature_shortfall_rows,
            STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS - 2
        );
        assert_eq!(
            status.production_validation_shortfall_rows,
            STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS - 2
        );
        assert!((status.calibration_brier_score.unwrap() - 0.04).abs() < 1e-9);
        assert!((status.calibration_propensity_weighted_brier_score.unwrap() - 0.04).abs() < 1e-9);
        assert!((status.calibration_expected_error.unwrap() - 0.0).abs() < 1e-9);
        assert!(status.warnings.iter().any(|warning| warning
            .starts_with("structural_path_ranking_target_raw_scored_mature_rows_insufficient")));
        assert!(status.warnings.iter().any(|warning| warning.starts_with(
            "structural_path_ranking_target_production_validation_insufficient_rows"
        )));
        assert!(status.summary_line.contains("raw_scored_mature=2/30"));
        assert!(status.summary_line.contains("history_rows=2"));
        assert!(status.summary_line.contains("production_validation=2/30"));
        assert!(status.summary_line.contains("observation_validation=0/30"));
        assert_eq!(status.target_row_validation.raw_scored_mature_rows, 2);
        assert_eq!(status.target_row_validation.production_validation_rows, 2);
        assert!(!status.target_row_validation.production_validation_ready);
        assert_eq!(
            status.feedback_observation_validation.mature_observations,
            0
        );
        assert_eq!(status.feedback_observation_validation.total_observations, 0);
        assert!(status.summary_line.contains("calibration=evaluated"));
        assert!(status.summary_line.contains("trainer_artifact=ready"));
        assert!(!status
            .warnings
            .contains(&"structural_path_ranking_target_trainer_manifest_incomplete".to_string()));
        assert!(!status
            .warnings
            .contains(&"structural_path_ranking_target_trainer_artifact_missing".to_string()));
        assert!(!status
            .warnings
            .contains(&"structural_path_ranking_target_trainer_artifact_incomplete".to_string()));
    }

    #[test]
    fn structural_path_ranking_target_training_status_reports_production_validation_ready() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let history_csv_path = summary_dir.join("structural_path_ranking_target_history.csv");
        let history_jsonl_path = summary_dir.join("structural_path_ranking_target_history.jsonl");
        let row_count = STRUCTURAL_PATH_RANKING_PRODUCTION_VALIDATION_MIN_ROWS;
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: row_count,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 3,
            mature_rows: row_count,
            rows_with_propensity_estimate: row_count,
            rows_with_calibrated_path_prob: row_count,
            rows_with_path_prob_lower_bound: row_count,
            rows_with_execution_gate_status: row_count,
            rows_with_training_weight: row_count,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            history_csv_path: history_csv_path.to_string_lossy().to_string(),
            history_jsonl_path: history_jsonl_path.to_string_lossy().to_string(),
            history_rows: row_count,
            history_mature_rows: row_count,
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: format!("structural_path_ranking_target rows={row_count}"),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let jsonl = (0..row_count)
            .map(|index| {
                let state = if index % 2 == 0 {
                    "matured_success"
                } else {
                    "matured_failure"
                };
                let probability = if state == "matured_success" { 0.8 } else { 0.2 };
                serde_json::to_string(&structural_path_ranking_row(
                    &format!("path-{index}"),
                    probability,
                    state,
                ))
                .unwrap()
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(&history_csv_path, "header\n").unwrap();
        std::fs::write(&history_jsonl_path, format!("{jsonl}\n")).unwrap();

        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();

        assert!(status.calibration_quality_ready);
        assert!(status.trainer_manifest_ready);
        assert!(status.production_validation_ready);
        assert_eq!(status.history_rows, row_count);
        assert_eq!(status.history_mature_rows, row_count);
        assert_eq!(status.history_rows_with_raw_path_score, row_count);
        assert_eq!(status.history_rows_with_calibrated_path_prob, row_count);
        assert_eq!(status.history_rows_with_path_prob_lower_bound, row_count);
        assert_eq!(status.history_rows_with_propensity_estimate, row_count);
        assert_eq!(status.history_rows_with_training_weight, row_count);
        assert_eq!(status.update_runs_with_structural_feedback, 0);
        assert_eq!(status.feedback_rows_with_structural_feedback, 0);
        assert_eq!(status.feedback_rows_total, 0);
        assert_eq!(status.feedback_rows_without_structural_feedback, 0);
        assert!(!status.pending_update_artifact_present);
        assert_eq!(status.pending_update_history_rows, 0);
        assert_eq!(status.raw_scored_mature_rows, row_count);
        assert_eq!(status.raw_scored_mature_shortfall_rows, 0);
        assert_eq!(status.production_validation_rows, row_count);
        assert_eq!(status.production_validation_shortfall_rows, 0);
        assert!(status
            .summary_line
            .contains(&format!("history_rows={row_count}")));
        assert!(status
            .summary_line
            .contains(&format!("raw_scored_mature={row_count}/{row_count}")));
        assert!(status
            .summary_line
            .contains(&format!("production_validation={row_count}/{row_count}")));
        assert!(!status.warnings.iter().any(|warning| warning
            .starts_with("structural_path_ranking_target_raw_scored_mature_rows_insufficient")));
        assert!(!status.warnings.iter().any(|warning| warning.starts_with(
            "structural_path_ranking_target_production_validation_insufficient_rows"
        )));
        assert!(!status
            .warnings
            .contains(&"structural_path_ranking_target_trainer_manifest_incomplete".to_string()));
        let full_status =
            policy_training_status(temp.path().to_str().unwrap(), "NQ", None).unwrap();
        assert!(
            full_status
                .structural_path_ranking_validation
                .calibration_ready
        );
        assert!(
            full_status
                .structural_path_ranking_validation
                .calibration_quality_ready
        );
        assert_eq!(
            full_status
                .structural_path_ranking_validation
                .calibration_status,
            "evaluated"
        );
        assert!(
            full_status
                .structural_path_ranking_validation
                .production_validation_ready
        );
        assert_eq!(
            full_status
                .structural_path_ranking_validation
                .production_validation_rows,
            row_count
        );
        assert!(full_status
            .structural_path_ranking_validation_summary
            .contains("production_validation"));
    }

    #[test]
    fn register_structural_path_ranking_trainer_artifact_writes_ready_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 3,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 3,
            mature_rows: 2,
            rows_with_training_weight: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 1,
            rows_with_path_prob_lower_bound: 1,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=3".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.8,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.2,
                "matured_failure",
            ))
            .unwrap(),
            serde_json::to_string(&StructuralPathRankingTargetRow {
                raw_path_score: None,
                calibrated_path_prob: None,
                path_prob_lower_bound: None,
                training_weight: None,
                calibrated_label: None,
                pending_reward_state: "unobserved".to_string(),
                maturity_mask: false,
                maturity_weight: 0.0,
                ..structural_path_ranking_row("path-pending", 0.4, "unobserved")
            })
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();

        let (artifact_path, artifact) = register_structural_path_ranking_trainer_artifact(
            temp.path().to_str().unwrap(),
            "NQ",
            "s3://rankers/nq-path-ranker-v1.bin",
            "catboost",
            None,
            None,
            None,
        )
        .unwrap();

        assert!(artifact_path.ends_with(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE));
        assert_eq!(
            artifact.dataset_role,
            "external_path_ranker_training_dataset"
        );
        assert_eq!(artifact.model_family, "catboost");
        assert_eq!(artifact.score_column, "raw_path_score");
        assert_eq!(artifact.trained_rows, 2);
        assert_eq!(artifact.calibration_rows, 2);
        assert_eq!(artifact.selected_features.len(), 2);

        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        assert!(status.trainer_artifact_ready);
        assert_eq!(
            status.trainer_artifact_model_family.as_deref(),
            Some("catboost")
        );
        assert_eq!(status.trainer_artifact_trained_rows, 2);
        assert_eq!(status.trainer_artifact_calibration_rows, 2);
        assert_eq!(
            status.trainer_artifact_status,
            "present_validation_insufficient"
        );
        assert!(status.trainer_artifact_uri_present);
        assert!(status.summary_line.contains("trainer_artifact=ready"));
    }

    #[test]
    fn register_structural_path_ranking_trainer_artifact_requires_rule_or_tree_for_explicit_family()
    {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            rows_with_training_weight: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 2,
            rows_with_path_prob_lower_bound: 2,
            history_rows: 2,
            history_mature_rows: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        std::fs::write(
            &jsonl_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&structural_path_ranking_row(
                    "path-win",
                    0.8,
                    "matured_success",
                ))
                .unwrap(),
                serde_json::to_string(&structural_path_ranking_row(
                    "path-loss",
                    0.2,
                    "matured_failure",
                ))
                .unwrap()
            ),
        )
        .unwrap();
        let explicit_artifact_path = temp.path().join("corels-artifact.json");
        std::fs::write(
            &explicit_artifact_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "protocol_version": STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION,
                "dataset_role": "external_path_ranker_training_dataset",
                "model_family": crate::belief_core::ranking_label::STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_CORELS,
                "selected_features": ["rank", "experience_prior"],
                "trained_rows": 2,
                "history_rows": 2,
                "validation_metrics": {
                    "raw_scored_mature_rows": 2,
                    "raw_scored_mature_min_rows": 30,
                    "production_validation_rows": 2,
                    "production_validation_min_rows": 30
                },
                "calibration_metrics": {
                    "eligible_rows": 2
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let err = register_structural_path_ranking_trainer_artifact(
            temp.path().to_str().unwrap(),
            "NQ",
            explicit_artifact_path.to_str().unwrap(),
            crate::belief_core::ranking_label::STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_CORELS,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("requires a readable JSON artifact with either rule_list or tree_json"));
    }

    #[test]
    fn register_structural_path_ranking_trainer_artifact_accepts_catboost_companion_scores() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            rows_with_training_weight: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 2,
            rows_with_path_prob_lower_bound: 2,
            history_rows: 2,
            history_mature_rows: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        std::fs::write(
            &jsonl_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&structural_path_ranking_row(
                    "path-win",
                    0.8,
                    "matured_success",
                ))
                .unwrap(),
                serde_json::to_string(&structural_path_ranking_row(
                    "path-loss",
                    0.2,
                    "matured_failure",
                ))
                .unwrap()
            ),
        )
        .unwrap();
        std::fs::write(
            summary_dir.join("catboost_scores.csv"),
            "candidate_set_id,path_id,raw_path_score\n\
             structural-candidates:NQ:test,path-win,0.91\n\
             structural-candidates:NQ:test,path-loss,0.19\n",
        )
        .unwrap();
        let model_path = temp.path().join("catboost_model.cbm");
        std::fs::write(&model_path, "binary model placeholder").unwrap();
        let companion_path = temp.path().join("catboost_trainer_artifact.json");
        std::fs::write(
            &companion_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "protocol_version": STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION,
                "dataset_role": "external_path_ranker_training_dataset",
                "model_family": "catboost",
                "artifact_uri": summary_dir.join("catboost_scores.csv").to_string_lossy(),
                "model_artifact_uri": model_path.to_string_lossy(),
                "score_column": "raw_path_score",
                "trained_rows": 11,
                "history_rows": 11,
                "calibration_rows": 2,
                "selected_features": ["rank", "experience_prior"],
                "validation_metrics": {
                    "raw_scored_mature_rows": 2,
                    "raw_scored_mature_min_rows": 30,
                    "production_validation_rows": 2,
                    "production_validation_min_rows": 30
                },
                "calibration_metrics": {
                    "eligible_rows": 2
                },
                "notes": ["catboost_runtime_scores_uri=required"]
            }))
            .unwrap(),
        )
        .unwrap();

        let (_, artifact) = register_structural_path_ranking_trainer_artifact(
            temp.path().to_str().unwrap(),
            "NQ",
            companion_path.to_str().unwrap(),
            "catboost",
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(artifact.model_family, "catboost");
        assert!(artifact.artifact_uri.ends_with("catboost_scores.csv"));
        assert!(artifact
            .notes
            .iter()
            .any(|note| note == "catboost_runtime_scores_uri=required"));

        enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();
        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        assert_eq!(
            status.runtime_selection_status,
            "enabled_registered_artifact_ready"
        );
        assert_eq!(
            status.runtime_source_kind.as_deref(),
            Some("registered_artifact")
        );
        assert_eq!(status.runtime_artifact_match_count, 2);
        assert_eq!(
            status.trainer_artifact_model_family.as_deref(),
            Some("catboost")
        );
    }

    #[test]
    fn register_structural_path_ranking_trainer_artifact_prefers_history_counts() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let history_jsonl_path = summary_dir.join("structural_path_ranking_target_history.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            history_rows: 11,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            history_mature_rows: 7,
            rows_with_training_weight: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 2,
            rows_with_path_prob_lower_bound: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            history_jsonl_path: history_jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2 history_rows=11".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let latest_jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.8,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.2,
                "matured_failure",
            ))
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{latest_jsonl}\n")).unwrap();
        std::fs::write(&history_jsonl_path, format!("{latest_jsonl}\n")).unwrap();

        let (_, artifact) = register_structural_path_ranking_trainer_artifact(
            temp.path().to_str().unwrap(),
            "NQ",
            "s3://rankers/nq-path-ranker-v2.bin",
            "catboost",
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(artifact.trained_rows, 11);
        assert_eq!(artifact.calibration_rows, 7);
    }

    #[test]
    fn clear_structural_path_ranking_trainer_artifact_removes_registered_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            rows_with_training_weight: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 2,
            rows_with_path_prob_lower_bound: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.8,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.2,
                "matured_failure",
            ))
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();
        register_structural_path_ranking_trainer_artifact(
            temp.path().to_str().unwrap(),
            "NQ",
            "s3://rankers/nq-path-ranker-v1.bin",
            "catboost",
            None,
            None,
            None,
        )
        .unwrap();

        assert!(clear_structural_path_ranking_trainer_artifact(
            temp.path().to_str().unwrap(),
            "NQ"
        )
        .unwrap());
        assert!(!clear_structural_path_ranking_trainer_artifact(
            temp.path().to_str().unwrap(),
            "NQ"
        )
        .unwrap());

        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        assert!(!status.trainer_artifact_ready);
        assert_eq!(status.trainer_artifact_trained_rows, 0);
        assert!(!status.trainer_artifact_uri_present);
        assert!(status.summary_line.contains("trainer_artifact=missing"));
        assert!(status
            .warnings
            .contains(&"structural_path_ranking_target_trainer_artifact_missing".to_string()));
    }

    #[test]
    fn enable_and_disable_structural_path_ranking_runtime_updates_status_surface() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let history_jsonl_path = summary_dir.join("structural_path_ranking_target_history.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 2,
            rows_with_path_prob_lower_bound: 2,
            rows_with_execution_gate_status: 2,
            rows_with_training_weight: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            history_jsonl_path: history_jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.8,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.2,
                "matured_failure",
            ))
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(&history_jsonl_path, format!("{jsonl}\n")).unwrap();

        enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY,
        )
        .unwrap();
        let enabled =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        assert!(enabled.runtime_selection_enabled);
        assert!(enabled.runtime_selection_ready);
        assert_eq!(
            enabled.runtime_selection_mode.as_deref(),
            Some(STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY)
        );
        assert_eq!(
            enabled.runtime_selection_status,
            "enabled_candidate_set_ready"
        );
        assert_eq!(
            enabled.runtime_source_kind.as_deref(),
            Some("candidate_set")
        );
        assert_eq!(enabled.runtime_active_match_count, 2);
        assert_eq!(enabled.runtime_candidate_set_match_count, 2);
        assert!(enabled.summary_line.contains("runtime_mode=prefer_history"));
        assert!(enabled
            .summary_line
            .contains("runtime_source=candidate_set"));
        assert!(enabled.summary_line.contains("runtime_matches=2"));
        assert!(enabled
            .summary_line
            .contains("runtime_selection=enabled_candidate_set_ready"));

        disable_structural_path_ranking_runtime_command(temp.path().to_str().unwrap(), "NQ")
            .unwrap();
        let disabled =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        assert!(!disabled.runtime_selection_enabled);
        assert!(!disabled.runtime_selection_ready);
        assert_eq!(disabled.runtime_selection_status, "disabled");
        assert!(disabled.runtime_source_kind.is_none());
        assert_eq!(disabled.runtime_active_match_count, 0);
    }

    #[test]
    fn runtime_status_prefers_registered_artifact_scores_when_available() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let history_jsonl_path = summary_dir.join("structural_path_ranking_target_history.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 2,
            rows_with_path_prob_lower_bound: 2,
            rows_with_execution_gate_status: 2,
            rows_with_training_weight: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            history_jsonl_path: history_jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.8,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.2,
                "matured_failure",
            ))
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(&history_jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(
            summary_dir.join("artifact_scores.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::json!({
                    "candidate_set_id": "structural-candidates:NQ:test",
                    "path_id": "path-win",
                    "raw_path_score": 0.95,
                    "calibrated_path_prob": 0.81,
                    "path_prob_lower_bound": 0.71,
                    "execution_gate_status": "pass"
                }),
                serde_json::json!({
                    "candidate_set_id": "structural-candidates:NQ:test",
                    "path_id": "path-loss",
                    "raw_path_score": 0.11,
                    "calibrated_path_prob": 0.19,
                    "path_prob_lower_bound": 0.09,
                    "execution_gate_status": "observe"
                })
            ),
        )
        .unwrap();
        let artifact = StructuralPathRankingTrainerArtifact {
            protocol_version: STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION.to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: "catboost".to_string(),
            artifact_uri: "artifact_scores.jsonl".to_string(),
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "raw_path_score".to_string()],
            validation_metrics: StructuralPathRankerValidationMetrics::default(),
            calibration_metrics: StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: None,
            notes: vec![],
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();

        enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();
        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        assert!(status.runtime_selection_enabled);
        assert!(status.runtime_selection_ready);
        assert_eq!(
            status.runtime_selection_status,
            "enabled_registered_artifact_ready"
        );
        assert_eq!(
            status.runtime_source_kind.as_deref(),
            Some("registered_artifact")
        );
        assert_eq!(status.runtime_active_match_count, 2);
        assert_eq!(status.runtime_artifact_match_count, 2);
        let full_status =
            policy_training_status(temp.path().to_str().unwrap(), "NQ", None).unwrap();
        assert!(full_status.structural_path_ranking_runtime.enabled);
        assert!(full_status.structural_path_ranking_runtime.ready);
        assert_eq!(
            full_status
                .structural_path_ranking_runtime
                .source_kind
                .as_deref(),
            Some("registered_artifact")
        );
        assert_eq!(
            full_status
                .structural_path_ranking_runtime
                .active_match_count,
            2
        );
        assert!(full_status
            .structural_path_ranking_runtime_summary
            .contains("runtime_source=registered_artifact"));
        assert!(full_status
            .structural_path_ranking_runtime_summary
            .contains("runtime_matches=2"));
        assert!(status
            .summary_line
            .contains("runtime_source=registered_artifact"));
        assert!(status.summary_line.contains("runtime_matches=2"));
    }

    #[test]
    fn runtime_status_reports_registered_direct_model_when_available() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let history_jsonl_path = summary_dir.join("structural_path_ranking_target_history.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 0,
            rows_with_path_prob_lower_bound: 0,
            rows_with_execution_gate_status: 0,
            rows_with_training_weight: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            history_jsonl_path: history_jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.0,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.0,
                "matured_failure",
            ))
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(&history_jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(
            summary_dir.join("path_ranker_direct_model.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "protocol_version": "structural-path-ranking-direct-model-v1",
                "model_family": crate::belief_core::ranking_label::STRUCTURAL_PATH_RANKER_DIRECT_MODEL_FAMILY_WEIGHTED_SUM_V1,
                "feature_schema_version": "structural-path-ranking-trainer-manifest-v1",
                "output_transform": "sigmoid",
                "intercept": 2.0,
                "numerical_feature_weights": {
                    "rank": -1.0,
                    "experience_prior": 0.25
                },
                "lower_bound_margin": 0.05,
                "execution_gate_min_path_prob": 0.5
            }))
            .unwrap(),
        )
        .unwrap();
        let artifact = StructuralPathRankingTrainerArtifact {
            protocol_version: STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION.to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: crate::belief_core::ranking_label::STRUCTURAL_PATH_RANKER_DIRECT_MODEL_FAMILY_WEIGHTED_SUM_V1.to_string(),
            artifact_uri: "path_ranker_direct_model.json".to_string(),
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "experience_prior".to_string()],
            validation_metrics: StructuralPathRankerValidationMetrics::default(),
            calibration_metrics: StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: None,
            notes: vec![],
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();

        enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();
        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        assert!(status.runtime_selection_enabled);
        assert!(status.runtime_selection_ready);
        assert_eq!(
            status.runtime_selection_status,
            "enabled_registered_model_ready"
        );
        assert_eq!(
            status.runtime_source_kind.as_deref(),
            Some("registered_model_artifact")
        );
        assert_eq!(status.runtime_active_match_count, 2);
        assert_eq!(status.runtime_artifact_match_count, 2);
        assert!(status
            .summary_line
            .contains("runtime_source=registered_model_artifact"));
        assert!(status.summary_line.contains("runtime_matches=2"));
    }

    #[test]
    fn runtime_status_reports_registered_service_when_available() {
        let temp = tempfile::tempdir().unwrap();
        let summary_dir = temp.path().join("NQ").join(POLICY_TRAINING_DIR);
        std::fs::create_dir_all(&summary_dir).unwrap();
        let jsonl_path = summary_dir.join("structural_path_ranking_target.jsonl");
        let history_jsonl_path = summary_dir.join("structural_path_ranking_target_history.jsonl");
        let summary = StructuralPathRankingTargetExportSummary {
            symbol: "NQ".to_string(),
            rows: 2,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 2,
            mature_rows: 2,
            rows_with_propensity_estimate: 2,
            rows_with_calibrated_path_prob: 0,
            rows_with_path_prob_lower_bound: 0,
            rows_with_execution_gate_status: 0,
            rows_with_training_weight: 2,
            csv_path: summary_dir
                .join("structural_path_ranking_target.csv")
                .to_string_lossy()
                .to_string(),
            jsonl_path: jsonl_path.to_string_lossy().to_string(),
            history_jsonl_path: history_jsonl_path.to_string_lossy().to_string(),
            summary_path: summary_dir
                .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE)
                .to_string_lossy()
                .to_string(),
            trainer_manifest: structural_path_ranking_trainer_manifest_for_test(),
            summary_line: "structural_path_ranking_target rows=2".to_string(),
            ..StructuralPathRankingTargetExportSummary::default()
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE),
            serde_json::to_string_pretty(&summary).unwrap(),
        )
        .unwrap();
        let jsonl = [
            serde_json::to_string(&structural_path_ranking_row(
                "path-win",
                0.0,
                "matured_success",
            ))
            .unwrap(),
            serde_json::to_string(&structural_path_ranking_row(
                "path-loss",
                0.0,
                "matured_failure",
            ))
            .unwrap(),
        ]
        .join("\n");
        std::fs::write(&jsonl_path, format!("{jsonl}\n")).unwrap();
        std::fs::write(&history_jsonl_path, format!("{jsonl}\n")).unwrap();
        let service_uri = serve_http_response_with_method(
            "rank-paths",
            serde_json::json!({
                "rows": [
                    {
                        "candidate_set_id": "structural-candidates:NQ:test",
                        "path_id": "path-win",
                        "raw_path_score": 0.91,
                        "calibrated_path_prob": 0.82,
                        "path_prob_lower_bound": 0.72,
                        "execution_gate_status": "pass"
                    },
                    {
                        "candidate_set_id": "structural-candidates:NQ:test",
                        "path_id": "path-loss",
                        "raw_path_score": 0.19,
                        "calibrated_path_prob": 0.21,
                        "path_prob_lower_bound": 0.11,
                        "execution_gate_status": "observe"
                    }
                ]
            })
            .to_string(),
            8,
            "POST",
        );
        let artifact = StructuralPathRankingTrainerArtifact {
            protocol_version: STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_PROTOCOL_VERSION.to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: crate::belief_core::ranking_label::STRUCTURAL_PATH_RANKER_SERVICE_FAMILY_ROW_SCORING_V1.to_string(),
            artifact_uri: service_uri,
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "experience_prior".to_string()],
            validation_metrics: StructuralPathRankerValidationMetrics::default(),
            calibration_metrics: StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: None,
            notes: vec![],
        };
        std::fs::write(
            summary_dir.join(STRUCTURAL_PATH_RANKING_TRAINER_ARTIFACT_FILE),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();

        enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();
        let status =
            structural_path_ranking_target_training_status(temp.path().to_str().unwrap(), "NQ")
                .unwrap();
        assert!(status.runtime_selection_enabled);
        assert!(status.runtime_selection_ready);
        assert_eq!(
            status.runtime_selection_status,
            "enabled_registered_service_ready"
        );
        assert_eq!(
            status.runtime_source_kind.as_deref(),
            Some("registered_service")
        );
        assert_eq!(status.runtime_active_match_count, 2);
        assert_eq!(status.runtime_artifact_match_count, 2);
        assert!(status
            .summary_line
            .contains("runtime_source=registered_service"));
        assert!(status.summary_line.contains("runtime_matches=2"));
    }

    #[test]
    fn export_structural_path_ranking_target_from_state_dir_uses_persisted_snapshot_and_learning_state(
    ) {
        let temp = tempfile::tempdir().unwrap();
        let snapshot =
            crate::application::orchestration::workflow_status::sample_human_workflow_snapshot();
        crate::state::save_workflow_snapshot(temp.path(), "NQ", &snapshot).unwrap();
        crate::state::save_learning_state(
            temp.path(),
            "NQ",
            &crate::state::LearningState::default(),
        )
        .unwrap();

        let summary = export_structural_path_ranking_target_from_state_dir(
            temp.path().to_str().unwrap(),
            "NQ",
        )
        .unwrap();

        assert_eq!(summary.symbol, "NQ");
        assert!(summary.history_rows >= summary.rows);
        assert!(summary
            .history_csv_path
            .ends_with("structural_path_ranking_target_history.csv"));
        assert!(summary
            .history_jsonl_path
            .ends_with("structural_path_ranking_target_history.jsonl"));
        assert!(std::path::Path::new(&summary.csv_path).exists());
        assert!(std::path::Path::new(&summary.jsonl_path).exists());
        assert!(std::path::Path::new(&summary.history_csv_path).exists());
        assert!(std::path::Path::new(&summary.history_jsonl_path).exists());
        assert!(std::path::Path::new(&summary.summary_path).exists());

        let second_summary = export_structural_path_ranking_target_from_state_dir(
            temp.path().to_str().unwrap(),
            "NQ",
        )
        .unwrap();
        assert_eq!(second_summary.history_rows, summary.history_rows);
    }
}
