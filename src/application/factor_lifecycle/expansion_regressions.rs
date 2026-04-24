use std::collections::BTreeMap;

use anyhow::Result;

use crate::application::backtest::pre_bayes_entry_quality_bridge_diff;
use crate::application::belief::build_expansion_factor_pipeline_report_with_registry;
use crate::application::decision_utils::pre_bayes_gate_regressed;
use crate::application::factor_lifecycle::expansion_factor_scores_for_market;
use crate::application::multi_timeframe_inputs::{
    build_multi_timeframe_research_signal, build_multi_timeframe_summary,
    resolve_multi_timeframe_inputs,
};
use crate::data::load_candles;
use crate::factors::FactorRegistry;

pub fn expansion_regression_reasons_by_market(
    baseline_registry: &FactorRegistry,
    mutated_registry: &FactorRegistry,
    datasets: &[(&str, &str)],
    lookback: usize,
    atr_multiplier: f64,
) -> Result<BTreeMap<String, Vec<String>>> {
    let mut reasons_by_market = BTreeMap::<String, Vec<String>>::new();
    for (market, output_path) in datasets {
        let candles = load_candles(output_path)?;
        let resolved_multi_timeframe_inputs =
            resolve_multi_timeframe_inputs(output_path, None, None, None, None, None, None);
        let multi_timeframe_summary =
            build_multi_timeframe_summary(output_path, &resolved_multi_timeframe_inputs)?
                .into_iter()
                .chain(
                    build_multi_timeframe_research_signal(&resolved_multi_timeframe_inputs)?
                        .summary
                        .into_iter(),
                )
                .collect::<Vec<_>>();
        let baseline_scores = expansion_factor_scores_for_market(
            baseline_registry,
            &candles,
            lookback,
            atr_multiplier,
        )?;
        let mutated_scores = expansion_factor_scores_for_market(
            mutated_registry,
            &candles,
            lookback,
            atr_multiplier,
        )?;
        let baseline_balanced = baseline_scores
            .first()
            .map(|score| score.balanced_accuracy)
            .unwrap_or_default();
        let mutated_balanced = mutated_scores
            .first()
            .map(|score| score.balanced_accuracy)
            .unwrap_or_default();
        let baseline_factor = baseline_scores
            .first()
            .map(|score| score.factor_name.as_str());
        let mutated_factor = mutated_scores
            .first()
            .map(|score| score.factor_name.as_str());
        let baseline_pipeline = baseline_factor
            .map(|factor| {
                build_expansion_factor_pipeline_report_with_registry(
                    market,
                    factor,
                    &candles,
                    None,
                    &multi_timeframe_summary,
                    baseline_registry,
                )
            })
            .transpose()?;
        let mutated_pipeline = mutated_factor
            .map(|factor| {
                build_expansion_factor_pipeline_report_with_registry(
                    market,
                    factor,
                    &candles,
                    None,
                    &multi_timeframe_summary,
                    mutated_registry,
                )
            })
            .transpose()?;
        let baseline_bridge_gap = baseline_pipeline
            .as_ref()
            .map(|pipeline| {
                pre_bayes_entry_quality_bridge_diff(&pipeline.entry_quality_bridge)
                    .long_short_signal_probability_gap
            })
            .unwrap_or_default();
        let mutated_bridge_gap = mutated_pipeline
            .as_ref()
            .map(|pipeline| {
                pre_bayes_entry_quality_bridge_diff(&pipeline.entry_quality_bridge)
                    .long_short_signal_probability_gap
            })
            .unwrap_or_default();
        let baseline_gate = baseline_pipeline
            .as_ref()
            .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.gating_status.as_str())
            .unwrap_or("");
        let mutated_gate = mutated_pipeline
            .as_ref()
            .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.gating_status.as_str())
            .unwrap_or("");
        let mut reasons = Vec::new();
        if mutated_balanced + 1e-9 < baseline_balanced {
            reasons.push("balanced_accuracy_regressed".to_string());
        }
        if mutated_bridge_gap + 1e-9 < baseline_bridge_gap {
            reasons.push("bridge_gap_regressed".to_string());
        }
        if pre_bayes_gate_regressed(baseline_gate, mutated_gate) {
            reasons.push("pre_bayes_gate_regressed".to_string());
        }
        if !reasons.is_empty() {
            reasons_by_market.insert((*market).to_string(), reasons);
        }
    }
    Ok(reasons_by_market)
}
