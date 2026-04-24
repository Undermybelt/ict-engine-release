use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn analyze_command(
    symbol: &str,
    data_htf: &str,
    data_mtf: &str,
    data_ltf: &str,
    state_dir: &str,
    output_format: OutputFormat,
    inline_ledger: bool,
    execution_focus: bool,
) -> Result<()> {
    let _ = migrate_ensemble_executor_scorecards(state_dir, symbol)?;
    let htf = load_candles(data_htf)?;
    let mtf = load_candles(data_mtf)?;
    let ltf = load_candles(data_ltf)?;
    let resolved_multi_timeframe_inputs =
        resolve_analyze_multi_timeframe_inputs(data_htf, data_mtf, data_ltf);
    let d1_owned = resolved_multi_timeframe_inputs
        .get("1d")
        .filter(|path| *path != data_htf && *path != data_mtf && *path != data_ltf)
        .map(load_candles)
        .transpose()?;
    let h4_owned = resolved_multi_timeframe_inputs
        .get("4h")
        .filter(|path| *path != data_htf && *path != data_mtf && *path != data_ltf)
        .map(load_candles)
        .transpose()?;
    let h1_owned = resolved_multi_timeframe_inputs
        .get("1h")
        .filter(|path| *path != data_htf && *path != data_mtf && *path != data_ltf)
        .map(load_candles)
        .transpose()?;
    let m15_owned = resolved_multi_timeframe_inputs
        .get("15m")
        .filter(|path| *path != data_htf && *path != data_mtf && *path != data_ltf)
        .map(load_candles)
        .transpose()?;
    let m5_owned = resolved_multi_timeframe_inputs
        .get("5m")
        .filter(|path| *path != data_htf && *path != data_mtf && *path != data_ltf)
        .map(load_candles)
        .transpose()?;
    let m1_owned = resolved_multi_timeframe_inputs
        .get("1m")
        .filter(|path| *path != data_htf && *path != data_mtf && *path != data_ltf)
        .map(load_candles)
        .transpose()?;
    let multi_timeframe_summary =
        build_multi_timeframe_summary(data_ltf, &resolved_multi_timeframe_inputs)?;
    let multi_timeframe_signal =
        build_multi_timeframe_research_signal(&resolved_multi_timeframe_inputs)?;
    let analyze_multi_timeframe_summary = multi_timeframe_summary
        .iter()
        .chain(multi_timeframe_signal.summary.iter())
        .cloned()
        .collect::<Vec<_>>();
    let params = load_or_init_hmm_params(symbol, state_dir);
    let network = load_or_init_trading_network(symbol, state_dir)?;
    let learning_state = load_learning_state(state_dir, symbol)?;
    let report = build_analyze_report(BuildAnalyzeReportInput {
        symbol,
        state_dir,
        htf: &htf,
        mtf: &mtf,
        ltf: &ltf,
        params: &params,
        network: &network,
        build_context: AnalyzeBuildContext {
            symbol,
            paired_candles: None,
            auxiliary: None,
            learning_state: &learning_state,
            multi_timeframe_summary: &analyze_multi_timeframe_summary,
            native_frames: AnalyzeNativeFrames {
                d1: if infer_interval_for_analyze_frame(data_htf, "1d") == "1d" {
                    Some(&htf)
                } else if infer_interval_for_analyze_frame(data_mtf, "1h") == "1d" {
                    Some(&mtf)
                } else if infer_interval_for_analyze_frame(data_ltf, "15m") == "1d" {
                    Some(&ltf)
                } else {
                    d1_owned.as_deref()
                },
                h4: if infer_interval_for_analyze_frame(data_htf, "1d") == "4h" {
                    Some(&htf)
                } else if infer_interval_for_analyze_frame(data_mtf, "1h") == "4h" {
                    Some(&mtf)
                } else if infer_interval_for_analyze_frame(data_ltf, "15m") == "4h" {
                    Some(&ltf)
                } else {
                    h4_owned.as_deref()
                },
                h1: if infer_interval_for_analyze_frame(data_htf, "1d") == "1h" {
                    Some(&htf)
                } else if infer_interval_for_analyze_frame(data_mtf, "1h") == "1h" {
                    Some(&mtf)
                } else if infer_interval_for_analyze_frame(data_ltf, "15m") == "1h" {
                    Some(&ltf)
                } else {
                    h1_owned.as_deref()
                },
                m15: if infer_interval_for_analyze_frame(data_htf, "1d") == "15m" {
                    Some(&htf)
                } else if infer_interval_for_analyze_frame(data_mtf, "1h") == "15m" {
                    Some(&mtf)
                } else if infer_interval_for_analyze_frame(data_ltf, "15m") == "15m" {
                    Some(&ltf)
                } else {
                    m15_owned.as_deref()
                },
                m5: if infer_interval_for_analyze_frame(data_htf, "1d") == "5m" {
                    Some(&htf)
                } else if infer_interval_for_analyze_frame(data_mtf, "1h") == "5m" {
                    Some(&mtf)
                } else if infer_interval_for_analyze_frame(data_ltf, "15m") == "5m" {
                    Some(&ltf)
                } else {
                    m5_owned.as_deref()
                },
                m1: if infer_interval_for_analyze_frame(data_htf, "1d") == "1m" {
                    Some(&htf)
                } else if infer_interval_for_analyze_frame(data_mtf, "1h") == "1m" {
                    Some(&mtf)
                } else if infer_interval_for_analyze_frame(data_ltf, "15m") == "1m" {
                    Some(&ltf)
                } else {
                    m1_owned.as_deref()
                },
            },
        },
        execution_focus,
    })?;
    let mut report = report;
    let pending_update_file =
        persist_pending_update_artifact_from_analyze(state_dir, &report, "analyze")?;
    let _execution_candidate_file =
        persist_execution_candidate_from_analyze(state_dir, &report, "analyze")?;
    let (artifact_factor_trends, artifact_family_trends) =
        artifact_trend_summaries_for_symbol(state_dir, symbol)?;
    let artifact_consumed_impact_summary =
        artifact_consumed_impact_summary_for_symbol(state_dir, symbol)?;
    augment_action_plan_with_artifact_trends(
        &mut report.supporting.agent_action_plan,
        symbol,
        state_dir,
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_consumed_impact_summary,
    );
    report.supporting.artifact_action_summary = artifact_action_summary(
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_consumed_impact_summary,
    );
    if let Ok(artifact) = ict_engine::pda_sequence::load_pda_sequence_analysis(state_dir, symbol) {
        let summary = ict_engine::pda_sequence::summarize_pda_sequence_artifact(&artifact);
        report.supporting.artifact_action_summary.push(format!(
            "pda_sequence:{} confidence={:.3} consistency={:.3}",
            summary
                .primary_cluster_label
                .unwrap_or_else(|| "unknown".to_string()),
            summary.primary_cluster_confidence.unwrap_or_default(),
            summary.consistency_ratio,
        ));
    }
    report.supporting.artifact_decision_summary =
        artifact_decision_summary_for_symbol(state_dir, symbol)?;
    report.supporting.artifact_decision_section = artifact_decision_section_from_parts(
        &report.supporting.artifact_decision_summary,
        &report.supporting.artifact_action_summary,
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_rule_break_effects_for_symbol(state_dir, symbol)?,
        &artifact_consumed_impact_summary,
    );
    apply_command_context_to_analyze_report(
        &mut report,
        &CommandContext {
            symbol: symbol.to_string(),
            state_dir: state_dir.to_string(),
            analyze: Some(AnalyzeCommandSource::Files {
                data_htf: data_htf.to_string(),
                data_mtf: data_mtf.to_string(),
                data_ltf: data_ltf.to_string(),
            }),
            research_data: Some(data_ltf.to_string()),
            paired_data: None,
            update_outcome: None,
            update_entry_signal: None,
            update_feedback_file: Some(pending_update_file),
            user_data_selection_required: true,
        },
    );
    report.supporting.workflow_snapshot = persist_analyze_run(
        state_dir,
        &report,
        "analyze",
        Some(data_htf),
        Some(data_mtf),
        Some(data_ltf),
        None,
    )?;
    report.supporting.artifact_decision_summary = artifact_decision_summary_from_snapshot(
        &report.supporting.workflow_snapshot,
        &report.supporting.artifact_action_summary,
    );
    report.supporting.artifact_decision_section =
        artifact_decision_section_from_snapshot(&report.supporting.workflow_snapshot);
    append_artifact_decision_prompt(
        &mut report.supporting.agent_prompts,
        symbol,
        &report.supporting.artifact_decision_section,
    );
    link_artifact_decision_summary_to_decisions(
        &report.supporting.artifact_decision_summary,
        &mut report.supporting.promotion_decision,
        &mut report.supporting.rollback_recommendation,
    );

    emit_analyze_output(&report, output_format, inline_ledger)
}
