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

fn correlation_asset_map(
    futures_symbol: &str,
    spot_symbol: &str,
    options_symbol: &str,
) -> CorrelationAssetMap {
    correlation_asset_map_with_available(futures_symbol, spot_symbol, options_symbol, None)
}

fn correlation_asset_map_with_available(
    futures_symbol: &str,
    spot_symbol: &str,
    options_symbol: &str,
    available_symbols: Option<&[String]>,
) -> CorrelationAssetMap {
    let upper = futures_symbol.to_ascii_uppercase();
    let mut related_futures_symbols = Vec::new();
    let mut related_etf_symbols = vec![spot_symbol.to_string()];
    let mut related_cfd_symbols = Vec::new();
    let mut related_crypto_symbols = Vec::new();

    match upper.as_str() {
        "NQ" | "MNQ" => {
            related_futures_symbols.extend(["ES", "YM", "RTY", "DXY", "VIX"].map(str::to_string));
            related_etf_symbols.extend(["QQQ", "SPY", "DIA", "IWM"].map(str::to_string));
            related_cfd_symbols.extend(["NAS100", "US500", "US30"].map(str::to_string));
        }
        "ES" | "MES" => {
            related_futures_symbols.extend(["NQ", "YM", "RTY", "DXY", "VIX"].map(str::to_string));
            related_etf_symbols.extend(["SPY", "QQQ", "DIA", "IWM"].map(str::to_string));
            related_cfd_symbols.extend(["US500", "NAS100", "US30"].map(str::to_string));
        }
        "YM" | "MYM" => {
            related_futures_symbols.extend(["ES", "NQ", "RTY"].map(str::to_string));
            related_etf_symbols.extend(["DIA", "SPY", "QQQ", "IWM"].map(str::to_string));
            related_cfd_symbols.extend(["US30", "US500", "NAS100"].map(str::to_string));
        }
        "RTY" | "M2K" => {
            related_futures_symbols.extend(["ES", "NQ", "YM"].map(str::to_string));
            related_etf_symbols.extend(["IWM", "SPY", "QQQ", "DIA"].map(str::to_string));
            related_cfd_symbols.extend(["US2000", "US500", "NAS100"].map(str::to_string));
        }
        "XAUUSD" | "GC" | "MGC" => {
            related_futures_symbols
                .extend(["XAGUSD", "SI", "DXY", "US10Y", "REAL_YIELD"].map(str::to_string));
            related_etf_symbols.extend(["GLD", "SLV", "GDX"].map(str::to_string));
        }
        "BTC" | "BTCUSD" | "BTCUSDT" => {
            related_crypto_symbols.extend(["ETH", "SOL", "TOTAL"].map(str::to_string));
            related_futures_symbols.extend(["DXY"].map(str::to_string));
            related_etf_symbols.extend(["IBIT", "ETHE", "QQQ"].map(str::to_string));
        }
        "EURUSD" => {
            related_futures_symbols.extend(["DXY"].map(str::to_string));
            related_etf_symbols.extend(["FXE", "UUP"].map(str::to_string));
            related_cfd_symbols.extend(["GBPUSD", "EURGBP"].map(str::to_string));
        }
        "GBPUSD" => {
            related_futures_symbols.extend(["DXY"].map(str::to_string));
            related_etf_symbols.extend(["FXB", "UUP"].map(str::to_string));
            related_cfd_symbols.extend(["EURUSD", "EURGBP"].map(str::to_string));
        }
        _ => {
            if looks_like_equity_symbol(&upper) {
                related_etf_symbols.extend(
                    equity_market_proxy_etfs(&upper)
                        .into_iter()
                        .map(str::to_string),
                );
                related_futures_symbols.extend(["ES", "NQ", "VIX", "DXY"].map(str::to_string));
            }
        }
    }

    if let Some(available) = available_symbols {
        filter_available_symbols(&mut related_futures_symbols, available);
        filter_available_symbols(&mut related_etf_symbols, available);
        filter_available_symbols(&mut related_cfd_symbols, available);
        filter_available_symbols(&mut related_crypto_symbols, available);
    }

    related_futures_symbols.sort();
    related_futures_symbols.dedup();
    related_etf_symbols.sort();
    related_etf_symbols.dedup();
    related_cfd_symbols.sort();
    related_cfd_symbols.dedup();
    related_crypto_symbols.sort();
    related_crypto_symbols.dedup();

    CorrelationAssetMap {
        related_futures_symbols,
        related_etf_symbols,
        related_options_symbols: vec![options_symbol.to_string()],
        related_cfd_symbols,
        related_crypto_symbols,
    }
}

fn looks_like_equity_symbol(symbol: &str) -> bool {
    !symbol.is_empty()
        && symbol.len() <= 5
        && symbol
            .chars()
            .all(|character| character.is_ascii_uppercase() || character == '.')
}

fn equity_market_proxy_etfs(symbol: &str) -> Vec<&'static str> {
    let sector_etf = match symbol {
        "AAPL" | "MSFT" | "NVDA" | "AMD" | "AVGO" | "META" | "GOOGL" | "GOOG" => "XLK",
        "JPM" | "BAC" | "GS" | "MS" | "WFC" => "XLF",
        "XOM" | "CVX" | "COP" | "SLB" => "XLE",
        "TSLA" | "AMZN" | "HD" | "MCD" | "NKE" => "XLY",
        "LLY" | "UNH" | "JNJ" | "MRK" | "PFE" => "XLV",
        _ => "SPY",
    };
    vec!["SPY", "QQQ", "IWM", sector_etf]
}

fn filter_available_symbols(symbols: &mut Vec<String>, available_symbols: &[String]) {
    let available: std::collections::BTreeSet<String> = available_symbols
        .iter()
        .map(|symbol| symbol.to_ascii_uppercase())
        .collect();
    symbols.retain(|symbol| available.contains(&symbol.to_ascii_uppercase()));
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
    pub smt_signal: Option<String>,
    pub base_swing_type: Option<String>,
    pub base_level: Option<f64>,
    pub comparison_swing_type: Option<String>,
    pub comparison_level: Option<f64>,
    pub raw_comparison_swing_type: Option<String>,
    pub raw_comparison_level: Option<f64>,
    pub swept_side: Option<String>,
    pub normalized_for_inverse_correlation: bool,
    pub relationship_type: String,
    pub relationship_confidence: f64,
    pub trade_use: String,
    pub fail_closed_reason: Option<String>,
    pub notes: Vec<String>,
    pub narrative: String,
}

#[derive(Debug, Clone)]
struct IctSmtSnapshot {
    smt_signal: Option<String>,
    base_swing_type: Option<String>,
    base_level: Option<f64>,
    comparison_swing_type: Option<String>,
    comparison_level: Option<f64>,
    raw_comparison_swing_type: Option<String>,
    raw_comparison_level: Option<f64>,
    swept_side: Option<String>,
    fail_closed_reason: Option<String>,
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
        smt_signal: None,
        base_swing_type: None,
        base_level: None,
        comparison_swing_type: None,
        comparison_level: None,
        raw_comparison_swing_type: None,
        raw_comparison_level: None,
        swept_side: None,
        normalized_for_inverse_correlation: false,
        relationship_type: "unavailable".to_string(),
        relationship_confidence: 0.0,
        trade_use: "confirmation_only".to_string(),
        fail_closed_reason: Some("paired_market_not_provided".to_string()),
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
    let asset_map = correlation_asset_map(futures_symbol, spot_symbol, &auxiliary.options_symbol);
    let (futures_series, spot_series) = aligned_close_series(futures_candles, spot_candles);
    let futures_returns = close_to_returns(&futures_series);
    let spot_returns = close_to_returns(&spot_series);
    let rolling_correlation_20 = Correlation::rolling(&futures_returns, &spot_returns, 20)
        .last()
        .copied();
    let rolling_correlation_50 = Correlation::rolling(&futures_returns, &spot_returns, 50)
        .last()
        .copied();
    let (relationship_type, relationship_confidence, normalized_for_inverse_correlation) =
        classify_relationship(rolling_correlation_20, rolling_correlation_50);
    let divergence_detected = Divergence::detect(&futures_series, &spot_series, 20)
        .last()
        .copied();
    let ict_smt = if relationship_type == "uncertain" {
        empty_ict_smt("relationship_uncertain")
    } else {
        detect_ict_smt(
            futures_candles,
            spot_candles,
            20,
            normalized_for_inverse_correlation,
        )
    };
    let (cointegration_stat, cointegrated) =
        Cointegration::engle_granger(&futures_series, &spot_series);
    let narrative = if let Some(signal) = &ict_smt.smt_signal {
        format!("ict_{signal}_is_confirmation_only_wait_for_pda_and_mss_or_cisd")
    } else if let Some(reason) = &ict_smt.fail_closed_reason {
        format!("ict_smt_fail_closed_{reason}")
    } else if cointegrated && rolling_correlation_20.unwrap_or(0.0) > 0.6 {
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
        smt_signal: ict_smt.smt_signal,
        base_swing_type: ict_smt.base_swing_type,
        base_level: ict_smt.base_level,
        comparison_swing_type: ict_smt.comparison_swing_type,
        comparison_level: ict_smt.comparison_level,
        raw_comparison_swing_type: ict_smt.raw_comparison_swing_type,
        raw_comparison_level: ict_smt.raw_comparison_level,
        swept_side: ict_smt.swept_side,
        normalized_for_inverse_correlation,
        relationship_type,
        relationship_confidence,
        trade_use: "confirmation_only".to_string(),
        fail_closed_reason: ict_smt.fail_closed_reason,
        notes: auxiliary.notes.clone(),
        narrative,
    })
}

fn classify_relationship(corr20: Option<f64>, corr50: Option<f64>) -> (String, f64, bool) {
    let corr = corr20.or(corr50).unwrap_or(0.0);
    let confidence = corr.abs().min(1.0);
    if corr >= 0.3 {
        ("positive".to_string(), confidence, false)
    } else if corr <= -0.3 {
        ("negative".to_string(), confidence, true)
    } else {
        ("uncertain".to_string(), confidence, false)
    }
}

fn detect_ict_smt(
    base_candles: &[Candle],
    comparison_candles: &[Candle],
    lookback: usize,
    normalize_comparison_for_inverse: bool,
) -> IctSmtSnapshot {
    let len = base_candles.len().min(comparison_candles.len());
    if len < 3 {
        return IctSmtSnapshot {
            smt_signal: None,
            base_swing_type: None,
            base_level: None,
            comparison_swing_type: None,
            comparison_level: None,
            raw_comparison_swing_type: None,
            raw_comparison_level: None,
            swept_side: None,
            fail_closed_reason: Some("insufficient_paired_candles".to_string()),
        };
    }

    let start = len.saturating_sub(lookback + 1);
    let base_window = &base_candles[start..len - 1];
    let comparison_window = &comparison_candles[start..len - 1];
    let Some(base_last) = base_candles.get(len - 1) else {
        return empty_ict_smt("insufficient_paired_candles");
    };
    let Some(comparison_last) = comparison_candles.get(len - 1) else {
        return empty_ict_smt("insufficient_paired_candles");
    };
    if base_window.is_empty() || comparison_window.is_empty() {
        return empty_ict_smt("insufficient_paired_candles");
    }

    let base_prev_high = base_window
        .iter()
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let base_prev_low = base_window
        .iter()
        .map(|candle| candle.low)
        .fold(f64::INFINITY, f64::min);
    let comparison_prev_high = comparison_window
        .iter()
        .map(|candle| normalized_high(candle, normalize_comparison_for_inverse))
        .fold(f64::NEG_INFINITY, f64::max);
    let comparison_prev_low = comparison_window
        .iter()
        .map(|candle| normalized_low(candle, normalize_comparison_for_inverse))
        .fold(f64::INFINITY, f64::min);

    let base_hh = base_last.high > base_prev_high;
    let base_ll = base_last.low < base_prev_low;
    let comparison_hh =
        normalized_high(comparison_last, normalize_comparison_for_inverse) > comparison_prev_high;
    let comparison_ll =
        normalized_low(comparison_last, normalize_comparison_for_inverse) < comparison_prev_low;

    if base_hh && !comparison_hh {
        IctSmtSnapshot {
            smt_signal: Some("bearish_smt".to_string()),
            base_swing_type: Some("HH_sweep".to_string()),
            base_level: Some(base_last.high),
            comparison_swing_type: Some("failed_HH".to_string()),
            comparison_level: Some(comparison_failure_high_level(
                comparison_last,
                normalize_comparison_for_inverse,
            )),
            raw_comparison_swing_type: Some(
                raw_swing_label("failed_HH", normalize_comparison_for_inverse).to_string(),
            ),
            raw_comparison_level: Some(comparison_failure_high_level(
                comparison_last,
                normalize_comparison_for_inverse,
            )),
            swept_side: Some("buy_side_liquidity".to_string()),
            fail_closed_reason: None,
        }
    } else if comparison_hh && !base_hh {
        IctSmtSnapshot {
            smt_signal: Some("bearish_smt".to_string()),
            base_swing_type: Some("failed_HH".to_string()),
            base_level: Some(base_last.high),
            comparison_swing_type: Some("HH_sweep".to_string()),
            comparison_level: Some(comparison_sweep_high_level(
                comparison_last,
                normalize_comparison_for_inverse,
            )),
            raw_comparison_swing_type: Some(
                raw_swing_label("HH_sweep", normalize_comparison_for_inverse).to_string(),
            ),
            raw_comparison_level: Some(comparison_sweep_high_level(
                comparison_last,
                normalize_comparison_for_inverse,
            )),
            swept_side: Some("buy_side_liquidity".to_string()),
            fail_closed_reason: None,
        }
    } else if base_ll && !comparison_ll {
        IctSmtSnapshot {
            smt_signal: Some("bullish_smt".to_string()),
            base_swing_type: Some("LL_sweep".to_string()),
            base_level: Some(base_last.low),
            comparison_swing_type: Some("failed_LL".to_string()),
            comparison_level: Some(comparison_failure_low_level(
                comparison_last,
                normalize_comparison_for_inverse,
            )),
            raw_comparison_swing_type: Some(
                raw_swing_label("failed_LL", normalize_comparison_for_inverse).to_string(),
            ),
            raw_comparison_level: Some(comparison_failure_low_level(
                comparison_last,
                normalize_comparison_for_inverse,
            )),
            swept_side: Some("sell_side_liquidity".to_string()),
            fail_closed_reason: None,
        }
    } else if comparison_ll && !base_ll {
        IctSmtSnapshot {
            smt_signal: Some("bullish_smt".to_string()),
            base_swing_type: Some("failed_LL".to_string()),
            base_level: Some(base_last.low),
            comparison_swing_type: Some("LL_sweep".to_string()),
            comparison_level: Some(comparison_sweep_low_level(
                comparison_last,
                normalize_comparison_for_inverse,
            )),
            raw_comparison_swing_type: Some(
                raw_swing_label("LL_sweep", normalize_comparison_for_inverse).to_string(),
            ),
            raw_comparison_level: Some(comparison_sweep_low_level(
                comparison_last,
                normalize_comparison_for_inverse,
            )),
            swept_side: Some("sell_side_liquidity".to_string()),
            fail_closed_reason: None,
        }
    } else {
        IctSmtSnapshot {
            smt_signal: None,
            base_swing_type: None,
            base_level: None,
            comparison_swing_type: None,
            comparison_level: None,
            raw_comparison_swing_type: None,
            raw_comparison_level: None,
            swept_side: None,
            fail_closed_reason: Some("no_swing_confirmation_failure".to_string()),
        }
    }
}

fn empty_ict_smt(reason: &str) -> IctSmtSnapshot {
    IctSmtSnapshot {
        smt_signal: None,
        base_swing_type: None,
        base_level: None,
        comparison_swing_type: None,
        comparison_level: None,
        raw_comparison_swing_type: None,
        raw_comparison_level: None,
        swept_side: None,
        fail_closed_reason: Some(reason.to_string()),
    }
}

fn normalized_high(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        -candle.low
    } else {
        candle.high
    }
}

fn normalized_low(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        -candle.high
    } else {
        candle.low
    }
}

fn raw_swing_label(normalized_label: &str, inverse: bool) -> &'static str {
    match (normalized_label, inverse) {
        ("HH_sweep", true) => "LL_sweep",
        ("failed_HH", true) => "failed_LL",
        ("LL_sweep", true) => "HH_sweep",
        ("failed_LL", true) => "failed_HH",
        ("HH_sweep", false) => "HH_sweep",
        ("failed_HH", false) => "failed_HH",
        ("LL_sweep", false) => "LL_sweep",
        ("failed_LL", false) => "failed_LL",
        _ => "unknown",
    }
}

fn comparison_sweep_high_level(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        candle.low
    } else {
        candle.high
    }
}

fn comparison_failure_high_level(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        candle.low
    } else {
        candle.high
    }
}

fn comparison_sweep_low_level(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        candle.high
    } else {
        candle.low
    }
}

fn comparison_failure_low_level(candle: &Candle, inverse: bool) -> f64 {
    if inverse {
        candle.high
    } else {
        candle.low
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::realtime::market_support::SpotInstrumentKind;
    use chrono::{Duration, TimeZone, Utc};

    fn candle(index: i64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 12, 13, 30, 0).unwrap()
                + Duration::minutes(index),
            open: close,
            high,
            low,
            close,
            volume: 1000.0,
        }
    }

    fn auxiliary() -> AuxiliaryMarketEvidence {
        AuxiliaryMarketEvidence {
            spot_symbol: "ES".to_string(),
            options_symbol: "SPY".to_string(),
            spot_kind: SpotInstrumentKind::Index,
            spot_last_close: None,
            futures_last_close: None,
            spot_return: None,
            futures_return: None,
            raw_basis_bps: None,
            normalized_basis_bps: None,
            rolling_price_ratio_mean: None,
            put_call_oi_ratio: None,
            put_call_volume_ratio: None,
            near_atm_implied_volatility: None,
            near_atm_delta: None,
            near_atm_gamma: None,
            near_atm_vega: None,
            call_gamma_oi: None,
            put_gamma_oi: None,
            gamma_skew: None,
            hedge_pressure_direction: None,
            hedge_pressure_score: None,
            long_bias: 0.0,
            short_bias: 0.0,
            uncertainty_penalty: 0.0,
            notes: Vec::new(),
        }
    }

    #[test]
    fn ict_smt_bearish_requires_higher_high_sweep_without_pair_confirmation() {
        let mut base = Vec::new();
        let mut pair = Vec::new();
        for index in 0..24 {
            base.push(candle(index, 100.0 + index as f64 * 0.1, 95.0, 99.0));
            pair.push(candle(index, 200.0 + index as f64 * 0.1, 195.0, 199.0));
        }
        base.push(candle(24, 106.75, 96.0, 105.5));
        pair.push(candle(24, 201.20, 196.0, 200.4));

        let section = build_smt_correlation_section("NQ", "ES", &base, &pair, &auxiliary())
            .expect("smt section");

        assert_eq!(section.smt_signal.as_deref(), Some("bearish_smt"));
        assert_eq!(section.base_swing_type.as_deref(), Some("HH_sweep"));
        assert_eq!(section.base_level, Some(106.75));
        assert_eq!(section.comparison_swing_type.as_deref(), Some("failed_HH"));
        assert_eq!(section.comparison_level, Some(201.2));
        assert_eq!(
            section.raw_comparison_swing_type.as_deref(),
            Some("failed_HH")
        );
        assert_eq!(section.raw_comparison_level, Some(201.2));
        assert_eq!(section.swept_side.as_deref(), Some("buy_side_liquidity"));
        assert_eq!(section.trade_use, "confirmation_only");
    }

    #[test]
    fn ict_smt_bullish_requires_lower_low_sweep_without_pair_confirmation() {
        let mut base = Vec::new();
        let mut pair = Vec::new();
        for index in 0..24 {
            base.push(candle(index, 105.0, 100.0 - index as f64 * 0.1, 101.0));
            pair.push(candle(index, 205.0, 200.0 - index as f64 * 0.1, 201.0));
        }
        base.push(candle(24, 103.0, 96.25, 97.0));
        pair.push(candle(24, 203.0, 197.85, 199.0));

        let section = build_smt_correlation_section("NQ", "ES", &base, &pair, &auxiliary())
            .expect("smt section");

        assert_eq!(section.smt_signal.as_deref(), Some("bullish_smt"));
        assert_eq!(section.base_swing_type.as_deref(), Some("LL_sweep"));
        assert_eq!(section.base_level, Some(96.25));
        assert_eq!(section.comparison_swing_type.as_deref(), Some("failed_LL"));
        assert_eq!(section.comparison_level, Some(197.85));
        assert_eq!(
            section.raw_comparison_swing_type.as_deref(),
            Some("failed_LL")
        );
        assert_eq!(section.raw_comparison_level, Some(197.85));
        assert_eq!(section.swept_side.as_deref(), Some("sell_side_liquidity"));
        assert_eq!(section.trade_use, "confirmation_only");
    }

    #[test]
    fn ict_smt_inverse_relationship_normalizes_comparison_structure() {
        let mut base = Vec::new();
        let mut inverse_pair = Vec::new();
        for index in 0..24 {
            base.push(candle(index, 100.0 + index as f64 * 0.1, 95.0, 99.0));
            inverse_pair.push(candle(index, 205.0, 200.0 - index as f64 * 0.1, 201.0));
        }
        base.push(candle(24, 106.75, 96.0, 105.5));
        inverse_pair.push(candle(24, 203.0, 197.85, 199.0));

        let snapshot = detect_ict_smt(&base, &inverse_pair, 20, true);

        assert_eq!(snapshot.smt_signal.as_deref(), Some("bearish_smt"));
        assert_eq!(snapshot.base_swing_type.as_deref(), Some("HH_sweep"));
        assert_eq!(snapshot.base_level, Some(106.75));
        assert_eq!(snapshot.comparison_swing_type.as_deref(), Some("failed_HH"));
        assert_eq!(snapshot.comparison_level, Some(197.85));
        assert_eq!(
            snapshot.raw_comparison_swing_type.as_deref(),
            Some("failed_LL")
        );
        assert_eq!(snapshot.raw_comparison_level, Some(197.85));
        assert_eq!(snapshot.swept_side.as_deref(), Some("buy_side_liquidity"));
    }

    #[test]
    fn smt_relationship_resolver_keeps_crypto_macro_driver() {
        let map = correlation_asset_map("BTC", "BTC", "IBIT");

        assert!(map.related_crypto_symbols.contains(&"ETH".to_string()));
        assert!(map.related_crypto_symbols.contains(&"SOL".to_string()));
        assert!(map.related_futures_symbols.contains(&"DXY".to_string()));
        assert!(map.related_etf_symbols.contains(&"QQQ".to_string()));
    }

    #[test]
    fn smt_relationship_resolver_filters_by_provider_universe() {
        let available = vec![
            "ETH".to_string(),
            "DXY".to_string(),
            "QQQ".to_string(),
            "SPY".to_string(),
        ];
        let map = correlation_asset_map_with_available("BTC", "BTC", "IBIT", Some(&available));

        assert_eq!(map.related_crypto_symbols, vec!["ETH".to_string()]);
        assert_eq!(map.related_futures_symbols, vec!["DXY".to_string()]);
        assert_eq!(map.related_etf_symbols, vec!["QQQ".to_string()]);
    }

    #[test]
    fn smt_relationship_resolver_adds_equity_index_sector_and_options_proxies() {
        let available = vec![
            "SPY".to_string(),
            "QQQ".to_string(),
            "XLK".to_string(),
            "DXY".to_string(),
            "VIX".to_string(),
        ];
        let map = correlation_asset_map_with_available("AAPL", "AAPL", "AAPL", Some(&available));

        assert_eq!(map.related_etf_symbols, vec!["QQQ", "SPY", "XLK"]);
        assert_eq!(map.related_futures_symbols, vec!["DXY", "VIX"]);
        assert_eq!(map.related_options_symbols, vec!["AAPL".to_string()]);
    }
}
