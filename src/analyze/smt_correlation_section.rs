use anyhow::Result;
use serde::Serialize;

use super::series::{aligned_close_series, close_to_returns};
use crate::data::realtime::market_support::AuxiliaryMarketEvidence;
use crate::smt::{Cointegration, Correlation, Divergence};
use crate::types::Candle;

#[derive(Debug, Clone)]
struct CorrelationAssetMap {
    related_futures_symbols: Vec<String>,
    related_etf_symbols: Vec<String>,
    related_options_symbols: Vec<String>,
    related_cfd_symbols: Vec<String>,
    related_crypto_symbols: Vec<String>,
}

fn correlation_asset_map(spot_symbol: &str, options_symbol: &str) -> CorrelationAssetMap {
    CorrelationAssetMap {
        related_futures_symbols: Vec::new(),
        related_etf_symbols: vec![spot_symbol.to_string()],
        related_options_symbols: vec![options_symbol.to_string()],
        related_cfd_symbols: Vec::new(),
        related_crypto_symbols: Vec::new(),
    }
}

#[derive(Debug, Serialize)]
pub struct SmtCorrelationSection {
    pub probability_role: String,
    pub paired_market_available: bool,
    pub futures_symbol: Option<String>,
    pub spot_symbol: Option<String>,
    pub related_futures_symbols: Vec<String>,
    pub related_etf_symbols: Vec<String>,
    pub related_options_symbols: Vec<String>,
    pub related_cfd_symbols: Vec<String>,
    pub related_crypto_symbols: Vec<String>,
    pub rolling_correlation_20: Option<f64>,
    pub rolling_correlation_50: Option<f64>,
    pub divergence_detected: Option<bool>,
    pub cointegration_stat: Option<f64>,
    pub cointegrated: Option<bool>,
    pub raw_basis_bps: Option<f64>,
    pub normalized_basis_bps: Option<f64>,
    pub rolling_price_ratio_mean: Option<f64>,
    pub notes: Vec<String>,
    pub narrative: String,
}

pub fn empty_smt_correlation_section() -> SmtCorrelationSection {
    SmtCorrelationSection {
        probability_role: "cross_market_confirmation_for_probability_model".to_string(),
        paired_market_available: false,
        futures_symbol: None,
        spot_symbol: None,
        related_futures_symbols: Vec::new(),
        related_etf_symbols: Vec::new(),
        related_options_symbols: Vec::new(),
        related_cfd_symbols: Vec::new(),
        related_crypto_symbols: Vec::new(),
        rolling_correlation_20: None,
        rolling_correlation_50: None,
        divergence_detected: None,
        cointegration_stat: None,
        cointegrated: None,
        raw_basis_bps: None,
        normalized_basis_bps: None,
        rolling_price_ratio_mean: None,
        notes: vec!["paired_market_not_provided".to_string()],
        narrative: "smt_analysis_unavailable_without_paired_market".to_string(),
    }
}

pub fn build_smt_correlation_section(
    futures_symbol: &str,
    spot_symbol: &str,
    futures_candles: &[Candle],
    spot_candles: &[Candle],
    auxiliary: &AuxiliaryMarketEvidence,
) -> Result<SmtCorrelationSection> {
    let asset_map = correlation_asset_map(spot_symbol, &auxiliary.options_symbol);
    let (futures_series, spot_series) = aligned_close_series(futures_candles, spot_candles);
    let futures_returns = close_to_returns(&futures_series);
    let spot_returns = close_to_returns(&spot_series);
    let rolling_correlation_20 = Correlation::rolling(&futures_returns, &spot_returns, 20)
        .last()
        .copied();
    let rolling_correlation_50 = Correlation::rolling(&futures_returns, &spot_returns, 50)
        .last()
        .copied();
    let divergence_detected = Divergence::detect(&futures_series, &spot_series, 20)
        .last()
        .copied();
    let (cointegration_stat, cointegrated) =
        Cointegration::engle_granger(&futures_series, &spot_series);
    let narrative = if cointegrated && rolling_correlation_20.unwrap_or(0.0) > 0.6 {
        "paired_markets_are_aligned_and_statistically_supportive".to_string()
    } else if divergence_detected.unwrap_or(false) {
        "paired_markets_show_divergence_so_smt_confidence_is_reduced".to_string()
    } else {
        "paired_markets_offer_mixed_confirmation".to_string()
    };

    Ok(SmtCorrelationSection {
        probability_role: "cross_market_confirmation_for_probability_model".to_string(),
        paired_market_available: true,
        futures_symbol: Some(futures_symbol.to_string()),
        spot_symbol: Some(spot_symbol.to_string()),
        related_futures_symbols: asset_map.related_futures_symbols,
        related_etf_symbols: asset_map.related_etf_symbols,
        related_options_symbols: asset_map.related_options_symbols,
        related_cfd_symbols: asset_map.related_cfd_symbols,
        related_crypto_symbols: asset_map.related_crypto_symbols,
        rolling_correlation_20,
        rolling_correlation_50,
        divergence_detected,
        cointegration_stat: Some(cointegration_stat),
        cointegrated: Some(cointegrated),
        raw_basis_bps: auxiliary.raw_basis_bps,
        normalized_basis_bps: auxiliary.normalized_basis_bps,
        rolling_price_ratio_mean: auxiliary.rolling_price_ratio_mean,
        notes: auxiliary.notes.clone(),
        narrative,
    })
}
