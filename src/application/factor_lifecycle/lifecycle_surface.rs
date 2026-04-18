use serde::Serialize;

use crate::state::{
    FactorMutationEvaluation, FactorMutationSpec, PromotionDecision, RollbackRecommendation,
};

#[derive(Debug, Clone, Serialize, Default)]
pub struct FactorLifecycleView {
    pub mutation_id: String,
    pub promotion_status: String,
    pub rollback_status: String,
    pub accepted: bool,
    pub next_action: String,
    pub target_factors: Vec<String>,
}

pub fn build_factor_lifecycle_view(
    mutation_spec: Option<&FactorMutationSpec>,
    mutation_evaluation: Option<&FactorMutationEvaluation>,
    promotion_decision: &PromotionDecision,
    rollback_recommendation: &RollbackRecommendation,
) -> FactorLifecycleView {
    let mutation_id = mutation_spec
        .map(|spec| spec.mutation_id.clone())
        .or_else(|| mutation_evaluation.map(|item| item.mutation_id.clone()))
        .unwrap_or_else(|| "unscoped".to_string());
    let accepted = mutation_evaluation
        .map(|item| item.accepted)
        .unwrap_or(false);
    let next_action = if rollback_recommendation.should_rollback {
        "rollback".to_string()
    } else if promotion_decision.approved {
        "promote".to_string()
    } else if accepted {
        "iterate".to_string()
    } else {
        "observe".to_string()
    };
    let mut target_factors = promotion_decision.target_factors.clone();
    for factor in &rollback_recommendation.target_factors {
        if !target_factors.contains(factor) {
            target_factors.push(factor.clone());
        }
    }
    FactorLifecycleView {
        mutation_id,
        promotion_status: promotion_decision.status.clone(),
        rollback_status: if rollback_recommendation.should_rollback {
            rollback_recommendation.scope.clone()
        } else {
            "none".to_string()
        },
        accepted,
        next_action,
        target_factors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_prefers_rollback_when_needed() {
        let promotion = PromotionDecision {
            approved: true,
            status: "promote".to_string(),
            reason: "promotion_reason_unavailable".to_string(),
            target_factors: vec!["a".to_string()],
            target_families: vec![],
        };
        let rollback = RollbackRecommendation {
            should_rollback: true,
            scope: "targeted".to_string(),
            reason: "rollback_reason_unavailable".to_string(),
            target_factors: vec!["b".to_string()],
            target_families: vec![],
        };
        let view = build_factor_lifecycle_view(None, None, &promotion, &rollback);
        assert_eq!(view.next_action, "rollback");
        assert_eq!(promotion.reason, "promotion_reason_unavailable");
        assert_eq!(rollback.reason, "rollback_reason_unavailable");
        assert!(view.target_factors.contains(&"a".to_string()));
        assert!(view.target_factors.contains(&"b".to_string()));
    }
}
