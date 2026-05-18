use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::Candle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
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

    pub fn historical_path(self) -> &'static str {
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
    pub source: Option<String>,
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

pub fn build_auxiliary_evidence(
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
    if let Some(source) = &options_summary.source {
        notes.push(format!("options_data_source={source}"));
    }

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

#[derive(Debug, Clone, Copy)]
struct RatioStats {
    rolling_mean: f64,
    normalized_basis_bps: Option<f64>,
}

fn trailing_return(candles: &[Candle], lookback: usize) -> Option<f64> {
    if candles.len() <= lookback {
        return None;
    }
    let start = candles[candles.len() - lookback - 1].close;
    let end = candles.last()?.close;
    if start.abs() <= f64::EPSILON {
        return None;
    }
    Some((end - start) / start)
}

fn rolling_price_ratio_stats(
    futures_candles: &[Candle],
    spot_candles: &[Candle],
    lookback: usize,
) -> Option<RatioStats> {
    let pairs = futures_candles
        .iter()
        .zip(spot_candles.iter())
        .filter_map(|(future, spot)| {
            if spot.close.abs() <= f64::EPSILON {
                None
            } else {
                Some(future.close / spot.close)
            }
        })
        .collect::<Vec<_>>();
    if pairs.len() < 4 {
        return None;
    }
    let start = pairs.len().saturating_sub(lookback);
    let window = &pairs[start..];
    let mean = window.iter().sum::<f64>() / window.len() as f64;
    let variance = window
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / window.len() as f64;
    let std = variance.sqrt();
    let latest = *window.last()?;
    let normalized_basis_bps = if mean.abs() <= f64::EPSILON || std <= f64::EPSILON {
        None
    } else {
        Some(((latest - mean) / mean) * 10_000.0)
    };

    Some(RatioStats {
        rolling_mean: mean,
        normalized_basis_bps,
    })
}

fn normalize(distribution: &mut [f64]) {
    let sum = distribution.iter().sum::<f64>();
    if sum.abs() <= f64::EPSILON {
        let uniform = 1.0 / distribution.len() as f64;
        distribution.fill(uniform);
        return;
    }
    for value in distribution.iter_mut() {
        *value = (*value / sum).clamp(0.0, 1.0);
    }
    let normalized_sum = distribution.iter().sum::<f64>();
    if normalized_sum.abs() > f64::EPSILON {
        for value in distribution.iter_mut() {
            *value /= normalized_sum;
        }
    }
}
