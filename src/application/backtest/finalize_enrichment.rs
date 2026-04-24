use std::collections::BTreeMap;

use crate::agent::AgentPrompt;
use crate::backtest_report_shell::BacktestReport;
use crate::state::{
    AgentActionPlan, AgentContextBundle, AgentContextBundleMinimal, CommandRecommendations,
    DatasetComparability, DecisionHistorySummary, FactorFamilyDiff, FactorFamilyHistory,
    FactorFamilyOutcome, ProbabilityDiff, PromotionDecision, RankingDiffItem,
    RollbackRecommendation, WorkflowState,
};

pub struct FinalizeBacktestEnrichmentInput<'a> {
    pub report: &'a mut BacktestReport,
    pub decision_thresholds: crate::state::DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    pub factor_family_history: Vec<FactorFamilyHistory>,
    pub decision_history_summary: DecisionHistorySummary,
    pub agent_action_plan: AgentActionPlan,
    pub workflow_state: WorkflowState,
    pub artifact_action_summary: Vec<String>,
    pub artifact_decision_summary: crate::state::ArtifactDecisionSummary,
    pub artifact_decision_section: crate::state::ArtifactDecisionSection,
    pub recommended_commands: CommandRecommendations,
    pub recommended_next_command: String,
    pub agent_context_bundle: AgentContextBundle,
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    pub score_deltas: Vec<RankingDiffItem>,
    pub probability_deltas: Vec<ProbabilityDiff>,
    pub final_trade_outcome_cpt: BTreeMap<String, BTreeMap<String, f64>>,
    pub dataset_audit_prompt: AgentPrompt,
    pub promotion_gate_prompt: AgentPrompt,
    pub rollback_review_prompt: AgentPrompt,
}

pub fn apply_finalize_backtest_enrichment(input: FinalizeBacktestEnrichmentInput<'_>) {
    let FinalizeBacktestEnrichmentInput {
        report,
        decision_thresholds,
        dataset_comparability,
        promotion_decision,
        rollback_recommendation,
        factor_family_outcomes,
        factor_family_diffs,
        factor_family_history,
        decision_history_summary,
        agent_action_plan,
        workflow_state,
        artifact_action_summary,
        artifact_decision_summary,
        artifact_decision_section,
        recommended_commands,
        recommended_next_command,
        agent_context_bundle,
        agent_context_bundle_minimal,
        score_deltas,
        probability_deltas,
        final_trade_outcome_cpt,
        dataset_audit_prompt,
        promotion_gate_prompt,
        rollback_review_prompt,
    } = input;

    report.decision_thresholds = decision_thresholds;
    report.dataset_comparability = dataset_comparability;
    report.promotion_decision = promotion_decision;
    report.rollback_recommendation = rollback_recommendation;
    report.factor_family_outcomes = factor_family_outcomes;
    report.factor_family_diffs = factor_family_diffs;
    report.factor_family_history = factor_family_history;
    report.decision_history_summary = decision_history_summary;
    report.agent_action_plan = agent_action_plan;
    report.workflow_state = workflow_state;
    report.artifact_action_summary = artifact_action_summary;
    report.artifact_decision_summary = artifact_decision_summary;
    report.artifact_decision_section = artifact_decision_section;
    report.recommended_commands = recommended_commands;
    report.recommended_next_command = recommended_next_command;
    report.agent_context_bundle = agent_context_bundle;
    report.agent_context_bundle_minimal = agent_context_bundle_minimal;
    report.factor_score_deltas = score_deltas;
    report.trade_outcome_deltas = probability_deltas;
    report.final_trade_outcome_cpt = final_trade_outcome_cpt;
    report.agent_prompts.prompts.insert(0, dataset_audit_prompt);
    report.agent_prompts.prompts.push(promotion_gate_prompt);
    report.agent_prompts.prompts.push(rollback_review_prompt);
}
