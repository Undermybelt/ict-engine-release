use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct FactorAttributionItem {
    pub factor_name: String,
    pub contribution: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BeliefAttributionItem {
    pub node: String,
    pub contribution: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DecisionAttribution {
    pub summary: String,
    pub factor_items: Vec<FactorAttributionItem>,
    pub belief_items: Vec<BeliefAttributionItem>,
}

pub fn build_decision_attribution(
    summary: impl Into<String>,
    factor_items: &[FactorAttributionItem],
    belief_items: &[BeliefAttributionItem],
) -> DecisionAttribution {
    DecisionAttribution {
        summary: summary.into(),
        factor_items: factor_items.to_vec(),
        belief_items: belief_items.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribution_builder_keeps_items() {
        let report = build_decision_attribution(
            "summary",
            &[FactorAttributionItem {
                factor_name: "f".to_string(),
                contribution: 0.7,
                explanation: "good".to_string(),
            }],
            &[BeliefAttributionItem {
                node: "n".to_string(),
                contribution: 0.4,
                explanation: "ok".to_string(),
            }],
        );
        assert_eq!(report.factor_items.len(), 1);
        assert_eq!(report.belief_items.len(), 1);
    }
}
