use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyRecommendation {
    pub direction: String,
    pub aggression_level: String,
    pub sizing_multiplier: f64,
    pub market_family: Option<String>,
    pub market_behavior_profile: Option<String>,
    pub selected_market_subgraph: Option<String>,
    pub invalidate_if: Vec<String>,
    pub rationale: Vec<String>,
}
