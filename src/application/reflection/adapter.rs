use serde::Serialize;

use super::{
    build_postmortem_artifact, build_prior_artifact, PostmortemArtifact, PostmortemArtifactInput,
    PriorArtifact, PriorArtifactInput,
};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ReflectionBundle {
    pub prior: PriorArtifact,
    pub postmortem: PostmortemArtifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ensemble_vote_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ensemble_vote_artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ensemble_disagreement_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_setup_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_setup_guardrail: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ReflectionBundleInput {
    pub symbol: String,
    pub timestamp: String,
    pub objective: String,
    pub expected_regime: String,
    pub expected_direction: String,
    pub realized_outcome: String,
    pub evidence: Vec<String>,
    pub next_candidates: Vec<String>,
}

pub fn build_reflection_bundle(input: ReflectionBundleInput) -> ReflectionBundle {
    let symbol = input.symbol;
    let timestamp = input.timestamp;
    let objective = input.objective;
    let expected_regime = input.expected_regime;
    let expected_direction = input.expected_direction;
    let realized_outcome = input.realized_outcome;
    let evidence = input.evidence;
    let next_candidates = input.next_candidates;

    ReflectionBundle {
        prior: build_prior_artifact(PriorArtifactInput {
            symbol: symbol.clone(),
            timestamp: timestamp.clone(),
            objective,
            expected_regime,
            expected_direction,
            expected_key_evidence: evidence.clone(),
            invalidation_conditions: next_candidates.clone(),
            freshness_expectation: "fresh_required".to_string(),
            notes: next_candidates.clone(),
        }),
        postmortem: build_postmortem_artifact(PostmortemArtifactInput {
            symbol,
            timestamp,
            expected_outcome: "unknown_expected_outcome".to_string(),
            realized_outcome,
            deviations: next_candidates.clone(),
            evidence_drift: evidence.clone(),
            what_worked: evidence,
            what_failed: next_candidates.clone(),
            next_candidates,
        }),
        ensemble_vote_summary: None,
        ensemble_vote_artifact_id: None,
        ensemble_disagreement_summary: None,
        execution_setup_summary: None,
        execution_setup_guardrail: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflection_bundle_contains_prior_and_postmortem() {
        let bundle = build_reflection_bundle(ReflectionBundleInput {
            symbol: "NQ".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            objective: "generic".to_string(),
            expected_regime: "bull".to_string(),
            expected_direction: "long".to_string(),
            realized_outcome: "win".to_string(),
            evidence: vec!["e1".to_string()],
            next_candidates: vec!["n1".to_string()],
        });
        assert_eq!(bundle.prior.symbol, "NQ");
        assert_eq!(bundle.postmortem.realized_outcome, "win");
    }
}
