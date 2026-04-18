pub mod agent_report;
pub mod compact_report;
pub mod glossary_map;
pub mod human_report;

pub use agent_report::{build_agent_guidance_report, AgentGuidanceReport};
pub use compact_report::{
    build_compact_analyze_report, build_compact_backtest_report, build_compact_reflection_report,
    humanize_decision_hint, CompactAnalyzeReport, CompactBacktestReport, CompactReflectionReport,
};
pub use human_report::{build_human_analyze_report, HumanAnalyzeReport};
