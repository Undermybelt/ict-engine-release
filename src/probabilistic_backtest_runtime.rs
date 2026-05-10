use super::*;

pub(crate) fn run_probabilistic_backtest(
    input: RunProbabilisticBacktestInput<'_>,
) -> Result<(
    BacktestReport,
    ict_engine::bbn::BayesianNetwork,
    Vec<TradeRecord>,
)> {
    let RunProbabilisticBacktestInput {
        symbol,
        state_dir,
        candles,
        paired_candles,
        warmup_bars,
        hold_bars,
        realism,
        online_learn,
        params,
        network,
        learning_state,
    } = input;
    let feedback_run_id = format!(
        "probabilistic-backtest-feedback:{}:{}",
        symbol,
        Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
    );
    let feedback_data_fingerprint =
        data_fingerprint(candles, paired_candles, "probabilistic_backtest_feedback");
    let minimum_history = warmup_bars.max(INDICATOR_PERIOD * 2 + 1);
    if candles.len() <= minimum_history + hold_bars {
        bail!(
            "need more candles for backtest: got {}, require at least {}",
            candles.len(),
            minimum_history + hold_bars + 1
        );
    }
    if hold_bars == 0 {
        bail!("hold_bars must be greater than zero");
    }

    let mut trades = Vec::new();
    let mut signals = 0usize;
    let mut learning_updates = 0usize;
    let mut last_decision = None;
    let last_signal_index = candles.len().saturating_sub(hold_bars + 1);
    let mut working_network = network.clone();
    let mut feedback_records = Vec::new();
    let mut bbn_feedback = Vec::new();

    for signal_index in (minimum_history - 1)..=last_signal_index {
        let window = &candles[..=signal_index];
        let analysis = build_analyze_report(BuildAnalyzeReportInput {
            symbol,
            state_dir,
            htf: window,
            mtf: window,
            ltf: window,
            params,
            network: &working_network,
            build_context: AnalyzeBuildContext {
                symbol,
                paired_candles: paired_candles.and_then(|series| {
                    if series.is_empty() {
                        None
                    } else {
                        Some(&series[..=signal_index.min(series.len().saturating_sub(1))])
                    }
                }),
                auxiliary: None,
                learning_state,
                multi_timeframe_summary: &[],
                native_frames: AnalyzeNativeFrames::default(),
            },
            regime_bundle_adapter: None,
            apply_regime_bundle_bbn_soft_evidence: false,
            execution_focus: true,
        })?;
        last_decision = Some(analysis.supporting.decision.clone());

        if analysis.supporting.raw_trade_plan.direction == Direction::Neutral
            || analysis.supporting.raw_trade_plan.kelly_fraction <= 0.0
        {
            continue;
        }

        signals += 1;

        if let Some(simulated) = BacktestEngine::simulate_trade_with_realism(
            candles,
            signal_index,
            &analysis.supporting.raw_trade_plan,
            hold_bars,
            realism,
        ) {
            trades.push(TradeRecord {
                timestamp: candles[simulated.entry_index].timestamp,
                symbol: parse_symbol(symbol),
                direction: analysis.supporting.raw_trade_plan.direction,
                entry_price: simulated.entry_price,
                exit_price: simulated.exit_price,
                pnl: simulated.pnl,
                exit_reason: Some(format!("{:?}", simulated.exit_reason)),
                regime_at_entry: analysis.supporting.model_state.regime_probs.dominant(),
                cascade_max_layer: selected_cascade_max_layer(&analysis.supporting.raw_trade_plan),
                cascade_direction: analysis.supporting.raw_trade_plan.direction,
                factor_values: decision_factor_values(
                    &analysis.supporting.decision,
                    &analysis.supporting.raw_trade_plan,
                    &analysis.supporting.factor_diagnostics,
                ),
            });

            feedback_records.push(enrich_feedback_record(
                build_feedback_record(BuildFeedbackRecordInput {
                    symbol,
                    source: "probabilistic_backtest",
                    timestamp: candles[simulated.entry_index].timestamp,
                    factor_diagnostics: &analysis.supporting.factor_diagnostics,
                    decision: &analysis.supporting.decision,
                    pnl: simulated.pnl,
                    realized_outcome: trade_outcome_label_from_pnl(simulated.pnl),
                    regime_at_entry: analysis.supporting.model_state.regime_probs.dominant(),
                }),
                &feedback_run_id,
                format!(
                    "{}:{}:{}",
                    symbol,
                    candles[simulated.entry_index].timestamp.to_rfc3339(),
                    candles[simulated.exit_index].timestamp.to_rfc3339()
                ),
                learning_state,
                &feedback_data_fingerprint,
            ));

            let outcome_label = trade_outcome_label_from_pnl(simulated.pnl);
            let evidence = trade_evidence_from_labels(
                &working_network,
                &[
                    (
                        "entry_quality",
                        analysis.supporting.entry_quality.selected_state.as_str(),
                    ),
                    (
                        "factor_alignment",
                        analysis
                            .supporting
                            .factor_diagnostics
                            .alignment_label
                            .as_str(),
                    ),
                    (
                        "factor_uncertainty",
                        analysis
                            .supporting
                            .factor_diagnostics
                            .uncertainty_label
                            .as_str(),
                    ),
                ],
            )?;
            let realized_state_index = working_network
                .nodes
                .get("trade_outcome")
                .and_then(|node| node.state_index(&outcome_label))
                .ok_or_else(|| anyhow!("unknown trade outcome state '{}'", outcome_label))?;

            if online_learn {
                CPTUpdater::default().update_from_trade(
                    &mut working_network,
                    &evidence,
                    TradeOutcome {
                        node_id: "trade_outcome".into(),
                        realized_state_index,
                    },
                )?;
                if let Some(last_feedback) = feedback_records.last() {
                    let new_feedback =
                        learning_state.merge_feedback_records(std::slice::from_ref(last_feedback));
                    WeightUpdater::default().apply_feedback(learning_state, &new_feedback);
                }
                learning_updates += 1;
            } else {
                bbn_feedback.push((
                    evidence,
                    TradeOutcome {
                        node_id: "trade_outcome".to_string(),
                        realized_state_index,
                    },
                ));
            }
        }
    }

    if !bbn_feedback.is_empty() && !online_learn {
        CPTUpdater::default().batch_update(&mut working_network, &bbn_feedback)?;
        learning_updates = bbn_feedback.len();
        let new_feedback = learning_state.merge_feedback_records(&feedback_records);
        WeightUpdater::default().apply_feedback(learning_state, &new_feedback);
    }

    let report = ict_engine::application::backtest::build_runtime_backtest_report(
        ict_engine::application::backtest::BuildRuntimeBacktestReportInput {
            symbol,
            state_dir,
            bars: candles.len(),
            warmup_bars: minimum_history,
            hold_bars,
            spread_bps: realism.spread_bps,
            slippage_bps: realism.slippage_bps,
            fee_bps: realism.fee_bps,
            ambiguous_bar_policy: ambiguous_bar_policy_label(realism.ambiguous_bar_policy),
            online_learning: online_learn,
            learning_updates,
            signals,
            trades: &trades,
            learning_state,
            network: &working_network,
            last_decision,
        },
    )?;

    Ok((report, working_network, trades))
}

pub(crate) fn finalize_backtest_report(
    input: FinalizeBacktestReportInput<'_>,
) -> Result<BacktestReport> {
    let FinalizeBacktestReportInput {
        report,
        symbol,
        data,
        paired_data,
        candles,
        paired_candles_slice,
        learning_state,
        previous_rankings,
        previous_trade_outcome_cpt,
        updated_network,
        state_dir,
        warmup_bars,
        hold_bars,
        realism,
        online_learning,
    } = input;
    let mut report = report;
    let previous_runs: Vec<BacktestRunRecord> =
        load_state_or_default(state_dir, symbol, BACKTEST_RUNS_FILE)?;
    let score_deltas = ranking_diffs(previous_rankings, &report.factor_ranking);
    let final_trade_outcome_cpt =
        ict_engine::application::backtest::trade_outcome_cpt_snapshot(updated_network)?;
    let probability_deltas =
        cpt_probability_diffs(previous_trade_outcome_cpt, &final_trade_outcome_cpt);
    report.provenance = run_provenance(
        learning_state,
        &[
            "backtest",
            data,
            paired_data.unwrap_or(""),
            &warmup_bars.to_string(),
            &hold_bars.to_string(),
            &format!("spread_bps={:.4}", realism.spread_bps),
            &format!("slippage_bps={:.4}", realism.slippage_bps),
            &format!("fee_bps={:.4}", realism.fee_bps),
            &ambiguous_bar_policy_label(realism.ambiguous_bar_policy),
            &online_learning.to_string(),
        ],
        data_fingerprint(candles, paired_candles_slice, "backtest"),
    );
    let dataset_comparability = dataset_comparability(
        previous_runs.last().map(|run| run.run_id.clone()),
        previous_runs.last().map(|run| &run.provenance),
        &report.provenance,
    );
    let artifact_consumed_gate = artifact_consumed_decision_gate(
        &artifact_consumed_impact_summary_for_symbol(state_dir, symbol)?,
    );
    let (_, artifact_family_trends) = artifact_trend_summaries_for_symbol(state_dir, symbol)?;
    let surfaces = ict_engine::application::backtest::derive_finalize_backtest_decision_surfaces(
        ict_engine::application::backtest::FinalizeBacktestDecisionSurfacesInput {
            previous_runs: &previous_runs,
            factor_ranking: &report.factor_ranking,
            factor_family_decisions: &report.factor_family_decisions,
            score_deltas: &score_deltas,
            probability_deltas: &probability_deltas,
            dataset_comparability: &dataset_comparability,
            artifact_consumed_gate: &artifact_consumed_gate,
            artifact_family_trends: &artifact_family_trends,
        },
    );
    let thresholds = surfaces.decision_thresholds.clone();
    let mut promotion_decision = surfaces.promotion_decision;
    let mut rollback_recommendation = surfaces.rollback_recommendation;
    let factor_family_outcomes = surfaces.factor_family_outcomes;
    let factor_family_diffs = surfaces.factor_family_diffs;
    let decision_history_summary = surfaces.decision_history_summary;
    let factor_family_history = surfaces.factor_family_history;
    let mut agent_action_plan = build_agent_action_plan(
        "backtest_review_ready",
        &promotion_decision,
        &rollback_recommendation,
        &report.factor_iteration_queue,
        &factor_family_outcomes,
    );
    let artifact_surfaces =
        ict_engine::application::backtest::load_finalize_backtest_artifact_surfaces(
            state_dir, symbol,
        )?;
    augment_action_plan_with_artifact_trends(
        &mut agent_action_plan,
        symbol,
        state_dir,
        &artifact_surfaces.factor_trends,
        &artifact_surfaces.family_trends,
        &artifact_surfaces.consumed_impact_summary,
    );
    let artifact_action_summary = artifact_surfaces.action_summary;
    let artifact_decision_summary = artifact_surfaces.decision_summary;
    let artifact_decision_section = artifact_surfaces.decision_section;
    ict_engine::application::backtest::link_artifact_decision_summary_to_decisions(
        &artifact_decision_summary,
        &mut promotion_decision,
        &mut rollback_recommendation,
    );
    let workflow_state = workflow_state_from_context(
        "backtest_review_ready",
        &promotion_decision,
        &rollback_recommendation,
    );
    report.objective_market_credibility_shrink = report
        .objective_market_credibility_shrink
        .clone()
        .or_else(|| {
            report
                .workflow_snapshot
                .latest_analyze
                .as_ref()
                .and_then(|snapshot| snapshot.objective_market_credibility_shrink.clone())
        });
    let recommended_commands = command_recommendations(&CommandContext {
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
        update_feedback_file: None,
        user_data_selection_required: true,
    });
    concretize_action_plan_commands(&mut agent_action_plan, &recommended_commands);
    let recommended_next_command =
        recommended_next_command(&agent_action_plan, &recommended_commands);
    let agent_context_bundle = build_agent_context_bundle(BuildAgentContextBundleInput {
        symbol,
        state_dir,
        workflow_state: &workflow_state,
        decision_hint: "backtest_review_ready",
        recommended_next_command: &recommended_next_command,
        recommended_commands: &recommended_commands,
        dataset_comparability: &dataset_comparability,
        factor_iteration_queue: &report.factor_iteration_queue,
        family_outcomes: &factor_family_outcomes,
        pre_bayes_evidence_filter: None,
        pre_bayes_entry_quality_bridge: None,
        pda_sequence_summary: None,
        factor_mutation_evaluation: None,
        artifact_decision_summary: Some(&artifact_decision_summary),
    });
    let agent_context_bundle_minimal = build_agent_context_bundle_minimal(&agent_context_bundle);
    let dataset_audit_prompt = dataset_audit_prompt(
        symbol,
        data,
        paired_data,
        candles.len(),
        paired_candles_slice.map(|items| items.len()),
        "backtest",
    );
    let promotion_gate_prompt =
        promotion_gate_prompt(symbol, &report.factor_ranking, &score_deltas, &thresholds);
    let rollback_review_prompt =
        rollback_review_prompt(symbol, &score_deltas, &probability_deltas, &thresholds);
    ict_engine::application::backtest::apply_finalize_backtest_enrichment(
        ict_engine::application::backtest::FinalizeBacktestEnrichmentInput {
            report: &mut report,
            decision_thresholds: thresholds.clone(),
            dataset_comparability,
            promotion_decision,
            rollback_recommendation,
            factor_family_outcomes,
            factor_family_diffs,
            factor_family_history,
            decision_history_summary,
            agent_action_plan,
            workflow_state,
            artifact_action_summary,
            artifact_decision_summary,
            artifact_decision_section,
            recommended_commands,
            recommended_next_command,
            agent_context_bundle,
            agent_context_bundle_minimal,
            score_deltas: score_deltas.clone(),
            probability_deltas: probability_deltas.clone(),
            final_trade_outcome_cpt: final_trade_outcome_cpt.clone(),
            dataset_audit_prompt,
            promotion_gate_prompt,
            rollback_review_prompt,
        },
    );
    ict_engine::application::backtest::persist_finalized_backtest_run(
        ict_engine::application::backtest::PersistFinalizedBacktestRunInput {
            report: &report,
            symbol,
            state_dir,
            data,
            paired_data,
            candles: candles.len(),
            paired_candles: paired_candles_slice.map(|items| items.len()),
            warmup_bars,
            hold_bars,
            online_learning,
        },
    )?;
    report.workflow_snapshot = refresh_workflow_snapshot(state_dir, symbol)?;

    Ok(report)
}
