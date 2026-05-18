use std::collections::BTreeMap;

use chrono::Utc;
use serde::Serialize;

use crate::agent::AgentPromptPack;
use crate::state::{
    AgentActionPlan, AgentContextBundle, AgentContextBundleMinimal, CommandRecommendations,
    DatasetComparability, DecisionHistorySummary, DecisionThresholds, FactorFamilyDecision,
    FactorFamilyDiff, FactorFamilyHistory, FactorFamilyOutcome, FactorIterationPrompt,
    FeedbackHistorySummary, PersistedFactorRanking, ProbabilityDiff, PromotionDecision,
    RankingDiffItem, RollbackRecommendation, RunProvenance, WorkflowSnapshot, WorkflowState,
};
use crate::types::{Direction, Regime};

#[derive(Debug, Serialize)]
pub struct BacktestReport {
    pub symbol: String,
    pub state_dir: String,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub bars: usize,
    pub warmup_bars: usize,
    pub hold_bars: usize,
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub fee_bps: f64,
    pub ambiguous_bar_policy: String,
    pub window_mode: String,
    pub evidence_policy: String,
    pub ict_role: String,
    pub online_learning: bool,
    pub learning_updates: usize,
    pub signals: usize,
    pub trades: usize,
    pub metrics: BacktestMetricsSummary,
    pub equity_curve: Vec<f64>,
    pub regime_metrics: Vec<BacktestRegimeSummary>,
    pub factor_ranking: Vec<PersistedFactorRanking>,
    pub factor_score_deltas: Vec<RankingDiffItem>,
    pub trade_outcome_deltas: Vec<ProbabilityDiff>,
    pub factor_iteration_queue: Vec<FactorIterationPrompt>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    pub factor_family_history: Vec<FactorFamilyHistory>,
    pub decision_history_summary: DecisionHistorySummary,
    pub agent_action_plan: AgentActionPlan,
    pub workflow_state: WorkflowState,
    pub agent_context_bundle: AgentContextBundle,
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    pub recommended_commands: CommandRecommendations,
    pub recommended_next_command: String,
    pub artifact_action_summary: Vec<String>,
    pub artifact_decision_summary: crate::state::ArtifactDecisionSummary,
    pub artifact_decision_section: crate::state::ArtifactDecisionSection,
    pub agent_prompts: AgentPromptPack,
    pub feedback_history_summary: FeedbackHistorySummary,
    pub multi_timeframe_summary: Vec<String>,
    pub last_decision: Option<crate::planner::ProbabilisticDecisionSnapshot>,
    pub final_trade_outcome_cpt: BTreeMap<String, BTreeMap<String, f64>>,
    pub recent_trades: Vec<BacktestTradeSample>,
    pub workflow_snapshot: WorkflowSnapshot,
    pub objective_market_credibility_shrink:
        Option<crate::domain::belief::ObjectiveMarketCredibilityShrink>,
}

#[derive(Debug, Serialize)]
pub struct BacktestMetricsSummary {
    pub total_return: f64,
    pub sharpe: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub conformal_coverage_1sigma: f64,
    pub conformal_miscoverage_1sigma: f64,
    pub mean_prediction_interval_half_width: f64,
    pub worst_window_miscoverage: f64,
    pub regime_break_penalty: f64,
    pub structural_break_score: f64,
    pub structural_break_index: Option<usize>,
    pub structural_break_detected: bool,
    pub signal_structural_break_score: f64,
    pub signal_structural_break_index: Option<usize>,
    pub signal_structural_break_detected: bool,
    pub residual_structural_break_score: f64,
    pub residual_structural_break_index: Option<usize>,
    pub residual_structural_break_detected: bool,
    pub rolling_ic_structural_break_score: f64,
    pub rolling_ic_structural_break_index: Option<usize>,
    pub rolling_ic_structural_break_detected: bool,
}

#[derive(Debug, Serialize)]
pub struct BacktestRegimeSummary {
    pub regime: Regime,
    pub win_rate: f64,
    pub avg_pnl: f64,
}

#[derive(Debug, Serialize)]
pub struct BacktestTradeSample {
    pub timestamp: chrono::DateTime<Utc>,
    pub direction: Direction,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl: f64,
    pub long_score: f64,
    pub short_score: f64,
    pub win_prob_long: f64,
    pub win_prob_short: f64,
    pub ict_role: String,
}
