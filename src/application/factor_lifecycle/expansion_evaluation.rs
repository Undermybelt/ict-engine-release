use anyhow::Result;

use crate::application::backtest::{
    pre_bayes_entry_quality_bridge_diff, pre_bayes_soft_evidence_diff,
};
use crate::application::belief::build_expansion_factor_pipeline_report_with_registry;
use crate::application::data_sources::{CleanFuturesReport, ExpansionMarketReport};
use crate::application::decision_utils::{pre_bayes_gate_is_hard_pass, ResearchObjectiveMode};
use crate::application::factor_lifecycle::{
    expansion_factor_scores_for_market, recommended_mutation_directions_from_failure_tags,
};
use crate::application::multi_timeframe_inputs::{
    build_multi_timeframe_research_signal, build_multi_timeframe_summary,
    resolve_multi_timeframe_inputs, MultiTimeframeInputPaths,
};
use crate::data::load_candles;
use crate::factors::FactorRegistry;
use crate::state::{FactorMutationEvaluation, FactorMutationMetricSet, FactorMutationSpec};

pub fn build_expansion_sop_mutation_metrics(
    registry: &FactorRegistry,
    clean_report: &CleanFuturesReport,
    lookback: usize,
    atr_multiplier: f64,
    _objective_mode: ResearchObjectiveMode,
) -> Result<FactorMutationMetricSet> {
    let mut market_reports = Vec::new();
    for dataset in &clean_report.datasets {
        let candles = load_candles(&dataset.output_path)?;
        let resolved_multi_timeframe_inputs = resolve_multi_timeframe_inputs(
            &dataset.output_path,
            MultiTimeframeInputPaths::default(),
        );
        let multi_timeframe_summary =
            build_multi_timeframe_summary(&dataset.output_path, &resolved_multi_timeframe_inputs)?
                .into_iter()
                .chain(
                    build_multi_timeframe_research_signal(&resolved_multi_timeframe_inputs)?
                        .summary,
                )
                .collect::<Vec<_>>();
        let scores =
            expansion_factor_scores_for_market(registry, &candles, lookback, atr_multiplier)?;
        let best_factor = scores.first().map(|score| score.factor_name.clone());
        let pipeline = best_factor
            .as_deref()
            .map(|factor| {
                build_expansion_factor_pipeline_report_with_registry(
                    &dataset.market,
                    factor,
                    &candles,
                    None,
                    &multi_timeframe_summary,
                    registry,
                )
            })
            .transpose()?;
        market_reports.push(ExpansionMarketReport {
            market: dataset.market.clone(),
            cleaned_path: dataset.output_path.clone(),
            total_candles: dataset.summary.aggregated_candles,
            expansion_samples: scores
                .first()
                .map(|score| score.expansion_samples)
                .unwrap_or(0),
            bull_expansion_samples: scores
                .first()
                .map(|score| score.bull_expansion_samples)
                .unwrap_or(0),
            bear_expansion_samples: scores
                .first()
                .map(|score| score.bear_expansion_samples)
                .unwrap_or(0),
            best_factor,
            top_factors: scores.into_iter().take(5).collect(),
            multi_timeframe_summary,
            pipeline,
        });
    }
    Ok(build_expansion_sop_metrics_from_market_reports(
        &market_reports,
    ))
}

pub fn build_expansion_sop_metrics_from_market_reports(
    market_reports: &[ExpansionMarketReport],
) -> FactorMutationMetricSet {
    let mut metrics = FactorMutationMetricSet {
        top_factor_names: market_reports
            .iter()
            .filter_map(|market| market.best_factor.clone())
            .take(3)
            .collect(),
        ..FactorMutationMetricSet::default()
    };
    if market_reports.is_empty() {
        return metrics;
    }
    metrics.best_factor_composite_score = market_reports
        .iter()
        .filter_map(|market| market.top_factors.first().map(|score| score.fit_score))
        .sum::<f64>()
        / market_reports.len() as f64;
    metrics.expansion_balanced_accuracy = Some(
        market_reports
            .iter()
            .filter_map(|market| {
                market
                    .top_factors
                    .first()
                    .map(|score| score.balanced_accuracy)
            })
            .sum::<f64>()
            / market_reports.len() as f64,
    );
    metrics.expansion_directional_accuracy = Some(
        market_reports
            .iter()
            .filter_map(|market| {
                market
                    .top_factors
                    .first()
                    .map(|score| score.directional_accuracy)
            })
            .sum::<f64>()
            / market_reports.len() as f64,
    );
    let selected_probabilities = market_reports
        .iter()
        .filter_map(|market| {
            market
                .pipeline
                .as_ref()
                .map(|pipeline| pipeline.bbn_support.selected_win_probability)
        })
        .collect::<Vec<_>>();
    if !selected_probabilities.is_empty() {
        metrics.expansion_selected_win_probability =
            Some(selected_probabilities.iter().sum::<f64>() / selected_probabilities.len() as f64);
    }
    metrics.pre_bayes_soft_evidence_divergence_count = market_reports
        .iter()
        .filter_map(|market| market.pipeline.as_ref())
        .map(|pipeline| {
            pre_bayes_soft_evidence_diff(&pipeline.bbn_support.pre_bayes_filter)
                .into_iter()
                .filter(|item| item.diverges_from_filtered_state)
                .count()
        })
        .sum::<usize>();
    metrics.pre_bayes_gate_status = market_reports
        .iter()
        .filter_map(|market| market.pipeline.as_ref())
        .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.gating_status.clone())
        .find(|status| status == "observe_only")
        .or_else(|| {
            market_reports
                .iter()
                .filter_map(|market| market.pipeline.as_ref())
                .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.gating_status.clone())
                .find(|status| status == "pass_neutralized")
        })
        .or_else(|| {
            market_reports
                .iter()
                .filter_map(|market| market.pipeline.as_ref())
                .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.gating_status.clone())
                .next()
        });
    let bridge_gaps = market_reports
        .iter()
        .filter_map(|market| market.pipeline.as_ref())
        .map(|pipeline| {
            pre_bayes_entry_quality_bridge_diff(&pipeline.entry_quality_bridge)
                .long_short_signal_probability_gap
        })
        .collect::<Vec<_>>();
    if !bridge_gaps.is_empty() {
        metrics.pre_bayes_bridge_probability_gap =
            Some(bridge_gaps.iter().sum::<f64>() / bridge_gaps.len() as f64);
    }
    metrics.pre_bayes_bridge_selected_entry_quality = market_reports
        .iter()
        .filter_map(|market| market.pipeline.as_ref())
        .find_map(|pipeline| {
            pre_bayes_entry_quality_bridge_diff(&pipeline.entry_quality_bridge)
                .selected_entry_quality
        });
    metrics.expansion_selected_direction = market_reports
        .iter()
        .filter_map(|market| market.pipeline.as_ref())
        .map(|pipeline| pipeline.bbn_support.selected_direction.clone())
        .next();
    metrics.worst_market_balanced_accuracy = market_reports
        .iter()
        .filter_map(|market| {
            market
                .top_factors
                .first()
                .map(|score| score.balanced_accuracy)
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    metrics.worst_market_bridge_probability_gap = market_reports
        .iter()
        .filter_map(|market| {
            market.pipeline.as_ref().map(|pipeline| {
                pre_bayes_entry_quality_bridge_diff(&pipeline.entry_quality_bridge)
                    .long_short_signal_probability_gap
            })
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    metrics
}

pub fn mechanical_mutation_score(
    metrics: &FactorMutationMetricSet,
    objective: ResearchObjectiveMode,
) -> f64 {
    match objective {
        ResearchObjectiveMode::Generic => {
            metrics.best_factor_composite_score * 0.55
                + metrics.aggregate_return * 0.20
                + metrics.expansion_balanced_accuracy.unwrap_or(0.0) * 0.15
                + metrics.expansion_selected_win_probability.unwrap_or(0.0) * 0.10
                + metrics.multi_timeframe_alignment_score.unwrap_or(0.0) * 0.08
                + metrics.multi_timeframe_entry_alignment_score.unwrap_or(0.0) * 0.04
                - metrics.pre_bayes_soft_evidence_divergence_count as f64 * 0.05
                + if metrics.multi_timeframe_direction_bias.as_deref() == Some("neutral") {
                    0.0
                } else {
                    0.03
                }
        }
        ResearchObjectiveMode::ExpansionManipulation => {
            let bridge_gap_score = metrics
                .pre_bayes_bridge_probability_gap
                .map(|gap| (gap / 0.25).clamp(0.0, 1.0))
                .unwrap_or_default();
            metrics.best_factor_composite_score * 0.60
                + metrics.expansion_balanced_accuracy.unwrap_or(0.0) * 0.20
                + metrics.expansion_directional_accuracy.unwrap_or(0.0) * 0.10
                + metrics.expansion_selected_win_probability.unwrap_or(0.0) * 0.05
                + bridge_gap_score * 0.03
                + metrics.multi_timeframe_alignment_score.unwrap_or(0.0) * 0.04
                + metrics.multi_timeframe_entry_alignment_score.unwrap_or(0.0) * 0.03
                - metrics.pre_bayes_soft_evidence_divergence_count as f64 * 0.05
                + if metrics.multi_timeframe_direction_bias.as_deref() == Some("neutral") {
                    0.0
                } else {
                    0.02
                }
        }
    }
}

pub fn evaluate_expansion_sop_mutation(
    spec: &FactorMutationSpec,
    root: &str,
    interval: &str,
    _lookback: usize,
    _atr_multiplier: f64,
    baseline_metrics: Option<&FactorMutationMetricSet>,
    metrics_after: FactorMutationMetricSet,
) -> FactorMutationEvaluation {
    let metrics_before = baseline_metrics.cloned();
    let score_before = metrics_before
        .as_ref()
        .map(|metrics| {
            mechanical_mutation_score(metrics, ResearchObjectiveMode::ExpansionManipulation)
        })
        .unwrap_or_default();
    let score_after =
        mechanical_mutation_score(&metrics_after, ResearchObjectiveMode::ExpansionManipulation);
    let score_delta = score_after - score_before;
    let balanced_accuracy_before = metrics_before
        .as_ref()
        .and_then(|metrics| metrics.expansion_balanced_accuracy)
        .unwrap_or_default();
    let balanced_accuracy_after = metrics_after
        .expansion_balanced_accuracy
        .unwrap_or_default();
    let bridge_gap_before = metrics_before
        .as_ref()
        .and_then(|metrics| metrics.pre_bayes_bridge_probability_gap)
        .unwrap_or_default();
    let bridge_gap_after = metrics_after
        .pre_bayes_bridge_probability_gap
        .unwrap_or_default();
    let worst_market_balanced_accuracy_after = metrics_after
        .worst_market_balanced_accuracy
        .unwrap_or_default();
    let worst_market_bridge_gap_after = metrics_after
        .worst_market_bridge_probability_gap
        .unwrap_or_default();
    let mut failure_tags = Vec::new();
    if balanced_accuracy_after < 0.60 {
        failure_tags.push("bull_bear_separation_weak".to_string());
    }
    if balanced_accuracy_after + 1e-9 < balanced_accuracy_before {
        failure_tags.push("bull_bear_separation_regressed".to_string());
    }
    if metrics_before
        .as_ref()
        .map(|before| {
            metrics_after.pre_bayes_soft_evidence_divergence_count
                > before.pre_bayes_soft_evidence_divergence_count
        })
        .unwrap_or(metrics_after.pre_bayes_soft_evidence_divergence_count > 0)
    {
        failure_tags.push("soft_evidence_divergence_elevated".to_string());
    }
    if bridge_gap_after < 0.08 {
        failure_tags.push("bridge_gap_too_small".to_string());
    }
    if worst_market_balanced_accuracy_after < 0.55 {
        failure_tags.push("worst_market_separation_weak".to_string());
    }
    if worst_market_bridge_gap_after < 0.06 {
        failure_tags.push("worst_market_bridge_gap_too_small".to_string());
    }
    if bridge_gap_after + 1e-9 < bridge_gap_before {
        failure_tags.push("bridge_gap_regressed".to_string());
    }
    if metrics_after.pre_bayes_gate_status.as_deref() == Some("observe_only") {
        failure_tags.push("pre_bayes_gate_observe_only".to_string());
    }
    if metrics_after.pre_bayes_gate_status.as_deref() == Some("pass_neutralized") {
        failure_tags.push("pre_bayes_gate_neutralized".to_string());
    }
    if !metrics_after.regressed_markets.is_empty() {
        failure_tags.push("market_specific_regressions_detected".to_string());
    }
    let recommended_mutation_directions = recommended_mutation_directions_from_failure_tags(
        &failure_tags,
        &metrics_after.regressed_markets,
        &metrics_after.regression_reasons_by_market,
    );

    FactorMutationEvaluation {
        mutation_id: if spec.mutation_id.is_empty() {
            format!("expansion-sop:{}:{}", interval, root)
        } else {
            spec.mutation_id.clone()
        },
        accepted: score_delta > 0.0
            && balanced_accuracy_after >= balanced_accuracy_before
            && bridge_gap_after >= bridge_gap_before
            && metrics_after
                .pre_bayes_gate_status
                .as_deref()
                .map(pre_bayes_gate_is_hard_pass)
                .unwrap_or(false)
            && failure_tags.is_empty(),
        score_before,
        score_after,
        score_delta,
        baseline_available: metrics_before.is_some(),
        reason: if score_delta > 0.0 && failure_tags.is_empty() {
            "expansion_preview_mechanical_score_improved".to_string()
        } else if failure_tags.is_empty() {
            "expansion_preview_mechanical_score_not_improved".to_string()
        } else {
            format!("expansion_preview_flagged:{}", failure_tags.join(","))
        },
        failure_tags,
        recommended_mutation_directions,
        metrics_before,
        metrics_after,
    }
}
