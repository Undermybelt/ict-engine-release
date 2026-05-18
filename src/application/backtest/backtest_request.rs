use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct BacktestRequest {
    pub symbol: String,
    pub market: String,
    pub interval: String,
    pub window: String,
    pub objective: String,
    pub factor_filter: Vec<String>,
    pub regime_filter: Vec<String>,
    pub use_multi_timeframe: bool,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct BacktestRequestInput {
    pub symbol: String,
    pub market: String,
    pub interval: String,
    pub window: String,
    pub objective: String,
    pub factor_filter: Vec<String>,
    pub regime_filter: Vec<String>,
    pub use_multi_timeframe: bool,
    pub source: String,
}

pub fn build_backtest_request(input: BacktestRequestInput) -> BacktestRequest {
    BacktestRequest {
        symbol: input.symbol,
        market: input.market,
        interval: input.interval,
        window: input.window,
        objective: input.objective,
        factor_filter: input.factor_filter,
        regime_filter: input.regime_filter,
        use_multi_timeframe: input.use_multi_timeframe,
        source: input.source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtest_request_builder_keeps_market() {
        let req = build_backtest_request(BacktestRequestInput {
            symbol: "NQ".to_string(),
            market: "futures".to_string(),
            interval: "15m".to_string(),
            window: "2024Q1".to_string(),
            objective: "generic".to_string(),
            factor_filter: vec![],
            regime_filter: vec![],
            use_multi_timeframe: true,
            source: "history".to_string(),
        });
        assert_eq!(req.market, "futures");
        assert!(req.use_multi_timeframe);
    }
}
