pub mod backtest;
pub mod engine;
pub mod factor_definition;
pub mod metrics;
pub mod research;
pub mod sparse;
pub mod tucker;
pub mod tucker_driver;
pub mod tucker_persistence;

pub use backtest::{
    BacktestConfig, BacktestResult, FactorBacktestEngine, FactorBacktestResult, FactorTrade,
    WalkForwardWindow,
};
pub use engine::{
    FactorContribution, FactorDiagnostics, FactorEngine, FactorEngineOutput, FactorResearchEngine,
};
pub use factor_definition::{
    FactorCategory, FactorContext, FactorDefinition, FactorSeries, FactorSignal,
    PairedMarketQualityReport,
};
pub use metrics::BacktestMetrics;
pub use research::{FactorLab, ResearchReport};
pub use sparse::{
    adaptive_lambda, sparse_select_by_softshrink, sparsity_ratio_within_bounds, SparseSelection,
    MECE_SPARSITY_LOWER_BOUND, MECE_SPARSITY_UPPER_BOUND,
};
pub use tucker::{fit_tucker_core, TuckerCore};
pub use tucker_driver::{
    build_factor_tensor_from_learning_state, build_factor_tensor_from_state_dir, default_ranks,
    fit_tucker_core_from_state_dir, FactorTensor, DEFAULT_METRIC_AXIS_LABELS,
};
pub use tucker_persistence::{
    build_factor_tucker_core_artifact, persist_factor_tucker_core_artifact,
    tucker_attribution_confidence_is_high, FactorTuckerCoreArtifact,
    FACTOR_TUCKER_CORE_ARTIFACT_FILE, FACTOR_TUCKER_CORE_ARTIFACT_KIND,
    FACTOR_TUCKER_CORE_LEDGER_VERSION, FACTOR_TUCKER_CORE_REVIEW_RULE_VERSION,
    TUCKER_ATTRIBUTION_CONFIDENCE_CAP,
};
