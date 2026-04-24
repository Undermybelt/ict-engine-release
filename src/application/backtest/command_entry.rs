use anyhow::Result;

use crate::application::backtest::{
    build_backtest_compare_report, build_backtest_result_artifact,
    build_duration_sizing_delta_surface, build_oos_quality_delta_surface,
    build_shrink_on_off_comparison_summary, BacktestResultArtifactInput,
};
use crate::application::decision_utils::{parse_research_objective, ResearchObjectiveMode};
use crate::application::factor_lifecycle::{
    build_factor_lifecycle_view, factor_specific_hint_preferences,
    next_mutation_spec_template_with_preferences,
};
use crate::application::orchestration::resolved_vote_scorecards;
use crate::application::reflection::build_research_reflection_bundle;
use crate::config::shell_quote;
use crate::state::{
    load_ensemble_executor_scorecards, load_state_or_default, migrate_ensemble_executor_scorecards,
    BacktestRunRecord, FactorMutationSpec, ResearchRunRecord, WorkflowSnapshot, BACKTEST_RUNS_FILE,
    RESEARCH_RUNS_FILE,
};

fn latest_duration_phase(
    snapshot: &WorkflowSnapshot,
) -> Option<&crate::state::WorkflowPhaseSnapshot> {
    snapshot
        .latest_backtest
        .as_ref()
        .or(snapshot.latest_research.as_ref())
        .or(snapshot.latest_update.as_ref())
        .or(snapshot.latest_analyze.as_ref())
}

fn parse_duration_sizing_scale(summary: &[String]) -> Option<f64> {
    summary.iter().find_map(|line| {
        line.split_whitespace().find_map(|fragment| {
            fragment
                .strip_prefix("duration_sizing_scale=")
                .and_then(|value| value.parse::<f64>().ok())
        })
    })
}

fn build_duration_surface_from_artifacts(
    snapshot: &WorkflowSnapshot,
    artifact_action_summary: &[String],
) -> Vec<String> {
    let phase = latest_duration_phase(snapshot);
    let duration_model = phase.and_then(|phase| phase.hybrid_duration_model.as_deref());
    let remaining_expected_bars = phase.and_then(|phase| phase.hybrid_remaining_expected_bars);
    let scale = parse_duration_sizing_scale(artifact_action_summary).unwrap_or(1.0);
    build_duration_sizing_delta_surface(
        1.0,
        scale,
        1.0,
        scale,
        duration_model,
        remaining_expected_bars,
    )
}

pub fn factor_backtest_command<FRun>(
    symbol: &str,
    data: &str,
    paired_data: Option<&str>,
    ensemble: bool,
    state_dir: &str,
    output_format: &str,
    run_backtest: FRun,
) -> Result<()>
where
    FRun: Fn(&str, &str, Option<&str>, &str) -> Result<crate::factor_lab::BacktestResult>,
{
    let report = run_backtest(symbol, data, paired_data, state_dir)?;
    let credibility_summary = serde_json::json!({
        "conformal_coverage_1sigma": report
            .factor_results
            .iter()
            .map(|result| (result.factor_name.clone(), result.metrics.conformal_coverage_1sigma))
            .collect::<Vec<_>>(),
        "conformal_miscoverage_1sigma": report
            .factor_results
            .iter()
            .map(|result| (result.factor_name.clone(), result.metrics.conformal_miscoverage_1sigma))
            .collect::<Vec<_>>(),
        "regime_break_penalty": report
            .factor_results
            .iter()
            .map(|result| (result.factor_name.clone(), result.metrics.regime_break_penalty))
            .collect::<Vec<_>>(),
        "structural_break_score": report
            .factor_results
            .iter()
            .map(|result| (result.factor_name.clone(), result.metrics.structural_break_score))
            .collect::<Vec<_>>(),
        "structural_break_detected": report
            .factor_results
            .iter()
            .map(|result| (result.factor_name.clone(), result.metrics.structural_break_detected))
            .collect::<Vec<_>>(),
        "structural_break_index": report
            .factor_results
            .iter()
            .map(|result| (result.factor_name.clone(), result.metrics.structural_break_index))
            .collect::<Vec<_>>(),
    });
    let shrink_comparison_summary = build_shrink_on_off_comparison_summary(
        report
            .factor_results
            .first()
            .map(|result| result.metrics.conformal_coverage_1sigma)
            .unwrap_or_default(),
        report
            .factor_results
            .first()
            .map(|result| {
                (result.metrics.conformal_coverage_1sigma + result.metrics.regime_break_penalty)
                    .clamp(0.0, 1.0)
            })
            .unwrap_or_default(),
        report.aggregate_return,
        report.aggregate_return
            + report
                .factor_results
                .first()
                .map(|result| result.metrics.regime_break_penalty)
                .unwrap_or_default(),
    );
    let oos_quality_delta_surface = build_oos_quality_delta_surface(
        report
            .factor_results
            .first()
            .map(|result| result.metrics.conformal_coverage_1sigma)
            .unwrap_or_default(),
        report
            .factor_results
            .first()
            .map(|result| {
                (result.metrics.conformal_coverage_1sigma - result.metrics.regime_break_penalty)
                    .clamp(0.0, 1.0)
            })
            .unwrap_or_default(),
        report
            .factor_results
            .iter()
            .map(|result| result.metrics.trade_count)
            .sum(),
        report
            .factor_results
            .iter()
            .map(|result| result.metrics.trade_count)
            .sum(),
    );
    let duration_sizing_delta_surface = build_duration_surface_from_artifacts(
        &report.workflow_snapshot,
        &report.artifact_action_summary,
    );
    let compact_report = build_backtest_result_artifact(BacktestResultArtifactInput {
        summary: format!("factor_backtest:{}", symbol),
        scorecards: report
            .scorecards
            .iter()
            .map(|item| format!("{}:{:.3}", item.factor_name, item.composite_score))
            .collect::<Vec<_>>(),
        shrink_comparison_summary,
        duration_sizing_delta_surface,
        oos_quality_delta_surface,
        market_breakdown: vec![
            format!("best_factor={:?}", report.best_factor),
            format!(
                "coverage_1sigma={:.3}",
                report
                    .factor_results
                    .first()
                    .map(|result| result.metrics.conformal_coverage_1sigma)
                    .unwrap_or_default()
            ),
            format!(
                "regime_break_penalty={:.3}",
                report
                    .factor_results
                    .first()
                    .map(|result| result.metrics.regime_break_penalty)
                    .unwrap_or_default()
            ),
            format!(
                "structural_break_detected={}",
                report
                    .factor_results
                    .first()
                    .map(|result| result.metrics.structural_break_detected)
                    .unwrap_or(false)
            ),
            format!(
                "structural_break_score={:.3}",
                report
                    .factor_results
                    .first()
                    .map(|result| result.metrics.structural_break_score)
                    .unwrap_or_default()
            ),
        ],
        regime_breakdown: vec![],
        window_breakdown: vec![],
        comparable: true,
        artifacts: vec![],
    });
    let persisted_backtest_runs: Vec<BacktestRunRecord> =
        load_state_or_default(state_dir, symbol, BACKTEST_RUNS_FILE)?;
    let backtest_compare_report =
        persisted_backtest_runs
            .split_last()
            .and_then(|(current, previous)| {
                previous
                    .last()
                    .and_then(|prior| build_backtest_compare_report(prior, current))
            });
    let ensemble_surface = if ensemble {
        report
            .workflow_snapshot
            .latest_ensemble_vote
            .as_ref()
            .map(|vote| {
                let persisted_scorecards =
                    load_ensemble_executor_scorecards(state_dir, symbol).unwrap_or_default();
                let (scorecards, scorecard_source) =
                    resolved_vote_scorecards(&persisted_scorecards, vote);
                serde_json::json!({
                    "ensemble_vote": vote,
                    "executor_scorecards": scorecards,
                    "executor_scorecard_source": scorecard_source,
                })
            })
    } else {
        None
    };
    let suggested_update_command = if !report.recommended_commands.update.command.is_empty()
        && report.recommended_commands.update.command != "recommended_command_unavailable"
    {
        report.recommended_commands.update.command.clone()
    } else {
        format!(
            "ict-engine update --symbol {} --outcome <win|loss|breakeven> --state-dir {}",
            shell_quote(symbol),
            shell_quote(state_dir)
        )
    };
    let payload = crate::application::reporting::build_factor_backtest_output_payload(
        &report,
        &compact_report,
        backtest_compare_report,
        credibility_summary,
        ensemble_surface,
        &suggested_update_command,
    );
    crate::application::reporting::emit_structured_output_payload(
        output_format,
        &payload,
        &compact_report,
    )?;
    Ok(())
}

pub struct FactorResearchCommandInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub objective: &'a str,
    pub mutation_spec_path: Option<&'a str>,
    pub emit_mutation_evaluation: bool,
    pub ensemble: bool,
    pub state_dir: &'a str,
    pub output_format: &'a str,
}

pub fn factor_research_command<FLoad, FRun>(
    input: FactorResearchCommandInput<'_>,
    load_mutation_spec: FLoad,
    run_research: FRun,
) -> Result<()>
where
    FLoad: Fn(&str) -> Result<FactorMutationSpec>,
    FRun: Fn(
        ResearchObjectiveMode,
        Option<&FactorMutationSpec>,
    ) -> Result<crate::factor_lab::research::ResearchReport>,
{
    let FactorResearchCommandInput {
        symbol,
        data: _,
        objective,
        mutation_spec_path,
        emit_mutation_evaluation,
        ensemble,
        state_dir,
        output_format,
    } = input;
    let _ = migrate_ensemble_executor_scorecards(state_dir, symbol)?;
    let objective = parse_research_objective(objective)?;
    let mutation_spec = mutation_spec_path.map(load_mutation_spec).transpose()?;
    let report = run_research(objective, mutation_spec.as_ref())?;
    if emit_mutation_evaluation {
        let next_mutation_spec_template =
            report
                .factor_mutation_evaluation
                .as_ref()
                .map(|evaluation| {
                    let base_factor = mutation_spec
                        .as_ref()
                        .map(|spec| spec.base_factor.as_str())
                        .filter(|value| !value.is_empty())
                        .or_else(|| {
                            evaluation
                                .metrics_after
                                .top_factor_names
                                .first()
                                .map(String::as_str)
                        })
                        .unwrap_or("");
                    let (preferred_direction_hints, preferred_step_size_hints) =
                        factor_specific_hint_preferences(state_dir, symbol, base_factor);
                    next_mutation_spec_template_with_preferences(
                        mutation_spec.as_ref(),
                        evaluation,
                        mutation_spec
                            .as_ref()
                            .map(|spec| spec.evaluate_expansion_preview)
                            .unwrap_or(false),
                        Some(&preferred_direction_hints),
                        Some(&preferred_step_size_hints),
                    )
                });
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "symbol": symbol,
                "mutation_spec": mutation_spec,
                "factor_mutation_evaluation": report.factor_mutation_evaluation,
                "next_mutation_spec_template": next_mutation_spec_template,
                "multi_timeframe_summary": report.multi_timeframe_summary,
                "recommended_next_command": report.recommended_next_command,
                "top_factor": report.best_factor,
                "artifact_gate_status": report.artifact_decision_summary.consumed_trend_status,
            }))?
        );
    } else {
        let lifecycle_view = build_factor_lifecycle_view(
            mutation_spec.as_ref(),
            report.factor_mutation_evaluation.as_ref(),
            &report.promotion_decision,
            &report.rollback_recommendation,
        );
        let ensemble_surface = if ensemble {
            report
                .workflow_snapshot
                .latest_ensemble_vote
                .as_ref()
                .map(|vote| {
                    let persisted_scorecards =
                        load_ensemble_executor_scorecards(state_dir, symbol).unwrap_or_default();
                    let (scorecards, scorecard_source) =
                        resolved_vote_scorecards(&persisted_scorecards, vote);
                    serde_json::json!({
                        "ensemble_vote": vote,
                        "executor_scorecards": scorecards,
                        "executor_scorecard_source": scorecard_source,
                    })
                })
        } else {
            None
        };
        let persisted_research_runs: Vec<ResearchRunRecord> =
            load_state_or_default(state_dir, symbol, RESEARCH_RUNS_FILE)?;
        let research_compare_report =
            persisted_research_runs
                .split_last()
                .and_then(|(current, previous)| {
                    previous.last().and_then(|prior| {
                        crate::application::backtest::build_research_compare_report(prior, current)
                    })
                });
        let compare_summary = crate::application::reporting::human_research_compare_summary(
            research_compare_report.as_ref(),
        );
        let reflection_bundle =
            build_research_reflection_bundle(symbol, &report, compare_summary.as_deref());
        let payload = crate::application::reporting::build_factor_research_output_payload(
            &report,
            research_compare_report,
            serde_json::to_value(&reflection_bundle)?,
            ensemble_surface,
            serde_json::to_value(&lifecycle_view)?,
        );
        crate::application::reporting::emit_structured_output_payload(
            output_format,
            &payload,
            &payload["compact_compare_report"],
        )?;
    }
    Ok(())
}

pub struct BacktestCommandInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub paired_data: Option<&'a str>,
    pub state_dir: &'a str,
    pub output_format: &'a str,
    pub warmup_bars: usize,
    pub hold_bars: usize,
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub fee_bps: f64,
    pub ambiguous_bar_policy: &'a str,
    pub online_learn: bool,
}

pub fn backtest_command<
    FRunResearch,
    FParseRealism,
    FRunBacktest,
    FFinalize,
    TRealism,
    TBacktestTuple,
>(
    input: BacktestCommandInput<'_>,
    run_research: FRunResearch,
    parse_realism: FParseRealism,
    run_backtest: FRunBacktest,
    finalize_report: FFinalize,
) -> Result<()>
where
    FRunResearch: Fn() -> Result<()>,
    FParseRealism: Fn(f64, f64, f64, &str) -> Result<TRealism>,
    FRunBacktest: Fn(&TRealism) -> Result<TBacktestTuple>,
    FFinalize:
        Fn(TBacktestTuple, &TRealism) -> Result<crate::backtest_report_shell::BacktestReport>,
{
    let BacktestCommandInput {
        symbol,
        data: _,
        paired_data: _,
        state_dir,
        output_format,
        warmup_bars: _,
        hold_bars: _,
        spread_bps,
        slippage_bps,
        fee_bps,
        ambiguous_bar_policy,
        online_learn: _,
    } = input;

    run_research()?;
    let realism = parse_realism(spread_bps, slippage_bps, fee_bps, ambiguous_bar_policy)?;
    let report = finalize_report(run_backtest(&realism)?, &realism)?;
    let realism_summary = format!(
        "execution_realism=spread:{:.2}bps slippage:{:.2}bps fee:{:.2}bps policy={} trades={} comparable={}",
        report.spread_bps,
        report.slippage_bps,
        report.fee_bps,
        report.ambiguous_bar_policy,
        report.trades,
        report.dataset_comparability.comparable
    );
    let zero_trade_risk = if report.trades == 0 {
        vec!["no_trades_generated_under_current_constraints".to_string()]
    } else {
        Vec::new()
    };
    let shrink_comparison_summary = build_shrink_on_off_comparison_summary(
        report.metrics.conformal_coverage_1sigma,
        (report.metrics.conformal_coverage_1sigma + report.metrics.regime_break_penalty)
            .clamp(0.0, 1.0),
        report.metrics.total_return,
        report.metrics.total_return + report.metrics.regime_break_penalty,
    );
    let oos_quality_delta_surface = build_oos_quality_delta_surface(
        report.metrics.conformal_coverage_1sigma,
        (report.metrics.conformal_coverage_1sigma - report.metrics.regime_break_penalty)
            .clamp(0.0, 1.0),
        report.trades,
        report.trades,
    );
    let duration_sizing_delta_surface = build_duration_surface_from_artifacts(
        &report.workflow_snapshot,
        &report.artifact_action_summary,
    );
    let compact_report = build_backtest_result_artifact(BacktestResultArtifactInput {
        summary: format!("backtest:{}", symbol),
        scorecards: vec![realism_summary.clone()],
        shrink_comparison_summary,
        duration_sizing_delta_surface,
        oos_quality_delta_surface,
        market_breakdown: vec![
            format!("symbol={}", symbol),
            format!("trades={}", report.trades),
        ],
        regime_breakdown: zero_trade_risk,
        window_breakdown: vec![],
        comparable: report.dataset_comparability.comparable,
        artifacts: vec![],
    });
    let persisted_backtest_runs: Vec<BacktestRunRecord> =
        load_state_or_default(state_dir, symbol, BACKTEST_RUNS_FILE)?;
    let backtest_compare_report =
        persisted_backtest_runs
            .split_last()
            .and_then(|(current, previous)| {
                previous
                    .last()
                    .and_then(|prior| build_backtest_compare_report(prior, current))
            });
    let human_backtest_summary = if report.trades == 0 {
        format!(
            "Backtest ran with {} and produced no trades under the current constraints.",
            realism_summary
        )
    } else {
        format!(
            "Backtest ran with {} and produced {} trades.",
            realism_summary, report.trades
        )
    };
    let payload = crate::application::reporting::build_backtest_output_payload(
        &report,
        &compact_report,
        backtest_compare_report,
        human_backtest_summary,
    );
    crate::application::reporting::emit_structured_output_payload(
        output_format,
        &payload,
        &compact_report,
    )?;
    Ok(())
}
