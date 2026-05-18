use crate::application::decision_utils::ArtifactConsumedDecisionGate;
use crate::state::{PromotionDecision, RollbackRecommendation};

pub fn artifact_consumed_decision_gate(
    consumed_impact_summary: &crate::state::ArtifactConsumedImpactSummary,
) -> ArtifactConsumedDecisionGate {
    let (status, reason, target_kinds) =
        crate::application::artifacts::artifact_consumed_trend_signal(consumed_impact_summary);
    ArtifactConsumedDecisionGate {
        status,
        reason,
        target_kinds,
    }
}

pub fn link_artifact_decision_summary_to_decisions(
    artifact_summary: &crate::state::ArtifactDecisionSummary,
    promotion_decision: &mut PromotionDecision,
    rollback_recommendation: &mut RollbackRecommendation,
) {
    let artifact_reason = format!(
        "artifact_actionable_count={} artifact_latest_promotable={:?} artifact_rule_breaks={} artifact_consumed_trend_status={} artifact_consumed_targets={:?}",
        artifact_summary.actionable_artifact_count,
        artifact_summary.latest_promotable_artifact_id,
        artifact_summary.artifact_rule_break_count,
        artifact_summary.consumed_trend_status,
        artifact_summary.consumed_target_kinds
    );
    if !artifact_reason.is_empty() {
        promotion_decision.reason = format!("{}|{}", promotion_decision.reason, artifact_reason);
        rollback_recommendation.reason =
            format!("{}|{}", rollback_recommendation.reason, artifact_reason);
    }
    promotion_decision.reason = format!(
        "{}|artifact_promotion_strength={}",
        promotion_decision.reason, artifact_summary.promotion_strength
    );
    rollback_recommendation.reason = format!(
        "{}|artifact_rollback_strength={}",
        rollback_recommendation.reason, artifact_summary.rollback_strength
    );
    if !artifact_summary.consumed_trend_reason.is_empty() {
        promotion_decision.reason = format!(
            "{}|artifact_consumed_trend_reason={}",
            promotion_decision.reason, artifact_summary.consumed_trend_reason
        );
        rollback_recommendation.reason = format!(
            "{}|artifact_consumed_trend_reason={}",
            rollback_recommendation.reason, artifact_summary.consumed_trend_reason
        );
    }
    for factor in &artifact_summary.highlighted_factor_targets {
        if !promotion_decision.target_factors.contains(factor) {
            promotion_decision.target_factors.push(factor.clone());
        }
        if !rollback_recommendation.target_factors.contains(factor) {
            rollback_recommendation.target_factors.push(factor.clone());
        }
    }
    for family in &artifact_summary.highlighted_family_targets {
        if !promotion_decision.target_families.contains(family) {
            promotion_decision.target_families.push(family.clone());
        }
        if !rollback_recommendation.target_families.contains(family) {
            rollback_recommendation.target_families.push(family.clone());
        }
    }
}
