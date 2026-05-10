use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::application::belief::{
    blend_branch_prior_with_transition_prior, blend_node_posterior_with_duration_prior,
    transition_adjusted_branch_posteriors,
};
use crate::application::provider_catalog::{
    build_workflow_provider_support, ProviderCatalogAgentSurface,
};
pub use crate::belief_core::ranking_label::{
    apply_structural_path_probability_bins, apply_structural_path_probability_calibration,
    apply_structural_path_ranking_execution_gates,
    clear_structural_path_ranking_target_row_outputs,
    evaluate_structural_path_probability_calibration_rows,
    load_structural_path_ranker_runtime_artifact_metadata,
    load_structural_path_ranker_runtime_artifact_rows,
    load_structural_path_ranking_runtime_selection, load_structural_path_ranking_target_rows,
    render_structural_path_ranking_target_csv, render_structural_path_ranking_target_jsonl,
    render_structural_path_ranking_target_rows_csv,
    render_structural_path_ranking_target_rows_jsonl,
    score_structural_path_ranker_runtime_rows_with_direct_model,
    score_structural_path_ranker_runtime_rows_with_explicit_family,
    score_structural_path_ranker_runtime_rows_with_service,
    structural_path_ranker_supports_direct_model_family,
    structural_path_ranker_supports_explicit_family,
    structural_path_ranker_supports_service_family, structural_path_ranking_beta_lower_bound,
    structural_path_ranking_beta_mean, structural_path_ranking_ips_weight,
    structural_path_ranking_propensity_estimate,
    structural_path_ranking_propensity_evaluation_weight, structural_path_ranking_reward_label,
    structural_path_ranking_runtime_selection_path, structural_path_ranking_target_export_summary,
    structural_path_ranking_target_row_history_key, structural_path_ranking_target_row_score_key,
    structural_path_ranking_trainer_manifest, structural_path_ranking_training_weight,
    upsert_structural_path_ranking_target_history, StructuralPathProbabilityCalibrationBin,
    StructuralPathProbabilityCalibrationEvaluationBin,
    StructuralPathProbabilityCalibrationEvaluationReport,
    StructuralPathProbabilityCalibrationReport, StructuralPathRankerRuntimeRow,
    StructuralPathRankerRuntimeSurface, StructuralPathRankingExternalScoreInput,
    StructuralPathRankingRuntimeSelection, StructuralPathRankingTargetArtifact,
    StructuralPathRankingTargetExportSummary, StructuralPathRankingTargetExportSummaryInput,
    StructuralPathRankingTargetRow, StructuralPathRankingTrainerManifest,
    STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
    STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY,
    STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_FILE,
    STRUCTURAL_PATH_RANKING_RUNTIME_SELECTION_PROTOCOL_VERSION,
};
pub use crate::belief_core::regime_filter::StructuralTemporalSummaryArtifact;
use crate::belief_core::regime_filter::StructuralTemporalSummaryArtifactInput;
pub use crate::belief_core::regime_filter::{
    build_structural_temporal_summary_artifact, structural_duration_avg_streak_length,
    structural_duration_bocpd_break_probability, structural_duration_bocpd_continue_probability,
    structural_duration_bocpd_evidence_weight, structural_duration_bocpd_raw_break_probability,
    structural_duration_bocpd_recursive_reset_probability,
    structural_duration_bocpd_recursive_run_length_entropy,
    structural_duration_bocpd_recursive_run_length_expected_value,
    structural_duration_bocpd_recursive_run_length_mode,
    structural_duration_bocpd_recursive_run_length_mode_probability,
    structural_duration_bocpd_run_length_mode,
    structural_duration_bocpd_run_length_mode_probability,
    structural_duration_bocpd_run_length_observation_mass,
    structural_duration_bocpd_run_length_tail_probability,
    structural_duration_bocpd_sequence_break_probability,
    structural_duration_bocpd_sequence_change_intensity,
    structural_duration_bocpd_sequence_recursive_reset_probability,
    structural_duration_bocpd_sequence_recursive_run_length_entropy,
    structural_duration_bocpd_sequence_recursive_run_length_expected_value,
    structural_duration_bocpd_sequence_recursive_run_length_mode,
    structural_duration_bocpd_sequence_recursive_run_length_mode_probability,
    structural_duration_bocpd_surprise, structural_duration_break_hazard,
    structural_duration_distribution_entropy, structural_duration_empirical_completion_hazard,
    structural_duration_empirical_survival, structural_duration_expected_dwell_steps,
    structural_duration_outcome_support, structural_duration_persistence_prior,
    structural_duration_remaining_dwell_steps, structural_duration_sticky_self_transition_strength,
    structural_duration_streak_count, structural_duration_temporal_posterior_support,
    structural_duration_weighted_streak_mass,
};
pub use crate::belief_core::source_reliability::{
    structural_branch_history_invalidation_rate, structural_branch_history_win_rate,
    structural_composite_preference_score, structural_delayed_reward_replay_validation,
    structural_dominant_source_panel, structural_experience_prior_runtime_metrics,
    structural_history_adjusted_branch_prior, structural_history_adjusted_node_prior,
    structural_history_adjusted_path_prior, structural_history_adjusted_scenario_prior,
    structural_history_invalidation_rate, structural_history_win_rate,
    structural_last_offline_seed_source, structural_node_history_invalidation_rate,
    structural_node_history_win_rate, structural_panel_derived_smoothed_prior,
    structural_prior_behavior_policy_probability,
    structural_prior_behavior_policy_probability_variance,
    structural_prior_censoring_adjusted_reward_lower_bound,
    structural_prior_censoring_adjusted_reward_prior, structural_prior_censoring_rate,
    structural_prior_counterfactual_reward_prior,
    structural_prior_delayed_reward_abandonment_competing_risk,
    structural_prior_delayed_reward_abandonment_cumulative_incidence_4h,
    structural_prior_delayed_reward_abandonment_hazard_per_hour,
    structural_prior_delayed_reward_avg_elapsed_hours,
    structural_prior_delayed_reward_censoring_probability,
    structural_prior_delayed_reward_competing_risk_entropy,
    structural_prior_delayed_reward_elapsed_feedback_count,
    structural_prior_delayed_reward_elapsed_hours_at_risk,
    structural_prior_delayed_reward_expected_resolution_hours,
    structural_prior_delayed_reward_failure_competing_risk,
    structural_prior_delayed_reward_failure_cumulative_incidence_4h,
    structural_prior_delayed_reward_failure_hazard_per_hour,
    structural_prior_delayed_reward_invalidation_competing_risk,
    structural_prior_delayed_reward_invalidation_cumulative_incidence_4h,
    structural_prior_delayed_reward_invalidation_hazard_per_hour,
    structural_prior_delayed_reward_resolution_hazard_per_hour,
    structural_prior_delayed_reward_resolution_horizon_1h_count,
    structural_prior_delayed_reward_resolution_horizon_24h_count,
    structural_prior_delayed_reward_resolution_horizon_4h_count,
    structural_prior_delayed_reward_resolution_probability,
    structural_prior_delayed_reward_resolution_probability_1h,
    structural_prior_delayed_reward_resolution_probability_24h,
    structural_prior_delayed_reward_resolution_probability_4h,
    structural_prior_delayed_reward_resolution_within_1h_count,
    structural_prior_delayed_reward_resolution_within_24h_count,
    structural_prior_delayed_reward_resolution_within_4h_count,
    structural_prior_delayed_reward_success_competing_risk,
    structural_prior_delayed_reward_success_cumulative_incidence_4h,
    structural_prior_delayed_reward_success_hazard_per_hour,
    structural_prior_delayed_reward_survival_probability_1h,
    structural_prior_delayed_reward_survival_probability_24h,
    structural_prior_delayed_reward_survival_probability_4h,
    structural_prior_doubly_robust_reward_prior, structural_prior_execution_propensity,
    structural_prior_ips_weight, structural_prior_matured_feedback_count,
    structural_prior_maturity_coverage, structural_prior_off_policy_adjusted_prior,
    structural_prior_positive_count, structural_prior_positive_value,
    structural_prior_snips_effective_sample_size, structural_prior_snips_reward_prior,
    structural_prior_snips_weight_mass, structural_prior_snips_weight_squared_mass,
    structural_prior_target_policy_calibration_weight,
    structural_prior_target_policy_probability_brier_score,
    structural_prior_target_policy_probability_calibration_error,
    structural_prior_target_policy_probability_confidence,
    structural_prior_target_policy_probability_lower_bound,
    structural_prior_target_policy_reward_lower_bound, structural_prior_target_policy_reward_prior,
    structural_prior_target_policy_variance_penalty, structural_prior_unresolved_feedback_count,
    structural_resolved_avg_pnl, structural_resolved_branch_invalidation_rate,
    structural_resolved_branch_win_rate, structural_resolved_followed_count,
    structural_resolved_node_invalidation_rate, structural_resolved_node_win_rate,
    structural_resolved_observations, structural_resolved_path_invalidation_rate,
    structural_resolved_path_win_rate, structural_resolved_scenario_invalidation_rate,
    structural_resolved_scenario_win_rate, structural_resolved_smoothed_prior,
    structural_scenario_history_invalidation_rate, structural_scenario_history_win_rate,
    structural_source_confusion_concentration_multiplier, structural_source_panel_count,
    structural_source_reliability_em_readiness, structural_source_reliability_multiplier,
    structural_target_policy_context_surface, structural_target_policy_context_surfaces,
    StructuralExperiencePriorEntry, StructuralExperiencePriorSurfaceArtifact,
    StructuralSourceReliabilityEmReadiness, StructuralTargetPolicyContextSurface,
};
pub use crate::belief_core::structural_state::{
    StructuralBranchArtifact, StructuralBranchHistoryArtifact, StructuralBranchOutcomeSummary,
    StructuralBranchSetArtifact, StructuralEntityHistorySummary, StructuralFeedbackField,
    StructuralFeedbackSubmission, StructuralFeedbackTemplateArtifact,
    StructuralHistorySummaryArtifact, StructuralNodeArtifact, StructuralNodeHistoryArtifact,
    StructuralNodeOutcomeSummary, StructuralPathArtifact, StructuralPathHistoryArtifact,
    StructuralPathHistorySummary, StructuralPathOutcomeSummary, StructuralPathPlanArtifact,
    StructuralPlaybookBundle, StructuralRecommendedPathBundleArtifact, StructuralScenarioArtifact,
    StructuralScenarioHistoryArtifact, StructuralScenarioOutcomeSummary,
    StructuralScenarioPlaybookArtifact, StructuralTopPathCandidate,
    StructuralTopPathCandidatesArtifact,
};
use crate::state::{
    recommended_next_command_meta, save_text_state, structural_feedback_learning_outcome,
    structural_feedback_outcome_is_unresolved, FeedbackFactorUsage, FeedbackRecord,
    ModelProbabilitySnapshot, StructuralFeedbackLearningOutcome, StructuralFeedbackRefs,
    StructuralPriorLearningState, WorkflowSnapshot,
};
use crate::types::{Direction, Regime};

#[cfg(test)]
use crate::state::StructuralPriorStats;

const STRUCTURAL_PLAYBOOK_ARTIFACT_VERSION: &str = "structural-playbook-v1";
const STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR: &str = "policy_training";
pub const STRUCTURAL_PATH_RANKING_TARGET_CSV_FILE: &str = "structural_path_ranking_target.csv";
pub const STRUCTURAL_PATH_RANKING_TARGET_JSONL_FILE: &str = "structural_path_ranking_target.jsonl";
pub const STRUCTURAL_PATH_RANKING_TARGET_HISTORY_CSV_FILE: &str =
    "structural_path_ranking_target_history.csv";
pub const STRUCTURAL_PATH_RANKING_TARGET_HISTORY_JSONL_FILE: &str =
    "structural_path_ranking_target_history.jsonl";
pub const STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE: &str =
    "structural_path_ranking_target_summary.json";

#[derive(Debug, Clone, Default)]
pub(crate) struct StructuralPathRankerRuntimeContext<'a> {
    pub(crate) state_dir: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
struct StructuralRankedPathSelection {
    candidate_set_id: String,
    runtime: Option<StructuralPathRankerRuntimeSurface>,
    paths: Vec<StructuralPathArtifact>,
}

#[derive(Debug, Clone)]
struct StructuralPathRankerRuntimeRowMatch {
    source: &'static str,
    row: StructuralPathRankerRuntimeRow,
}

pub fn resolved_latest_ensemble_vote(
    snapshot: &WorkflowSnapshot,
) -> Option<crate::state::EnsembleVoteRecord> {
    snapshot
        .latest_ensemble_vote
        .as_ref()
        .and_then(|vote| resolved_ensemble_vote_for_snapshot(snapshot, vote))
}

pub fn resolved_ensemble_vote_for_snapshot(
    snapshot: &WorkflowSnapshot,
    vote: &crate::state::EnsembleVoteRecord,
) -> Option<crate::state::EnsembleVoteRecord> {
    let mut vote = vote.clone();
    let Some(phase) = matching_phase_snapshot_for_ensemble_vote(snapshot, &vote) else {
        return Some(vote);
    };
    let Some((active_regime, probabilities, confidence)) = canonical_phase_regime_surface(phase)
    else {
        return Some(vote);
    };
    vote.posterior_active_regime = active_regime;
    vote.posterior_probabilities = probabilities;
    vote.posterior_confidence = Some(confidence);
    vote.confidence = confidence;
    vote.consensus_strength = confidence;
    vote.posterior_normalization_status = "canonical_structural_regime_posterior".to_string();
    Some(vote)
}

fn matching_phase_snapshot_for_ensemble_vote<'a>(
    snapshot: &'a WorkflowSnapshot,
    vote: &crate::state::EnsembleVoteRecord,
) -> Option<&'a crate::state::WorkflowPhaseSnapshot> {
    [
        snapshot.latest_update.as_ref(),
        snapshot.latest_research.as_ref(),
        snapshot.latest_analyze.as_ref(),
        snapshot.latest_backtest.as_ref(),
        snapshot.latest_train.as_ref(),
    ]
    .into_iter()
    .flatten()
    .find(|phase| {
        let phase_matches = vote.source_phase == phase.phase
            || (phase.phase == "research" && vote.source_phase == "factor-research")
            || (phase.phase == "backtest" && vote.source_phase == "factor-backtest");
        phase_matches
            && vote
                .source_run_id
                .as_deref()
                .map(|run_id| run_id == phase.run_id)
                .unwrap_or(false)
    })
}

pub fn canonical_phase_regime_surface(
    phase: &crate::state::WorkflowPhaseSnapshot,
) -> Option<(String, std::collections::BTreeMap<String, f64>, f64)> {
    if !phase.canonical_structural_probabilities.is_empty() {
        let active_regime = phase
            .canonical_structural_active_regime
            .clone()
            .or_else(|| {
                phase
                    .canonical_structural_probabilities
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(label, _)| label.clone())
            })?;
        let confidence = phase.canonical_structural_confidence.unwrap_or_else(|| {
            phase
                .canonical_structural_probabilities
                .get(&active_regime)
                .copied()
                .unwrap_or(0.0)
        });
        return Some((
            active_regime,
            phase.canonical_structural_probabilities.clone(),
            confidence,
        ));
    }
    let distribution = phase.pre_bayes_soft_evidence.get("market_regime")?;
    let mut probabilities = std::collections::BTreeMap::new();
    for (label, probability) in distribution {
        if let Some(canonical) = canonical_structural_regime_label(label) {
            *probabilities.entry(canonical).or_insert(0.0) += *probability;
        }
    }
    if probabilities.is_empty() {
        return None;
    }
    let active_regime = phase
        .pre_bayes_filtered_assignments
        .get("market_regime")
        .and_then(|value| canonical_structural_regime_label(value))
        .or_else(|| {
            probabilities
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(label, _)| label.clone())
        })?;
    let confidence = probabilities.get(&active_regime).copied().unwrap_or(0.0);
    Some((active_regime, probabilities, confidence))
}

pub fn canonical_analyze_regime_surface(
    analyze: &crate::state::WorkflowPhaseSnapshot,
) -> Option<(String, std::collections::BTreeMap<String, f64>, f64)> {
    canonical_phase_regime_surface(analyze)
}

pub fn build_structural_playbook_bundle(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
) -> StructuralPlaybookBundle {
    build_structural_playbook_bundle_with_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        &StructuralPriorLearningState::default(),
    )
}

pub fn build_structural_playbook_bundle_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralPlaybookBundle {
    build_structural_playbook_bundle_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        StructuralPathRankerRuntimeContext::default(),
    )
}

pub(crate) fn build_structural_playbook_bundle_with_runtime_context_and_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    runtime_context: StructuralPathRankerRuntimeContext<'_>,
) -> StructuralPlaybookBundle {
    let command = top_level_command(snapshot);
    let support_reason = structural_support_reason(snapshot);
    let provider_support =
        build_workflow_provider_support(provider_status_agent, &command, support_reason.as_deref());
    let focus_phase = structural_focus_phase(snapshot);
    let node = build_structural_node_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        structural_prior_state,
    );
    let branch_history = build_structural_branch_history_artifact(snapshot, feedback_history);
    let scenario_history = build_structural_scenario_history_artifact(snapshot, feedback_history);
    let path_history = build_structural_path_history_artifact(snapshot, feedback_history);
    let branch_set = build_structural_branch_set_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        &node,
        &branch_history,
        structural_prior_state,
    );
    let scenario_playbook = build_structural_scenario_playbook_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        &branch_set,
        &scenario_history,
        structural_prior_state,
    );
    let path_plan = build_structural_path_plan_artifact_with_runtime_context_and_prior_state(
        StructuralPathPlanArtifactInput {
            snapshot,
            provider_status_agent,
            provider_support: &provider_support,
            scenarios: &scenario_playbook,
            feedback_history,
            path_history: &path_history,
            structural_prior_state,
            runtime_context: runtime_context.clone(),
        },
    );
    let feedback_template = build_structural_feedback_template_artifact(
        snapshot,
        &node,
        &branch_set,
        &scenario_playbook,
        &path_plan,
    );
    let recommended_path_bundle =
        build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
            snapshot,
            provider_status_agent,
            feedback_history,
            structural_prior_state,
            runtime_context,
        );
    let history_summary = build_structural_history_summary_artifact(snapshot, feedback_history);
    let node_history = build_structural_node_history_artifact(snapshot, feedback_history);
    StructuralPlaybookBundle {
        artifact_version: STRUCTURAL_PLAYBOOK_ARTIFACT_VERSION.to_string(),
        symbol: structural_symbol(snapshot),
        selected_profile_id: provider_status_agent
            .selected_profile
            .as_ref()
            .map(|profile| profile.profile_id.clone()),
        selected_profile_data_contracts: structural_relevant_profile_data_contracts(
            snapshot,
            provider_status_agent,
        ),
        selected_profile_track_statuses: structural_relevant_profile_track_statuses(
            snapshot,
            provider_status_agent,
        ),
        node: StructuralNodeArtifact {
            focus_phase,
            ..node
        },
        branch_set,
        scenario_playbook,
        path_plan,
        history_summary,
        node_history,
        branch_history,
        scenario_history,
        path_history,
        recommended_path_bundle,
        feedback_template,
    }
}

pub fn build_structural_experience_prior_surface_artifact(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
) -> StructuralExperiencePriorSurfaceArtifact {
    build_structural_experience_prior_surface_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        &StructuralPriorLearningState::default(),
    )
}

fn structural_feedback_records_for_path<'a>(
    feedback_history: &'a [FeedbackRecord],
    path_id: &str,
) -> Vec<&'a FeedbackRecord> {
    feedback_history
        .iter()
        .filter(|record| {
            record
                .structural_feedback
                .as_ref()
                .map(|refs| refs.path_id == path_id)
                .unwrap_or(false)
        })
        .collect()
}

pub fn build_structural_experience_prior_surface_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralExperiencePriorSurfaceArtifact {
    let playbook = build_structural_playbook_bundle_with_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
    );
    let latest_feedback = structural_latest_feedback_refs(snapshot);
    let node_id = latest_feedback
        .as_ref()
        .map(|refs| refs.node_id.as_str())
        .unwrap_or(playbook.node.node_id.as_str());
    let branch_id = latest_feedback
        .as_ref()
        .map(|refs| refs.branch_id.as_str())
        .or_else(|| {
            playbook
                .branch_set
                .branches
                .first()
                .map(|branch| branch.branch_id.as_str())
        });
    let scenario_id = latest_feedback
        .as_ref()
        .map(|refs| refs.scenario_id.as_str())
        .or_else(|| {
            playbook
                .scenario_playbook
                .scenarios
                .first()
                .map(|scenario| scenario.scenario_id.as_str())
        });
    let path_id = latest_feedback
        .as_ref()
        .map(|refs| refs.path_id.as_str())
        .or_else(|| {
            playbook
                .path_plan
                .paths
                .first()
                .map(|path| path.path_id.as_str())
        });
    let node_summary = playbook
        .node_history
        .nodes
        .iter()
        .find(|node| node.node_id == node_id);
    let branch_summary = branch_id.and_then(|id| {
        playbook
            .branch_history
            .branches
            .iter()
            .find(|branch| branch.branch_id == id)
    });
    let scenario_summary = scenario_id.and_then(|id| {
        playbook
            .scenario_history
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == id)
    });
    let path_summary = path_id.and_then(|id| {
        playbook
            .path_history
            .paths
            .iter()
            .find(|path| path.path_id == id)
    });
    let branch = branch_id.and_then(|id| {
        let prior_stats = structural_prior_state.branches.get(id);
        let (dominant_source_panel, dominant_source_share, dominant_source_prior) =
            structural_dominant_source_panel(prior_stats);
        playbook
            .branch_set
            .branches
            .iter()
            .find(|branch| branch.branch_id == id)
            .map(|branch| StructuralExperiencePriorEntry {
                entity_kind: "branch".to_string(),
                entity_id: branch.branch_id.clone(),
                historical_total_records: branch.historical_total_records,
                historical_followed_count: branch.historical_followed_count,
                historical_win_rate: branch.historical_win_rate,
                historical_invalidation_rate: branch.historical_invalidation_rate,
                historical_avg_pnl: branch.historical_avg_pnl,
                experience_prior: branch.prior_probability,
                current_posterior: Some(branch.posterior_probability),
                composite_score: branch.composite_branch_score,
                dominant_source_panel: dominant_source_panel.clone(),
                dominant_source_share,
                dominant_source_prior,
                duration_streak_count: None,
                duration_avg_streak_length: None,
                duration_persistence_prior: None,
                duration_weighted_streak_mass: None,
                transition_weighted_observation_mass: branch.transition_weighted_observation_mass,
                duration_outcome_support: None,
                duration_temporal_posterior_support: None,
                transition_outcome_support: branch.transition_outcome_support,
                transition_temporal_posterior_support: branch.transition_temporal_posterior_support,
                ..structural_experience_prior_runtime_metrics(prior_stats, None)
            })
            .or_else(|| {
                branch_summary.map(|summary| {
                    let experience_prior =
                        structural_history_adjusted_branch_prior(0.5, Some(summary));
                    StructuralExperiencePriorEntry {
                        entity_kind: "branch".to_string(),
                        entity_id: summary.branch_id.clone(),
                        historical_total_records: summary.total_records,
                        historical_followed_count: summary.followed_count,
                        historical_win_rate: structural_branch_history_win_rate(Some(summary)),
                        historical_invalidation_rate: structural_branch_history_invalidation_rate(
                            Some(summary),
                        ),
                        historical_avg_pnl: Some(summary.avg_pnl),
                        experience_prior,
                        current_posterior: None,
                        composite_score: experience_prior,
                        dominant_source_panel: dominant_source_panel.clone(),
                        dominant_source_share,
                        dominant_source_prior,
                        duration_streak_count: None,
                        duration_avg_streak_length: None,
                        duration_persistence_prior: None,
                        duration_weighted_streak_mass: None,
                        transition_weighted_observation_mass: None,
                        duration_outcome_support: None,
                        duration_temporal_posterior_support: None,
                        transition_outcome_support: None,
                        transition_temporal_posterior_support: None,
                        ..structural_experience_prior_runtime_metrics(prior_stats, None)
                    }
                })
            })
    });
    let scenario = scenario_id.and_then(|id| {
        let prior_stats = structural_prior_state.scenarios.get(id);
        let (dominant_source_panel, dominant_source_share, dominant_source_prior) =
            structural_dominant_source_panel(prior_stats);
        playbook
            .scenario_playbook
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == id)
            .map(|scenario| StructuralExperiencePriorEntry {
                entity_kind: "scenario".to_string(),
                entity_id: scenario.scenario_id.clone(),
                historical_total_records: scenario.historical_total_records,
                historical_followed_count: scenario.historical_followed_count,
                historical_win_rate: scenario.historical_win_rate,
                historical_invalidation_rate: scenario.historical_invalidation_rate,
                historical_avg_pnl: scenario.historical_avg_pnl,
                experience_prior: scenario.prior_probability,
                current_posterior: Some(scenario.posterior_probability),
                composite_score: scenario.composite_scenario_score,
                dominant_source_panel: dominant_source_panel.clone(),
                dominant_source_share,
                dominant_source_prior,
                duration_streak_count: None,
                duration_avg_streak_length: None,
                duration_persistence_prior: None,
                duration_weighted_streak_mass: None,
                transition_weighted_observation_mass: None,
                duration_outcome_support: None,
                duration_temporal_posterior_support: None,
                transition_outcome_support: None,
                transition_temporal_posterior_support: None,
                ..structural_experience_prior_runtime_metrics(prior_stats, None)
            })
            .or_else(|| {
                scenario_summary.map(|summary| {
                    let experience_prior =
                        structural_history_adjusted_scenario_prior(0.5, Some(summary));
                    StructuralExperiencePriorEntry {
                        entity_kind: "scenario".to_string(),
                        entity_id: summary.scenario_id.clone(),
                        historical_total_records: summary.total_records,
                        historical_followed_count: summary.followed_count,
                        historical_win_rate: structural_scenario_history_win_rate(Some(summary)),
                        historical_invalidation_rate: structural_scenario_history_invalidation_rate(
                            Some(summary),
                        ),
                        historical_avg_pnl: Some(summary.avg_pnl),
                        experience_prior,
                        current_posterior: None,
                        composite_score: experience_prior,
                        dominant_source_panel: dominant_source_panel.clone(),
                        dominant_source_share,
                        dominant_source_prior,
                        duration_streak_count: None,
                        duration_avg_streak_length: None,
                        duration_persistence_prior: None,
                        duration_weighted_streak_mass: None,
                        transition_weighted_observation_mass: None,
                        duration_outcome_support: None,
                        duration_temporal_posterior_support: None,
                        transition_outcome_support: None,
                        transition_temporal_posterior_support: None,
                        ..structural_experience_prior_runtime_metrics(prior_stats, None)
                    }
                })
            })
    });
    let path = path_id.and_then(|id| {
        let prior_stats = structural_prior_state.paths.get(id);
        let delayed_reward_replay_validation = structural_delayed_reward_replay_validation(
            &structural_feedback_records_for_path(feedback_history, id),
        );
        let (dominant_source_panel, dominant_source_share, dominant_source_prior) =
            structural_dominant_source_panel(prior_stats);
        playbook
            .path_plan
            .paths
            .iter()
            .find(|path| path.path_id == id)
            .map(|path| StructuralExperiencePriorEntry {
                entity_kind: "path".to_string(),
                entity_id: path.path_id.clone(),
                historical_total_records: path.historical_total_records,
                historical_followed_count: path.historical_followed_count,
                historical_win_rate: path.historical_win_rate,
                historical_invalidation_rate: path.historical_invalidation_rate,
                historical_avg_pnl: path.historical_avg_pnl,
                experience_prior: path.path_prior,
                current_posterior: Some(path.path_posterior),
                composite_score: path.composite_preference_score,
                dominant_source_panel: dominant_source_panel.clone(),
                dominant_source_share,
                dominant_source_prior,
                duration_streak_count: None,
                duration_avg_streak_length: None,
                duration_persistence_prior: None,
                duration_weighted_streak_mass: None,
                transition_weighted_observation_mass: None,
                duration_outcome_support: None,
                duration_temporal_posterior_support: None,
                transition_outcome_support: None,
                transition_temporal_posterior_support: None,
                ..structural_experience_prior_runtime_metrics(
                    prior_stats,
                    delayed_reward_replay_validation.clone(),
                )
            })
            .or_else(|| {
                path_summary.map(|summary| {
                    let experience_prior =
                        structural_history_adjusted_path_prior(0.5, Some(summary));
                    StructuralExperiencePriorEntry {
                        entity_kind: "path".to_string(),
                        entity_id: summary.path_id.clone(),
                        historical_total_records: summary.total_records,
                        historical_followed_count: summary.followed_count,
                        historical_win_rate: structural_history_win_rate(Some(summary)),
                        historical_invalidation_rate: structural_history_invalidation_rate(Some(
                            summary,
                        )),
                        historical_avg_pnl: Some(summary.avg_pnl),
                        experience_prior,
                        current_posterior: None,
                        composite_score: experience_prior,
                        dominant_source_panel: dominant_source_panel.clone(),
                        dominant_source_share,
                        dominant_source_prior,
                        duration_streak_count: None,
                        duration_avg_streak_length: None,
                        duration_persistence_prior: None,
                        duration_weighted_streak_mass: None,
                        transition_weighted_observation_mass: None,
                        duration_outcome_support: None,
                        duration_temporal_posterior_support: None,
                        transition_outcome_support: None,
                        transition_temporal_posterior_support: None,
                        ..structural_experience_prior_runtime_metrics(
                            prior_stats,
                            delayed_reward_replay_validation.clone(),
                        )
                    }
                })
            })
    });
    let node_prior_stats = structural_prior_state.nodes.get(node_id);
    let (dominant_source_panel, dominant_source_share, dominant_source_prior) =
        structural_dominant_source_panel(node_prior_stats);
    let node_duration_prior = structural_prior_state.node_duration_priors.get(node_id);
    let node_temporal_state = structural_prior_state.node_temporal_posteriors.get(node_id);
    StructuralExperiencePriorSurfaceArtifact {
        symbol: structural_symbol(snapshot),
        source_reliability_em: structural_source_reliability_em_readiness(structural_prior_state),
        target_policy_contexts: structural_target_policy_context_surfaces(structural_prior_state),
        node: Some(StructuralExperiencePriorEntry {
            entity_kind: "node".to_string(),
            entity_id: node_id.to_string(),
            historical_total_records: node_summary
                .map(|summary| summary.total_records)
                .unwrap_or(0),
            historical_followed_count: node_summary
                .map(|summary| summary.followed_count)
                .unwrap_or(0),
            historical_win_rate: structural_resolved_node_win_rate(
                structural_prior_state.nodes.get(node_id),
                node_summary,
            ),
            historical_invalidation_rate: structural_resolved_node_invalidation_rate(
                structural_prior_state.nodes.get(node_id),
                node_summary,
            ),
            historical_avg_pnl: structural_resolved_avg_pnl(
                structural_prior_state.nodes.get(node_id),
                node_summary.map(|summary| summary.avg_pnl),
            ),
            experience_prior: structural_resolved_smoothed_prior(
                structural_prior_state.nodes.get(node_id),
                structural_prior_state,
                structural_history_adjusted_node_prior(playbook.node.belief_prior, node_summary),
            ),
            current_posterior: Some(playbook.node.posterior_confidence),
            composite_score: structural_composite_preference_score(
                playbook.node.posterior_confidence,
                structural_resolved_smoothed_prior(
                    structural_prior_state.nodes.get(node_id),
                    structural_prior_state,
                    structural_history_adjusted_node_prior(
                        playbook.node.belief_prior,
                        node_summary,
                    ),
                ),
            ),
            dominant_source_panel,
            dominant_source_share,
            dominant_source_prior,
            duration_streak_count: node_temporal_state
                .map(|state| state.streak_count)
                .or_else(|| structural_duration_streak_count(node_duration_prior)),
            duration_avg_streak_length: structural_duration_avg_streak_length(node_duration_prior),
            duration_persistence_prior: structural_duration_persistence_prior(node_duration_prior),
            duration_weighted_streak_mass: node_temporal_state
                .map(|state| state.weighted_streak_mass)
                .or_else(|| structural_duration_weighted_streak_mass(node_duration_prior)),
            transition_weighted_observation_mass: None,
            duration_outcome_support: node_temporal_state
                .map(|state| state.duration_outcome_support)
                .or_else(|| structural_duration_outcome_support(node_duration_prior)),
            duration_temporal_posterior_support: node_temporal_state
                .map(|state| state.temporal_posterior_support)
                .or_else(|| structural_duration_temporal_posterior_support(node_duration_prior)),
            transition_outcome_support: None,
            transition_temporal_posterior_support: None,
            ..structural_experience_prior_runtime_metrics(node_prior_stats, None)
        }),
        branch,
        scenario,
        path,
    }
}

pub fn build_structural_temporal_summary_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralTemporalSummaryArtifact {
    let node = build_structural_node_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        structural_prior_state,
    );
    let node_duration_prior = structural_prior_state
        .node_duration_priors
        .get(&node.node_id);
    let node_temporal_state = structural_prior_state
        .node_temporal_posteriors
        .get(&node.node_id);
    let active_regime = structural_active_regime(snapshot);
    let to_branch_id = active_regime.as_ref().map(|regime| {
        format!(
            "{}:{}",
            node.node_id,
            structural_branch_label_for_regime(regime)
        )
    });
    let latest_feedback = structural_latest_feedback_refs(snapshot);
    let branch_temporal_state = latest_feedback.as_ref().and_then(|refs| {
        to_branch_id.as_ref().and_then(|branch_id| {
            structural_prior_state
                .branch_temporal_posteriors
                .get(&format!("{}=>{}", refs.branch_id, branch_id))
        })
    });
    let node_transition_state = latest_feedback.as_ref().and_then(|refs| {
        structural_prior_state
            .node_transition_posteriors
            .get(&format!("{}=>{}", refs.node_id, node.node_id))
    });
    let transition_prior = latest_feedback.as_ref().and_then(|refs| {
        to_branch_id.as_ref().and_then(|branch_id| {
            structural_branch_transition_prior(structural_prior_state, &refs.branch_id, branch_id)
        })
    });
    build_structural_temporal_summary_artifact(StructuralTemporalSummaryArtifactInput {
        symbol: structural_symbol(snapshot),
        node_id: node.node_id,
        from_branch_id: latest_feedback.as_ref().map(|refs| refs.branch_id.clone()),
        to_branch_id,
        node_duration_prior,
        node_temporal_state,
        branch_temporal_state,
        node_transition_state,
        transition_prior,
    })
}

pub fn build_structural_top_path_candidates_artifact(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
) -> StructuralTopPathCandidatesArtifact {
    let candidate_paths =
        structural_ranked_paths(snapshot, provider_status_agent, feedback_history)
            .into_iter()
            .take(3)
            .collect::<Vec<_>>();
    let symbol = structural_symbol(snapshot);
    let candidate_set_id = structural_candidate_set_id(&symbol, &candidate_paths);
    let denominator = structural_candidate_policy_denominator(&candidate_paths);
    let candidate_count = candidate_paths.len();
    let candidates = candidate_paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let behavior_policy_probability = structural_candidate_policy_probability(
                path.composite_preference_score,
                denominator,
                candidate_count,
            );
            StructuralTopPathCandidate {
                rank: index + 1,
                candidate_set_id: candidate_set_id.clone(),
                behavior_policy_probability,
                path_id: path.path_id,
                scenario_id: path.scenario_id,
                path_label: path.path_label,
                direction: path.direction,
                experience_prior: path.path_prior,
                current_posterior: path.path_posterior,
                composite_score: path.composite_preference_score,
                historical_total_records: path.historical_total_records,
                historical_followed_count: path.historical_followed_count,
                historical_invalidation_rate: path.historical_invalidation_rate,
                path_ranker_raw_score: path.catboost_score,
                path_ranker_calibrated_path_prob: path.path_ranker_calibrated_path_prob,
                path_ranker_path_prob_lower_bound: path.path_ranker_path_prob_lower_bound,
                path_ranker_execution_gate_status: path.path_ranker_execution_gate_status,
                path_ranker_runtime_source: path.path_ranker_runtime_source,
                recommended_command: path.recommended_command,
            }
        })
        .collect::<Vec<_>>();
    StructuralTopPathCandidatesArtifact {
        symbol,
        candidate_set_id,
        candidate_count,
        path_ranker_runtime: None,
        candidates,
    }
}

pub fn build_structural_top_path_candidates_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralTopPathCandidatesArtifact {
    build_structural_top_path_candidates_artifact_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        StructuralPathRankerRuntimeContext::default(),
    )
}

pub(crate) fn build_structural_top_path_candidates_artifact_with_runtime_context_and_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    runtime_context: StructuralPathRankerRuntimeContext<'_>,
) -> StructuralTopPathCandidatesArtifact {
    let selection = structural_ranked_paths_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        runtime_context,
    );
    let candidate_paths = selection.paths.into_iter().take(3).collect::<Vec<_>>();
    let symbol = structural_symbol(snapshot);
    let candidate_set_id = if selection.candidate_set_id.is_empty() {
        structural_candidate_set_id(&symbol, &candidate_paths)
    } else {
        selection.candidate_set_id.clone()
    };
    let denominator = structural_candidate_policy_denominator(&candidate_paths);
    let candidate_count = candidate_paths.len();
    let candidates = candidate_paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let behavior_policy_probability = structural_candidate_policy_probability(
                path.composite_preference_score,
                denominator,
                candidate_count,
            );
            StructuralTopPathCandidate {
                rank: index + 1,
                candidate_set_id: candidate_set_id.clone(),
                behavior_policy_probability,
                path_id: path.path_id,
                scenario_id: path.scenario_id,
                path_label: path.path_label,
                direction: path.direction,
                experience_prior: path.path_prior,
                current_posterior: path.path_posterior,
                composite_score: path.composite_preference_score,
                historical_total_records: path.historical_total_records,
                historical_followed_count: path.historical_followed_count,
                historical_invalidation_rate: path.historical_invalidation_rate,
                path_ranker_raw_score: path.catboost_score,
                path_ranker_calibrated_path_prob: path.path_ranker_calibrated_path_prob,
                path_ranker_path_prob_lower_bound: path.path_ranker_path_prob_lower_bound,
                path_ranker_execution_gate_status: path.path_ranker_execution_gate_status,
                path_ranker_runtime_source: path.path_ranker_runtime_source,
                recommended_command: path.recommended_command,
            }
        })
        .collect::<Vec<_>>();
    StructuralTopPathCandidatesArtifact {
        symbol,
        candidate_set_id,
        candidate_count,
        path_ranker_runtime: selection.runtime.clone(),
        candidates,
    }
}

pub fn build_structural_path_ranking_target_artifact(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
) -> StructuralPathRankingTargetArtifact {
    build_structural_path_ranking_target_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        &StructuralPriorLearningState::default(),
    )
}

pub fn build_structural_path_ranking_target_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralPathRankingTargetArtifact {
    build_structural_path_ranking_target_artifact_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        StructuralPathRankerRuntimeContext::default(),
    )
}

fn structural_path_ranking_target_artifact_from_candidates(
    snapshot: &WorkflowSnapshot,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    candidate_paths: Vec<StructuralPathArtifact>,
    candidate_set_id: Option<String>,
) -> StructuralPathRankingTargetArtifact {
    let symbol = structural_symbol(snapshot);
    let candidate_set_id =
        candidate_set_id.unwrap_or_else(|| structural_candidate_set_id(&symbol, &candidate_paths));
    let denominator = structural_candidate_policy_denominator(&candidate_paths);
    let candidate_set_size = candidate_paths.len();
    let regime_calibration_bucket = structural_path_ranking_regime_bucket(snapshot);
    let rows = candidate_paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let behavior_policy_probability = structural_candidate_policy_probability(
                path.composite_preference_score,
                denominator,
                candidate_set_size,
            );
            let pending_reward_state =
                structural_path_ranking_pending_reward_state(&path.path_id, feedback_history);
            let calibrated_label = structural_path_ranking_reward_label(&pending_reward_state);
            let maturity_mask = calibrated_label.is_some();
            let maturity_weight = if maturity_mask { 1.0 } else { 0.0 };
            let propensity_estimate = structural_path_ranking_propensity_estimate(
                path.execution_propensity,
                behavior_policy_probability,
            );
            let ips_weight = structural_path_ranking_ips_weight(propensity_estimate);
            let training_weight = structural_path_ranking_training_weight(
                calibrated_label,
                maturity_weight,
                ips_weight,
            );
            let prior_stats = structural_prior_state.paths.get(&path.path_id);
            StructuralPathRankingTargetRow {
                rank: index + 1,
                candidate_set_id: candidate_set_id.clone(),
                candidate_set_size,
                path_id: path.path_id.clone(),
                scenario_id: path.scenario_id,
                path_label: path.path_label,
                direction: path.direction,
                raw_path_score: path.catboost_score,
                calibrated_path_prob: None,
                path_prob_lower_bound: None,
                execution_gate_status: None,
                execution_gate_min_path_prob: None,
                execution_gate_reason: None,
                pending_reward_state,
                maturity_mask,
                maturity_weight,
                calibrated_label,
                propensity_estimate,
                ips_weight,
                training_weight,
                regime_calibration_bucket: regime_calibration_bucket.clone(),
                behavior_policy_probability,
                execution_propensity: path.execution_propensity,
                target_policy_probability_confidence:
                    structural_prior_target_policy_probability_confidence(prior_stats),
                target_policy_probability_lower_bound:
                    structural_prior_target_policy_probability_lower_bound(prior_stats),
                target_policy_reward_prior: structural_prior_target_policy_reward_prior(
                    prior_stats,
                ),
                target_policy_reward_lower_bound: structural_prior_target_policy_reward_lower_bound(
                    prior_stats,
                ),
                experience_prior: path.path_prior,
                current_posterior: path.path_posterior,
                structural_baseline_score: path.composite_preference_score,
                score_model_family: None,
                score_source_kind: None,
                score_model_artifact_uri: None,
                score_generator: None,
            }
        })
        .collect::<Vec<_>>();
    let mut artifact = StructuralPathRankingTargetArtifact {
        protocol_version: "structural-path-ranking-target-v1".to_string(),
        symbol,
        candidate_set_id,
        candidate_set_size,
        generated_at: snapshot
            .generated_at
            .to_rfc3339_opts(SecondsFormat::Secs, true),
        rows,
    };
    apply_structural_path_probability_calibration(&mut artifact);
    artifact
}

fn structural_path_ranking_pending_reward_state_from_feedback(
    record: &FeedbackRecord,
    refs: &StructuralFeedbackRefs,
) -> Option<String> {
    if structural_feedback_outcome_is_unresolved(&record.realized_outcome) {
        return None;
    }
    if !refs.followed_path
        || record
            .realized_outcome
            .trim()
            .eq_ignore_ascii_case("not_followed")
    {
        return None;
    }
    if record
        .realized_outcome
        .trim()
        .eq_ignore_ascii_case("invalidated")
    {
        return Some("matured_invalidated".to_string());
    }
    match structural_feedback_learning_outcome(record) {
        Some(StructuralFeedbackLearningOutcome::Positive) => Some("matured_success".to_string()),
        Some(StructuralFeedbackLearningOutcome::Neutral)
        | Some(StructuralFeedbackLearningOutcome::Negative) => Some("matured_failure".to_string()),
        None => None,
    }
}

fn structural_feedback_runtime_candidate_set_id(
    symbol: &str,
    refs: &StructuralFeedbackRefs,
) -> String {
    let prefix = format!("structural-feedback:{symbol}:");
    if let Some(rest) = refs.recommendation_id.strip_prefix(&prefix) {
        if let Some((candidate_set_id, _)) = rest.split_once(":path:") {
            let candidate_set_id = candidate_set_id.trim();
            if !candidate_set_id.is_empty() {
                return candidate_set_id.to_string();
            }
        }
    }
    format!(
        "structural-feedback-history:{symbol}:{:016x}",
        structural_stable_hash64(&refs.path_id)
    )
}

fn structural_feedback_direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Bull => "bull",
        Direction::Bear => "bear",
        Direction::Neutral => "neutral",
    }
}

fn structural_feedback_regime_bucket(symbol: &str, regime: Regime) -> String {
    let regime = match regime {
        Regime::Accumulation => "accumulation",
        Regime::ManipulationExpansion => "manipulation_expansion",
        Regime::Distribution => "distribution",
    };
    format!("{symbol}:{regime}")
}

fn structural_path_ranking_feedback_target_rows(
    symbol: &str,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> Vec<StructuralPathRankingTargetRow> {
    feedback_history
        .iter()
        .enumerate()
        .filter_map(|(index, record)| {
            if record.symbol.trim() != symbol {
                return None;
            }
            let refs = record.structural_feedback.as_ref()?;
            let pending_reward_state =
                structural_path_ranking_pending_reward_state_from_feedback(record, refs)?;
            let raw_path_score = record
                .model_probabilities_before_trade
                .selected_probability
                .clamp(0.0, 1.0);
            if raw_path_score <= f64::EPSILON {
                return None;
            }
            let calibrated_label = structural_path_ranking_reward_label(&pending_reward_state)?;
            let behavior_policy_probability = raw_path_score;
            let execution_propensity = Some(1.0);
            let propensity_estimate = structural_path_ranking_propensity_estimate(
                execution_propensity,
                behavior_policy_probability,
            );
            let ips_weight = structural_path_ranking_ips_weight(propensity_estimate);
            let maturity_weight = 1.0;
            let training_weight = structural_path_ranking_training_weight(
                Some(calibrated_label),
                maturity_weight,
                ips_weight,
            );
            let prior_stats = structural_prior_state.paths.get(&refs.path_id);
            let experience_prior = prior_stats
                .map(|stats| stats.smoothed_prior.clamp(0.0, 1.0))
                .unwrap_or(raw_path_score);
            let current_posterior =
                structural_prior_target_policy_reward_prior(prior_stats).unwrap_or(raw_path_score);
            Some(StructuralPathRankingTargetRow {
                rank: index + 1,
                candidate_set_id: structural_feedback_runtime_candidate_set_id(symbol, refs),
                candidate_set_size: 1,
                path_id: refs.path_id.clone(),
                scenario_id: refs.scenario_id.clone(),
                path_label: refs.path_id.clone(),
                direction: structural_feedback_direction_label(
                    record.model_probabilities_before_trade.selected_direction,
                )
                .to_string(),
                raw_path_score: Some(raw_path_score),
                calibrated_path_prob: None,
                path_prob_lower_bound: None,
                execution_gate_status: None,
                execution_gate_min_path_prob: None,
                execution_gate_reason: None,
                pending_reward_state,
                maturity_mask: true,
                maturity_weight,
                calibrated_label: Some(calibrated_label),
                propensity_estimate,
                ips_weight,
                training_weight,
                regime_calibration_bucket: structural_feedback_regime_bucket(
                    symbol,
                    record.regime_at_entry,
                ),
                behavior_policy_probability,
                execution_propensity,
                target_policy_probability_confidence:
                    structural_prior_target_policy_probability_confidence(prior_stats),
                target_policy_probability_lower_bound:
                    structural_prior_target_policy_probability_lower_bound(prior_stats),
                target_policy_reward_prior: structural_prior_target_policy_reward_prior(
                    prior_stats,
                ),
                target_policy_reward_lower_bound: structural_prior_target_policy_reward_lower_bound(
                    prior_stats,
                ),
                experience_prior,
                current_posterior,
                structural_baseline_score: raw_path_score,
                score_model_family: None,
                score_source_kind: None,
                score_model_artifact_uri: None,
                score_generator: None,
            })
        })
        .collect()
}

pub(crate) fn build_structural_path_ranking_target_artifact_with_runtime_context_and_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    runtime_context: StructuralPathRankerRuntimeContext<'_>,
) -> StructuralPathRankingTargetArtifact {
    let selection = structural_ranked_paths_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        runtime_context,
    );
    let candidate_paths = selection.paths.into_iter().take(3).collect::<Vec<_>>();
    let symbol = structural_symbol(snapshot);
    let candidate_set_id = if selection.candidate_set_id.is_empty() {
        structural_candidate_set_id(&symbol, &candidate_paths)
    } else {
        selection.candidate_set_id
    };
    structural_path_ranking_target_artifact_from_candidates(
        snapshot,
        feedback_history,
        structural_prior_state,
        candidate_paths,
        Some(candidate_set_id),
    )
}

pub fn export_structural_path_ranking_target(
    state_dir: &str,
    symbol: &str,
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> Result<StructuralPathRankingTargetExportSummary> {
    let mut artifact = build_structural_path_ranking_target_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
    );
    let symbol_dir = Path::new(state_dir)
        .join(symbol)
        .join(STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR);
    fs::create_dir_all(&symbol_dir)?;
    let csv_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_CSV_FILE}"
    );
    let jsonl_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_JSONL_FILE}"
    );
    let history_csv_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_HISTORY_CSV_FILE}"
    );
    let history_jsonl_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_HISTORY_JSONL_FILE}"
    );
    let summary_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE}"
    );
    let history_jsonl_path = Path::new(state_dir).join(symbol).join(&history_jsonl_name);
    let existing_history_rows = load_structural_path_ranking_target_rows(&history_jsonl_path)?;
    let feedback_target_rows = structural_path_ranking_feedback_target_rows(
        symbol,
        feedback_history,
        structural_prior_state,
    );
    let existing_history_score_map = existing_history_rows
        .iter()
        .filter_map(|row| {
            row.raw_path_score
                .map(|score| (structural_path_ranking_target_row_score_key(row), score))
        })
        .collect::<BTreeMap<_, _>>();
    for row in &mut artifact.rows {
        if let Some(raw_score) =
            existing_history_score_map.get(&structural_path_ranking_target_row_score_key(row))
        {
            row.raw_path_score = Some(*raw_score);
            clear_structural_path_ranking_target_row_outputs(row);
        }
    }
    let mut rows_for_history = artifact.rows.clone();
    rows_for_history.extend(feedback_target_rows);
    let history_rows =
        upsert_structural_path_ranking_target_history(&history_jsonl_path, &rows_for_history)?;
    let mut history_artifact = StructuralPathRankingTargetArtifact {
        protocol_version: artifact.protocol_version.clone(),
        symbol: artifact.symbol.clone(),
        candidate_set_id: artifact.candidate_set_id.clone(),
        candidate_set_size: artifact.candidate_set_size,
        generated_at: artifact.generated_at.clone(),
        rows: history_rows,
    };
    let history_report = apply_structural_path_probability_calibration(&mut history_artifact);
    let mut current_artifact = StructuralPathRankingTargetArtifact {
        protocol_version: artifact.protocol_version.clone(),
        symbol: artifact.symbol.clone(),
        candidate_set_id: artifact.candidate_set_id.clone(),
        candidate_set_size: artifact.candidate_set_size,
        generated_at: artifact.generated_at.clone(),
        rows: artifact.rows,
    };
    apply_structural_path_probability_bins(&mut current_artifact.rows, &history_report.bins);
    apply_structural_path_ranking_execution_gates(&mut current_artifact);
    let history_csv = render_structural_path_ranking_target_rows_csv(
        &history_artifact.protocol_version,
        &history_artifact.symbol,
        &history_artifact.generated_at,
        &history_artifact.rows,
    );
    let history_jsonl = render_structural_path_ranking_target_rows_jsonl(&history_artifact.rows)?;
    let summary = structural_path_ranking_target_export_summary(
        StructuralPathRankingTargetExportSummaryInput {
            state_dir,
            symbol,
            artifact: &current_artifact,
            csv_name: &csv_name,
            jsonl_name: &jsonl_name,
            history_csv_name: &history_csv_name,
            history_jsonl_name: &history_jsonl_name,
            history_rows: &history_artifact.rows,
            summary_name: &summary_name,
        },
    );
    let summary_json = serde_json::to_string_pretty(&summary)?;
    let csv = render_structural_path_ranking_target_csv(&current_artifact);
    let jsonl = render_structural_path_ranking_target_jsonl(&current_artifact)?;
    save_text_state(state_dir, symbol, &csv_name, &csv)?;
    save_text_state(state_dir, symbol, &jsonl_name, &jsonl)?;
    save_text_state(state_dir, symbol, &history_csv_name, &history_csv)?;
    save_text_state(state_dir, symbol, &history_jsonl_name, &history_jsonl)?;
    save_text_state(state_dir, symbol, &summary_name, &summary_json)?;
    Ok(summary)
}

pub fn apply_structural_path_ranking_external_scores(
    state_dir: &str,
    symbol: &str,
    scores: &[StructuralPathRankingExternalScoreInput],
) -> Result<StructuralPathRankingTargetExportSummary> {
    let summary_path = Path::new(state_dir)
        .join(symbol)
        .join(STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR)
        .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE);
    let raw = fs::read_to_string(&summary_path)?;
    let summary: StructuralPathRankingTargetExportSummary = serde_json::from_str(&raw)?;
    let mut current_rows =
        load_structural_path_ranking_target_rows(Path::new(&summary.jsonl_path))?;
    let history_jsonl_path = if !summary.history_jsonl_path.is_empty() {
        Path::new(&summary.history_jsonl_path).to_path_buf()
    } else {
        Path::new(&summary.jsonl_path).to_path_buf()
    };
    let mut history_rows = load_structural_path_ranking_target_rows(&history_jsonl_path)?;
    let score_map = scores
        .iter()
        .map(|item| (format!("{}|{}", item.candidate_set_id, item.path_id), item))
        .collect::<BTreeMap<_, _>>();
    let mut matched = 0usize;
    for row in &mut current_rows {
        if let Some(score) = score_map.get(&structural_path_ranking_target_row_score_key(row)) {
            row.raw_path_score = Some(score.raw_path_score.clamp(0.0, 1.0));
            row.score_model_family = score.score_model_family.clone();
            row.score_source_kind = score.score_source_kind.clone();
            row.score_model_artifact_uri = score.score_model_artifact_uri.clone();
            row.score_generator = score.score_generator.clone();
            clear_structural_path_ranking_target_row_outputs(row);
            matched += 1;
        }
    }
    for row in &mut history_rows {
        if let Some(score) = score_map.get(&structural_path_ranking_target_row_score_key(row)) {
            row.raw_path_score = Some(score.raw_path_score.clamp(0.0, 1.0));
            row.score_model_family = score.score_model_family.clone();
            row.score_source_kind = score.score_source_kind.clone();
            row.score_model_artifact_uri = score.score_model_artifact_uri.clone();
            row.score_generator = score.score_generator.clone();
            clear_structural_path_ranking_target_row_outputs(row);
        }
    }
    if matched == 0 {
        anyhow::bail!("no structural path ranking target rows matched the supplied scores");
    }
    let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut history_artifact = StructuralPathRankingTargetArtifact {
        protocol_version: "structural-path-ranking-target-v1".to_string(),
        symbol: symbol.to_string(),
        candidate_set_id: summary.candidate_set_id.clone(),
        candidate_set_size: summary.candidate_set_size,
        generated_at: generated_at.clone(),
        rows: history_rows,
    };
    let history_report = apply_structural_path_probability_calibration(&mut history_artifact);
    let mut current_artifact = StructuralPathRankingTargetArtifact {
        protocol_version: "structural-path-ranking-target-v1".to_string(),
        symbol: symbol.to_string(),
        candidate_set_id: summary.candidate_set_id.clone(),
        candidate_set_size: summary.candidate_set_size,
        generated_at: generated_at.clone(),
        rows: current_rows,
    };
    apply_structural_path_probability_bins(&mut current_artifact.rows, &history_report.bins);
    apply_structural_path_ranking_execution_gates(&mut current_artifact);
    let csv_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_CSV_FILE}"
    );
    let jsonl_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_JSONL_FILE}"
    );
    let history_csv_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_HISTORY_CSV_FILE}"
    );
    let history_jsonl_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_HISTORY_JSONL_FILE}"
    );
    let summary_name = format!(
        "{STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR}/{STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE}"
    );
    let current_csv = render_structural_path_ranking_target_csv(&current_artifact);
    let current_jsonl = render_structural_path_ranking_target_jsonl(&current_artifact)?;
    let history_csv = render_structural_path_ranking_target_rows_csv(
        &current_artifact.protocol_version,
        &current_artifact.symbol,
        &current_artifact.generated_at,
        &history_artifact.rows,
    );
    let history_jsonl = render_structural_path_ranking_target_rows_jsonl(&history_artifact.rows)?;
    let updated_summary = structural_path_ranking_target_export_summary(
        StructuralPathRankingTargetExportSummaryInput {
            state_dir,
            symbol,
            artifact: &current_artifact,
            csv_name: &csv_name,
            jsonl_name: &jsonl_name,
            history_csv_name: &history_csv_name,
            history_jsonl_name: &history_jsonl_name,
            history_rows: &history_artifact.rows,
            summary_name: &summary_name,
        },
    );
    let summary_json = serde_json::to_string_pretty(&updated_summary)?;
    save_text_state(state_dir, symbol, &csv_name, &current_csv)?;
    save_text_state(state_dir, symbol, &jsonl_name, &current_jsonl)?;
    save_text_state(state_dir, symbol, &history_csv_name, &history_csv)?;
    save_text_state(state_dir, symbol, &history_jsonl_name, &history_jsonl)?;
    save_text_state(state_dir, symbol, &summary_name, &summary_json)?;
    Ok(updated_summary)
}

fn structural_candidate_policy_denominator(candidate_paths: &[StructuralPathArtifact]) -> f64 {
    candidate_paths
        .iter()
        .map(|path| path.composite_preference_score.max(0.0))
        .sum()
}

fn structural_candidate_policy_probability(
    composite_score: f64,
    denominator: f64,
    candidate_count: usize,
) -> f64 {
    if denominator > f64::EPSILON {
        (composite_score.max(0.0) / denominator).clamp(0.0, 1.0)
    } else if candidate_count > 0 {
        1.0 / candidate_count as f64
    } else {
        0.0
    }
}

fn structural_candidate_set_id(symbol: &str, candidate_paths: &[StructuralPathArtifact]) -> String {
    let mut fingerprint = String::new();
    fingerprint.push_str(symbol);
    let mut path_ids = candidate_paths
        .iter()
        .map(|path| path.path_id.as_str())
        .collect::<Vec<_>>();
    path_ids.sort_unstable();
    for path_id in path_ids {
        fingerprint.push('|');
        fingerprint.push_str(path_id);
    }
    format!(
        "structural-candidates:{symbol}:{:016x}",
        structural_stable_hash64(&fingerprint)
    )
}

fn structural_stable_hash64(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn resolve_structural_path_ranker_runtime(
    state_dir: Option<&str>,
    symbol: &str,
    candidate_set_id: &str,
    current_candidate_rows: &[StructuralPathRankingTargetRow],
    candidate_paths: &mut [StructuralPathArtifact],
) -> Option<StructuralPathRankerRuntimeSurface> {
    let state_dir = state_dir?;
    let selection = load_structural_path_ranking_runtime_selection(state_dir, symbol)?;
    let reuse_mode = selection.reuse_mode.clone();
    if !selection.enabled {
        return Some(StructuralPathRankerRuntimeSurface {
            enabled: false,
            status: "disabled".to_string(),
            reuse_mode: Some(reuse_mode),
            ..StructuralPathRankerRuntimeSurface::default()
        });
    }
    let summary_path = Path::new(state_dir)
        .join(symbol)
        .join(STRUCTURAL_PATH_RANKING_TARGET_EXPORT_DIR)
        .join(STRUCTURAL_PATH_RANKING_TARGET_SUMMARY_FILE);
    let Ok(raw_summary) = fs::read_to_string(&summary_path) else {
        return Some(StructuralPathRankerRuntimeSurface {
            enabled: true,
            status: "enabled_export_missing".to_string(),
            reuse_mode: Some(reuse_mode),
            ..StructuralPathRankerRuntimeSurface::default()
        });
    };
    let Ok(summary) =
        serde_json::from_str::<StructuralPathRankingTargetExportSummary>(&raw_summary)
    else {
        return Some(StructuralPathRankerRuntimeSurface {
            enabled: true,
            status: "enabled_export_invalid".to_string(),
            reuse_mode: Some(reuse_mode),
            ..StructuralPathRankerRuntimeSurface::default()
        });
    };
    let current_rows = load_structural_path_ranking_target_rows(Path::new(&summary.jsonl_path))
        .unwrap_or_default();
    let history_path = if summary.history_jsonl_path.trim().is_empty() {
        Path::new(&summary.jsonl_path).to_path_buf()
    } else {
        Path::new(&summary.history_jsonl_path).to_path_buf()
    };
    let history_rows = load_structural_path_ranking_target_rows(&history_path).unwrap_or_default();
    let artifact_metadata =
        load_structural_path_ranker_runtime_artifact_metadata(state_dir, symbol);
    let direct_model_rows = artifact_metadata
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
                current_candidate_rows,
            )
            .ok()
        })
        .unwrap_or_default();
    let using_direct_model = !direct_model_rows.is_empty();
    let explicit_rows = artifact_metadata
        .as_ref()
        .and_then(|artifact| {
            if !structural_path_ranker_supports_explicit_family(&artifact.model_family) {
                return None;
            }
            score_structural_path_ranker_runtime_rows_with_explicit_family(
                state_dir,
                symbol,
                &artifact.model_family,
                current_candidate_rows,
            )
            .ok()
        })
        .unwrap_or_default();
    let using_explicit = !explicit_rows.is_empty();
    let service_rows = artifact_metadata
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
                current_candidate_rows,
            )
            .ok()
        })
        .unwrap_or_default();
    let using_service = !service_rows.is_empty();
    let service_declared = artifact_metadata.as_ref().is_some_and(|artifact| {
        structural_path_ranker_supports_service_family(&artifact.model_family)
    });
    let artifact_rows = if using_direct_model {
        direct_model_rows
    } else if using_explicit {
        explicit_rows
    } else if using_service {
        service_rows
    } else if service_declared {
        Vec::new()
    } else {
        artifact_metadata
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
    let using_static_registered_artifact = !using_direct_model
        && !using_explicit
        && !using_service
        && !service_declared
        && artifact_metadata.is_some();

    let artifact_exact_matches = artifact_rows
        .iter()
        .filter(|row| {
            row.candidate_set_id == candidate_set_id
                && row.raw_path_score.is_some()
                && candidate_paths
                    .iter()
                    .any(|path| path.path_id == row.path_id)
        })
        .cloned()
        .map(|row| (row.path_id.clone(), row))
        .collect::<BTreeMap<_, _>>();
    let artifact_path_matches = if using_static_registered_artifact {
        artifact_rows
            .iter()
            .filter(|row| {
                row.raw_path_score.is_some()
                    && candidate_paths
                        .iter()
                        .any(|path| path.path_id == row.path_id)
            })
            .cloned()
            .map(|row| (row.path_id.clone(), row))
            .collect::<BTreeMap<_, _>>()
    } else {
        BTreeMap::new()
    };
    let mut artifact_history_matches = BTreeMap::<String, StructuralPathRankerRuntimeRow>::new();
    for row in &artifact_rows {
        if row.raw_path_score.is_none() {
            continue;
        }
        if candidate_paths
            .iter()
            .any(|path| path.path_id == row.path_id)
        {
            artifact_history_matches.insert(row.path_id.clone(), row.clone());
        }
    }

    let exact_matches = history_rows
        .iter()
        .chain(current_rows.iter())
        .filter(|row| {
            row.candidate_set_id == candidate_set_id
                && row.raw_path_score.is_some()
                && candidate_paths
                    .iter()
                    .any(|path| path.path_id == row.path_id)
        })
        .map(|row| {
            (
                row.path_id.clone(),
                StructuralPathRankerRuntimeRow {
                    candidate_set_id: row.candidate_set_id.clone(),
                    path_id: row.path_id.clone(),
                    raw_path_score: row.raw_path_score,
                    calibrated_path_prob: row.calibrated_path_prob,
                    path_prob_lower_bound: row.path_prob_lower_bound,
                    execution_gate_status: row.execution_gate_status.clone(),
                    score_model_family: row.score_model_family.clone(),
                    score_source_kind: row.score_source_kind.clone(),
                    score_model_artifact_uri: row.score_model_artifact_uri.clone(),
                    score_generator: row.score_generator.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut latest_history_matches = BTreeMap::<String, StructuralPathRankerRuntimeRow>::new();
    for row in history_rows.iter().chain(current_rows.iter()) {
        if row.raw_path_score.is_none() {
            continue;
        }
        if candidate_paths
            .iter()
            .any(|path| path.path_id == row.path_id)
        {
            latest_history_matches.insert(
                row.path_id.clone(),
                StructuralPathRankerRuntimeRow {
                    candidate_set_id: row.candidate_set_id.clone(),
                    path_id: row.path_id.clone(),
                    raw_path_score: row.raw_path_score,
                    calibrated_path_prob: row.calibrated_path_prob,
                    path_prob_lower_bound: row.path_prob_lower_bound,
                    execution_gate_status: row.execution_gate_status.clone(),
                    score_model_family: row.score_model_family.clone(),
                    score_source_kind: row.score_source_kind.clone(),
                    score_model_artifact_uri: row.score_model_artifact_uri.clone(),
                    score_generator: row.score_generator.clone(),
                },
            );
        }
    }

    let mut applied_path_count = 0usize;
    let mut artifact_match_count = 0usize;
    let mut history_match_count = 0usize;
    let mut candidate_set_match_count = 0usize;
    let mut score_model_families = BTreeSet::<String>::new();
    let mut score_source_kinds = BTreeSet::<String>::new();
    let mut score_model_artifact_uris = BTreeSet::<String>::new();
    let mut score_generators = BTreeSet::<String>::new();
    for path in candidate_paths {
        let artifact_history_match = if !using_direct_model
            && !using_service
            && reuse_mode == STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY
        {
            artifact_history_matches.get(&path.path_id).cloned()
        } else {
            None
        };
        let matched = if let Some(row) = artifact_exact_matches.get(&path.path_id) {
            artifact_match_count += 1;
            Some(StructuralPathRankerRuntimeRowMatch {
                source: if using_direct_model {
                    "registered_model_artifact"
                } else if using_explicit {
                    "registered_explicit_artifact"
                } else if using_service {
                    "registered_service"
                } else {
                    "registered_artifact"
                },
                row: row.clone(),
            })
        } else if let Some(row) = artifact_history_match {
            artifact_match_count += 1;
            Some(StructuralPathRankerRuntimeRowMatch {
                source: "registered_artifact_history",
                row,
            })
        } else if let Some(row) = artifact_path_matches.get(&path.path_id) {
            artifact_match_count += 1;
            Some(StructuralPathRankerRuntimeRowMatch {
                source: "registered_artifact_path",
                row: row.clone(),
            })
        } else if let Some(row) = exact_matches.get(&path.path_id) {
            candidate_set_match_count += 1;
            Some(StructuralPathRankerRuntimeRowMatch {
                source: "candidate_set",
                row: row.clone(),
            })
        } else if reuse_mode == STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY {
            latest_history_matches
                .get(&path.path_id)
                .cloned()
                .map(|row| {
                    history_match_count += 1;
                    StructuralPathRankerRuntimeRowMatch {
                        source: "history_path",
                        row,
                    }
                })
        } else {
            None
        };

        let Some(matched) = matched else {
            continue;
        };
        let Some(raw_score) = matched.row.raw_path_score else {
            continue;
        };
        let external_signal = matched
            .row
            .path_prob_lower_bound
            .or(matched.row.calibrated_path_prob)
            .or(matched.row.raw_path_score)
            .unwrap_or(raw_score)
            .clamp(0.0, 1.0);
        let blend_weight = if matched.source.starts_with("registered_artifact")
            || matched.source == "registered_explicit_artifact"
        {
            if matched.row.path_prob_lower_bound.is_some() {
                0.45
            } else if matched.row.calibrated_path_prob.is_some() {
                0.35
            } else {
                0.25
            }
        } else if matched.source == "candidate_set" {
            if matched.row.path_prob_lower_bound.is_some() {
                0.35
            } else if matched.row.calibrated_path_prob.is_some() {
                0.25
            } else {
                0.15
            }
        } else if matched.row.path_prob_lower_bound.is_some() {
            0.20
        } else if matched.row.calibrated_path_prob.is_some() {
            0.15
        } else {
            0.10
        };
        let blended_score = ((1.0 - blend_weight) * path.composite_preference_score
            + blend_weight * external_signal)
            .clamp(0.0, 1.0);
        path.catboost_score = Some(raw_score.clamp(0.0, 1.0));
        path.path_ranker_calibrated_path_prob = matched.row.calibrated_path_prob;
        path.path_ranker_path_prob_lower_bound = matched.row.path_prob_lower_bound;
        path.path_ranker_execution_gate_status = matched.row.execution_gate_status.clone();
        path.path_ranker_runtime_source = Some(matched.source.to_string());
        if let Some(value) = matched
            .row
            .score_model_family
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            score_model_families.insert(value.to_string());
        }
        if let Some(value) = matched
            .row
            .score_source_kind
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            score_source_kinds.insert(value.to_string());
        }
        if let Some(value) = matched
            .row
            .score_model_artifact_uri
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            score_model_artifact_uris.insert(value.to_string());
        }
        if let Some(value) = matched
            .row
            .score_generator
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            score_generators.insert(value.to_string());
        }
        path.composite_preference_score =
            if matched.row.execution_gate_status.as_deref() == Some("observe") {
                blended_score.min(path.composite_preference_score)
            } else {
                blended_score
            };
        applied_path_count += 1;
    }

    let single_or_mixed = |values: BTreeSet<String>| -> Option<String> {
        if values.is_empty() {
            None
        } else if values.len() == 1 {
            values.into_iter().next()
        } else {
            Some("mixed".to_string())
        }
    };

    Some(StructuralPathRankerRuntimeSurface {
        enabled: true,
        status: if using_direct_model && artifact_match_count > 0 {
            "using_registered_model_artifact".to_string()
        } else if using_explicit && artifact_match_count > 0 {
            "using_registered_explicit_artifact".to_string()
        } else if using_service && artifact_match_count > 0 {
            "using_registered_service_scores".to_string()
        } else if artifact_match_count > 0 {
            "using_registered_artifact_scores".to_string()
        } else if candidate_set_match_count > 0 {
            "using_candidate_set_scores".to_string()
        } else if history_match_count > 0 {
            "using_history_scores".to_string()
        } else {
            "enabled_no_matching_scores".to_string()
        },
        reuse_mode: Some(reuse_mode),
        artifact_match_count,
        candidate_set_match_count,
        history_match_count,
        applied_path_count,
        score_model_family: single_or_mixed(score_model_families),
        score_source_kind: single_or_mixed(score_source_kinds),
        score_model_artifact_uri: single_or_mixed(score_model_artifact_uris),
        score_generator: single_or_mixed(score_generators),
    })
}

fn structural_path_ranking_regime_bucket(snapshot: &WorkflowSnapshot) -> String {
    let symbol = structural_symbol(snapshot);
    let regime = structural_active_regime(snapshot).unwrap_or_else(|| "unknown".to_string());
    format!("{symbol}:{regime}")
}

fn structural_path_ranking_pending_reward_state(
    path_id: &str,
    feedback_history: &[FeedbackRecord],
) -> String {
    let Some(record) = feedback_history
        .iter()
        .filter(|record| {
            record
                .structural_feedback
                .as_ref()
                .map(|refs| refs.path_id == path_id)
                .unwrap_or(false)
        })
        .max_by_key(|record| record.timestamp)
    else {
        return "unobserved".to_string();
    };
    if structural_feedback_outcome_is_unresolved(&record.realized_outcome) {
        return "pending".to_string();
    }
    let followed = record
        .structural_feedback
        .as_ref()
        .map(|refs| refs.followed_path)
        .unwrap_or(true);
    if !followed
        || record
            .realized_outcome
            .trim()
            .eq_ignore_ascii_case("not_followed")
    {
        return "not_followed".to_string();
    }
    if record
        .realized_outcome
        .trim()
        .eq_ignore_ascii_case("invalidated")
    {
        return "matured_invalidated".to_string();
    }
    match structural_feedback_learning_outcome(record) {
        Some(StructuralFeedbackLearningOutcome::Positive) => "matured_success".to_string(),
        Some(StructuralFeedbackLearningOutcome::Neutral)
        | Some(StructuralFeedbackLearningOutcome::Negative) => "matured_failure".to_string(),
        None => "unobserved".to_string(),
    }
}

pub fn build_structural_recommended_path_bundle_artifact(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
) -> Option<StructuralRecommendedPathBundleArtifact> {
    let candidate_paths =
        structural_ranked_paths(snapshot, provider_status_agent, feedback_history)
            .into_iter()
            .take(3)
            .collect::<Vec<_>>();
    let symbol = structural_symbol(snapshot);
    let candidate_set_id = structural_candidate_set_id(&symbol, &candidate_paths);
    structural_recommended_path_bundle_from_candidates(
        symbol,
        candidate_set_id,
        None,
        candidate_paths,
    )
}

fn structural_recommended_path_bundle_from_candidates(
    symbol: String,
    candidate_set_id: String,
    path_ranker_runtime: Option<StructuralPathRankerRuntimeSurface>,
    candidate_paths: Vec<StructuralPathArtifact>,
) -> Option<StructuralRecommendedPathBundleArtifact> {
    let denominator = structural_candidate_policy_denominator(&candidate_paths);
    let candidate_set_size = candidate_paths.len();
    let path = candidate_paths
        .iter()
        .filter(|path| path.path_ranker_execution_gate_status.as_deref() == Some("pass"))
        .max_by(|left, right| {
            left.path_ranker_path_prob_lower_bound
                .or(left.path_ranker_calibrated_path_prob)
                .or(left.catboost_score)
                .unwrap_or(left.composite_preference_score)
                .total_cmp(
                    &right
                        .path_ranker_path_prob_lower_bound
                        .or(right.path_ranker_calibrated_path_prob)
                        .or(right.catboost_score)
                        .unwrap_or(right.composite_preference_score),
                )
        })
        .or_else(|| candidate_paths.first())?;
    let selected_path_probability = structural_candidate_policy_probability(
        path.composite_preference_score,
        denominator,
        candidate_set_size,
    );
    let why_this_path = structural_why_this_path_summary(path);
    Some(StructuralRecommendedPathBundleArtifact {
        symbol,
        rank: 1,
        candidate_set_id,
        candidate_set_size,
        path_ranker_runtime,
        selected_path_probability,
        path_id: path.path_id.clone(),
        scenario_id: path.scenario_id.clone(),
        path_label: path.path_label.clone(),
        direction: path.direction.clone(),
        experience_prior: path.path_prior,
        current_posterior: path.path_posterior,
        composite_score: path.composite_preference_score,
        historical_total_records: path.historical_total_records,
        historical_invalidation_rate: path.historical_invalidation_rate,
        path_ranker_raw_score: path.catboost_score,
        path_ranker_calibrated_path_prob: path.path_ranker_calibrated_path_prob,
        path_ranker_path_prob_lower_bound: path.path_ranker_path_prob_lower_bound,
        path_ranker_execution_gate_status: path.path_ranker_execution_gate_status.clone(),
        path_ranker_runtime_source: path.path_ranker_runtime_source.clone(),
        why_this_path,
        trigger_summary: structural_short_rule_summary(
            &path.trigger_conditions,
            "trigger_not_available",
        ),
        confirmation_summary: structural_short_rule_summary(
            &path.confirmation_conditions,
            "confirmation_not_available",
        ),
        stop_summary: structural_scalar_rule_summary(&path.stop_definition, "stop_not_available"),
        invalidation_summary: structural_short_rule_summary(
            &path.invalidation_conditions,
            "invalidation_not_available",
        ),
        recommended_command: path.recommended_command.clone(),
    })
}

pub fn build_structural_recommended_path_bundle_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> Option<StructuralRecommendedPathBundleArtifact> {
    build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        StructuralPathRankerRuntimeContext::default(),
    )
}

pub fn build_structural_recommended_path_bundle_artifact_with_state_dir_and_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    state_dir: Option<&str>,
) -> Option<StructuralRecommendedPathBundleArtifact> {
    build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        StructuralPathRankerRuntimeContext { state_dir },
    )
}

pub(crate) fn build_structural_recommended_path_bundle_artifact_with_runtime_context_and_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    runtime_context: StructuralPathRankerRuntimeContext<'_>,
) -> Option<StructuralRecommendedPathBundleArtifact> {
    let selection = structural_ranked_paths_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        runtime_context,
    );
    let candidate_paths = selection.paths.into_iter().take(3).collect::<Vec<_>>();
    let symbol = structural_symbol(snapshot);
    let candidate_set_id = if selection.candidate_set_id.is_empty() {
        structural_candidate_set_id(&symbol, &candidate_paths)
    } else {
        selection.candidate_set_id
    };
    structural_recommended_path_bundle_from_candidates(
        symbol,
        candidate_set_id,
        selection.runtime,
        candidate_paths,
    )
}

pub fn build_structural_node_artifact(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> StructuralNodeArtifact {
    build_structural_node_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        &StructuralPriorLearningState::default(),
    )
}

pub fn build_structural_node_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralNodeArtifact {
    let symbol = structural_symbol(snapshot);
    let command = top_level_command(snapshot);
    let support_reason = structural_support_reason(snapshot);
    let provider_support =
        build_workflow_provider_support(provider_status_agent, &command, support_reason.as_deref());
    let supporting_evidence = structural_supporting_evidence(snapshot, &provider_support);
    let active_regime = structural_active_regime(snapshot);
    let node_family = if structural_no_workflow_state(snapshot) {
        "bootstrap".to_string()
    } else if active_regime.is_some() {
        "belief_regime_node".to_string()
    } else if provider_support.active {
        "provider_gate".to_string()
    } else if support_reason.as_deref() == Some("user_selected_historical_data_missing") {
        "data_selection_gate".to_string()
    } else if structural_hard_block_active(snapshot) {
        "workflow_gate".to_string()
    } else {
        structural_focus_phase(snapshot)
    };
    let node_label = if structural_no_workflow_state(snapshot) {
        "no_workflow_state".to_string()
    } else if let Some(active_regime) = active_regime.as_ref() {
        active_regime.to_string()
    } else if provider_support.active
        || support_reason.as_deref() == Some("user_selected_historical_data_missing")
        || structural_hard_block_active(snapshot)
    {
        support_reason
            .clone()
            .filter(|value| !value.is_empty() && value != "none")
            .unwrap_or_else(|| "actionable".to_string())
    } else {
        "actionable".to_string()
    };
    let provisional_node_id = format!("{symbol}:{node_family}:{node_label}");
    let node_duration_prior = structural_prior_state
        .node_duration_priors
        .get(&provisional_node_id);
    let node_temporal_state = structural_prior_state
        .node_temporal_posteriors
        .get(&provisional_node_id);
    let posterior_confidence = if node_family == "belief_regime_node" {
        blend_node_posterior_with_duration_prior(
            structural_primary_probability(snapshot),
            node_duration_prior,
            node_temporal_state,
        )
    } else {
        structural_primary_probability(snapshot)
    };
    let belief_prior = structural_resolved_smoothed_prior(
        structural_prior_state.nodes.get(&provisional_node_id),
        structural_prior_state,
        structural_primary_prior(snapshot),
    );
    StructuralNodeArtifact {
        node_id: provisional_node_id,
        node_family,
        node_label,
        focus_phase: structural_focus_phase(snapshot),
        market_context: structural_market_context(snapshot),
        timeframe_scope: structural_timeframe_scope(snapshot),
        supporting_evidence,
        invalidating_evidence: structural_invalidating_evidence(snapshot, &provider_support),
        belief_prior,
        belief_posterior: posterior_confidence,
        posterior_confidence,
        origin_artifacts: structural_origin_artifacts(snapshot),
    }
}

pub fn build_structural_branch_set_artifact(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    node: &StructuralNodeArtifact,
    branch_history: &StructuralBranchHistoryArtifact,
) -> StructuralBranchSetArtifact {
    build_structural_branch_set_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        node,
        branch_history,
        &StructuralPriorLearningState::default(),
    )
}

pub fn build_structural_branch_set_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    node: &StructuralNodeArtifact,
    branch_history: &StructuralBranchHistoryArtifact,
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralBranchSetArtifact {
    let command = top_level_command(snapshot);
    let support_reason = structural_support_reason(snapshot);
    let provider_support =
        build_workflow_provider_support(provider_status_agent, &command, support_reason.as_deref());
    let mut branches = Vec::new();
    if structural_no_workflow_state(snapshot) {
        branches.push(StructuralBranchArtifact {
            branch_id: format!("{}:bootstrap_collect_inputs", node.node_id),
            target_node_id: format!("{}:bootstrap_ready", structural_symbol(snapshot)),
            branch_label: "collect_initial_inputs".to_string(),
            prior_probability: 1.0,
            transition_prior: None,
            transition_weighted_observation_mass: None,
            transition_outcome_support: None,
            transition_temporal_posterior_support: None,
            posterior_probability: 1.0,
            historical_total_records: 0,
            historical_followed_count: 0,
            historical_win_rate: None,
            historical_invalidation_rate: None,
            historical_avg_pnl: None,
            composite_branch_score: 1.0,
            activation_conditions: vec!["No workflow snapshot exists yet.".to_string()],
            failure_conditions: vec![
                "Required market data or state inputs stay missing.".to_string()
            ],
            supporting_evidence: vec!["workflow_status has no persisted phase state".to_string()],
        });
    } else if provider_support.active && structural_active_regime(snapshot).is_none() {
        branches.push(StructuralBranchArtifact {
            branch_id: format!("{}:resolve_provider_gate", node.node_id),
            target_node_id: format!("{}:provider_ready", structural_symbol(snapshot)),
            branch_label: "resolve_provider_prerequisites".to_string(),
            prior_probability: 0.7,
            transition_prior: None,
            transition_weighted_observation_mass: None,
            transition_outcome_support: None,
            transition_temporal_posterior_support: None,
            posterior_probability: 0.7,
            historical_total_records: 0,
            historical_followed_count: 0,
            historical_win_rate: None,
            historical_invalidation_rate: None,
            historical_avg_pnl: None,
            composite_branch_score: 0.7,
            activation_conditions: provider_support.pending_providers.clone(),
            failure_conditions: vec!["User declines provider/runtime setup.".to_string()],
            supporting_evidence: provider_support
                .install_prompts
                .iter()
                .take(2)
                .cloned()
                .collect(),
        });
        branches.push(StructuralBranchArtifact {
            branch_id: format!("{}:defer_external_runtime", node.node_id),
            target_node_id: format!("{}:observe_only", structural_symbol(snapshot)),
            branch_label: "defer_and_observe".to_string(),
            prior_probability: 0.3,
            transition_prior: None,
            transition_weighted_observation_mass: None,
            transition_outcome_support: None,
            transition_temporal_posterior_support: None,
            posterior_probability: 0.3,
            historical_total_records: 0,
            historical_followed_count: 0,
            historical_win_rate: None,
            historical_invalidation_rate: None,
            historical_avg_pnl: None,
            composite_branch_score: 0.3,
            activation_conditions: vec!["Provider runtime is optional for this path.".to_string()],
            failure_conditions: vec!["Execution requires unavailable external runtime.".to_string()],
            supporting_evidence: vec!["zero_config_fallback_may_still_exist".to_string()],
        });
    } else if support_reason.as_deref() == Some("user_selected_historical_data_missing")
        && structural_active_regime(snapshot).is_none()
    {
        branches.push(StructuralBranchArtifact {
            branch_id: format!("{}:choose_historical_dataset", node.node_id),
            target_node_id: format!("{}:research_ready", structural_symbol(snapshot)),
            branch_label: "choose_historical_dataset".to_string(),
            prior_probability: 0.75,
            transition_prior: None,
            transition_weighted_observation_mass: None,
            transition_outcome_support: None,
            transition_temporal_posterior_support: None,
            posterior_probability: 0.75,
            historical_total_records: 0,
            historical_followed_count: 0,
            historical_win_rate: None,
            historical_invalidation_rate: None,
            historical_avg_pnl: None,
            composite_branch_score: 0.75,
            activation_conditions: recommended_next_command_meta(&command).recorded_data_paths,
            failure_conditions: vec!["User does not confirm a valid dataset path.".to_string()],
            supporting_evidence: snapshot.blocking_truth.evidence.clone(),
        });
    } else {
        let regime_probabilities = structural_regime_probabilities(snapshot);
        let latest_feedback = structural_latest_feedback_refs(snapshot);
        let adjusted_posteriors = transition_adjusted_branch_posteriors(
            &node.node_id,
            &regime_probabilities,
            latest_feedback.as_ref().map(|refs| refs.branch_id.as_str()),
            &structural_prior_state.branch_transition_priors,
            &structural_prior_state.branch_temporal_posteriors,
            structural_branch_label_for_regime,
        );
        if !regime_probabilities.is_empty() {
            for (regime, probability) in regime_probabilities {
                let branch_label = structural_branch_label_for_regime(regime.as_str());
                let branch_id = format!("{}:{}", node.node_id, branch_label);
                let historical_summary = branch_history
                    .branches
                    .iter()
                    .find(|branch| branch.branch_id == branch_id);
                let history_adjusted_prior =
                    structural_history_adjusted_branch_prior(probability, historical_summary);
                let prior_stats = structural_prior_state.branches.get(&branch_id);
                let transition_prior = latest_feedback.as_ref().and_then(|refs| {
                    structural_branch_transition_prior(
                        structural_prior_state,
                        &refs.branch_id,
                        &branch_id,
                    )
                });
                let posterior_probability = adjusted_posteriors
                    .get(&branch_id)
                    .copied()
                    .unwrap_or(probability);
                let resolved_prior = structural_resolved_smoothed_prior(
                    prior_stats,
                    structural_prior_state,
                    history_adjusted_prior,
                );
                let blended_prior = blend_branch_prior_with_transition_prior(
                    resolved_prior,
                    transition_prior,
                    latest_feedback.as_ref().and_then(|refs| {
                        structural_prior_state
                            .branch_temporal_posteriors
                            .get(&format!("{}=>{}", refs.branch_id, branch_id))
                    }),
                );
                let branch_temporal_state = latest_feedback.as_ref().and_then(|refs| {
                    structural_prior_state
                        .branch_temporal_posteriors
                        .get(&format!("{}=>{}", refs.branch_id, branch_id))
                });
                branches.push(StructuralBranchArtifact {
                    branch_id,
                    target_node_id: format!("{}:{}:candidate", structural_symbol(snapshot), regime),
                    branch_label: branch_label.to_string(),
                    prior_probability: blended_prior,
                    transition_prior: transition_prior.map(|item| item.transition_prior),
                    transition_weighted_observation_mass: branch_temporal_state
                        .map(|state| state.weighted_observation_mass)
                        .or_else(|| transition_prior.map(|item| item.weighted_observation_mass)),
                    transition_outcome_support: branch_temporal_state
                        .map(|state| state.transition_outcome_support)
                        .or_else(|| transition_prior.map(|item| item.transition_outcome_support)),
                    transition_temporal_posterior_support: branch_temporal_state
                        .map(|state| state.temporal_posterior_support)
                        .or_else(|| transition_prior.map(|item| item.temporal_posterior_support)),
                    posterior_probability,
                    historical_total_records: structural_resolved_observations(
                        prior_stats,
                        historical_summary.map(|summary| summary.total_records).unwrap_or(0),
                    ),
                    historical_followed_count: structural_resolved_followed_count(
                        prior_stats,
                        historical_summary.map(|summary| summary.followed_count).unwrap_or(0),
                    ),
                    historical_win_rate: structural_resolved_branch_win_rate(
                        prior_stats,
                        historical_summary,
                    ),
                    historical_invalidation_rate: structural_resolved_branch_invalidation_rate(
                        prior_stats,
                        historical_summary,
                    ),
                    historical_avg_pnl: structural_resolved_avg_pnl(
                        prior_stats,
                        historical_summary.map(|summary| summary.avg_pnl),
                    ),
                    composite_branch_score: structural_composite_preference_score(
                        posterior_probability,
                        blended_prior,
                    ),
                    activation_conditions: vec![format!("regime_posterior={regime}:{probability:.3}")],
                    failure_conditions: vec![format!(
                        "regime branch {regime} loses posterior support or invalidates before trigger"
                    )],
                    supporting_evidence: structural_regime_supporting_evidence(
                        snapshot,
                        &provider_support,
                        regime.as_str(),
                        probability,
                    ),
                });
            }
        } else {
            branches.push(StructuralBranchArtifact {
                branch_id: format!("{}:execute_recommended_path", node.node_id),
                target_node_id: format!("{}:next_phase", structural_symbol(snapshot)),
                branch_label: "execute_recommended_path".to_string(),
                prior_probability: 0.6,
                transition_prior: None,
                transition_weighted_observation_mass: None,
                transition_outcome_support: None,
                transition_temporal_posterior_support: None,
                posterior_probability: structural_primary_probability(snapshot),
                historical_total_records: 0,
                historical_followed_count: 0,
                historical_win_rate: None,
                historical_invalidation_rate: None,
                historical_avg_pnl: None,
                composite_branch_score: structural_primary_probability(snapshot),
                activation_conditions: vec![command.clone()],
                failure_conditions: vec!["Recommended path invalidates before trigger.".to_string()],
                supporting_evidence: structural_supporting_evidence(snapshot, &provider_support),
            });
            branches.push(StructuralBranchArtifact {
                branch_id: format!("{}:observe_only", node.node_id),
                target_node_id: format!("{}:observe_only", structural_symbol(snapshot)),
                branch_label: "observe_only".to_string(),
                prior_probability: 0.4,
                transition_prior: None,
                transition_weighted_observation_mass: None,
                transition_outcome_support: None,
                transition_temporal_posterior_support: None,
                posterior_probability: (1.0 - structural_primary_probability(snapshot))
                    .clamp(0.0, 1.0),
                historical_total_records: 0,
                historical_followed_count: 0,
                historical_win_rate: None,
                historical_invalidation_rate: None,
                historical_avg_pnl: None,
                composite_branch_score: (1.0 - structural_primary_probability(snapshot))
                    .clamp(0.0, 1.0),
                activation_conditions: vec!["Confidence remains mixed or weak.".to_string()],
                failure_conditions: vec![
                    "Missed high-conviction trigger while observing.".to_string()
                ],
                supporting_evidence: snapshot.risk_flags.iter().take(2).cloned().collect(),
            });
        }
    }
    StructuralBranchSetArtifact {
        from_node_id: node.node_id.clone(),
        branches,
    }
}

pub fn build_structural_scenario_playbook_artifact(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    branches: &StructuralBranchSetArtifact,
    scenario_history: &StructuralScenarioHistoryArtifact,
) -> StructuralScenarioPlaybookArtifact {
    build_structural_scenario_playbook_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        branches,
        scenario_history,
        &StructuralPriorLearningState::default(),
    )
}

pub fn build_structural_scenario_playbook_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    branches: &StructuralBranchSetArtifact,
    scenario_history: &StructuralScenarioHistoryArtifact,
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralScenarioPlaybookArtifact {
    let command = top_level_command(snapshot);
    let support_reason = structural_support_reason(snapshot);
    let provider_support =
        build_workflow_provider_support(provider_status_agent, &command, support_reason.as_deref());
    let scenarios = branches
        .branches
        .iter()
        .map(|branch| {
            let (scenario_label, narrative) = if branch.branch_label == "collect_initial_inputs" {
                (
                    "bootstrap_readiness".to_string(),
                    "Collect the minimum inputs needed to create the first workflow state."
                        .to_string(),
                )
            } else if branch.branch_label == "resolve_provider_prerequisites" {
                (
                    "provider_runtime_enablement".to_string(),
                    format!(
                        "Enable the missing provider/runtime track before attempting the dependent path: {}.",
                        provider_support.pending_providers.join(", ")
                    ),
                )
            } else if branch.branch_label == "choose_historical_dataset" {
                (
                    "historical_dataset_selection".to_string(),
                    "Ask the user to choose the approved historical dataset before research/backtest continues."
                        .to_string(),
                )
            } else if branch.branch_label == "trend_follow_through" {
                (
                    "trend_follow_through".to_string(),
                    "Continuation branch: wait for aligned confirmation, then follow the dominant directional path."
                        .to_string(),
                )
            } else if branch.branch_label == "transition_confirmation" {
                (
                    "transition_confirmation".to_string(),
                    "Transition branch: wait for resolution evidence before committing to the next directional leg."
                        .to_string(),
                )
            } else if branch.branch_label == "range_mean_reversion" {
                (
                    "range_mean_reversion".to_string(),
                    "Range branch: fade extremes only after explicit confirmation and invalidation boundaries are known."
                        .to_string(),
                )
            } else if branch.branch_label == "stress_de_risk" {
                (
                    "stress_de_risk".to_string(),
                    "Stress branch: preserve capital, reduce aggression, and require stronger confirmation."
                        .to_string(),
                )
            } else if branch.branch_label == "observe_only" {
                (
                    "observe_and_wait".to_string(),
                    "Stay flat and wait for cleaner structural confirmation.".to_string(),
                )
            } else {
                (
                    "recommended_execution".to_string(),
                    "Follow the current recommended command path while monitoring invalidation pressure."
                        .to_string(),
                )
            };
            let scenario_id = format!("scenario:{}", branch.branch_id);
            let historical_summary = scenario_history
                .scenarios
                .iter()
                .find(|scenario| scenario.scenario_id == scenario_id);
            let history_adjusted_prior =
                structural_history_adjusted_scenario_prior(
                    branch.posterior_probability,
                    historical_summary,
                );
            let prior_stats = structural_prior_state.scenarios.get(&scenario_id);
            StructuralScenarioArtifact {
                scenario_id: scenario_id.clone(),
                branch_id: branch.branch_id.clone(),
                scenario_label,
                narrative,
                prior_probability: structural_resolved_smoothed_prior(
                    prior_stats,
                    structural_prior_state,
                    history_adjusted_prior,
                ),
                posterior_probability: branch.posterior_probability,
                historical_total_records: structural_resolved_observations(
                    prior_stats,
                    historical_summary.map(|summary| summary.total_records).unwrap_or(0),
                ),
                historical_followed_count: structural_resolved_followed_count(
                    prior_stats,
                    historical_summary.map(|summary| summary.followed_count).unwrap_or(0),
                ),
                historical_win_rate: structural_resolved_scenario_win_rate(
                    prior_stats,
                    historical_summary,
                ),
                historical_invalidation_rate: structural_resolved_scenario_invalidation_rate(
                    prior_stats,
                    historical_summary,
                ),
                historical_avg_pnl: structural_resolved_avg_pnl(
                    prior_stats,
                    historical_summary.map(|summary| summary.avg_pnl),
                ),
                composite_scenario_score: structural_composite_preference_score(
                    branch.posterior_probability,
                    structural_resolved_smoothed_prior(
                        prior_stats,
                        structural_prior_state,
                        history_adjusted_prior,
                    ),
                ),
                required_confirmations: branch.activation_conditions.clone(),
                hard_invalidations: branch.failure_conditions.clone(),
                timing_constraints: vec!["re-evaluate on the next workflow refresh".to_string()],
                path_ids: vec![format!("path:{scenario_id}:primary")],
            }
        })
        .collect::<Vec<_>>();
    StructuralScenarioPlaybookArtifact { scenarios }
}

pub fn build_structural_path_plan_artifact(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    provider_support: &crate::application::provider_catalog::WorkflowProviderSupportSurface,
    scenarios: &StructuralScenarioPlaybookArtifact,
    feedback_history: &[FeedbackRecord],
    path_history: &StructuralPathHistoryArtifact,
) -> StructuralPathPlanArtifact {
    build_structural_path_plan_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        provider_support,
        scenarios,
        feedback_history,
        path_history,
        &StructuralPriorLearningState::default(),
    )
}

pub fn build_structural_path_plan_artifact_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    provider_support: &crate::application::provider_catalog::WorkflowProviderSupportSurface,
    scenarios: &StructuralScenarioPlaybookArtifact,
    feedback_history: &[FeedbackRecord],
    path_history: &StructuralPathHistoryArtifact,
    structural_prior_state: &StructuralPriorLearningState,
) -> StructuralPathPlanArtifact {
    build_structural_path_plan_artifact_with_runtime_context_and_prior_state(
        StructuralPathPlanArtifactInput {
            snapshot,
            provider_status_agent,
            provider_support,
            scenarios,
            feedback_history,
            path_history,
            structural_prior_state,
            runtime_context: StructuralPathRankerRuntimeContext::default(),
        },
    )
}

pub(crate) struct StructuralPathPlanArtifactInput<'a> {
    pub snapshot: &'a WorkflowSnapshot,
    pub provider_status_agent: &'a ProviderCatalogAgentSurface,
    pub provider_support: &'a crate::application::provider_catalog::WorkflowProviderSupportSurface,
    pub scenarios: &'a StructuralScenarioPlaybookArtifact,
    pub feedback_history: &'a [FeedbackRecord],
    pub path_history: &'a StructuralPathHistoryArtifact,
    pub structural_prior_state: &'a StructuralPriorLearningState,
    pub runtime_context: StructuralPathRankerRuntimeContext<'a>,
}

pub(crate) fn build_structural_path_plan_artifact_with_runtime_context_and_prior_state(
    input: StructuralPathPlanArtifactInput<'_>,
) -> StructuralPathPlanArtifact {
    let StructuralPathPlanArtifactInput {
        snapshot,
        provider_status_agent,
        provider_support,
        scenarios,
        feedback_history,
        path_history,
        structural_prior_state,
        runtime_context,
    } = input;
    let command = top_level_command(snapshot);
    let next_meta = recommended_next_command_meta(&command);
    let symbol = structural_symbol(snapshot);
    let mut paths = scenarios
        .scenarios
        .iter()
        .map(|scenario| {
            let path_id = format!("path:{}:primary", scenario.scenario_id);
            let historical_summary = path_history
                .paths
                .iter()
                .find(|path| path.path_id == path_id);
            let selected_entry_quality = structural_selected_entry_quality(snapshot);
            let selected_entry_quality_probability =
                structural_selected_entry_quality_probability(snapshot);
            let pre_bayes_gate_status = structural_pre_bayes_gate_status(snapshot);
            let multi_timeframe_direction_bias =
                structural_multi_timeframe_direction_bias(snapshot);
            let execution_candidate_status = snapshot
                .latest_execution_candidate
                .as_ref()
                .map(|candidate| candidate.candidate_status.clone())
                .filter(|value| !value.trim().is_empty());
            let execution_candidate_artifact_id = snapshot
                .latest_execution_candidate
                .as_ref()
                .map(|candidate| candidate.artifact_id.clone());
            let base_prior = structural_primary_prior(snapshot);
            let history_adjusted_prior =
                structural_history_adjusted_path_prior(base_prior, historical_summary);
            let prior_stats = structural_prior_state.paths.get(&path_id);
            let resolved_prior = structural_resolved_smoothed_prior(
                prior_stats,
                structural_prior_state,
                history_adjusted_prior,
            );
            let composite_preference_score = structural_composite_preference_score(
                structural_posterior_confidence(snapshot),
                resolved_prior,
            );
            StructuralPathArtifact {
                path_id,
                scenario_id: scenario.scenario_id.clone(),
                path_label: scenario.scenario_label.clone(),
                direction: structural_direction(snapshot),
                entry_style: structural_entry_style(snapshot, scenario),
                selected_entry_quality,
                selected_entry_quality_probability,
                pre_bayes_gate_status,
                multi_timeframe_direction_bias,
                execution_candidate_status,
                execution_candidate_artifact_id,
                execution_readiness: snapshot
                    .latest_analyze
                    .as_ref()
                    .and_then(|phase| phase.execution_readiness),
                prediction_edge_share: snapshot
                    .latest_analyze
                    .as_ref()
                    .and_then(|phase| phase.prediction_edge_share),
                execution_edge_share: snapshot
                    .latest_analyze
                    .as_ref()
                    .and_then(|phase| phase.execution_edge_share),
                historical_total_records: structural_resolved_observations(
                    prior_stats,
                    historical_summary
                        .map(|summary| summary.total_records)
                        .unwrap_or(0),
                ),
                historical_followed_count: structural_resolved_followed_count(
                    prior_stats,
                    historical_summary
                        .map(|summary| summary.followed_count)
                        .unwrap_or(0),
                ),
                execution_propensity: structural_prior_execution_propensity(prior_stats),
                historical_win_rate: structural_resolved_path_win_rate(
                    prior_stats,
                    historical_summary,
                ),
                historical_invalidation_rate: structural_resolved_path_invalidation_rate(
                    prior_stats,
                    historical_summary,
                ),
                historical_avg_pnl: structural_resolved_avg_pnl(
                    prior_stats,
                    historical_summary.map(|summary| summary.avg_pnl),
                ),
                trigger_conditions: structural_trigger_conditions(snapshot, scenario),
                confirmation_conditions: structural_confirmation_conditions(
                    snapshot,
                    provider_support,
                    &next_meta,
                ),
                stop_definition: structural_stop_definition(snapshot, provider_support, scenario),
                target_definition: structural_target_definition(snapshot, &command, scenario),
                invalidation_conditions: structural_invalidation_conditions(snapshot, scenario),
                expected_failure_mode: structural_failure_mode(provider_support, scenario),
                max_time_in_trade: "re-evaluate on next structural node update".to_string(),
                path_prior: resolved_prior,
                path_posterior: structural_posterior_confidence(snapshot),
                bbn_support_score: structural_posterior_confidence(snapshot),
                catboost_score: None,
                path_ranker_calibrated_path_prob: None,
                path_ranker_path_prob_lower_bound: None,
                path_ranker_execution_gate_status: None,
                path_ranker_runtime_source: None,
                composite_preference_score,
                recommended_command: next_meta.executable_command.clone().or_else(|| {
                    if command.trim().is_empty() {
                        None
                    } else {
                        Some(command.clone())
                    }
                }),
            }
        })
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        right
            .composite_preference_score
            .total_cmp(&left.composite_preference_score)
            .then_with(|| right.path_posterior.total_cmp(&left.path_posterior))
            .then_with(|| right.path_prior.total_cmp(&left.path_prior))
    });
    let top_candidate_paths = paths.iter().take(3).cloned().collect::<Vec<_>>();
    let candidate_set_id = structural_candidate_set_id(&symbol, &top_candidate_paths);
    let current_candidate_rows = structural_path_ranking_target_artifact_from_candidates(
        snapshot,
        feedback_history,
        structural_prior_state,
        top_candidate_paths,
        Some(candidate_set_id.clone()),
    )
    .rows;
    let runtime = resolve_structural_path_ranker_runtime(
        runtime_context.state_dir,
        &symbol,
        &candidate_set_id,
        &current_candidate_rows,
        &mut paths,
    );
    paths.sort_by(|left, right| {
        right
            .composite_preference_score
            .total_cmp(&left.composite_preference_score)
            .then_with(|| right.path_posterior.total_cmp(&left.path_posterior))
            .then_with(|| right.path_prior.total_cmp(&left.path_prior))
    });
    StructuralPathPlanArtifact {
        required_data_contracts: structural_relevant_profile_data_contracts(
            snapshot,
            provider_status_agent,
        ),
        required_provider_tracks: structural_relevant_profile_track_statuses(
            snapshot,
            provider_status_agent,
        ),
        path_ranker_runtime: runtime,
        paths,
    }
}

fn structural_ranked_paths(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
) -> Vec<StructuralPathArtifact> {
    structural_ranked_paths_with_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        &StructuralPriorLearningState::default(),
    )
}

fn structural_ranked_paths_with_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
) -> Vec<StructuralPathArtifact> {
    structural_ranked_paths_with_runtime_context_and_prior_state(
        snapshot,
        provider_status_agent,
        feedback_history,
        structural_prior_state,
        StructuralPathRankerRuntimeContext::default(),
    )
    .paths
}

fn structural_ranked_paths_with_runtime_context_and_prior_state(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
    feedback_history: &[FeedbackRecord],
    structural_prior_state: &StructuralPriorLearningState,
    runtime_context: StructuralPathRankerRuntimeContext<'_>,
) -> StructuralRankedPathSelection {
    let command = top_level_command(snapshot);
    let support_reason = structural_support_reason(snapshot);
    let provider_support =
        build_workflow_provider_support(provider_status_agent, &command, support_reason.as_deref());
    let node = build_structural_node_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        structural_prior_state,
    );
    let branch_history = build_structural_branch_history_artifact(snapshot, feedback_history);
    let scenario_history = build_structural_scenario_history_artifact(snapshot, feedback_history);
    let path_history = build_structural_path_history_artifact(snapshot, feedback_history);
    let branch_set = build_structural_branch_set_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        &node,
        &branch_history,
        structural_prior_state,
    );
    let scenario_playbook = build_structural_scenario_playbook_artifact_with_prior_state(
        snapshot,
        provider_status_agent,
        &branch_set,
        &scenario_history,
        structural_prior_state,
    );
    let path_plan = build_structural_path_plan_artifact_with_runtime_context_and_prior_state(
        StructuralPathPlanArtifactInput {
            snapshot,
            provider_status_agent,
            provider_support: &provider_support,
            scenarios: &scenario_playbook,
            feedback_history,
            path_history: &path_history,
            structural_prior_state,
            runtime_context,
        },
    );
    let symbol = structural_symbol(snapshot);
    let candidate_set_id = structural_candidate_set_id(
        &symbol,
        &path_plan.paths.iter().take(3).cloned().collect::<Vec<_>>(),
    );
    StructuralRankedPathSelection {
        candidate_set_id,
        runtime: path_plan.path_ranker_runtime,
        paths: path_plan.paths,
    }
}

fn structural_short_rule_summary(items: &[String], fallback: &str) -> String {
    items
        .first()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| fallback.to_string())
}

fn structural_scalar_rule_summary(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn structural_why_this_path_summary(path: &StructuralPathArtifact) -> String {
    let invalidation = path
        .historical_invalidation_rate
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "n/a".to_string());
    format!(
        "posterior={:.3} prior={:.3} invalidation_rate={}",
        path.path_posterior, path.path_prior, invalidation
    )
}

fn structural_branch_transition_prior<'a>(
    structural_prior_state: &'a StructuralPriorLearningState,
    from_branch_id: &str,
    to_branch_id: &str,
) -> Option<&'a crate::state::StructuralBranchTransitionPrior> {
    let key = format!("{from_branch_id}=>{to_branch_id}");
    structural_prior_state.branch_transition_priors.get(&key)
}

fn structural_symbol(snapshot: &WorkflowSnapshot) -> String {
    if snapshot.symbol.trim().is_empty() {
        "UNKNOWN".to_string()
    } else {
        snapshot.symbol.clone()
    }
}

fn structural_latest_feedback_refs(snapshot: &WorkflowSnapshot) -> Option<StructuralFeedbackRefs> {
    [
        snapshot.latest_update.as_ref(),
        snapshot.latest_research.as_ref(),
        snapshot.latest_analyze.as_ref(),
        snapshot.latest_backtest.as_ref(),
        snapshot.latest_train.as_ref(),
    ]
    .into_iter()
    .flatten()
    .find_map(|phase| phase.structural_feedback.clone())
}

fn structural_focus_phase(snapshot: &WorkflowSnapshot) -> String {
    if snapshot.current_focus_phase.trim().is_empty() {
        "workflow_status".to_string()
    } else {
        snapshot.current_focus_phase.clone()
    }
}

fn structural_no_workflow_state(snapshot: &WorkflowSnapshot) -> bool {
    snapshot.latest_update.is_none()
        && snapshot.latest_research.is_none()
        && snapshot.latest_analyze.is_none()
        && snapshot.latest_backtest.is_none()
        && snapshot.latest_train.is_none()
}

fn structural_hard_block_active(snapshot: &WorkflowSnapshot) -> bool {
    matches!(
        snapshot.blocking_truth.status.as_str(),
        "blocked"
            | "bridge_needs_confirmation"
            | "validated_regressing"
            | "credibility_gate_blocked"
    )
}

fn structural_support_reason(snapshot: &WorkflowSnapshot) -> Option<String> {
    if snapshot
        .blocking_truth
        .reason
        .contains("user_selected_historical_data_missing")
    {
        Some("user_selected_historical_data_missing".to_string())
    } else if structural_hard_block_active(snapshot)
        && !snapshot.blocking_truth.reason.trim().is_empty()
    {
        Some(snapshot.blocking_truth.reason.clone())
    } else if snapshot.current_focus_reason.contains("provider")
        || snapshot.current_focus_reason.contains("historical_data")
    {
        Some(snapshot.current_focus_reason.clone())
    } else {
        None
    }
}

fn top_level_command(snapshot: &WorkflowSnapshot) -> String {
    if structural_hard_block_active(snapshot) {
        snapshot.blocking_truth.next_command.clone()
    } else {
        snapshot.recommended_next_command.clone()
    }
}

fn structural_posterior_confidence(snapshot: &WorkflowSnapshot) -> f64 {
    resolved_latest_ensemble_vote(snapshot)
        .as_ref()
        .and_then(|vote| vote.posterior_confidence.or(Some(vote.confidence)))
        .unwrap_or_else(|| {
            if structural_no_workflow_state(snapshot) {
                0.0
            } else {
                0.5
            }
        })
}

fn structural_primary_probability(snapshot: &WorkflowSnapshot) -> f64 {
    if let Some(probability) = structural_regime_probabilities(snapshot)
        .first()
        .map(|(_, probability)| *probability)
    {
        probability
    } else {
        structural_posterior_confidence(snapshot)
    }
}

fn structural_primary_prior(snapshot: &WorkflowSnapshot) -> f64 {
    if let Some(vote) = resolved_latest_ensemble_vote(snapshot).as_ref() {
        if !vote.posterior_probabilities.is_empty() {
            return (1.0 / vote.posterior_probabilities.len() as f64).clamp(0.0, 1.0);
        }
    }
    0.5
}

fn canonical_structural_regime_label(label: &str) -> Option<String> {
    let normalized = label.trim().to_ascii_lowercase();
    let canonical = match normalized.as_str() {
        "trend" | "bull" | "bear" | "trend_impulse" | "trend_decay" => "trend",
        "range" | "range_calm" | "range_choppy" => "range",
        "stress" => "stress",
        "transition" => "transition",
        _ => return None,
    };
    Some(canonical.to_string())
}

fn structural_sorted_regime_probabilities(
    probabilities: std::collections::BTreeMap<String, f64>,
) -> Vec<(String, f64)> {
    let mut out = probabilities
        .into_iter()
        .filter(|(_, probability)| probability.is_finite() && *probability > 0.0)
        .collect::<Vec<_>>();
    out.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    out
}

fn structural_ensemble_regime_probabilities(snapshot: &WorkflowSnapshot) -> Vec<(String, f64)> {
    let mut aggregated = std::collections::BTreeMap::new();
    if let Some(vote) = resolved_latest_ensemble_vote(snapshot).as_ref() {
        for (regime, probability) in &vote.posterior_probabilities {
            if let Some(canonical) = canonical_structural_regime_label(regime) {
                *aggregated.entry(canonical).or_insert(0.0) += *probability;
            }
        }
    }
    structural_sorted_regime_probabilities(aggregated)
}

fn structural_analyze_anchor_regime_probabilities(
    snapshot: &WorkflowSnapshot,
) -> Vec<(String, f64)> {
    let Some(analyze) = snapshot.latest_analyze.as_ref() else {
        return Vec::new();
    };
    canonical_analyze_regime_surface(analyze)
        .map(|(_, probabilities, _)| structural_sorted_regime_probabilities(probabilities))
        .unwrap_or_default()
}

fn structural_active_regime(snapshot: &WorkflowSnapshot) -> Option<String> {
    structural_regime_probabilities(snapshot)
        .first()
        .map(|(regime, _)| regime.clone())
        .or_else(|| {
            resolved_latest_ensemble_vote(snapshot)
                .as_ref()
                .and_then(|vote| canonical_structural_regime_label(&vote.posterior_active_regime))
        })
        .or_else(|| {
            snapshot.latest_analyze.as_ref().and_then(|analyze| {
                analyze
                    .pre_bayes_filtered_assignments
                    .get("market_regime")
                    .and_then(|value| canonical_structural_regime_label(value))
            })
        })
}

fn structural_regime_probabilities(snapshot: &WorkflowSnapshot) -> Vec<(String, f64)> {
    let ensemble = structural_ensemble_regime_probabilities(snapshot);
    if !ensemble.is_empty() {
        return ensemble;
    }

    let analyze = structural_analyze_anchor_regime_probabilities(snapshot);
    if !analyze.is_empty() {
        return analyze;
    }

    resolved_latest_ensemble_vote(snapshot)
        .as_ref()
        .and_then(|vote| canonical_structural_regime_label(&vote.posterior_active_regime))
        .map(|regime| vec![(regime, structural_posterior_confidence(snapshot))])
        .unwrap_or_default()
}

fn structural_branch_label_for_regime(regime: &str) -> &'static str {
    match regime {
        "trend" => "trend_follow_through",
        "transition" => "transition_confirmation",
        "range" => "range_mean_reversion",
        "stress" => "stress_de_risk",
        _ => "execute_recommended_path",
    }
}

fn structural_regime_supporting_evidence(
    snapshot: &WorkflowSnapshot,
    provider_support: &crate::application::provider_catalog::WorkflowProviderSupportSurface,
    regime: &str,
    probability: f64,
) -> Vec<String> {
    let mut evidence = vec![format!("posterior_probability={regime}:{probability:.3}")];
    evidence.extend(structural_supporting_evidence(snapshot, provider_support));
    evidence
}

fn structural_market_context(snapshot: &WorkflowSnapshot) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(vote) = resolved_latest_ensemble_vote(snapshot).as_ref() {
        if !vote.posterior_active_regime.trim().is_empty() {
            out.push(format!(
                "posterior_active_regime={}",
                vote.posterior_active_regime
            ));
        }
        if !vote.posterior_normalization_status.trim().is_empty() {
            out.push(format!(
                "posterior_normalization_status={}",
                vote.posterior_normalization_status
            ));
        }
        for (regime, probability) in structural_regime_probabilities(snapshot) {
            out.push(format!("posterior_probability={regime}:{probability:.3}"));
        }
    }
    out
}

fn structural_selected_entry_quality(snapshot: &WorkflowSnapshot) -> Option<String> {
    snapshot
        .latest_analyze
        .as_ref()
        .and_then(|phase| phase.selected_entry_quality.clone())
        .or_else(|| {
            snapshot
                .latest_execution_candidate
                .as_ref()
                .map(|candidate| candidate.pre_bayes_bridge_selected_entry_quality.clone())
                .filter(|value| !value.trim().is_empty())
        })
}

fn structural_selected_entry_quality_probability(snapshot: &WorkflowSnapshot) -> Option<f64> {
    snapshot
        .latest_analyze
        .as_ref()
        .and_then(|phase| phase.pre_bayes_selected_entry_quality_probability)
        .or_else(|| {
            snapshot
                .latest_pre_bayes_entry_quality_bridge
                .as_ref()
                .and_then(|bridge| {
                    bridge
                        .selected_entry_quality
                        .values()
                        .copied()
                        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                })
        })
}

fn structural_pre_bayes_gate_status(snapshot: &WorkflowSnapshot) -> Option<String> {
    snapshot
        .latest_analyze
        .as_ref()
        .map(|phase| phase.pre_bayes_gate_status.clone())
        .filter(|value| !value.trim().is_empty())
}

fn structural_multi_timeframe_direction_bias(snapshot: &WorkflowSnapshot) -> Option<String> {
    snapshot
        .latest_analyze
        .as_ref()
        .map(|phase| phase.pre_bayes_multi_timeframe_direction_bias.clone())
        .filter(|value| !value.trim().is_empty())
}

fn structural_context_hints(snapshot: &WorkflowSnapshot) -> Vec<String> {
    let command = top_level_command(snapshot).to_ascii_lowercase();
    let focus = structural_focus_phase(snapshot).to_ascii_lowercase();
    let reason = structural_support_reason(snapshot)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut hints = vec![command, focus, reason];
    if structural_no_workflow_state(snapshot) {
        hints.push("bootstrap".to_string());
    }
    hints
}

fn structural_relevant_profile_data_contracts(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> Vec<String> {
    let Some(profile) = provider_status_agent.selected_profile_full.as_ref() else {
        return Vec::new();
    };
    let hints = structural_context_hints(snapshot);
    let mut contracts = profile
        .data_contracts
        .iter()
        .filter(|contract| structural_contract_relevant(contract.category.as_str(), &hints))
        .map(|contract| contract.label.clone())
        .collect::<Vec<_>>();
    contracts.sort();
    contracts.dedup();
    contracts
}

fn structural_relevant_profile_track_statuses(
    snapshot: &WorkflowSnapshot,
    provider_status_agent: &ProviderCatalogAgentSurface,
) -> Vec<String> {
    let Some(profile) = provider_status_agent.selected_profile_full.as_ref() else {
        return Vec::new();
    };
    let hints = structural_context_hints(snapshot);
    let mut statuses = profile
        .track_details
        .iter()
        .filter(|track| structural_track_relevant(track.activation_hints.as_slice(), &hints))
        .map(|track| {
            let target = if !track.pending_provider_ids.is_empty() {
                track.pending_provider_ids.join(",")
            } else if !track.ready_provider_ids.is_empty() {
                track.ready_provider_ids.join(",")
            } else {
                "none".to_string()
            };
            format!("{}:{}:{}", track.track_id, track.status, target)
        })
        .collect::<Vec<_>>();
    statuses.sort();
    statuses.dedup();
    statuses
}

fn structural_contract_relevant(category: &str, hints: &[String]) -> bool {
    let wants_live = hints
        .iter()
        .any(|hint| hint.contains("analyze-live") || hint.contains("live"));
    let wants_research = hints.iter().any(|hint| {
        hint.contains("research")
            || hint.contains("backtest")
            || hint.contains("historical")
            || hint.contains("data_selection")
            || hint.contains("bootstrap")
    });
    let wants_kraken = hints
        .iter()
        .any(|hint| hint.contains("kraken") || hint.contains("crypto"));
    match category {
        "historical" => wants_research,
        "market_context" => wants_research || wants_live,
        "options" => wants_research || wants_live,
        "live" => wants_live,
        "credentials" => wants_kraken,
        _ => true,
    }
}

fn structural_track_relevant(activation_hints: &[String], hints: &[String]) -> bool {
    if activation_hints.is_empty() {
        return true;
    }
    activation_hints.iter().any(|track_hint| {
        let track_hint = track_hint.to_ascii_lowercase();
        hints.iter().any(|hint| hint.contains(track_hint.as_str()))
    })
}

fn structural_timeframe_scope(snapshot: &WorkflowSnapshot) -> Vec<String> {
    snapshot
        .latest_update
        .as_ref()
        .or(snapshot.latest_research.as_ref())
        .or(snapshot.latest_analyze.as_ref())
        .map(|phase| {
            phase
                .multi_timeframe_summary
                .iter()
                .filter_map(|line| line.split(':').next())
                .filter(|part| !part.trim().is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn structural_supporting_evidence(
    snapshot: &WorkflowSnapshot,
    provider_support: &crate::application::provider_catalog::WorkflowProviderSupportSurface,
) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(snapshot.blocking_truth.evidence.iter().take(3).cloned());
    out.extend(snapshot.pending_actions.iter().take(2).cloned());
    if provider_support.active {
        out.extend(
            provider_support
                .pending_providers
                .iter()
                .map(|provider| format!("pending_provider={provider}")),
        );
    }
    if out.is_empty() && structural_no_workflow_state(snapshot) {
        out.push("workflow snapshot not initialized".to_string());
    }
    out
}

fn structural_invalidating_evidence(
    snapshot: &WorkflowSnapshot,
    provider_support: &crate::application::provider_catalog::WorkflowProviderSupportSurface,
) -> Vec<String> {
    let mut out = snapshot
        .risk_flags
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    if provider_support.active {
        out.push("provider runtime still missing".to_string());
    }
    out
}

fn structural_origin_artifacts(snapshot: &WorkflowSnapshot) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(vote) = snapshot.latest_ensemble_vote.as_ref() {
        out.push(format!("ensemble_vote:{}", vote.artifact_id));
    }
    if let Some(artifact) = snapshot.actionable_artifacts.first() {
        out.push(format!(
            "{}:{}",
            artifact.artifact_kind, artifact.artifact_id
        ));
    }
    out
}

fn structural_direction(snapshot: &WorkflowSnapshot) -> String {
    snapshot
        .latest_ensemble_vote
        .as_ref()
        .map(|vote| vote.final_action.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "observe".to_string())
}

fn structural_entry_style(
    snapshot: &WorkflowSnapshot,
    scenario: &StructuralScenarioArtifact,
) -> String {
    if scenario.scenario_label.contains("bootstrap") || scenario.scenario_label.contains("provider")
    {
        "non_trading_precondition".to_string()
    } else if structural_hard_block_active(snapshot) {
        "blocked_until_resolution".to_string()
    } else {
        "conditional_execution".to_string()
    }
}

fn structural_confirmation_conditions(
    snapshot: &WorkflowSnapshot,
    provider_support: &crate::application::provider_catalog::WorkflowProviderSupportSurface,
    next_meta: &crate::state::RecommendedNextCommandMeta,
) -> Vec<String> {
    let mut out = Vec::new();
    if provider_support.active {
        out.push("all required provider tracks are ready".to_string());
    }
    if next_meta.requires_user_input {
        out.push("user confirms the required input".to_string());
    }
    if structural_hard_block_active(snapshot) {
        out.push("hard block is cleared on workflow refresh".to_string());
    }
    if out.is_empty() {
        out.push("recommended command remains valid on next refresh".to_string());
    }
    out
}

fn structural_trigger_conditions(
    snapshot: &WorkflowSnapshot,
    scenario: &StructuralScenarioArtifact,
) -> Vec<String> {
    let mut out = scenario.required_confirmations.clone();
    if let Some(entry_quality) = structural_selected_entry_quality(snapshot) {
        out.push(format!("selected_entry_quality={entry_quality}"));
    }
    if let Some(gate_status) = structural_pre_bayes_gate_status(snapshot) {
        out.push(format!("pre_bayes_gate_status={gate_status}"));
    }
    if let Some(direction_bias) = structural_multi_timeframe_direction_bias(snapshot) {
        out.push(format!("multi_timeframe_direction_bias={direction_bias}"));
    }
    out
}

fn structural_stop_definition(
    snapshot: &WorkflowSnapshot,
    provider_support: &crate::application::provider_catalog::WorkflowProviderSupportSurface,
    scenario: &StructuralScenarioArtifact,
) -> String {
    if provider_support.active || structural_no_workflow_state(snapshot) {
        "No trade until preconditions are satisfied.".to_string()
    } else if let Some(candidate) = snapshot.latest_execution_candidate.as_ref() {
        format!(
            "Use execution candidate '{}' once actionable; current candidate_status={}.",
            candidate.artifact_id, candidate.candidate_status
        )
    } else if scenario.scenario_label.contains("observe") {
        "Stay flat; stop is the next structural invalidation review.".to_string()
    } else {
        "Use the downstream execution path stop once the path is active.".to_string()
    }
}

fn structural_target_definition(
    snapshot: &WorkflowSnapshot,
    command: &str,
    scenario: &StructuralScenarioArtifact,
) -> String {
    if structural_no_workflow_state(snapshot) {
        "Reach the first valid workflow snapshot.".to_string()
    } else if let Some(candidate) = snapshot.latest_execution_candidate.as_ref() {
        format!(
            "Advance to execution candidate '{}' while preserving candidate_status={}.",
            candidate.artifact_id, candidate.candidate_status
        )
    } else if scenario.scenario_label.contains("provider") {
        "Reach provider/runtime readiness for the requested path.".to_string()
    } else if scenario
        .scenario_label
        .contains("historical_dataset_selection")
    {
        "Reach a user-approved research/backtest dataset selection.".to_string()
    } else {
        format!("Execute or review: {}", command.trim())
    }
}

fn structural_invalidation_conditions(
    snapshot: &WorkflowSnapshot,
    scenario: &StructuralScenarioArtifact,
) -> Vec<String> {
    let mut out = scenario.hard_invalidations.clone();
    if let Some(candidate) = snapshot.latest_execution_candidate.as_ref() {
        if !candidate.review_reason.trim().is_empty() {
            out.push(format!(
                "execution_candidate_review_reason={}",
                candidate.review_reason
            ));
        }
        if !candidate.pre_bayes_gate_status.trim().is_empty() {
            out.push(format!(
                "execution_candidate_pre_bayes_gate_status={}",
                candidate.pre_bayes_gate_status
            ));
        }
    }
    out
}

fn structural_failure_mode(
    provider_support: &crate::application::provider_catalog::WorkflowProviderSupportSurface,
    scenario: &StructuralScenarioArtifact,
) -> String {
    if provider_support.active {
        "provider_prerequisite_unresolved".to_string()
    } else if scenario
        .scenario_label
        .contains("historical_dataset_selection")
    {
        "dataset_selection_not_confirmed".to_string()
    } else if scenario.scenario_label.contains("observe") {
        "opportunity_passed_without_confirmation".to_string()
    } else {
        "structural_invalidation_before_path_completion".to_string()
    }
}

pub fn build_structural_feedback_template_artifact(
    snapshot: &WorkflowSnapshot,
    node: &StructuralNodeArtifact,
    branch_set: &StructuralBranchSetArtifact,
    scenario_playbook: &StructuralScenarioPlaybookArtifact,
    path_plan: &StructuralPathPlanArtifact,
) -> StructuralFeedbackTemplateArtifact {
    let selected_branch = branch_set.branches.first();
    let selected_scenario = selected_branch.and_then(|branch| {
        scenario_playbook
            .scenarios
            .iter()
            .find(|scenario| scenario.branch_id == branch.branch_id)
    });
    let selected_path = selected_scenario.and_then(|scenario| {
        path_plan
            .paths
            .iter()
            .find(|path| path.scenario_id == scenario.scenario_id)
    });
    let mut candidate_paths = path_plan.paths.clone();
    candidate_paths.sort_by(|left, right| {
        right
            .composite_preference_score
            .total_cmp(&left.composite_preference_score)
            .then_with(|| right.path_posterior.total_cmp(&left.path_posterior))
            .then_with(|| right.path_prior.total_cmp(&left.path_prior))
    });
    candidate_paths.truncate(3);
    let symbol = structural_symbol(snapshot);
    let candidate_set_id = structural_candidate_set_id(&symbol, &candidate_paths);
    let candidate_set_size = candidate_paths.len();
    let denominator = structural_candidate_policy_denominator(&candidate_paths);
    let selected_path_probability = selected_path
        .map(|path| {
            structural_candidate_policy_probability(
                path.composite_preference_score,
                denominator,
                candidate_set_size,
            )
        })
        .unwrap_or_default();
    let recommended_at = snapshot
        .generated_at
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let recommendation_id = format!(
        "structural-feedback:{}:{}:{}",
        structural_symbol(snapshot),
        node.node_id,
        selected_path
            .map(|path| path.path_id.as_str())
            .unwrap_or("path_unavailable")
    );
    StructuralFeedbackTemplateArtifact {
        protocol_version: "structural-feedback-v1".to_string(),
        recommendation_id,
        recommended_at,
        symbol,
        node_id: node.node_id.clone(),
        branch_id: selected_branch
            .map(|branch| branch.branch_id.clone())
            .unwrap_or_else(|| "branch_unavailable".to_string()),
        scenario_id: selected_scenario
            .map(|scenario| scenario.scenario_id.clone())
            .unwrap_or_else(|| "scenario_unavailable".to_string()),
        path_id: selected_path
            .map(|path| path.path_id.clone())
            .unwrap_or_else(|| "path_unavailable".to_string()),
        candidate_set_id,
        candidate_set_size,
        selected_path_probability,
        direction: selected_path
            .map(|path| path.direction.clone())
            .unwrap_or_else(|| "observe".to_string()),
        entry_style: selected_path
            .map(|path| path.entry_style.clone())
            .unwrap_or_else(|| "non_trading_precondition".to_string()),
        selected_entry_quality: selected_path
            .and_then(|path| path.selected_entry_quality.clone()),
        selected_entry_quality_probability: selected_path
            .and_then(|path| path.selected_entry_quality_probability),
        pre_bayes_gate_status: selected_path.and_then(|path| path.pre_bayes_gate_status.clone()),
        path_posterior: selected_path.map(|path| path.path_posterior),
        bbn_support_score: selected_path.map(|path| path.bbn_support_score),
        allowed_outcomes: vec![
            "win".to_string(),
            "loss".to_string(),
            "breakeven".to_string(),
            "invalidated".to_string(),
            "abandoned".to_string(),
            "not_followed".to_string(),
        ],
        feedback_fields: vec![
            StructuralFeedbackField {
                field_id: "followed_path".to_string(),
                label: "Followed Path".to_string(),
                value_type: "boolean".to_string(),
                required: true,
                description: "Whether the user actually followed the recommended path."
                    .to_string(),
            },
            StructuralFeedbackField {
                field_id: "realized_outcome".to_string(),
                label: "Realized Outcome".to_string(),
                value_type: "enum".to_string(),
                required: true,
                description:
                    "One of win, loss, breakeven, invalidated, abandoned, or not_followed."
                        .to_string(),
            },
            StructuralFeedbackField {
                field_id: "realized_pnl".to_string(),
                label: "Realized PnL".to_string(),
                value_type: "number".to_string(),
                required: false,
                description: "Optional realized PnL from the actual execution.".to_string(),
            },
            StructuralFeedbackField {
                field_id: "exit_reason".to_string(),
                label: "Exit Reason".to_string(),
                value_type: "string".to_string(),
                required: false,
                description:
                    "Freeform reason such as stop_hit, target_hit, invalidated, timed_out."
                        .to_string(),
            },
            StructuralFeedbackField {
                field_id: "notes".to_string(),
                label: "Notes".to_string(),
                value_type: "string".to_string(),
                required: false,
                description: "Optional operator notes about what actually happened.".to_string(),
            },
        ],
        notes: vec![
            "Preserve recommendation_id plus node/branch/scenario/path ids when recording live feedback."
                .to_string(),
            "This is a protocol contract only; canonical persistence wiring comes next."
                .to_string(),
        ],
    }
}

#[derive(Debug, Clone)]
struct StructuralFeedbackHistoryRow {
    node_id: String,
    branch_id: String,
    scenario_id: String,
    path_id: String,
    recommended_at: String,
    followed_path: bool,
    outcome: String,
    pnl: f64,
}

fn structural_feedback_history_rows(
    feedback_history: &[FeedbackRecord],
) -> Vec<StructuralFeedbackHistoryRow> {
    let mut rows = feedback_history
        .iter()
        .filter_map(|record| {
            let refs = record.structural_feedback.as_ref()?;
            Some(StructuralFeedbackHistoryRow {
                node_id: refs.node_id.clone(),
                branch_id: refs.branch_id.clone(),
                scenario_id: refs.scenario_id.clone(),
                path_id: refs.path_id.clone(),
                recommended_at: refs.recommended_at.clone(),
                followed_path: refs.followed_path,
                outcome: record.realized_outcome.clone(),
                pnl: record.pnl,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        a.recommended_at
            .cmp(&b.recommended_at)
            .then_with(|| a.path_id.cmp(&b.path_id))
    });
    rows
}

fn structural_history_row_not_followed(row: &StructuralFeedbackHistoryRow) -> bool {
    !row.followed_path || row.outcome.trim().eq_ignore_ascii_case("not_followed")
}

fn structural_history_execution_propensity(
    followed_count: usize,
    not_followed: usize,
) -> Option<f64> {
    let exposure = followed_count + not_followed;
    (exposure > 0)
        .then(|| ((1.0 + followed_count as f64) / (2.0 + exposure as f64)).clamp(0.0, 1.0))
}

fn structural_history_off_policy_exposure_rate(
    followed_count: usize,
    not_followed: usize,
) -> Option<f64> {
    let exposure = followed_count + not_followed;
    (exposure > 0).then(|| ((1.0 + not_followed as f64) / (2.0 + exposure as f64)).clamp(0.0, 1.0))
}

pub fn build_structural_history_summary_artifact(
    snapshot: &WorkflowSnapshot,
    feedback_history: &[FeedbackRecord],
) -> StructuralHistorySummaryArtifact {
    let rows = structural_feedback_history_rows(feedback_history);
    StructuralHistorySummaryArtifact {
        total_records: rows.len(),
        distinct_nodes: rows
            .iter()
            .map(|row| row.node_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        distinct_branches: rows
            .iter()
            .map(|row| row.branch_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        distinct_scenarios: rows
            .iter()
            .map(|row| row.scenario_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        distinct_paths: rows
            .iter()
            .map(|row| row.path_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        latest_node_id: snapshot
            .latest_update
            .as_ref()
            .and_then(|phase| phase.structural_feedback.as_ref())
            .map(|refs| refs.node_id.clone()),
        latest_branch_id: snapshot
            .latest_update
            .as_ref()
            .and_then(|phase| phase.structural_feedback.as_ref())
            .map(|refs| refs.branch_id.clone()),
        latest_scenario_id: snapshot
            .latest_update
            .as_ref()
            .and_then(|phase| phase.structural_feedback.as_ref())
            .map(|refs| refs.scenario_id.clone()),
        latest_path_id: snapshot
            .latest_update
            .as_ref()
            .and_then(|phase| phase.structural_feedback.as_ref())
            .map(|refs| refs.path_id.clone()),
    }
}

pub fn build_structural_node_history_artifact(
    snapshot: &WorkflowSnapshot,
    feedback_history: &[FeedbackRecord],
) -> StructuralNodeHistoryArtifact {
    let mut summaries = std::collections::BTreeMap::<String, StructuralNodeOutcomeSummary>::new();
    for row in structural_feedback_history_rows(feedback_history) {
        let entry =
            summaries
                .entry(row.node_id.clone())
                .or_insert_with(|| StructuralNodeOutcomeSummary {
                    node_id: row.node_id.clone(),
                    ..StructuralNodeOutcomeSummary::default()
                });
        entry.total_records += 1;
        entry.avg_pnl += row.pnl;
        if row.followed_path {
            entry.followed_count += 1;
        }
        if structural_history_row_not_followed(&row) {
            entry.not_followed += 1;
        }
        match row.outcome.as_str() {
            "win" => entry.wins += 1,
            "loss" => entry.losses += 1,
            "breakeven" => entry.breakevens += 1,
            "invalidated" => entry.invalidated += 1,
            "abandoned" => entry.abandoned += 1,
            "not_followed" => {}
            _ => {}
        }
        entry.last_recommended_at = Some(row.recommended_at);
        entry.last_realized_outcome = Some(row.outcome);
    }
    let mut nodes = summaries.into_values().collect::<Vec<_>>();
    finalize_structural_node_summaries(&mut nodes);
    StructuralNodeHistoryArtifact {
        summary: StructuralEntityHistorySummary {
            total_records: nodes.iter().map(|node| node.total_records).sum(),
            distinct_entities: nodes.len(),
            latest_entity_id: snapshot
                .latest_update
                .as_ref()
                .and_then(|phase| phase.structural_feedback.as_ref())
                .map(|refs| refs.node_id.clone()),
        },
        nodes,
    }
}

pub fn build_structural_branch_history_artifact(
    snapshot: &WorkflowSnapshot,
    feedback_history: &[FeedbackRecord],
) -> StructuralBranchHistoryArtifact {
    let mut summaries =
        std::collections::BTreeMap::<(String, String), StructuralBranchOutcomeSummary>::new();
    for row in structural_feedback_history_rows(feedback_history) {
        let entry = summaries
            .entry((row.node_id.clone(), row.branch_id.clone()))
            .or_insert_with(|| StructuralBranchOutcomeSummary {
                node_id: row.node_id.clone(),
                branch_id: row.branch_id.clone(),
                ..StructuralBranchOutcomeSummary::default()
            });
        entry.total_records += 1;
        entry.avg_pnl += row.pnl;
        if row.followed_path {
            entry.followed_count += 1;
        }
        if structural_history_row_not_followed(&row) {
            entry.not_followed += 1;
        }
        match row.outcome.as_str() {
            "win" => entry.wins += 1,
            "loss" => entry.losses += 1,
            "breakeven" => entry.breakevens += 1,
            "invalidated" => entry.invalidated += 1,
            "abandoned" => entry.abandoned += 1,
            "not_followed" => {}
            _ => {}
        }
        entry.last_recommended_at = Some(row.recommended_at);
        entry.last_realized_outcome = Some(row.outcome);
    }
    let mut branches = summaries.into_values().collect::<Vec<_>>();
    finalize_structural_branch_summaries(&mut branches);
    StructuralBranchHistoryArtifact {
        summary: StructuralEntityHistorySummary {
            total_records: branches.iter().map(|branch| branch.total_records).sum(),
            distinct_entities: branches.len(),
            latest_entity_id: snapshot
                .latest_update
                .as_ref()
                .and_then(|phase| phase.structural_feedback.as_ref())
                .map(|refs| refs.branch_id.clone()),
        },
        branches,
    }
}

pub fn build_structural_scenario_history_artifact(
    snapshot: &WorkflowSnapshot,
    feedback_history: &[FeedbackRecord],
) -> StructuralScenarioHistoryArtifact {
    let mut summaries = std::collections::BTreeMap::<
        (String, String, String),
        StructuralScenarioOutcomeSummary,
    >::new();
    for row in structural_feedback_history_rows(feedback_history) {
        let entry = summaries
            .entry((
                row.node_id.clone(),
                row.branch_id.clone(),
                row.scenario_id.clone(),
            ))
            .or_insert_with(|| StructuralScenarioOutcomeSummary {
                node_id: row.node_id.clone(),
                branch_id: row.branch_id.clone(),
                scenario_id: row.scenario_id.clone(),
                ..StructuralScenarioOutcomeSummary::default()
            });
        entry.total_records += 1;
        entry.avg_pnl += row.pnl;
        if row.followed_path {
            entry.followed_count += 1;
        }
        if structural_history_row_not_followed(&row) {
            entry.not_followed += 1;
        }
        match row.outcome.as_str() {
            "win" => entry.wins += 1,
            "loss" => entry.losses += 1,
            "breakeven" => entry.breakevens += 1,
            "invalidated" => entry.invalidated += 1,
            "abandoned" => entry.abandoned += 1,
            "not_followed" => {}
            _ => {}
        }
        entry.last_recommended_at = Some(row.recommended_at);
        entry.last_realized_outcome = Some(row.outcome);
    }
    let mut scenarios = summaries.into_values().collect::<Vec<_>>();
    finalize_structural_scenario_summaries(&mut scenarios);
    StructuralScenarioHistoryArtifact {
        summary: StructuralEntityHistorySummary {
            total_records: scenarios
                .iter()
                .map(|scenario| scenario.total_records)
                .sum(),
            distinct_entities: scenarios.len(),
            latest_entity_id: snapshot
                .latest_update
                .as_ref()
                .and_then(|phase| phase.structural_feedback.as_ref())
                .map(|refs| refs.scenario_id.clone()),
        },
        scenarios,
    }
}

pub fn build_structural_path_history_artifact(
    snapshot: &WorkflowSnapshot,
    feedback_history: &[FeedbackRecord],
) -> StructuralPathHistoryArtifact {
    let rows = structural_feedback_history_rows(feedback_history);

    let mut summaries = std::collections::BTreeMap::<
        (String, String, String, String),
        StructuralPathOutcomeSummary,
    >::new();
    for row in rows {
        let entry = summaries
            .entry((
                row.node_id.clone(),
                row.branch_id.clone(),
                row.scenario_id.clone(),
                row.path_id.clone(),
            ))
            .or_insert_with(|| StructuralPathOutcomeSummary {
                node_id: row.node_id.clone(),
                branch_id: row.branch_id.clone(),
                scenario_id: row.scenario_id.clone(),
                path_id: row.path_id.clone(),
                ..StructuralPathOutcomeSummary::default()
            });
        entry.total_records += 1;
        entry.avg_pnl += row.pnl;
        if row.followed_path {
            entry.followed_count += 1;
        }
        if structural_history_row_not_followed(&row) {
            entry.not_followed += 1;
        }
        match row.outcome.as_str() {
            "win" => entry.wins += 1,
            "loss" => entry.losses += 1,
            "breakeven" => entry.breakevens += 1,
            "invalidated" => entry.invalidated += 1,
            "abandoned" => entry.abandoned += 1,
            "not_followed" => {}
            _ => {}
        }
        entry.last_recommended_at = Some(row.recommended_at);
        entry.last_realized_outcome = Some(row.outcome);
    }

    let mut paths = summaries.into_values().collect::<Vec<_>>();
    finalize_structural_path_summaries(&mut paths);

    let latest_path_id = snapshot
        .latest_update
        .as_ref()
        .and_then(|phase| phase.structural_feedback.as_ref())
        .map(|refs| refs.path_id.clone());

    StructuralPathHistoryArtifact {
        summary: StructuralPathHistorySummary {
            total_records: paths.iter().map(|path| path.total_records).sum(),
            distinct_paths: paths.len(),
            distinct_branches: paths
                .iter()
                .map(|path| path.branch_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            distinct_scenarios: paths
                .iter()
                .map(|path| path.scenario_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            latest_path_id,
        },
        paths,
    }
}

fn finalize_structural_node_summaries(nodes: &mut [StructuralNodeOutcomeSummary]) {
    for node in nodes.iter_mut() {
        if node.total_records > 0 {
            node.avg_pnl /= node.total_records as f64;
        }
        node.execution_propensity =
            structural_history_execution_propensity(node.followed_count, node.not_followed);
        node.off_policy_exposure_rate =
            structural_history_off_policy_exposure_rate(node.followed_count, node.not_followed);
    }
    nodes.sort_by(|a, b| {
        b.total_records
            .cmp(&a.total_records)
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
}

fn finalize_structural_branch_summaries(branches: &mut [StructuralBranchOutcomeSummary]) {
    for branch in branches.iter_mut() {
        if branch.total_records > 0 {
            branch.avg_pnl /= branch.total_records as f64;
        }
        branch.execution_propensity =
            structural_history_execution_propensity(branch.followed_count, branch.not_followed);
        branch.off_policy_exposure_rate =
            structural_history_off_policy_exposure_rate(branch.followed_count, branch.not_followed);
    }
    branches.sort_by(|a, b| {
        b.total_records
            .cmp(&a.total_records)
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| a.branch_id.cmp(&b.branch_id))
    });
}

fn finalize_structural_scenario_summaries(scenarios: &mut [StructuralScenarioOutcomeSummary]) {
    for scenario in scenarios.iter_mut() {
        if scenario.total_records > 0 {
            scenario.avg_pnl /= scenario.total_records as f64;
        }
        scenario.execution_propensity =
            structural_history_execution_propensity(scenario.followed_count, scenario.not_followed);
        scenario.off_policy_exposure_rate = structural_history_off_policy_exposure_rate(
            scenario.followed_count,
            scenario.not_followed,
        );
    }
    scenarios.sort_by(|a, b| {
        b.total_records
            .cmp(&a.total_records)
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| a.scenario_id.cmp(&b.scenario_id))
    });
}

fn finalize_structural_path_summaries(paths: &mut [StructuralPathOutcomeSummary]) {
    for path in paths.iter_mut() {
        if path.total_records > 0 {
            path.avg_pnl /= path.total_records as f64;
        }
        path.execution_propensity =
            structural_history_execution_propensity(path.followed_count, path.not_followed);
        path.off_policy_exposure_rate =
            structural_history_off_policy_exposure_rate(path.followed_count, path.not_followed);
    }
    paths.sort_by(|a, b| {
        b.total_records
            .cmp(&a.total_records)
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| a.path_id.cmp(&b.path_id))
    });
}

pub fn feedback_record_from_structural_submission(
    submission: StructuralFeedbackSubmission,
    symbol_override: Option<&str>,
    outcome_override: Option<&str>,
    pnl_override: Option<f64>,
    regime_override: Option<Regime>,
    direction_override: Option<Direction>,
) -> FeedbackRecord {
    let selected_direction = direction_override.unwrap_or_else(|| {
        match submission.direction.trim().to_ascii_lowercase().as_str() {
            "bull" | "long" | "execute_follow_through" => Direction::Bull,
            "bear" | "short" | "stress" => Direction::Bear,
            _ => Direction::Neutral,
        }
    });
    let selected_probability = submission
        .selected_path_probability
        .or(submission.path_posterior)
        .or(submission.selected_entry_quality_probability)
        .or(submission.bbn_support_score)
        .map(|probability| probability.clamp(0.0, 1.0))
        .unwrap_or_else(|| {
            match submission
                .selected_entry_quality
                .as_deref()
                .unwrap_or("medium")
                .to_ascii_lowercase()
                .as_str()
            {
                "high" => 0.8,
                "low" => 0.2,
                _ => 0.5,
            }
        });
    let (long_score, short_score, win_prob_long, win_prob_short) = match selected_direction {
        Direction::Bull => (
            selected_probability,
            1.0 - selected_probability,
            selected_probability,
            1.0 - selected_probability,
        ),
        Direction::Bear => (
            1.0 - selected_probability,
            selected_probability,
            1.0 - selected_probability,
            selected_probability,
        ),
        Direction::Neutral => (0.0, 0.0, selected_probability, selected_probability),
    };
    let outcome = outcome_override
        .map(str::to_string)
        .unwrap_or(submission.realized_outcome.clone());
    let pnl = pnl_override
        .or(submission.realized_pnl)
        .unwrap_or_else(|| match outcome.as_str() {
            "win" => 0.01,
            "loss" => -0.01,
            _ => 0.0,
        });
    FeedbackRecord {
        timestamp: chrono::Utc::now(),
        symbol: symbol_override.unwrap_or(&submission.symbol).to_string(),
        source: "structural_feedback_submission".to_string(),
        run_id: Some(submission.recommendation_id.clone()),
        trade_id: None,
        prompt_version: Some(submission.protocol_version.clone()),
        factor_version: None,
        data_fingerprint: None,
        factors_used: Vec::<FeedbackFactorUsage>::new(),
        model_probabilities_before_trade: ModelProbabilitySnapshot {
            selected_direction,
            selected_probability,
            long_score,
            short_score,
            win_prob_long,
            win_prob_short,
            uncertainty: (1.0 - submission.bbn_support_score.unwrap_or(selected_probability))
                .clamp(0.0, 1.0),
        },
        realized_outcome: outcome,
        pnl,
        regime_at_entry: regime_override.unwrap_or(Regime::ManipulationExpansion),
        structural_feedback: Some(StructuralFeedbackRefs {
            protocol_version: submission.protocol_version,
            recommendation_id: submission.recommendation_id,
            recommended_at: submission.recommended_at,
            node_id: submission.node_id,
            branch_id: submission.branch_id,
            scenario_id: submission.scenario_id,
            path_id: submission.path_id,
            followed_path: submission.followed_path,
            exit_reason: submission.exit_reason,
            notes: submission.notes,
        }),
        reflection_mismatch_tags: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        structural_source_reliability_em_fit_from_state, FeedbackRecord, ModelProbabilitySnapshot,
        StructuralFeedbackRefs, STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_MULTI_SOURCE_ITEMS,
    };
    use crate::types::{Direction, Regime};
    use chrono::Utc;

    fn calibration_row(
        path_id: &str,
        raw_path_score: f64,
        pending_reward_state: &str,
    ) -> StructuralPathRankingTargetRow {
        StructuralPathRankingTargetRow {
            rank: 1,
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 3,
            path_id: path_id.to_string(),
            scenario_id: format!("scenario:{path_id}"),
            path_label: path_id.to_string(),
            direction: "bull".to_string(),
            raw_path_score: Some(raw_path_score),
            calibrated_path_prob: None,
            path_prob_lower_bound: None,
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
            calibrated_label: structural_path_ranking_reward_label(pending_reward_state),
            propensity_estimate: Some(0.5),
            ips_weight: Some(2.0),
            training_weight: if structural_path_ranking_reward_label(pending_reward_state).is_some()
            {
                Some(2.0)
            } else {
                None
            },
            regime_calibration_bucket: "NQ:trend".to_string(),
            behavior_policy_probability: 0.33,
            execution_propensity: Some(0.6),
            target_policy_probability_confidence: Some(0.55),
            target_policy_probability_lower_bound: Some(0.30),
            target_policy_reward_prior: Some(0.58),
            target_policy_reward_lower_bound: Some(0.28),
            experience_prior: 0.5,
            current_posterior: 0.7,
            structural_baseline_score: 0.4,
            score_model_family: None,
            score_source_kind: None,
            score_model_artifact_uri: None,
            score_generator: None,
        }
    }

    fn source_em_event(
        source_label: &str,
        recommendation_id: &str,
        realized_outcome: Option<&str>,
    ) -> crate::state::StructuralPriorEvent {
        crate::state::StructuralPriorEvent {
            source_label: source_label.to_string(),
            symbol: "NQ".to_string(),
            recommendation_id: recommendation_id.to_string(),
            recommended_at: "2026-05-02T00:00:00Z".to_string(),
            node_id: "node-em".to_string(),
            branch_id: "branch-em".to_string(),
            scenario_id: "scenario-em".to_string(),
            path_id: format!("path-{recommendation_id}"),
            followed_path: true,
            realized_outcome: realized_outcome.map(str::to_string),
        }
    }

    #[test]
    fn source_reliability_em_readiness_requires_multi_source_overlap() {
        let mut state = StructuralPriorLearningState::default();
        state.event_ledger.extend([
            source_em_event("backtest", "rec-1", Some("win")),
            source_em_event("live", "rec-1", Some("win")),
            source_em_event("backtest", "rec-2", Some("loss")),
            source_em_event("live", "rec-2", Some("invalidated")),
            source_em_event("backtest", "rec-3", Some("breakeven")),
            source_em_event("live", "rec-3", Some("win")),
            source_em_event("backtest", "rec-4", Some("loss")),
            source_em_event("live", "rec-4", Some("pending")),
        ]);
        crate::state::refresh_structural_source_reliability_em_state(&mut state);

        let readiness = structural_source_reliability_em_readiness(&state);

        assert!(readiness.ready);
        assert_eq!(readiness.status, "ready");
        assert_eq!(readiness.candidate_item_count, 4);
        assert_eq!(readiness.labeled_item_count, 4);
        assert_eq!(readiness.multi_source_item_count, 3);
        assert_eq!(readiness.distinct_source_count, 2);
        assert_eq!(readiness.observed_label_count, 7);
        assert_eq!(readiness.max_sources_per_item, 2);
        assert_eq!(
            readiness.min_multi_source_items,
            STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_MULTI_SOURCE_ITEMS
        );
        assert_eq!(readiness.consensus_item_count, 3);
        assert_eq!(readiness.conflict_item_count, 1);
        assert!((readiness.avg_consensus_confidence.unwrap() - (2.5 / 3.0)).abs() < 1e-9);
        assert_eq!(readiness.min_consensus_confidence, Some(0.5));
        assert_eq!(
            readiness.em_iteration_count,
            crate::state::STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS
        );
        assert_eq!(readiness.em_latent_item_count, 3);
        assert_eq!(readiness.em_distinct_label_count, 3);
        assert_eq!(readiness.em_confusion_cell_count, 18);
        let avg_latent_confidence = readiness.avg_em_latent_confidence.unwrap();
        let min_latent_confidence = readiness.min_em_latent_confidence.unwrap();
        let avg_source_reliability = readiness.avg_em_source_reliability.unwrap();
        let min_source_reliability = readiness.min_em_source_reliability.unwrap();
        assert!(avg_latent_confidence >= min_latent_confidence);
        assert!((0.0..=1.0).contains(&avg_latent_confidence));
        assert!((0.0..=1.0).contains(&min_latent_confidence));
        assert!(avg_source_reliability >= min_source_reliability);
        assert!((0.0..=1.0).contains(&avg_source_reliability));
        assert!((0.0..=1.0).contains(&min_source_reliability));
        assert_eq!(readiness.persisted_source_summary_count, 2);
        assert_eq!(readiness.persisted_confusion_cell_count, 18);
        assert!(readiness.avg_persisted_source_reliability.is_some());
        assert!(readiness.min_persisted_source_reliability.is_some());
        assert_eq!(readiness.em_calibration_status.as_deref(), Some("ready"));
        assert_eq!(readiness.em_calibration_observation_count, 6);
        assert_eq!(readiness.em_calibration_source_count, 2);
        assert_eq!(
            readiness.em_calibration_min_observations,
            crate::state::STRUCTURAL_SOURCE_RELIABILITY_EM_MIN_CALIBRATION_OBSERVATIONS
        );
        assert!(readiness.em_calibration_brier_score.unwrap() >= 0.0);
        assert!(readiness.em_calibration_log_loss.unwrap() >= 0.0);
    }

    #[test]
    fn experience_prior_surface_path_includes_delayed_reward_replay_validation() {
        let snapshot =
            crate::application::orchestration::workflow_status::sample_human_workflow_snapshot();
        let mut snapshot = snapshot;
        if let Some(vote) = snapshot.latest_ensemble_vote.as_mut() {
            vote.posterior_active_regime = "trend".to_string();
            vote.posterior_probabilities = BTreeMap::from([("trend".to_string(), 0.8)]);
        }
        let discovered_path_id =
            build_structural_experience_prior_surface_artifact_with_prior_state(
                &snapshot,
                &crate::application::provider_catalog::ProviderCatalogAgentSurface::default(),
                &[],
                &StructuralPriorLearningState::default(),
            )
            .path
            .as_ref()
            .map(|path| path.entity_id.clone())
            .expect("sample path id");
        let feedback_history = vec![
            FeedbackRecord {
                timestamp: chrono::DateTime::parse_from_rfc3339("2026-04-30T00:30:00Z")
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
                realized_outcome: "win".to_string(),
                pnl: 1.0,
                regime_at_entry: Regime::Accumulation,
                structural_feedback: Some(StructuralFeedbackRefs {
                    protocol_version: "structural-feedback-v1".to_string(),
                    recommendation_id: "rec-1".to_string(),
                    recommended_at: "2026-04-30T00:00:00Z".to_string(),
                    node_id: "NQ:belief_regime_node:trend".to_string(),
                    branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                    scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                        .to_string(),
                    path_id: discovered_path_id.clone(),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                }),
                reflection_mismatch_tags: Vec::new(),
            },
            FeedbackRecord {
                timestamp: chrono::DateTime::parse_from_rfc3339("2026-04-30T03:00:00Z")
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
                realized_outcome: "loss".to_string(),
                pnl: -1.0,
                regime_at_entry: Regime::Accumulation,
                structural_feedback: Some(StructuralFeedbackRefs {
                    protocol_version: "structural-feedback-v1".to_string(),
                    recommendation_id: "rec-2".to_string(),
                    recommended_at: "2026-04-30T01:00:00Z".to_string(),
                    node_id: "NQ:belief_regime_node:trend".to_string(),
                    branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                    scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                        .to_string(),
                    path_id: discovered_path_id.clone(),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                }),
                reflection_mismatch_tags: Vec::new(),
            },
            FeedbackRecord {
                timestamp: chrono::DateTime::parse_from_rfc3339("2026-04-30T08:00:00Z")
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
                realized_outcome: "invalidated".to_string(),
                pnl: -0.5,
                regime_at_entry: Regime::Accumulation,
                structural_feedback: Some(StructuralFeedbackRefs {
                    protocol_version: "structural-feedback-v1".to_string(),
                    recommendation_id: "rec-3".to_string(),
                    recommended_at: "2026-04-30T02:00:00Z".to_string(),
                    node_id: "NQ:belief_regime_node:trend".to_string(),
                    branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                    scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                        .to_string(),
                    path_id: discovered_path_id.clone(),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                }),
                reflection_mismatch_tags: Vec::new(),
            },
            FeedbackRecord {
                timestamp: chrono::DateTime::parse_from_rfc3339("2026-04-30T03:45:00Z")
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
                realized_outcome: "win".to_string(),
                pnl: 1.1,
                regime_at_entry: Regime::Accumulation,
                structural_feedback: Some(StructuralFeedbackRefs {
                    protocol_version: "structural-feedback-v1".to_string(),
                    recommendation_id: "rec-4".to_string(),
                    recommended_at: "2026-04-30T03:00:00Z".to_string(),
                    node_id: "NQ:belief_regime_node:trend".to_string(),
                    branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                    scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                        .to_string(),
                    path_id: discovered_path_id.clone(),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                }),
                reflection_mismatch_tags: Vec::new(),
            },
            FeedbackRecord {
                timestamp: chrono::DateTime::parse_from_rfc3339("2026-04-30T10:00:00Z")
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
                realized_outcome: "loss".to_string(),
                pnl: -1.2,
                regime_at_entry: Regime::Accumulation,
                structural_feedback: Some(StructuralFeedbackRefs {
                    protocol_version: "structural-feedback-v1".to_string(),
                    recommendation_id: "rec-5".to_string(),
                    recommended_at: "2026-04-30T04:00:00Z".to_string(),
                    node_id: "NQ:belief_regime_node:trend".to_string(),
                    branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                    scenario_id: "scenario:NQ:belief_regime_node:trend:trend_follow_through"
                        .to_string(),
                    path_id: discovered_path_id.clone(),
                    followed_path: true,
                    exit_reason: None,
                    notes: None,
                }),
                reflection_mismatch_tags: Vec::new(),
            },
        ];
        let surface = build_structural_experience_prior_surface_artifact_with_prior_state(
            &snapshot,
            &crate::application::provider_catalog::ProviderCatalogAgentSurface::default(),
            &feedback_history,
            &StructuralPriorLearningState::default(),
        );
        let replay = surface
            .path
            .as_ref()
            .and_then(|path| path.delayed_reward_replay_validation.as_ref())
            .expect("path replay validation");
        assert_eq!(replay.status, "ready");
        assert!(replay.training_record_count >= 3);
        assert!(replay.evaluation_record_count >= 1);
        assert!(replay.resolution_brier_score.is_some());
    }

    #[test]
    fn source_reliability_em_fit_learns_lower_reliability_for_conflicting_source() {
        let mut state = StructuralPriorLearningState::default();
        state.event_ledger.extend([
            source_em_event("backtest", "rec-1", Some("win")),
            source_em_event("live", "rec-1", Some("win")),
            source_em_event("analyze", "rec-1", Some("loss")),
            source_em_event("backtest", "rec-2", Some("loss")),
            source_em_event("live", "rec-2", Some("loss")),
            source_em_event("analyze", "rec-2", Some("win")),
            source_em_event("backtest", "rec-3", Some("win")),
            source_em_event("live", "rec-3", Some("win")),
            source_em_event("analyze", "rec-3", Some("loss")),
        ]);

        let fit = structural_source_reliability_em_fit_from_state(&state);

        assert_eq!(
            fit.iteration_count,
            crate::state::STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS
        );
        let backtest = fit.source_reliability["backtest"];
        let live = fit.source_reliability["live"];
        let analyze = fit.source_reliability["analyze"];
        assert!(backtest > analyze);
        assert!(live > analyze);
        assert!(
            structural_source_reliability_multiplier(
                &state,
                "backtest",
                Some(&fit.source_reliability)
            ) > structural_source_reliability_multiplier(
                &state,
                "analyze",
                Some(&fit.source_reliability)
            )
        );
    }

    #[test]
    fn panel_derived_prior_uses_persisted_source_reliability_em_summary() {
        let stats = crate::state::StructuralPriorStats {
            source_panel_summaries: BTreeMap::from([
                (
                    "analyze".to_string(),
                    crate::state::StructuralPriorSourceSummary {
                        weighted_success_mass: 1.0,
                        ..crate::state::StructuralPriorSourceSummary::default()
                    },
                ),
                (
                    "backtest".to_string(),
                    crate::state::StructuralPriorSourceSummary {
                        weighted_failure_mass: 1.0,
                        ..crate::state::StructuralPriorSourceSummary::default()
                    },
                ),
            ]),
            ..crate::state::StructuralPriorStats::default()
        };
        let state = crate::state::StructuralPriorLearningState {
            source_reliability_em_summaries: BTreeMap::from([
                (
                    "analyze".to_string(),
                    crate::state::StructuralSourceReliabilityEmSourceSummary {
                        source_label: "analyze".to_string(),
                        iteration_count: crate::state::STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS,
                        latent_item_count: 3,
                        distinct_label_count: 2,
                        confusion_cell_count: 4,
                        posterior_reliability: 0.2,
                        min_diagonal_probability: 0.2,
                        ..crate::state::StructuralSourceReliabilityEmSourceSummary::default()
                    },
                ),
                (
                    "backtest".to_string(),
                    crate::state::StructuralSourceReliabilityEmSourceSummary {
                        source_label: "backtest".to_string(),
                        iteration_count: crate::state::STRUCTURAL_SOURCE_RELIABILITY_EM_ITERATIONS,
                        latent_item_count: 3,
                        distinct_label_count: 2,
                        confusion_cell_count: 4,
                        posterior_reliability: 0.9,
                        min_diagonal_probability: 0.8,
                        ..crate::state::StructuralSourceReliabilityEmSourceSummary::default()
                    },
                ),
            ]),
            ..crate::state::StructuralPriorLearningState::default()
        };

        let prior = structural_panel_derived_smoothed_prior(&stats, &state)
            .expect("panel prior from persisted EM reliability");

        assert!(prior < 0.4);
        assert!(prior > 0.35);
    }

    #[test]
    fn structural_prior_maturity_diagnostics_count_unresolved_followed_feedback() {
        let stats = StructuralPriorStats {
            observations: 5,
            followed_count: 4,
            wins: 1,
            losses: 1,
            breakevens: 1,
            invalidated: 0,
            abandoned: 0,
            not_followed: 1,
            smoothed_prior: 0.5,
            target_policy_reward_prior: 0.6,
            target_policy_reward_lower_bound: 0.3,
            delayed_reward_elapsed_feedback_count: 3,
            delayed_reward_elapsed_hours_at_risk: 6.0,
            delayed_reward_avg_elapsed_hours: 2.0,
            delayed_reward_resolution_hazard_per_hour: 3.0 / 6.0,
            delayed_reward_expected_resolution_hours: 2.0,
            delayed_reward_survival_probability_1h: (-0.5_f64).exp(),
            delayed_reward_survival_probability_4h: (-2.0_f64).exp(),
            delayed_reward_survival_probability_24h: (-12.0_f64).exp(),
            delayed_reward_success_hazard_per_hour: 1.5 / 6.0,
            delayed_reward_failure_hazard_per_hour: 1.5 / 6.0,
            delayed_reward_success_cumulative_incidence_4h: 0.5 * (1.0 - (-2.0_f64).exp()),
            delayed_reward_failure_cumulative_incidence_4h: 0.5 * (1.0 - (-2.0_f64).exp()),
            delayed_reward_resolution_horizon_1h_count: 3,
            delayed_reward_resolution_within_1h_count: 1,
            delayed_reward_resolution_probability_1h: 2.0 / 5.0,
            delayed_reward_resolution_horizon_4h_count: 3,
            delayed_reward_resolution_within_4h_count: 3,
            delayed_reward_resolution_probability_4h: 4.0 / 5.0,
            delayed_reward_resolution_horizon_24h_count: 3,
            delayed_reward_resolution_within_24h_count: 3,
            delayed_reward_resolution_probability_24h: 4.0 / 5.0,
            ..StructuralPriorStats::default()
        };

        assert_eq!(
            structural_prior_matured_feedback_count(Some(&stats)),
            Some(3)
        );
        assert_eq!(
            structural_prior_unresolved_feedback_count(Some(&stats)),
            Some(1)
        );
        assert_eq!(structural_prior_maturity_coverage(Some(&stats)), Some(0.75));
        assert_eq!(structural_prior_censoring_rate(Some(&stats)), Some(0.25));
        assert_eq!(
            structural_prior_delayed_reward_resolution_probability(Some(&stats)),
            Some(4.0 / 6.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_censoring_probability(Some(&stats)),
            Some(2.0 / 6.0)
        );
        assert!(
            (structural_prior_censoring_adjusted_reward_prior(Some(&stats)).unwrap()
                - ((0.6 * (4.0 / 6.0)) + (0.5 * (2.0 / 6.0))))
                .abs()
                < 1e-9
        );
        assert!(
            (structural_prior_censoring_adjusted_reward_lower_bound(Some(&stats)).unwrap()
                - ((0.3 * (4.0 / 6.0)) + (0.5 * 0.5 * (2.0 / 6.0))))
                .abs()
                < 1e-9
        );
        let expected_competing_risks: [f64; 4] = [2.5 / 7.0, 2.5 / 7.0, 1.0 / 7.0, 1.0 / 7.0];
        let expected_competing_risk_entropy: f64 = expected_competing_risks
            .iter()
            .map(|risk| -*risk * (*risk).ln())
            .sum();
        assert_eq!(
            structural_prior_delayed_reward_success_competing_risk(Some(&stats)),
            Some(expected_competing_risks[0])
        );
        assert_eq!(
            structural_prior_delayed_reward_failure_competing_risk(Some(&stats)),
            Some(expected_competing_risks[1])
        );
        assert_eq!(
            structural_prior_delayed_reward_invalidation_competing_risk(Some(&stats)),
            Some(expected_competing_risks[2])
        );
        assert_eq!(
            structural_prior_delayed_reward_abandonment_competing_risk(Some(&stats)),
            Some(expected_competing_risks[3])
        );
        assert!(
            (structural_prior_delayed_reward_competing_risk_entropy(Some(&stats)).unwrap()
                - expected_competing_risk_entropy)
                .abs()
                < 1e-9
        );
        assert_eq!(
            structural_prior_delayed_reward_elapsed_feedback_count(Some(&stats)),
            Some(3)
        );
        assert_eq!(
            structural_prior_delayed_reward_elapsed_hours_at_risk(Some(&stats)),
            Some(6.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_avg_elapsed_hours(Some(&stats)),
            Some(2.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_resolution_hazard_per_hour(Some(&stats)),
            Some(3.0 / 6.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_expected_resolution_hours(Some(&stats)),
            Some(2.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_survival_probability_1h(Some(&stats)),
            Some((-0.5_f64).exp())
        );
        assert_eq!(
            structural_prior_delayed_reward_survival_probability_4h(Some(&stats)),
            Some((-2.0_f64).exp())
        );
        assert_eq!(
            structural_prior_delayed_reward_survival_probability_24h(Some(&stats)),
            Some((-12.0_f64).exp())
        );
        assert_eq!(
            structural_prior_delayed_reward_success_hazard_per_hour(Some(&stats)),
            Some(1.5 / 6.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_failure_hazard_per_hour(Some(&stats)),
            Some(1.5 / 6.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_success_cumulative_incidence_4h(Some(&stats)),
            Some(0.5 * (1.0 - (-2.0_f64).exp()))
        );
        assert_eq!(
            structural_prior_delayed_reward_failure_cumulative_incidence_4h(Some(&stats)),
            Some(0.5 * (1.0 - (-2.0_f64).exp()))
        );
        assert_eq!(
            structural_prior_delayed_reward_invalidation_cumulative_incidence_4h(Some(&stats)),
            None
        );
        assert_eq!(
            structural_prior_delayed_reward_abandonment_cumulative_incidence_4h(Some(&stats)),
            None
        );
        assert_eq!(
            structural_prior_delayed_reward_invalidation_hazard_per_hour(Some(&stats)),
            None
        );
        assert_eq!(
            structural_prior_delayed_reward_resolution_horizon_1h_count(Some(&stats)),
            Some(3)
        );
        assert_eq!(
            structural_prior_delayed_reward_resolution_within_1h_count(Some(&stats)),
            Some(1)
        );
        assert_eq!(
            structural_prior_delayed_reward_resolution_probability_1h(Some(&stats)),
            Some(2.0 / 5.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_resolution_probability_4h(Some(&stats)),
            Some(4.0 / 5.0)
        );
        assert_eq!(
            structural_prior_delayed_reward_resolution_probability_24h(Some(&stats)),
            Some(4.0 / 5.0)
        );

        let not_followed_only = StructuralPriorStats {
            observations: 1,
            not_followed: 1,
            ..StructuralPriorStats::default()
        };
        assert_eq!(
            structural_prior_matured_feedback_count(Some(&not_followed_only)),
            Some(0)
        );
        assert_eq!(
            structural_prior_unresolved_feedback_count(Some(&not_followed_only)),
            Some(0)
        );
        assert_eq!(
            structural_prior_maturity_coverage(Some(&not_followed_only)),
            None
        );
        assert_eq!(
            structural_prior_censoring_rate(Some(&not_followed_only)),
            None
        );
        assert_eq!(
            structural_prior_delayed_reward_resolution_probability(Some(&not_followed_only)),
            None
        );
        assert_eq!(
            structural_prior_delayed_reward_success_competing_risk(Some(&not_followed_only)),
            None
        );
    }

    #[test]
    fn panel_derived_prior_uses_source_confusion_concentration() {
        let mut stats = StructuralPriorStats::default();
        stats.source_panel_summaries.insert(
            "noisy".to_string(),
            crate::state::StructuralPriorSourceSummary {
                weighted_success_mass: 2.0,
                ..crate::state::StructuralPriorSourceSummary::default()
            },
        );
        let mut state = StructuralPriorLearningState::default();
        state.source_reliability_posteriors.insert(
            "noisy".to_string(),
            crate::state::StructuralSourceReliabilityPosterior {
                source_label: "noisy".to_string(),
                observations: 2,
                weighted_observation_mass: 2.0,
                posterior_reliability: 1.0,
                outcome_confusion: BTreeMap::from([
                    (
                        "tp->positive_executed".to_string(),
                        crate::state::StructuralSourceOutcomeConfusionCell {
                            observed_outcome: "tp".to_string(),
                            credit_class: "positive_executed".to_string(),
                            observations: 1,
                            weighted_observation_mass: 1.0,
                            weighted_success_mass: 1.0,
                            ..crate::state::StructuralSourceOutcomeConfusionCell::default()
                        },
                    ),
                    (
                        "take_profit->positive_executed".to_string(),
                        crate::state::StructuralSourceOutcomeConfusionCell {
                            observed_outcome: "take_profit".to_string(),
                            credit_class: "positive_executed".to_string(),
                            observations: 1,
                            weighted_observation_mass: 1.0,
                            weighted_success_mass: 1.0,
                            ..crate::state::StructuralSourceOutcomeConfusionCell::default()
                        },
                    ),
                ]),
                ..crate::state::StructuralSourceReliabilityPosterior::default()
            },
        );

        let prior =
            structural_panel_derived_smoothed_prior(&stats, &state).expect("panel-derived prior");

        assert!((prior - (2.0 / 3.0)).abs() < 1e-9);
    }

    fn calibrated_evaluation_row(
        path_id: &str,
        raw_path_score: f64,
        calibrated_path_prob: f64,
        pending_reward_state: &str,
        bucket: &str,
    ) -> StructuralPathRankingTargetRow {
        StructuralPathRankingTargetRow {
            calibrated_path_prob: Some(calibrated_path_prob),
            path_prob_lower_bound: Some((calibrated_path_prob - 0.1).clamp(0.0, 1.0)),
            regime_calibration_bucket: bucket.to_string(),
            ..calibration_row(path_id, raw_path_score, pending_reward_state)
        }
    }

    #[test]
    fn structural_path_probability_calibration_writes_probabilities_for_raw_scored_rows() {
        let mut artifact = StructuralPathRankingTargetArtifact {
            protocol_version: "structural-path-ranking-target-v1".to_string(),
            symbol: "NQ".to_string(),
            candidate_set_id: "structural-candidates:NQ:test".to_string(),
            candidate_set_size: 3,
            generated_at: "2026-05-02T00:00:00Z".to_string(),
            rows: vec![
                calibration_row("path-success", 0.8, "matured_success"),
                calibration_row("path-failure", 0.2, "matured_invalidated"),
                calibration_row("path-live", 0.6, "unobserved"),
                StructuralPathRankingTargetRow {
                    raw_path_score: None,
                    ..calibration_row("path-no-score", 0.4, "matured_success")
                },
            ],
        };

        let report = apply_structural_path_probability_calibration(&mut artifact);

        assert_eq!(report.status, "calibrated");
        assert_eq!(report.observed_rows, 2);
        assert_eq!(report.calibrated_rows, 3);
        assert_eq!(report.bins.len(), 1);
        assert!((report.bins[0].calibrated_path_prob - 0.5).abs() < 1e-9);
        assert!(report.bins[0].path_prob_lower_bound < 0.5);
        assert!(artifact
            .rows
            .iter()
            .filter(|row| row.raw_path_score.is_some())
            .all(|row| row.calibrated_path_prob == Some(0.5)));
        assert_eq!(
            artifact
                .rows
                .iter()
                .find(|row| row.path_id == "path-no-score")
                .and_then(|row| row.calibrated_path_prob),
            None
        );
        assert!(artifact
            .rows
            .iter()
            .filter(|row| row.raw_path_score.is_some())
            .all(|row| row.path_prob_lower_bound.is_some()));
        assert!(artifact
            .rows
            .iter()
            .filter(|row| row.raw_path_score.is_some())
            .all(|row| row.execution_gate_status.as_deref() == Some("observe")));
        assert!(artifact
            .rows
            .iter()
            .filter(|row| row.raw_path_score.is_some())
            .all(|row| row.execution_gate_min_path_prob == Some(0.5)));
        assert_eq!(
            artifact
                .rows
                .iter()
                .find(|row| row.path_id == "path-no-score")
                .and_then(|row| row.execution_gate_status.as_deref()),
            None
        );
        let matured_success = artifact
            .rows
            .iter()
            .find(|row| row.path_id == "path-success")
            .unwrap();
        assert_eq!(matured_success.calibrated_label, Some(1.0));
        assert_eq!(matured_success.ips_weight, Some(2.0));
        assert_eq!(matured_success.training_weight, Some(2.0));
        assert_eq!(
            artifact
                .rows
                .iter()
                .find(|row| row.path_id == "path-live")
                .and_then(|row| row.training_weight),
            None
        );
    }

    #[test]
    fn structural_path_probability_calibration_evaluation_scores_mature_calibrated_rows() {
        let rows = vec![
            calibrated_evaluation_row("trend-win", 0.8, 0.8, "matured_success", "NQ:trend"),
            calibrated_evaluation_row("trend-loss", 0.6, 0.6, "matured_failure", "NQ:trend"),
            calibrated_evaluation_row("range-loss", 0.2, 0.2, "matured_invalidated", "NQ:range"),
            calibrated_evaluation_row("range-win", 0.4, 0.4, "matured_success", "NQ:range"),
            calibrated_evaluation_row("pending", 0.5, 0.5, "unobserved", "NQ:trend"),
            StructuralPathRankingTargetRow {
                raw_path_score: None,
                ..calibrated_evaluation_row("no-score", 0.5, 0.5, "matured_success", "NQ:trend")
            },
        ];

        let report = evaluate_structural_path_probability_calibration_rows(&rows);

        assert_eq!(report.status, "evaluated");
        assert_eq!(report.eligible_rows, 4);
        assert!((report.brier_score.unwrap() - 0.20).abs() < 1e-9);
        assert!((report.expected_calibration_error.unwrap() - 0.20).abs() < 1e-9);
        assert!((report.max_calibration_error.unwrap() - 0.20).abs() < 1e-9);
        assert_eq!(report.bins.len(), 2);
    }

    #[test]
    fn structural_path_probability_calibration_evaluation_reports_propensity_weighted_brier() {
        let mut low_propensity_loss = calibrated_evaluation_row(
            "low-propensity-loss",
            0.9,
            0.9,
            "matured_failure",
            "NQ:trend",
        );
        low_propensity_loss.propensity_estimate = Some(0.25);
        low_propensity_loss.ips_weight = Some(4.0);
        low_propensity_loss.training_weight = Some(4.0);
        let mut high_propensity_win = calibrated_evaluation_row(
            "high-propensity-win",
            0.5,
            0.5,
            "matured_success",
            "NQ:trend",
        );
        high_propensity_win.propensity_estimate = Some(1.0);
        high_propensity_win.ips_weight = Some(1.0);
        high_propensity_win.training_weight = Some(1.0);

        let report = evaluate_structural_path_probability_calibration_rows(&[
            low_propensity_loss,
            high_propensity_win,
        ]);

        assert_eq!(report.status, "evaluated");
        assert_eq!(report.eligible_rows, 2);
        assert_eq!(report.propensity_weighted_rows, 2);
        assert!((report.brier_score.unwrap() - 0.53).abs() < 1e-9);
        assert!((report.propensity_weighted_brier_score.unwrap() - 0.698).abs() < 1e-9);
    }

    fn feedback_record_for_target_export(
        recommendation_id: &str,
        path_id: &str,
        outcome: &str,
        selected_probability: f64,
    ) -> FeedbackRecord {
        FeedbackRecord {
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            source: "structural_feedback_submission".to_string(),
            run_id: Some(recommendation_id.to_string()),
            trade_id: Some(recommendation_id.to_string()),
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: Vec::new(),
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: Direction::Bull,
                selected_probability,
                long_score: selected_probability,
                short_score: 1.0 - selected_probability,
                win_prob_long: selected_probability,
                win_prob_short: 1.0 - selected_probability,
                uncertainty: 1.0 - selected_probability,
            },
            realized_outcome: outcome.to_string(),
            pnl: if outcome == "win" { 1.0 } else { -1.0 },
            regime_at_entry: Regime::ManipulationExpansion,
            structural_feedback: Some(StructuralFeedbackRefs {
                protocol_version: "structural-feedback-v1".to_string(),
                recommendation_id: recommendation_id.to_string(),
                recommended_at: "2026-05-09T00:00:00Z".to_string(),
                node_id: "NQ:belief_regime_node:trend".to_string(),
                branch_id: "NQ:belief_regime_node:trend:trend_follow_through".to_string(),
                scenario_id: format!("scenario:{path_id}"),
                path_id: path_id.to_string(),
                followed_path: true,
                exit_reason: None,
                notes: None,
            }),
            reflection_mismatch_tags: Vec::new(),
        }
    }

    #[test]
    fn target_export_projects_mature_structural_feedback_into_history_rows() {
        let temp = tempfile::tempdir().unwrap();
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            ..WorkflowSnapshot::default()
        };
        let path_id = "path:scenario:NQ:belief_regime_node:trend:trend_follow_through:primary";
        let feedback = vec![
            feedback_record_for_target_export("rec-win", path_id, "win", 0.91),
            feedback_record_for_target_export("rec-loss", path_id, "loss", 0.24),
        ];

        let summary = export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &ProviderCatalogAgentSurface::default(),
            &feedback,
            &StructuralPriorLearningState::default(),
        )
        .unwrap();
        let history_rows =
            load_structural_path_ranking_target_rows(Path::new(&summary.history_jsonl_path))
                .unwrap();

        assert!(summary.history_mature_rows >= 2);
        assert!(summary.history_rows_with_raw_path_score >= 2);
        assert!(
            history_rows
                .iter()
                .filter(|row| row.path_id == path_id && row.maturity_mask)
                .count()
                >= 2
        );
        assert!(history_rows.iter().any(|row| {
            row.path_id == path_id
                && row.pending_reward_state == "matured_success"
                && row.raw_path_score == Some(0.91)
        }));
        assert!(history_rows.iter().any(|row| {
            row.path_id == path_id
                && row.pending_reward_state == "matured_failure"
                && row.raw_path_score == Some(0.24)
        }));
    }

    #[test]
    fn path_ranker_runtime_falls_back_to_history_when_registered_artifact_misses_path() {
        let temp = tempfile::tempdir().unwrap();
        let snapshot =
            crate::application::orchestration::workflow_status::sample_human_workflow_snapshot();
        let summary = export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &ProviderCatalogAgentSurface::default(),
            &[],
            &StructuralPriorLearningState::default(),
        )
        .unwrap();
        let current_rows =
            load_structural_path_ranking_target_rows(Path::new(&summary.jsonl_path)).unwrap();
        let mut history_score_row = current_rows.first().expect("current row").clone();
        let path_id = history_score_row.path_id.clone();
        history_score_row.candidate_set_id = "structural-candidates:NQ:history".to_string();
        history_score_row.raw_path_score = Some(0.91);
        fs::write(
            &summary.history_jsonl_path,
            render_structural_path_ranking_target_rows_jsonl(&[history_score_row]).unwrap(),
        )
        .unwrap();
        let artifact_dir = Path::new(&summary.summary_path)
            .parent()
            .expect("summary parent")
            .to_path_buf();
        fs::write(
            artifact_dir.join("artifact_scores.jsonl"),
            format!(
                "{}\n",
                serde_json::json!({
                    "candidate_set_id": "structural-candidates:NQ:other",
                    "path_id": "path:scenario:NQ:belief_regime_node:range:range_mean_reversion:primary",
                    "raw_path_score": 0.12
                })
            ),
        )
        .unwrap();
        let artifact =
            crate::application::entry_models::training_export::StructuralPathRankingTrainerArtifact {
                protocol_version: "structural-path-ranking-trainer-artifact-v1".to_string(),
                dataset_role: "external_path_ranker_training_dataset".to_string(),
                model_family: "catboost".to_string(),
                artifact_uri: "artifact_scores.jsonl".to_string(),
                model_artifact_uri: None,
                score_column: "raw_path_score".to_string(),
                trained_rows: 42,
                history_rows: 42,
                calibration_rows: 12,
                selected_features: vec!["rank".to_string()],
                validation_metrics:
                    crate::belief_core::ranking_label::StructuralPathRankerValidationMetrics::default(),
                calibration_metrics:
                    crate::belief_core::ranking_label::StructuralPathRankerCalibrationMetrics::default(),
                rule_list: Vec::new(),
                tree_json: None,
                created_at: None,
                notes: vec![],
            };
        fs::write(
            artifact_dir.join("structural_path_ranking_trainer_artifact.json"),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_PREFER_HISTORY,
        )
        .unwrap();

        let selection = structural_ranked_paths_with_runtime_context_and_prior_state(
            &snapshot,
            &ProviderCatalogAgentSurface::default(),
            &[],
            &StructuralPriorLearningState::default(),
            StructuralPathRankerRuntimeContext {
                state_dir: Some(temp.path().to_str().unwrap()),
            },
        );

        let runtime = selection.runtime.expect("runtime surface");
        assert_eq!(runtime.status, "using_history_scores");
        assert_eq!(runtime.artifact_match_count, 0);
        assert_eq!(runtime.history_match_count, 1);
        assert_eq!(runtime.applied_path_count, 1);
        assert_eq!(
            selection
                .paths
                .iter()
                .find(|path| path.path_id == path_id)
                .and_then(|path| path.path_ranker_runtime_source.as_deref()),
            Some("history_path")
        );
    }

    #[test]
    fn path_ranker_runtime_applies_registered_artifact_scores_to_ranked_paths() {
        let temp = tempfile::tempdir().unwrap();
        let snapshot =
            crate::application::orchestration::workflow_status::sample_human_workflow_snapshot();
        let summary = export_structural_path_ranking_target(
            temp.path().to_str().unwrap(),
            "NQ",
            &snapshot,
            &ProviderCatalogAgentSurface::default(),
            &[],
            &StructuralPriorLearningState::default(),
        )
        .unwrap();
        let current_rows =
            load_structural_path_ranking_target_rows(Path::new(&summary.jsonl_path)).unwrap();
        assert!(!current_rows.is_empty(), "expected structural target rows");
        let artifact_dir = Path::new(&summary.summary_path)
            .parent()
            .expect("summary parent")
            .to_path_buf();
        let score_lines = current_rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                serde_json::json!({
                    "candidate_set_id": row.candidate_set_id,
                    "path_id": row.path_id,
                    "raw_path_score": 0.91 - index as f64 * 0.04,
                    "calibrated_path_prob": 0.86 - index as f64 * 0.03,
                    "path_prob_lower_bound": 0.74 - index as f64 * 0.02,
                    "execution_gate_status": if index == 0 { "pass" } else { "observe" },
                })
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(
            artifact_dir.join("artifact_scores.jsonl"),
            format!("{score_lines}\n"),
        )
        .unwrap();
        let artifact =
            crate::application::entry_models::training_export::StructuralPathRankingTrainerArtifact {
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
        fs::write(
            artifact_dir.join("structural_path_ranking_trainer_artifact.json"),
            serde_json::to_string_pretty(&artifact).unwrap(),
        )
        .unwrap();
        crate::application::entry_models::enable_structural_path_ranking_runtime_command(
            temp.path().to_str().unwrap(),
            "NQ",
            STRUCTURAL_PATH_RANKING_RUNTIME_MODE_CANDIDATE_SET_ONLY,
        )
        .unwrap();

        let selection = structural_ranked_paths_with_runtime_context_and_prior_state(
            &snapshot,
            &ProviderCatalogAgentSurface::default(),
            &[],
            &StructuralPriorLearningState::default(),
            StructuralPathRankerRuntimeContext {
                state_dir: Some(temp.path().to_str().unwrap()),
            },
        );

        let runtime = selection.runtime.expect("runtime surface");
        assert_eq!(runtime.status, "using_registered_artifact_scores");
        assert_eq!(runtime.artifact_match_count, current_rows.len());
        assert_eq!(runtime.applied_path_count, current_rows.len());
        let expected_first = current_rows.first().expect("first row");
        let selected = selection
            .paths
            .iter()
            .find(|path| path.path_id == expected_first.path_id)
            .expect("ranked path with registered score");
        assert_eq!(
            selected.path_ranker_runtime_source.as_deref(),
            Some("registered_artifact")
        );
        assert_eq!(selected.catboost_score, Some(0.91));
        assert_eq!(
            selected.path_ranker_execution_gate_status.as_deref(),
            Some("pass")
        );
    }
}
