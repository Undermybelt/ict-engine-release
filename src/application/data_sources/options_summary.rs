use anyhow::Result;

use crate::data::realtime::market_support::OptionsChainSummary;
use crate::data::realtime::IntegratedLiveDataSource;

pub fn fetch_options_summary_with_fallback(
    provider: &dyn IntegratedLiveDataSource,
    options_symbol: &str,
    volatility_proxy_symbol: Option<&str>,
) -> Result<OptionsChainSummary> {
    match provider.fetch_options_chain_summary(options_symbol) {
        Ok(summary) => Ok(summary),
        Err(primary_error) => {
            if let Some(proxy_symbol) = volatility_proxy_symbol {
                provider.fetch_options_volatility_proxy_summary(proxy_symbol, options_symbol)
            } else {
                Err(primary_error)
            }
        }
    }
}
