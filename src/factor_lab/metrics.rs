use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub total_return: f64,
    pub sharpe: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub trade_count: usize,
    #[serde(default)]
    pub conformal_coverage_1sigma: f64,
    #[serde(default)]
    pub conformal_miscoverage_1sigma: f64,
    #[serde(default)]
    pub mean_prediction_interval_half_width: f64,
    #[serde(default)]
    pub worst_window_miscoverage: f64,
    #[serde(default)]
    pub regime_break_penalty: f64,
    #[serde(default)]
    pub structural_break_score: f64,
    #[serde(default)]
    pub structural_break_index: Option<usize>,
    #[serde(default)]
    pub structural_break_detected: bool,
    #[serde(default)]
    pub signal_structural_break_score: f64,
    #[serde(default)]
    pub signal_structural_break_index: Option<usize>,
    #[serde(default)]
    pub signal_structural_break_detected: bool,
    #[serde(default)]
    pub residual_structural_break_score: f64,
    #[serde(default)]
    pub residual_structural_break_index: Option<usize>,
    #[serde(default)]
    pub residual_structural_break_detected: bool,
    #[serde(default)]
    pub rolling_ic_structural_break_score: f64,
    #[serde(default)]
    pub rolling_ic_structural_break_index: Option<usize>,
    #[serde(default)]
    pub rolling_ic_structural_break_detected: bool,
}
