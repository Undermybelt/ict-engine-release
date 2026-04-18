use serde::{Deserialize, Serialize};

use super::{
    AnalysisArtifact, ExecutionPlanArtifact, PipelineState, PolicyDecisionArtifact,
    QualificationArtifact, StageRunTrace,
};

pub trait FinalSurfaceAdapter<T> {
    fn adapt(&self, state: &PipelineState, trace: &StageRunTrace) -> T;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinalOutputAdapter;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StagedArtifacts {
    pub analysis: AnalysisArtifact,
    pub qualification: QualificationArtifact,
    pub execution_plan: ExecutionPlanArtifact,
    pub policy_decision: PolicyDecisionArtifact,
}

impl FinalSurfaceAdapter<ExecutionPlanArtifact> for FinalOutputAdapter {
    fn adapt(&self, state: &PipelineState, trace: &StageRunTrace) -> ExecutionPlanArtifact {
        ExecutionPlanArtifact {
            stage: "final_output_adapter".to_string(),
            plan_status: if trace.feature_flag_enabled {
                "staged_surface_ready".to_string()
            } else {
                "legacy_surface_passthrough".to_string()
            },
            selected_direction: "direction_unavailable".to_string(),
            summary: format!(
                "symbol={} market={} executed_stages={}",
                state.symbol,
                state.market.as_deref().unwrap_or("market_unavailable"),
                trace.executed_stages.join(",")
            ),
        }
    }
}
