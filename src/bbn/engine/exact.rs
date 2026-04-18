use std::collections::BTreeMap;

use anyhow::Result;

use crate::bbn::adapters::{
    belief_snapshot_from_distribution, regime_posterior_from_pre_bayes_filter,
};
use crate::domain::belief::{BeliefNodePosteriorSnapshot, CredibleInterval};
use crate::domain::regime::RegimePosterior;
use crate::state::PreBayesEvidenceFilter;

use super::{BeliefInferenceEngine, InferenceRequest};

#[derive(Debug, Default, Clone, Copy)]
pub struct ExactEngine;

impl ExactEngine {
    fn market_behavior_profile(request: &InferenceRequest) -> Option<&str> {
        request
            .packet
            .market_evidence
            .iter()
            .find_map(|line| line.strip_prefix("market_behavior_profile="))
    }

    fn filter_from_request(&self, request: &InferenceRequest) -> Result<PreBayesEvidenceFilter> {
        let packet = &request.packet;
        let filtered_market_regime_label = packet
            .regime_features
            .market_regime_label
            .clone()
            .unwrap_or_else(|| "range".to_string());
        let filtered_liquidity_context_label = packet
            .regime_features
            .liquidity_regime_label
            .clone()
            .unwrap_or_else(|| "neutral".to_string());
        let filtered_factor_uncertainty = packet
            .regime_features
            .volatility_regime_label
            .clone()
            .unwrap_or_else(|| "high".to_string());
        Ok(PreBayesEvidenceFilter {
            entry_logic_id: packet.entry_logic_id.clone(),
            logic_family: packet.logic_family.clone(),
            filtered_market_regime_label,
            filtered_liquidity_context_label,
            filtered_factor_alignment: packet
                .evidence_assignments
                .get("factor_alignment")
                .cloned()
                .unwrap_or_else(|| "mixed".to_string()),
            filtered_factor_uncertainty,
            filtered_multi_timeframe_resonance_label: packet
                .multi_timeframe_evidence
                .get("filtered_resonance_label")
                .cloned()
                .unwrap_or_else(|| "mixed".to_string()),
            evidence_quality_score: 1.0
                - packet
                    .regime_features
                    .stress_score
                    .unwrap_or(0.5)
                    .clamp(0.0, 1.0),
            evidence_assignments: packet.evidence_assignments.clone(),
            rationale: packet.factor_evidence.clone(),
            ..PreBayesEvidenceFilter::default()
        })
    }
}

impl BeliefInferenceEngine for ExactEngine {
    fn name(&self) -> &'static str {
        "exact"
    }

    fn infer_regime(&self, request: &InferenceRequest) -> Result<RegimePosterior> {
        let filter = self.filter_from_request(request)?;
        Ok(regime_posterior_from_pre_bayes_filter(&filter))
    }

    fn infer_beliefs(
        &self,
        request: &InferenceRequest,
    ) -> Result<Vec<BeliefNodePosteriorSnapshot>> {
        let assignments = &request.packet.evidence_assignments;
        let market_behavior_profile = Self::market_behavior_profile(request);
        let mut out = Vec::new();

        let market_regime = BTreeMap::from([(
            assignments
                .get("market_regime")
                .cloned()
                .unwrap_or_else(|| "range".to_string()),
            1.0,
        )]);
        out.push(belief_snapshot_from_distribution(
            "market_regime",
            &market_regime,
        ));

        let liquidity_context = BTreeMap::from([(
            assignments
                .get("liquidity_context")
                .cloned()
                .unwrap_or_else(|| "neutral".to_string()),
            1.0,
        )]);
        out.push(belief_snapshot_from_distribution(
            "liquidity_context",
            &liquidity_context,
        ));

        let entry_quality = BTreeMap::from([
            (
                assignments
                    .get("entry_quality")
                    .cloned()
                    .unwrap_or_else(|| "medium".to_string()),
                0.7,
            ),
            ("medium".to_string(), 0.2),
            ("low".to_string(), 0.1),
        ]);
        out.push(belief_snapshot_from_distribution(
            "entry_quality",
            &entry_quality,
        ));

        let trade_outcome = BTreeMap::from([
            ("win".to_string(), 0.55),
            ("scratch".to_string(), 0.15),
            ("loss".to_string(), 0.30),
        ]);
        out.push(belief_snapshot_from_distribution(
            "trade_outcome",
            &trade_outcome,
        ));

        let risk_posture = BTreeMap::from([
            ("conservative".to_string(), 0.25),
            ("balanced".to_string(), 0.50),
            ("aggressive".to_string(), 0.25),
        ]);
        out.push(belief_snapshot_from_distribution(
            "risk_posture",
            &risk_posture,
        ));

        if let Some(profile) = market_behavior_profile {
            let market_profile = BTreeMap::from([(profile.to_string(), 1.0)]);
            out.push(belief_snapshot_from_distribution(
                "market_behavior_profile",
                &market_profile,
            ));
        }

        Ok(out)
    }

    fn credible_intervals(&self, request: &InferenceRequest) -> Result<Vec<CredibleInterval>> {
        let beliefs = self.infer_beliefs(request)?;
        Ok(beliefs
            .into_iter()
            .map(|snapshot| CredibleInterval {
                node_id: snapshot.node_id,
                state: snapshot.top_state,
                lower: (snapshot.top_probability - 0.15).clamp(0.0, 1.0),
                median: snapshot.top_probability,
                upper: (snapshot.top_probability + 0.15).clamp(0.0, 1.0),
                method: "exact-surrogate-band".to_string(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::belief::BeliefEvidencePacket;

    #[test]
    fn exact_engine_beliefs_do_not_change_with_market_category_string() {
        let packet = BeliefEvidencePacket {
            evidence_assignments: BTreeMap::from([
                ("market_regime".to_string(), "range".to_string()),
                ("liquidity_context".to_string(), "neutral".to_string()),
                ("entry_quality".to_string(), "high".to_string()),
            ]),
            ..BeliefEvidencePacket::default()
        };

        let mut energy_packet = packet.clone();
        energy_packet
            .market_evidence
            .push("market_category=energy".to_string());
        let mut metals_packet = packet.clone();
        metals_packet
            .market_evidence
            .push("market_category=metals".to_string());

        let energy_beliefs = ExactEngine
            .infer_beliefs(&InferenceRequest {
                packet: energy_packet,
            })
            .unwrap();
        let metals_beliefs = ExactEngine
            .infer_beliefs(&InferenceRequest {
                packet: metals_packet,
            })
            .unwrap();

        let energy_entry = energy_beliefs
            .iter()
            .find(|belief| belief.node_id == "entry_quality")
            .unwrap();
        let metals_entry = metals_beliefs
            .iter()
            .find(|belief| belief.node_id == "entry_quality")
            .unwrap();
        assert_eq!(energy_entry.probabilities, metals_entry.probabilities);
    }
}
