use anyhow::Result;

use crate::domain::belief::{BeliefNodePosteriorSnapshot, CredibleInterval};
use crate::domain::regime::RegimePosterior;

use super::{
    infer_with_loopy_adapter, loopy_adapter_ready, BeliefInferenceEngine, ExactEngine,
    InferenceRequest,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct LoopyEngine;

impl BeliefInferenceEngine for LoopyEngine {
    fn name(&self) -> &'static str {
        if loopy_adapter_ready() {
            "loopy"
        } else {
            "loopy-stub"
        }
    }

    fn infer_regime(&self, request: &InferenceRequest) -> Result<RegimePosterior> {
        ExactEngine.infer_regime(request)
    }

    fn infer_beliefs(
        &self,
        request: &InferenceRequest,
    ) -> Result<Vec<BeliefNodePosteriorSnapshot>> {
        if loopy_adapter_ready() {
            let market_behavior_profile = request
                .packet
                .market_evidence
                .iter()
                .find_map(|line| line.strip_prefix("market_behavior_profile="));
            infer_with_loopy_adapter(
                &request.packet.evidence_assignments,
                market_behavior_profile,
            )
        } else {
            ExactEngine.infer_beliefs(request)
        }
    }

    fn credible_intervals(&self, request: &InferenceRequest) -> Result<Vec<CredibleInterval>> {
        let beliefs = self.infer_beliefs(request)?;
        Ok(beliefs
            .into_iter()
            .map(|belief| CredibleInterval {
                node_id: belief.node_id,
                state: belief.top_state,
                lower: (belief.top_probability - 0.18).clamp(0.0, 1.0),
                median: belief.top_probability,
                upper: (belief.top_probability + 0.18).clamp(0.0, 1.0),
                method: "loopy-belief-propagation".to_string(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::belief::BeliefEvidencePacket;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn loopy_and_exact_engines_expose_same_node_ids_with_market_behavior_profile() {
        let mut packet = BeliefEvidencePacket {
            evidence_assignments: BTreeMap::from([
                ("market_regime".to_string(), "range".to_string()),
                ("liquidity_context".to_string(), "neutral".to_string()),
                ("entry_quality".to_string(), "high".to_string()),
            ]),
            ..BeliefEvidencePacket::default()
        };
        packet
            .market_evidence
            .push("market_behavior_profile=index_beta_regime_sensitive".to_string());
        let request = InferenceRequest { packet };

        let exact_ids = ExactEngine
            .infer_beliefs(&request)
            .expect("exact beliefs")
            .into_iter()
            .map(|belief| belief.node_id)
            .collect::<BTreeSet<_>>();
        let loopy_ids = LoopyEngine
            .infer_beliefs(&request)
            .expect("loopy beliefs")
            .into_iter()
            .map(|belief| belief.node_id)
            .collect::<BTreeSet<_>>();

        assert_eq!(exact_ids, loopy_ids);
    }
}
