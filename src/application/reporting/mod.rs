pub mod agent_report;
pub mod analyze_output;
pub mod backtest_output;
pub mod compact_report;
pub mod execution_focus;
pub mod glossary_map;
pub mod human_report;

pub use agent_report::{build_agent_guidance_report, AgentGuidanceReport};
pub use analyze_output::{
    build_analyze_compact_evidence, build_analyze_live_output_envelope,
    build_analyze_live_output_value, build_analyze_live_reporting_bundle,
    build_analyze_output_envelope, build_analyze_policy_outputs, build_analyze_reporting_bundle,
    build_human_analyze_surface, dispatch_analyze_live_output, dispatch_analyze_output,
    emit_analyze_live_output, emit_analyze_live_output_envelope,
    emit_analyze_live_output_with_input, emit_analyze_output, emit_analyze_output_envelope,
    AnalyzeHumanInput, AnalyzeLiveOutputDispatchInput, AnalyzeLiveOutputEmitInput,
    AnalyzeLiveOutputEnvelope, AnalyzeLiveReportingBundle, AnalyzeLiveReportingBundleInput,
    AnalyzeMarketFamilySummary, AnalyzeOutputDispatchInput, AnalyzeOutputEnvelope,
};
pub use backtest_output::{
    build_backtest_output_payload, build_compact_compare_report,
    build_factor_backtest_output_payload, build_factor_research_output_payload,
    emit_structured_output_payload, human_backtest_compare_summary, human_compare_summary,
    human_research_compare_summary, render_backtest_human_output,
    render_factor_backtest_human_output, render_factor_research_human_output,
};
pub use compact_report::{
    build_compact_analyze_report, build_compact_backtest_report, build_compact_reflection_report,
    humanize_decision_hint, CompactAnalyzeReport, CompactBacktestReport, CompactReflectionReport,
};
pub use execution_focus::{
    build_execution_focus_surface, execution_focus_enabled, ExecutionFocusSurface,
    EXECUTION_FOCUS_ENV_VAR,
};
pub use human_report::{build_human_analyze_report, humanize_next_step_line, HumanAnalyzeReport};
