use super::*;
use ict_engine::application::entry_models::export_policy_training_tables;
use ict_engine::application::orchestration::export_structural_path_ranking_target;
use ict_engine::application::provider_catalog::provider_status_agent_surface;

fn structural_prior_seed_from_artifact_validation(
    summary: &ict_engine::state::ArtifactDecisionSummary,
) -> Option<ict_engine::state::StructuralPriorSeed> {
    let (observations, wins, breakevens, losses, avg_pnl) =
        match summary.consumed_trend_status.as_str() {
            "validated_positive" | "validated_improving" => (2, 2, 0, 0, 0.03),
            "validated_negative" | "validated_regressing" => (2, 0, 0, 2, -0.03),
            "validated_neutral" => (1, 0, 1, 0, 0.0),
            _ => return None,
        };
    Some(ict_engine::state::StructuralPriorSeed {
        source_label: "artifact_validation".to_string(),
        tempering_coefficient: Some(1.0),
        observations,
        followed_count: observations,
        wins,
        losses,
        breakevens,
        invalidated: 0,
        abandoned: 0,
        not_followed: 0,
        avg_pnl,
    })
}

fn append_learning_semantics_to_family_outcomes(
    outcomes: &mut [FactorFamilyOutcome],
    semantics_summary: &str,
) {
    if semantics_summary.is_empty() {
        return;
    }
    for outcome in outcomes {
        if !outcome
            .promotion_decision
            .reason
            .contains("learning_semantics=")
        {
            outcome.promotion_decision.reason = format!(
                "{}|learning_semantics={}",
                outcome.promotion_decision.reason, semantics_summary
            );
        }
        if !outcome
            .rollback_recommendation
            .reason
            .contains("learning_semantics=")
        {
            outcome.rollback_recommendation.reason = format!(
                "{}|learning_semantics={}",
                outcome.rollback_recommendation.reason, semantics_summary
            );
        }
    }
}

fn append_learning_semantics_to_update_gate_prompts(
    prompts: &mut ict_engine::agent::AgentPromptPack,
    semantics_summary: &str,
) {
    if semantics_summary.is_empty() {
        return;
    }
    for prompt in &mut prompts.prompts {
        if prompt.id == "promotion_gate" || prompt.id == "rollback_review" {
            if !prompt.user_prompt.contains("learning_semantics=") {
                prompt.user_prompt = format!(
                    "{} learning_semantics={}",
                    prompt.user_prompt, semantics_summary
                );
            }
            let criterion = "Use learning_semantics to distinguish full-credit, fractional-credit, and no-credit feedback before approving promotion or rollback.";
            if !prompt
                .success_criteria
                .iter()
                .any(|existing| existing == criterion)
            {
                prompt.success_criteria.push(criterion.to_string());
            }
        }
    }
}

pub(crate) fn update_shell(input: UpdateCommandInput<'_>) -> Result<()> {
    ensure_state_dir_ready(input.state_dir)?;
    update_command(input)
}

pub(crate) fn update_command(input: UpdateCommandInput<'_>) -> Result<()> {
    let UpdateCommandInput {
        symbol,
        outcome,
        entry_signal,
        feedback_file,
        state_dir,
        pnl,
        regime,
        direction,
        ensemble,
    } = input;

    let _ = migrate_ensemble_executor_scorecards(state_dir, symbol)?;

    let update_run_id = format!(
        "update:{}:{}",
        symbol,
        Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
    );
    let mut network = load_or_init_trading_network(symbol, state_dir)?;
    let previous_runs: Vec<UpdateRunRecord> =
        load_state_or_default(state_dir, symbol, UPDATE_RUNS_FILE)?;
    let mut learning_state = load_learning_state(state_dir, symbol)?;
    let previous_rankings = learning_state.factor_rankings.clone();
    let raw_outcome = outcome.trim().to_ascii_lowercase();
    let outcome_label = normalize_trade_outcome_label(outcome);
    let entry_signal = entry_signal.unwrap_or("medium");
    let mut consumed_pending_update_artifact: Option<PendingUpdateArtifact> = None;
    let feedback = if let Some(path) = feedback_file {
        let content = std::fs::read_to_string(path)?;
        match serde_json::from_str::<FeedbackRecord>(&content) {
            Ok(feedback) => feedback_record_from_artifact(
                PendingUpdateArtifact {
                    template_feedback: feedback,
                    ..PendingUpdateArtifact::default()
                },
                &raw_outcome,
                pnl,
                regime,
                direction,
            ),
            Err(_) => match serde_json::from_str::<
                ict_engine::application::orchestration::StructuralFeedbackSubmission,
            >(&content)
            {
                Ok(submission) => {
                    ict_engine::application::orchestration::feedback_record_from_structural_submission(
                        submission,
                        Some(symbol),
                        Some(&raw_outcome),
                        pnl,
                        regime.map(normalize_regime_label),
                        direction.map(normalize_direction_label),
                    )
                }
                Err(_) => {
                    let artifact = serde_json::from_str::<PendingUpdateArtifact>(&content)?;
                    consumed_pending_update_artifact = Some(artifact.clone());
                    feedback_record_from_artifact(artifact, &raw_outcome, pnl, regime, direction)
                }
            },
        }
    } else if state_exists(state_dir, symbol, PENDING_UPDATE_ARTIFACT_FILE) {
        let artifact = load_pending_update_artifact(state_dir, symbol)?;
        consumed_pending_update_artifact = Some(artifact.clone());
        feedback_record_from_artifact(artifact, &raw_outcome, pnl, regime, direction)
    } else {
        FeedbackRecord {
            timestamp: Utc::now(),
            symbol: symbol.to_string(),
            source: "update_command".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: Some(PROMPT_PACK_VERSION.to_string()),
            factor_version: None,
            data_fingerprint: None,
            factors_used: Vec::new(),
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: normalize_direction_label(direction.unwrap_or("neutral")),
                selected_probability: 0.0,
                long_score: 0.0,
                short_score: 0.0,
                win_prob_long: 0.0,
                win_prob_short: 0.0,
                uncertainty: 0.0,
            },
            realized_outcome: raw_outcome.clone(),
            pnl: pnl.unwrap_or_else(|| match raw_outcome.as_str() {
                "win" => 0.01,
                "loss" => -0.01,
                _ => 0.0,
            }),
            regime_at_entry: normalize_regime_label(regime.unwrap_or("manipulation_expansion")),
            structural_feedback: None,
            reflection_mismatch_tags: Vec::new(),
        }
    };
    let consumed_execution_candidate_artifact = latest_execution_candidate_for_source_run(
        state_dir,
        symbol,
        consumed_pending_update_artifact
            .as_ref()
            .and_then(|artifact| artifact.source_run_id.as_deref()),
    )?;
    let consumed_analyze_context = consumed_analyze_context_for_update(
        state_dir,
        symbol,
        consumed_pending_update_artifact.as_ref(),
        consumed_execution_candidate_artifact.as_ref(),
    )?;
    let feedback = enrich_feedback_record(
        feedback,
        &update_run_id,
        format!("{}:{}:{}", symbol, entry_signal, outcome_label),
        &learning_state,
        &compute_hash(&[
            "update",
            symbol,
            entry_signal,
            &outcome_label,
            direction.unwrap_or("neutral"),
        ]),
    );
    let learning_semantics = ict_engine::state::structural_feedback_learning_semantics(&feedback);
    let learning_semantics_summary = ict_engine::state::structural_learning_semantics_summary(
        Some(learning_semantics.credit_class.as_str()),
        Some(learning_semantics.success_credit),
        Some(learning_semantics.observation_weight),
    );
    let structural_feedback = feedback.structural_feedback.clone();
    let consumed_feedback_pnl = feedback.pnl;
    let entry_quality = normalize_entry_quality_label(entry_signal);
    let factor_alignment = factor_alignment_label_from_feedback(&feedback);
    let factor_uncertainty = factor_uncertainty_label_from_feedback(&feedback);
    let evidence = trade_evidence_from_labels(
        &network,
        &[
            ("entry_quality", entry_quality.as_str()),
            ("factor_alignment", factor_alignment.as_str()),
            ("factor_uncertainty", factor_uncertainty.as_str()),
        ],
    )?;
    let previous_updated = network
        .nodes
        .get("trade_outcome")
        .and_then(|node| node.probabilities_for_evidence(&evidence).ok());
    let new_feedback = learning_state.merge_feedback_records(std::slice::from_ref(&feedback));
    let feedback_records_applied = new_feedback.len();
    let realized_outcome_for_prompts = new_feedback
        .first()
        .map(|feedback| feedback.realized_outcome.as_str())
        .unwrap_or(raw_outcome.as_str());

    if let Some(feedback) = new_feedback.first().filter(|feedback| {
        ict_engine::state::structural_feedback_counts_as_executed_trade(feedback)
    }) {
        let outcome_label = ict_engine::state::structural_feedback_trade_outcome_proxy(feedback)
            .unwrap_or_else(|| normalize_trade_outcome_label(&feedback.realized_outcome));
        let realized_state_index = network
            .nodes
            .get("trade_outcome")
            .and_then(|node| node.state_index(&outcome_label))
            .ok_or_else(|| anyhow!("unknown outcome state '{}'", feedback.realized_outcome))?;

        CPTUpdater::default().update_from_trade(
            &mut network,
            &evidence,
            TradeOutcome {
                node_id: "trade_outcome".to_string(),
                realized_state_index,
            },
        )?;
        WeightUpdater::default().apply_feedback(&mut learning_state, &new_feedback);
    }

    let updated_node = network
        .nodes
        .get("trade_outcome")
        .ok_or_else(|| anyhow!("missing node 'trade_outcome'"))?;
    let updated = updated_node.probabilities_for_evidence(&evidence)?;
    save_state(state_dir, symbol, BBN_STATE_FILE, &network)?;
    save_learning_state(state_dir, symbol, &learning_state)?;

    let factor_ranking = learning_state.factor_rankings.clone();
    let factor_iteration_queue = learning_state.iteration_queue();
    let factor_family_decisions = learning_state.family_decisions();
    let feedback_history_summary = learning_state.summary();
    let trade_outcome_map = probability_map(&updated_node.states, &updated);
    let trade_outcome_deltas = probability_diffs(
        &previous_updated.map(|values| probability_map(&updated_node.states, &values)),
        &trade_outcome_map,
    );
    let factor_score_deltas = ranking_diffs(&previous_rankings, &factor_ranking);
    let agent_prompts = build_update_agent_prompts(BuildUpdateAgentPromptsInput {
        symbol,
        factor_ranking: &factor_ranking,
        factor_iteration_queue: &factor_iteration_queue,
        feedback_history_summary: &feedback_history_summary,
        updated_trade_outcome: &trade_outcome_map,
        normalized_entry_quality: &entry_quality,
        factor_alignment: &factor_alignment,
        factor_uncertainty: &factor_uncertainty,
        realized_outcome: realized_outcome_for_prompts,
        structural_learning_credit_class: Some(learning_semantics.credit_class.as_str()),
        structural_learning_success_credit: Some(learning_semantics.success_credit),
        structural_learning_observation_weight: Some(learning_semantics.observation_weight),
        feedback_records_applied,
        consumed_pre_bayes_evidence_filter: consumed_analyze_context
            .pre_bayes_evidence_filter
            .as_ref(),
        consumed_pre_bayes_entry_quality_bridge: consumed_analyze_context
            .pre_bayes_entry_quality_bridge
            .as_ref(),
        consumed_canonical_structural_regime_posterior: consumed_analyze_context
            .canonical_structural_regime_posterior
            .as_ref(),
        consumed_multi_timeframe_summary: &consumed_analyze_context.multi_timeframe_summary,
    });
    let mut agent_prompts = agent_prompts;
    agent_prompts.prompts.insert(
        0,
        dataset_audit_prompt(
            symbol,
            "update-command",
            None,
            feedback_records_applied.max(1),
            None,
            "update",
        ),
    );
    agent_prompts.prompts.push(update_diff_prompt(
        symbol,
        &trade_outcome_deltas,
        &factor_score_deltas,
        feedback_records_applied == 0,
    ));
    let mut ensemble_executor_scorecards = load_canonical_executor_scorecards(
        state_dir,
        symbol,
        consumed_execution_candidate_artifact
            .as_ref()
            .and_then(|artifact| artifact.source_run_id.as_deref()),
    )?;
    let ensemble_quality_adjustment = match outcome {
        "win" => 20,
        "loss" => -20,
        _ => 0,
    };
    apply_update_outcome_to_executor_scorecards(
        &mut ensemble_executor_scorecards,
        outcome,
        ensemble_quality_adjustment,
    );

    let report = UpdateReport {
        symbol: symbol.to_string(),
        timestamp: Utc::now(),
        state_dir: state_dir.to_string(),
        provenance: run_provenance(
            &learning_state,
            &[
                "update",
                entry_signal,
                &outcome_label,
                &feedback_records_applied.to_string(),
            ],
            compute_hash(&[
                "update-command",
                symbol,
                &outcome_label,
                &factor_alignment,
                &factor_uncertainty,
            ]),
        ),
        decision_thresholds: decision_thresholds(),
        dataset_comparability: DatasetComparability::default(),
        promotion_decision: PromotionDecision::default(),
        rollback_recommendation: RollbackRecommendation::default(),
        trade_outcome_deltas: trade_outcome_deltas.clone(),
        factor_score_deltas: factor_score_deltas.clone(),
        normalized_entry_quality: entry_quality,
        factor_alignment,
        factor_uncertainty,
        realized_outcome: feedback.realized_outcome.clone(),
        structural_learning_credit_class: Some(learning_semantics.credit_class.clone()),
        structural_learning_success_credit: Some(learning_semantics.success_credit),
        structural_learning_observation_weight: Some(learning_semantics.observation_weight),
        structural_feedback: structural_feedback.clone(),
        feedback_records_applied,
        duplicate_feedback_skipped: feedback_records_applied == 0,
        consumed_pending_update_artifact_id: consumed_pending_update_artifact
            .as_ref()
            .map(|artifact| artifact.artifact_id.clone()),
        consumed_execution_candidate_artifact_id: consumed_execution_candidate_artifact
            .as_ref()
            .map(|artifact| artifact.artifact_id.clone()),
        consumed_artifact_path: consumed_pending_update_artifact.as_ref().map(|_| {
            std::path::Path::new(state_dir)
                .join(symbol)
                .join(PENDING_UPDATE_ARTIFACT_FILE)
                .to_string_lossy()
                .to_string()
        }),
        consumed_analyze_run_id: consumed_analyze_context.analyze_run_id.clone(),
        consumed_pre_bayes_evidence_filter: consumed_analyze_context
            .pre_bayes_evidence_filter
            .clone(),
        consumed_pre_bayes_entry_quality_bridge: consumed_analyze_context
            .pre_bayes_entry_quality_bridge
            .clone(),
        consumed_canonical_structural_regime_posterior: consumed_analyze_context
            .canonical_structural_regime_posterior
            .clone(),
        consumed_multi_timeframe_summary: consumed_analyze_context.multi_timeframe_summary.clone(),
        updated_trade_outcome: trade_outcome_map.clone(),
        factor_ranking,
        factor_iteration_queue,
        factor_family_decisions,
        factor_family_outcomes: Vec::new(),
        factor_family_diffs: Vec::new(),
        factor_family_history: Vec::new(),
        decision_history_summary: DecisionHistorySummary::default(),
        agent_action_plan: AgentActionPlan::default(),
        workflow_state: WorkflowState::default(),
        agent_context_bundle: AgentContextBundle::default(),
        agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
        recommended_commands: CommandRecommendations::default(),
        recommended_next_command: "recommended_command_unavailable".to_string(),
        agent_prompts: agent_prompts.clone(),
        feedback_history_summary,
        artifact_action_summary: Vec::new(),
        artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary::default(),
        artifact_decision_section: ict_engine::state::ArtifactDecisionSection::default(),
        workflow_snapshot: WorkflowSnapshot::default(),
    };
    let mut report = report;
    report.dataset_comparability = dataset_comparability(
        previous_runs.last().map(|run| run.run_id.clone()),
        previous_runs.last().map(|run| &run.provenance),
        &report.provenance,
    );
    let mut artifact_preview_ledger = load_artifact_ledger(state_dir, symbol)?;
    let preview_consumed_at = Utc::now();
    if let Some(artifact_id) = &report.consumed_pending_update_artifact_id {
        apply_artifact_consumption_preview(
            &mut artifact_preview_ledger,
            artifact_id,
            &update_run_id,
            &report.realized_outcome,
            consumed_feedback_pnl,
            preview_consumed_at,
        );
    }
    if let Some(artifact_id) = &report.consumed_execution_candidate_artifact_id {
        apply_artifact_consumption_preview(
            &mut artifact_preview_ledger,
            artifact_id,
            &update_run_id,
            &report.realized_outcome,
            consumed_feedback_pnl,
            preview_consumed_at,
        );
    }
    let artifact_consumed_impact_summary =
        build_artifact_consumed_impact_summary(&artifact_preview_ledger);
    let artifact_consumed_gate = artifact_consumed_decision_gate(&artifact_consumed_impact_summary);
    let (artifact_factor_trends, artifact_family_trends) =
        ict_engine::application::artifacts::artifact_trend_summaries_from_ledger(
            &artifact_preview_ledger,
        );
    let thresholds = decision_thresholds();
    report.promotion_decision = derive_promotion_decision(
        &report.factor_ranking,
        &report.factor_score_deltas,
        &report.dataset_comparability,
        &thresholds,
        Some(&artifact_consumed_gate),
    );
    report.rollback_recommendation = derive_rollback_recommendation(
        &report.factor_ranking,
        &report.factor_score_deltas,
        &report.trade_outcome_deltas,
        &report.dataset_comparability,
        &thresholds,
        Some(&artifact_consumed_gate),
    );
    report.factor_family_outcomes = derive_family_outcomes(
        &report.factor_family_decisions,
        &thresholds,
        &report.dataset_comparability,
        Some(&artifact_family_trends),
    );
    append_learning_semantics_to_family_outcomes(
        &mut report.factor_family_outcomes,
        &learning_semantics_summary,
    );
    report.factor_family_diffs = family_diffs(
        previous_runs
            .last()
            .map(|run| run.factor_family_decisions.as_slice())
            .unwrap_or(&[]),
        &report.factor_family_decisions,
    );
    report.factor_family_history = family_history_from_runs(previous_runs.iter().map(|run| {
        (
            run.run_id.clone(),
            run.timestamp,
            run.factor_family_decisions.clone(),
        )
    }));
    report.decision_history_summary = decision_history_summary(previous_runs.iter().map(|run| {
        (
            run.promotion_decision.clone(),
            run.rollback_recommendation.clone(),
        )
    }));
    report.agent_action_plan = build_agent_action_plan(
        &format!(
            "update_result:{}",
            if report.duplicate_feedback_skipped {
                "duplicate_skipped"
            } else {
                "applied"
            }
        ),
        &report.promotion_decision,
        &report.rollback_recommendation,
        &report.factor_iteration_queue,
        &report.factor_family_outcomes,
    );
    if let Some(filter) = report.consumed_pre_bayes_evidence_filter.as_ref() {
        augment_action_plan_with_consumed_pre_bayes_context(
            &mut report.agent_action_plan,
            filter,
            report.consumed_pre_bayes_entry_quality_bridge.as_ref(),
        );
    }
    augment_action_plan_with_artifact_trends(
        &mut report.agent_action_plan,
        symbol,
        state_dir,
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_consumed_impact_summary,
    );
    report.artifact_action_summary = artifact_action_summary(
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_consumed_impact_summary,
    );
    report
        .artifact_action_summary
        .push(format!("learning_semantics={learning_semantics_summary}"));
    report.artifact_decision_summary =
        ict_engine::application::artifacts::artifact_decision_summary_from_ledger(
            &artifact_preview_ledger,
        );
    report.artifact_decision_section = artifact_decision_section_from_parts(
        &report.artifact_decision_summary,
        &report.artifact_action_summary,
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_rule_break_effects_for_symbol(state_dir, symbol)?,
        &artifact_consumed_impact_summary,
    );
    append_artifact_decision_prompt(
        &mut report.agent_prompts,
        symbol,
        &report.artifact_decision_section,
    );
    link_artifact_decision_summary_to_decisions(
        &report.artifact_decision_summary,
        &mut report.promotion_decision,
        &mut report.rollback_recommendation,
    );
    report.workflow_state = workflow_state_from_context(
        &format!(
            "update_result:{}",
            if report.duplicate_feedback_skipped {
                "duplicate_skipped"
            } else {
                "applied"
            }
        ),
        &report.promotion_decision,
        &report.rollback_recommendation,
    );
    report.recommended_commands = command_recommendations(&CommandContext {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        analyze: None,
        research_data: None,
        paired_data: None,
        update_outcome: Some(report.realized_outcome.clone()),
        update_entry_signal: Some(entry_signal.to_string()),
        update_feedback_file: feedback_file.map(str::to_string),
        user_data_selection_required: true,
    });
    concretize_action_plan_commands(&mut report.agent_action_plan, &report.recommended_commands);
    report.recommended_next_command =
        recommended_next_command(&report.agent_action_plan, &report.recommended_commands);
    let update_result_reason = format!(
        "update_result:{}",
        if report.duplicate_feedback_skipped {
            "duplicate_skipped"
        } else {
            "applied"
        }
    );
    report.agent_context_bundle = build_agent_context_bundle(BuildAgentContextBundleInput {
        symbol,
        state_dir,
        workflow_state: &report.workflow_state,
        decision_hint: &update_result_reason,
        recommended_next_command: &report.recommended_next_command,
        recommended_commands: &report.recommended_commands,
        dataset_comparability: &report.dataset_comparability,
        factor_iteration_queue: &report.factor_iteration_queue,
        family_outcomes: &report.factor_family_outcomes,
        pre_bayes_evidence_filter: report.consumed_pre_bayes_evidence_filter.as_ref(),
        pre_bayes_entry_quality_bridge: report.consumed_pre_bayes_entry_quality_bridge.as_ref(),
        pda_sequence_summary: None,
        factor_mutation_evaluation: None,
        artifact_decision_summary: Some(&report.artifact_decision_summary),
    });
    report.agent_context_bundle.multi_timeframe_summary =
        report.consumed_multi_timeframe_summary.clone();
    report.agent_context_bundle_minimal =
        build_agent_context_bundle_minimal(&report.agent_context_bundle);
    report.agent_prompts.prompts.push(promotion_gate_prompt(
        symbol,
        &report.factor_ranking,
        &report.factor_score_deltas,
        &report.decision_thresholds,
    ));
    report.agent_prompts.prompts.push(rollback_review_prompt(
        symbol,
        &report.factor_score_deltas,
        &report.trade_outcome_deltas,
        &report.decision_thresholds,
    ));
    append_learning_semantics_to_update_gate_prompts(
        &mut report.agent_prompts,
        &learning_semantics_summary,
    );
    let update_execution_fields = derive_update_execution_fields(
        feedback_records_applied,
        &report.realized_outcome,
        report.duplicate_feedback_skipped,
        report.promotion_decision.approved,
    );
    append_update_run(
        state_dir,
        symbol,
        UpdateRunRecord {
            run_id: update_run_id.to_string(),
            timestamp: Utc::now(),
            symbol: symbol.to_string(),
            ensemble_executor_scorecards,
            provenance: report.provenance.clone(),
            decision_thresholds: report.decision_thresholds.clone(),
            dataset_comparability: report.dataset_comparability.clone(),
            promotion_decision: report.promotion_decision.clone(),
            rollback_recommendation: report.rollback_recommendation.clone(),
            family_history_window: family_history_window(),
            source_command: "update".to_string(),
            normalized_entry_quality: report.normalized_entry_quality.clone(),
            factor_alignment: report.factor_alignment.clone(),
            factor_uncertainty: report.factor_uncertainty.clone(),
            realized_outcome: report.realized_outcome.clone(),
            structural_learning_credit_class: report.structural_learning_credit_class.clone(),
            structural_learning_success_credit: report.structural_learning_success_credit,
            structural_learning_observation_weight: report.structural_learning_observation_weight,
            structural_feedback: report.structural_feedback.clone(),
            feedback_records_applied,
            duplicate_feedback_skipped: report.duplicate_feedback_skipped,
            consumed_pending_update_artifact_id: report.consumed_pending_update_artifact_id.clone(),
            consumed_execution_candidate_artifact_id: report
                .consumed_execution_candidate_artifact_id
                .clone(),
            consumed_artifact_path: report.consumed_artifact_path.clone(),
            consumed_analyze_run_id: report.consumed_analyze_run_id.clone(),
            consumed_pre_bayes_evidence_filter: report.consumed_pre_bayes_evidence_filter.clone(),
            consumed_pre_bayes_entry_quality_bridge: report
                .consumed_pre_bayes_entry_quality_bridge
                .clone(),
            consumed_canonical_structural_regime_posterior: report
                .consumed_canonical_structural_regime_posterior
                .clone(),
            consumed_multi_timeframe_summary: report.consumed_multi_timeframe_summary.clone(),
            trade_outcome_deltas,
            factor_score_deltas,
            factor_family_decisions: report.factor_family_decisions.clone(),
            factor_family_outcomes: report.factor_family_outcomes.clone(),
            factor_family_diffs: report.factor_family_diffs.clone(),
            factor_family_history: report.factor_family_history.clone(),
            decision_history_summary: report.decision_history_summary.clone(),
            workflow_state: report.workflow_state.clone(),
            agent_action_plan: report.agent_action_plan.clone(),
            recommended_commands: report.recommended_commands.clone(),
            recommended_next_command: report.recommended_next_command.clone(),
            recommended_next_command_meta: recommended_next_command_meta(
                &report.recommended_next_command,
            ),
            agent_context_bundle: report.agent_context_bundle.clone(),
            agent_context_bundle_minimal: report.agent_context_bundle_minimal.clone(),
            feedback_history_summary: report.feedback_history_summary.clone(),
            artifact_action_summary: report.artifact_action_summary.clone(),
            duration_sizing_scale: None,
            hybrid_duration_model: None,
            hybrid_remaining_expected_bars: None,
            execution_artifact_id: report.consumed_execution_candidate_artifact_id.clone(),
            execution_edge_share: update_execution_fields.execution_edge_share,
            prediction_edge_share: update_execution_fields.prediction_edge_share,
            execution_readiness: update_execution_fields.execution_readiness,
            execution_gate_status: update_execution_fields.execution_gate_status.clone(),
            artifact_decision_summary: report.artifact_decision_summary.clone(),
            artifact_decision_section: report.artifact_decision_section.clone(),
            agent_prompts: report.agent_prompts.clone(),
            prompt_workflow: report.agent_prompts.workflow.clone(),
        },
    )?;
    if let Err(err) = export_policy_training_tables(state_dir, symbol) {
        eprintln!(
            "warning: failed to export policy training tables for '{}' in '{}': {:#}",
            symbol, state_dir, err
        );
    }
    if let Some(artifact_id) = &report.consumed_pending_update_artifact_id {
        mark_artifact_consumed(
            state_dir,
            symbol,
            artifact_id,
            &update_run_id,
            &report.realized_outcome,
            consumed_feedback_pnl,
        )?;
    }
    if let Some(artifact_id) = &report.consumed_execution_candidate_artifact_id {
        mark_artifact_consumed(
            state_dir,
            symbol,
            artifact_id,
            &update_run_id,
            &report.realized_outcome,
            consumed_feedback_pnl,
        )?;
    }
    report.workflow_snapshot = refresh_workflow_snapshot(state_dir, symbol)?;
    report.artifact_decision_summary = artifact_decision_summary_from_snapshot(
        &report.workflow_snapshot,
        &report.artifact_action_summary,
    );
    report.artifact_decision_section =
        artifact_decision_section_from_snapshot(&report.workflow_snapshot);
    append_artifact_decision_prompt(
        &mut report.agent_prompts,
        symbol,
        &report.artifact_decision_section,
    );
    link_artifact_decision_summary_to_decisions(
        &report.artifact_decision_summary,
        &mut report.promotion_decision,
        &mut report.rollback_recommendation,
    );
    if let (Some(refs), Some(seed)) = (
        report.structural_feedback.as_ref(),
        structural_prior_seed_from_artifact_validation(&report.artifact_decision_summary),
    ) {
        learning_state.apply_structural_prior_seed(refs, &seed);
        save_learning_state(state_dir, symbol, &learning_state)?;
    }
    let provider_status_agent = provider_status_agent_surface(None, None, None).unwrap_or_default();
    if let Err(err) = export_structural_path_ranking_target(
        state_dir,
        symbol,
        &report.workflow_snapshot,
        &provider_status_agent,
        &learning_state.feedback_history,
        &learning_state.structural_prior_state,
    ) {
        eprintln!(
            "warning: failed to export structural path ranking target for '{}' in '{}': {:#}",
            symbol, state_dir, err
        );
    }

    emit_update_output(&report, ensemble)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structural_prior_seed_from_artifact_validation_maps_status() {
        let positive = structural_prior_seed_from_artifact_validation(
            &ict_engine::state::ArtifactDecisionSummary {
                consumed_trend_status: "validated_positive".to_string(),
                ..ict_engine::state::ArtifactDecisionSummary::default()
            },
        )
        .expect("positive seed");
        let regressing = structural_prior_seed_from_artifact_validation(
            &ict_engine::state::ArtifactDecisionSummary {
                consumed_trend_status: "validated_regressing".to_string(),
                ..ict_engine::state::ArtifactDecisionSummary::default()
            },
        )
        .expect("regressing seed");
        let neutral = structural_prior_seed_from_artifact_validation(
            &ict_engine::state::ArtifactDecisionSummary {
                consumed_trend_status: "validated_neutral".to_string(),
                ..ict_engine::state::ArtifactDecisionSummary::default()
            },
        )
        .expect("neutral seed");

        assert_eq!(positive.source_label, "artifact_validation");
        assert_eq!(positive.wins, 2);
        assert_eq!(regressing.losses, 2);
        assert_eq!(neutral.breakevens, 1);
        assert!(structural_prior_seed_from_artifact_validation(
            &ict_engine::state::ArtifactDecisionSummary::default()
        )
        .is_none());
    }
}
