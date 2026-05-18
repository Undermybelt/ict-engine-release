use crate::types::Direction;

/// Risk management calculations
pub struct Risk;

impl Risk {
    /// Calculate stop loss based on ATR
    pub fn atr_stop_loss(entry: f64, atr: f64, multiplier: f64, direction: Direction) -> f64 {
        match direction {
            Direction::Bull => entry - atr * multiplier,
            Direction::Bear => entry + atr * multiplier,
            Direction::Neutral => entry,
        }
    }

    /// Calculate take profit levels based on ATR
    pub fn atr_take_profits(entry: f64, atr: f64, direction: Direction) -> (f64, f64, f64) {
        match direction {
            Direction::Bull => (
                entry + atr * 1.0, // TP1: 1R
                entry + atr * 2.0, // TP2: 2R
                entry + atr * 3.0, // TP3: 3R
            ),
            Direction::Bear => (entry - atr * 1.0, entry - atr * 2.0, entry - atr * 3.0),
            Direction::Neutral => (entry, entry, entry),
        }
    }

    /// Calculate risk-reward ratio
    pub fn risk_reward(entry: f64, stop_loss: f64, take_profit: f64) -> f64 {
        let risk = (entry - stop_loss).abs();
        let reward = (take_profit - entry).abs();

        if risk > 0.0 {
            reward / risk
        } else {
            0.0
        }
    }

    /// Calculate position size based on risk percentage
    pub fn position_size(
        account_balance: f64,
        risk_percent: f64,
        entry: f64,
        stop_loss: f64,
    ) -> f64 {
        let risk_amount = account_balance * risk_percent / 100.0;
        let risk_per_unit = (entry - stop_loss).abs();

        if risk_per_unit > 0.0 {
            risk_amount / risk_per_unit
        } else {
            0.0
        }
    }
}
