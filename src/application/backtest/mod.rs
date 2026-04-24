pub mod backtest_compare;
pub mod backtest_request;
pub mod backtest_result;
pub mod command_entry;
pub mod feedback;
pub mod finalize_actions;
pub mod finalize_artifacts;
pub mod finalize_commands;
pub mod finalize_context;
pub mod finalize_decisions;
pub mod finalize_enrichment;
pub mod finalize_recommendations;
pub mod finalize_stats;
pub mod finalize_surfaces;
pub mod finalized_run;
pub mod pre_bayes_actions;
pub mod provenance;
pub mod runtime_report;

pub use backtest_compare::{
    build_backtest_compare_report, build_duration_sizing_delta_surface,
    build_oos_quality_delta_surface, build_research_compare_report,
    build_shrink_on_off_comparison_summary, compare_backtest_results, BacktestCompareReport,
};
pub use backtest_request::{build_backtest_request, BacktestRequest, BacktestRequestInput};
pub use backtest_result::{
    build_backtest_result_artifact, BacktestResultArtifact, BacktestResultArtifactInput,
};
pub use command_entry::{
    backtest_command, factor_backtest_command, factor_research_command, BacktestCommandInput,
    FactorResearchCommandInput,
};
pub use feedback::{
    apply_feedback_to_trade_outcome_network, build_feedback_record, enrich_feedback_record,
    factor_alignment_label_from_feedback, factor_uncertainty_label_from_feedback,
    trade_outcome_label_from_pnl, BuildFeedbackRecordInput,
};
pub use finalize_actions::{
    augment_action_plan_with_artifact_trends, build_agent_action_plan, workflow_state_from_context,
};
pub use finalize_artifacts::{
    load_finalize_backtest_artifact_surfaces, FinalizeBacktestArtifactSurfaces,
};
pub use finalize_commands::{
    concretize_action_plan_commands, recommended_next_command, render_recommended_command,
};
pub use finalize_context::{
    build_agent_context_bundle, build_agent_context_bundle_minimal, build_stage_views,
    pre_bayes_entry_quality_bridge_diff, pre_bayes_soft_evidence_diff,
    BuildAgentContextBundleInput,
};
pub use finalize_decisions::{
    artifact_consumed_decision_gate, link_artifact_decision_summary_to_decisions,
};
pub use finalize_enrichment::{
    apply_finalize_backtest_enrichment, FinalizeBacktestEnrichmentInput,
};
pub use finalize_recommendations::{
    command_recommendations, recommended_command, user_data_selection_prompt, AnalyzeCommandSource,
    CommandContext,
};
pub use finalize_stats::{
    cpt_probability_diffs, decision_history_summary, family_diffs, family_history_from_runs,
    probability_diffs, ranking_diffs,
};
pub use finalize_surfaces::{
    derive_finalize_backtest_decision_surfaces, FinalizeBacktestDecisionSurfaces,
    FinalizeBacktestDecisionSurfacesInput,
};
pub use finalized_run::{persist_finalized_backtest_run, PersistFinalizedBacktestRunInput};
pub use pre_bayes_actions::{
    augment_action_plan_with_consumed_pre_bayes_context, augment_action_plan_with_pre_bayes_filter,
    pda_sequence_review_commands, pda_sequence_review_files, pda_sequence_review_rationale,
    pda_sequence_review_title, workflow_state_from_pre_bayes_filter,
};
pub use provenance::{
    data_fingerprint, dataset_comparability, decision_thresholds, factor_version, run_provenance,
};
pub use runtime_report::{
    build_backtest_agent_prompts, build_runtime_backtest_report, trade_outcome_cpt_snapshot,
    BuildRuntimeBacktestReportInput,
};
