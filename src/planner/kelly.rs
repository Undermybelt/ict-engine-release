/// Kelly Criterion for position sizing
pub struct Kelly;

impl Kelly {
    /// Calculate Kelly fraction
    /// f* = (p * b - q) / b
    /// where p = win probability, b = win/loss ratio, q = 1 - p
    pub fn fraction(win_prob: f64, win_loss_ratio: f64) -> f64 {
        if win_prob < 0.5 || win_loss_ratio <= 0.0 {
            return 0.0;
        }

        let q = 1.0 - win_prob;
        let f = (win_prob * win_loss_ratio - q) / win_loss_ratio;

        f.max(0.0)
    }

    /// Calculate Kelly fraction with safety margin (half-Kelly)
    pub fn safe_fraction(win_prob: f64, win_loss_ratio: f64) -> f64 {
        Self::fraction(win_prob, win_loss_ratio) * 0.5
    }
}
