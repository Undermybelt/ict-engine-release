use anyhow::Result;
use std::collections::BTreeMap;

use crate::application::backtest::pre_bayes_soft_evidence_diff;
use crate::application::belief::{
    adapt_factor_pipeline_debug_report,
    build_expansion_factor_pipeline_report_with_registry as build_expansion_factor_pipeline_report_with_registry_v2,
    AdaptFactorPipelineDebugReportInput,
};
use crate::application::decision_utils::parse_research_objective;
use crate::application::decision_utils::research_objective_label;
use crate::application::multi_timeframe_inputs::{
    build_multi_timeframe_research_signal, build_multi_timeframe_summary,
    resolve_multi_timeframe_inputs,
};
use crate::config::env_f64;
use crate::data::load_candles;
use crate::factors::FactorRegistry;

pub struct FactorPipelineDebugCommandInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub factor: &'a str,
    pub objective: &'a str,
    pub data_1m: Option<&'a str>,
    pub data_5m: Option<&'a str>,
    pub data_15m: Option<&'a str>,
    pub data_1h: Option<&'a str>,
    pub data_4h: Option<&'a str>,
    pub data_1d: Option<&'a str>,
}

pub fn factor_pipeline_debug_command(input: FactorPipelineDebugCommandInput<'_>) -> Result<()> {
    let FactorPipelineDebugCommandInput {
        symbol,
        data,
        factor,
        objective,
        data_1m,
        data_5m,
        data_15m,
        data_1h,
        data_4h,
        data_1d,
    } = input;
    let objective_mode = parse_research_objective(objective)?;
    let resolved_multi_timeframe_inputs =
        resolve_multi_timeframe_inputs(data, data_1m, data_5m, data_15m, data_1h, data_4h, data_1d);
    let multi_timeframe_summary =
        build_multi_timeframe_summary(data, &resolved_multi_timeframe_inputs)?
            .into_iter()
            .chain(build_multi_timeframe_research_signal(&resolved_multi_timeframe_inputs)?.summary)
            .collect::<Vec<_>>();
    let candles = load_candles(data)?;
    let registry = FactorRegistry::default();
    let pipeline = build_expansion_factor_pipeline_report_with_registry_v2(
        symbol,
        factor,
        &candles,
        None,
        &multi_timeframe_summary,
        &registry,
    )?;
    let report = adapt_factor_pipeline_debug_report(AdaptFactorPipelineDebugReportInput {
        symbol,
        data,
        objective: research_objective_label(objective_mode),
        pipeline: &pipeline,
        multi_timeframe_summary: &multi_timeframe_summary,
        raw_pre_bayes_labels: BTreeMap::from([
            (
                "market_regime".to_string(),
                pipeline.bbn_support.market_regime_label.clone(),
            ),
            (
                "liquidity_context".to_string(),
                pipeline.bbn_support.liquidity_context_label.clone(),
            ),
            (
                "factor_alignment".to_string(),
                pipeline.probability_support.alignment_label.clone(),
            ),
            (
                "factor_uncertainty".to_string(),
                pipeline.probability_support.uncertainty_label.clone(),
            ),
            (
                "multi_timeframe_resonance".to_string(),
                pipeline
                    .bbn_support
                    .pre_bayes_filter
                    .raw_multi_timeframe_resonance_label
                    .clone(),
            ),
        ]),
        soft_evidence_divergence: pre_bayes_soft_evidence_diff(
            &pipeline.bbn_support.pre_bayes_filter,
        ),
        bridge_gap_clear_threshold: env_f64("ICT_ENGINE_BRIDGE_GAP_CLEAR_THRESHOLD", 0.12),
        paired_market_quality_report: None,
    })?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
