use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use futures::stream::{self, BoxStream};
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::types::{Candle, Timeframe};

use super::provider::RealtimeDataProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SentimentData {
    pub symbol: String,
    pub score: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EconomicEvent {
    pub name: String,
    pub impact: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct COTData {
    pub symbol: String,
    pub net_position: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewsItem {
    pub title: String,
    pub summary: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpotInstrumentKind {
    Equity,
    Index,
    Commodity,
}

impl SpotInstrumentKind {
    pub fn parse(input: &str) -> Result<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "equity" | "stock" | "etf" => Ok(Self::Equity),
            "index" => Ok(Self::Index),
            "commodity" | "spot" => Ok(Self::Commodity),
            other => bail!("unsupported spot instrument kind '{}'", other),
        }
    }

    fn historical_path(self) -> &'static str {
        match self {
            Self::Equity => "/equity/price/historical",
            Self::Index => "/index/price/historical",
            Self::Commodity => "/commodity/price/spot",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsChainSummary {
    pub symbol: String,
    pub underlying_price: Option<f64>,
    pub call_open_interest: f64,
    pub put_open_interest: f64,
    pub put_call_oi_ratio: Option<f64>,
    pub call_volume: f64,
    pub put_volume: f64,
    pub put_call_volume_ratio: Option<f64>,
    pub near_atm_implied_volatility: Option<f64>,
    pub near_atm_delta: Option<f64>,
    pub near_atm_gamma: Option<f64>,
    pub near_atm_vega: Option<f64>,
    pub call_gamma_oi: Option<f64>,
    pub put_gamma_oi: Option<f64>,
    pub gamma_skew: Option<f64>,
    pub nearest_expiration_dte: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuxiliaryMarketEvidence {
    pub spot_symbol: String,
    pub options_symbol: String,
    pub spot_kind: SpotInstrumentKind,
    pub spot_last_close: Option<f64>,
    pub futures_last_close: Option<f64>,
    pub spot_return: Option<f64>,
    pub futures_return: Option<f64>,
    pub raw_basis_bps: Option<f64>,
    pub normalized_basis_bps: Option<f64>,
    pub rolling_price_ratio_mean: Option<f64>,
    pub put_call_oi_ratio: Option<f64>,
    pub put_call_volume_ratio: Option<f64>,
    pub near_atm_implied_volatility: Option<f64>,
    pub near_atm_delta: Option<f64>,
    pub near_atm_gamma: Option<f64>,
    pub near_atm_vega: Option<f64>,
    pub call_gamma_oi: Option<f64>,
    pub put_gamma_oi: Option<f64>,
    pub gamma_skew: Option<f64>,
    pub hedge_pressure_direction: Option<String>,
    pub hedge_pressure_score: Option<f64>,
    pub long_bias: f64,
    pub short_bias: f64,
    pub uncertainty_penalty: f64,
    pub notes: Vec<String>,
}

pub struct OpenAliceProvider {
    pub base_url: String,
    pub api_key: Option<String>,
    pub client: Client,
}

impl OpenAliceProvider {
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
        let futures_last_close = futures_candles.last().map(|candle| candle.close);
        let spot_last_close = spot_candles.last().map(|candle| candle.close);
        let futures_return = trailing_return(futures_candles, 5);
        let spot_return = trailing_return(spot_candles, 5);
        let raw_basis_bps = match (futures_last_close, spot_last_close) {
            (Some(future), Some(spot)) if spot.abs() > f64::EPSILON => {
                Some((future - spot) / spot * 10_000.0)
            }
            _ => None,
        };
        let ratio_stats = rolling_price_ratio_stats(futures_candles, spot_candles, 96);

        let mut long_bias: f64 = 0.0;
        let mut short_bias: f64 = 0.0;
        let mut uncertainty_penalty: f64 = 0.0;
        let mut notes = Vec::new();

        match (spot_return, futures_return) {
            (Some(spot_ret), Some(fut_ret)) if spot_ret > 0.0 && fut_ret > 0.0 => {
                long_bias += 0.06;
                notes.push("spot_trend_confirms_long".to_string());
            }
            (Some(spot_ret), Some(fut_ret)) if spot_ret < 0.0 && fut_ret < 0.0 => {
                short_bias += 0.06;
                notes.push("spot_trend_confirms_short".to_string());
            }
            (Some(_), Some(_)) => {
                uncertainty_penalty += 0.03;
                notes.push("spot_futures_divergence".to_string());
            }
            _ => {}
        }

        if let Some(basis) = ratio_stats
            .as_ref()
            .and_then(|stats| stats.normalized_basis_bps)
            .or(raw_basis_bps)
        {
            if basis.abs() > 150.0 {
                uncertainty_penalty += 0.05;
                notes.push("elevated_basis".to_string());
            }
        }

        if let Some(pcr) = options_summary.put_call_oi_ratio {
            if pcr > 1.15 {
                short_bias += 0.08;
                notes.push("options_put_skew".to_string());
            } else if pcr < 0.85 {
                long_bias += 0.08;
                notes.push("options_call_skew".to_string());
            }
        }
        if let Some(pcr_vol) = options_summary.put_call_volume_ratio {
            if pcr_vol > 1.20 {
                short_bias += 0.04;
                notes.push("put_volume_dominates_call_volume".to_string());
            } else if pcr_vol < 0.80 {
                long_bias += 0.04;
                notes.push("call_volume_dominates_put_volume".to_string());
            }
        }

        if let Some(iv) = options_summary.near_atm_implied_volatility {
            if iv > 0.45 {
                uncertainty_penalty += 0.05;
                notes.push("high_options_iv".to_string());
            }
            if options_summary.put_call_oi_ratio.is_none()
                && options_summary.put_call_volume_ratio.is_none()
            {
                notes.push("options_volatility_proxy_only".to_string());
            }
        }

        let hedge_pressure_direction = options_summary.gamma_skew.map(|gamma_skew| {
            if gamma_skew > 0.0 {
                "bullish".to_string()
            } else if gamma_skew < 0.0 {
                "bearish".to_string()
            } else {
                "neutral".to_string()
            }
        });
        let hedge_pressure_score = options_summary
            .gamma_skew
            .map(|gamma_skew| gamma_skew.tanh());

        if let Some(gamma_skew) = options_summary.gamma_skew {
            if gamma_skew > 0.15 {
                long_bias += 0.03;
                notes.push("call_gamma_skew_supports_upside_hedging".to_string());
            } else if gamma_skew < -0.15 {
                short_bias += 0.03;
                notes.push("put_gamma_skew_supports_downside_hedging".to_string());
            }
        }
        if let (Some(pcr), Some(gamma_skew)) = (
            options_summary.put_call_oi_ratio,
            options_summary.gamma_skew,
        ) {
            if pcr > 1.20 && gamma_skew < 0.0 {
                short_bias += 0.03;
                notes.push("put_skew_and_negative_gamma_align_bearishly".to_string());
            } else if pcr < 0.85 && gamma_skew > 0.0 {
                long_bias += 0.03;
                notes.push("call_skew_and_positive_gamma_align_bullishly".to_string());
            }
        }
        if let (Some(iv), Some(gamma_skew)) = (
            options_summary.near_atm_implied_volatility,
            options_summary.gamma_skew,
        ) {
            if iv > 0.35 && gamma_skew.abs() > 0.10 {
                uncertainty_penalty += 0.03;
                notes.push("gamma_iv_combo_can_amplify_hedging_flows".to_string());
            }
        }

        AuxiliaryMarketEvidence {
            spot_symbol: spot_symbol.to_string(),
            options_symbol: options_symbol.to_string(),
            spot_kind,
            spot_last_close,
            futures_last_close,
            spot_return,
            futures_return,
            raw_basis_bps,
            normalized_basis_bps: ratio_stats
                .as_ref()
                .and_then(|stats| stats.normalized_basis_bps),
            rolling_price_ratio_mean: ratio_stats.as_ref().map(|stats| stats.rolling_mean),
            put_call_oi_ratio: options_summary.put_call_oi_ratio,
            put_call_volume_ratio: options_summary.put_call_volume_ratio,
            near_atm_implied_volatility: options_summary.near_atm_implied_volatility,
            near_atm_delta: options_summary.near_atm_delta,
            near_atm_gamma: options_summary.near_atm_gamma,
            near_atm_vega: options_summary.near_atm_vega,
            call_gamma_oi: options_summary.call_gamma_oi,
            put_gamma_oi: options_summary.put_gamma_oi,
            gamma_skew: options_summary.gamma_skew,
            hedge_pressure_direction,
            hedge_pressure_score,
            long_bias: long_bias.min(0.20),
            short_bias: short_bias.min(0.20),
            uncertainty_penalty: uncertainty_penalty.min(0.20),
            notes,
        }
    }

    pub fn apply_auxiliary_evidence_to_outcome(
        &self,
        base_distribution: &[f64],
        directional_bias: f64,
        uncertainty_penalty: f64,
    ) -> Vec<f64> {
        let mut distribution = if base_distribution.len() == 3 {
            base_distribution.to_vec()
        } else {
            vec![0.0, 0.0, 1.0]
        };

        let directional_bias = directional_bias.clamp(-0.20, 0.20);
        if directional_bias > 0.0 {
            let shift = distribution[2].min(directional_bias);
            distribution[2] -= shift;
            distribution[0] += shift;
        } else if directional_bias < 0.0 {
            let shift = distribution[0].min(-directional_bias);
            distribution[0] -= shift;
            distribution[2] += shift;
        }

        let penalty = uncertainty_penalty.clamp(0.0, 0.20);
        if penalty > 0.0 {
            let win_to_remove = distribution[0] * penalty;
            distribution[0] -= win_to_remove;
            distribution[1] += win_to_remove * 0.6;
            distribution[2] += win_to_remove * 0.4;
        }

        normalize(&mut distribution);
        distribution
    }

    pub async fn get_sentiment(&self, symbol: &str) -> Result<SentimentData> {
        Ok(SentimentData {
            symbol: symbol.to_string(),
            score: 0.0,
            confidence: 0.0,
        })
    }

    pub async fn get_economic_calendar(&self) -> Result<Vec<EconomicEvent>> {
        Ok(Vec::new())
    }

    pub async fn get_cot_data(&self, symbol: &str) -> Result<COTData> {
        Ok(COTData {
            symbol: symbol.to_string(),
            net_position: 0.0,
            timestamp: Utc::now(),
        })
    }

    pub async fn get_news(&self, _symbol: &str, _limit: usize) -> Result<Vec<NewsItem>> {
        Ok(Vec::new())
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
            .with_context(|| format!("failed to request OpenAlice path '{}'", path))?;
        let status = response.status();
        let body = response
            .text()
            .with_context(|| format!("failed to read OpenAlice response for '{}'", path))?;

        if !status.is_success() {
            bail!(
                "OpenAlice request to '{}' failed with status {}: {}",
                path,
                status,
                body
            );
        }

        serde_json::from_str(&body)
            .with_context(|| format!("failed to parse OpenAlice JSON for '{}': {}", path, body))
    }
}

#[async_trait::async_trait]
impl RealtimeDataProvider for OpenAliceProvider {
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

fn trailing_return(candles: &[Candle], lookback: usize) -> Option<f64> {
    if candles.len() <= lookback {
        return None;
    }

    let last = candles.last()?.close;
    let previous = candles[candles.len().saturating_sub(1 + lookback)].close;
    if previous.abs() <= f64::EPSILON {
        return None;
    }

    Some((last - previous) / previous)
}

struct PriceRatioStats {
    rolling_mean: f64,
    normalized_basis_bps: Option<f64>,
}

fn rolling_price_ratio_stats(
    futures_candles: &[Candle],
    spot_candles: &[Candle],
    window: usize,
) -> Option<PriceRatioStats> {
    let len = futures_candles.len().min(spot_candles.len());
    if len < 5 {
        return None;
    }

    let start = len.saturating_sub(window);
    let ratios = futures_candles[futures_candles.len() - len + start..]
        .iter()
        .zip(spot_candles[spot_candles.len() - len + start..].iter())
        .filter_map(|(future, spot)| {
            if spot.close.abs() <= f64::EPSILON {
                None
            } else {
                Some(future.close / spot.close)
            }
        })
        .collect::<Vec<_>>();

    if ratios.len() < 3 {
        return None;
    }

    let latest_ratio = *ratios.last()?;
    let rolling_mean = ratios.iter().sum::<f64>() / ratios.len() as f64;
    let normalized_basis_bps = if rolling_mean.abs() > f64::EPSILON {
        Some((latest_ratio / rolling_mean - 1.0) * 10_000.0)
    } else {
        None
    };

    Some(PriceRatioStats {
        rolling_mean,
        normalized_basis_bps,
    })
}

fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator.abs() <= f64::EPSILON {
        None
    } else {
        Some(numerator / denominator)
    }
}

fn normalize(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum <= f64::EPSILON {
        let uniform = 1.0 / values.len() as f64;
        values.fill(uniform);
        return;
    }

    for value in values.iter_mut() {
        *value /= sum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_auxiliary_evidence_to_outcome_biases_direction() {
        let provider = OpenAliceProvider::new("http://127.0.0.1:6901/api/v1", None);
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
