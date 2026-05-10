use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use super::structural_playbook::{
    build_structural_experience_prior_surface_artifact_with_prior_state,
    build_structural_history_summary_artifact, build_structural_path_history_artifact,
    build_structural_path_ranking_target_artifact_with_runtime_context_and_prior_state,
    build_structural_playbook_bundle_with_runtime_context_and_prior_state,
    build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state,
    build_structural_temporal_summary_artifact_with_prior_state,
    build_structural_top_path_candidates_artifact_with_runtime_context_and_prior_state,
    resolved_ensemble_vote_for_snapshot, resolved_latest_ensemble_vote,
    StructuralPathRankerRuntimeContext, StructuralRecommendedPathBundleArtifact,
};
use crate::application::auto_quant::handoff::apply_provider_profile_to_command;
use crate::application::auto_quant::AutoQuantResearchHandoffPayload;
use crate::application::belief::{
    jump_calibration_gate_workflow_summary, jump_model_workflow_summary,
};
use crate::application::data_sources::control_matrix_providers::IBKR_GATEWAY_PORT_CANDIDATES;
use crate::application::output_foundation::{
    print_redacted_json, redact_local_paths_in_human_text, redact_local_paths_in_value,
    short_workflow_phase_summary,
};
use crate::application::provider_catalog::{
    build_workflow_provider_support, provider_status_agent_command_for_surface,
    provider_status_agent_surface, ProviderCatalogAgentItem, ProviderCatalogAgentSurface,
};
use crate::application::release_closure::workflow_next_step_view;
use crate::config::shell_quote;
use crate::state::{
    ArtifactConsumedImpactSummary, ArtifactDecisionSummary, ArtifactFactorTrendSummary,
    ArtifactFamilyTrendSummary, ArtifactHistorySummary, ArtifactLedgerEntry,
    ArtifactLineageSummary, ArtifactRuleBreakEffect, ArtifactRuleBreakFactorImpact,
    ArtifactRuleBreakFamilyImpact, DatasetComparability, EnsembleExecutorScorecard,
    EnsembleVoteRecord, ExecutionCandidateArtifactSummary, PendingUpdateArtifactSummary,
    PreBayesEntryQualityBridge, PreBayesEntryQualityBridgeDiff, PreBayesEvidencePolicy,
    PreBayesPolicyDiff, PreBayesPolicyLineageSummary, PreBayesPolicyRecord,
    PreBayesSoftEvidenceNodeDiff, RunProvenance, StructuralPriorLearningState, WorkflowSnapshot,
};

fn build_structural_validation_summary_value(
    experience_prior_surface: &crate::belief_core::source_reliability::StructuralExperiencePriorSurfaceArtifact,
) -> Value {
    let em = &experience_prior_surface.source_reliability_em;
    let replay = experience_prior_surface
        .path
        .as_ref()
        .and_then(|path| path.delayed_reward_replay_validation.as_ref());
    let target_policy_context_count = experience_prior_surface.target_policy_contexts.len();
    serde_json::json!({
        "source_reliability": {
            "status": em.status,
            "ready": em.ready,
            "multi_source_item_count": em.multi_source_item_count,
            "distinct_source_count": em.distinct_source_count,
            "holdout_status": em.em_holdout_status,
            "holdout_split_strategy": em.em_holdout_split_strategy,
            "holdout_brier_score": em.em_holdout_brier_score,
            "holdout_log_loss": em.em_holdout_log_loss,
            "holdout_observation_coverage": em.em_holdout_observation_coverage,
            "holdout_training_item_count": em.em_holdout_training_item_count,
            "holdout_evaluation_item_count": em.em_holdout_evaluation_item_count,
            "holdout_reason": structural_validation_reason(
                em.em_holdout_status.as_deref(),
                em.em_holdout_training_item_count,
                em.em_holdout_evaluation_item_count,
                em.em_holdout_observation_count,
                em.em_holdout_source_count,
            ),
            "replay_status": em.em_replay_status,
            "replay_split_strategy": em.em_replay_split_strategy,
            "replay_evaluation_item_count": em.em_replay_evaluation_item_count,
            "replay_observation_count": em.em_replay_observation_count,
            "replay_source_count": em.em_replay_source_count,
            "replay_brier_score": em.em_replay_brier_score,
            "replay_log_loss": em.em_replay_log_loss,
            "replay_observation_coverage": em.em_replay_observation_coverage,
            "replay_reason": structural_validation_reason(
                em.em_replay_status.as_deref(),
                0,
                em.em_replay_evaluation_item_count,
                em.em_replay_observation_count,
                em.em_replay_source_count,
            ),
            "calibration_status": em.em_calibration_status,
            "calibration_observation_count": em.em_calibration_observation_count,
            "calibration_source_count": em.em_calibration_source_count,
            "calibration_brier_score": em.em_calibration_brier_score,
            "calibration_log_loss": em.em_calibration_log_loss,
        },
        "delayed_reward": replay.map(|replay| {
            serde_json::json!({
                "status": replay.status,
                "status_reason": structural_validation_reason(
                    Some(replay.status.as_str()),
                    replay.training_record_count,
                    replay.evaluation_record_count,
                    replay.resolution_observation_count,
                    0,
                ),
                "validation_owner": "horizon_replay_validation",
                "remaining_gap": "full_event_time_competing_risk_validation_not_yet_landed",
                "training_record_count": replay.training_record_count,
                "evaluation_record_count": replay.evaluation_record_count,
                "latest_training_recommended_at": replay.latest_training_recommended_at,
                "first_evaluation_recommended_at": replay.first_evaluation_recommended_at,
                "last_evaluation_recommended_at": replay.last_evaluation_recommended_at,
                "resolution_observation_count": replay.resolution_observation_count,
                "resolution_1h_observation_count": replay.resolution_1h_observation_count,
                "resolution_4h_observation_count": replay.resolution_4h_observation_count,
                "resolution_24h_observation_count": replay.resolution_24h_observation_count,
                "resolution_brier_score": replay.resolution_brier_score,
                "resolution_1h_brier_score": replay.resolution_1h_brier_score,
                "resolution_4h_brier_score": replay.resolution_4h_brier_score,
                "resolution_24h_brier_score": replay.resolution_24h_brier_score,
            })
        }),
        "target_policy": {
            "current_model": "symbol:regime:direction_bucket_posterior",
            "status": if target_policy_context_count > 0 { "bucket_posterior_live" } else { "bucket_posterior_empty" },
            "context_count": target_policy_context_count,
            "upgrade_path": "learned_contextual_model_not_yet_landed",
        },
        "live_regime_truth_rule": {
            "status": "enforced",
            "summary": "retrospective zigzag, tiny-leg, or cluster outputs are not sufficient by themselves for live regime truth",
            "current_state_branch": "temporal_hmm_pre_bayes_nowcast",
            "pivot_confirmation_dependency": "not_required",
        },
    })
}

fn build_structural_validation_line(
    experience_prior_surface: &crate::belief_core::source_reliability::StructuralExperiencePriorSurfaceArtifact,
) -> Option<String> {
    let em = &experience_prior_surface.source_reliability_em;
    let replay = experience_prior_surface
        .path
        .as_ref()
        .and_then(|path| path.delayed_reward_replay_validation.as_ref());
    if em.em_holdout_status.is_none() && replay.is_none() {
        return None;
    }
    let holdout_status = em.em_holdout_status.as_deref().unwrap_or("unavailable");
    let holdout_split_strategy = em
        .em_holdout_split_strategy
        .as_deref()
        .unwrap_or("unavailable");
    let holdout_brier = em
        .em_holdout_brier_score
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "n/a".to_string());
    let replay_status = em.em_replay_status.as_deref().unwrap_or("unavailable");
    let replay_brier = em
        .em_replay_brier_score
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "n/a".to_string());
    let calibration_status = em.em_calibration_status.as_deref().unwrap_or("unavailable");
    let calibration_brier = em
        .em_calibration_brier_score
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "n/a".to_string());
    let holdout_reason = structural_validation_reason(
        em.em_holdout_status.as_deref(),
        em.em_holdout_training_item_count,
        em.em_holdout_evaluation_item_count,
        em.em_holdout_observation_count,
        em.em_holdout_source_count,
    );
    let replay_reason = structural_validation_reason(
        em.em_replay_status.as_deref(),
        0,
        em.em_replay_evaluation_item_count,
        em.em_replay_observation_count,
        em.em_replay_source_count,
    );
    let replay_summary = replay.map(|replay| {
        format!(
            "replay={} overall_brier={} reason={} eval={} train_until={} eval_range={}..{} obs={} obs_1h={} obs_4h={} obs_24h={}",
            replay.status,
            replay
                .resolution_brier_score
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "n/a".to_string()),
            structural_validation_reason(
                Some(replay.status.as_str()),
                replay.training_record_count,
                replay.evaluation_record_count,
                replay.resolution_observation_count,
                0,
            ),
            replay.evaluation_record_count,
            replay
                .latest_training_recommended_at
                .as_deref()
                .unwrap_or("n/a"),
            replay
                .first_evaluation_recommended_at
                .as_deref()
                .unwrap_or("n/a"),
            replay
                .last_evaluation_recommended_at
                .as_deref()
                .unwrap_or("n/a"),
            replay.resolution_observation_count,
            replay.resolution_1h_observation_count,
            replay.resolution_4h_observation_count,
            replay.resolution_24h_observation_count
        )
    });
    let target_policy_context_count = experience_prior_surface.target_policy_contexts.len();
    let mut parts = vec![format!(
        "Validation: em={} holdout={} split={} holdout_brier={} holdout_cov={} holdout_reason={} replay={} replay_brier={} replay_cov={} replay_reason={} calib={} calib_brier={} calib_obs={} multi_source_items={} target_policy=bucket_posterior contexts={} live_truth=retrospective_not_sufficient current_state_branch=temporal_hmm_pre_bayes_nowcast",
        em.status,
        holdout_status,
        holdout_split_strategy,
        holdout_brier,
        em.em_holdout_observation_coverage
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_string()),
        holdout_reason,
        replay_status,
        replay_brier,
        em.em_replay_observation_coverage
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_string()),
        replay_reason,
        calibration_status,
        calibration_brier,
        em.em_calibration_observation_count,
        em.multi_source_item_count,
        target_policy_context_count
    )];
    if let Some(replay_summary) = replay_summary {
        parts.push(replay_summary);
    }
    Some(parts.join(" | "))
}

fn build_dataset_resolution_line(
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> Option<String> {
    let Some(profile) = provider_status_agent.selected_profile.as_ref() else {
        if provider_status_agent.available_opt_in_profiles.is_empty() {
            return None;
        }
        let profile = provider_status_agent
            .available_opt_in_profiles
            .iter()
            .find(|profile| profile.opt_in_only)
            .or_else(|| provider_status_agent.available_opt_in_profiles.first())?;
        return Some(format!(
            "Dataset resolver: generic_zero_config | optional_opt_in={} | reuse_with_profile={}",
            profile.summary, profile.selector
        ));
    };
    let data_contracts = if profile.data_contract_labels.is_empty() {
        "none".to_string()
    } else {
        profile
            .data_contract_labels
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    let track_statuses = if profile.track_statuses.is_empty() {
        "none".to_string()
    } else {
        profile
            .track_statuses
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    };
    Some(format!(
        "Dataset resolver: profile_opt_in={} | contracts={} | tracks={}",
        profile.summary, data_contracts, track_statuses
    ))
}

fn structural_validation_reason(
    status: Option<&str>,
    training_items: usize,
    evaluation_items: usize,
    observations: usize,
    sources: usize,
) -> String {
    let status = status.unwrap_or("unavailable");
    if status == "ready" {
        "ready".to_string()
    } else {
        format!(
            "status={} train={} eval={} obs={} sources={}",
            status, training_items, evaluation_items, observations, sources
        )
    }
}

fn build_path_ranker_summary_value(
    recommended_path_bundle: Option<&StructuralRecommendedPathBundleArtifact>,
) -> Value {
    let Some(bundle) = recommended_path_bundle else {
        return Value::Null;
    };
    let runtime = bundle.path_ranker_runtime.as_ref();
    serde_json::json!({
        "runtime_enabled": runtime.map(|runtime| runtime.enabled),
        "status": runtime.map(|runtime| runtime.status.clone()),
        "reuse_mode": runtime.and_then(|runtime| runtime.reuse_mode.clone()),
        "runtime_source": bundle.path_ranker_runtime_source,
        "applied_path_count": runtime.map(|runtime| runtime.applied_path_count),
        "artifact_match_count": runtime.map(|runtime| runtime.artifact_match_count),
        "candidate_set_match_count": runtime.map(|runtime| runtime.candidate_set_match_count),
        "history_match_count": runtime.map(|runtime| runtime.history_match_count),
        "raw_path_score": bundle.path_ranker_raw_score,
        "calibrated_path_prob": bundle.path_ranker_calibrated_path_prob,
        "path_prob_lower_bound": bundle.path_ranker_path_prob_lower_bound,
        "execution_gate_status": bundle.path_ranker_execution_gate_status,
    })
}

fn build_path_ranker_line(
    recommended_path_bundle: Option<&StructuralRecommendedPathBundleArtifact>,
) -> Option<String> {
    let bundle = recommended_path_bundle?;
    let runtime = bundle.path_ranker_runtime.as_ref();
    let status = runtime
        .map(|runtime| runtime.status.as_str())
        .unwrap_or("baseline_only");
    let source = bundle
        .path_ranker_runtime_source
        .as_deref()
        .unwrap_or("none");
    let has_runtime_signal = runtime.is_some()
        || bundle.path_ranker_runtime_source.is_some()
        || bundle.path_ranker_raw_score.is_some()
        || bundle.path_ranker_calibrated_path_prob.is_some()
        || bundle.path_ranker_path_prob_lower_bound.is_some()
        || bundle.path_ranker_execution_gate_status.is_some();
    if !has_runtime_signal {
        return None;
    }
    let score = bundle
        .path_ranker_path_prob_lower_bound
        .map(|value| format!("lb={value:.3}"))
        .or_else(|| {
            bundle
                .path_ranker_calibrated_path_prob
                .map(|value| format!("prob={value:.3}"))
        })
        .or_else(|| {
            bundle
                .path_ranker_raw_score
                .map(|value| format!("raw={value:.3}"))
        })
        .unwrap_or_else(|| "score=n/a".to_string());
    let match_summary = runtime
        .map(|runtime| {
            format!(
                "applied={} artifact={} candidate={} history={}",
                runtime.applied_path_count,
                runtime.artifact_match_count,
                runtime.candidate_set_match_count,
                runtime.history_match_count
            )
        })
        .unwrap_or_else(|| "applied=0 artifact=0 candidate=0 history=0".to_string());
    Some(format!(
        "Ranker: status={} source={} {} {} gate={}",
        status,
        source,
        match_summary,
        score,
        bundle
            .path_ranker_execution_gate_status
            .as_deref()
            .unwrap_or("n/a")
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowEnsembleVoteSurface {
    pub artifact_id: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub provenance: RunProvenance,
    pub dataset_comparability: DatasetComparability,
    pub ensemble_version: String,
    pub final_action: String,
    pub recommended_command: String,
    pub human_next_triage: String,
    pub hard_block: super::EnsembleHardBlockArtifact,
    pub confidence: f64,
    pub consensus_strength: f64,
    pub disagreement_flags: Vec<String>,
    pub executor_summaries: Vec<String>,
    pub policy_runtime_line: Option<String>,
    pub split_explanations: Vec<String>,
    pub executor_scorecards: Vec<EnsembleExecutorScorecard>,
    pub executor_scorecard_source: String,
    pub posterior_fingerprint: String,
    pub posterior_normalization_status: String,
    pub posterior_active_regime: String,
    pub posterior_confidence: Option<f64>,
    pub posterior_probabilities: std::collections::BTreeMap<String, f64>,
    pub posterior_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowEnsembleVoteHistoryRow {
    pub artifact_id: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub final_action: String,
    pub recommended_command: String,
    pub human_next_triage: String,
    pub hard_block: super::EnsembleHardBlockArtifact,
    pub policy_runtime_line: Option<String>,
    pub executor_scorecards: Vec<EnsembleExecutorScorecard>,
    pub executor_scorecard_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowHardBlockReasonCount {
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowHardBlockSummary {
    pub count: usize,
    pub reason_leaderboard: Vec<WorkflowHardBlockReasonCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowEnsembleVoteHistoryView {
    pub history: Vec<WorkflowEnsembleVoteHistoryRow>,
    pub hard_block_only: Vec<WorkflowEnsembleVoteHistoryRow>,
    pub hard_block_summary: WorkflowHardBlockSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowAuxiliaryArtifactSurfaces {
    pub pending_update: Option<PendingUpdateArtifactSummary>,
    pub pending_update_history: Vec<PendingUpdateArtifactSummary>,
    pub execution_candidate: Option<ExecutionCandidateArtifactSummary>,
    pub execution_candidate_history: Vec<ExecutionCandidateArtifactSummary>,
    pub artifact_history_summary: ArtifactHistorySummary,
    pub artifact_factor_trends: Vec<ArtifactFactorTrendSummary>,
    pub artifact_family_trends: Vec<ArtifactFamilyTrendSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowArtifactReportSurfaces {
    pub artifact_consumed_gate: Value,
    pub artifact_factor_consumed_validation: Vec<ArtifactFactorTrendSummary>,
    pub artifact_family_consumed_validation: Vec<ArtifactFamilyTrendSummary>,
    pub artifact_lineage_summaries: Vec<ArtifactLineageSummary>,
    pub artifact_decision_summary: ArtifactDecisionSummary,
    pub artifact_rule_breaks: Vec<ArtifactLineageSummary>,
    pub artifact_rule_break_effects: Vec<ArtifactRuleBreakEffect>,
    pub artifact_factor_rule_break_impacts: Vec<ArtifactRuleBreakFactorImpact>,
    pub artifact_family_rule_break_impacts: Vec<ArtifactRuleBreakFamilyImpact>,
    pub artifact_impact_leaderboard: Value,
    pub artifact_impact_consumed: Value,
    pub artifact_impact_consumed_trend: ArtifactConsumedImpactSummary,
    pub artifact_review_rules: crate::state::ArtifactReviewRules,
    pub artifact_review_rule_sources: crate::state::ArtifactReviewRuleSources,
    pub disagreements: Vec<crate::state::WorkflowDisagreement>,
    pub diffs: Vec<crate::state::WorkflowFieldDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowPreBayesSurfaces {
    pub pre_bayes_policy: Option<PreBayesEvidencePolicy>,
    pub pre_bayes_policy_history: Vec<PreBayesPolicyRecord>,
    pub pre_bayes_policy_diff: Option<PreBayesPolicyDiff>,
    pub pre_bayes_policy_lineage: Option<PreBayesPolicyLineageSummary>,
    pub pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    pub pre_bayes_entry_quality_bridge_diff: Option<PreBayesEntryQualityBridgeDiff>,
    pub canonical_structural_active_regime: Option<String>,
    pub canonical_structural_confidence: Option<f64>,
    pub canonical_structural_probabilities: std::collections::BTreeMap<String, f64>,
    pub pre_bayes_soft_evidence:
        Option<std::collections::BTreeMap<String, std::collections::BTreeMap<String, f64>>>,
    pub pre_bayes_soft_evidence_diff: Vec<PreBayesSoftEvidenceNodeDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowPhaseSnapshotSurfaces {
    pub train: Option<crate::state::WorkflowPhaseSnapshot>,
    pub analyze: Option<crate::state::WorkflowPhaseSnapshot>,
    pub research: Option<crate::state::WorkflowPhaseSnapshot>,
    pub backtest: Option<crate::state::WorkflowPhaseSnapshot>,
    pub update: Option<crate::state::WorkflowPhaseSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapView {
    pub symbol: String,
    pub project_role: String,
    pub closed_loop_chain: Vec<String>,
    pub agent_brief: Vec<String>,
    pub guardrails: Vec<String>,
    pub detected_paths: AgentBootstrapPaths,
    pub input_acquisition: AgentBootstrapInputs,
    pub commands: AgentBootstrapCommands,
    pub latest_snapshot: AgentBootstrapSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapPaths {
    pub tomac_history_root: Option<String>,
    pub multi_timeframe_clean_root: Option<String>,
    pub state_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapCommands {
    pub clean_multi_timeframe: String,
    pub train: String,
    pub analyze: String,
    pub futures_sop: String,
    pub expansion_sop: String,
    pub workflow_status: String,
    pub provider_status: String,
    pub recommended_next_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapSnapshot {
    pub current_focus_phase: String,
    pub current_focus_reason: String,
    pub blocking_truth: crate::state::WorkflowBlockingTruth,
    pub latest_train_phase: Option<String>,
    pub latest_analyze_phase: Option<String>,
    pub latest_pre_bayes_gate_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowFirstRunRoute {
    pub route_id: String,
    pub label: String,
    pub summary: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_up_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowFirstRunGuide {
    pub active: bool,
    pub summary: String,
    pub provider_command: String,
    pub provider_summary: String,
    pub bootstrap_command: String,
    pub optional_profile_policy: String,
    pub routes: Vec<WorkflowFirstRunRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowAutoQuantHandoffGuide {
    pub active: bool,
    pub artifact_id: String,
    pub handoff_kind: String,
    pub status: String,
    pub data_ready: bool,
    pub recommended_next_command: String,
    pub review_command: String,
    pub workflow_status_command: String,
    pub suggested_next_steps: Vec<String>,
    pub handoff_artifact_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowEvidenceReviewGuide {
    pub active: bool,
    pub summary: String,
    pub ensemble_vote_command: String,
    pub pre_bayes_status_command: String,
    pub structural_path_command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_up_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapInputs {
    pub backtest: AgentBootstrapBacktestInput,
    pub live: AgentBootstrapLiveInput,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkflowStatusDispatchInput<'a> {
    pub phase: Option<&'a str>,
    pub actionable_only: bool,
    pub conflicts_only: bool,
    pub latest_promotable: bool,
    pub hard_block_only: bool,
    pub hard_block_reason: Option<&'a str>,
    pub limit: Option<usize>,
    pub output_format: &'a str,
    pub stable: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowStatusBootstrapInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub detected_tomac_root: Option<String>,
    pub multi_timeframe_clean_root: Option<String>,
    pub tomac_root_placeholder: String,
}

#[derive(Debug, Clone)]
struct AgentBootstrapBuildInput<'a> {
    symbol: &'a str,
    state_dir: &'a str,
    snapshot: &'a WorkflowSnapshot,
    provider_status_agent: &'a ProviderCatalogAgentSurface,
    detected_tomac_root: Option<String>,
    multi_timeframe_clean_root: Option<String>,
    tomac_root_placeholder: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapBacktestInput {
    pub local_discovery_order: Vec<String>,
    pub preferred_user_inputs: Vec<String>,
    pub fallback_user_inputs: Vec<String>,
    pub should_ask_download_link_if_local_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapLiveInput {
    pub minimum_required_user_inputs: Vec<String>,
    pub inferable_defaults:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    pub additional_user_inputs_if_not_inferable: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_profile_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_profile_data_contracts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_profile_track_statuses: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_resolution_line: Option<String>,
    pub provider_access_requests: Vec<String>,
    pub provider_status_agent: ProviderCatalogAgentSurface,
    pub provider_status_command: String,
    pub ibkr_gateway_summary: AgentBootstrapIbkrGatewaySummary,
    pub ibkr_gateway_candidates: Vec<AgentBootstrapIbkrGatewayCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapIbkrGatewayCandidate {
    pub label: String,
    pub host: String,
    pub port: u16,
    pub reachable: bool,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentBootstrapIbkrGatewaySummary {
    pub preferred_label: Option<String>,
    pub preferred_port: Option<u16>,
    pub reachable_candidate_count: usize,
    pub occupied_judgement: String,
    pub recommended_action: String,
}

pub fn build_phase_snapshot_surfaces(snapshot: &WorkflowSnapshot) -> WorkflowPhaseSnapshotSurfaces {
    WorkflowPhaseSnapshotSurfaces {
        train: snapshot.latest_train.clone(),
        analyze: snapshot.latest_analyze.clone(),
        research: snapshot.latest_research.clone(),
        backtest: snapshot.latest_backtest.clone(),
        update: snapshot.latest_update.clone(),
    }
}

pub fn factor_autoresearch_status_value_for_empty_state(symbol: &str, state_dir: &str) -> Value {
    let recommended_next_step = format!(
        "ask-user: Provide a historical data file path before starting factor-autoresearch for {} | blocked until user_selected_historical_data | then ict-engine factor-autoresearch --symbol {} --data <historical-data.json> --state-dir {}",
        shell_quote(symbol),
        shell_quote(symbol),
        shell_quote(state_dir)
    );
    json!({
        "symbol": symbol,
        "state_dir": state_dir,
        "status": "no_autoresearch_state",
        "live_snapshot": Value::Null,
        "sessions": [],
        "attempts": [],
        "final_summary_exists": false,
        "recommended_next_step": recommended_next_step,
    })
}

const NO_WORKFLOW_STATE: &str = "no_workflow_state";
const NO_WORKFLOW_PHASE_SUMMARY: &str = "No workflow phase summary available yet.";
const WORKFLOW_STATUS_FOCUS_PHASE: &str = "workflow_status";

fn workflow_status_empty_state(snapshot: &WorkflowSnapshot) -> bool {
    snapshot.latest_update.is_none()
        && snapshot.latest_research.is_none()
        && snapshot.latest_analyze.is_none()
        && snapshot.latest_backtest.is_none()
        && snapshot.latest_train.is_none()
}

fn profile_matches_symbol(
    profile: &crate::application::provider_catalog::ProviderProfileReferenceSurface,
    symbol: &str,
) -> bool {
    let Ok(document) =
        crate::application::provider_catalog::load_provider_profile(&profile.selector)
    else {
        return false;
    };
    document.data_contracts.iter().any(|contract| {
        contract.symbols.is_empty() || contract.symbols.iter().any(|item| item == symbol)
    })
}

fn workflow_status_focus_phase(snapshot: &WorkflowSnapshot) -> String {
    if snapshot.current_focus_phase.trim().is_empty() {
        WORKFLOW_STATUS_FOCUS_PHASE.to_string()
    } else {
        snapshot.current_focus_phase.clone()
    }
}

pub fn humanize_workflow_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.is_empty()
        || trimmed == "recommended_command_unavailable"
        || trimmed == "next_command_unavailable"
    {
        return "No actionable command available.".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("ask-user: ") {
        let mut parts = rest.split(" | blocked until user_selected_historical_data | then ");
        let ask = parts.next().unwrap_or("").trim();
        let then = parts.next().unwrap_or("").trim();
        if then.is_empty() || then == "choose historical dataset with user before running command" {
            return format!("Ask the user to choose the historical dataset. {}", ask);
        }
        return format!(
            "Ask the user to choose the historical dataset. {} Then run: {}",
            ask, then
        );
    }
    if trimmed.starts_with("blocked:") {
        return format!("Blocked: {}", trimmed.trim_start_matches("blocked:").trim());
    }
    format!("Next step: {}", trimmed)
}

fn apply_selected_profile_to_workflow_command(
    command: &str,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> String {
    let Some(profile) = provider_status_agent.selected_profile.as_ref() else {
        return command.to_string();
    };
    apply_provider_profile_to_command(command, Some(&profile.selector))
}

fn human_display_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("ict-engine workflow-status ")
        && trimmed.contains(" --phase ")
        && !trimmed.contains(" --human")
    {
        return format!("{trimmed} --human");
    }
    if trimmed.starts_with("ict-engine pre-bayes-status ") && !trimmed.contains(" --human") {
        return format!("{trimmed} --human");
    }
    if trimmed.starts_with("ict-engine analyze-live ") && !trimmed.contains(" --human") {
        return format!("{trimmed} --human");
    }
    trimmed.to_string()
}

fn historical_data_candidate_kind(path: &str) -> String {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path);
    for (suffix, label) in [
        ("_htf.json", "htf"),
        ("_mtf.json", "mtf"),
        ("_ltf.json", "ltf"),
        ("_spot.json", "spot"),
        ("_m1.json", "1m"),
        ("_m5.json", "5m"),
        ("_h4.json", "4h"),
    ] {
        if file_name.ends_with(suffix) {
            return label.to_string();
        }
    }
    file_name.to_string()
}

fn historical_data_candidate_display(path: &str) -> String {
    format!("[{}] {}", historical_data_candidate_kind(path), path)
}

fn historical_data_candidate_display_list(paths: &[String]) -> String {
    paths
        .iter()
        .map(|path| historical_data_candidate_display(path))
        .collect::<Vec<_>>()
        .join(", ")
}

fn workflow_status_base_command(
    symbol: &str,
    state_dir: Option<&str>,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> String {
    format!(
        "ict-engine workflow-status --symbol {} --state-dir {}{}",
        shell_quote(symbol),
        shell_quote(state_dir.unwrap_or("<state-dir>")),
        provider_status_agent
            .selected_profile
            .as_ref()
            .map(|profile| format!(" --profile {}", shell_quote(&profile.selector)))
            .unwrap_or_default()
    )
}

fn workflow_status_bootstrap_command(
    symbol: &str,
    state_dir: Option<&str>,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> String {
    format!(
        "{} --phase bootstrap",
        workflow_status_base_command(symbol, state_dir, provider_status_agent)
    )
}

fn workflow_status_provider_compact_command(
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> String {
    if let Some(profile) = provider_status_agent.selected_profile.as_ref() {
        return format!(
            "ict-engine provider-status --compact --profile {}",
            shell_quote(&profile.selector)
        );
    }
    "ict-engine provider-status --compact".to_string()
}

fn first_run_provider_ids(
    providers: &[ProviderCatalogAgentItem],
    predicate: impl Fn(&ProviderCatalogAgentItem) -> bool,
) -> Vec<String> {
    providers
        .iter()
        .filter(|provider| predicate(provider))
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>()
}

fn build_first_run_provider_summary(provider_status_agent: &ProviderCatalogAgentSurface) -> String {
    let tradfi = provider_status_agent
        .providers
        .iter()
        .filter(|provider| {
            provider.ready
                && provider.user_access == "free_no_login"
                && provider.market_fit.iter().any(|fit| fit == "tradfi")
        })
        .min_by_key(|provider| provider.fallback_priority.unwrap_or(u8::MAX))
        .map(|provider| provider.provider_id.clone())
        .unwrap_or_else(|| "none".to_string());
    let live_zero_config = first_run_provider_ids(&provider_status_agent.providers, |provider| {
        provider.ready && provider.user_access == "zero_config_local"
    });
    let crypto = first_run_provider_ids(&provider_status_agent.providers, |provider| {
        provider.ready
            && provider.user_access == "public_no_login"
            && provider.market_fit.iter().any(|fit| fit == "crypto")
    });
    let setup_required = first_run_provider_ids(&provider_status_agent.providers, |provider| {
        provider.selectable_by_user
            && !provider.ready
            && matches!(
                provider.user_access.as_str(),
                "login_and_local_runtime" | "api_key_required" | "operator_runtime_optional"
            )
    });
    let live_zero_config = if live_zero_config.is_empty() {
        "none".to_string()
    } else {
        live_zero_config.join(", ")
    };
    let crypto = if crypto.is_empty() {
        "none".to_string()
    } else {
        crypto.join(", ")
    };
    let setup_required = if setup_required.is_empty() {
        "none".to_string()
    } else {
        setup_required.join(", ")
    };
    format!(
        "tradfi free fallback={} | live zero-config={} | crypto public={} | setup required={}",
        tradfi, live_zero_config, crypto, setup_required
    )
}

fn build_first_run_guide(
    symbol: &str,
    state_dir: Option<&str>,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> WorkflowFirstRunGuide {
    let bootstrap_command =
        workflow_status_bootstrap_command(symbol, state_dir, provider_status_agent);
    let state_dir = shell_quote(state_dir.unwrap_or("<state-dir>"));
    let symbol = shell_quote(symbol);
    WorkflowFirstRunGuide {
        active: true,
        summary: "Choose a first path: replay a demo/historical run, iterate factors/backtests from historical data, or bootstrap the live path after provider selection.".to_string(),
        provider_command: workflow_status_provider_compact_command(provider_status_agent),
        provider_summary: build_first_run_provider_summary(provider_status_agent),
        bootstrap_command: bootstrap_command.clone(),
        optional_profile_policy:
            "opt-in local profiles stay hidden by default; only reuse them when the user explicitly passes --profile.".to_string(),
        routes: vec![
            WorkflowFirstRunRoute {
                route_id: "replay".to_string(),
                label: "Replay / Review".to_string(),
                summary:
                    "Start with a safe demo or historical review path before changing factors."
                        .to_string(),
                command: format!(
                    "ict-engine analyze --symbol {} --demo --state-dir {} --human",
                    symbol, state_dir
                ),
                follow_up_command: None,
            },
            WorkflowFirstRunRoute {
                route_id: "backtest_or_factor_loop".to_string(),
                label: "Backtest / Factors".to_string(),
                summary:
                    "Use historical data to clarify strategy and iterate factors or backtests."
                        .to_string(),
                command: apply_selected_profile_to_workflow_command(
                    &format!(
                        "ict-engine factor-research --symbol {} --data <historical-data.json> --state-dir {} --human",
                        symbol, state_dir
                    ),
                    provider_status_agent,
                ),
                follow_up_command: Some(apply_selected_profile_to_workflow_command(
                    &format!(
                        "ict-engine factor-backtest --symbol {} --data <historical-data.json> --state-dir {} --human",
                        symbol, state_dir
                    ),
                    provider_status_agent,
                )),
            },
            WorkflowFirstRunRoute {
                route_id: "live".to_string(),
                label: "Live".to_string(),
                summary:
                    "Inspect provider and symbol prerequisites first, then continue into analyze-live."
                        .to_string(),
                command: bootstrap_command,
                follow_up_command: Some(format!(
                    "ict-engine analyze-live --symbol {} --state-dir {}",
                    symbol, state_dir
                )),
            },
        ],
    }
}

fn first_run_route_line(guide: &WorkflowFirstRunGuide) -> String {
    let replay = guide
        .routes
        .iter()
        .find(|route| route.route_id == "replay")
        .map(|route| route.command.clone())
        .unwrap_or_else(|| "ict-engine analyze --demo".to_string());
    let factor_loop = guide
        .routes
        .iter()
        .find(|route| route.route_id == "backtest_or_factor_loop")
        .map(|route| route.command.clone())
        .unwrap_or_else(|| "ict-engine factor-research --data <historical-data.json>".to_string());
    format!(
        "Routes: replay={} | factors/backtest={} | live bootstrap={}",
        human_display_command(&replay),
        human_display_command(&factor_loop),
        human_display_command(&guide.bootstrap_command)
    )
}

fn first_run_next_action(guide: &WorkflowFirstRunGuide) -> String {
    format!(
        "Start with {}. Then choose replay, factors/backtest, or live from the routes below.",
        guide.provider_command
    )
}

fn latest_auto_quant_handoff_entry(snapshot: &WorkflowSnapshot) -> Option<&ArtifactLedgerEntry> {
    snapshot
        .actionable_artifacts
        .iter()
        .filter(|entry| entry.artifact_kind == "auto_quant_handoff_candidate")
        .max_by_key(|entry| entry.generated_at)
}

fn load_auto_quant_handoff_payload(
    entry: &ArtifactLedgerEntry,
) -> Option<AutoQuantResearchHandoffPayload> {
    std::fs::read_to_string(&entry.path)
        .ok()
        .and_then(|raw| serde_json::from_str::<AutoQuantResearchHandoffPayload>(&raw).ok())
}

fn build_auto_quant_handoff_guide(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> Option<WorkflowAutoQuantHandoffGuide> {
    let entry = latest_auto_quant_handoff_entry(snapshot)?;
    let payload = load_auto_quant_handoff_payload(entry)?;
    let recommended_next_command = payload
        .readiness
        .as_ref()
        .map(|readiness| readiness.recommended_next_command.clone())
        .unwrap_or_default();
    Some(WorkflowAutoQuantHandoffGuide {
        active: true,
        artifact_id: payload.artifact_id.clone(),
        handoff_kind: payload.handoff_kind.clone(),
        status: payload
            .readiness
            .as_ref()
            .map(|readiness| readiness.status.clone())
            .unwrap_or_else(|| "status_unavailable".to_string()),
        data_ready: payload.data_ready,
        review_command: format!(
            "ict-engine auto-quant-adoption-review --symbol {} --state-dir {} --artifact-id {}",
            payload.symbol, payload.state_dir, payload.artifact_id
        ),
        workflow_status_command: format!(
            "{} --human",
            workflow_status_base_command(
                &payload.symbol,
                Some(&payload.state_dir),
                provider_status_agent
            )
        ),
        recommended_next_command,
        suggested_next_steps: payload.suggested_next_steps.clone(),
        handoff_artifact_path: payload.handoff_artifact_path.clone(),
    })
}

fn auto_quant_handoff_route_line(guide: &WorkflowAutoQuantHandoffGuide) -> String {
    let next_summary = guide
        .suggested_next_steps
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "Auto-Quant: {} Review: {} Workflow: {}",
        next_summary, guide.review_command, guide.workflow_status_command
    )
}

fn auto_quant_handoff_next_action(guide: &WorkflowAutoQuantHandoffGuide) -> String {
    if guide.recommended_next_command.trim().is_empty() {
        return format!(
            "Continue the Auto-Quant handoff review with {}.",
            guide.review_command
        );
    }
    format!(
        "Continue the Auto-Quant handoff. Run {} and then review with {}.",
        guide.recommended_next_command, guide.review_command
    )
}

fn build_evidence_review_guide(
    snapshot: &WorkflowSnapshot,
    state_dir: Option<&str>,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> Option<WorkflowEvidenceReviewGuide> {
    let latest_phase = latest_workflow_phase(snapshot)?;
    if !matches!(
        latest_phase.phase.as_str(),
        "research" | "backtest" | "analyze" | "update"
    ) {
        return None;
    }
    let raw_state_dir = state_dir;
    let state_dir = shell_quote(raw_state_dir.unwrap_or("<state-dir>"));
    let symbol = shell_quote(&snapshot.symbol);
    let ensemble_vote_command = format!(
        "{} --phase ensemble-vote",
        workflow_status_base_command(&snapshot.symbol, raw_state_dir, provider_status_agent)
    );
    let pre_bayes_status_command = format!(
        "ict-engine pre-bayes-status --symbol {} --state-dir {}",
        symbol, state_dir
    );
    let structural_path_command = format!(
        "{} --phase structural-recommended-path-bundle",
        workflow_status_base_command(&snapshot.symbol, raw_state_dir, provider_status_agent)
    );
    Some(WorkflowEvidenceReviewGuide {
        active: true,
        summary: format!(
            "Review the latest ensemble, pre-bayes evidence, and structural path before rerunning {}.",
            latest_phase.phase
        ),
        ensemble_vote_command,
        pre_bayes_status_command,
        structural_path_command,
        follow_up_command: Some(snapshot.recommended_next_command.clone()),
    })
}

fn evidence_review_route_line(guide: &WorkflowEvidenceReviewGuide) -> String {
    format!(
        "Evidence: {} | {} | {}",
        human_display_command(&guide.ensemble_vote_command),
        human_display_command(&guide.pre_bayes_status_command),
        human_display_command(&guide.structural_path_command)
    )
}

fn workflow_human_deferred_command(
    command: &str,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty()
        || trimmed == "recommended_command_unavailable"
        || trimmed == "next_command_unavailable"
    {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("ask-user: ") {
        return rest
            .split(" | then ")
            .nth(1)
            .map(str::trim)
            .filter(|value| {
                !value.is_empty()
                    && *value != "choose historical dataset with user before running command"
            })
            .map(|value| apply_selected_profile_to_workflow_command(value, provider_status_agent));
    }
    if trimmed.starts_with("blocked:") {
        return None;
    }
    Some(apply_selected_profile_to_workflow_command(
        trimmed,
        provider_status_agent,
    ))
}

pub fn executor_scorecard_surface(
    persisted_scorecards: &[EnsembleExecutorScorecard],
    fallback_scorecards: &[EnsembleExecutorScorecard],
) -> (Vec<EnsembleExecutorScorecard>, &'static str) {
    if persisted_scorecards.is_empty() {
        (fallback_scorecards.to_vec(), "fallback")
    } else {
        (persisted_scorecards.to_vec(), "persisted")
    }
}

pub fn resolved_vote_scorecards(
    persisted_scorecards: &[EnsembleExecutorScorecard],
    vote: &EnsembleVoteRecord,
) -> (Vec<EnsembleExecutorScorecard>, &'static str) {
    executor_scorecard_surface(persisted_scorecards, &vote.executor_scorecards)
}

fn push_paths_from_command_text(candidates: &mut Vec<String>, command: &str) {
    let tokens = command
        .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == '|' || ch == ';')
        .map(|token| token.trim_matches(|ch| ch == '\'' || ch == '"'));
    for token in tokens {
        if (token.starts_with('/') || token.starts_with("./") || token.starts_with("../"))
            && (token.ends_with(".json") || token.ends_with(".csv"))
            && !candidates.iter().any(|existing| existing == token)
        {
            candidates.push(token.to_string());
        }
    }
}

fn historical_data_candidates(snapshot: &WorkflowSnapshot) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(update) = &snapshot.latest_update {
        for line in &update.multi_timeframe_summary {
            if let Some(path) = line.split("path=").nth(1) {
                push_paths_from_command_text(&mut candidates, path);
                let trimmed = path.trim();
                if !trimmed.is_empty() && !candidates.iter().any(|existing| existing == trimmed) {
                    candidates.push(trimmed.to_string());
                }
            }
        }
    }

    push_paths_from_command_text(&mut candidates, &snapshot.blocking_truth.next_command);
    push_paths_from_command_text(&mut candidates, &snapshot.recommended_next_command);
    for phase in [
        snapshot.latest_update.as_ref(),
        snapshot.latest_research.as_ref(),
        snapshot.latest_analyze.as_ref(),
        snapshot.latest_backtest.as_ref(),
        snapshot.latest_train.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        push_paths_from_command_text(&mut candidates, &phase.recommended_next_command);
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn latest_workflow_phase(
    snapshot: &WorkflowSnapshot,
) -> Option<&crate::state::WorkflowPhaseSnapshot> {
    [
        snapshot.latest_update.as_ref(),
        snapshot.latest_research.as_ref(),
        snapshot.latest_analyze.as_ref(),
        snapshot.latest_backtest.as_ref(),
        snapshot.latest_train.as_ref(),
    ]
    .into_iter()
    .flatten()
    .max_by(|left, right| left.timestamp.cmp(&right.timestamp))
}

pub fn build_human_workflow_status_view(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
) -> Value {
    let provider_status_agent = provider_status_agent_surface(None, None, None).unwrap_or_default();
    build_human_workflow_status_view_with_provider_agent_and_structural_prior_state(
        snapshot,
        persisted_scorecards,
        &provider_status_agent,
        &[],
        &StructuralPriorLearningState::default(),
    )
}

#[cfg(test)]
fn build_human_workflow_status_view_with_provider_agent(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
) -> Value {
    build_human_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
        snapshot,
        persisted_scorecards,
        provider_status_agent,
        feedback_history,
        &StructuralPriorLearningState::default(),
        None,
    )
}

fn build_human_workflow_status_view_with_provider_agent_and_structural_prior_state(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> Value {
    build_human_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
        snapshot,
        persisted_scorecards,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        None,
    )
}

fn build_human_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    state_dir: Option<&str>,
) -> Value {
    let no_workflow_state = workflow_status_empty_state(snapshot);
    let latest_phase = latest_workflow_phase(snapshot);
    let latest_phase_label = latest_phase
        .map(|phase| phase.phase.clone())
        .unwrap_or_else(|| NO_WORKFLOW_STATE.to_string());
    let latest_phase_summary = latest_phase
        .map(|phase| phase.phase_summary.clone())
        .unwrap_or_else(|| NO_WORKFLOW_PHASE_SUMMARY.to_string());
    let latest_pda_cluster = latest_phase
        .and_then(|phase| phase.pda_cluster_label.clone())
        .unwrap_or_else(|| "unavailable".to_string());
    let latest_duration_model = latest_phase
        .and_then(|phase| phase.hybrid_duration_model.clone())
        .unwrap_or_else(|| "unavailable".to_string());
    let latest_remaining_bars = latest_phase
        .and_then(|phase| phase.hybrid_remaining_expected_bars)
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "unavailable".to_string());
    // Round 2 §3.4: surface spectral / sparsity / segment gate on the main
    // summary line so operators can spot a chaotic-regime or sparsity-collapse
    // without hunting through artifact files.
    let latest_spectral_entropy = latest_phase
        .and_then(|phase| phase.spectral_entropy)
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "unavailable".to_string());
    let latest_sparsity_ratio = latest_phase
        .and_then(|phase| phase.sparsity_ratio)
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "unavailable".to_string());
    let latest_segments_gate = latest_phase
        .and_then(|phase| phase.segments_gate.clone())
        .unwrap_or_else(|| "unavailable".to_string());
    let latest_phase_summary_short = latest_phase
        .map(short_human_phase_summary)
        .unwrap_or_else(|| NO_WORKFLOW_PHASE_SUMMARY.to_string());
    let selected_data_candidates = historical_data_candidates(snapshot);
    let hard_block_statuses = [
        "blocked",
        "bridge_needs_confirmation",
        "validated_regressing",
        "credibility_gate_blocked",
    ];
    let hard_block_active = hard_block_statuses
        .iter()
        .any(|status| snapshot.blocking_truth.status == *status);
    let raw_top_level_command = if hard_block_active {
        snapshot.blocking_truth.next_command.clone()
    } else {
        snapshot.recommended_next_command.clone()
    };
    let raw_top_level_command =
        apply_selected_profile_to_workflow_command(&raw_top_level_command, provider_status_agent);
    let provider_status_command = provider_status_agent_command_for_surface(provider_status_agent);
    let auto_quant_handoff_guide = if no_workflow_state && raw_top_level_command.trim().is_empty() {
        build_auto_quant_handoff_guide(snapshot, provider_status_agent)
    } else {
        None
    };
    let first_run_guide = if no_workflow_state
        && raw_top_level_command.trim().is_empty()
        && auto_quant_handoff_guide.is_none()
    {
        Some(build_first_run_guide(
            &snapshot.symbol,
            state_dir,
            provider_status_agent,
        ))
    } else {
        None
    };
    let evidence_review_guide = if no_workflow_state || hard_block_active {
        None
    } else {
        build_evidence_review_guide(snapshot, state_dir, provider_status_agent)
    };
    let top_level_command = auto_quant_handoff_guide
        .as_ref()
        .map(|guide| guide.recommended_next_command.clone())
        .or_else(|| {
            first_run_guide
                .as_ref()
                .map(|guide| guide.bootstrap_command.clone())
        })
        .or_else(|| {
            if raw_top_level_command.contains("factor-research")
                || raw_top_level_command.contains("factor-backtest")
            {
                evidence_review_guide
                    .as_ref()
                    .map(|guide| guide.ensemble_vote_command.clone())
            } else {
                None
            }
        })
        .unwrap_or(raw_top_level_command);
    let historical_data_gate_active = !selected_data_candidates.is_empty()
        && (top_level_command.contains("factor-research")
            || top_level_command.contains("factor-backtest")
            || snapshot
                .recommended_next_command
                .contains("factor-research")
            || snapshot
                .recommended_next_command
                .contains("factor-backtest"));
    let historical_data_request_template = if !selected_data_candidates.is_empty() {
        format!(
            "Please choose one historical data path for the next research/backtest run: {}",
            historical_data_candidate_display_list(&selected_data_candidates)
        )
    } else {
        String::new()
    };
    let user_path_input_prompt = if !selected_data_candidates.is_empty() {
        format!(
            "Reply with one path from the list, or paste another valid file path. Candidates: {}",
            historical_data_candidate_display_list(&selected_data_candidates)
        )
    } else {
        "Reply with a historical data file path to continue research/backtest.".to_string()
    };
    let user_selection_pending = historical_data_gate_active
        || top_level_command.contains("user_selected_historical_data")
        || snapshot
            .blocking_truth
            .reason
            .contains("user_selected_historical_data_missing");
    let deferred_user_selection_command =
        workflow_human_deferred_command(&top_level_command, provider_status_agent);
    let base_human_next_action = if let Some(guide) = auto_quant_handoff_guide.as_ref() {
        auto_quant_handoff_next_action(guide)
    } else if let Some(guide) = first_run_guide.as_ref() {
        first_run_next_action(guide)
    } else if let Some(guide) = evidence_review_guide.as_ref() {
        format!(
            "{} Start with {}.",
            guide.summary,
            human_display_command(&guide.ensemble_vote_command)
        )
    } else if user_selection_pending {
        if !historical_data_request_template.is_empty() {
            match deferred_user_selection_command.as_deref() {
                Some(command) => format!(
                    "Ask the user to choose the historical dataset. {} {} Then run: {}",
                    historical_data_request_template, user_path_input_prompt, command
                ),
                None => format!(
                    "Ask the user to choose the historical dataset. {} {}",
                    historical_data_request_template, user_path_input_prompt
                ),
            }
        } else {
            match deferred_user_selection_command.as_deref() {
                Some(command) => format!(
                    "Ask the user to provide the historical data path before running research/backtest. Then run: {}",
                    command
                ),
                None => {
                    "Ask the user to provide the historical data path before running research/backtest."
                        .to_string()
                }
            }
        }
    } else {
        humanize_workflow_command(&top_level_command)
    };
    let action_status_label = if user_selection_pending {
        "action_blocked".to_string()
    } else if no_workflow_state {
        NO_WORKFLOW_STATE.to_string()
    } else if snapshot.blocking_truth.status.is_empty() {
        "unblocked".to_string()
    } else {
        snapshot.blocking_truth.status.clone()
    };
    let gate_reason_label = if user_selection_pending {
        "user_selected_historical_data_missing".to_string()
    } else if no_workflow_state {
        NO_WORKFLOW_STATE.to_string()
    } else if hard_block_active && !snapshot.blocking_truth.reason.is_empty() {
        snapshot.blocking_truth.reason.clone()
    } else {
        "none".to_string()
    };
    let provider_support_reason =
        if gate_reason_label != "none" && gate_reason_label != NO_WORKFLOW_STATE {
            Some(gate_reason_label.as_str())
        } else if !snapshot.current_focus_reason.is_empty() {
            Some(snapshot.current_focus_reason.as_str())
        } else {
            None
        };
    let provider_support = build_workflow_provider_support(
        provider_status_agent,
        &top_level_command,
        provider_support_reason,
    );
    let selected_profile_summary = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| profile.summary.clone());
    let selected_profile_data_contracts = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| {
            profile
                .data_contract_labels
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let selected_profile_track_statuses = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| {
            profile
                .track_statuses
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let dataset_resolution_line = build_dataset_resolution_line(provider_status_agent);
    let opt_in_profile_line = if no_workflow_state
        && provider_status_agent.selected_profile.is_none()
        && !provider_status_agent.available_opt_in_profiles.is_empty()
    {
        provider_status_agent
            .available_opt_in_profiles
            .iter()
            .find(|profile| profile_matches_symbol(profile, &snapshot.symbol))
            .map(|profile| {
                format!(
                    "Optional personal lane: {}. Reuse with ict-engine workflow-status --symbol {} --state-dir <local-path> --profile {} --human if you want your saved data/runtime mix.",
                    profile.summary,
                    shell_quote(&snapshot.symbol),
                    shell_quote(&profile.selector)
                )
            })
    } else {
        None
    };
    let human_next_action = if provider_support.active {
        let ask_summary = provider_support
            .ask_user_prompts
            .first()
            .cloned()
            .unwrap_or_else(|| {
                format!(
                    "Resolve provider prerequisites for {}.",
                    provider_support.pending_providers.join(", ")
                )
            });
        format!("{} {}", ask_summary, base_human_next_action)
    } else {
        base_human_next_action
    };
    let provider_line = if let Some(guide) = first_run_guide.as_ref() {
        Some(format!(
            "Provider: {} Check: {}",
            guide.provider_summary, guide.provider_command
        ))
    } else if provider_support.active {
        let prompt_summary = provider_support
            .ask_user_prompts
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let prompt_summary = if prompt_summary.is_empty() {
            provider_support
                .install_prompts
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            prompt_summary
        };
        Some(format!(
            "Provider: {} pending. {} Check: {}",
            provider_support.pending_providers.join(", "),
            prompt_summary,
            provider_status_command
        ))
    } else {
        None
    };
    let route_line = if let Some(guide) = auto_quant_handoff_guide.as_ref() {
        Some(auto_quant_handoff_route_line(guide))
    } else if let Some(guide) = evidence_review_guide.as_ref() {
        Some(evidence_review_route_line(guide))
    } else {
        first_run_guide.as_ref().map(first_run_route_line)
    };
    let mut summary_parts = vec![
        snapshot.symbol.clone(),
        workflow_status_focus_phase(snapshot),
        action_status_label.clone(),
    ];
    if latest_pda_cluster != "unavailable" {
        summary_parts.push(format!("pda_cluster={latest_pda_cluster}"));
    }
    if latest_duration_model != "unavailable" {
        summary_parts.push(format!("duration={latest_duration_model}"));
    }
    if latest_remaining_bars != "unavailable" {
        summary_parts.push(format!("remaining_bars={latest_remaining_bars}"));
    }
    if latest_spectral_entropy != "unavailable" {
        summary_parts.push(format!("spectral_entropy={latest_spectral_entropy}"));
    }
    if latest_sparsity_ratio != "unavailable" {
        summary_parts.push(format!("sparsity={latest_sparsity_ratio}"));
    }
    if latest_segments_gate != "unavailable" {
        summary_parts.push(format!("segments_gate={latest_segments_gate}"));
    }
    let summary_line = summary_parts.join(" | ");
    let blocking_line = format!("Block: {}", gate_reason_label);
    let next_action_display = human_next_action
        .strip_prefix("Next step: ")
        .unwrap_or(&human_next_action);
    let next_action_line = format!("Next: {}", next_action_display);
    let phase_summary_line = format!(
        "Latest: {} | {}",
        latest_phase_label, latest_phase_summary_short
    );
    let structural_feedback_line = snapshot
        .latest_update
        .as_ref()
        .and_then(|phase| phase.structural_feedback.as_ref())
        .map(|feedback| {
            format!(
                "Feedback: recommendation={} path={} followed={} exit={}",
                feedback.recommendation_id,
                feedback.path_id,
                feedback.followed_path,
                feedback
                    .exit_reason
                    .clone()
                    .unwrap_or_else(|| "unreported".to_string())
            )
        });
    let structural_path_line = snapshot
        .latest_update
        .as_ref()
        .and_then(|phase| phase.structural_feedback.as_ref())
        .and_then(|feedback| {
            build_structural_path_history_artifact(snapshot, feedback_history)
                .paths
                .into_iter()
                .find(|path| path.path_id == feedback.path_id)
        })
        .map(|path| {
            format!(
                "Path: {} total={} wins={} losses={} invalidated={} avg_pnl={:.4}",
                path.path_id,
                path.total_records,
                path.wins,
                path.losses,
                path.invalidated,
                path.avg_pnl
            )
        });
    let structural_history_summary =
        build_structural_history_summary_artifact(snapshot, feedback_history);
    let experience_prior_surface =
        build_structural_experience_prior_surface_artifact_with_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
        );
    let structural_temporal_summary = build_structural_temporal_summary_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        structural_prior_state,
    );
    let experience_prior_line = experience_prior_surface.path.as_ref().map(|path| {
        format!(
            "Experience: path_prior={:.3} path_score={:.3} total={} wins={}",
            path.experience_prior,
            path.composite_score,
            path.historical_total_records,
            ((path.historical_win_rate.unwrap_or(0.0) * path.historical_followed_count as f64)
                .round()) as usize
        )
    });
    let structural_validation_line = build_structural_validation_line(&experience_prior_surface);
    let structural_temporal_line = Some(format!(
        "Temporal: {}",
        structural_temporal_summary.summary_line
    ));
    let runtime_context = StructuralPathRankerRuntimeContext { state_dir };
    let top_path_candidates =
        build_structural_top_path_candidates_artifact_with_runtime_context_and_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            runtime_context.clone(),
        );
    let path_ranking_target =
        build_structural_path_ranking_target_artifact_with_runtime_context_and_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            runtime_context.clone(),
        );
    let recommended_path_bundle =
        build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            runtime_context,
        );
    let execution_contract_active = !no_workflow_state
        && !hard_block_active
        && !historical_data_gate_active
        && !provider_support.active;
    let top_path_candidates_line = if top_path_candidates.candidates.is_empty() {
        None
    } else {
        Some(format!(
            "Candidates: {}",
            top_path_candidates
                .candidates
                .iter()
                .take(2)
                .map(|candidate| format!(
                    "{} score={:.3} prior={:.3} post={:.3}{}",
                    candidate.path_label,
                    candidate.composite_score,
                    candidate.experience_prior,
                    candidate.current_posterior,
                    candidate
                        .path_ranker_path_prob_lower_bound
                        .map(|value| format!(" lb={value:.3}"))
                        .or_else(|| {
                            candidate
                                .path_ranker_raw_score
                                .map(|value| format!(" raw={value:.3}"))
                        })
                        .unwrap_or_default()
                ))
                .collect::<Vec<_>>()
                .join(" | ")
        ))
    };
    let recommended_path_line = recommended_path_bundle.as_ref().map(|bundle| {
        format!(
            "Recommended: {} score={:.3}{} trigger={} stop={} invalidation={}",
            bundle.path_label,
            bundle.composite_score,
            bundle
                .path_ranker_path_prob_lower_bound
                .map(|value| format!(" lb={value:.3}"))
                .or_else(|| {
                    bundle
                        .path_ranker_raw_score
                        .map(|value| format!(" raw={value:.3}"))
                })
                .unwrap_or_default(),
            bundle.trigger_summary,
            bundle.stop_summary,
            bundle.invalidation_summary
        )
    });
    let human_next_action = if execution_contract_active {
        match recommended_path_bundle.as_ref() {
            Some(bundle) => format!(
                "{} Execution contract: path={} trigger={} stop={} invalidation={} why={}",
                human_next_action,
                bundle.path_label,
                bundle.trigger_summary,
                bundle.stop_summary,
                bundle.invalidation_summary,
                bundle.why_this_path
            ),
            None => human_next_action,
        }
    } else {
        human_next_action
    };
    let recommended_path_contract_line = recommended_path_bundle.as_ref().map(|bundle| {
        format!(
            "Contract: path={} trigger={} stop={} invalidation={} why={}",
            bundle.path_label,
            bundle.trigger_summary,
            bundle.stop_summary,
            bundle.invalidation_summary,
            bundle.why_this_path
        )
    });
    let path_ranker_line = build_path_ranker_line(recommended_path_bundle.as_ref());
    let recommended_path_contract =
        workflow_status_recommended_path_contract_value(recommended_path_bundle.as_ref());
    let recommended_next_step = workflow_status_next_step_with_execution_contract(
        &top_level_command,
        if hard_block_active || historical_data_gate_active {
            Some(gate_reason_label.as_str())
        } else {
            None
        },
        if execution_contract_active {
            recommended_path_contract.clone()
        } else {
            None
        },
    );
    let structural_history_line = if structural_history_summary.total_records > 0 {
        Some(format!(
            "History: nodes={} branches={} scenarios={} paths={} latest_path={}",
            structural_history_summary.distinct_nodes,
            structural_history_summary.distinct_branches,
            structural_history_summary.distinct_scenarios,
            structural_history_summary.distinct_paths,
            structural_history_summary
                .latest_path_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        ))
    } else {
        None
    };
    let credibility_risks = snapshot
        .risk_flags
        .iter()
        .filter(|flag| {
            flag.contains("conformal_coverage_low")
                || flag.contains("regime_break_penalty_high")
                || flag.contains("structural_break_detected")
                || flag.contains("conformal_credibility")
        })
        .cloned()
        .collect::<Vec<_>>();
    let ensemble_summary = resolved_latest_ensemble_vote(snapshot)
        .as_ref()
        .map(|vote| {
            let surface = build_ensemble_vote_surface(vote, persisted_scorecards);
            serde_json::to_value(surface).unwrap_or_default()
        });
    let agent_fill_path_instructions = if selected_data_candidates.is_empty() {
        Vec::new()
    } else {
        selected_data_candidates
            .iter()
            .map(|path| {
                format!(
                    "Ask user to confirm --data {} before running factor-research/factor-backtest.",
                    path
                )
            })
            .collect::<Vec<_>>()
    };
    let mut value = serde_json::json!({
        "status": if no_workflow_state {
            serde_json::Value::String(NO_WORKFLOW_STATE.to_string())
        } else {
            serde_json::Value::Null
        },
        "summary_line": summary_line,
        "blocking_line": blocking_line,
        "next_action_line": next_action_line,
        "provider_line": provider_line,
        "route_line": route_line,
        "opt_in_profile_line": opt_in_profile_line,
        "structural_feedback_line": structural_feedback_line,
        "structural_path_line": structural_path_line,
        "experience_prior_line": experience_prior_line,
        "structural_validation_line": structural_validation_line,
        "path_ranker_line": path_ranker_line,
        "top_path_candidates_line": top_path_candidates_line,
        "structural_history_line": structural_history_line,
        "phase_summary_line": phase_summary_line,
        "symbol": snapshot.symbol,
        "pda_cluster_label": if latest_pda_cluster == "unavailable" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(latest_pda_cluster.clone())
        },
        "hybrid_duration_model": if latest_duration_model == "unavailable" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(latest_duration_model.clone())
        },
        "hybrid_remaining_expected_bars": if latest_remaining_bars == "unavailable" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(latest_remaining_bars.clone())
        },
        "spectral_entropy": if latest_spectral_entropy == "unavailable" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(latest_spectral_entropy.clone())
        },
        "sparsity_ratio": if latest_sparsity_ratio == "unavailable" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(latest_sparsity_ratio.clone())
        },
        "segments_gate": if latest_segments_gate == "unavailable" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(latest_segments_gate.clone())
        },
        "current_status": {
            "focus_phase": workflow_status_focus_phase(snapshot),
            "focus_reason": snapshot.current_focus_reason,
            "blocking_stage": if historical_data_gate_active {
                workflow_status_focus_phase(snapshot)
            } else {
                snapshot.blocking_truth.stage.clone()
            },
            "blocking_status": action_status_label,
            "blocking_reason": gate_reason_label,
            "hard_block_active": hard_block_active || historical_data_gate_active,
            "top_level_command_source": if historical_data_gate_active {
                "historical_data_selection_gate"
            } else if hard_block_active {
                "blocking_truth"
            } else {
                "recommended_next_command"
            },
        },
        "hard_block": if hard_block_active || historical_data_gate_active {
            serde_json::json!({
                "active": true,
                "stage": if historical_data_gate_active {
                    snapshot.current_focus_phase.clone()
                } else {
                    snapshot.blocking_truth.stage.clone()
                },
                "status": action_status_label,
                "reason": gate_reason_label,
                "evidence": if historical_data_gate_active {
                    Vec::<String>::new()
                } else {
                    snapshot.blocking_truth.evidence.clone()
                },
                "command": if hard_block_active {
                    serde_json::Value::String(snapshot.blocking_truth.next_command.clone())
                } else {
                    serde_json::Value::Null
                },
                "human_action": human_next_action,
            })
        } else {
            serde_json::json!({
                "active": false,
                "stage": serde_json::Value::Null,
                "status": serde_json::Value::Null,
                "reason": serde_json::Value::Null,
                "evidence": Vec::<String>::new(),
                "command": serde_json::Value::Null,
                "human_action": serde_json::Value::Null,
            })
        },
        "what_you_should_do_now": human_next_action,
        "what_you_should_do_now_source": if historical_data_gate_active {
            "historical_data_selection_gate"
        } else if hard_block_active {
            "blocking_truth"
        } else {
            "recommended_next_command"
        },
        "latest_stage": {
            "phase": latest_phase_label,
            "summary": latest_phase_summary,
            "summary_short": latest_phase_summary_short,
        },
        "ensemble_consensus": ensemble_summary,
        "credibility_risks": credibility_risks,
        "pending_actions": snapshot.pending_actions,
        "risk_flags": snapshot.risk_flags,
        "historical_data_candidates": selected_data_candidates,
        "historical_data_request_template": historical_data_request_template,
        "user_path_input_prompt": user_path_input_prompt,
        "agent_fill_path_instructions": agent_fill_path_instructions,
        "jump_model": jump_model_workflow_summary(snapshot),
        "jump_calibration_gate": jump_calibration_gate_workflow_summary(snapshot),
        "latest_structural_feedback": snapshot
            .latest_update
            .as_ref()
            .and_then(|phase| phase.structural_feedback.clone()),
        "structural_history_summary": structural_history_summary,
        "structural_path_summary": snapshot
            .latest_update
            .as_ref()
            .and_then(|phase| phase.structural_feedback.as_ref())
            .and_then(|feedback| {
                build_structural_path_history_artifact(snapshot, feedback_history)
                    .paths
                    .into_iter()
                    .find(|path| path.path_id == feedback.path_id)
            }),
        "jump_disagreement": snapshot
            .latest_ensemble_vote
            .as_ref()
            .and_then(|vote| {
                vote.executor_summaries
                    .iter()
                    .find(|line| line.contains("jump_disagreement"))
                    .cloned()
            }),
    });
    if let Value::Object(map) = &mut value {
        map.insert(
            "provider_support".to_string(),
            serde_json::json!({
                "command": provider_status_command,
                "agent_summary": provider_status_agent,
                "workflow_support": {
                    "active": provider_support.active,
                    "profile_id": provider_support.profile_id,
                    "provider_status_command": provider_support.provider_status_command,
                    "pending_providers": provider_support.pending_providers,
                    "ask_user_prompts": provider_support.ask_user_prompts,
                    "install_prompts": provider_support.install_prompts,
                },
            }),
        );
        map.insert(
            "first_run_router".to_string(),
            serde_json::to_value(first_run_guide).unwrap_or_default(),
        );
        map.insert(
            "auto_quant_handoff".to_string(),
            serde_json::to_value(auto_quant_handoff_guide).unwrap_or_default(),
        );
        map.insert(
            "evidence_review".to_string(),
            serde_json::to_value(evidence_review_guide).unwrap_or_default(),
        );
        map.insert(
            "structural_temporal_line".to_string(),
            serde_json::to_value(structural_temporal_line).unwrap_or_default(),
        );
        map.insert(
            "recommended_path_line".to_string(),
            serde_json::to_value(recommended_path_line).unwrap_or_default(),
        );
        map.insert(
            "recommended_path_contract_line".to_string(),
            serde_json::to_value(recommended_path_contract_line).unwrap_or_default(),
        );
        map.insert(
            "path_ranking_target".to_string(),
            serde_json::to_value(path_ranking_target).unwrap_or_default(),
        );
        map.insert(
            "selected_profile_id".to_string(),
            serde_json::to_value(
                provider_status_agent
                    .selected_profile
                    .as_ref()
                    .map(|profile| profile.profile_id.clone()),
            )
            .unwrap_or_default(),
        );
        map.insert(
            "selected_profile_summary".to_string(),
            serde_json::to_value(selected_profile_summary).unwrap_or_default(),
        );
        map.insert(
            "selected_profile_data_contracts".to_string(),
            serde_json::to_value(selected_profile_data_contracts).unwrap_or_default(),
        );
        map.insert(
            "selected_profile_track_statuses".to_string(),
            serde_json::to_value(selected_profile_track_statuses).unwrap_or_default(),
        );
        map.insert(
            "dataset_resolution_line".to_string(),
            serde_json::to_value(dataset_resolution_line).unwrap_or_default(),
        );
        map.insert("recommended_next_step".to_string(), recommended_next_step);
    }
    value
}

pub fn build_compact_workflow_status_view(snapshot: &WorkflowSnapshot) -> Value {
    let blocking_status = if snapshot.blocking_truth.status.is_empty() {
        "unblocked".to_string()
    } else {
        snapshot.blocking_truth.status.clone()
    };
    let blocking_reason = if snapshot.blocking_truth.status.is_empty() {
        "none".to_string()
    } else {
        snapshot.blocking_truth.reason.clone()
    };
    let latest_phase = latest_workflow_phase(snapshot);
    let latest_phase_label = latest_phase
        .map(|phase| phase.phase.clone())
        .unwrap_or_else(|| "workflow_phase_unavailable".to_string());
    let latest_phase_summary = latest_phase
        .map(short_workflow_phase_summary)
        .unwrap_or_else(|| "workflow_phase_summary_unavailable".to_string());
    let top_actionable = snapshot.actionable_artifacts.first().map(|artifact| {
        serde_json::json!({
            "artifact_id": artifact.artifact_id,
            "artifact_kind": artifact.artifact_kind,
            "decision_hint": artifact.decision_hint,
            "generated_at": artifact.generated_at,
        })
    });
    let top_disagreement = snapshot.disagreements.first().map(|item| {
        serde_json::json!({
            "id": item.id,
            "severity": item.severity,
            "summary": item.summary,
        })
    });
    serde_json::json!({
        "symbol": snapshot.symbol,
        "generated_at": snapshot.generated_at,
        "focus_phase": snapshot.current_focus_phase,
        "focus_reason": snapshot.current_focus_reason,
        "latest_phase": latest_phase_label,
        "latest_phase_summary": latest_phase_summary,
        "blocking_status": blocking_status,
        "blocking_reason": blocking_reason,
        "next_command": snapshot.recommended_next_command,
        "pda_cluster_label": latest_phase.and_then(|phase| phase.pda_cluster_label.clone()),
        "pending_actions": snapshot.pending_actions.iter().take(3).cloned().collect::<Vec<_>>(),
        "risk_flags": snapshot.risk_flags.iter().take(3).cloned().collect::<Vec<_>>(),
        "top_actionable": top_actionable,
        "top_disagreement": top_disagreement,
    })
}

pub fn build_agent_workflow_status_view(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
) -> Value {
    let provider_status_agent = provider_status_agent_surface(None, None, None).unwrap_or_default();
    build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state(
        snapshot,
        persisted_scorecards,
        &provider_status_agent,
        &[],
        &StructuralPriorLearningState::default(),
    )
}

#[cfg(test)]
fn build_agent_workflow_status_view_with_provider_agent(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
) -> Value {
    build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
        snapshot,
        persisted_scorecards,
        provider_status_agent,
        feedback_history,
        &StructuralPriorLearningState::default(),
        None,
    )
}

fn build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> Value {
    build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
        snapshot,
        persisted_scorecards,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        None,
    )
}

fn build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    state_dir: Option<&str>,
) -> Value {
    let no_workflow_state = workflow_status_empty_state(snapshot);
    let latest_phase = latest_workflow_phase(snapshot);
    let latest_phase_label = latest_phase
        .map(|phase| phase.phase.clone())
        .unwrap_or_else(|| NO_WORKFLOW_STATE.to_string());
    let latest_phase_summary_short = latest_phase
        .map(short_workflow_phase_summary)
        .unwrap_or_else(|| NO_WORKFLOW_PHASE_SUMMARY.to_string());
    let hard_block_statuses = [
        "blocked",
        "bridge_needs_confirmation",
        "validated_regressing",
        "credibility_gate_blocked",
    ];
    let hard_block_active = hard_block_statuses
        .iter()
        .any(|status| snapshot.blocking_truth.status == *status);
    let command_source = if hard_block_active {
        "blocking_truth"
    } else {
        "recommended_next_command"
    };
    let raw_next_command = if hard_block_active {
        snapshot.blocking_truth.next_command.clone()
    } else {
        snapshot.recommended_next_command.clone()
    };
    let raw_next_command =
        apply_selected_profile_to_workflow_command(&raw_next_command, provider_status_agent);
    let auto_quant_handoff_guide = if no_workflow_state && raw_next_command.trim().is_empty() {
        build_auto_quant_handoff_guide(snapshot, provider_status_agent)
    } else {
        None
    };
    let first_run_guide = if no_workflow_state
        && raw_next_command.trim().is_empty()
        && auto_quant_handoff_guide.is_none()
    {
        Some(build_first_run_guide(
            &snapshot.symbol,
            state_dir,
            provider_status_agent,
        ))
    } else {
        None
    };
    let evidence_review_guide = if no_workflow_state || hard_block_active {
        None
    } else {
        build_evidence_review_guide(snapshot, state_dir, provider_status_agent)
    };
    let next_command = auto_quant_handoff_guide
        .as_ref()
        .map(|guide| guide.recommended_next_command.clone())
        .or_else(|| {
            first_run_guide
                .as_ref()
                .map(|guide| guide.bootstrap_command.clone())
        })
        .or_else(|| {
            if raw_next_command.contains("factor-research")
                || raw_next_command.contains("factor-backtest")
            {
                evidence_review_guide
                    .as_ref()
                    .map(|guide| guide.ensemble_vote_command.clone())
            } else {
                None
            }
        })
        .unwrap_or(raw_next_command);
    let next_command_value = if no_workflow_state && next_command.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(next_command.clone())
    };
    let blocking_status = if hard_block_active {
        snapshot.blocking_truth.status.clone()
    } else if no_workflow_state {
        NO_WORKFLOW_STATE.to_string()
    } else {
        "unblocked".to_string()
    };
    let blocking_reason = if hard_block_active {
        snapshot.blocking_truth.reason.clone()
    } else if no_workflow_state {
        NO_WORKFLOW_STATE.to_string()
    } else {
        "none".to_string()
    };
    let top_disagreement = snapshot.disagreements.first().map(|item| {
        serde_json::json!({
            "id": item.id,
            "severity": item.severity,
            "summary": item.summary,
            "recommended_action": item.recommended_action,
        })
    });
    let top_actionable = snapshot.actionable_artifacts.first().map(|artifact| {
        serde_json::json!({
            "artifact_id": artifact.artifact_id,
            "artifact_kind": artifact.artifact_kind,
            "decision_hint": artifact.decision_hint,
        })
    });
    let ensemble_summary = resolved_latest_ensemble_vote(snapshot)
        .as_ref()
        .map(|vote| {
            let (scorecards, scorecard_source) =
                resolved_vote_scorecards(persisted_scorecards, vote);
            serde_json::json!({
                "final_action": vote.final_action,
                "confidence": vote.confidence,
                "consensus_strength": vote.consensus_strength,
                "hard_block_active": vote.hard_block.active,
                "hard_block_reason": vote.hard_block.reason,
                "recommended_command": vote.recommended_command,
                "executor_scorecard_source": scorecard_source,
                "top_executor": scorecards.first().map(|item| {
                    serde_json::json!({
                        "executor": item.executor,
                        "latest_weight_hint": item.latest_weight_hint,
                        "wins": item.wins,
                    })
                }),
            })
        });
    let provider_support_reason =
        if blocking_reason != "none" && blocking_reason != NO_WORKFLOW_STATE {
            Some(blocking_reason.as_str())
        } else if !snapshot.current_focus_reason.is_empty() {
            Some(snapshot.current_focus_reason.as_str())
        } else {
            None
        };
    let provider_support = build_workflow_provider_support(
        provider_status_agent,
        &next_command,
        provider_support_reason,
    );
    let selected_profile_summary = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| profile.summary.clone());
    let selected_profile_data_contracts = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| {
            profile
                .data_contract_labels
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let selected_profile_track_statuses = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| {
            profile
                .track_statuses
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let dataset_resolution_line = build_dataset_resolution_line(provider_status_agent);
    let execution_contract_active =
        !no_workflow_state && !hard_block_active && !provider_support.active;
    let latest_structural_feedback = snapshot
        .latest_update
        .as_ref()
        .and_then(|phase| phase.structural_feedback.clone());
    let structural_path_summary = snapshot
        .latest_update
        .as_ref()
        .and_then(|phase| phase.structural_feedback.as_ref())
        .and_then(|feedback| {
            build_structural_path_history_artifact(snapshot, feedback_history)
                .paths
                .into_iter()
                .find(|path| path.path_id == feedback.path_id)
        });
    let structural_history_summary =
        build_structural_history_summary_artifact(snapshot, feedback_history);
    let experience_prior_surface =
        build_structural_experience_prior_surface_artifact_with_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
        );
    let structural_temporal_summary = build_structural_temporal_summary_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        structural_prior_state,
    );
    let structural_validation_summary =
        build_structural_validation_summary_value(&experience_prior_surface);
    let runtime_context = StructuralPathRankerRuntimeContext { state_dir };
    let top_path_candidates =
        build_structural_top_path_candidates_artifact_with_runtime_context_and_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            runtime_context.clone(),
        );
    let path_ranking_target =
        build_structural_path_ranking_target_artifact_with_runtime_context_and_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            runtime_context.clone(),
        );
    let recommended_path_bundle =
        build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            runtime_context,
        );
    let recommended_path_contract =
        workflow_status_recommended_path_contract_value(recommended_path_bundle.as_ref());
    let next_step = workflow_status_next_step_with_execution_contract(
        &next_command,
        if hard_block_active {
            Some(blocking_reason.as_str())
        } else {
            None
        },
        if execution_contract_active {
            recommended_path_contract.clone()
        } else {
            None
        },
    );
    let mut value = serde_json::json!({
        "status": if no_workflow_state {
            serde_json::Value::String(NO_WORKFLOW_STATE.to_string())
        } else {
            serde_json::Value::Null
        },
        "symbol": snapshot.symbol,
        "generated_at": snapshot.generated_at,
        "focus_phase": workflow_status_focus_phase(snapshot),
        "focus_reason": snapshot.current_focus_reason,
        "latest_phase": latest_phase_label,
        "latest_phase_summary": latest_phase_summary_short,
        "blocking_status": blocking_status,
        "blocking_reason": blocking_reason,
        "hard_block_active": hard_block_active,
        "next_command": next_command_value,
        "next_command_source": if auto_quant_handoff_guide.is_some() {
            "auto_quant_handoff_candidate"
        } else if evidence_review_guide.is_some()
            && (snapshot.recommended_next_command.contains("factor-research")
                || snapshot.recommended_next_command.contains("factor-backtest"))
        {
            "evidence_review_router"
        } else if no_workflow_state && snapshot.recommended_next_command.trim().is_empty() {
            "first_run_router"
        } else {
            command_source
        },
        "pda_cluster_label": latest_phase.and_then(|phase| phase.pda_cluster_label.clone()),
        "hybrid_duration_model": latest_phase.and_then(|phase| phase.hybrid_duration_model.clone()),
        "hybrid_remaining_expected_bars": latest_phase.and_then(|phase| phase.hybrid_remaining_expected_bars),
        "next_step": next_step,
        "pending_actions": snapshot.pending_actions.iter().take(3).cloned().collect::<Vec<_>>(),
        "risk_flags": snapshot.risk_flags.iter().take(3).cloned().collect::<Vec<_>>(),
        "top_disagreement": top_disagreement,
        "top_actionable": top_actionable,
        "ensemble": ensemble_summary,
        "provider_support": provider_support,
        "first_run_router": first_run_guide,
        "auto_quant_handoff": auto_quant_handoff_guide,
        "evidence_review": evidence_review_guide,
        "latest_structural_feedback": latest_structural_feedback,
        "experience_prior_surface": experience_prior_surface,
        "structural_validation_summary": structural_validation_summary,
        "path_ranker_summary": build_path_ranker_summary_value(recommended_path_bundle.as_ref()),
        "top_path_candidates": top_path_candidates.candidates,
        "path_ranking_target": path_ranking_target,
        "available_opt_in_profiles": provider_status_agent.available_opt_in_profiles.clone(),
        "structural_history_summary": structural_history_summary,
        "structural_path_summary": structural_path_summary,
    });
    if let Value::Object(map) = &mut value {
        map.insert(
            "structural_temporal_summary".to_string(),
            serde_json::to_value(structural_temporal_summary).unwrap_or_default(),
        );
        map.insert(
            "recommended_path_bundle".to_string(),
            serde_json::to_value(&recommended_path_bundle).unwrap_or_default(),
        );
        map.insert(
            "recommended_path_contract".to_string(),
            serde_json::to_value(recommended_path_contract).unwrap_or_default(),
        );
        map.insert(
            "selected_profile_id".to_string(),
            serde_json::to_value(
                provider_status_agent
                    .selected_profile
                    .as_ref()
                    .map(|profile| profile.profile_id.clone()),
            )
            .unwrap_or_default(),
        );
        map.insert(
            "selected_profile_summary".to_string(),
            serde_json::to_value(selected_profile_summary).unwrap_or_default(),
        );
        map.insert(
            "selected_profile_data_contracts".to_string(),
            serde_json::to_value(selected_profile_data_contracts).unwrap_or_default(),
        );
        map.insert(
            "selected_profile_track_statuses".to_string(),
            serde_json::to_value(selected_profile_track_statuses).unwrap_or_default(),
        );
        map.insert(
            "dataset_resolution_line".to_string(),
            serde_json::to_value(dataset_resolution_line).unwrap_or_default(),
        );
        map.insert(
            "structural_validation_summary".to_string(),
            build_structural_validation_summary_value(&experience_prior_surface),
        );
        map.insert(
            "path_ranker_summary".to_string(),
            build_path_ranker_summary_value(recommended_path_bundle.as_ref()),
        );
        map.insert(
            "available_opt_in_profiles".to_string(),
            serde_json::to_value(provider_status_agent.available_opt_in_profiles.clone())
                .unwrap_or_default(),
        );
        map.insert("recommended_next_step".to_string(), next_step.clone());
    }
    value
}

fn normalize_workflow_status_value_for_stability(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for key in [
                "generated_at",
                "timestamp",
                "updated_at",
                "last_updated_at",
                "fetched_at",
            ] {
                map.remove(key);
            }
            for child in map.values_mut() {
                normalize_workflow_status_value_for_stability(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_workflow_status_value_for_stability(item);
            }
        }
        _ => {}
    }
}

pub struct WorkflowStatusOutputInput<'a> {
    pub snapshot: &'a WorkflowSnapshot,
    pub persisted_scorecards: &'a [EnsembleExecutorScorecard],
    pub provider_status_agent: &'a ProviderCatalogAgentSurface,
    pub feedback_history: &'a [crate::state::FeedbackRecord],
    pub structural_prior_state: &'a StructuralPriorLearningState,
    pub state_dir: Option<&'a str>,
    pub output_format: &'a str,
    pub stable: bool,
}

pub fn emit_workflow_status_output(input: WorkflowStatusOutputInput<'_>) -> Result<()> {
    let WorkflowStatusOutputInput {
        snapshot,
        persisted_scorecards,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        state_dir,
        output_format,
        stable,
    } = input;
    match output_format.trim().to_ascii_lowercase().as_str() {
        "json" => {
            let mut value = serde_json::to_value(snapshot)?;
            if stable {
                normalize_workflow_status_value_for_stability(&mut value);
            }
            redact_local_paths_in_value(&mut value);
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        "compact" => {
            let mut value = build_compact_workflow_status_view(snapshot);
            if stable {
                normalize_workflow_status_value_for_stability(&mut value);
            }
            redact_local_paths_in_value(&mut value);
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        "agent" => {
            let mut value =
                build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                snapshot,
                persisted_scorecards,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            );
            if stable {
                normalize_workflow_status_value_for_stability(&mut value);
            }
            redact_local_paths_in_value(&mut value);
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        "human" => {
            let value =
                build_human_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                snapshot,
                persisted_scorecards,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            );
            if let Some(summary) = value.get("summary_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(summary));
            }
            if let Some(blocking) = value.get("blocking_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(blocking));
            }
            if let Some(latest) = value.get("phase_summary_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(latest));
            }
            if let Some(next) = value.get("next_action_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(next));
            }
            if let Some(provider) = value.get("provider_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(provider));
            }
            if let Some(routes) = value.get("route_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(routes));
            }
            if let Some(profile) = value.get("opt_in_profile_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(profile));
            }
            if let Some(dataset_resolution) =
                value.get("dataset_resolution_line").and_then(Value::as_str)
            {
                println!("{}", redact_local_paths_in_human_text(dataset_resolution));
            }
            if let Some(feedback) = value
                .get("structural_feedback_line")
                .and_then(Value::as_str)
            {
                println!("{}", redact_local_paths_in_human_text(feedback));
            }
            if let Some(path) = value.get("structural_path_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(path));
            }
            if let Some(validation) = value
                .get("structural_validation_line")
                .and_then(Value::as_str)
            {
                println!("{}", redact_local_paths_in_human_text(validation));
            }
            if let Some(ranker) = value.get("path_ranker_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(ranker));
            }
            if let Some(history) = value.get("structural_history_line").and_then(Value::as_str) {
                println!("{}", redact_local_paths_in_human_text(history));
            }
        }
        other => anyhow::bail!("unsupported output format '{}'", other),
    }
    Ok(())
}

fn emit_workflow_status_bootstrap_human_output(
    symbol: &str,
    state_dir: Option<&str>,
    provider_status_agent: &ProviderCatalogAgentSurface,
) {
    let guide = build_first_run_guide(symbol, state_dir, provider_status_agent);
    print_human_lines(&[
        format!("{symbol} | bootstrap | start_here"),
        format!("Next: {}", first_run_next_action(&guide)),
        format!(
            "Provider: {} Check: {}",
            guide.provider_summary, guide.provider_command
        ),
        first_run_route_line(&guide),
    ]);
}

fn emit_workflow_status_phase_human_output(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    state_dir: Option<&str>,
    phase_key: &str,
) -> Result<bool> {
    match phase_key {
        "human" | "human-next" | "human-next-action" => {
            emit_workflow_status_output(WorkflowStatusOutputInput {
                snapshot,
                persisted_scorecards,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
                output_format: "human",
                stable: false,
            })?;
            Ok(true)
        }
        "agent-bootstrap" | "bootstrap" => {
            emit_workflow_status_bootstrap_human_output(
                &snapshot.symbol,
                state_dir,
                provider_status_agent,
            );
            Ok(true)
        }
        "ensemble-vote" => {
            if let Some(vote) = resolved_latest_ensemble_vote(snapshot) {
                let surface = build_ensemble_vote_surface(&vote, persisted_scorecards);
                let comparability = surface.dataset_comparability.comparison_class.as_str();
                let mut lines = vec![format!(
                    "{} | ensemble-vote | action={} | confidence={:.3} | comparable={}",
                    surface.symbol, surface.final_action, surface.confidence, comparability
                )];
                if let Some(policy_runtime_line) = surface.policy_runtime_line.as_ref() {
                    lines.push(policy_runtime_line.clone());
                }
                if surface.hard_block.active {
                    lines.push(format!(
                        "Block: {}",
                        surface
                            .hard_block
                            .reason
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string())
                    ));
                }
                lines.push(format!(
                    "Next: {}",
                    humanize_workflow_command(&surface.recommended_command)
                        .trim_start_matches("Next step: ")
                ));
                print_human_lines(&lines);
                return Ok(true);
            }
            Ok(false)
        }
        "structural-recommended-path-bundle" | "structural-recommended-path" => {
            if let Some(bundle) =
                build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    StructuralPathRankerRuntimeContext { state_dir },
                )
            {
                print_human_lines(&[
                    format!(
                        "{} | structural-path | {} | posterior={:.3} | selected_prob={:.3}",
                        bundle.symbol,
                        bundle.path_label,
                        bundle.current_posterior,
                        bundle.selected_path_probability
                    ),
                    format!("Trigger: {}", bundle.trigger_summary),
                    format!("Stop: {}", bundle.stop_summary),
                    format!(
                        "Next: {}",
                        humanize_workflow_command(
                            bundle
                                .recommended_command
                                .as_deref()
                                .unwrap_or("recommended_command_unavailable")
                        )
                        .trim_start_matches("Next step: ")
                    ),
                ]);
                return Ok(true);
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

pub fn dispatch_workflow_status(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    input: WorkflowStatusDispatchInput<'_>,
    bootstrap: WorkflowStatusBootstrapInput<'_>,
) -> Result<()> {
    let filter_count = input.actionable_only as u8
        + input.conflicts_only as u8
        + input.latest_promotable as u8
        + input.hard_block_only as u8
        + input.hard_block_reason.is_some() as u8
        + input.limit.is_some() as u8;
    if input.phase.is_some() && filter_count > 0 {
        anyhow::bail!("workflow-status phase and filter flags are mutually exclusive");
    }
    if input.actionable_only as u8 + input.conflicts_only as u8 + input.latest_promotable as u8 > 1
    {
        anyhow::bail!("workflow-status accepts at most one artifact filter flag");
    }
    if input.actionable_only {
        print_redacted_json(&snapshot.actionable_artifacts)?;
        return Ok(());
    }
    if input.conflicts_only {
        print_redacted_json(&snapshot.disagreements)?;
        return Ok(());
    }
    if input.latest_promotable {
        print_redacted_json(&snapshot.latest_promotable_artifact)?;
        return Ok(());
    }
    if input.hard_block_only || input.hard_block_reason.is_some() || input.limit.is_some() {
        let history = filter_hard_block_rows(
            snapshot,
            persisted_scorecards,
            input.hard_block_only,
            input.hard_block_reason,
            input.limit,
        );
        let mut value = serde_json::to_value(history)?;
        if input.stable {
            normalize_workflow_status_value_for_stability(&mut value);
        }
        redact_local_paths_in_value(&mut value);
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }
    if let Some(phase) = input.phase {
        let phase_key = phase.trim().to_ascii_lowercase();
        if input.output_format.eq_ignore_ascii_case("human")
            && emit_workflow_status_phase_human_output(
                snapshot,
                persisted_scorecards,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                Some(bootstrap.state_dir),
                &phase_key,
            )?
        {
            return Ok(());
        }
        let mut value = match phase_key.as_str() {
            "agent-bootstrap" | "bootstrap" => build_workflow_status_bootstrap_phase_value(
                bootstrap.symbol,
                bootstrap.state_dir,
                snapshot,
                provider_status_agent,
                bootstrap.detected_tomac_root,
                bootstrap.multi_timeframe_clean_root,
                &bootstrap.tomac_root_placeholder,
            )?,
            other => build_workflow_status_phase_value_with_structural_prior_state_and_state_dir(
                snapshot,
                persisted_scorecards,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                Some(bootstrap.state_dir),
                other,
            )?,
        };
        if input.stable {
            normalize_workflow_status_value_for_stability(&mut value);
        }
        if phase_key != "agent-bootstrap" && phase_key != "bootstrap" {
            redact_local_paths_in_value(&mut value);
        }
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        emit_workflow_status_output(WorkflowStatusOutputInput {
            snapshot,
            persisted_scorecards,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            state_dir: Some(bootstrap.state_dir),
            output_format: input.output_format,
            stable: input.stable,
        })?;
    }
    Ok(())
}

fn build_agent_bootstrap_view(input: AgentBootstrapBuildInput<'_>) -> AgentBootstrapView {
    build_agent_bootstrap_view_with_candidates(input, build_ibkr_gateway_candidates())
}

fn build_agent_bootstrap_view_with_candidates(
    input: AgentBootstrapBuildInput<'_>,
    ibkr_gateway_candidates: Vec<AgentBootstrapIbkrGatewayCandidate>,
) -> AgentBootstrapView {
    let AgentBootstrapBuildInput {
        symbol,
        state_dir,
        snapshot,
        provider_status_agent,
        detected_tomac_root,
        multi_timeframe_clean_root,
        tomac_root_placeholder,
    } = input;
    let profile_tomac_clean_root = provider_status_agent
        .selected_profile_full
        .as_ref()
        .and_then(selected_profile_tomac_clean_root);
    let profile_tomac_root = profile_tomac_clean_root
        .as_deref()
        .and_then(infer_tomac_root_from_clean_root);
    let bootstrap_tomac_root = if provider_status_agent.selected_profile.is_some() {
        detected_tomac_root.or(profile_tomac_root)
    } else {
        None
    };
    let bootstrap_clean_root = if provider_status_agent.selected_profile.is_some() {
        multi_timeframe_clean_root.or(profile_tomac_clean_root)
    } else {
        None
    };
    let ibkr_gateway_summary = build_ibkr_gateway_summary(&ibkr_gateway_candidates);
    let provider_status_command = provider_status_agent_command_for_surface(provider_status_agent);
    let agent_brief = vec![
        "mission: formalize factor-pipeline debug from latest signal through pre-bayes / bridge / resonance".to_string(),
        "priority: promote expansion_manipulation to SOP-tier objective, not research-only".to_string(),
        "guardrail: do not blind-tune structure_ict before evidence pinpoints the blocking surface".to_string(),
        "success: either find a real structure_ict mutation win or prove near-local-optimum then shift to label refinement / market fork".to_string(),
        "live-provider prerequisite: if IBKR or TradingViewRemix MCP is needed, ask the user for local runtime/API access before attempting provider calls".to_string(),
    ];
    let analyze_command = if let Some(clean_root) = &bootstrap_clean_root {
        format!(
            "ict-engine analyze --symbol {} --data-root {} --state-dir {}",
            shell_quote(symbol),
            shell_quote(clean_root),
            shell_quote(state_dir)
        )
    } else {
        "ict-engine analyze --symbol <symbol> --data-root <clean-root> --state-dir <state-dir>"
            .to_string()
    };
    let train_command = if let Some(clean_root) = &bootstrap_clean_root {
        format!(
            "ict-engine train --symbol {} --data {}/cleaned-15m/{}.continuous-15m.json --epochs 200 --state-dir {}",
            shell_quote(symbol),
            shell_quote(clean_root),
            symbol.to_ascii_lowercase(),
            shell_quote(state_dir)
        )
    } else {
        "ict-engine train --symbol <symbol> --data <clean-root>/cleaned-15m/<market>.continuous-15m.json --epochs 200 --state-dir <state-dir>".to_string()
    };
    let clean_command = if let Some(root) = &bootstrap_tomac_root {
        format!(
            "ict-engine clean-futures --root {} --output-dir {} --multi-timeframe",
            shell_quote(root),
            shell_quote(
                &bootstrap_clean_root
                    .clone()
                    .unwrap_or_else(|| format!("{}/ict-engine-mtf", root))
            )
        )
    } else {
        "ict-engine clean-futures --root <tomac-root> --output-dir <output-dir> --multi-timeframe"
            .to_string()
    };
    let inferable_live_defaults = std::collections::BTreeMap::new();
    let selected_profile_id = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| profile.profile_id.clone());
    let selected_profile_summary = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| profile.summary.clone());
    let selected_profile_data_contracts = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| {
            profile
                .data_contract_labels
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let selected_profile_track_statuses = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| {
            profile
                .track_statuses
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let dataset_resolution_line = build_dataset_resolution_line(provider_status_agent);
    let provider_access_requests = provider_status_agent
        .selected_profile
        .as_ref()
        .map(|profile| profile.install_prompts.clone())
        .unwrap_or_else(|| provider_status_agent.install_prompts.clone());
    AgentBootstrapView {
        symbol: symbol.to_string(),
        project_role: "closed_loop_multi_timeframe_pre_bayes_bbn_engine".to_string(),
        closed_loop_chain: vec![
            "tomac_history -> clean-futures".to_string(),
            "clean-futures -> train/research/backtest/analyze".to_string(),
            "analyze -> pre-bayes-filter -> bridge -> bbn".to_string(),
            "analyze -> pending/execution artifacts".to_string(),
            "artifacts -> update -> learning feedback".to_string(),
        ],
        agent_brief,
        guardrails: vec![
            "do_not_bypass_pre_bayes_evidence_filter".to_string(),
            "do_not_feed_raw_factor_labels_directly_into_bbn".to_string(),
            "treat_factors_as_evidence_not_triggers".to_string(),
            "keep_six_timeframe_resonance_in_train_analyze_bridge_artifact_update".to_string(),
        ],
        detected_paths: AgentBootstrapPaths {
            tomac_history_root: bootstrap_tomac_root,
            multi_timeframe_clean_root: bootstrap_clean_root.clone(),
            state_dir: state_dir.to_string(),
        },
        input_acquisition: AgentBootstrapInputs {
            backtest: AgentBootstrapBacktestInput {
                local_discovery_order: vec![
                    "multi_timeframe_clean_root".to_string(),
                    "tomac_history_root".to_string(),
                    "direct_backtest_file".to_string(),
                ],
                preferred_user_inputs: vec![
                    "multi_timeframe_clean_root".to_string(),
                    "tomac_history_root".to_string(),
                ],
                fallback_user_inputs: vec![
                    "single_backtest_file_path".to_string(),
                    "download_link_to_backtest_file_or_directory".to_string(),
                ],
                should_ask_download_link_if_local_missing: true,
            },
            live: AgentBootstrapLiveInput {
                minimum_required_user_inputs: vec![],
                inferable_defaults: inferable_live_defaults,
                additional_user_inputs_if_not_inferable: vec![
                    "spot_symbol".to_string(),
                    "options_symbol".to_string(),
                    "spot_kind".to_string(),
                    "futures_backend".to_string(),
                    "aux_backend".to_string(),
                    "backend_base_urls_if_non_default".to_string(),
                ],
                selected_profile_id,
                selected_profile_summary,
                selected_profile_data_contracts,
                selected_profile_track_statuses,
                dataset_resolution_line,
                provider_access_requests,
                provider_status_agent: provider_status_agent.clone(),
                provider_status_command: provider_status_command.clone(),
                ibkr_gateway_summary,
                ibkr_gateway_candidates,
            },
        },
        commands: AgentBootstrapCommands {
            clean_multi_timeframe: clean_command,
            train: train_command,
            analyze: analyze_command,
            futures_sop: format!(
                "ict-engine futures-sop --root {} --output-dir {} --interval 15m",
                shell_quote(tomac_root_placeholder),
                shell_quote(
                    &bootstrap_clean_root
                        .clone()
                        .unwrap_or_else(|| "<output-dir>".to_string())
                )
            ),
            expansion_sop: format!(
                "ict-engine expansion-sop --root {} --output-dir {} --interval 15m --lookback 20 --atr-multiplier 1.50",
                shell_quote(tomac_root_placeholder),
                shell_quote(
                    &bootstrap_clean_root
                        .clone()
                        .unwrap_or_else(|| "<output-dir>".to_string())
                )
            ),
            workflow_status: format!(
                "ict-engine workflow-status --symbol {} --state-dir {}{}",
                shell_quote(symbol),
                shell_quote(state_dir),
                provider_status_agent
                    .selected_profile
                    .as_ref()
                    .map(|profile| format!(" --profile {}", shell_quote(&profile.selector)))
                    .unwrap_or_default()
            ),
            provider_status: provider_status_command.clone(),
            recommended_next_command: snapshot.recommended_next_command.clone(),
        },
        latest_snapshot: AgentBootstrapSnapshot {
            current_focus_phase: snapshot.current_focus_phase.clone(),
            current_focus_reason: snapshot.current_focus_reason.clone(),
            blocking_truth: snapshot.blocking_truth.clone(),
            latest_train_phase: snapshot.latest_train.as_ref().map(|phase| phase.phase.clone()),
            latest_analyze_phase: snapshot.latest_analyze.as_ref().map(|phase| phase.phase.clone()),
            latest_pre_bayes_gate_status: snapshot
                .latest_analyze
                .as_ref()
                .map(|phase| phase.pre_bayes_gate_status.clone()),
        },
    }
}

fn selected_profile_tomac_clean_root(
    profile: &crate::application::provider_catalog::ProviderProfileSelectionSurface,
) -> Option<String> {
    profile
        .data_contracts
        .iter()
        .find(|contract| {
            contract.contract_id == "tomac_clean_root"
                && contract.category == "historical"
                && contract.required
        })
        .and_then(|contract| contract.path_hint.clone())
}

fn infer_tomac_root_from_clean_root(clean_root: &str) -> Option<String> {
    let path = std::path::Path::new(clean_root);
    let leaf = path.file_name()?.to_str()?;
    if leaf != "ict-cleaned-mtf" {
        return None;
    }
    path.parent()
        .map(|parent| parent.to_string_lossy().to_string())
}

#[cfg(test)]
fn build_agent_bootstrap_view_with_probe<F>(
    input: AgentBootstrapBuildInput<'_>,
    probe: &F,
) -> AgentBootstrapView
where
    F: Fn(&str, u16) -> bool,
{
    build_agent_bootstrap_view_with_candidates(
        input,
        build_ibkr_gateway_candidates_with_probe("127.0.0.1", probe),
    )
}

fn build_ibkr_gateway_candidates() -> Vec<AgentBootstrapIbkrGatewayCandidate> {
    build_ibkr_gateway_candidates_with_probe("127.0.0.1", &|host, port| {
        let Ok(addr) = format!("{host}:{port}").parse::<SocketAddr>() else {
            return false;
        };
        TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
    })
}

fn build_ibkr_gateway_candidates_with_probe<F>(
    host: &str,
    probe: &F,
) -> Vec<AgentBootstrapIbkrGatewayCandidate>
where
    F: Fn(&str, u16) -> bool,
{
    let recommended_port = IBKR_GATEWAY_PORT_CANDIDATES
        .iter()
        .map(|(_, port)| *port)
        .find(|port| probe(host, *port));
    IBKR_GATEWAY_PORT_CANDIDATES
        .into_iter()
        .map(|(label, port)| AgentBootstrapIbkrGatewayCandidate {
            label: label.to_string(),
            host: host.to_string(),
            port,
            reachable: probe(host, port),
            recommended: recommended_port == Some(port),
        })
        .collect()
}

fn build_ibkr_gateway_summary(
    candidates: &[AgentBootstrapIbkrGatewayCandidate],
) -> AgentBootstrapIbkrGatewaySummary {
    let reachable_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.reachable)
        .count();
    let preferred = candidates.iter().find(|candidate| candidate.recommended);
    let occupied_judgement = match reachable_candidate_count {
        0 => "no_reachable_candidate",
        1 => "single_reachable_candidate",
        _ => "multiple_reachable_candidates_choose_explicit_port",
    }
    .to_string();
    let recommended_action = match reachable_candidate_count {
        0 => "Ask the user to launch TWS or IB Gateway, then rerun setup/status or pass --gateway-port once the local API port is known.".to_string(),
        1 => format!(
            "Use the single reachable local IBKR runtime on port {} unless the user says otherwise.",
            preferred.map(|candidate| candidate.port).unwrap_or_default()
        ),
        _ => format!(
            "Multiple reachable local IBKR runtimes detected; ask the user which one to use and pass --gateway-port {} or the chosen alternative explicitly.",
            preferred.map(|candidate| candidate.port).unwrap_or_default()
        ),
    };
    AgentBootstrapIbkrGatewaySummary {
        preferred_label: preferred.map(|candidate| candidate.label.clone()),
        preferred_port: preferred.map(|candidate| candidate.port),
        reachable_candidate_count,
        occupied_judgement,
        recommended_action,
    }
}

fn print_human_lines(lines: &[String]) {
    for line in lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            println!("{}", redact_local_paths_in_human_text(trimmed));
        }
    }
}

fn short_human_phase_summary(phase: &crate::state::WorkflowPhaseSnapshot) -> String {
    let mut parts = Vec::new();
    if let Some(direction) = &phase.selected_direction {
        parts.push(format!("direction={direction}"));
    }
    if let Some(entry) = &phase.selected_entry_quality {
        parts.push(format!("entry={entry}"));
    }
    if !phase.pre_bayes_gate_status.is_empty()
        && phase.pre_bayes_gate_status != "pre_bayes_gate_unavailable"
    {
        parts.push(format!("gate={}", phase.pre_bayes_gate_status));
    }
    if phase.pre_bayes_evidence_quality_score > 0.0 {
        parts.push(format!(
            "quality={:.3}",
            phase.pre_bayes_evidence_quality_score
        ));
    }
    if let Some(status) = phase
        .pre_bayes_filtered_assignments
        .get("regime_bundle_bbn_application_status")
    {
        if let Some(regime) = phase
            .pre_bayes_filtered_assignments
            .get("regime_bundle_bbn_market_regime")
        {
            parts.push(format!("regime_bundle_bbn={status}:{regime}"));
        }
    }
    if parts.is_empty() {
        compact_human_phase_summary(phase).unwrap_or_else(|| phase.phase_summary.clone())
    } else {
        parts.join(" ")
    }
}

fn compact_human_phase_summary(phase: &crate::state::WorkflowPhaseSnapshot) -> Option<String> {
    let wanted_keys: &[&str] = match phase.phase.as_str() {
        "research" => &[
            "objective",
            "best_factor",
            "aggregate_return",
            "feedback_applied",
            "execution_gate",
        ],
        "backtest" => &[
            "total_return",
            "trade_count",
            "coverage_1sigma",
            "execution_gate",
        ],
        _ => return None,
    };
    let mut selected = Vec::new();
    for token in phase.phase_summary.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            if wanted_keys.contains(&key) {
                let normalized = if key == "best_factor" {
                    value
                        .strip_prefix("Some(\"")
                        .and_then(|item| item.strip_suffix("\")"))
                        .unwrap_or(value)
                        .to_string()
                } else {
                    value.to_string()
                };
                selected.push(format!("{key}={normalized}"));
            }
        }
    }
    if selected.is_empty() {
        None
    } else {
        Some(selected.join(" "))
    }
}

pub fn build_ensemble_vote_surface(
    vote: &EnsembleVoteRecord,
    persisted_scorecards: &[EnsembleExecutorScorecard],
) -> WorkflowEnsembleVoteSurface {
    let (scorecards, scorecard_source) = resolved_vote_scorecards(persisted_scorecards, vote);
    let policy_runtime_line = {
        let items = vote
            .executor_summaries
            .iter()
            .filter_map(|line| {
                line.split_whitespace()
                    .find_map(|part| part.strip_prefix("policy_source="))
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            None
        } else {
            Some(format!("Policy runtime: {}", items.join(", ")))
        }
    };
    WorkflowEnsembleVoteSurface {
        artifact_id: vote.artifact_id.clone(),
        generated_at: vote.generated_at,
        symbol: vote.symbol.clone(),
        source_phase: vote.source_phase.clone(),
        source_run_id: vote.source_run_id.clone(),
        provenance: vote.provenance.clone(),
        dataset_comparability: vote.dataset_comparability.clone(),
        ensemble_version: vote.ensemble_version.clone(),
        final_action: vote.final_action.clone(),
        recommended_command: vote.recommended_command.clone(),
        human_next_triage: vote.human_next_triage.clone(),
        hard_block: vote.hard_block.clone(),
        confidence: vote.confidence,
        consensus_strength: vote.consensus_strength,
        disagreement_flags: vote.disagreement_flags.clone(),
        executor_summaries: vote.executor_summaries.clone(),
        policy_runtime_line,
        split_explanations: vote.split_explanations.clone(),
        executor_scorecards: scorecards,
        executor_scorecard_source: scorecard_source.to_string(),
        posterior_fingerprint: vote.posterior_fingerprint.clone(),
        posterior_normalization_status: vote.posterior_normalization_status.clone(),
        posterior_active_regime: vote.posterior_active_regime.clone(),
        posterior_confidence: vote.posterior_confidence,
        posterior_probabilities: vote.posterior_probabilities.clone(),
        posterior_evidence: vote.posterior_evidence.clone(),
    }
}

fn workflow_status_recommended_path_contract_value(
    recommended_path_bundle: Option<&StructuralRecommendedPathBundleArtifact>,
) -> Option<Value> {
    recommended_path_bundle.map(|bundle| {
        serde_json::json!({
            "candidate_set_id": bundle.candidate_set_id,
            "candidate_set_size": bundle.candidate_set_size,
            "selected_path_probability": bundle.selected_path_probability,
            "path_id": bundle.path_id,
            "path_label": bundle.path_label,
            "trigger": bundle.trigger_summary,
            "confirmation": bundle.confirmation_summary,
            "stop": bundle.stop_summary,
            "invalidation": bundle.invalidation_summary,
            "why": bundle.why_this_path,
            "recommended_command": bundle.recommended_command,
        })
    })
}

fn workflow_status_next_step_with_execution_contract(
    command: &str,
    blocked_reason: Option<&str>,
    execution_contract: Option<Value>,
) -> Value {
    let mut next_step = workflow_next_step_view(command, blocked_reason);
    if let Value::Object(map) = &mut next_step {
        map.insert(
            "execution_contract".to_string(),
            execution_contract.unwrap_or(Value::Null),
        );
    }
    next_step
}

fn workflow_status_value_with_recommended_next_step(
    mut value: Value,
    recommended_next_step: Value,
) -> Value {
    if let Value::Object(map) = &mut value {
        map.insert("recommended_next_step".to_string(), recommended_next_step);
    }
    value
}

fn workflow_status_structural_recommended_next_step_with_state_dir(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    state_dir: Option<&str>,
) -> Value {
    let hard_block_active = matches!(
        snapshot.blocking_truth.status.as_str(),
        "blocked"
            | "bridge_needs_confirmation"
            | "validated_regressing"
            | "credibility_gate_blocked"
    );
    let top_level_command = if hard_block_active {
        snapshot.blocking_truth.next_command.clone()
    } else {
        snapshot.recommended_next_command.clone()
    };
    let selected_data_candidates = historical_data_candidates(snapshot);
    let historical_data_gate_active = !selected_data_candidates.is_empty()
        && (top_level_command.contains("factor-research")
            || top_level_command.contains("factor-backtest")
            || snapshot
                .recommended_next_command
                .contains("factor-research")
            || snapshot
                .recommended_next_command
                .contains("factor-backtest"));
    let provider_support_reason = if hard_block_active && !snapshot.blocking_truth.reason.is_empty()
    {
        Some(snapshot.blocking_truth.reason.as_str())
    } else if !snapshot.current_focus_reason.is_empty() {
        Some(snapshot.current_focus_reason.as_str())
    } else {
        None
    };
    let provider_support = build_workflow_provider_support(
        provider_status_agent,
        &top_level_command,
        provider_support_reason,
    );
    let recommended_path_bundle =
        build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            StructuralPathRankerRuntimeContext { state_dir },
        );
    let recommended_path_contract =
        workflow_status_recommended_path_contract_value(recommended_path_bundle.as_ref());
    workflow_status_next_step_with_execution_contract(
        &top_level_command,
        if hard_block_active || historical_data_gate_active {
            if hard_block_active && !snapshot.blocking_truth.reason.is_empty() {
                Some(snapshot.blocking_truth.reason.as_str())
            } else if historical_data_gate_active {
                Some("user_selected_historical_data_missing")
            } else {
                None
            }
        } else {
            None
        },
        if !hard_block_active && !historical_data_gate_active && !provider_support.active {
            recommended_path_contract
        } else {
            None
        },
    )
}

pub fn build_workflow_status_phase_value(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    phase: &str,
) -> Result<Value> {
    build_workflow_status_phase_value_with_structural_prior_state(
        snapshot,
        persisted_scorecards,
        provider_status_agent,
        feedback_history,
        &StructuralPriorLearningState::default(),
        phase,
    )
}

pub fn build_workflow_status_phase_value_with_structural_prior_state(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    phase: &str,
) -> Result<Value> {
    build_workflow_status_phase_value_with_structural_prior_state_and_state_dir(
        snapshot,
        persisted_scorecards,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        None,
        phase,
    )
}

fn build_workflow_status_phase_value_with_structural_prior_state_and_state_dir(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[crate::state::FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    state_dir: Option<&str>,
    phase: &str,
) -> Result<Value> {
    Ok(match phase.trim().to_ascii_lowercase().as_str() {
        "human" | "human-next" | "human-next-action" => {
            build_human_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                snapshot,
                persisted_scorecards,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            )
        }
        "structural-playbook" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            let recommended_next_step =
                workflow_status_structural_recommended_next_step_with_state_dir(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle)?,
                recommended_next_step,
            )
        }
        "structural-node" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.node)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-branch-set" | "structural-branches" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.branch_set)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-scenario-playbook" | "structural-scenarios" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.scenario_playbook)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-path-plan" | "structural-paths" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.path_plan)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-path-history" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.path_history)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-path-outcome-summary" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.path_history.summary)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-node-history" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.node_history)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-branch-history" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.branch_history)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-scenario-history" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.scenario_history)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-history-summary" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.history_summary)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "structural-temporal-summary" => workflow_status_value_with_recommended_next_step(
            serde_json::to_value(build_structural_temporal_summary_artifact_with_prior_state(
                snapshot,
                provider_status_agent,
                structural_prior_state,
            ))?,
            workflow_status_structural_recommended_next_step_with_state_dir(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            ),
        ),
        "structural-experience-priors" | "structural-experience-prior-surface" => {
            let artifact = build_structural_experience_prior_surface_artifact_with_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
            );
            let recommended_next_step =
                workflow_status_structural_recommended_next_step_with_state_dir(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(artifact)?,
                recommended_next_step,
            )
        }
        "structural-validation" | "structural-validation-summary" => {
            let artifact = build_structural_experience_prior_surface_artifact_with_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
            );
            let recommended_next_step =
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                );
            workflow_status_value_with_recommended_next_step(
                build_structural_validation_summary_value(&artifact),
                recommended_next_step,
            )
        }
        "structural-ranker-runtime" | "structural-path-ranker-runtime" => {
            let bundle =
                build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    StructuralPathRankerRuntimeContext { state_dir },
                );
            let recommended_next_step =
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                );
            workflow_status_value_with_recommended_next_step(
                build_path_ranker_summary_value(bundle.as_ref()),
                recommended_next_step,
            )
        }
        "structural-top-path-candidates" | "structural-top-paths" => {
            let artifact =
                build_structural_top_path_candidates_artifact_with_runtime_context_and_prior_state(
                    snapshot,
                    provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            let recommended_next_step =
                workflow_status_structural_recommended_next_step_with_state_dir(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(artifact)?,
                recommended_next_step,
            )
        }
        "structural-path-ranking-target" | "structural-path-ranking" => {
            let artifact =
                build_structural_path_ranking_target_artifact_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            let recommended_next_step =
                workflow_status_structural_recommended_next_step_with_state_dir(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(artifact)?,
                recommended_next_step,
            )
        }
        "structural-recommended-path-bundle" | "structural-recommended-path" => {
            let bundle =
                build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            let recommended_next_step =
                workflow_status_structural_recommended_next_step_with_state_dir(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                state_dir,
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle)?,
                recommended_next_step,
            )
        }
        "structural-feedback-template" | "structural-feedback" => {
            let bundle = build_structural_playbook_bundle_with_runtime_context_and_prior_state(
                snapshot,
                provider_status_agent,
                feedback_history,
                structural_prior_state,
                StructuralPathRankerRuntimeContext { state_dir },
            );
            workflow_status_value_with_recommended_next_step(
                serde_json::to_value(bundle.feedback_template)?,
                workflow_status_structural_recommended_next_step_with_state_dir(
                    snapshot,
                    provider_status_agent,
                    feedback_history,
                    structural_prior_state,
                    state_dir,
                ),
            )
        }
        "train" => serde_json::to_value(&build_phase_snapshot_surfaces(snapshot).train)?,
        "analyze" => serde_json::to_value(&build_phase_snapshot_surfaces(snapshot).analyze)?,
        "research" => serde_json::to_value(&build_phase_snapshot_surfaces(snapshot).research)?,
        "backtest" => serde_json::to_value(&build_phase_snapshot_surfaces(snapshot).backtest)?,
        "update" => serde_json::to_value(&build_phase_snapshot_surfaces(snapshot).update)?,
        "pre-bayes-policy" => {
            serde_json::to_value(&build_pre_bayes_surfaces(snapshot).pre_bayes_policy)?
        }
        "pre-bayes-policy-history" => {
            serde_json::to_value(&build_pre_bayes_surfaces(snapshot).pre_bayes_policy_history)?
        }
        "pre-bayes-policy-diff" => {
            serde_json::to_value(&build_pre_bayes_surfaces(snapshot).pre_bayes_policy_diff)?
        }
        "pre-bayes-policy-lineage" => {
            serde_json::to_value(&build_pre_bayes_surfaces(snapshot).pre_bayes_policy_lineage)?
        }
        "pre-bayes-entry-quality-bridge" => serde_json::to_value(
            &build_pre_bayes_surfaces(snapshot).pre_bayes_entry_quality_bridge,
        )?,
        "pre-bayes-entry-quality-bridge-diff" => serde_json::to_value(
            &build_pre_bayes_surfaces(snapshot).pre_bayes_entry_quality_bridge_diff,
        )?,
        "pre-bayes-soft-evidence" => {
            serde_json::to_value(&build_pre_bayes_surfaces(snapshot).pre_bayes_soft_evidence)?
        }
        "pre-bayes-soft-evidence-diff" => {
            serde_json::to_value(&build_pre_bayes_surfaces(snapshot).pre_bayes_soft_evidence_diff)?
        }
        "pending-update" => {
            serde_json::to_value(&build_auxiliary_artifact_surfaces(snapshot).pending_update)?
        }
        "pending-update-history" => serde_json::to_value(
            &build_auxiliary_artifact_surfaces(snapshot).pending_update_history,
        )?,
        "execution-candidate" => {
            serde_json::to_value(&build_auxiliary_artifact_surfaces(snapshot).execution_candidate)?
        }
        "execution-candidate-history" => serde_json::to_value(
            &build_auxiliary_artifact_surfaces(snapshot).execution_candidate_history,
        )?,
        "ensemble-vote" => serde_json::to_value(
            resolved_latest_ensemble_vote(snapshot)
                .as_ref()
                .map(|vote| build_ensemble_vote_surface(vote, persisted_scorecards)),
        )?,
        "ensemble-vote-history" => serde_json::to_value(build_ensemble_vote_history_view(
            snapshot,
            persisted_scorecards,
        ))?,
        "ensemble-scorecards" | "ensemble-executor-scorecards" => {
            serde_json::to_value(persisted_scorecards)?
        }
        "artifact-history-summary" => serde_json::to_value(
            &build_auxiliary_artifact_surfaces(snapshot).artifact_history_summary,
        )?,
        "artifact-factor-trends" => serde_json::to_value(
            &build_auxiliary_artifact_surfaces(snapshot).artifact_factor_trends,
        )?,
        "artifact-family-trends" => serde_json::to_value(
            &build_auxiliary_artifact_surfaces(snapshot).artifact_family_trends,
        )?,
        "artifact-consumed-gate" => {
            serde_json::to_value(&build_artifact_report_surfaces(snapshot).artifact_consumed_gate)?
        }
        "artifact-factor-consumed-validation" | "artifact-factor-consumed-leaderboard" => {
            serde_json::to_value(
                &build_artifact_report_surfaces(snapshot).artifact_factor_consumed_validation,
            )?
        }
        "artifact-family-consumed-validation" | "artifact-family-consumed-leaderboard" => {
            serde_json::to_value(
                &build_artifact_report_surfaces(snapshot).artifact_family_consumed_validation,
            )?
        }
        "artifact-lineage-summaries" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_lineage_summaries,
        )?,
        "artifact-decision-summary" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_decision_summary,
        )?,
        "artifact-rule-breaks" => {
            serde_json::to_value(&build_artifact_report_surfaces(snapshot).artifact_rule_breaks)?
        }
        "artifact-rule-break-effects" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_rule_break_effects,
        )?,
        "artifact-factor-rule-break-impacts" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_factor_rule_break_impacts,
        )?,
        "artifact-family-rule-break-impacts" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_family_rule_break_impacts,
        )?,
        "artifact-impact-leaderboard" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_impact_leaderboard,
        )?,
        "artifact-impact-consumed" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_impact_consumed,
        )?,
        "artifact-impact-consumed-trend" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_impact_consumed_trend,
        )?,
        "artifact-review-rules" => {
            serde_json::to_value(&build_artifact_report_surfaces(snapshot).artifact_review_rules)?
        }
        "artifact-review-rule-sources" => serde_json::to_value(
            &build_artifact_report_surfaces(snapshot).artifact_review_rule_sources,
        )?,
        "disagreements" => {
            serde_json::to_value(&build_artifact_report_surfaces(snapshot).disagreements)?
        }
        "diffs" => serde_json::to_value(&build_artifact_report_surfaces(snapshot).diffs)?,
        other => anyhow::bail!("unsupported workflow-status phase '{}'", other),
    })
}

pub fn build_pre_bayes_status_value(
    snapshot: &WorkflowSnapshot,
    section: Option<&str>,
) -> Result<Value> {
    let pre = build_pre_bayes_surfaces(snapshot);
    let latest_phase = latest_pre_bayes_phase(snapshot);
    Ok(
        match section.map(|value| value.trim().to_ascii_lowercase()) {
            None => serde_json::to_value(json!({
                "latest_policy": pre.pre_bayes_policy,
                "latest_bridge": pre.pre_bayes_entry_quality_bridge,
                "latest_bridge_diff": pre.pre_bayes_entry_quality_bridge_diff,
                "latest_policy_diff": pre.pre_bayes_policy_diff,
                "latest_policy_lineage": pre.pre_bayes_policy_lineage,
                "latest_gate_status": latest_phase.map(|phase| phase.pre_bayes_gate_status.clone()),
                "latest_policy_version": latest_phase.map(|phase| phase.pre_bayes_policy_version.clone()),
                "latest_uses_soft_evidence": latest_phase.map(|phase| phase.pre_bayes_uses_soft_evidence),
                "latest_canonical_structural_active_regime": pre.canonical_structural_active_regime,
                "latest_canonical_structural_confidence": pre.canonical_structural_confidence,
                "latest_canonical_structural_probabilities": pre.canonical_structural_probabilities,
                "latest_soft_evidence_diff": pre.pre_bayes_soft_evidence_diff,
                "latest_soft_evidence": latest_phase.map(|phase| phase.pre_bayes_soft_evidence.clone()),
            }))?,
            Some(section) if section == "policy" => serde_json::to_value(&pre.pre_bayes_policy)?,
            Some(section) if section == "bridge" => {
                serde_json::to_value(&pre.pre_bayes_entry_quality_bridge)?
            }
            Some(section) if section == "bridge-diff" => {
                serde_json::to_value(&pre.pre_bayes_entry_quality_bridge_diff)?
            }
            Some(section) if section == "history" => {
                serde_json::to_value(&pre.pre_bayes_policy_history)?
            }
            Some(section) if section == "diff" => serde_json::to_value(&pre.pre_bayes_policy_diff)?,
            Some(section) if section == "lineage" => {
                serde_json::to_value(&pre.pre_bayes_policy_lineage)?
            }
            Some(section) if section == "gate" => serde_json::to_value(json!({
                "status": latest_phase.map(|phase| phase.pre_bayes_gate_status.clone()),
                "policy_version": latest_phase.map(|phase| phase.pre_bayes_policy_version.clone()),
                "uses_soft_evidence": latest_phase.map(|phase| phase.pre_bayes_uses_soft_evidence),
                "canonical_structural_active_regime": pre.canonical_structural_active_regime,
                "canonical_structural_confidence": pre.canonical_structural_confidence,
                "canonical_structural_probabilities": pre.canonical_structural_probabilities,
            }))?,
            Some(section) if section == "soft" || section == "soft-evidence" => {
                serde_json::to_value(
                    latest_phase.map(|phase| phase.pre_bayes_soft_evidence.clone()),
                )?
            }
            Some(section) if section == "soft-diff" => {
                serde_json::to_value(&pre.pre_bayes_soft_evidence_diff)?
            }
            Some(other) => anyhow::bail!("unsupported pre-bayes-status section '{}'", other),
        },
    )
}

pub fn emit_pre_bayes_status_output(
    snapshot: &WorkflowSnapshot,
    section: Option<&str>,
    output_format: &str,
) -> Result<()> {
    let value = build_pre_bayes_status_value(snapshot, section)?;
    match output_format.trim().to_ascii_lowercase().as_str() {
        "json" | "compact" => print_redacted_json(&value),
        "human" => {
            let latest_bridge = value
                .get("latest_bridge")
                .or_else(|| value.get("bridge"))
                .unwrap_or(&value);
            let gate_status = value
                .get("latest_gate_status")
                .or_else(|| value.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("unavailable");
            let policy_version = value
                .get("latest_policy_version")
                .or_else(|| value.get("policy_version"))
                .and_then(Value::as_str)
                .unwrap_or("unavailable");
            let uses_soft = value
                .get("latest_uses_soft_evidence")
                .or_else(|| value.get("uses_soft_evidence"))
                .and_then(Value::as_bool)
                .map(|flag| if flag { "yes" } else { "no" })
                .unwrap_or("unavailable");
            let selected_entry_quality = latest_bridge
                .get("selected_entry_quality")
                .and_then(Value::as_str)
                .unwrap_or("unavailable");
            let long_signal_probability = latest_bridge
                .get("long_signal_probability")
                .and_then(Value::as_f64)
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "unavailable".to_string());
            let short_signal_probability = latest_bridge
                .get("short_signal_probability")
                .and_then(Value::as_f64)
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "unavailable".to_string());
            let multi_timeframe_direction_bias = latest_bridge
                .get("multi_timeframe_direction_bias")
                .and_then(Value::as_str)
                .unwrap_or("unavailable");
            let multi_timeframe_alignment_score = latest_bridge
                .get("multi_timeframe_alignment_score")
                .and_then(Value::as_f64)
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "unavailable".to_string());
            let multi_timeframe_entry_alignment_score = latest_bridge
                .get("multi_timeframe_entry_alignment_score")
                .and_then(Value::as_f64)
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "unavailable".to_string());
            print_human_lines(&[
                format!(
                    "Pre-Bayes | gate={} | policy={} | soft_evidence={}",
                    gate_status, policy_version, uses_soft
                ),
                format!(
                    "Bridge: entry={} | long={} | short={} | mtf={} | align={} | entry_align={}",
                    selected_entry_quality,
                    long_signal_probability,
                    short_signal_probability,
                    multi_timeframe_direction_bias,
                    multi_timeframe_alignment_score,
                    multi_timeframe_entry_alignment_score
                ),
            ]);
            Ok(())
        }
        other => anyhow::bail!("unsupported pre-bayes-status output format '{}'", other),
    }
}

pub fn build_pre_bayes_diff_value(snapshot: &WorkflowSnapshot) -> Value {
    let latest_phase = latest_pre_bayes_phase(snapshot);
    json!({
        "latest_policy_diff": snapshot.latest_pre_bayes_policy_diff,
        "latest_policy_lineage": snapshot.latest_pre_bayes_policy_lineage,
        "latest_gate_status": latest_phase.map(|phase| phase.pre_bayes_gate_status.clone()),
        "latest_policy_version": latest_phase.map(|phase| phase.pre_bayes_policy_version.clone()),
        "latest_uses_soft_evidence": latest_phase.map(|phase| phase.pre_bayes_uses_soft_evidence),
        "latest_canonical_structural_active_regime": latest_phase.and_then(|phase| phase.canonical_structural_active_regime.clone()),
        "latest_canonical_structural_confidence": latest_phase.and_then(|phase| phase.canonical_structural_confidence),
        "latest_canonical_structural_probabilities": latest_phase.map(|phase| phase.canonical_structural_probabilities.clone()),
        "latest_soft_evidence_diff": snapshot.latest_pre_bayes_soft_evidence_diff,
        "latest_bridge": snapshot.latest_pre_bayes_entry_quality_bridge,
        "latest_bridge_diff": snapshot.latest_pre_bayes_entry_quality_bridge_diff,
    })
}

pub fn emit_pre_bayes_diff_output(snapshot: &WorkflowSnapshot) -> Result<()> {
    let value = build_pre_bayes_diff_value(snapshot);
    print_redacted_json(&value)
}

pub fn build_workflow_status_bootstrap_phase_value(
    symbol: &str,
    state_dir: &str,
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    detected_tomac_root: Option<String>,
    multi_timeframe_clean_root: Option<String>,
    tomac_root_placeholder: &str,
) -> Result<Value> {
    Ok(serde_json::to_value(build_agent_bootstrap_view(
        AgentBootstrapBuildInput {
            symbol,
            state_dir,
            snapshot,
            provider_status_agent,
            detected_tomac_root,
            multi_timeframe_clean_root,
            tomac_root_placeholder,
        },
    ))?)
}

#[cfg(test)]
fn build_workflow_status_bootstrap_phase_value_with_probe<F>(
    bootstrap: WorkflowStatusBootstrapInput<'_>,
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    probe: &F,
) -> Result<Value>
where
    F: Fn(&str, u16) -> bool,
{
    Ok(serde_json::to_value(
        build_agent_bootstrap_view_with_probe(
            AgentBootstrapBuildInput {
                symbol: bootstrap.symbol,
                state_dir: bootstrap.state_dir,
                snapshot,
                provider_status_agent,
                detected_tomac_root: bootstrap.detected_tomac_root,
                multi_timeframe_clean_root: bootstrap.multi_timeframe_clean_root,
                tomac_root_placeholder: &bootstrap.tomac_root_placeholder,
            },
            probe,
        ),
    )?)
}

pub fn build_ensemble_vote_history_view(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
) -> WorkflowEnsembleVoteHistoryView {
    let history = snapshot
        .recent_ensemble_votes
        .iter()
        .map(|vote| {
            let vote =
                resolved_ensemble_vote_for_snapshot(snapshot, vote).unwrap_or_else(|| vote.clone());
            let surface = build_ensemble_vote_surface(&vote, persisted_scorecards);
            WorkflowEnsembleVoteHistoryRow {
                artifact_id: surface.artifact_id,
                generated_at: surface.generated_at,
                symbol: surface.symbol,
                source_phase: surface.source_phase,
                source_run_id: surface.source_run_id,
                final_action: surface.final_action,
                recommended_command: surface.recommended_command,
                human_next_triage: surface.human_next_triage,
                hard_block: surface.hard_block,
                policy_runtime_line: surface.policy_runtime_line,
                executor_scorecards: surface.executor_scorecards,
                executor_scorecard_source: surface.executor_scorecard_source,
            }
        })
        .collect::<Vec<_>>();

    let hard_block_only = history
        .iter()
        .filter(|row| row.hard_block.active)
        .cloned()
        .collect::<Vec<_>>();

    let mut reason_counts = std::collections::BTreeMap::<String, usize>::new();
    for row in &hard_block_only {
        let reason = row
            .hard_block
            .reason
            .clone()
            .unwrap_or_else(|| "hard_block_reason_unavailable".to_string());
        *reason_counts.entry(reason).or_insert(0) += 1;
    }

    let reason_leaderboard = reason_counts
        .into_iter()
        .map(|(reason, count)| WorkflowHardBlockReasonCount { reason, count })
        .collect::<Vec<_>>();

    WorkflowEnsembleVoteHistoryView {
        hard_block_summary: WorkflowHardBlockSummary {
            count: hard_block_only.len(),
            reason_leaderboard,
        },
        history,
        hard_block_only,
    }
}

pub fn filter_hard_block_rows(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    hard_block_only: bool,
    hard_block_reason: Option<&str>,
    limit: Option<usize>,
) -> Vec<WorkflowEnsembleVoteHistoryRow> {
    build_ensemble_vote_history_view(snapshot, persisted_scorecards)
        .history
        .into_iter()
        .filter(|row| !hard_block_only || row.hard_block.active)
        .filter(|row| {
            hard_block_reason.is_none_or(|reason| row.hard_block.reason.as_deref() == Some(reason))
        })
        .take(limit.unwrap_or(usize::MAX))
        .collect()
}

pub fn build_auxiliary_artifact_surfaces(
    snapshot: &WorkflowSnapshot,
) -> WorkflowAuxiliaryArtifactSurfaces {
    WorkflowAuxiliaryArtifactSurfaces {
        pending_update: snapshot.latest_pending_update.clone(),
        pending_update_history: snapshot.recent_pending_updates.clone(),
        execution_candidate: snapshot.latest_execution_candidate.clone(),
        execution_candidate_history: snapshot.recent_execution_candidates.clone(),
        artifact_history_summary: snapshot.artifact_history_summary.clone(),
        artifact_factor_trends: snapshot.artifact_factor_trends.clone(),
        artifact_family_trends: snapshot.artifact_family_trends.clone(),
    }
}

pub fn sorted_artifact_factor_consumed_validation(
    snapshot: &WorkflowSnapshot,
) -> Vec<ArtifactFactorTrendSummary> {
    let mut items = snapshot.artifact_factor_trends.clone();
    items.sort_by(|a, b| {
        b.consumed_entries
            .cmp(&a.consumed_entries)
            .then_with(|| b.entries.cmp(&a.entries))
            .then_with(|| a.factor_name.cmp(&b.factor_name))
    });
    items
}

pub fn sorted_artifact_family_consumed_validation(
    snapshot: &WorkflowSnapshot,
) -> Vec<ArtifactFamilyTrendSummary> {
    let mut items = snapshot.artifact_family_trends.clone();
    items.sort_by(|a, b| {
        b.consumed_entries
            .cmp(&a.consumed_entries)
            .then_with(|| b.entries.cmp(&a.entries))
            .then_with(|| a.family.cmp(&b.family))
    });
    items
}

pub fn build_pre_bayes_surfaces(snapshot: &WorkflowSnapshot) -> WorkflowPreBayesSurfaces {
    let latest_phase = latest_pre_bayes_phase(snapshot);
    WorkflowPreBayesSurfaces {
        pre_bayes_policy: snapshot.latest_pre_bayes_policy.clone(),
        pre_bayes_policy_history: snapshot.recent_pre_bayes_policies.clone(),
        pre_bayes_policy_diff: snapshot.latest_pre_bayes_policy_diff.clone(),
        pre_bayes_policy_lineage: snapshot.latest_pre_bayes_policy_lineage.clone(),
        pre_bayes_entry_quality_bridge: snapshot.latest_pre_bayes_entry_quality_bridge.clone(),
        pre_bayes_entry_quality_bridge_diff: snapshot
            .latest_pre_bayes_entry_quality_bridge_diff
            .clone(),
        canonical_structural_active_regime: latest_phase
            .and_then(|phase| phase.canonical_structural_active_regime.clone()),
        canonical_structural_confidence: latest_phase
            .and_then(|phase| phase.canonical_structural_confidence),
        canonical_structural_probabilities: latest_phase
            .map(|phase| phase.canonical_structural_probabilities.clone())
            .unwrap_or_default(),
        pre_bayes_soft_evidence: latest_phase.map(|phase| phase.pre_bayes_soft_evidence.clone()),
        pre_bayes_soft_evidence_diff: snapshot.latest_pre_bayes_soft_evidence_diff.clone(),
    }
}

fn latest_pre_bayes_phase(
    snapshot: &WorkflowSnapshot,
) -> Option<&crate::state::WorkflowPhaseSnapshot> {
    [
        snapshot.latest_update.as_ref(),
        snapshot.latest_research.as_ref(),
        snapshot.latest_analyze.as_ref(),
        snapshot.latest_backtest.as_ref(),
    ]
    .into_iter()
    .flatten()
    .filter(|phase| {
        !phase.pre_bayes_gate_status.is_empty()
            || !phase.pre_bayes_policy_version.is_empty()
            || phase.pre_bayes_uses_soft_evidence
            || !phase.pre_bayes_soft_evidence.is_empty()
            || phase.canonical_structural_active_regime.is_some()
            || phase.canonical_structural_confidence.is_some()
            || !phase.canonical_structural_probabilities.is_empty()
    })
    .max_by(|left, right| left.timestamp.cmp(&right.timestamp))
}

pub fn build_artifact_report_surfaces(
    snapshot: &WorkflowSnapshot,
) -> WorkflowArtifactReportSurfaces {
    WorkflowArtifactReportSurfaces {
        artifact_consumed_gate: serde_json::json!({
            "status": snapshot.artifact_decision_summary.consumed_trend_status,
            "reason": snapshot.artifact_decision_summary.consumed_trend_reason,
            "target_kinds": snapshot.artifact_decision_summary.consumed_target_kinds,
            "promotion_strength": snapshot.artifact_decision_summary.promotion_strength,
            "rollback_strength": snapshot.artifact_decision_summary.rollback_strength,
        }),
        artifact_factor_consumed_validation: sorted_artifact_factor_consumed_validation(snapshot),
        artifact_family_consumed_validation: sorted_artifact_family_consumed_validation(snapshot),
        artifact_lineage_summaries: snapshot.artifact_lineage_summaries.clone(),
        artifact_decision_summary: snapshot.artifact_decision_summary.clone(),
        artifact_rule_breaks: snapshot
            .artifact_lineage_summaries
            .iter()
            .filter(|summary| summary.review_rule_break_count > 0)
            .cloned()
            .collect(),
        artifact_rule_break_effects: snapshot.artifact_rule_break_effects.clone(),
        artifact_factor_rule_break_impacts: snapshot.artifact_factor_rule_break_impacts.clone(),
        artifact_family_rule_break_impacts: snapshot.artifact_family_rule_break_impacts.clone(),
        artifact_impact_leaderboard: serde_json::json!({
            "factor": snapshot.artifact_factor_rule_break_impacts,
            "family": snapshot.artifact_family_rule_break_impacts,
        }),
        artifact_impact_consumed: serde_json::json!({
            "factor": snapshot
                .artifact_factor_rule_break_impacts
                .iter()
                .filter(|impact| impact.consumed_breaks > 0)
                .cloned()
                .collect::<Vec<_>>(),
            "family": snapshot
                .artifact_family_rule_break_impacts
                .iter()
                .filter(|impact| impact.consumed_breaks > 0)
                .cloned()
                .collect::<Vec<_>>(),
        }),
        artifact_impact_consumed_trend: snapshot.artifact_consumed_impact_summary.clone(),
        artifact_review_rules: snapshot.artifact_review_rules.clone(),
        artifact_review_rule_sources: snapshot.artifact_review_rule_sources.clone(),
        disagreements: snapshot.disagreements.clone(),
        diffs: snapshot.field_diffs.clone(),
    }
}

pub fn sample_human_workflow_snapshot() -> WorkflowSnapshot {
    let mut snapshot = WorkflowSnapshot::default();
    snapshot.symbol = "NQ".to_string();
    snapshot.current_focus_phase = "update".to_string();
    snapshot.current_focus_reason = "waiting_for_user_data_choice".to_string();
    snapshot.blocking_truth = crate::state::WorkflowBlockingTruth {
        stage: "research".to_string(),
        status: "blocked".to_string(),
        reason: "user_selected_historical_data_missing".to_string(),
        evidence: vec!["need user choice".to_string()],
        next_command: "ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state".to_string(),
    };
    snapshot.recommended_next_command = "ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state".to_string();
    snapshot.pending_actions = vec!["research:choose data".to_string()];
    snapshot.risk_flags = vec!["human_gate_active".to_string()];
    snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
        artifact_id: "ensemble-vote:update:test".to_string(),
        generated_at: Utc::now(),
        symbol: "NQ".to_string(),
        source_phase: "update".to_string(),
        source_run_id: Some("run-1".to_string()),
        provenance: RunProvenance::default(),
        dataset_comparability: DatasetComparability::default(),
        ensemble_version: "ensemble-audit-v1".to_string(),
        final_action: "observe".to_string(),
        recommended_command: "ict-engine workflow-status --symbol NQ --phase human-next".to_string(),
        human_next_triage: "hard_blocked=true ensemble_action=observe consensus=0.500 regime=research hard_block_reason=user_selected_historical_data_missing command=ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state".to_string(),
        hard_block: super::EnsembleHardBlockArtifact {
            active: true,
            stage: Some("research".to_string()),
            status: Some("blocked".to_string()),
            reason: Some("user_selected_historical_data_missing".to_string()),
            evidence: vec!["need user choice".to_string()],
            command: Some("ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state".to_string()),
            human_action: Some("Ask the user to choose the historical dataset. Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json Then run: ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state".to_string()),
        },
        confidence: 0.5,
        consensus_strength: 0.5,
        disagreement_flags: Vec::new(),
        executor_summaries: vec![
            "executor=catboost_stub action=observe confidence=0.500".to_string(),
            "jump_model active_state=jump_transition confidence=0.500 transition_risk=0.500"
                .to_string(),
            "jump_calibration_gate outcome=accepted sample_count=4 cooldown_status=ready"
                .to_string(),
            "jump_disagreement=jump_transition_vs_hmm_only".to_string(),
        ],
        split_explanations: vec!["active_regime=research".to_string()],
        executor_scorecards: vec![EnsembleExecutorScorecard {
            executor: "catboost_stub".to_string(),
            latest_weight_hint: Some(0.55),
            ..EnsembleExecutorScorecard::default()
        }],
        executor_scorecards_source: Some("fallback".to_string()),
        posterior_fingerprint: "fp-test".to_string(),
        posterior_normalization_status: "normalized".to_string(),
        posterior_active_regime: "research".to_string(),
        posterior_confidence: Some(0.5),
        posterior_probabilities: std::collections::BTreeMap::new(),
        posterior_evidence: vec!["mtf=test".to_string()],
    });
    snapshot.latest_update = Some(crate::state::WorkflowPhaseSnapshot {
        phase: "update".to_string(),
        workflow_reason: "waiting_for_data_choice".to_string(),
        phase_summary: "latest update complete".to_string(),
        top_actions: vec!["update:review".to_string()],
        risk_flags: vec!["human_gate_active".to_string()],
        multi_timeframe_summary: vec![
            "15m:80 bars path=/tmp/a.json".to_string(),
            "1h:80 bars path=/tmp/b.json".to_string(),
        ],
        pre_bayes_gate_status: "pass_neutralized".to_string(),
        pre_bayes_uses_soft_evidence: true,
        pre_bayes_policy_version: "v1".to_string(),
        pre_bayes_evidence_quality_score: 0.5,
        pre_bayes_multi_timeframe_direction_bias: "bullish".to_string(),
        pre_bayes_multi_timeframe_alignment_score: Some(0.8),
        pre_bayes_multi_timeframe_entry_alignment_score: Some(0.8),
        selected_entry_quality: Some("medium".to_string()),
        pre_bayes_bridge_selected_entry_quality: Some("medium".to_string()),
        pre_bayes_bridge_probability_gap: Some(0.01),
        hybrid_duration_model: Some("negative_binomial".to_string()),
        hybrid_remaining_expected_bars: Some(2.5),
        comparable_to_previous: true,
        comparison_class: "same_data_different_config".to_string(),
        recommended_next_command: snapshot.recommended_next_command.clone(),
        pda_cluster_label: Some("cluster_1".to_string()),
        realized_outcome: Some("win".to_string()),
        objective_market_credibility_shrink: None,
        ..crate::state::WorkflowPhaseSnapshot::default()
    });
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auto_quant::handoff::{
        build_factor_research_handoff_payload, BuildFactorResearchHandoffPayloadInput,
    };
    use crate::application::auto_quant::AutoQuantDependencyStatus;
    use crate::application::orchestration::EnsembleHardBlockArtifact;
    use crate::application::provider_catalog::{
        ProviderCatalogAgentSurface, ProviderCatalogPendingAgentItem,
    };
    use chrono::TimeZone;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn short_human_phase_summary_includes_applied_regime_bundle_bbn() {
        let summary = short_human_phase_summary(&crate::state::WorkflowPhaseSnapshot {
            selected_direction: Some("Bull".to_string()),
            selected_entry_quality: Some("medium".to_string()),
            pre_bayes_gate_status: "pass_hard".to_string(),
            pre_bayes_evidence_quality_score: 0.62,
            pre_bayes_filtered_assignments: std::collections::BTreeMap::from([
                (
                    "regime_bundle_bbn_application_status".to_string(),
                    "applied".to_string(),
                ),
                (
                    "regime_bundle_bbn_market_regime".to_string(),
                    "bull".to_string(),
                ),
            ]),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });

        assert!(summary.contains("regime_bundle_bbn=applied:bull"));
    }

    fn sample_provider_agent_surface() -> ProviderCatalogAgentSurface {
        ProviderCatalogAgentSurface {
            summary_line: "live_runtime:1/3 ready".to_string(),
            ready_by_domain: std::collections::BTreeMap::from([(
                "live_runtime".to_string(),
                "1/3".to_string(),
            )]),
            providers: vec![
                ProviderCatalogAgentItem {
                    provider_id: "yfinance".to_string(),
                    domain: "live_runtime".to_string(),
                    selectable_by_user: true,
                    adopted_by_default: true,
                    ready: true,
                    access_mode: "local_library".to_string(),
                    user_access: "zero_config_local".to_string(),
                    market_fit: vec!["tradfi".to_string(), "crypto".to_string()],
                    fallback_priority: Some(1),
                    status: "ready".to_string(),
                    reason: "native_yfinance_backend_available".to_string(),
                    summary: "Zero-config live runtime.".to_string(),
                    install_prompts: Vec::new(),
                },
                ProviderCatalogAgentItem {
                    provider_id: "external_http_runtime".to_string(),
                    domain: "live_runtime".to_string(),
                    selectable_by_user: true,
                    adopted_by_default: false,
                    ready: false,
                    access_mode: "external_http_runtime".to_string(),
                    user_access: "operator_runtime_optional".to_string(),
                    market_fit: vec!["tradfi".to_string(), "crypto".to_string()],
                    fallback_priority: Some(20),
                    status: "operator_runtime_required".to_string(),
                    reason: "base_url_and_service_required".to_string(),
                    summary: "Optional external HTTP runtime.".to_string(),
                    install_prompts: vec![
                        "Ask whether the user wants zero-config yfinance or external_http_runtime.".to_string(),
                    ],
                },
                ProviderCatalogAgentItem {
                    provider_id: "crypto_public_runtime".to_string(),
                    domain: "live_runtime".to_string(),
                    selectable_by_user: true,
                    adopted_by_default: false,
                    ready: false,
                    access_mode: "external_http_runtime".to_string(),
                    user_access: "operator_runtime_optional".to_string(),
                    market_fit: vec!["tradfi".to_string(), "crypto".to_string()],
                    fallback_priority: Some(21),
                    status: "operator_runtime_required".to_string(),
                    reason: "base_url_and_service_required".to_string(),
                    summary: "Optional crypto-public runtime.".to_string(),
                    install_prompts: vec![
                        "Ask whether the user wants zero-config yfinance or crypto_public_runtime.".to_string(),
                    ],
                },
            ],
            ready_providers: vec!["yfinance".to_string()],
            pending_providers: vec![
                "external_http_runtime@live_runtime:operator_runtime_required:base_url_and_service_required"
                    .to_string(),
                "crypto_public_runtime@live_runtime:operator_runtime_required:base_url_and_service_required"
                    .to_string(),
            ],
            pending_provider_details: vec![
                ProviderCatalogPendingAgentItem {
                    provider_id: "external_http_runtime".to_string(),
                    domain: "live_runtime".to_string(),
                    status: "operator_runtime_required".to_string(),
                    reason: "base_url_and_service_required".to_string(),
                    install_prompts: vec![
                        "Ask whether the user wants zero-config yfinance or external_http_runtime.".to_string(),
                    ],
                },
                ProviderCatalogPendingAgentItem {
                    provider_id: "crypto_public_runtime".to_string(),
                    domain: "live_runtime".to_string(),
                    status: "operator_runtime_required".to_string(),
                    reason: "base_url_and_service_required".to_string(),
                    install_prompts: vec![
                        "Ask whether the user wants zero-config yfinance or crypto_public_runtime.".to_string()
                    ],
                },
            ],
            selectable_providers: vec!["external_http_runtime".to_string(), "crypto_public_runtime".to_string()],
            default_enabled_providers: vec!["yfinance".to_string()],
            install_prompts: vec![
                "Ask whether the user wants zero-config yfinance or external_http_runtime.".to_string(),
                "Ask whether the user wants zero-config yfinance or crypto_public_runtime.".to_string(),
                "Ask the user for a TradingViewRemix MCP API key before attempting TradingViewRemix-backed live or options workflows. Search keywords: TradingViewRemix MCP API key.".to_string(),
                "Ask the user to install IBKR TWS or IB Gateway and enable the local API before attempting IBKR-backed live workflows. Search keywords: Interactive Brokers TWS download, IB Gateway download.".to_string(),
            ],
            available_opt_in_profiles: vec![
                crate::application::provider_catalog::ProviderProfileReferenceSurface {
                    profile_id: "thrill3r_nq_closed_loop_v1".to_string(),
                    display_name: "Thrill3r NQ Closed Loop v1".to_string(),
                    selector: "thrill3r-nq-closed-loop-v1".to_string(),
                    opt_in_only: true,
                    summary: "Personal NQ workflow".to_string(),
                },
            ],
            selected_profile: None,
            selected_profile_full: None,
        }
    }

    fn serve_http_response(path: &str, body: String, request_count: usize) -> String {
        serve_http_response_with_method(path, body, request_count, "GET")
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
        let response_path = expected_path.clone();
        let expected_method = method.to_string();
        thread::spawn(move || {
            for _ in 0..request_count {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buffer = [0_u8; 2048];
                    let read = stream.read(&mut buffer).unwrap_or_default();
                    let request = String::from_utf8_lossy(&buffer[..read]);
                    assert!(request.starts_with(&format!("{expected_method} ")));
                    assert!(request.contains(&expected_path));
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/jsonl\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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

    fn sample_provider_agent_surface_with_profile() -> ProviderCatalogAgentSurface {
        let mut surface = sample_provider_agent_surface();
        let full = crate::application::provider_catalog::ProviderProfileSelectionSurface {
            profile_id: "thrill3r_nq_closed_loop_v1".to_string(),
            display_name: "Thrill3r NQ Closed Loop v1".to_string(),
            opt_in_only: true,
            source: "repo-example".to_string(),
            selector: "thrill3r-nq-closed-loop-v1".to_string(),
            summary: "Personal NQ workflow".to_string(),
            data_contracts: vec![
                crate::application::provider_catalog::ProviderProfileDataContract {
                    contract_id: "tomac_clean_root".to_string(),
                    category: "historical".to_string(),
                    required: true,
                    label: "Tomac cleaned multi-timeframe futures root".to_string(),
                    symbols: vec!["NQ".to_string()],
                    timeframes: vec!["1d".to_string(), "1h".to_string(), "15m".to_string()],
                    path_hint: Some("/tmp/tomac/ict-cleaned-mtf".to_string()),
                    notes: Vec::new(),
                },
                crate::application::provider_catalog::ProviderProfileDataContract {
                    contract_id: "qqq_options_surface".to_string(),
                    category: "options".to_string(),
                    required: true,
                    label: "QQQ options Greeks / IV / OI".to_string(),
                    symbols: vec!["QQQ".to_string()],
                    timeframes: vec!["snapshot".to_string()],
                    path_hint: None,
                    notes: Vec::new(),
                },
            ],
            data_contract_labels: vec![
                "Tomac cleaned multi-timeframe futures root".to_string(),
                "QQQ options Greeks / IV / OI".to_string(),
            ],
            track_details: vec![
                crate::application::provider_catalog::ProviderProfileTrackSelection {
                    track_id: "research_zero_config".to_string(),
                    label: "Zero-config research companion data".to_string(),
                    required: true,
                    mode: "any_of".to_string(),
                    activation_hints: vec!["research".to_string(), "backtest".to_string()],
                    status: "ready".to_string(),
                    ready_provider_ids: vec!["yfinance".to_string()],
                    pending_provider_ids: Vec::new(),
                    install_prompts: Vec::new(),
                    notes: Vec::new(),
                },
                crate::application::provider_catalog::ProviderProfileTrackSelection {
                    track_id: "options_enriched".to_string(),
                    label: "Options enrichment".to_string(),
                    required: true,
                    mode: "any_of".to_string(),
                    activation_hints: vec!["research".to_string(), "options".to_string()],
                    status: "pending".to_string(),
                    ready_provider_ids: Vec::new(),
                    pending_provider_ids: vec!["tradingview_mcp".to_string()],
                    install_prompts: vec!["Ask for TradingViewRemix MCP API key.".to_string()],
                    notes: Vec::new(),
                },
            ],
            track_statuses: vec![
                "research_zero_config:ready:yfinance,yfinance".to_string(),
                "options_enriched:pending:tradingview_mcp".to_string(),
            ],
            ready_provider_ids: vec!["yfinance".to_string()],
            pending_provider_ids: vec!["tradingview_mcp".to_string()],
            install_prompts: vec!["Ask for TradingViewRemix MCP API key.".to_string()],
        };
        surface.selected_profile = Some(
            crate::application::provider_catalog::ProviderProfileAgentSelectionSurface {
                profile_id: full.profile_id.clone(),
                display_name: full.display_name.clone(),
                opt_in_only: full.opt_in_only,
                source_kind: "repo-example".to_string(),
                selector: full.selector.clone(),
                summary: full.summary.clone(),
                data_contract_labels: full.data_contract_labels.clone(),
                track_statuses: full.track_statuses.clone(),
                ready_provider_ids: full.ready_provider_ids.clone(),
                pending_provider_ids: full.pending_provider_ids.clone(),
                install_prompts: full.install_prompts.clone(),
            },
        );
        surface.selected_profile_full = Some(full);
        surface
    }

    fn sample_auto_quant_dependency_status(managed_dir: String) -> AutoQuantDependencyStatus {
        AutoQuantDependencyStatus {
            repo_url: "repo".to_string(),
            managed_dir,
            tracked_branch: "master".to_string(),
            pinned_ref: None,
            current_commit: None,
            upstream_commit: None,
            bootstrap_needed: false,
            config_present: true,
            managed_repo_present: true,
            healthy: true,
            update_available: false,
            required_files: Vec::new(),
            notes: Vec::new(),
            adapter_version: "v1".to_string(),
            last_sync: None,
        }
    }

    fn persist_sample_auto_quant_handoff(
        temp: &tempfile::TempDir,
    ) -> crate::state::ArtifactLedgerEntry {
        let mut payload =
            build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
                symbol: "DEMO",
                data: "examples/demo/demo-15m.json",
                objective: "expansion_manipulation",
                provider_profile_selector: None,
                paired_data: None,
                auxiliary_evidence_path: None,
                mutation_spec_path: None,
                strategy_material_root: None,
                state_dir: temp.path().to_str().unwrap(),
                dependency_status: sample_auto_quant_dependency_status(
                    temp.path()
                        .join(".deps/auto-quant")
                        .to_string_lossy()
                        .into_owned(),
                ),
            });
        let filename = format!("auto_quant_handoff.{}.json", payload.handoff_kind);
        crate::state::save_state(
            temp.path().to_str().unwrap(),
            &payload.symbol,
            &filename,
            &payload,
        )
        .unwrap();
        let path = crate::state::artifact_state_path(
            temp.path().to_str().unwrap(),
            &payload.symbol,
            &filename,
        );
        payload.handoff_artifact_path = path.clone();
        crate::state::save_state(
            temp.path().to_str().unwrap(),
            &payload.symbol,
            &filename,
            &payload,
        )
        .unwrap();
        crate::state::append_artifact_ledger_entry(
            temp.path().to_str().unwrap(),
            &payload.symbol,
            crate::state::ArtifactLedgerEntry {
                entry_id: format!("ledger:{}", payload.artifact_id),
                artifact_kind: "auto_quant_handoff_candidate".to_string(),
                artifact_id: payload.artifact_id.clone(),
                version: 1,
                generated_at: chrono::Utc::now(),
                symbol: payload.symbol.clone(),
                source_phase: payload.handoff_kind.clone(),
                source_run_id: payload.session_id.clone(),
                path: path.clone(),
                status: if payload.data_ready {
                    "ready_for_external_run".to_string()
                } else {
                    "prepare_required".to_string()
                },
                promote_candidate: false,
                actionable: true,
                decision_hint: payload.backend.clone(),
                review_reason: payload.suggested_next_steps.join(" | "),
                review_rule_version: "auto-quant-handoff-v1".to_string(),
                top_factor_name: None,
                top_factor_action: Some("review".to_string()),
                family_scores: std::collections::BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: if payload.data_ready { 70 } else { 30 },
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();
        let ledger =
            crate::state::load_state_or_default::<Vec<crate::state::ArtifactLedgerEntry>, _>(
                temp.path().to_str().unwrap(),
                "DEMO",
                crate::state::ARTIFACT_LEDGER_FILE,
            )
            .unwrap()
            .into_iter()
            .find(|entry| entry.path == path)
            .unwrap();
        ledger
    }

    fn sample_structural_feedback_history() -> Vec<crate::state::FeedbackRecord> {
        let timestamp = Utc::with_ymd_and_hms(&Utc, 2026, 4, 29, 0, 0, 0).unwrap();
        vec![
            crate::state::FeedbackRecord {
                timestamp,
                symbol: "NQ".to_string(),
                source: "structural_feedback_submission".to_string(),
                run_id: Some("run-1".to_string()),
                trade_id: None,
                prompt_version: Some("structural-feedback-v1".to_string()),
                factor_version: None,
                data_fingerprint: None,
                factors_used: Vec::new(),
                model_probabilities_before_trade: crate::state::ModelProbabilitySnapshot {
                    selected_direction: crate::types::Direction::Bull,
                    selected_probability: 0.72,
                    long_score: 0.72,
                    short_score: 0.28,
                    win_prob_long: 0.72,
                    win_prob_short: 0.28,
                    uncertainty: 0.28,
                },
                realized_outcome: "win".to_string(),
                pnl: 0.03,
                regime_at_entry: crate::types::Regime::ManipulationExpansion,
                structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                    protocol_version: "structural-feedback-v1".to_string(),
                    recommendation_id: "structural-feedback:NQ:node:path".to_string(),
                    recommended_at: "2026-04-29T00:00:00Z".to_string(),
                    node_id: "NQ:belief_regime_node:trend".to_string(),
                    branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                    scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                        .to_string(),
                    path_id:
                        "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                            .to_string(),
                    followed_path: true,
                    exit_reason: Some("target_hit".to_string()),
                    notes: None,
                }),
                reflection_mismatch_tags: Vec::new(),
            },
            crate::state::FeedbackRecord {
                timestamp: timestamp + chrono::Duration::minutes(5),
                symbol: "NQ".to_string(),
                source: "structural_feedback_submission".to_string(),
                run_id: Some("run-2".to_string()),
                trade_id: None,
                prompt_version: Some("structural-feedback-v1".to_string()),
                factor_version: None,
                data_fingerprint: None,
                factors_used: Vec::new(),
                model_probabilities_before_trade: crate::state::ModelProbabilitySnapshot {
                    selected_direction: crate::types::Direction::Bull,
                    selected_probability: 0.70,
                    long_score: 0.70,
                    short_score: 0.30,
                    win_prob_long: 0.70,
                    win_prob_short: 0.30,
                    uncertainty: 0.30,
                },
                realized_outcome: "invalidated".to_string(),
                pnl: -0.01,
                regime_at_entry: crate::types::Regime::ManipulationExpansion,
                structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                    protocol_version: "structural-feedback-v1".to_string(),
                    recommendation_id: "structural-feedback:NQ:node:path-2".to_string(),
                    recommended_at: "2026-04-29T00:05:00Z".to_string(),
                    node_id: "NQ:belief_regime_node:trend".to_string(),
                    branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                    scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                        .to_string(),
                    path_id:
                        "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                            .to_string(),
                    followed_path: true,
                    exit_reason: Some("invalidated".to_string()),
                    notes: None,
                }),
                reflection_mismatch_tags: Vec::new(),
            },
        ]
    }
    use crate::application::output_foundation::redact_local_paths_in_value;
    use crate::state::WorkflowPhaseSnapshot;

    #[test]
    fn build_pre_bayes_status_value_matches_main_policy_section() {
        let snapshot = WorkflowSnapshot {
            latest_pre_bayes_policy: Some(PreBayesEvidencePolicy {
                version: "v-policy".to_string(),
                ..PreBayesEvidencePolicy::default()
            }),
            ..WorkflowSnapshot::default()
        };
        let value = build_pre_bayes_status_value(&snapshot, Some("policy")).unwrap();
        assert_eq!(value["version"], "v-policy");
    }

    #[test]
    fn build_pre_bayes_status_value_default_includes_gate_and_soft_evidence() {
        let analyze = WorkflowPhaseSnapshot {
            pre_bayes_gate_status: "pass_neutralized".to_string(),
            pre_bayes_policy_version: "v2".to_string(),
            pre_bayes_uses_soft_evidence: true,
            canonical_structural_active_regime: Some("trend".to_string()),
            canonical_structural_confidence: Some(0.78),
            canonical_structural_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.78),
                ("range".to_string(), 0.14),
                ("transition".to_string(), 0.08),
            ]),
            pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                "node".to_string(),
                std::collections::BTreeMap::from([("state".to_string(), 0.25)]),
            )]),
            ..WorkflowPhaseSnapshot::default()
        };
        let snapshot = WorkflowSnapshot {
            latest_analyze: Some(analyze),
            latest_pre_bayes_soft_evidence_diff: vec![PreBayesSoftEvidenceNodeDiff::default()],
            ..WorkflowSnapshot::default()
        };
        let value = build_pre_bayes_status_value(&snapshot, None).unwrap();
        assert_eq!(value["latest_gate_status"], "pass_neutralized");
        assert_eq!(value["latest_policy_version"], "v2");
        assert_eq!(value["latest_uses_soft_evidence"], true);
        assert_eq!(value["latest_soft_evidence"]["node"]["state"], 0.25);
        assert_eq!(value["latest_canonical_structural_active_regime"], "trend");
        assert_eq!(value["latest_canonical_structural_confidence"], 0.78);
        assert_eq!(
            value["latest_canonical_structural_probabilities"]["trend"],
            0.78
        );
        assert_eq!(
            value["latest_soft_evidence_diff"].as_array().unwrap().len(),
            1
        );
    }

    #[test]
    fn build_pre_bayes_diff_value_matches_main_surface() {
        let analyze = WorkflowPhaseSnapshot {
            pre_bayes_gate_status: "blocked".to_string(),
            pre_bayes_policy_version: "v3".to_string(),
            pre_bayes_uses_soft_evidence: false,
            canonical_structural_active_regime: Some("range".to_string()),
            canonical_structural_confidence: Some(0.61),
            canonical_structural_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.21),
                ("range".to_string(), 0.61),
                ("transition".to_string(), 0.18),
            ]),
            ..WorkflowPhaseSnapshot::default()
        };
        let snapshot = WorkflowSnapshot {
            latest_analyze: Some(analyze),
            latest_pre_bayes_policy_diff: Some(PreBayesPolicyDiff::default()),
            latest_pre_bayes_policy_lineage: Some(PreBayesPolicyLineageSummary::default()),
            latest_pre_bayes_entry_quality_bridge: Some(PreBayesEntryQualityBridge::default()),
            latest_pre_bayes_entry_quality_bridge_diff: Some(
                PreBayesEntryQualityBridgeDiff::default(),
            ),
            latest_pre_bayes_soft_evidence_diff: vec![PreBayesSoftEvidenceNodeDiff::default()],
            ..WorkflowSnapshot::default()
        };
        let value = build_pre_bayes_diff_value(&snapshot);
        assert_eq!(value["latest_gate_status"], "blocked");
        assert_eq!(value["latest_policy_version"], "v3");
        assert_eq!(value["latest_uses_soft_evidence"], false);
        assert_eq!(value["latest_canonical_structural_active_regime"], "range");
        assert_eq!(value["latest_canonical_structural_confidence"], 0.61);
        assert_eq!(
            value["latest_canonical_structural_probabilities"]["range"],
            0.61
        );
        assert_eq!(
            value["latest_soft_evidence_diff"].as_array().unwrap().len(),
            1
        );
    }

    #[test]
    fn build_pre_bayes_status_value_prefers_latest_update_phase_when_analyze_missing() {
        let update = WorkflowPhaseSnapshot {
            pre_bayes_gate_status: "pass_hard".to_string(),
            pre_bayes_policy_version: "v-update".to_string(),
            pre_bayes_uses_soft_evidence: true,
            canonical_structural_active_regime: Some("range".to_string()),
            canonical_structural_confidence: Some(0.61),
            canonical_structural_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.21),
                ("range".to_string(), 0.61),
                ("transition".to_string(), 0.18),
            ]),
            pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                "market_regime".to_string(),
                std::collections::BTreeMap::from([
                    ("range".to_string(), 0.61),
                    ("trend".to_string(), 0.21),
                    ("transition".to_string(), 0.18),
                ]),
            )]),
            ..WorkflowPhaseSnapshot::default()
        };
        let snapshot = WorkflowSnapshot {
            latest_update: Some(update),
            latest_pre_bayes_soft_evidence_diff: vec![PreBayesSoftEvidenceNodeDiff::default()],
            ..WorkflowSnapshot::default()
        };

        let value = build_pre_bayes_status_value(&snapshot, None).unwrap();

        assert_eq!(value["latest_gate_status"], "pass_hard");
        assert_eq!(value["latest_policy_version"], "v-update");
        assert_eq!(value["latest_canonical_structural_active_regime"], "range");
        assert_eq!(value["latest_canonical_structural_confidence"], 0.61);
        assert_eq!(
            value["latest_soft_evidence"]["market_regime"]["range"],
            0.61
        );
    }

    #[test]
    fn build_pre_bayes_status_value_falls_back_to_analyze_when_latest_update_has_no_structural_or_pre_bayes_surface(
    ) {
        let analyze = WorkflowPhaseSnapshot {
            pre_bayes_gate_status: "pass_neutralized".to_string(),
            pre_bayes_policy_version: "v-analyze".to_string(),
            pre_bayes_uses_soft_evidence: true,
            canonical_structural_active_regime: Some("trend".to_string()),
            canonical_structural_confidence: Some(0.78),
            canonical_structural_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.78),
                ("range".to_string(), 0.14),
                ("transition".to_string(), 0.08),
            ]),
            pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                "market_regime".to_string(),
                std::collections::BTreeMap::from([
                    ("trend".to_string(), 0.78),
                    ("range".to_string(), 0.14),
                    ("transition".to_string(), 0.08),
                ]),
            )]),
            ..WorkflowPhaseSnapshot::default()
        };
        let update = WorkflowPhaseSnapshot {
            phase: "update".to_string(),
            pre_bayes_gate_status: String::new(),
            pre_bayes_policy_version: String::new(),
            pre_bayes_uses_soft_evidence: false,
            canonical_structural_active_regime: None,
            canonical_structural_confidence: None,
            canonical_structural_probabilities: std::collections::BTreeMap::new(),
            pre_bayes_soft_evidence: std::collections::BTreeMap::new(),
            ..WorkflowPhaseSnapshot::default()
        };
        let snapshot = WorkflowSnapshot {
            latest_analyze: Some(analyze),
            latest_update: Some(update),
            latest_pre_bayes_soft_evidence_diff: vec![PreBayesSoftEvidenceNodeDiff::default()],
            ..WorkflowSnapshot::default()
        };

        let value = build_pre_bayes_status_value(&snapshot, None).unwrap();

        assert_eq!(value["latest_gate_status"], "pass_neutralized");
        assert_eq!(value["latest_policy_version"], "v-analyze");
        assert_eq!(value["latest_canonical_structural_active_regime"], "trend");
        assert_eq!(value["latest_canonical_structural_confidence"], 0.78);
        assert_eq!(
            value["latest_soft_evidence"]["market_regime"]["trend"],
            0.78
        );
    }

    #[test]
    fn build_pre_bayes_status_value_prefers_newest_populated_phase_over_fixed_update_priority() {
        let update = WorkflowPhaseSnapshot {
            phase: "update".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
            pre_bayes_gate_status: "pass_hard".to_string(),
            pre_bayes_policy_version: "v-update".to_string(),
            canonical_structural_active_regime: Some("range".to_string()),
            canonical_structural_confidence: Some(0.61),
            canonical_structural_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.21),
                ("range".to_string(), 0.61),
                ("transition".to_string(), 0.18),
            ]),
            pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                "market_regime".to_string(),
                std::collections::BTreeMap::from([
                    ("range".to_string(), 0.61),
                    ("trend".to_string(), 0.21),
                    ("transition".to_string(), 0.18),
                ]),
            )]),
            ..WorkflowPhaseSnapshot::default()
        };
        let research = WorkflowPhaseSnapshot {
            phase: "research".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(),
            pre_bayes_gate_status: "observe".to_string(),
            pre_bayes_policy_version: "v-research".to_string(),
            canonical_structural_active_regime: Some("trend".to_string()),
            canonical_structural_confidence: Some(0.78),
            canonical_structural_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.78),
                ("range".to_string(), 0.14),
                ("transition".to_string(), 0.08),
            ]),
            pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                "market_regime".to_string(),
                std::collections::BTreeMap::from([
                    ("trend".to_string(), 0.78),
                    ("range".to_string(), 0.14),
                    ("transition".to_string(), 0.08),
                ]),
            )]),
            ..WorkflowPhaseSnapshot::default()
        };
        let snapshot = WorkflowSnapshot {
            latest_update: Some(update),
            latest_research: Some(research),
            latest_pre_bayes_soft_evidence_diff: vec![PreBayesSoftEvidenceNodeDiff::default()],
            ..WorkflowSnapshot::default()
        };

        let value = build_pre_bayes_status_value(&snapshot, None).unwrap();

        assert_eq!(value["latest_gate_status"], "observe");
        assert_eq!(value["latest_policy_version"], "v-research");
        assert_eq!(value["latest_canonical_structural_active_regime"], "trend");
        assert_eq!(value["latest_canonical_structural_confidence"], 0.78);
        assert_eq!(
            value["latest_soft_evidence"]["market_regime"]["trend"],
            0.78
        );
    }

    #[test]
    fn build_workflow_status_bootstrap_phase_value_matches_bootstrap_view() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_workflow_status_bootstrap_phase_value_with_probe(
            WorkflowStatusBootstrapInput {
                symbol: "NQ",
                state_dir: "state",
                detected_tomac_root: Some("/tmp/tomac".to_string()),
                multi_timeframe_clean_root: Some("/tmp/clean".to_string()),
                tomac_root_placeholder: "<root>".to_string(),
            },
            &snapshot,
            &sample_provider_agent_surface(),
            &|_, port| port == 4002,
        )
        .unwrap();
        assert_eq!(value["symbol"], "NQ");
        assert_eq!(value["detected_paths"]["state_dir"], "state");
        assert_eq!(
            value["commands"]["workflow_status"],
            "ict-engine workflow-status --symbol NQ --state-dir state"
        );
        assert!(
            value["input_acquisition"]["live"]["provider_access_requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item
                    .as_str()
                    .unwrap()
                    .contains("TradingViewRemix MCP API key"))
        );
        assert!(
            value["input_acquisition"]["live"]["provider_access_requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item.as_str().unwrap().contains("IBKR TWS or IB Gateway"))
        );
        assert_eq!(
            value["input_acquisition"]["live"]["ibkr_gateway_summary"]["occupied_judgement"],
            "single_reachable_candidate"
        );
        assert_eq!(
            value["input_acquisition"]["live"]["ibkr_gateway_candidates"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
    }

    #[test]
    fn build_ibkr_gateway_candidates_marks_first_reachable_as_recommended() {
        let candidates = build_ibkr_gateway_candidates_with_probe("127.0.0.1", &|_, port| {
            matches!(port, 4002 | 4001)
        });

        assert_eq!(candidates.len(), 4);
        assert!(
            candidates
                .iter()
                .find(|candidate| candidate.port == 4002)
                .unwrap()
                .recommended
        );
        assert!(
            candidates
                .iter()
                .find(|candidate| candidate.port == 4001)
                .unwrap()
                .reachable
        );
        assert!(
            !candidates
                .iter()
                .find(|candidate| candidate.port == 4001)
                .unwrap()
                .recommended
        );
    }

    #[test]
    fn build_ibkr_gateway_summary_flags_multiple_reachable_candidates() {
        let candidates = vec![
            AgentBootstrapIbkrGatewayCandidate {
                label: "TWS paper".to_string(),
                host: "127.0.0.1".to_string(),
                port: 7497,
                reachable: true,
                recommended: true,
            },
            AgentBootstrapIbkrGatewayCandidate {
                label: "IB Gateway paper".to_string(),
                host: "127.0.0.1".to_string(),
                port: 4002,
                reachable: true,
                recommended: false,
            },
        ];

        let summary = build_ibkr_gateway_summary(&candidates);
        assert_eq!(
            summary.occupied_judgement,
            "multiple_reachable_candidates_choose_explicit_port"
        );
        assert_eq!(summary.preferred_port, Some(7497));
        assert!(summary.recommended_action.contains("--gateway-port 7497"));
    }

    #[test]
    fn build_workflow_status_phase_value_matches_human_surface() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "human",
        )
        .unwrap();
        assert_eq!(
            value["summary_line"],
            "NQ | update | action_blocked | pda_cluster=cluster_1 | duration=negative_binomial | remaining_bars=2.50"
        );
        assert_eq!(value["current_status"]["focus_phase"], "update");
    }

    #[test]
    fn build_workflow_status_phase_value_supports_artifact_alias() {
        let snapshot = WorkflowSnapshot {
            artifact_factor_trends: vec![ArtifactFactorTrendSummary {
                factor_name: "fvg_rebalance".to_string(),
                consumed_entries: 2,
                entries: 3,
                ..ArtifactFactorTrendSummary::default()
            }],
            ..WorkflowSnapshot::default()
        };

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "artifact-factor-consumed-leaderboard",
        )
        .unwrap();
        assert_eq!(value.as_array().unwrap()[0]["factor_name"], "fvg_rebalance");
    }

    #[test]
    fn build_workflow_status_phase_value_rejects_unknown_phase() {
        let err = build_workflow_status_phase_value(
            &WorkflowSnapshot::default(),
            &[],
            &sample_provider_agent_surface(),
            &[],
            "wat",
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported workflow-status phase 'wat'"));
    }

    #[test]
    fn build_workflow_status_phase_value_preserves_redactable_paths() {
        let snapshot = sample_human_workflow_snapshot();
        let mut value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "human",
        )
        .unwrap();
        redact_local_paths_in_value(&mut value);
        let rendered = serde_json::to_string(&value).unwrap();
        assert!(rendered.contains("<local-path>"));
    }

    #[test]
    fn dispatch_workflow_status_rejects_phase_and_filter_mix() {
        let error = dispatch_workflow_status(
            &sample_human_workflow_snapshot(),
            &[],
            &sample_provider_agent_surface(),
            &[],
            &StructuralPriorLearningState::default(),
            WorkflowStatusDispatchInput {
                phase: Some("human"),
                actionable_only: true,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
            WorkflowStatusBootstrapInput {
                symbol: "NQ",
                state_dir: "/tmp/state",
                detected_tomac_root: None,
                multi_timeframe_clean_root: None,
                tomac_root_placeholder: "<tomac-root>".to_string(),
            },
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("phase and filter flags are mutually exclusive"));
    }

    #[test]
    fn dispatch_workflow_status_rejects_multiple_artifact_filters() {
        let error = dispatch_workflow_status(
            &sample_human_workflow_snapshot(),
            &[],
            &sample_provider_agent_surface(),
            &[],
            &StructuralPriorLearningState::default(),
            WorkflowStatusDispatchInput {
                phase: None,
                actionable_only: true,
                conflicts_only: true,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
            WorkflowStatusBootstrapInput {
                symbol: "NQ",
                state_dir: "/tmp/state",
                detected_tomac_root: None,
                multi_timeframe_clean_root: None,
                tomac_root_placeholder: "<tomac-root>".to_string(),
            },
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("accepts at most one artifact filter flag"));
    }

    #[test]
    fn normalize_workflow_status_value_for_stability_removes_timestamp_like_fields() {
        let mut value = serde_json::json!({
            "generated_at": "2024-01-01T00:00:00Z",
            "timestamp": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "last_updated_at": "2024-01-01T00:00:00Z",
            "fetched_at": "2024-01-01T00:00:00Z",
            "kept": "yes",
            "nested": {
                "generated_at": "2024-01-01T00:00:00Z",
                "kept": "still-here"
            },
            "items": [
                {
                    "generated_at": "2024-01-01T00:00:00Z",
                    "kept": 1
                }
            ]
        });

        normalize_workflow_status_value_for_stability(&mut value);

        assert!(value.get("generated_at").is_none());
        assert!(value.get("timestamp").is_none());
        assert!(value.get("updated_at").is_none());
        assert!(value.get("last_updated_at").is_none());
        assert!(value.get("fetched_at").is_none());
        assert_eq!(value["kept"], "yes");
        assert!(value["nested"].get("generated_at").is_none());
        assert_eq!(value["nested"]["kept"], "still-here");
        assert!(value["items"][0].get("generated_at").is_none());
        assert_eq!(value["items"][0]["kept"], 1);
    }

    #[test]
    fn human_workflow_status_view_exposes_candidates() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_human_workflow_status_view(&snapshot, &[]);
        assert_eq!(value["symbol"], "NQ");
        assert_eq!(value["current_status"]["focus_phase"], "update");
        assert_eq!(value["pda_cluster_label"], "cluster_1");
        assert_eq!(value["hard_block"]["active"], true);
        assert_eq!(value["hard_block"]["status"], "action_blocked");
        assert_eq!(
            value["hard_block"]["reason"],
            "user_selected_historical_data_missing"
        );
        assert!(value["hard_block"]["human_action"]
            .as_str()
            .unwrap()
            .contains("Ask the user to choose the historical dataset"));
        assert!(value["what_you_should_do_now"]
            .as_str()
            .unwrap()
            .contains("Ask the user to choose the historical dataset"));
        assert_eq!(value["historical_data_candidates"][0], "/tmp/a.json");
        assert_eq!(value["ensemble_consensus"]["final_action"], "observe");
        assert_eq!(value["ensemble_consensus"]["hard_block"]["active"], true);
        assert_eq!(
            value["ensemble_consensus"]["hard_block"]["reason"],
            "user_selected_historical_data_missing"
        );
    }

    #[test]
    fn jump_workflow_summaries_surface_calibration_gate() {
        let snapshot = sample_human_workflow_snapshot();
        assert_eq!(
            jump_model_workflow_summary(&snapshot).as_deref(),
            Some(
                "jump_model active_state=jump_transition confidence=0.500 transition_risk=0.500; jump_calibration_gate outcome=accepted sample_count=4 cooldown_status=ready"
            )
        );
        assert_eq!(
            jump_calibration_gate_workflow_summary(&snapshot).as_deref(),
            Some("jump_calibration_gate outcome=accepted sample_count=4 cooldown_status=ready")
        );
    }

    #[test]
    fn workflow_status_human_view_prefers_persisted_scorecards() {
        let snapshot = sample_human_workflow_snapshot();
        let persisted = vec![EnsembleExecutorScorecard {
            executor: "xgboost_file".to_string(),
            latest_weight_hint: Some(0.72),
            ..EnsembleExecutorScorecard::default()
        }];
        let value = build_human_workflow_status_view(&snapshot, &persisted);
        assert_eq!(
            value["ensemble_consensus"]["executor_scorecards"][0]["executor"],
            "xgboost_file"
        );
        assert_eq!(
            value["ensemble_consensus"]["executor_scorecard_source"],
            "persisted"
        );
    }

    #[test]
    fn human_workflow_status_view_exposes_human_summary_fields() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_human_workflow_status_view(&snapshot, &[]);
        assert_eq!(
            value["summary_line"],
            "NQ | update | action_blocked | pda_cluster=cluster_1 | duration=negative_binomial | remaining_bars=2.50"
        );
        assert_eq!(
            value["next_action_line"],
            "Next: Ask the user to choose the historical dataset. Please choose one historical data path for the next research/backtest run: [a.json] /tmp/a.json, [b.json] /tmp/b.json Reply with one path from the list, or paste another valid file path. Candidates: [a.json] /tmp/a.json, [b.json] /tmp/b.json Then run: ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state"
        );
        assert_eq!(
            value["blocking_line"],
            "Block: user_selected_historical_data_missing"
        );
        assert_eq!(
            value["phase_summary_line"],
            "Latest: update | entry=medium gate=pass_neutralized quality=0.500"
        );
        assert_eq!(value["hybrid_duration_model"], "negative_binomial");
        assert_eq!(value["hybrid_remaining_expected_bars"], "2.50");
    }

    #[test]
    fn human_workflow_status_deferred_command_keeps_selected_profile_on_factor_research() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
        );
        assert_eq!(
            value["recommended_next_step"]["blocked_reason"].as_str(),
            Some("user_selected_historical_data_missing")
        );
        assert_eq!(
            value["recommended_next_step"]["deferred_command"].as_str(),
            Some(
                "ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state --profile thrill3r-nq-closed-loop-v1"
            )
        );
        assert!(value["next_action_line"]
            .as_str()
            .unwrap_or_default()
            .contains("--profile thrill3r-nq-closed-loop-v1"));
    }

    #[test]
    fn human_workflow_status_next_line_does_not_duplicate_next_prefix() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "DEMO".to_string();
        snapshot.current_focus_phase = "research".to_string();
        snapshot.recommended_next_command =
            "ict-engine factor-research --symbol DEMO --backend native".to_string();
        snapshot.latest_research = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "research".to_string(),
            phase_summary: "research ready".to_string(),
            recommended_next_command: snapshot.recommended_next_command.clone(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });

        let value = build_human_workflow_status_view(&snapshot, &[]);

        let next = value["next_action_line"].as_str().unwrap();
        assert!(next.starts_with("Next: "));
        assert!(!next.contains("Next: Next step:"));
        assert!(next.contains("ensemble-vote"));
    }

    #[test]
    fn human_workflow_status_hides_unavailable_pre_bayes_gate_sentinel() {
        let snapshot = WorkflowSnapshot {
            symbol: "DEMO".to_string(),
            current_focus_phase: "research".to_string(),
            blocking_truth: crate::state::WorkflowBlockingTruth {
                status: "pass_neutralized".to_string(),
                ..crate::state::WorkflowBlockingTruth::default()
            },
            latest_research: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "research".to_string(),
                phase_summary: "research_ready".to_string(),
                pre_bayes_gate_status: "pre_bayes_gate_unavailable".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_human_workflow_status_view(&snapshot, &[]);

        assert_eq!(
            value["phase_summary_line"],
            "Latest: research | research_ready"
        );
        assert!(!value["phase_summary_line"]
            .as_str()
            .unwrap()
            .contains("pre_bayes_gate_unavailable"));
    }

    #[test]
    fn human_workflow_status_compacts_research_phase_summary() {
        let snapshot = WorkflowSnapshot {
            symbol: "DEMO".to_string(),
            current_focus_phase: "research".to_string(),
            latest_research: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "research".to_string(),
                phase_summary: "objective=expansion_manipulation best_factor=Some(\"trend_momentum\") aggregate_return=0.0017 feedback_applied=46 credibility=conformal_credibility:unavailable mtf_source=primary_only execution_gate=execution_observe_only".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_human_workflow_status_view(&snapshot, &[]);
        assert_eq!(
            value["phase_summary_line"],
            "Latest: research | objective=expansion_manipulation best_factor=trend_momentum aggregate_return=0.0017 feedback_applied=46 execution_gate=execution_observe_only"
        );
    }

    #[test]
    fn human_workflow_status_empty_state_uses_explicit_no_state_contract() {
        let value = build_human_workflow_status_view(&WorkflowSnapshot::default(), &[]);
        assert_eq!(value["status"], "no_workflow_state");
        assert_eq!(value["current_status"]["focus_phase"], "workflow_status");
        assert_eq!(
            value["current_status"]["blocking_status"],
            "no_workflow_state"
        );
        assert_eq!(value["latest_stage"]["phase"], "no_workflow_state");
        assert_eq!(
            value["latest_stage"]["summary_short"],
            "No workflow phase summary available yet."
        );
        assert!(value["structural_validation_line"].is_null());
        assert!(value["next_action_line"]
            .as_str()
            .unwrap()
            .contains("ict-engine provider-status --compact"));
        assert!(value["route_line"]
            .as_str()
            .unwrap()
            .contains("--phase bootstrap --human"));
    }

    #[test]
    fn human_workflow_status_prefers_newest_phase_over_fixed_update_priority() {
        let snapshot = WorkflowSnapshot {
            latest_update: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "update".to_string(),
                timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                phase_summary: "older_update_summary".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_research: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "research".to_string(),
                timestamp: Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(),
                phase_summary: "newer_research_summary".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_human_workflow_status_view(&snapshot, &[]);

        assert_eq!(value["latest_stage"]["phase"], "research");
        assert_eq!(value["latest_stage"]["summary"], "newer_research_summary");
    }

    #[test]
    fn workflow_status_routes_research_users_into_evidence_review_before_rerun() {
        let snapshot = WorkflowSnapshot {
            symbol: "DEMO".to_string(),
            current_focus_phase: "research".to_string(),
            current_focus_reason: "no_previous_run".to_string(),
            recommended_next_command: "ict-engine factor-research --symbol DEMO --data /tmp/demo.json --state-dir /tmp/state --backend native --objective expansion_manipulation".to_string(),
            latest_research: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "research".to_string(),
                phase_summary: "objective=expansion_manipulation best_factor=trend_momentum aggregate_return=0.0017 feedback_applied=46 execution_gate=execution_observe_only".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let human = build_human_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &StructuralPriorLearningState::default(),
            Some("/tmp/state"),
        );
        assert!(human["next_action_line"]
            .as_str()
            .unwrap()
            .contains("--phase ensemble-vote"));
        assert!(human["route_line"]
            .as_str()
            .unwrap()
            .contains("ict-engine pre-bayes-status --symbol DEMO --state-dir /tmp/state --human"));
        assert!(human["route_line"]
            .as_str()
            .unwrap()
            .contains("--phase structural-recommended-path-bundle --human"));

        let agent = build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &StructuralPriorLearningState::default(),
            Some("/tmp/state"),
        );
        assert_eq!(agent["next_command_source"], "evidence_review_router");
        assert!(agent["next_command"]
            .as_str()
            .unwrap()
            .contains("--phase ensemble-vote"));
        assert!(agent["evidence_review"]["pre_bayes_status_command"]
            .as_str()
            .unwrap()
            .contains("ict-engine pre-bayes-status --symbol DEMO"));
    }

    #[test]
    fn evidence_review_router_keeps_selected_profile_on_workflow_status_followups() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            current_focus_phase: "research".to_string(),
            current_focus_reason: "no_previous_run".to_string(),
            recommended_next_command: "ict-engine factor-research --symbol NQ --data /tmp/demo.json --state-dir /tmp/state --backend native --objective expansion_manipulation".to_string(),
            latest_research: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "research".to_string(),
                phase_summary: "research_summary".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let agent = build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
            &StructuralPriorLearningState::default(),
            Some("/tmp/state"),
        );

        assert_eq!(agent["next_command_source"], "evidence_review_router");
        assert_eq!(
            agent["next_command"].as_str(),
            Some("ict-engine workflow-status --symbol NQ --state-dir /tmp/state --profile thrill3r-nq-closed-loop-v1 --phase ensemble-vote")
        );
        assert_eq!(
            agent["evidence_review"]["structural_path_command"].as_str(),
            Some("ict-engine workflow-status --symbol NQ --state-dir /tmp/state --profile thrill3r-nq-closed-loop-v1 --phase structural-recommended-path-bundle")
        );
    }

    #[test]
    fn agent_workflow_status_empty_state_uses_explicit_no_state_contract() {
        let value = build_agent_workflow_status_view(&WorkflowSnapshot::default(), &[]);
        assert_eq!(value["status"], "no_workflow_state");
        assert_eq!(value["latest_phase"], "no_workflow_state");
        assert_eq!(value["blocking_status"], "no_workflow_state");
        assert_eq!(value["blocking_reason"], "no_workflow_state");
        assert_eq!(value["next_command_source"], "first_run_router");
        assert!(value["next_command"]
            .as_str()
            .unwrap()
            .contains("--phase bootstrap"));
        assert_eq!(value["next_step"]["action_type"], "run_command");
        assert!(value["first_run_router"]["provider_summary"]
            .as_str()
            .unwrap()
            .contains("tradfi free fallback"));
    }

    #[test]
    fn workflow_status_routes_auto_quant_handoff_candidate_before_first_run_router() {
        let temp = tempfile::tempdir().unwrap();
        let handoff = persist_sample_auto_quant_handoff(&temp);
        let snapshot = WorkflowSnapshot {
            symbol: "DEMO".to_string(),
            actionable_artifacts: vec![handoff],
            ..WorkflowSnapshot::default()
        };

        let human = build_human_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &StructuralPriorLearningState::default(),
            Some(temp.path().to_str().unwrap()),
        );
        assert!(human["next_action_line"]
            .as_str()
            .unwrap()
            .contains("Auto-Quant handoff"));
        assert!(human["route_line"]
            .as_str()
            .unwrap()
            .contains("auto-quant-adoption-review"));
        assert!(human["first_run_router"].is_null());

        let agent = build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &StructuralPriorLearningState::default(),
            Some(temp.path().to_str().unwrap()),
        );
        assert_eq!(agent["next_command_source"], "auto_quant_handoff_candidate");
        assert_eq!(
            agent["top_actionable"]["artifact_kind"],
            "auto_quant_handoff_candidate"
        );
        assert!(agent["recommended_next_step"]["deferred_command"]
            .as_str()
            .unwrap()
            .contains("auto-quant-prepare"));
        assert_eq!(agent["first_run_router"], serde_json::Value::Null);
        assert!(agent["auto_quant_handoff"]["review_command"]
            .as_str()
            .unwrap()
            .contains("auto-quant-adoption-review"));
    }

    #[test]
    fn auto_quant_handoff_router_keeps_selected_profile_on_workflow_status_followup() {
        let temp = tempfile::tempdir().unwrap();
        let handoff = persist_sample_auto_quant_handoff(&temp);
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            actionable_artifacts: vec![handoff],
            ..WorkflowSnapshot::default()
        };

        let agent = build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
            &StructuralPriorLearningState::default(),
            Some(temp.path().to_str().unwrap()),
        );

        assert_eq!(agent["next_command_source"], "auto_quant_handoff_candidate");
        let command = agent["auto_quant_handoff"]["workflow_status_command"]
            .as_str()
            .unwrap_or_default();
        assert!(command.contains("ict-engine workflow-status --symbol DEMO"));
        assert!(command.contains("--profile thrill3r-nq-closed-loop-v1"));
        assert!(command.ends_with(" --human"));
    }

    #[test]
    fn agent_workflow_status_view_exposes_relevant_provider_support() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            current_focus_phase: "analyze_live".to_string(),
            current_focus_reason: "provider_runtime_required".to_string(),
            blocking_truth: crate::state::WorkflowBlockingTruth {
                status: "blocked".to_string(),
                reason: "provider_runtime_required".to_string(),
                next_command: "ict-engine analyze-live --symbol NQ --futures-symbol NQ=F --spot-symbol QQQ --options-symbol QQQ --futures-backend external_http_runtime --aux-backend crypto_public_runtime".to_string(),
                ..crate::state::WorkflowBlockingTruth::default()
            },
            latest_analyze: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "analyze_live".to_string(),
                phase_summary: "live_provider_runtime_pending".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
        );

        assert_eq!(value["provider_support"]["active"], true);
        assert_eq!(value["provider_support"]["profile_id"], "workflow_auto");
        assert_eq!(
            value["provider_support"]["pending_providers"][0],
            "crypto_public_runtime"
        );
        assert!(value["provider_support"]["ask_user_prompts"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(value["provider_support"]["install_prompts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap().contains("zero-config yfinance")));
        assert_eq!(
            value["available_opt_in_profiles"]
                .as_array()
                .unwrap()
                .first()
                .and_then(|item| item.get("selector"))
                .and_then(serde_json::Value::as_str),
            Some("thrill3r-nq-closed-loop-v1")
        );
    }

    #[test]
    fn selected_profile_first_run_router_keeps_profile_on_provider_and_bootstrap_commands() {
        let agent = build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &WorkflowSnapshot {
                symbol: "NQ".to_string(),
                ..WorkflowSnapshot::default()
            },
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
            &StructuralPriorLearningState::default(),
            Some("/tmp/state"),
        );

        assert_eq!(agent["next_command_source"], "first_run_router");
        assert_eq!(
            agent["first_run_router"]["provider_command"].as_str(),
            Some("ict-engine provider-status --compact --profile thrill3r-nq-closed-loop-v1")
        );
        assert_eq!(
            agent["first_run_router"]["bootstrap_command"].as_str(),
            Some(
                "ict-engine workflow-status --symbol NQ --state-dir /tmp/state --profile thrill3r-nq-closed-loop-v1 --phase bootstrap"
            )
        );
        assert_eq!(
            agent["first_run_router"]["routes"][1]["command"].as_str(),
            Some(
                "ict-engine factor-research --symbol NQ --data <historical-data.json> --state-dir /tmp/state --human --profile thrill3r-nq-closed-loop-v1"
            )
        );
    }

    #[test]
    fn selected_profile_promotes_generic_workflow_status_next_command() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            recommended_next_command:
                "ict-engine workflow-status --symbol NQ --state-dir /tmp/state --phase human-next"
                    .to_string(),
            latest_update: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "update".to_string(),
                phase_summary: "update_summary".to_string(),
                recommended_next_command:
                    "ict-engine workflow-status --symbol NQ --state-dir /tmp/state --phase human-next"
                        .to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let agent = build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
            &StructuralPriorLearningState::default(),
            Some("/tmp/state"),
        );

        assert_eq!(
            agent["next_command"].as_str(),
            Some(
                "ict-engine workflow-status --symbol NQ --state-dir /tmp/state --phase human-next --profile thrill3r-nq-closed-loop-v1"
            )
        );
    }

    #[test]
    fn selected_profile_promotes_generic_blocking_truth_workflow_status_command() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            blocking_truth: crate::state::WorkflowBlockingTruth {
                status: "blocked".to_string(),
                reason: "credibility_gate_blocked".to_string(),
                next_command:
                    "ict-engine workflow-status --symbol NQ --state-dir /tmp/state --phase artifact-consumed-gate"
                        .to_string(),
                ..crate::state::WorkflowBlockingTruth::default()
            },
            latest_update: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "update".to_string(),
                phase_summary: "update_summary".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let agent = build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
            &StructuralPriorLearningState::default(),
            Some("/tmp/state"),
        );

        assert_eq!(
            agent["next_command"].as_str(),
            Some(
                "ict-engine workflow-status --symbol NQ --state-dir /tmp/state --phase artifact-consumed-gate --profile thrill3r-nq-closed-loop-v1"
            )
        );
    }

    #[test]
    fn human_workflow_status_view_adds_provider_line_for_missing_runtime() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            current_focus_phase: "analyze_live".to_string(),
            current_focus_reason: "provider_runtime_required".to_string(),
            recommended_next_command: "ict-engine analyze-live --symbol NQ --futures-symbol NQ=F --spot-symbol QQQ --options-symbol QQQ --futures-backend external_http_runtime --aux-backend crypto_public_runtime".to_string(),
            latest_analyze: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "analyze_live".to_string(),
                phase_summary: "live_provider_runtime_pending".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
        );

        assert!(value["provider_line"]
            .as_str()
            .unwrap()
            .contains("external_http_runtime"));
        assert!(value["provider_line"]
            .as_str()
            .unwrap()
            .contains("crypto_public_runtime"));
        assert!(value["provider_line"]
            .as_str()
            .unwrap()
            .contains("zero-config yfinance"));
        assert_eq!(
            value["provider_support"]["workflow_support"]["active"],
            true
        );
    }

    #[test]
    fn human_workflow_status_view_hides_opt_in_profile_hint_when_unselected() {
        let value = build_human_workflow_status_view(&WorkflowSnapshot::default(), &[]);
        assert!(value["opt_in_profile_line"].is_null());
        assert!(value["provider_line"]
            .as_str()
            .unwrap()
            .contains("tradfi free fallback"));
    }

    #[test]
    fn human_workflow_status_view_shows_opt_in_profile_hint_only_on_first_run_empty_state() {
        let value = build_human_workflow_status_view_with_provider_agent(
            &WorkflowSnapshot {
                symbol: "NQ".to_string(),
                ..WorkflowSnapshot::default()
            },
            &[],
            &sample_provider_agent_surface(),
            &[],
        );
        let no_profile_line = value["opt_in_profile_line"].as_str().unwrap_or_default();
        assert!(no_profile_line.contains("Optional personal lane:"));
        assert!(no_profile_line.contains(
            "ict-engine workflow-status --symbol NQ --state-dir <local-path> --profile thrill3r-nq-closed-loop-v1 --human"
        ));

        let first_run = build_human_workflow_status_view_with_provider_agent(
            &WorkflowSnapshot {
                symbol: "NQ".to_string(),
                ..WorkflowSnapshot::default()
            },
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
        );
        let selected_line = first_run["opt_in_profile_line"]
            .as_str()
            .unwrap_or_default();
        assert!(selected_line.is_empty() || selected_line.contains("Optional personal lane:"));
    }

    #[test]
    fn human_workflow_status_view_keeps_selected_profile_in_provider_command() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            current_focus_phase: "analyze_live".to_string(),
            current_focus_reason: "provider_runtime_required".to_string(),
            recommended_next_command: "ict-engine analyze-live --symbol NQ --futures-symbol NQ=F --spot-symbol QQQ --options-symbol QQQ --futures-backend external_http_runtime --aux-backend crypto_public_runtime".to_string(),
            latest_analyze: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "analyze_live".to_string(),
                phase_summary: "live_provider_runtime_pending".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
        );

        assert!(value["provider_line"]
            .as_str()
            .unwrap()
            .contains("ict-engine provider-status --agent --profile"));
        assert!(value["provider_line"]
            .as_str()
            .unwrap()
            .contains("thrill3r-nq-closed-loop-v1"));
        assert_eq!(
            value["provider_support"]["workflow_support"]["provider_status_command"],
            serde_json::json!(
                "ict-engine provider-status --agent --profile thrill3r-nq-closed-loop-v1"
            )
        );
        assert!(value["opt_in_profile_line"].is_null());
    }

    #[test]
    fn agent_bootstrap_live_input_surfaces_selected_profile_contracts() {
        let view = build_agent_bootstrap_view_with_candidates(
            AgentBootstrapBuildInput {
                symbol: "NQ",
                state_dir: "/tmp/state",
                snapshot: &WorkflowSnapshot::default(),
                provider_status_agent: &sample_provider_agent_surface_with_profile(),
                detected_tomac_root: Some("/tmp/tomac".to_string()),
                multi_timeframe_clean_root: Some("/tmp/ict-engine-mtf".to_string()),
                tomac_root_placeholder: "<tomac-root>",
            },
            Vec::new(),
        );

        assert_eq!(
            view.input_acquisition.live.selected_profile_id.as_deref(),
            Some("thrill3r_nq_closed_loop_v1")
        );
        assert!(view
            .input_acquisition
            .live
            .selected_profile_summary
            .as_deref()
            .unwrap()
            .contains("Personal NQ workflow"));
        assert!(view
            .input_acquisition
            .live
            .selected_profile_data_contracts
            .iter()
            .any(|item| item.contains("Tomac cleaned multi-timeframe futures root")));
        assert!(view
            .input_acquisition
            .live
            .selected_profile_track_statuses
            .iter()
            .any(|item| item.contains("options_enriched:pending:tradingview_mcp")));
        assert_eq!(
            view.input_acquisition.live.provider_access_requests,
            vec!["Ask for TradingViewRemix MCP API key.".to_string()]
        );
        assert_eq!(
            view.commands.provider_status,
            "ict-engine provider-status --agent --profile thrill3r-nq-closed-loop-v1"
        );
        assert_eq!(
            view.commands.workflow_status,
            "ict-engine workflow-status --symbol NQ --state-dir /tmp/state --profile thrill3r-nq-closed-loop-v1"
        );
    }

    #[test]
    fn agent_bootstrap_without_profile_does_not_reuse_detected_personal_paths() {
        let view = build_agent_bootstrap_view_with_candidates(
            AgentBootstrapBuildInput {
                symbol: "NQ",
                state_dir: "/tmp/state",
                snapshot: &WorkflowSnapshot::default(),
                provider_status_agent: &sample_provider_agent_surface(),
                detected_tomac_root: Some("/tmp/tomac".to_string()),
                multi_timeframe_clean_root: Some("/tmp/ict-engine-mtf".to_string()),
                tomac_root_placeholder: "<tomac-root>",
            },
            Vec::new(),
        );

        assert!(view.detected_paths.tomac_history_root.is_none());
        assert!(view.detected_paths.multi_timeframe_clean_root.is_none());
        assert!(view
            .commands
            .clean_multi_timeframe
            .contains("ict-engine clean-futures --root <tomac-root> --output-dir <output-dir> --multi-timeframe"));
        assert_eq!(
            view.commands.analyze,
            "ict-engine analyze --symbol <symbol> --data-root <clean-root> --state-dir <state-dir>"
        );
    }

    #[test]
    fn agent_bootstrap_profile_can_reuse_profile_path_hint_without_detected_root() {
        let view = build_agent_bootstrap_view_with_candidates(
            AgentBootstrapBuildInput {
                symbol: "NQ",
                state_dir: "/tmp/state",
                snapshot: &WorkflowSnapshot::default(),
                provider_status_agent: &sample_provider_agent_surface_with_profile(),
                detected_tomac_root: None,
                multi_timeframe_clean_root: None,
                tomac_root_placeholder: "<tomac-root>",
            },
            Vec::new(),
        );

        assert_eq!(
            view.detected_paths.tomac_history_root.as_deref(),
            Some("/tmp/tomac")
        );
        assert_eq!(
            view.detected_paths.multi_timeframe_clean_root.as_deref(),
            Some("/tmp/tomac/ict-cleaned-mtf")
        );
        assert!(view
            .commands
            .clean_multi_timeframe
            .contains("ict-engine clean-futures --root /tmp/tomac --output-dir /tmp/tomac/ict-cleaned-mtf --multi-timeframe"));
    }

    #[test]
    fn agent_workflow_status_view_surfaces_selected_profile_summary_contracts() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            current_focus_phase: "analyze_live".to_string(),
            current_focus_reason: "provider_runtime_required".to_string(),
            blocking_truth: crate::state::WorkflowBlockingTruth {
                status: "blocked".to_string(),
                reason: "provider_runtime_required".to_string(),
                next_command: "ict-engine analyze-live --symbol NQ --futures-symbol NQ=F --spot-symbol QQQ --options-symbol QQQ --futures-backend external_http_runtime --aux-backend crypto_public_runtime".to_string(),
                ..crate::state::WorkflowBlockingTruth::default()
            },
            latest_analyze: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "analyze_live".to_string(),
                phase_summary: "live_provider_runtime_pending".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
        );

        assert_eq!(value["selected_profile_id"], "thrill3r_nq_closed_loop_v1");
        assert!(value["selected_profile_summary"]
            .as_str()
            .unwrap()
            .contains("Personal NQ workflow"));
        assert!(value["selected_profile_data_contracts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .unwrap()
                .contains("Tomac cleaned multi-timeframe futures root")));
        assert!(value["selected_profile_track_statuses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .unwrap()
                .contains("options_enriched:pending:tradingview_mcp")));
    }

    #[test]
    fn agent_and_human_workflow_status_views_expose_dataset_resolution_line() {
        let snapshot = sample_human_workflow_snapshot();
        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
        );
        let human_value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
        );

        assert!(agent_value["dataset_resolution_line"]
            .as_str()
            .unwrap()
            .contains("profile_opt_in"));
        assert!(agent_value["dataset_resolution_line"]
            .as_str()
            .unwrap()
            .contains("Tomac cleaned multi-timeframe futures root"));
        assert_eq!(
            agent_value["dataset_resolution_line"],
            human_value["dataset_resolution_line"]
        );
    }

    #[test]
    fn generic_workflow_status_views_expose_zero_config_dataset_resolution_line() {
        let snapshot = sample_human_workflow_snapshot();
        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
        );

        assert!(agent_value["dataset_resolution_line"]
            .as_str()
            .unwrap()
            .contains("generic_zero_config"));
        assert!(agent_value["dataset_resolution_line"]
            .as_str()
            .unwrap()
            .contains("opt_in"));
    }

    #[test]
    fn human_workflow_status_view_exposes_dataset_resolution_line() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
        );

        assert!(value["dataset_resolution_line"]
            .as_str()
            .unwrap()
            .contains("profile_opt_in"));
        assert!(value["dataset_resolution_line"]
            .as_str()
            .unwrap()
            .contains("Tomac cleaned multi-timeframe futures root"));
    }

    #[test]
    fn workflow_status_phase_structural_node_exposes_current_blocker() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "structural-node",
        )
        .unwrap();

        assert_eq!(value["node_family"], "data_selection_gate");
        assert_eq!(value["node_label"], "user_selected_historical_data_missing");
        assert!(value["supporting_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap().contains("need user choice")));
        assert!(value["recommended_next_step"]["execution_contract"].is_null());
    }

    #[test]
    fn structural_node_prefers_active_regime_label_over_focus_reason_when_actionable() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.current_focus_reason =
            "market_policy=NQ hostile_liquidity_penalty=0.100 favorable_liquidity_bonus=0.040"
                .to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "structural-node",
        )
        .unwrap();

        assert_eq!(value["node_family"], "belief_regime_node");
        assert_eq!(value["node_label"], "trend");
        assert_eq!(value["node_id"], "NQ:belief_regime_node:trend");
    }

    #[test]
    fn structural_node_prefers_posterior_probability_key_when_active_regime_string_is_dirty() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime:
                "market_policy=NQ hostile_liquidity_penalty=0.100 favorable_liquidity_bonus=0.040"
                    .to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "structural-node",
        )
        .unwrap();

        assert_eq!(value["node_label"], "trend");
        assert_eq!(value["node_id"], "NQ:belief_regime_node:trend");
    }

    #[test]
    fn structural_node_falls_back_to_latest_analyze_anchor_when_latest_ensemble_vote_is_non_structural(
    ) {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "backtest".to_string();
        snapshot.current_focus_reason = "no_previous_run".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            pre_bayes_filtered_assignments: std::collections::BTreeMap::from([(
                "market_regime".to_string(),
                "bull".to_string(),
            )]),
            pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                "market_regime".to_string(),
                std::collections::BTreeMap::from([
                    ("bull".to_string(), 0.66),
                    ("range".to_string(), 0.24),
                    ("transition".to_string(), 0.10),
                ]),
            )]),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:non-structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "factor-research".to_string(),
            source_run_id: Some("run-non-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "observe_only".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=observe_only".to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.51,
            consensus_strength: 0.49,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: Some("fallback".to_string()),
            posterior_fingerprint: "fp-non-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "research_iteration".to_string(),
            posterior_confidence: Some(0.51),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("fallback".to_string(), 0.51),
                ("research_iteration".to_string(), 0.49),
            ]),
            posterior_evidence: vec!["objective=generic".to_string()],
        });

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "structural-node",
        )
        .unwrap();

        assert_eq!(value["node_family"], "belief_regime_node");
        assert_eq!(value["node_label"], "trend");
        assert_eq!(value["node_id"], "NQ:belief_regime_node:trend");
    }

    #[test]
    fn structural_node_uses_duration_prior_to_adjust_posterior_confidence() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.node_duration_priors.insert(
            "NQ:belief_regime_node:trend".to_string(),
            crate::state::StructuralNodeDurationPrior {
                observations: 6,
                streak_count: 3,
                weighted_streak_mass: 2.4,
                weighted_success_mass: 2.4,
                weighted_failure_mass: 0.0,
                total_streak_length: 6,
                avg_streak_length: 2.0,
                max_streak_length: 3,
                last_streak_length: 3,
                persistence_prior: 0.9,
                duration_outcome_support: 0.7727272727,
                temporal_posterior_support: 0.8618181818,
                last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
                ..crate::state::StructuralNodeDurationPrior::default()
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &structural_prior_state,
            "structural-node",
        )
        .unwrap();

        assert_eq!(value["node_id"], "NQ:belief_regime_node:trend");
        assert!(value["posterior_confidence"].as_f64().unwrap() > 0.72);
        assert_eq!(
            value["belief_posterior"].as_f64().unwrap(),
            value["posterior_confidence"].as_f64().unwrap()
        );
    }

    #[test]
    fn structural_playbook_falls_back_to_latest_analyze_anchor_when_latest_ensemble_vote_is_non_structural(
    ) {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "backtest".to_string();
        snapshot.current_focus_reason = "no_previous_run".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            pre_bayes_filtered_assignments: std::collections::BTreeMap::from([(
                "market_regime".to_string(),
                "bull".to_string(),
            )]),
            pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                "market_regime".to_string(),
                std::collections::BTreeMap::from([
                    ("bull".to_string(), 0.66),
                    ("range".to_string(), 0.24),
                    ("transition".to_string(), 0.10),
                ]),
            )]),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:non-structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "factor-research".to_string(),
            source_run_id: Some("run-non-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "observe_only".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=observe_only".to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.51,
            consensus_strength: 0.49,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: Some("fallback".to_string()),
            posterior_fingerprint: "fp-non-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "research_iteration".to_string(),
            posterior_confidence: Some(0.51),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("fallback".to_string(), 0.51),
                ("research_iteration".to_string(), 0.49),
            ]),
            posterior_evidence: vec!["objective=generic".to_string()],
        });

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "structural-playbook",
        )
        .unwrap();

        assert_eq!(value["node"]["node_id"], "NQ:belief_regime_node:trend");
        assert_eq!(
            value["branch_set"]["branches"][0]["branch_label"],
            "trend_follow_through"
        );
        assert_eq!(
            value["scenario_playbook"]["scenarios"][0]["scenario_label"],
            "trend_follow_through"
        );
    }

    #[test]
    fn structural_playbook_prefers_canonical_analyze_ensemble_surface_when_latest_vote_is_raw_analyze(
    ) {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            current_focus_phase: "analyze".to_string(),
            recommended_next_command: "ict-engine workflow-status --symbol NQ --phase human-next"
                .to_string(),
            latest_analyze: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "analyze".to_string(),
                run_id: "analyze:1".to_string(),
                pre_bayes_filtered_assignments: std::collections::BTreeMap::from([(
                    "market_regime".to_string(),
                    "trend".to_string(),
                )]),
                pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                    "market_regime".to_string(),
                    std::collections::BTreeMap::from([
                        ("trend".to_string(), 0.78),
                        ("range".to_string(), 0.14),
                        ("transition".to_string(), 0.08),
                    ]),
                )]),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_ensemble_vote: Some(EnsembleVoteRecord {
                artifact_id: "ensemble-vote:analyze:test".to_string(),
                generated_at: Utc::now(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:1".to_string()),
                provenance: RunProvenance::default(),
                dataset_comparability: DatasetComparability::default(),
                ensemble_version: "ensemble-audit-v2".to_string(),
                final_action: "execute_follow_through".to_string(),
                recommended_command: "ict-engine workflow-status --symbol NQ --phase human-next"
                    .to_string(),
                human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                    .to_string(),
                hard_block: EnsembleHardBlockArtifact::default(),
                confidence: 0.55,
                consensus_strength: 0.55,
                disagreement_flags: Vec::new(),
                executor_summaries: Vec::new(),
                split_explanations: Vec::new(),
                executor_scorecards: Vec::new(),
                executor_scorecards_source: None,
                posterior_fingerprint: "fp-raw".to_string(),
                posterior_normalization_status: "normalized".to_string(),
                posterior_active_regime: "bull".to_string(),
                posterior_confidence: Some(0.55),
                posterior_probabilities: std::collections::BTreeMap::from([
                    ("bull".to_string(), 0.55),
                    ("range".to_string(), 0.30),
                    ("transition".to_string(), 0.15),
                ]),
                posterior_evidence: vec!["raw".to_string()],
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "structural-playbook",
        )
        .unwrap();

        assert_eq!(value["node"]["node_id"], "NQ:belief_regime_node:trend");
        assert_eq!(value["node"]["posterior_confidence"], 0.78);
        assert_eq!(
            value["branch_set"]["branches"][0]["posterior_probability"],
            0.78
        );
        assert!(value["node"]["market_context"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .unwrap()
                .contains("posterior_active_regime=trend")));
    }

    #[test]
    fn workflow_status_phase_structural_playbook_surfaces_selected_profile_contracts() {
        let snapshot = WorkflowSnapshot::default();
        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
            "structural-playbook",
        )
        .unwrap();

        assert_eq!(value["selected_profile_id"], "thrill3r_nq_closed_loop_v1");
        assert!(value["selected_profile_data_contracts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .unwrap()
                .contains("Tomac cleaned multi-timeframe futures root")));
        assert!(value["path_plan"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .all(|path| path.get("path_id").is_some()));
    }

    #[test]
    fn workflow_status_phase_structural_playbook_exposes_recommended_path_bundle() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-playbook",
        )
        .unwrap();

        assert_eq!(value["recommended_path_bundle"]["rank"], 1);
        assert!(value["recommended_path_bundle"]["why_this_path"]
            .as_str()
            .unwrap()
            .contains("posterior"));
        assert_eq!(
            value["recommended_next_step"]["execution_contract"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert_eq!(
            value["recommended_next_step"]["execution_contract"]["candidate_set_id"],
            value["recommended_path_bundle"]["candidate_set_id"]
        );
        assert_eq!(
            value["recommended_next_step"]["execution_contract"]["selected_path_probability"],
            value["recommended_path_bundle"]["selected_path_probability"]
        );
    }

    #[test]
    fn workflow_status_phase_structural_branches_use_posterior_probabilities() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "structural-branches",
        )
        .unwrap();

        assert_eq!(value["branches"].as_array().unwrap().len(), 3);
        assert_eq!(value["branches"][0]["branch_label"], "trend_follow_through");
        assert_eq!(value["branches"][0]["posterior_probability"], 0.72);
    }

    #[test]
    fn structural_branches_use_transition_priors_from_latest_structural_feedback() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_update = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "update".to_string(),
            structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-prev".to_string(),
                recommended_at: "2026-04-30T01:00:00Z".to_string(),
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
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.branch_transition_priors.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
            crate::state::StructuralBranchTransitionPrior {
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:trend".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 3,
                weighted_observation_mass: 2.4,
                wins: 2,
                losses: 1,
                invalidated: 0,
                transition_prior: 0.8,
                transition_outcome_support: 0.56,
                temporal_posterior_support: 0.728,
                weighted_success_mass: 1.6,
                weighted_failure_mass: 1.25,
                last_recommended_at: Some("2026-04-30T02:00:00Z".to_string()),
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &structural_prior_state,
            "structural-branches",
        )
        .unwrap();

        let branch = value["branches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["branch_label"] == "transition_confirmation")
            .expect("transition branch");
        assert_eq!(branch["transition_prior"], 0.8);
        assert_eq!(branch["transition_weighted_observation_mass"], 2.4);
        assert_eq!(branch["transition_outcome_support"], 0.56);
        assert_eq!(branch["transition_temporal_posterior_support"], 0.728);
        assert!(branch["prior_probability"].as_f64().unwrap() > 0.6);
        assert!(branch["posterior_probability"].as_f64().unwrap() > 0.10);
    }

    #[test]
    fn structural_branches_prefer_persisted_temporal_state_values_over_raw_transition_prior_fields()
    {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_update = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "update".to_string(),
            structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-prev".to_string(),
                recommended_at: "2026-04-30T01:00:00Z".to_string(),
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
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        let key = "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string();
        structural_prior_state.branch_transition_priors.insert(
            key.clone(),
            crate::state::StructuralBranchTransitionPrior {
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:trend".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 3,
                weighted_observation_mass: 2.4,
                wins: 2,
                losses: 1,
                invalidated: 0,
                transition_prior: 0.8,
                transition_outcome_support: 0.56,
                temporal_posterior_support: 0.728,
                weighted_success_mass: 1.6,
                weighted_failure_mass: 1.25,
                last_recommended_at: Some("2026-04-30T02:00:00Z".to_string()),
            },
        );
        structural_prior_state.branch_temporal_posteriors.insert(
            key,
            crate::state::StructuralBranchTemporalPosteriorState {
                transition_key: "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:trend:transition_confirmation".to_string(),
                observations: 3,
                weighted_observation_mass: 1.1,
                transition_prior: 0.8,
                transition_outcome_support: 0.22,
                temporal_posterior_support: 0.33,
                posterior_multiplier: 0.61,
                normalized_transition_posterior: 0.8,
                summary_line: "transition_mass=1.100 transition_support=0.220 transition_temporal=0.330 multiplier=0.610".to_string(),
                last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &structural_prior_state,
            "structural-branches",
        )
        .unwrap();

        let branch = value["branches"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["branch_label"] == "transition_confirmation")
            .expect("transition branch");
        assert_eq!(branch["transition_weighted_observation_mass"], 1.1);
        assert_eq!(branch["transition_outcome_support"], 0.22);
        assert_eq!(branch["transition_temporal_posterior_support"], 0.33);
    }

    #[test]
    fn workflow_status_phase_ensemble_vote_prefers_canonical_analyze_regime_surface() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            latest_analyze: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "analyze".to_string(),
                run_id: "analyze:1".to_string(),
                pre_bayes_filtered_assignments: std::collections::BTreeMap::from([(
                    "market_regime".to_string(),
                    "trend".to_string(),
                )]),
                pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                    "market_regime".to_string(),
                    std::collections::BTreeMap::from([
                        ("trend".to_string(), 0.78),
                        ("range".to_string(), 0.14),
                        ("transition".to_string(), 0.08),
                    ]),
                )]),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_ensemble_vote: Some(EnsembleVoteRecord {
                artifact_id: "ensemble-vote:analyze:test".to_string(),
                generated_at: Utc::now(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:1".to_string()),
                provenance: RunProvenance::default(),
                dataset_comparability: DatasetComparability::default(),
                ensemble_version: "ensemble-audit-v2".to_string(),
                final_action: "execute_follow_through".to_string(),
                recommended_command: "ict-engine workflow-status --symbol NQ --phase human-next"
                    .to_string(),
                human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                    .to_string(),
                hard_block: EnsembleHardBlockArtifact::default(),
                confidence: 0.55,
                consensus_strength: 0.55,
                disagreement_flags: Vec::new(),
                executor_summaries: Vec::new(),
                split_explanations: Vec::new(),
                executor_scorecards: Vec::new(),
                executor_scorecards_source: None,
                posterior_fingerprint: "fp-raw".to_string(),
                posterior_normalization_status: "normalized".to_string(),
                posterior_active_regime: "bull".to_string(),
                posterior_confidence: Some(0.55),
                posterior_probabilities: std::collections::BTreeMap::from([
                    ("bull".to_string(), 0.55),
                    ("range".to_string(), 0.30),
                    ("transition".to_string(), 0.15),
                ]),
                posterior_evidence: vec!["raw".to_string()],
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "ensemble-vote",
        )
        .unwrap();

        assert_eq!(value["posterior_active_regime"], "trend");
        assert_eq!(value["posterior_confidence"], 0.78);
        assert_eq!(value["posterior_probabilities"]["trend"], 0.78);
    }

    #[test]
    fn workflow_status_phase_ensemble_vote_prefers_canonical_research_regime_surface() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            latest_research: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "research".to_string(),
                run_id: "research:1".to_string(),
                canonical_structural_active_regime: Some("range".to_string()),
                canonical_structural_confidence: Some(0.61),
                canonical_structural_probabilities: std::collections::BTreeMap::from([
                    ("trend".to_string(), 0.21),
                    ("range".to_string(), 0.61),
                    ("transition".to_string(), 0.18),
                ]),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_ensemble_vote: Some(EnsembleVoteRecord {
                artifact_id: "ensemble-vote:research:test".to_string(),
                generated_at: Utc::now(),
                symbol: "NQ".to_string(),
                source_phase: "research".to_string(),
                source_run_id: Some("research:1".to_string()),
                provenance: RunProvenance::default(),
                dataset_comparability: DatasetComparability::default(),
                ensemble_version: "ensemble-audit-v2".to_string(),
                final_action: "observe".to_string(),
                recommended_command: "ict-engine workflow-status --symbol NQ --phase human-next"
                    .to_string(),
                human_next_triage: "hard_blocked=false ensemble_action=observe".to_string(),
                hard_block: EnsembleHardBlockArtifact::default(),
                confidence: 0.20,
                consensus_strength: 0.20,
                disagreement_flags: Vec::new(),
                executor_summaries: Vec::new(),
                split_explanations: Vec::new(),
                executor_scorecards: Vec::new(),
                executor_scorecards_source: None,
                posterior_fingerprint: "fp-raw".to_string(),
                posterior_normalization_status: "normalized".to_string(),
                posterior_active_regime: "bull".to_string(),
                posterior_confidence: Some(0.20),
                posterior_probabilities: std::collections::BTreeMap::from([
                    ("bull".to_string(), 0.20),
                    ("range".to_string(), 0.60),
                    ("transition".to_string(), 0.20),
                ]),
                posterior_evidence: vec!["raw".to_string()],
            }),
            ..WorkflowSnapshot::default()
        };

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            "ensemble-vote",
        )
        .unwrap();

        assert_eq!(value["posterior_active_regime"], "range");
        assert_eq!(value["posterior_confidence"], 0.61);
        assert_eq!(value["posterior_probabilities"]["range"], 0.61);
    }

    #[test]
    fn workflow_status_phase_structural_feedback_template_exposes_stable_ids() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface_with_profile(),
            &[],
            "structural-feedback-template",
        )
        .unwrap();

        assert!(value["recommendation_id"]
            .as_str()
            .unwrap()
            .contains("structural-feedback"));
        assert!(value["node_id"].as_str().unwrap().contains("NQ"));
        assert!(value["branch_id"]
            .as_str()
            .unwrap()
            .contains("choose_historical_dataset"));
        assert!(value["candidate_set_id"]
            .as_str()
            .unwrap()
            .starts_with("structural-candidates:NQ:"));
        assert_eq!(value["candidate_set_size"], 1);
        assert!(value["selected_path_probability"].as_f64().unwrap() > 0.0);
        assert!(value["feedback_fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field["field_id"] == "realized_outcome"));
        assert!(value["allowed_outcomes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap() == "invalidated"));
    }

    #[test]
    fn workflow_status_phase_structural_path_history_aggregates_feedback() {
        let mut snapshot = sample_human_workflow_snapshot();
        if let Some(update) = snapshot.latest_update.as_mut() {
            update.structural_feedback = sample_structural_feedback_history()[1]
                .structural_feedback
                .clone();
        }
        let history = sample_structural_feedback_history();
        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-path-history",
        )
        .unwrap();

        assert_eq!(value["summary"]["total_records"], 2);
        assert_eq!(value["summary"]["distinct_paths"], 1);
        assert_eq!(
            value["paths"][0]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert_eq!(value["paths"][0]["wins"], 1);
        assert_eq!(value["paths"][0]["invalidated"], 1);
    }

    #[test]
    fn workflow_status_phase_structural_path_history_surfaces_off_policy_exposure() {
        let mut snapshot = sample_human_workflow_snapshot();
        if let Some(update) = snapshot.latest_update.as_mut() {
            update.structural_feedback = sample_structural_feedback_history()[1]
                .structural_feedback
                .clone();
        }
        let mut history = sample_structural_feedback_history();
        let mut skipped = history[0].clone();
        skipped.run_id = Some("run-not-followed".to_string());
        skipped.realized_outcome = "not_followed".to_string();
        skipped.pnl = 0.0;
        if let Some(refs) = skipped.structural_feedback.as_mut() {
            refs.recommendation_id = "structural-feedback:NQ:node:path-skipped".to_string();
            refs.recommended_at = "2026-04-29T00:10:00Z".to_string();
            refs.followed_path = false;
            refs.exit_reason = Some("skipped".to_string());
        }
        history.push(skipped);

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-path-history",
        )
        .unwrap();

        assert_eq!(value["paths"][0]["total_records"], 3);
        assert_eq!(value["paths"][0]["followed_count"], 2);
        assert_eq!(value["paths"][0]["not_followed"], 1);
        assert_eq!(value["paths"][0]["execution_propensity"], 0.6);
        assert_eq!(value["paths"][0]["off_policy_exposure_rate"], 0.4);
    }

    #[test]
    fn workflow_status_phase_structural_path_outcome_summary_is_token_friendly() {
        let history = sample_structural_feedback_history();
        let value = build_workflow_status_phase_value(
            &WorkflowSnapshot::default(),
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-path-outcome-summary",
        )
        .unwrap();

        assert_eq!(value["total_records"], 2);
        assert_eq!(value["distinct_paths"], 1);
    }

    #[test]
    fn structural_playbook_uses_history_to_raise_path_prior() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-paths",
        )
        .unwrap();

        assert_eq!(value["paths"][0]["historical_total_records"], 3);
        assert!(value["paths"][0]["path_prior"].as_f64().unwrap() > 0.5);
        assert!(
            value["paths"][0]["composite_preference_score"]
                .as_f64()
                .unwrap()
                >= value["paths"][0]["path_posterior"].as_f64().unwrap() * 0.7
        );
    }

    #[test]
    fn structural_playbook_uses_persisted_structural_prior_state_for_path_prior() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary".to_string(),
            crate::state::StructuralPriorStats {
                observations: 4,
                followed_count: 4,
                wins: 3,
                losses: 0,
                breakevens: 1,
                invalidated: 0,
                abandoned: 0,
                not_followed: 0,
                avg_pnl: 0.028,
                weighted_followed_mass: 4.0,
                weighted_success_mass: 3.5,
                weighted_failure_mass: 0.5,
                weighted_invalidation_mass: 0.0,
                weighted_exposure_mass: 4.0,
                weighted_not_followed_mass: 0.0,
                smoothed_prior: 0.75,
                execution_propensity: 0.8333333333,
                ips_weight: 1.2,
                counterfactual_success_mass: 4.2,
                counterfactual_failure_mass: 0.6,
                counterfactual_reward_prior: 0.7647058824,
                off_policy_adjusted_prior: 0.6372549020,
                source_panel_summaries: std::collections::BTreeMap::new(),
                last_offline_seed_source: None,
                ..Default::default()
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &structural_prior_state,
            "structural-paths",
        )
        .unwrap();

        assert_eq!(value["paths"][0]["historical_total_records"], 4);
        assert_eq!(value["paths"][0]["historical_followed_count"], 4);
        assert_eq!(value["paths"][0]["path_prior"], 0.75);
    }

    #[test]
    fn structural_playbook_uses_history_to_adjust_branch_and_scenario_scores() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let branches = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-branches",
        )
        .unwrap();
        let scenarios = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-scenarios",
        )
        .unwrap();

        assert_eq!(branches["branches"][0]["historical_total_records"], 3);
        assert_eq!(branches["branches"][0]["historical_followed_count"], 3);
        assert!(
            branches["branches"][0]["prior_probability"]
                .as_f64()
                .unwrap()
                < branches["branches"][0]["posterior_probability"]
                    .as_f64()
                    .unwrap()
        );
        assert_eq!(scenarios["scenarios"][0]["historical_total_records"], 3);
        assert_eq!(scenarios["scenarios"][0]["historical_followed_count"], 3);
        assert!(
            scenarios["scenarios"][0]["composite_scenario_score"]
                .as_f64()
                .unwrap()
                < scenarios["scenarios"][0]["posterior_probability"]
                    .as_f64()
                    .unwrap()
        );
    }

    #[test]
    fn workflow_status_phase_structural_branch_history_aggregates_feedback() {
        let history = sample_structural_feedback_history();
        let value = build_workflow_status_phase_value(
            &WorkflowSnapshot::default(),
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-branch-history",
        )
        .unwrap();

        assert_eq!(value["summary"]["total_records"], 2);
        assert_eq!(value["summary"]["distinct_entities"], 1);
        assert_eq!(
            value["branches"][0]["branch_id"],
            "NQ:belief_regime_node:trend:trend_follow_through"
        );
        assert_eq!(value["branches"][0]["wins"], 1);
    }

    #[test]
    fn workflow_status_phase_structural_experience_priors_tracks_current_lineage() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        if let Some(analyze) = snapshot.latest_analyze.as_mut() {
            analyze.structural_feedback = sample_structural_feedback_history()[1]
                .structural_feedback
                .clone();
        }
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary".to_string(),
            crate::state::StructuralPriorStats {
                observations: 3,
                followed_count: 3,
                wins: 2,
                losses: 1,
                breakevens: 0,
                invalidated: 0,
                abandoned: 0,
                not_followed: 0,
                avg_pnl: 0.01,
                weighted_followed_mass: 2.05,
                weighted_success_mass: 1.30,
                weighted_failure_mass: 0.75,
                weighted_invalidation_mass: 0.0,
                weighted_exposure_mass: 3.0,
                weighted_not_followed_mass: 0.0,
                smoothed_prior: 0.5483870968,
                execution_propensity: 0.8,
                ips_weight: 1.25,
                counterfactual_success_mass: 1.625,
                counterfactual_failure_mass: 0.9375,
                counterfactual_reward_prior: 0.5753424658,
                off_policy_adjusted_prior: 0.4602739726,
                policy_weighted_observation_mass: 2.05,
                behavior_policy_probability: 0.42,
                behavior_policy_probability_squared_mass: 0.38,
                behavior_policy_probability_variance: 0.018,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_probability_brier_score: 0.15625,
                target_policy_probability_calibration_error: 0.375,
                snips_weight_mass: 4.5,
                snips_weight_squared_mass: 10.125,
                snips_effective_sample_size: 2.0,
                snips_reward_mass: 2.7,
                snips_reward_prior: 0.6,
                doubly_robust_reward_mass: 1.23,
                doubly_robust_reward_prior: 0.6,
                target_policy_calibration_weight: 0.6666666667,
                target_policy_reward_prior: 0.5827956989,
                target_policy_variance_penalty: 0.284676,
                target_policy_reward_lower_bound: 0.2981196989,
                delayed_reward_elapsed_feedback_count: 3,
                delayed_reward_elapsed_hours_at_risk: 6.0,
                delayed_reward_avg_elapsed_hours: 2.0,
                delayed_reward_resolution_hazard_per_hour: 3.0 / 6.0,
                delayed_reward_expected_resolution_hours: 2.0,
                delayed_reward_survival_probability_1h: (-0.5_f64).exp(),
                delayed_reward_survival_probability_4h: (-2.0_f64).exp(),
                delayed_reward_survival_probability_24h: (-12.0_f64).exp(),
                delayed_reward_success_hazard_per_hour: 2.0 / 6.0,
                delayed_reward_failure_hazard_per_hour: 1.0 / 6.0,
                delayed_reward_success_cumulative_incidence_4h: (2.0 / 3.0)
                    * (1.0 - (-2.0_f64).exp()),
                delayed_reward_failure_cumulative_incidence_4h: (1.0 / 3.0)
                    * (1.0 - (-2.0_f64).exp()),
                delayed_reward_resolution_horizon_1h_count: 3,
                delayed_reward_resolution_within_1h_count: 1,
                delayed_reward_resolution_probability_1h: 2.0 / 5.0,
                delayed_reward_resolution_horizon_4h_count: 3,
                delayed_reward_resolution_within_4h_count: 2,
                delayed_reward_resolution_probability_4h: 3.0 / 5.0,
                delayed_reward_resolution_horizon_24h_count: 3,
                delayed_reward_resolution_within_24h_count: 3,
                delayed_reward_resolution_probability_24h: 4.0 / 5.0,
                source_panel_summaries: std::collections::BTreeMap::from([
                    (
                        "analyze".to_string(),
                        crate::state::StructuralPriorSourceSummary {
                            observations: 1,
                            followed_count: 1,
                            wins: 1,
                            losses: 0,
                            breakevens: 0,
                            invalidated: 0,
                            abandoned: 0,
                            not_followed: 0,
                            avg_pnl: 0.01,
                            weighted_followed_mass: 0.30,
                            weighted_success_mass: 0.30,
                            weighted_failure_mass: 0.0,
                            weighted_invalidation_mass: 0.0,
                            weighted_exposure_mass: 1.0,
                            weighted_not_followed_mass: 0.0,
                            smoothed_prior: 0.5652173913,
                            execution_propensity: 0.6666666667,
                            ips_weight: 1.5,
                            counterfactual_success_mass: 0.45,
                            counterfactual_failure_mass: 0.0,
                            counterfactual_reward_prior: 0.5918367347,
                            off_policy_adjusted_prior: 0.3945578231,
                            last_tempering_coefficient: None,
                            last_power_prior_contribution: None,
                            last_recommendation_id: Some("rec-analyze".to_string()),
                            last_recommended_at: Some("2026-04-30T00:00:00Z".to_string()),
                            last_note: Some("analyze_run_structural_prior_seed".to_string()),
                            ..Default::default()
                        },
                    ),
                    (
                        "backtest".to_string(),
                        crate::state::StructuralPriorSourceSummary {
                            observations: 2,
                            followed_count: 2,
                            wins: 1,
                            losses: 1,
                            breakevens: 0,
                            invalidated: 0,
                            abandoned: 0,
                            not_followed: 0,
                            avg_pnl: 0.01,
                            weighted_followed_mass: 1.50,
                            weighted_success_mass: 0.75,
                            weighted_failure_mass: 0.75,
                            weighted_invalidation_mass: 0.0,
                            weighted_exposure_mass: 2.0,
                            weighted_not_followed_mass: 0.0,
                            smoothed_prior: 0.5,
                            execution_propensity: 0.75,
                            ips_weight: 1.3333333333,
                            counterfactual_success_mass: 1.0,
                            counterfactual_failure_mass: 1.0,
                            counterfactual_reward_prior: 0.5,
                            off_policy_adjusted_prior: 0.375,
                            last_tempering_coefficient: None,
                            last_power_prior_contribution: None,
                            last_recommendation_id: Some("rec-backtest".to_string()),
                            last_recommended_at: Some("2026-04-30T01:00:00Z".to_string()),
                            last_note: Some("backtest_run_structural_prior_seed".to_string()),
                            ..Default::default()
                        },
                    ),
                ]),
                last_offline_seed_source: Some("backtest".to_string()),
                ..crate::state::StructuralPriorStats::default()
            },
        );
        structural_prior_state
            .target_policy_context_posteriors
            .insert(
                "NQ:manipulation_expansion:bull".to_string(),
                crate::state::StructuralTargetPolicyContextPosterior {
                    observations: 3,
                    weighted_observation_mass: 2.05,
                    success_mass: 1.30,
                    failure_mass: 0.75,
                    behavior_policy_probability: 0.42,
                    behavior_policy_probability_squared_mass: 0.38,
                    behavior_policy_probability_variance: 0.018,
                    learned_target_policy_probability: 2.30 / 4.05,
                    learned_target_policy_probability_lower_bound: 0.32,
                    learned_target_policy_probability_confidence: 0.41,
                    calibrated_target_policy_probability: (2.30 / 4.05) * 0.41 + 0.42 * 0.59,
                    calibrated_target_policy_probability_lower_bound: 0.32 * 0.41
                        + (0.42 - (0.018_f64 / 3.05).sqrt()) * 0.59,
                    target_policy_probability_brier_score: 0.15625,
                    target_policy_probability_calibration_error: 0.375,
                    last_recommendation_id: Some("rec-context".to_string()),
                    ..crate::state::StructuralTargetPolicyContextPosterior::default()
                },
            );
        structural_prior_state.node_duration_priors.insert(
            "NQ:belief_regime_node:trend".to_string(),
            crate::state::StructuralNodeDurationPrior {
                observations: 3,
                streak_count: 2,
                weighted_streak_mass: 1.85,
                weighted_success_mass: 1.85,
                weighted_failure_mass: 0.0,
                total_streak_length: 3,
                avg_streak_length: 1.5,
                max_streak_length: 2,
                last_streak_length: 1,
                persistence_prior: 0.6,
                duration_distribution_entropy: std::f64::consts::LN_2,
                empirical_duration_survival: 1.0,
                empirical_duration_completion_hazard: 0.5405405405,
                bocpd_duration_surprise: 0.0,
                bocpd_break_probability: 0.36,
                bocpd_continue_probability: 0.64,
                duration_outcome_support: 0.7407407407,
                temporal_posterior_support: 0.6422222222,
                last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
                ..crate::state::StructuralNodeDurationPrior::default()
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
            "structural-experience-priors",
        )
        .unwrap();

        assert_eq!(value["symbol"], "NQ");
        assert_eq!(
            value["path"]["entity_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert_eq!(value["path"]["historical_total_records"], 3);
        assert!(value["path"]["experience_prior"].as_f64().unwrap() > 0.5);
        assert_eq!(value["path"]["source_panel_count"], 2);
        assert_eq!(value["path"]["last_offline_seed_source"], "backtest");
        assert_eq!(value["path"]["dominant_source_panel"], "backtest");
        assert!(value["path"]["dominant_source_share"].as_f64().unwrap() > 0.80);
        assert_eq!(value["path"]["dominant_source_prior"], 0.5);
        assert_eq!(value["path"]["execution_propensity"], 0.8);
        assert_eq!(value["path"]["ips_weight"], 1.25);
        assert_eq!(value["path"]["counterfactual_reward_prior"], 0.5753424658);
        assert_eq!(value["path"]["off_policy_adjusted_prior"], 0.4602739726);
        assert_eq!(
            value["target_policy_contexts"][0]["context_key"],
            "NQ:manipulation_expansion:bull"
        );
        assert_eq!(value["target_policy_contexts"][0]["observations"], 3);
        assert_eq!(
            value["target_policy_contexts"][0]["learned_target_policy_probability"],
            2.30 / 4.05
        );
        assert_eq!(
            value["target_policy_contexts"][0]["learned_target_policy_probability_lower_bound"],
            0.32
        );
        assert_eq!(
            value["target_policy_contexts"][0]["calibrated_target_policy_probability"],
            (2.30 / 4.05) * 0.41 + 0.42 * 0.59
        );
        assert_eq!(
            value["target_policy_contexts"][0]["calibrated_target_policy_probability_lower_bound"],
            0.32 * 0.41 + (0.42 - (0.018_f64 / 3.05).sqrt()) * 0.59
        );
        assert_eq!(
            value["target_policy_contexts"][0]["target_policy_probability_brier_score"],
            0.15625
        );
        assert_eq!(value["path"]["behavior_policy_probability"], 0.42);
        assert_eq!(value["path"]["behavior_policy_probability_variance"], 0.018);
        assert_eq!(value["path"]["target_policy_probability_confidence"], 0.57);
        assert_eq!(value["path"]["target_policy_probability_lower_bound"], 0.31);
        assert_eq!(
            value["path"]["target_policy_probability_brier_score"],
            0.15625
        );
        assert_eq!(
            value["path"]["target_policy_probability_calibration_error"],
            0.375
        );
        assert_eq!(value["path"]["snips_weight_mass"], 4.5);
        assert_eq!(value["path"]["snips_weight_squared_mass"], 10.125);
        assert_eq!(value["path"]["snips_effective_sample_size"], 2.0);
        assert_eq!(value["path"]["snips_reward_prior"], 0.6);
        assert_eq!(value["path"]["doubly_robust_reward_prior"], 0.6);
        assert_eq!(
            value["path"]["target_policy_calibration_weight"],
            0.6666666667
        );
        assert_eq!(value["path"]["target_policy_reward_prior"], 0.5827956989);
        assert_eq!(value["path"]["target_policy_variance_penalty"], 0.284676);
        assert_eq!(
            value["path"]["target_policy_reward_lower_bound"],
            0.2981196989
        );
        assert_eq!(value["path"]["delayed_reward_resolution_probability"], 0.8);
        assert_eq!(value["path"]["delayed_reward_censoring_probability"], 0.2);
        assert!(
            (value["path"]["censoring_adjusted_reward_prior"]
                .as_f64()
                .unwrap()
                - (0.5827956989 * 0.8 + 0.5483870968 * 0.2))
                .abs()
                < 1e-9
        );
        assert!(
            (value["path"]["censoring_adjusted_reward_lower_bound"]
                .as_f64()
                .unwrap()
                - (0.2981196989 * 0.8 + 0.5483870968 * 0.5 * 0.2))
                .abs()
                < 1e-9
        );
        let expected_competing_risks: [f64; 4] = [3.0 / 7.0, 2.0 / 7.0, 1.0 / 7.0, 1.0 / 7.0];
        let expected_competing_risk_entropy: f64 = expected_competing_risks
            .iter()
            .map(|risk| -*risk * (*risk).ln())
            .sum();
        assert_eq!(
            value["path"]["delayed_reward_success_competing_risk"],
            expected_competing_risks[0]
        );
        assert_eq!(
            value["path"]["delayed_reward_failure_competing_risk"],
            expected_competing_risks[1]
        );
        assert_eq!(
            value["path"]["delayed_reward_invalidation_competing_risk"],
            expected_competing_risks[2]
        );
        assert_eq!(
            value["path"]["delayed_reward_abandonment_competing_risk"],
            expected_competing_risks[3]
        );
        assert!(
            (value["path"]["delayed_reward_competing_risk_entropy"]
                .as_f64()
                .unwrap()
                - expected_competing_risk_entropy)
                .abs()
                < 1e-9
        );
        assert_eq!(value["path"]["delayed_reward_elapsed_feedback_count"], 3);
        assert_eq!(value["path"]["delayed_reward_elapsed_hours_at_risk"], 6.0);
        assert_eq!(value["path"]["delayed_reward_avg_elapsed_hours"], 2.0);
        assert_eq!(
            value["path"]["delayed_reward_resolution_hazard_per_hour"],
            3.0 / 6.0
        );
        assert_eq!(
            value["path"]["delayed_reward_expected_resolution_hours"],
            2.0
        );
        assert_eq!(
            value["path"]["delayed_reward_survival_probability_1h"],
            (-0.5_f64).exp()
        );
        assert_eq!(
            value["path"]["delayed_reward_survival_probability_4h"],
            (-2.0_f64).exp()
        );
        assert_eq!(
            value["path"]["delayed_reward_survival_probability_24h"],
            (-12.0_f64).exp()
        );
        assert_eq!(
            value["path"]["delayed_reward_success_hazard_per_hour"],
            2.0 / 6.0
        );
        assert_eq!(
            value["path"]["delayed_reward_failure_hazard_per_hour"],
            1.0 / 6.0
        );
        assert_eq!(
            value["path"]["delayed_reward_success_cumulative_incidence_4h"],
            (2.0 / 3.0) * (1.0 - (-2.0_f64).exp())
        );
        assert_eq!(
            value["path"]["delayed_reward_failure_cumulative_incidence_4h"],
            (1.0 / 3.0) * (1.0 - (-2.0_f64).exp())
        );
        assert!(value["path"]
            .get("delayed_reward_invalidation_cumulative_incidence_4h")
            .is_none());
        assert!(value["path"]
            .get("delayed_reward_abandonment_cumulative_incidence_4h")
            .is_none());
        assert!(value["path"]
            .get("delayed_reward_invalidation_hazard_per_hour")
            .is_none());
        assert_eq!(
            value["path"]["delayed_reward_resolution_horizon_1h_count"],
            3
        );
        assert_eq!(
            value["path"]["delayed_reward_resolution_within_1h_count"],
            1
        );
        assert_eq!(
            value["path"]["delayed_reward_resolution_probability_1h"],
            2.0 / 5.0
        );
        assert_eq!(
            value["path"]["delayed_reward_resolution_horizon_4h_count"],
            3
        );
        assert_eq!(
            value["path"]["delayed_reward_resolution_within_4h_count"],
            2
        );
        assert_eq!(
            value["path"]["delayed_reward_resolution_probability_4h"],
            3.0 / 5.0
        );
        assert_eq!(
            value["path"]["delayed_reward_resolution_horizon_24h_count"],
            3
        );
        assert_eq!(
            value["path"]["delayed_reward_resolution_within_24h_count"],
            3
        );
        assert_eq!(
            value["path"]["delayed_reward_resolution_probability_24h"],
            4.0 / 5.0
        );
        assert_eq!(value["path"]["matured_feedback_count"], 3);
        assert_eq!(value["path"]["unresolved_feedback_count"], 0);
        assert_eq!(value["path"]["maturity_coverage"], 1.0);
        assert_eq!(value["path"]["censoring_rate"], 0.0);
        assert_eq!(value["node"]["duration_streak_count"], 2);
        assert_eq!(value["node"]["duration_avg_streak_length"], 1.5);
        assert_eq!(value["node"]["duration_persistence_prior"], 0.6);
        assert_eq!(value["node"]["duration_weighted_streak_mass"], 1.85);
        assert_eq!(value["node"]["duration_outcome_support"], 0.7407407407);
        assert_eq!(
            value["node"]["duration_temporal_posterior_support"],
            0.6422222222
        );
        assert_eq!(
            value["branch"]["entity_id"],
            "NQ:belief_regime_node:trend:trend_follow_through"
        );
        assert!(value["branch"]["composite_score"].as_f64().unwrap() > 0.6);
        assert_eq!(
            value["recommended_next_step"]["execution_contract"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );

        let blocked_value = build_workflow_status_phase_value_with_structural_prior_state(
            &sample_human_workflow_snapshot(),
            &[],
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
            "structural-experience-priors",
        )
        .unwrap();
        assert!(blocked_value["recommended_next_step"]["execution_contract"].is_null());
    }

    #[test]
    fn structural_experience_prior_surface_prefers_panel_derived_prior_over_stale_aggregate_prior()
    {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary".to_string(),
            crate::state::StructuralPriorStats {
                observations: 3,
                followed_count: 3,
                wins: 2,
                losses: 1,
                breakevens: 0,
                invalidated: 0,
                abandoned: 0,
                not_followed: 0,
                avg_pnl: 0.01,
                weighted_followed_mass: 1.8,
                weighted_success_mass: 0.9,
                weighted_failure_mass: 0.9,
                weighted_invalidation_mass: 0.0,
                weighted_exposure_mass: 3.0,
                weighted_not_followed_mass: 0.0,
                smoothed_prior: 0.95,
                execution_propensity: 0.8,
                ips_weight: 1.25,
                counterfactual_success_mass: 1.125,
                counterfactual_failure_mass: 1.125,
                counterfactual_reward_prior: 0.5,
                off_policy_adjusted_prior: 0.4,
                source_panel_summaries: std::collections::BTreeMap::from([
                    (
                        "analyze".to_string(),
                        crate::state::StructuralPriorSourceSummary {
                            observations: 1,
                            followed_count: 1,
                            wins: 1,
                            losses: 0,
                            breakevens: 0,
                            invalidated: 0,
                            abandoned: 0,
                            not_followed: 0,
                            avg_pnl: 0.01,
                            weighted_followed_mass: 0.3,
                            weighted_success_mass: 0.3,
                            weighted_failure_mass: 0.0,
                            weighted_invalidation_mass: 0.0,
                            weighted_exposure_mass: 1.0,
                            weighted_not_followed_mass: 0.0,
                            smoothed_prior: 0.5652173913,
                            execution_propensity: 0.6666666667,
                            ips_weight: 1.5,
                            counterfactual_success_mass: 0.45,
                            counterfactual_failure_mass: 0.0,
                            counterfactual_reward_prior: 0.5918367347,
                            off_policy_adjusted_prior: 0.3945578231,
                            last_tempering_coefficient: None,
                            last_power_prior_contribution: None,
                            last_recommendation_id: None,
                            last_recommended_at: None,
                            last_note: None,
                            ..Default::default()
                        },
                    ),
                    (
                        "backtest".to_string(),
                        crate::state::StructuralPriorSourceSummary {
                            observations: 2,
                            followed_count: 2,
                            wins: 1,
                            losses: 1,
                            breakevens: 0,
                            invalidated: 0,
                            abandoned: 0,
                            not_followed: 0,
                            avg_pnl: 0.01,
                            weighted_followed_mass: 1.5,
                            weighted_success_mass: 0.6,
                            weighted_failure_mass: 0.9,
                            weighted_invalidation_mass: 0.0,
                            weighted_exposure_mass: 2.0,
                            weighted_not_followed_mass: 0.0,
                            smoothed_prior: 0.4444444444,
                            execution_propensity: 0.75,
                            ips_weight: 1.3333333333,
                            counterfactual_success_mass: 0.8,
                            counterfactual_failure_mass: 1.2,
                            counterfactual_reward_prior: 0.45,
                            off_policy_adjusted_prior: 0.3375,
                            last_tempering_coefficient: None,
                            last_power_prior_contribution: None,
                            last_recommendation_id: None,
                            last_recommended_at: None,
                            last_note: None,
                            ..Default::default()
                        },
                    ),
                ]),
                last_offline_seed_source: Some("backtest".to_string()),
                ..Default::default()
            },
        );
        structural_prior_state.source_reliability_posteriors.insert(
            "analyze".to_string(),
            crate::state::StructuralSourceReliabilityPosterior {
                source_label: "analyze".to_string(),
                observations: 1,
                weighted_observation_mass: 1.0,
                posterior_reliability: 1.0,
                ..crate::state::StructuralSourceReliabilityPosterior::default()
            },
        );
        structural_prior_state.source_reliability_posteriors.insert(
            "backtest".to_string(),
            crate::state::StructuralSourceReliabilityPosterior {
                source_label: "backtest".to_string(),
                observations: 1,
                weighted_observation_mass: 1.0,
                posterior_reliability: 0.1,
                ..crate::state::StructuralSourceReliabilityPosterior::default()
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &structural_prior_state,
            "structural-experience-priors",
        )
        .unwrap();

        let prior = value["path"]["experience_prior"].as_f64().unwrap();
        assert!(prior < 0.57);
        assert!(prior > 0.54);
    }

    #[test]
    fn workflow_status_phase_structural_validation_summarizes_holdout_and_replay() {
        let mut snapshot = sample_human_workflow_snapshot();
        if let Some(update) = snapshot.latest_update.as_mut() {
            update.structural_feedback = sample_structural_feedback_history()[1]
                .structural_feedback
                .clone();
        }
        let history = sample_structural_feedback_history();

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-validation",
        )
        .unwrap();

        assert!(
            value["source_reliability"]["status"]
                .as_str()
                .unwrap()
                .len()
                > 3
        );
        assert!(value["source_reliability"].get("holdout_status").is_some());
        assert!(value["source_reliability"]
            .get("holdout_observation_coverage")
            .is_some());
        assert!(value["source_reliability"].get("holdout_reason").is_some());
        assert!(value["source_reliability"].get("replay_reason").is_some());
        assert!(value["source_reliability"]
            .get("calibration_status")
            .is_some());
        assert!(value["delayed_reward"]["status"].as_str().unwrap().len() > 3);
        assert!(value["delayed_reward"].get("status_reason").is_some());
        assert_eq!(
            value["delayed_reward"]["validation_owner"].as_str(),
            Some("horizon_replay_validation")
        );
        assert!(value["delayed_reward"]
            .get("resolution_brier_score")
            .is_some());
        assert!(value["delayed_reward"]
            .get("latest_training_recommended_at")
            .is_some());
        assert!(value["delayed_reward"]
            .get("resolution_observation_count")
            .is_some());
        assert!(value["delayed_reward"]
            .get("resolution_24h_observation_count")
            .is_some());
        assert_eq!(
            value["live_regime_truth_rule"]["status"].as_str(),
            Some("enforced")
        );
        assert_eq!(
            value["live_regime_truth_rule"]["current_state_branch"].as_str(),
            Some("temporal_hmm_pre_bayes_nowcast")
        );
    }

    #[test]
    fn workflow_status_phase_structural_ranker_runtime_summarizes_runtime_source() {
        let snapshot = sample_human_workflow_snapshot();
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );
        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        let current_rows: Vec<crate::application::orchestration::StructuralPathRankingTargetRow> =
            std::fs::read_to_string(&summary.jsonl_path)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()
                .unwrap();
        let artifact_scored_row = current_rows.first().expect("exported row").clone();
        let artifact_dir = std::path::Path::new(&summary.summary_path)
            .parent()
            .expect("summary parent")
            .to_path_buf();
        std::fs::write(
            artifact_dir.join("artifact_scores.jsonl"),
            format!(
                "{}\n",
                serde_json::json!({
                    "candidate_set_id": summary.candidate_set_id,
                    "path_id": artifact_scored_row.path_id,
                    "raw_path_score": 0.97,
                    "calibrated_path_prob": 0.88,
                    "path_prob_lower_bound": 0.79,
                    "execution_gate_status": "pass"
                })
            ),
        )
        .unwrap();
        let artifact = crate::application::entry_models::training_export::StructuralPathRankingTrainerArtifact {
            protocol_version: "structural-path-ranking-trainer-artifact-v1".to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: "catboost".to_string(),
            artifact_uri: "artifact_scores.jsonl".to_string(),
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "raw_path_score".to_string()],
            validation_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerValidationMetrics::default(),
            calibration_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: None,
            notes: vec![],
        };
        std::fs::write(
            artifact_dir.join("structural_path_ranking_trainer_artifact.json"),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();

        let value = build_workflow_status_phase_value_with_structural_prior_state_and_state_dir(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
            Some(temp.path().to_str().unwrap()),
            "structural-ranker-runtime",
        )
        .unwrap();

        assert_eq!(
            value["runtime_source"].as_str(),
            Some("registered_artifact")
        );
        assert_eq!(
            value["status"].as_str(),
            Some("using_registered_artifact_scores")
        );
        assert_eq!(value["runtime_enabled"].as_bool(), Some(true));
        assert_eq!(value["applied_path_count"].as_u64(), Some(1));
        assert_eq!(value["artifact_match_count"].as_u64(), Some(1));
        assert_eq!(value["candidate_set_match_count"].as_u64(), Some(0));
        assert_eq!(value["history_match_count"].as_u64(), Some(0));
        assert!(value.get("recommended_next_step").is_some());
    }

    #[test]
    fn workflow_status_phase_structural_top_path_candidates_ranks_paths() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-top-path-candidates",
        )
        .unwrap();

        assert_eq!(value["symbol"], "NQ");
        assert_eq!(value["candidate_count"], 3);
        assert_eq!(
            value["candidates"][0]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert!(
            value["candidates"][0]["composite_score"].as_f64().unwrap()
                >= value["candidates"][1]["composite_score"].as_f64().unwrap()
        );
        assert!(value["candidates"][0]["experience_prior"].as_f64().unwrap() > 0.5);
    }

    #[test]
    fn workflow_status_phase_structural_top_path_candidates_exposes_recommended_next_step_contract()
    {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-top-path-candidates",
        )
        .unwrap();

        assert_eq!(
            value["recommended_next_step"]["execution_contract"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );

        let blocked_value = build_workflow_status_phase_value(
            &sample_human_workflow_snapshot(),
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-top-path-candidates",
        )
        .unwrap();
        assert!(blocked_value["recommended_next_step"]["execution_contract"].is_null());
    }

    #[test]
    fn workflow_status_phase_structural_temporal_summary_exposes_discounted_masses() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_update = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "update".to_string(),
            structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-prev".to_string(),
                recommended_at: "2026-04-30T01:00:00Z".to_string(),
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
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "transition".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.18),
                ("range".to_string(), 0.10),
                ("transition".to_string(), 0.72),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.node_duration_priors.insert(
            "NQ:belief_regime_node:transition".to_string(),
            crate::state::StructuralNodeDurationPrior {
                observations: 3,
                streak_count: 2,
                weighted_streak_mass: 1.85,
                weighted_success_mass: 1.85,
                weighted_failure_mass: 0.0,
                total_streak_length: 3,
                avg_streak_length: 1.5,
                max_streak_length: 2,
                last_streak_length: 1,
                persistence_prior: 0.6,
                duration_distribution_entropy: std::f64::consts::LN_2,
                empirical_duration_survival: 1.0,
                empirical_duration_completion_hazard: 0.5405405405,
                bocpd_duration_surprise: 0.0,
                bocpd_evidence_weight: 0.62,
                bocpd_raw_break_probability: 0.44,
                bocpd_break_probability: 0.36,
                bocpd_continue_probability: 0.64,
                bocpd_run_length_mode: 1,
                bocpd_run_length_mode_probability: 0.5405405405,
                bocpd_run_length_tail_probability: 1.0,
                bocpd_run_length_observation_mass: 1.85,
                bocpd_recursive_reset_probability: 0.609,
                bocpd_recursive_run_length_mode: 0,
                bocpd_recursive_run_length_mode_probability: 0.609,
                bocpd_recursive_run_length_expected_value: 0.891,
                bocpd_recursive_run_length_entropy: 0.913,
                duration_outcome_support: 0.7407407407,
                temporal_posterior_support: 0.6422222222,
                last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
                ..crate::state::StructuralNodeDurationPrior::default()
            },
        );
        structural_prior_state.node_temporal_posteriors.insert(
            "NQ:belief_regime_node:transition".to_string(),
            crate::state::StructuralNodeTemporalPosteriorState {
                node_id: "NQ:belief_regime_node:transition".to_string(),
                observations: 4,
                streak_count: 3,
                weighted_streak_mass: 1.9,
                expected_dwell_steps: 1.7,
                remaining_dwell_steps: 0.7,
                break_hazard: 0.37,
                sticky_self_transition_strength: 0.63,
                duration_outcome_support: 0.75,
                temporal_posterior_support: 0.65,
                posterior_blend_weight: 0.42,
                summary_line:
                    "duration_mass=1.900 duration_support=0.750 duration_temporal=0.650 blend=0.420"
                        .to_string(),
                last_recommended_at: Some("2026-04-30T04:00:00Z".to_string()),
                ..crate::state::StructuralNodeTemporalPosteriorState::default()
            },
        );
        structural_prior_state.branch_transition_priors.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation".to_string(),
            crate::state::StructuralBranchTransitionPrior {
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:transition".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:transition:transition_confirmation".to_string(),
                observations: 2,
                weighted_observation_mass: 1.4875,
                wins: 2,
                losses: 0,
                invalidated: 0,
                transition_prior: 0.68,
                transition_outcome_support: 0.71,
                temporal_posterior_support: 0.689,
                weighted_success_mass: 1.4875,
                weighted_failure_mass: 0.0,
                last_recommended_at: Some("2026-04-30T05:00:00Z".to_string()),
            },
        );
        structural_prior_state.branch_temporal_posteriors.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation".to_string(),
            crate::state::StructuralBranchTemporalPosteriorState {
                transition_key: "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:transition:transition_confirmation".to_string(),
                observations: 2,
                weighted_observation_mass: 1.5,
                transition_prior: 0.68,
                transition_outcome_support: 0.72,
                temporal_posterior_support: 0.70,
                posterior_multiplier: 1.28,
                normalized_transition_posterior: 0.68,
                summary_line: "transition_mass=1.500 transition_support=0.720 transition_temporal=0.700 multiplier=1.280".to_string(),
                last_recommended_at: Some("2026-04-30T05:00:00Z".to_string()),
            },
        );
        structural_prior_state.node_transition_posteriors.insert(
            "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition".to_string(),
            crate::state::StructuralNodeTransitionPosteriorState {
                transition_key:
                    "NQ:belief_regime_node:trend=>NQ:belief_regime_node:transition".to_string(),
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:transition".to_string(),
                observations: 2,
                weighted_observation_mass: 1.5,
                transition_prior: 0.71,
                weighted_success_mass: 1.2,
                weighted_failure_mass: 0.3,
                transition_outcome_support: 0.76,
                temporal_posterior_support: 0.725,
                posterior_multiplier: 1.31,
                normalized_transition_posterior: 0.74,
                summary_line: "node_transition_mass=1.500 node_transition_support=0.760 node_transition_temporal=0.725 multiplier=1.310".to_string(),
                last_recommended_at: Some("2026-04-30T05:00:00Z".to_string()),
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &structural_prior_state,
            "structural-temporal-summary",
        )
        .unwrap();

        assert_eq!(value["node_id"], "NQ:belief_regime_node:transition");
        assert_eq!(value["duration_weighted_streak_mass"], 1.9);
        assert_eq!(value["duration_expected_dwell_steps"], 1.7);
        assert_eq!(value["duration_remaining_dwell_steps"], 0.7);
        assert_eq!(value["duration_break_hazard"], 0.37);
        assert_eq!(value["duration_sticky_self_transition_strength"], 0.63);
        assert_eq!(
            value["duration_distribution_entropy"],
            std::f64::consts::LN_2
        );
        assert_eq!(value["empirical_duration_survival"], 1.0);
        assert_eq!(value["empirical_duration_completion_hazard"], 0.5405405405);
        assert_eq!(value["bocpd_evidence_weight"], 0.62);
        assert_eq!(value["bocpd_raw_break_probability"], 0.44);
        assert_eq!(value["bocpd_break_probability"], 0.36);
        assert_eq!(value["bocpd_continue_probability"], 0.64);
        assert_eq!(value["bocpd_run_length_mode"], 1);
        assert_eq!(value["bocpd_run_length_mode_probability"], 0.5405405405);
        assert_eq!(value["bocpd_run_length_tail_probability"], 1.0);
        assert_eq!(value["bocpd_run_length_observation_mass"], 1.85);
        assert_eq!(value["bocpd_recursive_reset_probability"], 0.609);
        assert_eq!(value["bocpd_recursive_run_length_mode"], 0);
        assert_eq!(value["bocpd_recursive_run_length_mode_probability"], 0.609);
        assert_eq!(value["bocpd_recursive_run_length_expected_value"], 0.891);
        assert_eq!(value["bocpd_recursive_run_length_entropy"], 0.913);
        assert_eq!(value["duration_outcome_support"], 0.75);
        assert_eq!(value["duration_temporal_posterior_support"], 0.65);
        assert_eq!(value["duration_posterior_blend_weight"], 0.42);
        assert_eq!(value["transition_weighted_observation_mass"], 1.5);
        assert_eq!(value["transition_prior"], 0.68);
        assert_eq!(value["transition_outcome_support"], 0.72);
        assert_eq!(value["transition_temporal_posterior_support"], 0.70);
        assert_eq!(value["transition_posterior_multiplier"], 1.28);
        assert_eq!(value["transition_normalized_posterior"], 0.68);
        assert_eq!(value["node_transition_prior"], 0.71);
        assert_eq!(value["node_transition_temporal_posterior_support"], 0.725);
        assert_eq!(value["node_transition_posterior_multiplier"], 1.31);
        assert_eq!(value["node_transition_normalized_posterior"], 0.74);
    }

    #[test]
    fn structural_experience_priors_node_prefer_persisted_temporal_state_values_over_raw_duration_prior_fields(
    ) {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.node_duration_priors.insert(
            "NQ:belief_regime_node:trend".to_string(),
            crate::state::StructuralNodeDurationPrior {
                observations: 3,
                streak_count: 2,
                weighted_streak_mass: 1.85,
                weighted_success_mass: 1.85,
                weighted_failure_mass: 0.0,
                total_streak_length: 3,
                avg_streak_length: 1.5,
                max_streak_length: 2,
                last_streak_length: 1,
                persistence_prior: 0.6,
                duration_outcome_support: 0.7407407407,
                temporal_posterior_support: 0.6422222222,
                last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
                ..crate::state::StructuralNodeDurationPrior::default()
            },
        );
        structural_prior_state.node_temporal_posteriors.insert(
            "NQ:belief_regime_node:trend".to_string(),
            crate::state::StructuralNodeTemporalPosteriorState {
                node_id: "NQ:belief_regime_node:trend".to_string(),
                observations: 3,
                streak_count: 2,
                weighted_streak_mass: 1.1,
                bocpd_sequence_change_intensity: 0.25,
                bocpd_sequence_break_probability: 0.33,
                bocpd_sequence_recursive_reset_probability: 0.44,
                bocpd_sequence_recursive_run_length_mode: 2,
                bocpd_sequence_recursive_run_length_mode_probability: 0.55,
                bocpd_sequence_recursive_run_length_expected_value: 1.7,
                bocpd_sequence_recursive_run_length_entropy: 0.66,
                duration_outcome_support: 0.2,
                temporal_posterior_support: 0.3,
                posterior_blend_weight: 0.2,
                summary_line: "duration_mass=1.100 sequence_break=0.330 sequence_reset=0.440 duration_support=0.200 duration_temporal=0.300 blend=0.200".to_string(),
                last_recommended_at: Some("2026-04-30T04:00:00Z".to_string()),
                ..crate::state::StructuralNodeTemporalPosteriorState::default()
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &structural_prior_state,
            "structural-experience-priors",
        )
        .unwrap();

        assert_eq!(value["node"]["duration_weighted_streak_mass"], 1.1);
        assert_eq!(value["node"]["duration_outcome_support"], 0.2);
        assert_eq!(value["node"]["duration_temporal_posterior_support"], 0.3);
    }

    #[test]
    fn structural_temporal_summary_node_prefers_persisted_temporal_state_streak_count() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.node_duration_priors.insert(
            "NQ:belief_regime_node:trend".to_string(),
            crate::state::StructuralNodeDurationPrior {
                observations: 3,
                streak_count: 2,
                weighted_streak_mass: 1.85,
                weighted_success_mass: 1.85,
                weighted_failure_mass: 0.0,
                total_streak_length: 3,
                avg_streak_length: 1.5,
                max_streak_length: 2,
                last_streak_length: 1,
                persistence_prior: 0.6,
                duration_outcome_support: 0.7407407407,
                temporal_posterior_support: 0.6422222222,
                last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
                ..crate::state::StructuralNodeDurationPrior::default()
            },
        );
        structural_prior_state.node_temporal_posteriors.insert(
            "NQ:belief_regime_node:trend".to_string(),
            crate::state::StructuralNodeTemporalPosteriorState {
                node_id: "NQ:belief_regime_node:trend".to_string(),
                observations: 3,
                streak_count: 5,
                weighted_streak_mass: 1.1,
                bocpd_sequence_change_intensity: 0.25,
                bocpd_sequence_break_probability: 0.33,
                bocpd_sequence_recursive_reset_probability: 0.44,
                bocpd_sequence_recursive_run_length_mode: 2,
                bocpd_sequence_recursive_run_length_mode_probability: 0.55,
                bocpd_sequence_recursive_run_length_expected_value: 1.7,
                bocpd_sequence_recursive_run_length_entropy: 0.66,
                duration_outcome_support: 0.2,
                temporal_posterior_support: 0.3,
                posterior_blend_weight: 0.2,
                summary_line: "duration_mass=1.100 sequence_break=0.330 sequence_reset=0.440 duration_support=0.200 duration_temporal=0.300 blend=0.200".to_string(),
                last_recommended_at: Some("2026-04-30T04:00:00Z".to_string()),
                ..crate::state::StructuralNodeTemporalPosteriorState::default()
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
            &structural_prior_state,
            "structural-temporal-summary",
        )
        .unwrap();

        assert_eq!(value["duration_streak_count"], 5);
        assert_eq!(value["bocpd_sequence_change_intensity"], 0.25);
        assert_eq!(value["bocpd_sequence_break_probability"], 0.33);
        assert_eq!(value["bocpd_sequence_recursive_reset_probability"], 0.44);
        assert_eq!(value["bocpd_sequence_recursive_run_length_mode"], 2);
        assert_eq!(
            value["bocpd_sequence_recursive_run_length_mode_probability"],
            0.55
        );
        assert_eq!(
            value["bocpd_sequence_recursive_run_length_expected_value"],
            1.7
        );
        assert_eq!(value["bocpd_sequence_recursive_run_length_entropy"], 0.66);
    }

    #[test]
    fn agent_and_human_workflow_status_views_expose_structural_temporal_summary() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_update = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "update".to_string(),
            structural_feedback: Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "rec-prev".to_string(),
                recommended_at: "2026-04-30T01:00:00Z".to_string(),
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
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "transition".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.18),
                ("range".to_string(), 0.10),
                ("transition".to_string(), 0.72),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let mut structural_prior_state = crate::state::StructuralPriorLearningState::default();
        structural_prior_state.node_duration_priors.insert(
            "NQ:belief_regime_node:transition".to_string(),
            crate::state::StructuralNodeDurationPrior {
                observations: 3,
                streak_count: 2,
                weighted_streak_mass: 1.85,
                weighted_success_mass: 1.85,
                weighted_failure_mass: 0.0,
                total_streak_length: 3,
                avg_streak_length: 1.5,
                max_streak_length: 2,
                last_streak_length: 1,
                persistence_prior: 0.6,
                bocpd_sequence_change_intensity: 0.25,
                bocpd_sequence_break_probability: 0.33,
                bocpd_sequence_recursive_reset_probability: 0.44,
                bocpd_sequence_recursive_run_length_mode: 2,
                bocpd_sequence_recursive_run_length_mode_probability: 0.55,
                bocpd_sequence_recursive_run_length_expected_value: 1.7,
                bocpd_sequence_recursive_run_length_entropy: 0.66,
                duration_outcome_support: 0.7407407407,
                temporal_posterior_support: 0.6422222222,
                last_recommended_at: Some("2026-04-30T03:00:00Z".to_string()),
                ..crate::state::StructuralNodeDurationPrior::default()
            },
        );
        structural_prior_state.branch_transition_priors.insert(
            "NQ:belief_regime_node:trend:trend_follow_through=>NQ:belief_regime_node:transition:transition_confirmation".to_string(),
            crate::state::StructuralBranchTransitionPrior {
                from_node_id: "NQ:belief_regime_node:trend".to_string(),
                to_node_id: "NQ:belief_regime_node:transition".to_string(),
                from_branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                to_branch_id: "NQ:belief_regime_node:transition:transition_confirmation".to_string(),
                observations: 2,
                weighted_observation_mass: 1.4875,
                wins: 2,
                losses: 0,
                invalidated: 0,
                transition_prior: 0.68,
                transition_outcome_support: 0.71,
                temporal_posterior_support: 0.689,
                weighted_success_mass: 1.4875,
                weighted_failure_mass: 0.0,
                last_recommended_at: Some("2026-04-30T05:00:00Z".to_string()),
            },
        );

        let agent_value =
            build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &[],
                &structural_prior_state,
            );
        let human_value =
            build_human_workflow_status_view_with_provider_agent_and_structural_prior_state(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &[],
                &structural_prior_state,
            );

        assert_eq!(
            agent_value["structural_temporal_summary"]["duration_weighted_streak_mass"],
            1.85
        );
        assert_eq!(
            agent_value["structural_temporal_summary"]["transition_weighted_observation_mass"],
            1.4875
        );
        assert!(human_value["structural_temporal_line"]
            .as_str()
            .unwrap()
            .contains("duration_mass=1.850"));
        assert!(human_value["structural_temporal_line"]
            .as_str()
            .unwrap()
            .contains("duration_temporal=0.642"));
        assert!(human_value["structural_temporal_line"]
            .as_str()
            .unwrap()
            .contains("sequence_break=0.330"));
        assert!(human_value["structural_temporal_line"]
            .as_str()
            .unwrap()
            .contains("sequence_reset=0.440"));
        assert!(human_value["structural_temporal_line"]
            .as_str()
            .unwrap()
            .contains("transition_mass=1.488"));
        assert!(human_value["structural_temporal_line"]
            .as_str()
            .unwrap()
            .contains("transition_temporal=0.689"));
    }

    #[test]
    fn workflow_status_structural_detail_phases_share_recommended_next_step_contract() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];
        let phases = [
            "structural-node",
            "structural-branches",
            "structural-scenarios",
            "structural-paths",
            "structural-history-summary",
            "structural-feedback-template",
        ];

        for phase in phases {
            let value = build_workflow_status_phase_value(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                phase,
            )
            .unwrap();
            assert_eq!(
                value["recommended_next_step"]["execution_contract"]["path_id"],
                "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary",
                "phase={phase}"
            );
        }
    }

    #[test]
    fn workflow_status_phase_structural_recommended_path_bundle_is_token_friendly() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let value = build_workflow_status_phase_value(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            "structural-recommended-path-bundle",
        )
        .unwrap();

        assert_eq!(value["symbol"], "NQ");
        assert_eq!(value["rank"], 1);
        assert_eq!(
            value["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert!(value["candidate_set_id"]
            .as_str()
            .unwrap()
            .starts_with("structural-candidates:NQ:"));
        assert_eq!(value["candidate_set_size"], 3);
        assert!(value["selected_path_probability"].as_f64().unwrap() > 0.0);
        assert!(value["trigger_summary"]
            .as_str()
            .unwrap()
            .contains("regime"));
        assert!(value["stop_summary"].as_str().unwrap().len() > 4);
        assert!(value["confirmation_summary"].as_str().unwrap().len() > 4);
        assert!(value["invalidation_summary"].as_str().unwrap().len() > 4);
        assert_eq!(
            value["recommended_next_step"]["execution_contract"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
    }

    #[test]
    fn workflow_status_phase_structural_path_ranking_target_is_candidate_scoped() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );

        let value = build_workflow_status_phase_value_with_structural_prior_state(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
            "structural-path-ranking-target",
        )
        .unwrap();
        let rows = value["rows"].as_array().unwrap();
        let first = &rows[0];
        let row_object = first.as_object().unwrap();

        assert_eq!(value["candidate_set_size"], 3);
        assert_eq!(first["candidate_set_id"], value["candidate_set_id"]);
        assert_eq!(first["candidate_set_size"], value["candidate_set_size"]);
        assert_eq!(first["path_id"], path_id);
        assert_eq!(first["pending_reward_state"], "matured_invalidated");
        assert_eq!(first["maturity_mask"], true);
        assert_eq!(first["maturity_weight"].as_f64().unwrap(), 1.0);
        assert_eq!(first["calibrated_label"].as_f64().unwrap(), 0.0);
        assert_eq!(first["regime_calibration_bucket"], "NQ:trend");
        assert!(!row_object.contains_key("raw_path_score"));
        assert!(!row_object.contains_key("calibrated_path_prob"));
        assert!(!row_object.contains_key("path_prob_lower_bound"));
        let behavior_policy_probability = first["behavior_policy_probability"].as_f64().unwrap();
        assert!(behavior_policy_probability > 0.0);
        assert!(
            (first["propensity_estimate"].as_f64().unwrap() - behavior_policy_probability * 0.6)
                .abs()
                < 1e-9
        );
        assert_eq!(first["target_policy_probability_confidence"], 0.57);
        assert_eq!(first["target_policy_probability_lower_bound"], 0.31);
        assert_eq!(first["target_policy_reward_prior"], 0.58);
        assert_eq!(first["target_policy_reward_lower_bound"], 0.29);
        assert!(first["ips_weight"].as_f64().unwrap() > 0.0);
        assert!(first["training_weight"].as_f64().unwrap() > 0.0);

        let agent_value =
            build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                &structural_prior_state,
            );
        assert_eq!(
            agent_value["path_ranking_target"]["candidate_set_id"],
            agent_value["recommended_path_bundle"]["candidate_set_id"]
        );

        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        assert_eq!(summary.rows, 3);
        assert_eq!(summary.mature_rows, 1);
        assert_eq!(summary.rows_with_execution_gate_status, 0);
        assert_eq!(summary.rows_with_training_weight, 1);
        assert_eq!(
            summary.trainer_manifest.dataset_role,
            "external_path_ranker_training_dataset"
        );
        assert_eq!(summary.trainer_manifest.group_id_column, "candidate_set_id");
        assert_eq!(summary.trainer_manifest.label_column, "calibrated_label");
        assert_eq!(summary.trainer_manifest.weight_column, "training_weight");
        assert!(summary
            .trainer_manifest
            .feature_columns
            .contains(&"target_policy_reward_prior".to_string()));
        assert_eq!(summary.candidate_set_id, value["candidate_set_id"]);
        assert_eq!(summary.pending_reward_states["matured_invalidated"], 1);
        assert!(summary.history_rows >= summary.rows);
        assert!(std::path::Path::new(&summary.csv_path).exists());
        assert!(std::path::Path::new(&summary.jsonl_path).exists());
        assert!(std::path::Path::new(&summary.history_jsonl_path).exists());
        assert!(std::path::Path::new(&summary.summary_path).exists());
        let csv = std::fs::read_to_string(&summary.csv_path).unwrap();
        assert!(csv.contains("pending_reward_state"));
        assert!(csv.contains("maturity_mask"));
        assert!(csv.contains("maturity_weight"));
        assert!(csv.contains("execution_gate_status"));
        assert!(csv.contains("execution_gate_min_path_prob"));
        assert!(csv.contains("calibrated_label"));
        assert!(csv.contains("ips_weight"));
        assert!(csv.contains("training_weight"));
        assert!(csv.contains("propensity_estimate"));
        assert!(csv.contains("target_policy_probability_confidence"));
        assert!(csv.contains("target_policy_reward_lower_bound"));
        let jsonl = std::fs::read_to_string(&summary.jsonl_path).unwrap();
        assert!(jsonl.contains("\"pending_reward_state\":\"matured_invalidated\""));
        assert!(jsonl.contains("\"maturity_mask\":true"));
        assert!(jsonl.contains("\"maturity_weight\":1.0"));
        assert!(jsonl.contains("\"calibrated_label\":0.0"));
        assert!(jsonl.contains("\"target_policy_reward_prior\":0.58"));
    }

    #[test]
    fn applying_structural_path_ranking_external_scores_updates_current_and_history_exports() {
        let snapshot = sample_human_workflow_snapshot();
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );
        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        let current_rows: Vec<crate::application::orchestration::StructuralPathRankingTargetRow> =
            std::fs::read_to_string(&summary.jsonl_path)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()
                .unwrap();
        let scored_row = current_rows.first().expect("exported row").clone();
        let updated_summary =
            crate::application::orchestration::apply_structural_path_ranking_external_scores(
                temp.path().to_str().unwrap(),
                "NQ",
                &[
                    crate::application::orchestration::StructuralPathRankingExternalScoreInput {
                        candidate_set_id: scored_row.candidate_set_id.clone(),
                        path_id: scored_row.path_id.clone(),
                        raw_path_score: 0.91,
                        score_model_family: None,
                        score_source_kind: None,
                        score_model_artifact_uri: None,
                        score_generator: None,
                    },
                ],
            )
            .unwrap();
        let updated_current: Vec<
            crate::application::orchestration::StructuralPathRankingTargetRow,
        > = std::fs::read_to_string(&updated_summary.jsonl_path)
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        let updated_history: Vec<
            crate::application::orchestration::StructuralPathRankingTargetRow,
        > = std::fs::read_to_string(&updated_summary.history_jsonl_path)
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert!(std::path::Path::new(&updated_summary.history_csv_path).exists());
        assert!(updated_summary.rows_with_raw_path_score >= 1);
        assert_eq!(
            updated_current
                .iter()
                .find(|row| row.path_id == scored_row.path_id)
                .and_then(|row| row.raw_path_score),
            Some(0.91)
        );
        assert_eq!(
            updated_history
                .iter()
                .find(|row| {
                    row.path_id == scored_row.path_id
                        && row.candidate_set_id == scored_row.candidate_set_id
                })
                .and_then(|row| row.raw_path_score),
            Some(0.91)
        );
    }

    #[test]
    fn agent_workflow_status_view_reuses_opted_in_structural_path_ranking_scores() {
        let snapshot = sample_human_workflow_snapshot();
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );
        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        let current_rows: Vec<crate::application::orchestration::StructuralPathRankingTargetRow> =
            std::fs::read_to_string(&summary.jsonl_path)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()
                .unwrap();
        let scored_row = current_rows.first().expect("exported row").clone();
        crate::application::orchestration::apply_structural_path_ranking_external_scores(
            temp.path().to_str().unwrap(),
            "NQ",
            &[
                crate::application::orchestration::StructuralPathRankingExternalScoreInput {
                    candidate_set_id: scored_row.candidate_set_id.clone(),
                    path_id: scored_row.path_id.clone(),
                    raw_path_score: 0.91,
                    score_model_family: None,
                    score_source_kind: None,
                    score_model_artifact_uri: None,
                    score_generator: None,
                },
            ],
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();

        let agent_value =
            build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                &structural_prior_state,
                Some(temp.path().to_str().unwrap()),
            );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_raw_score"].as_f64(),
            Some(0.91)
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime"]["status"].as_str(),
            Some("using_candidate_set_scores")
        );

        let human_value =
            build_human_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                &structural_prior_state,
                Some(temp.path().to_str().unwrap()),
            );
        assert!(
            human_value["recommended_path_line"]
                .as_str()
                .unwrap_or_default()
                .contains("lb=")
                || human_value["recommended_path_line"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("raw=")
        );
    }

    #[test]
    fn agent_workflow_status_prefers_registered_artifact_scores_when_present() {
        let snapshot = sample_human_workflow_snapshot();
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );
        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        let current_rows: Vec<crate::application::orchestration::StructuralPathRankingTargetRow> =
            std::fs::read_to_string(&summary.jsonl_path)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()
                .unwrap();
        let artifact_scored_row = current_rows.first().expect("exported row").clone();
        let artifact_dir = std::path::Path::new(&summary.summary_path)
            .parent()
            .expect("summary parent")
            .to_path_buf();
        std::fs::write(
            artifact_dir.join("artifact_scores.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::json!({
                    "candidate_set_id": summary.candidate_set_id,
                    "path_id": artifact_scored_row.path_id,
                    "raw_path_score": 0.97,
                    "calibrated_path_prob": 0.88,
                    "path_prob_lower_bound": 0.79,
                    "execution_gate_status": "pass"
                }),
                serde_json::json!({
                    "candidate_set_id": "structural-candidates:NQ:other",
                    "path_id": "path:scenario:NQ:belief_regime_node:range:observe_only:primary",
                    "raw_path_score": 0.12,
                    "calibrated_path_prob": 0.18,
                    "path_prob_lower_bound": 0.08,
                    "execution_gate_status": "observe"
                })
            ),
        )
        .unwrap();
        let artifact = crate::application::entry_models::training_export::StructuralPathRankingTrainerArtifact {
            protocol_version: "structural-path-ranking-trainer-artifact-v1".to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: "catboost".to_string(),
            artifact_uri: "artifact_scores.jsonl".to_string(),
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "raw_path_score".to_string()],
            validation_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerValidationMetrics::default(),
            calibration_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: None,
            notes: vec![],
        };
        std::fs::write(
            artifact_dir.join("structural_path_ranking_trainer_artifact.json"),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();

        let agent_value =
            build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                &structural_prior_state,
                Some(temp.path().to_str().unwrap()),
            );
        assert_eq!(
            agent_value["path_ranking_target"]["rows"][0]["raw_path_score"].as_f64(),
            Some(0.97)
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_raw_score"].as_f64(),
            Some(0.97)
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime_source"].as_str(),
            Some("registered_artifact")
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime"]["status"].as_str(),
            Some("using_registered_artifact_scores")
        );
        assert_eq!(
            agent_value["path_ranker_summary"]["runtime_source"].as_str(),
            Some("registered_artifact")
        );
        assert_eq!(
            agent_value["path_ranker_summary"]["status"].as_str(),
            Some("using_registered_artifact_scores")
        );
        assert_eq!(
            agent_value["path_ranker_summary"]["runtime_enabled"].as_bool(),
            Some(true)
        );
        assert_eq!(
            agent_value["path_ranker_summary"]["applied_path_count"].as_u64(),
            Some(1)
        );
        assert_eq!(
            agent_value["path_ranker_summary"]["artifact_match_count"].as_u64(),
            Some(1)
        );
        assert_eq!(
            agent_value["path_ranker_summary"]["candidate_set_match_count"].as_u64(),
            Some(0)
        );
        assert_eq!(
            agent_value["path_ranker_summary"]["history_match_count"].as_u64(),
            Some(0)
        );
    }

    #[test]
    fn agent_workflow_status_can_consume_registered_direct_model_scores() {
        let snapshot = sample_human_workflow_snapshot();
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );
        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        let artifact_dir = std::path::Path::new(&summary.summary_path)
            .parent()
            .expect("summary parent")
            .to_path_buf();
        std::fs::write(
            artifact_dir.join("path_ranker_direct_model.json"),
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
        let artifact = crate::application::entry_models::training_export::StructuralPathRankingTrainerArtifact {
            protocol_version: "structural-path-ranking-trainer-artifact-v1".to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: crate::belief_core::ranking_label::STRUCTURAL_PATH_RANKER_DIRECT_MODEL_FAMILY_WEIGHTED_SUM_V1.to_string(),
            artifact_uri: "path_ranker_direct_model.json".to_string(),
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "experience_prior".to_string()],
            validation_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerValidationMetrics::default(),
            calibration_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: None,
            notes: vec![],
        };
        std::fs::write(
            artifact_dir.join("structural_path_ranking_trainer_artifact.json"),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();

        let agent_value =
            build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                &structural_prior_state,
                Some(temp.path().to_str().unwrap()),
            );
        let score = agent_value["recommended_path_bundle"]["path_ranker_raw_score"]
            .as_f64()
            .unwrap_or_default();
        assert!(score > 0.7);
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime_source"].as_str(),
            Some("registered_model_artifact")
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime"]["status"].as_str(),
            Some("using_registered_model_artifact")
        );
    }

    #[test]
    fn agent_workflow_status_can_consume_registered_explicit_rule_artifact() {
        let snapshot = sample_human_workflow_snapshot();
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );
        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        let artifact_dir = std::path::Path::new(&summary.summary_path)
            .parent()
            .expect("summary parent")
            .to_path_buf();
        let artifact =
            crate::application::entry_models::training_export::StructuralPathRankingTrainerArtifact {
                protocol_version: "structural-path-ranking-trainer-artifact-v1".to_string(),
                dataset_role: "external_path_ranker_training_dataset".to_string(),
                model_family:
                    crate::belief_core::ranking_label::STRUCTURAL_PATH_RANKER_EXPLICIT_FAMILY_CORELS
                        .to_string(),
                artifact_uri: artifact_dir
                    .join("corels_artifact.json")
                    .to_string_lossy()
                    .to_string(),
                model_artifact_uri: None,
                score_column: "raw_path_score".to_string(),
                trained_rows: 42,
                history_rows: 42,
                calibration_rows: 12,
                selected_features: vec!["rank".to_string(), "experience_prior".to_string()],
                validation_metrics:
                    crate::belief_core::ranking_label::StructuralPathRankerValidationMetrics::default(),
                calibration_metrics:
                    crate::belief_core::ranking_label::StructuralPathRankerCalibrationMetrics::default(),
                rule_list: vec![
                    crate::belief_core::ranking_label::StructuralPathRankerRule {
                        conditions: vec![
                            crate::belief_core::ranking_label::StructuralPathRankerRuleCondition {
                                feature: "rank".to_string(),
                                operator: "eq".to_string(),
                                numeric_value: Some(1.0),
                                string_value: None,
                            },
                            crate::belief_core::ranking_label::StructuralPathRankerRuleCondition {
                                feature: "experience_prior".to_string(),
                                operator: "ge".to_string(),
                                numeric_value: Some(0.5),
                                string_value: None,
                            },
                        ],
                        score: 0.91,
                        path_prob_lower_bound: Some(0.81),
                        execution_gate_status: Some("pass".to_string()),
                    },
                    crate::belief_core::ranking_label::StructuralPathRankerRule {
                        conditions: Vec::new(),
                        score: 0.21,
                        path_prob_lower_bound: Some(0.11),
                        execution_gate_status: Some("observe".to_string()),
                    },
                ],
                tree_json: None,
                created_at: None,
                notes: vec![],
            };
        std::fs::write(
            artifact_dir.join("structural_path_ranking_trainer_artifact.json"),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();

        let agent_value =
            build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                &structural_prior_state,
                Some(temp.path().to_str().unwrap()),
            );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_raw_score"].as_f64(),
            Some(0.91)
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime_source"].as_str(),
            Some("registered_explicit_artifact")
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime"]["status"].as_str(),
            Some("using_registered_explicit_artifact")
        );
    }

    #[test]
    fn agent_workflow_status_can_consume_remote_registered_artifact_scores() {
        let snapshot = sample_human_workflow_snapshot();
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );
        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        let current_rows: Vec<crate::application::orchestration::StructuralPathRankingTargetRow> =
            std::fs::read_to_string(&summary.jsonl_path)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()
                .unwrap();
        let artifact_scored_row = current_rows.first().expect("exported row").clone();
        let body = format!(
            "{}\n",
            serde_json::json!({
                "candidate_set_id": summary.candidate_set_id,
                "path_id": artifact_scored_row.path_id,
                "raw_path_score": 0.93,
                "calibrated_path_prob": 0.84,
                "path_prob_lower_bound": 0.74,
                "execution_gate_status": "pass"
            })
        );
        let remote_uri = serve_http_response("artifact_scores.jsonl", body, 8);
        let artifact_dir = std::path::Path::new(&summary.summary_path)
            .parent()
            .expect("summary parent")
            .to_path_buf();
        let artifact = crate::application::entry_models::training_export::StructuralPathRankingTrainerArtifact {
            protocol_version: "structural-path-ranking-trainer-artifact-v1".to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: "catboost".to_string(),
            artifact_uri: remote_uri,
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "raw_path_score".to_string()],
            validation_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerValidationMetrics::default(),
            calibration_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: None,
            notes: vec![],
        };
        std::fs::write(
            artifact_dir.join("structural_path_ranking_trainer_artifact.json"),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();

        let agent_value =
            build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                &structural_prior_state,
                Some(temp.path().to_str().unwrap()),
            );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_raw_score"].as_f64(),
            Some(0.93)
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime_source"].as_str(),
            Some("registered_artifact")
        );
    }

    #[test]
    fn agent_workflow_status_can_consume_registered_service_scores() {
        let snapshot = sample_human_workflow_snapshot();
        let history = sample_structural_feedback_history();
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let mut structural_prior_state = StructuralPriorLearningState::default();
        structural_prior_state.paths.insert(
            path_id.to_string(),
            crate::state::StructuralPriorStats {
                smoothed_prior: 0.62,
                execution_propensity: 0.6,
                target_policy_probability_confidence: 0.57,
                target_policy_probability_lower_bound: 0.31,
                target_policy_reward_prior: 0.58,
                target_policy_reward_lower_bound: 0.29,
                ..crate::state::StructuralPriorStats::default()
            },
        );
        let temp = tempfile::tempdir().unwrap();
        let summary = crate::application::orchestration::export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &sample_provider_agent_surface(),
            &history,
            &structural_prior_state,
        )
        .unwrap();
        let current_rows: Vec<crate::application::orchestration::StructuralPathRankingTargetRow> =
            std::fs::read_to_string(&summary.jsonl_path)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()
                .unwrap();
        let service_scored_row = current_rows.first().expect("exported row").clone();
        let body = serde_json::json!({
            "rows": [
                {
                    "candidate_set_id": summary.candidate_set_id,
                    "path_id": service_scored_row.path_id,
                    "raw_path_score": 0.92,
                    "calibrated_path_prob": 0.83,
                    "path_prob_lower_bound": 0.73,
                    "execution_gate_status": "pass"
                }
            ]
        })
        .to_string();
        let service_uri = serve_http_response_with_method("rank-paths", body, 8, "POST");
        let artifact_dir = std::path::Path::new(&summary.summary_path)
            .parent()
            .expect("summary parent")
            .to_path_buf();
        let artifact = crate::application::entry_models::training_export::StructuralPathRankingTrainerArtifact {
            protocol_version: "structural-path-ranking-trainer-artifact-v1".to_string(),
            dataset_role: "external_path_ranker_training_dataset".to_string(),
            model_family: crate::belief_core::ranking_label::STRUCTURAL_PATH_RANKER_SERVICE_FAMILY_ROW_SCORING_V1.to_string(),
            artifact_uri: service_uri,
            model_artifact_uri: None,
            score_column: "raw_path_score".to_string(),
            trained_rows: 42,
            history_rows: 42,
            calibration_rows: 12,
            selected_features: vec!["rank".to_string(), "experience_prior".to_string()],
            validation_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerValidationMetrics::default(),
            calibration_metrics:
                crate::belief_core::ranking_label::StructuralPathRankerCalibrationMetrics::default(),
            rule_list: Vec::new(),
            tree_json: None,
            created_at: None,
            notes: vec![],
        };
        std::fs::write(
            artifact_dir.join("structural_path_ranking_trainer_artifact.json"),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            crate::application::orchestration::STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();

        let agent_value =
            build_agent_workflow_status_view_with_provider_agent_and_structural_prior_state_and_state_dir(
                &snapshot,
                &[],
                &sample_provider_agent_surface(),
                &history,
                &structural_prior_state,
                Some(temp.path().to_str().unwrap()),
            );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_raw_score"].as_f64(),
            Some(0.92)
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime_source"].as_str(),
            Some("registered_service")
        );
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_ranker_runtime"]["status"].as_str(),
            Some("using_registered_service_scores")
        );
    }

    #[test]
    fn agent_workflow_status_view_exposes_latest_structural_feedback() {
        let mut snapshot = sample_human_workflow_snapshot();
        if let Some(update) = snapshot.latest_update.as_mut() {
            update.structural_feedback = Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "structural-feedback:NQ:node:path".to_string(),
                recommended_at: "2026-04-29T00:00:00Z".to_string(),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                    .to_string(),
                path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                    .to_string(),
                followed_path: true,
                exit_reason: Some("target_hit".to_string()),
                notes: Some("user followed structural path".to_string()),
            });
        }

        let value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
        );

        assert_eq!(
            value["latest_structural_feedback"]["recommendation_id"],
            "structural-feedback:NQ:node:path"
        );
        assert_eq!(
            value["latest_structural_feedback"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
    }

    #[test]
    fn agent_workflow_status_view_exposes_structural_path_summary() {
        let mut snapshot = sample_human_workflow_snapshot();
        if let Some(update) = snapshot.latest_update.as_mut() {
            update.structural_feedback = sample_structural_feedback_history()[1]
                .structural_feedback
                .clone();
        }
        let history = sample_structural_feedback_history();

        let value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert_eq!(
            value["structural_path_summary"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert_eq!(value["structural_path_summary"]["total_records"], 2);
        assert_eq!(value["structural_path_summary"]["invalidated"], 1);
    }

    #[test]
    fn agent_workflow_status_view_exposes_structural_history_summary() {
        let mut snapshot = sample_human_workflow_snapshot();
        if let Some(update) = snapshot.latest_update.as_mut() {
            update.structural_feedback = sample_structural_feedback_history()[1]
                .structural_feedback
                .clone();
        }
        let history = sample_structural_feedback_history();

        let value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert_eq!(value["structural_history_summary"]["total_records"], 2);
        assert_eq!(value["structural_history_summary"]["distinct_paths"], 1);
        assert_eq!(
            value["structural_history_summary"]["latest_path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
    }

    #[test]
    fn agent_and_human_workflow_status_views_expose_experience_prior_surface() {
        let mut snapshot = sample_human_workflow_snapshot();
        if let Some(update) = snapshot.latest_update.as_mut() {
            update.structural_feedback = sample_structural_feedback_history()[1]
                .structural_feedback
                .clone();
        }
        let history = sample_structural_feedback_history();

        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );
        let human_value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert_eq!(
            agent_value["experience_prior_surface"]["path"]["entity_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert_eq!(
            agent_value["experience_prior_surface"]["path"]["historical_total_records"],
            2
        );
        assert!(human_value["experience_prior_line"]
            .as_str()
            .unwrap()
            .contains("path_prior="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("Validation: em="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("target_policy=bucket_posterior"));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("split="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("holdout_cov="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("holdout_reason="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("replay="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("replay_cov="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("replay_reason="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("calib="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("current_state_branch=temporal_hmm_pre_bayes_nowcast"));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("train_until="));
        assert!(human_value["structural_validation_line"]
            .as_str()
            .unwrap()
            .contains("obs_1h="));
        assert!(
            agent_value["structural_validation_summary"]["source_reliability"]["status"]
                .as_str()
                .unwrap()
                .len()
                > 3
        );
        assert!(
            agent_value["structural_validation_summary"]["source_reliability"]
                .get("holdout_split_strategy")
                .is_some()
        );
        assert!(
            agent_value["structural_validation_summary"]["source_reliability"]
                .get("replay_split_strategy")
                .is_some()
        );
        assert!(
            agent_value["structural_validation_summary"]["source_reliability"]
                .get("replay_observation_coverage")
                .is_some()
        );
        assert!(
            agent_value["structural_validation_summary"]["source_reliability"]
                .get("holdout_reason")
                .is_some()
        );
        assert!(
            agent_value["structural_validation_summary"]["source_reliability"]
                .get("calibration_status")
                .is_some()
        );
        assert_eq!(
            agent_value["structural_validation_summary"]["target_policy"]["current_model"].as_str(),
            Some("symbol:regime:direction_bucket_posterior")
        );
        assert_eq!(
            agent_value["structural_validation_summary"]["live_regime_truth_rule"]["status"]
                .as_str(),
            Some("enforced")
        );
        assert_eq!(
            agent_value["structural_validation_summary"]["live_regime_truth_rule"]
                ["current_state_branch"]
                .as_str(),
            Some("temporal_hmm_pre_bayes_nowcast")
        );
        assert!(
            agent_value["structural_validation_summary"]["delayed_reward"]
                ["resolution_brier_score"]
                .is_number()
                || agent_value["structural_validation_summary"]["delayed_reward"]
                    ["resolution_brier_score"]
                    .is_null()
        );
        assert!(
            agent_value["structural_validation_summary"]["delayed_reward"]
                .get("resolution_observation_count")
                .is_some()
        );
        assert!(
            agent_value["structural_validation_summary"]["delayed_reward"]
                .get("status_reason")
                .is_some()
        );
        assert_eq!(
            agent_value["structural_validation_summary"]["delayed_reward"]["validation_owner"]
                .as_str(),
            Some("horizon_replay_validation")
        );
    }

    #[test]
    fn agent_and_human_workflow_status_views_expose_top_path_candidates() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );
        let human_value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert_eq!(agent_value["top_path_candidates"][0]["rank"], 1);
        assert_eq!(
            agent_value["top_path_candidates"][0]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert!(human_value["top_path_candidates_line"]
            .as_str()
            .unwrap()
            .contains("trend_follow_through"));
    }

    #[test]
    fn agent_and_human_workflow_status_views_expose_recommended_path_bundle() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );
        let human_value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert_eq!(agent_value["recommended_path_bundle"]["rank"], 1);
        assert_eq!(
            agent_value["recommended_path_bundle"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert!(agent_value["recommended_path_bundle"]["why_this_path"]
            .as_str()
            .unwrap()
            .contains("posterior"));
        assert!(human_value["path_ranker_line"].is_null());
        assert!(human_value["recommended_path_line"]
            .as_str()
            .unwrap()
            .contains("trend_follow_through"));
    }

    #[test]
    fn agent_and_human_workflow_status_views_expose_recommended_path_contract() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );
        let human_value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert_eq!(
            agent_value["recommended_path_contract"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert!(agent_value["recommended_path_contract"]["why"]
            .as_str()
            .unwrap()
            .contains("posterior"));
        assert!(human_value["recommended_path_contract_line"]
            .as_str()
            .unwrap()
            .contains("trigger="));
        assert!(human_value["recommended_path_contract_line"]
            .as_str()
            .unwrap()
            .contains("stop="));
    }

    #[test]
    fn agent_and_human_workflow_status_views_prefer_canonical_analyze_ensemble_surface() {
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            latest_analyze: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "analyze".to_string(),
                run_id: "analyze:1".to_string(),
                pre_bayes_filtered_assignments: std::collections::BTreeMap::from([(
                    "market_regime".to_string(),
                    "trend".to_string(),
                )]),
                pre_bayes_soft_evidence: std::collections::BTreeMap::from([(
                    "market_regime".to_string(),
                    std::collections::BTreeMap::from([
                        ("trend".to_string(), 0.78),
                        ("range".to_string(), 0.14),
                        ("transition".to_string(), 0.08),
                    ]),
                )]),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_ensemble_vote: Some(EnsembleVoteRecord {
                artifact_id: "ensemble-vote:analyze:test".to_string(),
                generated_at: Utc::now(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:1".to_string()),
                provenance: RunProvenance::default(),
                dataset_comparability: DatasetComparability::default(),
                ensemble_version: "ensemble-audit-v2".to_string(),
                final_action: "execute_follow_through".to_string(),
                recommended_command: "ict-engine workflow-status --symbol NQ --phase human-next"
                    .to_string(),
                human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                    .to_string(),
                hard_block: EnsembleHardBlockArtifact::default(),
                confidence: 0.55,
                consensus_strength: 0.55,
                disagreement_flags: Vec::new(),
                executor_summaries: Vec::new(),
                split_explanations: Vec::new(),
                executor_scorecards: Vec::new(),
                executor_scorecards_source: None,
                posterior_fingerprint: "fp-raw".to_string(),
                posterior_normalization_status: "normalized".to_string(),
                posterior_active_regime: "bull".to_string(),
                posterior_confidence: Some(0.55),
                posterior_probabilities: std::collections::BTreeMap::from([
                    ("bull".to_string(), 0.55),
                    ("range".to_string(), 0.30),
                    ("transition".to_string(), 0.15),
                ]),
                posterior_evidence: vec!["raw".to_string()],
            }),
            ..WorkflowSnapshot::default()
        };

        let human_value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
        );
        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
        );

        assert_eq!(
            human_value["ensemble_consensus"]["posterior_active_regime"],
            "trend"
        );
        assert_eq!(
            human_value["ensemble_consensus"]["posterior_confidence"],
            0.78
        );
        assert_eq!(agent_value["ensemble"]["confidence"], 0.78);
    }

    #[test]
    fn human_and_agent_workflow_status_inline_execution_contract_only_when_not_blocked() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );
        let human_value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert_eq!(
            agent_value["next_step"]["execution_contract"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert!(human_value["what_you_should_do_now"]
            .as_str()
            .unwrap()
            .contains("Execution contract:"));

        let blocked_human = build_human_workflow_status_view_with_provider_agent(
            &sample_human_workflow_snapshot(),
            &[],
            &sample_provider_agent_surface(),
            &history,
        );
        let blocked_agent = build_agent_workflow_status_view_with_provider_agent(
            &sample_human_workflow_snapshot(),
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert!(!blocked_human["what_you_should_do_now"]
            .as_str()
            .unwrap()
            .contains("Execution contract:"));
        assert!(blocked_agent["next_step"]["execution_contract"].is_null());
    }

    #[test]
    fn human_and_agent_workflow_status_expose_recommended_next_step_contract() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.symbol = "NQ".to_string();
        snapshot.current_focus_phase = "analyze".to_string();
        snapshot.recommended_next_command =
            "ict-engine workflow-status --symbol NQ --phase human-next".to_string();
        snapshot.latest_analyze = Some(crate::state::WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            phase_summary: "belief regime available".to_string(),
            ..crate::state::WorkflowPhaseSnapshot::default()
        });
        snapshot.latest_ensemble_vote = Some(EnsembleVoteRecord {
            artifact_id: "ensemble-vote:structural".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-structural".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2".to_string(),
            final_action: "execute_follow_through".to_string(),
            recommended_command: snapshot.recommended_next_command.clone(),
            human_next_triage: "hard_blocked=false ensemble_action=execute_follow_through"
                .to_string(),
            hard_block: EnsembleHardBlockArtifact::default(),
            confidence: 0.72,
            consensus_strength: 0.64,
            disagreement_flags: Vec::new(),
            executor_summaries: Vec::new(),
            split_explanations: Vec::new(),
            executor_scorecards: Vec::new(),
            executor_scorecards_source: None,
            posterior_fingerprint: "fp-structural".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "trend".to_string(),
            posterior_confidence: Some(0.72),
            posterior_probabilities: std::collections::BTreeMap::from([
                ("trend".to_string(), 0.72),
                ("range".to_string(), 0.18),
                ("transition".to_string(), 0.10),
            ]),
            posterior_evidence: vec!["mtf=aligned".to_string()],
        });
        let history = vec![
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[0].clone(),
            sample_structural_feedback_history()[1].clone(),
        ];

        let agent_value = build_agent_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );
        let human_value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert_eq!(
            agent_value["recommended_next_step"]["execution_contract"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );
        assert_eq!(
            human_value["recommended_next_step"]["execution_contract"]["path_id"],
            "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
        );

        let blocked_human = build_human_workflow_status_view_with_provider_agent(
            &sample_human_workflow_snapshot(),
            &[],
            &sample_provider_agent_surface(),
            &history,
        );
        let blocked_agent = build_agent_workflow_status_view_with_provider_agent(
            &sample_human_workflow_snapshot(),
            &[],
            &sample_provider_agent_surface(),
            &history,
        );

        assert!(blocked_human["recommended_next_step"]["execution_contract"].is_null());
        assert!(blocked_agent["recommended_next_step"]["execution_contract"].is_null());
    }

    #[test]
    fn human_workflow_status_view_exposes_structural_feedback_line() {
        let mut snapshot = sample_human_workflow_snapshot();
        if let Some(update) = snapshot.latest_update.as_mut() {
            update.structural_feedback = Some(crate::state::StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: "structural-feedback:NQ:node:path".to_string(),
                recommended_at: "2026-04-29T00:00:00Z".to_string(),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                    .to_string(),
                path_id: "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
                    .to_string(),
                followed_path: true,
                exit_reason: Some("target_hit".to_string()),
                notes: Some("user followed structural path".to_string()),
            });
        }

        let value = build_human_workflow_status_view_with_provider_agent(
            &snapshot,
            &[],
            &sample_provider_agent_surface(),
            &[],
        );

        assert!(value["structural_feedback_line"]
            .as_str()
            .unwrap()
            .contains("recommendation=structural-feedback:NQ:node:path"));
        assert!(value["structural_feedback_line"]
            .as_str()
            .unwrap()
            .contains(
                "path=path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary"
            ));
    }

    #[test]
    fn ensemble_vote_history_view_uses_resolved_scorecard_source() {
        let vote = sample_human_workflow_snapshot()
            .latest_ensemble_vote
            .expect("sample ensemble vote");
        let mut vote = vote;
        vote.executor_summaries = vec![
            "executor=catboost_file action=observe confidence=0.500 policy_source=catboost_file:placeholder".to_string(),
            "executor=xgboost_file action=observe confidence=0.450 policy_source=xgboost_file:sample_file".to_string(),
        ];
        let persisted = vec![EnsembleExecutorScorecard {
            executor: "xgboost_file".to_string(),
            latest_weight_hint: Some(0.80),
            ..EnsembleExecutorScorecard::default()
        }];
        let value = build_ensemble_vote_history_view(
            &WorkflowSnapshot {
                recent_ensemble_votes: vec![vote.clone()],
                ..WorkflowSnapshot::default()
            },
            &persisted,
        );
        assert_eq!(value.history[0].executor_scorecard_source, "persisted");
        assert_eq!(
            value.history[0].executor_scorecards[0].executor,
            "xgboost_file"
        );
        assert_eq!(value.hard_block_only[0].artifact_id, vote.artifact_id);
        assert_eq!(value.hard_block_summary.count, 1);
        assert_eq!(
            value.history[0].policy_runtime_line.as_deref(),
            Some("Policy runtime: catboost_file:placeholder, xgboost_file:sample_file")
        );
    }

    #[test]
    fn auxiliary_artifact_surfaces_clone_snapshot_fields() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.artifact_history_summary.total_entries = 3;
        snapshot.artifact_factor_trends = vec![ArtifactFactorTrendSummary::default()];
        snapshot.artifact_family_trends = vec![ArtifactFamilyTrendSummary::default()];
        let surfaces = build_auxiliary_artifact_surfaces(&snapshot);
        assert_eq!(surfaces.artifact_history_summary.total_entries, 3);
        assert_eq!(surfaces.artifact_factor_trends.len(), 1);
        assert_eq!(surfaces.artifact_family_trends.len(), 1);
    }

    #[test]
    fn artifact_report_surfaces_build_expected_views() {
        let mut snapshot = WorkflowSnapshot::default();
        snapshot.artifact_decision_summary.consumed_trend_status =
            "validated_regressing".to_string();
        snapshot.artifact_decision_summary.consumed_trend_reason = "quality_down".to_string();
        snapshot.artifact_lineage_summaries = vec![ArtifactLineageSummary {
            review_rule_break_count: 1,
            ..ArtifactLineageSummary::default()
        }];
        snapshot.artifact_factor_trends = vec![ArtifactFactorTrendSummary {
            factor_name: "f1".to_string(),
            consumed_entries: 2,
            entries: 3,
            ..ArtifactFactorTrendSummary::default()
        }];
        snapshot.artifact_family_trends = vec![ArtifactFamilyTrendSummary {
            family: "fam".to_string(),
            consumed_entries: 1,
            entries: 2,
            ..ArtifactFamilyTrendSummary::default()
        }];
        let report = build_artifact_report_surfaces(&snapshot);
        assert_eq!(
            report.artifact_consumed_gate["status"],
            "validated_regressing"
        );
        assert_eq!(report.artifact_rule_breaks.len(), 1);
        assert_eq!(
            report.artifact_factor_consumed_validation[0].factor_name,
            "f1"
        );
        assert_eq!(report.artifact_family_consumed_validation[0].family, "fam");
    }

    #[test]
    fn pre_bayes_surfaces_clone_snapshot_fields() {
        let mut snapshot = WorkflowSnapshot {
            latest_pre_bayes_policy: Some(PreBayesEvidencePolicy {
                version: "v2".to_string(),
                ..PreBayesEvidencePolicy::default()
            }),
            recent_pre_bayes_policies: vec![PreBayesPolicyRecord {
                policy: PreBayesEvidencePolicy {
                    version: "v1".to_string(),
                    ..PreBayesEvidencePolicy::default()
                },
                ..PreBayesPolicyRecord::default()
            }],
            latest_pre_bayes_policy_diff: Some(PreBayesPolicyDiff::default()),
            latest_pre_bayes_policy_lineage: Some(PreBayesPolicyLineageSummary::default()),
            latest_pre_bayes_entry_quality_bridge: Some(PreBayesEntryQualityBridge::default()),
            latest_pre_bayes_entry_quality_bridge_diff: Some(
                PreBayesEntryQualityBridgeDiff::default(),
            ),
            latest_pre_bayes_soft_evidence_diff: vec![PreBayesSoftEvidenceNodeDiff::default()],
            ..WorkflowSnapshot::default()
        };
        let mut analyze = crate::state::WorkflowPhaseSnapshot::default();
        let mut soft_evidence = std::collections::BTreeMap::new();
        soft_evidence.insert("a".to_string(), {
            let mut inner = std::collections::BTreeMap::new();
            inner.insert("b".to_string(), 0.5);
            inner
        });
        analyze.pre_bayes_soft_evidence = soft_evidence;
        snapshot.latest_analyze = Some(analyze);

        let pre = build_pre_bayes_surfaces(&snapshot);
        assert_eq!(pre.pre_bayes_policy.as_ref().unwrap().version, "v2");
        assert_eq!(pre.pre_bayes_policy_history.len(), 1);
        assert_eq!(pre.pre_bayes_policy_history[0].policy.version, "v1");
        assert!(pre.pre_bayes_policy_diff.is_some());
        assert!(pre.pre_bayes_policy_lineage.is_some());
        assert!(pre.pre_bayes_entry_quality_bridge.is_some());
        assert!(pre.pre_bayes_entry_quality_bridge_diff.is_some());
        assert_eq!(pre.pre_bayes_soft_evidence_diff.len(), 1);
        assert_eq!(
            pre.pre_bayes_soft_evidence
                .as_ref()
                .unwrap()
                .get("a")
                .unwrap()
                .get("b"),
            Some(&0.5)
        );
    }

    #[test]
    fn phase_snapshot_surfaces_clone_snapshot_fields() {
        let snapshot = WorkflowSnapshot {
            latest_train: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "train".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_analyze: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "analyze".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_research: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "research".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_backtest: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "backtest".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            latest_update: Some(crate::state::WorkflowPhaseSnapshot {
                phase: "update".to_string(),
                ..crate::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };
        let phases = build_phase_snapshot_surfaces(&snapshot);
        assert_eq!(phases.train.as_ref().unwrap().phase, "train");
        assert_eq!(phases.analyze.as_ref().unwrap().phase, "analyze");
        assert_eq!(phases.research.as_ref().unwrap().phase, "research");
        assert_eq!(phases.backtest.as_ref().unwrap().phase, "backtest");
        assert_eq!(phases.update.as_ref().unwrap().phase, "update");
    }

    #[test]
    fn factor_autoresearch_status_empty_state_returns_explicit_no_state_contract() {
        let value = factor_autoresearch_status_value_for_empty_state("DEMO", "state");
        assert_eq!(value["status"], "no_autoresearch_state");
        assert!(value["live_snapshot"].is_null());
        assert!(value["sessions"].as_array().unwrap().is_empty());
        assert!(value["attempts"].as_array().unwrap().is_empty());
        assert!(value["recommended_next_step"]
            .as_str()
            .unwrap()
            .contains("ask-user: Provide a historical data file path"));
        assert!(value["recommended_next_step"]
            .as_str()
            .unwrap()
            .contains("then ict-engine factor-autoresearch --symbol DEMO --data <historical-data.json> --state-dir state"));
    }

    #[test]
    fn human_workflow_status_hides_unrelated_personal_lane_for_non_matching_symbol() {
        let value = build_human_workflow_status_view_with_provider_agent(
            &WorkflowSnapshot {
                symbol: "NEWSYM".to_string(),
                ..WorkflowSnapshot::default()
            },
            &[],
            &sample_provider_agent_surface(),
            &[],
        );
        assert!(value["opt_in_profile_line"].is_null());
    }
}
