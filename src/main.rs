use anyhow::{anyhow, bail, Context, Result};
mod analyze_live_command;
mod analyze_shared;
mod factor_backtest_runtime;
mod factor_research_runtime;
mod probabilistic_backtest_runtime;
mod update_command;
mod update_output;
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
mod analyze_command;
use analyze_command::analyze_command;
use analyze_live_command::{analyze_live_command, AnalyzeLiveCommandInput};
use analyze_shared::{
    apply_command_context_to_analyze_report, persist_analyze_run,
    persist_execution_candidate_from_analyze, persist_pending_update_artifact_from_analyze,
};
use factor_backtest_runtime::run_factor_backtest;
use factor_research_runtime::run_factor_research;
use ict_engine::agent::{
    dataset_audit_prompt, factor_iteration_prompt_pack, promotion_gate_prompt,
    research_diff_prompt, rollback_review_prompt, update_diff_prompt, AgentPrompt,
    AgentPromptInput, AgentPromptPack, PROMPT_PACK_VERSION,
};
use ict_engine::analyze::multi_timeframe_parse::parse_multi_timeframe_evidence;
use ict_engine::analyze::multi_timeframe_section::build_analyze_multi_timeframe_section;
use ict_engine::analyze::options_hedging_section::OptionsHedgingSection;
use ict_engine::analyze::smt_correlation_section::{
    build_smt_correlation_section, empty_smt_correlation_section,
};
use ict_engine::analyze::technical_price_section::build_technical_price_section;
use ict_engine::analyze_builder_types::{AnalyzeBuildContext, AnalyzeNativeFrames};
use ict_engine::application::execution::{
    apply_analyze_run_execution_fields, apply_physics_overlay,
    build_execution_artifact_from_snapshot, derive_backtest_execution_fields,
    derive_execution_inputs, derive_research_execution_fields, derive_update_execution_fields,
    execution_phase_summary_suffix, ExecutionArtifactBuildContext, ExecutionInputSources,
    ExecutionOuFallback,
};
use ict_engine::application::{
    artifacts::{
        apply_artifact_consumption_preview, artifact_action_summary,
        artifact_consumed_impact_summary_for_symbol, artifact_decision_section_from_parts,
        artifact_decision_section_from_snapshot, artifact_decision_summary_for_symbol,
        artifact_decision_summary_from_snapshot, artifact_decision_summary_from_trends,
        artifact_diff_command, artifact_generated_recency_key, artifact_lineage_command,
        artifact_review_rule_sources, artifact_review_rules,
        artifact_rule_break_effects_for_symbol, artifact_status_command,
        artifact_trend_summaries_for_symbol, build_artifact_consumed_impact_summary,
        build_artifact_factor_rule_break_impacts, build_artifact_factor_trends,
        build_artifact_family_rule_break_impacts, build_artifact_family_trends,
        build_artifact_history_summary, build_artifact_lineage_summaries_with_embedded_snapshots,
        build_artifact_rule_break_effects, consumed_analyze_context_for_update,
        execution_candidate_artifact_diff, execution_candidate_review_rule_version,
        execution_candidate_summary, pending_update_artifact_diff, pending_update_artifact_path,
        pending_update_quality_score, pending_update_review_rule_version, pending_update_summary,
        ArtifactDiffCommandInput, ArtifactLineageCommandInput, ArtifactStatusCommandInput,
    },
    backtest::{
        apply_feedback_to_trade_outcome_network, artifact_consumed_decision_gate,
        augment_action_plan_with_artifact_trends,
        augment_action_plan_with_consumed_pre_bayes_context,
        augment_action_plan_with_pre_bayes_filter, build_agent_action_plan,
        build_agent_context_bundle, build_agent_context_bundle_minimal,
        build_backtest_result_artifact, build_feedback_record, command_recommendations,
        concretize_action_plan_commands, cpt_probability_diffs, data_fingerprint,
        dataset_comparability, decision_history_summary, decision_thresholds,
        enrich_feedback_record, factor_alignment_label_from_feedback,
        factor_uncertainty_label_from_feedback, family_diffs, family_history_from_runs,
        link_artifact_decision_summary_to_decisions, pre_bayes_entry_quality_bridge_diff,
        pre_bayes_soft_evidence_diff, probability_diffs, ranking_diffs, recommended_next_command,
        render_recommended_command, run_provenance, trade_outcome_label_from_pnl,
        workflow_state_from_context, workflow_state_from_pre_bayes_filter, AnalyzeCommandSource,
        BacktestResultArtifactInput, BuildAgentContextBundleInput, BuildFeedbackRecordInput,
        CommandContext,
    },
    belief::{
        apply_factor_outcome_overlay, build_canonical_belief_snapshot_with_pda,
        build_expansion_factor_pipeline_report as build_expansion_factor_pipeline_report_v2,
        build_expansion_factor_pipeline_report_with_registry as build_expansion_factor_pipeline_report_with_registry_v2,
        build_pre_bayes_entry_quality_bridge, combine_bias_vectors, combine_liquidity_labels,
        combine_regime_labels, historical_market_jump_objective_weight, infer_market_from_symbol,
        market_behavior_profile_for_family, market_category_for_symbol,
        multi_timeframe_entry_quality_bias, persist_market_jump_calibration_from_backtest_runs,
        persist_market_jump_calibration_from_research_runs,
        persist_market_jump_objective_calibration_from_research_runs, pre_bayes_evidence_policy,
        pre_bayes_policy_lineage_summary, probability_map, PreBayesEntryQualityBridgeInput,
    },
    data_sources::{
        build_expansion_sop_market_report, run_clean_futures, run_clean_futures_multi_timeframe,
        run_expansion_sop_with, run_futures_sop_with, ExpansionSopMarketInput, ExpansionSopReport,
        FuturesSopMarketInput, FuturesSopReport, RunExpansionSopInput,
    },
    decision_utils::{
        append_pda_sequence_hint, build_analyze_decision_hint, derive_family_outcomes,
        derive_promotion_decision, derive_rollback_recommendation, normalize_entry_quality_label,
        normalize_trade_outcome_label, parse_research_objective, pre_bayes_gate_is_hard_pass,
        pre_bayes_gate_regressed, research_objective_label, ResearchObjectiveMode,
    },
    factor_lifecycle::build_factor_lifecycle_view,
    factor_lifecycle::{
        apply_expansion_manipulation_objective, expansion_factor_scores_for_market,
        factor_mutation_focus_prompt, factor_mutation_priority_markets,
        factor_mutation_priority_reasons, factor_mutation_recommended_focus,
        mechanical_mutation_score, next_mutation_spec_template, no_superior_mutation_found,
        recommended_mutation_directions_from_failure_tags,
    },
    multi_timeframe_inputs::{
        build_live_multi_timeframe_signal, build_multi_timeframe_research_signal,
        build_multi_timeframe_summary, infer_interval_for_analyze_frame,
        resolve_analyze_cli_inputs, resolve_analyze_multi_timeframe_inputs,
        resolve_multi_timeframe_inputs,
    },
    orchestration::{
        build_execution_tree_artifact, build_execution_triage, build_stub_ensemble_vote_from_input,
        build_stub_ensemble_vote_from_research, persist_execution_tree_artifact, run_stage_plan,
        staged_orchestration_enabled, AnalyzeEnsembleVoteInput, CatBoostCompatiblePolicyEngine,
        DefaultExecutionTreeScorer, ExecutionShapProvider, ExecutionTreeArtifact,
        ExecutionTreeInput, ExecutionTreeScorer, FinalOutputAdapter, FinalSurfaceAdapter,
        PipelineState, StagePlan, StagedArtifactsInput, StructuralExecutionShap,
        EXECUTION_TREE_TRACE_FILE,
    },
    reflection::{build_reflection_bundle, ReflectionBundleInput},
    regime::{
        build_mece_recovery_artifact, build_multi_timeframe_training_observations,
        native_frame_computations, persist_mece_recovery_artifact,
        search_factors_for_mece_recovery, weighted_majority_label, weighted_regime_probs,
    },
};
use ict_engine::backtest::engine::{AmbiguousBarPolicy, ExecutionRealismConfig};
use ict_engine::backtest::BacktestEngine;
use ict_engine::bayesian::{cascade_bear, cascade_bull, CascadeConfig};
use ict_engine::bbn::learning::cpt_updater::{CPTUpdater, TradeOutcome};
use ict_engine::bbn::trading::{
    topology::{build_trading_network, upgrade_trading_network},
    update::{
        entry_quality_bias_from_signal, infer_entry_quality, infer_entry_quality_with_bias,
        infer_trade_outcome, infer_trade_outcome_with_entry_quality_bias,
        trade_evidence_from_labels, trade_evidence_from_pre_bayes_filter,
    },
};
use ict_engine::config::{
    build_frame_features, build_pre_bayes_evidence_filter, compute_hash, env_f64,
    family_history_window, left_pad, shell_quote, INDICATOR_PERIOD,
};
use ict_engine::data::{
    load_candles,
    realtime::{
        build_live_data_source,
        openalice::{AuxiliaryMarketEvidence, SpotInstrumentKind},
        LiveDataBackend,
    },
};
use ict_engine::domain::regime::{
    build_hybrid_regime_packet, manual_mece_labeler, RegimeSegmentationPacket,
};
#[cfg(test)]
use ict_engine::factor_lab::BacktestResult as FactorBacktestRunResult;
use ict_engine::factor_lab::{
    BacktestConfig as FactorBacktestConfig, FactorContext, FactorDiagnostics, FactorEngine,
    FactorLab,
};
use ict_engine::factors::{FactorRegistry, WeightUpdater};
use ict_engine::hmm::{init_hmm_params, state_name, BaumWelch, ForwardBackward, Viterbi};
use ict_engine::ict::{
    check_bear_expansion_exists, check_bull_expansion_exists, count_recent_breaks,
    count_recent_sweeps, detect_cisd, detect_liquidity_pools, detect_liquidity_sweep,
    detect_order_blocks, detect_structure_breaks, expansion_strength, find_swing_highs,
    find_swing_lows, find_unfilled_fvgs, find_untested_obs, has_recent_pinbar,
};
use ict_engine::indicators::compute_atr;
use ict_engine::planner::{
    generate_probabilistic_trade_plan, probabilistic_decision_snapshot,
    ProbabilisticDecisionSnapshot, ProbabilisticPlanConfig, ProbabilisticTradePlanInput,
};
use probabilistic_backtest_runtime::{finalize_backtest_report, run_probabilistic_backtest};
use serde_json::Value;
use update_command::update_command;
use update_output::{
    apply_update_outcome_to_executor_scorecards, build_ensemble_vote_record, emit_update_output,
    feedback_record_from_artifact, latest_execution_candidate_for_source_run,
    load_canonical_executor_scorecards, persist_ensemble_vote_record,
};

#[cfg(test)]
use ict_engine::application::backtest::recommended_command;
#[cfg(test)]
use ict_engine::application::factor_lifecycle::forced_cluster_jump_template;
#[cfg(test)]
use ict_engine::application::output_foundation::{redact_local_paths, redact_local_paths_in_value};
#[cfg(test)]
use ict_engine::state::FeedbackFactorUsage;
#[cfg(test)]
use ict_engine::state::RecommendedCommand;
use ict_engine::state::{
    append_analyze_run, append_artifact_ledger_entry, append_backtest_run,
    append_ensemble_vote_history, append_execution_candidate_history, append_factor_mutation_run,
    append_pending_update_artifact_history, append_pre_bayes_policy_history, append_research_run,
    append_trade_history, append_train_run, append_update_run, load_artifact_ledger,
    load_ensemble_executor_scorecards, load_ensemble_vote_history,
    load_execution_candidate_history, load_learning_state, load_pending_update_artifact,
    load_pending_update_history, load_pre_bayes_policy_history, load_state, load_state_or_default,
    mark_artifact_consumed, migrate_ensemble_executor_scorecards, recommended_next_command_meta,
    save_ensemble_executor_scorecards, save_ensemble_vote_artifact,
    save_execution_candidate_artifact, save_learning_state, save_pending_update_artifact,
    save_state, save_workflow_snapshot, state_exists, AgentActionItem, AgentActionPlan,
    AgentContextBundle, AgentContextBundleMinimal, AnalyzeRunRecord, ArtifactLedgerEntry,
    BacktestRunRecord, CommandRecommendations, DatasetComparability, DecisionHistorySummary,
    DecisionThresholds, EnsembleExecutorScorecard, EnsembleVoteRecord, ExecutionCandidateArtifact,
    ExecutionCandidateArtifactDecision, ExecutionCandidateArtifactDiff, ExpectedStateChange,
    FactorFamilyDecision, FactorFamilyDiff, FactorFamilyHistory, FactorFamilyOutcome,
    FactorIterationPrompt, FactorMutationEvaluation, FactorMutationMetricSet,
    FactorMutationRunRecord, FactorMutationSpec, FeedbackHistorySummary, FeedbackRecord,
    LearningState, LiveDataSourceProvenance, ModelProbabilitySnapshot, PendingUpdateArtifact,
    PendingUpdateArtifactDecision, PendingUpdateArtifactDiff, PersistedFactorRanking,
    PreBayesEvidenceFilter, PreBayesPolicyRecord, ProbabilityDiff, PromotionDecision,
    RankingDiffItem, ResearchRunRecord, RollbackRecommendation, RunProvenance, TrainRunRecord,
    UpdateRunRecord, WorkflowBlockingTruth, WorkflowConflictSource, WorkflowDisagreement,
    WorkflowFieldDiff, WorkflowPhaseSnapshot, WorkflowSnapshot, WorkflowState, ANALYZE_RUNS_FILE,
    BACKTEST_RUNS_FILE, ENSEMBLE_VOTE_FILE, EXECUTION_CANDIDATE_FILE, PENDING_UPDATE_ARTIFACT_FILE,
    RESEARCH_RUNS_FILE, TRAIN_RUNS_FILE, UPDATE_RUNS_FILE,
};
#[cfg(test)]
use ict_engine::types::Symbol;
use ict_engine::types::{
    normalize_direction_label, normalize_regime_label, parse_symbol, Candle, CascadeLayer,
    Direction, HMMParams, Regime, RegimeProbs, TradePlan, TradeRecord, OBS_DIM,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Json,
    Compact,
    Agent,
    Human,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "compact" => Ok(Self::Compact),
            "agent" => Ok(Self::Agent),
            "human" => Ok(Self::Human),
            other => bail!(
                "unsupported output format '{}'; expected json, compact, agent, or human",
                other
            ),
        }
    }
}

use std::collections::{BTreeMap, HashMap};
use std::env;

const HMM_STATE_FILE: &str = "hmm_params.json";
const BBN_STATE_FILE: &str = "bbn_network.json";
type AnalyzeReport = ict_engine::analyze_report_shell::AnalyzeReport;
type AnalyzeMeta = ict_engine::analyze_report_shell::AnalyzeMeta;
type AnalyzeSupporting = ict_engine::analyze_report_shell::AnalyzeSupporting;
type AnalyzeBars = ict_engine::analyze_report_shell::AnalyzeBars;
type AnalyzeModelState = ict_engine::analyze_report_shell::AnalyzeModelState;
type AnalyzeLabels = ict_engine::analyze_report_shell::AnalyzeLabels;
type AnalyzeIctSummary = ict_engine::analyze_report_shell::AnalyzeIctSummary;
type AnalyzeTradeOutcomeSummary = ict_engine::analyze_report_shell::AnalyzeTradeOutcomeSummary;
type AnalyzeEntryQualitySummary = ict_engine::analyze_report_shell::AnalyzeEntryQualitySummary;
type AnalyzeSections = ict_engine::analyze_sections::AnalyzeSections;
type PriceActionSection = ict_engine::analyze_sections::PriceActionSection;
type RegimeBayesianSection = ict_engine::analyze_sections::RegimeBayesianSection;
type TradePlanSection = ict_engine::analyze_sections::TradePlanSection;

type BacktestReport = ict_engine::backtest_report_shell::BacktestReport;
#[cfg(test)]
type BacktestMetricsSummary = ict_engine::backtest_report_shell::BacktestMetricsSummary;

#[derive(Debug, Serialize)]
struct UpdateReport {
    symbol: String,
    timestamp: chrono::DateTime<Utc>,
    state_dir: String,
    provenance: RunProvenance,
    decision_thresholds: DecisionThresholds,
    dataset_comparability: DatasetComparability,
    promotion_decision: PromotionDecision,
    rollback_recommendation: RollbackRecommendation,
    trade_outcome_deltas: Vec<ProbabilityDiff>,
    factor_score_deltas: Vec<RankingDiffItem>,
    normalized_entry_quality: String,
    factor_alignment: String,
    factor_uncertainty: String,
    realized_outcome: String,
    feedback_records_applied: usize,
    duplicate_feedback_skipped: bool,
    consumed_pending_update_artifact_id: Option<String>,
    consumed_execution_candidate_artifact_id: Option<String>,
    consumed_artifact_path: Option<String>,
    consumed_analyze_run_id: Option<String>,
    consumed_pre_bayes_evidence_filter: Option<PreBayesEvidenceFilter>,
    consumed_pre_bayes_entry_quality_bridge: Option<ict_engine::state::PreBayesEntryQualityBridge>,
    consumed_multi_timeframe_summary: Vec<String>,
    updated_trade_outcome: BTreeMap<String, f64>,
    factor_ranking: Vec<PersistedFactorRanking>,
    factor_iteration_queue: Vec<FactorIterationPrompt>,
    factor_family_decisions: Vec<FactorFamilyDecision>,
    factor_family_outcomes: Vec<FactorFamilyOutcome>,
    factor_family_diffs: Vec<FactorFamilyDiff>,
    factor_family_history: Vec<FactorFamilyHistory>,
    decision_history_summary: DecisionHistorySummary,
    agent_action_plan: AgentActionPlan,
    workflow_state: WorkflowState,
    agent_context_bundle: AgentContextBundle,
    agent_context_bundle_minimal: AgentContextBundleMinimal,
    recommended_commands: CommandRecommendations,
    recommended_next_command: String,
    artifact_action_summary: Vec<String>,
    artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary,
    artifact_decision_section: ict_engine::state::ArtifactDecisionSection,
    agent_prompts: AgentPromptPack,
    feedback_history_summary: FeedbackHistorySummary,
    workflow_snapshot: ict_engine::state::WorkflowSnapshot,
}

#[derive(Clone, Copy)]
struct BaselineFactorMutationMetricsInput<'a> {
    registry: &'a FactorRegistry,
    symbol: &'a str,
    objective: ResearchObjectiveMode,
    target_factor: Option<&'a str>,
    baseline_learning_state: &'a LearningState,
    candles: &'a [Candle],
    paired_candles: Option<&'a [Candle]>,
    multi_timeframe_summary: &'a [String],
    evaluate_expansion_preview: bool,
}

#[derive(Clone, Copy)]
struct BuildAnalyzeReportInput<'a> {
    symbol: &'a str,
    state_dir: &'a str,
    htf: &'a [Candle],
    mtf: &'a [Candle],
    ltf: &'a [Candle],
    params: &'a HMMParams,
    network: &'a ict_engine::bbn::BayesianNetwork,
    build_context: AnalyzeBuildContext<'a>,
    execution_focus: bool,
}

struct RunProbabilisticBacktestInput<'a> {
    symbol: &'a str,
    state_dir: &'a str,
    candles: &'a [Candle],
    paired_candles: Option<&'a [Candle]>,
    warmup_bars: usize,
    hold_bars: usize,
    realism: &'a ExecutionRealismConfig,
    online_learn: bool,
    params: &'a HMMParams,
    network: &'a ict_engine::bbn::BayesianNetwork,
    learning_state: &'a mut LearningState,
}

struct BuildWorkflowSnapshotInput<'a> {
    state_dir: &'a str,
    symbol: &'a str,
    latest_train: Option<&'a TrainRunRecord>,
    latest_analyze: Option<&'a AnalyzeRunRecord>,
    latest_research: Option<&'a ResearchRunRecord>,
    latest_backtest: Option<&'a BacktestRunRecord>,
    latest_update: Option<&'a UpdateRunRecord>,
    pre_bayes_policy_history: &'a [PreBayesPolicyRecord],
    pending_update_history: &'a [PendingUpdateArtifact],
    execution_candidate_history: &'a [ExecutionCandidateArtifact],
    artifact_ledger: &'a [ArtifactLedgerEntry],
}

struct UpdateCommandInput<'a> {
    symbol: &'a str,
    outcome: &'a str,
    entry_signal: Option<&'a str>,
    feedback_file: Option<&'a str>,
    state_dir: &'a str,
    pnl: Option<f64>,
    regime: Option<&'a str>,
    direction: Option<&'a str>,
    ensemble: bool,
}

#[derive(Parser)]
#[command(name = "ict-engine")]
#[command(about = "ICT Expansion Trading Engine", long_about = None, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

const DEFAULT_STATE_DIR: &str = "state";
const STATE_DIR_ENV_VAR: &str = "ICT_ENGINE_STATE_DIR";

#[derive(Subcommand)]
enum Commands {
    /// Analyze market data
    Analyze {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(long, help = "Higher-timeframe cleaned candle JSON path")]
        data_htf: Option<String>,
        #[arg(long, help = "Middle-timeframe cleaned candle JSON path")]
        data_mtf: Option<String>,
        #[arg(long, help = "Lower-timeframe cleaned candle JSON path")]
        data_ltf: Option<String>,
        #[arg(
            long,
            help = "Root directory for auto-resolving cleaned multi-timeframe data"
        )]
        data_root: Option<String>,
        #[arg(
            long,
            help = "Use bundled demo candles from examples/demo/demo-15m.json"
        )]
        demo: bool,
        #[arg(
            long,
            env = "ICT_ENGINE_STATE_DIR",
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
        #[arg(
            long,
            default_value = "",
            help = "Output format: json (default), compact, agent, or human. `--compact`, `--agent`, `--human` are aliases; do not combine them with `--output-format`."
        )]
        output_format: String,
        #[arg(long, help = "Alias for --output-format compact")]
        compact: bool,
        #[arg(long, help = "Alias for --output-format agent")]
        agent: bool,
        #[arg(long, help = "Alias for --output-format human")]
        human: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Inline full workflow snapshot ledger arrays in JSON output instead of trimming them to a token-friendly tail"
        )]
        inline_ledger: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Disable the leading Execution Triage section / envelope field (default: on)"
        )]
        no_execution_focus: bool,
    },
    /// Analyze live futures with integrated backends and spot/options auxiliary evidence
    AnalyzeLive {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(long, help = "Explicit futures symbol override for live data provider")]
        futures_symbol: Option<String>,
        #[arg(long, help = "Explicit spot symbol override for auxiliary evidence")]
        spot_symbol: Option<String>,
        #[arg(long, help = "Explicit options symbol override for auxiliary evidence")]
        options_symbol: Option<String>,
        #[arg(long, help = "Spot instrument kind override, e.g. spot, etf, index")]
        spot_kind: Option<String>,
        #[arg(
            long,
            default_value = "openbb",
            help = "Backend for live futures candles"
        )]
        futures_backend: String,
        #[arg(
            long,
            default_value = "openbb",
            help = "Backend for auxiliary spot/options evidence"
        )]
        aux_backend: String,
        #[arg(
            long,
            default_value = "http://127.0.0.1:6901/api/v1",
            help = "Base URL for OpenAlice-compatible live backend"
        )]
        openalice_base_url: String,
        #[arg(
            long,
            default_value = "http://127.0.0.1:8080",
            help = "Base URL for NoFX-compatible live backend"
        )]
        nofx_base_url: String,
        #[arg(
            long,
            env = "ICT_ENGINE_STATE_DIR",
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
    },
    /// Train HMM model
    Train {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(long, help = "Cleaned candle JSON path used for HMM training")]
        data: String,
        #[arg(
            short,
            long,
            default_value = "100",
            help = "Number of Baum-Welch training epochs"
        )]
        epochs: usize,
        #[arg(
            long,
            env = "ICT_ENGINE_STATE_DIR",
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
    },
    /// Run backtest
    Backtest {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(long, help = "Primary cleaned candle JSON path")]
        data: String,
        #[arg(long, help = "Optional paired-market candle JSON path")]
        paired_data: Option<String>,
        #[arg(
            long,
            env = "ICT_ENGINE_STATE_DIR",
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
        #[arg(
            long,
            default_value = "",
            help = "Output format: json (default), compact, agent, or human. `--compact`, `--agent`, `--human` are aliases; do not combine them with `--output-format`."
        )]
        output_format: String,
        #[arg(long, help = "Alias for --output-format compact")]
        compact: bool,
        #[arg(long, help = "Alias for --output-format agent")]
        agent: bool,
        #[arg(long, help = "Alias for --output-format human")]
        human: bool,
        #[arg(
            long,
            default_value = "60",
            help = "Warmup bars before trade simulation begins"
        )]
        warmup_bars: usize,
        #[arg(long, default_value = "10", help = "Maximum holding period in bars")]
        hold_bars: usize,
        #[arg(long, default_value = "0", help = "Spread cost in basis points")]
        spread_bps: f64,
        #[arg(long, default_value = "0", help = "Slippage cost in basis points")]
        slippage_bps: f64,
        #[arg(long, default_value = "0", help = "Fee cost in basis points")]
        fee_bps: f64,
        #[arg(
            long,
            default_value = "favor_stop_loss",
            help = "Ambiguous intrabar execution policy"
        )]
        ambiguous_bar_policy: String,
        #[arg(
            long,
            default_value_t = false,
            help = "Update online learning state during backtest"
        )]
        online_learn: bool,
    },
    /// Update BBN from realized trade outcome
    Update {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(long, help = "Realized trade outcome label")]
        outcome: String,
        #[arg(
            long,
            default_value = "strong_buy",
            help = "Entry signal label applied to the feedback update"
        )]
        entry_signal: String,
        #[arg(
            long,
            env = "ICT_ENGINE_STATE_DIR",
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
        #[arg(long, help = "Optional realized PnL used for outcome normalization")]
        pnl: Option<f64>,
        #[arg(long, help = "Optional regime label override at trade entry")]
        regime: Option<String>,
        #[arg(long, help = "Optional direction label override at trade entry")]
        direction: Option<String>,
        #[arg(long, help = "Optional feedback JSON artifact to consume")]
        feedback_file: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Also update ensemble executor scorecards"
        )]
        ensemble: bool,
    },
    /// Run factor research sandbox
    FactorResearch {
        #[arg(
            long,
            default_value = "RESEARCH",
            help = "Market symbol, e.g. NQ, ES, GC"
        )]
        symbol: String,
        #[arg(long, help = "Primary cleaned candle JSON path")]
        data: String,
        #[arg(
            long,
            default_value = "expansion_manipulation",
            help = "Research objective label"
        )]
        objective: String,
        #[arg(long, help = "Optional 1m candle JSON path")]
        data_1m: Option<String>,
        #[arg(long, help = "Optional 5m candle JSON path")]
        data_5m: Option<String>,
        #[arg(long, help = "Optional 15m candle JSON path")]
        data_15m: Option<String>,
        #[arg(long, help = "Optional 1h candle JSON path")]
        data_1h: Option<String>,
        #[arg(long, help = "Optional 4h candle JSON path")]
        data_4h: Option<String>,
        #[arg(long, help = "Optional 1d candle JSON path")]
        data_1d: Option<String>,
        #[arg(long, help = "Optional paired-market candle JSON path")]
        paired_data: Option<String>,
        #[arg(long, help = "Optional mutation spec JSON path")]
        mutation_spec: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Emit mutation evaluation details in output"
        )]
        emit_mutation_evaluation: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Also emit ensemble vote artifacts"
        )]
        ensemble: bool,
        #[arg(
            long,
            env = "ICT_ENGINE_STATE_DIR",
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
        #[arg(
            long,
            default_value = "",
            help = "Output format: json (default), compact, agent, or human. `--compact`, `--agent`, `--human` are aliases; do not combine them with `--output-format`."
        )]
        output_format: String,
        #[arg(long, help = "Alias for --output-format compact")]
        compact: bool,
        #[arg(long, help = "Alias for --output-format agent")]
        agent: bool,
        #[arg(long, help = "Alias for --output-format human")]
        human: bool,
    },
    /// Show factor mutation history and clustered failure tags
    FactorMutationStatus {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
        #[arg(long, help = "Optional source command substring filter")]
        source_command: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Show only the latest mutation attempt"
        )]
        latest_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Show only accepted mutation attempts"
        )]
        accepted_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Group attempts by source command"
        )]
        bucket_by_source: bool,
        #[arg(long, help = "Limit returned mutation attempts")]
        limit: Option<usize>,
    },
    /// Run checkpointed keep/discard factor mutation autoresearch loop
    FactorAutoresearch {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(long, help = "Primary cleaned candle JSON path")]
        data: String,
        #[arg(
            long,
            default_value = "expansion_manipulation",
            help = "Research objective label"
        )]
        objective: String,
        #[arg(long, help = "Optional mutation spec JSON path")]
        mutation_spec: Option<String>,
        #[arg(
            long,
            default_value_t = 1,
            help = "Number of autoresearch iterations to run"
        )]
        iterations: usize,
        #[arg(long, help = "Optional 1m candle JSON path")]
        data_1m: Option<String>,
        #[arg(long, help = "Optional 5m candle JSON path")]
        data_5m: Option<String>,
        #[arg(long, help = "Optional 15m candle JSON path")]
        data_15m: Option<String>,
        #[arg(long, help = "Optional 1h candle JSON path")]
        data_1h: Option<String>,
        #[arg(long, help = "Optional 4h candle JSON path")]
        data_4h: Option<String>,
        #[arg(long, help = "Optional 1d candle JSON path")]
        data_1d: Option<String>,
        #[arg(long, help = "Optional paired-market candle JSON path")]
        paired_data: Option<String>,
        #[arg(long, help = "Explicit autoresearch session id to resume or inspect")]
        session_id: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Resume the latest known autoresearch session"
        )]
        resume_latest: bool,
        #[arg(
            long,
            default_value_t = 2,
            help = "Maximum consecutive clustered failures before jumping templates"
        )]
        max_cluster_fail_streak: usize,
        #[arg(
            long,
            default_value_t = false,
            help = "Also emit ensemble vote artifacts"
        )]
        ensemble: bool,
        #[arg(
            long,
            env = "ICT_ENGINE_STATE_DIR",
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
    },
    /// Show the currently effective ICT-related environment settings
    Env,
    /// Show factor-autoresearch sessions and attempts
    FactorAutoresearchStatus {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
        #[arg(long, help = "Explicit autoresearch session id to inspect")]
        session_id: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Show only the latest session summary"
        )]
        latest_only: bool,
        #[arg(long, help = "Limit returned sessions or attempts")]
        limit: Option<usize>,
    },
    /// Summarize research closure truth into one compact verdict
    ResearchVerdict {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
    },
    /// Explain the latest Pre-Bayes evidence quality score composition
    EvidenceQualityBreakdown {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
        #[arg(
            long,
            default_value_t = true,
            help = "Refresh workflow snapshot before reading latest analyze state"
        )]
        refresh: bool,
    },
    /// Run factor walk-forward backtest and learning updates
    FactorBacktest {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(long, help = "Primary cleaned candle JSON path")]
        data: String,
        #[arg(long, help = "Optional 1m candle JSON path")]
        data_1m: Option<String>,
        #[arg(long, help = "Optional 5m candle JSON path")]
        data_5m: Option<String>,
        #[arg(long, help = "Optional 15m candle JSON path")]
        data_15m: Option<String>,
        #[arg(long, help = "Optional 1h candle JSON path")]
        data_1h: Option<String>,
        #[arg(long, help = "Optional 4h candle JSON path")]
        data_4h: Option<String>,
        #[arg(long, help = "Optional 1d candle JSON path")]
        data_1d: Option<String>,
        #[arg(long, help = "Optional paired-market candle JSON path")]
        paired_data: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Also emit ensemble vote artifacts"
        )]
        ensemble: bool,
        #[arg(
            long,
            default_value = "state",
            help = "State directory for model and workflow artifacts"
        )]
        state_dir: String,
        #[arg(
            long,
            default_value = "",
            help = "Output format: json (default), compact, agent, or human. `--compact`, `--agent`, `--human` are aliases; do not combine them with `--output-format`."
        )]
        output_format: String,
        #[arg(long, help = "Alias for --output-format compact")]
        compact: bool,
        #[arg(long, help = "Alias for --output-format agent")]
        agent: bool,
        #[arg(long, help = "Alias for --output-format human")]
        human: bool,
    },
    /// Clean TOMAC-style futures minute CSVs into continuous candles
    CleanFutures {
        #[arg(long, help = "Root directory containing TOMAC-style futures CSV files")]
        root: Option<String>,
        #[arg(long, help = "Output directory for cleaned candle JSON")]
        output_dir: String,
        #[arg(long, default_value = "15m", help = "Target output interval")]
        interval: String,
        #[arg(
            long,
            default_value_t = false,
            help = "Also emit sibling multi-timeframe intervals"
        )]
        multi_timeframe: bool,
    },
    /// Standard futures research SOP: clean, research, summarize best factors
    FuturesSop {
        #[arg(long, help = "Root directory containing TOMAC-style futures CSV files")]
        root: Option<String>,
        #[arg(long, help = "Output directory for cleaned candle JSON and reports")]
        output_dir: String,
        #[arg(long, default_value = "15m", help = "Primary research interval")]
        interval: String,
    },
    /// Expansion-focused futures SOP: rank factors by bull/bear expansion discrimination
    ExpansionSop {
        #[arg(long, help = "Root directory containing TOMAC-style futures CSV files")]
        root: Option<String>,
        #[arg(long, help = "Output directory for cleaned candle JSON and reports")]
        output_dir: String,
        #[arg(long, default_value = "15m", help = "Primary research interval")]
        interval: String,
        #[arg(long, default_value_t = 20, help = "Expansion lookback window in bars")]
        lookback: usize,
        #[arg(
            long,
            default_value_t = 1.5,
            help = "ATR multiplier used for expansion thresholding"
        )]
        atr_multiplier: f64,
        #[arg(
            long,
            default_value = "expansion_manipulation",
            help = "Research objective label"
        )]
        objective: String,
        #[arg(long, help = "Optional mutation spec JSON path")]
        mutation_spec: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Emit mutation evaluation details in output"
        )]
        emit_mutation_evaluation: bool,
    },
    /// Structured latest-sample trace from factor signal through Pre-Bayes, bridge, and resonance
    FactorPipelineDebug {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(long, help = "Primary cleaned candle JSON path")]
        data: String,
        #[arg(long, help = "Factor name to inspect")]
        factor: String,
        #[arg(
            long,
            default_value = "expansion_manipulation",
            help = "Research objective label"
        )]
        objective: String,
        #[arg(long, help = "Optional 1m candle JSON path")]
        data_1m: Option<String>,
        #[arg(long, help = "Optional 5m candle JSON path")]
        data_5m: Option<String>,
        #[arg(long, help = "Optional 15m candle JSON path")]
        data_15m: Option<String>,
        #[arg(long, help = "Optional 1h candle JSON path")]
        data_1h: Option<String>,
        #[arg(long, help = "Optional 4h candle JSON path")]
        data_4h: Option<String>,
        #[arg(long, help = "Optional 1d candle JSON path")]
        data_1d: Option<String>,
    },
    /// Show the latest cross-phase workflow snapshot
    WorkflowStatus {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory containing workflow artifacts"
        )]
        state_dir: String,
        #[arg(
            long,
            default_value_t = true,
            help = "Refresh snapshot from current artifacts before printing"
        )]
        refresh: bool,
        #[arg(
            long,
            help = "Print a named workflow phase surface instead of the full snapshot"
        )]
        phase: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Print only actionable artifacts"
        )]
        actionable_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Print only workflow disagreements"
        )]
        conflicts_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Print only the latest promotable artifact"
        )]
        latest_promotable: bool,
        #[arg(long, default_value_t = false, help = "Print only hard-block rows")]
        hard_block_only: bool,
        #[arg(long, help = "Filter hard-block rows by reason substring")]
        hard_block_reason: Option<String>,
        #[arg(long, help = "Limit hard-block rows")]
        limit: Option<usize>,
        #[arg(
            long,
            default_value = "",
            help = "Output format: json (default), compact, agent, or human. `--compact`, `--agent`, `--human` are aliases; do not combine them with `--output-format`."
        )]
        output_format: String,
        #[arg(long, help = "Alias for --output-format compact")]
        compact: bool,
        #[arg(long, help = "Alias for --output-format agent")]
        agent: bool,
        #[arg(long, help = "Alias for --output-format human")]
        human: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Strip volatile timestamp-like fields from workflow-status output so repeated calls are stable for caching/diffing"
        )]
        stable: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Disable Execution Triage surfacing in workflow-status output (default: on)"
        )]
        no_execution_focus: bool,
    },
    /// Show the latest Pre-Bayes status directly
    PreBayesStatus {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory containing workflow artifacts"
        )]
        state_dir: String,
        #[arg(
            long,
            default_value_t = true,
            help = "Refresh snapshot from current artifacts before printing"
        )]
        refresh: bool,
        #[arg(
            long,
            help = "Optional Pre-Bayes section to print, e.g. policy or bridge"
        )]
        section: Option<String>,
    },
    /// Show the latest Pre-Bayes diff package directly
    PreBayesDiff {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory containing workflow artifacts"
        )]
        state_dir: String,
        #[arg(
            long,
            default_value_t = true,
            help = "Refresh snapshot from current artifacts before printing"
        )]
        refresh: bool,
    },
    /// Show artifact lineage edges and related nodes
    ArtifactLineage {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory containing artifact ledger"
        )]
        state_dir: String,
        #[arg(long, help = "Optional artifact id to focus lineage output")]
        artifact_id: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Show only the latest lineage rows"
        )]
        latest_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Show only improving lineage rows"
        )]
        improving_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Show only regressing lineage rows"
        )]
        regressing_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Show only lineage rows with rule breaks"
        )]
        rule_break_only: bool,
    },
    /// Show artifact ledger status
    ArtifactStatus {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory containing artifact ledger"
        )]
        state_dir: String,
        #[arg(long, help = "Optional artifact id to inspect")]
        artifact_id: Option<String>,
        #[arg(long, help = "Optional artifact kind filter")]
        kind: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Keep only the latest artifact per kind (one row per artifact_kind, most recent by generated_at)"
        )]
        latest_only: bool,
        #[arg(long, default_value_t = false, help = "Show only actionable artifacts")]
        actionable_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Show only artifacts with review rule breaks"
        )]
        rule_break_only: bool,
        #[arg(long, default_value = "generated", help = "Sort key for artifact rows")]
        sort_by: String,
        #[arg(
            long,
            default_value_t = true,
            help = "Sort descending instead of ascending"
        )]
        descending: bool,
        #[arg(long, help = "Maximum artifact rows to print")]
        limit: Option<usize>,
        #[arg(long, help = "Print only the most recent N artifact rows")]
        recent_n: Option<usize>,
        #[arg(long, default_value_t = false, help = "Show only consumed artifacts")]
        consumed_only: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Aggregate artifact rows by kind"
        )]
        bucket_by_kind: bool,
        #[arg(
            long,
            default_value = "kind",
            help = "Sort key for bucketed artifact output"
        )]
        bucket_order_by: String,
        #[arg(long, help = "Maximum bucket rows to print")]
        bucket_limit: Option<usize>,
    },
    /// Diff two artifacts by id
    ArtifactDiff {
        #[arg(long, help = "Market symbol, e.g. NQ, ES, GC")]
        symbol: String,
        #[arg(
            long,
            default_value = "state",
            help = "State directory containing artifact ledger"
        )]
        state_dir: String,
        #[arg(long, help = "Left artifact id for diff comparison")]
        left_artifact_id: String,
        #[arg(long, help = "Right artifact id for diff comparison")]
        right_artifact_id: String,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            symbol,
            data_htf,
            data_mtf,
            data_ltf,
            data_root,
            demo,
            state_dir,
            output_format,
            compact,
            agent,
            human,
            inline_ledger,
            no_execution_focus,
        } => {
            ensure_state_dir_ready(&state_dir)?;
            let (data_htf, data_mtf, data_ltf) = resolve_analyze_cli_inputs(
                &symbol,
                data_htf.as_deref(),
                data_mtf.as_deref(),
                data_ltf.as_deref(),
                data_root.as_deref(),
                demo,
            )?;
            let output_format = resolve_output_format(&output_format, compact, agent, human)?;
            analyze_command(
                &symbol,
                &data_htf,
                &data_mtf,
                &data_ltf,
                &state_dir,
                output_format,
                inline_ledger,
                !no_execution_focus,
            )?
        }
        Commands::AnalyzeLive {
            symbol,
            futures_symbol,
            spot_symbol,
            options_symbol,
            spot_kind,
            futures_backend,
            aux_backend,
            openalice_base_url,
            nofx_base_url,
            state_dir,
        } => {
            ensure_state_dir_ready(&state_dir)?;
            let futures_base_url = resolve_live_backend_base_url(
                &futures_backend,
                &openalice_base_url,
                &nofx_base_url,
            );
            let aux_base_url =
                resolve_live_backend_base_url(&aux_backend, &openalice_base_url, &nofx_base_url);
            analyze_live_command(AnalyzeLiveCommandInput {
                symbol: &symbol,
                futures_symbol: futures_symbol.as_deref(),
                spot_symbol: spot_symbol.as_deref(),
                options_symbol: options_symbol.as_deref(),
                spot_kind: spot_kind.as_deref(),
                futures_backend: &futures_backend,
                aux_backend: &aux_backend,
                futures_base_url: &futures_base_url,
                aux_base_url: &aux_base_url,
                state_dir: &state_dir,
            })?
        }
        Commands::Train {
            symbol,
            data,
            epochs,
            state_dir,
        } => {
            ensure_state_dir_ready(&state_dir)?;
            train_command(&symbol, &data, epochs, &state_dir)?
        }
        Commands::Backtest {
            symbol,
            data,
            paired_data,
            state_dir,
            output_format,
            compact,
            agent,
            human,
            warmup_bars,
            hold_bars,
            spread_bps,
            slippage_bps,
            fee_bps,
            ambiguous_bar_policy,
            online_learn,
        } => {
            ensure_state_dir_ready(&state_dir)?;
            ict_engine::application::backtest::backtest_command(
                ict_engine::application::backtest::BacktestCommandInput {
                    symbol: &symbol,
                    data: &data,
                    paired_data: paired_data.as_deref(),
                    state_dir: &state_dir,
                    output_format: match resolve_output_format(
                        &output_format,
                        compact,
                        agent,
                        human,
                    )? {
                        OutputFormat::Json => "json",
                        OutputFormat::Compact => "compact",
                        OutputFormat::Agent => "agent",
                        OutputFormat::Human => "human",
                    },
                    warmup_bars,
                    hold_bars,
                    spread_bps,
                    slippage_bps,
                    fee_bps,
                    ambiguous_bar_policy: &ambiguous_bar_policy,
                    online_learn,
                },
                || {
                    run_factor_research(RunFactorResearchInput {
                        symbol: &symbol,
                        data: &data,
                        objective: ResearchObjectiveMode::ExpansionManipulation,
                        data_1m: None,
                        data_5m: None,
                        data_15m: None,
                        data_1h: None,
                        data_4h: None,
                        data_1d: None,
                        paired_data: paired_data.as_deref(),
                        mutation_spec: None,
                        state_dir: &state_dir,
                    })
                    .map(|_| ())
                },
                parse_execution_realism_config,
                |realism| {
                    let candles = load_candles(&data)?;
                    let paired_candles = paired_data.as_deref().map(load_candles).transpose()?;
                    let params = load_or_init_hmm_params(&symbol, &state_dir);
                    let network = load_or_init_trading_network(&symbol, &state_dir)?;
                    let mut learning_state = load_learning_state(&state_dir, &symbol)?;
                    let previous_rankings = learning_state.factor_rankings.clone();
                    let previous_trade_outcome_cpt =
                        ict_engine::application::backtest::trade_outcome_cpt_snapshot(&network)?;
                    let tuple = run_probabilistic_backtest(RunProbabilisticBacktestInput {
                        symbol: &symbol,
                        state_dir: &state_dir,
                        candles: &candles,
                        paired_candles: paired_candles.as_deref(),
                        warmup_bars,
                        hold_bars,
                        realism,
                        online_learn,
                        params: &params,
                        network: &network,
                        learning_state: &mut learning_state,
                    })?;
                    Ok((
                        tuple,
                        candles,
                        paired_candles,
                        learning_state,
                        previous_rankings,
                        previous_trade_outcome_cpt,
                    ))
                },
                |(
                    tuple,
                    candles,
                    paired_candles,
                    learning_state,
                    previous_rankings,
                    previous_trade_outcome_cpt,
                ),
                 realism| {
                    let (report, updated_network, trades) = tuple;
                    save_learning_state(&state_dir, &symbol, &learning_state)?;
                    save_state(&state_dir, &symbol, BBN_STATE_FILE, &updated_network)?;
                    append_trade_history(&state_dir, &symbol, &trades)?;
                    finalize_backtest_report(FinalizeBacktestReportInput {
                        report,
                        symbol: &symbol,
                        data: &data,
                        paired_data: paired_data.as_deref(),
                        candles: &candles,
                        paired_candles_slice: paired_candles.as_deref(),
                        learning_state: &learning_state,
                        previous_rankings: &previous_rankings,
                        previous_trade_outcome_cpt: &previous_trade_outcome_cpt,
                        updated_network: &updated_network,
                        state_dir: &state_dir,
                        warmup_bars,
                        hold_bars,
                        realism,
                        online_learning: online_learn,
                    })
                },
            )?
        }
        Commands::Update {
            symbol,
            outcome,
            entry_signal,
            state_dir,
            pnl,
            regime,
            direction,
            feedback_file,
            ensemble,
        } => {
            ensure_state_dir_ready(&state_dir)?;
            update_command(UpdateCommandInput {
                symbol: &symbol,
                outcome: &outcome,
                entry_signal: Some(&entry_signal),
                feedback_file: feedback_file.as_deref(),
                state_dir: &state_dir,
                pnl,
                regime: regime.as_deref(),
                direction: direction.as_deref(),
                ensemble,
            })?
        }
        Commands::FactorResearch {
            symbol,
            data,
            objective,
            data_1m,
            data_5m,
            data_15m,
            data_1h,
            data_4h,
            data_1d,
            paired_data,
            mutation_spec,
            emit_mutation_evaluation,
            ensemble,
            state_dir,
            output_format,
            compact,
            agent,
            human,
        } => {
            ensure_state_dir_ready(&state_dir)?;
            ict_engine::application::backtest::factor_research_command(
                ict_engine::application::backtest::FactorResearchCommandInput {
                    symbol: &symbol,
                    data: &data,
                    objective: &objective,
                    mutation_spec_path: mutation_spec.as_deref(),
                    emit_mutation_evaluation,
                    ensemble,
                    state_dir: &state_dir,
                    output_format: match resolve_output_format(
                        &output_format,
                        compact,
                        agent,
                        human,
                    )? {
                        OutputFormat::Json => "json",
                        OutputFormat::Compact => "compact",
                        OutputFormat::Agent => "agent",
                        OutputFormat::Human => "human",
                    },
                },
                load_factor_mutation_spec,
                |objective_mode, mutation_spec| {
                    run_factor_research(RunFactorResearchInput {
                        symbol: &symbol,
                        data: &data,
                        objective: objective_mode,
                        data_1m: data_1m.as_deref(),
                        data_5m: data_5m.as_deref(),
                        data_15m: data_15m.as_deref(),
                        data_1h: data_1h.as_deref(),
                        data_4h: data_4h.as_deref(),
                        data_1d: data_1d.as_deref(),
                        paired_data: paired_data.as_deref(),
                        mutation_spec,
                        state_dir: &state_dir,
                    })
                },
            )?
        }
        Commands::FactorMutationStatus {
            symbol,
            state_dir,
            source_command,
            latest_only,
            accepted_only,
            bucket_by_source,
            limit,
        } => ict_engine::application::factor_lifecycle::factor_mutation_status_command(
            &symbol,
            &state_dir,
            source_command.as_deref(),
            latest_only,
            accepted_only,
            bucket_by_source,
            limit,
        )?,
        Commands::FactorAutoresearch {
            symbol,
            data,
            objective,
            mutation_spec,
            iterations,
            data_1m,
            data_5m,
            data_15m,
            data_1h,
            data_4h,
            data_1d,
            paired_data,
            session_id,
            resume_latest,
            max_cluster_fail_streak,
            ensemble: _,
            state_dir,
        } => ict_engine::application::factor_lifecycle::factor_autoresearch_command(
            ict_engine::application::factor_lifecycle::FactorAutoresearchCommandInput {
                symbol: &symbol,
                data: &data,
                objective: &objective,
                mutation_spec_path: mutation_spec.as_deref(),
                iterations,
                data_1m: data_1m.as_deref(),
                data_5m: data_5m.as_deref(),
                data_15m: data_15m.as_deref(),
                data_1h: data_1h.as_deref(),
                data_4h: data_4h.as_deref(),
                data_1d: data_1d.as_deref(),
                paired_data: paired_data.as_deref(),
                session_id: session_id.as_deref(),
                resume_latest,
                max_cluster_fail_streak,
                state_dir: &state_dir,
            },
            load_factor_mutation_spec,
            |objective_mode, mutation_spec| {
                run_factor_research(RunFactorResearchInput {
                    symbol: &symbol,
                    data: &data,
                    objective: objective_mode,
                    data_1m: data_1m.as_deref(),
                    data_5m: data_5m.as_deref(),
                    data_15m: data_15m.as_deref(),
                    data_1h: data_1h.as_deref(),
                    data_4h: data_4h.as_deref(),
                    data_1d: data_1d.as_deref(),
                    paired_data: paired_data.as_deref(),
                    mutation_spec: Some(mutation_spec),
                    state_dir: &state_dir,
                })
            },
        )?,
        Commands::FactorAutoresearchStatus {
            symbol,
            state_dir,
            session_id,
            latest_only,
            limit,
        } => ict_engine::application::factor_lifecycle::factor_autoresearch_status_command(
            &symbol,
            &state_dir,
            session_id.as_deref(),
            latest_only,
            limit,
        )?,
        Commands::ResearchVerdict { symbol, state_dir } => {
            ict_engine::application::release_closure::research_verdict_command(&symbol, &state_dir)?
        }
        Commands::EvidenceQualityBreakdown {
            symbol,
            state_dir,
            refresh,
        } => ict_engine::application::release_closure::evidence_quality_breakdown_command(
            &symbol, &state_dir, refresh,
        )?,
        Commands::FactorBacktest {
            symbol,
            data,
            paired_data,
            ensemble,
            state_dir,
            output_format,
            compact,
            agent,
            human,
            ..
        } => {
            ensure_state_dir_ready(&state_dir)?;
            ict_engine::application::backtest::factor_backtest_command(
                &symbol,
                &data,
                paired_data.as_deref(),
                ensemble,
                &state_dir,
                match resolve_output_format(&output_format, compact, agent, human)? {
                    OutputFormat::Json => "json",
                    OutputFormat::Compact => "compact",
                    OutputFormat::Agent => "agent",
                    OutputFormat::Human => "human",
                },
                run_factor_backtest,
            )?
        }
        Commands::Env => env_command()?,
        Commands::CleanFutures {
            root,
            output_dir,
            interval,
            multi_timeframe,
        } => ict_engine::application::data_sources::clean_futures_command(
            root.as_deref(),
            &output_dir,
            &interval,
            multi_timeframe,
            run_clean_futures_multi_timeframe,
            run_clean_futures,
        )?,
        Commands::FuturesSop {
            root,
            output_dir,
            interval,
        } => ict_engine::application::data_sources::futures_sop_command(
            root.as_deref(),
            &output_dir,
            &interval,
            run_futures_sop,
        )?,
        Commands::ExpansionSop {
            root,
            output_dir,
            interval,
            lookback,
            atr_multiplier,
            objective,
            mutation_spec,
            emit_mutation_evaluation,
        } => ict_engine::application::data_sources::expansion_sop_command(
            ict_engine::application::data_sources::ExpansionSopCommandInput {
                root: root.as_deref(),
                output_dir: &output_dir,
                interval: &interval,
                lookback,
                atr_multiplier,
                objective: &objective,
                mutation_spec_path: mutation_spec.as_deref(),
                emit_mutation_evaluation,
            },
            parse_research_objective,
            load_factor_mutation_spec,
            run_expansion_sop,
            |report, mutation_spec, emit_mutation_evaluation| {
                if emit_mutation_evaluation {
                    let next_mutation_spec_template = report
                        .factor_mutation_evaluation
                        .as_ref()
                        .map(|evaluation| {
                            next_mutation_spec_template(mutation_spec, evaluation, true)
                        });
                    Ok(serde_json::json!({
                        "mutation_spec": mutation_spec,
                        "factor_mutation_evaluation": report.factor_mutation_evaluation,
                        "next_mutation_spec_template": next_mutation_spec_template,
                        "recommended_global_factor": report.recommended_global_factor,
                        "recommended_global_pre_bayes_summary": report.recommended_global_pre_bayes_summary,
                        "recommended_commands": report.recommended_commands,
                    }))
                } else {
                    let compact_report =
                        build_backtest_result_artifact(BacktestResultArtifactInput {
                            summary: format!("expansion_sop:{}", interval),
                            scorecards: report
                                .recommended_market_factors
                                .iter()
                                .map(|(market, factor)| format!("{}:{}", market, factor))
                                .collect::<Vec<_>>(),
                            shrink_comparison_summary: vec![],
                            duration_sizing_delta_surface: vec![],
                            oos_quality_delta_surface: vec![],
                            market_breakdown: vec![format!(
                                "recommended_global_factor={:?}",
                                report.recommended_global_factor
                            )],
                            regime_breakdown: vec![],
                            window_breakdown: vec![],
                            comparable: true,
                            artifacts: report.recommended_commands.clone(),
                        });
                    let factor_lifecycle = build_factor_lifecycle_view(
                        report.mutation_spec.as_ref(),
                        report.factor_mutation_evaluation.as_ref(),
                        &PromotionDecision {
                            approved: report.recommended_global_factor.is_some(),
                            status: if report.recommended_global_factor.is_some() {
                                "promote".to_string()
                            } else {
                                "hold".to_string()
                            },
                            reason: "expansion_sop_global_selection".to_string(),
                            target_factors: report
                                .recommended_global_factor
                                .iter()
                                .cloned()
                                .collect(),
                            target_families: vec![],
                        },
                        &RollbackRecommendation {
                            should_rollback: false,
                            scope: "none".to_string(),
                            reason: "no_global_rollback".to_string(),
                            target_factors: vec![],
                            target_families: vec![],
                        },
                    );
                    Ok(serde_json::json!({
                        "report": report,
                        "compact_backtest_report": compact_report,
                        "factor_lifecycle": factor_lifecycle,
                    }))
                }
            },
        )?,
        Commands::FactorPipelineDebug {
            symbol,
            data,
            factor,
            objective,
            data_1m,
            data_5m,
            data_15m,
            data_1h,
            data_4h,
            data_1d,
        } => ict_engine::application::factor_pipeline_debug::factor_pipeline_debug_command(
            ict_engine::application::factor_pipeline_debug::FactorPipelineDebugCommandInput {
                symbol: &symbol,
                data: &data,
                factor: &factor,
                objective: &objective,
                data_1m: data_1m.as_deref(),
                data_5m: data_5m.as_deref(),
                data_15m: data_15m.as_deref(),
                data_1h: data_1h.as_deref(),
                data_4h: data_4h.as_deref(),
                data_1d: data_1d.as_deref(),
            },
        )?,
        Commands::WorkflowStatus {
            symbol,
            state_dir,
            refresh,
            phase,
            actionable_only,
            conflicts_only,
            latest_promotable,
            hard_block_only,
            hard_block_reason,
            limit,
            output_format,
            compact,
            agent,
            human,
            stable,
            no_execution_focus: _no_execution_focus,
        } => ict_engine::application::orchestration::workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: &symbol,
                state_dir: &state_dir,
                refresh,
                phase: phase.as_deref(),
                actionable_only,
                conflicts_only,
                latest_promotable,
                hard_block_only,
                hard_block_reason: hard_block_reason.as_deref(),
                limit,
                output_format: match resolve_output_format(&output_format, compact, agent, human)? {
                    OutputFormat::Json => "json",
                    OutputFormat::Compact => "compact",
                    OutputFormat::Agent => "agent",
                    OutputFormat::Human => "human",
                },
                stable,
            },
            refresh_workflow_snapshot,
        )?,
        Commands::PreBayesStatus {
            symbol,
            state_dir,
            refresh,
            section,
        } => ict_engine::application::orchestration::pre_bayes_status_command(
            &symbol,
            &state_dir,
            refresh,
            section.as_deref(),
            refresh_workflow_snapshot,
        )?,
        Commands::PreBayesDiff {
            symbol,
            state_dir,
            refresh,
        } => ict_engine::application::orchestration::pre_bayes_diff_command(
            &symbol,
            &state_dir,
            refresh,
            refresh_workflow_snapshot,
        )?,
        Commands::ArtifactStatus {
            symbol,
            state_dir,
            artifact_id,
            kind,
            latest_only,
            actionable_only,
            rule_break_only,
            sort_by,
            descending,
            limit,
            recent_n,
            consumed_only,
            bucket_by_kind,
            bucket_order_by,
            bucket_limit,
        } => artifact_status_command(ArtifactStatusCommandInput {
            symbol: &symbol,
            state_dir: &state_dir,
            artifact_id: artifact_id.as_deref(),
            kind: kind.as_deref(),
            latest_only,
            actionable_only,
            rule_break_only,
            sort_by: &sort_by,
            descending,
            limit,
            recent_n,
            consumed_only,
            bucket_by_kind,
            bucket_order_by: &bucket_order_by,
            bucket_limit,
        })?,
        Commands::ArtifactDiff {
            symbol,
            state_dir,
            left_artifact_id,
            right_artifact_id,
        } => artifact_diff_command(ArtifactDiffCommandInput {
            symbol: &symbol,
            state_dir: &state_dir,
            left_artifact_id: &left_artifact_id,
            right_artifact_id: &right_artifact_id,
        })?,
        Commands::ArtifactLineage {
            symbol,
            state_dir,
            artifact_id,
            latest_only,
            improving_only,
            regressing_only,
            rule_break_only,
        } => {
            let ledger = load_artifact_ledger(&state_dir, &symbol)?;
            let snapshot = refresh_workflow_snapshot(&state_dir, &symbol)?;
            artifact_lineage_command(ArtifactLineageCommandInput {
                symbol: &symbol,
                ledger: &ledger,
                summaries: snapshot.artifact_lineage_summaries,
                artifact_id: artifact_id.as_deref(),
                latest_only,
                improving_only,
                regressing_only,
                rule_break_only,
            })?
        }
    }

    Ok(())
}

#[cfg(test)]
fn format_executor_summary_lines(executor_summaries: &[String]) -> Vec<String> {
    executor_summaries
        .iter()
        .map(|summary| summary.to_string())
        .collect()
}

fn resolved_vote_scorecards<'a>(
    persisted_scorecards: &'a [EnsembleExecutorScorecard],
    vote: &'a EnsembleVoteRecord,
) -> (&'a [EnsembleExecutorScorecard], &'a str) {
    if persisted_scorecards.is_empty() {
        (
            &vote.executor_scorecards,
            vote.executor_scorecards_source
                .as_deref()
                .unwrap_or("fallback"),
        )
    } else {
        (persisted_scorecards, "persisted")
    }
}

fn emit_analyze_output(
    report: &AnalyzeReport,
    output_format: OutputFormat,
    inline_ledger: bool,
) -> Result<()> {
    let output_format = match output_format {
        OutputFormat::Json => "json",
        OutputFormat::Compact => "compact",
        OutputFormat::Agent => "agent",
        OutputFormat::Human => "human",
    };
    ict_engine::application::reporting::dispatch_analyze_output(
        report,
        ict_engine::application::reporting::AnalyzeOutputDispatchInput {
            output_format,
            inline_ledger,
        },
    )
}

fn resolve_output_format(
    value: &str,
    compact: bool,
    agent: bool,
    human: bool,
) -> Result<OutputFormat> {
    let alias_count = compact as u8 + agent as u8 + human as u8;
    if alias_count > 1 {
        bail!("choose at most one of --compact, --agent, or --human");
    }
    if alias_count == 1 && !value.trim().is_empty() {
        bail!("do not combine --output-format with --compact/--agent/--human");
    }
    if compact {
        return Ok(OutputFormat::Compact);
    }
    if agent {
        return Ok(OutputFormat::Agent);
    }
    if human {
        return Ok(OutputFormat::Human);
    }
    if value.trim().is_empty() {
        return Ok(OutputFormat::Json);
    }
    OutputFormat::parse(value)
}

fn should_warn_about_default_state_dir(state_dir: &str) -> bool {
    if state_dir != DEFAULT_STATE_DIR || env::var_os(STATE_DIR_ENV_VAR).is_some() {
        return false;
    }
    let path = std::path::Path::new(state_dir);
    if path.exists() {
        return false;
    }
    let Ok(cwd) = env::current_dir() else {
        return false;
    };
    !cwd.join("Cargo.toml").exists() && !cwd.join(".ict-engine").exists()
}

fn ensure_state_dir_ready(state_dir: &str) -> Result<()> {
    if should_warn_about_default_state_dir(state_dir) {
        eprintln!(
            "auto-creating state dir at ./state; set --state-dir or {} to customize",
            STATE_DIR_ENV_VAR
        );
    }
    std::fs::create_dir_all(state_dir)
        .with_context(|| format!("creating state directory '{}'", state_dir))?;
    Ok(())
}

fn build_env_report() -> Value {
    let variables = [
        (
            "ICT_ENGINE_STATE_DIR",
            "default state directory for CLI commands",
        ),
        (
            "ICT_ENGINE_STAGED_ORCHESTRATION",
            "enable staged orchestration flow",
        ),
        (
            "ICT_ENGINE_BELIEF_PRIMARY",
            "select the primary belief engine",
        ),
        (
            "ICT_ENGINE_FAMILY_HISTORY_WINDOW",
            "override family history window length",
        ),
        (
            "ICT_ENGINE_TOMAC_ROOT",
            "set the TOMAC root for futures cleaning commands",
        ),
        (
            "ICT_EXECUTION_FOCUS",
            "enable execution-focus reporting surfaces",
        ),
        ("HOME", "OS-provided home directory used for path discovery"),
    ]
    .into_iter()
    .map(|(key, description)| {
        let value = env::var(key).ok();
        serde_json::json!({
            "name": key,
            "description": description,
            "set": value.is_some(),
            "value": value,
        })
    })
    .collect::<Vec<_>>();
    serde_json::json!({
        "state_dir_env_var": STATE_DIR_ENV_VAR,
        "default_state_dir": DEFAULT_STATE_DIR,
        "variables": variables,
    })
}

fn env_command() -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&build_env_report())?);
    Ok(())
}

fn multi_timeframe_phase_hint(summary: &[String]) -> String {
    let direction = summary
        .iter()
        .find_map(|item| item.strip_prefix("higher_timeframe_direction_bias="));
    let alignment = summary
        .iter()
        .find_map(|item| item.strip_prefix("higher_timeframe_alignment_score="));
    let entry = summary
        .iter()
        .find_map(|item| item.strip_prefix("lower_timeframe_entry_alignment_score="));
    let covered = summary
        .iter()
        .find_map(|item| item.strip_prefix("multi_timeframe_source="))
        .unwrap_or("primary_only");
    let mut parts = vec![format!("mtf_source={covered}")];
    if let Some(direction) = direction {
        parts.push(format!("mtf_direction={direction}"));
    }
    if let Some(alignment) = alignment {
        parts.push(format!("mtf_alignment={alignment}"));
    }
    if let Some(entry) = entry {
        parts.push(format!("mtf_entry_alignment={entry}"));
    }
    parts.join(" ")
}

fn run_futures_sop(root: &str, output_dir: &str, interval: &str) -> Result<FuturesSopReport> {
    run_futures_sop_with(
        root,
        output_dir,
        interval,
        |input: FuturesSopMarketInput| {
            let report = run_factor_research(RunFactorResearchInput {
                symbol: &input.market,
                data: &input.output_path,
                objective: ResearchObjectiveMode::Generic,
                data_1m: input.multi_timeframe_inputs.get("1m"),
                data_5m: input.multi_timeframe_inputs.get("5m"),
                data_15m: input.multi_timeframe_inputs.get("15m"),
                data_1h: input.multi_timeframe_inputs.get("1h"),
                data_4h: input.multi_timeframe_inputs.get("4h"),
                data_1d: input.multi_timeframe_inputs.get("1d"),
                paired_data: None,
                mutation_spec: None,
                state_dir: &input.state_dir,
            })?;
            let candles = load_candles(&input.output_path)?;
            let pipeline = report
                .best_factor
                .as_deref()
                .map(|factor| {
                    build_expansion_factor_pipeline_report_v2(
                        &input.market,
                        factor,
                        &candles,
                        &report.multi_timeframe_summary,
                    )
                })
                .transpose()?;
            Ok((report, pipeline))
        },
    )
}

fn run_expansion_sop(
    root: &str,
    output_dir: &str,
    interval: &str,
    lookback: usize,
    atr_multiplier: f64,
    objective_mode: ResearchObjectiveMode,
    mutation_spec: Option<&FactorMutationSpec>,
) -> Result<ExpansionSopReport> {
    let report = run_expansion_sop_with(
        RunExpansionSopInput {
            root,
            output_dir,
            interval,
            lookback,
            atr_multiplier,
            objective_mode,
            mutation_spec,
        },
        |input: ExpansionSopMarketInput, _state_dir, registry| {
            let candles = load_candles(&input.output_path)?;
            let resolved_multi_timeframe_inputs = resolve_multi_timeframe_inputs(
                &input.output_path,
                None,
                None,
                None,
                None,
                None,
                None,
            );
            let multi_timeframe_summary = build_multi_timeframe_summary(
                &input.output_path,
                &resolved_multi_timeframe_inputs,
            )?
            .into_iter()
            .chain(
                build_multi_timeframe_research_signal(&resolved_multi_timeframe_inputs)?
                    .summary
                    .into_iter(),
            )
            .collect::<Vec<_>>();
            let scores =
                expansion_factor_scores_for_market(registry, &candles, lookback, atr_multiplier)?;
            let expansion_samples = scores
                .first()
                .map(|score| score.expansion_samples)
                .unwrap_or(0);
            let bull_expansion_samples = scores
                .first()
                .map(|score| score.bull_expansion_samples)
                .unwrap_or(0);
            let bear_expansion_samples = scores
                .first()
                .map(|score| score.bear_expansion_samples)
                .unwrap_or(0);
            let best_factor = scores.first().map(|score| score.factor_name.clone());
            let pipeline = best_factor
                .as_deref()
                .map(|factor| {
                    build_expansion_factor_pipeline_report_with_registry_v2(
                        &input.market,
                        factor,
                        &candles,
                        None,
                        &multi_timeframe_summary,
                        registry,
                    )
                })
                .transpose()?;
            let (market_report, _) = build_expansion_sop_market_report(
                ict_engine::application::data_sources::BuildExpansionSopMarketReportInput {
                    market: input.market,
                    cleaned_path: input.output_path,
                    total_candles: candles.len(),
                    expansion_samples,
                    bull_expansion_samples,
                    bear_expansion_samples,
                    best_factor,
                    top_factors: scores.into_iter().take(5).collect(),
                    multi_timeframe_summary,
                    pipeline,
                },
            );
            Ok(market_report)
        },
    )?;

    let factor_mutation_evaluation = report.factor_mutation_evaluation.clone();
    if let (Some(spec), Some(evaluation)) = (mutation_spec, factor_mutation_evaluation.clone()) {
        let state_dir = std::path::Path::new(output_dir)
            .join("state")
            .to_string_lossy()
            .to_string();
        append_factor_mutation_run(
            &state_dir,
            "EXPANSION_SOP",
            FactorMutationRunRecord {
                run_id: format!(
                    "factor-mutation:expansion-sop:{}",
                    Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
                ),
                timestamp: Utc::now(),
                symbol: "EXPANSION_SOP".to_string(),
                source_command: "expansion-sop".to_string(),
                data_path: root.to_string(),
                paired_data_path: Some(interval.to_string()),
                mutation_spec: spec.clone(),
                evaluation,
            },
        )?;
    }
    Ok(report)
}

fn refresh_workflow_snapshot(state_dir: &str, symbol: &str) -> Result<WorkflowSnapshot> {
    let analyze_runs: Vec<AnalyzeRunRecord> =
        load_state_or_default(state_dir, symbol, ANALYZE_RUNS_FILE)?;
    let train_runs: Vec<TrainRunRecord> =
        load_state_or_default(state_dir, symbol, TRAIN_RUNS_FILE)?;
    let research_runs: Vec<ResearchRunRecord> =
        load_state_or_default(state_dir, symbol, RESEARCH_RUNS_FILE)?;
    let backtest_runs: Vec<BacktestRunRecord> =
        load_state_or_default(state_dir, symbol, BACKTEST_RUNS_FILE)?;
    let update_runs: Vec<UpdateRunRecord> =
        load_state_or_default(state_dir, symbol, UPDATE_RUNS_FILE)?;
    let pre_bayes_policy_history = load_pre_bayes_policy_history(state_dir, symbol)?;
    let pending_update_history = load_pending_update_history(state_dir, symbol)?;
    let execution_candidate_history = load_execution_candidate_history(state_dir, symbol)?;
    let artifact_ledger = load_artifact_ledger(state_dir, symbol)?;

    let snapshot = build_workflow_snapshot(BuildWorkflowSnapshotInput {
        state_dir,
        symbol,
        latest_train: train_runs.last(),
        latest_analyze: analyze_runs.last(),
        latest_research: research_runs.last(),
        latest_backtest: backtest_runs.last(),
        latest_update: update_runs.last(),
        pre_bayes_policy_history: &pre_bayes_policy_history,
        pending_update_history: &pending_update_history,
        execution_candidate_history: &execution_candidate_history,
        artifact_ledger: &artifact_ledger,
    });
    save_workflow_snapshot(state_dir, symbol, &snapshot)?;
    Ok(snapshot)
}

fn build_workflow_snapshot(input: BuildWorkflowSnapshotInput<'_>) -> WorkflowSnapshot {
    let BuildWorkflowSnapshotInput {
        state_dir,
        symbol,
        latest_train,
        latest_analyze,
        latest_research,
        latest_backtest,
        latest_update,
        pre_bayes_policy_history,
        pending_update_history,
        execution_candidate_history,
        artifact_ledger,
    } = input;

    let train = latest_train.map(workflow_phase_snapshot_from_train_run);
    let analyze = latest_analyze.map(workflow_phase_snapshot_from_analyze_run);
    let research = latest_research.map(workflow_phase_snapshot_from_research_run);
    let backtest = latest_backtest.map(workflow_phase_snapshot_from_backtest_run);
    let update = latest_update.map(workflow_phase_snapshot_from_update_run);
    let field_diffs = workflow_field_diffs(&analyze, &research, &backtest, &update);
    let disagreements = workflow_disagreements(&analyze, &research, &backtest, &update);
    let recent_pending_updates = pending_update_history
        .iter()
        .rev()
        .take(5)
        .map(|artifact| pending_update_summary(state_dir, symbol, artifact))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let recent_execution_candidates = execution_candidate_history
        .iter()
        .rev()
        .take(5)
        .map(|artifact| execution_candidate_summary(state_dir, symbol, artifact))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let recent_ensemble_votes = load_ensemble_vote_history(state_dir, symbol)
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let recent_artifacts = artifact_ledger
        .iter()
        .rev()
        .take(10)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let actionable_artifacts = artifact_ledger
        .iter()
        .filter(|entry| entry.actionable && entry.consumed_by_update_run_id.is_none())
        .cloned()
        .collect::<Vec<_>>();
    let latest_promotable_artifact = artifact_ledger
        .iter()
        .filter(|entry| entry.promote_candidate && entry.consumed_by_update_run_id.is_none())
        .max_by_key(|entry| artifact_generated_recency_key(entry))
        .cloned();
    let artifact_history_summary = build_artifact_history_summary(artifact_ledger);
    let artifact_factor_trends =
        build_artifact_factor_trends(artifact_ledger, &research, &backtest, &update);
    let artifact_family_trends =
        build_artifact_family_trends(artifact_ledger, &research, &backtest, &update);
    let review_rules = artifact_review_rules();
    let review_rule_sources = artifact_review_rule_sources();
    let artifact_lineage_summaries = build_artifact_lineage_summaries_with_embedded_snapshots(
        artifact_ledger,
        pending_update_history,
        execution_candidate_history,
    );
    let artifact_consumed_impact_summary = build_artifact_consumed_impact_summary(artifact_ledger);
    let artifact_decision_summary = artifact_decision_summary_from_trends(
        &actionable_artifacts,
        latest_promotable_artifact.as_ref(),
        &artifact_lineage_summaries,
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_consumed_impact_summary,
    );
    let latest_pre_bayes_policy =
        latest_analyze.map(|run| run.pre_bayes_evidence_filter.policy.clone());
    let latest_pre_bayes_entry_quality_bridge =
        latest_analyze.map(|run| run.pre_bayes_entry_quality_bridge.clone());
    let latest_pre_bayes_entry_quality_bridge_diff = latest_analyze
        .map(|run| pre_bayes_entry_quality_bridge_diff(&run.pre_bayes_entry_quality_bridge));
    let recent_pre_bayes_policies = pre_bayes_policy_history
        .iter()
        .rev()
        .take(5)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let latest_pre_bayes_policy_diff = recent_pre_bayes_policies
        .last()
        .map(|record| record.diff_from_previous.clone());
    let latest_pre_bayes_policy_lineage = Some(pre_bayes_policy_lineage_summary(
        &recent_pre_bayes_policies,
        latest_analyze
            .map(|run| run.pre_bayes_evidence_filter.gating_status.as_str())
            .unwrap_or(""),
    ));
    let latest_pre_bayes_soft_evidence_diff = latest_analyze
        .map(|run| pre_bayes_soft_evidence_diff(&run.pre_bayes_evidence_filter))
        .unwrap_or_default();
    let artifact_rule_break_effects = build_artifact_rule_break_effects(artifact_ledger);
    let artifact_factor_rule_break_impacts =
        build_artifact_factor_rule_break_impacts(artifact_ledger, &artifact_rule_break_effects);
    let artifact_family_rule_break_impacts =
        build_artifact_family_rule_break_impacts(artifact_ledger, &artifact_rule_break_effects);
    let mut phases = [
        train.clone(),
        analyze.clone(),
        research.clone(),
        backtest.clone(),
        update.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    phases.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    let current = phases.last().cloned();
    let blocking_truth = workflow_blocking_truth(
        symbol,
        state_dir,
        current.as_ref(),
        latest_analyze,
        &artifact_decision_summary,
    );

    let mut risk_flags = std::collections::BTreeSet::new();
    for phase in [
        train.as_ref(),
        analyze.as_ref(),
        research.as_ref(),
        backtest.as_ref(),
        update.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        for flag in &phase.risk_flags {
            risk_flags.insert(format!("{}:{}", phase.phase, flag));
        }
    }

    let current_recommended_next_command = current
        .as_ref()
        .map(|phase| phase.recommended_next_command.clone())
        .unwrap_or_default();
    let current_recommended_next_command_meta = current
        .as_ref()
        .map(|phase| {
            if phase.recommended_next_command_meta.kind
                == ict_engine::state::RecommendedNextCommandKind::Unknown
                && !phase.recommended_next_command.is_empty()
            {
                recommended_next_command_meta(&phase.recommended_next_command)
            } else {
                phase.recommended_next_command_meta.clone()
            }
        })
        .unwrap_or_else(|| recommended_next_command_meta(&current_recommended_next_command));

    WorkflowSnapshot {
        symbol: symbol.to_string(),
        generated_at: Utc::now(),
        current_focus_phase: current
            .as_ref()
            .map(|phase| phase.phase.clone())
            .unwrap_or_default(),
        current_focus_reason: current
            .as_ref()
            .map(|phase| phase.workflow_reason.clone())
            .unwrap_or_default(),
        blocking_truth,
        recommended_next_command: current_recommended_next_command,
        recommended_next_command_meta: current_recommended_next_command_meta,
        pending_actions: current.map(|phase| phase.top_actions).unwrap_or_default(),
        risk_flags: risk_flags
            .into_iter()
            .chain(
                disagreements
                    .iter()
                    .map(|item| format!("{}:{}", item.severity, item.id)),
            )
            .collect(),
        latest_train: train,
        latest_analyze: analyze,
        latest_research: research,
        latest_backtest: backtest,
        latest_update: update,
        latest_pre_bayes_policy,
        latest_pre_bayes_entry_quality_bridge,
        latest_pre_bayes_entry_quality_bridge_diff,
        latest_pre_bayes_policy_diff,
        latest_pre_bayes_policy_lineage,
        latest_pre_bayes_soft_evidence_diff,
        recent_pre_bayes_policies,
        latest_pending_update: recent_pending_updates.last().cloned(),
        recent_pending_updates,
        latest_execution_candidate: recent_execution_candidates.last().cloned(),
        recent_execution_candidates,
        latest_ensemble_vote: recent_ensemble_votes.last().cloned(),
        recent_ensemble_votes,
        recent_artifacts,
        actionable_artifacts,
        latest_promotable_artifact,
        artifact_history_summary,
        artifact_factor_trends,
        artifact_family_trends,
        artifact_decision_summary,
        artifact_review_rules: review_rules,
        artifact_review_rule_sources: review_rule_sources,
        artifact_lineage_summaries,
        artifact_rule_break_effects,
        artifact_factor_rule_break_impacts,
        artifact_family_rule_break_impacts,
        artifact_consumed_impact_summary,
        field_diffs,
        disagreements,
    }
}

fn gate_aware_recommended_next_command(stored: &str, commands: &CommandRecommendations) -> String {
    for command in [&commands.research, &commands.backtest] {
        if command.user_data_selection_required {
            return render_recommended_command(command);
        }
    }
    stored.to_string()
}

fn workflow_phase_snapshot_from_analyze_run(run: &AnalyzeRunRecord) -> WorkflowPhaseSnapshot {
    let bridge_diff = pre_bayes_entry_quality_bridge_diff(&run.pre_bayes_entry_quality_bridge);
    let duration_fragment = if let (Some(model), Some(remaining)) = (
        run.hybrid_duration_model.as_deref(),
        run.hybrid_remaining_expected_bars,
    ) {
        format!(
            " hybrid_duration_model={} hybrid_remaining_expected_bars={:.3}",
            model, remaining
        )
    } else {
        String::new()
    };
    let mut phase = WorkflowPhaseSnapshot {
        phase: "analyze".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: run.promotion_decision.status.clone(),
        rollback_scope: run.rollback_recommendation.scope.clone(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "selected_direction={:?} selected_entry_quality={} pre_bayes_status={} pre_bayes_quality={:.3} decision_hint={}{} {}",
            run.selected_direction,
            run.selected_entry_quality,
            run.pre_bayes_evidence_filter.gating_status,
            run.pre_bayes_evidence_filter.evidence_quality_score,
            run.decision_hint,
            duration_fragment,
            multi_timeframe_phase_hint(&run.multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: workflow_phase_risk_flags(
            &run.dataset_comparability,
            &run.promotion_decision,
            &run.rollback_recommendation,
        )
        .into_iter()
        .chain(
            run.pre_bayes_evidence_filter
                .conflict_flags
                .iter()
                .map(|flag| format!("pre_bayes:{}", flag)),
        )
        .collect(),
        selected_direction: Some(format!("{:?}", run.selected_direction)),
        selected_entry_quality: Some(run.selected_entry_quality.clone()),
        pre_bayes_gate_status: run.pre_bayes_evidence_filter.gating_status.clone(),
        pre_bayes_uses_soft_evidence: run.pre_bayes_evidence_filter.uses_soft_evidence,
        pre_bayes_policy_version: run.pre_bayes_evidence_filter.policy.version.clone(),
        pre_bayes_evidence_quality_score: run.pre_bayes_evidence_filter.evidence_quality_score,
        pre_bayes_conflict_flags: run.pre_bayes_evidence_filter.conflict_flags.clone(),
        pre_bayes_filtered_assignments: {
            let mut assignments = run.pre_bayes_evidence_filter.evidence_assignments.clone();
            assignments.insert(
                "__policy_version".to_string(),
                run.pre_bayes_evidence_filter.policy.version.clone(),
            );
            assignments
        },
        pre_bayes_soft_evidence: BTreeMap::from([
            (
                "market_regime".to_string(),
                run.pre_bayes_evidence_filter
                    .soft_market_regime_distribution
                    .clone(),
            ),
            (
                "liquidity_context".to_string(),
                run.pre_bayes_evidence_filter
                    .soft_liquidity_context_distribution
                    .clone(),
            ),
            (
                "factor_alignment".to_string(),
                run.pre_bayes_evidence_filter
                    .soft_factor_alignment_distribution
                    .clone(),
            ),
            (
                "factor_uncertainty".to_string(),
                run.pre_bayes_evidence_filter
                    .soft_factor_uncertainty_distribution
                    .clone(),
            ),
            (
                "multi_timeframe_resonance".to_string(),
                run.pre_bayes_evidence_filter
                    .soft_multi_timeframe_resonance_distribution
                    .clone(),
            ),
        ]),
        pre_bayes_long_signal_probability: Some(
            run.pre_bayes_entry_quality_bridge.long_signal_probability,
        ),
        pre_bayes_short_signal_probability: Some(
            run.pre_bayes_entry_quality_bridge.short_signal_probability,
        ),
        pre_bayes_selected_entry_quality_probability: run
            .pre_bayes_entry_quality_bridge
            .selected_entry_quality
            .values()
            .copied()
            .fold(None, |acc, value| {
                Some(acc.map(|current| current.max(value)).unwrap_or(value))
            }),
        pre_bayes_bridge_selected_entry_quality: bridge_diff.selected_entry_quality.clone(),
        pre_bayes_bridge_probability_gap: Some(bridge_diff.long_short_signal_probability_gap),
        pre_bayes_bridge_rationale_summary: bridge_diff.rationale_summary,
        pre_bayes_multi_timeframe_direction_bias: run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_direction_bias
            .clone(),
        pre_bayes_multi_timeframe_alignment_score: run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_alignment_score,
        pre_bayes_multi_timeframe_entry_alignment_score: run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_entry_alignment_score,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
        hybrid_duration_model: run.hybrid_duration_model.clone(),
        hybrid_remaining_expected_bars: run.hybrid_remaining_expected_bars,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: None,
        family_states: run
            .factor_family_outcomes
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.family, item.promotion_decision.status, item.rollback_recommendation.scope
                )
            })
            .collect(),
        factor_actions: run.agent_context_bundle.top_factor_actions.clone(),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        family_score_map: run
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        factor_score_map: BTreeMap::new(),
        objective_market_credibility_shrink: None,
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
    };
    apply_analyze_run_execution_fields(&mut phase, run);
    phase.phase_summary = format!(
        "{}{}",
        phase.phase_summary,
        execution_phase_summary_suffix(&phase)
    );
    phase
}

fn workflow_phase_snapshot_from_train_run(run: &TrainRunRecord) -> WorkflowPhaseSnapshot {
    WorkflowPhaseSnapshot {
        phase: "train".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: "promotion_status_unavailable".to_string(),
        rollback_scope: "rollback_scope_unavailable".to_string(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "final_state={} observations={} epochs={} log_likelihood={:.4} {}",
            run.final_state,
            run.observations,
            run.epochs,
            run.log_likelihood,
            multi_timeframe_phase_hint(&run.multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: if run.dataset_comparability.comparable {
            Vec::new()
        } else {
            vec![format!(
                "not_comparable:{}",
                run.dataset_comparability.comparison_class
            )]
        },
        selected_direction: None,
        selected_entry_quality: None,
        pre_bayes_gate_status: "pre_bayes_gate_unavailable".to_string(),
        pre_bayes_uses_soft_evidence: false,
        pre_bayes_policy_version: "policy_version_unavailable".to_string(),
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
        pre_bayes_multi_timeframe_direction_bias: "direction_bias_unavailable".to_string(),
        pre_bayes_multi_timeframe_alignment_score: None,
        pre_bayes_multi_timeframe_entry_alignment_score: None,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
        hybrid_duration_model: None,
        hybrid_remaining_expected_bars: None,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: None,
        family_states: Vec::new(),
        factor_actions: Vec::new(),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        family_score_map: BTreeMap::new(),
        factor_score_map: BTreeMap::new(),
        objective_market_credibility_shrink: None,
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
    }
}

fn workflow_phase_snapshot_from_research_run(run: &ResearchRunRecord) -> WorkflowPhaseSnapshot {
    let mut phase = WorkflowPhaseSnapshot {
        phase: "research".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: run.promotion_decision.status.clone(),
        rollback_scope: run.rollback_recommendation.scope.clone(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "objective={} best_factor={:?} aggregate_return={:.4} feedback_applied={} credibility={} {}",
            if run.research_objective.is_empty() {
                "generic"
            } else {
                run.research_objective.as_str()
            },
            run.best_factor,
            run.aggregate_return,
            run.feedback_records_applied,
            run.artifact_action_summary
                .iter()
                .find(|item| item.starts_with("conformal_credibility:"))
                .cloned()
                .unwrap_or_else(|| "conformal_credibility:unavailable".to_string()),
            multi_timeframe_phase_hint(&run.multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: workflow_phase_risk_flags(
            &run.dataset_comparability,
            &run.promotion_decision,
            &run.rollback_recommendation,
        ),
        selected_direction: None,
        selected_entry_quality: None,
        pre_bayes_gate_status: "pre_bayes_gate_unavailable".to_string(),
        pre_bayes_uses_soft_evidence: false,
        pre_bayes_policy_version: "policy_version_unavailable".to_string(),
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
        pre_bayes_multi_timeframe_direction_bias: "direction_bias_unavailable".to_string(),
        pre_bayes_multi_timeframe_alignment_score: None,
        pre_bayes_multi_timeframe_entry_alignment_score: None,
        hybrid_duration_model: None,
        hybrid_remaining_expected_bars: None,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: None,
        family_states: run
            .factor_family_outcomes
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.family, item.promotion_decision.status, item.rollback_recommendation.scope
                )
            })
            .collect(),
        factor_actions: run.agent_context_bundle.top_factor_actions.clone(),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        family_score_map: run
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        factor_score_map: run
            .factor_score_deltas
            .iter()
            .map(|item| (item.factor_name.clone(), item.new_score))
            .collect(),
        objective_market_credibility_shrink: None,
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
    };
    ict_engine::application::execution::apply_research_run_execution_fields(&mut phase, run);
    phase.phase_summary = format!(
        "{}{}",
        phase.phase_summary,
        execution_phase_summary_suffix(&phase)
    );
    phase
}

fn workflow_phase_snapshot_from_backtest_run(run: &BacktestRunRecord) -> WorkflowPhaseSnapshot {
    let objective_market_shrink_summary = run
        .objective_market_credibility_shrink
        .as_ref()
        .map(|item| {
            format!(
                " objective_market_shrink={:.3} objective_market_credibility={:.3} objective_market_shrink_triggered={}",
                item.shrink_weight, item.credibility_score, item.shrink_triggered
            )
        })
        .unwrap_or_default();
    let mut phase = WorkflowPhaseSnapshot {
        phase: "backtest".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: run.promotion_decision.status.clone(),
        rollback_scope: run.rollback_recommendation.scope.clone(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "total_return={:.4} trade_count={} source={} coverage_1sigma={:.3} break_penalty={:.3} structural_break_detected={} structural_break_score={:.3} structural_break_index={:?}{} {}",
            run.total_return,
            run.trade_count,
            run.source_command,
            run.conformal_coverage_1sigma,
            run.regime_break_penalty,
            run.structural_break_detected,
            run.structural_break_score,
            run.structural_break_index,
            objective_market_shrink_summary,
            multi_timeframe_phase_hint(&run.multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: workflow_phase_risk_flags(
            &run.dataset_comparability,
            &run.promotion_decision,
            &run.rollback_recommendation,
        ),
        selected_direction: None,
        selected_entry_quality: None,
        pre_bayes_gate_status: "pre_bayes_gate_unavailable".to_string(),
        pre_bayes_uses_soft_evidence: false,
        pre_bayes_policy_version: "policy_version_unavailable".to_string(),
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
        pre_bayes_multi_timeframe_direction_bias: "direction_bias_unavailable".to_string(),
        pre_bayes_multi_timeframe_alignment_score: None,
        pre_bayes_multi_timeframe_entry_alignment_score: None,
        hybrid_duration_model: None,
        hybrid_remaining_expected_bars: None,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: None,
        family_states: run
            .factor_family_outcomes
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.family, item.promotion_decision.status, item.rollback_recommendation.scope
                )
            })
            .collect(),
        factor_actions: run.agent_context_bundle.top_factor_actions.clone(),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        family_score_map: run
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        factor_score_map: run
            .factor_score_deltas
            .iter()
            .map(|item| (item.factor_name.clone(), item.new_score))
            .collect(),
        objective_market_credibility_shrink: run.objective_market_credibility_shrink.clone(),
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
    };
    ict_engine::application::execution::apply_backtest_run_execution_fields(&mut phase, run);
    phase.phase_summary = format!(
        "{}{}",
        phase.phase_summary,
        execution_phase_summary_suffix(&phase)
    );
    phase
}

fn workflow_phase_snapshot_from_update_run(run: &UpdateRunRecord) -> WorkflowPhaseSnapshot {
    let consumed_bridge_diff = run
        .consumed_pre_bayes_entry_quality_bridge
        .as_ref()
        .map(pre_bayes_entry_quality_bridge_diff);
    let mut phase = WorkflowPhaseSnapshot {
        phase: "update".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: run.promotion_decision.status.clone(),
        rollback_scope: run.rollback_recommendation.scope.clone(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "realized_outcome={} feedback_applied={} duplicate_feedback_skipped={} consumed_pre_bayes_gate_status={} {}",
            run.realized_outcome,
            run.feedback_records_applied,
            run.duplicate_feedback_skipped,
            run.consumed_pre_bayes_evidence_filter
                .as_ref()
                .map(|filter| filter.gating_status.clone())
                .unwrap_or_default(),
            multi_timeframe_phase_hint(&run.consumed_multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: workflow_phase_risk_flags(
            &run.dataset_comparability,
            &run.promotion_decision,
            &run.rollback_recommendation,
        ),
        selected_direction: None,
        selected_entry_quality: Some(run.normalized_entry_quality.clone()),
        pre_bayes_gate_status: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.gating_status.clone())
            .unwrap_or_default(),
        pre_bayes_uses_soft_evidence: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.uses_soft_evidence)
            .unwrap_or(false),
        pre_bayes_policy_version: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.policy.version.clone())
            .unwrap_or_default(),
        pre_bayes_evidence_quality_score: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.evidence_quality_score)
            .unwrap_or_default(),
        pre_bayes_conflict_flags: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.conflict_flags.clone())
            .unwrap_or_default(),
        pre_bayes_filtered_assignments: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.evidence_assignments.clone())
            .unwrap_or_default(),
        pre_bayes_soft_evidence: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| {
                BTreeMap::from([
                    (
                        "market_regime".to_string(),
                        filter.soft_market_regime_distribution.clone(),
                    ),
                    (
                        "liquidity_context".to_string(),
                        filter.soft_liquidity_context_distribution.clone(),
                    ),
                    (
                        "factor_alignment".to_string(),
                        filter.soft_factor_alignment_distribution.clone(),
                    ),
                    (
                        "factor_uncertainty".to_string(),
                        filter.soft_factor_uncertainty_distribution.clone(),
                    ),
                    (
                        "multi_timeframe_resonance".to_string(),
                        filter.soft_multi_timeframe_resonance_distribution.clone(),
                    ),
                ])
            })
            .unwrap_or_default(),
        pre_bayes_long_signal_probability: None,
        pre_bayes_short_signal_probability: None,
        pre_bayes_selected_entry_quality_probability: None,
        pre_bayes_bridge_selected_entry_quality: consumed_bridge_diff
            .as_ref()
            .and_then(|diff| diff.selected_entry_quality.clone()),
        pre_bayes_bridge_probability_gap: consumed_bridge_diff
            .as_ref()
            .map(|diff| diff.long_short_signal_probability_gap),
        pre_bayes_bridge_rationale_summary: consumed_bridge_diff
            .as_ref()
            .map(|diff| diff.rationale_summary.clone())
            .unwrap_or_default(),
        pre_bayes_multi_timeframe_direction_bias: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.filtered_multi_timeframe_direction_bias.clone())
            .unwrap_or_default(),
        pre_bayes_multi_timeframe_alignment_score: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .and_then(|filter| filter.filtered_multi_timeframe_alignment_score),
        pre_bayes_multi_timeframe_entry_alignment_score: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .and_then(|filter| filter.filtered_multi_timeframe_entry_alignment_score),
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
        hybrid_duration_model: None,
        hybrid_remaining_expected_bars: None,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: Some(run.realized_outcome.clone()),
        family_states: run
            .factor_family_outcomes
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.family, item.promotion_decision.status, item.rollback_recommendation.scope
                )
            })
            .collect(),
        factor_actions: run.agent_context_bundle.top_factor_actions.clone(),
        multi_timeframe_summary: run.consumed_multi_timeframe_summary.clone(),
        family_score_map: run
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        factor_score_map: run
            .factor_score_deltas
            .iter()
            .map(|item| (item.factor_name.clone(), item.new_score))
            .collect(),
        objective_market_credibility_shrink: None,
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
    };
    ict_engine::application::execution::apply_update_run_execution_fields(&mut phase, run);
    phase.phase_summary = format!(
        "{}{}",
        phase.phase_summary,
        execution_phase_summary_suffix(&phase)
    );
    phase
}

fn workflow_top_actions(plan: &AgentActionPlan) -> Vec<String> {
    plan.items
        .iter()
        .take(3)
        .map(|item| format!("{}:{}", item.stage, item.title))
        .collect()
}

fn workflow_blocking_truth(
    symbol: &str,
    state_dir: &str,
    current_phase: Option<&WorkflowPhaseSnapshot>,
    pre_bayes_filter: Option<&AnalyzeRunRecord>,
    artifact_decision_summary: &ict_engine::state::ArtifactDecisionSummary,
) -> WorkflowBlockingTruth {
    let current_recommended_command = current_phase
        .map(|phase| phase.recommended_next_command.clone())
        .unwrap_or_default();
    if current_recommended_command.contains("user_selected_historical_data") {
        return WorkflowBlockingTruth {
            stage: current_phase
                .map(|phase| phase.phase.clone())
                .unwrap_or_else(|| "data_selection".to_string()),
            status: "blocked".to_string(),
            reason: "user_selected_historical_data_missing".to_string(),
            evidence: vec![
                "historical data reuse requires explicit user path selection".to_string(),
                current_recommended_command.clone(),
            ],
            next_command: current_recommended_command,
        };
    }
    if let Some(analyze) = pre_bayes_filter {
        let gate_status = analyze.pre_bayes_evidence_filter.gating_status.clone();
        let bridge_diff =
            pre_bayes_entry_quality_bridge_diff(&analyze.pre_bayes_entry_quality_bridge);
        let bridge_gap = bridge_diff.long_short_signal_probability_gap;
        let hard_pass = pre_bayes_gate_is_hard_pass(&gate_status);
        let bridge_gap_clear_threshold = env_f64("ICT_ENGINE_BRIDGE_GAP_CLEAR_THRESHOLD", 0.12);
        if !hard_pass || bridge_gap < bridge_gap_clear_threshold {
            let mut evidence = vec![
                format!("pre_bayes_gate_status={gate_status}"),
                format!("bridge_probability_gap={bridge_gap:.3}"),
                format!(
                    "selected_entry_quality={}",
                    bridge_diff
                        .selected_entry_quality
                        .unwrap_or_else(|| "entry_quality_unavailable".to_string())
                ),
            ];
            evidence.extend(
                analyze
                    .pre_bayes_evidence_filter
                    .rationale
                    .iter()
                    .take(3)
                    .cloned(),
            );
            return WorkflowBlockingTruth {
                stage: "analyze".to_string(),
                status: if hard_pass {
                    "bridge_needs_confirmation".to_string()
                } else {
                    gate_status.clone()
                },
                reason: if hard_pass {
                    format!(
                        "pre_bayes passed but bridge gap {:.3} is below confirmation threshold",
                        bridge_gap
                    )
                } else {
                    analyze
                        .pre_bayes_evidence_filter
                        .rationale
                        .first()
                        .cloned()
                        .unwrap_or_else(|| {
                            "pre-bayes gate still blocks downstream chain".to_string()
                        })
                },
                evidence,
                next_command: if current_recommended_command.is_empty() {
                    format!(
                        "ict-engine pre-bayes-status --symbol {} --state-dir {}",
                        shell_quote(symbol),
                        shell_quote(state_dir)
                    )
                } else {
                    current_recommended_command
                },
            };
        }
    }
    if artifact_decision_summary.consumed_trend_status == "validated_regressing" {
        return WorkflowBlockingTruth {
            stage: "artifact_consumption".to_string(),
            status: artifact_decision_summary.consumed_trend_status.clone(),
            reason: artifact_decision_summary.consumed_trend_reason.clone(),
            evidence: artifact_decision_summary.consumed_target_kinds.clone(),
            next_command: format!(
                "ict-engine workflow-status --symbol {} --state-dir {} --phase artifact-consumed-gate",
                shell_quote(symbol),
                shell_quote(state_dir)
            ),
        };
    }
    if let Some(phase) = current_phase {
        if let Some(credibility_block) = phase.risk_flags.iter().find(|flag| {
            flag.contains("conformal_coverage_low")
                || flag.contains("regime_break_penalty_high")
                || flag.contains("structural_break_detected")
        }) {
            return WorkflowBlockingTruth {
                stage: phase.phase.clone(),
                status: "credibility_gate_blocked".to_string(),
                reason: format!(
                    "workflow credibility gate blocked next step because {}",
                    credibility_block
                ),
                evidence: phase.risk_flags.clone(),
                next_command: format!(
                    "ict-engine workflow-status --symbol {} --state-dir {} --phase human-next",
                    shell_quote(symbol),
                    shell_quote(state_dir)
                ),
            };
        }
    }
    if let Some(phase) = current_phase {
        return WorkflowBlockingTruth {
            stage: phase.phase.clone(),
            status: "follow_current_focus".to_string(),
            reason: phase.workflow_reason.clone(),
            evidence: phase.top_actions.clone(),
            next_command: phase.recommended_next_command.clone(),
        };
    }
    WorkflowBlockingTruth {
        stage: "stage_unavailable".to_string(),
        status: "insufficient_state".to_string(),
        reason: "no workflow phase snapshots available".to_string(),
        evidence: Vec::new(),
        next_command: "next_command_unavailable".to_string(),
    }
}

fn workflow_phase_risk_flags(
    comparability: &DatasetComparability,
    promotion: &PromotionDecision,
    rollback: &RollbackRecommendation,
) -> Vec<String> {
    let mut flags = Vec::new();
    if !comparability.comparable {
        flags.push(format!("not_comparable:{}", comparability.comparison_class));
    }
    if rollback.should_rollback {
        flags.push(format!("rollback:{}", rollback.reason));
    }
    if !promotion.approved && !promotion.status.is_empty() && promotion.status != "observe" {
        flags.push(format!("promotion_blocked:{}", promotion.reason));
    }
    flags
}

fn workflow_field_diffs(
    analyze: &Option<WorkflowPhaseSnapshot>,
    research: &Option<WorkflowPhaseSnapshot>,
    backtest: &Option<WorkflowPhaseSnapshot>,
    update: &Option<WorkflowPhaseSnapshot>,
) -> Vec<WorkflowFieldDiff> {
    let mut diffs = Vec::new();
    for (left, right) in [
        (research.as_ref(), backtest.as_ref()),
        (analyze.as_ref(), update.as_ref()),
        (research.as_ref(), update.as_ref()),
    ] {
        if let (Some(left), Some(right)) = (left, right) {
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "promotion_status",
                &left.promotion_status,
                &right.promotion_status,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "rollback_scope",
                &left.rollback_scope,
                &right.rollback_scope,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "workflow_phase",
                &left.workflow_phase,
                &right.workflow_phase,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "comparison_class",
                &left.comparison_class,
                &right.comparison_class,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_gate_status",
                &left.pre_bayes_gate_status,
                &right.pre_bayes_gate_status,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_policy_version",
                &left.pre_bayes_policy_version,
                &right.pre_bayes_policy_version,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_uses_soft_evidence",
                if left.pre_bayes_uses_soft_evidence {
                    "true"
                } else {
                    "false"
                },
                if right.pre_bayes_uses_soft_evidence {
                    "true"
                } else {
                    "false"
                },
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_soft_market_regime",
                &format!("{:?}", left.pre_bayes_soft_evidence.get("market_regime")),
                &format!("{:?}", right.pre_bayes_soft_evidence.get("market_regime")),
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_bridge_selected_entry_quality",
                &left
                    .pre_bayes_bridge_selected_entry_quality
                    .clone()
                    .unwrap_or_default(),
                &right
                    .pre_bayes_bridge_selected_entry_quality
                    .clone()
                    .unwrap_or_default(),
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_bridge_probability_gap",
                &left
                    .pre_bayes_bridge_probability_gap
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
                &right
                    .pre_bayes_bridge_probability_gap
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_multi_timeframe_direction_bias",
                &left.pre_bayes_multi_timeframe_direction_bias,
                &right.pre_bayes_multi_timeframe_direction_bias,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_multi_timeframe_alignment_score",
                &left
                    .pre_bayes_multi_timeframe_alignment_score
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
                &right
                    .pre_bayes_multi_timeframe_alignment_score
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_multi_timeframe_entry_alignment_score",
                &left
                    .pre_bayes_multi_timeframe_entry_alignment_score
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
                &right
                    .pre_bayes_multi_timeframe_entry_alignment_score
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
            );
        }
    }
    diffs
}

fn push_workflow_field_diff(
    diffs: &mut Vec<WorkflowFieldDiff>,
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
    field: &str,
    left_value: &str,
    right_value: &str,
) {
    if left_value != right_value {
        diffs.push(WorkflowFieldDiff {
            left_phase: left.phase.clone(),
            right_phase: right.phase.clone(),
            field: field.to_string(),
            left_value: left_value.to_string(),
            right_value: right_value.to_string(),
            severity: if field == "promotion_status" || field == "rollback_scope" {
                "high".to_string()
            } else {
                "medium".to_string()
            },
        });
    }
}

fn workflow_disagreements(
    analyze: &Option<WorkflowPhaseSnapshot>,
    research: &Option<WorkflowPhaseSnapshot>,
    backtest: &Option<WorkflowPhaseSnapshot>,
    update: &Option<WorkflowPhaseSnapshot>,
) -> Vec<WorkflowDisagreement> {
    let mut disagreements = Vec::new();

    if let (Some(analyze), Some(update)) = (analyze, update) {
        if analyze
            .selected_direction
            .as_deref()
            .map(|direction| direction == "Bull" || direction == "Bear")
            .unwrap_or(false)
            && update.rollback_scope != "none"
        {
            disagreements.push(WorkflowDisagreement {
                id: "analyze_direction_vs_update_rollback".to_string(),
                severity: "high".to_string(),
                summary: "analyze directional bias conflicts with the latest update rollback state"
                    .to_string(),
                phases: vec![analyze.phase.clone(), update.phase.clone()],
                recommended_action: "review realized feedback against the current directional evidence before trusting deployment decisions".to_string(),
                evidence: vec![
                    format!(
                        "analyze.selected_direction={}",
                        analyze.selected_direction.clone().unwrap_or_default()
                    ),
                    format!("update.rollback_scope={}", update.rollback_scope),
                    format!(
                        "update.realized_outcome={}",
                        update.realized_outcome.clone().unwrap_or_default()
                    ),
                ],
                sources: Vec::new(),
            });
        }
    }

    if let (Some(research), Some(backtest)) = (research, backtest) {
        if research.promotion_status != backtest.promotion_status {
            disagreements.push(WorkflowDisagreement {
                id: "research_vs_backtest_promotion_status".to_string(),
                severity: "high".to_string(),
                summary: "research and backtest disagree on promotion status".to_string(),
                phases: vec![research.phase.clone(), backtest.phase.clone()],
                recommended_action:
                    "compare score deltas with backtest returns before promoting factor changes"
                        .to_string(),
                evidence: vec![
                    format!("research.promotion_status={}", research.promotion_status),
                    format!("backtest.promotion_status={}", backtest.promotion_status),
                ],
                sources: family_conflict_sources(research, backtest)
                    .into_iter()
                    .chain(factor_conflict_sources(research, backtest))
                    .collect(),
            });
        }
    }

    if let Some(analyze) = analyze {
        for downstream in [research.as_ref(), backtest.as_ref(), update.as_ref()]
            .into_iter()
            .flatten()
        {
            if analyze.pre_bayes_gate_status == "observe_only"
                && downstream.promotion_status == "promote"
            {
                let soft_divergences = pre_bayes_soft_divergence_evidence(analyze);
                disagreements.push(WorkflowDisagreement {
                    id: format!("analyze_pre_bayes_observe_only_vs_{}_promote", downstream.phase),
                    severity: "high".to_string(),
                    summary:
                        "analyze pre-bayes gate is observe-only but a downstream phase still promotes"
                            .to_string(),
                    phases: vec![analyze.phase.clone(), downstream.phase.clone()],
                    recommended_action:
                        "resolve pre-bayes evidence quality before trusting downstream promotion"
                            .to_string(),
                    evidence: vec![
                        format!(
                            "analyze.pre_bayes_gate_status={}",
                            analyze.pre_bayes_gate_status
                        ),
                        format!(
                            "analyze.pre_bayes_quality={:.3}",
                            analyze.pre_bayes_evidence_quality_score
                        ),
                        format!(
                            "analyze.pre_bayes_policy_version={}",
                            analyze.pre_bayes_policy_version
                        ),
                        format!(
                            "analyze.pre_bayes_uses_soft_evidence={}",
                            analyze.pre_bayes_uses_soft_evidence
                        ),
                        format!(
                            "analyze.pre_bayes_long_signal_probability={:.3}",
                            analyze.pre_bayes_long_signal_probability.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_short_signal_probability={:.3}",
                            analyze.pre_bayes_short_signal_probability.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_selected_entry_quality_probability={:.3}",
                            analyze
                                .pre_bayes_selected_entry_quality_probability
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_bridge_selected_entry_quality={}",
                            analyze
                                .pre_bayes_bridge_selected_entry_quality
                                .clone()
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_bridge_probability_gap={:.3}",
                            analyze.pre_bayes_bridge_probability_gap.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_direction_bias={}",
                            analyze.pre_bayes_multi_timeframe_direction_bias
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_alignment_score={:.3}",
                            analyze
                                .pre_bayes_multi_timeframe_alignment_score
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_entry_alignment_score={:.3}",
                            analyze
                                .pre_bayes_multi_timeframe_entry_alignment_score
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_soft_divergences={}",
                            if soft_divergences.is_empty() {
                                "none".to_string()
                            } else {
                                soft_divergences.join("|")
                            }
                        ),
                        format!(
                            "{}.promotion_status={}",
                            downstream.phase, downstream.promotion_status
                        ),
                    ],
                    sources: vec![WorkflowConflictSource {
                        scope: "pre_bayes_bridge".to_string(),
                        subject: "policy_version_and_selected_entry_quality".to_string(),
                        left_phase: analyze.phase.clone(),
                        left_value: format!(
                            "{}:{}",
                            analyze.pre_bayes_policy_version,
                            analyze
                                .pre_bayes_bridge_selected_entry_quality
                                .clone()
                                .unwrap_or_default()
                        ),
                        right_phase: downstream.phase.clone(),
                        right_value: downstream.promotion_status.clone(),
                        evidence: vec![
                            "observe_only gate conflicts with downstream promote".to_string(),
                            format!(
                                "uses_soft_evidence={}",
                                analyze.pre_bayes_uses_soft_evidence
                            ),
                            format!(
                                "long_short_signal_probability_gap={:.3}",
                                analyze.pre_bayes_bridge_probability_gap.unwrap_or_default()
                            ),
                            format!(
                                "multi_timeframe_direction_bias={}",
                                analyze.pre_bayes_multi_timeframe_direction_bias
                            ),
                            format!(
                                "soft_divergences={}",
                                if soft_divergences.is_empty() {
                                    "none".to_string()
                                } else {
                                    soft_divergences.join("|")
                                }
                            ),
                        ],
                    }],
                });
            }
            if analyze.pre_bayes_gate_status == "pass_neutralized"
                && downstream.promotion_status == "promote"
            {
                let soft_divergences = pre_bayes_soft_divergence_evidence(analyze);
                disagreements.push(WorkflowDisagreement {
                    id: format!(
                        "analyze_pre_bayes_neutralized_vs_{}_promote",
                        downstream.phase
                    ),
                    severity: "medium".to_string(),
                    summary:
                        "analyze pre-bayes gate is neutralized while a downstream phase still promotes"
                            .to_string(),
                    phases: vec![analyze.phase.clone(), downstream.phase.clone()],
                    recommended_action:
                        "review whether neutralized evidence is strong enough to justify promotion"
                            .to_string(),
                    evidence: vec![
                        format!(
                            "analyze.pre_bayes_gate_status={}",
                            analyze.pre_bayes_gate_status
                        ),
                        format!(
                            "analyze.pre_bayes_quality={:.3}",
                            analyze.pre_bayes_evidence_quality_score
                        ),
                        format!(
                            "analyze.pre_bayes_policy_version={}",
                            analyze.pre_bayes_policy_version
                        ),
                        format!(
                            "analyze.pre_bayes_uses_soft_evidence={}",
                            analyze.pre_bayes_uses_soft_evidence
                        ),
                        format!(
                            "analyze.pre_bayes_long_signal_probability={:.3}",
                            analyze.pre_bayes_long_signal_probability.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_short_signal_probability={:.3}",
                            analyze.pre_bayes_short_signal_probability.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_selected_entry_quality_probability={:.3}",
                            analyze
                                .pre_bayes_selected_entry_quality_probability
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_bridge_selected_entry_quality={}",
                            analyze
                                .pre_bayes_bridge_selected_entry_quality
                                .clone()
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_bridge_probability_gap={:.3}",
                            analyze.pre_bayes_bridge_probability_gap.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_direction_bias={}",
                            analyze.pre_bayes_multi_timeframe_direction_bias
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_alignment_score={:.3}",
                            analyze
                                .pre_bayes_multi_timeframe_alignment_score
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_entry_alignment_score={:.3}",
                            analyze
                                .pre_bayes_multi_timeframe_entry_alignment_score
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_soft_divergences={}",
                            if soft_divergences.is_empty() {
                                "none".to_string()
                            } else {
                                soft_divergences.join("|")
                            }
                        ),
                        format!(
                            "{}.promotion_status={}",
                            downstream.phase, downstream.promotion_status
                        ),
                    ],
                    sources: vec![WorkflowConflictSource {
                        scope: "pre_bayes_bridge".to_string(),
                        subject: "policy_version_and_selected_entry_quality".to_string(),
                        left_phase: analyze.phase.clone(),
                        left_value: format!(
                            "{}:{}",
                            analyze.pre_bayes_policy_version,
                            analyze
                                .pre_bayes_bridge_selected_entry_quality
                                .clone()
                                .unwrap_or_default()
                        ),
                        right_phase: downstream.phase.clone(),
                        right_value: downstream.promotion_status.clone(),
                        evidence: vec![
                            "neutralized gate conflicts with downstream promote".to_string(),
                            format!(
                                "long_short_signal_probability_gap={:.3}",
                                analyze.pre_bayes_bridge_probability_gap.unwrap_or_default()
                            ),
                            format!(
                                "multi_timeframe_direction_bias={}",
                                analyze.pre_bayes_multi_timeframe_direction_bias
                            ),
                            format!(
                                "soft_divergences={}",
                                if soft_divergences.is_empty() {
                                    "none".to_string()
                                } else {
                                    soft_divergences.join("|")
                                }
                            ),
                        ],
                    }],
                });
            }
        }
    }

    for (left, right) in [
        (research.as_ref(), update.as_ref()),
        (backtest.as_ref(), update.as_ref()),
        (research.as_ref(), backtest.as_ref()),
    ] {
        if let (Some(left), Some(right)) = (left, right) {
            let score_promotes = left.promotion_status == "promote"
                && right.workflow_phase == "artifact_rollback_review";
            let reverse_score_promotes = right.promotion_status == "promote"
                && left.workflow_phase == "artifact_rollback_review";
            if score_promotes || reverse_score_promotes {
                let (promote_phase, artifact_phase) = if score_promotes {
                    (left, right)
                } else {
                    (right, left)
                };
                disagreements.push(WorkflowDisagreement {
                    id: format!(
                        "{}_vs_{}_artifact_consumption_gate",
                        promote_phase.phase, artifact_phase.phase
                    ),
                    severity: "high".to_string(),
                    summary:
                        "score-based promotion conflicts with an artifact consumption rollback gate"
                            .to_string(),
                    phases: vec![promote_phase.phase.clone(), artifact_phase.phase.clone()],
                    recommended_action:
                        "resolve artifact consumption regression before trusting score-based promotion"
                            .to_string(),
                    evidence: vec![
                        format!(
                            "{}.promotion_status={}",
                            promote_phase.phase, promote_phase.promotion_status
                        ),
                        format!(
                            "{}.workflow_phase={}",
                            artifact_phase.phase, artifact_phase.workflow_phase
                        ),
                        format!(
                            "{}.rollback_scope={}",
                            artifact_phase.phase, artifact_phase.rollback_scope
                        ),
                    ],
                    sources: family_conflict_sources(promote_phase, artifact_phase)
                        .into_iter()
                        .chain(factor_conflict_sources(promote_phase, artifact_phase))
                        .collect(),
                });
            }
        }
    }

    if let (Some(backtest), Some(update)) = (backtest, update) {
        if backtest.rollback_scope == "none" && update.rollback_scope != "none" {
            disagreements.push(WorkflowDisagreement {
                id: "backtest_stable_vs_update_rollback".to_string(),
                severity: "medium".to_string(),
                summary: "backtest stayed stable but the latest realized update recommends rollback".to_string(),
                phases: vec![backtest.phase.clone(), update.phase.clone()],
                recommended_action: "inspect live execution drift and feedback provenance before keeping or rolling back changes".to_string(),
                evidence: vec![
                    format!("backtest.rollback_scope={}", backtest.rollback_scope),
                    format!("update.rollback_scope={}", update.rollback_scope),
                ],
                sources: family_conflict_sources(backtest, update)
                    .into_iter()
                    .chain(factor_conflict_sources(backtest, update))
                    .collect(),
            });
        }
    }

    if let (Some(research), Some(backtest)) = (research, backtest) {
        let sources = family_conflict_sources(research, backtest);
        if !sources.is_empty() {
            disagreements.push(WorkflowDisagreement {
                id: "research_backtest_family_conflicts".to_string(),
                severity: "medium".to_string(),
                summary: "research and backtest disagree on family-level decisions".to_string(),
                phases: vec![research.phase.clone(), backtest.phase.clone()],
                recommended_action: "inspect family score deltas and rollback scopes before acting on a single phase".to_string(),
                evidence: sources
                    .iter()
                    .map(|source| {
                        format!(
                            "family:{} {}={} {}={}",
                            source.subject,
                            source.left_phase,
                            source.left_value,
                            source.right_phase,
                            source.right_value
                        )
                    })
                    .collect(),
                sources,
            });
        }
        let sources = factor_conflict_sources(research, backtest);
        if !sources.is_empty() {
            disagreements.push(WorkflowDisagreement {
                id: "research_backtest_factor_conflicts".to_string(),
                severity: "medium".to_string(),
                summary: "research and backtest disagree on factor-level actions".to_string(),
                phases: vec![research.phase.clone(), backtest.phase.clone()],
                recommended_action: "check factor scorecards and iteration queue ordering before selecting the next factor edit".to_string(),
                evidence: sources
                    .iter()
                    .map(|source| {
                        format!(
                            "factor:{} {}={} {}={}",
                            source.subject,
                            source.left_phase,
                            source.left_value,
                            source.right_phase,
                            source.right_value
                        )
                    })
                    .collect(),
                sources,
            });
        }
    }

    disagreements
}

fn pre_bayes_soft_divergence_evidence(snapshot: &WorkflowPhaseSnapshot) -> Vec<String> {
    snapshot
        .pre_bayes_soft_evidence
        .iter()
        .filter_map(|(node, distribution)| {
            let dominant = distribution
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))?;
            let filtered = snapshot.pre_bayes_filtered_assignments.get(node)?;
            (dominant.0 != filtered).then(|| {
                format!(
                    "{}:{}->{:.3}:filtered={}",
                    node, dominant.0, dominant.1, filtered
                )
            })
        })
        .collect()
}

fn family_conflict_sources(
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
) -> Vec<WorkflowConflictSource> {
    let left_map = left
        .family_states
        .iter()
        .filter_map(|item| {
            let mut parts = item.splitn(3, ':');
            Some((
                parts.next()?.to_string(),
                format!("{}:{}", parts.next()?, parts.next()?),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let right_map = right
        .family_states
        .iter()
        .filter_map(|item| {
            let mut parts = item.splitn(3, ':');
            Some((
                parts.next()?.to_string(),
                format!("{}:{}", parts.next()?, parts.next()?),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    left_map
        .iter()
        .filter_map(|(family, left_value)| {
            let right_value = right_map.get(family)?;
            (left_value != right_value).then(|| WorkflowConflictSource {
                scope: "family".to_string(),
                subject: family.clone(),
                left_phase: left.phase.clone(),
                left_value: left_value.clone(),
                right_phase: right.phase.clone(),
                right_value: right_value.clone(),
                evidence: workflow_numeric_family_evidence(left, right, family),
            })
        })
        .collect()
}

fn factor_conflict_sources(
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
) -> Vec<WorkflowConflictSource> {
    let left_map = left
        .factor_actions
        .iter()
        .filter_map(|item| {
            let mut parts = item.splitn(3, ':');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect::<BTreeMap<_, _>>();
    let right_map = right
        .factor_actions
        .iter()
        .filter_map(|item| {
            let mut parts = item.splitn(3, ':');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect::<BTreeMap<_, _>>();
    left_map
        .iter()
        .filter_map(|(factor, left_value)| {
            let right_value = right_map.get(factor)?;
            (left_value != right_value).then(|| WorkflowConflictSource {
                scope: "factor".to_string(),
                subject: factor.clone(),
                left_phase: left.phase.clone(),
                left_value: left_value.clone(),
                right_phase: right.phase.clone(),
                right_value: right_value.clone(),
                evidence: workflow_numeric_factor_evidence(left, right, factor),
            })
        })
        .collect()
}

fn workflow_numeric_family_evidence(
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
    family: &str,
) -> Vec<String> {
    let left_score = left.family_score_map.get(family).copied();
    let right_score = right.family_score_map.get(family).copied();
    match (left_score, right_score) {
        (Some(left_score), Some(right_score)) => vec![
            format!("left_avg_score={:.4}", left_score),
            format!("right_avg_score={:.4}", right_score),
            format!("avg_score_delta={:.4}", right_score - left_score),
        ],
        _ => Vec::new(),
    }
}

fn workflow_numeric_factor_evidence(
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
    factor: &str,
) -> Vec<String> {
    let left_score = left.factor_score_map.get(factor).copied();
    let right_score = right.factor_score_map.get(factor).copied();
    match (left_score, right_score) {
        (Some(left_score), Some(right_score)) => vec![
            format!("left_composite_score={:.4}", left_score),
            format!("right_composite_score={:.4}", right_score),
            format!("composite_score_delta={:.4}", right_score - left_score),
        ],
        _ => Vec::new(),
    }
}

fn train_command(symbol: &str, data: &str, epochs: usize, state_dir: &str) -> Result<()> {
    let (observations, multi_timeframe_summary, candles_total) =
        build_multi_timeframe_training_observations(data)?;
    let initial_params = load_or_init_hmm_params(symbol, state_dir);
    let trained_params = BaumWelch::fit(&observations, &initial_params, epochs, 1e-4);
    let (_, log_likelihood) = ForwardBackward::forward(&observations, &trained_params);
    let (states, viterbi_log_likelihood) = Viterbi::decode(&observations, &trained_params);
    let learning_state = load_learning_state(state_dir, symbol)?;
    let previous_runs: Vec<TrainRunRecord> =
        load_state_or_default(state_dir, symbol, TRAIN_RUNS_FILE)?;
    let provenance = run_provenance(
        &learning_state,
        &["train", data, &epochs.to_string()],
        compute_hash(&["train", symbol, data, &epochs.to_string()]),
    );
    let dataset_comparability = dataset_comparability(
        previous_runs.last().map(|run| run.run_id.clone()),
        previous_runs.last().map(|run| &run.provenance),
        &provenance,
    );
    let workflow_state = WorkflowState {
        phase: "train_review_ready".to_string(),
        reason: "multi_timeframe_hmm_training_completed".to_string(),
    };
    let mut agent_action_plan = AgentActionPlan {
        summary: "review multi-timeframe HMM training outcome".to_string(),
        items: vec![AgentActionItem {
            stage: "train".to_string(),
            blocking: false,
            priority: "medium".to_string(),
            title: "Review Train Run".to_string(),
            rationale: format!(
                "epochs={} observations={} final_state={}",
                epochs,
                observations.len(),
                states.last().copied().map(state_name).unwrap_or("Unknown")
            ),
            expected_output: "A training review confirming whether the latest HMM state should feed the next analyze/research cycle".to_string(),
            expected_state_changes: vec![ExpectedStateChange {
                target: "hmm_params".to_string(),
                direction: "trained_multi_timeframe".to_string(),
                rationale: "multi_timeframe_hmm_training_completed".to_string(),
            }],
            suggested_files: vec!["src/main.rs".to_string(), "src/hmm/baum_welch.rs".to_string()],
            suggested_commands: vec!["ict-engine analyze --data-htf <file> --data-mtf <file> --data-ltf <file>".to_string()],
        }],
    };
    let recommended_commands = command_recommendations(&CommandContext {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        analyze: Some(AnalyzeCommandSource::Files {
            data_htf: data.to_string(),
            data_mtf: data.to_string(),
            data_ltf: data.to_string(),
        }),
        research_data: Some(data.to_string()),
        paired_data: None,
        update_outcome: None,
        update_entry_signal: None,
        update_feedback_file: pending_update_artifact_path(state_dir, symbol),
        user_data_selection_required: true,
    });
    concretize_action_plan_commands(&mut agent_action_plan, &recommended_commands);
    let recommended_next_command =
        recommended_next_command(&agent_action_plan, &recommended_commands);
    let mut agent_context_bundle = build_agent_context_bundle(BuildAgentContextBundleInput {
        symbol,
        state_dir,
        workflow_state: &workflow_state,
        decision_hint: "train_review_ready",
        recommended_next_command: &recommended_next_command,
        recommended_commands: &recommended_commands,
        dataset_comparability: &dataset_comparability,
        factor_iteration_queue: &[],
        family_outcomes: &[],
        pre_bayes_evidence_filter: None,
        pre_bayes_entry_quality_bridge: None,
        pda_sequence_summary: None,
        factor_mutation_evaluation: None,
        artifact_decision_summary: None,
    });
    agent_context_bundle.multi_timeframe_summary = multi_timeframe_summary.clone();
    let agent_context_bundle_minimal = build_agent_context_bundle_minimal(&agent_context_bundle);
    let mut agent_prompts = AgentPromptPack {
        version: PROMPT_PACK_VERSION.to_string(),
        workflow: format!(
            "Review the latest multi-timeframe HMM training result for {} before the next analyze/research cycle.",
            symbol
        ),
        prompts: vec![dataset_audit_prompt(symbol, data, None, candles_total, None, "train")],
    };
    agent_prompts.prompts.push(AgentPrompt::new(AgentPromptInput {
        id: "train_review".to_string(),
        stage: "train".to_string(),
        priority: "high".to_string(),
        objective: "Review whether the latest multi-timeframe HMM training result is usable.".to_string(),
        system_prompt: "You are the train-review agent. Use the training observations, likelihoods, and multi-timeframe summary to decide whether the latest HMM training result should feed the next analysis cycle or be treated cautiously.".to_string(),
        user_prompt: format!(
            "Symbol={} epochs={} observations={} final_state={} log_likelihood={:.4} viterbi_log_likelihood={:.4} multi_timeframe_summary={:?}",
            symbol,
            epochs,
            observations.len(),
            states.last().copied().map(state_name).unwrap_or("Unknown"),
            log_likelihood,
            viterbi_log_likelihood,
            multi_timeframe_summary
        ),
        success_criteria: vec![
            "Prefer using the trained HMM only when likelihoods are finite and multi-timeframe coverage is present".to_string(),
            "If multi-timeframe coverage is missing, downgrade confidence in the next analyze cycle".to_string(),
        ],
        suggested_files: vec!["src/main.rs".to_string(), "src/hmm/baum_welch.rs".to_string()],
    }));

    save_state(state_dir, symbol, HMM_STATE_FILE, &trained_params)?;
    append_train_run(
        state_dir,
        symbol,
        TrainRunRecord {
            run_id: format!(
                "train:{}:{}",
                symbol,
                Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
            ),
            timestamp: Utc::now(),
            symbol: symbol.to_string(),
            provenance,
            dataset_comparability,
            source_command: "train".to_string(),
            data_path: data.to_string(),
            epochs,
            candles: candles_total,
            observations: observations.len(),
            final_state: states
                .last()
                .copied()
                .map(state_name)
                .unwrap_or("Unknown")
                .to_string(),
            log_likelihood,
            viterbi_log_likelihood,
            workflow_state,
            agent_action_plan,
            recommended_commands,
            recommended_next_command_meta: recommended_next_command_meta(&recommended_next_command),
            recommended_next_command,
            agent_context_bundle,
            agent_context_bundle_minimal,
            agent_prompts: agent_prompts.clone(),
            prompt_workflow: agent_prompts.workflow.clone(),
            multi_timeframe_summary: multi_timeframe_summary.clone(),
        },
    )?;
    let workflow_snapshot = refresh_workflow_snapshot(state_dir, symbol)?;

    println!(
        "train symbol={} state_dir={} epochs={} candles={} observations={} final_state={} log_likelihood={:.4} viterbi_log_likelihood={:.4} multi_timeframe_summary={:?} workflow_phase={} saved={}/{}",
        symbol,
        state_dir,
        epochs,
        candles_total,
        observations.len(),
        states.last().copied().map(state_name).unwrap_or("Unknown"),
        log_likelihood,
        viterbi_log_likelihood,
        multi_timeframe_summary,
        workflow_snapshot.current_focus_phase,
        symbol,
        HMM_STATE_FILE,
    );
    Ok(())
}

struct RunFactorResearchInput<'a> {
    symbol: &'a str,
    data: &'a str,
    objective: ResearchObjectiveMode,
    data_1m: Option<&'a str>,
    data_5m: Option<&'a str>,
    data_15m: Option<&'a str>,
    data_1h: Option<&'a str>,
    data_4h: Option<&'a str>,
    data_1d: Option<&'a str>,
    paired_data: Option<&'a str>,
    mutation_spec: Option<&'a FactorMutationSpec>,
    state_dir: &'a str,
}

fn load_factor_mutation_spec(path: &str) -> Result<FactorMutationSpec> {
    let path_ref = std::path::Path::new(path);
    if path_ref
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("csv"))
        .unwrap_or(false)
    {
        bail!(
            "factor mutation spec must be a single JSON spec, not CSV: '{}'",
            path
        );
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read factor mutation spec '{}'", path))?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse factor mutation spec '{}'", path))?;
    if parsed.is_array() {
        bail!(
            "factor mutation spec must be a single JSON object, not a history array: '{}'",
            path
        );
    }
    let Some(obj) = parsed.as_object() else {
        bail!(
            "factor mutation spec must be a single JSON object with mutation_id/base_factor: '{}'",
            path
        );
    };
    if obj.contains_key("evaluation")
        || obj.contains_key("attempt_id")
        || obj.contains_key("session_id")
    {
        bail!(
            "factor mutation spec path points to run history/attempt artifact, not a single spec: '{}'",
            path
        );
    }
    if parsed.is_array() {
        bail!(
            "factor mutation spec must be a single JSON object, not a history array: '{}'",
            path
        );
    }
    let mut spec: FactorMutationSpec = serde_json::from_value(parsed)
        .with_context(|| format!("failed to decode factor mutation spec '{}'", path))?;
    if spec.mutation_id.trim().is_empty() {
        spec.mutation_id = format!(
            "mutation:{}",
            std::path::Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("unnamed")
        );
    }
    if spec.base_factor.trim().is_empty() {
        bail!("factor mutation spec missing base_factor: '{}'", path);
    }
    Ok(spec)
}

fn apply_factor_mutation_spec(
    registry: &mut FactorRegistry,
    spec: &FactorMutationSpec,
) -> Result<()> {
    if !spec.base_factor.is_empty() && registry.get(&spec.base_factor).is_none() {
        bail!("unknown mutation base_factor '{}'", spec.base_factor);
    }
    for (factor, enabled) in &spec.enabled_overrides {
        if !registry.set_enabled(factor, *enabled) {
            bail!("unknown factor '{}' in enabled_overrides", factor);
        }
    }
    for (parameter, value) in &spec.parameter_overrides {
        if spec.base_factor.is_empty() {
            bail!("parameter_overrides require a base_factor");
        }
        if !registry.set_parameter(&spec.base_factor, parameter, *value) {
            bail!(
                "unknown factor '{}' for parameter override '{}'",
                spec.base_factor,
                parameter
            );
        }
    }
    Ok(())
}

fn baseline_factor_mutation_metrics(
    input: BaselineFactorMutationMetricsInput<'_>,
) -> Result<FactorMutationMetricSet> {
    let BaselineFactorMutationMetricsInput {
        registry,
        symbol,
        objective,
        target_factor,
        baseline_learning_state,
        candles,
        paired_candles,
        multi_timeframe_summary,
        evaluate_expansion_preview,
    } = input;
    let mut learning_state = baseline_learning_state.clone();
    let lab = FactorLab::new(registry.clone());
    let mut report = lab.run_research(
        symbol,
        candles,
        &FactorContext {
            paired_candles,
            auxiliary: None,
            regime: None,
        },
        Some(&mut learning_state),
        &FactorBacktestConfig::default(),
        true,
    )?;
    report.research_objective = research_objective_label(objective).to_string();
    report.multi_timeframe_summary = multi_timeframe_summary.to_vec();
    let market_family = market_category_for_symbol(symbol).map(str::to_string);
    let objective_jump_weight = historical_market_jump_objective_weight(
        std::env::temp_dir(),
        symbol,
        market_family.as_deref(),
        Some(report.research_objective.as_str()),
    );
    if objective == ResearchObjectiveMode::ExpansionManipulation {
        apply_expansion_manipulation_objective(
            &mut report,
            registry,
            symbol,
            candles,
            multi_timeframe_summary,
            objective_jump_weight,
        )?;
    }
    build_factor_mutation_metric_set(
        &report,
        symbol,
        candles,
        registry,
        target_factor,
        evaluate_expansion_preview,
    )
}

fn build_factor_mutation_metric_set(
    report: &ict_engine::factor_lab::ResearchReport,
    symbol: &str,
    candles: &[Candle],
    registry: &FactorRegistry,
    target_factor: Option<&str>,
    evaluate_expansion_preview: bool,
) -> Result<FactorMutationMetricSet> {
    let evaluated_factor = target_factor
        .filter(|value| !value.trim().is_empty())
        .or(report.best_factor.as_deref());
    let best_factor_composite_score = evaluated_factor
        .and_then(|factor_name| {
            report
                .backtest
                .scorecards
                .iter()
                .find(|score| score.factor_name == factor_name)
                .map(|score| score.composite_score)
        })
        .or_else(|| {
            report
                .backtest
                .scorecards
                .first()
                .map(|score| score.composite_score)
        })
        .unwrap_or_default();
    let mut metrics = FactorMutationMetricSet {
        best_factor_composite_score,
        aggregate_return: report.aggregate_return,
        feedback_records_generated: report.feedback_records_generated,
        feedback_records_applied: report.feedback_records_applied,
        top_factor_names: report
            .backtest
            .scorecards
            .iter()
            .take(3)
            .map(|score| score.factor_name.clone())
            .collect(),
        ..FactorMutationMetricSet::default()
    };
    for item in &report.multi_timeframe_summary {
        if let Some(value) = item.strip_prefix("higher_timeframe_direction_bias=") {
            metrics.multi_timeframe_direction_bias = Some(value.to_string());
        } else if let Some(value) = item.strip_prefix("higher_timeframe_alignment_score=") {
            metrics.multi_timeframe_alignment_score = value.parse::<f64>().ok();
        } else if let Some(value) = item.strip_prefix("lower_timeframe_entry_alignment_score=") {
            metrics.multi_timeframe_entry_alignment_score = value.parse::<f64>().ok();
        }
    }
    if evaluate_expansion_preview {
        if let Some(best_factor) = evaluated_factor {
            let pipeline = build_expansion_factor_pipeline_report_with_registry_v2(
                symbol,
                best_factor,
                candles,
                None,
                &report.multi_timeframe_summary,
                registry,
            )?;
            let bridge_diff = pre_bayes_entry_quality_bridge_diff(&pipeline.entry_quality_bridge);
            let soft_diff = pre_bayes_soft_evidence_diff(&pipeline.bbn_support.pre_bayes_filter);
            let expansion_lookback = registry
                .get(best_factor)
                .map(|f| f.parameter("lookback", 20.0) as usize)
                .unwrap_or(20);
            let expansion_atr_mult = registry
                .get(best_factor)
                .map(|f| f.parameter("expansion_threshold", 1.5))
                .unwrap_or(1.5);
            let score = expansion_factor_scores_for_market(
                registry,
                candles,
                expansion_lookback,
                expansion_atr_mult,
            )?
            .into_iter()
            .find(|item| item.factor_name == best_factor);
            metrics.expansion_selected_direction =
                Some(pipeline.bbn_support.selected_direction.clone());
            metrics.expansion_selected_win_probability =
                Some(pipeline.bbn_support.selected_win_probability);
            metrics.expansion_balanced_accuracy = score.as_ref().map(|item| item.balanced_accuracy);
            metrics.expansion_directional_accuracy =
                score.as_ref().map(|item| item.directional_accuracy);
            metrics.pre_bayes_gate_status =
                Some(pipeline.bbn_support.pre_bayes_filter.gating_status.clone());
            metrics.pre_bayes_bridge_selected_entry_quality = bridge_diff.selected_entry_quality;
            metrics.pre_bayes_bridge_probability_gap =
                Some(bridge_diff.long_short_signal_probability_gap);
            metrics.pre_bayes_soft_evidence_divergence_count = soft_diff
                .iter()
                .filter(|item| item.diverges_from_filtered_state)
                .count();
        }
    }
    Ok(metrics)
}

fn evaluate_factor_mutation(
    spec: &FactorMutationSpec,
    objective: ResearchObjectiveMode,
    baseline_metrics: Option<&Result<FactorMutationMetricSet>>,
    report: &ict_engine::factor_lab::ResearchReport,
    candles: &[Candle],
    _paired_candles: Option<&[Candle]>,
) -> FactorMutationEvaluation {
    let mut registry = FactorRegistry::default();
    let _ = apply_factor_mutation_spec(&mut registry, spec);
    let metrics_after = build_factor_mutation_metric_set(
        report,
        &report.workflow_snapshot.symbol,
        candles,
        &registry,
        if spec.base_factor.is_empty() {
            None
        } else {
            Some(spec.base_factor.as_str())
        },
        spec.evaluate_expansion_preview,
    )
    .unwrap_or_default();
    let metrics_before = baseline_metrics.and_then(|result| result.as_ref().ok().cloned());
    let score_before = metrics_before
        .as_ref()
        .map(|metrics| mechanical_mutation_score(metrics, objective))
        .unwrap_or_default();
    let score_after = mechanical_mutation_score(&metrics_after, objective);
    let score_delta = score_after - score_before;
    let mut failure_tags = Vec::new();
    if metrics_before
        .as_ref()
        .map(|before| {
            metrics_after.best_factor_composite_score + 1e-9 < before.best_factor_composite_score
        })
        .unwrap_or(false)
    {
        failure_tags.push("best_factor_composite_regressed".to_string());
    }
    if metrics_after.pre_bayes_soft_evidence_divergence_count > 0 {
        failure_tags.push("soft_evidence_conflicts_with_filtered_label".to_string());
    }
    if metrics_after
        .pre_bayes_bridge_probability_gap
        .map(|gap| gap < 0.05)
        .unwrap_or(false)
    {
        failure_tags.push("bridge_gap_too_small".to_string());
    }
    let gate_before = metrics_before
        .as_ref()
        .and_then(|metrics| metrics.pre_bayes_gate_status.as_deref())
        .unwrap_or("observe_only");
    let gate_after = metrics_after
        .pre_bayes_gate_status
        .as_deref()
        .unwrap_or("observe_only");
    if objective == ResearchObjectiveMode::ExpansionManipulation {
        if pre_bayes_gate_regressed(gate_before, gate_after) {
            failure_tags.push("pre_bayes_gate_regressed".to_string());
        }
    } else if gate_after == "observe_only" {
        failure_tags.push("pre_bayes_gate_observe_only".to_string());
    }
    if no_superior_mutation_found(score_delta, &failure_tags, objective) {
        failure_tags.push("no_superior_mutation_found".to_string());
    }
    let recommended_mutation_directions = if failure_tags.is_empty() {
        vec![
            "Keep the mutation atomic and continue searching for incremental PreBayes/bridge improvements"
                .to_string(),
        ]
    } else {
        recommended_mutation_directions_from_failure_tags(&failure_tags, &[], &BTreeMap::new())
    };
    FactorMutationEvaluation {
        mutation_id: spec.mutation_id.clone(),
        accepted: score_delta > 0.0 && failure_tags.is_empty(),
        score_before,
        score_after,
        score_delta,
        baseline_available: metrics_before.is_some(),
        reason: if score_delta > 0.0 && failure_tags.is_empty() {
            "mechanical_score_improved_without_pre_bayes_regression".to_string()
        } else if failure_tags.is_empty() {
            "mechanical_score_not_improved".to_string()
        } else {
            format!("mutation_flagged:{}", failure_tags.join(","))
        },
        failure_tags,
        recommended_mutation_directions,
        metrics_before,
        metrics_after,
    }
}

fn augment_action_plan_with_factor_mutation_evaluation(
    action_plan: &mut AgentActionPlan,
    evaluation: &FactorMutationEvaluation,
) {
    let priority_markets = factor_mutation_priority_markets(evaluation);
    let priority_reasons = factor_mutation_priority_reasons(evaluation);
    let recommended_focus = factor_mutation_recommended_focus(evaluation);
    action_plan.items.insert(
        0,
        AgentActionItem {
            stage: "iteration".to_string(),
            blocking: !evaluation.accepted,
            priority: "high".to_string(),
            title: if evaluation.accepted {
                "Promote Factor Mutation Candidate".to_string()
            } else {
                "Reject Factor Mutation Candidate".to_string()
            },
            rationale: format!(
                "mutation_id={} reason={} score_delta={:.4} priority_markets={} priority_reasons={}",
                evaluation.mutation_id,
                evaluation.reason,
                evaluation.score_delta,
                if priority_markets.is_empty() {
                    "none".to_string()
                } else {
                    priority_markets.join("|")
                },
                if priority_reasons.is_empty() {
                    "none".to_string()
                } else {
                    priority_reasons.join("|")
                }
            ),
            expected_output: "A mechanical mutation decision with explicit accept/reject status and failure tags".to_string(),
            expected_state_changes: vec![
                ExpectedStateChange {
                    target: "factor_mutation_evaluation".to_string(),
                    direction: if evaluation.accepted {
                        "accepted".to_string()
                    } else if evaluation
                        .failure_tags
                        .iter()
                        .any(|tag| tag == "no_superior_mutation_found")
                    {
                        "near_local_optimum".to_string()
                    } else {
                        "rejected".to_string()
                    },
                    rationale: if evaluation.failure_tags.is_empty() {
                        evaluation.reason.clone()
                    } else {
                        evaluation.failure_tags.join(",")
                    },
                },
                ExpectedStateChange {
                    target: "factor_mutation_focus".to_string(),
                    direction: if recommended_focus.is_empty() {
                        "review_required".to_string()
                    } else if evaluation
                        .failure_tags
                        .iter()
                        .any(|tag| tag == "no_superior_mutation_found")
                    {
                        "pivot_to_label_refinement_or_market_specific_fork".to_string()
                    } else {
                        "prioritized".to_string()
                    },
                    rationale: if recommended_focus.is_empty() {
                        "no explicit mutation focus available".to_string()
                    } else {
                        recommended_focus.join(" | ")
                    },
                },
            ],
            suggested_files: vec!["src/main.rs".to_string(), "src/factors/registry.rs".to_string()],
            suggested_commands: vec![
                "ict-engine factor-research --symbol <symbol> --data <file> --mutation-spec <spec.json> --emit-mutation-evaluation"
                    .to_string(),
            ],
        },
    );
}

fn build_analyze_report(input: BuildAnalyzeReportInput<'_>) -> Result<AnalyzeReport> {
    let BuildAnalyzeReportInput {
        symbol,
        state_dir,
        htf,
        mtf,
        ltf,
        params,
        network,
        build_context,
        execution_focus,
    } = input;
    let htf_features = build_frame_features(htf)?;
    let mtf_features = build_frame_features(mtf)?;
    let ltf_features = build_frame_features(ltf)?;
    let native_signals = native_frame_computations(params, build_context.native_frames)?;

    let regime_label = if native_signals.is_empty() {
        combine_regime_labels(&[
            htf_features.regime_label.as_str(),
            mtf_features.regime_label.as_str(),
            ltf_features.regime_label.as_str(),
        ])
    } else {
        weighted_majority_label(
            native_signals
                .iter()
                .map(|signal| (signal.features.regime_label.as_str(), signal.weight)),
            "bull",
            "bear",
            "range",
        )
    };
    let liquidity_label = if native_signals.is_empty() {
        combine_liquidity_labels(&[
            htf_features.liquidity_label.as_str(),
            mtf_features.liquidity_label.as_str(),
            ltf_features.liquidity_label.as_str(),
        ])
    } else {
        weighted_majority_label(
            native_signals
                .iter()
                .map(|signal| (signal.features.liquidity_label.as_str(), signal.weight)),
            "favorable",
            "hostile",
            "neutral",
        )
    };

    let (hmm_state, log_likelihood, viterbi_log_likelihood, regime_probs) = if native_signals
        .is_empty()
    {
        let (log_alpha, log_likelihood) =
            ForwardBackward::forward(&ltf_features.observations, params);
        let log_beta = ForwardBackward::backward(&ltf_features.observations, params);
        let gamma = ForwardBackward::compute_gamma(&log_alpha, &log_beta, log_likelihood);
        let (states, viterbi_log_likelihood) = Viterbi::decode(&ltf_features.observations, params);
        (
            states
                .last()
                .copied()
                .map(state_name)
                .unwrap_or("Unknown")
                .to_string(),
            log_likelihood,
            viterbi_log_likelihood,
            regime_probs_from_log_gamma(gamma.last())?,
        )
    } else {
        let weighted_regimes = native_signals
            .iter()
            .map(|signal| (signal.regime_probs, signal.weight))
            .collect::<Vec<_>>();
        let total_weight = native_signals
            .iter()
            .map(|signal| signal.weight)
            .sum::<f64>()
            .max(f64::EPSILON);
        (
            match weighted_regime_probs(&weighted_regimes).dominant() {
                Regime::Accumulation => "Accumulation",
                Regime::ManipulationExpansion => "ManipulationExpansion",
                Regime::Distribution => "Distribution",
            }
            .to_string(),
            native_signals
                .iter()
                .map(|signal| signal.log_likelihood * signal.weight)
                .sum::<f64>()
                / total_weight,
            native_signals
                .iter()
                .map(|signal| signal.viterbi_log_likelihood * signal.weight)
                .sum::<f64>()
                / total_weight,
            weighted_regime_probs(&weighted_regimes),
        )
    };

    let native_htf = build_context
        .native_frames
        .h4
        .or(build_context.native_frames.h1)
        .unwrap_or(htf);
    let native_mtf = build_context.native_frames.m15.unwrap_or(mtf);
    let native_ltf = build_context
        .native_frames
        .m5
        .or(build_context.native_frames.m1)
        .unwrap_or(ltf);

    let atr_htf = left_pad(
        compute_atr(native_htf, INDICATOR_PERIOD),
        native_htf.len(),
        0.0,
    );
    let atr_ltf = left_pad(
        compute_atr(native_ltf, INDICATOR_PERIOD),
        native_ltf.len(),
        0.0,
    );
    let cascade_config = CascadeConfig::default();
    let cascade_bull = cascade_bull(
        native_htf,
        native_mtf,
        native_ltf,
        &cascade_config,
        &atr_htf,
        &atr_ltf,
    );
    let cascade_bear = cascade_bear(
        native_htf,
        native_mtf,
        native_ltf,
        &cascade_config,
        &atr_htf,
        &atr_ltf,
    );
    let pre_bayes_policy = pre_bayes_evidence_policy();
    let multi_timeframe_evidence =
        parse_multi_timeframe_evidence(build_context.multi_timeframe_summary);
    let market = infer_market_from_symbol(build_context.symbol);
    let pda_sequence_artifact =
        ict_engine::pda_sequence::load_pda_sequence_analysis(state_dir, symbol).ok();
    let pda_sequence_summary = pda_sequence_artifact
        .as_ref()
        .map(ict_engine::pda_sequence::summarize_pda_sequence_artifact);
    let previous_runs: Vec<AnalyzeRunRecord> =
        load_state_or_default(state_dir, symbol, ANALYZE_RUNS_FILE)?;
    let initial_hybrid_regime_packet = build_hybrid_regime_packet(
        Some(&htf_features),
        &ltf_features,
        None,
        None,
        Some(&market),
        &[],
        pda_sequence_summary.as_ref(),
    )?;
    let current_hybrid_label = initial_hybrid_regime_packet
        .active_regime_cluster
        .as_deref()
        .unwrap_or_default()
        .to_string();
    let historical_hybrid_regime_ages = previous_runs
        .iter()
        .rev()
        .take(20)
        .filter(|run| run.hybrid_regime_label.as_deref() == Some(current_hybrid_label.as_str()))
        .filter_map(|run| run.hybrid_regime_age_bars)
        .collect::<Vec<_>>();
    let current_hybrid_age_bars = previous_runs
        .last()
        .map(|run| {
            if run.hybrid_regime_label.as_deref() == Some(current_hybrid_label.as_str()) {
                run.hybrid_regime_age_bars.unwrap_or(1) + 1
            } else {
                1
            }
        })
        .unwrap_or(1);
    let hybrid_regime_packet = build_hybrid_regime_packet(
        Some(&htf_features),
        &ltf_features,
        None,
        Some(current_hybrid_age_bars),
        Some(&market),
        &historical_hybrid_regime_ages,
        pda_sequence_summary.as_ref(),
    )?;
    let mut factor_registry = FactorRegistry::default();
    factor_registry.apply_learning_state(build_context.learning_state);
    let factor_engine = FactorEngine::new(factor_registry);
    let factor_output = factor_engine.run(
        ltf,
        &FactorContext {
            paired_candles: build_context.paired_candles,
            auxiliary: build_context.auxiliary,
            regime: Some(regime_probs.dominant()),
        },
        Some(build_context.learning_state),
    )?;
    let pre_bayes_evidence_filter = build_pre_bayes_evidence_filter(
        &pre_bayes_policy,
        &regime_label,
        &liquidity_label,
        &factor_output.diagnostics,
        &multi_timeframe_evidence,
        Some(&market),
        pda_sequence_summary.as_ref(),
    );

    let evidence = trade_evidence_from_pre_bayes_filter(network, &pre_bayes_evidence_filter)?;
    let base_entry_quality = infer_entry_quality(network, &evidence)?;
    let long_entry_bias = combine_bias_vectors(
        &combine_bias_vectors(
            &entry_quality_bias_from_signal(cascade_bull.final_posterior),
            &factor_output
                .diagnostics
                .entry_bias_for_direction(Direction::Bull),
        ),
        &multi_timeframe_entry_quality_bias(&multi_timeframe_evidence, Direction::Bull),
    );
    let short_entry_bias = combine_bias_vectors(
        &combine_bias_vectors(
            &entry_quality_bias_from_signal(cascade_bear.final_posterior),
            &factor_output
                .diagnostics
                .entry_bias_for_direction(Direction::Bear),
        ),
        &multi_timeframe_entry_quality_bias(&multi_timeframe_evidence, Direction::Bear),
    );
    let long_entry_quality = infer_entry_quality_with_bias(network, &evidence, &long_entry_bias)?;
    let short_entry_quality = infer_entry_quality_with_bias(network, &evidence, &short_entry_bias)?;
    let posterior = infer_trade_outcome(network, &evidence)?;
    let bull_trade_outcome = apply_factor_outcome_overlay(
        &infer_trade_outcome_with_entry_quality_bias(network, &evidence, &long_entry_bias)?,
        factor_output.diagnostics.directional_bias(Direction::Bull),
        factor_output.diagnostics.uncertainty,
    );
    let bear_trade_outcome = apply_factor_outcome_overlay(
        &infer_trade_outcome_with_entry_quality_bias(network, &evidence, &short_entry_bias)?,
        factor_output.diagnostics.directional_bias(Direction::Bear),
        factor_output.diagnostics.uncertainty,
    );
    let trade_outcome = network
        .nodes
        .get("trade_outcome")
        .ok_or_else(|| anyhow!("missing node 'trade_outcome'"))?;
    let fvgs = find_unfilled_fvgs(native_mtf);
    let obs = find_untested_obs(native_mtf);
    let decision = probabilistic_decision_snapshot(
        &regime_probs,
        &cascade_bull,
        &cascade_bear,
        &bull_trade_outcome,
        &bear_trade_outcome,
    );
    let entry_quality_node = network
        .nodes
        .get("entry_quality")
        .ok_or_else(|| anyhow!("missing node 'entry_quality'"))?;
    let selected_entry_quality_distribution = match decision.selected_direction {
        Direction::Bull => &long_entry_quality,
        Direction::Bear => &short_entry_quality,
        Direction::Neutral => &base_entry_quality,
    };
    let selected_entry_quality_state =
        select_state_name(selected_entry_quality_distribution, entry_quality_node)?;
    let pre_bayes_entry_quality_bridge =
        build_pre_bayes_entry_quality_bridge(PreBayesEntryQualityBridgeInput {
            factor_diagnostics: factor_output.diagnostics.clone(),
            decision: decision.clone(),
            long_entry_bias: long_entry_bias.clone(),
            short_entry_bias: short_entry_bias.clone(),
            long_entry_quality: long_entry_quality.clone(),
            short_entry_quality: short_entry_quality.clone(),
            selected_entry_quality: selected_entry_quality_distribution.to_vec(),
            entry_quality_states: entry_quality_node.states.clone(),
            multi_timeframe_evidence: multi_timeframe_evidence.clone(),
        });
    let trade_plan = generate_probabilistic_trade_plan(ProbabilisticTradePlanInput {
        mtf: native_mtf,
        ltf: native_ltf,
        fvgs: &fvgs,
        obs: &obs,
        symbol,
        regime_probs,
        cascade_bull: &cascade_bull,
        cascade_bear: &cascade_bear,
        bull_trade_outcome: &bull_trade_outcome,
        bear_trade_outcome: &bear_trade_outcome,
        config: &ProbabilisticPlanConfig::default(),
    });
    let mut trade_plan =
        apply_duration_sizing_adjustment(trade_plan, symbol, &hybrid_regime_packet);
    trade_plan.uncertainties.push(format!(
        "factor_uncertainty={:.3}",
        factor_output.diagnostics.uncertainty
    ));
    trade_plan.uncertainties.push(format!(
        "pre_bayes_gating_status={}",
        pre_bayes_evidence_filter.gating_status
    ));
    trade_plan.uncertainties.push(format!(
        "native_execution_frames=htf:{} mtf:{} ltf:{}",
        if std::ptr::eq(native_htf.as_ptr(), htf.as_ptr()) {
            "provided"
        } else {
            "native"
        },
        if std::ptr::eq(native_mtf.as_ptr(), mtf.as_ptr()) {
            "provided"
        } else {
            "native"
        },
        if std::ptr::eq(native_ltf.as_ptr(), ltf.as_ptr()) {
            "provided"
        } else {
            "native"
        }
    ));
    if let Some(remaining) = hybrid_regime_packet.duration_remaining_expected_bars {
        trade_plan
            .uncertainties
            .push(format!("hybrid_remaining_expected_bars={remaining:.3}"));
    }
    if let Some(model) = &hybrid_regime_packet.duration_model {
        trade_plan
            .uncertainties
            .push(format!("hybrid_duration_model={model}"));
    }
    let price_action = build_price_action_section(native_mtf, native_ltf, &atr_ltf, &fvgs, &obs);
    let technical_price =
        build_technical_price_section(native_ltf, None, None, build_context.auxiliary);
    let smt_correlation = if let Some(paired) = build_context.paired_candles {
        let fallback_auxiliary;
        let auxiliary = if let Some(auxiliary) = build_context.auxiliary {
            auxiliary
        } else {
            fallback_auxiliary = neutral_auxiliary(symbol);
            &fallback_auxiliary
        };
        build_smt_correlation_section(
            symbol,
            &format!("{}_paired", symbol),
            native_ltf,
            paired,
            auxiliary,
        )
    } else {
        empty_smt_correlation_section()
    };
    let regime_bayesian = build_regime_bayesian_section(
        &hmm_state,
        &regime_probs,
        &regime_label,
        &liquidity_label,
        &decision,
        "hmm_prior_times_bbn_trade_probability",
        None,
        Some(&hybrid_regime_packet),
        pda_sequence_summary.as_ref(),
    );
    let multi_timeframe_section = build_analyze_multi_timeframe_section(
        build_context.multi_timeframe_summary,
        Some(&pre_bayes_evidence_filter),
    );
    let trade_plan_section = build_trade_plan_section(&trade_plan, None);
    let factor_ranking = if build_context.learning_state.factor_rankings.is_empty() {
        analyze_signal_rankings(&factor_output.latest_signals, regime_probs.dominant())
    } else {
        build_context.learning_state.factor_rankings.clone()
    };
    let factor_iteration_queue = if build_context.learning_state.factor_rankings.is_empty() {
        factor_ranking
            .iter()
            .map(FactorIterationPrompt::from)
            .filter(|item| item.iteration_action != "keep" || item.replacement_candidate)
            .collect()
    } else {
        build_context.learning_state.iteration_queue()
    };
    let factor_family_decisions = if build_context.learning_state.factor_rankings.is_empty() {
        let synthetic_state = LearningState {
            factor_rankings: factor_ranking.clone(),
            ..LearningState::default()
        };
        synthetic_state.family_decisions()
    } else {
        build_context.learning_state.family_decisions()
    };
    let feedback_history_summary = build_context.learning_state.summary();
    let analyze_provenance = run_provenance(
        build_context.learning_state,
        &["analyze", symbol],
        data_fingerprint(ltf, build_context.paired_candles, "analyze"),
    );
    let dataset_comparability = dataset_comparability(
        previous_runs.last().map(|run| run.run_id.clone()),
        previous_runs.last().map(|run| &run.provenance),
        &analyze_provenance,
    );
    let thresholds = decision_thresholds();
    let base_decision_hint = build_analyze_decision_hint(
        &dataset_comparability,
        &factor_iteration_queue,
        &factor_output.diagnostics,
    );
    let base_decision_hint = append_pda_sequence_hint(
        &base_decision_hint,
        pda_sequence_summary.as_ref(),
        &pre_bayes_evidence_filter,
    );
    let multi_timeframe_hint = if build_context.multi_timeframe_summary.is_empty() {
        "|multi_timeframe_hint_unavailable".to_string()
    } else {
        format!(
            "|{}",
            multi_timeframe_phase_hint(build_context.multi_timeframe_summary)
        )
    };
    let decision_hint = format!(
        "{}|hybrid_regime_label={}|hybrid_regime_age={}|pre_bayes_gating_status={}|pre_bayes_quality_score={:.3}{}",
        base_decision_hint,
        hybrid_regime_packet
            .active_regime_cluster
            .as_deref()
            .unwrap_or("unknown"),
        current_hybrid_age_bars,
        pre_bayes_evidence_filter.gating_status,
        pre_bayes_evidence_filter.evidence_quality_score,
        multi_timeframe_hint
    );
    let (_, historical_artifact_family_trends) =
        artifact_trend_summaries_for_symbol(state_dir, symbol)?;
    let factor_family_outcomes = derive_family_outcomes(
        &factor_family_decisions,
        &thresholds,
        &dataset_comparability,
        Some(&historical_artifact_family_trends),
    );
    let factor_family_diffs = family_diffs(
        previous_runs
            .last()
            .map(|run| run.factor_family_decisions.as_slice())
            .unwrap_or(&[]),
        &factor_family_decisions,
    );
    let factor_family_history = family_history_from_runs(previous_runs.iter().map(|run| {
        (
            run.run_id.clone(),
            run.timestamp,
            run.factor_family_decisions.clone(),
        )
    }));
    let decision_history_summary = decision_history_summary(previous_runs.iter().map(|run| {
        (
            run.promotion_decision.clone(),
            run.rollback_recommendation.clone(),
        )
    }));
    let observe_promotion = PromotionDecision {
        approved: false,
        status: "observe".to_string(),
        reason: dataset_comparability.reason.clone(),
        target_factors: Vec::new(),
        target_families: Vec::new(),
    };
    let observe_rollback = RollbackRecommendation {
        should_rollback: false,
        scope: "none".to_string(),
        reason: "analyze_observe_only".to_string(),
        target_factors: Vec::new(),
        target_families: Vec::new(),
    };
    let workflow_state = workflow_state_from_pre_bayes_filter(
        workflow_state_from_context(&decision_hint, &observe_promotion, &observe_rollback),
        &pre_bayes_evidence_filter,
    );
    let mut agent_action_plan = build_agent_action_plan(
        &decision_hint,
        &observe_promotion,
        &observe_rollback,
        &factor_iteration_queue,
        &factor_family_outcomes,
    );
    augment_action_plan_with_pre_bayes_filter(&mut agent_action_plan, &pre_bayes_evidence_filter);
    let recommended_commands = command_recommendations(&CommandContext {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        analyze: None,
        research_data: None,
        paired_data: None,
        update_outcome: None,
        update_entry_signal: None,
        update_feedback_file: pending_update_artifact_path(state_dir, symbol),
        user_data_selection_required: true,
    });
    concretize_action_plan_commands(&mut agent_action_plan, &recommended_commands);
    let recommended_next_command =
        recommended_next_command(&agent_action_plan, &recommended_commands);
    let pda_sequence_artifact =
        ict_engine::pda_sequence::load_pda_sequence_analysis(state_dir, symbol).ok();
    let mut agent_context_bundle = build_agent_context_bundle(BuildAgentContextBundleInput {
        symbol,
        state_dir,
        workflow_state: &workflow_state,
        decision_hint: &decision_hint,
        recommended_next_command: &recommended_next_command,
        recommended_commands: &recommended_commands,
        dataset_comparability: &dataset_comparability,
        factor_iteration_queue: &factor_iteration_queue,
        family_outcomes: &factor_family_outcomes,
        pre_bayes_evidence_filter: Some(&pre_bayes_evidence_filter),
        pre_bayes_entry_quality_bridge: Some(&pre_bayes_entry_quality_bridge),
        pda_sequence_summary: pda_sequence_artifact
            .as_ref()
            .map(ict_engine::pda_sequence::summarize_pda_sequence_artifact)
            .as_ref(),
        factor_mutation_evaluation: None,
        artifact_decision_summary: None,
    });
    agent_context_bundle.multi_timeframe_summary = build_context.multi_timeframe_summary.to_vec();
    let agent_prompts = build_analyze_agent_prompts(BuildAnalyzeAgentPromptsInput {
        symbol,
        decision: &decision,
        factor_diagnostics: &factor_output.diagnostics,
        pre_bayes_evidence_filter: &pre_bayes_evidence_filter,
        factor_ranking: &factor_ranking,
        factor_iteration_queue: &factor_iteration_queue,
        feedback_history_summary: &feedback_history_summary,
        trade_plan: &trade_plan,
        dataset_comparability: &dataset_comparability,
        decision_hint: &decision_hint,
        multi_timeframe_summary: build_context.multi_timeframe_summary,
    });
    let canonical_belief_report = build_canonical_belief_snapshot_with_pda(
        symbol,
        Some(infer_market_from_symbol(symbol).as_str()),
        &pre_bayes_evidence_filter,
        pda_sequence_artifact.as_ref(),
        Some(&hybrid_regime_packet),
    )?;
    let execution_inputs = derive_execution_inputs(&ExecutionInputSources {
        pre_bayes_evidence_filter: &pre_bayes_evidence_filter,
        pre_bayes_entry_quality_bridge: &pre_bayes_entry_quality_bridge,
        selected_entry_quality_distribution,
        selected_win_probability: decision.selected_win_probability,
    });
    let ltf_prices = native_ltf
        .iter()
        .map(|candle| candle.close)
        .collect::<Vec<_>>();
    let ltf_timestamps = native_ltf
        .iter()
        .map(|candle| candle.timestamp)
        .collect::<Vec<_>>();
    let ltf_ou_fallback = ExecutionOuFallback {
        normalized_distance_to_projected_trend_bps: ltf_features
            .normalized_distance_to_projected_trend_bps,
        ou_half_life_bars: ltf_features.ou_half_life_bars,
        ou_pullback_expectation_zscore: ltf_features.ou_pullback_expectation_zscore,
        ou_reversion_speed_per_bar: ltf_features.ou_reversion_speed_per_bar,
        ou_expected_pullback_bps: ltf_features.ou_expected_pullback_bps,
    };
    let mut pipeline_state = PipelineState::new(
        symbol,
        Some(infer_market_from_symbol(symbol).as_str()),
        "ict_engine_staged_orchestration",
    );
    let physics_overlay = apply_physics_overlay(&mut pipeline_state, native_ltf, &ltf_features);
    let execution_artifact = build_execution_artifact_from_snapshot(
        symbol,
        &execution_inputs,
        ExecutionArtifactBuildContext {
            prices: Some(&ltf_prices),
            timestamps: Some(&ltf_timestamps),
            fallback_ou: Some(&ltf_ou_fallback),
            physics_overlay: Some(&physics_overlay),
        },
        &analyze_provenance,
    );

    let mece_labels = manual_mece_labeler(native_ltf, &ltf_features);
    let mece_recovery_report = search_factors_for_mece_recovery(
        native_ltf,
        &mece_labels,
        &factor_engine.registry,
        analyze_provenance.clone(),
    )
    .ok();
    let mece_recovery_confidence = mece_recovery_report.as_ref().map(|report| report.accuracy);
    if let Some(report) = mece_recovery_report.as_ref() {
        let mece_artifact = build_mece_recovery_artifact(symbol, report, &[], &mece_labels);
        persist_mece_recovery_artifact(state_dir, &mece_artifact, "analyze", None, &decision_hint)?;
    }

    let execution_tree_input = ExecutionTreeInput {
        execution_features: &execution_artifact.features,
        physics_overlay: &physics_overlay,
        hmm_posterior: &regime_probs,
        mece_recovery_confidence,
        prediction_vote_score: decision.selected_win_probability,
        axial_trace: None,
    };
    let execution_tree_output = DefaultExecutionTreeScorer.score(&execution_tree_input)?;
    let execution_tree_output =
        apply_regime_execution_guardrail(execution_tree_output, &hybrid_regime_packet);
    if hybrid_regime_packet.transition_hazard.unwrap_or_default() >= 0.60 {
        trade_plan.uncertainties.push(format!(
            "hybrid_transition_hazard={:.3}",
            hybrid_regime_packet.transition_hazard.unwrap_or_default()
        ));
    }
    if hybrid_regime_packet
        .evidence
        .iter()
        .any(|line| line == "pda_hybrid_alignment=false")
    {
        trade_plan
            .uncertainties
            .push("pda_hybrid_alignment=false".to_string());
    }
    let execution_shap_top_k = StructuralExecutionShap::default()
        .attributions(&execution_tree_input, &execution_tree_output);
    let execution_triage = if execution_focus {
        Some(build_execution_triage(&execution_tree_output))
    } else {
        None
    };
    let execution_tree_artifact = build_execution_tree_artifact(
        symbol,
        execution_tree_output,
        execution_shap_top_k,
        analyze_provenance.clone(),
    );
    persist_execution_tree_artifact(state_dir, &execution_tree_artifact, "analyze", None)?;

    let staged_orchestration_trace = if staged_orchestration_enabled() {
        let stage_trace = run_stage_plan(&StagePlan::analyze_risk_execution(), &mut pipeline_state);
        let policy_engine = CatBoostCompatiblePolicyEngine::load_default_or_placeholder();
        let staged_artifacts = ict_engine::application::orchestration::build_staged_artifacts(
            StagedArtifactsInput {
                diagnostics: &factor_output.diagnostics,
                decision_hint: &decision_hint,
                filter: &pre_bayes_evidence_filter,
                multi_timeframe_summary: build_context.multi_timeframe_summary,
                selected_entry_quality: &selected_entry_quality_state,
                direction: trade_plan.direction,
                risk_reward: trade_plan.risk_reward,
                kelly_fraction: trade_plan.kelly_fraction,
                recommended_command: &recommended_next_command,
            },
            &policy_engine,
        );
        let final_adapter = FinalOutputAdapter;
        let final_artifact = final_adapter.adapt(&pipeline_state, &stage_trace);
        Some(serde_json::json!({
            "pipeline_state": pipeline_state,
            "stage_trace": stage_trace,
            "staged_artifacts": staged_artifacts,
            "final_artifact": final_artifact,
        }))
    } else {
        None
    };

    Ok(AnalyzeReport {
        symbol: symbol.to_string(),
        timestamp: Utc::now(),
        analysis: AnalyzeSections {
            price_action,
            technical_price,
            smt_correlation,
            regime_bayesian,
            multi_timeframe: multi_timeframe_section,
            trade_plan: trade_plan_section,
        },
        meta: AnalyzeMeta {
            state_dir: state_dir.to_string(),
            bars: AnalyzeBars {
                htf: htf.len(),
                mtf: mtf.len(),
                ltf: ltf.len(),
                observations: ltf_features.observations.len(),
            },
            data_source: None,
        },
        supporting: AnalyzeSupporting {
            model_state: AnalyzeModelState {
                hmm_state: hmm_state.clone(),
                log_likelihood,
                viterbi_log_likelihood,
                regime_probs,
                evidence_policy:
                    "multi_timeframe_hmm_prior_times_pre_bayes_evidence_filter_times_bbn_trade_probability"
                        .to_string(),
                canonical_belief_engine: canonical_belief_report.engine_trace.primary_engine.clone(),
                canonical_shadow_status: canonical_belief_report
                    .shadow_comparison
                    .as_ref()
                    .map(|summary| summary.status.clone())
                    .unwrap_or_else(|| "shadow=unavailable".to_string()),
            },
            provenance: analyze_provenance,
            promotion_decision: observe_promotion,
            rollback_recommendation: observe_rollback,
            labels: AnalyzeLabels {
                regime_label,
                liquidity_label,
            },
            ict: AnalyzeIctSummary {
                total_sweeps: if native_signals.is_empty() {
                    htf_features.sweep_count + mtf_features.sweep_count + ltf_features.sweep_count
                } else {
                    native_signals
                        .iter()
                        .map(|signal| signal.features.sweep_count)
                        .sum()
                },
                total_fvgs: if native_signals.is_empty() {
                    htf_features.fvg_count + mtf_features.fvg_count + ltf_features.fvg_count
                } else {
                    native_signals
                        .iter()
                        .map(|signal| signal.features.fvg_count)
                        .sum()
                },
                mtf_open_fvgs: fvgs.len(),
                mtf_untested_obs: obs.len(),
                ict_role: "native_multi_timeframe_evidence_only_non_deterministic".to_string(),
            },
            entry_quality: AnalyzeEntryQualitySummary {
                base: probability_map(&entry_quality_node.states, &base_entry_quality),
                long: probability_map(&entry_quality_node.states, &long_entry_quality),
                short: probability_map(&entry_quality_node.states, &short_entry_quality),
                selected_state: selected_entry_quality_state,
            },
            auxiliary: build_context.auxiliary.cloned(),
            decision,
            trade_outcome: AnalyzeTradeOutcomeSummary {
                base: probability_map(&trade_outcome.states, &posterior),
                long: probability_map(&trade_outcome.states, &bull_trade_outcome),
                short: probability_map(&trade_outcome.states, &bear_trade_outcome),
            },
            factor_diagnostics: factor_output.diagnostics,
            pre_bayes_evidence_filter: pre_bayes_evidence_filter.clone(),
            pre_bayes_entry_quality_bridge: pre_bayes_entry_quality_bridge.clone(),
            objective_jump_weight: canonical_belief_report.gate_decision.jump_weight,
            canonical_belief_report: canonical_belief_report.clone(),
            decision_thresholds: thresholds,
            factor_ranking,
            factor_iteration_queue,
            factor_family_decisions,
            factor_family_outcomes,
            factor_family_diffs,
            factor_family_history,
            decision_history_summary,
            workflow_state,
            agent_context_bundle: agent_context_bundle.clone(),
            agent_context_bundle_minimal: build_agent_context_bundle_minimal(&agent_context_bundle),
            recommended_commands,
            recommended_next_command,
            agent_action_plan,
            dataset_comparability,
            decision_hint,
            artifact_action_summary: Vec::new(),
            artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary::default(),
            artifact_decision_section: ict_engine::state::ArtifactDecisionSection::default(),
            agent_prompts,
            feedback_history_summary,
            multi_timeframe_summary: build_context.multi_timeframe_summary.to_vec(),
            raw_trade_plan: trade_plan,
            workflow_snapshot: WorkflowSnapshot::default(),
            staged_orchestration_trace,
            execution_artifact: Some(execution_artifact),
            execution_triage,
        },
    })
}

fn build_price_action_section(
    mtf: &[Candle],
    ltf: &[Candle],
    atr_ltf: &[f64],
    fvgs: &[ict_engine::types::FairValueGap],
    obs: &[ict_engine::types::OrderBlock],
) -> PriceActionSection {
    let swing_highs = find_swing_highs(mtf, 3);
    let swing_lows = find_swing_lows(mtf, 3);
    let breaks = detect_structure_breaks(mtf, &swing_highs, &swing_lows);
    let latest_break = breaks
        .last()
        .map(|brk| format!("{:?}_{:?}", brk.break_type, brk.direction));
    let recent_break_count = count_recent_breaks(&breaks, 20, mtf.len());
    let pools = detect_liquidity_pools(mtf, atr_ltf, 0.5, 2);
    let sweeps = detect_liquidity_sweep(mtf, &pools, 5);
    let liquidity_sweeps_recent = count_recent_sweeps(mtf, &sweeps, 20);
    let bullish_cisds = detect_cisd(ltf, &detect_order_blocks(ltf), 1);
    let bullish_cisd = bullish_cisds.iter().any(|cisd| {
        cisd.direction == Direction::Bull && cisd.confirm_bar >= ltf.len().saturating_sub(10)
    });
    let bearish_cisd = bullish_cisds.iter().any(|cisd| {
        cisd.direction == Direction::Bear && cisd.confirm_bar >= ltf.len().saturating_sub(10)
    });
    let bull_expansion = check_bull_expansion_exists(mtf, 20, 1.5);
    let bear_expansion = check_bear_expansion_exists(mtf, 20, 1.5);
    let structure_bias = if bull_expansion && !bear_expansion {
        Direction::Bull
    } else if bear_expansion && !bull_expansion {
        Direction::Bear
    } else {
        breaks
            .last()
            .map(|brk| brk.direction)
            .unwrap_or(Direction::Neutral)
    };
    let rejection_block_present = has_recent_pinbar(ltf, atr_ltf, 5);
    let narrative = if structure_bias == Direction::Bull {
        "bullish_price_action_with_higher_probability_if_execution_confirms".to_string()
    } else if structure_bias == Direction::Bear {
        "bearish_price_action_with_higher_probability_if_execution_confirms".to_string()
    } else {
        "mixed_price_action_no_decisive_structure_edge".to_string()
    };

    PriceActionSection {
        probability_role: "structural_evidence_for_probability_model".to_string(),
        structure_bias,
        latest_break,
        recent_break_count,
        swing_highs: swing_highs.len(),
        swing_lows: swing_lows.len(),
        bull_expansion,
        bear_expansion,
        expansion_strength: expansion_strength(mtf, 20),
        liquidity_sweeps_recent,
        open_fvgs: fvgs.len(),
        untested_order_blocks: obs.len(),
        bullish_cisd,
        bearish_cisd,
        rejection_block_present,
        narrative,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_regime_bayesian_section(
    hmm_state: &str,
    regime_probs: &RegimeProbs,
    regime_label: &str,
    liquidity_label: &str,
    decision: &ProbabilisticDecisionSnapshot,
    evidence_policy: &str,
    options_hedging: Option<&OptionsHedgingSection>,
    hybrid_regime: Option<&RegimeSegmentationPacket>,
    pda_sequence_summary: Option<&ict_engine::pda_sequence::PdaSequenceArtifactSummary>,
) -> RegimeBayesianSection {
    let mut evidence_policy = evidence_policy.to_string();
    if let Some(hedging) = options_hedging {
        if hedging.hedge_pressure_direction.is_some() {
            evidence_policy.push_str("+options_hedging_overlay");
        }
    }

    RegimeBayesianSection {
        hmm_state: hmm_state.to_string(),
        regime_probs: *regime_probs,
        regime_label: regime_label.to_string(),
        liquidity_label: liquidity_label.to_string(),
        hybrid_regime_label: hybrid_regime.and_then(|packet| packet.active_regime_cluster.clone()),
        hybrid_transition_hazard: hybrid_regime.and_then(|packet| packet.transition_hazard),
        hybrid_duration_model: hybrid_regime.and_then(|packet| packet.duration_model.clone()),
        hybrid_remaining_expected_bars: hybrid_regime
            .and_then(|packet| packet.duration_remaining_expected_bars),
        pda_cluster_family: pda_sequence_summary
            .and_then(|summary| summary.primary_cluster_family.clone()),
        pda_hybrid_alignment: hybrid_regime.and_then(|packet| {
            packet
                .evidence
                .iter()
                .find_map(|line| line.strip_prefix("pda_hybrid_alignment="))
                .map(|value| value == "true")
        }),
        long_score: decision.long_score,
        short_score: decision.short_score,
        win_prob_long: decision.win_prob_long,
        win_prob_short: decision.win_prob_short,
        selected_direction: decision.selected_direction,
        evidence_policy,
        ict_role: decision.ict_role.clone(),
    }
}

fn build_trade_plan_section(
    trade_plan: &TradePlan,
    options_hedging: Option<&OptionsHedgingSection>,
) -> TradePlanSection {
    let actionable = trade_plan.direction != Direction::Neutral && trade_plan.position_size > 0.0;
    let hedge_fragment = options_hedging
        .and_then(|hedging| hedging.hedge_pressure_direction.as_deref())
        .map(|direction| format!(";options_hedging_bias={direction}"));
    let narrative = if actionable {
        format!(
            "preferred_{:?}_entry_with_defined_risk_and_positive_position_size{}",
            trade_plan.direction,
            hedge_fragment.clone().unwrap_or_default()
        )
    } else if trade_plan.direction != Direction::Neutral {
        format!(
            "{:?}_bias_exists_but_position_size_is_zero_due_to_probability_risk_constraints{}",
            trade_plan.direction,
            hedge_fragment.unwrap_or_default()
        )
    } else {
        "no_trade_due_to_insufficient_edge".to_string()
    };

    TradePlanSection {
        probability_role: "execution_plan_derived_from_probability_model".to_string(),
        actionable,
        direction: trade_plan.direction,
        entry: trade_plan.entry,
        stop_loss: trade_plan.stop_loss,
        take_profits: vec![trade_plan.tp1, trade_plan.tp2, trade_plan.tp3],
        risk_reward: trade_plan.risk_reward,
        posterior: trade_plan.posterior,
        win_probability: trade_plan.win_probability,
        kelly_fraction: trade_plan.kelly_fraction,
        position_size: trade_plan.position_size,
        uncertainties: trade_plan.uncertainties.clone(),
        narrative,
    }
}

fn apply_duration_sizing_adjustment(
    mut trade_plan: TradePlan,
    market: &str,
    hybrid_regime: &RegimeSegmentationPacket,
) -> TradePlan {
    let Some(remaining) = hybrid_regime.duration_remaining_expected_bars else {
        return trade_plan;
    };
    let family = hybrid_regime
        .active_regime_cluster
        .as_deref()
        .map(|label| {
            if label.contains("trend") {
                "trend"
            } else if label.contains("range") {
                "range"
            } else {
                "transition"
            }
        })
        .unwrap_or("transition");
    let scale = duration_sizing_scale(market, family, remaining);
    if scale < 1.0 {
        trade_plan.kelly_fraction *= scale;
        trade_plan.position_size *= scale;
        trade_plan.uncertainties.push(format!(
            "duration_sizing_scale={scale:.2} remaining_expected_bars={remaining:.3} market={} family={}",
            market,
            family
        ));
        if scale == 0.0 {
            trade_plan
                .uncertainties
                .push("duration_window_too_short_for_execution_size_zeroed".to_string());
        }
    }
    trade_plan
}

fn duration_sizing_scale(market: &str, family: &str, remaining_expected_bars: f64) -> f64 {
    match (market.to_ascii_uppercase().as_str(), family) {
        ("NQ", "trend") | ("CL", "trend") => {
            if remaining_expected_bars <= 1.5 {
                0.0
            } else if remaining_expected_bars <= 2.5 {
                0.25
            } else if remaining_expected_bars <= 4.0 {
                0.50
            } else {
                1.0
            }
        }
        ("GC", "range") => {
            if remaining_expected_bars <= 1.0 {
                0.0
            } else if remaining_expected_bars <= 2.0 {
                0.35
            } else if remaining_expected_bars <= 3.5 {
                0.60
            } else {
                1.0
            }
        }
        _ => {
            if remaining_expected_bars <= 1.5 {
                0.0
            } else if remaining_expected_bars <= 3.0 {
                0.40
            } else if remaining_expected_bars <= 5.0 {
                0.70
            } else {
                1.0
            }
        }
    }
}

#[cfg(test)]
fn latest_duration_phase(
    snapshot: &WorkflowSnapshot,
) -> Option<&ict_engine::state::WorkflowPhaseSnapshot> {
    snapshot
        .latest_backtest
        .as_ref()
        .or(snapshot.latest_research.as_ref())
        .or(snapshot.latest_update.as_ref())
        .or(snapshot.latest_analyze.as_ref())
}

fn parse_duration_sizing_scale(summary: &[String]) -> Option<f64> {
    summary.iter().find_map(|line| {
        line.split_whitespace().find_map(|fragment| {
            fragment
                .strip_prefix("duration_sizing_scale=")
                .and_then(|value| value.parse::<f64>().ok())
        })
    })
}

#[cfg(test)]
fn build_duration_surface_from_artifacts(
    snapshot: &WorkflowSnapshot,
    artifact_action_summary: &[String],
) -> Vec<String> {
    let phase = latest_duration_phase(snapshot);
    let duration_model = phase.and_then(|phase| phase.hybrid_duration_model.as_deref());
    let remaining_expected_bars = phase.and_then(|phase| phase.hybrid_remaining_expected_bars);
    let scale = parse_duration_sizing_scale(artifact_action_summary).unwrap_or(1.0);
    ict_engine::application::backtest::build_duration_sizing_delta_surface(
        1.0,
        scale,
        1.0,
        scale,
        duration_model,
        remaining_expected_bars,
    )
}

fn apply_regime_execution_guardrail(
    mut output: ict_engine::application::orchestration::ExecutionTreeOutput,
    hybrid_regime: &RegimeSegmentationPacket,
) -> ict_engine::application::orchestration::ExecutionTreeOutput {
    let high_transition_hazard = hybrid_regime.transition_hazard.unwrap_or_default() >= 0.60;
    let pda_disagreement = hybrid_regime
        .evidence
        .iter()
        .any(|line| line == "pda_hybrid_alignment=false");
    let low_remaining_duration = hybrid_regime
        .duration_remaining_expected_bars
        .unwrap_or(f64::INFINITY)
        <= 1.5;
    let short_remaining_duration = hybrid_regime
        .duration_remaining_expected_bars
        .unwrap_or(f64::INFINITY)
        <= 2.5;
    if high_transition_hazard || pda_disagreement || low_remaining_duration {
        output.gate_status = "observe".to_string();
        output.branch = "transition_guardrail".to_string();
        output.execution_bias = "guarded".to_string();
        output.branch_probability = output.branch_probability.min(0.50);
        output.posterior_uncertainty = output.posterior_uncertainty.max(0.60);
        output.decision_hint = if low_remaining_duration {
            "execution_guarded_due_to_low_remaining_regime_duration".to_string()
        } else if pda_disagreement {
            "execution_guarded_due_to_pda_hybrid_disagreement".to_string()
        } else {
            "execution_guarded_due_to_high_transition_hazard".to_string()
        };
        output.split_reason_lineage.push(format!(
            "hybrid_transition_hazard={:.3}",
            hybrid_regime.transition_hazard.unwrap_or_default()
        ));
        if pda_disagreement {
            output
                .split_reason_lineage
                .push("pda_hybrid_alignment=false".to_string());
        }
        if low_remaining_duration {
            output.split_reason_lineage.push(format!(
                "duration_remaining_expected_bars={:.3}",
                hybrid_regime
                    .duration_remaining_expected_bars
                    .unwrap_or_default()
            ));
        }
    } else if short_remaining_duration && output.execution_bias == "aggressive" {
        output.execution_bias = "passive".to_string();
        output.split_reason_lineage.push(format!(
            "duration_remaining_expected_bars={:.3} → execution_bias=passive",
            hybrid_regime
                .duration_remaining_expected_bars
                .unwrap_or_default()
        ));
    }
    output
}

fn pre_bayes_policy_diff(
    previous: Option<&ict_engine::state::PreBayesEvidencePolicy>,
    current: &ict_engine::state::PreBayesEvidencePolicy,
) -> ict_engine::state::PreBayesPolicyDiff {
    let mut changed_fields = Vec::new();
    if let Some(previous) = previous {
        if previous.min_directional_support_gap != current.min_directional_support_gap {
            changed_fields.push("min_directional_support_gap".to_string());
        }
        if previous.high_uncertainty_threshold != current.high_uncertainty_threshold {
            changed_fields.push("high_uncertainty_threshold".to_string());
        }
        if previous.min_multi_timeframe_alignment_score
            != current.min_multi_timeframe_alignment_score
        {
            changed_fields.push("min_multi_timeframe_alignment_score".to_string());
        }
        if previous.min_multi_timeframe_entry_alignment_score
            != current.min_multi_timeframe_entry_alignment_score
        {
            changed_fields.push("min_multi_timeframe_entry_alignment_score".to_string());
        }
        if previous.hard_pass_quality_threshold != current.hard_pass_quality_threshold {
            changed_fields.push("hard_pass_quality_threshold".to_string());
        }
        if previous.neutralized_quality_threshold != current.neutralized_quality_threshold {
            changed_fields.push("neutralized_quality_threshold".to_string());
        }
        if previous.directional_conflict_penalty != current.directional_conflict_penalty {
            changed_fields.push("directional_conflict_penalty".to_string());
        }
        if previous.mixed_alignment_penalty != current.mixed_alignment_penalty {
            changed_fields.push("mixed_alignment_penalty".to_string());
        }
        if previous.multi_timeframe_direction_conflict_penalty
            != current.multi_timeframe_direction_conflict_penalty
        {
            changed_fields.push("multi_timeframe_direction_conflict_penalty".to_string());
        }
        if previous.multi_timeframe_alignment_penalty != current.multi_timeframe_alignment_penalty {
            changed_fields.push("multi_timeframe_alignment_penalty".to_string());
        }
        if previous.multi_timeframe_entry_penalty != current.multi_timeframe_entry_penalty {
            changed_fields.push("multi_timeframe_entry_penalty".to_string());
        }
        if previous.multi_timeframe_alignment_bonus != current.multi_timeframe_alignment_bonus {
            changed_fields.push("multi_timeframe_alignment_bonus".to_string());
        }
        if previous.hostile_liquidity_penalty != current.hostile_liquidity_penalty {
            changed_fields.push("hostile_liquidity_penalty".to_string());
        }
        if previous.favorable_liquidity_bonus != current.favorable_liquidity_bonus {
            changed_fields.push("favorable_liquidity_bonus".to_string());
        }
        if previous.hostile_liquidity_forces_high_uncertainty
            != current.hostile_liquidity_forces_high_uncertainty
        {
            changed_fields.push("hostile_liquidity_forces_high_uncertainty".to_string());
        }
    } else {
        changed_fields.push("initial_policy".to_string());
    }
    ict_engine::state::PreBayesPolicyDiff {
        previous_version: previous.map(|policy| policy.version.clone()),
        summary: if changed_fields.is_empty() {
            "policy_unchanged".to_string()
        } else {
            format!("changed_fields={:?}", changed_fields)
        },
        changed_fields,
    }
}

fn resolve_live_backend_base_url(
    backend: &str,
    openalice_base_url: &str,
    nofx_base_url: &str,
) -> String {
    match backend.trim().to_ascii_lowercase().as_str() {
        "openbb" => "native://openbb".to_string(),
        "openalice" => openalice_base_url.to_string(),
        "nofx" => nofx_base_url.to_string(),
        _ => "native://openbb".to_string(),
    }
}

fn selected_cascade_max_layer(plan: &TradePlan) -> CascadeLayer {
    let cascade = match plan.direction {
        Direction::Bull => &plan.cascade_bull,
        Direction::Bear => &plan.cascade_bear,
        Direction::Neutral => &plan.cascade_bull,
    };

    cascade
        .steps
        .iter()
        .rev()
        .find(|step| step.satisfied)
        .map(|step| step.layer)
        .unwrap_or(CascadeLayer::L1)
}

fn decision_factor_values(
    decision: &ProbabilisticDecisionSnapshot,
    trade_plan: &TradePlan,
    factor_diagnostics: &FactorDiagnostics,
) -> HashMap<String, f64> {
    HashMap::from([
        ("long_score".to_string(), decision.long_score),
        ("short_score".to_string(), decision.short_score),
        ("win_prob_long".to_string(), decision.win_prob_long),
        ("win_prob_short".to_string(), decision.win_prob_short),
        ("selected_score".to_string(), decision.selected_score),
        (
            "selected_win_probability".to_string(),
            decision.selected_win_probability,
        ),
        ("kelly_fraction".to_string(), trade_plan.kelly_fraction),
        ("posterior".to_string(), trade_plan.posterior),
        ("ict_support_long".to_string(), decision.ict_support_long),
        ("ict_support_short".to_string(), decision.ict_support_short),
        (
            "factor_support_long".to_string(),
            factor_diagnostics.long_support,
        ),
        (
            "factor_support_short".to_string(),
            factor_diagnostics.short_support,
        ),
        (
            "factor_uncertainty".to_string(),
            factor_diagnostics.uncertainty,
        ),
    ])
}

fn select_state_name(distribution: &[f64], node: &ict_engine::bbn::Node) -> Result<String> {
    let state_index = distribution
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
        .ok_or_else(|| anyhow!("empty state distribution for '{}'", node.id))?;

    node.state_name(state_index)
        .map(str::to_string)
        .ok_or_else(|| {
            anyhow!(
                "state index {} out of bounds for '{}'",
                state_index,
                node.id
            )
        })
}

fn load_or_init_hmm_params(symbol: &str, state_dir: &str) -> HMMParams {
    if !state_exists(state_dir, symbol, HMM_STATE_FILE) {
        return init_hmm_params(OBS_DIM);
    }

    match load_state::<HMMParams, _>(state_dir, symbol, HMM_STATE_FILE) {
        Ok(params) if hmm_params_compatible(&params) => params,
        Ok(_) => init_hmm_params(OBS_DIM),
        Err(err) => {
            eprintln!(
                "warning: failed to load HMM state for '{}' from '{}': {}",
                symbol, state_dir, err
            );
            init_hmm_params(OBS_DIM)
        }
    }
}

fn load_or_init_trading_network(
    symbol: &str,
    state_dir: &str,
) -> Result<ict_engine::bbn::BayesianNetwork> {
    if !state_exists(state_dir, symbol, BBN_STATE_FILE) {
        return build_trading_network();
    }

    match load_state::<ict_engine::bbn::BayesianNetwork, _>(state_dir, symbol, BBN_STATE_FILE) {
        Ok(mut network) => {
            upgrade_trading_network(&mut network)?;
            Ok(network)
        }
        Err(err) => {
            eprintln!(
                "warning: failed to load BBN state for '{}' from '{}': {}",
                symbol, state_dir, err
            );
            build_trading_network()
        }
    }
}

fn hmm_params_compatible(params: &HMMParams) -> bool {
    params.n_states == 3
        && params.transition.len() == params.n_states
        && params.initial_probs.len() == params.n_states
        && params.emission_means.len() == params.n_states
        && params.emission_stds.len() == params.n_states
        && params.emission_means.iter().all(|row| row.len() == OBS_DIM)
        && params.emission_stds.iter().all(|row| row.len() == OBS_DIM)
}

fn regime_probs_from_log_gamma(log_gamma: Option<&Vec<f64>>) -> Result<RegimeProbs> {
    let log_gamma = log_gamma.ok_or_else(|| anyhow!("missing HMM posterior probabilities"))?;
    if log_gamma.len() < 3 {
        bail!("expected 3 HMM states, got {}", log_gamma.len());
    }

    let accumulation = log_gamma[0].exp();
    let manipulation_expansion = log_gamma[1].exp();
    let distribution = log_gamma[2].exp();
    let sum = accumulation + manipulation_expansion + distribution;
    if sum <= f64::EPSILON {
        bail!("invalid HMM posterior: probabilities sum to zero");
    }

    Ok(RegimeProbs {
        accumulation: accumulation / sum,
        manipulation_expansion: manipulation_expansion / sum,
        distribution: distribution / sum,
    })
}

fn distribution_from_map(states: &[String], probabilities: &BTreeMap<String, f64>) -> Vec<f64> {
    states
        .iter()
        .map(|state| probabilities.get(state).copied().unwrap_or(0.0))
        .collect()
}

struct BuildAnalyzeAgentPromptsInput<'a> {
    symbol: &'a str,
    decision: &'a ProbabilisticDecisionSnapshot,
    factor_diagnostics: &'a FactorDiagnostics,
    pre_bayes_evidence_filter: &'a PreBayesEvidenceFilter,
    factor_ranking: &'a [PersistedFactorRanking],
    factor_iteration_queue: &'a [FactorIterationPrompt],
    feedback_history_summary: &'a FeedbackHistorySummary,
    trade_plan: &'a TradePlan,
    dataset_comparability: &'a DatasetComparability,
    decision_hint: &'a str,
    multi_timeframe_summary: &'a [String],
}

fn build_analyze_agent_prompts(input: BuildAnalyzeAgentPromptsInput<'_>) -> AgentPromptPack {
    let BuildAnalyzeAgentPromptsInput {
        symbol,
        decision,
        factor_diagnostics,
        pre_bayes_evidence_filter,
        factor_ranking,
        factor_iteration_queue,
        feedback_history_summary,
        trade_plan,
        dataset_comparability,
        decision_hint,
        multi_timeframe_summary,
    } = input;
    let mut pack = factor_iteration_prompt_pack(
        symbol,
        factor_ranking,
        factor_iteration_queue,
        feedback_history_summary,
    );
    pack.workflow = format!(
        "Use current market analysis plus stored factor scorecards to decide whether the present trade plan is supported, overfit, or missing evidence for {}.",
        symbol
    );
    pack.prompts.insert(
        0,
        dataset_audit_prompt(symbol, "analyze", None, 0, None, "analyze"),
    );
    pack.prompts.insert(
        1,
        AgentPrompt::new(AgentPromptInput {
            id: "pre_bayes_evidence_review".to_string(),
            stage: "pre_bayes_filter".to_string(),
            priority: "high".to_string(),
            objective: "Review whether raw regime/liquidity/factor evidence should be passed to BBN directly or neutralized first.".to_string(),
            system_prompt: "You are the pre-bayes evidence gate. Compare raw labels with filtered labels, conflicts, and evidence quality before trusting the downstream Bayesian inference.".to_string(),
            user_prompt: format!(
                "Symbol={} raw_market_regime={} raw_liquidity_context={} raw_factor_alignment={} raw_factor_uncertainty={} raw_mtf_direction={} raw_mtf_alignment={:.3} raw_mtf_entry_alignment={:.3} raw_mtf_resonance={} filtered_market_regime={} filtered_liquidity_context={} filtered_factor_alignment={} filtered_factor_uncertainty={} filtered_mtf_direction={} filtered_mtf_alignment={:.3} filtered_mtf_entry_alignment={:.3} filtered_mtf_resonance={} evidence_quality_score={:.3} gating_status={} uses_soft_evidence={} conflict_flags={:?} rationale={:?} soft_market_regime={:?} soft_liquidity_context={:?} soft_factor_alignment={:?} soft_factor_uncertainty={:?} soft_mtf_resonance={:?}",
                symbol,
                pre_bayes_evidence_filter.raw_market_regime_label,
                pre_bayes_evidence_filter.raw_liquidity_context_label,
                pre_bayes_evidence_filter.raw_factor_alignment,
                pre_bayes_evidence_filter.raw_factor_uncertainty,
                pre_bayes_evidence_filter.raw_multi_timeframe_direction_bias,
                pre_bayes_evidence_filter
                    .raw_multi_timeframe_alignment_score
                    .unwrap_or_default(),
                pre_bayes_evidence_filter
                    .raw_multi_timeframe_entry_alignment_score
                    .unwrap_or_default(),
                pre_bayes_evidence_filter.raw_multi_timeframe_resonance_label,
                pre_bayes_evidence_filter.filtered_market_regime_label,
                pre_bayes_evidence_filter.filtered_liquidity_context_label,
                pre_bayes_evidence_filter.filtered_factor_alignment,
                pre_bayes_evidence_filter.filtered_factor_uncertainty,
                pre_bayes_evidence_filter.filtered_multi_timeframe_direction_bias,
                pre_bayes_evidence_filter
                    .filtered_multi_timeframe_alignment_score
                    .unwrap_or_default(),
                pre_bayes_evidence_filter
                    .filtered_multi_timeframe_entry_alignment_score
                    .unwrap_or_default(),
                pre_bayes_evidence_filter.filtered_multi_timeframe_resonance_label,
                pre_bayes_evidence_filter.evidence_quality_score,
                pre_bayes_evidence_filter.gating_status,
                pre_bayes_evidence_filter.uses_soft_evidence,
                pre_bayes_evidence_filter.conflict_flags,
                pre_bayes_evidence_filter.rationale,
                pre_bayes_evidence_filter.soft_market_regime_distribution,
                pre_bayes_evidence_filter.soft_liquidity_context_distribution,
                pre_bayes_evidence_filter.soft_factor_alignment_distribution,
                pre_bayes_evidence_filter.soft_factor_uncertainty_distribution,
                pre_bayes_evidence_filter.soft_multi_timeframe_resonance_distribution
            ),
            success_criteria: vec![
                "State explicitly whether the filtered evidence should be trusted as hard evidence or soft evidence".to_string(),
                "If regime and factor alignment conflict, prefer neutralization over direct Bayesian commitment".to_string(),
            ],
            suggested_files: vec![
                "src/main.rs".to_string(),
                "src/bbn/trading/update.rs".to_string(),
                "src/factor_lab/engine.rs".to_string(),
            ],
        }),
    );
    if pre_bayes_evidence_filter.uses_soft_evidence {
        pack.prompts.insert(
            2,
            AgentPrompt::new(AgentPromptInput {
                id: "pre_bayes_soft_evidence_review".to_string(),
                stage: "pre_bayes_soft_evidence".to_string(),
                priority: "high".to_string(),
                objective: "Review whether soft evidence diverges materially from filtered labels before trusting BBN output.".to_string(),
                system_prompt: "You are the pre-bayes soft-evidence reviewer. Compare filtered states with soft evidence distributions and explain whether the Bayesian layer is receiving stable or ambiguous evidence.".to_string(),
                user_prompt: format!(
                    "Symbol={} filtered_assignments={:?} soft_market_regime={:?} soft_liquidity_context={:?} soft_factor_alignment={:?} soft_factor_uncertainty={:?} soft_mtf_resonance={:?}",
                    symbol,
                    pre_bayes_evidence_filter.evidence_assignments,
                    pre_bayes_evidence_filter.soft_market_regime_distribution,
                    pre_bayes_evidence_filter.soft_liquidity_context_distribution,
                    pre_bayes_evidence_filter.soft_factor_alignment_distribution,
                    pre_bayes_evidence_filter.soft_factor_uncertainty_distribution,
                    pre_bayes_evidence_filter.soft_multi_timeframe_resonance_distribution
                ),
                success_criteria: vec![
                    "Call out when the dominant soft-evidence state diverges from the filtered hard label".to_string(),
                    "If entropy is high, prefer observe-only or neutralized review over confident Bayesian commitment".to_string(),
                ],
                suggested_files: vec![
                    "src/main.rs".to_string(),
                    "src/bbn/node.rs".to_string(),
                    "src/bbn/trading/update.rs".to_string(),
                ],
            }),
        );
    }
    pack.prompts.insert(
        if pre_bayes_evidence_filter.uses_soft_evidence { 3 } else { 2 },
        AgentPrompt::new(AgentPromptInput {
            id: "analysis_market_review".to_string(),
            stage: "market_analysis".to_string(),
            priority: "high".to_string(),
            objective: "Review the current market conclusion and identify whether factor evidence supports the selected direction.".to_string(),
            system_prompt: "You are the market-review agent. Challenge the current trade direction using price-action evidence, factor diagnostics, and uncertainty. Do not change factor definitions here; decide whether the current conclusion is supported or should be downgraded.".to_string(),
            user_prompt: format!(
                "Symbol={} decision_hint={} dataset_comparability={{comparable:{}, reason:{}}} multi_timeframe_summary={:?} selected_direction={:?} selected_score={:.3} selected_win_probability={:.3} trade_direction={:?} posterior={:.3} factor_alignment={} factor_uncertainty={} long_support={:.3} short_support={:.3} uncertainty={:.3} bullish_factors={:?} bearish_factors={:?}",
                symbol,
                decision_hint,
                dataset_comparability.comparable,
                dataset_comparability.reason,
                multi_timeframe_summary,
                decision.selected_direction,
                decision.selected_score,
                decision.selected_win_probability,
                trade_plan.direction,
                trade_plan.posterior,
                factor_diagnostics.alignment_label,
                factor_diagnostics.uncertainty_label,
                factor_diagnostics.long_support,
                factor_diagnostics.short_support,
                factor_diagnostics.uncertainty,
                factor_diagnostics
                    .bullish_factors
                    .iter()
                    .take(3)
                    .map(|factor| format!("{}:{:.3}", factor.factor_name, factor.weighted_score))
                    .collect::<Vec<_>>(),
                factor_diagnostics
                    .bearish_factors
                    .iter()
                    .take(3)
                    .map(|factor| format!("{}:{:.3}", factor.factor_name, factor.weighted_score))
                    .collect::<Vec<_>>()
            ),
            success_criteria: vec![
                "Explicitly name which factors support long, which support short, and which only add uncertainty".to_string(),
                "If uncertainty is high, recommend what evidence the next agent should wait for".to_string(),
            ],
            suggested_files: vec![
                "src/main.rs".to_string(),
                "src/factor_lab/engine.rs".to_string(),
                "src/bbn/trading/topology.rs".to_string(),
            ],
        }),
    );
    pack
}

fn analyze_signal_rankings(
    signals: &[ict_engine::factor_lab::FactorSignal],
    regime: Regime,
) -> Vec<PersistedFactorRanking> {
    let mut rankings = signals
        .iter()
        .map(|signal| {
            let confidence_score = signal.confidence.clamp(0.0, 1.0);
            let signal_score = signal.regime_adjusted_score.abs().clamp(0.0, 1.0);
            let reliability_score = signal.posterior_reliability.clamp(0.0, 1.0);
            let composite_score =
                (0.45 * confidence_score + 0.35 * signal_score + 0.20 * reliability_score)
                    .clamp(0.0, 1.0);
            let mut weaknesses = Vec::new();
            if signal.direction == Direction::Neutral {
                weaknesses.push("neutral_signal".to_string());
            }
            if signal.confidence < 0.35 {
                weaknesses.push("low_live_confidence".to_string());
            }
            if signal.posterior_reliability < 0.45 {
                weaknesses.push("low_posterior_reliability".to_string());
            }

            let iteration_action = if signal.direction == Direction::Neutral || signal.confidence < 0.35
            {
                "observe"
            } else if composite_score >= 0.65 {
                "keep"
            } else {
                "tune"
            };

            PersistedFactorRanking {
                factor_name: signal.factor_name.clone(),
                regime: ict_engine::state::regime_key(regime).to_string(),
                ic: 0.0,
                ir: 0.0,
                backtest_return: 0.0,
                sharpe: 0.0,
                stability: reliability_score,
                win_rate: 0.0,
                profit_factor: 1.0,
                trade_count: 0,
                conformal_coverage_1sigma: 0.0,
                conformal_miscoverage_1sigma: 0.0,
                mean_prediction_interval_half_width: 0.0,
                worst_window_miscoverage: 0.0,
                regime_break_penalty: 0.0,
                weight: signal.weight,
                regime_scores: BTreeMap::from([(
                    ict_engine::state::regime_key(regime).to_string(),
                    signal_score,
                )]),
                composite_score,
                score_breakdown: BTreeMap::from([
                    ("current_confidence".to_string(), confidence_score),
                    ("current_signal_strength".to_string(), signal_score),
                    ("posterior_reliability".to_string(), reliability_score),
                ]),
                grade: if composite_score >= 0.85 {
                    "A".to_string()
                } else if composite_score >= 0.70 {
                    "B".to_string()
                } else if composite_score >= 0.55 {
                    "C".to_string()
                } else if composite_score >= 0.40 {
                    "D".to_string()
                } else {
                    "F".to_string()
                },
                iteration_action: iteration_action.to_string(),
                replacement_candidate: false,
                weaknesses,
                agent_prompt: format!(
                    "Analyze-phase snapshot for '{}'. direction={:?} confidence={:.2} weighted_signal={:.2}. Treat as provisional evidence and confirm with factor-research before any promotion or replacement decision.",
                    signal.factor_name,
                    signal.direction,
                    signal.confidence,
                    signal.regime_adjusted_score
                ),
            }
        })
        .collect::<Vec<_>>();
    rankings.sort_by(|a, b| {
        b.composite_score
            .partial_cmp(&a.composite_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rankings
}

struct FinalizeBacktestReportInput<'a> {
    report: BacktestReport,
    symbol: &'a str,
    data: &'a str,
    paired_data: Option<&'a str>,
    candles: &'a [Candle],
    paired_candles_slice: Option<&'a [Candle]>,
    learning_state: &'a LearningState,
    previous_rankings: &'a [PersistedFactorRanking],
    previous_trade_outcome_cpt: &'a BTreeMap<String, BTreeMap<String, f64>>,
    updated_network: &'a ict_engine::bbn::BayesianNetwork,
    state_dir: &'a str,
    warmup_bars: usize,
    hold_bars: usize,
    realism: &'a ExecutionRealismConfig,
    online_learning: bool,
}

struct BuildUpdateAgentPromptsInput<'a> {
    symbol: &'a str,
    factor_ranking: &'a [PersistedFactorRanking],
    factor_iteration_queue: &'a [FactorIterationPrompt],
    feedback_history_summary: &'a FeedbackHistorySummary,
    updated_trade_outcome: &'a BTreeMap<String, f64>,
    normalized_entry_quality: &'a str,
    factor_alignment: &'a str,
    factor_uncertainty: &'a str,
    realized_outcome: &'a str,
    feedback_records_applied: usize,
    consumed_pre_bayes_evidence_filter: Option<&'a PreBayesEvidenceFilter>,
    consumed_pre_bayes_entry_quality_bridge:
        Option<&'a ict_engine::state::PreBayesEntryQualityBridge>,
    consumed_multi_timeframe_summary: &'a [String],
}

fn build_update_agent_prompts(input: BuildUpdateAgentPromptsInput<'_>) -> AgentPromptPack {
    let BuildUpdateAgentPromptsInput {
        symbol,
        factor_ranking,
        factor_iteration_queue,
        feedback_history_summary,
        updated_trade_outcome,
        normalized_entry_quality,
        factor_alignment,
        factor_uncertainty,
        realized_outcome,
        feedback_records_applied,
        consumed_pre_bayes_evidence_filter,
        consumed_pre_bayes_entry_quality_bridge,
        consumed_multi_timeframe_summary,
    } = input;
    let mut pack = factor_iteration_prompt_pack(
        symbol,
        factor_ranking,
        factor_iteration_queue,
        feedback_history_summary,
    );
    pack.workflow = format!(
        "Use the realized update for {} to review whether the latest result should change factor weights, factor evidence interpretation, or future trade acceptance thresholds.",
        symbol
    );
    pack.prompts.insert(
        0,
        AgentPrompt::new(AgentPromptInput {
            id: "update_feedback_review".to_string(),
            stage: "feedback_update".to_string(),
            priority: "high".to_string(),
            objective: "Review the newly realized outcome and decide what the next agent iteration should target.".to_string(),
            system_prompt: "You are the realized-feedback agent. Use the updated trade_outcome distribution plus factor scorecards to decide whether the latest result strengthens confidence, exposes a factor weakness, or suggests a problem in evidence mapping.".to_string(),
            user_prompt: format!(
                "Symbol={} entry_quality={} factor_alignment={} factor_uncertainty={} realized_outcome={} feedback_records_applied={} updated_trade_outcome={:?} iteration_queue={:?}",
                symbol,
                normalized_entry_quality,
                factor_alignment,
                factor_uncertainty,
                realized_outcome,
                feedback_records_applied,
                updated_trade_outcome,
                factor_iteration_queue
            ),
            success_criteria: vec![
                "If duplicate_feedback_skipped is true, do not infer a new learning event".to_string(),
                "If factor_alignment and realized_outcome disagree repeatedly, prioritize evidence-mapping review or factor replacement".to_string(),
                "If updated_trade_outcome improves while factor scores stay weak, review BBN calibration before editing factor code".to_string(),
            ],
            suggested_files: vec![
                "src/main.rs".to_string(),
                "src/factors/weight_updater.rs".to_string(),
                "src/bbn/trading/topology.rs".to_string(),
                "src/agent/prompts.rs".to_string(),
            ],
        }),
    );
    if let Some(filter) = consumed_pre_bayes_evidence_filter {
        let bridge_diff =
            consumed_pre_bayes_entry_quality_bridge.map(pre_bayes_entry_quality_bridge_diff);
        pack.prompts.insert(
            1,
            AgentPrompt::new(AgentPromptInput {
                id: "update_consumed_pre_bayes_review".to_string(),
                stage: "feedback_update".to_string(),
                priority: "high".to_string(),
                objective: "Review the consumed analyze pre-bayes evidence against the realized outcome.".to_string(),
                system_prompt: "You are the update-pre-bayes reviewer. Compare the realized outcome with the consumed analyze pre-bayes gate, bridge, and multi-timeframe summary before deciding whether to revise factor logic, evidence mapping, or BBN calibration.".to_string(),
                user_prompt: format!(
                    "Symbol={} realized_outcome={} consumed_pre_bayes_gate_status={} consumed_pre_bayes_quality={:.3} consumed_pre_bayes_conflicts={:?} consumed_pre_bayes_filtered_assignments={:?} consumed_multi_timeframe_summary={:?} consumed_bridge_selected_entry_quality={:?} consumed_bridge_probability_gap={:.3}",
                    symbol,
                    realized_outcome,
                    filter.gating_status,
                    filter.evidence_quality_score,
                    filter.conflict_flags,
                    filter.evidence_assignments,
                    consumed_multi_timeframe_summary,
                    bridge_diff.as_ref().and_then(|diff| diff.selected_entry_quality.clone()),
                    bridge_diff
                        .as_ref()
                        .map(|diff| diff.long_short_signal_probability_gap)
                        .unwrap_or_default()
                ),
                success_criteria: vec![
                    "If the consumed pre-bayes gate was weak or soft-evidence-heavy, prefer calibration review over factor churn".to_string(),
                    "Use the consumed multi-timeframe context to judge whether the realized outcome invalidates the previous resonance mapping or only the execution result".to_string(),
                ],
                suggested_files: vec![
                    "src/main.rs".to_string(),
                    "src/bbn/trading/update.rs".to_string(),
                    "src/state/types.rs".to_string(),
                ],
            }),
        );
    }
    pack
}

fn append_artifact_decision_prompt(
    pack: &mut AgentPromptPack,
    symbol: &str,
    section: &ict_engine::state::ArtifactDecisionSection,
) {
    pack.prompts.push(AgentPrompt::new(AgentPromptInput {
        id: "artifact_decision_review".to_string(),
        stage: "artifact_decision".to_string(),
        priority: "high".to_string(),
        objective: "Review artifact-driven actions and ensure they align with the next code or model iteration.".to_string(),
        system_prompt: "You are the artifact-decision agent. Use the artifact decision section to validate whether the current pending/execution artifacts strengthen promotion, trigger rollback review, or should only be observed.".to_string(),
        user_prompt: format!(
            "Symbol={} artifact_summary={} consumed_trend_status={} consumed_trend_reason={} highlighted_actions={:?} top_factor_trends={:?} top_family_trends={:?} top_rule_break_effects={:?} top_consumed_trends={:?}",
            symbol,
            section.summary.summary,
            section.summary.consumed_trend_status,
            section.summary.consumed_trend_reason,
            section.action_summary,
            section
                .top_factor_trends
                .iter()
                .map(|trend| format!("{}:{}:{}", trend.factor_name, trend.decision_status, trend.average_quality_score))
                .collect::<Vec<_>>(),
            section
                .top_family_trends
                .iter()
                .map(|trend| format!("{}:{}:{:?}", trend.family, trend.decision_status, trend.latest_score))
                .collect::<Vec<_>>(),
            section
                .top_rule_break_effects
                .iter()
                .map(|effect| format!("{}:{}->{}:{}", effect.artifact_kind, effect.from_rule_version, effect.to_rule_version, effect.conclusion))
                .collect::<Vec<_>>(),
            section
                .top_consumed_trend_comparisons
                .iter()
                .map(|trend| format!(
                    "{}:{}:{:.2}:{:.3}",
                    trend.label,
                    trend.conclusion,
                    trend.average_quality_score_delta,
                    trend.positive_rate_delta
                ))
                .collect::<Vec<_>>()
        ),
        success_criteria: vec![
            "Explicitly state whether artifact evidence strengthens promotion, rollback review, or observation only".to_string(),
            "Do not ignore rule-break effects when artifact review versions changed".to_string(),
            "Use consumed validation trends when realized artifact outcomes are available".to_string(),
            "Name the artifact-driven factor/family targets before recommending code edits".to_string(),
        ],
        suggested_files: vec![
            "src/main.rs".to_string(),
            "src/state/types.rs".to_string(),
            "src/agent/prompts.rs".to_string(),
        ],
    }));
    if matches!(
        section.summary.consumed_trend_status.as_str(),
        "validated_improving" | "validated_regressing" | "validated_stable"
    ) {
        pack.prompts.push(AgentPrompt::new(AgentPromptInput {
            id: "artifact_consumption_review".to_string(),
            stage: "artifact_consumption".to_string(),
            priority: "high".to_string(),
            objective: "Review realized artifact consumption validation before trusting promotion or rollback conclusions.".to_string(),
            system_prompt: "You are the artifact-consumption agent. Use realized artifact outcomes, consumed validation trends, and target kinds to decide whether artifact evidence confirms promotion, forces rollback, or only warrants observation.".to_string(),
            user_prompt: format!(
                "Symbol={} consumed_trend_status={} consumed_trend_reason={} consumed_target_kinds={:?} top_consumed_trends={:?}",
                symbol,
                section.summary.consumed_trend_status,
                section.summary.consumed_trend_reason,
                section.summary.consumed_target_kinds,
                section
                    .top_consumed_trend_comparisons
                    .iter()
                    .map(|trend| format!(
                        "{}:{}:{:.2}:{:.3}",
                        trend.label,
                        trend.conclusion,
                        trend.average_quality_score_delta,
                        trend.positive_rate_delta
                    ))
                    .collect::<Vec<_>>()
            ),
            success_criteria: vec![
                "State explicitly whether consumed artifact evidence validates or invalidates recent promotion logic".to_string(),
                "If consumed validation regresses, prefer rollback review before further factor promotion".to_string(),
                "Name which artifact kinds are implicated before recommending the next iteration".to_string(),
            ],
            suggested_files: vec![
                "src/main.rs".to_string(),
                "src/state/types.rs".to_string(),
                "src/factors/weight_updater.rs".to_string(),
            ],
        }));
    }
}

fn ambiguous_bar_policy_label(policy: AmbiguousBarPolicy) -> String {
    match policy {
        AmbiguousBarPolicy::FavorStopLoss => "favor_stop_loss".to_string(),
        AmbiguousBarPolicy::FavorTakeProfit => "favor_take_profit".to_string(),
    }
}

fn parse_execution_realism_config(
    spread_bps: f64,
    slippage_bps: f64,
    fee_bps: f64,
    ambiguous_bar_policy: &str,
) -> Result<ExecutionRealismConfig> {
    if spread_bps < 0.0 || slippage_bps < 0.0 || fee_bps < 0.0 {
        bail!("spread/slippage/fee bps must be non-negative");
    }
    let ambiguous_bar_policy = match ambiguous_bar_policy.trim().to_ascii_lowercase().as_str() {
        "favor_stop_loss" | "stop" | "stop_loss" => AmbiguousBarPolicy::FavorStopLoss,
        "favor_take_profit" | "tp" | "take_profit" => AmbiguousBarPolicy::FavorTakeProfit,
        other => bail!("unsupported ambiguous_bar_policy '{}'", other),
    };
    Ok(ExecutionRealismConfig {
        spread_bps,
        slippage_bps,
        fee_bps,
        ambiguous_bar_policy,
    })
}

fn neutral_auxiliary(symbol: &str) -> AuxiliaryMarketEvidence {
    AuxiliaryMarketEvidence {
        spot_symbol: symbol.to_string(),
        options_symbol: symbol.to_string(),
        spot_kind: SpotInstrumentKind::Equity,
        spot_last_close: None,
        futures_last_close: None,
        spot_return: None,
        futures_return: None,
        raw_basis_bps: None,
        normalized_basis_bps: None,
        rolling_price_ratio_mean: None,
        put_call_oi_ratio: None,
        put_call_volume_ratio: None,
        near_atm_implied_volatility: None,
        near_atm_delta: None,
        near_atm_gamma: None,
        near_atm_vega: None,
        call_gamma_oi: None,
        put_gamma_oi: None,
        gamma_skew: None,
        hedge_pressure_direction: None,
        hedge_pressure_score: Some(0.0),
        long_bias: 0.0,
        short_bias: 0.0,
        uncertainty_penalty: 0.0,
        notes: vec!["neutral_auxiliary".to_string()],
    }
}

fn neutral_options_summary(
    symbol: &str,
) -> ict_engine::data::realtime::openalice::OptionsChainSummary {
    ict_engine::data::realtime::openalice::OptionsChainSummary {
        symbol: symbol.to_string(),
        source: Some("fallback:neutral_options_summary".to_string()),
        underlying_price: None,
        call_open_interest: 0.0,
        put_open_interest: 0.0,
        put_call_oi_ratio: None,
        call_volume: 0.0,
        put_volume: 0.0,
        put_call_volume_ratio: None,
        near_atm_implied_volatility: None,
        near_atm_delta: None,
        near_atm_gamma: None,
        near_atm_vega: None,
        call_gamma_oi: None,
        put_gamma_oi: None,
        gamma_skew: None,
        nearest_expiration_dte: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use ict_engine::analyze::multi_timeframe_parse::ParsedMultiTimeframeEvidence;
    use ict_engine::application::belief::{
        historical_market_jump_weight, jump_calibration_gate_workflow_summary,
        jump_model_workflow_summary, persist_market_jump_calibration_from_backtest_runs,
        persist_market_jump_calibration_from_research_runs, ExpansionFactorPipelineReport,
    };
    use ict_engine::application::data_sources::{
        CleanFuturesReport, CleanedCandleOutput, ExpansionMarketReport, FuturesSopMarketReport,
    };
    use ict_engine::application::factor_lifecycle::ExpansionFactorScore;
    use ict_engine::application::multi_timeframe_inputs::MULTI_TIMEFRAME_INTERVALS;
    use ict_engine::bbn::trading::topology::build_trading_network;
    use ict_engine::config::build_frame_features_for_market;
    use ict_engine::state::{
        BacktestRunRecord, FactorAutoresearchAttempt, FactorAutoresearchDecision,
        FactorAutoresearchLiveSnapshot, FactorPipelineLabelSource, ResearchRunRecord,
    };

    fn workflow_status_command(
        input: ict_engine::application::orchestration::WorkflowStatusCommandInput<'_>,
    ) -> Result<()> {
        ict_engine::application::orchestration::workflow_status_command(
            input,
            refresh_workflow_snapshot,
        )
    }

    fn pre_bayes_status_command(
        symbol: &str,
        state_dir: &str,
        refresh: bool,
        section: Option<&str>,
    ) -> Result<()> {
        ict_engine::application::orchestration::pre_bayes_status_command(
            symbol,
            state_dir,
            refresh,
            section,
            refresh_workflow_snapshot,
        )
    }

    fn sample_candles(count: usize) -> Vec<Candle> {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        (0..count)
            .map(|index| {
                let drift = index as f64 * 0.35;
                Candle {
                    timestamp: start + Duration::minutes(index as i64),
                    open: 100.0 + drift,
                    high: 100.6 + drift,
                    low: 99.4 + drift,
                    close: 100.3 + drift,
                    volume: 1_000.0 + index as f64,
                }
            })
            .collect()
    }

    fn write_test_candles(path: &std::path::Path, count: usize) {
        std::fs::write(path, serde_json::to_string(&sample_candles(count)).unwrap()).unwrap();
    }

    fn test_market_category(symbol: &str) -> Option<&'static str> {
        match symbol {
            "ES" | "NQ" | "RTY" | "YM" => Some("futures_index"),
            "GC" | "SI" | "HG" => Some("metals"),
            "CL" | "NG" | "RB" => Some("energy"),
            _ => None,
        }
    }

    fn test_market_behavior_profile(category: &str) -> &'static str {
        match category {
            "futures_index" => "index_beta_regime_sensitive",
            "metals" => "metals_defensive_liquidity_sensitive",
            "energy" => "energy_volatility_shock_sensitive",
            _ => "generic",
        }
    }

    #[test]
    fn test_forced_cluster_jump_template_persists_and_increments_cluster_cycle() {
        let mut hints = BTreeMap::new();
        hints.insert("cluster_jump_cycle".to_string(), "1".to_string());
        let current = FactorMutationSpec {
            mutation_id: "ict-structure-001".to_string(),
            base_factor: "structure_ict".to_string(),
            direction_hints: hints,
            ..FactorMutationSpec::default()
        };
        let evaluation = FactorMutationEvaluation {
            mutation_id: "ict-structure-001".to_string(),
            failure_tags: vec![
                "best_factor_composite_regressed".to_string(),
                "no_superior_mutation_found".to_string(),
            ],
            metrics_after: FactorMutationMetricSet {
                top_factor_names: vec!["structure_ict".to_string()],
                ..FactorMutationMetricSet::default()
            },
            ..FactorMutationEvaluation::default()
        };

        let forced = forced_cluster_jump_template(Some(&current), &evaluation, true).unwrap();

        assert_eq!(
            forced
                .direction_hints
                .get("cluster_jump")
                .map(String::as_str),
            Some("mss_bos_cluster")
        );
        assert_eq!(
            forced
                .direction_hints
                .get("cluster_jump_cycle")
                .map(String::as_str),
            Some("2")
        );
        assert_eq!(forced.parameter_overrides.get("lookback"), Some(&10.0));
        assert_eq!(
            forced.parameter_overrides.get("expansion_threshold"),
            Some(&1.18)
        );
    }

    #[test]
    fn test_forced_cluster_jump_template_can_cycle_across_ict_families() {
        let mut hints = BTreeMap::new();
        hints.insert("cluster_jump_cycle".to_string(), "3".to_string());
        let current = FactorMutationSpec {
            mutation_id: "ict-structure-001".to_string(),
            base_factor: "structure_ict".to_string(),
            direction_hints: hints,
            ..FactorMutationSpec::default()
        };
        let evaluation = FactorMutationEvaluation {
            mutation_id: "ict-structure-001".to_string(),
            failure_tags: vec![
                "best_factor_composite_regressed".to_string(),
                "no_superior_mutation_found".to_string(),
            ],
            metrics_after: FactorMutationMetricSet {
                top_factor_names: vec!["structure_ict".to_string()],
                ..FactorMutationMetricSet::default()
            },
            ..FactorMutationEvaluation::default()
        };

        let forced = forced_cluster_jump_template(Some(&current), &evaluation, true).unwrap();

        assert_eq!(forced.base_factor, "cross_market_smt");
        assert_eq!(
            forced
                .direction_hints
                .get("cluster_jump")
                .map(String::as_str),
            Some("smt_cluster")
        );
        assert!(forced
            .direction_hints
            .get("available_clusters")
            .unwrap()
            .contains("smt_cluster"));
        assert_eq!(forced.parameter_overrides.get("lookback"), Some(&24.0));
        assert_eq!(
            forced.parameter_overrides.get("sweep_atr_multiplier"),
            Some(&0.60)
        );
    }

    #[test]
    fn test_forced_cluster_jump_template_maps_premium_discount_cluster_parameters() {
        let mut hints = BTreeMap::new();
        hints.insert("cluster_jump_cycle".to_string(), "2".to_string());
        let current = FactorMutationSpec {
            mutation_id: "ict-structure-001".to_string(),
            base_factor: "structure_ict".to_string(),
            direction_hints: hints,
            ..FactorMutationSpec::default()
        };
        let evaluation = FactorMutationEvaluation {
            mutation_id: "ict-structure-001".to_string(),
            failure_tags: vec![
                "best_factor_composite_regressed".to_string(),
                "no_superior_mutation_found".to_string(),
            ],
            metrics_after: FactorMutationMetricSet {
                top_factor_names: vec!["structure_ict".to_string()],
                ..FactorMutationMetricSet::default()
            },
            ..FactorMutationEvaluation::default()
        };

        let forced = forced_cluster_jump_template(Some(&current), &evaluation, true).unwrap();

        assert_eq!(
            forced
                .direction_hints
                .get("cluster_jump")
                .map(String::as_str),
            Some("premium_discount_ote_cluster")
        );
        assert_eq!(forced.base_factor, "structure_ict");
        assert_eq!(forced.parameter_overrides.get("lookback"), Some(&14.0));
        assert_eq!(
            forced.parameter_overrides.get("sweep_recency_bars"),
            Some(&8.0)
        );
    }

    #[test]
    fn test_forced_cluster_jump_template_marks_nq_market_specific_fork_for_structure_ict() {
        let current = FactorMutationSpec {
            mutation_id: "ict-structure-001".to_string(),
            base_factor: "structure_ict".to_string(),
            ..FactorMutationSpec::default()
        };
        let evaluation = FactorMutationEvaluation {
            mutation_id: "ict-structure-001".to_string(),
            failure_tags: vec![
                "best_factor_composite_regressed".to_string(),
                "no_superior_mutation_found".to_string(),
            ],
            metrics_after: FactorMutationMetricSet {
                top_factor_names: vec!["structure_ict".to_string(), "trend_momentum".to_string()],
                ..FactorMutationMetricSet::default()
            },
            ..FactorMutationEvaluation::default()
        };

        let forced = forced_cluster_jump_template(Some(&current), &evaluation, true).unwrap();

        assert!(forced
            .hypothesis
            .contains("NQ_market_specific_fork_validation"));
        assert_eq!(
            forced
                .direction_hints
                .get("market_specific_fork")
                .map(String::as_str),
            Some("NQ")
        );
    }

    #[test]
    fn test_forced_cluster_jump_template_triggers_on_stagnation_failure_pair() {
        let current = FactorMutationSpec {
            mutation_id: "ict-structure-001".to_string(),
            base_factor: "structure_ict".to_string(),
            hypothesis: "base".to_string(),
            ..FactorMutationSpec::default()
        };
        let evaluation = FactorMutationEvaluation {
            mutation_id: "ict-structure-001".to_string(),
            failure_tags: vec![
                "best_factor_composite_regressed".to_string(),
                "no_superior_mutation_found".to_string(),
            ],
            metrics_after: FactorMutationMetricSet {
                top_factor_names: vec!["structure_ict".to_string()],
                ..FactorMutationMetricSet::default()
            },
            ..FactorMutationEvaluation::default()
        };

        let forced = forced_cluster_jump_template(Some(&current), &evaluation, true).unwrap();

        assert!(forced.mutation_id.ends_with(":jump"));
        assert!(forced.hypothesis.contains("Forced cluster jump"));
        assert_eq!(
            forced
                .direction_hints
                .get("cluster_jump")
                .map(String::as_str),
            Some("displacement_fvg_cluster")
        );
        assert_eq!(
            forced.direction_hints.get("next_cycle").map(String::as_str),
            Some("label_refinement_or_market_specific_fork")
        );
    }

    #[test]
    fn test_next_mutation_spec_template_uses_forced_cluster_jump_on_stagnation() {
        let current = FactorMutationSpec {
            mutation_id: "ict-structure-001".to_string(),
            base_factor: "structure_ict".to_string(),
            ..FactorMutationSpec::default()
        };
        let evaluation = FactorMutationEvaluation {
            mutation_id: "ict-structure-001".to_string(),
            failure_tags: vec![
                "best_factor_composite_regressed".to_string(),
                "no_superior_mutation_found".to_string(),
            ],
            metrics_after: FactorMutationMetricSet {
                top_factor_names: vec!["structure_ict".to_string()],
                ..FactorMutationMetricSet::default()
            },
            ..FactorMutationEvaluation::default()
        };

        let next = next_mutation_spec_template(Some(&current), &evaluation, false);

        assert!(next.mutation_id.ends_with(":jump"));
        assert!(next.hypothesis.contains("Forced cluster jump"));
    }

    #[test]
    fn test_cluster_fail_streak_threshold_advances_cycle() {
        let max_cluster_fail_streak = 2usize;
        let cluster = "mss_bos_cluster".to_string();
        let mut cluster_fail_streaks = BTreeMap::<String, usize>::new();
        cluster_fail_streaks.insert(cluster.clone(), 2);
        let mut current_spec = FactorMutationSpec {
            direction_hints: BTreeMap::from([
                ("cluster_jump".to_string(), cluster),
                ("cluster_jump_cycle".to_string(), "2".to_string()),
            ]),
            ..FactorMutationSpec::default()
        };

        if cluster_fail_streaks
            .get("mss_bos_cluster")
            .copied()
            .unwrap_or(0)
            >= max_cluster_fail_streak
        {
            if let Some(cycle) = current_spec
                .direction_hints
                .get("cluster_jump_cycle")
                .and_then(|value| value.parse::<usize>().ok())
            {
                current_spec
                    .direction_hints
                    .insert("cluster_jump_cycle".to_string(), (cycle + 1).to_string());
            }
        }

        assert_eq!(
            current_spec
                .direction_hints
                .get("cluster_jump_cycle")
                .map(String::as_str),
            Some("3")
        );
    }

    #[test]
    fn test_factor_autoresearch_status_aggregation_counts_decisions_and_failure_tags() {
        let attempts = vec![
            FactorAutoresearchAttempt {
                decision: FactorAutoresearchDecision {
                    status: "keep".to_string(),
                    ..FactorAutoresearchDecision::default()
                },
                evaluation: FactorMutationEvaluation {
                    failure_tags: vec!["bridge_gap_too_small".to_string()],
                    ..FactorMutationEvaluation::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
            FactorAutoresearchAttempt {
                decision: FactorAutoresearchDecision {
                    status: "discard".to_string(),
                    ..FactorAutoresearchDecision::default()
                },
                evaluation: FactorMutationEvaluation {
                    failure_tags: vec![
                        "bridge_gap_too_small".to_string(),
                        "pre_bayes_gate_regressed".to_string(),
                    ],
                    ..FactorMutationEvaluation::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
        ];

        let mut decision_counts = BTreeMap::<String, usize>::new();
        let mut failure_tag_counts = BTreeMap::<String, usize>::new();
        for attempt in &attempts {
            *decision_counts
                .entry(attempt.decision.status.clone())
                .or_default() += 1;
            for tag in &attempt.evaluation.failure_tags {
                *failure_tag_counts.entry(tag.clone()).or_default() += 1;
            }
        }

        assert_eq!(decision_counts.get("keep"), Some(&1));
        assert_eq!(decision_counts.get("discard"), Some(&1));
        assert_eq!(failure_tag_counts.get("bridge_gap_too_small"), Some(&2));
        assert_eq!(failure_tag_counts.get("pre_bayes_gate_regressed"), Some(&1));
    }

    #[test]
    fn test_factor_mutation_spec_loader_rejects_csv() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.csv");
        std::fs::write(&path, "a,b,c\n1,2,3\n").unwrap();

        let err = load_factor_mutation_spec(path.to_str().unwrap()).unwrap_err();

        assert!(err.to_string().contains("not CSV"));
    }

    #[test]
    fn test_factor_mutation_spec_loader_rejects_history_array_json() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history.json");
        std::fs::write(
            &path,
            serde_json::to_string(&vec![
                serde_json::json!({"mutation_id":"m1","base_factor":"structure_ict"}),
            ])
            .unwrap(),
        )
        .unwrap();

        let err = load_factor_mutation_spec(path.to_str().unwrap()).unwrap_err();

        assert!(err.to_string().contains("history array"));
    }

    #[test]
    fn test_factor_mutation_spec_loader_rejects_attempt_artifact_json() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("attempt.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({
                "session_id":"s1",
                "attempt_id":"a1",
                "evaluation": {"accepted": false}
            }))
            .unwrap(),
        )
        .unwrap();

        let err = load_factor_mutation_spec(path.to_str().unwrap()).unwrap_err();

        assert!(err.to_string().contains("run history/attempt artifact"));
    }

    #[test]
    fn test_factor_autoresearch_effective_status_logic() {
        use chrono::Duration;

        // Helper: mirrors the new status logic in factor_autoresearch_status_command
        fn effective_status(
            snapshot: Option<&FactorAutoresearchLiveSnapshot>,
            final_summary_exists: bool,
        ) -> (&'static str, bool) {
            let staleness_threshold = Duration::minutes(10);
            let snapshot_is_stale = snapshot
                .map(|s| Utc::now().signed_duration_since(s.updated_at) > staleness_threshold)
                .unwrap_or(false);
            let snapshot_says_running = snapshot.map(|s| s.status == "running").unwrap_or(false);
            let snapshot_says_completed =
                snapshot.map(|s| s.status == "completed").unwrap_or(false);

            if final_summary_exists || snapshot_says_completed {
                ("completed", false)
            } else if snapshot_says_running && snapshot_is_stale {
                ("interrupted", true)
            } else if snapshot_says_running {
                ("running", false)
            } else {
                ("unknown", false)
            }
        }

        // Case 1: final_summary_exists → completed regardless of snapshot
        let running_snap = FactorAutoresearchLiveSnapshot {
            status: "running".to_string(),
            updated_at: Utc::now() - Duration::hours(1),
            ..FactorAutoresearchLiveSnapshot::default()
        };
        let (status, interrupted) = effective_status(Some(&running_snap), true);
        assert_eq!(status, "completed");
        assert!(!interrupted);

        // Case 2: snapshot says completed, no final summary → still completed
        let completed_snap = FactorAutoresearchLiveSnapshot {
            status: "completed".to_string(),
            ..FactorAutoresearchLiveSnapshot::default()
        };
        let (status, _) = effective_status(Some(&completed_snap), false);
        assert_eq!(status, "completed");

        // Case 3: snapshot says running, stale, no final → interrupted
        let stale_running = FactorAutoresearchLiveSnapshot {
            status: "running".to_string(),
            updated_at: Utc::now() - Duration::minutes(30),
            ..FactorAutoresearchLiveSnapshot::default()
        };
        let (status, interrupted) = effective_status(Some(&stale_running), false);
        assert_eq!(status, "interrupted");
        assert!(interrupted);

        // Case 4: snapshot says running, fresh, no final → running
        let fresh_running = FactorAutoresearchLiveSnapshot {
            status: "running".to_string(),
            updated_at: Utc::now(),
            ..FactorAutoresearchLiveSnapshot::default()
        };
        let (status, interrupted) = effective_status(Some(&fresh_running), false);
        assert_eq!(status, "running");
        assert!(!interrupted);

        // Case 5: no snapshot, no final → unknown
        let (status, _) = effective_status(None, false);
        assert_eq!(status, "unknown");
    }

    #[test]
    fn test_factor_autoresearch_cluster_scoreboard_aggregates_attempts() {
        let attempts = vec![
            FactorAutoresearchAttempt {
                candidate_mutation_spec: FactorMutationSpec {
                    direction_hints: BTreeMap::from([(
                        "cluster_jump".to_string(),
                        "mss_bos_cluster".to_string(),
                    )]),
                    ..FactorMutationSpec::default()
                },
                decision: FactorAutoresearchDecision {
                    status: "discard".to_string(),
                    score_delta: -0.1,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
            FactorAutoresearchAttempt {
                candidate_mutation_spec: FactorMutationSpec {
                    direction_hints: BTreeMap::from([(
                        "cluster_jump".to_string(),
                        "mss_bos_cluster".to_string(),
                    )]),
                    ..FactorMutationSpec::default()
                },
                decision: FactorAutoresearchDecision {
                    status: "discard".to_string(),
                    score_delta: -0.05,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
        ];

        let mut cluster_scoreboard = BTreeMap::<String, (usize, f64, f64)>::new();
        for attempt in &attempts {
            let cluster = attempt
                .candidate_mutation_spec
                .direction_hints
                .get("cluster_jump")
                .cloned()
                .unwrap_or_else(|| "none".to_string());
            let entry = cluster_scoreboard
                .entry(cluster)
                .or_insert((0, 0.0, f64::MIN));
            entry.0 += 1;
            entry.1 += attempt.decision.score_delta;
            entry.2 = entry.2.max(attempt.decision.score_delta);
        }

        let entry = cluster_scoreboard.get("mss_bos_cluster").unwrap();
        assert_eq!(entry.0, 2);
        assert!((entry.1 - (-0.15)).abs() < 1e-9);
        assert!((entry.2 - (-0.05)).abs() < 1e-9);
    }

    #[test]
    fn test_factor_autoresearch_branch_summary_contains_reason_tags_and_next_focus() {
        let evaluation = FactorMutationEvaluation {
            reason: "mutation_flagged:bridge_gap_too_small".to_string(),
            failure_tags: vec![
                "bridge_gap_too_small".to_string(),
                "pre_bayes_gate_regressed".to_string(),
            ],
            recommended_mutation_directions: vec![
                "tighten confirmation".to_string(),
                "reduce broad triggers".to_string(),
            ],
            ..FactorMutationEvaluation::default()
        };

        let summary = ict_engine::application::factor_lifecycle::factor_autoresearch_branch_summary(
            &evaluation,
        );

        assert_eq!(summary[0], "reason=mutation_flagged:bridge_gap_too_small");
        assert_eq!(
            summary[1],
            "failure_tags=bridge_gap_too_small|pre_bayes_gate_regressed"
        );
        assert!(summary[2].contains("tighten confirmation"));
        assert!(summary[2].contains("reduce broad triggers"));
    }

    #[test]
    fn test_factor_autoresearch_decision_maps_acceptance_to_keep() {
        let evaluation = FactorMutationEvaluation {
            accepted: true,
            reason: "mechanical_score_improved_without_pre_bayes_regression".to_string(),
            score_before: 0.41,
            score_after: 0.52,
            score_delta: 0.11,
            ..FactorMutationEvaluation::default()
        };

        let decision =
            ict_engine::application::factor_lifecycle::factor_autoresearch_decision(&evaluation);

        assert_eq!(decision.status, "keep");
        assert!(decision.promoted_to_baseline);
        assert_eq!(decision.reason, evaluation.reason);
        assert_eq!(decision.baseline_score_before, 0.41);
        assert_eq!(decision.candidate_score, 0.52);
        assert_eq!(decision.score_delta, 0.11);
    }

    #[test]
    fn test_factor_autoresearch_resume_prefers_latest_attempt_when_no_keep_exists() {
        let initial_spec = FactorMutationSpec {
            mutation_id: "initial-spec".to_string(),
            base_factor: "structure_ict".to_string(),
            hypothesis: "initial".to_string(),
            ..FactorMutationSpec::default()
        };
        let later_spec = FactorMutationSpec {
            mutation_id: "later-spec".to_string(),
            base_factor: "structure_ict".to_string(),
            hypothesis: "later".to_string(),
            ..FactorMutationSpec::default()
        };
        let attempts = [
            FactorAutoresearchAttempt {
                candidate_mutation_spec: initial_spec.clone(),
                decision: FactorAutoresearchDecision {
                    status: "discard".to_string(),
                    promoted_to_baseline: false,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
            FactorAutoresearchAttempt {
                candidate_mutation_spec: later_spec.clone(),
                decision: FactorAutoresearchDecision {
                    status: "discard".to_string(),
                    promoted_to_baseline: false,
                    ..FactorAutoresearchDecision::default()
                },
                ..FactorAutoresearchAttempt::default()
            },
        ];

        let resumed = attempts
            .iter()
            .rev()
            .find(|attempt| attempt.decision.promoted_to_baseline)
            .map(|attempt| attempt.candidate_mutation_spec.clone())
            .or_else(|| {
                attempts
                    .last()
                    .map(|attempt| attempt.candidate_mutation_spec.clone())
            })
            .unwrap_or_else(|| initial_spec.clone());

        assert_eq!(resumed.mutation_id, "later-spec");
    }

    #[test]
    fn test_research_calibration_writeback_updates_market_jump_weights() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "CL";
        let timestamp = Utc::now();
        let runs = vec![
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.20,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.24,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.28,
                ..ResearchRunRecord::default()
            },
        ];

        let family = test_market_category(symbol).unwrap();
        persist_market_jump_calibration_from_research_runs(
            temp.path(),
            symbol,
            &runs,
            Some(family),
            Some(test_market_behavior_profile(family)),
        )
        .unwrap();
        let weight = historical_market_jump_weight(
            temp.path(),
            symbol,
            Some("energy"),
            Some("energy_volatility_shock_sensitive"),
        );

        assert!(weight > 1.20);
    }

    #[test]
    fn test_backtest_calibration_writeback_updates_market_jump_weights() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "GC";
        let timestamp = Utc::now();
        let runs = vec![
            BacktestRunRecord {
                timestamp,
                total_return: -0.12,
                ..BacktestRunRecord::default()
            },
            BacktestRunRecord {
                timestamp,
                total_return: -0.10,
                ..BacktestRunRecord::default()
            },
            BacktestRunRecord {
                timestamp,
                total_return: -0.14,
                ..BacktestRunRecord::default()
            },
        ];

        let family = test_market_category(symbol).unwrap();
        persist_market_jump_calibration_from_backtest_runs(
            temp.path(),
            symbol,
            &runs,
            Some(family),
            Some(test_market_behavior_profile(family)),
        )
        .unwrap();
        let weight = historical_market_jump_weight(
            temp.path(),
            symbol,
            Some("metals"),
            Some("metals_defensive_liquidity_sensitive"),
        );

        assert!(weight < 0.98);
    }

    #[test]
    fn test_objective_calibration_writeback_updates_market_jump_weights_and_surfaces() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "NQ";
        let timestamp = Utc::now();
        let runs = vec![
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.20,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.24,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.28,
                ..ResearchRunRecord::default()
            },
        ];

        persist_market_jump_objective_calibration_from_research_runs(
            temp.path(),
            symbol,
            &runs,
            Some("futures_index"),
            Some("expansion_manipulation"),
        )
        .unwrap();
        let objective_weight = historical_market_jump_objective_weight(
            temp.path(),
            symbol,
            Some("futures_index"),
            Some("expansion_manipulation"),
        )
        .unwrap();

        let data = temp.path().join("candles.json");
        write_test_candles(&data, 160);
        let report = run_factor_research(RunFactorResearchInput {
            symbol,
            data: data.to_str().unwrap(),
            objective: ResearchObjectiveMode::ExpansionManipulation,
            data_1m: None,
            data_5m: None,
            data_15m: None,
            data_1h: None,
            data_4h: None,
            data_1d: None,
            paired_data: None,
            mutation_spec: None,
            state_dir: temp.path().to_str().unwrap(),
        })
        .unwrap();

        assert!(objective_weight > 1.0);
        assert!(!report.objective_surfaces.is_empty());
        assert!(report.objective_surfaces.iter().all(|surface| {
            surface.get("research_objective") == Some(&"expansion_manipulation".to_string())
                && surface.contains_key("objective_jump_weight")
                && surface.contains_key("objective_market_shrink_weight")
                && surface.contains_key("objective_market_credibility_score")
        }));
    }

    #[test]
    fn test_analyze_report_surfaces_objective_jump_weight() {
        let temp = tempfile::tempdir().unwrap();
        let htf = sample_candles(220);
        let mtf = sample_candles(180);
        let ltf = sample_candles(140);
        let params = load_or_init_hmm_params("NQ", temp.path().to_str().unwrap());
        let network = load_or_init_trading_network("NQ", temp.path().to_str().unwrap()).unwrap();
        let learning_state = load_learning_state(temp.path(), "NQ").unwrap();
        let report = build_analyze_report(BuildAnalyzeReportInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            htf: &htf,
            mtf: &mtf,
            ltf: &ltf,
            params: &params,
            network: &network,
            build_context: AnalyzeBuildContext {
                symbol: "NQ",
                paired_candles: None,
                auxiliary: None,
                learning_state: &learning_state,
                multi_timeframe_summary: &[],
                native_frames: AnalyzeNativeFrames::default(),
            },
            execution_focus: true,
        })
        .unwrap();

        let expected_weight = report
            .supporting
            .canonical_belief_report
            .gate_decision
            .jump_weight;
        assert_eq!(report.supporting.objective_jump_weight, expected_weight);

        let rendered = serde_json::to_value(&report).unwrap();
        assert_eq!(
            rendered["supporting"]["objective_jump_weight"],
            serde_json::to_value(expected_weight).unwrap()
        );
    }

    #[test]
    fn test_build_analyze_report_matches_shared_shell_type() {
        let temp = tempfile::tempdir().unwrap();
        let htf = sample_candles(220);
        let mtf = sample_candles(180);
        let ltf = sample_candles(140);
        let params = load_or_init_hmm_params("NQ", temp.path().to_str().unwrap());
        let network = load_or_init_trading_network("NQ", temp.path().to_str().unwrap()).unwrap();
        let learning_state = load_learning_state(temp.path(), "NQ").unwrap();
        let report = build_analyze_report(BuildAnalyzeReportInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            htf: &htf,
            mtf: &mtf,
            ltf: &ltf,
            params: &params,
            network: &network,
            build_context: AnalyzeBuildContext {
                symbol: "NQ",
                paired_candles: None,
                auxiliary: None,
                learning_state: &learning_state,
                multi_timeframe_summary: &[],
                native_frames: AnalyzeNativeFrames::default(),
            },
            execution_focus: true,
        })
        .unwrap();

        fn assert_shared_shell_type(_: &ict_engine::analyze_report_shell::AnalyzeReport) {}

        assert_shared_shell_type(&report);
    }

    #[test]
    fn test_workflow_phase_snapshot_from_backtest_run_surfaces_objective_market_shrink() {
        let shrink = ict_engine::application::belief::objective_market_credibility_shrink(
            Some("expansion_manipulation"),
            Some("energy"),
            0.34,
        );
        let run = BacktestRunRecord {
            source_command: "backtest".to_string(),
            total_return: 0.07,
            trade_count: 12,
            conformal_coverage_1sigma: 0.68,
            regime_break_penalty: 0.11,
            structural_break_score: 0.18,
            structural_break_index: Some(21),
            recommended_next_command: "ict-engine update".to_string(),
            objective_market_credibility_shrink: Some(shrink.clone()),
            ..BacktestRunRecord::default()
        };

        let snapshot = workflow_phase_snapshot_from_backtest_run(&run);

        assert_eq!(
            snapshot
                .objective_market_credibility_shrink
                .as_ref()
                .map(|item| item.shrink_weight),
            Some(shrink.shrink_weight)
        );
        assert!(snapshot.phase_summary.contains("objective_market_shrink="));
        assert!(snapshot
            .phase_summary
            .contains("objective_market_credibility="));
    }

    #[test]
    fn test_trade_outcome_label_from_pnl() {
        assert_eq!(
            ict_engine::application::backtest::trade_outcome_label_from_pnl(0.01),
            "win"
        );
        assert_eq!(
            ict_engine::application::backtest::trade_outcome_label_from_pnl(-0.01),
            "loss"
        );
        assert_eq!(
            ict_engine::application::backtest::trade_outcome_label_from_pnl(0.0),
            "breakeven"
        );
    }

    #[test]
    fn test_main_build_feedback_record_accepts_library_input_type() {
        let timestamp = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
        let feedback = build_feedback_record(
            ict_engine::application::backtest::BuildFeedbackRecordInput {
                symbol: "NQ",
                source: "test",
                timestamp,
                factor_diagnostics: &FactorDiagnostics::default(),
                decision: &ProbabilisticDecisionSnapshot {
                    long_score: 0.6,
                    short_score: 0.3,
                    win_prob_long: 0.58,
                    win_prob_short: 0.42,
                    ict_support_long: 0.7,
                    ict_support_short: 0.3,
                    selected_direction: Direction::Bull,
                    selected_score: 0.6,
                    selected_win_probability: 0.58,
                    ict_role: "structure".to_string(),
                },
                pnl: 0.0,
                realized_outcome: "breakeven".to_string(),
                regime_at_entry: Regime::Accumulation,
            },
        );

        assert_eq!(feedback.timestamp, timestamp);
    }

    #[test]
    fn test_trade_outcome_cpt_snapshot_contains_all_entry_quality_states() {
        let network = build_trading_network().unwrap();
        let snapshot =
            ict_engine::application::backtest::trade_outcome_cpt_snapshot(&network).unwrap();

        assert!(snapshot.contains_key("high"));
        assert!(snapshot.contains_key("medium"));
        assert!(snapshot.contains_key("low"));
        assert_eq!(snapshot["high"].len(), 3);
    }

    #[test]
    fn test_build_frame_features_for_market_neutralizes_nq_hostile_sweep_bias() {
        let candles = sample_candles(140);
        let baseline = build_frame_features(&candles).unwrap();
        let nq = build_frame_features_for_market(&candles, Some("NQ")).unwrap();

        assert_eq!(nq.market.as_deref(), Some("NQ"));
        if baseline.sweep_count > baseline.fvg_count.saturating_mul(2) {
            assert_eq!(nq.regime_label, "range");
        }
        if baseline.liquidity_label == "hostile" && baseline.fvg_count > 0 {
            assert_eq!(nq.liquidity_label, "neutral");
        }
    }

    #[test]
    fn test_build_frame_features_for_market_applies_market_overrides_conditionally() {
        let candles = sample_candles(140);
        let baseline = build_frame_features(&candles).unwrap();
        let es = build_frame_features_for_market(&candles, Some("ES")).unwrap();
        let ym = build_frame_features_for_market(&candles, Some("YM")).unwrap();
        let gc = build_frame_features_for_market(&candles, Some("GC")).unwrap();
        let cl = build_frame_features_for_market(&candles, Some("CL")).unwrap();

        assert_eq!(es.market.as_deref(), Some("ES"));
        assert_eq!(ym.market.as_deref(), Some("YM"));
        assert_eq!(gc.market.as_deref(), Some("GC"));
        assert_eq!(cl.market.as_deref(), Some("CL"));
        if baseline.regime_label == "range" && baseline.fvg_count > baseline.sweep_count {
            assert_eq!(es.regime_label, "bull");
        }
        if baseline.liquidity_label == "hostile"
            && baseline.fvg_count >= baseline.sweep_count
            && baseline.fvg_count > 0
        {
            assert_eq!(es.liquidity_label, "neutral");
        }
        if baseline.regime_label == "range" && baseline.sweep_count <= baseline.fvg_count {
            assert_eq!(ym.regime_label, "bull");
        }
        if baseline.liquidity_label == "hostile" && baseline.fvg_count > 0 {
            assert_eq!(ym.liquidity_label, "neutral");
        }
        if baseline.regime_label == "range"
            && baseline.fvg_count >= baseline.sweep_count.saturating_add(1)
        {
            assert_eq!(gc.regime_label, "bull");
        }
        if baseline.liquidity_label == "favorable" && baseline.fvg_count > 0 {
            assert_eq!(gc.liquidity_label, "neutral");
        }
        if baseline.regime_label == "bear" && baseline.sweep_count > baseline.fvg_count {
            assert_eq!(cl.regime_label, "range");
        }
        if baseline.liquidity_label == "favorable" && baseline.sweep_count >= 1 {
            assert_eq!(cl.liquidity_label, "neutral");
        }
    }

    #[test]
    fn test_parse_symbol_supports_gc_and_cl() {
        assert!(matches!(parse_symbol("GC"), Symbol::GC));
        assert!(matches!(parse_symbol("CL"), Symbol::CL));
    }

    #[test]
    fn test_market_family_helpers_available_via_application_belief_api() {
        assert_eq!(
            ict_engine::application::belief::market_category_for_symbol("NQ"),
            Some("futures_index")
        );
        assert_eq!(
            ict_engine::application::belief::market_category_for_symbol("GC"),
            Some("metals")
        );
        assert_eq!(
            ict_engine::application::belief::market_behavior_profile_for_family("energy"),
            "energy_volatility_shock_sensitive"
        );
    }

    #[test]
    fn test_clean_futures_market_code_available_via_application_data_sources_api() {
        assert_eq!(
            ict_engine::application::data_sources::infer_market_code_from_path(
                "/tmp/nq future 2021-2025/glbx-mdp3-20100606-20260403.ohlcv-1m.csv"
            ),
            "NQ"
        );
    }

    #[test]
    fn test_native_frame_aggregation_helpers_available_via_application_regime_api() {
        assert_eq!(
            ict_engine::application::regime::native_frame_weight("1d"),
            0.24
        );
        assert_eq!(
            ict_engine::application::regime::weighted_majority_label(
                [("bull", 0.6), ("bear", 0.2), ("range", 0.1)],
                "bull",
                "bear",
                "range",
            ),
            "bull"
        );
        let probs = ict_engine::application::regime::weighted_regime_probs(&[
            (
                RegimeProbs {
                    accumulation: 0.7,
                    manipulation_expansion: 0.2,
                    distribution: 0.1,
                },
                0.6,
            ),
            (
                RegimeProbs {
                    accumulation: 0.2,
                    manipulation_expansion: 0.6,
                    distribution: 0.2,
                },
                0.4,
            ),
        ]);
        assert!(probs.accumulation > probs.manipulation_expansion);
    }

    #[test]
    fn test_native_frame_computations_available_via_application_regime_api() {
        let candles = sample_candles(140);
        let params = init_hmm_params(OBS_DIM);
        let signals = ict_engine::application::regime::native_frame_computations(
            &params,
            ict_engine::analyze_builder_types::AnalyzeNativeFrames {
                d1: Some(&candles),
                h4: Some(&candles),
                h1: None,
                m15: None,
                m5: None,
                m1: None,
            },
        )
        .unwrap();

        assert_eq!(signals.len(), 2);
        assert!(signals.iter().all(|signal| signal.weight > 0.0));
    }

    #[test]
    fn test_pending_update_artifact_path_uses_application_artifacts_api() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "NQ";
        let symbol_dir = temp.path().join(symbol);
        std::fs::create_dir_all(&symbol_dir).unwrap();
        let artifact_path = symbol_dir.join(PENDING_UPDATE_ARTIFACT_FILE);
        std::fs::write(&artifact_path, "{}").unwrap();

        let resolved = ict_engine::application::artifacts::pending_update_artifact_path(
            temp.path().to_str().unwrap(),
            symbol,
        );

        assert_eq!(resolved.as_deref(), artifact_path.to_str());
    }

    #[test]
    fn test_consumed_analyze_context_for_update_prefers_pending_artifact_surfaces() {
        let expected_filter = PreBayesEvidenceFilter {
            gating_status: "observe_only".to_string(),
            conflict_flags: vec!["pda_sequence_cluster_weak".to_string()],
            ..PreBayesEvidenceFilter::default()
        };
        let expected_bridge = ict_engine::state::PreBayesEntryQualityBridge {
            long_signal_probability: 0.62,
            short_signal_probability: 0.38,
            multi_timeframe_direction_bias: "aligned".to_string(),
            ..ict_engine::state::PreBayesEntryQualityBridge::default()
        };
        let pending = PendingUpdateArtifact {
            source_run_id: Some("analyze-run-1".to_string()),
            pre_bayes_evidence_filter: Some(expected_filter.clone()),
            pre_bayes_entry_quality_bridge: Some(expected_bridge.clone()),
            multi_timeframe_summary: vec!["htf=trend".to_string()],
            ..PendingUpdateArtifact::default()
        };

        let context = ict_engine::application::artifacts::consumed_analyze_context_for_update(
            std::path::Path::new("/tmp"),
            "NQ",
            Some(&pending),
            None,
        )
        .unwrap();

        assert_eq!(context.analyze_run_id.as_deref(), Some("analyze-run-1"));
        assert_eq!(
            context
                .pre_bayes_evidence_filter
                .as_ref()
                .map(|f| f.gating_status.as_str()),
            Some("observe_only")
        );
        assert_eq!(
            context
                .pre_bayes_entry_quality_bridge
                .as_ref()
                .map(|b| b.multi_timeframe_direction_bias.as_str()),
            Some("aligned")
        );
        assert_eq!(
            context.multi_timeframe_summary,
            vec!["htf=trend".to_string()]
        );
    }

    #[test]
    fn test_emit_human_report_mentions_market_family_surface() {
        let price = "能源结构偏向：空头占优，但随时防剧烈反抽。这类盘最怕突发冲击，先防假突破和急反转；原始标签=bearish_price_action。";
        let technical = "能源技术面：指标易被波动放大，先看节奏是否稳定，再看趋势是否继续；原始标签=technicals_mixed。";
        let smt = "能源联动面：相关市场常会同步放大波动，若联动发散，先减信号强度；原始标签=paired_markets_offer_mixed_confirmation。";
        let regime = format!(
            "能源品种视角：regime={} liquidity={} direction={:?}。当前更该尊重波动冲击与状态切换，先防急拉急杀再谈延续；subgraph={}",
            "bull",
            "neutral",
            Direction::Bull,
            "energy_transition_subgraph"
        );
        assert!(price.contains("能源结构偏向"));
        assert!(technical.contains("能源技术面"));
        assert!(smt.contains("能源联动面"));
        assert!(regime.contains("能源品种视角"));
        assert!(regime.contains("subgraph=energy_transition_subgraph"));
    }

    #[test]
    fn test_live_reporting_bundle_preserves_regime_companion_suffix() {
        let temp = tempfile::tempdir().unwrap();
        let htf = sample_candles(220);
        let mtf = sample_candles(180);
        let ltf = sample_candles(140);
        let params = load_or_init_hmm_params("NQ", temp.path().to_str().unwrap());
        let network = load_or_init_trading_network("NQ", temp.path().to_str().unwrap()).unwrap();
        let learning_state = load_learning_state(temp.path(), "NQ").unwrap();
        let mut report = build_analyze_report(BuildAnalyzeReportInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            htf: &htf,
            mtf: &mtf,
            ltf: &ltf,
            params: &params,
            network: &network,
            build_context: AnalyzeBuildContext {
                symbol: "NQ",
                paired_candles: None,
                auxiliary: None,
                learning_state: &learning_state,
                multi_timeframe_summary: &[],
                native_frames: AnalyzeNativeFrames::default(),
            },
            execution_focus: true,
        })
        .unwrap();
        report.analysis.regime_bayesian.hybrid_regime_label = Some("transition_watch".to_string());
        report.analysis.regime_bayesian.hybrid_transition_hazard = Some(0.73);
        report.analysis.regime_bayesian.hybrid_duration_model = Some("hsmm".to_string());
        report
            .analysis
            .regime_bayesian
            .hybrid_remaining_expected_bars = Some(1.25);
        report.analysis.regime_bayesian.pda_cluster_family = Some("displacement".to_string());
        report.analysis.regime_bayesian.pda_hybrid_alignment = Some(false);

        let bundle = ict_engine::application::reporting::build_analyze_live_reporting_bundle(
            &report,
            ict_engine::application::reporting::AnalyzeLiveReportingBundleInput {
                include_pda_sequence_summary: false,
            },
        )
        .unwrap();
        let rendered = bundle.human_report.render();

        assert!(rendered.contains("hybrid_regime=transition_watch"));
        assert!(rendered.contains("hybrid_transition_hazard=0.730"));
        assert!(rendered.contains("hybrid_duration_model=hsmm"));
        assert!(rendered.contains("hybrid_remaining_expected_bars=1.25"));
        assert!(rendered.contains("pda_family=displacement"));
        assert!(rendered.contains("pda_hybrid_alignment=false"));
        assert!(bundle.pda_sequence_summary.is_none());
    }

    #[test]
    fn test_live_inferable_defaults_cover_gc_and_cl() {
        let defaults = BTreeMap::from([
            (
                "GC".to_string(),
                BTreeMap::from([
                    ("futures_symbol".to_string(), "GC=F".to_string()),
                    ("spot_symbol".to_string(), "GLD".to_string()),
                    ("options_symbol".to_string(), "GLD".to_string()),
                    ("spot_kind".to_string(), "etf".to_string()),
                ]),
            ),
            (
                "CL".to_string(),
                BTreeMap::from([
                    ("futures_symbol".to_string(), "CL=F".to_string()),
                    ("spot_symbol".to_string(), "USO".to_string()),
                    ("options_symbol".to_string(), "USO".to_string()),
                    ("spot_kind".to_string(), "etf".to_string()),
                ]),
            ),
        ]);
        assert_eq!(defaults["GC"]["futures_symbol"], "GC=F");
        assert_eq!(defaults["CL"]["spot_symbol"], "USO");
    }

    #[test]
    fn test_analyze_live_symbol_can_infer_gc_and_cl_defaults() {
        let gc = match "GC" {
            "GC" => Some(("GC=F", "GLD", "GLD", "etf")),
            _ => None,
        }
        .unwrap();
        let cl = match "CL" {
            "CL" => Some(("CL=F", "USO", "USO", "etf")),
            _ => None,
        }
        .unwrap();
        assert_eq!(gc.0, "GC=F");
        assert_eq!(gc.1, "GLD");
        assert_eq!(cl.0, "CL=F");
        assert_eq!(cl.1, "USO");
    }

    #[test]
    fn test_pre_bayes_market_policy_overrides_apply_market_profiles() {
        let policy = pre_bayes_evidence_policy();
        let diagnostics = FactorDiagnostics {
            alignment_label: "bullish".to_string(),
            uncertainty_label: "low".to_string(),
            long_support: 0.82,
            short_support: 0.18,
            uncertainty: 0.20,
            ..FactorDiagnostics::default()
        };
        let multi_timeframe_evidence = ParsedMultiTimeframeEvidence {
            direction_bias: "bullish".to_string(),
            alignment_score: Some(0.80),
            entry_alignment_score: Some(0.78),
            ..ParsedMultiTimeframeEvidence::default()
        };

        let generic = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "hostile",
            &diagnostics,
            &multi_timeframe_evidence,
            None,
            None,
        );
        let es = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "hostile",
            &diagnostics,
            &multi_timeframe_evidence,
            Some("ES"),
            None,
        );
        let ym = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "hostile",
            &diagnostics,
            &multi_timeframe_evidence,
            Some("YM"),
            None,
        );
        let gc = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "hostile",
            &diagnostics,
            &multi_timeframe_evidence,
            Some("GC"),
            None,
        );

        assert_eq!(generic.filtered_factor_uncertainty, "high");
        assert_eq!(es.filtered_factor_uncertainty, "low");
        assert_eq!(ym.filtered_factor_uncertainty, "low");
        assert_eq!(gc.filtered_factor_uncertainty, "low");
        assert!(es.evidence_quality_score > generic.evidence_quality_score);
        assert!(ym.evidence_quality_score > generic.evidence_quality_score);
        assert!(gc.evidence_quality_score > generic.evidence_quality_score);
        assert!(es
            .rationale
            .iter()
            .any(|line| line.contains("market_policy=ES")));
        assert!(ym
            .rationale
            .iter()
            .any(|line| line.contains("market_policy=YM")));
        assert!(gc
            .rationale
            .iter()
            .any(|line| line.contains("market_policy=GC")));
    }

    #[test]
    fn test_canonical_shadow_status_defaults_to_unavailable_without_shadow() {
        let summary = None::<ict_engine::domain::belief::ShadowComparisonSummary>;
        let status = summary
            .as_ref()
            .map(|item| item.status.clone())
            .unwrap_or_else(|| "shadow=unavailable".to_string());
        assert_eq!(status, "shadow=unavailable");
    }

    #[test]
    fn test_run_factor_research_persists_rankings_and_run_record() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("candles.json");
        std::fs::write(
            &data,
            serde_json::to_string(&serde_json::json!({
                "candles": sample_candles(140)
            }))
            .unwrap(),
        )
        .unwrap();

        let report = run_factor_research(RunFactorResearchInput {
            symbol: "NQ",
            data: data.to_str().unwrap(),
            objective: ResearchObjectiveMode::Generic,
            data_1m: None,
            data_5m: None,
            data_15m: None,
            data_1h: None,
            data_4h: None,
            data_1d: None,
            paired_data: None,
            mutation_spec: None,
            state_dir: temp.path().to_str().unwrap(),
        })
        .unwrap();
        let learning_state = load_learning_state(temp.path(), "NQ").unwrap();
        let runs: Vec<ResearchRunRecord> =
            load_state(temp.path(), "NQ", ict_engine::state::RESEARCH_RUNS_FILE).unwrap();
        let snapshot: WorkflowSnapshot =
            load_state(temp.path(), "NQ", ict_engine::state::WORKFLOW_SNAPSHOT_FILE).unwrap();

        assert!(!report.backtest.scorecards.is_empty());
        assert!(!learning_state.factor_rankings.is_empty());
        assert_eq!(report.research_objective, "generic");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].research_objective, "generic");
        let ensemble: EnsembleVoteRecord =
            load_state(temp.path(), "NQ", ict_engine::state::ENSEMBLE_VOTE_FILE).unwrap();
        assert_eq!(ensemble.symbol, "NQ");
        assert_eq!(ensemble.source_phase, "factor-research");
        assert!(snapshot.latest_research.is_some());
        assert!(snapshot.latest_ensemble_vote.is_some());
        assert_eq!(snapshot.current_focus_phase, "research");
        assert!(snapshot
            .latest_research
            .as_ref()
            .unwrap()
            .phase_summary
            .contains("objective=generic"));
    }

    #[test]
    fn test_train_command_persists_train_run_and_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        for interval in MULTI_TIMEFRAME_INTERVALS {
            let dir = temp.path().join(format!("cleaned-{}", interval));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("nq.continuous-{}.json", interval)),
                serde_json::to_string(&CleanedCandleOutput {
                    symbol: "NQ".to_string(),
                    candles: sample_candles(40),
                })
                .unwrap(),
            )
            .unwrap();
        }
        let primary = temp
            .path()
            .join("cleaned-15m")
            .join("nq.continuous-15m.json");

        train_command(
            "NQ",
            primary.to_str().unwrap(),
            5,
            temp.path().to_str().unwrap(),
        )
        .unwrap();

        let runs: Vec<TrainRunRecord> =
            load_state(temp.path(), "NQ", ict_engine::state::TRAIN_RUNS_FILE).unwrap();
        let snapshot: WorkflowSnapshot =
            load_state(temp.path(), "NQ", ict_engine::state::WORKFLOW_SNAPSHOT_FILE).unwrap();

        assert_eq!(runs.len(), 1);
        assert!(runs[0].observations > 0);
        assert!(!runs[0].multi_timeframe_summary.is_empty());
        assert!(snapshot.latest_train.is_some());
    }

    #[test]
    fn test_run_factor_backtest_persists_backtest_run_and_agent_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("candles.json");
        std::fs::write(
            &data,
            serde_json::to_string(&serde_json::json!({
                "candles": sample_candles(140)
            }))
            .unwrap(),
        )
        .unwrap();

        let report = run_factor_backtest(
            "NQ",
            data.to_str().unwrap(),
            None,
            temp.path().to_str().unwrap(),
        )
        .unwrap();
        let learning_state = load_learning_state(temp.path(), "NQ").unwrap();
        let runs: Vec<BacktestRunRecord> =
            load_state(temp.path(), "NQ", ict_engine::state::BACKTEST_RUNS_FILE).unwrap();
        let snapshot: WorkflowSnapshot =
            load_state(temp.path(), "NQ", ict_engine::state::WORKFLOW_SNAPSHOT_FILE).unwrap();

        assert!(!report.factor_family_decisions.is_empty());
        assert!(!report.agent_action_plan.items.is_empty());
        assert!(!report.final_trade_outcome_cpt.is_empty());
        assert!(!learning_state.feedback_history.is_empty());
        assert_eq!(runs.len(), 1);
        assert_eq!(
            runs[0].recommended_next_command,
            report.recommended_next_command
        );
        assert!(!runs[0].agent_prompts.prompts.is_empty());
        assert!(!runs[0].agent_context_bundle.stage_views.is_empty());
        assert_eq!(runs[0].duration_sizing_scale, Some(1.0));
        assert!(runs[0].hybrid_duration_model.is_none());
        assert!(runs[0].hybrid_remaining_expected_bars.is_none());
        assert!(snapshot.latest_backtest.is_some());
        assert_eq!(snapshot.current_focus_phase, "backtest");
    }

    #[test]
    fn test_run_factor_backtest_builds_compare_report_from_persisted_runs() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("candles.json");
        std::fs::write(
            &data,
            serde_json::to_string(&serde_json::json!({
                "candles": sample_candles(140)
            }))
            .unwrap(),
        )
        .unwrap();

        run_factor_backtest(
            "NQ",
            data.to_str().unwrap(),
            None,
            temp.path().to_str().unwrap(),
        )
        .unwrap();
        run_factor_backtest(
            "NQ",
            data.to_str().unwrap(),
            None,
            temp.path().to_str().unwrap(),
        )
        .unwrap();

        let runs: Vec<BacktestRunRecord> =
            load_state(temp.path(), "NQ", ict_engine::state::BACKTEST_RUNS_FILE).unwrap();
        let (current, previous) = runs.split_last().expect("missing current run");
        let compare = ict_engine::application::backtest::build_backtest_compare_report(
            previous.last().expect("missing previous run"),
            current,
        )
        .expect("missing compare report");

        assert!(compare.summary.contains("same_data_same_config"));
        assert!(!compare.duration_sizing_delta_surface.is_empty());
        assert!(compare
            .duration_sizing_delta_surface
            .iter()
            .any(|line| line.starts_with("duration_sizing_direction=")));
    }

    #[test]
    fn test_run_probabilistic_backtest_matches_shared_shell_type() {
        let temp = tempfile::tempdir().unwrap();
        let candles = sample_candles(140);
        let params = load_or_init_hmm_params("NQ", temp.path().to_str().unwrap());
        let network = load_or_init_trading_network("NQ", temp.path().to_str().unwrap()).unwrap();
        let mut learning_state = load_learning_state(temp.path(), "NQ").unwrap();
        let realism = ExecutionRealismConfig::default();

        let (report, _, _) = run_probabilistic_backtest(RunProbabilisticBacktestInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            candles: &candles,
            paired_candles: None,
            warmup_bars: 50,
            hold_bars: 8,
            realism: &realism,
            online_learn: false,
            params: &params,
            network: &network,
            learning_state: &mut learning_state,
        })
        .unwrap();

        fn assert_shared_shell_type(_: &ict_engine::backtest_report_shell::BacktestReport) {}

        assert_shared_shell_type(&report);
    }

    #[test]
    fn test_build_runtime_backtest_report_matches_shared_shell_type() {
        let temp = tempfile::tempdir().unwrap();
        let candles = sample_candles(140);
        let network = load_or_init_trading_network("NQ", temp.path().to_str().unwrap()).unwrap();
        let learning_state = load_learning_state(temp.path(), "NQ").unwrap();
        let trades = vec![TradeRecord {
            timestamp: candles[80].timestamp,
            symbol: Symbol::NQ,
            direction: Direction::Bull,
            entry_price: 120.0,
            exit_price: 121.2,
            pnl: 0.01,
            exit_reason: Some("take_profit".to_string()),
            regime_at_entry: Regime::ManipulationExpansion,
            cascade_max_layer: CascadeLayer::L1,
            cascade_direction: Direction::Bull,
            factor_values: HashMap::from([
                ("long_score".to_string(), 0.72),
                ("short_score".to_string(), 0.18),
                ("win_prob_long".to_string(), 0.64),
                ("win_prob_short".to_string(), 0.36),
            ]),
        }];

        let report = ict_engine::application::backtest::build_runtime_backtest_report(
            ict_engine::application::backtest::BuildRuntimeBacktestReportInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                bars: candles.len(),
                warmup_bars: 50,
                hold_bars: 8,
                spread_bps: 1.0,
                slippage_bps: 1.5,
                fee_bps: 0.5,
                ambiguous_bar_policy: "skip".to_string(),
                online_learning: false,
                learning_updates: 0,
                signals: 1,
                trades: &trades,
                learning_state: &learning_state,
                network: &network,
                last_decision: None,
            },
        )
        .unwrap();

        fn assert_shared_shell_type(_: &ict_engine::backtest_report_shell::BacktestReport) {}

        assert_shared_shell_type(&report);
        assert_eq!(report.trades, 1);
        assert_eq!(report.recent_trades.len(), 1);
        assert_eq!(report.metrics.win_rate, 1.0);
        assert_eq!(
            report.recent_trades[0].ict_role,
            "evidence_only_non_deterministic"
        );
    }

    #[test]
    fn test_persist_finalized_backtest_run_appends_run_record() {
        let temp = tempfile::tempdir().unwrap();
        let report = BacktestReport {
            symbol: "NQ".to_string(),
            state_dir: temp.path().to_str().unwrap().to_string(),
            provenance: RunProvenance::default(),
            decision_thresholds: DecisionThresholds::default(),
            dataset_comparability: DatasetComparability::default(),
            promotion_decision: PromotionDecision::default(),
            rollback_recommendation: RollbackRecommendation::default(),
            bars: 140,
            warmup_bars: 50,
            hold_bars: 8,
            spread_bps: 1.0,
            slippage_bps: 1.0,
            fee_bps: 1.0,
            ambiguous_bar_policy: "skip".to_string(),
            window_mode: "rolling".to_string(),
            evidence_policy: "default".to_string(),
            ict_role: "test".to_string(),
            online_learning: false,
            learning_updates: 0,
            signals: 1,
            trades: 1,
            metrics: BacktestMetricsSummary {
                total_return: 0.02,
                sharpe: 1.1,
                max_drawdown: 0.1,
                win_rate: 1.0,
                profit_factor: 1.5,
                conformal_coverage_1sigma: 0.0,
                conformal_miscoverage_1sigma: 0.0,
                mean_prediction_interval_half_width: 0.0,
                worst_window_miscoverage: 0.0,
                regime_break_penalty: 0.0,
                structural_break_score: 0.0,
                structural_break_index: None,
                structural_break_detected: false,
                signal_structural_break_score: 0.0,
                signal_structural_break_index: None,
                signal_structural_break_detected: false,
                residual_structural_break_score: 0.0,
                residual_structural_break_index: None,
                residual_structural_break_detected: false,
                rolling_ic_structural_break_score: 0.0,
                rolling_ic_structural_break_index: None,
                rolling_ic_structural_break_detected: false,
            },
            equity_curve: vec![1.0, 1.02],
            regime_metrics: vec![],
            factor_ranking: vec![],
            factor_score_deltas: vec![],
            trade_outcome_deltas: vec![],
            factor_iteration_queue: vec![],
            factor_family_decisions: vec![],
            factor_family_outcomes: vec![],
            factor_family_diffs: vec![],
            factor_family_history: vec![],
            decision_history_summary: DecisionHistorySummary::default(),
            agent_action_plan: AgentActionPlan::default(),
            workflow_state: WorkflowState::default(),
            agent_context_bundle: AgentContextBundle::default(),
            agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
            recommended_commands: CommandRecommendations::default(),
            recommended_next_command: "ict-engine factor-research".to_string(),
            artifact_action_summary: vec!["duration_sizing_scale=0.50".to_string()],
            artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary::default(),
            artifact_decision_section: ict_engine::state::ArtifactDecisionSection::default(),
            agent_prompts: AgentPromptPack::default(),
            feedback_history_summary: FeedbackHistorySummary::default(),
            multi_timeframe_summary: vec![],
            last_decision: None,
            final_trade_outcome_cpt: BTreeMap::new(),
            recent_trades: vec![],
            workflow_snapshot: WorkflowSnapshot::default(),
            objective_market_credibility_shrink: None,
        };

        let runs = ict_engine::application::backtest::persist_finalized_backtest_run(
            ict_engine::application::backtest::PersistFinalizedBacktestRunInput {
                report: &report,
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                data: "candles.json",
                paired_data: None,
                candles: 140,
                paired_candles: None,
                warmup_bars: 50,
                hold_bars: 8,
                online_learning: false,
            },
        )
        .unwrap();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].trade_count, 1);
        assert_eq!(runs[0].duration_sizing_scale, Some(0.5));
    }

    #[test]
    fn test_apply_finalize_backtest_enrichment_populates_report_fields() {
        let mut report = BacktestReport {
            symbol: "NQ".to_string(),
            state_dir: "state".to_string(),
            provenance: RunProvenance::default(),
            decision_thresholds: DecisionThresholds::default(),
            dataset_comparability: DatasetComparability::default(),
            promotion_decision: PromotionDecision::default(),
            rollback_recommendation: RollbackRecommendation::default(),
            bars: 140,
            warmup_bars: 50,
            hold_bars: 8,
            spread_bps: 1.0,
            slippage_bps: 1.0,
            fee_bps: 1.0,
            ambiguous_bar_policy: "skip".to_string(),
            window_mode: "rolling".to_string(),
            evidence_policy: "default".to_string(),
            ict_role: "test".to_string(),
            online_learning: false,
            learning_updates: 0,
            signals: 1,
            trades: 1,
            metrics: BacktestMetricsSummary {
                total_return: 0.02,
                sharpe: 1.1,
                max_drawdown: 0.1,
                win_rate: 1.0,
                profit_factor: 1.5,
                conformal_coverage_1sigma: 0.0,
                conformal_miscoverage_1sigma: 0.0,
                mean_prediction_interval_half_width: 0.0,
                worst_window_miscoverage: 0.0,
                regime_break_penalty: 0.0,
                structural_break_score: 0.0,
                structural_break_index: None,
                structural_break_detected: false,
                signal_structural_break_score: 0.0,
                signal_structural_break_index: None,
                signal_structural_break_detected: false,
                residual_structural_break_score: 0.0,
                residual_structural_break_index: None,
                residual_structural_break_detected: false,
                rolling_ic_structural_break_score: 0.0,
                rolling_ic_structural_break_index: None,
                rolling_ic_structural_break_detected: false,
            },
            equity_curve: vec![1.0, 1.02],
            regime_metrics: vec![],
            factor_ranking: vec![],
            factor_score_deltas: vec![],
            trade_outcome_deltas: vec![],
            factor_iteration_queue: vec![],
            factor_family_decisions: vec![],
            factor_family_outcomes: vec![],
            factor_family_diffs: vec![],
            factor_family_history: vec![],
            decision_history_summary: DecisionHistorySummary::default(),
            agent_action_plan: AgentActionPlan::default(),
            workflow_state: WorkflowState::default(),
            agent_context_bundle: AgentContextBundle::default(),
            agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
            recommended_commands: CommandRecommendations::default(),
            recommended_next_command: "recommended_command_unavailable".to_string(),
            artifact_action_summary: vec![],
            artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary::default(),
            artifact_decision_section: ict_engine::state::ArtifactDecisionSection::default(),
            agent_prompts: AgentPromptPack::default(),
            feedback_history_summary: FeedbackHistorySummary::default(),
            multi_timeframe_summary: vec![],
            last_decision: None,
            final_trade_outcome_cpt: BTreeMap::new(),
            recent_trades: vec![],
            workflow_snapshot: WorkflowSnapshot::default(),
            objective_market_credibility_shrink: None,
        };
        let score_deltas = vec![RankingDiffItem {
            factor_name: "structure_ict".to_string(),
            previous_score: Some(0.4),
            new_score: 0.6,
            score_delta: 0.2,
            previous_weight: Some(0.3),
            new_weight: 0.4,
            weight_delta: 0.1,
            previous_action: Some("tune".to_string()),
            new_action: "keep".to_string(),
        }];
        let probability_deltas = vec![ProbabilityDiff {
            state: "high:win".to_string(),
            previous: Some(0.5),
            new: 0.6,
            delta: 0.1,
        }];
        let final_trade_outcome_cpt = BTreeMap::from([(
            "high".to_string(),
            BTreeMap::from([("win".to_string(), 0.6)]),
        )]);
        let promotion_decision = PromotionDecision {
            approved: true,
            reason: "improved".to_string(),
            ..PromotionDecision::default()
        };
        let rollback_recommendation = RollbackRecommendation {
            should_rollback: false,
            reason: "stable".to_string(),
            ..RollbackRecommendation::default()
        };

        ict_engine::application::backtest::apply_finalize_backtest_enrichment(
            ict_engine::application::backtest::FinalizeBacktestEnrichmentInput {
                report: &mut report,
                decision_thresholds: DecisionThresholds::default(),
                dataset_comparability: DatasetComparability {
                    comparable: true,
                    ..DatasetComparability::default()
                },
                promotion_decision,
                rollback_recommendation,
                factor_family_outcomes: vec![],
                factor_family_diffs: vec![],
                factor_family_history: vec![],
                decision_history_summary: DecisionHistorySummary::default(),
                agent_action_plan: AgentActionPlan::default(),
                workflow_state: WorkflowState::default(),
                artifact_action_summary: vec!["duration_sizing_scale=0.50".to_string()],
                artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary::default(),
                artifact_decision_section: ict_engine::state::ArtifactDecisionSection::default(),
                recommended_commands: CommandRecommendations::default(),
                recommended_next_command: "ict-engine factor-research".to_string(),
                agent_context_bundle: AgentContextBundle::default(),
                agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
                score_deltas: score_deltas.clone(),
                probability_deltas: probability_deltas.clone(),
                final_trade_outcome_cpt: final_trade_outcome_cpt.clone(),
                dataset_audit_prompt: dataset_audit_prompt(
                    "NQ",
                    "candles.json",
                    None,
                    140,
                    None,
                    "backtest",
                ),
                promotion_gate_prompt: promotion_gate_prompt(
                    "NQ",
                    &[],
                    &score_deltas,
                    &DecisionThresholds::default(),
                ),
                rollback_review_prompt: rollback_review_prompt(
                    "NQ",
                    &score_deltas,
                    &probability_deltas,
                    &DecisionThresholds::default(),
                ),
            },
        );

        assert_eq!(
            report.recommended_next_command,
            "ict-engine factor-research"
        );
        assert_eq!(report.factor_score_deltas.len(), 1);
        assert_eq!(report.factor_score_deltas[0].factor_name, "structure_ict");
        assert_eq!(report.trade_outcome_deltas.len(), 1);
        assert_eq!(report.trade_outcome_deltas[0].state, "high:win");
        assert_eq!(report.final_trade_outcome_cpt, final_trade_outcome_cpt);
        assert_eq!(report.agent_prompts.prompts.len(), 3);
        assert_eq!(report.agent_prompts.prompts[0].id, "dataset_audit");
    }

    #[test]
    fn test_run_factor_research_builds_compare_report_from_persisted_runs() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("candles.json");
        std::fs::write(
            &data,
            serde_json::to_string(&serde_json::json!({
                "candles": sample_candles(140)
            }))
            .unwrap(),
        )
        .unwrap();

        run_factor_research(RunFactorResearchInput {
            symbol: "NQ",
            data: data.to_str().unwrap(),
            objective: ResearchObjectiveMode::Generic,
            data_1m: None,
            data_5m: None,
            data_15m: None,
            data_1h: None,
            data_4h: None,
            data_1d: None,
            paired_data: None,
            mutation_spec: None,
            state_dir: temp.path().to_str().unwrap(),
        })
        .unwrap();
        run_factor_research(RunFactorResearchInput {
            symbol: "NQ",
            data: data.to_str().unwrap(),
            objective: ResearchObjectiveMode::Generic,
            data_1m: None,
            data_5m: None,
            data_15m: None,
            data_1h: None,
            data_4h: None,
            data_1d: None,
            paired_data: None,
            mutation_spec: None,
            state_dir: temp.path().to_str().unwrap(),
        })
        .unwrap();

        let runs: Vec<ResearchRunRecord> =
            load_state(temp.path(), "NQ", ict_engine::state::RESEARCH_RUNS_FILE).unwrap();
        let (current, previous) = runs.split_last().expect("missing current run");
        let compare = ict_engine::application::backtest::build_research_compare_report(
            previous.last().expect("missing previous run"),
            current,
        )
        .expect("missing compare report");

        assert!(compare.summary.contains("same_data_same_config"));
        assert!(!compare.duration_sizing_delta_surface.is_empty());
        assert!(compare
            .duration_sizing_delta_surface
            .iter()
            .any(|line| line.starts_with("duration_sizing_direction=")));
    }

    #[test]
    fn test_analyze_command_persists_analyze_run() {
        let temp = tempfile::tempdir().unwrap();
        let htf = temp.path().join("htf.json");
        let mtf = temp.path().join("mtf.json");
        let ltf = temp.path().join("ltf.json");

        for (path, count) in [(&htf, 220usize), (&mtf, 180usize), (&ltf, 140usize)] {
            std::fs::write(
                path,
                serde_json::to_string(&serde_json::json!({
                    "candles": sample_candles(count)
                }))
                .unwrap(),
            )
            .unwrap();
        }

        analyze_command(
            "NQ",
            htf.to_str().unwrap(),
            mtf.to_str().unwrap(),
            ltf.to_str().unwrap(),
            temp.path().to_str().unwrap(),
            OutputFormat::Json,
            false,
            true,
        )
        .unwrap();

        let runs: Vec<AnalyzeRunRecord> =
            load_state(temp.path(), "NQ", ict_engine::state::ANALYZE_RUNS_FILE).unwrap();
        let snapshot: WorkflowSnapshot =
            load_state(temp.path(), "NQ", ict_engine::state::WORKFLOW_SNAPSHOT_FILE).unwrap();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].source_command, "analyze");
        assert!(!runs[0].recommended_next_command.is_empty());
        assert_eq!(runs[0].promotion_decision.status, "observe");
        assert_eq!(runs[0].rollback_recommendation.scope, "none");
        assert!(!runs[0].factor_family_decisions.is_empty());
        assert!(!runs[0].agent_prompts.prompts.is_empty());
        assert!(!runs[0].agent_context_bundle.stage_views.is_empty());
        let ensemble: EnsembleVoteRecord =
            load_state(temp.path(), "NQ", ict_engine::state::ENSEMBLE_VOTE_FILE).unwrap();
        assert_eq!(ensemble.symbol, "NQ");
        assert_eq!(ensemble.source_phase, "analyze");
        assert!(snapshot.latest_analyze.is_some());
        assert!(snapshot.latest_ensemble_vote.is_some());
        assert_eq!(snapshot.current_focus_phase, "analyze");
    }

    #[test]
    fn test_format_executor_summary_lines_clones_executor_summaries() {
        let lines = format_executor_summary_lines(&[
            "executor=catboost_file action=observe confidence=0.55 weight=0.55".to_string(),
            "executor=xgboost_file action=hold confidence=0.45 weight=0.45".to_string(),
        ]);

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("executor=catboost_file"));
        assert!(lines[1].contains("executor=xgboost_file"));
    }

    #[test]
    fn test_emit_analyze_output_includes_executor_scorecard_summary() {
        let temp = tempfile::tempdir().unwrap();
        let htf = sample_candles(220);
        let mtf = sample_candles(180);
        let ltf = sample_candles(140);
        let params = load_or_init_hmm_params("NQ", temp.path().to_str().unwrap());
        let network = load_or_init_trading_network("NQ", temp.path().to_str().unwrap()).unwrap();
        let learning_state = load_learning_state(temp.path(), "NQ").unwrap();
        let report = build_analyze_report(BuildAnalyzeReportInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            htf: &htf,
            mtf: &mtf,
            ltf: &ltf,
            params: &params,
            network: &network,
            build_context: AnalyzeBuildContext {
                symbol: "NQ",
                paired_candles: None,
                auxiliary: None,
                learning_state: &learning_state,
                multi_timeframe_summary: &[],
                native_frames: AnalyzeNativeFrames::default(),
            },
            execution_focus: true,
        })
        .unwrap();

        let ensemble_vote = build_stub_ensemble_vote_from_input(&AnalyzeEnsembleVoteInput {
            symbol: report.symbol.clone(),
            state_dir: None,
            hard_blocked: true,
            hard_block_reason: Some("pre-bayes gate still blocks downstream chain".to_string()),
            hard_block_command: Some(
                "ict-engine pre-bayes-status --symbol NQ --state-dir state".to_string(),
            ),
            recommended_next_command: report.supporting.recommended_next_command.clone(),
            provenance: report.supporting.provenance.clone(),
            dataset_comparability: report.supporting.dataset_comparability.clone(),
            pre_bayes_filter: Some(report.supporting.pre_bayes_evidence_filter.clone()),
            belief: report.supporting.canonical_belief_report.clone(),
            ict_structure: None,
        });
        let summary = format_executor_summary_lines(&ensemble_vote.executor_summaries);
        assert!(ensemble_vote
            .human_next_triage
            .contains("hard_blocked=true"));
        assert!(ensemble_vote
            .human_next_triage
            .contains("hard_block_reason=pre-bayes gate still blocks downstream chain"));
        assert_eq!(
            ensemble_vote.recommended_command,
            "ict-engine pre-bayes-status --symbol NQ --state-dir state"
        );

        assert!(!summary.is_empty());
        assert!(summary[0].contains("executor=catboost_file"));
    }

    #[test]
    fn test_factor_research_output_summary_uses_executor_summaries() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("candles.json");
        std::fs::write(
            &data,
            serde_json::to_string(&serde_json::json!({
                "candles": sample_candles(140)
            }))
            .unwrap(),
        )
        .unwrap();

        let report = run_factor_research(RunFactorResearchInput {
            symbol: "NQ",
            data: data.to_str().unwrap(),
            objective: ResearchObjectiveMode::Generic,
            data_1m: None,
            data_5m: None,
            data_15m: None,
            data_1h: None,
            data_4h: None,
            data_1d: None,
            paired_data: None,
            mutation_spec: None,
            state_dir: temp.path().to_str().unwrap(),
        })
        .unwrap();
        let ensemble_vote = build_stub_ensemble_vote_from_research(&report);
        let summary = format_executor_summary_lines(&ensemble_vote.executor_summaries);

        assert!(!summary.is_empty());
        assert!(summary
            .iter()
            .any(|line| line.contains("executor=catboost") || line.contains("executor=xgboost")));
    }

    #[test]
    fn test_analyze_command_persists_pending_update_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let htf = temp.path().join("htf.json");
        let mtf = temp.path().join("mtf.json");
        let ltf = temp.path().join("ltf.json");

        for (path, count) in [(&htf, 220usize), (&mtf, 180usize), (&ltf, 140usize)] {
            std::fs::write(
                path,
                serde_json::to_string(&serde_json::json!({
                    "candles": sample_candles(count)
                }))
                .unwrap(),
            )
            .unwrap();
        }

        analyze_command(
            "NQ",
            htf.to_str().unwrap(),
            mtf.to_str().unwrap(),
            ltf.to_str().unwrap(),
            temp.path().to_str().unwrap(),
            OutputFormat::Json,
            false,
            true,
        )
        .unwrap();

        let artifact: PendingUpdateArtifact = load_state(
            temp.path(),
            "NQ",
            ict_engine::state::PENDING_UPDATE_ARTIFACT_FILE,
        )
        .unwrap();
        assert_eq!(artifact.symbol, "NQ");
        assert_eq!(artifact.source_phase, "analyze");
        assert_eq!(artifact.template_feedback.realized_outcome, "pending");
        assert!(!artifact.template_feedback.factors_used.is_empty());
        assert!(artifact.pre_bayes_evidence_filter.is_some());
        assert!(artifact.pre_bayes_entry_quality_bridge.is_some());
        assert!(!artifact.multi_timeframe_summary.is_empty());
        assert_eq!(artifact.version, 1);
        assert_eq!(artifact.review_decision.status, "promote_latest");
    }

    #[test]
    fn test_pending_update_artifact_history_versions_increment() {
        let temp = tempfile::tempdir().unwrap();
        let htf = temp.path().join("htf.json");
        let mtf = temp.path().join("mtf.json");
        let ltf = temp.path().join("ltf.json");

        for (path, count) in [(&htf, 220usize), (&mtf, 180usize), (&ltf, 140usize)] {
            std::fs::write(
                path,
                serde_json::to_string(&serde_json::json!({
                    "candles": sample_candles(count)
                }))
                .unwrap(),
            )
            .unwrap();
        }

        analyze_command(
            "NQ",
            htf.to_str().unwrap(),
            mtf.to_str().unwrap(),
            ltf.to_str().unwrap(),
            temp.path().to_str().unwrap(),
            OutputFormat::Json,
            false,
            true,
        )
        .unwrap();
        analyze_command(
            "NQ",
            htf.to_str().unwrap(),
            mtf.to_str().unwrap(),
            ltf.to_str().unwrap(),
            temp.path().to_str().unwrap(),
            OutputFormat::Json,
            false,
            true,
        )
        .unwrap();

        let history = load_pending_update_history(temp.path(), "NQ").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 1);
        assert_eq!(history[1].version, 2);
        assert_eq!(history[1].review_decision.status, "discard");
        assert!(history[1].diff_from_previous.comparable_same_data);
        assert!(history[1].diff_from_previous.comparable_same_factor_version);
    }

    #[test]
    fn test_analyze_command_persists_execution_candidate_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let htf = temp.path().join("htf.json");
        let mtf = temp.path().join("mtf.json");
        let ltf = temp.path().join("ltf.json");

        for (path, count) in [(&htf, 220usize), (&mtf, 180usize), (&ltf, 140usize)] {
            std::fs::write(
                path,
                serde_json::to_string(&serde_json::json!({
                    "candles": sample_candles(count)
                }))
                .unwrap(),
            )
            .unwrap();
        }

        analyze_command(
            "NQ",
            htf.to_str().unwrap(),
            mtf.to_str().unwrap(),
            ltf.to_str().unwrap(),
            temp.path().to_str().unwrap(),
            OutputFormat::Json,
            false,
            true,
        )
        .unwrap();

        let candidate: ExecutionCandidateArtifact = load_state(
            temp.path(),
            "NQ",
            ict_engine::state::EXECUTION_CANDIDATE_FILE,
        )
        .unwrap();
        assert_eq!(candidate.version, 1);
        assert!(!candidate.candidate_status.is_empty());
        assert!(candidate.pre_bayes_evidence_filter.is_some());
        assert!(candidate.pre_bayes_entry_quality_bridge.is_some());
        assert!(!candidate.multi_timeframe_summary.is_empty());
        let snapshot: WorkflowSnapshot =
            load_state(temp.path(), "NQ", ict_engine::state::WORKFLOW_SNAPSHOT_FILE).unwrap();
        assert!(snapshot.latest_execution_candidate.is_some());
    }

    #[test]
    fn test_workflow_status_command_reads_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let snapshot = WorkflowSnapshot {
            symbol: "NQ".to_string(),
            current_focus_phase: "research".to_string(),
            recommended_next_command:
                "ict-engine factor-research --symbol NQ --data ltf.json --state-dir state"
                    .to_string(),
            ..WorkflowSnapshot::default()
        };
        save_workflow_snapshot(temp.path(), "NQ", &snapshot).unwrap();

        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: None,
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        let loaded = ict_engine::state::load_workflow_snapshot(temp.path(), "NQ").unwrap();

        assert_eq!(loaded.current_focus_phase, "research");
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("diffs"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("execution-candidate-history"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("ensemble-vote"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("ensemble-vote-history"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("ensemble-scorecards"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: None,
                actionable_only: true,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: None,
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: true,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("artifact-history-summary"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("artifact-review-rules"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: None,
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: true,
                hard_block_reason: None,
                limit: Some(5),
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("agent-bootstrap"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
    }

    #[test]
    fn test_workflow_snapshot_contains_actionable_and_promotable_artifacts() {
        let pending = PendingUpdateArtifact {
            artifact_id: "pending-1".to_string(),
            version: 1,
            generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("analyze:1".to_string()),
            decision_hint: "hint".to_string(),
            review_decision: PendingUpdateArtifactDecision {
                status: "promote_latest".to_string(),
                reason: "strict_probability_and_score_improvement".to_string(),
                supersedes_artifact_id: None,
            },
            ..PendingUpdateArtifact::default()
        };
        let execution = ExecutionCandidateArtifact {
            artifact_id: "candidate-1".to_string(),
            version: 1,
            generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 1, 0, 0).unwrap(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("analyze:1".to_string()),
            decision_hint: "hint".to_string(),
            trade_direction: Direction::Bull,
            actionable: true,
            candidate_status: "ready".to_string(),
            ..ExecutionCandidateArtifact::default()
        };

        let snapshot = build_workflow_snapshot(BuildWorkflowSnapshotInput {
            state_dir: "state",
            symbol: "NQ",
            latest_train: None,
            latest_analyze: None,
            latest_research: None,
            latest_backtest: None,
            latest_update: None,
            pre_bayes_policy_history: &[],
            pending_update_history: &[pending],
            execution_candidate_history: &[execution],
            artifact_ledger: &[
                ArtifactLedgerEntry {
                    entry_id: "ledger:pending-1".to_string(),
                    artifact_kind: "pending_update".to_string(),
                    artifact_id: "pending-1".to_string(),
                    version: 1,
                    generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                    symbol: "NQ".to_string(),
                    source_phase: "analyze".to_string(),
                    source_run_id: Some("analyze:1".to_string()),
                    path: "state/NQ/pending_update_feedback.json".to_string(),
                    status: "promote_latest".to_string(),
                    promote_candidate: true,
                    actionable: true,
                    decision_hint: "hint".to_string(),
                    review_reason: "strict_probability_and_score_improvement".to_string(),
                    review_rule_version: "rules-v1".to_string(),
                    top_factor_name: Some("trend_momentum".to_string()),
                    top_factor_action: Some("keep".to_string()),
                    family_scores: BTreeMap::from([("trend_momentum".to_string(), 0.72)]),
                    supersedes_artifact_id: None,
                    quality_score: 80,
                    consumed_by_update_run_id: None,
                    consumed_at: None,
                    consumed_outcome: None,
                    regraded_at: None,
                    consumption_regrade_status: None,
                    consumption_regrade_reason: None,
                },
                ArtifactLedgerEntry {
                    entry_id: "ledger:candidate-1".to_string(),
                    artifact_kind: "execution_candidate".to_string(),
                    artifact_id: "candidate-1".to_string(),
                    version: 1,
                    generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 1, 0, 0).unwrap(),
                    symbol: "NQ".to_string(),
                    source_phase: "analyze".to_string(),
                    source_run_id: Some("analyze:1".to_string()),
                    path: "state/NQ/execution_candidate.json".to_string(),
                    status: "ready".to_string(),
                    promote_candidate: true,
                    actionable: true,
                    decision_hint: "hint".to_string(),
                    review_reason: "low".to_string(),
                    review_rule_version: "rules-v1".to_string(),
                    top_factor_name: Some("trend_momentum".to_string()),
                    top_factor_action: Some("keep".to_string()),
                    family_scores: BTreeMap::from([("trend_momentum".to_string(), 0.72)]),
                    supersedes_artifact_id: None,
                    quality_score: 70,
                    consumed_by_update_run_id: None,
                    consumed_at: None,
                    consumed_outcome: None,
                    regraded_at: None,
                    consumption_regrade_status: None,
                    consumption_regrade_reason: None,
                },
            ],
        });

        assert_eq!(snapshot.actionable_artifacts.len(), 2);
        assert!(snapshot.latest_promotable_artifact.is_some());
        assert!(!snapshot.artifact_factor_trends.is_empty());
        assert!(!snapshot.artifact_family_trends.is_empty());
        assert!(!snapshot.artifact_lineage_summaries.is_empty());
        assert!(
            snapshot
                .artifact_review_rules
                .pending_update
                .require_same_data
        );
        assert!(!snapshot
            .artifact_review_rule_sources
            .pending_update
            .is_empty());
    }

    #[test]
    fn test_artifact_status_and_diff_commands_run() {
        let temp = tempfile::tempdir().unwrap();
        append_artifact_ledger_entry(
            temp.path(),
            "NQ",
            ArtifactLedgerEntry {
                entry_id: "ledger:pending-1".to_string(),
                artifact_kind: "pending_update".to_string(),
                artifact_id: "pending-1".to_string(),
                version: 1,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:1".to_string()),
                path: "state/NQ/pending_update_feedback.json".to_string(),
                status: "promote_latest".to_string(),
                promote_candidate: true,
                actionable: true,
                decision_hint: "hint".to_string(),
                review_reason: "strict_probability_and_score_improvement".to_string(),
                review_rule_version: "rules-v1".to_string(),
                top_factor_name: Some("trend_momentum".to_string()),
                top_factor_action: Some("tune".to_string()),
                family_scores: BTreeMap::from([("trend_momentum".to_string(), 0.45)]),
                supersedes_artifact_id: None,
                quality_score: 80,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();
        append_artifact_ledger_entry(
            temp.path(),
            "NQ",
            ArtifactLedgerEntry {
                entry_id: "ledger:pending-2".to_string(),
                artifact_kind: "pending_update".to_string(),
                artifact_id: "pending-2".to_string(),
                version: 2,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 1, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:2".to_string()),
                path: "state/NQ/pending_update_feedback.json".to_string(),
                status: "promote_latest".to_string(),
                promote_candidate: true,
                actionable: true,
                decision_hint: "hint-2".to_string(),
                review_reason: "strict_probability_and_score_improvement".to_string(),
                review_rule_version: "rules-v1".to_string(),
                top_factor_name: Some("trend_momentum".to_string()),
                top_factor_action: Some("keep".to_string()),
                family_scores: BTreeMap::from([("trend_momentum".to_string(), 0.74)]),
                supersedes_artifact_id: Some("pending-1".to_string()),
                quality_score: 90,
                consumed_by_update_run_id: None,
                consumed_at: None,
                consumed_outcome: None,
                regraded_at: None,
                consumption_regrade_status: None,
                consumption_regrade_reason: None,
            },
        )
        .unwrap();
        append_pending_update_artifact_history(
            temp.path(),
            "NQ",
            PendingUpdateArtifact {
                artifact_id: "pending-1".to_string(),
                version: 1,
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:1".to_string()),
                entry_quality: "high".to_string(),
                factor_alignment: "bullish".to_string(),
                factor_uncertainty: "low".to_string(),
                selected_win_probability: 0.64,
                top_factor_score: 0.72,
                avg_family_score: 0.68,
                ..PendingUpdateArtifact::default()
            },
        )
        .unwrap();
        append_pending_update_artifact_history(
            temp.path(),
            "NQ",
            PendingUpdateArtifact {
                artifact_id: "pending-2".to_string(),
                version: 2,
                source_phase: "analyze".to_string(),
                source_run_id: Some("analyze:2".to_string()),
                entry_quality: "high".to_string(),
                factor_alignment: "bullish".to_string(),
                factor_uncertainty: "low".to_string(),
                selected_win_probability: 0.69,
                top_factor_score: 0.80,
                avg_family_score: 0.74,
                ..PendingUpdateArtifact::default()
            },
        )
        .unwrap();

        artifact_status_command(ArtifactStatusCommandInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            artifact_id: None,
            kind: Some("pending_update"),
            latest_only: true,
            actionable_only: false,
            rule_break_only: false,
            sort_by: "generated",
            descending: true,
            limit: None,
            recent_n: None,
            consumed_only: false,
            bucket_by_kind: false,
            bucket_order_by: "kind",
            bucket_limit: None,
        })
        .unwrap();
        artifact_diff_command(ArtifactDiffCommandInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            left_artifact_id: "pending-1",
            right_artifact_id: "pending-2",
        })
        .unwrap();
        let ledger = load_artifact_ledger(temp.path(), "NQ").unwrap();
        let snapshot = refresh_workflow_snapshot(temp.path().to_str().unwrap(), "NQ").unwrap();
        artifact_lineage_command(ArtifactLineageCommandInput {
            symbol: "NQ",
            ledger: &ledger,
            summaries: snapshot.artifact_lineage_summaries.clone(),
            artifact_id: Some("pending-2"),
            latest_only: false,
            improving_only: false,
            regressing_only: false,
            rule_break_only: false,
        })
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("artifact-factor-trends"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("artifact-lineage-summaries"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("artifact-decision-summary"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        artifact_status_command(ArtifactStatusCommandInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            artifact_id: None,
            kind: Some("pending_update"),
            latest_only: false,
            actionable_only: false,
            rule_break_only: true,
            sort_by: "generated",
            descending: true,
            limit: None,
            recent_n: None,
            consumed_only: false,
            bucket_by_kind: false,
            bucket_order_by: "kind",
            bucket_limit: None,
        })
        .unwrap();
        artifact_status_command(ArtifactStatusCommandInput {
            symbol: "NQ",
            state_dir: temp.path().to_str().unwrap(),
            artifact_id: None,
            kind: Some("pending_update"),
            latest_only: false,
            actionable_only: false,
            rule_break_only: false,
            sort_by: "quality",
            descending: true,
            limit: Some(1),
            recent_n: None,
            consumed_only: false,
            bucket_by_kind: false,
            bucket_order_by: "kind",
            bucket_limit: None,
        })
        .unwrap();
        let snapshot = refresh_workflow_snapshot(temp.path().to_str().unwrap(), "NQ").unwrap();
        artifact_lineage_command(ArtifactLineageCommandInput {
            symbol: "NQ",
            ledger: &ledger,
            summaries: snapshot.artifact_lineage_summaries,
            artifact_id: None,
            latest_only: false,
            improving_only: false,
            regressing_only: false,
            rule_break_only: true,
        })
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("artifact-impact-leaderboard"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        workflow_status_command(
            ict_engine::application::orchestration::WorkflowStatusCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                refresh: false,
                phase: Some("artifact-impact-consumed-trend"),
                actionable_only: false,
                conflicts_only: false,
                latest_promotable: false,
                hard_block_only: false,
                hard_block_reason: None,
                limit: None,
                output_format: "json",
                stable: false,
            },
        )
        .unwrap();
        pre_bayes_status_command("NQ", temp.path().to_str().unwrap(), false, Some("policy"))
            .unwrap();
    }

    #[test]
    fn test_dataset_comparability_exposes_structured_diff_dimensions() {
        let previous = RunProvenance {
            prompt_version: "prompt-v1".to_string(),
            factor_version: "factor-v1".to_string(),
            config_hash: "config-a".to_string(),
            data_fingerprint: "data-a".to_string(),
        };
        let current = RunProvenance {
            prompt_version: "prompt-v2".to_string(),
            factor_version: "factor-v1".to_string(),
            config_hash: "config-b".to_string(),
            data_fingerprint: "data-a".to_string(),
        };

        let comparability = ict_engine::application::backtest::dataset_comparability(
            Some("run-1".to_string()),
            Some(&previous),
            &current,
        );

        assert!(comparability.comparable);
        assert!(comparability.same_data);
        assert!(!comparability.same_config);
        assert!(!comparability.same_prompt_version);
        assert!(comparability.same_factor_version);
        assert_eq!(comparability.comparison_class, "same_data_different_config");
    }

    #[test]
    fn test_pre_bayes_policy_lineage_summary_suggests_rollback_on_observe_only() {
        let history = vec![
            PreBayesPolicyRecord {
                policy: ict_engine::state::PreBayesEvidencePolicy {
                    version: "v1".to_string(),
                    ..ict_engine::state::PreBayesEvidencePolicy::default()
                },
                ..PreBayesPolicyRecord::default()
            },
            PreBayesPolicyRecord {
                policy: ict_engine::state::PreBayesEvidencePolicy {
                    version: "v2".to_string(),
                    ..ict_engine::state::PreBayesEvidencePolicy::default()
                },
                diff_from_previous: ict_engine::state::PreBayesPolicyDiff {
                    changed_fields: vec!["hard_pass_quality_threshold".to_string()],
                    ..ict_engine::state::PreBayesPolicyDiff::default()
                },
                ..PreBayesPolicyRecord::default()
            },
        ];

        let summary = ict_engine::application::belief::pre_bayes_policy_lineage_summary(
            &history,
            "observe_only",
        );

        assert_eq!(summary.latest_version.as_deref(), Some("v2"));
        assert_eq!(summary.rollback_candidate_version.as_deref(), Some("v1"));
        assert!(summary
            .changed_fields_union
            .contains(&"hard_pass_quality_threshold".to_string()));
    }

    #[test]
    fn test_pre_bayes_report_summary_includes_bridge_surface() {
        let summary = ict_engine::application::belief::pre_bayes_report_summary(
            Some(&ict_engine::state::PreBayesEvidencePolicy {
                version: "policy-v1".to_string(),
                source: "test".to_string(),
                hard_pass_quality_threshold: 0.7,
                neutralized_quality_threshold: 0.5,
                ..ict_engine::state::PreBayesEvidencePolicy::default()
            }),
            Some(&ict_engine::state::PreBayesEntryQualityBridge {
                long_signal_probability: 0.7,
                short_signal_probability: 0.3,
                multi_timeframe_direction_bias: "bullish".to_string(),
                multi_timeframe_alignment_score: Some(0.8),
                multi_timeframe_entry_alignment_score: Some(0.6),
                rationale: vec!["bridge".to_string()],
                selected_entry_quality: BTreeMap::from([("high".to_string(), 0.7)]),
                ..ict_engine::state::PreBayesEntryQualityBridge::default()
            }),
        );

        assert!(summary
            .iter()
            .any(|line| line.contains("policy_version=policy-v1")));
        assert!(summary
            .iter()
            .any(|line| line.contains("selected_entry_quality")));
        assert!(summary
            .iter()
            .any(|line| line.contains("mtf_direction=bullish")));
    }

    #[test]
    fn test_artifact_review_rules_surface_sources_and_versions() {
        let rules = ict_engine::application::artifacts::artifact_review_rules();
        let sources = ict_engine::application::artifacts::artifact_review_rule_sources();
        let pending_version =
            ict_engine::application::artifacts::pending_update_review_rule_version(
                &rules.pending_update,
            );
        let execution_version =
            ict_engine::application::artifacts::execution_candidate_review_rule_version(
                &rules.execution_candidate,
            );

        assert!(rules.pending_update.min_probability_improvement > 0.0);
        assert!(sources
            .pending_update
            .contains_key("min_probability_improvement"));
        assert!(!pending_version.is_empty());
        assert!(!execution_version.is_empty());
    }

    #[test]
    fn test_apply_artifact_consumption_preview_marks_consumed_entry() {
        let mut ledger = vec![ArtifactLedgerEntry {
            artifact_id: "pending-1".to_string(),
            artifact_kind: "pending_update".to_string(),
            actionable: true,
            promote_candidate: true,
            quality_score: 50,
            ..ArtifactLedgerEntry::default()
        }];

        ict_engine::application::artifacts::apply_artifact_consumption_preview(
            &mut ledger,
            "pending-1",
            "update:1",
            "win",
            0.02,
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        );

        assert_eq!(
            ledger[0].consumed_by_update_run_id.as_deref(),
            Some("update:1")
        );
        assert_eq!(
            ledger[0].consumption_regrade_status.as_deref(),
            Some("validated_positive")
        );
        assert!(!ledger[0].actionable);
        assert!(!ledger[0].promote_candidate);
    }

    #[test]
    fn test_link_artifact_decision_summary_to_decisions_updates_reasons_and_targets() {
        let summary = ict_engine::state::ArtifactDecisionSummary {
            actionable_artifact_count: 2,
            latest_promotable_artifact_id: Some("artifact-1".to_string()),
            artifact_rule_break_count: 1,
            consumed_trend_status: "validated_regressing".to_string(),
            consumed_trend_reason: "recent_consumed_regression".to_string(),
            consumed_target_kinds: vec!["pending_update".to_string()],
            promotion_strength: "promote_with_artifact_confirmation".to_string(),
            rollback_strength: "rollback_due_to_artifact_regression".to_string(),
            highlighted_factor_targets: vec!["structure_ict".to_string()],
            highlighted_family_targets: vec!["trend_momentum".to_string()],
            ..ict_engine::state::ArtifactDecisionSummary::default()
        };
        let mut promotion = PromotionDecision::default();
        let mut rollback = RollbackRecommendation::default();

        ict_engine::application::backtest::link_artifact_decision_summary_to_decisions(
            &summary,
            &mut promotion,
            &mut rollback,
        );

        assert!(promotion.reason.contains("artifact_actionable_count=2"));
        assert!(promotion.reason.contains("artifact_promotion_strength="));
        assert!(rollback.reason.contains("artifact_rollback_strength="));
        assert!(rollback
            .reason
            .contains("artifact_consumed_trend_reason=recent_consumed_regression"));
        assert!(promotion
            .target_factors
            .contains(&"structure_ict".to_string()));
        assert!(rollback
            .target_families
            .contains(&"trend_momentum".to_string()));
    }

    #[test]
    fn test_derive_finalize_backtest_decision_surfaces_returns_expected_counts() {
        let previous_runs = vec![BacktestRunRecord {
            run_id: "backtest:1".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            promotion_decision: PromotionDecision {
                approved: true,
                status: "promote".to_string(),
                ..PromotionDecision::default()
            },
            rollback_recommendation: RollbackRecommendation {
                should_rollback: false,
                scope: "none".to_string(),
                ..RollbackRecommendation::default()
            },
            factor_family_decisions: vec![FactorFamilyDecision {
                family: "trend_momentum".to_string(),
                avg_score: 0.4,
                replacement_candidates: vec![],
                ..FactorFamilyDecision::default()
            }],
            ..BacktestRunRecord::default()
        }];
        let score_deltas = vec![RankingDiffItem {
            factor_name: "trend_momentum".to_string(),
            score_delta: 0.2,
            new_score: 0.8,
            new_weight: 0.3,
            new_action: "keep".to_string(),
            ..RankingDiffItem::default()
        }];
        let probability_deltas = vec![ProbabilityDiff {
            state: "high:win".to_string(),
            delta: 0.1,
            new: 0.6,
            ..ProbabilityDiff::default()
        }];
        let factor_ranking = vec![PersistedFactorRanking {
            factor_name: "trend_momentum".to_string(),
            composite_score: 0.8,
            conformal_coverage_1sigma: 0.8,
            regime_break_penalty: 0.05,
            ..PersistedFactorRanking::default()
        }];
        let family_decisions = vec![FactorFamilyDecision {
            family: "trend_momentum".to_string(),
            avg_score: 0.8,
            replacement_candidates: vec![],
            ..FactorFamilyDecision::default()
        }];

        let surfaces = ict_engine::application::backtest::derive_finalize_backtest_decision_surfaces(
            ict_engine::application::backtest::FinalizeBacktestDecisionSurfacesInput {
                previous_runs: &previous_runs,
                factor_ranking: &factor_ranking,
                factor_family_decisions: &family_decisions,
                score_deltas: &score_deltas,
                probability_deltas: &probability_deltas,
                dataset_comparability: &DatasetComparability {
                    comparable: true,
                    ..DatasetComparability::default()
                },
                artifact_consumed_gate: &ict_engine::application::decision_utils::ArtifactConsumedDecisionGate::default(),
                artifact_family_trends: &[],
            },
        );

        assert_eq!(surfaces.decision_history_summary.total_runs, 1);
        assert_eq!(surfaces.factor_family_diffs.len(), 1);
        assert_eq!(surfaces.factor_family_history.len(), 1);
        assert_eq!(surfaces.factor_family_outcomes.len(), 1);
    }

    #[test]
    fn test_load_finalize_backtest_artifact_surfaces_builds_decision_summary() {
        let temp = tempfile::tempdir().unwrap();
        append_artifact_ledger_entry(
            temp.path(),
            "NQ",
            ArtifactLedgerEntry {
                entry_id: "entry-1".to_string(),
                artifact_kind: "pending_update".to_string(),
                artifact_id: "pending-1".to_string(),
                version: 1,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                path: "state/NQ/pending.json".to_string(),
                status: "promote_latest".to_string(),
                promote_candidate: true,
                actionable: true,
                ..ArtifactLedgerEntry::default()
            },
        )
        .unwrap();

        let surfaces = ict_engine::application::backtest::load_finalize_backtest_artifact_surfaces(
            temp.path().to_str().unwrap(),
            "NQ",
        )
        .unwrap();

        assert_eq!(surfaces.decision_summary.actionable_artifact_count, 1);
        assert_eq!(
            surfaces
                .decision_summary
                .latest_promotable_artifact_id
                .as_deref(),
            Some("pending-1")
        );
        assert_eq!(
            surfaces.decision_section.summary.actionable_artifact_count,
            1
        );
    }

    #[test]
    fn test_workflow_snapshot_detects_analyze_update_disagreement() {
        let analyze = AnalyzeRunRecord {
            run_id: "analyze:1".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            symbol: "NQ".to_string(),
            selected_direction: Direction::Bull,
            selected_entry_quality: "high".to_string(),
            workflow_state: WorkflowState {
                phase: "observe_or_deploy".to_string(),
                reason: "bull_bias".to_string(),
            },
            recommended_next_command:
                "ict-engine factor-research --symbol NQ --data ltf.json --state-dir state"
                    .to_string(),
            ..AnalyzeRunRecord::default()
        };
        let update = UpdateRunRecord {
            run_id: "update:1".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
            symbol: "NQ".to_string(),
            ensemble_executor_scorecards: vec![EnsembleExecutorScorecard {
                executor: "catboost_file".to_string(),
                wins: 1,
                ..EnsembleExecutorScorecard::default()
            }],
            rollback_recommendation: RollbackRecommendation {
                should_rollback: true,
                scope: "targeted".to_string(),
                reason: "outcome_calibration_regressed".to_string(),
                target_factors: vec!["trend_momentum".to_string()],
                target_families: Vec::new(),
            },
            workflow_state: WorkflowState {
                phase: "rollback_review".to_string(),
                reason: "outcome_calibration_regressed".to_string(),
            },
            realized_outcome: "loss".to_string(),
            recommended_next_command:
                "ict-engine update --symbol NQ --outcome loss --state-dir state".to_string(),
            ..UpdateRunRecord::default()
        };

        let snapshot = build_workflow_snapshot(BuildWorkflowSnapshotInput {
            state_dir: "state",
            symbol: "NQ",
            latest_train: None,
            latest_analyze: Some(&analyze),
            latest_research: None,
            latest_backtest: None,
            latest_update: Some(&update),
            pre_bayes_policy_history: &[],
            pending_update_history: &[],
            execution_candidate_history: &[],
            artifact_ledger: &[],
        });

        assert!(snapshot
            .disagreements
            .iter()
            .any(|item| item.id == "analyze_direction_vs_update_rollback"));
    }

    #[test]
    fn test_workflow_snapshot_exposes_family_factor_conflict_sources() {
        let research = WorkflowPhaseSnapshot {
            phase: "research".to_string(),
            family_states: vec!["trend_momentum:hold:none".to_string()],
            factor_actions: vec!["trend_momentum:replace:0.31".to_string()],
            family_score_map: BTreeMap::from([("trend_momentum".to_string(), 0.41)]),
            factor_score_map: BTreeMap::from([("trend_momentum".to_string(), 0.31)]),
            ..WorkflowPhaseSnapshot::default()
        };
        let backtest = WorkflowPhaseSnapshot {
            phase: "backtest".to_string(),
            family_states: vec!["trend_momentum:promote:none".to_string()],
            factor_actions: vec!["trend_momentum:keep:0.72".to_string()],
            family_score_map: BTreeMap::from([("trend_momentum".to_string(), 0.73)]),
            factor_score_map: BTreeMap::from([("trend_momentum".to_string(), 0.72)]),
            ..WorkflowPhaseSnapshot::default()
        };

        let family_sources = family_conflict_sources(&research, &backtest);
        let factor_sources = factor_conflict_sources(&research, &backtest);

        assert_eq!(family_sources[0].scope, "family");
        assert_eq!(family_sources[0].subject, "trend_momentum");
        assert_eq!(factor_sources[0].scope, "factor");
        assert_eq!(factor_sources[0].subject, "trend_momentum");
        assert!(!family_sources[0].evidence.is_empty());
        assert!(!factor_sources[0].evidence.is_empty());
    }

    #[test]
    fn test_workflow_snapshot_detects_score_vs_artifact_gate_conflict() {
        let research = WorkflowPhaseSnapshot {
            phase: "research".to_string(),
            promotion_status: "promote".to_string(),
            ..WorkflowPhaseSnapshot::default()
        };
        let update = WorkflowPhaseSnapshot {
            phase: "update".to_string(),
            workflow_phase: "artifact_rollback_review".to_string(),
            rollback_scope: "artifact".to_string(),
            ..WorkflowPhaseSnapshot::default()
        };

        let disagreements = workflow_disagreements(&None, &Some(research), &None, &Some(update));

        assert!(disagreements
            .iter()
            .any(|item| item.summary.contains("artifact consumption rollback gate")));
    }

    #[test]
    fn test_workflow_snapshot_detects_pre_bayes_gate_vs_promotion_conflict() {
        let analyze = WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            pre_bayes_gate_status: "observe_only".to_string(),
            pre_bayes_evidence_quality_score: 0.22,
            ..WorkflowPhaseSnapshot::default()
        };
        let research = WorkflowPhaseSnapshot {
            phase: "research".to_string(),
            promotion_status: "promote".to_string(),
            ..WorkflowPhaseSnapshot::default()
        };

        let disagreements = workflow_disagreements(&Some(analyze), &Some(research), &None, &None);

        assert!(disagreements
            .iter()
            .any(|item| item.id.contains("pre_bayes_observe_only")));
    }

    #[test]
    fn test_workflow_disagreement_exposes_pre_bayes_bridge_evidence() {
        let analyze = WorkflowPhaseSnapshot {
            phase: "analyze".to_string(),
            pre_bayes_gate_status: "observe_only".to_string(),
            pre_bayes_uses_soft_evidence: true,
            pre_bayes_policy_version: "policy-v2".to_string(),
            pre_bayes_filtered_assignments: BTreeMap::from([(
                "market_regime".to_string(),
                "range".to_string(),
            )]),
            pre_bayes_soft_evidence: BTreeMap::from([(
                "market_regime".to_string(),
                BTreeMap::from([("bull".to_string(), 0.60), ("range".to_string(), 0.40)]),
            )]),
            pre_bayes_bridge_selected_entry_quality: Some("medium".to_string()),
            pre_bayes_bridge_probability_gap: Some(0.18),
            ..WorkflowPhaseSnapshot::default()
        };
        let research = WorkflowPhaseSnapshot {
            phase: "research".to_string(),
            promotion_status: "promote".to_string(),
            ..WorkflowPhaseSnapshot::default()
        };

        let disagreements = workflow_disagreements(&Some(analyze), &Some(research), &None, &None);
        let disagreement = disagreements
            .iter()
            .find(|item| item.id.contains("pre_bayes_observe_only"))
            .expect("missing observe_only disagreement");

        assert!(disagreement
            .evidence
            .iter()
            .any(|item| item.contains("pre_bayes_bridge_selected_entry_quality=medium")));
        assert!(disagreement
            .evidence
            .iter()
            .any(|item| item.contains("pre_bayes_uses_soft_evidence=true")));
        assert!(disagreement
            .sources
            .iter()
            .any(|item| item.left_value.contains("policy-v2:medium")));
    }

    #[test]
    fn test_futures_sop_report_can_hold_pre_bayes_summary() {
        let report = FuturesSopReport {
            sop_version: "futures-sop-v1".to_string(),
            generated_at: Utc::now(),
            root: "root".to_string(),
            output_dir: "out".to_string(),
            cleaned_dir: "clean".to_string(),
            state_dir: "state".to_string(),
            interval: "15m".to_string(),
            selection_policy: "policy".to_string(),
            clean_report: CleanFuturesReport {
                root: "root".to_string(),
                output_dir: "out".to_string(),
                interval: "15m".to_string(),
                datasets: Vec::new(),
            },
            market_reports: vec![FuturesSopMarketReport {
                market: "NQ".to_string(),
                cleaned_path: "nq.json".to_string(),
                candle_count: 100,
                multi_timeframe_summary: Vec::new(),
                best_factor: Some("structure_ict".to_string()),
                promotion_status: "hold".to_string(),
                rollback_scope: "none".to_string(),
                workflow_phase: "research_iteration".to_string(),
                artifact_gate_status: "no_consumed_validation".to_string(),
                recommended_next_command: "cmd".to_string(),
                aggregate_return: 0.0,
                aggregate_return_warning: None,
                top_scorecards: Vec::new(),
                pipeline: None,
            }],
            global_factor_leaderboard: Vec::new(),
            recommended_global_factor: Some("structure_ict".to_string()),
            recommended_global_pre_bayes_policy: Some(pre_bayes_evidence_policy()),
            recommended_global_pre_bayes_entry_quality_bridge: Some(
                ict_engine::state::PreBayesEntryQualityBridge::default(),
            ),
            recommended_global_pre_bayes_summary: vec!["summary".to_string()],
            recommended_global_pre_bayes_policy_lineage: Some(
                ict_engine::state::PreBayesPolicyLineageSummary::default(),
            ),
            recommended_global_pre_bayes_soft_evidence_diff: Vec::new(),
            recommended_global_pipeline_debug: None,
            recommended_market_factors: BTreeMap::new(),
            warnings: Vec::new(),
            recommended_commands: Vec::new(),
        };

        assert_eq!(
            report.recommended_global_pre_bayes_summary,
            vec!["summary".to_string()]
        );
    }

    #[test]
    fn test_build_futures_sop_market_report_available_via_application_api() {
        let report = ict_engine::factor_lab::ResearchReport {
            best_factor: Some("structure_ict".to_string()),
            aggregate_return: 1_500_000.0,
            promotion_decision: PromotionDecision {
                status: "hold".to_string(),
                ..PromotionDecision::default()
            },
            rollback_recommendation: RollbackRecommendation {
                scope: "none".to_string(),
                ..RollbackRecommendation::default()
            },
            workflow_state: WorkflowState {
                phase: "research_iteration".to_string(),
                ..WorkflowState::default()
            },
            artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary {
                consumed_trend_status: "no_consumed_validation".to_string(),
                ..ict_engine::state::ArtifactDecisionSummary::default()
            },
            recommended_next_command: "cmd".to_string(),
            backtest: ict_engine::factor_lab::BacktestResult {
                scorecards: vec![PersistedFactorRanking {
                    factor_name: "structure_ict".to_string(),
                    composite_score: 0.88,
                    grade: "A".to_string(),
                    iteration_action: "keep".to_string(),
                    ..PersistedFactorRanking {
                        factor_name: String::new(),
                        regime: String::new(),
                        ic: 0.0,
                        ir: 0.0,
                        backtest_return: 0.0,
                        sharpe: 0.0,
                        stability: 0.0,
                        win_rate: 0.0,
                        profit_factor: 0.0,
                        trade_count: 0,
                        conformal_coverage_1sigma: 0.0,
                        conformal_miscoverage_1sigma: 0.0,
                        mean_prediction_interval_half_width: 0.0,
                        worst_window_miscoverage: 0.0,
                        regime_break_penalty: 0.0,
                        weight: 0.0,
                        regime_scores: BTreeMap::new(),
                        composite_score: 0.0,
                        score_breakdown: BTreeMap::new(),
                        grade: String::new(),
                        iteration_action: String::new(),
                        replacement_candidate: false,
                        weaknesses: Vec::new(),
                        agent_prompt: String::new(),
                    }
                }],
                ..ict_engine::factor_lab::BacktestResult::default()
            },
            multi_timeframe_summary: vec!["summary".to_string()],
            ..ict_engine::factor_lab::ResearchReport::default()
        };

        let (market_report, warning) =
            ict_engine::application::data_sources::build_futures_sop_market_report(
                "NQ", "nq.json", 100, &report, None,
            );

        assert_eq!(market_report.market, "NQ");
        assert_eq!(market_report.best_factor.as_deref(), Some("structure_ict"));
        assert!(warning
            .as_deref()
            .unwrap_or_default()
            .contains("aggregate_return="));
    }

    #[test]
    fn test_build_factor_pipeline_debug_report_contains_required_trace_fields() {
        let mut evidence_assignments = BTreeMap::new();
        evidence_assignments.insert("market_regime".to_string(), "bull".to_string());
        evidence_assignments.insert(
            "liquidity_context".to_string(),
            "sweep_supportive".to_string(),
        );
        evidence_assignments.insert("factor_alignment".to_string(), "aligned".to_string());
        evidence_assignments.insert("factor_uncertainty".to_string(), "stable".to_string());
        evidence_assignments.insert(
            "multi_timeframe_resonance".to_string(),
            "aligned".to_string(),
        );

        let bridge = ict_engine::state::PreBayesEntryQualityBridge {
            long_signal_probability: 0.72,
            short_signal_probability: 0.28,
            selected_entry_quality: BTreeMap::from([
                ("medium".to_string(), 0.35),
                ("high".to_string(), 0.65),
            ]),
            rationale: vec!["bridge_confirms_high".to_string()],
            ..ict_engine::state::PreBayesEntryQualityBridge::default()
        };

        let pipeline = ExpansionFactorPipelineReport {
            factor_name: "structure_ict".to_string(),
            parameters: BTreeMap::from([("lookback".to_string(), 20.0)]),
            latest_signal: ict_engine::application::belief::pipeline_types::ExpansionLatestSignal {
                timestamp: Utc::now(),
                direction: "bull".to_string(),
                value: 0.81,
                confidence: 0.74,
                explanation: "recent_sweep_then_displacement".to_string(),
            },
            probability_support:
                ict_engine::application::belief::pipeline_types::ExpansionProbabilitySupport {
                    long_support: 0.72,
                    short_support: 0.28,
                    support_gap: 0.44,
                    alignment_threshold: 0.10,
                    uncertainty: 0.18,
                    alignment_label: "aligned".to_string(),
                    uncertainty_label: "stable".to_string(),
                    long_entry_bias: vec![0.2, 0.3, 0.5],
                    short_entry_bias: vec![0.5, 0.3, 0.2],
                    bullish_factors: vec![ict_engine::factor_lab::FactorContribution {
                        factor_name: "structure_ict".to_string(),
                        category: "structure".to_string(),
                        direction: Direction::Bull,
                        value: 0.81,
                        confidence: 0.74,
                        weighted_score: 0.72,
                        uncertainty_contribution: 0.05,
                        explanation: "recent_sweep_then_displacement".to_string(),
                    }],
                    bearish_factors: vec![ict_engine::factor_lab::FactorContribution {
                        factor_name: "structure_ict_counterflow".to_string(),
                        category: "structure".to_string(),
                        direction: Direction::Bear,
                        value: -0.22,
                        confidence: 0.40,
                        weighted_score: -0.28,
                        uncertainty_contribution: 0.08,
                        explanation: "late bear expansion overlap".to_string(),
                    }],
                    uncertainty_factors: vec![ict_engine::factor_lab::FactorContribution {
                        factor_name: "multi_timeframe_noise".to_string(),
                        category: "context".to_string(),
                        direction: Direction::Neutral,
                        value: 0.0,
                        confidence: 0.52,
                        weighted_score: 0.0,
                        uncertainty_contribution: 0.18,
                        explanation: "entry window still carries opposing noise".to_string(),
                    }],
                },
            paired_market_quality_report: None,
            entry_quality_bridge: bridge.clone(),
            bbn_support: ict_engine::application::belief::pipeline_types::ExpansionBbnSupport {
                market_regime_label: "bull".to_string(),
                liquidity_context_label: "sweep_supportive".to_string(),
                evidence_policy: "policy-v2".to_string(),
                pre_bayes_filter: PreBayesEvidenceFilter {
                    raw_multi_timeframe_resonance_label: "mixed".to_string(),
                    filtered_multi_timeframe_resonance_label: "aligned".to_string(),
                    evidence_quality_score: 0.77,
                    gating_status: "pass_hard".to_string(),
                    evidence_assignments: evidence_assignments.clone(),
                    soft_multi_timeframe_resonance_distribution: BTreeMap::from([
                        ("aligned".to_string(), 0.68),
                        ("mixed".to_string(), 0.24),
                        ("dislocated".to_string(), 0.08),
                    ]),
                    ..PreBayesEvidenceFilter::default()
                },
                evidence_assignments,
                raw_market_regime_trace: FactorPipelineLabelSource {
                    label: "bull".to_string(),
                    derivation: "build_frame_features.regime_label".to_string(),
                    evidence: vec!["hmm_regime=bull".to_string()],
                },
                raw_liquidity_context_trace: FactorPipelineLabelSource {
                    label: "sweep_supportive".to_string(),
                    derivation: "build_frame_features.liquidity_label".to_string(),
                    evidence: vec!["frame_liquidity_label=sweep_supportive".to_string()],
                },
                raw_multi_timeframe_resonance_trace: FactorPipelineLabelSource {
                    label: "mixed".to_string(),
                    derivation: "classify_multi_timeframe_resonance".to_string(),
                    evidence: vec!["direction_conflict=false".to_string()],
                },
                entry_quality_base: BTreeMap::new(),
                entry_quality_long: BTreeMap::new(),
                entry_quality_short: BTreeMap::new(),
                trade_outcome_long: BTreeMap::new(),
                trade_outcome_short: BTreeMap::new(),
                selected_direction: "bull".to_string(),
                selected_win_probability: 0.66,
            },
            pipeline_summary: "latest sample clears pre-bayes and bridge".to_string(),
            recommended_actions: vec!["inspect_latest_sample".to_string()],
            frame_physics_trace: Vec::new(),
        };

        let report = ict_engine::application::belief::adapt_factor_pipeline_debug_report(
            ict_engine::application::belief::AdaptFactorPipelineDebugReportInput {
                symbol: "NQ",
                data: "/tmp/nq.json",
                objective: "expansion_manipulation",
                pipeline: &pipeline,
                raw_pre_bayes_labels: BTreeMap::from([
                    (
                        "market_regime".to_string(),
                        pipeline.bbn_support.market_regime_label.clone(),
                    ),
                    (
                        "liquidity_context".to_string(),
                        pipeline.bbn_support.liquidity_context_label.clone(),
                    ),
                    (
                        "factor_alignment".to_string(),
                        pipeline.probability_support.alignment_label.clone(),
                    ),
                    (
                        "factor_uncertainty".to_string(),
                        pipeline.probability_support.uncertainty_label.clone(),
                    ),
                    (
                        "multi_timeframe_resonance".to_string(),
                        pipeline
                            .bbn_support
                            .pre_bayes_filter
                            .raw_multi_timeframe_resonance_label
                            .clone(),
                    ),
                ]),
                soft_evidence_divergence: pre_bayes_soft_evidence_diff(
                    &pipeline.bbn_support.pre_bayes_filter,
                ),
                bridge_gap_clear_threshold: 0.12,
                multi_timeframe_summary: &[
                    "1m bull continuation".to_string(),
                    "5m aligned".to_string(),
                    "15m displacement confirmed".to_string(),
                    "1h bullish structure".to_string(),
                    "4h premium reprice".to_string(),
                    "1d higher-timeframe support".to_string(),
                ],
                paired_market_quality_report: None,
            },
        )
        .unwrap();

        assert_eq!(report.symbol, "NQ");
        assert_eq!(report.factor_name, "structure_ict");
        assert_eq!(report.objective, "expansion_manipulation");
        assert_eq!(report.gating_status, "pass_hard");
        assert_eq!(report.selected_entry_quality, "high");
        assert_eq!(report.factor_diagnostics.support_gap, 0.44);
        assert_eq!(report.factor_diagnostics.alignment_threshold, 0.10);
        assert_eq!(report.factor_diagnostics.bullish_factors.len(), 1);
        assert_eq!(report.factor_diagnostics.bearish_factors.len(), 1);
        assert_eq!(report.factor_diagnostics.uncertainty_factors.len(), 1);
        assert_eq!(report.raw_label_trace.market_regime.label, "bull");
        assert_eq!(
            report.raw_label_trace.market_regime.derivation,
            "build_frame_features.regime_label"
        );
        assert_eq!(
            report.raw_label_trace.liquidity_context.label,
            "sweep_supportive"
        );
        assert_eq!(
            report.raw_label_trace.multi_timeframe_resonance.label,
            "mixed"
        );
        assert!(report.bridge_gap > 0.0);
        assert_eq!(
            report
                .raw_pre_bayes_labels
                .get("multi_timeframe_resonance")
                .map(String::as_str),
            Some("mixed")
        );
        assert_eq!(
            report
                .filtered_pre_bayes_labels
                .get("multi_timeframe_resonance")
                .map(String::as_str),
            Some("aligned")
        );
        assert_eq!(report.six_timeframe_resonance.len(), 6);
    }

    #[test]
    fn test_expansion_sop_report_recommended_commands_include_objective_and_debug() {
        let report = ExpansionSopReport {
            sop_version: "expansion-sop-v1".to_string(),
            generated_at: Utc::now(),
            root: "/tmp/root".to_string(),
            output_dir: "/tmp/out".to_string(),
            cleaned_dir: "/tmp/out/cleaned-15m".to_string(),
            interval: "15m".to_string(),
            expansion_lookback: 20,
            expansion_atr_multiplier: 1.5,
            clean_report: CleanFuturesReport {
                root: "/tmp/root".to_string(),
                output_dir: "/tmp/out".to_string(),
                interval: "15m".to_string(),
                datasets: Vec::new(),
            },
            market_reports: Vec::new(),
            global_factor_leaderboard: Vec::new(),
            recommended_global_factor: Some("structure_ict".to_string()),
            recommended_global_pre_bayes_policy: None,
            recommended_global_pre_bayes_entry_quality_bridge: None,
            recommended_global_pre_bayes_summary: Vec::new(),
            recommended_global_pre_bayes_policy_lineage: None,
            recommended_global_pre_bayes_soft_evidence_diff: Vec::new(),
            recommended_global_pipeline_debug: None,
            recommended_market_factors: BTreeMap::new(),
            mutation_spec: None,
            factor_mutation_evaluation: None,
            warnings: Vec::new(),
            recommended_commands: vec![
                "ict-engine expansion-sop --root /tmp/root --output-dir /tmp/out --interval 15m --lookback 20 --atr-multiplier 1.50 --objective expansion_manipulation".to_string(),
                "ict-engine factor-pipeline-debug --symbol NQ --data /tmp/out/cleaned-15m/nq.continuous-15m.json --factor structure_ict --objective expansion_manipulation".to_string(),
            ],
        };

        assert!(report
            .recommended_commands
            .iter()
            .any(|cmd| cmd.contains("--objective expansion_manipulation")));
        assert!(report
            .recommended_commands
            .iter()
            .any(|cmd| cmd.contains("factor-pipeline-debug")));
    }

    #[test]
    fn test_expansion_factor_scores_for_market_available_via_application_api() {
        let candles = sample_candles(160);
        let scores = ict_engine::application::factor_lifecycle::expansion_factor_scores_for_market(
            &FactorRegistry::default(),
            &candles,
            20,
            1.5,
        )
        .unwrap();

        assert!(!scores.is_empty());
    }

    #[test]
    fn test_factor_specific_hint_preferences_available_via_application_api() {
        let temp = tempfile::tempdir().unwrap();
        let (direction_hints, step_hints) =
            ict_engine::application::factor_lifecycle::factor_specific_hint_preferences(
                temp.path().to_str().unwrap(),
                "NQ",
                "structure_ict",
            );

        assert!(direction_hints.is_empty());
        assert!(step_hints.is_empty());
    }

    #[test]
    fn test_apply_expansion_manipulation_objective_available_via_application_api() {
        let candles = sample_candles(160);
        let registry = FactorRegistry::default();
        let mut report = ict_engine::factor_lab::ResearchReport {
            backtest: ict_engine::factor_lab::BacktestResult {
                scorecards: vec![PersistedFactorRanking {
                    factor_name: "structure_ict".to_string(),
                    regime: "trend".to_string(),
                    ic: 0.0,
                    ir: 0.0,
                    backtest_return: 0.0,
                    sharpe: 0.0,
                    stability: 0.0,
                    win_rate: 0.0,
                    profit_factor: 0.0,
                    trade_count: 0,
                    conformal_coverage_1sigma: 0.0,
                    conformal_miscoverage_1sigma: 0.0,
                    mean_prediction_interval_half_width: 0.0,
                    worst_window_miscoverage: 0.0,
                    regime_break_penalty: 0.0,
                    weight: 1.0,
                    regime_scores: BTreeMap::new(),
                    composite_score: 0.0,
                    score_breakdown: BTreeMap::new(),
                    grade: "C".to_string(),
                    iteration_action: "observe".to_string(),
                    replacement_candidate: false,
                    weaknesses: Vec::new(),
                    agent_prompt: String::new(),
                }],
                ..ict_engine::factor_lab::BacktestResult::default()
            },
            ..ict_engine::factor_lab::ResearchReport::default()
        };

        ict_engine::application::factor_lifecycle::apply_expansion_manipulation_objective(
            &mut report,
            &registry,
            "NQ",
            &candles,
            &[],
            Some(1.10),
        )
        .unwrap();

        assert!(!report.objective_surfaces.is_empty());
        assert_eq!(report.best_factor.as_deref(), Some("structure_ict"));
    }

    #[test]
    fn test_build_expansion_sop_metrics_from_market_reports_available_via_application_api() {
        let metrics =
            ict_engine::application::factor_lifecycle::build_expansion_sop_metrics_from_market_reports(
                &[ExpansionMarketReport {
                    market: "NQ".to_string(),
                    cleaned_path: "nq.json".to_string(),
                    total_candles: 100,
                    expansion_samples: 12,
                    bull_expansion_samples: 7,
                    bear_expansion_samples: 5,
                    best_factor: Some("structure_ict".to_string()),
                    top_factors: vec![ExpansionFactorScore {
                        factor_name: "structure_ict".to_string(),
                        expansion_samples: 12,
                        bull_expansion_samples: 7,
                        bear_expansion_samples: 5,
                        bull_hit_rate: 0.7,
                        bear_hit_rate: 0.6,
                        balanced_accuracy: 0.65,
                        directional_accuracy: 0.66,
                        confidence_weighted_accuracy: 0.64,
                        mean_confidence: 0.61,
                        neutral_predictions: 0,
                        wrong_direction_predictions: 1,
                        fit_score: 0.655,
                    }],
                    multi_timeframe_summary: Vec::new(),
                    pipeline: None,
                }],
            );

        assert_eq!(metrics.top_factor_names, vec!["structure_ict".to_string()]);
        assert_eq!(metrics.expansion_balanced_accuracy, Some(0.65));
    }

    #[test]
    fn test_run_expansion_sop_with_available_via_application_api() {
        let _ = ict_engine::application::data_sources::run_expansion_sop_with::<
            fn(ExpansionSopMarketInput, &str, &FactorRegistry) -> Result<ExpansionMarketReport>,
        >;
    }

    #[test]
    fn test_expansion_regression_reasons_available_via_application_api() {
        let temp = tempfile::tempdir().unwrap();
        let market_dir = temp.path().join("cleaned-15m");
        std::fs::create_dir_all(&market_dir).unwrap();
        let output_path = market_dir.join("nq.continuous-15m.json");
        std::fs::write(
            &output_path,
            serde_json::to_string(&CleanedCandleOutput {
                symbol: "NQ".to_string(),
                candles: sample_candles(40),
            })
            .unwrap(),
        )
        .unwrap();

        let reasons =
            ict_engine::application::factor_lifecycle::expansion_regression_reasons_by_market(
                &FactorRegistry::default(),
                &FactorRegistry::default(),
                &[("NQ", output_path.to_str().unwrap())],
                20,
                1.5,
            )
            .unwrap();

        assert!(reasons.is_empty());
    }

    #[test]
    fn test_apply_update_outcome_to_executor_scorecards_updates_counts() {
        let mut scorecards = vec![EnsembleExecutorScorecard {
            executor: "catboost_file".to_string(),
            ..EnsembleExecutorScorecard::default()
        }];
        apply_update_outcome_to_executor_scorecards(&mut scorecards, "win", 20);
        assert_eq!(scorecards[0].wins, 1);
        assert_eq!(scorecards[0].validated_positive, 1);
        assert_eq!(scorecards[0].cumulative_quality_score, 20);
    }

    #[test]
    fn test_update_command_records_consumed_artifacts_and_marks_ledger() {
        let temp = tempfile::tempdir().unwrap();
        let htf = temp.path().join("htf.json");
        let mtf = temp.path().join("mtf.json");
        let ltf = temp.path().join("ltf.json");

        for (path, count) in [(&htf, 220usize), (&mtf, 180usize), (&ltf, 140usize)] {
            std::fs::write(
                path,
                serde_json::to_string(&serde_json::json!({
                    "candles": sample_candles(count)
                }))
                .unwrap(),
            )
            .unwrap();
        }

        analyze_command(
            "NQ",
            htf.to_str().unwrap(),
            mtf.to_str().unwrap(),
            ltf.to_str().unwrap(),
            temp.path().to_str().unwrap(),
            OutputFormat::Json,
            false,
            true,
        )
        .unwrap();
        update_command(UpdateCommandInput {
            symbol: "NQ",
            outcome: "win",
            entry_signal: Some("strong_buy"),
            feedback_file: None,
            state_dir: temp.path().to_str().unwrap(),
            pnl: None,
            regime: None,
            direction: None,
            ensemble: false,
        })
        .unwrap();

        let runs: Vec<UpdateRunRecord> =
            load_state(temp.path(), "NQ", ict_engine::state::UPDATE_RUNS_FILE).unwrap();
        let ledger = load_artifact_ledger(temp.path(), "NQ").unwrap();

        assert_eq!(runs.len(), 1);
        assert!(runs[0].consumed_pending_update_artifact_id.is_some());
        assert!(runs[0].consumed_execution_candidate_artifact_id.is_some());
        assert!(runs[0].consumed_artifact_path.is_some());
        assert!(runs[0].consumed_analyze_run_id.is_some());
        assert!(runs[0].consumed_pre_bayes_evidence_filter.is_some());
        assert!(!runs[0].consumed_multi_timeframe_summary.is_empty());
        assert!(!runs[0].ensemble_executor_scorecards.is_empty());
        assert!(runs[0]
            .ensemble_executor_scorecards
            .iter()
            .any(|scorecard| !scorecard.executor.is_empty()));
        assert!(ledger.iter().any(|entry| {
            entry.consumed_by_update_run_id.as_deref() == Some(runs[0].run_id.as_str())
        }));
    }

    #[test]
    fn test_build_artifact_consumed_impact_summary_tracks_quality_deltas() {
        let summary = build_artifact_consumed_impact_summary(&[
            ArtifactLedgerEntry {
                entry_id: "a".to_string(),
                artifact_kind: "pending_update".to_string(),
                artifact_id: "a".to_string(),
                version: 1,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: None,
                path: "p".to_string(),
                status: "promote_latest".to_string(),
                promote_candidate: true,
                actionable: false,
                decision_hint: "decision_hint_unavailable".to_string(),
                review_reason: "review_reason_unavailable".to_string(),
                review_rule_version: "r1".to_string(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 80,
                consumed_by_update_run_id: Some("u1".to_string()),
                consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()),
                consumed_outcome: Some("win".to_string()),
                regraded_at: Some(Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()),
                consumption_regrade_status: Some("validated_positive".to_string()),
                consumption_regrade_reason: Some("ok".to_string()),
            },
            ArtifactLedgerEntry {
                entry_id: "b".to_string(),
                artifact_kind: "execution_candidate".to_string(),
                artifact_id: "b".to_string(),
                version: 1,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: None,
                path: "p".to_string(),
                status: "observe".to_string(),
                promote_candidate: false,
                actionable: false,
                decision_hint: "decision_hint_unavailable".to_string(),
                review_reason: "review_reason_unavailable".to_string(),
                review_rule_version: "r1".to_string(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 55,
                consumed_by_update_run_id: Some("u2".to_string()),
                consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 4, 0, 0, 0).unwrap()),
                consumed_outcome: Some("loss".to_string()),
                regraded_at: Some(Utc.with_ymd_and_hms(2024, 1, 4, 0, 0, 0).unwrap()),
                consumption_regrade_status: Some("validated_negative".to_string()),
                consumption_regrade_reason: Some("bad".to_string()),
            },
        ]);

        assert_eq!(summary.total_consumed, 2);
        assert_eq!(summary.positive_consumed, 1);
        assert_eq!(summary.negative_consumed, 1);
        assert_eq!(summary.points[1].quality_delta_from_previous_consumed, -25);
        assert_eq!(
            summary.by_kind["pending_update"].average_quality_score,
            80.0
        );
        assert!(summary.trend_comparisons.is_empty());
    }

    #[test]
    fn test_build_artifact_consumed_impact_summary_sorts_by_consumed_at_and_builds_windows() {
        let summary = build_artifact_consumed_impact_summary(&[
            ArtifactLedgerEntry {
                entry_id: "late".to_string(),
                artifact_kind: "pending_update".to_string(),
                artifact_id: "late".to_string(),
                version: 2,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 4, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: None,
                path: "p".to_string(),
                status: "promote_latest".to_string(),
                promote_candidate: true,
                actionable: false,
                decision_hint: "decision_hint_unavailable".to_string(),
                review_reason: "review_reason_unavailable".to_string(),
                review_rule_version: "r1".to_string(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 90,
                consumed_by_update_run_id: Some("u4".to_string()),
                consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 4, 0, 0, 0).unwrap()),
                consumed_outcome: Some("win".to_string()),
                regraded_at: Some(Utc.with_ymd_and_hms(2024, 1, 4, 0, 0, 0).unwrap()),
                consumption_regrade_status: Some("validated_positive".to_string()),
                consumption_regrade_reason: Some("good".to_string()),
            },
            ArtifactLedgerEntry {
                entry_id: "early".to_string(),
                artifact_kind: "execution_candidate".to_string(),
                artifact_id: "early".to_string(),
                version: 1,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: None,
                path: "p".to_string(),
                status: "observe".to_string(),
                promote_candidate: false,
                actionable: false,
                decision_hint: "decision_hint_unavailable".to_string(),
                review_reason: "review_reason_unavailable".to_string(),
                review_rule_version: "r1".to_string(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 40,
                consumed_by_update_run_id: Some("u1".to_string()),
                consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()),
                consumed_outcome: Some("loss".to_string()),
                regraded_at: Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()),
                consumption_regrade_status: Some("validated_negative".to_string()),
                consumption_regrade_reason: Some("bad".to_string()),
            },
            ArtifactLedgerEntry {
                entry_id: "mid".to_string(),
                artifact_kind: "pending_update".to_string(),
                artifact_id: "mid".to_string(),
                version: 1,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: None,
                path: "p".to_string(),
                status: "promote_latest".to_string(),
                promote_candidate: true,
                actionable: false,
                decision_hint: "decision_hint_unavailable".to_string(),
                review_reason: "review_reason_unavailable".to_string(),
                review_rule_version: "r1".to_string(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 65,
                consumed_by_update_run_id: Some("u2".to_string()),
                consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()),
                consumed_outcome: Some("neutral".to_string()),
                regraded_at: Some(Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()),
                consumption_regrade_status: Some("validated_neutral".to_string()),
                consumption_regrade_reason: Some("flat".to_string()),
            },
            ArtifactLedgerEntry {
                entry_id: "later".to_string(),
                artifact_kind: "execution_candidate".to_string(),
                artifact_id: "later".to_string(),
                version: 2,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap(),
                symbol: "NQ".to_string(),
                source_phase: "analyze".to_string(),
                source_run_id: None,
                path: "p".to_string(),
                status: "observe".to_string(),
                promote_candidate: false,
                actionable: false,
                decision_hint: "decision_hint_unavailable".to_string(),
                review_reason: "review_reason_unavailable".to_string(),
                review_rule_version: "r1".to_string(),
                top_factor_name: None,
                top_factor_action: None,
                family_scores: BTreeMap::new(),
                supersedes_artifact_id: None,
                quality_score: 88,
                consumed_by_update_run_id: Some("u3".to_string()),
                consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap()),
                consumed_outcome: Some("win".to_string()),
                regraded_at: Some(Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap()),
                consumption_regrade_status: Some("validated_positive".to_string()),
                consumption_regrade_reason: Some("good".to_string()),
            },
        ]);

        assert_eq!(
            summary
                .points
                .iter()
                .map(|point| point.artifact_id.as_str())
                .collect::<Vec<_>>(),
            vec!["early", "mid", "later", "late"]
        );
        assert_eq!(summary.points[3].quality_delta_from_previous_consumed, 2);
        assert!(summary
            .recent_windows
            .iter()
            .any(|window| window.label == "recent_3" && window.count == 3));
        assert!(summary.trend_comparisons.iter().any(|comparison| {
            comparison.label == "recent_3_vs_previous_1"
                && comparison.conclusion == "improving"
                && comparison.average_quality_score_delta > 0.0
        }));
    }

    #[test]
    fn test_artifact_decision_summary_uses_consumed_validation_signal() {
        let consumed_impact_summary = ict_engine::state::ArtifactConsumedImpactSummary {
            total_consumed: 4,
            trend_comparisons: vec![ict_engine::state::ArtifactConsumedImpactTrendComparison {
                label: "recent_3_vs_previous_1".to_string(),
                recent: ict_engine::state::ArtifactConsumedImpactWindow {
                    label: "recent_3".to_string(),
                    count: 3,
                    positive: 0,
                    negative: 2,
                    neutral: 1,
                    average_quality_score: 41.0,
                    cumulative_quality_delta: -18,
                },
                baseline: ict_engine::state::ArtifactConsumedImpactWindow {
                    label: "previous_1".to_string(),
                    count: 1,
                    positive: 1,
                    negative: 0,
                    neutral: 0,
                    average_quality_score: 83.0,
                    cumulative_quality_delta: 0,
                },
                average_quality_score_delta: -42.0,
                cumulative_quality_delta_delta: -18,
                positive_rate_delta: -1.0,
                conclusion: "regressing".to_string(),
            }],
            by_kind_trend_comparisons: BTreeMap::from([(
                "execution_candidate".to_string(),
                vec![ict_engine::state::ArtifactConsumedImpactTrendComparison {
                    label: "recent_3_vs_previous_1".to_string(),
                    recent: ict_engine::state::ArtifactConsumedImpactWindow {
                        label: "recent_3".to_string(),
                        count: 3,
                        positive: 0,
                        negative: 2,
                        neutral: 1,
                        average_quality_score: 41.0,
                        cumulative_quality_delta: -18,
                    },
                    baseline: ict_engine::state::ArtifactConsumedImpactWindow {
                        label: "previous_1".to_string(),
                        count: 1,
                        positive: 1,
                        negative: 0,
                        neutral: 0,
                        average_quality_score: 83.0,
                        cumulative_quality_delta: 0,
                    },
                    average_quality_score_delta: -42.0,
                    cumulative_quality_delta_delta: -18,
                    positive_rate_delta: -1.0,
                    conclusion: "regressing".to_string(),
                }],
            )]),
            ..ict_engine::state::ArtifactConsumedImpactSummary::default()
        };
        let summary = artifact_decision_summary_from_trends(
            &[ArtifactLedgerEntry {
                artifact_id: "pending-1".to_string(),
                artifact_kind: "pending_update".to_string(),
                actionable: true,
                promote_candidate: true,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                ..ArtifactLedgerEntry::default()
            }],
            Some(&ArtifactLedgerEntry {
                artifact_id: "pending-1".to_string(),
                artifact_kind: "pending_update".to_string(),
                actionable: true,
                promote_candidate: true,
                generated_at: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                ..ArtifactLedgerEntry::default()
            }),
            &[],
            &[],
            &[],
            &consumed_impact_summary,
        );

        assert_eq!(summary.consumed_trend_status, "validated_regressing");
        assert_eq!(summary.promotion_strength, "low");
        assert_eq!(summary.rollback_strength, "high");
        assert!(summary
            .highlighted_actions
            .iter()
            .any(|item| item.contains("consumed:validated_regressing")));
        assert_eq!(
            summary.consumed_target_kinds,
            vec!["execution_candidate".to_string()]
        );
    }

    #[test]
    fn test_derive_decisions_apply_artifact_consumed_gate() {
        let gate = ict_engine::application::decision_utils::ArtifactConsumedDecisionGate {
            status: "validated_regressing".to_string(),
            reason: "label=recent_3_vs_previous_1 regression_thresholds=(-5.00,-0.25)".to_string(),
            target_kinds: vec!["pending_update".to_string()],
        };
        let promotion = derive_promotion_decision(
            &[PersistedFactorRanking {
                factor_name: "trend_momentum".to_string(),
                composite_score: 0.82,
                conformal_coverage_1sigma: 0.80,
                regime_break_penalty: 0.05,
                ..PersistedFactorRanking::default()
            }],
            &[RankingDiffItem {
                factor_name: "trend_momentum".to_string(),
                score_delta: 0.15,
                ..RankingDiffItem::default()
            }],
            &DatasetComparability {
                comparable: true,
                ..DatasetComparability::default()
            },
            &decision_thresholds(),
            Some(&gate),
        );
        let rollback = derive_rollback_recommendation(
            &[PersistedFactorRanking {
                factor_name: "trend_momentum".to_string(),
                composite_score: 0.82,
                conformal_coverage_1sigma: 0.80,
                regime_break_penalty: 0.05,
                ..PersistedFactorRanking::default()
            }],
            &[],
            &[],
            &DatasetComparability {
                comparable: true,
                ..DatasetComparability::default()
            },
            &decision_thresholds(),
            Some(&gate),
        );

        assert!(!promotion.approved);
        assert_eq!(promotion.status, "hold");
        assert!(promotion
            .reason
            .contains("artifact_consumption_validated_regression"));
        assert!(rollback.should_rollback);
        assert_eq!(rollback.scope, "artifact");
        assert!(rollback
            .reason
            .contains("artifact_consumption_validated_regression"));
    }

    #[test]
    fn test_derive_decisions_hold_on_credibility_regression() {
        let rankings = [PersistedFactorRanking {
            factor_name: "fragile_alpha".to_string(),
            composite_score: 0.91,
            conformal_coverage_1sigma: 0.42,
            regime_break_penalty: 0.31,
            ..PersistedFactorRanking::default()
        }];
        let promotion = derive_promotion_decision(
            &rankings,
            &[RankingDiffItem {
                factor_name: "fragile_alpha".to_string(),
                score_delta: 0.20,
                ..RankingDiffItem::default()
            }],
            &DatasetComparability {
                comparable: true,
                ..DatasetComparability::default()
            },
            &decision_thresholds(),
            None,
        );
        let rollback = derive_rollback_recommendation(
            &rankings,
            &[],
            &[],
            &DatasetComparability {
                comparable: true,
                ..DatasetComparability::default()
            },
            &decision_thresholds(),
            None,
        );
        assert!(!promotion.approved);
        assert_eq!(promotion.status, "hold");
        assert!(promotion.reason.contains("conformal_coverage_low"));
        assert!(rollback.should_rollback);
        assert!(
            rollback.reason.contains("conformal_coverage_low")
                || rollback.reason.contains("regime_break_penalty_high")
        );
    }

    #[test]
    fn test_derive_family_outcomes_apply_artifact_family_consumed_gate() {
        let outcomes = derive_family_outcomes(
            &[FactorFamilyDecision {
                family: "trend_momentum".to_string(),
                avg_score: 0.78,
                replacement_candidates: Vec::new(),
                actions: vec!["trend_factor:keep".to_string()],
                ..FactorFamilyDecision::default()
            }],
            &decision_thresholds(),
            &DatasetComparability {
                comparable: true,
                ..DatasetComparability::default()
            },
            Some(&[ict_engine::state::ArtifactFamilyTrendSummary {
                family: "trend_momentum".to_string(),
                consumed_entries: 4,
                consumed_validation_status: "validated_regressing".to_string(),
                consumed_validation_reason:
                    "label=recent_3_vs_previous_1 regression_thresholds=(-5.00,-0.25)".to_string(),
                ..ict_engine::state::ArtifactFamilyTrendSummary::default()
            }]),
        );

        assert_eq!(outcomes[0].promotion_decision.status, "hold");
        assert!(!outcomes[0].promotion_decision.approved);
        assert!(outcomes[0]
            .promotion_decision
            .reason
            .contains("family_artifact_consumption_validated_regression"));
        assert!(outcomes[0].rollback_recommendation.should_rollback);
        assert_eq!(outcomes[0].rollback_recommendation.scope, "family_artifact");
    }

    #[test]
    fn test_augment_action_plan_with_artifact_trends_adds_consumed_validation_item() {
        let mut plan = AgentActionPlan::default();
        let consumed_impact_summary = ict_engine::state::ArtifactConsumedImpactSummary {
            total_consumed: 4,
            trend_comparisons: vec![ict_engine::state::ArtifactConsumedImpactTrendComparison {
                label: "recent_3_vs_previous_1".to_string(),
                recent: ict_engine::state::ArtifactConsumedImpactWindow {
                    label: "recent_3".to_string(),
                    count: 3,
                    positive: 0,
                    negative: 2,
                    neutral: 1,
                    average_quality_score: 41.0,
                    cumulative_quality_delta: -18,
                },
                baseline: ict_engine::state::ArtifactConsumedImpactWindow {
                    label: "previous_1".to_string(),
                    count: 1,
                    positive: 1,
                    negative: 0,
                    neutral: 0,
                    average_quality_score: 83.0,
                    cumulative_quality_delta: 0,
                },
                average_quality_score_delta: -42.0,
                cumulative_quality_delta_delta: -18,
                positive_rate_delta: -1.0,
                conclusion: "regressing".to_string(),
            }],
            by_kind_trend_comparisons: BTreeMap::from([(
                "pending_update".to_string(),
                vec![ict_engine::state::ArtifactConsumedImpactTrendComparison {
                    label: "recent_3_vs_previous_1".to_string(),
                    recent: ict_engine::state::ArtifactConsumedImpactWindow {
                        label: "recent_3".to_string(),
                        count: 3,
                        positive: 0,
                        negative: 2,
                        neutral: 1,
                        average_quality_score: 41.0,
                        cumulative_quality_delta: -18,
                    },
                    baseline: ict_engine::state::ArtifactConsumedImpactWindow {
                        label: "previous_1".to_string(),
                        count: 1,
                        positive: 1,
                        negative: 0,
                        neutral: 0,
                        average_quality_score: 83.0,
                        cumulative_quality_delta: 0,
                    },
                    average_quality_score_delta: -42.0,
                    cumulative_quality_delta_delta: -18,
                    positive_rate_delta: -1.0,
                    conclusion: "regressing".to_string(),
                }],
            )]),
            ..ict_engine::state::ArtifactConsumedImpactSummary::default()
        };

        ict_engine::application::backtest::augment_action_plan_with_artifact_trends(
            &mut plan,
            "NQ",
            "state",
            &[],
            &[],
            &consumed_impact_summary,
        );

        assert!(plan.items.iter().any(|item| {
            item.stage == "artifact_consumption_review"
                && item.blocking
                && item
                    .suggested_commands
                    .iter()
                    .any(|command| command.contains("--symbol NQ"))
                && item
                    .expected_state_changes
                    .iter()
                    .any(|change| change.target == "artifact_kind:pending_update")
        }));
    }

    #[test]
    fn test_concretize_action_plan_commands_replaces_template_commands() {
        let mut plan = AgentActionPlan {
            summary: "test".to_string(),
            items: vec![
                AgentActionItem {
                    stage: "promotion".to_string(),
                    suggested_commands: vec!["ict-engine factor-research --data <file>".to_string()],
                    ..AgentActionItem::default()
                },
                AgentActionItem {
                    stage: "iteration".to_string(),
                    suggested_commands: vec!["ict-engine factor-backtest --data <file>".to_string()],
                    ..AgentActionItem::default()
                },
            ],
        };
        let commands = CommandRecommendations {
            research: RecommendedCommand {
                command:
                    "ict-engine factor-research --symbol NQ --data /tmp/ltf.json --state-dir state"
                        .to_string(),
                ready: true,
                ..RecommendedCommand::default()
            },
            backtest: RecommendedCommand {
                command:
                    "ict-engine factor-backtest --symbol NQ --data /tmp/ltf.json --state-dir state"
                        .to_string(),
                ready: true,
                ..RecommendedCommand::default()
            },
            ..CommandRecommendations::default()
        };

        concretize_action_plan_commands(&mut plan, &commands);

        assert_eq!(
            plan.items[0].suggested_commands[0],
            "ict-engine factor-research --symbol NQ --data /tmp/ltf.json --state-dir state"
        );
        assert_eq!(
            plan.items[1].suggested_commands[0],
            "ict-engine factor-backtest --symbol NQ --data /tmp/ltf.json --state-dir state"
        );
        assert!(plan.items[0]
            .suggested_commands
            .iter()
            .all(|command| !command.contains("<file>")));
    }

    #[test]
    fn test_build_artifact_factor_trends_exposes_consumed_validation() {
        let trends = build_artifact_factor_trends(
            &[
                ArtifactLedgerEntry {
                    artifact_id: "f1".to_string(),
                    artifact_kind: "pending_update".to_string(),
                    generated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                    top_factor_name: Some("trend_momentum".to_string()),
                    top_factor_action: Some("keep".to_string()),
                    consumed_by_update_run_id: Some("u1".to_string()),
                    consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()),
                    consumption_regrade_status: Some("validated_positive".to_string()),
                    quality_score: 85,
                    ..ArtifactLedgerEntry::default()
                },
                ArtifactLedgerEntry {
                    artifact_id: "f2".to_string(),
                    artifact_kind: "pending_update".to_string(),
                    generated_at: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                    top_factor_name: Some("trend_momentum".to_string()),
                    top_factor_action: Some("keep".to_string()),
                    consumed_by_update_run_id: Some("u2".to_string()),
                    consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()),
                    consumption_regrade_status: Some("validated_negative".to_string()),
                    quality_score: 45,
                    ..ArtifactLedgerEntry::default()
                },
                ArtifactLedgerEntry {
                    artifact_id: "f3".to_string(),
                    artifact_kind: "pending_update".to_string(),
                    generated_at: Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(),
                    top_factor_name: Some("trend_momentum".to_string()),
                    top_factor_action: Some("replace".to_string()),
                    consumed_by_update_run_id: Some("u3".to_string()),
                    consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap()),
                    consumption_regrade_status: Some("validated_negative".to_string()),
                    quality_score: 35,
                    ..ArtifactLedgerEntry::default()
                },
                ArtifactLedgerEntry {
                    artifact_id: "f4".to_string(),
                    artifact_kind: "pending_update".to_string(),
                    generated_at: Utc.with_ymd_and_hms(2024, 1, 4, 0, 0, 0).unwrap(),
                    top_factor_name: Some("trend_momentum".to_string()),
                    top_factor_action: Some("replace".to_string()),
                    consumed_by_update_run_id: Some("u4".to_string()),
                    consumed_at: Some(Utc.with_ymd_and_hms(2024, 1, 4, 0, 0, 0).unwrap()),
                    consumption_regrade_status: Some("validated_negative".to_string()),
                    quality_score: 30,
                    ..ArtifactLedgerEntry::default()
                },
            ],
            &None,
            &None,
            &None,
        );

        assert_eq!(trends[0].factor_name, "trend_momentum");
        assert_eq!(trends[0].consumed_validation_status, "validated_regressing");
        assert_eq!(trends[0].decision_status, "rollback_watch");
        assert!(trends[0]
            .consumed_validation_reason
            .contains("regression_thresholds"));
    }

    #[test]
    fn test_append_artifact_decision_prompt_adds_artifact_consumption_prompt() {
        let mut pack = AgentPromptPack::default();
        append_artifact_decision_prompt(
            &mut pack,
            "NQ",
            &ict_engine::state::ArtifactDecisionSection {
                summary: ict_engine::state::ArtifactDecisionSummary {
                    consumed_trend_status: "validated_regressing".to_string(),
                    consumed_trend_reason: "regression".to_string(),
                    consumed_target_kinds: vec!["pending_update".to_string()],
                    ..ict_engine::state::ArtifactDecisionSummary::default()
                },
                top_consumed_trend_comparisons: vec![
                    ict_engine::state::ArtifactConsumedImpactTrendComparison {
                        label: "recent_3_vs_previous_1".to_string(),
                        conclusion: "regressing".to_string(),
                        average_quality_score_delta: -20.0,
                        positive_rate_delta: -0.5,
                        ..ict_engine::state::ArtifactConsumedImpactTrendComparison::default()
                    },
                ],
                ..ict_engine::state::ArtifactDecisionSection::default()
            },
        );

        assert!(pack
            .prompts
            .iter()
            .any(|prompt| prompt.id == "artifact_consumption_review"));
    }

    #[test]
    fn test_build_analyze_agent_prompts_adds_pre_bayes_soft_evidence_prompt() {
        let prompts = build_analyze_agent_prompts(BuildAnalyzeAgentPromptsInput {
            symbol: "NQ",
            decision: &ProbabilisticDecisionSnapshot {
                long_score: 0.4,
                short_score: 0.2,
                win_prob_long: 0.55,
                win_prob_short: 0.45,
                ict_support_long: 0.4,
                ict_support_short: 0.2,
                selected_direction: Direction::Bull,
                selected_score: 0.4,
                selected_win_probability: 0.55,
                ict_role: "test".to_string(),
            },
            factor_diagnostics: &FactorDiagnostics::default(),
            pre_bayes_evidence_filter: &PreBayesEvidenceFilter {
                uses_soft_evidence: true,
                filtered_market_regime_label: "range".to_string(),
                filtered_liquidity_context_label: "neutral".to_string(),
                filtered_factor_alignment: "mixed".to_string(),
                filtered_factor_uncertainty: "high".to_string(),
                soft_market_regime_distribution: BTreeMap::from([
                    ("bull".to_string(), 0.2),
                    ("bear".to_string(), 0.2),
                    ("range".to_string(), 0.6),
                ]),
                soft_liquidity_context_distribution: BTreeMap::from([
                    ("favorable".to_string(), 0.2),
                    ("neutral".to_string(), 0.6),
                    ("hostile".to_string(), 0.2),
                ]),
                soft_factor_alignment_distribution: BTreeMap::from([
                    ("bullish".to_string(), 0.2),
                    ("mixed".to_string(), 0.6),
                    ("bearish".to_string(), 0.2),
                ]),
                soft_factor_uncertainty_distribution: BTreeMap::from([
                    ("low".to_string(), 0.3),
                    ("high".to_string(), 0.7),
                ]),
                ..PreBayesEvidenceFilter::default()
            },
            factor_ranking: &[],
            factor_iteration_queue: &[],
            feedback_history_summary: &FeedbackHistorySummary::default(),
            trade_plan: &TradePlan {
                symbol: Symbol::NQ,
                direction: Direction::Bull,
                entry: 100.0,
                stop_loss: 99.0,
                tp1: 101.0,
                tp2: 102.0,
                tp3: 103.0,
                risk_reward: 1.0,
                kelly_fraction: 0.1,
                position_size: 10.0,
                regime: Regime::ManipulationExpansion,
                posterior: 0.55,
                win_probability: 0.55,
                cascade_bull: ict_engine::types::CascadeResult {
                    direction: Direction::Bull,
                    stopped_at: None,
                    steps: Vec::new(),
                    final_posterior: 0.55,
                },
                cascade_bear: ict_engine::types::CascadeResult {
                    direction: Direction::Bear,
                    stopped_at: None,
                    steps: Vec::new(),
                    final_posterior: 0.45,
                },
                uncertainties: Vec::new(),
            },
            dataset_comparability: &DatasetComparability::default(),
            decision_hint: "hint",
            multi_timeframe_summary: &["higher_timeframe_direction_bias=bullish".to_string()],
        });

        assert!(prompts
            .prompts
            .iter()
            .any(|prompt| prompt.id == "pre_bayes_soft_evidence_review"));
        assert!(prompts
            .prompts
            .iter()
            .find(|prompt| prompt.id == "analysis_market_review")
            .map(|prompt| prompt
                .user_prompt
                .contains("higher_timeframe_direction_bias=bullish"))
            .unwrap_or(false));
    }

    #[test]
    fn test_workflow_snapshot_uses_full_ledger_for_actionable_artifacts() {
        let ledger = (0..11)
            .map(|index| ArtifactLedgerEntry {
                artifact_id: format!("artifact-{}", index),
                artifact_kind: if index % 2 == 0 {
                    "pending_update".to_string()
                } else {
                    "execution_candidate".to_string()
                },
                generated_at: Utc
                    .with_ymd_and_hms(2024, 1, 1 + index as u32, 0, 0, 0)
                    .unwrap(),
                actionable: index == 0,
                promote_candidate: index == 0,
                ..ArtifactLedgerEntry::default()
            })
            .collect::<Vec<_>>();

        let snapshot = build_workflow_snapshot(BuildWorkflowSnapshotInput {
            state_dir: "state",
            symbol: "NQ",
            latest_train: None,
            latest_analyze: None,
            latest_research: None,
            latest_backtest: None,
            latest_update: None,
            pre_bayes_policy_history: &[],
            pending_update_history: &[],
            execution_candidate_history: &[],
            artifact_ledger: &ledger,
        });

        assert_eq!(snapshot.recent_artifacts.len(), 10);
        assert_eq!(snapshot.actionable_artifacts.len(), 1);
        assert_eq!(
            snapshot
                .latest_promotable_artifact
                .as_ref()
                .map(|entry| entry.artifact_id.as_str()),
            Some("artifact-0")
        );
        assert!(!snapshot
            .recent_artifacts
            .iter()
            .any(|entry| entry.artifact_id == "artifact-0"));
        assert_eq!(
            snapshot.artifact_decision_summary.consumed_trend_status,
            "no_consumed_validation"
        );
    }

    #[test]
    fn test_command_recommendations_for_live_context_use_persisted_paths() {
        let commands = command_recommendations(&CommandContext {
            symbol: "NQ".to_string(),
            state_dir: "state".to_string(),
            analyze: Some(AnalyzeCommandSource::Live {
                source: Box::new(LiveDataSourceProvenance {
                    futures_backend: "openbb".to_string(),
                    aux_backend: "openalice".to_string(),
                    futures_base_url: "http://127.0.0.1:8080".to_string(),
                    aux_base_url: "http://127.0.0.1:6901/api/v1".to_string(),
                    futures_symbol: "NQ".to_string(),
                    spot_symbol: "QQQ".to_string(),
                    options_symbol: "QQQ".to_string(),
                    spot_kind: "equity".to_string(),
                    fetched_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                    persisted_htf_path: Some("/tmp/htf.json".to_string()),
                    persisted_h4_path: Some("/tmp/h4.json".to_string()),
                    persisted_mtf_path: Some("/tmp/mtf.json".to_string()),
                    persisted_m5_path: Some("/tmp/m5.json".to_string()),
                    persisted_ltf_path: Some("/tmp/ltf.json".to_string()),
                    persisted_m1_path: Some("/tmp/m1.json".to_string()),
                    persisted_spot_path: Some("/tmp/spot.json".to_string()),
                }),
            }),
            research_data: Some("/tmp/ltf.json".to_string()),
            paired_data: Some("/tmp/spot.json".to_string()),
            update_outcome: None,
            update_entry_signal: None,
            update_feedback_file: None,
            user_data_selection_required: true,
        });

        assert!(!commands.research.ready);
        assert!(!commands.backtest.ready);
        assert!(commands.research.command.contains("/tmp/ltf.json"));
        assert!(commands.backtest.command.contains("/tmp/spot.json"));
        assert!(commands.analyze.command.contains("analyze-live"));
        assert!(commands.research.user_data_selection_required);
        assert!(commands.backtest.user_data_selection_required);
        assert!(commands
            .research
            .missing_inputs
            .contains(&"user_selected_historical_data".to_string()));
        assert!(commands
            .research
            .user_data_selection_prompt
            .contains("ask the user"));
        assert!(commands
            .research
            .recorded_data_paths
            .contains(&"/tmp/ltf.json".to_string()));
    }

    #[test]
    fn test_build_feedback_record_keeps_trade_timestamp() {
        let timestamp = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
        let feedback = ict_engine::application::backtest::build_feedback_record(
            ict_engine::application::backtest::BuildFeedbackRecordInput {
                symbol: "NQ",
                source: "test",
                timestamp,
                factor_diagnostics: &FactorDiagnostics {
                    bullish_factors: vec![ict_engine::factor_lab::FactorContribution {
                        factor_name: "trend_momentum".to_string(),
                        category: "trend_momentum".to_string(),
                        direction: Direction::Bull,
                        value: 0.7,
                        confidence: 0.8,
                        weighted_score: 0.25,
                        uncertainty_contribution: 0.1,
                        explanation: "test".to_string(),
                    }],
                    long_support: 0.4,
                    short_support: 0.1,
                    uncertainty: 0.2,
                    ..FactorDiagnostics::default()
                },
                decision: &ProbabilisticDecisionSnapshot {
                    long_score: 0.6,
                    short_score: 0.3,
                    win_prob_long: 0.58,
                    win_prob_short: 0.42,
                    ict_support_long: 0.7,
                    ict_support_short: 0.3,
                    selected_direction: Direction::Bull,
                    selected_score: 0.6,
                    selected_win_probability: 0.58,
                    ict_role: "evidence_only_non_deterministic".to_string(),
                },
                pnl: 0.02,
                realized_outcome: "win".to_string(),
                regime_at_entry: Regime::ManipulationExpansion,
            },
        );

        assert_eq!(feedback.timestamp, timestamp);
        assert_eq!(feedback.factors_used.len(), 1);
    }

    #[test]
    fn test_apply_feedback_to_trade_outcome_network_updates_cpt() {
        let mut network = build_trading_network().unwrap();
        let before = network.nodes["trade_outcome"].cpt.entries[&vec![0, 0, 0]][0];
        let feedback = FeedbackRecord {
            timestamp: Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap(),
            symbol: "NQ".to_string(),
            source: "factor_research_backtest".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: vec![FeedbackFactorUsage {
                factor_name: "trend_momentum".to_string(),
                category: "factor_backtest".to_string(),
                direction: Direction::Bull,
                value: 0.8,
                confidence: 0.8,
                weight: 0.3,
                long_support: 0.3,
                short_support: 0.0,
                uncertainty_contribution: 0.1,
            }],
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: Direction::Bull,
                selected_probability: 0.8,
                long_score: 0.3,
                short_score: 0.0,
                win_prob_long: 0.8,
                win_prob_short: 0.0,
                uncertainty: 0.1,
            },
            realized_outcome: "win".to_string(),
            pnl: 0.02,
            regime_at_entry: Regime::ManipulationExpansion,
        };

        let updates = ict_engine::application::backtest::apply_feedback_to_trade_outcome_network(
            &mut network,
            &[feedback],
        )
        .unwrap();
        let after = network.nodes["trade_outcome"].cpt.entries[&vec![0, 0, 0]][0];

        assert_eq!(updates, 1);
        assert!(after > before);
    }

    #[test]
    fn test_build_update_agent_prompts_contains_feedback_review_stage() {
        let prompts = build_update_agent_prompts(BuildUpdateAgentPromptsInput {
            symbol: "NQ",
            factor_ranking: &[],
            factor_iteration_queue: &[],
            feedback_history_summary: &FeedbackHistorySummary::default(),
            updated_trade_outcome: &BTreeMap::from([
                ("win".to_string(), 0.6),
                ("breakeven".to_string(), 0.2),
                ("loss".to_string(), 0.2),
            ]),
            normalized_entry_quality: "high",
            factor_alignment: "bullish",
            factor_uncertainty: "low",
            realized_outcome: "win",
            feedback_records_applied: 1,
            consumed_pre_bayes_evidence_filter: None,
            consumed_pre_bayes_entry_quality_bridge: None,
            consumed_multi_timeframe_summary: &[],
        });

        assert!(!prompts.prompts.is_empty());
        assert_eq!(prompts.prompts[0].id, "update_feedback_review");
        assert_eq!(prompts.prompts[0].stage, "feedback_update");
    }

    #[test]
    fn test_dataset_audit_prompt_is_added_to_research_prompt_pack() {
        let prompt = dataset_audit_prompt("NQ", "data.json", None, 140, None, "factor-research");
        assert_eq!(prompt.id, "dataset_audit");
        assert_eq!(prompt.stage, "dataset_audit");
        assert!(prompt.user_prompt.contains("data.json"));
    }

    #[test]
    fn test_ranking_diffs_reports_score_and_weight_changes() {
        let previous = vec![PersistedFactorRanking {
            factor_name: "trend_momentum".to_string(),
            composite_score: 0.40,
            weight: 0.20,
            iteration_action: "tune".to_string(),
            ..PersistedFactorRanking::default()
        }];
        let current = vec![PersistedFactorRanking {
            factor_name: "trend_momentum".to_string(),
            composite_score: 0.65,
            weight: 0.32,
            iteration_action: "keep".to_string(),
            ..PersistedFactorRanking::default()
        }];

        let diff = ict_engine::application::backtest::ranking_diffs(&previous, &current);
        assert_eq!(diff.len(), 1);
        assert!(diff[0].score_delta > 0.0);
        assert!(diff[0].weight_delta > 0.0);
        assert_eq!(diff[0].previous_action.as_deref(), Some("tune"));
        assert_eq!(diff[0].new_action, "keep");
    }

    #[test]
    fn test_probability_diffs_reports_before_after_delta() {
        let previous = Some(BTreeMap::from([
            ("win".to_string(), 0.50),
            ("breakeven".to_string(), 0.20),
            ("loss".to_string(), 0.30),
        ]));
        let current = BTreeMap::from([
            ("win".to_string(), 0.58),
            ("breakeven".to_string(), 0.18),
            ("loss".to_string(), 0.24),
        ]);

        let diff = ict_engine::application::backtest::probability_diffs(&previous, &current);
        assert_eq!(diff.len(), 3);
        assert!(diff
            .iter()
            .any(|item| item.state == "win" && item.delta > 0.0));
        assert!(diff
            .iter()
            .any(|item| item.state == "loss" && item.delta < 0.0));
    }

    #[test]
    fn test_build_analyze_decision_hint_for_non_comparable_data() {
        let hint = ict_engine::application::decision_utils::build_analyze_decision_hint(
            &DatasetComparability {
                comparable: false,
                previous_run_id: Some("run-1".to_string()),
                reason: "different_data_fingerprint".to_string(),
                comparison_class: "different_data_fingerprint".to_string(),
                ..DatasetComparability::default()
            },
            &[],
            &FactorDiagnostics::default(),
        );
        assert_eq!(
            hint,
            "Observe only: current run not comparable to last analyze (different_data_fingerprint)."
        );
    }

    #[test]
    fn test_build_analyze_decision_hint_for_high_uncertainty() {
        let hint = ict_engine::application::decision_utils::build_analyze_decision_hint(
            &DatasetComparability {
                comparable: true,
                ..DatasetComparability::default()
            },
            &[],
            &FactorDiagnostics {
                uncertainty: 0.52,
                ..FactorDiagnostics::default()
            },
        );
        assert_eq!(
            hint,
            "Wait: evidence uncertainty remains high; defer action until structure clears."
        );
    }

    #[test]
    fn test_build_analyze_decision_hint_for_factor_backlog() {
        let hint = ict_engine::application::decision_utils::build_analyze_decision_hint(
            &DatasetComparability {
                comparable: true,
                ..DatasetComparability::default()
            },
            &[FactorIterationPrompt {
                factor_name: "structure_ict".to_string(),
                iteration_action: "tune".to_string(),
                ..FactorIterationPrompt::default()
            }],
            &FactorDiagnostics::default(),
        );
        assert_eq!(
            hint,
            "Comparable run, but factor backlog remains: tune structure_ict first."
        );
    }

    #[test]
    fn test_build_analyze_decision_hint_for_stable_factor_stack() {
        let hint = ict_engine::application::decision_utils::build_analyze_decision_hint(
            &DatasetComparability {
                comparable: true,
                ..DatasetComparability::default()
            },
            &[],
            &FactorDiagnostics::default(),
        );
        assert_eq!(
            hint,
            "Comparable run and factor stack stable; no immediate factor action required."
        );
    }

    #[test]
    fn test_append_pda_sequence_hint_marks_weak_cluster() {
        let hint = ict_engine::application::decision_utils::append_pda_sequence_hint(
            "Comparable run and factor stack stable; no immediate factor action required.",
            Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(1),
                primary_cluster_label: Some("cluster_1".to_string()),
                primary_cluster_family: Some("range".to_string()),
                primary_cluster_confidence: Some(0.41),
                consistency_ratio: 0.45,
                ensemble_mean_confidence: 0.50,
                valid_sessions: 8,
                kmer_k: 2,
            }),
            &PreBayesEvidenceFilter {
                conflict_flags: vec!["pda_sequence_cluster_weak".to_string()],
                ..PreBayesEvidenceFilter::default()
            },
        );
        assert!(hint.contains("|pda_sequence=weak_cluster:cluster_1:0.410:0.450"));
    }

    #[test]
    fn test_append_pda_sequence_hint_marks_reinforcing_cluster() {
        let hint = ict_engine::application::decision_utils::append_pda_sequence_hint(
            "Comparable run and factor stack stable; no immediate factor action required.",
            Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(1),
                primary_cluster_label: Some("cluster_1".to_string()),
                primary_cluster_family: Some("trend".to_string()),
                primary_cluster_confidence: Some(0.88),
                consistency_ratio: 0.75,
                ensemble_mean_confidence: 0.83,
                valid_sessions: 8,
                kmer_k: 2,
            }),
            &PreBayesEvidenceFilter::default(),
        );
        assert!(hint.contains("|pda_sequence=reinforcing_cluster:cluster_1:0.880:0.750"));
    }

    #[test]
    fn test_append_pda_sequence_hint_marks_regime_disagreement() {
        let hint = ict_engine::application::decision_utils::append_pda_sequence_hint(
            "Comparable run and factor stack stable; no immediate factor action required.",
            Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(0),
                primary_cluster_label: Some("cluster_0".to_string()),
                primary_cluster_family: Some("trend".to_string()),
                primary_cluster_confidence: Some(0.92),
                consistency_ratio: 0.82,
                ensemble_mean_confidence: 0.85,
                valid_sessions: 8,
                kmer_k: 2,
            }),
            &PreBayesEvidenceFilter {
                conflict_flags: vec!["pda_regime_family_disagreement".to_string()],
                ..PreBayesEvidenceFilter::default()
            },
        );
        assert!(hint.contains("|pda_sequence=regime_disagreement:cluster_0:trend:0.920"));
    }

    #[test]
    fn test_apply_regime_execution_guardrail_blocks_on_high_transition_hazard() {
        let output = apply_regime_execution_guardrail(
            ict_engine::application::orchestration::ExecutionTreeOutput {
                gate_status: "ready".to_string(),
                branch: "fill_viable".to_string(),
                execution_bias: "aggressive".to_string(),
                branch_probability: 0.72,
                posterior_uncertainty: 0.30,
                decision_hint: "execution_first_fill".to_string(),
                ..ict_engine::application::orchestration::ExecutionTreeOutput::default()
            },
            &RegimeSegmentationPacket {
                method: "hybrid_regime_first_pass_v1".to_string(),
                segmentation_version: "v2".to_string(),
                active_regime_cluster: Some("trend_impulse".to_string()),
                transition_hazard: Some(0.78),
                duration_elapsed_bars: Some(4),
                duration_model: Some("negative_binomial".to_string()),
                duration_remaining_expected_bars: Some(2.0),
                regime_membership: BTreeMap::new(),
                feature_attribution: BTreeMap::new(),
                evidence: Vec::new(),
                wasserstein_label: Some("trend_impulse".to_string()),
                wasserstein_distance: Some(0.12),
                governor_confidence: Some(0.70),
                governor_entropy: Some(0.90),
                governor_min_hold_active: Some(false),
                timeframe_alignment: Some(true),
                timeframe_alignment_score: Some(1.0),
            },
        );
        assert_eq!(output.gate_status, "observe");
        assert_eq!(output.branch, "transition_guardrail");
        assert_eq!(
            output.decision_hint,
            "execution_guarded_due_to_high_transition_hazard"
        );
    }

    #[test]
    fn test_apply_regime_execution_guardrail_blocks_on_pda_hybrid_disagreement() {
        let output = apply_regime_execution_guardrail(
            ict_engine::application::orchestration::ExecutionTreeOutput {
                gate_status: "ready".to_string(),
                branch: "fill_viable".to_string(),
                execution_bias: "aggressive".to_string(),
                branch_probability: 0.72,
                posterior_uncertainty: 0.30,
                decision_hint: "execution_first_fill".to_string(),
                ..ict_engine::application::orchestration::ExecutionTreeOutput::default()
            },
            &RegimeSegmentationPacket {
                method: "hybrid_regime_first_pass_v1".to_string(),
                segmentation_version: "v2".to_string(),
                active_regime_cluster: Some("trend_impulse".to_string()),
                transition_hazard: Some(0.22),
                duration_elapsed_bars: Some(2),
                duration_model: Some("negative_binomial".to_string()),
                duration_remaining_expected_bars: Some(4.0),
                regime_membership: BTreeMap::new(),
                feature_attribution: BTreeMap::new(),
                evidence: vec!["pda_hybrid_alignment=false".to_string()],
                wasserstein_label: Some("trend_impulse".to_string()),
                wasserstein_distance: Some(0.12),
                governor_confidence: Some(0.70),
                governor_entropy: Some(0.90),
                governor_min_hold_active: Some(false),
                timeframe_alignment: Some(true),
                timeframe_alignment_score: Some(1.0),
            },
        );
        assert_eq!(output.gate_status, "observe");
        assert_eq!(output.branch, "transition_guardrail");
        assert_eq!(
            output.decision_hint,
            "execution_guarded_due_to_pda_hybrid_disagreement"
        );
    }

    #[test]
    fn test_apply_regime_execution_guardrail_blocks_on_low_remaining_duration() {
        let output = apply_regime_execution_guardrail(
            ict_engine::application::orchestration::ExecutionTreeOutput {
                gate_status: "ready".to_string(),
                branch: "fill_viable".to_string(),
                execution_bias: "aggressive".to_string(),
                branch_probability: 0.72,
                posterior_uncertainty: 0.30,
                decision_hint: "execution_first_fill".to_string(),
                ..ict_engine::application::orchestration::ExecutionTreeOutput::default()
            },
            &RegimeSegmentationPacket {
                method: "hybrid_regime_first_pass_v1".to_string(),
                segmentation_version: "v2".to_string(),
                active_regime_cluster: Some("trend_impulse".to_string()),
                transition_hazard: Some(0.22),
                duration_elapsed_bars: Some(6),
                duration_model: Some("negative_binomial".to_string()),
                duration_remaining_expected_bars: Some(1.2),
                regime_membership: BTreeMap::new(),
                feature_attribution: BTreeMap::new(),
                evidence: Vec::new(),
                wasserstein_label: Some("trend_impulse".to_string()),
                wasserstein_distance: Some(0.12),
                governor_confidence: Some(0.70),
                governor_entropy: Some(0.90),
                governor_min_hold_active: Some(false),
                timeframe_alignment: Some(true),
                timeframe_alignment_score: Some(1.0),
            },
        );
        assert_eq!(output.gate_status, "observe");
        assert_eq!(
            output.decision_hint,
            "execution_guarded_due_to_low_remaining_regime_duration"
        );
    }

    #[test]
    fn test_apply_duration_sizing_adjustment_zeroes_size_for_tight_duration() {
        let adjusted = apply_duration_sizing_adjustment(
            TradePlan {
                symbol: Symbol::NQ,
                direction: Direction::Bull,
                entry: 100.0,
                stop_loss: 99.0,
                tp1: 101.0,
                tp2: 102.0,
                tp3: 103.0,
                risk_reward: 1.0,
                kelly_fraction: 0.10,
                position_size: 10.0,
                regime: Regime::ManipulationExpansion,
                posterior: 0.6,
                win_probability: 0.6,
                cascade_bull: ict_engine::types::CascadeResult::default(),
                cascade_bear: ict_engine::types::CascadeResult::default(),
                uncertainties: Vec::new(),
            },
            "NQ",
            &RegimeSegmentationPacket {
                method: "hybrid_regime_first_pass_v1".to_string(),
                segmentation_version: "v2".to_string(),
                active_regime_cluster: Some("trend_impulse".to_string()),
                transition_hazard: Some(0.78),
                duration_elapsed_bars: Some(6),
                duration_model: Some("negative_binomial".to_string()),
                duration_remaining_expected_bars: Some(1.2),
                regime_membership: BTreeMap::new(),
                feature_attribution: BTreeMap::new(),
                evidence: Vec::new(),
                wasserstein_label: Some("trend_impulse".to_string()),
                wasserstein_distance: Some(0.12),
                governor_confidence: Some(0.70),
                governor_entropy: Some(0.90),
                governor_min_hold_active: Some(false),
                timeframe_alignment: Some(true),
                timeframe_alignment_score: Some(1.0),
            },
        );
        assert_eq!(adjusted.kelly_fraction, 0.0);
        assert_eq!(adjusted.position_size, 0.0);
    }

    #[test]
    fn test_apply_duration_sizing_adjustment_scales_down_for_short_remaining_window() {
        let adjusted = apply_duration_sizing_adjustment(
            TradePlan {
                symbol: Symbol::NQ,
                direction: Direction::Bull,
                entry: 100.0,
                stop_loss: 99.0,
                tp1: 101.0,
                tp2: 102.0,
                tp3: 103.0,
                risk_reward: 1.0,
                kelly_fraction: 0.10,
                position_size: 10.0,
                regime: Regime::ManipulationExpansion,
                posterior: 0.6,
                win_probability: 0.6,
                cascade_bull: ict_engine::types::CascadeResult::default(),
                cascade_bear: ict_engine::types::CascadeResult::default(),
                uncertainties: Vec::new(),
            },
            "NQ",
            &RegimeSegmentationPacket {
                method: "hybrid_regime_first_pass_v1".to_string(),
                segmentation_version: "v2".to_string(),
                active_regime_cluster: Some("trend_impulse".to_string()),
                transition_hazard: Some(0.42),
                duration_elapsed_bars: Some(4),
                duration_model: Some("negative_binomial".to_string()),
                duration_remaining_expected_bars: Some(3.0),
                regime_membership: BTreeMap::new(),
                feature_attribution: BTreeMap::new(),
                evidence: Vec::new(),
                wasserstein_label: Some("trend_impulse".to_string()),
                wasserstein_distance: Some(0.12),
                governor_confidence: Some(0.70),
                governor_entropy: Some(0.90),
                governor_min_hold_active: Some(false),
                timeframe_alignment: Some(true),
                timeframe_alignment_score: Some(1.0),
            },
        );
        assert_eq!(adjusted.kelly_fraction, 0.05);
        assert_eq!(adjusted.position_size, 5.0);
    }

    #[test]
    fn test_duration_sizing_scale_is_market_family_aware() {
        assert_eq!(duration_sizing_scale("NQ", "trend", 2.0), 0.25);
        assert_eq!(duration_sizing_scale("GC", "range", 2.0), 0.35);
        assert_eq!(duration_sizing_scale("ES", "transition", 2.0), 0.40);
    }

    #[test]
    fn test_build_duration_surface_from_artifacts_uses_snapshot_and_scale_summary() {
        let snapshot = WorkflowSnapshot {
            latest_backtest: Some(ict_engine::state::WorkflowPhaseSnapshot {
                hybrid_duration_model: Some("negative_binomial".to_string()),
                hybrid_remaining_expected_bars: Some(2.5),
                ..ict_engine::state::WorkflowPhaseSnapshot::default()
            }),
            ..WorkflowSnapshot::default()
        };

        let surface = build_duration_surface_from_artifacts(
            &snapshot,
            &[String::from(
                "duration_sizing_scale=0.25 remaining_expected_bars=2.500 market=NQ family=trend",
            )],
        );

        assert_eq!(surface.len(), 5);
        assert!(surface[0].contains("duration_position_size_delta=-0.7500"));
        assert!(surface[1].contains("duration_kelly_fraction_delta=-0.7500"));
        assert_eq!(surface[2], "duration_sizing_direction=scaled_down");
        assert_eq!(surface[3], "duration_model=negative_binomial");
        assert_eq!(surface[4], "duration_remaining_expected_bars=2.500");
    }

    #[test]
    fn test_build_compact_compare_report_maps_duration_surface_to_comparisons() {
        let compact = ict_engine::application::reporting::build_compact_compare_report(Some(
            &ict_engine::application::backtest::BacktestCompareReport {
                summary: "compare".to_string(),
                shrink_comparison_summary: vec!["coverage_delta=+0.010".to_string()],
                duration_sizing_delta_surface: vec![
                    "duration_sizing_direction=scaled_down".to_string()
                ],
                improvements: vec![],
                regressions: vec!["duration_sizing_scale_delta=-0.750".to_string()],
                recommended_actions: vec!["inspect_duration_constraints".to_string()],
                oos_quality_delta_surface: vec![],
            },
        ))
        .expect("missing compact compare report");

        assert_eq!(
            compact.highlights,
            vec!["coverage_delta=+0.010".to_string()]
        );
        assert_eq!(
            compact.comparisons,
            vec!["duration_sizing_direction=scaled_down".to_string()]
        );
        assert_eq!(
            compact.risks,
            vec!["duration_sizing_scale_delta=-0.750".to_string()]
        );
        assert_eq!(
            compact.next_actions,
            vec!["inspect_duration_constraints".to_string()]
        );
    }

    #[test]
    fn test_human_compare_summary_surfaces_duration_risk_and_next_step() {
        let summary = ict_engine::application::reporting::human_compare_summary(Some(
            &ict_engine::application::backtest::BacktestCompareReport {
                summary: "compare".to_string(),
                shrink_comparison_summary: vec![],
                duration_sizing_delta_surface: vec![
                    "duration_sizing_direction=scaled_down".to_string()
                ],
                improvements: vec![],
                regressions: vec!["duration_sizing_scale_delta=-0.750".to_string()],
                recommended_actions: vec!["inspect_duration_constraints".to_string()],
                oos_quality_delta_surface: vec![],
            },
        ))
        .expect("missing human compare summary");

        assert!(summary.contains("duration_sizing_direction=scaled_down"));
        assert!(summary.contains("risk=duration_sizing_scale_delta=-0.750"));
        assert!(summary.contains("next=inspect_duration_constraints"));
    }

    #[test]
    fn test_human_backtest_compare_summary_labels_backtest_surface() {
        let summary = ict_engine::application::reporting::human_backtest_compare_summary(Some(
            &ict_engine::application::backtest::BacktestCompareReport {
                summary: "compare".to_string(),
                shrink_comparison_summary: vec![],
                duration_sizing_delta_surface: vec![
                    "duration_sizing_direction=scaled_down".to_string()
                ],
                improvements: vec![],
                regressions: vec!["duration_sizing_scale_delta=-0.750".to_string()],
                recommended_actions: vec!["inspect_duration_constraints".to_string()],
                oos_quality_delta_surface: vec![],
            },
        ))
        .expect("missing backtest human compare summary");

        assert!(summary.starts_with("Backtest compare:"));
        assert!(summary.contains("duration_sizing_direction=scaled_down"));
    }

    #[test]
    fn test_human_research_compare_summary_labels_research_surface() {
        let summary = ict_engine::application::reporting::human_research_compare_summary(Some(
            &ict_engine::application::backtest::BacktestCompareReport {
                summary: "compare".to_string(),
                shrink_comparison_summary: vec![],
                duration_sizing_delta_surface: vec![
                    "duration_sizing_direction=scaled_up".to_string()
                ],
                improvements: vec![],
                regressions: vec!["no_material_regressions".to_string()],
                recommended_actions: vec!["promote_duration_profile".to_string()],
                oos_quality_delta_surface: vec![],
            },
        ))
        .expect("missing research human compare summary");

        assert!(summary.starts_with("Research compare:"));
        assert!(summary.contains("duration_sizing_direction=scaled_up"));
    }

    fn sample_compare_report(
        direction: &str,
    ) -> ict_engine::application::backtest::BacktestCompareReport {
        ict_engine::application::backtest::BacktestCompareReport {
            summary: "same_data_same_config".to_string(),
            shrink_comparison_summary: vec!["coverage_delta=+0.010".to_string()],
            duration_sizing_delta_surface: vec![format!("duration_sizing_direction={direction}")],
            improvements: vec![],
            regressions: vec!["duration_sizing_scale_delta=-0.750".to_string()],
            recommended_actions: vec!["inspect_duration_constraints".to_string()],
            oos_quality_delta_surface: vec![],
        }
    }

    #[test]
    fn test_backtest_output_payload_includes_human_compare_summary() {
        let payload = ict_engine::application::reporting::build_backtest_output_payload(
            &BacktestReport {
                symbol: "NQ".to_string(),
                state_dir: "state".to_string(),
                provenance: RunProvenance::default(),
                decision_thresholds: DecisionThresholds::default(),
                dataset_comparability: DatasetComparability::default(),
                promotion_decision: PromotionDecision::default(),
                rollback_recommendation: RollbackRecommendation::default(),
                bars: 140,
                warmup_bars: 50,
                hold_bars: 8,
                spread_bps: 1.0,
                slippage_bps: 1.0,
                fee_bps: 1.0,
                ambiguous_bar_policy: "skip".to_string(),
                window_mode: "rolling".to_string(),
                evidence_policy: "default".to_string(),
                ict_role: "test".to_string(),
                online_learning: false,
                learning_updates: 0,
                signals: 1,
                trades: 1,
                metrics: BacktestMetricsSummary {
                    total_return: 0.0,
                    sharpe: 0.0,
                    max_drawdown: 0.0,
                    win_rate: 0.0,
                    profit_factor: 0.0,
                    conformal_coverage_1sigma: 0.0,
                    conformal_miscoverage_1sigma: 0.0,
                    mean_prediction_interval_half_width: 0.0,
                    worst_window_miscoverage: 0.0,
                    regime_break_penalty: 0.0,
                    structural_break_score: 0.0,
                    structural_break_index: None,
                    structural_break_detected: false,
                    signal_structural_break_score: 0.0,
                    signal_structural_break_index: None,
                    signal_structural_break_detected: false,
                    residual_structural_break_score: 0.0,
                    residual_structural_break_index: None,
                    residual_structural_break_detected: false,
                    rolling_ic_structural_break_score: 0.0,
                    rolling_ic_structural_break_index: None,
                    rolling_ic_structural_break_detected: false,
                },
                equity_curve: vec![],
                regime_metrics: vec![],
                factor_ranking: vec![],
                factor_score_deltas: vec![],
                trade_outcome_deltas: vec![],
                factor_iteration_queue: vec![],
                factor_family_decisions: vec![],
                factor_family_outcomes: vec![],
                factor_family_diffs: vec![],
                factor_family_history: vec![],
                decision_history_summary: DecisionHistorySummary::default(),
                agent_action_plan: AgentActionPlan::default(),
                workflow_state: WorkflowState::default(),
                agent_context_bundle: AgentContextBundle::default(),
                agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
                recommended_commands: CommandRecommendations::default(),
                recommended_next_command: "ict-engine factor-research".to_string(),
                artifact_action_summary: vec![],
                artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary::default(),
                artifact_decision_section: ict_engine::state::ArtifactDecisionSection::default(),
                agent_prompts: AgentPromptPack::default(),
                feedback_history_summary: FeedbackHistorySummary::default(),
                multi_timeframe_summary: vec![],
                last_decision: None,
                final_trade_outcome_cpt: BTreeMap::new(),
                recent_trades: vec![],
                workflow_snapshot: WorkflowSnapshot::default(),
                objective_market_credibility_shrink: None,
            },
            &serde_json::json!({"compact": true}),
            Some(sample_compare_report("scaled_down")),
            "Backtest ran with execution_realism=test and produced 1 trades.".to_string(),
        );

        assert_eq!(
            payload["human_backtest_compare_summary"],
            serde_json::json!(
                "Backtest compare: duration_sizing_direction=scaled_down | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"
            )
        );
        assert!(payload.get("compact_compare_report").is_some());
        assert!(payload.get("backtest_compare_report").is_some());
    }

    #[test]
    fn test_factor_backtest_output_payload_includes_human_compare_summary() {
        let payload = ict_engine::application::reporting::build_factor_backtest_output_payload(
            &FactorBacktestRunResult::default(),
            &serde_json::json!({"compact": true}),
            Some(sample_compare_report("scaled_down")),
            serde_json::json!({"credibility": true}),
            None,
            "ict-engine update --symbol NQ --outcome <win|loss|breakeven> --state-dir state",
        );

        assert_eq!(
            payload["human_backtest_compare_summary"],
            serde_json::json!(
                "Backtest compare: duration_sizing_direction=scaled_down | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"
            )
        );
        assert!(payload.get("compact_compare_report").is_some());
        assert!(payload.get("backtest_compare_report").is_some());
        let human_output = payload["human_output"].as_str().unwrap_or_default();
        assert!(human_output.contains("Factor backtest"));
        assert!(!human_output.contains("\"factor_results\""));
    }

    #[test]
    fn test_factor_backtest_output_payload_includes_suggested_update_command() {
        let expected =
            "ict-engine update --symbol NQ --outcome <win|loss|breakeven> --state-dir state";
        let payload = ict_engine::application::reporting::build_factor_backtest_output_payload(
            &FactorBacktestRunResult::default(),
            &serde_json::json!({"compact": true}),
            None,
            serde_json::json!({"credibility": true}),
            None,
            expected,
        );

        assert_eq!(
            payload["suggested_update_command"],
            serde_json::json!(expected)
        );
    }

    #[test]
    fn test_factor_research_output_payload_includes_human_compare_summary() {
        let payload = ict_engine::application::reporting::build_factor_research_output_payload(
            &serde_json::json!({"report": "factor_research"}),
            Some(sample_compare_report("scaled_up")),
            serde_json::json!({
                "reflection": true,
                "compare_summary": "Research compare: duration_sizing_direction=scaled_up | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"
            }),
            None,
            serde_json::json!({"lifecycle": true}),
        );

        assert_eq!(
            payload["human_research_compare_summary"],
            serde_json::json!(
                "Research compare: duration_sizing_direction=scaled_up | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"
            )
        );
        assert!(payload.get("compact_compare_report").is_some());
        assert!(payload.get("research_compare_report").is_some());
        assert!(payload["reflection_bundle"]
            .to_string()
            .contains("Research compare:"));
    }

    #[test]
    fn test_reporting_module_factor_research_output_payload_includes_human_compare_summary() {
        let payload = ict_engine::application::reporting::build_factor_research_output_payload(
            &serde_json::json!({"report": "factor_research"}),
            Some(sample_compare_report("scaled_up")),
            serde_json::json!({
                "reflection": true,
                "compare_summary": "Research compare: duration_sizing_direction=scaled_up | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"
            }),
            None,
            serde_json::json!({"lifecycle": true}),
        );

        assert_eq!(
            payload["human_research_compare_summary"],
            serde_json::json!(
                "Research compare: duration_sizing_direction=scaled_up | risk=duration_sizing_scale_delta=-0.750 | next=inspect_duration_constraints"
            )
        );
        assert!(payload.get("compact_compare_report").is_some());
        assert!(payload.get("research_compare_report").is_some());
    }

    #[test]
    fn test_render_backtest_human_output_includes_compare_block() {
        let rendered = ict_engine::application::reporting::render_backtest_human_output(
            &BacktestReport {
                symbol: "NQ".to_string(),
                state_dir: "state".to_string(),
                provenance: RunProvenance::default(),
                decision_thresholds: DecisionThresholds::default(),
                dataset_comparability: DatasetComparability {
                    comparable: true,
                    ..DatasetComparability::default()
                },
                promotion_decision: PromotionDecision::default(),
                rollback_recommendation: RollbackRecommendation::default(),
                bars: 140,
                warmup_bars: 50,
                hold_bars: 8,
                spread_bps: 1.0,
                slippage_bps: 1.0,
                fee_bps: 1.0,
                ambiguous_bar_policy: "skip".to_string(),
                window_mode: "rolling".to_string(),
                evidence_policy: "default".to_string(),
                ict_role: "test".to_string(),
                online_learning: false,
                learning_updates: 0,
                signals: 1,
                trades: 1,
                metrics: BacktestMetricsSummary {
                    total_return: 0.0,
                    sharpe: 0.0,
                    max_drawdown: 0.0,
                    win_rate: 0.0,
                    profit_factor: 0.0,
                    conformal_coverage_1sigma: 0.0,
                    conformal_miscoverage_1sigma: 0.0,
                    mean_prediction_interval_half_width: 0.0,
                    worst_window_miscoverage: 0.0,
                    regime_break_penalty: 0.0,
                    structural_break_score: 0.0,
                    structural_break_index: None,
                    structural_break_detected: false,
                    signal_structural_break_score: 0.0,
                    signal_structural_break_index: None,
                    signal_structural_break_detected: false,
                    residual_structural_break_score: 0.0,
                    residual_structural_break_index: None,
                    residual_structural_break_detected: false,
                    rolling_ic_structural_break_score: 0.0,
                    rolling_ic_structural_break_index: None,
                    rolling_ic_structural_break_detected: false,
                },
                equity_curve: vec![],
                regime_metrics: vec![],
                factor_ranking: vec![],
                factor_score_deltas: vec![],
                trade_outcome_deltas: vec![],
                factor_iteration_queue: vec![],
                factor_family_decisions: vec![],
                factor_family_outcomes: vec![],
                factor_family_diffs: vec![],
                factor_family_history: vec![],
                decision_history_summary: DecisionHistorySummary::default(),
                agent_action_plan: AgentActionPlan::default(),
                workflow_state: WorkflowState::default(),
                agent_context_bundle: AgentContextBundle::default(),
                agent_context_bundle_minimal: AgentContextBundleMinimal::default(),
                recommended_commands: CommandRecommendations::default(),
                recommended_next_command: "ict-engine factor-research".to_string(),
                artifact_action_summary: vec![],
                artifact_decision_summary: ict_engine::state::ArtifactDecisionSummary::default(),
                artifact_decision_section: ict_engine::state::ArtifactDecisionSection::default(),
                agent_prompts: AgentPromptPack::default(),
                feedback_history_summary: FeedbackHistorySummary::default(),
                multi_timeframe_summary: vec![],
                last_decision: None,
                final_trade_outcome_cpt: BTreeMap::new(),
                recent_trades: vec![],
                workflow_snapshot: WorkflowSnapshot::default(),
                objective_market_credibility_shrink: None,
            },
            Some(&sample_compare_report("scaled_down")),
        );

        assert!(rendered.contains("Backtest ran with"));
        assert!(rendered.contains("Backtest compare:"));
        assert!(rendered.contains("risk=duration_sizing_scale_delta=-0.750"));
    }

    #[test]
    fn test_render_research_human_output_includes_compare_block() {
        let rendered = ict_engine::application::reporting::render_factor_research_human_output(
            &serde_json::json!({"report": "factor_research"}),
            Some(&sample_compare_report("scaled_up")),
        );

        assert!(rendered.contains("Factor research summary:"));
        assert!(rendered.contains("Research compare:"));
        assert!(rendered.contains("next=inspect_duration_constraints"));
    }

    #[test]
    fn test_resolve_multi_timeframe_inputs_auto_detects_cleaned_siblings() {
        let temp = tempfile::tempdir().unwrap();
        for interval in MULTI_TIMEFRAME_INTERVALS {
            let dir = temp.path().join(format!("cleaned-{}", interval));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("nq.continuous-{}.json", interval)),
                serde_json::to_string(&CleanedCandleOutput {
                    symbol: "NQ".to_string(),
                    candles: sample_candles(8),
                })
                .unwrap(),
            )
            .unwrap();
        }

        let primary = temp
            .path()
            .join("cleaned-15m")
            .join("nq.continuous-15m.json")
            .to_string_lossy()
            .to_string();
        let resolved = resolve_multi_timeframe_inputs(&primary, None, None, None, None, None, None);
        let summary =
            ict_engine::application::multi_timeframe_inputs::build_multi_timeframe_summary(
                &primary, &resolved,
            )
            .unwrap();

        assert_eq!(resolved.source, "auto_from_cleaned_siblings");
        assert_eq!(resolved.paths.len(), MULTI_TIMEFRAME_INTERVALS.len());
        assert!(summary
            .iter()
            .any(|item| item.contains("covered_intervals=1m,5m,15m,1h,4h,1d")));
    }

    #[test]
    fn test_build_multi_timeframe_research_signal_available_via_application_api() {
        let temp = tempfile::tempdir().unwrap();
        for interval in MULTI_TIMEFRAME_INTERVALS {
            let dir = temp.path().join(format!("cleaned-{}", interval));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("nq.continuous-{}.json", interval)),
                serde_json::to_string(&CleanedCandleOutput {
                    symbol: "NQ".to_string(),
                    candles: sample_candles(8),
                })
                .unwrap(),
            )
            .unwrap();
        }

        let primary = temp
            .path()
            .join("cleaned-15m")
            .join("nq.continuous-15m.json")
            .to_string_lossy()
            .to_string();
        let resolved = resolve_multi_timeframe_inputs(&primary, None, None, None, None, None, None);
        let signal =
            ict_engine::application::multi_timeframe_inputs::build_multi_timeframe_research_signal(
                &resolved,
            )
            .unwrap();

        assert!(signal
            .summary
            .iter()
            .any(|item| item.starts_with("higher_timeframe_direction_bias=")));
    }

    #[test]
    fn test_resolve_analyze_cli_inputs_from_data_root() {
        let temp = tempfile::tempdir().unwrap();
        for interval in ["1d", "1h", "15m"] {
            let dir = temp.path().join(format!("cleaned-{}", interval));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("nq.continuous-{}.json", interval)),
                serde_json::to_string(&CleanedCandleOutput {
                    symbol: "NQ".to_string(),
                    candles: sample_candles(40),
                })
                .unwrap(),
            )
            .unwrap();
        }

        let (htf, mtf, ltf) = resolve_analyze_cli_inputs(
            "NQ",
            None,
            None,
            None,
            Some(temp.path().to_str().unwrap()),
            false,
        )
        .unwrap();

        assert!(htf.ends_with("cleaned-1d/nq.continuous-1d.json"));
        assert!(mtf.ends_with("cleaned-1h/nq.continuous-1h.json"));
        assert!(ltf.ends_with("cleaned-15m/nq.continuous-15m.json"));
    }

    #[test]
    fn test_resolve_analyze_cli_inputs_from_demo_flag() {
        let (htf, mtf, ltf) =
            resolve_analyze_cli_inputs("DEMO", None, None, None, None, true).unwrap();

        assert_eq!(htf, "examples/demo/demo-15m.json");
        assert_eq!(mtf, "examples/demo/demo-15m.json");
        assert_eq!(ltf, "examples/demo/demo-15m.json");
    }

    #[test]
    fn test_build_analyze_multi_timeframe_section_parses_summary() {
        let section = build_analyze_multi_timeframe_section(
            &[
                "multi_timeframe_source=auto_from_cleaned_siblings covered_intervals=1m,5m,15m,1h,4h,1d"
                    .to_string(),
                "higher_timeframe_direction_bias=bullish".to_string(),
                "higher_timeframe_alignment_score=0.7500".to_string(),
                "lower_timeframe_entry_alignment_score=0.6200".to_string(),
                "1d:40 bars path=/tmp/1d.json".to_string(),
                "15m:120 bars path=/tmp/15m.json".to_string(),
            ],
            Some(&PreBayesEvidenceFilter {
                filtered_multi_timeframe_resonance_label: "aligned".to_string(),
                ..PreBayesEvidenceFilter::default()
            }),
        );

        assert_eq!(section.direction_bias, "bullish");
        assert_eq!(section.alignment_score, Some(0.75));
        assert_eq!(section.entry_alignment_score, Some(0.62));
        assert_eq!(section.resonance_label, "aligned");
        assert_eq!(section.intervals.len(), 2);
    }

    #[test]
    fn test_build_multi_timeframe_training_observations_uses_all_intervals() {
        let temp = tempfile::tempdir().unwrap();
        for interval in MULTI_TIMEFRAME_INTERVALS {
            let dir = temp.path().join(format!("cleaned-{}", interval));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("nq.continuous-{}.json", interval)),
                serde_json::to_string(&CleanedCandleOutput {
                    symbol: "NQ".to_string(),
                    candles: sample_candles(40),
                })
                .unwrap(),
            )
            .unwrap();
        }

        let primary = temp
            .path()
            .join("cleaned-15m")
            .join("nq.continuous-15m.json")
            .to_string_lossy()
            .to_string();
        let (observations, summary, candles_total) =
            ict_engine::application::regime::build_multi_timeframe_training_observations(&primary)
                .unwrap();

        assert!(candles_total >= 40 * MULTI_TIMEFRAME_INTERVALS.len());
        assert!(!observations.is_empty());
        assert!(summary
            .iter()
            .any(|item| item.contains("train_multi_timeframe_source=auto_from_cleaned_siblings")));
    }

    #[test]
    fn test_find_tomac_root_from_candidates_requires_tomac_layout() {
        let temp = tempfile::tempdir().unwrap();
        let invalid = temp.path().join("invalid_tomac");
        let valid = temp.path().join("valid_tomac");
        std::fs::create_dir_all(&invalid).unwrap();
        let market_dir = valid.join("nq future 2021-2025");
        std::fs::create_dir_all(&market_dir).unwrap();
        std::fs::write(
            market_dir.join("glbx-mdp3-20100606-20260403.ohlcv-1m.csv"),
            "",
        )
        .unwrap();
        std::fs::write(market_dir.join("symbology.csv"), "").unwrap();

        let candidates = vec![
            invalid.to_string_lossy().to_string(),
            valid.to_string_lossy().to_string(),
        ];
        let detected =
            ict_engine::application::multi_timeframe_inputs::find_tomac_root_from_candidates(
                &candidates,
            )
            .unwrap();

        assert_eq!(detected, valid.to_string_lossy());
    }

    #[test]
    fn test_resolve_tomac_root_prefers_explicit_argument() {
        let resolved = ict_engine::application::multi_timeframe_inputs::resolve_tomac_root(Some(
            "/tmp/custom-tomac",
        ))
        .unwrap();
        assert_eq!(resolved, "/tmp/custom-tomac");
    }

    #[test]
    fn test_build_pre_bayes_evidence_filter_neutralizes_conflicting_labels() {
        let filter = build_pre_bayes_evidence_filter(
            &pre_bayes_evidence_policy(),
            "bull",
            "hostile",
            &FactorDiagnostics {
                long_support: 0.30,
                short_support: 0.28,
                uncertainty: 0.52,
                alignment_label: "bearish".to_string(),
                uncertainty_label: "low".to_string(),
                ..FactorDiagnostics::default()
            },
            &ParsedMultiTimeframeEvidence::default(),
            None,
            None,
        );

        assert_eq!(filter.filtered_factor_alignment, "mixed");
        assert_eq!(filter.filtered_factor_uncertainty, "high");
        assert!(!filter.conflict_flags.is_empty());
        assert!(matches!(
            filter.gating_status.as_str(),
            "pass_neutralized" | "observe_only"
        ));
    }

    #[test]
    fn test_build_pre_bayes_evidence_filter_uses_multi_timeframe_conflicts() {
        let filter = build_pre_bayes_evidence_filter(
            &pre_bayes_evidence_policy(),
            "bull",
            "neutral",
            &FactorDiagnostics {
                long_support: 0.34,
                short_support: 0.10,
                uncertainty: 0.20,
                alignment_label: "bullish".to_string(),
                uncertainty_label: "low".to_string(),
                ..FactorDiagnostics::default()
            },
            &ParsedMultiTimeframeEvidence {
                direction_bias: "bearish".to_string(),
                alignment_score: Some(0.42),
                entry_alignment_score: Some(0.35),
                covered_count: 6,
            },
            None,
            None,
        );

        assert!(filter
            .conflict_flags
            .iter()
            .any(|flag| flag == "multi_timeframe_direction_conflict"));
        assert!(filter
            .conflict_flags
            .iter()
            .any(|flag| flag == "multi_timeframe_alignment_weak"));
        assert!(filter
            .conflict_flags
            .iter()
            .any(|flag| flag == "multi_timeframe_entry_alignment_weak"));
        assert_eq!(filter.filtered_factor_alignment, "mixed");
        assert_eq!(filter.filtered_factor_uncertainty, "high");
    }

    #[test]
    fn test_build_pre_bayes_evidence_filter_applies_pda_sequence_quality_modifier() {
        let policy = pre_bayes_evidence_policy();
        let diagnostics = FactorDiagnostics {
            long_support: 0.70,
            short_support: 0.20,
            uncertainty: 0.18,
            alignment_label: "bullish".to_string(),
            uncertainty_label: "low".to_string(),
            ..FactorDiagnostics::default()
        };
        let mtf = ParsedMultiTimeframeEvidence {
            direction_bias: "bullish".to_string(),
            alignment_score: Some(0.80),
            entry_alignment_score: Some(0.78),
            covered_count: 6,
        };
        let no_pda = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "favorable",
            &diagnostics,
            &mtf,
            Some("NQ"),
            None,
        );
        let strong_pda = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "favorable",
            &diagnostics,
            &mtf,
            Some("NQ"),
            Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(1),
                primary_cluster_label: Some("cluster_1".to_string()),
                primary_cluster_family: Some("trend".to_string()),
                primary_cluster_confidence: Some(0.88),
                consistency_ratio: 0.75,
                ensemble_mean_confidence: 0.83,
                valid_sessions: 8,
                kmer_k: 2,
            }),
        );
        let weak_pda = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "favorable",
            &diagnostics,
            &mtf,
            Some("NQ"),
            Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(1),
                primary_cluster_label: Some("cluster_1".to_string()),
                primary_cluster_family: Some("range".to_string()),
                primary_cluster_confidence: Some(0.40),
                consistency_ratio: 0.45,
                ensemble_mean_confidence: 0.52,
                valid_sessions: 8,
                kmer_k: 2,
            }),
        );

        assert!(strong_pda.evidence_quality_score > no_pda.evidence_quality_score);
        assert!(weak_pda.evidence_quality_score < no_pda.evidence_quality_score);
        assert!(weak_pda
            .conflict_flags
            .iter()
            .any(|flag| flag == "pda_sequence_cluster_weak"));
    }

    #[test]
    fn test_build_pre_bayes_evidence_filter_sparse_pda_forces_observe_only() {
        let policy = pre_bayes_evidence_policy();
        let filter = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "favorable",
            &FactorDiagnostics {
                long_support: 0.80,
                short_support: 0.10,
                uncertainty: 0.10,
                alignment_label: "bullish".to_string(),
                uncertainty_label: "low".to_string(),
                ..FactorDiagnostics::default()
            },
            &ParsedMultiTimeframeEvidence {
                direction_bias: "bullish".to_string(),
                alignment_score: Some(0.85),
                entry_alignment_score: Some(0.82),
                covered_count: 6,
            },
            Some("NQ"),
            Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(0),
                primary_cluster_label: Some("cluster_0".to_string()),
                primary_cluster_family: Some("trend".to_string()),
                primary_cluster_confidence: Some(0.90),
                consistency_ratio: 0.88,
                ensemble_mean_confidence: 0.84,
                valid_sessions: 2,
                kmer_k: 2,
            }),
        );
        assert_eq!(filter.gating_status, "observe_only");
        assert!(filter
            .conflict_flags
            .iter()
            .any(|flag| flag == "pda_sequence_sparse_sessions"));
    }

    #[test]
    fn test_build_pre_bayes_evidence_filter_low_consistency_caps_hard_pass() {
        let policy = pre_bayes_evidence_policy();
        let filter = build_pre_bayes_evidence_filter(
            &policy,
            "bull",
            "favorable",
            &FactorDiagnostics {
                long_support: 0.82,
                short_support: 0.08,
                uncertainty: 0.08,
                alignment_label: "bullish".to_string(),
                uncertainty_label: "low".to_string(),
                ..FactorDiagnostics::default()
            },
            &ParsedMultiTimeframeEvidence {
                direction_bias: "bullish".to_string(),
                alignment_score: Some(0.90),
                entry_alignment_score: Some(0.86),
                covered_count: 6,
            },
            Some("NQ"),
            Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(0),
                primary_cluster_label: Some("cluster_0".to_string()),
                primary_cluster_family: Some("trend".to_string()),
                primary_cluster_confidence: Some(0.92),
                consistency_ratio: 0.52,
                ensemble_mean_confidence: 0.85,
                valid_sessions: 8,
                kmer_k: 2,
            }),
        );
        assert_eq!(filter.gating_status, "pass_neutralized");
        assert!(filter
            .conflict_flags
            .iter()
            .any(|flag| flag == "pda_sequence_low_consistency"));
    }

    #[test]
    fn test_build_pre_bayes_evidence_filter_pda_regime_family_disagreement_caps_hard_pass() {
        let policy = pre_bayes_evidence_policy();
        let filter = build_pre_bayes_evidence_filter(
            &policy,
            "range",
            "favorable",
            &FactorDiagnostics {
                long_support: 0.82,
                short_support: 0.08,
                uncertainty: 0.08,
                alignment_label: "bullish".to_string(),
                uncertainty_label: "low".to_string(),
                ..FactorDiagnostics::default()
            },
            &ParsedMultiTimeframeEvidence {
                direction_bias: "bullish".to_string(),
                alignment_score: Some(0.90),
                entry_alignment_score: Some(0.86),
                covered_count: 6,
            },
            Some("NQ"),
            Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                method: "pda_sequence_analysis_v2".to_string(),
                primary_cluster: Some(0),
                primary_cluster_label: Some("cluster_0".to_string()),
                primary_cluster_family: Some("trend".to_string()),
                primary_cluster_confidence: Some(0.92),
                consistency_ratio: 0.82,
                ensemble_mean_confidence: 0.85,
                valid_sessions: 8,
                kmer_k: 2,
            }),
        );
        assert_eq!(filter.gating_status, "pass_neutralized");
        assert!(filter
            .conflict_flags
            .iter()
            .any(|flag| flag == "pda_regime_family_disagreement"));
    }

    #[test]
    fn test_pre_bayes_gate_regression_uses_formal_status_ordering() {
        assert!(ict_engine::application::decision_utils::pre_bayes_gate_is_hard_pass("pass_hard"));
        assert!(
            !ict_engine::application::decision_utils::pre_bayes_gate_is_hard_pass(
                "pass_neutralized"
            )
        );
        assert!(
            ict_engine::application::decision_utils::pre_bayes_gate_regressed(
                "pass_hard",
                "pass_neutralized"
            )
        );
        assert!(
            ict_engine::application::decision_utils::pre_bayes_gate_regressed(
                "pass_neutralized",
                "observe_only"
            )
        );
        assert!(
            !ict_engine::application::decision_utils::pre_bayes_gate_regressed(
                "pass_neutralized",
                "pass_hard"
            )
        );
    }

    #[test]
    fn test_workflow_state_from_pre_bayes_filter_promotes_observe_only_phase() {
        let state = workflow_state_from_pre_bayes_filter(
            WorkflowState {
                phase: "observe_or_deploy".to_string(),
                reason: "base".to_string(),
            },
            &PreBayesEvidenceFilter {
                gating_status: "observe_only".to_string(),
                rationale: vec!["low_quality".to_string()],
                ..PreBayesEvidenceFilter::default()
            },
        );

        assert_eq!(state.phase, "pre_bayes_observe_only");
        assert!(state.reason.contains("low_quality"));
    }

    #[test]
    fn test_workflow_state_from_pre_bayes_filter_promotes_pda_sequence_review_phase() {
        let state = workflow_state_from_pre_bayes_filter(
            WorkflowState {
                phase: "observe_or_deploy".to_string(),
                reason: "base".to_string(),
            },
            &PreBayesEvidenceFilter {
                gating_status: "pass_neutralized".to_string(),
                rationale: vec!["pda weak".to_string()],
                conflict_flags: vec!["pda_sequence_cluster_weak".to_string()],
                ..PreBayesEvidenceFilter::default()
            },
        );

        assert_eq!(state.phase, "pda_sequence_review");
        assert!(state.reason.contains("pda weak"));
    }

    #[test]
    fn test_workflow_phase_snapshot_tracks_explicit_pre_bayes_soft_flag() {
        let snapshot = workflow_phase_snapshot_from_analyze_run(&AnalyzeRunRecord {
            run_id: "analyze:1".to_string(),
            source_command: "analyze".to_string(),
            multi_timeframe_summary: vec![
                "multi_timeframe_source=analyze_explicit_with_auto_fill covered_intervals=1m,5m,15m,1h,4h,1d"
                    .to_string(),
                "higher_timeframe_direction_bias=bullish".to_string(),
            ],
            pre_bayes_evidence_filter: PreBayesEvidenceFilter {
                gating_status: "pass_hard".to_string(),
                uses_soft_evidence: false,
                policy: ict_engine::state::PreBayesEvidencePolicy {
                    version: "policy-v1".to_string(),
                    ..ict_engine::state::PreBayesEvidencePolicy::default()
                },
                evidence_assignments: BTreeMap::from([(
                    "market_regime".to_string(),
                    "bull".to_string(),
                )]),
                soft_market_regime_distribution: BTreeMap::from([
                    ("bull".to_string(), 1.0),
                    ("bear".to_string(), 0.0),
                ]),
                ..PreBayesEvidenceFilter::default()
            },
            hybrid_duration_model: Some("negative_binomial".to_string()),
            hybrid_remaining_expected_bars: Some(2.5),
            ..AnalyzeRunRecord::default()
        });

        assert_eq!(snapshot.pre_bayes_policy_version, "policy-v1");
        assert!(!snapshot.pre_bayes_uses_soft_evidence);
        assert!(snapshot
            .pre_bayes_soft_evidence
            .contains_key("market_regime"));
        assert!(snapshot.phase_summary.contains("mtf_direction=bullish"));
        assert!(snapshot
            .phase_summary
            .contains("hybrid_remaining_expected_bars=2.500"));
        assert_eq!(snapshot.multi_timeframe_summary.len(), 2);
    }

    #[test]
    fn test_multi_timeframe_entry_quality_bias_respects_direction() {
        let supportive = multi_timeframe_entry_quality_bias(
            &ParsedMultiTimeframeEvidence {
                direction_bias: "bullish".to_string(),
                alignment_score: Some(0.80),
                entry_alignment_score: Some(0.75),
                covered_count: 6,
            },
            Direction::Bull,
        );
        let hostile = multi_timeframe_entry_quality_bias(
            &ParsedMultiTimeframeEvidence {
                direction_bias: "bullish".to_string(),
                alignment_score: Some(0.80),
                entry_alignment_score: Some(0.75),
                covered_count: 6,
            },
            Direction::Bear,
        );

        assert!(supportive[0] > hostile[0]);
        assert!(supportive[2] < hostile[2]);
    }

    #[test]
    fn test_pre_bayes_entry_quality_bridge_diff_exposes_multi_timeframe_fields() {
        let diff =
            pre_bayes_entry_quality_bridge_diff(&ict_engine::state::PreBayesEntryQualityBridge {
                long_signal_probability: 0.62,
                short_signal_probability: 0.38,
                multi_timeframe_direction_bias: "bullish".to_string(),
                multi_timeframe_alignment_score: Some(0.77),
                multi_timeframe_entry_alignment_score: Some(0.71),
                ..ict_engine::state::PreBayesEntryQualityBridge::default()
            });

        assert_eq!(diff.multi_timeframe_direction_bias, "bullish");
        assert_eq!(diff.multi_timeframe_alignment_score, Some(0.77));
        assert_eq!(diff.multi_timeframe_entry_alignment_score, Some(0.71));
    }

    #[test]
    fn test_build_agent_action_plan_prioritizes_rollback() {
        let plan = ict_engine::application::backtest::build_agent_action_plan(
            "hint",
            &PromotionDecision {
                approved: false,
                status: "hold".to_string(),
                reason: "insufficient_improvement".to_string(),
                target_factors: vec![],
                target_families: vec![],
            },
            &RollbackRecommendation {
                should_rollback: true,
                scope: "targeted".to_string(),
                reason: "factor_score_regression".to_string(),
                target_factors: vec!["trend_momentum".to_string()],
                target_families: vec![],
            },
            &[],
            &[],
        );

        assert!(!plan.items.is_empty());
        assert_eq!(plan.items[0].title, "Review Rollback");
        assert!(plan.items[0].blocking);
    }

    #[test]
    fn test_augment_action_plan_with_pre_bayes_filter_inserts_pda_review_item() {
        let mut plan = AgentActionPlan::default();
        augment_action_plan_with_pre_bayes_filter(
            &mut plan,
            &PreBayesEvidenceFilter {
                gating_status: "pass_neutralized".to_string(),
                rationale: vec!["pda weak".to_string()],
                conflict_flags: vec!["pda_sequence_cluster_weak".to_string()],
                ..PreBayesEvidenceFilter::default()
            },
        );

        assert!(!plan.items.is_empty());
        assert_eq!(plan.items[0].title, "Review PDA Sequence Cluster");
        assert_eq!(plan.items[0].stage, "pda_sequence_review");
        assert!(plan.items[0].blocking);
    }

    #[test]
    fn test_recommended_next_command_prefers_pda_sequence_review_stage() {
        let plan = AgentActionPlan {
            summary: "test".to_string(),
            items: vec![AgentActionItem {
                stage: "pda_sequence_review".to_string(),
                blocking: true,
                priority: "high".to_string(),
                title: "Review PDA Sequence Cluster".to_string(),
                rationale: "pda weak".to_string(),
                expected_output: "review".to_string(),
                expected_state_changes: vec![],
                suggested_files: vec![],
                suggested_commands: vec![
                    "cargo test pda_sequence::analysis -- --nocapture".to_string()
                ],
            }],
        };
        let commands = command_recommendations(&CommandContext {
            symbol: "NQ".to_string(),
            state_dir: "state".to_string(),
            analyze: Some(AnalyzeCommandSource::Files {
                data_htf: "htf.json".to_string(),
                data_mtf: "mtf.json".to_string(),
                data_ltf: "ltf.json".to_string(),
            }),
            research_data: Some("ltf.json".to_string()),
            paired_data: None,
            update_outcome: None,
            update_entry_signal: None,
            update_feedback_file: None,
            user_data_selection_required: false,
        });

        assert_eq!(
            recommended_next_command(&plan, &commands),
            "cargo test pda_sequence::analysis -- --nocapture"
        );
    }

    #[test]
    fn test_augment_action_plan_with_pre_bayes_filter_uses_specific_pda_review_title() {
        let mut plan = AgentActionPlan::default();
        augment_action_plan_with_pre_bayes_filter(
            &mut plan,
            &PreBayesEvidenceFilter {
                gating_status: "pass_neutralized".to_string(),
                rationale: vec!["consistency weak".to_string()],
                conflict_flags: vec![
                    "pda_sequence_cluster_weak".to_string(),
                    "pda_sequence_low_consistency".to_string(),
                ],
                ..PreBayesEvidenceFilter::default()
            },
        );

        assert_eq!(plan.items[0].title, "Review PDA Sequence Consistency");
        assert!(plan.items[0]
            .suggested_files
            .iter()
            .any(|file| file.ends_with("hmm_cluster.rs")));
        assert!(plan.items[0]
            .suggested_commands
            .iter()
            .any(|cmd| cmd == "cargo test pda_sequence::analysis -- --nocapture"));
    }

    #[test]
    fn test_build_stage_views_includes_specific_pda_review_action() {
        let views = ict_engine::application::backtest::build_stage_views(
            "NQ",
            "state",
            &CommandRecommendations {
                analyze: recommended_command(
                    "ict-engine analyze --symbol NQ --data-htf htf.json --data-mtf mtf.json --data-ltf ltf.json --state-dir state".to_string(),
                    true,
                    Vec::new(),
                    "",
                ),
                ..CommandRecommendations::default()
            },
            &[],
            &[],
            Some(&PreBayesEvidenceFilter {
                gating_status: "pass_neutralized".to_string(),
                rationale: vec!["coverage weak".to_string()],
                conflict_flags: vec![
                    "pda_sequence_cluster_weak".to_string(),
                    "pda_sequence_sparse_sessions".to_string(),
                ],
                ..PreBayesEvidenceFilter::default()
            }),
            None,
        );
        let pda_view = views
            .iter()
            .find(|view| view.stage == "pda_sequence_review")
            .expect("missing pda review stage");
        assert!(pda_view
            .actions
            .iter()
            .any(|item| item.contains("too few valid sessions")));
    }

    #[test]
    fn test_recommended_next_command_prefers_blocking_high_priority_items() {
        let plan = AgentActionPlan {
            summary: "test".to_string(),
            items: vec![
                AgentActionItem {
                    stage: "iteration".to_string(),
                    blocking: false,
                    priority: "medium".to_string(),
                    title: "Tune".to_string(),
                    rationale: "tune".to_string(),
                    expected_output: "tuned factor".to_string(),
                    expected_state_changes: vec![],
                    suggested_files: vec![],
                    suggested_commands: vec!["ict-engine factor-backtest --data a.json".to_string()],
                },
                AgentActionItem {
                    stage: "rollback".to_string(),
                    blocking: true,
                    priority: "high".to_string(),
                    title: "Rollback".to_string(),
                    rationale: "rollback".to_string(),
                    expected_output: "rollback decision".to_string(),
                    expected_state_changes: vec![],
                    suggested_files: vec![],
                    suggested_commands: vec!["ict-engine update --feedback-file f.json".to_string()],
                },
            ],
        };

        let commands = command_recommendations(&CommandContext {
            symbol: "NQ".to_string(),
            state_dir: "state".to_string(),
            analyze: None,
            research_data: Some("a.json".to_string()),
            paired_data: None,
            update_outcome: Some("loss".to_string()),
            update_entry_signal: None,
            update_feedback_file: Some("f.json".to_string()),
            user_data_selection_required: false,
        });

        let mut plan = plan;
        concretize_action_plan_commands(&mut plan, &commands);

        assert_eq!(
            recommended_next_command(&plan, &commands),
            "ict-engine update --symbol NQ --outcome loss --state-dir state"
        );
    }

    #[test]
    fn test_recommended_next_command_prefers_artifact_consumption_suggested_command() {
        let plan = AgentActionPlan {
            summary: "test".to_string(),
            items: vec![
                AgentActionItem {
                    stage: "artifact_consumption".to_string(),
                    blocking: true,
                    priority: "high".to_string(),
                    title: "Artifact Consumption".to_string(),
                    rationale: "artifact gate".to_string(),
                    expected_output: "expected_output_unavailable".to_string(),
                    expected_state_changes: vec![],
                    suggested_files: vec![],
                    suggested_commands: vec![
                        "ict-engine workflow-status --symbol NQ --state-dir state --phase artifact-consumed-gate".to_string()
                    ],
                },
                AgentActionItem {
                    stage: "rollback".to_string(),
                    blocking: true,
                    priority: "high".to_string(),
                    title: "Rollback".to_string(),
                    rationale: "rollback".to_string(),
                    expected_output: "expected_output_unavailable".to_string(),
                    expected_state_changes: vec![],
                    suggested_files: vec![],
                    suggested_commands: vec![
                        "ict-engine update --symbol NQ --outcome loss --state-dir state".to_string()
                    ],
                },
            ],
        };

        let commands = CommandRecommendations {
            update: recommended_command(
                "ict-engine update --symbol NQ --outcome loss --state-dir state".to_string(),
                true,
                Vec::new(),
                "",
            ),
            ..CommandRecommendations::default()
        };

        assert_eq!(
            recommended_next_command(&plan, &commands),
            "ict-engine workflow-status --symbol NQ --state-dir state --phase artifact-consumed-gate"
        );
    }

    #[test]
    fn test_humanize_workflow_command_for_user_data_gate() {
        let rendered = ict_engine::application::orchestration::humanize_workflow_command(
            "ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state"
        );
        assert_eq!(
            rendered,
            "Ask the user to choose the historical dataset. Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json Then run: ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state"
        );
    }

    #[test]
    fn test_workflow_status_agent_view_is_thinner_than_compact() {
        let snapshot = ict_engine::application::orchestration::sample_human_workflow_snapshot();
        let compact =
            ict_engine::application::orchestration::build_compact_workflow_status_view(&snapshot);
        let agent = ict_engine::application::orchestration::build_agent_workflow_status_view(
            &snapshot,
            &[],
        );
        assert_eq!(agent["symbol"], "NQ");
        assert_eq!(compact["symbol"], "NQ");
        assert!(agent.get("next_command").is_some());
        assert!(compact.get("next_command").is_some());
        assert!(agent.get("recommended_next_command").is_none());
        assert!(compact.get("recommended_next_command").is_none());
        assert!(compact.get("disagreements").is_none());
        assert!(compact.get("top_disagreement").is_some());
        assert!(agent.get("disagreements").is_none());
        assert!(agent.get("top_disagreement").is_some());
        assert!(agent.get("latest_phase_summary_full").is_none());
    }

    #[test]
    fn test_redact_local_paths_covers_local_path_prefixes_and_delimiters() {
        let input = concat!(
            "/Users/alice/file.json ",
            "/home/bob/file.json,",
            "/tmp/run.json;",
            "/var/log/app.json|",
            "/private/tmp/a.json)",
            "/Volumes/Data/demo.json]"
        );
        let redacted = redact_local_paths(input);
        assert_eq!(redacted.matches("<local-path>").count(), 6);
        assert!(redacted.contains("<local-path> <local-path>,<local-path>;"));
        assert!(redacted.contains("<local-path>|<local-path>)<local-path>]"));
    }

    #[test]
    fn test_redact_local_paths_in_value_walks_nested_strings() {
        let mut value = serde_json::json!({
            "top": "/Users/alice/top.json",
            "nested": [
                "/tmp/a.json",
                {"inner": "/Volumes/Data/demo.json"}
            ]
        });
        redact_local_paths_in_value(&mut value);
        assert_eq!(value["top"], "<local-path>");
        assert_eq!(value["nested"][0], "<local-path>");
        assert_eq!(value["nested"][1]["inner"], "<local-path>");
    }

    #[test]
    fn test_workflow_status_phase_human_view_redacts_local_paths() {
        let snapshot = ict_engine::application::orchestration::sample_human_workflow_snapshot();
        let mut value = ict_engine::application::orchestration::build_human_workflow_status_view(
            &snapshot,
            &[],
        );
        redact_local_paths_in_value(&mut value);

        let rendered = serde_json::to_string(&value).unwrap();
        assert!(rendered.contains("<local-path>"));
        assert!(!rendered.contains("/tmp/a.json"));
        assert!(!rendered.contains("/tmp/b.json"));
    }

    #[test]
    fn test_workflow_status_human_view_exposes_candidates() {
        let snapshot = ict_engine::application::orchestration::sample_human_workflow_snapshot();
        let value = ict_engine::application::orchestration::build_human_workflow_status_view(
            &snapshot,
            &[],
        );
        assert_eq!(value["symbol"], "NQ");
        assert_eq!(value["current_status"]["focus_phase"], "update");
        assert_eq!(value["hard_block"]["active"], true);
        assert_eq!(value["hard_block"]["status"], "action_blocked");
        assert_eq!(
            value["hard_block"]["reason"],
            "user_selected_historical_data_missing"
        );
        assert!(value["hard_block"]["human_action"]
            .as_str()
            .unwrap()
            .contains("Ask the user to choose the historical dataset"));
        assert!(value["what_you_should_do_now"]
            .as_str()
            .unwrap()
            .contains("Ask the user to choose the historical dataset"));
        assert!(!value["what_you_should_do_now"]
            .as_str()
            .unwrap()
            .contains("Next step: ict-engine factor-research"));
        assert_eq!(value["current_status"]["blocking_status"], "action_blocked");
        assert_eq!(
            value["current_status"]["blocking_reason"],
            "user_selected_historical_data_missing"
        );
        assert_eq!(
            value["current_status"]["top_level_command_source"],
            "historical_data_selection_gate"
        );
        assert_eq!(
            value["what_you_should_do_now_source"],
            "historical_data_selection_gate"
        );
        assert_eq!(value["historical_data_candidates"][0], "/tmp/a.json");
        assert!(value["historical_data_request_template"]
            .as_str()
            .unwrap()
            .contains("Please choose one historical data path"));
        assert!(value["historical_data_request_template"]
            .as_str()
            .unwrap()
            .contains("/tmp/a.json"));
        assert!(value["agent_fill_path_instructions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap().contains("--data /tmp/a.json")));
        assert!(value["user_path_input_prompt"]
            .as_str()
            .unwrap()
            .contains("Reply with one path"));
        assert_eq!(value["ensemble_consensus"]["final_action"], "observe");
        assert_eq!(value["ensemble_consensus"]["hard_block"]["active"], true);
        assert_eq!(
            value["ensemble_consensus"]["hard_block"]["reason"],
            "user_selected_historical_data_missing"
        );
        assert!(value["ensemble_consensus"]["human_next_triage"]
            .as_str()
            .unwrap()
            .contains("ensemble_action=observe"));
        assert_eq!(
            value["ensemble_consensus"]["executor_scorecards"][0]["executor"],
            "catboost_stub"
        );
        assert_eq!(
            value["ensemble_consensus"]["executor_scorecards"][0]["latest_weight_hint"],
            0.55
        );
        assert_eq!(
            value["jump_model"],
            serde_json::json!(
                "jump_model active_state=jump_transition confidence=0.500 transition_risk=0.500; jump_calibration_gate outcome=accepted sample_count=4 cooldown_status=ready"
            )
        );
        assert_eq!(
            value["jump_calibration_gate"],
            serde_json::json!(
                "jump_calibration_gate outcome=accepted sample_count=4 cooldown_status=ready"
            )
        );
    }

    #[test]
    fn test_jump_workflow_summaries_surface_calibration_gate() {
        let snapshot = ict_engine::application::orchestration::sample_human_workflow_snapshot();
        assert_eq!(
            jump_model_workflow_summary(&snapshot).as_deref(),
            Some(
                "jump_model active_state=jump_transition confidence=0.500 transition_risk=0.500; jump_calibration_gate outcome=accepted sample_count=4 cooldown_status=ready"
            )
        );
        assert_eq!(
            jump_calibration_gate_workflow_summary(&snapshot).as_deref(),
            Some("jump_calibration_gate outcome=accepted sample_count=4 cooldown_status=ready")
        );
    }

    #[test]
    fn test_workflow_status_human_view_prefers_persisted_scorecards() {
        let snapshot = ict_engine::application::orchestration::sample_human_workflow_snapshot();
        let persisted = vec![EnsembleExecutorScorecard {
            executor: "xgboost_file".to_string(),
            latest_weight_hint: Some(0.72),
            wins: 3,
            ..EnsembleExecutorScorecard::default()
        }];
        let value = ict_engine::application::orchestration::build_human_workflow_status_view(
            &snapshot, &persisted,
        );
        assert_eq!(
            value["ensemble_consensus"]["executor_scorecards"][0]["executor"],
            "xgboost_file"
        );
        assert_eq!(
            value["ensemble_consensus"]["executor_scorecard_source"],
            "persisted"
        );
        assert_eq!(
            value["ensemble_consensus"]["executor_scorecards"][0]["latest_weight_hint"],
            0.72
        );
    }

    #[test]
    fn test_executor_scorecard_surface_marks_fallback_and_persisted() {
        let fallback = vec![EnsembleExecutorScorecard {
            executor: "catboost_stub".to_string(),
            ..EnsembleExecutorScorecard::default()
        }];
        let persisted = vec![EnsembleExecutorScorecard {
            executor: "xgboost_file".to_string(),
            ..EnsembleExecutorScorecard::default()
        }];

        let (fallback_surface, fallback_source) =
            ict_engine::application::orchestration::executor_scorecard_surface(&[], &fallback);
        assert_eq!(fallback_source, "fallback");
        assert_eq!(fallback_surface[0].executor, "catboost_stub");

        let (persisted_surface, persisted_source) =
            ict_engine::application::orchestration::executor_scorecard_surface(
                &persisted, &fallback,
            );
        assert_eq!(persisted_source, "persisted");
        assert_eq!(persisted_surface[0].executor, "xgboost_file");
    }

    #[test]
    fn test_ensemble_vote_history_view_uses_resolved_scorecard_source() {
        let vote = ict_engine::application::orchestration::sample_human_workflow_snapshot()
            .latest_ensemble_vote
            .expect("sample ensemble vote");
        let persisted = vec![EnsembleExecutorScorecard {
            executor: "xgboost_file".to_string(),
            latest_weight_hint: Some(0.80),
            ..EnsembleExecutorScorecard::default()
        }];
        let (scorecards, scorecard_source) = resolved_vote_scorecards(&persisted, &vote);
        let history = vec![serde_json::json!({
            "artifact_id": vote.artifact_id,
            "hard_block": vote.hard_block,
            "executor_scorecards": scorecards,
            "executor_scorecard_source": scorecard_source,
        })];
        let hard_block_only = vec![serde_json::json!({
            "artifact_id": vote.artifact_id,
            "hard_block": vote.hard_block,
        })];
        let value = serde_json::json!({
            "history": history,
            "hard_block_only": hard_block_only,
            "hard_block_summary": {
                "count": 1,
                "reason_leaderboard": [serde_json::json!({
                    "reason": vote.hard_block.reason,
                    "count": 1,
                })],
            }
        });
        assert_eq!(
            value["history"][0]["executor_scorecard_source"],
            "persisted"
        );
        assert_eq!(
            value["history"][0]["executor_scorecards"][0]["executor"],
            "xgboost_file"
        );
        assert_eq!(value["hard_block_only"][0]["artifact_id"], vote.artifact_id);
        assert_eq!(value["hard_block_summary"]["count"], 1);
    }

    #[test]
    fn test_load_canonical_executor_scorecards_falls_back_to_vote_record() {
        let temp = tempfile::tempdir().unwrap();
        let record = EnsembleVoteRecord {
            artifact_id: "ensemble-vote:test".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-1".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2-weighted".to_string(),
            final_action: "observe".to_string(),
            recommended_command: "ict-engine workflow-status --symbol NQ --phase human-next"
                .to_string(),
            human_next_triage: "hard_blocked=false ensemble_action=observe".to_string(),
            hard_block: ict_engine::application::orchestration::EnsembleHardBlockArtifact::default(
            ),
            confidence: 0.5,
            consensus_strength: 0.5,
            disagreement_flags: Vec::new(),
            executor_summaries: vec![
                "executor=catboost_stub action=observe confidence=0.500".to_string()
            ],
            split_explanations: vec!["active_regime=research".to_string()],
            executor_scorecards: vec![EnsembleExecutorScorecard {
                executor: "catboost_stub".to_string(),
                latest_weight_hint: Some(0.55),
                ..EnsembleExecutorScorecard::default()
            }],
            executor_scorecards_source: Some("fallback".to_string()),
            posterior_fingerprint: "fp-test".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "research".to_string(),
            posterior_confidence: Some(0.5),
            posterior_probabilities: BTreeMap::new(),
            posterior_evidence: vec!["mtf=test".to_string()],
        };
        save_ensemble_vote_artifact(temp.path(), "NQ", &record).unwrap();
        append_ensemble_vote_history(temp.path(), "NQ", record).unwrap();

        let scorecards =
            load_canonical_executor_scorecards(temp.path().to_str().unwrap(), "NQ", Some("run-1"))
                .unwrap();
        assert_eq!(scorecards[0].executor, "catboost_stub");
        assert_eq!(scorecards[0].latest_weight_hint, Some(0.55));
    }

    fn save_then_load_vote_record_for_test(
        dir: &std::path::Path,
        record: &EnsembleVoteRecord,
    ) -> EnsembleVoteRecord {
        save_ensemble_vote_artifact(dir, "NQ", record).unwrap();
        load_state(dir, "NQ", ict_engine::state::ENSEMBLE_VOTE_FILE).unwrap()
    }

    #[test]
    fn test_persist_ensemble_vote_record_writes_canonical_scorecards_not_mirror() {
        let temp = tempfile::tempdir().unwrap();
        let record = EnsembleVoteRecord {
            artifact_id: "ensemble-vote:test".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-1".to_string()),
            provenance: RunProvenance::default(),
            dataset_comparability: DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2-weighted".to_string(),
            final_action: "observe".to_string(),
            recommended_command: "ict-engine workflow-status --symbol NQ --phase human-next"
                .to_string(),
            human_next_triage: "hard_blocked=false ensemble_action=observe".to_string(),
            hard_block: ict_engine::application::orchestration::EnsembleHardBlockArtifact::default(
            ),
            confidence: 0.5,
            consensus_strength: 0.5,
            disagreement_flags: Vec::new(),
            executor_summaries: vec![
                "executor=catboost_stub action=observe confidence=0.500".to_string()
            ],
            split_explanations: vec!["active_regime=research".to_string()],
            executor_scorecards: vec![EnsembleExecutorScorecard {
                executor: "mirror_only".to_string(),
                ..EnsembleExecutorScorecard::default()
            }],
            executor_scorecards_source: Some("fallback".to_string()),
            posterior_fingerprint: "fp-test".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "research".to_string(),
            posterior_confidence: Some(0.5),
            posterior_probabilities: BTreeMap::new(),
            posterior_evidence: vec!["mtf=test".to_string()],
        };
        let canonical = vec![EnsembleExecutorScorecard {
            executor: "canonical_only".to_string(),
            latest_weight_hint: Some(0.77),
            ..EnsembleExecutorScorecard::default()
        }];
        persist_ensemble_vote_record(temp.path().to_str().unwrap(), &record, &canonical).unwrap();

        let saved = load_ensemble_executor_scorecards(temp.path(), "NQ").unwrap();
        let saved_vote = save_then_load_vote_record_for_test(temp.path(), &record);
        assert_eq!(saved[0].executor, "canonical_only");
        assert_eq!(saved_vote.executor_scorecards[0].executor, "mirror_only");
    }

    #[test]
    fn test_command_recommendations_map_stages_to_commands() {
        let commands = command_recommendations(&CommandContext {
            symbol: "NQ".to_string(),
            state_dir: "state".to_string(),
            analyze: Some(AnalyzeCommandSource::Files {
                data_htf: "htf.json".to_string(),
                data_mtf: "mtf.json".to_string(),
                data_ltf: "ltf.json".to_string(),
            }),
            research_data: Some("a.json".to_string()),
            paired_data: None,
            update_outcome: Some("win".to_string()),
            update_entry_signal: None,
            update_feedback_file: Some("f.json".to_string()),
            user_data_selection_required: true,
        });
        assert_eq!(
            commands.research.command,
            "ict-engine factor-research --symbol NQ --data a.json --state-dir state"
        );
        assert_eq!(
            commands.update.command,
            "ict-engine update --symbol NQ --outcome win --state-dir state"
        );
        assert!(commands.research.user_data_selection_required);
        assert!(commands.backtest.user_data_selection_required);
        assert!(!commands.research.ready);
        assert!(commands
            .research
            .missing_inputs
            .contains(&"user_selected_historical_data".to_string()));
        assert_eq!(
            render_recommended_command(&commands.research),
            "ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=htf.json, mtf.json, ltf.json, a.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data a.json --state-dir state"
        );
    }

    #[test]
    fn test_command_recommendations_expose_update_template_without_outcome() {
        let commands = command_recommendations(&CommandContext {
            symbol: "NQ".to_string(),
            state_dir: "state".to_string(),
            analyze: None,
            research_data: Some("a.json".to_string()),
            paired_data: None,
            update_outcome: None,
            update_entry_signal: None,
            update_feedback_file: None,
            user_data_selection_required: false,
        });

        assert_eq!(
            commands.update.command,
            "ict-engine update --symbol NQ --outcome <win|loss|breakeven> --state-dir state"
        );
        assert!(!commands.update.ready);
        assert!(commands
            .update
            .missing_inputs
            .contains(&"realized_outcome".to_string()));
    }

    #[test]
    fn test_build_agent_context_bundle_contains_stage_views_and_window() {
        let bundle = ict_engine::application::backtest::build_agent_context_bundle(
            ict_engine::application::backtest::BuildAgentContextBundleInput {
                symbol: "NQ",
                state_dir: "state",
                workflow_state: &WorkflowState {
                    phase: "research_iteration".to_string(),
                    reason: "need_tuning".to_string(),
                },
                decision_hint: "hint",
                recommended_next_command: "ict-engine factor-research --data a.json",
                recommended_commands: &CommandRecommendations {
                    analyze: recommended_command("a".to_string(), true, Vec::new(), ""),
                    research: recommended_command("r".to_string(), true, Vec::new(), ""),
                    backtest: recommended_command("b".to_string(), true, Vec::new(), ""),
                    update: recommended_command("u".to_string(), true, Vec::new(), ""),
                },
                dataset_comparability: &DatasetComparability {
                    comparable: true,
                    previous_run_id: Some("run-1".to_string()),
                    reason: "same_data_same_config".to_string(),
                    comparison_class: "same_data_same_config".to_string(),
                    same_data: true,
                    same_config: true,
                    same_prompt_version: true,
                    same_factor_version: true,
                },
                factor_iteration_queue: &[FactorIterationPrompt {
                    factor_name: "trend_momentum".to_string(),
                    composite_score: 0.4,
                    grade: "D".to_string(),
                    iteration_action: "replace".to_string(),
                    replacement_candidate: true,
                    prompt: "replace".to_string(),
                }],
                family_outcomes: &[FactorFamilyOutcome {
                    family: "trend_momentum".to_string(),
                    promotion_decision: PromotionDecision {
                        approved: false,
                        status: "hold".to_string(),
                        reason: "weak".to_string(),
                        target_factors: vec![],
                        target_families: vec![],
                    },
                    rollback_recommendation: RollbackRecommendation {
                        should_rollback: true,
                        scope: "family".to_string(),
                        reason: "weak".to_string(),
                        target_factors: vec![],
                        target_families: vec![],
                    },
                }],
                pre_bayes_evidence_filter: Some(&PreBayesEvidenceFilter {
                    gating_status: "pass_neutralized".to_string(),
                    evidence_quality_score: 0.42,
                    rationale: vec!["neutralized".to_string()],
                    conflict_flags: vec!["pda_sequence_cluster_weak".to_string()],
                    evidence_assignments: BTreeMap::from([
                        ("market_regime".to_string(), "range".to_string()),
                        ("liquidity_context".to_string(), "neutral".to_string()),
                    ]),
                    ..PreBayesEvidenceFilter::default()
                }),
                pre_bayes_entry_quality_bridge: Some(
                    &ict_engine::state::PreBayesEntryQualityBridge {
                        rationale: vec!["bridge".to_string()],
                        ..ict_engine::state::PreBayesEntryQualityBridge::default()
                    },
                ),
                pda_sequence_summary: Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                    method: "pda_sequence_analysis_v2".to_string(),
                    primary_cluster: Some(1),
                    primary_cluster_label: Some("cluster_1".to_string()),
                    primary_cluster_family: Some("trend".to_string()),
                    primary_cluster_confidence: Some(0.88),
                    consistency_ratio: 0.75,
                    ensemble_mean_confidence: 0.83,
                    valid_sessions: 8,
                    kmer_k: 2,
                }),
                factor_mutation_evaluation: None,
                artifact_decision_summary: Some(&ict_engine::state::ArtifactDecisionSummary {
                    consumed_trend_status: "validated_regressing".to_string(),
                    consumed_trend_reason: "regression".to_string(),
                    consumed_target_kinds: vec!["pending_update".to_string()],
                    ..ict_engine::state::ArtifactDecisionSummary::default()
                }),
            },
        );

        assert_eq!(bundle.family_history_window, 5);
        assert_eq!(bundle.stage_views.len(), 6);
        assert_eq!(bundle.stage_views[1].stage, "research");
        assert_eq!(bundle.artifact_consumed_gate_status, "validated_regressing");
        assert_eq!(bundle.pda_cluster_label.as_deref(), Some("cluster_1"));
        assert!(bundle
            .pda_sequence_summary
            .as_deref()
            .unwrap_or_default()
            .contains("consistency=0.750"));
        assert!(bundle
            .stage_views
            .iter()
            .any(|view| view.stage == "pda_sequence_review"));
        assert!(bundle
            .stage_views
            .iter()
            .any(|view| view.stage == "artifact_consumption"));
    }

    #[test]
    fn test_agent_context_bundle_minimal_uses_explicit_pre_bayes_soft_flag() {
        let bundle = ict_engine::application::backtest::build_agent_context_bundle(
            ict_engine::application::backtest::BuildAgentContextBundleInput {
                symbol: "NQ",
                state_dir: "state",
                workflow_state: &WorkflowState {
                    phase: "observe_or_deploy".to_string(),
                    reason: "stable".to_string(),
                },
                decision_hint: "hint",
                recommended_next_command: "ict-engine analyze --symbol NQ",
                recommended_commands: &CommandRecommendations::default(),
                dataset_comparability: &DatasetComparability {
                    comparable: true,
                    ..DatasetComparability::default()
                },
                factor_iteration_queue: &[],
                family_outcomes: &[],
                pre_bayes_evidence_filter: Some(&PreBayesEvidenceFilter {
                    gating_status: "pass_hard".to_string(),
                    uses_soft_evidence: false,
                    evidence_assignments: BTreeMap::from([(
                        "market_regime".to_string(),
                        "bull".to_string(),
                    )]),
                    soft_market_regime_distribution: BTreeMap::from([
                        ("bull".to_string(), 1.0),
                        ("bear".to_string(), 0.0),
                    ]),
                    soft_liquidity_context_distribution: BTreeMap::from([
                        ("favorable".to_string(), 1.0),
                        ("neutral".to_string(), 0.0),
                    ]),
                    ..PreBayesEvidenceFilter::default()
                }),
                pre_bayes_entry_quality_bridge: Some(
                    &ict_engine::state::PreBayesEntryQualityBridge::default(),
                ),
                pda_sequence_summary: Some(&ict_engine::pda_sequence::PdaSequenceArtifactSummary {
                    method: "pda_sequence_analysis_v2".to_string(),
                    primary_cluster: Some(0),
                    primary_cluster_label: Some("cluster_0".to_string()),
                    primary_cluster_family: Some("trend".to_string()),
                    primary_cluster_confidence: Some(0.67),
                    consistency_ratio: 0.70,
                    ensemble_mean_confidence: 0.72,
                    valid_sessions: 6,
                    kmer_k: 2,
                }),
                factor_mutation_evaluation: None,
                artifact_decision_summary: None,
            },
        );

        let minimal =
            ict_engine::application::backtest::build_agent_context_bundle_minimal(&bundle);
        assert!(!minimal.pre_bayes_uses_soft_evidence);
        assert_eq!(minimal.pda_cluster_label.as_deref(), Some("cluster_0"));
    }

    #[test]
    fn test_family_diffs_reports_family_level_score_changes() {
        let previous = vec![FactorFamilyDecision {
            family: "trend_momentum".to_string(),
            factor_count: 1,
            avg_score: 0.40,
            actions: vec!["trend_momentum:tune".to_string()],
            replacement_candidates: vec![],
            ..FactorFamilyDecision::default()
        }];
        let current = vec![FactorFamilyDecision {
            family: "trend_momentum".to_string(),
            factor_count: 1,
            avg_score: 0.62,
            actions: vec!["trend_momentum:keep".to_string()],
            replacement_candidates: vec![],
            ..FactorFamilyDecision::default()
        }];

        let diffs = ict_engine::application::backtest::family_diffs(&previous, &current);
        assert_eq!(diffs.len(), 1);
        assert!(diffs[0].avg_score_delta > 0.0);
    }

    #[test]
    fn test_family_history_from_runs_tracks_trend() {
        let history = ict_engine::application::backtest::family_history_from_runs(vec![
            (
                "run-1".to_string(),
                Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                vec![FactorFamilyDecision {
                    family: "trend_momentum".to_string(),
                    factor_count: 1,
                    avg_score: 0.40,
                    actions: vec![],
                    replacement_candidates: vec![],
                    ..FactorFamilyDecision::default()
                }],
            ),
            (
                "run-2".to_string(),
                Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
                vec![FactorFamilyDecision {
                    family: "trend_momentum".to_string(),
                    factor_count: 1,
                    avg_score: 0.58,
                    actions: vec![],
                    replacement_candidates: vec!["trend_momentum".to_string()],
                    ..FactorFamilyDecision::default()
                }],
            ),
        ]);

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].window_size, 5);
        assert_eq!(history[0].score_trend, "improving");
        assert_eq!(history[0].replacement_trend, "worsening");
        assert_eq!(history[0].recent_run_ids.len(), 2);
    }

    #[test]
    fn test_decision_history_summary_counts_runs() {
        let summary = ict_engine::application::backtest::decision_history_summary(vec![
            (
                PromotionDecision {
                    approved: true,
                    status: "promote".to_string(),
                    reason: "ok".to_string(),
                    target_factors: vec![],
                    target_families: vec![],
                },
                RollbackRecommendation {
                    should_rollback: false,
                    scope: "none".to_string(),
                    reason: "ok".to_string(),
                    target_factors: vec![],
                    target_families: vec![],
                },
            ),
            (
                PromotionDecision {
                    approved: false,
                    status: "hold".to_string(),
                    reason: "weak".to_string(),
                    target_factors: vec![],
                    target_families: vec![],
                },
                RollbackRecommendation {
                    should_rollback: true,
                    scope: "targeted".to_string(),
                    reason: "regression".to_string(),
                    target_factors: vec!["trend_momentum".to_string()],
                    target_families: vec![],
                },
            ),
        ]);

        assert_eq!(summary.total_runs, 2);
        assert_eq!(summary.promotion_approved_runs, 1);
        assert_eq!(summary.rollback_recommended_runs, 1);
        assert_eq!(summary.latest_rollback_scope.as_deref(), Some("targeted"));
    }

    #[test]
    fn test_resolve_output_format_rejects_alias_and_explicit_mix() {
        let err = resolve_output_format("agent", false, false, true).unwrap_err();
        assert!(err
            .to_string()
            .contains("do not combine --output-format with --compact/--agent/--human"));
    }

    #[test]
    fn test_build_env_report_lists_state_dir_env_var() {
        let report = build_env_report();
        assert_eq!(report["state_dir_env_var"], STATE_DIR_ENV_VAR);
        assert_eq!(report["default_state_dir"], DEFAULT_STATE_DIR);
        assert!(report["variables"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["name"] == STATE_DIR_ENV_VAR));
    }

    #[test]
    fn test_cli_backtest_accepts_human_output_alias() {
        let cli = Cli::try_parse_from([
            "ict-engine",
            "backtest",
            "--symbol",
            "NQ",
            "--data",
            "candles.json",
            "--human",
        ])
        .unwrap();
        match cli.command {
            Commands::Backtest { human, .. } => assert!(human),
            other => panic!("unexpected command: {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cli_factor_research_accepts_output_format() {
        let cli = Cli::try_parse_from([
            "ict-engine",
            "factor-research",
            "--symbol",
            "NQ",
            "--data",
            "candles.json",
            "--output-format",
            "compact",
        ])
        .unwrap();
        match cli.command {
            Commands::FactorResearch { output_format, .. } => {
                assert_eq!(output_format, "compact");
            }
            other => panic!("unexpected command: {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cli_env_command_parses() {
        let cli = Cli::try_parse_from(["ict-engine", "env"]).unwrap();
        match cli.command {
            Commands::Env => {}
            other => panic!("unexpected command: {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_recommended_next_command_meta_classifies_ask_user_gate() {
        let meta = recommended_next_command_meta(
            "ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state"
        );
        assert!(meta.requires_user_input);
        assert!(meta.blocked);
        assert_eq!(
            meta.prompt.as_deref(),
            Some(
                "Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json"
            )
        );
        assert_eq!(
            meta.executable_command.as_deref(),
            Some("ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state")
        );
        assert_eq!(meta.recorded_data_paths.len(), 2);
    }

    #[test]
    fn test_recommended_next_command_meta_classifies_ict_engine_command() {
        let meta = recommended_next_command_meta(
            "ict-engine workflow-status --symbol NQ --state-dir state --phase artifact-consumed-gate",
        );
        assert!(!meta.requires_user_input);
        assert!(!meta.blocked);
        assert_eq!(
            meta.executable_command.as_deref(),
            Some(
                "ict-engine workflow-status --symbol NQ --state-dir state --phase artifact-consumed-gate"
            )
        );
    }

    #[test]
    fn test_output_format_resolve_rejects_human_and_explicit_json_mix() {
        let error = resolve_output_format("json", false, false, true).unwrap_err();
        assert!(error
            .to_string()
            .contains("do not combine --output-format with --compact/--agent/--human"));
    }

    #[test]
    fn test_output_format_resolve_rejects_compact_and_explicit_json_mix() {
        let error = resolve_output_format("json", true, false, false).unwrap_err();
        assert!(error
            .to_string()
            .contains("do not combine --output-format with --compact/--agent/--human"));
    }

    #[test]
    fn test_output_format_resolve_allows_alias_with_default_empty_value() {
        let resolved = resolve_output_format("", false, false, true).unwrap();
        assert_eq!(resolved, OutputFormat::Human);
    }

    #[test]
    fn test_output_format_resolve_empty_defaults_to_json() {
        let resolved = resolve_output_format("", false, false, false).unwrap();
        assert_eq!(resolved, OutputFormat::Json);
    }

    #[test]
    fn test_cli_analyze_accepts_json_alias_mix_at_parse_level() {
        let cli = Cli::try_parse_from([
            "ict-engine",
            "analyze",
            "--symbol",
            "DEMO",
            "--demo",
            "--human",
            "--output-format",
            "json",
        ]);
        assert!(
            cli.is_ok(),
            "cli parse should succeed; runtime guard handles conflict"
        );
    }

    #[test]
    fn test_cli_analyze_default_output_format_is_empty_sentinel() {
        let cli =
            Cli::try_parse_from(["ict-engine", "analyze", "--symbol", "DEMO", "--demo"]).unwrap();
        match cli.command {
            Commands::Analyze { output_format, .. } => {
                assert_eq!(output_format, "");
            }
            other => panic!("unexpected command: {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cli_workflow_status_accepts_stable_flag() {
        let cli = Cli::try_parse_from([
            "ict-engine",
            "workflow-status",
            "--symbol",
            "NQ",
            "--stable",
        ])
        .unwrap();

        match cli.command {
            Commands::WorkflowStatus { stable, .. } => {
                assert!(stable);
            }
            other => panic!("unexpected command: {:?}", std::mem::discriminant(&other)),
        }
    }
}
