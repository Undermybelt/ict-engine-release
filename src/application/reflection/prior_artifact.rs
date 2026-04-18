use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct PriorArtifact {
    pub symbol: String,
    pub timestamp: String,
    pub objective: String,
    pub expected_regime: String,
    pub expected_direction: String,
    pub expected_key_evidence: Vec<String>,
    pub invalidation_conditions: Vec<String>,
    pub freshness_expectation: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PriorArtifactInput {
    pub symbol: String,
    pub timestamp: String,
    pub objective: String,
    pub expected_regime: String,
    pub expected_direction: String,
    pub expected_key_evidence: Vec<String>,
    pub invalidation_conditions: Vec<String>,
    pub freshness_expectation: String,
    pub notes: Vec<String>,
}

pub fn build_prior_artifact(input: PriorArtifactInput) -> PriorArtifact {
    PriorArtifact {
        symbol: input.symbol,
        timestamp: input.timestamp,
        objective: input.objective,
        expected_regime: input.expected_regime,
        expected_direction: input.expected_direction,
        expected_key_evidence: input.expected_key_evidence,
        invalidation_conditions: input.invalidation_conditions,
        freshness_expectation: input.freshness_expectation,
        notes: input.notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prior_builder_keeps_symbol_and_objective() {
        let artifact = build_prior_artifact(PriorArtifactInput {
            symbol: "NQ".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            objective: "expansion_manipulation".to_string(),
            expected_regime: "bull".to_string(),
            expected_direction: "long".to_string(),
            expected_key_evidence: vec!["e1".to_string()],
            invalidation_conditions: vec!["i1".to_string()],
            freshness_expectation: "fresh".to_string(),
            notes: vec!["n1".to_string()],
        });
        assert_eq!(artifact.symbol, "NQ");
        assert_eq!(artifact.objective, "expansion_manipulation");
    }
}
