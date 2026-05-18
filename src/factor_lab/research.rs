use crate::agent::{factor_iteration_prompt_pack, AgentPromptPack};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::factor_lab::{
    BacktestConfig, BacktestResult, FactorBacktestEngine, FactorContext, FactorResearchEngine,
};
use crate::factors::{FactorRegistry, WeightUpdater};
use crate::state::{
    AgentActionPlan, AgentContextBundle, AgentContextBundleMinimal, CommandRecommendations,
    DatasetComparability, DecisionHistorySummary, DecisionThresholds, FactorFamilyDiff,
    FactorFamilyHistory, FactorFamilyOutcome, FactorMutationEvaluation, FeedbackFactorUsage,
    FeedbackHistorySummary, FeedbackRecord, LearningState, ModelProbabilitySnapshot,
    PreBayesEvidenceFilter, PromotionDecision, RankingDiffItem, RollbackRecommendation,
    RunProvenance, WorkflowState,
};
use crate::types::Candle;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchReport {
    pub factor_count: usize,
    #[serde(default)]
    pub research_objective: String,
    #[serde(default)]
    pub objective_surfaces: Vec<HashMap<String, String>>,
    pub best_factor: Option<String>,
    pub aggregate_return: f64,
    pub feedback_records_generated: usize,
    pub feedback_records_applied: usize,
    pub feedback_history_summary: FeedbackHistorySummary,
    pub factor_score_deltas: Vec<RankingDiffItem>,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub factor_family_decisions: Vec<crate::state::FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    pub factor_family_history: Vec<FactorFamilyHistory>,
    pub decision_history_summary: DecisionHistorySummary,
    pub agent_prompts: AgentPromptPack,
    pub workflow_state: WorkflowState,
    pub agent_action_plan: AgentActionPlan,
    pub agent_context_bundle: AgentContextBundle,
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    pub recommended_commands: CommandRecommendations,
    pub recommended_next_command: String,
    pub artifact_action_summary: Vec<String>,
    pub artifact_decision_summary: crate::state::ArtifactDecisionSummary,
    pub artifact_decision_section: crate::state::ArtifactDecisionSection,
    pub workflow_snapshot: crate::state::WorkflowSnapshot,
    pub backtest: BacktestResult,
    #[serde(default)]
    pub pre_bayes_evidence_filter: PreBayesEvidenceFilter,
    pub factor_mutation_evaluation: Option<FactorMutationEvaluation>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
}

pub struct FactorLab {
    registry: FactorRegistry,
}

impl FactorLab {
    pub fn new(registry: FactorRegistry) -> Self {
        Self { registry }
    }

    pub fn run_research<'a>(
        &self,
        symbol: &str,
        candles: &[Candle],
        context: &FactorContext<'a>,
        learning_state: Option<&mut LearningState>,
        config: &BacktestConfig,
        persist_feedback: bool,
    ) -> Result<ResearchReport> {
        let mut registry = self.registry.clone();
        if let Some(state) = learning_state.as_ref() {
            registry.apply_learning_state(state);
        }

        let engine = FactorResearchEngine::new(registry);
        let backtest = FactorBacktestEngine::new(engine);
        let mut result = backtest.run(candles, context, learning_state.as_deref(), config)?;

        let feedback_records = if persist_feedback {
            backtest_feedback_records(symbol, &result)
        } else {
            Vec::new()
        };

        let mut applied_feedback_count = 0usize;
        let mut prompt_rankings = result.scorecards.clone();
        let mut prompt_queue = result.iteration_queue.clone();
        let mut prompt_feedback_summary = FeedbackHistorySummary::default();
        if let Some(state) = learning_state {
            let updater = WeightUpdater::default();
            updater.apply_rankings(state, &result.rankings);
            if !feedback_records.is_empty() {
                let new_feedback = state.merge_feedback_records(&feedback_records);
                applied_feedback_count = new_feedback.len();
                updater.apply_feedback(state, &new_feedback);
            }
            prompt_rankings = state.factor_rankings.clone();
            prompt_queue = state.iteration_queue();
            prompt_feedback_summary = state.summary();
        }
        let decision_state = LearningState {
            factor_rankings: prompt_rankings.clone(),
            ..LearningState::default()
        };
        let factor_family_decisions = decision_state.family_decisions();

        let agent_prompts = factor_iteration_prompt_pack(
            symbol,
            &prompt_rankings,
            &prompt_queue,
            &prompt_feedback_summary,
        );
        result.scorecards = prompt_rankings.clone();
        result.iteration_queue = prompt_queue.clone();
        result.agent_prompts = agent_prompts.clone();
        result.feedback_records_generated = feedback_records.len();
        result.feedback_records_applied = applied_feedback_count;
        result.feedback_history_summary = prompt_feedback_summary.clone();
        result.factor_family_decisions = factor_family_decisions.clone();

        Ok(ResearchReport {
            factor_count: result.factor_results.len(),
            research_objective: String::new(),
            objective_surfaces: Vec::new(),
            best_factor: result.best_factor.clone(),
            aggregate_return: result.aggregate_return,
            feedback_records_generated: feedback_records.len(),
            feedback_records_applied: applied_feedback_count,
            feedback_history_summary: prompt_feedback_summary,
            factor_score_deltas: Vec::new(),
            provenance: RunProvenance::default(),
            decision_thresholds: DecisionThresholds::default(),
            dataset_comparability: DatasetComparability::default(),
            promotion_decision: PromotionDecision::default(),
            rollback_recommendation: RollbackRecommendation::default(),
            factor_family_decisions,
            factor_family_outcomes: Vec::new(),
            factor_family_diffs: Vec::new(),
            factor_family_history: Vec::new(),
            decision_history_summary: DecisionHistorySummary::default(),
            agent_prompts,
            workflow_state: WorkflowState::default(),
            agent_action_plan: AgentActionPlan::default(),
            agent_context_bundle: AgentContextBundle::default(),
            agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
            recommended_commands: CommandRecommendations::default(),
            recommended_next_command: "recommended_command_unavailable".to_string(),
            artifact_action_summary: Vec::new(),
            artifact_decision_summary: crate::state::ArtifactDecisionSummary::default(),
            artifact_decision_section: crate::state::ArtifactDecisionSection::default(),
            workflow_snapshot: crate::state::WorkflowSnapshot::default(),
            backtest: result,
            pre_bayes_evidence_filter: PreBayesEvidenceFilter::default(),
            factor_mutation_evaluation: None,
            multi_timeframe_summary: Vec::new(),
        })
    }
}

fn backtest_feedback_records(symbol: &str, result: &BacktestResult) -> Vec<FeedbackRecord> {
    let ranking_map = result
        .rankings
        .iter()
        .map(|ranking| (ranking.factor_name.clone(), ranking.weight))
        .collect::<HashMap<_, _>>();

    result
        .factor_results
        .iter()
        .flat_map(|factor_result| {
            let weight = ranking_map
                .get(&factor_result.factor_name)
                .copied()
                .unwrap_or(0.2);
            factor_result.trades.iter().map(move |trade| {
                let directional_support =
                    (trade.signal_value * trade.signal_confidence * weight).abs();
                let (long_support, short_support) = match trade.direction {
                    crate::types::Direction::Bull => (directional_support, 0.0),
                    crate::types::Direction::Bear => (0.0, directional_support),
                    crate::types::Direction::Neutral => (0.0, 0.0),
                };
                FeedbackRecord {
                    timestamp: trade.entry_time,
                    symbol: symbol.to_string(),
                    source: "factor_research_backtest".to_string(),
                    run_id: None,
                    trade_id: Some(format!(
                        "{}:{}:{}",
                        trade.factor_name,
                        trade.entry_time.to_rfc3339(),
                        trade.exit_time.to_rfc3339()
                    )),
                    prompt_version: Some(crate::agent::PROMPT_PACK_VERSION.to_string()),
                    factor_version: None,
                    data_fingerprint: None,
                    factors_used: vec![FeedbackFactorUsage {
                        factor_name: trade.factor_name.clone(),
                        category: "factor_backtest".to_string(),
                        direction: trade.direction,
                        value: trade.signal_value,
                        confidence: trade.signal_confidence,
                        weight,
                        long_support,
                        short_support,
                        uncertainty_contribution: (1.0 - trade.signal_confidence).clamp(0.0, 1.0),
                    }],
                    model_probabilities_before_trade: ModelProbabilitySnapshot {
                        selected_direction: trade.direction,
                        selected_probability: trade.signal_confidence,
                        long_score: if trade.direction == crate::types::Direction::Bull {
                            directional_support
                        } else {
                            0.0
                        },
                        short_score: if trade.direction == crate::types::Direction::Bear {
                            directional_support
                        } else {
                            0.0
                        },
                        win_prob_long: if trade.direction == crate::types::Direction::Bull {
                            trade.signal_confidence
                        } else {
                            0.0
                        },
                        win_prob_short: if trade.direction == crate::types::Direction::Bear {
                            trade.signal_confidence
                        } else {
                            0.0
                        },
                        uncertainty: (1.0 - trade.signal_confidence).clamp(0.0, 1.0),
                    },
                    realized_outcome: if trade.pnl > 1e-12 {
                        "win".to_string()
                    } else if trade.pnl < -1e-12 {
                        "loss".to_string()
                    } else {
                        "breakeven".to_string()
                    },
                    pnl: trade.pnl,
                    regime_at_entry: trade.regime_at_entry,
                    structural_feedback: None,
                    reflection_mismatch_tags: Vec::new(),
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factors::FactorRegistry;
    use chrono::{Duration, TimeZone, Utc};

    fn candles(count: usize) -> Vec<Candle> {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        (0..count)
            .map(|index| {
                let drift = index as f64 * 0.25;
                Candle {
                    timestamp: start + Duration::minutes(index as i64),
                    open: 100.0 + drift,
                    high: 100.7 + drift,
                    low: 99.4 + drift,
                    close: 100.4 + drift,
                    volume: 1_000.0,
                }
            })
            .collect()
    }

    #[test]
    fn test_run_research_can_persist_feedback_learning() {
        let lab = FactorLab::new(FactorRegistry::default());
        let mut learning_state = LearningState::default();

        let report = lab
            .run_research(
                "NQ",
                &candles(140),
                &FactorContext::default(),
                Some(&mut learning_state),
                &BacktestConfig::default(),
                true,
            )
            .unwrap();

        assert!(report.factor_count > 0);
        assert!(report.feedback_records_generated > 0);
        assert!(report.feedback_records_applied > 0);
        assert!(!learning_state.feedback_history.is_empty());
    }
}
