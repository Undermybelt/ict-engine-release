use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use futures::stream::{self, BoxStream};
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Deserialize};

use crate::types::{Candle, Timeframe};

use super::{
    market_support::{
        apply_auxiliary_evidence_to_outcome, build_auxiliary_evidence, AuxiliaryMarketEvidence,
        OptionsChainSummary, Quote, SpotInstrumentKind,
    },
    provider::RealtimeDataProvider,
};

pub struct ExternalHttpRuntimeProvider {
    pub base_url: String,
    pub api_key: Option<String>,
    pub client: Client,
}

impl ExternalHttpRuntimeProvider {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest blocking client"),
        }
    }

    pub fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_historical_candles(
            "/derivatives/futures/historical",
            symbol,
            interval,
            start,
            end,
        )
    }

    pub fn fetch_spot_candles(
        &self,
        kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_historical_candles(
            kind.historical_path(),
            symbol,
            interval.unwrap_or("1d"),
            start,
            end,
        )
    }

    pub fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        let contracts: Vec<OptionsContractRecord> = self.get_json(
            "/derivatives/options/chains",
            &[
                ("provider", "yfinance".to_string()),
                ("symbol", symbol.to_string()),
            ],
        )?;

        if contracts.is_empty() {
            bail!("no options chain returned for '{}'", symbol);
        }

        let underlying_price = contracts
            .iter()
            .find_map(|contract| contract.underlying_price);
        let call_open_interest: f64 = contracts
            .iter()
            .filter(|contract| contract.option_type.eq_ignore_ascii_case("call"))
            .map(|contract| contract.open_interest.unwrap_or(0.0))
            .sum();
        let put_open_interest: f64 = contracts
            .iter()
            .filter(|contract| contract.option_type.eq_ignore_ascii_case("put"))
            .map(|contract| contract.open_interest.unwrap_or(0.0))
            .sum();
        let call_volume: f64 = contracts
            .iter()
            .filter(|contract| contract.option_type.eq_ignore_ascii_case("call"))
            .map(|contract| contract.volume.unwrap_or(0.0))
            .sum();
        let put_volume: f64 = contracts
            .iter()
            .filter(|contract| contract.option_type.eq_ignore_ascii_case("put"))
            .map(|contract| contract.volume.unwrap_or(0.0))
            .sum();
        let nearest_expiration_dte = contracts
            .iter()
            .filter_map(|contract| contract.dte)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let near_atm_implied_volatility = underlying_price.and_then(|price| {
            let mut values = contracts
                .iter()
                .filter(|contract| {
                    contract.dte.unwrap_or(999.0) <= 45.0
                        && (contract.strike - price).abs() / price.max(f64::EPSILON) <= 0.10
                })
                .filter_map(|contract| contract.implied_volatility)
                .collect::<Vec<_>>();
            if values.is_empty() {
                None
            } else {
                Some(values.drain(..).sum::<f64>() / values.len() as f64)
            }
        });

        Ok(OptionsChainSummary {
            symbol: symbol.to_string(),
            source: Some("external_http_runtime:/derivatives/options/chains".to_string()),
            underlying_price,
            call_open_interest,
            put_open_interest,
            put_call_oi_ratio: ratio(put_open_interest, call_open_interest),
            call_volume,
            put_volume,
            put_call_volume_ratio: ratio(put_volume, call_volume),
            near_atm_implied_volatility,
            near_atm_delta: None,
            near_atm_gamma: None,
            near_atm_vega: None,
            call_gamma_oi: None,
            put_gamma_oi: None,
            gamma_skew: None,
            nearest_expiration_dte,
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

    fn fetch_historical_candles(
        &self,
        path: &str,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        let rows: Vec<HistoricalRecord> = self.get_json(
            path,
            &[
                ("provider", "yfinance".to_string()),
                ("symbol", symbol.to_string()),
                ("interval", interval.to_string()),
                ("start_date", start.format("%Y-%m-%d").to_string()),
                ("end_date", end.format("%Y-%m-%d").to_string()),
            ],
        )?;

        let mut candles = rows
            .into_iter()
            .filter_map(|row| {
                let (open, high, low, close) = match (row.open, row.high, row.low, row.close) {
                    (Some(open), Some(high), Some(low), Some(close)) => (open, high, low, close),
                    _ => return None,
                };

                let timestamp = parse_provider_timestamp(&row.date).ok()?;
                Some(Candle {
                    timestamp,
                    open,
                    high,
                    low,
                    close,
                    volume: row.volume.unwrap_or(0.0),
                })
            })
            .collect::<Vec<_>>();

        candles.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
        candles.dedup_by(|left, right| left.timestamp == right.timestamp);

        if candles.is_empty() {
            bail!("no valid candles returned for '{}' at '{}'", symbol, path);
        }

        Ok(candles)
    }

    fn get_json<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.get(url).query(query);
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request
            .send()
            .with_context(|| format!("failed to request external runtime path '{}'", path))?;
        let status = response.status();
        let body = response
            .text()
            .with_context(|| format!("failed to read external runtime response for '{}'", path))?;

        if !status.is_success() {
            bail!(
                "external runtime request to '{}' failed with status {}: {}",
                path,
                status,
                body
            );
        }

        serde_json::from_str(&body).with_context(|| {
            format!(
                "failed to parse external runtime JSON for '{}': {}",
                path, body
            )
        })
    }
}

#[async_trait::async_trait]
impl RealtimeDataProvider for ExternalHttpRuntimeProvider {
    async fn fetch_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_historical_candles(
            "/equity/price/historical",
            symbol,
            timeframe_to_interval(timeframe),
            start,
            end,
        )
    }

    async fn subscribe_candles(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
    ) -> Result<BoxStream<'static, Candle>> {
        Ok(Box::pin(stream::empty()))
    }

    async fn get_quote(&self, symbol: &str) -> Result<Quote> {
        let rows: Vec<QuoteRecord> = self.get_json(
            "/equity/price/quote",
            &[
                ("provider", "yfinance".to_string()),
                ("symbol", symbol.to_string()),
            ],
        )?;
        let quote = rows
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no quote returned for '{}'", symbol))?;

        Ok(Quote {
            symbol: quote.symbol.unwrap_or_else(|| symbol.to_string()),
            bid: quote.bid.unwrap_or(0.0),
            ask: quote.ask.unwrap_or(0.0),
            last: quote.last_price.or(quote.close).unwrap_or(0.0),
            timestamp: quote
                .last_timestamp
                .as_deref()
                .and_then(|value| parse_provider_timestamp(value).ok())
                .unwrap_or_else(Utc::now),
        })
    }

    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/widgets.json", self.base_url.trim_end_matches("/api/v1"));
        let response = self.client.get(url).send();
        Ok(matches!(response, Ok(resp) if resp.status().is_success()))
    }
}

#[derive(Debug, Deserialize)]
struct HistoricalRecord {
    date: String,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    volume: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct QuoteRecord {
    symbol: Option<String>,
    bid: Option<f64>,
    ask: Option<f64>,
    last_price: Option<f64>,
    close: Option<f64>,
    last_timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OptionsContractRecord {
    underlying_price: Option<f64>,
    strike: f64,
    option_type: String,
    open_interest: Option<f64>,
    volume: Option<f64>,
    implied_volatility: Option<f64>,
    dte: Option<f64>,
}

fn timeframe_to_interval(timeframe: Timeframe) -> &'static str {
    match timeframe {
        Timeframe::M15 => "15m",
        Timeframe::H1 => "1h",
        Timeframe::H4 => "1d",
        Timeframe::D1 => "1d",
    }
}

fn parse_provider_timestamp(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Ok(timestamp.with_timezone(&Utc));
    }

    if let Ok(timestamp) = DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f%#z") {
        return Ok(timestamp.with_timezone(&Utc));
    }

    if let Ok(timestamp) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc));
    }

    if let Ok(timestamp) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc));
    }

    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let timestamp = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow!("invalid date '{}'", value))?;
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc));
    }

    bail!("unsupported provider timestamp '{}'", value)
}

fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator.abs() <= f64::EPSILON {
        None
    } else {
        Some(numerator / denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_auxiliary_evidence_to_outcome_biases_direction() {
        let provider = ExternalHttpRuntimeProvider::new("http://127.0.0.1:6901/api/v1", None);
        let adjusted = provider.apply_auxiliary_evidence_to_outcome(&[0.4, 0.2, 0.4], 0.1, 0.0);

        assert!(adjusted[0] > 0.4);
        assert!(adjusted[2] < 0.4);
        assert!((adjusted.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_parse_provider_timestamp_accepts_dates() {
        let parsed = parse_provider_timestamp("2025-01-15").unwrap();
        assert_eq!(parsed.format("%Y-%m-%d").to_string(), "2025-01-15");
    }

    #[test]
    fn test_spot_kind_parse() {
        assert_eq!(
            SpotInstrumentKind::parse("equity").unwrap(),
            SpotInstrumentKind::Equity
        );
        assert_eq!(
            SpotInstrumentKind::parse("index").unwrap(),
            SpotInstrumentKind::Index
        );
        assert!(SpotInstrumentKind::parse("unknown").is_err());
    }
}
