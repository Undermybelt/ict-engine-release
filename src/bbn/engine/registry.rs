use anyhow::Result;

use crate::bbn::temporal::bootstrap_particle_summary;
use crate::domain::belief::{
    BeliefEvidencePacket, BeliefReportPacket, EngineTrace, ShadowComparisonSummary,
};

use super::{BeliefInferenceEngine, ExactEngine, InferenceRequest, LoopyEngine, SamplingEngine};
use crate::application::belief::{
    build_regime_disagreement_summary, objective_market_credibility_shrink,
};
use crate::bbn::adapters::{
    gate_decision_from_regime_posterior, jump_model_summary_from_belief_packet,
    strategy_recommendation_from_pre_bayes_filter,
};
use crate::state::PreBayesEvidenceFilter;

pub struct InferenceEngineRegistry {
    primary: Box<dyn BeliefInferenceEngine + Send + Sync>,
    shadow: Vec<Box<dyn BeliefInferenceEngine + Send + Sync>>,
}

impl Default for InferenceEngineRegistry {
    fn default() -> Self {
        let primary_name = std::env::var("ICT_ENGINE_BELIEF_PRIMARY")
            .unwrap_or_else(|_| "exact".to_string())
            .to_ascii_lowercase();
        let primary: Box<dyn BeliefInferenceEngine + Send + Sync> = match primary_name.as_str() {
            "loopy" => Box::new(LoopyEngine),
            "sampling" => Box::new(SamplingEngine),
            _ => Box::new(ExactEngine),
        };
        Self {
            primary,
            shadow: vec![Box::new(LoopyEngine), Box::new(SamplingEngine)],
        }
    }
}

impl InferenceEngineRegistry {
    pub fn build_report(&self, packet: BeliefEvidencePacket) -> Result<BeliefReportPacket> {
        let request = InferenceRequest { packet };
        let regime_posterior =
            crate::bbn::adapters::regime_posterior_from_belief_packet(&request.packet);
        let jump_model = regime_posterior
            .jump_model
            .clone()
            .unwrap_or_else(|| jump_model_summary_from_belief_packet(&request.packet));
        let gate_decision = gate_decision_from_regime_posterior(&regime_posterior);
        let credibility_score = 1.0
            - request
                .packet
                .regime_features
                .stress_score
                .unwrap_or(0.5)
                .clamp(0.0, 1.0);
        let shrink = objective_market_credibility_shrink(
            request.packet.logic_family.as_deref(),
            regime_posterior.market_family.as_deref(),
            credibility_score,
        );
        let jump_disagreement = build_regime_disagreement_summary(
            regime_posterior.active_regime.as_deref(),
            Some(&jump_model),
            Some(&shrink),
        );
        let belief_posteriors = self.primary.infer_beliefs(&request)?;
        let credible_intervals = self.primary.credible_intervals(&request)?;

        let selected_direction = if regime_posterior.active_regime.as_deref() == Some("trend") {
            "bull"
        } else {
            "neutral"
        };
        let pseudo_filter = PreBayesEvidenceFilter {
            filtered_market_regime_label: request
                .packet
                .regime_features
                .market_regime_label
                .clone()
                .unwrap_or_else(|| "range".to_string()),
            filtered_liquidity_context_label: request
                .packet
                .regime_features
                .liquidity_regime_label
                .clone()
                .unwrap_or_else(|| "neutral".to_string()),
            filtered_multi_timeframe_resonance_label: request
                .packet
                .multi_timeframe_evidence
                .get("filtered_resonance_label")
                .cloned()
                .unwrap_or_else(|| "mixed".to_string()),
            evidence_quality_score: 1.0
                - request
                    .packet
                    .regime_features
                    .stress_score
                    .unwrap_or(0.5)
                    .clamp(0.0, 1.0),
            rationale: request.packet.factor_evidence.clone(),
            gating_status: if gate_decision.selected_regime == "stress" {
                "observe_only".to_string()
            } else {
                "pass_neutralized".to_string()
            },
            ..PreBayesEvidenceFilter::default()
        };
        let strategy_recommendation = strategy_recommendation_from_pre_bayes_filter(
            &pseudo_filter,
            selected_direction,
            0.55,
            regime_posterior.market_family.as_deref(),
            regime_posterior.market_behavior_profile.as_deref(),
            Some(gate_decision.selected_subgraph.as_str()),
        );

        let mut match_rate = std::collections::BTreeMap::new();
        let mut kl_divergence = std::collections::BTreeMap::new();
        let mut interval_overlap = std::collections::BTreeMap::new();
        let mut recommendation_drift = Vec::new();
        let mut shadow_names = Vec::new();
        for engine in &self.shadow {
            shadow_names.push(engine.name().to_string());
            let shadow_beliefs = engine.infer_beliefs(&request)?;
            let shadow_intervals = engine.credible_intervals(&request)?;
            let shadow_direction = shadow_beliefs
                .iter()
                .find(|item| item.node_id == "trade_outcome")
                .map(|item| item.top_state.clone())
                .unwrap_or_else(|| strategy_recommendation.direction.clone());
            for primary in &belief_posteriors {
                if let Some(shadow) = shadow_beliefs
                    .iter()
                    .find(|item| item.node_id == primary.node_id)
                {
                    match_rate.insert(
                        format!("{}:{}", engine.name(), primary.node_id),
                        if primary.top_state == shadow.top_state {
                            1.0
                        } else {
                            0.0
                        },
                    );
                    kl_divergence.insert(
                        format!("{}:{}", engine.name(), primary.node_id),
                        (primary.top_probability - shadow.top_probability).abs(),
                    );
                }
            }
            for primary in &credible_intervals {
                if let Some(shadow) = shadow_intervals
                    .iter()
                    .find(|item| item.node_id == primary.node_id)
                {
                    let overlap_low = primary.lower.max(shadow.lower);
                    let overlap_high = primary.upper.min(shadow.upper);
                    let overlap = if overlap_high > overlap_low {
                        overlap_high - overlap_low
                    } else {
                        0.0
                    };
                    interval_overlap
                        .insert(format!("{}:{}", engine.name(), primary.node_id), overlap);
                }
            }
            recommendation_drift.push(format!("{}:direction={}", engine.name(), shadow_direction));
        }

        let node_count = belief_posteriors.len();
        let shadow_engine_names = shadow_names.join(",");
        let shadow_status = if kl_divergence.values().any(|value| *value > 0.35) {
            "red".to_string()
        } else if kl_divergence.values().any(|value| *value > 0.15) {
            "yellow".to_string()
        } else {
            "green".to_string()
        };
        let temporal_summary = bootstrap_particle_summary(
            regime_posterior.active_regime.as_deref().unwrap_or("range"),
            regime_posterior.market_family.as_deref(),
            regime_posterior.market_behavior_profile.as_deref(),
        );

        let market_family = regime_posterior.market_family.clone();
        let market_behavior_profile = regime_posterior.market_behavior_profile.clone();
        let selected_market_subgraph = Some(gate_decision.selected_subgraph.clone());
        let belief_posteriors = belief_posteriors
            .into_iter()
            .map(|mut item| {
                if let Some(family) = market_family.as_deref() {
                    item.probabilities.insert(
                        "market_family_weight".to_string(),
                        match family {
                            "energy" => 0.96,
                            "metals" => 0.88,
                            "futures_index" => 0.75,
                            _ => 0.50,
                        },
                    );
                }
                item.probabilities.insert(
                    "market_jump_weight".to_string(),
                    jump_model.market_jump_weight,
                );
                item
            })
            .collect();

        Ok(BeliefReportPacket {
            regime_posterior,
            gate_decision,
            belief_posteriors,
            credible_intervals,
            strategy_recommendation,
            regime_companion: crate::domain::belief::RegimeCompanionPacket {
                jump_model: Some(jump_model.clone()),
                disagreement: Some(jump_disagreement.clone()),
                objective_market_credibility_shrink: Some(shrink.clone()),
            },
            market_family,
            market_behavior_profile,
            selected_market_subgraph,
            engine_trace: EngineTrace {
                primary_engine: self.primary.name().to_string(),
                shadow_engine: Some(shadow_engine_names.clone()),
                sample_count: Some(self.shadow.len()),
                notes: vec![
                    "registry_build_report_v3".to_string(),
                    format!(
                        "jump_model_state={} confidence={:.3} transition_risk={:.3}",
                        jump_model.active_state, jump_model.confidence, jump_model.transition_risk
                    ),
                    format!(
                        "jump_disagreement_score={:.3} gate_bias={}",
                        jump_disagreement.disagreement_score,
                        jump_disagreement.gate_bias
                    ),
                    format!(
                        "particle_count={} ess={:.2} dominant_regime={}",
                        temporal_summary.particle_count,
                        temporal_summary.effective_sample_size,
                        temporal_summary.dominant_regime
                    ),
                ],
            },
            temporal_summary: Some(temporal_summary.clone()),
            shadow_comparison: Some(ShadowComparisonSummary {
                status: shadow_status.clone(),
                summary_line: format!(
                    "primary={} shadow={} nodes={} status={} jump_model={} transition_risk={:.3} particle_count={} ess={:.2}",
                    self.primary.name(),
                    shadow_engine_names,
                    node_count,
                    shadow_status,
                    jump_model.active_state,
                    jump_model.transition_risk,
                    temporal_summary.particle_count,
                    temporal_summary.effective_sample_size
                ),
                top_state_match_rate: match_rate,
                kl_divergence,
                interval_overlap,
                recommendation_drift,
            }),
            conformal_uncertainty: Vec::new(),
            market_policy: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn registry_builds_belief_report_packet() {
        let registry = InferenceEngineRegistry::default();
        let report = registry
            .build_report(BeliefEvidencePacket {
                symbol: "NQ".to_string(),
                market: Some("NQ".to_string()),
                timestamp: None,
                entry_logic_id: None,
                logic_family: None,
                regime_features: crate::domain::regime::RegimeFeatures {
                    market_regime_label: Some("bull".to_string()),
                    volatility_regime_label: Some("low".to_string()),
                    liquidity_regime_label: Some("favorable".to_string()),
                    stress_score: Some(0.2),
                    transition_score: Some(0.1),
                    evidence: vec![],
                    segmentation_context: None,
                    structural_break_context: None,
                },
                market_evidence: vec![
                    "market_category=futures_index".to_string(),
                    "market_behavior_profile=index_beta_regime_sensitive".to_string(),
                ],
                factor_evidence: vec!["aligned".to_string()],
                timed_pda_summary: BTreeMap::new(),
                multi_timeframe_evidence: BTreeMap::from([(
                    "filtered_resonance_label".to_string(),
                    "aligned".to_string(),
                )]),
                evidence_assignments: BTreeMap::from([
                    ("market_regime".to_string(), "bull".to_string()),
                    ("liquidity_context".to_string(), "favorable".to_string()),
                    ("entry_quality".to_string(), "high".to_string()),
                ]),
                uses_soft_evidence: false,
                soft_market_regime_distribution: BTreeMap::new(),
                soft_liquidity_context_distribution: BTreeMap::new(),
                soft_factor_alignment_distribution: BTreeMap::new(),
                soft_factor_uncertainty_distribution: BTreeMap::new(),
                soft_multi_timeframe_resonance_distribution: BTreeMap::new(),
                microstructure_context: None,
                market_policy: None,
            })
            .unwrap();
        assert!(!report.engine_trace.primary_engine.is_empty());
        assert!(!report.belief_posteriors.is_empty());
        assert_eq!(
            report.gate_decision.market_family.as_deref(),
            Some("futures_index")
        );
        assert_eq!(
            report.gate_decision.selected_subgraph,
            "futures_index_trend_subgraph"
        );
        assert_eq!(report.market_family.as_deref(), Some("futures_index"));
        assert_eq!(
            report.selected_market_subgraph.as_deref(),
            Some("futures_index_trend_subgraph")
        );
        assert_eq!(
            report.strategy_recommendation.market_family.as_deref(),
            Some("futures_index")
        );
        assert!(
            report
                .temporal_summary
                .as_ref()
                .unwrap()
                .market_family
                .as_deref()
                == Some("futures_index")
        );
        let shadow = report.shadow_comparison.as_ref().unwrap();
        assert!(shadow.summary_line.contains("particle_count="));
        assert!(shadow
            .recommendation_drift
            .iter()
            .any(|line| line.starts_with("sampling-stub:direction=")));
        assert!(shadow
            .recommendation_drift
            .iter()
            .all(|line| !line.ends_with(":direction=bull")));
        assert_eq!(
            report
                .regime_companion
                .jump_model
                .as_ref()
                .map(|item| item.active_state.as_str()),
            Some("trend_persistent")
        );
        assert!(shadow.summary_line.contains("jump_model="));
        assert!(report
            .engine_trace
            .notes
            .iter()
            .any(|note| note.contains("jump_model_state=")));
    }
}
