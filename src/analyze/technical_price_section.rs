use serde::Serialize;

use super::options_hedging_section::{build_options_hedging_section, OptionsHedgingSection};
use crate::data::realtime::market_support::AuxiliaryMarketEvidence;
use crate::types::Candle;

#[derive(Debug, Serialize)]
pub struct TechnicalPriceSection {
    pub probability_role: String,
    pub last_closed_bar_close: f64,
    pub live_market_price: Option<f64>,
    pub live_spot_price: Option<f64>,
    pub ema20: Option<f64>,
    pub ema50: Option<f64>,
    pub rsi14: Option<f64>,
    pub adx14: Option<f64>,
    pub atr14: Option<f64>,
    pub macd_line: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_histogram: Option<f64>,
    pub bollinger_upper: Option<f64>,
    pub bollinger_middle: Option<f64>,
    pub bollinger_lower: Option<f64>,
    pub bollinger_squeeze: bool,
    pub momentum_5_bar: Option<f64>,
    pub options_hedging: OptionsHedgingSection,
    pub narrative: String,
}

pub fn build_technical_price_section(
    ltf: &[Candle],
    live_market_price: Option<f64>,
    live_spot_price: Option<f64>,
    auxiliary: Option<&AuxiliaryMarketEvidence>,
) -> TechnicalPriceSection {
    use crate::indicators::{
        is_squeeze, latest_adx, latest_atr, latest_bollinger, latest_ema, latest_macd, latest_rsi,
    };

    let last_close = ltf.last().map(|candle| candle.close).unwrap_or(0.0);
    let atr14 = Some(latest_atr(ltf, 14)).filter(|value| *value > 0.0);
    let rsi14 = Some(latest_rsi(ltf, 14));
    let adx14 = Some(latest_adx(ltf, 14));
    let ema20 = Some(latest_ema(ltf, 20)).filter(|value| *value > 0.0);
    let ema50 = Some(latest_ema(ltf, 50)).filter(|value| *value > 0.0);
    let macd = latest_macd(ltf, 12, 26, 9);
    let bollinger = latest_bollinger(ltf, 20, 2.0);
    let momentum_5_bar = if ltf.len() > 5 {
        let previous = ltf[ltf.len() - 6].close;
        if previous.abs() > f64::EPSILON {
            Some((last_close - previous) / previous)
        } else {
            None
        }
    } else {
        None
    };
    let narrative = match (rsi14, adx14, macd) {
        (Some(rsi), Some(adx), Some((_, _, histogram)))
            if rsi > 55.0 && adx > 20.0 && histogram > 0.0 =>
        {
            "technicals_support_bullish_continuation".to_string()
        }
        (Some(rsi), Some(adx), Some((_, _, histogram)))
            if rsi < 45.0 && adx > 20.0 && histogram < 0.0 =>
        {
            "technicals_support_bearish_continuation".to_string()
        }
        _ => "technicals_mixed_or_range_bound".to_string(),
    };

    TechnicalPriceSection {
        probability_role: "technical_and_derivatives_evidence_for_probability_model".to_string(),
        last_closed_bar_close: last_close,
        live_market_price,
        live_spot_price,
        ema20,
        ema50,
        rsi14,
        adx14,
        atr14,
        macd_line: macd.map(|value| value.0),
        macd_signal: macd.map(|value| value.1),
        macd_histogram: macd.map(|value| value.2),
        bollinger_upper: bollinger.map(|value| value.0),
        bollinger_middle: bollinger.map(|value| value.1),
        bollinger_lower: bollinger.map(|value| value.2),
        bollinger_squeeze: is_squeeze(ltf, 20, 2.0, 0.05),
        momentum_5_bar,
        options_hedging: build_options_hedging_section(auxiliary),
        narrative,
    }
}
