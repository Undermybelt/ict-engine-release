use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::stream::{self, BoxStream};

use crate::types::{Candle, Timeframe};

use super::{market_support::Quote, provider::RealtimeDataProvider};

pub struct TradeCatProvider;

#[async_trait::async_trait]
impl RealtimeDataProvider for TradeCatProvider {
    async fn fetch_candles(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        Ok(Vec::new())
    }

    async fn subscribe_candles(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
    ) -> Result<BoxStream<'static, Candle>> {
        Ok(Box::pin(stream::empty()))
    }

    async fn get_quote(&self, symbol: &str) -> Result<Quote> {
        Ok(Quote {
            symbol: symbol.to_string(),
            bid: 0.0,
            ask: 0.0,
            last: 0.0,
            timestamp: Utc::now(),
        })
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}
