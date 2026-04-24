use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use crate::application::backtest::BacktestCompareReport;
use crate::application::output_foundation::{print_redacted_json, redact_local_paths};
use crate::application::reporting::{build_compact_backtest_report, humanize_next_step_line};
use crate::backtest_report_shell::BacktestReport;
use crate::factor_lab::BacktestResult as FactorBacktestRunResult;

pub fn build_compact_compare_report(
    compare: Option<&BacktestCompareReport>,
) -> Option<crate::application::reporting::CompactBacktestReport> {
    compare.map(|compare| {
        build_compact_backtest_report(
            compare.summary.clone(),
            &compare.shrink_comparison_summary,
            &compare.duration_sizing_delta_surface,
            &compare.regressions,
            &compare.recommended_actions,
        )
    })
}

pub fn human_compare_summary(compare: Option<&BacktestCompareReport>) -> Option<String> {
    compare.map(|compare| {
        let duration = compare
            .duration_sizing_delta_surface
            .iter()
            .find(|line| line.starts_with("duration_sizing_direction="))
            .cloned()
            .unwrap_or_else(|| "duration_sizing_direction=unchanged".to_string());
        let risk = compare
            .regressions
            .first()
            .cloned()
            .unwrap_or_else(|| "no_material_regressions".to_string());
        let action = compare
            .recommended_actions
            .first()
            .cloned()
            .unwrap_or_else(|| "no_follow_up_action".to_string());
        format!("Compare: {} | risk={} | next={}", duration, risk, action)
    })
}

pub fn human_backtest_compare_summary(compare: Option<&BacktestCompareReport>) -> Option<String> {
    human_compare_summary(compare)
        .map(|summary| summary.replacen("Compare:", "Backtest compare:", 1))
}

pub fn human_research_compare_summary(compare: Option<&BacktestCompareReport>) -> Option<String> {
    human_compare_summary(compare)
        .map(|summary| summary.replacen("Compare:", "Research compare:", 1))
}

pub fn render_factor_backtest_human_output(
    report: &FactorBacktestRunResult,
    compare: Option<&BacktestCompareReport>,
) -> String {
    let best = report.best_factor.as_deref().unwrap_or("n/a");
    let aggregate_return_pct = report.aggregate_return * 100.0;
    let total_trades: usize = report
        .factor_results
        .iter()
        .map(|result| result.metrics.trade_count)
        .sum();
    let top = report.factor_results.first();
    let top_coverage = top
        .map(|result| result.metrics.conformal_coverage_1sigma)
        .unwrap_or_default();
    let top_regime_penalty = top
        .map(|result| result.metrics.regime_break_penalty)
        .unwrap_or_default();
    let top_structural_break = top
        .map(|result| result.metrics.structural_break_detected)
        .unwrap_or(false);

    let mut lines = vec![
        "Factor backtest summary".to_string(),
        format!("- Best factor: {best}"),
        format!("- Aggregate return: {aggregate_return_pct:+.2}%"),
        format!("- Trades: {total_trades}"),
    ];
    let mut credibility_parts = vec![
        format!("conformal_coverage_1sigma={top_coverage:.3}"),
        format!("regime_break_penalty={top_regime_penalty:.3}"),
    ];
    if top_structural_break {
        credibility_parts.push("structural_break=detected".to_string());
    }
    lines.push(format!("- Credibility: {}", credibility_parts.join(" | ")));
    lines.push(format!(
        "- Next: {}",
        humanize_next_step_line(&report.recommended_next_command)
    ));

    if let Some(compare_summary) = human_backtest_compare_summary(compare) {
        lines.push(String::new());
        lines.push(compare_summary);
    }
    lines.join("\n")
}

pub fn render_factor_research_human_output(
    report: &impl Serialize,
    compare: Option<&BacktestCompareReport>,
) -> String {
    let mut lines = vec![format!(
        "Factor research summary: {}",
        serde_json::to_string(report).unwrap_or_else(|_| "unavailable".to_string())
    )];
    if let Some(compare_summary) = human_research_compare_summary(compare) {
        lines.push(compare_summary);
    }
    lines.join("\n")
}

pub fn build_backtest_output_payload(
    report: &BacktestReport,
    compact_backtest_report: &impl Serialize,
    compare: Option<BacktestCompareReport>,
    human_backtest_summary: String,
) -> Value {
    let compact_compare_report = build_compact_compare_report(compare.as_ref());
    let human_backtest_compare_summary = human_backtest_compare_summary(compare.as_ref());
    let human_output = render_backtest_human_output(report, compare.as_ref());
    serde_json::json!({
        "report": report,
        "compact_backtest_report": compact_backtest_report,
        "backtest_compare_report": compare,
        "compact_compare_report": compact_compare_report,
        "human_backtest_compare_summary": human_backtest_compare_summary,
        "human_backtest_summary": human_backtest_summary,
        "human_output": human_output,
    })
}

pub fn render_backtest_human_output(
    report: &BacktestReport,
    compare: Option<&BacktestCompareReport>,
) -> String {
    let mut lines = vec![if report.trades == 0 {
        format!(
            "Backtest ran with execution_realism=spread:{:.2}bps slippage:{:.2}bps fee:{:.2}bps policy={} trades={} comparable={} and produced no trades under the current constraints.",
            report.spread_bps,
            report.slippage_bps,
            report.fee_bps,
            report.ambiguous_bar_policy,
            report.trades,
            report.dataset_comparability.comparable
        )
    } else {
        format!(
            "Backtest ran with execution_realism=spread:{:.2}bps slippage:{:.2}bps fee:{:.2}bps policy={} trades={} comparable={} and produced {} trades.",
            report.spread_bps,
            report.slippage_bps,
            report.fee_bps,
            report.ambiguous_bar_policy,
            report.trades,
            report.dataset_comparability.comparable,
            report.trades
        )
    }];
    if let Some(compare_summary) = human_backtest_compare_summary(compare) {
        lines.push(compare_summary);
    }
    lines.join("\n")
}

pub fn build_factor_backtest_output_payload(
    report: &FactorBacktestRunResult,
    compact_backtest_report: &impl Serialize,
    compare: Option<BacktestCompareReport>,
    credibility_summary: Value,
    ensemble_surface: Option<Value>,
    suggested_update_command: &str,
) -> Value {
    let compact_compare_report = build_compact_compare_report(compare.as_ref());
    let human_backtest_compare_summary = human_backtest_compare_summary(compare.as_ref());
    let human_output = render_factor_backtest_human_output(report, compare.as_ref());
    serde_json::json!({
        "report": report,
        "compact_backtest_report": compact_backtest_report,
        "backtest_compare_report": compare,
        "compact_compare_report": compact_compare_report,
        "human_backtest_compare_summary": human_backtest_compare_summary,
        "credibility_summary": credibility_summary,
        "ensemble": ensemble_surface,
        "human_output": human_output,
        "suggested_update_command": suggested_update_command,
    })
}

pub fn build_factor_research_output_payload(
    report: &impl Serialize,
    compare: Option<BacktestCompareReport>,
    reflection_bundle: Value,
    ensemble_surface: Option<Value>,
    factor_lifecycle: Value,
) -> Value {
    let compact_compare_report = build_compact_compare_report(compare.as_ref());
    let human_research_compare_summary = human_research_compare_summary(compare.as_ref());
    let human_output = render_factor_research_human_output(report, compare.as_ref());
    serde_json::json!({
        "report": report,
        "research_compare_report": compare,
        "compact_compare_report": compact_compare_report,
        "human_research_compare_summary": human_research_compare_summary,
        "reflection_bundle": reflection_bundle,
        "ensemble": ensemble_surface,
        "factor_lifecycle": factor_lifecycle,
        "human_output": human_output,
    })
}

pub fn emit_structured_output_payload(
    output_format: &str,
    payload: &Value,
    compact_surface: &impl Serialize,
) -> Result<()> {
    match output_format.trim().to_ascii_lowercase().as_str() {
        "json" | "agent" => println!("{}", serde_json::to_string_pretty(payload)?),
        "compact" => print_redacted_json(compact_surface)?,
        "human" => println!(
            "{}",
            redact_local_paths(payload["human_output"].as_str().unwrap_or_default())
        ),
        other => anyhow::bail!("unsupported output format '{}'", other),
    }
    Ok(())
}
