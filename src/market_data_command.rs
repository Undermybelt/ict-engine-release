use super::*;

pub(crate) fn clean_futures_shell(
    root: Option<&str>,
    output_dir: &str,
    interval: &str,
    multi_timeframe: bool,
) -> Result<()> {
    ict_engine::application::data_sources::clean_futures_command(
        root,
        output_dir,
        interval,
        multi_timeframe,
        run_clean_futures_multi_timeframe,
        run_clean_futures,
    )
}

pub(crate) fn futures_sop_shell(
    root: Option<&str>,
    output_dir: &str,
    interval: &str,
) -> Result<()> {
    ict_engine::application::data_sources::futures_sop_command(
        root,
        output_dir,
        interval,
        run_futures_sop,
    )
}

pub(crate) fn expansion_sop_shell(
    input: ict_engine::application::data_sources::ExpansionSopCommandInput<'_>,
) -> Result<()> {
    let interval = input.interval.to_string();
    ict_engine::application::data_sources::expansion_sop_command(
        input,
        parse_research_objective,
        load_factor_mutation_spec,
        run_expansion_sop,
        move |report, mutation_spec, emit_mutation_evaluation| {
            if emit_mutation_evaluation {
                let next_mutation_spec_template = report
                    .factor_mutation_evaluation
                    .as_ref()
                    .map(|evaluation| next_mutation_spec_template(mutation_spec, evaluation, true));
                Ok(serde_json::json!({
                    "mutation_spec": mutation_spec,
                    "factor_mutation_evaluation": report.factor_mutation_evaluation,
                    "next_mutation_spec_template": next_mutation_spec_template,
                    "recommended_global_factor": report.recommended_global_factor,
                    "recommended_global_pre_bayes_summary": report.recommended_global_pre_bayes_summary,
                    "recommended_commands": report.recommended_commands,
                }))
            } else {
                let compact_report = build_backtest_result_artifact(BacktestResultArtifactInput {
                    summary: format!("expansion_sop:{}", interval),
                    scorecards: report
                        .recommended_market_factors
                        .iter()
                        .map(|(market, factor)| format!("{}:{}", market, factor))
                        .collect::<Vec<_>>(),
                    shrink_comparison_summary: vec![],
                    duration_sizing_delta_surface: vec![],
                    oos_quality_delta_surface: vec![],
                    market_breakdown: vec![format!(
                        "recommended_global_factor={:?}",
                        report.recommended_global_factor
                    )],
                    regime_breakdown: vec![],
                    window_breakdown: vec![],
                    comparable: true,
                    artifacts: report.recommended_commands.clone(),
                });
                let factor_lifecycle = build_factor_lifecycle_view(
                    report.mutation_spec.as_ref(),
                    report.factor_mutation_evaluation.as_ref(),
                    &PromotionDecision {
                        approved: report.recommended_global_factor.is_some(),
                        status: if report.recommended_global_factor.is_some() {
                            "promote".to_string()
                        } else {
                            "hold".to_string()
                        },
                        reason: "expansion_sop_global_selection".to_string(),
                        target_factors: report.recommended_global_factor.iter().cloned().collect(),
                        target_families: vec![],
                    },
                    &RollbackRecommendation {
                        should_rollback: false,
                        scope: "none".to_string(),
                        reason: "no_global_rollback".to_string(),
                        target_factors: vec![],
                        target_families: vec![],
                    },
                );
                Ok(serde_json::json!({
                    "report": report,
                    "compact_backtest_report": compact_report,
                    "factor_lifecycle": factor_lifecycle,
                }))
            }
        },
    )
}

pub(crate) fn market_data_harness_shell(
    action: &str,
    input: MarketDataHarnessCommandInput<'_>,
) -> Result<()> {
    match action.trim().to_ascii_lowercase().as_str() {
        "plan" => market_data_harness_plan_command(input),
        "fetch" => market_data_harness_fetch_command(input),
        other => anyhow::bail!("unsupported market-data-harness action '{}'", other),
    }
}
