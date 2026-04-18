use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::types::{Direction, FactorIC, Regime};

pub const LEARNING_STATE_FILE: &str = "learning_state.json";
pub const TRADE_HISTORY_FILE: &str = "trade_history.json";
pub const TRAIN_RUNS_FILE: &str = "train_runs.json";
pub const ANALYZE_RUNS_FILE: &str = "analyze_runs.json";
pub const RESEARCH_RUNS_FILE: &str = "research_runs.json";
pub const FACTOR_MUTATION_RUNS_FILE: &str = "factor_mutation_runs.json";
pub const FACTOR_AUTORESEARCH_SESSIONS_FILE: &str = "factor_autoresearch_sessions.json";
pub const FACTOR_AUTORESEARCH_ATTEMPTS_FILE: &str = "factor_autoresearch_attempts.json";
pub const FACTOR_AUTORESEARCH_LIVE_FILE: &str = "factor_autoresearch_live.json";
pub const FACTOR_AUTORESEARCH_FINAL_FILE: &str = "factor_autoresearch_final.json";
pub const BACKTEST_RUNS_FILE: &str = "backtest_runs.json";
pub const UPDATE_RUNS_FILE: &str = "update_runs.json";
pub const WORKFLOW_SNAPSHOT_FILE: &str = "workflow_snapshot.json";
pub const COMPACT_SNAPSHOT_FILE: &str = "compact_snapshot.json";
pub const PRE_BAYES_POLICY_HISTORY_FILE: &str = "pre_bayes_policy_history.json";
pub const PENDING_UPDATE_ARTIFACT_FILE: &str = "pending_update_feedback.json";
pub const PENDING_UPDATE_HISTORY_FILE: &str = "pending_update_history.json";
pub const EXECUTION_CANDIDATE_FILE: &str = "execution_candidate.json";
pub const EXECUTION_CANDIDATE_HISTORY_FILE: &str = "execution_candidate_history.json";
pub const ENSEMBLE_VOTE_FILE: &str = "ensemble_vote.json";
pub const ENSEMBLE_VOTE_HISTORY_FILE: &str = "ensemble_vote_history.json";
pub const ENSEMBLE_EXECUTOR_SCORECARDS_FILE: &str = "ensemble_executor_scorecards.json";
pub const ARTIFACT_LEDGER_FILE: &str = "artifact_ledger.json";

/// State persistence types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub hmm_params: Option<crate::types::HMMParams>,
    pub cascade_config: Option<crate::bayesian::CascadeConfig>,
    pub beta_learner: Option<crate::bayesian::CascadeBetaLearner>,
    pub sv_params: Option<SVParams>,
    pub learning_state: Option<LearningState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SVParams {
    pub mu: f64,
    pub phi: f64,
    pub sigma_eta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LearningState {
    pub factor_profiles: BTreeMap<String, FactorLearningProfile>,
    pub factor_rankings: Vec<PersistedFactorRanking>,
    pub feedback_history: Vec<FeedbackRecord>,
    pub last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorLearningProfile {
    pub enabled: bool,
    pub base_weight: f64,
    pub posterior_reliability: f64,
    pub last_ic: f64,
    pub last_ir: f64,
    pub last_backtest_return: f64,
    pub last_stability: f64,
    pub parameters: BTreeMap<String, f64>,
    pub regime_stats: BTreeMap<String, RegimeFactorStats>,
}

impl Default for FactorLearningProfile {
    fn default() -> Self {
        Self {
            enabled: true,
            base_weight: 0.2,
            posterior_reliability: 0.5,
            last_ic: 0.0,
            last_ir: 0.0,
            last_backtest_return: 0.0,
            last_stability: 0.0,
            parameters: BTreeMap::new(),
            regime_stats: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeFactorStats {
    pub observations: usize,
    pub wins: usize,
    pub avg_pnl: f64,
    pub multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub source: String,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub trade_id: Option<String>,
    #[serde(default)]
    pub prompt_version: Option<String>,
    #[serde(default)]
    pub factor_version: Option<String>,
    #[serde(default)]
    pub data_fingerprint: Option<String>,
    pub factors_used: Vec<FeedbackFactorUsage>,
    pub model_probabilities_before_trade: ModelProbabilitySnapshot,
    pub realized_outcome: String,
    pub pnl: f64,
    pub regime_at_entry: Regime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdateArtifactDiff {
    pub previous_artifact_id: Option<String>,
    pub changed_fields: Vec<String>,
    pub quality_delta: i32,
    pub exact_duplicate: bool,
    pub comparable_same_data: bool,
    pub comparable_same_factor_version: bool,
    pub comparable_same_prompt_version: bool,
    pub selected_probability_delta: f64,
    pub top_factor_score_delta: f64,
    pub avg_family_score_delta: f64,
}

impl Default for PendingUpdateArtifactDiff {
    fn default() -> Self {
        Self {
            previous_artifact_id: None,
            changed_fields: Vec::new(),
            quality_delta: 0,
            exact_duplicate: false,
            comparable_same_data: false,
            comparable_same_factor_version: false,
            comparable_same_prompt_version: false,
            selected_probability_delta: 0.0,
            top_factor_score_delta: 0.0,
            avg_family_score_delta: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PendingUpdateArtifactDecision {
    pub status: String,
    pub reason: String,
    pub supersedes_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdateArtifact {
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub source_command: String,
    pub provenance: RunProvenance,
    pub decision_hint: String,
    pub entry_quality: String,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub selected_win_probability: f64,
    pub top_factor_score: f64,
    pub avg_family_score: f64,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub family_scores: BTreeMap<String, f64>,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub review_rule_snapshot: PendingUpdateReviewRules,
    #[serde(default)]
    pub pre_bayes_evidence_filter: Option<PreBayesEvidenceFilter>,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    pub template_feedback: FeedbackRecord,
    #[serde(default)]
    pub diff_from_previous: PendingUpdateArtifactDiff,
    #[serde(default)]
    pub review_decision: PendingUpdateArtifactDecision,
}

impl Default for PendingUpdateArtifact {
    fn default() -> Self {
        Self {
            artifact_id: String::new(),
            version: 0,
            generated_at: Utc::now(),
            symbol: String::new(),
            source_phase: String::new(),
            source_run_id: None,
            source_command: String::new(),
            provenance: RunProvenance::default(),
            decision_hint: String::new(),
            entry_quality: String::new(),
            factor_alignment: String::new(),
            factor_uncertainty: String::new(),
            selected_win_probability: 0.0,
            top_factor_score: 0.0,
            avg_family_score: 0.0,
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::new(),
            review_rule_version: String::new(),
            review_rule_snapshot: PendingUpdateReviewRules::default(),
            pre_bayes_evidence_filter: None,
            pre_bayes_entry_quality_bridge: None,
            multi_timeframe_summary: Vec::new(),
            template_feedback: FeedbackRecord {
                timestamp: Utc::now(),
                symbol: String::new(),
                source: String::new(),
                run_id: None,
                trade_id: None,
                prompt_version: None,
                factor_version: None,
                data_fingerprint: None,
                factors_used: Vec::new(),
                model_probabilities_before_trade: ModelProbabilitySnapshot {
                    selected_direction: Direction::Neutral,
                    selected_probability: 0.0,
                    long_score: 0.0,
                    short_score: 0.0,
                    win_prob_long: 0.0,
                    win_prob_short: 0.0,
                    uncertainty: 0.0,
                },
                realized_outcome: String::new(),
                pnl: 0.0,
                regime_at_entry: Regime::ManipulationExpansion,
            },
            diff_from_previous: PendingUpdateArtifactDiff::default(),
            review_decision: PendingUpdateArtifactDecision::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PendingUpdateArtifactSummary {
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub path: String,
    pub decision_hint: String,
    pub entry_quality: String,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub review_status: String,
    #[serde(default)]
    pub review_reason: String,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: String,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub quality_delta: i32,
    #[serde(default)]
    pub selected_probability_delta: f64,
    #[serde(default)]
    pub top_factor_score_delta: f64,
    #[serde(default)]
    pub avg_family_score_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCandidateArtifactDiff {
    pub previous_artifact_id: Option<String>,
    pub changed_fields: Vec<String>,
    pub posterior_delta: f64,
    pub win_probability_delta: f64,
    pub entry_delta: f64,
    pub exact_duplicate: bool,
}

impl Default for ExecutionCandidateArtifactDiff {
    fn default() -> Self {
        Self {
            previous_artifact_id: None,
            changed_fields: Vec::new(),
            posterior_delta: 0.0,
            win_probability_delta: 0.0,
            entry_delta: 0.0,
            exact_duplicate: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionCandidateArtifactDecision {
    pub status: String,
    pub reason: String,
    pub supersedes_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCandidateArtifact {
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub provenance: RunProvenance,
    pub decision_hint: String,
    pub selected_direction: Direction,
    pub trade_direction: Direction,
    pub actionable: bool,
    pub entry: f64,
    pub stop_loss: f64,
    pub take_profits: Vec<f64>,
    pub posterior: f64,
    pub win_probability: f64,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub candidate_status: String,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub family_scores: BTreeMap<String, f64>,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub review_rule_snapshot: ExecutionCandidateReviewRules,
    #[serde(default)]
    pub pre_bayes_evidence_filter: Option<PreBayesEvidenceFilter>,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub executor_scorecards: Vec<EnsembleExecutorScorecard>,
    #[serde(default)]
    pub diff_from_previous: ExecutionCandidateArtifactDiff,
    #[serde(default)]
    pub review_decision: ExecutionCandidateArtifactDecision,
}

impl Default for ExecutionCandidateArtifact {
    fn default() -> Self {
        Self {
            artifact_id: String::new(),
            version: 0,
            generated_at: Utc::now(),
            symbol: String::new(),
            source_phase: String::new(),
            source_run_id: None,
            provenance: RunProvenance::default(),
            decision_hint: String::new(),
            selected_direction: Direction::Neutral,
            trade_direction: Direction::Neutral,
            actionable: false,
            entry: 0.0,
            stop_loss: 0.0,
            take_profits: Vec::new(),
            posterior: 0.0,
            win_probability: 0.0,
            factor_alignment: String::new(),
            factor_uncertainty: String::new(),
            candidate_status: String::new(),
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::new(),
            review_rule_version: String::new(),
            review_rule_snapshot: ExecutionCandidateReviewRules::default(),
            pre_bayes_evidence_filter: None,
            pre_bayes_entry_quality_bridge: None,
            multi_timeframe_summary: Vec::new(),
            executor_scorecards: Vec::new(),
            diff_from_previous: ExecutionCandidateArtifactDiff::default(),
            review_decision: ExecutionCandidateArtifactDecision::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionCandidateArtifactSummary {
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub path: String,
    pub trade_direction: String,
    pub actionable: bool,
    pub candidate_status: String,
    pub decision_hint: String,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub review_status: String,
    #[serde(default)]
    pub review_reason: String,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: String,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub posterior_delta: f64,
    #[serde(default)]
    pub win_probability_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactLedgerEntry {
    pub entry_id: String,
    pub artifact_kind: String,
    pub artifact_id: String,
    pub version: usize,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub path: String,
    pub status: String,
    pub promote_candidate: bool,
    pub actionable: bool,
    pub decision_hint: String,
    pub review_reason: String,
    #[serde(default)]
    pub review_rule_version: String,
    #[serde(default)]
    pub top_factor_name: Option<String>,
    #[serde(default)]
    pub top_factor_action: Option<String>,
    #[serde(default)]
    pub family_scores: BTreeMap<String, f64>,
    #[serde(default)]
    pub supersedes_artifact_id: Option<String>,
    #[serde(default)]
    pub quality_score: i32,
    pub consumed_by_update_run_id: Option<String>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub consumed_outcome: Option<String>,
    #[serde(default)]
    pub regraded_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub consumption_regrade_status: Option<String>,
    #[serde(default)]
    pub consumption_regrade_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdateReviewRules {
    pub min_probability_improvement: f64,
    pub min_top_factor_score_improvement: f64,
    pub min_avg_family_score_improvement: f64,
    pub require_same_data: bool,
    pub require_same_factor_version: bool,
    pub require_same_prompt_version: bool,
}

impl Default for PendingUpdateReviewRules {
    fn default() -> Self {
        Self {
            min_probability_improvement: 0.03,
            min_top_factor_score_improvement: 0.05,
            min_avg_family_score_improvement: 0.03,
            require_same_data: true,
            require_same_factor_version: true,
            require_same_prompt_version: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactReviewRuleSources {
    pub pending_update: BTreeMap<String, String>,
    pub execution_candidate: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCandidateReviewRules {
    pub min_posterior_improvement: f64,
    pub min_win_probability_improvement: f64,
    pub require_same_data: bool,
    pub require_same_factor_version: bool,
}

impl Default for ExecutionCandidateReviewRules {
    fn default() -> Self {
        Self {
            min_posterior_improvement: 0.03,
            min_win_probability_improvement: 0.03,
            require_same_data: true,
            require_same_factor_version: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactReviewRules {
    pub pending_update: PendingUpdateReviewRules,
    pub execution_candidate: ExecutionCandidateReviewRules,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnsembleExecutorScorecard {
    pub executor: String,
    pub wins: usize,
    pub losses: usize,
    pub breakevens: usize,
    pub validated_positive: usize,
    pub validated_negative: usize,
    pub cumulative_quality_score: i64,
    pub latest_weight_hint: Option<f64>,
    pub last_outcome: Option<String>,
    pub last_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnsembleVoteRecord {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub source_phase: String,
    pub source_run_id: Option<String>,
    pub provenance: RunProvenance,
    pub dataset_comparability: DatasetComparability,
    pub ensemble_version: String,
    pub final_action: String,
    pub recommended_command: String,
    pub human_next_triage: String,
    #[serde(default)]
    pub hard_block: crate::application::orchestration::EnsembleHardBlockArtifact,
    pub confidence: f64,
    pub consensus_strength: f64,
    pub disagreement_flags: Vec<String>,
    pub executor_summaries: Vec<String>,
    pub split_explanations: Vec<String>,
    #[serde(default)]
    // DEPRECATED compatibility mirror. Canonical executor scorecards live in
    // ensemble_executor_scorecards.json and should be read through canonical loaders.
    pub executor_scorecards: Vec<EnsembleExecutorScorecard>,
    #[serde(default)]
    // Source of the scorecard surface used for this record: persisted or fallback.
    pub executor_scorecards_source: Option<String>,
    pub posterior_fingerprint: String,
    pub posterior_normalization_status: String,
    pub posterior_active_regime: String,
    pub posterior_confidence: Option<f64>,
    pub posterior_probabilities: BTreeMap<String, f64>,
    pub posterior_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactHistorySummary {
    pub total_entries: usize,
    pub pending_update_entries: usize,
    pub execution_candidate_entries: usize,
    #[serde(default)]
    pub ensemble_vote_entries: usize,
    pub promotable_entries: usize,
    pub actionable_entries: usize,
    pub consumed_entries: usize,
    pub average_quality_score: f64,
    pub latest_consumed_artifact_id: Option<String>,
    pub statuses_by_kind: BTreeMap<String, BTreeMap<String, usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactLineageSummary {
    pub artifact_kind: String,
    pub root_artifact_id: String,
    pub latest_artifact_id: String,
    pub artifact_count: usize,
    pub quality_delta: i32,
    pub consumed_count: usize,
    pub conclusion: String,
    #[serde(default)]
    pub distinct_review_rule_versions: Vec<String>,
    #[serde(default)]
    pub review_rule_break_count: usize,
    #[serde(default)]
    pub embedded_pre_bayes_change_count: usize,
    #[serde(default)]
    pub latest_pre_bayes_gate_status: String,
    #[serde(default)]
    pub latest_pre_bayes_bridge_selected_entry_quality: String,
    #[serde(default)]
    pub latest_pre_bayes_multi_timeframe_direction_bias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactRuleBreakFactorImpact {
    pub factor_name: String,
    pub break_count: usize,
    pub cumulative_quality_delta: i32,
    pub improving_breaks: usize,
    pub deteriorating_breaks: usize,
    #[serde(default)]
    pub consumed_breaks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactRuleBreakFamilyImpact {
    pub family: String,
    pub break_count: usize,
    pub cumulative_quality_delta: i32,
    pub improving_breaks: usize,
    pub deteriorating_breaks: usize,
    #[serde(default)]
    pub consumed_breaks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConsumedImpactPoint {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub consumed_at: Option<DateTime<Utc>>,
    pub consumed_outcome: Option<String>,
    pub quality_score: i32,
    pub regrade_status: Option<String>,
    pub quality_delta_from_previous_consumed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConsumedImpactWindow {
    pub label: String,
    pub count: usize,
    pub positive: usize,
    pub negative: usize,
    pub neutral: usize,
    pub average_quality_score: f64,
    pub cumulative_quality_delta: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConsumedImpactTrendComparison {
    pub label: String,
    pub recent: ArtifactConsumedImpactWindow,
    pub baseline: ArtifactConsumedImpactWindow,
    pub average_quality_score_delta: f64,
    pub cumulative_quality_delta_delta: i32,
    pub positive_rate_delta: f64,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactConsumedImpactSummary {
    pub total_consumed: usize,
    pub positive_consumed: usize,
    pub negative_consumed: usize,
    pub neutral_consumed: usize,
    pub cumulative_quality_score: i32,
    pub points: Vec<ArtifactConsumedImpactPoint>,
    #[serde(default)]
    pub by_kind: BTreeMap<String, ArtifactConsumedImpactWindow>,
    #[serde(default)]
    pub recent_windows: Vec<ArtifactConsumedImpactWindow>,
    #[serde(default)]
    pub trend_comparisons: Vec<ArtifactConsumedImpactTrendComparison>,
    #[serde(default)]
    pub by_kind_trend_comparisons: BTreeMap<String, Vec<ArtifactConsumedImpactTrendComparison>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactRuleBreakEffect {
    pub artifact_kind: String,
    pub lineage_root_artifact_id: String,
    pub from_artifact_id: String,
    pub to_artifact_id: String,
    pub from_rule_version: String,
    pub to_rule_version: String,
    pub quality_delta: i32,
    pub consumed_delta: i32,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactDecisionSummary {
    pub actionable_artifact_count: usize,
    pub latest_promotable_artifact_id: Option<String>,
    pub artifact_rule_break_count: usize,
    pub highlighted_actions: Vec<String>,
    #[serde(default)]
    pub highlighted_factor_targets: Vec<String>,
    #[serde(default)]
    pub highlighted_family_targets: Vec<String>,
    #[serde(default)]
    pub promotion_strength: String,
    #[serde(default)]
    pub rollback_strength: String,
    #[serde(default)]
    pub consumed_trend_status: String,
    #[serde(default)]
    pub consumed_trend_reason: String,
    #[serde(default)]
    pub consumed_target_kinds: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactDecisionSection {
    pub summary: ArtifactDecisionSummary,
    pub action_summary: Vec<String>,
    pub top_factor_trends: Vec<ArtifactFactorTrendSummary>,
    pub top_family_trends: Vec<ArtifactFamilyTrendSummary>,
    pub top_rule_break_effects: Vec<ArtifactRuleBreakEffect>,
    #[serde(default)]
    pub top_consumed_trend_comparisons: Vec<ArtifactConsumedImpactTrendComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFactorTrendSummary {
    pub factor_name: String,
    pub entries: usize,
    pub promotable_entries: usize,
    pub consumed_entries: usize,
    pub average_quality_score: f64,
    pub latest_status: Option<String>,
    pub latest_action: Option<String>,
    #[serde(default)]
    pub decision_status: String,
    #[serde(default)]
    pub decision_reason: String,
    #[serde(default)]
    pub promotion_link_status: String,
    #[serde(default)]
    pub rollback_link_status: String,
    #[serde(default)]
    pub consumed_validation_status: String,
    #[serde(default)]
    pub consumed_validation_reason: String,
    #[serde(default)]
    pub consumed_validation_rank: i32,
    #[serde(default)]
    pub consumed_validation_score: f64,
}

impl Default for ArtifactFactorTrendSummary {
    fn default() -> Self {
        Self {
            factor_name: String::new(),
            entries: 0,
            promotable_entries: 0,
            consumed_entries: 0,
            average_quality_score: 0.0,
            latest_status: None,
            latest_action: None,
            decision_status: String::new(),
            decision_reason: String::new(),
            promotion_link_status: String::new(),
            rollback_link_status: String::new(),
            consumed_validation_status: String::new(),
            consumed_validation_reason: String::new(),
            consumed_validation_rank: 0,
            consumed_validation_score: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFamilyTrendSummary {
    pub family: String,
    pub entries: usize,
    pub promotable_entries: usize,
    pub consumed_entries: usize,
    pub average_quality_score: f64,
    pub latest_status: Option<String>,
    pub latest_score: Option<f64>,
    #[serde(default)]
    pub decision_status: String,
    #[serde(default)]
    pub decision_reason: String,
    #[serde(default)]
    pub promotion_link_status: String,
    #[serde(default)]
    pub rollback_link_status: String,
    #[serde(default)]
    pub consumed_validation_status: String,
    #[serde(default)]
    pub consumed_validation_reason: String,
    #[serde(default)]
    pub consumed_validation_rank: i32,
    #[serde(default)]
    pub consumed_validation_score: f64,
}

impl Default for ArtifactFamilyTrendSummary {
    fn default() -> Self {
        Self {
            family: String::new(),
            entries: 0,
            promotable_entries: 0,
            consumed_entries: 0,
            average_quality_score: 0.0,
            latest_status: None,
            latest_score: None,
            decision_status: String::new(),
            decision_reason: String::new(),
            promotion_link_status: String::new(),
            rollback_link_status: String::new(),
            consumed_validation_status: String::new(),
            consumed_validation_reason: String::new(),
            consumed_validation_rank: 0,
            consumed_validation_score: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackFactorUsage {
    pub factor_name: String,
    pub category: String,
    pub direction: Direction,
    pub value: f64,
    pub confidence: f64,
    pub weight: f64,
    pub long_support: f64,
    pub short_support: f64,
    pub uncertainty_contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProbabilitySnapshot {
    pub selected_direction: Direction,
    pub selected_probability: f64,
    pub long_score: f64,
    pub short_score: f64,
    pub win_prob_long: f64,
    pub win_prob_short: f64,
    pub uncertainty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeedbackHistorySummary {
    pub total_records: usize,
    pub wins: usize,
    pub losses: usize,
    pub avg_pnl: f64,
    pub factor_success_rates: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionThresholds {
    pub promotion_min_score_delta: f64,
    pub promotion_min_score: f64,
    pub rollback_score_delta: f64,
    pub rollback_probability_delta: f64,
    #[serde(default)]
    pub artifact_consumed_min_window: usize,
    #[serde(default)]
    pub artifact_consumed_improvement_quality_delta: f64,
    #[serde(default)]
    pub artifact_consumed_improvement_positive_rate_delta: f64,
    #[serde(default)]
    pub artifact_consumed_regression_quality_delta: f64,
    #[serde(default)]
    pub artifact_consumed_regression_positive_rate_delta: f64,
}

impl Default for DecisionThresholds {
    fn default() -> Self {
        Self {
            promotion_min_score_delta: 0.10,
            promotion_min_score: 0.70,
            rollback_score_delta: -0.10,
            rollback_probability_delta: 0.10,
            artifact_consumed_min_window: 3,
            artifact_consumed_improvement_quality_delta: 5.0,
            artifact_consumed_improvement_positive_rate_delta: 0.25,
            artifact_consumed_regression_quality_delta: -5.0,
            artifact_consumed_regression_positive_rate_delta: -0.25,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEvidencePolicy {
    pub version: String,
    pub source: String,
    pub min_directional_support_gap: f64,
    pub high_uncertainty_threshold: f64,
    pub min_multi_timeframe_alignment_score: f64,
    pub min_multi_timeframe_entry_alignment_score: f64,
    pub hard_pass_quality_threshold: f64,
    pub neutralized_quality_threshold: f64,
    pub directional_conflict_penalty: f64,
    pub mixed_alignment_penalty: f64,
    pub multi_timeframe_direction_conflict_penalty: f64,
    pub multi_timeframe_alignment_penalty: f64,
    pub multi_timeframe_entry_penalty: f64,
    pub multi_timeframe_alignment_bonus: f64,
    pub hostile_liquidity_penalty: f64,
    pub favorable_liquidity_bonus: f64,
    pub hostile_liquidity_forces_high_uncertainty: bool,
    #[serde(default)]
    pub market_overrides: BTreeMap<String, PreBayesMarketPolicyOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesMarketPolicyOverride {
    #[serde(default)]
    pub hostile_liquidity_penalty: Option<f64>,
    #[serde(default)]
    pub favorable_liquidity_bonus: Option<f64>,
    #[serde(default)]
    pub hostile_liquidity_forces_high_uncertainty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEntryQualityBridge {
    pub long_signal_probability: f64,
    pub short_signal_probability: f64,
    pub long_entry_bias: Vec<f64>,
    pub short_entry_bias: Vec<f64>,
    pub long_entry_quality: BTreeMap<String, f64>,
    pub short_entry_quality: BTreeMap<String, f64>,
    pub selected_entry_quality: BTreeMap<String, f64>,
    #[serde(default)]
    pub multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesPolicyDiff {
    pub previous_version: Option<String>,
    pub changed_fields: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesPolicyRecord {
    pub timestamp: DateTime<Utc>,
    pub run_id: String,
    pub source_command: String,
    pub policy: PreBayesEvidencePolicy,
    pub diff_from_previous: PreBayesPolicyDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesPolicyLineageSummary {
    pub latest_version: Option<String>,
    pub previous_version: Option<String>,
    pub total_versions: usize,
    pub changed_fields_union: Vec<String>,
    pub rollback_candidate_version: Option<String>,
    pub rollback_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesSoftEvidenceNodeDiff {
    pub node: String,
    pub filtered_state: String,
    pub dominant_soft_state: Option<String>,
    pub dominant_soft_probability: f64,
    pub entropy: f64,
    pub diverges_from_filtered_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEntryQualityBridgeDiff {
    pub dominant_long_entry_quality: Option<String>,
    pub dominant_long_entry_quality_probability: f64,
    pub dominant_short_entry_quality: Option<String>,
    pub dominant_short_entry_quality_probability: f64,
    pub selected_entry_quality: Option<String>,
    pub selected_entry_quality_probability: f64,
    pub long_short_signal_probability_gap: f64,
    #[serde(default)]
    pub multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub rationale_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorPipelineLabelSource {
    pub label: String,
    pub derivation: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEvidencePacket {
    pub filter: PreBayesEvidenceFilter,
    #[serde(default)]
    pub evidence_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub timed_pda_summary: crate::bbn::ICTStructureSummary,
    pub raw_market_regime_trace: FactorPipelineLabelSource,
    pub raw_liquidity_context_trace: FactorPipelineLabelSource,
    pub raw_multi_timeframe_resonance_trace: FactorPipelineLabelSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreBayesEvidenceFilter {
    #[serde(default)]
    pub policy: PreBayesEvidencePolicy,
    #[serde(default)]
    pub entry_logic_id: Option<String>,
    #[serde(default)]
    pub logic_family: Option<String>,
    pub raw_market_regime_label: String,
    pub raw_liquidity_context_label: String,
    pub raw_factor_alignment: String,
    pub raw_factor_uncertainty: String,
    #[serde(default)]
    pub raw_multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub raw_multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub raw_multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub raw_multi_timeframe_resonance_label: String,
    #[serde(default)]
    pub active_pda_count: usize,
    #[serde(default)]
    pub inversed_pda_count: usize,
    #[serde(default)]
    pub stale_pda_count: usize,
    #[serde(default)]
    pub nearest_active_pda: Option<String>,
    #[serde(default)]
    pub nearest_inversed_pda: Option<String>,
    pub filtered_market_regime_label: String,
    pub filtered_liquidity_context_label: String,
    pub filtered_factor_alignment: String,
    pub filtered_factor_uncertainty: String,
    #[serde(default)]
    pub filtered_multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub filtered_multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub filtered_multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub filtered_multi_timeframe_resonance_label: String,
    pub evidence_quality_score: f64,
    pub gating_status: String,
    pub pass_to_bbn: bool,
    #[serde(default)]
    pub uses_soft_evidence: bool,
    #[serde(default)]
    pub conflict_flags: Vec<String>,
    #[serde(default)]
    pub rationale: Vec<String>,
    #[serde(default)]
    pub evidence_assignments: BTreeMap<String, String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RankingDiffItem {
    pub factor_name: String,
    pub previous_score: Option<f64>,
    pub new_score: f64,
    pub score_delta: f64,
    pub previous_weight: Option<f64>,
    pub new_weight: f64,
    pub weight_delta: f64,
    pub previous_action: Option<String>,
    pub new_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProbabilityDiff {
    pub state: String,
    pub previous: Option<f64>,
    pub new: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunProvenance {
    pub prompt_version: String,
    pub factor_version: String,
    pub config_hash: String,
    pub data_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatasetComparability {
    pub comparable: bool,
    pub previous_run_id: Option<String>,
    pub reason: String,
    #[serde(default)]
    pub comparison_class: String,
    #[serde(default)]
    pub same_data: bool,
    #[serde(default)]
    pub same_config: bool,
    #[serde(default)]
    pub same_prompt_version: bool,
    #[serde(default)]
    pub same_factor_version: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromotionDecision {
    pub approved: bool,
    pub status: String,
    pub reason: String,
    pub target_factors: Vec<String>,
    #[serde(default)]
    pub target_families: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RollbackRecommendation {
    pub should_rollback: bool,
    pub scope: String,
    pub reason: String,
    pub target_factors: Vec<String>,
    #[serde(default)]
    pub target_families: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub provenance: RunProvenance,
    pub dataset_comparability: DatasetComparability,
    pub source_command: String,
    pub data_path: String,
    pub epochs: usize,
    pub candles: usize,
    pub observations: usize,
    pub final_state: String,
    pub log_likelihood: f64,
    pub viterbi_log_likelihood: f64,
    #[serde(default)]
    pub workflow_state: WorkflowState,
    #[serde(default)]
    pub agent_action_plan: AgentActionPlan,
    #[serde(default)]
    pub recommended_commands: CommandRecommendations,
    #[serde(default)]
    pub recommended_next_command: String,
    #[serde(default)]
    pub agent_context_bundle: AgentContextBundle,
    #[serde(default)]
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    #[serde(default)]
    pub agent_prompts: crate::agent::AgentPromptPack,
    #[serde(default)]
    pub prompt_workflow: String,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    #[serde(default)]
    pub research_objective: String,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub family_history_window: usize,
    pub data_path: String,
    pub paired_data_path: Option<String>,
    pub candles: usize,
    pub paired_candles: Option<usize>,
    pub config_name: String,
    pub source_command: String,
    pub factor_count: usize,
    pub best_factor: Option<String>,
    pub aggregate_return: f64,
    pub feedback_records_generated: usize,
    pub feedback_records_applied: usize,
    pub factor_score_deltas: Vec<RankingDiffItem>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    #[serde(default)]
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    #[serde(default)]
    pub factor_family_history: Vec<FactorFamilyHistory>,
    #[serde(default)]
    pub decision_history_summary: DecisionHistorySummary,
    #[serde(default)]
    pub workflow_state: WorkflowState,
    #[serde(default)]
    pub agent_action_plan: AgentActionPlan,
    #[serde(default)]
    pub recommended_commands: CommandRecommendations,
    #[serde(default)]
    pub recommended_next_command: String,
    #[serde(default)]
    pub agent_context_bundle: AgentContextBundle,
    #[serde(default)]
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    #[serde(default)]
    pub feedback_history_summary: FeedbackHistorySummary,
    #[serde(default)]
    pub artifact_action_summary: Vec<String>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_decision_section: ArtifactDecisionSection,
    #[serde(default)]
    pub agent_prompts: crate::agent::AgentPromptPack,
    pub prompt_workflow: String,
    #[serde(default)]
    pub factor_mutation_evaluation: Option<FactorMutationEvaluation>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMutationSpec {
    #[serde(default)]
    pub mutation_id: String,
    #[serde(default)]
    pub base_factor: String,
    #[serde(default)]
    pub hypothesis: String,
    #[serde(default)]
    pub parameter_overrides: BTreeMap<String, f64>,
    #[serde(default)]
    pub direction_hints: BTreeMap<String, String>,
    #[serde(default)]
    pub step_size_hints: BTreeMap<String, f64>,
    #[serde(default)]
    pub enabled_overrides: BTreeMap<String, bool>,
    #[serde(default)]
    pub evaluate_expansion_preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMutationMetricSet {
    pub best_factor_composite_score: f64,
    pub aggregate_return: f64,
    pub feedback_records_generated: usize,
    pub feedback_records_applied: usize,
    #[serde(default)]
    pub top_factor_names: Vec<String>,
    #[serde(default)]
    pub expansion_selected_direction: Option<String>,
    #[serde(default)]
    pub expansion_selected_win_probability: Option<f64>,
    #[serde(default)]
    pub expansion_balanced_accuracy: Option<f64>,
    #[serde(default)]
    pub expansion_directional_accuracy: Option<f64>,
    #[serde(default)]
    pub pre_bayes_gate_status: Option<String>,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: Option<String>,
    #[serde(default)]
    pub pre_bayes_bridge_probability_gap: Option<f64>,
    #[serde(default)]
    pub pre_bayes_soft_evidence_divergence_count: usize,
    #[serde(default)]
    pub worst_market_balanced_accuracy: Option<f64>,
    #[serde(default)]
    pub worst_market_bridge_probability_gap: Option<f64>,
    #[serde(default)]
    pub regressed_markets: Vec<String>,
    #[serde(default)]
    pub regression_reasons_by_market: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub multi_timeframe_direction_bias: Option<String>,
    #[serde(default)]
    pub multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub multi_timeframe_entry_alignment_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMutationEvaluation {
    pub mutation_id: String,
    pub accepted: bool,
    pub score_before: f64,
    pub score_after: f64,
    pub score_delta: f64,
    pub baseline_available: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub failure_tags: Vec<String>,
    #[serde(default)]
    pub recommended_mutation_directions: Vec<String>,
    #[serde(default)]
    pub metrics_before: Option<FactorMutationMetricSet>,
    pub metrics_after: FactorMutationMetricSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMutationRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub source_command: String,
    pub data_path: String,
    #[serde(default)]
    pub paired_data_path: Option<String>,
    pub mutation_spec: FactorMutationSpec,
    pub evaluation: FactorMutationEvaluation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchDecision {
    pub status: String,
    pub reason: String,
    pub promoted_to_baseline: bool,
    pub baseline_score_before: f64,
    pub candidate_score: f64,
    pub score_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchAttempt {
    pub session_id: String,
    pub attempt_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub source_command: String,
    pub base_factor: String,
    #[serde(default)]
    pub baseline_mutation_id_before: Option<String>,
    pub candidate_mutation_spec: FactorMutationSpec,
    pub evaluation: FactorMutationEvaluation,
    pub decision: FactorAutoresearchDecision,
    #[serde(default)]
    pub branch_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchSession {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub symbol: String,
    pub objective: String,
    pub source_command: String,
    pub base_factor: String,
    #[serde(default)]
    pub baseline_mutation_id: Option<String>,
    pub baseline_score: f64,
    pub attempts_total: usize,
    pub kept_attempts: usize,
    pub discarded_attempts: usize,
    #[serde(default)]
    pub last_attempt_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchLiveSnapshot {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub symbol: String,
    pub objective: String,
    pub current_iteration: usize,
    pub attempts_total: usize,
    pub kept_attempts: usize,
    pub discarded_attempts: usize,
    #[serde(default)]
    pub current_candidate_spec: Option<FactorMutationSpec>,
    #[serde(default)]
    pub latest_attempt_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorAutoresearchSummary {
    pub session: FactorAutoresearchSession,
    #[serde(default)]
    pub latest_attempt: Option<FactorAutoresearchAttempt>,
    #[serde(default)]
    pub next_mutation_spec_template: Option<FactorMutationSpec>,
    #[serde(default)]
    pub live_snapshot: Option<FactorAutoresearchLiveSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub family_history_window: usize,
    pub source_command: String,
    pub data_htf_path: Option<String>,
    pub data_mtf_path: Option<String>,
    pub data_ltf_path: Option<String>,
    pub live_data_source: Option<LiveDataSourceProvenance>,
    pub htf_bars: usize,
    pub mtf_bars: usize,
    pub ltf_bars: usize,
    pub selected_direction: Direction,
    pub selected_entry_quality: String,
    pub decision_hint: String,
    #[serde(default)]
    pub pre_bayes_evidence_filter: PreBayesEvidenceFilter,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge: PreBayesEntryQualityBridge,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    pub factor_family_history: Vec<FactorFamilyHistory>,
    pub decision_history_summary: DecisionHistorySummary,
    pub workflow_state: WorkflowState,
    pub agent_action_plan: AgentActionPlan,
    pub recommended_commands: CommandRecommendations,
    pub recommended_next_command: String,
    pub agent_context_bundle: AgentContextBundle,
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    pub feedback_history_summary: FeedbackHistorySummary,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub artifact_action_summary: Vec<String>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_decision_section: ArtifactDecisionSection,
    pub agent_prompts: crate::agent::AgentPromptPack,
    pub prompt_workflow: String,
}

impl Default for AnalyzeRunRecord {
    fn default() -> Self {
        Self {
            run_id: String::new(),
            timestamp: Utc::now(),
            symbol: String::new(),
            provenance: RunProvenance::default(),
            decision_thresholds: DecisionThresholds::default(),
            dataset_comparability: DatasetComparability::default(),
            promotion_decision: PromotionDecision::default(),
            rollback_recommendation: RollbackRecommendation::default(),
            family_history_window: 0,
            source_command: String::new(),
            data_htf_path: None,
            data_mtf_path: None,
            data_ltf_path: None,
            live_data_source: None,
            htf_bars: 0,
            mtf_bars: 0,
            ltf_bars: 0,
            selected_direction: Direction::Neutral,
            selected_entry_quality: String::new(),
            decision_hint: String::new(),
            pre_bayes_evidence_filter: PreBayesEvidenceFilter::default(),
            pre_bayes_entry_quality_bridge: PreBayesEntryQualityBridge::default(),
            factor_family_decisions: Vec::new(),
            factor_family_outcomes: Vec::new(),
            factor_family_diffs: Vec::new(),
            factor_family_history: Vec::new(),
            decision_history_summary: DecisionHistorySummary::default(),
            workflow_state: WorkflowState::default(),
            agent_action_plan: AgentActionPlan::default(),
            recommended_commands: CommandRecommendations::default(),
            recommended_next_command: String::new(),
            agent_context_bundle: AgentContextBundle::default(),
            agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
            feedback_history_summary: FeedbackHistorySummary::default(),
            multi_timeframe_summary: Vec::new(),
            artifact_action_summary: Vec::new(),
            artifact_decision_summary: ArtifactDecisionSummary::default(),
            artifact_decision_section: ArtifactDecisionSection::default(),
            agent_prompts: crate::agent::AgentPromptPack::default(),
            prompt_workflow: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    #[serde(default)]
    pub ensemble_executor_scorecards: Vec<EnsembleExecutorScorecard>,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub family_history_window: usize,
    pub source_command: String,
    pub normalized_entry_quality: String,
    pub factor_alignment: String,
    pub factor_uncertainty: String,
    pub realized_outcome: String,
    pub feedback_records_applied: usize,
    pub duplicate_feedback_skipped: bool,
    #[serde(default)]
    pub consumed_pending_update_artifact_id: Option<String>,
    #[serde(default)]
    pub consumed_execution_candidate_artifact_id: Option<String>,
    #[serde(default)]
    pub consumed_artifact_path: Option<String>,
    #[serde(default)]
    pub consumed_analyze_run_id: Option<String>,
    #[serde(default)]
    pub consumed_pre_bayes_evidence_filter: Option<PreBayesEvidenceFilter>,
    #[serde(default)]
    pub consumed_pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    #[serde(default)]
    pub consumed_multi_timeframe_summary: Vec<String>,
    pub trade_outcome_deltas: Vec<ProbabilityDiff>,
    pub factor_score_deltas: Vec<RankingDiffItem>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    #[serde(default)]
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    #[serde(default)]
    pub factor_family_history: Vec<FactorFamilyHistory>,
    #[serde(default)]
    pub decision_history_summary: DecisionHistorySummary,
    #[serde(default)]
    pub workflow_state: WorkflowState,
    #[serde(default)]
    pub agent_action_plan: AgentActionPlan,
    #[serde(default)]
    pub recommended_commands: CommandRecommendations,
    #[serde(default)]
    pub recommended_next_command: String,
    #[serde(default)]
    pub agent_context_bundle: AgentContextBundle,
    #[serde(default)]
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    #[serde(default)]
    pub feedback_history_summary: FeedbackHistorySummary,
    #[serde(default)]
    pub artifact_action_summary: Vec<String>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_decision_section: ArtifactDecisionSection,
    #[serde(default)]
    pub agent_prompts: crate::agent::AgentPromptPack,
    pub prompt_workflow: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BacktestRunRecord {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub provenance: RunProvenance,
    pub decision_thresholds: DecisionThresholds,
    pub dataset_comparability: DatasetComparability,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
    pub family_history_window: usize,
    pub data_path: String,
    pub paired_data_path: Option<String>,
    pub candles: usize,
    pub paired_candles: Option<usize>,
    pub warmup_bars: usize,
    pub hold_bars: usize,
    pub online_learning: bool,
    pub source_command: String,
    pub total_return: f64,
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
    pub factor_score_deltas: Vec<RankingDiffItem>,
    pub trade_outcome_deltas: Vec<ProbabilityDiff>,
    pub factor_family_decisions: Vec<FactorFamilyDecision>,
    pub factor_family_outcomes: Vec<FactorFamilyOutcome>,
    #[serde(default)]
    pub factor_family_diffs: Vec<FactorFamilyDiff>,
    #[serde(default)]
    pub factor_family_history: Vec<FactorFamilyHistory>,
    #[serde(default)]
    pub decision_history_summary: DecisionHistorySummary,
    #[serde(default)]
    pub workflow_state: WorkflowState,
    #[serde(default)]
    pub agent_action_plan: AgentActionPlan,
    #[serde(default)]
    pub recommended_commands: CommandRecommendations,
    #[serde(default)]
    pub recommended_next_command: String,
    #[serde(default)]
    pub agent_context_bundle: AgentContextBundle,
    #[serde(default)]
    pub agent_context_bundle_minimal: AgentContextBundleMinimal,
    #[serde(default)]
    pub feedback_history_summary: FeedbackHistorySummary,
    #[serde(default)]
    pub artifact_action_summary: Vec<String>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_decision_section: ArtifactDecisionSection,
    #[serde(default)]
    pub agent_prompts: crate::agent::AgentPromptPack,
    pub prompt_workflow: String,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub objective_market_credibility_shrink:
        Option<crate::domain::belief::ObjectiveMarketCredibilityShrink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedFactorRanking {
    pub factor_name: String,
    pub regime: String,
    pub ic: f64,
    pub ir: f64,
    pub backtest_return: f64,
    pub sharpe: f64,
    pub stability: f64,
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
    pub weight: f64,
    pub regime_scores: BTreeMap<String, f64>,
    pub composite_score: f64,
    pub score_breakdown: BTreeMap<String, f64>,
    pub grade: String,
    pub iteration_action: String,
    pub replacement_candidate: bool,
    pub weaknesses: Vec<String>,
    pub agent_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorIterationPrompt {
    pub factor_name: String,
    pub composite_score: f64,
    pub grade: String,
    pub iteration_action: String,
    pub replacement_candidate: bool,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorFamilyDecision {
    pub family: String,
    pub factor_count: usize,
    pub avg_score: f64,
    pub actions: Vec<String>,
    pub replacement_candidates: Vec<String>,
    #[serde(default)]
    pub dominant_action: String,
    #[serde(default)]
    pub decision_status: String,
    #[serde(default)]
    pub decision_reason: String,
    #[serde(default)]
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorFamilyOutcome {
    pub family: String,
    pub promotion_decision: PromotionDecision,
    pub rollback_recommendation: RollbackRecommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorFamilyDiff {
    pub family: String,
    pub previous_avg_score: Option<f64>,
    pub new_avg_score: f64,
    pub avg_score_delta: f64,
    pub previous_replacement_count: usize,
    pub new_replacement_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionHistorySummary {
    pub total_runs: usize,
    pub promotion_approved_runs: usize,
    pub rollback_recommended_runs: usize,
    pub latest_promotion_status: Option<String>,
    pub latest_rollback_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentActionItem {
    pub stage: String,
    pub blocking: bool,
    pub priority: String,
    pub title: String,
    pub rationale: String,
    pub expected_output: String,
    pub expected_state_changes: Vec<ExpectedStateChange>,
    pub suggested_files: Vec<String>,
    pub suggested_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExpectedStateChange {
    pub target: String,
    pub direction: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentActionPlan {
    pub summary: String,
    pub items: Vec<AgentActionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecommendedCommand {
    pub command: String,
    pub ready: bool,
    #[serde(default)]
    pub missing_inputs: Vec<String>,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub user_data_selection_required: bool,
    #[serde(default)]
    pub user_data_selection_prompt: String,
    #[serde(default)]
    pub recorded_data_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandRecommendations {
    pub analyze: RecommendedCommand,
    pub research: RecommendedCommand,
    pub backtest: RecommendedCommand,
    pub update: RecommendedCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiveDataSourceProvenance {
    pub futures_backend: String,
    pub aux_backend: String,
    pub futures_base_url: String,
    pub aux_base_url: String,
    pub futures_symbol: String,
    pub spot_symbol: String,
    pub options_symbol: String,
    pub spot_kind: String,
    pub fetched_at: DateTime<Utc>,
    #[serde(default)]
    pub persisted_htf_path: Option<String>,
    #[serde(default)]
    pub persisted_h4_path: Option<String>,
    #[serde(default)]
    pub persisted_mtf_path: Option<String>,
    #[serde(default)]
    pub persisted_m5_path: Option<String>,
    #[serde(default)]
    pub persisted_ltf_path: Option<String>,
    #[serde(default)]
    pub persisted_m1_path: Option<String>,
    #[serde(default)]
    pub persisted_spot_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorFamilyHistory {
    pub family: String,
    pub window_size: usize,
    pub recent_run_ids: Vec<String>,
    pub recent_timestamps: Vec<DateTime<Utc>>,
    pub recent_avg_scores: Vec<f64>,
    pub recent_replacement_counts: Vec<usize>,
    pub score_trend: String,
    pub replacement_trend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowState {
    pub phase: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPhaseSnapshot {
    pub phase: String,
    pub source_command: String,
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub workflow_phase: String,
    pub workflow_reason: String,
    pub promotion_status: String,
    pub rollback_scope: String,
    pub comparable_to_previous: bool,
    pub comparison_class: String,
    pub recommended_next_command: String,
    pub phase_summary: String,
    pub top_actions: Vec<String>,
    pub risk_flags: Vec<String>,
    #[serde(default)]
    pub selected_direction: Option<String>,
    #[serde(default)]
    pub selected_entry_quality: Option<String>,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_uses_soft_evidence: bool,
    #[serde(default)]
    pub pre_bayes_policy_version: String,
    #[serde(default)]
    pub pre_bayes_evidence_quality_score: f64,
    #[serde(default)]
    pub pre_bayes_conflict_flags: Vec<String>,
    #[serde(default)]
    pub pre_bayes_filtered_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub pre_bayes_soft_evidence: BTreeMap<String, BTreeMap<String, f64>>,
    #[serde(default)]
    pub pre_bayes_long_signal_probability: Option<f64>,
    #[serde(default)]
    pub pre_bayes_short_signal_probability: Option<f64>,
    #[serde(default)]
    pub pre_bayes_selected_entry_quality_probability: Option<f64>,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: Option<String>,
    #[serde(default)]
    pub pre_bayes_bridge_probability_gap: Option<f64>,
    #[serde(default)]
    pub pre_bayes_bridge_rationale_summary: Vec<String>,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub realized_outcome: Option<String>,
    #[serde(default)]
    pub family_states: Vec<String>,
    #[serde(default)]
    pub factor_actions: Vec<String>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub family_score_map: BTreeMap<String, f64>,
    #[serde(default)]
    pub factor_score_map: BTreeMap<String, f64>,
    #[serde(default)]
    pub objective_market_credibility_shrink:
        Option<crate::domain::belief::ObjectiveMarketCredibilityShrink>,
}

impl Default for WorkflowPhaseSnapshot {
    fn default() -> Self {
        Self {
            phase: String::new(),
            source_command: String::new(),
            run_id: String::new(),
            timestamp: Utc::now(),
            workflow_phase: String::new(),
            workflow_reason: String::new(),
            promotion_status: String::new(),
            rollback_scope: String::new(),
            comparable_to_previous: false,
            comparison_class: String::new(),
            recommended_next_command: String::new(),
            phase_summary: String::new(),
            top_actions: Vec::new(),
            risk_flags: Vec::new(),
            selected_direction: None,
            selected_entry_quality: None,
            pre_bayes_gate_status: String::new(),
            pre_bayes_uses_soft_evidence: false,
            pre_bayes_policy_version: String::new(),
            pre_bayes_evidence_quality_score: 0.0,
            pre_bayes_conflict_flags: Vec::new(),
            pre_bayes_filtered_assignments: BTreeMap::new(),
            pre_bayes_soft_evidence: BTreeMap::new(),
            pre_bayes_long_signal_probability: None,
            pre_bayes_short_signal_probability: None,
            pre_bayes_selected_entry_quality_probability: None,
            pre_bayes_bridge_selected_entry_quality: None,
            pre_bayes_bridge_probability_gap: None,
            pre_bayes_bridge_rationale_summary: Vec::new(),
            pre_bayes_multi_timeframe_direction_bias: String::new(),
            pre_bayes_multi_timeframe_alignment_score: None,
            pre_bayes_multi_timeframe_entry_alignment_score: None,
            realized_outcome: None,
            family_states: Vec::new(),
            factor_actions: Vec::new(),
            multi_timeframe_summary: Vec::new(),
            family_score_map: BTreeMap::new(),
            factor_score_map: BTreeMap::new(),
            objective_market_credibility_shrink: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowConflictSource {
    pub scope: String,
    pub subject: String,
    pub left_phase: String,
    pub left_value: String,
    pub right_phase: String,
    pub right_value: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowFieldDiff {
    pub left_phase: String,
    pub right_phase: String,
    pub field: String,
    pub left_value: String,
    pub right_value: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDisagreement {
    pub id: String,
    pub severity: String,
    pub summary: String,
    pub phases: Vec<String>,
    pub recommended_action: String,
    pub evidence: Vec<String>,
    #[serde(default)]
    pub sources: Vec<WorkflowConflictSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    pub symbol: String,
    pub generated_at: DateTime<Utc>,
    pub current_focus_phase: String,
    pub current_focus_reason: String,
    #[serde(default)]
    pub blocking_truth: WorkflowBlockingTruth,
    pub recommended_next_command: String,
    pub pending_actions: Vec<String>,
    pub risk_flags: Vec<String>,
    #[serde(default)]
    pub latest_train: Option<WorkflowPhaseSnapshot>,
    pub latest_analyze: Option<WorkflowPhaseSnapshot>,
    pub latest_research: Option<WorkflowPhaseSnapshot>,
    pub latest_backtest: Option<WorkflowPhaseSnapshot>,
    pub latest_update: Option<WorkflowPhaseSnapshot>,
    #[serde(default)]
    pub latest_pre_bayes_policy: Option<PreBayesEvidencePolicy>,
    #[serde(default)]
    pub latest_pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    #[serde(default)]
    pub latest_pre_bayes_entry_quality_bridge_diff: Option<PreBayesEntryQualityBridgeDiff>,
    #[serde(default)]
    pub latest_pre_bayes_policy_diff: Option<PreBayesPolicyDiff>,
    #[serde(default)]
    pub latest_pre_bayes_policy_lineage: Option<PreBayesPolicyLineageSummary>,
    #[serde(default)]
    pub latest_pre_bayes_soft_evidence_diff: Vec<PreBayesSoftEvidenceNodeDiff>,
    #[serde(default)]
    pub recent_pre_bayes_policies: Vec<PreBayesPolicyRecord>,
    #[serde(default)]
    pub latest_pending_update: Option<PendingUpdateArtifactSummary>,
    #[serde(default)]
    pub recent_pending_updates: Vec<PendingUpdateArtifactSummary>,
    #[serde(default)]
    pub latest_execution_candidate: Option<ExecutionCandidateArtifactSummary>,
    #[serde(default)]
    pub recent_execution_candidates: Vec<ExecutionCandidateArtifactSummary>,
    #[serde(default)]
    pub latest_ensemble_vote: Option<EnsembleVoteRecord>,
    #[serde(default)]
    pub recent_ensemble_votes: Vec<EnsembleVoteRecord>,
    #[serde(default)]
    pub recent_artifacts: Vec<ArtifactLedgerEntry>,
    #[serde(default)]
    pub actionable_artifacts: Vec<ArtifactLedgerEntry>,
    #[serde(default)]
    pub latest_promotable_artifact: Option<ArtifactLedgerEntry>,
    #[serde(default)]
    pub artifact_history_summary: ArtifactHistorySummary,
    #[serde(default)]
    pub artifact_factor_trends: Vec<ArtifactFactorTrendSummary>,
    #[serde(default)]
    pub artifact_family_trends: Vec<ArtifactFamilyTrendSummary>,
    #[serde(default)]
    pub artifact_decision_summary: ArtifactDecisionSummary,
    #[serde(default)]
    pub artifact_review_rules: ArtifactReviewRules,
    #[serde(default)]
    pub artifact_review_rule_sources: ArtifactReviewRuleSources,
    #[serde(default)]
    pub artifact_lineage_summaries: Vec<ArtifactLineageSummary>,
    #[serde(default)]
    pub artifact_rule_break_effects: Vec<ArtifactRuleBreakEffect>,
    #[serde(default)]
    pub artifact_factor_rule_break_impacts: Vec<ArtifactRuleBreakFactorImpact>,
    #[serde(default)]
    pub artifact_family_rule_break_impacts: Vec<ArtifactRuleBreakFamilyImpact>,
    #[serde(default)]
    pub artifact_consumed_impact_summary: ArtifactConsumedImpactSummary,
    #[serde(default)]
    pub field_diffs: Vec<WorkflowFieldDiff>,
    #[serde(default)]
    pub disagreements: Vec<WorkflowDisagreement>,
}

impl Default for WorkflowSnapshot {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            generated_at: Utc::now(),
            current_focus_phase: String::new(),
            current_focus_reason: String::new(),
            blocking_truth: WorkflowBlockingTruth::default(),
            recommended_next_command: String::new(),
            pending_actions: Vec::new(),
            risk_flags: Vec::new(),
            latest_train: None,
            latest_analyze: None,
            latest_research: None,
            latest_backtest: None,
            latest_update: None,
            latest_pre_bayes_policy: None,
            latest_pre_bayes_entry_quality_bridge: None,
            latest_pre_bayes_entry_quality_bridge_diff: None,
            latest_pre_bayes_policy_diff: None,
            latest_pre_bayes_policy_lineage: None,
            latest_pre_bayes_soft_evidence_diff: Vec::new(),
            recent_pre_bayes_policies: Vec::new(),
            latest_pending_update: None,
            recent_pending_updates: Vec::new(),
            latest_execution_candidate: None,
            recent_execution_candidates: Vec::new(),
            latest_ensemble_vote: None,
            recent_ensemble_votes: Vec::new(),
            recent_artifacts: Vec::new(),
            actionable_artifacts: Vec::new(),
            latest_promotable_artifact: None,
            artifact_history_summary: ArtifactHistorySummary::default(),
            artifact_factor_trends: Vec::new(),
            artifact_family_trends: Vec::new(),
            artifact_decision_summary: ArtifactDecisionSummary::default(),
            artifact_review_rules: ArtifactReviewRules::default(),
            artifact_review_rule_sources: ArtifactReviewRuleSources::default(),
            artifact_lineage_summaries: Vec::new(),
            artifact_rule_break_effects: Vec::new(),
            artifact_factor_rule_break_impacts: Vec::new(),
            artifact_family_rule_break_impacts: Vec::new(),
            artifact_consumed_impact_summary: ArtifactConsumedImpactSummary::default(),
            field_diffs: Vec::new(),
            disagreements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowBlockingTruth {
    pub stage: String,
    pub status: String,
    pub reason: String,
    pub evidence: Vec<String>,
    pub next_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentContextBundle {
    pub workflow_state: WorkflowState,
    pub decision_hint: String,
    pub recommended_next_command: String,
    pub recommended_commands: CommandRecommendations,
    pub family_history_window: usize,
    pub comparable_to_last_run: bool,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_uses_soft_evidence: bool,
    #[serde(default)]
    pub pre_bayes_evidence_quality_score: f64,
    #[serde(default)]
    pub pre_bayes_conflict_flags: Vec<String>,
    #[serde(default)]
    pub pre_bayes_filtered_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub pre_bayes_soft_evidence: BTreeMap<String, BTreeMap<String, f64>>,
    #[serde(default)]
    pub pre_bayes_policy_version: String,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_direction_bias: String,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_alignment_score: Option<f64>,
    #[serde(default)]
    pub pre_bayes_multi_timeframe_entry_alignment_score: Option<f64>,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge_summary: Vec<String>,
    #[serde(default)]
    pub pre_bayes_soft_evidence_diff: Vec<PreBayesSoftEvidenceNodeDiff>,
    #[serde(default)]
    pub pre_bayes_entry_quality_bridge_diff: Option<PreBayesEntryQualityBridgeDiff>,
    #[serde(default)]
    pub factor_mutation_evaluation: Option<FactorMutationEvaluation>,
    #[serde(default)]
    pub factor_mutation_priority_markets: Vec<String>,
    #[serde(default)]
    pub factor_mutation_priority_reasons: Vec<String>,
    #[serde(default)]
    pub factor_mutation_recommended_focus: Vec<String>,
    #[serde(default)]
    pub factor_mutation_direction_hints: Vec<String>,
    #[serde(default)]
    pub factor_mutation_step_size_hints: Vec<String>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub artifact_consumed_gate_status: String,
    #[serde(default)]
    pub artifact_consumed_gate_reason: String,
    #[serde(default)]
    pub artifact_consumed_gate_targets: Vec<String>,
    pub top_factor_actions: Vec<String>,
    pub family_actions: Vec<String>,
    pub stage_views: Vec<StageAgentContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageAgentContext {
    pub stage: String,
    pub blocking_items: usize,
    pub recommended_command: String,
    pub actions: Vec<String>,
    #[serde(default)]
    pub gate_status: String,
    #[serde(default)]
    pub gate_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentContextBundleMinimal {
    pub workflow_phase: String,
    pub recommended_next_command: String,
    pub family_history_window: usize,
    pub comparable_to_last_run: bool,
    #[serde(default)]
    pub pre_bayes_gate_status: String,
    #[serde(default)]
    pub pre_bayes_uses_soft_evidence: bool,
    #[serde(default)]
    pub pre_bayes_policy_version: String,
    #[serde(default)]
    pub pre_bayes_soft_evidence_divergence_count: usize,
    #[serde(default)]
    pub pre_bayes_bridge_selected_entry_quality: String,
    #[serde(default)]
    pub factor_mutation_acceptance_status: String,
    #[serde(default)]
    pub factor_mutation_failure_tags: Vec<String>,
    #[serde(default)]
    pub factor_mutation_priority_markets: Vec<String>,
    #[serde(default)]
    pub factor_mutation_priority_reasons: Vec<String>,
    #[serde(default)]
    pub factor_mutation_direction_hints: Vec<String>,
    #[serde(default)]
    pub factor_mutation_step_size_hints: Vec<String>,
    #[serde(default)]
    pub multi_timeframe_summary: Vec<String>,
    #[serde(default)]
    pub artifact_consumed_gate_status: String,
    pub top_factor_actions: Vec<String>,
    pub stage_views: Vec<StageAgentContextMinimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageAgentContextMinimal {
    pub stage: String,
    pub recommended_command: String,
    #[serde(default)]
    pub gate_status: String,
}

impl LearningState {
    pub fn ensure_profile(&mut self, factor_name: &str) -> &mut FactorLearningProfile {
        self.factor_profiles
            .entry(factor_name.to_string())
            .or_default()
    }

    pub fn profile(&self, factor_name: &str) -> Option<&FactorLearningProfile> {
        self.factor_profiles.get(factor_name)
    }

    pub fn feedback_key(record: &FeedbackRecord) -> String {
        let factors = record
            .factors_used
            .iter()
            .map(|factor| {
                format!(
                    "{}:{:?}:{:.6}:{:.6}:{:.6}",
                    factor.factor_name,
                    factor.direction,
                    factor.value,
                    factor.confidence,
                    factor.weight
                )
            })
            .collect::<Vec<_>>()
            .join("|");

        format!(
            "{}|{}|{}|{:?}|{}|{:.8}|{}|{:?}|{:?}|{:.6}|{:.6}|{}",
            record.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true),
            record.symbol,
            record.source,
            record.trade_id,
            record.realized_outcome,
            record.pnl,
            regime_key(record.regime_at_entry),
            record.model_probabilities_before_trade.selected_direction,
            record
                .model_probabilities_before_trade
                .selected_probability
                .to_bits(),
            record.model_probabilities_before_trade.long_score,
            record.model_probabilities_before_trade.short_score,
            factors
        )
    }

    pub fn merge_feedback_records(&mut self, feedback: &[FeedbackRecord]) -> Vec<FeedbackRecord> {
        let mut existing = self
            .feedback_history
            .iter()
            .map(Self::feedback_key)
            .collect::<std::collections::BTreeSet<_>>();
        let mut inserted = Vec::new();

        for record in feedback {
            let key = Self::feedback_key(record);
            if existing.insert(key) {
                self.feedback_history.push(record.clone());
                inserted.push(record.clone());
            }
        }

        inserted
    }

    pub fn replace_feedback_records(&mut self, feedback: &[FeedbackRecord]) {
        let replacements = feedback
            .iter()
            .map(|record| (Self::feedback_key(record), record.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        for existing in &mut self.feedback_history {
            if let Some(replacement) = replacements.get(&Self::feedback_key(existing)) {
                *existing = replacement.clone();
            }
        }
    }

    pub fn summary(&self) -> FeedbackHistorySummary {
        if self.feedback_history.is_empty() {
            return FeedbackHistorySummary::default();
        }

        let mut summary = FeedbackHistorySummary::default();
        let mut counts = BTreeMap::<String, (usize, usize)>::new();

        for record in &self.feedback_history {
            summary.total_records += 1;
            summary.avg_pnl += record.pnl;
            if record.pnl > 0.0 || record.realized_outcome == "win" {
                summary.wins += 1;
            } else if record.pnl < 0.0 || record.realized_outcome == "loss" {
                summary.losses += 1;
            }

            for factor in &record.factors_used {
                let entry = counts.entry(factor.factor_name.clone()).or_insert((0, 0));
                entry.0 += 1;

                let selected_direction = record.model_probabilities_before_trade.selected_direction;
                let aligned = match selected_direction {
                    Direction::Bull => factor.long_support >= factor.short_support,
                    Direction::Bear => factor.short_support >= factor.long_support,
                    Direction::Neutral => false,
                };
                let effective_success = if aligned {
                    record.pnl >= 0.0
                } else {
                    record.pnl < 0.0
                };
                if effective_success {
                    entry.1 += 1;
                }
            }
        }

        summary.avg_pnl /= self.feedback_history.len() as f64;
        summary.factor_success_rates = counts
            .into_iter()
            .map(|(factor, (count, success))| {
                (
                    factor,
                    if count > 0 {
                        success as f64 / count as f64
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        summary
    }

    pub fn iteration_queue(&self) -> Vec<FactorIterationPrompt> {
        let mut queue = self
            .factor_rankings
            .iter()
            .filter(|ranking| ranking.iteration_action != "keep" || ranking.replacement_candidate)
            .map(FactorIterationPrompt::from)
            .collect::<Vec<_>>();

        queue.sort_by(|a, b| {
            iteration_priority(&b.iteration_action)
                .cmp(&iteration_priority(&a.iteration_action))
                .then_with(|| {
                    a.composite_score
                        .partial_cmp(&b.composite_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        queue
    }

    pub fn family_decisions(&self) -> Vec<FactorFamilyDecision> {
        let mut grouped = BTreeMap::<String, Vec<&PersistedFactorRanking>>::new();
        for ranking in &self.factor_rankings {
            grouped
                .entry(factor_family(&ranking.factor_name).to_string())
                .or_default()
                .push(ranking);
        }

        let mut decisions = grouped
            .into_iter()
            .map(|(family, items)| {
                let avg_score = items.iter().map(|item| item.composite_score).sum::<f64>()
                    / items.len().max(1) as f64;
                let actions = items
                    .iter()
                    .map(|item| format!("{}:{}", item.factor_name, item.iteration_action))
                    .collect::<Vec<_>>();
                let replacement_candidates = items
                    .iter()
                    .filter(|item| item.replacement_candidate)
                    .map(|item| item.factor_name.clone())
                    .collect::<Vec<_>>();
                let dominant_action = family_dominant_action(&items);
                let avg_score_band = factor_grade(avg_score).to_ascii_lowercase();
                let risk_flags = family_risk_flags(&items, avg_score, &replacement_candidates);

                FactorFamilyDecision {
                    family,
                    factor_count: items.len(),
                    avg_score,
                    actions,
                    replacement_candidates,
                    dominant_action: dominant_action.to_string(),
                    decision_status: family_decision_status(&items, avg_score).to_string(),
                    decision_reason: family_decision_reason(
                        dominant_action,
                        avg_score_band.as_str(),
                        &risk_flags,
                    ),
                    risk_flags,
                }
            })
            .collect::<Vec<_>>();

        decisions.sort_by(|a, b| {
            a.avg_score
                .partial_cmp(&b.avg_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        decisions
    }
}

impl FactorLearningProfile {
    pub fn regime_multiplier(&self, regime: Regime) -> f64 {
        self.regime_stats
            .get(regime_key(regime))
            .map(|stats| {
                if stats.multiplier.abs() <= f64::EPSILON {
                    1.0
                } else {
                    stats.multiplier
                }
            })
            .unwrap_or(1.0)
    }
}

impl From<&FactorIC> for PersistedFactorRanking {
    fn from(value: &FactorIC) -> Self {
        let mut ranking = Self {
            factor_name: value.factor_name.clone(),
            regime: regime_key(value.regime).to_string(),
            ic: value.mean_ic,
            ir: value.ir,
            backtest_return: value.backtest_return,
            sharpe: value.sharpe,
            stability: value.stability,
            win_rate: value.win_rate,
            profit_factor: value.profit_factor,
            trade_count: value.trade_count,
            conformal_coverage_1sigma: 0.0,
            conformal_miscoverage_1sigma: 0.0,
            mean_prediction_interval_half_width: 0.0,
            worst_window_miscoverage: 0.0,
            regime_break_penalty: 0.0,
            weight: value.weight,
            regime_scores: value
                .regime_scores
                .iter()
                .map(|(key, value)| (key.clone(), *value))
                .collect(),
            composite_score: 0.0,
            score_breakdown: BTreeMap::new(),
            grade: String::new(),
            iteration_action: String::new(),
            replacement_candidate: false,
            weaknesses: Vec::new(),
            agent_prompt: String::new(),
        };
        ranking.refresh_scorecard();
        ranking
    }
}

impl From<&PersistedFactorRanking> for FactorIterationPrompt {
    fn from(value: &PersistedFactorRanking) -> Self {
        Self {
            factor_name: value.factor_name.clone(),
            composite_score: value.composite_score,
            grade: value.grade.clone(),
            iteration_action: value.iteration_action.clone(),
            replacement_candidate: value.replacement_candidate,
            prompt: value.agent_prompt.clone(),
        }
    }
}

impl PersistedFactorRanking {
    pub fn refresh_scorecard(&mut self) {
        let regime_breadth = if self.regime_scores.is_empty() {
            0.0
        } else {
            self.regime_scores
                .values()
                .filter(|score| **score > 0.0)
                .count() as f64
                / self.regime_scores.len() as f64
        };
        let sample_score = (self.trade_count as f64 / 20.0).clamp(0.0, 1.0);
        let ic_score = (self.ic.abs() / 0.08).clamp(0.0, 1.0);
        let ir_score = (self.ir / 1.5).clamp(0.0, 1.0);
        let return_score = ((self.backtest_return + 0.02) / 0.12).clamp(0.0, 1.0);
        let sharpe_score = ((self.sharpe + 0.2) / 1.7).clamp(0.0, 1.0);
        let stability_score = self.stability.clamp(0.0, 1.0);
        let win_rate_score = ((self.win_rate - 0.45) / 0.20).clamp(0.0, 1.0);
        let profit_factor_score = ((self.profit_factor - 0.95) / 0.55).clamp(0.0, 1.0);
        let regime_score = regime_breadth.clamp(0.0, 1.0);
        let conformal_score = ((self.conformal_coverage_1sigma - 0.45) / 0.35).clamp(0.0, 1.0);
        let break_penalty_score = (1.0 - (self.regime_break_penalty / 0.35)).clamp(0.0, 1.0);

        self.score_breakdown = BTreeMap::from([
            ("ic".to_string(), ic_score),
            ("ir".to_string(), ir_score),
            ("return".to_string(), return_score),
            ("sharpe".to_string(), sharpe_score),
            ("stability".to_string(), stability_score),
            ("win_rate".to_string(), win_rate_score),
            ("profit_factor".to_string(), profit_factor_score),
            ("regime_coverage".to_string(), regime_score),
            ("conformal_coverage".to_string(), conformal_score),
            ("regime_break_resilience".to_string(), break_penalty_score),
            ("sample".to_string(), sample_score),
        ]);

        self.composite_score = 0.16 * ic_score
            + 0.13 * ir_score
            + 0.16 * return_score
            + 0.10 * sharpe_score
            + 0.10 * stability_score
            + 0.10 * win_rate_score
            + 0.08 * profit_factor_score
            + 0.06 * regime_score
            + 0.07 * conformal_score
            + 0.05 * break_penalty_score
            + 0.05 * sample_score;
        self.composite_score = self.composite_score.clamp(0.0, 1.0);

        self.weaknesses = factor_weaknesses(self, regime_score, sample_score);
        self.grade = factor_grade(self.composite_score).to_string();
        self.iteration_action = factor_iteration_action(self, sample_score).to_string();
        self.replacement_candidate = self.iteration_action == "replace"
            && self.trade_count >= 8
            && self.composite_score < 0.45;
        self.agent_prompt = build_agent_prompt(self);
    }
}

fn factor_weaknesses(
    ranking: &PersistedFactorRanking,
    regime_score: f64,
    sample_score: f64,
) -> Vec<String> {
    let mut weaknesses = Vec::new();
    if sample_score < 0.40 {
        weaknesses.push("insufficient_sample".to_string());
    }
    if ranking.backtest_return <= 0.0 {
        weaknesses.push("negative_or_flat_return".to_string());
    }
    if ranking.ic.abs() < 0.03 {
        weaknesses.push("weak_ic".to_string());
    }
    if ranking.ir < 0.5 {
        weaknesses.push("weak_ir".to_string());
    }
    if ranking.sharpe < 0.5 {
        weaknesses.push("low_sharpe".to_string());
    }
    if ranking.stability < 0.45 {
        weaknesses.push("unstable_walk_forward".to_string());
    }
    if ranking.win_rate < 0.48 {
        weaknesses.push("low_win_rate".to_string());
    }
    if ranking.profit_factor < 1.05 {
        weaknesses.push("weak_profit_factor".to_string());
    }
    if ranking.conformal_coverage_1sigma < 0.55 {
        weaknesses.push("low_conformal_coverage".to_string());
    }
    if ranking.regime_break_penalty > 0.20 {
        weaknesses.push("high_regime_break_penalty".to_string());
    }
    if regime_score < 0.34 {
        weaknesses.push("narrow_regime_coverage".to_string());
    }
    weaknesses
}

fn factor_grade(score: f64) -> &'static str {
    if score >= 0.85 {
        "A"
    } else if score >= 0.70 {
        "B"
    } else if score >= 0.55 {
        "C"
    } else if score >= 0.40 {
        "D"
    } else {
        "F"
    }
}

fn factor_iteration_action(ranking: &PersistedFactorRanking, sample_score: f64) -> &'static str {
    if sample_score < 0.40 || ranking.trade_count < 8 {
        "observe"
    } else if ranking.composite_score >= 0.75 && ranking.weaknesses.len() <= 2 {
        "keep"
    } else if ranking.composite_score >= 0.45 {
        "tune"
    } else {
        "replace"
    }
}

fn build_agent_prompt(ranking: &PersistedFactorRanking) -> String {
    let weaknesses = if ranking.weaknesses.is_empty() {
        "none".to_string()
    } else {
        ranking.weaknesses.join(", ")
    };

    match ranking.iteration_action.as_str() {
        "keep" => format!(
            "Keep factor '{}' as the benchmark. score={:.2} grade={}. Only promote a new variant if composite score improves by >=0.05 without reducing stability below {:.2} or profit_factor below {:.2}.",
            ranking.factor_name,
            ranking.composite_score,
            ranking.grade,
            ranking.stability.max(0.50),
            ranking.profit_factor.max(1.05)
        ),
        "tune" => format!(
            "Tune factor '{}'. score={:.2} grade={}. Weaknesses: {}. Agent should iterate parameters/evidence mapping and only accept a variant if composite score improves by >=0.10 and walk-forward stability does not fall.",
            ranking.factor_name,
            ranking.composite_score,
            ranking.grade,
            weaknesses
        ),
        "replace" => format!(
            "Replace factor '{}'. score={:.2} grade={}. Weaknesses: {}. Agent should design a new factor for this slot, benchmark against the current factor, and only promote it if composite score improves by >=0.15 with trade_count >= 8 and profit_factor >= 1.10.",
            ranking.factor_name,
            ranking.composite_score,
            ranking.grade,
            weaknesses
        ),
        _ => format!(
            "Observe factor '{}'. score={:.2} grade={}. Weaknesses: {}. Agent should gather more data or expand sample coverage before replacing it.",
            ranking.factor_name,
            ranking.composite_score,
            ranking.grade,
            weaknesses
        ),
    }
}

fn iteration_priority(action: &str) -> u8 {
    match action {
        "replace" => 3,
        "tune" => 2,
        "observe" => 1,
        _ => 0,
    }
}

fn family_dominant_action(items: &[&PersistedFactorRanking]) -> &'static str {
    if items.iter().any(|item| item.iteration_action == "replace") {
        "replace"
    } else if items.iter().any(|item| item.iteration_action == "tune") {
        "tune"
    } else if items.iter().any(|item| item.iteration_action == "observe") {
        "observe"
    } else {
        "keep"
    }
}

fn family_decision_status(items: &[&PersistedFactorRanking], avg_score: f64) -> &'static str {
    if items.iter().any(|item| item.replacement_candidate) {
        "review_replace"
    } else if avg_score >= 0.75 && items.iter().all(|item| item.iteration_action == "keep") {
        "stable_keep"
    } else if items.iter().any(|item| item.iteration_action == "tune") {
        "needs_tuning"
    } else if items.iter().any(|item| item.iteration_action == "observe") {
        "needs_observation"
    } else {
        "hold"
    }
}

fn family_risk_flags(
    items: &[&PersistedFactorRanking],
    avg_score: f64,
    replacement_candidates: &[String],
) -> Vec<String> {
    let mut flags = Vec::new();
    if avg_score < 0.45 {
        flags.push("low_family_score".to_string());
    }
    if !replacement_candidates.is_empty() {
        flags.push("contains_replacement_candidates".to_string());
    }
    let unique_actions = items
        .iter()
        .map(|item| item.iteration_action.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if unique_actions.len() > 1 {
        flags.push("mixed_iteration_actions".to_string());
    }
    if items.iter().any(|item| {
        item.weaknesses
            .iter()
            .any(|w| w == "narrow_regime_coverage")
    }) {
        flags.push("narrow_regime_coverage".to_string());
    }
    flags
}

fn family_decision_reason(
    dominant_action: &str,
    avg_score_band: &str,
    risk_flags: &[String],
) -> String {
    if !risk_flags.is_empty() {
        format!(
            "dominant_action={} avg_score_band={} risk_flags={}",
            dominant_action,
            avg_score_band,
            risk_flags.join(",")
        )
    } else {
        format!(
            "dominant_action={} avg_score_band={} no_material_family_risks",
            dominant_action, avg_score_band
        )
    }
}

fn factor_family(name: &str) -> &'static str {
    match name {
        "trend_momentum" => "trend_momentum",
        "volatility_mean_reversion" => "volatility_mean_reversion",
        "structure_ict" => "structure_ict",
        "cross_market_smt" => "cross_market_smt",
        "options_hedging" => "options_hedging",
        _ => "other",
    }
}

pub fn regime_key(regime: Regime) -> &'static str {
    match regime {
        Regime::Accumulation => "accumulation",
        Regime::ManipulationExpansion => "manipulation_expansion",
        Regime::Distribution => "distribution",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct RankingInput<'a> {
        name: &'a str,
        mean_ic: f64,
        ir: f64,
        backtest_return: f64,
        sharpe: f64,
        stability: f64,
        win_rate: f64,
        profit_factor: f64,
        trade_count: usize,
    }

    fn ranking(input: RankingInput<'_>) -> PersistedFactorRanking {
        PersistedFactorRanking::from(&FactorIC {
            factor_name: input.name.to_string(),
            regime: Regime::ManipulationExpansion,
            ic_values: vec![0.1, 0.2],
            mean_ic: input.mean_ic,
            std_ic: 0.02,
            ir: input.ir,
            weight: 0.2,
            backtest_return: input.backtest_return,
            sharpe: input.sharpe,
            stability: input.stability,
            win_rate: input.win_rate,
            profit_factor: input.profit_factor,
            trade_count: input.trade_count,
            regime_scores: HashMap::from([("manipulation_expansion".to_string(), 0.02)]),
        })
    }

    fn sample_feedback() -> FeedbackRecord {
        FeedbackRecord {
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            source: "test".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: vec![FeedbackFactorUsage {
                factor_name: "trend_momentum".to_string(),
                category: "trend_momentum".to_string(),
                direction: Direction::Bull,
                value: 0.8,
                confidence: 0.7,
                weight: 0.2,
                long_support: 0.3,
                short_support: 0.0,
                uncertainty_contribution: 0.1,
            }],
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: Direction::Bull,
                selected_probability: 0.7,
                long_score: 0.4,
                short_score: 0.1,
                win_prob_long: 0.6,
                win_prob_short: 0.3,
                uncertainty: 0.2,
            },
            realized_outcome: "win".to_string(),
            pnl: 0.02,
            regime_at_entry: Regime::ManipulationExpansion,
        }
    }

    #[test]
    fn test_merge_feedback_records_deduplicates() {
        let feedback = sample_feedback();
        let mut state = LearningState::default();

        let first = state.merge_feedback_records(std::slice::from_ref(&feedback));
        let second = state.merge_feedback_records(&[feedback]);

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(state.feedback_history.len(), 1);
    }

    #[test]
    fn test_scorecard_assigns_keep_vs_replace_actions() {
        let keep = ranking(RankingInput {
            name: "trend_momentum",
            mean_ic: 0.08,
            ir: 1.2,
            backtest_return: 0.14,
            sharpe: 1.4,
            stability: 0.72,
            win_rate: 0.58,
            profit_factor: 1.35,
            trade_count: 18,
        });
        let replace = ranking(RankingInput {
            name: "weak_factor",
            mean_ic: 0.01,
            ir: 0.1,
            backtest_return: -0.03,
            sharpe: -0.2,
            stability: 0.20,
            win_rate: 0.42,
            profit_factor: 0.82,
            trade_count: 14,
        });

        assert_eq!(keep.iteration_action, "keep");
        assert!(keep.composite_score > replace.composite_score);
        assert_eq!(replace.iteration_action, "replace");
        assert!(replace.replacement_candidate);
    }

    #[test]
    fn test_iteration_queue_prioritizes_low_scoring_replace_items() {
        let state = LearningState {
            factor_rankings: vec![
                ranking(RankingInput {
                    name: "good_factor",
                    mean_ic: 0.07,
                    ir: 1.0,
                    backtest_return: 0.12,
                    sharpe: 1.1,
                    stability: 0.70,
                    win_rate: 0.55,
                    profit_factor: 1.28,
                    trade_count: 16,
                }),
                ranking(RankingInput {
                    name: "bad_factor",
                    mean_ic: 0.01,
                    ir: 0.0,
                    backtest_return: -0.04,
                    sharpe: -0.1,
                    stability: 0.25,
                    win_rate: 0.43,
                    profit_factor: 0.80,
                    trade_count: 14,
                }),
            ],
            ..LearningState::default()
        };

        let queue = state.iteration_queue();
        assert_eq!(queue[0].factor_name, "bad_factor");
        assert_eq!(queue[0].iteration_action, "replace");
    }

    #[test]
    fn test_family_decisions_group_by_factor_family() {
        let mut state = LearningState::default();
        let mut a = ranking(RankingInput {
            name: "trend_momentum",
            mean_ic: 0.08,
            ir: 1.2,
            backtest_return: 0.10,
            sharpe: 1.2,
            stability: 0.70,
            win_rate: 0.56,
            profit_factor: 1.3,
            trade_count: 15,
        });
        let mut b = ranking(RankingInput {
            name: "structure_ict",
            mean_ic: 0.02,
            ir: 0.3,
            backtest_return: -0.02,
            sharpe: 0.1,
            stability: 0.30,
            win_rate: 0.45,
            profit_factor: 0.95,
            trade_count: 12,
        });
        a.iteration_action = "keep".to_string();
        b.iteration_action = "replace".to_string();
        b.replacement_candidate = true;
        state.factor_rankings = vec![a, b];

        let families = state.family_decisions();
        assert_eq!(families.len(), 2);
        assert!(families
            .iter()
            .any(|family| family.family == "structure_ict"
                && !family.replacement_candidates.is_empty()
                && family.decision_status == "review_replace"
                && family.dominant_action == "replace"
                && family
                    .risk_flags
                    .iter()
                    .any(|flag| flag == "contains_replacement_candidates")));
    }
}
