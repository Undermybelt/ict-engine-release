use serde::Serialize;

use crate::types::{Direction, RegimeProbs};

#[derive(Debug, Serialize)]
pub struct AnalyzeSections {
    pub price_action: PriceActionSection,
    pub technical_price: TechnicalPriceSection,
    pub smt_correlation: SmtCorrelationSection,
    pub regime_bayesian: RegimeBayesianSection,
    pub multi_timeframe: AnalyzeMultiTimeframeSection,
    pub trade_plan: TradePlanSection,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeMultiTimeframeSection {
    pub probability_role: String,
    pub source_mode: String,
    pub direction_bias: String,
    pub alignment_score: Option<f64>,
    pub entry_alignment_score: Option<f64>,
    pub resonance_label: String,
    pub intervals: Vec<AnalyzeMultiTimeframeInterval>,
    pub summary: Vec<String>,
    pub narrative: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeMultiTimeframeInterval {
    pub interval: String,
    pub bars: usize,
    pub source_detail: String,
}

#[derive(Debug, Serialize)]
pub struct PriceActionSection {
    pub probability_role: String,
    pub structure_bias: Direction,
    pub latest_break: Option<String>,
    pub recent_break_count: usize,
    pub swing_highs: usize,
    pub swing_lows: usize,
    pub bull_expansion: bool,
    pub bear_expansion: bool,
    pub expansion_strength: f64,
    pub liquidity_sweeps_recent: usize,
    pub open_fvgs: usize,
    pub untested_order_blocks: usize,
    pub bullish_cisd: bool,
    pub bearish_cisd: bool,
    pub rejection_block_present: bool,
    pub narrative: String,
}

#[derive(Debug, Serialize)]
pub struct TechnicalPriceSection {
    pub probability_role: String,
    pub last_closed_bar_close: f64,
    pub live_market_price: Option<f64>,
    pub live_spot_price: Option<f64>,
    pub ema20: Option<f64>,
    pub ema50: Option<f64>,
    pub rsi14: Option<f64>,
    pub adx14: Option<f64>,
    pub atr14: Option<f64>,
    pub macd_line: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_histogram: Option<f64>,
    pub bollinger_upper: Option<f64>,
    pub bollinger_middle: Option<f64>,
    pub bollinger_lower: Option<f64>,
    pub bollinger_squeeze: bool,
    pub momentum_5_bar: Option<f64>,
    pub options_hedging: OptionsHedgingSection,
    pub narrative: String,
}

#[derive(Debug, Serialize)]
pub struct OptionsHedgingSection {
    pub probability_role: String,
    pub options_symbol: Option<String>,
    pub put_call_oi_ratio: Option<f64>,
    pub put_call_volume_ratio: Option<f64>,
    pub near_atm_implied_volatility: Option<f64>,
    pub near_atm_delta: Option<f64>,
    pub near_atm_gamma: Option<f64>,
    pub near_atm_vega: Option<f64>,
    pub call_gamma_oi: Option<f64>,
    pub put_gamma_oi: Option<f64>,
    pub gamma_skew: Option<f64>,
    pub hedge_pressure_direction: Option<String>,
    pub hedge_pressure_score: Option<f64>,
    pub long_bias_contribution: Option<f64>,
    pub short_bias_contribution: Option<f64>,
    pub uncertainty_penalty_contribution: Option<f64>,
    pub narrative: String,
}

pub use crate::analyze::smt_correlation_section::SmtCorrelationSection;

#[derive(Debug, Serialize)]
pub struct RegimeBayesianSection {
    pub hmm_state: String,
    pub regime_probs: RegimeProbs,
    pub regime_label: String,
    pub liquidity_label: String,
    pub long_score: f64,
    pub short_score: f64,
    pub win_prob_long: f64,
    pub win_prob_short: f64,
    pub selected_direction: Direction,
    pub evidence_policy: String,
    pub ict_role: String,
}

#[derive(Debug, Serialize)]
pub struct TradePlanSection {
    pub probability_role: String,
    pub actionable: bool,
    pub direction: Direction,
    pub entry: f64,
    pub stop_loss: f64,
    pub take_profits: Vec<f64>,
    pub risk_reward: f64,
    pub posterior: f64,
    pub win_probability: f64,
    pub kelly_fraction: f64,
    pub position_size: f64,
    pub uncertainties: Vec<String>,
    pub narrative: String,
}
