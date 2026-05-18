use crate::types::{Regime, TradeRecord};

/// Regime-split backtest analysis
pub struct RegimeSplit;

impl RegimeSplit {
    /// Split trades by regime
    pub fn split_by_regime(
        trades: &[TradeRecord],
    ) -> (Vec<&TradeRecord>, Vec<&TradeRecord>, Vec<&TradeRecord>) {
        let mut accumulation = Vec::new();
        let mut manipulation = Vec::new();
        let mut distribution = Vec::new();

        for trade in trades {
            match trade.regime_at_entry {
                Regime::Accumulation => accumulation.push(trade),
                Regime::ManipulationExpansion => manipulation.push(trade),
                Regime::Distribution => distribution.push(trade),
            }
        }

        (accumulation, manipulation, distribution)
    }

    /// Calculate metrics per regime
    pub fn regime_metrics(trades: &[TradeRecord]) -> Vec<(Regime, f64, f64)> {
        let (accum, manip, dist) = Self::split_by_regime(trades);

        vec![
            (
                Regime::Accumulation,
                Self::win_rate(&accum),
                Self::avg_pnl(&accum),
            ),
            (
                Regime::ManipulationExpansion,
                Self::win_rate(&manip),
                Self::avg_pnl(&manip),
            ),
            (
                Regime::Distribution,
                Self::win_rate(&dist),
                Self::avg_pnl(&dist),
            ),
        ]
    }

    fn win_rate(trades: &[&TradeRecord]) -> f64 {
        if trades.is_empty() {
            return 0.0;
        }
        let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
        wins as f64 / trades.len() as f64
    }

    fn avg_pnl(trades: &[&TradeRecord]) -> f64 {
        if trades.is_empty() {
            return 0.0;
        }
        trades.iter().map(|t| t.pnl).sum::<f64>() / trades.len() as f64
    }
}
