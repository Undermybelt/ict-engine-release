pub mod clean_futures;
pub mod command_entry;
pub mod control_matrix_providers;
pub mod control_matrix_runtime;
pub mod harness;
pub mod live_defaults;
pub mod options_summary;
pub mod provider_fetch;
pub mod sop_reports;
pub mod source_freshness;
pub mod source_health;
pub mod source_snapshot;
pub(crate) mod tradingview_mcp;

pub use clean_futures::{
    discover_tomac_futures_datasets, infer_market_code_from_path, run_clean_futures,
    run_clean_futures_multi_timeframe, CleanFuturesDatasetReport, CleanFuturesReport,
    CleanedCandleOutput, MultiTimeframeCleanFuturesReport,
};
pub use command_entry::{
    clean_futures_command, expansion_sop_command, futures_sop_command,
    market_data_harness_fetch_command, market_data_harness_plan_command, ExpansionSopCommandInput,
    MarketDataHarnessCommandInput,
};
pub use control_matrix_providers::{
    build_control_matrix_provider_summary, build_provider_summary_for_requirements,
    ControlMatrixDataRequirement, ControlMatrixProviderKind, ControlMatrixProviderStatus,
    ControlMatrixProviderSummary, IBKR_CAPABILITIES_RELATIVE_PATH, IBKR_CONSENT_RELATIVE_PATH,
    TRADINGVIEW_MCP_ARGS_ENV, TRADINGVIEW_MCP_CMD_ENV, TVREMIX_MCP_API_KEY_ENV,
    TVREMIX_MCP_DEFAULT_URL, TVREMIX_MCP_LOCAL_CONFIG_RELATIVE_PATH, TVREMIX_MCP_URL_ENV,
};
pub use control_matrix_runtime::{
    build_control_matrix_runtime_overrides, ControlMatrixRuntimeOverrides,
};
pub use harness::{
    build_market_data_harness_plan, execute_market_data_harness_plan,
    load_market_data_harness_preset_config, repo_root_from_harness, MarketDataHarnessBundle,
    MarketDataHarnessEnvelope, MarketDataHarnessIbkrSpec, MarketDataHarnessOperation,
    MarketDataHarnessPlan, MarketDataHarnessPreset, MarketDataHarnessPresetConfig,
    MarketDataHarnessRequest, MarketDataHarnessSymbolSpec, MarketDataHarnessTask,
    MarketLiveDefaultsSpec, ProviderExecutionRequest, MARKET_DATA_HARNESS_PRESETS_FILE,
};
pub use live_defaults::{
    analyze_live_inferred_symbols, build_inferable_live_defaults_map, parse_live_backend,
    resolve_live_backend_base_url, AnalyzeLiveSymbolDefaults,
};
pub use options_summary::fetch_options_summary_with_fallback;
pub use sop_reports::{
    build_expansion_sop_market_report, build_expansion_sop_report, build_futures_sop_market_report,
    build_futures_sop_report, run_expansion_sop_with, run_futures_sop_with,
    BuildExpansionSopMarketReportInput, BuildExpansionSopReportInput, BuildFuturesSopReportInput,
    ExpansionFactorLeaderboardEntry, ExpansionMarketReport, ExpansionSopMarketInput,
    ExpansionSopReport, FuturesSopFactorLeaderboardEntry, FuturesSopMarketInput,
    FuturesSopMarketReport, FuturesSopReport, FuturesSopScorecard, RunExpansionSopInput,
};
pub use source_freshness::{classify_freshness, DataFreshness};
pub use source_health::{build_source_health, SourceHealth};
pub use source_snapshot::{build_source_snapshot, SourceSnapshot};
