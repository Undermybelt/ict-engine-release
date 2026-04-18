use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ========== 基础数据类型 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Symbol {
    NQ,
    ES,
    YM,
    GC,
    CL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Timeframe {
    M15,
    H1,
    H4,
    D1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Bull,
    Bear,
    Neutral,
}

// ========== Regime ==========
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Regime {
    Accumulation,
    ManipulationExpansion,
    Distribution,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RegimeProbs {
    pub accumulation: f64,
    pub manipulation_expansion: f64,
    pub distribution: f64,
}

impl RegimeProbs {
    pub fn dominant(&self) -> Regime {
        if self.manipulation_expansion >= self.accumulation
            && self.manipulation_expansion >= self.distribution
        {
            Regime::ManipulationExpansion
        } else if self.accumulation >= self.distribution {
            Regime::Accumulation
        } else {
            Regime::Distribution
        }
    }

    pub fn confidence(&self) -> f64 {
        self.manipulation_expansion
            .max(self.accumulation)
            .max(self.distribution)
    }
}

// ========== 决策树 ==========
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CascadeLayer {
    L1,
    L2,
    L3,
    L4,
    L5,
    L6,
    L7,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeStep {
    pub layer: CascadeLayer,
    pub satisfied: bool,
    pub lr: f64,
    pub prior: f64,
    pub posterior: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeResult {
    pub direction: Direction,
    pub stopped_at: Option<CascadeLayer>,
    pub steps: Vec<CascadeStep>,
    pub final_posterior: f64,
}

impl Default for CascadeResult {
    fn default() -> Self {
        Self {
            direction: Direction::Neutral,
            stopped_at: None,
            steps: Vec::new(),
            final_posterior: 0.0,
        }
    }
}

// ========== Beta 分布参数 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaParams {
    pub alpha: f64,
    pub beta: f64,
}

impl BetaParams {
    pub fn posterior_mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    pub fn lr_estimate(&self, base_rate: f64) -> f64 {
        let p = self.posterior_mean();
        if base_rate > 1e-10 && base_rate < 1.0 - 1e-10 {
            (p / base_rate).max(1.0)
        } else {
            1.0
        }
    }

    pub fn update(&mut self, success: bool) {
        if success {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
    }
}

// ========== ICT 结构体 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwingPoint {
    pub index: usize,
    pub price: f64,
    pub sp_type: Direction, // Bull = Swing Low, Bear = Swing High
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBlock {
    pub high: f64,
    pub low: f64,
    pub ob_type: Direction,
    pub bar_index: usize,
    pub tested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevelBand {
    pub top: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdaLifecycleState {
    Active,
    Touched,
    Mitigated,
    Invalidated,
    Inversed,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdaInvalidationRule {
    WickThrough,
    CloseThrough,
    BodyAcceptance,
    FullFill,
    StructureBreak,
    TimeExpiry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdaInverseMode {
    None,
    FlipSameBand,
    FlipNeedsConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdaConceptKind {
    FairValueGap,
    InversionFairValueGap,
    BalancedPriceRange,
    LiquidityPool,
    EqualHighsLows,
    OptimalTradeEntry,
    Ndog,
    Nwog,
    OpenRangeGap,
    SwingFailurePattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdaStateTransition {
    pub state: PdaLifecycleState,
    pub at_bar: usize,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimedPdaState {
    pub concept: PdaConceptKind,
    pub direction: Direction,
    pub band: PriceLevelBand,
    pub anchor_bar: usize,
    pub last_updated_bar: usize,
    pub state: PdaLifecycleState,
    pub invalidation_rule: PdaInvalidationRule,
    pub inverse_mode: PdaInverseMode,
    pub validity_bars: usize,
    pub touch_count: usize,
    pub mitigation_progress: f64,
    pub inverse_confirmed: bool,
    pub transitions: Vec<PdaStateTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FairValueGap {
    pub top: f64,
    pub bottom: f64,
    pub direction: Direction,
    pub start_bar: usize,
    pub filled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPool {
    pub price_level: f64,
    pub sp_count: usize,
    pub pool_type: Direction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquiditySweep {
    pub sweep_bar: usize,
    pub return_bar: usize,
    pub pool_price: f64,
    pub sweep_direction: Direction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectionBlock {
    pub bar_index: usize,
    pub direction: Direction,
    pub body_ratio: f64,
    pub range_atr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CISD {
    pub confirm_bar: usize,
    pub direction: Direction,
    pub strength: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureBreak {
    pub bar_index: usize,
    pub break_type: StructureType,
    pub direction: Direction,
    pub level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructureType {
    BOS,
    CHoCH,
}

// ========== 交易计划 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradePlan {
    pub symbol: Symbol,
    pub direction: Direction,
    pub entry: f64,
    pub stop_loss: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub tp3: f64,
    pub risk_reward: f64,
    pub kelly_fraction: f64,
    pub position_size: f64,
    pub regime: Regime,
    pub posterior: f64,
    pub win_probability: f64,
    pub cascade_bull: CascadeResult,
    pub cascade_bear: CascadeResult,
    pub uncertainties: Vec<String>,
}

// ========== 因子 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorIC {
    pub factor_name: String,
    pub regime: Regime,
    pub ic_values: Vec<f64>,
    pub mean_ic: f64,
    pub std_ic: f64,
    pub ir: f64,
    pub weight: f64,
    pub backtest_return: f64,
    pub sharpe: f64,
    pub stability: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub trade_count: usize,
    pub regime_scores: HashMap<String, f64>,
}

// ========== HMM ==========
pub const OBS_DIM: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HMMParams {
    pub n_states: usize,
    pub transition: Vec<Vec<f64>>,
    pub emission_means: Vec<Vec<f64>>,
    pub emission_stds: Vec<Vec<f64>>,
    pub initial_probs: Vec<f64>,
}

impl HMMParams {
    pub fn new_3state(obs_dim: usize) -> Self {
        Self {
            n_states: 3,
            transition: vec![
                vec![0.80, 0.15, 0.05],
                vec![0.10, 0.80, 0.10],
                vec![0.05, 0.15, 0.80],
            ],
            emission_means: vec![vec![0.0; obs_dim]; 3],
            emission_stds: vec![vec![1.0; obs_dim]; 3],
            initial_probs: vec![0.5, 0.3, 0.2],
        }
    }
}

// ========== 卡尔曼 ==========
#[derive(Debug, Clone)]
pub struct KalmanState {
    pub x: ndarray::Array1<f64>, // 状态均值
    pub p: ndarray::Array2<f64>, // 状态协方差
}

#[derive(Debug, Clone)]
pub struct KalmanParams {
    pub f: ndarray::Array2<f64>, // 状态转移矩阵
    pub h: ndarray::Array2<f64>, // 观测矩阵
    pub q: ndarray::Array2<f64>, // 过程噪声协方差
    pub r: ndarray::Array2<f64>, // 观测噪声协方差
}

// ========== 粒子滤波 ==========
#[derive(Debug, Clone)]
pub struct Particle {
    pub state: Vec<f64>,
    pub weight: f64,
}

// ========== 高斯过程 ==========
pub trait Kernel: Send + Sync {
    fn eval(&self, x1: f64, x2: f64) -> f64;
}

#[derive(Debug, Clone)]
pub struct RBFKernel {
    pub length_scale: f64,
    pub variance: f64,
}

#[derive(Debug, Clone)]
pub struct MaternKernel {
    pub length_scale: f64,
    pub variance: f64,
    pub nu: f64, // 0.5, 1.5, 2.5
}

// ========== Hawkes ==========
#[derive(Debug, Clone)]
pub struct HawkesParams {
    pub mu: f64,    // 基础强度
    pub alpha: f64, // 自激系数
    pub beta: f64,  // 衰减率
}

// ========== BVAR ==========
#[derive(Debug, Clone)]
pub struct BVARParams {
    pub n_vars: usize,
    pub n_lags: usize,
    pub coefficients: Vec<Vec<f64>>, // [n_vars x n_vars * n_lags]
    pub sigma: Vec<Vec<f64>>,        // 残差协方差
}

// ========== 交易记录 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub timestamp: DateTime<Utc>,
    pub symbol: Symbol,
    pub direction: Direction,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl: f64,
    #[serde(default)]
    pub exit_reason: Option<String>,
    pub regime_at_entry: Regime,
    pub cascade_max_layer: CascadeLayer,
    pub cascade_direction: Direction,
    pub factor_values: HashMap<String, f64>,
}

// ========== 预测结果 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimesFMPrediction {
    pub point_forecast: Vec<f64>,
    pub quantile_forecast: Vec<Vec<f64>>, // [horizon x 10 quantiles]
    pub symbol: String,
    pub horizon: usize,
}

// ========== 分析输出 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOutput {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub regime: RegimeProbs,
    pub cascade_bull: CascadeResult,
    pub cascade_bear: CascadeResult,
    pub signals: SignalSummary,
    pub ict_structures: ICTStructureSummary,
    pub trade_plan: Option<TradePlan>,
    pub prediction: Option<TimesFMPrediction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSummary {
    pub kalman_trend: Direction,
    pub implied_vol: f64,
    pub gp_trend: Direction,
    pub hawkes_sweeps: usize,
    pub smt_divergence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ICTStructureSummary {
    pub pda_bull_count: usize,
    pub liquidity_sweeps: usize,
    pub fvgs_open: usize,
    pub order_blocks_nearby: usize,
    pub cisd_ltf_confirmed: bool,
    pub cisd_htf_confirmed: bool,
    pub rb_pinbar_detected: bool,
    pub timed_pda_states: Vec<TimedPdaState>,
}

pub fn normalize_direction_label(input: &str) -> Direction {
    match input.trim().to_ascii_lowercase().as_str() {
        "bull" | "long" | "buy" => Direction::Bull,
        "bear" | "short" | "sell" => Direction::Bear,
        _ => Direction::Neutral,
    }
}

pub fn normalize_regime_label(input: &str) -> Regime {
    match input.trim().to_ascii_lowercase().as_str() {
        "accumulation" | "accum" => Regime::Accumulation,
        "distribution" | "dist" => Regime::Distribution,
        _ => Regime::ManipulationExpansion,
    }
}

pub fn parse_symbol(symbol: &str) -> Symbol {
    match symbol.to_uppercase().as_str() {
        "NQ" => Symbol::NQ,
        "ES" => Symbol::ES,
        "YM" => Symbol::YM,
        "GC" => Symbol::GC,
        "CL" => Symbol::CL,
        _ => Symbol::NQ,
    }
}
