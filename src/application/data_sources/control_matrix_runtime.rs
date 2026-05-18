use anyhow::Result;

use crate::application::backtest::Pb12RunSpec;
use crate::application::data_sources::control_matrix_providers::tradingview_mcp_config_from_env_or_local;
use crate::application::data_sources::{
    build_market_data_harness_plan, execute_market_data_harness_plan, MarketDataHarnessRequest,
};
use crate::data::load_candles;
use crate::data::realtime::external_http_runtime::ExternalHttpRuntimeProvider;
use crate::data::realtime::market_support::{
    AuxiliaryMarketEvidence, OptionsChainSummary, SpotInstrumentKind,
};
use crate::types::Candle;

const CONTROL_MATRIX_REFERENCE_PROVIDER_ENV: &str = "ICT_ENGINE_CONTROL_MATRIX_REFERENCE_PROVIDER";
const CONTROL_MATRIX_OPTIONS_PROVIDER_ENV: &str = "ICT_ENGINE_CONTROL_MATRIX_OPTIONS_PROVIDER";
const CONTROL_MATRIX_REQUEST_JSON_ENV: &str = "ICT_ENGINE_CONTROL_MATRIX_REQUEST_JSON";

#[derive(Debug, Clone, Default)]
pub struct ControlMatrixRuntimeOverrides {
    pub paired_candles: Option<Vec<Candle>>,
    pub auxiliary: Option<AuxiliaryMarketEvidence>,
    pub runtime_notes: Vec<String>,
}

pub fn build_control_matrix_runtime_overrides(
    data_path: &str,
    symbol: &str,
    run_spec: &Pb12RunSpec,
) -> Result<ControlMatrixRuntimeOverrides> {
    if !(run_spec.use_etf
        || run_spec.use_cfd
        || run_spec.use_vix
        || run_spec.use_greeks
        || run_spec.use_oi
        || run_spec.use_iv)
    {
        return Ok(ControlMatrixRuntimeOverrides::default());
    }

    let primary_candles = load_candles(data_path)?;
    if primary_candles.len() < 2 {
        return Ok(ControlMatrixRuntimeOverrides {
            runtime_notes: vec!["control_matrix_runtime_insufficient_primary_history".to_string()],
            ..ControlMatrixRuntimeOverrides::default()
        });
    }
    let request = build_harness_request(data_path, symbol, run_spec, &primary_candles)?;
    let plan = build_market_data_harness_plan(request)?;
    let bundle = execute_market_data_harness_plan(&plan)?;
    let mut runtime_notes = plan
        .warnings
        .iter()
        .cloned()
        .chain(
            plan.missing_roles
                .iter()
                .map(|role| format!("missing_role={role}")),
        )
        .chain(
            plan.provider_summary
                .actionable_install_prompts
                .iter()
                .map(|item| format!("provider_prompt={item}")),
        )
        .collect::<Vec<_>>();

    let mut role_candles = std::collections::BTreeMap::<String, Vec<Candle>>::new();
    let mut role_options = std::collections::BTreeMap::<String, OptionsChainSummary>::new();
    for result in &bundle.results {
        if result.ok {
            runtime_notes.push(format!(
                "harness_result_ok role={} provider={} operation={}",
                result.role, result.provider, result.operation
            ));
        } else if let Some(error) = result.error.as_ref() {
            runtime_notes.push(format!(
                "harness_result_error role={} provider={} category={} retryable={} message={}",
                result.role, result.provider, error.category, error.retryable, error.message
            ));
        }
        if let Some(data) = result.data.as_ref() {
            match result.operation.as_str() {
                "ohlcv.fetch" => {
                    if let Ok(candles) = serde_json::from_value::<Vec<Candle>>(data.clone()) {
                        role_candles.insert(result.role.clone(), candles);
                    }
                }
                "options.summary" => {
                    if let Ok(summary) = serde_json::from_value::<OptionsChainSummary>(data.clone())
                    {
                        role_options.insert(result.role.clone(), summary);
                    }
                }
                _ => {}
            }
        }
    }

    let paired_candles = role_candles
        .get("etf_reference")
        .cloned()
        .or_else(|| role_candles.get("cfd_reference").cloned());
    let vix_candles = role_candles.get("volatility_reference").cloned();
    let spot_candles = role_candles
        .get("etf_reference")
        .cloned()
        .or_else(|| role_candles.get("cfd_reference").cloned())
        .or_else(|| vix_candles.clone());

    let auxiliary = if let Some(spot_candles) = spot_candles {
        let options_symbol = role_options
            .get("options_underlying")
            .map(|summary| summary.symbol.as_str())
            .unwrap_or(symbol);
        let mut options_summary = role_options
            .get("options_underlying")
            .cloned()
            .unwrap_or_else(|| default_options_summary(options_symbol));
        if !run_spec.use_oi {
            options_summary.put_call_oi_ratio = None;
            options_summary.call_open_interest = 0.0;
            options_summary.put_open_interest = 0.0;
            options_summary.call_volume = 0.0;
            options_summary.put_volume = 0.0;
            options_summary.put_call_volume_ratio = None;
        }
        if !run_spec.use_iv {
            options_summary.near_atm_implied_volatility = None;
        }
        if !run_spec.use_greeks {
            options_summary.near_atm_delta = None;
            options_summary.near_atm_gamma = None;
            options_summary.near_atm_vega = None;
            options_summary.call_gamma_oi = None;
            options_summary.put_gamma_oi = None;
            options_summary.gamma_skew = None;
        }

        let spot_role = if run_spec.use_vix && !run_spec.use_etf && !run_spec.use_cfd {
            "volatility_reference"
        } else if role_candles.contains_key("etf_reference") {
            "etf_reference"
        } else {
            "cfd_reference"
        };
        let spot_symbol = plan
            .tasks
            .iter()
            .find(|task| task.role == spot_role)
            .map(|task| task.symbol.as_str())
            .unwrap_or(symbol);
        let spot_kind = if spot_role == "volatility_reference" {
            SpotInstrumentKind::Index
        } else {
            SpotInstrumentKind::Equity
        };
        let builder = ExternalHttpRuntimeProvider::new("internal://market-data-harness", None);
        let mut auxiliary = builder.build_auxiliary_evidence(
            spot_kind,
            spot_symbol,
            options_symbol,
            &primary_candles,
            &spot_candles,
            &options_summary,
        );
        if let Some(vix) = vix_candles.as_ref() {
            apply_vix_overlay(&mut auxiliary, vix, &mut runtime_notes);
        }
        Some(auxiliary)
    } else {
        None
    };

    Ok(ControlMatrixRuntimeOverrides {
        paired_candles,
        auxiliary,
        runtime_notes,
    })
}

fn build_harness_request(
    data_path: &str,
    symbol: &str,
    run_spec: &Pb12RunSpec,
    primary_candles: &[Candle],
) -> Result<MarketDataHarnessRequest> {
    let mut request = load_control_matrix_request_template()?;
    let mut related_roles = Vec::new();

    if run_spec.use_etf {
        related_roles.push("etf_reference".to_string());
    }
    if run_spec.use_cfd {
        related_roles.push("cfd_reference".to_string());
    }
    if run_spec.use_vix {
        related_roles.push("volatility_reference".to_string());
    }
    if run_spec.use_greeks || run_spec.use_oi || run_spec.use_iv {
        related_roles.push("options_underlying".to_string());
    }
    request.market_key = symbol.to_string();
    request.primary_data_path = Some(data_path.to_string());
    request.interval = Some(yahoo_interval_from_candles(primary_candles));
    request.start = None;
    request.end = None;
    request.count = None;
    request.related_roles = related_roles;
    Ok(request)
}

fn load_control_matrix_request_template() -> Result<MarketDataHarnessRequest> {
    let Some(path) = std::env::var(CONTROL_MATRIX_REQUEST_JSON_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(MarketDataHarnessRequest {
            market_key: String::new(),
            primary_data_path: None,
            interval: None,
            start: None,
            end: None,
            count: None,
            related_roles: Vec::new(),
            provider_preferences: infer_provider_preferences_from_legacy_env(),
            symbol_overrides: std::collections::BTreeMap::new(),
            options_volatility_proxy_symbol: None,
        });
    };
    let raw = std::fs::read_to_string(&path)?;
    let mut request: MarketDataHarnessRequest = serde_json::from_str(&raw)?;
    if request.provider_preferences.is_empty() {
        request.provider_preferences = infer_provider_preferences_from_legacy_env();
    }
    Ok(request)
}

fn infer_provider_preferences_from_legacy_env() -> std::collections::BTreeMap<String, String> {
    let mut preferences = std::collections::BTreeMap::new();
    if let Some(reference_provider) = normalize_optional_provider(
        std::env::var(CONTROL_MATRIX_REFERENCE_PROVIDER_ENV)
            .ok()
            .as_deref(),
    ) {
        preferences.insert("etf_reference".to_string(), reference_provider.clone());
        preferences.insert("cfd_reference".to_string(), reference_provider.clone());
        preferences.insert("volatility_reference".to_string(), reference_provider);
    }
    if let Some(options_provider) = normalize_optional_provider(
        std::env::var(CONTROL_MATRIX_OPTIONS_PROVIDER_ENV)
            .ok()
            .as_deref()
            .or_else(|| {
                if tradingview_mcp_config_from_env_or_local().api_key.is_some() {
                    Some("tradingview_mcp")
                } else {
                    None
                }
            }),
    ) {
        preferences.insert("options_underlying".to_string(), options_provider);
    }
    preferences
}

fn normalize_optional_provider(value: Option<&str>) -> Option<String> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "ibkr" => Some("ibkr".to_string()),
        "tradingview_mcp" | "tradingview" | "tvremix" => Some("tradingview_mcp".to_string()),
        "yfinance" => Some("yfinance".to_string()),
        _ => None,
    }
}

fn apply_vix_overlay(
    auxiliary: &mut AuxiliaryMarketEvidence,
    vix_candles: &[Candle],
    runtime_notes: &mut Vec<String>,
) {
    if vix_candles.len() < 2 {
        return;
    }
    let last = vix_candles
        .last()
        .map(|item| item.close)
        .unwrap_or_default();
    let prior = vix_candles
        .get(vix_candles.len().saturating_sub(6))
        .map(|item| item.close)
        .unwrap_or(last);
    if prior.abs() <= f64::EPSILON {
        return;
    }
    let change = (last - prior) / prior;
    runtime_notes.push(format!("runtime_vix_change_5bar={change:.4}"));
    if change > 0.03 {
        auxiliary.uncertainty_penalty = (auxiliary.uncertainty_penalty + 0.05).min(0.25);
        auxiliary
            .notes
            .push("vix_rising_increases_uncertainty".to_string());
    } else if change < -0.03 {
        auxiliary.long_bias = (auxiliary.long_bias + 0.02).min(0.20);
        auxiliary.notes.push("vix_falling_relaxes_risk".to_string());
    }
}

fn default_options_summary(symbol: &str) -> OptionsChainSummary {
    OptionsChainSummary {
        symbol: symbol.to_string(),
        source: Some("disabled_or_unavailable".to_string()),
        underlying_price: None,
        call_open_interest: 0.0,
        put_open_interest: 0.0,
        put_call_oi_ratio: None,
        call_volume: 0.0,
        put_volume: 0.0,
        put_call_volume_ratio: None,
        near_atm_implied_volatility: None,
        near_atm_delta: None,
        near_atm_gamma: None,
        near_atm_vega: None,
        call_gamma_oi: None,
        put_gamma_oi: None,
        gamma_skew: None,
        nearest_expiration_dte: None,
    }
}

fn yahoo_interval_from_candles(candles: &[Candle]) -> String {
    if candles.len() < 2 {
        return "1d".to_string();
    }
    let delta = candles[1]
        .timestamp
        .signed_duration_since(candles[0].timestamp)
        .num_minutes()
        .abs();
    match delta {
        0 | 1 => "1m",
        2 => "2m",
        5 => "5m",
        15 => "15m",
        30 => "30m",
        60 => "60m",
        90 => "90m",
        1440 => "1d",
        _ if delta >= 1440 => "1d",
        _ => "60m",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn candle(ts: i64, close: f64) -> Candle {
        Candle {
            timestamp: chrono::Utc.timestamp_opt(ts, 0).unwrap(),
            open: close - 1.0,
            high: close + 1.0,
            low: close - 2.0,
            close,
            volume: 1_000.0,
        }
    }

    #[test]
    fn yahoo_interval_infers_common_bar_sizes() {
        let candles = vec![candle(1_700_000_000, 100.0), candle(1_700_000_900, 101.0)];
        assert_eq!(yahoo_interval_from_candles(&candles), "15m");
    }

    #[test]
    fn vix_overlay_only_adjusts_when_history_exists() {
        let mut auxiliary = AuxiliaryMarketEvidence {
            spot_symbol: "QQQ".to_string(),
            options_symbol: "QQQ".to_string(),
            spot_kind: SpotInstrumentKind::Equity,
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
        };
        let mut notes = Vec::new();
        apply_vix_overlay(
            &mut auxiliary,
            &[
                candle(1_700_000_000, 10.0),
                candle(1_700_086_400, 10.1),
                candle(1_700_172_800, 10.2),
                candle(1_700_259_200, 10.3),
                candle(1_700_345_600, 10.4),
                candle(1_700_432_000, 10.8),
            ],
            &mut notes,
        );
        assert!(auxiliary.uncertainty_penalty > 0.0);
        assert!(notes
            .iter()
            .any(|item| item.starts_with("runtime_vix_change_5bar=")));
    }
}
