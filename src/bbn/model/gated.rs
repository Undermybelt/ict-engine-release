use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{RegimeSpecificSubgraph, SharedBeliefSkeleton};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatedBeliefModel {
    pub skeleton: SharedBeliefSkeleton,
    pub subgraphs: BTreeMap<String, RegimeSpecificSubgraph>,
}

impl GatedBeliefModel {
    pub fn trading_core_default() -> Self {
        let skeleton = SharedBeliefSkeleton::trading_core();
        let subgraphs = BTreeMap::from([
            (
                "trend".to_string(),
                RegimeSpecificSubgraph {
                    regime_key: "trend".to_string(),
                    node_ids: skeleton.canonical_output_nodes.clone(),
                    edge_descriptions: vec![
                        "market_regime->entry_quality".to_string(),
                        "factor_alignment->entry_quality".to_string(),
                        "entry_quality->trade_outcome".to_string(),
                        "liquidity_context->risk_posture".to_string(),
                    ],
                    cpt_surface_id: "trend_surface_v1".to_string(),
                },
            ),
            (
                "range".to_string(),
                RegimeSpecificSubgraph {
                    regime_key: "range".to_string(),
                    node_ids: skeleton.canonical_output_nodes.clone(),
                    edge_descriptions: vec![
                        "market_regime->risk_posture".to_string(),
                        "multi_timeframe_resonance->entry_quality".to_string(),
                        "entry_quality->trade_outcome".to_string(),
                    ],
                    cpt_surface_id: "range_surface_v1".to_string(),
                },
            ),
            (
                "stress".to_string(),
                RegimeSpecificSubgraph {
                    regime_key: "stress".to_string(),
                    node_ids: skeleton.canonical_output_nodes.clone(),
                    edge_descriptions: vec![
                        "liquidity_context->risk_posture".to_string(),
                        "factor_uncertainty->risk_posture".to_string(),
                        "risk_posture->trade_outcome".to_string(),
                    ],
                    cpt_surface_id: "stress_surface_v1".to_string(),
                },
            ),
            (
                "transition".to_string(),
                RegimeSpecificSubgraph {
                    regime_key: "transition".to_string(),
                    node_ids: skeleton.canonical_output_nodes.clone(),
                    edge_descriptions: vec![
                        "market_regime->risk_posture".to_string(),
                        "multi_timeframe_resonance->risk_posture".to_string(),
                        "risk_posture->entry_quality".to_string(),
                        "entry_quality->trade_outcome".to_string(),
                    ],
                    cpt_surface_id: "transition_surface_v1".to_string(),
                },
            ),
        ]);

        Self {
            skeleton,
            subgraphs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_gated_trading_core_model() {
        let model = GatedBeliefModel::trading_core_default();
        assert_eq!(model.skeleton.canonical_output_nodes.len(), 5);
        assert!(model.subgraphs.contains_key("trend"));
        assert!(model.subgraphs.contains_key("stress"));
    }
}
