use serde::{Deserialize, Serialize};

use super::{OrchestrationStage, PipelineState, StagePlan};

pub const STAGED_ORCHESTRATION_FLAG: &str = "ict_engine_staged_orchestration";

pub trait StageNode {
    fn stage(&self) -> OrchestrationStage;
    fn run(&self, state: &mut PipelineState);
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageRunTrace {
    pub feature_flag_enabled: bool,
    pub executed_stages: Vec<String>,
}

pub fn staged_orchestration_enabled() -> bool {
    std::env::var("ICT_ENGINE_STAGED_ORCHESTRATION")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub fn run_stage_plan(plan: &StagePlan, state: &mut PipelineState) -> StageRunTrace {
    let mut trace = StageRunTrace {
        feature_flag_enabled: staged_orchestration_enabled(),
        executed_stages: Vec::new(),
    };

    if !trace.feature_flag_enabled {
        return trace;
    }

    for stage in &plan.stages {
        let label = match stage {
            OrchestrationStage::Analysis => "analysis",
            OrchestrationStage::Qualification => "qualification",
            OrchestrationStage::ExecutionPlan => "execution_plan",
        };
        state.mark_stage_completed(label);
        trace.executed_stages.push(label.to_string());
    }

    trace
}
