use serde::Serialize;

use crate::data::realtime::openalice::AuxiliaryMarketEvidence;

#[derive(Debug, Serialize)]
pub struct OptionsHedgingSection {
    pub probability_role: String,
    pub options_symbol: Option<String>,
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
    pub long_bias_contribution: Option<f64>,
    pub short_bias_contribution: Option<f64>,
    pub uncertainty_penalty_contribution: Option<f64>,
    pub narrative: String,
}

pub fn build_options_hedging_section(
    auxiliary: Option<&AuxiliaryMarketEvidence>,
) -> OptionsHedgingSection {
    let narrative = if let Some(aux) = auxiliary {
        let mut parts = Vec::new();

        if let Some(iv) = aux.near_atm_implied_volatility {
            if iv >= 0.35 {
                parts.push("high_iv_can_force_more_aggressive_hedging");
            } else if iv <= 0.18 {
                parts.push("contained_iv_limits_hedging_urgency");
            } else {
                parts.push("moderate_iv_environment");
            }
        }

        if let Some(gamma) = aux.near_atm_gamma {
            if gamma >= 0.05 {
                parts.push("elevated_gamma_makes_delta_hedging_more_sensitive");
            } else if gamma <= 0.02 {
                parts.push("subdued_gamma_reduces_convexity_pressure");
            }
        }

        if let Some(vega) = aux.near_atm_vega {
            if vega >= 0.20 {
                parts.push("vega_exposure_means_volatility_shifts_matter");
            }
        }

        match aux.hedge_pressure_direction.as_deref() {
            Some("bullish") => parts.push("dealer_hedging_bias_supports_upside"),
            Some("bearish") => parts.push("dealer_hedging_bias_supports_downside"),
            Some("neutral") | None => {}
            _ => {}
        }

        if aux
            .notes
            .iter()
            .any(|note| note == "options_volatility_proxy_only")
        {
            parts.push("options_signal_uses_proxy_not_full_chain");
        }

        if parts.is_empty() {
            "options_data_available_but_hedging_bias_is_neutral".to_string()
        } else {
            parts.join(";")
        }
    } else {
        "options_hedging_data_unavailable_or_proxied".to_string()
    };

    OptionsHedgingSection {
        probability_role: "options_hedging_is_auxiliary_evidence_not_trade_trigger".to_string(),
        options_symbol: auxiliary.map(|aux| aux.options_symbol.clone()),
        put_call_oi_ratio: auxiliary.and_then(|aux| aux.put_call_oi_ratio),
        put_call_volume_ratio: auxiliary.and_then(|aux| aux.put_call_volume_ratio),
        near_atm_implied_volatility: auxiliary.and_then(|aux| aux.near_atm_implied_volatility),
        near_atm_delta: auxiliary.and_then(|aux| aux.near_atm_delta),
        near_atm_gamma: auxiliary.and_then(|aux| aux.near_atm_gamma),
        near_atm_vega: auxiliary.and_then(|aux| aux.near_atm_vega),
        call_gamma_oi: auxiliary.and_then(|aux| aux.call_gamma_oi),
        put_gamma_oi: auxiliary.and_then(|aux| aux.put_gamma_oi),
        gamma_skew: auxiliary.and_then(|aux| aux.gamma_skew),
        hedge_pressure_direction: auxiliary.and_then(|aux| aux.hedge_pressure_direction.clone()),
        hedge_pressure_score: auxiliary.and_then(|aux| aux.hedge_pressure_score),
        long_bias_contribution: auxiliary.map(|aux| aux.long_bias),
        short_bias_contribution: auxiliary.map(|aux| aux.short_bias),
        uncertainty_penalty_contribution: auxiliary.map(|aux| aux.uncertainty_penalty),
        narrative,
    }
}
