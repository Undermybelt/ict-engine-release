use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SharedBeliefSkeleton {
    pub canonical_output_nodes: Vec<String>,
    pub evidence_nodes: Vec<String>,
    pub report_schema_version: String,
}

impl SharedBeliefSkeleton {
    pub fn trading_core() -> Self {
        Self {
            canonical_output_nodes: vec![
                "market_regime".to_string(),
                "liquidity_context".to_string(),
                "entry_quality".to_string(),
                "trade_outcome".to_string(),
                "risk_posture".to_string(),
            ],
            evidence_nodes: vec![
                "factor_alignment".to_string(),
                "factor_uncertainty".to_string(),
                "multi_timeframe_resonance".to_string(),
                "timed_pda_summary".to_string(),
            ],
            report_schema_version: "belief-packet-v1".to_string(),
        }
    }
}
