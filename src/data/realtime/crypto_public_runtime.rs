use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use futures::stream::{self, BoxStream};
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::types::{Candle, Timeframe};

use super::{
    market_support::{
        apply_auxiliary_evidence_to_outcome, build_auxiliary_evidence, AuxiliaryMarketEvidence,
        OptionsChainSummary, Quote, SpotInstrumentKind,
    },
    provider::RealtimeDataProvider,
};

const COINANK_API_URL: &str = "https://api.coinank.com/api/kline/list/open";
const HYPERLIQUID_API_URL: &str = "https://api.hyperliquid.xyz/info";

pub struct CryptoPublicRuntimeProvider {
    client: Client,
}

impl CryptoPublicRuntimeProvider {
    pub fn new(_base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("ict-engine/0.1")
                .build()
                .expect("failed to build reqwest blocking client"),
        }
    }

    pub fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        match parse_crypto_public_source(symbol) {
            CryptoPublicSource::Hyperliquid { coin } => {
                self.fetch_hyperliquid_candles(&coin, interval)
            }
            CryptoPublicSource::Coinank { exchange, symbol } => {
                self.fetch_coinank_candles(&symbol, &exchange, interval)
            }
        }
    }

    pub fn fetch_spot_candles(
        &self,
        _kind: SpotInstrumentKind,
        _symbol: &str,
        _interval: Option<&str>,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        bail!("crypto_public runtime does not provide spot auxiliary data")
    }

    pub fn fetch_options_chain_summary(&self, _symbol: &str) -> Result<OptionsChainSummary> {
        bail!("crypto_public runtime does not provide options chain data")
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

    fn fetch_coinank_candles(
        &self,
        symbol: &str,
        exchange: &str,
        interval: &str,
    ) -> Result<Vec<Candle>> {
        let response: CoinankResponse = self
            .client
            .get(COINANK_API_URL)
            .query(&[
                ("symbol", symbol.to_string()),
                ("exchange", exchange.to_string()),
                ("side", "To".to_string()),
                ("size", interval_limit(interval).to_string()),
                ("ts", Utc::now().timestamp_millis().to_string()),
                ("interval", normalize_coinank_interval(interval).to_string()),
            ])
            .send()
            .with_context(|| format!("failed to request CoinAnk candles for '{}'", symbol))?
            .error_for_status()
            .with_context(|| format!("CoinAnk returned error for '{}'", symbol))?
            .json()
            .context("failed to parse CoinAnk response")?;

        if !response.success {
            bail!("CoinAnk reported failure for '{}'", symbol);
        }

        let candles = response
            .data
            .into_iter()
            .filter(|row| row.len() >= 9)
            .map(|row| {
                let open_time = row[0] as i64;
                let timestamp = Utc
                    .timestamp_millis_opt(open_time)
                    .single()
                    .ok_or_else(|| anyhow!("invalid CoinAnk timestamp '{}'", open_time))?;
                Ok(Candle {
                    timestamp,
                    open: row[2],
                    high: row[4],
                    low: row[5],
                    close: row[3],
                    volume: row[6],
                })
            })
            .collect::<Result<Vec<_>>>()?;

        if candles.is_empty() {
            bail!("CoinAnk returned no usable candles for '{}'", symbol);
        }

        Ok(candles)
    }

    fn fetch_hyperliquid_candles(&self, coin: &str, interval: &str) -> Result<Vec<Candle>> {
        let interval = normalize_hyperliquid_interval(interval);
        let interval_millis = interval_duration_millis(interval);
        let limit = interval_limit(interval) as i64;
        let end_time = Utc::now().timestamp_millis();
        let start_time = end_time - interval_millis * limit;

        let response = self
            .client
            .post(HYPERLIQUID_API_URL)
            .json(&serde_json::json!({
                "type": "candleSnapshot",
                "req": {
                    "coin": normalize_hyperliquid_symbol(coin),
                    "interval": interval,
                    "startTime": start_time,
                    "endTime": end_time
                }
            }))
            .send()
            .with_context(|| format!("failed to request Hyperliquid candles for '{}'", coin))?
            .error_for_status()
            .with_context(|| format!("Hyperliquid returned error for '{}'", coin))?;

        let rows: Vec<HyperliquidCandle> = response
            .json()
            .context("failed to parse Hyperliquid candle response")?;

        let candles = rows
            .into_iter()
            .map(|row| {
                let timestamp = Utc
                    .timestamp_millis_opt(row.open_time)
                    .single()
                    .ok_or_else(|| anyhow!("invalid Hyperliquid timestamp '{}'", row.open_time))?;
                Ok(Candle {
                    timestamp,
                    open: row.open.parse().context("invalid Hyperliquid open")?,
                    high: row.high.parse().context("invalid Hyperliquid high")?,
                    low: row.low.parse().context("invalid Hyperliquid low")?,
                    close: row.close.parse().context("invalid Hyperliquid close")?,
                    volume: row.volume.parse().unwrap_or(0.0),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        if candles.is_empty() {
            bail!("Hyperliquid returned no usable candles for '{}'", coin);
        }

        Ok(candles)
    }
}

#[async_trait::async_trait]
impl RealtimeDataProvider for CryptoPublicRuntimeProvider {
    async fn fetch_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_futures_candles(symbol, timeframe_to_interval(timeframe), start, end)
    }

    async fn subscribe_candles(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
    ) -> Result<BoxStream<'static, Candle>> {
        Ok(Box::pin(stream::empty()))
    }

    async fn get_quote(&self, symbol: &str) -> Result<Quote> {
        let candles = self.fetch_futures_candles(
            symbol,
            "1m",
            Utc::now() - chrono::Duration::hours(2),
            Utc::now(),
        )?;
        let last = candles
            .last()
            .ok_or_else(|| anyhow!("crypto_public runtime returned no quoteable candles"))?;
        Ok(Quote {
            symbol: symbol.to_string(),
            bid: last.close,
            ask: last.close,
            last: last.close,
            timestamp: last.timestamp,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        let response = self.client.get("https://api.coinank.com").send();
        Ok(
            matches!(response, Ok(resp) if resp.status().is_success() || resp.status().is_redirection()),
        )
    }
}

#[derive(Debug, Deserialize)]
struct CoinankResponse {
    success: bool,
    #[serde(default)]
    data: Vec<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
struct HyperliquidCandle {
    #[serde(rename = "t")]
    open_time: i64,
    #[serde(rename = "o")]
    open: String,
    #[serde(rename = "h")]
    high: String,
    #[serde(rename = "l")]
    low: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "v")]
    volume: String,
}

enum CryptoPublicSource {
    Hyperliquid { coin: String },
    Coinank { exchange: String, symbol: String },
}

fn parse_crypto_public_source(symbol: &str) -> CryptoPublicSource {
    let raw = symbol.trim();
    if let Some(rest) = raw.strip_prefix("hyperliquid:") {
        return CryptoPublicSource::Hyperliquid {
            coin: rest.to_string(),
        };
    }
    if raw.starts_with("xyz:") {
        return CryptoPublicSource::Hyperliquid {
            coin: raw.to_string(),
        };
    }
    if let Some((exchange, base_symbol)) = raw.split_once(':') {
        return CryptoPublicSource::Coinank {
            exchange: normalize_coinank_exchange(exchange),
            symbol: base_symbol.to_uppercase(),
        };
    }

    CryptoPublicSource::Coinank {
        exchange: "Binance".to_string(),
        symbol: raw.to_uppercase(),
    }
}

fn normalize_coinank_exchange(exchange: &str) -> String {
    match exchange.trim().to_ascii_lowercase().as_str() {
        "binance" => "Binance".to_string(),
        "bybit" => "Bybit".to_string(),
        "okx" => "Okex".to_string(),
        "bitget" => "Bitget".to_string(),
        "gate" => "Gate".to_string(),
        "hyperliquid" => "Hyperliquid".to_string(),
        "aster" => "Aster".to_string(),
        _ => "Binance".to_string(),
    }
}

fn normalize_coinank_interval(interval: &str) -> &'static str {
    match interval {
        "1m" => "1m",
        "3m" => "3m",
        "5m" => "5m",
        "15m" => "15m",
        "30m" => "30m",
        "1h" => "1h",
        "2h" => "2h",
        "4h" => "4h",
        "6h" => "6h",
        "8h" => "8h",
        "12h" => "12h",
        "1d" => "1d",
        "3d" => "3d",
        "1w" => "1w",
        _ => "1h",
    }
}

fn normalize_hyperliquid_interval(interval: &str) -> &'static str {
    match interval {
        "1m" => "1m",
        "5m" | "3m" => "5m",
        "15m" => "15m",
        "30m" => "30m",
        "1h" | "2h" => "1h",
        "4h" | "6h" => "4h",
        "8h" => "8h",
        "12h" => "12h",
        "1d" | "3d" => "1d",
        "1w" => "1w",
        _ => "1h",
    }
}

fn normalize_hyperliquid_symbol(symbol: &str) -> String {
    let upper = symbol.trim().to_uppercase();
    if let Some(stripped) = upper.strip_prefix("XYZ:") {
        return format!("xyz:{}", stripped);
    }
    if upper.ends_with("USDT") {
        return upper.trim_end_matches("USDT").to_string();
    }
    if upper.ends_with("USD") {
        return upper.trim_end_matches("USD").to_string();
    }
    upper
}

fn timeframe_to_interval(timeframe: Timeframe) -> &'static str {
    match timeframe {
        Timeframe::M15 => "15m",
        Timeframe::H1 => "1h",
        Timeframe::H4 => "4h",
        Timeframe::D1 => "1d",
    }
}

fn interval_limit(interval: &str) -> usize {
    match interval {
        "1d" => 420,
        "4h" => 420,
        "1h" => 1000,
        "15m" => 1200,
        _ => 1000,
    }
}

fn interval_duration_millis(interval: &str) -> i64 {
    match interval {
        "1m" => 60_000,
        "5m" => 300_000,
        "15m" => 900_000,
        "30m" => 1_800_000,
        "1h" => 3_600_000,
        "4h" => 14_400_000,
        "8h" => 28_800_000,
        "12h" => 43_200_000,
        "1d" => 86_400_000,
        "1w" => 604_800_000,
        _ => 3_600_000,
    }
}
