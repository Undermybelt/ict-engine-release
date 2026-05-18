use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::data::realtime::market_support::AuxiliaryMarketEvidence;
use crate::ict::{
    check_bear_expansion_exists, check_bull_expansion_exists, detect_cisd, detect_liquidity_pools,
    detect_liquidity_sweep, detect_order_blocks, detect_structure_breaks, find_swing_highs,
    find_swing_lows, find_unfilled_fvgs, find_untested_obs,
};
use crate::indicators::{
    atr_percent, compute_adx, compute_atr, compute_bollinger, compute_ema, compute_rsi,
    BollingerBands,
};
use crate::pda_timeline::{
    build_pda_timeline, match_all_setups_extended, PdaEvent, SetupContext, SetupMatch,
};
use crate::smt::Correlation;
use crate::types::{Candle, Direction, Regime, RegimeV2};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FactorCategory {
    TrendMomentum,
    VolatilityMeanReversion,
    StructureIct,
    CrossMarketSmt,
    OptionsHedging,
    /// Family E: crowding/herding execution-risk factors
    CrowdingHerding,
    /// Family F: spectral rhythm / chaos execution filters
    SpectralRhythm,
    /// Family H: session / liquidity-window quality factors
    SessionLiquidity,
}

impl FactorCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TrendMomentum => "trend_momentum",
            Self::VolatilityMeanReversion => "volatility_mean_reversion",
            Self::StructureIct => "structure_ict",
            Self::CrossMarketSmt => "cross_market_smt",
            Self::OptionsHedging => "options_hedging",
            Self::CrowdingHerding => "crowding_herding",
            Self::SpectralRhythm => "spectral_rhythm",
            Self::SessionLiquidity => "session_liquidity",
        }
    }

    pub fn is_footprint_context_only(self) -> bool {
        matches!(
            self,
            Self::StructureIct
                | Self::CrossMarketSmt
                | Self::OptionsHedging
                | Self::CrowdingHerding
                | Self::SpectralRhythm
                | Self::SessionLiquidity
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
            Self::CrowdingHerding => &[
                FactorRole::PriorAdjuster,
                FactorRole::SetupClassifier,
                FactorRole::OutcomeValidator,
            ],
            Self::SpectralRhythm => &[
                FactorRole::PriorAdjuster,
                FactorRole::StateTransition,
                FactorRole::SetupClassifier,
            ],
            Self::SessionLiquidity => &[
                FactorRole::PriorAdjuster,
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

#[derive(Debug, Clone, Default, Serialize)]
pub struct FactorContext<'a> {
    pub paired_candles: Option<&'a [Candle]>,
    pub m1_events: Option<&'a [PdaEvent]>,
    pub m5_events: Option<&'a [PdaEvent]>,
    pub m15_events: Option<&'a [PdaEvent]>,
    pub m30_events: Option<&'a [PdaEvent]>,
    pub h1_events: Option<&'a [PdaEvent]>,
    pub h4_events: Option<&'a [PdaEvent]>,
    pub d1_events: Option<&'a [PdaEvent]>,
    pub w1_events: Option<&'a [PdaEvent]>,
    pub auxiliary: Option<&'a AuxiliaryMarketEvidence>,
    /// Legacy 3-state regime (deprecated, use regime_v2_labels)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regime: Option<Regime>,
    /// Regime V2 labels map: timestamp string -> RegimeV2
    /// Used for per-bar regime lookup during backtest
    #[serde(skip)]
    pub regime_v2_labels: Option<&'a std::collections::HashMap<String, RegimeV2>>,
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
                ("setup_weight".to_string(), 0.06),
                ("setup_recency_bars".to_string(), 4.0),
                ("setup_horizon_bars".to_string(), 30.0),
            ]),
        }
    }

    pub fn cross_market_smt() -> Self {
        Self {
            name: "cross_market_smt".to_string(),
            description: "ICT SMT sibling-market liquidity sweep confirmation failure".to_string(),
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

    /// Family E: crowding / herding execution-risk factors
    pub fn crowding_herding() -> Self {
        Self {
            name: "crowding_herding".to_string(),
            description: "Volume participation concentration and same-side herding pressure as execution-risk evidence".to_string(),
            category: FactorCategory::CrowdingHerding,
            enabled: true,
            parameters: BTreeMap::from([
                ("lookback".to_string(), 20.0),
                ("volume_spike_ratio".to_string(), 2.0),
                ("participation_concentration_weight".to_string(), 0.40),
                ("same_side_pressure_weight".to_string(), 0.35),
                ("crowding_relief_weight".to_string(), 0.25),
            ]),
        }
    }

    /// Family F: spectral rhythm / chaos execution filters
    pub fn spectral_rhythm() -> Self {
        Self {
            name: "spectral_rhythm".to_string(),
            description: "Spectral entropy, dominant cycle energy, and rhythm stability as execution-readiness filters".to_string(),
            category: FactorCategory::SpectralRhythm,
            enabled: true,
            parameters: BTreeMap::from([
                ("lookback".to_string(), 64.0),
                ("spectral_entropy_weight".to_string(), 0.45),
                ("cycle_energy_weight".to_string(), 0.35),
                ("rhythm_stability_weight".to_string(), 0.20),
            ]),
        }
    }

    /// Family H: session / liquidity-window quality factors
    pub fn session_liquidity() -> Self {
        Self {
            name: "session_liquidity".to_string(),
            description: "Session participation quality, kill-zone alignment, and session transition risk as execution-readiness multipliers".to_string(),
            category: FactorCategory::SessionLiquidity,
            enabled: true,
            parameters: BTreeMap::from([
                ("lookback".to_string(), 20.0),
                ("session_quality_weight".to_string(), 0.40),
                ("kill_zone_weight".to_string(), 0.35),
                ("transition_risk_weight".to_string(), 0.25),
            ]),
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
            FactorCategory::CrowdingHerding => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => {
                    vec!["lookback".to_string(), "volume_spike_ratio".to_string()]
                }
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => vec![
                    "participation_concentration_weight".to_string(),
                    "same_side_pressure_weight".to_string(),
                ],
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    vec!["lookback".to_string(), "volume_spike_ratio".to_string()]
                }
                _ => Vec::new(),
            },
            FactorCategory::SpectralRhythm => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => vec![
                    "lookback".to_string(),
                    "spectral_entropy_weight".to_string(),
                ],
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => vec![
                    "cycle_energy_weight".to_string(),
                    "rhythm_stability_weight".to_string(),
                ],
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => vec![
                    "lookback".to_string(),
                    "spectral_entropy_weight".to_string(),
                ],
                _ => Vec::new(),
            },
            FactorCategory::SessionLiquidity => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => {
                    vec!["lookback".to_string(), "session_quality_weight".to_string()]
                }
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => vec![
                    "kill_zone_weight".to_string(),
                    "transition_risk_weight".to_string(),
                ],
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    vec!["lookback".to_string(), "session_quality_weight".to_string()]
                }
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
            FactorCategory::CrowdingHerding => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("lookback".to_string(), "increase".to_string()),
                    ("volume_spike_ratio".to_string(), "decrease".to_string()),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => BTreeMap::from([(
                    "participation_concentration_weight".to_string(),
                    "increase".to_string(),
                )]),
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    BTreeMap::from([("lookback".to_string(), "increase".to_string())])
                }
                _ => BTreeMap::new(),
            },
            FactorCategory::SpectralRhythm => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("lookback".to_string(), "increase".to_string()),
                    (
                        "spectral_entropy_weight".to_string(),
                        "decrease".to_string(),
                    ),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    BTreeMap::from([("cycle_energy_weight".to_string(), "increase".to_string())])
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    BTreeMap::from([("lookback".to_string(), "increase".to_string())])
                }
                _ => BTreeMap::new(),
            },
            FactorCategory::SessionLiquidity => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("lookback".to_string(), "increase".to_string()),
                    ("session_quality_weight".to_string(), "increase".to_string()),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    BTreeMap::from([("kill_zone_weight".to_string(), "increase".to_string())])
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => {
                    BTreeMap::from([("lookback".to_string(), "increase".to_string())])
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
            FactorCategory::CrowdingHerding => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("lookback".to_string(), 0.10),
                    ("volume_spike_ratio".to_string(), 0.08),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    BTreeMap::from([("participation_concentration_weight".to_string(), 0.12)])
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([("lookback".to_string(), 0.10)]),
                _ => BTreeMap::new(),
            },
            FactorCategory::SpectralRhythm => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("lookback".to_string(), 0.10),
                    ("spectral_entropy_weight".to_string(), 0.08),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    BTreeMap::from([("cycle_energy_weight".to_string(), 0.12)])
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([("lookback".to_string(), 0.10)]),
                _ => BTreeMap::new(),
            },
            FactorCategory::SessionLiquidity => match reason {
                "balanced_accuracy_regressed"
                | "bull_bear_separation_regressed"
                | "bull_bear_separation_weak"
                | "worst_market_separation_weak" => BTreeMap::from([
                    ("lookback".to_string(), 0.10),
                    ("session_quality_weight".to_string(), 0.12),
                ]),
                "bridge_gap_regressed"
                | "bridge_gap_too_small"
                | "worst_market_bridge_gap_too_small" => {
                    BTreeMap::from([("kill_zone_weight".to_string(), 0.12)])
                }
                "pre_bayes_gate_regressed"
                | "pre_bayes_gate_observe_only"
                | "pre_bayes_gate_neutralized" => BTreeMap::from([("lookback".to_string(), 0.10)]),
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
            FactorCategory::StructureIct => self.evaluate_structure_ict(candles, context),
            FactorCategory::CrossMarketSmt => self.evaluate_cross_market_smt(candles, context),
            FactorCategory::OptionsHedging => self.evaluate_options_hedging(candles, context),
            FactorCategory::CrowdingHerding => self.evaluate_crowding_herding(candles),
            FactorCategory::SpectralRhythm => self.evaluate_spectral_rhythm(candles),
            FactorCategory::SessionLiquidity => self.evaluate_session_liquidity(candles),
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

    fn evaluate_structure_ict<'a>(
        &self,
        candles: &[Candle],
        context: &FactorContext<'a>,
    ) -> Vec<FactorSignal> {
        let lookback = self.parameter("lookback", 20.0) as usize;
        let expansion_threshold = self.parameter("expansion_threshold", 1.5);
        let sweep_atr_multiplier = self.parameter("sweep_atr_multiplier", 0.45);
        let sweep_return_bars = self.parameter("sweep_return_bars", 6.0) as usize;
        let sweep_recency_bars = self.parameter("sweep_recency_bars", 4.0) as usize;
        let sweep_weight = self.parameter("sweep_weight", 0.18);
        let unconfirmed_sweep_weight = self.parameter("unconfirmed_sweep_weight", 0.04);
        let opposing_sweep_penalty = self.parameter("opposing_sweep_penalty", 0.10);
        let post_sweep_displacement_weight = self.parameter("post_sweep_displacement_weight", 0.12);
        let setup_weight = self.parameter("setup_weight", 0.06);
        let setup_recency_bars = self.parameter("setup_recency_bars", 4.0) as usize;
        let atr = pad_indicator(compute_atr(candles, lookback.max(14)), candles.len(), 0.0);

        // Canonical-setup matches over the unified PDA timeline.
        // Built once for the whole series; per-bar lookups filter by
        // `confirm_bar <= index` (forward-only) and recency window.
        // Using the *extended* dispatcher activates the 5 session-
        // aware setups (FVG inside silver-bullet windows, etc.) and
        // the 3 OTE confluence setups, since `primary_candles` is
        // available here. Cross-TF and SMT setups stay dormant from
        // this entry point because `htf_events` / `paired_candles`
        // are not part of this factor's input contract; external
        // callers can supply them via `match_all_setups_extended`.
        let setup_matches = self.structure_ict_setup_matches_with_context(candles, context);
        let pda_context_events = format!(
            "m1:{}|m5:{}|m15:{}|m30:{}|h1:{}|h4:{}|d1:{}|w1:{}",
            context.m1_events.map_or(0, <[PdaEvent]>::len),
            context.m5_events.map_or(0, <[PdaEvent]>::len),
            context.m15_events.map_or(0, <[PdaEvent]>::len),
            context.m30_events.map_or(0, <[PdaEvent]>::len),
            context.h1_events.map_or(0, <[PdaEvent]>::len),
            context.h4_events.map_or(0, <[PdaEvent]>::len),
            context.d1_events.map_or(0, <[PdaEvent]>::len),
            context.w1_events.map_or(0, <[PdaEvent]>::len),
        );

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
                            .clamp(0.0, f64::MAX)
                    })
                    .unwrap_or(0.0);
                let bear_sweep_displacement = recent_bear_sweep
                    .map(|sweep| {
                        ((sweep.pool_price - candle.close) / sweep.pool_price.abs().max(1.0))
                            .clamp(0.0, f64::MAX)
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
                    bear_score = (bear_score - opposing_sweep_penalty).clamp(0.0, f64::MAX);
                }
                if bear_manipulation_confirmed && recent_bull_sweep.is_some() && !bull_manipulation_confirmed
                {
                    bull_score = (bull_score - opposing_sweep_penalty).clamp(0.0, f64::MAX);
                }
                bull_score += (bull_fvg.min(3.0) + bull_ob.min(3.0)) * 0.05;
                bear_score += (bear_fvg.min(3.0) + bear_ob.min(3.0)) * 0.05;

                // Canonical-setup contributions: count matches whose
                // confirm_bar falls within [index - recency, index].
                let recency_lo = index.saturating_sub(setup_recency_bars);
                let active_setups = setup_matches
                    .iter()
                    .filter(|m| m.confirm_bar >= recency_lo && m.confirm_bar <= index);
                let mut bull_setup_hits = 0usize;
                let mut bear_setup_hits = 0usize;
                for m in active_setups {
                    match m.direction {
                        Direction::Bull => {
                            bull_score += setup_weight;
                            bull_setup_hits += 1;
                        }
                        Direction::Bear => {
                            bear_score += setup_weight;
                            bear_setup_hits += 1;
                        }
                        Direction::Neutral => {}
                    }
                }

                let value = (bull_score - bear_score).clamp(-1.0, 1.0);
                let confidence = bull_score.max(bear_score).clamp(0.0, 1.0);

                build_signal(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "bull_expansion={};bear_expansion={};bull_sweep={};bear_sweep={};bull_manipulation_confirmed={};bear_manipulation_confirmed={};bull_sweep_displacement={:.4};bear_sweep_displacement={:.4};pda_context_events={};bull_setup_hits={};bear_setup_hits={};bull_score={:.2};bear_score={:.2}",
                        bull_expansion,
                        bear_expansion,
                        recent_bull_sweep.is_some(),
                        recent_bear_sweep.is_some(),
                        bull_manipulation_confirmed,
                        bear_manipulation_confirmed,
                        bull_sweep_displacement,
                        bear_sweep_displacement,
                        pda_context_events,
                        bull_setup_hits,
                        bear_setup_hits,
                        bull_score,
                        bear_score
                    ),
                )
            })
            .collect()
    }

    /// Returns the canonical-setup matches that this `structure_ict`
    /// factor would consume for the given candle slice. Exposed for
    /// the analyze report shell and factor_research diagnostics so
    /// they can render setup tallies without re-running the
    /// detection pipeline.
    pub fn structure_ict_setup_matches(&self, candles: &[Candle]) -> Vec<SetupMatch> {
        self.structure_ict_setup_matches_with_context(candles, &FactorContext::default())
    }

    pub fn structure_ict_setup_matches_with_context<'a>(
        &self,
        candles: &[Candle],
        context: &FactorContext<'a>,
    ) -> Vec<SetupMatch> {
        let lookback = self.parameter("lookback", 20.0) as usize;
        let setup_horizon_bars = self.parameter("setup_horizon_bars", 30.0) as usize;
        let atr = pad_indicator(compute_atr(candles, lookback.max(14)), candles.len(), 0.0);
        let timeline = build_pda_timeline(candles, &atr);
        collect_structure_ict_setup_matches(&timeline, candles, context, setup_horizon_bars)
    }

    fn evaluate_cross_market_smt<'a>(
        &self,
        candles: &[Candle],
        context: &FactorContext<'a>,
    ) -> Vec<FactorSignal> {
        let lookback = self.parameter("lookback", 20.0) as usize;
        let signal_lookback = lookback.max(31);
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
                if aligned_index < signal_lookback {
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

                let primary_window = &primary[aligned_index - signal_lookback..=aligned_index];
                let pair_window = &pair[aligned_index - signal_lookback..=aligned_index];
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
                let smt_event = detect_cross_market_smt_event(
                    primary_window,
                    pair_window,
                    correlation <= -0.3,
                );
                let relationship_type = if correlation >= 0.3 {
                    "positive"
                } else if correlation <= -0.3 {
                    "negative"
                } else {
                    "uncertain"
                };
                if relationship_type == "uncertain" {
                    return build_signal_with_pair_quality(
                        &self.name,
                        self.category,
                        candle.timestamp,
                        0.0,
                        0.0,
                        format!(
                            "relationship_uncertain;corr={:.4};trade_use=confirmation_only;standalone_actionable=false;{}",
                            correlation, window_quality_explanation
                        ),
                        Some(window_quality),
                    );
                }
                let (value, confidence, smt_explanation) = if let Some(event) = smt_event {
                    (
                        event.direction_value(),
                        (correlation.abs() * 0.85).clamp(0.10, 0.85),
                        event.explanation(),
                    )
                } else {
                    (
                        0.0,
                        0.10,
                        "smt_signal=none;fail_closed_reason=no_swing_confirmation_failure"
                            .to_string(),
                    )
                };

                build_signal_with_pair_quality(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "corr={:.4};relationship_type={};relationship_confidence={:.4};{};trade_use=confirmation_only;standalone_actionable=false;{}",
                        correlation,
                        relationship_type,
                        correlation.abs().min(1.0),
                        smt_explanation,
                        window_quality_explanation
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

fn collect_structure_ict_setup_matches(
    timeline: &[PdaEvent],
    primary_candles: &[Candle],
    context: &FactorContext<'_>,
    setup_horizon_bars: usize,
) -> Vec<SetupMatch> {
    let mut all_matches = Vec::new();

    let base_context = SetupContext {
        primary_candles: Some(primary_candles),
        paired_candles: context.paired_candles,
        ..SetupContext::default()
    };
    all_matches.extend(match_all_setups_extended(
        timeline,
        &base_context,
        setup_horizon_bars,
    ));

    for lower_events in [
        context.m1_events,
        context.m5_events,
        context.m15_events,
        context.m30_events,
        context.h1_events,
    ]
    .into_iter()
    .flatten()
    {
        all_matches.extend(match_all_setups_extended(
            lower_events,
            &base_context,
            setup_horizon_bars,
        ));
    }

    for (lower_events, higher_events) in [
        (context.m1_events, context.m5_events),
        (context.m5_events, context.m15_events),
        (context.m15_events, context.m30_events.or(context.h1_events)),
        (context.m30_events, context.h1_events),
        (context.h1_events, context.h4_events),
    ] {
        if let (Some(lower), Some(higher)) = (lower_events, higher_events) {
            let context = SetupContext {
                primary_candles: Some(primary_candles),
                paired_candles: context.paired_candles,
                htf_events: Some(higher),
                ..SetupContext::default()
            };
            all_matches.extend(match_all_setups_extended(
                lower,
                &context,
                setup_horizon_bars,
            ));
        }
    }

    if let Some(h1) = context.h1_events {
        let context = SetupContext {
            primary_candles: Some(primary_candles),
            paired_candles: context.paired_candles,
            htf_events: Some(h1),
            ..SetupContext::default()
        };
        all_matches.extend(match_all_setups_extended(
            timeline,
            &context,
            setup_horizon_bars,
        ));
    }

    if let Some(h4) = context.h4_events {
        let context = SetupContext {
            primary_candles: Some(primary_candles),
            paired_candles: context.paired_candles,
            htf_events: Some(h4),
            ..SetupContext::default()
        };
        all_matches.extend(match_all_setups_extended(
            timeline,
            &context,
            setup_horizon_bars,
        ));
    }
    if let Some(d1) = context.d1_events {
        let context = SetupContext {
            primary_candles: Some(primary_candles),
            paired_candles: context.paired_candles,
            htf_events: Some(d1),
            ..SetupContext::default()
        };
        all_matches.extend(match_all_setups_extended(
            timeline,
            &context,
            setup_horizon_bars,
        ));
    }
    if let (Some(w1), Some(d1)) = (context.w1_events, context.d1_events) {
        let context = SetupContext {
            primary_candles: Some(primary_candles),
            paired_candles: context.paired_candles,
            htf_events: Some(w1),
            mtf_events: Some(d1),
        };
        all_matches.extend(match_all_setups_extended(
            timeline,
            &context,
            setup_horizon_bars,
        ));
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut deduped = Vec::new();
    for item in all_matches {
        let key = format!(
            "{}::{:?}::{:?}",
            item.label(),
            item.direction,
            item.event_bars
        );
        if seen.insert(key) {
            deduped.push(item);
        }
    }
    deduped.sort_by_key(|item| (item.confirm_bar, item.label().to_string()));
    deduped
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

#[derive(Debug, Clone, PartialEq)]
struct CrossMarketSmtEvent {
    smt_signal: &'static str,
    base_swing_type: &'static str,
    base_level: f64,
    comparison_swing_type: &'static str,
    comparison_level: f64,
    raw_comparison_swing_type: &'static str,
    raw_comparison_level: f64,
    swept_side: &'static str,
    normalized_for_inverse_correlation: bool,
}

impl CrossMarketSmtEvent {
    fn direction_value(&self) -> f64 {
        match self.smt_signal {
            "bullish_smt" => 0.35,
            "bearish_smt" => -0.35,
            _ => 0.0,
        }
    }

    fn explanation(&self) -> String {
        format!(
            "smt_signal={};base_swing_type={};base_level={:.6};comparison_swing_type={};comparison_level={:.6};raw_comparison_swing_type={};raw_comparison_level={:.6};swept_side={};normalized_for_inverse_correlation={};fail_closed_reason=none",
            self.smt_signal,
            self.base_swing_type,
            self.base_level,
            self.comparison_swing_type,
            self.comparison_level,
            self.raw_comparison_swing_type,
            self.raw_comparison_level,
            self.swept_side,
            self.normalized_for_inverse_correlation
        )
    }
}

fn detect_cross_market_smt_event(
    base_window: &[Candle],
    comparison_window: &[Candle],
    normalize_comparison_for_inverse: bool,
) -> Option<CrossMarketSmtEvent> {
    let len = base_window.len().min(comparison_window.len());
    if len < 3 {
        return None;
    }
    let base_window = &base_window[..len];
    let comparison_window = &comparison_window[..len];
    let base_last = base_window.last()?;
    let comparison_last = comparison_window.last()?;
    if base_last.timestamp != comparison_last.timestamp {
        return None;
    }
    let base_prior = &base_window[..len - 1];
    let comparison_prior = &comparison_window[..len - 1];
    if base_prior.is_empty() || comparison_prior.is_empty() {
        return None;
    }

    let base_prev_high = base_prior
        .iter()
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let base_prev_low = base_prior
        .iter()
        .map(|candle| candle.low)
        .fold(f64::INFINITY, f64::min);
    let comparison_prev_high = comparison_prior
        .iter()
        .map(|candle| normalized_comparison_high(candle, normalize_comparison_for_inverse))
        .fold(f64::NEG_INFINITY, f64::max);
    let comparison_prev_low = comparison_prior
        .iter()
        .map(|candle| normalized_comparison_low(candle, normalize_comparison_for_inverse))
        .fold(f64::INFINITY, f64::min);

    let base_hh = base_last.high > base_prev_high;
    let base_ll = base_last.low < base_prev_low;
    let comparison_hh =
        normalized_comparison_high(comparison_last, normalize_comparison_for_inverse)
            > comparison_prev_high;
    let comparison_ll =
        normalized_comparison_low(comparison_last, normalize_comparison_for_inverse)
            < comparison_prev_low;

    if base_hh && !comparison_hh {
        Some(CrossMarketSmtEvent {
            smt_signal: "bearish_smt",
            base_swing_type: "HH",
            base_level: base_last.high,
            comparison_swing_type: "LH",
            comparison_level: comparison_high_level(
                comparison_last,
                normalize_comparison_for_inverse,
            ),
            raw_comparison_swing_type: raw_comparison_swing_type(
                "LH",
                normalize_comparison_for_inverse,
            ),
            raw_comparison_level: comparison_high_level(
                comparison_last,
                normalize_comparison_for_inverse,
            ),
            swept_side: "buy_side_liquidity",
            normalized_for_inverse_correlation: normalize_comparison_for_inverse,
        })
    } else if base_ll && !comparison_ll {
        Some(CrossMarketSmtEvent {
            smt_signal: "bullish_smt",
            base_swing_type: "LL",
            base_level: base_last.low,
            comparison_swing_type: "HL",
            comparison_level: comparison_low_level(
                comparison_last,
                normalize_comparison_for_inverse,
            ),
            raw_comparison_swing_type: raw_comparison_swing_type(
                "HL",
                normalize_comparison_for_inverse,
            ),
            raw_comparison_level: comparison_low_level(
                comparison_last,
                normalize_comparison_for_inverse,
            ),
            swept_side: "sell_side_liquidity",
            normalized_for_inverse_correlation: normalize_comparison_for_inverse,
        })
    } else {
        None
    }
}

fn normalized_comparison_high(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        -candle.low
    } else {
        candle.high
    }
}

fn normalized_comparison_low(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        -candle.high
    } else {
        candle.low
    }
}

fn comparison_high_level(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        candle.low
    } else {
        candle.high
    }
}

fn comparison_low_level(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        candle.high
    } else {
        candle.low
    }
}

fn raw_comparison_swing_type(normalized: &str, inverse: bool) -> &'static str {
    match (normalized, inverse) {
        ("HH", false) => "HH",
        ("LH", false) => "LH",
        ("LL", false) => "LL",
        ("HL", false) => "HL",
        ("HH", true) => "LL",
        ("LH", true) => "HL",
        ("LL", true) => "HH",
        ("HL", true) => "LH",
        _ => "unknown",
    }
}

fn normalize_signed(value: f64, cap: f64) -> f64 {
    if cap <= f64::EPSILON {
        0.0
    } else {
        (value / cap).clamp(-1.0, 1.0)
    }
}

// --- Hot-plug compute stubs for Families E, F, H ---

impl FactorDefinition {
    /// Family E: Crowding / Herding Execution Risk
    fn evaluate_crowding_herding(&self, candles: &[Candle]) -> Vec<FactorSignal> {
        let lookback = self.parameter("lookback", 20.0) as usize;
        let volume_spike_ratio = self.parameter("volume_spike_ratio", 2.0);
        let pc_weight = self.parameter("participation_concentration_weight", 0.40);
        let sp_weight = self.parameter("same_side_pressure_weight", 0.35);
        let cr_weight = self.parameter("crowding_relief_weight", 0.25);

        let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();

        candles
            .iter()
            .enumerate()
            .map(|(index, candle)| {
                let start = index.saturating_sub(lookback);
                let window_vol = &volumes[start..=index];

                // Participation concentration: current volume vs rolling median
                let median_vol = {
                    let mut sorted = window_vol.to_vec();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let mid = sorted.len() / 2;
                    if sorted.len().is_multiple_of(2) && sorted.len() > 1 {
                        (sorted[mid - 1] + sorted[mid]) / 2.0
                    } else {
                        sorted[mid]
                    }
                };
                let participation_concentration = if median_vol > f64::EPSILON {
                    (candle.volume / median_vol / volume_spike_ratio).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                // Same-side pressure: volume-weighted close direction over window
                let same_side_pressure = {
                    let mut bull_vol = 0.0f64;
                    let mut bear_vol = 0.0f64;
                    for (i, v) in window_vol.iter().enumerate() {
                        let c = &candles[start + i];
                        if c.close >= c.open {
                            bull_vol += v;
                        } else {
                            bear_vol += v;
                        }
                    }
                    let total = bull_vol + bear_vol;
                    if total > f64::EPSILON {
                        ((bull_vol - bear_vol) / total).clamp(-1.0, 1.0)
                    } else {
                        0.0
                    }
                };

                // Crowding relief: volume decay after spike (3-bar lookback)
                let crowding_relief = if index >= 3 && volumes[index] > f64::EPSILON {
                    let prev3_avg =
                        (volumes[index - 3] + volumes[index - 2] + volumes[index - 1]) / 3.0;
                    if prev3_avg > f64::EPSILON {
                        (1.0 - (candle.volume / prev3_avg) / 2.0).clamp(0.0, f64::MAX)
                    } else {
                        0.5
                    }
                } else {
                    0.5
                };

                let direction_score = same_side_pressure;
                let value = normalize_signed(
                    pc_weight * participation_concentration * direction_score.signum()
                        + sp_weight * same_side_pressure
                        - cr_weight * crowding_relief * direction_score.signum(),
                    1.0,
                );
                let confidence = (participation_concentration * pc_weight
                    + same_side_pressure.abs() * sp_weight
                    + crowding_relief * cr_weight)
                    .clamp(0.0, 1.0);

                build_signal(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "pc={:.3};sp={:.3};cr={:.3};vol_ratio={:.2}",
                        participation_concentration,
                        same_side_pressure,
                        crowding_relief,
                        if median_vol > f64::EPSILON {
                            candle.volume / median_vol
                        } else {
                            0.0
                        }
                    ),
                )
            })
            .collect()
    }

    /// Family F: Spectral Rhythm / Chaos
    fn evaluate_spectral_rhythm(&self, candles: &[Candle]) -> Vec<FactorSignal> {
        let lookback = self.parameter("lookback", 64.0) as usize;
        let se_weight = self.parameter("spectral_entropy_weight", 0.45);
        let ce_weight = self.parameter("cycle_energy_weight", 0.35);
        let rs_weight = self.parameter("rhythm_stability_weight", 0.20);

        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();

        candles
            .iter()
            .enumerate()
            .map(|(index, candle)| {
                let start = index.saturating_sub(lookback);
                let window = &closes[start..=index];

                // Spectral entropy: normalized log-variance of returns
                let spectral_entropy = {
                    let returns: Vec<f64> = window.windows(2).map(|w| w[1] - w[0]).collect();
                    if returns.len() < 4 {
                        0.5
                    } else {
                        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
                        let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
                            / returns.len() as f64;
                        if variance > f64::EPSILON {
                            (1.0 + variance.ln() / 10.0).clamp(0.0, 1.0)
                        } else {
                            0.0
                        }
                    }
                };

                // Dominant cycle energy: longest same-direction run / lookback
                let cycle_energy = {
                    let returns: Vec<f64> = window.windows(2).map(|w| w[1] - w[0]).collect();
                    if returns.is_empty() {
                        0.0
                    } else {
                        let mut max_run = 1usize;
                        let mut current_run = 1usize;
                        for i in 1..returns.len() {
                            if returns[i].signum() == returns[i - 1].signum()
                                && returns[i].abs() > f64::EPSILON
                            {
                                current_run += 1;
                                max_run = max_run.max(current_run);
                            } else {
                                current_run = 1;
                            }
                        }
                        (max_run as f64 / lookback as f64).min(1.0)
                    }
                };

                // Rhythm stability: autocorrelation lag-1 of returns
                let rhythm_stability = {
                    let returns: Vec<f64> = window.windows(2).map(|w| w[1] - w[0]).collect();
                    if returns.len() < 4 {
                        0.5
                    } else {
                        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
                        let var = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>();
                        if var < f64::EPSILON {
                            0.5
                        } else {
                            let cov: f64 = returns[..returns.len() - 1]
                                .iter()
                                .zip(&returns[1..])
                                .map(|(a, b)| (a - mean) * (b - mean))
                                .sum();
                            (cov / var).clamp(-1.0, 1.0).abs()
                        }
                    }
                };

                // High entropy = chaotic = negative for execution readiness
                let value = normalize_signed(
                    -se_weight * spectral_entropy
                        + ce_weight
                            * cycle_energy
                            * if candle.close >= candle.open {
                                1.0
                            } else {
                                -1.0
                            }
                        + rs_weight
                            * rhythm_stability
                            * if candle.close >= candle.open {
                                1.0
                            } else {
                                -1.0
                            },
                    1.0,
                );
                let confidence = (se_weight * spectral_entropy
                    + ce_weight * cycle_energy
                    + rs_weight * rhythm_stability)
                    .clamp(0.0, 1.0);

                build_signal(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "se={:.3};ce={:.3};rs={:.3}",
                        spectral_entropy, cycle_energy, rhythm_stability
                    ),
                )
            })
            .collect()
    }

    /// Family H: Session / Liquidity Window Quality
    fn evaluate_session_liquidity(&self, candles: &[Candle]) -> Vec<FactorSignal> {
        let sq_weight = self.parameter("session_quality_weight", 0.40);
        let kz_weight = self.parameter("kill_zone_weight", 0.35);
        let tr_weight = self.parameter("transition_risk_weight", 0.25);
        let lookback = self.parameter("lookback", 20.0) as usize;

        let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();

        candles
            .iter()
            .enumerate()
            .map(|(index, candle)| {
                // Session participation quality: current volume relative to rolling average
                let session_quality = {
                    let start = index.saturating_sub(lookback);
                    let window_vol = &volumes[start..=index];
                    let avg = window_vol.iter().sum::<f64>() / window_vol.len() as f64;
                    if avg > f64::EPSILON {
                        (candle.volume / avg) / 3.0
                    } else {
                        0.0
                    }
                };

                // Kill-zone alignment: hour-of-day proxy (UTC)
                // US RTH kill zones ~14-15, 19-20 UTC; London ~7-8, 12-13 UTC
                let kill_zone_alignment = {
                    let hour = candle
                        .timestamp
                        .format("%H")
                        .to_string()
                        .parse::<u32>()
                        .unwrap_or(12);
                    match hour {
                        7 | 8 | 12 | 13 | 14 | 15 | 19 | 20 => 0.9,
                        9..=11 | 16..=18 => 0.6,
                        _ => 0.3,
                    }
                };

                // Session transition risk: near session boundaries
                let session_transition_risk = {
                    let hour = candle
                        .timestamp
                        .format("%H")
                        .to_string()
                        .parse::<u32>()
                        .unwrap_or(12);
                    match hour {
                        7 | 8 | 13 | 14 | 19 | 20 => 0.7,
                        _ => 0.2,
                    }
                };

                let value = normalize_signed(
                    sq_weight
                        * session_quality
                        * if candle.close >= candle.open {
                            1.0
                        } else {
                            -1.0
                        }
                        + kz_weight
                            * kill_zone_alignment
                            * if candle.close >= candle.open {
                                1.0
                            } else {
                                -1.0
                            }
                        - tr_weight * session_transition_risk,
                    1.0,
                );
                let confidence = (sq_weight * session_quality
                    + kz_weight * kill_zone_alignment
                    + tr_weight * session_transition_risk)
                    .clamp(0.0, 1.0);

                build_signal(
                    &self.name,
                    self.category,
                    candle.timestamp,
                    value,
                    confidence,
                    format!(
                        "sq={:.3};kz={:.3};tr={:.3}",
                        session_quality, kill_zone_alignment, session_transition_risk
                    ),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pda_timeline::{CanonicalSetupKind, PdaEventKind};
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

    fn slope_candles(count: usize, start_price: f64, step: f64) -> Vec<Candle> {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        (0..count)
            .map(|index| {
                let base = start_price + index as f64 * step;
                Candle {
                    timestamp: start + Duration::minutes(index as i64),
                    open: base,
                    high: base + 0.6,
                    low: base - 0.5,
                    close: base + 0.2,
                    volume: 1_000.0 + index as f64,
                }
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
            ..FactorContext::default()
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
            ..FactorContext::default()
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
    fn test_cross_market_smt_signal_window_marks_flat_pair_observe_only() {
        let definition = FactorDefinition::cross_market_smt();
        let candles = candles(80);
        let paired = flat_candles(80, 200.0);
        let context = FactorContext {
            paired_candles: Some(&paired),
            ..FactorContext::default()
        };

        let series = definition.evaluate(&candles, &context).unwrap();
        let target = &series.signals[52];

        assert_eq!(target.value, 0.0);
        assert_eq!(target.confidence, 0.05);
        assert!(target.explanation.contains("status=valid_but_flat"));
        assert!(target.explanation.contains("quality_tier=flat"));
        assert!(target.explanation.contains("aligned_length=32"));
    }

    #[test]
    fn test_cross_market_smt_does_not_treat_relative_strength_as_smt() {
        let definition = FactorDefinition::cross_market_smt();
        let primary = slope_candles(80, 100.0, 0.8);
        let paired = slope_candles(80, 200.0, 0.2);
        let context = FactorContext {
            paired_candles: Some(&paired),
            ..FactorContext::default()
        };

        let series = definition.evaluate(&primary, &context).unwrap();
        let latest = series.latest_signal().unwrap();

        assert_eq!(latest.value, 0.0);
        assert_eq!(latest.direction, Direction::Neutral);
        assert!(latest.explanation.contains("smt_signal=none"));
        assert!(latest
            .explanation
            .contains("fail_closed_reason=no_swing_confirmation_failure"));
        assert!(!latest.explanation.contains("relative_strength"));
    }

    #[test]
    fn test_cross_market_smt_requires_same_event_swing_confirmation_failure() {
        let definition = FactorDefinition::cross_market_smt();
        let mut primary = slope_candles(80, 100.0, 0.4);
        let mut paired = slope_candles(80, 200.0, 0.4);
        let prior_pair_high = paired[..79]
            .iter()
            .map(|candle| candle.high)
            .fold(f64::NEG_INFINITY, f64::max);
        primary[79].high = primary[..79]
            .iter()
            .map(|candle| candle.high)
            .fold(f64::NEG_INFINITY, f64::max)
            + 2.0;
        primary[79].close = primary[79].high - 0.2;
        paired[79].high = prior_pair_high - 0.25;
        paired[79].close = paired[79].high - 0.2;
        let context = FactorContext {
            paired_candles: Some(&paired),
            ..FactorContext::default()
        };

        let series = definition.evaluate(&primary, &context).unwrap();
        let latest = series.latest_signal().unwrap();

        assert_eq!(latest.direction, Direction::Bear);
        assert!(latest.value < 0.0);
        assert!(latest.explanation.contains("smt_signal=bearish_smt"));
        assert!(latest.explanation.contains("base_swing_type=HH"));
        assert!(latest.explanation.contains("comparison_swing_type=LH"));
        assert!(latest.explanation.contains("swept_side=buy_side_liquidity"));
        assert!(latest.explanation.contains("trade_use=confirmation_only"));
        assert!(latest.explanation.contains("standalone_actionable=false"));
        assert!(latest.explanation.contains("base_level="));
        assert!(latest.explanation.contains("comparison_level="));
    }

    #[test]
    fn test_structure_ict_explanation_includes_setup_hits_fields() {
        // P1b-2: every structure_ict signal must surface the new
        // canonical-setup tally so analyze / factor_research can
        // render it without re-running detection.
        let factor = FactorDefinition::structure_ict();
        let candles = candles(80);
        let series = factor
            .evaluate(&candles, &FactorContext::default())
            .unwrap();
        assert_eq!(series.signals.len(), candles.len());
        for signal in &series.signals {
            assert!(
                signal.explanation.contains("bull_setup_hits="),
                "structure_ict explanation missing bull_setup_hits: {}",
                signal.explanation
            );
            assert!(
                signal.explanation.contains("bear_setup_hits="),
                "structure_ict explanation missing bear_setup_hits: {}",
                signal.explanation
            );
        }
    }

    #[test]
    fn test_structure_ict_setup_matches_helper_is_deterministic() {
        // The convenience helper must agree across calls and never
        // return a match whose confirm_bar exceeds the candle count.
        let factor = FactorDefinition::structure_ict();
        let candles = candles(80);
        let a = factor.structure_ict_setup_matches(&candles);
        let b = factor.structure_ict_setup_matches(&candles);
        assert_eq!(a, b);
        for m in &a {
            assert!(m.confirm_bar < candles.len());
            assert!(m.anchor_bar <= m.confirm_bar);
        }
    }

    #[test]
    fn test_structure_ict_setup_matches_with_context_activates_cross_tf_and_smt_paths() {
        let series_candles = candles(80);
        let paired = candles(80);
        let base_ts = series_candles[10].timestamp;
        let h4_events = vec![
            PdaEvent::new(PdaEventKind::MarketStructureShift, 3, Direction::Bull)
                .with_timestamp(base_ts),
        ];
        let d1_events = vec![
            PdaEvent::new(PdaEventKind::LiquiditySweep, 2, Direction::Bull)
                .with_timestamp(base_ts - Duration::hours(1)),
            PdaEvent::new(PdaEventKind::MarketStructureShift, 3, Direction::Bear)
                .with_timestamp(base_ts + Duration::minutes(30)),
        ];
        let w1_events = vec![
            PdaEvent::new(PdaEventKind::LiquiditySweep, 1, Direction::Bull)
                .with_timestamp(base_ts - Duration::hours(2)),
        ];
        let ltf_context_events = vec![
            PdaEvent::new(PdaEventKind::FairValueGap, 12, Direction::Bull).with_timestamp(base_ts),
            PdaEvent::new(PdaEventKind::MarketStructureShift, 15, Direction::Bear)
                .with_timestamp(base_ts + Duration::hours(1)),
            PdaEvent::new(PdaEventKind::FairValueGap, 16, Direction::Bear)
                .with_timestamp(base_ts + Duration::hours(2)),
        ];
        let context = FactorContext {
            paired_candles: Some(&paired),
            h4_events: Some(&h4_events),
            d1_events: Some(&d1_events),
            w1_events: Some(&w1_events),
            ..FactorContext::default()
        };
        let matches =
            collect_structure_ict_setup_matches(&ltf_context_events, &series_candles, &context, 30);

        assert!(matches
            .iter()
            .any(|item| item.kind == CanonicalSetupKind::HtfMssLtfFvg));
        assert!(matches
            .iter()
            .any(|item| item.kind == CanonicalSetupKind::DailyHighSweepLtfMssFvg));
        assert!(matches
            .iter()
            .any(|item| item.kind == CanonicalSetupKind::WeeklyOpenSweepDailyMss));
    }
}
