use crate::application::decision_utils::{
    derive_family_outcomes, derive_promotion_decision, derive_rollback_recommendation,
    ArtifactConsumedDecisionGate,
};
use crate::state::{
    BacktestRunRecord, DatasetComparability, DecisionHistorySummary, FactorFamilyDecision,
    FactorFamilyDiff, FactorFamilyHistory, FactorFamilyOutcome, PersistedFactorRanking,
    ProbabilityDiff, PromotionDecision, RankingDiffItem, RollbackRecommendation,
};

pub struct FinalizeBacktestDecisionSurfacesInput<'a> {
    pub previous_runs: &'a [BacktestRunRecord],
    pub factor_ranking: &'a [PersistedFactorRanking],
    pub factor_family_decisions: &'a [FactorFamilyDecision],
    pub score_deltas: &'a [RankingDiffItem],
    pub probability_deltas: &'a [ProbabilityDiff],
    pub dataset_comparability: &'a DatasetComparability,
    pub artifact_consumed_gate: &'a ArtifactConsumedDecisionGate,
    pub artifact_family_trends: &'a [crate::state::ArtifactFamilyTrendSummary],
}

pub struct FinalizeBacktestDecisionSurfaces {
    pub decision_thresholds: crate::state::DecisionThresholds,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    pub decision_history_summary: DecisionHistorySummary,
    pub factor_family_history: Vec<FactorFamilyHistory>,
}

pub fn derive_finalize_backtest_decision_surfaces(
    input: FinalizeBacktestDecisionSurfacesInput<'_>,
) -> FinalizeBacktestDecisionSurfaces {
    let FinalizeBacktestDecisionSurfacesInput {
        previous_runs,
        factor_ranking,
        factor_family_decisions,
        score_deltas,
        probability_deltas,
        dataset_comparability,
        artifact_consumed_gate,
        artifact_family_trends,
    } = input;

    let decision_thresholds = crate::application::backtest::decision_thresholds();
    let promotion_decision = derive_promotion_decision(
        factor_ranking,
        score_deltas,
        dataset_comparability,
        &decision_thresholds,
        Some(artifact_consumed_gate),
    );
    let rollback_recommendation = derive_rollback_recommendation(
        factor_ranking,
        score_deltas,
        probability_deltas,
        dataset_comparability,
        &decision_thresholds,
        Some(artifact_consumed_gate),
    );
    let factor_family_outcomes = derive_family_outcomes(
        factor_family_decisions,
        &decision_thresholds,
        dataset_comparability,
        Some(artifact_family_trends),
    );
    let factor_family_diffs = crate::application::backtest::family_diffs(
        previous_runs
            .last()
            .map(|run| run.factor_family_decisions.as_slice())
            .unwrap_or(&[]),
        factor_family_decisions,
    );
    let decision_history_summary =
        crate::application::backtest::decision_history_summary(previous_runs.iter().map(|run| {
            (
                run.promotion_decision.clone(),
                run.rollback_recommendation.clone(),
            )
        }));
    let factor_family_history =
        crate::application::backtest::family_history_from_runs(previous_runs.iter().map(|run| {
            (
                run.run_id.clone(),
                run.timestamp,
                run.factor_family_decisions.clone(),
            )
        }));

    FinalizeBacktestDecisionSurfaces {
        decision_thresholds,
        promotion_decision,
        rollback_recommendation,
        factor_family_outcomes,
        factor_family_diffs,
        decision_history_summary,
        factor_family_history,
    }
}
