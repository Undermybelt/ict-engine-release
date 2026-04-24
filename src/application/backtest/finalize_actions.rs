use std::collections::BTreeSet;

use crate::application::artifacts::artifact_consumed_trend_signal;
use crate::state::{
    AgentActionItem, AgentActionPlan, ArtifactConsumedImpactSummary, ArtifactFactorTrendSummary,
    ArtifactFamilyTrendSummary, ExpectedStateChange, FactorFamilyOutcome, FactorIterationPrompt,
    PromotionDecision, RollbackRecommendation, WorkflowState,
};

pub fn workflow_state_from_context(
    decision_hint: &str,
    promotion_decision: &PromotionDecision,
    rollback_recommendation: &RollbackRecommendation,
) -> WorkflowState {
    if rollback_recommendation.should_rollback {
        WorkflowState {
            phase: if rollback_recommendation.scope.contains("artifact")
                || rollback_recommendation
                    .reason
                    .contains("artifact_consumption_validated_regression")
            {
                "artifact_rollback_review".to_string()
            } else {
                "rollback_review".to_string()
            },
            reason: rollback_recommendation.reason.clone(),
        }
    } else if !promotion_decision.approved {
        WorkflowState {
            phase: "research_iteration".to_string(),
            reason: promotion_decision.reason.clone(),
        }
    } else {
        WorkflowState {
            phase: "observe_or_deploy".to_string(),
            reason: decision_hint.to_string(),
        }
    }
}

pub fn build_agent_action_plan(
    decision_hint: &str,
    promotion_decision: &PromotionDecision,
    rollback_recommendation: &RollbackRecommendation,
    factor_iteration_queue: &[FactorIterationPrompt],
    family_outcomes: &[FactorFamilyOutcome],
) -> AgentActionPlan {
    let mut items = Vec::new();

    if rollback_recommendation.should_rollback {
        items.push(AgentActionItem {
            stage: "rollback".to_string(),
            blocking: true,
            priority: "high".to_string(),
            title: "Review Rollback".to_string(),
            rationale: rollback_recommendation.reason.clone(),
            expected_output:
                "Updated rollback assessment with confirmed scope and impacted factors".to_string(),
            expected_state_changes: vec![ExpectedStateChange {
                target: "rollback_recommendation".to_string(),
                direction: "confirm_or_narrow_scope".to_string(),
                rationale: rollback_recommendation.reason.clone(),
            }],
            suggested_files: vec![
                "src/main.rs".to_string(),
                "src/factors/weight_updater.rs".to_string(),
            ],
            suggested_commands: vec!["ict-engine update --feedback-file <file>".to_string()],
        });
    }

    if !promotion_decision.approved {
        items.push(AgentActionItem {
            stage: "promotion".to_string(),
            blocking: true,
            priority: "high".to_string(),
            title: "Block Promotion".to_string(),
            rationale: promotion_decision.reason.clone(),
            expected_output: "Promotion decision resolved with explicit hold or approval rationale"
                .to_string(),
            expected_state_changes: vec![ExpectedStateChange {
                target: "promotion_decision".to_string(),
                direction: "hold_until_thresholds_met".to_string(),
                rationale: promotion_decision.reason.clone(),
            }],
            suggested_files: vec![
                "src/state/types.rs".to_string(),
                "src/agent/prompts.rs".to_string(),
            ],
            suggested_commands: vec!["ict-engine factor-research --data <file>".to_string()],
        });
    }

    if let Some(next) = factor_iteration_queue.first() {
        items.push(AgentActionItem {
            stage: "iteration".to_string(),
            blocking: false,
            priority: "medium".to_string(),
            title: format!(
                "{} {}",
                next.iteration_action.to_uppercase(),
                next.factor_name
            ),
            rationale: next.prompt.clone(),
            expected_output:
                "A revised factor implementation or parameter set benchmarked against the current baseline"
                    .to_string(),
            expected_state_changes: vec![ExpectedStateChange {
                target: format!("factor:{}", next.factor_name),
                direction: next.iteration_action.clone(),
                rationale: next.prompt.clone(),
            }],
            suggested_files: vec![
                "src/factor_lab/factor_definition.rs".to_string(),
                "src/factors/registry.rs".to_string(),
            ],
            suggested_commands: vec!["ict-engine factor-backtest --data <file>".to_string()],
        });
    }

    for family in family_outcomes.iter().take(2) {
        if family.rollback_recommendation.should_rollback || !family.promotion_decision.approved {
            items.push(AgentActionItem {
                stage: "family_review".to_string(),
                blocking: false,
                priority: "medium".to_string(),
                title: format!("Family Review {}", family.family),
                rationale: if family.rollback_recommendation.should_rollback {
                    family.rollback_recommendation.reason.clone()
                } else {
                    family.promotion_decision.reason.clone()
                },
                expected_output: format!(
                    "A family-level decision note covering whether {} should be tuned, replaced, or held",
                    family.family
                ),
                expected_state_changes: vec![ExpectedStateChange {
                    target: format!("family:{}", family.family),
                    direction: if family.rollback_recommendation.should_rollback {
                        "review_for_rollback".to_string()
                    } else {
                        family.promotion_decision.status.clone()
                    },
                    rationale: if family.rollback_recommendation.should_rollback {
                        family.rollback_recommendation.reason.clone()
                    } else {
                        family.promotion_decision.reason.clone()
                    },
                }],
                suggested_files: vec![
                    "src/factor_lab/factor_definition.rs".to_string(),
                    "src/factors/weight_updater.rs".to_string(),
                ],
                suggested_commands: vec!["ict-engine factor-research --data <file>".to_string()],
            });
        }
    }

    AgentActionPlan {
        summary: decision_hint.to_string(),
        items,
    }
}

pub fn augment_action_plan_with_artifact_trends(
    action_plan: &mut AgentActionPlan,
    symbol: &str,
    state_dir: &str,
    factor_trends: &[ArtifactFactorTrendSummary],
    family_trends: &[ArtifactFamilyTrendSummary],
    consumed_impact_summary: &ArtifactConsumedImpactSummary,
) {
    let mut seen_titles = action_plan
        .items
        .iter()
        .map(|item| item.title.clone())
        .collect::<BTreeSet<_>>();

    for trend in factor_trends.iter().take(2) {
        if trend.decision_status == "observe" {
            continue;
        }
        let title = format!("Artifact Factor {}", trend.factor_name);
        if !seen_titles.insert(title.clone()) {
            continue;
        }
        action_plan.items.push(AgentActionItem {
            stage: "artifact_factor_review".to_string(),
            blocking: false,
            priority: if trend.rollback_link_status != "none" {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            title,
            rationale: trend.decision_reason.clone(),
            expected_output: format!(
                "Factor-level artifact review for {} with explicit keep/tune/rollback conclusion",
                trend.factor_name
            ),
            expected_state_changes: vec![ExpectedStateChange {
                target: format!("artifact_factor:{}", trend.factor_name),
                direction: trend.decision_status.clone(),
                rationale: trend.decision_reason.clone(),
            }],
            suggested_files: vec![
                "src/state/types.rs".to_string(),
                "src/factors/weight_updater.rs".to_string(),
            ],
            suggested_commands: vec![
                format!(
                    "ict-engine workflow-status --symbol {} --state-dir {} --phase artifact-factor-trends",
                    symbol, state_dir
                ),
                format!(
                    "ict-engine artifact-status --symbol {} --state-dir {} --kind pending_update --sort-by improvement --limit 5",
                    symbol, state_dir
                ),
            ],
        });
    }

    for trend in family_trends.iter().take(2) {
        if trend.decision_status == "observe" {
            continue;
        }
        let title = format!("Artifact Family {}", trend.family);
        if !seen_titles.insert(title.clone()) {
            continue;
        }
        action_plan.items.push(AgentActionItem {
            stage: "artifact_family_review".to_string(),
            blocking: false,
            priority: if trend.rollback_link_status != "none" {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            title,
            rationale: trend.decision_reason.clone(),
            expected_output: format!(
                "Family-level artifact review for {} with promotion/rollback linkage",
                trend.family
            ),
            expected_state_changes: vec![ExpectedStateChange {
                target: format!("artifact_family:{}", trend.family),
                direction: trend.decision_status.clone(),
                rationale: trend.decision_reason.clone(),
            }],
            suggested_files: vec![
                "src/state/types.rs".to_string(),
                "src/factor_lab/factor_definition.rs".to_string(),
            ],
            suggested_commands: vec![
                format!(
                    "ict-engine workflow-status --symbol {} --state-dir {} --phase artifact-family-trends",
                    symbol, state_dir
                ),
                format!(
                    "ict-engine workflow-status --symbol {} --state-dir {} --phase artifact-family-rule-break-impacts",
                    symbol, state_dir
                ),
            ],
        });
    }

    let (consumed_trend_status, consumed_trend_reason, consumed_target_kinds) =
        artifact_consumed_trend_signal(consumed_impact_summary);
    if matches!(
        consumed_trend_status.as_str(),
        "validated_improving" | "validated_regressing"
    ) {
        let title = "Artifact Consumption Validation".to_string();
        if seen_titles.insert(title.clone()) {
            let mut expected_state_changes = vec![ExpectedStateChange {
                target: "artifact_consumption".to_string(),
                direction: consumed_trend_status.clone(),
                rationale: consumed_trend_reason.clone(),
            }];
            expected_state_changes.extend(consumed_target_kinds.iter().map(|kind| {
                ExpectedStateChange {
                    target: format!("artifact_kind:{}", kind),
                    direction: consumed_trend_status.clone(),
                    rationale: consumed_trend_reason.clone(),
                }
            }));
            action_plan.items.push(AgentActionItem {
                stage: "artifact_consumption_review".to_string(),
                blocking: consumed_trend_status == "validated_regressing",
                priority: if consumed_trend_status == "validated_regressing" {
                    "high".to_string()
                } else {
                    "medium".to_string()
                },
                title,
                rationale: consumed_trend_reason,
                expected_output:
                    "A consumption-validation note covering whether realized artifact use is improving or regressing".to_string(),
                expected_state_changes,
                suggested_files: vec![
                    "src/main.rs".to_string(),
                    "src/state/types.rs".to_string(),
                    "src/factors/weight_updater.rs".to_string(),
                ],
                suggested_commands: vec![
                    format!(
                        "ict-engine workflow-status --symbol {} --state-dir {} --phase artifact-impact-consumed-trend",
                        symbol, state_dir
                    ),
                    format!(
                        "ict-engine artifact-status --symbol {} --state-dir {} --consumed-only --sort-by regression --limit 5",
                        symbol, state_dir
                    ),
                ],
            });
        }
    }
}
