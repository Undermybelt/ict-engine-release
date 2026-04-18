use crate::agent::AgentPromptPack;
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use crate::factor_lab::{BacktestMetrics, FactorContext, FactorEngine};
use crate::factors::{ICCalculator, WeightUpdater};
use crate::state::{
    AgentActionPlan, AgentContextBundle, AgentContextBundleMinimal, CommandRecommendations,
    DatasetComparability, DecisionHistorySummary, DecisionThresholds, FactorFamilyDiff,
    FactorFamilyHistory, FactorFamilyOutcome, FactorIterationPrompt, FeedbackHistorySummary,
    LearningState, PersistedFactorRanking, ProbabilityDiff, PromotionDecision,
    RollbackRecommendation, RunProvenance, WorkflowState,
};
use crate::types::{Candle, Direction, FactorIC, Regime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub initial_capital: f64,
    pub signal_threshold: f64,
    pub max_hold_bars: usize,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub fee_rate: f64,
    pub slippage_rate: f64,
    pub position_size: f64,
    pub allow_long: bool,
    pub allow_short: bool,
    pub train_bars: usize,
    pub test_bars: usize,
    pub step_bars: usize,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100_000.0,
            signal_threshold: 0.15,
            max_hold_bars: 6,
            stop_loss_pct: 0.006,
            take_profit_pct: 0.010,
            fee_rate: 0.0004,
            slippage_rate: 0.0003,
            position_size: 0.10,
            allow_long: true,
            allow_short: true,
            train_bars: 60,
            test_bars: 30,
            step_bars: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorTrade {
    pub factor_name: String,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub direction: Direction,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl: f64,
    pub holding_bars: usize,
    pub regime_at_entry: Regime,
    pub signal_value: f64,
    pub signal_confidence: f64,
    pub signal_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardWindow {
    pub start_index: usize,
    pub end_index: usize,
    pub metrics: BacktestMetrics,
    pub ic: f64,
    pub ir: f64,
    #[serde(default)]
    pub conformal: WindowConformalDiagnostics,
    #[serde(default)]
    pub structural_break: StructuralBreakDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowConformalDiagnostics {
    pub predicted_return: f64,
    pub realized_return: f64,
    pub interval_lower: f64,
    pub interval_upper: f64,
    pub interval_half_width: f64,
    pub covered: bool,
    pub miscoverage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBreakDiagnostics {
    pub detected: bool,
    pub break_index: Option<usize>,
    pub break_score: f64,
    pub segment_mean_shift: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ParallelStructuralBreakDiagnostics {
    signal: StructuralBreakDiagnostics,
    residual: StructuralBreakDiagnostics,
    rolling_ic: StructuralBreakDiagnostics,
    verdict: StructuralBreakDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorBacktestResult {
    pub factor_name: String,
    pub metrics: BacktestMetrics,
    pub equity_curve: Vec<f64>,
    pub trades: Vec<FactorTrade>,
    pub windows: Vec<WalkForwardWindow>,
    pub ranking: FactorIC,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BacktestResult {
    pub factor_results: Vec<FactorBacktestResult>,
    pub rankings: Vec<FactorIC>,
    pub scorecards: Vec<PersistedFactorRanking>,
    pub iteration_queue: Vec<FactorIterationPrompt>,
    pub feedback_records_generated: usize,
    pub feedback_records_applied: usize,
    pub feedback_history_summary: FeedbackHistorySummary,
    pub agent_prompts: AgentPromptPack,
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
    pub agent_action_plan: AgentActionPlan,
    pub workflow_state: WorkflowState,
    pub agent_context_bundle: AgentContextBundle,
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    pub recommended_commands: CommandRecommendations,
    pub recommended_next_command: String,
    pub artifact_action_summary: Vec<String>,
    pub artifact_decision_summary: crate::state::ArtifactDecisionSummary,
    pub artifact_decision_section: crate::state::ArtifactDecisionSection,
    pub trade_outcome_deltas: Vec<ProbabilityDiff>,
    pub final_trade_outcome_cpt: BTreeMap<String, BTreeMap<String, f64>>,
    pub workflow_snapshot: crate::state::WorkflowSnapshot,
    pub aggregate_return: f64,
    pub best_factor: Option<String>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
}

pub struct FactorBacktestEngine {
    factor_engine: FactorEngine,
}

impl FactorBacktestEngine {
    pub fn new(factor_engine: FactorEngine) -> Self {
        Self { factor_engine }
    }

    pub fn run<'a>(
        &self,
        candles: &[Candle],
        context: &FactorContext<'a>,
        learning_state: Option<&LearningState>,
        config: &BacktestConfig,
    ) -> Result<BacktestResult> {
        if candles.len() < 3 {
            bail!("need at least 3 candles for factor backtest");
        }

        let engine_output = self.factor_engine.run(candles, context, learning_state)?;
        let forward_returns = forward_returns(candles, 1);
        let mut factor_results = Vec::new();
        let mut rankings = Vec::new();

        for series in engine_output.factor_series {
            let factor_values = series
                .signals
                .iter()
                .take(forward_returns.len())
                .map(|signal| signal.value)
                .collect::<Vec<_>>();
            let rolling_ic = ICCalculator::rolling_ic(
                &factor_values,
                &forward_returns,
                config.test_bars.clamp(5, factor_values.len().max(5)),
            );
            let (mean_ic, std_ic, ir) = ICCalculator::ir(&rolling_ic);

            let (trades, equity_curve, windows) = walk_forward_backtest(&series, candles, config);
            let metrics =
                metrics_from_trades(config.initial_capital, &equity_curve, &trades, &windows);
            let regime_scores = regime_scores(&trades);
            let stability = if windows.is_empty() {
                if metrics.total_return > 0.0 {
                    1.0
                } else {
                    0.0
                }
            } else {
                windows
                    .iter()
                    .filter(|window| window.metrics.total_return > 0.0)
                    .count() as f64
                    / windows.len() as f64
            };
            let regime = best_regime(&regime_scores);

            let mut ranking = FactorIC {
                factor_name: series.name.clone(),
                regime,
                ic_values: rolling_ic,
                mean_ic,
                std_ic,
                ir,
                weight: 0.0,
                backtest_return: metrics.total_return,
                sharpe: metrics.sharpe,
                stability,
                win_rate: metrics.win_rate,
                profit_factor: metrics.profit_factor,
                trade_count: metrics.trade_count,
                regime_scores,
            };
            let mut persisted_preview = PersistedFactorRanking::from(&ranking);
            persisted_preview.conformal_coverage_1sigma = metrics.conformal_coverage_1sigma;
            persisted_preview.conformal_miscoverage_1sigma = metrics.conformal_miscoverage_1sigma;
            persisted_preview.mean_prediction_interval_half_width =
                metrics.mean_prediction_interval_half_width;
            persisted_preview.worst_window_miscoverage = metrics.worst_window_miscoverage;
            persisted_preview.regime_break_penalty = metrics.regime_break_penalty;
            persisted_preview.refresh_scorecard();
            ranking.weight = persisted_preview.composite_score.max(0.0);
            rankings.push(ranking.clone());
            factor_results.push(FactorBacktestResult {
                factor_name: series.name.clone(),
                metrics,
                equity_curve,
                trades,
                windows,
                ranking,
            });
        }

        WeightUpdater::update_weights(&mut rankings);
        let mut scorecards = rankings
            .iter()
            .map(|ranking| {
                let mut scorecard = PersistedFactorRanking::from(ranking);
                scorecard.weight = ranking.weight;
                scorecard.refresh_scorecard();
                scorecard
            })
            .collect::<Vec<_>>();
        scorecards.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for result in &mut factor_results {
            if let Some(ranking) = rankings
                .iter()
                .find(|ranking| ranking.factor_name == result.factor_name)
            {
                result.ranking.weight = ranking.weight;
            }
        }

        let aggregate_return = if factor_results.is_empty() {
            0.0
        } else {
            factor_results
                .iter()
                .map(|result| result.metrics.total_return)
                .sum::<f64>()
                / factor_results.len() as f64
        };
        let aggregate_return = sanitize_non_finite_metric(aggregate_return);
        let best_factor = rankings
            .iter()
            .max_by(|a, b| {
                PersistedFactorRanking::from(*a)
                    .composite_score
                    .partial_cmp(&PersistedFactorRanking::from(*b).composite_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|ranking| ranking.factor_name.clone());

        Ok(BacktestResult {
            factor_results,
            rankings,
            iteration_queue: scorecards
                .iter()
                .map(FactorIterationPrompt::from)
                .filter(|item| item.iteration_action != "keep" || item.replacement_candidate)
                .collect(),
            scorecards,
            feedback_records_generated: 0,
            feedback_records_applied: 0,
            feedback_history_summary: FeedbackHistorySummary::default(),
            agent_prompts: AgentPromptPack::default(),
            provenance: RunProvenance::default(),
            decision_thresholds: DecisionThresholds::default(),
            dataset_comparability: DatasetComparability::default(),
            promotion_decision: PromotionDecision::default(),
            rollback_recommendation: RollbackRecommendation::default(),
            factor_family_decisions: Vec::new(),
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
            trade_outcome_deltas: Vec::new(),
            final_trade_outcome_cpt: BTreeMap::new(),
            workflow_snapshot: crate::state::WorkflowSnapshot::default(),
            aggregate_return,
            best_factor,
            multi_timeframe_summary: Vec::new(),
        })
    }
}

fn sanitize_non_finite_metric(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn empirical_quantile(mut values: Vec<f64>, quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index =
        ((values.len().saturating_sub(1)) as f64 * quantile.clamp(0.0, 1.0)).round() as usize;
    values[index.min(values.len().saturating_sub(1))]
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn realized_window_return(trades: &[FactorTrade]) -> f64 {
    trades
        .iter()
        .fold(1.0, |equity, trade| equity * (1.0 + trade.pnl))
        - 1.0
}

fn structural_break_diagnostics(realized_returns: &[f64]) -> StructuralBreakDiagnostics {
    if realized_returns.len() < 4 {
        return StructuralBreakDiagnostics::default();
    }

    let mut best = StructuralBreakDiagnostics::default();
    for split in 2..=realized_returns.len().saturating_sub(2) {
        let left = &realized_returns[..split];
        let right = &realized_returns[split..];
        let left_mean = mean(left);
        let right_mean = mean(right);
        let shift = (left_mean - right_mean).abs();
        let left_var = mean(
            &left
                .iter()
                .map(|value| (value - left_mean).powi(2))
                .collect::<Vec<_>>(),
        );
        let right_var = mean(
            &right
                .iter()
                .map(|value| (value - right_mean).powi(2))
                .collect::<Vec<_>>(),
        );
        let pooled_scale = (left_var + right_var).sqrt().max(1e-6);
        let score = shift / pooled_scale;
        if score > best.break_score {
            best = StructuralBreakDiagnostics {
                detected: score >= 1.5,
                break_index: Some(split),
                break_score: score,
                segment_mean_shift: shift,
            };
        }
    }
    best
}

fn aggregate_structural_breaks(
    signal: StructuralBreakDiagnostics,
    residual: StructuralBreakDiagnostics,
    rolling_ic: StructuralBreakDiagnostics,
) -> StructuralBreakDiagnostics {
    let mut candidates = [signal.clone(), residual.clone(), rolling_ic.clone()];
    candidates.sort_by(|left, right| {
        right
            .break_score
            .partial_cmp(&left.break_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut verdict = candidates.first().cloned().unwrap_or_default();
    let detected_votes = [signal.detected, residual.detected, rolling_ic.detected]
        .into_iter()
        .filter(|detected| *detected)
        .count();
    verdict.detected = detected_votes >= 2 || verdict.detected;
    verdict.segment_mean_shift = [
        signal.segment_mean_shift,
        residual.segment_mean_shift,
        rolling_ic.segment_mean_shift,
    ]
    .into_iter()
    .fold(0.0, f64::max);
    verdict
}

fn parallel_structural_break_diagnostics(
    signal_values: &[f64],
    realized_returns: &[f64],
    rolling_ic: &[f64],
) -> ParallelStructuralBreakDiagnostics {
    let signal = structural_break_diagnostics(signal_values);
    let residual_series = signal_values
        .iter()
        .zip(realized_returns.iter())
        .map(|(signal_value, realized_return)| realized_return - signal_value)
        .collect::<Vec<_>>();
    let residual = structural_break_diagnostics(&residual_series);
    let rolling_ic = structural_break_diagnostics(rolling_ic);
    let verdict = aggregate_structural_breaks(signal.clone(), residual.clone(), rolling_ic.clone());

    ParallelStructuralBreakDiagnostics {
        signal,
        residual,
        rolling_ic,
        verdict,
    }
}

fn conformal_diagnostics(
    predicted_return: f64,
    calibration_trade_returns: &[f64],
    realized_return: f64,
) -> WindowConformalDiagnostics {
    let nonconformity = calibration_trade_returns
        .iter()
        .map(|ret| (ret - predicted_return).abs())
        .collect::<Vec<_>>();
    let interval_half_width = empirical_quantile(nonconformity, 0.9);
    let interval_lower = predicted_return - interval_half_width;
    let interval_upper = predicted_return + interval_half_width;
    let covered = realized_return >= interval_lower && realized_return <= interval_upper;

    WindowConformalDiagnostics {
        predicted_return,
        realized_return,
        interval_lower,
        interval_upper,
        interval_half_width,
        covered,
        miscoverage: if covered { 0.0 } else { 1.0 },
    }
}

fn walk_forward_backtest(
    series: &crate::factor_lab::factor_definition::FactorSeries,
    candles: &[Candle],
    config: &BacktestConfig,
) -> (Vec<FactorTrade>, Vec<f64>, Vec<WalkForwardWindow>) {
    if candles.len() < 2 {
        return (Vec::new(), vec![config.initial_capital], Vec::new());
    }

    let mut all_trades = Vec::new();
    let mut equity_curve = vec![config.initial_capital];
    let mut windows = Vec::new();
    let mut equity = config.initial_capital;
    let mut start = 0usize;

    while start + config.train_bars < candles.len().saturating_sub(1) {
        let test_start = start + config.train_bars;
        let test_end = (test_start + config.test_bars).min(candles.len().saturating_sub(1));
        if test_start >= test_end {
            break;
        }

        let factor_values = series
            .signals
            .iter()
            .skip(start)
            .take(
                config
                    .train_bars
                    .min(series.signals.len().saturating_sub(start + 1)),
            )
            .map(|signal| signal.value)
            .collect::<Vec<_>>();
        let train_returns = forward_returns(&candles[start..=test_start], 1);
        let ic = ICCalculator::calculate(
            &factor_values[..factor_values.len().min(train_returns.len())],
            &train_returns[..factor_values.len().min(train_returns.len())],
        );
        let rolling_train_ic = ICCalculator::rolling_ic(
            &factor_values[..factor_values.len().min(train_returns.len())],
            &train_returns[..factor_values.len().min(train_returns.len())],
            factor_values.len().min(train_returns.len()).max(2),
        );
        let (_, _, ir) = ICCalculator::ir(&rolling_train_ic);
        let predicted_return = mean(&train_returns);
        let calibration_trade_returns = backtest_range(
            series,
            candles,
            start.max(1),
            test_start,
            config,
            config.initial_capital,
        )
        .trades
        .into_iter()
        .map(|trade| trade.pnl)
        .collect::<Vec<_>>();

        let window = backtest_range(series, candles, test_start, test_end, config, equity);
        let realized_return = realized_window_return(&window.trades);
        let conformal = conformal_diagnostics(
            predicted_return,
            &calibration_trade_returns,
            realized_return,
        );
        let test_signal_values = series
            .signals
            .iter()
            .skip(test_start.saturating_sub(1))
            .take(test_end.saturating_sub(test_start).max(1))
            .map(|signal| signal.value)
            .collect::<Vec<_>>();
        let test_returns = forward_returns(&candles[test_start.saturating_sub(1)..=test_end], 1);
        let pair_len = test_signal_values.len().min(test_returns.len());
        let rolling_ic_window = ICCalculator::rolling_ic(
            &test_signal_values[..pair_len],
            &test_returns[..pair_len],
            pair_len.clamp(2, 8),
        );
        let structural_break = parallel_structural_break_diagnostics(
            &test_signal_values[..pair_len],
            &test_returns[..pair_len],
            &rolling_ic_window,
        )
        .verdict;
        equity = *window.equity_curve.last().unwrap_or(&equity);
        if window.equity_curve.len() > 1 {
            equity_curve.extend(window.equity_curve.iter().copied().skip(1));
        }
        all_trades.extend(window.trades.clone());
        windows.push(WalkForwardWindow {
            start_index: test_start,
            end_index: test_end,
            metrics: metrics_from_trades(
                config.initial_capital,
                &window.equity_curve,
                &window.trades,
                &[],
            ),
            ic,
            ir,
            conformal,
            structural_break,
        });

        let step = config.step_bars.max(1);
        start = start.saturating_add(step);
    }

    if windows.is_empty() {
        let window = backtest_range(
            series,
            candles,
            1,
            candles.len().saturating_sub(1),
            config,
            equity,
        );
        if window.equity_curve.len() > 1 {
            equity_curve.extend(window.equity_curve.iter().copied().skip(1));
        }
        all_trades.extend(window.trades.clone());
        windows.push(WalkForwardWindow {
            start_index: 1,
            end_index: candles.len().saturating_sub(1),
            metrics: metrics_from_trades(config.initial_capital, &equity_curve, &all_trades, &[]),
            ic: 0.0,
            ir: 0.0,
            conformal: conformal_diagnostics(0.0, &[], realized_window_return(&all_trades)),
            structural_break: parallel_structural_break_diagnostics(
                &[
                    0.0,
                    realized_window_return(&all_trades),
                    realized_window_return(&all_trades),
                    0.0,
                ],
                &[
                    realized_window_return(&all_trades),
                    0.0,
                    realized_window_return(&all_trades),
                    0.0,
                ],
                &[0.0, 0.0, 0.0, 0.0],
            )
            .verdict,
        });
    }

    (all_trades, equity_curve, windows)
}

#[derive(Debug)]
struct WindowBacktest {
    equity_curve: Vec<f64>,
    trades: Vec<FactorTrade>,
}

fn backtest_range(
    series: &crate::factor_lab::factor_definition::FactorSeries,
    candles: &[Candle],
    start: usize,
    end: usize,
    config: &BacktestConfig,
    starting_equity: f64,
) -> WindowBacktest {
    let mut equity = starting_equity;
    let mut equity_curve = vec![equity];
    let mut trades = Vec::new();
    let mut index = start.max(1);

    while index <= end && index < candles.len() {
        let signal = &series.signals[index - 1];
        let direction_allowed = match signal.direction {
            Direction::Bull => config.allow_long,
            Direction::Bear => config.allow_short,
            Direction::Neutral => false,
        };

        if !direction_allowed || signal.confidence < config.signal_threshold {
            index += 1;
            continue;
        }

        let direction = signal.direction;
        let entry_price =
            adjusted_entry_price(candles[index].open, direction, config.slippage_rate);
        let stop_loss = stop_price(entry_price, direction, config.stop_loss_pct);
        let take_profit = take_profit_price(entry_price, direction, config.take_profit_pct);
        let exit_limit = (index + config.max_hold_bars.max(1)).min(end);
        let (exit_index, exit_price) = find_exit(
            candles,
            index,
            exit_limit,
            direction,
            stop_loss,
            take_profit,
            config.slippage_rate,
        );
        let size_fraction = (config.position_size * signal.confidence).clamp(0.01, 1.0);
        let signed_return = match direction {
            Direction::Bull => (exit_price - entry_price) / entry_price.max(f64::EPSILON),
            Direction::Bear => (entry_price - exit_price) / entry_price.max(f64::EPSILON),
            Direction::Neutral => 0.0,
        };
        let net_return = signed_return * size_fraction - config.fee_rate * size_fraction * 2.0;
        equity *= 1.0 + net_return;
        equity_curve.push(equity);
        trades.push(FactorTrade {
            factor_name: series.name.clone(),
            entry_time: candles[index].timestamp,
            exit_time: candles[exit_index].timestamp,
            direction,
            entry_price,
            exit_price,
            pnl: net_return,
            holding_bars: exit_index.saturating_sub(index),
            regime_at_entry: infer_regime(candles, index, config),
            signal_value: signal.value,
            signal_confidence: signal.confidence,
            signal_weight: signal.weight,
        });
        index = exit_index.saturating_add(1);
    }

    WindowBacktest {
        equity_curve,
        trades,
    }
}

fn adjusted_entry_price(price: f64, direction: Direction, slippage_rate: f64) -> f64 {
    match direction {
        Direction::Bull => price * (1.0 + slippage_rate),
        Direction::Bear => price * (1.0 - slippage_rate),
        Direction::Neutral => price,
    }
}

fn adjusted_exit_price(price: f64, direction: Direction, slippage_rate: f64) -> f64 {
    match direction {
        Direction::Bull => price * (1.0 - slippage_rate),
        Direction::Bear => price * (1.0 + slippage_rate),
        Direction::Neutral => price,
    }
}

fn stop_price(entry: f64, direction: Direction, stop_loss_pct: f64) -> f64 {
    match direction {
        Direction::Bull => entry * (1.0 - stop_loss_pct),
        Direction::Bear => entry * (1.0 + stop_loss_pct),
        Direction::Neutral => entry,
    }
}

fn take_profit_price(entry: f64, direction: Direction, take_profit_pct: f64) -> f64 {
    match direction {
        Direction::Bull => entry * (1.0 + take_profit_pct),
        Direction::Bear => entry * (1.0 - take_profit_pct),
        Direction::Neutral => entry,
    }
}

fn find_exit(
    candles: &[Candle],
    entry_index: usize,
    exit_limit: usize,
    direction: Direction,
    stop_loss: f64,
    take_profit: f64,
    slippage_rate: f64,
) -> (usize, f64) {
    for (index, candle) in candles
        .iter()
        .enumerate()
        .skip(entry_index)
        .take(exit_limit - entry_index + 1)
    {
        match direction {
            Direction::Bull => {
                if candle.low <= stop_loss {
                    return (
                        index,
                        adjusted_exit_price(stop_loss, direction, slippage_rate),
                    );
                }
                if candle.high >= take_profit {
                    return (
                        index,
                        adjusted_exit_price(take_profit, direction, slippage_rate),
                    );
                }
            }
            Direction::Bear => {
                if candle.high >= stop_loss {
                    return (
                        index,
                        adjusted_exit_price(stop_loss, direction, slippage_rate),
                    );
                }
                if candle.low <= take_profit {
                    return (
                        index,
                        adjusted_exit_price(take_profit, direction, slippage_rate),
                    );
                }
            }
            Direction::Neutral => {}
        }
    }

    (
        exit_limit,
        adjusted_exit_price(candles[exit_limit].close, direction, slippage_rate),
    )
}

fn forward_returns(candles: &[Candle], horizon: usize) -> Vec<f64> {
    if candles.len() <= horizon {
        return Vec::new();
    }

    (0..candles.len() - horizon)
        .map(|index| {
            let start = candles[index].close;
            if start.abs() <= f64::EPSILON {
                0.0
            } else {
                (candles[index + horizon].close - start) / start
            }
        })
        .collect()
}

type ConformalMetricsSummary = (
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    Option<usize>,
    bool,
    f64,
    Option<usize>,
    bool,
    f64,
    Option<usize>,
    bool,
    f64,
    Option<usize>,
    bool,
);

fn summarize_conformal_metrics(windows: &[WalkForwardWindow]) -> ConformalMetricsSummary {
    if windows.is_empty() {
        return (
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, None, false, 0.0, None, false, 0.0, None, false, 0.0,
            None, false,
        );
    }

    let coverage = windows
        .iter()
        .filter(|window| window.conformal.covered)
        .count() as f64
        / windows.len() as f64;
    let mean_half_width = windows
        .iter()
        .map(|window| window.conformal.interval_half_width)
        .sum::<f64>()
        / windows.len() as f64;
    let worst_miscoverage = windows
        .iter()
        .map(|window| window.conformal.miscoverage)
        .fold(0.0, f64::max);
    let signal_series = windows
        .iter()
        .map(|window| window.conformal.predicted_return)
        .collect::<Vec<_>>();
    let realized_returns = windows
        .iter()
        .map(|window| window.conformal.realized_return)
        .collect::<Vec<_>>();
    let rolling_ic_series = windows.iter().map(|window| window.ic).collect::<Vec<_>>();
    let parallel_breaks = parallel_structural_break_diagnostics(
        &signal_series,
        &realized_returns,
        &rolling_ic_series,
    );
    let structural_break = windows
        .iter()
        .map(|window| window.structural_break.clone())
        .max_by(|left, right| {
            left.break_score
                .partial_cmp(&right.break_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|window_break| {
            aggregate_structural_breaks(
                parallel_breaks.signal.clone(),
                parallel_breaks.residual.clone(),
                aggregate_structural_breaks(
                    window_break,
                    parallel_breaks.rolling_ic.clone(),
                    parallel_breaks.verdict.clone(),
                ),
            )
        })
        .unwrap_or_else(|| parallel_breaks.verdict.clone());
    let regime_break_penalty = structural_break.segment_mean_shift;

    (
        coverage,
        1.0 - coverage,
        mean_half_width,
        worst_miscoverage,
        regime_break_penalty,
        structural_break.break_score,
        structural_break.break_index,
        structural_break.detected,
        parallel_breaks.signal.break_score,
        parallel_breaks.signal.break_index,
        parallel_breaks.signal.detected,
        parallel_breaks.residual.break_score,
        parallel_breaks.residual.break_index,
        parallel_breaks.residual.detected,
        parallel_breaks.rolling_ic.break_score,
        parallel_breaks.rolling_ic.break_index,
        parallel_breaks.rolling_ic.detected,
    )
}

fn metrics_from_trades(
    starting_equity: f64,
    equity_curve: &[f64],
    trades: &[FactorTrade],
    windows: &[WalkForwardWindow],
) -> BacktestMetrics {
    let total_return = equity_curve
        .last()
        .copied()
        .map(|equity| {
            if starting_equity.abs() <= f64::EPSILON {
                0.0
            } else {
                equity / starting_equity - 1.0
            }
        })
        .unwrap_or(0.0);
    let trade_returns = trades.iter().map(|trade| trade.pnl).collect::<Vec<_>>();
    let sharpe = sharpe(&trade_returns);
    let max_drawdown = max_drawdown(equity_curve);
    let win_rate = if trades.is_empty() {
        0.0
    } else {
        trades.iter().filter(|trade| trade.pnl > 0.0).count() as f64 / trades.len() as f64
    };
    let gross_profit: f64 = trades
        .iter()
        .filter(|trade| trade.pnl > 0.0)
        .map(|trade| trade.pnl)
        .sum();
    let gross_loss: f64 = trades
        .iter()
        .filter(|trade| trade.pnl < 0.0)
        .map(|trade| trade.pnl.abs())
        .sum();
    let profit_factor = if gross_loss > 0.0 {
        gross_profit / gross_loss
    } else if gross_profit > 0.0 {
        gross_profit
    } else {
        0.0
    };
    let (
        conformal_coverage_1sigma,
        conformal_miscoverage_1sigma,
        mean_prediction_interval_half_width,
        worst_window_miscoverage,
        regime_break_penalty,
        structural_break_score,
        structural_break_index,
        structural_break_detected,
        signal_structural_break_score,
        signal_structural_break_index,
        signal_structural_break_detected,
        residual_structural_break_score,
        residual_structural_break_index,
        residual_structural_break_detected,
        rolling_ic_structural_break_score,
        rolling_ic_structural_break_index,
        rolling_ic_structural_break_detected,
    ) = summarize_conformal_metrics(windows);

    BacktestMetrics {
        total_return,
        sharpe,
        max_drawdown,
        win_rate,
        profit_factor,
        trade_count: trades.len(),
        conformal_coverage_1sigma,
        conformal_miscoverage_1sigma,
        mean_prediction_interval_half_width,
        worst_window_miscoverage,
        regime_break_penalty,
        structural_break_score,
        structural_break_index,
        structural_break_detected,
        signal_structural_break_score,
        signal_structural_break_index,
        signal_structural_break_detected,
        residual_structural_break_score,
        residual_structural_break_index,
        residual_structural_break_detected,
        rolling_ic_structural_break_score,
        rolling_ic_structural_break_index,
        rolling_ic_structural_break_detected,
    }
}

fn sharpe(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }

    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    let std = variance.sqrt();
    if std > 0.0 {
        mean / std * (252.0_f64).sqrt()
    } else {
        0.0
    }
}

fn max_drawdown(equity_curve: &[f64]) -> f64 {
    if equity_curve.is_empty() {
        return 0.0;
    }
    let mut peak = equity_curve[0];
    let mut max_drawdown: f64 = 0.0;
    for equity in equity_curve.iter().copied() {
        if equity > peak {
            peak = equity;
        }
        if peak > 0.0 {
            max_drawdown = max_drawdown.max((peak - equity) / peak);
        }
    }
    max_drawdown
}

fn regime_scores(trades: &[FactorTrade]) -> HashMap<String, f64> {
    let mut pnl = HashMap::<String, (usize, f64)>::new();
    for trade in trades {
        let key = match trade.regime_at_entry {
            Regime::Accumulation => "accumulation",
            Regime::ManipulationExpansion => "manipulation_expansion",
            Regime::Distribution => "distribution",
        }
        .to_string();
        let entry = pnl.entry(key).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += trade.pnl;
    }
    pnl.into_iter()
        .map(|(key, (count, pnl))| (key, pnl / count.max(1) as f64))
        .collect()
}

fn best_regime(regime_scores: &HashMap<String, f64>) -> Regime {
    regime_scores
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(regime, _)| match regime.as_str() {
            "accumulation" => Regime::Accumulation,
            "distribution" => Regime::Distribution,
            _ => Regime::ManipulationExpansion,
        })
        .unwrap_or(Regime::ManipulationExpansion)
}

fn infer_regime(candles: &[Candle], index: usize, _config: &BacktestConfig) -> Regime {
    let start = index.saturating_sub(20);
    let window = &candles[start..=index];
    let total_move = if window.first().unwrap().close.abs() <= f64::EPSILON {
        0.0
    } else {
        (window.last().unwrap().close - window.first().unwrap().close)
            / window.first().unwrap().close
    };
    let avg_range = window
        .iter()
        .map(|candle| candle.range() / candle.close.max(f64::EPSILON))
        .sum::<f64>()
        / window.len() as f64;

    if avg_range > 0.015 {
        Regime::ManipulationExpansion
    } else if total_move >= 0.0 {
        Regime::Accumulation
    } else {
        Regime::Distribution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_lab::factor_definition::FactorDefinition;
    use crate::factors::FactorRegistry;
    use chrono::{Duration, TimeZone};

    fn candles(count: usize) -> Vec<Candle> {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        (0..count)
            .map(|index| {
                let drift = index as f64 * 0.3;
                Candle {
                    timestamp: start + Duration::minutes(index as i64),
                    open: 100.0 + drift,
                    high: 100.5 + drift,
                    low: 99.5 + drift,
                    close: 100.2 + drift,
                    volume: 1_000.0,
                }
            })
            .collect()
    }

    #[test]
    fn test_factor_backtest_produces_real_trades_and_metrics() {
        let mut registry = FactorRegistry::default();
        registry.set_enabled("volatility_mean_reversion", false);
        registry.set_enabled("structure_ict", false);
        registry.set_enabled("cross_market_smt", false);
        registry.set_enabled("options_hedging", false);
        registry.register(FactorDefinition::trend_momentum());

        let engine = FactorEngine::new(registry);
        let backtest = FactorBacktestEngine::new(engine);
        let result = backtest
            .run(
                &candles(140),
                &FactorContext::default(),
                None,
                &BacktestConfig::default(),
            )
            .unwrap();

        assert!(!result.factor_results.is_empty());
        assert!(result.factor_results[0].metrics.trade_count > 0);
        assert!(!result.factor_results[0].equity_curve.is_empty());
        assert!(result.factor_results[0]
            .windows
            .iter()
            .all(|window| { window.conformal.interval_upper >= window.conformal.interval_lower }));
    }

    #[test]
    fn test_conformal_metrics_capture_window_diagnostics() {
        let windows = vec![
            WalkForwardWindow {
                start_index: 10,
                end_index: 20,
                metrics: BacktestMetrics::default(),
                ic: 0.1,
                ir: 0.2,
                conformal: WindowConformalDiagnostics {
                    predicted_return: 0.01,
                    realized_return: 0.015,
                    interval_lower: -0.01,
                    interval_upper: 0.03,
                    interval_half_width: 0.02,
                    covered: true,
                    miscoverage: 0.0,
                },
                structural_break: StructuralBreakDiagnostics {
                    detected: true,
                    break_index: Some(2),
                    break_score: 1.8,
                    segment_mean_shift: 0.03,
                },
            },
            WalkForwardWindow {
                start_index: 21,
                end_index: 30,
                metrics: BacktestMetrics::default(),
                ic: 0.0,
                ir: 0.0,
                conformal: WindowConformalDiagnostics {
                    predicted_return: 0.02,
                    realized_return: -0.03,
                    interval_lower: 0.0,
                    interval_upper: 0.04,
                    interval_half_width: 0.02,
                    covered: false,
                    miscoverage: 1.0,
                },
                structural_break: StructuralBreakDiagnostics {
                    detected: true,
                    break_index: Some(2),
                    break_score: 2.1,
                    segment_mean_shift: 0.05,
                },
            },
        ];

        let metrics = metrics_from_trades(100_000.0, &[100_000.0, 101_000.0], &[], &windows);
        assert!((metrics.conformal_coverage_1sigma - 0.5).abs() < 1e-9);
        assert!((metrics.conformal_miscoverage_1sigma - 0.5).abs() < 1e-9);
        assert!((metrics.mean_prediction_interval_half_width - 0.02).abs() < 1e-9);
        assert_eq!(metrics.worst_window_miscoverage, 1.0);
        assert!(metrics.regime_break_penalty > 0.0);
        assert!(metrics.structural_break_detected);
        assert!(metrics.structural_break_score > 0.0);
        assert!(metrics.structural_break_index.is_some());
        assert!(metrics.signal_structural_break_score >= 0.0);
        assert!(metrics.residual_structural_break_score >= 0.0);
        assert!(metrics.rolling_ic_structural_break_score >= 0.0);
    }
}
