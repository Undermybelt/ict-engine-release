use super::*;

pub(crate) fn run_factor_research(
    input: RunFactorResearchInput<'_>,
) -> Result<ict_engine::factor_lab::ResearchReport> {
    let RunFactorResearchInput {
        symbol,
        data,
        objective,
        data_1m,
        data_5m,
        data_15m,
        data_1h,
        data_4h,
        data_1d,
        paired_data,
        mutation_spec,
        state_dir,
    } = input;
    let candles = load_candles(data)?;
    let paired_candles = paired_data.map(load_candles).transpose()?;
    let resolved_multi_timeframe_inputs =
        resolve_multi_timeframe_inputs(data, data_1m, data_5m, data_15m, data_1h, data_4h, data_1d);
    let multi_timeframe_summary =
        build_multi_timeframe_summary(data, &resolved_multi_timeframe_inputs)?;
    let multi_timeframe_signal =
        build_multi_timeframe_research_signal(&resolved_multi_timeframe_inputs)?;
    let previous_runs: Vec<ResearchRunRecord> =
        load_state_or_default(state_dir, symbol, RESEARCH_RUNS_FILE)?;
    let mut learning_state = load_learning_state(state_dir, symbol)?;
    let baseline_learning_state = learning_state.clone();
    let previous_rankings = learning_state.factor_rankings.clone();
    let existing_feedback = learning_state
        .feedback_history
        .iter()
        .map(LearningState::feedback_key)
        .collect::<std::collections::BTreeSet<_>>();
    let mut registry = FactorRegistry::default();
    let baseline_multi_timeframe_summary = multi_timeframe_summary
        .iter()
        .chain(multi_timeframe_signal.summary.iter())
        .cloned()
        .collect::<Vec<_>>();
    let baseline_metrics = mutation_spec.map(|spec| {
        baseline_factor_mutation_metrics(BaselineFactorMutationMetricsInput {
            registry: &registry,
            symbol,
            objective,
            target_factor: if spec.base_factor.is_empty() {
                None
            } else {
                Some(spec.base_factor.as_str())
            },
            baseline_learning_state: &baseline_learning_state,
            candles: &candles,
            paired_candles: paired_candles.as_deref(),
            multi_timeframe_summary: &baseline_multi_timeframe_summary,
            evaluate_expansion_preview: spec.evaluate_expansion_preview,
        })
    });
    if let Some(spec) = mutation_spec {
        apply_factor_mutation_spec(&mut registry, spec)?;
    }
    let objective_registry = registry.clone();
    let lab = FactorLab::new(registry);
    let report = lab.run_research(
        symbol,
        &candles,
        &FactorContext {
            paired_candles: paired_candles.as_deref(),
            auxiliary: None,
            regime: None,
        },
        Some(&mut learning_state),
        &FactorBacktestConfig::default(),
        true,
    )?;
    let new_feedback = learning_state
        .feedback_history
        .iter()
        .filter(|record| !existing_feedback.contains(&LearningState::feedback_key(record)))
        .cloned()
        .collect::<Vec<_>>();
    let run_timestamp = Utc::now();
    let run_id = format!(
        "research:{}:{}",
        symbol,
        run_timestamp.format("%Y%m%dT%H%M%S%.3fZ")
    );
    let mut report = report;
    report.research_objective = research_objective_label(objective).to_string();
    let market_family = market_category_for_symbol(symbol).map(str::to_string);
    let objective_jump_weight = historical_market_jump_objective_weight(
        state_dir,
        symbol,
        market_family.as_deref(),
        Some(report.research_objective.as_str()),
    );
    if objective == ResearchObjectiveMode::ExpansionManipulation {
        apply_expansion_manipulation_objective(
            &mut report,
            &objective_registry,
            symbol,
            &candles,
            &multi_timeframe_summary
                .iter()
                .chain(multi_timeframe_signal.summary.iter())
                .cloned()
                .collect::<Vec<_>>(),
            objective_jump_weight,
        )?;
        learning_state.factor_rankings = report.backtest.scorecards.clone();
    }
    let score_deltas = ranking_diffs(&previous_rankings, &learning_state.factor_rankings);
    let thresholds = decision_thresholds();
    let factor_family_decisions = learning_state.family_decisions();
    report.factor_score_deltas = score_deltas.clone();
    report.feedback_history_summary = learning_state.summary();
    report.factor_family_decisions = factor_family_decisions.clone();
    report.decision_thresholds = thresholds.clone();
    report.provenance = run_provenance(
        &learning_state,
        &[
            "factor-research",
            "FactorBacktestConfig::default",
            data,
            paired_data.unwrap_or(""),
        ],
        data_fingerprint(&candles, paired_candles.as_deref(), "analyze"),
    );
    report.dataset_comparability = dataset_comparability(
        previous_runs.last().map(|run| run.run_id.clone()),
        previous_runs.last().map(|run| &run.provenance),
        &report.provenance,
    );
    let artifact_consumed_gate = ict_engine::application::backtest::artifact_consumed_decision_gate(
        &artifact_consumed_impact_summary_for_symbol(state_dir, symbol)?,
    );
    let (_, artifact_family_trends) = artifact_trend_summaries_for_symbol(state_dir, symbol)?;
    report.promotion_decision = derive_promotion_decision(
        &learning_state.factor_rankings,
        &score_deltas,
        &report.dataset_comparability,
        &thresholds,
        Some(&artifact_consumed_gate),
    );
    report.factor_family_outcomes = derive_family_outcomes(
        &factor_family_decisions,
        &thresholds,
        &report.dataset_comparability,
        Some(&artifact_family_trends),
    );
    report.factor_family_diffs = family_diffs(
        previous_runs
            .last()
            .map(|run| run.factor_family_decisions.as_slice())
            .unwrap_or(&[]),
        &factor_family_decisions,
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
    report.backtest.provenance = report.provenance.clone();
    report.backtest.feedback_records_generated = report.feedback_records_generated;
    report.backtest.feedback_records_applied = report.feedback_records_applied;
    report.backtest.feedback_history_summary = report.feedback_history_summary.clone();
    report.backtest.dataset_comparability = report.dataset_comparability.clone();
    report.backtest.promotion_decision = report.promotion_decision.clone();
    report.backtest.decision_thresholds = report.decision_thresholds.clone();
    report.backtest.factor_family_decisions = factor_family_decisions.clone();
    report.backtest.factor_family_outcomes = report.factor_family_outcomes.clone();
    report.backtest.factor_family_diffs = report.factor_family_diffs.clone();
    report.backtest.factor_family_history = report.factor_family_history.clone();
    report.backtest.decision_history_summary = report.decision_history_summary.clone();

    let enriched_feedback = new_feedback
        .into_iter()
        .enumerate()
        .map(|(index, feedback)| {
            enrich_feedback_record(
                feedback,
                &run_id,
                format!("factor-research:{}:{}", symbol, index),
                &learning_state,
                &report.provenance.data_fingerprint,
            )
        })
        .collect::<Vec<_>>();
    let mut network = load_or_init_trading_network(symbol, state_dir)?;
    let previous_trade_outcome_cpt =
        ict_engine::application::backtest::trade_outcome_cpt_snapshot(&network)?;
    if !enriched_feedback.is_empty() {
        learning_state.replace_feedback_records(&enriched_feedback);
        apply_feedback_to_trade_outcome_network(&mut network, &enriched_feedback)?;
    }
    let final_trade_outcome_cpt =
        ict_engine::application::backtest::trade_outcome_cpt_snapshot(&network)?;
    report.backtest.trade_outcome_deltas =
        cpt_probability_diffs(&previous_trade_outcome_cpt, &final_trade_outcome_cpt);
    report.backtest.final_trade_outcome_cpt = final_trade_outcome_cpt.clone();
    report.rollback_recommendation = derive_rollback_recommendation(
        &learning_state.factor_rankings,
        &score_deltas,
        &report.backtest.trade_outcome_deltas,
        &report.dataset_comparability,
        &thresholds,
        Some(&artifact_consumed_gate),
    );
    report.backtest.rollback_recommendation = report.rollback_recommendation.clone();
    report.agent_prompts = factor_iteration_prompt_pack(
        symbol,
        &learning_state.factor_rankings,
        &report.backtest.iteration_queue,
        &report.feedback_history_summary,
    );
    report.agent_prompts.prompts.insert(
        0,
        dataset_audit_prompt(
            symbol,
            data,
            paired_data,
            candles.len(),
            paired_candles.as_ref().map(Vec::len),
            "factor-research",
        ),
    );
    if objective != ResearchObjectiveMode::Generic {
        report.agent_prompts.prompts.insert(
            1,
            AgentPrompt::new(AgentPromptInput {
                id: "research_objective".to_string(),
                stage: "research".to_string(),
                priority: "high".to_string(),
                objective: "Score this run against the active research objective before trusting default aggregate-return rankings.".to_string(),
                system_prompt: "Treat the active research objective as the primary iteration gate for this run. Do not let generic aggregate-return rankings override expansion/manipulation separation quality.".to_string(),
                user_prompt: format!(
                    "research_objective={} best_factor={:?} factor_count={} iteration_queue_len={}",
                    report.research_objective,
                    report.best_factor,
                    report.factor_count,
                    report.backtest.iteration_queue.len()
                ),
                success_criteria: vec![
                    "Use objective-ranked scorecards for the next mutation cycle".to_string(),
                    "Preserve liquidity-sweep manipulation discrimination while improving PreBayes gate acceptance".to_string(),
                ],
                suggested_files: vec!["src/main.rs".to_string(), "src/factor_lab/factor_definition.rs".to_string()],
            }),
        );
    }
    report.multi_timeframe_summary = multi_timeframe_summary
        .iter()
        .chain(multi_timeframe_signal.summary.iter())
        .cloned()
        .collect();
    report.agent_prompts.prompts.push(research_diff_prompt(
        symbol,
        &score_deltas,
        report.feedback_records_generated,
        report.feedback_records_applied,
    ));
    report.agent_prompts.prompts.push(promotion_gate_prompt(
        symbol,
        &learning_state.factor_rankings,
        &score_deltas,
        &report.decision_thresholds,
    ));
    report.agent_prompts.prompts.push(rollback_review_prompt(
        symbol,
        &score_deltas,
        &report.backtest.trade_outcome_deltas,
        &report.decision_thresholds,
    ));
    report.workflow_state = workflow_state_from_context(
        "research_review_ready",
        &report.promotion_decision,
        &report.rollback_recommendation,
    );
    let coverage_caution = report
        .backtest
        .factor_results
        .iter()
        .filter(|result| result.metrics.conformal_coverage_1sigma < 0.55)
        .map(|result| {
            format!(
                "conformal_coverage_low:{}:{:.3}",
                result.factor_name, result.metrics.conformal_coverage_1sigma
            )
        })
        .collect::<Vec<_>>();
    let break_caution = report
        .backtest
        .factor_results
        .iter()
        .filter(|result| result.metrics.regime_break_penalty > 0.20)
        .map(|result| {
            format!(
                "regime_break_penalty_high:{}:{:.3}",
                result.factor_name, result.metrics.regime_break_penalty
            )
        })
        .collect::<Vec<_>>();
    let structural_break_caution = report
        .backtest
        .factor_results
        .iter()
        .filter(|result| result.metrics.structural_break_detected)
        .map(|result| {
            format!(
                "structural_break_detected:{}:score={:.3}:index={:?}",
                result.factor_name,
                result.metrics.structural_break_score,
                result.metrics.structural_break_index
            )
        })
        .collect::<Vec<_>>();
    report
        .artifact_action_summary
        .extend(coverage_caution.iter().cloned());
    report.artifact_action_summary.push(format!(
        "conformal_credibility:coverage_1sigma={:.3} miscoverage_1sigma={:.3} break_penalty={:.3}",
        report
            .backtest
            .factor_results
            .first()
            .map(|result| result.metrics.conformal_coverage_1sigma)
            .unwrap_or_default(),
        report
            .backtest
            .factor_results
            .first()
            .map(|result| result.metrics.conformal_miscoverage_1sigma)
            .unwrap_or_default(),
        report
            .backtest
            .factor_results
            .first()
            .map(|result| result.metrics.regime_break_penalty)
            .unwrap_or_default()
    ));
    report
        .artifact_action_summary
        .extend(break_caution.iter().cloned());
    report
        .artifact_action_summary
        .extend(structural_break_caution.iter().cloned());
    report.agent_action_plan = build_agent_action_plan(
        "research_review_ready",
        &report.promotion_decision,
        &report.rollback_recommendation,
        &report.backtest.iteration_queue,
        &report.factor_family_outcomes,
    );
    let (artifact_factor_trends, artifact_family_trends) =
        artifact_trend_summaries_for_symbol(state_dir, symbol)?;
    let artifact_consumed_impact_summary =
        artifact_consumed_impact_summary_for_symbol(state_dir, symbol)?;
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
    report.artifact_decision_summary = artifact_decision_summary_for_symbol(state_dir, symbol)?;
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
    report.recommended_commands = command_recommendations(&CommandContext {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        analyze: Some(AnalyzeCommandSource::Files {
            data_htf: data.to_string(),
            data_mtf: data.to_string(),
            data_ltf: data.to_string(),
        }),
        research_data: Some(data.to_string()),
        paired_data: paired_data.map(str::to_string),
        update_outcome: None,
        update_entry_signal: None,
        update_feedback_file: pending_update_artifact_path(state_dir, symbol),
        user_data_selection_required: true,
    });
    if objective != ResearchObjectiveMode::Generic && report.recommended_commands.research.ready {
        report.recommended_commands.research.command = format!(
            "{} --objective {}",
            report.recommended_commands.research.command,
            shell_quote(report.research_objective.as_str())
        );
    }
    concretize_action_plan_commands(&mut report.agent_action_plan, &report.recommended_commands);
    report.recommended_next_command =
        recommended_next_command(&report.agent_action_plan, &report.recommended_commands);
    let mutation_evaluation = mutation_spec.map(|spec| {
        evaluate_factor_mutation(
            spec,
            objective,
            baseline_metrics.as_ref(),
            &report,
            &candles,
            paired_candles.as_deref(),
        )
    });
    if let Some(evaluation) = &mutation_evaluation {
        augment_action_plan_with_factor_mutation_evaluation(
            &mut report.agent_action_plan,
            evaluation,
        );
        concretize_action_plan_commands(
            &mut report.agent_action_plan,
            &report.recommended_commands,
        );
        report.recommended_next_command =
            recommended_next_command(&report.agent_action_plan, &report.recommended_commands);
        report.agent_prompts.prompts.push(AgentPrompt::new(AgentPromptInput {
            id: "factor-mutation-evaluation".to_string(),
            stage: "iteration".to_string(),
            priority: "high".to_string(),
            objective: "Review the latest factor mutation evaluation before accepting the next factor edit.".to_string(),
            system_prompt: "Treat the mutation evaluation as a mechanical gate. Do not accept a mutation that regresses PreBayes quality or fails the score delta check.".to_string(),
            user_prompt: format!(
                "mutation_id={} accepted={} score_before={:.4} score_after={:.4} delta={:.4} failure_tags={}",
                evaluation.mutation_id,
                evaluation.accepted,
                evaluation.score_before,
                evaluation.score_after,
                evaluation.score_delta,
                if evaluation.failure_tags.is_empty() {
                    "none".to_string()
                } else {
                    evaluation.failure_tags.join(",")
                }
            ),
            success_criteria: vec![
                "Only accept positive score deltas without new PreBayes failure tags".to_string(),
                "Reject mutations that increase soft evidence divergence or collapse bridge probability gap".to_string(),
            ],
            suggested_files: vec!["src/main.rs".to_string(), "src/factors/registry.rs".to_string()],
        }));
        report
            .agent_prompts
            .prompts
            .push(factor_mutation_focus_prompt(
                mutation_spec,
                evaluation,
                mutation_spec
                    .map(|spec| spec.evaluate_expansion_preview)
                    .unwrap_or(false),
            ));
    }
    report.agent_context_bundle = build_agent_context_bundle(BuildAgentContextBundleInput {
        symbol,
        state_dir,
        workflow_state: &report.workflow_state,
        decision_hint: "research_review_ready",
        recommended_next_command: &report.recommended_next_command,
        recommended_commands: &report.recommended_commands,
        dataset_comparability: &report.dataset_comparability,
        factor_iteration_queue: &report.backtest.iteration_queue,
        family_outcomes: &report.factor_family_outcomes,
        pre_bayes_evidence_filter: None,
        pre_bayes_entry_quality_bridge: None,
        pda_sequence_summary: None,
        factor_mutation_evaluation: mutation_evaluation.as_ref(),
        artifact_decision_summary: Some(&report.artifact_decision_summary),
    });
    report.agent_context_bundle.multi_timeframe_summary = report.multi_timeframe_summary.clone();
    report.agent_context_bundle_minimal =
        build_agent_context_bundle_minimal(&report.agent_context_bundle);
    report.backtest.agent_prompts = report.agent_prompts.clone();
    report.backtest.agent_action_plan = report.agent_action_plan.clone();
    report.backtest.workflow_state = report.workflow_state.clone();
    report.backtest.recommended_next_command = report.recommended_next_command.clone();
    report.backtest.recommended_commands = report.recommended_commands.clone();
    report.backtest.artifact_action_summary = report.artifact_action_summary.clone();
    report.backtest.agent_context_bundle = report.agent_context_bundle.clone();
    report.backtest.agent_context_bundle_minimal = report.agent_context_bundle_minimal.clone();
    if !enriched_feedback.is_empty() {
        save_state(state_dir, symbol, BBN_STATE_FILE, &network)?;
    }
    save_learning_state(state_dir, symbol, &learning_state)?;
    let research_execution_fields = derive_research_execution_fields(
        report.dataset_comparability.comparable,
        report.promotion_decision.approved,
        report.rollback_recommendation.should_rollback,
        report.feedback_records_applied,
        report.aggregate_return,
        report.best_factor.is_some(),
    );
    let research_run_record = ResearchRunRecord {
        run_id,
        timestamp: run_timestamp,
        symbol: symbol.to_string(),
        research_objective: report.research_objective.clone(),
        provenance: report.provenance.clone(),
        decision_thresholds: report.decision_thresholds.clone(),
        dataset_comparability: report.dataset_comparability.clone(),
        promotion_decision: report.promotion_decision.clone(),
        rollback_recommendation: report.rollback_recommendation.clone(),
        family_history_window: family_history_window(),
        data_path: data.to_string(),
        paired_data_path: paired_data.map(str::to_string),
        candles: candles.len(),
        paired_candles: paired_candles.as_ref().map(Vec::len),
        config_name: "FactorBacktestConfig::default".to_string(),
        source_command: "factor-research".to_string(),
        factor_count: report.factor_count,
        best_factor: report.best_factor.clone(),
        aggregate_return: report.aggregate_return,
        feedback_records_generated: report.feedback_records_generated,
        feedback_records_applied: report.feedback_records_applied,
        factor_score_deltas: score_deltas,
        factor_family_decisions,
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
        duration_sizing_scale: Some(
            parse_duration_sizing_scale(&report.backtest.artifact_action_summary).unwrap_or(1.0),
        ),
        hybrid_duration_model: report
            .workflow_snapshot
            .latest_backtest
            .as_ref()
            .and_then(|phase| phase.hybrid_duration_model.clone()),
        hybrid_remaining_expected_bars: report
            .workflow_snapshot
            .latest_backtest
            .as_ref()
            .and_then(|phase| phase.hybrid_remaining_expected_bars),
        backtest_conformal_coverage_1sigma: report
            .backtest
            .factor_results
            .first()
            .map(|result| result.metrics.conformal_coverage_1sigma)
            .unwrap_or_default(),
        backtest_trade_count: report
            .backtest
            .factor_results
            .iter()
            .map(|result| result.trades.len())
            .sum(),
        artifact_decision_summary: report.artifact_decision_summary.clone(),
        artifact_decision_section: report.artifact_decision_section.clone(),
        agent_prompts: report.agent_prompts.clone(),
        prompt_workflow: report.agent_prompts.workflow.clone(),
        factor_mutation_evaluation: mutation_evaluation.clone(),
        multi_timeframe_summary: report.multi_timeframe_summary.clone(),
        execution_artifact_id: None,
        execution_edge_share: research_execution_fields.execution_edge_share,
        prediction_edge_share: research_execution_fields.prediction_edge_share,
        execution_readiness: research_execution_fields.execution_readiness,
        execution_gate_status: research_execution_fields.execution_gate_status.clone(),
        pda_cluster_label: report
            .agent_context_bundle_minimal
            .pda_cluster_label
            .clone(),
    };
    let research_runs = append_research_run(state_dir, symbol, research_run_record.clone())?;
    let market_family = market_category_for_symbol(symbol);
    let market_behavior_profile = market_family.map(market_behavior_profile_for_family);
    persist_market_jump_calibration_from_research_runs(
        state_dir,
        symbol,
        &research_runs,
        market_family,
        market_behavior_profile,
    )?;
    persist_market_jump_objective_calibration_from_research_runs(
        state_dir,
        symbol,
        &research_runs,
        market_family,
        Some(research_objective_label(objective)),
    )?;

    let research_ensemble_vote = build_stub_ensemble_vote_from_research(&report);
    let canonical_scorecards =
        load_ensemble_executor_scorecards(state_dir, symbol).unwrap_or_default();
    let research_ensemble_record = build_ensemble_vote_record(
        symbol,
        "factor-research",
        Some(research_run_record.run_id.clone()),
        &report.provenance,
        &report.dataset_comparability,
        &research_ensemble_vote,
        &canonical_scorecards,
    );
    persist_ensemble_vote_record(state_dir, &research_ensemble_record, &canonical_scorecards)?;
    if let (Some(spec), Some(evaluation)) = (mutation_spec, mutation_evaluation.clone()) {
        let mutation_run_id = format!(
            "factor-mutation:{}:{}",
            symbol,
            run_timestamp.format("%Y%m%dT%H%M%S%.3fZ")
        );
        append_factor_mutation_run(
            state_dir,
            symbol,
            FactorMutationRunRecord {
                run_id: mutation_run_id,
                timestamp: run_timestamp,
                symbol: symbol.to_string(),
                source_command: "factor-research".to_string(),
                data_path: data.to_string(),
                paired_data_path: paired_data.map(str::to_string),
                mutation_spec: spec.clone(),
                evaluation,
            },
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
    report.backtest.workflow_snapshot = report.workflow_snapshot.clone();
    report.backtest.artifact_decision_summary = report.artifact_decision_summary.clone();
    report.backtest.artifact_decision_section = report.artifact_decision_section.clone();
    report.factor_mutation_evaluation = mutation_evaluation;
    Ok(report)
}
