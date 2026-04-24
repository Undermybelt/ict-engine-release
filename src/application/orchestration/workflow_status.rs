use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::application::belief::{
    jump_calibration_gate_workflow_summary, jump_model_workflow_summary,
};
use crate::application::output_foundation::{
    print_redacted_json, redact_local_paths_in_value, short_workflow_phase_summary,
};
use crate::application::release_closure::workflow_next_step_view;
use crate::config::shell_quote;
use crate::state::{
    ArtifactConsumedImpactSummary, ArtifactDecisionSummary, ArtifactFactorTrendSummary,
    ArtifactFamilyTrendSummary, ArtifactHistorySummary, ArtifactLineageSummary,
    ArtifactRuleBreakEffect, ArtifactRuleBreakFactorImpact, ArtifactRuleBreakFamilyImpact,
    DatasetComparability, EnsembleExecutorScorecard, EnsembleVoteRecord,
    ExecutionCandidateArtifactSummary, PendingUpdateArtifactSummary, PreBayesEntryQualityBridge,
    PreBayesEntryQualityBridgeDiff, PreBayesEvidencePolicy, PreBayesPolicyDiff,
    PreBayesPolicyLineageSummary, PreBayesPolicyRecord, PreBayesSoftEvidenceNodeDiff,
    RunProvenance, WorkflowSnapshot,
};

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
        "ict-engine factor-autoresearch --symbol {} --state-dir {}",
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

pub fn build_human_workflow_status_view(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
) -> Value {
    let no_workflow_state = workflow_status_empty_state(snapshot);
    let latest_phase = snapshot
        .latest_update
        .as_ref()
        .or(snapshot.latest_research.as_ref())
        .or(snapshot.latest_analyze.as_ref())
        .or(snapshot.latest_backtest.as_ref())
        .or(snapshot.latest_train.as_ref());
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
    let top_level_command = if hard_block_active {
        snapshot.blocking_truth.next_command.clone()
    } else {
        snapshot.recommended_next_command.clone()
    };
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
            selected_data_candidates.join(", ")
        )
    } else {
        String::new()
    };
    let user_path_input_prompt = if !selected_data_candidates.is_empty() {
        format!(
            "Reply with one path from the list, or paste another valid file path. Candidates: {}",
            selected_data_candidates.join(", ")
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
    let human_next_action = if user_selection_pending {
        if !historical_data_request_template.is_empty() {
            format!(
                "Ask the user to choose the historical dataset. {} {}",
                historical_data_request_template, user_path_input_prompt
            )
        } else {
            "Ask the user to provide the historical data path before running research/backtest."
                .to_string()
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
    let summary_line = format!(
        "{} | {} | {} | pda_cluster={} | duration={} | remaining_bars={} | spectral_entropy={} | sparsity={} | segments_gate={}",
        snapshot.symbol,
        workflow_status_focus_phase(snapshot),
        action_status_label,
        latest_pda_cluster,
        latest_duration_model,
        latest_remaining_bars,
        latest_spectral_entropy,
        latest_sparsity_ratio,
        latest_segments_gate
    );
    let blocking_line = format!("Block: {}", gate_reason_label);
    let next_action_line = format!("Next: {}", human_next_action);
    let phase_summary_line = format!(
        "Latest: {} | {}",
        latest_phase_label, latest_phase_summary_short
    );
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
    let ensemble_summary = snapshot.latest_ensemble_vote.as_ref().map(|vote| {
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
    serde_json::json!({
        "status": if no_workflow_state {
            serde_json::Value::String(NO_WORKFLOW_STATE.to_string())
        } else {
            serde_json::Value::Null
        },
        "summary_line": summary_line,
        "blocking_line": blocking_line,
        "next_action_line": next_action_line,
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
        "jump_disagreement": snapshot
            .latest_ensemble_vote
            .as_ref()
            .and_then(|vote| {
                vote.executor_summaries
                    .iter()
                    .find(|line| line.contains("jump_disagreement"))
                    .cloned()
            }),
    })
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
    let latest_phase = snapshot
        .latest_update
        .as_ref()
        .or(snapshot.latest_research.as_ref())
        .or(snapshot.latest_analyze.as_ref())
        .or(snapshot.latest_backtest.as_ref())
        .or(snapshot.latest_train.as_ref());
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
    let no_workflow_state = workflow_status_empty_state(snapshot);
    let latest_phase = snapshot
        .latest_update
        .as_ref()
        .or(snapshot.latest_research.as_ref())
        .or(snapshot.latest_analyze.as_ref())
        .or(snapshot.latest_backtest.as_ref())
        .or(snapshot.latest_train.as_ref());
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
    let next_command = if hard_block_active {
        snapshot.blocking_truth.next_command.clone()
    } else {
        snapshot.recommended_next_command.clone()
    };
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
    let ensemble_summary = snapshot.latest_ensemble_vote.as_ref().map(|vote| {
        let (scorecards, scorecard_source) = resolved_vote_scorecards(persisted_scorecards, vote);
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
    serde_json::json!({
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
        "next_command_source": command_source,
        "pda_cluster_label": latest_phase.and_then(|phase| phase.pda_cluster_label.clone()),
        "hybrid_duration_model": latest_phase.and_then(|phase| phase.hybrid_duration_model.clone()),
        "hybrid_remaining_expected_bars": latest_phase.and_then(|phase| phase.hybrid_remaining_expected_bars),
        "next_step": workflow_next_step_view(&next_command, if hard_block_active { Some(blocking_reason.as_str()) } else { None }),
        "pending_actions": snapshot.pending_actions.iter().take(3).cloned().collect::<Vec<_>>(),
        "risk_flags": snapshot.risk_flags.iter().take(3).cloned().collect::<Vec<_>>(),
        "top_disagreement": top_disagreement,
        "top_actionable": top_actionable,
        "ensemble": ensemble_summary,
    })
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

pub fn emit_workflow_status_output(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    output_format: &str,
    stable: bool,
) -> Result<()> {
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
            let mut value = build_agent_workflow_status_view(snapshot, persisted_scorecards);
            if stable {
                normalize_workflow_status_value_for_stability(&mut value);
            }
            redact_local_paths_in_value(&mut value);
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        "human" => {
            let mut value = build_human_workflow_status_view(snapshot, persisted_scorecards);
            redact_local_paths_in_value(&mut value);
            if let Some(summary) = value.get("summary_line").and_then(Value::as_str) {
                println!("{}", summary);
            }
            if let Some(blocking) = value.get("blocking_line").and_then(Value::as_str) {
                println!("{}", blocking);
            }
            if let Some(latest) = value.get("phase_summary_line").and_then(Value::as_str) {
                println!("{}", latest);
            }
            if let Some(next) = value.get("next_action_line").and_then(Value::as_str) {
                println!("{}", next);
            }
        }
        other => anyhow::bail!("unsupported output format '{}'", other),
    }
    Ok(())
}

pub fn dispatch_workflow_status(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
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
        let mut value = match phase.trim().to_ascii_lowercase().as_str() {
            "agent-bootstrap" | "bootstrap" => build_workflow_status_bootstrap_phase_value(
                bootstrap.symbol,
                bootstrap.state_dir,
                snapshot,
                bootstrap.detected_tomac_root,
                bootstrap.multi_timeframe_clean_root,
                &bootstrap.tomac_root_placeholder,
            )?,
            other => build_workflow_status_phase_value(snapshot, persisted_scorecards, other)?,
        };
        if input.stable {
            normalize_workflow_status_value_for_stability(&mut value);
        }
        redact_local_paths_in_value(&mut value);
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        emit_workflow_status_output(
            snapshot,
            persisted_scorecards,
            input.output_format,
            input.stable,
        )?;
    }
    Ok(())
}

pub fn build_agent_bootstrap_view(
    symbol: &str,
    state_dir: &str,
    snapshot: &WorkflowSnapshot,
    detected_tomac_root: Option<String>,
    multi_timeframe_clean_root: Option<String>,
    tomac_root_placeholder: &str,
) -> AgentBootstrapView {
    let agent_brief = vec![
        "mission: formalize factor-pipeline debug from latest signal through pre-bayes / bridge / resonance".to_string(),
        "priority: promote expansion_manipulation to SOP-tier objective, not research-only".to_string(),
        "guardrail: do not blind-tune structure_ict before evidence pinpoints the blocking surface".to_string(),
        "success: either find a real structure_ict mutation win or prove near-local-optimum then shift to label refinement / market fork".to_string(),
    ];
    let analyze_command = if let Some(clean_root) = &multi_timeframe_clean_root {
        format!(
            "ict-engine analyze --symbol {} --data-root {} --market {} --state-dir {}",
            shell_quote(symbol),
            shell_quote(clean_root),
            shell_quote(&symbol.to_ascii_lowercase()),
            shell_quote(state_dir)
        )
    } else {
        "ict-engine analyze --symbol <symbol> --data-root <clean-root> --market <market> --state-dir <state-dir>".to_string()
    };
    let train_command = if let Some(clean_root) = &multi_timeframe_clean_root {
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
    let clean_command = if let Some(root) = &detected_tomac_root {
        format!(
            "ict-engine clean-futures --root {} --output-dir {} --multi-timeframe",
            shell_quote(root),
            shell_quote(
                &multi_timeframe_clean_root
                    .clone()
                    .unwrap_or_else(|| format!("{}/ict-engine-mtf", root))
            )
        )
    } else {
        "ict-engine clean-futures --root <tomac-root> --output-dir <output-dir> --multi-timeframe"
            .to_string()
    };
    let inferable_live_defaults = std::collections::BTreeMap::from([
        (
            "NQ".to_string(),
            std::collections::BTreeMap::from([
                ("futures_symbol".to_string(), "NQ=F".to_string()),
                ("spot_symbol".to_string(), "QQQ".to_string()),
                ("options_symbol".to_string(), "QQQ".to_string()),
                ("spot_kind".to_string(), "equity".to_string()),
            ]),
        ),
        (
            "ES".to_string(),
            std::collections::BTreeMap::from([
                ("futures_symbol".to_string(), "ES=F".to_string()),
                ("spot_symbol".to_string(), "SPY".to_string()),
                ("options_symbol".to_string(), "SPY".to_string()),
                ("spot_kind".to_string(), "equity".to_string()),
            ]),
        ),
        (
            "YM".to_string(),
            std::collections::BTreeMap::from([
                ("futures_symbol".to_string(), "YM=F".to_string()),
                ("spot_symbol".to_string(), "DIA".to_string()),
                ("options_symbol".to_string(), "DIA".to_string()),
                ("spot_kind".to_string(), "equity".to_string()),
            ]),
        ),
        (
            "GC".to_string(),
            std::collections::BTreeMap::from([
                ("futures_symbol".to_string(), "GC=F".to_string()),
                ("spot_symbol".to_string(), "GLD".to_string()),
                ("options_symbol".to_string(), "GLD".to_string()),
                ("spot_kind".to_string(), "etf".to_string()),
            ]),
        ),
        (
            "CL".to_string(),
            std::collections::BTreeMap::from([
                ("futures_symbol".to_string(), "CL=F".to_string()),
                ("spot_symbol".to_string(), "USO".to_string()),
                ("options_symbol".to_string(), "USO".to_string()),
                ("spot_kind".to_string(), "etf".to_string()),
            ]),
        ),
    ]);
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
            tomac_history_root: detected_tomac_root,
            multi_timeframe_clean_root: multi_timeframe_clean_root.clone(),
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
                    &multi_timeframe_clean_root
                        .clone()
                        .unwrap_or_else(|| "<output-dir>".to_string())
                )
            ),
            expansion_sop: format!(
                "ict-engine expansion-sop --root {} --output-dir {} --interval 15m --lookback 20 --atr-multiplier 1.50",
                shell_quote(tomac_root_placeholder),
                shell_quote(
                    &multi_timeframe_clean_root
                        .clone()
                        .unwrap_or_else(|| "<output-dir>".to_string())
                )
            ),
            workflow_status: format!(
                "ict-engine workflow-status --symbol {} --state-dir {}",
                shell_quote(symbol),
                shell_quote(state_dir)
            ),
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

fn short_human_phase_summary(phase: &crate::state::WorkflowPhaseSnapshot) -> String {
    let mut parts = Vec::new();
    if let Some(direction) = &phase.selected_direction {
        parts.push(format!("direction={direction}"));
    }
    if let Some(entry) = &phase.selected_entry_quality {
        parts.push(format!("entry={entry}"));
    }
    if !phase.pre_bayes_gate_status.is_empty() {
        parts.push(format!("gate={}", phase.pre_bayes_gate_status));
    }
    if phase.pre_bayes_evidence_quality_score > 0.0 {
        parts.push(format!(
            "quality={:.3}",
            phase.pre_bayes_evidence_quality_score
        ));
    }
    if parts.is_empty() {
        phase.phase_summary.clone()
    } else {
        parts.join(" ")
    }
}

pub fn build_ensemble_vote_surface(
    vote: &EnsembleVoteRecord,
    persisted_scorecards: &[EnsembleExecutorScorecard],
) -> WorkflowEnsembleVoteSurface {
    let (scorecards, scorecard_source) = resolved_vote_scorecards(persisted_scorecards, vote);
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

pub fn build_workflow_status_phase_value(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
    phase: &str,
) -> Result<Value> {
    Ok(match phase.trim().to_ascii_lowercase().as_str() {
        "human" | "human-next" | "human-next-action" => {
            build_human_workflow_status_view(snapshot, persisted_scorecards)
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
            snapshot
                .latest_ensemble_vote
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
    Ok(
        match section.map(|value| value.trim().to_ascii_lowercase()) {
            None => serde_json::to_value(json!({
                "latest_policy": pre.pre_bayes_policy,
                "latest_bridge": pre.pre_bayes_entry_quality_bridge,
                "latest_bridge_diff": pre.pre_bayes_entry_quality_bridge_diff,
                "latest_policy_diff": pre.pre_bayes_policy_diff,
                "latest_policy_lineage": pre.pre_bayes_policy_lineage,
                "latest_gate_status": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_gate_status.clone()),
                "latest_policy_version": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_policy_version.clone()),
                "latest_uses_soft_evidence": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_uses_soft_evidence),
                "latest_soft_evidence_diff": pre.pre_bayes_soft_evidence_diff,
                "latest_soft_evidence": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_soft_evidence.clone()),
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
                "status": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_gate_status.clone()),
                "policy_version": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_policy_version.clone()),
                "uses_soft_evidence": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_uses_soft_evidence),
            }))?,
            Some(section) if section == "soft" || section == "soft-evidence" => {
                serde_json::to_value(
                    snapshot
                        .latest_analyze
                        .as_ref()
                        .map(|phase| phase.pre_bayes_soft_evidence.clone()),
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
) -> Result<()> {
    let value = build_pre_bayes_status_value(snapshot, section)?;
    print_redacted_json(&value)
}

pub fn build_pre_bayes_diff_value(snapshot: &WorkflowSnapshot) -> Value {
    json!({
        "latest_policy_diff": snapshot.latest_pre_bayes_policy_diff,
        "latest_policy_lineage": snapshot.latest_pre_bayes_policy_lineage,
        "latest_gate_status": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_gate_status.clone()),
        "latest_policy_version": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_policy_version.clone()),
        "latest_uses_soft_evidence": snapshot.latest_analyze.as_ref().map(|phase| phase.pre_bayes_uses_soft_evidence),
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
    detected_tomac_root: Option<String>,
    multi_timeframe_clean_root: Option<String>,
    tomac_root_placeholder: &str,
) -> Result<Value> {
    Ok(serde_json::to_value(build_agent_bootstrap_view(
        symbol,
        state_dir,
        snapshot,
        detected_tomac_root,
        multi_timeframe_clean_root,
        tomac_root_placeholder,
    ))?)
}

pub fn build_ensemble_vote_history_view(
    snapshot: &WorkflowSnapshot,
    persisted_scorecards: &[EnsembleExecutorScorecard],
) -> WorkflowEnsembleVoteHistoryView {
    let history = snapshot
        .recent_ensemble_votes
        .iter()
        .map(|vote| {
            let surface = build_ensemble_vote_surface(vote, persisted_scorecards);
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
    WorkflowPreBayesSurfaces {
        pre_bayes_policy: snapshot.latest_pre_bayes_policy.clone(),
        pre_bayes_policy_history: snapshot.recent_pre_bayes_policies.clone(),
        pre_bayes_policy_diff: snapshot.latest_pre_bayes_policy_diff.clone(),
        pre_bayes_policy_lineage: snapshot.latest_pre_bayes_policy_lineage.clone(),
        pre_bayes_entry_quality_bridge: snapshot.latest_pre_bayes_entry_quality_bridge.clone(),
        pre_bayes_entry_quality_bridge_diff: snapshot
            .latest_pre_bayes_entry_quality_bridge_diff
            .clone(),
        pre_bayes_soft_evidence: snapshot
            .latest_analyze
            .as_ref()
            .map(|phase| phase.pre_bayes_soft_evidence.clone()),
        pre_bayes_soft_evidence_diff: snapshot.latest_pre_bayes_soft_evidence_diff.clone(),
    }
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
        assert_eq!(
            value["latest_soft_evidence_diff"].as_array().unwrap().len(),
            1
        );
    }

    #[test]
    fn build_workflow_status_bootstrap_phase_value_matches_bootstrap_view() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_workflow_status_bootstrap_phase_value(
            "NQ",
            "state",
            &snapshot,
            Some("/tmp/tomac".to_string()),
            Some("/tmp/clean".to_string()),
            "<root>",
        )
        .unwrap();
        assert_eq!(value["symbol"], "NQ");
        assert_eq!(value["detected_paths"]["state_dir"], "state");
        assert_eq!(
            value["commands"]["workflow_status"],
            "ict-engine workflow-status --symbol NQ --state-dir state"
        );
    }

    #[test]
    fn build_workflow_status_phase_value_matches_human_surface() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_workflow_status_phase_value(&snapshot, &[], "human").unwrap();
        assert_eq!(
            value["summary_line"],
            "NQ | update | action_blocked | pda_cluster=cluster_1 | duration=negative_binomial | remaining_bars=2.50 | spectral_entropy=unavailable | sparsity=unavailable | segments_gate=unavailable"
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
            "artifact-factor-consumed-leaderboard",
        )
        .unwrap();
        assert_eq!(value.as_array().unwrap()[0]["factor_name"], "fvg_rebalance");
    }

    #[test]
    fn build_workflow_status_phase_value_rejects_unknown_phase() {
        let err = build_workflow_status_phase_value(&WorkflowSnapshot::default(), &[], "wat")
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported workflow-status phase 'wat'"));
    }

    #[test]
    fn build_workflow_status_phase_value_preserves_redactable_paths() {
        let snapshot = sample_human_workflow_snapshot();
        let mut value = build_workflow_status_phase_value(&snapshot, &[], "human").unwrap();
        redact_local_paths_in_value(&mut value);
        let rendered = serde_json::to_string(&value).unwrap();
        assert!(rendered.contains("<local-path>"));
    }

    #[test]
    fn dispatch_workflow_status_rejects_phase_and_filter_mix() {
        let error = dispatch_workflow_status(
            &sample_human_workflow_snapshot(),
            &[],
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
            "NQ | update | action_blocked | pda_cluster=cluster_1 | duration=negative_binomial | remaining_bars=2.50 | spectral_entropy=unavailable | sparsity=unavailable | segments_gate=unavailable"
        );
        assert_eq!(
            value["next_action_line"],
            "Next: Ask the user to choose the historical dataset. Please choose one historical data path for the next research/backtest run: /tmp/a.json, /tmp/b.json Reply with one path from the list, or paste another valid file path. Candidates: /tmp/a.json, /tmp/b.json"
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
    }

    #[test]
    fn agent_workflow_status_empty_state_uses_explicit_no_state_contract() {
        let value = build_agent_workflow_status_view(&WorkflowSnapshot::default(), &[]);
        assert_eq!(value["status"], "no_workflow_state");
        assert_eq!(value["latest_phase"], "no_workflow_state");
        assert_eq!(value["blocking_status"], "no_workflow_state");
        assert_eq!(value["blocking_reason"], "no_workflow_state");
        assert!(value["next_command"].is_null());
        assert_eq!(value["next_step"]["action_type"], "none");
    }

    #[test]
    fn ensemble_vote_history_view_uses_resolved_scorecard_source() {
        let vote = sample_human_workflow_snapshot()
            .latest_ensemble_vote
            .expect("sample ensemble vote");
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
            .contains("ict-engine factor-autoresearch"));
    }
}
