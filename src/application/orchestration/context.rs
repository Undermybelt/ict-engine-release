use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineState {
    pub symbol: String,
    pub market: Option<String>,
    pub feature_flag: String,
    pub completed_stages: Vec<String>,
}

impl PipelineState {
    pub fn new(
        symbol: impl Into<String>,
        market: Option<&str>,
        feature_flag: impl Into<String>,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            market: market.map(str::to_string),
            feature_flag: feature_flag.into(),
            completed_stages: Vec::new(),
        }
    }

    pub fn mark_stage_completed(&mut self, stage: impl Into<String>) {
        self.completed_stages.push(stage.into());
    }
}
