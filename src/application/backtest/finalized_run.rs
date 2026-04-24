use anyhow::Result;

use crate::application::belief::persist_market_jump_calibration_from_backtest_runs;
use crate::application::execution::derive_backtest_execution_fields;
use crate::config::family_history_window;
use crate::state::{append_backtest_run, recommended_next_command_meta, BacktestRunRecord};

pub struct PersistFinalizedBacktestRunInput<'a> {
    pub report: &'a crate::backtest_report_shell::BacktestReport,
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub data: &'a str,
    pub paired_data: Option<&'a str>,
    pub candles: usize,
    pub paired_candles: Option<usize>,
    pub warmup_bars: usize,
    pub hold_bars: usize,
    pub online_learning: bool,
}

pub fn persist_finalized_backtest_run(
    input: PersistFinalizedBacktestRunInput<'_>,
) -> Result<Vec<BacktestRunRecord>> {
    let PersistFinalizedBacktestRunInput {
        report,
        symbol,
        state_dir,
        data,
        paired_data,
        candles,
        paired_candles,
        warmup_bars,
        hold_bars,
        online_learning,
    } = input;

    let backtest_execution_fields = derive_backtest_execution_fields(
        report.trades,
        report.metrics.total_return,
        report.metrics.regime_break_penalty,
        report.promotion_decision.approved,
    );

    let backtest_runs = append_backtest_run(
        state_dir,
        symbol,
        BacktestRunRecord {
            run_id: format!(
                "backtest:{}:{}",
                symbol,
                chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
            ),
            timestamp: chrono::Utc::now(),
            symbol: symbol.to_string(),
            provenance: report.provenance.clone(),
            decision_thresholds: report.decision_thresholds.clone(),
            dataset_comparability: report.dataset_comparability.clone(),
            promotion_decision: report.promotion_decision.clone(),
            rollback_recommendation: report.rollback_recommendation.clone(),
            family_history_window: family_history_window(),
            data_path: data.to_string(),
            paired_data_path: paired_data.map(str::to_string),
            candles,
            paired_candles,
            warmup_bars,
            hold_bars,
            online_learning,
            source_command: "backtest".to_string(),
            total_return: report.metrics.total_return,
            trade_count: report.trades,
            conformal_coverage_1sigma: report.metrics.conformal_coverage_1sigma,
            conformal_miscoverage_1sigma: report.metrics.conformal_miscoverage_1sigma,
            mean_prediction_interval_half_width: report.metrics.mean_prediction_interval_half_width,
            worst_window_miscoverage: report.metrics.worst_window_miscoverage,
            regime_break_penalty: report.metrics.regime_break_penalty,
            structural_break_score: report.metrics.structural_break_score,
            structural_break_index: report.metrics.structural_break_index,
            structural_break_detected: report.metrics.structural_break_detected,
            signal_structural_break_score: report.metrics.signal_structural_break_score,
            signal_structural_break_index: report.metrics.signal_structural_break_index,
            signal_structural_break_detected: report.metrics.signal_structural_break_detected,
            residual_structural_break_score: report.metrics.residual_structural_break_score,
            residual_structural_break_index: report.metrics.residual_structural_break_index,
            residual_structural_break_detected: report.metrics.residual_structural_break_detected,
            rolling_ic_structural_break_score: report.metrics.rolling_ic_structural_break_score,
            rolling_ic_structural_break_index: report.metrics.rolling_ic_structural_break_index,
            rolling_ic_structural_break_detected: report
                .metrics
                .rolling_ic_structural_break_detected,
            factor_score_deltas: report.factor_score_deltas.clone(),
            trade_outcome_deltas: report.trade_outcome_deltas.clone(),
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
            artifact_decision_summary: report.artifact_decision_summary.clone(),
            artifact_decision_section: report.artifact_decision_section.clone(),
            agent_prompts: report.agent_prompts.clone(),
            prompt_workflow: report.agent_prompts.workflow.clone(),
            multi_timeframe_summary: report.multi_timeframe_summary.clone(),
            objective_market_credibility_shrink: report.objective_market_credibility_shrink.clone(),
        },
    )?;

    persist_market_jump_calibration_from_backtest_runs(
        state_dir,
        symbol,
        &backtest_runs,
        None,
        None,
    )?;
    Ok(backtest_runs)
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
