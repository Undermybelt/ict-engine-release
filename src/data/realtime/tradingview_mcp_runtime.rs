use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use futures::stream::{self, BoxStream};
use serde_json::Value;

use crate::application::data_sources::tradingview_mcp::{
    fetch_tradingview_ohlcv_with_client, tradingview_interval, TradingViewMcpClient,
};
use crate::types::{Candle, Timeframe};

use super::{
    market_support::{
        apply_auxiliary_evidence_to_outcome, build_auxiliary_evidence, AuxiliaryMarketEvidence,
        OptionsChainSummary, Quote, SpotInstrumentKind,
    },
    provider::RealtimeDataProvider,
};

pub struct TradingViewMcpRuntimeProvider {
    client: TradingViewMcpClient,
}

impl TradingViewMcpRuntimeProvider {
    pub fn new(_base_url: impl Into<String>) -> Self {
        Self {
            client: TradingViewMcpClient::from_env_or_local(),
        }
    }

    pub fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_candles(symbol, interval, start, end)
    }

    pub fn fetch_futures_quote(&self, symbol: &str) -> Result<Quote> {
        self.fetch_quote(symbol)
    }

    pub fn fetch_spot_candles(
        &self,
        _kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_candles(symbol, interval.unwrap_or("1d"), start, end)
    }

    pub fn fetch_spot_quote(&self, _kind: SpotInstrumentKind, symbol: &str) -> Result<Quote> {
        self.fetch_quote(symbol)
    }

    pub fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        bail!(
            "tradingview_mcp runtime does not provide options chain summary for '{}'",
            symbol
        )
    }

    pub fn fetch_options_volatility_proxy_summary(
        &self,
        proxy_symbol: &str,
        _underlying_symbol: &str,
    ) -> Result<OptionsChainSummary> {
        let quote = self.fetch_quote(proxy_symbol)?;
        Ok(OptionsChainSummary {
            symbol: proxy_symbol.to_string(),
            source: Some("tradingview_mcp:yahoo_price_volatility_proxy".to_string()),
            underlying_price: Some(quote.last),
            call_open_interest: 0.0,
            put_open_interest: 0.0,
            put_call_oi_ratio: None,
            call_volume: 0.0,
            put_volume: 0.0,
            put_call_volume_ratio: None,
            near_atm_implied_volatility: Some((quote.last / 100.0).max(0.0)),
            near_atm_delta: None,
            near_atm_gamma: None,
            near_atm_vega: None,
            call_gamma_oi: None,
            put_gamma_oi: None,
            gamma_skew: None,
            nearest_expiration_dte: None,
        })
    }

    pub fn build_auxiliary_evidence(
        &self,
        spot_kind: SpotInstrumentKind,
        spot_symbol: &str,
        options_symbol: &str,
        futures_candles: &[Candle],
        spot_candles: &[Candle],
        options_summary: &OptionsChainSummary,
    ) -> AuxiliaryMarketEvidence {
        build_auxiliary_evidence(
            spot_kind,
            spot_symbol,
            options_symbol,
            futures_candles,
            spot_candles,
            options_summary,
        )
    }

    pub fn apply_auxiliary_evidence_to_outcome(
        &self,
        base_distribution: &[f64],
        directional_bias: f64,
        uncertainty_penalty: f64,
    ) -> Vec<f64> {
        apply_auxiliary_evidence_to_outcome(
            base_distribution,
            directional_bias,
            uncertainty_penalty,
        )
    }

    fn fetch_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        let count = estimate_count(interval, start, end);
        fetch_tradingview_ohlcv_with_client(&self.client, symbol, interval, start, end, count)
    }

    fn fetch_quote(&self, symbol: &str) -> Result<Quote> {
        if is_yahoo_style_symbol(symbol) {
            if let Ok(payload) = self.client.call_tool(
                "yahoo_price",
                serde_json::json!({
                    "symbol": symbol,
                }),
            ) {
                if let Some(quote) = quote_from_yahoo_payload(symbol, &payload) {
                    return Ok(quote);
                }
            }
        }

        let end = Utc::now();
        let start = end - chrono::TimeDelta::days(10);
        let candle =
            fetch_tradingview_ohlcv_with_client(&self.client, symbol, "1d", start, end, 2)?
                .into_iter()
                .last()
                .ok_or_else(|| anyhow::anyhow!("tradingview_mcp returned no quote candle"))?;
        Ok(Quote {
            symbol: symbol.to_string(),
            bid: candle.close,
            ask: candle.close,
            last: candle.close,
            timestamp: candle.timestamp,
        })
    }
}

#[async_trait::async_trait]
impl RealtimeDataProvider for TradingViewMcpRuntimeProvider {
    async fn fetch_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_candles(symbol, timeframe_to_interval(timeframe), start, end)
    }

    async fn subscribe_candles(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
    ) -> Result<BoxStream<'static, Candle>> {
        Ok(Box::pin(stream::empty()))
    }

    async fn get_quote(&self, symbol: &str) -> Result<Quote> {
        self.fetch_quote(symbol)
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

fn timeframe_to_interval(timeframe: Timeframe) -> &'static str {
    match timeframe {
        Timeframe::M15 => "15m",
        Timeframe::H1 => "1h",
        Timeframe::H4 => "1d",
        Timeframe::D1 => "1d",
    }
}

fn quote_from_yahoo_payload(symbol: &str, payload: &Value) -> Option<Quote> {
    let price = payload.get("price").and_then(Value::as_f64)?;
    Some(Quote {
        symbol: payload
            .get("symbol")
            .and_then(Value::as_str)
            .unwrap_or(symbol)
            .to_string(),
        bid: price,
        ask: price,
        last: price,
        timestamp: Utc::now(),
    })
}

fn is_yahoo_style_symbol(symbol: &str) -> bool {
    !symbol.contains(':')
}

fn estimate_count(interval: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> usize {
    let span_minutes = (end - start).num_minutes().max(1);
    let interval_minutes = match tradingview_interval(interval) {
        "1m" => 1,
        "2m" => 2,
        "5m" => 5,
        "15m" => 15,
        "30m" => 30,
        "1h" => 60,
        "90m" => 90,
        "1d" => 1_440,
        _ => 1_440,
    };
    ((span_minutes / interval_minutes) + 8).clamp(10, 5000) as usize
}
