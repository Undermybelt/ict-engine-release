use super::*;

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
