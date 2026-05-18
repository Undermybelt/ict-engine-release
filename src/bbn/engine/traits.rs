use crate::domain::belief::{BeliefEvidencePacket, BeliefNodePosteriorSnapshot, CredibleInterval};
use crate::domain::regime::RegimePosterior;

#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub packet: BeliefEvidencePacket,
}

pub trait BeliefInferenceEngine {
    fn name(&self) -> &'static str;
    fn infer_regime(&self, request: &InferenceRequest) -> anyhow::Result<RegimePosterior>;
    fn infer_beliefs(
        &self,
        request: &InferenceRequest,
    ) -> anyhow::Result<Vec<BeliefNodePosteriorSnapshot>>;
    fn credible_intervals(
        &self,
        request: &InferenceRequest,
    ) -> anyhow::Result<Vec<CredibleInterval>>;
    fn supports_temporal(&self) -> bool {
        false
    }
    fn supports_samples(&self) -> bool {
        false
    }
}
