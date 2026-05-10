use crate::bbn::temporal::ParticleBeliefSummary;
use crate::domain::regime::{
    JumpModelRegimeSummary, RegimeDisagreementSummary, RegimeFeatures, RegimeGateDecision,
    RegimePosterior,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConformalUncertaintyPacket {
    pub method: String,
    pub target: String,
    pub nominal_coverage: f64,
    pub empirical_coverage: Option<f64>,
    pub interval_width: Option<f64>,
    pub nonconformity_score: Option<f64>,
    pub abstain_threshold: Option<f64>,
    pub abstain: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MicrostructureContextPacket {
    pub source: String,
    pub granularity: String,
    pub usable_as_evidence: bool,
    pub prior_adjuster_bias: Option<f64>,
    pub transition_bias: Option<f64>,
    pub setup_quality_score: Option<f64>,
    pub context_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketPolicyPacket {
    pub market_family: Option<String>,
    pub market_behavior_profile: Option<String>,
    pub policy_mode: String,
    pub evidence_reliability: BTreeMap<String, f64>,
    pub abstention_bias: Option<f64>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefEvidencePacket {
    pub symbol: String,
    pub market: Option<String>,
    pub timestamp: Option<String>,
    pub entry_logic_id: Option<String>,
    pub logic_family: Option<String>,
    pub regime_features: RegimeFeatures,
    pub market_evidence: Vec<String>,
    pub factor_evidence: Vec<String>,
    pub timed_pda_summary: BTreeMap<String, String>,
    pub multi_timeframe_evidence: BTreeMap<String, String>,
    pub evidence_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub uses_soft_evidence: bool,
    #[serde(default)]
    pub soft_market_regime_distribution: BTreeMap<String, f64>,
    #[serde(default)]
    pub soft_liquidity_context_distribution: BTreeMap<String, f64>,
    #[serde(default)]
    pub soft_factor_alignment_distribution: BTreeMap<String, f64>,
    #[serde(default)]
    pub soft_factor_uncertainty_distribution: BTreeMap<String, f64>,
    #[serde(default)]
    pub soft_multi_timeframe_resonance_distribution: BTreeMap<String, f64>,
    pub microstructure_context: Option<MicrostructureContextPacket>,
    pub market_policy: Option<MarketPolicyPacket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CredibleInterval {
    pub node_id: String,
    pub state: String,
    pub lower: f64,
    pub median: f64,
    pub upper: f64,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefNodePosteriorSnapshot {
    pub node_id: String,
    pub top_state: String,
    pub top_probability: f64,
    pub entropy: f64,
    pub probabilities: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngineTrace {
    pub primary_engine: String,
    pub shadow_engine: Option<String>,
    pub sample_count: Option<usize>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShadowComparisonSummary {
    pub top_state_match_rate: BTreeMap<String, f64>,
    pub kl_divergence: BTreeMap<String, f64>,
    pub interval_overlap: BTreeMap<String, f64>,
    pub recommendation_drift: Vec<String>,
    pub status: String,
    pub summary_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObjectiveMarketCredibilityShrink {
    pub objective: Option<String>,
    pub market_family: Option<String>,
    pub credibility_score: f64,
    pub shrink_weight: f64,
    pub shrink_triggered: bool,
    pub hard_blocked: bool,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeCompanionPacket {
    pub jump_model: Option<JumpModelRegimeSummary>,
    pub disagreement: Option<RegimeDisagreementSummary>,
    pub objective_market_credibility_shrink: Option<ObjectiveMarketCredibilityShrink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefReportPacket {
    pub regime_posterior: RegimePosterior,
    pub regime_companion: RegimeCompanionPacket,
    pub gate_decision: RegimeGateDecision,
    pub belief_posteriors: Vec<BeliefNodePosteriorSnapshot>,
    pub credible_intervals: Vec<CredibleInterval>,
    pub strategy_recommendation: crate::domain::strategy::StrategyRecommendation,
    pub market_family: Option<String>,
    pub market_behavior_profile: Option<String>,
    pub selected_market_subgraph: Option<String>,
    pub engine_trace: EngineTrace,
    pub temporal_summary: Option<ParticleBeliefSummary>,
    pub shadow_comparison: Option<ShadowComparisonSummary>,
    pub conformal_uncertainty: Vec<ConformalUncertaintyPacket>,
    pub market_policy: Option<MarketPolicyPacket>,
}
