pub mod backtest;
pub mod engine;
pub mod factor_definition;
pub mod metrics;
pub mod research;

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
