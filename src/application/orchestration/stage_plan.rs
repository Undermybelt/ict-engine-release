use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrchestrationStage {
    Analysis,
    Qualification,
    ExecutionPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StagePlan {
    pub stages: Vec<OrchestrationStage>,
}

impl StagePlan {
    pub fn analyze_risk_execution() -> Self {
        Self {
            stages: vec![
                OrchestrationStage::Analysis,
                OrchestrationStage::Qualification,
                OrchestrationStage::ExecutionPlan,
            ],
        }
    }
}
