use super::*;
use ict_engine::application::provider_catalog::provider_status_agent_surface;
use ict_engine::state::ArtifactDecisionSummary;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OfflineStructuralSupportHintInput {
    pub baseline_support: f64,
    pub aggregate_return: Option<f64>,
    pub execution_readiness: Option<f64>,
    pub comparable_to_previous: bool,
    pub feedback_records_applied: usize,
    pub conformal_coverage_1sigma: Option<f64>,
    pub regime_break_penalty: Option<f64>,
    pub structural_break_detected: Option<bool>,
    pub best_factor_composite_score: Option<f64>,
    pub quality_delta: Option<f64>,
    pub score_before: Option<f64>,
    pub score_after: Option<f64>,
    pub baseline_available: Option<bool>,
    pub accepted: Option<bool>,
    pub artifact_validation_bias: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ResearchStructuralSupportInput {
    pub baseline_composite_score: Option<f64>,
    pub aggregate_return: f64,
    pub execution_readiness: Option<f64>,
    pub comparable_to_previous: bool,
    pub feedback_records_applied: usize,
    pub conformal_coverage_1sigma: Option<f64>,
    pub regime_break_penalty: Option<f64>,
    pub structural_break_detected: Option<bool>,
    pub quality_delta: Option<f64>,
    pub family_avg_score: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MutationStructuralSupportInput<'a> {
    pub baseline_composite_score: Option<f64>,
    pub aggregate_return: f64,
    pub execution_readiness: Option<f64>,
    pub comparable_to_previous: bool,
    pub feedback_records_applied: usize,
    pub conformal_coverage_1sigma: Option<f64>,
    pub regime_break_penalty: Option<f64>,
    pub structural_break_detected: Option<bool>,
    pub evaluation: &'a FactorMutationEvaluation,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BacktestStructuralSupportInput {
    pub baseline_composite_score: Option<f64>,
    pub aggregate_return: f64,
    pub execution_readiness: Option<f64>,
    pub comparable_to_previous: bool,
    pub feedback_records_applied: usize,
    pub conformal_coverage_1sigma: Option<f64>,
    pub regime_break_penalty: Option<f64>,
    pub structural_break_detected: Option<bool>,
    pub quality_delta: Option<f64>,
}

pub(crate) fn offline_structural_support_hint(input: OfflineStructuralSupportHintInput) -> f64 {
    let mut support = input.baseline_support.clamp(0.0, 1.0);
    if let Some(readiness) = input.execution_readiness {
        support = (support * 0.55 + readiness.clamp(0.0, 1.0) * 0.45).clamp(0.0, 1.0);
    }
    if let Some(aggregate_return) = input.aggregate_return {
        let return_bias = (aggregate_return * 4.0).clamp(-0.20, 0.20);
        support = (support + return_bias).clamp(0.0, 1.0);
    }
    if input.comparable_to_previous {
        support = (support + 0.05).clamp(0.0, 1.0);
    } else {
        support = (support - 0.03).clamp(0.0, 1.0);
    }
    if input.feedback_records_applied > 0 {
        let feedback_bias = (input.feedback_records_applied as f64 / 20.0).min(1.0) * 0.05;
        support = (support + feedback_bias).clamp(0.0, 1.0);
    }
    if let Some(coverage) = input.conformal_coverage_1sigma {
        let coverage_bias = ((coverage - 0.55) / 0.35).clamp(-0.15, 0.15);
        support = (support + coverage_bias).clamp(0.0, 1.0);
    }
    if let Some(regime_break_penalty) = input.regime_break_penalty {
        support = (support - regime_break_penalty.clamp(0.0, 0.25)).clamp(0.0, 1.0);
    }
    if matches!(input.structural_break_detected, Some(true)) {
        support = (support - 0.08).clamp(0.0, 1.0);
    }
    if let Some(score) = input.best_factor_composite_score {
        let score_bias = (score - 0.50).clamp(-0.10, 0.15);
        support = (support + score_bias).clamp(0.0, 1.0);
    }
    if let Some(score_delta) = input.quality_delta {
        support = (support + score_delta.clamp(-0.10, 0.10)).clamp(0.0, 1.0);
    }
    if let Some(artifact_validation_bias) = input.artifact_validation_bias {
        support = (support + artifact_validation_bias.clamp(-0.12, 0.12)).clamp(0.0, 1.0);
    }
    if let (Some(before), Some(after)) = (input.score_before, input.score_after) {
        support = (support + (after - before).clamp(-0.10, 0.10)).clamp(0.0, 1.0);
    }
    if let Some(baseline_available) = input.baseline_available {
        support = if baseline_available {
            (support + 0.03).clamp(0.0, 1.0)
        } else {
            (support - 0.03).clamp(0.0, 1.0)
        };
    }
    if let Some(accepted) = input.accepted {
        support = if accepted {
            (support + 0.08).clamp(0.0, 1.0)
        } else {
            (support - 0.08).clamp(0.0, 1.0)
        };
    }
    support
}

pub(crate) fn structural_baseline_support(score: Option<f64>, fallback: f64) -> f64 {
    score.unwrap_or(fallback).clamp(0.0, 1.0)
}

pub(crate) fn artifact_validation_support_bias(summary: &ArtifactDecisionSummary) -> f64 {
    let mut bias: f64 = match summary.consumed_trend_status.as_str() {
        "validated_positive" | "validated_improving" => 0.08,
        "validated_negative" | "validated_regressing" => -0.10,
        "validated_neutral" => -0.02,
        _ => 0.0,
    };
    bias += match summary.promotion_strength.as_str() {
        "high" => 0.03,
        "medium" => 0.01,
        _ => 0.0,
    };
    bias += match summary.rollback_strength.as_str() {
        "high" => -0.03,
        "medium" => -0.01,
        _ => 0.0,
    };
    bias.clamp(-0.12, 0.12)
}

pub(crate) fn structural_support_hint_for_analyze(
    posterior_confidence: f64,
    execution_readiness: Option<f64>,
    comparable_to_previous: bool,
    feedback_records_applied: usize,
) -> f64 {
    offline_structural_support_hint(OfflineStructuralSupportHintInput {
        baseline_support: structural_baseline_support(Some(posterior_confidence), 0.50),
        aggregate_return: None,
        execution_readiness,
        comparable_to_previous,
        feedback_records_applied,
        conformal_coverage_1sigma: None,
        regime_break_penalty: None,
        structural_break_detected: None,
        best_factor_composite_score: None,
        quality_delta: None,
        score_before: None,
        score_after: None,
        baseline_available: None,
        accepted: None,
        artifact_validation_bias: None,
    })
}

pub(crate) fn structural_support_hint_for_research(input: ResearchStructuralSupportInput) -> f64 {
    offline_structural_support_hint(OfflineStructuralSupportHintInput {
        baseline_support: structural_baseline_support(input.baseline_composite_score, 0.50),
        aggregate_return: Some(input.aggregate_return),
        execution_readiness: input.execution_readiness,
        comparable_to_previous: input.comparable_to_previous,
        feedback_records_applied: input.feedback_records_applied,
        conformal_coverage_1sigma: input.conformal_coverage_1sigma,
        regime_break_penalty: input.regime_break_penalty,
        structural_break_detected: input.structural_break_detected,
        best_factor_composite_score: input.family_avg_score.or(input.baseline_composite_score),
        quality_delta: input.quality_delta,
        score_before: None,
        score_after: None,
        baseline_available: None,
        accepted: None,
        artifact_validation_bias: None,
    })
}

pub(crate) fn structural_support_hint_for_mutation(
    input: MutationStructuralSupportInput<'_>,
) -> f64 {
    offline_structural_support_hint(OfflineStructuralSupportHintInput {
        baseline_support: structural_baseline_support(
            Some(input.evaluation.metrics_after.best_factor_composite_score)
                .or(input.baseline_composite_score),
            0.50,
        ),
        aggregate_return: Some(input.aggregate_return),
        execution_readiness: input.execution_readiness,
        comparable_to_previous: input.comparable_to_previous,
        feedback_records_applied: input.feedback_records_applied,
        conformal_coverage_1sigma: input.conformal_coverage_1sigma,
        regime_break_penalty: input.regime_break_penalty,
        structural_break_detected: input.structural_break_detected,
        best_factor_composite_score: Some(
            input.evaluation.metrics_after.best_factor_composite_score,
        ),
        quality_delta: Some(input.evaluation.score_delta),
        score_before: Some(input.evaluation.score_before),
        score_after: Some(input.evaluation.score_after),
        baseline_available: Some(input.evaluation.baseline_available),
        accepted: Some(input.evaluation.accepted),
        artifact_validation_bias: None,
    })
}

pub(crate) fn structural_support_hint_for_backtest(input: BacktestStructuralSupportInput) -> f64 {
    offline_structural_support_hint(OfflineStructuralSupportHintInput {
        baseline_support: structural_baseline_support(input.baseline_composite_score, 0.50),
        aggregate_return: Some(input.aggregate_return),
        execution_readiness: input.execution_readiness,
        comparable_to_previous: input.comparable_to_previous,
        feedback_records_applied: input.feedback_records_applied,
        conformal_coverage_1sigma: input.conformal_coverage_1sigma,
        regime_break_penalty: input.regime_break_penalty,
        structural_break_detected: input.structural_break_detected,
        best_factor_composite_score: input.baseline_composite_score,
        quality_delta: input.quality_delta,
        score_before: None,
        score_after: None,
        baseline_available: None,
        accepted: None,
        artifact_validation_bias: None,
    })
}

pub(crate) fn structural_prior_seed_from_support_hint(
    source_label: &str,
    support: f64,
) -> ict_engine::state::StructuralPriorSeed {
    let (observations, wins, breakevens, losses) = if support >= 0.75 {
        (3, 2, 1, 0)
    } else if support >= 0.60 {
        (2, 1, 1, 0)
    } else if support >= 0.50 {
        (1, 0, 1, 0)
    } else {
        (1, 0, 0, 1)
    };
    ict_engine::state::StructuralPriorSeed {
        source_label: source_label.to_string(),
        tempering_coefficient: Some(support.clamp(0.0, 1.0)),
        observations,
        followed_count: observations,
        wins,
        losses,
        breakevens,
        invalidated: 0,
        abandoned: 0,
        not_followed: 0,
        avg_pnl: (support - 0.5) * 0.04,
    }
}

pub(crate) fn apply_offline_structural_prior_seed(
    learning_state: &mut LearningState,
    snapshot: &WorkflowSnapshot,
    recommendation_id: &str,
    recommended_at: chrono::DateTime<chrono::Utc>,
    support_hint: f64,
    note: &str,
) {
    let provider_status_agent = provider_status_agent_surface(None, None, None).unwrap_or_default();
    if let Some(bundle) =
        ict_engine::application::orchestration::build_structural_recommended_path_bundle_artifact_with_prior_state(
            snapshot,
            &provider_status_agent,
            learning_state.feedback_history.as_slice(),
            &learning_state.structural_prior_state,
        )
    {
        let branch_id = bundle
            .scenario_id
            .strip_prefix("scenario:")
            .unwrap_or(bundle.scenario_id.as_str())
            .to_string();
        let node_id = branch_id
            .rsplit_once(':')
            .map(|(prefix, _)| prefix.to_string())
            .unwrap_or_else(|| branch_id.clone());
        let refs = ict_engine::state::StructuralFeedbackRefs {
            protocol_version: "structural-prior-seed-v1".to_string(),
            recommendation_id: recommendation_id.to_string(),
            recommended_at: recommended_at.to_rfc3339(),
            node_id,
            branch_id,
            scenario_id: bundle.scenario_id.clone(),
            path_id: bundle.path_id.clone(),
            followed_path: true,
            exit_reason: Some("offline_prior_seed".to_string()),
            notes: Some(note.to_string()),
        };
        let support = ((bundle.current_posterior + bundle.composite_score + support_hint) / 3.0)
            .clamp(0.0, 1.0);
        let seed = structural_prior_seed_from_support_hint(note, support);
        learning_state.apply_structural_prior_seed(&refs, &seed);
    }
}

pub(crate) fn persist_analyze_run(
    state_dir: &str,
    report: &AnalyzeReport,
    source_command: &str,
    data_htf_path: Option<&str>,
    data_mtf_path: Option<&str>,
    data_ltf_path: Option<&str>,
    live_data_source: Option<LiveDataSourceProvenance>,
) -> Result<WorkflowSnapshot> {
    let previous_policy = load_pre_bayes_policy_history(state_dir, &report.symbol)?
        .last()
        .map(|record| record.policy.clone());
    let analyze_run_record = AnalyzeRunRecord {
        run_id: format!(
            "{}:{}:{}",
            source_command,
            report.symbol,
            report.timestamp.format("%Y%m%dT%H%M%S%.3fZ")
        ),
        timestamp: report.timestamp,
        symbol: report.symbol.clone(),
        provenance: report.supporting.provenance.clone(),
        decision_thresholds: report.supporting.decision_thresholds.clone(),
        dataset_comparability: report.supporting.dataset_comparability.clone(),
        promotion_decision: report.supporting.promotion_decision.clone(),
        rollback_recommendation: report.supporting.rollback_recommendation.clone(),
        family_history_window: family_history_window(),
        source_command: source_command.to_string(),
        data_htf_path: data_htf_path.map(str::to_string),
        data_mtf_path: data_mtf_path.map(str::to_string),
        data_ltf_path: data_ltf_path.map(str::to_string),
        live_data_source,
        htf_bars: report.meta.bars.htf,
        mtf_bars: report.meta.bars.mtf,
        ltf_bars: report.meta.bars.ltf,
        selected_direction: report.supporting.decision.selected_direction,
        selected_entry_quality: report.supporting.entry_quality.selected_state.clone(),
        decision_hint: report.supporting.decision_hint.clone(),
        regime_probs: Some(report.supporting.model_state.regime_probs),
        market_state_evidence: report.supporting.market_state_evidence.clone(),
        canonical_structural_regime_posterior: Some(
            ict_engine::state::CanonicalStructuralRegimePosterior {
                active_regime: report
                    .supporting
                    .canonical_belief_report
                    .regime_posterior
                    .active_regime
                    .clone(),
                confidence: report
                    .supporting
                    .canonical_belief_report
                    .regime_posterior
                    .confidence,
                probabilities: report
                    .supporting
                    .canonical_belief_report
                    .regime_posterior
                    .probabilities
                    .clone(),
                evidence: report
                    .supporting
                    .canonical_belief_report
                    .regime_posterior
                    .evidence
                    .clone(),
            },
        ),
        hybrid_regime_label: report.analysis.regime_bayesian.hybrid_regime_label.clone(),
        hybrid_regime_age_bars: report
            .supporting
            .decision_hint
            .split('|')
            .find_map(|part| part.strip_prefix("hybrid_regime_age="))
            .and_then(|value| value.parse::<usize>().ok()),
        hybrid_duration_model: report
            .analysis
            .regime_bayesian
            .hybrid_duration_model
            .clone(),
        hybrid_remaining_expected_bars: report
            .analysis
            .regime_bayesian
            .hybrid_remaining_expected_bars,
        execution_artifact_id: report
            .supporting
            .execution_artifact
            .as_ref()
            .map(|artifact| artifact.artifact_id.clone()),
        execution_edge_share: report
            .supporting
            .execution_artifact
            .as_ref()
            .map(|artifact| artifact.features.execution_edge_share),
        prediction_edge_share: report
            .supporting
            .execution_artifact
            .as_ref()
            .map(|artifact| artifact.features.prediction_edge_share),
        execution_readiness: report
            .supporting
            .execution_artifact
            .as_ref()
            .map(|artifact| artifact.features.execution_readiness),
        execution_gate_status: report
            .supporting
            .execution_artifact
            .as_ref()
            .map(|artifact| artifact.hard_gate_status.clone()),
        entry_model_packets: report.supporting.entry_model_packets.clone(),
        pre_bayes_evidence_filter: report.supporting.pre_bayes_evidence_filter.clone(),
        pre_bayes_entry_quality_bridge: report.supporting.pre_bayes_entry_quality_bridge.clone(),
        factor_family_decisions: report.supporting.factor_family_decisions.clone(),
        factor_family_outcomes: report.supporting.factor_family_outcomes.clone(),
        factor_family_diffs: report.supporting.factor_family_diffs.clone(),
        factor_family_history: report.supporting.factor_family_history.clone(),
        decision_history_summary: report.supporting.decision_history_summary.clone(),
        workflow_state: report.supporting.workflow_state.clone(),
        agent_action_plan: report.supporting.agent_action_plan.clone(),
        recommended_commands: report.supporting.recommended_commands.clone(),
        recommended_next_command: report.supporting.recommended_next_command.clone(),
        recommended_next_command_meta: recommended_next_command_meta(
            &report.supporting.recommended_next_command,
        ),
        agent_context_bundle: report.supporting.agent_context_bundle.clone(),
        agent_context_bundle_minimal: report.supporting.agent_context_bundle_minimal.clone(),
        feedback_history_summary: report.supporting.feedback_history_summary.clone(),
        multi_timeframe_summary: report.supporting.multi_timeframe_summary.clone(),
        artifact_action_summary: report.supporting.artifact_action_summary.clone(),
        artifact_decision_summary: report.supporting.artifact_decision_summary.clone(),
        artifact_decision_section: report.supporting.artifact_decision_section.clone(),
        agent_prompts: report.supporting.agent_prompts.clone(),
        prompt_workflow: report.supporting.agent_prompts.workflow.clone(),
    };
    append_analyze_run(state_dir, &report.symbol, analyze_run_record.clone())?;
    let mut learning_state = load_learning_state(state_dir, &report.symbol).unwrap_or_default();
    let blocking_truth = report.supporting.workflow_snapshot.blocking_truth.clone();
    let hard_blocked = matches!(
        blocking_truth.status.as_str(),
        "blocked"
            | "bridge_needs_confirmation"
            | "validated_regressing"
            | "credibility_gate_blocked"
    );
    let analyze_ensemble_vote = build_stub_ensemble_vote_from_input(&AnalyzeEnsembleVoteInput {
        symbol: report.symbol.clone(),
        state_dir: Some(state_dir.to_string()),
        recommended_next_command: report.supporting.recommended_next_command.clone(),
        hard_blocked,
        hard_block_reason: if hard_blocked {
            Some(blocking_truth.reason.clone())
        } else {
            None
        },
        hard_block_command: if hard_blocked {
            Some(blocking_truth.next_command.clone())
        } else {
            None
        },
        provenance: report.supporting.provenance.clone(),
        dataset_comparability: report.supporting.dataset_comparability.clone(),
        pre_bayes_filter: Some(report.supporting.pre_bayes_evidence_filter.clone()),
        belief: report.supporting.canonical_belief_report.clone(),
        ict_structure: None,
    });
    let canonical_scorecards =
        load_ensemble_executor_scorecards(state_dir, &report.symbol).unwrap_or_default();
    let analyze_ensemble_record = build_ensemble_vote_record(
        &report.symbol,
        source_command,
        Some(analyze_run_record.run_id.clone()),
        &report.supporting.provenance,
        &report.supporting.dataset_comparability,
        &analyze_ensemble_vote,
        &canonical_scorecards,
    );
    let structural_snapshot = WorkflowSnapshot {
        symbol: report.symbol.clone(),
        current_focus_phase: "analyze".to_string(),
        current_focus_reason: report.supporting.workflow_state.reason.clone(),
        recommended_next_command: report.supporting.recommended_next_command.clone(),
        recommended_next_command_meta: recommended_next_command_meta(
            &report.supporting.recommended_next_command,
        ),
        blocking_truth: report.supporting.workflow_snapshot.blocking_truth.clone(),
        latest_analyze: Some(workflow_phase_snapshot_from_analyze_run(
            &analyze_run_record,
        )),
        latest_ensemble_vote: Some(analyze_ensemble_record.clone()),
        ..WorkflowSnapshot::default()
    };
    apply_offline_structural_prior_seed(
        &mut learning_state,
        &structural_snapshot,
        &format!("structural-prior-seed:{}", analyze_run_record.run_id),
        analyze_run_record.timestamp,
        structural_support_hint_for_analyze(
            analyze_ensemble_record
                .posterior_confidence
                .unwrap_or(analyze_ensemble_record.confidence),
            analyze_run_record.execution_readiness,
            analyze_run_record.dataset_comparability.comparable,
            analyze_run_record.feedback_history_summary.total_records,
        ),
        "analyze_run_structural_prior_seed",
    );
    save_learning_state(state_dir, &report.symbol, &learning_state)?;
    persist_ensemble_vote_record(state_dir, &analyze_ensemble_record, &canonical_scorecards)?;
    append_pre_bayes_policy_history(
        state_dir,
        &report.symbol,
        PreBayesPolicyRecord {
            timestamp: report.timestamp,
            run_id: format!(
                "{}:{}:{}",
                source_command,
                report.symbol,
                report.timestamp.format("%Y%m%dT%H%M%S%.3fZ")
            ),
            source_command: source_command.to_string(),
            policy: report.supporting.pre_bayes_evidence_filter.policy.clone(),
            diff_from_previous: pre_bayes_policy_diff(
                previous_policy.as_ref(),
                &report.supporting.pre_bayes_evidence_filter.policy,
            ),
        },
    )?;
    refresh_workflow_snapshot(state_dir, &report.symbol)
}

pub(crate) fn persist_pending_update_artifact_from_analyze(
    state_dir: &str,
    report: &AnalyzeReport,
    source_phase: &str,
) -> Result<String> {
    let rules = artifact_review_rules().pending_update;
    let review_rule_version = pending_update_review_rule_version(&rules);
    let history = load_pending_update_history(state_dir, &report.symbol)?;
    let version = history.len() + 1;
    let top_factor_score = report
        .supporting
        .factor_ranking
        .first()
        .map(|item| item.composite_score)
        .unwrap_or(0.0);
    let avg_family_score = if report.supporting.factor_family_decisions.is_empty() {
        0.0
    } else {
        report
            .supporting
            .factor_family_decisions
            .iter()
            .map(|family| family.avg_score)
            .sum::<f64>()
            / report.supporting.factor_family_decisions.len() as f64
    };
    let template_feedback = FeedbackRecord {
        prompt_version: Some(report.supporting.provenance.prompt_version.clone()),
        factor_version: Some(report.supporting.provenance.factor_version.clone()),
        data_fingerprint: Some(report.supporting.provenance.data_fingerprint.clone()),
        ..build_feedback_record(BuildFeedbackRecordInput {
            symbol: &report.symbol,
            source: source_phase,
            timestamp: report.timestamp,
            factor_diagnostics: &report.supporting.factor_diagnostics,
            decision: &report.supporting.decision,
            pnl: 0.0,
            realized_outcome: "pending".to_string(),
            regime_at_entry: report.supporting.model_state.regime_probs.dominant(),
        })
    };
    let mut artifact = PendingUpdateArtifact {
        artifact_id: format!(
            "pending-update:{}:{}:v{}",
            report.symbol, source_phase, version
        ),
        version,
        generated_at: report.timestamp,
        symbol: report.symbol.clone(),
        source_phase: source_phase.to_string(),
        source_run_id: Some(format!(
            "{}:{}:{}",
            source_phase,
            report.symbol,
            report.timestamp.format("%Y%m%dT%H%M%S%.3fZ")
        )),
        source_command: source_phase.to_string(),
        provenance: report.supporting.provenance.clone(),
        decision_hint: report.supporting.decision_hint.clone(),
        entry_quality: report.supporting.entry_quality.selected_state.clone(),
        factor_alignment: report.supporting.factor_diagnostics.alignment_label.clone(),
        factor_uncertainty: report
            .supporting
            .factor_diagnostics
            .uncertainty_label
            .clone(),
        selected_win_probability: report.supporting.decision.selected_win_probability,
        top_factor_score,
        avg_family_score,
        top_factor_name: report
            .supporting
            .factor_ranking
            .first()
            .map(|item| item.factor_name.clone()),
        top_factor_action: report
            .supporting
            .factor_ranking
            .first()
            .map(|item| item.iteration_action.clone()),
        family_scores: report
            .supporting
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        review_rule_version,
        review_rule_snapshot: rules,
        pre_bayes_evidence_filter: Some(report.supporting.pre_bayes_evidence_filter.clone()),
        pre_bayes_entry_quality_bridge: Some(
            report.supporting.pre_bayes_entry_quality_bridge.clone(),
        ),
        multi_timeframe_summary: report.supporting.multi_timeframe_summary.clone(),
        template_feedback,
        diff_from_previous: PendingUpdateArtifactDiff::default(),
        review_decision: PendingUpdateArtifactDecision::default(),
    };
    if let Some(previous) = history.last() {
        artifact.diff_from_previous = pending_update_artifact_diff(previous, &artifact);
        artifact.review_decision = pending_update_artifact_decision(previous, &artifact);
    } else {
        artifact.review_decision = PendingUpdateArtifactDecision {
            status: "promote_latest".to_string(),
            reason: "first_pending_update_artifact".to_string(),
            supersedes_artifact_id: None,
        };
    }
    append_artifact_ledger_entry(
        state_dir,
        &report.symbol,
        artifact_ledger_entry_from_pending_update(state_dir, &report.symbol, &artifact),
    )?;
    save_pending_update_artifact(state_dir, &report.symbol, &artifact)?;
    append_pending_update_artifact_history(state_dir, &report.symbol, artifact)?;
    Ok(std::path::Path::new(state_dir)
        .join(&report.symbol)
        .join(PENDING_UPDATE_ARTIFACT_FILE)
        .to_string_lossy()
        .to_string())
}

fn pending_update_artifact_decision(
    previous: &PendingUpdateArtifact,
    current: &PendingUpdateArtifact,
) -> PendingUpdateArtifactDecision {
    let rules = artifact_review_rules().pending_update;

    if current.diff_from_previous.exact_duplicate {
        PendingUpdateArtifactDecision {
            status: "discard".to_string(),
            reason: "duplicate_pending_update_context".to_string(),
            supersedes_artifact_id: None,
        }
    } else if (rules.require_same_data && !current.diff_from_previous.comparable_same_data)
        || (rules.require_same_factor_version
            && !current.diff_from_previous.comparable_same_factor_version)
        || (rules.require_same_prompt_version
            && !current.diff_from_previous.comparable_same_prompt_version)
    {
        PendingUpdateArtifactDecision {
            status: "observe".to_string(),
            reason: "artifact_not_comparable_same_data_factor_prompt_required".to_string(),
            supersedes_artifact_id: None,
        }
    } else if current.diff_from_previous.selected_probability_delta
        <= -rules.min_probability_improvement
        || current.diff_from_previous.top_factor_score_delta
            <= -rules.min_top_factor_score_improvement
        || current.diff_from_previous.avg_family_score_delta
            <= -rules.min_avg_family_score_improvement
    {
        PendingUpdateArtifactDecision {
            status: "discard".to_string(),
            reason: "strict_probability_or_score_regression".to_string(),
            supersedes_artifact_id: None,
        }
    } else if current.diff_from_previous.selected_probability_delta
        >= rules.min_probability_improvement
        && (current.diff_from_previous.top_factor_score_delta
            >= rules.min_top_factor_score_improvement
            || current.diff_from_previous.avg_family_score_delta
                >= rules.min_avg_family_score_improvement)
    {
        PendingUpdateArtifactDecision {
            status: "promote_latest".to_string(),
            reason: "strict_probability_and_score_improvement".to_string(),
            supersedes_artifact_id: Some(previous.artifact_id.clone()),
        }
    } else {
        PendingUpdateArtifactDecision {
            status: "observe".to_string(),
            reason: "within_probability_score_threshold_band".to_string(),
            supersedes_artifact_id: None,
        }
    }
}

fn artifact_ledger_entry_from_pending_update(
    state_dir: &str,
    symbol: &str,
    artifact: &PendingUpdateArtifact,
) -> ArtifactLedgerEntry {
    ArtifactLedgerEntry {
        entry_id: format!("ledger:{}", artifact.artifact_id),
        artifact_kind: "pending_update".to_string(),
        artifact_id: artifact.artifact_id.clone(),
        version: artifact.version,
        generated_at: artifact.generated_at,
        symbol: artifact.symbol.clone(),
        source_phase: artifact.source_phase.clone(),
        source_run_id: artifact.source_run_id.clone(),
        path: std::path::Path::new(state_dir)
            .join(symbol)
            .join(PENDING_UPDATE_ARTIFACT_FILE)
            .to_string_lossy()
            .to_string(),
        status: artifact.review_decision.status.clone(),
        promote_candidate: artifact.review_decision.status == "promote_latest",
        actionable: artifact.review_decision.status != "discard",
        decision_hint: artifact.decision_hint.clone(),
        review_reason: artifact.review_decision.reason.clone(),
        review_rule_version: artifact.review_rule_version.clone(),
        top_factor_name: artifact.top_factor_name.clone(),
        top_factor_action: artifact.top_factor_action.clone(),
        family_scores: artifact.family_scores.clone(),
        supersedes_artifact_id: artifact.review_decision.supersedes_artifact_id.clone(),
        quality_score: pending_update_quality_score(artifact),
        consumed_by_update_run_id: None,
        consumed_at: None,
        consumed_outcome: None,
        regraded_at: None,
        consumption_regrade_status: None,
        consumption_regrade_reason: None,
    }
}

fn execution_candidate_artifact_decision(
    previous: &ExecutionCandidateArtifact,
    current: &ExecutionCandidateArtifact,
) -> ExecutionCandidateArtifactDecision {
    let rules = artifact_review_rules().execution_candidate;

    if current.diff_from_previous.exact_duplicate {
        ExecutionCandidateArtifactDecision {
            status: "discard".to_string(),
            reason: "duplicate_execution_candidate_context".to_string(),
            supersedes_artifact_id: None,
        }
    } else if !current.actionable {
        ExecutionCandidateArtifactDecision {
            status: "observe".to_string(),
            reason: "candidate_not_actionable".to_string(),
            supersedes_artifact_id: None,
        }
    } else if (rules.require_same_data
        && previous.provenance.data_fingerprint != current.provenance.data_fingerprint)
        || (rules.require_same_factor_version
            && previous.provenance.factor_version != current.provenance.factor_version)
    {
        ExecutionCandidateArtifactDecision {
            status: "observe".to_string(),
            reason: "candidate_not_comparable_same_data_factor_required".to_string(),
            supersedes_artifact_id: None,
        }
    } else if current.diff_from_previous.posterior_delta <= -rules.min_posterior_improvement
        || current.diff_from_previous.win_probability_delta
            <= -rules.min_win_probability_improvement
    {
        ExecutionCandidateArtifactDecision {
            status: "discard".to_string(),
            reason: "candidate_probability_regression".to_string(),
            supersedes_artifact_id: None,
        }
    } else if current.diff_from_previous.posterior_delta >= rules.min_posterior_improvement
        && current.diff_from_previous.win_probability_delta >= rules.min_win_probability_improvement
    {
        ExecutionCandidateArtifactDecision {
            status: "promote_latest".to_string(),
            reason: "candidate_probability_improvement".to_string(),
            supersedes_artifact_id: Some(previous.artifact_id.clone()),
        }
    } else {
        ExecutionCandidateArtifactDecision {
            status: "observe".to_string(),
            reason: "candidate_within_probability_threshold_band".to_string(),
            supersedes_artifact_id: None,
        }
    }
}

pub(crate) fn persist_execution_candidate_from_analyze(
    state_dir: &str,
    report: &AnalyzeReport,
    source_phase: &str,
) -> Result<String> {
    let rules = artifact_review_rules().execution_candidate;
    let review_rule_version = execution_candidate_review_rule_version(&rules);
    let history = load_execution_candidate_history(state_dir, &report.symbol)?;
    let version = history.len() + 1;
    let trade_plan = &report.supporting.raw_trade_plan;
    let artifact = ExecutionCandidateArtifact {
        artifact_id: format!(
            "execution-candidate:{}:{}:v{}",
            report.symbol, source_phase, version
        ),
        version,
        generated_at: report.timestamp,
        symbol: report.symbol.clone(),
        source_phase: source_phase.to_string(),
        source_run_id: Some(format!(
            "{}:{}:{}",
            source_phase,
            report.symbol,
            report.timestamp.format("%Y%m%dT%H%M%S%.3fZ")
        )),
        provenance: report.supporting.provenance.clone(),
        decision_hint: report.supporting.decision_hint.clone(),
        selected_direction: report.supporting.decision.selected_direction,
        trade_direction: trade_plan.direction,
        actionable: trade_plan.direction != Direction::Neutral && trade_plan.position_size > 0.0,
        entry: trade_plan.entry,
        stop_loss: trade_plan.stop_loss,
        take_profits: vec![trade_plan.tp1, trade_plan.tp2, trade_plan.tp3],
        posterior: trade_plan.posterior,
        win_probability: trade_plan.win_probability,
        factor_alignment: report.supporting.factor_diagnostics.alignment_label.clone(),
        factor_uncertainty: report
            .supporting
            .factor_diagnostics
            .uncertainty_label
            .clone(),
        candidate_status: if trade_plan.direction != Direction::Neutral
            && trade_plan.position_size > 0.0
        {
            "ready".to_string()
        } else {
            "no_trade".to_string()
        },
        top_factor_name: report
            .supporting
            .factor_ranking
            .first()
            .map(|item| item.factor_name.clone()),
        top_factor_action: report
            .supporting
            .factor_ranking
            .first()
            .map(|item| item.iteration_action.clone()),
        family_scores: report
            .supporting
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        review_rule_version,
        review_rule_snapshot: rules,
        pre_bayes_evidence_filter: Some(report.supporting.pre_bayes_evidence_filter.clone()),
        pre_bayes_entry_quality_bridge: Some(
            report.supporting.pre_bayes_entry_quality_bridge.clone(),
        ),
        multi_timeframe_summary: report.supporting.multi_timeframe_summary.clone(),
        executor_scorecards: Vec::new(),
        diff_from_previous: ExecutionCandidateArtifactDiff::default(),
        review_decision: ExecutionCandidateArtifactDecision::default(),
    };
    let mut artifact = artifact;
    if let Some(previous) = history.last() {
        artifact.diff_from_previous = execution_candidate_artifact_diff(previous, &artifact);
        artifact.review_decision = execution_candidate_artifact_decision(previous, &artifact);
    } else {
        artifact.review_decision = ExecutionCandidateArtifactDecision {
            status: if artifact.actionable {
                "promote_latest".to_string()
            } else {
                "observe".to_string()
            },
            reason: "first_execution_candidate_artifact".to_string(),
            supersedes_artifact_id: None,
        };
    }
    append_artifact_ledger_entry(
        state_dir,
        &report.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "execution_candidate".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: artifact.version,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: artifact.source_phase.clone(),
            source_run_id: artifact.source_run_id.clone(),
            path: std::path::Path::new(state_dir)
                .join(&report.symbol)
                .join(EXECUTION_CANDIDATE_FILE)
                .to_string_lossy()
                .to_string(),
            status: artifact.review_decision.status.clone(),
            promote_candidate: artifact.review_decision.status == "promote_latest",
            actionable: artifact.actionable && artifact.review_decision.status != "discard",
            decision_hint: artifact.decision_hint.clone(),
            review_reason: artifact.review_decision.reason.clone(),
            review_rule_version: artifact.review_rule_version.clone(),
            top_factor_name: artifact.top_factor_name.clone(),
            top_factor_action: artifact.top_factor_action.clone(),
            family_scores: artifact.family_scores.clone(),
            supersedes_artifact_id: artifact.review_decision.supersedes_artifact_id.clone(),
            quality_score: ((artifact.posterior + artifact.win_probability) * 100.0) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    save_execution_candidate_artifact(state_dir, &report.symbol, &artifact)?;
    append_execution_candidate_history(state_dir, &report.symbol, artifact)?;
    Ok(std::path::Path::new(state_dir)
        .join(&report.symbol)
        .join(EXECUTION_CANDIDATE_FILE)
        .to_string_lossy()
        .to_string())
}

pub(crate) fn apply_command_context_to_analyze_report(
    report: &mut AnalyzeReport,
    command_context: &CommandContext,
) {
    report.supporting.recommended_commands = command_recommendations(command_context);
    concretize_action_plan_commands(
        &mut report.supporting.agent_action_plan,
        &report.supporting.recommended_commands,
    );
    report.supporting.recommended_next_command = recommended_next_command(
        &report.supporting.agent_action_plan,
        &report.supporting.recommended_commands,
    );
    report.supporting.agent_context_bundle =
        build_agent_context_bundle(BuildAgentContextBundleInput {
            symbol: &command_context.symbol,
            state_dir: &command_context.state_dir,
            workflow_state: &report.supporting.workflow_state,
            decision_hint: &report.supporting.decision_hint,
            recommended_next_command: &report.supporting.recommended_next_command,
            recommended_commands: &report.supporting.recommended_commands,
            dataset_comparability: &report.supporting.dataset_comparability,
            factor_iteration_queue: &report.supporting.factor_iteration_queue,
            family_outcomes: &report.supporting.factor_family_outcomes,
            pre_bayes_evidence_filter: Some(&report.supporting.pre_bayes_evidence_filter),
            pre_bayes_entry_quality_bridge: Some(&report.supporting.pre_bayes_entry_quality_bridge),
            pda_sequence_summary: None,
            factor_mutation_evaluation: None,
            artifact_decision_summary: Some(&report.supporting.artifact_decision_summary),
        });
    report
        .supporting
        .agent_context_bundle
        .multi_timeframe_summary = report.supporting.multi_timeframe_summary.clone();
    report.supporting.agent_context_bundle_minimal =
        build_agent_context_bundle_minimal(&report.supporting.agent_context_bundle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_structural_support_hint_prefers_positive_comparable_high_readiness_runs() {
        let weak = offline_structural_support_hint(OfflineStructuralSupportHintInput {
            baseline_support: 0.50,
            aggregate_return: Some(-0.01),
            execution_readiness: Some(0.42),
            comparable_to_previous: false,
            feedback_records_applied: 0,
            conformal_coverage_1sigma: None,
            regime_break_penalty: None,
            structural_break_detected: None,
            best_factor_composite_score: None,
            quality_delta: Some(-0.04),
            score_before: None,
            score_after: None,
            baseline_available: None,
            accepted: None,
            artifact_validation_bias: None,
        });
        let strong = offline_structural_support_hint(OfflineStructuralSupportHintInput {
            baseline_support: 0.50,
            aggregate_return: Some(0.04),
            execution_readiness: Some(0.83),
            comparable_to_previous: true,
            feedback_records_applied: 8,
            conformal_coverage_1sigma: None,
            regime_break_penalty: None,
            structural_break_detected: None,
            best_factor_composite_score: None,
            quality_delta: Some(0.06),
            score_before: None,
            score_after: None,
            baseline_available: None,
            accepted: None,
            artifact_validation_bias: None,
        });

        assert!(strong > weak);
        assert!(strong > 0.60);
    }

    #[test]
    fn test_offline_structural_support_hint_rewards_accepted_mutation() {
        let rejected = offline_structural_support_hint(OfflineStructuralSupportHintInput {
            baseline_support: 0.55,
            aggregate_return: None,
            execution_readiness: Some(0.60),
            comparable_to_previous: true,
            feedback_records_applied: 0,
            conformal_coverage_1sigma: None,
            regime_break_penalty: None,
            structural_break_detected: None,
            best_factor_composite_score: None,
            quality_delta: Some(-0.02),
            score_before: Some(0.52),
            score_after: Some(0.50),
            baseline_available: Some(true),
            accepted: Some(false),
            artifact_validation_bias: None,
        });
        let accepted = offline_structural_support_hint(OfflineStructuralSupportHintInput {
            baseline_support: 0.55,
            aggregate_return: None,
            execution_readiness: Some(0.60),
            comparable_to_previous: true,
            feedback_records_applied: 0,
            conformal_coverage_1sigma: None,
            regime_break_penalty: None,
            structural_break_detected: None,
            best_factor_composite_score: None,
            quality_delta: Some(0.02),
            score_before: Some(0.52),
            score_after: Some(0.58),
            baseline_available: Some(true),
            accepted: Some(true),
            artifact_validation_bias: None,
        });

        assert!(accepted > rejected);
    }

    #[test]
    fn test_structural_baseline_support_prefers_best_factor_score() {
        assert_eq!(structural_baseline_support(Some(0.78), 0.50), 0.78);
        assert_eq!(structural_baseline_support(None, 0.50), 0.50);
        assert_eq!(structural_baseline_support(Some(1.40), 0.50), 1.0);
        assert_eq!(structural_baseline_support(Some(-0.20), 0.50), 0.0);
    }

    #[test]
    fn test_artifact_validation_support_bias_penalizes_regression() {
        let positive = artifact_validation_support_bias(&ArtifactDecisionSummary {
            consumed_trend_status: "validated_positive".to_string(),
            promotion_strength: "high".to_string(),
            rollback_strength: "low".to_string(),
            ..ArtifactDecisionSummary::default()
        });
        let regressing = artifact_validation_support_bias(&ArtifactDecisionSummary {
            consumed_trend_status: "validated_regressing".to_string(),
            promotion_strength: "low".to_string(),
            rollback_strength: "high".to_string(),
            ..ArtifactDecisionSummary::default()
        });

        assert!(positive > regressing);
        assert!(regressing < 0.0);
    }

    #[test]
    fn test_structural_support_hint_for_research_uses_family_quality() {
        let low = structural_support_hint_for_research(ResearchStructuralSupportInput {
            baseline_composite_score: Some(0.58),
            aggregate_return: 0.01,
            execution_readiness: Some(0.60),
            comparable_to_previous: true,
            feedback_records_applied: 1,
            conformal_coverage_1sigma: Some(0.65),
            regime_break_penalty: Some(0.08),
            structural_break_detected: Some(false),
            quality_delta: Some(0.01),
            family_avg_score: Some(0.42),
        });
        let high = structural_support_hint_for_research(ResearchStructuralSupportInput {
            family_avg_score: Some(0.76),
            ..ResearchStructuralSupportInput {
                baseline_composite_score: Some(0.58),
                aggregate_return: 0.01,
                execution_readiness: Some(0.60),
                comparable_to_previous: true,
                feedback_records_applied: 1,
                conformal_coverage_1sigma: Some(0.65),
                regime_break_penalty: Some(0.08),
                structural_break_detected: Some(false),
                quality_delta: Some(0.01),
                family_avg_score: Some(0.42),
            }
        });

        assert!(high > low);
    }

    #[test]
    fn test_structural_support_hint_for_backtest_penalizes_breaks() {
        let low = structural_support_hint_for_backtest(BacktestStructuralSupportInput {
            baseline_composite_score: Some(0.68),
            aggregate_return: 0.02,
            execution_readiness: Some(0.70),
            comparable_to_previous: true,
            feedback_records_applied: 1,
            conformal_coverage_1sigma: Some(0.48),
            regime_break_penalty: Some(0.22),
            structural_break_detected: Some(true),
            quality_delta: Some(-0.02),
        });
        let high = structural_support_hint_for_backtest(BacktestStructuralSupportInput {
            baseline_composite_score: Some(0.68),
            aggregate_return: 0.02,
            execution_readiness: Some(0.70),
            comparable_to_previous: true,
            feedback_records_applied: 1,
            conformal_coverage_1sigma: Some(0.82),
            regime_break_penalty: Some(0.04),
            structural_break_detected: Some(false),
            quality_delta: Some(0.02),
        });

        assert!(high > low);
    }

    #[test]
    fn test_offline_structural_support_hint_penalizes_breaks_and_rewards_coverage() {
        let poor = offline_structural_support_hint(OfflineStructuralSupportHintInput {
            baseline_support: 0.55,
            aggregate_return: Some(0.01),
            execution_readiness: Some(0.62),
            comparable_to_previous: true,
            feedback_records_applied: 2,
            conformal_coverage_1sigma: Some(0.42),
            regime_break_penalty: Some(0.24),
            structural_break_detected: Some(true),
            best_factor_composite_score: Some(0.48),
            quality_delta: Some(-0.03),
            score_before: None,
            score_after: None,
            baseline_available: None,
            accepted: None,
            artifact_validation_bias: None,
        });
        let good = offline_structural_support_hint(OfflineStructuralSupportHintInput {
            baseline_support: 0.55,
            aggregate_return: Some(0.03),
            execution_readiness: Some(0.76),
            comparable_to_previous: true,
            feedback_records_applied: 6,
            conformal_coverage_1sigma: Some(0.81),
            regime_break_penalty: Some(0.04),
            structural_break_detected: Some(false),
            best_factor_composite_score: Some(0.74),
            quality_delta: Some(0.05),
            score_before: None,
            score_after: None,
            baseline_available: None,
            accepted: None,
            artifact_validation_bias: None,
        });

        assert!(good > poor);
        assert!(good > 0.65);
    }

    #[test]
    fn test_offline_structural_support_hint_rewards_baseline_available_and_score_improvement() {
        let weak = offline_structural_support_hint(OfflineStructuralSupportHintInput {
            baseline_support: 0.58,
            aggregate_return: Some(0.01),
            execution_readiness: Some(0.60),
            comparable_to_previous: true,
            feedback_records_applied: 1,
            conformal_coverage_1sigma: Some(0.66),
            regime_break_penalty: Some(0.08),
            structural_break_detected: Some(false),
            best_factor_composite_score: Some(0.62),
            quality_delta: Some(0.00),
            score_before: Some(0.62),
            score_after: Some(0.62),
            baseline_available: Some(false),
            accepted: None,
            artifact_validation_bias: None,
        });
        let strong = offline_structural_support_hint(OfflineStructuralSupportHintInput {
            baseline_support: 0.58,
            aggregate_return: Some(0.01),
            execution_readiness: Some(0.60),
            comparable_to_previous: true,
            feedback_records_applied: 1,
            conformal_coverage_1sigma: Some(0.66),
            regime_break_penalty: Some(0.08),
            structural_break_detected: Some(false),
            best_factor_composite_score: Some(0.62),
            quality_delta: Some(0.06),
            score_before: Some(0.62),
            score_after: Some(0.71),
            baseline_available: Some(true),
            accepted: None,
            artifact_validation_bias: None,
        });

        assert!(strong > weak);
    }
}
