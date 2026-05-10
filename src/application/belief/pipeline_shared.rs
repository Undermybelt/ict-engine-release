use anyhow::Result;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::analyze::multi_timeframe_parse::{
    multi_timeframe_direction_conflicts_with, ParsedMultiTimeframeEvidence,
};
use crate::application::belief::blend_node_posterior_with_duration_prior;
use crate::application::belief::transition_adjusted_branch_posteriors;
use crate::application::belief::transition_adjusted_node_posteriors;
use crate::application::entry_models::{apply_cisd_rb_to_belief_packet, CisdRbEntryModelPacket};
use crate::bbn::adapters::{
    belief_evidence_packet_from_pre_bayes_filter, gate_decision_from_regime_posterior,
};
use crate::bbn::engine::InferenceEngineRegistry;
use crate::domain::regime::RegimeSegmentationPacket;
use crate::factor_lab::{FactorDiagnostics, PairedMarketQualityReport};
use crate::pda_sequence::PdaSequenceAnalysisArtifact;
use crate::planner::ProbabilisticDecisionSnapshot;
use crate::reporting::belief::BeliefReportPacket;
use crate::state::{
    FactorPipelineLabelSource, PreBayesEntryQualityBridge, PreBayesEntryQualityBridgeDiff,
    PreBayesEvidenceFilter, PreBayesEvidencePolicy, PreBayesSoftEvidenceNodeDiff,
    StructuralPriorEvent, StructuralPriorLearningState,
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
    build_canonical_belief_report_with_pda(
        symbol,
        market,
        filter,
        raw_market_regime_trace,
        raw_liquidity_context_trace,
        raw_multi_timeframe_resonance_trace,
        None,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_canonical_belief_report_with_pda(
    symbol: &str,
    market: Option<&str>,
    filter: &PreBayesEvidenceFilter,
    raw_market_regime_trace: Option<&FactorPipelineLabelSource>,
    raw_liquidity_context_trace: Option<&FactorPipelineLabelSource>,
    raw_multi_timeframe_resonance_trace: Option<&FactorPipelineLabelSource>,
    pda_sequence_artifact: Option<&PdaSequenceAnalysisArtifact>,
    hybrid_regime_packet: Option<&RegimeSegmentationPacket>,
    cisd_rb_packet: Option<&CisdRbEntryModelPacket>,
) -> Result<BeliefReportPacket> {
    build_canonical_belief_report_with_pda_and_structural_prior_state(
        symbol,
        market,
        filter,
        raw_market_regime_trace,
        raw_liquidity_context_trace,
        raw_multi_timeframe_resonance_trace,
        pda_sequence_artifact,
        hybrid_regime_packet,
        cisd_rb_packet,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_canonical_belief_report_with_pda_and_structural_prior_state(
    symbol: &str,
    market: Option<&str>,
    filter: &PreBayesEvidenceFilter,
    raw_market_regime_trace: Option<&FactorPipelineLabelSource>,
    raw_liquidity_context_trace: Option<&FactorPipelineLabelSource>,
    raw_multi_timeframe_resonance_trace: Option<&FactorPipelineLabelSource>,
    pda_sequence_artifact: Option<&PdaSequenceAnalysisArtifact>,
    hybrid_regime_packet: Option<&RegimeSegmentationPacket>,
    cisd_rb_packet: Option<&CisdRbEntryModelPacket>,
    structural_prior_state: Option<&StructuralPriorLearningState>,
) -> Result<BeliefReportPacket> {
    let mut packet = belief_evidence_packet_from_pre_bayes_filter(
        symbol,
        market,
        filter,
        raw_market_regime_trace,
        raw_liquidity_context_trace,
        raw_multi_timeframe_resonance_trace,
    );
    if let Some(artifact) = pda_sequence_artifact {
        crate::bbn::adapters::apply_pda_sequence_artifact_to_belief_packet(&mut packet, artifact);
    }
    if let Some(hybrid) = hybrid_regime_packet {
        crate::bbn::adapters::apply_hybrid_regime_packet_to_belief_packet(&mut packet, hybrid);
    }
    if let Some(cisd_rb_packet) = cisd_rb_packet {
        apply_cisd_rb_to_belief_packet(cisd_rb_packet, &mut packet);
    }
    let mut report = InferenceEngineRegistry::default().build_report(packet)?;
    if let Some(structural_prior_state) = structural_prior_state {
        apply_structural_prior_state_to_belief_report(
            symbol,
            filter,
            structural_prior_state,
            &mut report,
        );
    }
    Ok(report)
}

pub fn build_canonical_belief_snapshot(
    symbol: &str,
    market: Option<&str>,
    filter: &PreBayesEvidenceFilter,
) -> Result<BeliefReportPacket> {
    build_canonical_belief_snapshot_with_pda(symbol, market, filter, None, None, None)
}

pub fn build_canonical_belief_snapshot_with_pda(
    symbol: &str,
    market: Option<&str>,
    filter: &PreBayesEvidenceFilter,
    pda_sequence_artifact: Option<&PdaSequenceAnalysisArtifact>,
    hybrid_regime_packet: Option<&RegimeSegmentationPacket>,
    cisd_rb_packet: Option<&CisdRbEntryModelPacket>,
) -> Result<BeliefReportPacket> {
    build_canonical_belief_snapshot_with_pda_and_structural_prior_state(
        symbol,
        market,
        filter,
        pda_sequence_artifact,
        hybrid_regime_packet,
        cisd_rb_packet,
        None,
    )
}

pub fn build_canonical_belief_snapshot_with_pda_and_structural_prior_state(
    symbol: &str,
    market: Option<&str>,
    filter: &PreBayesEvidenceFilter,
    pda_sequence_artifact: Option<&PdaSequenceAnalysisArtifact>,
    hybrid_regime_packet: Option<&RegimeSegmentationPacket>,
    cisd_rb_packet: Option<&CisdRbEntryModelPacket>,
    structural_prior_state: Option<&StructuralPriorLearningState>,
) -> Result<BeliefReportPacket> {
    build_canonical_belief_report_with_pda_and_structural_prior_state(
        symbol,
        market,
        filter,
        None,
        None,
        None,
        pda_sequence_artifact,
        hybrid_regime_packet,
        cisd_rb_packet,
        structural_prior_state,
    )
}

fn apply_structural_prior_state_to_belief_report(
    symbol: &str,
    filter: &PreBayesEvidenceFilter,
    structural_prior_state: &StructuralPriorLearningState,
    report: &mut BeliefReportPacket,
) {
    let mut canonical_probabilities = BTreeMap::new();
    for (regime, probability) in &report.regime_posterior.probabilities {
        if let Some(canonical) = canonical_structural_regime_label(regime) {
            *canonical_probabilities.entry(canonical).or_insert(0.0) += *probability;
        }
    }
    if filter.uses_soft_evidence && !filter.soft_market_regime_distribution.is_empty() {
        let trend = filter
            .soft_market_regime_distribution
            .get("bull")
            .copied()
            .unwrap_or(0.0)
            + filter
                .soft_market_regime_distribution
                .get("bear")
                .copied()
                .unwrap_or(0.0);
        let range = filter
            .soft_market_regime_distribution
            .get("range")
            .copied()
            .unwrap_or(0.0);
        canonical_probabilities.insert("trend".to_string(), trend.clamp(0.0, 1.0));
        canonical_probabilities.insert("range".to_string(), range.clamp(0.0, 1.0));
    }
    if canonical_probabilities.is_empty() {
        return;
    }

    for (regime, probability) in canonical_probabilities.iter_mut() {
        let node_id = format!("{symbol}:belief_regime_node:{regime}");
        let node_temporal_state = structural_prior_state
            .node_temporal_posteriors
            .get(&node_id);
        if let Some(duration_prior) = structural_prior_state.node_duration_priors.get(&node_id) {
            *probability = blend_node_posterior_with_duration_prior(
                *probability,
                Some(duration_prior),
                node_temporal_state,
            );
        }
    }

    let latest_branch_id =
        latest_structural_branch_id_for_symbol(&structural_prior_state.event_ledger, symbol);
    if let Some(latest_branch_id) = latest_branch_id.as_deref() {
        let regime_probabilities = canonical_probabilities
            .iter()
            .map(|(regime, probability)| (regime.clone(), *probability))
            .collect::<Vec<_>>();
        let adjusted_node_probabilities = transition_adjusted_node_posteriors(
            symbol,
            &regime_probabilities,
            Some(latest_branch_id),
            &structural_prior_state.branch_transition_priors,
            &structural_prior_state.branch_temporal_posteriors,
            &structural_prior_state.node_transition_posteriors,
        );
        let node_transition_adjusted =
            adjusted_node_probabilities
                .iter()
                .any(|(regime, probability)| {
                    (canonical_probabilities
                        .get(regime)
                        .copied()
                        .unwrap_or_default()
                        - *probability)
                        .abs()
                        > 1e-9
                });
        canonical_probabilities = adjusted_node_probabilities;
        if node_transition_adjusted {
            report
                .regime_posterior
                .evidence
                .push(format!("node_transition_posterior_from={latest_branch_id}"));
        }
    }

    let mut active_regime = canonical_probabilities
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
        .map(|(regime, _)| regime.clone());

    if let Some(active) = active_regime.as_ref() {
        let node_id = format!("{symbol}:belief_regime_node:{active}");
        let regime_probabilities = canonical_probabilities
            .iter()
            .map(|(regime, probability)| (regime.clone(), *probability))
            .collect::<Vec<_>>();
        if let Some(latest_branch_id) = latest_branch_id.as_deref() {
            let adjusted = transition_adjusted_branch_posteriors(
                &node_id,
                &regime_probabilities,
                Some(latest_branch_id),
                &structural_prior_state.branch_transition_priors,
                &structural_prior_state.branch_temporal_posteriors,
                structural_branch_label_for_regime,
            );
            for (regime, _) in &regime_probabilities {
                let branch_id = format!(
                    "{node_id}:{}",
                    structural_branch_label_for_regime(regime.as_str())
                );
                if let Some(probability) = adjusted.get(&branch_id) {
                    canonical_probabilities.insert(regime.clone(), *probability);
                }
            }
            let max_weighted_transition_mass = regime_probabilities
                .iter()
                .filter_map(|(regime, _)| {
                    let branch_id = format!(
                        "{}:{}",
                        node_id,
                        structural_branch_label_for_regime(regime.as_str())
                    );
                    structural_prior_state
                        .branch_transition_priors
                        .get(&format!("{latest_branch_id}=>{branch_id}"))
                        .map(|prior| prior.weighted_observation_mass)
                })
                .fold(0.0, f64::max);
            let max_transition_outcome_support = regime_probabilities
                .iter()
                .filter_map(|(regime, _)| {
                    let branch_id = format!(
                        "{}:{}",
                        node_id,
                        structural_branch_label_for_regime(regime.as_str())
                    );
                    let transition_key = format!("{latest_branch_id}=>{branch_id}");
                    structural_prior_state
                        .branch_temporal_posteriors
                        .get(&transition_key)
                        .map(|state| state.transition_outcome_support)
                        .or_else(|| {
                            structural_prior_state
                                .branch_transition_priors
                                .get(&transition_key)
                                .map(|prior| prior.transition_outcome_support)
                        })
                })
                .fold(0.0, f64::max);
            let max_transition_temporal_support = regime_probabilities
                .iter()
                .filter_map(|(regime, _)| {
                    let branch_id = format!(
                        "{}:{}",
                        node_id,
                        structural_branch_label_for_regime(regime.as_str())
                    );
                    let transition_key = format!("{latest_branch_id}=>{branch_id}");
                    structural_prior_state
                        .branch_temporal_posteriors
                        .get(&transition_key)
                        .map(|state| state.temporal_posterior_support)
                        .or_else(|| {
                            structural_prior_state
                                .branch_transition_priors
                                .get(&transition_key)
                                .map(|prior| prior.temporal_posterior_support)
                        })
                })
                .fold(0.0, f64::max);
            report
                .regime_posterior
                .evidence
                .push(format!(
                    "branch_transition_prior_from={} weighted_transition_mass={:.3} transition_outcome_support={:.3} transition_temporal_posterior_support={:.3}",
                    latest_branch_id,
                    max_weighted_transition_mass,
                    max_transition_outcome_support,
                    max_transition_temporal_support
                ));
            if let Some(max_transition_multiplier) = regime_probabilities
                .iter()
                .filter_map(|(regime, _)| {
                    let branch_id = format!(
                        "{}:{}",
                        node_id,
                        structural_branch_label_for_regime(regime.as_str())
                    );
                    structural_prior_state
                        .branch_temporal_posteriors
                        .get(&format!("{latest_branch_id}=>{branch_id}"))
                        .map(|state| state.posterior_multiplier)
                })
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            {
                report.regime_posterior.evidence.push(format!(
                    "transition_posterior_multiplier={max_transition_multiplier:.3}"
                ));
            }
            if let Some(summary_line) = regime_probabilities
                .iter()
                .filter_map(|(regime, _)| {
                    let branch_id = format!(
                        "{}:{}",
                        node_id,
                        structural_branch_label_for_regime(regime.as_str())
                    );
                    structural_prior_state
                        .branch_temporal_posteriors
                        .get(&format!("{latest_branch_id}=>{branch_id}"))
                        .map(|state| state.summary_line.clone())
                })
                .max()
            {
                report
                    .regime_posterior
                    .evidence
                    .push(format!("branch_temporal_summary={summary_line}"));
            }
        }
    }

    let total: f64 = canonical_probabilities.values().copied().sum();
    if total > f64::EPSILON {
        for probability in canonical_probabilities.values_mut() {
            *probability = (*probability / total).clamp(0.0, 1.0);
        }
    }
    active_regime = canonical_probabilities
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
        .map(|(regime, _)| regime.clone());
    if let Some(active) = active_regime.as_ref() {
        let node_id = format!("{symbol}:belief_regime_node:{active}");
        let node_temporal_state = structural_prior_state
            .node_temporal_posteriors
            .get(&node_id);
        if let Some(duration_prior) = structural_prior_state.node_duration_priors.get(&node_id) {
            let active_probability = canonical_probabilities.get(active).copied().unwrap_or(0.5);
            let base_confidence = report
                .regime_posterior
                .confidence
                .unwrap_or_default()
                .max(active_probability);
            report.regime_posterior.confidence = Some(blend_node_posterior_with_duration_prior(
                base_confidence,
                Some(duration_prior),
                node_temporal_state,
            ));
            report.regime_posterior.evidence.push(format!(
                "duration_persistence_prior={:.3} observations={} streaks={} weighted_streak_mass={:.3} expected_dwell={:.3} break_hazard={:.3} sticky_self_transition={:.3} duration_outcome_support={:.3} duration_temporal_posterior_support={:.3}",
                duration_prior.persistence_prior,
                node_temporal_state
                    .map(|state| state.observations)
                    .unwrap_or(duration_prior.observations),
                node_temporal_state
                    .map(|state| state.streak_count)
                    .unwrap_or(duration_prior.streak_count),
                node_temporal_state
                    .map(|state| state.weighted_streak_mass)
                    .unwrap_or(duration_prior.weighted_streak_mass),
                node_temporal_state
                    .map(|state| state.expected_dwell_steps)
                    .unwrap_or(duration_prior.expected_dwell_steps),
                node_temporal_state
                    .map(|state| state.break_hazard)
                    .unwrap_or(duration_prior.break_hazard),
                node_temporal_state
                    .map(|state| state.sticky_self_transition_strength)
                    .unwrap_or(duration_prior.sticky_self_transition_strength),
                node_temporal_state
                    .map(|state| state.duration_outcome_support)
                    .unwrap_or(duration_prior.duration_outcome_support),
                node_temporal_state
                    .map(|state| state.temporal_posterior_support)
                    .unwrap_or(duration_prior.temporal_posterior_support)
            ));
            if let Some(blend_weight) =
                node_temporal_state.map(|state| state.posterior_blend_weight)
            {
                report
                    .regime_posterior
                    .evidence
                    .push(format!("duration_posterior_blend_weight={blend_weight:.3}"));
            }
            if let Some(summary_line) = node_temporal_state.map(|state| state.summary_line.clone())
            {
                report
                    .regime_posterior
                    .evidence
                    .push(format!("node_temporal_summary={summary_line}"));
            }
        }
    }
    report.regime_posterior.active_regime = active_regime.clone();
    report.regime_posterior.probabilities = canonical_probabilities.clone();
    report.gate_decision = gate_decision_from_regime_posterior(&report.regime_posterior);
    report.strategy_recommendation.direction = if active_regime.as_deref() == Some("trend") {
        "bull".to_string()
    } else {
        "neutral".to_string()
    };
    report.strategy_recommendation.market_family = report.regime_posterior.market_family.clone();
    report.strategy_recommendation.market_behavior_profile =
        report.regime_posterior.market_behavior_profile.clone();
    report.strategy_recommendation.selected_market_subgraph =
        Some(report.gate_decision.selected_subgraph.clone());
    report.selected_market_subgraph = Some(report.gate_decision.selected_subgraph.clone());
    report
        .strategy_recommendation
        .rationale
        .push("regime_posterior_adjusted_by_structural_priors".to_string());
    if let Some(summary) = report.temporal_summary.as_mut() {
        if let Some(active_regime) = active_regime.as_ref() {
            summary.dominant_regime = active_regime.clone();
        }
    }
    synchronize_market_regime_belief_snapshot(
        report,
        active_regime.as_deref(),
        &canonical_probabilities,
    );
}

fn canonical_structural_regime_label(label: &str) -> Option<String> {
    let normalized = label.trim().to_ascii_lowercase();
    let canonical = match normalized.as_str() {
        "trend" | "bull" | "bear" | "trend_impulse" | "trend_decay" => "trend",
        "range" | "range_calm" | "range_choppy" => "range",
        "stress" => "stress",
        "transition" => "transition",
        _ => return None,
    };
    Some(canonical.to_string())
}

fn structural_branch_label_for_regime(regime: &str) -> &'static str {
    match regime {
        "trend" => "trend_follow_through",
        "transition" => "transition_confirmation",
        "range" => "range_mean_reversion",
        "stress" => "stress_de_risk",
        _ => "execute_recommended_path",
    }
}

fn latest_structural_branch_id_for_symbol(
    events: &[StructuralPriorEvent],
    symbol: &str,
) -> Option<String> {
    events
        .iter()
        .filter(|event| event.symbol == symbol)
        .max_by(|left, right| {
            left.recommended_at
                .cmp(&right.recommended_at)
                .then_with(|| left.recommendation_id.cmp(&right.recommendation_id))
        })
        .map(|event| event.branch_id.clone())
}

fn synchronize_market_regime_belief_snapshot(
    report: &mut BeliefReportPacket,
    active_regime: Option<&str>,
    canonical_probabilities: &BTreeMap<String, f64>,
) {
    let Some(snapshot) = report
        .belief_posteriors
        .iter_mut()
        .find(|item| item.node_id == "market_regime")
    else {
        return;
    };
    let top_state = active_regime
        .map(str::to_string)
        .or_else(|| {
            canonical_probabilities
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
                .map(|(regime, _)| regime.clone())
        })
        .unwrap_or_else(|| "state_unavailable".to_string());
    let top_probability = canonical_probabilities
        .get(&top_state)
        .copied()
        .unwrap_or_default();
    let entropy = canonical_probabilities
        .values()
        .filter(|value| **value > 0.0)
        .map(|value| -value * value.ln())
        .sum();
    snapshot.top_state = top_state;
    snapshot.top_probability = top_probability;
    snapshot.entropy = entropy;
    snapshot.probabilities = canonical_probabilities.clone();
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
    let pythagorean_overstretch = frame_physics_trace.get("pythagorean_overstretch").copied();
    let ou_target = frame_physics_trace
        .get("ou_mean_reversion_target_bps")
        .copied();
    let ou_pullback_bps = frame_physics_trace.get("ou_expected_pullback_bps").copied();
    let ising_phase_risk = frame_physics_trace
        .get("ising_phase_transition_risk")
        .copied();
    let ising_herding = frame_physics_trace.get("ising_herding_bias").copied();

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
    if let Some(overstretch) = pythagorean_overstretch {
        if overstretch >= 0.5 {
            notes.push(format!(
                "pythagorean overstretch is elevated ({overstretch:.2}), so execution extension risk is rising"
            ));
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
    if let (Some(phase_risk), Some(herding)) = (ising_phase_risk, ising_herding) {
        notes.push(format!(
            "ising herding_bias={herding:.2} with phase_transition_risk={phase_risk:.2}"
        ));
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

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
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
