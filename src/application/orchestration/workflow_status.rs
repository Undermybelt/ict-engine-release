use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::application::belief::{
    jump_calibration_gate_workflow_summary, jump_model_workflow_summary,
};
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

pub fn build_phase_snapshot_surfaces(snapshot: &WorkflowSnapshot) -> WorkflowPhaseSnapshotSurfaces {
    WorkflowPhaseSnapshotSurfaces {
        train: snapshot.latest_train.clone(),
        analyze: snapshot.latest_analyze.clone(),
        research: snapshot.latest_research.clone(),
        backtest: snapshot.latest_backtest.clone(),
        update: snapshot.latest_update.clone(),
    }
}

pub fn humanize_workflow_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.is_empty() {
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
        .map(|phase| phase.phase_summary.clone())
        .unwrap_or_else(|| "尚无可用阶段摘要。".to_string());
    let latest_phase_summary_short = latest_phase
        .map(short_human_phase_summary)
        .unwrap_or_else(|| "尚无可用阶段摘要。".to_string());
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
    } else if snapshot.blocking_truth.status.is_empty() {
        "unblocked".to_string()
    } else {
        snapshot.blocking_truth.status.clone()
    };
    let gate_reason_label = if user_selection_pending {
        "user_selected_historical_data_missing".to_string()
    } else if hard_block_active && !snapshot.blocking_truth.reason.is_empty() {
        snapshot.blocking_truth.reason.clone()
    } else {
        "none".to_string()
    };
    let summary_line = format!(
        "{} | {} | {}",
        snapshot.symbol, snapshot.current_focus_phase, action_status_label
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
        serde_json::to_value(surface).expect("serialize ensemble vote surface")
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
        "summary_line": summary_line,
        "blocking_line": blocking_line,
        "next_action_line": next_action_line,
        "phase_summary_line": phase_summary_line,
        "symbol": snapshot.symbol,
        "current_status": {
            "focus_phase": snapshot.current_focus_phase,
            "focus_reason": snapshot.current_focus_reason,
            "blocking_stage": if historical_data_gate_active {
                snapshot.current_focus_phase.clone()
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
        comparable_to_previous: true,
        comparison_class: "same_data_different_config".to_string(),
        recommended_next_command: snapshot.recommended_next_command.clone(),
        realized_outcome: Some("win".to_string()),
        objective_market_credibility_shrink: None,
        ..crate::state::WorkflowPhaseSnapshot::default()
    });
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_workflow_status_view_exposes_candidates() {
        let snapshot = sample_human_workflow_snapshot();
        let value = build_human_workflow_status_view(&snapshot, &[]);
        assert_eq!(value["symbol"], "NQ");
        assert_eq!(value["current_status"]["focus_phase"], "update");
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
        assert_eq!(value["summary_line"], "NQ | update | action_blocked");
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
}
