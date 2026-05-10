use super::*;
use ict_engine::application::backtest::parse_duration_sizing_scale;
use ict_engine::types::RegimeV2;
use std::collections::HashMap;
use std::path::Path;

/// Regime label entry from HMM output
#[derive(Debug, Clone, serde::Deserialize)]
struct RegimeLabelJson {
    ts: String,
    family: String,
}

/// Load regime V2 labels from JSON file alongside candle data
fn load_regime_v2_labels(data_path: &str) -> HashMap<String, RegimeV2> {
    // Try to find regime_v2_labels.json alongside the candle data
    let data_dir = Path::new(data_path).parent().unwrap_or(Path::new("."));
    let regime_path = data_dir.join("regime_v2_labels.json");

    if !regime_path.exists() {
        // Try /tmp as fallback for HMM output
        let tmp_path = Path::new("/tmp/hmm_regime_nq_15m_v8/regime_v2_labels.json");
        if tmp_path.exists() {
            return load_regime_v2_from_path(tmp_path);
        }
        return HashMap::new();
    }

    load_regime_v2_from_path(&regime_path)
}

fn load_regime_v2_from_path(path: &Path) -> HashMap<String, RegimeV2> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    let labels: Vec<RegimeLabelJson> = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(_) => return HashMap::new(),
    };

    labels
        .into_iter()
        .filter_map(|l| parse_regime_v2(&l.family).map(|r| (l.ts, r)))
        .collect()
}

/// Parse regime family string into RegimeV2 enum
fn parse_regime_v2(family: &str) -> Option<RegimeV2> {
    match family {
        "trend_up_strong" => Some(RegimeV2::TrendUpStrong),
        "trend_up_weak" => Some(RegimeV2::TrendUpWeak),
        "trend_down_strong" => Some(RegimeV2::TrendDownStrong),
        "trend_down_weak" => Some(RegimeV2::TrendDownWeak),
        "range_quiet" => Some(RegimeV2::RangeQuiet),
        "range_volatile" => Some(RegimeV2::RangeVolatile),
        "transition" => Some(RegimeV2::Transition),
        "crash_recovery" => Some(RegimeV2::CrashRecovery),
        _ => None,
    }
}

pub(crate) struct RunFactorBacktestInput<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) data: &'a str,
    pub(crate) multi_timeframe_inputs: MultiTimeframeInputPaths<'a>,
    pub(crate) paired_data: Option<&'a str>,
    pub(crate) auxiliary_override:
        Option<&'a ict_engine::data::realtime::market_support::AuxiliaryMarketEvidence>,
    pub(crate) state_dir: &'a str,
}

pub(crate) fn run_factor_backtest(
    input: RunFactorBacktestInput<'_>,
) -> Result<ict_engine::factor_lab::BacktestResult> {
    let RunFactorBacktestInput {
        symbol,
        data,
        multi_timeframe_inputs,
        paired_data,
        auxiliary_override,
        state_dir,
    } = input;
    let candles = load_candles(data)?;
    let paired_candles = paired_data.map(load_candles).transpose()?;
    let resolved_multi_timeframe_inputs =
        resolve_multi_timeframe_inputs(data, multi_timeframe_inputs);
    let multi_timeframe_summary =
        build_multi_timeframe_summary(data, &resolved_multi_timeframe_inputs)?;
    let multi_timeframe_signal =
        build_multi_timeframe_research_signal(&resolved_multi_timeframe_inputs)?;
    let structure_ict_context =
        build_structure_ict_context_events(&resolved_multi_timeframe_inputs)?;
    let previous_runs: Vec<BacktestRunRecord> =
        load_state_or_default(state_dir, symbol, BACKTEST_RUNS_FILE)?;
    let mut learning_state = load_learning_state(state_dir, symbol)?;
    let previous_rankings = learning_state.factor_rankings.clone();
    let existing_feedback = learning_state
        .feedback_history
        .iter()
        .map(LearningState::feedback_key)
        .collect::<std::collections::BTreeSet<_>>();
    let mut registry = FactorRegistry::default();
    ict_engine::factors::FactorHotplugConfig::apply_to_registry_if_present(
        state_dir,
        &mut registry,
    );
    let lab = FactorLab::new(registry);

    // Load regime V2 labels if available
    let regime_v2_labels = load_regime_v2_labels(data);

    let research = lab.run_research(
        symbol,
        &candles,
        &FactorContext {
            paired_candles: paired_candles.as_deref(),
            m1_events: structure_ict_context.m1_events.as_deref(),
            m5_events: structure_ict_context.m5_events.as_deref(),
            m15_events: structure_ict_context.m15_events.as_deref(),
            m30_events: structure_ict_context.m30_events.as_deref(),
            h1_events: structure_ict_context.h1_events.as_deref(),
            h4_events: structure_ict_context.h4_events.as_deref(),
            d1_events: structure_ict_context.d1_events.as_deref(),
            w1_events: structure_ict_context.w1_events.as_deref(),
            auxiliary: auxiliary_override,
            regime: None,
            regime_v2_labels: Some(&regime_v2_labels),
        },
        Some(&mut learning_state),
        &FactorBacktestConfig::default(),
        true,
    )?;
    let feedback_records_generated = research.feedback_records_generated;
    let feedback_records_applied = research.feedback_records_applied;
    let run_timestamp = Utc::now();
    let run_id = format!(
        "factor-backtest:{}:{}",
        symbol,
        run_timestamp.format("%Y%m%dT%H%M%S%.3fZ")
    );
    let new_feedback = learning_state
        .feedback_history
        .iter()
        .filter(|record| !existing_feedback.contains(&LearningState::feedback_key(record)))
        .cloned()
        .collect::<Vec<_>>();
    let mut report = research.backtest;
    let thresholds = decision_thresholds();
    let score_deltas = ranking_diffs(&previous_rankings, &learning_state.factor_rankings);
    let first_score_delta = score_deltas.first().map(|item| item.score_delta);
    let factor_family_decisions = learning_state.family_decisions();

    report.feedback_records_generated = feedback_records_generated;
    report.feedback_records_applied = feedback_records_applied;
    report.feedback_history_summary = learning_state.summary();
    report.factor_family_decisions = factor_family_decisions.clone();
    report.provenance = run_provenance(
        &learning_state,
        &[
            "factor-backtest",
            "FactorBacktestConfig::default",
            data,
            paired_data.unwrap_or(""),
        ],
        data_fingerprint(&candles, paired_candles.as_deref(), "factor-backtest"),
    );
    report.decision_thresholds = thresholds.clone();
    report.dataset_comparability = dataset_comparability(
        previous_runs.last().map(|run| run.run_id.clone()),
        previous_runs.last().map(|run| &run.provenance),
        &report.provenance,
    );
    let artifact_consumed_gate = artifact_consumed_decision_gate(
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

    let enriched_feedback = new_feedback
        .into_iter()
        .enumerate()
        .map(|(index, feedback)| {
            enrich_feedback_record(
                feedback,
                &run_id,
                format!("factor-backtest:{}:{}", symbol, index),
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
    report.trade_outcome_deltas =
        cpt_probability_diffs(&previous_trade_outcome_cpt, &final_trade_outcome_cpt);
    report.final_trade_outcome_cpt = final_trade_outcome_cpt.clone();
    report.rollback_recommendation = derive_rollback_recommendation(
        &learning_state.factor_rankings,
        &score_deltas,
        &report.trade_outcome_deltas,
        &report.dataset_comparability,
        &thresholds,
        Some(&artifact_consumed_gate),
    );
    report.workflow_state = workflow_state_from_context(
        "factor_backtest_review_ready",
        &report.promotion_decision,
        &report.rollback_recommendation,
    );
    report.agent_action_plan = build_agent_action_plan(
        "factor_backtest_review_ready",
        &report.promotion_decision,
        &report.rollback_recommendation,
        &report.iteration_queue,
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
    concretize_action_plan_commands(&mut report.agent_action_plan, &report.recommended_commands);
    report.recommended_next_command =
        recommended_next_command(&report.agent_action_plan, &report.recommended_commands);
    report.agent_context_bundle = build_agent_context_bundle(BuildAgentContextBundleInput {
        symbol,
        state_dir,
        workflow_state: &report.workflow_state,
        decision_hint: "factor_backtest_review_ready",
        recommended_next_command: &report.recommended_next_command,
        recommended_commands: &report.recommended_commands,
        dataset_comparability: &report.dataset_comparability,
        factor_iteration_queue: &report.iteration_queue,
        family_outcomes: &report.factor_family_outcomes,
        pre_bayes_evidence_filter: None,
        pre_bayes_entry_quality_bridge: None,
        pda_sequence_summary: None,
        factor_mutation_evaluation: None,
        artifact_decision_summary: Some(&report.artifact_decision_summary),
    });
    report.multi_timeframe_summary = multi_timeframe_summary
        .iter()
        .cloned()
        .chain(multi_timeframe_signal.summary.iter().cloned())
        .collect();
    report
        .multi_timeframe_summary
        .extend(build_market_state_summary_for_candles(&candles));
    report
        .multi_timeframe_summary
        .push(structure_ict_pda_context_summary(&structure_ict_context));
    report.agent_context_bundle.multi_timeframe_summary = report.multi_timeframe_summary.clone();
    report.agent_context_bundle_minimal =
        build_agent_context_bundle_minimal(&report.agent_context_bundle);
    report.agent_prompts = ict_engine::application::backtest::build_backtest_agent_prompts(
        symbol,
        &learning_state.factor_rankings,
        &report.iteration_queue,
        &report.feedback_history_summary,
        report.aggregate_return,
        report
            .factor_results
            .iter()
            .map(|result| result.trades.len())
            .sum(),
        &report.final_trade_outcome_cpt,
    );
    report.agent_prompts.prompts.insert(
        0,
        dataset_audit_prompt(
            symbol,
            data,
            paired_data,
            candles.len(),
            paired_candles.as_ref().map(Vec::len),
            "factor-backtest",
        ),
    );
    report.agent_prompts.prompts.push(promotion_gate_prompt(
        symbol,
        &learning_state.factor_rankings,
        &score_deltas,
        &report.decision_thresholds,
    ));
    report.agent_prompts.prompts.push(rollback_review_prompt(
        symbol,
        &score_deltas,
        &report.trade_outcome_deltas,
        &report.decision_thresholds,
    ));

    if !enriched_feedback.is_empty() {
        save_state(state_dir, symbol, BBN_STATE_FILE, &network)?;
    }
    save_learning_state(state_dir, symbol, &learning_state)?;
    let factor_backtest_objective_market_credibility_shrink = report
        .workflow_snapshot
        .latest_analyze
        .as_ref()
        .and_then(|snapshot| snapshot.objective_market_credibility_shrink.clone());
    let backtest_execution_fields = derive_backtest_execution_fields(
        report
            .factor_results
            .iter()
            .map(|result| result.trades.len())
            .sum(),
        report.aggregate_return,
        report
            .factor_results
            .first()
            .map(|result| result.metrics.regime_break_penalty)
            .unwrap_or_default(),
        report.promotion_decision.approved,
    );
    let backtest_runs = append_backtest_run(
        state_dir,
        symbol,
        BacktestRunRecord {
            run_id: run_id.clone(),
            timestamp: run_timestamp,
            symbol: symbol.to_string(),
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
            warmup_bars: FactorBacktestConfig::default().train_bars,
            hold_bars: FactorBacktestConfig::default().max_hold_bars,
            online_learning: true,
            source_command: "factor-backtest".to_string(),
            total_return: report.aggregate_return,
            trade_count: report
                .factor_results
                .iter()
                .map(|result| result.trades.len())
                .sum(),
            conformal_coverage_1sigma: report
                .factor_results
                .first()
                .map(|result| result.metrics.conformal_coverage_1sigma)
                .unwrap_or_default(),
            conformal_miscoverage_1sigma: report
                .factor_results
                .first()
                .map(|result| result.metrics.conformal_miscoverage_1sigma)
                .unwrap_or_default(),
            mean_prediction_interval_half_width: report
                .factor_results
                .first()
                .map(|result| result.metrics.mean_prediction_interval_half_width)
                .unwrap_or_default(),
            worst_window_miscoverage: report
                .factor_results
                .first()
                .map(|result| result.metrics.worst_window_miscoverage)
                .unwrap_or_default(),
            regime_break_penalty: report
                .factor_results
                .first()
                .map(|result| result.metrics.regime_break_penalty)
                .unwrap_or_default(),
            structural_break_score: report
                .factor_results
                .first()
                .map(|result| result.metrics.structural_break_score)
                .unwrap_or_default(),
            structural_break_index: report
                .factor_results
                .first()
                .and_then(|result| result.metrics.structural_break_index),
            structural_break_detected: report
                .factor_results
                .first()
                .map(|result| result.metrics.structural_break_detected)
                .unwrap_or(false),
            signal_structural_break_score: report
                .factor_results
                .first()
                .map(|result| result.metrics.signal_structural_break_score)
                .unwrap_or_default(),
            signal_structural_break_index: report
                .factor_results
                .first()
                .and_then(|result| result.metrics.signal_structural_break_index),
            signal_structural_break_detected: report
                .factor_results
                .first()
                .map(|result| result.metrics.signal_structural_break_detected)
                .unwrap_or(false),
            residual_structural_break_score: report
                .factor_results
                .first()
                .map(|result| result.metrics.residual_structural_break_score)
                .unwrap_or_default(),
            residual_structural_break_index: report
                .factor_results
                .first()
                .and_then(|result| result.metrics.residual_structural_break_index),
            residual_structural_break_detected: report
                .factor_results
                .first()
                .map(|result| result.metrics.residual_structural_break_detected)
                .unwrap_or(false),
            rolling_ic_structural_break_score: report
                .factor_results
                .first()
                .map(|result| result.metrics.rolling_ic_structural_break_score)
                .unwrap_or_default(),
            rolling_ic_structural_break_index: report
                .factor_results
                .first()
                .and_then(|result| result.metrics.rolling_ic_structural_break_index),
            rolling_ic_structural_break_detected: report
                .factor_results
                .first()
                .map(|result| result.metrics.rolling_ic_structural_break_detected)
                .unwrap_or(false),
            factor_score_deltas: score_deltas,
            trade_outcome_deltas: report.trade_outcome_deltas.clone(),
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
                parse_duration_sizing_scale(&report.artifact_action_summary).unwrap_or(1.0),
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
            execution_artifact_id: None,
            execution_edge_share: backtest_execution_fields.execution_edge_share,
            prediction_edge_share: backtest_execution_fields.prediction_edge_share,
            execution_readiness: backtest_execution_fields.execution_readiness,
            execution_gate_status: backtest_execution_fields.execution_gate_status.clone(),
            canonical_structural_regime_posterior: report
                .workflow_snapshot
                .latest_analyze
                .as_ref()
                .and_then(|phase| {
                    if phase.canonical_structural_probabilities.is_empty() {
                        None
                    } else {
                        Some(ict_engine::state::CanonicalStructuralRegimePosterior {
                            active_regime: phase.canonical_structural_active_regime.clone(),
                            confidence: phase.canonical_structural_confidence,
                            probabilities: phase.canonical_structural_probabilities.clone(),
                            evidence: Vec::new(),
                        })
                    }
                }),
            artifact_decision_summary: report.artifact_decision_summary.clone(),
            artifact_decision_section: report.artifact_decision_section.clone(),
            agent_prompts: report.agent_prompts.clone(),
            prompt_workflow: report.agent_prompts.workflow.clone(),
            multi_timeframe_summary: report.multi_timeframe_summary.clone(),
            objective_market_credibility_shrink:
                factor_backtest_objective_market_credibility_shrink.clone(),
        },
    )?;
    persist_market_jump_calibration_from_backtest_runs(
        state_dir,
        symbol,
        &backtest_runs,
        None,
        None,
    )?;
    report.workflow_snapshot = refresh_workflow_snapshot(state_dir, symbol)?;
    let backtest_support_hint = crate::analyze_shared::structural_support_hint_for_backtest(
        crate::analyze_shared::BacktestStructuralSupportInput {
            baseline_composite_score: report.scorecards.first().map(|score| score.composite_score),
            aggregate_return: report.aggregate_return,
            execution_readiness: backtest_execution_fields.execution_readiness,
            comparable_to_previous: report.dataset_comparability.comparable,
            feedback_records_applied: report.feedback_records_applied,
            conformal_coverage_1sigma: report
                .factor_results
                .first()
                .map(|result| result.metrics.conformal_coverage_1sigma),
            regime_break_penalty: report
                .factor_results
                .first()
                .map(|result| result.metrics.regime_break_penalty),
            structural_break_detected: report
                .factor_results
                .first()
                .map(|result| result.metrics.structural_break_detected),
            quality_delta: first_score_delta,
        },
    );
    let backtest_support_hint = crate::analyze_shared::offline_structural_support_hint(
        crate::analyze_shared::OfflineStructuralSupportHintInput {
            artifact_validation_bias: Some(
                crate::analyze_shared::artifact_validation_support_bias(
                    &report.workflow_snapshot.artifact_decision_summary,
                ),
            ),
            baseline_support: backtest_support_hint,
            ..crate::analyze_shared::OfflineStructuralSupportHintInput::default()
        },
    );
    crate::analyze_shared::apply_offline_structural_prior_seed(
        &mut learning_state,
        &report.workflow_snapshot,
        &format!("structural-prior-seed:{}", run_id),
        run_timestamp,
        backtest_support_hint,
        "backtest_run_structural_prior_seed",
    );
    save_learning_state(state_dir, symbol, &learning_state)?;
    report.artifact_decision_summary = artifact_decision_summary_from_snapshot(
        &report.workflow_snapshot,
        &report.artifact_action_summary,
    );
    report.artifact_decision_section =
        artifact_decision_section_from_snapshot(&report.workflow_snapshot);
    report.artifact_decision_section =
        artifact_decision_section_from_snapshot(&report.workflow_snapshot);
    link_artifact_decision_summary_to_decisions(
        &report.artifact_decision_summary,
        &mut report.promotion_decision,
        &mut report.rollback_recommendation,
    );

    Ok(report)
}
