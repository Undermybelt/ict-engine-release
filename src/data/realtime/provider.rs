use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;

use crate::types::{Candle, Timeframe};

use super::openalice::Quote;

#[async_trait]
pub trait RealtimeDataProvider: Send + Sync {
    async fn fetch_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>>;

    async fn subscribe_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> Result<BoxStream<'static, Candle>>;

    async fn get_quote(&self, symbol: &str) -> Result<Quote>;

    async fn health_check(&self) -> Result<bool>;
}
