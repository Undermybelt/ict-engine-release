use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RegimeKey {
    Trend,
    Range,
    Stress,
    Transition,
}

impl RegimeKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trend => "trend",
            Self::Range => "range",
            Self::Stress => "stress",
            Self::Transition => "transition",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeSegmentationPacket {
    pub method: String,
    pub segmentation_version: String,
    pub active_regime_cluster: Option<String>,
    pub transition_hazard: Option<f64>,
    pub regime_membership: BTreeMap<String, f64>,
    pub feature_attribution: BTreeMap<String, f64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuralBreakPacket {
    pub method: String,
    pub break_family: String,
    pub detected: bool,
    pub break_score: Option<f64>,
    pub break_index: Option<usize>,
    pub lookback_window: Option<usize>,
    pub affected_features: Vec<String>,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeValidationPacket {
    pub validation_scope: String,
    pub segmentation_consistency: Option<f64>,
    pub hindsight_risk_flags: Vec<String>,
    pub abstain_recommended: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeFeatures {
    pub market_regime_label: Option<String>,
    pub volatility_regime_label: Option<String>,
    pub liquidity_regime_label: Option<String>,
    pub stress_score: Option<f64>,
    pub transition_score: Option<f64>,
    pub evidence: Vec<String>,
    pub segmentation_context: Option<RegimeSegmentationPacket>,
    pub structural_break_context: Option<StructuralBreakPacket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimePosterior {
    pub active_regime: Option<String>,
    pub market_family: Option<String>,
    pub market_behavior_profile: Option<String>,
    pub jump_model: Option<JumpModelRegimeSummary>,
    pub probabilities: BTreeMap<String, f64>,
    pub confidence: Option<f64>,
    pub credible_intervals: BTreeMap<String, super::super::belief::CredibleInterval>,
    pub evidence: Vec<String>,
    pub regime_validation: Option<RegimeValidationPacket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JumpModelRegimeSummary {
    pub active_state: String,
    pub confidence: f64,
    pub transition_risk: f64,
    pub market_jump_weight: f64,
    pub state_probabilities: BTreeMap<String, f64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeDisagreementSummary {
    pub hmm_active_regime: Option<String>,
    pub jump_active_state: Option<String>,
    pub aligned: bool,
    pub disagreement_score: f64,
    pub gate_bias: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeGateDecision {
    pub selected_regime: String,
    pub selected_subgraph: String,
    pub market_family: Option<String>,
    pub jump_weight: Option<f64>,
    pub rationale: Vec<String>,
}
