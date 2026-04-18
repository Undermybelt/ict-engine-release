use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::data::realtime::openalice::AuxiliaryMarketEvidence;
use crate::ict::{
    check_bear_expansion_exists, check_bull_expansion_exists, detect_cisd, detect_liquidity_pools,
    detect_liquidity_sweep, detect_order_blocks, detect_structure_breaks, find_swing_highs,
    find_swing_lows, find_unfilled_fvgs, find_untested_obs,
};
use crate::indicators::{
    atr_percent, compute_adx, compute_atr, compute_bollinger, compute_ema, compute_rsi,
    BollingerBands,
};
use crate::smt::{Correlation, Divergence};
use crate::types::{Candle, Direction, Regime};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FactorCategory {
    TrendMomentum,
    VolatilityMeanReversion,
    StructureIct,
    CrossMarketSmt,
    OptionsHedging,
}

impl FactorCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TrendMomentum => "trend_momentum",
            Self::VolatilityMeanReversion => "volatility_mean_reversion",
            Self::StructureIct => "structure_ict",
            Self::CrossMarketSmt => "cross_market_smt",
            Self::OptionsHedging => "options_hedging",
        }
    }

    pub fn is_footprint_context_only(self) -> bool {
        matches!(
            self,
            Self::StructureIct | Self::CrossMarketSmt | Self::OptionsHedging
        )
    }

    pub fn allowed_roles(self) -> &'static [FactorRole] {
        match self {
            Self::TrendMomentum => &[FactorRole::Evidence, FactorRole::OutcomeValidator],
            Self::VolatilityMeanReversion => &[FactorRole::Evidence, FactorRole::SetupClassifier],
            Self::StructureIct | Self::CrossMarketSmt | Self::OptionsHedging => &[
                FactorRole::PriorAdjuster,
                FactorRole::StateTransition,
                FactorRole::SetupClassifier,
                FactorRole::OutcomeValidator,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FactorRole {
    PriorAdjuster,
    StateTransition,
    SetupClassifier,
    Evidence,
    OutcomeValidator,
}

impl FactorRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PriorAdjuster => "prior_adjuster",
            Self::StateTransition => "state_transition",
            Self::SetupClassifier => "setup_classifier",
            Self::Evidence => "evidence",
            Self::OutcomeValidator => "outcome_validator",
        }
    }

    pub fn allowed_for_category(self, category: FactorCategory) -> bool {
        category.allowed_roles().contains(&self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FactorUsagePhase {
    PriorAdjustment,
    HiddenStateTransition,
    SetupClassification,
    Evidence,
    OutcomeValidation,
}

impl FactorUsagePhase {
    pub fn required_role(self) -> FactorRole {
        match self {
            Self::PriorAdjustment => FactorRole::PriorAdjuster,
            Self::HiddenStateTransition => FactorRole::StateTransition,
            Self::SetupClassification => FactorRole::SetupClassifier,
            Self::Evidence => FactorRole::Evidence,
            Self::OutcomeValidation => FactorRole::OutcomeValidator,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FactorContext<'a> {
    pub paired_candles: Option<&'a [Candle]>,
    pub auxiliary: Option<&'a AuxiliaryMarketEvidence>,
    pub regime: Option<Regime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairedMarketQualityReport {
    pub paired_market_quality: String,
    pub aligned_length: usize,
    pub primary_length: usize,
    pub paired_length: usize,
    pub overlap_ratio: f64,
    pub safe_lookback: usize,
    pub status: String,
    pub reason: String,
}

fn paired_market_quality_report(
    primary_length: usize,
    paired_length: usize,
    requested_lookback: usize,
    primary_closes: Option<&[f64]>,
    pair_closes: Option<&[f64]>,
) -> PairedMarketQualityReport {
    let aligned_length = primary_length.min(paired_length);
    let overlap_ratio = if primary_length == 0 {
        0.0
    } else {
        aligned_length as f64 / primary_length as f64
    };
    let safe_lookback = requested_lookback.min(aligned_length.saturating_sub(1));

    let mut status = "valid".to_string();
    let mut paired_market_quality = "strong".to_string();
    let mut reason = "pair_quality_ok".to_string();

    if aligned_length < 32 || overlap_ratio < 0.60 {
        status = "invalid_due_to_pair_quality".to_string();
        paired_market_quality = "poor".to_string();
        reason = if aligned_length < 32 {
            "insufficient_aligned_history".to_string()
        } else {
            "overlap_ratio_below_threshold".to_string()
        };
    } else if aligned_length < 64 || overlap_ratio < 0.80 {
        paired_market_quality = "medium".to_string();
        reason = "limited_pair_overlap".to_string();
    }

    if status == "valid" {
        if let (Some(primary_closes), Some(pair_closes)) = (primary_closes, pair_closes) {
            let primary_returns = close_returns(primary_closes);
            let pair_returns = close_returns(pair_closes);
            let primary_flat = primary_returns.iter().all(|ret| ret.abs() <= 1e-9);
            let pair_flat = pair_returns.iter().all(|ret| ret.abs() <= 1e-9);
            if primary_flat || pair_flat {
                status = "valid_but_flat".to_string();
                paired_market_quality = "flat".to_string();
                reason = if pair_flat {
                    "paired_returns_flat".to_string()
                } else {
                    "primary_returns_flat".to_string()
                };
            }
        }
    }

    PairedMarketQualityReport {
        paired_market_quality,
        aligned_length,
        primary_length,
        paired_length,
        overlap_ratio,
        safe_lookback,
        status,
        reason,
    }
}

fn paired_market_window_quality_report(
    primary_length: usize,
    paired_length: usize,
    primary_closes: &[f64],
    pair_closes: &[f64],
    requested_lookback: usize,
) -> PairedMarketQualityReport {
    let aligned_length = primary_closes.len().min(pair_closes.len());
    let primary_closes = &primary_closes[..aligned_length];
    let pair_closes = &pair_closes[..aligned_length];
    let overlap_ratio = if primary_length == 0 {
        0.0
    } else {
        aligned_length as f64 / primary_length as f64
    };
    let safe_lookback = requested_lookback.min(aligned_length.saturating_sub(1));

    let mut status = "valid".to_string();
    let mut paired_market_quality = "strong".to_string();
    let mut reason = "pair_quality_ok".to_string();

    if aligned_length < 32 || overlap_ratio < 0.60 {
        status = "invalid_due_to_pair_quality".to_string();
        paired_market_quality = "poor".to_string();
        reason = if aligned_length < 32 {
            "insufficient_aligned_history".to_string()
        } else {
            "overlap_ratio_below_threshold".to_string()
        };
    } else if aligned_length < 64 || overlap_ratio < 0.80 {
        paired_market_quality = "medium".to_string();
        reason = "limited_pair_overlap".to_string();
    }

    let primary_returns = close_returns(primary_closes);
    let pair_returns = close_returns(pair_closes);
    let primary_flat = primary_returns.iter().all(|ret| ret.abs() <= 1e-9);
    let pair_flat = pair_returns.iter().all(|ret| ret.abs() <= 1e-9);
    if status == "valid" && (primary_flat || pair_flat) {
        status = "valid_but_flat".to_string();
        paired_market_quality = "flat".to_string();
        reason = if pair_flat {
            "paired_returns_flat".to_string()
        } else {
            "primary_returns_flat".to_string()
        };
    }

    PairedMarketQualityReport {
        paired_market_quality,
        aligned_length,
        primary_length,
        paired_length,
        overlap_ratio,
        safe_lookback,
        status,
        reason,
    }
}

fn paired_market_quality_explanation(report: &PairedMarketQualityReport) -> String {
    format!(
        "status={};quality_tier={};reason={};aligned_length={};primary_length={};paired_length={};overlap_ratio={:.4};safe_lookback={}",
        report.status,
        report.paired_market_quality,
        report.reason,
        report.aligned_length,
        report.primary_length,
        report.paired_length,
        report.overlap_ratio,
        report.safe_lookback
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSignal {
    pub factor_name: String,
    pub category: FactorCategory,
    pub roles: Vec<FactorRole>,
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub direction: Direction,
    pub confidence: f64,
    pub explanation: String,
    pub paired_market_quality_report: Option<PairedMarketQualityReport>,
    pub weight: f64,
    pub posterior_reliability: f64,
    pub regime_multiplier: f64,
    pub regime_adjusted_score: f64,
}

impl Default for FactorSignal {
    fn default() -> Self {
        Self {
            factor_name: String::new(),
            category: FactorCategory::TrendMomentum,
            roles: vec![FactorRole::Evidence],
            timestamp: Utc::now(),
            value: 0.0,
            direction: Direction::Neutral,
            confidence: 0.0,
            explanation: String::new(),
            paired_market_quality_report: None,
            weight: 0.0,
            posterior_reliability: 0.5,
            regime_multiplier: 1.0,
            regime_adjusted_score: 0.0,
        }
    }
}

impl FactorSignal {
    pub fn sanitized_roles(&self) -> Vec<FactorRole> {
        self.roles
            .iter()
            .copied()
            .filter(|role| role.allowed_for_category(self.category))
            .collect()
    }

    pub fn supports_role(&self, role: FactorRole) -> bool {
        self.sanitized_roles().contains(&role)
    }

    pub fn ensure_role(&self, role: FactorRole) -> Result<()> {
        if self.supports_role(role) {
            Ok(())
        } else {
            Err(anyhow!(
                "factor '{}' category '{}' cannot be used as {}",
                self.factor_name,
                self.category.as_str(),
                role.as_str()
            ))
        }
    }

    pub fn ensure_phase(&self, phase: FactorUsagePhase) -> Result<()> {
        self.ensure_role(phase.required_role())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSeries {
    pub name: String,
    pub description: String,
    pub category: FactorCategory,
    pub parameters: BTreeMap<String, f64>,
    pub signals: Vec<FactorSignal>,
}

impl FactorSeries {
    pub fn latest_signal(&self) -> Option<FactorSignal> {
        self.signals.last().cloned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorDefinition {
    pub name: String,
    pub description: String,
    pub category: FactorCategory,
    pub enabled: bool,
    pub parameters: BTreeMap<String, f64>,
}

impl FactorDefinition {
    pub fn trend_momentum() -> Self {
        Self {
            name: "trend_momentum".to_string(),
            description: "EMA slope and RSI persistence as directional evidence".to_string(),
            category: FactorCategory::TrendMomentum,
            enabled: true,
            parameters: BTreeMap::from([
                ("fast_period".to_string(), 20.0),
                ("slow_period".to_string(), 50.0),
                ("rsi_period".to_string(), 14.0),
                ("adx_period".to_string(), 14.0),
            ]),
        }
    }

    pub fn volatility_mean_reversion() -> Self {
        Self {
            name: "volatility_mean_reversion".to_string(),
            description: "Bollinger displacement and ATR volatility as reversion evidence"
                .to_string(),
            category: FactorCategory::VolatilityMeanReversion,
            enabled: true,
            parameters: BTreeMap::from([
                ("bollinger_period".to_string(), 20.0),
                ("bollinger_std".to_string(), 2.0),
                ("adx_period".to_string(), 14.0),
                ("atr_period".to_string(), 14.0),
            ]),
        }
    }

    pub fn structure_ict() -> Self {
        Self {
            name: "structure_ict".to_string(),
            description: "ICT structure, FVG, OB and CISD as probabilistic structure evidence"
                .to_string(),
            category: FactorCategory::StructureIct,
            enabled: true,
            parameters: BTreeMap::from([
                ("lookback".to_string(), 20.0),
                ("expansion_threshold".to_string(), 1.5),
                ("sweep_atr_multiplier".to_string(), 0.45),
                ("sweep_return_bars".to_string(), 6.0),
                ("sweep_recency_bars".to_string(), 4.0),
                ("sweep_weight".to_string(), 0.18),
                ("unconfirmed_sweep_weight".to_string(), 0.04),
                ("opposing_sweep_penalty".to_string(), 0.10),
                ("post_sweep_displacement_weight".to_string(), 0.12),
            ]),
        }
    }

    pub fn cross_market_smt() -> Self {
        Self {
            name: "cross_market_smt".to_string(),
            description: "Cross-market relative strength and SMT divergence confirmation"
                .to_string(),
            category: FactorCategory::CrossMarketSmt,
            enabled: true,
            parameters: BTreeMap::from([("lookback".to_string(), 20.0)]),
        }
    }

    pub fn options_hedging() -> Self {
        Self {
            name: "options_hedging".to_string(),
            description: "Dealer hedging and options skew as probabilistic evidence".to_string(),
            category: FactorCategory::OptionsHedging,
            enabled: true,
            parameters: BTreeMap::from([("atr_period".to_string(), 14.0)]),
        }
    }

    pub fn parameter(&self, key: &str, default: f64) -> f64 {
        self.parameters.get(key).copied().unwrap_or(default)
    }

    pub fn set_parameter(&mut self, key: impl Into<String>, value: f64) {
        self.parameters.insert(key.into(), value);
    }

    pub fn mutation_parameter_group(&self, reason: &str) -> Vec<String> {
        match self.category {
            FactorCategory::TrendMomentum => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => vec![
                    "fast_period".to_string(),
                    "slow_period".to_string(),
                    "rsi_period".to_string(),
                ],
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    vec!["rsi_period".to_string(), "adx_period".to_string()]
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    vec!["adx_period".to_string(), "rsi_period".to_string()]
                }
                _ => Vec::new(),
            },
            FactorCategory::VolatilityMeanReversion => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => {
                    vec!["bollinger_period".to_string(), "bollinger_std".to_string()]
                }
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    vec!["bollinger_std".to_string(), "atr_period".to_string()]
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    vec!["adx_period".to_string(), "atr_period".to_string()]
                }
                _ => Vec::new(),
            },
            FactorCategory::StructureIct => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => {
                    vec![
                        "lookback".to_string(),
                        "expansion_threshold".to_string(),
                        "sweep_atr_multiplier".to_string(),
                        "sweep_weight".to_string(),
                        "unconfirmed_sweep_weight".to_string(),
                        "opposing_sweep_penalty".to_string(),
                        "post_sweep_displacement_weight".to_string(),
                    ]
                }
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    vec![
                        "expansion_threshold".to_string(),
                        "sweep_weight".to_string(),
                        "unconfirmed_sweep_weight".to_string(),
                        "opposing_sweep_penalty".to_string(),
                        "post_sweep_displacement_weight".to_string(),
                    ]
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    vec![
                        "expansion_threshold".to_string(),
                        "lookback".to_string(),
                        "sweep_recency_bars".to_string(),
                        "sweep_return_bars".to_string(),
                    ]
                }
                _ => Vec::new(),
            },
            FactorCategory::CrossMarketSmt => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak"
                | "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small"
                | "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => vec!["lookback".to_string()],
                _ => Vec::new(),
            },
            FactorCategory::OptionsHedging => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak"
                | "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small"
                | "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => vec!["atr_period".to_string()],
                _ => Vec::new(),
            },
        }
    }

    pub fn mutation_direction_hint(&self, reason: &str) -> BTreeMap<String, String> {
        match self.category {
            FactorCategory::TrendMomentum => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("fast_period".to_string(), "decrease".to_string()),
                    ("slow_period".to_string(), "increase".to_string()),
                    ("rsi_period".to_string(), "decrease".to_string()),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => BTreeMap::from([
                    ("rsi_period".to_string(), "decrease".to_string()),
                    ("adx_period".to_string(), "increase".to_string()),
                ]),
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([
                    ("adx_period".to_string(), "increase".to_string()),
                    ("rsi_period".to_string(), "increase".to_string()),
                ]),
                _ => BTreeMap::new(),
            },
            FactorCategory::VolatilityMeanReversion => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("bollinger_period".to_string(), "decrease".to_string()),
                    ("bollinger_std".to_string(), "tighten".to_string()),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => BTreeMap::from([
                    ("bollinger_std".to_string(), "tighten".to_string()),
                    ("atr_period".to_string(), "decrease".to_string()),
                ]),
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([
                    ("adx_period".to_string(), "increase".to_string()),
                    ("atr_period".to_string(), "increase".to_string()),
                ]),
                _ => BTreeMap::new(),
            },
            FactorCategory::StructureIct => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("lookback".to_string(), "decrease".to_string()),
                    ("expansion_threshold".to_string(), "tighten".to_string()),
                    ("sweep_atr_multiplier".to_string(), "tighten".to_string()),
                    ("sweep_weight".to_string(), "increase".to_string()),
                    (
                        "unconfirmed_sweep_weight".to_string(),
                        "decrease".to_string(),
                    ),
                    ("opposing_sweep_penalty".to_string(), "increase".to_string()),
                    (
                        "post_sweep_displacement_weight".to_string(),
                        "increase".to_string(),
                    ),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => BTreeMap::from([
                    ("expansion_threshold".to_string(), "tighten".to_string()),
                    ("sweep_weight".to_string(), "increase".to_string()),
                    (
                        "unconfirmed_sweep_weight".to_string(),
                        "decrease".to_string(),
                    ),
                    ("opposing_sweep_penalty".to_string(), "increase".to_string()),
                    (
                        "post_sweep_displacement_weight".to_string(),
                        "increase".to_string(),
                    ),
                ]),
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([
                    ("expansion_threshold".to_string(), "widen".to_string()),
                    ("lookback".to_string(), "increase".to_string()),
                    ("sweep_recency_bars".to_string(), "increase".to_string()),
                    ("sweep_return_bars".to_string(), "increase".to_string()),
                ]),
                _ => BTreeMap::new(),
            },
            FactorCategory::CrossMarketSmt => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => {
                    BTreeMap::from([("lookback".to_string(), "decrease".to_string())])
                }
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    BTreeMap::from([("lookback".to_string(), "increase".to_string())])
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    BTreeMap::from([("lookback".to_string(), "increase".to_string())])
                }
                _ => BTreeMap::new(),
            },
            FactorCategory::OptionsHedging => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => {
                    BTreeMap::from([("atr_period".to_string(), "decrease".to_string())])
                }
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    BTreeMap::from([("atr_period".to_string(), "decrease".to_string())])
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    BTreeMap::from([("atr_period".to_string(), "increase".to_string())])
                }
                _ => BTreeMap::new(),
            },
        }
    }

    pub fn mutation_step_size_hint(&self, reason: &str) -> BTreeMap<String, f64> {
        match self.category {
            FactorCategory::TrendMomentum => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("fast_period".to_string(), 0.10),
                    ("slow_period".to_string(), 0.10),
                    ("rsi_period".to_string(), 0.08),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => BTreeMap::from([
                    ("rsi_period".to_string(), 0.08),
                    ("adx_period".to_string(), 0.10),
                ]),
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([
                    ("adx_period".to_string(), 0.12),
                    ("rsi_period".to_string(), 0.08),
                ]),
                _ => BTreeMap::new(),
            },
            FactorCategory::VolatilityMeanReversion => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("bollinger_period".to_string(), 0.10),
                    ("bollinger_std".to_string(), 0.05),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => BTreeMap::from([
                    ("bollinger_std".to_string(), 0.05),
                    ("atr_period".to_string(), 0.08),
                ]),
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([
                    ("adx_period".to_string(), 0.12),
                    ("atr_period".to_string(), 0.08),
                ]),
                _ => BTreeMap::new(),
            },
            FactorCategory::StructureIct => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("lookback".to_string(), 0.10),
                    ("expansion_threshold".to_string(), 0.08),
                    ("sweep_atr_multiplier".to_string(), 0.08),
                    ("sweep_weight".to_string(), 0.15),
                    ("unconfirmed_sweep_weight".to_string(), 0.12),
                    ("opposing_sweep_penalty".to_string(), 0.12),
                    ("post_sweep_displacement_weight".to_string(), 0.12),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => BTreeMap::from([
                    ("expansion_threshold".to_string(), 0.08),
                    ("sweep_weight".to_string(), 0.12),
                    ("unconfirmed_sweep_weight".to_string(), 0.10),
                    ("opposing_sweep_penalty".to_string(), 0.10),
                    ("post_sweep_displacement_weight".to_string(), 0.12),
                ]),
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([
                    ("expansion_threshold".to_string(), 0.10),
                    ("lookback".to_string(), 0.10),
                    ("sweep_recency_bars".to_string(), 0.15),
                    ("sweep_return_bars".to_string(), 0.15),
                ]),
                _ => BTreeMap::new(),
            },
            FactorCategory::CrossMarketSmt => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak"
                | "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small"
                | "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([("lookback".to_string(), 0.10)]),
                _ => BTreeMap::new(),
            },
            FactorCategory::OptionsHedging => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak"
                | "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small"
                | "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    BTreeMap::from([("atr_period".to_string(), 0.10)])
                }
                _ => BTreeMap::new(),
            },
        }
    }

    pub fn evaluate<'a>(
        &self,
        candles: &[Candle],
        context: &FactorContext<'a>,
    ) -> Result<FactorSeries> {
        let signals = match self.category {
            FactorCategory::TrendMomentum => self.evaluate_trend_momentum(candles),
            FactorCategory::VolatilityMeanReversion => {
                self.evaluate_volatility_mean_reversion(candles)
            }
            FactorCategory::StructureIct => self.evaluate_structure_ict(candles),
            FactorCategory::CrossMarketSmt => self.evaluate_cross_market_smt(candles, context),
            FactorCategory::OptionsHedging => self.evaluate_options_hedging(candles, context),
        };

        Ok(FactorSeries {
            name: self.name.clone(),
            description: self.description.clone(),
            category: self.category,
            parameters: self.parameters.clone(),
            signals,
        })
    }

    fn evaluate_trend_momentum(&self, candles: &[Candle]) -> Vec<FactorSignal> {
        let fast_period = self.parameter("fast_period", 20.0) as usize;
        let slow_period = self.parameter("slow_period", 50.0) as usize;
        let rsi_period = self.parameter("rsi_period", 14.0) as usize;
        let adx_period = self.parameter("adx_period", 14.0) as usize;

        let fast_ema = pad_indicator(compute_ema(candles, fast_period), candles.len(), 0.0);
        let slow_ema = pad_indicator(compute_ema(candles, slow_period), candles.len(), 0.0);
        let rsi = pad_indicator(compute_rsi(candles, rsi_period), candles.len(), 50.0);
        let adx = pad_indicator(compute_adx(candles, adx_period), candles.len(), 0.0);

        candles
            .iter()
            .enumerate()
            .map(|(index, candle)| {
                let ema_edge = if candle.close.abs() > f64::EPSILON {
                    (fast_ema[index] - slow_ema[index]) / candle.close
                } else {
                    0.0
                };
                let rsi_edge = (rsi[index] - 50.0) / 50.0;
                let adx_score = (adx[index] / 100.0).clamp(0.0, 1.0);
                let value = normalize_signed(ema_edge * 12.0 + rsi_edge * 0.6, 1.0);
                let confidence = (value.abs() * 0.65 + adx_score * 0.35).clamp(0.0, 1.0);

                build_signal(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "ema_edge={:.4};rsi={:.2};adx={:.2}",
                        ema_edge, rsi[index], adx[index]
                    ),
                )
            })
            .collect()
    }

    fn evaluate_volatility_mean_reversion(&self, candles: &[Candle]) -> Vec<FactorSignal> {
        let bollinger_period = self.parameter("bollinger_period", 20.0) as usize;
        let bollinger_std = self.parameter("bollinger_std", 2.0);
        let adx_period = self.parameter("adx_period", 14.0) as usize;
        let atr_period = self.parameter("atr_period", 14.0) as usize;

        let bands = pad_bollinger(
            compute_bollinger(candles, bollinger_period, bollinger_std),
            candles.len(),
        );
        let adx = pad_indicator(compute_adx(candles, adx_period), candles.len(), 0.0);
        let atr = pad_indicator(atr_percent(candles, atr_period), candles.len(), 0.0);

        candles
            .iter()
            .enumerate()
            .map(|(index, candle)| {
                let band_range = (bands.upper[index] - bands.lower[index]).abs();
                let zscore = if band_range > f64::EPSILON {
                    (candle.close - bands.middle[index]) / band_range
                } else {
                    0.0
                };
                let value = normalize_signed(-zscore * 2.5, 1.0);
                let quiet_trend_bonus =
                    (1.0 - (adx[index] / 100.0).clamp(0.0, 1.0)).clamp(0.0, 1.0);
                let volatility_scale = (atr[index] / 5.0).clamp(0.0, 1.0);
                let confidence = (zscore.abs().min(1.0) * 0.55
                    + quiet_trend_bonus * 0.25
                    + volatility_scale * 0.20)
                    .clamp(0.0, 1.0);

                build_signal(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "zscore={:.4};adx={:.2};atr_pct={:.2}",
                        zscore, adx[index], atr[index]
                    ),
                )
            })
            .collect()
    }

    fn evaluate_structure_ict(&self, candles: &[Candle]) -> Vec<FactorSignal> {
        let lookback = self.parameter("lookback", 20.0) as usize;
        let expansion_threshold = self.parameter("expansion_threshold", 1.5);
        let sweep_atr_multiplier = self.parameter("sweep_atr_multiplier", 0.45);
        let sweep_return_bars = self.parameter("sweep_return_bars", 6.0) as usize;
        let sweep_recency_bars = self.parameter("sweep_recency_bars", 4.0) as usize;
        let sweep_weight = self.parameter("sweep_weight", 0.18);
        let unconfirmed_sweep_weight = self.parameter("unconfirmed_sweep_weight", 0.04);
        let opposing_sweep_penalty = self.parameter("opposing_sweep_penalty", 0.10);
        let post_sweep_displacement_weight = self.parameter("post_sweep_displacement_weight", 0.12);
        let atr = pad_indicator(compute_atr(candles, lookback.max(14)), candles.len(), 0.0);

        candles
            .iter()
            .enumerate()
            .map(|(index, candle)| {
                let start = index.saturating_sub(lookback * 2);
                let window = &candles[start..=index];
                let window_atr = &atr[start..=index];

                let swing_highs = find_swing_highs(window, 3);
                let swing_lows = find_swing_lows(window, 3);
                let breaks = detect_structure_breaks(window, &swing_highs, &swing_lows);
                let latest_break = breaks.last().map(|item| item.direction);
                let bull_expansion = check_bull_expansion_exists(
                    window,
                    lookback.min(window.len()),
                    expansion_threshold,
                );
                let bear_expansion = check_bear_expansion_exists(
                    window,
                    lookback.min(window.len()),
                    expansion_threshold,
                );
                let order_blocks = detect_order_blocks(window);
                let cisds = detect_cisd(window, &order_blocks, 1);
                let bullish_cisd = cisds.iter().any(|cisd| {
                    cisd.direction == Direction::Bull
                        && cisd.confirm_bar >= window.len().saturating_sub(3)
                });
                let bearish_cisd = cisds.iter().any(|cisd| {
                    cisd.direction == Direction::Bear
                        && cisd.confirm_bar >= window.len().saturating_sub(3)
                });
                let pools = detect_liquidity_pools(window, window_atr, sweep_atr_multiplier, 2);
                let sweeps = detect_liquidity_sweep(window, &pools, sweep_return_bars);
                let recent_bull_sweep = sweeps
                    .iter()
                    .rev()
                    .find(|sweep| {
                        sweep.sweep_direction == Direction::Bull
                            && sweep.return_bar >= window.len().saturating_sub(sweep_recency_bars)
                    });
                let recent_bear_sweep = sweeps
                    .iter()
                    .rev()
                    .find(|sweep| {
                        sweep.sweep_direction == Direction::Bear
                            && sweep.return_bar >= window.len().saturating_sub(sweep_recency_bars)
                    });
                let bull_fvg = find_unfilled_fvgs(window)
                    .iter()
                    .filter(|fvg| fvg.direction == Direction::Bull)
                    .count() as f64;
                let bear_fvg = find_unfilled_fvgs(window)
                    .iter()
                    .filter(|fvg| fvg.direction == Direction::Bear)
                    .count() as f64;
                let bull_ob = find_untested_obs(window)
                    .iter()
                    .filter(|ob| ob.ob_type == Direction::Bull)
                    .count() as f64;
                let bear_ob = find_untested_obs(window)
                    .iter()
                    .filter(|ob| ob.ob_type == Direction::Bear)
                    .count() as f64;

                let mut bull_score = 0.0;
                let mut bear_score = 0.0;
                if bull_expansion {
                    bull_score += 0.40;
                }
                if bear_expansion {
                    bear_score += 0.40;
                }
                match latest_break {
                    Some(Direction::Bull) => bull_score += 0.25,
                    Some(Direction::Bear) => bear_score += 0.25,
                    _ => {}
                }
                if bullish_cisd {
                    bull_score += 0.20;
                }
                if bearish_cisd {
                    bear_score += 0.20;
                }
                let bull_sweep_displacement = recent_bull_sweep
                    .map(|sweep| {
                        ((candle.close - sweep.pool_price) / sweep.pool_price.abs().max(1.0))
                            .max(0.0)
                    })
                    .unwrap_or(0.0);
                let bear_sweep_displacement = recent_bear_sweep
                    .map(|sweep| {
                        ((sweep.pool_price - candle.close) / sweep.pool_price.abs().max(1.0))
                            .max(0.0)
                    })
                    .unwrap_or(0.0);
                let bull_manipulation_confirmed = recent_bull_sweep.is_some()
                    && (bullish_cisd
                        || latest_break == Some(Direction::Bull)
                        || bull_expansion);
                let bear_manipulation_confirmed = recent_bear_sweep.is_some()
                    && (bearish_cisd
                        || latest_break == Some(Direction::Bear)
                        || bear_expansion);
                if bull_manipulation_confirmed {
                    bull_score += sweep_weight;
                    bull_score += post_sweep_displacement_weight
                        * (1.0 + bull_sweep_displacement * 10.0).clamp(1.0, 2.0);
                } else if recent_bull_sweep.is_some() {
                    bull_score += unconfirmed_sweep_weight;
                }
                if bear_manipulation_confirmed {
                    bear_score += sweep_weight;
                    bear_score += post_sweep_displacement_weight
                        * (1.0 + bear_sweep_displacement * 10.0).clamp(1.0, 2.0);
                } else if recent_bear_sweep.is_some() {
                    bear_score += unconfirmed_sweep_weight;
                }
                if bull_manipulation_confirmed && recent_bear_sweep.is_some() && !bear_manipulation_confirmed
                {
                    bear_score = (bear_score - opposing_sweep_penalty).max(0.0);
                }
                if bear_manipulation_confirmed && recent_bull_sweep.is_some() && !bull_manipulation_confirmed
                {
                    bull_score = (bull_score - opposing_sweep_penalty).max(0.0);
                }
                bull_score += (bull_fvg.min(3.0) + bull_ob.min(3.0)) * 0.05;
                bear_score += (bear_fvg.min(3.0) + bear_ob.min(3.0)) * 0.05;

                let value = (bull_score - bear_score).clamp(-1.0, 1.0);
                let confidence = bull_score.max(bear_score).clamp(0.0, 1.0);

                build_signal(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "bull_expansion={};bear_expansion={};bull_sweep={};bear_sweep={};bull_manipulation_confirmed={};bear_manipulation_confirmed={};bull_sweep_displacement={:.4};bear_sweep_displacement={:.4};bull_score={:.2};bear_score={:.2}",
                        bull_expansion,
                        bear_expansion,
                        recent_bull_sweep.is_some(),
                        recent_bear_sweep.is_some(),
                        bull_manipulation_confirmed,
                        bear_manipulation_confirmed,
                        bull_sweep_displacement,
                        bear_sweep_displacement,
                        bull_score,
                        bear_score
                    ),
                )
            })
            .collect()
    }

    fn evaluate_cross_market_smt<'a>(
        &self,
        candles: &[Candle],
        context: &FactorContext<'a>,
    ) -> Vec<FactorSignal> {
        let lookback = self.parameter("lookback", 20.0) as usize;
        let Some(paired) = context.paired_candles else {
            return candles
                .iter()
                .map(|candle| {
                    build_signal(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.05,
                        "paired_market_unavailable".to_string(),
                    )
                })
                .collect();
        };

        let aligned = candles.len().min(paired.len());
        let start = candles.len().saturating_sub(aligned);
        let primary = &candles[start..];
        let pair = &paired[paired.len().saturating_sub(aligned)..];
        let pair_quality =
            paired_market_quality_report(candles.len(), paired.len(), lookback, None, None);
        let pair_quality_explanation = paired_market_quality_explanation(&pair_quality);

        candles
            .iter()
            .enumerate()
            .map(|(index, candle)| {
                if index < start {
                    return build_signal(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.05,
                        "waiting_for_aligned_cross_market_history".to_string(),
                    );
                }

                let aligned_index = index - start;
                if pair_quality.status == "invalid_due_to_pair_quality" {
                    return build_signal(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.0,
                        pair_quality_explanation.clone(),
                    );
                }
                if aligned_index < lookback {
                    return build_signal(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.10,
                        format!(
                            "insufficient_cross_market_lookback;{}",
                            pair_quality_explanation
                        ),
                    );
                }

                let primary_window = &primary[aligned_index - lookback..=aligned_index];
                let pair_window = &pair[aligned_index - lookback..=aligned_index];
                let primary_closes = primary_window
                    .iter()
                    .map(|item| item.close)
                    .collect::<Vec<_>>();
                let pair_closes = pair_window
                    .iter()
                    .map(|item| item.close)
                    .collect::<Vec<_>>();
                let aligned_len = primary_closes.len().min(pair_closes.len());
                if aligned_len < 3 {
                    return build_signal(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.05,
                        format!(
                            "insufficient_aligned_cross_market_samples;{}",
                            pair_quality_explanation
                        ),
                    );
                }
                let primary_closes = &primary_closes[..aligned_len];
                let pair_closes = &pair_closes[..aligned_len];
                let window_quality = paired_market_window_quality_report(
                    primary_window.len(),
                    pair_window.len(),
                    primary_closes,
                    pair_closes,
                    lookback,
                );
                let window_quality_explanation = paired_market_quality_explanation(&window_quality);
                if window_quality.status == "invalid_due_to_pair_quality" {
                    return build_signal_with_pair_quality(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.0,
                        window_quality_explanation,
                        Some(window_quality),
                    );
                }
                if window_quality.status == "valid_but_flat" {
                    return build_signal_with_pair_quality(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.05,
                        window_quality_explanation,
                        Some(window_quality),
                    );
                }
                let primary_returns = close_returns(primary_closes);
                let pair_returns = close_returns(pair_closes);
                let return_len = primary_returns.len().min(pair_returns.len());
                if return_len < 2 {
                    return build_signal(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.05,
                        format!(
                            "insufficient_aligned_cross_market_returns;{}",
                            window_quality_explanation
                        ),
                    );
                }
                let primary_returns = &primary_returns[..return_len];
                let pair_returns = &pair_returns[..return_len];
                let correlation = Correlation::pearson(primary_returns, pair_returns);
                let divergence = if window_quality.safe_lookback < 2 {
                    false
                } else {
                    Divergence::detect(primary_closes, pair_closes, window_quality.safe_lookback)
                        .last()
                        .copied()
                        .unwrap_or(false)
                };
                let primary_ret = total_return(primary_window);
                let pair_ret = total_return(pair_window);
                let relative_strength = primary_ret - pair_ret;
                let mut value = normalize_signed(relative_strength * 6.0, 1.0);
                if divergence {
                    value *= 0.5;
                }
                let confidence =
                    (correlation.abs() * if divergence { 0.45 } else { 0.85 }).clamp(0.0, 1.0);

                build_signal_with_pair_quality(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "corr={:.4};divergence={};relative_strength={:.4};{}",
                        correlation, divergence, relative_strength, window_quality_explanation
                    ),
                    Some(window_quality),
                )
            })
            .collect()
    }

    fn evaluate_options_hedging<'a>(
        &self,
        candles: &[Candle],
        context: &FactorContext<'a>,
    ) -> Vec<FactorSignal> {
        let atr_period = self.parameter("atr_period", 14.0) as usize;
        let atr = pad_indicator(atr_percent(candles, atr_period), candles.len(), 0.0);

        candles
            .iter()
            .enumerate()
            .map(|(index, candle)| {
                let (value, confidence, explanation) = if let Some(aux) = context.auxiliary {
                    let put_call_bias = aux
                        .put_call_oi_ratio
                        .map(|ratio| (1.0 - ratio).clamp(-1.0, 1.0))
                        .unwrap_or(0.0);
                    let gamma_skew = aux.gamma_skew.unwrap_or(0.0);
                    let hedge_score = aux.hedge_pressure_score.unwrap_or(0.0);
                    let value = normalize_signed(
                        hedge_score + gamma_skew * 0.4 + put_call_bias * 0.35,
                        1.0,
                    );
                    let mut confidence: f64 = 0.35;
                    if aux.near_atm_gamma.is_some() {
                        confidence += 0.20;
                    }
                    if aux.gamma_skew.is_some() {
                        confidence += 0.20;
                    }
                    if aux.put_call_oi_ratio.is_some() || aux.put_call_volume_ratio.is_some() {
                        confidence += 0.15;
                    }
                    (
                        value,
                        confidence.clamp(0.0, 1.0),
                        format!(
                            "hedge_score={:.4};gamma_skew={:.4};put_call_oi_ratio={:?}",
                            hedge_score, gamma_skew, aux.put_call_oi_ratio
                        ),
                    )
                } else {
                    let uncertainty_proxy = (atr[index] / 5.0).clamp(0.0, 1.0);
                    (
                        0.0,
                        (0.10 + uncertainty_proxy * 0.15).clamp(0.0, 0.30),
                        "options_proxy_only_from_realized_volatility".to_string(),
                    )
                };

                build_signal(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    explanation,
                )
            })
            .collect()
    }
}

fn pad_indicator(values: Vec<f64>, target_len: usize, fill: f64) -> Vec<f64> {
    if values.len() >= target_len {
        return values[values.len() - target_len..].to_vec();
    }

    let mut padded = vec![fill; target_len - values.len()];
    padded.extend(values);
    padded
}

fn pad_bollinger(bands: BollingerBands, target_len: usize) -> BollingerBands {
    BollingerBands {
        upper: pad_indicator(bands.upper, target_len, 0.0),
        middle: pad_indicator(bands.middle, target_len, 0.0),
        lower: pad_indicator(bands.lower, target_len, 0.0),
    }
}

fn build_signal(
    factor_name: &str,
    category: FactorCategory,
    timestamp: DateTime<Utc>,
    value: f64,
    confidence: f64,
    explanation: String,
) -> FactorSignal {
    build_signal_with_pair_quality(
        factor_name,
        category,
        timestamp,
        value,
        confidence,
        explanation,
        None,
    )
}

fn build_signal_with_pair_quality(
    factor_name: &str,
    category: FactorCategory,
    timestamp: DateTime<Utc>,
    value: f64,
    confidence: f64,
    explanation: String,
    paired_market_quality_report: Option<PairedMarketQualityReport>,
) -> FactorSignal {
    FactorSignal {
        factor_name: factor_name.to_string(),
        category,
        timestamp,
        value,
        direction: direction_from_value(value, confidence),
        confidence,
        explanation,
        paired_market_quality_report,
        ..FactorSignal::default()
    }
}

fn direction_from_value(value: f64, confidence: f64) -> Direction {
    if confidence < 0.10 {
        Direction::Neutral
    } else if value > 0.0 {
        Direction::Bull
    } else if value < 0.0 {
        Direction::Bear
    } else {
        Direction::Neutral
    }
}

fn close_returns(closes: &[f64]) -> Vec<f64> {
    closes
        .windows(2)
        .filter_map(|window| {
            if window[0].abs() <= f64::EPSILON {
                None
            } else {
                Some((window[1] - window[0]) / window[0])
            }
        })
        .collect()
}

fn total_return(candles: &[Candle]) -> f64 {
    if candles.len() < 2 || candles[0].close.abs() <= f64::EPSILON {
        return 0.0;
    }
    (candles.last().unwrap().close - candles[0].close) / candles[0].close
}

fn normalize_signed(value: f64, cap: f64) -> f64 {
    if cap <= f64::EPSILON {
        0.0
    } else {
        (value / cap).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn candles(count: usize) -> Vec<Candle> {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        (0..count)
            .map(|index| {
                let base = 100.0 + index as f64 * 0.4;
                Candle {
                    timestamp: start + Duration::minutes(index as i64),
                    open: base,
                    high: base + 0.6,
                    low: base - 0.5,
                    close: base + 0.3,
                    volume: 1_000.0 + index as f64,
                }
            })
            .collect()
    }

    fn flat_candles(count: usize, level: f64) -> Vec<Candle> {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        (0..count)
            .map(|index| Candle {
                timestamp: start + Duration::minutes(index as i64),
                open: level,
                high: level,
                low: level,
                close: level,
                volume: 1_000.0 + index as f64,
            })
            .collect()
    }

    #[test]
    fn test_factor_definition_emits_series_direction_and_confidence() {
        let factor = FactorDefinition::trend_momentum();
        let candles = candles(80);
        let series = factor
            .evaluate(&candles, &FactorContext::default())
            .unwrap();

        assert_eq!(series.signals.len(), candles.len());
        let last = series.latest_signal().unwrap();
        assert!(last.confidence >= 0.0 && last.confidence <= 1.0);
        assert!(matches!(
            last.direction,
            Direction::Bull | Direction::Bear | Direction::Neutral
        ));
    }

    #[test]
    fn test_cross_market_smt_handles_short_aligned_series_without_panic() {
        let definition = FactorDefinition::cross_market_smt();
        let candles = candles(25);
        let paired = (0..19)
            .map(|index| {
                let base = 200.0 + index as f64 * 0.25;
                Candle {
                    timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
                        + Duration::minutes(index as i64),
                    open: base,
                    high: base + 0.4,
                    low: base - 0.3,
                    close: base + 0.2,
                    volume: 1000.0,
                }
            })
            .collect::<Vec<_>>();
        let context = FactorContext {
            paired_candles: Some(&paired),
            auxiliary: None,
            regime: None,
        };

        let series = definition.evaluate(&candles, &context).unwrap();

        assert_eq!(series.signals.len(), candles.len());
        assert!(series.signals.iter().all(|signal| signal.confidence >= 0.0));
    }

    #[test]
    fn test_cross_market_smt_marks_low_overlap_pair_quality_invalid() {
        let definition = FactorDefinition::cross_market_smt();
        let primary = candles(100);
        let paired = candles(30);
        let context = FactorContext {
            paired_candles: Some(&paired),
            auxiliary: None,
            regime: None,
        };

        let series = definition.evaluate(&primary, &context).unwrap();
        let latest = series.latest_signal().unwrap();

        assert_eq!(latest.value, 0.0);
        assert_eq!(latest.confidence, 0.0);
        assert!(latest
            .explanation
            .contains("status=invalid_due_to_pair_quality"));
        assert!(latest.explanation.contains("quality_tier=poor"));
        assert!(latest.explanation.contains("aligned_length=30"));
        assert!(latest.explanation.contains("overlap_ratio=0.3000"));
    }

    #[test]
    fn test_cross_market_smt_marks_flat_pair_valid_but_flat() {
        let definition = FactorDefinition::cross_market_smt();
        let primary_window = candles(21)
            .into_iter()
            .map(|candle| candle.close)
            .collect::<Vec<_>>();
        let pair_window = flat_candles(21, 200.0)
            .into_iter()
            .map(|candle| candle.close)
            .collect::<Vec<_>>();

        let report = paired_market_window_quality_report(80, 80, &primary_window, &pair_window, 20);
        let explanation = paired_market_quality_explanation(&report);

        assert_eq!(definition.category, FactorCategory::CrossMarketSmt);
        assert_eq!(report.status, "invalid_due_to_pair_quality");
        assert_eq!(report.paired_market_quality, "poor");
        assert_eq!(report.reason, "insufficient_aligned_history");
        assert_eq!(report.aligned_length, 21);
        assert_eq!(report.primary_length, 80);
        assert_eq!(report.paired_length, 80);
        assert!(explanation.contains("status=invalid_due_to_pair_quality"));
        assert!(explanation.contains("quality_tier=poor"));
        assert!(explanation.contains("reason=insufficient_aligned_history"));
        assert!(explanation.contains("aligned_length=21"));
    }

    #[test]
    fn test_cross_market_smt_invalid_window_pair_quality_stays_invalid_even_if_flat() {
        let definition = FactorDefinition::cross_market_smt();
        let candles = candles(80);
        let paired = flat_candles(80, 200.0);
        let context = FactorContext {
            paired_candles: Some(&paired),
            auxiliary: None,
            regime: None,
        };

        let series = definition.evaluate(&candles, &context).unwrap();
        let target = &series.signals[52];

        assert_eq!(target.value, 0.0);
        assert_eq!(target.confidence, 0.0);
        assert!(target
            .explanation
            .contains("status=invalid_due_to_pair_quality"));
        assert!(target.explanation.contains("quality_tier=poor"));
        assert!(target.explanation.contains("aligned_length=21"));
    }
}
