use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;

use crate::types::{Candle, Timeframe};

use super::{provider::RealtimeDataProvider, Quote};

pub struct AggregatedRealtimeProvider {
    providers: Vec<Box<dyn RealtimeDataProvider>>,
}

impl AggregatedRealtimeProvider {
    pub fn new(providers: Vec<Box<dyn RealtimeDataProvider>>) -> Self {
        Self { providers }
    }

    fn first(&self) -> Result<&dyn RealtimeDataProvider> {
        self.providers
            .first()
            .map(|p| p.as_ref())
            .ok_or_else(|| anyhow!("no realtime providers configured"))
    }
}

#[async_trait::async_trait]
impl RealtimeDataProvider for AggregatedRealtimeProvider {
    async fn fetch_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.first()?
            .fetch_candles(symbol, timeframe, start, end)
            .await
    }

    async fn subscribe_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> Result<BoxStream<'static, Candle>> {
        self.first()?.subscribe_candles(symbol, timeframe).await
    }

    async fn get_quote(&self, symbol: &str) -> Result<Quote> {
        self.first()?.get_quote(symbol).await
    }

    async fn health_check(&self) -> Result<bool> {
        self.first()?.health_check().await
    }
}
