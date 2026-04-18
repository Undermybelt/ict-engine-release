pub mod backtest_compare;
pub mod backtest_request;
pub mod backtest_result;

pub use backtest_compare::{
    build_oos_quality_delta_surface, build_shrink_on_off_comparison_summary,
    compare_backtest_results, BacktestCompareReport,
};
pub use backtest_request::{build_backtest_request, BacktestRequest, BacktestRequestInput};
pub use backtest_result::{
    build_backtest_result_artifact, BacktestResultArtifact, BacktestResultArtifactInput,
};
