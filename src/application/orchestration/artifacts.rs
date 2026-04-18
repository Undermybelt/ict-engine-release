use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyDecisionArtifact {
    pub policy_version: String,
    pub action: String,
    pub qualification: String,
    pub recommended_command: String,
    pub confidence_band: String,
    pub leaf_id: String,
    pub split_trace: Vec<String>,
    pub invalidation_triggers: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalysisArtifact {
    pub stage: String,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub decision_hint: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualificationArtifact {
    pub stage: String,
    pub gating_status: String,
    pub selected_entry_quality: String,
    pub evidence_quality_score: f64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PosteriorAuditArtifact {
    pub posterior_version: String,
    pub fingerprint: String,
    pub comparable: bool,
    pub comparison_class: String,
    pub normalization_status: String,
    pub active_regime: String,
    pub confidence: Option<f64>,
    pub probabilities: BTreeMap<String, f64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnsembleHardBlockArtifact {
    pub active: bool,
    pub stage: Option<String>,
    pub status: Option<String>,
    pub reason: Option<String>,
    pub evidence: Vec<String>,
    pub command: Option<String>,
    pub human_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnsembleVoteArtifact {
    pub ensemble_version: String,
    pub posterior: PosteriorAuditArtifact,
    pub final_action: String,
    pub recommended_command: String,
    pub human_next_triage: String,
    #[serde(default)]
    pub hard_block: EnsembleHardBlockArtifact,
    pub confidence: f64,
    pub consensus_strength: f64,
    pub disagreement_flags: Vec<String>,
    pub executor_summaries: Vec<String>,
    pub split_explanations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnsembleDecision {
    pub final_action: String,
    pub recommended_command: String,
    pub human_next_triage: String,
    pub hard_block: EnsembleHardBlockArtifact,
    pub confidence: f64,
    pub consensus_strength: f64,
    pub disagreement_flags: Vec<String>,
    pub executor_summaries: Vec<String>,
    pub split_explanations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionPlanArtifact {
    pub stage: String,
    pub plan_status: String,
    pub selected_direction: String,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensemble_vote_artifact_deserializes_without_hard_block() {
        let artifact: EnsembleVoteArtifact = serde_json::from_value(serde_json::json!({
            "ensemble_version": "ensemble-audit-v2-weighted",
            "posterior": {
                "posterior_version": "v1",
                "fingerprint": "fp-1",
                "comparable": true,
                "comparison_class": "baseline",
                "normalization_status": "normalized",
                "active_regime": "research",
                "confidence": 0.5,
                "probabilities": {"research": 1.0},
                "evidence": []
            },
            "final_action": "observe",
            "recommended_command": "ict-engine workflow-status --symbol NQ --phase human-next",
            "human_next_triage": "hard_blocked=false ensemble_action=observe",
            "confidence": 0.5,
            "consensus_strength": 0.5,
            "disagreement_flags": [],
            "executor_summaries": [],
            "split_explanations": []
        }))
        .expect("deserialize ensemble vote artifact");

        assert!(!artifact.hard_block.active);
        assert!(artifact.hard_block.reason.is_none());
    }
}
