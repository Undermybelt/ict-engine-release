use anyhow::{bail, Result};
use chrono::Utc;
use std::path::Path;

use crate::application::backtest::{
    append_control_matrix_research_artifact, build_backtest_compare_report,
    build_backtest_result_artifact, build_control_matrix_discovery_summary_for_symbol,
    build_control_matrix_provider_summary_for_plan, build_control_matrix_research_artifact,
    build_duration_sizing_delta_surface, build_oos_quality_delta_surface,
    build_shrink_on_off_comparison_summary, BacktestResultArtifactInput, ControlMatrixPlan,
    ControlMatrixResearchArtifact, ControlMatrixResearchArtifactInput,
    ControlMatrixResearchRunSummary,
};
use crate::application::data_sources::{
    build_control_matrix_runtime_overrides, ControlMatrixRuntimeOverrides,
};
use crate::application::decision_utils::{
    parse_research_objective, research_objective_label, ResearchObjectiveMode,
};
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

pub fn latest_duration_phase(
    snapshot: &WorkflowSnapshot,
) -> Option<&crate::state::WorkflowPhaseSnapshot> {
    snapshot
        .latest_backtest
        .as_ref()
        .or(snapshot.latest_research.as_ref())
        .or(snapshot.latest_update.as_ref())
        .or(snapshot.latest_analyze.as_ref())
}

pub fn parse_duration_sizing_scale(summary: &[String]) -> Option<f64> {
    summary.iter().find_map(|line| {
        line.split_whitespace().find_map(|fragment| {
            fragment
                .strip_prefix("duration_sizing_scale=")
                .and_then(|value| value.parse::<f64>().ok())
        })
    })
}

pub fn build_duration_surface_from_artifacts(
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

fn copy_directory_tree(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_directory_tree(&source_path, &target_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn attach_policy_training_summaries(
    mut payload: serde_json::Value,
    surface: &crate::application::entry_models::PolicyTrainingStatusSurface,
) -> serde_json::Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "structural_path_ranking_runtime_summary".to_string(),
            serde_json::json!(surface.structural_path_ranking_runtime_summary),
        );
        object.insert(
            "structural_path_ranking_validation_summary".to_string(),
            serde_json::json!(surface.structural_path_ranking_validation_summary),
        );
    }
    payload
}

fn clone_symbol_state(source_root: &Path, symbol: &str, target_root: &Path) -> Result<()> {
    copy_directory_tree(&source_root.join(symbol), &target_root.join(symbol))
}

fn build_control_matrix_run_summary(
    run_spec: &crate::application::backtest::Pb12RunSpec,
    report: &crate::factor_lab::research::ResearchReport,
    runtime_notes: &[String],
) -> ControlMatrixResearchRunSummary {
    ControlMatrixResearchRunSummary {
        run_number: run_spec.run_number,
        run_label: run_spec.compact_label(),
        baseline: run_spec.baseline,
        enabled_toggles: run_spec
            .enabled_toggles()
            .into_iter()
            .map(str::to_string)
            .collect(),
        disabled_toggles: run_spec
            .disabled_toggles()
            .into_iter()
            .map(str::to_string)
            .collect(),
        best_factor: report.best_factor.clone(),
        aggregate_return: report.aggregate_return,
        feedback_records_generated: report.feedback_records_generated,
        feedback_records_applied: report.feedback_records_applied,
        dataset_comparable: report.dataset_comparability.comparable,
        recommended_next_command: report.recommended_next_command.clone(),
        runtime_notes: runtime_notes.to_vec(),
    }
}

fn run_control_matrix_research_sweep<FRun>(
    symbol: &str,
    objective: ResearchObjectiveMode,
    mutation_spec: Option<&FactorMutationSpec>,
    data_path: &str,
    state_dir: &str,
    run_research: &FRun,
) -> Result<ControlMatrixResearchArtifact>
where
    FRun: Fn(
        ResearchObjectiveMode,
        Option<FactorMutationSpec>,
        Option<ControlMatrixPlan>,
        Option<crate::application::backtest::Pb12RunSpec>,
        ControlMatrixRuntimeOverrides,
        &str,
    ) -> Result<crate::factor_lab::research::ResearchReport>,
{
    let generated_at = Utc::now();
    let sweep_id = format!(
        "pb12:{}:{}",
        symbol,
        generated_at.format("%Y%m%dT%H%M%S%.9fZ")
    );
    let control_matrix_plan = ControlMatrixPlan::pb12();
    let mut runs = Vec::with_capacity(control_matrix_plan.runs.len());
    for run_spec in &control_matrix_plan.runs {
        let isolated_state_dir = std::env::temp_dir().join(format!(
            "ict-engine-pb12-{}-{}-{:02}",
            symbol,
            generated_at.format("%Y%m%dT%H%M%S%.9fZ"),
            run_spec.run_number
        ));
        if isolated_state_dir.exists() {
            let _ = std::fs::remove_dir_all(&isolated_state_dir);
        }
        std::fs::create_dir_all(&isolated_state_dir)?;
        clone_symbol_state(Path::new(state_dir), symbol, &isolated_state_dir)?;
        let runtime_overrides = build_control_matrix_runtime_overrides(data_path, symbol, run_spec)
            .unwrap_or_else(|err| ControlMatrixRuntimeOverrides {
                runtime_notes: vec![format!("runtime_provider_error={err}")],
                ..ControlMatrixRuntimeOverrides::default()
            });
        let report = run_research(
            objective,
            mutation_spec.cloned(),
            Some(control_matrix_plan.clone()),
            Some(run_spec.clone()),
            runtime_overrides.clone(),
            isolated_state_dir.to_str().unwrap_or(state_dir),
        )?;
        let _ = std::fs::remove_dir_all(&isolated_state_dir);
        runs.push(build_control_matrix_run_summary(
            run_spec,
            &report,
            &runtime_overrides.runtime_notes,
        ));
    }
    let discovery_summary =
        build_control_matrix_discovery_summary_for_symbol(state_dir, symbol, data_path)?;
    let provider_summary = build_control_matrix_provider_summary_for_plan(&control_matrix_plan);
    let artifact = build_control_matrix_research_artifact(ControlMatrixResearchArtifactInput {
        symbol,
        sweep_id: &sweep_id,
        research_objective: research_objective_label(objective),
        generated_at,
        control_matrix_plan,
        runs,
        discovery_summary,
        provider_summary,
    });
    append_control_matrix_research_artifact(state_dir, symbol, artifact.clone())?;
    Ok(artifact)
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
    let policy_training_status =
        crate::application::entry_models::policy_training_status(state_dir, symbol, None)?;
    let payload = crate::application::reporting::build_factor_backtest_output_payload(
        crate::application::reporting::FactorBacktestOutputPayloadInput {
            report: &report,
            compact_backtest_report: &compact_report,
            compare: backtest_compare_report,
            credibility_summary,
            ensemble_surface,
            suggested_update_command: &suggested_update_command,
            structural_path_ranking_runtime_summary: Some(
                &policy_training_status.structural_path_ranking_runtime_summary,
            ),
            structural_path_ranking_validation_summary: Some(
                &policy_training_status.structural_path_ranking_validation_summary,
            ),
        },
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
    pub control_matrix_pb12: bool,
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
        Option<FactorMutationSpec>,
        Option<ControlMatrixPlan>,
        Option<crate::application::backtest::Pb12RunSpec>,
        ControlMatrixRuntimeOverrides,
        &str,
    ) -> Result<crate::factor_lab::research::ResearchReport>,
{
    let FactorResearchCommandInput {
        symbol,
        data,
        objective,
        mutation_spec_path,
        control_matrix_pb12,
        emit_mutation_evaluation,
        ensemble,
        state_dir,
        output_format,
    } = input;
    let _ = migrate_ensemble_executor_scorecards(state_dir, symbol)?;
    let objective = parse_research_objective(objective)?;
    let mutation_spec = mutation_spec_path.map(load_mutation_spec).transpose()?;
    if control_matrix_pb12 && emit_mutation_evaluation {
        bail!("--emit-mutation-evaluation is not supported with --control-matrix pb12");
    }
    if control_matrix_pb12 {
        let artifact = run_control_matrix_research_sweep(
            symbol,
            objective,
            mutation_spec.as_ref(),
            data,
            state_dir,
            &run_research,
        )?;
        let payload =
            crate::application::reporting::build_control_matrix_research_output_payload(&artifact);
        crate::application::reporting::emit_structured_output_payload(
            output_format,
            &payload,
            &artifact,
        )?;
        return Ok(());
    }
    let control_matrix_plan = control_matrix_pb12.then(ControlMatrixPlan::pb12);
    let report = run_research(
        objective,
        mutation_spec.clone(),
        control_matrix_plan.clone(),
        None,
        ControlMatrixRuntimeOverrides::default(),
        state_dir,
    )?;
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
            control_matrix_plan
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
        );
        let policy_training_status =
            crate::application::entry_models::policy_training_status(state_dir, symbol, None)?;
        let payload = attach_policy_training_summaries(payload, &policy_training_status);
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
        data,
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
    let report = finalize_report(run_backtest(&realism).map_err(|err| {
        let message = err.to_string();
        if message.contains("need more candles for backtest") {
            anyhow::anyhow!(
                "{message}. Try one of: 1. factor-only sanity check: ict-engine factor-backtest --symbol {symbol} --data {data} --state-dir {state_dir} --human 2. guided replay/demo review: ict-engine analyze --symbol {symbol} --demo --state-dir {state_dir} --human 3. fetch a longer dataset via ict-engine provider-status --compact and rerun backtest."
            )
        } else {
            err
        }
    })?, &realism)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::entry_models::PolicyTrainingStatusSurface;
    use crate::factor_lab::research::ResearchReport;
    use crate::state::{ArtifactLedgerEntry, ARTIFACT_LEDGER_FILE};
    use crate::types::Candle;
    use chrono::{Duration, TimeZone, Utc};
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    fn sample_candles(count: usize) -> Vec<Candle> {
        let mut candles = Vec::with_capacity(count);
        let mut base = 100.0;
        for index in 0..count {
            let open = base;
            let close = open + 0.2 + (index % 3) as f64 * 0.05;
            let high = close + 0.1;
            let low = open - 0.1;
            candles.push(Candle {
                timestamp: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
                    + Duration::minutes(index as i64),
                open,
                high,
                low,
                close,
                volume: 1_000.0,
            });
            base = close;
        }
        candles
    }

    #[test]
    fn attach_policy_training_summaries_inserts_top_level_ranker_fields() {
        let payload = serde_json::json!({
            "report": {"kind": "factor_research"}
        });
        let surface = PolicyTrainingStatusSurface {
            structural_path_ranking_runtime_summary:
                "Ranker runtime: runtime enabled=true ready=true source=registered_artifact status=enabled_registered_artifact_ready mode=candidate_set_only matches=2"
                    .to_string(),
            structural_path_ranking_validation_summary:
                "Ranker validation: calibration=true quality_ready=true raw_scored_mature=30/30 production_validation=30/30 observation_validation=0/30 ready=true"
                    .to_string(),
            ..PolicyTrainingStatusSurface::default()
        };

        let payload = attach_policy_training_summaries(payload, &surface);

        assert_eq!(
            payload["structural_path_ranking_runtime_summary"],
            serde_json::json!(
                "Ranker runtime: runtime enabled=true ready=true source=registered_artifact status=enabled_registered_artifact_ready mode=candidate_set_only matches=2"
            )
        );
        assert_eq!(
            payload["structural_path_ranking_validation_summary"],
            serde_json::json!(
                "Ranker validation: calibration=true quality_ready=true raw_scored_mature=30/30 production_validation=30/30 observation_validation=0/30 ready=true"
            )
        );
    }

    #[test]
    fn attach_policy_training_summaries_preserves_existing_payload_fields() {
        let payload = serde_json::json!({
            "human_output": "Factor research | objective=generic",
            "compact_compare_report": {"highlights": []}
        });
        let surface = PolicyTrainingStatusSurface {
            structural_path_ranking_runtime_summary:
                "Ranker runtime: runtime enabled=false ready=false source=none status=disabled mode=none matches=0"
                    .to_string(),
            structural_path_ranking_validation_summary:
                "Ranker validation: calibration=false quality_ready=false raw_scored_mature=0/0 production_validation=0/0 observation_validation=0/0 ready=false"
                    .to_string(),
            ..PolicyTrainingStatusSurface::default()
        };

        let payload = attach_policy_training_summaries(payload, &surface);

        assert_eq!(
            payload["human_output"],
            serde_json::json!("Factor research | objective=generic")
        );
        assert!(payload.get("compact_compare_report").is_some());
        assert_eq!(
            payload["structural_path_ranking_runtime_summary"],
            serde_json::json!(
                "Ranker runtime: runtime enabled=false ready=false source=none status=disabled mode=none matches=0"
            )
        );
        assert_eq!(
            payload["structural_path_ranking_validation_summary"],
            serde_json::json!(
                "Ranker validation: calibration=false quality_ready=false raw_scored_mature=0/0 production_validation=0/0 observation_validation=0/0 ready=false"
            )
        );
    }

    #[test]
    fn test_factor_research_command_pb12_uses_isolated_state_and_persists_sweep_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "NQ";
        let symbol_dir = temp.path().join(symbol);
        let data = temp.path().join("candles.json");
        std::fs::create_dir_all(&symbol_dir).unwrap();
        std::fs::write(symbol_dir.join("sentinel.json"), "{\"owner\":\"main\"}").unwrap();
        std::fs::write(
            &data,
            serde_json::to_string(&serde_json::json!({
                "candles": sample_candles(24)
            }))
            .unwrap(),
        )
        .unwrap();

        let observed_state_dirs = Arc::new(Mutex::new(Vec::<String>::new()));
        let observed_sentinel_contents = Arc::new(Mutex::new(Vec::<String>::new()));
        let run_counter = Arc::new(Mutex::new(0usize));

        let run_research = {
            let observed_state_dirs = Arc::clone(&observed_state_dirs);
            let observed_sentinel_contents = Arc::clone(&observed_sentinel_contents);
            let run_counter = Arc::clone(&run_counter);
            move |_objective,
                  _mutation_spec,
                  _control_matrix_plan,
                  _control_matrix_run,
                  runtime_overrides: ControlMatrixRuntimeOverrides,
                  run_state_dir: &str| {
                observed_state_dirs
                    .lock()
                    .unwrap()
                    .push(run_state_dir.to_string());
                observed_sentinel_contents.lock().unwrap().push(
                    std::fs::read_to_string(
                        Path::new(run_state_dir).join(symbol).join("sentinel.json"),
                    )
                    .unwrap_or_default(),
                );

                std::fs::create_dir_all(Path::new(run_state_dir).join(symbol)).unwrap();
                std::fs::write(
                    Path::new(run_state_dir)
                        .join(symbol)
                        .join("runner-owned.json"),
                    "{\"owner\":\"runner\"}",
                )
                .unwrap();

                let mut report = ResearchReport::default();
                let mut count = run_counter.lock().unwrap();
                *count += 1;
                report.research_objective = "generic".to_string();
                report.best_factor = Some(format!("factor_{}", count));
                report.aggregate_return = *count as f64 / 100.0;
                report.feedback_records_generated = 1;
                report.feedback_records_applied = 1;
                report.dataset_comparability.comparable = true;
                report.recommended_next_command = format!("pb12-next-{}", count);
                report.multi_timeframe_summary = runtime_overrides.runtime_notes;
                Ok(report)
            }
        };

        factor_research_command(
            FactorResearchCommandInput {
                symbol,
                data: data.to_str().unwrap(),
                objective: "generic",
                mutation_spec_path: None,
                control_matrix_pb12: true,
                emit_mutation_evaluation: false,
                ensemble: false,
                state_dir: temp.path().to_str().unwrap(),
                output_format: "human",
            },
            |_| unreachable!("mutation spec is not used in this test"),
            run_research,
        )
        .unwrap();

        let observed_state_dirs = observed_state_dirs.lock().unwrap().clone();
        assert_eq!(observed_state_dirs.len(), 12, "PB12 must execute 12 runs");
        assert!(
            observed_state_dirs
                .iter()
                .all(|dir| dir != temp.path().to_str().unwrap()),
            "PB12 runs must not reuse the primary state dir: {observed_state_dirs:?}"
        );
        assert!(
            observed_sentinel_contents
                .lock()
                .unwrap()
                .iter()
                .all(|content| content == "{\"owner\":\"main\"}"),
            "each isolated run must see the copied primary-state sentinel"
        );
        assert!(
            !symbol_dir.join("runner-owned.json").exists(),
            "runner-owned files must stay out of the primary state dir"
        );

        let ledger: Vec<ArtifactLedgerEntry> =
            load_state_or_default(temp.path(), symbol, ARTIFACT_LEDGER_FILE).unwrap();
        assert!(
            ledger
                .iter()
                .any(|entry| entry.artifact_kind == "auto_quant_pb12_research_run"),
            "PB12 sweep must persist a dedicated ledger entry"
        );
        let artifacts = crate::application::backtest::load_control_matrix_research_artifacts(
            temp.path(),
            symbol,
        )
        .unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(
            artifacts[0].discovery_summary.status,
            "baseline_unavailable"
        );
        assert!(
            symbol_dir
                .join("auto_quant_pb12_research_runs.json")
                .exists(),
            "PB12 sweep must persist its artifact history"
        );
    }

    #[test]
    fn backtest_command_wraps_short_history_error_with_guidance() {
        let err = backtest_command(
            BacktestCommandInput {
                symbol: "DEMO",
                data: "examples/demo/demo-15m.json",
                paired_data: None,
                state_dir: "/tmp/ict-engine-backtest-guidance",
                output_format: "human",
                warmup_bars: 20,
                hold_bars: 50,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                fee_bps: 0.0,
                ambiguous_bar_policy: "close",
                online_learn: false,
            },
            || Ok(()),
            |_, _, _, _| Ok(()),
            |_| anyhow::bail!("need more candles for backtest: got 52, require at least 71"),
            |_: (), _: &()| unreachable!("finalize should not run on short-history error"),
        )
        .unwrap_err();

        let message = err.to_string();
        assert!(message.contains("ict-engine factor-backtest --symbol DEMO"));
        assert!(message.contains("ict-engine analyze --symbol DEMO --demo"));
        assert!(message.contains("ict-engine provider-status --compact"));
    }
}
