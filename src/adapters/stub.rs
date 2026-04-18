use std::collections::BTreeMap;

use crate::adapters::contract::{
    ExternalMarketDataAdapter, ExternalToolResult, ToolCatalogEntry,
};
use crate::adapters::catalog::read_only_market_data_stub_catalog;

#[derive(Debug, Default, Clone)]
pub struct StubMarketDataAdapter;

impl StubMarketDataAdapter {
    fn ok(operation: &str, data: serde_json::Value) -> ExternalToolResult {
        ExternalToolResult {
            ok: true,
            provider: "stub".to_string(),
            operation: operation.to_string(),
            data,
            error: None,
        }
    }
}

impl ExternalMarketDataAdapter for StubMarketDataAdapter {
    fn provider_name(&self) -> &'static str {
        "stub"
    }

    fn tool_catalog(&self) -> Vec<ToolCatalogEntry> {
        read_only_market_data_stub_catalog()
    }

    fn fetch_ticker(
        &self,
        symbol: &str,
        _params: BTreeMap<String, String>,
    ) -> anyhow::Result<ExternalToolResult> {
        Ok(Self::ok(
            "ticker.fetch",
            serde_json::json!({
                "symbol": symbol,
                "last": 42000.0,
                "bid": 41999.5,
                "ask": 42000.5
            }),
        ))
    }

    fn fetch_ohlc(
        &self,
        symbol: &str,
        params: BTreeMap<String, String>,
    ) -> anyhow::Result<ExternalToolResult> {
        let interval = params
            .get("interval")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(60);
        Ok(Self::ok(
            "ohlc.fetch",
            serde_json::json!({
                "symbol": symbol,
                "interval": interval,
                "candles": [
                    {"open": 41900.0, "high": 42100.0, "low": 41850.0, "close": 42000.0, "volume": 100.0}
                ]
            }),
        ))
    }

    fn fetch_orderbook(
        &self,
        symbol: &str,
        params: BTreeMap<String, String>,
    ) -> anyhow::Result<ExternalToolResult> {
        let depth = params
            .get("depth")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(10);
        Ok(Self::ok(
            "orderbook.fetch",
            serde_json::json!({
                "symbol": symbol,
                "depth": depth,
                "bids": [[41999.5, 1.0]],
                "asks": [[42000.5, 1.2]]
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_adapter_returns_json_envelopes() {
        let adapter = StubMarketDataAdapter;
        let ticker = adapter.fetch_ticker("BTCUSD", BTreeMap::new()).unwrap();
        assert!(ticker.ok);
        assert_eq!(ticker.provider, "stub");
        assert_eq!(ticker.operation, "ticker.fetch");
        assert_eq!(ticker.data["symbol"], "BTCUSD");
    }
}
