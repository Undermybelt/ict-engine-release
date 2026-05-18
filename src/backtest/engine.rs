use crate::types::{Candle, Direction, TradePlan, TradeRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguousBarPolicy {
    FavorStopLoss,
    FavorTakeProfit,
}

#[derive(Debug, Clone, Copy)]
pub struct ExecutionRealismConfig {
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub fee_bps: f64,
    pub ambiguous_bar_policy: AmbiguousBarPolicy,
}

impl Default for ExecutionRealismConfig {
    fn default() -> Self {
        Self {
            spread_bps: 0.0,
            slippage_bps: 0.0,
            fee_bps: 0.0,
            ambiguous_bar_policy: AmbiguousBarPolicy::FavorStopLoss,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimulatedTrade {
    pub entry_index: usize,
    pub exit_index: usize,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl: f64,
    pub exit_reason: SimulatedExitReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulatedExitReason {
    StopLoss,
    TakeProfit,
    TimeExpiry,
}

/// Backtest engine
pub struct BacktestEngine {
    pub trades: Vec<TradeRecord>,
}

impl Default for BacktestEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BacktestEngine {
    pub fn new() -> Self {
        Self { trades: Vec::new() }
    }

    /// Run backtest
    pub fn run(&mut self, candles: &[Candle], strategy: &dyn Fn(&[Candle]) -> Vec<TradeRecord>) {
        self.trades = strategy(candles);
    }

    /// Get trades
    pub fn get_trades(&self) -> &[TradeRecord] {
        &self.trades
    }

    pub fn simulate_trade(
        candles: &[Candle],
        signal_index: usize,
        plan: &TradePlan,
        hold_bars: usize,
    ) -> Option<SimulatedTrade> {
        Self::simulate_trade_with_realism(
            candles,
            signal_index,
            plan,
            hold_bars,
            &ExecutionRealismConfig::default(),
        )
    }

    pub fn simulate_trade_with_realism(
        candles: &[Candle],
        signal_index: usize,
        plan: &TradePlan,
        hold_bars: usize,
        realism: &ExecutionRealismConfig,
    ) -> Option<SimulatedTrade> {
        if plan.direction == Direction::Neutral
            || hold_bars == 0
            || signal_index + 1 >= candles.len()
        {
            return None;
        }

        let search_end = (signal_index + hold_bars).min(candles.len().saturating_sub(1));
        let raw_entry_index = (signal_index + 1..=search_end)
            .find(|&idx| price_touches(&candles[idx], plan.entry))?;
        let adjusted_entry_price =
            apply_entry_execution_adjustments(plan.entry, plan.direction, realism);

        let exit_end = (raw_entry_index + hold_bars).min(candles.len().saturating_sub(1));
        for (exit_index, candle) in candles
            .iter()
            .enumerate()
            .take(exit_end + 1)
            .skip(raw_entry_index)
        {
            match plan.direction {
                Direction::Bull => {
                    let stop_hit = candle.low <= plan.stop_loss;
                    let tp_hit = candle.high >= plan.tp1;
                    if stop_hit && tp_hit {
                        let (exit_price, exit_reason) = match realism.ambiguous_bar_policy {
                            AmbiguousBarPolicy::FavorStopLoss => (
                                apply_exit_execution_adjustments(
                                    plan.stop_loss,
                                    plan.direction,
                                    SimulatedExitReason::StopLoss,
                                    realism,
                                ),
                                SimulatedExitReason::StopLoss,
                            ),
                            AmbiguousBarPolicy::FavorTakeProfit => (
                                apply_exit_execution_adjustments(
                                    plan.tp1,
                                    plan.direction,
                                    SimulatedExitReason::TakeProfit,
                                    realism,
                                ),
                                SimulatedExitReason::TakeProfit,
                            ),
                        };
                        return Some(Self::build_result(
                            plan,
                            raw_entry_index,
                            exit_index,
                            adjusted_entry_price,
                            exit_price,
                            exit_reason,
                            realism,
                        ));
                    }
                    if stop_hit {
                        return Some(Self::build_result(
                            plan,
                            raw_entry_index,
                            exit_index,
                            adjusted_entry_price,
                            apply_exit_execution_adjustments(
                                plan.stop_loss,
                                plan.direction,
                                SimulatedExitReason::StopLoss,
                                realism,
                            ),
                            SimulatedExitReason::StopLoss,
                            realism,
                        ));
                    }
                    if tp_hit {
                        return Some(Self::build_result(
                            plan,
                            raw_entry_index,
                            exit_index,
                            adjusted_entry_price,
                            apply_exit_execution_adjustments(
                                plan.tp1,
                                plan.direction,
                                SimulatedExitReason::TakeProfit,
                                realism,
                            ),
                            SimulatedExitReason::TakeProfit,
                            realism,
                        ));
                    }
                }
                Direction::Bear => {
                    let stop_hit = candle.high >= plan.stop_loss;
                    let tp_hit = candle.low <= plan.tp1;
                    if stop_hit && tp_hit {
                        let (exit_price, exit_reason) = match realism.ambiguous_bar_policy {
                            AmbiguousBarPolicy::FavorStopLoss => (
                                apply_exit_execution_adjustments(
                                    plan.stop_loss,
                                    plan.direction,
                                    SimulatedExitReason::StopLoss,
                                    realism,
                                ),
                                SimulatedExitReason::StopLoss,
                            ),
                            AmbiguousBarPolicy::FavorTakeProfit => (
                                apply_exit_execution_adjustments(
                                    plan.tp1,
                                    plan.direction,
                                    SimulatedExitReason::TakeProfit,
                                    realism,
                                ),
                                SimulatedExitReason::TakeProfit,
                            ),
                        };
                        return Some(Self::build_result(
                            plan,
                            raw_entry_index,
                            exit_index,
                            adjusted_entry_price,
                            exit_price,
                            exit_reason,
                            realism,
                        ));
                    }
                    if stop_hit {
                        return Some(Self::build_result(
                            plan,
                            raw_entry_index,
                            exit_index,
                            adjusted_entry_price,
                            apply_exit_execution_adjustments(
                                plan.stop_loss,
                                plan.direction,
                                SimulatedExitReason::StopLoss,
                                realism,
                            ),
                            SimulatedExitReason::StopLoss,
                            realism,
                        ));
                    }
                    if tp_hit {
                        return Some(Self::build_result(
                            plan,
                            raw_entry_index,
                            exit_index,
                            adjusted_entry_price,
                            apply_exit_execution_adjustments(
                                plan.tp1,
                                plan.direction,
                                SimulatedExitReason::TakeProfit,
                                realism,
                            ),
                            SimulatedExitReason::TakeProfit,
                            realism,
                        ));
                    }
                }
                Direction::Neutral => return None,
            }
        }

        Some(Self::build_result(
            plan,
            raw_entry_index,
            exit_end,
            adjusted_entry_price,
            apply_exit_execution_adjustments(
                candles[exit_end].close,
                plan.direction,
                SimulatedExitReason::TimeExpiry,
                realism,
            ),
            SimulatedExitReason::TimeExpiry,
            realism,
        ))
    }

    fn build_result(
        plan: &TradePlan,
        entry_index: usize,
        exit_index: usize,
        entry_price: f64,
        exit_price: f64,
        exit_reason: SimulatedExitReason,
        realism: &ExecutionRealismConfig,
    ) -> SimulatedTrade {
        let signed_return = match plan.direction {
            Direction::Bull => (exit_price - entry_price) / entry_price.max(f64::EPSILON),
            Direction::Bear => (entry_price - exit_price) / entry_price.max(f64::EPSILON),
            Direction::Neutral => 0.0,
        };
        let fee_fraction = (realism.fee_bps.max(0.0) / 10_000.0) * 2.0;

        SimulatedTrade {
            entry_index,
            exit_index,
            entry_price,
            exit_price,
            pnl: (signed_return - fee_fraction) * plan.kelly_fraction,
            exit_reason,
        }
    }
}

fn apply_entry_execution_adjustments(
    entry_price: f64,
    direction: Direction,
    realism: &ExecutionRealismConfig,
) -> f64 {
    let half_spread = realism.spread_bps.max(0.0) / 20_000.0;
    let slippage = realism.slippage_bps.max(0.0) / 10_000.0;
    let multiplier = match direction {
        Direction::Bull => 1.0 + half_spread + slippage,
        Direction::Bear => 1.0 - half_spread - slippage,
        Direction::Neutral => 1.0,
    };
    entry_price * multiplier
}

fn apply_exit_execution_adjustments(
    exit_price: f64,
    direction: Direction,
    exit_reason: SimulatedExitReason,
    realism: &ExecutionRealismConfig,
) -> f64 {
    let half_spread = realism.spread_bps.max(0.0) / 20_000.0;
    let slippage = realism.slippage_bps.max(0.0) / 10_000.0;
    let multiplier = match (direction, exit_reason) {
        (Direction::Bull, SimulatedExitReason::TakeProfit) => 1.0 - half_spread - slippage,
        (Direction::Bull, _) => 1.0 - half_spread - slippage,
        (Direction::Bear, SimulatedExitReason::TakeProfit) => 1.0 + half_spread + slippage,
        (Direction::Bear, _) => 1.0 + half_spread + slippage,
        (Direction::Neutral, _) => 1.0,
    };
    exit_price * multiplier
}

fn price_touches(candle: &Candle, price: f64) -> bool {
    candle.low <= price && candle.high >= price
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CascadeResult, Direction, Regime, Symbol, TradePlan};
    use chrono::Utc;

    fn candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open,
            high,
            low,
            close,
            volume: 1_000.0,
        }
    }

    fn plan(direction: Direction) -> TradePlan {
        TradePlan {
            symbol: Symbol::NQ,
            direction,
            entry: 100.0,
            stop_loss: 98.0,
            tp1: 103.0,
            tp2: 104.0,
            tp3: 105.0,
            risk_reward: 1.5,
            kelly_fraction: 0.1,
            position_size: 10.0,
            regime: Regime::ManipulationExpansion,
            posterior: 0.2,
            win_probability: 0.6,
            cascade_bull: CascadeResult {
                direction: Direction::Bull,
                stopped_at: None,
                steps: Vec::new(),
                final_posterior: 0.7,
            },
            cascade_bear: CascadeResult {
                direction: Direction::Bear,
                stopped_at: None,
                steps: Vec::new(),
                final_posterior: 0.3,
            },
            uncertainties: Vec::new(),
        }
    }

    #[test]
    fn test_simulate_trade_hits_take_profit() {
        let candles = vec![
            candle(99.0, 99.5, 98.5, 99.2),
            candle(99.5, 100.5, 99.0, 100.0),
            candle(100.0, 103.5, 99.8, 103.0),
        ];

        let simulated =
            BacktestEngine::simulate_trade(&candles, 0, &plan(Direction::Bull), 2).unwrap();
        assert_eq!(simulated.entry_index, 1);
        assert_eq!(simulated.exit_index, 2);
        assert_eq!(simulated.exit_reason, SimulatedExitReason::TakeProfit);
        assert!(simulated.pnl > 0.0);
    }

    #[test]
    fn test_simulate_trade_requires_entry_touch() {
        let candles = vec![
            candle(99.0, 99.5, 98.5, 99.2),
            candle(104.0, 105.0, 103.5, 104.5),
            candle(104.5, 105.5, 104.0, 105.0),
        ];

        let simulated = BacktestEngine::simulate_trade(&candles, 0, &plan(Direction::Bull), 2);
        assert!(simulated.is_none());
    }

    #[test]
    fn test_simulate_trade_marks_stop_loss_exit_reason() {
        let candles = vec![
            candle(99.0, 99.5, 98.5, 99.2),
            candle(99.5, 100.5, 99.0, 100.0),
            candle(100.0, 100.2, 97.5, 98.0),
        ];

        let simulated =
            BacktestEngine::simulate_trade(&candles, 0, &plan(Direction::Bull), 2).unwrap();
        assert_eq!(simulated.exit_reason, SimulatedExitReason::StopLoss);
        assert!(simulated.pnl < 0.0);
    }

    #[test]
    fn test_simulate_trade_penalizes_pnl_with_realism_costs() {
        let candles = vec![
            candle(99.0, 99.5, 98.5, 99.2),
            candle(99.5, 100.5, 99.0, 100.0),
            candle(100.0, 103.5, 99.8, 103.0),
        ];

        let baseline =
            BacktestEngine::simulate_trade(&candles, 0, &plan(Direction::Bull), 2).unwrap();
        let realism = ExecutionRealismConfig {
            spread_bps: 10.0,
            slippage_bps: 5.0,
            fee_bps: 2.0,
            ambiguous_bar_policy: AmbiguousBarPolicy::FavorStopLoss,
        };
        let stressed = BacktestEngine::simulate_trade_with_realism(
            &candles,
            0,
            &plan(Direction::Bull),
            2,
            &realism,
        )
        .unwrap();

        assert!(stressed.entry_price > baseline.entry_price);
        assert!(stressed.exit_price < baseline.exit_price);
        assert!(stressed.pnl < baseline.pnl);
    }

    #[test]
    fn test_simulate_trade_honors_ambiguous_bar_policy() {
        let candles = vec![
            candle(99.0, 99.5, 98.5, 99.2),
            candle(99.5, 100.5, 99.0, 100.0),
            candle(100.0, 103.5, 97.5, 102.0),
        ];
        let stop_first = ExecutionRealismConfig {
            ambiguous_bar_policy: AmbiguousBarPolicy::FavorStopLoss,
            ..ExecutionRealismConfig::default()
        };
        let tp_first = ExecutionRealismConfig {
            ambiguous_bar_policy: AmbiguousBarPolicy::FavorTakeProfit,
            ..ExecutionRealismConfig::default()
        };

        let stop_trade = BacktestEngine::simulate_trade_with_realism(
            &candles,
            0,
            &plan(Direction::Bull),
            2,
            &stop_first,
        )
        .unwrap();
        let tp_trade = BacktestEngine::simulate_trade_with_realism(
            &candles,
            0,
            &plan(Direction::Bull),
            2,
            &tp_first,
        )
        .unwrap();

        assert_eq!(stop_trade.exit_reason, SimulatedExitReason::StopLoss);
        assert_eq!(tp_trade.exit_reason, SimulatedExitReason::TakeProfit);
        assert!(stop_trade.pnl < tp_trade.pnl);
    }

    #[test]
    fn test_simulate_trade_marks_time_expiry_exit_reason() {
        let candles = vec![
            candle(99.0, 99.5, 98.5, 99.2),
            candle(99.5, 100.5, 99.0, 100.0),
            candle(100.0, 102.0, 99.5, 101.0),
            candle(101.0, 102.5, 100.0, 101.5),
        ];

        let simulated =
            BacktestEngine::simulate_trade(&candles, 0, &plan(Direction::Bull), 2).unwrap();
        assert_eq!(simulated.exit_index, 3);
        assert_eq!(simulated.exit_reason, SimulatedExitReason::TimeExpiry);
        assert!(simulated.pnl > 0.0);
    }
}
