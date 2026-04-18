use anyhow::Result;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::analyze::multi_timeframe_parse::{
    multi_timeframe_direction_conflicts_with, ParsedMultiTimeframeEvidence,
};
use crate::bbn::adapters::belief_evidence_packet_from_pre_bayes_filter;
use crate::bbn::engine::InferenceEngineRegistry;
use crate::factor_lab::{FactorDiagnostics, PairedMarketQualityReport};
use crate::planner::ProbabilisticDecisionSnapshot;
use crate::reporting::belief::BeliefReportPacket;
use crate::state::{
    FactorPipelineLabelSource, PreBayesEntryQualityBridge, PreBayesEntryQualityBridgeDiff,
    PreBayesEvidenceFilter, PreBayesEvidencePolicy, PreBayesSoftEvidenceNodeDiff,
};
use crate::types::Direction;

use super::pipeline_types::ExpansionFactorPipelineReport;

#[cfg(test)]
#[path = "pipeline_shared_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize)]
pub struct ExpansionLatestSignal {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub direction: String,
    pub value: f64,
    pub confidence: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpansionProbabilitySupport {
    pub long_support: f64,
    pub short_support: f64,
    pub support_gap: f64,
    pub alignment_threshold: f64,
    pub uncertainty: f64,
    pub alignment_label: String,
    pub uncertainty_label: String,
    pub long_entry_bias: Vec<f64>,
    pub short_entry_bias: Vec<f64>,
    pub bullish_factors: Vec<crate::factor_lab::FactorContribution>,
    pub bearish_factors: Vec<crate::factor_lab::FactorContribution>,
    pub uncertainty_factors: Vec<crate::factor_lab::FactorContribution>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpansionBbnSupport {
    pub market_regime_label: String,
    pub liquidity_context_label: String,
    pub evidence_policy: String,
    pub pre_bayes_filter: PreBayesEvidenceFilter,
    pub evidence_assignments: BTreeMap<String, String>,
    pub raw_market_regime_trace: FactorPipelineLabelSource,
    pub raw_liquidity_context_trace: FactorPipelineLabelSource,
    pub raw_multi_timeframe_resonance_trace: FactorPipelineLabelSource,
    pub entry_quality_base: BTreeMap<String, f64>,
    pub entry_quality_long: BTreeMap<String, f64>,
    pub entry_quality_short: BTreeMap<String, f64>,
    pub trade_outcome_long: BTreeMap<String, f64>,
    pub trade_outcome_short: BTreeMap<String, f64>,
    pub selected_direction: String,
    pub selected_win_probability: f64,
}

#[derive(Debug, Clone)]
pub struct AdaptFactorPipelineDebugReportInput<'a> {
    pub symbol: &'a str,
    pub data: &'a str,
    pub objective: &'a str,
    pub pipeline: &'a ExpansionFactorPipelineReport,
    pub raw_pre_bayes_labels: BTreeMap<String, String>,
    pub soft_evidence_divergence: Vec<PreBayesSoftEvidenceNodeDiff>,
    pub bridge_gap_clear_threshold: f64,
    pub multi_timeframe_summary: &'a [String],
    pub paired_market_quality_report: Option<PairedMarketQualityReport>,
}

#[derive(Debug, Clone)]
pub struct FactorPipelineDebugReportInput {
    pub symbol: String,
    pub data: String,
    pub objective: String,
    pub factor_name: String,
    pub latest_signal: ExpansionLatestSignal,
    pub factor_diagnostics: ExpansionProbabilitySupport,
    pub bbn_support: ExpansionBbnSupport,
    pub entry_quality_bridge: PreBayesEntryQualityBridge,
    pub bridge_diff: PreBayesEntryQualityBridgeDiff,
    pub multi_timeframe_summary: Vec<String>,
    pub raw_pre_bayes_labels: BTreeMap<String, String>,
    pub soft_evidence_divergence: Vec<PreBayesSoftEvidenceNodeDiff>,
    pub bridge_gap_clear_threshold: f64,
    pub paired_market_quality_report: Option<PairedMarketQualityReport>,
}

#[derive(Debug, Clone)]
pub struct PreBayesEntryQualityBridgeInput {
    pub factor_diagnostics: FactorDiagnostics,
    pub decision: ProbabilisticDecisionSnapshot,
    pub long_entry_bias: Vec<f64>,
    pub short_entry_bias: Vec<f64>,
    pub long_entry_quality: Vec<f64>,
    pub short_entry_quality: Vec<f64>,
    pub selected_entry_quality: Vec<f64>,
    pub entry_quality_states: Vec<String>,
    pub multi_timeframe_evidence: ParsedMultiTimeframeEvidence,
}

pub fn adapt_factor_pipeline_debug_report(
    input: AdaptFactorPipelineDebugReportInput<'_>,
) -> Result<FactorPipelineDebugReport> {
    build_factor_pipeline_debug_report(FactorPipelineDebugReportInput {
        symbol: input.symbol.to_string(),
        data: input.data.to_string(),
        objective: input.objective.to_string(),
        factor_name: input.pipeline.factor_name.clone(),
        latest_signal: ExpansionLatestSignal {
            timestamp: input.pipeline.latest_signal.timestamp,
            direction: input.pipeline.latest_signal.direction.clone(),
            value: input.pipeline.latest_signal.value,
            confidence: input.pipeline.latest_signal.confidence,
            explanation: input.pipeline.latest_signal.explanation.clone(),
        },
        factor_diagnostics: ExpansionProbabilitySupport {
            long_support: input.pipeline.probability_support.long_support,
            short_support: input.pipeline.probability_support.short_support,
            support_gap: input.pipeline.probability_support.support_gap,
            alignment_threshold: input.pipeline.probability_support.alignment_threshold,
            uncertainty: input.pipeline.probability_support.uncertainty,
            alignment_label: input.pipeline.probability_support.alignment_label.clone(),
            uncertainty_label: input.pipeline.probability_support.uncertainty_label.clone(),
            long_entry_bias: input.pipeline.probability_support.long_entry_bias.clone(),
            short_entry_bias: input.pipeline.probability_support.short_entry_bias.clone(),
            bullish_factors: input.pipeline.probability_support.bullish_factors.clone(),
            bearish_factors: input.pipeline.probability_support.bearish_factors.clone(),
            uncertainty_factors: input
                .pipeline
                .probability_support
                .uncertainty_factors
                .clone(),
        },
        bbn_support: ExpansionBbnSupport {
            market_regime_label: input.pipeline.bbn_support.market_regime_label.clone(),
            liquidity_context_label: input.pipeline.bbn_support.liquidity_context_label.clone(),
            evidence_policy: input.pipeline.bbn_support.evidence_policy.clone(),
            pre_bayes_filter: input.pipeline.bbn_support.pre_bayes_filter.clone(),
            evidence_assignments: input.pipeline.bbn_support.evidence_assignments.clone(),
            raw_market_regime_trace: input.pipeline.bbn_support.raw_market_regime_trace.clone(),
            raw_liquidity_context_trace: input
                .pipeline
                .bbn_support
                .raw_liquidity_context_trace
                .clone(),
            raw_multi_timeframe_resonance_trace: input
                .pipeline
                .bbn_support
                .raw_multi_timeframe_resonance_trace
                .clone(),
            entry_quality_base: input.pipeline.bbn_support.entry_quality_base.clone(),
            entry_quality_long: input.pipeline.bbn_support.entry_quality_long.clone(),
            entry_quality_short: input.pipeline.bbn_support.entry_quality_short.clone(),
            trade_outcome_long: input.pipeline.bbn_support.trade_outcome_long.clone(),
            trade_outcome_short: input.pipeline.bbn_support.trade_outcome_short.clone(),
            selected_direction: input.pipeline.bbn_support.selected_direction.clone(),
            selected_win_probability: input.pipeline.bbn_support.selected_win_probability,
        },
        entry_quality_bridge: input.pipeline.entry_quality_bridge.clone(),
        bridge_diff: pre_bayes_entry_quality_bridge_diff(&input.pipeline.entry_quality_bridge),
        multi_timeframe_summary: input.multi_timeframe_summary.to_vec(),
        raw_pre_bayes_labels: input.raw_pre_bayes_labels,
        soft_evidence_divergence: input.soft_evidence_divergence,
        bridge_gap_clear_threshold: input.bridge_gap_clear_threshold,
        paired_market_quality_report: input.paired_market_quality_report.clone().or_else(|| {
            input
                .pipeline
                .paired_market_quality_report
                .clone()
                .or_else(|| {
                    paired_market_quality_report_from_explanation(
                        &input.pipeline.factor_name,
                        &input.pipeline.latest_signal.explanation,
                    )
                })
        }),
    })
}

pub(crate) fn paired_market_quality_report_from_explanation(
    factor_name: &str,
    explanation: &str,
) -> Option<PairedMarketQualityReport> {
    if factor_name != "cross_market_smt" {
        return None;
    }
    let mut fields = BTreeMap::new();
    for part in explanation.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        fields.insert(key.trim().to_string(), value.trim().to_string());
    }
    let status = fields.get("status")?.clone();
    let paired_market_quality = fields.get("quality_tier")?.clone();
    let reason = fields.get("reason")?.clone();
    let aligned_length = fields.get("aligned_length")?.parse().ok()?;
    let primary_length = fields.get("primary_length")?.parse().ok()?;
    let paired_length = fields.get("paired_length")?.parse().ok()?;
    let overlap_ratio = fields.get("overlap_ratio")?.parse().ok()?;
    let safe_lookback = fields.get("safe_lookback")?.parse().ok()?;
    Some(PairedMarketQualityReport {
        paired_market_quality,
        aligned_length,
        primary_length,
        paired_length,
        overlap_ratio,
        safe_lookback,
        status,
        reason,
    })
}

pub fn build_canonical_belief_report(
    symbol: &str,
    market: Option<&str>,
    filter: &PreBayesEvidenceFilter,
    raw_market_regime_trace: Option<&FactorPipelineLabelSource>,
    raw_liquidity_context_trace: Option<&FactorPipelineLabelSource>,
    raw_multi_timeframe_resonance_trace: Option<&FactorPipelineLabelSource>,
) -> Result<BeliefReportPacket> {
    InferenceEngineRegistry::default().build_report(belief_evidence_packet_from_pre_bayes_filter(
        symbol,
        market,
        filter,
        raw_market_regime_trace,
        raw_liquidity_context_trace,
        raw_multi_timeframe_resonance_trace,
    ))
}

pub fn build_canonical_belief_snapshot(
    symbol: &str,
    market: Option<&str>,
    filter: &PreBayesEvidenceFilter,
) -> Result<BeliefReportPacket> {
    build_canonical_belief_report(symbol, market, filter, None, None, None)
}

pub fn market_category_from_symbol(symbol: &str) -> &'static str {
    match symbol
        .split(['.', '_', '-'])
        .next()
        .unwrap_or(symbol)
        .to_ascii_uppercase()
        .as_str()
    {
        "NQ" | "ES" | "YM" => "futures_index",
        "GC" => "metals",
        "CL" => "energy",
        _ => "generic",
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct FactorPipelineDebugReport {
    pub symbol: String,
    pub data: String,
    pub factor_name: String,
    pub objective: String,
    pub latest_signal: ExpansionLatestSignal,
    pub factor_diagnostics: ExpansionProbabilitySupport,
    pub raw_label_trace: FactorPipelineRawLabelTrace,
    pub raw_pre_bayes_labels: BTreeMap<String, String>,
    pub filtered_pre_bayes_labels: BTreeMap<String, String>,
    pub evidence_quality_score: f64,
    pub gating_status: String,
    pub soft_evidence_divergence: Vec<PreBayesSoftEvidenceNodeDiff>,
    pub bridge_gap: f64,
    pub selected_entry_quality: String,
    pub six_timeframe_resonance: Vec<String>,
    pub pipeline_verdict: String,
    pub pipeline_summary: String,
    pub recommended_actions: Vec<String>,
    pub frame_physics_trace: BTreeMap<String, f64>,
    pub paired_market_quality_report: Option<PairedMarketQualityReport>,
    pub entry_quality_bridge: PreBayesEntryQualityBridge,
    pub bbn_support: ExpansionBbnSupport,
    pub shadow_belief_report: BeliefReportPacket,
    pub shadow_summary_line: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FactorPipelineRawLabelTrace {
    pub market_regime: FactorPipelineLabelSource,
    pub liquidity_context: FactorPipelineLabelSource,
    pub multi_timeframe_resonance: FactorPipelineLabelSource,
}

fn summarize_frame_physics_trace(frame_physics_trace: &BTreeMap<String, f64>) -> Option<String> {
    let range_mid = frame_physics_trace
        .get("distance_to_range_mid_bps")
        .copied();
    let projected = frame_physics_trace
        .get("distance_to_projected_trend_bps")
        .copied();
    let half_life = frame_physics_trace.get("ou_half_life_bars").copied();
    let reversion = frame_physics_trace
        .get("ou_reversion_speed_per_bar")
        .copied();
    let pullback = frame_physics_trace
        .get("ou_pullback_expectation_zscore")
        .copied();
    let pythagorean_speed = frame_physics_trace
        .get("pythagorean_speed_bps_per_bar")
        .copied();
    let pythagorean_sweep = frame_physics_trace
        .get("pythagorean_distance_to_last_sweep")
        .copied();
    let pythagorean_fvg = frame_physics_trace
        .get("pythagorean_distance_to_last_fvg")
        .copied();
    let ou_target = frame_physics_trace
        .get("ou_mean_reversion_target_bps")
        .copied();
    let ou_pullback_bps = frame_physics_trace.get("ou_expected_pullback_bps").copied();

    let mut notes = Vec::new();
    if let (Some(range_mid), Some(pullback)) = (range_mid, pullback) {
        if range_mid.abs() >= 800.0 && pullback <= -2.0 {
            notes.push(
                "price is stretched above range mid and OU pullback pressure is elevated"
                    .to_string(),
            );
        } else if range_mid.abs() >= 800.0 && pullback >= 2.0 {
            notes.push(
                "price is stretched below range mid and OU rebound pressure is elevated"
                    .to_string(),
            );
        } else if range_mid.abs() <= 250.0 {
            notes.push(
                "price is still close to range mid, so extension pressure is limited".to_string(),
            );
        }
    }
    if let Some(projected) = projected {
        if projected.abs() <= 150.0 {
            notes.push("price remains close to the projected trend path, so continuation structure is intact".to_string());
        } else if projected.abs() >= 800.0 {
            notes.push(
                "price is far from the projected trend path, so continuation risk is degraded"
                    .to_string(),
            );
        }
    }
    if let (Some(half_life), Some(reversion)) = (half_life, reversion) {
        if half_life >= 1000.0 || reversion <= 0.001 {
            notes.push(
                "OU reversion is slow, so mean-reversion may take many bars to resolve".to_string(),
            );
        } else if half_life <= 50.0 {
            notes.push(
                "OU reversion is fast, so pullback pressure should resolve quickly if it triggers"
                    .to_string(),
            );
        }
    }
    if let Some(speed) = pythagorean_speed {
        if speed >= 50.0 {
            notes.push(format!(
                "Pythagorean speed is elevated ({:.1} bps/bar), indicating fast combined price+time displacement",
                speed
            ));
        } else if speed <= 5.0 {
            notes.push(format!(
                "Pythagorean speed is low ({:.1} bps/bar), indicating slow combined price+time displacement",
                speed
            ));
        }
    }
    if let (Some(sweep_dist), Some(fvg_dist)) = (pythagorean_sweep, pythagorean_fvg) {
        if sweep_dist.is_finite() && fvg_dist.is_finite() {
            if sweep_dist < fvg_dist {
                notes.push(format!(
                    "closer to last liquidity sweep ({:.0}) than last FVG ({:.0}) in Pythagorean distance",
                    sweep_dist, fvg_dist
                ));
            } else {
                notes.push(format!(
                    "closer to last FVG ({:.0}) than last liquidity sweep ({:.0}) in Pythagorean distance",
                    fvg_dist, sweep_dist
                ));
            }
        }
    }
    if let (Some(target), Some(pullback_bps)) = (ou_target, ou_pullback_bps) {
        if target.abs() >= 500.0 && pullback_bps >= 100.0 {
            notes.push(format!(
                "OU mean reversion target is {:.0} bps away with expected pullback {:.0} bps",
                target, pullback_bps
            ));
        }
    }

    if notes.is_empty() {
        None
    } else {
        Some(notes.join(" "))
    }
}

fn extract_frame_physics_metrics(
    trace: &[crate::state::FactorPipelineLabelSource],
) -> BTreeMap<String, f64> {
    let mut metrics = BTreeMap::new();
    for source in trace {
        for evidence in &source.evidence {
            if let Some((key, value)) = evidence.split_once('=') {
                if let Ok(parsed) = value.parse::<f64>() {
                    metrics.insert(key.to_string(), parsed);
                }
            }
        }
    }
    metrics
}

pub fn build_factor_pipeline_debug_report(
    input: FactorPipelineDebugReportInput,
) -> Result<FactorPipelineDebugReport> {
    let FactorPipelineDebugReportInput {
        symbol,
        data,
        objective,
        factor_name,
        latest_signal,
        factor_diagnostics,
        bbn_support,
        entry_quality_bridge,
        bridge_diff,
        multi_timeframe_summary,
        raw_pre_bayes_labels,
        soft_evidence_divergence,
        bridge_gap_clear_threshold,
        paired_market_quality_report,
    } = input;

    let filtered_pre_bayes_labels = bbn_support.evidence_assignments.clone();
    let gating_status = bbn_support.pre_bayes_filter.gating_status.clone();
    let selected_entry_quality = bridge_diff
        .selected_entry_quality
        .clone()
        .unwrap_or_else(|| "entry_quality_unavailable".to_string());
    let bridge_gap = bridge_diff.long_short_signal_probability_gap;
    let pipeline_verdict =
        if is_hard_pass(&gating_status) && bridge_gap >= bridge_gap_clear_threshold {
            "clear_through_pre_bayes_and_bridge".to_string()
        } else if gating_status == "pass_neutralized" {
            "pre_bayes_pass_but_bridge_needs_confirmation".to_string()
        } else if gating_status == "observe_only" {
            "blocked_at_pre_bayes_gate".to_string()
        } else if is_hard_pass(&gating_status) {
            "pre_bayes_pass_hard_but_bridge_gap_insufficient".to_string()
        } else {
            "pipeline_unclear".to_string()
        };

    let shadow_belief_report = build_canonical_belief_report(
        &symbol,
        Some(&data),
        &bbn_support.pre_bayes_filter,
        Some(&bbn_support.raw_market_regime_trace),
        Some(&bbn_support.raw_liquidity_context_trace),
        Some(&bbn_support.raw_multi_timeframe_resonance_trace),
    )?;
    let shadow_summary_line = shadow_belief_report
        .shadow_comparison
        .as_ref()
        .map(|summary| summary.summary_line.clone())
        .unwrap_or_else(|| "shadow=unavailable".to_string());
    let frame_physics_trace = extract_frame_physics_metrics(&[
        bbn_support.raw_market_regime_trace.clone(),
        bbn_support.raw_liquidity_context_trace.clone(),
    ]);
    let mut recommended_actions = vec![format!("inspect_factor={factor_name}")];
    if let Some(summary) = summarize_frame_physics_trace(&frame_physics_trace) {
        recommended_actions.push(format!("frame_physics_summary={summary}"));
    }

    Ok(FactorPipelineDebugReport {
        symbol,
        data,
        factor_name: factor_name.clone(),
        objective,
        latest_signal,
        factor_diagnostics,
        raw_label_trace: FactorPipelineRawLabelTrace {
            market_regime: bbn_support.raw_market_regime_trace.clone(),
            liquidity_context: bbn_support.raw_liquidity_context_trace.clone(),
            multi_timeframe_resonance: bbn_support.raw_multi_timeframe_resonance_trace.clone(),
        },
        raw_pre_bayes_labels,
        filtered_pre_bayes_labels,
        evidence_quality_score: bbn_support.pre_bayes_filter.evidence_quality_score,
        gating_status,
        soft_evidence_divergence,
        bridge_gap,
        selected_entry_quality,
        six_timeframe_resonance: multi_timeframe_summary,
        pipeline_verdict,
        pipeline_summary: bbn_support.evidence_policy.clone(),
        recommended_actions,
        frame_physics_trace,
        paired_market_quality_report,
        entry_quality_bridge,
        bbn_support,
        shadow_belief_report,
        shadow_summary_line,
    })
}

fn is_hard_pass(status: &str) -> bool {
    status == "pass_hard"
}

fn pre_bayes_entry_quality_bridge_diff(
    bridge: &PreBayesEntryQualityBridge,
) -> PreBayesEntryQualityBridgeDiff {
    let (dominant_long_entry_quality, dominant_long_entry_quality_probability) =
        max_probability_label(&bridge.long_entry_quality);
    let (dominant_short_entry_quality, dominant_short_entry_quality_probability) =
        max_probability_label(&bridge.short_entry_quality);
    let (selected_entry_quality, selected_entry_quality_probability) =
        if bridge.selected_entry_quality.is_empty() {
            let fallback = dominant_long_entry_quality
                .clone()
                .or_else(|| dominant_short_entry_quality.clone());
            let probability = if fallback == dominant_long_entry_quality {
                dominant_long_entry_quality_probability
            } else {
                dominant_short_entry_quality_probability
            };
            (fallback, probability)
        } else {
            max_probability_label(&bridge.selected_entry_quality)
        };

    PreBayesEntryQualityBridgeDiff {
        dominant_long_entry_quality,
        dominant_long_entry_quality_probability,
        dominant_short_entry_quality,
        dominant_short_entry_quality_probability,
        selected_entry_quality,
        selected_entry_quality_probability,
        long_short_signal_probability_gap: (bridge.long_signal_probability
            - bridge.short_signal_probability)
            .abs(),
        multi_timeframe_direction_bias: bridge.multi_timeframe_direction_bias.clone(),
        multi_timeframe_alignment_score: bridge.multi_timeframe_alignment_score,
        multi_timeframe_entry_alignment_score: bridge.multi_timeframe_entry_alignment_score,
        rationale_summary: bridge.rationale.clone(),
    }
}

fn max_probability_label(probabilities: &BTreeMap<String, f64>) -> (Option<String>, f64) {
    probabilities
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
        .map(|(label, value)| (Some(label.clone()), *value))
        .unwrap_or((None, 0.0))
}

pub fn raw_market_regime_trace(
    regime_label: &str,
    regime_evidence_label: &str,
    sweep_count: usize,
    fvg_count: usize,
    normalized_distance_to_range_mid_bps: f64,
    normalized_distance_to_projected_trend_bps: f64,
    ou_half_life_bars: f64,
    ou_reversion_speed_per_bar: f64,
    ou_pullback_expectation_zscore: f64,
) -> FactorPipelineLabelSource {
    FactorPipelineLabelSource {
        label: regime_label.to_string(),
        derivation: "build_frame_features.regime_label".to_string(),
        evidence: vec![
            format!("frame_regime_label={}", regime_evidence_label),
            format!("sweep_count={}", sweep_count),
            format!("fvg_count={}", fvg_count),
            format!(
                "distance_to_range_mid_bps={:.4}",
                normalized_distance_to_range_mid_bps
            ),
            format!(
                "distance_to_projected_trend_bps={:.4}",
                normalized_distance_to_projected_trend_bps
            ),
            format!("ou_half_life_bars={:.4}", ou_half_life_bars),
            format!(
                "ou_reversion_speed_per_bar={:.4}",
                ou_reversion_speed_per_bar
            ),
            format!(
                "ou_pullback_expectation_zscore={:.4}",
                ou_pullback_expectation_zscore
            ),
        ],
    }
}

pub fn raw_liquidity_context_trace(
    liquidity_label: &str,
    liquidity_evidence_label: &str,
    sweep_count: usize,
    fvg_count: usize,
    normalized_distance_to_range_mid_bps: f64,
    normalized_distance_to_projected_trend_bps: f64,
    ou_half_life_bars: f64,
    ou_reversion_speed_per_bar: f64,
    ou_pullback_expectation_zscore: f64,
) -> FactorPipelineLabelSource {
    FactorPipelineLabelSource {
        label: liquidity_label.to_string(),
        derivation: "build_frame_features.liquidity_label".to_string(),
        evidence: vec![
            format!("frame_liquidity_label={}", liquidity_evidence_label),
            format!("sweep_count={}", sweep_count),
            format!("fvg_count={}", fvg_count),
            format!(
                "distance_to_range_mid_bps={:.4}",
                normalized_distance_to_range_mid_bps
            ),
            format!(
                "distance_to_projected_trend_bps={:.4}",
                normalized_distance_to_projected_trend_bps
            ),
            format!("ou_half_life_bars={:.4}", ou_half_life_bars),
            format!(
                "ou_reversion_speed_per_bar={:.4}",
                ou_reversion_speed_per_bar
            ),
            format!(
                "ou_pullback_expectation_zscore={:.4}",
                ou_pullback_expectation_zscore
            ),
        ],
    }
}

pub fn raw_multi_timeframe_resonance_trace(
    policy: &PreBayesEvidencePolicy,
    pre_bayes_filter: &PreBayesEvidenceFilter,
    multi_timeframe_evidence: &ParsedMultiTimeframeEvidence,
    regime_label: &str,
    factor_alignment_label: &str,
) -> FactorPipelineLabelSource {
    let direction_conflict = multi_timeframe_direction_conflicts_with(
        regime_label,
        &multi_timeframe_evidence.direction_bias,
    ) || multi_timeframe_direction_conflicts_with(
        factor_alignment_label,
        &multi_timeframe_evidence.direction_bias,
    );

    FactorPipelineLabelSource {
        label: pre_bayes_filter.raw_multi_timeframe_resonance_label.clone(),
        derivation: "classify_multi_timeframe_resonance(policy, direction_conflict, parsed_multi_timeframe_evidence)".to_string(),
        evidence: vec![
            format!("direction_bias={}", multi_timeframe_evidence.direction_bias),
            format!(
                "alignment_score={:.4}",
                multi_timeframe_evidence.alignment_score.unwrap_or_default()
            ),
            format!(
                "entry_alignment_score={:.4}",
                multi_timeframe_evidence.entry_alignment_score.unwrap_or_default()
            ),
            format!("direction_conflict={}", direction_conflict),
            format!(
                "alignment_floor={:.4}",
                policy.min_multi_timeframe_alignment_score
            ),
            format!(
                "entry_alignment_floor={:.4}",
                policy.min_multi_timeframe_entry_alignment_score
            ),
        ],
    }
}

pub fn multi_timeframe_entry_quality_bias(
    evidence: &ParsedMultiTimeframeEvidence,
    direction: Direction,
) -> Vec<f64> {
    let alignment_score = evidence.alignment_score.unwrap_or(0.5).clamp(0.0, 1.0);
    let entry_alignment_score = evidence
        .entry_alignment_score
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let supportive = matches!(
        (direction, evidence.direction_bias.as_str()),
        (Direction::Bull, "bullish") | (Direction::Bear, "bearish")
    );
    let hostile = matches!(
        (direction, evidence.direction_bias.as_str()),
        (Direction::Bull, "bearish") | (Direction::Bear, "bullish")
    );

    let mut bias = vec![1.0, 1.0, 1.0];
    if supportive {
        bias[0] *= 1.0 + alignment_score * 0.45 + entry_alignment_score * 0.25;
        bias[1] *= 1.0 + alignment_score * 0.10;
        bias[2] *= (1.0 - alignment_score * 0.30 - entry_alignment_score * 0.20).max(0.20);
    } else if hostile {
        bias[0] *= (1.0 - alignment_score * 0.30).max(0.25);
        bias[1] *= 1.0 + (1.0 - entry_alignment_score) * 0.15;
        bias[2] *= 1.0 + alignment_score * 0.40 + (1.0 - entry_alignment_score) * 0.20;
    } else {
        bias[0] *= 1.0 + alignment_score * 0.08;
        bias[1] *= 1.0 + entry_alignment_score * 0.12;
    }
    normalize_distribution(&mut bias);
    bias
}

pub fn effective_trade_outcome_win_probability(trade_outcome: &[f64]) -> f64 {
    match trade_outcome {
        [win, breakeven, ..] => (win + 0.5 * breakeven).clamp(0.0, 0.999),
        [win] => (*win).clamp(0.0, 0.999),
        _ => 0.0,
    }
}

pub fn build_pre_bayes_entry_quality_bridge(
    input: PreBayesEntryQualityBridgeInput,
) -> PreBayesEntryQualityBridge {
    let PreBayesEntryQualityBridgeInput {
        factor_diagnostics,
        decision,
        long_entry_bias,
        short_entry_bias,
        long_entry_quality,
        short_entry_quality,
        selected_entry_quality,
        entry_quality_states,
        multi_timeframe_evidence,
    } = input;

    PreBayesEntryQualityBridge {
        long_signal_probability: decision.win_prob_long,
        short_signal_probability: decision.win_prob_short,
        long_entry_bias,
        short_entry_bias,
        long_entry_quality: probability_map(&entry_quality_states, &long_entry_quality),
        short_entry_quality: probability_map(&entry_quality_states, &short_entry_quality),
        selected_entry_quality: probability_map(&entry_quality_states, &selected_entry_quality),
        multi_timeframe_direction_bias: multi_timeframe_evidence.direction_bias.clone(),
        multi_timeframe_alignment_score: multi_timeframe_evidence.alignment_score,
        multi_timeframe_entry_alignment_score: multi_timeframe_evidence.entry_alignment_score,
        rationale: vec![
            format!(
                "factor_alignment={} factor_uncertainty={}",
                factor_diagnostics.alignment_label, factor_diagnostics.uncertainty_label
            ),
            format!(
                "long_support={:.3} short_support={:.3} uncertainty={:.3}",
                factor_diagnostics.long_support,
                factor_diagnostics.short_support,
                factor_diagnostics.uncertainty
            ),
            format!(
                "multi_timeframe_direction_bias={} multi_timeframe_alignment_score={:.3} multi_timeframe_entry_alignment_score={:.3}",
                multi_timeframe_evidence.direction_bias,
                multi_timeframe_evidence.alignment_score.unwrap_or_default(),
                multi_timeframe_evidence.entry_alignment_score.unwrap_or_default()
            ),
            "entry_quality_bias combines directional factor support with cascade probability bias"
                .to_string(),
        ],
    }
}

pub fn probability_map(states: &[String], probabilities: &[f64]) -> BTreeMap<String, f64> {
    states
        .iter()
        .cloned()
        .zip(probabilities.iter().copied())
        .collect()
}

pub fn combine_bias_vectors(left: &[f64], right: &[f64]) -> Vec<f64> {
    let len = left.len().max(right.len());
    let mut combined = vec![1.0; len];
    for (index, value) in combined.iter_mut().enumerate().take(len) {
        let left_value = left.get(index).copied().unwrap_or(1.0 / len as f64);
        let right_value = right.get(index).copied().unwrap_or(1.0 / len as f64);
        *value = (left_value * right_value).max(1e-6);
    }
    normalize_distribution(&mut combined);
    combined
}

pub fn apply_factor_outcome_overlay(
    distribution: &[f64],
    directional_bias: f64,
    uncertainty: f64,
) -> Vec<f64> {
    let mut adjusted = distribution.to_vec();
    if adjusted.len() < 3 {
        return adjusted;
    }

    adjusted[0] *= (1.0 + directional_bias * 0.35 - uncertainty * 0.10).max(0.05);
    adjusted[1] *= (1.0 + uncertainty * 0.20).max(0.05);
    adjusted[2] *= (1.0 - directional_bias * 0.35 + uncertainty * 0.30).max(0.05);
    normalize_distribution(&mut adjusted);
    adjusted
}

pub fn normalize_distribution(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum <= f64::EPSILON {
        let uniform = 1.0 / values.len() as f64;
        values.fill(uniform);
        return;
    }
    for value in values {
        *value /= sum;
    }
}
