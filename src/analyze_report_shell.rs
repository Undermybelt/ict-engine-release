use serde::Serialize;

use crate::agent::AgentPromptPack;
use crate::analyze_sections::AnalyzeSections;
use crate::application::orchestration::ExecutionTriage;
use crate::domain::execution::ExecutionArtifact;
use crate::state::{
    AgentActionPlan, AgentContextBundle, AgentContextBundleMinimal, CommandRecommendations,
    DatasetComparability, DecisionHistorySummary, DecisionThresholds, FactorFamilyDecision,
    FactorFamilyDiff, FactorFamilyHistory, FactorFamilyOutcome, FactorIterationPrompt,
    FeedbackHistorySummary, PersistedFactorRanking, PromotionDecision, RollbackRecommendation,
    RunProvenance, WorkflowSnapshot, WorkflowState,
};
use crate::types::TradePlan;

#[derive(Debug, Serialize)]
pub struct AnalyzeReport {
    pub symbol: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(flatten)]
    pub analysis: AnalyzeSections,
    pub meta: AnalyzeMeta,
    pub supporting: AnalyzeSupporting,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeMeta {
    pub state_dir: String,
    pub bars: AnalyzeBars,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<crate::state::LiveDataSourceProvenance>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeSupporting {
    pub model_state: AnalyzeModelState,
    pub provenance: RunProvenance,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub labels: AnalyzeLabels,
    pub ict: AnalyzeIctSummary,
    pub entry_quality: AnalyzeEntryQualitySummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auxiliary: Option<crate::data::realtime::openalice::AuxiliaryMarketEvidence>,
    pub decision: crate::planner::ProbabilisticDecisionSnapshot,
    pub trade_outcome: AnalyzeTradeOutcomeSummary,
    pub factor_diagnostics: crate::factor_lab::FactorDiagnostics,
    pub pre_bayes_evidence_filter: crate::state::PreBayesEvidenceFilter,
    pub pre_bayes_entry_quality_bridge: crate::state::PreBayesEntryQualityBridge,
    pub objective_jump_weight: Option<f64>,
    pub canonical_belief_report: crate::reporting::belief::BeliefReportPacket,
    pub decision_thresholds: DecisionThresholds,
    pub factor_ranking: Vec<PersistedFactorRanking>,
    pub factor_iteration_queue: Vec<FactorIterationPrompt>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    pub factor_family_history: Vec<FactorFamilyHistory>,
    pub decision_history_summary: DecisionHistorySummary,
    pub agent_action_plan: AgentActionPlan,
    pub workflow_state: WorkflowState,
    pub agent_context_bundle: AgentContextBundle,
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    pub recommended_commands: CommandRecommendations,
    pub recommended_next_command: String,
    pub dataset_comparability: DatasetComparability,
    pub decision_hint: String,
    pub artifact_action_summary: Vec<String>,
    pub artifact_decision_summary: crate::state::ArtifactDecisionSummary,
    pub artifact_decision_section: crate::state::ArtifactDecisionSection,
    pub agent_prompts: AgentPromptPack,
    pub feedback_history_summary: FeedbackHistorySummary,
    pub multi_timeframe_summary: Vec<String>,
    pub raw_trade_plan: TradePlan,
    pub workflow_snapshot: WorkflowSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_orchestration_trace: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_artifact: Option<ExecutionArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_triage: Option<ExecutionTriage>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeBars {
    pub htf: usize,
    pub mtf: usize,
    pub ltf: usize,
    pub observations: usize,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeModelState {
    pub hmm_state: String,
    pub log_likelihood: f64,
    pub viterbi_log_likelihood: f64,
    pub regime_probs: crate::types::RegimeProbs,
    pub evidence_policy: String,
    pub canonical_belief_engine: String,
    pub canonical_shadow_status: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeLabels {
    pub regime_label: String,
    pub liquidity_label: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeIctSummary {
    pub total_sweeps: usize,
    pub total_fvgs: usize,
    pub mtf_open_fvgs: usize,
    pub mtf_untested_obs: usize,
    pub ict_role: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeTradeOutcomeSummary {
    pub base: std::collections::BTreeMap<String, f64>,
    pub long: std::collections::BTreeMap<String, f64>,
    pub short: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeEntryQualitySummary {
    pub base: std::collections::BTreeMap<String, f64>,
    pub long: std::collections::BTreeMap<String, f64>,
    pub short: std::collections::BTreeMap<String, f64>,
    pub selected_state: String,
}
