use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use futures::stream::{self, BoxStream};
use reqwest::{
    blocking::{Client, Response},
    header::SET_COOKIE,
};
use serde::Deserialize;

use crate::types::{Candle, Timeframe};

use super::{
    browser_bridge,
    market_support::{
        apply_auxiliary_evidence_to_outcome, build_auxiliary_evidence, AuxiliaryMarketEvidence,
        OptionsChainSummary, Quote, SpotInstrumentKind,
    },
    provider::RealtimeDataProvider,
};

pub struct YahooFinanceProvider {
    client: Client,
}

impl YahooFinanceProvider {
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
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_chart_candles(&resolve_futures_symbol(symbol), interval, start, end)
    }

    pub fn fetch_futures_quote(&self, symbol: &str) -> Result<Quote> {
        self.fetch_quote(&resolve_futures_symbol(symbol))
    }

    pub fn fetch_spot_candles(
        &self,
        kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        let resolved_symbol = match kind {
            SpotInstrumentKind::Equity => symbol.trim().to_uppercase(),
            SpotInstrumentKind::Index => resolve_index_symbol(symbol),
            SpotInstrumentKind::Commodity => resolve_commodity_symbol(symbol),
        };
        self.fetch_chart_candles(&resolved_symbol, interval.unwrap_or("1d"), start, end)
    }

    pub fn fetch_spot_quote(&self, kind: SpotInstrumentKind, symbol: &str) -> Result<Quote> {
        let resolved_symbol = match kind {
            SpotInstrumentKind::Equity => symbol.trim().to_uppercase(),
            SpotInstrumentKind::Index => resolve_index_symbol(symbol),
            SpotInstrumentKind::Commodity => resolve_commodity_symbol(symbol),
        };
        self.fetch_quote(&resolved_symbol)
    }

    pub fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        let symbol = resolve_options_symbol(symbol);
        let symbol_key = symbol.clone();
        let mut summary = match self.fetch_options_chain_page(&symbol, None) {
            Ok(first_chain) => {
                let underlying_price = first_chain
                    .option_chain
                    .result
                    .first()
                    .and_then(|result| result.quote.as_ref())
                    .and_then(|quote| quote.regular_market_price);
                let expirations = first_chain
                    .option_chain
                    .result
                    .first()
                    .map(|result| result.expiration_dates.clone())
                    .unwrap_or_default();

                let mut contracts = Vec::new();
                collect_contracts(&first_chain, &mut contracts);
                for expiration in expirations.into_iter().skip(1).take(12) {
                    if let Ok(chain) = self.fetch_options_chain_page(&symbol, Some(expiration)) {
                        collect_contracts(&chain, &mut contracts);
                    }
                }

                if contracts.is_empty() {
                    bail!("no options contracts returned for '{}'", symbol);
                }

                let call_open_interest: f64 = contracts
                    .iter()
                    .filter(|contract| contract.contract_type == OptionContractType::Call)
                    .map(|contract| contract.open_interest.unwrap_or(0.0))
                    .sum();
                let put_open_interest: f64 = contracts
                    .iter()
                    .filter(|contract| contract.contract_type == OptionContractType::Put)
                    .map(|contract| contract.open_interest.unwrap_or(0.0))
                    .sum();
                let call_volume: f64 = contracts
                    .iter()
                    .filter(|contract| contract.contract_type == OptionContractType::Call)
                    .map(|contract| contract.volume.unwrap_or(0.0))
                    .sum();
                let put_volume: f64 = contracts
                    .iter()
                    .filter(|contract| contract.contract_type == OptionContractType::Put)
                    .map(|contract| contract.volume.unwrap_or(0.0))
                    .sum();
                let nearest_expiration_dte = contracts
                    .iter()
                    .map(|contract| contract.dte)
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let near_atm_implied_volatility = underlying_price.and_then(|price| {
                    let values = contracts
                        .iter()
                        .filter(|contract| {
                            contract.dte <= 45.0
                                && (contract.strike - price).abs() / price.max(f64::EPSILON) <= 0.10
                        })
                        .filter_map(|contract| contract.implied_volatility)
                        .collect::<Vec<_>>();
                    if values.is_empty() {
                        None
                    } else {
                        Some(values.iter().sum::<f64>() / values.len() as f64)
                    }
                });

                OptionsChainSummary {
                    symbol: symbol_key,
                    source: Some("yfinance:chart_and_options".to_string()),
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
                }
            }
            Err(primary_error) => {
                if let Ok(summary) = browser_bridge::yahoo_finance_options_summary(&symbol) {
                    summary
                } else {
                    return Err(primary_error);
                }
            }
        };

        if let Ok(greeks) = self.fetch_barchart_greeks(&symbol) {
            summary.near_atm_implied_volatility = greeks
                .near_atm_implied_volatility
                .or(summary.near_atm_implied_volatility);
            summary.put_call_oi_ratio = greeks.put_call_oi_ratio.or(summary.put_call_oi_ratio);
            summary.put_call_volume_ratio = greeks
                .put_call_volume_ratio
                .or(summary.put_call_volume_ratio);
            summary.near_atm_delta = greeks.near_atm_delta;
            summary.near_atm_gamma = greeks.near_atm_gamma;
            summary.near_atm_vega = greeks.near_atm_vega;
            summary.call_gamma_oi = greeks.call_gamma_oi;
            summary.put_gamma_oi = greeks.put_gamma_oi;
            summary.gamma_skew = greeks.gamma_skew;
            summary.nearest_expiration_dte = greeks
                .nearest_expiration_dte
                .or(summary.nearest_expiration_dte);
        }

        Ok(summary)
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

    fn fetch_chart_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        let yahoo_interval = map_interval(interval);
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}",
            urlencoding::encode(symbol)
        );
        let response: ChartResponse = {
            const MAX_ATTEMPTS: usize = 3;
            let mut last_error = None;
            let mut parsed = None;
            for attempt in 0..MAX_ATTEMPTS {
                let result = self
                    .client
                    .get(&url)
                    .query(&[
                        ("interval", yahoo_interval.to_string()),
                        ("period1", start.timestamp().to_string()),
                        ("period2", end.timestamp().to_string()),
                        ("includePrePost", "true".to_string()),
                        ("events", "div,splits".to_string()),
                    ])
                    .send()
                    .with_context(|| format!("failed to request yahoo chart for '{}'", symbol))
                    .and_then(|resp| {
                        resp.error_for_status()
                            .with_context(|| format!("yahoo chart returned error for '{}'", symbol))
                    })
                    .and_then(|resp| resp.json().context("failed to parse yahoo chart response"));
                match result {
                    Ok(value) => {
                        parsed = Some(value);
                        break;
                    }
                    Err(err) => {
                        let retryable = is_retryable_yahoo_chart_error(&err);
                        last_error = Some(err);
                        if !retryable || !should_sleep_before_retry(attempt, MAX_ATTEMPTS) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(700));
                    }
                }
            }
            parsed.ok_or_else(|| {
                last_error
                    .unwrap_or_else(|| anyhow!("failed to fetch yahoo chart for '{}'", symbol))
            })?
        };

        let result = response
            .chart
            .result
            .first()
            .ok_or_else(|| anyhow!("missing yahoo chart result for '{}'", symbol))?;
        let quote = result
            .indicators
            .quote
            .first()
            .ok_or_else(|| anyhow!("missing yahoo chart quote arrays for '{}'", symbol))?;

        let mut candles = Vec::new();
        for (index, timestamp) in result.timestamp.iter().enumerate() {
            let (open, high, low, close) = match (
                quote.open.get(index).and_then(|value| *value),
                quote.high.get(index).and_then(|value| *value),
                quote.low.get(index).and_then(|value| *value),
                quote.close.get(index).and_then(|value| *value),
            ) {
                (Some(open), Some(high), Some(low), Some(close)) => (open, high, low, close),
                _ => continue,
            };

            let timestamp = Utc
                .timestamp_opt(*timestamp, 0)
                .single()
                .ok_or_else(|| anyhow!("invalid yahoo timestamp '{}'", timestamp))?;
            candles.push(Candle {
                timestamp,
                open,
                high,
                low,
                close,
                volume: quote
                    .volume
                    .get(index)
                    .and_then(|value| *value)
                    .unwrap_or(0.0),
            });
        }

        if candles.is_empty() {
            bail!("no valid yahoo candles returned for '{}'", symbol);
        }

        Ok(candles)
    }

    fn fetch_quote(&self, symbol: &str) -> Result<Quote> {
        let response = self
            .client
            .get("https://query1.finance.yahoo.com/v7/finance/quote")
            .query(&[("symbols", symbol)])
            .send()
            .with_context(|| format!("failed to request yahoo quote for '{}'", symbol));

        let response: QuoteResponse = match response
            .and_then(|resp| {
                resp.error_for_status()
                    .context("yahoo quote returned error")
            })
            .and_then(|resp| resp.json().context("failed to parse yahoo quote response"))
        {
            Ok(response) => response,
            Err(_) => {
                if let Ok(quote) = self.fetch_quote_from_chart(symbol) {
                    return Ok(quote);
                }
                if let Ok(quote) = self.fetch_quote_from_barchart(symbol) {
                    return Ok(quote);
                }
                return browser_bridge::yahoo_finance_quote(symbol);
            }
        };

        let quote = response
            .quote_response
            .result
            .first()
            .ok_or_else(|| anyhow!("missing yahoo quote result for '{}'", symbol))?;
        let timestamp = quote
            .regular_market_time
            .and_then(|value| Utc.timestamp_opt(value, 0).single())
            .unwrap_or_else(Utc::now);
        let last = quote
            .regular_market_price
            .or(quote.regular_market_previous_close)
            .unwrap_or(0.0);

        Ok(Quote {
            symbol: quote.symbol.clone().unwrap_or_else(|| symbol.to_string()),
            bid: quote.bid.unwrap_or(last),
            ask: quote.ask.unwrap_or(last),
            last,
            timestamp,
        })
    }

    fn fetch_quote_from_chart(&self, symbol: &str) -> Result<Quote> {
        let resolved = if symbol.contains("=F") || symbol.starts_with('^') {
            symbol.to_string()
        } else {
            symbol.to_uppercase()
        };
        let response: ChartResponse = self
            .client
            .get(format!(
                "https://query1.finance.yahoo.com/v8/finance/chart/{}",
                urlencoding::encode(&resolved)
            ))
            .query(&[
                ("interval", "1d"),
                ("range", "1d"),
                ("includePrePost", "true"),
            ])
            .send()
            .with_context(|| format!("failed to request yahoo chart quote for '{}'", resolved))?
            .error_for_status()
            .with_context(|| format!("yahoo chart quote returned error for '{}'", resolved))?
            .json()
            .context("failed to parse yahoo chart quote response")?;

        let chart = response
            .chart
            .result
            .first()
            .ok_or_else(|| anyhow!("missing yahoo chart quote result"))?;
        let meta = &chart.meta;
        let quote = chart.indicators.quote.first();
        let last = meta
            .regular_market_price
            .or_else(|| quote.and_then(|q| q.close.iter().rev().flatten().copied().next()))
            .ok_or_else(|| anyhow!("yahoo chart quote missing price"))?;
        let timestamp = meta
            .regular_market_time
            .and_then(|value| Utc.timestamp_opt(value, 0).single())
            .unwrap_or_else(Utc::now);

        Ok(Quote {
            symbol: meta.symbol.clone().unwrap_or(resolved),
            bid: last,
            ask: last,
            last,
            timestamp,
        })
    }

    fn fetch_quote_from_barchart(&self, symbol: &str) -> Result<Quote> {
        let candidates = [
            format!(
                "https://www.barchart.com/stocks/quotes/{}/overview",
                urlencoding::encode(symbol)
            ),
            format!(
                "https://www.barchart.com/stocks/quotes/{}/options",
                urlencoding::encode(symbol)
            ),
        ];
        let mut last_error: Option<anyhow::Error> = None;

        for page_url in candidates {
            match self
                .client
                .get(&page_url)
                .send()
                .with_context(|| format!("failed to request Barchart page '{}'", page_url))
                .and_then(|resp| {
                    resp.error_for_status()
                        .with_context(|| format!("Barchart page returned error '{}'", page_url))
                })
                .and_then(|resp| resp.text().context("failed to read Barchart page html"))
                .and_then(|html| parse_barchart_current_symbol(&html))
            {
                Ok(value) => {
                    let last = value
                        .get("raw")
                        .and_then(|raw| raw.get("lastPrice"))
                        .and_then(serde_json::Value::as_f64)
                        .or_else(|| {
                            value
                                .get("lastPrice")
                                .and_then(serde_json::Value::as_str)
                                .and_then(|s| s.replace(',', "").parse::<f64>().ok())
                        })
                        .ok_or_else(|| anyhow!("Barchart quote missing lastPrice"))?;

                    return Ok(Quote {
                        symbol: value
                            .get("symbol")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or(symbol)
                            .to_string(),
                        bid: last,
                        ask: last,
                        last,
                        timestamp: Utc::now(),
                    });
                }
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Barchart quote extraction failed")))
    }

    fn fetch_options_chain_page(
        &self,
        symbol: &str,
        expiration: Option<i64>,
    ) -> Result<OptionsChainResponse> {
        let url = format!(
            "https://query2.finance.yahoo.com/v7/finance/options/{}",
            urlencoding::encode(symbol)
        );
        let request = self.client.get(&url);
        let request = if let Some(expiration) = expiration {
            request.query(&[("date", expiration.to_string())])
        } else {
            request
        };

        let response = request
            .send()
            .with_context(|| format!("failed to request yahoo options for '{}'", symbol))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return self.fetch_options_chain_page_with_crumb(symbol, expiration);
        }

        response
            .error_for_status()
            .with_context(|| format!("yahoo options returned error for '{}'", symbol))?
            .json()
            .context("failed to parse yahoo options response")
    }

    fn fetch_options_chain_page_with_crumb(
        &self,
        symbol: &str,
        expiration: Option<i64>,
    ) -> Result<OptionsChainResponse> {
        let (cookie_header, crumb) = self.fetch_yahoo_cookie_and_crumb(symbol)?;
        let url = format!(
            "https://query2.finance.yahoo.com/v7/finance/options/{}",
            urlencoding::encode(symbol)
        );
        let mut request = self
            .client
            .get(url)
            .header("cookie", cookie_header)
            .query(&[("crumb", crumb)]);
        if let Some(expiration) = expiration {
            request = request.query(&[("date", expiration.to_string())]);
        }

        request
            .send()
            .with_context(|| {
                format!(
                    "failed to request yahoo options with crumb for '{}'",
                    symbol
                )
            })?
            .error_for_status()
            .with_context(|| format!("yahoo options with crumb returned error for '{}'", symbol))?
            .json()
            .context("failed to parse yahoo options with crumb response")
    }

    pub fn fetch_options_volatility_proxy_summary(
        &self,
        proxy_symbol: &str,
        underlying_symbol: &str,
    ) -> Result<OptionsChainSummary> {
        let candles = self.fetch_chart_candles(
            proxy_symbol,
            "1d",
            Utc::now() - chrono::Duration::days(45),
            Utc::now(),
        )?;
        let latest = candles
            .last()
            .ok_or_else(|| anyhow!("no volatility proxy candles for '{}'", proxy_symbol))?;

        Ok(OptionsChainSummary {
            symbol: underlying_symbol.to_string(),
            source: Some(format!("yfinance:volatility_proxy:{proxy_symbol}")),
            underlying_price: None,
            call_open_interest: 0.0,
            put_open_interest: 0.0,
            put_call_oi_ratio: None,
            call_volume: 0.0,
            put_volume: 0.0,
            put_call_volume_ratio: None,
            near_atm_implied_volatility: Some((latest.close / 100.0).max(0.0)),
            near_atm_delta: None,
            near_atm_gamma: None,
            near_atm_vega: None,
            call_gamma_oi: None,
            put_gamma_oi: None,
            gamma_skew: None,
            nearest_expiration_dte: None,
        })
    }

    fn fetch_barchart_greeks(&self, symbol: &str) -> Result<BarchartGreeksSummary> {
        let page_url = format!(
            "https://www.barchart.com/stocks/quotes/{}/options",
            urlencoding::encode(symbol)
        );
        let page = self
            .client
            .get(&page_url)
            .send()
            .with_context(|| format!("failed to request Barchart options page for '{}'", symbol))?
            .error_for_status()
            .with_context(|| format!("Barchart options page returned error for '{}'", symbol))?;
        let cookie_header = cookie_header_from_response(&page)?;
        let html = page
            .text()
            .context("failed to read Barchart options page html")?;
        let csrf = parse_barchart_csrf(&html)?;

        let response: BarchartChainResponse = self
            .client
            .get(format!(
                "https://www.barchart.com/proxies/core-api/v1/options/chain?symbol={}&fields=strikePrice,lastPrice,volume,openInterest,volatility,delta,gamma,theta,vega,rho,expirationDate,optionType,percentFromLast&raw=1",
                urlencoding::encode(symbol)
            ))
            .header("X-CSRF-TOKEN", csrf)
            .header("cookie", cookie_header)
            .header("accept", "application/json")
            .send()
            .with_context(|| format!("failed to request Barchart greeks for '{}'", symbol))?
            .error_for_status()
            .with_context(|| format!("Barchart greeks returned error for '{}'", symbol))?
            .json()
            .context("failed to parse Barchart greeks response")?;

        if response.data.is_empty() {
            bail!("Barchart returned no options rows for '{}'", symbol);
        }

        let mut expiries = response
            .data
            .iter()
            .map(|row| row.raw.expiration_date.clone())
            .collect::<Vec<_>>();
        expiries.sort();
        expiries.dedup();
        let nearest_expiry = expiries
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("Barchart options response missing expiry"))?;

        let mut rows = response
            .data
            .into_iter()
            .filter(|row| row.raw.expiration_date == nearest_expiry)
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            a.raw
                .percent_from_last
                .abs()
                .partial_cmp(&b.raw.percent_from_last.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let sample = rows.iter().take(20).collect::<Vec<_>>();
        let mean = |values: Vec<f64>| {
            if values.is_empty() {
                None
            } else {
                Some(values.iter().sum::<f64>() / values.len() as f64)
            }
        };

        let near_atm_implied_volatility = mean(
            sample
                .iter()
                .filter_map(|row| row.raw.volatility)
                .collect::<Vec<_>>(),
        )
        .map(|value| value / 100.0);
        let near_atm_delta = mean(
            sample
                .iter()
                .filter_map(|row| row.raw.delta)
                .collect::<Vec<_>>(),
        );
        let near_atm_gamma = mean(
            sample
                .iter()
                .filter_map(|row| row.raw.gamma)
                .collect::<Vec<_>>(),
        );
        let near_atm_vega = mean(
            sample
                .iter()
                .filter_map(|row| row.raw.vega)
                .collect::<Vec<_>>(),
        );

        let call_gamma_oi: f64 = sample
            .iter()
            .filter(|row| row.raw.option_type.eq_ignore_ascii_case("call"))
            .map(|row| row.raw.gamma.unwrap_or(0.0) * row.raw.open_interest.unwrap_or(0.0))
            .sum();
        let put_gamma_oi: f64 = sample
            .iter()
            .filter(|row| row.raw.option_type.eq_ignore_ascii_case("put"))
            .map(|row| row.raw.gamma.unwrap_or(0.0) * row.raw.open_interest.unwrap_or(0.0))
            .sum();
        let gamma_skew = {
            let denom = call_gamma_oi.abs() + put_gamma_oi.abs();
            if denom <= f64::EPSILON {
                None
            } else {
                Some((call_gamma_oi - put_gamma_oi) / denom)
            }
        };
        let total_call_oi: f64 = rows
            .iter()
            .filter(|row| row.raw.option_type.eq_ignore_ascii_case("call"))
            .map(|row| row.raw.open_interest.unwrap_or(0.0))
            .sum();
        let total_put_oi: f64 = rows
            .iter()
            .filter(|row| row.raw.option_type.eq_ignore_ascii_case("put"))
            .map(|row| row.raw.open_interest.unwrap_or(0.0))
            .sum();
        let total_call_volume: f64 = rows
            .iter()
            .filter(|row| row.raw.option_type.eq_ignore_ascii_case("call"))
            .map(|row| row.raw.volume.unwrap_or(0.0))
            .sum();
        let total_put_volume: f64 = rows
            .iter()
            .filter(|row| row.raw.option_type.eq_ignore_ascii_case("put"))
            .map(|row| row.raw.volume.unwrap_or(0.0))
            .sum();

        Ok(BarchartGreeksSummary {
            put_call_oi_ratio: ratio(total_put_oi, total_call_oi),
            put_call_volume_ratio: ratio(total_put_volume, total_call_volume),
            near_atm_implied_volatility,
            near_atm_delta,
            near_atm_gamma,
            near_atm_vega,
            call_gamma_oi: Some(call_gamma_oi),
            put_gamma_oi: Some(put_gamma_oi),
            gamma_skew,
            nearest_expiration_dte: parse_barchart_expiry_dte(&nearest_expiry),
        })
    }

    fn fetch_yahoo_cookie_and_crumb(&self, symbol: &str) -> Result<(String, String)> {
        let quote_url = format!(
            "https://finance.yahoo.com/quote/{}",
            urlencoding::encode(symbol)
        );
        let quote_response = self
            .client
            .get(&quote_url)
            .header("accept", "text/html,application/xhtml+xml,application/xml")
            .send()
            .with_context(|| format!("failed to request yahoo quote page for '{}'", symbol))?;
        let cookie_header = cookie_header_from_response(&quote_response)?;

        let crumb = self
            .client
            .get("https://query1.finance.yahoo.com/v1/test/getcrumb")
            .header("cookie", &cookie_header)
            .header("origin", "https://finance.yahoo.com")
            .header("referer", &quote_url)
            .header("accept", "*/*")
            .send()
            .context("failed to request yahoo crumb")?
            .error_for_status()
            .context("yahoo crumb endpoint returned error")?
            .text()
            .context("failed to read yahoo crumb response")?;

        if crumb.trim().is_empty() {
            bail!("received empty yahoo crumb");
        }

        Ok((cookie_header, crumb))
    }
}

#[async_trait::async_trait]
impl RealtimeDataProvider for YahooFinanceProvider {
    async fn fetch_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        self.fetch_chart_candles(symbol, timeframe_to_interval(timeframe), start, end)
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
        let response = self
            .client
            .get("https://query1.finance.yahoo.com/v7/finance/quote")
            .query(&[("symbols", "AAPL")])
            .send();
        Ok(matches!(response, Ok(resp) if resp.status().is_success()))
    }
}

#[derive(Debug, Deserialize)]
struct ChartResponse {
    chart: ChartContainer,
}

#[derive(Debug, Deserialize)]
struct ChartContainer {
    result: Vec<ChartResult>,
}

#[derive(Debug, Deserialize)]
struct ChartResult {
    timestamp: Vec<i64>,
    #[serde(default)]
    meta: ChartMeta,
    indicators: ChartIndicators,
}

#[derive(Debug, Deserialize, Default)]
struct ChartMeta {
    #[serde(rename = "symbol")]
    symbol: Option<String>,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketTime")]
    regular_market_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ChartIndicators {
    quote: Vec<ChartQuote>,
}

#[derive(Debug, Deserialize)]
struct ChartQuote {
    #[serde(default)]
    open: Vec<Option<f64>>,
    #[serde(default)]
    high: Vec<Option<f64>>,
    #[serde(default)]
    low: Vec<Option<f64>>,
    #[serde(default)]
    close: Vec<Option<f64>>,
    #[serde(default)]
    volume: Vec<Option<f64>>,
}

#[derive(Debug, Deserialize)]
struct QuoteResponse {
    #[serde(rename = "quoteResponse")]
    quote_response: QuoteResponseBody,
}

#[derive(Debug, Deserialize)]
struct QuoteResponseBody {
    result: Vec<YahooQuote>,
}

#[derive(Debug, Deserialize)]
struct YahooQuote {
    symbol: Option<String>,
    bid: Option<f64>,
    ask: Option<f64>,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketPreviousClose")]
    regular_market_previous_close: Option<f64>,
    #[serde(rename = "regularMarketTime")]
    regular_market_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OptionsChainResponse {
    #[serde(rename = "optionChain")]
    option_chain: OptionsChainContainer,
}

#[derive(Debug, Deserialize)]
struct OptionsChainContainer {
    result: Vec<OptionsResult>,
}

#[derive(Debug, Deserialize)]
struct OptionsResult {
    #[serde(rename = "expirationDates", default)]
    expiration_dates: Vec<i64>,
    quote: Option<OptionsQuote>,
    options: Vec<OptionsSeries>,
}

#[derive(Debug, Deserialize)]
struct OptionsQuote {
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BarchartChainResponse {
    data: Vec<BarchartChainRow>,
}

#[derive(Debug, Deserialize)]
struct BarchartChainRow {
    raw: BarchartChainRaw,
}

#[derive(Debug, Deserialize)]
struct BarchartChainRaw {
    #[serde(rename = "openInterest")]
    open_interest: Option<f64>,
    volume: Option<f64>,
    volatility: Option<f64>,
    delta: Option<f64>,
    gamma: Option<f64>,
    vega: Option<f64>,
    #[serde(rename = "expirationDate")]
    expiration_date: String,
    #[serde(rename = "optionType")]
    option_type: String,
    #[serde(rename = "percentFromLast")]
    percent_from_last: f64,
}

struct BarchartGreeksSummary {
    put_call_oi_ratio: Option<f64>,
    put_call_volume_ratio: Option<f64>,
    near_atm_implied_volatility: Option<f64>,
    near_atm_delta: Option<f64>,
    near_atm_gamma: Option<f64>,
    near_atm_vega: Option<f64>,
    call_gamma_oi: Option<f64>,
    put_gamma_oi: Option<f64>,
    gamma_skew: Option<f64>,
    nearest_expiration_dte: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct OptionsSeries {
    #[serde(default)]
    calls: Vec<YahooOptionContract>,
    #[serde(default)]
    puts: Vec<YahooOptionContract>,
}

#[derive(Debug, Deserialize)]
struct YahooOptionContract {
    strike: f64,
    #[serde(rename = "openInterest")]
    open_interest: Option<f64>,
    volume: Option<f64>,
    #[serde(rename = "impliedVolatility")]
    implied_volatility: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionContractType {
    Call,
    Put,
}

#[derive(Debug)]
struct OptionContractMetric {
    contract_type: OptionContractType,
    strike: f64,
    open_interest: Option<f64>,
    volume: Option<f64>,
    implied_volatility: Option<f64>,
    dte: f64,
}

fn collect_contracts(response: &OptionsChainResponse, output: &mut Vec<OptionContractMetric>) {
    let Some(result) = response.option_chain.result.first() else {
        return;
    };
    let Some(series) = result.options.first() else {
        return;
    };

    for contract in &series.calls {
        output.push(OptionContractMetric {
            contract_type: OptionContractType::Call,
            strike: contract.strike,
            open_interest: contract.open_interest,
            volume: contract.volume,
            implied_volatility: contract.implied_volatility,
            dte: derive_dte(result),
        });
    }

    for contract in &series.puts {
        output.push(OptionContractMetric {
            contract_type: OptionContractType::Put,
            strike: contract.strike,
            open_interest: contract.open_interest,
            volume: contract.volume,
            implied_volatility: contract.implied_volatility,
            dte: derive_dte(result),
        });
    }
}

fn derive_dte(result: &OptionsResult) -> f64 {
    result
        .expiration_dates
        .first()
        .and_then(|epoch| Utc.timestamp_opt(*epoch, 0).single())
        .map(|date| ((date - Utc::now()).num_seconds().max(0) as f64) / 86_400.0)
        .unwrap_or(0.0)
}

fn map_interval(interval: &str) -> &'static str {
    match interval {
        "1m" => "1m",
        "2m" => "2m",
        "5m" => "5m",
        "15m" => "15m",
        "30m" => "30m",
        "60m" | "1h" => "60m",
        "90m" => "90m",
        "1d" => "1d",
        "5d" => "5d",
        "1W" | "1w" => "1wk",
        "1M" | "1mth" => "1mo",
        _ => "1d",
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

fn resolve_futures_symbol(symbol: &str) -> String {
    let upper = symbol.trim().to_uppercase();
    if upper.contains("=F")
        || upper.contains("=X")
        || upper.starts_with('^')
        || upper.contains("-USD")
        || upper.contains("-USDT")
    {
        upper
    } else {
        format!("{upper}=F")
    }
}

fn resolve_index_symbol(symbol: &str) -> String {
    match symbol.trim().to_ascii_lowercase().as_str() {
        "sp500" => "^GSPC".to_string(),
        "spx" => "^SPX".to_string(),
        "ndx" | "nasdaq100" => "^NDX".to_string(),
        "nasdaq" => "^IXIC".to_string(),
        "dji" | "dow" => "^DJI".to_string(),
        raw if raw.starts_with('^') => raw.to_uppercase(),
        _ => symbol.trim().to_uppercase(),
    }
}

fn resolve_commodity_symbol(symbol: &str) -> String {
    match symbol.trim().to_ascii_lowercase().as_str() {
        "gold" => "GC=F".to_string(),
        "silver" => "SI=F".to_string(),
        "copper" => "HG=F".to_string(),
        "crude_oil" | "wti" => "CL=F".to_string(),
        "brent" => "BZ=F".to_string(),
        "natural_gas" => "NG=F".to_string(),
        "corn" => "ZC=F".to_string(),
        "wheat" => "ZW=F".to_string(),
        "soybeans" => "ZS=F".to_string(),
        raw if raw.ends_with("=f") => raw.to_uppercase(),
        raw => raw.to_uppercase(),
    }
}

fn resolve_options_symbol(symbol: &str) -> String {
    match symbol.trim().to_ascii_lowercase().as_str() {
        "spx" => "^SPX".to_string(),
        "ndx" => "^NDX".to_string(),
        other if other.starts_with('^') => other.to_uppercase(),
        _ => symbol.trim().to_uppercase(),
    }
}

fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator.abs() <= f64::EPSILON {
        None
    } else {
        Some(numerator / denominator)
    }
}

fn cookie_header_from_response(response: &Response) -> Result<String> {
    let cookies = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|raw| raw.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if cookies.is_empty() {
        bail!("no yahoo set-cookie headers found");
    }

    Ok(cookies.join("; "))
}

fn parse_barchart_csrf(html: &str) -> Result<String> {
    let marker = r#"<meta name="csrf-token" content=""#;
    let start = html
        .find(marker)
        .ok_or_else(|| anyhow!("Barchart csrf token not found"))?
        + marker.len();
    let end = html[start..]
        .find('"')
        .ok_or_else(|| anyhow!("Barchart csrf token malformed"))?;
    Ok(html[start..start + end].to_string())
}

fn parse_barchart_current_symbol(html: &str) -> Result<serde_json::Value> {
    let marker = "\"currentSymbol\":";
    let start = html
        .find(marker)
        .ok_or_else(|| anyhow!("Barchart currentSymbol block not found"))?
        + marker.len();
    let tail = &html[start..];
    let end = tail
        .find(",\"dynamicAssets\"")
        .ok_or_else(|| anyhow!("Barchart currentSymbol block terminator not found"))?;
    let current_symbol_json = &tail[..end];
    serde_json::from_str(current_symbol_json).context("failed to parse Barchart currentSymbol json")
}

fn parse_barchart_expiry_dte(expiration: &str) -> Option<f64> {
    let date = chrono::NaiveDate::parse_from_str(expiration, "%Y-%m-%d").ok()?;
    let dt = date.and_hms_opt(0, 0, 0)?;
    let expiry = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
    Some(((expiry - Utc::now()).num_seconds().max(0) as f64) / 86_400.0)
}

fn should_retry_yahoo_chart_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_yahoo_chart_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|reqwest_err| {
                reqwest_err.is_timeout()
                    || reqwest_err.is_connect()
                    || reqwest_err
                        .status()
                        .is_some_and(should_retry_yahoo_chart_status)
            })
    })
}

fn should_sleep_before_retry(attempt: usize, max_attempts: usize) -> bool {
    attempt + 1 < max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_futures_symbol_preserves_explicit_yahoo_symbols() {
        assert_eq!(resolve_futures_symbol("BTC-USD"), "BTC-USD");
        assert_eq!(resolve_futures_symbol("EURUSD=X"), "EURUSD=X");
        assert_eq!(resolve_futures_symbol("^GSPC"), "^GSPC");
    }

    #[test]
    fn resolve_futures_symbol_still_normalizes_futures_contracts() {
        assert_eq!(resolve_futures_symbol("NQ"), "NQ=F");
        assert_eq!(resolve_futures_symbol("GC=F"), "GC=F");
    }

    #[test]
    fn yahoo_chart_retry_policy_only_retries_retryable_statuses() {
        assert!(should_retry_yahoo_chart_status(
            reqwest::StatusCode::TOO_MANY_REQUESTS
        ));
        assert!(should_retry_yahoo_chart_status(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(!should_retry_yahoo_chart_status(
            reqwest::StatusCode::BAD_REQUEST
        ));
        assert!(!should_retry_yahoo_chart_status(
            reqwest::StatusCode::NOT_FOUND
        ));
    }

    #[test]
    fn yahoo_chart_retry_only_sleeps_when_another_attempt_remains() {
        assert!(should_sleep_before_retry(0, 3));
        assert!(should_sleep_before_retry(1, 3));
        assert!(!should_sleep_before_retry(2, 3));
    }
}
