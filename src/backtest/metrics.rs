use crate::types::TradeRecord;

/// Backtest metrics calculator
pub struct Metrics;

impl Metrics {
    /// Calculate Sharpe ratio
    pub fn sharpe(returns: &[f64], risk_free_rate: f64) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }

        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance =
            returns.iter().map(|&r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
        let std = variance.sqrt();

        if std > 0.0 {
            (mean - risk_free_rate) / std * (252.0_f64).sqrt()
        } else {
            0.0
        }
    }

    /// Calculate maximum drawdown
    pub fn max_drawdown(equity_curve: &[f64]) -> f64 {
        if equity_curve.is_empty() {
            return 0.0;
        }

        let mut max_dd = 0.0;
        let mut peak = equity_curve[0];

        for &equity in equity_curve {
            if equity > peak {
                peak = equity;
            }
            let dd = (peak - equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        max_dd
    }

    /// Calculate win rate
    pub fn win_rate(trades: &[TradeRecord]) -> f64 {
        if trades.is_empty() {
            return 0.0;
        }

        let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
        wins as f64 / trades.len() as f64
    }

    /// Calculate profit factor
    pub fn profit_factor(trades: &[TradeRecord]) -> f64 {
        let gross_profit: f64 = trades.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).sum();
        let gross_loss: f64 = trades
            .iter()
            .filter(|t| t.pnl < 0.0)
            .map(|t| t.pnl.abs())
            .sum();

        if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else {
            0.0
        }
    }
}
