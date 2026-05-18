use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct PostmortemArtifact {
    pub symbol: String,
    pub timestamp: String,
    pub expected_outcome: String,
    pub realized_outcome: String,
    pub deviations: Vec<String>,
    pub evidence_drift: Vec<String>,
    pub what_worked: Vec<String>,
    pub what_failed: Vec<String>,
    pub next_candidates: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PostmortemArtifactInput {
    pub symbol: String,
    pub timestamp: String,
    pub expected_outcome: String,
    pub realized_outcome: String,
    pub deviations: Vec<String>,
    pub evidence_drift: Vec<String>,
    pub what_worked: Vec<String>,
    pub what_failed: Vec<String>,
    pub next_candidates: Vec<String>,
}

pub fn build_postmortem_artifact(input: PostmortemArtifactInput) -> PostmortemArtifact {
    PostmortemArtifact {
        symbol: input.symbol,
        timestamp: input.timestamp,
        expected_outcome: input.expected_outcome,
        realized_outcome: input.realized_outcome,
        deviations: input.deviations,
        evidence_drift: input.evidence_drift,
        what_worked: input.what_worked,
        what_failed: input.what_failed,
        next_candidates: input.next_candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postmortem_builder_keeps_outcomes() {
        let artifact = build_postmortem_artifact(PostmortemArtifactInput {
            symbol: "NQ".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            expected_outcome: "win".to_string(),
            realized_outcome: "loss".to_string(),
            deviations: vec!["d1".to_string()],
            evidence_drift: vec!["e1".to_string()],
            what_worked: vec!["w1".to_string()],
            what_failed: vec!["f1".to_string()],
            next_candidates: vec!["n1".to_string()],
        });
        assert_eq!(artifact.expected_outcome, "win");
        assert_eq!(artifact.realized_outcome, "loss");
    }
}
