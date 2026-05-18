use std::collections::BTreeMap;

use anyhow::{anyhow, Result};

use crate::agent::{factor_iteration_prompt_pack, AgentPrompt, AgentPromptInput, AgentPromptPack};
use crate::application::belief::pipeline_shared::probability_map;
use crate::backtest::{Metrics, RegimeSplit};
use crate::backtest_report_shell::{
    BacktestMetricsSummary, BacktestRegimeSummary, BacktestReport, BacktestTradeSample,
};
use crate::state::{
    AgentActionPlan, AgentContextBundle, AgentContextBundleMinimal, CommandRecommendations,
    DatasetComparability, DecisionHistorySummary, FeedbackHistorySummary, LearningState,
    PromotionDecision, RollbackRecommendation, WorkflowSnapshot, WorkflowState,
};
use crate::types::TradeRecord;

pub struct BuildRuntimeBacktestReportInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub bars: usize,
    pub warmup_bars: usize,
    pub hold_bars: usize,
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub fee_bps: f64,
    pub ambiguous_bar_policy: String,
    pub online_learning: bool,
    pub learning_updates: usize,
    pub signals: usize,
    pub trades: &'a [TradeRecord],
    pub learning_state: &'a LearningState,
    pub network: &'a crate::bbn::BayesianNetwork,
    pub last_decision: Option<crate::planner::ProbabilisticDecisionSnapshot>,
}

pub fn build_runtime_backtest_report(
    input: BuildRuntimeBacktestReportInput<'_>,
) -> Result<BacktestReport> {
    let BuildRuntimeBacktestReportInput {
        symbol,
        state_dir,
        bars,
        warmup_bars,
        hold_bars,
        spread_bps,
        slippage_bps,
        fee_bps,
        ambiguous_bar_policy,
        online_learning,
        learning_updates,
        signals,
        trades,
        learning_state,
        network,
        last_decision,
    } = input;

    let trade_returns: Vec<f64> = trades.iter().map(|trade| trade.pnl).collect();
    let equity_curve = build_equity_curve(&trade_returns);
    let total_return = equity_curve.last().copied().unwrap_or(1.0) - 1.0;
    let regime_metrics = RegimeSplit::regime_metrics(trades)
        .into_iter()
        .map(|(regime, win_rate, avg_pnl)| BacktestRegimeSummary {
            regime,
            win_rate,
            avg_pnl,
        })
        .collect();
    let recent_trades = trades
        .iter()
        .rev()
        .take(5)
        .map(backtest_trade_sample)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let factor_ranking = learning_state.factor_rankings.clone();
    let factor_iteration_queue = learning_state.iteration_queue();
    let factor_family_decisions = learning_state.family_decisions();
    let feedback_history_summary = learning_state.summary();
    let final_trade_outcome_cpt = trade_outcome_cpt_snapshot(network)?;
    let agent_prompts = build_backtest_agent_prompts(
        symbol,
        &factor_ranking,
        &factor_iteration_queue,
        &feedback_history_summary,
        total_return,
        trades.len(),
        &final_trade_outcome_cpt,
    );

    Ok(BacktestReport {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        provenance: crate::state::RunProvenance::default(),
        decision_thresholds: crate::state::DecisionThresholds::default(),
        dataset_comparability: DatasetComparability::default(),
        promotion_decision: PromotionDecision::default(),
        rollback_recommendation: RollbackRecommendation::default(),
        bars,
        warmup_bars,
        hold_bars,
        spread_bps,
        slippage_bps,
        fee_bps,
        ambiguous_bar_policy,
        window_mode: "expanding".to_string(),
        evidence_policy: "same_as_analyze_json_snapshot".to_string(),
        ict_role: "evidence_only_non_deterministic".to_string(),
        online_learning,
        learning_updates,
        signals,
        trades: trades.len(),
        metrics: BacktestMetricsSummary {
            total_return,
            sharpe: Metrics::sharpe(&trade_returns, 0.0),
            max_drawdown: Metrics::max_drawdown(&equity_curve),
            win_rate: Metrics::win_rate(trades),
            profit_factor: Metrics::profit_factor(trades),
            conformal_coverage_1sigma: 0.0,
            conformal_miscoverage_1sigma: 0.0,
            mean_prediction_interval_half_width: 0.0,
            worst_window_miscoverage: 0.0,
            regime_break_penalty: 0.0,
            structural_break_score: 0.0,
            structural_break_index: None,
            structural_break_detected: false,
            signal_structural_break_score: 0.0,
            signal_structural_break_index: None,
            signal_structural_break_detected: false,
            residual_structural_break_score: 0.0,
            residual_structural_break_index: None,
            residual_structural_break_detected: false,
            rolling_ic_structural_break_score: 0.0,
            rolling_ic_structural_break_index: None,
            rolling_ic_structural_break_detected: false,
        },
        equity_curve,
        regime_metrics,
        factor_ranking,
        factor_score_deltas: Vec::new(),
        trade_outcome_deltas: Vec::new(),
        factor_iteration_queue,
        factor_family_decisions,
        factor_family_outcomes: Vec::new(),
        factor_family_diffs: Vec::new(),
        factor_family_history: Vec::new(),
        decision_history_summary: DecisionHistorySummary::default(),
        agent_action_plan: AgentActionPlan::default(),
        workflow_state: WorkflowState::default(),
        agent_context_bundle: AgentContextBundle::default(),
        agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
        recommended_commands: CommandRecommendations::default(),
        recommended_next_command: "recommended_command_unavailable".to_string(),
        artifact_action_summary: Vec::new(),
        artifact_decision_summary: crate::state::ArtifactDecisionSummary::default(),
        artifact_decision_section: crate::state::ArtifactDecisionSection::default(),
        agent_prompts,
        feedback_history_summary,
        multi_timeframe_summary: Vec::new(),
        last_decision,
        final_trade_outcome_cpt,
        recent_trades,
        workflow_snapshot: WorkflowSnapshot::default(),
        objective_market_credibility_shrink: None,
    })
}

pub fn trade_outcome_cpt_snapshot(
    network: &crate::bbn::BayesianNetwork,
) -> Result<BTreeMap<String, BTreeMap<String, f64>>> {
    let trade_outcome = network
        .nodes
        .get("trade_outcome")
        .ok_or_else(|| anyhow!("missing node 'trade_outcome'"))?;
    let entry_quality = network
        .nodes
        .get("entry_quality")
        .ok_or_else(|| anyhow!("missing node 'entry_quality'"))?;

    let mut snapshot = BTreeMap::new();
    let factor_alignment = network
        .nodes
        .get("factor_alignment")
        .ok_or_else(|| anyhow!("missing node 'factor_alignment'"))?;
    let factor_uncertainty = network
        .nodes
        .get("factor_uncertainty")
        .ok_or_else(|| anyhow!("missing node 'factor_uncertainty'"))?;
    let alignment_index = factor_alignment
        .state_index("mixed")
        .ok_or_else(|| anyhow!("missing state 'mixed' on factor_alignment"))?;
    let uncertainty_index = factor_uncertainty
        .state_index("low")
        .ok_or_else(|| anyhow!("missing state 'low' on factor_uncertainty"))?;

    for (entry_index, entry_state) in entry_quality.states.iter().enumerate() {
        let probabilities = trade_outcome
            .cpt
            .get(&vec![entry_index, alignment_index, uncertainty_index])
            .ok_or_else(|| {
                anyhow!(
                    "missing CPT entry for entry_quality index {} with baseline factor evidence",
                    entry_index
                )
            })?;
        snapshot.insert(
            entry_state.clone(),
            probability_map(&trade_outcome.states, probabilities),
        );
    }

    Ok(snapshot)
}

pub fn build_backtest_agent_prompts(
    symbol: &str,
    factor_ranking: &[crate::state::PersistedFactorRanking],
    factor_iteration_queue: &[crate::state::FactorIterationPrompt],
    feedback_history_summary: &FeedbackHistorySummary,
    total_return: f64,
    trades: usize,
    final_trade_outcome_cpt: &BTreeMap<String, BTreeMap<String, f64>>,
) -> AgentPromptPack {
    let mut pack = factor_iteration_prompt_pack(
        symbol,
        factor_ranking,
        factor_iteration_queue,
        feedback_history_summary,
    );
    pack.workflow = format!(
        "Use backtest performance, updated factor scorecards, and final trade_outcome CPT state to decide the next agent iteration plan for {}.",
        symbol
    );
    pack.prompts.push(AgentPrompt::new(AgentPromptInput {
        id: "backtest_model_review".to_string(),
        stage: "backtest_review".to_string(),
        priority: "high".to_string(),
        objective: "Review whether factor/BBN updates improved the model or just overfit recent trades.".to_string(),
        system_prompt: "You are the backtest-review agent. Use the final CPT snapshot, total return, trade count, and factor iteration queue to decide whether the next change should target factor definitions, factor weighting, or BBN evidence mapping.".to_string(),
        user_prompt: format!(
            "Symbol={} total_return={:.6} trade_count={} factor_iteration_queue={:?} final_trade_outcome_cpt={:?}",
            symbol, total_return, trades, factor_iteration_queue, final_trade_outcome_cpt
        ),
        success_criteria: vec![
            "Prefer factor replacement only when composite score and CPT-adjusted evidence both remain weak".to_string(),
            "If BBN outcome probabilities shifted but factor scores did not improve, review evidence mapping before replacing factors".to_string(),
        ],
        suggested_files: vec![
            "src/main.rs".to_string(),
            "src/factors/weight_updater.rs".to_string(),
            "src/bbn/trading/topology.rs".to_string(),
        ],
    }));
    pack
}

fn build_equity_curve(returns: &[f64]) -> Vec<f64> {
    let mut equity = 1.0;
    let mut curve = vec![equity];

    for trade_return in returns {
        equity *= 1.0 + trade_return;
        curve.push(equity);
    }

    curve
}

fn backtest_trade_sample(trade: &TradeRecord) -> BacktestTradeSample {
    BacktestTradeSample {
        timestamp: trade.timestamp,
        direction: trade.direction,
        entry_price: trade.entry_price,
        exit_price: trade.exit_price,
        pnl: trade.pnl,
        long_score: *trade.factor_values.get("long_score").unwrap_or(&0.0),
        short_score: *trade.factor_values.get("short_score").unwrap_or(&0.0),
        win_prob_long: *trade.factor_values.get("win_prob_long").unwrap_or(&0.0),
        win_prob_short: *trade.factor_values.get("win_prob_short").unwrap_or(&0.0),
        ict_role: "evidence_only_non_deterministic".to_string(),
    }
}
